# Task: platform-capability-matrix

## Type
discovery

## Description
Probe OS, filesystem, permissions, native links, trash and replacement primitives. Record supported and degraded combinations.

Traceability covers RT-001 RT-005 RT-006 RT-008 DA-001.

## Depends On
- `none`

## Likely Files
- Platform adapter spikes and requirements/open-decisions.md

## Progress
- [ ] Complete the work described above.

## Validation
- Pending. Native results on Windows 11 x64 and macOS Apple Silicon with OS, filesystem and capability recorded.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. The user approved the first-release backlog. Verified 9 feature records, 43 task contexts, index alignment and an acyclic dependency graph. This is the first dependency-ready task. Before starting, confirm implementation autonomy, then run the native capability probes for DA-001 on Windows 11 x64 and macOS Apple Silicon. Native evidence is pending.
