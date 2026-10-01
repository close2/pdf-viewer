//! Structural similarity, for the disagreements a pixel difference cannot classify.
//!
//! # Why a second kind of metric is needed
//!
//! Every metric in this crate so far asks *how far apart* two pixels are. That question
//! has a floor set by antialiasing: two correct rasterisers put different amounts of ink
//! in the pixels along every edge, so a page full of edges reports a large difference
//! whether or not anything is wrong. On text the floor rises above the signal — the
//! disagreement between two references over glyph hinting is larger than the disagreement
//! a genuinely misplaced glyph would produce — which is why [`crate::Comparison`] alone
//! forces a text tolerance so loose it catches almost nothing.
//!
//! Structural similarity asks a different question: whether the two images have the same
//! *structure*, by comparing local mean, local variance and local covariance rather than
//! values. Two renderings of the same glyph, one with heavier antialiasing, have nearly
//! the same structure — the edge is in the same place and the contrast across it has the
//! same sign and similar magnitude — so SSIM stays near 1. A glyph that is absent, in the
//! wrong place, or the wrong shape destroys the local covariance and SSIM collapses.
//!
//! That is the separation the pixel metrics cannot make, and it is what lets a text page
//! be gated on something better than "not blank".
//!
//! # What is implemented
//!
//! The index of Wang, Bovik, Sheikh and Simoncelli (2004), in its standard form: an 11×11
//! Gaussian window of σ = 1.5, stabilising constants `C1 = (0.01 L)²` and `C2 = (0.03 L)²`
//! at `L = 255`, evaluated on luminance. The window is separable, so the five statistics
//! it needs cost two one-dimensional passes each rather than 121 multiplications per pixel.
//!
//! Luminance is Rec. 709 and scaled by alpha, so a region opaque in one rendering and
//! transparent in the other differs structurally rather than only in its alpha channel.

use pdf_render::Raster;

/// Radius of the Gaussian window, giving the standard 11×11 support.
const RADIUS: usize = 5;

/// Standard deviation of the Gaussian window, in pixels.
const SIGMA: f32 = 1.5;

/// Dynamic range of the luminance signal, which sets the stabilising constants.
const RANGE: f32 = 255.0;

/// Stabilises the luminance term where both local means are near zero.
const C1: f32 = (0.01 * RANGE) * (0.01 * RANGE);

/// Stabilises the contrast and structure terms where both local variances are near zero.
const C2: f32 = (0.03 * RANGE) * (0.03 * RANGE);

/// The per-pixel map of structural similarity between two rasters, in `-1.0..=1.0`, produced a
/// band of rows at a time.
///
/// Both rasters must share dimensions and be `Rgba8`; callers reach this through
/// [`crate::compare_with_tile`], which checks. [`Self::rows`] gives the map's entries for one
/// band of rows, in the same row-major order as the rasters.
///
/// **Banded because the whole map costs nine planes of `f32` per pixel.** Two luminance planes,
/// the five blurred statistics, the blur's own intermediate and the map itself are each four bytes
/// a pixel, so the page of 12 608 × 16 806 pixels the corpus gate compares at its own scale asked
/// 7.4 GB of the walk for its comparison alone, against 0.85 GB for each of the two rasters (ADR
/// 1481). A band holds the same planes for its own rows plus the window's [`RADIUS`] rows above
/// and below it, and every entry is computed by the same operations in the same order as over the
/// whole raster: a horizontal pass reads only its own row, and the vertical pass reads rows that
/// clamp into the band's margin exactly as they clamp into the raster. So the map is the same to
/// the bit, which `a_banded_map_is_the_whole_map` checks across band boundaries.
///
/// The window clamps at the raster's edges rather than cropping, which keeps the map the same
/// size as the image so that a per-tile mean needs no special case at the margins; evaluating
/// the index only where the full window fits, as the original paper does, would silently
/// exclude the outermost five pixels of every page from the gate.
pub(crate) struct Bands<'a> {
    left: &'a Raster,
    right: &'a Raster,
    width: usize,
    height: usize,
    kernel: [f32; (RADIUS * 2) + 1],
    /// Luminance of the band's rows and its margins, left then right.
    x: Vec<f32>,
    y: Vec<f32>,
    /// The five statistics after the horizontal pass, over the same rows.
    horizontal: [Vec<f32>; 5],
    /// The map's entries for the band's own rows.
    out: Vec<f32>,
    /// Which rows [`Self::out`] holds.
    held: std::ops::Range<usize>,
}

