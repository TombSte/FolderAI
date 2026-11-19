use super::{
    FileError, captured_directory_path,
    native::{self, Directory, EntryKind},
};
use crate::{
    local_identity::{LiveIdentity, LiveObservation, Locator, canonical_path},
    storage::{CopyIntent, CopyKey, Storage, StorageError, TrackedReference},
};
use std::{
    ffi::{OsStr, OsString},
    fs::{self, File, FileTimes, Metadata},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use uuid::Uuid;

pub(super) fn copy_name(source: &OsStr, sequence: usize, is_directory: bool) -> OsString {
    if sequence == 0 {
        return source.to_owned();
    }
    let original = source.to_string_lossy();
    let lowered = original.to_ascii_lowercase();
    let extension = (!is_directory)
        .then(|| {
            [".tar.gz", ".tar.bz2", ".tar.xz"]
                .iter()
                .find(|extension| {
                    lowered.ends_with(**extension) && original.len() > extension.len()
                })
                .map(|extension| &original[original.len() - extension.len()..])
                .or_else(|| {
                    original
                        .rfind('.')
                        .filter(|dot| *dot > 0)
                        .map(|dot| &original[dot..])
                })
        })
        .flatten();
    let (stem, extension) = extension.map_or((original.as_ref(), ""), |ext| {
        (&original[..original.len() - ext.len()], ext)
    });
    if sequence == 1 {
        format!("{stem} - copy{extension}").into()
    } else {
        format!("{stem} - copy ({sequence}){extension}").into()
    }
}

fn prepare_candidate(
    storage: &Storage,
    directory: &Directory,
    destination: &Path,
    source_name: &OsStr,
    source: &TrackedReference,
    operation: &str,
    sequence: &mut usize,
) -> Result<(OsString, CopyIntent), FileError> {
    while *sequence < 10_000 {
        directory.verify()?;
        let attempt = *sequence;
        *sequence += 1;
        let name = copy_name(source_name, attempt, false);
        let path = destination.join(&name);
        match fs::symlink_metadata(&path) {
            Ok(_) => continue,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let locator = Locator::new(path).ok_or(FileError::InvalidRequest)?;
        let attempt = attempt.to_string();
        let key = CopyKey {
            operation_id: operation,
            item_key: &attempt,
        };
        let intent = storage.access(|store| {
            if store.copy_destination_reserved(&locator)? {
                return Ok(None);
            }
            match store.record_copy_intent(key, source, &locator) {
                Err(StorageError::ReferenceConflict)
                    if store.copy_destination_reserved(&locator)? =>
                {
                    Ok(None)
                }
                result => result.map(Some),
            }
        })?;
        if let Some(intent) = intent {
            return Ok((name, intent));
        }
    }
    Err(FileError::AlreadyExists)
}

fn version(metadata: &Metadata) -> io::Result<(u64, std::time::SystemTime)> {
    Ok((metadata.len(), metadata.modified()?))
}

fn check_cancellation(signal: &AtomicBool) -> Result<(), FileError> {
    if signal.load(Ordering::Acquire) {
        Err(FileError::Cancelled)
    } else {
        Ok(())
    }
}

fn copy_chunks(
    source: &mut impl Read,
    destination: &mut impl Write,
    signal: &AtomicBool,
) -> Result<u64, FileError> {
    let mut buffer = vec![0; 256 * 1024];
    let mut total = 0;
    loop {
        check_cancellation(signal)?;
        let count = source.read(&mut buffer)?;
        if count == 0 {
            return Ok(total);
        }
        destination.write_all(&buffer[..count])?;
        total += count as u64;
    }
}

struct Stage<'a> {
    parent: &'a Directory,
    name: OsString,
    directory: Directory,
}

const PAYLOAD: &str = "payload";

impl<'a> Stage<'a> {
    fn prepare(parent: &'a Directory, operation: &str) -> Result<Self, FileError> {
        let name = OsString::from(format!(".folderai-{operation}"));
        parent.verify()?;
        parent.create_directory(&name)?;
        let directory = parent
            .open_directory(&name)
            .map_err(|_| FileError::CleanupFailed)?;
        Ok(Self {
            parent,
            name,
            directory,
        })
    }

    fn fill(
        &self,
        mut source: File,
        reference: &TrackedReference,
        cancelled: &AtomicBool,
    ) -> Result<LiveIdentity, FileError> {
        let payload = OsStr::new(PAYLOAD);
        let mut staged = self.directory.create_file(payload)?;
        // The publication witness comes from this exact exclusive creation.
        let identity = staged
            .try_clone()
            .and_then(LiveIdentity::from_file)
            .map_err(|_| FileError::CleanupFailed)?;
        let result = (|| {
            let initial = version(&source.metadata()?)?;
            if copy_chunks(&mut source, &mut staged, cancelled)? != initial.0
                || version(&source.metadata()?)? != initial
                || reference.verify_source().is_err()
            {
                return Err(FileError::SourceChanged);
            }
            staged.set_times(FileTimes::new().set_modified(initial.1))?;
            staged.set_permissions(source.metadata()?.permissions())?;
            staged.sync_all()?;
            Ok(())
        })();
        if let Err(error) = result {
            self.discard_payload(&identity)?;
            return Err(error);
        }
        Ok(identity)
    }

    fn assert_payload(&self, identity: &LiveIdentity) -> Result<(), FileError> {
        self.directory
            .verify()
            .map_err(|_| FileError::RecoveryPending)?;
        let children = self
            .directory
            .entries()
            .map_err(|_| FileError::RecoveryPending)?;
        if !matches!(children.as_slice(), [(name, EntryKind::RegularFile)] if name == OsStr::new(PAYLOAD))
        {
            return Err(FileError::RecoveryPending);
        }
        let path = canonical_path(&self.directory.path().join(PAYLOAD))
            .map_err(|_| FileError::RecoveryPending)?;
        let expected = Locator::new(path).ok_or(FileError::RecoveryPending)?;
        if identity.observe().map_err(|_| FileError::RecoveryPending)?
            == LiveObservation::Present(expected)
        {
            Ok(())
        } else {
            Err(FileError::RecoveryPending)
        }
    }

    fn discard_payload(&self, identity: &LiveIdentity) -> Result<(), FileError> {
        self.assert_payload(identity)?;
        native::remove_file(&self.directory, OsStr::new(PAYLOAD))
            .map_err(|_| FileError::CleanupFailed)
    }

    fn discard_empty(self) -> Result<(), FileError> {
        self.directory
            .verify()
            .map_err(|_| FileError::CleanupFailed)?;
        native::remove_directory(self.parent, &self.name).map_err(|_| FileError::CleanupFailed)
    }
}

fn abandon(storage: &Storage, intent: &CopyIntent) -> Result<(), FileError> {
    storage
        .access(|store| store.fail_copy(intent))
        .map_err(|_| FileError::RecoveryPending)
}

pub(super) fn copy_file(
    storage: &Storage,
    path: &Path,
    destination: &Path,
    cancelled: &AtomicBool,
) -> Result<(PathBuf, bool), FileError> {
    check_cancellation(cancelled)?;
    let source_name = path.file_name().ok_or(FileError::InvalidRequest)?;
    let origin = Directory::open(path.parent().ok_or(FileError::InvalidRequest)?)?;
    let directory = Directory::open(destination)?;
    let destination = captured_directory_path(&directory)?;
    let source = origin.open_file(source_name)?;
    let opened = LiveIdentity::from_file(source.try_clone()?)?;
    let LiveObservation::Present(locator) = opened.observe()? else {
        return Err(FileError::SourceChanged);
    };
    origin.verify()?;
    let reference = storage.access(|store| {
        let id = store
            .resolve_live_item(&locator, true)?
            .ok_or(StorageError::ReferenceConflict)?;
        let reference = store.attach_live_reference(&id, &locator)?;
        if !reference.matches_open_identity(&opened)? {
            return Err(StorageError::ReferenceConflict);
        }
        Ok(reference)
    })?;
    let operation = Uuid::new_v4().to_string();
    let mut sequence = 0;
    let mut candidate = prepare_candidate(
        storage,
        &directory,
        &destination,
        source_name,
        &reference,
        &operation,
        &mut sequence,
    )?;
    let stage = match Stage::prepare(&directory, &operation) {
        Ok(stage) => stage,
        Err(error) => {
            abandon(storage, &candidate.1)?;
            return Err(error);
        }
    };
    let staged = match stage.fill(source, &reference, cancelled) {
        Ok(staged) => staged,
        Err(error) => {
            abandon(storage, &candidate.1)?;
            // If an identity cannot be established, retain the staging directory.
            if !matches!(error, FileError::RecoveryPending | FileError::CleanupFailed) {
                stage.discard_empty()?;
            }
            return Err(error);
        }
    };

    loop {
        let (name, intent) = candidate;
        stage.assert_payload(&staged)?;
        if let Err(error) =
            check_cancellation(cancelled).and_then(|_| directory.verify().map_err(FileError::from))
        {
            abandon(storage, &intent)?;
            stage.discard_payload(&staged)?;
            stage.discard_empty()?;
            return Err(error);
        }
        let stage_file = stage.directory.open_file(OsStr::new(PAYLOAD))?;
        let witness = LiveIdentity::from_file(stage_file)?;
        if witness.comparison_key()? != staged.comparison_key()? {
            return Err(FileError::RecoveryPending);
        }
        let publication = intent.bind_staged_destination(witness)?;
        match native::publish(&stage.directory, OsStr::new(PAYLOAD), &directory, &name) {
            Ok(()) => {
                directory.verify().map_err(|_| FileError::RecoveryPending)?;
                storage
                    .access(|store| store.record_completed_copy(&intent, &publication))
                    .map_err(|_| FileError::RecoveryPending)?;
                let cleanup_pending = stage.discard_empty().is_err();
                return Ok((directory.path().join(name), cleanup_pending));
            }
            Err(error) => {
                // If the object no longer lives under the captured staging name,
                // leave the attempt and its artifacts for reconciliation.
                stage.assert_payload(&staged)?;
                abandon(storage, &intent)?;
                let next = if error.kind() == io::ErrorKind::AlreadyExists {
                    prepare_candidate(
                        storage,
                        &directory,
                        &destination,
                        source_name,
                        &reference,
                        &operation,
                        &mut sequence,
                    )
                } else {
                    Err(error.into())
                };
                match next {
                    Ok(attempt) => candidate = attempt,
                    Err(error) => {
                        stage.discard_payload(&staged)?;
                        stage.discard_empty()?;
                        return Err(error);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelling_after_a_chunk_stops_before_the_next_write() {
        struct StopAfterWrite<'a> {
            signal: &'a AtomicBool,
            written: usize,
        }
        impl Write for StopAfterWrite<'_> {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.written += bytes.len();
                self.signal.store(true, Ordering::Release);
                Ok(bytes.len())
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let signal = AtomicBool::new(false);
        let bytes = vec![42; 2 * 256 * 1024];
        let mut output = StopAfterWrite {
            signal: &signal,
            written: 0,
        };
        assert_eq!(
            copy_chunks(&mut bytes.as_slice(), &mut output, &signal),
            Err(FileError::Cancelled)
        );
        assert_eq!(output.written, 256 * 1024);
    }

    #[test]
    fn numbered_names_preserve_extension_and_hidden_stem() {
        assert_eq!(
            copy_name(OsStr::new("demo.tar.gz"), 2, false),
            "demo - copy (2).tar.gz"
        );
        assert_eq!(copy_name(OsStr::new(".env"), 1, false), ".env - copy");
        assert_eq!(
            copy_name(OsStr::new("foo - copy.txt"), 1, false),
            "foo - copy - copy.txt"
        );
        assert_eq!(
            copy_name(OsStr::new("dir.with.dots"), 2, true),
            "dir.with.dots - copy (2)"
        );
    }
}
