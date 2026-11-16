//! PDF417, XFA 3.3's `pdf417`, from ISO/IEC 15438:2006.
//!
//! # The source
//!
//! The template reference (XFA 3.3 page 618) defines `pdf417` by its standard, ISO/IEC 15438. The
//! text read is **ISO/IEC 15438:2006** as Public.Resource.Org publishes it, incorporated by
//! reference into US federal regulation; the owner's answer A857 has this symbology built from that
//! copy, kept outside the tree, its tables carried as data (`doc/third-party-data.md`) and its text
//! cited by clause and paraphrased, never quoted. Clauses below are that standard's.
//!
//! # What XFA 3.3 decides
//!
//! - **The data is bytes**, serialised by the caller (page 424), as for every two-dimensional type.
//! - **`errorCorrectionLevel`**, 0 to 8 (page 614); with it the `checksum` property is ignored.
//!   Absent, the level is Annex E's recommended minimum for the number of data codewords
//!   (ISO/IEC 15438 Table E.1), the standard's own advice — XFA 3.3 states no default.
//! - **`dataRowCount` and `dataColumnCount`** fix the matrix, padded where the data does not fill
//!   it (pages 613–614); **`rowColumnRatio`** lets the rows grow at a ratio (page 615); and absent
//!   all three the processor picks legal counts (page 438). [`Shape`] is the three, and
//!   [`Shape::Square`] the last: the matrix whose rows at clause 5.8.2's least row height, three
//!   modules, come nearest to the row's width — what a square field asks for until the layout,
//!   which knows the field, says otherwise.
//! - **`truncate`** asks for no right-hand synchronisation mark (page 439): Compact PDF417, Annex G.
//!
//! # The construction
//!
//! The data is cut into segments and each compacted (clause 5.4): a run of 13 digits or more in
//! Numeric Compaction (clause 5.4.4, Annex D), a run of five or more characters Text Compaction holds
//! in Text Compaction (clause 5.4.2, Table 5), and the rest in Byte Compaction (clause 5.4.3, Annex
//! C). Which compaction carries which byte is the encoder's choice — the decoder reads every
//! choice the same — and the thresholds are the ones Annex N's informative algorithm uses; the
//! annex's further look-ahead is not built. The Symbol Length Descriptor and the pads (clause 5.9.2)
//! complete the data codewords, the error correction codewords follow (clause 5.10), and each row
//! is drawn from Annex A in its cluster between its row indicators (clause 5.11).

#![expect(
    clippy::arithmetic_side_effects,
    reason = "every quantity is bounded before it is computed: [`codewords`] refuses more than \
              `MOST_BYTES` bytes, a matrix is at most 90 rows, 30 columns and 928 codewords, a \
              codeword is below 929 and products of two are reduced modulo 929 in `u32`, and six \
              bytes are below 2⁴⁸ in `u64`; the `xfa_barcode` fuzz target holds the claim (ADR 0096)"
)]

mod annex_a;

pub use annex_a::ANNEX_A;

use crate::{Matrix, Refusal, Symbology};

/// The codeword that latches to Text Compaction (clause 5.4.1, Table 4).
pub const TEXT_LATCH: u16 = 900;
/// The codeword that latches to Byte Compaction for a count of bytes not a multiple of six.
pub const BYTE_LATCH: u16 = 901;
/// The codeword that latches to Numeric Compaction.
pub const NUMERIC_LATCH: u16 = 902;
/// The codeword that latches to Byte Compaction for a multiple of six bytes.
pub const BYTE_LATCH_SIX: u16 = 924;
/// The pad codeword, by convention the Text Compaction latch (clause 5.9.2).
pub const PAD: u16 = 900;

/// Most symbol characters a symbol holds, clause 5.2.3.
const MOST_CODEWORDS: usize = 928;
/// The most bytes any compaction fits into 928 codewords: Numeric's 44 digits in 15, under three a
/// codeword.
const MOST_BYTES: usize = 3 * MOST_CODEWORDS;
/// Least and most rows, clause 5.2.1.
const ROWS: std::ops::RangeInclusive<usize> = 3..=90;
/// Least and most data columns, clause 5.2.1.
const COLUMNS: std::ops::RangeInclusive<usize> = 1..=30;
/// The start character's bar-space sequence, Table 1.
const START: [u8; 8] = [8, 1, 1, 1, 1, 1, 1, 3];
/// The stop character's, Table 1: one more bar than a symbol character.
const STOP: [u8; 9] = [7, 1, 1, 3, 1, 1, 1, 2, 1];
/// Rows of modules each symbol row is drawn as: clause 5.8.2's least row height, `Y ≥ 3X`.
pub const ROW_HEIGHT: usize = 3;

