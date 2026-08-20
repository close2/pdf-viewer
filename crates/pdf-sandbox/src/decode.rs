//! The three image filters, as the worker implements them.
//!
//! This module is the only place in the tree where a JBIG2, JPEG 2000 or CCITT codestream is
//! looked at, and it runs after [`crate::lockdown::apply`] has taken away everything the
//! process could do with what it finds there.
//!
//! Its job is the *filter*, not the codec. `hayro-jbig2`, `hayro-jpeg2000` and `hayro-ccitt`
//! implement ITU-T T.88, T.800 and T.4/T.6; ISO 32000-2 §7.4.6, §7.4.7 and §7.4.9 say what a
//! PDF filter built on them delivers, and the difference between those two statements is what
//! is written here — the embedded segment organisation, the sense of a bilevel sample, the
//! eight-bit sample format, and the bounds.
//!
//! # Where the boundary of responsibility is
//!
//! Nothing here decides what a colour *means*. A JPEG 2000 codestream's own colour space
//! comes back out unevaluated, because §7.4.9 makes choosing between it and the image
//! dictionary's `/ColorSpace` a question about the PDF, and answering it needs the image
//! dictionary — which is on the other side of this boundary, deliberately.

use crate::protocol::{Bilevel, CcittParameters, Colour, Raster, Request};
use crate::{Decoded, SandboxError};

/// Decodes in this process, with no confinement.
///
/// The path [`crate::Isolation::InProcess`] selects. It is the same two functions the worker
/// calls, so the two settings cannot diverge in what they produce — only in what happens
/// when one of them goes wrong.
pub(crate) fn here(request: &Request<'_>) -> Result<Decoded, SandboxError> {
    let decoded = match request {
        Request::Jbig2 { data, globals } => jbig2(data, globals).map(Decoded::Bilevel),
        Request::Jpx { data, indices } => jpx(data, *indices).map(Decoded::Raster),
        Request::Ccitt { data, parameters } => ccitt(data, *parameters).map(Decoded::Bilevel),
    };
    decoded.map_err(|detail| SandboxError::Undecodable { detail })
}

/// Largest bilevel image this will produce, in pixels.
///
/// The same bound `pdf-model` applies to every other image, restated here because this
/// process must not depend on the caller having applied it: the request arrives over a
/// pipe, and a pipe carries no invariants. Crossing it is refused cheaply, before the
/// address-space limit has to end the process the expensive way.
///
/// A bilevel decoder holds about a byte per pixel, so 2^28 pixels is comfortably inside
/// [`crate::lockdown`]'s gigabyte.
const MAX_PIXELS: u64 = 1 << 28;

/// Largest continuous-tone image this will produce, in *samples* — pixels times channels.
///
/// Samples rather than pixels, because a JPEG 2000 decoder's working set scales with both:
/// the decoder holds every component's coefficients as `f32` through the wavelet, the
/// synthesised samples as `f32` beside them, two more buffers for the transform itself, and
/// then an eight-bit interleaved copy.
///
/// **This said "roughly five bytes per sample" and 2^27 until the three-hundred-and-ninety-sixth
/// session, and the estimate was wrong by a factor of two in the direction that matters** — the
/// bound exists to refuse cheaply *before* the address-space limit has to end the process the
/// expensive way, and at 2^27 it admitted an image that ends it. Measured rather than estimated,
/// on codestreams built for the purpose and decoded through the same crate this worker uses:
///
/// | codestream | samples | peak address space | inside [`crate::lockdown`]'s gigabyte |
/// |---|---|---|---|
/// | 4096×4096, four channels | 2^26 | 600 MB | yes |
/// | 6690×6690, three channels | 2^27 + 50 572 | 1253 MB | **no — the allocation fails** |
///
/// So the cost is nine to thirteen bytes a sample rather than five, and 2^26 is the bound with
/// the measurement behind it: 600 MB leaves the rest of the gigabyte for the codestream, the
/// code-block buffers and the allocator's slack. No corpus codestream is between the two — the
/// largest that decodes is 2.4 million samples — so this narrows a bound nothing reaches rather
/// than refusing anything that was drawing.
///
/// This bound is *reached* by real files, and what happens then depends on the codec.
/// `issue19517.pdf` carries a 12608×16806 scan — 212 megapixels in four channels, 847 million
/// samples, ten gigabytes to decode at full resolution — for a page that will be drawn about
/// four megapixels. JPEG 2000 answers this itself: [`jpx`] steps down the codestream's own
/// resolution progression until the sample count fits, so that file decodes at 3152×4202
/// instead of being refused. Only a codestream that cannot get under the bound at any level
/// is refused.
const MAX_SAMPLES: u64 = 1 << 26;

