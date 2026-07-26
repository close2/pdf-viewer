//! Tolerant image comparison for rasteriser and reference-renderer diffing.
//!
//! # Why tolerance is a requirement, not a concession
//!
//! Exact pixel equality is unachievable between independent rasterisers, and it is
//! unachievable even between two *correct* ones. They differ in antialiasing
//! strategy, gamma handling, subpixel positioning and edge rounding. `poppler` and
//! `mupdf` do not agree with each other on any page with a curve in it.
//!
//! A comparison suite built on exact equality therefore produces constant false
//! positives, and a suite that cries wolf gets switched off. Tolerance is what makes
//! the suite trustworthy enough to keep running.
//!
//! # The metric that matters most
//!
//! [`Comparison::worst_tile_error`] exists because mean error is actively misleading
//! on the failures that matter. A missing glyph on a dense page of text changes the
//! mean by a fraction of a percent — indistinguishable from antialiasing noise — but
//! it is a serious rendering bug. Splitting the image into tiles and reporting the
//! worst one catches localised errors that averaging hides.
//!
//! Always assert on both: the mean catches broad shifts such as a gamma or colour
//! error, the worst tile catches concentrated ones such as missing or misplaced
//! geometry.

#![forbid(unsafe_code)]

use pdf_render::{Raster, RasterFormat};

/// Side length in pixels of the tiles used for localised error.
///
/// 32 is small enough that a single missing glyph dominates its tile, and large
/// enough that antialiasing differences along an edge are averaged within it rather
/// than each producing their own maximum.
pub const DEFAULT_TILE: u32 = 32;

/// The result of comparing two rasters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Comparison {
    /// Mean absolute difference across every channel of every pixel, in `0.0..=255.0`.
    ///
    /// Sensitive to broad, uniform differences: a gamma mismatch, a colour-space
    /// error, an inverted image. Insensitive to localised ones — see
    /// [`Self::worst_tile_error`].
    pub mean_error: f64,
    /// The largest single-channel absolute difference anywhere in the image.
    ///
    /// Almost always large when antialiasing differs, since an edge pixel can be fully
    /// covered in one renderer and empty in the other, so this is diagnostic rather
    /// than something to assert on directly.
    pub max_error: u8,
    /// The highest per-tile mean absolute difference.
    ///
    /// The metric to assert on: it survives averaging over a large page while still
    /// reflecting a concentrated defect.
    pub worst_tile_error: f64,
    /// Where the worst tile begins, in pixels from the top-left.
    pub worst_tile_at: (u32, u32),
    /// Fraction of pixels differing by more than a just-noticeable amount, `0.0..=1.0`.
    ///
    /// Useful for separating "everything is slightly off" from "a small region is
    /// completely wrong" when the two metrics above disagree.
    pub differing_fraction: f64,
}

/// A channel difference at or below this is treated as noise for
/// [`Comparison::differing_fraction`].
///
/// Antialiasing and rounding routinely produce differences of a few levels on edge
/// pixels; a difference of four or fewer is not visible in practice.
const JUST_NOTICEABLE: u8 = 4;

/// Compares two rasters using [`DEFAULT_TILE`].
///
/// # Errors
///
/// See [`compare_with_tile`].
pub fn compare(left: &Raster, right: &Raster) -> Result<Comparison, CompareError> {
    compare_with_tile(left, right, DEFAULT_TILE)
}

