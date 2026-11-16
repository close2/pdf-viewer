//! Data Matrix ECC 200, XFA 3.3's `dataMatrix`, from permissively licensed open source.
//!
//! # The source, and why it is not a standard
//!
//! The template reference (XFA 3.3 page 617) defines `dataMatrix` by ANSI/AIM BC11, whose
//! successor is ISO/IEC 16022; neither is held, and no public document prints the encodations,
//! the Reed-Solomon field or the module placement (`doc/questions/Q857`). The owner's answer A857
//! lifts principle 5 for this symbology alone: its tables and algorithms may be derived from
//! permissively licensed implementations, verified by reading every symbol back with an
//! independent decoder. Everything below is derived from **`rxing` 0.9.4** (Apache-2.0, a port of
//! `ZXing`'s encoder by Jeremias Maerki), the files named beside each item under
//! `src/datamatrix/encoder/`; `doc/third-party-data.md` records the copy. The symbol is read back
//! by `rxing`'s *decoder*, which shares no code with its encoder, so a round trip tests the
//! derivation and not the copy against itself — but both halves come from one project, and
//! agreement is evidence about this crate's reading, not proof (trap 9).
//!
//! # What XFA 3.3 decides
//!
//! - **ECC 200, and square symbols only.** Page 442: the ratio of rows to columns and of module
//!   width to height are both fixed at 1:1, the size depends on the payload, and processors default
//!   to ECC-200; [`SIZES`] holds the 24 square sizes and no rectangular one.
//! - **The data is bytes.** Page 424: the data is serialised as bytes before it is placed in a
//!   barcode, UTF-8 unless `charEncoding` says otherwise (page 611), so this module takes bytes and
//!   the caller serialises.
//! - **What it ignores.** `checksum` (page 614 says a type that takes `errorCorrectionLevel` ignores
//!   it, and ECC 200's level is fixed by the size), `moduleHeight` (page 437), and
//!   `dataRowCount`, `dataColumnCount` and `rowColumnRatio` (page 442: the size depends on the
//!   payload).
//!
//! # The construction
//!
//! The data becomes **data codewords** in one encodation ([`Encodation`]), padded to the smallest
//! size that holds them; the **error correction codewords** are computed per interleaved block over
//! GF(2⁸) with the field polynomial `0x12D`; the codewords are **placed** into the mapping matrix by
//! the diagonal "utah" walk with its four corner shapes; and the matrix is cut into data regions,
//! each framed by its finder pattern. Which encodation a symbol uses is not a requirement of the
//! symbology — any encodation the decoder reads yields the same data — so [`encode`] makes a
//! documented choice: the encodation with the fewest codewords, the whole message in one mode.
//! Switching modes inside a message (the informative look-ahead of ISO/IEC 16022's annex) would
//! shorten some mixed messages and is not built.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "every quantity is bounded before it is computed: [`codewords`] refuses more than \
              `MOST_BYTES` bytes, a size is at most 144 modules a side and 2 178 codewords, a \
              codeword or a value is a byte, and the field arithmetic reduces modulo 255; the \
              `xfa_barcode` fuzz target holds the claim (ADR 0096)"
)]

use crate::{Matrix, Refusal, Symbology};

/// One square ECC 200 symbol size: `rxing`'s `PROD_SYMBOLS` (`symbol_info.rs`), its square rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    /// Modules per side, finder patterns included, quiet zone excluded.
    pub side: usize,
    /// Data regions per side.
    pub regions: usize,
    /// Data codewords the symbol holds.
    pub data: usize,
    /// Error correction codewords, all blocks together.
    pub error: usize,
    /// Interleaved Reed-Solomon blocks.
    pub blocks: usize,
}

impl Size {
    /// Modules per side of one data region, its finder pattern excluded.
    #[must_use]
    pub fn region(self) -> usize {
        self.side / self.regions - 2
    }

    /// Modules per side of the mapping matrix: every data region's modules, no finder.
    #[must_use]
    pub fn mapping(self) -> usize {
        self.region() * self.regions
    }
}

const fn size(side: usize, regions: usize, data: usize, error: usize, blocks: usize) -> Size {
    Size {
        side,
        regions,
        data,
        error,
        blocks,
    }
}

