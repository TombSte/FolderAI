use super::{MoveKey, StorageError, Store, TrackedReference, relocate_folder_view};
use crate::local_identity::{LiveIdentity, LiveObservation, Locator, canonical_path};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::{collections::HashMap, fs, io, sync::OnceLock};
use uuid::Uuid;

/// An operation-specific authorization paired with the live source handle.
/// Durable path and native keys can report uncertainty after a restart.
pub struct CrossMoveIntent {
    operation: String,
    item: String,
    original_id: String,
    source: Locator,
    source_key: Vec<u8>,
    destination: Locator,
    kind: &'static str,
    nonce: String,
    bound: OnceLock<()>,
}

/// Retain this identity from staging through publication. A handle obtained
/// by reopening the intended destination would not prove what was published.
pub struct CrossMovePublication {
    nonce: String,
    staging: Locator,
    identity: LiveIdentity,
    key: Vec<u8>,
}

pub struct CrossMoveRemoval {
    nonce: String,
    original_id: String,
    source_key: Vec<u8>,
    source: Locator,
    destination: Locator,
    destination_key: Vec<u8>,
    quarantine: Option<Locator>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossMoveSource {
    Original,
    Quarantined,
    Removed,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CrossMoveRecovery {
    AwaitingEvidence,
    DestinationPublished {
        destination: Locator,
        source_quarantine: Option<Locator>,
    },
    Completed,
    Failed,
}

impl CrossMoveIntent {
    pub fn key(&self) -> MoveKey<'_> {
        MoveKey {
            operation_id: &self.operation,
            item_key: &self.item,
        }
    }

    pub fn source(&self) -> &Locator {
        &self.source
    }

    pub fn destination(&self) -> &Locator {
        &self.destination
    }

    pub fn bind_staged_destination(
        &self,
        staged: LiveIdentity,
    ) -> Result<CrossMovePublication, StorageError> {
        let LiveObservation::Present(staging) = staged.observe()? else {
            return Err(StorageError::ReferenceConflict);
        };
        let key = staged.comparison_key()?;
        if staging == self.destination
            || staging == self.source
            || staged.kind()? != self.kind
            || key == self.source_key
        {
            return Err(StorageError::ReferenceConflict);
        }
        self.bound
            .set(())
            .map_err(|_| StorageError::ReferenceConflict)?;
        Ok(CrossMovePublication {
            nonce: self.nonce.clone(),
            staging,
            identity: staged,
            key,
        })
    }
}

impl CrossMoveRemoval {
    /// Check again at the native boundary. This is a live-evidence check,
    /// so the filesystem adapter still needs anchored, guarded removal.
    pub fn verify_before_removal(
        &self,
        source: &TrackedReference,
        publication: &CrossMovePublication,
    ) -> Result<CrossMoveSource, StorageError> {
        if self.nonce != publication.nonce
            || self.original_id != source.id
            || self.source != source.previous
            || self.source_key != source.key
            || self.source_key != source.identity.comparison_key()?
            || self.destination_key != publication.key
        {
            return Err(StorageError::ReferenceConflict);
        }
        publication.verify_at(&self.destination)?;
        match source.identity.observe()? {
            LiveObservation::Present(locator) if locator == self.source => {
                Ok(CrossMoveSource::Original)
            }
            LiveObservation::Present(locator) if Some(&locator) == self.quarantine.as_ref() => {
                Ok(CrossMoveSource::Quarantined)
            }
            LiveObservation::Removed => Ok(CrossMoveSource::Removed),
            _ => Err(StorageError::ReferenceConflict),
        }
    }
}

impl CrossMovePublication {
    fn verify_at(&self, locator: &Locator) -> Result<(), StorageError> {
        if self.identity.comparison_key()? != self.key
            || self.identity.observe()? != LiveObservation::Present(locator.clone())
        {
            Err(StorageError::ReferenceConflict)
        } else {
            Ok(())
        }
    }
}

struct JournalRow {
    state: String,
    published_identity: Option<Vec<u8>>,
    staging: Option<Vec<u8>>,
    quarantine: Option<Vec<u8>>,
}

fn load_intent(
    database: &Connection,
    intent: &CrossMoveIntent,
) -> Result<JournalRow, StorageError> {
    database
        .query_row(
            "SELECT state, destination_identity, cross_move_staging, cross_move_quarantine
             FROM operation_items
             WHERE operation_id=?1 AND item_key=?2 AND kind='move' AND cross_move_nonce=?3
               AND item_id=?4 AND source=?5 AND source_identity=?6 AND destination=?7",
            params![
                intent.operation,
                intent.item,
                intent.nonce,
                intent.original_id,
                intent.source.encode(),
                intent.source_key,
                intent.destination.encode(),
            ],
            |row| {
                Ok(JournalRow {
                    state: row.get(0)?,
                    published_identity: row.get(1)?,
                    staging: row.get(2)?,
                    quarantine: row.get(3)?,
                })
            },
        )
        .optional()?
        .ok_or(StorageError::ReferenceConflict)
}

fn validate_key(key: &str) -> Result<(), StorageError> {
    if key.is_empty() || key.len() > 128 || key.chars().any(char::is_control) {
        Err(StorageError::ReferenceConflict)
    } else {
        Ok(())
    }
}

impl Store {
    /// Reserve a cross-volume move and every tracked descendant atomically.
    /// These rows share reservations with copies and ordinary native moves.
    pub fn record_cross_volume_move(
        &mut self,
        operation_id: &str,
        item_prefix: &str,
        root: &TrackedReference,
        descendants: &[&TrackedReference],
        destination: &Locator,
    ) -> Result<Vec<CrossMoveIntent>, StorageError> {
        validate_key(operation_id)?;
        validate_key(item_prefix)?;
        let path = destination.path();
        let leaf = path.file_name().ok_or(StorageError::ReferenceConflict)?;
        let parent = path.parent().ok_or(StorageError::ReferenceConflict)?;
        let destination = Locator::new(canonical_path(parent)?.join(leaf))
            .ok_or(StorageError::ReferenceConflict)?;
        if destination.path().starts_with(root.previous.path())
            || (!descendants.is_empty() && !root.is_directory()?)
        {
            return Err(StorageError::ReferenceConflict);
        }
        match fs::symlink_metadata(destination.path()) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Ok(_) => return Err(StorageError::ReferenceConflict),
            Err(error) => return Err(error.into()),
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut intents = Vec::with_capacity(1 + descendants.len());
        for (index, tracked) in std::iter::once(root)
            .chain(descendants.iter().copied())
            .enumerate()
        {
            let item = format!("{item_prefix}.{index}");
            validate_key(&item)?;
            let mapped = tracked
                .previous
                .after_tree_move(&root.previous, &destination)
                .ok_or(StorageError::ReferenceConflict)?;
            tracked.verify_source()?;
            if tracked.identity.comparison_key()? != tracked.key {
                return Err(StorageError::ReferenceConflict);
            }
            let intent = CrossMoveIntent {
                operation: operation_id.to_owned(),
                item,
                original_id: tracked.id.clone(),
                source: tracked.previous.clone(),
                source_key: tracked.key.clone(),
                destination: mapped,
                kind: tracked.identity.kind()?,
                nonce: Uuid::new_v4().to_string(),
                bound: OnceLock::new(),
            };
            let reserved = transaction.execute(
                "INSERT INTO operation_items
                 (operation_id, item_key, item_id, kind, source, destination, source_identity,
                    state, cross_move_nonce)
                 SELECT ?1, ?2, id, 'move', locator, ?3, native_key, 'planned', ?4
                 FROM items WHERE id=?5 AND locator=?6 AND native_key=?7
                    AND availability='available'
                 ON CONFLICT DO NOTHING",
                params![
                    intent.operation,
                    intent.item,
                    intent.destination.encode(),
                    intent.nonce,
                    intent.original_id,
                    intent.source.encode(),
                    intent.source_key,
                ],
            )?;
            if reserved != 1 {
                return Err(StorageError::ReferenceConflict);
            }
            intents.push(intent);
        }
        transaction.commit()?;
        Ok(intents)
    }

