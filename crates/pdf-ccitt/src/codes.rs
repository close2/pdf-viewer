//! The code tables: run lengths (ITU-T T.4 Tables 2, 3a and 3b, which T.6 section 2.2.4
//! adopts) and the two-dimensional mode codes (T.4 Table 4, T.6 Table 1).
//!
//! Each table is written the way the Recommendation prints it — a code as a string of binary
//! digits beside the value it stands for — and turned into a lookup table by a `const fn`, so
//! the tables a reader checks against the text are the tables the decoder runs, and building
//! them costs nothing at startup (they are `static` data in the binary).
//!
//! **Why lookup tables rather than a bit-at-a-time tree.** A 600 dpi scan line is a few dozen
//! codewords, and a page is thousands of lines; indexing a table with the next thirteen bits
//! decodes a codeword in one step where a tree takes one step per bit. The run tables are 32 KiB
//! each, built at compile time; ADR 1349 has the decode-time measurement over the corpus.

/// The longest run-length codeword, in bits: the black make-up codes of T.4 Table 3a.
pub(crate) const RUN_CODE_BITS: u32 = 13;

/// The longest mode codeword this decoder reads, in bits (the vertical modes of offset three).
pub(crate) const MODE_CODE_BITS: u32 = 7;

/// White terminating codes, T.4 Table 2, indexed by run length 0 to 63.
const WHITE_TERMINATING: [&str; 64] = [
    "00110101", "000111", "0111", "1000", "1011", "1100", "1110", "1111", "10011", "10100",
    "00111", "01000", "001000", "000011", "110100", "110101", "101010", "101011", "0100111",
    "0001100", "0001000", "0010111", "0000011", "0000100", "0101000", "0101011", "0010011",
    "0100100", "0011000", "00000010", "00000011", "00011010", "00011011", "00010010", "00010011",
    "00010100", "00010101", "00010110", "00010111", "00101000", "00101001", "00101010", "00101011",
    "00101100", "00101101", "00000100", "00000101", "00001010", "00001011", "01010010", "01010011",
    "01010100", "01010101", "00100100", "00100101", "01011000", "01011001", "01011010", "01011011",
    "01001010", "01001011", "00110010", "00110011", "00110100",
];

/// Black terminating codes, T.4 Table 2, indexed by run length 0 to 63.
const BLACK_TERMINATING: [&str; 64] = [
    "0000110111",
    "010",
    "11",
    "10",
    "011",
    "0011",
    "0010",
    "00011",
    "000101",
    "000100",
    "0000100",
    "0000101",
    "0000111",
    "00000100",
    "00000111",
    "000011000",
    "0000010111",
    "0000011000",
    "0000001000",
    "00001100111",
    "00001101000",
    "00001101100",
    "00000110111",
    "00000101000",
    "00000010111",
    "00000011000",
    "000011001010",
    "000011001011",
    "000011001100",
    "000011001101",
    "000001101000",
    "000001101001",
    "000001101010",
    "000001101011",
    "000011010010",
    "000011010011",
    "000011010100",
    "000011010101",
    "000011010110",
    "000011010111",
    "000001101100",
    "000001101101",
    "000011011010",
    "000011011011",
    "000001010100",
    "000001010101",
    "000001010110",
    "000001010111",
    "000001100100",
    "000001100101",
    "000001010010",
    "000001010011",
    "000000100100",
    "000000110111",
    "000000111000",
    "000000100111",
    "000000101000",
    "000001011000",
    "000001011001",
    "000000101011",
    "000000101100",
    "000001011010",
    "000001100110",
    "000001100111",
];

/// White make-up codes, T.4 Table 3a, for run lengths 64 to 1728 in steps of 64.
const WHITE_MAKEUP: [&str; 27] = [
    "11011",
    "10010",
    "010111",
    "0110111",
    "00110110",
    "00110111",
    "01100100",
    "01100101",
    "01101000",
    "01100111",
    "011001100",
    "011001101",
    "011010010",
    "011010011",
    "011010100",
    "011010101",
    "011010110",
    "011010111",
    "011011000",
    "011011001",
    "011011010",
    "011011011",
    "010011000",
    "010011001",
    "010011010",
    "011000",
    "010011011",
];

