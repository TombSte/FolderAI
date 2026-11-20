import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Archive, FileArchive, Trash2, Undo2, Copy, Ellipsis, FilePlus2, FolderInput, FolderOpen, FolderPlus, ListChecks, PanelTop, Pencil, Replace, SquareArrowOutUpRight } from 'lucide-react';
import { useEffect, useId, useLayoutEffect, useRef, useState, type ComponentPropsWithRef, type FormEvent } from 'react';
import { parentPath } from './paths';
import { locale, strings } from './strings';
import './file-mutations.scss';

export function WorkspaceContextMenu(props: ComponentPropsWithRef<'div'>) {
  return <div {...props} className="file-mutation-actions file-context-actions" role="group"
    onContextMenu={event => event.preventDefault()} />;
}

export type FileActionSelection = { path: string; name: string; kind: 'file' | 'directory' };
export type FileActionContext = {
  directory: string;
  selection: FileActionSelection | null;
  sources?: FileActionSelection[];
  x: number;
  y: number;
  anchor: HTMLElement | null;
};
type Action =
  | { kind: 'create'; directory: string; isDirectory: boolean }
  | { kind: 'rename'; directory: string; path: string }
  | { kind: 'transfer'; transfer: 'copy' | 'move'; sources: string[] }
  | { kind: 'replace'; source: string };
type Request =
  | { kind: 'create'; directory: string; name: string; isDirectory: boolean }
  | { kind: 'rename'; path: string; name: string };
type CopyResult = { path: string; cleanupPending: boolean; recoveryPending?: boolean; stagingPath?: string };
type ReplacementApproval = { path: string; requestId: string };
type TransferResult = CopyResult & { requestId: string };

