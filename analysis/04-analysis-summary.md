# FolderAI — Riepilogo della prima analisi

**Stato:** perimetro funzionale del file explorer confermato dall'utente (FE-DEC-061). Riepilogo delle decisioni, non ancora specifica formale di requisiti né piano di implementazione.

## 1. Visione e vincoli condivisi

FolderAI è una vera alternativa ai file manager di sistema, con Finder ed Esplora file come riferimenti. Deve essere comodo per utenti generalisti e potente/personalizzabile per professionisti e sviluppatori.

- Applicazione desktop multipiattaforma, con differenze di sistema gestite tramite astrazioni.
- Matrice iniziale confermata: **Windows 11 x64 e macOS Apple Silicon**. Fuori dal primo rilascio macOS Intel, Linux, Windows ARM64 e Windows 10/precedenti. Versione minima di macOS da fissare dopo verifica di Tauri e dipendenze; compatibilità dei singoli engine/modelli locali distinta dal supporto dell'app.
- Stack scelto: **Tauri 2, Rust, React, TypeScript e SCSS**.
- Valore distintivo: ricerca avanzata nei contenuti di archivi ampi e integrazione AI contestuale.
- Documenti originali sul filesystem; metadati applicativi e indici hanno responsabilità separate.
- Database vettoriale locale confermato. Database SQL locale proposto per i dati strutturati; motori e schema non ancora scelti. SQLite è un candidato, non una decisione acquisita.
- Engine di embedding astratto sin dalla prima implementazione, con identità/versione dei modelli e indici dedicati a spazi compatibili, senza confronti tra modelli differenti: AI-DEC-005 e `requirements/embedding.md` (EMB-001–005).

## 2. File explorer: primo rilascio confermato

### Navigazione e finestre

- Pannello singolo iniziale e doppio pannello opzionale, con schede e navigazione indipendenti per pannello.
- Tornando al pannello singolo, tutte le schede si riuniscono in una sola barra, senza gruppi nascosti.
- Nuovo pannello e nuova scheda tramite «+» partono dalla cartella iniziale configurata.
- Schede riordinabili e trasferibili tra pannelli tramite trascinamento, senza duplicazione.
- Un pannello vuoto scompare; chiudere l'ultima scheda del pannello singolo chiude la finestra.
- Finestre multiple indipendenti. «Nuova finestra» apre un pannello singolo con una scheda nella cartella iniziale.
- Sidebar configurabile e percorso a breadcrumb modificabile come testo.
- Avvio configurabile: cartella iniziale oppure ripristino dell'ambiente precedente. Default: cartella iniziale, che inizialmente è la Home.
- Cartella iniziale personalizzabile; se non disponibile, Home e avviso, senza cancellare la preferenza.

**Tracciabilità:** FE-DEC-015–024, FE-DEC-026, FE-DEC-030; base confermata nel documento 2.

### Operazioni, sicurezza e recupero

- Creazione di file vuoti e cartelle, copia, spostamento, rinomina singola ed eliminazione; selezioni multiple, appunti e drag-and-drop.
- Apertura con applicazione associata o «Apri con». Il cambio dell'applicazione predefinita vale solo in FolderAI.
- Centro attività separato per finestra. Chiudendo una finestra con trasferimenti attivi si sceglie se lasciarla aperta oppure interrompere in sicurezza i suoi trasferimenti; le altre finestre non sono coinvolte.
- Su errore relativo a un singolo file: riprova, salta o interrompi. La scelta per errori simili vale solo nell'operazione corrente. In attesa, si sospende quella operazione, non tutta l'app.
- «Annulla» interrompe anche il trasferimento corrente appena sicuro; rimuove la nuova copia parziale e segnala eventuali errori di pulizia. Quanto già completato rimane tale.
- Nei conflitti di nome si mantengono entrambi automaticamente: `Documenti - copy`, poi `Documenti - copy (2)`; per i file il suffisso precede l'estensione.
- Sostituzioni esplicite soltanto con preparazione sicura della nuova copia; se manca lo spazio aggiuntivo, non si procede alla sovrascrittura diretta.
- Eliminazione ordinaria nel cestino senza conferma, con notifica. Se il cestino non è disponibile, arresto e segnalazione, senza proposta di eliminazione permanente nello stesso flusso.
- Eliminazione permanente separata, esplicita e confermata.
- «Annulla ultima azione»: una sola azione per finestra, quando recuperabile; nessuna persistenza tra sessioni e nessuna promessa di ripristino universale.

