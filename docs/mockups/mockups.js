/* Presentation-only interactions. No filesystem access, network requests or AI calls. */
(() => {
  "use strict";

  const paths = {
    layers: '<path d="m12 3 9 5-9 5-9-5 9-5Z"/><path d="m3 12 9 5 9-5M3 16l9 5 9-5"/>',
    home: '<path d="m3 10 9-7 9 7v10a1 1 0 0 1-1 1h-5v-7H9v7H4a1 1 0 0 1-1-1V10Z"/>',
    clock: '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
    folder: '<path d="M3 6a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V6Z"/>',
    "search-folder": '<path d="M12 20H4a1 1 0 0 1-1-1V6a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v2"/><circle cx="17" cy="16" r="4"/><path d="m20 19 3 3"/>',
    sparkles: '<path d="m12 3 2.5 6.5L21 12l-6.5 2.5L12 21l-2.5-6.5L3 12l6.5-2.5L12 3ZM20 2v4m-2-2h4"/>',
    settings: '<path d="m9 3-.6 2.2-2 .9L4.3 5.5 2 9.5 3.6 11v2L2 14.5l2.3 4 2.1-.6 2 .9L9 21h6l.6-2.2 2-.9 2.1.6 2.3-4-1.6-1.5v-2L22 9.5l-2.3-4-2.1.6-2-.9L15 3H9Z"/><circle cx="12" cy="12" r="3"/>',
    drive: '<path d="m5 4-3 12h20L19 4H5Z"/><rect x="2" y="16" width="20" height="5" rx="1"/><path d="M17 18.5h2"/>',
    "arrow-left": '<path d="m12 5-7 7 7 7M5 12h15"/>',
    "arrow-right": '<path d="m12 5 7 7-7 7M4 12h15"/>',
    "arrow-up": '<path d="m5 12 7-7 7 7M12 5v15"/>',
    "arrow-up-right": '<path d="M6 18 18 6M6 6h12v12"/>',
    "chevron-right": '<path d="m9 5 7 7-7 7"/>',
    "chevron-down": '<path d="m5 9 7 7 7-7"/>',
    search: '<circle cx="10.5" cy="10.5" r="7"/><path d="m16 16 5 5"/>',
    eye: '<path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12Z"/><circle cx="12" cy="12" r="3"/>',
    activity: '<path d="M3 12h4l3-8 4 16 3-8h4"/>',
    plus: '<path d="M12 4v16M4 12h16"/>',
    list: '<path d="M8 5h13M8 12h13M8 19h13M3 5h.01M3 12h.01M3 19h.01"/>',
    grid: '<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>',
    columns: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16M15 4v16"/>',
    split: '<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M12 4v16M6 8h3M15 8h3"/>',
    file: '<path d="M14 2H5v20h14V7l-5-5Z"/><path d="M14 2v6h5M8 12h8M8 16h6"/>',
    image: '<rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8" cy="8" r="1.5"/><path d="m21 15-5-5L5 21"/>',
    message: '<path d="M21 15a2 2 0 0 1-2 2H8l-5 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v10Z"/><path d="M7 8h10M7 12h6"/>',
    more: '<circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>',
    external: '<path d="M14 3h7v7M21 3 10 14M10 3H4a1 1 0 0 0-1 1v16a1 1 0 0 0 1 1h16a1 1 0 0 0 1-1v-6"/>',
    check: '<path d="m5 12 4 4L19 6"/>',
    "check-circle": '<circle cx="12" cy="12" r="9"/><path d="m8 12 3 3 5-6"/>',
    shield: '<path d="m12 3 8 3v6c0 5-8 9-8 9S4 17 4 12V6l8-3Z"/><path d="m8 12 3 3 5-6"/>',
    lock: '<rect x="5" y="10" width="14" height="11" rx="2"/><path d="M8 10V7a4 4 0 0 1 8 0v3M12 14v3"/>',
    moon: '<path d="M20.5 14A8.5 8.5 0 0 1 10 3.5 9 9 0 1 0 20.5 14Z"/>',
    sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M2 12h2M20 12h2M5 5l1.5 1.5M17.5 17.5 19 19M5 19l1.5-1.5M17.5 6.5 19 5"/>',
    database: '<ellipse cx="12" cy="5" rx="8" ry="3"/><path d="M4 5v14c0 1.7 3.6 3 8 3s8-1.3 8-3V5M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3"/>',
    sliders: '<path d="M4 6h6m4 0h6M4 12h12m4 0h0M4 18h2m4 0h10"/><circle cx="12" cy="6" r="2"/><circle cx="18" cy="12" r="2"/><circle cx="8" cy="18" r="2"/>',
    bookmark: '<path d="M5 3h14v18l-7-5-7 5V3Z"/>',
    wand: '<path d="m4 20 12-12 4 4L8 24M14 10l4 4M6 2v4M4 4h4M19 2v4M17 4h4" transform="translate(0 -2)"/>',
    cpu: '<rect x="6" y="6" width="12" height="12" rx="2"/><path d="M10 6V3M14 6V3M10 21v-3M14 21v-3M6 10H3M6 14H3M21 10h-3M21 14h-3"/><rect x="9" y="9" width="6" height="6" rx="1"/>',
    info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v6M12 7h.01"/>',
    alert: '<path d="m12 3 10 18H2L12 3Z"/><path d="M12 9v5M12 17h.01"/>',
    copy: '<rect x="8" y="8" width="13" height="13" rx="2"/><path d="M16 8V3H3v13h5"/>',
    book: '<path d="M12 6S7 3 2 5v15s5-2 10 1c5-3 10-1 10-1V5c-5-2-10 1-10 1ZM12 6v15"/>',
    x: '<path d="m6 6 12 12M6 18 18 6"/>'
  };

  const svg = name => `<svg class="icon" viewBox="0 0 24 24" aria-hidden="true">${paths[name] || paths.file}</svg>`;
  function renderIcons(root = document) {
    root.querySelectorAll("[data-icon]").forEach(element => {
      element.innerHTML = svg(element.dataset.icon);
    });
  }
  renderIcons();

  const toast = document.createElement("div");
  toast.className = "toast";
  toast.setAttribute("role", "status");
  toast.setAttribute("aria-live", "polite");
  document.body.append(toast);
  let toastTimer;
  function notify(message) {
    clearTimeout(toastTimer);
    toast.textContent = message;
    toast.classList.add("is-visible");
    toastTimer = setTimeout(() => toast.classList.remove("is-visible"), 4200);
  }

  function setTheme(theme) {
    document.documentElement.dataset.theme = theme;
    document.querySelectorAll('[data-action="theme"]').forEach(button => {
      button.innerHTML = svg(theme === "dark" ? "sun" : "moon");
      button.setAttribute("aria-label", theme === "dark" ? "Passa al tema chiaro" : "Passa al tema scuro");
    });
    try { localStorage.setItem("folderai-concept-theme", theme); } catch { /* Local files may block storage. */ }
  }
  let initialTheme = "light";
  try { initialTheme = localStorage.getItem("folderai-concept-theme") || "light"; } catch { /* Default stays light. */ }
  setTheme(initialTheme === "dark" ? "dark" : "light");

  const dialog = document.createElement("dialog");
  dialog.setAttribute("aria-labelledby", "dialog-title");
  document.body.append(dialog);
  function showDialog(title, description, content = "") {
    dialog.innerHTML = '<div class="dialog-heading"><h2 id="dialog-title"></h2><button class="icon-button" data-action="close-dialog" aria-label="Chiudi">' + svg("x") + '</button></div><p id="dialog-description"></p><div id="dialog-content"></div>';
    dialog.querySelector("h2").textContent = title;
    dialog.querySelector("#dialog-description").textContent = description;
    dialog.querySelector("#dialog-content").innerHTML = content;
    dialog.setAttribute("aria-describedby", "dialog-description");
    renderIcons(dialog);
    if (!dialog.open) dialog.showModal();
  }
  dialog.addEventListener("click", event => { if (event.target === dialog) dialog.close(); });

  const files = {
    assets: { tags: ["Apollo"], note: "Elementi grafici e materiali del progetto." },
    design: { tags: ["Apollo"], note: "Esplorazioni e versioni del design." },
    documents: { tags: ["Apollo"], note: "Documentazione condivisa del progetto." },
    research: { tags: [], note: "Interviste e materiali di ricerca." },
    brief: { tags: ["Apollo", "Approvato"], caption: "Documento PDF · 4 pagine", title: "Il punto di partenza.", note: "Brief approvato. Tenere come riferimento per gli obiettivi." },
    budget: { tags: [], caption: "Foglio di calcolo · apertura esterna", note: "Apri con l'app associata. L'anteprima Office non è prevista nel primo rilascio." },
    contract: { tags: ["Apollo", "Approvato"], caption: "Documento PDF · 12 pagine", title: "Un accordo, una direzione.", note: "Documento contrattuale di riferimento. Dati fittizi." },
    moodboard: { tags: ["Apollo"], caption: "Immagine PNG · anteprima illustrativa", note: "Riferimenti visuali della prima esplorazione." },
    notes: { tags: ["Apollo", "Da rivedere"], caption: "Documento Markdown", note: "Raccogliere i feedback prima del prossimo incontro." },
    proposal: { tags: ["Apollo", "Da rivedere"], caption: "Documento PDF · 8 pagine", title: "Dalle idee al prossimo passo.", note: "Pronta per il prossimo incontro. Verificare obiettivi e tempistiche prima di condividere la proposta." },
    archive: { tags: [], caption: "Archivio ZIP · apertura dimostrativa", note: "Archivio dei materiali di ricerca. Nessuna estrazione reale nel mockup." },
    roadmap: { tags: ["Da rivedere"], caption: "Documento Markdown", note: "Rivedere le milestone con il team." },
    hidden: { tags: [], caption: "File JSON nascosto · esempio", note: "Esempio per il comando Mostra elementi nascosti." }
  };
  const rowContainer = document.querySelector("#file-rows");
  let activeTag = null;
  let showHidden = false;
  let selectedFile = "proposal";
  let ascending = true;

  function selectFile(id) {
    const row = document.querySelector(`[data-file="${id}"]`);
    if (!row) return;
    selectedFile = id;
    document.querySelectorAll("tr[data-file]").forEach(item => {
      const selected = item === row;
      item.classList.toggle("is-selected", selected);
      item.setAttribute("aria-selected", String(selected));
    });
    const data = files[id] || { tags: [], note: "Nessuna nota." };
    const name = row.querySelector(".name-text").textContent;
    document.querySelector("#inspector-name").textContent = name;
    document.querySelector("#inspector-caption").textContent = data.caption || "Cartella locale";
    document.querySelector("#inspector-size").textContent = row.cells[2].textContent === "—" ? "Su richiesta" : row.cells[2].textContent;
    document.querySelector("#inspector-modified").textContent = row.cells[3].textContent;
    document.querySelector("#inspector-note").textContent = data.note;
    const tags = document.querySelector("#inspector-tags");
    tags.replaceChildren();
    data.tags.forEach(label => {
      const tag = document.createElement("span");
      tag.className = "tag" + (label === "Da rivedere" ? " amber" : label === "Approvato" ? " green" : "");
      tag.textContent = label;
      tags.append(tag);
    });
    if (!data.tags.length) {
      const empty = document.createElement("span");
      empty.textContent = "Nessun tag";
      empty.style.cssText = "font-size:10px;color:var(--muted)";
      tags.append(empty);
    }
    const isPDF = row.dataset.kind === "pdf";
    document.querySelector("#document-preview").hidden = !isPDF;
    document.querySelector("#preview-fallback").hidden = isPDF;
    if (isPDF) {
      document.querySelector("#paper-title").textContent = data.title;
      document.querySelector(".paper > p").textContent = name.replace(/\.pdf$/, "") + " · Esempio";
      document.querySelector("#document-preview").setAttribute("aria-label", "Anteprima illustrativa di " + name + ", non un documento reale");
    } else {
      document.querySelector("#preview-fallback").innerHTML = svg(row.dataset.kind === "folder" ? "folder" : row.dataset.kind === "image" ? "image" : "file");
    }
    document.querySelector("#selection-status").textContent = "1 elemento selezionato";
  }

  function filterFiles() {
    if (!rowContainer) return;
    const query = document.querySelector("#file-search").value.toLocaleLowerCase("it").trim();
    const allRows = [...rowContainer.querySelectorAll("tr")];
    allRows.forEach(row => {
      const matchName = row.querySelector(".name-text").textContent.toLocaleLowerCase("it").includes(query);
      const matchTag = !activeTag || (files[row.dataset.file]?.tags || []).includes(activeTag);
      row.hidden = !matchName || !matchTag || (row.dataset.file === "hidden" && !showHidden);
    });
    const visible = allRows.filter(row => !row.hidden);
    const folders = visible.filter(row => row.dataset.kind === "folder").length;
    document.querySelector("#file-count").textContent = `${visible.length} elementi · ${folders} cartelle, ${visible.length - folders} file`;
    document.querySelector("#no-files").hidden = visible.length > 0;
    document.querySelector(".inspector").hidden = !visible.length;
    if (!visible.some(row => row.dataset.file === selectedFile)) {
      if (visible.length) selectFile(visible[0].dataset.file);
      else document.querySelector("#selection-status").textContent = "Nessun elemento selezionato";
    }
  }

  if (rowContainer) {
    const hiddenRow = document.createElement("tr");
    hiddenRow.tabIndex = 0;
    hiddenRow.hidden = true;
    hiddenRow.dataset.file = "hidden";
    hiddenRow.dataset.kind = "txt";
    hiddenRow.setAttribute("aria-selected", "false");
    hiddenRow.innerHTML = '<td><div class="file-name"><span class="file-type-icon txt">JSON</span><span class="name-text">.folderai.json</span></div></td><td>File JSON</td><td>2 KB</td><td>Ieri, 09:00</td>';
    rowContainer.append(hiddenRow);
    const empty = document.createElement("p");
    empty.id = "no-files";
    empty.hidden = true;
    empty.textContent = "Nessun elemento dimostrativo corrisponde al filtro.";
    empty.style.cssText = "padding:30px 0;color:var(--muted);font-size:12px";
    document.querySelector(".file-table-footer").before(empty);
    document.querySelector("#file-search").addEventListener("input", () => { activeTag = null; filterFiles(); });
    rowContainer.addEventListener("click", event => {
      const row = event.target.closest("tr[data-file]");
      if (row) selectFile(row.dataset.file);
    });
    rowContainer.addEventListener("keydown", event => {
      const row = event.target.closest("tr[data-file]");
      if (!row) return;
      if (event.key === "Enter" || event.key === " ") { event.preventDefault(); selectFile(row.dataset.file); }
      if (["ArrowDown", "ArrowUp"].includes(event.key)) {
        event.preventDefault();
        const rows = [...rowContainer.querySelectorAll("tr")].filter(item => !item.hidden);
        const index = Math.max(0, Math.min(rows.length - 1, rows.indexOf(row) + (event.key === "ArrowDown" ? 1 : -1)));
        rows[index]?.focus();
        if (rows[index]) selectFile(rows[index].dataset.file);
      }
    });
  }

  let mode = "auto";
  let running = false;
  const modeLabels = { auto: "Auto", simple: "Semplice", semantic: "Semantica", agentic: "Agentica" };
  function routedMode() {
    if (mode !== "auto") return mode;
    const query = document.querySelector("#ai-query").value.toLocaleLowerCase("it");
    if (/confront|differenz|sintetizz|riassum|ricostrui|analizz/.test(query)) return "agentic";
    if (/\.(pdf|md|txt|png)\b|nome:|tipo:|estensione:/.test(query)) return "simple";
    return "semantic";
  }
  function updateMode() {
    if (!document.querySelector("#ai-query")) return;
    const route = routedMode();
    document.querySelectorAll("[data-mode]").forEach(button => {
      const selected = button.dataset.mode === mode;
      button.classList.toggle("is-active", selected);
      button.setAttribute("aria-pressed", String(selected));
    });
    document.querySelector("#route-label").textContent = mode === "auto" ? `Auto → ${modeLabels[route]}` : modeLabels[route];
    document.querySelector("#agent-answer").hidden = route !== "agentic";
    document.querySelector("#non-agent-answer").hidden = route === "agentic";
    document.querySelector('[data-action="copy-answer"]').hidden = route !== "agentic";
    const activity = document.querySelector("#activity-steps");
    if (route === "agentic") {
      document.querySelector("#results-heading").innerHTML = 'Fonti trovate <span style="margin-left:5px">4 documenti</span>';
      document.querySelector("#activity-summary").textContent = "Ricerca completata · 4 fonti consultate";
      activity.innerHTML = '<li>' + svg("check-circle") + 'Ricerca nei contenuti autorizzati<small>3 cartelle</small></li><li>' + svg("check-circle") + 'Lettura dei passaggi pertinenti<small>4 fonti</small></li><li>' + svg("check-circle") + 'Confronto e risposta con citazioni<small>Completato</small></li>';
    } else {
      document.querySelector("#results-heading").innerHTML = 'Risultati dimostrativi <span style="margin-left:5px">4 documenti</span>';
      document.querySelector("#non-agent-title").textContent = route === "simple" ? "Corrispondenze, non interpretazioni." : "Il significato, senza la sintesi.";
      document.querySelector("#non-agent-description").textContent = route === "simple" ? "La ricerca semplice usa nomi, testo e filtri. Non richiede un LLM. In questo mockup i quattro risultati restano esemplificativi." : "La ricerca semantica recupera passaggi per significato. Non genera una risposta e non avvia un agente. Questi risultati sono fittizi.";
      document.querySelector("#activity-summary").textContent = route === "simple" ? "Demo · nessun modello necessario" : "Demo · recupero semantico locale";
      activity.innerHTML = '<li>' + svg("check-circle") + (route === "simple" ? 'Nome, testo e filtri<small>Senza LLM</small>' : 'Recupero per significato<small>Embedding locali</small>') + '</li><li>' + svg("check-circle") + 'Elenco di risultati dimostrativi<small>Nessuna sintesi</small></li>';
    }
  }

  const searchForm = document.querySelector("#ai-search-form");
  if (searchForm) {
    document.querySelector("#ai-query").addEventListener("input", updateMode);
    searchForm.addEventListener("submit", event => {
      event.preventDefault();
      if (running) return;
      if (!document.querySelector("#ai-query").value.trim()) {
        notify("Scrivi una richiesta per provare la simulazione.");
        document.querySelector("#ai-query").focus();
        return;
      }
      running = true;
      const button = document.querySelector("#run-search");
      button.disabled = true;
      document.querySelector("#ai-status").textContent = "Simulazione di ricerca in corso…";
      document.querySelector("#activity-summary").textContent = "Simulazione in corso…";
      setTimeout(() => {
        running = false;
        button.disabled = false;
        updateMode();
        document.querySelector("#ai-status").textContent = "Demo locale · nessun dato inviato";
        notify("Simulazione completata. I documenti e le risposte restano esempi fittizi.");
      }, 850);
    });
    updateMode();
  }

  document.addEventListener("click", async event => {
    const citation = event.target.closest(".citation");
    if (citation) {
      event.preventDefault();
      const source = document.querySelector(citation.getAttribute("href"));
      document.querySelectorAll(".source-card").forEach(card => card.classList.remove("is-highlighted"));
      source.classList.add("is-highlighted");
      source.scrollIntoView({ behavior: matchMedia("(prefers-reduced-motion: reduce)").matches ? "instant" : "smooth", block: "nearest" });
      source.focus({ preventScroll: true });
      return;
    }
    const sourceButton = event.target.closest("[data-open-source]");
    if (sourceButton) {
      const source = document.querySelector("#source-" + sourceButton.dataset.openSource);
      showDialog(source.querySelector("h3").textContent, source.querySelector(".source-bottom > span").textContent, '<blockquote id="source-passage" style="margin:0;background:var(--subtle);padding:18px;border-radius:9px;color:var(--secondary);line-height:1.9"></blockquote><p style="font-size:10px;margin-bottom:0">Fonte fittizia incorporata nel mockup. Nessun documento viene aperto sul dispositivo.</p>');
      dialog.querySelector("#source-passage").textContent = source.querySelector("p").textContent;
      return;
    }
    const view = event.target.closest("[data-view]");
    if (view) {
      document.querySelectorAll("[data-view]").forEach(button => {
        button.classList.toggle("is-active", button === view);
        button.setAttribute("aria-pressed", String(button === view));
      });
      const filelist = document.querySelector("#filelist");
      filelist.classList.toggle("is-grid", view.dataset.view === "grid");
      filelist.classList.toggle("is-columns", view.dataset.view === "columns");
      document.querySelector("#column-browser").hidden = view.dataset.view !== "columns";
      return;
    }
    const modeButton = event.target.closest("[data-mode]");
    if (modeButton) { mode = modeButton.dataset.mode; updateMode(); return; }
    const selectButton = event.target.closest("[data-select-file]");
    if (selectButton) { selectFile(selectButton.dataset.selectFile); return; }
    const demo = event.target.closest("[data-demo-message]");
    if (demo) { notify(demo.dataset.demoMessage); return; }
    const button = event.target.closest("[data-action]");
    if (!button) return;
    switch (button.dataset.action) {
      case "theme": setTheme(document.documentElement.dataset.theme === "dark" ? "light" : "dark"); break;
      case "close-dialog": dialog.close(); break;
      case "preferences":
        showDialog("Preferenze", "Esempio della sezione impostazioni. Qui è interattiva soltanto la scelta del tema.", '<label class="model-role"><span>Tema dell’interfaccia</span><select id="theme-select" aria-label="Tema"><option value="light">Chiaro</option><option value="dark">Scuro</option></select></label><div class="model-role"><span>Vista iniziale</span><strong>Dettagli</strong></div><div class="model-role"><span>Cartella iniziale</span><strong>Home</strong></div><div class="model-role"><span>Estensioni dei file</span><strong>Visibili</strong></div><p style="font-size:10px;margin-bottom:0">Il concept non modifica preferenze del sistema operativo o associazioni dei file.</p>');
        dialog.querySelector("#theme-select").value = document.documentElement.dataset.theme;
        dialog.querySelector("#theme-select").addEventListener("change", event => setTheme(event.target.value));
        break;
      case "models":
        showDialog("Un modello per ogni ruolo", "Configurazione esemplificativa, non una scelta tecnica già approvata. Un modello può svolgere più ruoli.", '<div class="model-role"><span>Routing Auto</span><strong>Regole + modello rapido</strong></div><div class="model-role"><span>Embedding</span><strong>BGE-M3 · locale</strong></div><div class="model-role"><span>Assistente e agente</span><strong>Qwen3 8B · LM Studio</strong></div><div class="model-role"><span>OCR / visione</span><strong>Non richiesto in questo esempio</strong></div><div class="dialog-footnote">Tutto è simulato. Nessun modello viene caricato e nessun dato viene inviato a un provider.</div>');
        break;
      case "activities": showDialog("Centro attività", "Nessuna operazione in corso in questa finestra.", '<div class="dialog-footnote">La demo non legge, copia, sposta o elimina file reali.</div>'); break;
      case "focus-search": document.querySelector("#file-search, #ai-query")?.focus(); break;
      case "hidden-files":
        showHidden = !showHidden;
        button.setAttribute("aria-pressed", String(showHidden));
        filterFiles();
        notify(showHidden ? "Elemento nascosto dimostrativo ora visibile." : "Elementi nascosti non visibili.");
        break;
      case "saved-search": activeTag = null; document.querySelector("#file-search").value = ".pdf"; filterFiles(); notify("Ricerca salvata dimostrativa: file PDF in Apollo."); break;
      case "tag-filter": activeTag = activeTag === button.dataset.tag ? null : button.dataset.tag; document.querySelector("#file-search").value = ""; filterFiles(); notify(activeTag ? "Filtro per tag: " + activeTag + ". Premi di nuovo il tag per rimuoverlo." : "Filtro per tag rimosso."); break;
      case "split": {
        const active = document.querySelector("#file-area").classList.toggle("is-split");
        document.querySelector("#secondary-pane").hidden = !active;
        button.setAttribute("aria-pressed", String(active));
        button.setAttribute("aria-label", active ? "Disattiva doppio pannello" : "Attiva doppio pannello");
        break;
      }
      case "sort-name": {
        ascending = !ascending;
        [...rowContainer.rows].sort((a, b) => {
          const folderDifference = Number(b.dataset.kind === "folder") - Number(a.dataset.kind === "folder");
          return folderDifference || a.querySelector(".name-text").textContent.localeCompare(b.querySelector(".name-text").textContent, "it", { numeric: true, sensitivity: "base" }) * (ascending ? 1 : -1);
        }).forEach(row => rowContainer.append(row));
        button.closest("th").setAttribute("aria-sort", ascending ? "ascending" : "descending");
        button.querySelector("svg").style.transform = ascending ? "" : "rotate(180deg)";
        notify("Nome " + (ascending ? "crescente" : "decrescente") + ": cartelle sempre prima dei file.");
        break;
      }
      case "new-folder": {
        const names = [...rowContainer.querySelectorAll(".name-text")].map(element => element.textContent);
        let name = "Nuova cartella";
        let number = 1;
        while (names.includes(name)) {
          name = "Nuova cartella - copy" + (number === 1 ? "" : ` (${number})`);
          number += 1;
        }
        const id = "new-folder-" + Object.keys(files).length;
        files[id] = { tags: [], note: "Cartella creata soltanto nella memoria di questo mockup." };
        const row = document.createElement("tr");
        row.dataset.file = id;
        row.dataset.kind = "folder";
        row.tabIndex = 0;
        row.setAttribute("aria-selected", "false");
        row.innerHTML = '<td><div class="file-name"><span class="file-type-icon folder">' + svg("folder") + '</span><span class="name-text"></span></div></td><td>Cartella</td><td>—</td><td>Adesso</td>';
        row.querySelector(".name-text").textContent = name;
        rowContainer.prepend(row);
        activeTag = null;
        document.querySelector("#file-search").value = "";
        filterFiles();
        selectFile(id);
        notify("Cartella aggiunta solo alla demo. Nessun file creato sul dispositivo.");
        break;
      }
      case "open-file": notify("Apertura simulata: nessuna applicazione o documento reale viene avviato."); break;
      case "deepen": mode = "agentic"; updateMode(); notify("Vista agentica dimostrativa. Nessuna chiamata a un modello."); break;
      case "copy-answer":
        try {
          await navigator.clipboard.writeText("ESEMPIO FITTIZIO — " + document.querySelector("#agent-answer").innerText);
          notify("Sintesi dimostrativa copiata negli appunti.");
        } catch { notify("Il browser non consente l'accesso agli appunti da questo contesto. Puoi selezionare il testo manualmente."); }
        break;
      case "save-search":
        showDialog("Salva ricerca", "Il mockup aggiunge un collegamento solo per questa pagina, senza persistenza.", '<label style="font-size:11px;display:block;margin-bottom:7px" for="search-name">Nome della ricerca</label><input id="search-name" value="Rinnovi Apollo" maxlength="60" style="width:100%;padding:10px;border:1px solid var(--line-strong);border-radius:7px;background:var(--surface);color:var(--text)"><button class="button primary" data-action="confirm-save-search" style="margin-top:16px">Salva nella demo</button>');
        break;
      case "confirm-save-search": {
        const name = dialog.querySelector("#search-name").value.trim();
        if (!name) { dialog.querySelector("#search-name").focus(); return; }
        const item = document.createElement("button");
        item.className = "nav-item";
        item.dataset.demoMessage = "Ricerca salvata nella memoria del mockup: " + name;
        item.innerHTML = svg("search-folder") + '<span class="nav-text"></span>';
        item.querySelector("span.nav-text").textContent = name;
        document.querySelector('[title="Rinnovi Apollo"]').parentElement.append(item);
        dialog.close();
        notify("Ricerca aggiunta alla sidebar della demo. Si azzera ricaricando la pagina.");
        break;
      }
    }
  });

  document.addEventListener("keydown", event => {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
      event.preventDefault();
      const search = document.querySelector("#file-search, #ai-query");
      search?.focus();
      search?.select();
    }
  });
})();
