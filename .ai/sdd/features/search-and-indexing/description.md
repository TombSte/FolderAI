# Feature: search-and-indexing

## Goal
Find documents with offline lexical search and isolated semantic spaces over authorized content.

## Problem
Each mode needs coherent scope and coverage, and indexed data must track changes and authorization.

## Scope
Search UX, saved queries, source policy, persistent indexing, embeddings and semantic retrieval.

## Out of Scope
Auto and the assistant orchestration are covered by the assistant feature.

## Acceptance Criteria
Verify offline search, access invalidation, restart and removal, compatibility checks and semantic provenance.

## Constraints and Dependencies
DA-013–015 govern the contracts. DA-020 sets quantitative evaluation.

## Planned Tasks
- [search-and-source-contracts](tasks/search-and-source-contracts/context.md) (`discovery`). Define scope/exclusion precedence, index recovery, lexical filters, ranking and saved-query behavior across all modes.
- [simple-and-saved-search](tasks/simple-and-saved-search/context.md) (`implementation`). Implement search bar and modes, live offline simple search with text/metadata/available OCR and persisted query criteria; defer execution for saved Agentica.
- [authorized-sources](tasks/authorized-sources/context.md) (`implementation`). Centralize authorized scope evaluation and UI for recursive sources, exclusions, capabilities and access revocation across reads and indexing.
- [embedding-space-contract](tasks/embedding-space-contract/context.md) (`discovery`). Specify the abstract engine, model revision identity, compatibility descriptor, storage interface and lifecycle of old or partial spaces.
- [embedding-index-isolation](tasks/embedding-index-isolation/context.md) (`implementation`). Implement abstract embedding and vector storage contracts, per-space persistence and compatibility checks on insertion and search.
- [persistent-incremental-indexing](tasks/persistent-incremental-indexing/context.md) (`implementation`). Coordinate extraction and embedding jobs with progress, checkpoints, cancellation, restart, authorization and derived data cleanup.
- [semantic-search-and-mode-recovery](tasks/semantic-search-and-mode-recovery/context.md) (`implementation`). Implement permission-filtered semantic retrieval with ranking, excerpts, current-space selection, coverage and saved-search degradation handling.

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
