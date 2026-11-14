# FolderAI — Riepilogo della prima analisi

**Stato:** perimetro funzionale del file explorer confermato dall'utente (FE-DEC-061). Riepilogo delle decisioni, non ancora specifica formale di requisiti né piano di implementazione.

## 1. Visione e vincoli condivisi

FolderAI è una vera alternativa ai file manager di sistema, con Finder ed Esplora file come riferimenti. Deve essere comodo per utenti generalisti e potente/personalizzabile per professionisti e sviluppatori.

- Applicazione desktop multipiattaforma, con differenze di sistema gestite tramite astrazioni.
- Stack scelto: **Tauri 2, Rust, React, TypeScript e SCSS**.
- Valore distintivo: ricerca avanzata nei contenuti di archivi ampi e integrazione AI contestuale.
- Documenti originali sul filesystem; metadati applicativi e indici hanno responsabilità separate.
- Database vettoriale locale confermato. Database SQL locale proposto per i dati strutturati; motori e schema non ancora scelti. SQLite è un candidato, non una decisione acquisita.

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
- Apertura con applicazione associata o «Apri con».
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
- Ricerche salvate conservano richiesta, criteri e ambito, non risultati statici; accessibili dalla sidebar, rinominabili ed eliminabili, ricalcolate all'apertura o all'aggiornamento.
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

## 5. AI: indirizzo separato, non riaperto

L'analisi AI rimane in `03-ai-integration.md`:

- Ricerca semplice, semantica e agentica selezionabili manualmente, insieme ad Auto.
- Assistente contestuale, ricerca iterativa con fonti e agenti iniziali in sola lettura.
- Modelli locali e remoti; funzionamento AI completamente locale/offline con componenti adeguati.
- API OpenAI/Anthropic e provider terzi; integrazioni con subscription richieste ma da verificare per disponibilità e modalità autorizzate.
- Consenso configurabile per l'invio remoto.
- Specializzazione dei modelli per ruolo ancora da dettagliare; non ogni ruolo o profilo proposto è automaticamente approvato.
- Punti confermati AI-ATT-001–004: risorse multi-modello, compatibilità embedding/indice, affidabilità del routing e nessun fallback cloud silenzioso.

La conferma del perimetro del file explorer **non assegna automaticamente ogni proposta AI al primo rilascio**.

## 6. Punti aperti da mantenere espliciti

| Area | Da definire prima delle relative scelte o verifiche |
| --- | --- |
| Applicazioni predefinite | Preferenze interne a FolderAI, associazioni di sistema o entrambe; decisione esplicitamente rinviata |
| Formati | Matrice dei contenuti ricercabili/indicizzabili e varianti precise delle anteprime, senza confonderle |
| Piattaforme | Sistemi, versioni, architetture e filesystem supportati; capacità e limiti delle astrazioni |
| Identità e metadati | Riconciliazione esterna, casi ambigui, concorrenza, salvataggi pendenti e operazioni parziali |
| Preferiti non disponibili | Coordinare riconciliazione e rimozione automatica per non rimuovere prematuramente collegamenti ancora riconoscibili |
| Operazioni sicure | Errori globali, sostituzione finale, crash, concorrenza, recuperabilità, link ed estrazione ZIP |
| Sessioni e finestre | Ripristino multi-finestra, posizioni indisponibili, uscita dall'intera app e attività diverse dai trasferimenti |
| Persistenza | Motori SQL/vettoriali, migrazioni, protezione dei dati, invalidazione e conservazione delle cache |
| Qualità | Hardware e archivi campione, soglie prestazionali, prove multipiattaforma e accessibilità |
| AI | Fattibilità subscription, capacità dei modelli, ruoli, formati e granularità del consenso nelle attività multi-chiamata |

I dettagli puntuali sono conservati nel documento 2 e nel documento AI. I punti aperti non sono stati risolti tramite assunzioni implicite.

## 7. Passaggio ai requisiti

La raccolta iniziale delle decisioni del file explorer è conclusa. Il prossimo passo è tradurre il perimetro confermato in requisiti numerati e verificabili in `requirements/`, collegandoli alle decisioni FE-DEC e ai punti AI-ATT quando pertinenti.

Per ogni requisito: descrizione, ambito di rilascio, fonte della decisione e criterio di accettazione. Dove manca una decisione o una soglia, indicare esplicitamente il punto aperto invece di inventare un valore. Le capacità AI con perimetro ancora non definito vanno mantenute distinte.

Il primo rilascio è un obiettivo funzionale, non l'impegno a sviluppare tutto insieme. Ordine di implementazione e architettura saranno definiti successivamente.
