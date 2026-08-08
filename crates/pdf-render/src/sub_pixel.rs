//! What a shape thinner than one device pixel is handed to a rasteriser whose coverage is
//! quantised: ISO 32000-2 §10.7.4, one step past [`crate::collapsed`].
//!
//! # The sentence, and the second way of failing it
//!
//! §10.7.4:
//!
//! > A shape shall be scan-converted by painting any pixel whose half-open square region
//! > intersects the shape, no matter how small the intersection is. This ensures that no shape
//! > ever disappears as a result of unfavourable placement relative to the device pixel grid,
//! > as might happen with other possible scan conversion rules.
//!
//! [`crate::collapsed`] answers that for a subpath with **no** area, which no anti-aliasing
//! rasteriser can give coverage to at any resolution. This module answers it for a subpath that
//! *has* an area and loses it anyway, because the coverage the rasteriser can express does not go
//! that low. The two are different facts — the first is about the geometry, the second about the
//! device — and a shape can fall through the first and be lost by the second.
//!
//! # Why anything is owed here at all, when this tree anti-aliases
//!
//! Anti-aliasing is a departure from the clause read literally, licensed by §10.7.1's NOTE that
//! "[t]he specifics of the scan conversion algorithm are not defined as part of PDF", and it
//! replaces the clause's "paint the pixel" with **coverage proportional to area**. That
//! replacement is what has to hold: a shape occupying a twentieth of a pixel is drawn at a
//! twentieth of the ink, which is neither the clause's answer nor nothing. Coverage that rounds
//! to nothing is not the anti-aliasing departure — it is the disappearance the clause's stated
//! purpose forbids, arrived at by a different route.
//!
//! # What one rasteriser can express
//!
//! `tiny-skia`'s anti-aliased scan converter supersamples four times per pixel row and takes each
//! sub-row's sample at its centre — device y = 0.125, 0.375, 0.625, 0.875 — so a shape lying
//! between two of those lines crosses none of them and contributes nothing at all, and one that
//! crosses a single line is rounded **up** to a quarter of a row. Along x the same converter
//! quantises a run to quarter-pixel steps. So its coverage is a multiple of a sixteenth of a
//! pixel and its smallest non-zero answer is 1/16, measured by
//! `render-quorra/examples/sub_pixel_marks`: an 80-unit rule 0.05 and 0.1 units thick draws
//! **nothing**, 0.2 units draws 0.2471 where its area is 0.2. The graphics device has no such
//! quantum and answers 0.0510, 0.1020 and 0.2000 for the same three.
//!
//! # The substitution
//!
//! No geometry can ask that rasteriser for a twentieth of a pixel, because its smallest
//! expressible coverage is larger than that. What *can* is alpha: a shape whose coverage is `c`
//! and whose paint is opaque puts down exactly what a fully covered shape of alpha `c` does, for
//! every blend function the PDF family states, because §11.3.6's formula is linear in the source
//! alpha. So a rectangle thinner than a device pixel is replaced by the **whole device pixel line
//! it lies in**, painted at the coverage its own area in that line implies — and where it
//! straddles a boundary, by one such substitute per line, each carrying that line's share.
//!
//! For an axis-aligned rectangle that is not an approximation of proportional coverage but
//! exactly it, in both axes at once: the substitute's ink in pixel (i, j) is the rectangle's area
//! inside pixel (i, j). Nothing is snapped and nothing is promoted — a 0.05-thick rule draws
//! 0.05 of a row, where §10.7.5's stroke adjustment would have drawn a whole one and where this
//! rasteriser drew nothing.
//!
//! `tiny-skia` reaches for the same instrument one case over and it is worth naming, because it
//! is what makes this a use of the library rather than a fight with it: a stroke under a pixel
//! wide is drawn by its painter as a hairline with the paint's opacity scaled by the width. What
//! that construction gets wrong is *where* the ink goes — a hairline is smeared symmetrically
//! about the path, so the placement is approximate and the half that falls off the raster's edge
//! is simply lost. The substitution below is the same trade made exactly.
//!
//! # What is not substituted, and why each is left alone
//!
//! - **Anything but an axis-aligned rectangle.** The substitution's exactness rests on the
//!   shape's cross-section being constant along its length; a thin triangle's is not, and
//!   stretching it into a pixel line would state one coverage along a run where a ramp is right.
//!   A glyph stem is the case that makes this a rule rather than a caution: contours a fraction
//!   of a pixel wide are what small text is made of, and a uniform coverage across one is worse
//!   than what the rasteriser already does.
//! - **A transform that is not axis-preserving.** Under a rotation or a shear the run of pixels a
//!   thin band passes through is a staircase, and there is no pixel line to stretch it into.
//!   [`crate::collapsed`] declines the same case for the same reason.
//! - **A path with one subpath left over.** A fill's subpaths share a winding rule, so removing
//!   one from the path changes what the others enclose — an even-odd hole would become a filled
//!   region. The substitution therefore takes every subpath of a path or none of it.
//! - **Two subpaths meeting in one pixel line.** Separate draws composite as
//!   `1 − (1 − a)(1 − b)` where coverage inside one scan conversion *adds*, so two parallel
//!   sub-pixel rules sharing a pixel row would come out light. Where that happens the rasteriser's
//!   own scan conversion accumulates them and nothing is disappearing, so the substitution
//!   declines. Two *perpendicular* bands need no such guard: at their crossing the exact union of
//!   a row and a column of coverage `a` and `b` is `a + b − ab`, which is what compositing them
//!   gives.
//! - **The even-odd rule where the thin axes differ.** Under §8.5.3.3.3's rule two crossing
//!   rectangles leave a hole at their crossing, which separate draws cannot express. Where every
//!   substituted subpath is thin along the same axis they are disjoint — the guard above sees to
//!   that — and the two fill rules agree.
//!
//! The one this module is deliberately silent about is the **stroke** whose outline is not a
//! rectangle: a round cap ends a rule with an arc, and such a path is declined here and drawn by
//! whatever the backend would otherwise have done. ADR 0226.

