pub(crate) mod batch;
mod copy;
mod cross_file;
mod native;
pub(crate) mod removal;
#[cfg(target_os = "macos")]
mod replacement;
mod trash;
mod tree;
pub(crate) mod undo;
pub(crate) mod zip_archive;

use crate::{
    filesystem,
    local_identity::{LiveIdentity, LiveObservation, Locator},
    storage::{MoveKey, MoveRecovery, Storage, StorageError, Store, TrackedReference},
};
use batch::{BatchFailure, BatchItem, TransferKind};
use native::Directory;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ffi::OsStr,
    io,
    path::{Component, Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{Manager, WebviewWindow};
use uuid::Uuid;

const MAX_PATH_BYTES: usize = 128 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum FileError {
    InvalidRequest,
    NotFound,
    PermissionDenied,
    AlreadyExists,
    Unsupported,
    SourceChanged,
    StorageFull,
    CleanupFailed,
    Cancelled,
    RecoveryPending,
    Unavailable,
}

impl From<io::Error> for FileError {
    fn from(error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::NotFound => Self::NotFound,
            io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            io::ErrorKind::AlreadyExists => Self::AlreadyExists,
            io::ErrorKind::Unsupported | io::ErrorKind::CrossesDevices => Self::Unsupported,
            io::ErrorKind::StorageFull => Self::StorageFull,
            io::ErrorKind::InvalidInput | io::ErrorKind::InvalidFilename => Self::InvalidRequest,
            _ => Self::Unavailable,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CopyRequest {
    source: String,
    directory: String,
    #[serde(rename = "requestId")]
    request_id: Option<String>,
}

#[derive(Default)]
pub(crate) struct Transfers(Mutex<HashMap<(String, Uuid), Arc<AtomicBool>>>);

impl Transfers {
    fn register(&self, owner: String, id: Uuid) -> Result<Arc<AtomicBool>, FileError> {
        use std::collections::hash_map::Entry;

        let mut active = self.0.lock().map_err(|_| FileError::Unavailable)?;
        let Entry::Vacant(entry) = active.entry((owner, id)) else {
            return Err(FileError::InvalidRequest);
        };
        let cancelled = Arc::new(AtomicBool::new(false));
        entry.insert(cancelled.clone());
        Ok(cancelled)
    }

    fn finish(&self, owner: String, id: Uuid) {
        if let Ok(mut active) = self.0.lock() {
            active.remove(&(owner, id));
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CopyResult {
    path: String,
    cleanup_pending: bool,
    recovery_pending: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    staging_path: Option<String>,
}

impl CopyResult {
    fn completed(path: PathBuf, cleanup_pending: bool) -> Result<Self, FileError> {
        Ok(Self {
            path: path
                .into_os_string()
                .into_string()
                .map_err(|_| FileError::RecoveryPending)?,
            cleanup_pending,
            recovery_pending: false,
            staging_path: None,
        })
    }
}

enum PendingCopy {
    File(Box<copy::PendingFileCopy>),
    Tree(tree::PendingTreeCopy),
    CrossFile(Box<cross_file::PendingCrossFile>),
    CrossTree(Box<tree::cross_tree::CrossTreeMove>),
    #[cfg(target_os = "macos")]
    Replacement(Box<replacement::ReplacementRecovery>),
}

impl PendingCopy {
    fn path(&self) -> &Path {
        match self {
            Self::File(copy) => copy.path(),
            Self::Tree(copy) => copy.path(),
            Self::CrossFile(movement) => movement.path(),
            Self::CrossTree(movement) => movement.path(),
            #[cfg(target_os = "macos")]
            Self::Replacement(copy) => copy.destination_path(),
        }
    }

    fn commit(&mut self, storage: &Storage) -> Result<(PathBuf, bool), FileError> {
        match self {
            Self::File(copy) => copy.commit(storage),
            Self::Tree(copy) => copy.commit(storage),
            Self::CrossFile(movement) => movement.commit(storage),
            Self::CrossTree(movement) => movement.commit(storage),
            #[cfg(target_os = "macos")]
            Self::Replacement(copy) => copy.retry(storage),
        }
    }
}

#[derive(Default)]
pub(crate) struct CopyRecoveries(Mutex<HashMap<(String, Uuid, usize), PendingCopy>>);

impl CopyRecoveries {
    fn remember(
        &self,
        owner: &str,
        request: Uuid,
        index: usize,
        pending: PendingCopy,
    ) -> Result<(), FileError> {
        let mut copies = self.0.lock().map_err(|_| FileError::RecoveryPending)?;
        let entry = copies.entry((owner.to_owned(), request, index));
        match entry {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(pending);
                Ok(())
            }
            std::collections::hash_map::Entry::Occupied(_) => Err(FileError::RecoveryPending),
        }
    }

    fn has_pending(&self, owner: &str, request: Uuid, index: usize) -> Result<bool, FileError> {
        let copies = self.0.lock().map_err(|_| FileError::Unavailable)?;
        Ok(copies.contains_key(&(owner.to_owned(), request, index)))
    }

    fn retry(
        &self,
        owner: &str,
        request: Uuid,
        index: usize,
        storage: &Storage,
    ) -> Result<(PathBuf, bool), FileError> {
        let mut copies = self.0.lock().map_err(|_| FileError::RecoveryPending)?;
        let key = (owner.to_owned(), request, index);
        let copy = copies.get_mut(&key).ok_or(FileError::RecoveryPending)?;
        let output = copy.commit(storage)?;
        copies.remove(&key);
        Ok(output)
    }
}

fn finish_pending(
    app: &tauri::AppHandle,
    owner: &str,
    request_id: Uuid,
    index: usize,
    pending: PendingCopy,
) -> Result<CopyResult, FileError> {
    let path = pending.path().to_owned();
    let staging_path = match &pending {
        PendingCopy::CrossFile(movement) => {
            Some(movement.staging_path().to_string_lossy().into_owned())
        }
        PendingCopy::CrossTree(movement) => {
            Some(movement.staging_path().to_string_lossy().into_owned())
        }
        #[cfg(target_os = "macos")]
        PendingCopy::Replacement(copy) => Some(copy.staging_path().to_string_lossy().into_owned()),
        _ => None,
    };
    let mut result = CopyResult::completed(path, false)?;
    result.staging_path = staging_path;
    #[cfg(target_os = "macos")]
    if let PendingCopy::Replacement(copy) = &pending {
        result.cleanup_pending = copy.metadata_committed();
    }
    app.state::<CopyRecoveries>()
        .remember(owner, request_id, index, pending)?;
    result.recovery_pending = true;
    Ok(result)
}

#[tauri::command]
pub(crate) async fn copy_regular_file(
    window: WebviewWindow,
    request: CopyRequest,
) -> Result<CopyResult, FileError> {
    copy_single(window, request, false).await
}

#[tauri::command]
pub(crate) async fn copy_directory(
    window: WebviewWindow,
    request: CopyRequest,
) -> Result<CopyResult, FileError> {
    copy_single(window, request, true).await
}

async fn copy_single(
    window: WebviewWindow,
    request: CopyRequest,
    is_directory: bool,
) -> Result<CopyResult, FileError> {
    let source = parse_path(request.source)?;
    let destination = parse_path(request.directory)?;
    let request_id = request
        .request_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| FileError::InvalidRequest)?
        .unwrap_or_else(Uuid::new_v4);
    let owner = window.label().to_owned();
    let app = window.app_handle().clone();
    let cancelled = app
        .state::<Transfers>()
        .register(owner.clone(), request_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| {
            if app
                .state::<CopyRecoveries>()
                .has_pending(&owner, request_id, 0)?
            {
                return Err(FileError::RecoveryPending);
            }
            let directory = Directory::open(&destination)?;
            copy_item(
                BatchItem {
                    app: &app,
                    owner: &owner,
                    operation: request_id,
                    index: 0,
                    attempt: 0,
                    kind: TransferKind::Copy,
                    source: &source,
                    destination: &directory,
                    cancelled: &cancelled,
                },
                is_directory,
            )
        })();
        if result.as_ref().is_ok_and(|result| !result.recovery_pending) {
            app.state::<undo::UndoSlots>().record(&app, &owner, None);
        }
        app.state::<Transfers>().finish(owner, request_id);
        result
    })
    .await
    .map_err(|_| FileError::Unavailable)?
}

fn copy_item(item: BatchItem<'_>, is_directory: bool) -> Result<CopyResult, FileError> {
    let BatchItem {
        app,
        owner,
        operation: request,
        index,
        attempt,
        source,
        destination,
        cancelled,
        ..
    } = item;
    let operation = request.to_string();
    let item_key = format!("{index}:{attempt}");
    let storage = app.state::<Storage>();
    if is_directory {
        match tree::copy_tree(
            &storage,
            source,
            destination,
            (&operation, &item_key),
            cancelled,
        )? {
            tree::TreeOutcome::Completed(path, cleanup) => CopyResult::completed(path, cleanup),
            tree::TreeOutcome::Pending(pending) => {
                finish_pending(app, owner, request, index, PendingCopy::Tree(pending))
            }
        }
    } else {
        match copy::copy_file_in(
            &storage,
            source,
            destination,
            (&operation, &item_key),
            cancelled,
        )? {
            copy::FileOutcome::Completed(path, cleanup) => CopyResult::completed(path, cleanup),
            copy::FileOutcome::Pending(pending) => {
                finish_pending(app, owner, request, index, PendingCopy::File(pending))
            }
        }
    }
}

fn perform_batch_item(item: BatchItem<'_>) -> Result<CopyResult, BatchFailure> {
    let destination = item.destination;
    let outcome = match item.kind {
        TransferKind::Copy => {
            let metadata = std::fs::symlink_metadata(item.source)
                .map_err(|error| BatchFailure::Item(error.into()))?;
            if metadata.is_dir() || metadata.is_file() {
                let result = copy_item(item, metadata.is_dir());
                if result.as_ref().is_ok_and(|result| !result.recovery_pending) {
                    item.app.state::<undo::UndoSlots>().record_batch_item(
                        item.owner,
                        item.operation,
                        None,
                    );
                }
                result
            } else {
                Err(FileError::Unsupported)
            }
        }
        TransferKind::Move => move_batch_item(item),
    };
    match outcome {
        Ok(result) => Ok(result),
        Err(
            error @ (FileError::RecoveryPending | FileError::CleanupFailed | FileError::Cancelled),
        ) => Err(BatchFailure::Item(error)),
        Err(error) if error == FileError::StorageFull || destination.verify().is_err() => {
            Err(BatchFailure::Destination(error))
        }
        Err(error) => Err(BatchFailure::Item(error)),
    }
}

fn move_batch_item(item: BatchItem<'_>) -> Result<CopyResult, FileError> {
    let BatchItem {
        app,
        owner,
        operation,
        index,
        attempt,
        source,
        destination,
        cancelled,
        ..
    } = item;
    let storage = app.state::<Storage>();
    let origin = Directory::open(source.parent().ok_or(FileError::InvalidRequest)?)?;
    if origin.same_volume(destination)? {
        let movement = move_in_directory(&storage, source, &origin, destination, cancelled)?;
        if movement.changed {
            app.state::<undo::UndoSlots>()
                .record_batch_item(owner, operation, movement.undo);
        }
        return CopyResult::completed(movement.path, false);
    }
    let operation_id = operation.to_string();
    let item_key = format!("{index}:{attempt}");
    let metadata = std::fs::symlink_metadata(source)?;
    let result = if metadata.is_dir() {
        match tree::cross_tree::move_tree(
            &storage,
            source,
            destination,
            (&operation_id, &item_key),
            cancelled,
        )? {
            tree::cross_tree::CrossTreeOutcome::Completed(path, cleanup) => {
                CopyResult::completed(path, cleanup)
            }
            tree::cross_tree::CrossTreeOutcome::Pending(movement) => finish_pending(
                app,
                owner,
                operation,
                index,
                PendingCopy::CrossTree(movement),
            ),
        }
    } else if metadata.is_file() {
        match cross_file::move_file(
            &storage,
            source,
            destination,
            (&operation_id, &item_key),
            cancelled,
        )? {
            cross_file::CrossFileOutcome::Completed(path, cleanup) => {
                CopyResult::completed(path, cleanup)
            }
            cross_file::CrossFileOutcome::Pending(movement) => finish_pending(
                app,
                owner,
                operation,
                index,
                PendingCopy::CrossFile(movement),
            ),
        }
    } else {
        Err(FileError::Unsupported)
    };
    if result.as_ref().is_ok_and(|result| !result.recovery_pending) {
        app.state::<undo::UndoSlots>()
            .record_batch_item(owner, operation, None);
    }
    result
}

#[tauri::command]
pub(crate) async fn recover_copy_transfer(
    window: WebviewWindow,
    request_id: String,
    item_index: Option<usize>,
) -> Result<CopyResult, FileError> {
    let request_id = Uuid::parse_str(&request_id).map_err(|_| FileError::InvalidRequest)?;
    let owner = window.label().to_owned();
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (path, cleanup) = app.state::<CopyRecoveries>().retry(
            &owner,
            request_id,
            item_index.unwrap_or(0),
            &app.state::<Storage>(),
        )?;
        let result = CopyResult::completed(path, cleanup)?;
        if item_index.is_none() {
            app.state::<undo::UndoSlots>().record(&app, &owner, None);
        }
        if let Some(index) = item_index {
            app.state::<batch::BatchTransfers>()
                .record_recovered(&app, &owner, request_id, index, &result);
        }
        Ok(result)
    })
    .await
    .map_err(|_| FileError::Unavailable)?
}

