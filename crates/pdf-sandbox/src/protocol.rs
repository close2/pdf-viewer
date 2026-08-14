//! The wire format between the parent and the worker.
//!
//! Deliberately small. Every byte of this protocol is an interface a subverted worker can
//! push on, so there is one request shape, one response shape, and no negotiation: a field
//! that cannot be misinterpreted is a field that costs nothing to defend.
//!
//! Lengths are big-endian and fixed-width, sizes are checked against what they describe
//! before anything is allocated from them, and the parent validates the worker's answers as
//! carefully as the document's own numbers. The worker is the untrusted side of this
//! boundary — that is the entire point of it being on the other side.

use crate::lockdown::{Confinement, LandlockLevel, SystemCalls};
use crate::{Decoded, SandboxError};

/// Greeting bytes, changed whenever this format changes incompatibly.
///
/// A parent and a worker from different builds must not talk to each other, and the
/// cheapest place to find that out is the first thing either says. `02` since the
/// three-hundred-and-fifteenth session, when the greeting gained the byte that says whether
/// the system-call filter is in force — a worker from the build before it would answer a
/// question this one asks with silence, which is exactly what the magic is for. `03` since
/// the four-hundred-and-eighty-sixth, when a raster response gained the codestream's own
/// stated grid beside the raster's — a parent from the build before would read those eight
/// bytes as sample data.
const MAGIC: &[u8; 8] = b"PDFSBX03";

/// Length of the worker's greeting: the magic, the Landlock level, the address-space limit,
/// and whether system calls are filtered.
pub(crate) const HANDSHAKE_LEN: usize = 8 + 1 + 8 + 1;

/// Length of a response header: the status byte and the payload length.
pub(crate) const RESPONSE_HEADER_LEN: usize = 1 + 4;

/// Length of a request header: the kind, the flags, and two payload lengths.
const REQUEST_HEADER_LEN: usize = 1 + 1 + 4 + 4;

/// Request kind: the `JBIG2Decode` filter.
const KIND_JBIG2: u8 = 1;
/// Request kind: the `JPXDecode` filter.
const KIND_JPX: u8 = 2;
/// Request kind: the `CCITTFaxDecode` filter.
const KIND_CCITT: u8 = 3;

/// Request flag: return palette indices rather than colours. `JPXDecode` only.
const FLAG_INDICES: u8 = 1 << 0;

/// Response status: the decoder refused, and the payload says why.
const STATUS_ERROR: u8 = 0;
/// Response status: one bit per pixel.
const STATUS_BILEVEL: u8 = 1;
/// Response status: eight bits per component.
const STATUS_RASTER: u8 = 2;

/// An image to decode.
///
/// Not `#[non_exhaustive]`, deliberately. These types cross a crate boundary but not a
/// *release* boundary — everything that matches on them is in this workspace — and a new
/// filter should break every match that has to learn about it, which is exactly what
/// `non_exhaustive` would prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request<'a> {
    /// The `JBIG2Decode` filter (ISO 32000-2 §7.4.7).
    Jbig2 {
        /// The image `XObject`'s own stream: the segments for this page.
        data: &'a [u8],
        /// The `/JBIG2Globals` stream, or empty. Table 12.
        globals: &'a [u8],
    },
    /// The `JPXDecode` filter (ISO 32000-2 §7.4.9).
    Jpx {
        /// A JP2 file or a bare JPEG 2000 codestream.
        data: &'a [u8],
        /// Whether the samples are wanted as palette *indices* rather than as colours.
        ///
        /// A JP2 file may carry its own palette, which is the format's equivalent of an
        /// `Indexed` colour space. Normally that palette is applied here and direct colour
        /// comes out. But §7.4.9 says an image dictionary's `/ColorSpace` overrides the
        /// codestream's colour specifications entirely, so a dictionary declaring `Indexed`
        /// wants the raw indices and will do its own lookup — applying the codestream's
        /// palette first would feed a colour to a table expecting an index.
        ///
        /// Indices also come back unscaled. Every other sample is stretched to eight bits;
        /// an index stretched to eight bits is a different index.
        indices: bool,
    },
    /// The `CCITTFaxDecode` filter (ISO 32000-2 §7.4.6).
    Ccitt {
        /// The encoded bit stream, after any filter that precedes this one in the chain.
        data: &'a [u8],
        /// Table 11's parameters, already resolved against their defaults by the caller.
        parameters: CcittParameters,
    },
}

