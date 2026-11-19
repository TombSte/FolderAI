use super::{StorageError, Store, TrackedReference, copy_metadata};
use crate::local_identity::{LiveIdentity, LiveObservation, Locator};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::{fs, io, sync::OnceLock};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CopyKey<'a> {
    pub operation_id: &'a str,
    pub item_key: &'a str,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CopyRecovery {
    Completed(String),
    AwaitingEvidence,
    Conflict,
    Failed,
}

/// Authority issued exactly once, when the intent is first durably recorded.
/// The filesystem engine retains this token and its sealed staging handle
/// until the metadata commit has succeeded.
pub struct CopyIntent {
    operation_id: String,
    item_key: String,
    nonce: String,
    destination: Locator,
    witness_bound: OnceLock<()>,
    completion_attempted: OnceLock<()>,
}

pub struct CopyPublication {
    nonce: String,
    identity: LiveIdentity,
    staging_location: Locator,
}

impl CopyIntent {
    pub fn destination(&self) -> &Locator {
        &self.destination
    }

    /// Seal the actual staging object before the filesystem engine publishes
    /// it. The existing or freshly reopened final path is not evidence.
    pub fn bind_staged_destination(
        &self,
        staged: LiveIdentity,
    ) -> Result<CopyPublication, StorageError> {
        let LiveObservation::Present(staging_location) = staged.observe()? else {
            return Err(StorageError::ReferenceConflict);
        };
        if staging_location == self.destination {
            return Err(StorageError::ReferenceConflict);
        }
        self.witness_bound
            .set(())
            .map_err(|_| StorageError::ReferenceConflict)?;
        Ok(CopyPublication {
            nonce: self.nonce.clone(),
            identity: staged,
            staging_location,
        })
    }
}

struct CopyRecord {
    state: String,
    item: Option<String>,
    source_item: Option<String>,
    source_kind: Option<String>,
    source_key: Option<Vec<u8>>,
    destination: Option<Vec<u8>>,
    nonce: Option<String>,
    prior_destination_key: Option<Vec<u8>>,
    destination_key: Option<Vec<u8>>,
    copy_metadata: Option<bool>,
}

fn load_record(database: &Connection, key: CopyKey<'_>) -> Result<CopyRecord, StorageError> {
    database
        .query_row(
            "SELECT o.state, o.item_id, o.copy_source_item_id, i.kind, o.source_identity,
                o.destination, o.copy_intent_nonce, o.prior_destination_identity,
                o.destination_identity, p.copy_metadata
         FROM operation_items o
         LEFT JOIN copy_operations p ON p.operation_id=o.operation_id
         LEFT JOIN items i ON i.id=o.copy_source_item_id
         WHERE o.operation_id=?1 AND o.item_key=?2 AND o.kind='copy'",
            params![key.operation_id, key.item_key],
            |row| {
                Ok(CopyRecord {
                    state: row.get(0)?,
                    item: row.get(1)?,
                    source_item: row.get(2)?,
                    source_kind: row.get(3)?,
                    source_key: row.get(4)?,
                    destination: row.get(5)?,
                    nonce: row.get(6)?,
                    prior_destination_key: row.get(7)?,
                    destination_key: row.get(8)?,
                    copy_metadata: row.get(9)?,
                })
            },
        )
        .optional()?
        .ok_or(StorageError::ReferenceConflict)
}

