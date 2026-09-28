//! ITU-T T.4 and T.6 fax decoding, as ISO 32000-2 §7.4.6's `CCITTFaxDecode` filter asks for it.
//!
//! One responsibility: turn a Group 3 or Group 4 coded bit stream into scan lines. A scan line
//! leaves this crate as its *changing elements* — the positions where the colour differs from
//! the pel before it, T.4 section 4.2.1.3.1's own vocabulary — and the caller turns those into
//! samples. What a PDF makes of a sample (`/BlackIs1`, `/Decode`, the image grid) is not here.
//!
//! # Why this tree has its own fax decoder
//!
//! §7.4.6 Table 11's `/DamagedRowsBeforeError` asks the filter to *conceal* damaged rows —
//!
//! > Tolerating a damaged row shall mean locating its end in the encoded data by searching for an
//! > `EndOfLine` pattern and then substituting decoded data from the previous row if the previous
//! > row was not damaged, or a white scan line if the previous row was also damaged.
//!
//! — and that needs a decoder that can say *where* a row broke and resume after it. The shared
//! decoder the tree used before answers only "it broke", which is why ADR 1349 moved §7.4.6 onto
//! this crate. The concealment is built in rather than wrapped around, and it is the only error
//! handling there is: the same clause says "[t]he filter shall not perform any error correction
//! or resynchronization, except as noted for the `DamagedRowsBeforeError` parameter", so every
//! other kind of damage ends the decode where it is found, with the rows before it delivered.
//!
//! # Where the texts are cited
//!
//! T.4 and T.6 are cited by section and paraphrased (ADR 0187's discipline;
//! `doc/third-party-data.md` records the licence position); only ISO 32000-2 is quoted.

#![forbid(unsafe_code)]

mod bits;
mod codes;

use bits::Bits;
use codes::{BLACK, MODE_CODE_BITS, MODES, Mode, RUN_CODE_BITS, WHITE};

/// Which of the three codings the data uses — Table 11's `/K`, read by its sign.
///
/// §7.4.6: the filter "shall not distinguish between different positive K values", so the mixed
/// coding carries no `K`: each line's own tag bit says how it is coded (T.4 section 4.2.1.3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coding {
    /// `/K` negative: T.6's two-dimensional coding, no end-of-line codes.
    Group4,
    /// `/K` zero: T.4's one-dimensional coding (section 4.1).
    Group3OneDimensional,
    /// `/K` positive: T.4's mixed coding, each line tagged one- or two-dimensional (section 4.2).
    Group3Mixed,
}

impl Coding {
    /// Table 11's `/K`, by its sign.
    #[must_use]
    pub fn from_k(k: i64) -> Self {
        match k {
            ..0 => Self::Group4,
            0 => Self::Group3OneDimensional,
            _ => Self::Group3Mixed,
        }
    }

    /// How many consecutive end-of-line codes end the data: T.6 section 2.4.1.1's end-of-facsimile
    /// block is two, T.4 section 4.1.4's return to control six — "the CCITT end-of-facsimile-block
    /// (EOFB) or return-to-control (RTC) appropriate for the K parameter", as Table 11 puts it.
    fn end_of_block(self) -> u32 {
        match self {
            Self::Group4 => 2,
            Self::Group3OneDimensional | Self::Group3Mixed => 6,
        }
    }
}

/// What the data is and how far to decode it: Table 11's entries, resolved by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parameters {
    /// The coding, from `/K`.
    pub coding: Coding,
    /// Pels in a scan line — `/Columns`.
    pub columns: u32,
    /// The most scan lines to decode. The caller's resolution of `/Rows` against `/EndOfBlock`
    /// and the image's height; the decode may end earlier, on the end-of-block pattern or on the
    /// end of the data.
    pub rows: u32,
    /// `/EndOfLine`: every line is preceded by an end-of-line code.
    pub end_of_line: bool,
    /// `/EncodedByteAlign`: every line begins on a byte boundary.
    pub encoded_byte_align: bool,
    /// `/EndOfBlock`: an end-of-block pattern ends the data.
    pub end_of_block: bool,
    /// `/DamagedRowsBeforeError`: how many damaged rows to conceal before damage ends the decode.
    ///
    /// Honoured only where Table 11 lets it apply — "only if `EndOfLine` is true and K is
    /// non-negative", because the concealment finds a row's end by its end-of-line code and only
    /// Group 3 with `/EndOfLine` true is promised one. Elsewhere it is inert, and the first damaged
    /// row ends the decode as it would at zero.
    pub damaged_rows_before_error: u32,
}

