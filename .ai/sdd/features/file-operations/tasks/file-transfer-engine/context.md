# Task: file-transfer-engine

## Type
implementation

## Description
Implement file creation, rename, copy, move and guarded replacement with per-item results, safe cancellation, cleanup and identity reconciliation.

Traceability covers RF-017–020 RT-006–009 RT-013.

## Depends On
- `file-operations/transfer-and-undo-contract`
- `local-data/identity-and-reconciliation`

## Likely Files
- Rust file commands, transfer engine and operation fixtures

## Progress
- [ ] Complete the work described above.

## Validation
- Pending. Inject space exhaustion, concurrent collisions, interruption, commit and cleanup failures and cross-volume copy failure.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
