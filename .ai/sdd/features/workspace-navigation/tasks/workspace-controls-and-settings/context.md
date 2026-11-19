# Task: workspace-controls-and-settings

## Type
implementation

## Description
Build configurable sidebar/toolbars/settings sections, themes, densities, IT/EN localization, accessible focus, shortcuts and command catalog.

Traceability covers RF-007 RF-016 RF-054–056 RT-032.

## Depends On
- `workspace-navigation/file-views-and-selection`

## Likely Files
- React layout, commands, settings, translations and SCSS

## Progress
- [x] Implement persistent theme, density, language, sidebar controls, toolbar choices and configurable navigation shortcuts.
- [x] Verify settings focus, section reset, shortcut conflicts and reachability at the native minimum.

## Validation
- Native WebKit on macOS verified defaults, conflicting shortcut rejection, restart persistence, and isolated appearance reset. Brave with mocked IPC verified Italian/English, dark/compact appearance, sidebar reorder and keyboard resizing, focus return, and shortcut conflict at 1120 and 620 px. A follow-up at 620×380 confirmed the modal actions remain visible with a scrolling content area.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
- 2026-11-19 codex. Workspace controls are ready for the organization UI. Domain commands and domain settings are routed by their owning features. Retain the overall workspace feature checkpoint and downstream release validation.
