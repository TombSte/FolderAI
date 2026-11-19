# Task: native-integration-contract

## Type
discovery

## Description
Validate opening preferences, terminal choices and native clipboard/drop payloads and modifiers on both OSes.

Traceability covers DA-003–004 RF-024–026 RT-005.

## Depends On
- `desktop-foundation/platform-capability-matrix`

## Likely Files
- Platform integration spikes and decision register

## Progress
- [x] Probe AppKit file URLs, installed default handlers, native content types and Terminal discovery.
- [x] Verify the native working directory for a new macOS Terminal instance with a Unicode and space-containing path.
- [x] Document the adapter contract and separate native API evidence from application walkthroughs.

## Validation
- `swiftc -warnings-as-errors` compiled the macOS probe. `--two-processes` passed on macOS Apple Silicon using two processes and a private pasteboard. Both URLs retained their paths and order. Native lookup returned TextEdit for TXT, Preview for PNG, and a Terminal bundle. Those defaults are specific to the host used for the probe.
- The isolated [macOS Terminal probe](../../../../../../docs/platform-probes/macos-terminal.swift) compiled with warnings treated as errors. In a desktop session, `NSWorkspace.open` accepted a file URL without command interpolation and `lsof` verified the login shell's working directory on APFS.
- The native-file-integration task owns Finder, Desktop and representative app walkthroughs, reuse and alternate terminals, and removed-handler fallback. Run Windows Explorer scenarios with the deferred Windows desktop acceptance.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-19 codex. Discovery delivered the [interop evidence and adapter contract](../../../../../../docs/platform-probes/file-interop.md). Native file integration owns the walkthroughs with Finder, Desktop and representative apps. Windows runtime evidence remains scheduled for acceptance. The feature now uses the project-wide continuous mode.
