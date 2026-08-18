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
//! A **stroke** whose outline is not a rectangle is declined by the substitution above for the
//! first of those reasons — a round cap ends a rule with an arc — and falls to the general
//! construction below rather than to whatever the backend would otherwise have done. ADR 0226.
//!
//! # The turned rule, and why it needs no scan converter of its own
//!
//! Everything above is stated for an axis-aligned rectangle, and until the
//! four-hundred-and-thirty-second session a sub-pixel *stroke* that was not one kept
//! `tiny-skia`'s hairline. Measuring what that hairline draws — `sub_pixel_marks`'s fourth
//! section, a 200-unit band at seven angles — says it is not a placement approximation but a
//! systematic deficit, and §10.7.4 names the direction it goes in:
//!
//! > The area covered by painted pixels shall always be at least as large as the area of the
//! > original shape.
//!
//! The hairline lays down **one pixel per step along the line's longer device axis**, so a band
//! of length `L` at `θ` from the nearer axis carries `L · cos θ · w` of ink where its area is
//! `L · w`: 3.4% short at 15°, 13.4% at 30° and **29.3% at 45°**, at every thickness under a
//! pixel rather than only near the quantum.
//!
//! What answers it is [`substitute_width`] and needs none of the machinery above. §10.7.4's own
//! construction for a shape too thin to measure is a run of *whole pixels* — NOTE 1 and the
//! EXAMPLE beside it say so for the degenerate case, and [`crate::collapsed`] already draws it —
//! so the substitute for a turned rule is **the same rule stroked one device pixel wide**, with
//! the width it gave up carried in the paint's alpha. The ink is then exactly the shape's area
//! for every angle, because widening by `k` and scaling the alpha by `1/k` cancel; nothing is
//! snapped, since the band stays centred on the path at the fractional position the document put
//! it; and at one device pixel the factor is 1, so the construction is the identity at the
//! boundary where it stops.
//!
//! It is *less* exact than the substitution above, which is why that one is still tried first: a
//! band one pixel wide spreads its ink over a pixel where a band of the true width spreads it over
//! `w`, and at the raster's edge the half that falls outside is still lost. What it replaces is a
//! construction that was wrong about the total as well as the placement. ADR 0268.
//!
//! # The marks whose area is a *square* of the width, which the same identity does not reach
//!
//! ADR 0268's identity is the swept **body's**: widening a band by `k` multiplies its area by `k`
//! and dividing the alpha by `k` gives it back. Every other mark a stroke makes has an area that
//! goes as the square of the width — §8.4.3.3's two caps that project, and §8.5.3.2's dot — so
//! widening one by `k` multiplies its area by `k²` and the body's alpha restores only one factor.
//! ADR 0268 dropped the cap for exactly that reason, and measured what dropping it costs: on a
//! rule as long as it is wide the two round caps are 44% of what the document asked for, and on a
//! rule shorter than its own width they are most of it.
//!
//! The factor is known, so the answer is to divide by it rather than to give the mark up:
//! [`enlarged_mark`] states such a mark at the substitute width with an alpha of `(w / W)²`, which
//! puts down exactly the area the document's own width implies at every width under the quantum.
//! Three shapes reach it and they are the whole of §8.4.3.3's and §8.5.3.2's marks:
//!
//! - a cap that projects — [`sub_pixel_caps`], built here so that the shape a backend fills is
//!   Table 53's rather than a stroker's idea of it;
//! - §8.5.3.2's "filled circle centred at the single point" for a degenerate subpath, and
//! - the mark a zero-length *dash* leaves, whose cap "shall always be painted".
//!
//! The last two are [`crate::degenerate`]'s geometry already, and they need nothing here but the
//! diameter and the alpha: both are circles or squares of the line's width, so the same `(w / W)²`
//! is exact for them too.
//!
//! **The caps are a separate mark rather than a wider stroke**, which is what keeps the arithmetic
//! exact instead of merely closer. A butt-capped body at `W` and Table 53's cap at `W` are
//! disjoint — the cap is what projects *beyond* the body's squared-off end — so the two draws add
//! where a stroker's own round-capped outline at `W` would have carried the cap at the body's
//! alpha and overstated it by `W / w`. What the two draws still lose is the seam: an anti-aliasing
//! rasteriser gives each of them partial coverage in the pixels along the shared edge, and two
//! draws composite as `1 − (1 − a)(1 − b)` where one scan conversion would have added. That is one
//! pixel line at the end of a mark this whole module exists because it is under a pixel, and it is
//! the same seam `doc/todo/11`'s last item carries for a tiling's cell boundary.
//!
//! # Where the alpha itself runs out, and what the clause says about that
//!
//! Every construction above carries a mark's given-up area in the paint's **alpha**, and an alpha
//! is eight bits. So each of them has a second floor, further down than the rasteriser's coverage
//! quantum and reached by a different road: a coverage under `1/255` rounds to nothing, and the
//! mark is gone as completely as it was before any of this existed. Measured on
//! `render-quorra/examples/sub_pixel_marks`' seventh section, a 200-unit rule at 0.002 and 0.001
//! of a device pixel drew **no ink at all** on both backends, and §8.5.3.2's dot — whose alpha is
//! a *square* — was gone at 0.05, which is twenty times thicker and is a width the corpus states.
//!
//! §10.7.4 decides what an answer of zero means, and it is the sentence this whole module opens
//! with: "[t]his ensures that no shape ever disappears". [`expressible_coverage`] is that sentence
//! as arithmetic — a coverage that is positive and under one level is stated at one level — and it
//! is the only place the raster's depth is named, so that neither backend decides it alone.
//!
//! **It costs ink, deliberately, and the clause asks for that direction rather than the other**:
//! the same paragraph says "[t]he area covered by painted pixels shall always be at least as large
//! as the area of the original shape", so a mark this floor raises is a mark drawn heavier than
//! its geometry, which is the side of the sentence a `shall` is on. What it does *not* do is what
//! §10.7.5 would: the mark stays where the document put it and keeps the width the substitution
//! gave it, and only the last level of its alpha moves. §10.7.5's promotion of a sub-half-pixel
//! stroke to a single-pixel line is a different rule with a different trigger — "[i]f stroke
//! adjustment is enabled" — and Table 52 gives that parameter an initial value of `false`.

