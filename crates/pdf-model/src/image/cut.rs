//! A baseline `DCTDecode` frame with no restart interval, decoded in bands beside each other at the
//! MCU rows one entropy pass finds (ADR 1481).
//!
//! # Why a frame without restart markers can still be cut
//!
//! ISO/IEC 10918-1 (paraphrased, since only ISO 32000-2 is quoted in this tree) carries two things
//! from one MCU of a sequential Huffman scan to the next, and only two: the position in the bit
//! stream, and each component's DC prediction, against which section F.2.1.3.1 codes the next
//! block's DC coefficient as a difference. A restart marker resets both, which is why
//! [`super::restart`] can cut a frame at one. A frame that has none can be cut anyway once both
//! are known at a row: one pass that decodes the Huffman symbols and nothing else — no
//! dequantisation, no inverse transform, no colour — finds every MCU row's first bit and the
//! prediction each component carries into it. A band is then a codestream of its own: the frame's
//! header segments with its number of lines, the scan header, its rows' bits realigned to a byte,
//! the first DC difference of each component re-coded against a prediction of zero by section
//! F.1.2.1's categories (which is what a decoder starting the band predicts), padded with one-bits
//! as section F.1.2.3 pads a segment, and ended by `EOI`. Every block after those first ones has
//! the prediction it had in the whole frame, so its bits are copied unchanged.
//!
//! Each band decodes one MCU row above and one below what it keeps, for [`super::restart`]'s
//! reason — the chroma upsampling reads neighbouring rows — and is held to the sampling shapes
//! that module admits, whose upsamplers that argument was read against.
//!
//! # When it pays, and so when it is asked
//!
//! The pass is serial and costs most of what the codec's own Huffman decoding costs; what it buys
//! is the rest of the codec — the Huffman decoding again, the inverse transforms, the upsampling
//! and the colour conversion — divided across the pool. That is a gain where the pool would
//! otherwise idle, and a loss where the page's other images are already decoding beside this one
//! (ADR 1321): built and measured without a signal, it took 2–3 ms off a page of one photograph and
//! added 8–10 ms to one of eight (ADR 1457 section 3). So it is asked only where the caller says
//! the frame is decoded alone ([`super::InFlight::Alone`]).
//!
//! # What is admitted, and what is declined
//!
//! Admitted: what [`super::restart`] admits, with no `DRI` (or one stating no interval), entropy
//! data in which every `FF` is a stuffed `FF 00`, and the scan ended by `EOI` or by the end of the
//! data. Declined: a pass that meets a code no table holds, a DC category past eleven or an AC
//! category past ten (section F.1.2's bounds for eight-bit samples), a run past the 63rd
//! coefficient, or the data's end before the last MCU; and a band whose first DC value has no code
//! in its table. A declined frame is decoded whole, exactly as before, by the one decoder it always
//! was.

use super::FirstScan;
use super::restart::{self, Band, Geometry};

/// One Huffman table of a `DHT` segment, in the forms section F.2.2.3 decodes by and Annex C
/// encodes by.
struct Huffman {
    /// For each nine-bit prefix: the length of the code it begins with in the high byte and the
    /// code's symbol in the low, or 0 where that code is longer than nine bits.
    fast: Box<[u16; 512]>,
    /// For each length `l` from 1 to 16: the largest code of that length, or −1 where there is
    /// none (section F.2.2.3's `MAXCODE`).
    largest: [i32; 17],
    /// For each length: the index into [`Self::symbols`] of its first code, less that code
    /// (`VALPTR − MINCODE`).
    offset: [i32; 17],
    /// The symbols in code order (`HUFFVAL`).
    symbols: Vec<u8>,
    /// Each symbol's code and length, where the table holds it (Annex C's `EHUFCO`, `EHUFSI`).
    codes: Vec<Option<(u16, u8)>>,
}

