# An instrument for the interactive surface

Status: **design settled, nothing built** — ADR 0323 is the design, with the measured
reference-vs-reference spread the tolerance rests on. One round per instrument remains, in the
order below.
Priority: 05 — standing band, because it is an instrument like 00 and 01 rather than a feature
Corpus: the denominator is stated per instrument in ADR 0323, with every refusal printed by reason
Clauses: §9.10 (extraction), §12.7 (forms), §7.5.6 (the incremental update a save appends),
§14.7–§14.9 (the tree a screen reader walks), §12.3.2/§12.6 (what a click does)
Code: none yet; ADR 0323 names the `Query`/`Command` surfaces each instrument exercises

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

1. **Selection geometry.** The frame audit, the unique-word matcher, the two bounds re-derived
   by the instrument over its real population before they gate — then the drag half in
   `viewer-core`'s headless harness, with the drag's endpoints taken from the *reference's* box
   (trap 12a's rule: the point comes from outside the code under test). Lands as an `--ignored`
   test in `pdf-model`'s `text_extraction` binary — §2's line runs it with no new line — with
   the reference output under `pdfref`'s cache rules, the invocation in the key.
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
