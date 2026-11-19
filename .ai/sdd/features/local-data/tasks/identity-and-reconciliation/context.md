# Task: identity-and-reconciliation

## Type
implementation

## Description
Maintain stable references and reconcile internal effects, lost events, trustworthy external changes and unavailable versus removed items.

Traceability covers RF-030 RF-034 RT-012–013 RT-030.

## Depends On
- `local-data/storage-migrations`
- `desktop-foundation/typed-ipc-and-read-adapter`

## Likely Files
- Rust identity, watcher, reference and reconciliation services

## Progress
- [x] Add lossless native locator encoding and component-aware descendant relocation.
- [x] Verify live APFS identity and transactionally reconcile a tracked reference without following a same-path replacement.
- [x] Implement watcher subscriptions, coalescing and rescan scheduling for explicitly tracked references.
- [x] Implement a move journal with idempotent recovery given live native proof. Preserve unresolved records when continuity is uncertain.
- [x] Implement the Windows NTFS identity adapter and cross-check its Rust types.
- [x] Resolve saved metadata at its existing location without binding a same-path replacement to the old item.
- Windows native runtime and full product workflow integration remain acceptance cases in the respective consumer and release tasks.

## Validation
- `cargo test --locked --offline --manifest-path src-tauri/Cargo.toml` passed on macOS Apple Silicon. The APFS probe verifies a live handle after a subtree move and reports a hard link as ambiguous. The database reconciliation test preserves both favorites on ambiguity and preserves notes/tags when a fully removed item loses its favorite links.
- 18 Rust tests passed, including watcher registration and hints (polling fallback available), a simulated lost-event rescan, a deferred SQL foreign-key failure at COMMIT after a native rename, idempotent recovery and same-path replacement while the journal was pending.
- Native WebKit on macOS browsed a Unicode folder after startup upgraded the synthetic profile from schema 2 to schema 3. The current store and the version 2 snapshot both returned `integrity_check=ok`.
- Isolated `cargo check --offline --target x86_64-pc-windows-msvc` passed for identity, watcher and storage modules. No native Windows or full Tauri target build evidence was obtained. Formatting, Clippy and frontend build passed on macOS.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
- 2026-11-18 codex. Active implementation under `src-tauri/src/local_identity.rs`, `src-tauri/src/local_identity/macos.rs` and `src-tauri/src/storage/`. A volatile tracked reference binds a live handle to one persisted record and uses the last recorded locator and comparison key as write preconditions. Restart must not turn a matching persisted key into proof of continuity without trusted evidence. Continue the watcher and internal recovery work before closing this task. Favorites, tags and notes still depend on `workspace-navigation/workspace-controls-and-settings`.
- 2026-11-18 codex. Added `local_identity/windows.rs`, `local_identity/watch.rs`, `storage/reference_monitor.rs`, `storage/moves.rs` and migration 003. Continue validation with Windows NTFS and connect the services when the relevant file-operation and workspace consumers are implemented. Durable operation records are visible to the recovery API after restart. A caller without an independent witness must leave the original association unresolved. The feature remains in progress. The favorites/tags/notes task depends on workspace controls in the existing plan.
- 2026-11-18 codex. Added atomic registration of a directory move and its tracked descendants before their new paths exist. The APFS test rolls the plan back if it includes a neighbor outside the subtree, then verifies a real rename preserves descendant metadata. Work is switching to the user-approved workspace navigation prerequisite. Reconciliation remains open for integration and native NTFS execution.
- 2026-11-19 codex. Native APFS tests prove successful lookup after restart, rejection of a replacement at the saved path, and safe preservation of the original note. Windows identity includes creation time and the isolated x64 type check passed. The verified native services are available to organization and file operations. Carry Windows runtime execution to release validation under the Windows verification deferral.
