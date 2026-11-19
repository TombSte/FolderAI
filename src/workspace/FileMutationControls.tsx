import { invoke } from '@tauri-apps/api/core';
import { Copy, Ellipsis, FilePlus2, FolderInput, FolderOpen, FolderPlus, ListChecks, PanelTop, Pencil, SquareArrowOutUpRight } from 'lucide-react';
import { useEffect, useId, useLayoutEffect, useRef, useState, type FormEvent } from 'react';
import { parentPath, pathLabel } from './paths';
import { locale, strings } from './strings';
import './file-mutations.scss';

export type FileActionSelection = { path: string; name: string; kind: 'file' | 'directory' };
export type FileActionContext = {
  directory: string;
  selection: FileActionSelection | null;
  x: number;
  y: number;
  anchor: HTMLElement | null;
};
type Action =
  | { kind: 'create'; directory: string; isDirectory: boolean }
  | { kind: 'rename'; directory: string; path: string }
  | { kind: 'copy'; source: string }
  | { kind: 'move'; source: string; directory: string };
type Request =
  | { kind: 'create'; directory: string; name: string; isDirectory: boolean }
  | { kind: 'rename'; path: string; name: string };
type CopyResult = { path: string; cleanupPending: boolean };
type TransferResult = (CopyResult & { kind: 'copy' })
  | { kind: 'move'; path: string; disposition: 'moved' | 'kept-both' | 'already-there' };

const english = {
  actions: 'File actions',
  newFile: 'New empty file',
  newFolder: 'New folder',
  selectAll: 'Select all',
  rename: 'Rename',
  copyToFolder: 'Copy to folder',
  moveToFolder: 'Move to folder…',
  create: 'Create',
  copy: 'Copy',
  move: 'Move',
  destination: 'Destination folder',
  copiedTo: 'Copied to',
  movedTo: 'Moved to',
  movedAs: 'Moved under a different name to',
  alreadyThere: 'Already at',
  cleanupPending: 'The file was copied. Temporary file cleanup is pending.',
  close: 'Close',
  working: 'Working…',
  stop: 'Stop copy',
  stopping: 'Stopping…',
  errors: {
    'invalid-request': 'Enter a valid name for a single item.',
    'not-found': 'The file or folder is no longer at this location.',
    'permission-denied': 'You do not have permission to change this item.',
    'already-exists': 'An item with this name already exists.',
    unsupported: 'This operation is unavailable for this item or filesystem.',
    'source-changed': 'The source changed during copying. Check the source before trying again.',
    'storage-full': 'There is not enough space in the destination folder.',
    'cleanup-failed': 'Could not remove temporary files. Check the destination before doing anything else.',
    cancelled: 'The copy was stopped.',
    'recovery-pending': 'The result could not be verified. Check the folder before doing anything else.',
    unavailable: 'Could not complete the file operation.',
  },
};

const italian: typeof english = {
  actions: 'Azioni sui file',
  newFile: 'Nuovo file vuoto',
  newFolder: 'Nuova cartella',
  selectAll: 'Seleziona tutto',
  rename: 'Rinomina',
  copyToFolder: 'Copia in cartella',
  moveToFolder: 'Sposta in cartella…',
  create: 'Crea',
  copy: 'Copia',
  move: 'Sposta',
  destination: 'Cartella di destinazione',
  copiedTo: 'Copiato in',
  movedTo: 'Spostato in',
  movedAs: 'Spostato con un altro nome in',
  alreadyThere: 'Già presente in',
  cleanupPending: 'Il file è stato copiato. La pulizia dei file temporanei è in sospeso.',
  close: 'Chiudi',
  working: 'Operazione in corso…',
  stop: 'Interrompi copia',
  stopping: 'Interruzione in corso…',
  errors: {
    'invalid-request': 'Inserisci un nome valido per un singolo elemento.',
    'not-found': 'Il file o la cartella non è più in questa posizione.',
    'permission-denied': 'Non hai i permessi per modificare questo elemento.',
    'already-exists': 'Esiste già un elemento con questo nome.',
    unsupported: 'L’operazione non è disponibile per questo elemento o filesystem.',
    'source-changed': 'L’origine è cambiata durante la copia. Controlla il file prima di riprovare.',
    'storage-full': 'Lo spazio nella cartella di destinazione è insufficiente.',
    'cleanup-failed': 'Impossibile rimuovere i file temporanei. Controlla la destinazione prima di proseguire.',
    cancelled: 'La copia è stata interrotta.',
    'recovery-pending': 'Impossibile verificare il risultato. Controlla la cartella prima di proseguire.',
    unavailable: 'Impossibile completare l’operazione.',
  },
};