/// Decodes a JBIG2 image, in the embedded organisation PDF requires.
///
/// ISO 32000-2 §7.4.7: the stream holds the segments for one page, the file header and the
/// end-of-page and end-of-file segments are absent, and page-0 segments live in a separate
/// `/JBIG2Globals` stream which several images may share.
///
/// # Errors
///
/// Returns a description of what the decoder refused, which the page reports verbatim.
pub(crate) fn jbig2(data: &[u8], globals: &[u8]) -> Result<Bilevel, String> {
    let globals = (!globals.is_empty()).then_some(globals);
    let image = hayro_jbig2::Image::new_embedded(data, globals)
        .map_err(|error| format!("JBIG2: {error}"))?;

    let (width, height) = (image.width(), image.height());
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if pixels > MAX_PIXELS {
        return Err(format!(
            "JBIG2: {width}x{height} exceeds {MAX_PIXELS} pixels"
        ));
    }

    let mut rows = PackedRows::new("JBIG2", width, height);
    image
        .decode(&mut rows)
        .map_err(|error| format!("JBIG2: {error}"))?;
    rows.finish()
}

/// Packs decoded pixels into the rows a bilevel filter delivers.
///
/// **Packing only.** One bit per pixel, most significant bit first, each row starting on a
/// byte boundary — what ISO 32000-2 Table 87 requires of a bilevel filter's output, and what
/// the image pipeline already unpacks for every other 1-bit image, so neither filter needs a
/// special case above this point.
///
/// **What a bit *means* belongs to each filter, not here**, because the two disagree and each
/// disagrees with `DeviceGray`:
///
/// - ISO/IEC 14492 gives a *set* JBIG2 pixel the value 1 and calls it black, where
///   `DeviceGray` gives black 0. So the [`hayro_jbig2::Decoder`] impl below writes the
///   complement of what it is handed.
/// - `CCITTFaxDecode` has no fixed answer: §7.4.6 Table 11's `/BlackIs1` decides it per
///   image, and defaults to the PDF convention. So [`CcittRows`] applies that entry.
///
/// Getting either backwards produces a photographic negative of a scanned page — wrong in a
/// way that is obvious on sight and invisible to every metric, which is why the two rules are
/// stated separately rather than folded into one shared inversion.
struct PackedRows {
    /// Which filter this is packing for, so a length complaint names it.
    filter: &'static str,
    /// Width in pixels, which is not in general a whole number of bytes.
    width: u32,
    /// Bytes in one packed row.
    row_bytes: usize,
    /// Rows expected, so a short or long decode is caught rather than reshaped.
    height: u32,
    /// The packed output.
    rows: Vec<u8>,
    /// The byte being filled.
    partial: u8,
    /// How many bits of `partial` are filled, always less than eight.
    filled: u32,
}

impl PackedRows {
    fn new(filter: &'static str, width: u32, height: u32) -> Self {
        let row_bytes = (width as usize).saturating_add(7) / 8;
        Self {
            filter,
            width,
            row_bytes,
            height,
            rows: Vec::with_capacity(row_bytes.saturating_mul(height as usize)),
            partial: 0,
            filled: 0,
        }
    }

    /// Returns the packed image, or says why it is not one.
    ///
    /// The length check is the one thing this type can assert that the decoder above it
    /// cannot: a bilevel image that is one row short unpacks into a page that looks right
    /// and is wrong at the bottom edge, and no comparison metric would name the cause.
    fn finish(self) -> Result<Bilevel, String> {
        let expected = self.row_bytes.saturating_mul(self.height as usize);
        if self.rows.len() != expected {
            return Err(format!(
                "{}: the decoder produced {} packed bytes where {expected} were expected",
                self.filter,
                self.rows.len()
            ));
        }
        Ok(Bilevel {
            width: self.width,
            height: self.height,
            rows: self.rows,
        })
    }

