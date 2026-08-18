# ADR 0408 — The grid a window could not see

Status: accepted, 2026-08-18. Session 573. Cuts §8.7.4.5.2's colour grid to the part of the
domain a target can sample, across `pdf-render`, `pdf-model` and both backends that draw one.
Takes the lever ADR 0406 section 7 named and did not take. Consumes ADR 0339's device-resolved
grid and ADR 0364's parallel rows, and neither had to move.

## 1. The claim, and why it had to be measured first

ADR 0406 recorded three levers considered and not taken, and named this one as worth doing next:

> clip the shading grid to the part of the domain the target covers — exact, and quadratic in
> magnification once a page is zoomed past the window.

Two things in that sentence are claims about *this tree* rather than about arithmetic, and
`doc/todo/16`'s rule — a latency finding is a defect until it has been attributed — applies to
both. The first is that the viewer rasterises a window rather than a page. **`viewer_core` does
not**: `TargetSpec::for_page` is what `Viewer::render_if_needed` builds, and it is the whole page
at the magnification. **The winit host does**, and that is the path a person uses:
`viewer-ui`'s `surface.rs` takes the core's target transform, composes the scroll origin into it,
and states the *window's* width and height. `render-quorra/examples/zoom_ladder.rs` has modelled
exactly that target since it was written, and says so in its first sentence.

The second is the size of the waste. `crates/pdf-model/examples/shading_grid_census.rs` measures
it, in one sitting, from the shipping path on both arms: `Patch::whole` is what every backend
asked for before this decision and `Shading::sampled_at` is what they ask for now, so the
denominator is not a second implementation of the conditions. Over every type 1 shading in the
corpus, in a 900 × 1100 window with the page's middle held in the window's:

| zoom | cells resolved | cells the window samples | share |
|---:|---:|---:|---:|
| 100% | 2 763 241 | 1 537 870 | 55.7% |
| 200% | 11 052 481 | 3 050 966 | 27.6% |
| 400% | 18 288 961 | 3 498 332 | 19.1% |
| 800% | 31 875 844 | 1 754 963 | **5.5%** |

The share at 1× is already 55.7% rather than 100%, which the quadratic argument does not predict
and which is the more interesting half: `function_based_shading_cmyk.pdf` page 2 is larger than
the window at its own natural size, so a *reader who has not zoomed at all* was paying for a grid
twice the one on screen.

**The clause family had already been cut everywhere except here**, which is the answer to "is it
clipped somewhere you have not found". §8.7.4.5.4's radial raster takes a `within` rectangle —
the shape's device bounds intersected with the target — and §8.7.4.5.5's mesh clamps its own
bounds to the target's extent. Both were written that way for the reason this ADR gives, on the
page (`radial_gradients.pdf`) that made the cost visible. Type 1's grid is the one that was not.

## 2. The decision

`pdf_render::Patch` — a lattice, and the part of the domain's unit square a target can reach:

```rust
pub struct Patch {
    pub grid: Grid,
    pub within: [f32; 4],
}
```

`Patch::for_target` derives `within` by mapping the target's four corners back through the
placement and taking their bounding box, clamped to the unit square. It lives in `pdf-render`
beside `Grid::for_placement` and for that function's own reason: a decision either backend could
make alone is a decision neither has made.

`ColourGrid` grew one field, `covers` — the rectangle of the shading's own coordinates the cells
returned actually cover — and one method, `onto_shading`, which is the transform that places
them. Both backends now build their placement from that method and **neither reads the shading's
`/Domain` for it any more**, which is one expression for a block and for a whole grid rather than
two. `render-quorra`'s `sampled_fill` lost four lines net; `render-cpu`'s `sampled_shader` lost
three.

## 3. Why it is exact, which is the whole point

The clip is worth nothing if a colour can move: a shading whose colours shifted at a zoom
boundary would be far worse than a slow one, and §8.7.4.5.2 says in its own words that "[t]he
function need not be smooth or continuous" — so a sample that moved by one ulp across a
discontinuity is a whole unit of colour, not a rounding. ADR 0406's own witness draws digits
through a `truncate`.

