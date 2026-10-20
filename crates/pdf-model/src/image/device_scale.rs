//! A soft mask kept at its own resolution and sampled at the device's grid when the pair would
//! combine past `PREFER_DEVICE_SCALE_ABOVE` samples on the finer of the two (ISO 32000-2
//! §11.6.5.2, ADR 0456).

use std::sync::Arc;

use pdf_render::{Image, SampleAlpha};

use super::combine::{MaskSamples, combine_on_the_finer_grid};
use super::mask::PREFER_DEVICE_SCALE_ABOVE;
use super::space::{ColourSpace, Decode};
use super::unpack::index_of;

/// §11.6.5.2's soft-mask image, kept in the file's own packed samples.
///
/// The alternative is one byte of RGBA per sample per channel, which is what
/// `combine_on_the_finer_grid` would need and what `issue16263.pdf`'s 34862×4332 mask makes
/// impossible: 604 MB for two distinct colours. Packed, the same mask is the 19 MB its
/// `FlateDecode` stream inflates to, and a raster of any grid can be read out of it.
///
/// Everything the read needs is settled before a sample is touched — the layout, and §8.9.5.2's
/// map from an integer sample to a component value — so this holds no document and no lifetime,
/// which is what lets it travel in a display list. That is not incidental: `Document` caches
/// what it parses behind `RefCell`, so it is not `Sync`, and a display list is drawn on every
/// core.
#[derive(Clone)]
pub struct SoftMaskAtDeviceScale {
    /// The stream's bytes with every non-image filter applied, packed as Table 87 describes.
    pub(super) data: Arc<[u8]>,
    /// The grid the file states.
    pub(super) width: u32,
    /// The grid the file states.
    pub(super) height: u32,
    /// Bits per sample: 1, 2, 4, 8 or 16.
    pub(super) bits: u32,
    /// §8.9.5.2's map from a raw sample to an opacity, as one table.
    ///
    /// Shared because [`MaskCache`] hands the same mask to every command that draws it.
    ///
    /// [`MaskCache`]: super::cache::MaskCache
    pub(super) decode: Arc<Decode>,
}

impl std::fmt::Debug for SoftMaskAtDeviceScale {
    /// The shape of the mask, never its samples: this exists to hold a raster too large to
    /// print, and a display list is printed by `Command`'s own derive.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SoftMaskAtDeviceScale")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bits", &self.bits)
            .finish_non_exhaustive()
    }
}

impl SoftMaskAtDeviceScale {
    /// This mask over `base`, as one raster a backend produces at the grid it draws at.
    ///
    /// The two rasters travel together from here on, which is the whole of what the display
    /// list needed to be told: §8.9.6.3 and Table 143 both put an image and its mask on the
    /// same unit square without putting them on the same grid, so nothing but the device can
    /// say where a sample of one lies in the other.
    #[must_use]
    pub fn over(self, base: Image) -> pdf_render::DeferredImage {
        pdf_render::DeferredImage::new(Arc::new(MaskedAtDeviceScale { base, mask: self }))
    }

    /// A mask whose opacities are already decoded, one byte per sample on a `width` × `height`
    /// grid, row by row.
    ///
    /// The map §8.9.5.2 puts between a sample and a value has been applied by whatever decoded
    /// the plane, so what is stored beside it is Table 88's identity for eight-bit
    /// `DeviceGray`.
    pub(super) fn grey_plane(width: u32, height: u32, plane: Arc<[u8]>) -> Self {
        Self {
            data: plane,
            width,
            height,
            bits: 8,
            decode: Arc::new(Decode::from_pairs(&[], &ColourSpace::Gray, 8)),
        }
    }

