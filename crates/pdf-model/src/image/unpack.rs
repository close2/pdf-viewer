//! Samples read out of their packed rows at ISO 32000-2 §8.9.5.1 Table 87's bit depths and
//! painted into RGBA, through a one-component palette or an eight-bit device table where one
//! fits.
//!
//! §8.9.6.4's colour-key ranges are compared here, on the raw integers before `/Decode`, which
//! is what the clause means by "before decoding".

use rayon::iter::IndexedParallelIterator as _;
use rayon::iter::ParallelIterator as _;
use rayon::slice::ParallelSliceMut as _;

use crate::colour::Conversion;

use super::ImageError;
use super::convert::{SampleMemo, channel};
use super::dispatch::Premultiplied;
use super::space::{ColourSpace, Decode};

/// What a route needs beyond the bytes to turn samples into a raster.
///
/// The four things every route from bytes to colour asks for and none of them decides: whether
/// §8.9.6.2's stencil rule applies and what colour it paints through, §8.9.6.4's ranges, what the
/// samples are being composited into, and §11.6.5.2's pre-blending where they carry one. Grouped
/// because the routes differ in where the bytes came from and in nothing else, which is
/// [`Samples`]'s reason one level down.
#[derive(Clone, Copy)]
pub(super) struct Painting<'a> {
    /// §8.9.5.1's `/ImageMask`: the samples are one bit of shape rather than colour.
    pub(super) is_mask: bool,
    /// The current fill colour, which a stencil paints through the bits that mark the page.
    pub(super) fill: pdf_render::Color,
    /// §8.9.6.4's colour-key ranges, one per component, in raw sample values.
    pub(super) colour_key: Option<&'a [(u32, u32)]>,
    /// How the samples are converted (`crate::colour::Conversion`).
    pub(super) into: &'a Conversion,
    /// §11.6.5.2's pre-blending, where the samples carry one. See [`Prematte`].
    pub(super) matte: Option<&'a Prematte<'a>>,
}

/// §11.6.5.2's pre-blending, and what a reader needs to undo it.
///
/// > If the image data are pre-blended, the matte colour shall be specified by a Matte entry
/// > in the soft-mask image dictionary
///
/// The clause states the blending as `c′ = m + α × (c - m)`, "performed independently for
/// each component", and requires the inversion to run where the blending did:
///
/// > The preblending computation shall be done in the colour space specified by the parent
/// > image's ColorSpace entry. This is independent of the group colour space into which the
/// > image may be painted. If a colour conversion is required, inversion of the pre-blending
/// > shall precede the colour conversion.
///
/// So this travels *into* the routes that turn samples into colour and is applied to the
/// components, before the space is consulted. Doing it afterwards — on the raster this crate
/// holds — is exact only where the conversion is the identity on components, which is the two
/// device spaces and nothing else; in any other space the bytes are a function of the
/// pre-blended components rather than the components. ADR 1268.
///
/// Table 143 makes the mask's `/Width` and `/Height` "the same as the … value of the parent
/// image" wherever a `/Matte` is present, so [`Prematte::alpha`] is one byte per sample of the
/// parent's own grid and the two are paired by position.
pub(super) struct Prematte<'a> {
    /// Table 144's colour, in the parent image's own component values.
    pub(super) matte: &'a [f32],
    /// The mask's samples, one byte each, on the parent image's grid: the clause's `α`.
    pub(super) alpha: &'a [u8],
    /// Whether [`Self::alpha`] is the very plane [`apply_soft_mask`] would multiply into the
    /// raster, on the raster's own grid — so that an inversion writing every pixel may write `α`
    /// as its alpha and leave the multiplication nothing to do (ADR 1519).
    ///
    /// The byte is the same: the inversion leaves every alpha at 255, and the multiplication
    /// writes `(255 × α + 127) ÷ 255`, which is `α` for every `α` from 0 to 255 because 127 is
    /// less than 255. No rounding is taken or saved, so this is a pass over the raster removed and
    /// not an answer changed.
    ///
    /// [`apply_soft_mask`]: super::soft_mask::apply_soft_mask
    pub(super) opacity: bool,
}

impl<'a> Prematte<'a> {
    /// The pre-blending `matte_before_samples` found, borrowed.
    pub(super) fn of((matte, alpha, opacity): &'a Premultiplied) -> Self {
        Self {
            matte,
            alpha,
            opacity: *opacity,
        }
    }

