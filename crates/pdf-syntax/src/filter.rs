//! Stream filters.
//!
//! # Only what can be done safely
//!
//! `FlateDecode` covers the overwhelming majority of real streams and is implemented here
//! via `flate2` with the pure-Rust `zlib-rs` backend. `ASCIIHexDecode`, `ASCII85Decode` and
//! `RunLengthDecode` are simple enough to implement directly.
//!
//! Everything else — `DCTDecode`, `JPXDecode`, `JBIG2Decode`, `CCITTFaxDecode` — returns
//! `None`. Those are image codecs, they belong to the image pipeline rather than to stream
//! decoding, and two of them (JBIG2, JPX) have no memory-safe implementation and are among
//! the worst attack surfaces in the format. Returning `None` means an unsupported stream is
//! visibly unsupported rather than silently garbage.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "offsets and lengths are saturating throughout and every slice access uses \
              `get`, so no operation can overflow or index out of bounds"
)]

use std::sync::Arc;

use crate::object::{Dictionary, Object};
use crate::parser::Limits;

/// Decodes one filter stage and applies any predictor from `parms`.
///
/// Predictors are part of decoding, not a separate step: `FlateDecode` with
/// `/Predictor 12` produces PNG-filtered rows, and treating those as the final output
/// yields plausible-looking numbers that are all wrong. Cross-reference streams use this
/// combination almost universally, so getting it wrong makes modern PDFs unreadable while
/// appearing to work.
#[must_use]
pub fn decode_with_parms(
    filter: &[u8],
    data: &[u8],
    parms: Option<&Dictionary>,
    limits: Limits,
) -> Option<Arc<[u8]>> {
    let decoded = decode(filter, data, limits)?;

    let Some(parms) = parms else {
        return Some(decoded);
    };
    let predictor = parms
        .get("Predictor")
        .and_then(Object::as_integer)
        .unwrap_or(1);
    if predictor <= 1 {
        return Some(decoded);
    }

    let colors = parms
        .get("Colors")
        .and_then(Object::as_integer)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(1);
    let bits = parms
        .get("BitsPerComponent")
        .and_then(Object::as_integer)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(8);
    let columns = parms
        .get("Columns")
        .and_then(Object::as_integer)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(1);

    apply_predictor(&decoded, predictor, colors, bits, columns)
}

/// Reverses a PNG or TIFF predictor.
///
/// Returns `None` for a predictor code that is not defined, rather than passing the data
/// through: unpredicted data read as predicted is silently wrong, and for a
/// cross-reference stream that means fabricated byte offsets.
#[must_use]
pub fn apply_predictor(
    data: &[u8],
    predictor: i64,
    colors: usize,
    bits: usize,
    columns: usize,
) -> Option<Arc<[u8]>> {
    // Bytes per pixel, at least one: a sub-byte pixel still shifts by one byte.
    let bpp = colors.saturating_mul(bits).saturating_add(7) / 8;
    let bpp = bpp.max(1);
    let row_len = colors
        .saturating_mul(bits)
        .saturating_mul(columns)
        .saturating_add(7)
        / 8;
    if row_len == 0 {
        return None;
    }

    // 2 is the TIFF predictor; 10..=15 are the PNG filters, which are selected per row by
    // a leading byte, so the declared value only says "PNG" and is otherwise unused.
    if predictor == 2 {
        return Some(tiff_predictor(data, colors, bits, row_len));
    }
    if !(10..=15).contains(&predictor) {
        return None;
    }

    let stride = row_len.saturating_add(1);
    let mut out: Vec<u8> = Vec::with_capacity(data.len());
    let mut previous = vec![0u8; row_len];

    for chunk in data.chunks(stride) {
        let (&tag, row) = chunk.split_first()?;
        let mut current = vec![0u8; row_len];
        let copy = row.len().min(row_len);
        current.get_mut(..copy)?.copy_from_slice(row.get(..copy)?);

        for index in 0..copy {
            let left = if index >= bpp {
                *current.get(index.saturating_sub(bpp))?
            } else {
                0
            };
            let up = *previous.get(index)?;
            let up_left = if index >= bpp {
                *previous.get(index.saturating_sub(bpp))?
            } else {
                0
            };
            let raw = *current.get(index)?;

            let value = match tag {
                0 => raw,
                1 => raw.wrapping_add(left),
                2 => raw.wrapping_add(up),
                // The PNG Average filter is the floor of the mean, which `midpoint`
                // computes without an intermediate that could overflow.
                3 => raw.wrapping_add(u8::midpoint(left, up)),
                4 => raw.wrapping_add(paeth(left, up, up_left)),
                // An undefined row filter cannot be reversed; guessing would corrupt every
                // subsequent row too, since rows depend on their predecessor.
                _ => return None,
            };
            *current.get_mut(index)? = value;
        }

        out.extend_from_slice(current.get(..copy)?);
        previous = current;
    }

    Some(Arc::from(out.as_slice()))
}