**The guarantee is structural rather than empirical.** A cell's colour is the function at the
cell's centre, and the centre is computed from the cell's index in the **lattice** — never in the
block. `FunctionColours::row` adds the block's origin to the row and column before it computes
anything, so cell (i, j) of the lattice is the same coordinate, in the same `f32` bits, whether
the block is the whole grid or a corner of it.

Three places where that could have been given away, and what each cost:

- **The budget.** `MAX_FUNCTION_CELLS` is §10.7.3's "internal limits" and it halves both axes
  until the grid fits. It is applied to the **lattice**, before the block is cut. Applying it to
  the block instead would be bounding the *work* rather than *where the samples fall*, and a
  magnified page would then get a finer lattice than it had unclipped — faster and better and
  **different**, which is section 6's lever and not this one.
- **The domain rectangle a block reports.** `x0 + 1.0 × (x1 − x0)` is not `x1` in `f32` for most
  pairs, so `Block::covers` takes the domain's own bound verbatim wherever the block reaches an
  edge of the lattice. Without that an *unclipped* grid would be placed by a number a hair from
  the one that placed it before.
- **The filter.** Both backends read the grid bilinearly, so a device pixel just inside the
  target reads the cell either side of it, and a block cut exactly to the target would show its
  own edge colour there. `Patch::within`'s contract therefore asks for one cell of margin on
  every side. That is the one place the clip is conservative, and it costs two cells per axis.

Two tests, and both were checked by breaking the thing they guard:

- `a_clipped_grid_is_the_whole_grids_cells_bit_for_bit` walks five blocks — against each edge, in
  the middle, straddling a discontinuity, and one narrower than the margin — and compares
  `f32::to_bits` of every channel against the whole grid's corresponding cell. Making `row` use
  block-relative indices fails it.
- `a_window_of_a_magnified_page_draws_what_the_whole_page_has_there` renders the same list twice
  at the same magnification, once into the whole page — where nothing is clipped — and once into
  a window whose transform differs by an integer translation, and demands the window's 20 687
  pixels be the page's own, byte for byte.

**The second one is worth reading for how its constants were chosen**, because the first draft of
it passed with the margin deleted and therefore established nothing. At one cell per device pixel
the cell centres coincide with the pixel centres, the filter's second weight is zero, and no
margin is needed at all. Three things together are what make the filter reach past the block: a
magnification at which the budget halves the lattice (24× on a 100-unit page), an offset that
puts the block's first cell on a whole lattice cell (device 1200 is unit 0.5 is cell 600), and a
**discontinuity there** — a smooth function's neighbouring cells differ by a fifth of a level
over 1200 cells and every rounding absorbs it. With all three, deleting the margin moves 151
pixels: one whole column of the window.

## 4. What it bought

`examples/zoom_frame`, which gained `ZOOM_FRAME_WINDOW` for this — a page-sized target contains
the whole of every shading's domain, so nothing about clipping a grid to a target can show there.
Real Radeon 890M, headless, 900 × 1100 window, page's middle centred, minima of five rounds, the
two arms taken by removing the suspect (`Patch::for_target` returning `Patch::whole`) rather than
by two trees.

`function_based_shading.pdf` page 1 — nine type 1 shadings, load 10.74 and 6.77:

| zoom | total before | total after | scene before | scene after | bytes before | bytes after |
|---:|---:|---:|---:|---:|---:|---:|
| 100% | 3.1 | 3.5 | 0.9 | 1.1 | 144 532 | 144 532 |
| 200% | 9.1 | 7.0 | 3.8 | 2.5 | 531 112 | 393 752 |
| 400% | 14.0 | **2.4** | 11.1 | **0.5** | 1 849 632 | 40 640 |
| 800% | 47.5 | **6.9** | 44.0 | **0.3** | 7 398 432 | **828** |

