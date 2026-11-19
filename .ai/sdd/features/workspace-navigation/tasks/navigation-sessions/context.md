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
- [x] Implement bounded navigation capture and restore through the scoped Tauri workspace commands and local SQLite preferences.
- [x] Add native window creation, tab lifecycle and independent two-panel browsing with a narrow-window switch.
- [x] Verify native macOS creation, session persistence, restore and window close.

## Validation
- Brave macOS with mocked Tauri verified independent split navigation at 1120×740, switching at 620×380, merging without losing tabs, moving a tab without duplicating it, restoring an unavailable path with Retry and Up, and retaining focus after closing the active tab.
- A native macOS probe on an isolated profile and local port opened two native windows, captured distinct paths, restarted and restored both windows, pruned an explicitly closed secondary window, preserved the last-window capture and rejected a relative path and an ungranted window command in the secondary.
- 24 Rust tests and Clippy passed on macOS. TypeScript and Vite build passed. Browser QA rechecked keyboard focus after tab close, creation and transfer, and verified that tab-to-panel accessibility references resolve for lazily loaded restored tabs. Ten path cases passed for POSIX, drive roots and conventional and extended UNC paths.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
- 2026-11-18 codex. Frontend state in `src/workspace/{Workspace,FolderPane,session,navigation,paths}` and native backend in `src-tauri/src/workspace_session.rs`. Validate final native flow, then begin file views and selection. Preserve the approved prerequisite order for local-data.
- 2026-11-18 codex. Native macOS and browser session evidence passed. Continue with file views and selection. The actual activity cancellation and broader accessibility cases belong to their owning feature and release acceptance. Windows runtime remains pending.

- Follow-up (pi): integrare le schede nella barra del titolo macOS mantenendo i pulsanti nativi, le interazioni delle schede e una zona trascinabile. Da implementare e verificare nella build locale; la chiusura della consegna originale resta invariata.

- Follow-up implementato (pi): overlay macOS in `src-tauri/tauri.macos.conf.json`, spazio per pulsanti nativi e area di trascinamento in `src/workspace/Workspace.tsx` e `workspace.scss`, permesso in `src-tauri/capabilities/main-browse.json`. `npm run tauri -- build` riuscito. Da verificare visivamente nella build macOS: allineamento, trascinamento, interazioni e finestra secondaria. Nessuna validazione runtime Windows; configurazione overlay solo macOS. Stato originale invariato.
