//! Recursive copies stage under an exclusive temporary name, journal each
//! entry, then publish the whole tree with one native rename.

#[path = "cross_tree.rs"]
pub(super) mod cross_tree;

use super::{
    FileError, captured_directory_path,
    copy::{check_cancellation, copy_chunks, copy_name, version},
    native::{self, Directory, EntryKind},
};
use crate::{
    local_identity::{LiveIdentity, LiveObservation, Locator, canonical_path},
    storage::{CopyIntent, CopyKey, CopyPublication, Storage, StorageError},
};
use std::{
    ffi::{OsStr, OsString},
    fs::{self, File, FileTimes},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::SystemTime,
};
use uuid::Uuid;

type Version = (u64, SystemTime);

struct SourceNode {
    parent: Option<usize>,
    name: OsString,
    relative: PathBuf,
    kind: SourceKind,
}

enum SourceKind {
    Directory {
        directory: Directory,
        entries: Vec<(OsString, EntryKind)>,
    },
    File {
        file: File,
        identity: LiveIdentity,
        version: Version,
        expected: Locator,
    },
}

pub(super) struct SourceTree {
    nodes: Vec<SourceNode>,
}

/// A retained staging witness for each node. These must be bound to the
/// corresponding copy intents before publication, not reopened by path later.
struct StagedNode {
    relative: PathBuf,
    identity: LiveIdentity,
    origin: WitnessOrigin,
    file_version: Option<(File, Version)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WitnessOrigin {
    ProvidedRoot,
    ExclusiveFile,
    WitnessedDirectory,
}

/// A creation succeeded but obtaining a usable identity did not. Never
/// remove this entry based on its name; retain any handle we did acquire.
struct PendingNode {
    relative: PathBuf,
    file: Option<File>,
    directory: Option<Directory>,
}

struct StageFailure {
    error: FileError,
    partial: Option<Box<StagedTree>>,
    pending: Option<Box<PendingNode>>,
}

pub(super) enum TreeOutcome {
    Completed(PathBuf, bool),
    Pending(PendingTreeCopy),
}

pub(super) struct PendingTreeCopy {
    destination: Directory,
    visible_path: PathBuf,
    journal: TreeJournal,
    bound: Vec<BoundItem>,
}

impl PendingTreeCopy {
    pub(super) fn path(&self) -> &Path {
        &self.visible_path
    }

    pub(super) fn commit(&self, storage: &Storage) -> Result<(PathBuf, bool), FileError> {
        self.destination
            .verify()
            .map_err(|_| FileError::RecoveryPending)?;
        let mut pending = false;
        for (item, bound) in self.journal.items.iter().zip(&self.bound) {
            debug_assert_eq!(item.relative, bound.relative);
            if storage
                .access(|store| store.record_completed_copy(&item.intent, &bound.publication))
                .is_err()
                || !matches!(
                    bound.publication.is_present_at(item.intent.destination()),
                    Ok(true)
                )
            {
                pending = true;
            }
        }
        if pending {
            return Err(FileError::RecoveryPending);
        }
        self.destination
            .verify()
            .map_err(|_| FileError::RecoveryPending)?;
        Ok((self.visible_path.clone(), false))
    }
}

impl StageFailure {
    fn before_writing(error: FileError) -> Self {
        Self {
            error,
            partial: None,
            pending: None,
        }
    }
}

impl std::fmt::Debug for StageFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StageFailure")
            .field("error", &self.error)
            .field(
                "retained_nodes",
                &self.partial.as_ref().map(|tree| tree.nodes.len()),
            )
            .field(
                "pending",
                &self.pending.as_ref().map(|entry| &entry.relative),
            )
            .finish()
    }
}

struct StagedTree {
    nodes: Vec<StagedNode>,
    source: SourceTree,
    directories: Vec<Option<Directory>>,
}

struct RecordedIntent {
    relative: PathBuf,
    item_key: String,
    intent: CopyIntent,
}

struct TreeJournal {
    operation_id: String,
    destination: PathBuf,
    items: Vec<RecordedIntent>,
}

struct JournalFailure {
    error: FileError,
    journal: TreeJournal,
}

struct BoundItem {
    relative: PathBuf,
    publication: CopyPublication,
}

struct BindingFailure {
    error: FileError,
    bound: Vec<BoundItem>,
}

impl TreeJournal {
    fn mark_recovery(&self, storage: &Storage) {
        for item in &self.items {
            let _ = storage.access(|store| {
                store.recover_copy(CopyKey {
                    operation_id: &self.operation_id,
                    item_key: &item.item_key,
                })
            });
        }
    }

    fn abandon(&self, storage: &Storage) -> Result<(), FileError> {
        let mut pending = false;
        for item in &self.items {
            if storage
                .access(|store| store.fail_copy(&item.intent))
                .is_err()
            {
                pending = true;
            }
        }
        if pending {
            Err(FileError::RecoveryPending)
        } else {
            Ok(())
        }
    }
}

impl std::fmt::Debug for BindingFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BindingFailure")
            .field("error", &self.error)
            .field("bound_count", &self.bound.len())
            .finish()
    }
}

impl std::fmt::Debug for JournalFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("JournalFailure")
            .field("error", &self.error)
            .field("recorded_count", &self.journal.items.len())
            .finish()
    }
}

fn node_path(root: &Path, relative: &Path) -> PathBuf {
    if relative.as_os_str().is_empty() {
        root.to_owned()
    } else {
        root.join(relative)
    }
}

fn verify_file_at(
    file: &File,
    identity: &LiveIdentity,
    expected: &Path,
    original: Version,
) -> Result<(), FileError> {
    let expected = Locator::new(expected.to_owned()).ok_or(FileError::RecoveryPending)?;
    if identity.observe()? != LiveObservation::Present(expected)
        || version(&file.metadata()?)? != original
    {
        return Err(FileError::SourceChanged);
    }
    Ok(())
}

fn sorted_entries(directory: &Directory) -> Result<Vec<(OsString, EntryKind)>, FileError> {
    let mut entries = directory.entries()?;
    entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    Ok(entries)
}

impl SourceTree {
    pub(super) fn capture(path: &Path, cancelled: &AtomicBool) -> Result<Self, FileError> {
        check_cancellation(cancelled)?;
        // Do not silently follow a link selected as the root.
        if !fs::symlink_metadata(path)?.is_dir() {
            return Err(FileError::Unsupported);
        }
        let chosen_root = Directory::open(path)?;
        let canonical = captured_directory_path(&chosen_root)?;
        let root = Directory::open(&canonical)?;
        let chosen = LiveIdentity::from_file(chosen_root.try_clone_file()?)?;
        let opened = LiveIdentity::from_file(root.try_clone_file()?)?;
        if chosen.comparison_key()? != opened.comparison_key()? {
            return Err(FileError::SourceChanged);
        }
        let mut nodes = vec![SourceNode {
            parent: None,
            name: OsString::new(),
            relative: PathBuf::new(),
            kind: SourceKind::Directory {
                directory: root,
                entries: Vec::new(),
            },
        }];
        let mut index = 0;
        while index < nodes.len() {
            check_cancellation(cancelled)?;
            let entries = match &nodes[index].kind {
                SourceKind::Directory { directory, .. } => {
                    directory.verify().map_err(|_| FileError::SourceChanged)?;
                    sorted_entries(directory)?
                }
                SourceKind::File { .. } => {
                    index += 1;
                    continue;
                }
            };
            if entries
                .iter()
                .any(|(_, kind)| *kind == EntryKind::Unsupported)
            {
                return Err(FileError::Unsupported);
            }
            for (name, kind) in &entries {
                check_cancellation(cancelled)?;
                let SourceKind::Directory { directory, .. } = &nodes[index].kind else {
                    unreachable!();
                };
                let relative = nodes[index].relative.join(name);
                let node_kind = match kind {
                    EntryKind::Directory => SourceKind::Directory {
                        directory: directory.open_directory(name)?,
                        entries: Vec::new(),
                    },
                    EntryKind::RegularFile => {
                        let file = directory.open_file(name)?;
                        let initial = version(&file.metadata()?)?;
                        let identity = LiveIdentity::from_file(file.try_clone()?)?;
                        let expected = Locator::new(directory.path().join(name))
                            .ok_or(FileError::InvalidRequest)?;
                        if identity.observe()? != LiveObservation::Present(expected.clone()) {
                            return Err(FileError::SourceChanged);
                        }
                        SourceKind::File {
                            file,
                            identity,
                            version: initial,
                            expected,
                        }
                    }
                    EntryKind::Unsupported => unreachable!(),
                };
                nodes.push(SourceNode {
                    parent: Some(index),
                    name: name.clone(),
                    relative,
                    kind: node_kind,
                });
            }
            if let SourceKind::Directory { entries: saved, .. } = &mut nodes[index].kind {
                *saved = entries;
            }
            index += 1;
        }
        let tree = Self { nodes };
        tree.verify(cancelled)?;
        Ok(tree)
    }

