use super::{
    FileError, captured_directory_path,
    copy::{PAYLOAD, Stage, capture_source, check_cancellation, copy_chunks, copy_name, version},
    native::{self, Directory, EntryKind},
};
use crate::{
    local_identity::{LiveIdentity, Locator},
    storage::{
        CrossMoveIntent, CrossMovePublication, CrossMoveRemoval, CrossMoveSource, Storage,
        StorageError, TrackedReference,
    },
};
use std::{
    ffi::{OsStr, OsString},
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::SystemTime,
};
use uuid::Uuid;

type Version = (u64, SystemTime);

pub(super) enum CrossFileOutcome {
    Completed(PathBuf, bool),
    Pending(Box<PendingCrossFile>),
}

pub(super) struct PendingCrossFile {
    source: File,
    source_version: Version,
    origin: Directory,
    source_name: OsString,
    reference: TrackedReference,
    output: File,
    output_version: Version,
    destination: Directory,
    visible_path: PathBuf,
    intent: CrossMoveIntent,
    publication: CrossMovePublication,
    destination_stage: Option<Stage>,
    hold_token: String,
    hold_path: Locator,
    hold: Option<Stage>,
    quarantine_attempted: bool,
}

impl PendingCrossFile {
    pub(super) fn path(&self) -> &Path {
        &self.visible_path
    }

    pub(super) fn staging_path(&self) -> &Path {
        if self.quarantine_attempted {
            self.hold_path.path()
        } else {
            self.intent.source().path()
        }
    }

    pub(super) fn commit(&mut self, storage: &Storage) -> Result<(PathBuf, bool), FileError> {
        self.finish(storage, &AtomicBool::new(false), native::remove_file)
    }

    fn finish(
        &mut self,
        storage: &Storage,
        cancelled: &AtomicBool,
        remove: impl FnOnce(&Directory, &OsStr) -> io::Result<()>,
    ) -> Result<(PathBuf, bool), FileError> {
        // Persist the visible copy while the original is still recoverable.
        // Retrying this phase reuses the same publication and quarantine path.
        self.destination
            .verify()
            .map_err(|_| FileError::RecoveryPending)?;
        let removal = storage
            .access(|store| {
                store.record_cross_volume_publication(
                    &self.intent,
                    &self.reference,
                    &self.publication,
                    Some(&self.hold_path),
                )
            })
            .map_err(|_| FileError::RecoveryPending)?;

        // On APFS the destination directory can be synchronized before an
        // irreversible effect on the other volume.
        #[cfg(target_os = "macos")]
        self.destination.try_clone_file()?.sync_all()?;

        match self.source_location(&removal)? {
            CrossMoveSource::Original => {
                self.verify_before_effect(cancelled)?;
                self.verify_contents(cancelled)?;
                if self.hold.is_none() {
                    self.hold = Some(Stage::prepare(&self.origin, &self.hold_token)?);
                }
                let hold = self.hold.as_ref().ok_or(FileError::RecoveryPending)?;
                hold.directory.verify()?;
                if !hold.directory.entries()?.is_empty()
                    || self.source_location(&removal)? != CrossMoveSource::Original
                {
                    return Err(FileError::RecoveryPending);
                }
                check_cancellation(cancelled)?;
                self.quarantine_attempted = true;
                let effect = native::publish(
                    &self.origin,
                    &self.source_name,
                    &hold.directory,
                    OsStr::new(PAYLOAD),
                );
                if self.source_location(&removal)? != CrossMoveSource::Quarantined {
                    return Err(effect
                        .err()
                        .map_or(FileError::RecoveryPending, FileError::from));
                }
            }
            CrossMoveSource::Quarantined | CrossMoveSource::Removed => {}
        }

        let location = self.source_location(&removal)?;
        if location == CrossMoveSource::Original {
            return Err(FileError::RecoveryPending);
        }
        // Read the retained files, including after quarantine. A source edit
        // made with its original length and modification time cannot be ignored.
        self.verify_contents(cancelled)?;

        if location == CrossMoveSource::Quarantined {
            self.verify_before_effect(cancelled)?;
            let hold = self.hold.as_ref().ok_or(FileError::RecoveryPending)?;
            hold.directory.verify()?;
            let entries = hold.directory.entries()?;
            if !matches!(entries.as_slice(), [(name, EntryKind::RegularFile)] if name == PAYLOAD)
                || self.source_location(&removal)? != CrossMoveSource::Quarantined
            {
                return Err(FileError::RecoveryPending);
            }
            check_cancellation(cancelled)?;
            let effect = remove(&hold.directory, OsStr::new(PAYLOAD));
            if self.source_location(&removal)? != CrossMoveSource::Removed {
                return Err(effect
                    .err()
                    .map_or(FileError::RecoveryPending, FileError::from));
            }
        }

        self.destination
            .verify()
            .map_err(|_| FileError::RecoveryPending)?;
        storage
            .access(|store| {
                store.complete_cross_volume_move(
                    &self.intent,
                    &self.reference,
                    &self.publication,
                    &removal,
                )
            })
            .map_err(|_| FileError::RecoveryPending)?;
        self.destination
            .verify()
            .map_err(|_| FileError::RecoveryPending)?;

        let mut cleanup_pending = false;
        if let Some(hold) = self.hold.take() {
            cleanup_pending |= hold.discard_empty(&self.origin).is_err();
        }
        if let Some(stage) = self.destination_stage.take() {
            cleanup_pending |= stage.discard_empty(&self.destination).is_err();
        }
        Ok((self.visible_path.clone(), cleanup_pending))
    }

