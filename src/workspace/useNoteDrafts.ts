import { useRef, useState } from 'react';
import { loadOrganization, updateOrganization, type OrganizedItem } from './organization';

type Draft = {
  body: string;
  revision: number;
  status: 'pending' | 'saving' | 'saved' | 'conflict' | 'error';
};

export function useNoteDrafts(onSaved: () => void) {
  const [drafts, setDrafts] = useState<Record<string, Draft>>({});
  const current = useRef(drafts);
  const timers = useRef(new Map<string, ReturnType<typeof setTimeout>>());
  const writes = useRef(new Map<string, Promise<void>>());

  function publish(id: string, draft: Draft) {
    current.current = { ...current.current, [id]: draft };
    setDrafts(current.current);
  }

  function cancelTimer(id: string) {
    clearTimeout(timers.current.get(id));
    timers.current.delete(id);
  }

  function schedule(id: string) {
    cancelTimer(id);
    timers.current.set(id, setTimeout(() => {
      timers.current.delete(id);
      void save(id);
    }, 350));
  }

  function save(id: string, resolution?: number): Promise<void> {
    const running = writes.current.get(id);
    if (running) return running;
    const writing = write(id, resolution).finally(() => writes.current.delete(id));
    writes.current.set(id, writing);
    return writing;
  }

  async function write(id: string, resolution?: number) {
    const draft = current.current[id];
    if (!draft || draft.status === 'saving' || (draft.status === 'conflict' && resolution === undefined)) return;
    cancelTimer(id);
    publish(id, { ...draft, status: 'saving' });
    try {
      const revision = await updateOrganization({
        kind: 'save-note', itemId: id, expectedRevision: resolution ?? draft.revision, body: draft.body,
      });
      if (revision === null) throw new Error();
      const latest = current.current[id];
      const status = latest.body === draft.body ? 'saved' : 'pending';
      publish(id, { ...latest, revision, status });
      onSaved();
      if (status === 'pending') schedule(id);
    } catch (error) {
      publish(id, { ...current.current[id], status: error === 'revision-conflict' ? 'conflict' : 'error' });
    }
  }

  function edit(item: OrganizedItem, body: string) {
    const draft = current.current[item.id];
    const revision = draft && draft.status !== 'saved' ? draft.revision
      : Math.max(draft?.revision ?? 0, item.note?.revision ?? 0);
    const status = draft?.status === 'saving' || draft?.status === 'conflict' ? draft.status : 'pending';
    publish(item.id, { body, revision, status });
    if (status === 'pending') schedule(item.id);
  }

  async function resolve(item: OrganizedItem) {
    try {
      const latest = (await loadOrganization([item.path])).items.find(row => row.id === item.id);
      if (latest && current.current[item.id]?.status === 'conflict') {
        await save(item.id, latest.note?.revision ?? 0);
      }
    } catch {
      // Keep the conflict and the draft visible if the current revision could not be read.
    }
  }

  async function flush(retry = false) {
    await Promise.all(writes.current.values());
    const eligible = Object.entries(current.current).filter(([, draft]) =>
      draft.status === 'pending' || (retry && draft.status === 'error'));
    await Promise.all(eligible.map(([id]) => save(id)));
    return Object.values(current.current).every(draft => draft.status === 'saved');
  }

  return { drafts, edit, save, resolve, flush };
}