/// The 24 square sizes, smallest first: side, regions per side, data and error codewords, blocks.
/// Transcribed from `rxing`'s `symbol_info.rs` row by row (a row's block count is its data
/// capacity over its per-block data, the table's last two columns).
pub const SIZES: [Size; 24] = [
    size(10, 1, 3, 5, 1),
    size(12, 1, 5, 7, 1),
    size(14, 1, 8, 10, 1),
    size(16, 1, 12, 12, 1),
    size(18, 1, 18, 14, 1),
    size(20, 1, 22, 18, 1),
    size(22, 1, 30, 20, 1),
    size(24, 1, 36, 24, 1),
    size(26, 1, 44, 28, 1),
    size(32, 2, 62, 36, 1),
    size(36, 2, 86, 42, 1),
    size(40, 2, 114, 48, 1),
    size(44, 2, 144, 56, 1),
    size(48, 2, 174, 68, 1),
    size(52, 2, 204, 84, 2),
    size(64, 4, 280, 112, 2),
    size(72, 4, 368, 144, 4),
    size(80, 4, 456, 192, 4),
    size(88, 4, 576, 224, 4),
    size(96, 4, 696, 272, 4),
    size(104, 4, 816, 336, 6),
    size(120, 6, 1050, 408, 6),
    size(132, 6, 1304, 496, 8),
    size(144, 6, 1558, 620, 10),
];

/// The encodations a message may be written in (`rxing`'s `*_encoder.rs`, one file each).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encodation {
    /// One codeword a byte, two digits to a codeword (`ascii_encoder.rs`).
    Ascii,
    /// Three values to two codewords, upper-case letters and digits in the basic set
    /// (`c40_encoder.rs`).
    C40,
    /// As C40 with the cases of the letters exchanged (`text_encoder.rs`).
    Text,
    /// Three values to two codewords, ANSI X12's 40 characters only (`x12_encoder.rs`).
    X12,
    /// Four six-bit values to three codewords, bytes 32 to 94 only (`edifact_encoder.rs`).
    Edifact,
    /// Bytes as they are, behind a length field, randomised (`base256_encoder.rs`).
    Base256,
}

/// Every encodation, in the order [`encode`] prefers on a tie.
pub const ENCODATIONS: [Encodation; 6] = [
    Encodation::Ascii,
    Encodation::C40,
    Encodation::Text,
    Encodation::X12,
    Encodation::Edifact,
    Encodation::Base256,
];

/// The codeword that pads unused data capacity (`high_level_encoder.rs`, `PAD`).
pub const PAD: u8 = 129;
const UPPER_SHIFT: u8 = 235;
const LATCH_C40: u8 = 230;
const LATCH_BASE256: u8 = 231;
const LATCH_X12: u8 = 238;
const LATCH_TEXT: u8 = 239;
const LATCH_EDIFACT: u8 = 240;
const UNLATCH: u8 = 254;
const EDIFACT_UNLATCH: u8 = 0x1F;
/// The most bytes any encodation fits into the largest size: 1 558 codewords of ASCII digit pairs.
const MOST_BYTES: usize = 2 * 1558;
/// The longest Base 256 field a two-codeword length can state (`base256_encoder.rs`).
const BASE256_LONGEST: usize = 1555;

fn data_matrix() -> Symbology {
    Symbology("dataMatrix")
}

/// A message's data codewords in one encodation, before padding, and the least data capacity
/// a symbol needs to hold them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Codewords {
    /// The data codewords.
    pub codewords: Vec<u8>,
    /// The least data capacity that holds them: their number, or more where an EDIFACT group must
    /// fit whole (see [`codewords`]).
    pub needs: usize,
}

/// ASCII encodation: a digit pair is `130 + value`, a byte up to 127 is itself plus one, and a byte
/// above is the upper shift and the byte less 127 (`ascii_encoder.rs`).
fn ascii(data: &[u8], into: &mut Vec<u8>) {
    let mut at = 0;
    while let Some(&byte) = data.get(at) {
        match data.get(at + 1) {
            Some(&next) if byte.is_ascii_digit() && next.is_ascii_digit() => {
                into.push(130 + (byte - b'0') * 10 + (next - b'0'));
                at += 2;
                continue;
            }
            _ => {}
        }
        if byte < 128 {
            into.push(byte + 1);
        } else {
            into.push(UPPER_SHIFT);
            into.push(byte - 127);
        }
        at += 1;
    }
}

