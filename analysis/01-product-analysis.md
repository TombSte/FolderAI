# FolderAI — Analisi preliminare del prodotto

**Stato:** documento di contesto iniziale aggiornato dopo la prima analisi. Perimetro del file explorer confermato in FE-DEC-061; riepilogo corrente in `04-analysis-summary.md`. Restano distinti indirizzi AI, proposte e decisioni ancora aperte; i requisiti formali non sono ancora redatti.

## 1. Contesto e obiettivo

FolderAI è un nuovo progetto open source: un esplora risorse sviluppato in Rust con funzionalità di agentic RAG.

Il repository contiene le directory `analysis/`, `requirements/`, `docs/` e `src/`. Al momento non sono presenti codice, configurazione Cargo o vincoli architetturali documentati.

Obiettivo confermato: realizzare una vera alternativa ai file manager di sistema, non soltanto un'applicazione di ricerca e consultazione AI. Il prodotto deve consentire all'utente di esplorare e gestire i propri file, trovarli anche attraverso il loro significato e interrogarne i contenuti tramite un assistente che possa pianificare ricerche e consultare le fonti. L'ampiezza del prodotto non implica parità con tutte le funzionalità dei file manager esistenti. Il perimetro del file explorer per il primo rilascio è stato confermato in `02-file-explorer-and-local-data.md`, FE-DEC-061.

Vincoli tecnologici confermati dall'utente: applicazione desktop multipiattaforma con Tauri 2, backend Rust e UI React + TypeScript + SCSS. L'obiettivo dichiarato per la UI è un'esperienza moderna con ampio controllo sulla sua realizzazione. Le differenze tra piattaforme devono essere gestite attraverso opportune astrazioni; collocazione e contratti di queste astrazioni saranno definiti nell'analisi architetturale.

## 2. Distinzione tra le capacità

- **Esplorazione tradizionale:** navigazione del filesystem, metadati e operazioni su file e cartelle.
- **Ricerca lessicale:** ricerca per nome e, se prevista, per parole presenti nei contenuti.
- **Ricerca semantica:** recupero di documenti o passaggi affini al significato della richiesta, anche senza corrispondenze testuali esatte.
- **RAG:** generazione di risposte basate su contenuti recuperati, con riferimenti verificabili ai file sorgente.
- **Agentic RAG:** il sistema sceglie strumenti e passi di ricerca, riformula le query, consulta altre fonti e verifica se dispone di informazioni sufficienti prima di rispondere.
- **Agente operativo:** esegue azioni sul filesystem. È una capacità distinta dall'agentic RAG e non necessariamente inclusa nel primo rilascio.

La ricerca semantica può funzionare senza generazione di risposte; un agente di ricerca può funzionare senza permessi di modifica dei file.

## 3. Utenti e scenari da validare

### Indicazioni confermate dall'utente

