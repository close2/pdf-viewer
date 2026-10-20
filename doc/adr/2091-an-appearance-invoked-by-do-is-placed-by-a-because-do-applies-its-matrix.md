# 2091 — An appearance invoked by `Do` is placed by `A`, because `Do` applies its `/Matrix`

Session 1627, the clause slot of batch ninety-eight. Status: **accepted**, **built** in
`pdf-model`. Code: `crates/pdf-model/src/annotation.rs` (`alignment`, `alignment_of`, `placement`
now `/Matrix` then `alignment`), `crates/pdf-model/src/appearance.rs` (`placement`'s meaning and
documentation), `crates/pdf-model/tests/annotations.rs`
(`an_appearance_invoked_under_its_placement_draws_what_the_annotation_draws`).
Clauses: ISO 32000-2 §12.5.5, §8.10.1. Companion: ADR 2090; the construction is ADR 1123's.

## 1. What was found

Measuring ADR 2090's sweep, the veraPDF witness `PDF_A-4/…/6-3-1-t01-fail-g` — a `/line`
annotation whose appearance states `/BBox [107.614 679.983 228.476 692.983]` and `/Matrix [1 0 0 1
-107.614 -679.983]` — was converted to a page whose red line lies at the bottom-left corner, 759
pixels away from where the annotation drew it (looked at, trap 1). The relocation had written
`1 0 0 1 0 0 cm` before `/PreservedMarks Do`: `appearance::placement` returned §12.5.5's
`AA = Matrix × A`, and `Do` then concatenated the `/Matrix` a second time.

## 2. Decision

§8.10.1, of a form `XObject`'s `/Matrix`:

> Each time the form XObject is painted by the Do operator, this matrix shall be concatenated with
> the current transformation matrix to define the mapping from form space to device space.

So a caller that *invokes* the stream needs step 2's `A` as the operand of its `cm`, and the stream
is drawn under `Matrix × A`, which is step 3's `AA` to the bit — `placement` in `annotation.rs` is
now literally `matrix.then(alignment(…))`, the same arithmetic as before for the annotation path,
which runs the stream rather than invoking it and keeps `AA`. `appearance::placement` returns `A`
and says why; its one caller, the archive conversion's `marks_of`, needed no edit. With `cm` set to
`A` by hand in the converted file the page was byte-identical to the annotation path at 72 and
720 dpi before the code was changed; the test was red on HEAD, under `q 2 0 0 2 -180 -370 cm`.

## 3. What it reaches

Every relocation whose appearance states a `/Matrix` other than the identity, on the producer's
page and on an appended one alike (both write `Marks::placement`). The corpus's arms
do not move: `pdf-transform` is outside `pdf-model`'s, `render-cpu`'s and `render-raster`'s
dependency graphs, and the annotation path's own matrix is computed as it was.