impl Parameters {
    /// Whether `damaged_rows_before_error` applies to this data.
    fn conceals(self) -> bool {
        self.end_of_line && self.coding != Coding::Group4
    }
}

/// Where decoded scan lines go.
pub trait Rows {
    /// One scan line, as the positions of its changing elements in increasing order: the first
    /// is where the line turns black, the second where it turns white again, and so on. An empty
    /// slice is an all-white line. Every position is below [`Parameters::columns`].
    fn row(&mut self, changes: &[u32]);
}

/// What a decode delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Summary {
    /// Scan lines handed to [`Rows::row`], concealed ones included.
    pub rows: u32,
    /// How many of them were damaged rows concealed under `/DamagedRowsBeforeError`.
    pub concealed: u32,
}

/// What made a scan line damaged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Damage {
    /// Bits that are no codeword of the kind the line expected at that point.
    InvalidCode,
    /// The data ended inside a scan line.
    UnexpectedEnd,
    /// Runs that together pass the end of the scan line, or a two-dimensional changing element
    /// placed before the one it follows.
    OutsideTheLine,
    /// `/EndOfLine` is true and a complete line was not followed by an end-of-line code.
    MissingEndOfLine,
}

impl std::fmt::Display for Damage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidCode => "invalid CCITT code sequence",
            Self::UnexpectedEnd => "the data ends inside a scan line",
            Self::OutsideTheLine => "a run passes the end of its scan line",
            Self::MissingEndOfLine => "a scan line is not followed by an end-of-line code",
        })
    }
}

/// Why a decode ended before its bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// `/Columns` is zero, so there is no scan line to decode.
    NoColumns,
    /// A damaged row, beyond any `/DamagedRowsBeforeError` tolerates. The rows before it were
    /// delivered and are what `delivered` counts.
    Damaged {
        /// What was wrong with the row.
        damage: Damage,
        /// The damaged row, counted from zero — equal to `delivered.rows`.
        row: u32,
        /// The bit offset in the data at which the damage was found.
        bit: usize,
        /// What was delivered before it.
        delivered: Summary,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoColumns => f.write_str("/Columns is zero"),
            Self::Damaged {
                damage, row, bit, ..
            } => write!(f, "{damage} (scan line {row}, bit {bit})"),
        }
    }
}

impl std::error::Error for Error {}

/// Decodes `data` into scan lines, handing each to `rows`.
///
/// # Errors
///
/// [`Error::NoColumns`] for a zero `/Columns`, and [`Error::Damaged`] where a damaged row ends the
/// decode — the first one, or the first past what [`Parameters::damaged_rows_before_error`]
/// conceals. Either way every row before it has been handed to `rows`.
pub fn decode(
    data: &[u8],
    parameters: &Parameters,
    rows: &mut impl Rows,
) -> Result<Summary, Error> {
    if parameters.columns == 0 {
        return Err(Error::NoColumns);
    }
    Decoder {
        bits: Bits::new(data),
        parameters,
        reference: Vec::new(),
        coding: Vec::new(),
        summary: Summary::default(),
    }
    .run(rows)
}

/// One decode's state between scan lines.
struct Decoder<'a> {
    bits: Bits<'a>,
    parameters: &'a Parameters,
    /// The reference line's changing elements: the line above the one being decoded, as it was
    /// delivered. Empty before the first line — T.4 section 4.2.1.3.1 and T.6 section 2.2.1 give
    /// the first coding line an imaginary white line to refer to.
    reference: Vec<u32>,
    /// The coding line's changing elements, as decoded so far.
    coding: Vec<u32>,
    summary: Summary,
}

