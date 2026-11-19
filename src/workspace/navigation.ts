import { useCallback, useEffect, useRef, useState } from 'react';
import { browse, type BrowseError, type Directory, type Location } from '../ipc';

type Target = Parameters<typeof browse>[0];

type ViewPosition = { selected: string | null; selectedRows?: string[]; scrollTop: number; scrollLeft: number };
export type Destination = {
  target: Target;
  source: 'browse' | 'address' | 'refresh';
  historyIndex?: number;
  showHidden?: boolean;
  selectPath?: string;
};

type Visit = { location: Location; view: ViewPosition };
type Failure = { destination: Destination; code: BrowseError['code'] };
type Navigation = {
  directory: Directory | null;
  home: Location | null;
  visits: Visit[];
  index: number;
  pending: Destination | null;
  failure: Failure | null;
  view: ViewPosition | null;
};

const initialDestination: Destination = { target: { kind: 'home' }, source: 'browse' };
const emptyView: ViewPosition = { selected: null, scrollTop: 0, scrollLeft: 0 };
const initialNavigation: Navigation = {
  directory: null,
  home: null,
  visits: [],
  index: -1,
  pending: initialDestination,
  failure: null,
  view: null,
};

function committed(
  current: Navigation,
  directory: Directory,
  destination: Destination,
  previousView: ViewPosition,
): Navigation {
  const visits = [...current.visits];
  const previous = visits[current.index];
  if (previous) visits[current.index] = { ...previous, view: previousView };

  let index = destination.historyIndex ?? current.index;
  let view = visits[index]?.view ?? emptyView;
  if (destination.historyIndex === undefined && previous?.location.path !== directory.location.path) {
    index++;
    visits.splice(index);
    view = emptyView;
  }
  if (destination.selectPath) {
    view = { ...view, selected: destination.selectPath, selectedRows: [destination.selectPath] };
  }
  visits[index] = { location: directory.location, view };

  return {
    directory,
    home: destination.target.kind === 'home' ? directory.location : current.home,
    visits,
    index,
    pending: null,
    failure: null,
    view: { ...view },
  };
}

function errorCode(error: unknown): BrowseError['code'] {
  const code = typeof error === 'object' && error !== null && 'code' in error ? error.code : undefined;
  switch (code) {
    case 'invalid-request':
    case 'not-found':
    case 'permission-denied':
    case 'not-directory':
    case 'cancelled':
    case 'unavailable':
      return code;
    default:
      return 'unavailable';
  }
}

export function useWorkspaceNavigation(panelId: string, tabId: string, startPath: string | null, fallback: boolean, selection: string | null) {
  const [navigation, setNavigation] = useState(initialNavigation);
  const [usedFallback, setUsedFallback] = useState(false);
  const active = useRef<{ controller: AbortController; source: Destination['source'] } | null>(null);
  const context = useRef({ panelId, tabId });
  const initialSelection = useRef(selection);
  useEffect(() => { context.current = { panelId, tabId }; }, [panelId, tabId]);

  const open = useCallback(async (destination: Destination, previousView = emptyView, startup = false) => {
    active.current?.controller.abort();
    const controller = new AbortController();
    active.current = { controller, source: destination.source };
    setNavigation((current) => ({ ...current, pending: destination, failure: null }));

    if (!startup) setUsedFallback(false);
    let committedDestination = destination;
    try {
      const requestId = Array.from(crypto.getRandomValues(new Uint8Array(16)),
        (byte) => byte.toString(16).padStart(2, '0')).join('');
      const request = { ...context.current, requestId };
      let directory: Directory;
      try {
        directory = await browse(destination.target, request, controller.signal, destination.showHidden);
      } catch (error) {
        if (!startup || !fallback || destination.target.kind !== 'path' || controller.signal.aborted) throw error;
        setUsedFallback(true);
        committedDestination = initialDestination;
        directory = await browse(initialDestination.target, { ...request, requestId: requestId + '-home' }, controller.signal, destination.showHidden);
      }
      if (active.current?.controller !== controller) return false;
      setNavigation((current) => {
        const next = committed(current, directory, committedDestination, previousView);
        return startup && initialSelection.current && next.view
          ? { ...next, view: { ...next.view, selected: initialSelection.current } }
          : next;
      });
      return true;
    } catch (error) {
      if (active.current?.controller !== controller) return false;
      const code = errorCode(error);
      setNavigation((current) => ({ ...current, pending: null, failure: { destination: committedDestination, code } }));
      return false;
    } finally {
      if (active.current?.controller === controller) active.current = null;
    }
  }, [fallback]);

  const cancelAddress = useCallback(() => {
    if (active.current?.source === 'address') {
      active.current.controller.abort();
      active.current = null;
    }
    setNavigation((current) => {
      const pending = current.pending?.source === 'address' ? null : current.pending;
      const failure = current.failure?.destination.source === 'address' ? null : current.failure;
      return pending === current.pending && failure === current.failure ? current : { ...current, pending, failure };
    });
  }, []);

  useEffect(() => {
    void open(startPath === null ? initialDestination : { target: { kind: 'path', path: startPath }, source: 'browse' }, emptyView, true);
    return () => {
      active.current?.controller.abort();
      active.current = null;
    };
  }, [open, startPath]);

  return { navigation, open, cancelAddress, usedFallback };
}
