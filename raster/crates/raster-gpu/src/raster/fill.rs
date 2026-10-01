//! Filling: flattened polylines become coverage bytes over a region of device pixels.
//!
//! One thing: the accumulation grid, and the two rules read off it. This is
//! [`raster`](super)'s definition of coverage executed — flattened edges deposit exact
//! signed trapezoid areas, a left-to-right prefix sum recovers the average winding per
//! pixel, and ISO 32000-2 §8.5.3.3's two rules turn a winding into a coverage byte.
//!
//! **The region is a window on one answer, not an answer of its own** (ADR 0049): an
//! edge that leaves the region is cut at the border and deposits its winding *there*,
//! which is what lets one rasterisation serve every tile cut out of it, and what makes
//! [`CoverageMask::crop`] a lookup rather than a second rasterisation. The three
//! deposit functions below are that one rule at three scales — a row slab, the borders
//! it crosses, and one single-cell trapezoid.
//!
//! **The integral is the set's area only where a pixel sees at most two adjacent windings**
//! (ADR 1389). §8.5.3.3's rules decide, point by point, whether a point is inside, and
//! §10.7.4 scan-converts only after "all 'insideness' computations have been performed" —
//! so a pixel's coverage is the area of the inside set within it. Clamping the integrated
//! winding gives that area wherever the winding in the pixel takes two neighbouring
//! values, which is every pixel one boundary passes through; where two boundaries of the
//! path meet in one pixel, it may not (two same-wound squares' overlap reads their sum,
//! two opposed ones cancel). [`topology`] settles most fills at once — no subpath crossing
//! itself or another, and their nesting alternating in orientation, winds every pixel two
//! neighbouring values — [`overlap`] finds the pixels of the rest where more can happen,
//! and [`exact`] replaces each by the area of the set the rule declares inside.

use super::flatten::Polyline;

mod exact;
mod overlap;
mod rows;
mod topology;

pub(crate) use rows::RowIndex;

/// A rasterised coverage tile: `width × height` bytes anchored at integer device
/// pixel `(left, top)`.
#[derive(Debug, Clone)]
pub(crate) struct CoverageMask {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
    /// Row-major coverage bytes, `width × height`.
    pub coverage: Vec<u8>,
}

impl CoverageMask {
    /// A mask that admits nothing, over the given pixels.
    ///
    /// A legitimate mask rather than the absence of one: an empty clip region admits
    /// nothing *inside* it too, which is a different statement from having no clip, and
    /// both have tests (`raster/doc/PLAN.md` section 1.4).
    pub(crate) fn transparent(left: i32, top: i32, width: u32, height: u32) -> Self {
        Self {
            left,
            top,
            width,
            height,
            coverage: vec![0; (width as usize).saturating_mul(height as usize)],
        }
    }

    /// The window of this mask over another rectangle of device pixels, transparent
    /// wherever the two do not meet.
    ///
    /// **This is only a lookup because [`fill_mask`] cuts at its region's border** — the
    /// two share the pixel grid, and the same device pixel carries the same coverage
    /// whichever region computed it, to within the 1-of-255 rounding
    /// `a_tile_is_the_crop_of_the_region_that_contains_it` bounds. Outside this mask the
    /// answer is transparent by construction: a region is the intersection of its
    /// chain's bounds, and a closed path winds nothing beyond its own.
    // The corners are computed in `i64`, where an `i32` origin plus a `u32` extent cannot
    // wrap; every offset below is then a difference between two of those corners inside
    // the overlap, so it is non-negative and no larger than the smaller mask's extent.
    // Stated once here rather than at each of the six.
    #[expect(clippy::arithmetic_side_effects)]
    #[expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub(crate) fn crop(&self, left: i32, top: i32, width: u32, height: u32) -> Self {
        let mut cut = Self::transparent(left, top, width, height);
        let (ax0, ay0) = (i64::from(left), i64::from(top));
        let (ax1, ay1) = (ax0 + i64::from(width), ay0 + i64::from(height));
        let (bx0, by0) = (i64::from(self.left), i64::from(self.top));
        let (bx1, by1) = (bx0 + i64::from(self.width), by0 + i64::from(self.height));
        let (x0, y0) = (ax0.max(bx0), ay0.max(by0));
        let (x1, y1) = (ax1.min(bx1), ay1.min(by1));
        if x0 >= x1 || y0 >= y1 {
            return cut;
        }
        let (span, rows) = ((x1 - x0) as usize, (y1 - y0) as usize);
        let (from_x, from_y) = ((x0 - bx0) as usize, (y0 - by0) as usize);
        let (into_x, into_y) = ((x0 - ax0) as usize, (y0 - ay0) as usize);
        for row in 0..rows {
            let from = (from_y + row) * self.width as usize + from_x;
            let into = (into_y + row) * width as usize + into_x;
            cut.coverage[into..into + span].copy_from_slice(&self.coverage[from..from + span]);
        }
        cut
    }
}