/// Black make-up codes, T.4 Table 3a, for run lengths 64 to 1728 in steps of 64.
const BLACK_MAKEUP: [&str; 27] = [
    "0000001111",
    "000011001000",
    "000011001001",
    "000001011011",
    "000000110011",
    "000000110100",
    "000000110101",
    "0000001101100",
    "0000001101101",
    "0000001001010",
    "0000001001011",
    "0000001001100",
    "0000001001101",
    "0000001110010",
    "0000001110011",
    "0000001110100",
    "0000001110101",
    "0000001110110",
    "0000001110111",
    "0000001010010",
    "0000001010011",
    "0000001010100",
    "0000001010101",
    "0000001011010",
    "0000001011011",
    "0000001100100",
    "0000001100101",
];

/// The extended make-up codes both colours share, T.4 Table 3b, for run lengths 1792 to 2560 in
/// steps of 64.
const EXTENDED_MAKEUP: [&str; 13] = [
    "00000001000",
    "00000001100",
    "00000001101",
    "000000010010",
    "000000010011",
    "000000010100",
    "000000010101",
    "000000010110",
    "000000010111",
    "000000011100",
    "000000011101",
    "000000011110",
    "000000011111",
];

/// One entry of a run-length lookup table: how many bits the codeword takes and the run length
/// it stands for. A `bits` of zero marks an index no codeword begins with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RunCode {
    /// The codeword's length in bits; zero where the bits are not a codeword.
    pub(crate) bits: u8,
    /// The run length: below 64 for a terminating code, a multiple of 64 for a make-up code.
    pub(crate) run: u16,
}

/// Which lookup table a run is read from.
pub(crate) type RunTable = [RunCode; 1 << RUN_CODE_BITS];

/// White run lengths, indexed by the next [`RUN_CODE_BITS`] bits of the data.
pub(crate) static WHITE: RunTable = run_table(&WHITE_TERMINATING, &WHITE_MAKEUP);

/// Black run lengths, indexed by the next [`RUN_CODE_BITS`] bits of the data.
pub(crate) static BLACK: RunTable = run_table(&BLACK_TERMINATING, &BLACK_MAKEUP);

/// A two-dimensional coding mode, T.4 section 4.2.1.3.2 and T.6 section 2.2.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Pass mode: `b2` lies to the left of `a1`.
    Pass,
    /// Horizontal mode: two runs follow as run-length codes.
    Horizontal,
    /// Vertical mode: `a1` is `b1` plus this offset, between minus three and three.
    Vertical(i8),
}

/// One entry of the mode lookup table: the codeword's length and its mode, or `None` where the
/// bits are not a mode codeword this decoder reads.
pub(crate) type ModeCode = Option<(u8, Mode)>;

/// The mode codes, T.4 Table 4, indexed by the next [`MODE_CODE_BITS`] bits.
///
/// The seven-bit prefix `0000001` is the extension code T.4 section 4.2.1.3.2 reserves for
/// optional modes (uncompressed mode among them) that §7.4.6 does not name and no parameter of
/// Table 11 announces, and seven zeros begin no mode codeword at all — so both read as `None`,
/// which is damage in the line that holds them.
pub(crate) static MODES: [ModeCode; 1 << MODE_CODE_BITS] = mode_table(&[
    ("0001", Mode::Pass),
    ("001", Mode::Horizontal),
    ("1", Mode::Vertical(0)),
    ("011", Mode::Vertical(1)),
    ("000011", Mode::Vertical(2)),
    ("0000011", Mode::Vertical(3)),
    ("010", Mode::Vertical(-1)),
    ("000010", Mode::Vertical(-2)),
    ("0000010", Mode::Vertical(-3)),
]);

/// A codeword's bits as an integer, and its length.
#[expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    reason = "evaluated at compile time over the tables above, where an overflow or a lost bit is \
              a build error rather than a wrong table"
)]
const fn parse(code: &str) -> (u32, u32) {
    let digits = code.as_bytes();
    let mut value = 0_u32;
    let mut at = 0;
    while at < digits.len() {
        value = (value << 1) | (digits[at] == b'1') as u32;
        at += 1;
    }
    (value, digits.len() as u32)
}

/// Fills every index of a `width`-bit table whose leading bits are `code` with `entry`.
///
/// A codeword of `n` bits occupies `2^(width - n)` consecutive indices — every way the bits after
/// it can go.
#[expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    reason = "evaluated at compile time over the tables above, where an overflow or a lost bit is \
              a build error rather than a wrong table"
)]
const fn place(table: &mut RunTable, code: &str, run: u16) {
    let (value, length) = parse(code);
    let spare = RUN_CODE_BITS - length;
    let first = (value << spare) as usize;
    let count = 1_usize << spare;
    let mut index = 0;
    while index < count {
        table[first + index] = RunCode {
            bits: length as u8,
            run,
        };
        index += 1;
    }
}

