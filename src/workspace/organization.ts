import { invoke } from '@tauri-apps/api/core';
import { locale } from './strings';

export type Note = { body: string; revision: number };
export type OrganizedItem = { id: string; path: string; note: Note | null; tagIds: string[] };
export type FavoriteGroup = { id: string; name: string };
export type Tag = { id: string; name: string; color: string | null };
export type Favorite = {
  id: string; itemId: string; groupId: string | null; path: string | null; name: string | null; isDirectory: boolean;
  defaultLocation?: 'desktop' | 'downloads' | 'documents' | 'pictures';
};
export type PathMetadata = { path: string; tagIds: string[]; hasNote: boolean };
export type SelectionDetails = { paths: string[]; count: number; kinds: string; size: string | null };

export type Organization = {
  items: OrganizedItem[];
  groups: FavoriteGroup[];
  tags: Tag[];
  favorites: Favorite[];
  copyMetadata: boolean;
};

export type OrganizationChange =
  | { kind: 'save-note'; itemId: string; expectedRevision: number; body: string }
  | { kind: 'create-group'; name: string }
  | { kind: 'rename-group'; id: string; name: string }
  | { kind: 'remove-group'; id: string }
  | { kind: 'add-favorite'; itemId: string; groupId: string | null }
  | { kind: 'remove-favorite'; id: string }
  | { kind: 'move-favorite'; id: string; groupId: string | null; index: number }
  | { kind: 'upsert-tag'; id: string | null; name: string; color: string | null }
  | { kind: 'remove-tag'; id: string }
  | { kind: 'assign-tag'; itemIds: string[]; tagId: string; assigned: boolean }
  | { kind: 'set-copy-metadata'; enabled: boolean };

export function loadOrganization(paths: string[] = [], inspect = false, includeNotes = true) {
  return invoke<Organization>('load_organization', { paths, inspect, includeNotes });
}

export function updateOrganization(change: OrganizationChange) {
  return invoke<number | null>('update_organization', { change });
}

export function lookupOrganizationTags(paths: string[]) {
  return invoke<PathMetadata[]>('lookup_organization_tags', { paths });
}

const english = {
  defaultFavoriteNames: { desktop: 'Desktop', downloads: 'Downloads', documents: 'Documents', pictures: 'Pictures' },
  inspector: 'Inspector', closeInspector: 'Close inspector', resize: 'Inspector width', nothing: 'Select a file or folder.',
  loading: 'Loading metadata…', unavailable: 'Metadata is unavailable for this selection.',
  failure: 'Could not update local metadata.', retry: 'Try again', cancel: 'Cancel', save: 'Save',
  favorites: 'Favorites', newGroup: 'New group', groupName: 'Group name', renameGroup: 'Rename group', removeGroup: 'Remove group',
  root: 'Ungrouped', addFavorite: 'Add favorite', removeFavorite: 'Remove favorite',
  moveUp: 'Move up', moveDown: 'Move down', moveTo: 'Move to group', missing: 'Unavailable favorite',
  tags: 'Tags', newTag: 'New tag', editTag: 'Edit tag', removeTag: 'Delete tag', tagName: 'Tag name',
  tagColor: 'Tag color', useColor: 'Use a color', clearFilter: 'Clear tag filter',
  noMatches: 'No items with this tag in this folder.', filterLoading: 'Filtering by tag…',
  confirmTag: (name: string) => `Delete “${name}” and remove it from all items?`,
  count: (n: number) => `${n} items selected`, unresolved: (n: number) => `Metadata available for ${n} items.`,
  note: 'Markdown note', hasNote: 'Has a note', preview: 'Preview', edit: 'Edit', saving: 'Saving…', pending: 'Unsaved changes',
  saved: 'Saved', conflict: 'This note changed in another window. Your draft is still here.',
  replaceNote: 'Save my draft over the current version', tooLong: 'Notes can be up to 256 KiB.',
  pendingNotes: 'Some notes are still unsaved.', stay: 'Keep editing', discard: 'Discard drafts and close',
  copyMetadata: 'Copy notes and tags with files and folders',
};

const italian: typeof english = {
  defaultFavoriteNames: { desktop: 'Scrivania', downloads: 'Download', documents: 'Documenti', pictures: 'Immagini' },
  inspector: 'Ispettore', closeInspector: 'Chiudi ispettore', resize: 'Larghezza ispettore', nothing: 'Seleziona un file o una cartella.',
  loading: 'Caricamento dei metadati…', unavailable: 'Metadati non disponibili per la selezione.',
  failure: 'Impossibile aggiornare i metadati locali.', retry: 'Riprova', cancel: 'Annulla', save: 'Salva',
  favorites: 'Preferiti', newGroup: 'Nuovo gruppo', groupName: 'Nome del gruppo', renameGroup: 'Rinomina gruppo', removeGroup: 'Rimuovi gruppo',
  root: 'Senza gruppo', addFavorite: 'Aggiungi ai preferiti', removeFavorite: 'Rimuovi dai preferiti',
  moveUp: 'Sposta in alto', moveDown: 'Sposta in basso', moveTo: 'Sposta nel gruppo', missing: 'Preferito non disponibile',
  tags: 'Tag', newTag: 'Nuovo tag', editTag: 'Modifica tag', removeTag: 'Elimina tag', tagName: 'Nome del tag',
  tagColor: 'Colore del tag', useColor: 'Usa un colore', clearFilter: 'Rimuovi filtro per tag',
  noMatches: 'Nessun elemento con questo tag nella cartella.', filterLoading: 'Filtro per tag in corso…',
  confirmTag: name => `Eliminare «${name}» e rimuoverlo da tutti gli elementi?`,
  count: n => `${n} elementi selezionati`, unresolved: n => `Metadati disponibili per ${n} elementi.`,
  note: 'Nota Markdown', hasNote: 'Nota presente', preview: 'Anteprima', edit: 'Modifica', saving: 'Salvataggio…', pending: 'Modifiche da salvare',
  saved: 'Salvata', conflict: 'La nota è cambiata in un’altra finestra. La tua bozza è ancora qui.',
  replaceNote: 'Salva la mia bozza al posto della versione attuale', tooLong: 'Le note possono occupare fino a 256 KiB.',
  pendingNotes: 'Alcune note sono ancora da salvare.', stay: 'Continua a modificare', discard: 'Scarta le bozze e chiudi',
  copyMetadata: 'Copia note e tag insieme a file e cartelle',
};

export function organizationStrings() {
  return locale === 'it' ? italian : english;
}
