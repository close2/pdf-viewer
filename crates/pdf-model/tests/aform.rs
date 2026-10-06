//! `pdf_model::aform` — the `AF*` form library — held to Adobe's documented examples, and to the
//! choices ADR 1578 names where Adobe documents none.
//!
//! **Where each expected value comes from**, which `CLAUDE.md` principle 5 asks of every test.
//! Adobe documents none of the `AF*` functions as algorithms. What it documents, at
//! `adobe/dc-acrobat-sdk-docs` commit `ab3b42a7`, is:
//!
//! - the *JavaScript for Acrobat API Reference*, "util methods": `printd`'s table of place-holders,
//!   each with an example — one coherent moment, Wednesday the third of September 1997 at
//!   08 minutes and 05 seconds past nine in the morning — `scand`'s two-digit-year horizon,
//!   `printx`'s masking characters and its telephone-number example, and `printf`'s `%.2f` of
//!   `Math.PI * 100`;
//! - the *Interapplication Communication API Reference*, `Field.SetJavaScriptAction`: the four time
//!   formats each with an example, the fourteen date pictures, the four special formats, the
//!   negative styles and `AFSimple_Calculate`'s five functions.
//!
//! Those examples are the fixtures marked **Adobe's example**. Every other expected value follows
//! from a rule ADR 1578 writes down as a choice, and its test says which section.
//!
//! `/AA` dispatch — Table 199's triggers raised over `ViewState`, Table 224's `/CO` walked — is the
//! second half of this file, on fixtures built here (ADR 1579).

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise what the test is about is a failure"
)]

use std::fmt::Write as _;

use pdf_model::aform::{
    Call, DateTime, Function, Keyed, Keystroke, Literal, NotOneCall, extract_nums, make_number,
    merge_change, parse_date, print_date, print_mask,
};
use pdf_model::view::{Committed, Entered, ViewState};
use pdf_syntax::{Document, ObjectId};

/// Parses a script that is one call, failing the test where it is not.
fn call(script: &str) -> Call {
    Call::parse(script).expect("the script is one call")
}

/// What a format call shows for a value.
fn shown(script: &str, value: &str) -> String {
    call(script).format(value).expect("the format runs").text
}

/// Whether a keystroke call lets a whole text stand, typed or committed.
fn stands(script: &str, text: &str, will_commit: bool) -> bool {
    let event = if will_commit {
        Keystroke {
            value: text,
            change: "",
            selection: (text.len(), text.len()),
            will_commit,
        }
    } else {
        Keystroke {
            value: "",
            change: text,
            selection: (0, 0),
            will_commit,
        }
    };
    matches!(
        call(script).keystroke(&event).expect("the keystroke runs"),
        Keyed::Accepted { .. }
    )
}

/// What a keystroke call commits a text as, where it rewrites it.
fn committed(script: &str, text: &str) -> Option<String> {
    let event = Keystroke {
        value: text,
        change: "",
        selection: (text.len(), text.len()),
        will_commit: true,
    };
    match call(script).keystroke(&event).expect("the keystroke runs") {
        Keyed::Accepted { value } => value,
        Keyed::Rejected { .. } => None,
    }
}

/// The moment `printd`'s table illustrates every place-holder with.
const PRINTD_MOMENT: DateTime = DateTime {
    year: 1997,
    month: 9,
    day: 3,
    hour: 9,
    minute: 8,
    second: 5,
};

// --- the grammar: one call with literal arguments, and nothing else (ADR 1579) ---------------

#[test]
fn every_function_the_library_defines_is_named_back() {
    assert_eq!(Function::ALL.len(), 21);
    for function in Function::ALL {
        assert_eq!(Function::from_name(function.name()), Some(function));
    }
}

#[test]
fn one_call_is_read_with_white_space_both_quotes_and_a_trailing_semicolon() {
    let read = call("  AFNumber_Format( 2, 0,0 ,0, \"$\", true ) ;\n");
    assert_eq!(read.function, Function::NumberFormat);
    assert_eq!(
        read.arguments,
        vec![
            Literal::Number(2.0),
            Literal::Number(0.0),
            Literal::Number(0.0),
            Literal::Number(0.0),
            Literal::String("$".to_owned()),
            Literal::Boolean(true),
        ]
    );
    assert_eq!(
        call("AFDate_FormatEx('mm/dd/yyyy')").arguments,
        vec![Literal::String("mm/dd/yyyy".to_owned())]
    );
    assert_eq!(
        Call::parse("AFSimple_Calculate(\"SUM\", new_list)"),
        Err(NotOneCall::Shape),
        "an identifier argument is not a literal"
    );
}

