# An instrument for the interactive surface

Status: **all three built (ADRs 0333, 0334, 0342); what is left is the standing rule at the foot
of this file and the three named remainders under the items.** ADR 0323 is the design, with the
measured reference-vs-reference spread the tolerance rests on. Each instrument's first-run numbers
are in its session's history file, not here. **The file stays open for the rule rather than for
the build**: no instrument's numbers have held across rounds yet, so none of them gates, and this
is where that promise is kept.
Priority: 05 — standing band, because it is an instrument like 00 and 01 rather than a feature
Corpus: the denominator is stated per instrument in ADR 0323, with every refusal printed by reason
Clauses: §9.10 (extraction), §12.7 (forms), §7.5.6 (the incremental update a save appends),
§14.7–§14.9 (the tree a screen reader walks), §12.3.2/§12.6 (what a click does)
Code: instrument 1 is `tools/pdfref/src/extract.rs` (the extractors and their cache),
`crates/pdf-model/tests/text_extraction.rs` (the verdict and the derivation) and
`crates/viewer-core/tests/headless.rs` (the drag half); instrument 2 is
`crates/pdf-model/tests/save_round_trip.rs`; instrument 3 is
`crates/viewer-core/tests/accessibility_census.rs`, asked by `tools/state.sh accessibility`

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
2. **The save round-trip — built (ADR 0334).** One synthetic `Edit::FreeText` per document
   plus `Edit::SetField` wherever a text field exists; the three exact assertions corpus-wide
   in `crates/pdf-model/tests/save_round_trip.rs`, with poppler asked through poppler-glib
   (no poppler CLI prints an annotation or a field value) and mupdf through its raw object
   layer; refusals counted and printed by reason — under the default `Restrict(On)` a policy
   refusal is the policy working and stays its own census, with `Restrict(Off)` run over
   exactly the differing population. A reference that cannot read the *original* is excluded
   by reason rather than counted as a disagreement. Nothing is cached (the saved file's hash
   is new whenever the writer changes, so `pdfref`'s key never amortises) and nothing needed
   sampling — the questions are object reads, not renders. **Still owed from this item:**
   ratcheting the census counts once they have held across rounds, a §2 line by the standing
   rule below, and the oracle-over-saved-files sample, which stays unbuilt at the price ADR
   0334 states (about two minutes of reference CPU per writer-change, unamortisable by
   construction; encrypted documents outside that half only, ADR 0129).

3. **The accessibility ratchet — built (ADR 0342).** `tools/state.sh accessibility` over
   `crates/viewer-core/tests/accessibility_census.rs`: page one of every document in
   `doc/pdf.js/test/pdfs` **and in `doc/`**, plus every page of every document that states a
   structure tree — the specifications are in the population because the corpus's tagged
   documents are 17 pages at their largest and the first count needs a document big enough for a
   bound to run out. *Pages that answer at all* comes first and an empty answer is **classified**
   by §14.7.5.4 rather than counted, into the file naming elements for a page that answers
   nothing (the defect class), the page stating no `/StructParents`, and the file naming nothing.
   Then structure by two predicates, `/Alt`, placement, headers, controls, and untagged honesty.
   Nothing is ratcheted yet, by the rule below; two *decisions* are asserted from the first run —
   no panic, and no untagged page given a structure it does not state (ADR 0214).
   **What remains of this item:** the ratchet itself once the counts have held, and a §2 line with
   it. Its first run found a defect on the page-object join and `doc/todo/31`'s two residues now
   have numbers — both in ADR 0342.

Each instrument's numbers enter `doc/todo/02` §2 only once they have held across rounds, never
before.
