import { invoke } from '@tauri-apps/api/core';
import {
  ArrowDown,
  ArrowLeft,
  ArrowRight,
  ArrowUp,
  ChevronRight,
  File,
  Folder,
  Link2,
  Pencil,
  RotateCw,
  Shield,
  X,
} from 'lucide-react';
import {
  Fragment,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type FormEvent,
  type KeyboardEvent,
  type ComponentProps,
  type CSSProperties,
  type MouseEvent,
  type PointerEvent,
  type ReactNode,
} from 'react';
import type { Directory } from '../ipc';
import { ColumnPreview } from './ColumnPreview';
import { fileActionLabels, fileErrorCode, FileMutationControls, type FileActionContext, type FileActionSelection } from './FileMutationControls';
import { defaultView, displayName, fileExtension, loadWorkspaceViews, saveWorkspaceView, sortEntries, sortIds, type ColumnId, type ViewOptions as Options } from './fileViews';
import { ViewOptions, getViewStrings } from './ViewOptions';
import { defaultControls, shortcutFromEvent, type Controls } from './controls';
import { useWorkspaceNavigation, type Destination } from './navigation';
import { parentPath, pathLabel } from './paths';
import { lookupOrganizationTags, organizationStrings, type PathMetadata, type SelectionDetails, type Tag } from './organization';
import type { Tab } from './session';
import { locale, strings } from './strings';
import './workspace.scss';

type Entry = Directory['entries'][number];

const formats = {
  en: { date: new Intl.DateTimeFormat('en', { dateStyle: 'medium' }), number: new Intl.NumberFormat('en', { maximumFractionDigits: 1 }) },
  it: { date: new Intl.DateTimeFormat('it', { dateStyle: 'medium' }), number: new Intl.NumberFormat('it', { maximumFractionDigits: 1 }) },
};
const sizeUnits = ['B', 'kB', 'MB', 'GB', 'TB', 'PB'];

function readableSize(size: number | null) {
  if (size === null) return '—';
  let unit = 0;
  while (size >= 1000 && unit < sizeUnits.length - 1) {
    size /= 1000;
    unit++;
  }
  return `${formats[locale].number.format(size)} ${sizeUnits[unit]}`;
}

function readableDate(time: number | null) {
  if (time === null) return '—';
  const date = new Date(time);
  if (!Number.isFinite(date.getTime())) return '—';
  return <time dateTime={date.toISOString()}>{formats[locale].date.format(date)}</time>;
}

function entryKey(entry: Entry, index: number) {
  return entry.location?.path ?? `unavailable-${index}`;
}

function fileActionSelection(entry: Entry): FileActionSelection | null {
  return entry.location && (entry.kind === 'file' || entry.kind === 'directory')
    ? { path: entry.location.path, kind: entry.kind, name: entry.name }
    : null;
}

function isContextKey(event: KeyboardEvent) {
  return event.key === 'ContextMenu' || (event.key === 'F10' && event.shiftKey);
}

function ToolButton({ label, children, ...props }: ComponentProps<'button'> & { label: string; children: ReactNode }) {
  return (
    <button type="button" className="tool-button" title={label} aria-label={label} {...props}>
      {children}
    </button>
  );
}

type PaneProps = {
  tab: Tab;
  panelId: string;
  panelIndex: number;
  focused: boolean;
  visible: boolean;
  onFocus: () => void;
  onUpdate: (id: string, path: string, selected: string | null) => void;
  onSelection: (tabId: string, details: SelectionDetails) => void;
  tagFilter: string | null;
  tags: Tag[];
  metadataVersion: number;
  fileChanges: ReadonlyMap<string, number>;
  activityControl: import('react').ReactNode;
  clearFilter: () => void;
  openTab: (path: string) => void;
  openWindow: (path: string) => void;
  shortcuts?: Controls['shortcuts'];
};

const workspaceFileDrag = 'application/x-folderai-entry';
let draggedEntry: FileActionSelection | null = null;

