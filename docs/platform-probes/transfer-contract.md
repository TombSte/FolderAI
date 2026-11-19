# File transfer and undo contract

This implementation contract maps RF-017–023, RT-006–009 and RT-013 onto the [native evidence](README.md) and [local data journal](../local-data/contract.md). Product choices below define the initial behavior. Native acceptance and the remaining recovery proofs are tracked separately.

## Destination, naming and publication

Capture the owning window, source locators and proposed destination at invocation. Resolve the native path again before an effect, and verify that the observed objects still match the captured references. A folder transfer into its own subtree fails before staging. Validate the native leaf name without rewriting Unicode or assuming case sensitivity. Treat the destination filesystem and its exclusive publication primitive as the authority for name equality.

Keep the publication relative to a verified directory handle on adapters that support it. A parent rename between check and publish can leave the completed output under the original directory's new name. Compare the live parent against the intended destination again before reporting completion. If they diverge, record the witnessed location for recovery and keep the source for any move that still requires source removal. The APFS directory substitution probe verifies the handle behavior. Windows needs equivalent evidence for directory handles and reparse points.

When an ordinary transfer collides, try the original name followed by ` - copy`, ` - copy (2)` and later candidates. A file suffix goes before its extension. A directory suffix goes after its whole name, and a directory transfer publishes one independent tree. An exclusive publication collision advances to the next candidate only after the engine knows that the attempt published nothing. Every publication attempt has its own journal identity under the same logical item and operation. A retry after an uncertain outcome inspects the prior attempt before it reserves another name.

Explicit replacement binds the user's approval to the selected target and observed version. Stage the complete copy beside the destination on the target filesystem and retain the staging witness. Space preflight can reject early. A successful preflight still allows a later write to return `ENOSPC`. If staging fails or is interrupted, preserve the original destination and remove owned staging only. Surface a separate cleanup error if that removal fails.

APFS exchange can displace a different target despite an immediately preceding identity check. The adapter must verify the displaced identity and version, keep both objects while ownership is uncertain, and recheck both names before any compensating action. APFS exchange evidence alone establishes that both contents survive the primitive. Enabling replacement also requires a validated recovery path for a concurrent change after exchange. Native Windows replacement requires its own evidence for backup disposition and final NTFS identity.

## Operation and item outcomes

The activity belongs to its initiating window. Its visible flow is Planning, Running, Waiting for decision, Stopping and Finished. A pending choice stops the whole affected transfer at the safe boundary and keeps navigation and other operations usable. A choice carries the operation, item and current decision identity so that a delayed command cannot resolve a later prompt.

| Evidence at the boundary | Item outcome | Next permitted effect |
| --- | --- | --- |
| Source cannot be read and processing can continue on other entries | Waiting for decision | Offer Retry, Skip and Stop. Revalidate the failed item before retry. |
| Destination directory or volume becomes unavailable | Destination unavailable | Recheck the captured destination on explicit retry. Skip and automatic reuse of the previous source-error choice do not resolve this condition. |
| Staging stopped before publication | Cancelled or failed, with staging status | Remove only verified, operation-owned staging. |
| Exclusive publish returned collision without an effect | Candidate occupied | Close the attempt and journal a new candidate for the same logical item. |
| Publication succeeded and its metadata commit failed | Recovery pending | Verify the same native witness and retry metadata. Preserve the published output. |
| Publication succeeded and source removal did not | Destination published, source retained | Recheck the published object and source witness. Resume removal only if both still match. |
| Publication outcome or object identity remains uncertain | Recovery pending or conflict | Inspect journal and native evidence. Preserve all uncertain paths. |
| Publication and required metadata commit succeeded, with source removal confirmed for a move | Completed | Update the UI and the current window's undo eligibility. |

Check cancellation between copy chunks and before publication. If cancellation arrives inside a native effect, inspect the outcome before cleanup. Earlier completed entries remain completed. A window close asks whether to stay or stop its own active transfers. Closure waits for safe boundaries and durable outcomes for that window.

For cross-volume moves, journal the source identity and destination publication separately. Preserve the original item and descendant IDs. Verify the completed native copy and source continuity before removing the source. If a source removal is blocked, report both surviving objects and keep the partial outcome available to recovery. Treat making another copy as a different operation.

## Trash and undo

Ordinary deletion uses the native trash adapter and records its verified restore reference where available. A trash error preserves the remaining source, reports the affected item and leaves permanent deletion as a separate action with confirmation. Successful entries remain in the trash when another entry fails.

Undo is a transient slot per window that holds at most one eligible action. Undo uses exclusive publication when restoring an original name and checks native identity and later changes before applying an inverse. A conflict preserves the current object and the conflicting path. The durable recovery journal records interrupted effects independently of whether an undo command is available after restart.

## Implementation choices

- **Names.** Add suffixes to the selected stem, preserving `.tar.gz`, `.tar.bz2`, `.tar.xz` and the ordinary final extension. Treat names with a leading dot and no further dot as extensionless. Apply keep-both to mixed file and directory collisions. Report a scoped name error if the generated component exceeds the filesystem limit.
- **Errors.** Group an item-scoped failure by stage and semantic error class within the same operation. Keep destination unavailable, full staging volume, cleanup failure and uncertain publication distinct.
- **Undo batches.** Apply verified inverses in reverse completion order, consuming each entry after success and leaving unresolved conflicts in the current window. A fully failed operation leaves the previous undo slot. A later completed action with no supported inverse clears it. A platform-level trash failure stops that batch with unprocessed items intact.
- **Replacement entry point.** Offer a separate explicit action for regular-file replacement after the adapter proves its recovery path. Other transfers use keep-both. Folder merging and replacement of a directory remain unavailable.

## Acceptance follow-through

Exercise two contenders for the same destination, the second item interrupted after the first committed, actual staging exhaustion, injected cleanup failure, a displaced replacement or edit during the commit boundary, and a cross-volume publication followed by a denied source removal. In each fixture assert item identity and content at every surviving path, outcome and retry behavior. Verify undo with an occupied restore name, an intervening modification, a mixed-result batch, independent windows and app restart. Native Windows proof and the complete release flows remain pending.