    /// Appends one sample, `one` being the bit the filter delivers rather than a colour.
    fn push_bit(&mut self, one: bool) {
        if one {
            self.partial |= 0x80u8 >> self.filled.min(7);
        }
        self.filled = self.filled.saturating_add(1);
        if self.filled == 8 {
            self.rows.push(self.partial);
            self.partial = 0;
            self.filled = 0;
        }
    }

    /// Appends `count` samples of the same value, `one` being the bit rather than a colour.
    ///
    /// A run may begin and end anywhere in a byte, so the three parts are separate: the bits
    /// that finish the byte in progress, the whole bytes, and the bits that start the next
    /// one. Only the middle part is a `memset`, and it is the part that matters — a 600 dpi
    /// scan line is mostly one long white run — but the packing has to be right at both ends
    /// or every pixel after the first run is shifted.
    fn push_run(&mut self, one: bool, count: u32) {
        // `filled` is always less than eight, so this is zero exactly on a byte boundary.
        let to_boundary = (8_u32.saturating_sub(self.filled) % 8).min(count);
        for _ in 0..to_boundary {
            self.push_bit(one);
        }
        let mut remaining = count.saturating_sub(to_boundary);
        let whole = remaining / 8;
        if whole > 0 {
            let byte = if one { 0xFF } else { 0x00 };
            self.rows.extend(std::iter::repeat_n(byte, whole as usize));
            remaining = remaining.saturating_sub(whole.saturating_mul(8));
        }
        for _ in 0..remaining {
            self.push_bit(one);
        }
    }

    /// Closes the current row, padding the last byte.
    fn end_row(&mut self) {
        if self.filled != 0 {
            // The bits past the image's width are not part of any sample: the unpacker
            // reads exactly `/Width` of them per row and stops. They are left clear.
            self.rows.push(self.partial);
            self.partial = 0;
            self.filled = 0;
        }
    }

    /// Fills any rows the decoder never produced with the sample value `one`.
    ///
    /// Only `CCITTFaxDecode` uses this, and only because a decode that ends early is a legal
    /// decode rather than a damaged one — for either of two reasons, which end here alike: the
    /// data ran out, or Table 11's `/Rows` bounded the filter below the image's height under an
    /// `/EndOfBlock` of false. ISO 32000-2 §7.4.6 Table 11 says the first twice over, and
    /// the *unconditional* half is the `/Rows` row — "the encoded data shall be terminated by
    /// an end-of-block bit pattern or by the end of the filter's data". (The `/EndOfBlock`
    /// row's "whichever occurs first" says it again and used to be quoted here instead, but
    /// that sentence opens with "If false" and is about which of `/Rows` and the end-of-block
    /// pattern *bounds* the decode. Which one that is now belongs to `pdf_model::ccitt_rows`,
    /// where the two numbers are known; here the count has already been decided and all that
    /// is left is what an undelivered line shows.) That is not stated anywhere in ISO 32000-2,
    /// so it is a choice, and it is made here: blank, which is what an unsent fax scan line is.
    fn pad_to_height(&mut self, one: bool) {
        self.end_row();
        let expected = self.row_bytes.saturating_mul(self.height as usize);
        let byte = if one { 0xFF } else { 0x00 };
        while self.rows.len() < expected {
            self.rows.push(byte);
        }
    }
}

impl hayro_jbig2::Decoder for PackedRows {
    fn push_pixel(&mut self, black: bool) {
        self.push_bit(!black);
    }

    fn push_pixel_chunk(&mut self, black: bool, chunk_count: u32) {
        // This decoder still counts in eight-pixel chunks, so the count is converted here
        // rather than in the packer, which counts pixels for both filters.
        self.push_run(!black, chunk_count.saturating_mul(8));
    }

    fn next_line(&mut self) {
        self.end_row();
    }
}

