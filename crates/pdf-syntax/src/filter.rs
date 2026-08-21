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

/// Why a filter stage produced no decoded bytes.
///
/// Three statements about one stream, and keeping them apart is what
/// [`decode_with_parms_reported`] exists for. A caller that only needs bytes takes
/// [`decode_with_parms`] and gets `None` for all three.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterRefusal {
    /// The filter is not one this module decodes: an image codec, or a name from no table.
    Unsupported,
    /// The data is not what the filter's grammar admits, and nothing survived.
    ///
    /// A *partly* decodable stream is not this: `FlateDecode`, `LZWDecode` and
    /// `RunLengthDecode` all keep what they decoded before the damage and say so in
    /// [`Decoded::damage`], because a partly-decoded content stream still renders most of a
    /// page. **Saying so is the half that was missing until ADR 0343**, which is the whole of
    /// trap 5: a prefix handed over as though it were the stream is a plausible-looking
    /// fallback, and only the report tells a short page from a sparse one.
    Corrupt,
    /// The decoded data passed [`Limits::max_stream_len`].
    ///
    /// **Kept apart from [`Self::Corrupt`] because the two are opposite statements about the
    /// same bytes**: a corrupt stream gave everything it had, and this one had more to give.
    /// Until the four-hundred-and-seventy-first session `flate` and `lzw` answered a bomb with
    /// the prefix they had inflated and no report at all — `io::Take` returns `Ok` at its
    /// limit, so a stream clamped at two gibibytes was indistinguishable from a complete
    /// decode. Trap 5: unsupported input stays loud.
    TooLarge {
        /// The bound, in bytes.
        limit: usize,
    },
}

/// Why a decode stopped before the filter's own end-of-data.
///
/// **Not an error**: the bytes that came out are what the encoder's own algorithm produced
/// from the bytes that were there, and ISO 32000-2 §7.4.1 asks a reader to "invoke the
/// corresponding decoding filter" — which is what was done. What the decode did *not* achieve
/// is the rest of that sentence, "to convert the information back to its original form", and
/// the difference between those two is exactly this value. ADR 0343.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Damage {
    /// The encoded data ran out before the filter's end-of-data marker.
    ///
    /// Every filter that has one states it: §7.4.4.2's EOD code 257 for `LZWDecode`,
    /// §7.4.5's "[a] length value of 128 shall denote EOD" for `RunLengthDecode`, and RFC
    /// 1951's final block for `FlateDecode`. Reaching the end of the input without seeing it
    /// means the bytes stop short of what the stream says it is — which §7.3.8.2 makes a
    /// statement about the file, since `/Length` "indicates how many bytes of the PDF file are
    /// used for the stream's data" and "[a]ll of these constraints shall be consistent".
    Truncated,
    /// The encoded data is not what the filter's grammar admits, at a definite point in it.
    ///
    /// §7.4.4.1 makes RFC 1951 normative for `FlateDecode` — the Flate method "is fully
    /// defined in Internet RFC 1950 , and Internet RFC 1951 " — so a back-reference past the
    /// start of the window is not a Flate stream, and everything after that point is
    /// unrecoverable. Everything *before* it is not: the decoder emitted it from bits the
    /// producer's compressor wrote.
    Corrupt,
}

/// What a filter stage produced, and whether it is all of it.
///
/// The second field is the whole point of the type. A caller that only wants bytes takes
/// [`decode`] or [`decode_with_parms`] and gets an `Option`; a caller that has to *say*
/// something about the page takes the reported form and reads [`Self::damage`].
#[derive(Debug, Clone)]
pub struct Decoded {
    /// The decoded bytes, which are all of them where [`Self::damage`] is `None`.
    pub data: Arc<[u8]>,
    /// Why the decode stopped short, or `None` where it reached the filter's end-of-data.
    pub damage: Option<Damage>,
}

impl Decoded {
    /// A complete decode.
    fn whole(data: impl Into<Arc<[u8]>>) -> Self {
        Self {
            data: data.into(),
            damage: None,
        }
    }

    /// A decode that stopped short, keeping what it had.
    fn damaged(data: impl Into<Arc<[u8]>>, damage: Damage) -> Self {
        Self {
            data: data.into(),
            damage: Some(damage),
        }
    }
}

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
    decode_with_parms_reported(filter, data, parms, limits)
        .ok()
        .map(|decoded| decoded.data)
}

