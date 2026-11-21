import { AppWindow, ArrowRight, Columns2, Copy, Folder, House, Layers, PanelLeft, PanelRight, Plus, Settings, X } from 'lucide-react';
import { useCallback, useEffect, useRef, useState, type CSSProperties, type PointerEvent } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import FolderPane from './FolderPane';
import { parentPath, pathLabel } from './paths';
import { defaultControls, loadWorkspaceControls, saveWorkspaceControls, shortcutFromEvent, shortcutLabel, type Controls, type ToolbarId } from './controls';
import { WorkspaceSettings, sectionLabel, toolbarLabel } from './WorkspaceSettings';
import {
  closeWorkspaceWindow, defaultStartup, initialLayout, loadWorkspace, newPanel, newTab,
  openWorkspaceWindow, saveStartup, saveWorkspace, snapshotOf, type Layout, type Panel, type Startup, type Tab, type TabLocationRequest,
} from './session';
import { applyLocale, strings } from './strings';
import { Favorites, Inspector, Tags } from './OrganizationPanel';
import { loadOrganization, organizationStrings, updateOrganization, type Favorite, type Organization, type OrganizationChange, type OrganizedItem, type SelectionDetails, type Tag } from './organization';
import { useNoteDrafts } from './useNoteDrafts';
import { ActivityToggle, WindowActivity, activityLabels, useTransferActivities } from './WindowActivity';
import './workspace.scss';
import './frosted-glass.scss';

const noTags: Tag[] = [];

function replacePanel(layout: Layout, panel: Panel): Layout {
  return { ...layout, panels: layout.panels.map(item => item.id === panel.id ? panel : item) };
}

function editPanel(layout: Layout | undefined, id: string, update: (panel: Panel) => Panel, activate = false) {
  const panel = layout?.panels.find(panel => panel.id === id);
  if (!layout || !panel) return layout;
  return replacePanel(activate ? { ...layout, active: id } : layout, update(panel));
}

function activateTab(panel: Panel, id: string): Panel {
  return { ...panel, active: id, tabs: panel.tabs.map(tab => tab.id === id ? { ...tab, mounted: true } : tab) };
}

function withoutTab(panel: Panel, closing: string): Panel {
  const next = panel.tabs.filter(tab => tab.id !== closing);
  const position = panel.tabs.findIndex(tab => tab.id === closing);
  const active = panel.active === closing ? next[Math.min(position, next.length - 1)]?.id : panel.active;
  return active ? activateTab({ ...panel, tabs: next }, active) : { ...panel, tabs: next };
}

function focusTab(id: string) {
  requestAnimationFrame(() => document.getElementById(`tab-${id}`)?.focus());
}