    pub(super) fn count(&self) -> usize {
        self.nodes.len()
    }

    pub(super) fn remove_from_quarantine(
        &self,
        root: &Path,
        quarantine: &Directory,
        root_name: &OsStr,
        removed: &mut [bool],
        cancelled: &AtomicBool,
        mut verify_effect: impl FnMut(usize) -> Result<(), FileError>,
    ) -> Result<(), FileError> {
        quarantine.verify()?;
        let mut captured = Vec::with_capacity(self.nodes.len());
        let mut expected = vec![Vec::new(); self.nodes.len()];
        for (index, node) in self.nodes.iter().enumerate() {
            check_cancellation(cancelled)?;
            if removed[index] {
                captured.push(None);
                continue;
            }
            let resolved = node_path(root, &node.relative);
            let (directory, kind) = match &node.kind {
                SourceKind::File {
                    file,
                    identity,
                    version: original,
                    ..
                } => {
                    verify_file_at(file, identity, &resolved, *original)?;
                    (None, EntryKind::RegularFile)
                }
                SourceKind::Directory { directory, .. } => {
                    (Some(directory.relocate(resolved)?), EntryKind::Directory)
                }
            };
            if let Some(parent) = node.parent {
                expected[parent].push((node.name.clone(), kind));
            }
            captured.push(directory);
        }
        for (directory, entries) in captured.iter().zip(expected) {
            if let Some(directory) = directory
                && sorted_entries(directory)? != entries
            {
                return Err(FileError::RecoveryPending);
            }
        }
        for index in (0..self.nodes.len()).rev() {
            if removed[index] {
                continue;
            }
            check_cancellation(cancelled)?;
            verify_effect(index)?;
            let node = &self.nodes[index];
            let (parent, name) = if let Some(parent) = node.parent {
                (
                    captured[parent]
                        .as_ref()
                        .ok_or(FileError::RecoveryPending)?,
                    node.name.as_os_str(),
                )
            } else {
                (quarantine, root_name)
            };
            parent.verify()?;
            match &node.kind {
                SourceKind::File {
                    file,
                    identity,
                    version: original,
                    ..
                } => {
                    verify_file_at(file, identity, &node_path(root, &node.relative), *original)?;
                    native::remove_file(parent, name)?;
                }
                SourceKind::Directory { .. } => {
                    let directory = captured[index].as_ref().ok_or(FileError::RecoveryPending)?;
                    directory.verify()?;
                    native::remove_directory(parent, name)?;
                }
            }
            removed[index] = true;
        }
        Ok(())
    }

    pub(super) fn matches_root(&self, identity: &LiveIdentity) -> Result<bool, FileError> {
        let SourceKind::Directory { directory, .. } = &self.nodes[0].kind else {
            return Ok(false);
        };
        let root = LiveIdentity::from_file(directory.try_clone_file()?)?;
        Ok(root.comparison_key()? == identity.comparison_key()?)
    }

    pub(super) fn verify(&self, cancelled: &AtomicBool) -> Result<(), FileError> {
        for node in &self.nodes {
            check_cancellation(cancelled)?;
            match &node.kind {
                SourceKind::Directory { directory, entries } => {
                    directory.verify().map_err(|_| FileError::SourceChanged)?;
                    if sorted_entries(directory)? != *entries {
                        return Err(FileError::SourceChanged);
                    }
                }
                SourceKind::File {
                    file,
                    identity,
                    version: initial,
                    expected,
                } => {
                    if version(&file.metadata()?)? != *initial
                        || identity.observe()? != LiveObservation::Present(expected.clone())
                    {
                        return Err(FileError::SourceChanged);
                    }
                }
            }
        }
        Ok(())
    }

    /// Reserve one durable intent per node before creating staging entries.
    /// If preparation fails, the caller retains every previously recorded
    /// intent and must explicitly fail it only while no publication occurred.
    #[cfg(test)]
    fn record_intents(
        &self,
        storage: &Storage,
        destination: &Path,
        operation_id: &str,
        cancelled: &AtomicBool,
    ) -> Result<TreeJournal, JournalFailure> {
        self.record_intents_with_hook(storage, destination, (operation_id, ""), cancelled, |_| {})
    }

