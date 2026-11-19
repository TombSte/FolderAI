# Feature: file-operations

## Goal
Perform manual file operations with recoverable failure handling and native interoperability.

## Problem
Native concurrency, partial results and filesystem differences require explicit guarantees.

## Scope
Mutations, transfers, activity UI, safe replacement, undo, native integration, links and ZIP.

## Out of Scope
External drives, network sources, batch rename, archive creation and formats beyond ZIP are outside release scope.

## Acceptance Criteria
Demonstrate native operations and fault-injected transfers with safe collision, interruption and restoration outcomes.

## Constraints and Dependencies
DA-003–007 and the DA-001 matrix govern support and failure behavior.

## Planned Tasks
- [transfer-and-undo-contract](tasks/transfer-and-undo-contract/context.md) (`discovery`). Specify error grouping, collision naming, replacement commit, partial results, retry, trash outcomes and recoverable action rules.
- [file-transfer-engine](tasks/file-transfer-engine/context.md) (`implementation`). Implement file creation, rename, copy, move and guarded replacement with per-item results, safe cancellation, cleanup and identity reconciliation.
- [trash-undo-and-activity](tasks/trash-undo-and-activity/context.md) (`implementation`). Implement trash, confirmed permanent deletion, window-scoped undo and activity center with coordinated transfer close behavior.
- [native-integration-contract](tasks/native-integration-contract/context.md) (`discovery`). Validate opening preferences, terminal choices and native clipboard/drop payloads and modifiers on both OSes.
- [native-file-integration](tasks/native-file-integration/context.md) (`implementation`). Wire Open With, app-local associations, terminal, clipboard and drag/drop including hover navigation, modifiers and successful cut handling.
- [links-and-zip](tasks/links-and-zip/context.md) (`implementation`). Close the ZIP/link support matrix, then add link navigation/preservation and bounded archive browsing and extraction with guarded destinations.

## Autonomy
- Mode: `continuous`, selected project-wide by the user on 2026-11-19.
- Checkpoints: feature validation and explicit final user confirmation.

## Definition of Done
- [ ] Behavior implemented
- [ ] Tests updated or reason documented
- [ ] Relevant validation passes
- [ ] Docs updated or reason documented
- [ ] Feature ready for validation
- [ ] User confirms feature completion after successful validation.