/// ISO 32000-2 §7.4.6 Table 11's optional parameters, resolved.
///
/// Resolved, not raw: every field here has a value, because Table 11 gives every one of them
/// a default and the place that can read a `/DecodeParms` dictionary is the place that can
/// apply those defaults. What crosses the pipe is therefore a complete description of what to
/// decode, which is what lets the worker hold no opinion about PDF at all.
///
/// `/DamagedRowsBeforeError` is deliberately absent: it is the one entry this decoder cannot
/// honour, so it is refused where the dictionary is read rather than dropped here — see
/// `pdf_model::image`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the fields are Table 11's entries and the count is the standard's; a state \
              machine or two-variant enums would make the correspondence unreadable"
)]
pub struct CcittParameters {
    /// `/K`. Negative selects Group 4, zero Group 3 one-dimensional, positive Group 3 mixed.
    pub k: i32,
    /// `/Columns`, the width of the image in pixels. Table 11's default is 1728.
    pub columns: u32,
    /// How many rows to decode.
    ///
    /// `/Rows` where the document gives a non-zero one. Table 11 says a zero or absent
    /// `/Rows` means "the image's height is not predetermined", and the only other statement
    /// of that height is the image dictionary's `/Height` (§8.9.5.1) — which is on the other
    /// side of this pipe, so the caller substitutes it and this field is never zero.
    pub rows: u32,
    /// `/EndOfLine`: whether end-of-line bit patterns are present. Default false.
    pub end_of_line: bool,
    /// `/EncodedByteAlign`: whether each encoded line begins on a byte boundary. Default
    /// false.
    pub encoded_byte_align: bool,
    /// `/EndOfBlock`: whether an end-of-block pattern terminates the data. Default true.
    pub end_of_block: bool,
    /// `/BlackIs1`: whether a 1 bit is black, "the reverse of the normal PDF syntactic
    /// convention for image data". Default false.
    pub black_is_1: bool,
}

impl Default for CcittParameters {
    /// Table 11's defaults, except for [`Self::rows`] — see its documentation.
    fn default() -> Self {
        Self {
            k: 0,
            columns: 1728,
            rows: 0,
            end_of_line: false,
            encoded_byte_align: false,
            end_of_block: true,
            black_is_1: false,
        }
    }
}

/// How many bytes [`CcittParameters`] occupies on the wire.
const CCITT_PARAMETERS_LEN: usize = 4 + 4 + 4 + 1;

/// Bit positions of the four booleans, in Table 11's order.
const CCITT_END_OF_LINE: u8 = 1 << 0;
const CCITT_ENCODED_BYTE_ALIGN: u8 = 1 << 1;
const CCITT_END_OF_BLOCK: u8 = 1 << 2;
const CCITT_BLACK_IS_1: u8 = 1 << 3;

impl CcittParameters {
    /// The fixed-width encoding that travels in a request's auxiliary payload.
    fn encode(self) -> [u8; CCITT_PARAMETERS_LEN] {
        let mut out = [0u8; CCITT_PARAMETERS_LEN];
        let (k, rest) = out.split_at_mut(4);
        k.copy_from_slice(&self.k.to_be_bytes());
        let (columns, rest) = rest.split_at_mut(4);
        columns.copy_from_slice(&self.columns.to_be_bytes());
        let (rows, flags) = rest.split_at_mut(4);
        rows.copy_from_slice(&self.rows.to_be_bytes());
        for (set, bit) in [
            (self.end_of_line, CCITT_END_OF_LINE),
            (self.encoded_byte_align, CCITT_ENCODED_BYTE_ALIGN),
            (self.end_of_block, CCITT_END_OF_BLOCK),
            (self.black_is_1, CCITT_BLACK_IS_1),
        ] {
            if set {
                flags[0] |= bit;
            }
        }
        out
    }