    /// The same matte over `alpha`, a plane carried onto another grid, which is therefore not the
    /// plane the eager route multiplies in.
    pub(super) fn carried<'b>(&self, alpha: &'b [u8]) -> Prematte<'b>
    where
        'a: 'b,
    {
        Prematte {
            matte: self.matte,
            alpha,
            opacity: false,
        }
    }

    /// The original components of the pixel at `at`, from the pre-blended ones.
    ///
    /// `c = m + (c′ - m) ÷ α`, per component, clamped by §11.6.5.2's own closing requirement —
    /// "\[t]he resulting c value shall lie within the range of colour component values for the
    /// image colour space". Where α is 0 the clause's NOTE says the inversion divides by zero
    /// and "an arbitrary value for c can be chosen", because a fully transparent sample cannot
    /// affect the output; the matte colour is the value that costs nothing to justify.
    pub(super) fn restore(
        &self,
        values: &mut [f32],
        at: usize,
        permitted: &dyn Fn(usize) -> (f32, f32),
    ) {
        let alpha = self.alpha.get(at).copied().unwrap_or(0);
        for (component, value) in values.iter_mut().enumerate() {
            *value = self.restore_component(*value, alpha, component, permitted(component));
        }
    }

    /// [`Self::restore`] for one component of a sample whose mask value is `alpha`, which is
    /// the whole of the inversion: a function of the pre-blended value, `α`, and the
    /// component's matte and permitted range, and of nothing about where the pixel is.
    fn restore_component(
        &self,
        value: f32,
        alpha: u8,
        component: usize,
        (low, high): (f32, f32),
    ) -> f32 {
        let matte = self.matte.get(component).copied().unwrap_or(0.0);
        if alpha == 0 {
            matte
        } else {
            let opacity = f32::from(alpha) / 255.0;
            (value - matte).mul_add(1.0 / opacity, matte)
        }
        .clamp(low.min(high), low.max(high))
    }
}

/// How a row of raw bytes becomes colour: the layout, and what a value means.
///
/// Grouped rather than passed one at a time because all five are settled by the image
/// dictionary before a byte is read, and because the four routes that reach [`unpack`]
/// differ only in where the bytes came from.
pub(super) struct Samples<'a> {
    /// Bits per component.
    pub(super) bits: u32,
    /// What a component means.
    pub(super) space: &'a ColourSpace,
    /// What a sample means, per component (§8.9.5.2).
    pub(super) decode: &'a Decode,
    /// §8.9.6.4's colour-key ranges, one per component, in raw sample values.
    ///
    /// The clause's test is on the samples "before decoding with the Decode array", which is
    /// why this travels into the unpacker rather than filtering the RGBA it produces: after
    /// conversion the component values are gone, and for every space but the device ones
    /// they were never in the raster to begin with.
    pub(super) colour_key: Option<&'a [(u32, u32)]>,
    /// The current fill colour, which a stencil paints through the bits that mark the page.
    pub(super) fill: pdf_render::Color,
    /// How the samples are converted (`crate::colour::Conversion`).
    ///
    /// A sample is a colour like any other, and §11.6.5.1's mask group composites in a
    /// quantity that is not the device's — so a raster that ignored this would be the one
    /// thing in such a group painted in the wrong units (ADR 0220).
    pub(super) into: &'a Conversion,
    /// §11.6.5.2's pre-blending, where the samples carry one and it has to be undone before
    /// the space is consulted. See [`Prematte`].
    pub(super) matte: Option<&'a Prematte<'a>>,
}

