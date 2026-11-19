# Workspace navigation contract

The user approved keeping unavailable restored tabs and alternating visible panels in narrow windows. This contract guides DA-002 and DA-019 without changing the approved [Workspace sobrio design](../mockups/README.md) or [functional requirements](../../requirements/functional.md).

## Startup, session and close behavior

| Scenario | Visible result |
| --- | --- |
| Default startup or a generic new window | Open one panel and one tab at the configured start location, which defaults to Home. A new window starts with its own independent state. |
| Custom start location unavailable | Keep the preference and show a notice while attempting Home. If Home is unavailable too, show an error with Retry and keep the path editor usable. |
| Restore enabled | Restore each saved window, the order and membership of its tabs and panels, and the active panel and tabs. Restore the active tab's selection only when those entries still exist. Validate locations as tabs open. |
| Restored location unavailable | Keep the tab and its saved location. Show Retry and Up when its parent can be determined. Other tabs and windows continue browsing. |
| Enable a second panel | Retain the existing tabs and create one new tab at the configured start location in the new panel. |
| Merge two panels | Append tabs from the second panel after the first panel's tabs. Preserve the active tab from the focused panel. Enabling split again creates a fresh second panel. |
| Close or move a tab | If an active tab is removed, focus the next tab or, at the end, the previous one. Closing the last tab of a split panel removes that panel. Closing the last tab of a single panel follows the window-close flow. Moving transfers a tab rather than duplicating it. |
| Close tabs or windows with work | Ask once for the affected tabs or window. Keep open or cancel affected AI and wait for safe transfer interruption when applicable. Other windows continue normally. |
| Quit the app with work | Coordinate one application-level decision with affected-work summaries. Wait for safe cancellation of foreground work. Checkpoint application-level indexing separately, then reapply indexing authorizations on restart. |

Persist only the configured startup behavior and navigation snapshot data needed by RF-001. The navigation snapshot stores IDs, order, active tab/panel, current paths and the active selection. History, transient UI, undo, transfers, search drafts and Assistant conversations remain in memory. The session adapter must validate fields and scope writes to the submitting window. Runtime close blockers are owned by the file operations and Assistant features.

## Layout and keyboard

- Use the existing 620 × 380 native minimum. The file area uses side-by-side panels while both can remain useful. Below 900 CSS px, show the focused panel and a labeled control to switch. Switching preserves both panel states. This breakpoint is an implementation starting point to verify at the minimum width and with text scaling.
- Sidebar and the right panel must leave file commands reachable. In a narrow window, use an overlay for the right panel, return focus to its trigger when closed, and keep panel switching available. File tables can scroll horizontally inside the file area. Keep menus, tabs and current paths reachable when their contents overflow.
- Preserve the focus target across an external refresh if that target still exists. Otherwise move focus to the retained selection, adjacent row, or file view header. When a tab is activated using the keyboard, keep focus at the tab control until the user enters the pane. Return focus to the originating control after modal dialogs or menus close.
- Start with Command/Ctrl+L for the address field, Command/Ctrl+T for a new tab, Command/Ctrl+W for close, Command/Ctrl+N for a new window, Enter to open a selected directory and Escape to dismiss the active editor or menu. Display platform-appropriate shortcut labels and prevent configurable conflicts.
- Use natural locale sorting with directory-first and name ascending defaults. Display missing metadata as unavailable and place it after present values. In mixed selections, retain selection across context menus and enable actions only when their owning feature supports the entire selection. Use normal and compact density, system UI fonts and visible focus under all themes.

## Validation

Exercise the cases above in an isolated native session. Use two windows, two panels, a restored unavailable path and concurrent browse requests. Confirm a clean launch without prior state, both locales, keyboard-only navigation and switching at the native minimum. Test transaction failures on session persistence and confirm recovery preserves the previous saved record. Recheck assistive technology, Windows behavior and integrated transfer and AI close scenarios during release acceptance.
