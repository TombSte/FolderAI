# Feature: desktop-foundation

## Goal
Launch a native desktop explorer on both targets and establish contracts for subsequent features.

## Problem
The repository has specifications and a browser mockup, while src/ is empty.

## Scope
Native boot, IPC, first browse, shell and per-context state boundaries.

## Out of Scope
Full session recovery and data storage are covered by downstream features.

## Acceptance Criteria
A clean checkout builds and launches on each target. A user can browse Home with no AI setup and IPC errors remain scoped.

## Constraints and Dependencies
Initial support is provisional until DA-001 is closed. Use the approved Workspace sobrio design.

## Planned Tasks
Follow the [first-release implementation order](../../execution-order.md) when work moves between features.

- [platform-capability-matrix](tasks/platform-capability-matrix/context.md) (`discovery`). Probe OS, filesystem, permissions, native links, trash and replacement primitives. Record supported and degraded combinations.
- [scaffold-desktop-app](tasks/scaffold-desktop-app/context.md) (`implementation`). Establish Tauri 2, Rust, React, TypeScript and SCSS, lockfiles, dev/build commands and modular boundaries.
- [typed-ipc-and-read-adapter](tasks/typed-ipc-and-read-adapter/context.md) (`implementation`). Define typed commands, context IDs, errors, events and cancellation; implement native read/list and Home discovery with least-privilege webview configuration.
- [initial-workspace](tasks/initial-workspace/context.md) (`implementation`). Render the approved workspace shell and startup-to-Home browse flow with local UI assets and native window controls.

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
