# FolderAI — File explorer e dati locali

**Stato:** prima analisi funzionale conclusa a livello di perimetro. L'utente ha confermato il primo rilascio in FE-DEC-061. Le decisioni confermate sono distinte dai dettagli ancora aperti e dalle proposte non approvate; non costituiscono ancora una specifica formale di requisiti. Riepilogo: `04-analysis-summary.md`.

## 1. Base confermata

- Il prodotto è una vera alternativa al file manager di sistema.
- Finder su macOS ed Esplora file su Windows sono i riferimenti indicati dall'utente.
- Deve essere comodo per utenti generalisti e potente/personalizzabile per professionisti e sviluppatori.
- Gli agenti iniziali sono in sola lettura; questo non vieta le normali operazioni manuali dell'utente.
- È previsto un database vettoriale locale per le cartelle configurate.
- L'utente propone anche un piccolo database SQL locale per metadati aggiuntivi, preferiti e altri dati applicativi, se utile.
- Nel primo rilascio sono confermati preferiti per file e cartelle con gruppi facoltativi, tag personalizzati per file e cartelle, note Markdown con salvataggio automatico e ricerche salvate dinamiche. I dettagli sono tracciati nelle sezioni successive.
- Dischi esterni e cartelle di rete non sono richiesti nel primo rilascio.
- Navigazione confermata: pannello singolo iniziale, schede, doppio pannello opzionale, barra laterale configurabile con preferiti/cartelle/tag/ricerche salvate e breadcrumb modificabile come percorso testuale. In modalità doppio pannello, ciascun pannello ha la propria barra di schede e navigazione indipendente (FE-DEC-018).
- Viste confermate: dettagli, griglia e colonne stile Finder, senza rimandare quella a colonne. Le caratteristiche specifiche delle viste restano da approfondire.
- Pannello laterale opzionale condiviso: anteprima in sola lettura e informazioni (metadati, tag, note e stato di indicizzazione). Formati iniziali definiti in FE-DEC-050; riepilogo per selezioni multiple in FE-DEC-051. Interazione di dettaglio da progettare.
- Preferenze per cartella confermate: vista, ordinamento, raggruppamento e colonne visibili.
- Avvio configurabile nella sezione Preferenze: aprire la cartella iniziale oppure ripristinare l'ambiente di navigazione precedente (FE-DEC-015). Il valore predefinito è aprire la cartella iniziale (FE-DEC-016): Home dell'utente, personalizzabile nelle Preferenze; se la cartella scelta non è accessibile, apertura della Home con avviso senza cancellare la preferenza (FE-DEC-017).
- Operazioni manuali di base e gestione sicura confermate, come dettagliato nella sezione 7. La rinomina in blocco è rimandata a una fase successiva (FE-DEC-014).
- Apertura dei file: l'utente richiede di poter cambiare l'applicazione predefinita in modo intuitivo e immediato. La preferenza vale solo per le aperture effettuate da FolderAI. Vedere sezione 8.

## 2. Esperienza di esplorazione: aspetti da definire

I riferimenti Finder ed Esplora file indicano familiarità dell'esperienza, non una richiesta di replicare ogni loro funzione.

| Ambito | Aspetti da approfondire |
| --- | --- |
| Navigazione | Confermati pannello singolo iniziale, schede, doppio pannello opzionale, sidebar configurabile e breadcrumb/percorso testuale; da dettagliare cronologia, cartella superiore e comportamento di schede e pannelli |
| Viste | Confermate dettagli, griglia e colonne stile Finder, pannello laterale e preferenze per cartella; valori iniziali e copertura in FE-DEC-045–054, dettagli residui di presentazione da progettare |
| Selezione | Selezione multipla, tastiera, azioni contestuali e comportamento dei risultati di ricerca |
| Operazioni manuali | Base confermata nella sezione 7; dettagliare semantica multipiattaforma e gestione degli errori. Cambio dell'app predefinita richiesto, con ambito da definire |
| Operazioni lunghe | Centro attività, avanzamento, annullamento ed errori confermati; definire stati e risultati parziali |
| Conflitti | Per file e cartelle omonimi confermato «mantieni entrambi» automatico con suffisso; sostituzione soltanto esplicita e sicura. Distinguere conflitti di nome da permessi e file in uso |
| Recupero | Confermati cestino predefinito, eliminazione permanente esplicita e confermata, distinzione tra interruzione e ripristino; recuperabilità da dettagliare |
| Integrazione OS | Appunti, drag-and-drop, applicazioni associate e funzioni specifiche della piattaforma |

Occorre distinguere le aspettative comuni tra piattaforme dalle convenzioni native. Le astrazioni non devono nascondere differenze che incidono sui dati o sul comportamento delle operazioni.

## 3. Perché affiancare dati relazionali all'indice vettoriale

La proposta di un database SQL locale è coerente con la necessità di conservare dati strutturati e relazioni. L'indice vettoriale risolve un problema diverso: recuperare contenuti per similarità.

Separazione logica proposta:

| Livello | Responsabilità | Esempi |
| --- | --- | --- |
| Filesystem | Fonte autorevole per esistenza, contenuto e stato corrente dei file | File, cartelle, permessi, dimensioni e date |
| Dati applicativi relazionali | Informazioni proprie di FolderAI e stato delle elaborazioni | Preferiti, cartelle configurate, eventuali tag/note e ricerche salvate, stato di indicizzazione |
| Indice vettoriale | Ricerca per similarità su rappresentazioni derivate dai contenuti | Embedding e riferimenti ai passaggi o alle altre unità indicizzate |

Questa separazione è logica, non impone tre sistemi di storage distinti. Database SQL e indice vettoriale potrebbero condividere lo stesso motore oppure usare componenti separati: la scelta richiede verifiche su volumi, ricerca, aggiornamenti e distribuzione multipiattaforma.

**SQLite è un candidato da valutare**, non una scelta già approvata. È coerente con un'app desktop locale priva di un servizio database dedicato; andranno verificate anche l'eventuale estensione vettoriale e le alternative.

Non è necessario salvare ogni impostazione in SQL. La collocazione dipende dalla natura dei dati; le credenziali richiedono un archivio sicuro adatto alla piattaforma, non normali tabelle o file di configurazione in chiaro.

## 4. Metadati: distinzioni importanti