`function_based_shading_cmyk.pdf` page 2 — a `DeviceCMYK` type 1 shading, which is a colour
space no device program can be built for (ADR 0376), so the grid is the only path it has. Load
4.95 and 9.24:

| zoom | total before | total after | scene before | scene after | bytes before | bytes after |
|---:|---:|---:|---:|---:|---:|---:|
| 100% | 54.3 | **31.8** | 50.7 | 28.8 | 8 640 032 | 3 738 548 |
| 200% | 249.2 | **41.3** | 220.9 | 34.3 | 34 560 032 | 3 840 224 |
| 400% | 263.5 | **16.7** | 237.7 | 9.5 | 34 560 032 | 931 640 |
| 800% | 249.4 | **3.9** | 237.7 | 1.9 | 34 560 032 | **218 992** |

A zoom step on that page: **249.4 ms → 3.9 ms**, and 34.5 MB of uploaded grid → 219 KB. It is
the page a person would have called broken, and it is a page nobody had drawn in a window.

At 1× on the first witness the numbers move the wrong way by 0.2 ms and the bytes are identical,
which is what "nothing is clipped, and the clip was computed anyway" looks like: one matrix
inversion and four point maps per shading, twenty of them, inside the run-to-run spread.

**`pi_seven_segment.pdf`, the type 4 witness ADR 0406 was written for, is unchanged and uploads
32 bytes at every rung** — the device evaluates its program per fragment and builds no grid at
all. That is the correct outcome and it is worth stating: the two decisions are levers on
different paths, and a page that takes ADR 0406's path is not on this one.

## 5. The population, stated as a denominator

`shading_grid_census` over 1 479 corpus files — pdf.js, the four `doc/corpora/` submodules and
`doc/corpora-own/` — finds **5 pages carrying 20 type 1 shadings, in 4 documents**, two of which
are this project's own fixtures. So the real corpus witnesses are two: `function_based_shading.pdf`
and `function_based_shading_cmyk.pdf`, both from pdf.js.

That is a small population and saying so is the point. ADR 0406 stated three pages in 1 251 as its
denominator rather than burying it, and this is the same family one type wider: **the corpus
barely exercises type 1 shadings at all.** What the change is worth is therefore not a corpus
average — it is what happens to the reader of one of those pages, and on `function_based_shading_cmyk.pdf`
page 2 that is a frame time of a quarter of a second becoming four milliseconds.

The lever generalises further than the population does, and the code says where: `Patch` is a
`pdf-render` type over `ColoursAtDeviceScale`, which is the vocabulary any deferred colour source
uses. Nothing about it is specific to a type 1 shading.

## 6. What this does not do

- **It does not clip to the *shape*.** `sampled_fill` and `sampled_shader` both know the path
  being filled, and §8.7.4.5.4's radial raster already uses exactly that — the shape's device
  bounds intersected with the target. A shading pattern filling a small path over a large domain
  still resolves the whole window's worth of it. That is the next rung and it is exact by the
  same argument; what it needs is the shape's bounds computed identically in both backends, which
  is a `pdf-render` function rather than two.
- **It does not make a magnified shading sharper, and it now could.** Past `MAX_FUNCTION_CELLS`
  the lattice halves and a magnified page is drawn from a grid coarser than its own pixels.
  Bounding the *block* instead of the lattice would spend the same budget on the cells a person
  can actually see — at 8× on a 400-unit page that is one cell per device pixel where today it is
  one per four — and §10.7.3 permits either. It is not taken here because it **changes pixels**,
  and this decision's entire claim is that it changes none. It wants its own argument, its own
  witness and its own comparison against the oracle.
- **It does not touch the axial, radial or mesh kinds**, which carry no grid or were already cut
  to the target, nor the device-evaluated program path, which has no grid to cut.
- **It does not change `MAX_FUNCTION_CELLS`, the cell centre, or the parallel division.** ADRs
  0339 and 0364 are untouched, and the same tests that pinned them still pin them.
