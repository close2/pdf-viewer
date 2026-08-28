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
//!
//! **Where a stream *ends* is a separate question from what it decodes to, and this module
//! answers it for two filters it does not decode.** [`encoded_extent`] walks §7.4.6's and
//! §7.4.8's framing — an end-of-block bit pattern and ISO/IEC 10918-1's marker segments — and
//! reconstructs no sample doing it, so the boundary above is not crossed: no codec arrives here,
//! and a walk over lengths and markers is what a parser does anyway. §8.9.7's inline image is
//! the only caller and [`Delimiting`] is the whole list. ADR 0467.

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
    ///
    /// **`FlateDecode` has one way of reaching the end of its input that is not this**, and it
    /// wore this value for as long as the value has existed: a producer that flushed and never
    /// finished wrote every byte of its data and no final block, so what is absent is the
    /// declaration rather than the data. [`ended_on_a_block`] is what tells the two apart; ADR
    /// 0744 has the argument and the three corpus documents that carried the wrong report.
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
    // of it in `calloc` and `free` rather than in this loop. ADR 0180.
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

        // **The row's filter selects a loop; it is not re-tested per byte.** §7.4.4.4 makes
        // the tag a property of the *row*, so testing it per byte asks a question whose
        // answer cannot change — and it also forced every filter to pay for `left` and
        // `up_left`, which types 0 and 2 never read. A cross-reference stream is the case
        // that cares: `/Predictor 12` is type 2 on every row, and type 2 is now a `zip` of
        // two slices with no bounds check and nothing else in it. ADR 0667 has the
        // measurement; the output is byte-identical by construction, because this moves
        // where the tag is examined and changes none of the arithmetic.
        //
        // **An empty row validates no tag, exactly as the per-byte form did**: `data.chunks`
        // can yield a final chunk of one byte, whose row is empty, and the loop that would
        // have rejected an undefined tag never ran. That is a statement about malformed
        // input, so it is preserved rather than tidied.
        match (tag, copy) {
            (_, 0) | (0, _) => {}
            (1, _) => unfilter_sub(current.get_mut(..copy)?, bpp),
            (2, _) => unfilter_up(current.get_mut(..copy)?, previous.get(..copy)?),
            (3, _) => unfilter_average(current.get_mut(..copy)?, previous.get(..copy)?, bpp),
            (4, _) => unfilter_paeth(current.get_mut(..copy)?, previous.get(..copy)?, bpp),
            // An undefined row filter cannot be reversed; guessing would corrupt every
            // subsequent row too, since rows depend on their predecessor.
            _ => return None,
        }

        out.extend_from_slice(current.get(..copy)?);
        std::mem::swap(&mut previous, &mut current);
    }

    Some(Arc::from(out.as_slice()))
}

/// Reverses §7.4.4.4's PNG filter type 1, `Sub`: a byte is a delta from the byte `bpp`
/// earlier in the same row.
///
/// Indices below `bpp` have no left neighbour and the filter adds zero to them, so the loop
/// starts at `bpp` rather than testing for it.
fn unfilter_sub(row: &mut [u8], bpp: usize) {
    for index in bpp..row.len() {
        let left = row.get(index.saturating_sub(bpp)).copied().unwrap_or(0);
        if let Some(value) = row.get_mut(index) {
            *value = value.wrapping_add(left);
        }
    }
}

/// Reverses §7.4.4.4's PNG filter type 2, `Up`: a byte is a delta from the byte above it.
///
/// This is the one a cross-reference stream takes, and it is why the tag was hoisted out of
/// the byte loop: with no left neighbour to fetch and no tag to re-test, the whole filter is
/// a walk of two slices in step.
fn unfilter_up(row: &mut [u8], above: &[u8]) {
    for (value, &up) in row.iter_mut().zip(above) {
        *value = value.wrapping_add(up);
    }
}

/// Reverses §7.4.4.4's PNG filter type 3, `Average`.
///
/// The filter is the floor of the mean of the left and upper neighbours, which
/// `u8::midpoint` computes without an intermediate that could overflow.
fn unfilter_average(row: &mut [u8], above: &[u8], bpp: usize) {
    for index in 0..row.len() {
        let left = if index >= bpp {
            row.get(index.saturating_sub(bpp)).copied().unwrap_or(0)
        } else {
            0
        };
        let up = above.get(index).copied().unwrap_or(0);
        if let Some(value) = row.get_mut(index) {
            *value = value.wrapping_add(u8::midpoint(left, up));
        }
    }
}

/// Reverses §7.4.4.4's PNG filter type 4, `Paeth`.
fn unfilter_paeth(row: &mut [u8], above: &[u8], bpp: usize) {
    for index in 0..row.len() {
        let (left, up_left) = if index >= bpp {
            let earlier = index.saturating_sub(bpp);
            (
                row.get(earlier).copied().unwrap_or(0),
                above.get(earlier).copied().unwrap_or(0),
            )
        } else {
            (0, 0)
        };
        let up = above.get(index).copied().unwrap_or(0);
        if let Some(value) = row.get_mut(index) {
            *value = value.wrapping_add(paeth(left, up, up_left));
        }
    }
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
        b"LZWDecode" | b"LZW" => lzw(data, early_change(parms), limits),
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
        // A stream the producer *flushed* and never finished is not one that stopped short:
        // every byte the encoder produced is here and only the declaration of the end is
        // absent. See [`ended_on_a_block`], which is what decides that rather than guessing it.
        //
        // **Not asked of a decode that produced nothing**, because that answer is [`flate`]'s
        // signal to try the other framing (see [`finish`]) and this must not take a raw deflate
        // stream's fallback away from it. A zlib stream whose whole content is a flush marker
        // decodes to no bytes and is refused as before.
        Stopped::Damaged(Damage::Truncated)
            if !out.is_empty() && ended_on_a_block(data, zlib_header) =>
        {
            finish(&out, None, limits)
        }
        Stopped::Damaged(damage) => finish(&out, Some(damage), limits),
    }
}

/// RFC 1951 section 3.2.4's final empty stored block: `BFINAL` set, `BTYPE` 00, `LEN` 0, `NLEN` 0xffff.
///
/// Forty bits that carry no data, which is what makes them a probe rather than a repair. See
/// [`ended_on_a_block`].
const FINAL_EMPTY_BLOCK: [u8; 5] = [0x01, 0x00, 0x00, 0xff, 0xff];
/// The tail of what `zlib` writes for a `Z_SYNC_FLUSH`: the same stored block with `BFINAL`
/// clear, whose `LEN` and `NLEN` are these four bytes.
///
/// **Four rather than five**, and the fifth byte is the difference between a marker and a
/// coincidence: the flush terminates the block in progress and pads to a byte boundary, so what
/// stands in front of `LEN` is the *last bits of that block* and is `00` only where they
/// happened to be zeros.
///
/// Only ever a filter on *cost* — see [`ended_on_a_block`], which decides on the decoder's
/// answer and never on these bytes.
const FLUSH_MARKER: [u8; 4] = [0x00, 0x00, 0xff, 0xff];

/// Whether a deflate stream that ran out of input had in fact ended on a completed block.
///
/// **A flush is not a truncation, and until this function existed this module called it one.**
/// A producer that calls `Z_SYNC_FLUSH` and then never calls `deflateEnd` writes every byte of
/// its data, terminates the block it was in, and writes no final block and no RFC 1950
/// `ADLER32`. ISO 32000-2 §7.4.1 asks a reader to "invoke the corresponding decoding filter or
/// filters to convert the information back to its original form", and such a decode does: what
/// is missing is a *declaration* that there is no more, and a declaration carries no marks.
/// [`Damage::Truncated`]'s own words — the encoded data ran out before the filter's
/// end-of-data marker — are true of it and are not what the report is for (trap 11).
///
/// # Why the tail bytes are not the test
///
/// "The last bytes are [`FLUSH_MARKER`]" is a *heuristic*: those bytes could be the tail of a
/// Huffman block's bits or the data of a stored one, in which case the stream really did stop
/// mid-block and data really is missing. They are used here only to decide whether the decidable
/// test below is worth its cost, never to decide the answer.
///
/// # The decidable test
///
/// A deflate stream that ended on a *completed* block is one final block short of whole, so
/// feeding a decoder in that state [`FINAL_EMPTY_BLOCK`] must make it report `StreamEnd` and
/// write no further output byte. A decoder stopped inside a block cannot reach `StreamEnd` from
/// those forty bits without emitting something, because anything it could still emit comes from
/// bits that are not there; and if it reaches `StreamEnd` emitting nothing, the only symbols it
/// consumed were an end-of-block, which carries no data. Both directions hold.
///
/// # Why the replay, rather than the decoder that stopped
///
/// Under RFC 1950's framing the decoder wants four bytes of `ADLER32` after the final block, so
/// the probe fed to the *live* decoder returns `Ok` rather than `StreamEnd` and cannot be read.
/// Supplying that checksum means computing an Adler-32 over every byte of every stream this
/// program decodes, on the hot inflate path, to answer a question about 0.03% of documents. So
/// the probe runs on a **raw** decoder over the same input instead, where there is no checksum
/// to satisfy: one extra inflate, on the damaged path, of a stream whose tail carries the
/// marker — and nothing at all for every other stream. The output is thrown away a scratch
/// buffer at a time, so a bomb costs the buffer rather than its decode. ADR 0744 priced the
/// three ways out; this is its second.
///
/// A stream whose header sets `FDICT` is refused rather than replayed: the raw decoder would
/// need the dictionary the zlib framing named, and this returns the answer it can defend.
fn ended_on_a_block(encoded: &[u8], zlib_header: bool) -> bool {
    /// Room for one turn of the replay's output, which is thrown away.
    const SINK: usize = 8192;

    let Some(raw) = raw_deflate(encoded, zlib_header) else {
        return false;
    };
    if !raw.ends_with(&FLUSH_MARKER) {
        return false;
    }

    let mut sink = [0u8; SINK];
    let mut decoder = flate2::Decompress::new(false);
    let mut consumed = 0usize;
    loop {
        let input = raw.get(consumed..).unwrap_or_default();
        let (before_in, before_out) = (decoder.total_in(), decoder.total_out());
        let status = decoder.decompress(input, &mut sink, flate2::FlushDecompress::None);
        consumed = consumed.saturating_add(
            usize::try_from(decoder.total_in().saturating_sub(before_in)).unwrap_or(usize::MAX),
        );
        let progressed = decoder.total_in() != before_in || decoder.total_out() != before_out;
        match turn(&status, progressed) {
            Turn::Again => {}
            // The framed decoder answered `Truncated` over these bytes, so a raw replay that
            // ends or breaks over them is two decoders disagreeing rather than an answer.
            Turn::Whole | Turn::Damaged(Damage::Corrupt) => return false,
            Turn::Damaged(Damage::Truncated) => break,
        }
    }

    let before_out = decoder.total_out();
    let status = decoder.decompress(&FINAL_EMPTY_BLOCK, &mut sink, flate2::FlushDecompress::None);
    matches!(status, Ok(flate2::Status::StreamEnd)) && decoder.total_out() == before_out
}

