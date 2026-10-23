# 2286 — A knockout element's shape is the area its path covers

Status: accepted for integration. Session 1726, 2026-10-23, the pixels slot of batch one hundred and
eleven, under the project owner's switch to code improvements and bug fixes of 2026-10-14. Clause:
ISO 32000-2 §11.4.6 (the row stays `implemented`, its note and tests amended), with §11.3.7.2's
NOTE 1 and §10.7.4 (the row stays `departed`). Context: ADRs 0226, 0498, 1082, 1320, 2166 §3;
raster's ADRs 0016 and 0076. Code: `render-cpu/src/encode.rs`'s `encode`, at the knockout branch.
Test: `render-cpu/tests/knockout_element_coverage.rs`, red on the tree before.

## 1. The ranking, and which of its pages were already named

Against HEAD's six arms, ranked by each page's worst lane, the head of the list is the glyph
quantum's: `copy_paste_ligatures` 1.1395 of 255 on the `cpu` and `gpu` lanes at 1x, `issue16316`
1.0212, `endchar` 0.5882, `issue2884_reduced` 0.4788 — every one a page cropped to a line of type,
where a sixteenth of a pixel's phase is most of the page's ink. With the quantum off the same four
read 0.0566, 0.0650, 0.0020 and 0.0390, the compute lane's figures, so that head is ADR 0498's trade
and not a defect. **A pixels round therefore ranks at the quantum off**: the `cpu` and `gpu` arms at
1x over every page and at 4x over the sixty HEAD ranks worst, the shipped arms for the rest. That
ranking's head is `endchar`, `issue21068` and `issue16038` on the `gpu` lane at 4x (0.5775, 0.4903,
0.4895, against 0.003 to 0.032 on the other two lanes), which is raster's sampled lane on its ordered
four-by-four grid — an axis-aligned edge crosses its samples four at a time and reads one of five
levels: on `endchar`'s row 37 at 4x the area is 182 and the lane 199 — recorded as a non-conformance by raster's ADR 0076 and taken by the viewer at
ten times magnification; and `issue2177`, ADRs 1435 and 1443. The first page none of those names is
`knockout_groups_test.pdf`, 0.3602 on the `cpu` and `compute` lanes alike at 1x and 0.2079 at 4x.

**The brief's hypothesis did not hold**: the worst three pages share no construction of a soft mask or
a knockout group — two are the quantum and the sampled lattice — and the knockout page is fifth.

## 2. The smallest mark, and the clause

The worst pixel is (131, 126) at 1x, 50 levels: the oracle 53, 53, 107 and both raster lanes 84, 84,
168, inside the left panel's knockout group, where two 0.5-unit outlines (0.66 of a device pixel) meet
three translucent rectangles. Drawing that group with subsets of its 95 elements isolates it: the
background and the three rectangles agree on every backend (127, 127, 255); the red rectangle's
outline alone *inside the group* is 191 on the oracle and 222 on raster, and *outside any group* 222
on both. Its band lies 0.13 of a pixel deep in that row, so 222 is the area; 191 is a quarter.

§11.4.6 generalises knockout to a fractional shape:

> The concept of knockout is generalized to accommodate fractional shape values. In that case, the
> immediate backdrop shall be only partially knocked out and shall be replaced by only a fraction of
> the result of compositing the object with the initial backdrop.

and §11.3.7.2's NOTE 1 says an anti-aliased boundary's coverage is that shape. The clause's stage b)
evaluated in `f64` from each outline's own area (0.1294 and 0.2420) gives α 0.6700 and 84, 84, 168 —
raster's value. **The oracle was the lane in error.**

## 3. The cause

`compose::Compose::Knockout` is Porter-Duff `Source`, and `scan::intersected` declines `Source`
(`compose::carries_coverage_as_alpha`): its mask scales the source where `Source` must interpolate.
So a bare knockout element's coverage was the library's supersampled converter — the lattice of
sixteenths ADR 1082 took every other mark off. ADR 2166 §3 named the remedy and left it for a later
round: "a coverage route for `Source` needs its own composition". The composition already exists:
ADR 1320's two stages for a knockout element its clip cuts — Destination-Out with an opaque twin,
then Plus with the element — and both are Over-family modes, so both reach `crate::area`.

