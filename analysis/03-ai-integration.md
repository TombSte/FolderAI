# FolderAI — Integrazione AI e modalità di ricerca

**Stato:** perimetro AI iniziale confermato dall'utente in AI-DEC-001, sezione 11. Restano da definire comportamenti di esecuzione, limiti, formati, consenso e criteri di accettazione. Le decisioni UI successive sono raccolte in `05-ui-design.md`; le proposte storiche qui sotto vanno lette distinguendole dalle conferme. I quattro punti di attenzione sulla configurazione multi-modello sono tracciati nella sezione 10.

## 1. Indirizzo condiviso

L'utente concorda con le idee di integrazione AI proposte, in particolare con la separazione tra ricerca semplice, semantica e agentica. Conferma inoltre la coesistenza delle tre modalità selezionabili manualmente con una modalità Auto. Propone di valutare più modelli specializzati, per esempio un modello rapido per il routing, modelli per OCR e modelli per l'attività agentica; questa possibilità è ancora in discussione.

Capacità condivise a livello di prodotto:

- Ricerca unificata che combina nomi, metadati, contenuti testuali e, per i formati da concordare, contenuti visuali.
- Assistente contestuale per riassumere, interrogare e confrontare file selezionati.
- Agente di ricerca capace di consultazioni iterative, con fonti e limiti espliciti.
- Ricerche salvate dinamiche, distinguendo filtri, ricerca semantica e analisi agentiche.
- Collegamenti tra contenuti: documenti correlati, possibili versioni, duplicati e suggerimenti di tag.
- Controllo dell'utente su ambito, dati inviati, provider, tempi e costi.

Questa condivisione non equivale a includere ogni capacità nel primo rilascio né a scegliere motori, modelli o schemi dati.

## 2. Tre modalità con aspettative diverse

La tabella seguente propone un contratto funzionale da discutere per rendere concreta la separazione confermata.

| Aspetto | Semplice | Semantica | Agentica |
| --- | --- | --- | --- |
| Obiettivo | Trovare corrispondenze e applicare criteri espliciti | Trovare contenuti pertinenti per significato | Risolvere una domanda attraverso più passi di ricerca e lettura |
| Esempio | PDF con «Apollo», modificati dopo una data | Documenti sulle condizioni di rinnovo | Confronta le condizioni di rinnovo dei contratti e segnala le differenze |
| Strumenti | Nome, percorso, metadati, tag, indice testuale e filtri | Embedding, recupero vettoriale, eventuale combinazione con ricerca lessicale e filtri | Ricerca semplice/semantica, lettura mirata, confronto e sintesi |
| Modelli | Nessun LLM necessario per l'esecuzione ordinaria | Modello di embedding o altra rappresentazione semantica compatibile | Modello per pianificazione e sintesi, con orchestrazione limitata |
| Risultato principale | Elenco di file e corrispondenze | Elenco ordinato per pertinenza con evidenze | Risposta o rapporto con citazioni e accesso ai file consultati |
| Lavoro | Criteri di ricerca espliciti, senza pianificazione autonoma | Recupero per similarità, senza un ciclo autonomo di ricerca | Più ricerche e letture entro limiti configurati |
| Disponibilità | Indipendente dalla presenza di un modello generativo | Dipende da indice e capacità semantiche disponibili | Dipende dai modelli e strumenti richiesti per il compito |

La ricerca semplice nei contenuti può comunque richiedere un indice testuale. L'estrazione preliminare da scansioni o immagini può richiedere OCR: il fatto che una query non usi un LLM non rende gratuita la preparazione dei dati.

La ricerca semantica non implica necessariamente generare una risposta. La ricerca agentica può usare anche strumenti deterministici e non deve limitarsi al database vettoriale.

## 3. Recupero ibrido e contenuti visuali

Proposta: combinare ricerca lessicale, filtri e similarità dove utile, senza affidare ogni richiesta esclusivamente agli embedding.

- Nomi esatti, codici e identificativi possono beneficiare della ricerca lessicale.
- Domande per concetto possono beneficiare della similarità semantica.
- Filtri e ambiti espliciti devono conservare il loro significato anche nella ricerca semantica.
- OCR e comprensione visuale sono capacità diverse: trovare testo in uno screenshot non equivale a riconoscere un diagramma o una scena.
- Modelli e rappresentazioni compatibili vanno definiti per ciascun tipo di contenuto. Il supporto locale completo è un obiettivo confermato, non una garanzia che qualunque modello locale gestisca qualsiasi formato.

Restano da definire fusione dei risultati, valutazione della pertinenza, gestione delle corrispondenze insufficienti e matrice dei formati.

## 4. Esperienza di ricerca proposta

Una superficie di ricerca comune può mantenere un'interazione familiare, rendendo però esplicite le tre modalità.

Confermata la disponibilità di tre modalità manuali e di Auto. Dettagli di interazione proposti:

