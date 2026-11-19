use crate::{
    filesystem,
    local_identity::{LiveIdentity, LiveObservation, Locator, canonical_path},
    storage::{COPY_METADATA_KEY, Storage, StorageError, Store, TrackedReference, copy_metadata},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Mutex,
};
use tauri::{AppHandle, Manager, WebviewWindow};
use uuid::Uuid;

const MAX_PATHS: usize = 512;
const MAX_TAG_ASSIGNMENTS: usize = 50_000;
const MAX_PATH_BYTES: usize = 128 * 1024;
const MAX_BATCH_PATH_BYTES: usize = 4 * 1024 * 1024;
const MAX_NOTE_BYTES: usize = 256 * 1024;
const MAX_REVISION: i64 = 9_007_199_254_740_991;
const MAX_RETAINED_REFERENCES: usize = 128;
const FAVORITE_DEFAULTS_KEY: &str = "organization.default-favorites.v1";

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum OrganizationError {
    InvalidRequest,
    RevisionConflict,
    Unavailable,
    NotFound,
}

impl From<rusqlite::Error> for OrganizationError {
    fn from(_: rusqlite::Error) -> Self {
        Self::Unavailable
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Organization {
    items: Vec<Item>,
    groups: Vec<Group>,
    tags: Vec<Tag>,
    favorites: Vec<Favorite>,
    copy_metadata: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Item {
    id: String,
    path: String,
    note: Option<Note>,
    tag_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Note {
    body: String,
    revision: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PathMetadata {
    path: String,
    tag_ids: Vec<String>,
    has_note: bool,
}

#[derive(Debug, Serialize)]
struct Group {
    id: String,
    name: String,
}

#[derive(Debug, Serialize)]
struct Tag {
    id: String,
    name: String,
    color: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Favorite {
    id: String,
    item_id: String,
    group_id: Option<String>,
    path: Option<String>,
    name: Option<String>,
    is_directory: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_location: Option<DefaultLocation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum DefaultLocation {
    Desktop,
    Downloads,
    Documents,
    Pictures,
}

impl DefaultLocation {
    fn favorite_id(self) -> &'static str {
        match self {
            Self::Desktop => "default-desktop",
            Self::Downloads => "default-downloads",
            Self::Documents => "default-documents",
            Self::Pictures => "default-pictures",
        }
    }

    fn for_favorite(id: &str) -> Option<Self> {
        [
            Self::Desktop,
            Self::Downloads,
            Self::Documents,
            Self::Pictures,
        ]
        .into_iter()
        .find(|location| location.favorite_id() == id)
    }
}

fn add_default_favorites(
    store: &mut Store,
    locations: impl FnOnce() -> [(DefaultLocation, Option<PathBuf>); 4],
) -> Result<(), OrganizationError> {
    let initialized: bool = store.connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM preferences WHERE key=?1)",
        [FAVORITE_DEFAULTS_KEY],
        |row| row.get(0),
    )?;
    if initialized {
        return Ok(());
    }

    let mut items = Vec::with_capacity(4);
    for (location, path) in locations() {
        let Some(locator) = path
            .and_then(|path| canonical_path(&path).ok())
            .and_then(Locator::new)
        else {
            continue;
        };
        let Ok(identity) = LiveIdentity::open(&locator) else {
            continue;
        };
        if !matches!(identity.kind(), Ok("directory")) {
            continue;
        }
        match store.resolve_live_item(&locator, true) {
            Ok(Some(id)) => items.push((location.favorite_id(), id)),
            Ok(None) | Err(StorageError::ReferenceConflict) => {}
            Err(_) => return Err(OrganizationError::Unavailable),
        }
    }

    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let initialized_here = transaction.execute(
        "INSERT INTO preferences (key, value) VALUES (?1, 'true') ON CONFLICT(key) DO NOTHING",
        [FAVORITE_DEFAULTS_KEY],
    )? == 1;
    if initialized_here {
        for (favorite, item) in items {
            transaction.execute(
                "INSERT INTO favorites (id, item_id, group_id, position)
                 SELECT ?1, ?2, NULL, (
                     SELECT COALESCE(MAX(position)+1, 0) FROM favorites WHERE group_id IS NULL
                 )
                 FROM items WHERE id=?2 AND kind='directory' AND availability='available'
                   AND NOT EXISTS (SELECT 1 FROM favorites WHERE item_id=?2)",
                params![favorite, item],
            )?;
        }
    }
    transaction.commit()?;
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum Change {
    SaveNote {
        item_id: String,
        expected_revision: i64,
        body: String,
    },
    CreateGroup {
        name: String,
    },
    RenameGroup {
        id: String,
        name: String,
    },
    RemoveGroup {
        id: String,
    },
    AddFavorite {
        item_id: String,
        group_id: Option<String>,
    },
    RemoveFavorite {
        id: String,
    },
    MoveFavorite {
        id: String,
        group_id: Option<String>,
        index: u32,
    },
    UpsertTag {
        id: Option<String>,
        name: String,
        color: Option<String>,
    },
    RemoveTag {
        id: String,
    },
    AssignTag {
        item_ids: Vec<String>,
        tag_id: String,
        assigned: bool,
    },
    SetCopyMetadata {
        enabled: bool,
    },
}

impl Change {
    fn validate(&mut self) -> Result<(), OrganizationError> {
        fn id(value: &str) -> bool {
            !value.is_empty()
                && value.len() <= 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        }

        fn name(value: &mut String) -> bool {
            if value.len() > 512 {
                return false;
            }
            *value = value.trim().to_owned();
            !value.is_empty()
                && value.chars().count() <= 128
                && !value.chars().any(char::is_control)
        }

        fn color(value: &str) -> bool {
            value.len() == 7
                && value.starts_with('#')
                && value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
        }

        let valid = match self {
            Self::SaveNote {
                item_id,
                expected_revision,
                body,
            } => {
                id(item_id)
                    && (0..MAX_REVISION).contains(expected_revision)
                    && body.len() <= MAX_NOTE_BYTES
            }
            Self::CreateGroup { name: value } => name(value),
            Self::RenameGroup {
                id: value,
                name: label,
            } => id(value) && name(label),
            Self::RemoveGroup { id: value }
            | Self::RemoveFavorite { id: value }
            | Self::RemoveTag { id: value } => id(value),
            Self::AddFavorite { item_id, group_id }
            | Self::MoveFavorite {
                id: item_id,
                group_id,
                ..
            } => id(item_id) && group_id.as_deref().is_none_or(id),
            Self::UpsertTag {
                id: value,
                name: label,
                color: tint,
            } => {
                value.as_deref().is_none_or(id) && name(label) && tint.as_deref().is_none_or(color)
            }
            Self::AssignTag {
                item_ids, tag_id, ..
            } => {
                !item_ids.is_empty()
                    && item_ids.len() <= MAX_TAG_ASSIGNMENTS
                    && item_ids.iter().all(|item| id(item))
                    && id(tag_id)
            }
            Self::SetCopyMetadata { .. } => true,
        };
        valid.then_some(()).ok_or(OrganizationError::InvalidRequest)
    }
}

#[derive(Default)]
struct References {
    by_id: HashMap<String, Retained>,
    sequence: u64,
}

struct Retained {
    reference: TrackedReference,
    last_seen: u64,
    favorite: bool,
}

impl References {
    fn retain(&mut self, store: &mut Store, id: &str, locator: &Locator, favorite: bool) {
        self.sequence += 1;
        if let Some(retained) = self.by_id.get_mut(id) {
            retained.last_seen = self.sequence;
            retained.favorite |= favorite;
            return;
        }

        let eviction = if self.by_id.len() == MAX_RETAINED_REFERENCES {
            let oldest = self
                .by_id
                .iter()
                .filter(|(_, retained)| !retained.favorite)
                .min_by_key(|(_, retained)| retained.last_seen);
            let Some((id, _)) = oldest else {
                return;
            };
            Some(id.clone())
        } else {
            None
        };

        if let Ok(reference) = store.attach_live_reference(id, locator) {
            if let Some(id) = eviction {
                self.by_id.remove(&id);
            }
            self.by_id.insert(
                id.to_owned(),
                Retained {
                    reference,
                    last_seen: self.sequence,
                    favorite,
                },
            );
        }
    }

    fn reconcile(&mut self, store: &mut Store) {
        self.by_id.retain(|_, retained| {
            !matches!(
                store.reconcile_live_reference(&mut retained.reference),
                Ok(LiveObservation::Removed)
            )
        });
    }
}

#[derive(Default)]
pub(crate) struct OrganizationReferences(Mutex<References>);

impl OrganizationReferences {
    pub(crate) fn reconcile_now(&self, store: &mut Store) {
        if let Ok(mut references) = self.0.lock() {
            references.reconcile(store);
        }
    }

    pub(crate) fn track(&self, store: &mut Store, id: &str, locator: &Locator) {
        if let Ok(mut references) = self.0.lock() {
            references.retain(store, id, locator, false);
        }
    }
}

fn validate_paths(paths: Vec<String>) -> Result<Vec<(String, Locator)>, OrganizationError> {
    if paths.len() > MAX_PATHS
        || paths.iter().any(|path| path.len() > MAX_PATH_BYTES)
        || paths.iter().map(String::len).sum::<usize>() > MAX_BATCH_PATH_BYTES
    {
        return Err(OrganizationError::InvalidRequest);
    }
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .filter(|path| seen.insert(path.clone()))
        .map(|path| {
            let locator = filesystem::native_path(path.clone())
                .ok()
                .and_then(Locator::new)
                .ok_or(OrganizationError::InvalidRequest)?;
            Ok((path, locator))
        })
        .collect()
}

fn get_note(database: &Connection, id: &str) -> Result<Option<Note>, OrganizationError> {
    Ok(database
        .prepare_cached("SELECT body, revision FROM notes WHERE item_id=?1")?
        .query_row([id], |row| {
            Ok(Note {
                body: row.get(0)?,
                revision: row.get(1)?,
            })
        })
        .optional()?)
}

fn get_tag_ids(database: &Connection, id: &str) -> Result<Vec<String>, OrganizationError> {
    Ok(database
        .prepare_cached("SELECT tag_id FROM item_tags WHERE item_id=?1 ORDER BY tag_id")?
        .query_map([id], |row| row.get(0))?
        .collect::<Result<_, _>>()?)
}

fn lookup_path_metadata(
    store: &mut Store,
    paths: Vec<(String, Locator)>,
) -> Result<Vec<PathMetadata>, OrganizationError> {
    let has_metadata: bool = store.connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM item_tags) OR EXISTS(SELECT 1 FROM notes WHERE body <> '')",
        [],
        |row| row.get(0),
    )?;
    if !has_metadata {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    for (path, locator) in paths {
        if let Ok(Some(id)) = store.resolve_live_item(&locator, false) {
            let tag_ids = get_tag_ids(&store.connection, &id)?;
            let has_note: bool = store
                .connection
                .prepare_cached(
                    "SELECT EXISTS(SELECT 1 FROM notes WHERE item_id=?1 AND body <> '')",
                )?
                .query_row([&id], |row| row.get(0))?;
            if has_note || !tag_ids.is_empty() {
                result.push(PathMetadata {
                    path,
                    tag_ids,
                    has_note,
                });
            }
        }
    }
    Ok(result)
}

fn get_groups(database: &Connection) -> Result<Vec<Group>, OrganizationError> {
    Ok(database
        .prepare("SELECT id, name FROM favorite_groups ORDER BY position, id")?
        .query_map([], |row| {
            Ok(Group {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?
        .collect::<Result<_, _>>()?)
}

fn get_tags(database: &Connection) -> Result<Vec<Tag>, OrganizationError> {
    Ok(database
        .prepare("SELECT id, name, color FROM tags ORDER BY name COLLATE NOCASE, name")?
        .query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?)
}

fn get_favorites(
    store: &mut Store,
    references: &mut References,
) -> Result<Vec<Favorite>, OrganizationError> {
    let rows = store
        .connection
        .prepare(
            "SELECT f.id, f.item_id, f.group_id, i.locator, i.kind = 'directory'
             FROM favorites f JOIN items i ON i.id=f.item_id
             LEFT JOIN favorite_groups g ON g.id=f.group_id
             WHERE i.availability != 'missing'
             ORDER BY f.group_id IS NOT NULL, g.position, f.group_id, f.position, f.id",
        )?
        .query_map([], |row| {
            let id: String = row.get(0)?;
            Ok((
                Favorite {
                    default_location: DefaultLocation::for_favorite(&id),
                    id,
                    item_id: row.get(1)?,
                    group_id: row.get(2)?,
                    path: None,
                    name: None,
                    is_directory: row.get(4)?,
                },
                row.get::<_, Vec<u8>>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let favorite_ids = rows
        .iter()
        .map(|(favorite, _)| favorite.item_id.as_str())
        .collect::<HashSet<_>>();
    for (id, retained) in &mut references.by_id {
        retained.favorite = favorite_ids.contains(id.as_str());
    }
    let mut validated = HashMap::new();
    let favorites = rows
        .into_iter()
        .map(|(mut favorite, bytes)| {
            let Some(locator) = Locator::decode(bytes) else {
                return favorite;
            };
            let path = locator.path();
            favorite.name = path
                .file_name()
                .unwrap_or(path.as_os_str())
                .to_str()
                .map(str::to_owned);
            let current = validated
                .entry(favorite.item_id.clone())
                .or_insert_with(|| {
                    let found = matches!(
                        store.resolve_live_item(&locator, false),
                        Ok(Some(id)) if id == favorite.item_id
                    );
                    if found {
                        references.retain(store, &favorite.item_id, &locator, true);
                    }
                    found
                });
            if *current {
                favorite.path = path.to_str().map(str::to_owned);
            }
            favorite
        })
        .collect();
    Ok(favorites)
}

fn load(
    store: &mut Store,
    references: &mut References,
    paths: Vec<(String, Locator)>,
    inspect: bool,
    include_notes: bool,
) -> Result<Organization, OrganizationError> {
    references.reconcile(store);
    let favorites = get_favorites(store, references)?;
    let mut items = Vec::new();
    for (path, locator) in paths {
        // An unavailable, replaced, or identity-unsupported entry has no
        // usable item ID for this path.
        let Ok(Some(id)) = store.resolve_live_item(&locator, inspect) else {
            continue;
        };
        if inspect {
            references.retain(store, &id, &locator, false);
        }
        let note = if include_notes {
            get_note(&store.connection, &id)?
        } else {
            None
        };
        let tag_ids = get_tag_ids(&store.connection, &id)?;
        items.push(Item {
            id,
            path,
            note,
            tag_ids,
        });
    }
    Ok(Organization {
        items,
        groups: get_groups(&store.connection)?,
        tags: get_tags(&store.connection)?,
        favorites,
        copy_metadata: copy_metadata(&store.connection)
            .map_err(|_| OrganizationError::Unavailable)?,
    })
}

fn require_exists(exists: bool) -> Result<(), OrganizationError> {
    exists.then_some(()).ok_or(OrganizationError::NotFound)
}

fn require_item(database: &Connection, item: &str) -> Result<(), OrganizationError> {
    require_exists(database.query_row(
        "SELECT EXISTS(SELECT 1 FROM items WHERE id=?1)",
        [item],
        |row| row.get(0),
    )?)
}

fn require_group(database: &Connection, group: Option<&str>) -> Result<(), OrganizationError> {
    if let Some(group) = group {
        require_exists(database.query_row(
            "SELECT EXISTS(SELECT 1 FROM favorite_groups WHERE id=?1)",
            [group],
            |row| row.get(0),
        )?)?;
    }
    Ok(())
}

fn favorite_ids(
    database: &Connection,
    group: Option<&str>,
) -> Result<Vec<String>, OrganizationError> {
    Ok(database
        .prepare("SELECT id FROM favorites WHERE group_id IS ?1 ORDER BY position, id")?
        .query_map([group], |row| row.get(0))?
        .collect::<Result<_, _>>()?)
}

fn order_favorites(
    database: &Connection,
    group: Option<&str>,
    ids: Vec<String>,
) -> Result<(), OrganizationError> {
    let mut update =
        database.prepare_cached("UPDATE favorites SET group_id=?1, position=?2 WHERE id=?3")?;
    for (position, id) in ids.into_iter().enumerate() {
        update.execute(params![group, position as i64, id])?;
    }
    Ok(())
}

fn update(database: &mut Connection, mut change: Change) -> Result<Option<i64>, OrganizationError> {
    change.validate()?;
    let transaction = database.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let mut revision = None;
    match change {
        Change::SaveNote {
            item_id,
            expected_revision,
            body,
        } => {
            require_item(&transaction, &item_id)?;
            let changed = if expected_revision == 0 {
                transaction.execute(
                    "INSERT INTO notes (item_id, body, revision) VALUES (?1,?2,1)
                     ON CONFLICT(item_id) DO NOTHING",
                    params![item_id, body],
                )?
            } else {
                transaction.execute(
                    "UPDATE notes SET body=?2, revision=revision+1 WHERE item_id=?1 AND revision=?3",
                    params![item_id, body, expected_revision],
                )?
            };
            if changed == 0 {
                return Err(OrganizationError::RevisionConflict);
            }
            revision = Some(expected_revision + 1);
        }
        Change::CreateGroup { name } => {
            transaction.execute(
                "INSERT INTO favorite_groups (id, name, position)
                 SELECT ?1, ?2, COALESCE(MAX(position)+1,0) FROM favorite_groups",
                params![Uuid::new_v4().to_string(), name],
            )?;
        }
        Change::RenameGroup { id, name } => {
            require_exists(
                transaction.execute(
                    "UPDATE favorite_groups SET name=?2 WHERE id=?1",
                    params![id, name],
                )? == 1,
            )?;
        }
        Change::RemoveGroup { id } => {
            require_group(&transaction, Some(&id))?;
            let mut links = favorite_ids(&transaction, None)?;
            links.extend(favorite_ids(&transaction, Some(&id))?);
            order_favorites(&transaction, None, links)?;
            transaction.execute("DELETE FROM favorite_groups WHERE id=?1", [id])?;
        }
        Change::AddFavorite { item_id, group_id } => {
            require_exists(transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM items WHERE id=?1 AND availability != 'missing')",
                [&item_id],
                |row| row.get(0),
            )?)?;
            require_group(&transaction, group_id.as_deref())?;
            transaction.execute(
                "INSERT INTO favorites (id, item_id, group_id, position)
                 SELECT ?1, ?2, ?3, (
                    SELECT COALESCE(MAX(position)+1,0) FROM favorites WHERE group_id IS ?3
                 )
                 WHERE NOT EXISTS (SELECT 1 FROM favorites WHERE item_id=?2 AND group_id IS ?3)",
                params![Uuid::new_v4().to_string(), item_id, group_id],
            )?;
        }
        Change::RemoveFavorite { id } => {
            require_exists(transaction.execute("DELETE FROM favorites WHERE id=?1", [id])? == 1)?;
        }
        Change::MoveFavorite {
            id,
            group_id,
            index,
        } => {
            require_group(&transaction, group_id.as_deref())?;
            let previous_group: Option<String> = transaction
                .query_row("SELECT group_id FROM favorites WHERE id=?1", [&id], |row| {
                    row.get(0)
                })
                .optional()?
                .ok_or(OrganizationError::NotFound)?;
            let mut destination = favorite_ids(&transaction, group_id.as_deref())?;
            destination.retain(|link| link != &id);
            destination.insert((index as usize).min(destination.len()), id);
            order_favorites(&transaction, group_id.as_deref(), destination)?;
            if previous_group != group_id {
                let previous_links = favorite_ids(&transaction, previous_group.as_deref())?;
                order_favorites(&transaction, previous_group.as_deref(), previous_links)?;
            }
        }
        Change::UpsertTag { id, name, color } => {
            let duplicate: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM tags WHERE name=?1 AND id IS NOT ?2)",
                params![name, id],
                |row| row.get(0),
            )?;
            if duplicate {
                return Err(OrganizationError::InvalidRequest);
            }
            if let Some(id) = id {
                require_exists(
                    transaction.execute(
                        "UPDATE tags SET name=?2, color=?3 WHERE id=?1",
                        params![id, name, color],
                    )? == 1,
                )?;
            } else {
                transaction.execute(
                    "INSERT INTO tags (id, name, color) VALUES (?1,?2,?3)",
                    params![Uuid::new_v4().to_string(), name, color],
                )?;
            }
        }
        Change::RemoveTag { id } => {
            require_exists(transaction.execute("DELETE FROM tags WHERE id=?1", [id])? == 1)?;
        }
        Change::AssignTag {
            item_ids,
            tag_id,
            assigned,
        } => {
            require_exists(transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM tags WHERE id=?1)",
                [&tag_id],
                |row| row.get(0),
            )?)?;
            let sql = if assigned {
                "INSERT INTO item_tags (item_id,tag_id) VALUES (?1,?2)
                 ON CONFLICT(item_id,tag_id) DO NOTHING"
            } else {
                "DELETE FROM item_tags WHERE item_id=?1 AND tag_id=?2"
            };
            let mut statement = transaction.prepare_cached(sql)?;
            let mut check_item =
                transaction.prepare_cached("SELECT EXISTS(SELECT 1 FROM items WHERE id=?1)")?;
            for item in item_ids {
                require_exists(check_item.query_row([&item], |row| row.get(0))?)?;
                statement.execute(params![item, tag_id])?;
            }
        }
        Change::SetCopyMetadata { enabled } => {
            transaction.execute(
                "INSERT INTO preferences (key, value) VALUES (?1,?2)
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                params![COPY_METADATA_KEY, if enabled { "true" } else { "false" }],
            )?;
        }
    }
    transaction.commit()?;
    Ok(revision)
}

async fn with_store<T: Send + 'static>(
    app: AppHandle,
    work: impl FnOnce(&mut Store, &AppHandle) -> Result<T, OrganizationError> + Send + 'static,
) -> Result<T, OrganizationError> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Storage>().access(|store| Ok(work(store, &app)))
    })
    .await
    .map_err(|_| OrganizationError::Unavailable)?
    .map_err(|_| OrganizationError::Unavailable)?
}

#[tauri::command]
pub(crate) async fn load_organization(
    window: WebviewWindow,
    paths: Vec<String>,
    inspect: Option<bool>,
    include_notes: Option<bool>,
) -> Result<Organization, OrganizationError> {
    let paths = validate_paths(paths)?;
    with_store(window.app_handle().clone(), move |store, app| {
        add_default_favorites(store, || {
            let path = app.path();
            [
                (DefaultLocation::Desktop, path.desktop_dir().ok()),
                (DefaultLocation::Downloads, path.download_dir().ok()),
                (DefaultLocation::Documents, path.document_dir().ok()),
                (DefaultLocation::Pictures, path.picture_dir().ok()),
            ]
        })?;
        let state = app.state::<OrganizationReferences>();
        let mut references = state.0.lock().map_err(|_| OrganizationError::Unavailable)?;
        load(
            store,
            &mut references,
            paths,
            inspect.unwrap_or(false),
            include_notes.unwrap_or(true),
        )
    })
    .await
}

#[tauri::command]
pub(crate) async fn lookup_organization_tags(
    window: WebviewWindow,
    paths: Vec<String>,
) -> Result<Vec<PathMetadata>, OrganizationError> {
    let paths = validate_paths(paths)?;
    with_store(window.app_handle().clone(), move |store, _| {
        lookup_path_metadata(store, paths)
    })
    .await
}

#[tauri::command]
pub(crate) async fn update_organization(
    window: WebviewWindow,
    change: Change,
) -> Result<Option<i64>, OrganizationError> {
    with_store(window.app_handle().clone(), move |store, _| {
        update(&mut store.connection, change)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};

    fn database() -> Connection {
        let database = Connection::open_in_memory().unwrap();
        database
            .execute_batch(concat!(
                "PRAGMA foreign_keys=ON; ",
                include_str!("storage/schema/001.sql"),
                include_str!("storage/schema/002.sql"),
                include_str!("storage/schema/003.sql"),
            ))
            .unwrap();
        database
    }

    fn add_item(database: &Connection, id: &str) {
        database
            .execute(
                "INSERT INTO items (id, locator, kind, availability) VALUES (?1, x'00', 'file', 'available')",
                [id],
            )
            .unwrap();
    }

    fn save_note(item: &str, expected_revision: i64, body: &str) -> Change {
        Change::SaveNote {
            item_id: item.into(),
            expected_revision,
            body: body.into(),
        }
    }

    #[test]
    fn note_writes_compare_revisions_for_the_captured_item() {
        let mut database = database();
        add_item(&database, "original");
        add_item(&database, "next-selection");

        assert_eq!(
            update(&mut database, save_note("original", 0, "first")),
            Ok(Some(1))
        );
        assert_eq!(
            update(&mut database, save_note("next-selection", 0, "independent")),
            Ok(Some(1))
        );
        assert_eq!(
            update(&mut database, save_note("original", 0, "stale create")),
            Err(OrganizationError::RevisionConflict)
        );
        assert_eq!(
            update(&mut database, save_note("original", 1, "# Committed\n")),
            Ok(Some(2))
        );
        assert_eq!(
            update(&mut database, save_note("original", 1, "stale update")),
            Err(OrganizationError::RevisionConflict)
        );
        assert_eq!(
            update(&mut database, save_note("missing", 0, "orphan")),
            Err(OrganizationError::NotFound)
        );
        assert_eq!(
            update(
                &mut database,
                save_note("original", 2, &"à".repeat(MAX_NOTE_BYTES / 2 + 1))
            ),
            Err(OrganizationError::InvalidRequest)
        );
        let original = get_note(&database, "original").unwrap().unwrap();
        assert_eq!(
            (original.body.as_str(), original.revision),
            ("# Committed\n", 2)
        );
        assert_eq!(
            get_note(&database, "next-selection").unwrap().unwrap().body,
            "independent"
        );
    }

    #[test]
    fn removing_a_group_preserves_duplicate_links_and_appends_in_order() {
        let mut database = database();
        add_item(&database, "original");
        add_item(&database, "another");
        database
            .execute_batch(
                "INSERT INTO favorite_groups VALUES ('group', 'Group', 0);
             INSERT INTO favorites (id, item_id, group_id, position) VALUES
               ('root', 'original', NULL, 4), ('grouped', 'original', 'group', 1),
               ('other', 'another', 'group', 8);",
            )
            .unwrap();

        assert_eq!(
            update(&mut database, Change::RemoveGroup { id: "group".into() }),
            Ok(None)
        );
        assert!(get_groups(&database).unwrap().is_empty());
        assert_eq!(
            favorite_ids(&database, None).unwrap(),
            ["root", "grouped", "other"]
        );
        assert_eq!(
            update(
                &mut database,
                Change::MoveFavorite {
                    id: "other".into(),
                    group_id: None,
                    index: 0
                }
            ),
            Ok(None)
        );
        assert_eq!(
            favorite_ids(&database, None).unwrap(),
            ["other", "root", "grouped"]
        );
        assert_eq!(
            update(&mut database, Change::RemoveFavorite { id: "root".into() }),
            Ok(None)
        );
        assert_eq!(favorite_ids(&database, None).unwrap(), ["other", "grouped"]);
    }

    #[test]
    fn tag_assignment_is_atomic_and_duplicate_names_are_rejected() {
        let mut database = database();
        add_item(&database, "first");
        add_item(&database, "second");
        assert_eq!(
            update(
                &mut database,
                Change::UpsertTag {
                    id: None,
                    name: " Progetto ".into(),
                    color: Some("#0011aa".into())
                }
            ),
            Ok(None)
        );
        assert_eq!(
            update(
                &mut database,
                Change::UpsertTag {
                    id: None,
                    name: "Progetto".into(),
                    color: None,
                }
            ),
            Err(OrganizationError::InvalidRequest)
        );
        let tag = get_tags(&database).unwrap().remove(0);
        assert_eq!(tag.name, "Progetto");
        let change = |items: &[&str], assigned| Change::AssignTag {
            item_ids: items.iter().map(|id| (*id).to_owned()).collect(),
            tag_id: tag.id.clone(),
            assigned,
        };
        assert_eq!(
            update(&mut database, change(&["first", "missing"], true)),
            Err(OrganizationError::NotFound)
        );
        assert!(get_tag_ids(&database, "first").unwrap().is_empty());
        assert_eq!(
            update(&mut database, change(&["first", "second"], true)),
            Ok(None)
        );
        assert_eq!(update(&mut database, change(&["first"], false)), Ok(None));
        assert!(get_tag_ids(&database, "first").unwrap().is_empty());
        assert_eq!(
            get_tag_ids(&database, "second").unwrap(),
            std::slice::from_ref(&tag.id)
        );
        assert_eq!(
            update(&mut database, Change::RemoveTag { id: tag.id }),
            Ok(None)
        );
        assert!(get_tag_ids(&database, "second").unwrap().is_empty());
    }

    #[test]
    fn bulk_tag_assignment_accepts_more_than_one_inspection_batch() {
        let mut database = database();
        database
            .execute("INSERT INTO tags VALUES ('tag', 'Test', NULL)", [])
            .unwrap();
        let ids = (0..MAX_PATHS + 25)
            .map(|index| format!("item-{index}"))
            .collect::<Vec<_>>();
        for id in &ids {
            add_item(&database, id);
        }
        assert_eq!(
            update(
                &mut database,
                Change::AssignTag {
                    item_ids: ids.clone(),
                    tag_id: "tag".into(),
                    assigned: true,
                },
            ),
            Ok(None),
        );
        let assigned: u32 = database
            .query_row("SELECT count(*) FROM item_tags", [], |row| row.get(0))
            .unwrap();
        assert_eq!(assigned as usize, ids.len());
        assert_eq!(
            update(
                &mut database,
                Change::AssignTag {
                    item_ids: vec!["item-0".into(); MAX_TAG_ASSIGNMENTS + 1],
                    tag_id: "tag".into(),
                    assigned: false,
                },
            ),
            Err(OrganizationError::InvalidRequest),
        );
    }

    #[test]
    fn the_ipc_contract_is_typed_and_strict() {
        let change: Change = serde_json::from_value(serde_json::json!({
            "kind": "save-note", "itemId": "file-1", "expectedRevision": 0, "body": "note"
        }))
        .unwrap();
        assert!(matches!(
            change,
            Change::SaveNote {
                expected_revision: 0,
                ..
            }
        ));
        let extra =
            serde_json::json!({ "kind": "remove-tag", "id": "a", "path": "/tmp/unexpected" });
        assert!(serde_json::from_value::<Change>(extra).is_err());
        assert!(validate_paths(vec!["relative".to_owned()]).is_err());
        assert!(validate_paths(vec![String::new(); MAX_PATHS + 1]).is_err());
    }

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let directory = std::env::temp_dir().join(format!("folderai-org-{}", Uuid::new_v4()));
            fs::create_dir(&directory).unwrap();
            Self(directory)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn defaults_are_ordered_and_removals_survive_reopening() {
        use DefaultLocation::*;

        let fixture = Fixture::new();
        let locations = |root: &std::path::Path| {
            [
                (Desktop, Some(root.join("Desktop"))),
                (Downloads, Some(root.join("Downloads"))),
                (Documents, Some(root.join("Documents"))),
                (Pictures, Some(root.join("Pictures"))),
            ]
        };
        for (_, path) in locations(&fixture.0) {
            fs::create_dir(path.unwrap()).unwrap();
        }
        let mut store = Store::open(&fixture.0, &fixture.0).unwrap();
        add_default_favorites(&mut store, || locations(&fixture.0)).unwrap();
        let favorites = get_favorites(&mut store, &mut References::default()).unwrap();
        assert_eq!(
            favorites
                .iter()
                .map(|favorite| favorite.default_location)
                .collect::<Vec<_>>(),
            [
                Some(Desktop),
                Some(Downloads),
                Some(Documents),
                Some(Pictures)
            ]
        );
        assert!(
            favorites
                .iter()
                .all(|favorite| favorite.is_directory && favorite.path.is_some())
        );
        update(
            &mut store.connection,
            Change::RemoveFavorite {
                id: favorites[0].id.clone(),
            },
        )
        .unwrap();

        drop(store);
        let mut store = Store::open(&fixture.0, &fixture.0).unwrap();
        add_default_favorites(&mut store, || {
            panic!("initialized profiles must not rediscover defaults")
        })
        .unwrap();
        assert_eq!(
            favorite_ids(&store.connection, None).unwrap(),
            [
                Downloads.favorite_id(),
                Documents.favorite_id(),
                Pictures.favorite_id(),
            ]
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn defaults_keep_existing_grouped_links_and_skip_missing_locations() {
        use DefaultLocation::*;

        let fixture = Fixture::new();
        let documents = fixture.0.join("Documents");
        let desktop = fixture.0.join("Desktop");
        fs::create_dir(&documents).unwrap();
        fs::create_dir(&desktop).unwrap();
        let mut store = Store::open(&fixture.0, &fixture.0).unwrap();
        let document = Locator::new(documents.clone()).unwrap();
        let id = store.resolve_live_item(&document, true).unwrap().unwrap();
        store
            .connection
            .execute(
                "INSERT INTO favorite_groups VALUES ('personal', 'Personal', 0)",
                [],
            )
            .unwrap();
        store
            .connection
            .execute(
                "INSERT INTO favorites VALUES ('saved-documents', ?1, 'personal', 0)",
                [&id],
            )
            .unwrap();
        add_default_favorites(&mut store, || {
            [
                (Desktop, Some(desktop)),
                (Downloads, None),
                (Documents, Some(documents)),
                (Pictures, Some(fixture.0.join("missing"))),
            ]
        })
        .unwrap();
        assert_eq!(
            favorite_ids(&store.connection, None).unwrap(),
            [Desktop.favorite_id()]
        );
        assert_eq!(
            favorite_ids(&store.connection, Some("personal")).unwrap(),
            ["saved-documents"]
        );
    }

    #[test]
    fn organization_tracks_live_favorites_and_keeps_metadata_after_confirmed_removal() {
        let fixture = Fixture::new();
        let original = fixture.0.join("draft.md");
        let destination = fixture.0.join("renamed.md");
        fs::write(&original, "original file").unwrap();
        let paths = |path: &std::path::Path| {
            validate_paths(vec![path.to_str().unwrap().to_owned()]).unwrap()
        };
        let mut store = Store {
            connection: database(),
            derived_dir: fixture.0.clone(),
        };
        let mut references = References::default();

        let before = load(&mut store, &mut references, paths(&original), false, true).unwrap();
        assert!(before.items.is_empty());
        assert!(before.copy_metadata);
        let inspected = load(&mut store, &mut references, paths(&original), true, true).unwrap();
        let id = inspected.items[0].id.clone();
        assert_eq!(
            update(&mut store.connection, save_note(&id, 0, "personal note")),
            Ok(Some(1))
        );
        let note_only = lookup_path_metadata(&mut store, paths(&original)).unwrap();
        assert!(note_only[0].has_note);
        assert!(note_only[0].tag_ids.is_empty());
        store
            .connection
            .execute("INSERT INTO tags VALUES ('tag', 'Test', NULL)", [])
            .unwrap();
        assert_eq!(
            update(
                &mut store.connection,
                Change::AssignTag {
                    item_ids: vec![id.clone()],
                    tag_id: "tag".into(),
                    assigned: true,
                },
            ),
            Ok(None),
        );
        let tagged = lookup_path_metadata(&mut store, paths(&original)).unwrap();
        assert_eq!(tagged[0].path, original.to_str().unwrap());
        assert_eq!(tagged[0].tag_ids, ["tag"]);
        let no_note_bodies =
            load(&mut store, &mut references, paths(&original), true, false).unwrap();
        assert!(no_note_bodies.items[0].note.is_none());
        assert_eq!(no_note_bodies.items[0].tag_ids, ["tag"]);
        assert_eq!(
            update(
                &mut store.connection,
                Change::AddFavorite {
                    item_id: id.clone(),
                    group_id: None
                }
            ),
            Ok(None)
        );
        assert_eq!(
            update(
                &mut store.connection,
                Change::SetCopyMetadata { enabled: false }
            ),
            Ok(None)
        );
        let first = load(&mut store, &mut references, vec![], false, true).unwrap();
        assert!(!first.copy_metadata);
        assert_eq!(first.favorites[0].name.as_deref(), Some("draft.md"));
        assert!(first.favorites[0].path.is_some());
        assert!(!first.favorites[0].is_directory);

        fs::rename(&original, &destination).unwrap();
        fs::write(&original, "replacement file").unwrap();
        let replaced = load(&mut store, &mut references, paths(&original), true, true).unwrap();
        assert_ne!(replaced.items[0].id, id);
        assert!(replaced.items[0].note.is_none());
        assert_eq!(replaced.favorites[0].name.as_deref(), Some("renamed.md"));
        assert_eq!(replaced.favorites[0].id, first.favorites[0].id);
        assert_eq!(replaced.favorites[0].item_id, id);
        assert!(
            lookup_path_metadata(&mut store, paths(&original))
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            lookup_path_metadata(&mut store, paths(&destination)).unwrap()[0].tag_ids,
            ["tag"]
        );

        fs::remove_file(&destination).unwrap();
        let removed = load(&mut store, &mut references, paths(&destination), true, true).unwrap();
        assert!(removed.items.is_empty());
        assert!(removed.favorites.is_empty());
        assert!(favorite_ids(&store.connection, None).unwrap().is_empty());
        assert_eq!(
            get_note(&store.connection, &id).unwrap().unwrap().body,
            "personal note"
        );
        assert_eq!(get_tag_ids(&store.connection, &id).unwrap(), ["tag"]);
        assert_eq!(fs::read_to_string(&original).unwrap(), "replacement file");
    }
}