fn pdf417() -> Symbology {
    Symbology("pdf417")
}

/// The four Text Compaction sub-modes, clause 5.4.2.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SubMode {
    Alpha,
    Lower,
    Mixed,
    Punctuation,
}

/// Table 5's Mixed column, values 0 to 24 (25 is `pl`, 26 the space, 27 to 29 latches and a
/// shift): the digits, `&`, CR, HT, `,`, `:`, `#`, `-`, `.`, `$`, `/`, `+`, `%`, `*`, `=` and `^`.
const MIXED: &[u8; 25] = b"0123456789&\r\t,:#-.$/+%*=^";

/// Table 5's Punctuation column, values 0 to 28 (29 is `al`).
const PUNCTUATION: &[u8; 29] = b";<>@[\\]_`~!\r\t,:\n-.$/\"|*()?{}'";

/// Table 5's latch and shift values.
const LATCH_LOWER: u8 = 27;
const LATCH_MIXED: u8 = 28;
const SHIFT_PUNCTUATION: u8 = 29;
const SHIFT_ALPHA: u8 = 27;
const LATCH_PUNCTUATION: u8 = 25;
const LATCH_ALPHA_FROM_MIXED: u8 = 28;
const LATCH_ALPHA_FROM_PUNCTUATION: u8 = 29;
const SPACE: u8 = 26;

/// A byte's value in one sub-mode of Table 5, if the sub-mode holds it.
fn value_in(sub_mode: SubMode, byte: u8) -> Option<u8> {
    let position = |table: &[u8]| {
        table
            .iter()
            .position(|held| *held == byte)
            .and_then(|at| u8::try_from(at).ok())
    };
    match sub_mode {
        SubMode::Alpha if byte.is_ascii_uppercase() => Some(byte - b'A'),
        SubMode::Lower if byte.is_ascii_lowercase() => Some(byte - b'a'),
        SubMode::Alpha | SubMode::Lower | SubMode::Mixed if byte == b' ' => Some(SPACE),
        SubMode::Alpha | SubMode::Lower => None,
        SubMode::Mixed => position(MIXED),
        SubMode::Punctuation => position(PUNCTUATION),
    }
}

/// Whether Text Compaction holds the byte at all: clause 5.4.2's printable ASCII, tab, line feed
/// and carriage return.
fn is_text(byte: u8) -> bool {
    matches!(byte, b'\t' | b'\n' | b'\r' | b' '..=b'~')
}

/// Text Compaction (clause 5.4.2): base-30 values, sub-modes switched by Table 7's latches and
/// shifts, two values to a codeword (`30 h + l`, clause 5.4.2.2), an odd count padded with `ps`
/// or, in Punctuation, `al` (clause 5.4.2.4 case a). A lower-case letter is reached by a latch, an
/// upper-case one from Lower by `as` when it stands alone and by `ml al` when a run follows, and a
/// punctuation character by `ps`, or by `pl` from Mixed when a run follows. Every segment starts
/// in Alpha: the symbol begins there and a latch to Text resets to it (clause 5.4.2.1).
fn text(segment: &[u8], into: &mut Vec<u16>) {
    let mut values: Vec<u8> = Vec::with_capacity(segment.len() * 2);
    let mut sub_mode = SubMode::Alpha;
    let mut at = 0;
    while let Some(&byte) = segment.get(at) {
        if let Some(value) = value_in(sub_mode, byte) {
            values.push(value);
            at += 1;
            continue;
        }
        let next = segment.get(at + 1).copied();
        let run_of = |wanted: SubMode| next.is_some_and(|next| value_in(wanted, next).is_some());
        match sub_mode {
            SubMode::Punctuation => {
                values.push(LATCH_ALPHA_FROM_PUNCTUATION);
                sub_mode = SubMode::Alpha;
            }
            _ if value_in(SubMode::Lower, byte).is_some() => {
                values.push(LATCH_LOWER);
                sub_mode = SubMode::Lower;
            }
            SubMode::Lower if value_in(SubMode::Alpha, byte).is_some() => {
                if run_of(SubMode::Alpha) {
                    values.extend([LATCH_MIXED, LATCH_ALPHA_FROM_MIXED]);
                    sub_mode = SubMode::Alpha;
                } else {
                    values.extend([SHIFT_ALPHA, byte - b'A']);
                    at += 1;
                }
            }
            SubMode::Mixed if value_in(SubMode::Alpha, byte).is_some() => {
                values.push(LATCH_ALPHA_FROM_MIXED);
                sub_mode = SubMode::Alpha;
            }
            _ if value_in(SubMode::Mixed, byte).is_some() => {
                values.push(LATCH_MIXED);
                sub_mode = SubMode::Mixed;
            }
            SubMode::Mixed if run_of(SubMode::Punctuation) => {
                values.push(LATCH_PUNCTUATION);
                sub_mode = SubMode::Punctuation;
            }
            _ => {
                // Only Punctuation holds it, and Alpha, Lower and Mixed each shift there by `ps`.
                values.push(SHIFT_PUNCTUATION);
                values.extend(value_in(SubMode::Punctuation, byte));
                at += 1;
            }
        }
    }
    if values.len() % 2 == 1 {
        values.push(SHIFT_PUNCTUATION);
    }
    for pair in values.chunks(2) {
        if let [high, low] = pair {
            into.push(u16::from(*high) * 30 + u16::from(*low));
        }
    }
}