- Selettore visibile «Semplice / Semantica / Agentica / Auto».
- Modalità iniziale Semplice, confermata nell'analisi UI; scelta successiva mantenuta per scheda. Input agentico multilinea nel pannello destro e input solo testuale nella prima fase: dettagli confermati in `05-ui-design.md`.
- Ambito sempre visibile: cartella corrente, selezione o insieme di cartelle autorizzate.
- Nelle modalità manuali, nessun passaggio autonomo a una modalità diversa. In Auto, scelta dell'approccio entro capacità, ambiti e limiti autorizzati, con indicazione della modalità effettivamente utilizzata. Nessun ampliamento implicito dell'ambito o invio remoto non autorizzato.
- Possibilità di suggerire «Cerca anche per significato» oppure «Approfondisci con l'agente», lasciando la scelta all'utente.
- Query in linguaggio naturale eventualmente convertibili in filtri visibili e modificabili. Se la conversione usa un modello, deve essere riconoscibile come assistenza AI, non requisito nascosto della ricerca semplice.

Lo stesso punto di ingresso non impone una presentazione identica: un elenco di file e un rapporto con citazioni hanno esigenze diverse.

## 5. Assistente contestuale e agente

L'assistente può proporre azioni a partire dalla selezione: riassumi, spiega, confronta o fai una domanda.

Un'azione semplice di sintesi non deve necessariamente attivare un ciclo agentico. L'agente serve quando il compito richiede di scegliere strumenti, cercare nuovamente o consultare più fonti.

Proposte di comportamento:

- Dichiarare se l'attività riguarda soltanto la selezione o un ambito più ampio autorizzato.
- Mostrare attività osservabili e fonti consultate, non ragionamenti interni del modello.
- Consentire annullamento e limiti di iterazioni, letture, durata e costi.
- Esplicitare evidenze insufficienti, contraddizioni e parti non analizzate.
- Collegare le affermazioni a fonti apribili, con pagina, righe o riferimento adatto al formato.

Gli agenti restano in sola lettura sui file dell'utente. Suggerimenti automatici di tag e rilevamento proattivo di duplicati/versioni sono successivamente rimandati in AI-DEC-001; in nessun caso le proposte storiche autorizzano ad applicare tag o eliminare file automaticamente.

## 6. Ricerche salvate e organizzazione

Distinzione proposta:

- **Semplici:** conservano criteri e filtri; risultati aggiornati sullo stato disponibile del filesystem e dell'indice.
- **Semantiche:** conservano richiesta, ambito e configurazione semantica rilevante; i risultati possono cambiare al variare dei contenuti, dell'indice o del modello.
- **Agentiche:** conservano il compito e le impostazioni; le nuove esecuzioni sono esplicite, senza costi o invii ricorrenti impliciti.

Documenti correlati, possibili versioni e contenuti simili sono suggerimenti, non fatti certi. I duplicati esatti devono essere distinti dalla semplice somiglianza; l'individuazione dei primi non richiede necessariamente AI.

## 7. Ambito, privacy e affidabilità

- Distinguere navigabilità, autorizzazione all'indicizzazione e accesso durante un'attività AI.
- Distinguere «nessun risultato» da «file non indicizzati», «formato non supportato» o «file non più accessibile».
- Rendere visibili modello/provider e destinazione dei dati; nessun fallback automatico dal locale al cloud.
- Applicare la politica di consenso configurata dall'utente. AI-DEC-007 conferma che «ogni richiesta» è una singola richiesta utente comprensiva delle chiamate necessarie, non una conferma per ogni passo dell'agente; modifiche non coperte richiedono nuova approvazione.
- Considerare anche embedding, query, passaggi estratti e metadati quando si valuta l'invio di dati a un provider remoto.
- Trattare i contenuti recuperati come dati non attendibili, non come istruzioni capaci di ampliare i privilegi dell'agente.
- Non promettere accesso alle subscription come equivalente alle API senza la verifica specifica per provider già prevista nell'analisi preliminare.

## 8. Decisioni da approfondire

1. Interfaccia e selezione delle modalità: struttura, selettore, modalità iniziale e pannello conversazionale definiti in `05-ui-design.md`; restano eventuali suggerimenti di passaggio e casi limite.
2. Ambiti: Globale come cartelle abilitate e Cartella con sottocartelle confermati nell'analisi UI; restano rapporto tra autorizzazioni e indicizzazione, copertura parziale e gestione dei riferimenti puntuali.
3. Filtri, operatori e rappresentazione delle query in linguaggio naturale.
4. Risultati: estratti, citazioni, pertinenza, apertura e passaggio tra elenco e analisi.
5. Attività agentiche: capacità e famiglie di strumenti confermate in AI-DEC-001; restano contratti operativi, limiti, consenso, chiarimenti e comportamento in caso di interruzione.
6. Copertura dei formati e qualità misurabile; perimetro funzionale iniziale confermato in AI-DEC-001, senza approvazione implicita delle altre proposte.

## 9. Auto e specializzazione dei modelli

