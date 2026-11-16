/* Explorer-first macOS/Windows mockups. No filesystem, network or model access. */
(() => {
  "use strict";
  const paths = {
    layers: '<path d="m12 3 9 5-9 5-9-5 9-5Z"/><path d="m3 12 9 5 9-5M3 16l9 5 9-5"/>',
    folder: '<path d="M3 6a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V6Z"/>',
    "folder-open": '<path d="M3 12V6a2 2 0 0 1 2-2h5l2 3h7v4M3 12h19l-3 8H3V12Z"/>',
    home: '<path d="m3 10 9-7 9 7v10a1 1 0 0 1-1 1h-5v-7H9v7H4a1 1 0 0 1-1-1V10Z"/>',
    clock: '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
    document: '<path d="M14 2H5v20h14V7l-5-5Z"/><path d="M14 2v6h5M8 12h8M8 16h6"/>',
    download: '<path d="M12 3v12m-5-5 5 5 5-5M4 16v5h16v-5"/>',
    drive: '<path d="m5 4-3 12h20L19 4H5Z"/><rect x="2" y="16" width="20" height="5" rx="1"/><path d="M17 18.5h2"/>',
    settings: '<path d="m9 3-.6 2.2-2 .9L4.3 5.5 2 9.5 3.6 11v2L2 14.5l2.3 4 2.1-.6 2 .9L9 21h6l.6-2.2 2-.9 2.1.6 2.3-4-1.6-1.5v-2L22 9.5l-2.3-4-2.1.6-2-.9L15 3H9Z"/><circle cx="12" cy="12" r="3"/>',
    "search-folder": '<path d="M12 20H4a1 1 0 0 1-1-1V6a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v2"/><circle cx="17" cy="16" r="4"/><path d="m20 19 3 3"/>',
    sidebar: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16M5.5 8H7M5.5 11H7"/>',
    "panel-right": '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M15 4v16M17 8h1M17 11h1"/>',
    moon: '<path d="M20.5 14A8.5 8.5 0 0 1 10 3.5 9 9 0 1 0 20.5 14Z"/>',
    sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M2 12h2M20 12h2M5 5l1.5 1.5M17.5 17.5 19 19M5 19l1.5-1.5M17.5 6.5 19 5"/>',
    activity: '<path d="M3 12h4l3-8 4 16 3-8h4"/>',
    "arrow-left": '<path d="m12 5-7 7 7 7M5 12h15"/>',
    "arrow-right": '<path d="m12 5 7 7-7 7M4 12h15"/>',
    "arrow-up": '<path d="m5 12 7-7 7 7M12 5v15"/>',
    "chevron-right": '<path d="m9 5 7 7-7 7"/>',
    "chevron-down": '<path d="m5 9 7 7 7-7"/>',
    chevrons: '<path d="m9 8 3-3 3 3m-6 8 3 3 3-3"/>',
    search: '<circle cx="10.5" cy="10.5" r="7"/><path d="m16 16 5 5"/>',
    "x-circle": '<circle cx="12" cy="12" r="8"/><path d="m9 9 6 6m-6 0 6-6"/>',
    wand: '<path d="m4 20 12-12 4 4L8 24M14 10l4 4M6 2v4M4 4h4M19 2v4M17 4h4" transform="translate(0 -2)"/>',
    orbit: '<circle cx="12" cy="12" r="3"/><ellipse cx="12" cy="12" rx="10" ry="5" transform="rotate(-35 12 12)"/>',
    sparkles: '<path d="m12 3 2.5 6.5L21 12l-6.5 2.5L12 21l-2.5-6.5L3 12l6.5-2.5L12 3ZM20 2v4m-2-2h4"/>',
    sliders: '<path d="M4 6h6m4 0h6M4 12h12M4 18h2m4 0h10"/><circle cx="12" cy="6" r="2"/><circle cx="18" cy="12" r="2"/><circle cx="8" cy="18" r="2"/>',
    shield: '<path d="m12 3 8 3v6c0 5-8 9-8 9S4 17 4 12V6l8-3Z"/><path d="m8 12 3 3 5-6"/>',
    lock: '<rect x="5" y="10" width="14" height="11" rx="2"/><path d="M8 10V7a4 4 0 0 1 8 0v3M12 14v3"/>',
    check: '<path d="m5 12 4 4L19 6"/>',
    "check-circle": '<circle cx="12" cy="12" r="9"/><path d="m8 12 3 3 5-6"/>',
    plus: '<path d="M12 4v16M4 12h16"/>',
    list: '<path d="M8 5h13M8 12h13M8 19h13M3 5h.01M3 12h.01M3 19h.01"/>',
    grid: '<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>',
    bookmark: '<path d="M5 3h14v18l-7-5-7 5V3Z"/>',
    more: '<circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>',
    alert: '<path d="m12 3 10 18H2L12 3Z"/><path d="M12 9v5M12 17h.01"/>',
    info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v6M12 7h.01"/>',
    external: '<path d="M14 3h7v7M21 3 10 14M10 3H4a1 1 0 0 0-1 1v16a1 1 0 0 0 1 1h16a1 1 0 0 0 1-1v-6"/>',
    x: '<path d="m6 6 12 12M6 18 18 6"/>'
  };
  const icon = name => `<svg class="icon" viewBox="0 0 24 24" aria-hidden="true">${paths[name] || paths.document}</svg>`;
  const $ = selector => document.querySelector(selector);
  const $$ = selector => [...document.querySelectorAll(selector)];
  function icons(root = document) {
    root.querySelectorAll("[data-icon]").forEach(element => { element.innerHTML = icon(element.dataset.icon); });
  }
  icons();

  const entries = [
    { id: "assets", name: "Assets", kind: "folder", path: "Apollo", size: "—", date: "Oggi, 10:24", tags: ["Apollo"], note: "Materiali del progetto." },
    { id: "contracts", name: "Contratti", kind: "folder", path: "Apollo", size: "—", date: "Ieri, 16:20", tags: ["Apollo", "Contratti"], note: "Accordi e condizioni contrattuali." },
    { id: "documents", name: "Documentazione", kind: "folder", path: "Apollo", size: "—", date: "Oggi, 09:12", tags: ["Apollo"], note: "Documentazione interna." },
    { id: "agreement", name: "Accordo integrativo.pdf", kind: "pdf", path: "Apollo / Contratti", size: "480 KB", date: "12 nov, 14:20", tags: ["Apollo", "Contratti"], note: "Da leggere insieme al contratto quadro.", source: 2, page: "Articolo 4 · Pagina 3", title: "Accordo integrativo", passage: "Il rinnovo è subordinato alla conferma scritta delle parti, da comunicare almeno 30 giorni prima della scadenza.", emphasis: "conferma scritta" },
    { id: "brief", name: "Brief di progetto.pdf", kind: "pdf", path: "Apollo", size: "842 KB", date: "Oggi, 09:12", tags: ["Apollo"], note: "Obiettivi condivisi del progetto.", title: "Il punto di partenza.", passage: "Obiettivo: definire il perimetro del progetto e le milestone condivise.", page: "Documento illustrativo" },
    { id: "checklist", name: "Checklist rinnovo.md", kind: "md", path: "Apollo / Documentazione", size: "8 KB", date: "Ieri, 17:15", tags: ["Apollo", "Da verificare"], note: "Checklist interna: non è un allegato contrattuale.", source: 3, page: "Righe 14–18", title: "Prima del rinnovo", passage: "Verificare l’approvazione del budget e ottenere una conferma scritta. Checklist interna, non allegata agli accordi.", emphasis: "conferma scritta" },
    { id: "contract", name: "Contratto quadro 2025.pdf", kind: "pdf", path: "Apollo / Contratti", size: "1,2 MB", date: "Ieri, 14:22", tags: ["Apollo", "Contratti"], note: "Verificare la relazione con l’accordo integrativo.", source: 1, page: "Articolo 12 · Pagina 8", title: "Contratto quadro", passage: "Il contratto si rinnova tacitamente per dodici mesi, salvo disdetta scritta con un preavviso di almeno 60 giorni.", emphasis: "60 giorni" },
    { id: "moodboard", name: "Moodboard.png", kind: "png", path: "Apollo / Assets", size: "4,8 MB", date: "12 nov, 11:06", tags: ["Apollo"], note: "Riferimenti visuali. Nessuna immagine reale caricata." },
    { id: "minutes", name: "Verbale 24 ottobre.txt", kind: "txt", path: "Apollo / Riunioni", size: "4 KB", date: "24 ott, 16:30", tags: ["Apollo", "Da verificare"], note: "Il team ha richiesto una verifica delle condizioni.", source: 4, page: "Righe 22–25", title: "Decisioni aperte", passage: "Aperto: chiarire quale procedura di rinnovo si applica. Il team richiede una verifica dell’accordo integrativo prima di procedere.", emphasis: "chiarire" }
  ];
  const byId = new Map(entries.map(entry => [entry.id, entry]));
  const modes = {
    auto: { name: "Auto", icon: "wand" },
    simple: { name: "Semplice", icon: "search" },
    semantic: { name: "Semantica", icon: "orbit" },
    agentic: { name: "Agentica", icon: "sparkles" }
  };
  const state = {
    mode: "simple", selected: null, panel: "preview", busy: false,
    searchOpen: false, inspectorOpen: false, agentRequested: false,
    agentPanelVisible: false, revision: 0, timer: null
  };
  const typeNames = { folder: "Cartella", pdf: "Documento PDF", md: "Markdown", txt: "File di testo", png: "Immagine PNG" };
  const saved = new Map();
  let visibleEntries = [];
  let toastTimer;
  const toast = document.createElement("div");
  toast.className = "unified-toast";
  toast.setAttribute("role", "status");
  document.body.append(toast);
  function notify(message) {
    clearTimeout(toastTimer);
    toast.textContent = message;
    toast.classList.add("visible");
    toastTimer = setTimeout(() => toast.classList.remove("visible"), 4200);
  }

  function setTheme(theme) {
    document.documentElement.dataset.theme = theme;
    $('[data-action="theme"]').innerHTML = icon(theme === "dark" ? "sun" : "moon");
    $('[data-action="theme"]').setAttribute("aria-label", theme === "dark" ? "Passa al tema chiaro" : "Passa al tema scuro");
    try { localStorage.setItem("folderai-integrated-theme", theme); } catch { /* Storage may be unavailable for local files. */ }
  }
  let theme = "light";
  try { theme = localStorage.getItem("folderai-integrated-theme") || "light"; } catch { /* Use the light concept by default. */ }
  setTheme(theme === "dark" ? "dark" : "light");

  const dialog = document.createElement("dialog");
  dialog.setAttribute("aria-labelledby", "unified-dialog-title");
  document.body.append(dialog);
  function showDialog(title, description, content = "") {
    closePopovers();
    dialog.innerHTML = '<div class="dialog-header"><h2 id="unified-dialog-title"></h2><button class="icon-button" data-action="close-dialog" aria-label="Chiudi">' + icon("x") + '</button></div><p id="unified-dialog-description"></p><div id="unified-dialog-content"></div>';
    $("#unified-dialog-title").textContent = title;
    $("#unified-dialog-description").textContent = description;
    $("#unified-dialog-content").innerHTML = content;
    dialog.setAttribute("aria-describedby", "unified-dialog-description");
    if (!dialog.open) dialog.showModal();
  }
  dialog.addEventListener("click", event => { if (event.target === dialog) dialog.close(); });

  function closePopovers(focusMode = false) {
    $("#search-mode-menu").hidden = true;
    $("#search-filter-menu").hidden = true;
    $("#mode-trigger").setAttribute("aria-expanded", "false");
    $("#filter-trigger").setAttribute("aria-expanded", "false");
    if (focusMode) $("#mode-trigger").focus();
  }
  function openModeMenu() {
    closePopovers();
    $("#search-mode-menu").hidden = false;
    $("#mode-trigger").setAttribute("aria-expanded", "true");
    $(`[data-search-mode="${state.mode}"]`).focus();
  }
  function syncSearchVisibility() {
    $("#unified-search").classList.toggle("is-expanded", state.searchOpen);
    $(".workspace-toolbar").classList.toggle("search-expanded", state.searchOpen);
    $("#search-fields").hidden = !state.searchOpen;
    $("#search-toggle").setAttribute("aria-expanded", String(state.searchOpen));
    $("#search-toggle").setAttribute("aria-label", state.searchOpen ? "Chiudi ricerca e torna ai file" : "Apri ricerca");
  }
  function openSearch(selectAll = false) {
    state.searchOpen = true;
    syncSearchVisibility();
    $("#unified-query").focus();
    if (selectAll) $("#unified-query").select();
  }
  function resetSearchActivity() {
    state.revision += 1;
    clearTimeout(state.timer);
    state.timer = null;
    state.busy = false;
    state.agentRequested = false;
    state.agentPanelVisible = false;
    $("#unified-search").setAttribute("aria-busy", "false");
    $(".submit-search").disabled = false;
    $("#status-text").textContent = "Demo locale · nessun dato inviato";
  }
  function hasAgentAnswer() {
    return state.agentRequested && Boolean($("#unified-query").value.trim()) && actualMode() === "agentic";
  }
  function panelIsVisible() {
    return state.inspectorOpen || (hasAgentAnswer() && state.agentPanelVisible);
  }
  function actualMode() {
    if (state.mode !== "auto") return state.mode;
    const query = $("#unified-query").value.toLocaleLowerCase("it");
    if (/confront|differenz|riassum|sintetizz|analizz|chiarir/.test(query)) return "agentic";
    if (/\.(pdf|md|txt|png)\b|tipo:|nome:/.test(query)) return "simple";
    return "semantic";
  }
  function getEntries() {
    const query = $("#unified-query").value.trim().toLocaleLowerCase("it");
    if (!query) return entries;
    const formats = $$('[data-format]:checked').map(input => input.dataset.format);
    const simple = actualMode() === "simple";
    let results = entries.filter(entry => (simple || entry.source) && formats.includes(entry.kind));
    if (simple) {
      const terms = query.split(/\s+/).filter(term => term.length > 2 && !["con", "del", "delle", "dei", "che", "per", "sono", "una", "della"].includes(term));
      results = results.filter(entry => terms.some(term => `${entry.name} ${entry.passage} ${entry.note} ${entry.tags.join(" ")}`.toLocaleLowerCase("it").includes(term)));
    }
    return simple
      ? results.sort((a, b) => Number(b.kind === "folder") - Number(a.kind === "folder") || a.name.localeCompare(b.name, "it", { numeric: true }))
      : results.sort((a, b) => a.source - b.source);
  }
  function passageNode(entry) {
    const element = document.createElement("span");
    element.className = "result-passage";
    const text = entry.passage || "";
    const index = entry.emphasis ? text.indexOf(entry.emphasis) : -1;
    if (index < 0) element.textContent = text;
    else {
      element.append(document.createTextNode(text.slice(0, index)));
      const mark = document.createElement("mark");
      mark.textContent = entry.emphasis;
      element.append(mark, document.createTextNode(text.slice(index + entry.emphasis.length)));
    }
    return element;
  }
  function renderPreview() {
    const entry = byId.get(state.selected);
    $("#preview-sheet").hidden = !entry || ["folder", "png"].includes(entry.kind);
    $("#selected-file-title").textContent = entry?.name || "Nessun elemento selezionato";
    $("#selected-file-path").textContent = entry?.path || "Modifica la ricerca o i filtri.";
    $("#selected-note-text").textContent = entry?.note || "Le note dei documenti non vengono modificate da questo mockup.";
    $("#selected-file-tags").replaceChildren();
    entry?.tags.forEach(label => {
      const tag = document.createElement("span");
      tag.textContent = label;
      $("#selected-file-tags").append(tag);
    });
    $("#preview-paper-title").textContent = entry?.title || entry?.name || "";
    $("#preview-passage").textContent = entry?.passage || "";
    $("#preview-page").textContent = entry?.page || "";
    $('[data-action="open-source"]').disabled = !entry;
  }
  function renderPanel() {
    const agentic = hasAgentAnswer();
    const visible = panelIsVisible();
    if (!agentic) state.panel = "preview";
    $("#context-panel").hidden = !visible;
    $("#workspace-content").classList.toggle("panel-hidden", !visible);
    const toggle = $('[data-action="inspector"]');
    toggle.setAttribute("aria-pressed", String(visible));
    toggle.setAttribute("aria-label", visible ? "Nascondi pannello laterale" : "Mostra anteprima");
    $(".privacy-strip").hidden = !$("#unified-query").value.trim();
    $(".panel-menu").hidden = !$("#unified-query").value.trim();
    $("#answer-tab").hidden = !agentic;
    $$('[data-panel]').forEach(button => {
      const active = button.dataset.panel === state.panel;
      button.classList.toggle("active", active);
      button.setAttribute("aria-pressed", String(active));
    });
    $("#answer-content").hidden = state.panel !== "answer";
    $("#preview-content").hidden = state.panel !== "preview";
    const complete = ["contract", "agreement", "checklist", "minutes"].every(id => visibleEntries.some(entry => entry.id === id));
    $("#complete-answer").hidden = !complete;
    $("#incomplete-answer").hidden = complete;
    $(".answer-state > span:nth-child(2)").textContent = complete ? "Sintesi dimostrativa" : "Fonti dell’esempio incomplete";
    $("#privacy-description").textContent = actualMode() === "simple" || !$("#unified-query").value.trim() ? "Ricerca senza LLM · dati della demo" : "Scenario locale · nessuna chiamata AI reale";
    renderPreview();
  }
  function render() {
    const searching = Boolean($("#unified-query").value.trim());
    document.body.classList.toggle("is-searching", searching);
    syncSearchVisibility();
    visibleEntries = getEntries();
    if (state.selected && !visibleEntries.some(entry => entry.id === state.selected)) state.selected = searching ? visibleEntries[0]?.id || null : null;
    const list = $("#unified-file-list");
    list.replaceChildren();
    visibleEntries.forEach(entry => {
      const wrapper = document.createElement("div");
      wrapper.setAttribute("role", "listitem");
      const row = document.createElement("button");
      row.className = "result-row" + (entry.id === state.selected ? " selected" : "");
      row.dataset.entryId = entry.id;
      row.setAttribute("aria-pressed", String(entry.id === state.selected));
      row.innerHTML = '<span class="result-file-icon"></span><span class="result-row-main"><span class="result-file-name"></span><span class="result-path"></span></span><span class="result-row-type"></span><span class="result-row-size"></span><span class="result-row-date"></span>';
      const fileIcon = row.querySelector(".result-file-icon");
      fileIcon.classList.add(entry.kind);
      if (entry.kind === "folder") fileIcon.innerHTML = icon("folder");
      else fileIcon.textContent = entry.kind.toUpperCase();
      row.querySelector(".result-file-name").textContent = entry.name;
      row.querySelector(".result-path").textContent = searching ? [entry.path, entry.page].filter(Boolean).join(" · ") : entry.kind === "folder" ? "Cartella locale" : entry.path;
      if (searching && entry.passage) row.querySelector(".result-row-main").append(passageNode(entry));
      row.querySelector(".result-row-type").textContent = typeNames[entry.kind];
      row.querySelector(".result-row-size").textContent = entry.size;
      row.querySelector(".result-row-date").textContent = entry.date;
      if (searching && entry.source) {
        const number = document.createElement("span");
        number.className = "result-source-number";
        number.textContent = "Fonte " + entry.source;
        row.append(number);
      }
      wrapper.append(row);
      list.append(wrapper);
    });
    $("#search-context").hidden = !searching;
    $(".clear-query").hidden = !state.searchOpen;
    $("#section-kicker").hidden = !searching;
    $("#results-description").hidden = !searching;
    $('[data-action="save-search"]').hidden = !searching;
    $("#section-kicker").textContent = searching ? "La ricerca resta nel tuo spazio" : "Cartella locale";
    $("#main-heading").replaceChildren(document.createTextNode(searching ? "Risultati in Apollo" : "Apollo"));
    const badge = document.createElement("span");
    badge.className = "result-count";
    badge.id = "result-count";
    badge.textContent = visibleEntries.length;
    $("#main-heading").append(badge);
    const route = actualMode();
    $("#results-description").textContent = !searching ? "File, cartelle e contesto. Senza cambiare ambiente." : route === "agentic" ? hasAgentAnswer() ? "Confronto dimostrativo con fonti, nel pannello a lato." : "Premi Invio per avviare il confronto dimostrativo con fonti." : route === "semantic" ? "Risultati per significato, simulati. Nessuna risposta generata." : "Corrispondenze testuali nei dati della demo. Nessun LLM.";
    $("#route-description").textContent = state.mode === "auto" ? `Auto ha scelto ${modes[route].name}` : `${modes[route].name} · ${route === "simple" ? "senza LLM" : route === "semantic" ? "senza sintesi" : "con fonti"}`;
    $("#mode-name").textContent = modes[state.mode].name;
    $("#mode-icon").innerHTML = icon(modes[state.mode].icon);
    $$('[data-search-mode]').forEach(button => button.setAttribute("aria-checked", String(button.dataset.searchMode === state.mode)));
    $("#empty-list").hidden = visibleEntries.length > 0;
    $("#exploration-hint").hidden = !searching;
    $("#list-summary").textContent = searching ? `${visibleEntries.length} ${route === "simple" ? "elementi" : "documenti"} fittizi · ambito Apollo` : `${visibleEntries.length} elementi · 3 cartelle, 6 file`;
    renderPanel();
  }
  function selectFile(id, showPreview = true) {
    if (!byId.has(id) || !visibleEntries.some(entry => entry.id === id)) return;
    state.selected = id;
    if (showPreview) state.panel = "preview";
    $$('[data-entry-id]').forEach(row => {
      row.classList.remove("source-flash");
      row.classList.toggle("selected", row.dataset.entryId === id);
      row.setAttribute("aria-pressed", String(row.dataset.entryId === id));
    });
    renderPanel();
  }
  function browse(focusToggle = false) {
    closePopovers();
    resetSearchActivity();
    $("#unified-query").value = "";
    state.searchOpen = false;
    state.panel = "preview";
    render();
    if (focusToggle) $("#search-toggle").focus();
  }
  function setMode(mode) {
    resetSearchActivity();
    state.mode = mode;
    render();
  }
  function runSearch() {
    closePopovers();
    if (!$("#unified-query").value.trim()) { browse(true); return; }
    if (state.busy) return;
    resetSearchActivity();
    openSearch();
    state.busy = true;
    const revision = state.revision;
    $("#unified-search").setAttribute("aria-busy", "true");
    $(".submit-search").disabled = true;
    $("#status-text").textContent = "Ricerca simulata…";
    state.timer = setTimeout(() => {
      if (revision !== state.revision) return;
      state.timer = null;
      state.busy = false;
      state.agentRequested = actualMode() === "agentic";
      state.agentPanelVisible = state.agentRequested;
      if (state.agentRequested) state.panel = "answer";
      $("#unified-search").setAttribute("aria-busy", "false");
      $(".submit-search").disabled = false;
      $("#status-text").textContent = "Demo locale · nessun dato inviato";
      render();
      notify("Ricerca simulata: fonti e risposta sono fittizie. Nessun modello o file reale viene consultato.");
    }, 550);
  }
  document.addEventListener("folderai:focus-search", () => openSearch(true));

  $("#mode-trigger").addEventListener("click", () => {
    if ($("#search-mode-menu").hidden) openModeMenu();
    else closePopovers(true);
  });
  $("#filter-trigger").addEventListener("click", () => {
    const open = $("#search-filter-menu").hidden;
    closePopovers();
    if (open) {
      $("#search-filter-menu").hidden = false;
      $("#filter-trigger").setAttribute("aria-expanded", "true");
      $('[data-format]').focus();
    }
  });
  $("#search-mode-menu").addEventListener("keydown", event => {
    const choices = $$('[data-search-mode]');
    const current = choices.indexOf(document.activeElement);
    if (["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) {
      event.preventDefault();
      const next = event.key === "Home" ? 0 : event.key === "End" ? choices.length - 1 : (current + (event.key === "ArrowDown" ? 1 : -1) + choices.length) % choices.length;
      choices[next].focus();
    }
  });
  $$('[data-format]').forEach(input => input.addEventListener("change", render));
  $("#unified-query").addEventListener("input", () => {
    resetSearchActivity();
    render();
  });
  $("#unified-search").addEventListener("submit", event => {
    event.preventDefault();
    runSearch();
  });
  $("#unified-search").addEventListener("focusout", event => {
    if ($("#unified-search").contains(event.relatedTarget)) return;
    setTimeout(() => {
      if (state.searchOpen && !$("#unified-query").value.trim() && !$("#unified-search").contains(document.activeElement) && !dialog.open) {
        state.searchOpen = false;
        closePopovers();
        syncSearchVisibility();
      }
    }, 0);
  });
  $("#unified-file-list").addEventListener("keydown", event => {
    if (!["ArrowUp", "ArrowDown"].includes(event.key)) return;
    const row = event.target.closest("[data-entry-id]");
    if (!row) return;
    event.preventDefault();
    const rows = $$('[data-entry-id]');
    const next = Math.max(0, Math.min(rows.length - 1, rows.indexOf(row) + (event.key === "ArrowDown" ? 1 : -1)));
    rows[next]?.focus();
    if (rows[next]) selectFile(rows[next].dataset.entryId);
  });
  document.addEventListener("focusin", event => {
    if (!event.target.closest("#search-mode-menu, #mode-trigger")) {
      $("#search-mode-menu").hidden = true;
      $("#mode-trigger").setAttribute("aria-expanded", "false");
    }
    if (!event.target.closest("#search-filter-menu, #filter-trigger")) {
      $("#search-filter-menu").hidden = true;
      $("#filter-trigger").setAttribute("aria-expanded", "false");
    }
  });
  document.addEventListener("keydown", event => {
    if (dialog.open) return;
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
      event.preventDefault();
      closePopovers();
      openSearch(true);
    }
    if (event.key === "ArrowDown" && (event.target === $("#mode-trigger") || (event.altKey && event.target === $("#unified-query")))) {
      event.preventDefault();
      openModeMenu();
    }
    if (event.key === "Escape") {
      if (!$("#search-mode-menu").hidden) { event.preventDefault(); closePopovers(true); }
      else if (!$("#search-filter-menu").hidden) { event.preventDefault(); closePopovers(); $("#filter-trigger").focus(); }
      else if (state.searchOpen) { event.preventDefault(); browse(true); }
    }
  });

  document.addEventListener("click", event => {
    if (!event.target.closest(".unified-search")) {
      closePopovers();
      if (state.searchOpen && !$("#unified-query").value.trim() && !event.target.closest("#taskbar-search")) {
        state.searchOpen = false;
        syncSearchVisibility();
      }
    }
    const choice = event.target.closest("[data-search-mode]");
    if (choice) { setMode(choice.dataset.searchMode); closePopovers(true); return; }
    const source = event.target.closest("[data-source]");
    if (source) {
      if (hasAgentAnswer()) state.agentPanelVisible = true;
      else state.inspectorOpen = true;
      selectFile(source.dataset.source);
      const row = $(`[data-entry-id="${source.dataset.source}"]`);
      row?.classList.add("source-flash");
      row?.scrollIntoView({ behavior: "instant", block: "nearest" });
      return;
    }
    const row = event.target.closest("[data-entry-id]");
    if (row) { selectFile(row.dataset.entryId); return; }
    const tab = event.target.closest("[data-panel]");
    if (tab) { state.panel = tab.dataset.panel; renderPanel(); return; }
    const layout = event.target.closest("[data-layout]");
    if (layout) {
      const grid = layout.dataset.layout === "grid";
      $("#unified-file-list").classList.toggle("grid-layout", grid);
      $(".file-column-labels").hidden = grid;
      $$('[data-layout]').forEach(button => {
        const active = button === layout;
        button.classList.toggle("active", active);
        button.setAttribute("aria-pressed", String(active));
      });
      return;
    }
    const savedButton = event.target.closest("[data-saved-id]");
    if (savedButton) {
      const search = saved.get(savedButton.dataset.savedId);
      $("#unified-query").value = search.query;
      $$('[data-format]').forEach(input => { input.checked = search.formats.includes(input.dataset.format); });
      openSearch();
      setMode(search.mode);
      runSearch();
      return;
    }
    const message = event.target.closest("[data-message]");
    if (message) { notify(message.dataset.message); return; }
    const button = event.target.closest("[data-action]");
    if (!button) return;
    switch (button.dataset.action) {
      case "theme": setTheme(document.documentElement.dataset.theme === "dark" ? "light" : "dark"); break;
      case "sidebar": {
        const hidden = $(".unified-body").classList.toggle("sidebar-hidden");
        button.setAttribute("aria-pressed", String(!hidden));
        button.setAttribute("aria-label", hidden ? "Mostra barra laterale" : "Nascondi barra laterale");
        break;
      }
      case "inspector": {
        if (panelIsVisible()) {
          state.inspectorOpen = false;
          state.agentPanelVisible = false;
        } else {
          state.inspectorOpen = true;
          if (!state.selected) state.selected = visibleEntries.find(entry => entry.kind !== "folder")?.id || visibleEntries[0]?.id || null;
        }
        render();
        break;
      }
      case "toggle-search": if (state.searchOpen) browse(true); else openSearch(); break;
      case "clear": browse(true); break;
      case "browse": browse(); break;
      case "example-search":
        $("#unified-query").value = "Confronta le condizioni di rinnovo";
        $$('[data-format]').forEach(input => { input.checked = true; });
        openSearch();
        setMode("auto");
        runSearch();
        break;
      case "tag":
        $("#unified-query").value = button.dataset.tag;
        openSearch();
        setMode("simple");
        notify("Filtro dimostrativo nei nomi, nei passaggi e nei metadati fittizi.");
        break;
      case "close-dialog": dialog.close(); break;
      case "models":
        showDialog("Ricerca locale, ruoli distinti", "Configurazione puramente illustrativa. Nessun modello è connesso.", '<div class="dialog-line"><span>Auto / routing</span><strong>Regole dimostrative</strong></div><div class="dialog-line"><span>Semantica</span><strong>Embedding locali · esempio</strong></div><div class="dialog-line"><span>Assistente / agente</span><strong>LLM via LM Studio · esempio</strong></div><div class="dialog-note">Nessun fallback cloud. Cambiare modalità non estende l’ambito autorizzato. Modelli e implementazione restano da definire.</div>');
        break;
      case "preferences":
        showDialog("Preferenze", "Nel mockup puoi cambiare il tema. Le altre impostazioni sono soltanto illustrative.", '<label class="dialog-line"><span>Aspetto</span><select id="unified-theme"><option value="light">Chiaro</option><option value="dark">Scuro</option></select></label><div class="dialog-line"><span>Ricerca</span><strong>Controlli nella barra</strong></div><div class="dialog-line"><span>Cartella iniziale</span><strong>Home</strong></div>');
        $("#unified-theme").value = document.documentElement.dataset.theme;
        $("#unified-theme").addEventListener("change", event => setTheme(event.target.value));
        break;
      case "follow-up":
        showDialog("Punti da chiarire · esempio", "Una proposta di approfondimento contestuale, non una nuova analisi eseguita.", '<div class="dialog-line"><span>01</span><strong>Ambito dell’accordo integrativo</strong></div><div class="dialog-line"><span>02</span><strong>Validità e data di sottoscrizione</strong></div><div class="dialog-line"><span>03</span><strong>Procedura da confermare con le parti</strong></div><div class="dialog-note">Nessuna conclusione sulla prevalenza delle clausole viene dedotta automaticamente.</div>');
        break;
      case "open-source": {
        const entry = byId.get(state.selected);
        if (!entry) break;
        showDialog(entry.name, `${entry.path} · ${entry.page || "Elemento illustrativo"}`, '<p id="source-dialog-text"></p><div class="dialog-note">Passaggio incorporato nel mockup. Non viene aperto alcun file reale.</div>');
        $("#source-dialog-text").textContent = entry.passage || entry.note;
        break;
      }
      case "save-search":
        if (!$("#unified-query").value.trim()) { notify("Avvia una ricerca prima di salvarla nella demo."); break; }
        showDialog("Salva ricerca", "Query, modalità e filtri vengono ricordati solo nella memoria della pagina.", '<label for="saved-search-name" style="font-size:11px;display:block;margin:16px 0 7px">Nome</label><input id="saved-search-name" value="Rinnovi Apollo" maxlength="70" style="width:100%;padding:10px;border:1px solid var(--border);border-radius:8px;background:var(--surface)"><button class="outlined-button" data-action="confirm-save" style="margin-top:16px">Salva nella demo</button>');
        break;
      case "confirm-save": {
        const name = $("#saved-search-name").value.trim();
        if (!name) { $("#saved-search-name").focus(); break; }
        const id = String(saved.size + 1);
        saved.set(id, { query: $("#unified-query").value, mode: state.mode, formats: $$('[data-format]:checked').map(input => input.dataset.format) });
        const link = document.createElement("button");
        link.className = "nav-row";
        link.dataset.savedId = id;
        link.title = name;
        link.innerHTML = icon("search-folder") + "<span></span>";
        link.querySelector("span").textContent = name;
        $("#saved-search-list").append(link);
        dialog.close();
        notify("Ricerca salvata solo nella demo. Si azzera ricaricando la pagina.");
        break;
      }
    }
  });
  render();
})();
