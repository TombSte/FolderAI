# Task: initial-workspace

## Type
implementation

## Description
Render the approved workspace shell and startup-to-Home browse flow with local UI assets and native window controls.

Traceability covers RF-001 RF-002 RF-046 RF-055.

## Depends On
- `desktop-foundation/typed-ipc-and-read-adapter`

## Likely Files
- React workspace, SCSS, docs/mockups/folderai-unified.html

## Progress
- [x] Implement Workspace sobrio initial layout, real Home listing, current folder tab, sidebar, breadcrumb, editable path, history, selection and retry in `src/workspace/`.
- [x] Supply local icons, light/dark system themes and standard native window controls.
- [x] Verify the native window controls on macOS.
- Deferred release requirement: [Windows clean-profile offline and native window control validation](../../../release-acceptance/tasks/native-workflow-and-accessibility-validation/context.md).

## Validation
- `npm run build` passed. Headless Brave with mocked Tauri and external requests blocked passed startup, folder-first natural sort, keyboard selection, directory navigation, breadcrumb, history, invalid path, retry, cancellation and stale response isolation.
- Independent visual QA compared the workspace with the approved mockup at 1120×740 and 620×380 in Brave. Narrow screens now keep all columns, with keyboard-accessible scrolling inside the table. The follow-up browser run checked this at 560 px in dark mode and Italian.
- The unmodified macOS release bundle stayed running on a synthetic profile without a development server. A separate release build with test-only probes confirmed that its native WebKit rendered Home from the bundled protocol and read a Unicode folder through IPC. Visual evidence comes from browser checks.
- macOS 26.5.1 Apple Silicon, local ad hoc signed release test copy on a synthetic profile. Native desktop automation could not connect. In-process AppKit instrumentation retrieved the actual standard buttons from the configured Tauri `NSWindow`. It verified visibility and enabled state, activated the minimize button and observed miniaturization, restored the visible window via AppKit, observed the display change from the green button, and observed `NSWindowWillCloseNotification` from the close button. The process exited 0. The probe was confined to the temporary copy.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
- 2026-11-18 codex. Validate the installed app on a clean Windows 11 x64 profile with network disconnected and WebView2 already present. Check Home without AI setup, a directory with spaces and Unicode, navigation and local path errors, then check the native window controls. Recheck native window actions on macOS if desktop automation becomes available. Get explicit user sign-off only after the feature's native acceptance passes.
- 2026-11-18 codex. Windows checks transferred to release acceptance. For the macOS foundation, verify minimize, restore, zoom or full-screen and close on the actual window. Then request explicit feature sign-off.
- 2026-11-18 codex. macOS native window acceptance passed using the standard AppKit controls in the release test copy. The macOS foundation tasks are validated. The feature is waiting for explicit user confirmation.
- 2026-11-18 codex. The user gave final confirmation after macOS validation. The feature is complete and native Windows evidence remains scheduled in release acceptance.