    /// Reads the encoding above, or `None` if it is not one.
    ///
    /// Length-checked like every other field the worker receives: the pipe carries no
    /// invariants, and a short parameter block must be a refusal rather than a default.
    fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != CCITT_PARAMETERS_LEN {
            return None;
        }
        let k = i32::from_be_bytes(bytes.get(0..4)?.try_into().ok()?);
        let columns = u32::from_be_bytes(bytes.get(4..8)?.try_into().ok()?);
        let rows = u32::from_be_bytes(bytes.get(8..12)?.try_into().ok()?);
        let flags = *bytes.get(12)?;
        Some(Self {
            k,
            columns,
            rows,
            end_of_line: flags & CCITT_END_OF_LINE != 0,
            encoded_byte_align: flags & CCITT_ENCODED_BYTE_ALIGN != 0,
            end_of_block: flags & CCITT_END_OF_BLOCK != 0,
            black_is_1: flags & CCITT_BLACK_IS_1 != 0,
        })
    }
}

/// A decoded bilevel image.
///
/// `rows` is packed one bit per pixel, most significant bit first, each row starting on a
/// byte boundary — the sample format ISO 32000-2 Table 87 says the `JBIG2Decode` filter
/// delivers, so it feeds the ordinary image path with no special case.
///
/// **The sense is PDF's, not JBIG2's.** ISO/IEC 14492 gives a set pixel the value 1 and
/// calls it black; `DeviceGray` gives 0 to black. The bits are therefore complemented while
/// they are packed, in the one place that knows it is implementing a PDF filter rather than
/// a JBIG2 decoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bilevel {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Packed rows, `ceil(width / 8)` bytes each.
    pub rows: Vec<u8>,
}

/// What a JPEG 2000 codestream says its samples mean.
///
/// This is the codestream's own answer, carried out of the sandbox unevaluated. Choosing
/// between it and an overriding `/ColorSpace` entry is §7.4.9's rule and belongs to the
/// crate that can read the image dictionary, not to the one that decoded the samples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Colour {
    /// One component.
    Gray,
    /// Three components, sRGB.
    Rgb,
    /// Four components, subtractive.
    Cmyk,
    /// An embedded ICC profile.
    Icc(Vec<u8>),
    /// The codestream named a space this decoder does not recognise.
    ///
    /// §7.4.9 says to fall back on the channel count, which the caller has.
    Unknown,
}

/// A decoded continuous-tone image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raster {
    /// Width in pixels of the raster [`Self::data`] holds.
    pub width: u32,
    /// Height in pixels of the raster [`Self::data`] holds.
    pub height: u32,
    /// Width the codestream itself states, which a reduced decode does not change.
    ///
    /// A JPEG 2000 codestream over the decoder's budget is decoded at a reduced
    /// resolution level (§7.4.9 NOTE 3), so [`Self::width`] is then the level's grid
    /// rather than the image's. §7.4.9's conformance rule — "Width and Height shall match
    /// the corresponding width and height values in the JPEG 2000 data" — is about what
    /// the *data* says, so that statement travels beside the raster instead of being
    /// erased by the reduction. Equal to [`Self::width`] wherever nothing was reduced.
    pub stated_width: u32,
    /// Height the codestream itself states; see [`Self::stated_width`].
    pub stated_height: u32,
    /// Colour components per pixel, excluding opacity.
    pub components: u8,
    /// Whether an opacity channel follows the colour components in each pixel.
    pub has_opacity: bool,
    /// What the codestream says the colour components mean.
    pub colour: Colour,
    /// Interleaved samples, eight bits each, opacity last where present.
    ///
    /// Always eight bits, whatever the codestream's precision was: §7.4.9 leaves the depth
    /// to the decoder, and the image pipeline this feeds is eight-bit throughout.
    pub data: Vec<u8>,
}