    fn record_intents_with_hook(
        &self,
        storage: &Storage,
        destination: &Path,
        (operation_id, item_prefix): (&str, &str),
        cancelled: &AtomicBool,
        mut after_intent: impl FnMut(usize),
    ) -> Result<TreeJournal, JournalFailure> {
        let mut journal = TreeJournal {
            operation_id: operation_id.to_owned(),
            destination: destination.to_owned(),
            items: Vec::new(),
        };
        let outcome = (|| -> Result<(), FileError> {
            self.verify(cancelled)?;
            let name = destination.file_name().ok_or(FileError::InvalidRequest)?;
            let parent = destination.parent().ok_or(FileError::InvalidRequest)?;
            let parent = canonical_path(parent)?;
            let root_destination = parent.join(name);
            let SourceKind::Directory {
                directory: source_root,
                ..
            } = &self.nodes[0].kind
            else {
                unreachable!();
            };
            if root_destination != source_root.path()
                && root_destination.starts_with(source_root.path())
            {
                return Err(FileError::InvalidRequest);
            }
            match fs::symlink_metadata(&root_destination) {
                Ok(_) => return Err(FileError::AlreadyExists),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            journal.destination = root_destination;
            for (index, node) in self.nodes.iter().enumerate() {
                check_cancellation(cancelled)?;
                let identity = match &node.kind {
                    SourceKind::Directory { directory, .. } => {
                        LiveIdentity::from_file(directory.try_clone_file()?)?
                    }
                    SourceKind::File { file, .. } => LiveIdentity::from_file(file.try_clone()?)?,
                };
                let source = Locator::new(node_path(source_root.path(), &node.relative))
                    .ok_or(FileError::InvalidRequest)?;
                if identity.observe()? != LiveObservation::Present(source.clone()) {
                    return Err(FileError::SourceChanged);
                }
                let target = Locator::new(node_path(&journal.destination, &node.relative))
                    .ok_or(FileError::InvalidRequest)?;
                let item_key = format!("{item_prefix}{index}");
                let intent = storage.access(|store| {
                    if store.copy_destination_reserved(&target)? {
                        return Ok(None);
                    }
                    let id = store
                        .resolve_live_item(&source, true)?
                        .ok_or(StorageError::ReferenceConflict)?;
                    let reference = store.attach_live_reference(&id, &source)?;
                    if !reference.matches_open_identity(&identity)? {
                        return Err(StorageError::ReferenceConflict);
                    }
                    let intent = store.record_copy_intent(
                        CopyKey {
                            operation_id,
                            item_key: &item_key,
                        },
                        &reference,
                        &target,
                    );
                    match intent {
                        Err(StorageError::ReferenceConflict)
                            if store.copy_destination_reserved(&target)? =>
                        {
                            Ok(None)
                        }
                        result => result.map(Some),
                    }
                })?;
                let Some(intent) = intent else {
                    return Err(FileError::AlreadyExists);
                };
                journal.items.push(RecordedIntent {
                    relative: node.relative.clone(),
                    item_key,
                    intent,
                });
                after_intent(index);
            }
            self.verify(cancelled)?;
            Ok(())
        })();
        match outcome {
            Ok(()) => Ok(journal),
            Err(error) => Err(JournalFailure { error, journal }),
        }
    }

    /// Populate an already-created empty staging directory. The directory must
    /// belong exclusively to this operation and must be outside the source.
    /// No rollback is attempted here, even on cancellation or I/O failure.
    fn stage(self, stage: &Directory, cancelled: &AtomicBool) -> Result<StagedTree, StageFailure> {
        self.stage_with_hook(stage, cancelled, |_| {})
    }

    // Deterministic fault injection after an entry has been staged.
    fn stage_with_hook(
        self,
        stage: &Directory,
        cancelled: &AtomicBool,
        after_node: impl FnMut(&Path),
    ) -> Result<StagedTree, StageFailure> {
        self.stage_with_hooks(stage, cancelled, after_node, |_| Ok(()), copy_chunks)
    }

    fn stage_with_hooks(
        mut self,
        stage: &Directory,
        cancelled: &AtomicBool,
        mut after_node: impl FnMut(&Path),
        mut after_create: impl FnMut(&Path) -> Result<(), FileError>,
        mut copy: impl FnMut(&mut File, &mut File, &AtomicBool) -> Result<u64, FileError>,
    ) -> Result<StagedTree, StageFailure> {
        let prepared = (|| -> Result<_, FileError> {
            self.verify(cancelled)?;
            stage.verify().map_err(|_| FileError::RecoveryPending)?;
            let SourceKind::Directory {
                directory: root, ..
            } = &self.nodes[0].kind
            else {
                unreachable!();
            };
            if canonical_path(stage.path())?.starts_with(root.path())
                || !stage.entries()?.is_empty()
            {
                return Err(FileError::InvalidRequest);
            }
            // Reopen through the verified path, then compare live identities.
            let staged_root = Directory::open(stage.path())?;
            let root_identity = LiveIdentity::from_file(stage.try_clone_file()?)?;
            let reopened = LiveIdentity::from_file(staged_root.try_clone_file()?)?;
            if root_identity.comparison_key()? != reopened.comparison_key()? {
                return Err(FileError::RecoveryPending);
            }
            Ok((staged_root, root_identity))
        })();
        let (staged_root, root_identity) = prepared.map_err(StageFailure::before_writing)?;
        let mut directories = vec![Some(staged_root)];
        let mut staged = vec![StagedNode {
            relative: PathBuf::new(),
            identity: root_identity,
            origin: WitnessOrigin::ProvidedRoot,
            file_version: None,
        }];
        let mut pending = None;
        let outcome = (|| -> Result<(), FileError> {
            for node in self.nodes.iter_mut().skip(1) {
                check_cancellation(cancelled)?;
                let parent = directories[node.parent.expect("only the root has no parent")]
                    .as_ref()
                    .ok_or(FileError::RecoveryPending)?;
                parent.verify().map_err(|_| FileError::RecoveryPending)?;
                match &mut node.kind {
                    SourceKind::Directory { .. } => {
                        let child = parent.create_staging_directory(&node.name)?;
                        pending = Some(PendingNode {
                            relative: node.relative.clone(),
                            file: None,
                            directory: child.ok(),
                        });
                        after_create(&node.relative)?;
                        let entry = pending.as_mut().unwrap();
                        let identity = LiveIdentity::from_file(
                            entry
                                .directory
                                .as_ref()
                                .ok_or(FileError::CleanupFailed)?
                                .try_clone_file()?,
                        )?;
                        let child = pending.take().unwrap().directory.unwrap();
                        directories.push(Some(child));
                        staged.push(StagedNode {
                            relative: node.relative.clone(),
                            identity,
                            origin: WitnessOrigin::WitnessedDirectory,
                            file_version: None,
                        });
                    }
                    SourceKind::File {
                        file,
                        version: initial,
                        ..
                    } => {
                        let output = parent.create_file(&node.name)?;
                        pending = Some(PendingNode {
                            relative: node.relative.clone(),
                            file: Some(output),
                            directory: None,
                        });
                        after_create(&node.relative)?;
                        let identity = LiveIdentity::from_file(
                            pending
                                .as_ref()
                                .unwrap()
                                .file
                                .as_ref()
                                .unwrap()
                                .try_clone()?,
                        )?;
                        let output = pending.take().unwrap().file.unwrap();
                        directories.push(None);
                        staged.push(StagedNode {
                            relative: node.relative.clone(),
                            identity,
                            origin: WitnessOrigin::ExclusiveFile,
                            file_version: Some((output, *initial)),
                        });
                        // Save the live witness before writing even the first byte.
                        let (output, _) = staged.last_mut().unwrap().file_version.as_mut().unwrap();
                        if copy(file, output, cancelled)? != initial.0 {
                            return Err(FileError::SourceChanged);
                        }
                        output.set_times(FileTimes::new().set_modified(initial.1))?;
                        output.set_permissions(file.metadata()?.permissions())?;
                        output.sync_all()?;
                    }
                }
                after_node(&node.relative);
            }
            // Creating a child changes its parent's timestamp.
            for (node, output) in self.nodes.iter().zip(&directories).rev() {
                check_cancellation(cancelled)?;
                if let (SourceKind::Directory { directory, .. }, Some(output)) =
                    (&node.kind, output)
                {
                    let modified = directory.try_clone_file()?.metadata()?.modified()?;
                    output.set_modified(modified)?;
                }
            }
            Ok(())
        })();
        let result = StagedTree {
            nodes: staged,
            source: self,
            directories,
        };
        if let Err(error) = outcome {
            return Err(StageFailure {
                error,
                partial: Some(Box::new(result)),
                pending: pending.map(Box::new),
            });
        }
        if let Err(error) = result.verify(stage, cancelled) {
            return Err(StageFailure {
                error,
                partial: Some(Box::new(result)),
                pending: None,
            });
        }
        Ok(result)
    }
}

impl StagedTree {
    /// Bind each intent to a duplicate of the retained staging handle. A
    /// failure returns every already-bound publication; the caller must keep
    /// it alongside the journal and tree instead of attempting to bind again.
    fn bind_publications(
        &self,
        stage: &Directory,
        journal: &TreeJournal,
        cancelled: &AtomicBool,
    ) -> Result<Vec<BoundItem>, BindingFailure> {
        self.bind_publications_with_hook(stage, journal, cancelled, |_| Ok(()))
    }

    fn bind_publications_with_hook(
        &self,
        stage: &Directory,
        journal: &TreeJournal,
        cancelled: &AtomicBool,
        mut after_binding: impl FnMut(usize) -> Result<(), FileError>,
    ) -> Result<Vec<BoundItem>, BindingFailure> {
        let mut bound = Vec::new();
        let outcome = (|| -> Result<(), FileError> {
            self.verify(stage, cancelled)?;
            if journal.items.len() != self.nodes.len() {
                return Err(FileError::InvalidRequest);
            }
            for (node, intent) in self.nodes.iter().zip(&journal.items) {
                let destination = Locator::new(node_path(&journal.destination, &node.relative))
                    .ok_or(FileError::InvalidRequest)?;
                if node.relative != intent.relative || intent.intent.destination() != &destination {
                    return Err(FileError::InvalidRequest);
                }
            }
            for (index, (node, intent)) in self.nodes.iter().zip(&journal.items).enumerate() {
                check_cancellation(cancelled)?;
                let file = match &node.file_version {
                    Some((file, _)) => file.try_clone()?,
                    None => self.directories[index]
                        .as_ref()
                        .ok_or(FileError::RecoveryPending)?
                        .try_clone_file()?,
                };
                let witness = LiveIdentity::from_file(file)?;
                if witness.comparison_key()? != node.identity.comparison_key()?
                    || witness.observe()? != node.identity.observe()?
                {
                    return Err(FileError::RecoveryPending);
                }
                let publication = intent.intent.bind_staged_destination(witness)?;
                bound.push(BoundItem {
                    relative: node.relative.clone(),
                    publication,
                });
                after_binding(index)?;
            }
            Ok(())
        })();
        match outcome {
            Ok(()) => Ok(bound),
            Err(error) => Err(BindingFailure { error, bound }),
        }
    }

    /// Recheck retained source and staging witnesses immediately before any
    /// future publication. This is not an atomic compare-and-publish.
    fn verify(&self, stage: &Directory, cancelled: &AtomicBool) -> Result<(), FileError> {
        self.source.verify(cancelled)?;
        self.verify_output(stage, cancelled)
    }

    fn verify_output(&self, stage: &Directory, cancelled: &AtomicBool) -> Result<(), FileError> {
        stage.verify().map_err(|_| FileError::RecoveryPending)?;
        let root_path = canonical_path(stage.path()).map_err(|_| FileError::RecoveryPending)?;
        for (index, (source, output)) in self.source.nodes.iter().zip(&self.nodes).enumerate() {
            check_cancellation(cancelled)?;
            let expected = Locator::new(node_path(&root_path, &output.relative))
                .ok_or(FileError::RecoveryPending)?;
            if output
                .identity
                .observe()
                .map_err(|_| FileError::RecoveryPending)?
                != LiveObservation::Present(expected)
            {
                return Err(FileError::RecoveryPending);
            }
            if let Some((file, initial)) = &output.file_version
                && version(&file.metadata()?).map_err(|_| FileError::RecoveryPending)? != *initial
            {
                return Err(FileError::RecoveryPending);
            }
            if let SourceKind::Directory { entries, .. } = &source.kind {
                let directory = self.directories[index]
                    .as_ref()
                    .ok_or(FileError::RecoveryPending)?;
                directory.verify().map_err(|_| FileError::RecoveryPending)?;
                if sorted_entries(directory).map_err(|_| FileError::RecoveryPending)? != *entries {
                    return Err(FileError::RecoveryPending);
                }
            }
        }
        Ok(())
    }

