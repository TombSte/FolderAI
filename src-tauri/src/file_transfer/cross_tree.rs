use super::*;
use crate::{
    file_transfer::{TrackedMove, tracked_move},
    storage::{CrossMoveIntent, CrossMovePublication, CrossMoveRemoval},
};
use std::io::{Read, Seek, SeekFrom};

const QUARANTINED: &str = "source";

pub(in crate::file_transfer) enum CrossTreeOutcome {
    Completed(PathBuf, bool),
    Pending(Box<CrossTreeMove>),
}

pub(in crate::file_transfer) struct CrossTreeMove {
    destination: Directory,
    output: PathBuf,
    copied: StagedTree,
    origin: Directory,
    source_name: OsString,
    quarantine_name: OsString,
    quarantine_path: PathBuf,
    quarantine: Option<Directory>,
    relocated: bool,
    removed: Vec<bool>,
    tracked: TrackedMove,
    intents: Vec<CrossMoveIntent>,
    publications: Vec<CrossMovePublication>,
    claims: Vec<CrossMoveRemoval>,
}

impl CrossTreeMove {
    pub(in crate::file_transfer) fn path(&self) -> &Path {
        &self.output
    }

    pub(in crate::file_transfer) fn staging_path(&self) -> &Path {
        if self.relocated && !self.removed[0] {
            &self.quarantine_path
        } else {
            self.intents[0].source().path()
        }
    }

    pub(in crate::file_transfer) fn commit(
        &mut self,
        storage: &Storage,
    ) -> Result<(PathBuf, bool), FileError> {
        self.finish(storage, &AtomicBool::new(false))
    }

    fn finish(
        &mut self,
        storage: &Storage,
        cancelled: &AtomicBool,
    ) -> Result<(PathBuf, bool), FileError> {
        self.destination
            .verify()
            .map_err(|_| FileError::RecoveryPending)?;
        #[cfg(target_os = "macos")]
        self.destination.try_clone_file()?.sync_all()?;
        let published_root = self.intents[0].destination().path();
        verify_published(&self.copied, published_root)?;
        let references = std::iter::once(&self.tracked.root)
            .chain(self.tracked.descendants.iter().map(|(_, entry)| entry));
        for (index, (source, (intent, publication))) in references
            .clone()
            .zip(self.intents.iter().zip(&self.publications))
            .enumerate()
        {
            if index < self.claims.len() {
                self.claims[index].verify_before_removal(source, publication)?;
                continue;
            }
            let relative = intent
                .source()
                .path()
                .strip_prefix(self.intents[0].source().path())
                .map_err(|_| FileError::RecoveryPending)?;
            let backup = Locator::new(node_path(&self.quarantine_path, relative))
                .ok_or(FileError::RecoveryPending)?;
            let claim = storage.access(|store| {
                store.record_cross_volume_publication(intent, source, publication, Some(&backup))
            })?;
            self.claims.push(claim);
        }
        if !self.removed[0] {
            check_cancellation(cancelled)?;
            self.origin.verify()?;
            if !self.relocated {
                self.copied.source.verify(cancelled)?;
                if self.quarantine.is_none() {
                    self.quarantine = Some(
                        self.origin
                            .create_staging_directory(&self.quarantine_name)?
                            .map_err(|_| FileError::CleanupFailed)?,
                    );
                }
                let quarantine = self.quarantine.as_ref().ok_or(FileError::RecoveryPending)?;
                quarantine.verify()?;
                if !quarantine.entries()?.is_empty() {
                    return Err(FileError::RecoveryPending);
                }
                check_cancellation(cancelled)?;
                let native_result = native::publish(
                    &self.origin,
                    &self.source_name,
                    quarantine,
                    OsStr::new(QUARANTINED),
                );
                let root = source_directory(&self.copied.source.nodes[0])?;
                if root.relocate(self.quarantine_path.clone()).is_ok() {
                    self.relocated = true;
                } else {
                    native_result.map_err(FileError::from)?;
                    return Err(FileError::RecoveryPending);
                }
            }
            self.copied.source.remove_from_quarantine(
                &self.quarantine_path,
                self.quarantine.as_ref().ok_or(FileError::RecoveryPending)?,
                OsStr::new(QUARANTINED),
                &mut self.removed,
                cancelled,
                |index| {
                    self.destination.verify()?;
                    let node = &self.copied.source.nodes[index];
                    let published_location = node_path(published_root, &node.relative);
                    let output = &self.copied.nodes[index];
                    match &node.kind {
                        SourceKind::File { file, version, .. } => {
                            let (published, expected) = output
                                .file_version
                                .as_ref()
                                .ok_or(FileError::RecoveryPending)?;
                            check_file_at(
                                published,
                                &output.identity,
                                &published_location,
                                *expected,
                            )?;
                            verify_matching_contents(file, published, *version, cancelled)?;
                            check_file_at(
                                published,
                                &output.identity,
                                &published_location,
                                *expected,
                            )?;
                        }
                        SourceKind::Directory { .. } => {
                            self.copied.directories[index]
                                .as_ref()
                                .ok_or(FileError::RecoveryPending)?
                                .relocate(published_location)?;
                        }
                    }
                    Ok(())
                },
            )?;
        }
        for (((source, intent), publication), claim) in references
            .zip(&self.intents)
            .zip(&self.publications)
            .zip(&self.claims)
        {
            storage.access(|store| {
                store.complete_cross_volume_move(intent, source, publication, claim)
            })?;
        }
        self.destination
            .verify()
            .map_err(|_| FileError::RecoveryPending)?;
        let cleanup_pending = self.quarantine.take().is_some_and(|quarantine| {
            discard_empty_stage(&quarantine, &self.origin, &self.quarantine_name).is_err()
        });
        Ok((self.output.clone(), cleanup_pending))
    }
}

