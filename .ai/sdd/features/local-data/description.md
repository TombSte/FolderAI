# Feature: local-data

## Goal
Keep personal data durable and attached to the correct objects.

## Problem
Notes, tags, favorites and view preferences need storage and reliable identity independent of rebuildable data.

## Scope
Schema, migration, recovery, identities, watcher reconciliation and user-facing organization.

## Out of Scope
Manual backup/export and full vector implementation are covered outside this feature.

## Acceptance Criteria
Rebuild derived data and recover a failed migration while preserving originals and user metadata. Verify concurrent note edits and partial-operation reconciliation.

## Constraints and Dependencies
DA-008, DA-009 and DA-010 guide implementation. Coordinate with file transfers and the vector index.

## Planned Tasks
- [data-and-identity-contract](tasks/data-and-identity-contract/context.md) (`discovery`). Compare durable and vector storage candidates and establish schema, recovery, identity evidence, watcher reliability and concurrent metadata rules.
- [storage-migrations](tasks/storage-migrations/context.md) (`implementation`). Implement versioned persistent user and operational stores and separate rebuildable data, with migration recovery and logging rules.
- [identity-and-reconciliation](tasks/identity-and-reconciliation/context.md) (`implementation`). Maintain stable references and reconcile internal effects, lost events, trustworthy external changes and unavailable versus removed items.
- [favorites-tags-and-notes](tasks/favorites-tags-and-notes/context.md) (`implementation`). Implement favorites groups, tag CRUD and multi-selection, revision-checked Markdown notes, per-folder metadata and configurable copy semantics.

## Autonomy
- Mode pending user selection before implementation.
- Checkpoints pending autonomy selection.
- Checkpoint decisions pending.

## Definition of Done
- [ ] Behavior implemented
- [ ] Tests updated or reason documented
- [ ] Relevant validation passes
- [ ] Docs updated or reason documented
- [ ] Feature ready for validation
- [ ] User confirms feature completion after successful validation.
