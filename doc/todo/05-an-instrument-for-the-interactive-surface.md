# An instrument for the interactive surface

Status: **instrument 1 built (ADR 0333); instruments 2 and 3 remain** — ADR 0323 is the design,
with the measured reference-vs-reference spread the tolerance rests on. One round per remaining
instrument, in the order below.
Priority: 05 — standing band, because it is an instrument like 00 and 01 rather than a feature
Corpus: the denominator is stated per instrument in ADR 0323, with every refusal printed by reason
Clauses: §9.10 (extraction), §12.7 (forms), §7.5.6 (the incremental update a save appends),
§14.7–§14.9 (the tree a screen reader walks), §12.3.2/§12.6 (what a click does)
Code: instrument 1 is `tools/pdfref/src/extract.rs` (the extractors and their cache),
`crates/pdf-model/tests/text_extraction.rs` (the verdict and the derivation) and
`crates/viewer-core/tests/headless.rs` (the drag half); ADR 0323 names the `Query`/`Command`
surfaces the other two exercise

## Why

The 455–484 block summary says it in one sentence: **the gates measure a raster and the work has
moved off the raster.** A drag that selects text, a field filled and saved, a click that follows
a destination, a tree a screen reader walks: each is a capability `doc/HANDOVER.md` claims and no
independent judge has ever looked at, at corpus scale. Trap 1 — the metrics lie, look at the page
— has no analogue here yet, and the defects it would catch are exactly the kind that lived for
seventy-five sessions in `user_space_at` because *no gate clicks* (ADR 0118).

## The design, settled (ADR 0323)

Three instruments, three verdict shapes — one each, not one for all:

| instrument | verdict | judged against |
|---|---|---|
| selection/extraction geometry | **bounded**, split by axis | `pdftotext -bbox -cropbox` word boxes; drag composed end-to-end from the reference's box |
| the save path | **exact**, three assertions | the prefix property; our own readback; poppler and mupdf reading the saved file — plus the existing oracle over a saved sample |
| the accessibility tree | **ratchet**, stated as one | nobody: no reference puts a comparable tree on the bus |

The tolerance for the first was measured this round the way the raster `Tolerance` was, over
matched word pairs from the two references on the corpus's first pages: horizontal edges agree
essentially exactly, the vertical *extent* of a word box is each library's own convention and is
excluded from the verdict, vertical *centres* are judged relative to word height, and reading
order is settled **out** of the geometry verdict because the references answer the order question
differently from each other. The frame audit that must precede any comparison — MediaBox against
CropBox, `/UserUnit`, `/Rotate`, each found by measurement — is ADR 0323's Finding 1 and trap 3's
text-domain entry.

## The build items, one round each, in this order

1. **Selection geometry — built (ADR 0333).** The frame audit, the unique-word
   matcher, the two bounds re-derived by the instrument itself
   (`PDFVIEWER_SELECTION_SPREAD=1`, an environment guard on ADR 0282's rule), and the drag half
   in `viewer-core`'s headless harness with both endpoints from the reference's box. It runs in
   §2's existing `text_extraction` line with no new line, printing and **not yet gating**: the
   per-word bounds (horizontal edges 0.5 pt, vertical centres half the word's height) and the
   verdict distribution enter §2 only once they have held across rounds — the rule below — and
   the first full run's figures are session 498's history file, reprinted by the gate line
   itself. **What remains of this item**: the ratchet, once held; and the two self-inverse
   properties ADR 0323 puts beside the drag half (`Query::Offset` of `Query::Caret`'s point,
   `Selection::All` byte-for-byte against `Interpretation::text`), which no round has written.
2. **The save round-trip.** One synthetic `Edit::FreeText` per document plus `Edit::SetField`
   wherever a text field exists; the three exact assertions corpus-wide; refusals counted,
   printed, and ratcheted — under the default `Restrict(On)` a policy refusal is the policy
   working and must stay legible. The oracle-over-saved-files sample last, priced when built;
   encrypted documents are outside the cached-render half only (§7.6.3.2's fresh IV makes their
   saves non-deterministic, ADR 0129).
3. **The accessibility ratchet.** Counts in `tools/state.sh`, the first being *pages that
   answer at all* — the count that makes `doc/todo/31`'s 8192-element truncation
   corpus-visible — then structure/`/Alt`/headers/untagged-honesty, ratcheted in both
   directions like the oracle's lists.

Each instrument's numbers enter `doc/todo/02` §2 only once they have held across rounds, never
before.