use crate::collapsed::{Extent, subpath_extents};
use crate::degenerate::KAPPA;
use crate::geom::{Path, PathCommand, Point, Transform};
use crate::paint::{FillRule, LineCap};

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
    /// The coverage the shape's own area implies there, in `0.0 < coverage <= 1.0`, and never
    /// under one level of the raster: [`expressible_coverage`].
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
                coverage: expressible_coverage(band.coverage),
            });
        }
    }
    Some(bands)
}

/// The smallest coverage an eight-bit raster can hold: one level of 255.
///
/// Named here because it is the raster's depth rather than any clause's number, and because
/// [`expressible_coverage`] is the only place this crate is allowed to know it.
const ONE_LEVEL: f32 = 1.0 / 255.0;

/// A substituted mark's coverage, raised to what the raster can hold — ISO 32000-2 §10.7.4.
///
/// Every substitution in this module states a mark wider than the document's own and carries the
/// area it gave up in the paint's alpha. That alpha is eight bits, so a coverage under one level
/// of 255 puts down nothing at all, and the mark disappears — which is what the clause's stated
/// purpose forbids:
///
/// > This ensures that no shape ever disappears as a result of unfavourable placement relative to
/// > the device pixel grid, as might happen with other possible scan conversion rules.
///
/// So a coverage that is positive and under one level is stated *at* one level. Anything at or
/// above it, and anything that is not positive, is returned unchanged: a mark with no area names
/// no shape, and [`crate::collapsed`] rather than this answers a subpath that has none.
///
/// The mark is drawn heavier than its geometry wherever this bites, which is the direction the
/// same paragraph asks for — "[t]he area covered by painted pixels shall always be at least as
/// large as the area of the original shape" — and it is the only thing that moves: the mark keeps
/// its place and the width the substitution gave it, so this is not §10.7.5's promotion, which
/// needs a stroke adjustment parameter Table 52 initialises to `false`.
#[must_use]
pub fn expressible_coverage(coverage: f32) -> f32 {
    if coverage > 0.0 && coverage < ONE_LEVEL {
        ONE_LEVEL
    } else {
        coverage
    }
}