    fn source_location(&self, removal: &CrossMoveRemoval) -> Result<CrossMoveSource, FileError> {
        removal
            .verify_before_removal(&self.reference, &self.publication)
            .map_err(|_| FileError::RecoveryPending)
    }

    fn verify_contents(&mut self, cancelled: &AtomicBool) -> Result<(), FileError> {
        equal_bytes(
            &mut self.source,
            self.source_version,
            &mut self.output,
            self.output_version,
            cancelled,
        )
    }

    fn verify_before_effect(&self, cancelled: &AtomicBool) -> Result<(), FileError> {
        check_cancellation(cancelled)?;
        self.origin.verify()?;
        self.destination.verify()?;
        if version(&self.source.metadata()?)? != self.source_version
            || version(&self.output.metadata()?)? != self.output_version
        {
            return Err(FileError::SourceChanged);
        }
        Ok(())
    }
}

fn equal_bytes(
    source: &mut File,
    source_version: Version,
    output: &mut File,
    output_version: Version,
    cancelled: &AtomicBool,
) -> Result<(), FileError> {
    if version(&source.metadata()?)? != source_version
        || version(&output.metadata()?)? != output_version
        || source_version.0 != output_version.0
    {
        return Err(FileError::SourceChanged);
    }
    source.seek(SeekFrom::Start(0))?;
    output.seek(SeekFrom::Start(0))?;
    let mut source_buffer = vec![0; 256 * 1024];
    let mut output_buffer = vec![0; source_buffer.len()];
    let mut remaining = source_version.0;
    while remaining != 0 {
        check_cancellation(cancelled)?;
        let count = remaining.min(source_buffer.len() as u64) as usize;
        let source_slice = &mut source_buffer[..count];
        let output_slice = &mut output_buffer[..count];
        source.read_exact(source_slice)?;
        output.read_exact(output_slice)?;
        if source_slice != output_slice {
            return Err(FileError::SourceChanged);
        }
        remaining -= count as u64;
    }
    check_cancellation(cancelled)?;
    if version(&source.metadata()?)? != source_version
        || version(&output.metadata()?)? != output_version
    {
        return Err(FileError::SourceChanged);
    }
    Ok(())
}

fn prepare_candidate(
    storage: &Storage,
    directory: &Directory,
    destination: &Path,
    source_name: &OsStr,
    source: &TrackedReference,
    operation: (&str, &str),
    sequence: &mut usize,
) -> Result<(OsString, CrossMoveIntent), FileError> {
    while *sequence < 10_000 {
        directory.verify()?;
        let name = copy_name(source_name, *sequence, false);
        let item_key = format!("{}:{}", operation.1, *sequence);
        *sequence += 1;
        let destination = Locator::new(destination.join(&name)).ok_or(FileError::InvalidRequest)?;
        if occupied(destination.path())? {
            continue;
        }
        let intents = storage.access(|store| {
            if store.copy_destination_reserved(&destination)? {
                return Ok(None);
            }
            match store.record_cross_volume_move(operation.0, &item_key, source, &[], &destination)
            {
                Err(StorageError::ReferenceConflict)
                    if store.copy_destination_reserved(&destination)?
                        || fs::symlink_metadata(destination.path()).is_ok() =>
                {
                    Ok(None)
                }
                result => result.map(Some),
            }
        })?;
        if let Some(mut intents) = intents {
            let intent = intents.pop().ok_or(FileError::RecoveryPending)?;
            return Ok((name, intent));
        }
    }
    Err(FileError::AlreadyExists)
}

