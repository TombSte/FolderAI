use super::{
    FileError, captured_directory_path,
    copy::{PAYLOAD, Stage, abandon, capture_source, check_cancellation, copy_chunks, version},
    native::{self, Directory},
};
use crate::{
    local_identity::{LiveIdentity, LiveObservation, Locator},
    storage::{CopyIntent, CopyKey, CopyPublication, Storage, StorageError},
};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsStr,
    fs::{File, Metadata},
    io::{self, Read},
    os::unix::fs::{FileExt, MetadataExt},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq)]
struct FileVersion {
    len: u64,
    modified: (i64, i64),
    changed: (i64, i64),
    mode: u32,
    uid: u32,
    gid: u32,
}

impl From<Metadata> for FileVersion {
    fn from(metadata: Metadata) -> Self {
        Self {
            len: metadata.len(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
        }
    }
}

#[derive(Clone, Copy)]
struct Fingerprint {
    version: FileVersion,
    digest: [u8; 32],
}

struct HashingReader<'a> {
    source: &'a mut File,
    hash: &'a mut Sha256,
}

impl Read for HashingReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = self.source.read(buffer)?;
        self.hash.update(&buffer[..count]);
        Ok(count)
    }
}

impl Fingerprint {
    fn read(file: &File, cancelled: Option<&AtomicBool>) -> Result<Self, FileError> {
        let before = FileVersion::from(file.metadata()?);
        let mut hash = Sha256::new();
        let mut buffer = vec![0; 256 * 1024];
        let mut offset = 0;
        while offset < before.len {
            if let Some(cancelled) = cancelled {
                check_cancellation(cancelled)?;
            }
            let count = file.read_at(&mut buffer, offset)?;
            if count == 0 {
                return Err(FileError::SourceChanged);
            }
            hash.update(&buffer[..count]);
            offset += count as u64;
        }
        if offset != before.len || FileVersion::from(file.metadata()?) != before {
            return Err(FileError::SourceChanged);
        }
        Ok(Self {
            version: before,
            digest: hash.finalize().into(),
        })
    }

    fn unchanged_since(&self, file: &File) -> Result<bool, FileError> {
        Ok(FileVersion::from(file.metadata()?) == self.version)
    }

    fn matches_across_swap(&self, current: &Self) -> bool {
        // APFS updates both ctimes during exchange. Compare the bytes as well
        // as the metadata that the exchange itself preserves.
        let mut version = current.version;
        version.changed = self.version.changed;
        self.version == version && self.digest == current.digest
    }
}

pub(super) struct ReplacementTarget {
    directory: Directory,
    location: Locator,
    file: File,
    identity: LiveIdentity,
    approved: Fingerprint,
}

impl ReplacementTarget {
    pub(super) fn capture(path: &Path) -> Result<Self, FileError> {
        let name = path.file_name().ok_or(FileError::InvalidRequest)?;
        let directory = Directory::open(path.parent().ok_or(FileError::InvalidRequest)?)?;
        let file = directory.open_file(name)?;
        let identity = LiveIdentity::from_file(file.try_clone()?)?;
        let LiveObservation::Present(location) = identity.observe()? else {
            return Err(FileError::SourceChanged);
        };
        if location.path().parent() != Some(captured_directory_path(&directory)?.as_path()) {
            return Err(FileError::SourceChanged);
        }
        let current = LiveIdentity::from_file(directory.open_file(name)?)?;
        if current.comparison_key()? != identity.comparison_key()? {
            return Err(FileError::SourceChanged);
        }
        let approved = Fingerprint::read(&file, None)?;
        let target = Self {
            directory,
            location,
            file,
            identity,
            approved,
        };
        target.verify_approval()?;
        Ok(target)
    }

    pub(super) fn path(&self) -> &Path {
        self.location.path()
    }

    fn name(&self) -> &OsStr {
        self.path()
            .file_name()
            .expect("validated replacement target")
    }

