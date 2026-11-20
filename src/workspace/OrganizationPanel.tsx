import { WorkspaceContextMenu } from './FileMutationControls';
import Markdown from 'react-markdown';
import { Ellipsis, Folder, Pencil, Plus, Star, Tag as TagIcon, X } from 'lucide-react';
import { useId, useRef, useState, type FormEvent } from 'react';
import { pathLabel } from './paths';
import { organizationStrings, type Favorite, type Organization, type OrganizationChange, type OrganizedItem, type SelectionDetails, type Tag } from './organization';
import type { useNoteDrafts } from './useNoteDrafts';

type Mutate = (change: OrganizationChange) => Promise<boolean>;
const favoriteDrag = 'application/x-folderai-favorite';

export function Favorites({ data, mutate, open }: { data: Organization; mutate: Mutate; open: (favorite: Favorite) => void }) {
  const t = organizationStrings();
  const [editing, setEditing] = useState<{ id: string | null; name: string } | null>(null);
  const [dragged, setDragged] = useState<string | null>(null);
  const [preview, setPreview] = useState<{ groupId: string | null; slot: number } | null>(null);
  const menuId = useId();
  const favoriteMenuPoint = useRef<{ x: number; y: number } | null>(null);

  async function saveGroup(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const name = String(new FormData(event.currentTarget).get('name')).trim();
    if (name && await mutate(editing?.id
      ? { kind: 'rename-group', id: editing.id, name }
      : { kind: 'create-group', name })) setEditing(null);
  }

  function list(groupId: string | null, name: string) {
    const links = data.favorites.filter(favorite => favorite.groupId === groupId);
    function insertionSlot(group: HTMLElement, y: number) {
      const rows = Array.from(group.querySelectorAll(':scope > .favorite-row'));
      const next = rows.findIndex(row => {
        const rect = row.getBoundingClientRect();
        return y < rect.top + rect.height / 2;
      });
      return next < 0 ? links.length : next;
    }

    return (
      <div key={groupId ?? 'root'} className="favorite-group" data-drop-target={!!dragged}
        data-insert-end={preview?.groupId === groupId && preview.slot === links.length}
        onDragOver={event => {
          if (!dragged) return;
          event.preventDefault();
          event.dataTransfer.dropEffect = 'move';
          const slot = insertionSlot(event.currentTarget, event.clientY);
          setPreview(current => current?.groupId === groupId && current.slot === slot ? current : { groupId, slot });
        }}
        onDragLeave={event => {
          if (!(event.relatedTarget instanceof Node) || !event.currentTarget.contains(event.relatedTarget)) {
            setPreview(current => current?.groupId === groupId ? null : current);
          }
        }}
        onDrop={event => {
          if (!dragged) return;
          event.preventDefault();
          const slot = insertionSlot(event.currentTarget, event.clientY);
          const from = links.findIndex(link => link.id === dragged);
          const index = slot - (from >= 0 && from < slot ? 1 : 0);
          if (index !== from) void mutate({ kind: 'move-favorite', id: dragged, groupId, index });
          setDragged(null); setPreview(null);
        }}>
        {groupId && <div className="organization-row"><Folder aria-hidden="true" /><span title={name}>{name}</span>
          <button type="button" className="tool-button" title={t.renameGroup} aria-label={`${t.renameGroup} ${name}`}
            onClick={() => setEditing({ id: groupId, name })}><Pencil aria-hidden="true" /></button>
          <button type="button" className="tool-button" title={t.removeGroup} aria-label={`${t.removeGroup} ${name}`}
            onClick={() => void mutate({ kind: 'remove-group', id: groupId })}><X aria-hidden="true" /></button>
        </div>}
        {!groupId && !links.length && data.groups.length > 0 && <span className="favorite-root-label">{t.root}</span>}
        {links.map((favorite, index) => {
          const label = (favorite.defaultLocation && t.defaultFavoriteNames[favorite.defaultLocation])
            || favorite.name || t.missing;
          return (
            <div className="favorite-row" key={favorite.id}
              data-insert-before={preview?.groupId === groupId && preview.slot === index}
              data-drag-source={dragged === favorite.id}
              onContextMenu={event => {
              event.preventDefault();
              const menu = document.getElementById(`${menuId}-${favorite.id}`);
              favoriteMenuPoint.current = { x: event.clientX, y: event.clientY };
              menu?.showPopover();
              menu?.querySelector('button')?.focus();
            }}>
              <button type="button" className="home-position organization-label" title={favorite.path ?? t.missing}
                aria-disabled={!favorite.path} onClick={() => { if (favorite.path) open(favorite); }} draggable
                onDragStart={event => { event.dataTransfer.setData(favoriteDrag, favorite.id); event.dataTransfer.effectAllowed = 'move'; setDragged(favorite.id); }}
                onDragEnd={() => { setDragged(null); setPreview(null); }}>
                <Star aria-hidden="true" /><span>{label}</span>
              </button>
              <button type="button" className="tool-button" aria-label={`${label}: ${t.removeFavorite}`}
                onClick={event => {
                  const rect = event.currentTarget.getBoundingClientRect();
                  favoriteMenuPoint.current = { x: rect.left, y: rect.bottom };
                }} popoverTarget={`${menuId}-${favorite.id}`}><Ellipsis aria-hidden="true" /></button>
              <WorkspaceContextMenu id={`${menuId}-${favorite.id}`} popover="auto" aria-label={label}
                onBeforeToggle={event => {
                  if (event.newState !== 'open') return;
                  const menu = event.currentTarget;
                  const row = menu.parentElement!.getBoundingClientRect();
                  const { x, y } = favoriteMenuPoint.current ?? { x: row.left, y: row.bottom };
                  favoriteMenuPoint.current = null;
                  menu.style.left = `${x}px`;
                  menu.style.top = `${y}px`;
                  requestAnimationFrame(() => {
                    if (!menu.matches(':popover-open')) return;
                    menu.style.left = `${Math.max(8, Math.min(x, innerWidth - menu.offsetWidth - 8))}px`;
                    menu.style.top = `${Math.max(8, Math.min(y, innerHeight - menu.offsetHeight - 8))}px`;
                  });
                }}>
                <button type="button" autoFocus onClick={event => {
                  event.currentTarget.parentElement?.hidePopover();
                  void mutate({ kind: 'remove-favorite', id: favorite.id });
                }}><X aria-hidden="true" />{t.removeFavorite}</button>
              </WorkspaceContextMenu>
            </div>
          );
        })}
      </div>
    );
  }

  return <div className="organization-sidebar-content">
    {list(null, t.root)}
    {data.groups.map(group => list(group.id, group.name))}
    {editing ? <form className="organization-form" key={editing.id ?? 'new'} onSubmit={saveGroup}>
      <input name="name" required maxLength={128} aria-label={t.groupName} placeholder={t.groupName}
        autoFocus defaultValue={editing.name} />
      <div className="organization-row"><button>{t.save}</button><button type="button" onClick={() => setEditing(null)}>{t.cancel}</button></div>
    </form> : <button className="home-position" type="button" onClick={() => setEditing({ id: null, name: '' })}><Plus aria-hidden="true" />{t.newGroup}</button>}
  </div>;
}

