# Task: navigation-sessions

## Type
implementation

## Description
Implement history, editable breadcrumb, startup fallback, multiple windows and panels, tab lifecycle and restore of navigation-only state.

Traceability covers RF-001–006 RT-004.

## Depends On
- `workspace-navigation/session-and-layout-contract`
- `local-data/storage-migrations`

## Likely Files
- React navigation, Rust window/session services

## Progress
- [ ] Complete the work described above.

## Validation
- Pending. Test fallback, invalid path, external refresh, tab moves and closing the last tab across isolated windows.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