/// The deflate bits of `encoded`, past white space and past RFC 1950's two-byte header.
///
/// `None` where there is no such thing to hand back: an input that is white space to its end, a
/// zlib header these two bytes do not make, or one whose `FDICT` names a dictionary a raw
/// decoder would not have. See [`ended_on_a_block`], which is the only caller.
fn raw_deflate(encoded: &[u8], zlib_header: bool) -> Option<&[u8]> {
    let start = encoded
        .iter()
        .position(|&byte| !crate::lexer::is_whitespace(byte))?;
    let body = encoded.get(start..)?;
    if !zlib_header {
        return Some(body);
    }
    // RFC 1950 section 2.2: CMF then FLG, with bit 5 of FLG the `FDICT` flag. Asking for the
    // second byte is also what requires there to be a header at all — `get(2..)` alone would
    // hand back an empty slice for a one-byte input rather than refusing it.
    if body.get(1)? & 0x20 != 0 {
        return None;
    }
    body.get(2..)
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

/// One of §7.4's filters as a *resumable* decoder: which filter, and with the parameter that
/// decides what its bits mean.
///
/// **The route is chosen once, by `Document::pumping`, and carried rather than re-derived.**
/// One of §7.8.2's content streams is read more than once — a form, a tiling cell, a glyph
/// description — so a fresh pump is made per read, and a value that says which decoder to build
/// is what keeps the second read from asking the question again and answering it differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// `FlateDecode`.
    Inflate,
    /// `LZWDecode`.
    Lzw {
        /// Table 8's `/EarlyChange`, which decides where the code width grows and therefore
        /// what every bit after that point decodes to. Its default is 1, hence `true`.
        early_change: bool,
    },
    /// `ASCIIHexDecode`.
    AsciiHex,
    /// `ASCII85Decode`.
    Ascii85,
    /// `RunLengthDecode`.
    RunLength,
}

/// Which resumable stage a filter name and its own parameters describe, or `None` where §7.4
/// gives this crate nothing a window can run.
///
/// **All five of §7.4's byte-to-byte filters are here, and what is missing is missing because it
/// is not one.** §7.4.6's `CCITTFaxDecode`, §7.4.7's `JBIG2Decode`, §7.4.8's `DCTDecode` and
/// §7.4.9's `JPXDecode` produce a raster with a width, a depth and a component count rather than
/// a sequence of bytes ([`is_image_codec`] is where that line is drawn), and §7.4.10's `Crypt`
/// is not a transformation this code performs at all — it declares that the stream is encrypted,
/// which §7.6 answers before any filter is reached.
///
/// **The predictor is deliberately not asked about here**, because the two callers want
/// different answers and each is right: `Document::pumping` refuses a predicted stage a window,
/// since §7.4.4.4 reverses each row against its predecessor and that is not a transformation a
/// few thousand bytes at a time can apply; `Document::filtered_extent` does not care, because a
/// predictor runs over a stage's *output* and so moves no byte of its input.
#[must_use]
pub fn stage(filter: &[u8], parms: Option<&Dictionary>) -> Option<Stage> {
    Some(match filter {
        b"FlateDecode" | b"Fl" => Stage::Inflate,
        b"LZWDecode" | b"LZW" => Stage::Lzw {
            early_change: early_change(parms),
        },
        b"ASCIIHexDecode" | b"AHx" => Stage::AsciiHex,
        b"ASCII85Decode" | b"A85" => Stage::Ascii85,
        b"RunLengthDecode" | b"RL" => Stage::RunLength,
        _ => return None,
    })
}

/// Table 8's `/EarlyChange`, ISO 32000-2 §7.4.4.3, read in one place so that the whole decode
/// and the window cannot build different decoders over the same bits.
///
/// Zero postpones a code-length increase as long as possible and one takes it a code early; the
/// default is one, which is the *incorrect* behaviour of a widely-copied encoder and is
/// therefore what almost every file needs.
fn early_change(parms: Option<&Dictionary>) -> bool {
    parms
        .and_then(|parms| parms.get("EarlyChange"))
        .and_then(Object::as_integer)
        .unwrap_or(1)
        != 0
}

/// The chain of stages a [`Pump`] runs, in Table 5's application order — ISO 32000-2 §7.3.8.2:
///
/// > Multiple filters shall be specified in the order in which they are to be applied.
///
/// **A chain rather than a single stage, because §7.4.1's own two cascades are chains.** Its
/// EXAMPLE 2 is `/Filter [/ASCII85Decode /LZWDecode]` and its EXAMPLE 3 is a page's marking
/// instructions under `[/ASCII85Decode /FlateDecode]` — so a window that could run only one
/// filter left the standard's own worked arrangement decoding a bomb whole, which is how a
/// hex-wrapped gibibyte escaped road D at 25 000× the cost of the same bomb unwrapped. ADR 0587,
/// and `doc/todo/14` for the road.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pumping {
    /// Never empty: a chain of no stages is a stream with no filter, which nothing pumps.
    stages: Vec<Stage>,
}

impl Pumping {
    /// The chain, or `None` where there is no stage to run.
    #[must_use]
    pub fn of(stages: Vec<Stage>) -> Option<Self> {
        if stages.is_empty() {
            None
        } else {
            Some(Self { stages })
        }
    }

    /// A chain of one.
    #[must_use]
    pub fn single(stage: Stage) -> Self {
        Self {
            stages: vec![stage],
        }
    }

    /// The stages, in the order they are applied.
    #[must_use]
    pub fn stages(&self) -> &[Stage] {
        &self.stages
    }
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
    /// How many of `data` the first stage has taken.
    at: usize,
    /// Which chain this is, kept so that a second read of the same stream builds the same
    /// decoders without asking the document again.
    pumping: Pumping,
    /// One per stage, in Table 5's application order. Never empty.
    running: Vec<Running>,
    /// Set once end-of-data or damage has been reported, so that a further turn is a no-op
    /// rather than a second report.
    finished: bool,
}

/// One stage of a chain in progress: its decoder, and the bytes it has produced that the next
/// stage has not taken.
///
/// **The link is a fixed buffer and that is the whole trick.** Stage *n*'s output is stage
/// *n+1*'s input, and if that output were a `Vec` grown to the size of the answer then a bomb
/// behind an ASCII armour would cost its gibibyte before the pumping stage ever saw it — which
/// is exactly how one escaped road D (ADR 0586). Here it costs [`LINK`] bytes per stage.
#[derive(Debug)]
struct Running {
    /// The decoder, held across turns — which is what makes this a pump rather than a decode.
    engine: Engine,
    /// What this stage has produced and the next has not taken. The last stage of a chain
    /// writes into the caller's window instead and leaves this empty.
    link: Vec<u8>,
    /// How much of `link` holds bytes.
    filled: usize,
    /// How much of `link` the next stage has taken.
    at: usize,
    /// How this stage finished, once it has.
    done: Option<Ending>,
}

/// The two ways a stage stops having more to give.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ending {
    /// The filter's own end-of-data was read.
    Ended,
    /// The decode stopped short of it.
    Damaged(Damage),
}

/// How many bytes stand between one stage and the next.
///
/// **A scratch buffer rather than a policy**, which is `doc/todo/14`'s argument for the whole
/// road: a window's size is not a number anybody has to defend, and ADR 0362 measured 4 KiB,
/// 64 KiB and 1 MiB windows at the same peak on the same document. What this one also has to be
/// is large enough for [`Inflate`]'s framing retry — see [`Pump::pump`].
const LINK: usize = 8192;

/// What one turn of one stage did with the input it was offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Turned {
    /// How many of the offered input bytes it took.
    took: usize,
    /// How many bytes it wrote into the room it was given.
    wrote: usize,
    /// Where the stage stands now.
    state: Standing,
}

/// Where a stage stands after a turn.
///
/// There is deliberately no *hungry* answer beside [`Standing::More`]: a stage that has run out
/// of the input it was offered and a stage that has more to do with what it holds are the same
/// instruction to the driver — turn me again — and the difference between them is `last`, which
/// the driver already knows because it owns the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Standing {
    /// Turn it again.
    More,
    /// It wants to re-read its input from the beginning under the other framing. Only
    /// [`Inflate`] asks, and only before it has produced a byte.
    Rewind,
    /// The filter's own end-of-data was read.
    Ended,
    /// The decode stopped short of it.
    Damaged(Damage),
}

/// The decoder a [`Running`] stage holds across its turns.
#[derive(Debug)]
enum Engine {
    /// `FlateDecode`, driven through `flate2::Decompress`.
    Inflate(Inflate),
    /// `LZWDecode`. **Boxed**, because §7.4.4.2's table is twelve kilobytes and every other
    /// engine would otherwise carry room for one it will never fill.
    Lzw(Box<Lzw>),
    /// `ASCIIHexDecode`.
    AsciiHex(AsciiHex),
    /// `ASCII85Decode`.
    Ascii85(Ascii85),
    /// `RunLengthDecode`.
    RunLength(RunLength),
}

/// One `FlateDecode` in progress. See [`Pump`].
#[derive(Debug)]
struct Inflate {
    /// The decoder, held across turns.
    decoder: flate2::Decompress,
    /// Whether `decoder` expects zlib's two-byte header.
    zlib_header: bool,
    /// Whether leading white space is still being skipped; [`flate`] skips it before the header
    /// check and a resumable decoder has to skip the same bytes.
    skipping: bool,
    /// How much output this stage has produced, which is half of what says whether a restart
    /// under the other framing is still free.
    produced: u64,
    /// Set where the driver can no longer offer the input again, which is the other half. See
    /// [`Pump::pump`].
    settled: bool,
    /// This stage's whole encoded input, where the driver still holds it.
    ///
    /// **The first stage of a chain has one and no later stage does**, which is the same fact
    /// [`Pump::pump`]'s rewind rests on: the driver keeps the encoded buffer for the whole
    /// pump, and a later stage reads a [`LINK`]-byte window of the stage in front of it. It is
    /// an `Arc` clone rather than a copy, so a pump that never needs it pays a refcount.
    ///
    /// [`ended_on_a_block`] is what it is for, and the cost of not having it is that a
    /// `FlateDecode` *behind* another filter reports a flush as [`Damage::Truncated`] where a
    /// whole-buffer decode of the same stream does not. The population is a chain whose second
    /// or later stage is a deflate **and** whose producer flushed without finishing; the
    /// corpus holds none.
    replayable: Option<Arc<[u8]>>,
}

/// One `ASCIIHexDecode` in progress, ISO 32000-2 §7.4.2. See [`ascii_hex`] for the clause.
///
/// **The whole of the state is one nibble**, which is what makes this the easiest stage in §7.4
/// to window: the filter "shall produce one byte of binary data for each pair of ASCII
/// hexadecimal digits", so a byte of output depends on two bytes of input and on nothing else.
#[derive(Debug, Default)]
struct AsciiHex {
    /// The high nibble of a byte whose second digit has not arrived.
    pending: Option<u8>,
    /// §7.4.2's EOD has been read, or the input ran out; what may be left is the odd digit's
    /// zero.
    ended: bool,
}

