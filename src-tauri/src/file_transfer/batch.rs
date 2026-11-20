use super::{CopyResult, FileError, native::Directory, parse_path, perform_batch_item};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{AppHandle, Emitter, EventTarget, Manager, WebviewWindow};
use uuid::Uuid;

const TRANSFERS_CHANGED: &str = "file-transfers-changed";

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum TransferKind {
    Copy,
    Move,
}

#[derive(Clone, Copy)]
pub(super) struct BatchItem<'a> {
    pub(super) app: &'a AppHandle,
    pub(super) owner: &'a str,
    pub(super) operation: Uuid,
    pub(super) index: usize,
    pub(super) attempt: usize,
    pub(super) kind: TransferKind,
    pub(super) source: &'a Path,
    pub(super) destination: &'a Directory,
    pub(super) cancelled: &'a AtomicBool,
}

pub(super) enum BatchFailure {
    Item(FileError),
    Destination(FileError),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TransferRequest {
    kind: TransferKind,
    sources: Vec<String>,
    directory: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Choice {
    Retry,
    Skip,
    Stop,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TransferDecision {
    id: String,
    decision_id: String,
    choice: Choice,
    apply_to_similar: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Phase {
    Planning,
    Running,
    Waiting,
    Stopping,
    Finished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum DecisionScope {
    Item,
    Destination,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Decision {
    id: String,
    item_index: usize,
    error: FileError,
    scope: DecisionScope,
}

#[derive(Clone, Debug, Serialize)]
#[serde(
    tag = "status",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
enum Outcome {
    Pending,
    Running,
    Waiting,
    Completed {
        path: String,
        cleanup_pending: bool,
    },
    Skipped {
        error: FileError,
    },
    Failed {
        error: FileError,
    },
    Cancelled,
    RecoveryPending {
        path: Option<String>,
        retained_path: Option<String>,
    },
}

#[derive(Serialize)]
struct ItemReport {
    source: PathBuf,
    #[serde(flatten)]
    outcome: Outcome,
}

#[derive(Serialize)]
pub(crate) struct TransferSnapshot {
    id: String,
    kind: TransferKind,
    directory: PathBuf,
    phase: Phase,
    total: usize,
    settled: usize,
    completed: usize,
    skipped: usize,
    failed: usize,
    decision: Option<Decision>,
    items: Vec<ItemReport>,
}

struct Activity {
    phase: Phase,
    items: Vec<Outcome>,
    next: usize,
    attempt: usize,
    directory: Option<Arc<Directory>>,
    decision: Option<Decision>,
    similar: Vec<(FileError, Choice)>,
}

impl Activity {
    fn new(total: usize) -> Self {
        Self {
            phase: Phase::Planning,
            items: vec![Outcome::Pending; total],
            next: 0,
            attempt: 0,
            directory: None,
            decision: None,
            similar: Vec::new(),
        }
    }

    fn settle(&mut self, outcome: Outcome) {
        self.items[self.next] = outcome;
        self.next += 1;
        self.attempt = 0;
    }

    fn finish(&mut self) {
        for item in &mut self.items[self.next..] {
            *item = Outcome::Cancelled;
        }
        self.phase = Phase::Finished;
        self.decision = None;
        self.directory = None;
    }

    fn failure(&mut self, error: FileError, scope: DecisionScope, stopping: bool) {
        if stopping {
            self.settle(Outcome::Failed { error });
            return;
        }
        if scope == DecisionScope::Item
            && self.attempt == 0
            && let Some((_, choice)) = self.similar.iter().find(|(code, _)| *code == error)
        {
            match choice {
                // A persistent error needs a decision after one automatic retry.
                Choice::Retry => self.attempt += 1,
                Choice::Skip => self.settle(Outcome::Skipped { error }),
                Choice::Stop => unreachable!("stop finishes the transfer immediately"),
            }
            return;
        }
        self.items[self.next] = Outcome::Waiting;
        self.phase = Phase::Waiting;
        self.decision = Some(Decision {
            id: Uuid::new_v4().to_string(),
            item_index: self.next,
            error,
            scope,
        });
    }

    fn record(&mut self, result: Result<CopyResult, BatchFailure>, stopping: bool) {
        let (error, scope) = match result {
            Ok(CopyResult {
                path,
                recovery_pending: true,
                staging_path,
                ..
            }) => {
                self.settle(Outcome::RecoveryPending {
                    path: Some(path),
                    retained_path: staging_path,
                });
                return;
            }
            Ok(CopyResult {
                path,
                cleanup_pending,
                ..
            }) => {
                self.settle(Outcome::Completed {
                    path,
                    cleanup_pending,
                });
                return;
            }
            Err(BatchFailure::Item(error)) => (error, DecisionScope::Item),
            Err(BatchFailure::Destination(error)) => (error, DecisionScope::Destination),
        };
        match error {
            FileError::RecoveryPending => {
                self.settle(Outcome::RecoveryPending {
                    path: None,
                    retained_path: None,
                });
            }
            FileError::Cancelled => {
                self.settle(Outcome::Cancelled);
                self.finish();
            }
            FileError::CleanupFailed => {
                self.settle(Outcome::Failed { error });
            }
            FileError::StorageFull => self.failure(error, DecisionScope::Destination, stopping),
            _ => self.failure(error, scope, stopping),
        }
    }
}

struct Transfer {
    id: Uuid,
    kind: TransferKind,
    sources: Vec<PathBuf>,
    directory: PathBuf,
    activity: Mutex<Activity>,
    cancelled: AtomicBool,
}

impl Transfer {
    fn new(request: TransferRequest) -> Result<Self, FileError> {
        let directory = parse_path(request.directory)?;
        let sources = request
            .sources
            .into_iter()
            .map(parse_path)
            .collect::<Result<Vec<_>, _>>()?;
        let distinct: HashSet<_> = sources.iter().collect();
        if sources.is_empty()
            || distinct.len() != sources.len()
            || sources.iter().any(|path| path.file_name().is_none())
        {
            return Err(FileError::InvalidRequest);
        }
        Ok(Self {
            id: Uuid::new_v4(),
            kind: request.kind,
            activity: Mutex::new(Activity::new(sources.len())),
            sources,
            directory,
            cancelled: AtomicBool::new(false),
        })
    }

    fn activity(&self) -> Result<MutexGuard<'_, Activity>, FileError> {
        self.activity.lock().map_err(|_| FileError::Unavailable)
    }

    fn snapshot(&self) -> Result<TransferSnapshot, FileError> {
        let activity = self.activity()?;
        let mut snapshot = TransferSnapshot {
            id: self.id.to_string(),
            kind: self.kind,
            directory: self.directory.clone(),
            phase: activity.phase,
            total: activity.items.len(),
            settled: 0,
            completed: 0,
            skipped: 0,
            failed: 0,
            decision: activity.decision.clone(),
            items: Vec::with_capacity(activity.items.len()),
        };
        for (source, outcome) in self.sources.iter().zip(&activity.items) {
            match outcome {
                Outcome::Pending | Outcome::Running | Outcome::Waiting => {}
                Outcome::Completed { .. } => {
                    snapshot.completed += 1;
                    snapshot.settled += 1;
                }
                Outcome::Skipped { .. } => {
                    snapshot.skipped += 1;
                    snapshot.settled += 1;
                }
                Outcome::Failed { .. } | Outcome::RecoveryPending { .. } => {
                    snapshot.failed += 1;
                    snapshot.settled += 1;
                }
                Outcome::Cancelled => snapshot.settled += 1,
            }
            snapshot.items.push(ItemReport {
                source: source.clone(),
                outcome: outcome.clone(),
            });
        }
        Ok(snapshot)
    }

    fn stop(&self) -> Result<(), FileError> {
        let mut activity = self.activity()?;
        self.cancelled.store(true, Ordering::Release);
        match activity.phase {
            Phase::Waiting => {
                if let Some(decision) = activity.decision.take() {
                    activity.settle(Outcome::Failed {
                        error: decision.error,
                    });
                }
                activity.finish();
            }
            Phase::Planning | Phase::Running => activity.phase = Phase::Stopping,
            Phase::Stopping | Phase::Finished => {}
        }
        Ok(())
    }

    /// Returns true when a new worker owns the continuation.
    fn choose(&self, request: &TransferDecision) -> Result<bool, FileError> {
        let mut activity = self.activity()?;
        let decision = activity
            .decision
            .as_ref()
            .filter(|decision| decision.id == request.decision_id)
            .ok_or(FileError::InvalidRequest)?;
        if decision.scope == DecisionScope::Destination
            && (request.apply_to_similar || request.choice == Choice::Skip)
        {
            return Err(FileError::InvalidRequest);
        }
        let Decision { error, scope, .. } = *decision;
        activity.decision = None;
        if request.choice == Choice::Stop {
            self.cancelled.store(true, Ordering::Release);
            activity.settle(Outcome::Failed { error });
            activity.finish();
            return Ok(false);
        }
        if scope == DecisionScope::Item && request.apply_to_similar {
            activity.similar.retain(|(code, _)| *code != error);
            activity.similar.push((error, request.choice));
        }
        match request.choice {
            Choice::Retry => activity.attempt += 1,
            Choice::Skip => activity.settle(Outcome::Skipped { error }),
            Choice::Stop => unreachable!("stop has already finished the transfer"),
        }
        activity.phase = Phase::Running;
        Ok(true)
    }

    fn captured_destination(
        &self,
        captured: Option<Arc<Directory>>,
    ) -> Result<Arc<Directory>, FileError> {
        if let Some(destination) = captured {
            destination.verify()?;
            return Ok(destination);
        }
        let destination = Arc::new(Directory::open(&self.directory)?);
        self.activity()?.directory = Some(destination.clone());
        destination.verify()?;
        Ok(destination)
    }

    fn run(
        &self,
        mut copy_or_move: impl FnMut(usize, usize, &Directory) -> Result<CopyResult, BatchFailure>,
        notify: impl Fn(),
    ) -> Result<(), FileError> {
        loop {
            let (index, attempt, destination) = {
                let mut state = self.activity()?;
                if self.cancelled.load(Ordering::Acquire) || state.next == self.sources.len() {
                    state.finish();
                    drop(state);
                    notify();
                    return Ok(());
                }
                state.phase = Phase::Running;
                let index = state.next;
                state.items[index] = Outcome::Running;
                (index, state.attempt, state.directory.clone())
            };
            notify();
            let result = self
                .captured_destination(destination)
                .map_err(BatchFailure::Destination)
                .and_then(|destination| {
                    if self.cancelled.load(Ordering::Acquire) {
                        return Err(BatchFailure::Item(FileError::Cancelled));
                    }
                    copy_or_move(index, attempt, &destination)
                });
            let finished_or_waiting = {
                let mut state = self.activity()?;
                let stopping = self.cancelled.load(Ordering::Acquire);
                state.record(result, stopping);
                matches!(state.phase, Phase::Waiting | Phase::Finished)
            };
            notify();
            if finished_or_waiting {
                return Ok(());
            }
        }
    }

    fn abort_uncertain_worker(&self) {
        if let Ok(mut state) = self.activity()
            && matches!(
                state.phase,
                Phase::Planning | Phase::Running | Phase::Stopping
            )
        {
            if state.next < self.sources.len() {
                state.settle(Outcome::RecoveryPending {
                    path: None,
                    retained_path: None,
                });
            }
            state.finish();
        }
    }

    fn record_recovered(&self, index: usize, result: &CopyResult) -> Result<bool, FileError> {
        let mut activity = self.activity()?;
        let Some(outcome @ Outcome::RecoveryPending { .. }) = activity.items.get_mut(index) else {
            return Ok(false);
        };
        *outcome = Outcome::Completed {
            path: result.path.clone(),
            cleanup_pending: result.cleanup_pending,
        };
        Ok(true)
    }
}

#[derive(Default)]
pub(crate) struct BatchTransfers(Mutex<HashMap<String, Vec<Arc<Transfer>>>>);

impl BatchTransfers {
    fn register(&self, owner: &str, transfer: Arc<Transfer>) -> Result<(), FileError> {
        self.0
            .lock()
            .map_err(|_| FileError::Unavailable)?
            .entry(owner.to_owned())
            .or_default()
            .push(transfer);
        Ok(())
    }

    fn for_owner(&self, owner: &str) -> Result<Vec<Arc<Transfer>>, FileError> {
        let transfers = self.0.lock().map_err(|_| FileError::Unavailable)?;
        Ok(transfers.get(owner).cloned().unwrap_or_default())
    }

    fn get(&self, owner: &str, id: Uuid) -> Result<Arc<Transfer>, FileError> {
        let transfers = self.0.lock().map_err(|_| FileError::Unavailable)?;
        transfers
            .get(owner)
            .and_then(|rows| rows.iter().find(|transfer| transfer.id == id))
            .cloned()
            .ok_or(FileError::InvalidRequest)
    }

    pub(crate) fn forget_window(&self, owner: &str) {
        let transfers = self.0.lock().ok().and_then(|mut rows| rows.remove(owner));
        for transfer in transfers.into_iter().flatten() {
            let _ = transfer.stop();
        }
    }

    pub(super) fn has_active(&self, owner: &str) -> Result<bool, FileError> {
        for transfer in self.for_owner(owner)? {
            if transfer.activity()?.phase != Phase::Finished {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(super) fn stop_all(&self, app: &AppHandle, owner: &str) -> Result<(), FileError> {
        for transfer in self.for_owner(owner)? {
            if transfer.activity()?.phase != Phase::Finished {
                transfer.stop()?;
                notify_change(app, owner, transfer.id);
            }
        }
        Ok(())
    }

    pub(super) fn record_recovered(
        &self,
        app: &AppHandle,
        owner: &str,
        operation: Uuid,
        index: usize,
        result: &CopyResult,
    ) {
        if self
            .get(owner, operation)
            .and_then(|transfer| transfer.record_recovered(index, result))
            .unwrap_or(false)
        {
            notify_change(app, owner, operation);
        }
    }
}

fn notify_change(app: &AppHandle, owner: &str, id: Uuid) {
    if app
        .state::<BatchTransfers>()
        .get(owner, id)
        .is_ok_and(|transfer| {
            transfer
                .activity()
                .is_ok_and(|activity| activity.phase == Phase::Finished)
        })
    {
        app.state::<super::undo::UndoSlots>()
            .finish_batch(app, owner, id);
    }
    let _ = app.emit_to(
        EventTarget::webview_window(owner),
        TRANSFERS_CHANGED,
        id.to_string(),
    );
}

fn launch(app: AppHandle, owner: String, transfer: Arc<Transfer>) {
    tauri::async_runtime::spawn(async move {
        let worker = transfer.clone();
        let worker_app = app.clone();
        let worker_owner = owner.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            worker.run(
                |index, attempt, destination| {
                    perform_batch_item(BatchItem {
                        app: &worker_app,
                        owner: &worker_owner,
                        operation: worker.id,
                        index,
                        attempt,
                        kind: worker.kind,
                        source: &worker.sources[index],
                        destination,
                        cancelled: &worker.cancelled,
                    })
                },
                || notify_change(&worker_app, &worker_owner, worker.id),
            )
        })
        .await;
        if !matches!(result, Ok(Ok(()))) {
            transfer.abort_uncertain_worker();
            notify_change(&app, &owner, transfer.id);
        }
    });
}

#[tauri::command]
pub(crate) fn start_transfer(
    window: WebviewWindow,
    request: TransferRequest,
) -> Result<TransferSnapshot, FileError> {
    let transfer = Arc::new(Transfer::new(request)?);
    let snapshot = transfer.snapshot()?;
    window
        .state::<BatchTransfers>()
        .register(window.label(), transfer.clone())?;
    launch(
        window.app_handle().clone(),
        window.label().to_owned(),
        transfer,
    );
    Ok(snapshot)
}

#[tauri::command]
pub(crate) fn list_transfers(window: WebviewWindow) -> Result<Vec<TransferSnapshot>, FileError> {
    window
        .state::<BatchTransfers>()
        .for_owner(window.label())?
        .iter()
        .rev()
        .map(|transfer| transfer.snapshot())
        .collect()
}

#[tauri::command]
pub(crate) fn decide_transfer(
    window: WebviewWindow,
    request: TransferDecision,
) -> Result<TransferSnapshot, FileError> {
    let id = Uuid::parse_str(&request.id).map_err(|_| FileError::InvalidRequest)?;
    let transfer = window.state::<BatchTransfers>().get(window.label(), id)?;
    let run = transfer.choose(&request)?;
    let snapshot = transfer.snapshot()?;
    notify_change(window.app_handle(), window.label(), transfer.id);
    if run {
        launch(
            window.app_handle().clone(),
            window.label().to_owned(),
            transfer,
        );
    }
    Ok(snapshot)
}

#[tauri::command]
pub(crate) fn stop_transfer(
    window: WebviewWindow,
    id: String,
) -> Result<TransferSnapshot, FileError> {
    let id = Uuid::parse_str(&id).map_err(|_| FileError::InvalidRequest)?;
    let transfer = window.state::<BatchTransfers>().get(window.label(), id)?;
    transfer.stop()?;
    notify_change(window.app_handle(), window.label(), transfer.id);
    transfer.snapshot()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_identity::canonical_path;
    use std::fs;

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("folderai-batch-{}", Uuid::new_v4()));
            fs::create_dir_all(root.join("destination")).unwrap();
            Self(canonical_path(&root).unwrap())
        }

        fn transfer(&self) -> Transfer {
            let request = TransferRequest {
                kind: TransferKind::Copy,
                directory: self.0.join("destination").to_str().unwrap().to_owned(),
                sources: ["one.txt", "two.txt", "three.txt"]
                    .iter()
                    .map(|name| self.0.join(name).to_str().unwrap().to_owned())
                    .collect(),
            };
            Transfer::new(request).unwrap()
        }

        fn completed(&self, index: usize) -> CopyResult {
            CopyResult::completed(self.0.join(format!("destination/file-{index}")), false).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn reply(transfer: &Transfer, choice: Choice, apply_to_similar: bool) -> TransferDecision {
        let snapshot = transfer.snapshot().unwrap();
        TransferDecision {
            id: snapshot.id,
            decision_id: snapshot.decision.unwrap().id,
            choice,
            apply_to_similar,
        }
    }

    #[test]
    fn decisions_pause_only_one_transfer_and_do_not_leak_to_a_new_batch() {
        let fixture = Fixture::new();
        let first = fixture.transfer();
        let independent = fixture.transfer();
        let mut calls = vec![];
        first
            .run(
                |index, _, _| {
                    calls.push(index);
                    Err(BatchFailure::Item(FileError::PermissionDenied))
                },
                || {},
            )
            .unwrap();
        let waiting = first.snapshot().unwrap();
        assert_eq!(waiting.phase, Phase::Waiting);
        assert_eq!(calls, [0]);
        assert_eq!(waiting.decision.unwrap().scope, DecisionScope::Item);

        independent
            .run(|index, _, _| Ok(fixture.completed(index)), || {})
            .unwrap();
        assert_eq!(independent.snapshot().unwrap().completed, 3);

        assert!(first.choose(&reply(&first, Choice::Skip, true)).unwrap());
        first
            .run(
                |index, _, _| {
                    calls.push(index);
                    if index == 1 {
                        Err(BatchFailure::Item(FileError::PermissionDenied))
                    } else {
                        Ok(fixture.completed(index))
                    }
                },
                || {},
            )
            .unwrap();
        let snapshot = first.snapshot().unwrap();
        assert_eq!(snapshot.phase, Phase::Finished);
        assert_eq!(
            (snapshot.completed, snapshot.skipped, snapshot.failed),
            (1, 2, 0)
        );
        assert_eq!(calls, [0, 1, 2]);

        let second = fixture.transfer();
        second
            .run(
                |_, _, _| Err(BatchFailure::Item(FileError::PermissionDenied)),
                || {},
            )
            .unwrap();
        assert_eq!(second.snapshot().unwrap().phase, Phase::Waiting);
    }

    #[test]
    fn retry_issues_new_decision_id_and_automatic_retry_runs_only_once() {
        let fixture = Fixture::new();
        let transfer = fixture.transfer();
        transfer
            .run(
                |_, _, _| Err(BatchFailure::Item(FileError::NotFound)),
                || {},
            )
            .unwrap();
        let retry = reply(&transfer, Choice::Retry, true);
        assert!(transfer.choose(&retry).unwrap());

        let mut calls = vec![];
        transfer
            .run(
                |index, attempt, _| {
                    calls.push((index, attempt));
                    if index == 0 {
                        Ok(fixture.completed(index))
                    } else {
                        Err(BatchFailure::Item(FileError::NotFound))
                    }
                },
                || {},
            )
            .unwrap();
        assert_eq!(calls, [(0, 1), (1, 0), (1, 1)]);
        assert_eq!(transfer.choose(&retry), Err(FileError::InvalidRequest));
        let decision = transfer.snapshot().unwrap().decision.unwrap();
        assert_ne!(decision.id, retry.decision_id);
        assert_eq!(decision.item_index, 1);
    }

    #[test]
    fn stopping_during_second_item_keeps_completed_item_and_cancels_remaining_items() {
        let fixture = Fixture::new();
        let transfer = fixture.transfer();
        let mut calls = vec![];
        transfer
            .run(
                |index, _, _| {
                    calls.push(index);
                    if index == 0 {
                        Ok(fixture.completed(index))
                    } else {
                        transfer.stop().unwrap();
                        assert_eq!(transfer.snapshot().unwrap().phase, Phase::Stopping);
                        assert!(transfer.cancelled.load(Ordering::Acquire));
                        Err(BatchFailure::Item(FileError::Cancelled))
                    }
                },
                || {},
            )
            .unwrap();
        let snapshot = transfer.snapshot().unwrap();
        assert_eq!(snapshot.phase, Phase::Finished);
        assert_eq!((snapshot.completed, snapshot.failed), (1, 0));
        assert_eq!(calls, [0, 1]);
        assert!(matches!(
            snapshot.items[0].outcome,
            Outcome::Completed { .. }
        ));
        assert!(matches!(snapshot.items[1].outcome, Outcome::Cancelled));
        assert!(matches!(snapshot.items[2].outcome, Outcome::Cancelled));
    }

    #[test]
    fn destination_substitution_cannot_be_skipped_or_accepted_on_retry() {
        let fixture = Fixture::new();
        let transfer = fixture.transfer();
        let renamed = fixture.0.join("original");
        let mut calls = vec![];
        transfer
            .run(
                |index, _, destination| {
                    calls.push(index);
                    fs::rename(destination.path(), &renamed).unwrap();
                    fs::create_dir(destination.path()).unwrap();
                    Ok(fixture.completed(index))
                },
                || {},
            )
            .unwrap();
        let decision = transfer.snapshot().unwrap().decision.unwrap();
        assert_eq!(decision.scope, DecisionScope::Destination);
        assert_eq!(
            transfer.choose(&reply(&transfer, Choice::Skip, false)),
            Err(FileError::InvalidRequest)
        );
        assert_eq!(
            transfer.choose(&reply(&transfer, Choice::Retry, true)),
            Err(FileError::InvalidRequest)
        );
        assert!(
            transfer
                .choose(&reply(&transfer, Choice::Retry, false))
                .unwrap()
        );
        transfer
            .run(
                |index, _, _| {
                    calls.push(index);
                    Ok(fixture.completed(index))
                },
                || {},
            )
            .unwrap();
        assert_eq!(calls, [0]);
        assert_eq!(transfer.snapshot().unwrap().phase, Phase::Waiting);

        fs::remove_dir(&transfer.directory).unwrap();
        fs::rename(&renamed, &transfer.directory).unwrap();
        assert!(
            transfer
                .choose(&reply(&transfer, Choice::Retry, false))
                .unwrap()
        );
        transfer
            .run(
                |index, _, _| {
                    calls.push(index);
                    Ok(fixture.completed(index))
                },
                || {},
            )
            .unwrap();
        assert_eq!(calls, [0, 1, 2]);
        assert_eq!(transfer.snapshot().unwrap().completed, 3);
    }

    #[test]
    fn uncertain_publication_and_cleanup_failures_are_visible_and_never_copied_again() {
        let fixture = Fixture::new();
        let transfer = fixture.transfer();
        let mut calls = vec![];
        transfer
            .run(
                |index, _, _| {
                    calls.push(index);
                    match index {
                        0 => Ok(CopyResult {
                            recovery_pending: true,
                            ..fixture.completed(index)
                        }),
                        1 => Err(BatchFailure::Item(FileError::CleanupFailed)),
                        _ => Ok(CopyResult {
                            cleanup_pending: true,
                            ..fixture.completed(index)
                        }),
                    }
                },
                || {},
            )
            .unwrap();
        let snapshot = transfer.snapshot().unwrap();
        assert_eq!(
            (snapshot.completed, snapshot.failed, snapshot.settled),
            (1, 2, 3)
        );
        assert!(snapshot.decision.is_none());
        assert_eq!(calls, [0, 1, 2]);
        let json = serde_json::to_value(snapshot).unwrap();
        assert_eq!(json["items"][0]["status"], "recovery-pending");
        assert_eq!(json["items"][1]["error"], "cleanup-failed");
        assert_eq!(json["items"][2]["cleanupPending"], true);
        assert!(transfer.record_recovered(0, &fixture.completed(0)).unwrap());
        assert!(!transfer.record_recovered(1, &fixture.completed(1)).unwrap());
        let recovered = transfer.snapshot().unwrap();
        assert_eq!((recovered.completed, recovered.failed), (2, 1));
    }

    #[test]
    fn window_registry_cannot_read_or_stop_another_windows_transfer() {
        let fixture = Fixture::new();
        let registry = BatchTransfers::default();
        let first = Arc::new(fixture.transfer());
        let second = Arc::new(fixture.transfer());
        registry.register("first", first.clone()).unwrap();
        registry.register("second", second.clone()).unwrap();
        assert!(matches!(
            registry.get("second", first.id),
            Err(FileError::InvalidRequest)
        ));
        registry.forget_window("first");
        assert!(registry.for_owner("first").unwrap().is_empty());
        assert_eq!(first.snapshot().unwrap().phase, Phase::Stopping);
        assert_eq!(second.snapshot().unwrap().phase, Phase::Planning);
    }
}