fn source_directory(node: &SourceNode) -> Result<&Directory, FileError> {
    match &node.kind {
        SourceKind::Directory { directory, .. } => Ok(directory),
        SourceKind::File { .. } => Err(FileError::RecoveryPending),
    }
}

fn check_file_at(
    file: &File,
    identity: &LiveIdentity,
    expected: &Path,
    original: Version,
) -> Result<(), FileError> {
    if identity.observe()?
        != LiveObservation::Present(
            Locator::new(expected.to_owned()).ok_or(FileError::RecoveryPending)?,
        )
        || version(&file.metadata()?)? != original
    {
        return Err(FileError::RecoveryPending);
    }
    Ok(())
}

fn verify_published(tree: &StagedTree, root: &Path) -> Result<(), FileError> {
    for ((output, source), directory) in tree
        .nodes
        .iter()
        .zip(&tree.source.nodes)
        .zip(&tree.directories)
    {
        let location = node_path(root, &output.relative);
        match (&output.file_version, &source.kind) {
            (Some((file, initial)), SourceKind::File { .. }) => {
                check_file_at(file, &output.identity, &location, *initial)?
            }
            (None, SourceKind::Directory { entries, .. }) => {
                let directory = directory
                    .as_ref()
                    .ok_or(FileError::RecoveryPending)?
                    .relocate(location)?;
                if sorted_entries(&directory)? != *entries {
                    return Err(FileError::RecoveryPending);
                }
            }
            _ => return Err(FileError::RecoveryPending),
        }
    }
    Ok(())
}

fn verify_matching_contents(
    source: &File,
    destination: &File,
    expected: Version,
    cancelled: &AtomicBool,
) -> Result<(), FileError> {
    let mut source = source.try_clone()?;
    let mut destination = destination.try_clone()?;
    source.seek(SeekFrom::Start(0))?;
    destination.seek(SeekFrom::Start(0))?;
    let mut left = vec![0; 256 * 1024];
    let mut right = vec![0; 256 * 1024];
    let mut remaining = expected.0;
    while remaining != 0 {
        check_cancellation(cancelled)?;
        let count = remaining.min(left.len() as u64) as usize;
        source.read_exact(&mut left[..count])?;
        destination.read_exact(&mut right[..count])?;
        if left[..count] != right[..count] {
            return Err(FileError::SourceChanged);
        }
        remaining -= count as u64;
    }
    if version(&source.metadata()?)? != expected || version(&destination.metadata()?)? != expected {
        return Err(FileError::SourceChanged);
    }
    Ok(())
}

fn abandon(
    storage: &Storage,
    intents: &[CrossMoveIntent],
    publications: &[CrossMovePublication],
) -> Result<(), FileError> {
    storage
        .access(|store| store.fail_unpublished_cross_volume_move(intents, publications))
        .map_err(|_| FileError::RecoveryPending)
}

fn bind_tree(
    staged: &StagedTree,
    intents: &[CrossMoveIntent],
    bound: &mut Vec<CrossMovePublication>,
) -> Result<(), FileError> {
    let source = intents[0].source().path();
    let entries: std::collections::HashMap<_, _> = staged
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.relative.as_path(), index))
        .collect();
    for intent in intents {
        let relative = intent
            .source()
            .path()
            .strip_prefix(source)
            .map_err(|_| FileError::SourceChanged)?;
        let index = *entries.get(relative).ok_or(FileError::SourceChanged)?;
        let file = match (
            &staged.nodes[index].file_version,
            &staged.directories[index],
        ) {
            (Some((file, _)), _) => file.try_clone()?,
            (None, Some(directory)) => directory.try_clone_file()?,
            _ => return Err(FileError::RecoveryPending),
        };
        bound.push(intent.bind_staged_destination(LiveIdentity::from_file(file)?)?);
    }
    Ok(())
}

