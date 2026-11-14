# FolderAI — Mockup HTML

Cinque render per discutere l'interfaccia prima dell'implementazione Tauri/React. I primi due esplorano viste separate; i nuovi concept integrano la scelta della ricerca direttamente nell'explorer, senza un toggle globale Explorer/AI.

| Vista | File | Contenuto |
| --- | --- | --- |
| Explorer classico | [folderai-explorer.html](folderai-explorer.html) | Sidebar, schede, viste dettagli/griglia/colonne, selezione e pannello anteprima con metadati, tag e nota |
| Ricerca e AI | [folderai-ai.html](folderai-ai.html) | Semplice/Semantica/Agentica/Auto, ambito esplicito, risultati con passaggi, confronto con citazioni e attività osservabili |
| Ricerca integrata | [folderai-integrated.html](folderai-integrated.html) | Stile originale, menu delle modalità dentro la barra, file e risposta contestuale nello stesso ambiente |
| Interpretazione macOS | [folderai-macos.html](folderai-macos.html) | Stessa interazione integrata, con chrome traslucido, accenti blu, superfici morbide e desktop illustrativo |
| Windows 11 / WinUI 3 | [folderai-winui.html](folderai-winui.html) | Ricerca integrata, superfici stile Mica, controlli Fluent, command bar, caption Windows e taskbar illustrativa |

## Apertura

Aprire direttamente uno dei file HTML nel browser. CSS e JavaScript sono locali: non servono build, installazioni, font remoti o connessioni di rete.

In alternativa, dalla radice del repository:

```sh
python3 -m http.server 8000 --bind 127.0.0.1 --directory docs/mockups
```

Poi aprire:

- http://127.0.0.1:8000/folderai-explorer.html
- http://127.0.0.1:8000/folderai-ai.html
- http://127.0.0.1:8000/folderai-integrated.html
- http://127.0.0.1:8000/folderai-macos.html
- http://127.0.0.1:8000/folderai-winui.html

Nei primi due render, il selettore in alto a destra permette di confrontare le viste originali. Nei nuovi render **non c'è un toggle Explorer/AI**: la modalità si sceglie nella barra di ricerca. I collegamenti di confronto tra direzioni visive sono fuori dalla finestra dell'app, nella didascalia. Il pulsante luna/sole cambia il tema; il tema scelto viene ricordato dal browser quando lo storage locale è disponibile. Le viste sono pensate soprattutto per desktop, con adattamenti per finestre più piccole.

## Interazioni dimostrative

### Explorer

- Selezionare le righe per aggiornare informazioni, tag e nota del pannello laterale. Frecce su/giù e Invio funzionano sull'elenco.
- Filtrare per nome con la ricerca; `Cmd/Ctrl + K` porta il focus al campo.
- Usare tag e ricerca salvata PDF come filtri sui dati fittizi.
- Passare tra dettagli, griglia e una rappresentazione illustrativa della vista a colonne.
- Attivare/disattivare il secondo pannello dimostrativo.
- Mostrare un file nascosto, invertire l'ordinamento o aggiungere una cartella solo alla memoria della pagina.
- Aprire Preferenze e Centro attività dimostrativi.

### AI

- Cambiare modalità: semplice e semantica mostrano risultati senza sintesi; agentica mostra la risposta documentata.
- In Auto, poche regole locali illustrano il routing: richieste di confronto portano ad Agentica, estensioni esplicite a Semplice, le altre a Semantica. **Non è un classificatore AI né la logica definitiva del prodotto.**
- Il pulsante Cerca simula un'attesa, ma **non esegue una ricerca**: fonti e risposta rimangono esempi fissi anche cambiando la domanda.
- Le citazioni evidenziano la fonte; «Apri fonte» mostra il passaggio fittizio in una finestra di dialogo.
- Il pulsante del modello mostra un esempio di ruoli specializzati. Nomi e assegnazioni sono illustrativi, non scelte tecniche approvate.
- «Salva ricerca» aggiunge un collegamento alla pagina, senza persistenza dopo il ricaricamento.
- «Copia sintesi» scrive soltanto il testo dimostrativo negli appunti, su azione esplicita; se il browser nega l'accesso, viene segnalato.

### Ricerca integrata: originale, macOS e Windows

