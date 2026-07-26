//! Reconciling raster sizes before comparison.
//!
//! # The problem, measured
//!
//! A4 is 595.276 PDF units wide. At 72 dpi that is 595.276 pixels, and the reference
//! renderers do not agree on how to turn a fraction into an integer: `pdftoppm` and
//! `mutool draw` produce 596 pixels, `gs` produces 595. Every A4 document in the corpus
//! therefore fails comparison outright, on a difference that is nobody's bug.
//!
//! # The rule
//!
//! A disagreement of at most [`MAX_ROUNDING_SLACK`] pixels per axis is treated as
//! rounding and reconciled by cropping every raster to the smallest common size.
//! Anything larger is left as an error.
//!
//! Cropping rather than scaling, because all these renderers place the page origin at
//! the raster's top-left corner: the surplus is a sliver at the right or bottom edge, so
//! removing it aligns the images without resampling. Scaling would blur every pixel to
//! fix a one-pixel edge and destroy the exactness the comparison depends on.
//!
//! The bound is deliberately tight. A two-pixel difference at 72 dpi is not rounding, and
//! quietly absorbing it would hide exactly the `MediaBox` and `CropBox` misreadings the
//! harness exists to catch. Normalisation is always reported, never silent — see
//! [`Normalisation`].

use pdf_render::{Raster, RasterFormat};

use crate::HarnessError;

/// Largest per-axis pixel difference attributable to rounding a fractional page size.
///
/// One pixel. A fractional dimension can round either way, so two implementations differ
/// by at most one; anything beyond that is a genuine disagreement about the page.
pub const MAX_ROUNDING_SLACK: u32 = 1;

/// Records that rasters were cropped to a common size, and by how much.
///
/// Returned so a report can state it. A harness that silently reshaped its inputs would
/// be untrustworthy in the one situation that matters — when the sizes differ for a real
/// reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Normalisation {
    /// The size everything was cropped to.
    pub common: (u32, u32),
    /// The largest size seen before cropping.
    pub largest: (u32, u32),
}

impl Normalisation {
    /// Returns `true` if any cropping actually took place.
    #[must_use]
    pub fn cropped(&self) -> bool {
        self.common != self.largest
    }
}

impl std::fmt::Display for Normalisation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.cropped() {
            write!(
                f,
                "cropped {}x{} -> {}x{} (rounding)",
                self.largest.0, self.largest.1, self.common.0, self.common.1
            )
        } else {
            write!(f, "sizes already identical")
        }
    }
}

/// Crops every raster to the smallest common size, if the spread is only rounding.
///
/// # Errors
///
/// [`HarnessError::Compare`] if the rasters differ by more than
/// [`MAX_ROUNDING_SLACK`] on either axis, or if none were given. That is a real
/// disagreement about page geometry and must not be absorbed.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "max is at least min by construction, so the spread subtraction cannot \
              underflow"
)]
pub fn to_common_size(rasters: &mut [&mut Raster]) -> Result<Normalisation, HarnessError> {
    let first = rasters.first().ok_or_else(|| HarnessError::Compare {
        detail: "nothing to normalise".to_owned(),
    })?;

    let mut min = (first.width, first.height);
    let mut max = min;
    for raster in rasters.iter() {
        min = (min.0.min(raster.width), min.1.min(raster.height));
        max = (max.0.max(raster.width), max.1.max(raster.height));
    }

    let spread = (max.0 - min.0, max.1 - min.1);
    if spread.0 > MAX_ROUNDING_SLACK || spread.1 > MAX_ROUNDING_SLACK {
        return Err(HarnessError::Compare {
            detail: format!(
                "page sizes differ by {}x{} pixels, more than rounding can explain \
                 ({}x{} to {}x{})",
                spread.0, spread.1, min.0, min.1, max.0, max.1
            ),
        });
    }

    for raster in rasters.iter_mut() {
        if (raster.width, raster.height) != min {
            **raster = crop(raster, min.0, min.1);
        }
    }

    Ok(Normalisation {
        common: min,
        largest: max,
    })
}

/// Returns the top-left `width` x `height` region of a raster.
///
/// # Panics
///
/// Does not panic: a request larger than the source is clamped to the source size.
#[must_use]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "width and height are clamped to the source above, so every row offset \
              lies inside the source buffer"
)]
pub fn crop(raster: &Raster, width: u32, height: u32) -> Raster {
    let width = width.min(raster.width);
    let height = height.min(raster.height);

    let source_row = raster.width as usize * 4;
    let target_row = width as usize * 4;
    let mut data = Vec::with_capacity(target_row * height as usize);

    for y in 0..height as usize {
        let start = y * source_row;
        data.extend_from_slice(&raster.data[start..start + target_row]);
    }

    Raster {
        width,
        height,
        format: RasterFormat::Rgba8,
        data,
    }
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test dimensions are small literals"
)]
mod tests {
    use super::{MAX_ROUNDING_SLACK, crop, to_common_size};
    use pdf_render::{Raster, RasterFormat};

    fn raster(width: u32, height: u32, fill: u8) -> Raster {
        Raster {
            width,
            height,
            format: RasterFormat::Rgba8,
            data: vec![fill; (width as usize) * (height as usize) * 4],
        }
    }

    /// The exact case measured on the corpus: 596 vs 595 pixels wide for A4.
    #[test]
    fn a_one_pixel_width_difference_is_reconciled() {
        let mut a = raster(596, 842, 1);
        let mut b = raster(595, 842, 2);
        let result = to_common_size(&mut [&mut a, &mut b]).expect("one pixel is rounding");

        assert_eq!(result.common, (595, 842));
        assert!(result.cropped());
        assert_eq!((a.width, a.height), (595, 842));
        assert_eq!((b.width, b.height), (595, 842));
        assert_eq!(a.data.len(), b.data.len());
    }

    #[test]
    fn identical_sizes_are_left_alone() {
        let mut a = raster(100, 100, 1);
        let mut b = raster(100, 100, 2);
        let result = to_common_size(&mut [&mut a, &mut b]).expect("identical");
        assert!(!result.cropped(), "no cropping should be reported");
    }

    /// The bound must stay tight: absorbing a larger difference would hide a `MediaBox`
    /// misreading, which is precisely what the harness is for.
    #[test]
    fn a_difference_beyond_rounding_stays_an_error() {
        let mut a = raster(600, 842, 1);
        let mut b = raster(595, 842, 2);
        let err = to_common_size(&mut [&mut a, &mut b]).unwrap_err();
        assert!(matches!(err, crate::HarnessError::Compare { .. }));
        assert_eq!(a.width, 600, "inputs must be left untouched on error");
    }

    #[test]
    fn slack_of_exactly_the_bound_is_accepted() {
        let mut a = raster(100 + MAX_ROUNDING_SLACK, 100 + MAX_ROUNDING_SLACK, 1);
        let mut b = raster(100, 100, 2);
        assert!(to_common_size(&mut [&mut a, &mut b]).is_ok());
    }

    /// Cropping must take the top-left region, since that is where the page origin is.
    #[test]
    fn crop_keeps_the_top_left_region() {
        let mut source = raster(4, 2, 0);
        // Mark the second row so its position after cropping is observable.
        for byte in &mut source.data[4 * 4..] {
            *byte = 9;
        }
        let cropped = crop(&source, 2, 2);
        assert_eq!((cropped.width, cropped.height), (2, 2));
        assert_eq!(&cropped.data[..8], &[0; 8], "first row comes from the top");
        assert_eq!(&cropped.data[8..], &[9; 8], "second row follows it");
    }
}
