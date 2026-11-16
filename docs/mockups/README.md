# FolderAI — Mockup UI

## Studio corrente: UI unificata «Workspace sobrio»

[Anteprima statica dell'explorer](unified-preview.png).

Aprire [folderai-unified.html](folderai-unified.html) direttamente nel browser, anche offline. In alternativa, con il server descritto sotto: http://127.0.0.1:8000/folderai-unified.html.

**Mockup e direzione visiva validati dall'utente.** È il riferimento corrente per lo stile della UI unificata macOS/Windows, senza due skin native separate; non costituisce una specifica completa né una validazione delle funzionalità ancora assenti nella demo. Decisioni UI e limiti dell'approvazione sono raccolti in [`analysis/05-ui-design.md`](../../analysis/05-ui-design.md). I prototipi precedenti restano disponibili come materiale storico.

- Superfici opache, accento blu, bordi sottili, icone lineari e temi chiaro/scuro. Font locali di sistema; Inter non è ancora distribuita con il mockup.
- All'avvio: file e cartelle, sidebar senza profilo, ricerca ridotta alla lente, pannello destro chiuso e pulsanti delle operazioni nascosti.
- I tre controlli **esterni alla finestra** mostrano Explorer, un esempio con Assistente e l'anteprima. Non sono comandi previsti per l'app.
- Provare elenco/griglia, selezione e frecce da tastiera, sidebar nascondibile, sezioni richiudibili e albero Apollo espandibile.
- La lente o Cmd/Ctrl+K aprono la ricerca. Semplice filtra nomi, tag e note fittizi; Semantica usa lo stesso filtro dopo Invio, senza un vero indice. Agentica e Auto aprono l'input conversazionale esteso.
- Il pannello destro alterna **Dettagli / Assistente**. È ridimensionabile con il separatore, anche tramite frecce da tastiera; nelle finestre strette viene sovrapposto all'area file per non tagliare l'input.
- Input AI **solo testuale**, multilinea: Invio va a capo, Cmd/Ctrl+Invio o il pulsante inviano. Nessun allegato o input per immagini in questa fase.
- Vicino all'input: **Globale = cartelle abilitate**, oppure **Cartella e sottocartelle**, con cartella dimostrativa modificabile. Navigare nell'explorer non cambia questo ambito.
- «Con assistente» mostra il confronto Apollo fisso, con due citazioni apribili in Dettagli e ritorno alla conversazione. Le richieste libere ricevono soltanto un avviso dimostrativo, non risposte inventate. Il suggerimento nella conversazione carica il testo senza inviarlo.
- Le due schede conservano conversazione, bozza, ambito, ricerca e vista nella memoria della pagina. Ricaricando si perde tutto. L'utente ha successivamente escluso la cronologia persistente dal prodotto (AI-DEC-009); il relativo pulsante illustrativo non va interpretato come requisito ancora valido.
- In **Impostazioni** si provano tema, densità compatta e visibilità dei comandi illustrativi Copia/Elimina. Le note sono modificabili solo in memoria; tag, percorsi e anteprime sono illustrativi.

Nessuna registrazione, risorsa remota, accesso al filesystem, chiamata AI o operazione reale sui file. La demo non implementa tutte le decisioni discusse: ad esempio doppio pannello, vista a colonne, filtri avanzati, gestione completa delle schede e centro attività restano fuori da questo studio di stile. Non attesta accessibilità o compatibilità multipiattaforma già validate.

Asset dedicati: `unified.css`, `unified.js`. I file dei prototipi precedenti non sono stati modificati da questo studio.

Validazione dello studio: sintassi JavaScript verificata con `node --check docs/mockups/unified.js`; smoke test su Chromium/Brave headless di temi, ricerca, viste, pannelli, citazioni, ambito, input multilinea, stato per scheda, tastiera e impostazioni. Controllati caricamento senza richieste HTTP, assenza di errori JavaScript e layout a larghezze 720/950/1100/1440 px. Non equivale a una validazione su tutti i browser o sistemi operativi.

## Prototipi precedenti: explorer-first per piattaforma

Due render HTML della stessa esperienza, con interpretazioni grafiche macOS e Windows 11 / WinUI 3.

| Variante | File | Stile |
| --- | --- | --- |
| macOS | [folderai-macos.html](folderai-macos.html) | Chrome traslucido, accenti blu e superfici morbide |
| Windows 11 / WinUI 3 | [folderai-winui.html](folderai-winui.html) | Superfici stile Mica, controlli Fluent e caption Windows |

## Impostazione comune

- **Schede nella prima riga della finestra**, accanto ai controlli nativi illustrativi, come nei terminali. Nessuna barra di schede a metà dell'interfaccia.
- All'apertura si vedono **file e cartelle**, non una ricerca già avviata.
- **Ricerca inizialmente ridotta alla sola lente**. Al clic si espande orizzontalmente nella toolbar esistente, senza aggiungere una riga o ridurre l'altezza dell'elenco.
- Scelta Semplice / Semantica / Agentica / Auto dentro la barra espansa. Semplice è il valore iniziale dimostrativo, non una nuova decisione sui requisiti del prodotto.
- Filtri e ambito compaiono soltanto durante la ricerca. Non esiste un toggle globale Explorer/AI.
- **Pannello laterale chiuso di default**: l'area dei file occupa tutta la larghezza disponibile dopo la sidebar.
- Il pannello di anteprima si apre su richiesta. La risposta agentica appare soltanto dopo aver inviato esplicitamente una richiesta.
- Chiusura della ricerca e cancellazione di un'attività dimostrativa pendente ripristinano l'explorer, senza un'apertura tardiva del pannello AI.

## Apertura

Aprire direttamente il file HTML nel browser. Non servono build, installazioni, font remoti o connessioni di rete.

In alternativa, dalla radice del repository:

```sh
python3 -m http.server 8000 --bind 127.0.0.1 --directory docs/mockups
```

- http://127.0.0.1:8000/folderai-macos.html
- http://127.0.0.1:8000/folderai-winui.html

I collegamenti di confronto sono nella didascalia esterna alla finestra dell'app.

## Interazioni da provare

### Explorer

- All'avvio: nove elementi fittizi, colonne nome/tipo/dimensione/modifica, nessuna ricerca o AI attiva.
- Passare tra elenco e griglia; mostrare/nascondere la sidebar.
- Selezionare un file senza aprire automaticamente altri pannelli. L'icona **Anteprima e informazioni** apre o chiude il pannello laterale.
- In un pannello già aperto, la selezione aggiorna documento illustrativo, nota e tag. Frecce su/giù navigano l'elenco.
- Le schede sono in alto: Apollo torna alla cartella; Documenti, «+» e i comandi di creazione/gestione mostrano spiegazioni illustrative, senza operazioni reali.
- Il pulsante luna/sole cambia tema; la scelta è ricordata dal browser se lo storage locale è disponibile.

### Ricerca a scomparsa

1. Cliccare la lente oppure premere **Cmd/Ctrl + K**: il campo si espande e riceve il focus.
2. Provare `.pdf` in modalità Semplice: il filtro usa nomi e metadati fittizi. Il pannello AI non si apre.
3. Il mini menu permette di scegliere le quattro modalità. Frecce, Home/End ed Esc navigano il menu; **Alt + Freccia giù** lo apre dal campo.
4. Semantica mostra risultati illustrativi senza sintesi. Auto usa poche regole locali, non un modello.
5. Per l'esempio agentico, scegliere **Agentica** e scrivere «Confronta le condizioni di rinnovo», poi premere Invio. Solo dopo l'attesa simulata compare il confronto con fonti.
6. In alternativa, **Rinnovi Apollo** nella sidebar carica e avvia esplicitamente lo stesso esempio in Auto.
7. Le citazioni selezionano la fonte e ne mostrano l'anteprima nel pannello contestuale; «Risposta» torna al confronto.
8. I filtri includono/escludono formati. Il confronto fisso non viene mostrato se mancano le fonti necessarie.
9. **Esc** chiude prima un eventuale menu, poi la ricerca. La «×» o un secondo clic sulla lente chiudono direttamente la ricerca, svuotando la query. Un campo vuoto si richiude anche quando perde il focus.
10. «Salva ricerca» conserva query, modalità e filtri soltanto nella memoria della pagina; si azzera ricaricandola.

La risposta AI è un esempio fisso, non cambia realmente per una domanda arbitraria. Il pannello aperto automaticamente dall'agente si richiude uscendo dalla ricerca; un'anteprima aperta esplicitamente dall'utente può invece rimanere visibile.

## Caratterizzazione delle piattaforme

### macOS

Interpretazione ispirata a macOS/glass, non replica ufficiale di macOS 27. Le trasparenze riguardano principalmente il chrome; i menu mantengono una superficie leggibile. È previsto un fallback CSS per la riduzione delle trasparenze.

### Windows 11 / WinUI 3

Interpretazione HTML del linguaggio WinUI/Fluent, non controlli WinUI/XAML nativi o Mica di sistema. Il font usa Segoe UI Variable / Segoe UI dove installato, con fallback locale.

- Schede accanto all'icona dell'app e ai controlli finestra; toolbar compatta sotto, senza command bar aggiuntiva.
- Massimizza/Ripristina ridimensiona soltanto il mockup nella pagina. Riduci a icona e Chiudi non agiscono sulle finestre reali.
- Taskbar, disco, meteo e orologio sono illustrativi. «Cerca» nella taskbar espande la ricerca di FolderAI; l'icona dell'app riporta il focus alla scheda, senza attivare una ricerca.
- CSS per colori forzati e riduzione delle trasparenze, da verificare anche in un ambiente Windows reale.

## Limiti

- Nessun accesso al filesystem, chiamata a provider/modelli o caricamento di risorse esterne.
- Documenti, anteprime, date, capacità e risultati sono fittizi.
- Nessuna copia, eliminazione, estrazione, autenticazione o modifica reale di note/file.
- Non è un'implementazione Tauri/React e non attesta conformità di accessibilità già verificata.
- Le proposte visive non sostituiscono il [perimetro analizzato](../../analysis/04-analysis-summary.md).
- Non vengono ripristinate le precedenti pagine HTML rimosse dal repository.

Asset condivisi: `integrated.css`, `integrated.js`, `explorer-first.css`. macOS aggiunge `macos.css`; Windows aggiunge `winui.css` e `winui.js`.
