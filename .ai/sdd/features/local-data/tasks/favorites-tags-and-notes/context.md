# Task: favorites-tags-and-notes

## Type
implementation

## Description
Implement favorites groups, tag CRUD and multi-selection, revision-checked Markdown notes, per-folder metadata and configurable copy semantics.

Traceability covers RF-007 RF-013–014 RF-029–034 RT-013–015.

## Depends On
- `local-data/identity-and-reconciliation`
- `workspace-navigation/workspace-controls-and-settings`

## Likely Files
- React organization, Rust metadata services and sanitized renderer

## Progress
- [x] Connect SQLite favorites, groups, tags and revision-checked notes to the workspace.
- [x] Add tag and note columns, folder view identity checks, safe Markdown preview, and the unsaved-draft close guard.
- [x] Implement the copy metadata journal and preference for the file-transfer adapter.

## Validation
- 2026-11-19 macOS 26.5.1/APFS. `cargo test --offline --locked --manifest-path src-tauri/Cargo.toml` passed 44 tests. Strict Clippy, rustfmt, `npm run build`, `git diff --check`, and `python3 docs/local-data/probe.py` passed.
- Rust covers independent favorite links, atomic tag assignment, revision conflicts, retained APFS rename and removal, a stale folder view save against a replacement, and copy policy and recovery. The schema 3 to 4 migration preserves copy history. The copy tests publish staged filesystem entries and cover failed commit, retry after reopening with the same witness, independent notes, and replacement before retry.
- Native macOS WebKit with an isolated SQLite profile verified folder settings over IPC, favorites/tags and notes across restart, blocking unsafe Markdown content, a pending note flushed on native window close, and an intercepted native close with a retained conflicting draft. The Brave fixture at 1120 × 740 and 620 × 380 verified group creation, drag and drop, tag filtering, preview rendering, and selection change during a conflicting write.
- A separate macOS WebKit probe opened two native windows against the same synthetic file. Both resolved the same item ID at revision 0. The second window committed revision 1, the main window received `revision-conflict` for its stale save, and the source file's contents remained intact.
- Follow-up requested by the user. Available Desktop, Downloads, Documents and Pictures directories are added once per profile using the platform's known folder paths. Existing links and their groups are retained and removed defaults stay removed. Both APFS regression tests and a two-launch macOS WebKit IPC probe passed with an isolated home directory. The macOS app bundle was rebuilt.

## Notes
- Favorites require conclusive continuity or loss evidence. The workspace refresh reconciles at most 128 retained handles, prioritizing favorites. Older identity keys and unanchored path-only views stay stored for recovery.
- File favorites currently reveal and select the item in its parent folder. Opening it in an associated application belongs to native file integration. Read-only file previews and the Assistant tab belong to content processing.
- Native NTFS/WebView2, assistive technology and remounted/cloned APFS are release checks. Full Windows cross-check could not run offline because `jiff-tzdb 0.1.9` is absent from the Cargo cache.

## Handoff
- 2026-11-19 codex. The local-data workspace and storage scope passed automated and macOS validation. The user confirmed closure. Use the [local data contract](../../../../../../docs/local-data/contract.md) for the copy journal API and live identity limits. The [file-transfer engine](../../../file-operations/tasks/file-transfer-engine/context.md) owns integration with native publication and end-to-end copy verification. Deferred native and accessibility checks are recorded under release acceptance.
