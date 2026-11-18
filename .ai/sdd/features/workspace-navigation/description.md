# Feature: workspace-navigation

## Goal
Navigate and organize the desktop workspace with independent tabs, panels and windows.

## Problem
A single browsing shell needs persistent navigation and a complete keyboard-operable workspace.

## Scope
Session restore, views, selection, context menus, sidebar, settings, localization and accessibility implementation.

## Out of Scope
Real commands and domain settings are wired by the owning features.

## Acceptance Criteria
Verify tab transfers and window isolation, restored views, settings reset, native shortcuts and both languages.

## Constraints and Dependencies
DA-002 and DA-019 govern edge cases. Final accessibility verification is in release acceptance.

## Planned Tasks
- [session-and-layout-contract](tasks/session-and-layout-contract/context.md) (`discovery`). Agree startup, restore and close outcomes across tabs, windows and the app. Specify layout overflow, focus, shortcuts and defaults.
- [navigation-sessions](tasks/navigation-sessions/context.md) (`implementation`). Implement history, editable breadcrumb, startup fallback, multiple windows and panels, tab lifecycle and restore of navigation-only state.
- [file-views-and-selection](tasks/file-views-and-selection/context.md) (`implementation`). Implement Details, Grid and Columns, per-folder views, column options, sorting, grouping, extensions, hidden items and native selection.
- [workspace-controls-and-settings](tasks/workspace-controls-and-settings/context.md) (`implementation`). Build configurable sidebar/toolbars/settings sections, themes, densities, IT/EN localization, accessible focus, shortcuts and command catalog.

## Autonomy
- Mode pending user selection before implementation.
- Checkpoints pending autonomy selection.
- Checkpoint decisions pending.

## Definition of Done
- [ ] Behavior implemented
- [ ] Tests updated or reason documented
- [ ] Relevant validation passes
- [ ] Docs updated or reason documented
- [ ] Feature ready for validation
- [ ] User confirms feature completion after successful validation.
