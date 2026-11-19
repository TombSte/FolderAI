import { invoke } from '@tauri-apps/api/core';

export type Startup = { mode: 'home' | 'custom' | 'restore'; path: string | null };
type SavedTab = { path: string | null; selected: string | null };
type SavedPanel = { tabs: SavedTab[]; activeTab: number };
export type Snapshot = { panels: SavedPanel[]; activePanel: number };
export type TabLocationRequest = { path: string | null; selectPath?: string };
export type Tab = SavedTab & { id: string; start: string | null; fallback: boolean; mounted: boolean; locationRequest?: TabLocationRequest };
export type Panel = { id: string; tabs: Tab[]; active: string };
export type Layout = { panels: Panel[]; active: string };

export const defaultStartup: Startup = { mode: 'home', path: null };

function id() {
  return Array.from(crypto.getRandomValues(new Uint8Array(12)), byte => byte.toString(16).padStart(2, '0')).join('');
}

export function newTab(path: string | null, fallback = false): Tab {
  return { id: id(), path, selected: null, start: path, fallback, mounted: true };
}

export function newPanel(tab: Tab): Panel {
  return { id: id(), active: tab.id, tabs: [tab] };
}

export function initialLayout(snapshot: Snapshot | null, startup: Startup): Layout {
  if (!snapshot?.panels.length) {
    const panel = newPanel(newTab(startup.mode === 'home' ? null : startup.path, true));
    return { panels: [panel], active: panel.id };
  }
  const panels = snapshot.panels.map((saved) => {
    const tabs = saved.tabs.map((tab, index) => ({
      ...newTab(tab.path),
      selected: tab.selected,
      mounted: index === saved.activeTab,
    }));
    return { id: id(), tabs, active: tabs[saved.activeTab].id };
  });
  return { panels, active: panels[snapshot.activePanel].id };
}

export function snapshotOf(layout: Layout): Snapshot {
  return {
    activePanel: layout.panels.findIndex(panel => panel.id === layout.active),
    panels: layout.panels.map(panel => ({
      activeTab: panel.tabs.findIndex(tab => tab.id === panel.active),
      tabs: panel.tabs.map(({ path, selected }, index) => ({
        path,
        selected: index === panel.tabs.findIndex(tab => tab.id === panel.active) ? selected : null,
      })),
    })),
  };
}

export function loadWorkspace() {
  return invoke<{ startup: Startup; window: Snapshot | null }>('load_workspace');
}

export function saveWorkspace(window: Snapshot) {
  return invoke<void>('save_workspace', { window });
}

export function saveStartup(startup: Startup) {
  return invoke<void>('save_startup', { startup });
}

export function openWorkspaceWindow(path: string | null) {
  return invoke<void>('open_workspace_window', { path });
}

export function closeWorkspaceWindow() {
  return invoke<void>('close_workspace_window');
}
