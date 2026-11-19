use notify::{Config, Event, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

const PATH_LIMIT: usize = 256;

#[derive(Debug, Default, PartialEq, Eq)]
pub enum DirtyPaths {
    #[default]
    Unchanged,
    Paths(BTreeSet<PathBuf>),
    Rescan,
}

impl DirtyPaths {
    fn add_result(&mut self, next: notify::Result<Event>) {
        if *self == Self::Rescan {
            return;
        }
        let Ok(event) = next else {
            *self = Self::Rescan;
            return;
        };
        if event.need_rescan() || event.paths.is_empty() {
            *self = Self::Rescan;
            return;
        }
        if *self == Self::Unchanged {
            *self = Self::Paths(BTreeSet::new());
        }
        if let Self::Paths(paths) = self {
            for path in event.paths {
                paths.insert(path);
                if paths.len() > PATH_LIMIT {
                    *self = Self::Rescan;
                    break;
                }
            }
        }
    }
}

/// Notifications are hints. A registered root still needs a baseline checked
/// by the caller, and a rescan still requires native comparison.
pub struct WatchSet {
    native: Option<RecommendedWatcher>,
    polling: Option<PollWatcher>,
    polling_roots: BTreeSet<PathBuf>,
    pending: Arc<Mutex<DirtyPaths>>,
    send: mpsc::SyncSender<()>,
    signal: mpsc::Receiver<()>,
}

impl WatchSet {
    pub fn new() -> Self {
        let pending = Arc::new(Mutex::new(DirtyPaths::Unchanged));
        let (send, signal) = mpsc::sync_channel(1);
        let native =
            notify::recommended_watcher(event_handler(Arc::clone(&pending), send.clone())).ok();
        Self {
            native,
            polling: None,
            polling_roots: BTreeSet::new(),
            pending,
            send,
            signal,
        }
    }

    pub fn watch(&mut self, root: &Path, mode: RecursiveMode) -> notify::Result<()> {
        if self
            .native
            .as_mut()
            .is_some_and(|watcher| watcher.watch(root, mode).is_ok())
        {
            return Ok(());
        }
        let polling = match &mut self.polling {
            Some(polling) => polling,
            empty => empty.insert(PollWatcher::new(
                event_handler(Arc::clone(&self.pending), self.send.clone()),
                Config::default().with_poll_interval(Duration::from_secs(2)),
            )?),
        };
        polling.watch(root, mode)?;
        self.polling_roots.insert(root.to_path_buf());
        if let Ok(mut state) = self.pending.lock() {
            *state = DirtyPaths::Rescan;
        }
        let _ = self.send.try_send(());
        Ok(())
    }

    pub fn unwatch(&mut self, root: &Path) -> notify::Result<()> {
        if self.polling_roots.contains(root) {
            let polling = self
                .polling
                .as_mut()
                .ok_or_else(notify::Error::watch_not_found)?;
            polling.unwatch(root)?;
            self.polling_roots.remove(root);
            return Ok(());
        }
        self.native
            .as_mut()
            .ok_or_else(notify::Error::watch_not_found)?
            .unwatch(root)
    }

    pub fn wait(&mut self, timeout: Duration) -> DirtyPaths {
        match self.signal.recv_timeout(timeout) {
            Ok(()) => self.take_pending(),
            Err(mpsc::RecvTimeoutError::Timeout) => self.take_pending(),
            Err(mpsc::RecvTimeoutError::Disconnected) => DirtyPaths::Rescan,
        }
    }

    fn take_pending(&mut self) -> DirtyPaths {
        // There can only be one queued wakeup; drain it before taking the state
        // so an event arriving during the drain stays in the pending batch.
        let _ = self.signal.try_recv();
        self.pending
            .lock()
            .map_or(DirtyPaths::Rescan, |mut state| std::mem::take(&mut *state))
    }
}

impl Default for WatchSet {
    fn default() -> Self {
        Self::new()
    }
}

fn event_handler(
    pending: Arc<Mutex<DirtyPaths>>,
    send: mpsc::SyncSender<()>,
) -> impl FnMut(notify::Result<Event>) {
    move |event| {
        if let Ok(mut state) = pending.lock() {
            state.add_result(event);
        }
        let _ = send.try_send(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::{EventKind, event::Flag};

    #[test]
    fn coalesces_duplicates_and_promotes_lost_events_to_rescan() {
        let mut state = DirtyPaths::Unchanged;
        let path = PathBuf::from("first");
        for _ in 0..PATH_LIMIT {
            state.add_result(Ok(Event::new(EventKind::Any).add_path(path.clone())));
        }
        assert_eq!(state, DirtyPaths::Paths(BTreeSet::from([path])));
        state.add_result(Ok(Event::new(EventKind::Other).set_flag(Flag::Rescan)));
        assert_eq!(state, DirtyPaths::Rescan);

        let mut state = DirtyPaths::Unchanged;
        for index in 0..=PATH_LIMIT {
            state.add_result(Ok(
                Event::new(EventKind::Any).add_path(PathBuf::from(index.to_string()))
            ));
        }
        assert_eq!(state, DirtyPaths::Rescan);
    }
}