impl Huffman {
    /// The table a `DHT` segment states with `counts[l − 1]` codes of length `l` and `symbols` in
    /// code order, or `None` where the counts do not describe a prefix code.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`code` is checked below `1 << length` before each step and `length` is at most \
                  16, so it stays under 2^17; `index` counts at most the 4 080 codes sixteen \
                  counts of a byte can state; `9 - length` is taken only where `length <= 9`"
    )]
    fn new(counts: [u8; 16], symbols: &[u8]) -> Option<Self> {
        let mut fast = Box::new([0u16; 512]);
        let mut largest = [-1i32; 17];
        let mut offset = [0i32; 17];
        let mut codes = vec![None; 256];
        let mut code = 0u32;
        let mut index = 0usize;
        for (length, count) in (1u8..=16).zip(counts) {
            let length_u32 = u32::from(length);
            offset[usize::from(length)] = i32::try_from(index).ok()? - i32::try_from(code).ok()?;
            for _ in 0..count {
                let symbol = *symbols.get(index)?;
                if code >= 1 << length_u32 {
                    return None;
                }
                *codes.get_mut(usize::from(symbol))? = Some((u16::try_from(code).ok()?, length));
                if length <= 9 {
                    let spread = 9 - length_u32;
                    let first = usize::try_from(code << spread).ok()?;
                    let entry = u16::from(length) << 8 | u16::from(symbol);
                    fast.get_mut(first..first + (1 << spread))?.fill(entry);
                }
                code += 1;
                index += 1;
            }
            if count > 0 {
                largest[usize::from(length)] = i32::try_from(code).ok()? - 1;
            }
            code <<= 1;
        }
        (index == symbols.len()).then_some(Self {
            fast,
            largest,
            offset,
            symbols: symbols.to_vec(),
            codes,
        })
    }

    /// The next symbol in `bits`, or `None` where no code of this table begins there.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`length` runs from 10 to 16, so `16 - length` is a shift below 7; `code` is a \
                  prefix of at most 16 bits and `offset` lies between −2^17 and 2^12, so their \
                  sum is an `i32` whatever the table"
    )]
    fn decode(&self, bits: &mut Bits<'_>) -> Option<u8> {
        bits.refill();
        let prefix = bits.peek(16);
        let entry = self.fast[usize::try_from(prefix >> 7).ok()?];
        if entry != 0 {
            bits.consume(u32::from(entry >> 8));
            return Some(entry.to_le_bytes()[0]);
        }
        for length in 10u32..=16 {
            let code = i32::try_from(prefix >> (16 - length)).ok()?;
            let at = usize::try_from(length).ok()?;
            if code <= self.largest[at] {
                bits.consume(length);
                return self
                    .symbols
                    .get(usize::try_from(code + self.offset[at]).ok()?)
                    .copied();
            }
        }
        None
    }
}

/// A big-endian reader over entropy-coded data with its stuffing removed, which can say where it
/// is to the bit.
struct Bits<'a> {
    bytes: &'a [u8],
    /// The next byte to load; past the end, zeros are loaded and counted.
    next: usize,
    /// Loaded bits, the first in the top bit.
    held: u64,
    /// How many of [`Self::held`]'s bits are loaded.
    count: u32,
}

impl<'a> Bits<'a> {
    /// A reader at bit `at` of `bytes`.
    fn at(bytes: &'a [u8], at: u64) -> Self {
        let mut bits = Self {
            bytes,
            next: usize::try_from(at / 8).unwrap_or(usize::MAX),
            held: 0,
            count: 0,
        };
        bits.refill();
        bits.consume(u32::try_from(at % 8).unwrap_or(0));
        bits
    }

    /// Loads whole bytes until more than 56 bits are held.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the loop runs only while `count <= 56`, so `56 - count` is a shift of at most \
                  56 and `count + 8` at most 64"
    )]
    fn refill(&mut self) {
        while self.count <= 56 {
            let byte = self.bytes.get(self.next).copied().unwrap_or(0);
            self.next = self.next.saturating_add(1);
            self.held |= u64::from(byte) << (56 - self.count);
            self.count += 8;
        }
    }

    /// The next `n` bits, `n` at most 32 and at most what is held, without consuming them.
    fn peek(&self, n: u32) -> u64 {
        self.held.checked_shr(64_u32.saturating_sub(n)).unwrap_or(0)
    }

    /// Drops the next `n` bits, `n` at most what is held.
    fn consume(&mut self, n: u32) {
        self.held = self.held.checked_shl(n).unwrap_or(0);
        self.count = self.count.saturating_sub(n);
    }

    /// The next `n` bits as a number, `n` at most 32 (section F.2.2.4's `RECEIVE`, and a copy's
    /// stride).
    fn receive(&mut self, n: u32) -> u64 {
        self.refill();
        let value = self.peek(n);
        self.consume(n);
        value
    }

    /// Where the reader is, in bits from the start of the data.
    fn position(&self) -> u64 {
        u64::try_from(self.next)
            .unwrap_or(u64::MAX)
            .saturating_mul(8)
            .saturating_sub(u64::from(self.count))
    }
}