/// One `ASCII85Decode` in progress, ISO 32000-2 §7.4.3. See [`ascii85`] for the clause.
#[derive(Debug, Default)]
struct Ascii85 {
    /// The digits of a group that is not yet five long.
    group: [u8; 5],
    /// How many of them there are.
    count: usize,
    /// Whether the optional `<~` introducer has been looked for.
    opened: bool,
    /// The bytes a completed group produced, in as far as the window has not taken them: a
    /// group yields four at once and a window is not obliged to have room for four.
    spill: [u8; 4],
    /// How many of `spill` are the group's.
    spill_len: usize,
    /// How many of those have been handed over.
    spill_at: usize,
    /// The EOD marker has been read, or the input ran out.
    ended: bool,
}

/// Where a `RunLengthDecode` stands between bytes, ISO 32000-2 §7.4.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Run {
    /// Waiting for a run's length byte.
    #[default]
    Length,
    /// Copying this many more bytes literally.
    Literal {
        /// How many are left of the run.
        left: usize,
    },
    /// Waiting for the byte a repeat run repeats.
    Repeating {
        /// How many times it will be repeated.
        left: usize,
    },
    /// Repeating that byte, that many more times.
    Repeat {
        /// The byte.
        byte: u8,
        /// How many are left of the run.
        left: usize,
    },
}

/// One `RunLengthDecode` in progress. See [`run_length`] for the clause.
#[derive(Debug, Default)]
struct RunLength {
    /// Where it stands between bytes.
    run: Run,
}

impl Engine {
    /// The decoder `stage` names, at the start of its input.
    ///
    /// `FlateDecode`'s white-space skip is [`flate`]'s, kept exactly, and the zlib-then-raw
    /// fallback is taken later by [`Inflate::turn`]: a stream missing its two-byte header is
    /// common in the wild, and a decoder that has produced nothing yet can be restarted under
    /// the other framing for nothing.
    ///
    /// `replayable` is this stage's whole encoded input where the driver still holds it, which
    /// is the first stage of a chain and no other. See [`Inflate::replayable`].
    fn new(stage: Stage, replayable: Option<Arc<[u8]>>) -> Self {
        match stage {
            Stage::Inflate => Self::Inflate(Inflate {
                decoder: flate2::Decompress::new(true),
                zlib_header: true,
                skipping: true,
                produced: 0,
                settled: false,
                replayable,
            }),
            Stage::Lzw { early_change } => Self::Lzw(Box::new(Lzw::new(early_change))),
            Stage::AsciiHex => Self::AsciiHex(AsciiHex::default()),
            Stage::Ascii85 => Self::Ascii85(Ascii85::default()),
            Stage::RunLength => Self::RunLength(RunLength::default()),
        }
    }

    /// One turn of whichever decoder this is, over the input it has not taken.
    ///
    /// `last` says that no further input will arrive, which is what tells a decoder that has run
    /// out of bytes apart from one that is merely waiting for the stage in front of it.
    fn turn(&mut self, input: &[u8], last: bool, out: &mut [u8]) -> Turned {
        match self {
            Self::Inflate(inflate) => inflate.turn(input, last, out),
            Self::Lzw(lzw) => lzw.turn(input, last, out),
            Self::AsciiHex(hex) => hex.turn(input, last, out),
            Self::Ascii85(ascii85) => ascii85.turn(input, last, out),
            Self::RunLength(runs) => runs.turn(input, last, out),
        }
    }

    /// Whether this stage may still ask for its input from the beginning again.
    fn may_rewind(&self) -> bool {
        match self {
            Self::Inflate(inflate) => {
                inflate.produced == 0 && inflate.zlib_header && !inflate.settled
            }
            _ => false,
        }
    }