/// Unpacks raw samples into RGBA8.
pub(super) fn unpack(
    data: &[u8],
    width: u32,
    height: u32,
    samples: &Samples,
) -> Result<Vec<u8>, ImageError> {
    let &Samples {
        bits,
        space,
        decode,
        colour_key,
        fill: _,
        into,
        matte,
    } = samples;
    // Table 87 names five, and a value it does not name says nothing about how the bytes are
    // packed — so it is refused rather than rounded to a depth that would shift every sample.
    if !matches!(bits, 1 | 2 | 4 | 8 | 16) {
        return Err(ImageError::UnsupportedDepth { bits });
    }

    let components = space.components();
    // A space taking one component has at most 2^bits colours, so every one of them can be
    // converted once instead of once per sample. This is exact rather than an approximation
    // — the samples are integers and the table holds every value one can take — and it is
    // what keeps an `Indexed` image whose base is an ICC profile, or a `Separation` whose
    // tint transform is a PostScript program, off the per-sample path.
    //
    // Measured over the sixteen corpus documents that draw a space this arm reaches:
    // 1.03 s of rendering without it, 330 ms with. `issue9940.pdf` is the extreme — 620 ms
    // to 42 ms — because its images are `Indexed` over a `DeviceN` whose tint transform is a
    // PostScript calculator, which was being run once per sample rather than once per entry
    // of a 256-entry table.
    // Only up to eight bits: a 16-bit table is 65 536 conversions, which is more work than
    // the image itself for anything smaller than a quarter-megapixel, and the per-sample arm
    // below already memoises exactly. No corpus image reaches that combination.
    // A matte takes both of this function's shortcuts away, and for the same reason: the
    // colour of a sample is no longer a function of the sample alone — §11.6.5.2 makes it a
    // function of the sample *and* the mask value at that pixel — so a table indexed by the
    // sample, and a memo keyed on it, would both answer for the wrong pixel. An eight-bit
    // device image tabulates the pair instead ([`matted_eight_bit_device_tables`]); every other
    // space pays one conversion per sample.
    let palette = match space {
        ColourSpace::Resolved(resolved)
            if resolved.components() == 1 && bits <= 8 && matte.is_none() =>
        {
            Some(palette(resolved, bits, decode, into))
        }
        _ => None,
    };
    let width_usize = width as usize;
    let height_usize = height as usize;
    // Each row starts on a byte boundary.
    let row_bits = width_usize
        .saturating_mul(components)
        .saturating_mul(bits as usize);
    let row_bytes = row_bits.saturating_add(7) / 8;

    // Only the arm that converts per sample needs one, and only where a tuple fits a key.
    // The memo's key packs eight bits per component, so it is exact only at or below that
    // depth; see `resolved_sample`.
    let mut cache = matches!(space, ColourSpace::Resolved(_))
        .then(|| palette.is_none() && components <= 4 && bits <= 8 && matte.is_none())
        .filter(|fits| *fits)
        .map(|_| {
            SampleMemo::for_pixels(
                width_usize.saturating_mul(height_usize),
                pdf_render::Color::BLACK,
            )
        });

    if let Some(tables) = matted_eight_bit_device_tables(samples, width_usize, height_usize) {
        return Ok(unpack_matted_eight_bit_device(
            data,
            (width_usize, height_usize),
            (row_bytes, components),
            &tables,
            matte.map_or(&[][..], |matte| matte.alpha),
        ));
    }
    if let Some(tables) = eight_bit_device_tables(samples) {
        return Ok(unpack_eight_bit_device(
            data,
            width_usize,
            height_usize,
            row_bytes,
            &tables,
        ));
    }
    if let Some(table) = packed_pixels(samples, palette.as_deref()) {
        return Ok(unpack_packed(
            data,
            (width_usize, height_usize),
            row_bytes,
            &table,
        ));
    }

    let mut out = Vec::with_capacity(width_usize.saturating_mul(height_usize).saturating_mul(4));

    // §7.3.8.2 puts this image's extent among the lengths "from whose attributes a length can
    // be inferred", and its EXAMPLE is this arithmetic: a 10-row, 20-column, 8-bit, one-component
    // image "requires exactly 200 bytes of image data", and "[a]n error occurs … if the decoded
    // data does not contain 200 bytes". So a stream that stops short carries samples for part of
    // the grid and none at all for the rest, and the two halves are treated differently for the
    // reason ADR 0343 gives one clause over: the samples the producer wrote are drawn where they
    // belong, and the ones the file never carried are left **unpainted** rather than read as
    // zero. Reading them as zero is what painted a solid stencil over `178360.pdf`'s page — 359
    // bytes of a 50 048-byte `/ImageMask`, the remaining 99.3% of it marking the page in the fill
    // colour, which no reference renderer draws. `short_of_its_grid` says so out loud.
    let sample_bits = components.max(1).saturating_mul(bits as usize).max(1);
    for y in 0..height_usize {
        let start = y.saturating_mul(row_bytes);
        let available = data.len().saturating_sub(start).min(row_bytes);
        let row = data
            .get(start..start.saturating_add(available))
            .unwrap_or_default();
        // Whole samples only: a sample whose last bits are past the end of the data was never
        // written, and half of one is not a colour.
        let carried = available
            .saturating_mul(8)
            .checked_div(sample_bits)
            .unwrap_or(0);

        for x in 0..width_usize {
            if x >= carried {
                out.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            // §8.9.6.4: "An image sample shall be masked (not painted) if all of its colour
            // components before decoding … fall within the specified ranges". A masked
            // sample keeps its position and loses its opacity; nothing else about it is
            // read, which is why this stands before the conversion rather than after it.
            if colour_key.is_some_and(|ranges| colour_key_masks(row, x, bits, components, ranges)) {
                out.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            out.extend_from_slice(&sample_rgba(
                samples,
                palette.as_deref(),
                &mut cache,
                row,
                x,
                y.saturating_mul(width_usize).saturating_add(x),
            ));
        }
    }

    Ok(out)
}

/// The channel table of each component of an eight-bit `DeviceGray` or `DeviceRGB` image, where
/// [`sample_rgba`]'s answer for a pixel is those tables' entries and nothing else — or `None`
/// for every other shape of image, which keeps [`unpack`]'s per-sample route.
///
/// The conditions are the ones under which [`sample_rgba`]'s `Gray` and `Rgb` arms reduce to
/// [`Decode::channel`]: no §11.6.5.2 pre-blending to undo (the matte'd arms convert through
/// floats), no §8.9.6.4 colour key (which masks by the raw sample), and eight bits, where a
/// sample is one byte and [`raw_sample`] reads it unshifted.
pub(super) fn eight_bit_device_tables<'a>(samples: &Samples<'a>) -> Option<Vec<&'a [u8; 256]>> {
    let &Samples {
        bits,
        space,
        decode,
        colour_key,
        matte,
        ..
    } = samples;
    if bits != 8 || colour_key.is_some() || matte.is_some() {
        return None;
    }
    let components = match space {
        ColourSpace::Gray => 1,
        ColourSpace::Rgb => 3,
        _ => return None,
    };
    (0..components)
        .map(|component| {
            decode
                .channels
                .get(component)
                .and_then(|table| <&[u8; 256]>::try_from(table.as_slice()).ok())
        })
        .collect()
}

/// The fewest pixels an image carrying §11.6.5.2's pre-blending must have before its inversion
/// is tabulated: a table is 65 536 evaluations of [`Prematte::restore_component`] a component,
/// and below this many pixels the per-sample route does no more than that.
const MATTE_TABLE_FLOOR: usize = 1 << 16;