/// A big-endian writer of entropy-coded data, stuffing a `00` after every `FF` it writes
/// (section B.1.1.5).
struct Writer {
    out: Vec<u8>,
    held: u64,
    count: u32,
}

impl Writer {
    /// Appends the low `n` bits of `value`, `n` at most 32.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`n` is between 1 and 32 here, and `count` is under 8 on entry because every \
                  call ends by writing out whole bytes, so `64 − count − n` is at least 25 and \
                  `count + n` at most 39; the loop subtracts 8 only while `count >= 8`"
    )]
    fn write(&mut self, value: u64, n: u32) {
        if n == 0 {
            return;
        }
        let value = value & ((1u64 << n) - 1);
        self.held |= value << (64 - self.count - n);
        self.count += n;
        while self.count >= 8 {
            let byte = self.held.to_be_bytes()[0];
            self.out.push(byte);
            if byte == 0xFF {
                self.out.push(0x00);
            }
            self.held <<= 8;
            self.count -= 8;
        }
    }

    /// Copies bits `from..to` of `bytes`.
    fn copy(&mut self, bytes: &[u8], from: u64, to: u64) {
        let mut bits = Bits::at(bytes, from);
        let mut left = to.saturating_sub(from);
        while left > 0 {
            let take = u32::try_from(left.min(32)).unwrap_or(32);
            let value = bits.receive(take);
            self.write(value, take);
            left = left.saturating_sub(u64::from(take));
        }
    }

    /// Pads the last byte with one-bits (section F.1.2.3) and hands the bytes over.
    fn finish(mut self) -> Vec<u8> {
        let partial = self.count % 8;
        if partial != 0 {
            self.write(u64::MAX, 8_u32.saturating_sub(partial));
        }
        self.out
    }
}

/// One component of the scan: its tables and how many blocks it has in an MCU.
#[derive(Debug, Clone, Copy)]
struct ScanComponent {
    /// Its DC table's destination (`Td`).
    dc: usize,
    /// Its AC table's destination (`Ta`).
    ac: usize,
    /// Blocks in one MCU: `H × V` in an interleaved scan.
    blocks: u32,
}

/// What the pass records where a component's first block of an MCU row is coded.
#[derive(Debug, Clone, Copy)]
struct FirstBlock {
    /// Where the block's DC difference begins.
    opens: u64,
    /// Where it ends, after its additional bits.
    closes: u64,
    /// The DC coefficient the block decodes to: the prediction carried in plus its difference.
    value: i32,
}

/// Where one MCU row begins, and its first block of each component.
#[derive(Debug, Clone)]
struct RowStart {
    bit: u64,
    first: Vec<FirstBlock>,
}

/// The frame as the cut needs it.
struct Frame<'a> {
    data: &'a [u8],
    /// The bytes before the scan header, and where in them the frame header states `Y`.
    header: std::ops::Range<usize>,
    lines_at: usize,
    /// The scan header segment, marker included.
    scan: std::ops::Range<usize>,
    /// The scan's entropy-coded data, its stuffing removed.
    entropy: Vec<u8>,
    width: u32,
    lines: u32,
    mcu_lines: u32,
    mcus_per_row: u32,
    mcu_rows: u32,
    components: Vec<ScanComponent>,
    dc: [Option<Huffman>; 4],
    ac: [Option<Huffman>; 4],
}

