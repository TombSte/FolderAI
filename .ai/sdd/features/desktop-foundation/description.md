# Feature: desktop-foundation

## Goal
Launch a native desktop explorer on macOS Apple Silicon and establish contracts for subsequent features. Keep Windows 11 x64 as a first-release target with native verification in release acceptance.

## Problem
The project needs a native explorer baseline and a validated first-run browsing flow.

## Scope
Native boot, IPC, first browse, shell and per-context state boundaries.

## Out of Scope
Full session recovery and data storage are covered by downstream features.

## Acceptance Criteria
A clean checkout builds and launches on macOS Apple Silicon. A user can browse Home with no AI setup, IPC errors remain scoped and the native window controls work. Initial macOS platform evidence is recorded. Windows 11 x64 gets the same native acceptance as part of [release acceptance](../release-acceptance/description.md).

## Constraints and Dependencies
Initial support is provisional until DA-001 is closed. Use the approved Workspace sobrio design.

The matrix is provisional and downstream development may use the capabilities already observed. Windows capabilities must be treated as pending until verified. Closing DA-001 and accepting Windows for the first release require native Windows evidence.

## Planned Tasks
Follow the [first-release implementation order](../../execution-order.md) when work moves between features.

- [platform-capability-matrix](tasks/platform-capability-matrix/context.md) (`discovery`). Probe OS, filesystem, permissions, native links, trash and replacement primitives. Record supported and degraded combinations.
- [scaffold-desktop-app](tasks/scaffold-desktop-app/context.md) (`implementation`). Establish Tauri 2, Rust, React, TypeScript and SCSS, lockfiles, dev/build commands and modular boundaries.
- [typed-ipc-and-read-adapter](tasks/typed-ipc-and-read-adapter/context.md) (`implementation`). Define typed commands, context IDs, errors, events and cancellation; implement native read/list and Home discovery with least-privilege webview configuration.
- [initial-workspace](tasks/initial-workspace/context.md) (`implementation`). Render the approved workspace shell and startup-to-Home browse flow with local UI assets and native window controls.

## Autonomy
- Mode: `human-checkpoints`, selected by the user on 2026-11-18.
- Checkpoints: feature validation and explicit user confirmation for completion. No intermediate checkpoint has been agreed yet.
- Checkpoint decisions: the user requested continuation through feature completion after the parallel scaffold option was presented. Proceed with the reversible implementation while retaining native acceptance and final confirmation.
- 2026-11-18 codex. The user requested postponing Windows verification. Move the Windows native checks and the DA-001 release decision into the existing release acceptance tasks. Retain `human-checkpoints` and final confirmation for the macOS foundation checkpoint.
- 2026-11-18 codex. macOS acceptance passed, including the native window controls. The user explicitly confirmed feature completion after validation.

## Definition of Done
- [x] Behavior implemented
- [x] Tests updated or reason documented
- [x] Relevant validation passes
- [x] Docs updated or reason documented
- [x] Feature ready for validation
- [x] User confirms feature completion after successful validation.
