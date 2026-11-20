# Task: links-and-zip

## Type
implementation

## Description
Close the ZIP/link support matrix, then add link navigation/preservation and bounded archive browsing and extraction with guarded destinations.

Traceability covers DA-007 RF-027–028 RT-006 RT-010.

## Depends On
- `file-operations/file-transfer-engine`
- `desktop-foundation/platform-capability-matrix`

## Likely Files
- Rust link adapters and ZIP service, React archive views

## Progress
- [ ] Complete the work described above.

## Validation
- Pending. Test native links, permission errors, malformed/encrypted ZIP, collisions, resource exhaustion and containment under destination changes.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.

- pi: collegati anteprima ZIP ed Estrai qui (singolo file ZIP) in `src/workspace/FileMutationControls.tsx` con icone e IPC registrato. Estrazione accanto all’archivio, in cartella separata senza sovrascrittura. Test Rust attraverso le funzioni comando: anteprima, ZIP maiuscolo, doppia estrazione e integrità; 123 passati, 1 ignorato (Cestino). Clippy e build frontend passati. Restano da verificare interazione desktop e Windows/NTFS; nessun cambio di stato.

- pi: estensione approvata: Comprimi in ZIP da singola cartella, con icona nel menu e IPC Tauri. Archivio Deflate accanto alla sorgente, pubblicazione esclusiva e nome alternativo; link/elementi speciali rifiutati, limiti ZIP esistenti, staging conservato e segnalato in caso di errore. Round-trip file e sottocartella vuota e doppia compressione verificati su macOS; 123 Rust passati, 1 ignorato, Clippy e build frontend passati. UI desktop e Windows ancora da verificare.

- pi: rilevato processo da bundle release, non dev. Ricostruito bundle macOS con `npm run tauri build -- --bundles app`; serve chiudere e riaprire l’istanza già in esecuzione.

- pi: esteso Comprimi in ZIP al singolo file (`nome.ext.zip`), con lo stesso handler di cartelle e pubblicazione esclusiva. Round-trip file e cartella passato su macOS; 123 test passati, 1 ignorato, Clippy e bundle release macOS ricostruito. Riavviare il bundle per provare il menu; verifica interattiva e Windows ancora aperte.