impl Decoder<'_> {
    fn run(mut self, rows: &mut impl Rows) -> Result<Summary, Error> {
        let parameters = *self.parameters;
        // Whether the row before this one was concealed, which decides what the next concealed
        // row is filled with.
        let mut previous_damaged = false;
        while self.summary.rows < parameters.rows {
            if parameters.encoded_byte_align {
                self.bits.align();
            }
            let ends = self.end_of_lines();
            if parameters.end_of_block && ends >= parameters.coding.end_of_block() {
                break;
            }
            // §7.4.6 Table 11's `/Rows`: without a count to reach, "the encoded data shall be
            // terminated by an end-of-block bit pattern or by the end of the filter's data".
            if self.bits.exhausted() {
                break;
            }
            let (start, line) = self.line();
            let line = line.and_then(|()| self.followed_by_end_of_line());
            match line {
                Ok(()) => previous_damaged = false,
                Err(damage) => {
                    let tolerated = parameters.conceals()
                        && self.summary.concealed < parameters.damaged_rows_before_error;
                    if !tolerated {
                        return Err(Error::Damaged {
                            damage,
                            row: self.summary.rows,
                            bit: self.bits.position(),
                            delivered: self.summary,
                        });
                    }
                    self.conceal(start, previous_damaged);
                    previous_damaged = true;
                    self.summary.concealed = self.summary.concealed.saturating_add(1);
                }
            }
            rows.row(&self.coding);
            std::mem::swap(&mut self.reference, &mut self.coding);
            self.summary.rows = self.summary.rows.saturating_add(1);
        }
        Ok(self.summary)
    }

    /// Reads one scan line into `self.coding`, returning the bit offset its coded data begins at
    /// (after any tag bit) beside the outcome.
    fn line(&mut self) -> (usize, Result<(), Damage>) {
        let parameters = self.parameters;
        let two_dimensional = match parameters.coding {
            Coding::Group4 => true,
            Coding::Group3OneDimensional => false,
            // T.4 section 4.2.1.3.4: the bit after the end-of-line code is 1 for a line coded
            // one-dimensionally and 0 for one coded two-dimensionally.
            Coding::Group3Mixed => match self.bits.bit() {
                Some(one) => !one,
                None => return (self.bits.position(), Err(Damage::UnexpectedEnd)),
            },
        };
        let start = self.bits.position();
        let outcome = if two_dimensional {
            two_dimensional_line(
                &mut self.bits,
                parameters.columns,
                &self.reference,
                &mut self.coding,
            )
        } else {
            one_dimensional_line(&mut self.bits, parameters.columns, &mut self.coding)
        };
        (start, outcome)
    }

    /// Consumes the end-of-line codes (with their fill) before a line, and says how many there
    /// were.
    ///
    /// §7.4.6 Table 11: "The `CCITTFaxDecode` filter shall always accept end- of-line bit patterns"
    /// — so they are read wherever they stand at a line's start, whatever `/EndOfLine` says. In
    /// the mixed coding each end-of-line code carries its tag bit, and T.4 section 4.1.4 makes the
    /// return to control six of those pairs; a tag bit followed by another end-of-line code is
    /// therefore one of the six rather than the start of a line (no line is empty).
    fn end_of_lines(&mut self) -> u32 {
        let mut count = 0_u32;
        loop {
            let zeros = self.bits.zeros();
            if zeros < 11 || zeros >= self.bits.remaining() {
                return count;
            }
            self.bits
                .skip(u32::try_from(zeros).unwrap_or(u32::MAX).saturating_add(1));
            count = count.saturating_add(1);
            if self.parameters.coding == Coding::Group3Mixed {
                let mut after_tag = self.bits.clone();
                after_tag.skip(1);
                let zeros = after_tag.zeros();
                if zeros < 11 || zeros >= after_tag.remaining() {
                    return count;
                }
                self.bits.skip(1);
            }
        }
    }

    /// Checks that a line ends where an end-of-line code begins, where the concealment is on.
    ///
    /// The concealment is the one place the filter is told a row *is* the bits between two
    /// end-of-line codes: its precondition is `/EndOfLine` true, and it finds a damaged row's end
    /// "by searching for an `EndOfLine` pattern". A line that decodes to its full width and is
    /// followed by anything else has been misread — its codewords fitted the width by chance — and
    /// treating it as whole would put the rest of it at the start of the next row, so under the
    /// concealment it is a damaged row. Without it, a missing code is the producer's breach of
    /// Table 11's "If `EndOfLine` is true end-of-line bit patterns shall be present", which the
    /// filter "shall always accept" end-of-line patterns around and is not asked to police: the
    /// next line is decoded where the last one ended (ADR 1349). The last line the bound allows,
    /// and a line followed only by the padding of the last byte, need no code after them.
    fn followed_by_end_of_line(&self) -> Result<(), Damage> {
        let parameters = self.parameters;
        let last = self.summary.rows.saturating_add(1) >= parameters.rows;
        if !parameters.conceals() || parameters.damaged_rows_before_error == 0 || last {
            return Ok(());
        }
        let mut next = self.bits.clone();
        if parameters.encoded_byte_align {
            next.align();
        }
        if next.exhausted() || next.zeros() >= 11 {
            Ok(())
        } else {
            Err(Damage::MissingEndOfLine)
        }
    }

    /// Conceals the damaged row whose coded data began at `start`, as §7.4.6 Table 11 states:
    /// its end is the next end-of-line code, and its content is the previous row's, or white
    /// where that row was concealed too.
    ///
    /// Before the first row there is no previous row to copy, and the line T.4 section 4.2.1.3.1
    /// gives the first row to refer to is white, so a damaged first row is white (ADR 1349). Where
    /// the data holds no further end-of-line code the row's end is the end of the data, and the
    /// decode ends there on the next line's check.
    fn conceal(&mut self, start: usize, previous_damaged: bool) {
        let end = self
            .bits
            .next_end_of_line(start)
            .unwrap_or_else(|| self.bits.position().saturating_add(self.bits.remaining()));
        // Possibly behind where decoding stopped: the damaged row's codewords may have been read
        // into the zeros that begin the end-of-line code, which is why the search starts from the
        // row's own first bit.
        self.bits.seek(end);
        self.coding.clear();
        if !previous_damaged {
            self.coding.extend_from_slice(&self.reference);
        }
    }
}