pub(in crate::file_transfer) fn move_tree(
    storage: &Storage,
    path: &Path,
    destination: &Directory,
    operation: (&str, &str),
    cancelled: &AtomicBool,
) -> Result<CrossTreeOutcome, FileError> {
    move_tree_with_hook(storage, path, destination, operation, cancelled, || {})
}

fn move_tree_with_hook(
    storage: &Storage,
    path: &Path,
    destination: &Directory,
    operation: (&str, &str),
    cancelled: &AtomicBool,
    after_publication: impl FnOnce(),
) -> Result<CrossTreeOutcome, FileError> {
    let source_name = path
        .file_name()
        .ok_or(FileError::InvalidRequest)?
        .to_owned();
    let source = SourceTree::capture(path, cancelled)?;
    let root = source_directory(&source.nodes[0])?;
    let source_location = Locator::new(root.path().to_owned()).ok_or(FileError::InvalidRequest)?;
    let source_parent = root.path().parent().ok_or(FileError::InvalidRequest)?;
    let origin = Directory::open(source_parent)?;
    let destination_path = captured_directory_path(destination)?;
    if destination_path.starts_with(root.path()) {
        return Err(FileError::InvalidRequest);
    }
    let root_identity = LiveIdentity::from_file(root.try_clone_file()?)?;
    let tracked = storage.access(|store| {
        let tracked = tracked_move(store, &source_location)?;
        if tracked.root.matches_open_identity(&root_identity)? {
            Ok(tracked)
        } else {
            Err(StorageError::ReferenceConflict)
        }
    })?;
    let descendants: Vec<_> = tracked.descendants.iter().map(|(_, entry)| entry).collect();
    let quarantine_name = OsString::from(format!(".folderai-move-{}", Uuid::new_v4()));
    let quarantine_path = source_parent.join(&quarantine_name).join(QUARANTINED);
    let temporary_name = OsString::from(format!(".folderai-{}", Uuid::new_v4()));
    let mut pending_stage: Option<(Directory, StagedTree)> = None;
    let mut source = Some(source);
    for sequence in 0..10_000 {
        let candidate = (|| {
            check_cancellation(cancelled)?;
            destination.verify()?;
            let name = copy_name(&source_name, sequence, true);
            let target =
                Locator::new(destination_path.join(&name)).ok_or(FileError::InvalidRequest)?;
            match fs::symlink_metadata(target.path()) {
                Ok(_) => return Ok(None),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            let intents = storage.access(|store| {
                if store.copy_destination_reserved(&target)? {
                    return Ok(None);
                }
                match store.record_cross_volume_move(
                    operation.0,
                    &format!("{}:{sequence}", operation.1),
                    &tracked.root,
                    &descendants,
                    &target,
                ) {
                    Err(StorageError::ReferenceConflict)
                        if store.copy_destination_reserved(&target)? =>
                    {
                        Ok(None)
                    }
                    result => result.map(Some),
                }
            })?;
            Ok(intents.map(|intents| (name, intents)))
        })();
        let (name, intents) = match candidate {
            Ok(Some(candidate)) => candidate,
            Ok(None) => continue,
            Err(error) => {
                if let Some((stage, staged)) = &pending_stage {
                    staged.discard(stage, destination, &temporary_name)?;
                }
                return Err(error);
            }
        };
        if let Some(source) = source.take() {
            let stage = match destination.create_staging_directory(&temporary_name) {
                Ok(Ok(stage)) => stage,
                Err(error) => {
                    abandon(storage, &intents, &[])?;
                    return Err(error.into());
                }
                Ok(Err(_)) => {
                    abandon(storage, &intents, &[])?;
                    return Err(FileError::CleanupFailed);
                }
            };
            match source.stage(&stage, cancelled) {
                Ok(staged) => pending_stage = Some((stage, staged)),
                Err(failure) => {
                    abandon(storage, &intents, &[])?;
                    if failure.pending.is_some() || failure.error == FileError::RecoveryPending {
                        return Err(FileError::CleanupFailed);
                    }
                    if let Some(partial) = failure.partial {
                        partial.discard(&stage, destination, &temporary_name)?;
                    } else {
                        discard_empty_stage(&stage, destination, &temporary_name)?;
                    }
                    return Err(failure.error);
                }
            }
        }
        let (stage, staged) = pending_stage.as_ref().ok_or(FileError::RecoveryPending)?;
        let mut publications = Vec::new();
        let ready = (|| {
            bind_tree(staged, &intents, &mut publications)?;
            staged.verify(stage, cancelled)?;
            let retained = destination.duplicate()?;
            destination.verify()?;
            origin.verify()?;
            check_cancellation(cancelled)?;
            Ok::<_, FileError>(retained)
        })();
        let retained = match ready {
            Ok(retained) => retained,
            Err(error) => {
                abandon(storage, &intents, &publications)?;
                if error != FileError::RecoveryPending {
                    staged.discard(stage, destination, &temporary_name)?;
                }
                return Err(error);
            }
        };
        if let Err(error) = native::publish(destination, &temporary_name, destination, &name)
            && staged.verify_output(stage, &AtomicBool::new(false)).is_ok()
        {
            abandon(storage, &intents, &publications)?;
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                continue;
            }
            staged.discard(stage, destination, &temporary_name)?;
            return Err(error.into());
        }
        let (_, copied) = pending_stage.take().ok_or(FileError::RecoveryPending)?;
        let mut movement = CrossTreeMove {
            destination: retained,
            output: destination.path().join(name),
            removed: vec![false; copied.source.nodes.len()],
            copied,
            origin,
            source_name,
            quarantine_name,
            quarantine_path,
            quarantine: None,
            relocated: false,
            tracked,
            intents,
            publications,
            claims: Vec::new(),
        };
        after_publication();
        return match movement.finish(storage, cancelled) {
            Ok((path, cleanup)) => Ok(CrossTreeOutcome::Completed(path, cleanup)),
            Err(_) => Ok(CrossTreeOutcome::Pending(Box::new(movement))),
        };
    }
    if let Some((stage, staged)) = &pending_stage {
        staged.discard(stage, destination, &temporary_name)?;
    }
    Err(FileError::AlreadyExists)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        root: PathBuf,
        destination: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let name = format!("folderai-cross-tree-{}", Uuid::new_v4());
            let root = std::env::temp_dir().join(&name);
            let destination = std::env::var_os("FOLDERAI_TEST_SECOND_VOLUME")
                .map(PathBuf::from)
                .map_or_else(|| root.join("destination"), |volume| volume.join(name));
            fs::create_dir_all(root.join("origin/Project/empty")).unwrap();
            fs::create_dir_all(&destination).unwrap();
            fs::write(root.join("origin/Project/records.txt"), b"important").unwrap();
            Self { root, destination }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.destination);
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn cross_tree_resumes_after_publication_stop_and_preserves_descendant_id() {
        let f = Fixture::new();
        let storage = Storage::for_test(&f.root.join("data"), &f.root.join("cache")).unwrap();
        let source = f.root.join("origin/Project");
        let destination = Directory::open(&f.destination).unwrap();
        if std::env::var_os("FOLDERAI_TEST_SECOND_VOLUME").is_some() {
            let origin = Directory::open(source.parent().unwrap()).unwrap();
            assert!(!origin.same_volume(&destination).unwrap());
        }
        let original = Locator::new(canonical_path(&source.join("records.txt")).unwrap()).unwrap();
        let original_id = storage
            .access(|store| store.resolve_live_item(&original, true))
            .unwrap()
            .unwrap();
        let cancelled = AtomicBool::new(false);
        let moved = move_tree_with_hook(
            &storage,
            &source,
            &destination,
            ("fixture-move", "0"),
            &cancelled,
            || cancelled.store(true, std::sync::atomic::Ordering::Release),
        )
        .unwrap();
        let CrossTreeOutcome::Pending(mut pending) = moved else {
            panic!("cancellation should preserve the original after publication");
        };
        assert_eq!(fs::read(source.join("records.txt")).unwrap(), b"important");
        let published = f.destination.join("Project");
        assert_eq!(
            fs::read(published.join("records.txt")).unwrap(),
            b"important"
        );
        assert_eq!(pending.commit(&storage), Ok((published.clone(), false)));
        assert!(!source.exists());
        assert!(published.join("empty").is_dir());
        let target = Locator::new(canonical_path(&published.join("records.txt")).unwrap()).unwrap();
        assert_eq!(
            storage.access(|store| store.resolve_live_item(&target, false)),
            Ok(Some(original_id))
        );
        assert_eq!(f.destination.read_dir().unwrap().count(), 1);
        assert_eq!(f.root.join("origin").read_dir().unwrap().count(), 0);
    }
}