fn canonical_destination(destination: &Locator) -> Result<Locator, StorageError> {
    let path = destination.path();
    // For a tree copy, some parents may only exist after publication.
    for ancestor in path.ancestors().skip(1) {
        match crate::local_identity::canonical_path(ancestor) {
            Ok(canonical) => {
                if !fs::metadata(&canonical)?.is_dir() {
                    return Err(StorageError::ReferenceConflict);
                }
                let suffix = path
                    .strip_prefix(ancestor)
                    .map_err(|_| StorageError::ReferenceConflict)?;
                return Locator::new(canonical.join(suffix)).ok_or(StorageError::ReferenceConflict);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Err(StorageError::ReferenceConflict)
}

fn prior_destination_key(destination: &Locator) -> Result<Option<Vec<u8>>, StorageError> {
    match fs::symlink_metadata(destination.path()) {
        Ok(_) => {
            let identity = LiveIdentity::open(destination)?;
            if !matches!(identity.observe()?, LiveObservation::Present(_)) {
                return Err(StorageError::ReferenceConflict);
            }
            Ok(Some(identity.comparison_key()?))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn update_state(database: &Connection, key: CopyKey<'_>, state: &str) -> Result<(), StorageError> {
    database.execute(
        "UPDATE operation_items SET state=?3 WHERE operation_id=?1 AND item_key=?2",
        params![key.operation_id, key.item_key, state],
    )?;
    Ok(())
}

impl Store {
    pub(crate) fn copy_destination_reserved(
        &self,
        destination: &Locator,
    ) -> Result<bool, StorageError> {
        Ok(self.connection.query_row(
            "SELECT EXISTS (SELECT 1 FROM operation_items WHERE destination=?1
             AND kind='copy' AND state IN ('planned','recovery'))",
            [destination.encode()],
            |row| row.get(0),
        )?)
    }

    /// Commit intent before staging or publishing an independent copy.
    /// The first successful intent captures the preference for all siblings.
    pub fn record_copy_intent(
        &mut self,
        key: CopyKey<'_>,
        source: &TrackedReference,
        destination: &Locator,
    ) -> Result<CopyIntent, StorageError> {
        if [key.operation_id, key.item_key]
            .iter()
            .any(|id| id.is_empty() || id.len() > 128 || id.chars().any(char::is_control))
        {
            return Err(StorageError::ReferenceConflict);
        }
        let destination = canonical_destination(destination)?;
        if source.previous == destination
            || (source.identity.kind()? == "directory"
                && destination.path().starts_with(source.previous.path()))
            || source.identity.observe()? != LiveObservation::Present(source.previous.clone())
            || source.identity.comparison_key()? != source.key
        {
            return Err(StorageError::ReferenceConflict);
        }
        let prior_key = prior_destination_key(&destination)?;
        if prior_key.as_ref() == Some(&source.key) {
            return Err(StorageError::ReferenceConflict);
        }
        let nonce = Uuid::new_v4().to_string();
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let captured: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM copy_operations WHERE operation_id=?1)",
            [key.operation_id],
            |row| row.get(0),
        )?;
        if !captured {
            let legacy: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM operation_items WHERE operation_id=?1 AND kind='copy')",
                [key.operation_id],
                |row| row.get(0),
            )?;
            if legacy {
                return Err(StorageError::ReferenceConflict);
            }
            let enabled = copy_metadata(&transaction)?;
            transaction.execute(
                "INSERT INTO copy_operations (operation_id, copy_metadata) VALUES (?1,?2)",
                params![key.operation_id, enabled],
            )?;
        }
        let inserted = transaction.execute(
            "INSERT INTO operation_items (operation_id, item_key, kind, source, destination,
                source_identity, state, copy_source_item_id, copy_intent_nonce, prior_destination_identity)
             SELECT ?1, ?2, 'copy', locator, ?3, native_key, 'planned', id, ?4, ?5
             FROM items WHERE id=?6 AND locator=?7 AND native_key=?8 AND availability='available'
             ON CONFLICT DO NOTHING",
            params![
                key.operation_id,
                key.item_key,
                destination.encode(),
                nonce,
                prior_key,
                source.id,
                source.previous.encode(),
                source.key,
            ],
        )?;
        if inserted != 1 {
            return Err(StorageError::ReferenceConflict);
        }
        transaction.commit()?;
        Ok(CopyIntent {
            operation_id: key.operation_id.to_owned(),
            item_key: key.item_key.to_owned(),
            nonce,
            destination,
            witness_bound: OnceLock::new(),
            completion_attempted: OnceLock::new(),
        })
    }

    /// Apply the metadata effect using the handle retained from native
    /// publication. Opening an object by its destination path to reconstruct
    /// evidence is prohibited by this contract.
    pub fn record_completed_copy(
        &mut self,
        intent: &CopyIntent,
        publication: &CopyPublication,
    ) -> Result<String, StorageError> {
        if publication.nonce != intent.nonce {
            return Err(StorageError::ReferenceConflict);
        }
        intent.completion_attempted.get_or_init(|| ());
        let native_key = publication.identity.comparison_key()?;

        let key = CopyKey {
            operation_id: &intent.operation_id,
            item_key: &intent.item_key,
        };
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let record = load_record(&transaction, key)?;
        let expected_path = intent.destination.encode();
        if record.nonce.as_deref() != Some(&intent.nonce)
            || record.destination.as_ref() != Some(&expected_path)
        {
            return Err(StorageError::ReferenceConflict);
        }
        if record.state == "completed" {
            return if record.destination_key.as_ref() == Some(&native_key) {
                record.item.ok_or(StorageError::ReferenceConflict)
            } else {
                Err(StorageError::ReferenceConflict)
            };
        }
        if !matches!(record.state.as_str(), "planned" | "recovery") {
            return Err(StorageError::ReferenceConflict);
        }
        let kind = publication.identity.kind()?;
        let observation = publication.identity.observe()?;
        if observation == LiveObservation::Unavailable
            || matches!(&observation,
                LiveObservation::Present(locator) if locator == &publication.staging_location)
        {
            update_state(&transaction, key, "recovery")?;
            transaction.commit()?;
            return Err(StorageError::Unavailable);
        }
        if observation != LiveObservation::Present(intent.destination.clone())
            || record.source_key.as_ref() == Some(&native_key)
            || record.prior_destination_key.as_ref() == Some(&native_key)
            || record.source_kind.as_deref() != Some(kind)
        {
            update_state(&transaction, key, "conflict")?;
            transaction.commit()?;
            return Err(StorageError::ReferenceConflict);
        }
        let source_item = record.source_item.ok_or(StorageError::ReferenceConflict)?;
        let copy_metadata = record
            .copy_metadata
            .ok_or(StorageError::ReferenceConflict)?;
        let already_tracked: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM items WHERE native_key=?1 AND availability!='missing')",
            [&native_key],
            |row| row.get(0),
        )?;
        if already_tracked {
            update_state(&transaction, key, "conflict")?;
            transaction.commit()?;
            return Err(StorageError::ReferenceConflict);
        }

        let id = Uuid::new_v4().to_string();
        transaction.execute(
            "INSERT INTO items (id, locator, native_key, kind, availability)
             VALUES (?1,?2,?3,?4,'available')",
            params![id, expected_path, native_key, kind],
        )?;
        if copy_metadata {
            transaction.execute(
                "INSERT INTO notes (item_id, body, revision)
                 SELECT ?1, body, 1 FROM notes WHERE item_id=?2",
                params![id, source_item],
            )?;
            transaction.execute(
                "INSERT INTO item_tags (item_id, tag_id)
                 SELECT ?1, tag_id FROM item_tags WHERE item_id=?2",
                params![id, source_item],
            )?;
        }
        transaction.execute(
            "UPDATE operation_items SET item_id=?3, destination_identity=?4, state='completed'
             WHERE operation_id=?1 AND item_key=?2",
            params![key.operation_id, key.item_key, id, native_key],
        )?;
        transaction.commit()?;
        Ok(id)
    }

    /// Reopening a journal can report prior completion or expose recoverable
    /// uncertainty. It cannot issue native proof or repeat filesystem work.
    pub fn recover_copy(&mut self, key: CopyKey<'_>) -> Result<CopyRecovery, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let record = load_record(&transaction, key)?;
        let outcome = match record.state.as_str() {
            "completed" => {
                CopyRecovery::Completed(record.item.ok_or(StorageError::ReferenceConflict)?)
            }
            "failed" => CopyRecovery::Failed,
            "conflict" => CopyRecovery::Conflict,
            "planned" | "recovery" => {
                update_state(&transaction, key, "recovery")?;
                CopyRecovery::AwaitingEvidence
            }
            _ => return Err(StorageError::ReferenceConflict),
        };
        transaction.commit()?;
        Ok(outcome)
    }

    /// Mark a confirmed no-publication failure. An uncertain filesystem
    /// outcome must remain recoverable instead.
    pub fn fail_copy(&mut self, intent: &CopyIntent) -> Result<(), StorageError> {
        if intent.completion_attempted.get().is_some() {
            return Err(StorageError::ReferenceConflict);
        }
        let changed = self.connection.execute(
            "UPDATE operation_items SET state='failed'
             WHERE operation_id=?1 AND item_key=?2 AND copy_intent_nonce=?3 AND kind='copy'
                 AND state IN ('planned', 'recovery', 'failed')",
            params![intent.operation_id, intent.item_key, intent.nonce],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StorageError::ReferenceConflict)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{COPY_METADATA_KEY, MIGRATIONS, create_private_file, migrate};
    use std::path::{Path, PathBuf};

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("folderai-copy-{}", Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn open(&self) -> Store {
            Store::open(&self.0, &self.0).unwrap()
        }

        fn locator(&self, relative: &str) -> Locator {
            Locator::new(self.0.join(relative)).unwrap()
        }

        fn source(&self, store: &mut Store) -> TrackedReference {
            let file = self.0.join("source.txt");
            fs::write(&file, "contents").unwrap();
            let source = store.insert_live_reference("source", open(&file)).unwrap();
            store
                .connection
                .execute_batch(
                    "INSERT INTO notes VALUES ('source','draft',7);
                 INSERT INTO tags VALUES ('tag','known',NULL);
                 INSERT INTO item_tags VALUES ('source','tag');
                 INSERT INTO favorites VALUES ('favorite','source',NULL,0);",
                )
                .unwrap();
            source
        }

        fn publish(&self, intent: &CopyIntent) -> CopyPublication {
            let staged = self.0.join(format!(".staged-{}", Uuid::new_v4()));
            fs::copy(self.0.join("source.txt"), &staged).unwrap();
            let witness = intent.bind_staged_destination(open(&staged)).unwrap();
            fs::rename(staged, intent.destination().path()).unwrap();
            witness
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn open(path: &Path) -> LiveIdentity {
        LiveIdentity::open(&Locator::new(path.to_path_buf()).unwrap()).unwrap()
    }

    fn key<'a>(operation_id: &'a str, item_key: &'a str) -> CopyKey<'a> {
        CopyKey {
            operation_id,
            item_key,
        }
    }

    fn note(store: &Store, item: &str) -> Option<(String, i64)> {
        store
            .connection
            .query_row(
                "SELECT body, revision FROM notes WHERE item_id=?1",
                [item],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .unwrap()
    }

    #[test]
    fn copy_preference_is_captured_once_and_metadata_is_independent() {
        let fixture = Fixture::new();
        let mut store = fixture.open();
        let source = fixture.source(&mut store);

        let first_key = key("batch", "first");
        let first = store
            .record_copy_intent(first_key, &source, &fixture.locator("first.txt"))
            .unwrap();
        assert!(!first.destination().path().exists());
        assert!(
            store
                .record_copy_intent(first_key, &source, &fixture.locator("first.txt"))
                .is_err()
        );
        assert!(
            store
                .record_copy_intent(
                    key("other", "collision"),
                    &source,
                    &fixture.locator("first.txt")
                )
                .is_err()
        );
        store
            .connection
            .execute(
                "INSERT INTO preferences VALUES (?1,'false')",
                [COPY_METADATA_KEY],
            )
            .unwrap();
        let sibling = store
            .record_copy_intent(
                key("batch", "sibling"),
                &source,
                &fixture.locator("future/child.txt"),
            )
            .unwrap();
        let disabled = store
            .record_copy_intent(
                key("disabled", "copy"),
                &source,
                &fixture.locator("disabled.txt"),
            )
            .unwrap();
        let witness = fixture.publish(&first);
        let first_id = store.record_completed_copy(&first, &witness).unwrap();
        let staged_tree = fixture.0.join(".staged-tree");
        fs::create_dir(&staged_tree).unwrap();
        let staged_child = staged_tree.join("child.txt");
        fs::copy(fixture.0.join("source.txt"), &staged_child).unwrap();
        let child_witness = sibling
            .bind_staged_destination(open(&staged_child))
            .unwrap();
        assert_eq!(
            store.record_completed_copy(&sibling, &child_witness),
            Err(StorageError::Unavailable)
        );
        assert!(!sibling.destination().path().exists());
        fs::rename(staged_tree, sibling.destination().path().parent().unwrap()).unwrap();
        let sibling_id = store
            .record_completed_copy(&sibling, &child_witness)
            .unwrap();
        let without_id = store
            .record_completed_copy(&disabled, &fixture.publish(&disabled))
            .unwrap();
        assert_ne!(first_id, sibling_id);
        assert_eq!(note(&store, &first_id), Some(("draft".into(), 1)));
        assert_eq!(note(&store, &sibling_id), Some(("draft".into(), 1)));
        assert_eq!(note(&store, &without_id), None);
        let flags = |id: &str| {
            store
                .connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM item_tags WHERE item_id=?1 AND tag_id='tag'),
                    EXISTS(SELECT 1 FROM favorites WHERE item_id=?1)",
                    [id],
                    |row| Ok((row.get::<_, bool>(0)?, row.get::<_, bool>(1)?)),
                )
                .unwrap()
        };
        assert_eq!(flags(&first_id), (true, false));
        assert_eq!(flags(&without_id), (false, false));

        store
            .connection
            .execute(
                "UPDATE notes SET body='edited', revision=2 WHERE item_id=?1",
                [&first_id],
            )
            .unwrap();
        assert_eq!(
            store.record_completed_copy(&first, &witness),
            Ok(first_id.clone())
        );
        assert_eq!(
            store.recover_copy(first_key),
            Ok(CopyRecovery::Completed(first_id.clone()))
        );
        assert_eq!(note(&store, &first_id), Some(("edited".into(), 2)));
        assert_eq!(note(&store, "source"), Some(("draft".into(), 7)));
    }

    #[test]
    fn failed_commit_retries_after_reopen_only_with_the_original_publication() {
        let fixture = Fixture::new();
        let mut store = fixture.open();
        let source = fixture.source(&mut store);
        let operation = key("batch", "recover");
        let destination = fixture.locator("published.txt");
        let intent = store
            .record_copy_intent(operation, &source, &destination)
            .unwrap();
        let witness = fixture.publish(&intent);
        store
            .connection
            .execute_batch(
                "CREATE TABLE deferred_failure (
                missing TEXT REFERENCES items(id) DEFERRABLE INITIALLY DEFERRED
             );
             CREATE TRIGGER reject_copy_commit AFTER UPDATE OF state ON operation_items
             WHEN NEW.state='completed'
             BEGIN INSERT INTO deferred_failure VALUES ('missing-item'); END;",
            )
            .unwrap();
        assert_eq!(
            store.record_completed_copy(&intent, &witness),
            Err(StorageError::Unavailable)
        );
        assert_eq!(
            store.recover_copy(operation),
            Ok(CopyRecovery::AwaitingEvidence)
        );
        assert_eq!(
            store.resolve_live_item(&destination, true),
            Err(StorageError::ReferenceConflict)
        );
        assert!(store.fail_copy(&intent).is_err());
        drop(store);

        let mut reopened = fixture.open();
        assert_eq!(
            reopened.recover_copy(operation),
            Ok(CopyRecovery::AwaitingEvidence)
        );
        assert!(
            reopened
                .record_copy_intent(operation, &source, &destination)
                .is_err()
        );
        assert_eq!(reopened.resolve_live_item(&destination, false), Ok(None));
        reopened
            .connection
            .execute_batch("DROP TRIGGER reject_copy_commit")
            .unwrap();
        let id = reopened.record_completed_copy(&intent, &witness).unwrap();
        assert_eq!(
            reopened.recover_copy(operation),
            Ok(CopyRecovery::Completed(id.clone()))
        );
        assert_eq!(
            reopened.resolve_live_item(&destination, false),
            Ok(Some(id.clone()))
        );
        assert_eq!(note(&reopened, &id), Some(("draft".into(), 1)));
        assert_eq!(fs::read_to_string(destination.path()).unwrap(), "contents");
    }

    #[test]
    fn replacement_after_publication_cannot_adopt_the_original_copy() {
        let fixture = Fixture::new();
        let mut store = fixture.open();
        let source = fixture.source(&mut store);
        let operation = key("batch", "conflict");
        let destination = fixture.locator("published.txt");
        let intent = store
            .record_copy_intent(operation, &source, &destination)
            .unwrap();
        let witness = fixture.publish(&intent);
        fs::rename(destination.path(), fixture.0.join("moved.txt")).unwrap();
        fs::write(destination.path(), "replacement").unwrap();
        let other = store
            .record_copy_intent(
                key("batch", "independent"),
                &source,
                &fixture.locator("independent.txt"),
            )
            .unwrap();
        let unrelated_witness = fixture.publish(&other);

        assert_eq!(
            store.record_completed_copy(&intent, &witness),
            Err(StorageError::ReferenceConflict)
        );
        assert_eq!(store.recover_copy(operation), Ok(CopyRecovery::Conflict));
        assert_eq!(
            store.record_completed_copy(&intent, &unrelated_witness),
            Err(StorageError::ReferenceConflict)
        );
        assert!(
            intent
                .bind_staged_destination(open(destination.path()))
                .is_err()
        );
        let id = store
            .resolve_live_item(&destination, true)
            .unwrap()
            .unwrap();
        assert_eq!(note(&store, &id), None);
        assert_eq!(note(&store, "source"), Some(("draft".into(), 7)));
    }

    #[test]
    fn existing_target_is_never_evidence_of_replacement() {
        let fixture = Fixture::new();
        let mut store = fixture.open();
        let source = fixture.source(&mut store);
        let destination = fixture.locator("existing.txt");
        fs::write(destination.path(), "old contents").unwrap();
        let mistaken_key = key("first", "existing");
        let mistaken = store
            .record_copy_intent(mistaken_key, &source, &destination)
            .unwrap();
        assert!(matches!(
            mistaken.bind_staged_destination(open(destination.path())),
            Err(StorageError::ReferenceConflict)
        ));
        let staging_path = fixture.0.join("wrong-stage.txt");
        fs::rename(destination.path(), &staging_path).unwrap();
        let old = mistaken
            .bind_staged_destination(open(&staging_path))
            .unwrap();
        fs::rename(&staging_path, destination.path()).unwrap();
        assert_eq!(
            store.record_completed_copy(&mistaken, &old),
            Err(StorageError::ReferenceConflict)
        );
        assert_eq!(store.recover_copy(mistaken_key), Ok(CopyRecovery::Conflict));
        assert_eq!(
            fs::read_to_string(destination.path()).unwrap(),
            "old contents"
        );

        let actual = store
            .record_copy_intent(key("second", "existing"), &source, &destination)
            .unwrap();
        fs::rename(destination.path(), fixture.0.join("old.txt")).unwrap();
        let published = fixture.publish(&actual);
        let id = store.record_completed_copy(&actual, &published).unwrap();
        assert_eq!(note(&store, &id), Some(("draft".into(), 1)));
    }

    #[test]
    fn failed_unpublished_intent_releases_the_destination() {
        let fixture = Fixture::new();
        let mut store = fixture.open();
        let source = fixture.source(&mut store);
        let operation = key("batch", "failed");
        let destination = fixture.locator("never-published.txt");
        let intent = store
            .record_copy_intent(operation, &source, &destination)
            .unwrap();
        store.fail_copy(&intent).unwrap();
        assert_eq!(store.recover_copy(operation), Ok(CopyRecovery::Failed));
        let new = store
            .record_copy_intent(key("batch", "retry"), &source, &destination)
            .unwrap();
        let witness = fixture.publish(&new);
        let id = store.record_completed_copy(&new, &witness).unwrap();
        assert_eq!(note(&store, &id), Some(("draft".into(), 1)));
    }

    #[test]
    fn migration_preserves_legacy_completed_copy_without_inventing_a_policy() {
        let fixture = Fixture::new();
        let database = fixture.0.join("user.sqlite");
        create_private_file(&database).unwrap();
        let mut old = Store {
            connection: migrate(&database, &MIGRATIONS[..3]).unwrap(),
            derived_dir: fixture.0.join("derived"),
        };
        let source = fixture.source(&mut old);
        let destination = fixture.locator("legacy-target.txt");
        fs::copy(fixture.0.join("source.txt"), destination.path()).unwrap();
        old.insert_live_reference("destination", open(destination.path()))
            .unwrap();
        old.connection.execute(
            "INSERT INTO operation_items (operation_id,item_key,item_id,kind,source,destination,source_identity,state)
             VALUES ('legacy','completed',?1,'copy',?2,?3,?4,'completed')",
            params!["destination", source.previous.encode(), destination.encode(), source.key],
        ).unwrap();
        drop(old);
        let mut store = fixture.open();
        assert_eq!(
            store.recover_copy(key("legacy", "completed")),
            Ok(CopyRecovery::Completed("destination".into()))
        );
        assert_eq!(note(&store, "source"), Some(("draft".into(), 7)));
        assert!(
            store
                .record_copy_intent(
                    key("legacy", "new"),
                    &source,
                    &fixture.locator("another.txt")
                )
                .is_err()
        );
    }
}
