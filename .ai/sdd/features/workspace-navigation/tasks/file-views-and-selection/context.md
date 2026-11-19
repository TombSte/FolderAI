# Task: file-views-and-selection

## Type
implementation

## Description
Implement Details, Grid and Columns, per-folder views, column options, sorting, grouping, extensions, hidden items and native selection.

Traceability covers RF-008–011 RF-014–015.

## Depends On
- `workspace-navigation/navigation-sessions`

## Likely Files
- React file views and local preferences

## Progress
- [x] Implement Details, Grid and Columns, global and per-folder view options, grouping, sorting and platform hidden flags.
- [x] Implement range and additive selection, independent pane state, contextual directory actions, keyboard movement and drag selection.

## Validation
- Browser QA on macOS/Brave with mocked IPC at 1120×740 and 620×380 verified natural folder-first sorting, Shift and Cmd selection, all view modes, column previews, accessible View menu focus and narrow overflow. Follow-up covered corrected vertical grid focus and leftward focus across columns.
- Native macOS WebKit checked default and explicit dotfile and UF_HIDDEN visibility, created time, invalid preference rejection, SQLite folder/global priority and reset after restart. Isolated Windows x64 type checking covered the filesystem module and its hidden flag fixture. No native Windows runtime evidence yet.
- 28 Rust tests, Clippy and the frontend build passed during the integrated validation.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
- 2026-11-18 codex. View preferences currently use an explicit per-path SQLite key, plus a global default. Wiring stable tracked references and preserving per-folder options through a verified move belongs to the local-data integration. Content thumbnails, tags, notes and indexing values require their domain providers. Continue with workspace controls.