/// Decodes a CCITT fax-encoded image.
///
/// ISO 32000-2 §7.4.6: Group 3 or Group 4 encoding as ITU-T T.4 and T.6 define it, with
/// Table 11's parameters deciding which. Everything PDF-specific about it is in these thirty
/// lines, because `hayro-ccitt` implements the two ITU recommendations and nothing else:
///
/// - **Which scheme.** Table 11 says the filter "shall distinguish among negative, zero, and
///   positive values of K to determine how to interpret the encoded data; however, it shall
///   not distinguish between different positive K values" — so the sign selects, and the
///   magnitude is carried through for the mixed mode's own use rather than compared.
/// - **The sense of a bit**, which `/BlackIs1` decides. See [`CcittRows`].
/// - **Where the decode stops and where the image ends**, which are two numbers and not one:
///   [`CcittParameters::rows`] bounds the filter and [`CcittParameters::height`] is the grid
///   §8.9.5.1 states. Table 11 lets the first fall short of the second on purpose — with
///   `/EndOfBlock` false "the filter shall stop when it has decoded the number of lines
///   indicated by Rows or when its data has been exhausted, whichever occurs first" — and what
///   the rows between them show is [`PackedRows::pad_to_height`]'s paragraph.
///
/// # Errors
///
/// Returns a description of what the decoder refused. A malformed stream is reported rather
/// than partially drawn: the decoder can leave usable rows behind an error, and taking them
/// would be a page that is silently missing its bottom half.
pub(crate) fn ccitt(data: &[u8], parameters: CcittParameters) -> Result<Bilevel, String> {
    use hayro_ccitt::{DecodeSettings, DecoderContext, EncodingMode};

    let (width, height, bound) = (parameters.columns, parameters.height, parameters.rows);
    // Both numbers, because either can be the larger: the decode may legitimately stop short of
    // the image (Table 11's `/Rows` under an `/EndOfBlock` of false), and a document may equally
    // state more scan lines than its `/Height`, in which case the decoder writes them here.
    let pixels = u64::from(width).saturating_mul(u64::from(height.max(bound)));
    if pixels > MAX_PIXELS {
        return Err(format!(
            "CCITTFaxDecode: {width}x{} exceeds {MAX_PIXELS} pixels",
            height.max(bound)
        ));
    }

    let settings = DecodeSettings {
        columns: width,
        rows: bound,
        end_of_block: parameters.end_of_block,
        end_of_line: parameters.end_of_line,
        rows_are_byte_aligned: parameters.encoded_byte_align,
        encoding: match parameters.k {
            ..0 => EncodingMode::Group4,
            0 => EncodingMode::Group3_1D,
            k => EncodingMode::Group3_2D {
                k: k.unsigned_abs(),
            },
        },
        // Left false, and `/BlackIs1` applied in `CcittRows` instead: the decoder's flag and
        // the PDF entry mean the same thing here, and having one place that turns a colour
        // into a sample keeps the clause beside the line that implements it.
        invert_black: false,
    };

    let mut rows = CcittRows {
        packed: PackedRows::new("CCITTFaxDecode", width, height),
        black_is_1: parameters.black_is_1,
    };
    hayro_ccitt::decode(data, &mut rows, &mut DecoderContext::new(settings))
        .map_err(|error| format!("CCITTFaxDecode: {error}"))?;

    // White, in whichever sense this image's `/BlackIs1` gives the word.
    rows.packed.pad_to_height(!parameters.black_is_1);
    rows.packed.finish()
}

/// Packs a CCITT decode, applying `/BlackIs1`.
///
/// ISO 32000-2 §7.4.6 Table 11 defines the entry as
///
/// > A flag indicating whether bits with a value of 1 shall be interpreted as black pixels
/// > and 0 bits as white pixels, the reverse of the normal PDF syntactic convention for image
/// > data. Default value: false .
///
/// So a white pixel is the bit 1 by default and the bit 0 when `/BlackIs1` is true, which is
/// the single exclusive-or below. Unlike JBIG2's inversion this is not a property of the
/// codec — the same encoded bytes mean opposite pictures in two documents that differ only in
/// this entry.
struct CcittRows {
    packed: PackedRows,
    black_is_1: bool,
}

impl hayro_ccitt::Decoder for CcittRows {
    fn push_pixels(&mut self, white: bool, count: u32) {
        self.packed.push_run(white != self.black_is_1, count);
    }

    fn next_line(&mut self) {
        self.packed.end_row();
    }
}