/// Byte Compaction (clause 5.4.3.3): the latch 924 for a multiple of six bytes and 901 otherwise,
/// each six bytes as five codewords by Annex C's base-256 to base-900 conversion, and the bytes
/// left over one to a codeword.
fn bytes(segment: &[u8], into: &mut Vec<u16>) {
    into.push(if segment.len().is_multiple_of(6) {
        BYTE_LATCH_SIX
    } else {
        BYTE_LATCH
    });
    let mut groups = segment.chunks_exact(6);
    for group in groups.by_ref() {
        let mut value = group
            .iter()
            .fold(0_u64, |sum, byte| sum * 256 + u64::from(*byte));
        let mut codewords = [0_u16; 5];
        for slot in codewords.iter_mut().rev() {
            // `value % 900` is below 900.
            *slot = u16::try_from(value % 900).unwrap_or(0);
            value /= 900;
        }
        into.extend(codewords);
    }
    into.extend(groups.remainder().iter().map(|byte| u16::from(*byte)));
}

/// Numeric Compaction (clause 5.4.4.2): the latch, then each group of up to 44 digits with a 1
/// before it converted from base 10 to base 900 (Annex D), most significant codeword first. The
/// conversion is long division of the decimal digits by 900, repeated until nothing is left.
fn numeric(segment: &[u8], into: &mut Vec<u16>) {
    into.push(NUMERIC_LATCH);
    for group in segment.chunks(44) {
        let mut digits: Vec<u16> = std::iter::once(1)
            .chain(group.iter().map(|digit| u16::from(digit - b'0')))
            .collect();
        let mut codewords = Vec::new();
        while !digits.is_empty() {
            let mut quotient = Vec::with_capacity(digits.len());
            let mut remainder: u32 = 0;
            for digit in &digits {
                let current = remainder * 10 + u32::from(*digit);
                let next = current / 900;
                remainder = current % 900;
                if !(quotient.is_empty() && next == 0) {
                    // `current` is below 9000, so `next` is a decimal digit.
                    quotient.push(u16::try_from(next).unwrap_or(0));
                }
            }
            codewords.push(u16::try_from(remainder).unwrap_or(0));
            digits = quotient;
        }
        into.extend(codewords.into_iter().rev());
    }
}

/// The segments of clause 5.4's choice, as described in the module comment.
fn segments(data: &[u8]) -> Vec<(Compaction, std::ops::Range<usize>)> {
    let run = |from: usize, held: fn(u8) -> bool| {
        data.get(from..).map_or(0, |rest| {
            rest.iter().take_while(|byte| held(**byte)).count()
        })
    };
    let mut segments: Vec<(Compaction, std::ops::Range<usize>)> = Vec::new();
    let mut at = 0;
    while at < data.len() {
        let digits = run(at, |byte| byte.is_ascii_digit());
        let (compaction, length) = if digits >= 13 {
            (Compaction::Numeric, digits)
        } else {
            let text_run = run(at, is_text);
            if text_run >= 5 || (text_run > 0 && text_run == data.len() - at) {
                // A text run stops where a run of 13 digits begins.
                let mut end = at;
                while end < at + text_run && run(end, |byte| byte.is_ascii_digit()) < 13 {
                    end += 1;
                }
                (Compaction::Text, end - at)
            } else {
                // Bytes run until a run Text or Numeric would take.
                let mut end = at + text_run.max(1);
                while end < data.len()
                    && run(end, is_text) < 5
                    && run(end, |byte| byte.is_ascii_digit()) < 13
                {
                    end += 1;
                }
                (Compaction::Byte, end - at)
            }
        };
        match segments.last_mut() {
            Some((last, range)) if *last == compaction => range.end = at + length,
            _ => segments.push((compaction, at..at + length)),
        }
        at += length;
    }
    segments
}