    /// Gives up the right to ask, because the driver can no longer offer the bytes.
    fn settle(&mut self) {
        if let Self::Inflate(inflate) = self {
            inflate.settled = true;
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

/// How a filter states where its own encoded data ends, and therefore how to find it.
///
/// **Three shapes rather than one, and the difference is what a caller pays.** A decoder's own
/// end-of-data is a structure in a compressed bit stream and can only be reached by decoding;
/// a textual marker is a byte pair the encoding's alphabet cannot otherwise contain; and the
/// remaining three are *walks* over the encoded bytes' own framing, which read the structure
/// without reconstructing a single sample.
///
/// **The `/L`-less inline image is the only thing that asks** — Table 5 makes `/Length`
/// required of every stream *object* — so this enum is built by
/// [`Document::filtered_extent`](crate::Document::filtered_extent) from the image's dictionary
/// and by nothing else.
#[expect(
    clippy::doc_markdown,
    reason = "the erratum's sentence is quoted verbatim, and a quotation with backticks added \
              to please a lint is no longer a quotation"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delimiting {
    /// Run the decoder and ask how much input it took: `FlateDecode` and `LZWDecode`, whose
    /// end-of-data is RFC 1951's final block and §7.4.4.2's `EarlyChange`-dependent 257.
    ///
    /// One stage rather than a chain, because Table 5 applies the filters "in the order in
    /// which they are to be applied" and the bytes in the file are the *first* stage's input.
    Decoded(Stage),
    /// A byte sequence no correctly encoded byte of the filter's own alphabet can be.
    ///
    /// §7.4.2, of `ASCIIHexDecode`:
    ///
    /// > A GREATER-THAN SIGN (3Eh) indicates EOD (End Of Data).
    ///
    /// §7.4.3 gives `ASCII85Decode` the two-character sequence (7Eh)(3Eh), over an alphabet of
    /// `!` through `u` and `z` in which (7Eh) cannot otherwise occur. Errata Collection 3 makes
    /// that a rule rather than an inference, Issue #293 adding to the clause: "If the
    /// ASCII85Decode filter encounters the character ~ in its input, the next character shall be >
    /// and the filter will reach EOD. Any other characters shall cause an error."
    Marker(&'static [u8]),
    /// §7.4.5's run headers, walked. Each header says how far the next one stands, so the walk
    /// visits one byte in every two to a hundred and twenty-nine and decodes nothing at all.
    RunLength,
    /// ISO/IEC 10918-1's marker segments, walked to the EOI marker (§7.4.8).
    Jpeg,
    /// §7.4.6 Table 11's end-of-block bit pattern: this many consecutive end-of-line codes.
    EndOfBlock {
        /// Two for Group 4's EOFB, six for Group 3's RTC — "appropriate for the K parameter",
        /// which is the whole of what Table 11 says about which.
        end_of_lines: u32,
    },
}

/// How many of `data`'s bytes the filter's own end-of-data marker delimits.
///
/// ISO 32000-2 §7.3.8.2 is what makes this question answerable at all:
///
/// > In addition, most filters are defined so that the data shall be self-limiting; that is,
/// > they use an encoding scheme in which an explicit end-of-data (EOD) marker delimits the
/// > extent of the data.
///
/// and Errata Collection 3's Issue #319 adds the sentence that says where the answer *stops*,
/// which is the difference between an off-by-a-marker and a correct extent: "The 'encoded data'
/// of a stream encompasses all enveloping markers of the encoding, e.g. end-of-data markers, if
/// the encoding scheme uses them." So every arm below counts the marker in.
///
/// Nothing in a *file* needs it, because Table 5 makes `/Length` required and every stream
/// object states one. §8.9.7's inline image is the exception the clause exists for: it is
/// written into a content stream with no `/Length` before PDF 2.0, so where its encoded data
/// ends is the filter's answer to give.
/// [`Document::filtered_extent`](crate::Document::filtered_extent) is the caller, and
/// `pdf_model::inline_image` is what asks it.
///
/// `ceiling` bounds the *output* of the one arm that produces any, and is ignored by the four
/// that do not.
#[must_use]
pub fn encoded_extent(delimiting: Delimiting, data: &[u8], ceiling: usize) -> EncodedExtent {
    match delimiting {
        Delimiting::Decoded(pumping) => decoded_extent(pumping, data, ceiling),
        Delimiting::Marker(marker) => marker_extent(data, marker),
        Delimiting::RunLength => run_length_extent(data),
        Delimiting::Jpeg => jpeg_extent(data),
        Delimiting::EndOfBlock { end_of_lines } => end_of_block_extent(data, end_of_lines),
    }
}

/// [`Delimiting::Decoded`]: the extent a decoder's consumed input states.
///
/// `ceiling` bounds the output, which is thrown away a window at a time and never held: it is
/// the same number [`decode_reported`] spends on an allocation and here it buys time instead, so
/// that a decompression bomb whose marker is a gibibyte away costs neither.
///
/// **The cost is one decode, and the caller pays for a second one afterwards.** This runs the
/// filter to find where it stops and keeps nothing; the bytes are then decoded again by
/// whoever wanted them. That is deliberate — the alternative is a decoded buffer of unbounded
/// size held across a scan whose whole purpose is to avoid one — and it replaces a linear
/// search over the same bytes, so the population it runs on is one where a walk over those
/// bytes was the cost already.
fn decoded_extent(stage: Stage, data: &[u8], ceiling: usize) -> EncodedExtent {
    // Room for one turn's output, which is thrown away. §7.4.4.2 caps an `LZWDecode` entry at
    // 4096 bytes and `Lzw::turn` hands a longer sequence over in pieces, so any size works and
    // this one is a page of them.
    let mut sink = [0u8; 8192];
    // **No replay here, and the asymmetry with [`Pump::new`] is the question this function
    // asks.** It looks for where a filter's own end-of-data marker stands, and a flush marker
    // is not one: [`EncodedExtent::Short`]'s "these bytes ran out before one" stays true of a
    // stream the producer flushed and never finished, whatever [`ended_on_a_block`] would say
    // about its damage.
    let mut engine = Engine::new(stage, None);
    let mut consumed = 0usize;
    let mut produced = 0usize;
    loop {
        // The whole buffer is offered at once, so there is never more input to come and the
        // stage's own end-of-data is the only thing that can stop it short.
        let turned = engine.turn(data.get(consumed..).unwrap_or_default(), true, &mut sink);
        consumed = consumed.saturating_add(turned.took);
        produced = produced.saturating_add(turned.wrote);
        match turned.state {
            Standing::More => {
                if produced > ceiling {
                    return EncodedExtent::Unknown;
                }
            }
            // The other framing gets its turn over the same bytes, exactly as [`flate`] gives
            // it one over the whole buffer.
            Standing::Rewind => consumed = 0,
            Standing::Ended => return EncodedExtent::Ends(consumed.min(data.len())),
            // The input ending before the marker is what [`Damage::Truncated`] is, and it is
            // the one damage a longer buffer could still answer.
            Standing::Damaged(Damage::Truncated) => return EncodedExtent::Short,
            Standing::Damaged(_) => return EncodedExtent::Unknown,
        }
    }
}

/// [`Delimiting::Marker`]: the first occurrence of a byte sequence the alphabet cannot contain.
///
/// A marker absent from these bytes is [`EncodedExtent::Short`] rather than
/// [`EncodedExtent::Unknown`], and the distinction is the reason the two are separate answers:
/// neither §7.4.2's nor §7.4.3's encoding can *contain* its own marker, so bytes without one are
/// bytes that stop before the end rather than bytes that carry no end.
fn marker_extent(data: &[u8], marker: &[u8]) -> EncodedExtent {
    match data
        .windows(marker.len())
        .position(|window| window == marker)
    {
        Some(at) => EncodedExtent::Ends(at.saturating_add(marker.len())),
        None => EncodedExtent::Short,
    }
}

/// [`Delimiting::RunLength`]: §7.4.5's run headers, walked.
///
/// > The encoded data shall be a sequence of runs, where each run shall consist of a length byte
/// > followed by 1 to 128 bytes of data. If the length byte is in the range 0 to 127, the
/// > following length + 1 (1 to 128) bytes shall be copied literally during decompression. If
/// > length is in the range 129 to 255, the following single byte shall be copied 257 length (2
/// > to 128) times during decompression. A length value of 128 shall denote EOD.
///
/// **Every byte of the encoded data is either a header or is counted by one**, so where the data
/// ends needs no decoder: the walk reads a header, steps over what it governs, and stops on the
/// 128. That is the whole of it, and it is why this filter costs a walk over one byte in every
/// two to a hundred and twenty-nine rather than a decode.
///
/// [`run_length`] is the decoder over the same rule, and the two must not drift: this counts the
/// input that one would consume, up to and including the EOD byte.
fn run_length_extent(data: &[u8]) -> EncodedExtent {
    let mut at = 0usize;
    loop {
        // Running out of input is the same statement here as in [`run_length`], where it is
        // `Damage::Truncated`: §7.4.5 gives this filter no invalid byte, so the only way the
        // walk can fail is the data ending before its EOD.
        let Some(&length) = data.get(at) else {
            return EncodedExtent::Short;
        };
        at = at.saturating_add(1);
        match length {
            128 => return EncodedExtent::Ends(at),
            // The header and the literal bytes it governs.
            0..=127 => at = at.saturating_add(usize::from(length).saturating_add(1)),
            // The header and the one byte it repeats.
            _ => at = at.saturating_add(1),
        }
        if at > data.len() {
            return EncodedExtent::Short;
        }
    }
}

/// [`Delimiting::Jpeg`]: ISO/IEC 10918-1's marker segments, walked to EOI.
///
/// §7.4.8 states the framing by reference rather than in its own words — the data is "encoded in
/// the JPEG baseline format in accordance with ISO/IEC 10918 (all parts)" — so the end-of-data
/// §7.3.8.2 promises is that standard's EOI marker, `FFD9`, and this walk is 10918-1's own
/// structure rather than a search for those two bytes.
///
/// **A search would be wrong, and the reason is worth stating**: `FFD9` occurs freely inside a
/// marker segment's payload, and an `APPn` segment is allowed to carry an entire second JPEG —
/// which is what a camera's thumbnail is. The walk steps over each segment by the length it
/// states, so a thumbnail's own EOI cannot end the outer image. Inside entropy-coded data the
/// converse holds by construction: 10918-1 stuffs a zero byte after every `FF` a coder emits, so
/// the only `FF` pairs there are `FF00` and the restart markers `FFD0`–`FFD7`, and both are
/// stepped over.
///
/// No sample is reconstructed and no table is read. What the walk needs is the length field of
/// each segment and the rule for where entropy-coded data ends, which is [`entropy_end`].
fn jpeg_extent(data: &[u8]) -> EncodedExtent {
    // 10918-1's interchange format begins with SOI. Bytes that do not are not this filter's
    // input as the clause describes it, and answering `Unknown` leaves the caller its own
    // fallback rather than inventing an extent from a structure that is not there.
    let mut at = match next_marker(data, 0) {
        Framing::Marker { code: 0xd8, after } => after,
        Framing::Marker { .. } | Framing::NotAMarker => return EncodedExtent::Unknown,
        Framing::RanOut => return EncodedExtent::Short,
    };
    loop {
        let (code, after) = match next_marker(data, at) {
            Framing::Marker { code, after } => (code, after),
            Framing::RanOut => return EncodedExtent::Short,
            Framing::NotAMarker => return EncodedExtent::Unknown,
        };
        at = after;
        match code {
            // EOI. §7.3.8.2's erratum counts the marker itself in.
            0xd9 => return EncodedExtent::Ends(at),
            // The markers 10918-1 gives no length: a second SOI, TEM, and the restart markers.
            0xd8 | 0x01 | 0xd0..=0xd7 => {}
            // `FF00` is a stuffed byte and stands only inside entropy-coded data, which the walk
            // never reads a marker from; meeting one here is a framing this walk cannot follow.
            0x00 => return EncodedExtent::Unknown,
            _ => {
                let Some(bytes) = data.get(at..at.saturating_add(2)) else {
                    return EncodedExtent::Short;
                };
                let length = usize::from(u16::from_be_bytes([bytes[0], bytes[1]]));
                // The length counts its own two bytes, so anything below two is not a length.
                if length < 2 {
                    return EncodedExtent::Unknown;
                }
                at = at.saturating_add(length);
                if at > data.len() {
                    return EncodedExtent::Short;
                }
                // SOS is the one marker segment followed by data rather than by another marker.
                if code == 0xda {
                    match entropy_end(data, at) {
                        Some(end) => at = end,
                        None => return EncodedExtent::Short,
                    }
                }
            }
        }
    }
}

/// What stands at `at` where [`jpeg_extent`] expects a marker.
///
/// 10918-1 lets any number of `FF` fill bytes precede a marker, so a marker is a run of `FF`
/// followed by the one byte that identifies it.
enum Framing {
    /// The marker's identifying byte, and the offset just past it.
    Marker {
        /// The byte after the `FF` run.
        code: u8,
        /// Where the marker's own bytes end.
        after: usize,
    },
    /// The bytes ran out inside the marker, or before one.
    RanOut,
    /// Something that is not `FF` stands where a marker must.
    NotAMarker,
}

/// Reads the marker at `at`, past any fill bytes. See [`Framing`].
fn next_marker(data: &[u8], at: usize) -> Framing {
    let mut cursor = at;
    match data.get(cursor) {
        Some(0xff) => {}
        Some(_) => return Framing::NotAMarker,
        None => return Framing::RanOut,
    }
    while data.get(cursor) == Some(&0xff) {
        cursor = cursor.saturating_add(1);
    }
    match data.get(cursor) {
        Some(&code) => Framing::Marker {
            code,
            after: cursor.saturating_add(1),
        },
        None => Framing::RanOut,
    }
}

/// Where the entropy-coded data starting at `at` ends: the offset of the `FF` that opens the
/// next marker.
///
/// 10918-1's byte stuffing is what makes this decidable without decoding: a coder that emits
/// `FF` writes `00` after it, so inside entropy-coded data the only two-byte `FF` sequences are
/// `FF00` and the restart markers `FFD0`–`FFD7`. Anything else is the next marker, and the
/// offset returned is the `FF` rather than the byte after it, so [`next_marker`] reads it.
fn entropy_end(data: &[u8], at: usize) -> Option<usize> {
    let mut cursor = at;
    while let Some(&byte) = data.get(cursor) {
        if byte != 0xff {
            cursor = cursor.saturating_add(1);
            continue;
        }
        let mut code = cursor.saturating_add(1);
        while data.get(code) == Some(&0xff) {
            code = code.saturating_add(1);
        }
        match data.get(code)? {
            // A stuffed zero, or a restart: the entropy-coded data continues past it.
            0x00 | 0xd0..=0xd7 => cursor = code.saturating_add(1),
            _ => return Some(cursor),
        }
    }
    None
}

/// [`Delimiting::EndOfBlock`]: §7.4.6's end-of-block bit pattern, found without a fax decoder.
///
/// Table 11's `/EndOfBlock` is the whole of the permission to look for one:
///
/// > A flag indicating whether the filter shall expect the encoded data to be terminated by an
/// > end-of-block pattern, overriding the Rows parameter. If false , the filter shall stop when
/// > it has decoded the number of lines indicated by Rows or when its data has been exhausted,
/// > whichever occurs first. The end-of-block pattern shall be the CCITT end-of-facsimile-block
/// > (EOFB) or return-to-control (RTC) appropriate for the K parameter. Default value: true .
///
/// So where the flag is true — its default, and therefore the common case — the data *shall* be
/// terminated by a pattern, and the pattern is built out of ITU-T T.4's end-of-line code
/// `000000000001`. Both forms are a run of those: EOFB is two and RTC is six.
///
/// **What makes finding it a reading rather than a guess** is T.4's own construction of that
/// code — it is chosen so that no sequence of valid codewords can contain it, which is why a fax
/// receiver can resynchronise on it. Eleven zero bits followed by a one therefore cannot stand
/// inside encoded scan lines, and the run of them this looks for cannot either. The converse
/// gives the walk its shape: only the *leading* and *trailing* zero runs of a byte can reach
/// eleven, because a run bounded by one-bits inside a byte is at most six long, so one pass over
/// the bytes finds every candidate.
///
/// Fill is T.4's, and costs nothing here: a variable run of zero bits may precede any end-of-line
/// code, and those zeros are absorbed by the run this counts. In Group 3's mixed mode each
/// end-of-line carries a tag bit after it, which is why two patterns count as consecutive when
/// the second's zeros begin at most one bit past the first's one-bit.
///
/// The byte count comes from §7.4.6's own sentence about what a filter does when it gets there:
///
/// > When a filter reaches EOD, it shall always skip to the next byte boundary following the
/// > encoded data.
fn end_of_block_extent(data: &[u8], end_of_lines: u32) -> EncodedExtent {
    /// T.4's end-of-line code is eleven zero bits and a one.
    const ZEROS: u64 = 11;

    // Zero bits standing immediately before the current byte, and where the previous
    // end-of-line's one-bit ended, both counted in bits from the start of the data.
    let mut zeros = 0u64;
    let mut previous_end: Option<u64> = None;
    let mut run = 0u32;
    let mut base = 0u64;

    for &byte in data {
        // A whole byte of zeros carries the run forward and can end no pattern of its own.
        if byte == 0 {
            zeros = zeros.saturating_add(8);
            base = base.saturating_add(8);
            continue;
        }
        let leading = u64::from(byte.leading_zeros());
        let total = zeros.saturating_add(leading);
        if total >= ZEROS {
            // Just past the one-bit that closes the code, and where its zeros began.
            let end = base.saturating_add(leading).saturating_add(1);
            let start = end.saturating_sub(1).saturating_sub(total);
            run = match previous_end {
                Some(previous) if start <= previous.saturating_add(1) => run.saturating_add(1),
                _ => 1,
            };
            previous_end = Some(end);
            if run >= end_of_lines {
                let bytes = end.saturating_add(7) / 8;
                return match usize::try_from(bytes) {
                    Ok(bytes) => EncodedExtent::Ends(bytes.min(data.len())),
                    Err(_) => EncodedExtent::Unknown,
                };
            }
        }
        zeros = u64::from(byte.trailing_zeros());
        base = base.saturating_add(8);
    }
    // The pattern Table 11 requires is not in these bytes. Through a window that is more bytes
    // to ask for; over a whole content stream it is a producer that wrote none, and the caller's
    // own fallback is what answers then.
    EncodedExtent::Short
}

impl Pump {
    /// A pump over `data`, decoding it as [`decode_reported`] would run the same chain.
    ///
    /// `FlateDecode`'s white-space skip and zlib-then-raw fallback are [`flate`]'s, kept
    /// exactly: a stream missing its two-byte header is common in the wild, and a decoder that
    /// has produced nothing yet can be restarted under the other framing for nothing.
    #[must_use]
    pub fn new(pumping: Pumping, data: Arc<[u8]>) -> Self {
        let count = pumping.stages().len();
        let running = pumping
            .stages()
            .iter()
            .enumerate()
            .map(|(index, stage)| Running {
                // Only the first stage's input is the encoded buffer, which this pump holds for
                // its whole life; a later stage's is a link window. See [`Inflate::replayable`].
                engine: Engine::new(*stage, (index == 0).then(|| Arc::clone(&data))),
                // The last stage writes into the caller's window, so it needs no link of its
                // own — and a chain of one, which is every stream this crate pumped before ADR
                // 0587, therefore allocates nothing here at all.
                link: if index.saturating_add(1) == count {
                    Vec::new()
                } else {
                    vec![0u8; LINK]
                },
                filled: 0,
                at: 0,
                done: None,
            })
            .collect();
        Self {
            data,
            at: 0,
            pumping,
            running,
            finished: false,
        }
    }

    /// Which chain this pump runs, so that a second read of the same stream builds the same
    /// decoders without asking the document again.
    #[must_use]
    pub fn pumping(&self) -> Pumping {
        self.pumping.clone()
    }

    /// Writes the next bytes of the decoded stream into `out`.
    ///
    /// `out` must not be empty: a decoder given no room makes no progress, and for
    /// `FlateDecode` no progress is how [`turn`] recognises a truncated input.
    ///
    /// **One pass over the chain per call, front to back**, so that bytes a stage produces reach
    /// the stage after it within the same call rather than a call later. A pass that moves bytes
    /// without any of them reaching the end of the chain answers [`Pumped::Wrote`] with zero,
    /// which `pdf_model::content::reader` already documents as "progress of a kind the caller
    /// must keep asking through" — an inflate that takes input without emitting has always been
    /// able to answer that way.
    ///
    /// **The one thing a chain has that a single stage did not is a rewind.** [`Inflate`] asks
    /// to re-read its input under raw framing when zlib's produced nothing, and for the first
    /// stage that is free because the whole encoded buffer is still there. For a later stage the
    /// input is a [`LINK`]-byte link, so the driver holds its bytes back from compaction until
    /// the stage reading them has produced something — and where that cannot be done, because
    /// the stage consumed a whole link's worth without emitting a byte, it is told
    /// ([`Engine::settle`]) that the offer is withdrawn rather than being allowed to ask for
    /// bytes that are gone. A zlib framing that is wrong fails at its two-byte header, so the
    /// withdrawal needs a stage that consumes eight kilobytes of a *valid* zlib stream while
    /// emitting nothing and then fails; that stream decodes to nothing under raw framing either.
    pub fn pump(&mut self, out: &mut [u8]) -> Pumped {
        if self.finished || out.is_empty() {
            return Pumped::Wrote(0);
        }
        let count = self.running.len();
        let mut progressed = false;
        let mut wrote = 0usize;

        // **Every link is compacted before any stage runs**, because a link is compacted on the
        // authority of the stage that *reads* it and filled by the stage that writes it — and a
        // pass that did both in one visit left the writer looking at a full buffer it was about
        // to be given room in, which stalled the whole chain at eight kilobytes a turn.
        for index in 1..count {
            let (before, from) = self.running.split_at_mut(index);
            let (Some(up), Some(running)) = (before.last_mut(), from.first_mut()) else {
                break;
            };
            if up.at == 0 {
                continue;
            }
            // The bytes may only be dropped once the stage reading them can no longer ask for
            // them again — and where that stage has taken a whole link's worth without emitting
            // a byte, the offer is withdrawn rather than the buffer being allowed to grow.
            if running.engine.may_rewind() && up.filled < up.link.len() {
                continue;
            }
            running.engine.settle();
            up.link.copy_within(up.at..up.filled, 0);
            up.filled = up.filled.saturating_sub(up.at);
            up.at = 0;
        }

        for index in 0..count {
            let (before, from) = self.running.split_at_mut(index);
            let mut upstream = before.last_mut();
            let Some(running) = from.first_mut() else {
                break;
            };
            if running.done.is_some() {
                continue;
            }

            let (input, last) = match upstream.as_deref() {
                Some(up) => (
                    up.link.get(up.at..up.filled).unwrap_or_default(),
                    up.done.is_some(),
                ),
                // The first stage's input is the whole encoded buffer, and there is never any
                // more of it than the file holds.
                None => (self.data.get(self.at..).unwrap_or_default(), true),
            };

            let turned = if index.saturating_add(1) == count {
                running.engine.turn(input, last, out)
            } else {
                let Running {
                    engine,
                    link,
                    filled,
                    ..
                } = running;
                let room = link.get_mut(*filled..).unwrap_or_default();
                if room.is_empty() {
                    // The stage after this one has to drain the link before there is anywhere
                    // to put more.
                    continue;
                }
                engine.turn(input, last, room)
            };

            if turned.took > 0 || turned.wrote > 0 {
                progressed = true;
            }
            match upstream.as_deref_mut() {
                Some(up) => up.at = up.at.saturating_add(turned.took),
                None => self.at = self.at.saturating_add(turned.took),
            }
            if index.saturating_add(1) == count {
                wrote = turned.wrote;
            } else {
                running.filled = running.filled.saturating_add(turned.wrote);
            }

            match turned.state {
                Standing::More => {}
                Standing::Rewind => {
                    // A stage may ask at most twice — zlib once, raw once — and `may_rewind`
                    // is false for ever afterwards, so this cannot loop.
                    progressed = true;
                    match upstream {
                        Some(up) => up.at = 0,
                        None => self.at = 0,
                    }
                }
                Standing::Ended => running.done = Some(Ending::Ended),
                Standing::Damaged(damage) => running.done = Some(Ending::Damaged(damage)),
            }
        }

        match self.running.last().and_then(|last| last.done) {
            Some(Ending::Ended) => {
                self.finished = true;
                Pumped::Ended(wrote)
            }
            Some(Ending::Damaged(damage)) => {
                self.finished = true;
                Pumped::Damaged(wrote, damage)
            }
            None if progressed => Pumped::Wrote(wrote),
            // **Unreachable, and loud rather than silent because the alternative is a hang.**
            // A pass moves nothing only if every stage is waiting for input that will not come,
            // and a stage whose source is closed is given `last`, on which every engine here
            // ends or reports damage rather than asking again. Saying so costs a branch and
            // keeps a decoder defect from becoming an unkillable loop in the reader's refill.
            None => {
                self.finished = true;
                Pumped::Damaged(wrote, Damage::Truncated)
            }
        }
    }
}

impl Inflate {
    /// One turn of the inflate. See [`Engine::turn`].
    fn turn(&mut self, input: &[u8], last: bool, out: &mut [u8]) -> Turned {
        let mut took = 0usize;
        if self.skipping {
            match input
                .iter()
                .position(|&byte| !crate::lexer::is_whitespace(byte))
            {
                Some(at) => {
                    took = at;
                    self.skipping = false;
                }
                // [`flate`] refuses a buffer that is white space to its end, and refuses it as
                // corrupt rather than as damaged, so a chain that produced nothing but white
                // space reaches the same answer through `part.kept == 0`.
                None => {
                    return Turned {
                        took: input.len(),
                        wrote: 0,
                        state: if last {
                            Standing::Damaged(Damage::Corrupt)
                        } else {
                            Standing::More
                        },
                    };
                }
            }
        }

        let body = input.get(took..).unwrap_or_default();
        let (before_in, before_out) = (self.decoder.total_in(), self.decoder.total_out());
        let status = self
            .decoder
            .decompress(body, out, flate2::FlushDecompress::None);
        let read = usize::try_from(self.decoder.total_in().saturating_sub(before_in))
            .unwrap_or(usize::MAX);
        let wrote = usize::try_from(self.decoder.total_out().saturating_sub(before_out))
            .unwrap_or(usize::MAX);
        took = took.saturating_add(read);
        self.produced = self.produced.saturating_add(wrote as u64);

        let state = match turn(&status, read > 0 || wrote > 0) {
            Turn::Again => Standing::More,
            Turn::Whole => Standing::Ended,
            // No progress with input still to come is a stage waiting rather than a stream
            // that stopped: [`turn`] cannot tell those apart, because over a whole buffer
            // there is no such thing as input still to come.
            Turn::Damaged(Damage::Truncated) if !last => Standing::More,
            // Nothing has come out under this framing, so the other one gets its turn —
            // [`flate`]'s fallback, taken here at the point the first framing fails rather
            // than after a whole decode. A restart is free exactly while the stage has
            // produced nothing, because there is nothing to un-hand-over.
            Turn::Damaged(damage) => {
                if self.produced == 0 && self.zlib_header && !self.settled {
                    self.zlib_header = false;
                    self.decoder = flate2::Decompress::new(false);
                    self.skipping = true;
                    return Turned {
                        took: 0,
                        wrote: 0,
                        state: Standing::Rewind,
                    };
                }
                // A producer that flushed and never finished wrote every byte of its data, so
                // the stream *ends* here rather than stopping short. Asked after the fallback
                // above and never in front of it, for the reason [`inflate`] gives: a decode
                // that produced nothing is the other framing's cue, not this question's.
                if damage == Damage::Truncated
                    && self
                        .replayable
                        .as_deref()
                        .is_some_and(|encoded| ended_on_a_block(encoded, self.zlib_header))
                {
                    Standing::Ended
                } else {
                    Standing::Damaged(damage)
                }
            }
        };
        Turned { took, wrote, state }
    }
}

impl Lzw {
    /// One turn of the LZW decode. See [`Engine::turn`].
    ///
    /// **A code names a sequence and a window has room for however much it has room for**, so
    /// the sequence the last [`Self::step`] produced is handed over in pieces across as many
    /// turns as it takes. That is the only thing this route has that [`lzw`]'s has not: an
    /// entry can be 4096 bytes and a window is not obliged to be larger than one.
    fn turn(&mut self, input: &[u8], last: bool, out: &mut [u8]) -> Turned {
        let mut wrote = 0usize;
        let mut stopped: Option<Standing> = None;
        let state = loop {
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
                break Standing::More;
            }
            if let Some(standing) = stopped {
                break standing;
            }
            if wrote >= out.len() {
                break Standing::More;
            }
            match self.step(input) {
                Step::Again => {}
                // The input this stage was offered ran out mid-code; `held` and `bits` keep
                // the part of it already read, so the next turn resumes inside the same code.
                Step::Damaged(Damage::Truncated) if !last => break Standing::More,
                Step::Ended => stopped = Some(Standing::Ended),
                Step::Damaged(damage) => stopped = Some(Standing::Damaged(damage)),
            }
        };
        // `at` indexes the bytes this turn was offered, and the driver takes them from the
        // source; the next turn starts at nothing again.
        let took = self.at;
        self.at = 0;
        Turned { took, wrote, state }
    }
}

impl AsciiHex {
    /// One turn of the hexadecimal decode. See [`Engine::turn`] and [`ascii_hex`].
    fn turn(&mut self, input: &[u8], last: bool, out: &mut [u8]) -> Turned {
        let mut took = 0usize;
        let mut wrote = 0usize;
        while !self.ended && wrote < out.len() {
            let Some(&byte) = input.get(took) else {
                // No EOD marker in what there is. If there will be no more, §7.4.2's odd-digit
                // rule is all that is left to apply — which is what [`ascii_hex`] does when it
                // falls off the end of its buffer, and it calls that whole rather than damaged.
                if last {
                    self.ended = true;
                }
                break;
            };
            took = took.saturating_add(1);
            if byte == b'>' {
                self.ended = true;
                break;
            }
            let value = match byte {
                b'0'..=b'9' => byte.saturating_sub(b'0'),
                b'a'..=b'f' => byte.saturating_sub(b'a').saturating_add(10),
                b'A'..=b'F' => byte.saturating_sub(b'A').saturating_add(10),
                // [`ascii_hex`]'s deliberate departure from "[a]ny other characters shall cause
                // an error", stated there and pinned by its own test.
                _ => continue,
            };
            match self.pending.take() {
                Some(high) => {
                    if let Some(slot) = out.get_mut(wrote) {
                        *slot = high.saturating_mul(16).saturating_add(value);
                        wrote = wrote.saturating_add(1);
                    }
                }
                None => self.pending = Some(value),
            }
        }
        if !self.ended {
            return Turned {
                took,
                wrote,
                state: Standing::More,
            };
        }
        // "If the filter encounters the EOD marker after reading an odd number of hexadecimal
        // digits, it shall behave as if a 0 (zero) followed the last digit."
        if let Some(high) = self.pending {
            let Some(slot) = out.get_mut(wrote) else {
                // No room for the last byte; it is the next turn's, and the end with it.
                return Turned {
                    took,
                    wrote,
                    state: Standing::More,
                };
            };
            *slot = high.saturating_mul(16);
            wrote = wrote.saturating_add(1);
            self.pending = None;
        }
        Turned {
            took,
            wrote,
            state: Standing::Ended,
        }
    }
}

impl Ascii85 {
    /// One turn of the base-85 decode. See [`Engine::turn`] and [`ascii85`].
    fn turn(&mut self, input: &[u8], last: bool, out: &mut [u8]) -> Turned {
        let mut took = 0usize;
        let mut wrote = 0usize;
        if !self.opened {
            // The optional `<~` introducer, which needs two bytes to recognise and is only ever
            // at the very beginning.
            if input.len() < 2 && !last {
                return Turned {
                    took: 0,
                    wrote: 0,
                    state: Standing::More,
                };
            }
            self.opened = true;
            if input.starts_with(b"<~") {
                took = 2;
            }
        }
        loop {
            while self.spill_at < self.spill_len && wrote < out.len() {
                let (Some(slot), Some(&byte)) = (out.get_mut(wrote), self.spill.get(self.spill_at))
                else {
                    break;
                };
                *slot = byte;
                wrote = wrote.saturating_add(1);
                self.spill_at = self.spill_at.saturating_add(1);
            }
            if self.spill_at < self.spill_len {
                return Turned {
                    took,
                    wrote,
                    state: Standing::More,
                };
            }
            self.spill_len = 0;
            self.spill_at = 0;
            if self.ended {
                return Turned {
                    took,
                    wrote,
                    state: Standing::Ended,
                };
            }
            if wrote >= out.len() {
                return Turned {
                    took,
                    wrote,
                    state: Standing::More,
                };
            }
            let Some(&byte) = input.get(took) else {
                if !last {
                    return Turned {
                        took,
                        wrote,
                        state: Standing::More,
                    };
                }
                // The bytes ran out before the EOD marker. [`ascii85`] flushes the partial
                // group and calls that whole, because §7.4.3 makes a partial final group the
                // encoding rather than damage.
                self.close();
                continue;
            };
            took = took.saturating_add(1);
            if crate::lexer::is_whitespace(byte) {
                continue;
            }
            if byte == b'~' {
                self.close();
                continue;
            }
            // "if all five bytes are 0, they shall be represented by the character with code
            // 122 (z)", and only between groups.
            if byte == b'z' && self.count == 0 {
                self.spill = [0; 4];
                self.spill_len = 4;
                self.spill_at = 0;
                continue;
            }
            if !(b'!'..=b'u').contains(&byte) {
                // §7.4.3: any other character "shall cause an error". A window has already given
                // its bytes to a lexer, so the error is met where it is and the groups in front
                // of it stand — which is ADR 0343's rule for a content stream, and a content
                // stream is the only thing a window is ever run over. [`ascii85`] refuses the
                // whole stream instead, for the population *it* serves, and says why.
                return Turned {
                    took,
                    wrote,
                    state: Standing::Damaged(Damage::Corrupt),
                };
            }
            if let Some(slot) = self.group.get_mut(self.count) {
                *slot = byte.saturating_sub(b'!');
            }
            self.count = self.count.saturating_add(1);
            if self.count == 5 {
                self.expand(5);
                self.count = 0;
            }
        }
    }

    /// Ends the stream, flushing §7.4.3's partial final group.
    fn close(&mut self) {
        self.ended = true;
        if self.count > 1 {
            // "the last, partial group of 4 shall be used to produce a last, partial group of 5
            // output characters" — padded with the maximum digit and cut back to `count - 1`.
            for slot in self.group.iter_mut().skip(self.count) {
                *slot = 84;
            }
            self.expand(self.count);
        }
        self.count = 0;
    }

    /// Turns the group into the `count - 1` bytes it names.
    fn expand(&mut self, count: usize) {
        let mut value = 0u32;
        for digit in self.group {
            value = value.saturating_mul(85).saturating_add(u32::from(digit));
        }
        self.spill = value.to_be_bytes();
        self.spill_len = count.saturating_sub(1).min(4);
        self.spill_at = 0;
    }
}

impl RunLength {
    /// One turn of the run-length decode. See [`Engine::turn`] and [`run_length`].
    fn turn(&mut self, input: &[u8], last: bool, out: &mut [u8]) -> Turned {
        let mut took = 0usize;
        let mut wrote = 0usize;
        // "The encoded data shall be a sequence of runs, where each run shall consist of a
        // length byte followed by 1 to 128 bytes of data", so the input ending anywhere other
        // than on a `128` is the stream stopping short — [`run_length`]'s [`Damage::Truncated`].
        let short = |took: usize, wrote: usize| Turned {
            took,
            wrote,
            state: if last {
                Standing::Damaged(Damage::Truncated)
            } else {
                Standing::More
            },
        };
        loop {
            if wrote >= out.len() {
                return Turned {
                    took,
                    wrote,
                    state: Standing::More,
                };
            }
            match self.run {
                Run::Length => {
                    let Some(&length) = input.get(took) else {
                        return short(took, wrote);
                    };
                    took = took.saturating_add(1);
                    self.run = match length {
                        // "A length value of 128 shall denote EOD."
                        128 => {
                            return Turned {
                                took,
                                wrote,
                                state: Standing::Ended,
                            };
                        }
                        0..=127 => Run::Literal {
                            left: usize::from(length).saturating_add(1),
                        },
                        _ => Run::Repeating {
                            left: 257usize.saturating_sub(usize::from(length)),
                        },
                    };
                }
                Run::Literal { left } => {
                    let available = input.len().saturating_sub(took);
                    if available == 0 {
                        return short(took, wrote);
                    }
                    let take = left.min(available).min(out.len().saturating_sub(wrote));
                    let (Some(slot), Some(source)) = (
                        out.get_mut(wrote..wrote.saturating_add(take)),
                        input.get(took..took.saturating_add(take)),
                    ) else {
                        return short(took, wrote);
                    };
                    slot.copy_from_slice(source);
                    took = took.saturating_add(take);
                    wrote = wrote.saturating_add(take);
                    self.run = if take >= left {
                        Run::Length
                    } else {
                        Run::Literal {
                            left: left.saturating_sub(take),
                        }
                    };
                }
                Run::Repeating { left } => {
                    let Some(&byte) = input.get(took) else {
                        return short(took, wrote);
                    };
                    took = took.saturating_add(1);
                    self.run = Run::Repeat { byte, left };
                }
                Run::Repeat { byte, left } => {
                    let take = left.min(out.len().saturating_sub(wrote));
                    let Some(slot) = out.get_mut(wrote..wrote.saturating_add(take)) else {
                        return short(took, wrote);
                    };
                    slot.fill(byte);
                    wrote = wrote.saturating_add(take);
                    self.run = if take >= left {
                        Run::Length
                    } else {
                        Run::Repeat {
                            byte,
                            left: left.saturating_sub(take),
                        }
                    };
                }
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

/// Decodes `ASCII85Decode`, ISO 32000-2 §7.4.3.
///
/// > Any other characters, and any character sequences that represent impossible combinations
/// > in the ASCII base-85 encoding, shall cause an error.
///
/// **The whole stream is refused, and [`Ascii85`] — the same clause through a window — keeps
/// the groups in front of the character instead. That is not a drift between two readings; it
/// is ADR 0343's own distinction arriving by route rather than by consumer**, and the
/// seven-hundred-and-fourteenth session had it the other way round for an afternoon until a
/// fuzzed corpus document said so. `PDFBOX-3148-2-fuzzed.pdf` states its **cross-reference
/// stream** as `/Filter [/ASCII85Decode]` with a byte outside `!`..=`u` eight bytes in: refusing
/// it sends [`crate::Parser`] to its header scan and the file's page is found, while handing
/// back the eight bytes as a decode makes them a cross-reference *section* with almost every
/// entry missing and the document loses its only page in silence. A prefix of a table is not a
/// shorter table — the question `doc/traps/parsers-and-streams.md` puts as *what a prefix of the
/// thing is* — whereas a prefix of §7.8.2's "sequence of instructions" is a shorter sequence of
/// the same kind.
///
/// So the buffered route, which every consumer but one takes — cross-reference streams, font
/// programs, image samples, ICC profiles — refuses; and the windowed route, which only ever runs
/// over a content stream, reports [`Damage::Corrupt`] over the bytes it has already handed to a
/// lexer, which is what ADR 0343 requires of exactly that population. ADR 0587.
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
                    super::Pumping::single(super::Stage::Lzw { early_change: true }),
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
            super::Pumping::single(super::Stage::Lzw {
                early_change: false,
            }),
            std::sync::Arc::from(packed.as_slice()),
        );
        assert_eq!(drain(&mut late, 16).0.as_slice(), &*whole);

        let mut early = super::Pump::new(
            super::Pumping::single(super::Stage::Lzw { early_change: true }),
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
                super::Pumping::single(super::Stage::Lzw { early_change: true }),
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
            super::Pumping::single(super::Stage::Lzw { early_change: true }),
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

    /// §7.4.2's encoding, so that a chain test can build what a producer would have written.
    fn hex_encoded(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        for &byte in data {
            out.extend_from_slice(format!("{byte:02X}").as_bytes());
            // "All white-space characters shall be ignored", and a producer wrapping long lines
            // is why that sentence is there — so the encoder writes some.
            if out.len() % 65 == 0 {
                out.push(b'\n');
            }
        }
        out.push(b'>');
        out
    }

    /// §7.4.3's encoding, including its `z` special case.
    fn base85_encoded(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        for group in data.chunks(4) {
            let mut value = 0u32;
            for index in 0..4 {
                value = value.wrapping_mul(256) + u32::from(group.get(index).copied().unwrap_or(0));
            }
            if group.len() == 4 && value == 0 {
                out.push(b'z');
                continue;
            }
            let mut digits = [0u8; 5];
            for slot in digits.iter_mut().rev() {
                *slot = u8::try_from(value % 85).expect("a base-85 digit") + b'!';
                value /= 85;
            }
            out.extend_from_slice(digits.get(..group.len() + 1).expect("five digits"));
        }
        out.extend_from_slice(b"~>");
        out
    }

    /// §7.4.5's encoding: repeat runs where the data repeats, literal runs elsewhere.
    fn run_length_encoded(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut at = 0usize;
        while at < data.len() {
            let byte = data.get(at).copied().expect("in range");
            let mut run = 1usize;
            while run < 128 && data.get(at + run).copied() == Some(byte) {
                run += 1;
            }
            if run >= 2 {
                out.push(u8::try_from(257 - run).expect("129 to 255"));
                out.push(byte);
                at += run;
            } else {
                let start = at;
                let mut length = 0usize;
                while at < data.len() && length < 128 {
                    if length > 0
                        && data.get(at).copied() == data.get(at + 1).copied()
                        && data.get(at).copied() == data.get(at + 2).copied()
                    {
                        break;
                    }
                    at += 1;
                    length += 1;
                }
                out.push(u8::try_from(length - 1).expect("0 to 127"));
                out.extend_from_slice(data.get(start..at).expect("in range"));
            }
        }
        out.push(128);
        out
    }

    /// A zlib stream, which is what `FlateDecode` names.
    fn deflated(data: &[u8]) -> Vec<u8> {
        use std::io::Write as _;
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
        encoder.write_all(data).expect("in-memory write");
        encoder.finish().expect("finish")
    }

    /// A zlib stream whose deflate data is one RFC 1951 section 3.2.4 stored block, so the payload
    /// stands in it byte for byte and a cut can be placed at a known offset.
    fn deflated_stored(data: &[u8]) -> Vec<u8> {
        use std::io::Write as _;
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::none());
        encoder.write_all(data).expect("in-memory write");
        encoder.finish().expect("finish")
    }

    /// A zlib stream its producer flushed and never finished, which is ADR 0744's witness.
    ///
    /// `Z_SYNC_FLUSH` terminates the block in progress and writes RFC 1951 section 3.2.4's empty
    /// stored block, so every byte of `data` is encoded; what never arrives is the final block
    /// and RFC 1950's `ADLER32`, because `deflateEnd` is never called.
    fn flushed(data: &[u8], level: flate2::Compression) -> Vec<u8> {
        let mut compressor = flate2::Compress::new(level, true);
        let mut out: Vec<u8> = Vec::with_capacity(data.len().saturating_mul(2).max(4096));
        loop {
            let taken = usize::try_from(compressor.total_in()).expect("fits");
            let before = (taken, out.len());
            compressor
                .compress_vec(
                    data.get(taken..).unwrap_or_default(),
                    &mut out,
                    flate2::FlushCompress::Sync,
                )
                .expect("an in-memory compress");
            if out.len() == out.capacity() {
                out.reserve(out.capacity());
                continue;
            }
            if (
                usize::try_from(compressor.total_in()).expect("fits"),
                out.len(),
            ) == before
            {
                return out;
            }
        }
    }

    /// Runs a whole `/Filter` chain the way `Document::chain_over` does, stage by stage.
    fn whole_chain(filters: &[&[u8]], data: &[u8], limits: Limits) -> super::Decoded {
        let mut decoded = super::Decoded::whole(data);
        for filter in filters {
            let stage = super::decode_reported(filter, &decoded.data, None, limits)
                .expect("the whole route decodes it");
            decoded = super::Decoded {
                data: stage.data,
                damage: decoded.damage.or(stage.damage),
            };
        }
        decoded
    }

    /// Where two byte sequences first differ, so that a failure names a place rather than
    /// printing two megabytes of `Debug`.
    fn same(got: &[u8], want: &[u8], context: &str) {
        if got == want {
            return;
        }
        let at = got
            .iter()
            .zip(want.iter())
            .position(|(left, right)| left != right);
        panic!(
            "{context}: {} bytes against {}, first difference at {at:?} ({:?} against {:?})",
            got.len(),
            want.len(),
            at.and_then(|at| got.get(at.saturating_sub(4)..at + 4).map(<[u8]>::to_vec)),
            at.and_then(|at| want.get(at.saturating_sub(4)..at + 4).map(<[u8]>::to_vec)),
        );
    }

    /// The pump over a chain, from `Document::pumping`'s own vocabulary.
    fn chain_pump(stages: &[super::Stage], data: &[u8]) -> super::Pump {
        super::Pump::new(
            super::Pumping::of(stages.to_vec()).expect("a chain of at least one stage"),
            std::sync::Arc::from(data),
        )
    }

    /// One `/Filter` chain to test: its names, the stages they become, and encoded bytes.
    type Arrangement = (Vec<&'static [u8]>, Vec<super::Stage>, Vec<u8>);

    /// **Every chain a window runs hands back what the whole decode hands back.**
    ///
    /// This is what the whole of ADR 0587 rests on: five filters, each with a resumable decoder
    /// beside its buffered one, composed in the arrangements §7.4.1's own examples use. A window
    /// of **one byte** is in the list deliberately — a `LZWDecode` entry may be 4096 bytes and a
    /// base-85 group four, so the smallest window is what exercises handing a sequence over in
    /// pieces across turns, and the eight-kilobyte link between two stages is what exercises the
    /// opposite.
    #[cfg_attr(
        miri,
        ignore = "zlib-rs's deallocation, not this tree's — see the note above"
    )]
    #[test]
    fn every_pumpable_chain_agrees_with_the_whole_decode() {
        use super::Stage::{Ascii85, AsciiHex, Inflate, Lzw, RunLength};

        // Long enough to cross a link buffer several times, with a run for `RunLengthDecode`
        // and a zero group for §7.4.3's `z`.
        let mut payload = Vec::new();
        for index in 0..40_000u32 {
            match index % 7 {
                0 => payload.extend_from_slice(&[0, 0, 0, 0]),
                1 => payload.extend_from_slice(b"BT /F1 12 Tf (chain) Tj ET "),
                2 => payload.resize(payload.len() + 300, b'#'),
                _ => payload.extend_from_slice(&index.to_be_bytes()),
            }
        }

        let lzw = |data: &[u8]| -> Vec<u8> {
            // §7.4.4.2 without a compressor: every byte as its own literal code, which is a
            // valid encoding and the one a decoder has the least help from.
            let mut codes = vec![256u16];
            codes.extend(data.iter().map(|&byte| u16::from(byte)));
            codes.push(257);
            pack_lzw(&codes, true)
        };

        let arrangements: Vec<Arrangement> = vec![
            (
                vec![b"ASCIIHexDecode"],
                vec![AsciiHex],
                hex_encoded(&payload),
            ),
            (
                vec![b"ASCII85Decode"],
                vec![Ascii85],
                base85_encoded(&payload),
            ),
            (
                vec![b"RunLengthDecode"],
                vec![RunLength],
                run_length_encoded(&payload),
            ),
            (vec![b"FlateDecode"], vec![Inflate], deflated(&payload)),
            // §7.4.1 EXAMPLE 3: a page's marking instructions, deflated and then base-85 armoured.
            (
                vec![b"ASCII85Decode", b"FlateDecode"],
                vec![Ascii85, Inflate],
                base85_encoded(&deflated(&payload)),
            ),
            // ADR 0437's witness and ADR 0586's, which is the arrangement that escaped road D.
            (
                vec![b"ASCIIHexDecode", b"FlateDecode"],
                vec![AsciiHex, Inflate],
                hex_encoded(&deflated(&payload)),
            ),
            // §7.4.1 EXAMPLE 2, verbatim: `/Filter [/ASCII85Decode /LZWDecode]`.
            (
                vec![b"ASCII85Decode", b"LZWDecode"],
                vec![Ascii85, Lzw { early_change: true }],
                base85_encoded(&lzw(&payload)),
            ),
            // A compressing stage in front of an armouring one, which no sane producer writes
            // and a hostile one may: the expansion is then *inside* the chain.
            (
                vec![b"FlateDecode", b"ASCIIHexDecode"],
                vec![Inflate, AsciiHex],
                deflated(&hex_encoded(&payload)),
            ),
            // Three stages, so that a link feeds a link.
            (
                vec![b"ASCII85Decode", b"FlateDecode", b"RunLengthDecode"],
                vec![Ascii85, Inflate, RunLength],
                base85_encoded(&deflated(&run_length_encoded(&payload))),
            ),
        ];

        for (filters, stages, encoded) in arrangements {
            let name = filters
                .iter()
                .map(|filter| String::from_utf8_lossy(filter).into_owned())
                .collect::<Vec<_>>()
                .join(" ");
            let whole = whole_chain(&filters, &encoded, Limits::DEFAULT);
            assert_eq!(&*whole.data, payload.as_slice(), "[{name}] whole");
            assert_eq!(whole.damage, None, "[{name}] whole");
            for window in [1usize, 3, 64, 4096, 100_000] {
                let mut pump = chain_pump(&stages, &encoded);
                let (pumped, ended) = drain(&mut pump, window);
                same(
                    pumped.as_slice(),
                    &whole.data,
                    &format!("[{name}] through a {window}-byte window"),
                );
                assert!(
                    matches!(ended, super::Pumped::Ended(_)),
                    "[{name}] through a {window}-byte window ended {ended:?}"
                );
            }
        }
    }

    /// **A bomb behind an ASCII armour costs the window rather than its decode**, which is the
    /// hole ADR 0586 measured at about 25 000× and declined to close in the cache.
    ///
    /// The whole route refuses it, having inflated to the bound first; the pump reads every byte
    /// of it in a buffer that never grows. `Document::pumping` is what sends a chain here, and
    /// `nested_content_window.rs` is where that routing is asserted from a document.
    #[cfg_attr(
        miri,
        ignore = "a bomb's worth of inflation under the interpreter; ADR 0463's reason"
    )]
    #[test]
    fn a_bomb_behind_an_ascii_armour_costs_the_window() {
        let bomb = hex_encoded(&deflated(&vec![0u8; 8 << 20]));
        let limits = Limits {
            max_stream_len: 1 << 16,
            ..Limits::DEFAULT
        };

        assert_eq!(
            whole_route_refusal(&[b"ASCIIHexDecode", b"FlateDecode"], &bomb, limits),
            Some(super::FilterRefusal::TooLarge {
                limit: limits.max_stream_len
            }),
            "the whole route refuses it"
        );

        let window = 4096usize;
        let mut buffer = vec![0u8; window];
        let mut pump = chain_pump(&[super::Stage::AsciiHex, super::Stage::Inflate], &bomb);
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
        assert_eq!(produced, 8 << 20, "every byte of it came through");
        assert_eq!(buffer.len(), window, "the window never grew");
        assert!(
            produced > bomb.len() * 500,
            "{} encoded bytes named {produced} decoded ones",
            bomb.len()
        );
    }

    /// The first stage of a chain that refuses, or `None` where the chain decodes.
    fn whole_route_refusal(
        filters: &[&[u8]],
        data: &[u8],
        limits: Limits,
    ) -> Option<super::FilterRefusal> {
        let mut decoded: std::sync::Arc<[u8]> = std::sync::Arc::from(data);
        for filter in filters {
            match super::decode_reported(filter, &decoded, None, limits) {
                Ok(stage) => decoded = stage.data,
                Err(why) => return Some(why),
            }
        }
        None
    }

    /// §7.4.3's error refuses the buffered route and keeps the window's groups, on purpose.
    ///
    /// **The two answers are ADR 0343's own distinction rather than a drift**, and this is what
    /// pins both halves. The buffered route serves every consumer whose prefix is not a shorter
    /// thing of the same kind — a cross-reference stream above all, which is how
    /// `PDFBOX-3148-2-fuzzed.pdf` loses its only page when this filter hands back eight bytes
    /// instead of refusing. The window is only ever run over §7.8.2's "sequence of instructions",
    /// where the groups in front of the character are the producer's own and are drawn.
    /// [`super::ascii85`] carries the argument; ADR 0587 is the round that had it backwards for
    /// an afternoon.
    #[test]
    fn a_base85_error_keeps_the_groups_before_it() {
        let mut broken = base85_encoded(b"the groups before it");
        // A character outside `!`..`u` that is neither white space, `z`, nor the marker, placed
        // on a group boundary so that what stands before it is a whole number of groups.
        broken.splice(10..10, *b"\x01");

        assert_eq!(
            super::decode_reported(b"ASCII85Decode", &broken, None, Limits::DEFAULT).err(),
            Some(super::FilterRefusal::Corrupt),
            "the buffered route refuses, for the consumers whose prefix is not one"
        );

        for window in [1usize, 4, 4096] {
            let mut pump = chain_pump(&[super::Stage::Ascii85], &broken);
            let (pumped, ended) = drain(&mut pump, window);
            // Ten base-85 characters are two whole groups, so eight bytes stand and the group
            // the character interrupted does not.
            same(
                pumped.as_slice(),
                b"the grou",
                &format!("through {window} bytes"),
            );
            assert!(
                matches!(ended, super::Pumped::Damaged(_, super::Damage::Corrupt)),
                "through {window} bytes: {ended:?}"
            );
        }
    }

    /// Damage inside a chain is met where it is, and the prefix in front of it stands.
    ///
    /// A deflate stream cut short behind a hex armour: the hex stage decodes everything it was
    /// given, the inflate stops where the encoder's bytes stop, and both routes report
    /// [`super::Damage::Truncated`] over the same prefix. ADR 0343's rule, through two stages.
    #[cfg_attr(
        miri,
        ignore = "zlib-rs's deallocation, not this tree's — see the note above"
    )]
    #[test]
    fn a_chain_reports_the_damage_the_whole_decode_does() {
        let payload: Vec<u8> = (0..20_000u32).flat_map(u32::to_be_bytes).collect();
        let mut deflate = deflated(&payload);
        deflate.truncate(deflate.len() / 2);
        let encoded = hex_encoded(&deflate);

        let whole = super::decode_reported(
            b"FlateDecode",
            &super::decode_reported(b"ASCIIHexDecode", &encoded, None, Limits::DEFAULT)
                .expect("hex decodes")
                .data,
            None,
            Limits::DEFAULT,
        )
        .expect("a prefix, not a refusal");
        assert_eq!(whole.damage, Some(super::Damage::Truncated));
        assert!(!whole.data.is_empty(), "a prefix came out");

        for window in [1usize, 512, 65_536] {
            let mut pump = chain_pump(&[super::Stage::AsciiHex, super::Stage::Inflate], &encoded);
            let (pumped, ended) = drain(&mut pump, window);
            same(
                pumped.as_slice(),
                &whole.data,
                &format!("through {window} bytes"),
            );
            assert!(
                matches!(ended, super::Pumped::Damaged(_, super::Damage::Truncated)),
                "through {window} bytes: {ended:?}"
            );
        }
    }

