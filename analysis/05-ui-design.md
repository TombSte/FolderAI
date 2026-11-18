# FolderAI — Analisi UI e direzione visiva

**Stato:** decisioni raccolte nel confronto con l'utente; mockup unificato «Workspace sobrio» validato dall'utente con «Okay mockup validato, mi piace».

Questo documento integra l'analisi funzionale, senza sostituire le decisioni FE-DEC o costituire già una specifica formale di requisiti. L'approvazione riguarda il mockup e la direzione visiva: non attesta implementazione, accessibilità, prestazioni o validazione delle funzionalità mancanti nella demo.

## 1. Riferimento corrente

- Mockup approvato: `docs/mockups/folderai-unified.html`.
- Asset: `docs/mockups/unified.css`, `docs/mockups/unified.js`.
- Anteprima: `docs/mockups/unified-preview.png`.
- Funzioni dimostrative e limiti: `docs/mockups/README.md`.
- Le precedenti interpretazioni macOS/WinUI restano materiale storico, non due versioni da sviluppare.
- L'utente ha chiesto di aggiornare i mockup alla fine dell'analisi, non a ogni decisione. Questo studio è stato realizzato successivamente su sua richiesta esplicita per valutare lo stile.

## 2. Struttura e sidebar — confermate

- UI unificata su macOS e Windows: stesso stile, componenti e disposizione. Restano specifici del sistema controlli finestra, scorciatoie e integrazioni native.
- Schede in alto; toolbar compatta con navigazione, percorso, ricerca e viste; sidebar sinistra; area file centrale; pannello destro opzionale; barra di stato in basso.
- Nessun profilo utente nella sidebar e nessuna registrazione necessaria per usare FolderAI. Eventuali autenticazioni AI appartengono alle integrazioni.
- Sidebar con Posizioni, Preferiti, Tag e Ricerche salvate; sezioni richiudibili, riordinabili e nascondibili. Sidebar ridimensionabile e nascondibile; Impostazioni in basso.
- Cartelle espandibili ad albero tramite freccia; clic sul nome per aprirle. Albero inizialmente compatto, senza espansioni profonde automatiche.
- Aggiungere un preferito non autorizza l'indicizzazione o la lettura AI.
- Pulsanti delle operazioni sui file nascosti di default; impostazioni per scegliere i singoli comandi e riordinarli. Menu contestuali e scorciatoie restano disponibili.

## 3. Navigazione e area file — confermate

- Breadcrumb cliccabile e modificabile come testo tramite clic nello spazio libero o scorciatoia. Invio apre il percorso, Esc abbandona la modifica. Errori vicini al campo, mantenendo la posizione precedente.
- Indietro, Avanti e Cartella superiore nella toolbar; cronologia per scheda. Al ritorno, recupero di selezione e scorrimento quando possibile.
- Titolo della scheda con nome della cartella e percorso completo al passaggio del mouse. Menu con Duplica, Chiudi, Chiudi le altre e Chiudi a destra.
- «Apri in nuova scheda» apre e attiva la nuova scheda; cambiare scheda ne conserva stato, selezione, scorrimento e ricerca. Restano valide le precedenti decisioni su «+», trasferimento delle schede e chiusura dell'ultima scheda.
- Posizione diventata indisponibile: stato esplicito con azioni per riprovare o raggiungere la cartella superiore, senza navigazione alternativa silenziosa.
- Viste dettagli, griglia e colonne; densità iniziale equilibrata e modalità Compatta. Dimensione di icone e miniature regolabile in griglia.
- Nella vista a colonne selezionare una cartella apre la colonna successiva, non il pannello informazioni.
- Clic singolo seleziona, doppio clic apre; selezione multipla e tastiera secondo convenzioni della piattaforma. Selezione con trascinamento da un'area vuota.
- Selezionare un file non apre automaticamente anteprima o AI.
- Clic destro su un elemento già selezionato mantiene la selezione multipla; su un elemento non selezionato seleziona quello. Menu dello spazio vuoto riferito alla cartella corrente.
- Durante il trascinamento, destinazione ed effetto riconoscibili prima del rilascio. Nel doppio pannello, comandi riferiti alla selezione del pannello attivo, evidenziato discretamente.
- Aggiornamenti esterni senza rubare focus o azzerare la selezione degli elementi ancora presenti.

## 4. Ricerca — confermata con input agentico dedicato

