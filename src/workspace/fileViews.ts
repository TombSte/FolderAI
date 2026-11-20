import { invoke } from '@tauri-apps/api/core';
import type { Directory } from '../ipc';

export type Entry = Directory['entries'][number];
export type ColumnId = 'name' | 'kind' | 'size' | 'modified' | 'created' | 'tags' | 'note' | 'indexing';
export type SortId = 'name' | 'kind' | 'size' | 'modified' | 'created';
export type ViewMode = 'details' | 'grid' | 'columns';
export type Column = { id: ColumnId; width: number; visible: boolean };
export type ViewOptions = {
  mode: ViewMode; sort: SortId; descending: boolean; foldersFirst: boolean;
  groupBy: 'none' | 'kind'; showExtensions: boolean; showHidden: boolean;
  iconSize: number; columns: Column[];
};

const columnIds: ColumnId[] = ['name', 'kind', 'size', 'modified', 'created', 'tags', 'note', 'indexing'];
export const sortIds: SortId[] = ['name', 'kind', 'size', 'modified', 'created'];
export const modes: ViewMode[] = ['details', 'grid', 'columns'];
export const defaultView: ViewOptions = {
  mode: 'details', sort: 'name', descending: false, foldersFirst: true, groupBy: 'none',
  showExtensions: true, showHidden: false, iconSize: 40,
  columns: columnIds.map((id, index) => ({ id, width: [260, 120, 96, 144, 144, 120, 96, 120][index], visible: index < 4 })),
};

export function loadWorkspaceViews(path: string | null) {
  return invoke<{ global: ViewOptions; folder: ViewOptions | null; folderIdentity: string | null }>('load_workspace_views', { path });
}

export function saveWorkspaceView(path: string | null, view: ViewOptions | null, folderIdentity: string | null) {
  return invoke<void>('save_workspace_view', { path, view, folderIdentity });
}

export function fileExtension(entry: Entry) {
  if (entry.kind !== 'file') return '';
  const dot = entry.name.lastIndexOf('.');
  return dot > 0 ? entry.name.slice(dot + 1).toLowerCase() : '';
}

export function sortEntries(entries: Entry[], options: ViewOptions, collator: Intl.Collator) {
  const field = (entry: Entry) => {
    switch (options.sort) {
      case 'name': return entry.name;
      case 'kind': return `${entry.kind}:${fileExtension(entry)}`;
      case 'size': return entry.size;
      case 'modified': return entry.modifiedMs;
      case 'created': return entry.createdMs ?? null;
    }
  };
  return [...entries].sort((first, second) => {
    if (options.foldersFirst && (first.kind === 'directory') !== (second.kind === 'directory')) {
      return first.kind === 'directory' ? -1 : 1;
    }
    if (options.groupBy === 'kind' && first.kind !== second.kind) {
      return collator.compare(first.kind, second.kind);
    }
    const a = field(first);
    const b = field(second);
    if ((a === null) !== (b === null)) return a === null ? 1 : -1;
    const sorted = typeof a === 'number' && typeof b === 'number'
      ? a - b
      : typeof a === 'string' && typeof b === 'string' ? collator.compare(a, b) : 0;
    return (options.descending ? -sorted : sorted) || collator.compare(first.name, second.name);
  });
}

const knownExtensions = new Set([
  'bmp', 'csv', 'doc', 'docx', 'gif', 'jpeg', 'jpg', 'json', 'md', 'mov', 'mp3', 'mp4',
  'odt', 'pdf', 'png', 'ppt', 'pptx', 'rtf', 'svg', 'tif', 'tiff', 'txt', 'webp', 'xls', 'xlsx', 'zip',
]);

export function displayName(entry: Entry, showExtensions: boolean) {
  if (showExtensions || entry.kind !== 'file') return entry.name;
  const dot = entry.name.lastIndexOf('.');
  return dot > 0 && knownExtensions.has(entry.name.slice(dot + 1).toLowerCase())
    ? entry.name.slice(0, dot) : entry.name;
}
