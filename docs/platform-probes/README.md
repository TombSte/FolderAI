# Matrice delle capacità native

Evidenza per DA-001 e RT-001, RT-005, RT-006 e RT-008. La matrice descrive i risultati delle prove sulle primitive. La copertura ufficiale, la versione minima di macOS e le garanzie di FolderAI restano da definire.

| Target e volume | Nomi e permessi | Link | Cestino | Pubblicazione e sostituzione |
| --- | --- | --- | --- | --- |
| macOS 26.5.1 (25F80), Apple Silicon, APFS locale case-insensitive, utente standard | Nome con spazi e Unicode riuscito. Maiuscole e forma Unicode decomposta risolvono allo stesso inode. Limite osservato per componenti ASCII di 255 byte. File assente `ENOENT`, lettura con modo `000` `EACCES`. | Hard link sullo stesso inode. Symlink visibile anche con destinazione assente, ciclo `ELOOP`. Rinomina dentro un discendente respinta con `EINVAL`. L’alias Finder ha ritrovato il file rinominato. | `FileManager.trashItem` ha restituito l’URL dell’elemento spostato. Ripristino del file sintetico riuscito. In sandbox la chiamata restituisce `NSCocoaErrorDomain` 513. | `renamex_np(RENAME_EXCL)` restituisce `EEXIST` sui conflitti e lascia intatto il file presente. Due tentativi concorrenti hanno prodotto un solo vincitore. `rename` sullo stesso volume ha pubblicato il file preparato e un handle già aperto ha continuato a leggere i dati precedenti. La sorgente di staging assente ha lasciato intatta la destinazione. |
| Stesso host macOS, APFS case-sensitive, immagine disco scrivibile da 128 MiB | La variante in maiuscolo risulta assente. La forma Unicode decomposta risolve allo stesso inode. Stessi esiti per lunghezza ASCII, file assente e modo `000`. | Le prove su link e alias sono passate anche su questo volume. | Spostamento e ripristino riusciti. | Stessi esiti sullo stesso volume. Tra i due volumi APFS, `rename` ha restituito `EXDEV` in entrambe le direzioni e ha conservato i contenuti di sorgente e destinazione. |
| Stesso host macOS, ExFAT, immagine disco scrivibile | Spazi, Unicode e limite ASCII di 255 byte verificati. Le varianti in maiuscolo e decomposta hanno trovato lo stesso file. `chmod 000` è riuscito ma la lettura è rimasta possibile sul volume montato. | Hard link respinto con `ENOTSUP`. I symlink creati da macOS, anche ciclici e con destinazione assente, hanno superato le prove. L’alias Finder è stato creato, ma dopo la rinomina del file il relativo test di risoluzione è fallito. | Spostamento e ripristino riusciti sull’immagine. La rimozione finale della directory di prova tramite `FileManager` è fallita con `NSCocoaErrorDomain` 513. | Sostituzione tramite `rename` riuscita sullo stesso volume. Su destinazione libera, entrambe le pubblicazioni con `renamex_np(RENAME_EXCL)` hanno restituito `ENOTSUP` in cinque ripetizioni. Tra ExFAT e APFS, `rename` ha restituito `EXDEV` e conservato i due file in entrambe le direzioni. |
| macOS, altri volumi rimovibili | Da provare su ogni filesystem candidato. | Da provare. | Da provare per volume. | Da provare sullo stesso volume e tra volumi. |
| Windows 11 x64, NTFS locale | Da provare su un host del target. | Da provare con e senza privilegi di creazione symlink. | Da provare nella sessione desktop. | Da provare su NTFS e tra volumi candidati. |
| Windows 11 x64, altri filesystem locali candidati | Da provare su ogni filesystem candidato. | Da provare per volume. | Da provare per volume. | Da provare per volume. |

## Riprodurre la prova macOS

Usare un terminale come utente standard e, per la prova sul Cestino, una sessione con accesso al Cestino del volume. Eseguire dalla radice del repository. Per provare un altro volume indicare come argomento del programma una directory scrivibile su quel volume. Il programma crea una directory univoca, usa solo file sintetici, tenta il ripristino dell’elemento spostato nel Cestino e rimuove i fixture.

