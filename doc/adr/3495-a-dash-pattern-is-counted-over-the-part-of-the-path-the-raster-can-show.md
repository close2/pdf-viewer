# 3495 — A dash pattern is counted over the part of the path the raster can show

Session 2198. Status: **accepted**; built. Context: ADR 3334 (`MAX_DASHES`), ADR 3474 (the finding),
trap 2 (one decision for three backends), principle 3. Clauses: ISO 32000-2 §8.4.3.6, §8.5.3.2,
§10.7.4, §14.11.2.1.

## Decision

`pdf_render::dashing` replaces `past_the_dash_bound`, and answers one of three things for a stroke:
as stated, as stated over a cut path, or solid at the pattern's coverage.

1. **The whole path first.** Within `MAX_DASHES` the path is untouched, which is every page that
   drew as stated before; nothing else in this ADR reaches it.
2. **Past it, counted again over the region a mark can reach.** Each backend gives its target's
   pixels under the map from the path's space — `render-cpu` the whole page's grid, so every strip
   and window of a page answers as the page does; `render-raster` and `render-gpu` their target —
   and `pdf-model`'s report gives the page's `content_clip` under the stated width. The region is
   grown by a mark's reach at its edge: half the width times the mitre limit (under mitre joins)
   or `√2` (under projecting caps), and one unit for §10.7.4's whole pixel.
3. **The cut.** A subpath of straight segments keeps the part of each segment inside the grown
   region (Liang and Barsky's clipping, in double precision); each run of kept parts that does not
   continue the subpath's start becomes a subpath of its own, led in by a line from off the grown
   region, pointing away from it, as long in the path's space as the distance the subpath had
   travelled there modulo the period. §8.4.3.6 restarts the pattern at every subpath, so the
   piece's first dash begins where the whole path's did, and the lead-in's dashes lie outside the
   region. Kept as stated: a subpath inside the grown region, one holding a curve (whose length is
   the dasher's own measure), a closed one starting in the region (opening it would put two ends
   where the close drew a join), and one that is not `m`, `l`s and at most a final `h`.
4. **The bound still guards what is built.** A cut path past `MAX_DASHES` is drawn at its coverage,
   reported by name as before.

The count depends on the device only through the stroke's device width and the grown pixel, so a
stroke within a few dashes of the bound may be cut at one magnification and drawn at its coverage at
another, and `pdf-model`'s device-free count may differ from a backend's there. A tiled target
decides per tile. Both are this ADR's costs.

## Proof

The unit tests in `dash_bound.rs` hold the cut's dashes on the region to an independent dasher's,
for a rule and for a closed zig-zag leaving and re-entering the region forty times. `raster_golden`
and `drawn_ahead_in_strips` hold every tracked page as it was.