- **Destinatari multipli:** utenti generalisti, sviluppatori e professionisti; non è stato individuato un unico target principale.
- **Esperienza desiderata:** comoda per gli utenti comuni, ma potente e personalizzabile per sviluppatori e professionisti.
- **Valore prioritario:** semplificare la ricerca avanzata di contenuti all'interno di migliaia di file.
- **Autonomia iniziale degli agenti:** accesso in sola lettura. Creazione, modifica e altre operazioni sui file da parte degli agenti sono rimandate a una fase successiva. Questo limite riguarda gli agenti; è stata confermata separatamente una base di operazioni manuali, descritta in `02-file-explorer-and-local-data.md`, sezione 7.
- **Ampiezza della ricerca:** supportare molteplici modalità di ricerca sulla maggior parte delle tipologie di file testuali e visuali. Le modalità specifiche e la matrice dei formati per il primo rilascio sono ancora da definire.
- **Indice vettoriale:** database vettoriale locale contenente gli embedding dei file nelle cartelle configurate. Granularità degli embedding (file, passaggi o altre unità) e metadati saranno approfonditi successivamente.
- **Posizione delle sorgenti:** nel primo rilascio non è richiesto il supporto a dischi esterni o cartelle di rete; entrambi sono rimandati a una fase futura.
- **Perimetro del prodotto:** vera alternativa a Finder/Esplora file, non semplice interfaccia di ricerca AI. Le operazioni manuali e i comportamenti attesi saranno analizzati separatamente dalle capacità degli agenti, che restano in sola lettura nel primo rilascio.
- **Riferimenti di esperienza:** Finder su macOS ed Esplora file su Windows. Le decisioni raccolte su navigazione, visualizzazione, operazioni manuali e integrazione con il sistema sono tracciate in `02-file-explorer-and-local-data.md`.
- **Persistenza aggiuntiva proposta dall'utente:** valutare un piccolo database SQL locale per metadati aggiuntivi, preferiti e altri dati dell'applicazione. È una proposta da analizzare, non ancora una scelta di motore né uno schema approvato.
- **Organizzazione dei contenuti confermata nel primo rilascio:** preferiti per file e cartelle con gruppi facoltativi, tag multipli senza ereditarietà per file e cartelle, note Markdown con salvataggio automatico e ricerche salvate dinamiche. Dettagli nel documento 2.
- **Navigazione e disposizione confermate:** pannello singolo come impostazione iniziale, schede per più posizioni aperte, doppio pannello opzionale, barra laterale configurabile con preferiti/cartelle/tag/ricerche salvate e barra del percorso a breadcrumb modificabile come percorso testuale.
- **Viste confermate:** dettagli, griglia e colonne stile Finder. L'utente ha scelto tutte e tre, senza rimandare quella a colonne; comportamenti specifici e pianificazione complessiva dei rilasci restano da definire.
- **Anteprima e informazioni confermate:** pannello laterale opzionale condiviso, organizzato in sezioni per anteprima del file e informazioni quali metadati, tag, note e stato di indicizzazione. Anteprime iniziali per testo/codice, Markdown, immagini comuni e PDF; riepilogo per selezioni multiple. Dettagli nel documento 2, FE-DEC-049–051.
- **Preferenze per cartella confermate:** ricordare vista, ordinamento, raggruppamento e colonne visibili.
- **Operazioni manuali confermate come base:** creazione di file vuoti/cartelle, copia, spostamento, rinomina, eliminazione, selezione multipla, appunti, drag-and-drop, apertura con applicazioni, gestione dei conflitti e centro attività. Cestino predefinito, eliminazione permanente esplicita e confermata, nessuna promessa di ripristino universale. Rinomina in blocco rimandata a una fase successiva; nel primo rilascio rinomina di un elemento alla volta.
- **Applicazioni predefinite:** richiesto un cambio intuitivo e immediato dell'applicazione usata per aprire i file. L'utente ha confermato che la preferenza si applica solo alle aperture effettuate da FolderAI. Vedere la sezione 8 del documento 2.
- **Integrazione AI condivisa:** ricerca unificata/ibrida, assistente contestuale, agente di ricerca con fonti, ricerche salvate dinamiche, collegamenti tra contenuti e controllo dell'utente. L'utente sottolinea la separazione tra ricerca semplice, semantica e agentica e conferma la coesistenza delle tre modalità manuali con una modalità Auto. È inoltre in valutazione l'assegnazione di modelli diversi a ruoli specializzati (routing, OCR/visione, agentica). L'adesione riguarda l'indirizzo di prodotto; comportamenti di dettaglio e priorità restano da definire in `03-ai-integration.md`.

La ricerca nei contenuti è il caso d'uso prioritario. La scala attesa comprende anche centinaia di migliaia di file distribuiti tra cartelle; 16 GB di RAM sono un riferimento orientativo e il supporto a 8 GB è desiderato senza ridurre funzionalità o qualità. Non sono soglie prestazionali già verificate. Formati ricercabili e criteri misurabili restano da definire; personalizzazioni e ricerca semplice sono dettagliate nel documento 2.

### Scenari proposti

Scenari rappresentativi:

