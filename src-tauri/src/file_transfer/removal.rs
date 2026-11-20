use super::{
    FileError, captured_directory_path,
    native::Directory,
    open_move_source, parse_path, tracked_move, trash,
    undo::{UndoMove, UndoSlots},
};
use crate::{
    local_identity::LiveObservation,
    storage::{Storage, StorageError},
};
use serde::Serialize;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use tauri::{Manager, WebviewWindow};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemovalReport {
    completed: usize,
    problem: Option<RemovalProblem>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemovalProblem {
    path: PathBuf,
    error: FileError,
    retained_path: Option<PathBuf>,
}

fn selection(sources: Vec<String>) -> Result<Vec<PathBuf>, FileError> {
    let paths = sources
        .into_iter()
        .map(parse_path)
        .collect::<Result<Vec<_>, _>>()?;
    let distinct: HashSet<_> = paths.iter().collect();
    if paths.is_empty()
        || distinct.len() != paths.len()
        || paths.iter().any(|path| path.file_name().is_none())
    {
        return Err(FileError::InvalidRequest);
    }
    Ok(paths)
}

fn trash_one(storage: &Storage, path: &Path) -> Result<Option<UndoMove>, FileError> {
    let original_name = path.file_name().ok_or(FileError::InvalidRequest)?;
    let original_dir = Directory::open(path.parent().ok_or(FileError::InvalidRequest)?)?;
    let expected = open_move_source(&original_dir, original_name)?;
    let LiveObservation::Present(original) = expected.observe()? else {
        return Err(FileError::SourceChanged);
    };
    let mut tracked = storage.access(|store| {
        let row = tracked_move(store, &original)?;
        if row.root.matches_open_identity(&expected)? {
            Ok(row)
        } else {
            Err(StorageError::ReferenceConflict)
        }
    })?;
    let trashed = trash::trash_verified(path, &expected)?;
    if trashed.original != original {
        return Err(FileError::RecoveryPending);
    }
    // The original tracking handles predate the trash effect. Reconcile only
    // the move to the witnessed system location, including tracked descendants.
    let reconcile = storage.access(|store| {
        for (id, reference) in std::iter::once((&tracked.id, &mut tracked.root)).chain(
            tracked
                .descendants
                .iter_mut()
                .map(|(id, entry)| (&*id, entry)),
        ) {
            if store.reconcile_live_reference(reference).is_err() {
                let LiveObservation::Present(location) = reference.current_observation()? else {
                    return Err(StorageError::ReferenceConflict);
                };
                if store.resolve_live_item(&location, false)?.as_ref() != Some(id) {
                    return Err(StorageError::ReferenceConflict);
                }
            }
        }
        Ok(())
    });
    if reconcile.is_err() {
        return Err(FileError::RecoveryPending);
    }
    let Some(restore) = trashed.restoration else {
        return Ok(None);
    };
    let bin = Directory::open(
        restore
            .location
            .path()
            .parent()
            .ok_or(FileError::RecoveryPending)?,
    )?;
    if captured_directory_path(&bin)?.as_path()
        != restore
            .location
            .path()
            .parent()
            .ok_or(FileError::RecoveryPending)?
    {
        return Ok(None);
    }
    Ok(UndoMove::capture(
        &original_dir,
        original_name,
        &bin,
        restore.location.path(),
        restore.witness,
    )
    .ok())
}

#[tauri::command]
pub(crate) async fn trash_files(
    window: WebviewWindow,
    sources: Vec<String>,
) -> Result<RemovalReport, FileError> {
    let sources = selection(sources)?;
    let owner = window.label().to_owned();
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut completed = 0;
        let mut problem = None;
        let mut undo = Vec::new();
        for path in sources {
            match trash_one(&app.state::<Storage>(), &path) {
                Ok(inverse) => {
                    completed += 1;
                    undo.extend(inverse);
                }
                Err(error) => {
                    problem = Some(RemovalProblem {
                        path,
                        error,
                        retained_path: None,
                    });
                    break;
                }
            }
        }
        if completed != 0 {
            app.state::<UndoSlots>().record_action(&app, &owner, undo);
        }
        Ok(RemovalReport { completed, problem })
    })
    .await
    .map_err(|_| FileError::Unavailable)?
}

#[derive(Default)]
pub(crate) struct RemovalApprovals(
    std::sync::Mutex<std::collections::HashMap<String, (uuid::Uuid, Vec<UndoMove>)>>,
);