    /// The default state after publication is source retained. Record the
    /// intended quarantine before issuing authority to relocate or remove it.
    pub fn record_cross_volume_publication(
        &mut self,
        intent: &CrossMoveIntent,
        source: &TrackedReference,
        publication: &CrossMovePublication,
        quarantine: Option<&Locator>,
    ) -> Result<CrossMoveRemoval, StorageError> {
        if quarantine.is_some_and(|path| {
            path.path().starts_with(intent.source.path())
                || path.path().starts_with(intent.destination.path())
                || path == &publication.staging
        }) {
            return Err(StorageError::ReferenceConflict);
        }
        let removal = CrossMoveRemoval {
            nonce: intent.nonce.clone(),
            original_id: intent.original_id.clone(),
            source_key: intent.source_key.clone(),
            source: intent.source.clone(),
            destination: intent.destination.clone(),
            destination_key: publication.key.clone(),
            quarantine: quarantine.cloned(),
        };
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row = load_intent(&transaction, intent)?;
        #[cfg(target_os = "macos")]
        source.identity.arm_directory_removal()?;
        let location = removal.verify_before_removal(source, publication)?;
        let staging = publication.staging.encode();
        let quarantine = quarantine.map(Locator::encode);
        match row.state.as_str() {
            "planned" if location == CrossMoveSource::Original => {
                transaction.execute(
                    "UPDATE operation_items SET state='recovery', destination_identity=?3,
                     cross_move_staging=?4, cross_move_quarantine=?5
                     WHERE operation_id=?1 AND item_key=?2",
                    params![
                        intent.operation,
                        intent.item,
                        publication.key,
                        staging,
                        quarantine
                    ],
                )?;
            }
            "recovery" | "completed"
                if row.published_identity.as_ref() == Some(&publication.key)
                    && row.staging.as_ref() == Some(&staging)
                    && row.quarantine == quarantine => {}
            _ => return Err(StorageError::ReferenceConflict),
        }
        transaction.commit()?;
        Ok(removal)
    }