/// For an eight-bit `DeviceGray` or `DeviceRGB` image carrying §11.6.5.2's pre-blending, each
/// component's channel as a table of `(α, sample)` — `table[α × 256 + sample]` — or `None` for
/// every other shape of image, and for one smaller than [`MATTE_TABLE_FLOOR`].
///
/// **Each entry is what [`sample_rgba`]'s matte'd arm computes for that pair**, by the same
/// calls in the same order: [`Decode::value`], then [`Prematte::restore_component`] with the
/// space's permitted range, then [`channel`]. The inversion is a function of the sample, the
/// mask value and the component, and nothing else — so evaluating it once for each of the
/// 65 536 pairs a component can meet answers every pixel with the bytes the per-sample route
/// writes. On `issue13931.pdf`, a 2 996 × 4 256 `DeviceRGB` photograph under a `/Matte`'d mask,
/// the per-sample route was 38 million inversions with two allocations a pixel (ADR 1457).
pub(super) fn matted_eight_bit_device_tables(
    samples: &Samples,
    width: usize,
    height: usize,
) -> Option<Vec<MatteTable>> {
    let &Samples {
        bits,
        space,
        decode,
        colour_key,
        matte,
        ..
    } = samples;
    let matte = matte?;
    if bits != 8 || colour_key.is_some() || width.saturating_mul(height) < MATTE_TABLE_FLOOR {
        return None;
    }
    let components = match space {
        ColourSpace::Gray => 1,
        ColourSpace::Rgb => 3,
        _ => return None,
    };
    (0..components)
        .map(|component| {
            let permitted = space.permitted(component);
            let table: Box<[u8]> = (0..=u16::MAX)
                .map(|index| {
                    let [raw, alpha] = index.to_le_bytes();
                    let value = decode.value(component, usize::from(raw));
                    channel(matte.restore_component(value, alpha, component, permitted))
                })
                .collect();
            MatteTable::try_from(table).ok()
        })
        .collect()
}

/// One component's [`matted_eight_bit_device_tables`] entry for every `(α, sample)` pair,
/// indexed `α × 256 + sample`: a fixed size, so that an index made of two bytes needs no check.
pub(super) type MatteTable = Box<[u8; 1 << 16]>;

/// [`unpack`] for the images [`matted_eight_bit_device_tables`] admits: each sample one lookup
/// by its own value and its pixel's mask value, read from rows of `row_bytes` whose pixels are
/// `stride` bytes apart.
///
/// The same pixels as the per-sample route, by the tables' construction and by the layout
/// [`unpack_eight_bit_device`] shares with it: a pixel past what its row carries stays
/// `[0, 0, 0, 0]`, and a pixel past the mask's samples reads `α` as 0, as
/// [`Prematte::restore`] does.
pub(super) fn unpack_matted_eight_bit_device(
    data: &[u8],
    (width, height): (usize, usize),
    (row_bytes, stride): (usize, usize),
    tables: &[MatteTable],
    alpha: &[u8],
) -> Vec<u8> {
    let pixel_row = width.saturating_mul(4);
    let mut out = vec![0u8; pixel_row.saturating_mul(height)];
    if pixel_row == 0 || stride == 0 {
        return out;
    }
    let entry = |table: &MatteTable, alpha: u8, sample: u8| {
        table[usize::from(alpha) << 8 | usize::from(sample)]
    };
    // A row is a function of its own samples and mask values, so the rows divide across the pool
    // and no byte depends on how (ADR 0147's argument for `convert_three`).
    let fill = |y: usize, pixels: &mut [u8]| {
        let start = y.saturating_mul(row_bytes);
        let available = data.len().saturating_sub(start).min(row_bytes);
        let row = data
            .get(start..start.saturating_add(available))
            .unwrap_or_default();
        let first = y.saturating_mul(width);
        let alphas = alpha
            .get(first..first.saturating_add(width).min(alpha.len()))
            .unwrap_or_default();
        // A mask shorter than the image reads its missing values as 0, as `Prematte::restore`
        // does.
        let alphas = alphas.iter().copied().chain(std::iter::repeat(0));
        let pixels = pixels
            .chunks_exact_mut(4)
            .zip(row.chunks_exact(stride))
            .zip(alphas);
        match tables {
            [grey] => {
                for ((pixel, sample), alpha) in pixels {
                    let level = entry(grey, alpha, sample[0]);
                    pixel.copy_from_slice(&[level, level, level, u8::MAX]);
                }
            }
            [red, green, blue] => {
                for ((pixel, sample), alpha) in pixels {
                    pixel.copy_from_slice(&[
                        entry(red, alpha, sample[0]),
                        entry(green, alpha, sample[1]),
                        entry(blue, alpha, sample[2]),
                        u8::MAX,
                    ]);
                }
            }
            _ => {}
        }
    };
    out.par_chunks_exact_mut(pixel_row)
        .enumerate()
        .for_each(|(y, pixels)| fill(y, pixels));
    out
}