/// The PNG Paeth predictor.
fn paeth(left: u8, up: u8, up_left: u8) -> u8 {
    let estimate = i16::from(left) + i16::from(up) - i16::from(up_left);
    let distance_left = (estimate - i16::from(left)).abs();
    let distance_up = (estimate - i16::from(up)).abs();
    let distance_up_left = (estimate - i16::from(up_left)).abs();

    if distance_left <= distance_up && distance_left <= distance_up_left {
        left
    } else if distance_up <= distance_up_left {
        up
    } else {
        up_left
    }
}

/// Reverses the TIFF predictor, which adds the preceding pixel with no per-row tag.
fn tiff_predictor(data: &[u8], colors: usize, bits: usize, row_len: usize) -> Arc<[u8]> {
    // Only 8 bits per component is handled; other depths need bit-level work and do not
    // occur in the streams this reader meets. Returned unchanged rather than refused,
    // because for TIFF prediction the data is at least still image samples.
    if bits != 8 {
        return Arc::from(data);
    }

    let mut out = data.to_vec();
    for row in out.chunks_mut(row_len) {
        for index in colors..row.len() {
            let previous = row.get(index.saturating_sub(colors)).copied().unwrap_or(0);
            if let Some(slot) = row.get_mut(index) {
                *slot = slot.wrapping_add(previous);
            }
        }
    }
    Arc::from(out.as_slice())
}

/// Decodes one filter stage.
///
/// Returns `None` for an unsupported filter or corrupt data.
#[must_use]
pub fn decode(filter: &[u8], data: &[u8], limits: Limits) -> Option<Arc<[u8]>> {
    match filter {
        b"FlateDecode" | b"Fl" => flate(data, limits),
        b"ASCIIHexDecode" | b"AHx" => Some(ascii_hex(data)),
        b"ASCII85Decode" | b"A85" => ascii85(data, limits),
        b"RunLengthDecode" | b"RL" => run_length(data, limits),
        // Not a compression filter: it declares that the stream is encrypted, which is
        // handled elsewhere. Passing the data through unchanged is correct.
        b"Crypt" => Some(Arc::from(data)),
        _ => None,
    }
}

/// Inflates a zlib or raw deflate stream.
///
/// Tries zlib framing first and falls back to raw deflate, because streams missing their
/// two-byte zlib header are common in the wild. Truncated output is kept rather than
/// discarded: a partially-inflated content stream still renders most of a page, and
/// discarding it would lose everything over one corrupt byte at the end.
fn flate(data: &[u8], limits: Limits) -> Option<Arc<[u8]>> {
    use std::io::Read as _;

    // Leading whitespace before the compressed data occurs and confuses the header check.
    let start = data
        .iter()
        .position(|&byte| !crate::lexer::is_whitespace(byte))?;
    let data = data.get(start..)?;

    for raw in [false, true] {
        let mut out = Vec::new();
        let result = if raw {
            flate2::read::DeflateDecoder::new(data)
                .take(limits.max_stream_len as u64)
                .read_to_end(&mut out)
        } else {
            flate2::read::ZlibDecoder::new(data)
                .take(limits.max_stream_len as u64)
                .read_to_end(&mut out)
        };
        match result {
            Ok(_) => return Some(Arc::from(out.as_slice())),
            // Partial output from a truncated stream is still useful.
            Err(_) if !out.is_empty() => return Some(Arc::from(out.as_slice())),
            Err(_) => {}
        }
    }
    None
}

/// Decodes `ASCIIHexDecode`: hex digits terminated by `>`.
fn ascii_hex(data: &[u8]) -> Arc<[u8]> {
    let mut out = Vec::new();
    let mut pending: Option<u8> = None;

    for &byte in data {
        if byte == b'>' {
            break;
        }
        let value = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => continue,
        };
        match pending.take() {
            Some(high) => out.push(high.saturating_mul(16).saturating_add(value)),
            None => pending = Some(value),
        }
    }
    // An odd final digit is treated as if followed by zero, per the specification.
    if let Some(high) = pending {
        out.push(high.saturating_mul(16));
    }
    Arc::from(out.as_slice())
}

