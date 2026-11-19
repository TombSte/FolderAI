use super::{StorageError, Store, TrackedReference};
use crate::local_identity::{LiveObservation, Locator};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveKey<'a> {
    pub operation_id: &'a str,
    pub item_key: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveRecovery {
    Completed,
    AwaitingEvidence,
    Conflict,
}

struct MoveRecord {
    item: String,
    source: Vec<u8>,
    destination: Option<Vec<u8>>,
    identity: Option<Vec<u8>>,
    state: String,
}

fn canonical_destination(destination: &Locator) -> Result<Locator, StorageError> {
    let path = destination.path();
    let leaf = path.file_name().ok_or(StorageError::ReferenceConflict)?;
    let parent = path.parent().ok_or(StorageError::ReferenceConflict)?;
    Locator::new(crate::local_identity::canonical_path(parent)?.join(leaf))
        .ok_or(StorageError::ReferenceConflict)
}

fn insert_move_intent(
    connection: &Connection,
    key: MoveKey<'_>,
    tracked: &TrackedReference,
    destination: &Locator,
) -> Result<(), StorageError> {
    if destination == &tracked.previous
        || tracked.identity.observe()? != LiveObservation::Present(tracked.previous.clone())
    {
        return Err(StorageError::ReferenceConflict);
    }
    let count = connection.execute(
        "INSERT INTO operation_items
         (operation_id, item_key, item_id, kind, source, destination, source_identity, state)
         SELECT ?1, ?2, id, 'move', locator, ?3, native_key, 'planned'
         FROM items WHERE id=?4 AND locator=?5 AND native_key=?6 AND availability='available'
         ON CONFLICT DO NOTHING",
        params![
            key.operation_id,
            key.item_key,
            destination.encode(),
            tracked.id,
            tracked.previous.encode(),
            tracked.key
        ],
    )?;
    if count == 1 {
        Ok(())
    } else {
        Err(StorageError::ReferenceConflict)
    }
}

impl Store {
    pub(crate) fn tracked_descendants(
        &mut self,
        root: &Locator,
    ) -> Result<Vec<(String, TrackedReference)>, StorageError> {
        let rows = self.connection.prepare(
            "SELECT id, locator FROM items WHERE native_key IS NOT NULL AND availability!='missing'"
        )?.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        let mut references = Vec::new();
        for (id, bytes) in rows {
            let Some(locator) = Locator::decode(bytes) else {
                continue;
            };
            if &locator != root
                && locator.path().starts_with(root.path())
                && let Ok(reference) = self.attach_live_reference(&id, &locator)
            {
                references.push((id, reference));
            }
        }
        Ok(references)
    }