**Tracciabilità:** FE-DEC-001–013, FE-DEC-025, FE-DEC-027–029.

### Preferiti, tag e note

- Preferiti per file e cartelle; gruppi facoltativi a un solo livello, senza sottogruppi, organizzabili tramite trascinamento.
- Uno stesso elemento può comparire in più gruppi tramite collegamenti indipendenti.
- Eliminare un gruppo riporta i collegamenti nella lista principale. I preferiti non più raggiungibili vengono rimossi automaticamente; questo non cancella note o tag.
- Tag multipli per file e cartelle, riutilizzabili in tutta l'app, con colore facoltativo, senza gerarchie e senza ereditarietà ai contenuti. Gestione anche su selezioni multiple.
- Note per file e cartelle, in Markdown con anteprima, salvataggio automatico e stato/errori visibili; ricercabili testualmente con indicazione della provenienza della corrispondenza.
- Copia di note e tag configurabile, attiva di default; i metadati copiati diventano indipendenti.
- Rinomine/spostamenti tramite FolderAI conservano note, tag e preferenze delle cartelle, aggiornando tutti i relativi preferiti.
- Per modifiche esterne si tenta la riconciliazione solo quando l'identità dell'elemento è riconosciuta affidabilmente, senza associazioni basate sulla sola somiglianza del nome.

**Tracciabilità:** FE-DEC-031–044.

### Viste e anteprime

- Dettagli, griglia e colonne stile Finder; pannello laterale opzionale per anteprima e informazioni.
- Anteprime in sola lettura: testo/codice, Markdown, immagini comuni e PDF. Note e tag rimangono modificabili come metadati.
- Miniature per gli elementi visibili, generate quando servono e conservate in cache locale; nessun cloud necessario.
- Con selezioni multiple: riepilogo di numero, tipi, dimensioni disponibili e tag comuni/differenti; gestione dei tag senza sovrascrivere le note individuali.
- Preferenze ricordate per cartella. Default: Dettagli; colonne nome, tipo, dimensione e ultima modifica; cartelle prima, nome crescente con ordinamento naturale, nessun raggruppamento.
- Colonne configurabili, riordinabili e ridimensionabili; ulteriori colonne attivabili.
- Elementi nascosti non visibili di default, con comando rapido e preferenza persistente. Estensioni visibili di default e configurabili.
- Dimensioni complessive delle cartelle calcolate solo su richiesta.

**Tracciabilità:** FE-DEC-045–054; base confermata nel documento 2.

### Ricerca, filesystem ed esperienza

- Ricerca semplice per nome, percorso, tipo/estensione, dimensioni/date disponibili, tag, note e testo nei formati supportati; nessun LLM o indice vettoriale obbligatorio.
- Ambito selezionabile, sottocartelle facoltative, filtri AND/OR ed esclusioni tramite UI; stato della ricerca ed elementi non analizzabili visibili.
- Le ricerche semplici salvate conservano richiesta, criteri e ambito, senza salvare un elenco statico di risultati. Sono accessibili dalla sidebar, rinominabili ed eliminabili, con risultati ricalcolati all'apertura o all'aggiornamento.
- Collegamenti simbolici distinguibili e navigabili, conservati come link nelle copie dove supportato; niente ricorsione incontrollata.
- Rispetto dei permessi, accessi negati espliciti, nessuna richiesta automatica di privilegi amministrativi.
- ZIP: consultazione ed estrazione sicura nella destinazione scelta, senza scritture fuori ambito o sovrascritture implicite.
- Scorciatoie native personalizzabili, con gestione dei conflitti e ripristino.
- Tema chiaro/scuro/sistema, italiano e inglese, preferenze persistenti, adattamento alla scala e alle dimensioni della finestra.
- Tastiera, focus visibile, compatibilità con screen reader, informazioni non basate sul solo colore e riduzione delle animazioni secondo il sistema; da validare esplicitamente.

**Tracciabilità:** FE-DEC-055–058.

## 3. Scala prevista