#[tauri::command]
pub(crate) fn stop_copy(window: WebviewWindow, request_id: String) -> Result<bool, FileError> {
    let request_id = Uuid::parse_str(&request_id).map_err(|_| FileError::InvalidRequest)?;
    let transfers = window.state::<Transfers>();
    let active = transfers.0.lock().map_err(|_| FileError::Unavailable)?;
    if let Some(signal) = active.get(&(window.label().to_owned(), request_id)) {
        signal.store(true, Ordering::Release);
        return Ok(true);
    }
    Ok(false)
}

pub(crate) fn window_has_transfers(app: &tauri::AppHandle, owner: &str) -> Result<bool, FileError> {
    let transfers = app.state::<Transfers>();
    let active = transfers.0.lock().map_err(|_| FileError::Unavailable)?;
    Ok(active.keys().any(|(window, _)| window == owner)
        || app.state::<batch::BatchTransfers>().has_active(owner)?)
}

#[tauri::command]
pub(crate) fn has_window_transfers(window: WebviewWindow) -> Result<bool, FileError> {
    window_has_transfers(window.app_handle(), window.label())
}

#[tauri::command]
pub(crate) async fn stop_window_transfers(window: WebviewWindow) -> Result<(), FileError> {
    let app = window.app_handle().clone();
    let owner = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        {
            let transfers = app.state::<Transfers>();
            let active = transfers.0.lock().map_err(|_| FileError::Unavailable)?;
            for ((window, _), signal) in active.iter() {
                if window == &owner {
                    signal.store(true, Ordering::Release);
                }
            }
        }
        app.state::<batch::BatchTransfers>()
            .stop_all(&app, &owner)?;
        while window_has_transfers(&app, &owner)? {
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        Ok(())
    })
    .await
    .map_err(|_| FileError::Unavailable)?
}