/// [`unpack`] for the images [`eight_bit_device_tables`] admits: each sample one table lookup,
/// each row one pass pairing its bytes with its pixels.
///
/// **The same pixels as the per-sample route, by construction.** A carried pixel is
/// `[t₀[r], t₁[g], t₂[b], 255]` (or `[t[v], t[v], t[v], 255]` for grey), which is what
/// [`sample_rgba`] builds from [`Decode::channel`]; a pixel past what the row carries stays the
/// `[0, 0, 0, 0]` the buffer starts as, which is what [`unpack`] writes there for §7.3.8.2's
/// reason. What goes is the per-sample work around the lookups — a `match` on the space, a
/// `match` on the depth and three bounds-checked reads a pixel — which callgrind put at 48% of
/// interpreting `images.pdf`'s page (ADR 1433).
fn unpack_eight_bit_device(
    data: &[u8],
    width: usize,
    height: usize,
    row_bytes: usize,
    tables: &[&[u8; 256]],
) -> Vec<u8> {
    let components = tables.len().max(1);
    let pixel_row = width.saturating_mul(4);
    let mut out = vec![0u8; pixel_row.saturating_mul(height)];
    if pixel_row == 0 {
        return out;
    }
    for (y, pixels) in out.chunks_exact_mut(pixel_row).enumerate() {
        let start = y.saturating_mul(row_bytes);
        let available = data.len().saturating_sub(start).min(row_bytes);
        let row = data
            .get(start..start.saturating_add(available))
            .unwrap_or_default();
        match tables {
            [grey] => {
                for (pixel, &value) in pixels.chunks_exact_mut(4).zip(row) {
                    let level = grey[usize::from(value)];
                    pixel.copy_from_slice(&[level, level, level, u8::MAX]);
                }
            }
            [red, green, blue] => {
                for (pixel, sample) in pixels.chunks_exact_mut(4).zip(row.chunks_exact(components))
                {
                    pixel.copy_from_slice(&[
                        red[usize::from(sample[0])],
                        green[usize::from(sample[1])],
                        blue[usize::from(sample[2])],
                        u8::MAX,
                    ]);
                }
            }
            _ => {}
        }
    }
    out
}

/// The pixels each value of a byte of a one-component image unpacks to, as 256 runs of
/// `8 ÷ bits` pixels — or `None` for every image whose pixel is not a function of its own sample
/// alone, which keeps [`unpack`]'s per-sample route.
///
/// A one-component sample of at most eight bits takes at most 256 values, and **each pixel in
/// the table is [`sample_rgba`]'s own answer** for a row holding that one sample: the table is
/// built by calling it, which is what makes this route exact rather than a second reading of the
/// colour. A pixel is a function of its sample alone wherever there is no §11.6.5.2 pre-blending
/// (whose inversion reads the pixel's mask value) and no §8.9.6.4 colour key (which masks by the
/// raw sample, and is answered per sample by the route that reads one). That covers a §8.9.6.2
/// stencil, whose "Decode entry determines how the source samples shall be interpreted" through
/// [`Decode::value`] and whose mark is the current colour; a `DeviceGray` image under §8.9.5.2's
/// `/Decode` at one, two or four bits; and every one-component space [`palette`] tabulates —
/// `Indexed`, `Separation`, `CalGray`, a one-component `ICCBased` — at any depth to eight.
///
/// §8.9.3's layout is what lets a byte stand for its pixels:
///
/// > Byte boundaries shall be ignored, except that each row of sample data shall begin on a byte
/// > boundary. If the number of data bits per row is not a multiple of 8, the end of the row is
/// > padded with extra bits to fill out the last byte. A PDF processor shall ignore these padding
/// > bits.
///
/// So no byte holds samples of two rows, and Table 87's depths divide a byte exactly, so no sample
/// straddles two; a row's last byte contributes only the pixels its row has, which
/// [`unpack_packed`] copies, and the padding's pixels it never reads. ADR 1557.
pub(super) fn packed_pixels(
    samples: &Samples,
    palette: Option<&[pdf_render::Color]>,
) -> Option<PackedPixels> {
    // Table 87's depths to eight, as how many samples a byte holds and how far right a byte's
    // first sample is shifted to read it.
    let (per_byte, shift) = match samples.bits {
        1 => (8_usize, 7_u32),
        2 => (4, 6),
        4 => (2, 4),
        8 => (1, 0),
        _ => return None,
    };
    if samples.space.components() != 1 || samples.colour_key.is_some() || samples.matte.is_some() {
        return None;
    }
    // A row whose first sample is `value` is that value in the byte's high-order bits, which is
    // where §8.9.3 puts a row's first unit; `at` is read only by a matte, which is excluded above.
    let answers: Vec<[u8; 4]> = (0..=u8::MAX.checked_shr(shift).unwrap_or(0))
        .map(|value| {
            let row = [value.checked_shl(shift).unwrap_or(0)];
            sample_rgba(samples, palette, &mut None, &row, 0, 0)
        })
        .collect();
    let mut runs = Vec::with_capacity(per_byte.saturating_mul(1024));
    for byte in 0..=u8::MAX {
        // Each sample in turn moved to the high-order end and read from there.
        let mut rest = byte;
        for _ in 0..per_byte {
            let value = rest.checked_shr(shift).unwrap_or(0);
            runs.extend_from_slice(answers.get(usize::from(value))?);
            rest = rest.checked_shl(samples.bits).unwrap_or(0);
        }
    }
    Some(PackedPixels {
        runs,
        run: per_byte.saturating_mul(4),
    })
}

/// [`packed_pixels`]'s table: for each value of a byte, the RGBA pixels its samples stand for,
/// `run` bytes apiece and laid end to end.
pub(super) struct PackedPixels {
    /// 256 runs, the one for byte `b` at `b × run`.
    runs: Vec<u8>,
    /// The bytes of one run: four for each sample a byte holds.
    run: usize,
}