/// Which of ISO 32000-2 §8.5.3.3's two rules decides insideness.
///
/// `Hash` because it is part of the glyph cache's key: the same outline under the two
/// rules is two different pictures wherever a subpath nests (ADR 0024).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Rule {
    NonZero,
    EvenOdd,
}

/// Rasterise closed polylines into a coverage mask over the given integer pixel
/// region (`left..left+width`, `top..top+height`), by the module's stated definition.
///
/// Every subpath is treated as closed (fill semantics, ISO 32000-2 §8.5.3.1: filling
/// implicitly closes open subpaths).
///
/// **The region is a window on one answer, not an answer of its own.** Geometry outside
/// it is cut at the border and deposits its winding there ([`deposit_slab`]), so asking
/// for a tighter region returns what a wider one holds over the same pixels — to within
/// the accumulator's own rounding, which ADR 0049 measures at **1 of 255 on 2 pixels in
/// 2.9 million**. That is what lets one rasterisation of a clip's region serve every
/// tile cut out of it (`encode::residue`).
pub(crate) fn fill_mask(
    polylines: &[Polyline],
    rule: Rule,
    left: i32,
    top: i32,
    width: u32,
    height: u32,
) -> CoverageMask {
    fill_mask_settled(polylines, rule, (left, top, width, height), false)
}

/// [`fill_mask`], told whether the fill is already known to wind every point two
/// neighbouring values — the answer a stored outline keeps for all its placements
/// ([`winds_two_values`]) — in which case the integral is the set's area everywhere and
/// the question is not asked again (ADR 1389).
pub(crate) fn fill_mask_settled(
    polylines: &[Polyline],
    rule: Rule,
    region: (i32, i32, u32, u32),
    two_values: bool,
) -> CoverageMask {
    fill_over(polylines, None, rule, region, two_values)
}

/// [`fill_mask`], reading only the edges and subpaths `index` lists for the region's rows:
/// the same bytes, from the few edges that can reach a small region of a large fill
/// (ADR 1479, [`RowIndex`]).
pub(crate) fn fill_mask_indexed(
    polylines: &[Polyline],
    index: &RowIndex,
    rule: Rule,
    left: i32,
    top: i32,
    width: u32,
    height: u32,
) -> CoverageMask {
    fill_over(
        polylines,
        Some(index),
        rule,
        (left, top, width, height),
        false,
    )
}

/// The one fill both entries share: every edge, or the ones `index` lists for the region,
/// visited in the same order.
// The accumulation arithmetic below is bounded by construction: coordinates are
// clamped into the region, whose dimensions were checked against the frame budget
// before allocation. Stated once here rather than per line of a hot loop.
#[expect(clippy::arithmetic_side_effects)]
fn fill_over(
    polylines: &[Polyline],
    index: Option<&RowIndex>,
    rule: Rule,
    (left, top, width, height): (i32, i32, u32, u32),
    two_values: bool,
) -> CoverageMask {
    let w = width as usize;
    let h = height as usize;
    // One spill column: a deposit at the right edge lands in it rather than wrapping.
    let mut acc = vec![0.0_f32; (w + 1) * h];

    #[expect(clippy::cast_precision_loss)] // region dims are bounded by target limits
    let (fw, fh) = (w as f32, h as f32);
    let mut deposit = |polyline: &Polyline, i: usize| {
        let (x0, y0, x1, y1) = local_edge(polyline, i, left, top);
        if let Some(edge) = Edge::cut(x0, y0, x1, y1, fh) {
            accumulate_edge(&mut acc, w, fw, &edge);
        }
    };
    let mut listed = Vec::new();
    if index.is_some_and(|index| index.edges_meeting(top, h, &mut listed)) {
        for (subpath, i) in listed {
            deposit(&polylines[subpath], i);
        }
    } else {
        for polyline in polylines {
            for i in 0..polyline.points.len() {
                deposit(polyline, i);
            }
        }
    }

    // Prefix-sum each row: the running total is the average winding per pixel; the
    // rule maps winding to coverage; `round` quantises (our stated rule, ADR 0005).
    let mut coverage = vec![0_u8; w * h];
    for y in 0..h {
        let mut running = 0.0_f32;
        for x in 0..w {
            running += acc[y * (w + 1) + x];
            // Kept: the pixel's average winding is what fixes the winding's constant
            // where a complex pixel is recomputed.
            acc[y * (w + 1) + x] = running;
            let cov = match rule {
                Rule::NonZero => running.abs().min(1.0),
                Rule::EvenOdd => {
                    let m = running.abs().rem_euclid(2.0);
                    1.0 - (m - 1.0).abs()
                }
            };
            #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                coverage[y * w + x] = (cov * 255.0).round() as u8;
            }
        }
    }
    // Where the fill can wind a pixel more than two neighbouring values, the integral
    // above is not the set's area there: those pixels are recomputed from the set itself.
    if !two_values && let Some(complex) = complex_pixels(polylines, index, (left, top), (w, h)) {
        exact::correct(
            &complex,
            polylines,
            rule,
            (left, top),
            (w, h),
            &acc,
            &mut coverage,
        );
    }
    CoverageMask {
        left,
        top,
        width,
        height,
        coverage,
    }
}