impl Raster {
    /// Channels per pixel, opacity included.
    #[must_use]
    pub fn channels(&self) -> usize {
        usize::from(self.components).saturating_add(usize::from(self.has_opacity))
    }
}

/// Encodes the worker's greeting.
pub(crate) fn encode_handshake(confinement: Confinement) -> [u8; HANDSHAKE_LEN] {
    let mut greeting = [0u8; HANDSHAKE_LEN];
    let (magic, rest) = greeting.split_at_mut(8);
    magic.copy_from_slice(MAGIC);
    let (level, rest) = rest.split_at_mut(1);
    level[0] = match confinement.landlock {
        LandlockLevel::Enforced => 2,
        LandlockLevel::Partial => 1,
        LandlockLevel::Unavailable => 0,
    };
    let (limit, filtered) = rest.split_at_mut(8);
    limit.copy_from_slice(&confinement.address_space_limit.to_be_bytes());
    filtered[0] = u8::from(confinement.system_calls == SystemCalls::Filtered);
    greeting
}

/// Reads the worker's greeting, or `None` if it is not one.
pub(crate) fn parse_handshake(greeting: &[u8; HANDSHAKE_LEN]) -> Option<Confinement> {
    let (magic, rest) = greeting.split_at(8);
    if magic != MAGIC {
        return None;
    }
    let (level, rest) = rest.split_at(1);
    let landlock = match level.first()? {
        2 => LandlockLevel::Enforced,
        1 => LandlockLevel::Partial,
        0 => LandlockLevel::Unavailable,
        _ => return None,
    };
    let (limit, filtered) = rest.split_at(8);
    let bytes: [u8; 8] = limit.try_into().ok()?;
    let system_calls = match filtered.first()? {
        1 => SystemCalls::Filtered,
        0 => SystemCalls::Unfiltered,
        _ => return None,
    };
    Some(Confinement {
        landlock,
        address_space_limit: u64::from_be_bytes(bytes),
        system_calls,
    })
}

/// Encodes a request.
pub(crate) fn encode_request(request: &Request<'_>) -> Vec<u8> {
    // Outlives the borrow the match arm takes of it, which is why it is declared here.
    let encoded_parameters;
    let (kind, flags, primary, auxiliary) = match request {
        Request::Jbig2 { data, globals } => (KIND_JBIG2, 0, *data, *globals),
        Request::Jpx { data, indices } => (
            KIND_JPX,
            if *indices { FLAG_INDICES } else { 0 },
            *data,
            [].as_slice(),
        ),
        Request::Ccitt { data, parameters } => {
            encoded_parameters = parameters.encode();
            (KIND_CCITT, 0, *data, encoded_parameters.as_slice())
        }
    };

    let mut out = Vec::with_capacity(
        REQUEST_HEADER_LEN
            .saturating_add(primary.len())
            .saturating_add(auxiliary.len()),
    );
    out.push(kind);
    out.push(flags);
    out.extend_from_slice(&length(primary));
    out.extend_from_slice(&length(auxiliary));
    out.extend_from_slice(primary);
    out.extend_from_slice(auxiliary);
    out
}

/// A length as four big-endian bytes, saturating rather than wrapping.
fn length(bytes: &[u8]) -> [u8; 4] {
    u32::try_from(bytes.len()).unwrap_or(u32::MAX).to_be_bytes()
}

