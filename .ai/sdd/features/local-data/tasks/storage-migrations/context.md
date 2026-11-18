# Task: storage-migrations

## Type
implementation

## Description
Implement versioned persistent user and operational stores and separate rebuildable data, with migration recovery and logging rules.

Traceability covers RT-011 RT-017 RT-026.

## Depends On
- `local-data/data-and-identity-contract`
- `desktop-foundation/scaffold-desktop-app`

## Likely Files
- Rust persistence layer and migration fixtures

## Progress
- [ ] Complete the work described above.

## Validation
- Pending. Open old data, induce migration failure and rebuild derivations while comparing personal data.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