/// The three compaction modes, clause 5.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compaction {
    /// Text Compaction, clause 5.4.2.
    Text,
    /// Byte Compaction, clause 5.4.3.
    Byte,
    /// Numeric Compaction, clause 5.4.4.
    Numeric,
}

/// The source data codewords for `data` (clause 5.9.2's `m` of them), every segment in its
/// compaction: the symbol starts in Text Compaction, so a leading text segment needs no latch.
#[must_use]
pub fn data_codewords(data: &[u8]) -> Vec<u16> {
    let mut codewords = Vec::new();
    for (index, (compaction, range)) in segments(data).into_iter().enumerate() {
        let segment = data.get(range).unwrap_or_default();
        match compaction {
            Compaction::Text => {
                if index > 0 {
                    codewords.push(TEXT_LATCH);
                }
                text(segment, &mut codewords);
            }
            Compaction::Byte => bytes(segment, &mut codewords),
            Compaction::Numeric => numeric(segment, &mut codewords),
        }
    }
    codewords
}

/// The data codewords of one compaction alone, for a caller (or a test) that wants a named mode.
#[must_use]
pub fn compacted(data: &[u8], compaction: Compaction) -> Vec<u16> {
    let mut codewords = Vec::new();
    match compaction {
        Compaction::Text => text(data, &mut codewords),
        Compaction::Byte => bytes(data, &mut codewords),
        Compaction::Numeric => numeric(data, &mut codewords),
    }
    codewords
}

/// The generator polynomial's coefficients `α₀ … α_{k−1}` for `k` error correction codewords:
/// `(x − 3)(x − 3²)…(x − 3ᵏ)` modulo 929 (clause 5.10), the leading `xᵏ` omitted. Annex F prints
/// these for each level; they are computed here from the clause's product.
#[must_use]
pub fn coefficients(count: usize) -> Vec<u16> {
    // Lowest power first: `polynomial[j]` is the coefficient of `xʲ`.
    let mut polynomial: Vec<u32> = vec![1];
    let mut root: u32 = 1;
    for _ in 0..count {
        root = root * 3 % 929;
        let negated = 929 - root;
        let mut next = vec![0_u32; polynomial.len() + 1];
        for (power, coefficient) in polynomial.iter().enumerate() {
            if let Some(slot) = next.get_mut(power + 1) {
                *slot = (*slot + coefficient) % 929;
            }
            if let Some(slot) = next.get_mut(power) {
                *slot = (*slot + coefficient * negated) % 929;
            }
        }
        polynomial = next;
    }
    polynomial.pop();
    polynomial
        .into_iter()
        .map(|coefficient| u16::try_from(coefficient).unwrap_or(0))
        .collect()
}

/// The error correction codewords of the data codewords (Symbol Length Descriptor and pads
/// included), clause 5.10's algorithm step by step, returned `E_{k−1}` first — the order they are
/// placed in (Figure 8).
#[must_use]
pub fn error_codewords(data: &[u16], count: usize) -> Vec<u16> {
    let alpha: Vec<u32> = coefficients(count).into_iter().map(u32::from).collect();
    // `e[j]` is `E_j`.
    let mut e = vec![0_u32; count];
    for codeword in data {
        let t1 = (u32::from(*codeword) + e.last().copied().unwrap_or(0)) % 929;
        for j in (1..count).rev() {
            let t3 = 929 - t1 * alpha.get(j).copied().unwrap_or(0) % 929;
            let below = e.get(j - 1).copied().unwrap_or(0);
            if let Some(slot) = e.get_mut(j) {
                *slot = (below + t3) % 929;
            }
        }
        let t3 = 929 - t1 * alpha.first().copied().unwrap_or(0) % 929;
        if let Some(slot) = e.first_mut() {
            *slot = t3 % 929;
        }
    }
    e.into_iter()
        .rev()
        .map(|value| u16::try_from(if value == 0 { 0 } else { 929 - value }).unwrap_or(0))
        .collect()
}