- Anche **centinaia di migliaia di file complessivi**, distribuiti tra cartelle.
- Decine di migliaia di figli diretti in una cartella non sono lo scenario ordinario previsto; nessun limite rigido approvato.
- **16 GB di RAM** come riferimento orientativo, non minimo obbligatorio.
- Supporto a **8 GB desiderato**, senza farne un vincolo che riduca funzioni o qualità.
- Risorse dei modelli AI da valutare separatamente. Nessuna soglia di latenza o consumo già concordata o verificata.

**Tracciabilità:** FE-DEC-060.

## 4. Funzionalità rimandate

- Dischi esterni e cartelle di rete.
- Rinomina in blocco.
- Editor integrato dei documenti.
- Anteprime Office/OpenDocument, senza implicare esclusione di tali formati dalla futura ricerca/indicizzazione.
- Creazione/modifica di archivi e formati compressi diversi da ZIP.
- Esportazione/importazione dei dati applicativi e gestione backup dedicata all'utente.
- Azioni agentiche di creazione/modifica dei file, già rimandate nell'analisi AI.

I rinvii non equivalgono a rinunciare a protezione dei dati o sicurezza delle migrazioni, i cui dettagli restano da progettare. Riferimenti: FE-DEC-014, FE-DEC-049–050, FE-DEC-056, FE-DEC-059 e indirizzi iniziali.

## 5. AI: perimetro iniziale confermato separatamente

L'analisi AI rimane in `03-ai-integration.md`; il perimetro iniziale è stato successivamente confermato dall'utente in AI-DEC-001:

- Ricerca semplice, semantica e agentica selezionabili manualmente, insieme ad Auto.
- Capacità AI incluse: ricerca semantica, ricerca agentica, domande sui contenuti, riassunto, confronto ed estrazione mirata anche in tabelle nella risposta.
- Aprire una ricerca agentica salvata ripristina richiesta, ambito e filtri. Una nuova esecuzione parte con «Esegui» e segue la politica di consenso remoto configurata.
- Strumenti di esplorazione autorizzata, ricerca e lettura dei documenti supportati; agenti iniziali in sola lettura, senza terminale o navigazione web. Il terminale manuale resta distinto.
- Rimandati suggerimenti automatici di tag, rilevamento proattivo di duplicati/versioni e analisi periodiche senza richiesta esplicita. Input solo testuale iniziale; immagini in input in fase due, senza decidere implicitamente la copertura OCR.
- Modelli locali e remoti; funzionamento AI completamente locale/offline con componenti adeguati.
- API OpenAI/Anthropic e provider terzi; integrazioni con subscription richieste ma da verificare per disponibilità e modalità autorizzate.
- Consenso remoto confermato in AI-DEC-007: Solo locale iniziale, Chiedi a ogni richiesta utente (incluse le chiamate necessarie) o Consenti ai provider selezionati. Autorizzazione esplicita per provider/ruolo, ambito e categorie di dati; nuova approvazione per variazioni non coperte. Credenziali nell'archivio protetto del sistema.
- Gestione delle sorgenti confermata in AI-DEC-004: cartelle esplicite e ricorsive con esclusioni, indicizzazione in background sospendibile, aggiornamenti incrementali, copertura visibile e rimozione sicura dei derivati senza toccare originali o metadati utente.
- Esclusioni iniziali confermate in AI-DEC-008: sistema/app, dipendenze e generati, metadati repository, contenuti sensibili riconoscibili, nascosti e dati interni di FolderAI. Regole modificabili e vincolanti anche per letture AI dirette, senza nascondere gli elementi nell'explorer.
- Cronologia persistente delle conversazioni esclusa in AI-DEC-009, sostituendo la precedente scelta della cronologia locale riapribile. Solo contesto temporaneo, senza salvataggio dei messaggi su disco: resta nascondendo il pannello o cambiando scheda, si elimina con Nuova conversazione o chiudendo la scheda. L'utente accetta la perdita alla chiusura.
- Configurazione AI confermata in AI-DEC-006: nessun modello obbligatorio all'avvio, connessioni locali/API/subscription distinte dai modelli, configurazione guidata e avanzata, ruoli Embedding/Assistente-agente/Comprensione visiva con riuso se compatibile, stato e destinazione visibili. Ruoli ulteriori e profili proposti non sono automaticamente approvati.
- Punti confermati AI-ATT-001–004: risorse multi-modello, compatibilità embedding/indice, affidabilità del routing e nessun fallback cloud silenzioso.