/// A byte's C40 or Text values (`c40_encoder.rs`, `text_encoder.rs`, `encodeChar`): the basic set
/// directly, the rest behind shift 1 (controls), shift 2 (punctuation) or shift 3, and a byte above
/// 127 behind shift 2's upper shift (value 30).
fn triple_values(byte: u8, text: bool, into: &mut Vec<u8>) {
    if byte >= 128 {
        into.extend([1, 30]);
        triple_values(byte - 128, text, into);
        return;
    }
    let (basic, other) = if text {
        (b'a'..=b'z', b'A'..=b'Z')
    } else {
        (b'A'..=b'Z', b'a'..=b'z')
    };
    match byte {
        b' ' => into.push(3),
        b'0'..=b'9' => into.push(byte - b'0' + 4),
        _ if basic.contains(&byte) => into.push(byte - basic.start() + 14),
        0..=31 => into.extend([0, byte]),
        b'!'..=b'/' => into.extend([1, byte - b'!']),
        b':'..=b'@' => into.extend([1, byte - b':' + 15]),
        b'['..=b'_' => into.extend([1, byte - b'[' + 22]),
        b'`' => into.extend([2, 0]),
        _ if other.contains(&byte) => into.extend([2, byte - other.start() + 1]),
        // `{` to DEL: 27 to 31 in shift 3 (the C40 and Text tables agree from `{` on).
        _ => into.extend([2, byte - b'{' + 27]),
    }
}

/// An X12 byte's value (`x12_encoder.rs`): CR, `*`, `>`, space, the digits and the capitals.
fn x12_value(byte: u8) -> Option<u8> {
    match byte {
        b'\r' => Some(0),
        b'*' => Some(1),
        b'>' => Some(2),
        b' ' => Some(3),
        b'0'..=b'9' => Some(byte - b'0' + 4),
        b'A'..=b'Z' => Some(byte - b'A' + 14),
        _ => None,
    }
}

/// Three values as two codewords: `1600 a + 40 b + c + 1`, high byte first (`writeNextTriplet`).
fn triplets(values: &[u8], into: &mut Vec<u8>) {
    for triplet in values.chunks(3) {
        let value = triplet
            .iter()
            .fold(0_u16, |sum, value| sum * 40 + u16::from(*value))
            + 1;
        into.extend(value.to_be_bytes());
    }
}

/// The C40, Text or X12 codewords: the latch, as many whole triplets as the bytes fill, the
/// unlatch, and the bytes left over in ASCII.
///
/// A message whose values end one short of a triplet gives its last bytes to ASCII until they do
/// not; two short are completed with a shift 1 value as the pad (`handleEOD_c40`, `rest == 2`).
/// The unlatch is always written — its omission when a symbol ends on a triplet is an option, not
/// a requirement, and writing it keeps the end of every message one rule.
fn triple_mode(data: &[u8], encodation: Encodation) -> Result<Vec<u8>, Refusal> {
    let latch = match encodation {
        Encodation::C40 => LATCH_C40,
        Encodation::Text => LATCH_TEXT,
        _ => LATCH_X12,
    };
    let mut per_byte: Vec<Vec<u8>> = Vec::with_capacity(data.len());
    for &byte in data {
        let mut values = Vec::new();
        if encodation == Encodation::X12 {
            let value = x12_value(byte).ok_or(Refusal::Unencodable {
                symbology: data_matrix(),
                character: char::from(byte),
            })?;
            values.push(value);
        } else {
            triple_values(byte, encodation == Encodation::Text, &mut values);
        }
        per_byte.push(values);
    }
    let mut taken = per_byte.len();
    let count = |taken: usize| per_byte.iter().take(taken).map(Vec::len).sum::<usize>();
    while taken > 0
        && (count(taken) % 3 == 1 || (encodation == Encodation::X12 && count(taken) % 3 != 0))
    {
        taken -= 1;
    }
    let mut values: Vec<u8> = per_byte.iter().take(taken).flatten().copied().collect();
    if values.len() % 3 == 2 {
        values.push(0);
    }
    let mut codewords = vec![latch];
    triplets(&values, &mut codewords);
    codewords.push(UNLATCH);
    ascii(data.get(taken..).unwrap_or_default(), &mut codewords);
    Ok(codewords)
}

