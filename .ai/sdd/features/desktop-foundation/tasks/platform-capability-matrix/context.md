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
- [x] Add a reproducible macOS probe and publish the preliminary [platform matrix](../../../../../../docs/platform-probes/README.md).
- [x] Record initial native evidence for macOS 26.5.1 (25F80), Apple Silicon, local case-insensitive APFS, as a standard user.
- [x] Record case-sensitive APFS, Finder alias resolution after a target rename and cross-volume rename behavior on macOS.
- [x] Prepare a reusable Windows 11 x64 probe for volume capabilities, links, naming, exclusive publication and explicit replacement.
- [x] Publish provisional capabilities and degraded combinations on the tested macOS volumes.
- Deferred release requirement: Windows 11 x64 probes and the official DA-001 matrix, including the minimum supported macOS version. See [native workflow validation](../../../release-acceptance/tasks/native-workflow-and-accessibility-validation/context.md).

## Validation
- `sw_vers`, `uname -sm` and `df -T apfs` identified the tested environment and volume. `diskutil info` could not access DiskManagement in the sandbox.
- `xcrun clang -Wall -Wextra -Werror -fobjc-arc -framework Foundation docs/platform-probes/macos.m -o "$probe_dir/macos-probe"` compiled successfully. The probe passed name and permission cases, hard links and symlinks, same-volume replacement, and two exclusive publications competing for one pathname.
- Native trash initially returned `NSCocoaErrorDomain` 513 in the sandbox. With access to the system Trash, the complete probe exited 0 and restored its synthetic item before cleanup.
- Both directions of the macOS probe between the local APFS volume and a mounted case-sensitive APFS image exited 0. `diskutil info` confirmed the case-sensitive volume. The Finder alias resolved after the target was renamed. Cross-volume `rename` returned `EXDEV` and preserved both files. The case-sensitive image was detached and the temporary artifacts removed.
- Local PowerShell 7.5.5 parsed `docs/platform-probes/windows.ps1` and compiled its embedded C# via `Add-Type`. No Windows 11 x64 environment was available for the Win32 calls. The matrix lists the remaining manual Windows cases and macOS gaps, including system-protected locations and failed cross-volume copies.
- On local APFS, a directory ACL denying list and search for the current standard user blocked access while the parent listing still included the directory. The matching native WebKit IPC check returned `permission-denied`; listing the parent again succeeded. Reproduction steps are in the platform probe README.
- On macOS 26.5.1, `diskutil image create blank --fs ExFAT --format RAW --size 128M` produced a temporary ExFAT volume. Five runs showed that `chmod 000` did not deny reads, hard links returned `ENOTSUP`, the Finder alias failed to resolve after rename, and both concurrent exclusive publications to a free destination returned `ENOTSUP`. Trash and restore passed. The final fixture cleanup by `FileManager` returned Cocoa error 513 on the image. Cross-volume rename returned `EXDEV` and preserved both files in both directions. The APFS-side repeat passed and the image was detached.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. The user approved the first-release backlog. Verified 9 feature records, 43 task contexts, index alignment and an acyclic dependency graph. This is the first dependency-ready task. Before starting, confirm implementation autonomy, then run the native capability probes for DA-001 on Windows 11 x64 and macOS Apple Silicon. Native evidence is pending.
- 2026-11-18 codex. The user selected `human-checkpoints`. This task is blocked on native Windows 11 x64 access. Resume by running the Windows NTFS cases from the matrix, then confirm the candidate macOS versions and volumes and record remaining native results. Keep DA-001 open and retain the scaffold dependency until this task is validated. The probe and matrix are under `docs/platform-probes/`, and the DA-001 row links to them.
- 2026-11-18 codex. On continuation, expanded and ran the macOS probe against both APFS variants and added a Windows probe that passes local source checks. Resume on a Windows 11 x64 desktop as a standard user using `pwsh -NoProfile -File .\docs\platform-probes\windows.ps1` on NTFS, then collect the remaining shell and filesystem evidence in the matrix. The approved dependency remains in effect unless the user approves a plan revision.
- 2026-11-18 codex. The feature implementation proceeded in parallel with the matrix. The remaining Windows native evidence is still required for DA-001 and feature acceptance. The user confirmed no Windows 11 x64 desktop is currently available.
- 2026-11-18 codex. Measured ExFAT on the macOS host and documented the degraded capabilities. The matrix preserves the Windows 11 x64 validation requirement, and no compatible native environment became available.
- 2026-11-18 codex. The macOS discovery deliverable is validated for the tested OS and volumes. At the user's request, Windows verification moves to release acceptance. DA-001 remains open for the official cross-platform matrix.
