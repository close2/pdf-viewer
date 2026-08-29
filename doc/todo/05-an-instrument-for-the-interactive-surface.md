# An instrument for the interactive surface

Status: **all three built (ADRs 0333, 0334, 0342), and two of them gate — the first one's verdict
since ADR 0421 and the third one's counts since ADR 0425; what is left is the save round-trip's
ratchet, the standing rule at the foot of this file, and the named remainders under the items.** ADR 0323 is the design, with the measured reference-vs-reference spread the tolerance
rests on. Each instrument's first-run numbers are in its session's history file, not here. **The
file stays open for the rule rather than for the build**: instrument 1's geometry verdict and
instrument 3's counts have each held across rounds and therefore ratchet, instrument 2's have not,
and this is where that promise is kept.
Priority: 05 — standing band, because it is an instrument like 00 and 01 rather than a feature
Corpus: the denominator is stated per instrument in ADR 0323, with every refusal printed by reason
Clauses: §9.10 (extraction), §12.7 (forms), §7.5.6 (the incremental update a save appends),
§14.7–§14.9 (the tree a screen reader walks), §12.3.2/§12.6 (what a click does)
Code: instrument 1 is `tools/pdfref/src/extract.rs` (the extractors and their cache),
`crates/pdf-model/tests/text_extraction.rs` (the verdict, its ratchet and the derivation),
`crates/viewer-core/tests/selection_census.rs` (the drag at corpus scale, and the two
self-inverse properties) and `crates/viewer-core/tests/headless.rs` (the single-document drag and
the press-over-an-annotation regression); instrument 2 is
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
differently from each other. (The centre half was narrowed by ADR 0759: it is judged only where
§9.8.1's Table 120 states the band both boxes are built from, because a centre is a baseline plus
a band and where the file states no band the measure returns the excluded convention rather than
the position.) The frame audit that must precede any comparison — MediaBox against
CropBox, `/UserUnit`, `/Rotate`, each found by measurement — is ADR 0323's Finding 1 and trap 3's
text-domain entry.

## The build items, one round each, in this order

1. **Selection geometry — built (ADR 0333), ratcheted and composed (ADR 0421).** The frame audit,
   the unique-word matcher, the two bounds re-derived by the instrument itself
   (`PDFVIEWER_SELECTION_SPREAD=1`, an environment guard on ADR 0282's rule), and the drag half
   in `viewer-core`'s headless harness with both endpoints from the reference's box. It runs in
   §2's existing `text_extraction` line with no new line. **Its verdict gates since the
   five-hundred-and-eighty-sixth session**, which is the rule at the foot of this file being kept
   rather than waived: the figures held from session 498 to session 586 — the same fraction of
   matched words in bounds, one more document judged — so the gate now carries a named list of
   the documents with a word out of bounds, checked in both directions, and a floor under the
   judged set (trap 11's arithmetic as a ratchet).

   **And the composed half is corpus-scale**, in `crates/viewer-core/tests/selection_census.rs`
   with a §2 line of its own: poppler's word box → device pixels → `Command::Pointer` →
   `Query::Selection`, over every corpus document at a *fitted* magnification, beside the two
   self-inverse properties ADR 0323 asked for. It found a defect on its first run — a press over
   an annotation set no selection anchor at all, because §12.5.5's appearance state is changed
   before the anchor is taken and changing it discards the interpretation.

   **What remains of this item**: the drag fraction's own ratchet, once it has held across
   rounds, and the eleven drags that still miss, which ADR 0421 names in four classes.
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
   Three *decisions* are asserted whatever the counts do — no panic, no untagged page given a
   structure it does not state (ADR 0214), and no line whose characters disagree with its own text.
   **And the counts ratchet since ADR 0425**, the rule below having been met: every one of them was
   unchanged from this instrument's own round to the five-hundred-and-fifty-ninth, which added a
   caret to all of them and moved none. A capability has a floor and a defect class a ceiling, the
   population is checked before either, and the line is in `doc/todo/02` §2. Its first run found a
   defect on the page-object join and `doc/todo/31`'s two residues now have numbers — both in ADR
   0342.

   **And it counts what this program *refused*, since ADR 0573**, which no count here did: a
   refused image drew nothing, §14.8.3.3 derives an element's rectangle from what its content drew,
   and the elements that lost their only place were counted with no cause beside them. Two printed
   counts — elements enclosing a refusal, and elements with **both** no place and a refusal inside
   them, per page with the page's own report sentence. **Neither is ratcheted**, by the rule at the
   foot of this file: they are one round old. `placeless_and_refused` is a defect class and wants a
   ceiling once it has held; `refused` is a denominator and may want no bound at all, and a later
   round decides which of the two it is putting a bound under.

   **What remains of this item is instrument 2's ratchet**, plus what each item names.

Each instrument's numbers enter `doc/todo/02` §2 only once they have held across rounds, never
before.
