# Task: storage-migrations

## Type
implementation

## Description
Implement versioned persistent user and operational stores and separate rebuildable data, with migration recovery and logging rules.

Traceability covers RT-011 RT-017 RT-026.

## Depends On
- `local-data/data-and-identity-contract`
- `desktop-foundation/scaffold-desktop-app`

## Likely Files
- Rust persistence layer and migration fixtures

## Progress
- [x] Add bundled SQLite, schema migrations, a verified pre-upgrade snapshot and application startup integration.
- [x] Isolate the user database from the derived artifact directory and validate migration failure and retry.

## Validation
- `cargo test --locked --offline --manifest-path src-tauri/Cargo.toml` passed. The storage tests open an old store, verify an SQLite-native backup, induce a constraint failure after DDL, simulate subprocess exit before commit, reject a foreign, future or corrupt database without replacing it, and rebuild a disposable derived directory. Note revisions, tag links and favorites are checked during recovery.
- `cargo clippy --locked --offline --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `npm run build` and `git diff --check` passed on macOS Apple Silicon. Native Windows packaging and recovery remain unverified.
- Native WebKit startup on a fresh synthetic macOS profile rendered Home and navigated a Unicode directory. The SQLite file initialized at schema 2 with note and operation tables, reported `integrity_check=ok`, and had mode 0600.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
- 2026-11-18 codex. Storage base implemented under `src-tauri/src/storage/`, started off the UI thread and isolated from browsing. DA-008 remains open for release verification, recovery/product surface and the vector engine. Next: native identity and reconciliation, then organization when the workspace controls dependency is complete.
