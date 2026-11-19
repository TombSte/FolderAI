use rusqlite::{
    Connection, OpenFlags, OptionalExtension, TransactionBehavior,
    backup::{Backup, StepResult},
};
use std::{
    fs::{self, DirBuilder, OpenOptions},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager, State};

#[cfg(any(target_os = "macos", windows))]
mod copies;
#[cfg(any(target_os = "macos", windows))]
pub use copies::{CopyIntent, CopyKey, CopyPublication, CopyRecovery};
#[cfg(any(target_os = "macos", windows))]
mod moves;
#[cfg(any(target_os = "macos", windows))]
pub use moves::{MoveKey, MoveRecovery};
#[cfg(any(target_os = "macos", windows))]
mod reference_monitor;
#[cfg(any(target_os = "macos", windows))]
pub use reference_monitor::{ReconciledReference, ReferenceMonitor};

const APPLICATION_ID: i64 = 0x4641_4953;
pub(crate) const FOLDER_VIEW_PREFIX: &str = "workspace.views.folder.v1:";
pub(crate) const COPY_METADATA_KEY: &str = "organization.copy-metadata.v1";
const MIGRATIONS: &[&str] = &[
    include_str!("schema/001.sql"),
    include_str!("schema/002.sql"),
    include_str!("schema/003.sql"),
    include_str!("schema/004.sql"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageError {
    Starting,
    Unavailable,
    UnexpectedDatabase,
    FutureSchema,
    IntegrityCheck,
    ReferenceConflict,
}

impl From<std::io::Error> for StorageError {
    fn from(_: std::io::Error) -> Self {
        Self::Unavailable
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(_: rusqlite::Error) -> Self {
        Self::Unavailable
    }
}

pub struct Store {
    pub connection: Connection,
    pub derived_dir: PathBuf,
}

#[derive(Default)]
pub struct Storage {
    store: OnceLock<Result<Mutex<Store>, StorageError>>,
}

impl Storage {
    #[cfg(test)]
    pub(crate) fn for_test(data: &Path, cache: &Path) -> Result<Self, StorageError> {
        let storage = Self::default();
        storage
            .store
            .set(Ok(Mutex::new(Store::open(data, cache)?)))
            .map_err(|_| StorageError::Unavailable)?;
        Ok(storage)
    }

    pub fn access<R>(
        &self,
        work: impl FnOnce(&mut Store) -> Result<R, StorageError>,
    ) -> Result<R, StorageError> {
        let result = self.store.get().ok_or(StorageError::Starting)?;
        let store = result.as_ref().map_err(|error| *error)?;
        let mut store = store.lock().map_err(|_| StorageError::Unavailable)?;
        work(&mut store)
    }
}

pub fn initialize(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state: State<'_, Storage> = app.state();
        state.store.get_or_init(|| {
            let open = || {
                let data = app
                    .path()
                    .app_local_data_dir()
                    .map_err(|_| StorageError::Unavailable)?;
                let cache = app
                    .path()
                    .app_cache_dir()
                    .map_err(|_| StorageError::Unavailable)?;
                Store::open(&data, &cache)
            };
            open().map(Mutex::new)
        });
    });
}

impl Store {
    pub fn open(data: &Path, cache: &Path) -> Result<Self, StorageError> {
        ensure_directory(data)?;
        let derived_dir = cache.join("derived");
        let path = data.join("user.sqlite");
        create_private_file_if_missing(&path)?;
        let connection = migrate(&path, MIGRATIONS)?;
        // The user database remains usable if the cache cannot currently be created.
        Ok(Self {
            connection,
            derived_dir,
        })
    }
}

pub(crate) fn copy_metadata(database: &Connection) -> Result<bool, StorageError> {
    let value: Option<String> = database
        .query_row(
            "SELECT value FROM preferences WHERE key=?1",
            [COPY_METADATA_KEY],
            |row| row.get(0),
        )
        .optional()?;
    match value.as_deref() {
        None | Some("true") => Ok(true),
        Some("false") => Ok(false),
        _ => Err(StorageError::Unavailable),
    }
}

#[cfg(any(target_os = "macos", windows))]
pub struct TrackedReference {
    id: String,
    previous: crate::local_identity::Locator,
    key: Vec<u8>,
    identity: crate::local_identity::LiveIdentity,
}

#[cfg(any(target_os = "macos", windows))]
impl TrackedReference {
    pub(crate) fn is_directory(&self) -> Result<bool, StorageError> {
        Ok(self.identity.kind()? == "directory")
    }

    pub(crate) fn verify_source(&self) -> Result<(), StorageError> {
        if self.identity.observe()?
            != crate::local_identity::LiveObservation::Present(self.previous.clone())
        {
            return Err(StorageError::ReferenceConflict);
        }
        Ok(())
    }

    pub(crate) fn matches_open_identity(
        &self,
        identity: &crate::local_identity::LiveIdentity,
    ) -> Result<bool, StorageError> {
        Ok(identity.comparison_key()? == self.key
            && identity.observe()?
                == crate::local_identity::LiveObservation::Present(self.previous.clone()))
    }
}

#[cfg(any(target_os = "macos", windows))]
impl Store {
    /// Resolve an explicit access to the object currently at this path. A
    /// stored identity never authorizes a rebind at a different path.
    pub fn resolve_live_item(
        &mut self,
        locator: &crate::local_identity::Locator,
        create: bool,
    ) -> Result<Option<String>, StorageError> {
        use crate::local_identity::{LiveIdentity, LiveObservation};

        let identity = LiveIdentity::open(locator)?;
        let LiveObservation::Present(observed) = identity.observe()? else {
            return Err(StorageError::ReferenceConflict);
        };
        let key = identity.comparison_key()?;
        let path = observed.encode();
        let match_at_path = |database: &Connection| -> Result<Option<String>, StorageError> {
            let (count, matching): (i64, Option<String>) = database.query_row(
                "SELECT count(*), min(CASE WHEN locator=?1 THEN id END)
                 FROM items WHERE native_key=?2 AND availability != 'missing'",
                rusqlite::params![path, key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            if count > 1 || (create && count == 1 && matching.is_none()) {
                return Err(StorageError::ReferenceConflict);
            }
            Ok(matching)
        };
        if !create {
            return match_at_path(&self.connection);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(id) = match_at_path(&transaction)? {
            return Ok(Some(id));
        }
        let pending: bool = transaction.query_row(
            "SELECT EXISTS (
                SELECT 1 FROM operation_items WHERE destination=?1
                 AND ((kind='copy' AND state IN ('planned', 'recovery'))
                   OR (kind='move' AND source_identity=?2 AND state IN ('planned', 'recovery', 'conflict')))
            )",
            rusqlite::params![path, key],
            |row| row.get(0),
        )?;
        if pending {
            return Err(StorageError::ReferenceConflict);
        }
        let id = uuid::Uuid::new_v4().to_string();
        transaction.execute(
            "INSERT INTO items (id, locator, native_key, kind, availability)
             VALUES (?1, ?2, ?3, ?4, 'available')",
            rusqlite::params![id, path, key, identity.kind()?],
        )?;
        transaction.commit()?;
        Ok(Some(id))
    }

    pub fn attach_live_reference(
        &mut self,
        id: &str,
        locator: &crate::local_identity::Locator,
    ) -> Result<TrackedReference, StorageError> {
        use crate::local_identity::{LiveIdentity, LiveObservation};

        let identity = LiveIdentity::open(locator)?;
        let LiveObservation::Present(previous) = identity.observe()? else {
            return Err(StorageError::ReferenceConflict);
        };
        let key = identity.comparison_key()?;
        let matches: bool = self.connection.query_row(
            "SELECT EXISTS (SELECT 1 FROM items WHERE id=?1 AND locator=?2
                            AND native_key=?3 AND availability != 'missing')",
            rusqlite::params![id, previous.encode(), key],
            |row| row.get(0),
        )?;
        if !matches {
            return Err(StorageError::ReferenceConflict);
        }
        Ok(TrackedReference {
            id: id.to_owned(),
            previous,
            key,
            identity,
        })
    }

    /// Persist a new tracked reference obtained from a live handle. The ID
    /// belongs to the application; paths and filesystem IDs can be reused.
    pub fn insert_live_reference(
        &mut self,
        id: &str,
        identity: crate::local_identity::LiveIdentity,
    ) -> Result<TrackedReference, StorageError> {
        use crate::local_identity::LiveObservation;
        let LiveObservation::Present(locator) = identity.observe()? else {
            return Err(StorageError::ReferenceConflict);
        };
        let key = identity.comparison_key()?;
        self.connection.execute(
            "INSERT INTO items (id, locator, native_key, kind, availability)
             VALUES (?1, ?2, ?3, ?4, 'available')",
            rusqlite::params![id, locator.encode(), key, identity.kind()?],
        )?;
        Ok(TrackedReference {
            id: id.to_owned(),
            previous: locator,
            key,
            identity,
        })
    }

    pub fn reconcile_live_reference(
        &mut self,
        tracked: &mut TrackedReference,
    ) -> Result<crate::local_identity::LiveObservation, StorageError> {
        use crate::local_identity::LiveObservation;
        let observation = tracked.identity.observe()?;
        let (locator, availability) = match &observation {
            LiveObservation::Present(locator) => (locator, "available"),
            LiveObservation::Unavailable => (&tracked.previous, "unavailable"),
            LiveObservation::Removed => (&tracked.previous, "missing"),
        };
        let observed = locator.encode();
        let previous = tracked.previous.encode();
        if observed == previous && observation != LiveObservation::Removed {
            let unchanged: bool = self.connection.query_row(
                "SELECT EXISTS (SELECT 1 FROM items WHERE id=?1 AND locator=?2
                                AND native_key=?3 AND availability=?4)",
                rusqlite::params![tracked.id, previous, tracked.key, availability],
                |row| row.get(0),
            )?;
            if unchanged {
                return Ok(observation);
            }
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let count = transaction.execute(
            "UPDATE items SET locator=?1, availability=?2
             WHERE id=?3 AND locator=?4 AND native_key=?5 AND availability != 'missing'
             AND NOT EXISTS (
                 SELECT 1 FROM items other
                 WHERE other.id != items.id AND other.locator=?1
                 AND other.native_key=?5 AND other.availability != 'missing'
             )
             AND NOT EXISTS (
                 SELECT 1 FROM operation_items
                 WHERE item_id = items.id AND kind = 'move'
                 AND state IN ('planned', 'recovery', 'conflict')
             )",
            rusqlite::params![observed, availability, tracked.id, previous, tracked.key],
        )?;
        if count != 1 {
            return Err(StorageError::ReferenceConflict);
        }
        if let LiveObservation::Present(locator) = &observation {
            relocate_folder_view(&transaction, &tracked.id, &tracked.previous, locator)?;
        }
        if observation == LiveObservation::Removed {
            transaction.execute("DELETE FROM favorites WHERE item_id=?1", [&tracked.id])?;
        }
        transaction.commit()?;
        if let LiveObservation::Present(locator) = &observation {
            tracked.previous = locator.clone();
        }
        Ok(observation)
    }
}

#[cfg(any(target_os = "macos", windows))]
fn relocate_folder_view(
    database: &Connection,
    id: &str,
    previous: &crate::local_identity::Locator,
    destination: &crate::local_identity::Locator,
) -> Result<(), StorageError> {
    if previous == destination {
        return Ok(());
    }
    let (Some(previous), Some(destination)) =
        (previous.path().to_str(), destination.path().to_str())
    else {
        return Ok(());
    };
    let previous = format!("{FOLDER_VIEW_PREFIX}{previous}");
    let destination = format!("{FOLDER_VIEW_PREFIX}{destination}");
    // Folder-view preferences created by the workspace are path keyed. Move
    // them only with verified folder identity, and preserve both on conflict.
    let copied = database.execute(
        "INSERT INTO preferences (key, value)
         SELECT ?1, value FROM preferences WHERE key=?2
         AND EXISTS (SELECT 1 FROM items WHERE id=?3 AND kind='directory')
         ON CONFLICT DO NOTHING",
        rusqlite::params![destination, previous, id],
    )?;
    if copied == 1 {
        database.execute("DELETE FROM preferences WHERE key=?1", [previous])?;
    }
    Ok(())
}

fn ensure_directory(path: &Path) -> Result<(), StorageError> {
    let mut builder = DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    Ok(())
}

fn create_private_file(path: &Path) -> std::io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)?;
    Ok(())
}