/// Reads a request, on the worker's side.
///
/// Returns `None` at end of input, which is how the worker learns the parent is gone.
///
/// # Errors
///
/// Returns the read error, or a malformed-input error if the framing is wrong.
pub(crate) fn read_request(input: &mut impl std::io::Read) -> std::io::Result<Option<Wire>> {
    let mut header = [0u8; REQUEST_HEADER_LEN];
    match input.read_exact(&mut header) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }

    let kind = *header.first().unwrap_or(&0);
    let flags = header.get(1).copied().unwrap_or(0);
    let primary_len = read_u32(&header, 2)?;
    let auxiliary_len = read_u32(&header, 6)?;

    let mut primary = vec![0u8; primary_len];
    input.read_exact(&mut primary)?;
    let mut auxiliary = vec![0u8; auxiliary_len];
    input.read_exact(&mut auxiliary)?;
    Ok(Some(Wire {
        kind,
        flags,
        primary,
        auxiliary,
    }))
}

/// A request as it arrived, before it is known to name a filter this worker implements.
#[derive(Debug)]
pub(crate) struct Wire {
    /// Which filter, as [`KIND_JBIG2`], [`KIND_JPX`] or [`KIND_CCITT`].
    pub(crate) kind: u8,
    /// Per-filter flags, currently only [`FLAG_INDICES`].
    pub(crate) flags: u8,
    /// The image's own stream.
    pub(crate) primary: Vec<u8>,
    /// Whatever the filter needs beside its data: JBIG2's globals stream, or
    /// `CCITTFaxDecode`'s parameter block.
    pub(crate) auxiliary: Vec<u8>,
}

/// Reads a big-endian length from `at`.
fn read_u32(bytes: &[u8], at: usize) -> std::io::Result<usize> {
    let field = bytes
        .get(at..at.saturating_add(4))
        .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
        .ok_or_else(|| std::io::Error::other("truncated length"))?;
    usize::try_from(u32::from_be_bytes(field)).map_err(std::io::Error::other)
}

/// Turns a wire request into a typed one, on the worker's side.
pub(crate) fn typed_request(wire: &Wire) -> Option<Request<'_>> {
    match wire.kind {
        KIND_JBIG2 => Some(Request::Jbig2 {
            data: &wire.primary,
            globals: &wire.auxiliary,
        }),
        KIND_JPX => Some(Request::Jpx {
            data: &wire.primary,
            indices: wire.flags & FLAG_INDICES != 0,
        }),
        KIND_CCITT => Some(Request::Ccitt {
            data: &wire.primary,
            parameters: CcittParameters::decode(&wire.auxiliary)?,
        }),
        _ => None,
    }
}

/// Encodes a successful decode.
pub(crate) fn encode_response(decoded: &Decoded) -> Vec<u8> {
    let (status, payload) = match decoded {
        Decoded::Bilevel(bilevel) => {
            let mut payload = Vec::with_capacity(bilevel.rows.len().saturating_add(8));
            payload.extend_from_slice(&bilevel.width.to_be_bytes());
            payload.extend_from_slice(&bilevel.height.to_be_bytes());
            payload.extend_from_slice(&bilevel.rows);
            (STATUS_BILEVEL, payload)
        }
        Decoded::Raster(raster) => {
            let (tag, profile) = match &raster.colour {
                Colour::Gray => (0u8, [].as_slice()),
                Colour::Rgb => (1, [].as_slice()),
                Colour::Cmyk => (2, [].as_slice()),
                Colour::Icc(profile) => (3, profile.as_slice()),
                Colour::Unknown => (4, [].as_slice()),
            };
            let mut payload = Vec::with_capacity(
                raster
                    .data
                    .len()
                    .saturating_add(profile.len())
                    .saturating_add(23),
            );
            payload.extend_from_slice(&raster.width.to_be_bytes());
            payload.extend_from_slice(&raster.height.to_be_bytes());
            payload.extend_from_slice(&raster.stated_width.to_be_bytes());
            payload.extend_from_slice(&raster.stated_height.to_be_bytes());
            payload.push(raster.components);
            payload.push(u8::from(raster.has_opacity));
            payload.push(tag);
            payload.extend_from_slice(&length(profile));
            payload.extend_from_slice(profile);
            payload.extend_from_slice(&raster.data);
            (STATUS_RASTER, payload)
        }
    };
    frame(status, &payload)
}

