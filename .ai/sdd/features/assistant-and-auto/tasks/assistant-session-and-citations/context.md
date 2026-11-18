# Task: assistant-session-and-citations

## Type
implementation

## Description
Add multiline input and scope selection, tab-local conversation, progress and citation navigation. Coordinate tab/window/app close with running transfers and indexing.

Traceability covers RF-013 RF-038 RF-040 RF-043–044 RF-048–049 RT-004 RT-026 RT-029–030.

## Depends On
- `assistant-and-auto/read-only-assistant-runtime`
- `file-operations/trash-undo-and-activity`
- `content-processing/details-previews-and-thumbnails`

## Likely Files
- React Assistant, activity center and Rust lifecycle

## Progress
- [ ] Complete the work described above.

## Validation
- Pending. Verify submit shortcut, explicit saved execution, scope snapshot, citation round trip, no saved chat and suppression of late events after close.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