export default function Workspace() {
  const [startup, setStartup] = useState(defaultStartup);
  const [layout, setLayout] = useState<Layout>();
  const [sidebar, setSidebar] = useState(true);
  const [error, setError] = useState(false);
  const [tabMenu, setTabMenu] = useState<string | null>(null);
  const [tabDrag, setTabDrag] = useState<{ source: string; target?: string } | null>(null);
  const [controls, setControls] = useState(defaultControls);
  const [inspector, setInspector] = useState(false);
  const [inspectorWidth, setInspectorWidth] = useState(278);
  const inspectorResize = useRef<{ width: number; start: number } | null>(null);
  const [organization, setOrganization] = useState<Organization>();
  const [organizationError, setOrganizationError] = useState(false);
  const [selected, setSelected] = useState<SelectionDetails & { tabId: string }>();
  const [inspection, setInspection] = useState<{ selection: typeof selected; items: OrganizedItem[] }>();
  const [tagFilter, setTagFilter] = useState<string | null>(null);
  const [metadataVersion, setMetadataVersion] = useState(0);
  const notes = useNoteDrafts(() => setMetadataVersion(version => version + 1));
  const t = organizationStrings();
  const inspectorToggle = useRef<HTMLButtonElement>(null);
  const unsavedDialog = useRef<HTMLDialogElement>(null);
  const transfersDialog = useRef<HTMLDialogElement>(null);
  const activityReturn = useRef<HTMLButtonElement | null>(null);
  const [activityOpen, setActivityOpen] = useState(false);
  const { transfers, fileChanges } = useTransferActivities();
  const activity = activityLabels();
  const closing = useRef(false);
  const nativeClose = useRef<() => void>(() => {});
  const [retryingClose, setRetryingClose] = useState(false);
  const settingsDialog = useRef<HTMLDialogElement>(null);
  const controlSaveTail = useRef<Promise<void>>(Promise.resolve());
  const resize = useRef<{ width: number; start: number } | null>(null);
  const saveTail = useRef<Promise<void>>(Promise.resolve());
  const canSave = useRef(false);
  const active = layout?.panels.find(panel => panel.id === layout.active);
  const selection = selected?.tabId === active?.active ? selected : undefined;
  const filteredTag = organization?.tags.some(tag => tag.id === tagFilter) ? tagFilter : null;
  const hasLayout = Boolean(layout);

  const receiveSelection = useCallback((tabId: string, details: SelectionDetails) => {
    setSelected(current => current?.tabId === tabId && current.count === details.count && current.kinds === details.kinds
      && current.size === details.size && current.paths.length === details.paths.length
      && current.paths.every((path, index) => path === details.paths[index])
      ? current : { tabId, ...details });
  }, []);

  useEffect(() => {
    if (!hasLayout) return;
    let cancelled = false;
    let pending = false;
    async function refresh() {
      if (pending) return;
      pending = true;
      try {
        const data = await loadOrganization();
        if (!cancelled) setOrganization(data);
      } catch {
        if (!cancelled) setOrganizationError(true);
      } finally {
        pending = false;
      }
    }
    void refresh();
    const timer = setInterval(() => { if (!document.hidden) void refresh(); }, 4000);
    return () => { cancelled = true; clearInterval(timer); };
  }, [hasLayout, metadataVersion]);

  useEffect(() => {
    if (!inspector || !selection?.paths.length) return;
    const captured = selection;
    let cancelled = false;
    async function inspect() {
      try {
        const items: OrganizedItem[] = [];
        for (let offset = 0; offset < captured.paths.length && !cancelled; offset += 512) {
          const result = await loadOrganization(captured.paths.slice(offset, offset + 512), true, captured.count === 1);
          items.push(...result.items);
        }
        if (!cancelled) setInspection({ selection: captured, items });
      } catch {
        if (!cancelled) {
          setInspection({ selection: captured, items: [] });
          setOrganizationError(true);
        }
      }
    }
    void inspect();
    return () => { cancelled = true; };
  }, [inspector, selection, metadataVersion]);

  async function mutateOrganization(change: OrganizationChange) {
    setOrganizationError(false);
    try {
      await updateOrganization(change);
      setMetadataVersion(version => version + 1);
      return true;
    } catch {
      setOrganizationError(true);
      return false;
    }
  }

  useEffect(() => {
    applyLocale(defaultControls.language);
    let cancelled = false;
    async function start() {
      for (let attempt = 0; attempt < 6; attempt++) {
        try {
          const restored = await loadWorkspace();
          if (cancelled) return;
          canSave.current = true;
          setStartup(restored.startup);
          setLayout(initialLayout(restored.window, restored.startup));
          void loadWorkspaceControls().then(saved => {
            if (!cancelled) {
              applyLocale(saved.language);
              setControls(saved);
            }
          }).catch(() => {});
          return;
        } catch {
          await new Promise(resolve => setTimeout(resolve, 100));
          if (cancelled) return;
        }
      }
      setError(true);
      setLayout(initialLayout(null, defaultStartup));
    }
    void start();
    return () => { cancelled = true; };
  }, []);

  function applyControls(next: Controls) {
    applyLocale(next.language);
    setControls(next);
    setError(false);
    controlSaveTail.current = controlSaveTail.current.then(() => saveWorkspaceControls(next)).catch(() => setError(true));
  }

  useEffect(() => { document.documentElement.dataset.theme = controls.theme; }, [controls.theme]);

  const persist = useCallback((current: Layout) => {
    if (!canSave.current) return Promise.resolve();
    const saved = snapshotOf(current);
    saveTail.current = saveTail.current.then(() => saveWorkspace(saved)).catch(() => { setError(true); });
    return saveTail.current;
  }, []);

  useEffect(() => {
    if (!layout) return;
    const timer = setTimeout(() => { void persist(layout); }, 120);
    return () => clearTimeout(timer);
  }, [layout, persist]);

  async function requestClose(retry = false, discard = false, stopTransfers = false) {
    if (closing.current || !layout) return;
    closing.current = true;
    setRetryingClose(true);
    try {
      if (stopTransfers) {
        await invoke('stop_window_transfers');
        transfersDialog.current?.close();
      } else if (await invoke<boolean>('has_window_transfers')) {
        if (!transfersDialog.current?.open) transfersDialog.current?.showModal();
        return;
      }
      if (discard || await notes.flush(retry)) {
        await persist(layout);
        await closeWorkspaceWindow();
      } else if (!unsavedDialog.current?.open) {
        unsavedDialog.current?.showModal();
      }
    } catch {
      setError(true);
    } finally {
      closing.current = false;
      setRetryingClose(false);
    }
  }

  function closeActivity() {
    setActivityOpen(false);
    activityReturn.current?.focus();
  }

  useEffect(() => { nativeClose.current = () => void requestClose(); });
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void Promise.resolve().then(() => getCurrentWindow().listen('tauri://close-requested', () => nativeClose.current()))
      .then(stop => { if (disposed) stop(); else unlisten = stop; }).catch(() => {});
    return () => { disposed = true; unlisten?.(); };
  }, []);

  function newWorkspaceTab(path?: string, source = active) {
    if (!source) return;
    const tab = newTab(path ?? (startup.mode === 'home' ? null : startup.path), path === undefined);
    setLayout(current => editPanel(current, source.id, panel => ({
      ...panel, tabs: [...panel.tabs, tab], active: tab.id,
    }), true));
    focusTab(tab.id);
  }

  function openFavorite(favorite: Favorite) {
    if (!favorite.path) return;
    const target = favorite.isDirectory ? favorite.path : parentPath(favorite.path);
    if (!target) return;
    setTagFilter(null);
    navigateActiveTab({ path: target, selectPath: favorite.isDirectory ? undefined : favorite.path });
  }

  function closeInspector() {
    setInspector(false);
    inspectorToggle.current?.focus();
  }

  function resizeInspector(width: number) {
    setInspectorWidth(Math.min(480, Math.max(210, Math.round(width))));
  }

  function duplicateTab(panel: Panel, tab: Tab) {
    const duplicate = { ...newTab(tab.path), selected: tab.selected };
    setLayout(current => editPanel(current, panel.id, currentPanel => ({
      ...currentPanel, tabs: [...currentPanel.tabs, duplicate], active: duplicate.id,
    }), true));
    setTabMenu(null);
    focusTab(duplicate.id);
  }

  function closeTab(panel: Panel, id: string) {
    if (!layout) return;
    if (layout.panels.length === 1 && panel.tabs.length === 1) {
      void requestClose();
      return;
    }
    const remaining = withoutTab(panel, id);
    if (remaining.tabs.length) {
      setLayout(replacePanel(layout, remaining));
      focusTab(remaining.active);
    }
    else {
      const panels = layout.panels.filter(item => item.id !== panel.id);
      setLayout({ panels, active: panels[0].id });
      focusTab(panels[0].active);
    }
    setTabMenu(null);
  }

  function openWindow(path: string | null) {
    void openWorkspaceWindow(path).catch(() => setError(true));
  }

  useEffect(() => {
    const shortcuts = (event: KeyboardEvent) => {
      if (document.querySelector('dialog[open]') || event.defaultPrevented) return;
      switch (shortcutFromEvent(event)) {
        case controls.shortcuts.newTab: event.preventDefault(); newWorkspaceTab(); break;
        case controls.shortcuts.closeTab: if (active) { event.preventDefault(); closeTab(active, active.active); } break;
        case controls.shortcuts.newWindow: event.preventDefault(); openWindow(null); break;
      }
    };
    window.addEventListener('keydown', shortcuts);
    return () => window.removeEventListener('keydown', shortcuts);
  });

  const update = useCallback((id: string, path: string, selected: string | null) => {
    setLayout(current => {
      if (!current) return current;
      const panel = current.panels.find(panel => panel.tabs.some(tab => tab.id === id));
      const tab = panel?.tabs.find(tab => tab.id === id);
      if (!panel || !tab || (tab.path === path && tab.selected === selected)) return current;
      return replacePanel(current, { ...panel, tabs: panel.tabs.map(tab => tab.id === id ? { ...tab, path, selected } : tab) });
    });
  }, []);

  function splitOrMerge() {
    if (!layout || !active) return;
    if (layout.panels.length === 1) {
      const next = newPanel(newTab(startup.mode === 'home' ? null : startup.path, true));
      setLayout({ panels: [...layout.panels, next], active: next.id });
    } else {
      const panel = { ...layout.panels[0], active: active.active, tabs: layout.panels.flatMap(panel => panel.tabs) };
      setLayout({ panels: [panel], active: panel.id });
    }
  }

  function moveTab(panel: Panel, tab: Tab) {
    if (!layout) return;
    const destination = layout.panels.find(item => item.id !== panel.id);
    if (!destination) return;
    const source = withoutTab(panel, tab.id);
    const moved = { ...destination, active: tab.id, tabs: [...destination.tabs, { ...tab, mounted: true }] };
    const panels = layout.panels.map(item => item.id === source.id ? source : moved).filter(panel => panel.tabs.length > 0);
    setLayout({ panels, active: moved.id });
    setTabMenu(null);
    focusTab(tab.id);
  }

  function selectTab(panel: Panel, id: string) {
    setLayout(current => editPanel(current, panel.id, currentPanel => activateTab(currentPanel, id), true));
  }

  function closeMany(panel: Panel, tab: Tab, toRight: boolean) {
    setLayout(current => editPanel(current, panel.id, currentPanel => {
      const index = currentPanel.tabs.findIndex(other => other.id === tab.id);
      if (index < 0) return currentPanel;
      const tabs = toRight ? currentPanel.tabs.slice(0, index + 1) : [currentPanel.tabs[index]];
      return activateTab({ ...currentPanel, tabs }, tab.id);
    }, true));
  }

  function reorder(panel: Panel, id: string, next: number) {
    setLayout(current => editPanel(current, panel.id, currentPanel => {
      const tabs = [...currentPanel.tabs];
      const index = tabs.findIndex(tab => tab.id === id);
      if (index < 0 || next < 0 || next >= tabs.length || index === next) return currentPanel;
      tabs.splice(next, 0, ...tabs.splice(index, 1));
      return { ...currentPanel, tabs };
    }));
  }

  function navigateActiveTab(locationRequest: TabLocationRequest) {
    setLayout(current => current && editPanel(current, current.active, panel => ({
      ...panel, tabs: panel.tabs.map(tab => tab.id === panel.active ? { ...tab, locationRequest } : tab),
    })));
  }

  function home() {
    navigateActiveTab({ path: null });
  }

  async function saveStartupSettings(startup: Startup) {
    try {
      await saveStartup(startup);
      setStartup(startup);
      setError(false);
    } catch {
      setError(true);
    }
  }

  function beginSidebarResize(event: PointerEvent<HTMLDivElement>) {
    event.currentTarget.setPointerCapture(event.pointerId);
    resize.current = { width: controls.sidebarWidth, start: event.clientX };
  }

  function resizeSidebar(event: PointerEvent<HTMLDivElement>, commit = false) {
    if (!resize.current) return;
    const width = Math.min(480, Math.max(120, Math.round(resize.current.width + event.clientX - resize.current.start)));
    const next = { ...controls, sidebarWidth: width };
    if (commit) {
      resize.current = null;
      applyControls(next);
    } else setControls(next);
  }

  const actions: Record<ToolbarId, () => void> = {
    'new-tab': () => newWorkspaceTab(),
    'new-window': () => openWindow(null),
    'close-tab': () => { if (active) closeTab(active, active.active); },
    split: splitOrMerge,
    sidebar: () => setSidebar(value => !value),
  };

  return (
    <div className="explorer" data-density={controls.density}
      style={{ '--sidebar-width': controls.sidebarWidth + 'px', '--inspector-width': inspectorWidth + 'px' } as CSSProperties}>
      <header className="workspace-chrome" data-mac-titlebar={/mac/i.test(navigator.platform)}>
        <div className="workspace-window-drag" data-tauri-drag-region aria-hidden="true" />
        <span className="workspace-mark" aria-hidden="true"><Layers /></span>
        <div className="workspace-tab-bars" data-split={layout?.panels.length === 2}>
          {layout?.panels.map((panel, paneIndex) => (
            <div className="workspace-tab-group" key={panel.id} data-focused={panel.id === active?.id}>
              <div className="workspace-tab-strip" role="tablist" aria-label={strings.pane(paneIndex + 1)}>
                {panel.tabs.map((tab, index) => (
                  <span className="workspace-tab-item" key={tab.id}
                    data-tab-drag-source={tabDrag?.source === tab.id} data-tab-drop-target={tabDrag?.target === tab.id}
                    onDragOver={event => {
                      event.preventDefault();
                      if (tabDrag && tabDrag.source !== tab.id && tabDrag.target !== tab.id
                        && panel.tabs.some(candidate => candidate.id === tabDrag.source)
                        && event.dataTransfer.types.includes('application/x-folderai-tab')) {
                        setTabDrag({ source: tabDrag.source, target: tab.id });
                      }
                    }}
                    onDragLeave={event => {
                      if (event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget)) return;
                      setTabDrag(current => current?.target === tab.id ? { source: current.source } : current);
                    }}
                    onDrop={event => {
                      setTabDrag(null);
                      reorder(panel, event.dataTransfer.getData('application/x-folderai-tab'), index);
                    }}>
                    <button id={`tab-${tab.id}`} type="button" role="tab" aria-selected={panel.active === tab.id}
                      aria-controls={`pane-${tab.id}`} className="workspace-tab" title={tab.path ?? 'Home'}
                      tabIndex={panel.active === tab.id ? 0 : -1} draggable
                      onDragStart={event => {
                        event.dataTransfer.setData('application/x-folderai-tab', tab.id);
                        setTabDrag({ source: tab.id });
                      }}
                      onDragEnd={() => setTabDrag(null)}
                      onClick={() => selectTab(panel, tab.id)}
                      onContextMenu={event => { event.preventDefault(); setTabMenu(tab.id); }}
                      onKeyDown={event => {
                        if (event.key === 'ContextMenu' || (event.key === 'F10' && event.shiftKey)) {
                          event.preventDefault();
                          setTabMenu(tab.id);
                          return;
                        }
                        const offset = event.key === 'ArrowLeft' ? -1 : event.key === 'ArrowRight' ? 1 : 0;
                        if (!offset) return;
                        event.preventDefault();
                        if (event.altKey) reorder(panel, tab.id, index + offset);
                        else {
                          const next = panel.tabs[(index + panel.tabs.length + offset) % panel.tabs.length];
                          selectTab(panel, next.id);
                          document.getElementById(`tab-${next.id}`)?.focus();
                        }
                      }}>
                      <Folder aria-hidden="true" /><span>{pathLabel(tab.path)}</span>
                    </button>
                    <button type="button" className="tab-close" aria-label={strings.closeTab}
                      title={strings.closeTab} onClick={() => closeTab(panel, tab.id)}><X aria-hidden="true" /></button>
                  </span>
                ))}
              </div>
              <button type="button" className="tool-button" aria-label={strings.newTab} title={strings.newTab} onClick={() => newWorkspaceTab(undefined, panel)}><Plus /></button>
            </div>
          ))}
        </div>
        <button type="button" className="tool-button" disabled={!layout} aria-label={layout?.panels.length === 2 ? strings.merge : strings.split}
          title={layout?.panels.length === 2 ? strings.merge : strings.split} onClick={splitOrMerge}><Columns2 /></button>
        {layout?.panels.length === 2 && (
          <button type="button" className="panel-switch" onClick={() => {
            const other = layout.panels.find(panel => panel.id !== layout.active);
            if (other) setLayout({ ...layout, active: other.id });
          }}>{strings.switchPanel}</button>
        )}
        <button type="button" className="tool-button" aria-label={strings.sidebar} title={strings.sidebar}
          aria-expanded={sidebar} aria-controls="workspace-sidebar" onClick={() => setSidebar(!sidebar)}><PanelLeft /></button>
        <button type="button" ref={inspectorToggle} className="tool-button" aria-label={t.inspector} title={t.inspector}
          aria-expanded={inspector} aria-controls="organization-inspector" onClick={() => setInspector(value => !value)}><PanelRight /></button>
        <button type="button" className="tool-button" aria-label={strings.newWindow} title={shortcutLabel(controls.shortcuts.newWindow)}
          onClick={() => openWindow(null)}><AppWindow /></button>
      </header>
      {controls.toolbar.some(action => Object.hasOwn(actions, action)) &&
        <div className="workspace-pinned" role="toolbar" aria-label={strings.settings}>
          {controls.toolbar.flatMap(action => Object.hasOwn(actions, action) ? [
            <button type="button" key={action} onClick={actions[action as ToolbarId]}>{toolbarLabel(action as ToolbarId)}</button>,
          ] : [])}
        </div>}
      {layout && tabMenu && layout.panels.map(panel => {
        const tab = panel.tabs.find(tab => tab.id === tabMenu);
        if (!tab) return null;
        const dismiss = () => {
          setTabMenu(null);
          focusTab(tab.id);
        };
        return <div className="tab-action-menu" key={tab.id} role="group" aria-label={strings.tabActions}
          onKeyDown={event => { if (event.key === 'Escape') dismiss(); }}>
          <button type="button" autoFocus onClick={() => duplicateTab(panel, tab)}><Copy aria-hidden="true" />{strings.duplicateTab}</button>
          {layout.panels.length > 1 && <button type="button" onClick={() => moveTab(panel, tab)}><ArrowRight aria-hidden="true" />{strings.moveTab}</button>}
          <button type="button" onClick={() => { closeMany(panel, tab, false); dismiss(); }}><X aria-hidden="true" />{strings.closeOtherTabs}</button>
          <button type="button" onClick={() => { closeMany(panel, tab, true); dismiss(); }}><X aria-hidden="true" />{strings.closeRightTabs}</button>
          <button type="button" onClick={() => closeTab(panel, tab.id)}><X aria-hidden="true" />{strings.closeTab}</button>
          <button type="button" onClick={dismiss}><X aria-hidden="true" />{strings.cancel}</button>
        </div>;
      })}
      {error && <p className="workspace-feedback" role="status">{strings.workspaceError}</p>}
      {organizationError && <div className="workspace-feedback organization-row" role="status">
        <span>{t.failure}</span><button type="button" onClick={() => {
          setOrganizationError(false);
          setMetadataVersion(value => value + 1);
        }}>{t.retry}</button>
      </div>}
      <div className="workspace-body">
        {sidebar && <>
          <aside className="workspace-sidebar" id="workspace-sidebar" aria-label={strings.sidebar}>
            {controls.sidebar.filter(section => section.visible).map(section => (
              <details key={section.id} open={section.expanded} onToggle={event => {
                const expanded = event.currentTarget.open;
                if (expanded === section.expanded) return;
                applyControls({ ...controls, sidebar: controls.sidebar.map(row => row.id === section.id ? { ...row, expanded } : row) });
              }}>
                <summary>{sectionLabel(section.id)}</summary>
                {section.id === 'locations' && <button type="button" className="home-position" onClick={home}><House />Home</button>}
                {section.id === 'favorites' && organization && <Favorites data={organization} mutate={mutateOrganization} open={openFavorite} />}
                {section.id === 'tags' && organization && <Tags data={organization} mutate={mutateOrganization}
                  filter={filteredTag} setFilter={setTagFilter} />}
              </details>
            ))}
            <button type="button" className="home-position workspace-settings"
              onClick={() => settingsDialog.current?.showModal()}><Settings />{strings.settings}</button>
          </aside>
          <div className="sidebar-resize" role="separator" aria-orientation="vertical" aria-label={strings.sidebar}
            aria-valuemin={120} aria-valuemax={480} aria-valuenow={controls.sidebarWidth} tabIndex={0}
            onPointerDown={beginSidebarResize} onPointerMove={event => resizeSidebar(event)}
            onPointerUp={event => resizeSidebar(event, true)} onPointerCancel={() => { resize.current = null; }}
            onKeyDown={event => {
              if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
              event.preventDefault();
              applyControls({ ...controls, sidebarWidth: Math.max(120, Math.min(480, controls.sidebarWidth + (event.key === 'ArrowLeft' ? -10 : 10))) });
            }} />
        </>}
        <main className="workspace-panels" aria-label={strings.explorer}>
          {!layout && <p className="folder-empty">{strings.loading}</p>}
          {layout?.panels.flatMap((panel, panelIndex) => panel.tabs.map(tab => !tab.mounted
            ? <section key={tab.id} id={`pane-${tab.id}`} aria-labelledby={`tab-${tab.id}`} role="tabpanel" hidden />
            : <FolderPane key={tab.id} tab={tab} panelId={panel.id} panelIndex={panelIndex}
              focused={panel.id === layout.active && tab.id === panel.active} visible={tab.id === panel.active}
              onFocus={() => setLayout(current => current && current.active !== panel.id ? { ...current, active: panel.id } : current)}
              onUpdate={update} onSelection={receiveSelection} tagFilter={filteredTag} tags={organization?.tags ?? noTags}
              clearFilter={() => setTagFilter(null)}
              metadataVersion={metadataVersion} fileChanges={fileChanges}
              activityControl={panel.id === layout.active && tab.id === panel.active ? <ActivityToggle
                transfers={transfers} open={activityOpen} toggle={event => {
                  activityReturn.current = event.currentTarget;
                  setActivityOpen(value => !value);
                }} /> : null}
              openTab={path => newWorkspaceTab(path, panel)} openWindow={openWindow} shortcuts={controls.shortcuts} />))}
        </main>
        {inspector && organization && <>
          <div className="sidebar-resize inspector-resize" role="separator" aria-orientation="vertical" aria-label={t.resize}
            aria-valuemin={210} aria-valuemax={480} aria-valuenow={inspectorWidth} tabIndex={0}
            onPointerDown={event => {
              event.currentTarget.setPointerCapture(event.pointerId);
              inspectorResize.current = { width: inspectorWidth, start: event.clientX };
            }}
            onPointerMove={event => {
              const origin = inspectorResize.current;
              if (origin) resizeInspector(origin.width + origin.start - event.clientX);
            }}
            onLostPointerCapture={() => { inspectorResize.current = null; }}
            onKeyDown={event => {
              if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
              event.preventDefault();
              resizeInspector(inspectorWidth + (event.key === 'ArrowLeft' ? 10 : -10));
            }} />
          <Inspector data={organization} selected={selection}
          items={inspection && inspection.selection === selection ? inspection.items : []}
          loading={Boolean(selection?.paths.length && inspection?.selection !== selection)} close={closeInspector}
          mutate={mutateOrganization} notes={notes} />
        </>}
      </div>
      <WorkspaceSettings dialog={settingsDialog} startup={startup} saveStartup={saveStartupSettings}
        controls={controls} apply={applyControls} error={error || organizationError} copyMetadata={organization?.copyMetadata}
        setCopyMetadata={enabled => void mutateOrganization({ kind: 'set-copy-metadata', enabled })} />
      {activityOpen && <WindowActivity transfers={transfers} close={closeActivity} />}
      <dialog ref={transfersDialog} className="organization-dialog" aria-label={activity.closeQuestion}
        onCancel={event => { if (retryingClose) event.preventDefault(); }}>
        <p>{activity.closeQuestion}</p>
        <div className="organization-row organization-form">
          <button type="button" autoFocus disabled={retryingClose}
            onClick={() => transfersDialog.current?.close()}>{activity.stay}</button>
          <button type="button" disabled={retryingClose}
            onClick={() => void requestClose(false, false, true)}>{activity.stopAndClose}</button>
        </div>
        {retryingClose && <p role="status">{activity.phase.stopping}</p>}
      </dialog>
      <dialog ref={unsavedDialog} className="organization-dialog" aria-label={t.pendingNotes}>
        <p>{t.pendingNotes}</p>
        <div className="organization-row organization-form">
          <button type="button" autoFocus onClick={() => {
            unsavedDialog.current?.close();
            setInspector(true);
          }}>{t.stay}</button>
          <button type="button" disabled={retryingClose} onClick={() => void requestClose(true)}>{t.retry}</button>
          <button type="button" disabled={retryingClose} onClick={() => void requestClose(false, true)}>{t.discard}</button>
        </div>
      </dialog>
    </div>
  );
}
