# ADR 0333 — The selection-geometry instrument, built as designed

Status: accepted, 2026-08-14. Builds ADR 0323's instrument 1, its build item 1, exactly as that
design states it; the design itself is not re-argued here.

## Context

ADR 0323 settled the design of three instruments for the interactive surface and measured the
reference-vs-reference spread the first one's bounds rest on — with throwaway scripts,
deliberately uncommitted, recording only the invocations. This round turns that first design into
committed code: the frame audit, the unique-word matcher, the two bounds re-derived by the
instrument itself over its real population, and the drag half in `viewer-core`'s headless
harness. What was decided *in this round*, rather than inherited, is below; everything else is
ADR 0323's.

## Decision

### Where the pieces landed

- **`tools/pdfref/src/extract.rs`** — `Extractor` (`pdftotext -bbox -cropbox`, `mutool draw -F
  stext`) and `ExtractionCache`. The cache follows `pdfref::cache`'s rules exactly: the key is
  the invocation itself (`command_signature`, so `-cropbox` is in the key by construction — trap
  10a), plus the extractor's version and the document's SHA-256; deterministic failures are
  stored; a timeout is its own entry kind believed for a week. It is a second small
  implementation rather than a generalisation of `Cache`, and that is a decision with its reason
  in the module: the render cache's payload is a PNG with evidence-directory obligations, this
  one's is a string, and one struct serving both would put those obligations behind a type
  parameter for two callers. It shares the render cache's directory (`PDFREF_CACHE`, same
  default, an `extraction/` level down) because it is the same kind of memory under the same
  rules.
- **`crates/pdf-model/tests/text_extraction.rs`** — the corpus instrument, two `#[ignore]`d
  tests in the binary `doc/todo/02` §2's existing line already runs:
  - `the_word_boxes_we_place_agree_with_the_references` — the verdict half, run by the gate line
    with no new line, **printing and not yet ratcheting** (ADR 0323's rule: an instrument's
    numbers enter §2 only once they have held across rounds). It asserts only that it has a
    population, because a judged set that silently went empty would otherwise print a perfect
    verdict over nothing.
  - `the_selection_bounds_against_the_references_own_spread` — the derivation, poppler against
    mupdf over the same matcher and audit, so the bound's derivation lives next to the bound. It
    declines by itself unless `PDFVIEWER_SELECTION_SPREAD=1` is set, which is ADR 0282's rule:
    `-- --ignored` runs every ignored test in a binary, an invocation can be copied without its
    guard, and a test cannot be run without itself.
- **`crates/viewer-core/tests/headless.rs`** — `a_drag_across_the_references_word_box_selects_the_word`,
  ordinary and un-ignored: `pdftotext -bbox -cropbox` over the committed
  `doc/PDF20_AN001-BPC.pdf`, the three longest unique words, the viewport resized to the page's
  own point size at scale 1, `Command::Pointer` press → drag → release across the reference's
  box, and `Query::Selection` must contain the word. Both endpoints come from the reference and
  neither from this tree's geometry — trap 12a's rule, and the construction that catches
  `user_space_at`'s mirror on its first run, because the mirror of a word's box selects text
  that is not the word.

### The frame audit, as implemented

ADR 0323's Finding 1 names three mechanisms; the audit meets each explicitly rather than
generically:

- **Which box** — `pdftotext`'s stated page size must equal the unrotated, unscaled crop box
  this tree reads from §7.7.3.3, within one point per axis (the harness's one-pixel-per-axis
  rule at 72 dpi); `mutool`'s must equal the page *as displayed* — rotated, `/UserUnit`-scaled.
  A mismatch is a refusal by name, never a judged pair.
- **`/Rotate` and `/UserUnit` are normalised, not refused.** Every box is mapped into one
  canonical frame — poppler's own: points, origin at the crop box's top-left, y down, unrotated,
  unscaled — by inverting `base_transform`'s three steps explicitly (`Frame::reference_point`).
  So a rotated or `/UserUnit`-scaled document stays in the population, and the y flip that trap
  12a is about happens in exactly one named place per side.
- **One guard the design did not name**: a page whose §12.2 `/ViewArea` departs from the crop
  box is refused, because the display list's space is built from the view area and `pdftotext`
  knows nothing of viewer preferences. Table 147's default makes the two identical, and 0 of the
  974 state anything else, so the refusal is a tripwire rather than a population.