La conferma del file explorer e del mockup **non assegna automaticamente ogni proposta AI al primo rilascio**: valgono le capacità e i rinvii esplicitamente confermati in AI-DEC-001. Autonomia controllata confermata in AI-DEC-002: letture senza conferme ripetute nell'ambito autorizzato, chiarimenti per ambiguità sostanziali, risultati qualificati, limiti finiti e interruzione esplicita. Auto confermato in AI-DEC-003: approccio adeguato meno complesso, modalità effettiva visibile e nessuna estensione di autorizzazioni. Gestione concreta e valori dei limiti restano rinviati; varianti e qualità dei formati, dettagli del consenso per indicizzazione continua e revoca richiedono approfondimento. Il consenso per singola richiesta utente è confermato in AI-DEC-007; routing tecnico e sua affidabilità richiedono progettazione e verifica.

## 6. Punti aperti da mantenere espliciti

| Area | Da definire prima delle relative scelte o verifiche |
| --- | --- |
| Applicazioni predefinite | Criterio di associazione e gestione delle applicazioni indisponibili. L'ambito della preferenza è confermato e vale solo in FolderAI. |
| Formati | Copertura iniziale per famiglie e comprensione visiva oltre OCR confermate in `06-content-formats.md`, anche per immagini/grafici in PDF e Office/OpenDocument; restano varianti precise, limiti e qualità, distinguendo anteprima e indicizzazione |
| Piattaforme | Windows 11 x64 e macOS Apple Silicon confermati per il primo rilascio; restano versione minima macOS, filesystem supportati, compatibilità delle dipendenze e capacità/limiti delle astrazioni |
| Identità e metadati | Riconciliazione esterna, casi ambigui, concorrenza, salvataggi pendenti e operazioni parziali |
| Preferiti non disponibili | Coordinare riconciliazione e rimozione automatica per non rimuovere prematuramente collegamenti ancora riconoscibili |
| Operazioni sicure | Errori globali, sostituzione finale, crash, concorrenza, recuperabilità, link ed estrazione ZIP |
| Sessioni e finestre | Chiusura con AI attiva e arresto/ripresa dell'indicizzazione confermati in AI-DEC-010, senza servizi autonomi dopo uscita; restano ripristino multi-finestra, posizioni indisponibili e coordinamento tecnico dell'uscita |
| Indicizzazione | Sorgenti confermate in AI-DEC-004 ed esclusioni iniziali in AI-DEC-008; autorizzazione temporanea per singola attività esclusa. Restano sorgenti abilitate non ancora indicizzate, precedenza regole, sovrapposizioni, invalidazione e meccanismi di ripresa (comportamento generale confermato in AI-DEC-010) |
| Persistenza | Motori SQL/vettoriali, migrazioni, protezione dei dati, invalidazione e conservazione delle cache |
| Qualità | Hardware e archivi campione, soglie prestazionali, prove multipiattaforma e accessibilità |
| AI | Configurazione e tre ruoli iniziali confermati in AI-DEC-006, privacy/consenso in AI-DEC-007; restano fattibilità subscription, verifica capacità, adattatori concreti, consenso per indicizzazione continua e revoca |

I dettagli puntuali sono conservati nel documento 2 e nel documento AI. I punti aperti non sono stati risolti tramite assunzioni implicite.

## 7. Passaggio ai requisiti

La raccolta iniziale delle decisioni del file explorer è conclusa. Prima dei requisiti, su richiesta dell'utente, è stata approfondita anche la UI: le decisioni sono raccolte in `05-ui-design.md`. Il mockup unificato «Workspace sobrio» (`docs/mockups/folderai-unified.html`) e la relativa direzione visiva sono stati validati dall'utente; ciò non equivale a validazione delle funzionalità non implementate nella demo.

Il perimetro confermato è formalizzato nella [specifica del primo rilascio](../requirements/README.md). I requisiti numerati riportano criteri di accettazione e riferimenti alle decisioni di analisi. La [specifica dell'engine di embedding](../requirements/embedding.md) conserva gli identificativi EMB-001–005 collegati ad AI-DEC-005 e AI-ATT-002.

Il [registro delle decisioni](../requirements/open-decisions.md) identifica le scelte di progettazione e le soglie ancora da definire, con il lavoro che ne dipende. La specifica include una sequenza proposta per avviare l'implementazione.
