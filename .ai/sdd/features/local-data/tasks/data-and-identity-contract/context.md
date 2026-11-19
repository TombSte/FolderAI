# Task: data-and-identity-contract

## Type
discovery

## Description
Compare durable and vector storage candidates and establish schema, recovery, identity evidence, watcher reliability and concurrent metadata rules.

Traceability covers DA-008–010 RT-011–014 RT-017 RT-026.

## Depends On
- `desktop-foundation/platform-capability-matrix`

## Likely Files
- Storage and identity spikes, decision register

## Progress
- [x] Document the storage and identity contract, provisional defaults, assumptions, degraded cases and acceptance coverage.
- [x] Validate transactional rollback, backup, note conflicts and identity observations on disposable macOS fixtures.

## Validation
- `python3 docs/local-data/probe.py` passed on macOS Apple Silicon and APFS. It covered rollback, backup, revision conflict, derived rebuild, same-volume rename, hard links, same-path replacement and a moved descendant.
- An optimized Rust spike on 100,000 synthetic records ran against bundled SQLite 3.54.1 and redb 4.6.0. The [contract](../../../../../../docs/local-data/contract.md) records timing, size, scope, design review and the open vector and native validation decisions.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
- 2026-11-18 codex. Discovery deliverable validated. Use bundled SQLite for the user and operation tables in one transaction and separate the disposable derived data. DA-008–010 remain open for native, product-flow and vector-engine acceptance. Continue with versioned migrations and failure tests.
