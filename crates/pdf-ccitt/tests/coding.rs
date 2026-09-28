//! The decoder against coded data written out by hand from ITU-T T.4 and T.6, and against ISO
//! 32000-2 §7.4.6 Table 11's `/DamagedRowsBeforeError`.
//!
//! Every stream below is a string of binary digits assembled from the code tables — T.4 Table 2
//! (terminating codes), Tables 3a and 3b (make-up codes), Table 4 (mode codes), the end-of-line
//! code of section 4.1.2 and T.6 section 2.4.1.1's end-of-facsimile block — and every expected
//! scan line is written as its changing elements, read off the figure or worked by hand from the
//! coding rules, never taken from a decoder's output.

use pdf_ccitt::{Coding, Damage, Error, Parameters, Rows, Summary, decode};

/// The scan lines a decode delivered, each as its changing elements.
#[derive(Default)]
struct Lines(Vec<Vec<u32>>);

impl Rows for Lines {
    fn row(&mut self, changes: &[u32]) {
        self.0.push(changes.to_vec());
    }
}

/// Packs a string of binary digits (spaces ignored) into bytes, padding the last with zeros.
fn bits(digits: &str) -> Vec<u8> {
    let digits: Vec<bool> = digits
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c == '1')
        .collect();
    digits
        .chunks(8)
        .map(|chunk| {
            chunk.iter().enumerate().fold(0_u8, |byte, (i, one)| {
                byte | (u8::from(*one) << 7_usize.saturating_sub(i))
            })
        })
        .collect()
}

/// Group 4 parameters for `columns` pels and at most `rows` lines.
fn group_4(columns: u32, rows: u32) -> Parameters {
    Parameters {
        coding: Coding::Group4,
        columns,
        rows,
        end_of_line: false,
        encoded_byte_align: false,
        end_of_block: true,
        damaged_rows_before_error: 0,
    }
}

fn decoded(data: &str, parameters: Parameters) -> (Result<Summary, Error>, Vec<Vec<u32>>) {
    let mut lines = Lines::default();
    let result = decode(&bits(data), &parameters, &mut lines);
    (result, lines.0)
}

// The codewords the fixtures use, named by what T.4's tables say they stand for.
const EOL: &str = "000000000001";
const EOFB: &str = "000000000001 000000000001";
const H: &str = "001";
const P: &str = "0001";
const V0: &str = "1";
const VR2: &str = "000011";
const VR3: &str = "0000011";
const VL1: &str = "010";
const VR1: &str = "011";
const VL2: &str = "000010";

/// T.4 Figure 10, "Coding examples: First part of scan line", each example as a two-line Group 4
/// image sixteen pels wide: the first line is the figure's reference line, coded against T.6
/// section 2.2.1's imaginary white line, and the second is the figure's coding line, coded with
/// the modes the figure names and completed with `V(0)` to the line's end. Pel 1 of the figure is
/// position 0 here.
#[test]
fn t4_figure_10_first_part_of_a_scan_line() {
    // White 0, black 16, white 1, black 15, white 9, black 7, black 3 — T.4 Table 2.
    let (w0, b16, w1, b15, w9, b7, b3) = (
        "00110101",
        "0000010111",
        "000111",
        "000011000",
        "10100",
        "00011",
        "10",
    );
    let examples: [(&str, String, Vec<u32>, Vec<u32>); 5] = [
        // 1: reference and coding line both black from pel 1; V(0).
        ("V(0)", format!("{H}{w0}{b16} {V0}{V0}"), vec![0], vec![0]),
        // 2: reference black from pel 2, coding line from pel 1; V_L(1).
        (
            "V_L(1)",
            format!("{H}{w1}{b15} {VL1}{V0}"),
            vec![1],
            vec![0],
        ),
        // 3: reference black on pels 1 to 3, coding line white; P, a'0 under b2 at pel 4.
        ("P", format!("{H}{w0}{b3}{V0} {P}{V0}"), vec![0, 3], vec![]),
        // 4: reference black from pel 1, coding line black from pel 3; V_R(2).
        (
            "V_R(2)",
            format!("{H}{w0}{b16} {VR2}{V0}"),
            vec![0],
            vec![2],
        ),
        // 5: reference black from pel 10; coding line black on pels 1 to 3, then black again
        // from pel 11: H(0,3) then V_R(1).
        (
            "H(0,3) V_R(1)",
            format!("{H}{w9}{b7} {H}{w0}{b3}{VR1}{V0}"),
            vec![9],
            vec![0, 3, 10],
        ),
    ];
    for (name, coded, reference, coding) in examples {
        let (result, lines) = decoded(&format!("{coded}{EOFB}"), group_4(16, 10));
        assert_eq!(
            result,
            Ok(Summary {
                rows: 2,
                concealed: 0
            }),
            "{name}"
        );
        assert_eq!(lines, vec![reference, coding], "{name}");
    }
}