/// [`unpack`] for the images [`packed_pixels`] admits: each byte of a row one copy of the
/// pixels it stands for, read from rows of `row_bytes`.
///
/// **The same pixels as the per-sample route.** A carried byte's run is the answers
/// [`sample_rgba`] gives for its samples, most significant first as §8.9.3 orders them; the row's
/// last byte is cut to the pixels the row has; and a pixel past what the row carries stays the
/// `[0, 0, 0, 0]` the buffer starts as, which is what [`unpack`] writes there for §7.3.8.2's
/// reason. What goes is a call, a `match` on the space and a bit extraction a pixel: decoding
/// `bug1815476.pdf`'s three one-bit stencils cost 244 M instructions by the per-sample route and
/// 5.4 M by this one, and `issue9940.pdf`'s two eight-bit `Indexed` images 496 M and 34 M
/// (ADR 1557).
fn unpack_packed(
    data: &[u8],
    (width, height): (usize, usize),
    row_bytes: usize,
    table: &PackedPixels,
) -> Vec<u8> {
    let pixel_row = width.saturating_mul(4);
    let mut out = vec![0u8; pixel_row.saturating_mul(height)];
    if pixel_row == 0 {
        return out;
    }
    // One copy per run of a size the compiler knows, rather than a `memcpy` call for each:
    // on `issue9940.pdf`'s two eight-bit `Indexed` images the call was 72 M of the 162 M this
    // route cost with a run whose length was read at run time (ADR 1557).
    let fill = match table.run {
        32 => fill_packed_rows::<32>,
        16 => fill_packed_rows::<16>,
        8 => fill_packed_rows::<8>,
        4 => fill_packed_rows::<4>,
        _ => return out,
    };
    for (y, row_pixels) in out.chunks_exact_mut(pixel_row).enumerate() {
        let start = y.saturating_mul(row_bytes);
        let available = data.len().saturating_sub(start).min(row_bytes);
        let row = data
            .get(start..start.saturating_add(available))
            .unwrap_or_default();
        fill(row, row_pixels, &table.runs);
    }
    out
}

/// One row of [`unpack_packed`]: each carried byte's `RUN` bytes of pixels copied from `runs`,
/// the row's last byte cut to the pixels the row still has, and every pixel past the carried
/// bytes left as it was.
fn fill_packed_rows<const RUN: usize>(row: &[u8], pixels: &mut [u8], runs: &[u8]) {
    let (runs, _) = runs.as_chunks::<RUN>();
    let (whole, tail) = pixels.as_chunks_mut::<RUN>();
    let carried = whole.len().min(row.len());
    for (run, &byte) in whole.iter_mut().zip(row) {
        if let Some(source) = runs.get(usize::from(byte)) {
            *run = *source;
        }
    }
    // A row whose width the byte's samples do not divide ends inside a byte; its padding is
    // §8.9.3's, and only the pixels before it are copied.
    if carried == whole.len()
        && !tail.is_empty()
        && let Some(source) = row
            .get(carried)
            .and_then(|&byte| runs.get(usize::from(byte)))
        && let Some(prefix) = source.get(..tail.len())
    {
        tail.copy_from_slice(prefix);
    }
}

/// Whether §8.9.6.4's ranges cover every component of one sample.
///
/// The values compared are the raw integers the filter delivered, which is what the clause
/// means by "before decoding": `min i ≤ c i ≤ max i for all 1 ≤ i ≤ n`. `ranges` has one
/// entry per component, which [`mask_entry`] checked against the colour space before the
/// samples were read.
///
/// [`mask_entry`]: super::mask::mask_entry
fn colour_key_masks(
    row: &[u8],
    x: usize,
    bits: u32,
    components: usize,
    ranges: &[(u32, u32)],
) -> bool {
    let at = x.saturating_mul(components);
    ranges.iter().enumerate().all(|(offset, (low, high))| {
        let index = at.saturating_add(offset);
        let raw = raw_sample(row, index, bits);
        raw >= *low && raw <= *high
    })
}