- **Nativi:** nome, percorso, dimensioni, date e altri attributi esposti dal filesystem; eventuali copie nel database sono cache da riconciliare.
- **Aggiunti dall'utente:** tag personalizzati assegnabili sia a file sia a cartelle (FE-DEC-032) e note o descrizioni associate sia ai file sia alle cartelle (FE-DEC-035) sono funzionalità confermate. Formato Markdown con anteprima (FE-DEC-036) e salvataggio automatico con stato ed errori visibili (FE-DEC-037) confermati; limiti e gestione delle modifiche concorrenti restano da definire. Questi dati non devono essere considerati rigenerabili.
- **Derivati automaticamente:** testo estratto, riassunti o classificazioni, se previsti; devono essere collegati alla versione del contenuto e alla provenienza.
- **Operativi:** stato di elaborazione, errori, versione dell'estrattore e del modello di embedding; servono a capire se un indice è aggiornato.

L'eventuale scrittura dei metadati aggiuntivi dentro i file o negli attributi del filesystem è una decisione distinta dalla loro conservazione nel database locale. Nessuna scrittura automatica nei file da parte degli agenti è approvata per il primo rilascio.

## 5. Consistenza e ciclo di vita

Problemi da risolvere prima di definire schema e requisiti:

1. **Identità dei file:** il percorso cambia con una rinomina; gli identificatori nativi non sono universalmente disponibili o stabili. Confermate conservazione dei metadati nelle operazioni interne (FE-DEC-043) e riconciliazione delle modifiche esterne quando l'identità è riconosciuta con affidabilità (FE-DEC-044). Definire meccanismi, limiti e casi non risolvibili.
2. **Modifiche ed eliminazioni:** aggiornare o invalidare cache ed embedding senza presentare contenuti vecchi come attuali.
3. **Separazione degli ambiti:** cartelle navigabili, preferiti e cartelle autorizzate all'indicizzazione non devono essere considerate automaticamente lo stesso insieme.
4. **Ricostruibilità:** ricreare un indice non deve cancellare metadati inseriti dall'utente o preferiti. Il testo estratto e gli embedding possono essere rigenerabili, ma con un costo di elaborazione.
5. **Aggiornamenti coordinati:** se dati relazionali e vettoriali sono separati, gestire lavori incompleti, errori e riavvii senza presumere una transazione unica tra motori.
6. **Carico e reattività:** navigazione e preferiti non devono dipendere dall'attesa dei lavori AI; servono misurazioni su volumi rappresentativi. Scala complessiva attesa: anche centinaia di migliaia di file. Compatibilità con 8 GB di RAM desiderata, senza ridurre funzionalità o qualità per farne un vincolo rigido (FE-DEC-060).
7. **Protezione e manutenzione:** chiarire conservazione dei contenuti estratti, cancellazione dei dati derivati, backup dei dati utente e migrazioni dopo gli aggiornamenti dell'app.

Il database dell'applicazione e l'indice restano scrivibili per il loro funzionamento anche se gli agenti possono soltanto leggere i file dell'utente: si tratta di autorizzazioni diverse.

## 6. Decisioni successive

- Preferiti per file e cartelle confermati: gruppi facoltativi a un solo livello, lista semplice iniziale e organizzazione tramite trascinamento (FE-DEC-031). I collegamenti non più raggiungibili vengono rimossi automaticamente (FE-DEC-039); definire tempi e modalità di rilevamento.
- Tag personalizzati confermati per file e cartelle (FE-DEC-032), senza ereditarietà ai contenuti (FE-DEC-033). Confermati tag multipli, riuso in tutta l'app, colore facoltativo, struttura piatta e assegnazione/rimozione su selezioni multiple (FE-DEC-034); definire gestione dei tag e utilizzo nei filtri.
- Note o descrizioni confermate per file e cartelle, senza modificare i contenuti originali (FE-DEC-035), in Markdown con anteprima (FE-DEC-036), salvataggio automatico (FE-DEC-037) e ricerca testuale con provenienza della corrispondenza esplicita (FE-DEC-038): definire gestione delle modifiche concorrenti.
- Ricerche semplici e salvataggio di richiesta, filtri e ambito confermati in FE-DEC-057. Restano da definire matrice dei formati ricercabili e dettagli delle modalità semantiche/agentiche, analizzate separatamente.
- Navigazione: disposizione generale confermata; ripristino della sessione configurabile nelle Preferenze (FE-DEC-015). Valore iniziale confermato: aprire la cartella iniziale (FE-DEC-016). Schede e navigazione indipendenti per pannello confermate (FE-DEC-018). Definire gestione delle posizioni non disponibili e mappatura delle scorciatoie; scorciatoie native personalizzabili confermate (FE-DEC-055).
- Visualizzazione: comportamenti iniziali confermati in FE-DEC-045–054; definire varianti esatte dei formati supportati, criteri di ordinamento/raggruppamento disponibili e dettagli di presentazione.
- Pannello laterale: anteprima e informazioni condividono un pannello opzionale a sezioni. Confermata anteprima dei contenuti in sola lettura nel primo rilascio, con note e tag modificabili come metadati (FE-DEC-049); definire interazione e comportamento nel doppio pannello.
- Preferenze per cartella: persistenza e conservazione durante rinomina/spostamento confermate (FE-DEC-043); valori iniziali in FE-DEC-052–054. Definire ripristino e riconciliazione delle modifiche esterne.
- Operazioni manuali: base confermata nella sezione 7; approfondire casi limite e recuperabilità. Rinomina in blocco esclusa dal primo rilascio e rimandata (FE-DEC-014).
- Apertura dei file: la preferenza è interna a FolderAI. Restano da chiarire tipo o singolo file, scelta delle applicazioni e gestione delle associazioni non più valide.
- Scala e hardware definiti a livello orientativo in FE-DEC-060; soglie misurabili da stabilire. Esportazione/importazione rimandate (FE-DEC-059); protezione, migrazione e recupero dei dati da progettare.

Queste scelte guideranno i requisiti. Schema SQL, librerie e motore vettoriale verranno valutati dopo, senza anticipare l'implementazione.

## 7. Operazioni manuali — base confermata

L'utente accetta come base per il primo rilascio:

- Creazione di file vuoti e cartelle.
- Copia, spostamento, rinomina ed eliminazione.
- Operazioni su selezioni multiple.
- Copia/taglia/incolla e drag-and-drop tra cartelle, schede e pannelli.
- Apertura con applicazione predefinita oppure «Apri con».
- Cestino come scelta predefinita; eliminazione permanente esplicita e confermata.
- Gestione dei conflitti inizialmente condivisa con scelte sostituisci, salta o mantieni entrambi. Le decisioni successive FE-DEC-007 e FE-DEC-008 precisano il comportamento predefinito per cartelle e file omonimi: mantenere entrambi automaticamente. «Sostituisci», quando esplicitamente richiesto, resta soggetto a FE-DEC-006; la sua accessibilità nell'interfaccia è da definire.
- Centro attività per operazioni lunghe, con avanzamento, annullamento ed errori.
- Distinzione tra interrompere un'operazione e annullarne gli effetti: il ripristino non è sempre possibile.