use crate::collapsed::{Extent, subpath_extents};
use crate::geom::{Path, PathCommand, Point, Transform};
use crate::paint::FillRule;

/// One device pixel line's worth of a shape too thin for the rasteriser to measure.
///
/// Produced by [`sub_pixel_bands`]. The shape is stated in the path's own space, so a backend
/// draws it with exactly the transform the command carries; the coverage is what the paint's
/// alpha is to be multiplied by, and is the fraction of the pixel line the original shape's area
/// occupies.
#[derive(Debug, Clone, PartialEq)]
pub struct SubPixelBand {
    /// The whole device pixel line the shape lies in, expressed in the path's own space.
    pub shape: Path,
    /// The coverage the shape's own area implies there, in `0.0 < coverage <= 1.0`.
    pub coverage: f32,
}

/// The substitutes for a fill whose every subpath is a rectangle thinner than a device pixel,
/// ISO 32000-2 §10.7.4.
///
/// `to_device` maps the path's own space onto the device pixel grid, and it is the whole of what
/// decides both halves of the question: which shapes are thin, and which pixel lines they lie in.
///
/// Returns `None` — the answer for essentially every path, and the one that must cost nothing —
/// wherever the substitution does not apply in full. The module comment lists the cases and
/// argues each; the shape of the answer is deliberate, since a partial substitution would leave a
/// path drawn as two overlapping halves.
///
/// The first test is [`Path::narrowest_rectangle`], which is memoised: this is asked once per fill
/// command per strip the rasteriser cuts the page into, and walking a path's subpaths every time
/// is what [`Path::collapses`] exists not to do.
#[must_use]
pub fn sub_pixel_bands(
    path: &Path,
    to_device: Transform,
    fill_rule: FillRule,
) -> Option<Vec<SubPixelBand>> {
    // §10.7.4's pixel lines are the device's, so they can only be reached where a device axis is
    // a path axis; and the substitute has to be stated back in the path's own space.
    if !to_device.preserves_axes() {
        return None;
    }
    // A rectangle's device extent along an axis is its path extent times that axis's scale, so
    // no rectangle in this path is thinner than this — one multiplication in front of the walk.
    let narrowest = path.narrowest_rectangle()? * min_axis_scale(to_device);
    if !narrowest.is_finite() || narrowest >= 1.0 {
        return None;
    }
    let inverse = to_device.invert()?;

    let commands = path.commands();
    let mut plans: Vec<Plan> = Vec::new();
    for extent in subpath_extents(path) {
        let range = extent.range();
        let rectangle = is_axis_aligned_rectangle(&commands[range], extent.min, extent.max);
        // One subpath the substitution cannot take is the whole path's answer: see the module
        // comment on a path's shared winding rule.
        let plan = rectangle.then(|| substitute(&extent, to_device))??;
        plans.push(plan);
    }
    if plans.is_empty() {
        return None;
    }
    if fill_rule == FillRule::EvenOdd && !plans.iter().all(|plan| plan.thin == plans[0].thin) {
        return None;
    }
    if contested(&plans) {
        return None;
    }
    let mut bands = Vec::with_capacity(plans.len());
    for plan in &plans {
        for band in &plan.bands {
            bands.push(SubPixelBand {
                shape: stated_in_path_space(*band, inverse)?,
                coverage: band.coverage,
            });
        }
    }
    Some(bands)
}