- Tutti e tre partono da una ricerca in Apollo, con file e passaggi al centro e risposta nel pannello contestuale.
- Il pulsante **Auto** nella barra apre un mini menu con Auto, Semplice, Semantica e Agentica, descrizioni e selezione corrente.
- Semplice usa un filtro testuale sui nomi, passaggi e metadati fittizi; Semantica mostra i quattro risultati illustrativi senza generare una sintesi; Agentica mostra il confronto fisso con fonti. Auto usa poche regole locali, non un modello.
- La scelta del tipo di ricerca non cambia pagina, sidebar o schede.
- Cancellare la ricerca, premere Esc dal campo oppure scegliere «Torna alla cartella» ripristina l'elenco di nove elementi nello stesso explorer.
- Il pulsante filtri permette di includere/escludere i formati della demo. Il confronto fisso non viene mostrato se mancano le fonti necessarie.
- Selezionare un risultato o una citazione apre «File selezionato» nel medesimo pannello; «Risposta» torna al confronto. Nelle modalità senza generazione il pannello resta dedicato al file.
- Sono disponibili vista elenco/griglia, visibilità di sidebar/pannello, tema e salvataggio di query/modalità/filtri nella sola memoria della pagina.
- `Cmd/Ctrl + K` seleziona il campo. Il menu modalità supporta frecce, Home/End e Esc; `Alt + Freccia giù` lo apre dal campo.
- «Cerca» simula soltanto un'attesa. La ricerca semantica e la risposta non cambiano realmente in base a una domanda arbitraria.
- Il render macOS è **un'interpretazione ispirata al linguaggio macOS/glass**, non una riproduzione ufficiale o verificata di macOS 27. Mantiene le superfici di lettura relativamente opache e prevede un fallback CSS per la preferenza di riduzione delle trasparenze.
- Le versioni precedenti sono conservate: il nuovo stile non sostituisce quello originale. Tema dei concept integrati e tema dei primi due render sono ricordati separatamente dal browser.

### Variante Windows 11 / WinUI 3

- Interpretazione HTML del linguaggio WinUI 3 / Fluent: superfici stile Mica, accenti blu, angoli da 4/8 px, indicatore verticale della navigazione e tab contestuali sottolineati.
- Header con controlli finestra a destra, schede, command bar e barra di ricerca integrata; nessun selettore globale Explorer/AI.
- Font locale `Segoe UI Variable Text` / `Segoe UI` dove installato, con fallback di sistema. Nessun font viene scaricato; su macOS/Linux la resa tipografica può differire.
- Tema chiaro/scuro e le stesse interazioni di ricerca, citazioni, filtri, griglia, pannello e salvataggio dimostrativo degli altri concept integrati.
- Il pulsante Massimizza/Ripristina ridimensiona soltanto il mockup nella pagina. Riduci a icona, Chiudi e i comandi del filesystem mostrano messaggi illustrativi: non controllano finestre native, file o appunti.
- Taskbar, icone, meteo, disco e orologio sono fittizi. «Cerca» nella taskbar porta il focus al campo dell'app; l'icona FolderAI permette di tornarvi. Non viene aperto il menu Start reale.
- Adattamento alle finestre strette e CSS per riduzione delle trasparenze e colori forzati; non sostituisce verifiche di accessibilità su Windows reale.
- È **un mockup HTML ispirato a WinUI 3**, non un'app WinUI/XAML o un'implementazione nativa di Mica. Non cambia lo stack Tauri/React/Rust scelto per il prodotto.
- I quattro render precedenti sono conservati senza modificarne grafica o comportamento.

## Limiti e relazione con l'analisi

- Nessun accesso al filesystem, nessuna chiamata a modelli/provider, nessun caricamento di documenti o dipendenze esterne.
- File, capacità del disco, conteggi, date, fonti, modelli e risultati sono fittizi.
- Le anteprime sono composizioni illustrative, non rendering di PDF o documenti reali.
- Non sono implementati spostamenti, eliminazioni, estrazione ZIP, autenticazione, indicizzazione, note modificabili o sincronizzazione delle preferenze tra finestre.
- Alcuni controlli mostrano soltanto una spiegazione del comportamento previsto. Il layout a colonne e il secondo pannello sono esempi visivi, non navigatori completi.
- I testi su elaborazione locale e privacy descrivono lo scenario rappresentato: nessun engine AI è realmente connesso.
- I riferimenti macOS sono una scelta illustrativa, non un vincolo di piattaforma.
- Questi file non sostituiscono React/TypeScript/SCSS previsti per il prodotto e non attestano conformità o accessibilità già validate in Tauri.
- Le soluzioni visive sono **proposte da discutere**, non nuovi requisiti automaticamente approvati. Il perimetro resta in [../../analysis/04-analysis-summary.md](../../analysis/04-analysis-summary.md).

File condivisi dei primi due render: `mockups.css` e `mockups.js`. Concept integrati: `integrated.css` e `integrated.js`; macOS aggiunge `macos.css`, Windows aggiunge `winui.css` e `winui.js`.