    /// The mask's samples as a grey raster, on a grid no finer than `grid`.
    ///
    /// ISO 32000-2 §10.7.4, of a sampled image drawn at a lower resolution than its own:
    ///
    /// > The position of the centre of such a pixel -in other words, the point whose
    /// > coordinate values have fractional parts of one-half -shall be mapped back into
    /// > source space to determine how to colour the pixel. There shall not be averaging over
    /// > the pixel area. If the resolution of the source image is higher than that of device
    /// > space, some source samples might not be used.
    ///
    /// That is the rule this follows, centre and all: output sample `i` of `cells` reads
    /// source sample `⌊(2i + 1) × samples ÷ 2 cells⌋`. It is a **departure from the departure**
    /// [`pdf_render::Image::area_averaged`] records — this renderer averages a reduced image
    /// where the clause says not to, on ADR 0025's argument — and the difference is deliberate:
    /// that argument rests on a measured witness whose thin features vanish, and averaging here
    /// would mean decoding every one of a mask's samples, which is the cost this whole route
    /// exists to avoid. What it costs is a mask feature thinner than a device pixel, which is
    /// exactly what the clause's last sentence says happens.
    fn raster(&self, grid: pdf_render::Grid) -> Image {
        let cells = self.cells(grid);
        let row_bytes = (self.width as usize)
            .saturating_mul(self.bits as usize)
            .saturating_add(7)
            / 8;
        let mut data = Vec::with_capacity(
            (cells.width as usize)
                .saturating_mul(cells.height as usize)
                .saturating_mul(4),
        );
        for down in 0..u64::from(cells.height) {
            let y = centre(down, u64::from(cells.height), u64::from(self.height));
            let from = y.saturating_mul(row_bytes);
            let row = self
                .data
                .get(from..from.saturating_add(row_bytes))
                .unwrap_or_default();
            for across in 0..u64::from(cells.width) {
                let x = centre(across, u64::from(cells.width), u64::from(self.width));
                let grey = self.decode.channel(0, index_of(row, x, self.bits));
                data.extend_from_slice(&[grey, grey, grey, u8::MAX]);
            }
        }
        Image {
            width: cells.width,
            height: cells.height,
            data: Arc::from(data.as_slice()),
            // Table 143 lists `/Interpolate` as "Optional" and the entry is about magnifying
            // an image's own samples; what a backend does with the *combined* raster is
            // decided by the base image's entry, which is where §8.9.5.3's hint was stated.
            interpolate: false,
            // This raster is the mask's grey levels, opaque at every sample, and it is never
            // a command's image: [`MaskedAtDeviceScale::samples`] reads it and nothing else
            // does. Its own alpha channel is the rectangle, which is `Shape`.
            sample_alpha: SampleAlpha::Shape,
        }
    }

    /// The grid this will actually produce: no finer than asked, than stated, or than fits.
    ///
    /// The third bound is the one a document controls. A backend asks for the device pixels the
    /// image covers, which a magnification the user chooses can make arbitrarily large, so the
    /// same [`PREFER_DEVICE_SCALE_ABOVE`] that sent the pair down this route bounds the grid it
    /// produces — halving both axes until the product fits, so the raster stays the shape of the
    /// request.
    fn cells(&self, grid: pdf_render::Grid) -> pdf_render::Grid {
        let mut cells = pdf_render::Grid {
            width: grid.width.min(self.width).max(1),
            height: grid.height.min(self.height).max(1),
        };
        while u64::from(cells.width).saturating_mul(u64::from(cells.height))
            > PREFER_DEVICE_SCALE_ABOVE
            && (cells.width > 1 || cells.height > 1)
        {
            cells.width = (cells.width / 2).max(1);
            cells.height = (cells.height / 2).max(1);
        }
        cells
    }
}

/// §10.7.4's centre of output cell `index` of `cells`, mapped back into a source of `samples`.
///
/// Integer arithmetic, and the last sample is never passed: `cells` is at most `samples`, so
/// `(2 × index + 1) × samples ÷ (2 × cells)` is below `samples` for every `index` below `cells`.
fn centre(index: u64, cells: u64, samples: u64) -> usize {
    let numerator = index
        .saturating_mul(2)
        .saturating_add(1)
        .saturating_mul(samples);
    let at = numerator
        .checked_div(cells.saturating_mul(2))
        .unwrap_or(0)
        .min(samples.saturating_sub(1));
    usize::try_from(at).unwrap_or(0)
}

/// An image and the soft mask that belongs with it, combined where the device scale is known.
#[derive(Debug)]
struct MaskedAtDeviceScale {
    base: Image,
    mask: SoftMaskAtDeviceScale,
}