    fn discard(
        &self,
        stage: &Directory,
        parent: &Directory,
        name: &OsStr,
    ) -> Result<(), FileError> {
        let root = captured_directory_path(stage).map_err(|_| FileError::CleanupFailed)?;
        let mut contents = vec![Vec::new(); self.nodes.len()];
        for (source, output) in self.source.nodes.iter().zip(&self.nodes) {
            if let Some(parent) = source.parent {
                let kind = match output.origin {
                    WitnessOrigin::ExclusiveFile => EntryKind::RegularFile,
                    WitnessOrigin::WitnessedDirectory => EntryKind::Directory,
                    WitnessOrigin::ProvidedRoot => return Err(FileError::CleanupFailed),
                };
                contents[parent].push((source.name.clone(), kind));
            }
        }
        for (index, output) in self.nodes.iter().enumerate() {
            Self::verify_owned(&root, output)?;
            if let Some(directory) = &self.directories[index] {
                directory.verify().map_err(|_| FileError::CleanupFailed)?;
                if sorted_entries(directory).map_err(|_| FileError::CleanupFailed)?
                    != contents[index]
                {
                    return Err(FileError::CleanupFailed);
                }
            }
        }
        // Only remove witnessed entries, deepest first. An unexpected child
        // racing cleanup prevents its parent from being removed.
        for index in (1..self.nodes.len()).rev() {
            let source = &self.source.nodes[index];
            let output = &self.nodes[index];
            Self::verify_owned(&root, output)?;
            let captured_parent = self.directories
                [source.parent.ok_or(FileError::CleanupFailed)?]
            .as_ref()
            .ok_or(FileError::CleanupFailed)?;
            captured_parent
                .verify()
                .map_err(|_| FileError::CleanupFailed)?;
            match output.origin {
                WitnessOrigin::ExclusiveFile => native::remove_file(captured_parent, &source.name),
                WitnessOrigin::WitnessedDirectory => {
                    native::remove_directory(captured_parent, &source.name)
                }
                WitnessOrigin::ProvidedRoot => return Err(FileError::CleanupFailed),
            }
            .map_err(|_| FileError::CleanupFailed)?;
        }
        Self::verify_owned(&root, &self.nodes[0])?;
        discard_empty_stage(stage, parent, name)
    }

