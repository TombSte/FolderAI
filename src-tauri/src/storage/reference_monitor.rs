use super::{StorageError, Store, TrackedReference};
use crate::local_identity::{
    LiveIdentity, LiveObservation, Locator,
    watch::{DirtyPaths, WatchSet},
};
use notify::RecursiveMode;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

pub struct ReconciledReference {
    pub id: String,
    pub outcome: Result<LiveObservation, StorageError>,
}

/// Tracks only references explicitly registered by a local data consumer.
pub struct ReferenceMonitor {
    watch: WatchSet,
    roots: BTreeSet<PathBuf>,
    tracked: BTreeMap<String, TrackedReference>,
}

impl ReferenceMonitor {
    pub fn new() -> Self {
        Self {
            watch: WatchSet::new(),
            roots: BTreeSet::new(),
            tracked: BTreeMap::new(),
        }
    }

    pub fn watch_root(&mut self, root: &Path) -> Result<(), StorageError> {
        let root = fs::canonicalize(root)?;
        if !self.roots.contains(&root) {
            self.watch
                .watch(&root, RecursiveMode::Recursive)
                .map_err(|_| StorageError::Unavailable)?;
            self.roots.insert(root);
        }
        Ok(())
    }

    pub fn track(
        &mut self,
        store: &mut Store,
        id: &str,
        locator: &Locator,
    ) -> Result<(), StorageError> {
        let identity = LiveIdentity::open(locator)?;
        match identity.observe()? {
            LiveObservation::Present(path)
                if self.roots.iter().any(|root| path.path().starts_with(root)) => {}
            _ => return Err(StorageError::ReferenceConflict),
        }
        let reference = store.insert_live_reference(id, identity)?;
        self.tracked.insert(id.to_owned(), reference);
        Ok(())
    }

    pub fn stop_tracking(&mut self, id: &str) {
        self.tracked.remove(id);
    }

    pub fn tracked_mut(&mut self, id: &str) -> Option<&mut TrackedReference> {
        self.tracked.get_mut(id)
    }

    /// Wait without holding the storage writer. A periodic caller can request
    /// a full rescan when subscriptions or events are unreliable.
    pub fn wait(&mut self, timeout: Duration) -> DirtyPaths {
        self.watch.wait(timeout)
    }

    pub fn reconcile(&mut self, store: &mut Store, hints: DirtyPaths) -> Vec<ReconciledReference> {
        let mut results = Vec::new();
        self.tracked.retain(|id, tracked| {
            let should_check = match &hints {
                DirtyPaths::Unchanged => false,
                DirtyPaths::Rescan => true,
                DirtyPaths::Paths(paths) => paths.iter().any(|path| {
                    let old = tracked.previous.path();
                    old.starts_with(path)
                        || path.starts_with(old)
                        || old
                            .parent()
                            .is_some_and(|parent| path.parent() == Some(parent))
                }),
            };
            if !should_check {
                return true;
            }
            let outcome = store.reconcile_live_reference(tracked);
            let removed = matches!(outcome, Ok(LiveObservation::Removed));
            results.push(ReconciledReference {
                id: id.clone(),
                outcome,
            });
            !removed
        });
        results
    }
}

impl Default for ReferenceMonitor {
    fn default() -> Self {
        Self::new()
    }
}
