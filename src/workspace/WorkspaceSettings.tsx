import { useState, type RefObject } from 'react';
import { defaultControls, isReservedShortcut, shortcutFromEvent, shortcutIds, shortcutLabel, toolbarIds, type Controls, type SidebarId, type ToolbarId } from './controls';
import { defaultStartup, type Startup } from './session';
import { locale, strings } from './strings';
import { organizationStrings } from './organization';

const labels = {
  en: {
    general: 'General', appearance: 'Appearance', sidebar: 'Toolbar and sidebar', keys: 'Shortcuts',
    navigation: 'Navigation and views', language: 'Language', theme: 'Theme', density: 'Density',
    system: 'System', light: 'Light', dark: 'Dark', normal: 'Normal', compact: 'Compact',
    reset: 'Reset section', width: 'Sidebar width', earlier: 'Move earlier', later: 'Move later',
    conflict: 'This shortcut is already assigned.', capture: 'Press a keyboard shortcut',
    favorites: 'Favorites', tags: 'Tags', searches: 'Saved searches', toolbar: 'Toolbar actions', close: 'Close',
  },
  it: {
    general: 'Generali', appearance: 'Aspetto', sidebar: 'Toolbar e sidebar', keys: 'Scorciatoie',
    navigation: 'Navigazione e viste', language: 'Lingua', theme: 'Tema', density: 'Densità',
    system: 'Sistema', light: 'Chiaro', dark: 'Scuro', normal: 'Normale', compact: 'Compatta',
    reset: 'Ripristina sezione', width: 'Larghezza sidebar', earlier: 'Sposta prima', later: 'Sposta dopo',
    conflict: 'Questa scorciatoia è già assegnata.', capture: 'Premi una scorciatoia',
    favorites: 'Preferiti', tags: 'Tag', searches: 'Ricerche salvate', toolbar: 'Azioni della toolbar', close: 'Chiudi',
  },
};

export function sectionLabel(section: SidebarId) {
  switch (section) {
    case 'locations': return strings.positions;
    case 'favorites': return labels[locale].favorites;
    case 'tags': return labels[locale].tags;
    case 'saved-searches': return labels[locale].searches;
  }
}

export function toolbarLabel(id: ToolbarId) {
  switch (id) {
    case 'new-tab': return strings.newTab;
    case 'new-window': return strings.newWindow;
    case 'close-tab': return strings.closeTab;
    case 'split': return strings.split;
    case 'sidebar': return strings.sidebar;
  }
}

type Props = {
  dialog: RefObject<HTMLDialogElement | null>;
  startup: Startup;
  saveStartup: (startup: Startup) => Promise<void>;
  controls: Controls;
  apply: (controls: Controls) => void;
  error: boolean;
  copyMetadata?: boolean;
  setCopyMetadata: (enabled: boolean) => void;
};

const categories = ['general', 'appearance', 'navigation', 'sidebar', 'keys'] as const;

