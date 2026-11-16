# FolderAI — Formati e contenuti del primo rilascio

**Stato:** copertura iniziale per famiglie concordata dall'utente; comprensione visiva oltre l'OCR inclusa esplicitamente nel primo rilascio, anche per immagini e grafici contenuti in PDF e documenti Office/OpenDocument. Varianti precise, limiti, estrattori, qualità e compatibilità multipiattaforma ancora da definire e verificare.

## 1. Distinzione delle capacità

La proposta concordata distingue:

1. Gestione nell'explorer: copia, spostamento, rinomina e apertura esterna.
2. Anteprima integrata.
3. Ricerca nei contenuti, senza dipendenza obbligatoria da un LLM per la query.
4. Consultazione dei contenuti da parte dell'AI per ricerca semantica, domande e confronti.

L'assenza di anteprima integrata non esclude ricerca o consultazione AI. La gestione manuale dei file non è limitata alle sole famiglie di cui si sa estrarre il contenuto.

## 2. Copertura iniziale concordata

| Famiglia | Contenuto ricercabile e consultabile dall'AI | Anteprima integrata |
| --- | --- | --- |
| Testo, Markdown e codice | Testo, inclusi formati come JSON, XML, YAML e log | Sì |
| CSV/TSV | Testo e dati tabellari | Testuale inizialmente |
| PDF con testo | Testo estratto con riferimenti alle pagine; comprensione di immagini e grafici contenuti | Sì |
| PDF scansionati | Testo ottenuto tramite OCR; comprensione del contenuto visivo delle pagine | Sì |
| DOCX/ODT | Testo e tabelle estraibili; comprensione di immagini e grafici contenuti | No, apertura esterna |
| XLSX/ODS | Contenuto leggibile dei fogli con riferimenti a foglio/celle; comprensione di immagini e grafici contenuti | No, apertura esterna |
| PPTX/ODP | Testo delle diapositive con relativo riferimento; comprensione di immagini e grafici contenuti | No, apertura esterna |
| PNG, JPEG, WebP | Testo riconosciuto tramite OCR; comprensione del contenuto visivo per ricerca semantica e consultazione AI | Sì |
| Audio/video | Trascrizione e analisi rimandate | No, apertura esterna |

Sono obiettivi funzionali, non capacità già implementate o verificate. Non dedurre da queste famiglie supporto universale a tutte le varianti, codifiche, strutture incorporate o versioni dei formati.

## 3. OCR e immagini

- Per scansioni e immagini, la ricerca semplice usa il testo già estratto tramite OCR, senza chiamare un modello generativo a ogni query.
- Riconoscere testo è distinto dal comprendere oggetti, diagrammi, scene o altri contenuti visuali.
- Il rinvio dell'input immagini alla fase due riguarda le richieste dell'utente, non impedisce l'analisi delle immagini già presenti nelle cartelle autorizzate.
- Dopo l'accordo sulla tabella, l'utente ha esplicitamente incluso anche la comprensione visiva delle immagini nel primo rilascio: oggetti, diagrammi, grafici e scene oltre al testo OCR.
- L'utente ha successivamente confermato l'estensione a immagini e grafici contenuti in PDF e documenti Office/OpenDocument. Eventuali parti non analizzabili devono essere segnalate.
- Questo amplia l'analisi dei contenuti, non il perimetro delle anteprime: quelle Office/OpenDocument rimangono escluse dal primo rilascio.
- L'input delle richieste resta testuale: si possono cercare/interrogare immagini già presenti nelle cartelle autorizzate. Non è un'approvazione anticipata dell'input tramite immagini.
- Non sono ancora scelti modelli visuali, rappresentazioni semantiche, pipeline di descrizione o embedding multimodali; l'inclusione funzionale non approva automaticamente una di queste implementazioni.
- Restano validi funzionamento locale/offline con componenti adeguati, consenso per dati remoti e nessun fallback cloud silenzioso.

## 4. Sicurezza e copertura parziale

- Nessuna esecuzione di codice, macro o contenuti attivi dei documenti.
- File protetti, danneggiati o non interpretabili segnalati esplicitamente come non analizzati, senza presentarli come contenuti privi di corrispondenze.
- Riferimenti a pagine, fogli/celle e diapositive sono parte dell'obiettivo di tracciabilità. Estrattori e precisione effettiva devono essere verificati.

## 5. Punti aperti

- Estrazione/rappresentazione delle immagini e dei grafici contenuti nei formati confermati, riferimenti al documento/pagina/foglio/diapositiva e gestione delle parti non analizzabili. L'inclusione funzionale è confermata, la copertura tecnica delle varianti richiede verifica.
- Codifiche testuali, varianti PDF, immagini ulteriori, formati Office precedenti e contenuti incorporati.
- Fogli di calcolo: valori disponibili, formule, dati non aggiornati e strutture complesse, senza dedurre un motore di ricalcolo dall'inclusione di XLSX/ODS.
- Dimensioni, pagine e altre soglie di elaborazione, file cifrati/protetti, risultati parziali e annullamento.
- Parser/OCR/modelli, conservazione del testo estratto, aggiornamento degli indici e criteri di qualità da scegliere e verificare successivamente.

Riferimenti: `02-file-explorer-and-local-data.md` per il perimetro delle anteprime; `03-ai-integration.md` per capacità, autorizzazioni e limiti dell'AI; `05-ui-design.md` per input solo testuale e rappresentazione degli stati.