Il comportamento funzionale di Auto è successivamente confermato in AI-DEC-003, sezione 13. Le soluzioni di routing e specializzazione dei modelli qui sotto restano proposte tecniche, non automaticamente approvate da tale conferma.

### Auto non è un quarto motore di ricerca

Auto seleziona o combina gli approcci semplice, semantico e agentico. Il routing è distinto dalla pianificazione iterativa di un agente.

Proposta: usare prima criteri deterministici dove sufficienti (filtri espliciti, azioni richieste, modalità disponibili), ricorrendo a un modello rapido per richieste ambigue. Non è necessario chiamare un LLM per ogni ricerca per nome. Il termine «System 1» indica qui un ruolo rapido e poco costoso, non un'interfaccia o una capacità garantita da ogni modello.

Un router potrebbe produrre una decisione strutturata: approccio, strumenti richiesti e motivazione sintetica per l'utente. Il modello non decide autorizzazioni, provider consentiti o limiti di costo: questi vincoli sono applicati dall'applicazione. In caso di incertezza o errore, definire un comportamento prevedibile, eventualmente chiedendo chiarimenti.

### Ruoli distinti, modelli riutilizzabili

| Ruolo candidato | Responsabilità | Note |
| --- | --- | --- |
| Routing rapido | Classificare la richiesta e scegliere l'approccio | Regole prima del modello quando sufficienti; decisione da validare su casi rappresentativi |
| Embedding | Indicizzare e rappresentare le query semantiche | Modello/configurazione compatibili con lo spazio vettoriale dell'indice |
| Estrazione/OCR/visione | Estrarre testo o interpretare contenuti visuali | Parser e OCR tradizionale possono bastare; visione multimodale è una capacità distinta |
| Assistente/agente | Sintesi, confronto, pianificazione e uso degli strumenti | Verificare capacità effettive e limiti del runtime, non soltanto il nome del modello |
| Reranking opzionale | Riordinare un insieme di risultati recuperati | Da introdurre solo se migliora la qualità misurata rispetto a costo e latenza |

I ruoli non impongono modelli tutti diversi: un modello può coprirne più di uno se adeguato. Non tutti i modelli di chat producono embedding utilizzabili, supportano immagini o gestiscono strumenti in modo affidabile.

### Configurazione accessibile

La configurazione guidata con sezione avanzata e assegnazione per ruolo è successivamente confermata in AI-DEC-006: embedding, assistente/agente e comprensione visiva, con riuso dello stesso modello se compatibile. Non è necessario obbligare l'utente comune a comprendere l'intera pipeline.

Eventuali profili «Locale», «Bilanciato» o «Qualità» sono proposte, non funzionalità già approvate. Profilo e fallback non possono superare le autorizzazioni dell'utente; la modalità locale non deve introdurre chiamate cloud implicite.

### Vincoli e costi della specializzazione

- Ogni passaggio ha propri tempi, consumi di memoria, eventuali costi e implicazioni di privacy. Caricare più modelli locali può peggiorare l'esperienza su hardware limitato.
- Il routing richiede valutazione: un modello piccolo può essere sufficiente per classificare, ma non è una garanzia di correttezza né un sostituto dei controlli deterministici.
- Cambiare modello/configurazione di embedding può richiedere una nuova indicizzazione. Vettori di spazi incompatibili non devono essere mescolati, anche se hanno la stessa dimensione.
- Gli output di OCR ed estrazione dovrebbero mantenere provenienza e versione, permettendo di individuare dati obsoleti senza perdere metadati dell'utente.
- I fallback vanno esplicitati: un modello remoto alternativo può cambiare destinatario dei dati, costo e capacità. Nessun fallback cloud silenzioso.
- Il primo rilascio dovrebbe evitare una pipeline obbligatoriamente composta da molti modelli: partire dai ruoli necessari e introdurre specializzazioni sulla base di benefici misurabili.

La disponibilità di più provider già richiesta rende sensata questa direzione, ma non costituisce ancora approvazione di ogni ruolo, profilo o comportamento proposto.

## 10. Registro dei punti di attenzione confermati

L'utente concorda sui quattro punti seguenti e ne richiede il tracciamento. Nascono come vincoli e rischi riconosciuti nell'analisi; le formalizzazioni successive sono collegate esplicitamente, senza considerare i rischi già risolti. In particolare AI-ATT-002 è approfondito in AI-DEC-005 e `requirements/embedding.md`. Gli identificativi consentono di collegare decisioni architetturali, requisiti e verifiche.

Le misure e le verifiche riportate sono proposte da approfondire; soglie, strumenti e criteri di accettazione non sono ancora concordati. Nessun punto è considerato risolto o verificato.

### AI-ATT-001 — Risorse e prestazioni multi-modello