const english = {
  actions: 'File actions',
  newFile: 'New empty file',
  newFolder: 'New folder',
  selectAll: 'Select all',
  rename: 'Rename',
  copyToFolder: 'Copy to folder',
  moveToFolder: 'Move to folder…',
  replaceFile: 'Replace existing file…',
  replace: 'Replace',
  review: 'Review',
  targetFile: 'Path to the file to replace',
  confirmReplace: 'Replace the contents of this file?',
  stagedFile: 'File retained in',
  create: 'Create',
  copy: 'Copy',
  move: 'Move',
  destination: 'Destination folder',
  copiedTo: 'Copied to',
  cleanupPending: 'The copy finished. Temporary file cleanup is pending.',
  verifyCopy: 'Retry verification',
  verifyCopyPending: 'The copy needs verification. Retrying checks the existing data.',
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
  replaceFile: 'Sostituisci file esistente…',
  replace: 'Sostituisci',
  review: 'Verifica',
  targetFile: 'Percorso del file da sostituire',
  confirmReplace: 'Sostituire il contenuto di questo file?',
  stagedFile: 'File conservato in',
  create: 'Crea',
  copy: 'Copia',
  move: 'Sposta',
  destination: 'Cartella di destinazione',
  copiedTo: 'Copiato in',
  cleanupPending: 'La copia è terminata. La pulizia dei file temporanei è in sospeso.',
  verifyCopy: 'Riprova verifica',
  verifyCopyPending: 'La copia richiede una verifica. Il nuovo tentativo controlla i dati già presenti.',
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

export type FileErrorCode = keyof typeof english.errors;

export function fileActionLabels() {
  return locale === 'it' ? italian : english;
}

export function fileErrorCode(error: unknown): FileErrorCode {
  return typeof error === 'string' && Object.hasOwn(english.errors, error) ? error as FileErrorCode : 'unavailable';
}

export function FileMutationControls({ directory, selected, batchSelection, onChanged, context, dismissContext, selectAll, openFolder, openTab, openWindow }: {
  directory: string | null;
  selected: FileActionSelection | null;
  batchSelection: FileActionSelection[];
  onChanged: (directory: string) => boolean;
  context: FileActionContext | null;
  dismissContext: () => void;
  selectAll: () => void;
  openFolder: (path: string) => void;
  openTab: (path: string) => void;
  openWindow: (path: string) => void;
}) {
  const labels = fileActionLabels();
  const isItalian = locale === 'it';
  const [undoAvailable, setUndoAvailable] = useState(false);
  const [removing, setRemoving] = useState(false);

  useEffect(() => {
    let disposed = false;
    function refresh() {
      void invoke<boolean>('file_undo_available').then(value => {
        if (!disposed) setUndoAvailable(value);
      }).catch(() => {});
    }
    const subscription = listen('file-undo-changed', refresh);
    void subscription.then(refresh).catch(() => {});
    return () => {
      disposed = true;
      void subscription.then(unlisten => unlisten()).catch(() => {});
    };
  }, []);

  async function removeOrUndo(target: string, sources?: string[]) {
    if (removing) return;
    dismissContext();
    if (menu.current) menu.current.open = false;
    setRemoving(true);
    try {
      if (sources) {
        const { problem } = await invoke<{ problem?: { path: string; error: string; retainedPath?: string } }>('trash_files', { sources });
        changed.current?.(target);
        if (problem) window.alert([labels.errors[fileErrorCode(problem.error)], problem.path, problem.retainedPath].filter(Boolean).join('\n'));
      } else {
        const result = await invoke<{ directories: string[]; error?: string }>('undo_last_file_action');
        for (const path of result.directories) changed.current?.(path);
        if (result.error) window.alert(labels.errors[fileErrorCode(result.error)]);
      }
    } catch (error) {
      window.alert(labels.errors[fileErrorCode(error)]);
    } finally {
      setRemoving(false);
    }
  }

  const [zipBusy, setZipBusy] = useState(false);
  const [zipPreview, setZipPreview] = useState<{ title: string; entries: { path: string; extractable: boolean }[] } | null>(null);

  async function zipAction(target: string, item: FileActionSelection, action: 'inspect' | 'extract' | 'compress') {
    if (zipBusy) return;
    dismissContext();
    if (menu.current) menu.current.open = false;
    setZipBusy(true);
    try {
      if (action !== 'inspect') {
        if (action === 'extract') await invoke('extract_zip_here', { source: item.path });
        else await invoke('compress_entry_zip', { source: item.path, isDirectory: item.kind === 'directory' });
        changed.current?.(target);
      } else {
        const entries = await invoke<{ path: string; extractable: boolean }[]>('inspect_zip', { source: item.path });
        setZipPreview({ title: item.name, entries });
      }
    } catch (error) {
      const failure = error as { code?: string; path?: string } | null;
      window.alert([isItalian ? 'Operazione ZIP non riuscita' : 'ZIP operation failed', failure?.code ?? String(error), failure?.path].filter(Boolean).join('\n'));
    } finally {
      setZipBusy(false);
    }
  }

  function trashAction(target: string, sources: string[]) {
    if (!/Macintosh|Windows/.test(navigator.userAgent)) return null;
    return <button type="button" disabled={removing} onClick={() => void removeOrUndo(target, sources)}>
      <Trash2 aria-hidden="true" /> {isItalian ? 'Sposta nel Cestino' : 'Move to Trash'}
    </button>;
  }
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
  const [error, setError] = useState<FileErrorCode | null>(null);
  const [approval, setApproval] = useState<ReplacementApproval | null>(null);
  const [busy, setBusy] = useState(false);
  const [activeCopy, setActiveCopy] = useState<{ id: string; stopping: boolean } | null>(null);
  const copy = action?.kind === 'transfer' && action.transfer === 'copy';
  const move = action?.kind === 'transfer' && action.transfer === 'move';
  const replace = action?.kind === 'replace';
  const title = action?.kind === 'rename' ? labels.rename
    : replace ? labels.replaceFile
    : copy ? labels.copyToFolder
    : move ? labels.moveToFolder
    : action?.kind === 'create' && action.isDirectory ? labels.newFolder : labels.newFile;
  const requiresReview = error === 'recovery-pending' || error === 'cleanup-failed';


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
    setApproval(null);
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
      {target && undoAvailable && <button type="button" disabled={removing} onClick={() => void removeOrUndo(target)}>
        <Undo2 aria-hidden="true" /> {isItalian ? 'Annulla ultima operazione' : 'Undo last file action'}
      </button>}
    </>;
  }

  function entryActions(target: string | null, item: FileActionSelection | null, anchor = trigger.current) {
    return <>
      <button type="button" disabled={!item || !target}
        onClick={() => target && item && start({ kind: 'rename', directory: target, path: item.path }, item.name, anchor)}>
        <Pencil aria-hidden="true" /> {labels.rename}
      </button>
      <button type="button" disabled={!target || !item}
        onClick={() => target && item && start({ kind: 'transfer', transfer: 'copy', sources: [item.path] }, target, anchor)}>
        <Copy aria-hidden="true" /> {labels.copyToFolder}
      </button>
      <button type="button" disabled={!target || !item}
        onClick={() => target && item && start({ kind: 'transfer', transfer: 'move', sources: [item.path] }, target, anchor)}>
        <FolderInput aria-hidden="true" /> {labels.moveToFolder}
      </button>
      {item?.kind === 'file' && navigator.userAgent.includes('Macintosh') && <button type="button" disabled={!target}
        onClick={() => target && start({ kind: 'replace', source: item.path }, '', anchor)}>
        <Replace aria-hidden="true" /> {labels.replaceFile}
      </button>}
      {target && item && trashAction(target, [item.path])}
      {target && item && <button type="button" disabled={zipBusy}
        onClick={() => void zipAction(target, item, 'compress')}>
        <Archive aria-hidden="true" /> {isItalian ? 'Comprimi in ZIP' : 'Compress to ZIP'}
      </button>}
      {target && item?.kind === 'file' && /\.zip$/i.test(item.name) && <>
        <button type="button" disabled={zipBusy} onClick={() => void zipAction(target, item, 'inspect')}>
          <Archive aria-hidden="true" /> {isItalian ? 'Mostra contenuto ZIP' : 'View ZIP contents'}
        </button>
        <button type="button" disabled={zipBusy} onClick={() => void zipAction(target, item, 'extract')}>
          <FileArchive aria-hidden="true" /> {isItalian ? 'Estrai qui' : 'Extract here'}
        </button>
      </>}
    </>;
  }

  function batchActions(target: string, items: FileActionSelection[], anchor = trigger.current) {
    const sources = items.map(item => item.path);
    return <>
      <button type="button" onClick={() => start({ kind: 'transfer', transfer: 'copy', sources }, target, anchor)}>
        <Copy aria-hidden="true" /> {labels.copyToFolder}
      </button>
      <button type="button" onClick={() => start({ kind: 'transfer', transfer: 'move', sources }, target, anchor)}>
        <FolderInput aria-hidden="true" /> {labels.moveToFolder}
      </button>
      {trashAction(target, sources)}
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
    const destination = 'directory' in action ? action.directory : value;
    setBusy(true);
    setError(null);
    try {
      if (action.kind === 'transfer') {
        await invoke('start_transfer', { request: { kind: action.transfer, sources: action.sources, directory: destination } });
      } else if (action.kind === 'replace') {
        if (!approval) {
          setApproval(await invoke<ReplacementApproval>('prepare_file_replacement', { path: destination }));
          return;
        }
        setActiveCopy({ id: approval.requestId, stopping: false });
        const result = await invoke<CopyResult>('replace_regular_file', { source: action.source, requestId: approval.requestId });
        changed.current?.(parentPath(result.path) ?? destination);
        setTransferResult({ requestId: approval.requestId, ...result });
        return;
      } else {
        const request: Request = action.kind === 'rename'
          ? { kind: 'rename', path: action.path, name: value }
          : { ...action, name: value };
        await invoke<string>('mutate_file', { request });
        changed.current?.(destination);
      }
    } catch (error) {
      const code = fileErrorCode(error);
      setError(code);
      if (action.kind === 'replace') setApproval(null);
      if (code === 'recovery-pending' || code === 'cleanup-failed') {
        changed.current?.(action.kind === 'replace' ? parentPath(destination) ?? destination : destination);
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

  async function retryCopy() {
    if (!transferResult?.recoveryPending || busy) return;
    setBusy(true);
    setError(null);
    try {
      const result = await invoke<CopyResult>('recover_copy_transfer', { requestId: transferResult.requestId });
      setTransferResult({ requestId: transferResult.requestId, ...result });
      changed.current?.(parentPath(result.path) ?? result.path);
    } catch (error) {
      setError(fileErrorCode(error));
    } finally {
      setBusy(false);
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
          {directory && batchSelection.length > 1 ? batchActions(directory, batchSelection) : entryActions(directory, selected)}
        </div>
      </details>
      {context && <WorkspaceContextMenu ref={contextActions} aria-label={labels.actions}
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
        {context.sources ? batchActions(context.directory, context.sources, context.anchor) : !context.selection ? <>
          {createActions(context.directory, context.anchor)}
          {context.directory === directory &&
            <button type="button" onClick={() => { dismissContext(); focusBack(context.anchor); selectAll(); }}><ListChecks aria-hidden="true" /> {labels.selectAll}</button>}
        </> : <>
          {context.selection.kind === 'directory' && <>
            <button type="button" onClick={() => browseFromContext(context.selection!.path, openFolder)}><FolderOpen aria-hidden="true" /> {strings.openPath}</button>
            <button type="button" onClick={() => browseFromContext(context.selection!.path, openTab)}><PanelTop aria-hidden="true" /> {strings.newTab}</button>
            <button type="button" onClick={() => browseFromContext(context.selection!.path, openWindow)}><SquareArrowOutUpRight aria-hidden="true" /> {strings.newWindow}</button>
          </>}
          {entryActions(context.directory, context.selection, context.anchor)}
        </>}
      </WorkspaceContextMenu>}
      {zipBusy && <p role="status">{labels.working}</p>}
      {zipPreview && <dialog ref={node => { if (node && !node.open) node.showModal(); }}
        aria-label={zipPreview.title} onClose={() => setZipPreview(null)}>
        <h2>{zipPreview.title}</h2>
        <ul style={{ maxHeight: '50vh', overflow: 'auto', overflowWrap: 'anywhere' }}>
          {zipPreview.entries.map(entry => <li key={entry.path}>{entry.path}{!entry.extractable &&
            (isItalian ? ' (non estraibile)' : ' (not extractable)')}</li>)}
        </ul>
        <form method="dialog"><button>{isItalian ? 'Chiudi' : 'Close'}</button></form>
      </dialog>}
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
              <p role="status">
                {transferResult.recoveryPending ? labels.verifyCopyPending : labels.copiedTo}
                {' '}<strong>{transferResult.path}</strong>
              </p>
              {transferResult.cleanupPending && <p role="status">{labels.cleanupPending}</p>}
              {transferResult.stagingPath &&
                <p>{labels.stagedFile} <strong>{transferResult.stagingPath}</strong></p>}
              {error && <p role="alert">{labels.errors[error]}</p>}
              <div className="file-mutation-buttons">
                {transferResult.recoveryPending &&
                  <button type="button" disabled={busy} onClick={retryCopy}>{labels.verifyCopy}</button>}
                <button ref={closeResult} type="button" disabled={busy} onClick={() => dialog.current?.close()}>{labels.close}</button>
              </div>
            </>
          ) : (
            <>
              {approval ? <p>{labels.confirmReplace} <strong>{approval.path}</strong></p> : <>
              <label htmlFor={nameId}>{replace ? labels.targetFile : copy || move ? labels.destination : strings.name}</label>
              <input ref={input} id={nameId} value={value} required autoFocus
                autoComplete="off" spellCheck={false} readOnly={busy || requiresReview}
                aria-invalid={error === 'invalid-request' || error === 'already-exists' || undefined}
                aria-describedby={error ? errorId : undefined}
                onChange={event => { setValue(event.target.value); setError(null); }} />
              </>}
              {error && <p id={errorId} role="alert">
                {(copy || move) && error === 'invalid-request' ? strings.errors[error] : labels.errors[error]}
              </p>}
              {busy && <p role="status">{activeCopy?.stopping ? labels.stopping : labels.working}</p>}
              <div className="file-mutation-buttons">
                {activeCopy
                  ? <button type="button" disabled={activeCopy.stopping} onClick={stopCopy}>{labels.stop}</button>
                  : <button type="button" disabled={busy} onClick={() => dialog.current?.close()}>{strings.cancel}</button>}
                <button type="submit" disabled={!value || busy || requiresReview}>
                  {action?.kind === 'rename' ? labels.rename : copy ? labels.copy : move ? labels.move
                    : replace ? approval ? labels.replace : labels.review : labels.create}
                </button>
              </div>
            </>
          )}
        </form>
      </dialog>
    </>
  );
}