/// T.4 Figure 11, "Coding examples: Last part of scan line", on a line of 1728 pels — the width the
/// figure numbers its pels against — each example as a two-line Group 4 image. Pels left of the
/// figure's cells are white on both lines except where a figure's first cell is black on both,
/// which is carried in from pel 1717 (0-based 1716 or later), so that the codes the figure names
/// are the ones that code its changing elements.
#[test]
fn t4_figure_11_last_part_of_a_scan_line() {
    // White make-up 1664 with white terminating 60, 56, 61, 54, 53, 52 — T.4 Tables 2 and 3a.
    let white = |terminating: &str| format!("011000{terminating}");
    let (w1724, w1720, w1725, w1718, w1717, w1716) = (
        white("01001011"),
        white("01011001"),
        white("00110010"),
        white("00100101"),
        white("00100100"),
        white("01010101"),
    );
    // Short runs: white 2 and 7, black 0, 2, 3, 4, 5, 6 and 10.
    let (w2, w7) = ("0111", "1111");
    let (b0, b2, b3, b4, b5, b6, b10) =
        ("0000110111", "11", "10", "011", "0011", "0010", "0000100");
    let examples: [(&str, String, Vec<u32>, Vec<u32>); 8] = [
        (
            "1: V(0), V(0)",
            format!("{H}{w1724}{b4} {V0}{V0}"),
            vec![1724],
            vec![1724],
        ),
        (
            "2: V(0), V_R(2)",
            format!("{H}{w1724}{b2}{V0} {V0}{VR2}"),
            vec![1724, 1726],
            vec![1724],
        ),
        (
            "3: V(0), V_L(2), V(0)",
            format!("{H}{w1720}{b3}{V0} {V0}{V0}{VL2}{V0}"),
            vec![1720, 1723],
            vec![1720, 1723, 1726],
        ),
        (
            "4: V(0), V_L(2), V(0)",
            format!("{H}{w1724}{b4} {V0}{VL2}{V0}"),
            vec![1724],
            vec![1724, 1726],
        ),
        (
            "5: V(0), V_R(3)",
            format!("{H}{w1720}{b3}{H}{w2}{b3} {V0}{V0}{VR3}"),
            vec![1720, 1723, 1725],
            vec![1720, 1723],
        ),
        (
            "6: P, V(0)",
            format!("{H}{w1725}{b2}{V0} {P}{V0}"),
            vec![1725, 1727],
            vec![],
        ),
        (
            "7: H(2,6)",
            format!("{H}{w1718}{b10} {H}{w1716}{b4}{H}{w2}{b6}"),
            vec![1718],
            vec![1716, 1720, 1722],
        ),
        (
            "8: V(0), H(7,0)",
            format!("{H}{w1717}{b4}{H}{w2}{b5} {V0}{V0}{H}{w7}{b0}"),
            vec![1717, 1721, 1723],
            vec![1717, 1721],
        ),
    ];
    for (name, coded, reference, coding) in examples {
        let (result, lines) = decoded(&format!("{coded}{EOFB}"), group_4(1728, 10));
        assert_eq!(result.map(|summary| summary.rows), Ok(2), "{name}");
        assert_eq!(lines, vec![reference, coding], "{name}");
    }
}