/// Compares two rasters, tiling with the given side length.
///
/// # Errors
///
/// [`CompareError::DimensionMismatch`] if the rasters differ in size, and
/// [`CompareError::FormatMismatch`] if either is not [`RasterFormat::Rgba8`].
/// Dimensions are checked first and separately because a size difference means the
/// two renderers disagreed about the *page*, which is a different and more serious
/// class of bug than disagreeing about pixels.
///
/// # Panics
///
/// Does not panic: `tile` of zero is clamped to 1.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "every operation in the comparison loop is bounded by the raster \
              dimensions: tile_x < width and tile_y < height by construction of the \
              step_by ranges, and the channel index is within a row that the \
              dimension check above proved both rasters share. Saturating operations \
              would add per-channel cost to the hottest loop in the harness for no \
              gain in safety."
)]
pub fn compare_with_tile(
    left: &Raster,
    right: &Raster,
    tile: u32,
) -> Result<Comparison, CompareError> {
    if left.width != right.width || left.height != right.height {
        return Err(CompareError::DimensionMismatch {
            left: (left.width, left.height),
            right: (right.width, right.height),
        });
    }
    if left.format != RasterFormat::Rgba8 || right.format != RasterFormat::Rgba8 {
        return Err(CompareError::FormatMismatch);
    }

    let tile = tile.max(1);
    let width = left.width;
    let height = left.height;

    let mut total_diff = 0u64;
    let mut max_error = 0u8;
    let mut differing = 0u64;
    let mut worst_tile_error = 0.0f64;
    let mut worst_tile_at = (0, 0);

    for tile_y in (0..height).step_by(tile as usize) {
        for tile_x in (0..width).step_by(tile as usize) {
            let tile_w = tile.min(width - tile_x);
            let tile_h = tile.min(height - tile_y);

            let mut tile_diff = 0u64;
            for y in tile_y..tile_y + tile_h {
                for x in tile_x..tile_x + tile_w {
                    let index = ((y as usize) * (width as usize) + (x as usize)) * 4;
                    for channel in 0..4 {
                        let a = left.data[index + channel];
                        let b = right.data[index + channel];
                        let diff = a.abs_diff(b);
                        tile_diff += u64::from(diff);
                        max_error = max_error.max(diff);
                        if diff > JUST_NOTICEABLE {
                            differing += 1;
                        }
                    }
                }
            }

            total_diff += tile_diff;

            // Channels per tile, so the per-tile figure is comparable to the mean.
            let tile_channels = f64::from(tile_w) * f64::from(tile_h) * 4.0;
            #[expect(
                clippy::cast_precision_loss,
                reason = "tile_diff is bounded by 255 * tile area; for any tile size \
                          that fits in memory this is far below f64's exact range"
            )]
            let tile_mean = tile_diff as f64 / tile_channels;
            if tile_mean > worst_tile_error {
                worst_tile_error = tile_mean;
                worst_tile_at = (tile_x, tile_y);
            }
        }
    }

    let channels = f64::from(width) * f64::from(height) * 4.0;
    #[expect(
        clippy::cast_precision_loss,
        reason = "counters are bounded by 255 * channel count; an image large enough to \
                  lose precision here could not be allocated"
    )]
    let (mean_error, differing_fraction) =
        (total_diff as f64 / channels, differing as f64 / channels);

    Ok(Comparison {
        mean_error,
        max_error,
        worst_tile_error,
        worst_tile_at,
        differing_fraction,
    })
}

/// Reasons two rasters cannot be compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CompareError {
    /// The rasters have different dimensions.
    #[error("dimension mismatch: {}x{} vs {}x{}", left.0, left.1, right.0, right.1)]
    DimensionMismatch {
        /// Dimensions of the first raster.
        left: (u32, u32),
        /// Dimensions of the second raster.
        right: (u32, u32),
    },
    /// One or both rasters are in an unsupported pixel format.
    #[error("both rasters must be RGBA8")]
    FormatMismatch,
}

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "these assert an exact zero difference, which is the property under test"
)]
mod tests {
    use super::{CompareError, compare, compare_with_tile};
    use pdf_render::{Raster, RasterFormat};

    fn solid(width: u32, height: u32, rgba: [u8; 4]) -> Raster {
        Raster {
            width,
            height,
            format: RasterFormat::Rgba8,
            data: rgba
                .iter()
                .copied()
                .cycle()
                .take(
                    (width as usize)
                        .saturating_mul(height as usize)
                        .saturating_mul(4),
                )
                .collect(),
        }
    }

    #[test]
    fn identical_rasters_compare_equal() {
        let a = solid(64, 64, [10, 20, 30, 255]);
        let result = compare(&a, &a).expect("same size and format");
        assert_eq!(result.mean_error, 0.0);
        assert_eq!(result.max_error, 0);
        assert_eq!(result.worst_tile_error, 0.0);
        assert_eq!(result.differing_fraction, 0.0);
    }

    #[test]
    fn dimension_mismatch_is_its_own_error() {
        let err = compare(&solid(10, 10, [0; 4]), &solid(10, 11, [0; 4])).unwrap_err();
        assert!(matches!(err, CompareError::DimensionMismatch { .. }));
    }

    /// The property the whole metric design rests on: a small, severe, localised
    /// difference must be visible in the worst tile even when the mean buries it.
    #[test]
    fn a_localised_defect_shows_in_the_worst_tile_but_not_the_mean() {
        let a = solid(256, 256, [255, 255, 255, 255]);
        let mut b = a.clone();

        // Blacken one 32x32 tile: 1/64th of the image, entirely wrong.
        for y in 64_usize..96 {
            for x in 32_usize..64 {
                let index = (y * 256 + x) * 4;
                b.data[index..index + 3].fill(0);
            }
        }

        let result = compare_with_tile(&a, &b, 32).expect("same size");

        // The mean is diluted to roughly 255 * (3/4 channels) / 64 tiles ~= 3.
        assert!(result.mean_error < 5.0, "mean was {}", result.mean_error);
        // The worst tile is not diluted at all.
        assert!(
            result.worst_tile_error > 150.0,
            "worst tile was {}",
            result.worst_tile_error
        );
        assert_eq!(result.worst_tile_at, (32, 64));
    }

    #[test]
    fn noise_below_the_just_noticeable_threshold_is_not_counted_as_differing() {
        let a = solid(32, 32, [100, 100, 100, 255]);
        let b = solid(32, 32, [103, 103, 103, 255]);
        let result = compare(&a, &b).expect("same size");
        assert!(result.mean_error > 0.0, "the difference is still measured");
        assert_eq!(
            result.differing_fraction, 0.0,
            "but 3 levels is not 'differing'"
        );
    }
}
