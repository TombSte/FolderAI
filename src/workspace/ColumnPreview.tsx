import { File, Folder } from 'lucide-react';
import { useEffect, useMemo, useState, type KeyboardEvent, type MouseEvent } from 'react';
import { browse, type RequestContext } from '../ipc';
import { displayName, sortEntries, type Entry, type ViewOptions } from './fileViews';
import { locale, strings } from './strings';

type Props = {
  path: string;
  context: Pick<RequestContext, 'panelId' | 'tabId'>;
  options: ViewOptions;
  onNavigate: (path: string) => void;
  onContext: (event: MouseEvent | KeyboardEvent, entry: Entry | null, directory: string, anchor: HTMLElement) => void;
  refresh: number;
};

export function ColumnPreview({ path, context, options, onNavigate, onContext, refresh }: Props) {
  const collator = useMemo(() => new Intl.Collator(locale, { numeric: true, sensitivity: 'base' }), [locale]);
  const [data, setData] = useState<Entry[] | null>(null);
  const [failed, setFailed] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const { panelId, tabId } = context;
  useEffect(() => {
    const controller = new AbortController();
    const requestId = Array.from(crypto.getRandomValues(new Uint8Array(12)), byte => byte.toString(16).padStart(2, '0')).join('');
    setData(null);
    setFailed(false);
    browse({ kind: 'path', path }, { panelId, tabId, requestId }, controller.signal, options.showHidden)
      .then(result => { if (!controller.signal.aborted) setData(result.entries); })
      .catch(() => { if (!controller.signal.aborted) setFailed(true); });
    return () => controller.abort();
  }, [panelId, tabId, path, options.showHidden, refresh]);

  const entries = sortEntries(data ?? [], options, collator);
  const child = entries.find(entry => entry.location?.path === selected && entry.kind === 'directory');
  return (
    <>
      <div className="file-column" role="group" aria-label={path} data-preview-path={data ? path : undefined}
        tabIndex={-1} onContextMenu={event => { if (data) onContext(event, null, path, event.currentTarget); }}>
        {!data && <p className="folder-empty" role="status">{failed ? strings.errors.unavailable : strings.loading}</p>}
        {data && entries.length === 0 && <p className="folder-empty">{strings.empty}</p>}
        {entries.map((entry, index) => {
          const location = entry.location?.path;
          const Icon = entry.kind === 'directory' ? Folder : File;
          return <button type="button" key={location ?? index} className="column-entry file-name"
            aria-pressed={!!location && selected === location} title={entry.name}
            onClick={() => setSelected(location ?? null)}
            onContextMenu={event => {
              setSelected(location ?? null);
              onContext(event, entry, path, event.currentTarget);
            }}
            onDoubleClick={() => { if (entry.kind === 'directory' && location) onNavigate(location); }}
            onKeyDown={event => {
              if (event.key === 'ContextMenu' || (event.key === 'F10' && event.shiftKey)) {
                setSelected(location ?? null);
                onContext(event, entry, path, event.currentTarget);
                return;
              }
              const column = event.currentTarget.closest('.file-column');
              let next: HTMLButtonElement | null | undefined;
              switch (event.key) {
                case 'ArrowLeft':
                  next = column?.previousElementSibling?.querySelector<HTMLButtonElement>('[aria-pressed="true"]');
                  break;
                case 'ArrowRight':
                  if (entry.kind === 'directory' && location !== selected) setSelected(location ?? null);
                  else next = column?.nextElementSibling?.querySelector<HTMLButtonElement>('.column-entry');
                  break;
                case 'ArrowUp':
                  next = event.currentTarget.previousElementSibling as HTMLButtonElement | null;
                  break;
                case 'ArrowDown':
                  next = event.currentTarget.nextElementSibling as HTMLButtonElement | null;
                  break;
                case 'Enter':
                  if (!location || entry.kind !== 'directory') return;
                  onNavigate(location);
                  break;
                default:
                  return;
              }
              event.preventDefault();
              next?.focus();
            }}>
            <Icon aria-hidden="true" /><span>{displayName(entry, options.showExtensions)}</span>
          </button>;
        })}
      </div>
      {child?.location && <ColumnPreview key={child.location.path} path={child.location.path}
        context={context} options={options} onNavigate={onNavigate} onContext={onContext} refresh={refresh} />}
    </>
  );
}