    fn verify_approval(&self) -> Result<(), FileError> {
        self.directory.verify()?;
        if self.identity.observe()? == LiveObservation::Present(self.location.clone())
            && self.approved.unchanged_since(&self.file)?
        {
            Ok(())
        } else {
            Err(FileError::SourceChanged)
        }
    }
}

struct PreparedReplacement {
    operation: String,
    intent: CopyIntent,
    publication: CopyPublication,
    target: ReplacementTarget,
    stage: Stage,
    staged: LiveIdentity,
    staged_file: File,
    staged_fingerprint: Fingerprint,
    metadata_committed: bool,
}

pub(super) enum ReplacementOutcome {
    Completed {
        path: PathBuf,
        cleanup_pending: bool,
    },
    RecoveryPending(Box<ReplacementRecovery>),
}

/// Retains both pre-exchange handles and the journal token even if the first
/// completion attempt fails. Recovery checks existing effects and never swaps
/// again or looks up a fresh target by name as a substitute for proof.
pub(super) struct ReplacementRecovery {
    prepared: Option<PreparedReplacement>,
}

impl ReplacementRecovery {
    fn prepared(&self) -> &PreparedReplacement {
        self.prepared.as_ref().expect("unfinished replacement")
    }

    pub(super) fn destination_path(&self) -> &Path {
        self.prepared().target.path()
    }

    pub(super) fn staging_path(&self) -> PathBuf {
        let stage = &self.prepared().stage.directory;
        let witness = stage.try_clone_file().and_then(LiveIdentity::from_file);
        if let Ok(LiveObservation::Present(location)) = witness.and_then(|handle| handle.observe())
        {
            return location.path().join(PAYLOAD);
        }
        stage.path().join(PAYLOAD)
    }

    pub(super) fn metadata_committed(&self) -> bool {
        self.prepared().metadata_committed
    }

    pub(super) fn retry(&mut self, storage: &Storage) -> Result<(PathBuf, bool), FileError> {
        self.finish(storage, || {})
    }

    fn finish(
        &mut self,
        storage: &Storage,
        before_cleanup: impl FnOnce(),
    ) -> Result<(PathBuf, bool), FileError> {
        let prepared = self.prepared.as_mut().ok_or(FileError::InvalidRequest)?;
        let (backup, published) = prepared.verify_exchange()?;
        storage
            .access(|store| store.record_completed_copy(&prepared.intent, &prepared.publication))
            .map_err(|_| FileError::RecoveryPending)?;
        prepared.metadata_committed = true;
        before_cleanup();
        // A concurrent edit or namespace change since verification leaves
        // both objects available for review, even after the metadata commit.
        if !backup.unchanged_since(&prepared.target.file)?
            || !published.unchanged_since(&prepared.staged_file)?
            || prepared.target.directory.verify().is_err()
            || prepared.staged.observe()?
                != LiveObservation::Present(prepared.target.location.clone())
        {
            return Err(FileError::RecoveryPending);
        }
        prepared.stage.discard_payload(&prepared.target.identity)?;

        let prepared = self.prepared.take().expect("unfinished replacement");
        let cleanup_pending = prepared
            .stage
            .discard_empty(&prepared.target.directory)
            .is_err();
        Ok((prepared.target.path().to_owned(), cleanup_pending))
    }
}

impl PreparedReplacement {
    fn verify_exchange(&self) -> Result<(Fingerprint, Fingerprint), FileError> {
        let uncertain = |_| FileError::RecoveryPending;
        self.target.directory.verify().map_err(uncertain)?;
        self.stage.assert_payload(&self.target.identity)?;
        if self.staged.observe().map_err(uncertain)?
            != LiveObservation::Present(self.target.location.clone())
        {
            return Err(FileError::RecoveryPending);
        }
        let backup =
            Fingerprint::read(&self.target.file, None).map_err(|_| FileError::RecoveryPending)?;
        let published =
            Fingerprint::read(&self.staged_file, None).map_err(|_| FileError::RecoveryPending)?;
        if !self.target.approved.matches_across_swap(&backup)
            || !self.staged_fingerprint.matches_across_swap(&published)
        {
            return Err(FileError::RecoveryPending);
        }
        Ok((backup, published))
    }

