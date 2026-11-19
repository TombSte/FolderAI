CREATE TABLE operation_items (
    operation_id TEXT NOT NULL,
    item_key TEXT NOT NULL,
    item_id TEXT REFERENCES items(id),
    kind TEXT NOT NULL CHECK (kind IN ('move', 'copy', 'trash')),
    source BLOB NOT NULL,
    destination BLOB,
    source_identity BLOB,
    state TEXT NOT NULL CHECK (state IN ('planned', 'completed', 'recovery', 'conflict', 'failed')),
    PRIMARY KEY (operation_id, item_key)
) STRICT;
