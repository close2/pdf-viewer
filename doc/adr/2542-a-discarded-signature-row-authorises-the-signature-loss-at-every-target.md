# 2542 — A discarded signature row authorises the signature loss at every target

Status: implemented. Session 1854.
Context: ISO 32000-2 §12.8.1; ISO 19005-2 section 6.1.12; ISO 19005-4 sections 6.5.1 to 6.5.3; ADR 2535 section 4;
`doc/pdfa-remedies.md` sections 2 and 5 (the `signature-assertion` loss).
Code: `crates/pdf-transform/src/archive/config.rs` (`Configuration::authorisations`, `ASKED_AT_EVERY_TARGET`),
`crates/pdf-transform/tests/profiles.rs` (`a_discarded_signature_row_authorises_the_signature_loss_at_every_target`).

## Decision

A `discard` at `file-structure/document-signature-states-no-digest` authorises `signature-assertion` at every target,
part 4 included, although that row's requirement binds only part 2. A profile that wants a signed source refused at part
4 says so with a target-qualified row, `[site."file-structure/document-signature-states-no-digest".target."4"]` with
`remedy = "stop"`, which decides part 4 alone. The two shipped profiles that discard the row (`as-if-printed`,
`as-shown`) therefore convert a signed source at both parts; `refuse-any-loss` and `keep-everything` authorise it at
neither.

## Why it is a profile-table decision and not a fix

A profile's rows say what its author decided at the targets the rows bind. Before this change a row decided nothing at a
target where its requirement does not bind, and `authorisations` took only binding rows. Widening that is a change to
what a shipped profile deletes where its author wrote nothing, so it is argued from the table rather than taken as a
repair:

1. **The question is the conversion's, not the requirement's.** `signature_decision` asks it of every signed source at
   every target, because a rewrite moves every byte a signature's `/ByteRange` covered (ISO 32000-2 §12.8.1) whichever
   requirement caused the rewrite. ISO 19005-4 states no requirement whose answer is this loss — its signature rows
   (sections 6.5.1 to 6.5.3) state what a signature's widget meets, how a signature is made and how a timestamp
   follows the base standard, and none is answered by removing one — so at part 4 a profile had no row it
   could write to answer the question it is asked, and every signed source stopped with no site naming why (ADR 2535
   section 4 measured five such documents).
2. **The table keys the loss by loss.** `doc/pdfa-remedies.md` section 5 lists one `signature-assertion` loss and the
   same consequence at both parts: the output carries no signature, each signature field keeping its widget and
   appearance and losing its value. A profile author who accepted that consequence at part 2 accepted the same
   consequence, not a different one, at part 4.
3. **The narrowing is one entry.** `ASKED_AT_EVERY_TARGET` holds only `Loss::SignatureAssertion`; every other loss is
   still authorised only where its row binds, because each other loss is asked by its own requirement and does not exist
   where that requirement does not bind.
4. **The author keeps the last word.** The target qualifier already in the grammar is the way back, and the test above
   proves it decides part 4 without moving part 2.

## Rejected

- *A new part 4 site for the signature.* No requirement of ISO 19005-4 states it, and a site that names no clause would
  be the converter's invention presented as the standard's.
- *Leaving part 4 stopped.* It refuses a document the profile's author already accepted the loss for, and the census
  filed it under a decision no row could take.

## Evidence

The five signed documents ADR 2535 section 4 named convert at part 4 under `as-if-printed`, where HEAD's binaries refused
each with the signature sentence; each output passes veraPDF at part 4 and the three that convert at 2b pass at 2b.