/// Whether every subpath of a path has no extent along one axis, so that stroking it produces
/// axis-aligned rectangles.
///
/// The cheap question a backend asks before converting a sub-pixel stroke into the fill of its own
/// outline: only a straight rule's outline is a shape [`sub_pixel_bands`] can measure, and
/// building the outline of every thin stroke on a page of line work to find that out would be the
/// cost the rasteriser's own hairline path exists to avoid.
///
/// False for a path with no subpath at all, which has no outline either.
#[must_use]
pub fn only_flat_subpaths(path: &Path) -> bool {
    let mut any = false;
    for extent in subpath_extents(path) {
        any = true;
        if !flat(extent.min, extent.max) {
            return false;
        }
    }
    any
}

/// The narrowest side of any axis-aligned rectangle among a path's subpaths, in the path's own
/// space.
///
/// The predicate behind [`Path::narrowest_rectangle`], which memoises it, and the reason that memo
/// can exist: an affine map scales an extent, so which rectangle is narrowest is a property of the
/// commands and the *thinness* is one comparison against the transform's scale.
pub(crate) fn narrowest_rectangle(path: &Path) -> Option<f32> {
    let commands = path.commands();
    let mut narrowest: Option<f32> = None;
    for extent in subpath_extents(path) {
        if is_axis_aligned_rectangle(&commands[extent.range()], extent.min, extent.max) {
            let side = (extent.max.x - extent.min.x).min(extent.max.y - extent.min.y);
            narrowest = Some(narrowest.map_or(side, |seen: f32| seen.min(side)));
        }
    }
    narrowest
}

/// One subpath's substitution, before its bands are stated back in the path's own space.
#[derive(Debug)]
struct Plan {
    /// The bands, at most one per device pixel line the subpath meets along each thin axis.
    bands: Vec<DeviceBand>,
    /// Which axes the subpath is thinner than a pixel along.
    thin: Thin,
    /// The pixel lines it claims, deduplicated, for the collision test.
    lines: Vec<Line>,
}

/// Which of a subpath's axes is thinner than one device pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Thin {
    /// The subpath is under a pixel wide.
    x: bool,
    /// The subpath is under a pixel tall.
    y: bool,
}

/// A device pixel row or column a substitution occupies.
///
/// The line's coordinate is carried as the bits of the `f32` that names it, which is exact
/// because it is a whole number produced by `floor`, and which costs no cast. Negative zero is
/// normalised away by the addition, so the pixel at 0 has one spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Line {
    /// `false` for a column of the device, `true` for a row.
    row: bool,
    /// The line's own coordinate.
    at: u32,
}

impl Line {
    fn new(row: bool, at: f32) -> Self {
        Self {
            row,
            at: (at + 0.0).to_bits(),
        }
    }
}

/// One substitute rectangle, in device space.
#[derive(Debug, Clone, Copy)]
struct DeviceBand {
    /// The corner with the smallest coordinates.
    min: Point,
    /// The corner with the largest.
    max: Point,
    /// What the paint's alpha is multiplied by there.
    coverage: f32,
}

/// One axis of a substitute: the span it covers and the share of the shape's area in it.
#[derive(Debug, Clone, Copy)]
struct Span {
    lo: f32,
    hi: f32,
    /// The fraction of the original shape's extent along this axis that falls in the span.
    share: f32,
    /// The pixel line this span is, where the axis is thin.
    line: Option<f32>,
}