Comportamenti di dettaglio e criteri di accettazione rimangono da definire. La rinomina in blocco con anteprima è rimandata a una fase successiva (FE-DEC-014).

### 7.1. Errore su un singolo file — decisione FE-DEC-001

**Stato:** comportamento predefinito confermato dall'utente, scegliendo l'opzione C.

Durante una copia o uno spostamento multiplo, se un singolo file non è leggibile ma gli altri sono elaborabili, FolderAI attende la scelta dell'utente: **riprova, salta oppure interrompi**. Deve essere disponibile l'opzione **«Applica ai prossimi errori simili»**.

**Durata della scelta — FE-DEC-002:** l'utente sceglie l'opzione A. «Applica ai prossimi errori simili» vale soltanto per l'operazione corrente. Una nuova operazione torna a chiedere; la scelta non diventa una preferenza di sessione né persistente.

**Attesa della decisione — FE-DEC-003:** l'utente sceglie l'opzione A. In attesa della risposta, si sospende soltanto l'operazione interessata e non si procede con gli altri file della stessa copia o spostamento. La navigazione e le altre operazioni indipendenti restano utilizzabili; non si blocca tutta l'applicazione.

Da approfondire: definizione di errori simili. Errori che rendono indisponibile l'intera destinazione e conflitti di sovrascrittura saranno analizzati separatamente. La decisione non implica un ripristino automatico di quanto già completato.

### 7.2. Annullamento di un trasferimento — decisione FE-DEC-004

**Stato:** comportamento confermato dall'utente, scegliendo l'opzione A.

Durante una copia o uno spostamento, «Annulla» interrompe anche il trasferimento del file in corso appena tecnicamente sicuro, senza attenderne necessariamente il completamento. I file già trasferiti restano tali: l'annullamento dell'attività non comporta un ripristino automatico della situazione iniziale.

**Gestione della copia parziale — FE-DEC-005:** l'utente sceglie l'opzione A. Dopo l'interruzione si rimuove automaticamente la nuova copia parziale nella destinazione; se la rimozione fallisce, FolderAI lo segnala. Questa decisione non autorizza a eliminare un file preesistente nella destinazione.

Per gli spostamenti tra volumi, l'originale non viene eliminato finché la copia non è completata con successo. La decisione riguarda il comportamento delle operazioni manuali e non anticipa il supporto a dischi esterni o cartelle di rete, rinviato a una fase futura.

### 7.3. Sostituzione sicura — decisione FE-DEC-006

**Stato:** vincolo di sicurezza confermato dall'utente, scegliendo l'opzione A.

Quando l'utente sceglie «Sostituisci», FolderAI prepara la nuova copia mantenendo intatto il file precedente e procede alla sostituzione soltanto dopo il completamento della preparazione. Se il trasferimento viene annullato o fallisce prima della sostituzione finale, il file precedente resta intatto.

La procedura richiede spazio aggiuntivo; se non è sufficiente, la sostituzione non procede. Non viene offerta una sovrascrittura diretta come alternativa per aggirare questo vincolo.

Da approfondire: garanzie della sostituzione finale sui filesystem supportati, errori durante tale fase, modifiche concorrenti e pulizia dei dati temporanei. La decisione non costituisce una garanzia universale di recupero dopo crash o perdita di alimentazione.

### 7.4. Cartelle omonime — decisione FE-DEC-007

**Stato:** comportamento predefinito confermato dall'utente, scegliendo l'opzione B e specificando il formato del suffisso.

Quando si copia o sposta una cartella in una destinazione che contiene già una cartella omonima, FolderAI mantiene entrambe e assegna automaticamente un nuovo nome alla cartella in arrivo. Non unisce automaticamente i contenuti.

Per una cartella chiamata `Documenti`, la sequenza dei nomi candidati è:

1. `Documenti - copy`
2. `Documenti - copy (2)`
3. `Documenti - copy (3)`, e così via finché si trova un nome disponibile.

La cartella preesistente non viene rinominata. Il comportamento predefinito non richiede la scelta manuale del nuovo nome.

Da approfondire: nomi già provvisti del suffisso, limiti di lunghezza, equivalenza dei nomi secondo il filesystem e conflitti introdotti da modifiche concorrenti. Il caso di un file nella destinazione con lo stesso nome della cartella resta da analizzare.

### 7.5. File omonimi — decisione FE-DEC-008

**Stato:** comportamento predefinito confermato dall'utente, scegliendo l'opzione A.

Anche per i file omonimi, durante copia o spostamento, FolderAI mantiene entrambi automaticamente, rinominando il file in arrivo e preservando l'estensione. Non chiede ogni volta se sostituire, saltare o mantenere entrambi.

Per `relazione.pdf`, la sequenza dei nomi candidati è:

1. `relazione - copy.pdf`
2. `relazione - copy (2).pdf`
3. `relazione - copy (3).pdf`, e così via finché si trova un nome disponibile.

Il file preesistente non viene modificato. Questa decisione precisa la gestione dei conflitti di nome senza cambiare la politica di richiesta all'utente per gli errori di lettura o trasferimento (FE-DEC-001).

Da approfondire: estensioni composte come `.tar.gz`, file senza estensione, nomi nascosti, suffissi già presenti, conflitti tra file e cartelle e modalità di richiesta esplicita della sostituzione sicura.

### 7.6. Conferma prima del cestino — decisione FE-DEC-009

**Stato:** comportamento predefinito confermato dall'utente, scegliendo l'opzione A.

L'eliminazione normale sposta subito file e cartelle nel cestino, senza conferma preventiva, con una notifica dell'esito. L'eliminazione permanente resta un'azione distinta, esplicita e confermata.

**Cestino non disponibile — FE-DEC-010:** l'utente sceglie l'opzione A. Se un elemento non può essere spostato nel cestino, FolderAI si ferma e segnala il problema, senza eliminarlo e senza proporre in quel flusso l'eliminazione permanente. L'utente può eventualmente richiedere quest'ultima tramite un'azione separata, esplicita e confermata.

Non è autorizzata una conversione silenziosa dell'operazione in eliminazione permanente. La decisione non implica il ripristino degli elementi già cestinati in un'operazione multipla; il trattamento degli elementi rimanenti resta da precisare.

### 7.7. Annullamento di azioni completate — decisione FE-DEC-011

**Stato:** inclusione nel primo rilascio confermata dall'utente, scegliendo l'opzione A.

FolderAI offre «Annulla ultima azione», accessibile anche da tastiera, per le operazioni recuperabili, come rinomina, spostamento e invio al cestino, dove supportato. Prima del ripristino verifica eventuali modifiche o conflitti, senza sovrascrivere silenziosamente altri file.

