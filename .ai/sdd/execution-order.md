# First-release implementation order

The [SDD index](index.md) groups the approved backlog by feature and status. This dependency order covers tasks across all features.

Start a task only after its direct dependencies are `done` and the approved autonomy allows work to proceed. Each level shows the earliest point at which its tasks can become ready. Readiness depends on each task’s own prerequisites. Within a level, the list follows the plan order.

Workspace navigation uses local storage before favorites, tags and notes are added to workspace controls. The embedding index contract and isolation precede the model adapters. The local adapters enable visual processing, which feeds persistent indexing and semantic search.

Task contexts contain the authoritative dependencies. Refresh this order if the approved dependency graph changes.

## Level 1

- [desktop-foundation/platform-capability-matrix](features/desktop-foundation/tasks/platform-capability-matrix/context.md)

## Level 2

- [desktop-foundation/scaffold-desktop-app](features/desktop-foundation/tasks/scaffold-desktop-app/context.md)
- [local-data/data-and-identity-contract](features/local-data/tasks/data-and-identity-contract/context.md)
- [file-operations/native-integration-contract](features/file-operations/tasks/native-integration-contract/context.md)
- [release-acceptance/acceptance-protocol-and-instrumentation](features/release-acceptance/tasks/acceptance-protocol-and-instrumentation/context.md)

## Level 3

- [desktop-foundation/typed-ipc-and-read-adapter](features/desktop-foundation/tasks/typed-ipc-and-read-adapter/context.md)
- [local-data/storage-migrations](features/local-data/tasks/storage-migrations/context.md)
- [file-operations/transfer-and-undo-contract](features/file-operations/tasks/transfer-and-undo-contract/context.md)
- [providers-and-consent/provider-capability-matrix](features/providers-and-consent/tasks/provider-capability-matrix/context.md)
- [search-and-indexing/search-and-source-contracts](features/search-and-indexing/tasks/search-and-source-contracts/context.md)
- [search-and-indexing/embedding-space-contract](features/search-and-indexing/tasks/embedding-space-contract/context.md)
- [release-acceptance/cross-platform-test-harness](features/release-acceptance/tasks/cross-platform-test-harness/context.md)

## Level 4

- [desktop-foundation/initial-workspace](features/desktop-foundation/tasks/initial-workspace/context.md)
- [local-data/identity-and-reconciliation](features/local-data/tasks/identity-and-reconciliation/context.md)
- [content-processing/format-coverage-and-limits](features/content-processing/tasks/format-coverage-and-limits/context.md)
- [search-and-indexing/embedding-index-isolation](features/search-and-indexing/tasks/embedding-index-isolation/context.md)
- [assistant-and-auto/assistant-tools-and-routing-contract](features/assistant-and-auto/tasks/assistant-tools-and-routing-contract/context.md)

## Level 5

- [workspace-navigation/session-and-layout-contract](features/workspace-navigation/tasks/session-and-layout-contract/context.md)
- [file-operations/file-transfer-engine](features/file-operations/tasks/file-transfer-engine/context.md)
- [content-processing/extract-text-and-structure](features/content-processing/tasks/extract-text-and-structure/context.md)

## Level 6

- [workspace-navigation/navigation-sessions](features/workspace-navigation/tasks/navigation-sessions/context.md)
- [file-operations/links-and-zip](features/file-operations/tasks/links-and-zip/context.md)

## Level 7

- [workspace-navigation/file-views-and-selection](features/workspace-navigation/tasks/file-views-and-selection/context.md)

## Level 8

- [workspace-navigation/workspace-controls-and-settings](features/workspace-navigation/tasks/workspace-controls-and-settings/context.md)

## Level 9

- [local-data/favorites-tags-and-notes](features/local-data/tasks/favorites-tags-and-notes/context.md)
- [file-operations/trash-undo-and-activity](features/file-operations/tasks/trash-undo-and-activity/context.md)
- [providers-and-consent/connection-setup-and-secrets](features/providers-and-consent/tasks/connection-setup-and-secrets/context.md)
- [search-and-indexing/authorized-sources](features/search-and-indexing/tasks/authorized-sources/context.md)

## Level 10

- [file-operations/native-file-integration](features/file-operations/tasks/native-file-integration/context.md)
- [content-processing/details-previews-and-thumbnails](features/content-processing/tasks/details-previews-and-thumbnails/context.md)
- [providers-and-consent/local-model-adapters](features/providers-and-consent/tasks/local-model-adapters/context.md)
- [providers-and-consent/remote-consent-gate](features/providers-and-consent/tasks/remote-consent-gate/context.md)
- [search-and-indexing/simple-and-saved-search](features/search-and-indexing/tasks/simple-and-saved-search/context.md)

## Level 11

- [content-processing/ocr-and-visual-understanding](features/content-processing/tasks/ocr-and-visual-understanding/context.md)
- [providers-and-consent/remote-api-adapters](features/providers-and-consent/tasks/remote-api-adapters/context.md)

## Level 12

- [search-and-indexing/persistent-incremental-indexing](features/search-and-indexing/tasks/persistent-incremental-indexing/context.md)

## Level 13

- [search-and-indexing/semantic-search-and-mode-recovery](features/search-and-indexing/tasks/semantic-search-and-mode-recovery/context.md)

## Level 14

- [assistant-and-auto/read-only-assistant-runtime](features/assistant-and-auto/tasks/read-only-assistant-runtime/context.md)

## Level 15

- [assistant-and-auto/assistant-session-and-citations](features/assistant-and-auto/tasks/assistant-session-and-citations/context.md)

## Level 16

- [assistant-and-auto/auto-mode-routing](features/assistant-and-auto/tasks/auto-mode-routing/context.md)

## Level 17

- [release-acceptance/native-workflow-and-accessibility-validation](features/release-acceptance/tasks/native-workflow-and-accessibility-validation/context.md)
- [release-acceptance/ai-quality-privacy-and-offline-validation](features/release-acceptance/tasks/ai-quality-privacy-and-offline-validation/context.md)

## Level 18

- [release-acceptance/installation-updates-and-release-signoff](features/release-acceptance/tasks/installation-updates-and-release-signoff/context.md)
