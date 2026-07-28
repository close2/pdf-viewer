//! The two filters, as the worker implements them.
//!
//! This module is the only place in the tree where a JBIG2 or JPEG 2000 codestream is
//! looked at, and it runs after [`crate::lockdown::apply`] has taken away everything the
//! process could do with what it finds there.
//!
//! Its job is the *filter*, not the codec. `hayro-jbig2` and `hayro-jpeg2000` implement
//! ITU-T T.88 and T.800; ISO 32000-2 §7.4.7 and §7.4.9 say what a PDF filter built on them
//! delivers, and the difference between those two statements is what is written here — the
//! embedded segment organisation, the sense of a bilevel sample, the eight-bit sample
//! format, and the bounds.
//!
//! # Where the boundary of responsibility is
//!
//! Nothing here decides what a colour *means*. A JPEG 2000 codestream's own colour space
//! comes back out unevaluated, because §7.4.9 makes choosing between it and the image
//! dictionary's `/ColorSpace` a question about the PDF, and answering it needs the image
//! dictionary — which is on the other side of this boundary, deliberately.

use crate::protocol::{Bilevel, Colour, Raster, Request};
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
/// Samples rather than pixels, because a JPEG 2000 decoder's working set scales with both.
/// It holds each component as `f32` while the wavelet runs and then writes an eight-bit
/// interleaved copy, so roughly five bytes per sample; 2^27 samples is therefore around
/// 670 megabytes, which leaves headroom inside the gigabyte for the codestream, the
/// code-block buffers and the allocator's own slack.
///
/// This bound is *reached* by real files, and that is worth knowing rather than hiding.
/// `issue19517.pdf` carries a 12608×16806 scan — 212 megapixels in three components, 636
/// million samples, several gigabytes to decode at full resolution — for a page that will
/// be drawn about four megapixels. Refusing it here reports one undrawn image; the proper
/// answer is JPEG 2000's own, which is to decode a reduced resolution level, and that needs
/// the scale a page is about to be drawn at to reach this crate. See the note on
/// `target_resolution` below.
const MAX_SAMPLES: u64 = 1 << 27;

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

    let mut rows = PackedRows::new(width, height);
    image
        .decode(&mut rows)
        .map_err(|error| format!("JBIG2: {error}"))?;
    rows.finish()
}

/// Packs decoded pixels into the rows the `JBIG2Decode` filter delivers.
///
/// Two things are happening at once here, and they are easy to conflate:
///
/// - **Packing.** One bit per pixel, most significant bit first, each row starting on a
///   byte boundary. This is what ISO 32000-2 Table 87 requires of the filter's output, and
///   it is what the image pipeline already unpacks for every other 1-bit image, so JBIG2
///   needs no special case above this point.
/// - **Inverting.** ISO/IEC 14492 gives a *set* pixel the value 1 and calls it black.
///   `DeviceGray` gives black the value 0. The two conventions are opposite, so the bit
///   written here is the complement of the bit decoded. Getting this backwards produces a
///   photographic negative of a scanned page — wrong in a way that is obvious on sight and
///   invisible to every metric, which is why it is stated rather than implied.
struct PackedRows {
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
    fn new(width: u32, height: u32) -> Self {
        let row_bytes = (width as usize).saturating_add(7) / 8;
        Self {
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
                "JBIG2: the decoder produced {} packed bytes where {expected} were expected",
                self.rows.len()
            ));
        }
        Ok(Bilevel {
            width: self.width,
            height: self.height,
            rows: self.rows,
        })
    }
}

impl hayro_jbig2::Decoder for PackedRows {
    fn push_pixel(&mut self, black: bool) {
        if !black {
            self.partial |= 0x80u8 >> self.filled.min(7);
        }
        self.filled = self.filled.saturating_add(1);
        if self.filled == 8 {
            self.rows.push(self.partial);
            self.partial = 0;
            self.filled = 0;
        }
    }

    fn push_pixel_chunk(&mut self, black: bool, chunk_count: u32) {
        // The contract says this arrives only on a byte boundary. Honouring that rather
        // than assuming it costs one branch and means a future change upstream degrades
        // into slower packing instead of into shifted pixels.
        if self.filled != 0 {
            for _ in 0..chunk_count.saturating_mul(8) {
                self.push_pixel(black);
            }
            return;
        }
        let byte = if black { 0x00 } else { 0xFF };
        self.rows
            .extend(std::iter::repeat_n(byte, chunk_count as usize));
    }

    fn next_line(&mut self) {
        if self.filled != 0 {
            // The bits past the image's width are not part of any sample: the unpacker
            // reads exactly `/Width` of them per row and stops. They are left clear.
            self.rows.push(self.partial);
            self.partial = 0;
            self.filled = 0;
        }
    }
}

/// Decodes a JPEG 2000 image.
///
/// ISO 32000-2 §7.4.9: the filter reads a whole JP2 file structure, or — as the corpus
/// shows real producers writing — a bare codestream. Samples come back at eight bits
/// whatever the codestream's precision, because the depth is the decoder's to determine
/// (§7.4.9 and Table 87's note on `BitsPerComponent`) and everything above this is
/// eight-bit.
///
/// # Errors
///
/// Returns a description of what the decoder refused.
pub(crate) fn jpx(data: &[u8], indices: bool) -> Result<Raster, String> {
    use hayro_jpeg2000::{ColorSpace, DecodeSettings, DecoderContext, Image};

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
        // Full resolution. Decoding a reduced level is the obvious lever for a thumbnail
        // grid later, and it is deliberately not pulled now: it would need the display
        // list to carry the scale a page is about to be drawn at, which it does not.
        target_resolution: None,
    };

    let image = Image::new(data, &settings).map_err(|error| format!("JPX: {error}"))?;
    let (width, height) = (image.width(), image.height());
    // Checked before decoding rather than after: the whole point is to answer without
    // having allocated what the answer is about.
    let declared_channels = u64::from(image.color_space().num_channels())
        .saturating_add(u64::from(image.has_alpha()))
        .max(1);
    let samples = u64::from(width)
        .saturating_mul(u64::from(height))
        .saturating_mul(declared_channels);
    if samples > MAX_SAMPLES {
        return Err(format!(
            "JPX: {width}x{height} in {declared_channels} channels is {samples} samples, \
             beyond the {MAX_SAMPLES} this decoder is given room for"
        ));
    }

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
        components,
        has_opacity,
        colour,
        data: decoded.data_u8(),
    })
}