- Ricerca inizialmente ridotta alla lente; espansione nella toolbar tramite clic o scorciatoia. Chiusura con ritorno alla cartella e recupero della selezione e dello scorrimento.
- Modalità Semplice, Semantica, Agentica e Auto; Semplice iniziale nelle nuove schede, poi scelta mantenuta per scheda. Auto indica l'approccio usato; nessun cambio silenzioso di modalità o fallback cloud.
- Ambito iniziale cartella corrente e sottocartelle, modificabile. Filtri attivi rimovibili; condizioni AND/OR ed esclusioni in un pannello espandibile.
- Semplice aggiornata durante la digitazione; Semantica, Agentica e Auto avviate esplicitamente, non a ogni carattere.
- Risultati nell'area centrale con percorso, motivo della corrispondenza e stato/copertura della ricerca. Ricerca semantica con distinzione dei contenuti non indicizzati.
- Le ricerche salvate conservano nome, richiesta, modalità, ambito e filtri. L'apertura ripristina la configurazione e riesegue la ricerca, salvo in modalità Agentica.
- Una ricerca agentica salvata richiede «Esegui» per avviare una nuova analisi. La sola apertura permette di rivedere la richiesta, l'ambito e i filtri. Si applica la politica di consenso remoto configurata.
- In Agentica la toolbar non deve contenere tutta la richiesta: si apre il pannello conversazionale destro con input multilinea. Anche Auto permette di aprire l'input esteso prima dell'invio.

## 5. Assistente e ambito — confermati

- **Prima fase: solo input testuale**, senza allegati o trascinamento di file nell'input. Ricerca tramite immagini rimandata alla fase due per Semantica/Agentica; questo rinvio non esclude le anteprime delle immagini nell'explorer.
- Input multilinea espandibile; Invio va a capo, pulsante esplicito o scorciatoia dedicata inviano.
- Vicino all'input: **Globale = tutte le cartelle esplicitamente abilitate alla ricerca AI**, oppure **Cartella e sottocartelle**, con percorso visibile e modificabile.
- Ambito iniziale derivato dalla cartella di avvio. La navigazione successiva non lo modifica automaticamente. Cambiarlo non avvia una ricerca e vale dalla richiesta successiva; ogni richiesta conserva visibile il proprio ambito.
- Conversazione corrente per scheda; Nuova conversazione riparte senza il contesto precedente. La precedente cronologia locale riapribile è stata esclusa in AI-DEC-009 (`03-ai-integration.md`): nessun archivio persistente, salvataggio su disco dei messaggi o ripristino delle conversazioni precedenti. Il contesto temporaneo rimane cambiando scheda senza chiuderla, passando a Dettagli o nascondendo il pannello; viene eliminato con Nuova conversazione o chiudendo la scheda. L'utente accetta esplicitamente la perdita alla chiusura.
- Indicazione della configurazione locale/remota; attività sintetiche comprensibili, senza log tecnici invasivi.
- Interruzione esplicita con risultati parziali identificati. Nascondere il pannello non interrompe l'attività; indicatore per ritrovarla.
- Risposte con fonti cliccabili e distinzione tra informazioni supportate e punti non verificabili.

## 6. Unico pannello destro — confermato

- Pannello ridimensionabile e inizialmente chiuso, con selettore **Dettagli / Assistente**; non due pannelli affiancati.
- Il cambio conserva contenuti, scorrimento e bozza; attività AI non interrotte.
- Dettagli: anteprima sopra, proprietà/tag/note in sezioni richiudibili sotto. Originali in sola lettura; note Markdown e tag modificabili, stato del salvataggio visibile.
- Selezione multipla: riepilogo e tag senza sovrascrivere note individuali. Nel doppio pannello, Dettagli segue la selezione del pannello file attivo, non cambia l'ambito AI.
- Citazione AI: passaggio a Dettagli con fonte/passaggio quando supportato e comando «Torna alla conversazione». Non sostituisce i risultati centrali e non cambia l'ambito; apertura esterna e visualizzazione nella cartella sono azioni separate.

## 7. Attività e feedback — confermati

- Barra di stato con numero di elementi, riepilogo selezione e accesso al centro attività della finestra.
- Centro attività come pannello sovrapposto temporaneo; stato, oggetto, progresso e azioni pertinenti. Percentuali solo se attendibili, altrimenti fase ed elementi elaborati.
- Interrompi attività e Annulla ultima azione distinti. Errori con scelta utente marcati «Richiede attenzione»; si sospende solo l'operazione interessata.
- Riepiloghi con completati, saltati e non riusciti; dettagli tecnici su richiesta.
- Notifiche brevi, non bloccanti e senza sottrarre focus. Errori consultabili anche dopo la scomparsa della notifica; nessuna notifica per ogni file.
- Conferme modali per decisioni esplicite come eliminazione permanente e chiusura con trasferimenti attivi, nel rispetto delle decisioni FE-DEC.
- Avanzamento AI nella conversazione, richiamabile dal centro attività della finestra. Stato dell'indicizzazione nelle impostazioni delle sorgenti, distinto dai trasferimenti della singola finestra.

