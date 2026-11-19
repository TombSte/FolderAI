use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
const METADATA: &str = "
    INSERT INTO items VALUES ('item', x'2f74657374', NULL, 'file', 'available');
    INSERT INTO notes VALUES ('item', 'metadata', 7);
    INSERT INTO tags VALUES ('tag', 'Test', NULL);
    INSERT INTO item_tags VALUES ('item', 'tag');
    INSERT INTO favorite_groups VALUES ('group', 'Group', 0);
    INSERT INTO favorites VALUES ('root', 'item', NULL, 0), ('nested', 'item', 'group', 0);
";

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let location =
            std::env::temp_dir().join(format!("folderai-storage-{}-{id}", std::process::id()));
        fs::create_dir(&location).unwrap();
        Self(location)
    }

    fn old_database(&self) -> PathBuf {
        let path = self.0.join("user.sqlite");
        create_private_file(&path).unwrap();
        let database = migrate(&path, &MIGRATIONS[..1]).unwrap();
        database.execute_batch(METADATA).unwrap();
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn assert_metadata(database: &Connection) {
    let note = database
        .query_row("SELECT body, revision FROM notes", [], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .unwrap();
    assert_eq!(note, ("metadata".to_owned(), 7));
    let tags: i64 = database
        .query_row("SELECT count(*) FROM item_tags", [], |row| row.get(0))
        .unwrap();
    assert_eq!(tags, 1);
    let favorite_count: i64 = database
        .query_row(
            "SELECT count(*) FROM favorites WHERE item_id='item'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(favorite_count, 2);
}

fn version(database: &Connection) -> usize {
    database
        .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
        .unwrap() as usize
}

#[test]
fn failed_upgrade_keeps_old_data_and_retry_preserves_metadata() {
    let fixture = Fixture::new();
    let path = fixture.old_database();
    let failing = [
        MIGRATIONS[0],
        "ALTER TABLE notes ADD COLUMN edited INTEGER;
         INSERT INTO notes (item_id, body, revision) VALUES ('missing', 'invalid', 1);",
    ];
    assert!(migrate(&path, &failing).is_err());
    let old = Connection::open(&path).unwrap();
    assert_eq!(version(&old), 1);
    assert_metadata(&old);
    assert!(old.prepare("SELECT edited FROM notes").is_err());
    drop(old);

    let backup = fs::read_dir(fixture.0.join("recovery"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(backup.extension().unwrap(), "sqlite");
    let snapshot = Connection::open_with_flags(backup, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(version(&snapshot), 1);
    check_integrity(&snapshot).unwrap();
    assert_metadata(&snapshot);

    let database = migrate(&path, MIGRATIONS).unwrap();
    assert_eq!(version(&database), MIGRATIONS.len());
    assert_metadata(&database);
    assert!(
        database
            .prepare("SELECT state FROM operation_items")
            .is_ok()
    );
}

#[test]
fn startup_preserves_unrelated_future_and_corrupt_stores() {
    let fixture = Fixture::new();
    let path = fixture.0.join("unrelated.sqlite");
    Connection::open(&path)
        .unwrap()
        .execute("CREATE TABLE other(p)", [])
        .unwrap();
    let before = fs::read(&path).unwrap();
    assert!(matches!(
        migrate(&path, MIGRATIONS),
        Err(StorageError::UnexpectedDatabase)
    ));
    assert_eq!(fs::read(path).unwrap(), before);

    let future = fixture.old_database();
    let connection = Connection::open(&future).unwrap();
    connection.pragma_update(None, "user_version", 99).unwrap();
    drop(connection);
    let before = fs::read(&future).unwrap();
    assert!(matches!(
        migrate(&future, MIGRATIONS),
        Err(StorageError::FutureSchema)
    ));
    assert_eq!(fs::read(future).unwrap(), before);

    let corrupt = fixture.0.join("corrupt.sqlite");
    let bytes = [b'a'; 1024];
    fs::write(&corrupt, bytes).unwrap();
    assert!(migrate(&corrupt, MIGRATIONS).is_err());
    assert_eq!(fs::read(corrupt).unwrap(), bytes);
}

#[test]
fn rebuilding_derived_store_leaves_user_database_intact() {
    let fixture = Fixture::new();
    let data = fixture.0.join("data");
    let cache = fixture.0.join("cache");
    let store = Store::open(&data, &cache).unwrap();
    store.connection.execute_batch(METADATA).unwrap();
    ensure_directory(&store.derived_dir).unwrap();
    fs::write(store.derived_dir.join("synthetic-index"), "derived").unwrap();
    fs::remove_dir_all(&store.derived_dir).unwrap();
    drop(store);
    let reopened = Store::open(&data, &cache).unwrap();
    ensure_directory(&reopened.derived_dir).unwrap();
    assert_metadata(&reopened.connection);
}

#[test]
fn migration_recovers_after_process_exit_before_commit() {
    // The subprocess runs this test again, writes to the rollback journal,
    // then exits without unwinding or closing SQLite.
    const CHILD_DATABASE: &str = "FOLDERAI_TEST_INTERRUPTED_MIGRATION_DB";
    if let Some(path) = std::env::var_os(CHILD_DATABASE) {
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch(
                "
            PRAGMA cache_size=2;
            BEGIN IMMEDIATE;
            ALTER TABLE notes ADD COLUMN edited INTEGER;
            PRAGMA user_version = 2;
            UPDATE notes SET body = 'should roll back';
        ",
            )
            .unwrap();
        std::process::exit(91);
    }
    let fixture = Fixture::new();
    let path = fixture.old_database();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "storage::tests::migration_recovers_after_process_exit_before_commit",
        ])
        .env(CHILD_DATABASE, &path)
        .output()
        .unwrap();
    assert_eq!(
        child.status.code(),
        Some(91),
        "{}",
        String::from_utf8_lossy(&child.stderr)
    );
    let unchanged = Connection::open(&path).unwrap();
    assert_eq!(version(&unchanged), 1);
    assert_metadata(&unchanged);
    drop(unchanged);
    let migrated = migrate(&path, MIGRATIONS).unwrap();
    assert_eq!(version(&migrated), MIGRATIONS.len());
    assert_metadata(&migrated);
}

#[cfg(target_os = "macos")]
#[test]
fn live_reconciliation_preserves_notes_and_removes_favorites_only_after_confirmed_loss() {
    use crate::local_identity::{LiveIdentity, LiveObservation, Locator};

    let fixture = Fixture::new();
    let mut store = Store::open(&fixture.0.join("data"), &fixture.0.join("cache")).unwrap();
    let original = fixture.0.join("original");
    fs::write(&original, "original").unwrap();
    let handle = LiveIdentity::open(&Locator::new(original.clone()).unwrap()).unwrap();
    let mut tracked = store.insert_live_reference("item", handle).unwrap();
    store
        .connection
        .execute_batch(METADATA.split_once(';').unwrap().1)
        .unwrap();

    let moved = fixture.0.join("moved");
    fs::rename(&original, &moved).unwrap();
    let LiveObservation::Present(location) = store.reconcile_live_reference(&mut tracked).unwrap()
    else {
        panic!("expected moved reference");
    };
    assert_eq!(
        fs::canonicalize(location.path()).unwrap(),
        fs::canonicalize(&moved).unwrap()
    );
    assert_metadata(&store.connection);

    let alias = fixture.0.join("alias");
    fs::hard_link(&moved, &alias).unwrap();
    assert_eq!(
        store.reconcile_live_reference(&mut tracked).unwrap(),
        LiveObservation::Unavailable
    );
    assert_metadata(&store.connection);
    fs::remove_file(alias).unwrap();

    fs::write(&original, "replacement").unwrap();
    fs::rename(&original, &moved).unwrap();
    assert_eq!(
        store.reconcile_live_reference(&mut tracked).unwrap(),
        LiveObservation::Removed
    );
    let favorites: i64 = store
        .connection
        .query_row("SELECT count(*) FROM favorites", [], |row| row.get(0))
        .unwrap();
    let note: String = store
        .connection
        .query_row("SELECT body FROM notes WHERE item_id='item'", [], |row| {
            row.get(0)
        })
        .unwrap();
    let tags: i64 = store
        .connection
        .query_row(
            "SELECT count(*) FROM item_tags WHERE item_id='item'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!((favorites, note.as_str(), tags), (0, "metadata", 1));
    assert_eq!(fs::read_to_string(moved).unwrap(), "replacement");
}

#[cfg(target_os = "macos")]
#[test]
fn move_commit_failure_rolls_back_metadata_and_retry_is_idempotent() {
    use crate::local_identity::{LiveIdentity, Locator};

    let fixture = Fixture::new();
    let mut store = Store::open(&fixture.0.join("data"), &fixture.0.join("cache")).unwrap();
    let source = fixture.0.join("source");
    let destination = fixture.0.join("destination");
    fs::write(&source, "synthetic original").unwrap();
    let handle = LiveIdentity::open(&Locator::new(source.clone()).unwrap()).unwrap();
    let mut tracked = store.insert_live_reference("item", handle).unwrap();
    store
        .connection
        .execute_batch(METADATA.split_once(';').unwrap().1)
        .unwrap();

    let key = MoveKey {
        operation_id: "move",
        item_key: "first",
    };
    let target = Locator::new(destination.clone()).unwrap();
    store.record_move(key, &tracked, &target).unwrap();
    fs::rename(&source, &destination).unwrap();

    // The trigger succeeds inside the transaction. The deferred foreign key
    // makes COMMIT fail after the item locator and journal have been updated.
    store.connection.execute_batch(
        "CREATE TEMP TABLE fault_parent (id INTEGER PRIMARY KEY);
         CREATE TEMP TABLE fault_child (parent INTEGER REFERENCES fault_parent(id) DEFERRABLE INITIALLY DEFERRED);
         CREATE TEMP TRIGGER fault_on_completion AFTER UPDATE OF state ON main.operation_items
         WHEN NEW.state = 'completed' BEGIN INSERT INTO fault_child VALUES (42); END;",
    ).unwrap();
    assert!(store.recover_move(key, Some(&mut tracked)).is_err());
    let original: Vec<u8> = store
        .connection
        .query_row("SELECT locator FROM items", [], |row| row.get(0))
        .unwrap();
    let phase: String = store
        .connection
        .query_row("SELECT state FROM operation_items", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        Locator::decode(original).unwrap().path(),
        fs::canonicalize(&fixture.0).unwrap().join("source")
    );
    assert_eq!(phase, "planned");
    assert!(destination.is_file());
    assert_metadata(&store.connection);

    store
        .connection
        .execute_batch("DROP TRIGGER fault_on_completion")
        .unwrap();
    assert_eq!(
        store.recover_move(key, Some(&mut tracked)).unwrap(),
        MoveRecovery::Completed
    );
    assert_eq!(
        store.recover_move(key, Some(&mut tracked)).unwrap(),
        MoveRecovery::Completed
    );
    let saved: Vec<u8> = store
        .connection
        .query_row("SELECT locator FROM items", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        Locator::decode(saved).unwrap().path(),
        fs::canonicalize(destination).unwrap()
    );
    assert_metadata(&store.connection);
}

#[cfg(target_os = "macos")]
#[test]
fn reloaded_move_waits_for_proof_and_replacement_never_inherits_metadata() {
    use crate::local_identity::{LiveIdentity, Locator};

    let fixture = Fixture::new();
    let data = fixture.0.join("data");
    let cache = fixture.0.join("cache");
    let mut store = Store::open(&data, &cache).unwrap();
    let source = fixture.0.join("source");
    let destination = fixture.0.join("destination");
    fs::write(&source, "original").unwrap();
    let handle = LiveIdentity::open(&Locator::new(source.clone()).unwrap()).unwrap();
    let mut tracked = store.insert_live_reference("item", handle).unwrap();
    store
        .connection
        .execute_batch(METADATA.split_once(';').unwrap().1)
        .unwrap();
    let key = MoveKey {
        operation_id: "move",
        item_key: "first",
    };
    store
        .record_move(key, &tracked, &Locator::new(destination.clone()).unwrap())
        .unwrap();
    fs::rename(&source, &destination).unwrap();
    drop(store);

    let mut reopened = Store::open(&data, &cache).unwrap();
    assert_eq!(
        reopened.recover_move(key, None).unwrap(),
        MoveRecovery::AwaitingEvidence
    );
    assert_metadata(&reopened.connection);
    fs::write(&source, "unrelated").unwrap();
    fs::rename(&source, &destination).unwrap();
    assert_eq!(
        reopened.recover_move(key, Some(&mut tracked)).unwrap(),
        MoveRecovery::Conflict
    );
    assert_metadata(&reopened.connection);
    assert_eq!(fs::read_to_string(destination).unwrap(), "unrelated");
    assert!(matches!(
        reopened.reconcile_live_reference(&mut tracked),
        Err(StorageError::ReferenceConflict)
    ));
}

#[cfg(target_os = "macos")]
#[test]
fn watch_rescan_reconciles_a_rename_without_a_rename_event() {
    use crate::local_identity::{LiveObservation, Locator, watch::DirtyPaths};

    let fixture = Fixture::new();
    let files = fixture.0.join("files");
    fs::create_dir(&files).unwrap();
    let mut store = Store::open(&fixture.0.join("data"), &fixture.0.join("cache")).unwrap();
    let mut monitor = ReferenceMonitor::new();
    monitor.watch_root(&files).unwrap();
    let original = files.join("before");
    fs::write(&original, "synthetic file").unwrap();
    monitor
        .track(&mut store, "item", &Locator::new(original.clone()).unwrap())
        .unwrap();
    store
        .connection
        .execute_batch(METADATA.split_once(';').unwrap().1)
        .unwrap();

    let moved = files.join("after");
    fs::rename(original, &moved).unwrap();
    let results = monitor.reconcile(&mut store, DirtyPaths::Rescan);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "item");
    let Ok(LiveObservation::Present(found)) = &results[0].outcome else {
        panic!("full rescan should resolve the live item");
    };
    assert_eq!(found.path(), fs::canonicalize(moved).unwrap());
    assert_metadata(&store.connection);
}

#[cfg(target_os = "macos")]
#[test]
fn watcher_hints_trigger_reference_reconciliation() {
    use crate::local_identity::{LiveObservation, Locator};
    use std::time::Instant;

    let fixture = Fixture::new();
    let files = fixture.0.join("files");
    fs::create_dir(&files).unwrap();
    let original = files.join("before");
    fs::write(&original, "synthetic file").unwrap();
    let mut store = Store::open(&fixture.0.join("data"), &fixture.0.join("cache")).unwrap();
    let mut monitor = ReferenceMonitor::new();
    monitor.watch_root(&files).unwrap();
    monitor
        .track(&mut store, "item", &Locator::new(original.clone()).unwrap())
        .unwrap();
    let _ = monitor.wait(Duration::ZERO);
    let moved = files.join("after");
    fs::rename(original, &moved).unwrap();

    let destination = fs::canonicalize(moved).unwrap();
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        let hints = monitor.wait(Duration::from_millis(250));
        let results = monitor.reconcile(&mut store, hints);
        if results.iter().any(|result| {
            matches!(&result.outcome, Ok(LiveObservation::Present(locator)) if locator.path() == destination)
        }) {
            return;
        }
    }
    panic!("no watcher hint reconciled the renamed fixture");
}

#[cfg(target_os = "macos")]
#[test]
fn tree_move_journals_descendants_atomically_before_destination_exists() {
    use crate::local_identity::{LiveIdentity, Locator};
    let fixture = Fixture::new();
    let mut store = Store::open(&fixture.0.join("data"), &fixture.0.join("cache")).unwrap();
    let root = fixture.0.join("folder");
    let destination = fixture.0.join("renamed");
    let neighbor = fixture.0.join("folder-copy");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("child"), "retained").unwrap();
    fs::write(&neighbor, "unrelated").unwrap();
    let mut track = |id, path| {
        let handle = LiveIdentity::open(&Locator::new(path).unwrap()).unwrap();
        store.insert_live_reference(id, handle).unwrap()
    };
    let mut root_ref = track("root-folder", root.clone());
    let mut child_ref = track("item", root.join("child"));
    let neighbor_ref = track("neighbor", neighbor);
    store
        .connection
        .execute_batch(METADATA.split_once(';').unwrap().1)
        .unwrap();
    let target = Locator::new(destination.clone()).unwrap();
    assert!(matches!(
        store.record_tree_move("tree", &root_ref, &[&child_ref, &neighbor_ref], &target),
        Err(StorageError::ReferenceConflict)
    ));
    let intents: i64 = store
        .connection
        .query_row("SELECT count(*) FROM operation_items", [], |row| row.get(0))
        .unwrap();
    assert_eq!(intents, 0);

    store
        .record_tree_move("tree", &root_ref, &[&child_ref], &target)
        .unwrap();
    fs::rename(root, &destination).unwrap();
    for (item_key, reference) in [("root-folder", &mut root_ref), ("item", &mut child_ref)] {
        let key = MoveKey {
            operation_id: "tree",
            item_key,
        };
        assert_eq!(
            store.recover_move(key, Some(reference)).unwrap(),
            MoveRecovery::Completed
        );
    }
    let saved: Vec<u8> = store
        .connection
        .query_row("SELECT locator FROM items WHERE id='item'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(
        Locator::decode(saved).unwrap().path(),
        fs::canonicalize(destination.join("child")).unwrap()
    );
    assert_metadata(&store.connection);
}

#[cfg(target_os = "macos")]
#[test]
fn opening_metadata_after_restart_distinguishes_same_path_replacement() {
    use crate::local_identity::Locator;

    let fixture = Fixture::new();
    let data = fixture.0.join("data");
    let cache = fixture.0.join("cache");
    let path = fixture.0.join("target");
    fs::write(&path, "original").unwrap();
    let locator = Locator::new(path.clone()).unwrap();
    let mut store = Store::open(&data, &cache).unwrap();
    assert_eq!(store.resolve_live_item(&locator, false).unwrap(), None);
    let id = store.resolve_live_item(&locator, true).unwrap().unwrap();
    store
        .connection
        .execute("INSERT INTO notes VALUES (?1, 'original note', 1)", [&id])
        .unwrap();
    drop(store);
    let mut reopened = Store::open(&data, &cache).unwrap();
    assert_eq!(
        reopened.resolve_live_item(&locator, false).unwrap(),
        Some(id.clone())
    );
    let replacement = fixture.0.join("replacement");
    fs::write(&replacement, "another object").unwrap();
    fs::rename(replacement, path).unwrap();
    assert_eq!(reopened.resolve_live_item(&locator, false).unwrap(), None);
    assert_ne!(
        reopened.resolve_live_item(&locator, true).unwrap(),
        Some(id.clone())
    );
    let note: String = reopened
        .connection
        .query_row("SELECT body FROM notes WHERE item_id=?1", [&id], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(note, "original note");
}
