# Task: transfer-and-undo-contract

## Type
discovery

## Description
Specify error grouping, collision naming, replacement commit, partial results, retry, trash outcomes and recoverable action rules.

Traceability covers DA-005–006 RT-006–009.

## Depends On
- `desktop-foundation/platform-capability-matrix`
- `local-data/data-and-identity-contract`

## Likely Files
- Rust filesystem spikes and decision register

## Progress
- [x] Draft destination publication, per-item outcomes, recovery, trash and undo from the requirements and native evidence.
- [x] Record the initial implementation choices. Native release coverage remains subject to adapter and fault-injection tests.

## Validation
- Operation states and item outcomes were mapped to RF-017–023 and the local-data journal. Explicit replacement still needs its engine recovery proof on each enabled platform.
- macOS Apple Silicon/APFS on 2026-11-19. Built the [native probe](../../../../../../docs/platform-probes/macos.m) with `xcrun clang -Wall -Wextra -Werror -fobjc-arc -framework Foundation` and ran it with `/private/tmp --swap-only`. All 10 checks passed, including displaced-file identity, missing staging, a foreign object substituted before swap, an intervening in-place edit and fixture cleanup. This verifies primitive behavior. Runtime authorization, recovery, durability and Windows replacement remain open.
- The same native probe with `--space-only` passed on a disposable 128 MiB APFS image. Staging wrote 129,499,136 bytes before receiving `ENOSPC`. The probe removed the partial staging file and checked the destination against the original open handle and expected content. The engine still needs to expose this outcome and distinguish a later cleanup failure.
- The `--anchor-only` APFS probe passed all five checks. After renaming the captured destination directory and placing a symlink at its previous path, descriptor-relative exclusive publication reached the captured directory and preserved the unrelated directory. The adapter must still revalidate the intended visible location and withhold source removal if it diverged.
- The `--cross-only` APFS probe passed all ten checks with a second volume on a disposable 128 MiB APFS image. Staging, synchronization and exclusive publication succeeded, then removal returned `EACCES` with both native identities and contents intact. After restoring directory permissions and verifying both held handles, a removal-only retry succeeded and the published identity stayed unchanged. The image was detached. Journal-driven retry and logical identity rebinding remain engine tests.

## Notes
- See the specifications under `requirements/` for detailed acceptance criteria.

## Handoff
- 2026-11-18 codex. Planned and ready once dependencies and implementation autonomy are resolved. Review the cited acceptance criteria, then work in the areas above. Validation is pending.
- 2026-11-19 codex. Architecture review identified three contract gaps to resolve when this task becomes active. Use separate verified source and publication witnesses for cross-volume moves so the original item and descendant IDs survive. Record destination published and source retained as separate outcomes when removal fails. Probe APFS `RENAME_SWAP` with a displaced-object check for explicit replacement and test concurrent target edits and substitution. Validate Windows `ReplaceFileW` terminal states and final NTFS identity on the desktop before enabling replacement there. Define atomic retry records for keep-both name collisions and a terminal move outcome for proven pre-effect failure. [Windows replacement outcomes](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew)
- 2026-11-19 codex. The [transfer and undo contract](../../../../../../docs/platform-probes/transfer-contract.md) records the initial behavior, transitions, journal boundaries and existing platform evidence. The engine and release tasks carry the remaining implementation and fault-injection checks.