La funzione è distinta dall'interruzione di un'attività in corso. Non costituisce una promessa di ripristino universale; l'eliminazione permanente resta esclusa dal recupero garantito.

**Durata della cronologia — FE-DEC-012:** l'utente sceglie l'opzione A. La cronologia di annullamento è limitata alla sessione e viene azzerata alla chiusura dell'applicazione; non persiste tra i riavvii. I file nel cestino restano comunque recuperabili tramite il sistema, dove supportato.

**Profondità della cronologia — FE-DEC-013:** l'utente sceglie l'opzione A. Si può annullare soltanto l'ultima azione; dopo averla annullata non è possibile risalire alle precedenti. Non è prevista una sequenza di annullamenti a più livelli.

Da definire: azioni effettivamente supportate, gestione delle operazioni multiple o parzialmente completate e dei conflitti durante il ripristino. Il comportamento di eventuali azioni non annullabili rispetto all'unica azione memorizzata e l'eventuale funzione «Ripeti» restano aperti.

### 7.8. Rinomina in blocco — decisione FE-DEC-014

**Stato:** rinvio confermato dall'utente, scegliendo l'opzione B.

Nel primo rilascio è disponibile soltanto la rinomina di un elemento alla volta. La rinomina in blocco è rimandata a una fase successiva. Sostituzione di testo, prefissi/suffissi, numerazione e anteprima erano proposte per tale funzione e non sono requisiti del primo rilascio.

## 8. Apertura dei file e applicazioni predefinite

### Esigenza confermata

Cambiare l'applicazione predefinita per aprire un file deve essere intuitivo e immediato, senza obbligare l'utente a cercare impostazioni difficili da raggiungere. Il criterio di associazione resta da definire.

### Flusso di interazione proposto

1. Menu contestuale «Apri con», con l'applicazione attuale riconoscibile e le alternative disponibili.
2. Scelta di un'applicazione per una singola apertura.
3. Azione vicina alla scelta: «Usa sempre per questo tipo di file in FolderAI».
4. Possibilità di scegliere un'altra applicazione se non compare tra quelle suggerite.
5. Indicazione della preferenza attiva e azione per ripristinare il comportamento predefinito del sistema.

Il flusso è una proposta da confermare, non una UI già approvata.

### Ambito della preferenza — confermato

Il cambio dell'applicazione predefinita vale esclusivamente per le aperture effettuate da FolderAI. L'associazione del sistema operativo continua a valere per le aperture da Finder, Esplora file e altre applicazioni che la usano.

### Aspetti da chiarire

- Associazione per tipo di file oppure eventuale eccezione per singolo file.
- Riconoscimento dei tipi: estensione, MIME/type identifier e differenze tra sistemi operativi.
- Comportamento quando l'applicazione scelta è stata rimossa o non è disponibile.
- Selezione multipla con tipi diversi e disponibilità di applicazioni compatibili.
- Apertura tramite meccanismi della piattaforma, senza costruire comandi shell a partire da percorsi non attendibili.
- Applicazione predefinita non equivale a autorizzazione a eseguire automaticamente script o programmi selezionati.

## 9. Navigazione, schede e pannelli

### 9.1. Avvio e ripristino della sessione — decisione FE-DEC-015

**Stato:** configurabilità confermata dall'utente. Non è stato scelto un unico comportamento fisso per tutti gli utenti.

Nella sezione **Preferenze**, l'utente deve poter scegliere tra:

- Partire dalla cartella iniziale senza ripristinare schede e pannelli precedenti.
- Ripristinare l'ambiente di navigazione precedente: schede, cartelle aperte, disposizione dei pannelli e scheda attiva.

Il ripristino riguarda soltanto l'ambiente di navigazione: non conserva la cronologia di annullamento tra sessioni e non riavvia i trasferimenti interrotti.

**Valore predefinito — FE-DEC-016:** l'utente sceglie l'opzione A. Prima di modifiche nelle Preferenze, FolderAI apre la cartella iniziale senza ripristinare la sessione precedente. Resta disponibile la scelta di attivare il ripristino della sessione (FE-DEC-015).

**Cartella iniziale — FE-DEC-017:** l'utente conferma la Home dell'utente come valore iniziale, individuata tramite il sistema operativo, con possibilità di scegliere un'altra cartella nelle Preferenze. Se la cartella scelta non è più accessibile, FolderAI apre la Home e mostra un avviso, senza cancellare la preferenza salvata.

Da definire: comportamento per posizioni non disponibili durante il ripristino di una sessione e caso in cui anche la Home non sia accessibile.

### 9.2. Schede nel doppio pannello — decisione FE-DEC-018

**Stato:** organizzazione confermata dall'utente, scegliendo l'opzione A.

In modalità doppio pannello, ciascun pannello ha la propria barra di schede e navigazione indipendente. Cambiare scheda in un pannello non cambia quella attiva nell'altro. Non viene adottata una barra globale di schede intese come spazi di lavoro a uno o due pannelli.

**Disattivazione del doppio pannello — FE-DEC-019:** dopo il chiarimento con un esempio, l'utente sceglie l'opzione B. Tornando al pannello singolo, tutte le schede dei due pannelli vengono riunite in un'unica barra nel pannello rimasto, senza conservarne la divisione precedente. Non si mantiene un gruppo di schede nascosto da ripristinare automaticamente.

**Attivazione del doppio pannello — FE-DEC-020:** l'utente sceglie l'opzione B. Quando si attiva o riattiva il doppio pannello, quello nuovo parte con una sola scheda nella cartella iniziale configurata (Home o cartella scelta nelle Preferenze). Le schede esistenti restano nell'altro pannello. Se la cartella iniziale non è accessibile, si applica il fallback definito in FE-DEC-017.

### 9.3. Apertura di una nuova scheda — decisione FE-DEC-021

**Stato:** comportamento confermato dall'utente, scegliendo l'opzione B.

Premendo «+», si apre una nuova scheda nel pannello attivo, nella cartella iniziale configurata (Home o cartella scelta nelle Preferenze), non nella cartella della scheda attuale. Se la cartella iniziale non è accessibile, si applica il fallback definito in FE-DEC-017. La decisione riguarda l'apertura di una scheda vuota tramite «+», non l'azione «Apri questa cartella in una nuova scheda».

### 9.4. Organizzazione delle schede — decisione FE-DEC-022

**Stato:** inclusione nel primo rilascio confermata dall'utente, scegliendo l'opzione A.

Le schede possono essere trascinate per riordinarle nel proprio pannello e per spostarle tra i due pannelli. Il trascinamento verso l'altro pannello trasferisce la scheda, senza duplicarla.