/// The stroke width, in the path's own space, at which a mark is one whole device pixel across
/// whichever way the path runs — ISO 32000-2 §10.7.4's substitute for a rule too thin to measure.
///
/// The reciprocal of [`Transform::min_stretch`], and the module comment argues why that is the
/// right one of the two singular values: a substitute narrower than a device pixel anywhere along
/// its length would be handed back to the quantum this whole module exists to get around. For a
/// similarity — every page transform — it is [`crate::thinnest_line`] to the last bit, and the two
/// differ only where the transform stretches the axes by different factors, where this one is the
/// larger. What that costs is sharpness along the stretched axis and it costs no ink at all: a
/// caller scales the paint's alpha by `width / substitute_width`, and widening by a factor while
/// dividing the alpha by it leaves the area exactly where it was.
///
/// Returns `None` where the transform is singular or not finite, for [`crate::thinnest_line`]'s
/// reason: a path collapsed to a point has no space of its own left for a width to be stated in.
#[must_use]
pub fn substitute_width(to_device: Transform) -> Option<f32> {
    let floor = to_device.min_stretch();
    if !floor.is_finite() || floor <= 0.0 {
        return None;
    }
    Some(1.0 / floor)
}

/// A mark whose area goes as the *square* of the stroke's width, restated at a width the device
/// can measure — ISO 32000-2 §10.7.4.
///
/// Produced by [`enlarged_mark`]. §8.4.3.3's two projecting caps and §8.5.3.2's dot are all stated
/// by the standard as shapes of the line's width in both directions, so widening one to `width`
/// multiplies its area by the square of the factor and [`Self::coverage`] divides it back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnlargedMark {
    /// The width the mark is to be stated at, in the path's own space: [`substitute_width`]'s.
    pub width: f32,
    /// What the paint's alpha is multiplied by, in `0.0 < coverage < 1.0`, and never under one
    /// level of the raster: [`expressible_coverage`].
    pub coverage: f32,
}

/// How a mark whose area is a square of the line's width is stated where that width is under the
/// device's coverage quantum, ISO 32000-2 §10.7.4.
///
/// Returns `None` — and the caller draws the mark the document's own width states — for a width at
/// or above [`substitute_width`], where nothing is being widened and the rasteriser can measure the
/// mark as it is, and for a width that is not positive and finite, which names no mark at all.
///
/// The module comment argues the construction and lists the three marks that reach it. The arithmetic
/// is one line: a shape of width `w` restated at `W` covers `(W / w)²` times the area, so an alpha of
/// `(w / W)²` puts down what the document asked for, exactly, at every width under the quantum —
/// except at the bottom, where the *square* takes it under one level of 255 and
/// [`expressible_coverage`] states the least the raster can hold instead of nothing at all.
#[must_use]
pub fn enlarged_mark(width: f32, to_device: Transform) -> Option<EnlargedMark> {
    let substitute = substitute_width(to_device)?;
    if !width.is_finite() || width <= 0.0 || width >= substitute {
        return None;
    }
    let ratio = width / substitute;
    Some(EnlargedMark {
        width: substitute,
        // A square is where the alpha runs out first: at 0.05 of a device pixel a cap's own
        // `(w / W)²` is 0.0025, which is under one level of 255 and drew nothing.
        coverage: expressible_coverage(ratio * ratio),
    })
}