type ErrorCode = keyof typeof english.errors;

function errorCode(error: unknown): ErrorCode {
  return typeof error === 'string' && Object.hasOwn(english.errors, error) ? error as ErrorCode : 'unavailable';
}

export function FileMutationControls({ directory, selected, onChanged, context, dismissContext, selectAll, openFolder, openTab, openWindow }: {
  directory: string | null;
  selected: FileActionSelection | null;
  onChanged: (directory: string) => boolean;
  context: FileActionContext | null;
  dismissContext: () => void;
  selectAll: () => void;
  openFolder: (path: string) => void;
  openTab: (path: string) => void;
  openWindow: (path: string) => void;
}) {
  const labels = locale === 'it' ? italian : english;
  const id = useId();
  const titleId = id + '-title';
  const nameId = id + '-name';
  const errorId = id + '-error';
  const menu = useRef<HTMLDetailsElement>(null);
  const trigger = useRef<HTMLElement>(null);
  const contextActions = useRef<HTMLDivElement>(null);
  const contextReturn = useRef<HTMLElement | null>(null);
  const dialogReturn = useRef<HTMLElement | null>(null);
  const dialog = useRef<HTMLDialogElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const closeResult = useRef<HTMLButtonElement>(null);
  const changed = useRef<typeof onChanged | null>(null);
  const [action, setAction] = useState<Action | null>(null);
  const [value, setValue] = useState('');
  const [transferResult, setTransferResult] = useState<TransferResult | null>(null);
  const [error, setError] = useState<ErrorCode | null>(null);
  const [busy, setBusy] = useState(false);
  const [activeCopy, setActiveCopy] = useState<{ id: string; stopping: boolean } | null>(null);
  const copy = action?.kind === 'copy';
  const move = action?.kind === 'move';
  const title = action?.kind === 'rename' ? labels.rename
    : copy ? labels.copyToFolder
    : move ? labels.moveToFolder
    : action?.kind === 'create' && action.isDirectory ? labels.newFolder : labels.newFile;
  const requiresReview = error === 'recovery-pending' || error === 'cleanup-failed';
  const resultLabel = transferResult?.kind === 'copy' ? labels.copiedTo
    : transferResult?.disposition === 'kept-both' ? labels.movedAs
    : transferResult?.disposition === 'already-there' ? labels.alreadyThere : labels.movedTo;

  useLayoutEffect(() => {
    changed.current = onChanged;
    return () => { changed.current = null; };
  }, [onChanged]);

  useLayoutEffect(() => {
    if (!action || !dialog.current || dialog.current.open) return;
    dialog.current.showModal();
    input.current?.focus();
    input.current?.select();
  }, [action]);

  useLayoutEffect(() => {
    if (transferResult) closeResult.current?.focus();
  }, [transferResult]);

  useLayoutEffect(() => {
    const popup = contextActions.current;
    if (!context) {
      if (contextReturn.current) {
        const target = contextReturn.current;
        contextReturn.current = null;
        focusBack(target);
      }
      return;
    }
    if (!popup) return;
    const margin = 8;
    popup.style.left = Math.max(margin, Math.min(context.x, innerWidth - popup.offsetWidth - margin)) + 'px';
    popup.style.top = Math.max(margin, Math.min(context.y, innerHeight - popup.offsetHeight - margin)) + 'px';
    popup.querySelector('button')?.focus({ preventScroll: true });
  }, [context]);

  useEffect(() => {
    if (!context) return;
    const onOutside = (event: Event) => {
      if (event.target instanceof Node && !contextActions.current?.contains(event.target)) dismissContext();
    };
    window.addEventListener('pointerdown', onOutside, true);
    window.addEventListener('focusin', onOutside);
    window.addEventListener('resize', dismissContext);
    return () => {
      window.removeEventListener('pointerdown', onOutside, true);
      window.removeEventListener('focusin', onOutside);
      window.removeEventListener('resize', dismissContext);
    };
  }, [context, dismissContext]);

  function focusBack(anchor: HTMLElement | null) {
    (anchor?.isConnected ? anchor : trigger.current)?.focus({ preventScroll: true });
  }

  function start(next: Action, initialValue = '', anchor = trigger.current) {
    if (menu.current) menu.current.open = false;
    dialogReturn.current = anchor;
    dismissContext();
    setValue(initialValue);
    setTransferResult(null);
    setError(null);
    setAction(next);
  }

  function createActions(target: string | null, anchor = trigger.current) {
    return <>
      <button type="button" disabled={!target}
        onClick={() => target && start({ kind: 'create', directory: target, isDirectory: false }, '', anchor)}>
        <FilePlus2 aria-hidden="true" /> {labels.newFile}
      </button>
      <button type="button" disabled={!target}
        onClick={() => target && start({ kind: 'create', directory: target, isDirectory: true }, '', anchor)}>
        <FolderPlus aria-hidden="true" /> {labels.newFolder}
      </button>
    </>;
  }

  function entryActions(target: string | null, item: FileActionSelection | null, contextual = false, anchor = trigger.current) {
    return <>
      <button type="button" disabled={!item || !target}
        onClick={() => target && item && start({ kind: 'rename', directory: target, path: item.path }, item.name, anchor)}>
        <Pencil aria-hidden="true" /> {labels.rename}
      </button>
      {(!contextual || item?.kind === 'file') && <button type="button" disabled={!target || item?.kind !== 'file'}
        onClick={() => target && item?.kind === 'file' && start({ kind: 'copy', source: item.path }, target, anchor)}>
        <Copy aria-hidden="true" /> {labels.copyToFolder}
      </button>}
      <button type="button" disabled={!target || !item}
        onClick={() => target && item && start({ kind: 'move', source: item.path, directory: target }, target, anchor)}>
        <FolderInput aria-hidden="true" /> {labels.moveToFolder}
      </button>
    </>;
  }

  function browseFromContext(path: string, open: (path: string) => void) {
    dismissContext();
    focusBack(context?.anchor ?? null);
    open(path);
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!action || !value || busy || requiresReview || transferResult) return;
    const destination = action.kind === 'copy' || action.kind === 'move' ? value : action.directory;
    setBusy(true);
    setError(null);
    try {
      if (action.kind === 'copy') {
        const requestId = crypto.randomUUID();
        setActiveCopy({ id: requestId, stopping: false });
        const result = await invoke<CopyResult>('copy_regular_file', {
          request: { source: action.source, directory: destination, requestId },
        });
        const refreshed = changed.current?.(parentPath(result.path) ?? destination);
        if (!refreshed || result.cleanupPending) {
          setTransferResult({ kind: 'copy', ...result });
          return;
        }
      } else if (action.kind === 'move') {
        const path = await invoke<string>('move_file_or_directory', {
          request: { source: action.source, directory: destination },
        });
        if (!changed.current?.(action.directory)) {
          changed.current?.(parentPath(path) ?? destination);
        }
        const disposition = path === action.source ? 'already-there'
          : pathLabel(path).normalize() === pathLabel(action.source).normalize() ? 'moved' : 'kept-both';
        setTransferResult({ kind: 'move', path, disposition });
        return;
      } else {
        const request: Request = action.kind === 'rename'
          ? { kind: 'rename', path: action.path, name: value }
          : { ...action, name: value };
        await invoke<string>('mutate_file', { request });
        changed.current?.(destination);
      }
    } catch (error) {
      const code = errorCode(error);
      setError(code);
      if (code === 'recovery-pending' || code === 'cleanup-failed') {
        if (action.kind !== 'move' || !changed.current?.(action.directory)) {
          changed.current?.(destination);
        }
      }
      return;
    } finally {
      setActiveCopy(null);
      setBusy(false);
    }
    dialog.current?.close();
  }

  async function stopCopy() {
    if (!activeCopy || activeCopy.stopping) return;
    const { id } = activeCopy;
    setActiveCopy({ id, stopping: true });
    const accepted = await invoke<boolean>('stop_copy', { requestId: id }).catch(() => false);
    if (!accepted) {
      setActiveCopy(current => current?.id === id ? { id, stopping: false } : current);
    }
  }

  return (
    <>
      <details className="file-mutation-tools" ref={menu}
        onBlur={event => {
          if (event.relatedTarget && !event.currentTarget.contains(event.relatedTarget)) event.currentTarget.open = false;
        }}
        onKeyDown={event => {
          if (event.key !== 'Escape' || !event.currentTarget.open) return;
          event.preventDefault();
          event.currentTarget.open = false;
          trigger.current?.focus();
        }}>
        <summary ref={trigger} className="tool-button" aria-label={labels.actions} title={labels.actions}
          aria-disabled={!directory || undefined}
          onClick={event => { if (!directory) event.preventDefault(); }}>
          <Ellipsis aria-hidden="true" />
        </summary>
        <div className="file-mutation-actions">
          {createActions(directory)}
          {entryActions(directory, selected)}
        </div>
      </details>
      {context && <div ref={contextActions} className="file-mutation-actions file-context-actions"
        role="group" aria-label={labels.actions} onContextMenu={event => event.preventDefault()}
        onKeyDown={event => {
          if (event.key === 'Escape') {
            event.preventDefault();
            event.stopPropagation();
            contextReturn.current = context.anchor ?? trigger.current;
            dismissContext();
          } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
            const buttons = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'));
            const index = buttons.findIndex(node => node === document.activeElement);
            const step = event.key === 'ArrowDown' ? 1 : -1;
            event.preventDefault();
            buttons[(index + step + buttons.length) % buttons.length]?.focus();
          }
        }}>
        {!context.selection ? <>
          {createActions(context.directory, context.anchor)}
          {context.directory === directory &&
            <button type="button" onClick={() => { dismissContext(); focusBack(context.anchor); selectAll(); }}><ListChecks aria-hidden="true" /> {labels.selectAll}</button>}
        </> : <>
          {context.selection.kind === 'directory' && <>
            <button type="button" onClick={() => browseFromContext(context.selection!.path, openFolder)}><FolderOpen aria-hidden="true" /> {strings.openPath}</button>
            <button type="button" onClick={() => browseFromContext(context.selection!.path, openTab)}><PanelTop aria-hidden="true" /> {strings.newTab}</button>
            <button type="button" onClick={() => browseFromContext(context.selection!.path, openWindow)}><SquareArrowOutUpRight aria-hidden="true" /> {strings.newWindow}</button>
          </>}
          {entryActions(context.directory, context.selection, true, context.anchor)}
        </>}
      </div>}
      <dialog ref={dialog} className="file-mutation-dialog" aria-labelledby={titleId}
        onCancel={event => { if (busy) event.preventDefault(); }}
        onClose={() => {
          setAction(null);
          focusBack(dialogReturn.current);
        }}>
        <form onSubmit={submit} aria-busy={busy}>
          <h2 id={titleId}>{title}</h2>
          {transferResult ? (
            <>
              <p role="status">{resultLabel} <strong>{transferResult.path}</strong></p>
              {transferResult.kind === 'copy' && transferResult.cleanupPending && <p role="status">{labels.cleanupPending}</p>}
              <div className="file-mutation-buttons">
                <button ref={closeResult} type="button" onClick={() => dialog.current?.close()}>{labels.close}</button>
              </div>
            </>
          ) : (
            <>
              <label htmlFor={nameId}>{copy || move ? labels.destination : strings.name}</label>
              <input ref={input} id={nameId} value={value} required autoFocus
                autoComplete="off" spellCheck={false} readOnly={busy || requiresReview}
                aria-invalid={error === 'invalid-request' || error === 'already-exists' || undefined}
                aria-describedby={error ? errorId : undefined}
                onChange={event => { setValue(event.target.value); setError(null); }} />
              {error && <p id={errorId} role="alert">
                {(copy || move) && error === 'invalid-request' ? strings.errors[error] : labels.errors[error]}
              </p>}
              {busy && <p role="status">{activeCopy?.stopping ? labels.stopping : labels.working}</p>}
              <div className="file-mutation-buttons">
                {activeCopy
                  ? <button type="button" disabled={activeCopy.stopping} onClick={stopCopy}>{labels.stop}</button>
                  : <button type="button" disabled={busy} onClick={() => dialog.current?.close()}>{strings.cancel}</button>}
                <button type="submit" disabled={!value || busy || requiresReview}>
                  {action?.kind === 'rename' ? labels.rename : copy ? labels.copy : move ? labels.move : labels.create}
                </button>
              </div>
            </>
          )}
        </form>
      </dialog>
    </>
  );
}