/// The same, saying which of [`FilterRefusal`]'s three answers it is.
///
/// # Errors
///
/// [`FilterRefusal`], whose variants are the three reasons.
pub fn decode_with_parms_reported(
    filter: &[u8],
    data: &[u8],
    parms: Option<&Dictionary>,
    limits: Limits,
) -> Result<Decoded, FilterRefusal> {
    let decoded = decode_reported(filter, data, parms, limits)?;

    let Some(parms) = parms else {
        return Ok(decoded);
    };
    let predictor = parms
        .get("Predictor")
        .and_then(Object::as_integer)
        .unwrap_or(1);
    if predictor <= 1 {
        return Ok(decoded);
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

    // An undefined predictor is data this reader cannot reverse, which is the `Corrupt` case:
    // it has all the bytes and no rule for them.
    //
    // The stage's own damage survives the predictor rather than being replaced by it: a
    // truncated inflate whose prefix un-predicts cleanly is still a truncated stream, and the
    // rows that came out of it are still short of the rows the file states.
    let unpredicted = apply_predictor(&decoded.data, predictor, colors, bits, columns)
        .ok_or(FilterRefusal::Corrupt)?;
    Ok(Decoded {
        data: unpredicted,
        damage: decoded.damage,
    })
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
/// Returns `None` for an unsupported filter, corrupt data or a decode past
/// [`Limits::max_stream_len`]; [`decode_reported`] says which.
#[must_use]
pub fn decode(
    filter: &[u8],
    data: &[u8],
    parms: Option<&Dictionary>,
    limits: Limits,
) -> Option<Arc<[u8]>> {
    decode_reported(filter, data, parms, limits)
        .ok()
        .map(|decoded| decoded.data)
}

/// Decodes one filter stage, saying which of [`FilterRefusal`]'s three answers a failure is.
///
/// # Errors
///
/// [`FilterRefusal`], whose variants are the three reasons.
pub fn decode_reported(
    filter: &[u8],
    data: &[u8],
    parms: Option<&Dictionary>,
    limits: Limits,
) -> Result<Decoded, FilterRefusal> {
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
        b"ASCIIHexDecode" | b"AHx" => Ok(Decoded::whole(ascii_hex(data))),
        b"ASCII85Decode" | b"A85" => ascii85(data, limits),
        b"RunLengthDecode" | b"RL" => run_length(data, limits),
        // Not a compression filter: it declares that the stream is encrypted, which is
        // handled elsewhere. Passing the data through unchanged is correct.
        b"Crypt" => Ok(Decoded::whole(data)),
        _ => Err(FilterRefusal::Unsupported),
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
/// [`Lzw`] holds the table and [`Lzw::step`] reads one code; the three details that decide
/// whether a decoder is right are stated there, each beside the sentence of the clause it comes
/// from.
///
/// A truncated or corrupt stream keeps what it decoded, for [`flate`]'s reason: a partial
/// content stream still renders most of a page. **A stream that passes
/// [`Limits::max_stream_len`] does not**, and the difference is the whole of ADR 0306: damage
/// means the encoder had no more to give, and the bound means it had a great deal more.
///
/// **The algorithm is [`Lzw`] and this function is a loop over it**, which is the shape
/// `doc/todo/14` asks of a filter that gains a pump: the clause is implemented once and the two
/// routes differ in where the bytes go. What stays here rather than moving into the state is the
/// **bound**, because a bound is a statement about an allocation and the windowed route has none
/// to make it about — there the aggregate bound belongs to the reader that owns the window.
fn lzw(data: &[u8], early_change: bool, limits: Limits) -> Result<Decoded, FilterRefusal> {
    let mut state = Lzw::new(early_change);
    let mut out: Vec<u8> = Vec::new();
    loop {
        match state.step(data) {
            Step::Again => {
                if out.len().saturating_add(state.pending().len()) > limits.max_stream_len {
                    // A bomb rather than a stream: LZW reaches about 1365:1 on long runs of one
                    // byte, so a small file can name a very large output. **What it decoded so
                    // far is discarded rather than handed back**, because a prefix of a bomb
                    // read as a whole stream is exactly the silence this guard exists to break.
                    return Err(FilterRefusal::TooLarge {
                        limit: limits.max_stream_len,
                    });
                }
                out.extend_from_slice(state.pending());
            }
            Step::Ended => return Ok(Decoded::whole(out.as_slice())),
            Step::Damaged(damage) => return salvage(&out, damage),
        }
    }
}

/// Restart with the initial table and a nine-bit code.
const LZW_CLEAR: u16 = 256;
/// End of data.
const LZW_EOD: u16 = 257;
/// The first code the table assigns; 0 to 255 are themselves and 256, 257 are markers.
const LZW_FIRST: u16 = 258;
/// "Codes shall never be longer than 12 bits; therefore, entry 4095 is the last entry."
const LZW_ENTRIES: usize = 4096;

/// What one call of [`Lzw::step`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// A code was read and the sequence it names is [`Lzw::pending`].
    Again,
    /// §7.4.4.2's EOD marker was read: the stream is whole.
    Ended,
    /// The decode stopped short, and the bytes before it are what the encoder wrote.
    Damaged(Damage),
}

/// One `LZWDecode` in progress: §7.4.4.2's table, its bit accumulator and its input cursor.
///
/// **This is the state a decoder keeps between codes, held in a struct so that it can also be
/// kept between *calls*.** [`lzw`] drives it into a growing `Vec` and [`Pump`] drives it into a
/// fixed window; neither owns a copy of the algorithm. Trap 6 is the reason it is one type
/// rather than two functions — a second decoder beside the first is how two implementations of
/// one clause drift — and `doc/todo/14` names this shape outright.
///
/// The table is a prefix code and one byte per entry rather than a growing sequence per entry,
/// which is the standard construction and the reason the clause can say "the encoder and the
/// decoder shall maintain identical copies of this table" without either of them copying
/// anything: entry *n* is entry `prefix[n]` followed by `suffix[n]`, so appending one is two
/// stores.
struct Lzw {
    /// Each entry's predecessor, by code.
    prefix: [u16; LZW_ENTRIES],
    /// Each entry's own last byte, by code.
    suffix: [u8; LZW_ENTRIES],
    /// The first code not yet assigned.
    next: u16,
    /// How many bits the next code occupies, 9 to 12.
    width: u32,
    /// The code before this one, which step (d) needs to create the next entry.
    previous: Option<u16>,
    /// Table 8's `/EarlyChange`, which moves the width increase one code earlier.
    early_change: bool,
    /// The bits read from the input and not yet consumed by a code.
    held: u32,
    /// How many of `held`'s low bits are those.
    bits: u32,
    /// How many of the encoded bytes have been read.
    at: usize,
    /// The sequence the last code named, built backwards by walking the prefix chain and then
    /// reversed. Bounded by the table's size, since a chain longer than that would be a cycle.
    spill: Vec<u8>,
    /// How much of `spill` has been handed over, which is always zero on [`lzw`]'s route and
    /// moves only where a window took the sequence in pieces.
    spill_at: usize,
}

/// Everything but the table, which is twelve kilobytes of it and says nothing a reader of a
/// panic message wants.
impl std::fmt::Debug for Lzw {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Lzw")
            .field("next", &self.next)
            .field("width", &self.width)
            .field("previous", &self.previous)
            .field("early_change", &self.early_change)
            .field("at", &self.at)
            .field("pending", &self.pending().len())
            .finish_non_exhaustive()
    }
}

impl Lzw {
    /// A decoder at §7.4.4.2's initial state.
    fn new(early_change: bool) -> Self {
        Self {
            prefix: [0; LZW_ENTRIES],
            suffix: [0; LZW_ENTRIES],
            next: LZW_FIRST,
            width: 9,
            previous: None,
            early_change,
            held: 0,
            bits: 0,
            at: 0,
            spill: Vec::with_capacity(LZW_ENTRIES),
            spill_at: 0,
        }
    }

    /// The bytes the last [`Step::Again`] produced that nobody has taken yet.
    fn pending(&self) -> &[u8] {
        self.spill.get(self.spill_at..).unwrap_or_default()
    }

    /// Reads one code from `data` and decodes the sequence it names into [`Self::pending`].
    ///
    /// Three details decide whether a decoder is right, and each is a sentence of the clause:
    ///
    /// - **The code width grows before the entry that needs it.** "The first output code that is
    ///   10 bits long shall be the one following the creation of table entry 511", and Table 8's
    ///   `/EarlyChange` moves that one code earlier, which is the default because a
    ///   widely-copied encoder did it. Getting this wrong desynchronises the bit stream from
    ///   that point on and produces plausible bytes for ever after.
    /// - **A code may name the entry about to be created.** The encoder emits the code for a
    ///   sequence it has just added, so a decoder that has not added it yet must reconstruct it:
    ///   it is the previous sequence followed by that sequence's own first byte. This is the
    ///   case an input of one repeated character reaches immediately.
    /// - **Bits are packed high-order first**, across byte boundaries, "thus, codes may straddle
    ///   byte boundaries arbitrarily".
    fn step(&mut self, data: &[u8]) -> Step {
        loop {
            while self.bits < self.width {
                let Some(&byte) = data.get(self.at) else {
                    // No EOD marker: the encoder is required to emit one, and a file that does
                    // not is truncated rather than empty.
                    return Step::Damaged(Damage::Truncated);
                };
                self.at += 1;
                self.held = (self.held << 8) | u32::from(byte);
                self.bits += 8;
            }
            #[expect(
                clippy::cast_possible_truncation,
                reason = "masked to `width` bits, and the clause caps a code at twelve"
            )]
            let code =
                ((self.held >> (self.bits - self.width)) & ((1u32 << self.width) - 1)) as u16;
            self.bits -= self.width;

            if code == LZW_EOD {
                return Step::Ended;
            }
            if code == LZW_CLEAR {
                self.next = LZW_FIRST;
                self.width = 9;
                self.previous = None;
                continue;
            }

            // The sequence this code names, or — where it names the entry about to be
            // created — the one the encoder just added.
            self.spill.clear();
            self.spill_at = 0;
            let mut walk = if code < self.next {
                code
            } else if code == self.next && self.previous.is_some() {
                // Reconstructed below from `previous`; start the walk there and append its
                // own first byte afterwards.
                self.previous.unwrap_or(0)
            } else {
                // A code past the end of the table is corrupt data, not a sequence.
                return Step::Damaged(Damage::Corrupt);
            };
            let extends = code == self.next;
            for _ in 0..LZW_ENTRIES {
                let index = usize::from(walk);
                if walk < 256 {
                    self.spill.push(u8::try_from(walk).unwrap_or(0));
                    break;
                }
                self.spill
                    .push(self.suffix.get(index).copied().unwrap_or(0));
                walk = self.prefix.get(index).copied().unwrap_or(0);
            }
            self.spill.reverse();
            if extends {
                let first = self.spill.first().copied().unwrap_or(0);
                self.spill.push(first);
            }

            // Step (d): "create a new table entry for the first unused code. Its value is
            // the sequence found in step (a) followed by the next input character" — which
            // the decoder sees as the previous code followed by this sequence's first byte.
            if let Some(previous) = self.previous
                && usize::from(self.next) < LZW_ENTRIES
            {
                let index = usize::from(self.next);
                if let Some(slot) = self.prefix.get_mut(index) {
                    *slot = previous;
                }
                if let Some(slot) = self.suffix.get_mut(index) {
                    *slot = self.spill.first().copied().unwrap_or(0);
                }
                self.next += 1;
                let grown = u32::from(self.next) + u32::from(self.early_change);
                if self.width < 12 && grown >= (1u32 << self.width) {
                    self.width += 1;
                }
            }
            self.previous = Some(code);
            return Step::Again;
        }
    }
}