/// Encodes a refusal.
pub(crate) fn encode_error(detail: &str) -> Vec<u8> {
    frame(STATUS_ERROR, detail.as_bytes())
}

/// Prefixes a payload with its status and length.
fn frame(status: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len().saturating_add(RESPONSE_HEADER_LEN));
    out.push(status);
    out.extend_from_slice(&length(payload));
    out.extend_from_slice(payload);
    out
}

/// What a response header announced.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ResponseShape {
    status: u8,
    pub(crate) payload_len: usize,
}

/// Reads a response header, on the parent's side.
pub(crate) fn parse_response_header(header: [u8; RESPONSE_HEADER_LEN]) -> Option<ResponseShape> {
    let status = *header.first()?;
    if !matches!(status, STATUS_ERROR | STATUS_BILEVEL | STATUS_RASTER) {
        return None;
    }
    let bytes: [u8; 4] = header.get(1..5)?.try_into().ok()?;
    Some(ResponseShape {
        status,
        payload_len: usize::try_from(u32::from_be_bytes(bytes)).ok()?,
    })
}

/// Reads a response payload, on the parent's side.
///
/// Every derived length is checked against the payload it describes. A worker claiming a
/// 4000×4000 image in eight bytes of samples gets a malformed-response error rather than a
/// buffer this process then reads past the end of — which is why the sizes are recomputed
/// here instead of being trusted.
pub(crate) fn parse_response(
    shape: ResponseShape,
    payload: &[u8],
) -> Result<Decoded, SandboxError> {
    let malformed = |detail: String| SandboxError::Malformed { detail };

    match shape.status {
        STATUS_ERROR => Err(SandboxError::Undecodable {
            detail: String::from_utf8_lossy(payload).into_owned(),
        }),
        STATUS_BILEVEL => {
            let (width, height, rest) = dimensions(payload)?;
            let row_bytes = usize::try_from(width)
                .ok()
                .and_then(|width| width.checked_add(7))
                .map(|bits| bits / 8)
                .ok_or_else(|| malformed("implausible width".to_owned()))?;
            let expected = row_bytes
                .checked_mul(usize::try_from(height).unwrap_or(usize::MAX))
                .ok_or_else(|| malformed("implausible size".to_owned()))?;
            if rest.len() != expected {
                return Err(malformed(format!(
                    "{width}x{height} needs {expected} packed bytes, got {}",
                    rest.len()
                )));
            }
            Ok(Decoded::Bilevel(Bilevel {
                width,
                height,
                rows: rest.to_vec(),
            }))
        }
        STATUS_RASTER => {
            let (width, height, rest) = dimensions(payload)?;
            let (stated_width, stated_height, rest) = dimensions(rest)?;
            // The decode can only ever *reduce* a grid, so a statement smaller than the
            // raster is not one the worker's own code can produce — refused for the same
            // reason every other derived field is recomputed here.
            if stated_width < width || stated_height < height {
                return Err(malformed(format!(
                    "a {width}x{height} raster claims the codestream states \
                     {stated_width}x{stated_height}"
                )));
            }
            let components = *rest
                .first()
                .ok_or_else(|| malformed("no component count".to_owned()))?;
            let has_opacity = rest.get(1).is_some_and(|flag| *flag != 0);
            let tag = *rest
                .get(2)
                .ok_or_else(|| malformed("no colour".to_owned()))?;
            let profile_len = rest
                .get(3..7)
                .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
                .map(u32::from_be_bytes)
                .and_then(|len| usize::try_from(len).ok())
                .ok_or_else(|| malformed("no profile length".to_owned()))?;
            let after_profile = 7usize
                .checked_add(profile_len)
                .ok_or_else(|| malformed("implausible profile".to_owned()))?;
            let profile = rest
                .get(7..after_profile)
                .ok_or_else(|| malformed("truncated profile".to_owned()))?;
            let colour = match tag {
                0 => Colour::Gray,
                1 => Colour::Rgb,
                2 => Colour::Cmyk,
                3 => Colour::Icc(profile.to_vec()),
                _ => Colour::Unknown,
            };
            let samples = rest
                .get(after_profile..)
                .ok_or_else(|| malformed("truncated samples".to_owned()))?;

            let channels = usize::from(components).saturating_add(usize::from(has_opacity));
            let expected = usize::try_from(width)
                .ok()
                .zip(usize::try_from(height).ok())
                .and_then(|(width, height)| width.checked_mul(height))
                .and_then(|pixels| pixels.checked_mul(channels))
                .ok_or_else(|| malformed("implausible size".to_owned()))?;
            if samples.len() != expected {
                return Err(malformed(format!(
                    "{width}x{height} in {channels} channels needs {expected} samples, got {}",
                    samples.len()
                )));
            }
            if components == 0 {
                return Err(malformed("no colour components".to_owned()));
            }
            Ok(Decoded::Raster(Raster {
                width,
                height,
                stated_width,
                stated_height,
                components,
                has_opacity,
                colour,
                data: samples.to_vec(),
            }))
        }
        _ => Err(malformed("unrecognised status".to_owned())),
    }
}