/// One pixel, as straight-alpha RGBA8.
///
/// Split out of [`unpack`] so that the loop above is the *layout* — rows, byte boundaries,
/// bit packing — and this is the colour, which is the only part that depends on the space.
pub(super) fn sample_rgba(
    samples: &Samples,
    palette: Option<&[pdf_render::Color]>,
    cache: &mut Option<SampleMemo>,
    row: &[u8],
    x: usize,
    at: usize,
) -> [u8; 4] {
    let &Samples {
        bits,
        space,
        decode,
        colour_key: _,
        fill,
        matte,
        ..
    } = samples;
    let opaque =
        |colour: pdf_render::Color| [channel(colour.r), channel(colour.g), channel(colour.b), 255];
    // §11.6.5.2's inversion, "independently for each component" and before the space is
    // consulted, which is the clause's own ordering: "inversion of the pre-blending shall
    // precede the colour conversion". `at` is the pixel's position on the parent's grid,
    // which Table 143 makes the mask's grid too wherever a `/Matte` is present.
    let restored = |mut values: Vec<f32>| -> Vec<f32> {
        if let Some(matte) = matte {
            matte.restore(&mut values, at, &|component| space.permitted(component));
        }
        values
    };
    match (space, bits) {
        (ColourSpace::Mask, _) => {
            // §8.9.6.2: a sample decoding to 0 marks the page, so the default `[0 1]` paints
            // through the *clear* bits and `[1 0]` reverses that. Both are the same lookup.
            let bit = usize::from(sample_bit(row, x));
            let paint = decode.value(0, bit) < 0.5;
            if paint {
                [
                    channel(fill.r),
                    channel(fill.g),
                    channel(fill.b),
                    channel(fill.a),
                ]
            } else {
                [0, 0, 0, 0]
            }
        }
        // `Decode::channel` is `channel` applied to `Decode::value` and tabulated, so the two
        // arms of each pair are one answer; the matte'd one recomputes what the table holds,
        // because §11.6.5.2's inversion stands between the two steps.
        (ColourSpace::Gray, _) if matte.is_none() => {
            let value = decode.channel(0, index_of(row, x, bits));
            [value, value, value, 255]
        }
        (ColourSpace::Gray, _) => {
            let value = channels(&restored(vec![decode.value(0, index_of(row, x, bits))]))[0];
            [value, value, value, 255]
        }
        (ColourSpace::Rgb, _) if matte.is_none() => {
            let read =
                |component: usize| decode.channel(component, rgb_index(row, x, bits, component));
            [read(0), read(1), read(2), 255]
        }
        (ColourSpace::Rgb, _) => {
            let read =
                |component: usize| decode.value(component, rgb_index(row, x, bits, component));
            let rgb = channels(&restored(vec![read(0), read(1), read(2)]));
            [rgb[0], rgb[1], rgb[2], 255]
        }
        (ColourSpace::Cmyk, _) => {
            let first = x.saturating_mul(4);
            let read = |offset: usize| {
                decode.value(offset, index_of(row, first.saturating_add(offset), bits))
            };
            // The *same* conversion a `k` operator or an `scn` in DeviceCMYK gets. Having a
            // second one here is how the same colour came to render differently depending on
            // whether it was drawn as a fill or as an image, which is exactly the bug this
            // crate should not have.
            opaque(crate::colour::ColourSpace::Cmyk.to_rgb(&restored(vec![
                read(0),
                read(1),
                read(2),
                read(3),
            ])))
        }
        (ColourSpace::Resolved(_), _) if palette.is_some() => {
            let sample = index_of(row, x, bits);
            // The table already carries `/Decode`'s map, so the sample is the index and
            // nothing else happens to it here.
            opaque(
                palette
                    .and_then(|table| table.get(sample))
                    .copied()
                    .unwrap_or(pdf_render::Color::BLACK),
            )
        }
        (ColourSpace::Resolved(resolved), _)
            if matte.is_some() && resolved.indexed_base().is_some() =>
        {
            opaque(indexed_entry_sample(
                resolved,
                samples,
                decode.value(0, index_of(row, x, bits)),
                at,
            ))
        }
        (ColourSpace::Resolved(resolved), _) => opaque(resolved_sample(
            resolved,
            samples,
            (row, x),
            cache,
            &restored,
        )),
    }
}

/// Three component values as the channels a raster holds, in [`Decode::channel`]'s own terms.
fn channels(values: &[f32]) -> [u8; 3] {
    std::array::from_fn(|index| channel(values.get(index).copied().unwrap_or(0.0)))
}

/// Where component `component` of pixel `x` sits among a three-component row's samples.
fn rgb_index(row: &[u8], x: usize, bits: u32, component: usize) -> usize {
    index_of(row, x.saturating_mul(3).saturating_add(component), bits)
}

/// One pixel of an `Indexed` image whose table entries carry Table 144's matte.
///
/// §11.6.5.2's inversion runs on "the colour values in the colour table (not the index values
/// themselves)", so the entry an index selects is what is restored, in the base space's
/// components — after which it is painted as a colour of that space rather than as an index of
/// this one. `at` is the pixel's position on the parent's grid, which Table 143 makes the mask's
/// grid too wherever a `/Matte` is present. ADR 1268.
fn indexed_entry_sample(
    space: &crate::colour::ColourSpace,
    samples: &Samples,
    index: f32,
    at: usize,
) -> pdf_render::Color {
    let base = space.indexed_base().unwrap_or(space);
    let mut values = space.entry_of(&[index]);
    if let Some(matte) = samples.matte {
        matte.restore(&mut values, at, &|component| {
            base.component_range(component)
        });
    }
    samples.into.paint(base, &values)
}

