# Specifica del primo rilascio di FolderAI

FolderAI è un file manager desktop per Windows 11 x64 e macOS Apple Silicon, con ricerca nei contenuti e un assistente AI in sola lettura. La ricerca e l’organizzazione dei file hanno priorità negli scenari d’uso. Questa specifica traduce il perimetro concordato in comportamenti, vincoli di implementazione e verifiche.

| Documento | Contenuto |
| --- | --- |
| [Requisiti funzionali](functional.md) | RF-001–056, comportamenti osservabili e criteri di accettazione |
| [Requisiti tecnici e non funzionali](technical.md) | RT-001–034, piattaforma, dati, operazioni, AI, sicurezza, qualità e collaudo |
| [Engine di embedding](embedding.md) | EMB-001–005, specifica tecnica già concordata e richiamata da RT-022 |
| [Decisioni aperte](open-decisions.md) | DA-001–021, lavoro che dipende da ogni scelta e criterio di chiusura |

## Perimetro di rilascio

- Il file explorer include navigazione con schede, doppio pannello e finestre multiple, operazioni manuali, viste, anteprime, organizzazione, ricerca semplice, ZIP e integrazioni native. Le operazioni confermate riguardano i filesystem locali della matrice di supporto.
- Ricerca semantica, Agentica e Auto, domande, riassunti, confronti ed estrazione fanno parte del perimetro, insieme a estrazione testuale, OCR e comprensione visiva delle famiglie confermate. L’assistente opera in sola lettura nelle cartelle abilitate e usa input testuale. Il consenso remoto segue la politica configurata, con Solo locale come impostazione iniziale.
- Il mockup Workspace sobrio fissa la direzione visiva. Le verifiche funzionali e di accessibilità sono descritte dai requisiti e dai relativi criteri di accettazione. [Analisi di riferimento](../analysis/04-analysis-summary.md)

### Rinvii ed esclusioni confermati

| Area | Funzionalità rimandate o escluse | Fonte |
| --- | --- | --- |
| File explorer | Dischi esterni e sorgenti di rete, rinomina in blocco, editor integrato, anteprime Office/OpenDocument, creazione o modifica degli archivi e formati compressi diversi da ZIP. | [Riepilogo §4](../analysis/04-analysis-summary.md#4-funzionalit%C3%A0-rimandate), FE-DEC-014, 049–050, 056 |
| Dati applicativi | Esportazione/importazione manuale e gestione backup dedicata all’utente. Protezione delle migrazioni e recupero dei dati applicativi fanno parte di RT-011. | [File explorer §16](../analysis/02-file-explorer-and-local-data.md), FE-DEC-059 |
| AI | Operazioni sui file, terminale e web per l’agente, suggerimenti automatici di tag, rilevamento proattivo di duplicati e versioni, analisi periodiche autonome, input immagini, autorizzazione temporanea di cartelle, cronologia persistente delle conversazioni e indicizzazione dopo l’uscita dall’app. | [Analisi AI §§11, 15, 20–21](../analysis/03-ai-integration.md), AI-DEC-001, 004, 009–010 |
| Contenuti | Trascrizione e analisi audio/video. | [Formati §2](../analysis/06-content-formats.md#2-copertura-iniziale-concordata) |

## Sequenza proposta per iniziare

Le fasi indicano un ordine di lavoro proposto per ridurre le dipendenze tecniche. I requisiti conservano il perimetro di rilascio indicato sopra.

| Fase | Esito verificabile | Decisioni a cui dare priorità |
| --- | --- | --- |
| 1. Fondazioni e navigazione | App avviabile sui due target, elenco navigabile in una finestra, breadcrumb, schede, contratti IPC e adattatore di lettura del filesystem. | DA-001 per i target, DA-002 per lo stato, DA-008 per la persistenza definitiva. |
| 2. Operazioni e organizzazione | Operazioni manuali con prove di interruzione e conflitto, integrazioni native, metadati persistenti e viste. I percorsi di sostituzione e undo passano le prove prima dell’abilitazione. | DA-003–011 per i comportamenti interessati. |
| 3. Contenuti, ricerca e sorgenti | Ricerca semplice offline, estrazione con copertura visibile, sorgenti e aggiornamenti incrementali. L’indice semantico verifica EMB-001–005 e rispetta ambito ed esclusioni. | DA-012–015, DA-020. |
| 4. Assistente, Auto e consenso | Prima esecuzione end-to-end con provider di prova e modelli locali adeguati. Verifica di citazioni, annullamento, transizione fra modalità, apertura di una ricerca salvata e blocco degli invii non autorizzati. | DA-016–018, DA-020. |
| 5. Collaudo multipiattaforma | Prove native, accessibilità, corpus e soglie approvate, installazione, aggiornamento e recupero dei dati. | DA-001, DA-019–021. |

## Verifica dell’implementazione

Per ogni criterio di accettazione, registrare versione, piattaforma, esito ed evidenza. Per i casi dipendenti da hardware, corpus o provider, registrare anche la configurazione. Prima del rilascio verificare i criteri applicabili e chiudere le DA che ne impediscono la valutazione.

| Percorso | Evidenza minima |
| --- | --- |
| Navigazione e UI | Prove native con tastiera e screen reader di avvio, schede e finestre, pannelli, temi, lingue, ridimensionamento e appunti/drag-and-drop. |
| Operazioni distruttive e concorrenza | Test su directory di prova con collisioni, errore su elemento, interruzione, spazio insufficiente, cancellazione, ripristino in conflitto, modifiche esterne e percorso ZIP fuori destinazione. |
| Metadati e persistenza | Prove di copia configurabile, riconciliazione, salvataggi concorrenti, migrazione fallita e ricostruzione degli indici con confronto dei dati utente. |
| Formati, copertura e pertinenza | Corpus versionato per ogni famiglia, output attesi, riferimenti puntuali e casi parziali o indisponibili. Misure di qualità e prestazioni secondo DA-020. |
| Accessi, modalità e consenso | Tracce di provider simulati, controllo degli invii con Solo locale e revoca, prove di esclusioni e link, nessuna chiamata all’apertura di un’agentica salvata, timeout e stop. Prova offline con componenti locali disponibili. |
| Ciclo di vita | Arresto e ripresa dell’indicizzazione, separazione fra finestre, cancellazione del contesto AI, impossibilità di usare embedding incompatibili e verifica di installazione/aggiornamento su entrambi i target. |