- **Punto confermato:** più modelli non garantiscono prestazioni migliori. In locale possono competere per RAM/VRAM e introdurre caricamenti ripetuti, latenza o indisponibilità.
- **Misure proposte:** consentire il riuso di un modello per più ruoli, evitare specializzazioni obbligatorie e valutare caricamento, concorrenza e limiti di risorse.
- **Verifica proposta:** confrontare configurazioni essenziali e specializzate su hardware e archivi rappresentativi, misurando qualità, latenza, memoria e reattività dell'esplora risorse.
- **Da definire:** hardware di riferimento, scenari di carico, budget e politica di gestione dei modelli.
- **Stato:** aperto; punto confermato, mitigazioni da definire e validare.

### AI-ATT-002 — Compatibilità tra embedding e indice

- **Punto confermato:** l'indice dipende dal modello e dalla configurazione di embedding. Un cambio può richiedere reindicizzazione; vettori di spazi incompatibili non devono essere mescolati, anche se hanno la stessa dimensione.
- **Vincoli successivamente confermati:** interfaccia astratta dell'engine fin dalla prima implementazione; identificatore/versione per modello e indice dedicato; isolamento e controlli di compatibilità degli spazi. Specifica: AI-DEC-005 e `requirements/embedding.md`.
- **Ulteriori misure da progettare:** descrittore completo della configurazione semantica e gestione della ricostruzione senza perdere preferiti o metadati utente.
- **Verifica proposta:** simulare cambi di modello e configurazione, includendo spazi incompatibili con uguale dimensionalità e interruzioni della reindicizzazione.
- **Da definire:** descrittore di compatibilità, strategia di migrazione, disponibilità della ricerca durante la ricostruzione e costi comunicati all'utente.
- **Stato:** aperto; punto confermato, mitigazioni da definire e validare.

### AI-ATT-003 — Affidabilità del routing Auto

- **Punto confermato:** un modello rapido può scegliere l'approccio sbagliato; non sostituisce controlli deterministici, autorizzazioni o limiti applicativi.
- **Misure proposte:** usare regole nei casi espliciti, validare le decisioni strutturate del router e gestire ambiguità, output non validi e indisponibilità del modello con un comportamento prevedibile.
- **Verifica proposta:** preparare richieste rappresentative delle tre modalità, casi ambigui e richieste fuori ambito; misurare errori di selezione e conseguenze su qualità, tempi e costi, verificando che il routing non ampli i privilegi.
- **Da definire:** insieme di valutazione, tolleranze, gestione dell'incertezza e condizioni in cui chiedere chiarimenti.
- **Stato:** aperto; punto confermato, mitigazioni da definire e validare.

### AI-ATT-004 — Nessun fallback cloud silenzioso

- **Punto confermato:** nessun passaggio automatico e non autorizzato dal locale al cloud, inclusi routing, OCR/visione, embedding e agentica. Il consenso configurato dall'utente resta vincolante.
- **Misure proposte:** applicare i controlli di destinazione e consenso a ogni passaggio remoto; rendere espliciti provider alternativi e possibili differenze di costo, capacità e dati inviati.
- **Verifica proposta:** simulare indisponibilità dei componenti locali e fallback, controllando che non avvengano chiamate remote non autorizzate e che il comportamento offline sia rispettato.
- **Consenso successivamente confermato:** modalità Solo locale iniziale, Chiedi a ogni richiesta utente e Consenti ai provider selezionati; autorizzazione vincolata a provider, ruolo, ambito e categorie di dati, come AI-DEC-007.
- **Da definire:** dettagli del consenso per lavori di indicizzazione continuativi, gestione del rifiuto, revoca durante attività e presentazione delle alternative autorizzabili.
- **Stato:** aperto; punto confermato, mitigazioni da definire e validare.

## 11. Perimetro AI del primo rilascio — AI-DEC-001

**Stato:** confermato esplicitamente dall'utente con «si concordo, confermo il perimetro iniziale». È una decisione funzionale, non una scelta di implementazione o una validazione tecnica.

### Capacità incluse

- **Ricerca semantica:** documenti pertinenti per significato, con estratti; non richiede necessariamente una risposta generata.
- **Ricerca agentica:** ricerche e letture successive per risolvere una richiesta articolata nell'ambito scelto.
- **Domande sui contenuti:** risposte basate sui documenti nell'ambito autorizzato, con citazioni.
- **Riassunto:** sintesi di uno o più documenti, segnalando parti non analizzate.
- **Confronto:** differenze, punti comuni e contraddizioni collegati alle fonti.
- **Estrazione mirata:** raccolta delle informazioni richieste, ad esempio date, importi, obblighi e scadenze, anche in tabelle nella risposta. Non implica creazione automatica di file esportati.

### Strumenti e confini