function TagName({ tag }: { tag: Tag }) {
  return <><TagIcon aria-hidden="true" style={{ color: tag.color ?? undefined }} /><span>{tag.name}</span></>;
}

export function Tags({ data, filter, setFilter, mutate }: {
  data: Organization; filter: string | null; setFilter: (id: string | null) => void; mutate: Mutate;
}) {
  const t = organizationStrings();
  const dialog = useRef<HTMLDialogElement>(null);
  const [editing, setEditing] = useState<Tag | null>(null);
  const [colorEnabled, setColorEnabled] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState(false);

  function edit(tag: Tag | null) {
    setEditing(tag);
    setColorEnabled(Boolean(tag?.color));
    setError(false);
    dialog.current?.showModal();
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const fields = new FormData(event.currentTarget);
    setSaving(true);
    setError(false);
    if (await mutate({
      kind: 'upsert-tag', id: editing?.id ?? null,
      name: String(fields.get('name')).trim(), color: colorEnabled ? String(fields.get('color')) : null,
    })) dialog.current?.close();
    else setError(true);
    setSaving(false);
  }

  return <div className="organization-sidebar-content">
    {data.tags.map(tag => <div className="organization-row" key={tag.id}>
      <button className="home-position organization-label" type="button" aria-pressed={filter === tag.id}
        onClick={() => setFilter(filter === tag.id ? null : tag.id)}><TagName tag={tag} /></button>
      <button className="tool-button organization-edit" type="button" title={t.editTag} aria-label={`${t.editTag} ${tag.name}`}
        onClick={() => edit(tag)}>⋯</button>
    </div>)}
    <button className="home-position" type="button" onClick={() => edit(null)}><Plus aria-hidden="true" />{t.newTag}</button>
    <dialog ref={dialog} className="organization-dialog" aria-label={editing ? t.editTag : t.newTag}>
      <form key={editing?.id ?? 'new'} className="organization-form" onSubmit={submit}>
        <label>{t.tagName}<input name="name" required maxLength={128} defaultValue={editing?.name} autoFocus /></label>
        <label className="organization-row"><input type="checkbox" checked={colorEnabled}
          onChange={event => setColorEnabled(event.target.checked)} />{t.useColor}</label>
        {colorEnabled && <input type="color" name="color" aria-label={t.tagColor} defaultValue={editing?.color ?? '#2864d7'} />}
        <div className="organization-row">
          <button disabled={saving}>{t.save}</button>
          <button type="button" onClick={() => dialog.current?.close()}>{t.cancel}</button>
          {editing && <button type="button" disabled={saving} onClick={async () => {
            if (!window.confirm(t.confirmTag(editing.name))) return;
            setSaving(true);
            setError(false);
            if (await mutate({ kind: 'remove-tag', id: editing.id })) dialog.current?.close();
            else setError(true);
            setSaving(false);
          }}>{t.removeTag}</button>}
        </div>
        {error && <p className="workspace-feedback" role="alert">{t.failure}</p>}
      </form>
    </dialog>
  </div>;
}

