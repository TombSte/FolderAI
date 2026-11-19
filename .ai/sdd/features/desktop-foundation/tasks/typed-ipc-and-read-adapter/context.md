# Task: typed-ipc-and-read-adapter

## Type
implementation

## Description
Define typed commands, context IDs, errors, events and cancellation; implement native read/list and Home discovery with least-privilege webview configuration.

Traceability covers RT-002 RT-003 RT-006.

## Depends On
- `desktop-foundation/scaffold-desktop-app`

## Likely Files
- Rust IPC and filesystem adapters, frontend contracts

## Progress
- [x] Implement typed browse targets, request context, start event, scoped cancellation and errors in `src/ipc.ts` and `src-tauri/src/browse.rs`.
- [x] Implement native Home discovery and read-only listing in `src-tauri/src/filesystem.rs`. Constrain the main webview capability to browse and cancel-browse commands.
- Deferred release requirement: [native Windows IPC and filesystem results](../../../release-acceptance/tasks/native-workflow-and-accessibility-validation/context.md).

## Validation
- `cargo test --locked --offline --manifest-path src-tauri/Cargo.toml` passed all five tests covering malformed input, context and window isolation, cancellation, Unicode names, hidden files, missing folders and denied permissions.
- A temporary native WebKit development smoke confirmed Home and Unicode directory contents, `not-found`, `invalid-request`, rejection of an ungranted Tauri Window command, and rendering of real native data.
- A native WebKit replay on APFS verified that a child protected by a deny-list-and-search ACL returns `permission-denied` and leaves the parent browsable on the next request.
- Browser interaction checks with mocked transport confirmed a superseded response leaves the newer directory intact. Native Windows IPC and path interpretation remain unverified.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
- 2026-11-18 codex. Implementation and macOS checks are available. On Windows, run native Cargo tests and exercise Home, Unicode paths, an invalid absolute target, a removed folder and a denied directory in the WebView2 app. Confirm the error remains local to the active browse view.
- 2026-11-18 codex. macOS tests and native WebKit checks pass the foundation IPC checkpoint. Windows remains in release acceptance.