#[test]
fn an_array_literal_and_escapes_are_literals() {
    assert_eq!(
        call("AFSimple_Calculate(\"SUM\", [\"Line.1\", 'Line.2',])").arguments,
        vec![
            Literal::String("SUM".to_owned()),
            Literal::Array(vec!["Line.1".to_owned(), "Line.2".to_owned()]),
        ]
    );
    assert_eq!(
        call(r#"AFNumber_Format(2, 0, 0, 0, "\u20ac ", false)"#)
            .arguments
            .get(4),
        Some(&Literal::String("\u{20ac} ".to_owned()))
    );
    assert_eq!(
        call(r"AFNumber_Format(-1)").arguments,
        vec![Literal::Number(-1.0)]
    );
}

#[test]
fn anything_but_one_call_is_not_one_call() {
    for script in [
        "if (event.value > 0) AFNumber_Format(2, 0, 0, 0, \"\", true);",
        "AFNumber_Format(2, 0, 0, 0, \"\", true); AFDate_FormatEx(\"m/d\");",
        "AFNumber_Format(2, 0, 0, 0, \"\", true) // two places",
        "AFRange_Validate(true, x, true, 100)",
        "event.value = 1",
        "",
    ] {
        assert_eq!(Call::parse(script), Err(NotOneCall::Shape), "{script}");
    }
    assert_eq!(
        Call::parse("AFExactMatch(1)"),
        Err(NotOneCall::Unknown("AFExactMatch".to_owned()))
    );
}

#[test]
fn a_function_at_a_trigger_it_does_not_serve_refuses_with_a_sentence() {
    let refusal = call("AFNumber_Keystroke(2, 0, 0, 0, \"\", true)")
        .format("1")
        .expect_err("a keystroke function is not a format");
    assert!(
        refusal.sentence().contains("keystroke function"),
        "{refusal}"
    );
}

// --- util.printd, util.scand, util.printx: Adobe's examples ---------------------------------

/// Adobe's example: every row of `printd`'s place-holder table, on the one moment the examples
/// describe.
#[test]
fn printd_writes_every_place_holder_as_its_table_shows() {
    for (picture, expected) in [
        ("mmmm", "September"),
        ("mmm", "Sep"),
        ("mm", "09"),
        ("m", "9"),
        ("dddd", "Wednesday"),
        ("ddd", "Wed"),
        ("dd", "03"),
        ("d", "3"),
        ("yyyy", "1997"),
        ("yy", "97"),
        ("HH", "09"),
        ("H", "9"),
        ("hh", "09"),
        ("h", "9"),
        ("MM", "08"),
        ("M", "8"),
        ("ss", "05"),
        ("s", "5"),
        ("tt", "am"),
        ("t", "a"),
    ] {
        assert_eq!(
            print_date(picture, &PRINTD_MOMENT).expect("a picture"),
            expected,
            "{picture}"
        );
    }
    assert_eq!(
        print_date("\\mmmm", &PRINTD_MOMENT).expect("a picture"),
        "mSep",
        "the escape takes one character"
    );
    assert!(
        print_date("jj", &PRINTD_MOMENT).is_err(),
        "the deprecated era is refused"
    );
}

/// Adobe's example: `scand`'s date horizon — below 50 is the twenty-first century.
#[test]
fn scand_reads_a_two_digit_year_through_the_horizon() {
    let year = |text: &str| parse_date(text, "m/d/yy").expect("a date").year;
    assert_eq!(year("1/2/49"), 2049);
    assert_eq!(year("1/2/50"), 1950);
    assert_eq!(year("1/2/00"), 2000);
    // `scand`'s first example: a date written through `mm/dd/yyyy` reads back through it.
    let written = print_date("mm/dd/yyyy", &PRINTD_MOMENT).expect("a picture");
    let read = parse_date(&written, "mm/dd/yyyy").expect("it reads back");
    assert_eq!((read.year, read.month, read.day), (1997, 9, 3));
}

/// Adobe's example: `printx`'s telephone number, pulled out of letters on either side.
#[test]
fn printx_writes_the_telephone_number_its_example_shows() {
    assert_eq!(
        print_mask("9 (999) 999-9999", "aaa14159697489zzz"),
        "1 (415) 969-7489"
    );
    assert_eq!(print_mask("*", "as is"), "as is");
    assert_eq!(print_mask(">?<?=?", "aBc"), "Abc");
}

// --- AFTime: the Interapplication reference's four examples ---------------------------------

/// Adobe's example: `24HR_MM [ 14:30 ]`, `12HR_MM [ 2:30 PM ]`, `24HR_MM_SS [ 14:30:15 ]`,
/// `12HR_MM_SS [ 2:30:15 PM ]`.
#[test]
fn aftime_format_writes_the_menus_four_examples() {
    assert_eq!(shown("AFTime_Format(0)", "14:30"), "14:30");
    assert_eq!(shown("AFTime_Format(1)", "14:30"), "2:30 PM");
    assert_eq!(shown("AFTime_Format(2)", "14:30:15"), "14:30:15");
    assert_eq!(shown("AFTime_Format(3)", "14:30:15"), "2:30:15 PM");
    assert_eq!(shown("AFTime_Format(1)", "2:30 pm"), "2:30 PM");
    // ADR 1578 section 6: the `Ex` form writes `printd`'s own lower-case marker.
    assert_eq!(shown("AFTime_FormatEx(\"h:MM tt\")", "14:30"), "2:30 pm");
}

#[test]
fn aftime_keystroke_judges_a_time_at_commit() {
    assert!(
        stands("AFTime_Keystroke(0)", "14:3", false),
        "typing is not judged"
    );
    assert!(stands("AFTime_Keystroke(0)", "14:30", true));
    assert!(!stands("AFTime_Keystroke(0)", "25:30", true));
    assert!(stands("AFTime_KeystrokeEx(\"h:MM tt\")", "2:30 PM", true));
    assert!(!stands("AFTime_KeystrokeEx(\"h:MM tt\")", "13:30 PM", true));
}

// --- AFDate: the fourteen pictures, read and written (ADR 1578 section 6) -------------------

#[test]
fn afdate_format_names_the_fourteen_pictures_by_index_and_by_text() {
    assert_eq!(shown("AFDate_Format(2)", "1/5/24"), "01/05/24");
    assert_eq!(shown("AFDate_Format(\"mm/dd/yy\")", "1/5/24"), "01/05/24");
    assert_eq!(
        shown("AFDate_Format(11)", "January 5, 2024"),
        "January 5, 2024"
    );
    assert_eq!(shown("AFDate_Format(10)", "jan 5 2024"), "Jan 5, 2024");
    assert_eq!(shown("AFDate_Format(5)", "5-Jan-24"), "5-Jan-24");
    assert_eq!(
        shown("AFDate_Format(12)", "1/5/24 2:30 pm"),
        "1/5/24 2:30 pm"
    );
    assert!(
        call("AFDate_Format(14)").format("1/5/24").is_err(),
        "a fifteenth picture"
    );
}

#[test]
fn afdate_formatex_reads_and_writes_its_own_picture() {
    assert_eq!(
        shown("AFDate_FormatEx(\"mm/dd/yyyy\")", "1/5/2024"),
        "01/05/2024"
    );
    assert_eq!(
        shown("AFDate_FormatEx(\"mm/dd/yyyy\")", "1-5-2024"),
        "01/05/2024"
    );
    assert_eq!(
        shown("AFDate_FormatEx(\"yyyymmdd\")", "20240105"),
        "20240105"
    );
    assert_eq!(
        shown(
            "AFDate_FormatEx(\"dddd, mmmm d, yyyy\")",
            "Friday, January 5, 2024"
        ),
        "Friday, January 5, 2024"
    );
    // ADR 1578 section 1: a value the picture does not read is shown as it stands.
    assert_eq!(shown("AFDate_FormatEx(\"mm/dd/yyyy\")", "soon"), "soon");
    assert_eq!(shown("AFDate_FormatEx(\"mm/dd/yyyy\")", ""), "");
}

#[test]
fn afdate_keystroke_refuses_a_date_that_does_not_exist() {
    let ex = "AFDate_KeystrokeEx(\"mm/dd/yyyy\")";
    assert!(stands(ex, "02/29/2024", true));
    assert!(!stands(ex, "02/29/2023", true), "2023 is not a leap year");
    assert!(!stands(ex, "13/01/2024", true));
    assert!(stands(ex, "13/", false), "typing is not judged");
    assert!(
        stands(ex, "", true),
        "an empty field is not a date to refuse"
    );
    assert!(
        stands("AFDate_Keystroke(0)", "2/29", true),
        "no year reads as a leap year"
    );
}

#[test]
fn afparsedateex_is_the_reading_the_formats_use() {
    let read = parse_date("3:04:05 PM", "h:MM:ss tt").expect("a time");
    assert_eq!((read.hour, read.minute, read.second), (15, 4, 5));
    assert_eq!(parse_date("2024-01-05 trailing", "yyyy-mm-dd"), None);
    assert_eq!(
        parse_date("24-1-5", "yyyy-mm-dd").map(|d| d.year),
        Some(2024)
    );
    assert_eq!(
        parse_date("124-1-5", "yyyy-mm-dd"),
        None,
        "three digits are no year"
    );
}

// --- AFNumber and AFPercent (ADR 1578 sections 1–3) ------------------------------------------

/// Adobe's example: `util.printf("%.2f", Math.PI * 100)` writes `314.16`, which is the
/// separator style 1 conversion `AFNumber_Format` is written with.
#[test]
fn afnumber_format_rounds_as_printf_does_on_its_example() {
    assert_eq!(
        shown(
            "AFNumber_Format(2, 1, 0, 0, \"\", false)",
            "314.159265358979"
        ),
        "314.16"
    );
}

#[test]
fn afnumber_format_writes_each_separator_style() {
    let at = |style: u32| {
        shown(
            &format!("AFNumber_Format(2, {style}, 0, 0, \"\", false)"),
            "1234567.891",
        )
    };
    assert_eq!(at(0), "1,234,567.89");
    assert_eq!(at(1), "1234567.89");
    assert_eq!(at(2), "1.234.567,89");
    assert_eq!(at(3), "1234567,89");
    assert_eq!(at(4), "1'234'567.89");
    assert!(
        call("AFNumber_Format(2, 5, 0, 0, \"\", false)")
            .format("1")
            .is_err(),
        "a sixth style is refused by name"
    );
}

#[test]
fn afnumber_format_writes_each_negative_style_and_currency_placement() {
    let at = |negative: u32, prepend: bool| {
        call(&format!(
            "AFNumber_Format(2, 0, {negative}, 0, \"$\", {prepend})"
        ))
        .format("-1234.5")
        .expect("the format runs")
    };
    assert_eq!(
        (at(0, true).text, at(0, true).red),
        ("-$1,234.50".to_owned(), false)
    );
    assert_eq!(
        (at(1, true).text, at(1, true).red),
        ("$1,234.50".to_owned(), true)
    );
    assert_eq!(
        (at(2, true).text, at(2, true).red),
        ("($1,234.50)".to_owned(), false)
    );
    assert_eq!(
        (at(3, true).text, at(3, true).red),
        ("($1,234.50)".to_owned(), true)
    );
    assert_eq!(at(0, false).text, "-1,234.50$");
    assert_eq!(
        shown("AFNumber_Format(2, 0, 0, 0, \"$\", true)", "1234.5"),
        "$1,234.50"
    );
    assert_eq!(
        shown(
            "AFNumber_Format(2, 0, 0, 0, \" \u{20ac}\", false)",
            "1234.5"
        ),
        "1,234.50 \u{20ac}"
    );
    // A negative that rounds to zero is written without its sign.
    assert_eq!(
        shown("AFNumber_Format(2, 0, 0, 0, \"\", false)", "-0.001"),
        "0.00"
    );
    // Decimal rounding, half away from zero, on what was typed.
    assert_eq!(
        shown("AFNumber_Format(2, 0, 0, 0, \"\", false)", "1.005"),
        "1.01"
    );
    assert_eq!(
        shown("AFNumber_Format(0, 0, 0, 0, \"\", false)", "-2.5"),
        "-3"
    );
    // A value that states no number is shown as it stands; an empty one is empty.
    assert_eq!(
        shown("AFNumber_Format(2, 0, 0, 0, \"$\", true)", "n/a"),
        "n/a"
    );
    assert_eq!(shown("AFNumber_Format(2, 0, 0, 0, \"$\", true)", ""), "");
}

#[test]
fn afnumber_keystroke_accepts_every_prefix_of_a_number_and_nothing_else() {
    let dot = "AFNumber_Keystroke(2, 0, 0, 0, \"\", true)";
    for typed in ["", "-", "1", "12.", "-12.5", ".5", "+3"] {
        assert!(stands(dot, typed, false), "{typed}");
    }
    for typed in ["12a", "1.2.3", "1,5", "$1"] {
        assert!(!stands(dot, typed, false), "{typed}");
    }
    assert!(!stands(dot, "-", true), "a sign alone is no number");
    assert!(stands(dot, "12.", true));
    let comma = "AFNumber_Keystroke(2, 2, 0, 0, \"\", true)";
    assert!(stands(comma, "1,5", false));
    assert!(
        stands(comma, "1.5", false),
        "a comma style reads back the period its own commit stored"
    );
    assert!(!stands(comma, "1,5.0", false), "one decimal point");
    assert_eq!(committed(comma, "1,5").as_deref(), Some("1.5"));
}

#[test]
fn afpercent_writes_a_hundredfold_value_with_its_sign() {
    assert_eq!(shown("AFPercent_Format(2, 0)", "0.1234"), "12.34%");
    assert_eq!(shown("AFPercent_Format(0, 0)", "12.5"), "1,250%");
    assert_eq!(shown("AFPercent_Format(1, 0, true)", "0.5"), "%50.0");
    assert_eq!(shown("AFPercent_Format(1, 0)", "-0.5"), "-50.0%");
    assert!(stands("AFPercent_Keystroke(2, 0)", "12.5", true));
    assert!(!stands("AFPercent_Keystroke(2, 0)", "12%", false));
}

// --- AFSpecial (ADR 1578 section 7) ---------------------------------------------------------

#[test]
fn afspecial_format_writes_the_four_masks() {
    assert_eq!(shown("AFSpecial_Format(0)", "12345"), "12345");
    assert_eq!(shown("AFSpecial_Format(1)", "123456789"), "12345-6789");
    assert_eq!(shown("AFSpecial_Format(2)", "4155551234"), "(415) 555-1234");
    assert_eq!(shown("AFSpecial_Format(2)", "5551234"), "555-1234");
    assert_eq!(shown("AFSpecial_Format(3)", "123456789"), "123-45-6789");
    assert!(call("AFSpecial_Format(4)").format("1").is_err());
}

#[test]
fn afspecial_keystroke_counts_digits_and_admits_the_masks_punctuation() {
    assert!(stands("AFSpecial_Keystroke(0)", "1234", false));
    assert!(!stands("AFSpecial_Keystroke(0)", "1234", true));
    assert!(stands("AFSpecial_Keystroke(0)", "12345", true));
    assert!(!stands("AFSpecial_Keystroke(0)", "123456", false));
    assert!(stands("AFSpecial_Keystroke(1)", "12345-6789", true));
    assert!(stands("AFSpecial_Keystroke(2)", "(415) 555-1234", true));
    assert!(stands("AFSpecial_Keystroke(2)", "555-1234", true));
    assert!(!stands("AFSpecial_Keystroke(2)", "55512345", true));
    assert!(stands("AFSpecial_Keystroke(3)", "123-45-6789", true));
    assert!(!stands("AFSpecial_Keystroke(3)", "123-45-678a", false));
}

#[test]
fn afspecial_keystrokeex_reads_an_arbitrary_mask() {
    let mask = "AFSpecial_KeystrokeEx(\"AA-9999\")";
    assert!(stands(mask, "AB-12", false));
    assert!(!stands(mask, "A1", false));
    assert!(stands(mask, "AB-1234", true));
    assert_eq!(committed(mask, "AB1234").as_deref(), Some("AB-1234"));
    assert!(!stands(mask, "AB-123", true));
    assert!(stands("AFSpecial_KeystrokeEx(\"OOX\")", "a1%", true));
}

// --- AFRange_Validate (ADR 1578 section 4) --------------------------------------------------

#[test]
fn afrange_validate_holds_inclusive_bounds() {
    let range = call("AFRange_Validate(true, 0, true, 100)");
    let judged = |value: &str| range.validate(value).expect("the validation runs");
    assert_eq!(judged("50"), None);
    assert_eq!(judged("0"), None);
    assert_eq!(judged("100"), None);
    assert!(judged("100.5").is_some());
    assert!(judged("-1").is_some());
    assert_eq!(judged(""), None, "an empty value is not a number to judge");
    assert_eq!(judged("abc"), None, "nor is text");
    let floor = call("AFRange_Validate(true, 10, false, 0)");
    assert!(floor.validate("9").expect("runs").is_some());
    assert_eq!(floor.validate("1000").expect("runs"), None);
}

// --- AFSimple_Calculate (ADR 1578 section 5) -------------------------------------------------

/// Runs a calculation over a table of field names and values.
fn calculated(script: &str, fields: &[(&str, &str)]) -> String {
    let mut values = |listed: &str| {
        fields
            .iter()
            .filter(|(name, _)| {
                *name == listed
                    || name
                        .strip_prefix(listed)
                        .is_some_and(|rest| rest.starts_with('.'))
            })
            .map(|(_, value)| (*value).to_owned())
            .collect()
    };
    call(script)
        .calculate(&mut values)
        .expect("the calculation runs")
}

#[test]
fn afsimple_calculate_runs_the_five_functions_of_its_menu() {
    let fields = [("A", "1.5"), ("B", "2"), ("C", "")];
    assert_eq!(
        calculated(
            "AFSimple_Calculate(\"SUM\", [\"A\", \"B\", \"C\"])",
            &fields
        ),
        "3.5"
    );
    assert_eq!(
        calculated("AFSimple_Calculate(\"PRD\", \"A, B\")", &fields),
        "3"
    );
    assert_eq!(
        calculated("AFSimple_Calculate(\"AVG\", \"A, B, C\")", &fields),
        "1.16666666666667",
        "the mean of three, blanks counted, to fifteen significant digits"
    );
    assert_eq!(
        calculated("AFSimple_Calculate(\"MIN\", \"A, B, C\")", &fields),
        "0",
        "a blank is zero"
    );
    assert_eq!(
        calculated("AFSimple_Calculate(\"MAX\", \"A, B\")", &fields),
        "2"
    );
    assert_eq!(
        calculated("AFSimple_Calculate(\"SUM\", \"nothing\")", &fields),
        "0"
    );
    assert!(
        call("AFSimple_Calculate(\"sum\", \"A\")")
            .calculate(&mut |_| Vec::new())
            .is_err()
    );
}

#[test]
fn afsimple_calculate_writes_decimal_sums_as_decimals() {
    let fields = [("A", "0.1"), ("B", "0.2")];
    assert_eq!(
        calculated("AFSimple_Calculate(\"SUM\", \"A, B\")", &fields),
        "0.3"
    );
    let lines = [("Line.1", "10"), ("Line.2", "20.25"), ("Lines", "1000")];
    assert_eq!(
        calculated("AFSimple_Calculate(\"SUM\", \"Line\")", &lines),
        "30.25",
        "a name with descendants names every terminal field below it, and only those"
    );
}

// --- the helpers (ADR 1578 section 1) --------------------------------------------------------

#[test]
fn the_helpers_read_numbers_and_merge_changes() {
    assert_eq!(make_number(" -12.5 "), Some(-12.5));
    assert_eq!(make_number("12,5"), Some(12.5));
    assert_eq!(make_number("1e3"), Some(1000.0));
    assert_eq!(make_number("$12"), None);
    assert_eq!(make_number("1,234.5"), None);
    assert_eq!(
        extract_nums("a12b3"),
        Some(vec!["12".to_owned(), "3".to_owned()])
    );
    assert_eq!(
        extract_nums(".5"),
        Some(vec!["0".to_owned(), "5".to_owned()])
    );
    assert_eq!(extract_nums("none"), None);
    let event = Keystroke {
        value: "12345",
        change: "x",
        selection: (1, 3),
        will_commit: false,
    };
    assert_eq!(merge_change(&event), "1x45");
    assert_eq!(
        merge_change(&Keystroke {
            will_commit: true,
            ..event
        }),
        "12345"
    );
}

// --- the dispatch over ViewState (ADR 1579) ---------------------------------------------------

/// A one-page document whose objects are given from 4 on, with an interactive form dictionary
/// whose body is `form`.
fn form_document(form: &str, objects: &[&str]) -> Document {
    let mut bodies = vec![
        format!("<< /Type /Catalog /Pages 2 0 R /AcroForm {form} >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
    ];
    let annots: Vec<String> = (0..objects.len())
        .map(|index| format!("{} 0 R", index.saturating_add(4)))
        .collect();
    bodies.push(format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [{}] >>",
        annots.join(" ")
    ));
    bodies.extend(objects.iter().map(|body| (*body).to_owned()));
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = bodies.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    Document::open(out.into_bytes()).expect("the fixture opens")
}

/// A text field widget with a `/AA` holding `actions`.
fn text_field(name: &str, rect: &str, actions: &str) -> String {
    format!(
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T ({name}) /Rect [{rect}] \
         /DA (/Helv 10 Tf 0 g) /AA << {actions} >> >>"
    )
}

/// The decoded content of a widget's saved normal appearance.
fn saved_appearance(view: &ViewState, document: &Document, widget: u32) -> String {
    let bytes = view.save(document).expect("the update writes").bytes;
    let saved = Document::open(bytes).expect("the update reads back");
    let object = saved.get(ObjectId {
        number: widget,
        generation: 0,
    });
    let dict = object.as_dict().expect("a widget");
    let appearances = saved.get_key(dict, "AP");
    let normal = appearances
        .as_dict()
        .map(|appearances| saved.get_key(appearances, "N"))
        .expect("an /AP");
    let stream = normal.as_stream().expect("a stream");
    String::from_utf8_lossy(&saved.decoded_stream_data(stream).expect("decodes")).into_owned()
}

/// The `/V` a saved widget states, as text.
fn saved_value(view: &ViewState, document: &Document, widget: u32) -> String {
    let bytes = view.save(document).expect("the update writes").bytes;
    let saved = Document::open(bytes).expect("the update reads back");
    let object = saved.get(ObjectId {
        number: widget,
        generation: 0,
    });
    let dict = object.as_dict().expect("a widget");
    saved
        .get_key(dict, "V")
        .as_string()
        .map(pdf_syntax::text_string)
        .unwrap_or_default()
}

const CURRENCY: &str = "/F << /S /JavaScript /JS (AFNumber_Format\\(2, 0, 0, 0, \"$\", true\\);) >> \
                        /K << /S /JavaScript /JS (AFNumber_Keystroke\\(2, 0, 0, 0, \"$\", true\\);) >>";

#[test]
fn a_format_script_shapes_the_appearance_a_save_writes() {
    let document = form_document(
        "<< /Fields [4 0 R] >>",
        &[&text_field("Price", "10 10 210 40", CURRENCY)],
    );
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.set_field(&document, "Price", &Entered::Text("1234.5".to_owned())),
        1
    );
    assert!(
        view.annotation(ObjectId {
            number: 4,
            generation: 0
        })
        .editing,
        "typed, not committed"
    );
    assert_eq!(view.commit_field(&document, "Price"), Committed::Accepted);
    assert!(
        !view
            .annotation(ObjectId {
                number: 4,
                generation: 0
            })
            .editing
    );
    let content = saved_appearance(&view, &document, 4);
    assert!(content.contains("($1,234.50) Tj"), "{content}");
    assert_eq!(
        saved_value(&view, &document, 4),
        "1234.5",
        "the value is the number, not its format"
    );
}