/// The EDIFACT codewords: the latch, each byte's low six bits, the unlatch value `0x1F`, packed
/// four values to three codewords with the last codeword's unused bits zero (`edifact_encoder.rs`).
/// Returns the codewords and the capacity the last group needs: the decoder reads EDIFACT three
/// codewords at a time and returns to ASCII where fewer remain, so the symbol must hold the whole
/// group the unlatch ends.
fn edifact(data: &[u8]) -> Result<(Vec<u8>, usize), Refusal> {
    let mut values = Vec::with_capacity(data.len() + 1);
    for &byte in data {
        if !(32..=94).contains(&byte) {
            return Err(Refusal::Unencodable {
                symbology: data_matrix(),
                character: char::from(byte),
            });
        }
        values.push(byte & 0x3F);
    }
    values.push(EDIFACT_UNLATCH);
    let mut codewords = vec![LATCH_EDIFACT];
    let mut bits = 0_u32;
    let mut held = 0;
    for value in &values {
        bits = (bits << 6) | u32::from(*value);
        held += 6;
        while held >= 8 {
            held -= 8;
            codewords.extend(u8::try_from((bits >> held) & 0xFF).ok());
        }
    }
    if held > 0 {
        codewords.extend(u8::try_from((bits << (8 - held)) & 0xFF).ok());
    }
    let needs = 1 + values.len().div_ceil(4) * 3;
    Ok((codewords, needs))
}

/// One codeword randomised by the 255-state algorithm at its 1-based position
/// (`randomize255State`).
fn randomise_255(codeword: u8, position: usize) -> u8 {
    let pseudo = (149 * position) % 255 + 1;
    let value = usize::from(codeword) + pseudo;
    // `pseudo` is at most 255, so `value` is below 511 and one subtraction brings it under 256.
    u8::try_from(if value <= 255 { value } else { value - 256 }).unwrap_or(0)
}

/// The Base 256 codewords: the latch, the length (one codeword to 249, two to 1555), the bytes, the
/// length and the bytes randomised by their positions (`base256_encoder.rs`). The length is always
/// written; its zero form, "to the end of the symbol", is an option this module does not take,
/// and because a length of zero means that, an empty message writes no field at all.
fn base256(data: &[u8]) -> Result<Vec<u8>, Refusal> {
    if data.is_empty() {
        return Ok(Vec::new());
    }
    if data.len() > BASE256_LONGEST {
        return Err(Refusal::Capacity {
            symbology: data_matrix(),
            codewords: data.len() + 3,
        });
    }
    let mut field = Vec::with_capacity(data.len() + 2);
    match u8::try_from(data.len()) {
        Ok(short) if short <= 249 => field.push(short),
        _ => {
            // Bounded by BASE256_LONGEST above: the quotient is at most 6 and the rest below 250.
            field.push(u8::try_from(data.len() / 250 + 249).unwrap_or(u8::MAX));
            field.push(u8::try_from(data.len() % 250).unwrap_or(u8::MAX));
        }
    }
    field.extend_from_slice(data);
    let mut codewords = vec![LATCH_BASE256];
    for (offset, codeword) in field.into_iter().enumerate() {
        // The latch is position 1; the field begins at 2.
        codewords.push(randomise_255(codeword, offset + 2));
    }
    Ok(codewords)
}

/// A message's data codewords in one encodation.
///
/// # Errors
///
/// [`Refusal::Unencodable`] for a byte X12 or EDIFACT does not hold, and [`Refusal::Capacity`]
/// for a Base 256 field past its length's reach.
pub fn codewords(data: &[u8], encodation: Encodation) -> Result<Codewords, Refusal> {
    if data.len() > MOST_BYTES {
        return Err(Refusal::Capacity {
            symbology: data_matrix(),
            codewords: data.len().div_ceil(2),
        });
    }
    let (codewords, needs) = match encodation {
        Encodation::Ascii => {
            let mut codewords = Vec::with_capacity(data.len());
            ascii(data, &mut codewords);
            let needs = codewords.len();
            (codewords, needs)
        }
        Encodation::C40 | Encodation::Text | Encodation::X12 => {
            let codewords = triple_mode(data, encodation)?;
            let needs = codewords.len();
            (codewords, needs)
        }
        Encodation::Edifact => edifact(data)?,
        Encodation::Base256 => {
            let codewords = base256(data)?;
            let needs = codewords.len();
            (codewords, needs)
        }
    };
    Ok(Codewords {
        needs: needs.max(codewords.len()),
        codewords,
    })
}

/// The smallest size whose data capacity is at least `needs`.
#[must_use]
pub fn smallest(needs: usize) -> Option<Size> {
    SIZES.iter().copied().find(|size| size.data >= needs)
}

