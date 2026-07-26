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
//!
//! # The metric that answers a different question
//!
//! Both of those measure *how far apart* pixels are, and that quantity has a noise floor
//! set by antialiasing which, on text, rises above the signal.
//! [`Comparison::structural_similarity`] measures whether the two images have the same
//! structure instead, which is nearly unaffected by how heavily an edge is antialiased and
//! collapses when something is the wrong shape or in the wrong place. See [`ssim`].

#![forbid(unsafe_code)]

mod ssim;

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
    /// Mean structural similarity over the image, in `-1.0..=1.0`, where 1.0 is identical.
    ///
    /// Insensitive to the antialiasing and hinting differences that dominate every metric
    /// above, and sensitive to geometry being absent, displaced or the wrong shape. This is
    /// what makes a meaningful gate on text pages possible at all.
    pub structural_similarity: f64,
    /// The lowest per-tile mean structural similarity.
    ///
    /// Stands to [`Self::structural_similarity`] as [`Self::worst_tile_error`] stands to
    /// [`Self::mean_error`]: one missing glyph on a dense page barely moves the mean, and
    /// takes its own tile close to zero.
    pub worst_tile_similarity: f64,
    /// Where the least similar tile begins, in pixels from the top-left.
    pub worst_tile_similarity_at: (u32, u32),
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
    let similarity = ssim::map(left, right);
    let mut total_similarity = 0.0f64;
    let mut worst_tile_similarity = f64::INFINITY;
    let mut worst_tile_similarity_at = (0, 0);

    for tile_y in (0..height).step_by(tile as usize) {
        for tile_x in (0..width).step_by(tile as usize) {
            let tile_w = tile.min(width - tile_x);
            let tile_h = tile.min(height - tile_y);

            let mut tile_diff = 0u64;
            let mut tile_similarity = 0.0f64;
            for y in tile_y..tile_y + tile_h {
                for x in tile_x..tile_x + tile_w {
                    let pixel = (y as usize) * (width as usize) + (x as usize);
                    tile_similarity += f64::from(similarity.get(pixel).copied().unwrap_or(1.0));
                    let index = pixel * 4;
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
            total_similarity += tile_similarity;
            let tile_mean_similarity = tile_similarity / (f64::from(tile_w) * f64::from(tile_h));
            if tile_mean_similarity < worst_tile_similarity {
                worst_tile_similarity = tile_mean_similarity;
                worst_tile_similarity_at = (tile_x, tile_y);
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

    let pixels = f64::from(width) * f64::from(height);
    Ok(Comparison {
        mean_error,
        max_error,
        worst_tile_error,
        worst_tile_at,
        differing_fraction,
        // An empty raster is vacuously identical to another empty one.
        structural_similarity: if pixels > 0.0 {
            total_similarity / pixels
        } else {
            1.0
        },
        worst_tile_similarity: if worst_tile_similarity.is_finite() {
            worst_tile_similarity
        } else {
            1.0
        },
        worst_tile_similarity_at,
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
        assert!(result.structural_similarity > 0.999_9);
        assert!(result.worst_tile_similarity > 0.999_9);
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

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code over fixtures whose dimensions are written three lines above the \
              index that reads them"
)]
mod structural_tests {
    use super::{compare, ssim};
    use pdf_render::{Raster, RasterFormat};

    /// A page of vertical stripes `bar` pixels wide, alternating two grey levels.
    ///
    /// Sixteen pixels by default, which is several times the eleven-pixel window the index
    /// uses. Features narrower than the window are a legitimate thing to render, but they
    /// make a poor fixture: the index would then be measuring the window's own response
    /// rather than the difference between the two images.
    fn stripes(size: u32, bar: u32, dark: u8, light: u8) -> Raster {
        let mut data = Vec::with_capacity((size as usize) * (size as usize) * 4);
        for _ in 0..size {
            for x in 0..size {
                let value = if (x / bar).is_multiple_of(2) {
                    dark
                } else {
                    light
                };
                data.extend_from_slice(&[value, value, value, 255]);
            }
        }
        Raster {
            width: size,
            height: size,
            format: RasterFormat::Rgba8,
            data,
        }
    }

    /// Overwrites a column range with one grey level.
    fn paint_columns(raster: &mut Raster, columns: std::ops::Range<u32>, value: u8) {
        for y in 0..raster.height {
            for x in columns.clone() {
                let at = ((y * raster.width + x) as usize) * 4;
                raster.data[at] = value;
                raster.data[at + 1] = value;
                raster.data[at + 2] = value;
            }
        }
    }

    /// The formula itself, on the one input whose answer can be written down.
    ///
    /// Two uniform images have no variance and no covariance, so every term involving them
    /// cancels and the index reduces to `(2ab + C1) / (a² + b² + C1)`. For greys of 100 and
    /// 140 that is `0.945_958` exactly, independent of image size, window and everything else
    /// this module does. A test that pins the whole pipeline against one hand-computed
    /// number is worth more than several that only compare it against itself.
    #[test]
    fn two_uniform_images_give_the_index_its_closed_form() {
        let a = Raster {
            width: 32,
            height: 32,
            format: RasterFormat::Rgba8,
            data: [100, 100, 100, 255].repeat(32 * 32),
        };
        let b = Raster {
            data: [140, 140, 140, 255].repeat(32 * 32),
            ..a.clone()
        };

        let result = compare(&a, &b).expect("same size and format");
        assert!(
            (result.structural_similarity - 0.945_958).abs() < 1e-4,
            "expected the closed form 0.945958, got {}",
            result.structural_similarity
        );
    }

    /// The separable window must give the same answer as the window it stands in for.
    ///
    /// The Gaussian is applied as two one-dimensional passes because 11×11 taps per pixel,
    /// five times over, is the difference between a comparison harness that runs on every
    /// page and one that does not. Separability is exact in theory; this checks that it is
    /// exact in the implementation, which is the only claim the optimisation rests on.
    #[test]
    fn the_separable_window_matches_the_two_dimensional_one() {
        let a = stripes(48, 7, 20, 230);
        let mut b = a.clone();
        paint_columns(&mut b, 10..14, 120);

        let fast = ssim::map(&a, &b);
        let slow = ssim::reference_map(&a, &b);
        assert_eq!(fast.len(), slow.len());
        for (index, (quick, direct)) in fast.iter().zip(&slow).enumerate() {
            assert!(
                (quick - direct).abs() < 1e-4,
                "pixel {index}: separable {quick} against direct {direct}"
            );
        }
    }

    /// The property that justifies the metric existing, tested fairly.
    ///
    /// Two perturbations of the same page, constructed to be *identical* to the pixel
    /// metrics: the same number of pixels, moved by the same amount, in the same colour.
    /// One shifts the boundary column of each bar, which is what two correct rasterisers do
    /// differently. The other punches the same columns out of the middle of each bar, which
    /// is a hole that should not be there.
    ///
    /// Mean error, worst tile and differing fraction cannot tell these apart — they are
    /// equal to the last decimal, because the metrics only count how far pixels moved. The
    /// structural index is what distinguishes an edge that landed differently from a
    /// feature that is not there.
    ///
    /// The separation on this fixture is 0.824 against 0.750 — real, and smaller than the
    /// metric's reputation suggests, because a 32-pixel tile averages the affected columns
    /// against a great deal of untouched page. It is recorded here as the *floor* of what
    /// the index buys on synthetic content; what it buys on real pages, where the noise is
    /// dense rather than confined to one column per bar, is a separate measurement and
    /// belongs with the tolerances that use it.
    #[test]
    fn an_edge_shifting_and_a_hole_are_the_same_to_the_pixel_metrics() {
        let page = stripes(128, 16, 0, 255);

        // The boundary column of every dark bar, lightened.
        let mut shifted = page.clone();
        for bar in (0..128).step_by(32) {
            paint_columns(&mut shifted, bar..bar + 1, 128);
        }
        // The same count of columns, the same colour, in the middle of every dark bar.
        let mut holed = page.clone();
        for bar in (0..128).step_by(32) {
            paint_columns(&mut holed, bar + 8..bar + 9, 128);
        }

        let edge = compare(&page, &shifted).expect("same size and format");
        let hole = compare(&page, &holed).expect("same size and format");

        assert_eq!(
            (
                edge.mean_error,
                edge.worst_tile_error,
                edge.differing_fraction
            ),
            (
                hole.mean_error,
                hole.worst_tile_error,
                hole.differing_fraction
            ),
            "the fixtures are built to be indistinguishable to the pixel metrics"
        );
        assert!(
            edge.worst_tile_similarity > hole.worst_tile_similarity + 0.05,
            "the structural index must tell them apart: edge {} against hole {}",
            edge.worst_tile_similarity,
            hole.worst_tile_similarity
        );
    }

    /// The complementary property: a defect the pixel metrics struggle with must be found.
    ///
    /// One stripe removed from a page of stripes — the missing-glyph case, which moves the
    /// whole-page mean by a fraction of a percent. The tile containing it must come out
    /// structurally dissimilar rather than merely different in value.
    #[test]
    fn a_missing_stripe_collapses_the_similarity_of_its_tile() {
        let full = stripes(128, 16, 0, 255);
        let mut missing = full.clone();
        paint_columns(&mut missing, 32..48, 255);

        let result = compare(&full, &missing).expect("same size and format");
        assert!(
            result.structural_similarity > 0.8,
            "most of the page is untouched: {}",
            result.structural_similarity
        );
        // Against the 0.99 the softened-edge fixture keeps, this is not a near miss: the
        // two cases are separated by most of the metric's range.
        assert!(
            result.worst_tile_similarity < 0.5,
            "the tile that lost a stripe must stand out: {}",
            result.worst_tile_similarity
        );
        assert_eq!(
            result.worst_tile_similarity_at.0, 32,
            "and be located where the stripe was"
        );
    }

    /// A uniform brightness shift is a colour error, not a structural one.
    ///
    /// The two kinds of metric must disagree here, and the disagreement is diagnostic: it
    /// is how a gamma or colour-space mistake is told apart from geometry going wrong.
    /// Asserting on both is what makes the pair informative rather than redundant.
    #[test]
    fn a_uniform_shift_is_seen_by_the_mean_and_not_by_the_structure() {
        // Levels chosen so the shift saturates nothing: saturation would compress the
        // contrast, which *is* a structural change, and the fixture would test nothing.
        let base = stripes(128, 16, 60, 200);
        let mut lifted = base.clone();
        for pixel in lifted.data.chunks_exact_mut(4) {
            for channel in &mut pixel[..3] {
                *channel = channel.saturating_add(40);
            }
        }

        let result = compare(&base, &lifted).expect("same size and format");
        assert!(result.mean_error > 25.0, "{}", result.mean_error);
        assert!(
            result.structural_similarity > 0.85,
            "a shift that preserves every edge is not a structural difference: {}",
            result.structural_similarity
        );
    }
}
