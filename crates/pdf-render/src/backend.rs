//! The interface every rasteriser backend implements.

use crate::display_list::DisplayList;
use crate::geom::Transform;

/// Pixel layout of a [`Raster`].
///
/// **Deliberately not `#[non_exhaustive]`, and it was until the four-hundred-and-eleventh
/// session.** A [`Raster`] is what `viewer-core` hands a host inside `Rendered::Raster` and
/// `Answer::Frame`, so this enum is part of that crate's vocabulary whichever crate declares it —
/// and `doc/ui-boundary.md` states the rule for that vocabulary in one sentence: *"a new `Event`
/// should fail to compile in every consumer"*. `#[non_exhaustive]` bought the opposite. It forced
/// a catch-all arm on `viewer-gtk`, on `viewer-qt`, on `viewer-ui`'s software path and on
/// `viewer-confined`'s wire format, each of which had to refuse an unknown layout at *runtime* —
/// the best a Rust consumer can do, and no help at all to a C consumer, which cannot fail to
/// compile in anybody.
///
/// What it costs is stated rather than hidden: adding a second pixel layout here is a breaking
/// change for every consumer of `pdf-render`, and that is the intended price. A raster's bytes
/// mean something, and a consumer that has not been told what they mean must not composite them.
/// ADR 0247.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RasterFormat {
    /// Four bytes per pixel: red, green, blue, alpha, with straight alpha.
    ///
    /// Straight rather than premultiplied alpha is chosen because it is what both
    /// PNG encoding and the comparison harness expect, and converting once at the
    /// backend boundary is cheaper than converting on every comparison.
    Rgba8,
}

/// What to rasterise into, and at what resolution.
///
/// The transform is supplied separately from the [`DisplayList`] so that one display
/// list can be rendered at many scales — the zoom and pan case — and tiled across
/// several targets, without rebuilding it. Rebuilding a display list means
/// re-interpreting the content stream, which is the expensive part.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetSpec {
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
    /// Maps page user space to target pixel space.
    ///
    /// This carries the device resolution, the flip from PDF's bottom-left origin to
    /// the top-left origin of a raster, and any tile offset.
    pub transform: Transform,
}

/// Largest permitted raster dimension, in pixels.
///
/// `2^24` is the largest integer that `f32` represents exactly, and the page's extent in
/// device units is converted to `f32` when the target transform is built. Capping here keeps
/// that conversion inside the range where an `f32` still resolves a fraction of a pixel —
/// which is the difference between a guarantee and a hope, and which the y flip depends on:
/// see [`TargetSpec::for_page`] on why it translates by the page's height and not the
/// raster's.
pub const MAX_EXTENT: u32 = 1 << 24;

/// Deepest nesting of [`crate::Command::Group`] a backend will composite.
///
/// A backend renders a group by recursing over its elements, so the nesting depth of a
/// display list becomes stack depth. `pdf-model` bounds groups by its own form-XObject
/// depth, but a [`DisplayList`] is a public type that anything may build, and a rasteriser
/// must not be able to exhaust the stack because it was handed a list nobody checked. The
/// value matches that bound: legitimate artwork nests a handful of groups, and sixteen is
/// already far past anything the corpus contains.
pub const MAX_GROUP_DEPTH: usize = 16;

impl TargetSpec {
    /// Builds a spec that renders a whole page at the given scale.
    ///
    /// `scale` is pixels per PDF unit: at 72 dpi it is `1.0`, and at 150 dpi it is
    /// `150.0 / 72.0`. The returned spec includes the y-axis flip.
    ///
    /// # Errors
    ///
    /// - [`BackendError::DegenerateTarget`] if either dimension is not finite or
    ///   rounds to less than one pixel.
    /// - [`BackendError::ExtentTooLarge`] if either dimension exceeds
    ///   [`MAX_EXTENT`].
    /// - [`BackendError::TargetTooLarge`] if the total pixel count exceeds
    ///   `max_pixels`.
    ///
    /// Callers must pass a real `max_pixels` bound. Page dimensions come from the
    /// document, so an unbounded scale multiplication is a memory-exhaustion vector.
    pub fn for_page(list: &DisplayList, scale: f32, max_pixels: u64) -> Result<Self, BackendError> {
        let exact_height = f64::from(list.page_size.height) * f64::from(scale);
        let width = pixel_extent(f64::from(list.page_size.width) * f64::from(scale))?;
        let height = pixel_extent(exact_height)?;

        // Integer arithmetic throughout, so the limit check needs no floating-point
        // reasoning at all.
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "both factors are bounded by MAX_EXTENT = 2^24, so their product \
                      is at most 2^48 and cannot overflow u64"
        )]
        let total = u64::from(width) * u64::from(height);
        if total > max_pixels {
            return Err(BackendError::TargetTooLarge {
                requested: total,
                limit: max_pixels,
            });
        }

        Ok(Self {
            width,
            height,
            // Scale, then flip y about the page's top edge: PDF places the origin at
            // the bottom left, a raster at the top left.
            //
            // **About the page's top edge, not the raster's bottom row.** The two are the
            // same number only when the page's height in device units is a whole number of
            // pixels, and `pixel_extent` rounds *up* so that the raster contains the page —
            // so a page 56.122 units tall gets 57 rows and the leftover 0.878 has to go
            // somewhere. Translating by the raster's height puts it at the top and pushes
            // every mark on the page down by that fraction; translating by the page's own
            // height puts it at the bottom, where the x axis already puts its own leftover
            // because nothing flips x. Anchoring both axes at the crop box's top-left corner
            // is the only choice that treats them alike.
            //
            // ISO 32000-2 states none of this — §10.7 leaves scan conversion to the device
            // and says nothing about a page whose size is not a whole number of pixels — so
            // it is a documented choice. What it is not is invisible: `issue3694_reduced.pdf`
            // is 272.595 x 56.122, and until the sixty-first session its content sat one row
            // below where all four reference renderers put it, *including the one whose
            // raster is the same size as ours*. See ADR 0064.
            transform: Transform::scale(scale, -scale)
                .then(Transform::translate(0.0, height_as_f32(exact_height))),
        })
    }
}

