//! A straight stroke whose ink lies in two by two device pixels, painted from its closed-form area
//! without entering `tiny-skia` — ISO 32000-2 §10.7.4 and §11.3.
//!
//! `pdf_render::thin_segment` decides which marks these are and `pdf_render::ThinSegment::cells`
//! states each pixel's area; this module is what the processor does with the four numbers. ADR 1817
//! measured what the general construction spent on such a mark — 1 656 ns, against 31 ns for
//! blending its pixels — and none of it was the area: a stroker's outline, a scan converter's edges
//! and a compositing pipeline built per mark. ADR 1829 is this construction.
//!
//! # Composited the way every other mark is
//!
//! The coverage meets the clip and the soft mask exactly as `scan::intersected` composes a mark it
//! measured itself: §10.7.4's set `C · S` by `min`, §11.6.5's value `S` by a product, with
//! `scan::scaled`'s rounding, so a mark drawn here and the same mark drawn there are one function of
//! their coverage. The paint is then laid over the backdrop by §11.3.6's `Normal` compositing,
//! `cr = (1 − αs) · cb + cs` on premultiplied values, written out in the arithmetic `tiny-skia`'s
//! high-precision pipeline uses for a solid colour drawn through a mask — the coverage as a level
//! divided by 255, the backdrop read as a level times the reciprocal of 255, and the result rounded
//! to the nearest level, ties to even — so that a pixel this module and the library are both handed
//! at one coverage comes out as the same byte (ADR 0418 is why that pipeline is the high-precision
//! one).
//!
//! # The level, and the one place it is lifted
//!
//! A coverage is rounded to the nearest level. A mark none of whose pixels reaches one level is the
//! exception `scan::mask_rectangle` makes for a rectangle, for the same clause and on the same
//! terms (ADR 1374): every pixel it touches is stated at one level, because §10.7.4's purpose is that
//! "no shape ever disappears as a result of unfavourable placement relative to the device pixel
//! grid".

use pdf_render::{BlendMode, Paint, Path, Stroke, Transform};

use crate::{ToDevice, scan};

/// One level of an eight-bit coverage, as a share of a pixel.
const ONE_LEVEL: f64 = 1.0 / 255.0;

