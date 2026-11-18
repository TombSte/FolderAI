# Task: authorized-sources

## Type
implementation

## Description
Centralize authorized scope evaluation and UI for recursive sources, exclusions, capabilities and access revocation across reads and indexing.

Traceability covers RF-050–052 RT-019 RT-030.

## Depends On
- `search-and-indexing/search-and-source-contracts`
- `local-data/identity-and-reconciliation`
- `workspace-navigation/workspace-controls-and-settings`

## Likely Files
- Rust source policy and React indexing settings

## Progress
- [ ] Complete the work described above.

## Validation
- Pending. Verify link resolution, out-of-scope access, default exclusions, overlapping sources and invalidation when permissions change.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
