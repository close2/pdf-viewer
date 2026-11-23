//! A decoded video picture converted to RGB on the processor: the oracle the device's conversion is
//! compared against (RFC 0015 section 6, round 4; ADR 3529).
//!
//! The formula, its numbers and the choices in it are `pdf_render::video`'s; this is that formula
//! computed in `f64` and rounded once, to the nearest level, so that it stands for the exact value
//! a reader of Annex E's equations would compute by hand.

use pdf_render::video::{Conversion, ConversionRefused, Planes, chroma_sixteenths};
use pdf_render::{Raster, RasterFormat};
use rayon::prelude::*;

/// `planes` converted by `conversion` into an opaque straight-alpha RGBA8 raster of the picture's
/// size.
///
/// Rows are converted in parallel: a 1080-line picture is two million independent pixels, and the
/// rows share nothing but the read-only planes. Measured on the development machine's EPYC, one
/// thread takes 114 ms for it and the default pool 12.1 ms (ADR 3529); the cost is the one
/// `par_chunks_mut` line.
///
/// # Errors
///
/// [`ConversionRefused::Planes`] where the planes are not the size's.
pub fn convert_picture(
    planes: &Planes<'_>,
    conversion: &Conversion,
) -> Result<Raster, ConversionRefused> {
    if !planes.are_consistent() {
        return Err(ConversionRefused::Planes {
            width: planes.width,
            height: planes.height,
        });
    }
    let width = planes.width as usize;
    let mut data = vec![
        0u8;
        width
            .saturating_mul(planes.height as usize)
            .saturating_mul(4)
    ];
    data.par_chunks_mut(width.saturating_mul(4))
        .zip(0..planes.height)
        .for_each(|(row, y)| convert_row(planes, conversion, y, row));
    Ok(Raster {
        width: planes.width,
        height: planes.height,
        format: RasterFormat::Rgba8,
        data,
    })
}

/// Row `y` of the picture into `row`, four bytes a pixel.
fn convert_row(planes: &Planes<'_>, conversion: &Conversion, y: u32, row: &mut [u8]) {
    let affine = conversion.affine();
    let (chroma_width, chroma_height) = planes.chroma_size();
    let luma_row = (y as usize).saturating_mul(planes.width as usize);
    for (x, pixel) in (0..planes.width).zip(row.as_chunks_mut::<4>().0) {
        let luma = planes
            .y
            .get(luma_row.saturating_add(x as usize))
            .map_or(0.0, |&sample| f64::from(sample));
        let blue = f64::from(chroma_sixteenths(
            planes.cb,
            chroma_width,
            chroma_height,
            conversion,
            x,
            y,
        )) - 2048.0;
        let red = f64::from(chroma_sixteenths(
            planes.cr,
            chroma_width,
            chroma_height,
            conversion,
            x,
            y,
        )) - 2048.0;
        let base = affine.luma * (luma - affine.luma_offset);
        let [red_level, green_level, blue_level] = [0, 1, 2]
            .map(|channel| level(base + affine.cb[channel] * blue + affine.cr[channel] * red));
        *pixel = [red_level, green_level, blue_level, u8::MAX];
    }
}

/// A value in levels held to `0..=255` and rounded to the nearest, half away from zero.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to 0..=255 before the cast"
)]
fn level(value: f64) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}