**Pannello rimasto senza schede — FE-DEC-023:** l'utente sceglie l'opzione B. Se si chiude o trasferisce l'ultima scheda di uno dei due pannelli, quello vuoto scompare e si torna al pannello singolo, mantenendo aperto quello con le schede. Non si crea automaticamente una scheda sostitutiva nel pannello vuoto.

**Chiusura dell'ultima scheda — FE-DEC-024:** l'utente sceglie l'opzione A. In modalità pannello singolo, chiudere l'unica scheda rimasta chiude la finestra di FolderAI; non viene creata una scheda sostitutiva nella cartella iniziale. La decisione non stabilisce ancora il comportamento delle attività in corso né implica necessariamente la terminazione dell'intera applicazione.

### 9.5. Chiusura con trasferimenti in corso — decisione FE-DEC-025

**Stato:** comportamento confermato dall'utente, scegliendo l'opzione A.

Se si chiude una finestra mentre sono in corso copie o spostamenti a essa associati, FolderAI chiede conferma. L'utente può lasciare la finestra aperta oppure interrompere in sicurezza i trasferimenti e chiuderla. Non si passa automaticamente all'esecuzione in background.

L'interruzione applica le decisioni FE-DEC-004, FE-DEC-005 e FE-DEC-006: arresto appena tecnicamente sicuro, rimozione delle nuove copie parziali con segnalazione degli errori e protezione dei file preesistenti durante le sostituzioni. I trasferimenti già completati non vengono annullati automaticamente.

Da definire: errori di pulizia durante la chiusura e comportamento alla chiusura durante attività diverse dai trasferimenti. La proprietà dei trasferimenti per finestra è definita in FE-DEC-028.

### 9.6. Finestre multiple — decisione FE-DEC-026

**Stato:** inclusione nel primo rilascio confermata dall'utente, scegliendo l'opzione A.

FolderAI supporta più finestre contemporaneamente. Ogni finestra ha le proprie schede e la propria disposizione a uno o due pannelli.

Da definire: ripristino di più finestre quando abilitato nelle Preferenze. L'ambito dell'annullamento è definito in FE-DEC-029 e lo stato iniziale di una nuova finestra in FE-DEC-030.

**Visibilità del centro attività — FE-DEC-027:** l'utente sceglie l'opzione B. Il centro attività è separato per finestra: ogni finestra mostra soltanto le operazioni avviate al suo interno, non un elenco globale delle attività dell'applicazione. Questa decisione riguarda la visibilità; il comportamento dei trasferimenti alla chiusura è precisato in FE-DEC-028.

**Proprietà dei trasferimenti e chiusura — FE-DEC-028:** l'utente conferma la separazione per finestra. La conferma di chiusura riguarda soltanto i trasferimenti avviati dalla finestra che si sta chiudendo. Se l'utente conferma, vengono interrotti in sicurezza soltanto quei trasferimenti. Le altre finestre e i relativi trasferimenti continuano normalmente. Questa decisione non definisce il comportamento di un comando di uscita dall'intera applicazione.

**Ambito dell'annullamento — FE-DEC-029:** l'utente sceglie l'opzione A. «Annulla ultima azione» riguarda la finestra corrente: ciascuna finestra conserva indipendentemente la propria ultima azione, annullabile quando recuperabile. Non si annulla un'azione appartenente a un'altra finestra. Restano validi il limite di una sola azione e l'assenza di persistenza dopo la chiusura dell'applicazione (FE-DEC-012 e FE-DEC-013).

**Nuova finestra — FE-DEC-030:** l'utente sceglie l'opzione A. Il comando «Nuova finestra» apre un ambiente iniziale pulito: pannello singolo con una sola scheda nella cartella iniziale configurata. Non replica l'organizzazione della finestra attiva. Se la cartella iniziale non è accessibile, si applica il fallback FE-DEC-017. Questa decisione riguarda il comando «Nuova finestra», non il ripristino della sessione né l'eventuale azione «Apri questa cartella in una nuova finestra».

Altri punti di navigazione aperti: ordine delle schede riunite, scheda attiva dopo l'unione e pannello di destinazione delle altre azioni di navigazione.

## 10. Organizzazione dei contenuti

### 10.1. Gruppi di preferiti — decisione FE-DEC-031

**Stato:** soluzione confermata dall'utente dopo il chiarimento e la raccomandazione.

Nel primo rilascio, i preferiti per file e cartelle possono essere organizzati in **gruppi personalizzati facoltativi**. L'esperienza iniziale resta una semplice lista: l'utente non deve creare gruppi per utilizzare i preferiti.

Quando necessario, l'utente può creare gruppi, per esempio «Lavoro» e «Personale», e organizzare gruppi ed elementi tramite trascinamento. È previsto **un solo livello di gruppi, senza sottogruppi annidati**. I preferiti restano collegamenti: raggrupparli o riordinarli non sposta i file originali.

**Preferiti non disponibili — FE-DEC-039:** l'utente sceglie l'opzione B. Se il file o la cartella di destinazione non è più raggiungibile, per eliminazione, spostamento esterno non riconciliato o inaccessibilità, il preferito viene rimosso automaticamente. Non resta un collegamento «Non disponibile» da ricollegare manualmente. La decisione riguarda soltanto il collegamento nei preferiti e non autorizza a cancellare note o tag.

**Eliminazione di un gruppo — FE-DEC-040:** l'utente sceglie l'opzione A. Eliminando un gruppo di preferiti, i collegamenti contenuti vengono conservati e riportati nella lista principale, senza gruppo. Viene eliminato soltanto il raggruppamento, non i collegamenti né i file o le cartelle originali.

**Stesso elemento in più gruppi — FE-DEC-041:** l'utente sceglie l'opzione B. Uno stesso file o una stessa cartella può comparire in più gruppi di preferiti, tramite collegamenti indipendenti allo stesso originale, senza duplicare il file. Rimuovere un collegamento da un gruppo non implica rimuovere quelli negli altri gruppi.

Da definire: tempi e modalità di rilevamento dell'indisponibilità, ordine dei collegamenti riportati nella lista principale, gestione delle duplicazioni nello stesso gruppo e conflitti quando si elimina un gruppo contenente elementi già presenti nella lista principale.

### 10.2. Tag su file e cartelle — decisione FE-DEC-032

**Stato:** ambito confermato dall'utente, scegliendo l'opzione A.

I tag personalizzati possono essere assegnati sia ai file sia alle cartelle. Uno stesso tag, per esempio «Apollo», può classificare elementi di entrambi i tipi in posizioni diverse.

