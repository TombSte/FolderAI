import { invoke } from '@tauri-apps/api/core';

export type Language = 'system' | 'en' | 'it';
export type Theme = 'system' | 'light' | 'dark';
export type SidebarId = 'locations' | 'favorites' | 'tags' | 'saved-searches';
export type SidebarSection = { id: SidebarId; visible: boolean; expanded: boolean };
export type ShortcutId = 'newTab' | 'newWindow' | 'closeTab' | 'address' | 'hidden';
export type ToolbarId = 'new-tab' | 'new-window' | 'close-tab' | 'split' | 'sidebar';
export type Controls = {
  language: Language;
  theme: Theme;
  density: 'normal' | 'compact';
  sidebarWidth: number;
  sidebar: SidebarSection[];
  shortcuts: Record<ShortcutId, string>;
  toolbar: string[];
};

const platformMac = /mac/i.test(navigator.platform);
export const toolbarIds: ToolbarId[] = ['new-tab', 'new-window', 'close-tab', 'split', 'sidebar'];
export const shortcutIds: ShortcutId[] = ['newTab', 'newWindow', 'closeTab', 'address', 'hidden'];
export const defaultControls: Controls = {
  language: 'system', theme: 'system', density: 'normal', sidebarWidth: 208,
  sidebar: (['locations', 'favorites', 'tags', 'saved-searches'] as const)
    .map(id => ({ id, visible: true, expanded: true })),
  shortcuts: { newTab: 'Mod+T', newWindow: 'Mod+N', closeTab: 'Mod+W', address: 'Mod+L',
    hidden: platformMac ? 'Mod+Shift+Period' : 'Mod+H' },
  toolbar: [],
};

export function shortcutFromEvent(event: Pick<KeyboardEvent, 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey' | 'code' | 'key'>) {
  if (platformMac ? !event.metaKey || event.ctrlKey : !event.ctrlKey || event.metaKey) return null;
  const key = event.code.match(/^Key([A-Z])$/)?.[1] ?? event.code.match(/^Digit([0-9])$/)?.[1]
    ?? (/^[A-Za-z0-9]$/.test(event.key) ? event.key.toUpperCase() : event.code);
  if (!/^(?:[A-Z0-9]|F(?:[1-9]|1[0-2])|Period|Comma|Slash|Semicolon|Quote|Backslash|BracketLeft|BracketRight|Space|Minus|Equal|Backquote|Enter|Backspace|Delete|Home|End|PageUp|PageDown|ArrowLeft|ArrowRight|ArrowUp|ArrowDown)$/.test(key)) return null;
  return 'Mod+' + (event.altKey ? 'Alt+' : '') + (event.shiftKey ? 'Shift+' : '') + key;
}

export function isReservedShortcut(value: string) {
  return (platformMac
    ? ['Mod+C', 'Mod+V', 'Mod+X', 'Mod+A', 'Mod+Q', 'Mod+H', 'Mod+M', 'Mod+Alt+H']
    : ['Mod+C', 'Mod+V', 'Mod+X', 'Mod+A', 'Mod+Alt+Delete']).includes(value);
}

export function shortcutLabel(value: string) {
  return value.replace('Mod', platformMac ? '⌘' : 'Ctrl').replace('Period', '.');
}

export function loadWorkspaceControls() {
  return invoke<Controls>('load_workspace_controls');
}

export function saveWorkspaceControls(controls: Controls) {
  return invoke<void>('save_workspace_controls', { controls });
}
