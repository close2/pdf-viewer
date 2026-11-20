# 3334 — A dash pattern past half a million dashes is drawn at its coverage

Session 2158. Status: accepted. Context: audit 2155's finding B02, `CLAUDE.md` principle 3, trap 5.

## What was wrong

§8.4.3.6 admits any lengths that are "nonnegative and not all zero", so `[0.00001 0.00001] 0 d` on an
80-unit line is a valid pattern of four million dashes. `tiny-skia`'s dasher refuses past a million
dashes a path (`MAX_DASH_COUNT`, `tiny-skia-path` 0.12 `dash.rs`); its stroker then returns nothing and
`render-cpu` drew the whole stroke as nothing, with `convert.rs` saying a refused dash "yields a solid
line". `kurbo`'s dasher, under both graphics-device backends, has no bound and builds every dash.

## Decision

1. **The bound is `pdf_render::MAX_DASHES`, 500 000 dashes a stroke**, counted by
   `pdf_render::dash_count`: the path's length over the pattern's period times the dashes a period
   holds, plus one a subpath for the phase. The length is the control polygon's, closing segments
   included, which is never shorter than the curve, so the count is an upper bound on the one
   `tiny-skia` makes. Half that library's million is the margin for the two precisions; 500 000
   dashes is about 56 MB of `kurbo` path elements (two 56-byte elements a dash), the most one stroke
   may build on the device.
2. **Past it, the stroke is drawn solid at `pdf_render::pattern_coverage`**: the share of the line the
   dashes and Table 53's caps cover — the dash lengths, plus a width a dash for a projecting cap and
   `π w / 4` for a round one, over the period, at most one. This is §10.7.4's area reading taken over a
   period rather than per dash; it is exact where a period is finer than a pixel, which is where such
   counts arise, and it is a choice, not a derivation: the standard states no bound.
3. **Decided once, for all three backends** (trap 2): each asks `pdf_render::past_the_dash_bound` of
   the command's path before it dashes, and draws the rest at that coverage as an opacity — `tiny-skia`'s
   `apply_opacity`, a `vello` brush's `multiply_alpha`, raster's `faint`. A stroke so drawn takes the
   separate draw of its rest and its dots rather than §11.6.2's one-composite construction.
4. **Reported.** `pdf-model` names the bound where a stroke reaches it,
   `Unsupported::LimitReached { limit: "MAX_DASHES" }`, because a pattern drawn as its coverage is not
   the pattern (trap 5).

## Cost

Coverage as opacity is exact under the Normal blend outside a knockout group; inside one, and for a
non-solid paint on raster (whose builder carries alpha only in a solid colour), the stroke is drawn at
its whole coverage. Both are the report's to say.
