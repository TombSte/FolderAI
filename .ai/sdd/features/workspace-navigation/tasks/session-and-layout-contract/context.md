# Task: session-and-layout-contract

## Type
discovery

## Description
Agree startup, restore and close outcomes across tabs, windows and the app. Specify layout overflow, focus, shortcuts and defaults.

Traceability covers DA-002 DA-019 RF-001 RT-004.

## Depends On
- `desktop-foundation/initial-workspace`

## Likely Files
- requirements/open-decisions.md and docs/mockups/

## Progress
- [x] Specify startup, fallback, restore, close, narrow layout and keyboard behavior.

## Validation
- Reviewed RF-001–006, RF-022, RF-049, RF-052, RF-055–056, RT-004 and the approved mockup. The user confirmed preserving unavailable restored tabs and showing one switchable panel in a narrow window. The [contract](../../../../../../docs/workspace-navigation/contract.md) defines remaining implementation defaults and acceptance scenarios.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
- 2026-11-18 codex. Contract reviewed and accepted for implementation. Route implementation under the user's continuous mode and verify edge cases in the implementation and release tasks.