- Esplorazione delle cartelle autorizzate.
- Ricerca per nome, percorso, metadati e contenuto.
- Ricerca semantica quando disponibile.
- Lettura mirata dei documenti supportati e consultazioni successive.
- Input solo testuale nella prima fase, ambito esplicito e agenti in sola lettura, come già confermato nell'analisi UI.
- Nessuna esecuzione di comandi di terminale, navigazione web o modifica dei file da parte dell'agente. Il comando manuale «Apri nel terminale» è indipendente.
- L'accesso ai provider dei modelli non equivale a navigazione web dell'agente. Restano vincolanti consenso configurato, funzionamento locale/offline e divieto di fallback cloud silenzioso.

### Rinvii confermati

- Suggerimenti automatici di tag.
- Rilevamento proattivo di duplicati e versioni.
- Analisi periodiche avviate senza richiesta esplicita.
- Input tramite immagini in fase due e operazioni agentiche sui file, già rimandati in precedenza.

Il rinvio dell'input immagini non stabilisce automaticamente il perimetro di OCR o ricerca nel contenuto delle immagini: dipende dalla matrice dei formati ancora da definire. Non dedurre l'inclusione di tutte le altre capacità proposte nelle sezioni storiche.

### Ancora da definire

L'autonomia controllata è successivamente confermata in AI-DEC-002 e il comportamento funzionale di Auto in AI-DEC-003. Restano da definire gestione concreta e valori dei budget/limiti, formati, copertura degli indici, contratti degli strumenti, consenso nelle attività multi-chiamata e qualità misurabile. Nessuna soglia numerica è stata approvata con queste decisioni.

## 12. Autonomia e controllo dell'agente — AI-DEC-002

**Stato:** confermato esplicitamente dall'utente. Non costituisce approvazione di valori numerici o modalità di consenso remoto. Il comportamento di Auto è confermato separatamente in AI-DEC-003.

### Autonomia nell'ambito autorizzato

- Dopo l'invio l'agente può cercare, leggere e confrontare documenti senza chiedere conferma per ogni lettura.
- Il consenso all'invio remoto resta separato e vincolante: l'autonomia non lo sostituisce.
- Nessun ampliamento autonomo dell'ambito. Se serve un'altra cartella, propone il cambiamento e attende l'utente.

### Chiarimenti e ipotesi

- Se la richiesta è sufficientemente chiara, procede.
- Se interpretazioni sostanzialmente diverse cambiano il risultato, chiede chiarimenti prima, per esempio su progetto, periodo o versione.
- Non interrompe per dettagli secondari gestibili con un'ipotesi dichiarata.

### Qualificazione dei risultati

- Distingue informazioni supportate dalle fonti, interpretazioni/deduzioni, contraddizioni, informazioni mancanti e documenti non accessibili o non analizzati.
- Non presenta una ricerca parziale come esaustiva.

### Limiti e continuazione

- Ogni attività ha limiti finiti e configurabili di durata, chiamate e letture, oltre a eventuali budget remoti.
- Raggiunto un limite, presenta quanto trovato e permette una richiesta di continuazione, senza aggirare i limiti imposti nelle impostazioni.
- Gestione concreta, conteggi, valori iniziali e valori massimi restano da definire separatamente; non sono state approvate soglie numeriche.

### Interruzione

- «Interrompi» blocca nuovi passi e conserva risultati parziali chiaramente identificati.
- Una chiamata remota già partita potrebbe non essere arrestabile o potrebbe comunque essere addebitata; non si promette annullamento universale né assenza di addebito.

## 13. Comportamento funzionale di Auto — AI-DEC-003

**Stato:** confermato esplicitamente dall'utente. Non approva automaticamente una particolare implementazione del router, un modello dedicato o soglie numeriche.

- Auto sceglie l'approccio adeguato meno complesso, senza attivare sempre un agente.
- Esempi indicativi: criteri espliciti come «PDF modificati questa settimana» verso Semplice; «Documenti sulle condizioni di rinnovo» verso Semantica eventualmente combinata con ricerca testuale; richiesta di confronto delle condizioni verso Agentica. Non sono regole rigide basate su singole parole.
- Mostra la modalità effettiva, ad esempio «Auto → Semantica»; se combina approcci, indica quali ha usato.
- Può combinare Semplice e Semantica o avviare un'attività agentica entro ambito, consenso e limiti configurati.
- Il risultato rispetta l'intento: una ricerca di file non diventa automaticamente un lungo rapporto.
- Richieste con criteri espliciti non richiedono necessariamente un modello per essere classificate.
- Se l'ambiguità cambia sostanzialmente il lavoro, chiede un chiarimento anziché scegliere arbitrariamente.
- Se manca una capacità necessaria, spiega il limite e propone alternative; non presenta ricerca testuale come equivalente a Semantica e non passa silenziosamente al cloud.
- Le modalità manuali non cambiano automaticamente: eventuali approfondimenti sono suggeriti, non eseguiti cambiando modalità.
- L'invio in Auto autorizza la scelta dell'approccio, non nuove autorizzazioni sui dati o sui provider.

Restano da definire gestione concreta e valori dei limiti, consenso nelle attività multi-chiamata, contratti e valutazione del routing. Restano aperti i punti di attenzione AI-ATT, inclusa l'affidabilità del router.