/// Builds one colour's run-length table from its terminating and make-up codes and the shared
/// extended make-up codes.
#[expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::large_stack_arrays,
    reason = "evaluated at compile time over the tables above, where an overflow or a lost bit is \
              a build error rather than a wrong table"
)]
const fn run_table(terminating: &[&str; 64], makeup: &[&str; 27]) -> RunTable {
    let mut table = [RunCode { bits: 0, run: 0 }; 1 << RUN_CODE_BITS];
    let mut run = 0;
    while run < 64 {
        place(&mut table, terminating[run], run as u16);
        run += 1;
    }
    let mut step = 0;
    while step < 27 {
        place(&mut table, makeup[step], ((step + 1) * 64) as u16);
        step += 1;
    }
    let mut step = 0;
    while step < 13 {
        place(&mut table, EXTENDED_MAKEUP[step], ((step + 28) * 64) as u16);
        step += 1;
    }
    table
}

/// Builds the mode table from T.4 Table 4's codes.
#[expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    reason = "evaluated at compile time over the tables above, where an overflow or a lost bit is \
              a build error rather than a wrong table"
)]
const fn mode_table(codes: &[(&str, Mode); 9]) -> [ModeCode; 1 << MODE_CODE_BITS] {
    let mut table: [ModeCode; 1 << MODE_CODE_BITS] = [None; 1 << MODE_CODE_BITS];
    let mut which = 0;
    while which < codes.len() {
        let (value, length) = parse(codes[which].0);
        let spare = MODE_CODE_BITS - length;
        let first = (value << spare) as usize;
        let mut index = 0;
        while index < 1 << spare {
            table[first + index] = Some((length as u8, codes[which].1));
            index += 1;
        }
        which += 1;
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every codeword of a colour, with its run length.
    fn every_code(
        terminating: &[&'static str; 64],
        makeup: &[&'static str; 27],
    ) -> Vec<(&'static str, u16)> {
        let terminating = terminating.iter().zip(0_u16..);
        let makeup = makeup
            .iter()
            .zip((1_u16..).map(|step| step.saturating_mul(64)));
        let extended = EXTENDED_MAKEUP
            .iter()
            .zip((28_u16..).map(|step| step.saturating_mul(64)));
        terminating
            .chain(makeup)
            .chain(extended)
            .map(|(code, run)| (*code, run))
            .collect()
    }

    /// No codeword of a colour is a prefix of another. A code that failed this would make the
    /// table's answer depend on which of two codewords was placed last, and T.4's codes are
    /// prefix-free by construction — so a typing error in the tables above shows here.
    #[test]
    fn each_colours_codes_are_prefix_free() {
        for (terminating, makeup) in [
            (&WHITE_TERMINATING, &WHITE_MAKEUP),
            (&BLACK_TERMINATING, &BLACK_MAKEUP),
        ] {
            let codes = every_code(terminating, makeup);
            for (i, (a, _)) in codes.iter().enumerate() {
                for (j, (b, _)) in codes.iter().enumerate() {
                    assert!(i == j || !b.starts_with(a), "{a} is a prefix of {b}");
                }
            }
        }
    }

    /// Every codeword looks itself up: the table answers its run and its length.
    #[test]
    fn every_codeword_decodes_to_its_run() {
        for (table, terminating, makeup) in [
            (&WHITE, &WHITE_TERMINATING, &WHITE_MAKEUP),
            (&BLACK, &BLACK_TERMINATING, &BLACK_MAKEUP),
        ] {
            for (code, run) in every_code(terminating, makeup) {
                let (value, length) = parse(code);
                let entry = table[(value << (RUN_CODE_BITS - length)) as usize];
                assert_eq!(
                    (u32::from(entry.bits), entry.run),
                    (length, run),
                    "code {code}"
                );
            }
        }
    }

    /// The end-of-line code, eleven zeros and a one, begins no codeword of either colour and no
    /// mode codeword — the property T.4 builds the pattern for, which is what lets a decoder
    /// find it (ADR 1349).
    #[test]
    fn no_codeword_begins_with_eight_zeros() {
        for table in [&WHITE, &BLACK] {
            assert!(
                table[..1 << (RUN_CODE_BITS - 8)]
                    .iter()
                    .all(|e| e.bits == 0)
            );
        }
        assert!(MODES[..2].iter().all(Option::is_none));
    }
}