    /// Publish the new identity on the existing application item only after
    /// this specific original handle reports that its object has been removed.
    pub fn complete_cross_volume_move(
        &mut self,
        intent: &CrossMoveIntent,
        source: &TrackedReference,
        publication: &CrossMovePublication,
        removal: &CrossMoveRemoval,
    ) -> Result<(), StorageError> {
        if intent.nonce != removal.nonce {
            return Err(StorageError::ReferenceConflict);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row = load_intent(&transaction, intent)?;
        if !matches!(row.state.as_str(), "recovery" | "completed")
            || row.published_identity.as_ref() != Some(&publication.key)
            || row.staging.as_ref() != Some(&publication.staging.encode())
            || row.quarantine != removal.quarantine.as_ref().map(Locator::encode)
            || removal.verify_before_removal(source, publication)? != CrossMoveSource::Removed
        {
            return Err(StorageError::ReferenceConflict);
        }
        if row.state == "completed" {
            return Ok(());
        }
        let updated = transaction.execute(
            "UPDATE items SET locator=?1, native_key=?2, availability='available'
             WHERE id=?3 AND locator=?4 AND native_key=?5 AND availability!='missing'
             AND NOT EXISTS (
                 SELECT 1 FROM items AS other WHERE other.id!=items.id
                   AND other.native_key=?2 AND other.availability!='missing'
             )",
            params![
                intent.destination.encode(),
                publication.key,
                intent.original_id,
                intent.source.encode(),
                intent.source_key,
            ],
        )?;
        if updated != 1 {
            return Err(StorageError::ReferenceConflict);
        }
        relocate_folder_view(
            &transaction,
            &intent.original_id,
            &intent.source,
            &intent.destination,
        )?;
        transaction.execute(
            "UPDATE operation_items SET state='completed' WHERE operation_id=?1 AND item_key=?2",
            params![intent.operation, intent.item],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Close an attempt only before any publication was recorded. Bound
    /// entries additionally need their original staging handles at the
    /// expected locations. Call before removing owned staging.
    pub fn fail_unpublished_cross_volume_move(
        &mut self,
        intents: &[CrossMoveIntent],
        publications: &[CrossMovePublication],
    ) -> Result<(), StorageError> {
        let mut bound: HashMap<_, _> = publications
            .iter()
            .map(|publication| (publication.nonce.as_str(), publication))
            .collect();
        if bound.len() != publications.len() {
            return Err(StorageError::ReferenceConflict);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        for intent in intents {
            match (intent.bound.get(), bound.remove(intent.nonce.as_str())) {
                (Some(()), Some(publication)) => publication.verify_at(&publication.staging)?,
                (None, None) => {}
                _ => return Err(StorageError::ReferenceConflict),
            }
            if !matches!(
                load_intent(&transaction, intent)?.state.as_str(),
                "planned" | "failed"
            ) {
                return Err(StorageError::ReferenceConflict);
            }
            transaction.execute(
                "UPDATE operation_items SET state='failed'
                 WHERE operation_id=?1 AND item_key=?2",
                params![intent.operation, intent.item],
            )?;
        }
        if !bound.is_empty() {
            return Err(StorageError::ReferenceConflict);
        }
        transaction.commit()?;
        Ok(())
    }

    /// Durable status only. A reopened database cannot recreate discard or
    /// removal authority from a stored destination key.
    pub fn recover_cross_volume_move(
        &self,
        key: MoveKey<'_>,
    ) -> Result<CrossMoveRecovery, StorageError> {
        let (state, destination, quarantine): (String, Vec<u8>, Option<Vec<u8>>) = self
            .connection
            .query_row(
                "SELECT state, destination, cross_move_quarantine FROM operation_items
                 WHERE operation_id=?1 AND item_key=?2 AND kind='move'
                   AND cross_move_nonce IS NOT NULL",
                params![key.operation_id, key.item_key],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
            .ok_or(StorageError::ReferenceConflict)?;
        match state.as_str() {
            "planned" => Ok(CrossMoveRecovery::AwaitingEvidence),
            "recovery" => Ok(CrossMoveRecovery::DestinationPublished {
                destination: Locator::decode(destination).ok_or(StorageError::ReferenceConflict)?,
                source_quarantine: quarantine
                    .map(|bytes| Locator::decode(bytes).ok_or(StorageError::ReferenceConflict))
                    .transpose()?,
            }),
            "completed" => Ok(CrossMoveRecovery::Completed),
            "failed" => Ok(CrossMoveRecovery::Failed),
            _ => Err(StorageError::ReferenceConflict),
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use crate::storage::MoveRecovery;
    use std::path::{Path, PathBuf};

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("folderai-cross-move-{}", Uuid::new_v4()));
            fs::create_dir(&root).unwrap();
            Self(canonical_path(&root).unwrap())
        }

        fn store(&self) -> Store {
            Store::open(&self.0.join("data"), &self.0.join("cache")).unwrap()
        }

        fn locator(&self, name: &str) -> Locator {
            Locator::new(self.0.join(name)).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn track(store: &mut Store, id: &str, source: &Locator) -> TrackedReference {
        store
            .insert_live_reference(id, LiveIdentity::open(source).unwrap())
            .unwrap()
    }

    fn write(path: &Locator, content: &[u8]) {
        fs::write(path.path(), content).unwrap();
    }

    fn identity(path: &Locator) -> Vec<u8> {
        LiveIdentity::open(path).unwrap().comparison_key().unwrap()
    }

    fn item_location(store: &Store, id: &str) -> (Locator, Vec<u8>) {
        let (bytes, key): (Vec<u8>, Vec<u8>) = store
            .connection
            .query_row(
                "SELECT locator, native_key FROM items WHERE id=?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        (Locator::decode(bytes).unwrap(), key)
    }

    fn rename(source: &Locator, target: &Locator) {
        fs::rename(source.path(), target.path()).unwrap();
    }

    fn seal(intent: &CrossMoveIntent, stage: &Locator) -> CrossMovePublication {
        intent
            .bind_staged_destination(LiveIdentity::open(stage).unwrap())
            .unwrap()
    }

    #[test]
    fn source_retained_preserves_ids_and_tracks_quarantine_until_each_entry_is_removed() {
        let f = Fixture::new();
        let mut store = f.store();
        let source = f.locator("source");
        let source_child = f.locator("source/item");
        let stage = f.locator("stage");
        let stage_child = f.locator("stage/item");
        let target = f.locator("destination");
        let target_child = f.locator("destination/item");
        let quarantine = f.locator("quarantine");
        let quarantine_child = f.locator("quarantine/item");
        for path in [&source, &stage] {
            fs::create_dir(path.path()).unwrap();
        }
        for path in [&source_child, &stage_child] {
            write(path, b"data");
        }
        let mut root = track(&mut store, "root", &source);
        let child = track(&mut store, "child", &source_child);
        let original_root_key = identity(&source);
        let original_child_key = identity(&source_child);
        store
            .connection
            .execute_batch(
                "INSERT INTO notes VALUES ('child', 'preserved note', 7);
             INSERT INTO tags VALUES ('tag', 'label', NULL);
             INSERT INTO item_tags VALUES ('child', 'tag');
             INSERT INTO favorites VALUES ('fav', 'child', NULL, 0);",
            )
            .unwrap();
        let original_view = format!(
            "{}{}",
            super::super::FOLDER_VIEW_PREFIX,
            source.path().display()
        );
        store
            .connection
            .execute(
                "INSERT INTO preferences (key, value) VALUES (?1, 'true')",
                [&original_view],
            )
            .unwrap();
        let intents = store
            .record_cross_volume_move("operation", "first", &root, &[&child], &target)
            .unwrap();
        assert_eq!(intents[0].source(), &source);
        assert_eq!(intents[1].destination(), &target_child);
        assert_eq!(
            store.recover_cross_volume_move(intents[0].key()),
            Ok(CrossMoveRecovery::AwaitingEvidence)
        );
        assert!(matches!(
            store.recover_move(intents[0].key(), Some(&mut root)),
            Err(StorageError::ReferenceConflict)
        ));
        let publications = [seal(&intents[0], &stage), seal(&intents[1], &stage_child)];
        assert!(
            store
                .record_cross_volume_publication(
                    &intents[0],
                    &root,
                    &publications[0],
                    Some(&quarantine)
                )
                .is_err()
        );
        rename(&stage, &target);
        assert!(store.resolve_live_item(&target, true).is_err());
        let removals: Vec<_> = [(&root, &quarantine), (&child, &quarantine_child)]
            .into_iter()
            .enumerate()
            .map(|(index, (reference, quarantine))| {
                store
                    .record_cross_volume_publication(
                        &intents[index],
                        reference,
                        &publications[index],
                        Some(quarantine),
                    )
                    .unwrap()
            })
            .collect();

        assert_eq!(
            store.recover_cross_volume_move(intents[0].key()),
            Ok(CrossMoveRecovery::DestinationPublished {
                destination: target.clone(),
                source_quarantine: Some(quarantine.clone()),
            })
        );
        assert_eq!(
            removals[0].verify_before_removal(&root, &publications[0]),
            Ok(CrossMoveSource::Original)
        );
        assert!(
            store
                .complete_cross_volume_move(&intents[0], &root, &publications[0], &removals[0])
                .is_err()
        );
        assert_eq!(
            item_location(&store, "root"),
            (source.clone(), original_root_key)
        );
        assert_eq!(
            item_location(&store, "child"),
            (source_child, original_child_key)
        );
        assert!(
            store
                .fail_unpublished_cross_volume_move(&intents, &publications)
                .is_err()
        );

        rename(&source, &quarantine);
        assert!(store.reconcile_live_reference(&mut root).is_err());
        assert_eq!(
            removals[0].verify_before_removal(&root, &publications[0]),
            Ok(CrossMoveSource::Quarantined)
        );
        assert_eq!(
            removals[1].verify_before_removal(&child, &publications[1]),
            Ok(CrossMoveSource::Quarantined)
        );
        fs::remove_file(quarantine_child.path()).unwrap();
        store
            .complete_cross_volume_move(&intents[1], &child, &publications[1], &removals[1])
            .unwrap();
        assert_eq!(
            store.recover_cross_volume_move(intents[1].key()),
            Ok(CrossMoveRecovery::Completed)
        );
        assert_eq!(
            item_location(&store, "child"),
            (target_child.clone(), identity(&target_child))
        );
        assert_eq!(
            removals[0].verify_before_removal(&root, &publications[0]),
            Ok(CrossMoveSource::Quarantined)
        );
        fs::remove_dir(quarantine.path()).unwrap();
        for _ in 0..2 {
            store
                .complete_cross_volume_move(&intents[0], &root, &publications[0], &removals[0])
                .unwrap();
        }
        assert_eq!(
            store.recover_cross_volume_move(intents[0].key()),
            Ok(CrossMoveRecovery::Completed)
        );
        assert_eq!(
            item_location(&store, "root"),
            (target.clone(), identity(&target))
        );
        assert_eq!(
            store
                .resolve_live_item(&target_child, true)
                .unwrap()
                .as_deref(),
            Some("child")
        );
        assert_eq!(
            store
                .connection
                .query_row(
                    "SELECT n.body, n.revision FROM notes n
                 JOIN item_tags t USING(item_id) JOIN favorites f USING(item_id)
                 WHERE n.item_id='child' AND t.tag_id='tag' AND f.id='fav'",
                    [],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?)),
                )
                .unwrap(),
            ("preserved note".into(), 7)
        );
        let moved_view = format!(
            "{}{}",
            super::super::FOLDER_VIEW_PREFIX,
            target.path().display()
        );
        assert_eq!(
            store
                .connection
                .query_row(
                    "SELECT key FROM preferences WHERE key IN (?1, ?2)",
                    [original_view, moved_view.clone()],
                    |row| row.get::<_, String>(0),
                )
                .unwrap(),
            moved_view
        );
    }

    #[test]
    fn survives_reopened_db_and_failed_commit_without_rebinding_from_names() {
        let f = Fixture::new();
        let mut store = f.store();
        let source = f.locator("source");
        let stage = f.locator("stage");
        let target = f.locator("destination");
        write(&source, b"data");
        write(&stage, b"data");
        let tracked = track(&mut store, "item", &source);
        let original = item_location(&store, "item");
        let intents = store
            .record_cross_volume_move("operation", "file", &tracked, &[], &target)
            .unwrap();
        let intent = &intents[0];
        let publication = seal(intent, &stage);
        rename(&stage, &target);
        let removal = store
            .record_cross_volume_publication(intent, &tracked, &publication, None)
            .unwrap();
        fs::remove_file(source.path()).unwrap();
        drop(store);
        let mut reopened = f.store();
        assert!(matches!(
            reopened.recover_cross_volume_move(intent.key()),
            Ok(CrossMoveRecovery::DestinationPublished { .. })
        ));
        assert_eq!(item_location(&reopened, "item"), original);
        reopened.connection.execute_batch(
            "CREATE TEMP TABLE fault_parent (id INTEGER PRIMARY KEY);
             CREATE TEMP TABLE fault_child (parent INTEGER REFERENCES fault_parent(id) DEFERRABLE INITIALLY DEFERRED);
             CREATE TEMP TRIGGER fault_cross_completion AFTER UPDATE OF state ON main.operation_items
             WHEN NEW.state='completed' BEGIN INSERT INTO fault_child VALUES (42); END;",
        ).unwrap();
        assert!(
            reopened
                .complete_cross_volume_move(intent, &tracked, &publication, &removal)
                .is_err()
        );
        assert_eq!(item_location(&reopened, "item"), original);
        reopened
            .connection
            .execute_batch("DROP TRIGGER fault_cross_completion")
            .unwrap();

        let moved = f.locator("moved-output");
        rename(&target, &moved);
        write(&target, b"replacement");
        assert!(
            reopened
                .complete_cross_volume_move(intent, &tracked, &publication, &removal)
                .is_err()
        );
        assert_eq!(item_location(&reopened, "item"), original);
        fs::remove_file(target.path()).unwrap();
        rename(&moved, &target);
        reopened
            .complete_cross_volume_move(intent, &tracked, &publication, &removal)
            .unwrap();
        assert_eq!(
            item_location(&reopened, "item"),
            (target.clone(), identity(&target))
        );
        assert_eq!(
            reopened.recover_cross_volume_move(intent.key()),
            Ok(CrossMoveRecovery::Completed)
        );
    }

    #[test]
    fn unproven_destination_and_wrong_quarantine_never_grant_a_removal() {
        let f = Fixture::new();
        let mut store = f.store();
        let source = f.locator("source");
        let stage = f.locator("stage");
        let target = f.locator("destination");
        let quarantine = f.locator("quarantine");
        let other = f.locator("other");
        write(&source, b"data");
        write(&stage, b"data");
        let tracked = track(&mut store, "item", &source);
        let intents = store
            .record_cross_volume_move("op", "file", &tracked, &[], &target)
            .unwrap();
        let intent = &intents[0];
        let publication = seal(intent, &stage);
        write(&target, b"foreign");
        assert!(
            store
                .record_cross_volume_publication(intent, &tracked, &publication, Some(&quarantine))
                .is_err()
        );
        assert!(
            store
                .fail_unpublished_cross_volume_move(&intents, &[])
                .is_err()
        );
        fs::remove_file(target.path()).unwrap();
        rename(&stage, &target);
        let removal = store
            .record_cross_volume_publication(intent, &tracked, &publication, Some(&quarantine))
            .unwrap();
        assert!(
            store
                .record_cross_volume_publication(intent, &tracked, &publication, Some(&other))
                .is_err()
        );
        rename(&source, &other);
        assert!(
            removal
                .verify_before_removal(&tracked, &publication)
                .is_err()
        );
        assert!(
            store
                .complete_cross_volume_move(intent, &tracked, &publication, &removal)
                .is_err()
        );
        assert!(Path::exists(other.path()));
    }

    #[test]
    fn late_collision_can_abandon_the_same_sealed_staging_and_reuse_the_source() {
        let f = Fixture::new();
        let mut store = f.store();
        let source = f.locator("source");
        let stage = f.locator("stage");
        let target = f.locator("destination");
        let kept_both = f.locator("destination - copy");
        write(&source, b"data");
        write(&stage, b"data");
        let tracked = track(&mut store, "item", &source);
        let intents = store
            .record_cross_volume_move("op", "file", &tracked, &[], &target)
            .unwrap();
        let publication = seal(&intents[0], &stage);
        assert!(!matches!(
            store.recover_move(intents[0].key(), None),
            Ok(MoveRecovery::AwaitingEvidence)
        ));
        write(&target, b"foreign");
        store
            .fail_unpublished_cross_volume_move(&intents, &[publication])
            .unwrap();
        assert_eq!(
            store.recover_cross_volume_move(intents[0].key()),
            Ok(CrossMoveRecovery::Failed)
        );
        let next = store
            .record_cross_volume_move("op", "retry", &tracked, &[], &kept_both)
            .unwrap();
        assert_eq!(next[0].destination(), &kept_both);
        assert_eq!(fs::read(target.path()).unwrap(), b"foreign");
        assert_eq!(fs::read(source.path()).unwrap(), b"data");
    }
}