### Approfondimento dei limiti rinviato dall'utente

Alla successiva proposta sui limiti pratici l'utente ha indicato «non mi interessano queste informazioni per ora». L'approfondimento è quindi rinviato, non approvato né rimosso dal progetto. Restano valide le sole conferme già registrate in AI-DEC-002: limiti finiti/configurabili, continuazione senza aggirare i limiti delle impostazioni e interruzione esplicita. La proposta di distinguere budget operativo e tetti di sicurezza, i relativi contatori, la gestione dei tetti monetari e i valori numerici non sono stati confermati. Non bloccare su questo dettaglio il successivo confronto funzionale.

## 14. Copertura iniziale dei formati

La proposta per famiglie è stata concordata dall'utente e raccolta in `06-content-formats.md`: testo/Markdown/codice, CSV/TSV, PDF testuali e scansionati con OCR, DOCX/ODT, XLSX/ODS, PPTX/ODP e OCR su PNG/JPEG/WebP. Restano distinte gestione file, anteprima, ricerca testuale e consultazione AI. Anteprime Office/OpenDocument escluse; trascrizione/analisi audio-video rimandate.

Successivamente l'utente ha esplicitamente incluso nel primo rilascio anche la comprensione visiva delle immagini oltre l'OCR: oggetti, diagrammi, grafici e scene; ha inoltre confermato l'estensione a immagini e grafici contenuti in PDF e documenti Office/OpenDocument, con segnalazione delle eventuali parti non analizzabili. Questo non introduce anteprime Office/OpenDocument.

L'input delle richieste resta solo testuale e non è automaticamente approvata una pipeline multimodale specifica. Restano vincolanti funzionamento locale/offline con componenti adeguati, consenso remoto e nessun fallback cloud silenzioso.

Varianti, estrattori, qualità, limiti e riferimenti puntuali richiedono progettazione e verifica. Nessuna esecuzione di codice, macro o contenuti attivi; file non interpretabili segnalati come non analizzati.

## 15. Gestione delle sorgenti e indicizzazione — AI-DEC-004

**Stato:** comportamento funzionale confermato dall'utente. Non sceglie motori, schema, pipeline o soglie tecniche.

- Cartelle aggiunte esplicitamente nelle impostazioni delle sorgenti; nessuna indicizzazione automatica dell'intero computer.
- Sottocartelle incluse, con esclusioni configurabili di cartelle, file e tipi di contenuto, visibili e modificabili.
- Aggiungendo una sorgente parte la prima indicizzazione in background, con stato visibile e possibilità di sospendere/riprendere; navigazione utilizzabile.
- File nuovi, modificati, spostati o eliminati aggiornano progressivamente l'indice, senza ricostruzione completa a ogni cambiamento.
- Stati distinti per contenuti pronti, in elaborazione, esclusi, non supportati e non accessibili. Ricerca incompleta non presentata come definitiva.
- Rimuovere una sorgente interrompe la sua indicizzazione e rimuove i dati derivati non più coperti da altre sorgenti, senza eliminare originali, note, tag o preferiti. La cronologia persistente delle conversazioni è successivamente esclusa in AI-DEC-009; il contesto conversazionale temporaneo ha un ciclo di vita distinto da quello degli indici.
- Ricerca semplice indipendente dall'indice vettoriale; l'indicizzazione non concede nuove autorizzazioni all'invio remoto. Avvio/elaborazione restano soggetti a capacità disponibili e consenso configurato.

### Autorizzazione temporanea esclusa dal perimetro iniziale

Alla proposta di consultare cartelle non abilitate con un'autorizzazione limitata alla singola attività, l'utente ha indicato «per ora non includiamo questa feature (autorizzazione temporanea)». Non includere quindi tale autorizzazione nel primo rilascio né il relativo flusso di conferma. Non dedurre un'abilitazione o un'indicizzazione silenziosa come alternativa. La ricerca semplice resta indipendente dall'indice vettoriale, come già confermato; il resto della proposta sui comportamenti delle singole modalità nelle cartelle non indicizzate non è stato approvato implicitamente.

Le esclusioni iniziali sono successivamente confermate in AI-DEC-008 e il comportamento generale di arresto/ripresa in AI-DEC-010. Restano da definire dettagli operativi su sorgenti abilitate ma non ancora indicizzate, relazione puntuale tra autorizzazione AI e indicizzazione, gestione delle sorgenti sovrapposte e indisponibili, aggiornamento/invalidazione, meccanismi di ripresa dopo riavvio e conservazione dei contenuti derivati. Le regole di identità e riconciliazione già confermate restano vincolanti.

## 16. Astrazione dell'engine e indici per modello — AI-DEC-005

**Stato:** richiesto esplicitamente dall'utente per la progettazione sin dall'inizio. Specifica dedicata: [`requirements/embedding.md`](../requirements/embedding.md), requisiti EMB-001–005; collegamento ad AI-ATT-002.