/// The page's height in device units, for the y flip to translate by.
///
/// No validity check of its own: `pixel_extent` has already refused anything that is not
/// finite, is under one pixel, or exceeds [`MAX_EXTENT`], and it was handed this exact value.
#[expect(
    clippy::cast_possible_truncation,
    reason = "pixel_extent has already bounded this value to 1.0..=MAX_EXTENT = 2^24, which \
              f32 represents with room to spare"
)]
fn height_as_f32(exact: f64) -> f32 {
    debug_assert!(exact.is_finite() && exact > 0.0, "checked by pixel_extent");
    exact as f32
}

/// Rounds a computed pixel extent up to a raster dimension.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "value is bounds-checked to 1.0..=MAX_EXTENT immediately before each \
              cast, and float-to-int `as` saturates rather than wrapping"
)]
fn pixel_extent(value: f64) -> Result<u32, BackendError> {
    let value = value.ceil();
    if !value.is_finite() || value < 1.0 {
        return Err(BackendError::DegenerateTarget);
    }
    if value > f64::from(MAX_EXTENT) {
        return Err(BackendError::ExtentTooLarge {
            extent: value as u64,
            limit: MAX_EXTENT,
        });
    }
    Ok(value as u32)
}

/// A rendered image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raster {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Pixel layout of `data`.
    pub format: RasterFormat,
    /// Pixel data, row-major from the top-left, with no row padding.
    pub data: Vec<u8>,
}

/// A rasteriser: turns a resolved display list into pixels.
///
/// # Why the whole list at once
///
/// The interface deliberately takes an entire [`DisplayList`] rather than accepting
/// commands one at a time. A per-command interface would prevent a GPU backend from
/// batching: Vello's model is to build a scene and dispatch it as a small number of
/// compute passes, and issuing one call per command would serialise that into
/// thousands of dispatches. Whole-list submission also lets any backend parallelise
/// across tiles, since resolved commands are independent.
pub trait Rasterizer {
    /// Backend-specific failure type.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Human-readable backend name, used in comparison reports and golden filenames.
    fn name(&self) -> &'static str;

    /// Renders `list` into a new raster described by `target`.
    ///
    /// # Errors
    ///
    /// Returns `Self::Error` if the backend cannot produce the raster — device loss
    /// or allocation failure for a GPU backend, allocation failure for a CPU one.
    /// Content that a backend does not yet support must be reported as an error
    /// rather than silently skipped, so the comparison harness sees a failure rather
    /// than a plausible-looking wrong image.
    fn rasterize(&mut self, list: &DisplayList, target: TargetSpec) -> Result<Raster, Self::Error>;
}

/// Failures common to all backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BackendError {
    /// The requested target would need more pixels than the configured limit.
    #[error("target of {requested} pixels exceeds the limit of {limit}")]
    TargetTooLarge {
        /// Pixel count that was asked for.
        requested: u64,
        /// Configured maximum.
        limit: u64,
    },
    /// A single dimension exceeds [`MAX_EXTENT`].
    #[error("raster dimension of {extent} exceeds the maximum extent of {limit}")]
    ExtentTooLarge {
        /// Dimension that was asked for.
        extent: u64,
        /// Configured maximum for a single dimension.
        limit: u32,
    },
    /// The requested target rounds to zero pixels, or its size is not finite.
    #[error("target dimensions are degenerate")]
    DegenerateTarget,
    /// Transparency groups nest deeper than [`MAX_GROUP_DEPTH`].
    #[error("transparency groups nest {depth} deep, over the limit of {limit}")]
    GroupsTooDeep {
        /// Depth that was reached.
        depth: usize,
        /// Configured maximum.
        limit: usize,
    },
}

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "these assertions compare exactly representable values on purpose — the \
              y-flip must land precisely on row zero, and pixel extents are integers \
              — so an approximate comparison would weaken the test"
)]
mod tests {
    use super::{BackendError, MAX_EXTENT, TargetSpec};
    use crate::display_list::DisplayList;
    use crate::geom::{Point, Size};