/// The fewest rows a band is computed for, whatever the caller asks.
///
/// Each band recomputes the window's margin, [`RADIUS`] rows above and below it, so a band of a
/// comparison tile's 32 rows did 31% more horizontal work than the whole map and measured 17%
/// slower on the corpus's largest page; at 256 rows the margin is 4% of the band, and the band's
/// planes are a few megabytes on any page this tree compares (ADR 1481).
const BAND_ROWS: usize = 256;

impl<'a> Bands<'a> {
    /// A producer of the map of `left` against `right`.
    pub(crate) fn new(left: &'a Raster, right: &'a Raster) -> Self {
        Self {
            left,
            right,
            width: left.width as usize,
            height: left.height as usize,
            kernel: gaussian(),
            x: Vec::new(),
            y: Vec::new(),
            horizontal: Default::default(),
            out: Vec::new(),
            held: 0..0,
        }
    }

    /// The map's entries for rows `top..bottom`, clamped to the raster, row-major — computed
    /// [`BAND_ROWS`] at a time and handed out from the band held while the rows asked lie in it.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`top` and `bottom` lie inside the held rows by the test above them, so the \
                  differences are non-negative and the products are offsets into `out`"
    )]
    pub(crate) fn rows(&mut self, top: usize, bottom: usize) -> &[f32] {
        let bottom = bottom.min(self.height);
        if top >= bottom {
            return &[];
        }
        if top < self.held.start || bottom > self.held.end {
            self.compute(
                top,
                bottom.max(top.saturating_add(BAND_ROWS)).min(self.height),
            );
        }
        let width = self.width;
        let start = (top - self.held.start) * width;
        let end = (bottom - self.held.start) * width;
        self.out.get(start..end).unwrap_or(&[])
    }

    /// Exactly rows `top..bottom`, however few: the band boundaries a test needs to reach.
    #[cfg(test)]
    pub(crate) fn band(&mut self, top: usize, bottom: usize) -> &[f32] {
        self.compute(top, bottom.min(self.height));
        &self.out
    }

    /// Computes the map's entries for rows `top..bottom`, which lie inside the raster, into
    /// [`Self::out`].
    #[expect(
        clippy::arithmetic_side_effects,
        clippy::indexing_slicing,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "every index derives from the shared width and height the caller checked, \
                  and a vertical position is clamped into the raster and so into the band's \
                  margin, which reaches RADIUS rows past each edge of the band or to the \
                  raster's own edge; the loop is the hot path of the comparison harness"
    )]
    fn compute(&mut self, top: usize, bottom: usize) {
        let (width, height) = (self.width, self.height);
        self.out.clear();
        self.held = top..bottom;
        if width == 0 {
            return;
        }
        let first = top.saturating_sub(RADIUS);
        let last = (bottom + RADIUS).min(height);
        let span = first * width..last * width;
        luminance_into(self.left, span.clone(), &mut self.x);
        luminance_into(self.right, span, &mut self.y);

        // The five statistics the index is built from. Each is a Gaussian-weighted local
        // average, so each is one separable blur; this is its horizontal pass, row by row.
        let count = self.x.len();
        for plane in &mut self.horizontal {
            plane.clear();
            plane.resize(count, 0.0);
        }
        let [mean_x, mean_y, squares_x, squares_y, cross] = &mut self.horizontal;
        let (x, y) = (&self.x, &self.y);
        for row in (0..count).step_by(width) {
            for column in 0..width {
                let (mut mx, mut my) = (0.0f32, 0.0f32);
                let (mut xx, mut yy, mut xy) = (0.0f32, 0.0f32, 0.0f32);
                for (index, tap) in self.kernel.iter().enumerate() {
                    let at = row
                        + (column as isize + index as isize - RADIUS as isize)
                            .clamp(0, width as isize - 1) as usize;
                    let (a, b) = (x[at], y[at]);
                    mx += *tap * a;
                    my += *tap * b;
                    xx += *tap * (a * a);
                    yy += *tap * (b * b);
                    xy += *tap * (a * b);
                }
                let at = row + column;
                (mean_x[at], mean_y[at]) = (mx, my);
                (squares_x[at], squares_y[at], cross[at]) = (xx, yy, xy);
            }
        }

        // The vertical pass, and the index from the five statistics it leaves.
        for out_row in top..bottom {
            for column in 0..width {
                let (mut mx, mut my) = (0.0f32, 0.0f32);
                let (mut sxx, mut syy, mut sxy) = (0.0f32, 0.0f32, 0.0f32);
                for (index, tap) in self.kernel.iter().enumerate() {
                    let source = (out_row as isize + index as isize - RADIUS as isize)
                        .clamp(0, height as isize - 1) as usize;
                    let at = ((source - first) * width) + column;
                    mx += *tap * mean_x[at];
                    my += *tap * mean_y[at];
                    sxx += *tap * squares_x[at];
                    syy += *tap * squares_y[at];
                    sxy += *tap * cross[at];
                }
                self.out.push(index_of(mx, my, sxx, syy, sxy));
            }
        }
    }
}