- Interfaccia astratta dell'engine di embedding dalla prima implementazione, senza accoppiamento delle pipeline a un particolare provider/modello.
- Identificatore non ambiguo e versione/revisione per ogni modello, associati al descrittore dell'indice e alla provenienza degli embedding.
- Indice vettoriale dedicato per modello/versione, isolando anche configurazioni con spazi incompatibili. Separazione logica ammessa nello stesso motore locale, senza imporre database fisici separati.
- Nessun confronto o mescolamento diretto tra embedding di spazi diversi, anche a pari dimensionalità; query e inserimenti devono essere compatibili con l'indice selezionato.
- Un cambio incompatibile richiede nuovi embedding e il relativo indice, non la rietichettatura dei vettori esistenti; conservazione dei dati utente invariata.

Contratto concreto, identificazione delle versioni remote, descrittore completo, storage e ciclo di vita degli indici restano da progettare. Non sono stati scelti motori o librerie; AI-ATT-002 resta aperto fino all'implementazione e alla verifica.

## 17. Configurazione AI e provider — AI-DEC-006

**Stato:** organizzazione funzionale confermata dall'utente. Non sceglie librerie o modelli specifici e non conferma la fattibilità delle singole integrazioni tramite subscription.

### Avvio e configurazione

- File manager utilizzabile senza registrazione e senza configurare un modello all'avvio. Capacità AI non configurate spiegano cosa manca, senza bloccare navigazione e ricerca semplice.
- Configurazione guidata iniziale e impostazioni avanzate quando necessarie.
- Connessioni distinte dai modelli, configurabili in AI e provider: engine locale supportato ed endpoint (ad esempio LM Studio); provider remoto via API con API key ed endpoint quando necessario; subscription come metodo separato, disponibile solo dopo verifica dell'integrazione autorizzata dal provider.
- Ogni connessione ha un nome riconoscibile e «Verifica connessione». Elenco modelli recuperato quando supportato, altrimenti configurabile manualmente.

### Ruoli e compatibilità

- Ruoli configurabili: Embedding, Assistente/agente e Comprensione visiva.
- Lo stesso modello può coprire più ruoli se compatibile; nessun modello distinto obbligatorio per il routing di Auto. OCR possibile anche tramite componenti non generativi.
- Capacità mancanti o non verificate segnalate, senza dedurle soltanto dal nome del modello. Per gli embedding valgono identità, versione e isolamento degli indici di AI-DEC-005.

### Disponibilità

- Una connessione indisponibile non blocca le funzioni indipendenti; viene indicata la capacità temporaneamente indisponibile.
- Nessun passaggio silenzioso ad altro provider o al cloud.
- Destinazione locale/remota riconoscibile nell'uso quotidiano senza riaprire le impostazioni.

Consenso ai dati remoti distinto dalla configurazione delle connessioni e successivamente confermato in AI-DEC-007. Ruoli ulteriori, profili preconfezionati, adattatori concreti, rilevamento delle capacità e verifica delle subscription non sono approvati implicitamente.

## 18. Privacy e consenso remoto — AI-DEC-007

**Stato:** confermato esplicitamente dall'utente, inclusi Solo locale come default e consenso per singola richiesta utente. Non implica implementazione o validazione delle integrazioni.

### Politiche configurabili

- **Solo locale:** nessun contenuto inviato a provider remoti; impostazione iniziale.
- **Chiedi a ogni richiesta:** autorizzazione prima di ogni attività remota.
- **Consenti ai provider selezionati:** autorizzazione persistente, revocabile, per provider e ruoli esplicitamente scelti.
- Configurare una connessione o inserire una API key non equivale ad autorizzare l'invio dei documenti.

### Portata dell'autorizzazione

- Il consenso chiarisce provider destinatario, ruolo, ambito delle cartelle e categorie di dati: richiesta, passaggi testuali, immagini, nomi e altri metadati necessari.
- Autorizzare la generazione di risposte non autorizza automaticamente embedding o comprensione visiva remoti.
- «A ogni richiesta» significa una singola richiesta dell'utente, comprese le chiamate necessarie a completarla, non ogni passo dell'agente.
- Una domanda successiva richiede un nuovo consenso nella politica Chiedi. Cambi di provider, ambito o categorie di dati non coperti richiedono nuova approvazione.

### Indicizzazione e credenziali

- Elaborazioni locali possono procedere. Quelle remote in background rispettano la stessa politica; in Chiedi il lavoro interessato attende autorizzazione, senza inviare dati silenziosamente.
- Credenziali nell'archivio protetto del sistema, con rimozione/disconnessione; nessun segreto nei log o nelle configurazioni esportabili.

Restano da progettare dettagli del raggruppamento dei consensi per indicizzazione continua, applicazione della revoca a lavori già avviati e chiamate in corso, classificazione/verifica degli endpoint e flussi specifici di autenticazione. Non riaprire la granularità per singola richiesta già confermata né dedurre autorizzazione permanente per un'intera conversazione.