1. Navigare una cartella, visualizzare i metadati e aprire un file nell'applicazione associata.
2. Cercare «il documento che descrive le penali del contratto» senza ricordarne il nome.
3. Chiedere una sintesi di più documenti selezionati, con rimandi alle fonti.
4. Chiedere all'agente di trovare informazioni distribuite in più file, eseguendo ricerche successive quando necessario.
5. Aggiornare i risultati quando un file indicizzato viene modificato, spostato o eliminato.
6. Eventualmente proporre una riorganizzazione di file e mostrarne l'anteprima prima di eseguirla.

Questi scenari sono proposte, non requisiti approvati.

## 4. Mappa funzionale proposta

| Area | Capacità candidate | Collocazione proposta |
| --- | --- | --- |
| Esplorazione | Navigazione, cronologia, ordinamento, selezione, metadati, apertura file | MVP |
| Gestione file | Creazione file vuoti/cartelle, copia, spostamento, rinomina singola, cestino, gestione sicura e ultimo annullamento | Primo rilascio confermato; dettagli nel documento 2 |
| Ricerca tradizionale | Nome, percorso, metadati, tag, note, testo nei formati supportati, filtri e ricerche salvate | Primo rilascio confermato; matrice dei formati ancora aperta |
| Sorgenti AI | Scelta esplicita delle cartelle indicizzate, esclusioni e rimozione dall'indice | MVP |
| Acquisizione | Supporto ampio a contenuti testuali e visuali; estrazione e modalità di elaborazione da definire per formato | Obiettivo confermato; formati e capacità del primo rilascio da concordare |
| Indicizzazione | Database vettoriale locale con embedding dei file nelle cartelle configurate; granularità, aggiornamenti incrementali e stato visibile da dettagliare | Database locale ed embedding confermati; dettagli proposti per MVP |
| Ricerca semantica | Risultati con percorso, estratto e apertura della fonte | MVP |
| Risposte RAG | Domande su file/cartelle selezionati, citazioni, segnalazione di evidenza insufficiente | MVP |
| Agentic RAG | Ricerche iterative e lettura mirata, strumenti limitati e budget di esecuzione | Sola lettura confermata per il primo rilascio; dettagli da concordare |
| Controllo utente | Annullamento delle attività AI, stato di avanzamento e riepilogo delle operazioni | MVP |
| Anteprime | Sola lettura per testo/codice, Markdown, immagini comuni e PDF | Primo rilascio confermato |
| Operazioni agentiche | Creazione, modifica, rinomina, spostamento, eliminazione e altre operazioni sui file | Rimandate a una fase successiva, come confermato dall'utente; modalità da definire |
| Contenuti avanzati | OCR, analisi di immagini, audio e archivi | Collocazione da definire nell'ambito del supporto a file testuali e visuali |
| Sorgenti esterne | Dischi esterni e cartelle di rete | Fase futura confermata |
| Altre estensioni | Integrazioni cloud, plugin | Successivo salvo priorità diverse |

Per il file explorer, la tabella va letta insieme al perimetro confermato FE-DEC-061 e al riepilogo `04-analysis-summary.md`. Le collocazioni MVP delle capacità AI restano proposte da consolidare separatamente; non sono approvate automaticamente dalla conferma del file explorer.

## 5. Flusso agentic RAG proposto

1. L'utente sceglie l'ambito della domanda: file selezionati o cartelle autorizzate.
2. Il sistema identifica gli strumenti utili: ricerca per metadati, lessicale, semantica e lettura di passaggi.
3. Recupera evidenze e, se necessario, esegue ulteriori ricerche entro limiti espliciti.
4. Genera una risposta collegando le affermazioni alle fonti consultate.
5. Se le evidenze non bastano, esplicita i limiti o chiede chiarimenti.

Una citazione deve consentire di aprire il file e individuare il passaggio, tramite pagina, righe o altro riferimento adatto al formato. Una citazione non garantisce di per sé che la risposta sia corretta: la fedeltà alle fonti dovrà essere verificata.

## 6. Vincoli e rischi da trasformare in requisiti