/// The smaller of the two factors an axis-preserving transform scales an axis by.
///
/// For such a transform one diagonal of the linear part is zero, so each axis is stretched by
/// exactly one of the four entries and the two live entries are the two scales.
fn min_axis_scale(to_device: Transform) -> f32 {
    if to_device.b == 0.0 && to_device.c == 0.0 {
        to_device.a.abs().min(to_device.d.abs())
    } else {
        to_device.b.abs().min(to_device.c.abs())
    }
}

/// Whether a subpath's bounding box has no extent along one axis.
#[expect(
    clippy::float_cmp,
    reason = "exactness is the question: a rule whose ends differ by a rounding step is not a \
              rule whose stroked outline is a rectangle, and the caller is about to rely on it \
              being one"
)]
fn flat(min: Point, max: Point) -> bool {
    min.x == max.x || min.y == max.y
}

/// Whether a subpath is the rectangle Table 58's `re` writes, with an area.
///
/// Four distinct corners of the subpath's own bounding box, joined by segments each of which
/// moves along one axis, is a traversal of that box's perimeter and nothing else — a bowtie is
/// excluded by the second condition and a shape that doubles back by the first.
pub(crate) fn is_axis_aligned_rectangle(commands: &[PathCommand], min: Point, max: Point) -> bool {
    if !(min.x < max.x && min.y < max.y) {
        return false;
    }
    let mut points = [Point::new(0.0, 0.0); 4];
    let mut count = 0usize;
    let last = commands.len().saturating_sub(1);
    for (index, command) in commands.iter().enumerate() {
        match *command {
            PathCommand::MoveTo(p) => {
                if index != 0 {
                    return false;
                }
                points[0] = p;
                count = 1;
            }
            PathCommand::LineTo(p) => {
                if count == 0 {
                    return false;
                }
                if count == 4 {
                    // Beyond the four corners the only segment left is the one back to the
                    // first, which is how `tiny-skia`'s stroker closes an outline and how a
                    // producer may write `re` out by hand.
                    if p != points[0] {
                        return false;
                    }
                } else {
                    points[count] = p;
                    count = count.saturating_add(1);
                }
            }
            // A curve is not a rectangle's side even where its control points make it straight:
            // the substitution's exactness is claimed for a shape, not for a parameterisation.
            PathCommand::CurveTo(..) => return false,
            // Table 58's `h` terminates the subpath, so nothing may follow it.
            PathCommand::Close => {
                if index != last {
                    return false;
                }
            }
        }
    }
    rectangular(&points, count)
}

/// Whether four collected points are the four corners of their own bounding box, in perimeter
/// order. See [`is_axis_aligned_rectangle`], whose second half this is.
#[expect(
    clippy::float_cmp,
    reason = "see `is_axis_aligned_rectangle`: every comparison here is between coordinates \
              copied from these same points"
)]
fn rectangular(points: &[Point; 4], count: usize) -> bool {
    if count != 4 {
        return false;
    }
    for (index, (point, next)) in points.iter().zip(points.iter().cycle().skip(1)).enumerate() {
        // Distinct: with four points each equal to none of the others and each a corner of the
        // box they span, the four corners are all present.
        if points[..index].contains(point) {
            return false;
        }
        // Each side moves along exactly one axis.
        if (point.x == next.x) == (point.y == next.y) {
            return false;
        }
    }
    true
}

/// One subpath's substitution, or `None` where it is not thin enough to need one.
fn substitute(extent: &Extent, to_device: Transform) -> Option<Plan> {
    let a = to_device.apply(extent.min);
    let b = to_device.apply(extent.max);
    let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
    let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
    if !(x0.is_finite() && x1.is_finite() && y0.is_finite() && y1.is_finite()) {
        return None;
    }
    let thin = Thin {
        x: x1 - x0 < 1.0,
        y: y1 - y0 < 1.0,
    };
    if !(thin.x || thin.y) {
        return None;
    }
    let xs = spans(x0, x1, thin.x);
    let ys = spans(y0, y1, thin.y);
    let lines = xs
        .iter()
        .filter_map(|x| x.line.map(|at| Line::new(false, at)))
        .chain(
            ys.iter()
                .filter_map(|y| y.line.map(|at| Line::new(true, at))),
        )
        .collect();
    let mut bands = Vec::with_capacity(xs.len().saturating_mul(ys.len()));
    for x in &xs {
        for y in &ys {
            let coverage = x.share * y.share;
            if coverage > 0.0 {
                bands.push(DeviceBand {
                    min: Point::new(x.lo, y.lo),
                    max: Point::new(x.hi, y.hi),
                    coverage: coverage.min(1.0),
                });
            }
        }
    }
    // A thin axis whose spans all had zero share leaves nothing to draw, which is not a shape
    // that disappeared: it is a rectangle the transform placed outside every pixel it names.
    (!bands.is_empty()).then_some(Plan { bands, thin, lines })
}