/// Whether `polylines` wind every point of the plane two neighbouring values — the
/// question [`fill_mask`] asks of a region, asked of the whole fill — or `None` where the
/// fill is too crowded to answer within the bound [`topology`] states.
pub(crate) fn winds_two_values(polylines: &[Polyline]) -> Option<bool> {
    let (x0, y0, x1, y1) = super::flatten::polyline_bounds(polylines)?;
    let region = [x0, y0, x1, y1];
    if topology::plainly_two_values(polylines, region) {
        return Some(true);
    }
    Some(topology::Topology::of(polylines, None, region, 0)?.two_values())
}

/// The pixels of the region at `origin` of `size` pixels whose winding may take more than
/// two neighbouring values, or `None` where no pixel's can — the plain fill, and the fill
/// whose subpaths nest in alternation ([`topology`]) — or where the question is past its
/// bound, and the integral stands (ADR 1389).
#[expect(clippy::cast_precision_loss)] // region corners are bounded by target limits
fn complex_pixels(
    polylines: &[Polyline],
    index: Option<&RowIndex>,
    origin: (i32, i32),
    size: (usize, usize),
) -> Option<overlap::Complex> {
    let (w, h) = size;
    let (x, y) = (origin.0 as f32, origin.1 as f32);
    let region = [x, y, x + w as f32, y + h as f32];
    let listed = index.and_then(|index| index.subpaths_meeting(origin.1, h));
    let plain = match &listed {
        Some(listed) => {
            topology::plainly_two_values_among(polylines, listed.iter().copied(), region)
        }
        None => topology::plainly_two_values(polylines, region),
    };
    if plain {
        return None;
    }
    let mut topology =
        topology::Topology::of(polylines, listed.as_deref(), region, w.saturating_mul(h))?;
    if topology.two_values() {
        return None;
    }
    Some(overlap::Marks::walk(polylines, origin, size).complex_pixels(&mut topology, h))
}

/// Edge `i` of `polyline` in the region's coordinates, the last one returning to the
/// start: filling closes every subpath (§8.5.3.1).
#[expect(clippy::arithmetic_side_effects)] // `i + 1` below `usize::MAX`, `% n` with n > i
#[expect(clippy::cast_precision_loss)] // region origins are bounded by target limits
fn local_edge(polyline: &Polyline, i: usize, left: i32, top: i32) -> (f32, f32, f32, f32) {
    let n = polyline.points.len();
    let (p0, p1) = (polyline.points[i], polyline.points[(i + 1) % n]);
    (
        p0.x - left as f32,
        p0.y - top as f32,
        p1.x - left as f32,
        p1.y - top as f32,
    )
}

/// One edge cut to the region's rows, top to bottom, with the winding it carries.
#[derive(Debug, Clone, Copy)]
struct Edge {
    top_x: f32,
    top_y: f32,
    bot_y: f32,
    /// `x` per unit of `y`, finite by construction.
    dxdy: f32,
    /// `+1` for an edge running down the rows, `−1` for one running up.
    dir: f32,
}