## 19. Esclusioni iniziali dell'indicizzazione e dell'AI — AI-DEC-008

**Stato:** base confermata dall'utente. Regole visibili e modificabili; elenco tecnico completo e riconoscimento per piattaforma ancora da progettare.

### Categorie escluse inizialmente

- Cartelle di sistema e applicazioni installate.
- Dipendenze e contenuti generati: ad esempio `node_modules`, ambienti virtuali, `target`, `dist`, cache e file temporanei.
- Metadati dei repository, ad esempio `.git`, senza escludere il codice del progetto.
- Contenuti potenzialmente sensibili: ad esempio `.env`, directory di credenziali e chiavi private riconoscibili.
- File e cartelle nascosti, includibili esplicitamente.
- Dati interni di FolderAI: database, indici e cache dell'applicazione.

Il riconoscimento dei contenuti sensibili è una precauzione, non una garanzia di individuare tutti i segreti.

### Comportamento

- Esclusioni personali aggiungibili per percorso, nome o tipo.
- Esclusioni AI applicate anche alle letture dirette dell'agente: non sono aggirabili semplicemente evitando l'indice.
- Elementi esclusi ancora navigabili nell'explorer; esclusione AI distinta dalla visibilità. «Mostra elementi nascosti» non autorizza automaticamente l'indicizzazione.
- Nuova esclusione: il contenuto non è più utilizzabile dall'AI e viene avviata la rimozione dei dati derivati pertinenti, senza eliminare originali, note, tag o altri metadati personali.

Precedenza delle regole e sorgenti sovrapposte, riferimenti già presenti nelle conversazioni, invalidazione dei lavori in corso, riconoscimento dei segreti e garanzie della rimozione fisica richiedono progettazione. L'accordo non promette cancellazione retroattiva dei dati eventualmente già inviati a provider remoti.

## 20. Nessuna cronologia persistente delle conversazioni — AI-DEC-009

**Stato:** l'utente esclude la cronologia persistente e conferma il solo contesto temporaneo, accettando esplicitamente di perdere la conversazione alla chiusura della scheda. Questo sostituisce la precedente approvazione di una cronologia locale salvata, riapribile ed eliminabile.

- Nessun archivio persistente delle conversazioni e nessuna funzione di riapertura delle conversazioni precedenti.
- Il ripristino dell'ambiente di navigazione non deve implicare il recupero di conversazioni salvate.
- Solo contesto temporaneo della conversazione corrente, per permettere messaggi e domande successive; nessun salvataggio dei messaggi su disco da parte di FolderAI.
- Il contesto rimane passando tra Assistente e Dettagli, nascondendo il pannello o cambiando scheda senza chiuderla.
- «Nuova conversazione» elimina il contesto precedente. Chiudere la scheda elimina la relativa conversazione: l'utente accetta la perdita e non è previsto ripristino.
- Le proposte appena presentate su ripristino delle conversazioni, conservazione dei parziali nella cronologia e retention fino a cancellazione non sono approvate e non vanno implementate.
- La gestione delle attività AI attive e della ripresa dell'indicizzazione è stata successivamente confermata separatamente in AI-DEC-010.

L'assenza di un archivio locale non promette cancellazione o assenza di conservazione da parte di provider remoti: restano valide le autorizzazioni e i limiti già esplicitati.

## 21. Chiusura con attività AI e indicizzazione — AI-DEC-010

**Stato:** comportamenti confermati esplicitamente dall'utente. Non introducono cronologia persistente né servizi autonomi dopo l'uscita.

### Ricerca AI

- Con una ricerca AI attiva, chiudere la scheda propone «Mantieni aperta» oppure «Interrompi e chiudi».
- Confermare la chiusura interrompe l'attività ed elimina la conversazione della scheda, coerentemente con AI-DEC-009.
- Chiudendo una finestra, un'unica conferma riguarda le sue attività; le altre finestre continuano normalmente.
- Senza attività in corso, nessuna richiesta di conservare la conversazione alla chiusura della scheda.
- Restano valide le regole dei trasferimenti e il limite delle chiamate remote già partite, che potrebbero non essere arrestabili.

### Indicizzazione

- Attività dell'applicazione, non della singola scheda.
- Continua chiudendo una finestra finché FolderAI resta in esecuzione; non equiparare automaticamente chiusura dell'ultima finestra e uscita dall'app su ogni piattaforma.
- Uscendo dall'app viene arrestata in modo controllato.
- Al successivo avvio riprende i lavori pendenti rispettando le autorizzazioni correnti, senza ripristinare o rieseguire conversazioni AI.
- Nessun servizio autonomo che continui dopo l'uscita dall'app nel primo rilascio.

Checkpoint, consistenza dopo crash, dettagli delle chiamate in corso e coordinamento tecnico fra attività richiedono progettazione e verifica; la decisione funzionale non promette un recupero universale dopo interruzioni anomale.