impl pdf_render::ImageAtDeviceScale for MaskedAtDeviceScale {
    /// §11.6.5.2's mask applied to the base image, both resolved onto the device's grid.
    ///
    /// The combination itself is [`combine_on_the_finer_grid`]'s, unchanged: once the mask has
    /// been read at a grid the device can use, the two rasters are an ordinary pair and the one
    /// rule this crate has for combining a pair applies. What is different is only which grid
    /// the pair is on, which is the whole of what deferring bought.
    ///
    /// No `/Matte`: Table 143 makes the mask's `/Width` and `/Height` "the same as the ... value
    /// of the parent image" wherever one is present, so an image whose grids differ enough to
    /// reach this route cannot have one, and `soft_mask_entry` checks it rather than assuming.
    /// A stencil reaches it with or without one, and has no colour components for one to have
    /// been blended into (`decode_parts`, ADR 1279).
    fn samples(&self, grid: pdf_render::Grid) -> Image {
        let mask = self.mask.raster(grid);
        combine_on_the_finer_grid(
            &self.base,
            MaskSamples::of_raster(&mask),
            |colour, sample| (colour, sample.first().copied().unwrap_or(0)),
        )
    }

    /// The base's, which is what every raster [`Self::samples`] returns carries.
    ///
    /// [`decode_parts`] states it on the base before the pair is built — this route exists
    /// only because a §11.6.5.2 soft mask is being applied, so the answer is
    /// [`pdf_render::SampleAlpha::Opacity`], or `Both` where the base is a stencil — and
    /// [`combine_on_the_finer_grid`] carries it onto the combined raster. Answering from the
    /// field rather than restating it is what keeps the deferred route and the eager one one
    /// decision.
    ///
    /// [`decode_parts`]: super::decode_parts
    /// [`Self::samples`]: pdf_render::ImageAtDeviceScale::samples
    fn sample_alpha(&self) -> SampleAlpha {
        self.base.sample_alpha
    }

    /// The base alone, which for a stencil under its own `/SMask` is §11.6.4.2's shape.
    ///
    /// > For image masks (8.9.6.2, "Stencil masking"), the shape shall be 1.0 for painted
    /// > areas and 0.0 for masked areas.
    ///
    /// So the stencil's own samples *are* the shape, and this hands them back on the grid the
    /// file states them on — the grid a stencil with no soft mask would be drawn from anyway.
    /// The kind is restated as [`SampleAlpha::Shape`] because that is what this raster's alpha
    /// now is: the product is in [`Self::samples`] and not in this one. `None` for a base that
    /// is not a stencil, whose alpha is the mask's opacity alone and has no shape to separate.
    /// ADR 1218.
    ///
    /// [`Self::samples`]: pdf_render::ImageAtDeviceScale::samples
    fn shape(&self) -> Option<pdf_render::ImageSource> {
        (self.base.sample_alpha == SampleAlpha::Both).then(|| {
            pdf_render::ImageSource::Decoded(Image {
                sample_alpha: SampleAlpha::Shape,
                ..self.base.clone()
            })
        })
    }
}

#[cfg(test)]
mod tests {
    /// §10.7.4's centre rule, which is what maps a device pixel back to a mask sample.
    ///
    /// > The position of the centre of such a pixel -in other words, the point whose
    /// > coordinate values have fractional parts of one-half -shall be mapped back into
    /// > source space to determine how to colour the pixel.
    ///
    /// Three properties, and the first is the one a corner rule fails: a grid that is not
    /// reduced at all reads every sample once and in order, so nothing shifts when a mask
    /// happens to be the size the device wants. The second is the rule itself at a four-fold
    /// reduction — cell `i` of 2 over 8 samples reads the sample at `(2i + 1) x 8 / 4`, which
    /// is 2 and 6, the centres of the two halves rather than their corners. The third is that
    /// no index ever leaves the source, which is what a bounds check would otherwise have to
    /// do once per sample.
    #[test]
    fn a_device_cell_reads_the_sample_under_its_centre() {
        for index in 0..8u64 {
            assert_eq!(
                super::centre(index, 8, 8),
                usize::try_from(index).expect("a literal under eight"),
                "the identity"
            );
        }
        assert_eq!((super::centre(0, 2, 8), super::centre(1, 2, 8)), (2, 6));
        assert_eq!(
            (
                super::centre(0, 3, 7),
                super::centre(1, 3, 7),
                super::centre(2, 3, 7)
            ),
            (1, 3, 5),
            "an odd reduction that divides nothing evenly"
        );
        for cells in 1..=32u64 {
            for index in 0..cells {
                assert!(super::centre(index, cells, 32) < 32);
            }
        }
    }
}