    fn verify_owned(root: &Path, entry: &StagedNode) -> Result<(), FileError> {
        let expected =
            Locator::new(node_path(root, &entry.relative)).ok_or(FileError::CleanupFailed)?;
        if entry
            .identity
            .observe()
            .map_err(|_| FileError::CleanupFailed)?
            == LiveObservation::Present(expected)
        {
            Ok(())
        } else {
            Err(FileError::CleanupFailed)
        }
    }
}

fn discard_empty_stage(
    stage: &Directory,
    parent: &Directory,
    name: &OsStr,
) -> Result<(), FileError> {
    stage.verify().map_err(|_| FileError::CleanupFailed)?;
    parent.verify().map_err(|_| FileError::CleanupFailed)?;
    native::remove_directory(parent, name).map_err(|_| FileError::CleanupFailed)
}

fn reserve_destination(
    storage: &Storage,
    source: &SourceTree,
    directory: &Directory,
    source_name: &OsStr,
    operation: (&str, &str),
    sequence: &mut usize,
    cancelled: &AtomicBool,
) -> Result<(OsString, TreeJournal), FileError> {
    while *sequence < 10_000 {
        directory.verify()?;
        let name = copy_name(source_name, *sequence, true);
        let prefix = format!("{}:{}:", operation.1, *sequence);
        *sequence += 1;
        match source.record_intents_with_hook(
            storage,
            &directory.path().join(&name),
            (operation.0, &prefix),
            cancelled,
            |_| {},
        ) {
            Ok(journal) => return Ok((name, journal)),
            Err(failure) => {
                failure.journal.abandon(storage)?;
                if failure.error != FileError::AlreadyExists {
                    return Err(failure.error);
                }
            }
        }
    }
    Err(FileError::AlreadyExists)
}

pub(super) fn copy_tree(
    storage: &Storage,
    path: &Path,
    destination: &Directory,
    operation: (&str, &str),
    cancelled: &AtomicBool,
) -> Result<TreeOutcome, FileError> {
    copy_tree_with_hook(storage, path, destination, operation, cancelled, || {})
}

fn copy_tree_with_hook(
    storage: &Storage,
    path: &Path,
    destination: &Directory,
    operation: (&str, &str),
    cancelled: &AtomicBool,
    after_staging: impl FnOnce(),
) -> Result<TreeOutcome, FileError> {
    let source_name = path.file_name().ok_or(FileError::InvalidRequest)?;
    let source = SourceTree::capture(path, cancelled)?;
    let visible_destination = destination.duplicate()?;
    let canonical = captured_directory_path(destination)?;
    let directory = Directory::open(&canonical)?;
    let visible = LiveIdentity::from_file(visible_destination.try_clone_file()?)?;
    let anchored = LiveIdentity::from_file(directory.try_clone_file()?)?;
    if visible.comparison_key()? != anchored.comparison_key()? {
        return Err(FileError::NotFound);
    }
    let SourceKind::Directory {
        directory: source_root,
        ..
    } = &source.nodes[0].kind
    else {
        unreachable!();
    };
    if directory.path().starts_with(source_root.path()) {
        return Err(FileError::InvalidRequest);
    }
    let mut sequence = 0;
    let mut candidate = reserve_destination(
        storage,
        &source,
        &directory,
        source_name,
        operation,
        &mut sequence,
        cancelled,
    )?;
    let temporary_name = OsString::from(format!(".folderai-{}", Uuid::new_v4()));
    let stage = match directory.create_staging_directory(&temporary_name) {
        Ok(Ok(stage)) => stage,
        result => {
            candidate.1.abandon(storage)?;
            return Err(match result {
                Err(error) => error.into(),
                _ => FileError::CleanupFailed,
            });
        }
    };
    let staged = match source.stage(&stage, cancelled) {
        Ok(staged) => staged,
        Err(failure) => {
            candidate.1.abandon(storage)?;
            if failure.pending.is_some() || failure.error == FileError::RecoveryPending {
                return Err(FileError::CleanupFailed);
            }
            if let Some(partial) = failure.partial {
                partial.discard(&stage, &directory, &temporary_name)?;
            } else {
                discard_empty_stage(&stage, &directory, &temporary_name)?;
            }
            return Err(failure.error);
        }
    };
    after_staging();
    loop {
        let (name, journal) = candidate;
        let ready = staged
            .bind_publications(&stage, &journal, cancelled)
            .map_err(|failure| failure.error)
            .and_then(|bound| {
                staged.verify(&stage, cancelled)?;
                visible_destination.verify()?;
                directory.verify()?;
                check_cancellation(cancelled)?;
                Ok(bound)
            });
        let bound = match ready {
            Ok(bound) => bound,
            Err(error) => {
                journal.abandon(storage)?;
                if error == FileError::RecoveryPending {
                    return Err(FileError::CleanupFailed);
                }
                staged.discard(&stage, &directory, &temporary_name)?;
                return Err(error);
            }
        };
        let publication = native::publish(&directory, &temporary_name, &directory, &name);
        if let Err(error) = publication {
            let unchanged = staged
                .verify_output(&stage, &AtomicBool::new(false))
                .is_ok();
            if unchanged {
                journal.abandon(storage)?;
                let next = if error.kind() == std::io::ErrorKind::AlreadyExists {
                    reserve_destination(
                        storage,
                        &staged.source,
                        &directory,
                        source_name,
                        operation,
                        &mut sequence,
                        cancelled,
                    )
                } else {
                    Err(error.into())
                };
                match next {
                    Ok(attempt) => {
                        candidate = attempt;
                        continue;
                    }
                    Err(error) => {
                        staged.discard(&stage, &directory, &temporary_name)?;
                        return Err(error);
                    }
                }
            }
        }
        let pending = PendingTreeCopy {
            visible_path: visible_destination.path().join(name),
            destination: visible_destination,
            journal,
            bound,
        };
        return match pending.commit(storage) {
            Ok((path, cleanup)) => Ok(TreeOutcome::Completed(path, cleanup)),
            Err(_) => {
                pending.journal.mark_recovery(storage);
                Ok(TreeOutcome::Pending(pending))
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, sync::atomic::Ordering};
    use uuid::Uuid;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("folderai-tree-{}", Uuid::new_v4()));
            fs::create_dir_all(path.join("source/nested")).unwrap();
            fs::create_dir_all(path.join("stage")).unwrap();
            fs::write(path.join("source/first.txt"), b"first").unwrap();
            fs::write(path.join("source/nested/caffè.txt"), b"nested").unwrap();
            Self(path)
        }
        fn source(&self) -> PathBuf {
            self.0.join("source")
        }
        fn stage(&self) -> Directory {
            Directory::open(&canonical_path(&self.0.join("stage")).unwrap()).unwrap()
        }
        fn storage(&self) -> Storage {
            Storage::for_test(&self.0.join("data"), &self.0.join("cache")).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn engine_publishes_whole_tree_after_late_collision() {
        let f = Fixture::new();
        let storage = f.storage();
        fs::create_dir(f.source().join("empty")).unwrap();
        let modified = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        Directory::open(&f.source().join("nested"))
            .unwrap()
            .set_modified(modified)
            .unwrap();
        let outcome = copy_tree_with_hook(
            &storage,
            &f.source(),
            &Directory::open(&f.0).unwrap(),
            ("tree-copy", "0"),
            &AtomicBool::new(false),
            || fs::write(f.0.join("source - copy"), b"racing file").unwrap(),
        )
        .unwrap();
        let TreeOutcome::Completed(path, false) = outcome else {
            panic!("copy should publish");
        };
        assert_eq!(path, f.0.join("source - copy (2)"));
        assert_eq!(fs::read(f.0.join("source - copy")).unwrap(), b"racing file");
        assert!(path.join("empty").is_dir());
        assert_eq!(fs::read(path.join("nested/caffè.txt")).unwrap(), b"nested");
        assert_eq!(
            fs::metadata(path.join("nested"))
                .unwrap()
                .modified()
                .unwrap(),
            modified
        );
        assert_eq!(fs::read(f.source().join("first.txt")).unwrap(), b"first");
        assert!(!f.0.read_dir().unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".folderai-")
        }));
    }

    #[test]
    fn engine_cleans_cancelled_tree_and_preserves_unknown_staging_content() {
        let f = Fixture::new();
        let storage = f.storage();
        let destination = f.0.join("destination");
        fs::create_dir(&destination).unwrap();
        let cancelled = AtomicBool::new(false);
        let run = copy_tree_with_hook(
            &storage,
            &f.source(),
            &Directory::open(&destination).unwrap(),
            ("cancel-tree", "0"),
            &cancelled,
            || cancelled.store(true, Ordering::Release),
        );
        assert!(matches!(run, Err(FileError::Cancelled)));
        assert_eq!(destination.read_dir().unwrap().count(), 0);
        let cancelled = AtomicBool::new(false);
        let run = copy_tree_with_hook(
            &storage,
            &f.source(),
            &Directory::open(&destination).unwrap(),
            ("foreign-content", "0"),
            &cancelled,
            || {
                let stage = destination
                    .read_dir()
                    .unwrap()
                    .next()
                    .unwrap()
                    .unwrap()
                    .path();
                fs::write(stage.join("foreign"), b"preserve").unwrap();
                cancelled.store(true, Ordering::Release);
            },
        );
        assert!(matches!(run, Err(FileError::CleanupFailed)));
        let stage = destination
            .read_dir()
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert_eq!(fs::read(stage.join("foreign")).unwrap(), b"preserve");
        assert_eq!(fs::read(stage.join("first.txt")).unwrap(), b"first");
        assert_eq!(fs::read(f.source().join("first.txt")).unwrap(), b"first");
        assert!(!destination.join("source").exists());
    }

    #[test]
    fn engine_recovers_published_tree_without_copying_again() {
        let f = Fixture::new();
        let storage = f.storage();
        let destination = f.0.join("destination");
        fs::create_dir(&destination).unwrap();
        let outcome = copy_tree_with_hook(
            &storage,
            &f.source(),
            &Directory::open(&destination).unwrap(),
            ("recover-tree", "0"),
            &AtomicBool::new(false),
            || {
                storage
                    .access(|store| {
                        store.connection.execute_batch(
                    "CREATE TRIGGER reject_engine_copy AFTER UPDATE OF state ON operation_items
                     WHEN NEW.state='completed' AND NEW.item_key='0:0:2'
                     BEGIN SELECT raise(ABORT, 'test copy commit'); END;",
                )?;
                        Ok(())
                    })
                    .unwrap()
            },
        )
        .unwrap();
        let TreeOutcome::Pending(copy) = outcome else {
            panic!("copy should still be pending");
        };
        assert_eq!(
            fs::read(destination.join("source/nested/caffè.txt")).unwrap(),
            b"nested"
        );
        let copies = super::super::CopyRecoveries::default();
        let request = Uuid::new_v4();
        copies
            .remember("main", request, 0, super::super::PendingCopy::Tree(copy))
            .unwrap();
        assert_eq!(
            copies.retry("other", request, 0, &storage),
            Err(FileError::RecoveryPending)
        );
        storage
            .access(|store| {
                store
                    .connection
                    .execute_batch("DROP TRIGGER reject_engine_copy")?;
                Ok(())
            })
            .unwrap();
        assert_eq!(
            copies.retry("main", request, 0, &storage),
            Ok((destination.join("source"), false))
        );
        assert!(!copies.has_pending("main", request, 0).unwrap());
        assert_eq!(destination.read_dir().unwrap().count(), 1);
    }

    #[test]
    fn records_durable_intents_for_every_tree_entry_before_staging() {
        use crate::storage::CopyRecovery;
        let f = Fixture::new();
        let storage = f.storage();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let journal = tree
            .record_intents(&storage, &f.0.join("output"), "tree-operation", &cancelled)
            .unwrap();
        assert_eq!(journal.items.len(), 4);
        assert_eq!(journal.operation_id, "tree-operation");
        assert!(f.stage().entries().unwrap().is_empty());
        assert!(!f.0.join("output").exists());
        for item in &journal.items {
            let target = Locator::new(node_path(&journal.destination, &item.relative)).unwrap();
            assert!(
                storage
                    .access(|store| store.copy_destination_reserved(&target))
                    .unwrap()
            );
            assert_eq!(
                storage
                    .access(|store| store.recover_copy(CopyKey {
                        operation_id: &journal.operation_id,
                        item_key: &item.item_key,
                    }))
                    .unwrap(),
                CopyRecovery::AwaitingEvidence
            );
            storage
                .access(|store| store.fail_copy(&item.intent))
                .unwrap();
            assert_eq!(
                storage
                    .access(|store| store.recover_copy(CopyKey {
                        operation_id: &journal.operation_id,
                        item_key: &item.item_key,
                    }))
                    .unwrap(),
                CopyRecovery::Failed
            );
        }
        assert_eq!(
            fs::read(f.source().join("nested/caffè.txt")).unwrap(),
            b"nested"
        );
    }

    #[test]
    fn cancelled_journal_preparation_retains_existing_intents_for_explicit_failure() {
        use crate::storage::CopyRecovery;
        let f = Fixture::new();
        let storage = f.storage();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let result = tree.record_intents_with_hook(
            &storage,
            &f.0.join("output"),
            ("partial-operation", ""),
            &cancelled,
            |index| {
                if index == 1 {
                    cancelled.store(true, Ordering::Release);
                }
            },
        );
        let failure = match result {
            Err(error) => error,
            Ok(_) => panic!("journal should stop"),
        };
        assert_eq!(failure.error, FileError::Cancelled);
        assert_eq!(failure.journal.items.len(), 2);
        assert!(!f.0.join("output").exists());
        for item in &failure.journal.items {
            let key = CopyKey {
                operation_id: &failure.journal.operation_id,
                item_key: &item.item_key,
            };
            assert_eq!(
                storage.access(|store| store.recover_copy(key)).unwrap(),
                CopyRecovery::AwaitingEvidence
            );
            storage
                .access(|store| store.fail_copy(&item.intent))
                .unwrap();
            assert_eq!(
                storage.access(|store| store.recover_copy(key)).unwrap(),
                CopyRecovery::Failed
            );
        }
        assert!(f.stage().entries().unwrap().is_empty());
        assert_eq!(fs::read(f.source().join("first.txt")).unwrap(), b"first");
    }

    fn run_journaled_tree_publication(enabled: bool) {
        use crate::{
            file_transfer::native,
            storage::{COPY_METADATA_KEY, CopyRecovery},
        };
        use std::ffi::OsStr;
        let f = Fixture::new();
        let storage = f.storage();
        if !enabled {
            storage
                .access(|store| {
                    store.connection.execute(
                        "INSERT INTO preferences VALUES (?1, 'false')",
                        [COPY_METADATA_KEY],
                    )?;
                    Ok(())
                })
                .unwrap();
        }
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let journal = tree
            .record_intents(&storage, &f.0.join("output"), "published-tree", &cancelled)
            .unwrap();
        let original =
            Locator::new(canonical_path(&f.source().join("nested/caffè.txt")).unwrap()).unwrap();
        let source_id = storage
            .access(|store| store.resolve_live_item(&original, false))
            .unwrap()
            .unwrap();
        storage
            .access(|store| {
                store.connection.execute(
                    "INSERT INTO notes (item_id, body, revision) VALUES (?1, 'personal note', 1)",
                    [&source_id],
                )?;
                Ok(())
            })
            .unwrap();
        let stage = f.stage();
        let staged = tree.stage(&stage, &cancelled).unwrap();
        let bound = staged
            .bind_publications(&stage, &journal, &cancelled)
            .unwrap();
        staged.verify(&stage, &cancelled).unwrap();
        let parent = Directory::open(&canonical_path(&f.0).unwrap()).unwrap();
        native::publish(&parent, OsStr::new("stage"), &parent, OsStr::new("output")).unwrap();
        let mut copied_file = None;
        for (item, witness) in journal.items.iter().zip(&bound) {
            assert_eq!(item.relative, witness.relative);
            let id = storage
                .access(|store| store.record_completed_copy(&item.intent, &witness.publication))
                .unwrap();
            assert_eq!(
                storage
                    .access(|store| store.recover_copy(CopyKey {
                        operation_id: &journal.operation_id,
                        item_key: &item.item_key,
                    }))
                    .unwrap(),
                CopyRecovery::Completed(id.clone())
            );
            if item.relative == Path::new("nested/caffè.txt") {
                copied_file = Some(id);
            }
        }
        let copied_id = copied_file.unwrap();
        assert_ne!(copied_id, source_id);
        let note: Option<String> = storage
            .access(|store| {
                use rusqlite::OptionalExtension;
                Ok(store
                    .connection
                    .query_row(
                        "SELECT body FROM notes WHERE item_id=?1",
                        [&copied_id],
                        |row| row.get(0),
                    )
                    .optional()?)
            })
            .unwrap();
        assert_eq!(note.as_deref(), enabled.then_some("personal note"));
        assert_eq!(
            fs::read(f.0.join("output/nested/caffè.txt")).unwrap(),
            b"nested"
        );
        assert_eq!(
            fs::read(f.source().join("nested/caffè.txt")).unwrap(),
            b"nested"
        );
    }

    #[test]
    fn journaled_tree_publication_copies_metadata_when_enabled() {
        run_journaled_tree_publication(true);
    }

    #[test]
    fn journaled_tree_publication_omits_metadata_when_disabled() {
        run_journaled_tree_publication(false);
    }

    #[test]
    fn late_tree_destination_collision_preserves_both_trees() {
        use crate::file_transfer::native;
        use std::ffi::OsStr;
        let f = Fixture::new();
        let storage = f.storage();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let journal = tree
            .record_intents(&storage, &f.0.join("output"), "late-collision", &cancelled)
            .unwrap();
        let stage = f.stage();
        let staged = tree.stage(&stage, &cancelled).unwrap();
        let _bound = staged
            .bind_publications(&stage, &journal, &cancelled)
            .unwrap();
        fs::create_dir(f.0.join("output")).unwrap();
        fs::write(f.0.join("output/foreign"), b"keep").unwrap();
        staged.verify(&stage, &cancelled).unwrap();
        let parent = Directory::open(&canonical_path(&f.0).unwrap()).unwrap();
        let error = native::publish(&parent, OsStr::new("stage"), &parent, OsStr::new("output"))
            .unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        for item in &journal.items {
            storage
                .access(|store| store.fail_copy(&item.intent))
                .unwrap();
        }
        assert_eq!(fs::read(f.0.join("output/foreign")).unwrap(), b"keep");
        assert_eq!(
            fs::read(stage.path().join("nested/caffè.txt")).unwrap(),
            b"nested"
        );
        assert_eq!(
            fs::read(f.source().join("nested/caffè.txt")).unwrap(),
            b"nested"
        );
    }

    #[test]
    fn partial_metadata_commit_retries_with_original_tree_witness() {
        use crate::{file_transfer::native, storage::CopyRecovery};
        use std::ffi::OsStr;
        let f = Fixture::new();
        let storage = f.storage();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let journal = tree
            .record_intents(&storage, &f.0.join("output"), "partial-commit", &cancelled)
            .unwrap();
        let stage = f.stage();
        let staged = tree.stage(&stage, &cancelled).unwrap();
        let bound = staged
            .bind_publications(&stage, &journal, &cancelled)
            .unwrap();
        staged.verify(&stage, &cancelled).unwrap();
        let parent = Directory::open(&canonical_path(&f.0).unwrap()).unwrap();
        native::publish(&parent, OsStr::new("stage"), &parent, OsStr::new("output")).unwrap();
        storage
            .access(|store| {
                store.connection.execute_batch(
                    "CREATE TABLE deferred_tree_failure (
                missing TEXT REFERENCES items(id) DEFERRABLE INITIALLY DEFERRED
            ); CREATE TRIGGER reject_one_tree_commit AFTER UPDATE OF state ON operation_items
                WHEN NEW.state='completed' AND NEW.item_key='2'
                BEGIN INSERT INTO deferred_tree_failure VALUES ('missing-item'); END;",
                )?;
                Ok(())
            })
            .unwrap();
        for (index, (item, witness)) in journal.items.iter().zip(&bound).enumerate() {
            let result = storage
                .access(|store| store.record_completed_copy(&item.intent, &witness.publication));
            if index == 2 {
                assert_eq!(result, Err(StorageError::Unavailable));
                assert_eq!(
                    storage
                        .access(|store| store.recover_copy(CopyKey {
                            operation_id: &journal.operation_id,
                            item_key: &item.item_key,
                        }))
                        .unwrap(),
                    CopyRecovery::AwaitingEvidence
                );
                assert!(
                    storage
                        .access(|store| store.fail_copy(&item.intent))
                        .is_err()
                );
            } else {
                assert!(result.is_ok());
            }
        }
        assert_eq!(
            fs::read(f.0.join("output/nested/caffè.txt")).unwrap(),
            b"nested"
        );
        storage
            .access(|store| {
                store
                    .connection
                    .execute_batch("DROP TRIGGER reject_one_tree_commit")?;
                Ok(())
            })
            .unwrap();
        let item = &journal.items[2];
        let witness = &bound[2];
        let id = storage
            .access(|store| store.record_completed_copy(&item.intent, &witness.publication))
            .unwrap();
        assert_eq!(
            storage
                .access(|store| store.record_completed_copy(&item.intent, &witness.publication))
                .unwrap(),
            id
        );
        assert_eq!(
            storage
                .access(|store| store.recover_copy(CopyKey {
                    operation_id: &journal.operation_id,
                    item_key: &item.item_key,
                }))
                .unwrap(),
            CopyRecovery::Completed(id)
        );
        assert_eq!(
            fs::read(f.source().join("nested/caffè.txt")).unwrap(),
            b"nested"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn replaced_tree_descendant_is_not_assigned_the_original_copy_metadata() {
        use crate::{file_transfer::native, storage::CopyRecovery};
        use std::ffi::OsStr;
        let f = Fixture::new();
        let storage = f.storage();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let journal = tree
            .record_intents(&storage, &f.0.join("output"), "replaced-entry", &cancelled)
            .unwrap();
        let stage = f.stage();
        let staged = tree.stage(&stage, &cancelled).unwrap();
        let bound = staged
            .bind_publications(&stage, &journal, &cancelled)
            .unwrap();
        staged.verify(&stage, &cancelled).unwrap();
        let parent = Directory::open(&canonical_path(&f.0).unwrap()).unwrap();
        native::publish(&parent, OsStr::new("stage"), &parent, OsStr::new("output")).unwrap();
        let target = f.0.join("output/nested/caffè.txt");
        let retained = f.0.join("saved-copy.txt");
        fs::rename(&target, &retained).unwrap();
        fs::write(&target, b"replacement").unwrap();
        let index = journal
            .items
            .iter()
            .position(|item| item.relative == Path::new("nested/caffè.txt"))
            .unwrap();
        let item = &journal.items[index];
        assert_eq!(
            storage.access(
                |store| store.record_completed_copy(&item.intent, &bound[index].publication)
            ),
            Err(StorageError::ReferenceConflict)
        );
        assert_eq!(
            storage
                .access(|store| store.recover_copy(CopyKey {
                    operation_id: &journal.operation_id,
                    item_key: &item.item_key,
                }))
                .unwrap(),
            CopyRecovery::Conflict
        );
        assert_eq!(fs::read(&retained).unwrap(), b"nested");
        assert_eq!(fs::read(&target).unwrap(), b"replacement");
        assert_eq!(
            fs::read(f.source().join("nested/caffè.txt")).unwrap(),
            b"nested"
        );
    }

    #[test]
    fn binding_failure_returns_publications_already_sealed() {
        let f = Fixture::new();
        let storage = f.storage();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let journal = tree
            .record_intents(&storage, &f.0.join("output"), "bind-failure", &cancelled)
            .unwrap();
        let stage = f.stage();
        let staged = tree.stage(&stage, &cancelled).unwrap();
        let result = staged.bind_publications_with_hook(&stage, &journal, &cancelled, |index| {
            if index == 1 {
                Err(FileError::Unavailable)
            } else {
                Ok(())
            }
        });
        let failure = match result {
            Err(error) => error,
            Ok(_) => panic!("binding should fail"),
        };
        assert_eq!(failure.error, FileError::Unavailable);
        assert_eq!(failure.bound.len(), 2);
        assert_eq!(failure.bound[0].relative, Path::new(""));
        assert_eq!(failure.bound[1].relative, Path::new("first.txt"));
        // These intents were already bound: a second attempt must not invent new evidence.
        assert!(
            staged
                .bind_publications(&stage, &journal, &cancelled)
                .is_err()
        );
        for item in &journal.items {
            storage
                .access(|store| store.fail_copy(&item.intent))
                .unwrap();
        }
        assert!(!f.0.join("output").exists());
        assert_eq!(fs::read(stage.path().join("first.txt")).unwrap(), b"first");
    }

    #[test]
    fn concurrent_journal_cannot_reserve_the_same_tree_destination() {
        let f = Fixture::new();
        let storage = f.storage();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let first = tree
            .record_intents(&storage, &f.0.join("output"), "first", &cancelled)
            .unwrap();
        let result = tree.record_intents(&storage, &f.0.join("output"), "second", &cancelled);
        let failure = match result {
            Err(error) => error,
            Ok(_) => panic!("reservation should conflict"),
        };
        assert_eq!(failure.error, FileError::AlreadyExists);
        assert!(failure.journal.items.is_empty());
        assert_eq!(first.items.len(), 4);
        for item in &first.items {
            storage
                .access(|store| store.fail_copy(&item.intent))
                .unwrap();
        }
        assert!(!f.0.join("output").exists());
        assert_eq!(fs::read(f.source().join("first.txt")).unwrap(), b"first");
    }

    #[test]
    fn source_changed_during_journaling_keeps_all_recorded_intents() {
        let f = Fixture::new();
        let storage = f.storage();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let result = tree.record_intents_with_hook(
            &storage,
            &f.0.join("output"),
            ("changing", ""),
            &cancelled,
            |index| {
                if index == 0 {
                    fs::write(f.source().join("first.txt"), b"changed later").unwrap();
                }
            },
        );
        let failure = match result {
            Err(error) => error,
            Ok(_) => panic!("changed source"),
        };
        assert_eq!(failure.error, FileError::SourceChanged);
        assert_eq!(failure.journal.items.len(), 4);
        for item in &failure.journal.items {
            storage
                .access(|store| store.fail_copy(&item.intent))
                .unwrap();
        }
        assert!(!f.0.join("output").exists());
        assert!(f.stage().entries().unwrap().is_empty());
        assert_eq!(
            fs::read(f.source().join("first.txt")).unwrap(),
            b"changed later"
        );
    }

    #[test]
    fn journal_preparation_rejects_existing_destination_without_creating_intents() {
        let f = Fixture::new();
        let storage = f.storage();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let result = tree.record_intents(&storage, &f.0.join("stage"), "collision", &cancelled);
        let failure = match result {
            Err(error) => error,
            Ok(_) => panic!("collision expected"),
        };
        assert_eq!(failure.error, FileError::AlreadyExists);
        assert!(failure.journal.items.is_empty());
        assert!(f.stage().entries().unwrap().is_empty());
    }

    #[test]
    fn stages_nested_tree_and_retains_a_witness_for_each_entry() {
        let f = Fixture::new();
        fs::create_dir(f.source().join("empty")).unwrap();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let stage = f.stage();
        let staged = tree.stage(&stage, &cancelled).unwrap();
        assert_eq!(staged.nodes.len(), 5);
        assert!(stage.path().join("empty").is_dir());
        assert_eq!(fs::read(stage.path().join("first.txt")).unwrap(), b"first");
        assert_eq!(
            fs::read(stage.path().join("nested/caffè.txt")).unwrap(),
            b"nested"
        );
        assert_eq!(staged.nodes[0].origin, WitnessOrigin::ProvidedRoot);
        assert_eq!(
            staged
                .nodes
                .iter()
                .find(|node| node.relative == Path::new("nested"))
                .unwrap()
                .origin,
            WitnessOrigin::WitnessedDirectory
        );
        for entry in &staged.nodes {
            let path = stage.path().join(&entry.relative);
            assert_eq!(
                entry.identity.observe().unwrap(),
                LiveObservation::Present(Locator::new(path).unwrap())
            );
        }
        assert_eq!(
            fs::read(f.source().join("nested/caffè.txt")).unwrap(),
            b"nested"
        );
    }

    #[test]
    fn retains_witnesses_to_recheck_source_after_staging() {
        let f = Fixture::new();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let stage = f.stage();
        let staged = tree.stage(&stage, &cancelled).unwrap();
        fs::write(f.source().join("first.txt"), b"changed later").unwrap();
        assert_eq!(
            staged.verify(&stage, &cancelled),
            Err(FileError::SourceChanged)
        );
        assert_eq!(fs::read(stage.path().join("first.txt")).unwrap(), b"first");
    }

    #[test]
    fn rejects_changed_source_before_writing_anything() {
        let f = Fixture::new();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        fs::write(f.source().join("nested/new"), b"new").unwrap();
        let stage = f.stage();
        assert!(matches!(
            tree.stage(&stage, &cancelled),
            Err(StageFailure {
                error: FileError::SourceChanged,
                ..
            })
        ));
        assert!(stage.entries().unwrap().is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn rejects_replaced_source_file_before_staging_even_with_same_size() {
        let f = Fixture::new();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        fs::rename(f.source().join("first.txt"), f.0.join("original.txt")).unwrap();
        fs::write(f.source().join("first.txt"), b"other").unwrap();
        let stage = f.stage();
        assert!(matches!(
            tree.stage(&stage, &cancelled),
            Err(StageFailure {
                error: FileError::SourceChanged,
                ..
            })
        ));
        assert!(stage.entries().unwrap().is_empty());
        assert_eq!(fs::read(f.0.join("original.txt")).unwrap(), b"first");
        assert_eq!(fs::read(f.source().join("first.txt")).unwrap(), b"other");
    }

    #[test]
    fn rejects_existing_staging_content_and_preserves_it() {
        let f = Fixture::new();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let stage = f.stage();
        fs::write(stage.path().join("foreign"), b"foreign").unwrap();
        assert!(matches!(
            tree.stage(&stage, &cancelled),
            Err(StageFailure {
                error: FileError::InvalidRequest,
                ..
            })
        ));
        assert_eq!(fs::read(stage.path().join("foreign")).unwrap(), b"foreign");
    }

    #[test]
    fn cancellation_before_staging_preserves_original_and_stage() {
        let f = Fixture::new();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        cancelled.store(true, Ordering::Release);
        let stage = f.stage();
        assert!(matches!(
            tree.stage(&stage, &cancelled),
            Err(StageFailure {
                error: FileError::Cancelled,
                ..
            })
        ));
        assert!(stage.entries().unwrap().is_empty());
        assert_eq!(fs::read(f.source().join("first.txt")).unwrap(), b"first");
    }

    #[test]
    fn cancellation_during_staging_preserves_partial_tree_for_recovery() {
        let f = Fixture::new();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let stage = f.stage();
        let result = tree.stage_with_hook(&stage, &cancelled, |path| {
            if path == Path::new("first.txt") {
                cancelled.store(true, Ordering::Release);
            }
        });
        let failure = match result {
            Err(error) => error,
            Ok(_) => panic!("copy should stop"),
        };
        assert_eq!(failure.error, FileError::Cancelled);
        let partial = failure.partial.as_ref().unwrap();
        assert!(failure.pending.is_none());
        assert_eq!(partial.nodes.len(), 2);
        assert_eq!(partial.nodes[1].origin, WitnessOrigin::ExclusiveFile);
        assert_eq!(
            partial.nodes[1].identity.observe().unwrap(),
            LiveObservation::Present(Locator::new(stage.path().join("first.txt")).unwrap())
        );
        assert_eq!(fs::read(stage.path().join("first.txt")).unwrap(), b"first");
        assert_eq!(fs::read(f.source().join("first.txt")).unwrap(), b"first");
        assert_eq!(
            fs::read(f.source().join("nested/caffè.txt")).unwrap(),
            b"nested"
        );
    }

    #[test]
    fn failed_copy_retains_live_witness_and_partial_file() {
        use std::io::Write;
        let f = Fixture::new();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let stage = f.stage();
        let result = tree.stage_with_hooks(
            &stage,
            &cancelled,
            |_| {},
            |_| Ok(()),
            |_, output, _| {
                output.write_all(b"partial")?;
                Err(FileError::StorageFull)
            },
        );
        let failure = match result {
            Err(error) => error,
            Ok(_) => panic!("copy should fail"),
        };
        assert_eq!(failure.error, FileError::StorageFull);
        assert!(failure.pending.is_none());
        let partial = failure.partial.as_ref().unwrap();
        assert_eq!(partial.nodes.len(), 2);
        let file = &partial.nodes[1];
        assert_eq!(file.origin, WitnessOrigin::ExclusiveFile);
        assert_eq!(file.relative, Path::new("first.txt"));
        assert_eq!(
            file.identity.observe().unwrap(),
            LiveObservation::Present(Locator::new(stage.path().join("first.txt")).unwrap())
        );
        assert_eq!(
            file.file_version
                .as_ref()
                .unwrap()
                .0
                .metadata()
                .unwrap()
                .len(),
            7
        );
        assert_eq!(
            fs::read(stage.path().join("first.txt")).unwrap(),
            b"partial"
        );
        assert_eq!(fs::read(f.source().join("first.txt")).unwrap(), b"first");
    }

    #[test]
    fn failure_before_identity_binding_retains_created_file_handle() {
        let f = Fixture::new();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let stage = f.stage();
        let result = tree.stage_with_hooks(
            &stage,
            &cancelled,
            |_| {},
            |path| {
                if path == Path::new("first.txt") {
                    Err(FileError::Unavailable)
                } else {
                    Ok(())
                }
            },
            copy_chunks,
        );
        let failure = match result {
            Err(error) => error,
            Ok(_) => panic!("binding should fail"),
        };
        assert_eq!(failure.error, FileError::Unavailable);
        let pending = failure.pending.as_ref().unwrap();
        assert_eq!(pending.relative, Path::new("first.txt"));
        assert!(pending.file.as_ref().unwrap().metadata().unwrap().is_file());
        assert!(pending.directory.is_none());
        assert_eq!(failure.partial.as_ref().unwrap().nodes.len(), 1);
        assert!(stage.path().join("first.txt").exists());
        assert_eq!(fs::read(f.source().join("first.txt")).unwrap(), b"first");
    }

    #[test]
    fn failure_after_mkdir_retains_directory_handle_without_deleting_it() {
        let f = Fixture::new();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let stage = f.stage();
        let result = tree.stage_with_hooks(
            &stage,
            &cancelled,
            |_| {},
            |path| {
                if path == Path::new("nested") {
                    Err(FileError::Unavailable)
                } else {
                    Ok(())
                }
            },
            copy_chunks,
        );
        let failure = match result {
            Err(error) => error,
            Ok(_) => panic!("open should fail"),
        };
        assert_eq!(failure.error, FileError::Unavailable);
        let pending = failure.pending.as_ref().unwrap();
        assert_eq!(pending.relative, Path::new("nested"));
        assert!(pending.file.is_none());
        assert!(pending.directory.is_some());
        assert!(stage.path().join("nested").is_dir());
        assert_eq!(
            fs::read(f.source().join("nested/caffè.txt")).unwrap(),
            b"nested"
        );
    }

    #[test]
    fn detects_modified_staged_file_and_preserves_it_for_recovery() {
        let f = Fixture::new();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let stage = f.stage();
        let result = tree.stage_with_hook(&stage, &cancelled, |path| {
            if path == Path::new("first.txt") {
                fs::write(stage.path().join(path), b"changed elsewhere").unwrap();
            }
        });
        assert!(matches!(
            result,
            Err(StageFailure {
                error: FileError::RecoveryPending,
                ..
            })
        ));
        assert_eq!(
            fs::read(stage.path().join("first.txt")).unwrap(),
            b"changed elsewhere"
        );
        assert_eq!(fs::read(f.source().join("first.txt")).unwrap(), b"first");
    }

    #[test]
    fn detects_unexpected_staging_entry_without_removing_it() {
        let f = Fixture::new();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let stage = f.stage();
        let result = tree.stage_with_hook(&stage, &cancelled, |path| {
            if path == Path::new("first.txt") {
                fs::write(stage.path().join("foreign"), b"foreign").unwrap();
            }
        });
        assert!(matches!(
            result,
            Err(StageFailure {
                error: FileError::RecoveryPending,
                ..
            })
        ));
        assert_eq!(fs::read(stage.path().join("foreign")).unwrap(), b"foreign");
        assert_eq!(fs::read(f.source().join("first.txt")).unwrap(), b"first");
    }

    #[test]
    fn rejects_source_modified_during_staging() {
        let f = Fixture::new();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let stage = f.stage();
        let result = tree.stage_with_hook(&stage, &cancelled, |path| {
            if path == Path::new("first.txt") {
                fs::write(f.source().join("nested/caffè.txt"), b"changed").unwrap();
            }
        });
        assert!(matches!(
            result,
            Err(StageFailure {
                error: FileError::SourceChanged,
                ..
            })
        ));
        assert_eq!(
            fs::read(f.source().join("nested/caffè.txt")).unwrap(),
            b"changed"
        );
        assert_eq!(fs::read(stage.path().join("first.txt")).unwrap(), b"first");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn rejects_symlinks_in_source_without_following_them() {
        use std::os::unix::fs::symlink;
        let f = Fixture::new();
        symlink("first.txt", f.source().join("link")).unwrap();
        let cancelled = AtomicBool::new(false);
        assert!(matches!(
            SourceTree::capture(&f.source(), &cancelled),
            Err(FileError::Unsupported)
        ));
        assert!(f.stage().entries().unwrap().is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn refuses_stage_inside_source_through_symlinked_parent() {
        use std::os::unix::fs::symlink;
        let f = Fixture::new();
        fs::create_dir(f.source().join("empty-stage")).unwrap();
        symlink(f.source(), f.0.join("alias")).unwrap();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let stage = Directory::open(&f.0.join("alias/empty-stage")).unwrap();
        assert!(matches!(
            tree.stage(&stage, &cancelled),
            Err(StageFailure {
                error: FileError::InvalidRequest,
                ..
            })
        ));
        assert!(stage.entries().unwrap().is_empty());
    }

    #[test]
    fn refuses_to_stage_inside_the_source() {
        let f = Fixture::new();
        let nested_stage = f.source().join("empty-stage");
        fs::create_dir(&nested_stage).unwrap();
        let cancelled = AtomicBool::new(false);
        let tree = SourceTree::capture(&f.source(), &cancelled).unwrap();
        let stage = Directory::open(&canonical_path(&nested_stage).unwrap()).unwrap();
        assert!(matches!(
            tree.stage(&stage, &cancelled),
            Err(StageFailure {
                error: FileError::InvalidRequest,
                ..
            })
        ));
        assert!(stage.entries().unwrap().is_empty());
    }
}
