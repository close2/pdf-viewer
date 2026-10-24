//! How thin a mark is, and the width below which the device lane stops being able to
//! promise ISO 32000-2 §10.7.4.
//!
//! The clause, verbatim, with the sentence that carries the reason:
//!
//! > A shape shall be scan-converted by painting any pixel whose half-open square region
//! > intersects the shape, no matter how small the intersection is. This ensures that no
//! > shape ever disappears as a result of unfavourable placement relative to the device
//! > pixel grid, as might happen with other possible scan conversion rules.
//!
//! The device lane of ADR 0016 counts samples at fixed points, so it answers *is this
//! sample point inside the shape* and never *does this shape intersect this pixel*. A
//! mark narrower than the gaps between the samples can therefore fall entirely between
//! them and read zero — which is the disappearance the sentence above forbids, reached by
//! the route it names. The processor lane computes the exact area (ADR 0005) and cannot
//! lose such a mark.
//!
//! So this module is the two numbers ADR 0070's fifth lane condition compares:
//! [`vanishing_width`], derived from the sample pattern rather than written down, and
//! [`ThinAxis`], which is the one definition of *how thin a mark is* in this tree.

use crate::winding::SamplePattern;

/// The narrowest mark the device lane can promise not to lose wherever it lands, in device
/// pixels: [`SamplePattern::vanishing_width`] of the pattern `samples` lays out.
///
/// **Derived from the pattern, not chosen** (the viewer's ADRs 2296 and 2312). The widest band
/// at any angle that can fall between the samples and the widest axis-aligned square are each
/// computed from where the samples sit; this is the larger, so a mark whose thin axis — the
/// narrower side of its device box, or a stroke's own width — reaches it holds a sample under
/// either reading. Below it there are placements that hold none. Each count's width is found
/// once in the process's life and kept, since a frame asks for it twice.
///
/// `tests/thin_marks.rs` holds both halves of that on the device rather than on paper:
/// `at_the_sample_spacing_the_device_lane_holds_the_clause_at_every_position` sweeps ten
/// sub-pixel positions at exactly this width, and the neighbouring test sweeps the same
/// positions below it.
///
/// **What the extremes of [`Options::coverage_samples`] make of it.** That field is
/// public and settable; construction rounds it down to a square and clamps it to
/// `4..=64`, so this runs from **0.75 at four samples** — the widest empty square of every one
/// of the twenty-four four-sample patterns with a row to each sample — down to **0.141 at
/// sixty-four**. The condition narrows as a caller buys quality, which is the direction it
/// should move in.
///
/// [`Options::coverage_samples`]: crate::startup::Options::coverage_samples
pub(super) fn vanishing_width(samples: u32) -> f32 {
    let pattern = SamplePattern::of(samples);
    match VANISHING_WIDTHS.get(pattern.count() as usize) {
        Some(kept) => *kept.get_or_init(|| pattern.vanishing_width()),
        None => pattern.vanishing_width(),
    }
}

/// Each sample count's [`vanishing_width`], found on first use.
static VANISHING_WIDTHS: [std::sync::OnceLock<f32>; 65] =
    [const { std::sync::OnceLock::new() }; 65];

/// How narrow a mark is across its thinnest known axis, in device pixels.
///
/// **The one definition of "thin" in this tree**, and a newtype rather than an `f32` so
/// that the lane condition cannot be handed a tile side or a stroke width by accident.
///
/// # What it is measured from, and what that misses
///
/// A mark reaches [`Encoder::take_gpu_lane`](super::Encoder::take_gpu_lane) as device
/// geometry plus, for a stroke, the width §8.4.3 already resolved into device pixels
/// (section 4.5 of the brief settles that upstream). Two bounds on its thickness follow, and
/// this is the smaller of the ones that apply:
///
/// - **the narrower side of its device box.** The mark lies inside that box, so across
///   that axis it is nowhere wider. Exact for the axis-aligned rule a document draws
///   most of its thin marks with.
/// - **a stroke's own resolved device width.** A stroke is nowhere wider than that
///   across its path, at any angle, so this catches a turned rule that the box does not
///   — which is why a stroke is measured by both and kept at the smaller.
///
/// **The residual, stated rather than discovered later: a filled hairline at 45°.** A
/// thin parallelogram given as a *fill* has a device box far wider than the mark and no
/// stroke width to be read instead, so it reads thick, keeps the device lane, and is not
/// diverted by ADR 0070's condition. It does not vanish there — it crosses many pixels
/// and catches a sample column in some of them — it **dots**: its coverage is uneven
/// along its length where the processor lane's would be even. The corpus says the
/// residual is small (`raster/doc/notes-thin-mark-options.md` section 2.4: the corpus's largest
/// sub-quarter-pixel stroke population is 29 375 marks on one page and exactly one of
/// them takes the device lane at all).
///
/// A second, quieter case of the same shape: a *curved* mark's box here is its control
/// hull's, which is an over-estimate for a curve, so a thin curve can read thicker than
/// it is. Same direction, same consequence, and the corpus's answer is the same one.
///
/// # Which way the error may point
///
/// Over-stating a mark's thinness costs it the device lane, which is CPU rasterisation
/// for a mark that would have been drawn correctly anyway — a cost. Under-stating it
/// leaves a mark on a lane that can lose it — a §10.7.4 violation. So every bound above
/// is an **upper** bound on the mark's thickness, and where two apply the smaller is
/// taken.
#[derive(Debug, Clone, Copy)]
pub(super) struct ThinAxis(f32);