/// How a shape's extent along one axis is divided between the device pixel lines it meets.
///
/// Along an axis it is not thin along, the shape keeps its own extent and its whole share: this
/// rule is about the axis where the coverage is lost, and stretching the other one would move a
/// departure into an axis where the clause is being followed — [`crate::collapsed`]'s own reason
/// for snapping one axis only.
///
/// Along a thin axis the shape spans at most two lines, since its extent is under one pixel, and
/// each line's share is the length of the shape lying in it. §10.7.4 identifies the line by
/// flooring: "let i = floor( x ) and j = floor( y ). The pixel that contains this point is the one
/// identified as ( i, j )".
fn spans(lo: f32, hi: f32, thin: bool) -> Vec<Span> {
    if !thin {
        return vec![Span {
            lo,
            hi,
            share: 1.0,
            line: None,
        }];
    }
    let first = lo.floor();
    let mut out = Vec::with_capacity(2);
    let mut push = |at: f32| {
        out.push(Span {
            lo: at,
            hi: at + 1.0,
            share: (hi.min(at + 1.0) - lo.max(at)).max(0.0),
            line: Some(at),
        });
    };
    push(first);
    if hi > first + 1.0 {
        push(first + 1.0);
    }
    out
}

/// Whether two of a path's subpaths claim one device pixel line.
///
/// See the module comment: parallel sub-pixel rules meeting in one row would composite where
/// coverage inside one scan conversion adds, and what the rasteriser already does with them is
/// not a disappearance.
fn contested(plans: &[Plan]) -> bool {
    let mut lines: Vec<Line> = plans
        .iter()
        .flat_map(|plan| plan.lines.iter().copied())
        .collect();
    lines.sort_unstable();
    lines.windows(2).any(|pair| pair[0] == pair[1])
}

/// A device-space band stated back in the path's own space, as the rectangle a fill draws.
///
/// An axis-preserving transform carries a rectangle to a rectangle, so two opposite corners of
/// the band are two opposite corners of the answer — whether the axes were kept or exchanged by a
/// quarter turn, which is why the sides are sorted rather than assumed.
fn stated_in_path_space(band: DeviceBand, inverse: Transform) -> Option<Path> {
    let (p, q) = (inverse.apply(band.min), inverse.apply(band.max));
    if !(p.x.is_finite() && p.y.is_finite() && q.x.is_finite() && q.y.is_finite()) {
        return None;
    }
    let min = Point::new(p.x.min(q.x), p.y.min(q.y));
    let max = Point::new(p.x.max(q.x), p.y.max(q.y));
    let mut shape = Path::new();
    shape.push(PathCommand::MoveTo(min));
    shape.push(PathCommand::LineTo(Point::new(max.x, min.y)));
    shape.push(PathCommand::LineTo(max));
    shape.push(PathCommand::LineTo(Point::new(min.x, max.y)));
    shape.push(PathCommand::Close);
    Some(shape)
}

#[cfg(test)]
mod tests {
    use super::{only_flat_subpaths, sub_pixel_bands};
    use crate::geom::{Path, PathCommand, Point, Transform};
    use crate::paint::FillRule;

    fn path(commands: &[PathCommand]) -> Path {
        let mut p = Path::new();
        for c in commands {
            p.push(*c);
        }
        p
    }

    /// `x y w h re`, the shape a producer rules a line with.
    fn rectangle(x0: f32, y0: f32, x1: f32, y1: f32) -> Path {
        path(&[
            PathCommand::MoveTo(Point::new(x0, y0)),
            PathCommand::LineTo(Point::new(x1, y0)),
            PathCommand::LineTo(Point::new(x1, y1)),
            PathCommand::LineTo(Point::new(x0, y1)),
            PathCommand::Close,
        ])
    }