/// Pad data codewords to the size's capacity: one [`PAD`], then pads randomised by the 253-state
/// algorithm at their 1-based positions (`high_level_encoder.rs`, `randomize253State`).
fn pad(codewords: &mut Vec<u8>, capacity: usize) {
    if codewords.len() < capacity {
        codewords.push(PAD);
    }
    while codewords.len() < capacity {
        let position = codewords.len() + 1;
        let pseudo = (149 * position) % 253 + 1;
        let value = usize::from(PAD) + pseudo;
        // At most 129 + 253 = 382, and one subtraction of 254 brings it to 1..=254.
        codewords.push(u8::try_from(if value <= 254 { value } else { value - 254 }).unwrap_or(PAD));
    }
}

/// GF(2⁸) with the field polynomial `x⁸ + x⁵ + x³ + x² + 1`, `0x12D` (`error_correction.rs`'s
/// `LOG` and `ALOG`, which it builds from that polynomial): `EXP[i]` is α to the power `i`.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the value is reduced below 256 by the field polynomial before it is stored"
)]
const EXP: [u8; 255] = {
    let mut table = [0_u8; 255];
    let mut value: u16 = 1;
    let mut power = 0;
    while power < 255 {
        table[power] = value as u8;
        value <<= 1;
        if value >= 256 {
            value ^= 0x12D;
        }
        power += 1;
    }
    table
};

/// The discrete logarithm, `LOG[EXP[i]] == i`; `LOG[0]` is unused.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a power below 255 and a field element below 256 are each a byte"
)]
const LOG: [u8; 256] = {
    let mut table = [0_u8; 256];
    let mut power = 0;
    while power < 255 {
        table[EXP[power] as usize] = power as u8;
        power += 1;
    }
    table
};

fn multiply(left: u8, right: u8) -> u8 {
    if left == 0 || right == 0 {
        return 0;
    }
    let power = (usize::from(LOG[usize::from(left)]) + usize::from(LOG[usize::from(right)])) % 255;
    EXP.get(power).copied().unwrap_or(0)
}

/// The generator polynomial of `count` error correction codewords, `(x + α¹)(x + α²)…(x + αⁿ)`,
/// coefficients from the highest power down, the leading 1 omitted (`error_correction.rs`'s
/// `FACTORS` are these, lowest power first).
fn generator(count: usize) -> Vec<u8> {
    let mut polynomial = vec![1_u8];
    for root in 1..=count {
        let alpha = EXP.get(root % 255).copied().unwrap_or(1);
        let mut next = vec![0_u8; polynomial.len() + 1];
        for (power, coefficient) in polynomial.iter().enumerate() {
            if let Some(slot) = next.get_mut(power) {
                *slot ^= coefficient;
            }
            if let Some(slot) = next.get_mut(power + 1) {
                *slot ^= multiply(*coefficient, alpha);
            }
        }
        polynomial = next;
    }
    polynomial.split_off(1)
}

/// The Reed-Solomon check codewords of one block: the remainder of the block, times `xⁿ`, divided
/// by the generator (`createECCBlock`).
#[must_use]
pub fn error_codewords(block: &[u8], count: usize) -> Vec<u8> {
    let generator = generator(count);
    let mut remainder = vec![0_u8; count];
    for codeword in block {
        let factor = codeword ^ remainder.first().copied().unwrap_or(0);
        remainder.rotate_left(1);
        if let Some(last) = remainder.last_mut() {
            *last = 0;
        }
        for (slot, coefficient) in remainder.iter_mut().zip(&generator) {
            *slot ^= multiply(factor, *coefficient);
        }
    }
    remainder
}

/// Data and error codewords interleaved: data codeword `i` belongs to block `i mod b`, and error
/// codeword `j` of block `k` is placed at `data + j b + k` (`encodeECC200`).
fn with_error_correction(data: &[u8], size: Size) -> Vec<u8> {
    let per_block = size.error / size.blocks;
    let mut all = data.to_vec();
    all.resize(size.data + size.error, 0);
    for block in 0..size.blocks {
        let members: Vec<u8> = data
            .iter()
            .skip(block)
            .step_by(size.blocks)
            .copied()
            .collect();
        for (index, codeword) in error_codewords(&members, per_block).into_iter().enumerate() {
            if let Some(slot) = all.get_mut(size.data + index * size.blocks + block) {
                *slot = codeword;
            }
        }
    }
    all
}