/// Parses a JPEG 2000 codestream at the finest resolution level [`MAX_SAMPLES`] admits.
///
/// Returns the parse — no sample is decoded here, each step is a header read — together with
/// the grid the codestream *states*, taken from the first, unreduced parse: a reduced parse
/// answers `width()` with the level it will synthesise rather than with what the data says,
/// and §7.4.9's conformance check on the caller's side needs the statement.
///
/// The loop asks for half of what the last reading offered. `target_resolution` selects the
/// finest level *at least* that large, so the request is relative to the current size rather
/// than an absolute: each accepted step is one more decomposition level skipped, and a
/// request the codestream cannot better is how the loop learns the levels have run out —
/// either because the codestream has no more of them, or because it is a palettised JP2,
/// whose indices a reduced decode would corrupt and whose decoder therefore declines the
/// request. Both end in the same refusal, naming the stated grid.
///
/// # Errors
///
/// Returns a description of what the decoder refused, or the refusal above.
fn jpx_within_budget(
    data: &[u8],
    settings: hayro_jpeg2000::DecodeSettings,
) -> Result<(hayro_jpeg2000::Image<'_>, (u32, u32)), String> {
    use hayro_jpeg2000::{DecodeSettings, Image};

    let mut image = Image::new(data, &settings).map_err(|error| format!("JPX: {error}"))?;
    let stated = (image.width(), image.height());
    // Checked before decoding rather than after: the whole point is to answer without
    // having allocated what the answer is about.
    let declared_channels = u64::from(image.color_space().num_channels())
        .saturating_add(u64::from(image.has_alpha()))
        .max(1);
    let count = |width: u32, height: u32| {
        u64::from(width)
            .saturating_mul(u64::from(height))
            .saturating_mul(declared_channels)
    };
    let mut samples = count(image.width(), image.height());
    while samples > MAX_SAMPLES {
        let target = ((image.width() / 2).max(1), (image.height() / 2).max(1));
        let reduced_settings = DecodeSettings {
            target_resolution: Some(target),
            ..settings
        };
        let reduced =
            Image::new(data, &reduced_settings).map_err(|error| format!("JPX: {error}"))?;
        let reduced_samples = count(reduced.width(), reduced.height());
        if reduced_samples >= samples {
            return Err(format!(
                "JPX: {}x{} in {declared_channels} channels is {} samples, beyond the \
                 {MAX_SAMPLES} this decoder is given room for, and no resolution level of \
                 the codestream fits it",
                stated.0,
                stated.1,
                count(stated.0, stated.1)
            ));
        }
        image = reduced;
        samples = reduced_samples;
    }
    Ok((image, stated))
}

