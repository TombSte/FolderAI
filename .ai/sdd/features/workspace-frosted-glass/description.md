# Feature: Workspace frosted glass

## Goal and scope
Restyle the existing workspace against `docs/images/image.png`. Map the mockup's navigation, toolbar, file list and inspector to `Workspace.tsx`, `FolderPane.tsx` and `OrganizationPanel.tsx`. Give chrome a cool frosted treatment; keep file and inspector content more opaque. Use compact rows, subtle borders and blue selection. Style existing context menus in `FileMutationControls.tsx` and preserve Lucide command icons.

## Out of scope
Do not add mockup-only AI cards, summary or composer if there is no existing equivalent. Do not change commands, transfers or drag acceptance. Do not simulate desktop wallpaper.

## Acceptance
Comparable light/dark scenes with selected file, open inspector and context menu. Distinct keyboard focus and selection. Existing file drag ghost, source and valid target readable. Opaque fallback and reduced-motion support. Record tests and any missing platform evidence.

## Plan
1. `spec`: document mapping and exclusions.
2. `workspace`: implement light/dark materials and visual hierarchy.
3. `feedback`: refine menus, focus and existing drag feedback.
4. `validation`: tests, screenshots, and handoff.

## Autonomy
Continuous through validation per repository preferences. Final feature signoff requires explicit user confirmation.