### The matcher and the verdict

Words are matched **by unique folded text** (both sides' counts exactly one, three characters or
longer — `reference_words`' own floor). This is what makes the comparison independent of
segmentation and order, the two things Findings 4 and 5 settled out of the verdict; a word an
extractor split differently simply fails to match, and the matched fraction is printed per
Finding 5 rather than judged. Per matched pair: horizontal edges within **0.5 pt** each,
vertical centres within **0.5 of the word's height** — the height being the mean of the two
boxes' heights, stated because the two conventions differ by a third and the choice moves the
number. Vertical edges are deliberately unjudged (Finding 3).

### What the first run found: the reference's stated frame is not its coordinates' frame

**The instrument's very first pass produced two outliers that were the builder's own frame
reading, and correcting it is this round's finding.** `hello_world_rotated.pdf` came back 152 pt
out and `issue14415.pdf` 517 pt out — both rotated pages — because the normalisation inverted
`/Rotate`, on ADR 0323 Finding 1's sentence that "`pdftotext -bbox` does not" apply it. Measured
against the pages themselves, that sentence is true of the reference's **stated page size** and
false of its **coordinates**: `pdftotext -bbox` prints the unrotated crop box in its `<page>`
element while its `<word>` boxes are in the rotated, displayed frame — on
`hello_world_rotated.pdf` (`/Rotate 90`) the stated size is 612x792 and the word boxes match
`mutool`'s 792x612 frame to the point, with `issue14415.pdf` (180) and `bug1947248_text.pdf`
(`/UserUnit 3`, boxes unscaled) pinning the other two mechanisms. Trap 3 one level deeper: **a
reference's stated frame and its coordinates can answer different questions**, and the audit now
checks each extractor's *stated size* against the entry it actually states while the
normalisation maps every box into the displayed frame in unscaled points —
`Frame::reference_point` carries the witnesses.

### What the first runs printed

The numbers below are a record of this round's runs, not a table for a later round to read
instead of running — the commands beside them reprint them, and the first-run figures live in
`doc/history/498-*`.

**The derivation** (`PDFVIEWER_SELECTION_SPREAD=1 cargo test --profile gates -p pdf-model --test
text_extraction -- --ignored --nocapture the_selection_bounds`) reproduces ADR 0323's throwaway
measurement from committed code: 12 778 matched word pairs over 516 documents, 458 refused by
name. Vertical centre deltas median 0.57 pt, p90 1.24, p99 2.96 against the design round's
0.59/1.23/2.79; height ratio median 1.29 exactly as measured then. The 0.5 pt horizontal bound
rejects **0.41%** of reference pairs and the half-word-height centre bound **1.23%** — both
above the references' own p90, at their p99, which is where `Tolerance`'s bounds sit relative to
their populations.

**The verdict** (runs in `doc/todo/02` §2's existing `text_extraction` line, ~11 s warm): 507 of
974 documents judged, 11 161 matched words, **98.26% in bounds**; 485 of 507 documents fully in
bounds; 467 refused, every one printed by reason, the largest being pages with no words in the
reference (292) and no unique matches (117). Trap 11's arithmetic is in the gate's own output:
each refusal is a document off the judged set, printed as such. What remains out of bounds is
named per document and is the population the instrument exists to name — `issue1350.pdf`'s
hundred Type 3 words agree horizontally to 0.00 pt and fail only the vertical centre, which is
the box-convention question ADR 0216 already argues, and rotated-text pages fail on the
cross-axis where the convention lies.

## Consequences

- The interactive surface has its first corpus-scale independent judge, and it runs in §2's
  existing gate line at the cost of one cached `pdftotext` invocation per document.
- The bounds gate nothing yet. The round that ratchets them reads the printed numbers off its
  own run, per ADR 0323's held-across-rounds rule; `doc/todo/05` carries that as the item's
  remaining half, along with the self-inverse properties (`Query::Caret`/`Offset`,
  `Selection::All` against `Interpretation::text`) that belong beside the drag half.
- `pdfref` now answers a second kind of question — *where does a reference say the text is* —
  under the same cache discipline as its first, so instrument 2's save-path readback (poppler
  and mupdf reading a saved file) has its plumbing precedent.