/// T.6 section 2.2.4: a run of 2560 pels or more is coded as make-up codes of 2560 until less
/// than 2560 remains, then as the shorter run would be. 5200 white is 2560, 2560, then 64 + 16,
/// and the black run after it in horizontal mode is zero.
#[test]
fn a_long_run_is_a_sequence_of_make_up_codes() {
    let m2560 = "000000011111";
    let coded = format!("{H}{m2560}{m2560}11011 101010 0000110111{EOFB}");
    let (result, lines) = decoded(&coded, group_4(5200, 1));
    assert_eq!(result.map(|summary| summary.rows), Ok(1));
    assert_eq!(lines, vec![Vec::<u32>::new()]);
}

/// Group 3 parameters, with `/EndOfLine` true.
fn group_3(coding: Coding, columns: u32, rows: u32, tolerated: u32) -> Parameters {
    Parameters {
        coding,
        columns,
        rows,
        end_of_line: true,
        encoded_byte_align: false,
        end_of_block: true,
        damaged_rows_before_error: tolerated,
    }
}

/// White 4, black 8, white 4 on a line of sixteen — T.4 Table 2.
const ONE_D_4_8_4: &str = "1011 000101 1011";
/// White 16.
const ONE_D_WHITE: &str = "101010";

/// Eight zeros begin no codeword of either colour, so a line holding them is damaged there.
const DAMAGED: &str = "00000000 1 1";

/// T.4 section 4.2.4's return to control in the mixed coding: six end-of-line codes, each with
/// the tag bit 1.
fn rtc_mixed() -> String {
    format!("{EOL}1").repeat(6)
}

/// A Group 3 mixed-coding stream of four lines of sixteen pels, the third damaged:
///
/// - line 0, one-dimensional: white 4, black 8, white 4;
/// - line 1, two-dimensional, `V(0)` three times against line 0: the same line;
/// - line 2, one-dimensional, damaged: eight zeros begin no codeword;
/// - line 3, two-dimensional: `V_R(1)`, `V(0)`, `V(0)`. Against a reference line black on 4 to
///   11 that is black on 5 to 11; against a white reference it puts `a1` at 17, past the line.
fn one_damaged_row() -> String {
    format!(
        "{EOL}1{ONE_D_4_8_4} {EOL}0{V0}{V0}{V0} {EOL}1{DAMAGED} {EOL}0{VR1}{V0}{V0} {}",
        rtc_mixed()
    )
}

/// Table 11: "Tolerating a damaged row shall mean locating its end in the encoded data by searching
/// for an `EndOfLine` pattern and then substituting decoded data from the previous row if the
/// previous row was not damaged". So line 2 is line 1 again, and line 3 is decoded against it —
/// the reference line T.4 section 4.2.1.3 names is the line above, as delivered.
#[test]
fn a_damaged_row_is_the_previous_row_and_the_decode_goes_on() {
    let (result, lines) = decoded(&one_damaged_row(), group_3(Coding::Group3Mixed, 16, 100, 2));
    assert_eq!(
        result,
        Ok(Summary {
            rows: 4,
            concealed: 1
        })
    );
    assert_eq!(
        lines,
        vec![vec![4, 12], vec![4, 12], vec![4, 12], vec![5, 12]]
    );
}

/// Table 11's default of zero: "[t]he number of damaged rows of data that shall be tolerated before
/// an error occurs" is none, so the decode ends at line 2 with lines 0 and 1 delivered.
#[test]
fn with_no_rows_tolerated_the_first_damaged_row_is_the_error() {
    let (result, lines) = decoded(&one_damaged_row(), group_3(Coding::Group3Mixed, 16, 100, 0));
    let Err(Error::Damaged {
        damage,
        row,
        delivered,
        ..
    }) = result
    else {
        panic!("the damaged row is an error: {result:?}");
    };
    assert_eq!((damage, row), (Damage::InvalidCode, 2));
    assert_eq!(
        delivered,
        Summary {
            rows: 2,
            concealed: 0
        }
    );
    assert_eq!(lines, vec![vec![4, 12], vec![4, 12]]);
}