/// The mapping matrix being filled: each module unset, dark or light.
struct Mapping {
    side: usize,
    modules: Vec<Option<bool>>,
}

impl Mapping {
    fn at(&self, row: usize, column: usize) -> Option<bool> {
        self.modules
            .get(row * self.side + column)
            .copied()
            .flatten()
    }

    fn set(&mut self, row: usize, column: usize, dark: bool) {
        if let Some(slot) = self.modules.get_mut(row * self.side + column) {
            *slot = Some(dark);
        }
    }

    /// Bit `bit` (1 the most significant) of `codeword` at (`row`, `column`), wrapped where the
    /// walk leaves the matrix (`DefaultPlacement::module`).
    fn module(&mut self, row: isize, column: isize, codeword: u8, bit: u32) {
        // The side is at most 132, so every sum below is small.
        let side = isize::try_from(self.side).unwrap_or(isize::MAX);
        let (mut row, mut column) = (row, column);
        if row < 0 {
            row += side;
            column += 4 - (side + 4) % 8;
        }
        if column < 0 {
            column += side;
            row += 4 - (side + 4) % 8;
        }
        if let (Ok(row), Ok(column)) = (usize::try_from(row), usize::try_from(column)) {
            self.set(row, column, codeword & (1 << (8 - bit)) != 0);
        }
    }

    /// A codeword's eight bits in the "utah" shape whose last module is (`row`, `column`).
    fn utah(&mut self, row: isize, column: isize, codeword: u8) {
        let shape = [
            (-2, -2),
            (-2, -1),
            (-1, -2),
            (-1, -1),
            (-1, 0),
            (0, -2),
            (0, -1),
            (0, 0),
        ];
        for (bit, (down, across)) in (1..).zip(shape) {
            self.module(row + down, column + across, codeword, bit);
        }
    }

    /// The four corner shapes (`corner1` to `corner4`), each eight modules given as (row, column)
    /// with negative numbers counted from the far edge.
    fn corner(&mut self, shape: [(isize, isize); 8], codeword: u8) {
        let side = isize::try_from(self.side).unwrap_or(isize::MAX);
        for (bit, (row, column)) in (1..).zip(shape) {
            let row = if row < 0 { side + row } else { row };
            let column = if column < 0 { side + column } else { column };
            self.module(row, column, codeword, bit);
        }
    }
}

const CORNER_1: [(isize, isize); 8] = [
    (-1, 0),
    (-1, 1),
    (-1, 2),
    (0, -2),
    (0, -1),
    (1, -1),
    (2, -1),
    (3, -1),
];
const CORNER_2: [(isize, isize); 8] = [
    (-3, 0),
    (-2, 0),
    (-1, 0),
    (0, -4),
    (0, -3),
    (0, -2),
    (0, -1),
    (1, -1),
];
const CORNER_3: [(isize, isize); 8] = [
    (-3, 0),
    (-2, 0),
    (-1, 0),
    (0, -2),
    (0, -1),
    (1, -1),
    (2, -1),
    (3, -1),
];
const CORNER_4: [(isize, isize); 8] = [
    (-1, 0),
    (-1, -1),
    (0, -3),
    (0, -2),
    (0, -1),
    (1, -3),
    (1, -2),
    (1, -1),
];