/// ISO/IEC 15438 Table E.1 (Annex E): the recommended least error correction level for a count of data codewords.
#[must_use]
pub fn recommended_level(data_codewords: usize) -> u8 {
    match data_codewords {
        0..=40 => 2,
        41..=160 => 3,
        161..=320 => 4,
        _ => 5,
    }
}

/// How the matrix of rows and columns is chosen (XFA 3.3 pages 613–615 and 438).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// `dataRowCount` and `dataColumnCount`: exactly these, padded.
    Fixed {
        /// Rows, 3 to 90.
        rows: usize,
        /// Data columns, 1 to 30.
        columns: usize,
    },
    /// `rowColumnRatio`: the column count whose least row count comes nearest the ratio of rows
    /// to columns.
    Ratio(f64),
    /// A column count a layout chose from the field's width; the rows grow to hold the data.
    Columns(usize),
    /// None of the three: the matrix whose height at three modules a row comes nearest its width.
    Square,
}

/// What the template asks of a PDF417 symbol.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    /// `errorCorrectionLevel`, 0 to 8; `None` for Annex E's recommendation.
    pub level: Option<u8>,
    /// The matrix.
    pub shape: Shape,
    /// `truncate`: Compact PDF417, Annex G.
    pub compact: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            level: None,
            shape: Shape::Square,
            compact: false,
        }
    }
}

/// The chosen matrix and level for a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    /// Rows, `r`.
    pub rows: usize,
    /// Data columns, `c`.
    pub columns: usize,
    /// Error correction level, `s`.
    pub level: u8,
}

impl Layout {
    /// Error correction codewords, `k = 2^(s+1)` (Table 9).
    #[must_use]
    pub fn error_codewords(self) -> usize {
        2_usize << self.level
    }

    /// Data codewords the matrix holds, `n = c r − k` (clause 5.9.2).
    #[must_use]
    pub fn data_capacity(self) -> usize {
        (self.columns * self.rows).saturating_sub(self.error_codewords())
    }
}

/// The least rows that hold `needed` codewords in `columns`, if within clause 5.2.1's limits.
fn rows_for(needed: usize, columns: usize) -> Option<usize> {
    let rows = needed.div_ceil(columns).max(*ROWS.start());
    (ROWS.contains(&rows) && rows * columns <= MOST_CODEWORDS).then_some(rows)
}

/// Choose the matrix (clause 5.9.2): `needed` is the data codewords with the Symbol Length
/// Descriptor plus the error correction codewords.
fn choose(shape: Shape, needed: usize, level: u8) -> Option<Layout> {
    let layout = |rows, columns| Layout {
        rows,
        columns,
        level,
    };
    match shape {
        Shape::Fixed { rows, columns } => {
            let fits = ROWS.contains(&rows)
                && COLUMNS.contains(&columns)
                && rows * columns <= MOST_CODEWORDS
                && rows * columns >= needed;
            fits.then(|| layout(rows, columns))
        }
        Shape::Columns(columns) => COLUMNS
            .contains(&columns)
            .then(|| rows_for(needed, columns))
            .flatten()
            .map(|rows| layout(rows, columns)),
        Shape::Ratio(_) | Shape::Square => {
            let distance = |rows: usize, columns: usize| -> f64 {
                // Both are at most 90, so each is exactly a `u16` and an `f64`.
                let as_float = |count: usize| f64::from(u16::try_from(count).unwrap_or(u16::MAX));
                let (rows_f, columns_f) = (as_float(rows), as_float(columns));
                match shape {
                    Shape::Ratio(ratio) => (rows_f / columns_f - ratio).abs(),
                    // A row is 17 c + 69 modules wide (start, two indicators, stop) and drawn
                    // three modules high.
                    _ => (rows_f * 3.0 - (columns_f * 17.0 + 69.0)).abs(),
                }
            };
            COLUMNS
                .filter_map(|columns| rows_for(needed, columns).map(|rows| (rows, columns)))
                .min_by(|left, right| {
                    distance(left.0, left.1).total_cmp(&distance(right.0, right.1))
                })
                .map(|(rows, columns)| layout(rows, columns))
        }
    }
}