#[test]
fn a_keystroke_script_refuses_characters_that_are_no_number() {
    let document = form_document(
        "<< /Fields [4 0 R] >>",
        &[&text_field("Price", "10 10 210 40", CURRENCY)],
    );
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.set_field(&document, "Price", &Entered::Text("12".to_owned())),
        1
    );
    assert_eq!(
        view.set_field(&document, "Price", &Entered::Text("12a".to_owned())),
        0,
        "Table 199's /K may reject the added text"
    );
    assert_eq!(
        view.field_value(&document, "Price")
            .map(|shown| shown.text)
            .as_deref(),
        Some("12")
    );
}

#[test]
fn a_refused_commit_puts_back_what_the_field_showed() {
    let document = form_document(
        "<< /Fields [4 0 R] >>",
        &[&text_field(
            "Score",
            "10 10 210 40",
            "/V << /S /JavaScript /JS (AFRange_Validate\\(true, 0, true, 100\\)) >>",
        )],
    );
    let mut view = ViewState::of(&document);
    view.set_field(&document, "Score", &Entered::Text("50".to_owned()));
    assert_eq!(view.commit_field(&document, "Score"), Committed::Accepted);
    view.set_field(&document, "Score", &Entered::Text("150".to_owned()));
    let Committed::Refused(sentence) = view.commit_field(&document, "Score") else {
        panic!("a value over the bound is refused");
    };
    assert!(sentence.starts_with("Score: "), "{sentence}");
    assert_eq!(
        view.field_value(&document, "Score")
            .map(|shown| shown.text)
            .as_deref(),
        Some("50"),
        "the committed value stands"
    );
    assert_eq!(view.commit_field(&document, "Score"), Committed::Nothing);
}