```sh
(
  set -eu
  probe_dir=$(mktemp -d)
  trap 'rm -f "$probe_dir/macos-probe"; rmdir "$probe_dir"' EXIT
  sw_vers
  uname -sm
  diskutil info "$probe_dir"
  xcrun clang -Wall -Wextra -Werror -fobjc-arc -framework Foundation \
    docs/platform-probes/macos.m -o "$probe_dir/macos-probe"
  "$probe_dir/macos-probe" "$probe_dir"
)
```

Il 18 novembre 2026 il volume delle prove è stato identificato con `df -T apfs` perché DiskManagement era indisponibile in sandbox. La compilazione con avvisi trattati come errori è riuscita. La prova confinata è terminata con codice `1` per il Cestino. La ripetizione con accesso al Cestino ha restituito codice `0`.

### Scambio di file durante una sostituzione

Passare `--swap-only` dopo la directory di prova per eseguire solo le verifiche con `renamex_np(RENAME_SWAP)`. La prova del 19 novembre su APFS ha conservato il file precedente nel percorso di staging dopo lo scambio e ha lasciato intatto il destinatario quando mancava lo staging.

In una prova con sostituzione concorrente simulata, lo scambio è riuscito anche dopo l'arrivo di un file diverso al percorso di destinazione. Lo staging ha conservato il file concorrente, e i tre contenuti sono rimasti leggibili. Un cambiamento di contenuto nello stesso inode è sopravvissuto nello staging dopo lo scambio. L'esito mostra che il codice di successo attesta lo scambio dei nomi. L'implementazione deve verificare identità e versione del file spostato nello staging, conservare entrambi gli oggetti quando l'esito è incerto e riesaminare i percorsi prima di qualsiasi ripristino.

### Spazio insufficiente durante la preparazione

Eseguire il programma con la directory montata di un'immagine APFS temporanea da 128 MiB e l'opzione `--space-only`. Il programma rifiuta i filesystem che riportano una capacità superiore a 160 MiB. Il 19 novembre, la scrittura dello staging si è fermata con `ENOSPC` dopo 129.499.136 byte. Dopo la rimozione del parziale, il nome di destinazione puntava ancora all'oggetto originale e la lettura restituiva il contenuto precedente. La sonda verifica la preparazione e la pulizia su APFS. La gestione degli esiti nel centro attività richiede il motore dei trasferimenti.

### Cambio della cartella di destinazione durante la pubblicazione

Con `--anchor-only`, la sonda apre la directory di destinazione, la rinomina e colloca un collegamento simbolico all'indirizzo precedente. Su APFS, `renameatx_np` con `RENAME_EXCL` pubblica nella directory identificata dal descrittore aperto. La cartella indicata dal collegamento rimane intatta. Il descrittore protegge dall'inseguimento del percorso sostituito. Per classificare correttamente l'esito, il motore deve anche verificare se il percorso scelto dall'utente identifica ancora la directory catturata.

### Accesso negato da ACL su APFS

Con FolderAI aperto come utente standard, creare una cartella temporanea con una sottocartella e negare solo a quella sottocartella l’elenco e l’attraversamento.

```sh
acl_probe_dir=$(mktemp -d)
mkdir "$acl_probe_dir/denied"
chmod 700 "$acl_probe_dir/denied"
chmod +a "user:$(id -un) deny list,search" "$acl_probe_dir/denied"
printf '%s\n' "$acl_probe_dir"
```

Aprire il percorso stampato in FolderAI e poi tentare di entrare in `denied`. Il genitore deve rimanere accessibile. Nel collaudo nativo WebKit con questa ACL, l’elenco del genitore includeva `denied`, l’ingresso restituiva `permission-denied` e una nuova richiesta per il genitore riusciva. Rimuovere la regola prima di rimuovere le due cartelle di prova.

```sh
chmod -N "$acl_probe_dir/denied"
rmdir "$acl_probe_dir/denied" "$acl_probe_dir"
```

### Volumi distinti

Su due volumi APFS, l'opzione `--cross-only /directory-del-secondo-volume` verifica il recupero dopo una rimozione negata. Il 19 novembre la sonda ha pubblicato sul secondo volume una copia preparata e sincronizzata. Quando la directory sorgente ha perso il permesso di scrittura, la rimozione ha restituito `EACCES`. Entrambi i file hanno conservato identità e contenuto. Ripristinato il permesso, la sonda ha rimosso soltanto l'origine, mantenendo la copia già pubblicata. La sonda esercita le primitive del filesystem. Il retry tramite journal richiede il motore dei trasferimenti.