export function WorkspaceSettings({ dialog, startup, saveStartup, controls, apply, error, copyMetadata, setCopyMetadata }: Props) {
  const [section, setSection] = useState<typeof categories[number]>('general');
  const [draft, setDraft] = useState<Startup | null>(null);
  const [conflict, setConflict] = useState(false);
  const text = labels[locale];
  const starting = draft ?? startup;
  const change = (next: Partial<Controls>) => apply({ ...controls, ...next });
  const keyLabels = {
    newTab: strings.newTab, newWindow: strings.newWindow, closeTab: strings.closeTab,
    address: strings.editPath, hidden: locale === 'it' ? 'Mostra elementi nascosti' : 'Show hidden items',
  };

  function reset() {
    setConflict(false);
    switch (section) {
      case 'general': change({ language: defaultControls.language }); break;
      case 'appearance': change({ theme: defaultControls.theme, density: defaultControls.density }); break;
      case 'navigation': setDraft(defaultStartup); void saveStartup(defaultStartup); break;
      case 'sidebar': change({ sidebar: defaultControls.sidebar, sidebarWidth: defaultControls.sidebarWidth, toolbar: [] }); break;
      case 'keys': change({ shortcuts: defaultControls.shortcuts }); break;
    }
  }

  function moveSidebar(index: number, direction: number) {
    const sidebar = [...controls.sidebar];
    sidebar.splice(index + direction, 0, ...sidebar.splice(index, 1));
    change({ sidebar });
  }

  return (
    <dialog ref={dialog} className="startup-settings workspace-settings-dialog" aria-labelledby="startup-title"
      onClose={() => { setDraft(null); setConflict(false); }}>
      <h2 id="startup-title">{strings.settings}</h2>
      <div className="settings-layout">
        <nav aria-label={strings.settings}>
          {categories.map(category => <button type="button" key={category}
            aria-current={section === category ? 'page' : undefined}
            onClick={() => { setConflict(false); setSection(category); }}>
            {text[category]}
          </button>)}
        </nav>
        <div className="settings-section" key={section}>
          <h3>{text[section]}</h3>
          {section === 'general' && copyMetadata !== undefined &&
            <label>{organizationStrings().copyMetadata}<input type="checkbox" checked={copyMetadata}
              onChange={event => setCopyMetadata(event.target.checked)} /></label>}
          {section === 'general' && <label>{text.language}
            <select value={controls.language} onChange={event => change({ language: event.target.value as Controls['language'] })}>
              <option value="system">{text.system}</option><option value="it">Italiano</option><option value="en">English</option>
            </select>
          </label>}
          {section === 'appearance' && <>
            <label>{text.theme}<select value={controls.theme} onChange={event => change({ theme: event.target.value as Controls['theme'] })}>
              {(['system', 'light', 'dark'] as const).map(value => <option key={value} value={value}>{text[value]}</option>)}
            </select></label>
            <label>{text.density}<select value={controls.density} onChange={event => change({ density: event.target.value as Controls['density'] })}>
              <option value="normal">{text.normal}</option><option value="compact">{text.compact}</option>
            </select></label>
          </>}
          {section === 'navigation' && <form onSubmit={event => {
            event.preventDefault();
            void saveStartup(starting);
          }}>
            {(['home', 'custom', 'restore'] as const).map((mode, index) => (
              <label key={mode}><input type="radio" name="startup" checked={starting.mode === mode}
                onChange={() => setDraft({ ...starting, mode })} />
                {[strings.startAtHome, strings.startAtPath, strings.restoreSession][index]}</label>
            ))}
            {starting.mode !== 'home' && <label>{strings.path}<input value={starting.path ?? ''} required={starting.mode === 'custom'}
              onChange={event => setDraft({ ...starting, path: event.target.value || null })} /></label>}
            <button type="submit">{strings.save}</button>
          </form>}
          {section === 'sidebar' && <>
            <label>{text.width}<input type="range" min="120" max="480" value={controls.sidebarWidth}
              onChange={event => change({ sidebarWidth: event.target.valueAsNumber })}/></label>
            {controls.sidebar.map((item, index) => <div className="column-settings" key={item.id}>
              <label><input type="checkbox" checked={item.visible} onChange={event => change({
                sidebar: controls.sidebar.map(row => row.id === item.id ? { ...row, visible: event.target.checked } : row),
              })} />{sectionLabel(item.id)}</label>
              <button type="button" disabled={index === 0} aria-label={text.earlier + ' · ' + sectionLabel(item.id)}
                onClick={() => moveSidebar(index, -1)}>↑</button>
              <button type="button" disabled={index === controls.sidebar.length - 1} aria-label={text.later + ' · ' + sectionLabel(item.id)}
                onClick={() => moveSidebar(index, 1)}>↓</button>
            </div>)}
            <fieldset><legend>{text.toolbar}</legend>
              {toolbarIds.map(id => <label key={id}><input type="checkbox" checked={controls.toolbar.includes(id)}
                onChange={event => change({ toolbar: event.target.checked ? [...controls.toolbar, id] : controls.toolbar.filter(item => item !== id) })}/>
                {toolbarLabel(id)}</label>)}
            </fieldset>
          </>}
          {section === 'keys' && shortcutIds.map(id => <label key={id}>{keyLabels[id]}
            <input type="text" value={shortcutLabel(controls.shortcuts[id])} readOnly placeholder={text.capture}
              onKeyDown={event => {
                const shortcut = shortcutFromEvent(event);
                if (!shortcut) return;
                event.preventDefault();
                event.stopPropagation();
                const occupied = Object.entries(controls.shortcuts).some(([key, value]) => key !== id && value === shortcut);
                const reserved = isReservedShortcut(shortcut);
                setConflict(occupied || reserved);
                if (!occupied && !reserved) change({ shortcuts: { ...controls.shortcuts, [id]: shortcut } });
              }}/>
          </label>)}
          {conflict && <p role="alert">{text.conflict}</p>}
          {error && <p role="alert">{strings.workspaceError}</p>}
        </div>
      </div>
      <div className="settings-actions">
        <button type="button" onClick={reset}>{text.reset}</button>
        <button type="button" onClick={() => dialog.current?.close()}>{text.close}</button>
      </div>
    </dialog>
  );
}
