mod copy;
mod native;
// The tree staging primitive is deliberately not exposed until it is wired to
// per-entry journal intents and publication recovery.
#[allow(dead_code)]
mod tree;

use crate::{
    filesystem,
    local_identity::{LiveIdentity, LiveObservation, Locator},
    storage::{MoveKey, MoveRecovery, Storage, StorageError, Store, TrackedReference},
};
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

#[derive(Debug, PartialEq, Eq, Serialize)]
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
}

#[tauri::command]
pub(crate) async fn copy_regular_file(
    window: WebviewWindow,
    request: CopyRequest,
) -> Result<CopyResult, FileError> {
    let source = parse_path(request.source)?;
    let directory = parse_path(request.directory)?;
    let request_id = request
        .request_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| FileError::InvalidRequest)?
        .unwrap_or_else(Uuid::new_v4);
    let owner = window.label().to_owned();
    let cancelled = window
        .state::<Transfers>()
        .register(owner.clone(), request_id)?;
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = copy::copy_file(&app.state::<Storage>(), &source, &directory, &cancelled);
        app.state::<Transfers>().finish(owner, request_id);
        let (path, cleanup_pending) = result?;
        Ok(CopyResult {
            path: path
                .into_os_string()
                .into_string()
                .map_err(|_| FileError::RecoveryPending)?,
            cleanup_pending,
        })
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

fn rename(storage: &Storage, path: &Path, name: &OsStr) -> Result<PathBuf, FileError> {
    let parent = path.parent().ok_or(FileError::InvalidRequest)?;
    let source_name = path.file_name().ok_or(FileError::InvalidRequest)?;
    let directory = Directory::open(parent)?;
    let witness = open_move_source(&directory, source_name)?;
    move_entry(storage, &directory, source_name, &witness, &directory, name)
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

#[tauri::command]
pub(crate) async fn move_file_or_directory(
    window: WebviewWindow,
    request: MoveRequest,
) -> Result<String, FileError> {
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let source = parse_path(request.source)?;
        let name = source.file_name().ok_or(FileError::InvalidRequest)?;
        let origin = Directory::open(source.parent().ok_or(FileError::InvalidRequest)?)?;
        let destination = Directory::open(&parse_path(request.directory)?)?;
        let source_witness = open_move_source(&origin, name)?;
        let source_kind = source_witness.kind()?;
        for sequence in 0..10_000 {
            let candidate = copy::copy_name(name, sequence, source_kind == "directory");
            match move_entry(
                &app.state::<Storage>(),
                &origin,
                name,
                &source_witness,
                &destination,
                &candidate,
            ) {
                Ok(path) => {
                    return path
                        .into_os_string()
                        .into_string()
                        .map_err(|_| FileError::RecoveryPending);
                }
                Err(FileError::AlreadyExists) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(FileError::AlreadyExists)
    })
    .await
    .map_err(|_| FileError::Unavailable)?
}

#[tauri::command]
pub(crate) async fn mutate_file(
    window: WebviewWindow,
    request: Mutation,
) -> Result<String, FileError> {
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let path = match request {
            Mutation::Create {
                directory,
                name,
                is_directory,
            } => {
                let directory = Directory::open(&parse_path(directory)?)?;
                create_in_directory(&directory, leaf(&name)?, is_directory)?
            }
            Mutation::Rename { path, name } => {
                rename(&app.state::<Storage>(), &parse_path(path)?, leaf(&name)?)?
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