/// Appends a run of `length` pels of one colour to a coding line that has reached `at`.
///
/// A changing element is recorded where the run's colour differs from the colour the line has at
/// that point — which is black exactly when an odd number of changes precede it. An empty run
/// records nothing.
fn run(line: &mut Vec<u32>, at: u32, white: bool, length: u32) {
    let line_is_white = line.len().is_multiple_of(2);
    if length > 0 && white != line_is_white {
        line.push(at);
    }
}

/// Reads one run length of the given colour: any number of make-up codes and the terminating
/// code after them (T.4 section 4.1.1; T.6 section 2.2.4 uses the same codes).
///
/// `room` is how many pels the line has left, and a run longer than that is damage — the filter
/// may not shorten it to fit, §7.4.6 forbidding it "any error correction".
fn run_length(bits: &mut Bits<'_>, white: bool, room: u32) -> Result<u32, Damage> {
    let table = if white { &WHITE } else { &BLACK };
    let mut total = 0_u32;
    loop {
        let index = usize::try_from(bits.peek(RUN_CODE_BITS)).unwrap_or(0);
        let code = table
            .get(index)
            .copied()
            .unwrap_or(codes::RunCode { bits: 0, run: 0 });
        if code.bits == 0 {
            return Err(
                if bits.remaining() < RUN_CODE_BITS as usize && bits.only_zeros_remain() {
                    Damage::UnexpectedEnd
                } else {
                    Damage::InvalidCode
                },
            );
        }
        if usize::from(code.bits) > bits.remaining() {
            return Err(Damage::UnexpectedEnd);
        }
        bits.skip(u32::from(code.bits));
        total = total.saturating_add(u32::from(code.run));
        if total > room {
            return Err(Damage::OutsideTheLine);
        }
        if code.run < 64 {
            return Ok(total);
        }
    }
}

/// Decodes a one-dimensionally coded line: runs alternating white and black, white first, until
/// they fill the line (T.4 section 4.1.1).
fn one_dimensional_line(
    bits: &mut Bits<'_>,
    columns: u32,
    line: &mut Vec<u32>,
) -> Result<(), Damage> {
    line.clear();
    let mut at = 0_u32;
    let mut white = true;
    while at < columns {
        let length = run_length(bits, white, columns.saturating_sub(at))?;
        run(line, at, white, length);
        at = at.saturating_add(length);
        white = !white;
    }
    Ok(())
}

