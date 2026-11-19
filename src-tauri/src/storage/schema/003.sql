CREATE UNIQUE INDEX unresolved_move_by_item ON operation_items(item_id)
WHERE kind = 'move' AND state IN ('planned', 'recovery', 'conflict');
