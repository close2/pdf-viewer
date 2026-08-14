# 499 — The save read back by three readers

**Finding.** ADR 0323's second instrument is built and its first run is green over the whole
corpus: every §7.5.6 incremental update this tree writes keeps the producer's bytes as a
byte-for-byte prefix, reads back through this tree, and is seen — the annotation where it was
put, the field holding its value — by poppler and mupdf, with **zero undiagnosed
disagreements**. What the first run also found is three facts about the *references*: mupdf 1.28
refuses the empty owner password §7.6.4.4.11's Algorithm 12 accepts (`pr6531_2.pdf`), neither
reference implements §7.6.4.3.3's `SASLprep` (`saslprep-r6.pdf`, both accept the normalised
form), and nine documents this reader recovers are ones a reference cannot read at all — so a
reference that cannot read the original was made its own excluded category rather than a
disagreement, classified by pointing the same witness script at the original file.

**Date.** 2026-08-14.
**ADR.** [0334](../adr/0334-the-save-read-back-by-three-readers.md).
**Touched.** `crates/pdf-model/tests/save_round_trip.rs` (new — the instrument, one ignored
corpus test and one un-ignored single-document smoke test),
`crates/pdf-model/tests/save_round_trip/poppler_witness.py` and `mupdf_witness.js` (new — the
two reference readers), `doc/conformance/ledger.toml` (§7.5.6's row names the instrument),
`doc/todo/05-an-instrument-for-the-interactive-surface.md` (build item 2 done),
`doc/adr/0334-*` (new), this file.

## The first run's distribution

974 documents, under the default `Restrict(On)` (no wall-clock figure — the round ran beside
nine parallel ones, and the three full runs' wall clocks differed by an order of magnitude
with the tree unchanged, which is a measurement of the load):

- **Denominator.** 2 refused open (`PDFBOX-4352-0.pdf`, `issue21579.pdf` — the two encryptions
  this reader refuses, same as the corpus gate's `MAX_UNREADABLE_ENCRYPTION`); 7 with no page
  to annotate — the corpus gate's six pageless plus `issue9105_other.pdf`, which *renders*
  but whose page one is an **inline** dictionary inside `/Kids` rather than an indirect
  object, so §7.5.6's update (which replaces objects) cannot address it and Table 166's `/P`
  — "[a]n indirect reference to the page object with which this annotation is associated" —
  could not name it; 80 documents carry a fillable text field.
- **Policy census under `Restrict(On)` — the policy working:** 9 documents refuse *adding an
  annotation*: 8 by §7.6.4.2 Table 22 bit 6 (encrypted documents whose `/P` withholds
  annotation), 1 by §12.8.2.2 certification at `/P` 2 (`xfa_filled_imm1344e.pdf`,
  FormFilling). None refuses *filling in a form field* — bit 9 grants it at revision ≥ 3
  everywhere bit 6 is clear, and `print_protection.pdf` authenticates with its owner password,
  which §7.6.4.1 gives full access.
- **Saved and judged: 935** (933 free texts — the two annotate-refused-but-field-permitted
  documents saved the field edit alone — and 80 field values).
- **Refused by construction: 23**, every one `UpdateError::Recovered` — a cross-reference
  table rebuilt by scanning leaves §7.5.6's update nothing honest to chain to. 7 more saved
  nothing under `On` because every edit was policy-withheld (the census above names them).
- **Assertion 1 (prefix): 0 failures. Assertion 2 (readback): 0 failures. Panics: 0.**
- **Assertion 3: 0 disagreements, 0 reference errors, 9 exclusions** — references that cannot
  read the *original*: poppler cannot reach page one of `issue9418.pdf`, `bug1978317.pdf`,
  `poppler-395-0-fuzzed.pdf`, `poppler-67295-0.pdf`, `poppler-91414-0-53/54.pdf`; mupdf
  refuses `issue9418.pdf` ("expected object number"), `issue21436.pdf` ("too many kids in
  page tree") and `poppler-395-0-fuzzed.pdf` outright. All are files this reader opens through
  recovery the references do not perform.
- **`Restrict(Off)` over the 9-document differing population:** 8 saved (1 more
  `Recovered` refusal — `issue15893_reduced.pdf`), prefix and readback again 0 failures, and
  4 exclusions: `issue19484_1/2.pdf`, where poppler cannot reach the original's first page and
  mupdf fails on the original's own streams ("zlib error") — the self-contradictory double-key
  files `corpus.rs`'s `MAX_PAGELESS` note reads out. The encrypted saves themselves —
  §7.6.3.2's fresh vectors and all — were read correctly by both references everywhere the
  references could read the original, including every document that opens with the empty
  password and the six covering §7.6's revision/method matrix.

## What the disagreement diagnoses came to

Every candidate disagreement resolved into one of ADR 0323's three bins without a residue:

- **Their gap:** mupdf's Algorithm 12 refusal (`pr6531_2.pdf`) and both references' missing
  `SASLprep` (`saslprep-r6.pdf`) — handled by handing the reference the password it
  understands, with the diagnosis in `REFERENCE_PASSWORDS`; and the nine originals a
  reference cannot read, handled by the exclusion category.
- **A question for the file rather than §7.5.6:** `issue19484_1/2.pdf`, whose two key-length
  claims contradict each other, so what a save "should" decrypt to depends on which claim a
  reader believed. Our update is consistent with our stated reading.
- **Our defect:** none found. The instrument's exception list (`KNOWN_DISAGREEMENTS`) is
  empty.

## Gates

Recorded after the runs, per §2's rule that a number comes off the run: fmt clean; clippy
silent (one `semicolon_if_nothing_returned` in the new test found by the workspace run and
fixed); nextest workspace green, re-run warm after the final edit; doctests green; the
corpus, oracle, text-extraction, dates, xmp, jpeg2000, quorra and conformance gates all
exit 0 in sequence (this round changes no rendering and no readback — the instrument only
reads). The instrument itself: three full corpus runs, all green — the second after the
exclusion classification landed, the third after the inline-page split, with the
distribution identical throughout.
