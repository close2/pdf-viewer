# 3006 — A stroke's rest and its §8.5.3.2 dots meet the backdrop once

Session 2080. Status: **implemented.** Supersedes, for this point, the order ADR 1875 set ("the rest is stroked and the
dots painted after it") where that order composited the two over each other. Context: review 2038 (finding B39).
Clauses: ISO 32000-2 §11.6.2, §8.5.3.2.

§11.6.2: "Portions of an object shall not be composited with one another." A path stroked with `S` is one object.
`pdf_render::split_degenerate` takes its zero-length round-capped subpaths (and zero-length dashes) out as filled
discs, because no stroker states them as §8.5.3.2 asks, and every backend then drew the rest and the discs as two
marks. Under a paint that is not opaque, or a blend that is not Normal, the overlap took the paint twice: an
alpha-0.5 black line with a dot inside it was red 64 over white where it is 128.

Now each backend draws the two portions so that the union meets the backdrop once:

1. **`render-cpu`** draws both in opaque white onto a buffer of the target's size (the union of their shapes is its
   alpha), meets that shape with the clip and soft mask as one mark's coverage (`scan::own_coverage`: §10.7.4's set
   by `min`, §11.6.5's value by a product), and fills the surface once with the stroke's brush through it.
2. **`render-raster`** encodes both as the elements of an isolated knockout group of their own
   (`Encoder::one_object`) that carries the stroke's clip, mask and blend: the dots replace the rest within their
   shape and the group composites once.
3. **`render-gpu`** draws both in an isolated layer over the marks' bounds and knocks the rest out under the dots
   (`knock_out`) before filling them, so the layer holds one application of the paint.

**Where it is not done.** An opaque solid colour under the Normal blend composites identically either way
(`(1 − a)(1 − b)` of the backdrop survives both), so the separate draw is kept there and costs nothing. On the two
device backends a dot §10.7.4 substituted (coverage carried in the alpha) keeps the separate draw, since a
knockout reads alpha as shape; so does, on `render-raster`, a stroke that is itself a knockout element, which
cannot be a group (ADR 1295). Both are marks thinner than the raster's quantum.

**Cost.** The CPU buffer is one surface-sized pixmap per such stroke, which only a path holding both a degenerate
subpath and an ordinary one under a translucent paint asks for.

Tests: `render-cpu::transparent_degenerate_spec_audit`, `pdf-model::stroke_composition_pdf_spec_audit`,
`render-gpu::stroke_composition_spec_audit`, `render-raster::stroke_composition_spec_audit` (both lanes).