/// What a *damaged* stream hands back: whatever it decoded and why it stopped, or
/// [`FilterRefusal::Corrupt`] where nothing survived at all.
///
/// Only for damage. A stream stopped by [`Limits::max_stream_len`] does not come through here
/// — see [`FilterRefusal::TooLarge`].
fn salvage(out: &[u8], damage: Damage) -> Result<Decoded, FilterRefusal> {
    if out.is_empty() {
        Err(FilterRefusal::Corrupt)
    } else {
        Ok(Decoded::damaged(out, damage))
    }
}

/// Inflates a zlib or raw deflate stream.
///
/// Tries zlib framing first and falls back to raw deflate, because streams missing their
/// two-byte zlib header are common in the wild. Damaged output is kept rather than discarded
/// — a partially-inflated content stream still renders most of a page — and [`Decoded::damage`]
/// says so, which is ADR 0343 and is the half that was missing.
///
/// **A stream stopped by [`Limits::max_stream_len`] is a different case, and one code path used
/// to serve both.** `io::Take` yields end-of-file at its limit and `read_to_end` reports that
/// as `Ok`, so a bomb clamped at the bound came back as a complete decode of its own prefix,
/// with nothing said. The ceiling here is therefore **one byte past** the bound: reaching that
/// byte is what tells the two apart, and it costs one byte of memory to know. ADR 0306.
fn flate(data: &[u8], limits: Limits) -> Result<Decoded, FilterRefusal> {
    // Leading whitespace before the compressed data occurs and confuses the header check.
    let start = data
        .iter()
        .position(|&byte| !crate::lexer::is_whitespace(byte))
        .ok_or(FilterRefusal::Corrupt)?;
    let data = data.get(start..).ok_or(FilterRefusal::Corrupt)?;

    for zlib_header in [true, false] {
        match inflate(data, zlib_header, limits) {
            // Nothing at all came out under this framing, so the other one gets its turn:
            // a raw deflate stream fails zlib's two-byte header check having produced no
            // bytes, which is exactly this case and is why the fallback exists.
            Err(FilterRefusal::Corrupt) => {}
            other => return other,
        }
    }
    Err(FilterRefusal::Corrupt)
}

/// One inflate attempt, under zlib framing or raw deflate.
///
/// **Driven through [`flate2::Decompress`] rather than the `Read` adapter, and the reason is a
/// defect the adapter cannot express.** `read_to_end` learns nothing about *why* the decoder
/// stopped: RFC 1951's final block and an input that simply ran out both arrive as `Ok`, so a
/// truncated stream was indistinguishable from a whole one. Worse, on an actual error the
/// adapter discards whatever the erroring `read` call had already produced, so the prefix this
/// function is supposed to keep survived only as far as the last whole call — 1024 bytes of a
/// partial ICC profile on one witness and *nothing at all* on another, purely by where the
/// damage fell relative to a buffer boundary. Both are ADR 0343.
///
/// The three outcomes are RFC 1951's own: the final block was read (whole), the data violated
/// the format at a definite bit ([`Damage::Corrupt`]), or the input ended first
/// ([`Damage::Truncated`]).
fn inflate(data: &[u8], zlib_header: bool, limits: Limits) -> Result<Decoded, FilterRefusal> {
    let (mut out, stopped) = inflate_buffer(data, zlib_header, limits);
    // **The buffer's slack is resident, and [`finish`] is about to allocate the whole decode
    // beside it.** A decode of L bytes ends in a buffer of up to 2L — the loop doubles, and it
    // cannot know where to stop — and `Arc<[u8]>` is a copy rather than a hand-over, so the peak
    // is capacity plus length. Releasing the slack first turns 2L + L into L + L, measured with
    // `massif` and `ru_maxrss` on §2's Bomb A: **1145 MB → 768 MB**, and on the owner's 50 MB
    // drawing **429 MB → 381 MB**. It is *not* a second copy for the ordinary stream — callgrind
    // over ten opens of ISO 32000-2 and over one interpreted page reads **−0.145%** and
    // **−0.116%**, both slightly cheaper, because the allocator shrinks a large mapping in place
    // and the copy that follows touches fewer pages. ADR 0354.
    out.shrink_to_fit();
    match stopped {
        // A stream this reader *could* have decoded and declined to: the bytes it did inflate
        // are its prefix rather than its content, and handing them over would be the silent
        // clamp ADR 0306 removed.
        Stopped::PastTheBound => Err(FilterRefusal::TooLarge {
            limit: limits.max_stream_len,
        }),
        Stopped::Whole => finish(&out, None, limits),
        Stopped::Damaged(damage) => finish(&out, Some(damage), limits),
    }
}

/// Why [`inflate_buffer`]'s loop stopped, before its bytes are judged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stopped {
    /// RFC 1951's final block was read: the bytes are the whole stream.
    Whole,
    /// The decode stopped short and the bytes before it are what the encoder wrote.
    Damaged(Damage),
    /// The buffer reached one byte past [`Limits::max_stream_len`].
    PastTheBound,
}

/// What one turn of `flate2::Decompress` was, given whether it moved any bytes.
///
/// Shared by the two loops that drive the decoder — [`inflate_buffer`], which grows a `Vec`
/// until the stream ends, and [`Pump`], which fills a fixed window — so that the three
/// outcomes RFC 1951 admits are classified in one place rather than twice. The two loops
/// differ in where they write and in nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Turn {
    /// Bytes moved and the stream has more to give.
    Again,
    /// RFC 1951's final block was read.
    Whole,
    /// The stream stopped short, for this reason.
    Damaged(Damage),
}

/// Classifies one turn of the decoder. See [`Turn`].
fn turn(status: &Result<flate2::Status, flate2::DecompressError>, progressed: bool) -> Turn {
    match status {
        Ok(flate2::Status::StreamEnd) => Turn::Whole,
        Err(_) => Turn::Damaged(Damage::Corrupt),
        // No input read and no output written means the decoder can make no further
        // progress. Output room is guaranteed by both callers, so the only way to be here is
        // an input that ended before RFC 1951's final block — and terminating on it is also
        // what makes both loops provably finite.
        Ok(flate2::Status::Ok | flate2::Status::BufError) => {
            if progressed {
                Turn::Again
            } else {
                Turn::Damaged(Damage::Truncated)
            }
        }
    }
}

/// What one turn of a [`Pump`] produced.
///
/// The window-fed counterpart of [`Decoded`]: that type says what a whole stream is, and this
/// one says what its next few thousand bytes are. Damage is reported the moment the pump meets
/// it rather than at the end, because a reader that has already handed those bytes to a lexer
/// cannot wait to be told (ADR 0343's report, produced as the pump goes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pumped {
    /// This turn wrote that many bytes and the stream has more to give.
    Wrote(usize),
    /// RFC 1951's final block was read: this turn's bytes are the last of the stream.
    Ended(usize),
    /// The decode stopped short of the filter's own end-of-data.
    ///
    /// The bytes this turn wrote stand — they are what the producer's compressor emitted from
    /// bytes the producer wrote — and there are no more.
    Damaged(usize, Damage),
}

