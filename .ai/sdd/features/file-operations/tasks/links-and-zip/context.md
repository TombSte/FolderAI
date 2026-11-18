# Task: links-and-zip

## Type
implementation

## Description
Close the ZIP/link support matrix, then add link navigation/preservation and bounded archive browsing and extraction with guarded destinations.

Traceability covers DA-007 RF-027–028 RT-006 RT-010.

## Depends On
- `file-operations/file-transfer-engine`
- `desktop-foundation/platform-capability-matrix`

## Likely Files
- Rust link adapters and ZIP service, React archive views

## Progress
- [ ] Complete the work described above.

## Validation
- Pending. Test native links, permission errors, malformed/encrypted ZIP, collisions, resource exhaustion and containment under destination changes.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
