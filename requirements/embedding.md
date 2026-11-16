# FolderAI — Specifica dell'engine di embedding

**Stato:** vincolo richiesto esplicitamente dall'utente per il primo rilascio; non ancora implementato o verificato. Questa specifica anticipa il solo ambito embedding, senza avviare la formalizzazione completa degli altri requisiti.

**Fonti:** richiesta diretta dell'utente, decisione AI-DEC-005 in `analysis/03-ai-integration.md` e punto di attenzione AI-ATT-002.

## EMB-001 — Interfaccia astratta fin dall'inizio

La generazione degli embedding deve essere accessibile tramite un'interfaccia astratta dell'engine, presente sin dalla prima implementazione. Indicizzazione e ricerca non devono dipendere direttamente da uno specifico modello, SDK, provider o runtime. Le integrazioni concrete devono implementare tale contratto.

**Accettazione:** un engine di prova sostituisce quello concreto senza modificare la logica di indicizzazione o ricerca; entrambe le pipeline usano l'astrazione per richiedere gli embedding.

## EMB-002 — Identità e versione del modello

Ogni modello di embedding deve avere un identificatore non ambiguo e una versione/revisione esplicita. Questi dati devono essere conservati con il descrittore del relativo indice e la provenienza degli embedding. Il solo nome visualizzato, un alias mobile o la dimensione del vettore non dimostrano compatibilità.

Il descrittore dello spazio deve inoltre distinguere le configurazioni che possono produrre embedding incompatibili, anche a parità di modello/versione, in continuità con AI-ATT-002. Campi esatti e modalità di identificazione restano da progettare.

**Accettazione:** modelli differenti e revisioni differenti sono distinguibili in persistenza; l'indice espone l'identità/versione e il descrittore dello spazio cui appartengono i suoi vettori.

## EMB-003 — Indice vettoriale dedicato

Ogni modello/versione deve avere il proprio indice vettoriale; configurazioni con spazi incompatibili devono restare isolate. Non è consentito mescolare vettori di modelli diversi in un unico spazio di confronto, anche quando hanno la stessa dimensionalità.

La separazione richiesta è logica: può essere realizzata con indici, collezioni o partizioni isolate nello stesso motore locale. Non impone database fisici o servizi separati per ogni modello. Un adattatore dell'engine e il motore di storage vettoriale hanno responsabilità distinte.

**Accettazione:** due modelli che producono vettori della stessa dimensione mantengono indici isolati; lo stesso vale per due revisioni o configurazioni incompatibili dello stesso modello.

## EMB-004 — Controllo di compatibilità in scrittura e ricerca

Prima dell'inserimento nell'indice e della ricerca per similarità, il sistema deve verificare la compatibilità con lo spazio dichiarato dall'indice. L'embedding della query deve essere prodotto dal modello/versione e dalla configurazione compatibili con quello spazio.

Un mismatch deve impedire l'operazione e produrre un errore comprensibile, senza confronti incrociati, inserimenti incompatibili o fallback silenziosi. Il controllo non può basarsi soltanto sulla lunghezza dei vettori.

**Accettazione:** l'inserimento e la ricerca con un modello/revisione incompatibile vengono rifiutati anche a parità di dimensione; il percorso compatibile continua a funzionare. Nessun risultato viene presentato come semanticamente valido usando vettori di spazi differenti.

## EMB-005 — Cambio di modello o revisione

Il passaggio a un modello, una revisione o una configurazione incompatibile richiede la generazione degli embedding nello spazio nuovo e la popolazione del relativo indice. I vettori precedenti non devono essere riutilizzati o semplicemente rietichettati come compatibili.

La ricostruzione non deve eliminare originali, note, tag o preferiti. Restano applicabili consenso remoto, disponibilità locale e divieto di fallback cloud silenzioso.

**Accettazione:** un cambio di modello/revisione seleziona o richiede un indice dello spazio corretto; se non ancora popolato, la condizione è esplicita e non si interroga quello incompatibile. I metadati utente rimangono intatti.

## Punti ancora da progettare

- Contratto concreto dell'interfaccia e adattatori iniziali.
- Identificazione affidabile della revisione, soprattutto per servizi con alias o versioni non esposte; nessuna versione deve essere inventata.
- Descrittore completo della compatibilità, incluse eventuali capacità testuali/multimodali e configurazioni specifiche per documenti e query.
- Motore vettoriale e rappresentazione fisica degli indici isolati.
- Conservazione o rimozione dei vecchi indici, migrazioni, ricostruzioni interrotte e disponibilità durante la reindicizzazione.

Questi punti non annullano il vincolo confermato: l'astrazione, l'identità/versione e l'isolamento degli spazi devono essere previsti fin dall'inizio. Non sono richiesti caricamento simultaneo di più modelli o confronti diretti tra i loro vettori.
