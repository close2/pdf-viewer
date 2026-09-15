# 1083 — The substitution is what is coarser, not the converter

2026-09-15. No ADR: every number here is a measurement and every clause reading is already ADR
0268's, 0498's or 1082's. Files: `crates/render-raster/tests/corpus.rs` (`differing_pages`' note),
this file. Five siblings live in the tree. The oracle's undiagnosed head was empty for a ninth
round (`1960 pages in 50.8s`, `agrees 1010, contradicted 46, ambiguous 835`, nothing under the
ranking), so the contract was `render-raster`'s own differing list — whose standing diagnosis was
written for a quarter-pixel oracle.

**The claim that decayed.** `differing_pages`' note said the coarser of the two placements is this
tree's own because `tiny-skia` states a path edge on a quarter-pixel grid, that raster has no such
quantum, and that a text page here stays until that converter changes. Round 1068 changed the
converter (ADR 1082) and nine pages stayed. `examples/coverage_lattice` over them puts **both**
backends at the chance its own run prints — 25.0% for a ±1.5-level band: `endchar` 14.2/14.2,
`pr12564` 17.0/17.5, `standard_fonts` 15.5/14.2, `issue11473` 15.5/20.0, `issue12295` 21.3/10.8.
The lattice is gone and it was never what held these pages.

**raster does have a quantum and it is a glyph *phase*.** This gate has drawn at
`glyph_quantum: Some(16)` since ADR 0498. `PDFVIEWER_RASTER_GLYPH_QUANTUM=off` over the whole
corpus prints the same sixteen names at `935 agree, 16 differ, 7 refused, 16 not comparable`: worth
up to 0.32 of 255 of mean error (`pr12564` 0.6792 → 0.3632, `issue18030` 1.7311 → 1.4812,
`standard_fonts` 1.5342 → 1.4888) and worth no page. Trap 13's control is in the same run — five of
the sixteen do not move by a digit (`issue15150`, `issue16038`, `issue21068`, `issue2177`,
`issue269_2`), so the knob reached what it names rather than everything.

**What is coarser is `pdf_render::substitute_width`.** `sub_pixel_width_census`: six of the nine
state strokes under a device pixel — `issue12295` 65 859 at 0.1366, `standard_fonts` 516 at 0.5700,
`issue11473` 492 at 0.3985, `issue16038` 214 at 0.3985, `issue4402_reduced` 22 at 0.5000 and
`issue18030` 4 at 0.5000 — where `issue269_2` and `issue2177` state none. The oracle draws such a
rule one device pixel wide with the width it gave up in the alpha (ADR 0268, for a hairline that carried `cos θ` of a diagonal rule's area); raster
outlines it in path space at the stated width, so §10.7.4's third sentence is met by the oracle and
approached by raster. Trap 1: `issue11473`'s whole disagreement is 0.074% of its pixels inside one
140 × 116 box — the hatch swatches ADR 0268 was measured on — and `standard_fonts`' worst tile is a
table rule rather than a glyph.

**Two of the nine were mis-classified as placement.** `ink_ladder`: `standard_fonts` cpu 31 937.84
against raster 29 627.02 at 1× (7.8% apart) and `issue12295` 13 450.31 against 12 834.50 (4.8%),
where the other seven are inside 2% at every rung and both close by 4×; one of `standard_fonts`'
0.57-pixel rules reads 0.894 + 0.459 of two columns on the oracle against 0.965 of one on raster.
They belong with `issue19083` — narrowing it is `pdf_render::sub_pixel`'s round, not this backend's.

No list, ratchet or constant moved: the change is a comment. Gates are in the report.