/// Decodes `ASCII85Decode`.
fn ascii85(data: &[u8], limits: Limits) -> Option<Arc<[u8]>> {
    let mut out = Vec::new();
    let mut group = [0u8; 5];
    let mut count = 0usize;

    // An optional `<~` introduces the data.
    let body = if data.starts_with(b"<~") {
        data.get(2..).unwrap_or_default()
    } else {
        data
    };

    for byte in body.iter().copied() {
        if crate::lexer::is_whitespace(byte) {
            continue;
        }
        if byte == b'~' {
            break;
        }
        // `z` stands for four zero bytes, and is only valid between groups.
        if byte == b'z' && count == 0 {
            out.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        if !(b'!'..=b'u').contains(&byte) {
            return None;
        }

        if let Some(slot) = group.get_mut(count) {
            *slot = byte - b'!';
        }
        count = count.saturating_add(1);

        if count == 5 {
            push_ascii85_group(&mut out, group, 5);
            count = 0;
        }
        if out.len() > limits.max_stream_len {
            return None;
        }
    }

    if count > 1 {
        // A partial final group is padded with the maximum digit.
        let mut padded = group;
        for slot in padded.iter_mut().skip(count) {
            *slot = 84;
        }
        push_ascii85_group(&mut out, padded, count);
    }

    Some(Arc::from(out.as_slice()))
}

/// Expands one base-85 group, keeping `count - 1` of the four decoded bytes.
fn push_ascii85_group(out: &mut Vec<u8>, group: [u8; 5], count: usize) {
    let mut value = 0u32;
    for digit in group {
        value = value.saturating_mul(85).saturating_add(u32::from(digit));
    }
    let bytes = value.to_be_bytes();
    let keep = count.saturating_sub(1).min(4);
    out.extend_from_slice(bytes.get(..keep).unwrap_or_default());
}

/// Decodes `RunLengthDecode`.
fn run_length(data: &[u8], limits: Limits) -> Option<Arc<[u8]>> {
    let mut out = Vec::new();
    let mut at = 0usize;

    while let Some(&length) = data.get(at) {
        at = at.saturating_add(1);
        match length {
            // 128 marks the end of the data.
            128 => break,
            // 0..=127: copy the next length + 1 bytes literally.
            0..=127 => {
                let run = usize::from(length).saturating_add(1);
                let slice = data.get(at..at.saturating_add(run))?;
                out.extend_from_slice(slice);
                at = at.saturating_add(run);
            }
            // 129..=255: repeat the next byte 257 - length times.
            _ => {
                let &byte = data.get(at)?;
                at = at.saturating_add(1);
                let run = 257usize.saturating_sub(usize::from(length));
                out.resize(out.len().saturating_add(run), byte);
            }
        }
        if out.len() > limits.max_stream_len {
            return None;
        }
    }

    Some(Arc::from(out.as_slice()))
}

#[cfg(test)]
mod tests {
    use super::{Limits, decode};

    #[test]
    fn ascii_hex_decodes_and_stops_at_the_terminator() {
        let out = decode(b"ASCIIHexDecode", b"48656C6C6F>ignored", Limits::DEFAULT).expect("valid");
        assert_eq!(&*out, b"Hello");
    }

    #[test]
    fn ascii_hex_pads_an_odd_final_digit() {
        let out = decode(b"ASCIIHexDecode", b"4A5>", Limits::DEFAULT).expect("valid");
        assert_eq!(&*out, &[0x4a, 0x50]);
    }

    #[test]
    fn run_length_expands_literal_and_repeated_runs() {
        // 2 -> copy 3 bytes; 254 -> repeat next byte 3 times; 128 -> end.
        let out = decode(
            b"RunLengthDecode",
            &[2, b'a', b'b', b'c', 254, b'x', 128],
            Limits::DEFAULT,
        )
        .expect("valid");
        assert_eq!(&*out, b"abcxxx");
    }

    #[test]
    fn ascii85_round_trips_a_known_value() {
        // "Man " encodes as "9jqo" in base 85 terms; use the canonical empty and 'z' cases.
        let out = decode(b"ASCII85Decode", b"z~>", Limits::DEFAULT).expect("valid");
        assert_eq!(&*out, &[0, 0, 0, 0], "'z' stands for four zero bytes");
    }

    #[test]
    fn flate_round_trips() {
        use std::io::Write as _;
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(b"round trip").expect("in-memory write");
        let compressed = encoder.finish().expect("finish");

        let out = decode(b"FlateDecode", &compressed, Limits::DEFAULT).expect("valid");
        assert_eq!(&*out, b"round trip");
    }

    /// Streams missing the two-byte zlib header are common; the raw fallback handles them.
    #[test]
    fn flate_falls_back_to_raw_deflate() {
        use std::io::Write as _;
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder
            .write_all(b"no zlib header")
            .expect("in-memory write");
        let compressed = encoder.finish().expect("finish");

        let out = decode(b"FlateDecode", &compressed, Limits::DEFAULT).expect("valid");
        assert_eq!(&*out, b"no zlib header");
    }

    /// An unsupported filter must be visibly unsupported, not silently passed through.
    #[test]
    fn image_codecs_are_reported_as_unsupported() {
        for filter in [
            &b"DCTDecode"[..],
            b"JPXDecode",
            b"JBIG2Decode",
            b"CCITTFaxDecode",
        ] {
            assert!(
                decode(filter, b"whatever", Limits::DEFAULT).is_none(),
                "{} must not be treated as decoded",
                String::from_utf8_lossy(filter)
            );
        }
    }
}