**Nessuna ereditarietà — FE-DEC-033:** l'utente sceglie l'opzione A. Un tag assegnato a una cartella riguarda soltanto quella cartella. Non viene ereditato da file o sottocartelle, né dai contenuti aggiunti successivamente; gli elementi mantengono i propri tag.

**Struttura e assegnazione — FE-DEC-034:** l'utente conferma per il primo rilascio:

- Più tag assegnabili a ciascun file o cartella.
- Tag riutilizzabili in tutta l'applicazione, senza ricrearli per ogni cartella.
- Colore facoltativo per ciascun tag.
- Elenco piatto, senza gerarchie o sottotag.
- Assegnazione e rimozione dei tag su selezioni multiple.

Restano da definire rinomina, eliminazione e gestione di tag duplicati, utilizzo nei filtri e rappresentazione delle assegnazioni diverse in una selezione multipla.

### 10.3. Note e descrizioni — decisione FE-DEC-035

**Stato:** confermata l'estensione proposta alle cartelle.

Le note possono essere associate sia ai file sia alle cartelle. Sono metadati di FolderAI e non modificano il contenuto originale dei documenti.

**Formato delle note — FE-DEC-036:** l'utente sceglie l'opzione B. Le note supportano Markdown con anteprima, inclusi titoli, elenchi, grassetto, collegamenti e blocchi di codice. Deve rimanere immediato scrivere una nota come normale testo, senza essere obbligati a usare formattazione.

**Salvataggio delle note — FE-DEC-037:** l'utente sceglie l'opzione A. Le modifiche vengono salvate automaticamente dopo una breve pausa nella digitazione, con indicazione dello stato («Salvataggio…» / «Salvato») ed eventuali errori visibili. Non è stato definito un intervallo numerico.

**Ricerca nelle note — FE-DEC-038:** l'utente sceglie l'opzione A. La ricerca semplice può trovare file e cartelle tramite il testo delle loro note. Il risultato indica che la corrispondenza proviene dalla nota, distinguendola dal contenuto originale del file. Questa decisione non implica invio delle note a modelli AI né indicizzazione semantica.

Da definire: limiti, gestione delle modifiche concorrenti, cambio di selezione o chiusura con salvataggio pendente e rendering sicuro dei contenuti Markdown. Il salvataggio automatico non deve essere considerato già validato contro la perdita di modifiche in queste situazioni.

### 10.4. Metadati durante la copia — decisione FE-DEC-042

**Stato:** configurabilità e valore predefinito confermati dall'utente.

Per le copie di file e cartelle effettuate tramite FolderAI, l'utente può configurare se copiare anche note e tag oppure creare il nuovo elemento senza questi metadati. **Il valore predefinito è copiare note e tag dell'originale (opzione A).**

Quando copiati, i metadati diventano indipendenti: modificarli su un elemento non cambia quelli dell'altro. La decisione riguarda note e tag, non implica duplicazione dei preferiti né un collegamento permanente tra copia e originale. Non introduce ereditarietà dei tag delle cartelle ai contenuti (FE-DEC-033).

Da definire: collocazione della preferenza, comportamento nelle copie ricorsive di cartelle e gestione degli errori o risultati parziali nella copia dei metadati.

### 10.5. Rinomina e spostamento — decisione FE-DEC-043

**Stato:** comportamento confermato dall'utente.

Quando un elemento viene rinominato o spostato tramite FolderAI:

- Note e tag associati vengono conservati.
- Tutti i relativi collegamenti nei preferiti vengono aggiornati, anche se presenti in più gruppi.
- Se si tratta di una cartella, vengono mantenute le sue preferenze di visualizzazione.

Il cambio di nome o posizione non comporta la perdita dell'organizzazione dell'elemento. La decisione riguarda operazioni eseguite tramite FolderAI; il comportamento per modifiche esterne è definito a livello di indirizzo in FE-DEC-044.

Da definire: aggiornamento dei riferimenti ai contenuti di una cartella spostata, coerenza in caso di operazioni parziali o errori di aggiornamento dei metadati e integrazione con «Annulla ultima azione».

### 10.6. Modifiche esterne — decisione FE-DEC-044

**Stato:** comportamento confermato dall'utente, scegliendo l'opzione A.

Se un file o una cartella viene rinominato o spostato tramite Finder, Esplora file o un'altra applicazione, FolderAI tenta di riconoscere il cambiamento. Quando identifica con affidabilità lo stesso elemento, aggiorna i riferimenti e conserva i metadati associati.

Non associa automaticamente note e tag a un elemento diverso soltanto perché ha un nome simile. Non è garantito il riconoscimento di ogni spostamento o rinomina.

Da definire: evidenze di identità accettabili per ciascuna piattaforma/filesystem, rilevamento delle modifiche, casi ambigui o non risolvibili e coordinamento della riconciliazione con la rimozione dei preferiti non raggiungibili (FE-DEC-039), evitando di trattare prematuramente un elemento riconoscibile come scomparso.

## 11. Dettagli di visualizzazione

### 11.1. Elementi nascosti — decisione FE-DEC-045

**Stato:** impostazione confermata dall'utente.

File e cartelle nascosti non vengono mostrati per impostazione predefinita. Il comando rapido «Mostra elementi nascosti» è disponibile dal menu e da tastiera. Una preferenza persistente permette di mantenerli visibili.

La visibilità non modifica i permessi di accesso agli elementi.

Da definire: ambito del comando rapido e della preferenza, riconoscimento degli elementi nascosti sulle diverse piattaforme ed eventuale trattamento distinto degli elementi protetti di sistema.

### 11.2. Estensioni dei file — decisione FE-DEC-046

**Stato:** configurabilità e valore predefinito confermati dall'utente.

Per impostazione predefinita, FolderAI mostra il nome completo dei file, inclusa l'estensione (opzione A), per esempio `contratto.pdf` e `script.py`.

La visualizzazione è configurabile nelle Preferenze: l'utente può scegliere di nascondere le estensioni dei tipi conosciuti. La preferenza riguarda soltanto la presentazione e non modifica i nomi effettivi dei file.

Da definire: riconoscimento dei tipi conosciuti, gestione delle estensioni composte e rappresentazione dei nomi ambigui.

### 11.3. Dimensioni delle cartelle — decisione FE-DEC-047

**Stato:** comportamento per il primo rilascio confermato dall'utente, scegliendo l'opzione A.

La dimensione totale di una cartella viene calcolata su richiesta esplicita dell'utente, per esempio dal pannello informazioni. La normale navigazione non avvia scansioni automatiche per calcolare le dimensioni delle cartelle visualizzate.

Da definire: avanzamento e annullamento del calcolo, gestione di permessi insufficienti, link e contenuti modificati durante la scansione, distinzione tra dimensione logica e spazio occupato ed eventuale conservazione dei risultati con indicazione del loro aggiornamento.