## 8. Impostazioni e stile — confermati

- Sezioni impostazioni: Generali, Aspetto, Navigazione e viste, Toolbar e sidebar, Ricerca e indicizzazione, AI e provider, Scorciatoie.
- Applicazione immediata quando possibile, eccezioni esplicite e ripristino per sezione.
- Stile approvato «Workspace sobrio»: file protagonisti, superfici opache e neutre, accento blu comune anche all'AI, bordi sottili, ombre contenute per elementi sovrapposti, angoli moderati, icone lineari e densità desktop equilibrata.
- Temi chiaro/scuro della stessa identità, non skin separate macOS/WinUI. Nessun glassmorphism o gradiente decorativo.
- Tipografia sans-serif neutra: Inter proposta; il mockup validato usa font locali di sistema e non include Inter. Non considerare già approvati asset tipografici non presenti nella demo.

## 9. Interoperabilità esterna — comportamenti confermati

L'utente conferma come base iniziale:

- Trasferimento bidirezionale di file e cartelle reali con Desktop/Scrivania, Finder/Esplora file e applicazioni compatibili.
- Copia/taglia/incolla tramite appunti di sistema, in interoperabilità con il file manager del sistema operativo.
- File virtuali, allegati non ancora salvati e contenuti provenienti dal browser da valutare separatamente; non considerarli automaticamente equivalenti a file già presenti sul disco.

L'utente ha confermato anche i seguenti comportamenti:

- Copia/spostamento e modificatori da tastiera secondo le convenzioni del sistema operativo; effetto e destinazione riconoscibili prima del rilascio.
- Rilascio nello spazio vuoto verso la cartella visualizzata; rilascio sopra una cartella evidenziata verso quella cartella.
- Apertura della cartella sotto il puntatore dopo una breve attesa per raggiungere destinazioni annidate; intervallo numerico ancora da definire.
- Appunti interoperabili in entrambe le direzioni. Il taglio evidenzia gli elementi, ma non li rimuove prima del trasferimento riuscito.
- Verso altre applicazioni, riferimenti ai file reali utilizzabili dalle applicazioni compatibili. Allegare o aprire un documento non equivale automaticamente a spostarlo.
- Rilascio non consentito segnalato esplicitamente, senza modificare gli originali.
- Se il trasferimento è eseguito da FolderAI valgono le regole confermate su conflitti, errori e copie parziali. Se è eseguito da un'altra applicazione non si promette di controllarne gestione errori o annullamento.

Le capacità specifiche per piattaforma richiedono verifica tecnica, non sono già implementate dal mockup. L'interoperabilità non introduce allegati nell'input AI: la prima fase resta solo testuale.

**Apertura nel terminale — confermata dall'utente:**

- «Apri nel terminale» nel menu contestuale di una cartella: nuova sessione con quella cartella come directory di lavoro.
- Stessa voce sullo spazio vuoto, riferita alla cartella corrente.
- Terminale predefinito del sistema inizialmente, con possibilità di scegliere un'app diversa nelle impostazioni; identificazione e avvio da verificare per piattaforma.
- Nessuna elevazione automatica dei privilegi.
- È un comando manuale dell'utente, non un'autorizzazione a eseguire comandi per gli agenti AI.

## 10. Menu contestuali dell'area file — catalogo confermato

L'utente ha confermato il catalogo proposto per file, cartelle, selezioni multiple, spazio vuoto e risultati di ricerca, con l'aggiunta del terminale.

### File singolo

Ordine e gruppi confermati:

1. Apri; Apri con… (applicazioni disponibili); Anteprima e informazioni (pannello Dettagli).
2. Taglia; Copia; Copia percorso; Rinomina.
3. Aggiungi ai preferiti (scelta del gruppo, rimozione quando già presente); Tag… (assegnazione/rimozione); Modifica nota (sezione del pannello).
4. Sposta nel cestino; Elimina definitivamente… (azione distinta e sempre confermata).

Il cambio dell'applicazione predefinita vale solo in FolderAI. Il flusso per impostare la preferenza e il criterio di associazione restano da definire nella sezione 8 del documento 2. Per gli originali con più collegamenti nei preferiti, il successivo blocco sidebar conferma appartenenze esplicite nel menu dell'area file, evitando rimozioni ambigue.

### Varianti

