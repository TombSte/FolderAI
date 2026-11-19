# Feature: providers-and-consent

## Goal
Configure and use models without implicit remote data transfer.

## Problem
The AI stack needs verified capabilities, protected credentials and one authorization boundary for every remote role.

## Scope
Provider matrix, model setup, local and remote adapters, OS secrets and consent.

## Out of Scope
Subscription access depends on an official verified mechanism.

## Acceptance Criteria
Demonstrate local-only default, verified local models, OpenAI and Anthropic API integrations, and denial or revocation before unauthorized sends.

## Constraints and Dependencies
DA-016 and DA-017 precede their applicable activations. Record provider, runtime, model and platform evidence.

## Planned Tasks
- [provider-capability-matrix](tasks/provider-capability-matrix/context.md) (`discovery`). Verify runtimes, supported model roles, endpoint classification, official API capabilities and any eligible subscription flow.
- [connection-setup-and-secrets](tasks/connection-setup-and-secrets/context.md) (`implementation`). Implement provider/model contracts, connection tests and guided/advanced setup with manual model selection and OS credential storage.
- [local-model-adapters](tasks/local-model-adapters/context.md) (`implementation`). Support validated local embedding, assistant and vision roles with explicit capability and availability reporting.
- [remote-consent-gate](tasks/remote-consent-gate/context.md) (`implementation`). Resolve the DA-017 lifecycle, then mediate all remote roles by policy, request, provider, scope and data category, including background work and revocation.
- [remote-api-adapters](tasks/remote-api-adapters/context.md) (`implementation`). Integrate OpenAI and Anthropic APIs through the consent boundary and incorporate additional providers only where the matrix confirms coverage.

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