- **Privacy:** l'utente configura nelle impostazioni il consenso all'invio verso provider remoti; la modalità «a ogni richiesta» richiede un'autorizzazione ogni volta. Chiarire quali contenuti, query o embedding possano lasciare il dispositivo, le altre modalità di consenso e la gestione delle chiamate multiple di un agente.
- **Autorizzazioni:** distinguere le cartelle navigabili da quelle autorizzate per indicizzazione e accesso degli agenti.
- **Prompt injection:** i contenuti dei file sono dati non attendibili, non istruzioni che possano ampliare i privilegi dell'agente.
- **Sicurezza delle operazioni:** separare lettura e modifica; definire conferme, gestione dei conflitti e recuperabilità prima di abilitare azioni agentiche.
- **Coerenza:** gestire file modificati, eliminati o diventati inaccessibili senza presentare informazioni obsolete come attuali.
- **Filesystem:** definire comportamento per link simbolici, permessi insufficienti, file grandi, directory nascoste, volumi rimovibili e percorsi fuori ambito.
- **Risorse:** mantenere reattiva la navigazione durante estrazione, embedding e inferenza; prevedere limiti e annullamento dei lavori.
- **Affidabilità AI:** prevenire risposte prive di riscontro e misurare qualità del recupero, pertinenza delle citazioni e comportamento quando manca una risposta.
- **Costi e latenza:** limitare chiamate, iterazioni e token degli agenti; rendere comprensibili gli eventuali costi esterni.
- **Disponibilità:** tutte le funzionalità AI, inclusa l'indicizzazione, devono essere utilizzabili offline tramite modelli locali su LM Studio o altri engine. Definire i prerequisiti locali (modelli e dipendenze già disponibili), le capacità richieste ai modelli e il comportamento quando un engine o un modello non è disponibile; non presumere che qualsiasi modello supporti tutti gli strumenti.

## 7. Decisioni aperte

1. **Destinatari e valore — parzialmente chiarito:** prodotto multi-target, comodo per utenti comuni e potente/personalizzabile per sviluppatori e professionisti; priorità alla ricerca nei contenuti di migliaia di file. Da approfondire scenari concreti e personalizzazioni attese.
2. **Piattaforme e interfaccia — indirizzo confermato:** supporto multipiattaforma, con differenze gestite tramite astrazioni; applicazione desktop Tauri 2, backend Rust e UI React + TypeScript + SCSS. Da dettagliare matrice di sistemi operativi, versioni e architetture supportate.
3. **Modelli, autenticazione e privacy — indirizzo confermato:** supportare modelli locali e remoti; integrazioni iniziali con API OpenAI e Anthropic tramite API key, anche con provider terzi come OpenRouter; supportare inoltre l'autenticazione associata alle subscription OpenAI e Anthropic, con estendibilità futura ad altre subscription, ad esempio Google e Mistral. Il supporto alle subscription è una funzionalità richiesta, la cui fattibilità, disponibilità e modalità autorizzata d'integrazione vanno verificate per ciascun provider. Tutte le funzionalità AI, inclusa l'indicizzazione, devono poter funzionare offline con modelli locali tramite LM Studio o altri engine. Il consenso all'invio di dati a provider remoti è configurabile nelle impostazioni: se l'utente sceglie l'autorizzazione a ogni richiesta, questa deve essere domandata ogni volta. Restano da dettagliare le altre modalità di consenso, la granularità delle autorizzazioni e la gestione dei provider per generazione ed embedding.
4. **Autonomia — confermato:** agenti in sola lettura nel primo rilascio. Creazione, modifica e altre operazioni sui file da parte degli agenti saranno affrontate successivamente; autorizzazioni e modalità di conferma non sono ancora definite.
5. **Dati e ricerca — indirizzo consolidato:** ricerca semplice, semantica, agentica e Auto; database vettoriale locale per le cartelle configurate. Scala attesa fino a centinaia di migliaia di file complessivi. Dischi esterni e cartelle di rete rimandati. Restano da definire matrice dei formati ricercabili/indicizzabili, capacità OCR/multimodali, distribuzione dei volumi e soglie di qualità/prestazione.
6. **Ampiezza dell'esplora risorse — perimetro iniziale confermato:** vera alternativa ai file manager di sistema; primo rilascio e rinvii documentati in FE-DEC-061 e `04-analysis-summary.md`, senza presumere parità immediata con ogni funzione di Finder o Esplora file.

