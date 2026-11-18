# Task: embedding-index-isolation

## Type
implementation

## Description
Implement abstract embedding and vector storage contracts, per-space persistence and compatibility checks on insertion and search.

Traceability covers RT-011 RT-022 EMB-001–005.

## Depends On
- `search-and-indexing/embedding-space-contract`
- `local-data/storage-migrations`

## Likely Files
- Rust embeddings, vector storage adapters and test engine

## Progress
- [ ] Complete the work described above.

## Validation
- Pending. Swap a fake engine, reject incompatible equal-size vectors and unknown revisions, isolate partitions and verify index switching preserves user data.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
