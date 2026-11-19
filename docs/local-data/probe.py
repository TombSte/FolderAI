"""Run storage and native identity experiments against disposable fixtures."""

import json
import os
from pathlib import Path
import platform
import sqlite3
import tempfile
from contextlib import closing


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def open_db(path):
    connection = sqlite3.connect(path, isolation_level=None)
    connection.execute("PRAGMA foreign_keys = ON")
    connection.execute("PRAGMA synchronous = FULL")
    return closing(connection)


def test_storage(root):
    database = root / "user.sqlite"
    with open_db(database) as writer, open_db(database) as other:
        writer.executescript("""
            BEGIN;
            CREATE TABLE item (id INTEGER PRIMARY KEY);
            CREATE TABLE note (
                item_id INTEGER PRIMARY KEY REFERENCES item(id),
                body TEXT NOT NULL,
                revision INTEGER NOT NULL
            );
            INSERT INTO item VALUES (1), (2);
            INSERT INTO note VALUES (1, 'before migration', 1), (2, 'another item', 1);
            PRAGMA user_version = 1;
            COMMIT;
        """)
        with open_db(root / "pre-upgrade.sqlite") as backup:
            writer.backup(backup)
            require(backup.execute("PRAGMA integrity_check").fetchone() == ("ok",), "backup invalid")
            require(backup.execute("SELECT body FROM note WHERE item_id = 1").fetchone() ==
                    ("before migration",), "backup missed note")

        try:
            writer.executescript("""
                BEGIN IMMEDIATE;
                ALTER TABLE note ADD COLUMN edited_at INTEGER;
                PRAGMA user_version = 2;
                INSERT INTO note (item_id, body, revision) VALUES (100, 'invalid reference', 1);
                COMMIT;
            """)
        except sqlite3.IntegrityError:
            writer.execute("ROLLBACK")
        else:
            raise AssertionError("expected migration failure")
        require(other.execute("PRAGMA user_version").fetchone() == (1,), "version advanced after rollback")
        require(len(other.execute("PRAGMA table_info(note)").fetchall()) == 3, "schema changed after rollback")
        require(other.execute("SELECT body FROM note WHERE item_id = 1").fetchone() ==
                ("before migration",), "rollback lost note")

        writer.executescript("""
            BEGIN IMMEDIATE;
            ALTER TABLE note ADD COLUMN edited_at INTEGER;
            PRAGMA user_version = 2;
            COMMIT;
        """)
        require(other.execute("PRAGMA user_version").fetchone() == (2,), "migration did not commit")
        require(other.execute("SELECT body FROM note WHERE item_id = 1").fetchone() ==
                ("before migration",), "migration lost note")

        read_revision = "SELECT revision FROM note WHERE item_id = 1"
        first_revision = writer.execute(read_revision).fetchone()[0]
        stale_revision = other.execute(read_revision).fetchone()[0]
        update = "UPDATE note SET body = ?, revision = revision + 1 WHERE item_id = ? AND revision = ?"
        require(writer.execute(update, ("first edit", 1, first_revision)).rowcount == 1, "first edit failed")
        require(other.execute(update, ("stale edit", 1, stale_revision)).rowcount == 0, "stale edit overwrote note")

        cache = root / "derived.sqlite"
        with open_db(cache) as derived:
            derived.execute("CREATE TABLE cache (value TEXT)")
            derived.execute("INSERT INTO cache VALUES ('synthetic document fragment')")
        cache.unlink()
        with open_db(cache) as rebuilt:
            rebuilt.execute("CREATE TABLE cache (value TEXT)")
        require(writer.execute("SELECT item_id, body, revision FROM note ORDER BY item_id").fetchall() ==
                [(1, "first edit", 2), (2, "another item", 1)], "derived rebuild changed durable notes")

    return {"sqlite": sqlite3.sqlite_version, "rollback": True, "backup": True,
            "migration": True, "revision_conflict": True, "derived_rebuild": True}


def native_sample(path):
    info = path.lstat()
    return info.st_dev, info.st_ino


def test_identity(root):
    original = root / "original"
    moved = root / "renamed"
    alias = root / "hardlink"
    replacement = root / "replacement"
    original.write_text("original", encoding="utf-8")
    reference = native_sample(original)
    original.rename(moved)
    require(native_sample(moved) == reference, "native identifier changed on same-volume rename")

    os.link(moved, alias)
    require(native_sample(alias) == reference, "hard link has a different native identifier")
    replacement.write_text("replacement", encoding="utf-8")
    replacement.replace(moved)
    require(native_sample(moved) != reference, "same-path replacement retained the old identifier")
    require(native_sample(alias) == reference, "replacement changed the hard-linked object")

    folder = root / "folder"
    folder.mkdir()
    child = folder / "child"
    child.write_text("child", encoding="utf-8")
    child_reference = native_sample(child)
    renamed_folder = root / "renamed-folder"
    folder.rename(renamed_folder)
    require(native_sample(renamed_folder / "child") == child_reference, "moved child changed identifier")
    return {"rename_preserved": True, "hard_link_shared": True, "replacement_distinguished": True,
            "moved_child_preserved": True}


if __name__ == "__main__":
    with tempfile.TemporaryDirectory(prefix="folderai-local-data-") as directory:
        root = Path(directory)
        print(json.dumps({"host": platform.platform(), "storage": test_storage(root),
                          "identity": test_identity(root)}, indent=2))