### 11.4. Miniature nella vista griglia — decisione FE-DEC-048

**Stato:** comportamento confermato dall'utente, scegliendo l'opzione A.

Per i formati supportati, FolderAI genera e carica le miniature quando servono per gli elementi visibili nella griglia. Le miniature vengono conservate in una cache locale per evitare rigenerazioni continue.

La generazione delle miniature è distinta dall'indicizzazione AI e non richiede invii al cloud.

Da definire: formati supportati, limiti e pulizia della cache, invalidazione dopo modifiche o eliminazioni, protezione dei dati derivati, gestione degli errori e limiti delle risorse impiegate.

### 11.5. Anteprima in sola lettura — decisione FE-DEC-049

**Stato:** perimetro del primo rilascio confermato dall'utente, scegliendo l'opzione A.

Il pannello laterale permette di consultare i contenuti in sola lettura. Per modificarli, l'utente apre il file nell'applicazione associata. Non è incluso nel primo rilascio un editor integrato dei contenuti, nemmeno per testo o Markdown.

Note e tag restano modificabili perché sono metadati di FolderAI e non alterano i documenti originali.

Da definire: rendering sicuro dei contenuti non attendibili, limiti per file grandi e gestione delle selezioni multiple. La copertura iniziale delle anteprime è definita in FE-DEC-050.

### 11.6. Formati di anteprima — decisione FE-DEC-050

**Stato:** perimetro del primo rilascio confermato dall'utente, scegliendo l'opzione A.

Le anteprime integrate coprono un insieme iniziale mirato:

- Testo e codice sorgente.
- Markdown.
- Immagini comuni.
- PDF.

Gli altri formati vengono aperti tramite applicazioni esterne. L'anteprima integrata di documenti, fogli di calcolo e presentazioni Office/OpenDocument non è inclusa nel primo rilascio.

La decisione riguarda soltanto l'anteprima nell'interfaccia, non stabilisce la copertura dei formati ricercabili o indicizzabili. Restano da precisare estensioni, codifiche testuali, varianti e limiti supportati per ciascuna categoria.

### 11.7. Pannello laterale e selezione multipla — decisione FE-DEC-051

**Stato:** comportamento confermato dall'utente, scegliendo l'opzione A.

Con più elementi selezionati, il pannello laterale mostra un riepilogo della selezione anziché l'anteprima dell'ultimo elemento selezionato:

- Numero e tipi degli elementi.
- Dimensioni già disponibili, senza avviare calcoli automatici delle dimensioni delle cartelle (FE-DEC-047).
- Tag comuni o differenti tra gli elementi, con possibilità di aggiungerli o rimuoverli in blocco.

La gestione multipla dei tag non sovrascrive le note individuali.

Da definire: rappresentazione delle assegnazioni miste, riepiloghi con dati incompleti e limiti per selezioni molto grandi.

### 11.8. Vista iniziale — decisione FE-DEC-052

**Stato:** valore predefinito confermato dall'utente, scegliendo l'opzione A.

Per una cartella senza preferenze salvate, la vista iniziale è **Dettagli**. Il valore generale è modificabile nelle Preferenze; le preferenze ricordate per una singola cartella hanno precedenza sul valore generale.

Le colonne sono definite in FE-DEC-053; ordinamento e raggruppamento predefiniti sono definiti in FE-DEC-054.

### 11.9. Colonne della vista Dettagli — decisione FE-DEC-053

**Stato:** impostazione confermata dall'utente.

Colonne inizialmente visibili:

- Nome.
- Tipo.
- Dimensione.
- Ultima modifica.

Colonne attivabili: data di creazione dove disponibile, tag, presenza di note e stato di indicizzazione.

Le colonne possono essere mostrate o nascoste, riordinate e ridimensionate. La configurazione viene ricordata per cartella. Le dimensioni delle cartelle restano calcolate su richiesta (FE-DEC-047).

Da definire: rappresentazione di valori assenti, non ancora calcolati o non applicabili, larghezze iniziali e gestione di nomi e valori lunghi.

### 11.10. Ordinamento iniziale — decisione FE-DEC-054

**Stato:** valori predefiniti confermati dall'utente.

Per le cartelle senza preferenze salvate:

- Cartelle prima dei file.
- Ordinamento per nome crescente.
- Ordinamento naturale dei numeri, per esempio `file2` prima di `file10`.
- Nessun raggruppamento iniziale.

L'utente può cambiare criterio e direzione dell'ordinamento, scegliere di mescolare file e cartelle e attivare raggruppamenti, per esempio per tipo o data. Le scelte vengono ricordate per cartella.

Da definire: criteri e raggruppamenti disponibili, ordinamento di valori assenti, regole locali per confronto dei nomi e comportamento durante aggiornamenti del contenuto.

## 12. Interazione e accessibilità

### 12.1. Scorciatoie da tastiera — decisione FE-DEC-055

**Stato:** inclusione nel primo rilascio confermata dall'utente, scegliendo l'opzione A.

Le scorciatoie hanno valori iniziali coerenti con le convenzioni del sistema operativo e sono personalizzabili nelle Preferenze, con rilevamento dei conflitti e ripristino dei valori originali.

La navigazione e le operazioni principali sono utilizzabili da tastiera.

Da definire: elenco e mappatura dei comandi per piattaforma, combinazioni riservate dal sistema e gestione del focus nei diversi contesti. Il perimetro generale di interfaccia e accessibilità è confermato in FE-DEC-058.

## 13. Filesystem, collegamenti e archivi — decisione FE-DEC-056

**Stato:** perimetro del primo rilascio confermato dall'utente nel blocco conclusivo di analisi 1/6.

### Collegamenti simbolici

- Riconoscibili tramite un indicatore e apribili per raggiungere la destinazione.
- Nelle copie si conserva il collegamento, anziché duplicare implicitamente il contenuto puntato, dove supportato.
- Nessuna esplorazione ricorsiva incontrollata dei collegamenti; evitare cicli.

### Permessi

- Rispetto dei permessi del sistema operativo.
- Accessi negati segnalati chiaramente, applicando le regole concordate di gestione degli errori dove pertinenti.
- Nessuna richiesta automatica di privilegi amministrativi; gestione avanzata rimandata.

### Archivi compressi

- Supporto iniziale ZIP: consultazione del contenuto ed estrazione nella destinazione scelta.
- Nessuna scrittura fuori dalla destinazione autorizzata durante l'estrazione e nessuna sovrascrittura implicita.
- Creazione/modifica di archivi e supporto ad altri formati rimandati.

