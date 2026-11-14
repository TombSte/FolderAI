# FolderAI — Integrazione AI e modalità di ricerca

**Stato:** analisi approfondita in corso. Le idee generali sono condivise dall'utente; comportamenti specifici, priorità di rilascio e criteri di accettazione restano da definire. I quattro punti di attenzione sulla configurazione multi-modello sono confermati dall'utente e tracciati con identificativi stabili nella sezione 10.

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
- Modalità iniziale ancora da concordare; la proposta precedente era Semplice.
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

Gli agenti restano in sola lettura sui file dell'utente. Suggerire tag o evidenziare possibili duplicati non autorizza ad applicare tag o eliminare file automaticamente.

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
- Applicare la politica di consenso configurata dall'utente. Resta da chiarire se «ogni richiesta» significhi un'attività utente oppure ciascuna chiamata remota all'interno di un'attività agentica.
- Considerare anche embedding, query, passaggi estratti e metadati quando si valuta l'invio di dati a un provider remoto.
- Trattare i contenuti recuperati come dati non attendibili, non come istruzioni capaci di ampliare i privilegi dell'agente.
- Non promettere accesso alle subscription come equivalente alle API senza la verifica specifica per provider già prevista nell'analisi preliminare.

## 8. Decisioni da approfondire

1. Interfaccia e selezione delle modalità: selettore, impostazione iniziale e suggerimenti di passaggio.
2. Ambiti di ricerca e distinzione tra cartella corrente, sottocartelle, selezione e archivio indicizzato.
3. Filtri, operatori e rappresentazione delle query in linguaggio naturale.
4. Risultati: estratti, citazioni, pertinenza, apertura e passaggio tra elenco e analisi.
5. Attività agentiche: strumenti iniziali, limiti, consenso e comportamento in caso di interruzione.
6. Copertura dei formati, qualità misurabile e perimetro del primo rilascio.

## 9. Auto e specializzazione dei modelli — valutazione proposta

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

Proposta: una configurazione guidata sceglie un insieme compatibile per generazione, embedding e, se abilitata, elaborazione visuale. Una sezione avanzata permette di assegnare modelli/provider ai singoli ruoli, senza obbligare l'utente comune a comprendere l'intera pipeline.

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

L'utente concorda sui quattro punti seguenti e ne richiede il tracciamento. Sono vincoli e rischi riconosciuti nell'analisi, non requisiti già formalizzati. Gli identificativi consentono di collegarli successivamente a decisioni architetturali, requisiti e verifiche.

Le misure e le verifiche riportate sono proposte da approfondire; soglie, strumenti e criteri di accettazione non sono ancora concordati. Nessun punto è considerato risolto o verificato.

### AI-ATT-001 — Risorse e prestazioni multi-modello

- **Punto confermato:** più modelli non garantiscono prestazioni migliori. In locale possono competere per RAM/VRAM e introdurre caricamenti ripetuti, latenza o indisponibilità.
- **Misure proposte:** consentire il riuso di un modello per più ruoli, evitare specializzazioni obbligatorie e valutare caricamento, concorrenza e limiti di risorse.
- **Verifica proposta:** confrontare configurazioni essenziali e specializzate su hardware e archivi rappresentativi, misurando qualità, latenza, memoria e reattività dell'esplora risorse.
- **Da definire:** hardware di riferimento, scenari di carico, budget e politica di gestione dei modelli.
- **Stato:** aperto; punto confermato, mitigazioni da definire e validare.

### AI-ATT-002 — Compatibilità tra embedding e indice

- **Punto confermato:** l'indice dipende dal modello e dalla configurazione di embedding. Un cambio può richiedere reindicizzazione; vettori di spazi incompatibili non devono essere mescolati, anche se hanno la stessa dimensione.
- **Misure proposte:** registrare l'identità/versione della configurazione semantica, verificare la compatibilità delle query con l'indice e gestire esplicitamente la ricostruzione senza perdere preferiti o metadati utente.
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
- **Da definire:** granularità del consenso nelle attività multi-chiamata, gestione del rifiuto e presentazione delle alternative autorizzabili.
- **Stato:** aperto; punto confermato, mitigazioni da definire e validare.