impl Edge {
    /// The edge from `(x0, y0)` to `(x1, y1)` cut to the rows `0..fh`, or `None` where it
    /// deposits nothing: a horizontal edge, one outside the rows, or one whose slope `f32`
    /// cannot state.
    fn cut(x0: f32, y0: f32, x1: f32, y1: f32, fh: f32) -> Option<Self> {
        // Exact comparison: a horizontal edge deposits nothing by definition, and a
        // nearly-horizontal one deposits its nearly-zero area correctly.
        #[expect(clippy::float_cmp)]
        if y0 == y1 {
            return None;
        }
        let (dir, top_x, top_y, bot_x, bot_y) = if y0 < y1 {
            (1.0_f32, x0, y0, x1, y1)
        } else {
            (-1.0, x1, y1, x0, y0)
        };
        // Clip vertically to the region; x interpolates along the clipped span.
        let (top_x, top_y) = if top_y < 0.0 {
            (
                top_x + (bot_x - top_x) * (0.0 - top_y) / (bot_y - top_y),
                0.0,
            )
        } else {
            (top_x, top_y)
        };
        let (bot_x, bot_y) = if bot_y > fh {
            (top_x + (bot_x - top_x) * (fh - top_y) / (bot_y - top_y), fh)
        } else {
            (bot_x, bot_y)
        };
        if bot_y <= top_y {
            return None;
        }
        let dxdy = (bot_x - top_x) / (bot_y - top_y);
        // **A slope this edge cannot state is a slab this edge cannot fill.** The numerator
        // is bounded by twice the largest device coordinate the scene contract admits
        // (`MAX_COORDINATE` on a point and on a transform coefficient, so `4e27`), and the
        // denominator is positive by the test above — so a non-finite ratio means the slab
        // is under `2.4e-11` of a pixel tall, and the exact area such an edge deposits is
        // under `2.4e-11` where one coverage step is `1/255`. Depositing nothing is the
        // right answer to eleven decimal places, and it is the only answer that keeps a NaN
        // out of the accumulator: a NaN survives the prefix sum, and `abs().min(1.0)`
        // returns **1.0** for it, so one such edge paints the rest of its row solid. The
        // same test also catches a NaN arriving from a coordinate that is not finite, which
        // `Device::render` refuses at the viewport before it can reach here.
        if !dxdy.is_finite() {
            return None;
        }
        Some(Self {
            top_x,
            top_y,
            bot_y,
            dxdy,
            dir,
        })
    }

    /// The edge's `x` at height `y`, by the one interpolation every user of it shares.
    fn x_at(&self, y: f32) -> f32 {
        self.top_x + (y - self.top_y) * self.dxdy
    }
}

/// Deposit one edge's signed trapezoid areas into the accumulation grid.
///
/// The edge is split at every horizontal pixel row and every vertical pixel column
/// it crosses, so each piece lies within one cell; a piece from `(xs, ys)` to
/// `(xe, ye)` inside cell `k` deposits `d·(1 − xm)` into `k` and `d·xm` into `k+1`,
/// where `d` is the signed slab height and `xm` the piece's mean x within the cell —
/// the exact trapezoid area to the right of the edge, plus the spill that keeps the
/// running sum equal to the full winding beyond the crossing.
#[expect(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)]
#[expect(clippy::cast_sign_loss)]
fn accumulate_edge(acc: &mut [f32], w: usize, fw: f32, edge: &Edge) {
    let mut y = edge.top_y.floor().max(0.0);
    while y < edge.bot_y {
        let row = y as usize;
        if row >= acc.len() / (w + 1) {
            break;
        }
        let entry_y = edge.top_y.max(y);
        let exit_y = edge.bot_y.min(y + 1.0);
        let (entry_x, exit_x) = (edge.x_at(entry_y), edge.x_at(exit_y));
        deposit_slab(
            &mut acc[row * (w + 1)..(row + 1) * (w + 1)],
            fw,
            edge.dir,
            entry_x,
            entry_y,
            exit_x,
            exit_y,
        );
        y += 1.0;
    }
}

