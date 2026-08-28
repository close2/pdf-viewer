# 792 — The font whose bytecode is the artwork

The batch's general-improvement round, chosen rather than assigned. It took the four crawl-head
rows `doc/todo/03` §27 left "placed but not settled", settled all four with the instruments that
entry had named — and one of the four turned out to be a defect of this tree, now fixed: a
TrueType face whose glyph shapes are *computed by its instruction programs* was drawn from the
uninstructed skeleton, and the hint-reliant family now draws through `skrifa`'s interpreter
(ADR 0727). 2026-08-28.

## Why this item

The sweeps offered reading lists and no verdicts; the standing bands' open defects are either
owner-gated or already priced. What the instruments did name was §27's own closing paragraph:
four rows of the final chunk's head, each placed by *where it cannot be*, with the settling
instrument written beside it — trap 9's colour probe for one pair, resolution attribution for the
other. Siblings held the errata, confined-boundary and text-extraction lanes, so the corpus
diagnosis lane was free. All four rows reproduced first (−6.275, −4.003, −3.259, −2.635 against
recorded −6.295, −3.988, −3.241, −2.606) before anything was diagnosed.

## What was found

- **`7803013.pdf` was ours.** The persistent −2.25 at 8× was *shape*: the embedded
  `DFKaiShu-SB-Estd-BF` subset builds each CJK glyph from stroke-component composites whose
  final assembly is done by the font's TrueType instructions. Drawing what `glyf` states —
  confirmed against an independent decomposition — reproduces our thin, misassembled render;
  FreeType draws it correctly even under `FT_LOAD_NO_HINTING` because the face is on its
  tricky-font list and the interpreter is forced. `skrifa` carries the same detection as
  `OutlineGlyphCollection::require_interpreter`, with the fix's contract in its documentation.
- **The fix**: `LoadedFont` builds one `HintingInstance` (Interpreter, Mono) per hint-reliant
  font, at **one pixel per design unit** — the grid the program rounds to is the design grid, so
  shape construction runs in full and grid-fitting stays declined, and the outline stays
  resolution-independent for the per-glyph cache. Failure falls back to the skeleton. Witness
  ink 16.28 → 18.51 at 8× against references at 18.52; row seeded in
  `doc/checks/fixed-documents.toml` and trap-13 calibrated both ways (defect planted: the gate
  fails the row by name at 16.214; restored: 18.469 in band). Two hermetic unit tests carry
  the mechanism onto machines without the crawl — a two-glyph fixture font whose instruction
  program moves one corner a tenth of an em — each calibrated by planting its own defect
  shape; the first plant of the fallback test failed to fail it and was replaced by the shape
  the test actually guards, which is trap 13 doing its job on the test rather than the code.
- **The other three rows are decisions already argued**, now with by-construction probes
  recorded in ADR 0727: `7557015` is the sub-pixel stroke lattice (§10.7.4 departure + ADR 0308),
  not resampling — its photos agree with `pdftoppm` to 0.33; `7557305` is §11.4.7's page-level
  `/CS /DeviceCMYK` group (one probe entry reproduces our page colour exactly; poppler and
  mutool do not composite in the stated space, gs answers through SWOP); `7557122` is ADR 0510's
  darkest-few-percent ICC finding on the document's own FOGRA profile — on plain `DeviceCMYK`
  patches we and poppler agree byte for byte.
- **An instrument defect, found on the way in**: `tools/state.sh counts` printed
  `doc/corpora: 0` in every parallel worktree — the corpora there are symlinks and `find`
  does not follow discovered links. `find -L`; the worktree now reads 275.

## Cost

`callgrind_interpret` (fifty interpretations of ISO 32000-2 page 101, no family font, pinned
pool, A/B one sitting): 1,187,649,176 → 1,191,309,728, **+0.31%**, of which 0.21% is the
once-per-font-load detection and the rest codegen movement (`show_text` itself shrank). The
witness page pays its own bytecode: 6.2 ms → 14–16 ms interpretation.

## Gates

The full §2 sequence, split around the machine's load (siblings held it at 15–50, so the
reference-spawning gates went into the quietest window; §2's load rule): fmt (one formatting
diff, fixed, re-run green), clippy `-D warnings` green, nextest 2730 passed (the two new tests among them), doctests green,
fuzz check green; corpus 974 green (129 silent codes over 7 documents, unchanged); selection
and accessibility censuses green; dates, xmp, jpeg2000 green; fixed-documents 41 rows green
including the new one at ink 18.469 — one FAILED line in this round's own log is the trap-13
calibration plant racing the gate script, re-run green on the clean tree; conformance green
(11634 citations). Oracle: 1945 pages, agrees 983, contradicted 61, ambiguous 836, all
equality ratchets held, at a 100.0% reference-cache hit rate against the main tree's cache
(`PDFREF_CACHE`). Text: PDFBox 99.8%, pdftotext 99.2%, position verdict 98.26%, green.
Quorra: 957 pages, 932 agree, 22 differ, ratchet green. doc/todo/00 step 7's ink sweep
re-run over all 836 ambiguous pages from the oracle's artefacts: the standing names
reproduce — `issue16038` −5.642 and `issue7821` −0.957 to the thousandth,
`checkbox_no_appearance` −1.200 — and the deep tail is the incomplete list, so the font
change moved nothing outside its family.

## Files

`crates/pdf-font/src/loading.rs` (the fix), `doc/checks/fixed-documents.toml` (the row),
`doc/conformance/ledger.toml` §9.6.3, `doc/todo/03-more-corpora.md` §27, `tools/state.sh`
(`find -L`), ADR 0727, this file.