/// The shapes ISO 32000-2 §8.4.3.3's line cap adds at the ends of a path's open subpaths, stated at
/// an [`EnlargedMark`]'s width and to be **filled** with the stroking paint.
///
/// Table 53 states two caps that project beyond the path and one that does not:
///
/// > Round cap . A semicircular arc with a diameter equal to the line width shall be drawn around
/// > the endpoint and shall be filled in.
///
/// > Projecting square cap . The stroke shall continue beyond the endpoint of the path for a
/// > distance equal to half the line width and shall be squared off.
///
/// Both are outside the butt-capped body the caller draws — that is what "beyond the endpoint"
/// means — so the two marks are disjoint and their ink adds. Returns `None` for a butt cap, which
/// projects nothing, and for a path whose subpaths are all closed or all without a direction: a
/// closed subpath has a join where an open one has a cap, and a subpath with no direction at all is
/// §8.5.3.2's, answered by [`crate::degenerate`] before this is reached.
///
/// The shapes are this crate's own geometry rather than a stroker's, for [`crate::degenerate`]'s
/// reason: two backends that each asked their own library for a cap would be two decisions rather
/// than one.
#[must_use]
pub fn sub_pixel_caps(path: &Path, cap: LineCap, mark: EnlargedMark) -> Option<Path> {
    let radius = mark.width / 2.0;
    if cap == LineCap::Butt || !radius.is_finite() || radius <= 0.0 {
        return None;
    }
    let mut caps = Path::new();
    let commands = path.commands();
    for extent in subpath_extents(path) {
        let subpath = &commands[extent.range()];
        // A closed subpath has a join where an open one has a cap, and a subpath opened by a
        // segment after an `h` begins at a point outside this slice — Table 58's rule that such a
        // segment "shall begin a new subpath" starting where the closed one did. Declining the
        // second is a cap not drawn rather than one drawn in the wrong place.
        if !matches!(subpath.first(), Some(PathCommand::MoveTo(_)))
            || subpath.iter().any(|c| matches!(c, PathCommand::Close))
        {
            continue;
        }
        for (at, outward) in open_ends(subpath) {
            append_cap(&mut caps, at, outward, radius, cap);
        }
    }
    (!caps.is_empty()).then_some(caps)
}

/// A subpath's two ends, each with the unit vector pointing out of the path there.
///
/// The direction at an end is towards the nearest point of the subpath that differs from it,
/// reversed — which is the segment's own direction for a line and the cubic's tangent for a curve,
/// since a Bézier leaves its first point towards its first control point. Where a control point
/// coincides with the end the next one along answers, which is the same fallback a stroker makes
/// and the only one that keeps a cap facing outwards on `c` curves a producer wrote flat.
///
/// Empty for a subpath that reaches no second point at all: §8.5.3.2 gives such a subpath its own
/// answer, and a cap with no orientation is what that clause declines to draw.
///
/// The slice begins with the subpath's own `m`, which the caller has established.
fn open_ends(subpath: &[PathCommand]) -> impl Iterator<Item = (Point, Point)> {
    let mut points = subpath.iter().flat_map(|command| match *command {
        PathCommand::MoveTo(p) | PathCommand::LineTo(p) => [Some(p), None, None],
        PathCommand::CurveTo(a, b, p) => [Some(a), Some(b), Some(p)],
        PathCommand::Close => [None, None, None],
    });
    let first = points.next().flatten();
    let mut ends = [None, None];
    if let Some(start) = first {
        let mut last = start;
        let mut before_last = None;
        for point in points.flatten() {
            if point != last {
                before_last = Some(last);
                last = point;
            }
            if ends[0].is_none() && point != start {
                // The direction *out* of the subpath at its start is away from where it goes.
                ends[0] = Some((start, away(start, point)));
            }
        }
        if let Some(before) = before_last {
            ends[1] = Some((last, away(last, before)));
        }
    }
    ends.into_iter().flatten()
}

/// The unit vector at `from` pointing away from `towards`, or `None`'s stand-in — the caller only
/// asks where the two points differ, so the length is positive.
fn away(from: Point, towards: Point) -> Point {
    let (dx, dy) = (from.x - towards.x, from.y - towards.y);
    let length = crate::geom::length(dx, dy);
    if length > 0.0 && length.is_finite() {
        Point::new(dx / length, dy / length)
    } else {
        Point::new(0.0, 0.0)
    }
}