Lo stack applicativo Tauri 2 + Rust + React + TypeScript + SCSS è stato scelto dall'utente. Le famiglie iniziali di provider e i metodi di autenticazione richiesti sono indicati al punto 3; modelli specifici, runtime locali, database e motore di ricerca restano da scegliere. Le ulteriori decisioni tecniche seguiranno l'analisi dei requisiti.

### Aspetti delle integrazioni AI da approfondire

- Distinguere provider, protocollo API, endpoint, modello e metodo di autenticazione: la sola disponibilità di un'API key non garantisce compatibilità tra servizi.
- Distinguere l'accesso API a consumo dall'accesso tramite subscription: un abbonamento a un prodotto non implica automaticamente accesso API o disponibilità di un flusso di autenticazione per applicazioni terze.
- Verificare per ogni integrazione con subscription i meccanismi ufficialmente supportati, i termini applicabili, le capacità esposte e i limiti; non considerare già acquisita l'equivalenza con le API.
- Prevedere l'estendibilità a ulteriori provider e metodi di autenticazione senza vincolare il dominio a OpenAI o Anthropic; i contratti architetturali saranno definiti successivamente.
- Chiarire separatamente i requisiti dei modelli di generazione e di embedding: non tutti i provider o abbonamenti espongono entrambe le capacità.
- Definire conservazione sicura delle credenziali, revoca e disconnessione, evitando segreti in log o configurazioni esportate.

## 8. Organizzazione documentale proposta

- `analysis/`: analisi del problema, scenari, alternative e decisioni ancora aperte. Approfondimenti: `02-file-explorer-and-local-data.md` per file explorer e dati locali; `03-ai-integration.md` per integrazione AI e modalità di ricerca; `04-analysis-summary.md` come riepilogo corrente di decisioni, rinvii e punti aperti.
- `requirements/`: requisiti concordati, numerati e verificabili; perimetro MVP e criteri di accettazione.
- `docs/`: architettura, decisioni tecniche e documentazione di utilizzo.
- `src/`: implementazione; la struttura Rust sarà definita in una fase successiva.

Dopo aver chiarito le decisioni aperte, produrre i requisiti con identificativo, descrizione, priorità, scenario di riferimento e criterio di accettazione. I requisiti non funzionali dovranno includere condizioni e soglie concordate, evitando formule non verificabili come «veloce» o «sicuro».

## 9. Percorso proposto per l'analisi approfondita

Le risposte iniziali definiscono l'indirizzo del prodotto, ma non esauriscono l'analisi. Prima di formalizzare i requisiti, approfondire nell'ordine:

1. **File explorer:** scenari quotidiani, navigazione, viste, schede/pannelli, operazioni manuali, conflitti, cestino e recupero dagli errori.
2. **Ricerca:** modalità lessicali, semantiche e visuali, filtri, ambito, combinazione dei risultati e collegamento alle fonti.
3. **Contenuti e indicizzazione:** matrice dei formati, estrazione/OCR/multimodalità, esclusioni, aggiornamenti, ciclo di vita dell'indice e volumi attesi.
4. **Assistente e agentic RAG:** domande su selezioni e cartelle, strumenti di sola lettura, iterazioni, citazioni, limiti e annullamento.
5. **Modelli e privacy:** runtime locali, provider remoti, autenticazione, capacità richieste, consenso e protezione delle credenziali.
6. **Esperienza multi-target:** semplicità iniziale, funzionalità avanzate, personalizzazioni, accessibilità e scorciatoie.
7. **Multipiattaforma e qualità:** comportamenti comuni e specifici, limiti del filesystem, sicurezza, prestazioni e distribuzione.
8. **Sintesi e priorità:** distinguere obiettivi complessivi, primo rilascio ed evoluzioni; solo dopo redigere i requisiti verificabili.

Questo percorso è una proposta di lavoro, non una scelta architetturale né un elenco di funzionalità già approvate.