#[cfg(target_os = "macos")]
#[derive(Default)]
pub(crate) struct ReplacementApprovals(
    Mutex<HashMap<String, (Uuid, replacement::ReplacementTarget)>>,
);

#[cfg(target_os = "macos")]
impl ReplacementApprovals {
    pub(crate) fn forget_window(&self, owner: &str) {
        if let Ok(mut approvals) = self.0.lock() {
            approvals.remove(owner);
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReplacementApproval {
    request_id: String,
    path: String,
}

#[tauri::command]
pub(crate) async fn prepare_file_replacement(
    window: WebviewWindow,
    path: String,
) -> Result<ReplacementApproval, FileError> {
    #[cfg(windows)]
    {
        let _ = (window, path);
        Err(FileError::Unsupported)
    }
    #[cfg(target_os = "macos")]
    {
        let path = parse_path(path)?;
        let owner = window.label().to_owned();
        let app = window.app_handle().clone();
        tauri::async_runtime::spawn_blocking(move || {
            let candidate = replacement::ReplacementTarget::capture(&path)?;
            if app.get_webview_window(&owner).is_none() {
                return Err(FileError::Cancelled);
            }
            let request_id = Uuid::new_v4();
            let path = candidate
                .path()
                .to_str()
                .ok_or(FileError::InvalidRequest)?
                .to_owned();
            let approvals = app.state::<ReplacementApprovals>();
            approvals
                .0
                .lock()
                .map_err(|_| FileError::Unavailable)?
                .insert(owner, (request_id, candidate));
            Ok(ReplacementApproval {
                request_id: request_id.to_string(),
                path,
            })
        })
        .await
        .map_err(|_| FileError::Unavailable)?
    }
}

#[tauri::command]
pub(crate) async fn replace_regular_file(
    window: WebviewWindow,
    source: String,
    request_id: String,
) -> Result<CopyResult, FileError> {
    #[cfg(windows)]
    {
        let _ = (window, source, request_id);
        Err(FileError::Unsupported)
    }
    #[cfg(target_os = "macos")]
    {
        let source = parse_path(source)?;
        let request_id = Uuid::parse_str(&request_id).map_err(|_| FileError::InvalidRequest)?;
        let app = window.app_handle().clone();
        let owner = window.label().to_owned();
        let target = {
            let approvals = app.state::<ReplacementApprovals>();
            let mut approvals = approvals.0.lock().map_err(|_| FileError::Unavailable)?;
            if approvals
                .get(&owner)
                .is_none_or(|(id, _)| *id != request_id)
            {
                return Err(FileError::InvalidRequest);
            }
            approvals.remove(&owner).ok_or(FileError::InvalidRequest)?.1
        };
        let cancelled = app
            .state::<Transfers>()
            .register(owner.clone(), request_id)?;
        tauri::async_runtime::spawn_blocking(move || {
            let result = match replacement::replace_file(
                &app.state::<Storage>(),
                &source,
                target,
                &cancelled,
            ) {
                Ok(replacement::ReplacementOutcome::Completed {
                    path,
                    cleanup_pending,
                }) => CopyResult::completed(path, cleanup_pending),
                Ok(replacement::ReplacementOutcome::RecoveryPending(pending)) => finish_pending(
                    &app,
                    &owner,
                    request_id,
                    0,
                    PendingCopy::Replacement(pending),
                ),
                Err(error) => Err(error),
            };
            if result.as_ref().is_ok_and(|result| !result.recovery_pending) {
                app.state::<undo::UndoSlots>().record(&app, &owner, None);
            }
            app.state::<Transfers>().finish(owner, request_id);
            result
        })
        .await
        .map_err(|_| FileError::Unavailable)?
    }
}

impl From<StorageError> for FileError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::ReferenceConflict => Self::RecoveryPending,
            _ => Self::Unavailable,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum Mutation {
    Create {
        directory: String,
        name: String,
        is_directory: bool,
    },
    Rename {
        path: String,
        name: String,
    },
}

fn parse_path(path: String) -> Result<PathBuf, FileError> {
    if path.len() > MAX_PATH_BYTES {
        return Err(FileError::InvalidRequest);
    }
    let path = filesystem::native_path(path).map_err(|_| FileError::InvalidRequest)?;
    if path
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(FileError::InvalidRequest);
    }
    Ok(path)
}

fn leaf(value: &str) -> Result<&OsStr, FileError> {
    let mut parts = Path::new(value).components();
    let (Some(Component::Normal(name)), None) = (parts.next(), parts.next()) else {
        return Err(FileError::InvalidRequest);
    };
    if name != OsStr::new(value) || value.as_bytes().contains(&0) {
        return Err(FileError::InvalidRequest);
    }
    #[cfg(windows)]
    {
        let stem = value
            .split('.')
            .next()
            .unwrap_or_default()
            .trim_end()
            .to_ascii_uppercase();
        let port = stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"));
        let reserved_port = matches!(
            port,
            Some("1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³")
        );
        if matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        ) || reserved_port
            || value.ends_with([' ', '.'])
            || value
                .chars()
                .any(|ch| ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
        {
            return Err(FileError::InvalidRequest);
        }
    }
    Ok(name)
}

fn create_in_directory(
    directory: &Directory,
    name: &OsStr,
    is_directory: bool,
) -> Result<PathBuf, FileError> {
    directory.verify()?;
    if is_directory {
        directory.create_directory(name)?;
    } else {
        directory.create_file(name)?;
    }
    directory.verify().map_err(|_| FileError::RecoveryPending)?;
    Ok(directory.path().join(name))
}

fn captured_directory_path(directory: &Directory) -> Result<PathBuf, FileError> {
    directory.verify()?;
    let identity = LiveIdentity::from_file(directory.try_clone_file()?)?;
    match identity.observe()? {
        LiveObservation::Present(locator) => Ok(locator.path().to_owned()),
        _ => Err(FileError::RecoveryPending),
    }
}

struct TrackedMove {
    id: String,
    root: TrackedReference,
    descendants: Vec<(String, TrackedReference)>,
}

fn tracked_move(store: &mut Store, source: &Locator) -> Result<TrackedMove, StorageError> {
    let id = store
        .resolve_live_item(source, true)?
        .ok_or(StorageError::ReferenceConflict)?;
    let root = store.attach_live_reference(&id, source)?;
    let descendants = if root.is_directory()? {
        store.tracked_descendants(source)?
    } else {
        Vec::new()
    };
    Ok(TrackedMove {
        id,
        root,
        descendants,
    })
}

struct CompletedMove {
    path: PathBuf,
    undo: Option<undo::UndoMove>,
    changed: bool,
}

fn remember_move(
    origin: &Directory,
    old_name: &OsStr,
    destination: &Directory,
    path: PathBuf,
    witness: LiveIdentity,
    old_location: &LiveObservation,
) -> CompletedMove {
    let changed = witness.observe().is_ok_and(|now| &now != old_location);
    let undo = if changed {
        undo::UndoMove::capture(origin, old_name, destination, &path, witness).ok()
    } else {
        None
    };
    CompletedMove {
        path,
        undo,
        changed,
    }
}

fn rename(storage: &Storage, path: &Path, name: &OsStr) -> Result<CompletedMove, FileError> {
    let parent = path.parent().ok_or(FileError::InvalidRequest)?;
    let source_name = path.file_name().ok_or(FileError::InvalidRequest)?;
    let directory = Directory::open(parent)?;
    let witness = open_move_source(&directory, source_name)?;
    let old_location = witness.observe()?;
    let path = move_entry(storage, &directory, source_name, &witness, &directory, name)?;
    Ok(remember_move(
        &directory,
        source_name,
        &directory,
        path,
        witness,
        &old_location,
    ))
}

fn open_move_source(origin: &Directory, name: &OsStr) -> Result<LiveIdentity, FileError> {
    let source = Locator::new(captured_directory_path(origin)?.join(name))
        .ok_or(FileError::InvalidRequest)?;
    Ok(LiveIdentity::open(&source)?)
}

fn move_entry(
    storage: &Storage,
    origin: &Directory,
    source_name: &OsStr,
    source_witness: &LiveIdentity,
    destination: &Directory,
    name: &OsStr,
) -> Result<PathBuf, FileError> {
    let canonical_parent = captured_directory_path(origin)?;
    let canonical_destination = captured_directory_path(destination)?;
    let source =
        Locator::new(canonical_parent.join(source_name)).ok_or(FileError::InvalidRequest)?;
    let target = Locator::new(canonical_destination.join(name)).ok_or(FileError::InvalidRequest)?;
    if source == target {
        if source_witness.observe()? != LiveObservation::Present(source) {
            return Err(FileError::SourceChanged);
        }
        return Ok(destination.path().join(name));
    }
    if target.path().starts_with(source.path()) {
        return Err(FileError::InvalidRequest);
    }
    origin.verify()?;
    destination.verify()?;
    let operation = Uuid::new_v4().to_string();
    let TrackedMove {
        id,
        mut root,
        mut descendants,
    } = storage
        .access(|store| {
            let tracked = tracked_move(store, &source)?;
            if !tracked.root.matches_open_identity(source_witness)? {
                return Ok(None);
            }
            if tracked.root.is_directory()? {
                store.record_tree_move(
                    &operation,
                    &tracked.root,
                    &tracked
                        .descendants
                        .iter()
                        .map(|(_, row)| row)
                        .collect::<Vec<_>>(),
                    &target,
                )?;
            } else {
                store.record_move(
                    MoveKey {
                        operation_id: &operation,
                        item_key: &tracked.id,
                    },
                    &tracked.root,
                    &target,
                )?;
            }
            Ok(Some(tracked))
        })?
        .ok_or(FileError::SourceChanged)?;
    root.verify_source()
        .map_err(|_| FileError::RecoveryPending)?;
    if let Err(error) = origin
        .verify()
        .and_then(|_| destination.verify())
        .and_then(|_| native::publish(origin, source_name, destination, name))
    {
        return match storage.access(|store| {
            store.fail_unpublished_move(
                &operation,
                std::iter::once(&root).chain(descendants.iter().map(|(_, row)| row)),
            )
        }) {
            Ok(()) => Err(error.into()),
            Err(_) => Err(FileError::RecoveryPending),
        };
    }
    destination
        .verify()
        .map_err(|_| FileError::RecoveryPending)?;
    let completion = storage.access(|store| {
        for (id, witness) in std::iter::once((&id, &mut root))
            .chain(descendants.iter_mut().map(|(id, witness)| (&*id, witness)))
        {
            let key = MoveKey {
                operation_id: &operation,
                item_key: id,
            };
            if store.recover_move(key, Some(witness))? != MoveRecovery::Completed {
                return Err(StorageError::ReferenceConflict);
            }
        }
        Ok(())
    });
    completion.map_err(|_| FileError::RecoveryPending)?;
    destination
        .verify()
        .map_err(|_| FileError::RecoveryPending)?;
    Ok(destination.path().join(name))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MoveRequest {
    source: String,
    directory: String,
}

fn move_to_directory(
    storage: &Storage,
    source: &Path,
    destination: &Directory,
    cancelled: &AtomicBool,
) -> Result<CompletedMove, FileError> {
    let origin = Directory::open(source.parent().ok_or(FileError::InvalidRequest)?)?;
    move_in_directory(storage, source, &origin, destination, cancelled)
}

fn move_in_directory(
    storage: &Storage,
    source: &Path,
    origin: &Directory,
    destination: &Directory,
    cancelled: &AtomicBool,
) -> Result<CompletedMove, FileError> {
    let name = source.file_name().ok_or(FileError::InvalidRequest)?;
    let source_witness = open_move_source(origin, name)?;
    let source_kind = source_witness.kind()?;
    let old_location = source_witness.observe()?;
    for sequence in 0..10_000 {
        copy::check_cancellation(cancelled)?;
        let candidate = copy::copy_name(name, sequence, source_kind == "directory");
        match move_entry(
            storage,
            origin,
            name,
            &source_witness,
            destination,
            &candidate,
        ) {
            Err(FileError::AlreadyExists) => continue,
            Ok(path) => {
                return Ok(remember_move(
                    origin,
                    name,
                    destination,
                    path,
                    source_witness,
                    &old_location,
                ));
            }
            Err(error) => return Err(error),
        }
    }
    Err(FileError::AlreadyExists)
}

#[tauri::command]
pub(crate) async fn move_file_or_directory(
    window: WebviewWindow,
    request: MoveRequest,
) -> Result<String, FileError> {
    let owner = window.label().to_owned();
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let source = parse_path(request.source)?;
        let destination = Directory::open(&parse_path(request.directory)?)?;
        let movement = move_to_directory(
            &app.state::<Storage>(),
            &source,
            &destination,
            &AtomicBool::new(false),
        )?;
        if movement.changed {
            app.state::<undo::UndoSlots>()
                .record(&app, &owner, movement.undo);
        }
        movement
            .path
            .into_os_string()
            .into_string()
            .map_err(|_| FileError::RecoveryPending)
    })
    .await
    .map_err(|_| FileError::Unavailable)?
}

#[tauri::command]
pub(crate) async fn mutate_file(
    window: WebviewWindow,
    request: Mutation,
) -> Result<String, FileError> {
    let owner = window.label().to_owned();
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let path = match request {
            Mutation::Create {
                directory,
                name,
                is_directory,
            } => {
                let directory = Directory::open(&parse_path(directory)?)?;
                let path = create_in_directory(&directory, leaf(&name)?, is_directory)?;
                app.state::<undo::UndoSlots>().record(&app, &owner, None);
                path
            }
            Mutation::Rename { path, name } => {
                let movement = rename(&app.state::<Storage>(), &parse_path(path)?, leaf(&name)?)?;
                if movement.changed {
                    app.state::<undo::UndoSlots>()
                        .record(&app, &owner, movement.undo);
                }
                movement.path
            }
        };
        path.into_os_string()
            .into_string()
            .map_err(|_| FileError::RecoveryPending)
    })
    .await
    .map_err(|_| FileError::Unavailable)?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_mutations_accept_only_one_native_leaf_and_absolute_paths() {
        for invalid in ["", ".", "..", "/tmp/item", "nested/item", "item/", "a\0b"] {
            assert_eq!(leaf(invalid), Err(FileError::InvalidRequest));
        }
        assert!(leaf("prova caffè.txt").is_ok());
        assert!(parse_path("relative".into()).is_err());
    }
}
