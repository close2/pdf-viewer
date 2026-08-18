# 573 — The grid a window could not see

**Finding:** a magnified page's type 1 shading was resolving its whole domain while the window
showed a twentieth of it — 5.5% of the cells at 8×, and only 55.7% at 1×, because two corpus pages
are larger than the window at their own natural size — and cutting the grid to the block a target
can sample took a zoom step on `function_based_shading_cmyk.pdf` page 2 from **249.4 ms to 3.9 ms**
without moving one bit of any cell.

Date: 2026-08-18. Argued by [ADR 0408](../adr/0408-the-grid-a-window-could-not-see.md), which takes
the lever [ADR 0406](../adr/0406-the-series-a-page-recomputed-once-per-pixel.md) section 7 named and
did not take.

Touched: `crates/pdf-render/src/shading.rs` (`Patch`, `ColourGrid::covers`, `onto_shading`,
`sampled_at`), `crates/pdf-render/src/lib.rs`, `crates/pdf-model/src/shading.rs` (`Block`,
`cells_touched`, `FunctionColours::row`), `crates/render-cpu/src/{lib,shading}.rs`,
`crates/render-quorra/src/scene.rs`, `crates/test-scenes/src/lib.rs`,
`crates/pdf-model/examples/shading_grid_census.rs` (new),
`crates/render-quorra/examples/zoom_frame.rs` (`ZOOM_FRAME_WINDOW`),
`crates/pdf-model/tests/shadings.rs` (two tests), `crates/render-cpu/tests/sampled_shading.rs`,
`doc/conformance/ledger.toml` (§8.7.4.5.2, §10.7.3).

## The round in order

### The premise was checked before the code was

ADR 0406's sentence contains two claims about *this tree*, and `doc/todo/16`'s rule applies to
both. The first — that a viewer past fit rasterises the window — is **false of `viewer-core` and
true of the host**: `Viewer::render_if_needed` builds `TargetSpec::for_page`, the whole page at the
magnification, and `viewer-ui`'s `surface.rs` then throws that extent away and states the window's,
keeping the transform and composing the scroll origin into it. `zoom_ladder.rs` has said so in its
first sentence since it was written. Half an hour reading two files, and it decided which target
the census had to model.

The second is the size of the waste, and it was measured rather than argued.
`examples/shading_grid_census` asks the shipping producer for both arms in one sitting —
`Patch::whole`, which is what every backend used to ask for, and `Shading::sampled_at`, which is
what they ask for now — so neither side is a second implementation of the conditions.

### Where the waste was, including where the quadratic argument does not reach

| zoom | cells resolved | cells the window samples | share |
|---:|---:|---:|---:|
| 100% | 2 763 241 | 1 537 870 | 55.7% |
| 200% | 11 052 481 | 3 050 966 | 27.6% |
| 400% | 18 288 961 | 3 498 332 | 19.1% |
| 800% | 31 875 844 | 1 754 963 | 5.5% |

The 1× row is the one worth keeping. A quadratic-in-magnification argument predicts 100% there and
the answer is 55.7%, because `function_based_shading_cmyk.pdf` page 2 is bigger than a
900 × 1100 window at its own size: **a reader who never zoomed was already paying for twice the
grid on their screen.** The lever's own framing would have missed that page entirely.

And the answer to "is it already clipped somewhere you have not found" is *nearly*: §8.7.4.5.4's
radial raster takes a `within` rectangle and §8.7.4.5.5's mesh clamps to the target's extent, both
written that way when `radial_gradients.pdf` made the cost visible. Type 1 was the one type in the
family that was not.

### Exactness, and the test that established nothing until it was rebuilt

The clip is worth nothing if a colour moves, and §8.7.4.5.2 states in its own words that the
function "need not be smooth or continuous", so the bar is bits rather than levels. The guarantee
is structural: `FunctionColours::row` computes a cell's centre from its index in the **lattice**,
adding the block's origin first, so a cell inside a block is the same coordinate in the same `f32`
bits as it would have been unclipped. The budget is applied to the lattice and not to the block for
that reason and no other.

**The end-to-end test passed with the margin deleted, which meant it was proving nothing**, and
fixing that was the most useful hour of the round. At one cell per device pixel the cell centres
coincide with the pixel centres and the bilinear filter's second weight is zero, so no margin is
needed and none of it shows. Three conditions together are what make the filter reach past a
block's edge, and every constant in
`a_window_of_a_magnified_page_draws_what_the_whole_page_has_there` now exists to satisfy one of
them: a magnification at which `MAX_FUNCTION_CELLS` halves the lattice, an offset that puts the
block's first cell on a whole lattice cell, and a discontinuity of the function at that very cell.
With all three, deleting the margin moves 151 pixels — one whole column of the window. The
producer-level test was checked the same way, by making `row` use block-relative indices.

That is trap 2's fifth lesson in a different clause: *a scene must fail at the defect's magnitude
as well as in its axis*, and here the magnitude was a property of three constants at once.

### What it moved

`zoom_frame` gained `ZOOM_FRAME_WINDOW`, because a page-sized target contains the whole of every
shading's domain and nothing about this change can show there. The arms were taken by removing the
suspect — `Patch::for_target` returning `Patch::whole` — rather than by two trees.

`function_based_shading_cmyk.pdf` page 2, a `DeviceCMYK` type 1 shading that no device program can
be built for, so the grid is its only path: a zoom step at 8× went **249.4 ms → 3.9 ms** and
34.5 MB of uploaded grid → 219 KB. At 1×, with no zoom at all, 54.3 → 31.8.
`function_based_shading.pdf`, nine shadings: 47.5 → 6.9 at 8×, with `scene` 44.0 → 0.3.

`pi_seven_segment.pdf` — ADR 0406's own witness — is **unchanged and uploads 32 bytes at every
rung**, because the device evaluates its program per fragment and builds no grid at all. Two
levers, two paths; a page on one is not on the other.

### The denominator, stated rather than buried

5 pages carrying 20 type 1 shadings, in 4 documents, over 1 479 corpus files — and two of those
four documents are this project's own fixtures, so the real witnesses are two pdf.js files. That is
ADR 0406's denominator one shading type wider and it is the same conclusion: **the corpus barely
exercises §8.7.4.5.2.** What the change is worth is therefore what happens to a reader of one of
those pages, not a corpus average, and on the CMYK page that is a quarter-second frame becoming
four milliseconds.

## What was considered and not taken

- **Clipping to the *shape* as well as to the target.** §8.7.4.5.4 already does exactly this and it
  is exact by the same argument; a shading pattern filling a small path over a large domain still
  resolves the whole window's worth. It needs the shape's device bounds computed in `pdf-render` so
  the two backends cannot answer it differently, which is the next rung.
- **Spending the cell budget on the block rather than the lattice.** Past `MAX_FUNCTION_CELLS` a
  magnified page draws from a grid coarser than its own pixels, and bounding the block would make
  it one cell per device pixel again — faster *and* sharper. §10.7.3 permits it. It is not taken
  here because it changes pixels and this decision's whole claim is that it changes none; it wants
  its own argument and its own comparison against the oracle.