/// Deposit one row slab's areas. `xs`/`xe` are x at the slab's top and bottom; the part
/// of the slab spent left or right of the region is **cut off at the border** and
/// deposited there, rather than compressed into the columns inside it.
///
/// # Why the cut, and what clamping the endpoints instead used to cost (ADR 0049)
///
/// A slab piece running from `x = −25` to `x = +2` covers the region's first column for
/// most of its height and only reaches `x = 2` at the very end. Clamping the two
/// endpoints to `[0, fw]` and interpolating between them — which is what this function
/// did until ADR 0049 — spreads that height evenly from column 0 to column 2 instead.
/// The row's *total* winding survives (every column past the crossing reads the same
/// value, which is why nothing downstream ever saw it), but the columns at the border
/// get somebody else's share: **up to 185 of 255 on a shallow edge**, measured by
/// `a_tile_whose_geometry_enters_from_outside_is_exact`.
///
/// Cutting at the border is the same statement §10.7.4 makes about a clipping region —
/// the pixels a fill would cover — applied to the region this mask is asked for: what
/// lies outside contributes its winding, at the border, for exactly the height it spends
/// there.
///
/// The cut runs only when an endpoint is outside; a piece wholly inside takes the same
/// arithmetic it always did, to the bit, which is what keeps every tile that is not cut
/// by a clip or by the page edge pixel-for-pixel where it was.
#[expect(clippy::arithmetic_side_effects)]
fn deposit_slab(row: &mut [f32], fw: f32, dir: f32, xs: f32, ys: f32, xe: f32, ye: f32) {
    if xs >= 0.0 && xs <= fw && xe >= 0.0 && xe <= fw {
        deposit_inside(row, fw, dir, xs, ys, xe, ye);
        return;
    }
    // A piece wholly left or wholly right of the region crosses neither border — each
    // `t` below rounds to at least `1` or below `0`, rounding being monotone — and
    // `deposit_inside` then clamps both ends onto one border column: its whole height into
    // the column at that border and a zero beside it. That deposit, made directly, without
    // the two divisions and the walk that arrive at it (ADR 1479): a small tile of a large
    // clip meets most of the clip's edges in its rows on one side or the other.
    if xs < 0.0 && xe < 0.0 {
        deposit_at_border(row, 0, dir * (ye - ys), 0.0);
        return;
    }
    if xs > fw && xe > fw {
        let last = row.len().saturating_sub(2);
        #[expect(clippy::cast_precision_loss)] // a column index of a bounded region
        deposit_at_border(row, last, dir * (ye - ys), fw - last as f32);
        return;
    }
    let (dx, dy) = (xe - xs, ye - ys);
    // At most two borders can be crossed, and `dx == 0` crosses neither: a vertical
    // piece is on one side for its whole height.
    let mut cuts = [(0.0_f32, 0.0_f32); 2];
    let mut count = 0;
    if dx != 0.0 {
        for border in [0.0_f32, fw] {
            let t = (border - xs) / dx;
            if t > 0.0 && t < 1.0 {
                cuts[count] = (t, border);
                count += 1;
            }
        }
        if count == 2 && cuts[1].0 < cuts[0].0 {
            cuts.swap(0, 1);
        }
    }
    // Each part is interpolated from the piece's own ends, so a cut cannot move where
    // the piece starts or finishes: the border's own x is used at the seam, and the
    // outer ends stay the values the caller passed.
    let (mut px, mut py) = (xs, ys);
    for (t, border) in cuts.iter().take(count).copied() {
        let (nx, ny) = (border, ys + dy * t);
        deposit_inside(row, fw, dir, px, py, nx, ny);
        (px, py) = (nx, ny);
    }
    deposit_inside(row, fw, dir, px, py, xe, ye);
}

/// What [`deposit_inside`] deposits for a piece both of whose ends it clamps onto one border
/// column: `d · (1 − frac)` into `cell` and `d · frac` beside it, the same two products.
#[expect(clippy::arithmetic_side_effects)] // `cell + 1` is the spill column at most
fn deposit_at_border(row: &mut [f32], cell: usize, d: f32, frac: f32) {
    if d != 0.0 {
        row[cell] += d * (1.0 - frac);
        row[cell + 1] += d * frac;
    }
}

/// One slab piece that does not cross the region's borders: split at each vertical cell
/// boundary it does cross, and deposit the exact trapezoid areas.
///
/// A piece wholly outside arrives here with both ends on the same side; the clamp then
/// collapses it onto the border column, which is where its winding belongs.
#[expect(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)]
#[expect(clippy::cast_sign_loss, clippy::cast_precision_loss)]
fn deposit_inside(row: &mut [f32], fw: f32, dir: f32, xs: f32, ys: f32, xe: f32, ye: f32) {
    let xs = xs.clamp(0.0, fw);
    let xe = xe.clamp(0.0, fw);
    let (mut px, mut py) = (xs, ys);
    loop {
        // The next vertical boundary in the direction of travel, or the slab's end.
        let boundary = if xe > px {
            let b = px.floor() + 1.0;
            if b < xe { Some(b) } else { None }
        } else if xe < px {
            let b = px.ceil() - 1.0;
            if b > xe { Some(b) } else { None }
        } else {
            None
        };
        let (nx, ny) = match boundary {
            Some(b) => {
                let t = (b - xs) / (xe - xs);
                (b, ys + (ye - ys) * t)
            }
            None => (xe, ye),
        };
        // One single-cell piece: exact trapezoid deposit.
        let d = dir * (ny - py);
        if d != 0.0 {
            let xm = 0.5 * (px + nx);
            let cell = (xm.floor().max(0.0) as usize).min(row.len().saturating_sub(2));
            let frac = xm - cell as f32;
            row[cell] += d * (1.0 - frac);
            row[cell + 1] += d * frac;
        }
        if boundary.is_none() {
            break;
        }
        (px, py) = (nx, ny);
    }
}
