# Task: identity-and-reconciliation

## Type
implementation

## Description
Maintain stable references and reconcile internal effects, lost events, trustworthy external changes and unavailable versus removed items.

Traceability covers RF-030 RF-034 RT-012–013 RT-030.

## Depends On
- `local-data/storage-migrations`
- `desktop-foundation/typed-ipc-and-read-adapter`

## Likely Files
- Rust identity, watcher, reference and reconciliation services

## Progress
- [ ] Complete the work described above.

## Validation
- Pending. Exercise rename, same-path replacement, moved subtree, event loss and post-filesystem metadata commit failure.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
