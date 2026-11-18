# Task: simple-and-saved-search

## Type
implementation

## Description
Implement search bar and modes, live offline simple search with text/metadata/available OCR and persisted query criteria; defer execution for saved Agentica.

Traceability covers RF-035–036 RF-038 RT-018.

## Depends On
- `search-and-indexing/search-and-source-contracts`
- `workspace-navigation/workspace-controls-and-settings`
- `content-processing/extract-text-and-structure`
- `local-data/favorites-tags-and-notes`

## Likely Files
- Rust lexical search and saved query store, React search and sidebar

## Progress
- [ ] Complete the work described above.

## Validation
- Pending. Without models or vector store, verify AND/OR, exclusions, notes, available OCR, provenance, restore and zero calls on opening saved Agentica.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