    /// Each band as its rectangle's two corners and its coverage, the last rounded to four
    /// decimals: a substitute's *placement* is exact arithmetic on whole numbers, while its
    /// coverage is a difference of device coordinates and carries one rounding step.
    fn bands(path: &Path, to_device: Transform) -> Vec<(f32, f32, f32, f32, f32)> {
        sub_pixel_bands(path, to_device, FillRule::NonZero)
            .unwrap_or_default()
            .iter()
            .map(|band| {
                let bounds = band.shape.hull().expect("a rectangle names four points");
                (
                    bounds.min.x,
                    bounds.min.y,
                    bounds.max.x,
                    bounds.max.y,
                    (band.coverage * 10_000.0).round() / 10_000.0,
                )
            })
            .collect()
    }

    /// The rule itself: a rule a twentieth of a pixel thick becomes the whole pixel row it lies
    /// in, at a twentieth of the ink — which is the area it covers there.
    #[test]
    fn a_rule_thinner_than_a_pixel_becomes_its_own_row_at_its_own_coverage() {
        assert_eq!(
            bands(&rectangle(10.0, 160.0, 90.0, 160.05), Transform::IDENTITY),
            [(10.0, 160.0, 90.0, 161.0, 0.05)]
        );
    }

    /// A rule straddling a pixel boundary is two substitutes, and the two shares are the two
    /// areas — which is the placement the graphics device gives it and the one this rasteriser
    /// cannot.
    #[test]
    fn a_rule_across_a_boundary_divides_its_coverage_between_two_rows() {
        let drawn = bands(&rectangle(10.0, 159.96, 90.0, 160.04), Transform::IDENTITY);
        assert_eq!(drawn.len(), 2, "one substitute per row: {drawn:?}");
        assert_eq!((drawn[0].1, drawn[0].3), (159.0, 160.0));
        assert_eq!((drawn[1].1, drawn[1].3), (160.0, 161.0));
        let total = drawn[0].4 + drawn[1].4;
        assert!(
            (total - 0.08).abs() < 1e-4,
            "the two shares are the shape's own thickness: {total}"
        );
    }

    /// The pixel is the *device's*, so doubling the scale halves what counts as thin and moves
    /// the row — which a test at one scale could not tell from a constant (trap 2).
    #[test]
    fn the_row_is_found_in_device_space_and_stated_back_in_the_paths() {
        assert_eq!(
            bands(
                &rectangle(10.0, 80.0, 90.0, 80.2),
                Transform::scale(2.0, 2.0)
            ),
            [(10.0, 80.0, 90.0, 80.5, 0.4)],
            "device row 160 is path units 80 to 80.5, and 0.2 units is 0.4 of a device pixel"
        );
    }

    /// At a scale where the same rule is a whole pixel thick there is nothing to substitute: the
    /// rasteriser measures it, and this rule must not touch a shape that is not disappearing.
    #[test]
    fn a_rule_a_whole_pixel_thick_is_left_to_the_rasteriser() {
        assert_eq!(
            sub_pixel_bands(
                &rectangle(10.0, 80.0, 90.0, 80.2),
                Transform::scale(5.0, 5.0),
                FillRule::NonZero
            ),
            None
        );
    }

    /// A shape thin along *both* axes is the same rule twice, and its coverage is the product —
    /// a quarter-pixel square is a sixteenth of a pixel's ink, which is exactly its area.
    #[test]
    fn a_speck_thin_in_both_axes_takes_the_product_of_its_two_shares() {
        assert_eq!(
            bands(&rectangle(10.0, 20.0, 10.25, 20.25), Transform::IDENTITY),
            [(10.0, 20.0, 11.0, 21.0, 0.0625)]
        );
    }

    /// Under a shear no device axis is a path axis, so there is no pixel line to stretch into and
    /// the rasteriser keeps the shape — [`crate::collapsed`] declines the same case.
    #[test]
    fn a_shear_is_declined_because_a_thin_bands_pixel_run_is_a_staircase() {
        let shear = Transform::new(1.0, 0.25, 0.0, 1.0, 0.0, 0.0);
        assert!(!shear.preserves_axes(), "the case under test");
        assert_eq!(
            sub_pixel_bands(&rectangle(10.0, 20.0, 90.0, 20.1), shear, FillRule::NonZero),
            None
        );
    }