/// The row indicators of row `row` (1-based), clause 5.11.3.
fn indicators(row: usize, layout: Layout) -> (usize, usize) {
    let base = 30 * ((row - 1) / 3);
    let rows_part = (layout.rows - 1) / 3;
    let level_part = usize::from(layout.level) * 3 + (layout.rows - 1) % 3;
    let columns_part = layout.columns - 1;
    match (row - 1) % 3 {
        0 => (base + rows_part, base + columns_part),
        1 => (base + level_part, base + rows_part),
        _ => (base + columns_part, base + level_part),
    }
}

/// Every codeword of the symbol in placement order: the Symbol Length Descriptor, the source data
/// codewords, the pads, and the error correction codewords (clause 5.2.3, clause 5.9.2).
///
/// # Errors
///
/// [`Refusal::Capacity`] for data no matrix within clause 5.2's limits holds at the level, or a
/// level past 8.
pub fn codewords(data: &[u8], options: Options) -> Result<(Vec<u16>, Layout), Refusal> {
    let capacity = |codewords| Refusal::Capacity {
        symbology: pdf417(),
        codewords,
    };
    if data.len() > MOST_BYTES {
        return Err(capacity(data.len().div_ceil(3)));
    }
    let source = data_codewords(data);
    let with_length = source.len() + 1;
    let level = options
        .level
        .unwrap_or_else(|| recommended_level(with_length));
    if level > 8 {
        return Err(capacity(with_length));
    }
    let needed = with_length + (2_usize << level);
    let layout = choose(options.shape, needed, level).ok_or_else(|| capacity(needed))?;
    let length = layout.data_capacity();
    let mut all = Vec::with_capacity(layout.rows * layout.columns);
    // `length` is at most 928 by `rows_for` and `choose`.
    all.push(u16::try_from(length).unwrap_or(0));
    all.extend(source);
    all.resize(length, PAD);
    let error = error_codewords(&all, layout.error_codewords());
    all.extend(error);
    Ok((all, layout))
}

/// Append an element sequence's modules, bar first, to a row.
fn modules(widths: &[u8], into: &mut Vec<bool>) {
    for (index, width) in widths.iter().enumerate() {
        into.extend(std::iter::repeat_n(index % 2 == 0, usize::from(*width)));
    }
}

/// A codeword's symbol character in a cluster (Annex A), as its eight element widths.
fn character(codeword: usize, cluster: usize) -> [u8; 8] {
    let sequence = ANNEX_A
        .get(codeword)
        .and_then(|clusters| clusters.get(cluster))
        .copied()
        .unwrap_or(0);
    let mut widths = [0_u8; 8];
    let mut rest = sequence;
    for slot in widths.iter_mut().rev() {
        // `rest % 10` is one decimal digit.
        *slot = u8::try_from(rest % 10).unwrap_or(0);
        rest /= 10;
    }
    widths
}

/// Encode bytes as a PDF417 symbol, each symbol row [`ROW_HEIGHT`] module rows high.
///
/// # Errors
///
/// [`Refusal::Capacity`] as [`codewords`] says.
pub fn encode(data: &[u8], options: Options) -> Result<(Matrix, Layout), Refusal> {
    let (all, layout) = codewords(data, options)?;
    let width = if options.compact {
        17 * layout.columns + 35
    } else {
        17 * layout.columns + 69
    };
    let mut dark = Vec::with_capacity(width * layout.rows * ROW_HEIGHT);
    for (index, row) in all.chunks(layout.columns).enumerate() {
        let number = index + 1;
        // Clause 5.11.1: rows 1, 2, 3 use clusters 0, 3, 6, and on (the table's columns 0, 1, 2).
        let cluster = index % 3;
        let (left, right) = indicators(number, layout);
        let mut line = Vec::with_capacity(width);
        modules(&START, &mut line);
        modules(&character(left, cluster), &mut line);
        for codeword in row {
            modules(&character(usize::from(*codeword), cluster), &mut line);
        }
        if options.compact {
            // Annex G: no right row indicator, and the stop pattern one module of bar.
            line.push(true);
        } else {
            modules(&character(right, cluster), &mut line);
            modules(&STOP, &mut line);
        }
        for _ in 0..ROW_HEIGHT {
            dark.extend_from_slice(&line);
        }
    }
    Ok((
        Matrix {
            width,
            height: layout.rows * ROW_HEIGHT,
            dark,
        },
        layout,
    ))
}