/// Which of §7.4's filters a [`Pump`] is to run, and with which of its parameters.
///
/// **The route is chosen once, by `Document::pumping`, and carried rather than re-derived.**
/// One of §7.8.2's content streams is read more than once — a form, a tiling cell, a glyph
/// description — so a fresh pump is made per read, and a value that says which decoder to build
/// is what keeps the second read from asking the question again and answering it differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pumping {
    /// `FlateDecode`, with no predictor.
    Inflate,
    /// `LZWDecode`, with no predictor.
    Lzw {
        /// Table 8's `/EarlyChange`, which decides where the code width grows and therefore
        /// what every bit after that point decodes to. Its default is 1, hence `true`.
        early_change: bool,
    },
}

/// A decode in progress, producing its output a window at a time.
///
/// [`decode`] and its neighbours answer "what does this stream decode to", which is a
/// question with an allocation in it. This one answers "what are the next few thousand bytes
/// of it", which is the question a content stream read through a fixed window asks — and the
/// difference is the whole of road D in `doc/todo/14`: a bomb becomes time rather than an
/// allocation nothing can take back. ADR 0365.
///
/// **The bytes are the same bytes**, and that is a property of how the two routes are built
/// rather than a hope: each filter is one resumable decoder, and the whole-buffer entry point is
/// a loop over the same decoder. `FlateDecode` shares [`turn`] between its two loops;
/// `LZWDecode` shares [`Lzw`] itself.
#[derive(Debug)]
pub struct Pump {
    /// The still-encoded bytes, held whole because they are already resident: the pump takes
    /// its input from the stream object rather than copying it.
    data: Arc<[u8]>,
    /// The decoder, held across turns — which is what makes this a pump rather than a decode.
    engine: Engine,
    /// Set once end-of-data or damage has been reported, so that a further turn is a no-op
    /// rather than a second report.
    finished: bool,
}

/// The decoder a [`Pump`] holds across its turns.
#[derive(Debug)]
enum Engine {
    /// `FlateDecode`, driven through `flate2::Decompress`.
    Inflate(Inflate),
    /// `LZWDecode`. **Boxed**, because §7.4.4.2's table is twelve kilobytes and an inflating
    /// pump would otherwise carry room for one it will never fill.
    Lzw(Box<Lzw>),
}

/// One `FlateDecode` in progress. See [`Pump`].
#[derive(Debug)]
struct Inflate {
    /// The decoder, held across turns.
    decoder: flate2::Decompress,
    /// Whether `decoder` expects zlib's two-byte header.
    zlib_header: bool,
    /// How many of the encoded bytes the decoder has taken, from `start`.
    consumed: usize,
    /// Where the encoded data begins; [`flate`] skips leading white space before the header,
    /// and a resumable decoder has to skip the same bytes.
    start: usize,
    /// How much output the pump has produced, which is what says whether a restart under the
    /// other framing is still free.
    produced: u64,
}

impl Engine {
    /// The decoder `pumping` names, positioned at the start of `data`.
    ///
    /// `FlateDecode`'s white-space skip is [`flate`]'s, kept exactly, and the zlib-then-raw
    /// fallback is taken later by [`Inflate::pump`]: a stream missing its two-byte header is
    /// common in the wild, and a decoder that has produced nothing yet can be restarted under
    /// the other framing for nothing.
    fn new(pumping: Pumping, data: &[u8]) -> Self {
        match pumping {
            Pumping::Inflate => {
                let start = data
                    .iter()
                    .position(|&byte| !crate::lexer::is_whitespace(byte))
                    .unwrap_or(data.len());
                Self::Inflate(Inflate {
                    decoder: flate2::Decompress::new(true),
                    zlib_header: true,
                    consumed: 0,
                    start,
                    produced: 0,
                })
            }
            Pumping::Lzw { early_change } => Self::Lzw(Box::new(Lzw::new(early_change))),
        }
    }

    /// How many of the encoded bytes the decoder has taken, counting from the start of `data`.
    fn consumed(&self) -> usize {
        match self {
            Self::Inflate(inflate) => inflate.start.saturating_add(inflate.consumed),
            Self::Lzw(lzw) => lzw.at,
        }
    }

    /// One turn of whichever decoder this is. See [`Pump::pump`].
    fn pump(&mut self, data: &[u8], out: &mut [u8]) -> Pumped {
        match self {
            Self::Inflate(inflate) => inflate.pump(data, out),
            Self::Lzw(lzw) => lzw.pump(data, out),
        }
    }
}

/// What a filter's own end-of-data marker says about how far its input reaches.
///
/// See [`encoded_extent`]. The three answers are kept apart because a caller reading through a
/// window has to tell "these bytes carry no marker" from "these bytes ran out before one",
/// which are a statement about the file and a statement about the buffer respectively.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodedExtent {
    /// The marker stands after this many of the input's bytes.
    Ends(usize),
    /// The input ran out before the marker did, so the answer is past the bytes offered.
    Short,
    /// No marker is to be found in these bytes: the data is corrupt before one, or the decode
    /// outran the ceiling it was given.
    Unknown,
}

/// How many of `data`'s bytes the filter's own end-of-data marker delimits.
///
/// ISO 32000-2 §7.3.8.2 is what makes this question answerable at all:
///
/// > In addition, most filters are defined so that the data shall be self-limiting; that is,
/// > they use an encoding scheme in which an explicit end-of-data (EOD) marker delimits the
/// > extent of the data.
///
/// Nothing in a *file* needs it, because Table 5 makes `/Length` required and every stream
/// object states one. §8.9.7's inline image is the exception the clause exists for: it is
/// written into a content stream with no `/Length` before PDF 2.0, so where its encoded data
/// ends is the filter's answer to give.
/// [`Document::filtered_extent`](crate::Document::filtered_extent) is the caller, and
/// `pdf_model::inline_image` is what asks it.
///
/// `ceiling` bounds the *output*, which is thrown away a window at a time and never held: it
/// is the same number [`decode_reported`] spends on an allocation and here it buys time
/// instead, so that a decompression bomb whose marker is a gibibyte away costs neither.
///
/// **The cost is one decode, and the caller pays for a second one afterwards.** This runs the
/// filter to find where it stops and keeps nothing; the bytes are then decoded again by
/// whoever wanted them. That is deliberate — the alternative is a decoded buffer of unbounded
/// size held across a scan whose whole purpose is to avoid one — and it replaces a linear
/// search over the same bytes, so the population it runs on is one where a walk over those
/// bytes was the cost already.
#[must_use]
pub fn encoded_extent(pumping: Pumping, data: &[u8], ceiling: usize) -> EncodedExtent {
    // Room for one turn's output, which is thrown away. §7.4.4.2 caps an `LZWDecode` entry at
    // 4096 bytes and `Lzw::pump` hands a longer sequence over in pieces, so any size works and
    // this one is a page of them.
    let mut sink = [0u8; 8192];
    let mut engine = Engine::new(pumping, data);
    let mut produced = 0usize;
    loop {
        match engine.pump(data, &mut sink) {
            Pumped::Wrote(wrote) => {
                produced = produced.saturating_add(wrote);
                if produced > ceiling {
                    return EncodedExtent::Unknown;
                }
            }
            Pumped::Ended(_) => return EncodedExtent::Ends(engine.consumed().min(data.len())),
            // The input ending before the marker is what [`Damage::Truncated`] is, and it is
            // the one damage a longer buffer could still answer.
            Pumped::Damaged(_, Damage::Truncated) => return EncodedExtent::Short,
            Pumped::Damaged(_, _) => return EncodedExtent::Unknown,
        }
    }
}

impl Pump {
    /// A pump over `data`, decoding it as [`decode_reported`] would.
    ///
    /// `FlateDecode`'s white-space skip and zlib-then-raw fallback are [`flate`]'s, kept
    /// exactly: a stream missing its two-byte header is common in the wild, and a decoder that
    /// has produced nothing yet can be restarted under the other framing for nothing.
    #[must_use]
    pub fn new(pumping: Pumping, data: Arc<[u8]>) -> Self {
        let engine = Engine::new(pumping, &data);
        Self {
            data,
            engine,
            finished: false,
        }
    }