    pub(crate) fn fail_unpublished_move<'a>(
        &mut self,
        operation_id: &str,
        references: impl IntoIterator<Item = &'a TrackedReference>,
    ) -> Result<(), StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        for tracked in references {
            if tracked.identity.observe()? != LiveObservation::Present(tracked.previous.clone()) {
                return Err(StorageError::ReferenceConflict);
            }
            let changed = transaction.execute(
                "UPDATE operation_items SET state='failed'
                 WHERE operation_id=?1 AND item_id=?2 AND source_identity=?3
                 AND kind='move' AND state='planned'",
                params![operation_id, tracked.id, tracked.key],
            )?;
            if changed != 1 {
                return Err(StorageError::ReferenceConflict);
            }
        }
        transaction.commit()?;
        Ok(())
    }

    /// Record intent before the caller invokes the native filesystem adapter.
    /// This records metadata for recovery and never performs a filesystem move.
    pub fn record_move(
        &mut self,
        key: MoveKey<'_>,
        tracked: &TrackedReference,
        destination: &Locator,
    ) -> Result<(), StorageError> {
        insert_move_intent(
            &self.connection,
            key,
            tracked,
            &canonical_destination(destination)?,
        )
    }

    /// Journal a directory move and all tracked descendants before the native
    /// operation begins. Each item can then be recovered independently.
    pub fn record_tree_move(
        &mut self,
        operation_id: &str,
        root: &TrackedReference,
        descendants: &[&TrackedReference],
        destination: &Locator,
    ) -> Result<(), StorageError> {
        let destination = canonical_destination(destination)?;
        if root.identity.kind()? != "directory"
            || destination.path().starts_with(root.previous.path())
        {
            return Err(StorageError::ReferenceConflict);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        for tracked in std::iter::once(root).chain(descendants.iter().copied()) {
            let mapped = tracked
                .previous
                .after_tree_move(&root.previous, &destination)
                .ok_or(StorageError::ReferenceConflict)?;
            insert_move_intent(
                &transaction,
                MoveKey {
                    operation_id,
                    item_key: &tracked.id,
                },
                tracked,
                &mapped,
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// A retry may consume the existing live handle. On restart the handle is
    /// gone, so a matching path alone cannot apply the move to user metadata.
    pub fn recover_move(
        &mut self,
        key: MoveKey<'_>,
        mut tracked: Option<&mut TrackedReference>,
    ) -> Result<MoveRecovery, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let record = transaction
            .query_row(
                "SELECT item_id, source, destination, source_identity, state FROM operation_items
             WHERE operation_id=?1 AND item_key=?2 AND kind='move'",
                params![key.operation_id, key.item_key],
                |row| {
                    Ok(MoveRecord {
                        item: row.get(0)?,
                        source: row.get(1)?,
                        destination: row.get(2)?,
                        identity: row.get(3)?,
                        state: row.get(4)?,
                    })
                },
            )
            .optional()?
            .ok_or(StorageError::ReferenceConflict)?;
        match record.state.as_str() {
            "completed" => return Ok(MoveRecovery::Completed),
            "conflict" | "failed" => return Ok(MoveRecovery::Conflict),
            _ => {}
        }

        let (result, resolved) = match tracked.as_ref() {
            None => (MoveRecovery::AwaitingEvidence, None),
            Some(tracked) => {
                if record.item != tracked.id
                    || record.identity.as_ref() != Some(&tracked.key)
                    || record.source != tracked.previous.encode()
                {
                    return Err(StorageError::ReferenceConflict);
                }
                let destination = record
                    .destination
                    .as_ref()
                    .ok_or(StorageError::ReferenceConflict)?;
                match tracked.identity.observe()? {
                    LiveObservation::Present(locator) if &locator.encode() == destination => {
                        let changed = transaction.execute(
                            "UPDATE items SET locator=?1, availability='available'
                             WHERE id=?2 AND locator=?3 AND native_key=?4 AND availability!='missing'",
                            params![destination, record.item, record.source, tracked.key],
                        )?;
                        if changed == 1 {
                            super::relocate_folder_view(
                                &transaction,
                                &record.item,
                                &tracked.previous,
                                &locator,
                            )?;
                            (MoveRecovery::Completed, Some(locator))
                        } else {
                            (MoveRecovery::Conflict, None)
                        }
                    }
                    LiveObservation::Present(locator) if locator == tracked.previous => {
                        (MoveRecovery::AwaitingEvidence, None)
                    }
                    LiveObservation::Unavailable => (MoveRecovery::AwaitingEvidence, None),
                    _ => (MoveRecovery::Conflict, None),
                }
            }
        };
        let state = match result {
            MoveRecovery::Completed => "completed",
            MoveRecovery::AwaitingEvidence => "recovery",
            MoveRecovery::Conflict => "conflict",
        };
        transaction.execute(
            "UPDATE operation_items SET state=?3 WHERE operation_id=?1 AND item_key=?2",
            params![key.operation_id, key.item_key, state],
        )?;
        transaction.commit()?;
        if let (Some(tracked), Some(resolved)) = (&mut tracked, resolved) {
            tracked.previous = resolved;
        }
        Ok(result)
    }
}