fn occupied(path: &Path) -> Result<bool, FileError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn fail_unpublished(
    storage: &Storage,
    intent: &CrossMoveIntent,
    publication: Option<&CrossMovePublication>,
) -> Result<(), FileError> {
    storage
        .access(|store| {
            store.fail_unpublished_cross_volume_move(
                std::slice::from_ref(intent),
                publication.map_or(&[], std::slice::from_ref),
            )
        })
        .map_err(|_| FileError::RecoveryPending)
}

pub(super) fn move_file(
    storage: &Storage,
    source: &Path,
    destination: &Directory,
    operation: (&str, &str),
    cancelled: &AtomicBool,
) -> Result<CrossFileOutcome, FileError> {
    let mut pending = stage_and_publish(
        storage,
        source,
        destination,
        operation,
        cancelled,
        copy_chunks,
        native::publish,
    )?;
    match pending.finish(storage, cancelled, native::remove_file) {
        Ok((path, cleanup)) => Ok(CrossFileOutcome::Completed(path, cleanup)),
        Err(_) => Ok(CrossFileOutcome::Pending(Box::new(pending))),
    }
}

fn stage_and_publish(
    storage: &Storage,
    path: &Path,
    directory: &Directory,
    operation: (&str, &str),
    cancelled: &AtomicBool,
    transfer: impl FnOnce(&mut File, &mut File, &AtomicBool) -> Result<u64, FileError>,
    mut publish: impl FnMut(&Directory, &OsStr, &Directory, &OsStr) -> io::Result<()>,
) -> Result<PendingCrossFile, FileError> {
    check_cancellation(cancelled)?;
    let source_name = path
        .file_name()
        .ok_or(FileError::InvalidRequest)?
        .to_owned();
    let chosen_origin = Directory::open(path.parent().ok_or(FileError::InvalidRequest)?)?;
    let source_directory = captured_directory_path(&chosen_origin)?;
    let destination = captured_directory_path(directory)?;
    let origin = Directory::open(&source_directory)?;
    let (mut source, reference) = capture_source(storage, path)?;
    let origin_witness = LiveIdentity::from_file(origin.open_file(&source_name)?)?;
    if !reference.matches_open_identity(&origin_witness)? {
        return Err(FileError::SourceChanged);
    }
    chosen_origin.verify()?;
    let retained_destination = directory.duplicate()?;
    let hold_token = Uuid::new_v4().to_string();
    let hold_path = Locator::new(
        source_directory
            .join(format!(".folderai-{hold_token}"))
            .join(PAYLOAD),
    )
    .ok_or(FileError::InvalidRequest)?;

    let mut sequence = 0;
    let mut candidate = prepare_candidate(
        storage,
        directory,
        &destination,
        &source_name,
        &reference,
        operation,
        &mut sequence,
    )?;
    let stage = match Stage::prepare(directory, &Uuid::new_v4().to_string()) {
        Ok(stage) => stage,
        Err(error) => {
            fail_unpublished(storage, &candidate.1, None)?;
            return Err(error);
        }
    };
    let (staged, source_version) =
        match stage.fill_with(&mut source, &reference, cancelled, transfer) {
            Ok(result) => result,
            Err(error) => {
                fail_unpublished(storage, &candidate.1, None)?;
                if !matches!(error, FileError::RecoveryPending | FileError::CleanupFailed) {
                    stage.discard_empty(directory)?;
                }
                return Err(error);
            }
        };
    let staged_output = (|| -> Result<(File, Version), FileError> {
        stage.assert_payload(&staged)?;
        let file = stage.directory.open_file(OsStr::new(PAYLOAD))?;
        let opened = LiveIdentity::from_file(file.try_clone()?)?;
        if opened.comparison_key()? != staged.comparison_key()? {
            return Err(FileError::RecoveryPending);
        }
        let file_version = version(&file.metadata()?)?;
        Ok((file, file_version))
    })();
    let (output, output_version) = match staged_output {
        Ok(output) => output,
        Err(error) => {
            fail_unpublished(storage, &candidate.1, None)?;
            if error != FileError::RecoveryPending {
                stage.discard_payload(&staged)?;
                stage.discard_empty(directory)?;
            }
            return Err(error);
        }
    };
    loop {
        let (name, intent) = candidate;
        let ready = (|| {
            check_cancellation(cancelled)?;
            origin.verify()?;
            directory.verify()?;
            if reference.verify_source().is_err() || version(&source.metadata()?)? != source_version
            {
                return Err(FileError::SourceChanged);
            }
            stage.assert_payload(&staged)?;
            if version(&output.metadata()?)? != output_version {
                return Err(FileError::RecoveryPending);
            }
            let publication =
                intent.bind_staged_destination(LiveIdentity::from_file(output.try_clone()?)?)?;
            Ok(publication)
        })();
        let publication = match ready {
            Ok(publication) => publication,
            Err(error) => {
                fail_unpublished(storage, &intent, None)?;
                if error != FileError::RecoveryPending {
                    stage.discard_payload(&staged)?;
                    stage.discard_empty(directory)?;
                }
                return Err(error);
            }
        };
        if let Err(error) = check_cancellation(cancelled) {
            fail_unpublished(storage, &intent, Some(&publication))?;
            stage.discard_payload(&staged)?;
            stage.discard_empty(directory)?;
            return Err(error);
        }
        let effect = publish(&stage.directory, OsStr::new(PAYLOAD), directory, &name);
        if let Err(error) = effect
            && stage.assert_payload(&staged).is_ok()
        {
            fail_unpublished(storage, &intent, Some(&publication))?;
            let next = if error.kind() == io::ErrorKind::AlreadyExists {
                prepare_candidate(
                    storage,
                    directory,
                    &destination,
                    &source_name,
                    &reference,
                    operation,
                    &mut sequence,
                )
            } else {
                Err(error.into())
            };
            match next {
                Ok(new_candidate) => {
                    candidate = new_candidate;
                    continue;
                }
                Err(error) => {
                    stage.discard_payload(&staged)?;
                    stage.discard_empty(directory)?;
                    return Err(error);
                }
            }
        }
        return Ok(PendingCrossFile {
            origin,
            source_name,
            source,
            source_version,
            reference,
            output,
            output_version,
            destination: retained_destination,
            visible_path: directory.path().join(name),
            intent,
            publication,
            destination_stage: Some(stage),
            hold_token,
            hold_path,
            hold: None,
            quarantine_attempted: false,
        });
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use crate::{local_identity::canonical_path, storage::CrossMoveRecovery};
    use std::{fs::FileTimes, io::Write, sync::atomic::Ordering};

    const OPERATION: (&str, &str) = ("regular-cross-volume", "item");
    const ORIGINAL: &[u8] = b"original bytes";

    struct Fixture {
        root: PathBuf,
        source: PathBuf,
        destination: Directory,
        storage: Storage,
    }

    impl Fixture {
        fn new() -> Self {
            let name = format!("folderai-cross-file-{}", Uuid::new_v4());
            let root = std::env::temp_dir().join(&name);
            fs::create_dir_all(root.join("from")).unwrap();
            let root = canonical_path(&root).unwrap();
            let source = root.join("from/résumé.txt");
            fs::write(&source, ORIGINAL).unwrap();
            let second_volume = std::env::var_os("FOLDERAI_TEST_SECOND_VOLUME");
            let destination_path = second_volume.as_ref().map_or_else(
                || root.join("to"),
                |mount| canonical_path(Path::new(mount)).unwrap().join(&name),
            );
            fs::create_dir(&destination_path).unwrap();
            let destination = Directory::open(&destination_path).unwrap();
            if second_volume.is_some() {
                let origin = Directory::open(source.parent().unwrap()).unwrap();
                assert!(!origin.same_volume(&destination).unwrap());
            }
            let storage = Storage::for_test(&root.join("data"), &root.join("cache")).unwrap();
            Self {
                root,
                source,
                destination,
                storage,
            }
        }

        fn stage(&self, cancelled: &AtomicBool) -> PendingCrossFile {
            stage_and_publish(
                &self.storage,
                &self.source,
                &self.destination,
                OPERATION,
                cancelled,
                copy_chunks,
                native::publish,
            )
            .unwrap()
        }

        fn status(&self, pending: &PendingCrossFile) -> CrossMoveRecovery {
            self.storage
                .access(|store| store.recover_cross_volume_move(pending.intent.key()))
                .unwrap()
        }

        fn target(&self) -> PathBuf {
            self.destination
                .path()
                .join(self.source.file_name().unwrap())
        }

        fn track_with_note(&self) -> String {
            self.storage
                .access(|store| {
                    let source = Locator::new(self.source.clone()).unwrap();
                    let id = store.resolve_live_item(&source, true)?.unwrap();
                    store.connection.execute(
                        "INSERT INTO notes (item_id, body, revision) VALUES (?1, 'surviving note', 5)",
                        [&id],
                    )?;
                    Ok(id)
                })
                .unwrap()
        }

        fn assert_original_metadata_at_target(&self, id: &str, path: &Path) {
            self.storage
                .access(|store| {
                    let locator = Locator::new(path.to_owned()).unwrap();
                    assert_eq!(
                        store.resolve_live_item(&locator, false)?.as_deref(),
                        Some(id)
                    );
                    let note: (String, i64) = store.connection.query_row(
                        "SELECT body, revision FROM notes WHERE item_id=?1",
                        [id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )?;
                    assert_eq!(note, ("surviving note".into(), 5));
                    Ok(())
                })
                .unwrap();
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(self.destination.path());
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn moves_regular_file_and_keeps_item_identity_and_note() {
        let fixture = Fixture::new();
        let id = fixture.track_with_note();
        let outcome = move_file(
            &fixture.storage,
            &fixture.source,
            &fixture.destination,
            OPERATION,
            &AtomicBool::new(false),
        )
        .unwrap();
        let CrossFileOutcome::Completed(path, cleanup_pending) = outcome else {
            panic!("expected a completed move");
        };
        assert!(!cleanup_pending);
        assert!(!fixture.source.exists());
        assert_eq!(fs::read(&path).unwrap(), ORIGINAL);
        fixture.assert_original_metadata_at_target(&id, &path);
        assert_eq!(fs::read_dir(fixture.destination.path()).unwrap().count(), 1);
        assert_eq!(
            fs::read_dir(fixture.source.parent().unwrap())
                .unwrap()
                .count(),
            0
        );
    }

    #[test]
    fn refuses_partial_copy_and_preserves_original() {
        let fixture = Fixture::new();
        let error = stage_and_publish(
            &fixture.storage,
            &fixture.source,
            &fixture.destination,
            OPERATION,
            &AtomicBool::new(false),
            |_, output, _| {
                output.write_all(b"partial")?;
                Err(FileError::StorageFull)
            },
            |_, _, _, _| panic!("partial copy cannot publish"),
        )
        .err()
        .unwrap();
        assert_eq!(error, FileError::StorageFull);
        assert_eq!(fs::read(&fixture.source).unwrap(), ORIGINAL);
        assert_eq!(fs::read_dir(fixture.destination.path()).unwrap().count(), 0);
    }

    #[test]
    fn late_collision_keeps_both_and_reuses_staged_copy() {
        let fixture = Fixture::new();
        let mut calls = 0;
        let mut pending = stage_and_publish(
            &fixture.storage,
            &fixture.source,
            &fixture.destination,
            OPERATION,
            &AtomicBool::new(false),
            copy_chunks,
            |stage, payload, destination, name| {
                calls += 1;
                if calls == 1 {
                    fs::write(destination.path().join(name), b"other").unwrap();
                }
                native::publish(stage, payload, destination, name)
            },
        )
        .unwrap();
        assert_eq!(calls, 2);
        let (path, cleanup) = pending.commit(&fixture.storage).unwrap();
        assert_eq!(path.file_name().unwrap(), OsStr::new("résumé - copy.txt"));
        assert!(!cleanup);
        assert_eq!(fs::read(&path).unwrap(), ORIGINAL);
        assert_eq!(fs::read(fixture.target()).unwrap(), b"other");
        assert!(!fixture.source.exists());
    }

    #[test]
    fn stopping_after_publication_records_both_objects_until_explicit_retry() {
        let fixture = Fixture::new();
        let cancelled = AtomicBool::new(false);
        let mut pending = fixture.stage(&cancelled);
        cancelled.store(true, Ordering::Release);
        assert_eq!(
            pending.finish(&fixture.storage, &cancelled, |_, _| panic!("stopped")),
            Err(FileError::Cancelled)
        );
        assert_eq!(pending.staging_path(), fixture.source);
        assert_eq!(fs::read(pending.path()).unwrap(), ORIGINAL);
        assert_eq!(fs::read(&fixture.source).unwrap(), ORIGINAL);
        assert!(matches!(
            fixture.status(&pending),
            CrossMoveRecovery::DestinationPublished { .. }
        ));
        let destination_before = native_key(pending.path());
        let (path, cleanup) = pending.commit(&fixture.storage).unwrap();
        assert!(!cleanup);
        assert_eq!(native_key(&path), destination_before);
        assert!(!fixture.source.exists());
        assert_eq!(fixture.status(&pending), CrossMoveRecovery::Completed);
    }

    #[test]
    fn failed_source_removal_retains_quarantined_file_and_retries_without_copying() {
        let fixture = Fixture::new();
        let id = fixture.track_with_note();
        let cancelled = AtomicBool::new(false);
        let source_before = native_key(&fixture.source);
        let mut pending = fixture.stage(&cancelled);
        let destination_before = native_key(pending.path());
        let failure = pending.finish(&fixture.storage, &cancelled, |_, _| {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        });
        assert_eq!(failure, Err(FileError::PermissionDenied));
        assert!(!fixture.source.exists());
        let retained = pending.staging_path().to_owned();
        let status = fixture.status(&pending);
        assert!(matches!(status, CrossMoveRecovery::DestinationPublished {
            source_quarantine: Some(ref locator), ..
        } if locator.path() == retained));
        assert_eq!(native_key(&retained), source_before);
        assert_eq!(fs::read(&retained).unwrap(), ORIGINAL);
        assert_eq!(fs::read(pending.path()).unwrap(), ORIGINAL);

        let (path, cleanup) = pending.commit(&fixture.storage).unwrap();
        assert!(!cleanup);
        assert!(!retained.exists());
        assert_eq!(native_key(&path), destination_before);
        assert_eq!(fixture.status(&pending), CrossMoveRecovery::Completed);
        fixture.assert_original_metadata_at_target(&id, &path);
    }

    #[test]
    fn same_length_source_edit_with_restored_mtime_does_not_relocate_original() {
        let fixture = Fixture::new();
        let mut pending = fixture.stage(&AtomicBool::new(false));
        let old_time = fs::metadata(&fixture.source).unwrap().modified().unwrap();
        fs::write(&fixture.source, b"modified bytes").unwrap();
        File::options()
            .write(true)
            .open(&fixture.source)
            .unwrap()
            .set_times(FileTimes::new().set_modified(old_time))
            .unwrap();
        assert_eq!(
            pending.commit(&fixture.storage),
            Err(FileError::SourceChanged)
        );
        assert_eq!(fs::read(&fixture.source).unwrap(), b"modified bytes");
        assert_eq!(fs::read(pending.path()).unwrap(), ORIGINAL);
        assert_eq!(pending.staging_path(), fixture.source);
    }

    #[test]
    fn unrelated_destination_substitution_prevents_original_removal_and_retry() {
        let fixture = Fixture::new();
        let mut pending = fixture.stage(&AtomicBool::new(false));
        let swapped_away = fixture.destination.path().join("published-moved");
        fs::rename(pending.path(), &swapped_away).unwrap();
        fs::write(pending.path(), b"replacement").unwrap();
        assert_eq!(
            pending.commit(&fixture.storage),
            Err(FileError::RecoveryPending)
        );
        assert_eq!(fs::read(&fixture.source).unwrap(), ORIGINAL);
        assert_eq!(fs::read(&swapped_away).unwrap(), ORIGINAL);
        assert_eq!(fs::read(pending.path()).unwrap(), b"replacement");
        assert_eq!(
            fixture.status(&pending),
            CrossMoveRecovery::AwaitingEvidence
        );
    }

    fn native_key(path: &Path) -> Vec<u8> {
        LiveIdentity::open(&Locator::new(path.to_owned()).unwrap())
            .unwrap()
            .comparison_key()
            .unwrap()
    }
}
