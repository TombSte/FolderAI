# Task: trash-undo-and-activity

## Type
implementation

## Description
Implement trash, confirmed permanent deletion, window-scoped undo and activity center with coordinated transfer close behavior.

Traceability covers RF-021–023 RT-005 RT-007 RT-009.

## Depends On
- `file-operations/file-transfer-engine`
- `workspace-navigation/workspace-controls-and-settings`

## Likely Files
- Rust trash and undo adapters, React dialogs and activities

## Progress
- [ ] Complete the work described above.

## Validation
- Pending. Verify trash unavailable, modified undo destination, session reset, skip/retry behavior and isolation between windows.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