#[test]
fn the_calculation_order_is_walked_once_per_change() {
    let sum = "/C << /S /JavaScript /JS (AFSimple_Calculate\\(\"SUM\", [\"Line.1\", \"Line.2\"]\\)) >> \
               /F << /S /JavaScript /JS (AFNumber_Format\\(2, 0, 0, 0, \"$\", true\\)) >>";
    let double = "/C << /S /JavaScript /JS (AFSimple_Calculate\\(\"SUM\", \"Total, Total\"\\)) >>";
    let document = form_document(
        "<< /Fields [4 0 R 5 0 R 6 0 R 7 0 R] /CO [6 0 R 7 0 R] >>",
        &[
            &text_field("Line.1", "10 10 110 30", ""),
            &text_field("Line.2", "10 40 110 60", ""),
            &format!(
                "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Total) /Ff 1 /Rect [10 70 110 90] \
                 /DA (/Helv 10 Tf 0 g) /AA << {sum} >> >>"
            ),
            &text_field("Twice", "10 100 110 120", double),
        ],
    );
    let mut view = ViewState::of(&document);
    view.set_field(&document, "Line.1", &Entered::Text("10".to_owned()));
    view.set_field(&document, "Line.2", &Entered::Text("2.5".to_owned()));
    let value =
        |view: &ViewState, name: &str| view.field_value(&document, name).map(|shown| shown.text);
    assert_eq!(
        value(&view, "Total").as_deref(),
        Some("12.5"),
        "read-only binds a person, not a calculation"
    );
    assert_eq!(
        value(&view, "Twice").as_deref(),
        Some("25"),
        "the order runs Total before Twice"
    );
    let content = saved_appearance(&view, &document, 6);
    assert!(content.contains("($12.50) Tj"), "{content}");
    assert_eq!(saved_value(&view, &document, 6), "12.5");
}