/// Place the codewords into a square mapping matrix of `side` modules (`DefaultPlacement::place`):
/// diagonal sweeps up and to the right, then down and to the left, each codeword in the utah
/// shape, the four corner shapes where a sweep begins at a corner of the matrix, and the fixed
/// pattern in the lower right corner where it is left unfilled.
fn place(codewords: &[u8], side: usize) -> Vec<bool> {
    let mut mapping = Mapping {
        side,
        modules: vec![None; side * side],
    };
    let n = isize::try_from(side).unwrap_or(isize::MAX);
    let mut next = codewords.iter().copied();
    let (mut row, mut column) = (4_isize, 0_isize);
    loop {
        if row == n && column == 0 {
            mapping.corner(CORNER_1, next.next().unwrap_or(0));
        }
        if row == n - 2 && column == 0 && !side.is_multiple_of(4) {
            mapping.corner(CORNER_2, next.next().unwrap_or(0));
        }
        if row == n - 2 && column == 0 && side % 8 == 4 {
            mapping.corner(CORNER_3, next.next().unwrap_or(0));
        }
        if row == n + 4 && column == 2 && side.is_multiple_of(8) {
            mapping.corner(CORNER_4, next.next().unwrap_or(0));
        }
        loop {
            if row < n && column >= 0 && unset(&mapping, row, column) {
                mapping.utah(row, column, next.next().unwrap_or(0));
            }
            row -= 2;
            column += 2;
            if !(row >= 0 && column < n) {
                break;
            }
        }
        row += 1;
        column += 3;
        loop {
            if row >= 0 && column < n && unset(&mapping, row, column) {
                mapping.utah(row, column, next.next().unwrap_or(0));
            }
            row += 2;
            column -= 2;
            if !(row < n && column >= 0) {
                break;
            }
        }
        row += 3;
        column += 1;
        if !(row < n || column < n) {
            break;
        }
    }
    if mapping.at(side - 1, side - 1).is_none() {
        mapping.set(side - 1, side - 1, true);
        mapping.set(side - 2, side - 2, true);
    }
    mapping
        .modules
        .into_iter()
        .map(|module| module == Some(true))
        .collect()
}

fn unset(mapping: &Mapping, row: isize, column: isize) -> bool {
    match (usize::try_from(row), usize::try_from(column)) {
        (Ok(row), Ok(column)) => mapping.at(row, column).is_none(),
        _ => false,
    }
}

/// The symbol: the mapping matrix cut into data regions, each framed by its finder pattern — a
/// solid left column and bottom row, a top row dark at even columns and a right column dark at the
/// region's odd rows (`DataMatrixWriter::encodeLowLevel`).
fn frame(mapping: &[bool], size: Size) -> Matrix {
    let region = size.region();
    let framed = region + 2;
    let width = size.side;
    let inner = size.mapping();
    let mut dark = vec![false; width * width];
    for (index, module) in dark.iter_mut().enumerate() {
        let (row, column) = (index / width, index % width);
        let (local_row, local_column) = (row % framed, column % framed);
        *module = if local_row == 0 {
            column % 2 == 0
        } else if local_row == framed - 1 || local_column == 0 {
            true
        } else if local_column == framed - 1 {
            local_row % 2 == 1
        } else {
            let data_row = row / framed * region + local_row - 1;
            let data_column = column / framed * region + local_column - 1;
            mapping
                .get(data_row * inner + data_column)
                .copied()
                .unwrap_or(false)
        };
    }
    Matrix {
        width,
        height: width,
        dark,
    }
}

/// The symbol for data codewords already chosen, padded to `size`.
fn symbol(mut data: Vec<u8>, size: Size) -> Matrix {
    pad(&mut data, size.data);
    let all = with_error_correction(&data, size);
    frame(&place(&all, size.mapping()), size)
}

/// Encode bytes in one named encodation, in the smallest size that holds them.
///
/// # Errors
///
/// [`Refusal::Unencodable`] for a byte the encodation does not hold, and [`Refusal::Capacity`] for
/// a message the largest size, 144 by 144, cannot hold.
pub fn encode_in(data: &[u8], encodation: Encodation) -> Result<(Matrix, Size), Refusal> {
    let stream = codewords(data, encodation)?;
    let size = smallest(stream.needs).ok_or(Refusal::Capacity {
        symbology: data_matrix(),
        codewords: stream.needs,
    })?;
    Ok((symbol(stream.codewords, size), size))
}

/// Encode bytes as the smallest symbol any one encodation reaches, preferring the earlier of
/// [`ENCODATIONS`] on a tie.
///
/// # Errors
///
/// [`Refusal::Capacity`] for a message no encodation fits into 144 by 144.
pub fn encode(data: &[u8]) -> Result<Matrix, Refusal> {
    let mut best: Option<Codewords> = None;
    for encodation in ENCODATIONS {
        if let Ok(stream) = codewords(data, encodation)
            && best.as_ref().is_none_or(|held| stream.needs < held.needs)
        {
            best = Some(stream);
        }
    }
    // ASCII holds every byte, so `best` is set; its absence would be a refusal all the same.
    let stream = best.ok_or(Refusal::Capacity {
        symbology: data_matrix(),
        codewords: data.len(),
    })?;
    let size = smallest(stream.needs).ok_or(Refusal::Capacity {
        symbology: data_matrix(),
        codewords: stream.needs,
    })?;
    Ok(symbol(stream.codewords, size))
}