fn create_private_file_if_missing(path: &Path) -> Result<(), StorageError> {
    match create_private_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn check_integrity(connection: &Connection) -> Result<(), StorageError> {
    let integrity: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    let mut foreign_keys = connection.prepare("PRAGMA foreign_key_check")?;
    if integrity == "ok" && foreign_keys.query([])?.next()?.is_none() {
        Ok(())
    } else {
        Err(StorageError::IntegrityCheck)
    }
}

fn snapshot(path: &Path, version: usize) -> Result<(), StorageError> {
    let parent = path.parent().ok_or(StorageError::Unavailable)?;
    let recovery = parent.join("recovery");
    ensure_directory(&recovery)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StorageError::Unavailable)?
        .as_nanos();
    let pending = recovery.join(format!(
        "pre-v{version}-{}-{timestamp}.partial",
        std::process::id()
    ));
    let verified = pending.with_extension("sqlite");
    create_private_file(&pending)?;

    let result = (|| {
        let source = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let mut target = Connection::open_with_flags(&pending, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        if !matches!(
            Backup::new(&source, &mut target)?.step(-1)?,
            StepResult::Done
        ) {
            return Err(StorageError::Unavailable);
        }
        check_integrity(&target)?;
        target.close().map_err(|(_, error)| error)?;
        fs::File::open(&pending)?.sync_all()?;
        fs::rename(&pending, &verified)?;
        Ok::<_, StorageError>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&pending);
    }
    result
}

fn migrate(path: &Path, migrations: &[&str]) -> Result<Connection, StorageError> {
    let mut connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    connection.busy_timeout(Duration::from_secs(2))?;
    connection.pragma_update(None, "foreign_keys", true)?;
    connection.pragma_update(None, "synchronous", "FULL")?;

    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let version =
        transaction.pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))? as usize;
    let application_id: i64 =
        transaction.pragma_query_value(None, "application_id", |row| row.get(0))?;

    if version == 0 {
        let objects: i64 =
            transaction.query_row("SELECT count(*) FROM sqlite_schema", [], |row| row.get(0))?;
        if application_id != 0 || objects != 0 {
            return Err(StorageError::UnexpectedDatabase);
        }
        transaction.pragma_update(None, "application_id", APPLICATION_ID)?;
    } else {
        if application_id != APPLICATION_ID {
            return Err(StorageError::UnexpectedDatabase);
        }
        if version > migrations.len() {
            return Err(StorageError::FutureSchema);
        }
        if version < migrations.len() {
            // The transaction holds a reserved write lock while a second
            // connection snapshots the last committed contents.
            snapshot(path, version)?;
        }
    }

    for (index, sql) in migrations.iter().enumerate().skip(version) {
        transaction.execute_batch(sql)?;
        transaction.pragma_update(None, "user_version", index as i64 + 1)?;
    }
    check_integrity(&transaction)?;
    transaction.commit()?;
    Ok(connection)
}

#[cfg(test)]
mod tests;