/// The index at one pixel from its five blurred statistics.
fn index_of(mx: f32, my: f32, squares_x: f32, squares_y: f32, cross: f32) -> f32 {
    let (mxx, myy) = (mx * mx, my * my);
    // Variance and covariance from the raw second moments. These can come out very
    // slightly negative where a region is flat and the blur rounds, which the index
    // tolerates: the stabilising constants dominate there by construction.
    let var_x = squares_x - mxx;
    let var_y = squares_y - myy;
    let covariance = cross - (mx * my);

    let luminance_term = (2.0 * mx * my) + C1;
    let structure_term = (2.0 * covariance) + C2;
    let denominator = (mxx + myy + C1) * (var_x + var_y + C2);
    if denominator.abs() > f32::EPSILON {
        (luminance_term * structure_term) / denominator
    } else {
        // Both windows are uniform black. Identical, so perfectly similar.
        1.0
    }
}

/// The whole map at once, for testing only: the definition [`Bands`] is checked against to the
/// bit, and itself checked against [`reference_map`]'s two-dimensional window.
///
/// Both rasters must share dimensions and be `Rgba8`; callers reach this through
/// [`crate::compare_with_tile`], which checks. The map has one entry per pixel, in the
/// same row-major order as the rasters.
#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "every index derives from the shared width and height the caller checked, \
              and the loop is the hot path of the comparison harness: bounds-checked \
              accessors here cost more than the statistics they guard"
)]
pub(crate) fn map(left: &Raster, right: &Raster) -> Vec<f32> {
    let width = left.width as usize;
    let height = left.height as usize;
    let count = width.saturating_mul(height);
    if count == 0 {
        return Vec::new();
    }

    let x = luminance(left, count);
    let y = luminance(right, count);

    // The five statistics the index is built from. Each is a Gaussian-weighted local
    // average, so each is one separable blur.
    let kernel = gaussian();
    let mean_x = blur(&x, width, height, &kernel);
    let mean_y = blur(&y, width, height, &kernel);
    let squares_x = blur(&product(&x, &x), width, height, &kernel);
    let squares_y = blur(&product(&y, &y), width, height, &kernel);
    let cross = blur(&product(&x, &y), width, height, &kernel);

    let mut out = Vec::with_capacity(count);
    for index in 0..count {
        let (mx, my) = (mean_x[index], mean_y[index]);
        let (mxx, myy) = (mx * mx, my * my);
        // Variance and covariance from the raw second moments. These can come out very
        // slightly negative where a region is flat and the blur rounds, which the index
        // tolerates: the stabilising constants dominate there by construction.
        let var_x = squares_x[index] - mxx;
        let var_y = squares_y[index] - myy;
        let covariance = cross[index] - (mx * my);

        let luminance_term = (2.0 * mx * my) + C1;
        let structure_term = (2.0 * covariance) + C2;
        let denominator = (mxx + myy + C1) * (var_x + var_y + C2);
        out.push(if denominator.abs() > f32::EPSILON {
            (luminance_term * structure_term) / denominator
        } else {
            // Both windows are uniform black. Identical, so perfectly similar.
            1.0
        });
    }
    out
}

/// Rec. 709 luminance scaled by alpha, in `0.0..=255.0`.
///
/// Alpha participates so that a region drawn in one rendering and left empty in the other
/// registers as a structural difference. Without it a missing shape over a transparent
/// background would show only in the alpha channel, which this index never looks at.
#[expect(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "the pixels are a range of the raster's own, which the caller derived from its \
              dimensions, and `data` is four bytes per pixel by RasterFormat::Rgba8"
)]
fn luminance_into(raster: &Raster, pixels: std::ops::Range<usize>, out: &mut Vec<f32>) {
    out.clear();
    out.reserve(pixels.len());
    for index in pixels {
        let at = index * 4;
        let red = f32::from(raster.data[at]);
        let green = f32::from(raster.data[at + 1]);
        let blue = f32::from(raster.data[at + 2]);
        let alpha = f32::from(raster.data[at + 3]) / 255.0;
        out.push(((0.212_6 * red) + (0.715_2 * green) + (0.072_2 * blue)) * alpha);
    }
}