- **Cartella singola:** Apri, Apri in nuova scheda, Apri in nuova finestra; niente Apri con. Incolla nella cartella, Calcola dimensione e Apri nel terminale; restano operazioni, preferiti, tag e note.
- **Selezione multipla:** Copia, Taglia, Copia percorsi, Preferiti, Tag, Informazioni e cancellazione. Niente rinomina in blocco o modifica collettiva delle note.
- **Spazio vuoto:** Nuovo file, Nuova cartella, Incolla, Seleziona tutto, Vista, Ordina per, Raggruppa per, Mostra elementi nascosti, Informazioni sulla cartella e Apri nel terminale.
- **Risultato di ricerca:** comandi pertinenti all'elemento reale, più Mostra nella cartella. Nessuna modifica implicita ai criteri della ricerca.

### Regole comuni

- Scorciatoie mostrate accanto ai comandi.
- Azioni incompatibili con il contesto omesse; quelle temporaneamente indisponibili disabilitate.
- Nessuna integrazione automatica con estensioni dei menu di Finder/Esplora file: capacità separata da valutare.
- Eventuali voci AI da definire solo dopo averne confermato il perimetro.

## 11. Menu di sidebar e schede — base attuale accettata

L'utente accetta queste voci con «Per ora vanno bene queste»: costituiscono la base corrente, rivedibile con i successivi approfondimenti, non un catalogo immutabile.

### Sidebar

| Elemento | Voci |
| --- | --- |
| Posizione | Apri; Apri in nuova scheda/finestra; Apri nel terminale; Nascondi dalla sidebar. Comandi riferiti alla cartella quando applicabili. |
| Preferito | Apri; Mostra nella cartella; Organizza nei gruppi; Rimuovi questo preferito. Per cartelle: anche nuova scheda/finestra e terminale. |
| Gruppo di preferiti | Rinomina gruppo; Elimina gruppo, riportando i collegamenti nella lista principale come già concordato. |
| Tag | Mostra elementi; Rinomina tag; Cambia colore; Elimina tag… |
| Ricerca salvata | Esegui; Modifica criteri; Rinomina; Elimina ricerca. |

- Sull'intestazione Preferiti: Nuovo gruppo. Sull'intestazione Tag: Nuovo tag.
- Rimuovere un preferito non elimina l'originale né gli altri collegamenti allo stesso elemento.
- Organizza nei gruppi permette di scegliere più gruppi senza spostare i file; il menu dell'area file mostra esplicitamente le appartenenze, evitando rimozioni ambigue.
- Eliminare un tag richiede conferma e rimuove il tag dalle assegnazioni, non i file.
- Eliminare una ricerca salvata non modifica risultati o documenti.

### Schede

- Duplica scheda; Chiudi; Chiudi le altre; Chiudi a destra, già confermate.
- Sposta nell'altro pannello, disponibile con doppio pannello attivo.
- Copia percorso e Apri nel terminale, per schede riferite a una cartella.

Interoperabilità e catalogo iniziale dei menu sono stati analizzati prima di riprendere l'AI. Successivamente l'utente ha confermato esplicitamente il perimetro AI iniziale, registrato in `03-ai-integration.md`, AI-DEC-001: ricerca semantica/agentica, domande, riassunto, confronto ed estrazione mirata, con strumenti di ricerca/lettura autorizzati e i rinvii espliciti. Questa conferma è distinta dall'accettazione dei menu. L'autonomia controllata è successivamente confermata in AI-DEC-002 e il comportamento funzionale di Auto in AI-DEC-003. Gestione concreta e valori dei limiti, consenso multi-chiamata e implementazione/verifica del routing restano da approfondire.

## 12. Limiti e passi successivi

- Non equiparare controlli illustrativi del mockup a funzionalità implementate. Il README distingue le interazioni effettive, lo stato solo in memoria e le funzioni assenti.
- I selettori di scenario esterni alla finestra sono strumenti della demo, non comandi del prodotto.
- La sovrapposizione del pannello nelle finestre strette è una soluzione dimostrativa: restano da specificare layout minimo, breakpoint, overflow e casi con doppio pannello.
- Il catalogo iniziale dei menu è raccolto nelle sezioni 10–11. Restano da dettagliare scorciatoie per piattaforma, mappatura dei formati, stati vuoti/errore, dettagli di coordinamento delle attività e consenso per indicizzazione continua/revoca. Contesto conversazionale temporaneo e sua eliminazione confermati in AI-DEC-009, chiusura con AI attiva e arresto/ripresa dell'indicizzazione in AI-DEC-010; consenso per singola richiesta utente già confermato in AI-DEC-007.
- Focus, tecnologie assistive, contrasto, riduzione del movimento e prove di accessibilità richiedono criteri verificabili; la validazione visiva non li certifica.
- Per il passaggio ai requisiti, collegare questa analisi e il mockup approvato alle decisioni funzionali esistenti. Non iniziare implementazione o modificare il perimetro AI per deduzione dal solo mockup.
