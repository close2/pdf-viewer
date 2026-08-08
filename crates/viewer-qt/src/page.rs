//! Tier 1: the raster the viewer is holding, checked before it becomes a `QImage`.
//!
//! `doc/ui-boundary.md` prices the tier at "one copy per frame", and this module is what makes
//! that copy legal rather than what performs it. [`pdf_render::Raster`] is row-major RGBA with
//! straight alpha and no row padding, which is exactly `QImage::Format_RGBA8888` — the byte
//! order Qt names R, G, B, A regardless of the machine's endianness, and *not* premultiplied,
//! which Qt spells `Format_RGBA8888_Premultiplied` and which this is not. So what crosses is a
//! `memcpy` into a `QImage` and no conversion at all, exactly as `viewer-gtk`'s crossing into a
//! `gdk::MemoryTexture` is.
//!
//! Both hosts therefore pay one copy of the same bytes into the same layout, which is what makes
//! ADR 0244's ≈3.2 GB/s and ADR 0246's number comparable at all.

use pdf_render::{Raster, RasterFormat};

/// Why a raster could not become a `QImage`.
///
/// Both are conditions Qt's own interface imposes and neither can be reached by any page this
/// program draws today — a raster is bounded by `pdf_render::MAX_EXTENT`, which is 2^24. They are
/// typed and reported rather than asserted away, because the alternative to a report here is a
/// window that shows the previous page and says nothing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum PixelError {
    /// The raster holds fewer bytes than its own dimensions call for.
    ///
    /// Checked here rather than trusted, because the `QImage` the C++ side builds reads
    /// `height × width × 4` bytes out of a slice this crate handed it: a short raster would be a
    /// read past the end, on the C++ side of a bridge, which is precisely the class of mistake
    /// the rest of this tree has a compiler to prevent.
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

/// The raster's dimensions, once it is known to be one Qt can take.
///
/// # Errors
///
/// [`PixelError`], one variant per condition above.
pub(crate) fn describe(raster: &Raster) -> Result<(u32, u32), PixelError> {
    // Exhaustive, and that is what ADR 0247 bought: `RasterFormat` is no longer
    // `#[non_exhaustive]`, so a second pixel layout added to `pdf-render` fails to compile *here*
    // — beside the `QImage::Format_RGBA8888` the C++ side names — rather than arriving at runtime
    // as a refusal. This arm is the whole of the check that used to be a catch-all.
    match raster.format {
        RasterFormat::Rgba8 => {}
    }
    let need = usize::try_from(raster.width)
        .ok()
        .and_then(|width| width.checked_mul(4))
        .and_then(|stride| {
            usize::try_from(raster.height)
                .ok()
                .and_then(|height| height.checked_mul(stride))
        })
        .ok_or(PixelError::Short {
            width: raster.width,
            height: raster.height,
            need: usize::MAX,
            have: raster.data.len(),
        })?;
    if raster.data.len() < need {
        return Err(PixelError::Short {
            width: raster.width,
            height: raster.height,
            need,
            have: raster.data.len(),
        });
    }
    Ok((raster.width, raster.height))
}

#[cfg(test)]
mod tests {
    use super::{PixelError, describe};
    use pdf_render::{Raster, RasterFormat};

    /// A raster the C++ side would read past the end of is refused before it gets there.
    ///
    /// This is the one check in this crate that is *load-bearing for memory safety* rather than
    /// for correctness: everything else the bridge carries is a value `cxx` sizes itself, and
    /// this is the one place a length on the Rust side decides how many bytes C++ reads.
    #[test]
    fn a_raster_shorter_than_its_own_dimensions_is_refused_by_name() {
        let raster = Raster {
            width: 4,
            height: 4,
            format: RasterFormat::Rgba8,
            data: vec![0; 4 * 4 * 4 - 1],
        };
        assert_eq!(
            describe(&raster),
            Err(PixelError::Short {
                width: 4,
                height: 4,
                need: 64,
                have: 63,
            })
        );
    }

    /// And a raster that is exactly long enough is taken.
    #[test]
    fn a_whole_raster_answers_its_own_dimensions() {
        let raster = Raster {
            width: 3,
            height: 2,
            format: RasterFormat::Rgba8,
            data: vec![0; 3 * 2 * 4],
        };
        assert_eq!(describe(&raster), Ok((3, 2)));
    }
}