/// Appends ISO 32000-2 Table 53's cap at `at`, facing `outward`, for a line of radius `radius`.
///
/// A zero `outward` — the two points the direction was taken from coincide after all — appends
/// nothing, which is [`open_ends`]'s own answer for a subpath with no direction.
fn append_cap(into: &mut Path, at: Point, outward: Point, radius: f32, cap: LineCap) {
    if outward.x == 0.0 && outward.y == 0.0 {
        return;
    }
    let along = Point::new(outward.x * radius, outward.y * radius);
    let across = Point::new(-along.y, along.x);
    let point = |a: f32, b: f32| {
        Point::new(
            at.x + a * along.x + b * across.x,
            at.y + a * along.y + b * across.y,
        )
    };
    match cap {
        // Table 53's butt cap "shall be squared off at the endpoint", which is where the body the
        // caller draws already ends.
        LineCap::Butt => {}
        LineCap::Square => {
            into.push(PathCommand::MoveTo(point(0.0, 1.0)));
            into.push(PathCommand::LineTo(point(1.0, 1.0)));
            into.push(PathCommand::LineTo(point(1.0, -1.0)));
            into.push(PathCommand::LineTo(point(0.0, -1.0)));
            into.push(PathCommand::Close);
        }
        LineCap::Round => {
            into.push(PathCommand::MoveTo(point(0.0, 1.0)));
            into.push(PathCommand::CurveTo(
                point(KAPPA, 1.0),
                point(1.0, KAPPA),
                point(1.0, 0.0),
            ));
            into.push(PathCommand::CurveTo(
                point(1.0, -KAPPA),
                point(KAPPA, -1.0),
                point(0.0, -1.0),
            ));
            into.push(PathCommand::Close);
        }
    }
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
    use crate::paint::{FillRule, LineCap};

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

    /// **A bowtie is not a rectangle, however many of the box's corners it touches.**
    ///
    /// [`is_axis_aligned_rectangle`] argues this in prose — "a bowtie is excluded by the second
    /// condition" — and prose is what `hayro`'s issue 1336 was: their `path_as_rect` accepted
    /// any five-element path whose vertices hit all four corners of the bounding box *in any
    /// order*, so a self-intersecting quadrilateral took a fill-rule-independent fast path and
    /// came out as its full box under both rules. CAD plots draw valve symbols as filled
    /// bowties, so the shape is ordinary rather than adversarial.
    ///
    /// The rule that must not be lost is §8.5.3.3's: the two fill rules disagree about a
    /// crossing — the even-odd rule (§8.5.3.3.2) cancels it, the non-zero rule (§8.5.3.3.1)
    /// does not — so no shortcut that erases the crossing can be correct for both. Here the
    /// per-edge test rejects the shape before the question arises, because a bowtie's crossing
    /// sides are diagonals and every side of a rectangle moves along exactly one axis.
    ///
    /// The path below is deliberately *thin*, which is what makes the test sharp: it is inside
    /// the band this rule is looking for, it names all four corners of its bounding box, and it
    /// is five elements long. Only the traversal order tells it apart from a rectangle.
    #[test]
    fn a_bowtie_is_not_taken_for_a_rectangle() {
        let bowtie = path(&[
            PathCommand::MoveTo(Point::new(10.0, 20.0)),
            PathCommand::LineTo(Point::new(90.0, 20.4)),
            PathCommand::LineTo(Point::new(90.0, 20.0)),
            PathCommand::LineTo(Point::new(10.0, 20.4)),
            PathCommand::Close,
        ]);
        for rule in [FillRule::NonZero, FillRule::EvenOdd] {
            assert_eq!(
                sub_pixel_bands(&bowtie, Transform::IDENTITY, rule),
                None,
                "a crossing quadrilateral has no rectangle substitute under {rule:?}"
            );
        }
        // The same four corners in perimeter order *are* a rectangle, so the rejection above
        // is about the order rather than about the points.
        assert!(
            sub_pixel_bands(
                &rectangle(10.0, 20.0, 90.0, 20.4),
                Transform::IDENTITY,
                FillRule::NonZero
            )
            .is_some(),
            "the perimeter traversal of the same box is the shape this rule exists for"
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

    /// The turned rule's substitute is one device pixel across, and for a page transform that is
    /// [`crate::thinnest_line`]'s answer exactly.
    #[test]
    fn the_substitute_is_one_device_pixel_wide_under_a_similarity() {
        for scale in [1.0_f32, 0.5, 4.0] {
            let at = Transform::scale(scale, scale);
            assert_eq!(
                super::substitute_width(at),
                crate::thinnest_line(at),
                "a similarity stretches every direction alike, so the two readings agree"
            );
        }
        assert_eq!(super::substitute_width(Transform::scale(0.0, 0.0)), None);
    }

    /// Where the axes are stretched by different factors the substitute takes the *smaller*, so
    /// that it is a whole pixel across whichever way the rule runs.
    #[test]
    fn an_anisotropic_transform_widens_the_substitute_rather_than_narrowing_it() {
        let at = Transform::scale(4.0, 0.25);
        let width = super::substitute_width(at).expect("a nonsingular transform");
        assert!(
            (width - 4.0).abs() < 1e-4,
            "the y axis shrinks by four, so a whole pixel there is four path units: {width}"
        );
        assert!(
            width > crate::thinnest_line(at).expect("a nonsingular transform"),
            "the reading §8.4.3.2 asks for is the narrower of the two"
        );
    }

    /// A mark whose area is a square of the width keeps that area at the substitute: a quarter of
    /// a pixel across is a sixteenth of a pixel of ink, which is what the document's own width
    /// covers.
    #[test]
    fn a_mark_stated_at_the_substitute_keeps_the_area_its_own_width_implies() {
        let mark = super::enlarged_mark(0.25, Transform::IDENTITY).expect("under the quantum");
        assert!((mark.width - 1.0).abs() < 1e-6, "{mark:?}");
        assert!((mark.coverage - 0.0625).abs() < 1e-6, "{mark:?}");
        // The device's pixel, not the page's: at twice the scale the same width is half a pixel.
        let doubled = super::enlarged_mark(0.25, Transform::scale(2.0, 2.0)).expect("under it");
        assert!((doubled.width - 0.5).abs() < 1e-6, "{doubled:?}");
        assert!((doubled.coverage - 0.25).abs() < 1e-6, "{doubled:?}");
    }

    /// At and above the quantum nothing is widened, so nothing is owed: the rasteriser measures
    /// the mark the document stated and a scaled alpha would only make it faint.
    #[test]
    fn a_mark_at_the_quantum_is_left_exactly_as_the_document_stated_it() {
        assert_eq!(super::enlarged_mark(1.0, Transform::IDENTITY), None);
        assert_eq!(super::enlarged_mark(4.0, Transform::IDENTITY), None);
        assert_eq!(super::enlarged_mark(0.0, Transform::IDENTITY), None);
        assert_eq!(super::enlarged_mark(0.5, Transform::scale(0.0, 0.0)), None);
    }

    /// Table 53's projecting square cap continues "beyond the endpoint of the path for a distance
    /// equal to half the line width", which is where the butt-capped body the caller draws stops.
    #[test]
    fn a_square_cap_is_the_half_width_the_table_states_beyond_each_end() {
        let rule = path(&[
            PathCommand::MoveTo(Point::new(10.0, 20.0)),
            PathCommand::LineTo(Point::new(90.0, 20.0)),
        ]);
        let mark = super::enlarged_mark(0.2, Transform::IDENTITY).expect("under the quantum");
        let caps = super::sub_pixel_caps(&rule, LineCap::Square, mark).expect("two caps");
        let hull = caps.hull().expect("two squares");
        assert_eq!(
            (hull.min.x, hull.max.x),
            (9.5, 90.5),
            "half of one device pixel beyond each end"
        );
        assert_eq!(
            (hull.min.y, hull.max.y),
            (19.5, 20.5),
            "the substitute's width"
        );
        assert_eq!(caps.commands().len(), 10, "two closed quadrilaterals");
    }

    /// The round cap is "a semicircular arc with a diameter equal to the line width … around the
    /// endpoint": it reaches half a width past the end and no further back than the end itself,
    /// which is what makes it disjoint from the body.
    #[test]
    fn a_round_cap_is_the_semicircle_beyond_the_end_and_nothing_behind_it() {
        let rule = path(&[
            PathCommand::MoveTo(Point::new(10.0, 20.0)),
            PathCommand::LineTo(Point::new(90.0, 20.0)),
        ]);
        let mark = super::enlarged_mark(0.2, Transform::IDENTITY).expect("under the quantum");
        let caps = super::sub_pixel_caps(&rule, LineCap::Round, mark).expect("two caps");
        let hull = caps.hull().expect("two half discs");
        assert_eq!((hull.min.x, hull.max.x), (9.5, 90.5));
        assert_eq!((hull.min.y, hull.max.y), (19.5, 20.5));
        assert_eq!(caps.commands().len(), 8, "two arcs of two cubics apiece");
    }

    /// A butt cap "shall be squared off at the endpoint", so there is nothing outside the body to
    /// state — the case that must cost nothing, since it is what most documents write.
    #[test]
    fn a_butt_cap_states_no_mark_of_its_own() {
        let rule = path(&[
            PathCommand::MoveTo(Point::new(10.0, 20.0)),
            PathCommand::LineTo(Point::new(90.0, 20.0)),
        ]);
        let mark = super::enlarged_mark(0.2, Transform::IDENTITY).expect("under the quantum");
        assert_eq!(super::sub_pixel_caps(&rule, LineCap::Butt, mark), None);
    }

    /// §8.4.3.3 gives the cap to "both ends of open subpaths", so a closed one has none: what it
    /// has at the point where it meets itself is §8.4.3.5's join.
    #[test]
    fn a_closed_subpath_is_capped_nowhere() {
        let triangle = path(&[
            PathCommand::MoveTo(Point::new(10.0, 20.0)),
            PathCommand::LineTo(Point::new(90.0, 20.0)),
            PathCommand::LineTo(Point::new(90.0, 60.0)),
            PathCommand::Close,
        ]);
        let mark = super::enlarged_mark(0.2, Transform::IDENTITY).expect("under the quantum");
        assert_eq!(super::sub_pixel_caps(&triangle, LineCap::Round, mark), None);
    }

    /// A cap faces along the path at the end it caps, which for a curve is its own tangent —
    /// towards the last control point rather than towards where the curve started.
    #[test]
    fn a_curves_cap_faces_along_its_tangent() {
        let curve = path(&[
            PathCommand::MoveTo(Point::new(10.0, 20.0)),
            PathCommand::CurveTo(
                Point::new(10.0, 60.0),
                Point::new(50.0, 90.0),
                Point::new(90.0, 90.0),
            ),
        ]);
        let mark = super::enlarged_mark(0.2, Transform::IDENTITY).expect("under the quantum");
        let caps = super::sub_pixel_caps(&curve, LineCap::Square, mark).expect("two caps");
        let hull = caps.hull().expect("two squares");
        assert_eq!(
            (hull.min.x, hull.max.x),
            (9.5, 90.5),
            "the start's tangent is vertical and the end's horizontal, so only the end reaches \
             past 90 in x"
        );
        assert_eq!((hull.min.y, hull.max.y), (19.5, 90.5));
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

    /// ISO 32000-2 §10.7.4's "no shape ever disappears", reaching the eight bits the substituted
    /// coverage is carried in.
    ///
    /// The three rungs are the whole rule: a coverage the raster can hold is untouched, one it
    /// cannot is stated at the least it can, and nothing at all stays nothing — a mark with no
    /// area names no shape, and `crate::collapsed` rather than this answers a subpath with none.
    #[test]
    fn a_coverage_under_one_level_is_stated_at_one_level() {
        assert!((super::expressible_coverage(0.5) - 0.5).abs() < f32::EPSILON);
        let floor = 1.0 / 255.0;
        assert!((super::expressible_coverage(floor) - floor).abs() < f32::EPSILON);
        assert!((super::expressible_coverage(floor / 10.0) - floor).abs() < f32::EPSILON);
        assert!((super::expressible_coverage(0.0) - 0.0).abs() < f32::EPSILON);
    }

    /// A mark whose area is a *square* of the width reaches that floor twenty times sooner.
    ///
    /// `(w / W)²` at a fiftieth of a device pixel is 0.0004, which is a tenth of a level: the body
    /// of a rule that thin still lands and §8.4.3.3's cap on the end of it would not.
    #[test]
    fn an_enlarged_marks_coverage_is_floored_where_its_square_takes_it_under_a_level() {
        let mark = super::enlarged_mark(0.02, Transform::IDENTITY).expect("under the quantum");
        assert!((mark.width - 1.0).abs() < f32::EPSILON, "one device pixel");
        assert!(
            (mark.coverage - 1.0 / 255.0).abs() < f32::EPSILON,
            "0.02 squared is 0.0004, a tenth of a level, and one level is what the raster holds"
        );
        let ordinary = super::enlarged_mark(0.2, Transform::IDENTITY).expect("under the quantum");
        assert!(
            (ordinary.coverage - 0.04).abs() < f32::EPSILON,
            "0.2 squared is well above a level and is left exactly where the arithmetic puts it"
        );
    }
}
