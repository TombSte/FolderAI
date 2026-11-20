import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { Activity, CircleAlert, X } from 'lucide-react';
import { useEffect, useRef, useState, type MouseEvent } from 'react';
import { fileActionLabels, fileErrorCode } from './FileMutationControls';
import { parentPath, pathLabel } from './paths';
import { locale } from './strings';
import './window-activity.scss';

type Phase = 'planning' | 'running' | 'waiting' | 'stopping' | 'finished';
type ItemStatus = 'pending' | 'running' | 'waiting' | 'completed' | 'skipped' | 'failed' | 'cancelled' | 'recovery-pending';
type Choice = 'retry' | 'skip' | 'stop';
type Decision = { id: string; itemIndex: number; error: string; scope: 'item' | 'destination' };
type Transfer = {
  id: string;
  kind: 'copy' | 'move';
  directory: string;
  phase: Phase;
  total: number;
  settled: number;
  completed: number;
  skipped: number;
  failed: number;
  decision: Decision | null;
  items: { source: string; status: ItemStatus; error?: string; path?: string; cleanupPending?: boolean; retainedPath?: string }[];
};

const english = {
  activity: 'Activities',
  noActivity: 'No transfers in this window.',
  wait: 'A transfer needs a decision',
  skip: 'Skip',
  retry: 'Retry',
  verify: 'Retry verification',
  stop: 'Stop',
  similar: 'Apply to the next similar errors',
  destinationError: 'Destination',
  details: 'Item details',
  close: 'Close activities',
  closeQuestion: 'Stop this window’s transfers before closing?',
  stay: 'Keep window open',
  stopAndClose: 'Stop and close',
  completed: 'Completed',
  skipped: 'Skipped',
  failed: 'Failed',
  phase: { planning: 'Planning', running: 'Running', waiting: 'Waiting for a decision', stopping: 'Stopping', finished: 'Finished' },
  item: { pending: 'Pending', running: 'Running', waiting: 'Waiting', completed: 'Completed', skipped: 'Skipped',
    failed: 'Failed', cancelled: 'Stopped', 'recovery-pending': 'Verification pending' },
};

const italian: typeof english = {
  activity: 'Attività',
  noActivity: 'Nessun trasferimento in questa finestra.',
  wait: 'Un trasferimento richiede una decisione',
  skip: 'Salta',
  retry: 'Riprova',
  verify: 'Riprova verifica',
  stop: 'Interrompi',
  similar: 'Applica ai prossimi errori simili',
  destinationError: 'Destinazione',
  details: 'Dettaglio degli elementi',
  close: 'Chiudi attività',
  closeQuestion: 'Interrompere i trasferimenti di questa finestra prima di chiuderla?',
  stay: 'Mantieni aperta',
  stopAndClose: 'Interrompi e chiudi',
  completed: 'Completati',
  skipped: 'Saltati',
  failed: 'Falliti',
  phase: { planning: 'Pianificazione', running: 'In corso', waiting: 'In attesa di una decisione', stopping: 'Interruzione', finished: 'Terminato' },
  item: { pending: 'In attesa', running: 'In corso', waiting: 'In attesa', completed: 'Completato', skipped: 'Saltato',
    failed: 'Fallito', cancelled: 'Interrotto', 'recovery-pending': 'Verifica in sospeso' },
};

export function activityLabels() {
  return locale === 'it' ? italian : english;
}

export function useTransferActivities() {
  const [transfers, setTransfers] = useState<Transfer[]>([]);
  const [fileChanges, setFileChanges] = useState<ReadonlyMap<string, number>>(new Map());
  useEffect(() => {
    let disposed = false;
    let pending = false;
    let queued = false;
    let timer: number | undefined;
    let unlisten: (() => void) | undefined;
    const seen = new Map<string, ItemStatus>();
    async function refresh() {
      if (disposed) return;
      if (pending) {
        queued = true;
        return;
      }
      pending = true;
      try {
        const latest = await invoke<Transfer[]>('list_transfers');
        if (disposed || !Array.isArray(latest)) return;
        const changes = new Set<string>();
        for (const transfer of latest) {
          transfer.items.forEach((item, index) => {
            const key = transfer.id + '/' + index;
            if (seen.get(key) === item.status) return;
            seen.set(key, item.status);
            if (item.status === 'completed' || item.status === 'recovery-pending') {
              changes.add(transfer.directory);
              const sourceDirectory = parentPath(item.source);
              if (sourceDirectory && transfer.kind === 'move') changes.add(sourceDirectory);
            }
          });
        }
        setTransfers(latest);
        if (changes.size) setFileChanges(current => {
          const next = new Map(current);
          for (const path of changes) next.set(path, (next.get(path) ?? 0) + 1);
          return next;
        });
      } catch {
        // Another event can refresh the current window after a transient IPC failure.
      } finally {
        pending = false;
        if (queued) {
          queued = false;
          scheduleRefresh();
        }
      }
    }
    function scheduleRefresh() {
      if (disposed || timer !== undefined) return;
      timer = window.setTimeout(() => {
        timer = undefined;
        void refresh();
      }, 60);
    }
    void refresh();
    void Promise.resolve().then(() => getCurrentWebviewWindow().listen('file-transfers-changed', scheduleRefresh))
      .then(stop => {
        if (disposed) stop();
        else { unlisten = stop; scheduleRefresh(); }
      }).catch(() => {});
    return () => { disposed = true; window.clearTimeout(timer); unlisten?.(); };
  }, []);
  return { transfers, fileChanges };
}

