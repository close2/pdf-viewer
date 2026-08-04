//! Stream filters.
//!
//! # Only what can be done safely
//!
//! `FlateDecode` covers the overwhelming majority of real streams and is implemented here
//! via `flate2` with the pure-Rust `zlib-rs` backend. `ASCIIHexDecode`, `ASCII85Decode`,
//! `RunLengthDecode` and `LZWDecode` are simple enough to implement directly — the last of
//! them is §7.4.4.2, which states the whole algorithm and even supplies a worked example
//! this module's tests decode.
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
    let decoded = decode(filter, data, parms, limits)?;

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
    // **Two row buffers, swapped rather than allocated per row.** A PNG image has a few
    // thousand wide rows and would not care; a cross-reference stream has one *six-byte* row
    // per object, and ISO 32000-2 has 101 318 of them — so a `vec![0u8; row_len]` inside this
    // loop was one allocation per object, on the launch path. Measured with callgrind over ten
    // opens of that file (`cargo run -p pdf-syntax --example callgrind_open`): **138.3 M
    // instructions per `Document::open` before, 131.3 M after — 5.1%** — and every millisecond
    // of it in `calloc` and `free` rather than in this loop, which is why the loop is unchanged.
    // ADR 0180.
    let mut previous = vec![0u8; row_len];
    let mut current = vec![0u8; row_len];

    for chunk in data.chunks(stride) {
        let (&tag, row) = chunk.split_first()?;
        let copy = row.len().min(row_len);
        current.get_mut(..copy)?.copy_from_slice(row.get(..copy)?);
        // Only a final short row can leave anything behind — `chunks` yields a short chunk
        // last or never — but the buffer is reused, so what the old code got from allocating
        // a zeroed row is written rather than assumed.
        current.get_mut(copy..)?.fill(0);

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
        std::mem::swap(&mut previous, &mut current);
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

/// Whether a filter produces image samples rather than bytes.
///
/// These are the filters [`decode`] returns `None` for, and the distinction is not that
/// they are unimplemented — `DCTDecode` has worked since the first images drew. It is that
/// their output is a raster with a width, a component count and a depth, which a function
/// returning a byte slice cannot describe. They belong to the image pipeline, and this is
/// how it recognises the stage it must run itself.
///
/// `DCT` and `CCF` are the inline-image abbreviations of Table 92. `JBIG2Decode` and
/// `JPXDecode` have none, because ISO 32000-2 §8.9.7 forbids both in an inline image.
#[must_use]
pub fn is_image_codec(filter: &[u8]) -> bool {
    matches!(
        filter,
        b"DCTDecode" | b"DCT" | b"JPXDecode" | b"JBIG2Decode" | b"CCITTFaxDecode" | b"CCF"
    )
}

/// Decodes one filter stage.
///
/// `parms` is the stage's own `/DecodeParms`, needed by exactly one filter: `LZWDecode`'s
/// `/EarlyChange` changes where the code width grows and therefore what every byte after
/// that point decodes to.
///
/// Returns `None` for an unsupported filter or corrupt data.
#[must_use]
pub fn decode(
    filter: &[u8],
    data: &[u8],
    parms: Option<&Dictionary>,
    limits: Limits,
) -> Option<Arc<[u8]>> {
    match filter {
        b"FlateDecode" | b"Fl" => flate(data, limits),
        b"LZWDecode" | b"LZW" => {
            // Table 8: "If the value of this entry is 0, code length increases shall be
            // postponed as long as possible. If the value is 1, code length increases shall
            // occur one code early." Default 1, which is the *incorrect* behaviour of a
            // widely-copied encoder and is therefore what almost every file needs.
            let early = parms
                .and_then(|parms| parms.get("EarlyChange"))
                .and_then(Object::as_integer)
                .unwrap_or(1);
            lzw(data, early != 0, limits)
        }
        b"ASCIIHexDecode" | b"AHx" => Some(ascii_hex(data)),
        b"ASCII85Decode" | b"A85" => ascii85(data, limits),
        b"RunLengthDecode" | b"RL" => run_length(data, limits),
        // Not a compression filter: it declares that the stream is encrypted, which is
        // handled elsewhere. Passing the data through unchanged is correct.
        b"Crypt" => Some(Arc::from(data)),
        _ => None,
    }
}

/// Decodes `LZWDecode`: ISO 32000-2 §7.4.4.2, whose four paragraphs are the whole algorithm.
///
/// > Data encoded using the LZW compression method shall consist of a sequence of codes that
/// > are 9 to 12 bits long. Each code shall represent a single character of input data
/// > (0 -255), a clear-table marker (256), an EOD marker (257), or a table entry representing
/// > a multiple-character sequence that has been encountered previously in the input (258 or
/// > greater).
///
/// The table is held as a prefix code and one byte per entry rather than as a growing
/// sequence per entry, which is the standard construction and the reason the clause can say
/// "the encoder and the decoder shall maintain identical copies of this table" without
/// either of them copying anything: entry *n* is entry `prefix[n]` followed by `suffix[n]`,
/// so appending one is two stores.
///
/// Three details decide whether a decoder is right, and each is a sentence of the clause:
///
/// - **The code width grows before the entry that needs it.** "The first output code that is
///   10 bits long shall be the one following the creation of table entry 511", and Table 8's
///   `/EarlyChange` moves that one code earlier, which is the default because a widely-copied
///   encoder did it. Getting this wrong desynchronises the bit stream from that point on and
///   produces plausible bytes for ever after.
/// - **A code may name the entry about to be created.** The encoder emits the code for a
///   sequence it has just added, so a decoder that has not added it yet must reconstruct it:
///   it is the previous sequence followed by that sequence's own first byte. This is the case
///   an input of one repeated character reaches immediately.
/// - **Bits are packed high-order first**, across byte boundaries, "thus, codes may straddle
///   byte boundaries arbitrarily".
///
/// A truncated or corrupt stream keeps what it decoded, for [`flate`]'s reason: a partial
/// content stream still renders most of a page.
fn lzw(data: &[u8], early_change: bool, limits: Limits) -> Option<Arc<[u8]>> {
    /// Restart with the initial table and a nine-bit code.
    const CLEAR: u16 = 256;
    /// End of data.
    const EOD: u16 = 257;
    /// The first code the table assigns; 0 to 255 are themselves and 256, 257 are markers.
    const FIRST: u16 = 258;
    /// "Codes shall never be longer than 12 bits; therefore, entry 4095 is the last entry."
    const ENTRIES: usize = 4096;

    let mut prefix = [0u16; ENTRIES];
    let mut suffix = [0u8; ENTRIES];

    let mut next = FIRST;
    let mut width = 9u32;
    let mut previous: Option<u16> = None;
    let mut out: Vec<u8> = Vec::new();
    // One entry's sequence, built backwards by walking the prefix chain. Bounded by the
    // table's size, since a chain longer than that would be a cycle.
    let mut scratch: Vec<u8> = Vec::with_capacity(ENTRIES);

    let mut held: u32 = 0;
    let mut bits: u32 = 0;
    for &byte in data {
        held = (held << 8) | u32::from(byte);
        bits += 8;
        while bits >= width {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "masked to `width` bits, and the clause caps a code at twelve"
            )]
            let code = ((held >> (bits - width)) & ((1u32 << width) - 1)) as u16;
            bits -= width;

            if code == EOD {
                return Some(Arc::from(out.as_slice()));
            }
            if code == CLEAR {
                next = FIRST;
                width = 9;
                previous = None;
                continue;
            }

            // The sequence this code names, or — where it names the entry about to be
            // created — the one the encoder just added.
            scratch.clear();
            let mut walk = if code < next {
                code
            } else if code == next && previous.is_some() {
                // Reconstructed below from `previous`; start the walk there and append its
                // own first byte afterwards.
                previous.unwrap_or(0)
            } else {
                // A code past the end of the table is corrupt data, not a sequence.
                return (!out.is_empty()).then(|| Arc::from(out.as_slice()));
            };
            let extends = code == next;
            for _ in 0..ENTRIES {
                let index = usize::from(walk);
                if walk < 256 {
                    scratch.push(u8::try_from(walk).unwrap_or(0));
                    break;
                }
                scratch.push(suffix.get(index).copied().unwrap_or(0));
                walk = prefix.get(index).copied().unwrap_or(0);
            }
            scratch.reverse();
            if extends {
                let first = scratch.first().copied().unwrap_or(0);
                scratch.push(first);
            }

            if out.len().saturating_add(scratch.len()) > limits.max_stream_len {
                // A bomb rather than a stream: LZW reaches about 1365:1 on long runs of one
                // byte, so a small file can name a very large output.
                return (!out.is_empty()).then(|| Arc::from(out.as_slice()));
            }
            out.extend_from_slice(&scratch);

            // Step (d): "create a new table entry for the first unused code. Its value is
            // the sequence found in step (a) followed by the next input character" — which
            // the decoder sees as the previous code followed by this sequence's first byte.
            if let Some(previous) = previous
                && usize::from(next) < ENTRIES
            {
                let index = usize::from(next);
                if let Some(slot) = prefix.get_mut(index) {
                    *slot = previous;
                }
                if let Some(slot) = suffix.get_mut(index) {
                    *slot = scratch.first().copied().unwrap_or(0);
                }
                next += 1;
                let grown = u32::from(next) + u32::from(early_change);
                if width < 12 && grown >= (1u32 << width) {
                    width += 1;
                }
            }
            previous = Some(code);
        }
    }

    // No EOD marker: the encoder is required to emit one, and a file that does not is
    // truncated rather than empty.
    (!out.is_empty()).then(|| Arc::from(out.as_slice()))
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
    use super::{Dictionary, Limits, Object, decode};
    use crate::object::Name;

    #[test]
    fn ascii_hex_decodes_and_stops_at_the_terminator() {
        let out = decode(
            b"ASCIIHexDecode",
            b"48656C6C6F>ignored",
            None,
            Limits::DEFAULT,
        )
        .expect("valid");
        assert_eq!(&*out, b"Hello");
    }

    #[test]
    fn ascii_hex_pads_an_odd_final_digit() {
        let out = decode(b"ASCIIHexDecode", b"4A5>", None, Limits::DEFAULT).expect("valid");
        assert_eq!(&*out, &[0x4a, 0x50]);
    }

    /// §7.4.4.2's own worked example, decoded.
    ///
    /// The clause gives an input, the codes it encodes to, and the bytes those codes pack
    /// into — which makes this the rarest kind of test in the tree: an expected value the
    /// standard states outright rather than one derived from a rule. EXAMPLE 1's input is
    ///
    /// > 45 45 45 45 45 65 45 45 45 66
    ///
    /// and EXAMPLE 2 is what it packs to. Note what the sequence exercises: the third code
    /// the encoder emits is 258, whose table entry the *decoder* has not created yet at the
    /// point it reads it, so a decoder without the "code names the entry about to be
    /// created" case fails on the standard's own example.
    #[test]
    fn lzw_decodes_the_clauses_own_example() {
        let out = decode(
            b"LZWDecode",
            &[0x80, 0x0B, 0x60, 0x50, 0x22, 0x0C, 0x0C, 0x85, 0x01],
            None,
            Limits::DEFAULT,
        )
        .expect("the standard's example decodes");
        assert_eq!(&*out, &[45, 45, 45, 45, 45, 65, 45, 45, 45, 66]);
    }

    /// The code width grows where the clause says, and `/EarlyChange` moves it.
    ///
    /// §7.4.4.2: "the first output code that is 10 bits long shall be the one following the
    /// creation of table entry 511". Table 8's `/EarlyChange` makes that one code earlier and
    /// **defaults to doing so**, so the two settings disagree about one code's width and
    /// therefore about every bit after it.
    ///
    /// The fixture is built rather than quoted, and it is a stream of literal codes packed
    /// at nine bits throughout. The 254th of them creates table entry 510 and leaves the next
    /// free code at 511, which is where the two settings part: the default reads the 255th
    /// code as ten bits and `/EarlyChange 0` reads it as nine. From there the two readings of
    /// the *same bytes* diverge completely, which is the point — this parameter is not a
    /// rounding difference, it is a different bit stream.
    #[test]
    fn early_change_moves_the_width_increase_by_one_code() {
        let literals: Vec<u16> = (0..300u16).map(|index| index % 256).collect();
        let packed = pack_codes(&literals, 9);

        let early = decode(b"LZWDecode", &packed, None, Limits::DEFAULT).expect("decodes");
        let mut parms = Dictionary::new();
        parms.insert(Name::new(b"EarlyChange".as_slice()), Object::Integer(0));
        let late = decode(b"LZWDecode", &packed, Some(&parms), Limits::DEFAULT).expect("decodes");

        assert_eq!(
            &early[..254],
            &late[..254],
            "the codes before the boundary are the same either way"
        );
        assert_ne!(
            &*early, &*late,
            "the two settings must disagree about where the tenth bit starts"
        );
    }

    /// Packs codes high-order bit first, which is how §7.4.4.2 says they are written.
    fn pack_codes(codes: &[u16], width: u32) -> Vec<u8> {
        let mut out = Vec::new();
        let mut held: u32 = 0;
        let mut bits: u32 = 0;
        for code in codes {
            held = (held << width) | (u32::from(*code) & ((1u32 << width) - 1));
            bits += width;
            while bits >= 8 {
                out.push(((held >> (bits - 8)) & 0xFF) as u8);
                bits -= 8;
            }
        }
        if bits > 0 {
            out.push(((held << (8 - bits)) & 0xFF) as u8);
        }
        out
    }

    /// A code past the end of the table keeps what was decoded rather than inventing bytes.
    #[test]
    fn a_code_the_table_does_not_have_stops_rather_than_guessing() {
        // Two literals, then code 400, which is inside nine bits and past the table's end
        // — after two literals the first unused code is 259.
        let packed = pack_codes(&[65, 66, 400], 9);
        let out = decode(b"LZWDecode", &packed, None, Limits::DEFAULT).expect("partial output");
        assert_eq!(&*out, b"AB");
    }

    #[test]
    fn run_length_expands_literal_and_repeated_runs() {
        // 2 -> copy 3 bytes; 254 -> repeat next byte 3 times; 128 -> end.
        let out = decode(
            b"RunLengthDecode",
            &[2, b'a', b'b', b'c', 254, b'x', 128],
            None,
            Limits::DEFAULT,
        )
        .expect("valid");
        assert_eq!(&*out, b"abcxxx");
    }

    #[test]
    fn ascii85_round_trips_a_known_value() {
        // "Man " encodes as "9jqo" in base 85 terms; use the canonical empty and 'z' cases.
        let out = decode(b"ASCII85Decode", b"z~>", None, Limits::DEFAULT).expect("valid");
        assert_eq!(&*out, &[0, 0, 0, 0], "'z' stands for four zero bytes");
    }

    #[test]
    fn flate_round_trips() {
        use std::io::Write as _;
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(b"round trip").expect("in-memory write");
        let compressed = encoder.finish().expect("finish");

        let out = decode(b"FlateDecode", &compressed, None, Limits::DEFAULT).expect("valid");
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

        let out = decode(b"FlateDecode", &compressed, None, Limits::DEFAULT).expect("valid");
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
                decode(filter, b"whatever", None, Limits::DEFAULT).is_none(),
                "{} must not be treated as decoded",
                String::from_utf8_lossy(filter)
            );
        }
    }
}
