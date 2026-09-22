# 1208 — The draft that was free, the branch that was the eleven levels, and a bound with a reason

The colour round of batch thirty-two: §8.6.5.9's `ON` half, §11.3.4 and §11.5.3's "non-affine
route", and `colour::MAX_PRESSES` under trap 38.

## What moved

§8.6.5.9 `partial` → `implemented`; §11.6.6 and §11.7.2 `partial` → `departed`; §11.4.7, §11.5.3
and §11.3.4 stay `partial`, each narrowed to one named residue.

## Findings

**ISO 18619 is on the disk and its procedure is built.** The three texts ADR 1208 named were fetched
at the sizes it recorded, converted by `tools/spec-md.py` and rowed in `doc/third-party-data.md`;
cited by section, never quoted. `Profile::source_black_point` is the draft's clause 4.

**ADR 1208's diagnosis of ADR 0510's eleven levels was wrong.** Not the LUT-destination estimation —
that never runs here, since the destination is the display and section 4.2.4 gives such a profile a
black of L\* 0 — but section 4.2.3's branch for an *output-capable* CMYK source, whose local black
comes through the profile's perceptual `B2A`. Artifex's press carries one and disagreed by eleven
levels; `hayro`'s CGATS profile carries none and already agreed. `Profile::to_rgb` of the deep ink
is now (26, 35, 46) against (25, 34, 45), and a real corpus profile's darkest patch went from eight
levels out to one. ADR 1253.

**`MAX_PRESSES` was 8 with nothing above it saying why.** Trap 38's question has an answer: Table 69
names four rendering intents, §8.6.5.9 adds `/UseBlackPtComp` and forbids one of the eight pairs, and
§11.7.5.3 makes both select the conversion out — so one profile is **seven** presses and no bound
below seven may refuse a conformant single-profile page; Annex C is informative and states nothing
here. The new `examples/press_depth` reads `Interpretation::presses_named`: no page of the
974-document corpus names a press at all, and the deepest of 13 188 pages over a 5 977-document
sample of the crawl names one. The bound is twice the clause's floor, stated as such. ADR 1254.

## Measured, not built

The "non-affine route" is two questions and neither is a representability limit. §11.3.4 forbids
`Lab` as a blending space outright, and `ColourCube` is a sampled lookup. What is wrong is that
`into_parent_cube` hands it *identity* input curves: at side 33 that is 4.66 of 255 from the exact
conversion into a `CalRGB` with `/Gamma 2.2`, and it does not converge (3.40 at 65) because an
inverse gamma's slope is unbounded at zero. The fix is the cube's own curves; it moves pixels on 31
documents of 88 890, so it is a round with `raster_golden` behind it. `doc/todo/23` carries it.