    /// A subpath that is not a rectangle takes the whole path out of the rule, because a fill's
    /// subpaths share a winding rule and removing one changes what the others enclose.
    #[test]
    fn one_subpath_the_rule_cannot_take_declines_the_whole_path() {
        let mut mixed = rectangle(10.0, 20.0, 90.0, 20.1);
        mixed.extend(
            path(&[
                PathCommand::MoveTo(Point::new(0.0, 0.0)),
                PathCommand::LineTo(Point::new(30.0, 0.0)),
                PathCommand::LineTo(Point::new(30.0, 30.0)),
                PathCommand::Close,
            ])
            .commands(),
        );
        assert_eq!(
            sub_pixel_bands(&mixed, Transform::IDENTITY, FillRule::NonZero),
            None
        );
    }

    /// Two rules meeting in one pixel row are declined: separate draws would composite them where
    /// one scan conversion adds them, and the rasteriser's own answer is not a disappearance.
    #[test]
    fn two_rules_sharing_a_row_are_left_to_the_rasteriser() {
        let mut pair = rectangle(10.0, 20.0, 90.0, 20.4);
        pair.extend(rectangle(10.0, 20.5, 90.0, 20.9).commands());
        assert_eq!(
            sub_pixel_bands(&pair, Transform::IDENTITY, FillRule::NonZero),
            None
        );
    }

    /// Two rules in *different* rows are both substituted, which is the case a table of ruled
    /// lines written as one path is.
    #[test]
    fn two_rules_in_different_rows_are_both_substituted() {
        let mut pair = rectangle(10.0, 20.0, 90.0, 20.4);
        pair.extend(rectangle(10.0, 22.5, 90.0, 22.9).commands());
        let drawn = bands(&pair, Transform::IDENTITY);
        assert_eq!(drawn.len(), 2, "{drawn:?}");
        assert_eq!((drawn[0].1, drawn[0].3, drawn[0].4), (20.0, 21.0, 0.4));
        assert_eq!((drawn[1].1, drawn[1].3, drawn[1].4), (22.0, 23.0, 0.4));
    }

    /// Under the even-odd rule two crossing rectangles leave a hole at their crossing, which two
    /// composited draws cannot express — so a path whose thin axes differ is declined there and
    /// taken under the non-zero rule.
    #[test]
    fn crossing_rules_are_declined_under_the_even_odd_rule_only() {
        let mut cross = rectangle(10.0, 20.0, 90.0, 20.4);
        cross.extend(rectangle(40.0, 5.0, 40.4, 60.0).commands());
        assert_eq!(
            sub_pixel_bands(&cross, Transform::IDENTITY, FillRule::EvenOdd),
            None
        );
        assert_eq!(
            sub_pixel_bands(&cross, Transform::IDENTITY, FillRule::NonZero)
                .expect("the non-zero rule composites the crossing correctly")
                .len(),
            2
        );
    }

    /// The common case must cost nothing beyond one memoised comparison: a shape with an area
    /// larger than a pixel is not a rectangle this rule takes.
    #[test]
    fn an_ordinary_shape_is_not_substituted() {
        assert_eq!(
            sub_pixel_bands(
                &rectangle(0.0, 0.0, 10.0, 10.0),
                Transform::IDENTITY,
                FillRule::NonZero
            ),
            None
        );
    }

    /// A flat subpath is [`crate::collapsed`]'s rule and not this one: it has no area at all, so
    /// there is no coverage to be proportional to.
    #[test]
    fn a_collapsed_subpath_is_not_this_rule() {
        assert_eq!(
            sub_pixel_bands(
                &rectangle(10.0, 20.0, 90.0, 20.0),
                Transform::IDENTITY,
                FillRule::NonZero
            ),
            None
        );
    }

    /// The gate a backend puts in front of converting a thin stroke into a fill: a rule is flat,
    /// and a polyline that turns a corner is not.
    #[test]
    fn only_a_path_of_straight_rules_is_worth_stroking_into_an_outline() {
        let rule = path(&[
            PathCommand::MoveTo(Point::new(10.0, 20.0)),
            PathCommand::LineTo(Point::new(90.0, 20.0)),
        ]);
        assert!(only_flat_subpaths(&rule));
        let corner = path(&[
            PathCommand::MoveTo(Point::new(10.0, 20.0)),
            PathCommand::LineTo(Point::new(90.0, 20.0)),
            PathCommand::LineTo(Point::new(90.0, 60.0)),
        ]);
        assert!(!only_flat_subpaths(&corner));
        assert!(!only_flat_subpaths(&Path::new()), "nothing to stroke");
    }
}