/// One pixel of a space the colour module resolves, converted through it.
///
/// Reached only for a space taking more than one component; the one-component spaces are a
/// table lookup, built by [`palette`].
fn resolved_sample(
    space: &crate::colour::ColourSpace,
    samples: &Samples,
    (row, x): (&[u8], usize),
    cache: &mut Option<SampleMemo>,
    restored: &dyn Fn(Vec<f32>) -> Vec<f32>,
) -> pdf_render::Color {
    let &Samples {
        bits, decode, into, ..
    } = samples;
    let count = space.components();
    let at = x.saturating_mul(count);
    // The raw samples, before `/Decode`, which is what makes them a key: the map from a
    // tuple of samples to a colour is fixed for the whole image. Eight bits per component
    // and at most four components is 32 bits, which the tag then sits above — so at sixteen
    // bits per component the key cannot hold the samples and the caller supplies no cache at
    // all rather than a lossy one.
    //
    // **The tag is not decoration** (ADR 0220). A key seeded with the tag and then shifted
    // left eight bits per component would push it out of the word at four components, and an
    // all-zero sample tuple would key on 0 — which is what [`SampleMemo`]'s slots hold before
    // anything is put in one, so `get` would answer a *hit* out of an empty table and every
    // such pixel would come back `Color::BLACK`: a `DeviceN` of four colourants at no tint, or
    // a `DeviceCMYK` image inside a `/Luminosity` mask group, where zero ink is white. So the
    // bytes are packed into the low half and the tag stays in the high one.
    let mut packed = 0u32;
    let values: Vec<f32> = (0..count)
        .map(|component| {
            // One sample per *component*, so at depths below eight the components of a
            // pixel are adjacent groups of bits rather than adjacent bytes.
            let raw = index_of(row, at.saturating_add(component), bits);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "masked to one byte before the cast"
            )]
            {
                packed = (packed << 8) | ((raw & 0xFF) as u32);
            }
            decode.value(component, raw)
        })
        .collect();
    let key = (1u64 << 32) | u64::from(packed);
    if let Some(cache) = cache.as_ref()
        && let Some(colour) = cache.get(key)
    {
        return colour;
    }
    let colour = into.paint(space, &restored(values));
    if let Some(cache) = cache.as_mut() {
        cache.put(key, colour);
    }
    colour
}

/// Every colour a one-component space can produce at this bit depth, in sample order.
///
/// The `/Decode` map is baked in, so the table is indexed by the raw sample and the
/// per-pixel path does nothing but index it.
pub(super) fn palette(
    space: &crate::colour::ColourSpace,
    bits: u32,
    decode: &Decode,
    into: &Conversion,
) -> Vec<pdf_render::Color> {
    let max = (1u32 << bits.min(8)).saturating_sub(1);
    debug_assert!(bits <= 8, "the caller keeps a 16-bit image off this path");
    (0..=max)
        .map(|raw| into.paint(space, &[decode.value(0, raw as usize)]))
        .collect()
}

/// Reads bit `index` of a packed row, most significant bit first.
fn sample_bit(row: &[u8], index: usize) -> bool {
    let byte = row.get(index / 8).copied().unwrap_or(0);
    let shift = 7u32.saturating_sub(u32::try_from(index % 8).unwrap_or(0));
    (byte >> shift) & 1 == 1
}

/// [`raw_sample`] as a `usize`, which is what `Decode`'s tables are indexed by.
pub(super) fn index_of(row: &[u8], index: usize, bits: u32) -> usize {
    raw_sample(row, index, bits) as usize
}

/// The `index`-th component of a row, at any of §8.9.5.1 Table 87's five bit depths.
///
/// > The value shall be 1 , 2 , 4 , 8 , or (from PDF 1.5) 16 .
///
/// Samples are packed continuously across a row with no padding between them, and a row
/// starts on a byte boundary — which is why the caller computes `row_bytes` and this reads
/// only within one row. The three sub-byte depths divide a byte exactly, so a sample never
/// straddles one; 16 bits is two bytes, most significant first, as every multi-byte integer
/// in a PDF is.
fn raw_sample(row: &[u8], index: usize, bits: u32) -> u32 {
    let byte = |at: usize| u32::from(row.get(at).copied().unwrap_or(0));
    match bits {
        1 => u32::from(sample_bit(row, index)),
        16 => {
            let at = index.saturating_mul(2);
            (byte(at) << 8) | byte(at.saturating_add(1))
        }
        8 => byte(index),
        // 2 and 4: `per` samples to a byte, most significant first.
        _ => {
            let per = usize::try_from(8u32.checked_div(bits).unwrap_or(8))
                .unwrap_or(8)
                .max(1);
            let at = index.checked_div(per).unwrap_or(0);
            let within = u32::try_from(index.checked_rem(per).unwrap_or(0)).unwrap_or(0);
            let shift = 8u32
                .saturating_sub(bits)
                .saturating_sub(within.saturating_mul(bits));
            (byte(at) >> shift) & (1u32 << bits).saturating_sub(1)
        }
    }
}

/// Packs one-bit samples into rows, most significant bit first.
///
/// The inverse of what [`unpack`] reads, so that a codec delivering a byte per pixel can
/// still take the ordinary 1-bit path rather than growing a parallel one.
pub(super) fn pack_bits(samples: &[u8], width: u32, height: u32) -> Vec<u8> {
    let row_bytes = (width as usize).saturating_add(7) / 8;
    let mut packed = vec![0u8; row_bytes.saturating_mul(height as usize)];
    for y in 0..(height as usize) {
        for x in 0..(width as usize) {
            let index = y.saturating_mul(width as usize).saturating_add(x);
            if samples.get(index).copied().unwrap_or(0) != 0 {
                let at = y.saturating_mul(row_bytes).saturating_add(x / 8);
                if let Some(byte) = packed.get_mut(at) {
                    *byte |= 0x80u8 >> (x % 8);
                }
            }
        }
    }
    packed
}