    /// A producer that flushed and never finished wrote a whole stream, and both routes say so.
    ///
    /// ISO 32000-2 §7.4.1 asks a reader to "invoke the corresponding decoding filter or filters
    /// to convert the information back to its original form", and this decode reaches it: every
    /// byte the encoder was given comes back. What never arrives is RFC 1951's final block, and
    /// a declaration that there is no more carries no marks. ADR 0744.
    ///
    /// Both compression levels are here because they are two different last blocks: `best`
    /// leaves a Huffman block to be terminated by the flush, `none` a stored one.
    #[cfg_attr(
        miri,
        ignore = "zlib-rs's deallocation, not this tree's — see the note above"
    )]
    #[test]
    fn a_stream_flushed_and_never_finished_is_whole() {
        let payload: Vec<u8> = (0..9_000u32).flat_map(u32::to_be_bytes).collect();
        for level in [flate2::Compression::best(), flate2::Compression::none()] {
            let encoded = flushed(&payload, level);
            assert!(
                encoded.ends_with(&super::FLUSH_MARKER),
                "level {level:?}: the encoder did not write the flush marker this test is about"
            );

            let whole = super::decode_reported(b"FlateDecode", &encoded, None, Limits::DEFAULT)
                .expect("a decode, not a refusal");
            assert_eq!(
                whole.damage, None,
                "level {level:?}: the buffered route calls a flush a truncation"
            );
            same(&whole.data, &payload, &format!("level {level:?}, buffered"));

            for window in [1usize, 512, 65_536] {
                let mut pump = chain_pump(&[super::Stage::Inflate], &encoded);
                let (pumped, ended) = drain(&mut pump, window);
                same(
                    pumped.as_slice(),
                    &payload,
                    &format!("level {level:?}, through {window} bytes"),
                );
                assert!(
                    matches!(ended, super::Pumped::Ended(_)),
                    "level {level:?}, through {window} bytes: {ended:?}"
                );
            }
        }
    }

    /// The tail bytes are not the test, and a stream that ends in them mid-block still reports.
    ///
    /// **The calibration [`super::ended_on_a_block`] exists for.** A stored block carries its
    /// data verbatim, so a payload holding `00 00 00 ff ff` and cut immediately after it is a
    /// stream whose last five bytes are the flush marker and whose data really is missing.
    /// Anything deciding on those five bytes calls this whole; the probe hands the decoder RFC
    /// 1951's final empty block, the decoder reads it as five more literal bytes of the stored
    /// block it is inside, and output moves — which is the answer.
    #[cfg_attr(
        miri,
        ignore = "zlib-rs's deallocation, not this tree's — see the note above"
    )]
    #[test]
    fn a_stream_cut_inside_a_block_that_ends_in_the_marker_is_still_truncated() {
        let mut payload: Vec<u8> = (0..200u32)
            .map(|byte| u8::try_from(byte % 251).expect("under 251"))
            .collect();
        payload.splice(100..100, super::FLUSH_MARKER);
        let encoded = deflated_stored(&payload);
        // Two bytes of RFC 1950 header, five of RFC 1951 section 3.2.4's stored-block header, then the
        // payload byte for byte — so cutting here ends the input on the marker's last byte.
        let cut = 2 + 5 + 104;
        let short = encoded
            .get(..cut)
            .expect("the stored block is longer than this")
            .to_vec();
        assert!(
            short.ends_with(&super::FLUSH_MARKER),
            "the cut did not land on the marker this test is about"
        );

        let whole = super::decode_reported(b"FlateDecode", &short, None, Limits::DEFAULT)
            .expect("a prefix, not a refusal");
        assert_eq!(
            whole.damage,
            Some(super::Damage::Truncated),
            "the buffered route believed five bytes over a decoder"
        );
        same(
            &whole.data,
            payload.get(..104).expect("in range"),
            "buffered",
        );

        for window in [1usize, 512, 65_536] {
            let mut pump = chain_pump(&[super::Stage::Inflate], &short);
            let (_, ended) = drain(&mut pump, window);
            assert!(
                matches!(ended, super::Pumped::Damaged(_, super::Damage::Truncated)),
                "through {window} bytes: {ended:?}"
            );
        }
    }

    /// A raw deflate stream behind an armouring stage still takes [`super::flate`]'s fallback.
    ///
    /// **This is the one thing a chain has that a single stage did not**: the retry re-reads the
    /// stage's input, which for a later stage lives in a link that the driver has to hold back
    /// from compaction. See [`super::Pump::pump`].
    #[cfg_attr(
        miri,
        ignore = "zlib-rs's deallocation, not this tree's — see the note above"
    )]
    #[test]
    fn a_raw_deflate_stream_behind_an_armour_still_falls_back() {
        use std::io::Write as _;
        let payload: Vec<u8> = (0..30_000u32).flat_map(u32::to_le_bytes).collect();
        let mut deflating =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::fast());
        deflating.write_all(&payload).expect("in-memory write");
        let armoured = base85_encoded(&deflating.finish().expect("finish"));

        let whole = whole_chain(
            &[b"ASCII85Decode", b"FlateDecode"],
            &armoured,
            Limits::DEFAULT,
        );
        assert_eq!(&*whole.data, payload.as_slice());

        for window in [1usize, 4096] {
            let mut pump = chain_pump(&[super::Stage::Ascii85, super::Stage::Inflate], &armoured);
            let (pumped, ended) = drain(&mut pump, window);
            assert_eq!(pumped.as_slice(), payload.as_slice(), "through {window}");
            assert!(matches!(ended, super::Pumped::Ended(_)), "{ended:?}");
        }
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