export default function FolderPane({ tab, panelId, panelIndex, focused, visible, onFocus, onUpdate, onSelection, tagFilter, tags, metadataVersion, fileChanges, activityControl, clearFilter, openTab, openWindow, shortcuts = defaultControls.shortcuts }: PaneProps) {
  const viewStrings = getViewStrings();
  const collator = useMemo(() => new Intl.Collator(locale, { numeric: true, sensitivity: 'base' }), [locale]);
  const { navigation, open, cancelAddress, usedFallback } = useWorkspaceNavigation(panelId, tab.id, tab.start, tab.fallback, tab.selected);
  const { directory, visits, index, failure, pending } = navigation;
  const [editing, setEditing] = useState(false);
  const [address, setAddress] = useState('');
  const [selected, setSelected] = useState<string | null>(null);
  const [selectedRows, setSelectedRows] = useState<string[]>([]);
  const selection = useMemo(() => new Set(selectedRows), [selectedRows]);
  const rangeAnchor = useRef<string | null>(null);
  const [fileContext, setFileContext] = useState<FileActionContext | null>(null);
  const [previewRefresh, setPreviewRefresh] = useState(0);
  const dismissContext = useCallback(() => setFileContext(null), []);
  const [view, setView] = useState<{ path: string | null; options: Options }>({ path: null, options: defaultView });
  const [folderReference, setFolderReference] = useState<{ path: string | null; id: string | null }>();
  const [viewError, setViewError] = useState(false);
  const writeQueue = useRef<Promise<void>>(Promise.resolve());
  const editedView = useRef(0);
  const hiddenApplied = useRef(false);
  const dragOrigin = useRef<{ x: number; y: number; pointer: number } | null>(null);
  const [dragBox, setDragBox] = useState<{ x: number; y: number; width: number; height: number } | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const tableRef = useRef<HTMLDivElement>(null);
  const breadcrumbRef = useRef<HTMLOListElement>(null);
  const addressRef = useRef<HTMLInputElement>(null);
  const editRef = useRef<HTMLButtonElement>(null);
  const rowButtons = useRef(new Map<string, HTMLButtonElement>());

  const folderPath = directory?.location.path ?? null;
  const observedChanges = useRef(new Map<string, number>());
  useEffect(() => {
    if (!folderPath || !visible || pending) return;
    const version = fileChanges.get(folderPath) ?? 0;
    if (version === (observedChanges.current.get(folderPath) ?? 0)) return;
    observedChanges.current.set(folderPath, version);
    navigate({ target: { kind: 'path', path: folderPath }, source: 'refresh' });
  });
  useLayoutEffect(dismissContext, [dismissContext, folderPath, focused, visible, pending]);
  const option = view.path === folderPath ? view.options : defaultView;
  const sorted = useMemo(() => sortEntries(directory?.entries ?? [], option, collator), [directory, option, collator]);
  const [metadata, setMetadata] = useState<{ entries: Entry[]; paths: Map<string, PathMetadata> | null }>();
  const wantMetadata = Boolean(tagFilter) || (option.mode === 'details'
    && option.columns.some(column => column.visible && (column.id === 'tags' || column.id === 'note')));
  const settled = metadata?.entries === sorted ? metadata : undefined;
  const entries = useMemo(() => tagFilter
    ? sorted.filter(entry => entry.location && settled?.paths?.get(entry.location.path)?.tagIds.includes(tagFilter))
    : sorted, [tagFilter, sorted, settled]);
  const selectionDetails = useMemo<SelectionDetails>(() => {
    const rows = entries.filter((entry, index) => selection.has(entryKey(entry, index)));
    if (!rows.length) return { paths: folderPath ? [folderPath] : [], count: folderPath ? 1 : 0, kinds: folderPath ? strings.folder : '', size: null };
    const kinds = new Map<Entry['kind'], number>();
    let size = 0;
    let sized = false;
    for (const row of rows) {
      kinds.set(row.kind, (kinds.get(row.kind) ?? 0) + 1);
      if (row.kind !== 'directory' && row.size !== null) { size += row.size; sized = true; }
    }
    return {
      paths: rows.flatMap(row => row.location ? [row.location.path] : []),
      count: rows.length,
      kinds: [...kinds].map(([kind, count]) => `${kind === 'directory' ? strings.folder : strings[kind]} · ${count}`).join(', '),
      size: sized ? readableSize(size) : null,
    };
  }, [entries, selection, folderPath, collator]);

  useEffect(() => {
    if (focused && visible) onSelection(tab.id, selectionDetails);
  }, [focused, visible, tab.id, selectionDetails, onSelection]);

  useEffect(() => {
    if (!wantMetadata || !visible) return;
    let cancelled = false;
    async function filterEntries() {
      const paths = sorted.flatMap(entry => entry.location ? [entry.location.path] : []);
      const metadata = new Map<string, PathMetadata>();
      try {
        for (let offset = 0; offset < paths.length && !cancelled; offset += 512) {
          const result = await lookupOrganizationTags(paths.slice(offset, offset + 512));
          for (const item of result) metadata.set(item.path, item);
        }
        if (!cancelled) setMetadata({ entries: sorted, paths: metadata });
      } catch {
        if (!cancelled) setMetadata({ entries: sorted, paths: null });
      }
    }
    void filterEntries();
    return () => { cancelled = true; };
  }, [sorted, wantMetadata, metadataVersion, visible]);

  useEffect(() => {
    let cancelled = false;
    const captured = editedView.current;
    void loadWorkspaceViews(folderPath).then(saved => {
      if (cancelled) return;
      setFolderReference({ path: folderPath, id: saved.folderIdentity });
      if (captured === editedView.current) setView({ path: folderPath, options: saved.folder ?? saved.global });
    }).catch(() => {});
    return () => { cancelled = true; };
  }, [folderPath, directory]);

  useEffect(() => {
    if (!folderPath || option.showHidden === hiddenApplied.current) return;
    hiddenApplied.current = option.showHidden;
    void open({ target: { kind: 'path', path: folderPath }, source: 'refresh', showHidden: option.showHidden }, {
      selected, selectedRows, scrollTop: listRef.current?.scrollTop ?? 0, scrollLeft: tableRef.current?.scrollLeft ?? 0,
    });
  }, [folderPath, open, option.showHidden, selected, selectedRows]);

  function updateOptions(options: Options, scope = folderPath) {
    editedView.current++;
    setView({ path: folderPath, options });
    setViewError(false);
    const identity = scope && folderReference?.path === scope ? folderReference.id : null;
    if (scope && !identity) {
      setViewError(true);
      return;
    }
    writeQueue.current = writeQueue.current.then(() => saveWorkspaceView(scope, options, identity)).catch(() => setViewError(true));
  }

  function resetOptions() {
    if (!folderPath) return;
    editedView.current++;
    setViewError(false);
    const revision = editedView.current;
    const identity = folderReference?.path === folderPath ? folderReference.id : null;
    writeQueue.current = writeQueue.current.then(async () => {
      if (identity) await saveWorkspaceView(folderPath, null, identity);
      const saved = await loadWorkspaceViews(folderPath);
      if (revision === editedView.current) setView({ path: folderPath, options: saved.global });
    }).catch(() => setViewError(true));
  }

  useEffect(() => {
    function toggleHidden(event: globalThis.KeyboardEvent) {
      if (!focused || !visible || event.defaultPrevented || editing || document.querySelector('dialog[open]')) return;
      if (shortcutFromEvent(event) !== shortcuts.hidden) return;
      event.preventDefault();
      updateOptions({ ...option, showHidden: !option.showHidden });
    }
    window.addEventListener('keydown', toggleHidden);
    return () => window.removeEventListener('keydown', toggleHidden);
  });

  const breadcrumbs = directory?.breadcrumbs ?? [];
  const fallbackParent = !directory && tab.start && failure ? parentPath(tab.start) : null;
  const parent = breadcrumbs.at(-2) ?? (fallbackParent ? { path: fallbackParent, label: fallbackParent } : null);
  const currentPath = directory?.location.path ?? tab.start ?? '';
  const currentLabel = directory?.location.label ?? pathLabel(currentPath);
  const addressFailure = failure?.destination.source === 'address';

  useEffect(() => {
    function editPathShortcut(event: globalThis.KeyboardEvent) {
      if (!focused || !visible || shortcutFromEvent(event) !== shortcuts.address || event.defaultPrevented
        || document.querySelector('dialog[open]')) return;
      event.preventDefault();
      if (editing) {
        addressRef.current?.focus();
        addressRef.current?.select();
      } else {
        setAddress(currentPath);
        setEditing(true);
      }
    }
    window.addEventListener('keydown', editPathShortcut);
    return () => window.removeEventListener('keydown', editPathShortcut);
  }, [editing, currentPath, focused, visible, shortcuts.address]);

  useEffect(() => {
    if (!directory) return;
    const selectedPath = entries.find((entry, index) => entryKey(entry, index) === selected)?.location?.path ?? null;
    onUpdate(tab.id, directory.location.path, selectedPath);
  }, [directory, entries, onUpdate, selected, tab.id]);

  useLayoutEffect(() => {
    const view = navigation.view;
    if (!view) return;
    const keys = new Set(entries.map(entryKey));
    const persistedSelection = view.selectedRows ?? (view.selected ? [view.selected] : []);
    const available = persistedSelection.filter(key => keys.has(key));
    const selected = view.selected && available.includes(view.selected) ? view.selected : available.at(-1) ?? null;
    setSelected(selected);
    setSelectedRows(available);
    rangeAnchor.current = selected;
    if (listRef.current) listRef.current.scrollTop = view.scrollTop;
    if (tableRef.current) tableRef.current.scrollLeft = view.scrollLeft;
  }, [navigation.view]); // Ordering and view controls do not reset the live selection.

  useLayoutEffect(() => {
    if (breadcrumbRef.current) breadcrumbRef.current.scrollLeft = breadcrumbRef.current.scrollWidth;
  }, [currentPath, editing, visible]);

  useLayoutEffect(() => {
    if (editing) {
      addressRef.current?.focus();
      addressRef.current?.select();
    }
  }, [editing]);

  const visit = useCallback((destination: Destination) => {
    return open({ ...destination, showHidden: option.showHidden }, {
      selected,
      selectedRows,
      scrollTop: listRef.current?.scrollTop ?? 0,
      scrollLeft: tableRef.current?.scrollLeft ?? 0,
    });
  }, [open, option.showHidden, selected, selectedRows]);

  const lastLocationRequest = useRef<Tab['locationRequest']>(undefined);
  useEffect(() => {
    const request = tab.locationRequest;
    if (!request || lastLocationRequest.current === request) return;
    lastLocationRequest.current = request;
    dismissContext();
    setEditing(false);
    void visit({
      target: request.path ? { kind: 'path', path: request.path } : { kind: 'home' },
      source: 'browse',
      selectPath: request.selectPath,
    });
  }, [dismissContext, tab.locationRequest, visit]);

  function navigate(destination: Destination) {
    dismissContext();
    setEditing(false);
    void visit(destination);
  }

  function goTo(path: string) {
    navigate({ target: { kind: 'path', path }, source: 'browse' });
  }

  function goToHistory(historyIndex: number) {
    const location = visits[historyIndex]?.location;
    if (location) navigate({ target: { kind: 'path', path: location.path }, source: 'browse', historyIndex });
  }

  function startAddress() {
    setAddress(currentPath);
    setEditing(true);
  }

  function closeAddress() {
    cancelAddress();
    setEditing(false);
    requestAnimationFrame(() => editRef.current?.focus());
  }

  async function submitAddress(event: FormEvent) {
    event.preventDefault();
    if (await visit({ target: { kind: 'path', path: address }, source: 'address' })) {
      setEditing(false);
      requestAnimationFrame(() => editRef.current?.focus());
    }
  }

  function selectRow(entry: Entry, entryIndex: number, modifiers?: { shiftKey: boolean; metaKey: boolean; ctrlKey: boolean }) {
    const key = entryKey(entry, entryIndex);
    dismissContext();
    const keys = entries.map(entryKey);
    if (modifiers?.shiftKey && rangeAnchor.current && keys.includes(rangeAnchor.current)) {
      const anchor = keys.indexOf(rangeAnchor.current);
      const range = keys.slice(Math.min(anchor, entryIndex), Math.max(anchor, entryIndex) + 1);
      setSelectedRows(current => modifiers.metaKey || modifiers.ctrlKey ? Array.from(new Set([...current, ...range])) : range);
    } else if (modifiers?.metaKey || modifiers?.ctrlKey) {
      setSelectedRows(current => current.includes(key) ? current.filter(item => item !== key) : [...current, key]);
      rangeAnchor.current = key;
    } else {
      setSelectedRows([key]);
      rangeAnchor.current = key;
    }
    setSelected(key);
    rowButtons.current.get(key)?.focus({ preventScroll: true });
  }

  function openRow(entry: Entry) {
    if ((entry.kind === 'directory' || entry.kind === 'link') && entry.location) {
      listRef.current?.focus({ preventScroll: true });
      goTo(entry.location.path);
    }
  }

  function fileKeyDown(event: KeyboardEvent, entryIndex: number) {
    const entry = entries[entryIndex];
    if (isContextKey(event)) {
      openItemContext(event, entry, entryIndex);
      return;
    }
    if (option.mode === 'columns' && event.key === 'ArrowRight') {
      event.preventDefault();
      event.currentTarget.closest('.file-column')?.nextElementSibling
        ?.querySelector<HTMLButtonElement>('.column-entry')?.focus();
      return;
    }
    if (option.mode === 'grid' && event.key.startsWith('Arrow')) {
      event.preventDefault();
      const origin = event.currentTarget.getBoundingClientRect();
      const x = origin.left + origin.width / 2;
      const y = origin.top + origin.height / 2;
      const vertical = event.key === 'ArrowUp' || event.key === 'ArrowDown';
      const direction = event.key === 'ArrowUp' || event.key === 'ArrowLeft' ? -1 : 1;
      let best: { index: number; primary: number; secondary: number } | null = null;
      for (const [index, candidate] of entries.entries()) {
        const bounds = rowButtons.current.get(entryKey(candidate, index))?.getBoundingClientRect();
        if (!bounds) continue;
        const dx = bounds.left + bounds.width / 2 - x;
        const dy = bounds.top + bounds.height / 2 - y;
        const primary = (vertical ? dy : dx) * direction;
        const secondary = Math.abs(vertical ? dx : dy);
        if (primary < 4 || (!vertical && secondary > 4)) continue;
        if (!best || primary < best.primary || (primary === best.primary && secondary < best.secondary)) {
          best = { index, primary, secondary };
        }
      }
      if (best) {
        selectRow(entries[best.index], best.index, event);
        rowButtons.current.get(entryKey(entries[best.index], best.index))?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
      }
      return;
    }
    let next: number;
    switch (event.key) {
      case 'ArrowUp': next = Math.max(0, entryIndex - 1); break;
      case 'ArrowDown': next = Math.min(entries.length - 1, entryIndex + 1); break;
      case 'Home': next = 0; break;
      case 'End': next = entries.length - 1; break;
      case 'Enter':
        event.preventDefault();
        selectRow(entries[entryIndex], entryIndex);
        openRow(entries[entryIndex]);
        return;
      default: return;
    }
    event.preventDefault();
    const target = entries[next];
    selectRow(target, next, event);
    rowButtons.current.get(entryKey(target, next))?.scrollIntoView({ block: 'nearest' });
  }

  function rowName(entry: Entry, entryIndex: number) {
    const key = entryKey(entry, entryIndex);
    const Icon = entry.kind === 'directory' ? Folder : entry.kind === 'link' ? Link2 : File;
    return (
      <>
        <button ref={node => { if (node) rowButtons.current.set(key, node); else rowButtons.current.delete(key); }}
          className="file-name" type="button" aria-pressed={selection.has(key)}
          tabIndex={key === selected || (selected === null && entryIndex === 0) ? 0 : -1}
          title={entry.name} onKeyDown={event => fileKeyDown(event, entryIndex)}>
          <Icon aria-hidden="true" className={entry.kind === 'directory' ? 'folder-icon' : undefined} />
          <span>{displayName(entry, option.showExtensions)}</span>
        </button>
      </>
    );
  }

  function showContext(event: MouseEvent | KeyboardEvent, selection: FileActionSelection | null, anchor: HTMLElement | null, target = folderPath, sources?: FileActionSelection[]) {
    if (!target || pending || document.querySelector('dialog[open]')) return;
    const bounds = anchor?.getBoundingClientRect();
    setFileContext({
      directory: target,
      selection,
      sources,
      anchor,
      x: 'clientX' in event ? event.clientX : (bounds?.left ?? 0) + 8,
      y: 'clientY' in event ? event.clientY : (bounds?.bottom ?? 0),
    });
  }

  function openItemContext(event: MouseEvent | KeyboardEvent, entry: Entry, entryIndex: number) {
    event.preventDefault();
    event.stopPropagation();
    const key = entryKey(entry, entryIndex);
    const included = selection.has(key);
    if (!included) selectRow(entry, entryIndex);
    const item = fileActionSelection(entry);
    const anchor = rowButtons.current.get(key) ?? null;
    if (included && batchSelection.length > 1) showContext(event, null, anchor, folderPath, batchSelection);
    else if (item && (!included || selectedRows.length === 1)) showContext(event, item, anchor);
    else dismissContext();
  }

  function openEmptyContext(event: MouseEvent<HTMLDivElement>) {
    if (!(event.target instanceof Element) ||
      event.target.closest('button, input, th, .folder-error, .folder-tag-filter, .file-column:not(:first-child)')) return;
    event.preventDefault();
    showContext(event, null, event.currentTarget);
  }

  function openPreviewContext(event: MouseEvent | KeyboardEvent, entry: Entry | null, directory: string, anchor: HTMLElement) {
    event.preventDefault();
    event.stopPropagation();
    const item = entry ? fileActionSelection(entry) : null;
    if (!entry || item) showContext(event, item, anchor, directory);
    else dismissContext();
  }

  function startDrag(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0 || event.ctrlKey || !(event.target instanceof Element)
      || event.target.closest('[data-file-key], button, input, .file-column:not(:first-child), .folder-heading, th, .folder-error')) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    dragOrigin.current = { x: event.clientX, y: event.clientY, pointer: event.pointerId };
  }

  function extendDrag(event: PointerEvent<HTMLDivElement>) {
    const origin = dragOrigin.current;
    if (!origin || origin.pointer !== event.pointerId) return;
    const left = Math.min(origin.x, event.clientX);
    const top = Math.min(origin.y, event.clientY);
    const width = Math.abs(origin.x - event.clientX);
    const height = Math.abs(origin.y - event.clientY);
    if (Math.max(width, height) < 4 && !dragBox) return;
    const area = event.currentTarget.getBoundingClientRect();
    setDragBox({ x: left - area.left, y: top - area.top, width, height });
    const matches = Array.from(event.currentTarget.querySelectorAll<HTMLElement>('[data-file-key]')).flatMap(element => {
      const row = element.getBoundingClientRect();
      return row.right >= left && row.left <= left + width && row.bottom >= top && row.top <= top + height
        ? [element.dataset.fileKey!] : [];
    });
    setSelectedRows(matches);
    setSelected(matches.at(-1) ?? null);
    rangeAnchor.current = matches[0] ?? null;
  }

  function endDrag(event: PointerEvent<HTMLDivElement>) {
    if (dragOrigin.current?.pointer !== event.pointerId) return;
    if (!dragBox && event.type === 'pointerup') {
      setSelectedRows([]);
      setSelected(null);
      rangeAnchor.current = null;
    }
    dragOrigin.current = null;
    setDragBox(null);
  }

  function renderCell(entry: Entry, index: number, column: ColumnId) {
    switch (column) {
      case 'name': return rowName(entry, index);
      case 'kind': return entry.kind === 'directory' ? strings.folder : fileExtension(entry).toUpperCase() || strings[entry.kind];
      case 'size': return readableSize(entry.size);
      case 'modified': return readableDate(entry.modifiedMs);
      case 'created': return readableDate(entry.createdMs ?? null);
      case 'tags': {
        const ids = entry.location ? settled?.paths?.get(entry.location.path)?.tagIds : undefined;
        return tags.filter(tag => ids?.includes(tag.id)).map(tag => tag.name).join(', ') || '—';
      }
      case 'note': return entry.location && settled?.paths?.get(entry.location.path)?.hasNote
        ? organizationStrings().hasNote : '—';
      default: return '—';
    }
  }

  const visibleColumns = option.columns.filter(column => column.visible);
  const preview = entries.find((entry, index) => entryKey(entry, index) === selected && entry.kind === 'directory');
  const selectedEntries = entries.filter((entry, index) => selection.has(entryKey(entry, index)));
  function dragEntry(entry: Entry): React.HTMLAttributes<HTMLElement> {
    const item = fileActionSelection(entry);
    if (!item || pending) return {};
    const target = item.path;
    function accepts(event: React.DragEvent<HTMLElement>) {
      if (item?.kind !== 'directory' || !draggedEntry || !event.dataTransfer.types.includes(workspaceFileDrag)) return false;
      for (let ancestor: string | null = target; ancestor;) {
        if (ancestor === draggedEntry.path) return false;
        const parent = parentPath(ancestor);
        if (parent === ancestor) break;
        ancestor = parent;
      }
      return parentPath(draggedEntry.path) !== target;
    }
    function clearHighlight(event: React.DragEvent<HTMLElement>) {
      delete event.currentTarget.dataset.fileDropTarget;
    }
    return {
      draggable: true,
      onDragStart(event) {
        draggedEntry = item;
        event.currentTarget.dataset.fileDragSource = 'true';
        event.dataTransfer.setData(workspaceFileDrag, 'move');
        event.dataTransfer.effectAllowed = 'move';
        const ghost = document.createElement('div');
        ghost.className = 'file-drag-ghost';
        const icon = document.createElement('span');
        icon.textContent = item.kind === 'directory' ? '📁' : '📄';
        ghost.append(icon, document.createTextNode(item.name));
        document.body.append(ghost);
        event.dataTransfer.setDragImage(ghost, 16, 16);
        window.setTimeout(() => ghost.remove(), 0);
      },
      onDragEnd(event) {
        delete event.currentTarget.dataset.fileDragSource;
        draggedEntry = null;
        document.querySelectorAll('[data-file-drop-target]').forEach(node => node.removeAttribute('data-file-drop-target'));
      },
      onDragOver(event) {
        if (!accepts(event)) return;
        event.preventDefault();
        event.dataTransfer.dropEffect = 'move';
        event.currentTarget.dataset.fileDropTarget = 'true';
      },
      onDragLeave(event) {
        if (!(event.relatedTarget instanceof Node) || !event.currentTarget.contains(event.relatedTarget)) clearHighlight(event);
      },
      onDrop(event) {
        clearHighlight(event);
        if (!accepts(event) || !draggedEntry) return;
        event.preventDefault();
        event.stopPropagation();
        const source = draggedEntry.path;
        draggedEntry = null;
        void invoke('start_transfer', { request: { kind: 'move', sources: [source], directory: item.path } })
          .catch(error => window.alert(fileActionLabels().errors[fileErrorCode(error)]));
      },
    };
  }

  const mutationSelection = selectedEntries.length === 1 ? fileActionSelection(selectedEntries[0]) : null;
  const selectedActions = selectedEntries.map(fileActionSelection);
  const batchSelection = selectedActions.every(item => item !== null) ? selectedActions : [];

  return (
        <section id={`pane-${tab.id}`} className="workspace-main" hidden={!visible} data-panel-index={panelIndex}
          data-focused={focused} aria-labelledby={`tab-${tab.id}`} role="tabpanel"
          onPointerDown={onFocus} onFocus={onFocus}>
          <div className="workspace-toolbar">
            <div className="navigation-buttons">
              <ToolButton label={strings.back} disabled={index <= 0} onClick={() => goToHistory(index - 1)}><ArrowLeft aria-hidden="true" /></ToolButton>
              <ToolButton label={strings.forward} disabled={index === visits.length - 1} onClick={() => goToHistory(index + 1)}><ArrowRight aria-hidden="true" /></ToolButton>
              <ToolButton label={strings.up} disabled={!parent} onClick={() => parent && goTo(parent.path)}><ArrowUp aria-hidden="true" /></ToolButton>
            </div>
            <div className="workspace-address">
              {editing ? (
                <form className="address-form" onSubmit={submitAddress}>
                  <input
                    ref={addressRef}
                    aria-label={strings.path}
                    aria-invalid={addressFailure || undefined}
                    aria-describedby={addressFailure ? 'address-error' : undefined}
                    autoComplete="off"
                    spellCheck={false}
                    value={address}
                    onChange={(event) => {
                      cancelAddress();
                      setAddress(event.target.value);
                    }}
                    onKeyDown={(event) => {
                      if (event.key === 'Escape') {
                        event.preventDefault();
                        closeAddress();
                      }
                    }}
                  />
                  <ToolButton type="submit" label={strings.openPath}><ArrowRight aria-hidden="true" /></ToolButton>
                  <ToolButton label={strings.cancel} onClick={closeAddress}><X aria-hidden="true" /></ToolButton>
                </form>
              ) : (
                <nav className="workspace-breadcrumb" aria-label={strings.path}>
                  <ol ref={breadcrumbRef}>
                    {breadcrumbs.map((crumb, crumbIndex) => (
                      <li key={crumb.path}>
                        {crumbIndex > 0 && <ChevronRight aria-hidden="true" />}
                        {crumb.path === currentPath ? (
                          <strong aria-current="location" title={crumb.path}>{crumb.label}</strong>
                        ) : (
                          <button type="button" onClick={() => goTo(crumb.path)} title={crumb.path}>{crumb.label}</button>
                        )}
                      </li>
                    ))}
                  </ol>
                  <button ref={editRef} type="button" className="edit-path" onClick={startAddress} aria-label={strings.editPath} title={strings.editPath}>
                    <Pencil aria-hidden="true" />
                  </button>
                </nav>
              )}
              {editing && addressFailure && <p className="address-error" id="address-error" role="alert">{strings.errors[failure.code]}</p>}
            </div>

            <ToolButton
              label={strings.refresh}
              disabled={!directory}
              onClick={() => navigate({ target: { kind: 'path', path: currentPath }, source: 'refresh' })}
            >
              <RotateCw aria-hidden="true" />
            </ToolButton>
            <ViewOptions value={option} onChange={updateOptions} hasFolder={!!folderPath}
              saveDefault={() => updateOptions(option, null)} reset={resetOptions} />
            <FileMutationControls directory={!pending ? folderPath : null} selected={mutationSelection}
              batchSelection={batchSelection}
              context={fileContext} dismissContext={dismissContext} selectAll={() => setSelectedRows(entries.map(entryKey))}
              openFolder={goTo} openTab={openTab} openWindow={openWindow}
              onChanged={path => {
                if (!visible || pending) return false;
                if (folderPath === path) {
                  navigate({ target: { kind: 'path', path }, source: 'refresh' });
                  return true;
                }
                const previewColumns = listRef.current?.querySelectorAll<HTMLElement>('[data-preview-path]') ?? [];
                if (Array.from(previewColumns).some(column => column.dataset.previewPath === path)) {
                  setPreviewRefresh(value => value + 1);
                  return true;
                }
                return false;
              }} />
          </div>

          <div className="workspace-files" ref={listRef} tabIndex={-1} aria-label={strings.folderContents} aria-busy={pending !== null}
            onPointerDown={startDrag} onPointerMove={extendDrag} onPointerUp={endDrag} onPointerCancel={endDrag}
            onContextMenu={openEmptyContext} onWheel={dismissContext}
            onKeyDown={event => {
              if (event.target === event.currentTarget && isContextKey(event)) {
                event.preventDefault();
                showContext(event, null, event.currentTarget);
                return;
              }
              if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== 'a') return;
              event.preventDefault();
              setSelectedRows(entries.map(entryKey));
            }}>
            {dragBox && <svg className="selection-box" aria-hidden="true"><rect {...dragBox} /></svg>}
            {viewError && <p className="workspace-feedback" role="status">{strings.workspaceError}</p>}
            {usedFallback && <p className="folder-empty" role="status">{strings.startFallback}</p>}
            <div className="folder-heading">
              {parent && <p title={parent.path}>{parent.label}</p>}
              <h1 title={currentPath}>{currentLabel}</h1>
            </div>
            {tagFilter && <div className="organization-row folder-tag-filter">
              <button type="button" onClick={clearFilter}>{organizationStrings().clearFilter}</button>
              <span role="status">{!settled ? organizationStrings().filterLoading : !settled.paths
                ? organizationStrings().failure : !entries.length ? organizationStrings().noMatches : ''}</span>
            </div>}

            {failure && !addressFailure && (
              <div className="folder-error" role="alert">
                <p>{strings.errors[failure.code]}</p>
                <button type="button" onClick={() => navigate(failure.destination)}>{strings.retry}</button>
                {parent && <button type="button" onClick={() => goTo(parent.path)}>{strings.up}</button>}
              </div>
            )}

            {directory && option.mode === 'details' && (
              <div className="files-table-scroll" ref={tableRef} tabIndex={0} role="region" aria-label={strings.fileList}>
                <table className="files-table" aria-label={strings.folderContents}
                  style={{ minWidth: visibleColumns.reduce((total, column) => total + column.width, 0) }}>
                  <colgroup>{visibleColumns.map(column => <col key={column.id} width={column.width} />)}</colgroup>
                  <thead>
                    <tr>{visibleColumns.map(column => <th key={column.id} scope="col"
                      aria-sort={option.sort === column.id ? option.descending ? 'descending' : 'ascending' : undefined}>
                      {(() => {
                        const sort = sortIds.find(id => id === column.id);
                        if (!sort) return viewStrings[column.id];
                        const active = option.sort === sort;
                        return <button type="button" className="file-sort-button"
                          onClick={() => updateOptions({ ...option, sort, descending: active && !option.descending })}>
                          {viewStrings[column.id]} {active && (option.descending
                            ? <ArrowDown aria-hidden="true" /> : <ArrowUp aria-hidden="true" />)}
                        </button>;
                      })()}
                    </th>)}</tr>
                  </thead>
                  <tbody>
                    {entries.map((entry, entryIndex) => {
                      const key = entryKey(entry, entryIndex);
                      const group = option.groupBy === 'kind' && entries[entryIndex - 1]?.kind !== entry.kind;
                      return <Fragment key={key}>
                        {group && <tr className="file-group-row"><th colSpan={visibleColumns.length} scope="rowgroup">
                          {entry.kind === 'directory' ? strings.folder : strings[entry.kind]}
                        </th></tr>}
                        <tr {...dragEntry(entry)} data-file-key={key} data-selected={selection.has(key)}
                          onClick={event => selectRow(entry, entryIndex, event)} onDoubleClick={() => openRow(entry)}
                          onContextMenu={event => openItemContext(event, entry, entryIndex)}>
                          {visibleColumns.map(column => <td key={column.id}>{renderCell(entry, entryIndex, column.id)}</td>)}
                        </tr>
                      </Fragment>;
                    })}
                  </tbody>
                </table>
              </div>
            )}
            {directory && option.mode !== 'details' && (
              <div className={option.mode === 'grid' ? 'file-grid' : 'file-columns'} ref={tableRef}
                style={{ '--file-icon-size': option.iconSize + 'px' } as CSSProperties}>
                <div className={option.mode === 'grid' ? 'file-grid-items' : 'file-column'} aria-label={strings.folderContents}>
                  {entries.map((entry, index) => {
                    const key = entryKey(entry, index);
                    const group = option.groupBy === 'kind' && entries[index - 1]?.kind !== entry.kind;
                    return <Fragment key={key}>
                      {group && <div className="file-group-label">{entry.kind === 'directory' ? strings.folder : strings[entry.kind]}</div>}
                      <div {...dragEntry(entry)} className="file-tile" data-file-key={key} data-selected={selection.has(key)}
                        onClick={event => selectRow(entry, index, event)} onDoubleClick={() => openRow(entry)}
                        onContextMenu={event => openItemContext(event, entry, index)}>
                        {rowName(entry, index)}
                      </div>
                    </Fragment>;
                  })}
                </div>
                {option.mode === 'columns' && preview?.location && (
                  <ColumnPreview key={preview.location.path} path={preview.location.path}
                    context={{ panelId, tabId: tab.id }} options={option} onNavigate={goTo}
                    onContext={openPreviewContext} refresh={previewRefresh} />
                )}
              </div>
            )}
            {!directory && pending && <p className="folder-empty">{strings.loading}</p>}
            {directory && entries.length === 0 && !pending && <p className="folder-empty">{strings.empty}</p>}
            {directory && directory.skipped > 0 && <p className="skipped-entries">{strings.skipped(directory.skipped)}</p>}
          </div>

          <footer className="workspace-status">
            <span role="status">{pending ? strings.loading : directory ? strings.items(entries.length) : ''}</span>
            {selectedEntries.length > 0 && <span>{viewStrings.selection(selectedEntries.length)}
              {selectedEntries.length > 1 && ' · ' + readableSize(selectedEntries.some(entry => entry.size !== null)
                ? selectedEntries.reduce((total, entry) => total + (entry.size ?? 0), 0) : null)}
            </span>}
            <span className="local-status"><Shield aria-hidden="true" />{strings.local}</span>
            {activityControl}
          </footer>
        </section>
  );
}
