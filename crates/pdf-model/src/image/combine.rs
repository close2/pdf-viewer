//! A mask's samples multiplied into an image's opacity, on the finer of the two grids (ISO
//! 32000-2 §8.9.6.3, §11.6.5.2); shared by an explicit mask and a soft mask.

use std::sync::Arc;

use pdf_render::Image;
use rayon::iter::IndexedParallelIterator as _;
use rayon::iter::ParallelIterator as _;
use rayon::slice::ParallelSlice as _;
use rayon::slice::ParallelSliceMut as _;

use super::convert::band_pixels;

/// A mask's samples as [`combine_on_the_finer_grid`] reads them: `stride` bytes a sample, row by
/// row on a `width` × `height` grid.
///
/// Four bytes for a decoded raster — an explicit mask's stencil, a soft mask resolved at device
/// scale — and one for a [`MaskPlane`], which is what a soft mask decoded eagerly is held as.
///
/// [`MaskPlane`]: super::soft_mask::MaskPlane
#[derive(Clone, Copy)]
pub(super) struct MaskSamples<'a> {
    /// Samples per row.
    pub(super) width: u32,
    /// Rows.
    pub(super) height: u32,
    /// The samples.
    pub(super) data: &'a [u8],
    /// Bytes a sample.
    pub(super) stride: usize,
}

impl<'a> MaskSamples<'a> {
    /// A decoded RGBA raster's samples, four bytes each.
    pub(super) fn of_raster(raster: &'a Image) -> Self {
        Self {
            width: raster.width,
            height: raster.height,
            data: &raster.data,
            stride: 4,
        }
    }
}

