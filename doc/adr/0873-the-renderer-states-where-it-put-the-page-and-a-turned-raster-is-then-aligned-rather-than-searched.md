# 0873 — The renderer states where it put the page, and a turned raster is then aligned rather than searched

Session 915. Status: **accepted**. The second of this round's two records, and it is ADR 0831 §1's
own price paid: "[w]hat would make it an assertion is `render` reporting the sub-pixel offset it
placed the page at, so the walk can *derive* the whole-pixel shift instead of searching for one.
That is a change to the renderer's report, not to the walk."

## Context

RFC 0002 §9's third layer asks for "the rotation-transformed comparison for rotate": draw the
source page, turn the raster a quarter turn — which is what §7.7.3.3 says the page now is — and
hold the rotated page's raster to it. ADR 0831 §1 measured that comparison and found it failed on
905 of 905 rotated pages, for two reasons that are both the renderer's:

- **the grid turns with the page**, so a glyph edge covering a pixel 6 % on one grid covers it 8 %
  on the other; and
- **the leftover sliver changes edges**, which is worth up to a whole pixel.

The second is arithmetic rather than antialiasing. `TargetSpec::for_page` takes `ceil(W × s)` and
`ceil(H × s)` and anchors the page at the raster's **top-left** corner on both axes — ADR 0064's
choice, taken because translating by the raster's height instead put every mark a fraction of a row
low — so the strip of raster the page does not reach is at the **right** across and at the
**bottom** down. Turn the *page* a quarter turn and the vertical strip becomes a horizontal one at
the right of the new raster. Turn the *raster* and the same strip lands at the **left**. The two
therefore disagree by the source's own vertical overrun, which is less than one whole pixel and is
usually most of one.

ADR 0831 found that by hand, on three documents, by trying an offset of one column and seeing the
mean absolute difference fall to 0.000 on `issue2761.pdf`. A walk cannot try offsets: searching for
the offset that makes two rasters agree is fitting, and it would report agreement on a page that
had genuinely moved by a pixel.

## Decision

### 1. `render` reports the overrun, because only the renderer knows it

`pdf_transform::render::Overrun` is two numbers in `0.0..1.0` — `across` and `down` — being the
strip of raster the page does not reach on each axis. `Rendered` carries one, and `Origin::Page`
carries it into the report as `overrun_across` and `overrun_down`, so RFC 0002 §4.5's JSON states
it beside the `width` and `height` it already stated.

It is computed from the same three numbers `TargetSpec::for_page` builds the target out of — the
page's extent in user space, the scale, and the raster's integer size. **Not recomputed in the
walk**, which is the whole reason it is in the report: a second implementation of the rounding in a
test file can only ever disagree with the renderer's, and it is the renderer's placement the
comparison has to undo. That is the same argument §12.4.2's `merge::page_labels` is called by four
writers for.

The report's JSON gained a `Value::Number`, which it had not had: every field of it until now was a
count, an index, a page or a byte size. RFC 8259 §6 admits a number and states no range or
precision; what is written is Rust's shortest representation that reads back as the same `f64`, so
the report stays deterministic, and a value JSON has no spelling for is written `null` rather than
as a token no parser accepts.

**ISO 32000-2 requires none of this**, and the report says so: §10.7 leaves scan conversion to the
device and says nothing about a page whose size is not a whole number of pixels — `CLAUDE.md` names
this as one of the two places where the standard defines nothing and *done* means a documented
choice. So what the field states is the choice, not a requirement met.

### 2. The walk shifts by the nearest whole column, and the figures say the derivation is right

`pages_corpus.rs` takes the source page's `overrun.down`, rounds it to a whole number of columns —
0 or 1 by construction — and drops that many columns from the turned source's left and from the
rotated page's right. Over the corpus:

| | worst mean | worst tile error | least similar tile | byte-identical |
|---|---|---|---|---|
| as ADR 0831 measured it | 26.44 | 43.40 | −0.4325 | 0 |
| aligned from the report | **15.84** | 47.03 | **0.0021** | **17** |
| aligned the *other* way | 39.19 | 71.72 | −0.4272 | 0 |

905 rotated pages, 194 of which the overrun puts at 0 columns and 711 at 1.

The third row is the calibration trap 13 asks for, and it is what makes the first two mean
something: shifted in the direction the arithmetic does *not* predict, every figure is worse than
doing nothing and no page becomes exact. **Seventeen pages that differed become byte-identical**,
which is the outcome no wrong derivation produces.

**Read the mean and not the worst tile**, and trap 26 is why: the worst tile is taken on a fixed
grid, so cropping a column moves every tile boundary and the two worst tiles are not the same
region of the page. The worst tile error rising from 43.40 to 47.03 while the mean falls by ten
levels is that effect and not a regression. Both are printed, because a figure whose instrument
moved is worth having beside one whose did not.

### 3. It is still measured and not asserted, and the two remaining terms are now named

This is the honest half of the record. The alignment removes one of ADR 0831's two terms; it does
not produce an equality, and two things are left:

- **The grid**, which no integer shift undoes. `issue15150.pdf` is the standing witness — a 7 × 7
  raster whose one non-white pixel reads (255, 239, 239) before the rotation and (255, 234, 234)
  after, with an overrun of zero on both axes, so there is nothing to align away and five levels of
  255 remain.
- **The sub-pixel remainder**, which is new information. An overrun of a third of a pixel puts the
  two rasters on grids a third of a pixel apart and no integer shift is the identity there. It is
  at most half a pixel by construction and the walk prints the largest it saw: **0.5000**, which is
  the worst case actually occurring rather than a bound.

So `doc/todo/57`'s item narrows rather than closing: what it asked for is done and named, and what
would make the comparison an *assertion* is a tolerance stated against those two terms — which is a
statement about the renderer's antialiasing, not about the writer, and belongs to a round that can
derive one rather than pick one. The three assertions ADR 0831 put in place of it are unchanged and
all still exact: the round trip, the dimension swap, and bit identity for every page the plan did
not rotate.

## Consequences

- `pdf_transform::render::Overrun` is public, `Rendered` and `Origin::Page` carry one, and the
  report's JSON has a real number in it. `Report`, `Output` and `Origin` lose `Eq` and keep
  `PartialEq`; nothing in the workspace used the former.
- `pages_corpus.rs` prints three lines where it printed one: the unaligned figures, the aligned
  ones with the count that became exact, and the distribution of the derived shift beside the worst
  remainder.
- §7.7.3.3's ledger row carries the figures and the calibration.
- A consumer of the report can now compare two rasters of differently shaped pages without knowing
  this renderer's rounding rule, which is the general capability the rotated comparison is one case
  of.