    /// Which filter this pump runs, so that a second read of the same stream builds the same
    /// decoder without asking the document again.
    #[must_use]
    pub fn pumping(&self) -> Pumping {
        match &self.engine {
            Engine::Inflate(_) => Pumping::Inflate,
            Engine::Lzw(lzw) => Pumping::Lzw {
                early_change: lzw.early_change,
            },
        }
    }

    /// Writes the next bytes of the decoded stream into `out`.
    ///
    /// `out` must not be empty: a decoder given no room makes no progress, and for
    /// `FlateDecode` no progress is how [`turn`] recognises a truncated input.
    pub fn pump(&mut self, out: &mut [u8]) -> Pumped {
        if self.finished || out.is_empty() {
            return Pumped::Wrote(0);
        }
        let pumped = self.engine.pump(&self.data, out);
        if matches!(pumped, Pumped::Ended(_) | Pumped::Damaged(_, _)) {
            self.finished = true;
        }
        pumped
    }
}

impl Inflate {
    /// One turn of the inflate, writing into `out`. See [`Pump::pump`].
    fn pump(&mut self, data: &[u8], out: &mut [u8]) -> Pumped {
        loop {
            let input = data
                .get(self.start.saturating_add(self.consumed)..)
                .unwrap_or_default();
            let (before_in, before_out) = (self.decoder.total_in(), self.decoder.total_out());
            let status = self
                .decoder
                .decompress(input, out, flate2::FlushDecompress::None);
            let took = self.decoder.total_in().saturating_sub(before_in);
            let wrote = usize::try_from(self.decoder.total_out().saturating_sub(before_out))
                .unwrap_or(usize::MAX);
            self.consumed = self
                .consumed
                .saturating_add(usize::try_from(took).unwrap_or(usize::MAX));
            self.produced = self.produced.saturating_add(wrote as u64);

            match turn(&status, took > 0 || wrote > 0) {
                Turn::Again => return Pumped::Wrote(wrote),
                Turn::Whole => return Pumped::Ended(wrote),
                // Nothing has come out under this framing, so the other one gets its turn —
                // [`flate`]'s fallback, taken here at the point the first framing fails
                // rather than after a whole decode. A restart is free exactly while the
                // pump has produced nothing, because there is nothing to un-hand-over.
                Turn::Damaged(damage) => {
                    if self.produced == 0 && self.zlib_header {
                        self.zlib_header = false;
                        self.decoder = flate2::Decompress::new(false);
                        self.consumed = 0;
                        continue;
                    }
                    return Pumped::Damaged(wrote, damage);
                }
            }
        }
    }
}

impl Lzw {
    /// One turn of the LZW decode, writing into `out`. See [`Pump::pump`].
    ///
    /// **A code names a sequence and a window has room for however much it has room for**, so
    /// the sequence the last [`Self::step`] produced is handed over in pieces across as many
    /// turns as it takes. That is the only thing this route has that [`lzw`]'s has not: an
    /// entry can be 4096 bytes and a window is not obliged to be larger than one.
    fn pump(&mut self, data: &[u8], out: &mut [u8]) -> Pumped {
        let mut wrote = 0usize;
        let mut stopped: Option<Step> = None;
        loop {
            let room = out.len().saturating_sub(wrote);
            let pending = self.pending();
            let take = pending.len().min(room);
            if take > 0
                && let Some(slot) = out.get_mut(wrote..wrote.saturating_add(take))
                && let Some(source) = self
                    .spill
                    .get(self.spill_at..self.spill_at.saturating_add(take))
            {
                slot.copy_from_slice(source);
                wrote = wrote.saturating_add(take);
                self.spill_at = self.spill_at.saturating_add(take);
            }
            if self.spill_at < self.spill.len() {
                // The window filled before the sequence did; the rest is the next turn's.
                return Pumped::Wrote(wrote);
            }
            match stopped {
                Some(Step::Ended) => return Pumped::Ended(wrote),
                Some(Step::Damaged(damage)) => return Pumped::Damaged(wrote, damage),
                // `step` never hands back `Again` as a stopping reason.
                Some(Step::Again) | None => {}
            }
            if wrote >= out.len() {
                return Pumped::Wrote(wrote);
            }
            match self.step(data) {
                Step::Again => {}
                ended => stopped = Some(ended),
            }
        }
    }
}

/// The inflate loop itself, handing back the buffer rather than a decode.
///
/// **Split from [`inflate`] so that the buffer can be asserted on**, which is not a shape
/// preference: the bound this function honours is a statement about an *allocation*, and an
/// `Arc<[u8]>` has no capacity to read. `an_inflate_never_buys_a_buffer_past_the_bound` is the
/// test, and it fails with `reserve` in place of `reserve_exact` below. ADR 0354.
fn inflate_buffer(data: &[u8], zlib_header: bool, limits: Limits) -> (Vec<u8>, Stopped) {
    /// The smallest buffer worth asking the allocator for, and the step it grows by.
    const FLOOR: usize = 4096;

    // One byte past the bound, so that reaching it is what says "there was more" — ADR 0306's
    // distinction, kept exactly. The buffer never grows past it, so a bomb costs the bound
    // rather than whatever it claims to inflate to.
    let ceiling = limits.max_stream_len.saturating_add(1);
    // Real streams inflate by roughly three to five times, so this is one allocation for most
    // of them rather than a ladder of doublings from nothing. The ceiling wins over the floor
    // rather than the other way round, because a test may set the bound to sixty-four and an
    // allocation of `FLOOR` under a ceiling of 65 would ask for more room than the bound allows.
    let initial = data.len().saturating_mul(4).max(FLOOR).min(ceiling);
    let mut out: Vec<u8> = Vec::with_capacity(initial);
    let mut decoder = flate2::Decompress::new(zlib_header);
    let mut consumed = 0usize;

    loop {
        if out.len() >= ceiling {
            return (out, Stopped::PastTheBound);
        }
        if out.len() == out.capacity() {
            let room = ceiling.saturating_sub(out.capacity());
            // **`reserve_exact`, and the difference between it and `reserve` is the bound.**
            // `Vec::reserve` grows *amortised* — it takes `max(2 × capacity, len + additional)` —
            // so the step computed here was a floor rather than a ceiling and the last step
            // before the bound doubled straight past it. A gibibyte bound bought a 1.76 GiB
            // buffer: §2's Bomb B peaked at 1811 MB of resident memory where the bound promises
            // 1024 MiB, and the comment above claiming the buffer never grows past the ceiling
            // was false for the one input it is written for. `reserve_exact` takes the step as
            // stated, which is still a doubling everywhere below the bound — `len == capacity`
            // here, so the step *is* the capacity — and is exactly the room left at it.
            //
            // `room` cannot be zero: reaching a capacity of `ceiling` means the length check
            // above fired first, and every other capacity leaves at least one byte.
            out.reserve_exact(out.capacity().max(FLOOR).min(room));
        }

        let input = data.get(consumed..).unwrap_or_default();
        let (before_in, before_out) = (decoder.total_in(), out.len());
        let status = decoder.decompress_vec(input, &mut out, flate2::FlushDecompress::None);
        consumed = consumed.saturating_add(
            usize::try_from(decoder.total_in().saturating_sub(before_in)).unwrap_or(usize::MAX),
        );

        let progressed = decoder.total_in() != before_in || out.len() != before_out;
        match turn(&status, progressed) {
            Turn::Again => {}
            Turn::Whole => return (out, Stopped::Whole),
            Turn::Damaged(damage) => return (out, Stopped::Damaged(damage)),
        }
    }
}