/// Combines an image with a mask that need not share its resolution, on the finer grid.
///
/// `sample` is asked of each base colour and the bytes of the mask sample above it — four of a
/// decoded raster's, one of a [`MaskPlane`]'s — for
/// the colour to write and the opacity to scale the base pixel's own alpha by. Both of the
/// standard's per-image masks reduce to an opacity: a stencil (§8.9.6.3) answers all or
/// nothing, a soft-mask image (§11.6.5.2) answers its grey level. Multiplying rather than
/// replacing is §11.3.7's `α = shape × opacity` read through §11.6.4.1, which names the two as
/// separate sources; it also means the answer does not depend on which mask is applied first.
/// The colour passes through unchanged except where §11.6.5.2's `/Matte` has to be undone,
/// which is the one thing a mask says about a base *colour* rather than about its alpha.
///
/// # Which grid the two are combined on
///
/// §8.9.6.3, of an explicit mask:
///
/// > The base image and the image mask need not have the same resolution ( Width and Height
/// > values), but since all images shall be defined on the unit square in user space, their
/// > boundaries on the page will coincide; that is, they will overlay each other.
///
/// §11.6.5.2 Table 143 says it of a soft mask's `/Width` in different words — "independent of
/// it. Both images shall be mapped to the unit square in user space (as are all images),
/// regardless of whether the samples coincide individually" — and both leave the sampling to
/// the device. Correctly, the two are combined at *output* resolution, which this crate does
/// not know: it holds one raster per image and hands it to a rasteriser that may draw it at
/// any scale. The choice made here is the finer of the two grids in each axis, sampled
/// nearest-neighbour, and its merit is that it discards nothing either raster carries:
/// `issue4246.pdf` masks a 50×40 image with a 1000×800 stencil, and combining on the image's
/// grid would throw away 399 of every 400 mask samples; `smaskdim.pdf` gives a 2×2 image a
/// 76×102 soft mask, where the image's grid would leave four samples of a rounded rectangle.
/// What it costs is memory, which [`combined_grid`] bounds, and the difference from a
/// device-resolution composite where the page is drawn larger than both.
///
/// Nearest-neighbour is a choice for the stencil and a compromise for the soft mask. A
/// stencil's samples are two values with no meaningful average, and §8.9.5.3 leaves a
/// magnified image unsmoothed unless `/Interpolate` asks otherwise, which is the same answer
/// the base image gets. A soft mask carries continuous values, so its magnified edges are
/// stepped where a device-resolution composite would have interpolated them — visible only
/// where the mask is *coarser* than the grid, which is the case this function is not asked
/// about: it is the image that gets magnified in every corpus pair.
///
/// [`MaskPlane`]: super::soft_mask::MaskPlane
/// [`combined_grid`]: super::mask::combined_grid
pub(super) fn combine_on_the_finer_grid(
    image: &Image,
    mask: MaskSamples<'_>,
    sample: impl Fn([u8; 3], &[u8]) -> ([u8; 3], u8),
) -> Image {
    let width = image.width.max(mask.width);
    let height = image.height.max(mask.height);
    let (width_usize, height_usize) = (width as usize, height as usize);
    let pixel_row = width_usize.saturating_mul(4);
    let mut data = vec![0u8; pixel_row.saturating_mul(height_usize)];

    // `scale` maps a column or row of the combined grid onto one of a source grid. Integer
    // arithmetic throughout: both grids are at least one sample and the divisor is the
    // combined extent, so nothing here can divide by zero or leave the source's range.
    let scale = |along: usize, source: u32, combined: usize| {
        along
            .saturating_mul(source as usize)
            .checked_div(combined)
            .unwrap_or(0)
    };
    // **A column's source is a function of the column alone**, so it is asked once per column
    // rather than once per pixel: the two divisions `scale` makes were most of what this loop
    // cost on a photograph under its soft mask (ADR 1433). The answers are the same numbers.
    let image_columns: Vec<usize> = (0..width_usize)
        .map(|x| scale(x, image.width, width_usize))
        .collect();
    let mask_columns: Vec<usize> = (0..width_usize)
        .map(|x| scale(x, mask.width, width_usize))
        .collect();
    let combine = |pixel: &[u8], above: &[u8], out: &mut [u8]| {
        let colour = [
            pixel.first().copied().unwrap_or(0),
            pixel.get(1).copied().unwrap_or(0),
            pixel.get(2).copied().unwrap_or(0),
        ];
        let (colour, opacity) = sample(colour, above);
        let own = u16::from(pixel.get(3).copied().unwrap_or(0));
        // Rounded rather than truncated, so an opaque pixel under a fully opaque mask
        // stays opaque: 255 × 255 / 255 is exact either way, but 255 × 254 / 255 is not.
        let combined = own
            .saturating_mul(u16::from(opacity))
            .saturating_add(127)
            .checked_div(255)
            .unwrap_or(0);
        out.copy_from_slice(&[
            colour[0],
            colour[1],
            colour[2],
            u8::try_from(combined).unwrap_or(u8::MAX),
        ]);
    };

    let pixels = width_usize.saturating_mul(height_usize);
    let stride = mask.stride.max(1);
    let one_grid = image.width == width
        && image.height == height
        && image.data.len() == data.len()
        && mask.width == width
        && mask.height == height
        && mask.data.len() == pixels.saturating_mul(stride);
    if one_grid {
        // **Both rasters on the combined grid**, which is every corpus pair this function
        // meets under a soft mask: `scale` is then the identity on every row and column, so the
        // pixels pair by position and the walk is three iterators in step (ADR 1433).
        for ((out, pixel), above) in data
            .chunks_exact_mut(4)
            .zip(image.data.chunks_exact(4))
            .zip(mask.data.chunks_exact(stride))
        {
            combine(pixel, above, out);
        }
    } else if pixel_row != 0 {
        for (y, out_row) in data.chunks_exact_mut(pixel_row).enumerate() {
            let image_row =
                scale(y, image.height, height_usize).saturating_mul(image.width as usize);
            let mask_row = scale(y, mask.height, height_usize).saturating_mul(mask.width as usize);
            for ((out, &image_x), &mask_x) in out_row
                .chunks_exact_mut(4)
                .zip(&image_columns)
                .zip(&mask_columns)
            {
                let at = image_row.saturating_add(image_x).saturating_mul(4);
                let mask_at = mask_row.saturating_add(mask_x).saturating_mul(stride);
                let pixel = image
                    .data
                    .get(at..at.saturating_add(4))
                    .unwrap_or(&[0, 0, 0, 0]);
                let above = mask
                    .data
                    .get(mask_at..mask_at.saturating_add(stride))
                    .unwrap_or(&[0, 0, 0, 0][..stride.min(4)]);
                combine(pixel, above, out);
            }
        }
    }

    Image {
        width,
        height,
        data: Arc::from(data.as_slice()),
        interpolate: image.interpolate,
        // The base's, and the caller's to correct: §8.9.6.3's explicit mask leaves the alpha
        // §11.6.4.2's shape, where §11.6.5.2's soft mask makes it §11.6.4.3's opacity, and
        // this function is the same arithmetic for both. `decode_parts` states which it asked
        // for, on the raster it hands on.
        sample_alpha: image.sample_alpha,
    }
}