    /// A4 at 72 dpi, the size most of these cases are built from.
    fn a4() -> DisplayList {
        DisplayList::new(Size::new(595.0, 842.0))
    }

    const GENEROUS: u64 = 1 << 30;

    #[test]
    fn scale_of_one_maps_points_to_pixels() {
        let spec = TargetSpec::for_page(&a4(), 1.0, GENEROUS).unwrap();
        assert_eq!((spec.width, spec.height), (595, 842));
    }

    #[test]
    fn dimensions_round_up_so_no_content_is_clipped() {
        // 150 dpi: 595 * 150/72 = 1239.58..., which must become 1240 rather than 1239.
        let spec = TargetSpec::for_page(&a4(), 150.0 / 72.0, GENEROUS).unwrap();
        assert_eq!(spec.width, 1240);
    }

    /// The y-flip is the detail most easily got wrong: PDF's origin is bottom-left,
    /// a raster's is top-left, so the page's top edge must land on pixel row zero.
    #[test]
    fn transform_flips_the_y_axis() {
        let list = a4();
        let spec = TargetSpec::for_page(&list, 1.0, GENEROUS).unwrap();

        let bottom_left = spec.transform.apply(Point::new(0.0, 0.0));
        let top_left = spec.transform.apply(Point::new(0.0, 842.0));

        assert_eq!(
            bottom_left.y, 842.0,
            "page bottom belongs at the last raster row"
        );
        assert_eq!(top_left.y, 0.0, "page top belongs at raster row zero");
    }

    /// The same rule on a page that is not a whole number of pixels tall.
    ///
    /// A4 is 842 units high and the flip is 842 whichever edge it is anchored to, so the test
    /// above passes under both conventions and passed under the wrong one for the project's
    /// whole life. `issue3694_reduced.pdf`'s crop box is 272.595 x 56.122: the raster gets 57
    /// rows because [`pixel_extent`] rounds up so the page fits, and the leftover 0.878 of a
    /// row goes to the *bottom*, because nothing flips x and the leftover 0.405 of a column
    /// already goes to the right. Anchoring y to the raster's last row instead pushed every
    /// mark on such a page down by that fraction, which was 11 of the corpus's contradicted
    /// pages. ADR 0064.
    #[test]
    fn a_fractional_page_still_starts_at_raster_row_zero() {
        let list = DisplayList::new(Size::new(272.595, 56.122));
        let spec = TargetSpec::for_page(&list, 1.0, GENEROUS).unwrap();
        assert_eq!(
            (spec.width, spec.height),
            (273, 57),
            "the raster rounds up so that it contains the page"
        );

        let top_left = spec.transform.apply(Point::new(0.0, 56.122));
        let bottom_left = spec.transform.apply(Point::new(0.0, 0.0));
        assert!(
            top_left.y.abs() < 1e-3,
            "page top belongs at raster row zero, not at {}",
            top_left.y
        );
        assert!(
            (bottom_left.y - 56.122).abs() < 1e-3,
            "the spare fraction of a row belongs below the page, not above it: {}",
            bottom_left.y
        );
    }

    #[test]
    fn total_pixel_limit_is_enforced() {
        let err = TargetSpec::for_page(&a4(), 10.0, 1000).unwrap_err();
        assert!(matches!(
            err,
            BackendError::TargetTooLarge { limit: 1000, .. }
        ));
    }

    #[test]
    fn single_dimension_limit_is_enforced_before_the_total() {
        // Scale chosen so one dimension alone exceeds MAX_EXTENT.
        let huge = f64::from(MAX_EXTENT) / 595.0 * 2.0;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "test input; the f32 value only needs to be large, not exact"
        )]
        let err = TargetSpec::for_page(&a4(), huge as f32, u64::MAX).unwrap_err();
        assert!(matches!(
            err,
            BackendError::ExtentTooLarge {
                limit: MAX_EXTENT,
                ..
            }
        ));
    }

    #[test]
    fn subpixel_pages_are_rejected_rather_than_rounded_to_zero() {
        let err = TargetSpec::for_page(&a4(), 0.0, GENEROUS).unwrap_err();
        assert!(matches!(err, BackendError::DegenerateTarget));
    }

    /// A non-finite scale must not reach the cast. This is the case that would be
    /// undefined behaviour in C and merely wrong here — it should be an error.
    #[test]
    fn non_finite_scale_is_rejected() {
        for scale in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let err = TargetSpec::for_page(&a4(), scale, GENEROUS).unwrap_err();
            assert!(
                matches!(err, BackendError::DegenerateTarget),
                "scale {scale} should be degenerate, got {err:?}"
            );
        }
    }

    #[test]
    fn negative_scale_is_rejected() {
        let err = TargetSpec::for_page(&a4(), -1.0, GENEROUS).unwrap_err();
        assert!(matches!(err, BackendError::DegenerateTarget));
    }
}
