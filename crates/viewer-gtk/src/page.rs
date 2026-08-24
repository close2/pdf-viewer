//! Tier 1: the raster the viewer is holding, turned into something GSK can put on the screen.
//!
//! `doc/ui-boundary.md` prices the tier at "one copy per frame", and this module is that copy.
//! [`pdf_render::Raster`] is row-major RGBA with straight alpha and no row padding, which is
//! exactly [`gdk::MemoryFormat::R8g8b8a8`] — so what crosses is a `memcpy` into a
//! [`glib::Bytes`] and no conversion at all. GSK uploads it to the graphics device on its own
//! schedule, which is the part of a native host that is the toolkit's business rather than ours.

use gtk4::gdk;
use gtk4::glib;
use pdf_render::{Image, Raster, RasterFormat};

/// Why a raster could not become a texture.
///
/// Both are conditions GDK's own interface imposes and neither can be reached by any page this
/// program draws today — a raster is bounded by `pdf_render::MAX_EXTENT`, which is 2^24. They are
/// typed and reported rather than asserted away, because the alternative to a report here is a
/// window that shows the previous page and says nothing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum PixelError {
    /// GDK measures a texture in `i32`, and this raster does not fit.
    #[error("a raster of {width}x{height} exceeds GDK's 32-bit texture dimensions")]
    TooLarge {
        /// The raster's width in pixels.
        width: u32,
        /// Its height.
        height: u32,
    },
    /// The raster holds fewer bytes than its own dimensions call for.
    #[error("a raster of {width}x{height} needs {need} bytes and holds {have}")]
    Short {
        /// The raster's width in pixels.
        width: u32,
        /// Its height.
        height: u32,
        /// How many bytes four channels of those dimensions need.
        need: usize,
        /// How many it holds.
        have: usize,
    },
}

/// The viewer's pixels, as a paintable.
pub(crate) fn texture(raster: &Raster) -> Result<gdk::MemoryTexture, PixelError> {
    // Exhaustive, and that is what ADR 0247 bought: `RasterFormat` is no longer
    // `#[non_exhaustive]`, so a second pixel layout added to `pdf-render` fails to compile *here*
    // — beside the `GDK_MEMORY_R8G8B8A8` this function goes on to name — rather than arriving at
    // runtime as a refusal. This arm is the whole of the check that used to be a catch-all.
    match raster.format {
        RasterFormat::Rgba8 => {}
    }
    samples(raster.width, raster.height, &raster.data)
}

/// ISO 32000-2 §12.3.4's miniature, as a paintable.
///
/// The same conversion over a different carrier, which is why it is here rather than in the panel
/// that draws it: [`pdf_render::Image`] is the row-major straight-alpha RGBA8 a [`Raster`] is —
/// `pdf_model::thumbnail` decodes a `/Thumb` "into the same RGBA raster every other image in this
/// tree becomes" — so the two differ in where the pixels came from and in nothing GDK can see.
pub(crate) fn thumbnail(image: &Image) -> Result<gdk::MemoryTexture, PixelError> {
    samples(image.width, image.height, &image.data)
}

/// Row-major RGBA8 with no row padding, as a texture, with GDK's own two limits checked.
fn samples(width: u32, height: u32, data: &[u8]) -> Result<gdk::MemoryTexture, PixelError> {
    let (Ok(wide), Ok(tall)) = (i32::try_from(width), i32::try_from(height)) else {
        return Err(PixelError::TooLarge { width, height });
    };
    let stride = usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(4))
        .ok_or(PixelError::TooLarge { width, height })?;
    let need = usize::try_from(height)
        .ok()
        .and_then(|height| height.checked_mul(stride))
        .ok_or(PixelError::TooLarge { width, height })?;
    if data.len() < need {
        return Err(PixelError::Short {
            width,
            height,
            need,
            have: data.len(),
        });
    }
    let bytes = glib::Bytes::from(data);
    Ok(gdk::MemoryTexture::new(
        wide,
        tall,
        gdk::MemoryFormat::R8g8b8a8,
        &bytes,
        stride,
    ))
}
