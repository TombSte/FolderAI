(() => {
  'use strict';
  const $ = (selector) => document.querySelector(selector);
  const $$ = (selector) => [...document.querySelectorAll(selector)];
  const paths = {
    layers: '<path d="m12 3 9 5-9 5-9-5 9-5Z"/><path d="m3 12 9 5 9-5M3 16l9 5 9-5"/>',
    folder: '<path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v10H3V7Z"/>',
    'search-folder': '<path d="M10 19H3V7a2 2 0 0 1 2-2h5l2 2h7v3"/><circle cx="16" cy="16" r="4"/><path d="m19 19 3 3"/>',
    document: '<path d="M6 3h8l4 4v14H6V3Z"/><path d="M14 3v5h4M9 12h6M9 15h6"/>',
    home: '<path d="m3 10 9-7 9 7M5 9v12h14V9M9 21v-7h6v7"/>',
    download: '<path d="M12 3v12m-5-5 5 5 5-5M4 16v5h16v-5"/>',
    image: '<rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8" cy="8" r="1.5"/><path d="m3 17 5-5 4 4 4-6 5 7"/>',
    'chevron-right': '<path d="m9 5 7 7-7 7"/>',
    'chevron-down': '<path d="m5 9 7 7 7-7"/>',
    'arrow-left': '<path d="M20 12H4m6-6-6 6 6 6"/>',
    'arrow-right': '<path d="M4 12h16m-6-6 6 6-6 6"/>',
    'arrow-up': '<path d="M12 20V4m-6 6 6-6 6 6"/>',
    'arrow-up-right': '<path d="M6 18 18 6M6 6h12v12"/>',
    plus: '<path d="M12 5v14M5 12h14"/>',
    x: '<path d="m6 6 12 12M6 18 18 6"/>',
    search: '<circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 5 5"/>',
    list: '<path d="M9 6h12M9 12h12M9 18h12M3 6h1M3 12h1M3 18h1"/>',
    grid: '<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>',
    sidebar: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16"/>',
    'panel-right': '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M15 4v16"/>',
    sparkles: '<path d="m12 3 2.5 6.5L21 12l-6.5 2.5L12 21l-2.5-6.5L3 12l6.5-2.5L12 3ZM20 2v4M18 4h4"/>',
    settings: '<path d="m9 3-1 3-3 1-2 5 2 5 3 1 1 3h6l1-3 3-1 2-5-2-5-3-1-1-3H9Z"/><circle cx="12" cy="12" r="3"/>',
    moon: '<path d="M20.5 14A9 9 0 0 1 10 3.5 9 9 0 1 0 20.5 14Z"/>',
    sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5"/>',
    shield: '<path d="m12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6l8-3Z"/><path d="m8 12 3 3 5-6"/>',
    sort: '<path d="M4 6h16M4 12h11M4 18h6"/>',
    external: '<path d="M14 3h7v7m-9 2 9-9M10 3H3v18h18v-7"/>',
    history: '<path d="M3 10a9 9 0 1 1 1 7M3 4v6h6M12 7v6l4 2"/>',
    activity: '<path d="M3 12h4l3-7 4 14 3-7h4"/>',
    copy: '<rect x="8" y="8" width="13" height="13" rx="2"/><path d="M16 8V3H3v13h5"/>',
    trash: '<path d="M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7M14 10v7"/>',
    check: '<path d="m5 12 4 4L19 6"/>'
  };
  const icon = (name) => `<svg class="icon" viewBox="0 0 24 24" aria-hidden="true">${paths[name] || paths.document}</svg>`;
  function hydrate(root = document) {
    root.querySelectorAll('[data-icon]').forEach((el) => { el.innerHTML = icon(el.dataset.icon); });
  }
  hydrate();

  const files = [
    { id: 'archive', name: 'Archives', kind: 'folder', type: 'Dossier', size: '—', date: '12 nov 2026', tags: [], note: '' },
    { id: 'contracts', name: 'Contratti', kind: 'folder', type: 'Dossier', size: '—', date: '14 nov 2026', tags: ['Contratti'], note: '' },
    { id: 'design', name: 'Design', kind: 'folder', type: 'Dossier', size: '—', date: '13 nov 2026', tags: [], note: '' },
    { id: 'agreement', name: 'Accordo integrativo.pdf', kind: 'pdf', type: 'PDF', size: '840 KB', date: '13 nov 2026', tags: ['Contratti', 'Da verificare'], note: 'Confrontare con il contratto quadro.', passage: 'Il rinnovo richiede conferma scritta almeno 30 giorni prima della scadenza.' },
    { id: 'brief', name: 'Brief di progetto.md', kind: 'document', type: 'Markdown', size: '12 KB', date: '11 nov 2026', tags: ['Approvato'], note: 'Obiettivi e ambito del progetto.' },
    { id: 'checklist', name: 'Checklist rinnovi.md', kind: 'document', type: 'Markdown', size: '4 KB', date: '14 nov 2026', tags: ['Da verificare'], note: 'Controllare i termini di rinnovo.' },
    { id: 'contract', name: 'Contratto quadro 2025.pdf', kind: 'pdf', type: 'PDF', size: '2,4 MB', date: '14 nov 2026', tags: ['Contratti', 'Da verificare'], note: 'Verificare la relazione con l’accordo integrativo.', passage: 'Il contratto si rinnova tacitamente per dodici mesi, salvo disdetta scritta con un preavviso di almeno 60 giorni.' },
    { id: 'notes', name: 'Note riunione.md', kind: 'document', type: 'Markdown', size: '8 KB', date: '12 nov 2026', tags: [], note: '' },
    { id: 'plan', name: 'Piano attività.pdf', kind: 'pdf', type: 'PDF', size: '1,1 MB', date: '10 nov 2026', tags: ['Approvato'], note: '' },
    { id: 'preview', name: 'Proposta visuale.png', kind: 'image', type: 'Immagine', size: '3,2 MB', date: '13 nov 2026', tags: [], note: '' },
    { id: 'readme', name: 'README.md', kind: 'document', type: 'Markdown', size: '2 KB', date: '9 nov 2026', tags: [], note: '' },
    { id: 'minutes', name: 'Verbale approvazione.txt', kind: 'document', type: 'Testo', size: '6 KB', date: '14 nov 2026', tags: ['Da verificare'], note: '' }
  ];
  const example = 'Cerca i contratti relativi al progetto Apollo e confronta le condizioni di rinnovo. Escludi le bozze, evidenzia le scadenze di disdetta e cita la clausola da cui ricavi ogni informazione.';
  const welcome = $('#chat-welcome').cloneNode(true);
  const states = Object.fromEntries(['apollo', 'documents'].map((id) => [id, {
    query: '', mode: 'simple', searchOpen: false, view: 'list', selected: 'contract', tag: '',
    messages: [welcome.cloneNode(true)], draft: '', scope: 'folder', scopeFolder: id === 'apollo' ? 'Apollo' : 'Documenti', chatMode: 'agentic'
  }]));
  let currentTab = 'apollo';
  let panel = null;
  let toastTimer;
  const state = () => states[currentTab];
  const tabTitle = () => currentTab === 'apollo' ? 'Apollo' : 'Documenti';
  const tabFiles = () => currentTab === 'apollo' ? files : files.filter((file) => file.kind !== 'folder');
  let visibleFiles = [];

  function toast(text) {
    clearTimeout(toastTimer);
    $('#toast').textContent = text;
    $('#toast').hidden = false;
    toastTimer = setTimeout(() => { $('#toast').hidden = true; }, 5000);
  }
  function renderFiles() {
    const s = state();
    const query = s.query.toLocaleLowerCase('it');
    visibleFiles = tabFiles().filter((file) => (!s.tag || file.tags.includes(s.tag)) &&
      (!query || `${file.name} ${file.note} ${file.tags.join(' ')}`.toLocaleLowerCase('it').includes(query)));
    const list = $('#file-list');
    list.replaceChildren();
    list.classList.toggle('grid-view', s.view === 'grid');
    $('.file-columns').hidden = s.view === 'grid';
    visibleFiles.forEach((file, index) => {
      const row = document.createElement('button');
      row.className = `file-row${s.selected === file.id ? ' selected' : ''}`;
      row.dataset.file = file.id;
      row.setAttribute('role', 'option');
      row.setAttribute('aria-selected', String(s.selected === file.id));
      row.tabIndex = s.selected === file.id || (!visibleFiles.some((f) => f.id === s.selected) && index === 0) ? 0 : -1;
      row.title = file.name;
      const dot = file.kind === 'folder' ? -1 : file.name.lastIndexOf('.');
      const base = dot > 0 ? file.name.slice(0, dot) : file.name;
      const extension = dot > 0 ? file.name.slice(dot) : '';
      row.innerHTML = `<span class="file-name"><span class="file-symbol ${file.kind}">${icon(file.kind === 'folder' ? 'folder' : file.kind === 'image' ? 'image' : 'document')}</span><span class="file-name-text"><span class="file-base">${base}</span><span class="file-extension">${extension}</span></span></span><span class="file-type">${file.type}</span><span class="file-size">${file.size}</span><span class="file-date">${file.date}</span>`;
      list.append(row);
    });
    $('#empty-state').hidden = visibleFiles.length !== 0;
    $('#file-count').textContent = `${visibleFiles.length} elementi`;
    const selected = visibleFiles.find((file) => file.id === s.selected);
    $('#selection-status').textContent = selected ? `1 selezionato${selected.size !== '—' ? ` · ${selected.size}` : ''}` : '';
    $('#folder-title').textContent = s.query || s.tag ? 'Risultati di ricerca' : tabTitle();
    $('#folder-eyebrow').textContent = s.query || s.tag ? `${tabTitle()} e sottocartelle · demo locale` : currentTab === 'apollo' ? 'Progetti' : 'Home';
    $('#breadcrumb-current').textContent = tabTitle();
    const summary = $('#search-summary');
    summary.hidden = !s.query && !s.tag;
    summary.textContent = s.tag ? `Tag: ${s.tag} · filtro sui dati fittizi` : `“${s.query}” · corrispondenze su nomi, tag e note fittizi`;
    $$('[data-view]').forEach((button) => {
      const active = button.dataset.view === s.view;
      button.classList.toggle('active', active);
      button.setAttribute('aria-pressed', String(active));
    });
    renderDetails();
  }
  function selectFile(id, focus = false) {
    state().selected = id;
    renderFiles();
    if (focus) $(`[data-file="${id}"]`)?.focus();
  }
  function renderDetails() {
    const file = files.find((f) => f.id === state().selected) || files[6];
    $('#detail-name').textContent = file.name;
    $('#detail-path').textContent = `${currentTab === 'apollo' ? 'Progetti' : 'Home'} / ${tabTitle()}`;
    $('#detail-type').textContent = file.type;
    $('#detail-size').textContent = file.size;
    $('.file-info dd:last-child').textContent = file.date;
    $('#paper-title').textContent = file.name.replace(/\.[^.]+$/, '');
    $('.paper mark').textContent = file.passage || 'Esempio di anteprima. Il contenuto reale di questo elemento non è disponibile nel mockup.';
    $('.paper-subtitle').textContent = file.kind === 'pdf' ? 'Estratto illustrativo · documento fittizio' : 'Rappresentazione illustrativa, non un visualizzatore reale';
    $('#file-note').value = file.note;
    $('#detail-tags').replaceChildren();
    (file.tags.length ? file.tags : ['Nessun tag']).forEach((tag) => {
      const label = document.createElement('span');
      label.className = 'detail-tag';
      label.textContent = tag;
      $('#detail-tags').append(label);
    });
  }
  function setPanel(next) {
    panel = next;
    $('#right-panel').hidden = !next;
    $('#panel-resizer').hidden = !next;
    $('#details-panel').hidden = next !== 'details';
    $('#assistant-panel').hidden = next !== 'assistant';
    $$('[data-panel]').forEach((button) => {
      button.classList.toggle('active', button.dataset.panel === next);
      button.setAttribute('aria-pressed', String(button.dataset.panel === next));
    });
    for (const action of ['inspector', 'assistant']) {
      const button = $(`[data-action="${action}"]`);
      const active = action === 'inspector' ? next === 'details' : next === 'assistant';
      button.classList.toggle('active', active);
      button.setAttribute('aria-expanded', String(active));
    }
    $$('[data-action^="scene-"]').forEach((button) => {
      const scene = next === 'assistant' ? 'scene-assistant' : next === 'details' ? 'scene-details' : 'scene-explorer';
      button.setAttribute('aria-pressed', String(button.dataset.action === scene));
    });
  }
  function setSearch(open, focus = false) {
    state().searchOpen = open;
    $('#search-fields').hidden = !open;
    $('#search-form').classList.toggle('expanded', open);
    $('[data-action="search"]').setAttribute('aria-expanded', String(open));
    $('[data-action="search"]').setAttribute('aria-label', open ? 'Ricerca aperta' : 'Apri ricerca');
    if (open && focus) $('#search-input').focus();
    if (!open) {
      state().query = '';
      state().tag = '';
      $('#search-input').value = '';
      renderFiles();
    }
  }
  function syncScope() {
    const global = $('#chat-scope').value === 'global';
    $('#scope-folder').hidden = global;
    $('#scope-folder').innerHTML = `${state().scopeFolder}${icon('chevron-down')}`;
    state().scope = $('#chat-scope').value;
  }
  function storeConversation() {
    Object.assign(state(), {
      messages: [...$('#conversation').children], draft: $('#chat-input').value,
      scope: $('#chat-scope').value, chatMode: $('#chat-mode').value
    });
  }
  function switchTab(id) {
    if (id === currentTab) return;
    storeConversation();
    currentTab = id;
    const s = state();
    $('#conversation').replaceChildren(...s.messages);
    $('#chat-input').value = s.draft;
    $('#chat-scope').value = s.scope;
    $('#chat-mode').value = s.chatMode;
    $('#search-input').value = s.query;
    $('#search-mode').value = s.mode;
    syncScope();
    setSearch(s.searchOpen);
    $$('[data-tab]').forEach((button) => {
      const active = button.dataset.tab === id;
      button.classList.toggle('active', active);
      if (active) button.setAttribute('aria-current', button.classList.contains('workspace-tab') ? 'page' : 'location');
      else button.removeAttribute('aria-current');
    });
    renderFiles();
  }
  function newChat() {
    $('#conversation').replaceChildren(welcome.cloneNode(true));
    $('#chat-input').value = '';
    $('.back-to-chat').hidden = true;
    setPanel('assistant');
  }
  function sendMessage() {
    const input = $('#chat-input');
    const text = input.value.trim();
    if (!text) { input.focus(); return; }
    $('#chat-welcome')?.remove();
    const user = document.createElement('article');
    user.className = 'message user';
    const label = document.createElement('div');
    label.className = 'message-label';
    label.textContent = 'Tu';
    const content = document.createElement('p');
    content.textContent = text;
    const scope = document.createElement('small');
    scope.className = 'message-scope';
    scope.textContent = `${$('#chat-mode').value === 'auto' ? 'Auto' : 'Agentica'} · ${state().scope === 'global' ? 'Globale · cartelle abilitate' : `${state().scopeFolder} e sottocartelle`}`;
    user.append(label, content, scope);
    const answer = document.createElement('article');
    answer.className = 'message answer';
    if (text === example && state().scopeFolder === 'Apollo') {
      answer.innerHTML = `<div class="research-state">${icon('check')}Esempio completato · 2 fonti fittizie</div><div class="message-label">${icon('sparkles')}FolderAI · risposta dimostrativa</div><h3 class="answer-title">Due condizioni di rinnovo diverse.</h3><p>Il <strong>contratto quadro</strong> prevede il rinnovo tacito, con disdetta almeno <strong>60 giorni prima</strong> della scadenza. <button class="citation" data-source="contract" aria-label="Apri fonte 1">1</button></p><p>L’<strong>accordo integrativo</strong> richiede invece una conferma scritta almeno <strong>30 giorni prima</strong>. <button class="citation" data-source="agreement" aria-label="Apri fonte 2">2</button></p><p class="answer-caution">Da verificare: gli estratti non stabiliscono quale clausola prevalga. Non è stata determinata una scadenza di calendario.</p><div class="answer-sources">${icon('document')}2 fonti · clic sui riferimenti per l’anteprima</div>`;
    } else {
      const label = document.createElement('div');
      label.className = 'message-label';
      label.textContent = 'FolderAI · demo senza modello';
      const message = document.createElement('p');
      message.textContent = 'Richiesta acquisita solo per mostrare il flusso conversazionale. Nessuna ricerca reale è stata eseguita. Usa l’esempio Apollo per vedere una risposta illustrativa con fonti.';
      answer.append(label, message);
    }
    $('#conversation').append(user, answer);
    input.value = '';
    $('#conversation').scrollTop = $('#conversation').scrollHeight;
    storeConversation();
  }
  function exampleScene() {
    switchTab('apollo');
    newChat();
    state().scopeFolder = 'Apollo';
    $('#chat-scope').value = 'folder';
    syncScope();
    $('#chat-input').value = example;
    sendMessage();
  }
  function setTheme(dark) {
    document.documentElement.dataset.theme = dark ? 'dark' : 'light';
    $('#setting-dark').checked = dark;
    const button = $('[data-action="theme"]');
    button.setAttribute('aria-label', dark ? 'Passa al tema chiaro' : 'Passa al tema scuro');
    button.querySelector('[data-icon]').innerHTML = icon(dark ? 'sun' : 'moon');
  }

  const actions = {
    'scene-explorer': () => { setPanel(null); setSearch(false); },
    'scene-assistant': exampleScene,
    'scene-details': () => { selectFile('contract'); setPanel('details'); },
    theme: () => setTheme(document.documentElement.dataset.theme !== 'dark'),
    sidebar: () => {
      const hidden = !$('#sidebar').hidden;
      $('#sidebar').hidden = hidden;
      $('[data-action="sidebar"]').setAttribute('aria-expanded', String(!hidden));
      $('[data-action="sidebar"]').setAttribute('aria-label', hidden ? 'Mostra sidebar' : 'Nascondi sidebar');
    },
    tree: () => {
      const expanded = $('#apollo-tree').hidden;
      $('#apollo-tree').hidden = !expanded;
      $('[data-action="tree"]').setAttribute('aria-expanded', String(expanded));
      $('[data-action="tree"]').setAttribute('aria-label', `${expanded ? 'Comprimi' : 'Espandi'} sottocartelle di Apollo`);
    },
    search: () => setSearch(true, true),
    'close-search': () => { setSearch(false); $('[data-action="search"]').focus(); },
    inspector: () => setPanel(panel === 'details' ? null : 'details'),
    assistant: () => { setPanel(panel === 'assistant' ? null : 'assistant'); if (panel) $('#chat-input').focus(); },
    'close-panel': () => { setPanel(null); $('[data-action="inspector"]').focus(); },
    'back-to-chat': () => setPanel('assistant'),
    'new-chat': newChat,
    example: () => { $('#chat-input').value = example; $('#chat-input').focus(); },
    'saved-search': () => { setPanel('assistant'); $('#chat-input').value = example; toast('Esempio caricato: premi Invia per visualizzare il confronto fittizio.'); },
    settings: () => $('#settings-dialog').showModal(),
    'close-settings': () => $('#settings-dialog').close(),
    'scope-folder': () => $('#scope-dialog').showModal(),
    'close-scope': () => $('#scope-dialog').close(),
    history: () => toast('Demo: le due schede conservano conversazioni separate nella memoria della pagina. Nessuna cronologia persistente.'),
    activities: () => toast('Nessuna attività reale in corso. Ricerca e risposte sono dimostrative.'),
    'demo-location': () => toast('Collegamento illustrativo. Apollo e Documenti sono le due posizioni navigabili della demo.'),
    'new-tab': () => toast('Nell’app una nuova scheda aprirà la cartella iniziale. Qui puoi provare Apollo e Documenti.'),
    'nav-demo': () => toast('Navigazione illustrativa: nessun accesso al filesystem. Puoi cambiare scheda dalla barra in alto.'),
    'file-demo': () => toast('Comando illustrativo: nessun file viene copiato, modificato o eliminato.'),
    'open-demo': () => toast('Apertura illustrativa: nessuna applicazione o file reale viene aperto.'),
    'sort-demo': () => toast('Ordine dimostrativo: cartelle prima, poi nome crescente.'),
    'window-demo': () => toast('Controlli finestra illustrativi: non agiscono sulla finestra del browser.')
  };
  document.addEventListener('click', (event) => {
    const button = event.target.closest('button');
    if (!button || button.disabled) return;
    if (button.dataset.action) actions[button.dataset.action]?.();
    if (button.dataset.tab) switchTab(button.dataset.tab);
    if (button.dataset.view) { state().view = button.dataset.view; renderFiles(); }
    if (button.dataset.file) selectFile(button.dataset.file, true);
    if (button.dataset.panel) setPanel(button.dataset.panel);
    if (button.dataset.tag) { state().tag = button.dataset.tag; state().query = ''; $('#search-input').value = ''; setSearch(true); renderFiles(); }
    if (button.dataset.source) {
      selectFile(button.dataset.source);
      $('.back-to-chat').hidden = false;
      setPanel('details');
      $('.back-to-chat').focus();
    }
    if (button.dataset.scopeFolder) {
      state().scopeFolder = button.dataset.scopeFolder;
      syncScope();
      $('#scope-dialog').close();
    }
  });
  $('#file-list').addEventListener('dblclick', (event) => {
    if (event.target.closest('[data-file]')) toast('Apertura illustrativa: usa il pannello Dettagli per vedere l’anteprima fittizia.');
  });
  $('#file-list').addEventListener('keydown', (event) => {
    const row = event.target.closest('[data-file]');
    if (!row) return;
    const index = visibleFiles.findIndex((file) => file.id === row.dataset.file);
    let next;
    if (event.key === 'ArrowDown' || event.key === 'ArrowRight') next = Math.min(index + 1, visibleFiles.length - 1);
    if (event.key === 'ArrowUp' || event.key === 'ArrowLeft') next = Math.max(index - 1, 0);
    if (event.key === 'Home') next = 0;
    if (event.key === 'End') next = visibleFiles.length - 1;
    if (next !== undefined) { event.preventDefault(); selectFile(visibleFiles[next].id, true); }
  });
  $('#search-input').addEventListener('input', () => {
    if (state().mode === 'simple') { state().query = $('#search-input').value; state().tag = ''; renderFiles(); }
  });
  $('#search-mode').addEventListener('change', () => {
    state().mode = $('#search-mode').value;
    if (['agentic', 'auto'].includes(state().mode)) {
      $('#chat-mode').value = state().mode;
      if ($('#search-input').value.trim()) $('#chat-input').value = $('#search-input').value;
      setPanel('assistant');
      $('#chat-input').focus();
    }
  });
  $('#search-form').addEventListener('submit', (event) => {
    event.preventDefault();
    if (['agentic', 'auto'].includes(state().mode)) {
      setPanel('assistant');
      $('#chat-input').value = $('#search-input').value;
      $('#chat-input').focus();
    } else {
      state().query = $('#search-input').value;
      state().tag = '';
      renderFiles();
      if (state().mode === 'semantic') toast('La demo applica solo un filtro testuale. Nessun indice semantico o modello è collegato.');
    }
  });
  $('#chat-form').addEventListener('submit', (event) => { event.preventDefault(); sendMessage(); });
  $('#chat-input').addEventListener('keydown', (event) => {
    if (event.key === 'Enter' && (event.metaKey || event.ctrlKey) && !event.isComposing) { event.preventDefault(); sendMessage(); }
  });
  $('#chat-scope').addEventListener('change', syncScope);
  $('#setting-dark').addEventListener('change', (event) => setTheme(event.target.checked));
  $('#setting-compact').addEventListener('change', (event) => $('#app-window').classList.toggle('compact', event.target.checked));
  $('#setting-commands').addEventListener('change', (event) => { $('#file-commands').hidden = !event.target.checked; });
  $('#file-note').addEventListener('input', (event) => {
    const file = files.find((f) => f.id === state().selected);
    if (file) file.note = event.target.value;
  });
  document.addEventListener('keydown', (event) => {
    if ($('#settings-dialog').open || $('#scope-dialog').open) return;
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') { event.preventDefault(); setSearch(true, true); }
    if (event.key === 'Escape' && !event.isComposing) {
      if (state().searchOpen) { setSearch(false); $('[data-action="search"]').focus(); }
      else if (panel) { setPanel(null); $('[data-action="inspector"]').focus(); }
    }
  });
  function resizePanel(width) {
    const max = Math.min(560, Math.max(340, $('.workspace').clientWidth - 280));
    const next = Math.round(Math.min(max, Math.max(340, width)));
    document.documentElement.style.setProperty('--panel-width', `${next}px`);
    $('#panel-resizer').setAttribute('aria-valuenow', next);
  }
  $('#panel-resizer').addEventListener('pointerdown', (event) => {
    if (event.button !== 0) return;
    const handle = event.currentTarget;
    handle.setPointerCapture(event.pointerId);
    const startX = event.clientX;
    const startWidth = $('#right-panel').getBoundingClientRect().width;
    const move = (e) => resizePanel(startWidth + startX - e.clientX);
    const stop = () => { handle.removeEventListener('pointermove', move); handle.removeEventListener('lostpointercapture', stop); };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('lostpointercapture', stop);
    event.preventDefault();
  });
  $('#panel-resizer').addEventListener('keydown', (event) => {
    if (!['ArrowLeft', 'ArrowRight'].includes(event.key)) return;
    event.preventDefault();
    resizePanel($('#right-panel').getBoundingClientRect().width + (event.key === 'ArrowLeft' ? 20 : -20));
  });
  setTheme(window.matchMedia('(prefers-color-scheme: dark)').matches);
  syncScope();
  renderFiles();
})();