Per ripetere la prova a due volumi, creare un’immagine APFS con `hdiutil create -size 128m -fs 'Case-sensitive APFS' -volname FolderAI-Probe-APFS` seguito dal percorso di una nuova immagine `.dmg`. Per ExFAT, creare una nuova immagine `.img` con `diskutil image create blank --fs ExFAT --format RAW --size 128M` seguito dal percorso dell’immagine.

Montare solo l’immagine di prova con `hdiutil attach` e passare al programma due directory scrivibili con percorsi assoluti, una su ciascun volume. Eseguire la prova in entrambe le direzioni. Quando ExFAT è il primo volume, il programma restituisce `1` per i casi che divergono dalle aspettative APFS, elencati nella matrice. Smontare l’immagine prima di rimuoverla.

Su ExFAT, un conflitto con un nome già esistente ha restituito `EEXIST` prima della prova concorrente. Per verificare che la primitiva sia utilizzabile occorre provare anche la pubblicazione su destinazione libera, che in questo collaudo ha restituito `ENOTSUP`.

## Eseguire la prova Windows

Da PowerShell 7 x64 su Windows 11 x64, come utente standard, eseguire dalla radice del repository. La directory predefinita è quella temporanea dell’utente. `-FixtureParent` permette di scegliere una directory esistente e scrivibile sul volume da provare. Il programma crea e rimuove una sottodirectory univoca con i soli file sintetici.

```powershell
pwsh -NoProfile -File .\docs\platform-probes\windows.ps1
```

Il programma registra build, architettura, elevazione, tipo di filesystem, limite dei componenti, equivalenze dei nomi, disponibilità di hard link e symlink, pubblicazione concorrente e sostituzione con e senza lock sulla destinazione. Il parser PowerShell e la compilazione del codice C# sono stati verificati su macOS. L’esecuzione Win32 è in attesa di un host Windows 11 x64.

## Prove da completare

- Su Windows 11 x64 registrare versione e build di Windows, architettura, utente standard, filesystem dei volumi e configurazione che consente la creazione dei link. Su fixture locali confrontare nomi Unicode composti e decomposti, collisioni di maiuscole e minuscole, nomi riservati e percorsi lunghi. Registrare gli errori per accesso negato, elemento rimosso e destinazione discendente. Eseguire i casi su hard link, symlink, junction, file `.lnk` e ciclo.
- Nel desktop Windows provare la rimozione nel Cestino con `IFileOperation` e il ripristino, registrando esito per elemento, annullamento e comportamento con Cestino indisponibile. Su macOS estendere le prove alle protezioni di sistema e ai volumi candidati con Cestino indisponibile. Le prove di apertura associata, terminale, clipboard e drag-and-drop con le applicazioni native si collegano anche a DA-003 e DA-004. [Apple FileManager](https://developer.apple.com/documentation/foundation/filemanager/trashitem(at:resultingitemurl:)), [Microsoft IFileOperation](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-ifileoperation), [Flag di IFileOperation](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperation-setoperationflags).
- Su NTFS provare `MoveFileExW` senza `MOVEFILE_REPLACE_EXISTING` con due pubblicazioni sullo stesso nome e `ReplaceFileW` con una sostituzione esplicita. Verificare contenuto, metadati e stato di tutti i percorsi dopo un errore, anche con destinazione aperta senza condivisione per la cancellazione. Su ciascun target registrare anche gli esiti di staging interrotto, spazio insufficiente, modifica concorrente della destinazione e copia fallita tra volumi. La prova APFS di `rename` copre il cambio del nome nel filesystem durante l’esecuzione. Durabilità dopo crash e metadati richiedono test dedicati. [Microsoft MoveFileExW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw), [Microsoft ReplaceFileW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew).
- Stabilire il minimo macOS da collaudare e registrare nella matrice i filesystem candidati e le operazioni disponibili per ogni combinazione prima di chiudere DA-001. Build, installazione e avvio dell’app saranno verificati sui target quando esisterà lo scaffold.
