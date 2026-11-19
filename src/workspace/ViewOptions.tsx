import { ListFilter } from 'lucide-react';
import { modes, sortIds, type ColumnId, type ViewOptions as Options } from './fileViews';
import { locale } from './strings';

const en = {
  view: 'View', details: 'Details', grid: 'Grid', columns: 'Columns',
  name: 'Name', kind: 'Kind', size: 'Size', modified: 'Modified', created: 'Created',
  tags: 'Tags', note: 'Note', indexing: 'Indexing',
  sort: 'Sort by', reverse: 'Descending', foldersFirst: 'Folders first', group: 'Group by kind',
  extensions: 'Show known extensions', hidden: 'Show hidden items', iconSize: 'Icon size',
  default: 'Set as default', reset: 'Use default for this folder', left: 'Move earlier', right: 'Move later',
  width: 'Width', selection: (count: number) => `${count} selected`,
};
const it: typeof en = {
  view: 'Vista', details: 'Dettagli', grid: 'Griglia', columns: 'Colonne',
  name: 'Nome', kind: 'Tipo', size: 'Dimensione', modified: 'Modificato', created: 'Creato',
  tags: 'Tag', note: 'Nota', indexing: 'Indicizzazione',
  sort: 'Ordina per', reverse: 'Decrescente', foldersFirst: 'Prima le cartelle', group: 'Raggruppa per tipo',
  extensions: 'Mostra le estensioni note', hidden: 'Mostra gli elementi nascosti', iconSize: 'Dimensione icone',
  default: 'Imposta come predefinita', reset: 'Usa la vista predefinita per questa cartella',
  left: 'Sposta prima', right: 'Sposta dopo', width: 'Larghezza', selection: count => `${count} selezionati`,
};
export function getViewStrings() {
  return locale === 'it' ? it : en;
}

type Props = {
  value: Options;
  onChange: (value: Options) => void;
  saveDefault: () => void;
  reset: () => void;
  hasFolder: boolean;
};

export function ViewOptions({ value, onChange, saveDefault, reset, hasFolder }: Props) {
  const text = getViewStrings();
  const updateColumn = (id: ColumnId, field: 'visible' | 'width', next: boolean | number) => {
    onChange({ ...value, columns: value.columns.map(column => column.id === id ? { ...column, [field]: next } : column) });
  };
  const moveColumn = (index: number, offset: number) => {
    const columns = [...value.columns];
    columns.splice(index + offset, 0, ...columns.splice(index, 1));
    onChange({ ...value, columns });
  };

  return (
    <details className="view-settings" onKeyDown={event => {
      if (event.key !== 'Escape') return;
      event.currentTarget.open = false;
      event.currentTarget.querySelector('summary')?.focus();
    }}>
      <summary className="tool-button" aria-label={text.view} title={text.view}><ListFilter /></summary>
      <div className="view-settings-menu">
        <label>{text.view}
          <select value={value.mode} onChange={event => onChange({ ...value, mode: event.target.value as Options['mode'] })}>
            {modes.map(mode => <option key={mode} value={mode}>{text[mode]}</option>)}
          </select>
        </label>
        <label>{text.sort}
          <select value={value.sort} onChange={event => onChange({ ...value, sort: event.target.value as Options['sort'] })}>
            {sortIds.map(sort => <option key={sort} value={sort}>{text[sort]}</option>)}
          </select>
        </label>
        {(['descending', 'foldersFirst', 'showExtensions', 'showHidden'] as const).map((key, index) => (
          <label key={key}><input type="checkbox" checked={value[key]}
            onChange={event => onChange({ ...value, [key]: event.target.checked })} />
            {[text.reverse, text.foldersFirst, text.extensions, text.hidden][index]}</label>
        ))}
        <label><input type="checkbox" checked={value.groupBy === 'kind'}
          onChange={event => onChange({ ...value, groupBy: event.target.checked ? 'kind' : 'none' })} />{text.group}</label>
        {value.mode === 'grid' && <label>{text.iconSize}
          <input type="range" min="16" max="160" value={value.iconSize}
            onChange={event => onChange({ ...value, iconSize: event.target.valueAsNumber })}/>
        </label>}
        {value.mode === 'details' && <fieldset>
          <legend>{text.columns}</legend>
          {value.columns.map((column, index) => <div key={column.id} className="column-settings">
            <label><input type="checkbox" checked={column.visible} disabled={column.id === 'name'}
              onChange={event => updateColumn(column.id, 'visible', event.target.checked)} />{text[column.id]}</label>
            <input key={column.width} type="number" aria-label={`${text.width} · ${text[column.id]}`} min={48} max={1200} defaultValue={column.width}
              onBlur={event => {
                if (!event.target.validity.valid || !event.target.value) { event.target.value = String(column.width); return; }
                if (event.target.valueAsNumber !== column.width) updateColumn(column.id, 'width', event.target.valueAsNumber);
              }} />
            <button type="button" aria-label={`${text.left} · ${text[column.id]}`} disabled={index === 0}
              onClick={() => moveColumn(index, -1)}>↑</button>
            <button type="button" aria-label={`${text.right} · ${text[column.id]}`} disabled={index + 1 === value.columns.length}
              onClick={() => moveColumn(index, 1)}>↓</button>
          </div>)}
        </fieldset>}
        {hasFolder && <div className="view-settings-actions">
          <button type="button" onClick={saveDefault}>{text.default}</button>
          <button type="button" onClick={reset}>{text.reset}</button>
        </div>}
      </div>
    </details>
  );
}
