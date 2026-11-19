CREATE TABLE items (
    id TEXT PRIMARY KEY NOT NULL,
    locator BLOB NOT NULL,
    native_key BLOB,
    kind TEXT NOT NULL CHECK (kind IN ('directory', 'file', 'link', 'other')),
    availability TEXT NOT NULL CHECK (availability IN ('available', 'unavailable', 'missing'))
) STRICT;

CREATE TABLE notes (
    item_id TEXT PRIMARY KEY NOT NULL REFERENCES items(id),
    body TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision > 0)
) STRICT;

CREATE TABLE tags (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT UNIQUE NOT NULL,
    color TEXT
) STRICT;

CREATE TABLE item_tags (
    item_id TEXT NOT NULL REFERENCES items(id),
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (item_id, tag_id)
) STRICT;

CREATE TABLE favorite_groups (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    position INTEGER NOT NULL CHECK (position >= 0)
) STRICT;

CREATE TABLE favorites (
    id TEXT PRIMARY KEY NOT NULL,
    item_id TEXT NOT NULL REFERENCES items(id),
    group_id TEXT REFERENCES favorite_groups(id),
    position INTEGER NOT NULL CHECK (position >= 0)
) STRICT;

CREATE INDEX favorites_by_item ON favorites(item_id);
CREATE INDEX favorites_by_group ON favorites(group_id, position);

CREATE TABLE folder_views (
    item_id TEXT PRIMARY KEY NOT NULL REFERENCES items(id),
    value TEXT NOT NULL CHECK (json_valid(value))
) STRICT;

CREATE TABLE preferences (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL CHECK (json_valid(value))
) STRICT;