/// Decodes `data` in bands at the rows a pass over its entropy-coded data finds, or `None` where
/// the codestream is not one the module comment admits, the pass declines it, or any band is
/// refused. `scan` is the caller's walk of `data`'s first scan, as [`restart::decode`] takes it.
pub(super) fn decode(
    data: &[u8],
    scan: &FirstScan,
    options: zune_jpeg::zune_core::options::DecoderOptions,
    channels: usize,
) -> Option<Vec<u8>> {
    let frame = read(data, scan)?;
    let samples = u64::from(frame.width).saturating_mul(u64::from(frame.lines));
    if samples < restart::BANDED_FLOOR {
        return None;
    }
    let bands = restart::plan_rows((frame.mcu_rows, frame.mcu_lines), 1, restart::BAND_LINES)?;
    let (rows, end) = pass(&frame)?;
    // Every band's first DC values are re-coded before any band is decoded, so that a table
    // without the code one needs declines the cut rather than wasting the bands begun before it.
    let encodable = bands.iter().all(|band| {
        band.decoded.0 == 0
            || rows
                .get(usize::try_from(band.decoded.0).unwrap_or(usize::MAX))
                .is_some_and(|row| {
                    row.first
                        .iter()
                        .zip(&frame.components)
                        .all(|(block, component)| {
                            dc_code(frame.dc.get(component.dc), block.value).is_some()
                        })
                })
    });
    if !encodable {
        return None;
    }
    let lines = Geometry {
        width: frame.width,
        lines: frame.lines,
        mcu_lines: frame.mcu_lines,
    };
    restart::decode_bands(lines, &bands, options, channels, &|band| {
        codestream(&frame, &rows, end, band)
    })
}

/// What a sequential frame header states that the cut reads.
struct FrameHeader {
    /// Where `Y` sits.
    lines_at: usize,
    /// `Y`.
    lines: u32,
    /// `X`.
    width: u32,
    /// Each component's identifier and sampling byte, in the header's order.
    components: Vec<(u8, u8)>,
}

/// Reads the marker segments up to the scan, or `None` for any codestream the module comment
/// declines.
fn read<'a>(data: &'a [u8], scan: &FirstScan) -> Option<Frame<'a>> {
    if data.get(..2)? != [0xFF, 0xD8] {
        return None;
    }
    let field = |at: usize| -> Option<usize> {
        Some(usize::from(u16::from_be_bytes([
            *data.get(at)?,
            *data.get(at.checked_add(1)?)?,
        ])))
    };
    let mut dc: [Option<Huffman>; 4] = Default::default();
    let mut ac: [Option<Huffman>; 4] = Default::default();
    let mut sof: Option<FrameHeader> = None;
    let mut at = 2usize;
    loop {
        let opens = at;
        if data.get(at) != Some(&0xFF) {
            return None;
        }
        while data.get(at) == Some(&0xFF) {
            at = at.checked_add(1)?;
        }
        let code = *data.get(at)?;
        at = at.checked_add(1)?;
        let length = field(at)?;
        if length < 2 {
            return None;
        }
        let ends = at.checked_add(length)?;
        let body = data.get(at.checked_add(2)?..ends)?;
        match code {
            0xC0 | 0xC1 => {
                if sof.is_some() || *body.first()? != 8 {
                    return None;
                }
                let lines = u32::try_from(field(at.checked_add(3)?)?).ok()?;
                let width = u32::try_from(field(at.checked_add(5)?)?).ok()?;
                let count = usize::from(*body.get(5)?);
                let components = (0..count)
                    .map(|index| {
                        let at = 6usize.checked_add(index.checked_mul(3)?)?;
                        Some((*body.get(at)?, *body.get(at.checked_add(1)?)?))
                    })
                    .collect::<Option<Vec<_>>>()?;
                sof = Some(FrameHeader {
                    lines_at: at.checked_add(3)?,
                    lines,
                    width,
                    components,
                });
            }
            0xC4 => {
                let mut rest = body;
                while let Some((&class_and_id, after)) = rest.split_first() {
                    let counts: &[u8; 16] = after.get(..16)?.try_into().ok()?;
                    let total = counts.iter().map(|&n| usize::from(n)).sum::<usize>();
                    let symbols = after.get(16..16usize.checked_add(total)?)?;
                    let table = Huffman::new(*counts, symbols)?;
                    let id = usize::from(class_and_id & 0x0F);
                    match class_and_id >> 4 {
                        0 => *dc.get_mut(id)? = Some(table),
                        1 => *ac.get_mut(id)? = Some(table),
                        _ => return None,
                    }
                    rest = after.get(16usize.checked_add(total)?..)?;
                }
            }
            // A `DRI` stating an interval is [`restart`]'s; one stating none changes nothing.
            0xDD => {
                if field(at.checked_add(2)?)? != 0 {
                    return None;
                }
            }
            // Every other frame, `DNL`, `DAC`, and markers with no business before the scan.
            0xC2
            | 0xC3
            | 0xC5..=0xC7
            | 0xC9..=0xCB
            | 0xCD..=0xCF
            | 0xDC
            | 0xCC
            | 0xD0..=0xD9
            | 0x01 => return None,
            0xDA => {
                return admit(data, scan, (opens, ends, body), &sof?, (dc, ac));
            }
            _ => {}
        }
        at = ends;
    }
}