Da definire nella progettazione: rappresentazione e gestione dei collegamenti specifici delle piattaforme, limiti di espansione degli archivi, ZIP cifrati o non validi, collegamenti contenuti negli archivi e prevenzione degli attraversamenti della destinazione anche tramite link. Le verifiche tecniche non introducono nuove capacità approvate per il primo rilascio.

## 14. Ricerca semplice e ricerche salvate — decisione FE-DEC-057

**Stato:** base funzionale confermata dall'utente nel blocco conclusivo di analisi 2/6.

### Criteri

- Nome, percorso, estensione e tipo.
- Dimensioni e date, dove disponibili.
- Tag e testo delle note.
- Parole o frasi nei contenuti dei formati supportati, senza dipendere da un LLM.

### Ambito e filtri

- Cartella corrente, con scelta se includere le sottocartelle, oppure una o più cartelle selezionate.
- Filtri combinabili tramite interfaccia con condizioni AND/OR ed esclusioni, senza obbligare a scrivere una sintassi.
- Distinzione visibile tra ricerca completata, ancora in corso ed elementi non analizzabili.

### Ricerche salvate

- Conservano richiesta, filtri e ambito, non un elenco statico di file.
- Richiamabili dalla barra laterale, rinominabili ed eliminabili.
- Risultati ricalcolati quando vengono aperte o aggiornate.
- Nessuna indicizzazione vettoriale obbligatoria per usare la ricerca semplice.

La matrice dei formati ricercabili resta da definire separatamente dalle anteprime. Restano inoltre da progettare indicizzazione testuale o scansione, aggiornamento dei dati, prestazioni, semantica precisa dei filtri e comportamento sui contenuti non disponibili. Queste decisioni non riaprono il perimetro AI, documentato separatamente.

## 15. Interfaccia e accessibilità — decisione FE-DEC-058

**Stato:** perimetro del primo rilascio confermato dall'utente nel blocco conclusivo di analisi 3/6.

- Tema chiaro, scuro e automatico; impostazione iniziale che segue il sistema.
- Italiano e inglese, con struttura estendibile ad altre lingue. Lingua iniziale del sistema se supportata, altrimenti inglese.
- Preferenze di tema e lingua modificabili e persistenti.
- Interfaccia adattabile al ridimensionamento della finestra e alla scala dello schermo, senza una risoluzione unica obbligatoria.
- Navigazione completa da tastiera, focus visibile ed etichette compatibili con gli screen reader.
- Informazioni non comunicate soltanto tramite colore: errori, tag e stati hanno anche indicazioni testuali o simboliche.
- Rispetto della preferenza di sistema per ridurre le animazioni.

L'accessibilità richiede verifiche esplicite e non è considerata garantita dalla sola scelta di React e Tauri. Restano da definire scenari di verifica, combinazioni di piattaforme e tecnologie assistive, dimensioni minime delle finestre e criteri di accettazione misurabili. Non è stata dichiarata una certificazione o conformità formale già verificata.

## 16. Esportazione, importazione e backup — decisione FE-DEC-059

**Stato:** esportazione/importazione non prioritarie, rimandate dopo il chiarimento nel blocco conclusivo di analisi 4/6.

L'utente non considera fondamentali in questo momento le funzionalità di esportazione/importazione manuale dei dati di FolderAI. Non vengono quindi incluse nel perimetro iniziale confermato; potranno essere riprese in futuro.

La proposta precedente comprendeva anche backup prima delle migrazioni, gestione dei conflitti di importazione e trattamento delle credenziali. La risposta non viene interpretata come approvazione dell'intero insieme né come decisione di rinunciare alla protezione dei dati. I dettagli di backup, conservazione e recupero restano da progettare, senza aggiungere ora una gestione backup rivolta all'utente.

Resta la distinzione già analizzata tra dati dell'utente non rigenerabili (note, tag e preferiti), cache/indici ricostruibili e documenti originali sul filesystem. Un eventuale ripristino dei dati applicativi non equivale al ripristino dei documenti originali.

## 17. Scala e hardware — decisione FE-DEC-060

**Stato:** indirizzo confermato dall'utente nel blocco conclusivo di analisi 5/6; soglie prestazionali non ancora definite.

- **Volume complessivo:** considerare anche centinaia di migliaia di file, distribuiti nella gerarchia delle cartelle.
- **Elementi direttamente contenuti in una cartella:** decine di migliaia di figli diretti non rappresentano lo scenario ordinario atteso dall'utente. Non è stata definita una soglia numerica né autorizzato un limite rigido al numero di elementi navigabili.
- **8 GB di RAM:** supporto desiderato, se possibile, ma non requisito stretto tale da ridurre funzionalità o qualità.
- **16 GB di RAM:** riferimento orientativo principale per un computer moderno nella discussione; non costituisce un minimo obbligatorio né un budget di memoria già validato.
- Il riferimento riguarda l'esplora risorse senza modelli AI caricati. I consumi dei modelli locali e delle pipeline AI devono essere valutati separatamente.

Da definire nella progettazione e validazione: archivi campione, distribuzione degli elementi tra cartelle, hardware completo, concorrenza dei lavori, consumi e tempi misurabili. Queste verifiche non devono trasformare l'obiettivo di compatibilità con 8 GB in una riduzione implicita del perimetro funzionale.

## 18. Conferma del perimetro — decisione FE-DEC-061

**Stato:** perimetro complessivo confermato dall'utente nel blocco conclusivo di analisi 6/6.

Per la parte file explorer del primo rilascio:

- Navigazione con schede, doppio pannello e finestre multiple.
- Viste dettagli, griglia e colonne; anteprime e miniature.
- Operazioni manuali con gestione sicura di errori, conflitti, cancellazioni e ultimo annullamento.
- Preferiti con gruppi, tag e note Markdown.
- Ricerca semplice con filtri e ricerche salvate.
- Collegamenti simbolici, consultazione ed estrazione ZIP.
- Preferenze, scorciatoie personalizzabili, temi, italiano/inglese e accessibilità.

Rimandati: dischi esterni/rete, rinomina in blocco, editor dei documenti, anteprime Office/OpenDocument, creazione/modifica archivi, altri formati compressi ed esportazione/importazione.

Ancora da analizzare: criterio di associazione delle applicazioni predefinite, matrice dei formati ricercabili e dettagli tecnici già tracciati. L'AI resta nel documento separato `03-ai-integration.md`: questa conferma non attribuisce automaticamente ogni proposta AI al primo rilascio.

Si conclude la raccolta iniziale delle decisioni funzionali del file explorer. Il riepilogo `04-analysis-summary.md` distingue perimetro, rinvii e punti aperti; la successiva formalizzazione dei requisiti dovrà mantenere queste distinzioni.