/// Reads one two-dimensional mode code (T.4 Table 4).
fn mode(bits: &mut Bits<'_>) -> Result<Mode, Damage> {
    let index = usize::try_from(bits.peek(MODE_CODE_BITS)).unwrap_or(0);
    match MODES.get(index).copied().flatten() {
        Some((length, mode)) if usize::from(length) <= bits.remaining() => {
            bits.skip(u32::from(length));
            Ok(mode)
        }
        Some(_) => Err(Damage::UnexpectedEnd),
        None if bits.remaining() < MODE_CODE_BITS as usize && bits.only_zeros_remain() => {
            Err(Damage::UnexpectedEnd)
        }
        None => Err(Damage::InvalidCode),
    }
}

/// Decodes a two-dimensionally coded line against its reference line (T.4 section 4.2.1.3,
/// T.6 section 2.2).
///
/// The names are the Recommendations': `a0` is where the coding line's current run begins —
/// imaginary, before the first pel, at the start of the line — `b1` is the first changing element
/// on the reference line to the right of `a0` whose colour is the opposite of `a0`'s, and `b2` is
/// the changing element after `b1`. A reference line with no such element has them at the line's
/// end, where T.4 section 4.2.1.3.1 places its imaginary changing elements.
fn two_dimensional_line(
    bits: &mut Bits<'_>,
    columns: u32,
    reference: &[u32],
    line: &mut Vec<u32>,
) -> Result<(), Damage> {
    line.clear();
    // `None` is the imaginary `a0` before the line's first pel.
    let mut a0: Option<u32> = None;
    let mut white = true;
    // Reference elements at or left of `a0` are never `b1` again, because `a0` only moves right.
    let mut passed = 0_usize;
    loop {
        let at = a0.unwrap_or(0);
        if a0.is_some() && at >= columns {
            return Ok(());
        }
        while let (Some(a0), Some(&change)) = (a0, reference.get(passed)) {
            if change > a0 {
                break;
            }
            passed = passed.saturating_add(1);
        }
        // Changes alternate in colour, and the one at an even index is a change to black: the
        // right colour for `b1` is black while the current run is white.
        let b1_index = if passed.is_multiple_of(2) == white {
            passed
        } else {
            passed.saturating_add(1)
        };
        let b1 = reference.get(b1_index).copied().unwrap_or(columns);
        let b2 = reference
            .get(b1_index.saturating_add(1))
            .copied()
            .unwrap_or(columns);

        match mode(bits)? {
            // T.4 section 4.2.1.3.2 (a): the run continues under `b2` in the same colour.
            Mode::Pass => {
                run(line, at, white, b2.saturating_sub(at));
                a0 = Some(b2);
            }
            // (b): `a1` is `b1` moved by at most three pels; the colour changes there.
            Mode::Vertical(offset) => {
                let a1 = i64::from(b1).saturating_add(i64::from(offset));
                let a1 = u32::try_from(a1).map_err(|_| Damage::OutsideTheLine)?;
                if a1 < at || a1 > columns {
                    return Err(Damage::OutsideTheLine);
                }
                run(line, at, white, a1.saturating_sub(at));
                white = !white;
                a0 = Some(a1);
            }
            // (c): two runs coded one-dimensionally, `a0a1` then `a1a2`.
            Mode::Horizontal => {
                let room = columns.saturating_sub(at);
                let first = run_length(bits, white, room)?;
                let second = run_length(bits, !white, room.saturating_sub(first))?;
                let a1 = at.saturating_add(first);
                run(line, at, white, first);
                run(line, a1, !white, second);
                // Two empty runs code no pel and no changing element, so `a0` stays where it
                // was — before the line, if nothing has been coded yet. Encoders write `H(0,0)`
                // at a line's start, and moving `a0` onto the first pel would take that pel out
                // of every later `b1` (ADR 1349 section 3).
                if first.saturating_add(second) > 0 {
                    a0 = Some(a1.saturating_add(second));
                }
            }
        }
        if a0.is_some_and(|a0| a0 >= columns) {
            return Ok(());
        }
    }
}