/// Splits the leading width and height off a payload.
fn dimensions(payload: &[u8]) -> Result<(u32, u32, &[u8]), SandboxError> {
    let read = |at: usize| {
        payload
            .get(at..at.saturating_add(4))
            .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
            .map(u32::from_be_bytes)
    };
    let (Some(width), Some(height), Some(rest)) = (read(0), read(4), payload.get(8..)) else {
        return Err(SandboxError::Malformed {
            detail: "truncated dimensions".to_owned(),
        });
    };
    if width == 0 || height == 0 {
        return Err(SandboxError::Malformed {
            detail: format!("empty {width}x{height} image"),
        });
    }
    Ok((width, height, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_greeting_round_trips() {
        for system_calls in [SystemCalls::Filtered, SystemCalls::Unfiltered] {
            let confinement = Confinement {
                landlock: LandlockLevel::Partial,
                address_space_limit: 1 << 30,
                system_calls,
            };
            let encoded = encode_handshake(confinement);
            assert_eq!(parse_handshake(&encoded), Some(confinement));
        }
    }

    /// The byte a platform without seccomp sets, read back as the thing it means.
    ///
    /// Not decoration: a parent that could not tell an unconfined worker from a confined one
    /// would be exactly the failure the `compile_error!` this replaced was written against.
    #[test]
    fn an_unconfined_worker_is_legible_as_one() {
        let encoded = encode_handshake(Confinement::NONE);
        let parsed = parse_handshake(&encoded).expect("this protocol's own greeting");
        assert!(!parsed.is_enforced());
        assert!(parsed.shortfall().is_some());
    }

    #[test]
    fn a_greeting_from_another_program_is_rejected() {
        // What a program that ignored its arguments and printed something would send.
        let mut noise = [0u8; HANDSHAKE_LEN];
        noise[..8].copy_from_slice(b"usage: p");
        assert_eq!(parse_handshake(&noise), None);
    }

    #[test]
    fn a_request_round_trips() {
        let request = Request::Jbig2 {
            data: b"segments",
            globals: b"globals",
        };
        let encoded = encode_request(&request);
        let mut cursor = std::io::Cursor::new(encoded);
        let wire = read_request(&mut cursor).unwrap().unwrap();
        assert_eq!(
            typed_request(&wire),
            Some(Request::Jbig2 {
                data: b"segments",
                globals: b"globals"
            })
        );
    }

    #[test]
    fn end_of_input_is_not_an_error() {
        let mut empty = std::io::Cursor::new(Vec::new());
        assert!(read_request(&mut empty).unwrap().is_none());
    }

    #[test]
    fn a_bilevel_response_round_trips() {
        // Nine pixels wide, so a row is two bytes and the second holds one used bit.
        let decoded = Decoded::Bilevel(Bilevel {
            width: 9,
            height: 2,
            rows: vec![0b1010_1010, 0b1000_0000, 0x00, 0x00],
        });
        let encoded = encode_response(&decoded);
        let (header, payload) = encoded.split_at(RESPONSE_HEADER_LEN);
        let shape = parse_response_header(header.try_into().unwrap()).unwrap();
        assert_eq!(parse_response(shape, payload).unwrap(), decoded);
    }

    #[test]
    fn a_raster_response_round_trips() {
        let decoded = Decoded::Raster(Raster {
            width: 2,
            height: 1,
            // Larger than the raster, as a reduced JPEG 2000 decode leaves it.
            stated_width: 8,
            stated_height: 4,
            components: 3,
            has_opacity: true,
            colour: Colour::Icc(vec![1, 2, 3]),
            data: vec![10, 20, 30, 40, 50, 60, 70, 80],
        });
        let encoded = encode_response(&decoded);
        let (header, payload) = encoded.split_at(RESPONSE_HEADER_LEN);
        let shape = parse_response_header(header.try_into().unwrap()).unwrap();
        assert_eq!(parse_response(shape, payload).unwrap(), decoded);
    }

    #[test]
    fn a_worker_cannot_claim_more_samples_than_it_sent() {
        // The check that matters: this is the shape of a response from a worker that has
        // been subverted, and the parent must refuse it rather than size a buffer from it.
        let mut payload = Vec::new();
        payload.extend_from_slice(&4000u32.to_be_bytes());
        payload.extend_from_slice(&4000u32.to_be_bytes());
        payload.extend_from_slice(&4000u32.to_be_bytes());
        payload.extend_from_slice(&4000u32.to_be_bytes());
        payload.extend_from_slice(&[3, 0, 1]);
        payload.extend_from_slice(&0u32.to_be_bytes());
        payload.extend_from_slice(&[0; 8]);
        let shape = ResponseShape {
            status: STATUS_RASTER,
            payload_len: payload.len(),
        };
        assert!(matches!(
            parse_response(shape, &payload),
            Err(SandboxError::Malformed { .. })
        ));
    }

    /// A decode only ever reduces a grid, so a statement smaller than the raster is a lie.
    #[test]
    fn a_worker_cannot_state_a_grid_smaller_than_its_raster() {
        let decoded = Decoded::Raster(Raster {
            width: 2,
            height: 1,
            stated_width: 1,
            stated_height: 1,
            components: 3,
            has_opacity: true,
            colour: Colour::Rgb,
            data: vec![10, 20, 30, 40, 50, 60, 70, 80],
        });
        let encoded = encode_response(&decoded);
        let (header, payload) = encoded.split_at(RESPONSE_HEADER_LEN);
        let shape = parse_response_header(header.try_into().unwrap()).unwrap();
        assert!(matches!(
            parse_response(shape, payload),
            Err(SandboxError::Malformed { .. })
        ));
    }

    #[test]
    fn a_refusal_arrives_as_its_own_error() {
        let encoded = encode_error("JBIG2: unknown segment type 42");
        let (header, payload) = encoded.split_at(RESPONSE_HEADER_LEN);
        let shape = parse_response_header(header.try_into().unwrap()).unwrap();
        let Err(SandboxError::Undecodable { detail }) = parse_response(shape, payload) else {
            panic!("a refusal must not look like a decode");
        };
        assert_eq!(detail, "JBIG2: unknown segment type 42");
    }
}