/// Paints the stroke `path` under `stroke` in `paint` onto `pixmap` through `clip` where
/// [`pdf_render::thin_segment`] admits it, and returns whether it did. `false`, with nothing drawn,
/// for any other mark, and where the clip's buffers do not match the pixmap — the pairing
/// `scan::intersected` declines too, and which the ordinary construction then answers.
///
/// **The mark is admitted and measured on the whole page's device grid, and only its rows are moved
/// to the surface**, which holds the page's rows from `to_device`'s first. A block's row is a whole
/// number, so subtracting that row from it is exact, and every division of the page admits the same
/// marks and draws the same areas at the same levels into the same page pixels. That is what lets
/// the strip planner cut through such a mark, asking `pdf_render::thin_segment` under the same
/// transform (ADR 1847). Asked on a strip's own grid, the endpoints would round at another
/// magnitude (ADR 0219), and the block — and with it the converter — could change at a cut (trap
/// 146).
pub(crate) fn draw(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    (path, stroke, paint): (&Path, &Stroke, &Paint),
    (transform, to_device): (Transform, ToDevice),
    clip: scan::Clip<'_>,
) -> bool {
    let Paint::Solid(colour) = *paint else {
        return false;
    };
    let Some(thin) = pdf_render::thin_segment(
        path,
        stroke,
        paint,
        BlendMode::Normal,
        transform.then(to_device.page),
    ) else {
        return false;
    };
    let top = to_device.top;
    let (width, height) = (pixmap.width(), pixmap.height());
    let (bound, value) = clip.factors();
    if bound.is_some_and(|bound| bound.width() != width || bound.height() != height) {
        return false;
    }
    let stride = width as usize;
    if value.is_some_and(|value| value.len() != stride.saturating_mul(height as usize)) {
        return false;
    }
    let Some((block, areas)) = thin.cells() else {
        return false;
    };

    // The floor is decided over the whole shape before any pixel is clipped to the raster: a mark
    // is lifted for what it is, not for where a strip cuts it.
    let largest = areas.iter().copied().fold(0.0_f64, f64::max);
    let lifted = largest > 0.0 && largest < ONE_LEVEL;
    let mut cells = [(0.0_f64, 0.0_f64, 0.0_f64); 4];
    let mut count = 0;
    // `cells` lists the block by rows, and within a row by columns, which is this loop's order.
    let mut in_order = areas.iter();
    for row in 0..block.rows {
        for column in 0..block.columns {
            let (x, y) = (
                block.column + f64::from(column),
                block.row + f64::from(row) - f64::from(top),
            );
            if let (Some(cell), Some(&area)) = (cells.get_mut(count), in_order.next()) {
                *cell = (x, y, area);
                count = count.saturating_add(1);
            }
        }
    }

    let premultiplied = crate::convert::color(colour).premultiply();
    let data = pixmap.data_mut();
    for &(x, y, coverage) in cells.iter().take(count) {
        let (Some(x), Some(y)) = (index_of(x, width), index_of(y, height)) else {
            continue;
        };
        let level = if lifted && coverage > 0.0 {
            1
        } else {
            level_of(coverage)
        };
        let at = y.saturating_mul(stride).saturating_add(x);
        let mask = match (bound, value) {
            (None, None) => level,
            (Some(bound), None) => bound.data().get(at).map_or(0, |&b| level.min(b)),
            (None, Some(value)) => value.get(at).map_or(0, |&s| scan::scaled(level, s)),
            (Some(bound), Some(value)) => match (bound.data().get(at), value.get(at)) {
                (Some(&b), Some(&s)) => scan::scaled(level, s).min(b),
                _ => 0,
            },
        };
        if mask == 0 {
            continue;
        }
        let byte = at.saturating_mul(4);
        if let Some(pixel) = data.get_mut(byte..byte.saturating_add(4)) {
            source_over(pixel, premultiplied, mask);
        }
    }
    true
}

/// A pixel index on an axis of `limit` pixels, or `None` off the raster.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is a whole number checked to lie in 0..limit before the cast, and a raster's \
              dimension is far inside usize"
)]
fn index_of(value: f64, limit: u32) -> Option<usize> {
    (value >= 0.0 && value < f64::from(limit)).then_some(value as usize)
}

/// A coverage in `0.0..=1.0` as the level an eight-bit mask holds, rounded to nearest.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is a coverage clamped to 0..=1 multiplied by 255, so the cast is in range by \
              construction"
)]
fn level_of(coverage: f64) -> u8 {
    (coverage.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// §11.3.6's `Normal` compositing of a premultiplied colour at coverage `mask` over one
/// premultiplied pixel, in the high-precision pipeline's own order of operations: the source scaled
/// by `mask / 255`, the backdrop read as `level · (1 / 255)`, `d · (1 − αs) + s` unfused, and the
/// result clamped and rounded to the nearest level, ties to even, as `_mm_cvtps_epi32` does.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to 0..=1 and multiplied by 255 before the cast, so it is in u8's \
              range by construction"
)]
fn source_over(pixel: &mut [u8], colour: tiny_skia::PremultipliedColor, mask: u8) {
    let scale = f32::from(mask) / 255.0;
    let source = [
        colour.red() * scale,
        colour.green() * scale,
        colour.blue() * scale,
        colour.alpha() * scale,
    ];
    let keep = 1.0 - colour.alpha() * scale;
    for (byte, source) in pixel.iter_mut().zip(source) {
        let backdrop = f32::from(*byte) * (1.0 / 255.0);
        let result = backdrop * keep + source;
        *byte = (result.clamp(0.0, 1.0) * 255.0).round_ties_even() as u8;
    }
}