    fn retain(self, storage: &Storage) -> ReplacementRecovery {
        let _ = storage.access(|store| {
            store.recover_copy(CopyKey {
                operation_id: &self.operation,
                item_key: "replace",
            })
        });
        ReplacementRecovery {
            prepared: Some(self),
        }
    }

    fn discard(self, storage: &Storage) -> Result<(), FileError> {
        abandon(storage, &self.intent)?;
        self.stage.discard_payload(&self.staged)?;
        self.stage.discard_empty(&self.target.directory)
    }
}

pub(super) fn replace_file(
    storage: &Storage,
    source: &Path,
    target: ReplacementTarget,
    cancelled: &AtomicBool,
) -> Result<ReplacementOutcome, FileError> {
    replace_file_with_hooks(storage, source, target, cancelled, Hooks::default())
}

#[derive(Default)]
struct Hooks<'a> {
    before_copy: Option<&'a BeforeCopy>,
    before_swap: Option<&'a dyn Fn()>,
    after_swap: Option<&'a dyn Fn()>,
    before_cleanup: Option<&'a dyn Fn()>,
}

type BeforeCopy = dyn Fn(&mut File) -> Result<(), FileError>;

fn replace_file_with_hooks(
    storage: &Storage,
    source: &Path,
    target: ReplacementTarget,
    cancelled: &AtomicBool,
    hooks: Hooks<'_>,
) -> Result<ReplacementOutcome, FileError> {
    check_cancellation(cancelled)?;
    target.verify_approval()?;
    let (mut source, reference) = capture_source(storage, source)?;
    if reference.matches_open_identity(&target.identity)? {
        return Err(FileError::InvalidRequest);
    }
    let operation = Uuid::new_v4().to_string();
    let intent = storage.access(|store| {
        if store.copy_destination_reserved(&target.location)? {
            return Err(StorageError::ReferenceConflict);
        }
        store.record_copy_intent(
            CopyKey {
                operation_id: &operation,
                item_key: "replace",
            },
            &reference,
            &target.location,
        )
    })?;
    if let Err(error) = target.verify_approval() {
        abandon(storage, &intent)?;
        return Err(error);
    }
    let stage = match Stage::prepare(&target.directory, &operation) {
        Ok(stage) => stage,
        Err(error) => {
            abandon(storage, &intent)?;
            return Err(error);
        }
    };
    let mut copied_hash = Sha256::new();
    let (staged, source_version) = match stage.fill_with(
        &mut source,
        &reference,
        cancelled,
        |source, output, cancelled| {
            if let Some(hook) = hooks.before_copy {
                hook(output)?;
            }
            let mut reader = HashingReader {
                source,
                hash: &mut copied_hash,
            };
            copy_chunks(&mut reader, output, cancelled)
        },
    ) {
        Ok(result) => result,
        Err(error) => {
            abandon(storage, &intent)?;
            if !matches!(error, FileError::CleanupFailed | FileError::RecoveryPending) {
                stage.discard_empty(&target.directory)?;
            }
            return Err(error);
        }
    };
    let prepare = || {
        stage.assert_payload(&staged)?;
        let staged_file = stage.directory.open_file(OsStr::new(PAYLOAD))?;
        let witness = LiveIdentity::from_file(staged_file.try_clone()?)?;
        if witness.comparison_key()? != staged.comparison_key()? {
            return Err(FileError::RecoveryPending);
        }
        let staged_fingerprint = Fingerprint::read(&staged_file, Some(cancelled))?;
        let copied_hash: [u8; 32] = copied_hash.finalize().into();
        if staged_fingerprint.digest != copied_hash {
            return Err(FileError::RecoveryPending);
        }
        let publication = intent.bind_staged_destination(witness)?;
        check_cancellation(cancelled)?;
        if version(&source.metadata()?)? != source_version || reference.verify_source().is_err() {
            return Err(FileError::SourceChanged);
        }
        target.verify_approval()?;
        if !staged_fingerprint.unchanged_since(&staged_file)? {
            return Err(FileError::RecoveryPending);
        }
        stage.assert_payload(&staged)?;
        check_cancellation(cancelled)?;
        Ok((staged_file, staged_fingerprint, publication))
    };
    let (staged_file, staged_fingerprint, publication) = match prepare() {
        Ok(result) => result,
        Err(error) => {
            abandon(storage, &intent)?;
            if error != FileError::RecoveryPending {
                stage.discard_payload(&staged)?;
                stage.discard_empty(&target.directory)?;
            }
            return Err(error);
        }
    };
    let prepared = PreparedReplacement {
        operation,
        intent,
        publication,
        target,
        stage,
        staged,
        staged_file,
        staged_fingerprint,
        metadata_committed: false,
    };
    if let Some(hook) = hooks.before_swap {
        hook();
    }
    if let Err(error) = check_cancellation(cancelled) {
        prepared.discard(storage)?;
        return Err(error);
    }
    let swap = native::swap(
        &prepared.stage.directory,
        OsStr::new(PAYLOAD),
        &prepared.target.directory,
        prepared.target.name(),
    );
    if let Err(error) = swap {
        if prepared.stage.assert_payload(&prepared.staged).is_ok()
            && prepared.target.verify_approval().is_ok()
        {
            prepared.discard(storage)?;
            return Err(error.into());
        }
        return Ok(ReplacementOutcome::RecoveryPending(Box::new(
            prepared.retain(storage),
        )));
    }
    if let Some(hook) = hooks.after_swap {
        hook();
    }
    let mut recovery = ReplacementRecovery {
        prepared: Some(prepared),
    };
    match recovery.finish(storage, || {
        if let Some(hook) = hooks.before_cleanup {
            hook();
        }
    }) {
        Ok((path, cleanup_pending)) => Ok(ReplacementOutcome::Completed {
            path,
            cleanup_pending,
        }),
        Err(_) => {
            let prepared = recovery.prepared.take().expect("unfinished replacement");
            Ok(ReplacementOutcome::RecoveryPending(Box::new(
                prepared.retain(storage),
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::{self, FileTimes, Permissions},
        io::Write,
        os::unix::fs::PermissionsExt,
        sync::atomic::Ordering,
    };

    struct Fixture {
        root: PathBuf,
        storage: Storage,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("folderai-replace-{}", Uuid::new_v4()));
            fs::create_dir_all(root.join("destination")).unwrap();
            fs::write(root.join("source.txt"), b"source").unwrap();
            fs::write(root.join("destination/target.txt"), b"before").unwrap();
            let storage = Storage::for_test(&root.join("data"), &root.join("cache")).unwrap();
            Self { root, storage }
        }

        fn source(&self) -> PathBuf {
            self.root.join("source.txt")
        }

        fn target(&self) -> PathBuf {
            self.root.join("destination/target.txt")
        }

        fn approval(&self) -> ReplacementTarget {
            ReplacementTarget::capture(&self.target()).unwrap()
        }

        fn attempt(
            &self,
            cancelled: &AtomicBool,
            hooks: Hooks<'_>,
        ) -> Result<ReplacementOutcome, FileError> {
            replace_file_with_hooks(
                &self.storage,
                &self.source(),
                self.approval(),
                cancelled,
                hooks,
            )
        }

        fn pending(&self, hooks: Hooks<'_>) -> Box<ReplacementRecovery> {
            match self.attempt(&AtomicBool::new(false), hooks).unwrap() {
                ReplacementOutcome::RecoveryPending(recovery) => recovery,
                ReplacementOutcome::Completed { .. } => panic!("expected pending recovery"),
            }
        }

        fn journal_state(&self) -> String {
            self.storage
                .access(|store| {
                    Ok(store.connection.query_row(
                        "SELECT state FROM operation_items WHERE kind='copy'",
                        [],
                        |row| row.get(0),
                    )?)
                })
                .unwrap()
        }

        fn stage(&self) -> PathBuf {
            fs::read_dir(self.target().parent().unwrap())
                .unwrap()
                .filter_map(Result::ok)
                .find(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".folderai-")
                })
                .unwrap()
                .path()
        }

        fn assert_no_stage(&self) {
            assert_eq!(
                fs::read_dir(self.target().parent().unwrap())
                    .unwrap()
                    .count(),
                1
            );
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn replaces_only_the_approved_file_and_gives_the_copy_a_new_identity() {
        let f = Fixture::new();
        let target = f.approval();
        let old = target.identity.comparison_key().unwrap();
        let source = LiveIdentity::open(&Locator::new(f.source()).unwrap()).unwrap();
        let result =
            replace_file(&f.storage, &f.source(), target, &AtomicBool::new(false)).unwrap();
        let ReplacementOutcome::Completed {
            path,
            cleanup_pending: false,
        } = result
        else {
            panic!("replacement did not complete");
        };
        let published = LiveIdentity::open(&Locator::new(path.clone()).unwrap()).unwrap();
        assert_ne!(published.comparison_key().unwrap(), old);
        assert_ne!(
            published.comparison_key().unwrap(),
            source.comparison_key().unwrap()
        );
        assert_eq!(fs::read(path).unwrap(), b"source");
        assert_eq!(fs::read(f.source()).unwrap(), b"source");
        assert_eq!(f.journal_state(), "completed");
        f.assert_no_stage();
    }

    #[test]
    fn cancellation_after_full_staging_preserves_the_approved_target() {
        let f = Fixture::new();
        let cancelled = AtomicBool::new(false);
        let result = f.attempt(
            &cancelled,
            Hooks {
                before_swap: Some(&|| cancelled.store(true, Ordering::Release)),
                ..Hooks::default()
            },
        );
        assert!(matches!(result, Err(FileError::Cancelled)));
        assert_eq!(fs::read(f.target()).unwrap(), b"before");
        assert_eq!(f.journal_state(), "failed");
        f.assert_no_stage();
    }

    #[test]
    fn staging_space_failure_cleans_the_partial_copy_and_preserves_the_target() {
        let f = Fixture::new();
        let result = f.attempt(
            &AtomicBool::new(false),
            Hooks {
                before_copy: Some(&|staged| {
                    staged.write_all(b"partial")?;
                    Err(io::Error::from_raw_os_error(libc::ENOSPC).into())
                }),
                ..Hooks::default()
            },
        );
        assert!(matches!(result, Err(FileError::StorageFull)));
        assert_eq!(fs::read(f.target()).unwrap(), b"before");
        assert_eq!(fs::read(f.source()).unwrap(), b"source");
        assert_eq!(f.journal_state(), "failed");
        f.assert_no_stage();
    }

    #[test]
    fn target_change_since_approval_prevents_publication() {
        let f = Fixture::new();
        let target = f.approval();
        fs::write(f.target(), b"edited").unwrap();
        let result = replace_file(&f.storage, &f.source(), target, &AtomicBool::new(false));
        assert!(matches!(result, Err(FileError::SourceChanged)));
        assert_eq!(fs::read(f.target()).unwrap(), b"edited");
        f.assert_no_stage();
    }

    #[test]
    fn substituted_target_at_swap_retains_all_three_objects() {
        let f = Fixture::new();
        let original = f.root.join("retained-target.txt");
        let mut recovery = f.pending(Hooks {
            before_swap: Some(&|| {
                fs::rename(f.target(), &original).unwrap();
                fs::write(f.target(), b"foreign").unwrap();
            }),
            ..Hooks::default()
        });
        assert_eq!(fs::read(&original).unwrap(), b"before");
        assert_eq!(fs::read(recovery.destination_path()).unwrap(), b"source");
        assert_eq!(fs::read(recovery.staging_path()).unwrap(), b"foreign");
        assert_eq!(f.journal_state(), "recovery");
        assert_eq!(recovery.retry(&f.storage), Err(FileError::RecoveryPending));
        assert_eq!(fs::read(recovery.staging_path()).unwrap(), b"foreign");
    }

    #[test]
    fn detects_an_edit_with_original_size_and_mtime_in_the_swap_gap() {
        let f = Fixture::new();
        let mut recovery = f.pending(Hooks {
            before_swap: Some(&|| {
                let file = File::options()
                    .read(true)
                    .write(true)
                    .open(f.target())
                    .unwrap();
                let originally_modified = file.metadata().unwrap().modified().unwrap();
                file.write_all_at(b"edited", 0).unwrap();
                file.set_times(FileTimes::new().set_modified(originally_modified))
                    .unwrap();
            }),
            ..Hooks::default()
        });
        assert_eq!(fs::read(recovery.staging_path()).unwrap(), b"edited");
        assert_eq!(fs::read(recovery.destination_path()).unwrap(), b"source");
        assert_eq!(recovery.retry(&f.storage), Err(FileError::RecoveryPending));
        assert_eq!(fs::read(recovery.staging_path()).unwrap(), b"edited");
    }

    #[test]
    fn retries_the_metadata_commit_with_the_same_published_object() {
        let f = Fixture::new();
        f.storage
            .access(|store| {
                Ok(store.connection.execute_batch(
                    "CREATE TRIGGER fail_replace_completion BEFORE UPDATE OF state ON operation_items
                     WHEN NEW.state='completed' BEGIN SELECT RAISE(ABORT, 'fault'); END;",
                )?)
            })
            .unwrap();
        let mut recovery = f.pending(Hooks::default());
        let output =
            LiveIdentity::open(&Locator::new(recovery.destination_path().to_owned()).unwrap())
                .unwrap();
        assert_eq!(f.journal_state(), "recovery");
        assert_eq!(fs::read(recovery.staging_path()).unwrap(), b"before");
        f.storage
            .access(|store| {
                Ok(store
                    .connection
                    .execute_batch("DROP TRIGGER fail_replace_completion;")?)
            })
            .unwrap();
        let (path, cleanup_pending) = recovery.retry(&f.storage).unwrap();
        assert!(!cleanup_pending);
        let current = LiveIdentity::open(&Locator::new(path).unwrap()).unwrap();
        assert_eq!(
            output.comparison_key().unwrap(),
            current.comparison_key().unwrap()
        );
        assert_eq!(f.journal_state(), "completed");
        f.assert_no_stage();
    }

    #[test]
    fn after_swap_edit_leaves_the_approved_backup_intact() {
        let f = Fixture::new();
        let mut recovery = f.pending(Hooks {
            after_swap: Some(&|| fs::write(f.target(), b"intervening edit").unwrap()),
            ..Hooks::default()
        });
        assert_eq!(
            fs::read(recovery.destination_path()).unwrap(),
            b"intervening edit"
        );
        assert_eq!(fs::read(recovery.staging_path()).unwrap(), b"before");
        assert_eq!(recovery.retry(&f.storage), Err(FileError::RecoveryPending));
        assert_eq!(f.journal_state(), "recovery");
    }

    #[test]
    fn cleanup_failure_keeps_the_backup_and_retries_without_republishing() {
        let f = Fixture::new();
        let mut recovery = f.pending(Hooks {
            before_cleanup: Some(&|| {
                fs::set_permissions(f.stage(), Permissions::from_mode(0o500)).unwrap();
            }),
            ..Hooks::default()
        });
        assert_eq!(f.journal_state(), "completed");
        assert_eq!(fs::read(recovery.staging_path()).unwrap(), b"before");
        assert_eq!(recovery.retry(&f.storage), Err(FileError::CleanupFailed));
        fs::set_permissions(f.stage(), Permissions::from_mode(0o700)).unwrap();
        let (path, cleanup_pending) = recovery.retry(&f.storage).unwrap();
        assert_eq!(fs::read(path).unwrap(), b"source");
        assert!(!cleanup_pending);
        f.assert_no_stage();
    }
}
