//! A content stream's integer past ISO 19005-2 section 6.1.13's bound is written as the real of the
//! same value, a period after its last digit, and nothing else moves (`doc/adr/3533`).
//!
//! The expected values come from ISO 32000-2 §7.3.3, which makes an integer and a real two
//! spellings of a mathematical number, spells the real with "a leading, trailing, or embedded
//! PERIOD", and says "\[w]herever a real number is expected, an integer may be used instead" — and
//! from ISO 19005-2 section 6.1.13, which bounds integers at ±2^31 and reals at ±3.403×10^38. The
//! period is written only where the operator's operands are numbers; where §7.3.3 expects an
//! integer, a real "shall not be present", and the row keeps the target's refusal.
use super::*;

const ROW: &str = "implementation-limits/values-written-in-content-streams";

/// A page whose content stream is `data`, written whole.
fn a_page_drawing(data: &[u8]) -> Vec<u8> {
    Conforming {
        contents: Some((String::new(), data.to_vec())),
        ..Conforming::part_two()
    }
    .build()
}

/// `data` with every `.` removed, which is what the source said where the conversion added one.
fn without_periods_after_wide_runs(data: &[u8]) -> Vec<u8> {
    let text = String::from_utf8(data.to_vec()).expect("ASCII");
    text.replace(
        "2366454844678207931159090160467968.",
        "2366454844678207931159090160467968",
    )
    .into_bytes()
}

#[test]
fn a_wide_path_operand_is_written_as_the_real_of_its_value_and_nothing_else_moves() {
    let drawn: &[u8] =
        b"q 0 0 m 2366454844678207931159090160467968 -2366454844678207931159090160467968 l S Q";
    let source = a_page_drawing(drawn);
    let plan = plan_from(
        pdf_transform::profiles::named("as-shown")
            .expect("a shipped profile")
            .text,
        Target::Two(Level::B),
    );
    let (report, output) = convert_with_plan(&source, &plan);
    assert_eq!(
        decision(&report, ROW),
        Decision::Mechanical(Rewrite::IntegerWrittenAsReal)
    );
    let output = output.expect("a file is written");
    assert_eq!(
        holds(&output, Target::Two(Level::B)).verdict(),
        Verdict::Conforms
    );
    let contents = decoded_contents(&output);
    assert_eq!(
        contents.len(),
        drawn.len() + 2,
        "{}",
        String::from_utf8_lossy(&contents)
    );
    assert!(
        String::from_utf8_lossy(&contents)
            .contains("2366454844678207931159090160467968. -2366454844678207931159090160467968. l"),
        "{}",
        String::from_utf8_lossy(&contents)
    );
    // Every byte that is not one of the two periods is the producer's.
    assert_eq!(without_periods_after_wide_runs(&contents), drawn);
}

#[test]
fn an_operand_where_an_integer_is_expected_keeps_the_targets_refusal() {
    let source = a_page_drawing(b"BT 3000000000 Tr ET");
    let plan = plan_from(
        pdf_transform::profiles::named("as-shown")
            .expect("a shipped profile")
            .text,
        Target::Two(Level::B),
    );
    let (report, output) = convert_with_plan(&source, &plan);
    assert!(
        matches!(
            decision(&report, ROW),
            Decision::Refused(Because::NotThisTarget(_))
        ),
        "{:?}",
        decision(&report, ROW)
    );
    assert!(output.is_none());
}
