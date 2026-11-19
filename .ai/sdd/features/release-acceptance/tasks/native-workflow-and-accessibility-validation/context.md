# Task: native-workflow-and-accessibility-validation

## Type
validation

## Description
Exercise shell, settings and all native file workflows under keyboard, screen readers, both languages, themes, scaling and constrained layouts. Compare with Workspace sobrio and measure explorer performance.

Traceability covers RF-001–002, RF-046, RF-054–056, RT-001–003, RT-005–006, RT-008, RT-031–033, DA-001 and DA-019–020.

## Depends On
- `release-acceptance/cross-platform-test-harness`
- `file-operations/native-file-integration`
- `file-operations/links-and-zip`
- `assistant-and-auto/auto-mode-routing`

## Likely Files
- Native acceptance scenarios and accessibility evidence

## Progress
- [ ] Complete the work described above.

## Validation
- Pending. Record Windows and macOS traces including folders, windows, drag/drop, dialogs, error states, search and Assistant.
- Deferred Windows 11 x64 checks from desktop foundation. Run the [platform probes](../../../../../../docs/platform-probes/README.md) on NTFS and on candidate volumes. Record native permission, link, trash, naming and replacement results and publish the final DA-001 matrix. Exercise Home and Unicode, missing and denied locations with WebView2 on a clean profile, offline and without AI setup. Verify navigation, scoped IPC errors and native window controls.
- Exercise favorites, tag filtering, note conflicts across native windows, pending drafts at window close and folder view continuity on Windows 11 x64 with NTFS and WebView2. Cover keyboard and the native screen reader on both platforms. Include APFS remount and cloned-volume identity evidence. Verify metadata copy with both preference values through the finished file-transfer workflow.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