function safeLink(url: string) {
  try {
    const parsed = new URL(url);
    return ['http:', 'https:'].includes(parsed.protocol) && !parsed.username && !parsed.password ? parsed.href : undefined;
  } catch {
    return undefined;
  }
}

function NoteEditor({ item, notes }: { item: OrganizedItem; notes: ReturnType<typeof useNoteDrafts> }) {
  const t = organizationStrings();
  const [preview, setPreview] = useState(false);
  const [tooLong, setTooLong] = useState(false);
  const local = notes.drafts[item.id];
  const draft = local?.status === 'saved' && local.revision < (item.note?.revision ?? 0) ? undefined : local;
  const body = draft?.body ?? item.note?.body ?? '';
  const status = tooLong ? t.tooLong : draft?.status === 'conflict' ? t.conflict : draft?.status === 'error' ? t.failure
    : draft?.status === 'saving' ? t.saving : draft?.status === 'pending' ? t.pending : body ? t.saved : '';

  return <section className="note-editor" aria-label={t.note}>
    <div className="organization-row">
      <h3>{t.note}</h3>
      <button type="button" aria-pressed={preview} onClick={() => setPreview(!preview)}>{preview ? t.edit : t.preview}</button>
    </div>
    {preview ? <div className="note-preview">
      <Markdown skipHtml disallowedElements={['img']} urlTransform={safeLink}
        components={{ a: ({ href, children }) => href
          ? <a href={href} target="_blank" rel="noopener noreferrer">{children}</a>
          : <span>{children}</span> }}>{body}</Markdown>
    </div> : <textarea aria-label={t.note} value={body} onChange={event => {
      const value = event.target.value;
      const exceedsLimit = new TextEncoder().encode(value).length > 256 * 1024;
      setTooLong(exceedsLimit);
      if (!exceedsLimit) notes.edit(item, value);
    }} />}
    <span role="status">{status}</span>
    {draft?.status === 'error' && <button type="button" onClick={() => void notes.save(item.id)}>{t.retry}</button>}
    {draft?.status === 'conflict' && <button type="button" onClick={() => void notes.resolve(item)}>{t.replaceNote}</button>}
  </section>;
}

export function Inspector({ data, selected, items, loading, close, mutate, notes }: {
  data: Organization; selected?: SelectionDetails; items: OrganizedItem[]; loading: boolean; close: () => void;
  mutate: Mutate; notes: ReturnType<typeof useNoteDrafts>;
}) {
  const t = organizationStrings();
  const [group, setGroup] = useState('');
  const count = selected?.count ?? 0;
  const single = count === 1 ? items[0] : undefined;
  const favorite = single && data.favorites.find(row => row.itemId === single.id);

  return <aside className="organization-inspector" id="organization-inspector" aria-label={t.inspector}>
    <header className="organization-row"><h2>{t.inspector}</h2>
      <button type="button" className="tool-button" aria-label={t.closeInspector} onClick={close}><X aria-hidden="true" /></button>
    </header>
    <div className="organization-inspector-content">
      {loading ? <p role="status">{t.loading}</p> : <>
        {!count ? <p>{t.nothing}</p> : <>
          <h3 title={single?.path}>{single ? pathLabel(single.path) : t.count(count)}</h3>
          <p>{[selected?.kinds, selected?.size].filter(Boolean).join(' · ')}</p>
          {single && <p className="organization-path">{single.path}</p>}
          {items.length === 0 ? <p>{t.unavailable}</p> : count > items.length && <p>{t.unresolved(items.length)}</p>}
          {single && <div className="organization-row favorite-add">
            {!favorite && data.groups.length > 0 && <select aria-label={t.moveTo} value={group} onChange={event => setGroup(event.target.value)}>
              <option value="">{t.root}</option>
              {data.groups.map(row => <option key={row.id} value={row.id}>{row.name}</option>)}
            </select>}
            <button type="button" onClick={() => void mutate(favorite
              ? { kind: 'remove-favorite', id: favorite.id }
              : { kind: 'add-favorite', itemId: single.id, groupId: group || null })}>
              <Star aria-hidden="true" fill={favorite ? 'currentColor' : 'none'} />
              {favorite ? t.removeFavorite : t.addFavorite}
            </button>
          </div>}
          {data.tags.length > 0 && items.length > 0 && <fieldset>
            <legend>{t.tags}</legend>
            {data.tags.map(tag => {
              const count = items.filter(item => item.tagIds.includes(tag.id)).length;
              return <label className="organization-row" key={tag.id}>
                <input type="checkbox" checked={count === items.length}
                  ref={input => { if (input) input.indeterminate = count > 0 && count < items.length; }}
                  onChange={event => void mutate({ kind: 'assign-tag', itemIds: items.map(item => item.id), tagId: tag.id, assigned: event.target.checked })} />
                <TagName tag={tag} />
              </label>;
            })}
          </fieldset>}
          {single && <NoteEditor key={single.id} item={single} notes={notes} />}
        </>}
      </>}
    </div>
  </aside>;
}