/// §11.6.5.2's opacity multiplied into a raster that is the only holder of its samples, where the
/// mask is on the raster's own grid; `false`, and nothing touched, otherwise.
///
/// The same arithmetic [`apply_soft_mask`] asks of [`combine_on_the_finer_grid`] — the colour
/// kept, the alpha the rounded product of the raster's own and the mask's sample — on the pair
/// that function's paired walk takes, which is every corpus pair under a soft mask. What it
/// does not do is build a second raster: [`decode_parts`] has just made this one and nothing
/// else holds it, so the alpha bytes are rewritten where they are, and a large raster's rows
/// are divided across the pool, since no pixel reads another (ADR 1469). Measured alone, pinned:
/// `issue13931.pdf`'s turn 66.8 → 53.1 ms and `images.pdf`'s 51.3 → 40.7; `apply_soft_mask`
/// 548 M → 170 M instructions on `22060_A1_01_Plans.pdf`.
///
/// [`apply_soft_mask`]: super::soft_mask::apply_soft_mask
/// [`decode_parts`]: super::decode_parts
pub(super) fn opacity_multiplied_in_place(image: &mut Image, mask: MaskSamples<'_>) -> bool {
    let pixels = (image.width as usize).saturating_mul(image.height as usize);
    if mask.stride != 1
        || image.width != mask.width
        || image.height != mask.height
        || image.data.len() != pixels.saturating_mul(4)
        || mask.data.len() != pixels
    {
        return false;
    }
    let Some(data) = Arc::get_mut(&mut image.data) else {
        return false;
    };
    let multiply = |pixels: &mut [[u8; 4]], above: &[u8]| {
        for (pixel, &above) in pixels.iter_mut().zip(above) {
            // Read and written as one word, alpha in the top byte, so that the loop is the same
            // arithmetic on every lane of a vector; the colour bytes pass through untouched.
            let word = u32::from_le_bytes(*pixel);
            // [`combine_on_the_finer_grid`]'s rounding, so a fully opaque pair stays opaque; at
            // most 255 × 255 + 127, so nothing wraps.
            let product = (word >> 24)
                .wrapping_mul(u32::from(above))
                .wrapping_add(127)
                / 255;
            *pixel = ((word & 0x00FF_FFFF) | (product << 24)).to_le_bytes();
        }
    };
    let (pixels, _) = data.as_chunks_mut::<4>();
    let above = mask.data;
    match band_pixels(pixels.len()) {
        Some(band) => pixels
            .par_chunks_mut(band)
            .zip(above.par_chunks(band))
            .for_each(|(pixels, above)| multiply(pixels, above)),
        None => multiply(pixels, above),
    }
    true
}

/// A mask's samples carried from the grid they are on to another one, each target cell the mean
/// of the samples its footprint covers.
///
/// What it is for is §11.6.5.2's inversion on a parent that did not decode on the grid the
/// dictionary states: §7.4.9 NOTE 3 lets a `JPXDecode` codestream over the worker's budget come
/// back as "a lower-resolution version" (ADR 0321), and Table 143's pairing of the two grids is
/// then carried onto that one — the mask's own samples, the file's, reduced over the footprint
/// each reduced parent sample covers. Pre-blending is linear in the colour, so the parent's
/// reduced sample is `m + mean(α·(c − m))` and dividing its departure from the matte by
/// `mean(α)` gives the opacity-weighted mean colour of the footprint: what the full grid,
/// undone and then reduced as premultiplied samples are, would have drawn there. It is exact
/// where the codestream's reduction is the footprint's mean and within its own filter's
/// departure from one elsewhere, which is the approximation the reduced level already is
/// (ADR 1324). Borrowing is not offered: the grids are equal on every other route, and there
/// this returns the samples unchanged.
pub(super) fn alpha_on_grid(alpha: &[u8], from: (u32, u32), to: (u32, u32)) -> Vec<u8> {
    if from == to || from.0 == 0 || from.1 == 0 || to.0 == 0 || to.1 == 0 {
        return alpha.to_vec();
    }
    let (from_w, from_h) = (u64::from(from.0), u64::from(from.1));
    let (to_w, to_h) = (u64::from(to.0), u64::from(to.1));
    // The source samples a target cell's footprint covers along one axis: `[t·F/T, (t+1)·F/T)`,
    // rounded outward so that every cell reads at least one sample on a magnification too. Every
    // product is of two grid sides, each a `u32`, so none reaches a `u64`'s range.
    let span = |cell: u64, source: u64, target: u64| {
        let start = cell
            .saturating_mul(source)
            .checked_div(target)
            .unwrap_or(0)
            .min(source.saturating_sub(1));
        let end = cell
            .saturating_add(1)
            .saturating_mul(source)
            .div_ceil(target)
            .clamp(start.saturating_add(1), source);
        (start, end)
    };
    let mut out = Vec::with_capacity(usize::try_from(to_w.saturating_mul(to_h)).unwrap_or(0));
    for row in 0..to_h {
        let (y0, y1) = span(row, from_h, to_h);
        for column in 0..to_w {
            let (x0, x1) = span(column, from_w, to_w);
            let sum: u64 = (y0..y1)
                .flat_map(|y| (x0..x1).map(move |x| y.saturating_mul(from_w).saturating_add(x)))
                .map(|at| {
                    usize::try_from(at)
                        .ok()
                        .and_then(|at| alpha.get(at))
                        .copied()
                        .map_or(0, u64::from)
                })
                .sum();
            let count = y1
                .saturating_sub(y0)
                .saturating_mul(x1.saturating_sub(x0))
                .max(1);
            // The mean of bytes is a byte; rounded to nearest, as §10.7.4's area averaging is.
            let mean = sum
                .saturating_add(count / 2)
                .checked_div(count)
                .unwrap_or(0);
            out.push(u8::try_from(mean).unwrap_or(u8::MAX));
        }
    }
    out
}