/// Hands back an inflate's bytes, or refuses them for the bound, or refuses an empty prefix.
///
/// A *damaged* decode that produced nothing is [`FilterRefusal::Corrupt`] rather than an empty
/// [`Decoded`], because [`flate`] reads that as "this framing produced nothing, try the other
/// one" — which is how a raw deflate stream gets past zlib's header check. A **whole** decode
/// that produced nothing is not the same thing and is handed back as itself: a stream whose
/// producer deflated zero bytes is a conforming stream that decodes to zero bytes.
fn finish(out: &[u8], damage: Option<Damage>, limits: Limits) -> Result<Decoded, FilterRefusal> {
    if out.len() > limits.max_stream_len {
        return Err(FilterRefusal::TooLarge {
            limit: limits.max_stream_len,
        });
    }
    match damage {
        Some(_) if out.is_empty() => Err(FilterRefusal::Corrupt),
        Some(damage) => Ok(Decoded::damaged(out, damage)),
        None => Ok(Decoded::whole(out)),
    }
}

/// Decodes `ASCIIHexDecode`, ISO 32000-2 §7.4.2: hex digits terminated by `>`.
///
/// > The ASCIIHexDecode filter shall produce one byte of binary data for each pair of ASCII
/// > hexadecimal digits (0 -9 and A -F or a -f). All white-space characters (see 7.2, "Lexical
/// > conventions") shall be ignored. A GREATER-THAN SIGN (3Eh) indicates EOD (End Of Data).
/// > Any other characters shall cause an error. If the filter encounters the EOD marker after
/// > reading an odd number of hexadecimal digits, it shall behave as if a 0 (zero) followed
/// > the last digit.
///
/// Four of those five sentences are executed here. The fourth is a **deliberate departure**:
/// a stray byte is skipped rather than made an error, because a hex stream with one bad byte
/// in it decodes to what its producer meant everywhere except at that byte, and refusing loses
/// the whole stream. §7.4.3's identical sentence *is* enforced, in [`ascii85`], and the
/// difference between the two is that base-85 has no way to resynchronise: a skipped character
/// shifts every group after it, so what would come back is not the producer's data. The ledger
/// carries the departure as this clause's `partial`, and
/// `ascii_hex_skips_a_character_the_clause_calls_an_error` is what pins it.
///
/// **The one filter here that takes no [`Limits`], and the reason is the clause's arithmetic
/// rather than an omission.** One byte comes out for every two that go in, so this is a 2:1
/// contraction and no file can inflate through it; the raw bytes are already bounded by
/// `max_stream_len` where the parser reads them (`Parser::parse_stream_data`), which is what makes
/// that bound "raw or decoded" rather than only the second. [`Document::pumping`] states the
/// same ratio for the road not taken.
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
fn ascii85(data: &[u8], limits: Limits) -> Result<Decoded, FilterRefusal> {
    let mut out = Vec::new();
    let mut group = [0u8; 5];
    let mut count = 0usize;

    // An optional `<~` introduces the data.
    let body = if data.starts_with(b"<~") {
        data.get(2..).unwrap_or_default()
    } else {
        data
    };

    // **Checked here rather than beside the two places that push**, because one of those is
    // `z` and it used to `continue` straight past the guard: eight `z` under a bound of eight
    // produced thirty-two bytes and reported nothing, which is the same defect this file's
    // other two filters had and was found by asserting that all four agree. One input byte
    // yields at most four output bytes, so the overshoot inside the loop is four; the check
    // after it is what catches a group that landed on the last byte.
    for byte in body.iter().copied() {
        if out.len() > limits.max_stream_len {
            return Err(FilterRefusal::TooLarge {
                limit: limits.max_stream_len,
            });
        }
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
            return Err(FilterRefusal::Corrupt);
        }

        if let Some(slot) = group.get_mut(count) {
            *slot = byte - b'!';
        }
        count = count.saturating_add(1);

        if count == 5 {
            push_ascii85_group(&mut out, group, 5);
            count = 0;
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

    if out.len() > limits.max_stream_len {
        return Err(FilterRefusal::TooLarge {
            limit: limits.max_stream_len,
        });
    }
    // No [`Damage`] here, and §7.4.3 is why: a final partial group is not damage but the
    // clause's own encoding — "[i]f the length of the data to be encoded is not a multiple of
    // 4 bytes, the last, partial group of 4 shall be used to produce a last, partial group of
    // 5 output characters" — and anything the grammar does not admit "shall cause an error",
    // which is [`FilterRefusal::Corrupt`] above rather than a prefix.
    Ok(Decoded::whole(out.as_slice()))
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

/// Decodes `RunLengthDecode`, ISO 32000-2 §7.4.5.
///
/// > A length value of 128 shall denote EOD.
///
/// So data that ends without one has ended early, and [`Damage::Truncated`] says so: the
/// runs that were read are the encoder's own and the page draws them, but the file states a
/// stream that does not finish. The same shape as [`flate`]'s and [`lzw`]'s, and it was
/// silent for the same reason until ADR 0343.
fn run_length(data: &[u8], limits: Limits) -> Result<Decoded, FilterRefusal> {
    let mut out = Vec::new();
    let mut at = 0usize;
    let mut ended = false;

    while let Some(&length) = data.get(at) {
        at = at.saturating_add(1);
        match length {
            // 128 marks the end of the data.
            128 => {
                ended = true;
                break;
            }
            // 0..=127: copy the next length + 1 bytes literally.
            0..=127 => {
                let run = usize::from(length).saturating_add(1);
                // **Running out of input inside a run is the same statement as running out
                // before the EOD**, and it used to throw away everything decoded so far.
                // §7.4.5 gives this filter no invalid byte — every header value is a legal
                // run length and 128 is the EOD — so the only way it can fail is the input
                // ending early, which is [`Damage::Truncated`] and not
                // [`FilterRefusal::Corrupt`]. The runs already read are the encoder's own.
                let Some(slice) = data.get(at..at.saturating_add(run)) else {
                    break;
                };
                out.extend_from_slice(slice);
                at = at.saturating_add(run);
            }
            // 129..=255: repeat the next byte 257 - length times.
            _ => {
                // The same, one byte earlier: a repeat header with no byte after it.
                let Some(&byte) = data.get(at) else {
                    break;
                };
                at = at.saturating_add(1);
                let run = 257usize.saturating_sub(usize::from(length));
                out.resize(out.len().saturating_add(run), byte);
            }
        }
        if out.len() > limits.max_stream_len {
            return Err(FilterRefusal::TooLarge {
                limit: limits.max_stream_len,
            });
        }
    }

    if ended {
        Ok(Decoded::whole(out.as_slice()))
    } else {
        salvage(&out, Damage::Truncated)
    }
}

/// Four tests here decline to run under Miri, and each says so itself.
///
/// **Three of the four name `zlib-rs`. The fourth names its own input**, and the difference is
/// worth keeping: [`an_lzw_bomb_costs_the_window_rather_than_its_decode`] interprets correctly
/// and cost 50 minutes 45 seconds of the `nightly` job's one-hour ceiling doing it, on 7 MB of
/// decoded bomb that the neighbouring tests already put through the same decoder at a
/// hundredth of the size. A declination for *cost* is a weaker thing than one for a
/// dependency's unsafe, so it says what it gives up in its own doc comment rather than here.
/// ADR 0463.
///
/// **The defect is `zlib-rs` 0.6.6's and not this tree's**: it deallocates through a pointer other
/// than the one its allocation was made through, which *both* of Miri's aliasing models reject —
/// Stacked Borrows with "deallocating while item is strongly protected", Tree Borrows with
/// "deallocation through the root of the allocation is forbidden". `doc/todo/52` holds the
/// reduction and the report owed upstream, and says what to delete when a fixed version lands.
///
/// **Why the declination is here rather than in the workflow.** It used to be `--skip flate` on
/// CI's Miri line, and a name-substring filter turned out to be a poor instrument twice over: it
/// silently took a *third* test with it — `an_inflate_never_buys_a_buffer_past_the_bound`, whose
/// name contains `flate` inside `inflate` — while the note beside it said two; and it could only
/// ever exclude what somebody had remembered to name in a file the test does not live in, which
/// is how a second dependency's unsafe went unnoticed until it failed the job. A test that must
/// not run under an interpreter declines *by itself*, for the same reason `doc/todo/02` §2 gives
/// for a gate binary's: an invocation can be copied without its guard, and a test cannot be run
/// without itself. ADR 0450.
#[cfg(test)]
mod tests {
    use super::{Dictionary, Limits, Object, Stopped, decode, inflate_buffer};
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

    /// §7.4.2 says a stray character "shall cause an error", and this decoder recovers instead.
    ///
    /// **A deliberate departure, and until this test only prose said so** — the ledger row for
    /// the clause and a comment in [`ascii_hex`]. A departure nothing exercises is one a later
    /// round can undo by accident and every gate will agree with, which is the same argument
    /// the rest of this module's tests are written from. What is pinned is the recovery: the
    /// bytes on either side of the stray character are the producer's and are decoded, so a
    /// hex stream with one bad byte in it loses that byte rather than the page.
    #[test]
    fn ascii_hex_skips_a_character_the_clause_calls_an_error() {
        let out = decode(b"ASCIIHexDecode", b"48 65 6C*6C 6F>", None, Limits::DEFAULT)
            .expect("the stray asterisk is skipped, not refused");
        assert_eq!(&*out, b"Hello", "and the white space is ignored beside it");
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

    /// A stream ending *inside* a run keeps the runs before it, like one ending before the EOD.
    ///
    /// §7.4.5 gives `RunLengthDecode` no invalid byte — every header 0 to 127 is a literal run,
    /// 129 to 255 a repeat and 128 the EOD — so the only way it can fail is the data running
    /// out, which is [`Damage::Truncated`] however far into a run it happens. Both endings used
    /// to be [`FilterRefusal::Corrupt`] and threw the decoded prefix away.
    ///
    /// **The witness is a whole page**: a crawled 1216×1753 bilevel scan whose run-length data
    /// decodes to exactly the 266 456 bytes its dictionary describes and then carries one more
    /// run header with no bytes after it. This tree drew nothing and `poppler`, `mupdf` and
    /// `ghostscript` each drew the scan (session 613).
    #[test]
    fn run_length_keeps_what_it_decoded_when_the_data_ends_inside_a_run() {
        for tail in [
            // A literal run of three bytes with only one of them present.
            vec![2u8, b'd'],
            // A repeat header with no byte to repeat.
            vec![254u8],
        ] {
            let mut data = vec![2, b'a', b'b', b'c'];
            data.extend_from_slice(&tail);
            let decoded = super::decode_reported(b"RunLengthDecode", &data, None, Limits::DEFAULT)
                .expect("the runs before the damage are the encoder's own");
            assert_eq!(&*decoded.data, b"abc", "the whole prefix survives");
            assert_eq!(
                decoded.damage,
                Some(super::Damage::Truncated),
                "and the stream is reported as ending early rather than as corrupt"
            );
        }
    }

    #[test]
    fn ascii85_round_trips_a_known_value() {
        // "Man " encodes as "9jqo" in base 85 terms; use the canonical empty and 'z' cases.
        let out = decode(b"ASCII85Decode", b"z~>", None, Limits::DEFAULT).expect("valid");
        assert_eq!(&*out, &[0, 0, 0, 0], "'z' stands for four zero bytes");
    }

    #[cfg_attr(
        miri,
        ignore = "zlib-rs's deallocation, not this tree's — see the note above"
    )]
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
    #[cfg_attr(
        miri,
        ignore = "zlib-rs's deallocation, not this tree's — see the note above"
    )]
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

    /// `max_stream_len` bounds an allocation, so the allocation is what the test reads.
    ///
    /// **This is the assertion `tests/stream_length_bound.rs` cannot make.** That file checks the
    /// *report* a bomb gets, which was already right; the buffer behind it was twice the bound,
    /// because `Vec::reserve` grows amortised and the last step before the ceiling doubled past
    /// it. Nothing observable from outside `inflate` changes when that happens — the refusal is
    /// the same refusal — which is why the defect survived the round that wrote the loop and the
    /// round that measured its output. Reading `capacity()` is the only instrument that sees it,
    /// and [`inflate_buffer`] exists to hand it over. ADR 0354.
    #[cfg_attr(
        miri,
        ignore = "zlib-rs's deallocation, not this tree's — see the note above"
    )]
    #[test]
    fn an_inflate_never_buys_a_buffer_past_the_bound() {
        use std::io::Write as _;
        // 4 MiB of one byte deflates to a few kilobytes: a bomb in the small, so that the test
        // costs a bound's worth of memory rather than a bomb's.
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
        encoder
            .write_all(&vec![b'n'; 4 << 20])
            .expect("in-memory write");
        let compressed = encoder.finish().expect("finish");

        let limits = Limits {
            max_stream_len: 1 << 16,
            ..Limits::DEFAULT
        };
        let ceiling = limits.max_stream_len + 1;
        let (out, stopped) = inflate_buffer(&compressed, true, limits);

        assert_eq!(stopped, Stopped::PastTheBound, "the bomb is past the bound");
        assert!(
            out.capacity() <= ceiling,
            "a bound of {} bought a buffer of {}",
            limits.max_stream_len,
            out.capacity()
        );
    }

    /// Packs codes at the width §7.4.4.2's decoder reads each of them at.
    ///
    /// The mirror of [`super::Lzw::step`]'s width rule, written out here rather than shared,
    /// because a test that computed the width by asking the decoder would agree with it however
    /// wrong both were. This is an *encoder*; the thing under test is the decoder.
    fn pack_lzw(codes: &[u16], early_change: bool) -> Vec<u8> {
        let mut out = Vec::new();
        let mut held: u32 = 0;
        let mut bits: u32 = 0;
        let mut width = 9u32;
        let mut next: u16 = 258;
        let mut previous: Option<u16> = None;

        for &code in codes {
            held = (held << width) | (u32::from(code) & ((1u32 << width) - 1));
            bits += width;
            while bits >= 8 {
                out.push(((held >> (bits - 8)) & 0xFF) as u8);
                bits -= 8;
            }
            match code {
                256 => {
                    next = 258;
                    width = 9;
                    previous = None;
                }
                257 => {}
                _ => {
                    if previous.is_some() && usize::from(next) < 4096 {
                        next += 1;
                        let grown = u32::from(next) + u32::from(early_change);
                        if width < 12 && grown >= (1u32 << width) {
                            width += 1;
                        }
                    }
                    previous = Some(code);
                }
            }
        }
        if bits > 0 {
            out.push(((held << (8 - bits)) & 0xFF) as u8);
        }
        out
    }

    /// A stream of `entries - 258` codes that names ever longer runs of one byte.
    ///
    /// This is the shape LZW reaches about 1365:1 on: after the literal, every code names the
    /// entry the decoder is about to create, so entry *n* is *n* − 256 bytes long and the output
    /// grows with the square of the number of codes while the input grows linearly.
    fn lzw_bomb(entries: u16) -> Vec<u8> {
        let mut codes = vec![256u16, u16::from(b'A')];
        codes.extend(258..entries);
        codes.push(257);
        pack_lzw(&codes, true)
    }

    /// Everything a [`super::Pump`] hands over, in windows of `window` bytes, and how it ended.
    fn drain(pump: &mut super::Pump, window: usize) -> (Vec<u8>, super::Pumped) {
        let mut out = Vec::new();
        let mut buffer = vec![0u8; window];
        loop {
            let pumped = pump.pump(&mut buffer);
            let (wrote, done) = match pumped {
                super::Pumped::Wrote(wrote) => (wrote, false),
                super::Pumped::Ended(wrote) | super::Pumped::Damaged(wrote, _) => (wrote, true),
            };
            out.extend_from_slice(buffer.get(..wrote).unwrap_or_default());
            if done {
                return (out, pumped);
            }
        }
    }

    /// The two routes through §7.4.4.2 are one decoder, so they agree byte for byte.
    ///
    /// The window sizes are deliberately smaller than a table entry: an entry may be 4096 bytes
    /// and a window is not obliged to be larger than one, so a window of **one byte** is the
    /// case that exercises handing a sequence over in pieces. `doc/todo/14`'s road is only worth
    /// taking if the bytes are the same bytes, and this is what says so.
    #[test]
    fn an_lzw_pump_and_the_whole_decode_agree() {
        let clauses_example: &[u8] = &[0x80, 0x0B, 0x60, 0x50, 0x22, 0x0C, 0x0C, 0x85, 0x01];
        let bomb = lzw_bomb(600);
        for data in [clauses_example, bomb.as_slice()] {
            let whole = decode(b"LZWDecode", data, None, Limits::DEFAULT).expect("decodes whole");
            for window in [1usize, 3, 7, 64, 4096] {
                let mut pump = super::Pump::new(
                    super::Pumping::Lzw { early_change: true },
                    std::sync::Arc::from(data),
                );
                let (pumped, end) = drain(&mut pump, window);
                assert_eq!(
                    pumped.as_slice(),
                    &*whole,
                    "a window of {window} bytes read something else"
                );
                assert!(
                    matches!(end, super::Pumped::Ended(_)),
                    "a window of {window} bytes ended as {end:?}"
                );
            }
        }
    }

    /// `/EarlyChange` decides the bit stream, and the pump reads it from the route decision.
    #[test]
    fn the_lzw_pump_reads_early_change() {
        let literals: Vec<u16> = (0..300u16).map(|index| index % 256).collect();
        let packed = pack_lzw(&literals, false);
        let mut parms = Dictionary::new();
        parms.insert(Name::new(b"EarlyChange".as_slice()), Object::Integer(0));
        let whole = decode(b"LZWDecode", &packed, Some(&parms), Limits::DEFAULT).expect("decodes");

        let mut late = super::Pump::new(
            super::Pumping::Lzw {
                early_change: false,
            },
            std::sync::Arc::from(packed.as_slice()),
        );
        assert_eq!(drain(&mut late, 16).0.as_slice(), &*whole);

        let mut early = super::Pump::new(
            super::Pumping::Lzw { early_change: true },
            std::sync::Arc::from(packed.as_slice()),
        );
        assert_ne!(
            drain(&mut early, 16).0.as_slice(),
            &*whole,
            "the two settings read the same bytes as different codes"
        );
    }

    /// Damage is the same statement about the stream on both routes, and reaches the window
    /// where it falls rather than at the end. ADR 0343.
    #[test]
    fn the_lzw_pump_reports_the_damage_the_whole_decode_does() {
        // A code past the end of the table: two literals leave the first unused code at 259.
        let corrupt = pack_lzw(&[65, 66, 400], true);
        // Codes with no EOD after them.
        let truncated = pack_lzw(&[65, 66, 67], true);

        for (data, damage) in [
            (corrupt, super::Damage::Corrupt),
            (truncated, super::Damage::Truncated),
        ] {
            let whole = super::decode_reported(b"LZWDecode", &data, None, Limits::DEFAULT)
                .expect("a prefix survives");
            assert_eq!(whole.damage, Some(damage));

            let mut pump = super::Pump::new(
                super::Pumping::Lzw { early_change: true },
                std::sync::Arc::from(data.as_slice()),
            );
            let (bytes, end) = drain(&mut pump, 2);
            assert_eq!(bytes.as_slice(), &*whole.data);
            assert!(
                matches!(end, super::Pumped::Damaged(_, met) if met == damage),
                "the window ended as {end:?}"
            );
        }
    }

    /// **The kind of the quantity changes, which is the whole of road D.**
    ///
    /// The same bytes are a refusal on the whole route — [`Limits::max_stream_len`] is a bound
    /// on an allocation and the decode wants more than it — and on the windowed route they are
    /// simply read, in a buffer that never grows. `doc/todo/14`; the bound the windowed route
    /// still answers to is the *reader's* aggregate one, which `pdf_model::content` applies.
    ///
    /// # It declines to run under Miri, and the reason is its own *size* rather than a dependency
    ///
    /// The three declinations above name somebody else's `unsafe`. This one names its input.
    /// `lzw_bomb(4096)` decodes to 7 370 880 bytes — that is the point of it — and the
    /// interpreter is four orders of magnitude slower than the processor, so this **one test
    /// took 50 minutes 45 seconds** of the `nightly` job's one-hour ceiling on 2026-08-20
    /// (run 32411230902, `filter::tests::an_lzw_bomb… ok` at 20:50:43 against 19:59:58 for the
    /// test before it), and the job was cancelled four minutes into the next test with 57 of
    /// this crate's 92 still unrun. That is the whole of the discrepancy the
    /// six-hundred-and-fourteenth session left open and twice mis-attributed to `sccache`.
    ///
    /// Nothing was bought for those fifty minutes. This module is under
    /// `#![forbid(unsafe_code)]`; what the test asserts is a *bound* — that the window never
    /// grew and that the whole route refuses above [`Limits::max_stream_len`] — and a bound is
    /// a resource question rather than an aliasing one, which is the only kind Miri answers.
    ///
    /// **What declining costs is one input size and no code path.**
    /// [`an_lzw_pump_and_the_whole_decode_agree`] drives the same [`Pump`] over the same shape
    /// of bomb under the interpreter, in windows of one byte to 4096, so every line of the
    /// decoder is still interpreted — on 60 KB instead of 7 MB. Outside Miri this test runs
    /// unchanged, at full size, on every gate.
    ///
    /// [`an_lzw_pump_and_the_whole_decode_agree`]: self::an_lzw_pump_and_the_whole_decode_agree
    /// [`Pump`]: super::Pump
    #[cfg_attr(
        miri,
        ignore = "7 MB of decoded bomb, 50 minutes of interpreter — see the doc comment"
    )]
    #[test]
    fn an_lzw_bomb_costs_the_window_rather_than_its_decode() {
        // The whole table, which is where §7.4.4.2's ratio is highest: 3 838 codes naming
        // entries of 2 to 3 839 bytes.
        let bomb = lzw_bomb(4096);
        let limits = Limits {
            max_stream_len: 1 << 16,
            ..Limits::DEFAULT
        };

        assert_eq!(
            super::decode_reported(b"LZWDecode", &bomb, None, limits).err(),
            Some(super::FilterRefusal::TooLarge {
                limit: limits.max_stream_len
            }),
            "the whole route refuses it"
        );

        let window = 4096usize;
        let mut buffer = vec![0u8; window];
        let mut pump = super::Pump::new(
            super::Pumping::Lzw { early_change: true },
            std::sync::Arc::from(bomb.as_slice()),
        );
        let mut produced = 0usize;
        loop {
            match pump.pump(&mut buffer) {
                super::Pumped::Wrote(wrote) => produced += wrote,
                super::Pumped::Ended(wrote) => {
                    produced += wrote;
                    break;
                }
                super::Pumped::Damaged(_, damage) => panic!("the bomb is whole, not {damage:?}"),
            }
        }
        assert!(
            produced > limits.max_stream_len * 6,
            "{produced} bytes came through a {window}-byte window"
        );
        assert_eq!(buffer.len(), window, "the window never grew");
        assert!(
            produced > bomb.len() * 1000,
            "{} encoded bytes named {produced} decoded ones",
            bomb.len()
        );
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
