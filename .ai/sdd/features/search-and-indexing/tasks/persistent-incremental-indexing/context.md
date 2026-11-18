# Task: persistent-incremental-indexing

## Type
implementation

## Description
Coordinate extraction and embedding jobs with progress, checkpoints, cancellation, restart, authorization and derived data cleanup.

Traceability covers RF-050–052 RT-017 RT-020–022 EMB-005.

## Depends On
- `search-and-indexing/authorized-sources`
- `search-and-indexing/embedding-index-isolation`
- `content-processing/ocr-and-visual-understanding`

## Likely Files
- Rust job scheduler, derived stores and React indexing status

## Progress
- [ ] Complete the work described above.

## Validation
- Pending. Change files mid-job, revoke a source, restart after interruption and prove no work continues after exit.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
