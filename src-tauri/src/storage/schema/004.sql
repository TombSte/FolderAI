CREATE INDEX items_by_live_native_key ON items(native_key, locator) WHERE availability != 'missing';

CREATE TABLE copy_operations (
    operation_id TEXT PRIMARY KEY NOT NULL,
    copy_metadata INTEGER NOT NULL CHECK (copy_metadata IN (0, 1))
) STRICT;

ALTER TABLE operation_items ADD COLUMN copy_source_item_id TEXT REFERENCES items(id);
ALTER TABLE operation_items ADD COLUMN copy_intent_nonce TEXT;
ALTER TABLE operation_items ADD COLUMN prior_destination_identity BLOB;
ALTER TABLE operation_items ADD COLUMN destination_identity BLOB;

CREATE UNIQUE INDEX unresolved_copy_by_destination ON operation_items(destination)
WHERE kind = 'copy' AND state IN ('planned', 'recovery');