impl RemovalApprovals {
    pub(crate) fn forget_window(&self, owner: &str) {
        if let Ok(mut windows) = self.0.lock() {
            windows.remove(owner);
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemovalApproval {
    request_id: String,
    paths: Vec<PathBuf>,
}

#[tauri::command]
pub(crate) async fn prepare_permanent_deletion(
    window: WebviewWindow,
    sources: Vec<String>,
) -> Result<RemovalApproval, FileError> {
    let sources = selection(sources)?;
    let owner = window.label().to_owned();
    let app = window.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let previews = sources
            .iter()
            .map(|path| {
                let name = path.file_name().ok_or(FileError::InvalidRequest)?;
                let parent = Directory::open(path.parent().ok_or(FileError::InvalidRequest)?)?;
                let witness = open_move_source(&parent, name)?;
                UndoMove::capture(&parent, name, &parent, path, witness)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut paths: Vec<_> = previews
            .iter()
            .map(|item| item.location().path().to_owned())
            .collect();
        paths.sort();
        if paths
            .windows(2)
            .any(|items| items[1].starts_with(&items[0]))
        {
            return Err(FileError::InvalidRequest);
        }
        if app.get_webview_window(&owner).is_none() {
            return Err(FileError::Cancelled);
        }
        let request_id = uuid::Uuid::new_v4();
        app.state::<RemovalApprovals>()
            .0
            .lock()
            .map_err(|_| FileError::Unavailable)?
            .insert(owner, (request_id, previews));
        Ok(RemovalApproval {
            request_id: request_id.to_string(),
            paths,
        })
    })
    .await
    .map_err(|_| FileError::Unavailable)?
}

#[tauri::command]
pub(crate) async fn permanently_delete_files(
    window: WebviewWindow,
    request_id: String,
) -> Result<RemovalReport, FileError> {
    let request_id = uuid::Uuid::parse_str(&request_id).map_err(|_| FileError::InvalidRequest)?;
    let owner = window.label().to_owned();
    let app = window.app_handle().clone();
    let targets = {
        let state = app.state::<RemovalApprovals>();
        let mut state = state.0.lock().map_err(|_| FileError::Unavailable)?;
        if state.get(&owner).is_none_or(|(id, _)| *id != request_id) {
            return Err(FileError::InvalidRequest);
        }
        state.remove(&owner).ok_or(FileError::InvalidRequest)?.1
    };
    tauri::async_runtime::spawn_blocking(move || {
        let mut completed = 0;
        let mut problem = None;
        for target in targets {
            let path = target.location().path().to_owned();
            match permanent_one(&app.state::<Storage>(), &target) {
                Ok(warning) => {
                    completed += 1;
                    if let Some((error, retained_path)) = warning {
                        problem = Some(RemovalProblem {
                            path,
                            error,
                            retained_path,
                        });
                        break;
                    }
                }
                Err(error) => {
                    problem = Some(RemovalProblem {
                        path,
                        error: error.error,
                        retained_path: error.retained_path,
                    });
                    break;
                }
            }
        }
        if completed != 0 {
            app.state::<UndoSlots>().record(&app, &owner, None);
        }
        Ok(RemovalReport { completed, problem })
    })
    .await
    .map_err(|_| FileError::Unavailable)?
}

fn permanent_one(
    storage: &Storage,
    target: &UndoMove,
) -> Result<Option<(FileError, Option<PathBuf>)>, super::undo::PermanentFailure> {
    let path = target.location().path();
    let name = path.file_name().ok_or(FileError::InvalidRequest)?;
    let parent = Directory::open(path.parent().ok_or(FileError::InvalidRequest)?)
        .map_err(FileError::from)?;
    let identity = open_move_source(&parent, name)?;
    if !target.matches_witness(&identity)? {
        return Err(FileError::SourceChanged.into());
    }
    let mut tracked = storage
        .access(|store| {
            let selected = tracked_move(store, target.location())?;
            if !selected.root.matches_open_identity(&identity)? {
                return Err(StorageError::ReferenceConflict);
            }
            #[cfg(target_os = "macos")]
            for entry in std::iter::once(&selected.root)
                .chain(selected.descendants.iter().map(|(_, entry)| entry))
            {
                entry.arm_removal()?;
            }
            Ok(selected)
        })
        .map_err(FileError::from)?;
    let cleanup_path = target.permanently_remove()?;
    let reconciled = storage.access(|store| {
        for (id, entry) in std::iter::once((&tracked.id, &mut tracked.root)).chain(
            tracked
                .descendants
                .iter_mut()
                .map(|(id, entry)| (&*id, entry)),
        ) {
            if store.reconcile_live_reference(entry) != Ok(LiveObservation::Removed) {
                let missing: bool = store.connection.query_row(
                    "SELECT EXISTS(SELECT 1 FROM items WHERE id=?1 AND availability='missing')",
                    [id],
                    |row| row.get(0),
                )?;
                if !missing {
                    return Err(StorageError::ReferenceConflict);
                }
            }
        }
        Ok(())
    });
    if reconciled.is_err() {
        return Ok(Some((FileError::RecoveryPending, cleanup_path)));
    }
    Ok(cleanup_path.map(|path| (FileError::CleanupFailed, Some(path))))
}