/// Decodes a JPEG 2000 image, stepping down resolution levels until it fits the budget.
///
/// ISO 32000-2 §7.4.9: the filter reads a whole JP2 file structure, or — as the corpus
/// shows real producers writing — a bare codestream. Samples come back at eight bits
/// whatever the codestream's precision, because the depth is the decoder's to determine
/// (§7.4.9 and Table 87's note on `BitsPerComponent`) and everything above this is
/// eight-bit.
///
/// # A codestream over the budget is decoded at a reduced resolution level
///
/// §7.4.9 NOTE 3 addresses the answer to an over-budget image to this program by name:
///
/// > Viewing and printing applications can gain performance benefits by using the resolution
/// > progression. If the full-resolution image is densely sampled, an application can select
/// > and decode only the data making up a lower-resolution version, thereby spending less
/// > time decoding.
///
/// So where the full image exceeds [`MAX_SAMPLES`], the codestream is re-read asking for half
/// of what the last reading offered, until the sample count fits or the size stops shrinking —
/// which it does when the codestream has no further decomposition levels, and for a palettised
/// JP2, whose indices a reduced decode would corrupt and whose decoder therefore declines the
/// request. Each step is a *header* parse, cheap by construction; no sample is decoded until
/// the loop has settled on a grid the budget admits. The raster that leaves carries the grid it
/// is actually on beside the grid the codestream states, because §7.4.9's "Width and Height
/// shall match" check is against the data's statement, not against what this budget chose to
/// synthesise — see [`Raster::stated_width`].
///
/// The reduction became *usable* with `close2/hayro`'s `feat/reduced-resolution-allocates-less`
/// (the `1dc833f7` revision this workspace pins): before it, asking for a reduced level skipped
/// the bit-planes and the wavelet but still reserved a coefficient for every sample of the
/// full-resolution image — one 3.4 GB allocation for `issue19517.pdf` however small a raster
/// was asked for, which [`crate::lockdown`]'s gigabyte turns into a dead worker rather than a
/// picture. ADR 0233 has the measurements; with the fix, that file's two-levels-down decode
/// peaks at 743 MB of address space.
///
/// # Errors
///
/// Returns a description of what the decoder refused.
pub(crate) fn jpx(data: &[u8], indices: bool) -> Result<Raster, String> {
    use hayro_jpeg2000::{ColorSpace, DecodeSettings, DecoderContext};

    let settings = DecodeSettings {
        // A palette in the codestream is the JPEG 2000 equivalent of an `Indexed` colour
        // space (§7.4.9), and resolving it here means one kind of image leaves this
        // process: direct colour. The alternative — passing indices out and rebuilding the
        // palette in `pdf-model` — would put a second indexed-colour path in a crate that
        // already has one.
        //
        // The exception is a caller that *wants* the indices, because the image dictionary
        // declared its own `Indexed` space and §7.4.9 gives it precedence over anything the
        // codestream says. `issue12213.pdf` is that file.
        resolve_palette_indices: !indices,
        // Not strict. A PDF viewer that refuses a codestream every other viewer reads has
        // chosen the wrong failure: the file is what it is, and the alternative to
        // repairing a producer's mistake is a blank space where the page has a photograph.
        // Nothing repaired here is a *colour* decision — those all leave as `Unknown` and
        // are decided against §7.4.9 by the caller.
        strict: false,
        // Full resolution first; `jpx_within_budget` only asks for less where the full
        // image does not fit `MAX_SAMPLES`.
        target_resolution: None,
    };

    let (image, (stated_width, stated_height)) = jpx_within_budget(data, settings)?;
    let (width, height) = (image.width(), image.height());

    let mut context = DecoderContext::default();
    let decoded = image
        .decode(&mut context)
        .map_err(|error| format!("JPX: {error}"))?;
    let channels = decoded.components().len();

    if indices {
        // Raw sample values, not the eight-bit stretch `data_u8` produces. An index is a
        // position in a table, so scaling it is not a loss of precision — it is a different
        // entry. A palette large enough to need more than eight bits cannot be addressed by
        // a PDF `Indexed` space either, whose `hival` is at most 255.
        let component = decoded
            .components()
            .first()
            .ok_or_else(|| "JPX: no component to read indices from".to_owned())?;
        let data = component
            .samples()
            .iter()
            .map(|sample| {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "clamped to 0..=255 before the cast, which then cannot round \
                              anywhere but to an index in range"
                )]
                let index = sample.round().clamp(0.0, 255.0) as u8;
                index
            })
            .collect();
        return Ok(Raster {
            width,
            height,
            stated_width,
            stated_height,
            components: 1,
            has_opacity: false,
            colour: Colour::Unknown,
            data,
        });
    }

    let declared = image.color_space();
    let has_opacity = image.has_alpha();
    let colour_channels = usize::from(declared.num_channels());

    // If the codestream's declared space does not account for the channels that came out,
    // the space is not usable and §7.4.9's own fallback applies: the number of ordinary
    // channels decides, 1, 3 or 4 meaning `DeviceGray`, `DeviceRGB` or `DeviceCMYK`. That
    // decision belongs to the caller, so what leaves here is `Unknown` plus a count.
    let (colour, components) =
        if colour_channels.saturating_add(usize::from(has_opacity)) == channels {
            let colour = match declared {
                ColorSpace::Gray => Colour::Gray,
                ColorSpace::RGB => Colour::Rgb,
                ColorSpace::CMYK => Colour::Cmyk,
                ColorSpace::Icc { profile, .. } => Colour::Icc(profile.clone()),
                ColorSpace::Unknown { .. } => Colour::Unknown,
            };
            (colour, colour_channels)
        } else {
            (
                Colour::Unknown,
                channels.saturating_sub(usize::from(has_opacity)),
            )
        };

    let components = u8::try_from(components)
        .map_err(|_| format!("JPX: {components} colour components is more than any PDF space"))?;
    if components == 0 {
        return Err("JPX: the codestream has no colour components".to_owned());
    }

    Ok(Raster {
        width,
        height,
        stated_width,
        stated_height,
        components,
        has_opacity,
        colour,
        data: decoded.data_u8(),
    })
}