/// The scan header `body` (its segment `opens..ends`) against the frame, and the entropy-coded
/// data unstuffed — or `None` for a scan or a shape the module comment declines.
fn admit<'a>(
    data: &'a [u8],
    scan: &FirstScan,
    (opens, ends, body): (usize, usize, &[u8]),
    header: &FrameHeader,
    (dc, ac): ([Option<Huffman>; 4], [Option<Huffman>; 4]),
) -> Option<Frame<'a>> {
    let FrameHeader {
        lines_at,
        lines,
        width,
        ref components,
    } = *header;
    let frame_components = components.as_slice();
    let count = usize::from(*body.first()?);
    if count != frame_components.len() || lines == 0 || width == 0 || scan.data_starts != ends {
        return None;
    }
    // Spectral selection 0 to 63 and no successive approximation: a sequential scan.
    let tail = 1usize.checked_add(count.checked_mul(2)?)?;
    if body.get(tail..tail.checked_add(3)?)? != [0, 63, 0] {
        return None;
    }
    // [`restart`]'s shapes: one component of 1 × 1, or three whose first is 1 × 1, 2 × 1, 1 × 2 or
    // 2 × 2 and whose other two are 1 × 1.
    let first = frame_components.first()?.1;
    let shaped = match frame_components {
        [_] => first == 0x11,
        [_, (_, second), (_, third)] => {
            *second == 0x11
                && *third == 0x11
                && matches!((first >> 4, first & 0x0F), (1 | 2, 1 | 2))
        }
        _ => false,
    };
    if !shaped {
        return None;
    }
    let (h, v) = (u32::from(first >> 4), u32::from(first & 0x0F));
    let mut components = Vec::with_capacity(count);
    for index in 0..count {
        let at = 1usize.checked_add(index.checked_mul(2)?)?;
        let (id, tables) = (*body.get(at)?, *body.get(at.checked_add(1)?)?);
        let (_, sampling) = frame_components.iter().find(|(own, _)| *own == id)?;
        let blocks = if count == 1 {
            1
        } else {
            u32::from(sampling >> 4).checked_mul(u32::from(sampling & 0x0F))?
        };
        let component = ScanComponent {
            dc: usize::from(tables >> 4),
            ac: usize::from(tables & 0x0F),
            blocks,
        };
        dc.get(component.dc)?.as_ref()?;
        ac.get(component.ac)?.as_ref()?;
        components.push(component);
    }
    let (mcu_width, mcu_lines) = (h.checked_mul(8)?, v.checked_mul(8)?);
    Some(Frame {
        data,
        header: 0..opens,
        lines_at,
        scan: opens..ends,
        entropy: unstuffed(data, scan)?,
        width,
        lines,
        mcu_lines,
        mcus_per_row: width.div_ceil(mcu_width),
        mcu_rows: lines.div_ceil(mcu_lines),
        components,
        dc,
        ac,
    })
}

/// The scan's entropy-coded data with every stuffed `00` taken out, or `None` where an `FF` in it
/// is anything but a stuffed pair — a restart marker, or fill bytes before one.
fn unstuffed(data: &[u8], scan: &FirstScan) -> Option<Vec<u8>> {
    let end = scan.ends.as_ref().map_or(data.len(), |marker| marker.opens);
    let mut entropy = Vec::with_capacity(end.checked_sub(scan.data_starts)?);
    let mut from = scan.data_starts;
    for marker in &scan.inside {
        if marker.code != 0x00 || marker.resumes != marker.opens.checked_add(2)? {
            return None;
        }
        entropy.extend_from_slice(data.get(from..=marker.opens)?);
        from = marker.resumes;
    }
    if let Some(marker) = &scan.ends
        && marker.code != 0xD9
    {
        return None;
    }
    entropy.extend_from_slice(data.get(from..end)?);
    Some(entropy)
}

