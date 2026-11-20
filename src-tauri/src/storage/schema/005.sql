ALTER TABLE operation_items ADD COLUMN cross_move_nonce TEXT;
ALTER TABLE operation_items ADD COLUMN cross_move_staging BLOB;
ALTER TABLE operation_items ADD COLUMN cross_move_quarantine BLOB;

DROP INDEX unresolved_copy_by_destination;
CREATE UNIQUE INDEX unresolved_transfer_by_destination ON operation_items(destination)
WHERE state IN ('planned', 'recovery')
  AND (kind = 'copy' OR (kind = 'move' AND cross_move_nonce IS NOT NULL));