impl ThinAxis {
    /// The thin axis of a mark with these device bounds — `(x0, y0, x1, y1)` — and, where
    /// the mark is a stroke, the width §8.4.3 resolved for it.
    ///
    /// `None` is a fill, whose box is the only bound there is.
    // two subtractions of device coordinates,
    // each already bounded by `MAX_COORDINATE`; a non-finite difference is handled below
    pub(super) fn of(bounds: (f32, f32, f32, f32), stroke_width: Option<f32>) -> Self {
        let (x0, y0, x1, y1) = bounds;
        let across = (x1 - x0).min(y1 - y0);
        Self(match stroke_width {
            Some(width) => across.min(width),
            None => across,
        })
    }

    /// Whether the device lane's sample pattern could miss this mark entirely.
    ///
    /// Strictly below the width: at exactly the [`vanishing_width`] the pattern catches the
    /// mark wherever it lands, which is the arithmetic that function states and
    /// `tests/thin_marks.rs` measures.
    ///
    /// **A non-finite thin axis answers `false`**, because `f32` comparison against a
    /// `NaN` is false — the mark keeps whichever lane the four cost conditions choose,
    /// which is what it did before this condition existed. That is the right default:
    /// this condition exists to *decline* a lane, and a mark whose geometry is already
    /// beyond the encoder's coordinate ceiling is refused elsewhere by name rather than
    /// being routed on a comparison that means nothing.
    pub(super) fn can_miss_every_sample(self, vanishing_width: f32) -> bool {
        self.0 < vanishing_width
    }
}

#[cfg(test)]
mod tests {
    use super::{ThinAxis, vanishing_width};

    /// The width at each sample count `Options::coverage_samples` can become, including
    /// both ends of the clamp — the numbers the condition's reach is stated in, and wider
    /// than the ordered grid's `1/√n` at every one of them (ADR 2296). At each the derived
    /// pattern's widest empty square is the wider of its two widths (ADR 2313), so each is a
    /// whole number of sample pitches: the lattice's own square at four, nine, sixteen and
    /// thirty-six, where the lattice's band had been wider still at the last three.
    #[test]
    fn the_vanishing_width_at_every_admitted_sample_count() {
        for (samples, expected) in [
            (4, 0.75_f32),
            (9, 4.0 / 9.0_f32),
            (16, 5.0 / 16.0_f32),
            (36, 7.0 / 36.0_f32),
        ] {
            let width = vanishing_width(samples);
            assert!(
                (width - expected).abs() <= f32::EPSILON,
                "{samples} samples: expected {expected}, got {width}"
            );
        }
        assert!(vanishing_width(64) < vanishing_width(16));
    }

    /// A stroke is measured by its own resolved device width as well as by its box, so a
    /// turned hairline — whose box is wide — is still read as thin.
    #[test]
    fn a_strokes_width_is_read_where_it_is_thinner_than_the_box() {
        let width = vanishing_width(16);
        let turned = (0.0, 0.0, 100.0, 100.0);
        assert!(
            !ThinAxis::of(turned, None).can_miss_every_sample(width),
            "a fill with a 100-pixel box is not thin by any bound this type has — which \
             is the residual the type's own documentation states"
        );
        assert!(
            ThinAxis::of(turned, Some(0.1)).can_miss_every_sample(width),
            "a stroke a tenth of a pixel wide is thin however its box is turned"
        );
        // And the box still decides where it is the narrower of the two: a wide stroke
        // clipped to a sliver of a tile is as losable as a thin one.
        assert!(
            ThinAxis::of((0.0, 0.0, 0.1, 100.0), Some(9.0)).can_miss_every_sample(width),
            "the smaller of the two bounds is the one that decides"
        );
    }

    /// The boundary is *below*, not *at*: a mark exactly the vanishing width wide catches
    /// a sample wherever it lands, so it keeps the lane the cost conditions choose.
    #[test]
    fn a_mark_at_exactly_the_spacing_is_not_declined() {
        let width = vanishing_width(16);
        assert!(
            !ThinAxis::of((0.0, 0.0, width, 10.0), None).can_miss_every_sample(width),
            "at exactly the vanishing width the pattern cannot miss the mark"
        );
        assert!(
            ThinAxis::of((0.0, 0.0, width * 0.99, 10.0), None).can_miss_every_sample(width),
            "just below it, it can"
        );
    }

    /// Principle 3's rule at this seam: a thin axis that is not a number must not become
    /// a lane choice made on a comparison that means nothing.
    ///
    /// **Both extents are NaN, and that is the point.** This fixture read
    /// `(0.0, 0.0, f32::NAN, 10.0)` until 2026-08-18, which never produced a NaN thin axis
    /// at all: `ThinAxis::of` takes the smaller extent with `f32::min`, which is IEEE
    /// `minNum` and *discards* a NaN operand, so the axis was a finite `10.0` — forty times
    /// the spacing — and the test asserted only that a ten-pixel mark is not thin. It
    /// passed with the comparison written as `self.0 < spacing`, which is the one
    /// rewriting that flips a NaN's answer, so the gate could not see the property it
    /// names. Found by a round that forced that exact defect and watched nothing go red.
    #[test]
    fn a_non_finite_thin_axis_declines_nothing() {
        assert!(
            !ThinAxis::of((0.0, 0.0, f32::NAN, f32::NAN), None).can_miss_every_sample(0.25),
            "a NaN extent answers false, leaving the four cost conditions to choose"
        );
    }
}
