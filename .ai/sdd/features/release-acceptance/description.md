# Feature: release-acceptance

## Goal
Produce evidence and supported release packages for Windows and macOS.

## Problem
The release needs native validation, measured quality and a verified installation and update path.

## Scope
Evidence matrix, accessibility, performance, offline and privacy checks, install/update and release prerequisites.

## Out of Scope
Publishing is a separate authorized action.

## Acceptance Criteria
Record passing criteria, versions, hardware/provider configuration and native evidence. Close release-blocking decisions and obtain final feature confirmation after validation.

## Constraints and Dependencies
DA-020 and DA-021 must close before final acceptance. Track missing evidence explicitly.

## Planned Tasks
- [acceptance-protocol-and-instrumentation](tasks/acceptance-protocol-and-instrumentation/context.md) (`discovery`). Define representative corpora, hardware, accessibility procedures, measurements and thresholds. Establish the criterion-to-evidence matrix and implement measurement support in a follow-on task.
- [cross-platform-test-harness](tasks/cross-platform-test-harness/context.md) (`test`). Implement the evidence harness, measurement instrumentation and repeatable build/test flows with controlled fixtures and provider traces.
- [native-workflow-and-accessibility-validation](tasks/native-workflow-and-accessibility-validation/context.md) (`validation`). Exercise shell, settings and all native file workflows under keyboard, screen readers, both languages, themes, scaling and constrained layouts. Compare with Workspace sobrio and measure explorer performance.
- [ai-quality-privacy-and-offline-validation](tasks/ai-quality-privacy-and-offline-validation/context.md) (`validation`). Measure formats, relevance, indexing and routing on the approved corpus with offline and authorized remote configurations. Verify model changes, storage preservation and transfer denial.
- [installation-updates-and-release-signoff](tasks/installation-updates-and-release-signoff/context.md) (`validation`). Resolve packaging, signing and update decisions; build release artifacts and verify clean installation and an upgrade that preserves application data. Review acceptance evidence and obtain confirmation.

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