/// [`luminance_into`] over the first `count` pixels, as a plane of its own.
#[cfg(test)]
fn luminance(raster: &Raster, count: usize) -> Vec<f32> {
    let mut out = Vec::new();
    luminance_into(raster, 0..count, &mut out);
    out
}

/// Elementwise product of two equally long signals.
#[cfg(test)]
fn product(left: &[f32], right: &[f32]) -> Vec<f32> {
    left.iter().zip(right).map(|(a, b)| *a * *b).collect()
}

/// The normalised one-dimensional Gaussian window.
#[expect(
    clippy::cast_precision_loss,
    reason = "eleven taps of a constant kernel; the offsets are small integers"
)]
fn gaussian() -> [f32; (RADIUS * 2) + 1] {
    let mut kernel = [0.0f32; (RADIUS * 2) + 1];
    let mut total = 0.0f32;
    for (index, tap) in kernel.iter_mut().enumerate() {
        let offset = index as f32 - RADIUS as f32;
        *tap = (-(offset * offset) / (2.0 * SIGMA * SIGMA)).exp();
        total += *tap;
    }
    for tap in &mut kernel {
        *tap /= total;
    }
    kernel
}

/// Applies the separable Gaussian window, clamping at the edges.
///
/// Clamping rather than cropping keeps the map the same size as the image, so a per-tile
/// mean needs no special case at the margins. The alternative — evaluating the index only
/// where the full window fits — is what the original paper does, and it would silently
/// exclude the outermost five pixels of every page from the gate.
#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "positions are clamped into 0..width and 0..height before indexing, and the \
              dimensions come from a raster so they fit an isize comfortably"
)]
fn blur(signal: &[f32], width: usize, height: usize, kernel: &[f32]) -> Vec<f32> {
    let mut horizontal = vec![0.0f32; signal.len()];
    for y in 0..height {
        let row = y * width;
        for x in 0..width {
            let mut total = 0.0f32;
            for (index, tap) in kernel.iter().enumerate() {
                let at = (x as isize + index as isize - RADIUS as isize)
                    .clamp(0, width as isize - 1) as usize;
                total += *tap * signal[row + at];
            }
            horizontal[row + x] = total;
        }
    }

    let mut out = vec![0.0f32; signal.len()];
    for y in 0..height {
        for x in 0..width {
            let mut total = 0.0f32;
            for (index, tap) in kernel.iter().enumerate() {
                let at = (y as isize + index as isize - RADIUS as isize)
                    .clamp(0, height as isize - 1) as usize;
                total += *tap * horizontal[(at * width) + x];
            }
            out[(y * width) + x] = total;
        }
    }
    out
}

/// The same index computed with the full two-dimensional window, for testing only.
///
/// [`map`] applies the window as two one-dimensional passes, which is mathematically the
/// same thing and about ten times less work. This is the definition that claim is checked
/// against — kept beside the implementation rather than in the test file so the two cannot
/// drift apart, and behind `cfg(test)` so it never reaches a build.
#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "test-only reference implementation, written for transparency rather than \
              for speed or defensiveness; every index is clamped into the raster"
)]
pub(crate) fn reference_map(left: &Raster, right: &Raster) -> Vec<f32> {
    let width = left.width as usize;
    let height = left.height as usize;
    let count = width * height;
    let x = luminance(left, count);
    let y = luminance(right, count);
    let kernel = gaussian();

    let mut out = Vec::with_capacity(count);
    for row in 0..height {
        for column in 0..width {
            let (mut mx, mut my) = (0.0f32, 0.0f32);
            let (mut xx, mut yy, mut xy) = (0.0f32, 0.0f32, 0.0f32);
            for (j, vertical) in kernel.iter().enumerate() {
                let at_y =
                    (row as isize + j as isize - RADIUS as isize).clamp(0, height as isize - 1);
                for (i, horizontal) in kernel.iter().enumerate() {
                    let at_x = (column as isize + i as isize - RADIUS as isize)
                        .clamp(0, width as isize - 1);
                    let at = (at_y as usize * width) + at_x as usize;
                    let weight = vertical * horizontal;
                    mx += weight * x[at];
                    my += weight * y[at];
                    xx += weight * x[at] * x[at];
                    yy += weight * y[at] * y[at];
                    xy += weight * x[at] * y[at];
                }
            }
            let numerator = ((2.0 * mx * my) + C1) * ((2.0 * (xy - (mx * my))) + C2);
            let denominator =
                ((mx * mx) + (my * my) + C1) * ((xx - (mx * mx)) + (yy - (my * my)) + C2);
            out.push(if denominator.abs() > f32::EPSILON {
                numerator / denominator
            } else {
                1.0
            });
        }
    }
    out
}