#[test]
fn a_script_this_tier_does_not_run_is_reported_once() {
    let document = form_document(
        "<< /Fields [4 0 R] >>",
        &[&text_field(
            "Name",
            "10 10 210 40",
            "/K << /S /JavaScript /JS (if \\(event.willCommit\\) event.value = 1;) >>",
        )],
    );
    let mut view = ViewState::of(&document);
    view.set_field(&document, "Name", &Entered::Text("A".to_owned()));
    view.set_field(&document, "Name", &Entered::Text("Ab".to_owned()));
    assert_eq!(
        view.script_reports().len(),
        1,
        "{:?}",
        view.script_reports()
    );
    let sentence = view.script_reports().first().expect("one report");
    assert!(
        sentence.contains("a script this tier does not run"),
        "{sentence}"
    );
    assert_eq!(
        view.field_value(&document, "Name")
            .map(|shown| shown.text)
            .as_deref(),
        Some("Ab"),
        "a script nobody ran refused nothing"
    );
}

#[test]
fn a_stored_value_is_drawn_through_its_format_when_the_appearance_is_constructed() {
    let document = form_document(
        "<< /Fields [4 0 R 5 0 R] >>",
        &[
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Due) /V (1/5/2024) \
             /Rect [10 10 210 40] /DA (/Helv 10 Tf 0 g) \
             /AA << /F << /S /JavaScript /JS (AFDate_FormatEx\\(\"mmmm d, yyyy\"\\)) >> >> >>",
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Price) /V (-1234.5) \
             /Rect [10 50 210 80] /DA (/Helv 10 Tf 0 g) \
             /AA << /F << /S /JavaScript /JS (AFNumber_Format\\(2, 0, 3, 0, \"$\", true\\)) >> >> >>",
        ],
    );
    let drawn = |number: u32| {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        let widget = object.as_dict().expect("a widget");
        let written = pdf_model::appearance::for_annotation(&document, widget).expect("built");
        let stream = written.stream.as_stream().expect("a form XObject");
        String::from_utf8_lossy(&stream.data).into_owned()
    };
    // "1/5/2024" does not follow "mmmm d, yyyy", so it is drawn as it stands (ADR 1578 section 1).
    let due = drawn(4);
    assert!(due.contains("(1/5/2024) Tj"), "{due}");
    // ParensRed: in parentheses, and the `/DA`'s colour followed by red (ADR 1578 section 2).
    let price = drawn(5);
    assert!(
        price.contains("(\\($1,234.50\\)) Tj") || price.contains("(($1,234.50)) Tj"),
        "{price}"
    );
    assert!(price.contains("1 0 0 rg"), "{price}");
}

#[test]
fn a_keystroke_script_names_a_value_it_accepts() {
    let example = |script: &str| call(script).accepted_example();
    assert_eq!(
        example("AFNumber_Keystroke(2, 0, 0, 0, \"\", true)").as_deref(),
        Some("1234.5")
    );
    assert_eq!(
        example("AFNumber_Keystroke(2, 2, 0, 0, \"\", true)").as_deref(),
        Some("1234.5")
    );
    assert_eq!(
        example("AFPercent_Keystroke(2, 0)").as_deref(),
        Some("1234.5")
    );
    assert_eq!(
        example("AFDate_KeystrokeEx(\"dd.mm.yyyy\")").as_deref(),
        Some("05.01.2024")
    );
    assert_eq!(
        example("AFDate_Keystroke(11)").as_deref(),
        Some("January 5, 2024")
    );
    assert_eq!(example("AFTime_Keystroke(1)").as_deref(), Some("2:30 pm"));
    assert_eq!(
        example("AFSpecial_Keystroke(2)").as_deref(),
        Some("4155551234")
    );
    assert_eq!(
        example("AFSpecial_KeystrokeEx(\"AA-9999\")").as_deref(),
        Some("AA-1111")
    );
    assert_eq!(
        example("AFNumber_Format(2, 0, 0, 0, \"\", true)"),
        None,
        "not a keystroke"
    );
}