/// Table 11's second case: "or a white scan line if the previous row was also damaged". And the
/// count is of damaged rows, so a third is the error where two are tolerated and a second is where
/// one is.
#[test]
fn a_second_damaged_row_in_a_row_is_white_and_the_count_is_the_limit() {
    let coded = format!(
        "{EOL}{ONE_D_4_8_4} {EOL}{DAMAGED} {EOL}{DAMAGED} {EOL}{ONE_D_4_8_4} {}",
        EOL.repeat(6)
    );
    let one_d = |tolerated| group_3(Coding::Group3OneDimensional, 16, 100, tolerated);

    let (result, lines) = decoded(&coded, one_d(2));
    assert_eq!(
        result,
        Ok(Summary {
            rows: 4,
            concealed: 2
        })
    );
    assert_eq!(lines, vec![vec![4, 12], vec![4, 12], vec![], vec![4, 12]]);

    let (result, lines) = decoded(&coded, one_d(1));
    assert!(
        matches!(result, Err(Error::Damaged { row: 2, .. })),
        "{result:?}"
    );
    assert_eq!(lines, vec![vec![4, 12], vec![4, 12]]);
}

/// A damaged first row has no previous row to copy. T.6 section 2.2.1 makes the line before the
/// first an imaginary white line, and that is what it is (ADR 1349).
#[test]
fn a_damaged_first_row_is_white() {
    let coded = format!("{EOL}{DAMAGED} {EOL}{ONE_D_4_8_4} {}", EOL.repeat(6));
    let (result, lines) = decoded(&coded, group_3(Coding::Group3OneDimensional, 16, 100, 1));
    assert_eq!(result.map(|summary| summary.concealed), Ok(1));
    assert_eq!(lines, vec![vec![], vec![4, 12]]);
}

/// Where the concealment is on, a line whose codewords fill its width and are followed by something
/// other than an end-of-line code is a damaged row, and its end is still the next end-of-line code:
/// the concealment finds a row's end "by searching for an `EndOfLine` pattern", so a row is what
/// lies between two. Without the concealment the filter is not asked to police Table 11's "If
/// `EndOfLine` is true end-of-line bit patterns shall be present", and the next line begins where
/// the last one ended — here with the stray `1`, which reads as a white run of three and leaves
/// eight zeros that begin no codeword.
#[test]
fn a_full_line_not_followed_by_an_end_of_line_is_damaged_under_the_concealment() {
    let coded = format!(
        "{EOL}{ONE_D_WHITE} {EOL}{ONE_D_4_8_4}1 {EOL}{ONE_D_4_8_4} {}",
        EOL.repeat(6)
    );
    let (result, lines) = decoded(&coded, group_3(Coding::Group3OneDimensional, 16, 100, 1));
    assert_eq!(result.map(|summary| summary.concealed), Ok(1));
    assert_eq!(lines, vec![vec![], vec![], vec![4, 12]]);

    let (result, lines) = decoded(&coded, group_3(Coding::Group3OneDimensional, 16, 100, 0));
    assert!(
        matches!(
            result,
            Err(Error::Damaged {
                damage: Damage::InvalidCode,
                row: 2,
                ..
            })
        ),
        "{result:?}"
    );
    assert_eq!(lines, vec![vec![], vec![4, 12]]);
}

/// A horizontal mode coding two empty runs codes no pel and moves nothing: at a line's start `a0`
/// stays the imaginary element before the first pel, so the reference line's change at pel 0 is
/// still `b1` for the `V(0)` after it. A producer writes exactly this (ADR 1349 section 3).
#[test]
fn a_horizontal_mode_of_two_empty_runs_moves_nothing() {
    let coded = format!("{H}00110101 0000010111 {H}00110101 0000110111 {V0}{V0}{EOFB}");
    let (result, lines) = decoded(&coded, group_4(16, 10));
    assert_eq!(result.map(|summary| summary.rows), Ok(2));
    assert_eq!(lines, vec![vec![0], vec![0]]);
}

