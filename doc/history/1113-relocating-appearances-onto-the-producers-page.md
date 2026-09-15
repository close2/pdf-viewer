# 1113 — Relocating a producer's appearance onto its own page

Round 1113 of the fifty-round run, batch 1111–1116. Built the on-page relocation construction
ADR 1120's fourth amendment put in scope and reserved ADR 1123 for: a forbidden annotation's
normal appearance now goes back onto the producer's own page under §12.5.5's matrix, rather than
only onto the appended page ADR 1099 built as the fallback.

## What was built

- **The `q`-prepend construction** (`preserve.rs`, `prepare.rs`, `rewrite.rs`): the page's
  `/Contents` becomes an array (§7.7.3.3 Table 31, single stream flattened by reference) of a `q`
  stream, the producer's own streams unchanged, and a closing stream issuing `depth + 1` `Q`s then
  §12.5.5's placement of each appearance. `depth` is counted by lexing the content, skipping §8.9.7
  inline-image data via `pdf_model::inline_image::scan`. The appearance is named in the page's
  `/Resources`. Balance is **§8.4.2**, kept across the array; no producer byte moves.
- **Two refusals, each falling back to the appended page**, with sentences on the report row
  (`Preserved::declined`) and in `--json`: a producer that pops further than it pushes (or nests
  past ISO 32000-1:2008 Annex C Table C.1's 28); and a remaining, unhidden annotation (§12.5.3
  `Hidden`/`NoView` clear) whose normalised `/Rect` (§7.9.5) overlaps the moved appearance's device
  rectangle — the population is every *remaining* annotation, because `/Annots` order is not z-order
  (the standard states none), so it over-refuses in the safe direction.
- New `Rewrite::RelocatedOnPage`; the marks site's preserve now relocates by default.

## Measured

Census (`archive_corpus.rs`, extended): over the veraPDF corpus, 16 appearances drive the remedy —
10 at PDF/A-2b, 6 at PDF/A-4 — and **all 16 relocate cleanly**; 0 hit either refusal. So the corpus
does not exercise the refusals, and the three fixtures in `archive.rs` are their calibration
(trap 8/13): the clean relocation, an extra `Q`, and an overlap, each naming its outcome. The main
archive sweep is unchanged: every conforming document stays conforming; converted counts hold.

## Records

ADR 1123 (the construction, its two refusals, the fallback). ADR 1120's forward references updated
from "future ADR (1123, reserved)" to built; CLAUDE.md's fourth-amendment paragraph names the
construction as built; `mod.rs`/`preserve.rs` prose and the §8.4.2 and §12.5.5 ledger rows carry
the converter-side reading with their `pdf-transform` code refs.