/// Section F.2.2.1's `EXTEND`: the difference `value` stands for in a category of `size` bits.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "`size` is a DC category, at most 11 where the pass calls this, so every shift and \
              sum is under 2^12"
)]
fn extend(value: u64, size: u32) -> i32 {
    let value = i32::try_from(value).unwrap_or(0);
    if size == 0 {
        0
    } else if value < 1 << (size - 1) {
        value - (1 << size) + 1
    } else {
        value
    }
}

/// The pass: every MCU row's first bit and first blocks, and where the last MCU ends — or `None`
/// where the data does not decode as the module comment requires.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "`coefficient` stays at most 64 + 16 by the checks after each step, and a run is at \
              most 15"
)]
fn pass(frame: &Frame<'_>) -> Option<(Vec<RowStart>, u64)> {
    let mut bits = Bits::at(&frame.entropy, 0);
    let mut predictions = vec![0i32; frame.components.len()];
    let mut rows = Vec::with_capacity(usize::try_from(frame.mcu_rows).ok()?);
    let tables = frame
        .components
        .iter()
        .map(|component| {
            Some((
                frame.dc.get(component.dc)?.as_ref()?,
                frame.ac.get(component.ac)?.as_ref()?,
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    for _ in 0..frame.mcu_rows {
        let mut first = Vec::with_capacity(frame.components.len());
        let bit = bits.position();
        for mcu in 0..frame.mcus_per_row {
            for ((component, (dc, ac)), prediction) in
                frame.components.iter().zip(&tables).zip(&mut predictions)
            {
                for block in 0..component.blocks {
                    let opens = bits.position();
                    let size = u32::from(dc.decode(&mut bits)?);
                    if size > 11 {
                        return None;
                    }
                    *prediction = prediction.checked_add(extend(bits.receive(size), size))?;
                    if mcu == 0 && block == 0 {
                        first.push(FirstBlock {
                            opens,
                            closes: bits.position(),
                            value: *prediction,
                        });
                    }
                    let mut coefficient = 1u32;
                    while coefficient < 64 {
                        let symbol = ac.decode(&mut bits)?;
                        let (run, size) = (u32::from(symbol >> 4), u32::from(symbol & 0x0F));
                        if size == 0 {
                            if run != 15 {
                                break;
                            }
                            coefficient += 16;
                            if coefficient > 64 {
                                return None;
                            }
                            continue;
                        }
                        coefficient += run;
                        if coefficient > 63 || size > 10 {
                            return None;
                        }
                        bits.receive(size);
                        coefficient += 1;
                    }
                }
            }
        }
        rows.push(RowStart { bit, first });
    }
    let end = bits.position();
    let carried = u64::try_from(frame.entropy.len()).ok()?.checked_mul(8)?;
    (end <= carried).then_some((rows, end))
}

/// The code and additional bits section F.1.2.1 gives a DC difference of `value` against a
/// prediction of zero, in `table`, or `None` where the table has no code for its category.
fn dc_code(table: Option<&Option<Huffman>>, value: i32) -> Option<((u16, u8), u64, u32)> {
    let size = u32::BITS.saturating_sub(value.unsigned_abs().leading_zeros());
    let code = (*table?.as_ref()?.codes.get(usize::try_from(size).ok()?)?)?;
    // A negative difference's additional bits are its value less one, in `size` bits.
    let bits = if value < 0 {
        u64::from(value.wrapping_sub(1).cast_unsigned())
    } else {
        u64::from(value.cast_unsigned())
    };
    Some((code, bits, size))
}

/// One band as a codestream of its own: the frame's header segments with `Y` set to the band's
/// lines, the scan header, the band's rows' bits with each component's first DC difference
/// re-coded from a prediction of zero, padded, and `EOI`.
fn codestream(frame: &Frame<'_>, rows: &[RowStart], end: u64, band: Band) -> Option<Vec<u8>> {
    let (first, last) = band.decoded;
    let finishes = last == frame.mcu_rows;
    let lines = if finishes {
        frame
            .lines
            .checked_sub(first.checked_mul(frame.mcu_lines)?)?
    } else {
        last.checked_sub(first)?.checked_mul(frame.mcu_lines)?
    };
    let lines = u16::try_from(lines).ok()?;
    let start = rows.get(usize::try_from(first).ok()?)?;
    let stop = if finishes {
        end
    } else {
        rows.get(usize::try_from(last).ok()?)?.bit
    };

    let mut writer = Writer {
        out: Vec::with_capacity(
            usize::try_from(stop.saturating_sub(start.bit) / 8)
                .ok()?
                .saturating_add(64),
        ),
        held: 0,
        count: 0,
    };
    let mut cursor = start.bit;
    if first > 0 {
        for (block, component) in start.first.iter().zip(&frame.components) {
            writer.copy(&frame.entropy, cursor, block.opens);
            let ((code, length), bits, size) = dc_code(frame.dc.get(component.dc), block.value)?;
            writer.write(u64::from(code), u32::from(length));
            writer.write(bits, size);
            cursor = block.closes;
        }
    }
    writer.copy(&frame.entropy, cursor, stop);
    let entropy = writer.finish();

    let header = frame.data.get(frame.header.clone())?;
    let scan = frame.data.get(frame.scan.clone())?;
    let mut codestream = Vec::with_capacity(
        header
            .len()
            .checked_add(scan.len())?
            .checked_add(entropy.len())?
            .checked_add(2)?,
    );
    codestream.extend_from_slice(header);
    codestream
        .get_mut(frame.lines_at..frame.lines_at.checked_add(2)?)?
        .copy_from_slice(&lines.to_be_bytes());
    codestream.extend_from_slice(scan);
    codestream.extend_from_slice(&entropy);
    codestream.extend_from_slice(&[0xFF, 0xD9]);
    Some(codestream)
}

#[cfg(test)]
mod tests {
    use super::{Bits, Huffman, Writer, decode, extend};
    use zune_jpeg::zune_core::colorspace::ColorSpace;
    use zune_jpeg::zune_core::options::DecoderOptions;

    /// Codestreams with no restart interval, each a 48 × 200 synthetic image (the restart
    /// fixtures' picture) encoded by `libjpeg-turbo`'s `cjpeg` at quality 90 under every sampling
    /// shape the module admits, and one with `-optimize`, whose tables hold only the codes the
    /// image uses — so a re-coded first DC value can find its category missing.
    const FIXTURES: [(&str, &[u8]); 6] = [
        ("hv", include_bytes!("../../tests/cut/hv.jpg")),
        ("h", include_bytes!("../../tests/cut/h.jpg")),
        ("v", include_bytes!("../../tests/cut/v.jpg")),
        ("none", include_bytes!("../../tests/cut/none.jpg")),
        ("grey", include_bytes!("../../tests/cut/grey.jpg")),
        (
            "hv_optimised",
            include_bytes!("../../tests/cut/hv_optimised.jpg"),
        ),
    ];

    fn rgba() -> DecoderOptions {
        DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA)
    }

    /// Every fixture cut at the rows the pass finds decodes to the bytes the whole frame's decoder
    /// writes, at every band height that cuts it — or, for a table missing a code a band needs,
    /// is declined and left to that decoder.
    #[test]
    fn a_frame_cut_where_the_pass_finds_its_rows_is_the_whole_frame() {
        for (name, data) in FIXTURES {
            let whole = zune_jpeg::JpegDecoder::new_with_options(
                zune_jpeg::zune_core::bytestream::ZCursor::new(data),
                rgba(),
            )
            .decode()
            .expect("the whole frame decodes");
            let scan = super::super::first_scan(data).expect("a scan");
            let frame = super::read(data, &scan).unwrap_or_else(|| panic!("{name} is admitted"));
            let (rows, end) = super::pass(&frame).unwrap_or_else(|| panic!("{name} passes"));
            assert_eq!(rows.len(), usize::try_from(frame.mcu_rows).expect("rows"));
            let mut cut = 0;
            for lines in [8, 16, 24, 40, 64] {
                let Some(bands) =
                    super::restart::plan_rows((frame.mcu_rows, frame.mcu_lines), 1, lines)
                else {
                    continue;
                };
                let codestreams: Option<Vec<_>> = bands
                    .iter()
                    .map(|band| super::codestream(&frame, &rows, end, *band))
                    .collect();
                let Some(_) = codestreams else {
                    assert_eq!(name, "hv_optimised", "{name} declined a band");
                    continue;
                };
                cut += 1;
                let banded = super::restart::decode_bands(
                    super::Geometry {
                        width: frame.width,
                        lines: frame.lines,
                        mcu_lines: frame.mcu_lines,
                    },
                    &bands,
                    rgba(),
                    4,
                    &|band| super::codestream(&frame, &rows, end, band),
                )
                .unwrap_or_else(|| panic!("{name} decodes in bands of {lines} lines"));
                assert!(
                    banded == whole,
                    "{name} in bands of {lines} lines moved a byte"
                );
            }
            assert!(
                cut >= 2 || name == "hv_optimised",
                "{name} was cut at two band heights or more"
            );
            // Below the floor the whole frame is the decoder's, and the entry point says so.
            assert_eq!(
                decode(data, &scan, rgba(), 4),
                None,
                "{name} is under the floor"
            );
        }
    }

    /// A table built from a `DHT`'s counts decodes each code it assigns to its symbol, through the
    /// nine-bit prefix and past it, and gives the code back for re-coding.
    #[test]
    fn a_table_decodes_the_codes_it_assigns() {
        // Two codes of length 2, one of 3, one of 10 and one of 16: symbols 5, 6, 7, 8, 9.
        let mut counts = [0u8; 16];
        counts[1] = 2;
        counts[2] = 1;
        counts[9] = 1;
        counts[15] = 1;
        let table = Huffman::new(counts, &[5, 6, 7, 8, 9]).expect("a prefix code");
        for symbol in [5u8, 6, 7, 8, 9] {
            let (code, length) = table.codes[usize::from(symbol)].expect("a code");
            let mut writer = Writer {
                out: Vec::new(),
                held: 0,
                count: 0,
            };
            writer.write(u64::from(code), u32::from(length));
            let bytes = writer.finish();
            let mut bits = Bits::at(&bytes, 0);
            assert_eq!(table.decode(&mut bits), Some(symbol));
            assert_eq!(bits.position(), u64::from(length));
        }
        // Three codes of length 1 are not a prefix code.
        let mut counts = [0u8; 16];
        counts[0] = 3;
        assert!(Huffman::new(counts, &[1, 2, 3]).is_none());
    }

    /// A copied run of bits is the run, wherever it starts and ends inside a byte, and an `FF` the
    /// writer puts out is followed by its stuffed `00`.
    #[test]
    fn bits_copied_across_bytes_are_the_same_bits() {
        let source = [0b1011_0011, 0xFF, 0b0101_1010, 0x0F, 0xA5];
        for from in 0..20u64 {
            for to in from..40u64 {
                let mut writer = Writer {
                    out: Vec::new(),
                    held: 0,
                    count: 0,
                };
                writer.copy(&source, from, to);
                let written = writer.finish();
                let mut unstuffed = Vec::new();
                let mut skip = false;
                for &byte in &written {
                    if skip {
                        assert_eq!(byte, 0, "a stuffed zero after FF");
                        skip = false;
                        continue;
                    }
                    unstuffed.push(byte);
                    skip = byte == 0xFF;
                }
                let (mut a, mut b) = (Bits::at(&source, from), Bits::at(&unstuffed, 0));
                for _ in from..to {
                    assert_eq!(a.receive(1), b.receive(1), "{from}..{to}");
                }
            }
        }
    }

    /// Section F.2.2.1's `EXTEND` on each side of a category's midpoint.
    #[test]
    fn extend_reads_a_category_both_ways() {
        assert_eq!(extend(0, 0), 0);
        assert_eq!(extend(0, 1), -1);
        assert_eq!(extend(1, 1), 1);
        assert_eq!(extend(0b00, 2), -3);
        assert_eq!(extend(0b01, 2), -2);
        assert_eq!(extend(0b10, 2), 2);
        assert_eq!(extend(0b11, 2), 3);
    }
}