/// Table 11: the entry "shall apply only if `EndOfLine` is true and K is non-negative". Group 4
/// carries no end-of-line codes to find a row's end by, and neither does Group 3 with
/// `/EndOfLine` false, so a value there is inert and the first damaged row ends the decode.
#[test]
fn the_tolerance_is_inert_where_the_entry_does_not_apply() {
    let group_4_damaged = format!("{H}00110101 0000010111 00000001 {EOFB}");
    let mut parameters = group_4(16, 10);
    parameters.damaged_rows_before_error = 5;
    let (result, lines) = decoded(&group_4_damaged, parameters);
    assert!(
        matches!(result, Err(Error::Damaged { row: 1, .. })),
        "{result:?}"
    );
    assert_eq!(lines, vec![vec![0]]);

    let mut parameters = group_3(Coding::Group3OneDimensional, 16, 100, 5);
    parameters.end_of_line = false;
    let coded = format!("{ONE_D_4_8_4}{DAMAGED}{EOL}{ONE_D_4_8_4}");
    let (result, lines) = decoded(&coded, parameters);
    assert!(
        matches!(result, Err(Error::Damaged { row: 1, .. })),
        "{result:?}"
    );
    assert_eq!(lines, vec![vec![4, 12]]);
}

/// Table 11's `/EndOfBlock`: the end-of-block pattern "appropriate for the K parameter" ends the
/// data — two end-of-line codes for Group 4, six for Group 3 — whatever `/Rows` would allow. And
/// with `/EndOfBlock` false the filter stops "when it has decoded the number of lines indicated by
/// Rows or when its data has been exhausted, whichever occurs first".
#[test]
fn the_decode_ends_on_the_pattern_the_rows_or_the_data() {
    let two_lines = format!("{H}00110101 0000010111 {V0}{V0} {EOFB}");
    let (result, _) = decoded(&two_lines, group_4(16, 100));
    assert_eq!(result.map(|summary| summary.rows), Ok(2));
    let (result, _) = decoded(&two_lines, group_4(16, 1));
    assert_eq!(result.map(|summary| summary.rows), Ok(1));

    let mut unbounded = group_3(Coding::Group3OneDimensional, 16, 100, 0);
    unbounded.end_of_block = false;
    let three_lines = format!("{EOL}{ONE_D_WHITE}{EOL}{ONE_D_WHITE}{EOL}{ONE_D_WHITE}");
    let (result, _) = decoded(&three_lines, unbounded);
    assert_eq!(result.map(|summary| summary.rows), Ok(3));
}

/// Table 11's `/EncodedByteAlign`: each line begins on a byte boundary, the zeros before it
/// skipped.
#[test]
fn byte_aligned_lines_skip_their_padding() {
    let mut parameters = group_3(Coding::Group3OneDimensional, 16, 2, 0);
    parameters.end_of_line = false;
    parameters.encoded_byte_align = true;
    // White 16 is six bits; two zeros pad it to a byte, and the second line begins after them.
    let coded = format!("{ONE_D_WHITE}00 {ONE_D_4_8_4}");
    let (result, lines) = decoded(&coded, parameters);
    assert_eq!(result.map(|summary| summary.rows), Ok(2));
    assert_eq!(lines, vec![vec![], vec![4, 12]]);
}

/// §7.4.6 forbids "any error correction", so a run longer than what is left of its line is damage
/// rather than a run cut to fit.
#[test]
fn a_run_past_the_end_of_the_line_is_damage() {
    let mut parameters = group_3(Coding::Group3OneDimensional, 8, 1, 0);
    parameters.end_of_line = false;
    let (result, lines) = decoded(ONE_D_4_8_4, parameters);
    assert!(
        matches!(
            result,
            Err(Error::Damaged {
                damage: Damage::OutsideTheLine,
                row: 0,
                ..
            })
        ),
        "{result:?}"
    );
    assert!(lines.is_empty());
}