export function ActivityToggle({ transfers, open, toggle }: {
  transfers: Transfer[];
  open: boolean;
  toggle: (event: MouseEvent<HTMLButtonElement>) => void;
}) {
  const t = activityLabels();
  const waiting = transfers.filter(transfer => transfer.phase === 'waiting').length;
  const active = transfers.filter(transfer => transfer.phase !== 'finished').length;
  return <button type="button" className="workspace-activity-toggle" aria-expanded={open}
    aria-controls="workspace-activity" onClick={toggle}>
    {waiting > 0 ? <CircleAlert aria-hidden="true" className="activity-waiting" /> : <Activity aria-hidden="true" />}
    {t.activity}{active > 0 && <span aria-hidden="true">{active}</span>}
    <span className="activity-announcement" role="status">{waiting > 0 ? t.wait : ''}</span>
  </button>;
}

function TransferRow({ transfer }: { transfer: Transfer }) {
  const t = activityLabels();
  const labels = fileActionLabels();
  const [applyForDecision, setApplyForDecision] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function operate(command: string, args: Record<string, unknown>) {
    setError(null);
    setBusy(true);
    try {
      await invoke(command, args);
    } catch (problem) {
      setError(labels.errors[fileErrorCode(problem)]);
    } finally {
      setBusy(false);
    }
  }
  const decision = transfer.decision;
  const applying = decision?.scope === 'item' && applyForDecision === decision.id;
  const decide = (choice: Choice) => decision && operate('decide_transfer', {
    request: { id: transfer.id, decisionId: decision.id, choice, applyToSimilar: choice !== 'stop' && applying },
  });
  return <li className="activity-transfer">
    <div className="activity-transfer-heading">
      <strong>{transfer.kind === 'copy' ? labels.copy : labels.move}</strong>
      <span>{t.phase[transfer.phase]}</span>
    </div>
    <p title={transfer.directory}>{transfer.directory}</p>
    {transfer.phase !== 'finished' && <progress aria-label={t.phase[transfer.phase]} max={transfer.total} value={transfer.settled} />}
    <p>{t.completed} {transfer.completed} · {t.skipped} {transfer.skipped} · {t.failed} {transfer.failed}</p>
    {decision ? <div className="activity-decision">
      <p><strong>{decision.scope === 'destination' ? t.destinationError : pathLabel(transfer.items[decision.itemIndex].source)}</strong>
        {' '}{labels.errors[fileErrorCode(decision.error)]}</p>
      {decision.scope === 'item' && <label><input type="checkbox" checked={applying}
        onChange={event => setApplyForDecision(event.target.checked ? decision.id : null)} />{t.similar}</label>}
      <div className="activity-buttons">
        <button type="button" disabled={busy} onClick={() => void decide('retry')}>{t.retry}</button>
        {decision.scope === 'item' && <button type="button" disabled={busy} onClick={() => void decide('skip')}>{t.skip}</button>}
        <button type="button" disabled={busy} onClick={() => void decide('stop')}>{t.stop}</button>
      </div>
    </div> : transfer.phase !== 'finished' && <div className="activity-buttons">
      <button type="button" disabled={busy || transfer.phase === 'stopping'}
        onClick={() => void operate('stop_transfer', { id: transfer.id })}>{t.stop}</button>
    </div>}
    <details>
      <summary>{t.details}</summary>
      <ul className="activity-items">{transfer.items.map((item, itemIndex) => <li key={item.source}>
        <strong title={item.source}>{pathLabel(item.source)}</strong>
        <span>{t.item[item.status]}</span>
        {item.path && <small>{item.path}</small>}
        {item.retainedPath && <small>{labels.stagedFile} {item.retainedPath}</small>}
        {item.error && <small>{labels.errors[fileErrorCode(item.error)]}</small>}
        {item.cleanupPending && <small>{labels.cleanupPending}</small>}
        {item.status === 'recovery-pending' && item.path &&
          <button type="button" disabled={busy} onClick={() => void operate('recover_copy_transfer', {
            requestId: transfer.id, itemIndex,
          })}>{t.verify}</button>}
      </li>)}</ul>
    </details>
    {error && <p role="alert">{error}</p>}
  </li>;
}

export function WindowActivity({ transfers, close }: { transfers: Transfer[]; close: () => void }) {
  const t = activityLabels();
  const button = useRef<HTMLButtonElement>(null);
  useEffect(() => { button.current?.focus(); }, []);
  return <aside id="workspace-activity" className="window-activity" aria-label={t.activity}
    onKeyDown={event => { if (event.key === 'Escape') { event.stopPropagation(); close(); } }}>
    <header><h2>{t.activity}</h2><button ref={button} type="button" className="tool-button"
      aria-label={t.close} title={t.close} onClick={close}><X aria-hidden="true" /></button></header>
    {transfers.length ? <ul>{[...transfers].reverse().map(transfer => <TransferRow key={transfer.id} transfer={transfer} />)}</ul>
      : <p>{t.noActivity}</p>}
  </aside>;
}
