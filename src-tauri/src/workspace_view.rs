use crate::{
    filesystem,
    local_identity::Locator,
    organization::OrganizationReferences,
    storage::{Storage, StorageError, Store},
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewWindow};

const DEFAULT_KEY: &str = "workspace.views.default.v1";
const MAX_PATH_BYTES: usize = 128 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Mode {
    Details,
    Grid,
    Columns,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Sort {
    Name,
    Kind,
    Size,
    Modified,
    Created,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum GroupBy {
    None,
    Kind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum ColumnId {
    Name,
    Kind,
    Size,
    Modified,
    Created,
    Tags,
    Note,
    Indexing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Column {
    id: ColumnId,
    width: u16,
    visible: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ViewOptions {
    mode: Mode,
    sort: Sort,
    descending: bool,
    folders_first: bool,
    group_by: GroupBy,
    show_extensions: bool,
    show_hidden: bool,
    icon_size: u16,
    columns: [Column; 8],
}

impl Default for ViewOptions {
    fn default() -> Self {
        use ColumnId::*;
        Self {
            mode: Mode::Details,
            sort: Sort::Name,
            descending: false,
            folders_first: true,
            group_by: GroupBy::None,
            show_extensions: true,
            show_hidden: false,
            icon_size: 40,
            columns: [
                (Name, 260, true),
                (Kind, 120, true),
                (Size, 96, true),
                (Modified, 144, true),
                (Created, 144, false),
                (Tags, 120, false),
                (Note, 96, false),
                (Indexing, 120, false),
            ]
            .map(|(id, width, visible)| Column { id, width, visible }),
        }
    }
}

impl ViewOptions {
    fn validate(&self) -> Result<(), ViewError> {
        if !(16..=160).contains(&self.icon_size)
            || !self.columns.iter().any(|column| column.visible)
        {
            return Err(ViewError::InvalidRequest);
        }
        for (index, column) in self.columns.iter().enumerate() {
            if !(48..=1200).contains(&column.width)
                || self.columns[..index]
                    .iter()
                    .any(|other| other.id == column.id)
            {
                return Err(ViewError::InvalidRequest);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ViewSettings {
    global: ViewOptions,
    folder: Option<ViewOptions>,
    folder_identity: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ViewError {
    InvalidRequest,
    Unavailable,
}

#[cfg(test)]
fn folder_key(path: Option<String>) -> Result<Option<String>, ViewError> {
    let Some(path) = path else {
        return Ok(None);
    };
    if path.len() > MAX_PATH_BYTES || filesystem::native_path(path.clone()).is_err() {
        return Err(ViewError::InvalidRequest);
    }
    Ok(Some(format!(
        "{}{path}",
        crate::storage::FOLDER_VIEW_PREFIX
    )))
}

fn read_view(database: &Connection, key: &str) -> Result<Option<ViewOptions>, StorageError> {
    let value: Option<String> = database
        .query_row("SELECT value FROM preferences WHERE key=?1", [key], |row| {
            row.get(0)
        })
        .optional()?;
    decode_view(value)
}

fn decode_view(value: Option<String>) -> Result<Option<ViewOptions>, StorageError> {
    value
        .map(|value| {
            let options: ViewOptions =
                serde_json::from_str(&value).map_err(|_| StorageError::Unavailable)?;
            options.validate().map_err(|_| StorageError::Unavailable)?;
            Ok(options)
        })
        .transpose()
}

#[cfg(test)]
fn load_views(database: &Connection, folder: Option<&str>) -> Result<ViewSettings, StorageError> {
    let global = read_view(database, DEFAULT_KEY)?.unwrap_or_default();
    let folder = match folder {
        Some(folder) => read_view(database, folder)?,
        None => None,
    };
    Ok(ViewSettings {
        global,
        folder,
        folder_identity: None,
    })
}

fn write_view(
    database: &Connection,
    key: &str,
    view: Option<&ViewOptions>,
) -> Result<(), StorageError> {
    if let Some(view) = view {
        let value = serde_json::to_string(view).map_err(|_| StorageError::Unavailable)?;
        database.execute(
            "INSERT INTO preferences (key,value) VALUES (?1,?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
    } else {
        database.execute("DELETE FROM preferences WHERE key=?1", [key])?;
    }
    Ok(())
}

async fn with_store<T: Send + 'static>(
    app: AppHandle,
    work: impl FnOnce(&mut Store) -> Result<T, StorageError> + Send + 'static,
) -> Result<T, ViewError> {
    tauri::async_runtime::spawn_blocking(move || app.state::<Storage>().access(work))
        .await
        .map_err(|_| ViewError::Unavailable)?
        .map_err(|_| ViewError::Unavailable)
}

fn folder_locator(path: &str) -> Result<Locator, ViewError> {
    if path.len() > MAX_PATH_BYTES {
        return Err(ViewError::InvalidRequest);
    }
    filesystem::native_path(path.to_owned())
        .ok()
        .and_then(Locator::new)
        .ok_or(ViewError::InvalidRequest)
}

fn registered_folder(
    store: &mut Store,
    references: &OrganizationReferences,
    locator: &Locator,
    create: bool,
) -> Result<Option<String>, StorageError> {
    if !locator.path().is_dir() {
        return Ok(None);
    }
    let found = match store.resolve_live_item(locator, create) {
        Err(StorageError::ReferenceConflict) => {
            references.reconcile_now(store);
            store.resolve_live_item(locator, create)
        }
        result => result,
    }?;
    let Some(id) = found else { return Ok(None) };
    let directory: bool = store.connection.query_row(
        "SELECT kind='directory' FROM items WHERE id=?1",
        [&id],
        |row| row.get(0),
    )?;
    Ok(directory.then_some(id))
}

fn write_folder_view(
    store: &mut Store,
    id: &str,
    view: Option<&ViewOptions>,
) -> Result<(), StorageError> {
    if let Some(view) = view {
        let value = serde_json::to_string(view).map_err(|_| StorageError::Unavailable)?;
        store.connection.execute(
            "INSERT INTO folder_views (item_id,value) VALUES (?1,?2)
             ON CONFLICT(item_id) DO UPDATE SET value=excluded.value",
            params![id, value],
        )?;
    } else {
        store
            .connection
            .execute("DELETE FROM folder_views WHERE item_id=?1", [id])?;
    }
    Ok(())
}

fn load_folder_views(
    store: &mut Store,
    references: &OrganizationReferences,
    locator: Option<&Locator>,
) -> Result<ViewSettings, StorageError> {
    let global = read_view(&store.connection, DEFAULT_KEY)?.unwrap_or_default();
    let mut result = ViewSettings {
        global,
        folder: None,
        folder_identity: None,
    };
    let Some(locator) = locator else {
        return Ok(result);
    };
    let id = match registered_folder(store, references, locator, true) {
        Ok(id) => id,
        Err(StorageError::ReferenceConflict | StorageError::Unavailable) => None,
        Err(error) => return Err(error),
    };
    if let Some(id) = id {
        let stored = store
            .connection
            .query_row(
                "SELECT value FROM folder_views WHERE item_id=?1",
                [&id],
                |row| row.get(0),
            )
            .optional()?;
        result.folder = decode_view(stored)?;
        // Even with global defaults, the view editor needs continuity if the
        // current folder is renamed before the first local override is saved.
        references.track(store, &id, locator);
        result.folder_identity = Some(id);
    }
    Ok(result)
}

fn save_folder_view(
    store: &mut Store,
    references: &OrganizationReferences,
    locator: &Locator,
    expected_id: &str,
    view: Option<&ViewOptions>,
) -> Result<(), StorageError> {
    let id = registered_folder(store, references, locator, false)?;
    if id.as_deref() != Some(expected_id) {
        return Err(StorageError::ReferenceConflict);
    }
    write_folder_view(store, expected_id, view)
}

#[tauri::command]
pub(crate) async fn load_workspace_views(
    window: WebviewWindow,
    path: Option<String>,
) -> Result<ViewSettings, ViewError> {
    let locator = path.as_deref().map(folder_locator).transpose()?;
    let app = window.app_handle().clone();
    with_store(window.app_handle().clone(), move |store| {
        load_folder_views(
            store,
            &app.state::<OrganizationReferences>(),
            locator.as_ref(),
        )
    })
    .await
}

#[tauri::command]
pub(crate) async fn save_workspace_view(
    window: WebviewWindow,
    path: Option<String>,
    folder_identity: Option<String>,
    view: Option<ViewOptions>,
) -> Result<(), ViewError> {
    let locator = path.as_deref().map(folder_locator).transpose()?;
    if locator.is_some() != folder_identity.is_some()
        || folder_identity
            .as_deref()
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_err())
    {
        return Err(ViewError::InvalidRequest);
    }
    if let Some(view) = &view {
        view.validate()?;
    }
    let app = window.app_handle().clone();
    with_store(window.app_handle().clone(), move |store| {
        if let (Some(locator), Some(id)) = (locator, folder_identity) {
            return save_folder_view(
                store,
                &app.state::<OrganizationReferences>(),
                &locator,
                &id,
                view.as_ref(),
            );
        }
        write_view(&store.connection, DEFAULT_KEY, view.as_ref())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preferences() -> Connection {
        let database = Connection::open_in_memory().unwrap();
        database.execute_batch(
            "CREATE TABLE preferences (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL CHECK (json_valid(value))) STRICT;"
        ).unwrap();
        database
    }

    #[test]
    fn folder_override_loads_and_resets_separately_from_global_default() {
        let database = preferences();
        let path = std::env::temp_dir().join("folder-A");
        let key = folder_key(Some(path.to_str().unwrap().to_owned()))
            .unwrap()
            .unwrap();
        let other = std::env::temp_dir().join("folder-B");
        let other_key = folder_key(Some(other.to_str().unwrap().to_owned()))
            .unwrap()
            .unwrap();

        assert_eq!(
            load_views(&database, Some(&key)).unwrap().global,
            ViewOptions::default()
        );
        let global = ViewOptions {
            mode: Mode::Columns,
            ..ViewOptions::default()
        };
        let mut folder = ViewOptions {
            mode: Mode::Grid,
            show_hidden: true,
            ..ViewOptions::default()
        };
        folder.columns.swap(1, 3);
        folder.columns[1].width = 320;
        write_view(&database, DEFAULT_KEY, Some(&global)).unwrap();
        write_view(&database, &key, Some(&folder)).unwrap();

        let settings = load_views(&database, Some(&key)).unwrap();
        assert_eq!(settings.global, global);
        assert_eq!(settings.folder, Some(folder));
        assert_eq!(
            load_views(&database, Some(&other_key)).unwrap().folder,
            None
        );
        write_view(&database, &key, None).unwrap();
        assert_eq!(load_views(&database, Some(&key)).unwrap().folder, None);
        assert_eq!(load_views(&database, None).unwrap().global, global);
        write_view(&database, DEFAULT_KEY, None).unwrap();
        assert_eq!(
            load_views(&database, None).unwrap().global,
            ViewOptions::default()
        );
    }

    #[test]
    fn rejects_invalid_paths_columns_and_sizes() {
        assert!(folder_key(Some("relative/folder".into())).is_err());
        assert!(folder_key(Some(format!("/bad\0{}", "x".repeat(MAX_PATH_BYTES)))).is_err());

        let view = ViewOptions::default();
        assert!(view.validate().is_ok());
        let mut duplicated = view.clone();
        duplicated.columns[1].id = duplicated.columns[0].id;
        assert!(duplicated.validate().is_err());
        let mut hidden = view.clone();
        for column in &mut hidden.columns {
            column.visible = false;
        }
        assert!(hidden.validate().is_err());
        assert!(
            ViewOptions {
                icon_size: 161,
                ..view
            }
            .validate()
            .is_err()
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn folder_view_follows_verified_rename_and_stale_save_rejects_replacement() {
        use std::fs;

        let root = std::env::temp_dir().join(format!("folderai-view-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(root.clone());
        let mut store = Store::open(&root, &root).unwrap();
        let references = OrganizationReferences::default();
        let source = root.join("documenti");
        let dest = root.join("rinominati");
        fs::create_dir(&source).unwrap();
        let source_locator = Locator::new(source.clone()).unwrap();
        let dest_locator = Locator::new(dest.clone()).unwrap();
        let legacy = folder_key(Some(dest.to_str().unwrap().to_owned()))
            .unwrap()
            .unwrap();
        let legacy_view = ViewOptions {
            mode: Mode::Columns,
            ..ViewOptions::default()
        };
        write_view(&store.connection, &legacy, Some(&legacy_view)).unwrap();
        let initial = load_folder_views(&mut store, &references, Some(&source_locator)).unwrap();
        let original = initial.folder_identity.unwrap();
        let folder = ViewOptions {
            mode: Mode::Grid,
            ..ViewOptions::default()
        };
        save_folder_view(
            &mut store,
            &references,
            &source_locator,
            &original,
            Some(&folder),
        )
        .unwrap();

        fs::rename(&source, &dest).unwrap();
        assert_eq!(
            store.resolve_live_item(&dest_locator, true),
            Err(StorageError::ReferenceConflict)
        );
        let moved = load_folder_views(&mut store, &references, Some(&dest_locator)).unwrap();
        assert_eq!(moved.folder_identity.as_deref(), Some(original.as_str()));
        assert_eq!(moved.folder, Some(folder));
        assert_eq!(
            read_view(&store.connection, &legacy).unwrap(),
            Some(legacy_view)
        );

        fs::create_dir(&source).unwrap();
        assert_eq!(
            save_folder_view(&mut store, &references, &source_locator, &original, None),
            Err(StorageError::ReferenceConflict),
        );
        let replacement =
            load_folder_views(&mut store, &references, Some(&source_locator)).unwrap();
        assert_ne!(
            replacement.folder_identity.as_deref(),
            Some(original.as_str())
        );
        assert!(replacement.folder.is_none());
    }
}