## 4. Decision

**A knockout element that is a `Fill` or a `Stroke` takes the two stages whether or not a clip cuts
it**, wherever `coverage_twin` can state its twin; an image, a nested group and an element with no
twin keep the bare draw. `(1 − f) × P + S` is the same line either way, so what moves is `f`.

- **Why not a `Source` route inside `scan::intersected`**, one measurement composed twice: it is a
  new construction in a file this batch's slot 4 owns, and the two stages are a proven one. Its
  price is a second rasterisation of each knockout path; `raster_golden` says ten first pages hold
  one.
- **The residual, by name**: a knockout image whose edges are not axis-aligned, and an element whose
  paint is translucent per pixel, keep the library's lattice.
- **A cost ADR 2166 §3 accepted is taken back for paths**: past the 8191-pixel range a knockout path
  is now measured by `crate::area` too; `window_on_the_page_grid.rs`'s two knockout tests still hold.

## 5. What moved

`raster_golden`: 10 of 974 moved, every one `raster only` — `22060_A1_01_Plans`, `alphatrans`,
`bug852992_reduced`, `issue13447`, `issue14438`, `issue17069`, `issue17215`, `issue1905`,
`issue20062`, `knockout_groups_test` — and re-taken. On the arms (§6) the `cpu` and `compute` lanes'
error falls on every moved page; the `gpu` lane's rises on some, because its sampled lattice and the
library's had agreed by construction where both count samples in rows of four. That rise is the
oracle leaving a lattice, not a lane regressing: raster's frames are byte for byte HEAD's
(`render-cpu` is only a development dependency of `render-raster`).

## 6. Measured

Against HEAD's six arms, my export's: the three 1x arms over all 968 pages, the three 4x arms over the
18 first pages `group_shape_census` finds holding a knockout group and the 1x movers (26 page-arms
compared, `issue1905` refused at 4x by the device's scratch on both trees). **No frame digest moved**
on any arm. The mean error moved on 9 pages at 1x and 5 at 4x, the oracle's frame having moved:

| page | `cpu` 1x | `compute` 1x | `gpu` 1x | `cpu` 4x | `compute` 4x | `gpu` 4x |
|---|---|---|---|---|---|---|
| `knockout_groups_test` | 0.3955 → 0.1829 | 0.3602 → 0.1295 | 0.3849 → 0.4500 | 0.2165 → 0.1387 | 0.2079 → 0.1264 | 0.2139 → 0.2003 |
| `alphatrans` | 0.0661 → 0.0497 | 0.0519 → 0.0355 | 0.0605 → 0.0776 | 0.0395 → 0.0357 | 0.0368 → 0.0330 | 0.0383 → 0.0420 |
| `issue1905` | 0.0617 → 0.0526 | 0.0568 → 0.0477 | 0.2618 → 0.2700 | refused | refused | refused |

and six more by thousandths, every `cpu` and `compute` figure down; the `gpu` lane rises on five
pages at 1x and four at 4x, by at most 0.0651. **That rise is the sampled lane's, not a regression**:
inside the group the red outline alone reads 191 on the old oracle *and* on the `gpu` lane — four
samples of sixteen, one row of the ordered grid — where its area is 222; the two lattices agreed, and
the oracle has left one of them. Looked at (trap 1): `knockout_groups_test`'s outlines inside the
knockout panels were drawn a sixteenth-lattice heavier than the same outlines in the normal panels
below and now match them; `alphatrans` is unchanged to the eye, 2 410 pixels moved by at most 25.
`t2-turn_path`: see the record; `render-cpu` is in `render-raster`'s graph as a development
dependency only, and no frame a turn draws crosses it.
