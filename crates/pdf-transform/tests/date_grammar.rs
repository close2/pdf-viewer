//! ISO 32000-2 §7.9.4's date format, checked on what metadata editing writes by a reading of the
//! clause that shares no code with the writer.
//!
//! `pdf_syntax::date::Date::parse` is the writer's own validation, so asking it whether an output
//! is valid would let one regression in it bless both sides. [`conforms`] is written from the
//! clause alone: the prefix, the year, then each later field only "if all of their preceding
//! fields are also present", each in its stated range, and the apostrophe and the offset minutes
//! only after the offset hours. The one departure is the clause's own: its example,
//! `D:199812231952-08'00`, states an offset with no seconds before it.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "valid fixtures and successful controls are prerequisites; positions inside a date \
              string of a few bytes"
)]

mod support;

use pdf_model::metadata::Information;
use pdf_syntax::Document;
use pdf_transform::update::{Edit, InfoEntry, UpdatePlan};
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Source, apply};

/// Two decimal digits at `at`, if they are there and their value is at most `most`.
fn field(bytes: &[u8], at: usize, least: u32, most: u32) -> Option<u32> {
    let pair = bytes.get(at..at.checked_add(2)?)?;
    if !pair.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let value = pair
        .iter()
        .fold(0, |sum, digit| sum * 10 + u32::from(digit - b'0'));
    (least..=most).contains(&value).then_some(value)
}

/// Whether `text` is a date §7.9.4 admits, `(D: YYYYMMDDHHmmSSOHH'mm )` with every field after the
/// year optional only from the end, bar the seconds before an offset, which the clause's example
/// omits.
fn conforms(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("D:") else {
        return false;
    };
    let bytes = rest.as_bytes();
    if bytes.len() < 4 || !bytes.iter().take(4).all(u8::is_ascii_digit) {
        return false;
    }
    // MM 01-12, DD 01-31, HH 00-23, mm 00-59, SS 00-59, each present only after the one before.
    // The clause's own example, `D:199812231952-08'00`, states the offset straight after the
    // minutes, so the seconds are the one field the offset may follow the absence of.
    let ranges = [(1, 12), (1, 31), (0, 23), (0, 59), (0, 59)];
    let mut at = 4;
    for (position, (least, most)) in ranges.into_iter().enumerate() {
        if at == bytes.len() {
            return true;
        }
        if position == 4 && matches!(bytes.get(at), Some(b'+' | b'-' | b'Z')) {
            break;
        }
        if field(bytes, at, least, most).is_none() {
            return false;
        }
        at += 2;
    }
    if at == bytes.len() {
        return true;
    }
    if !matches!(bytes.get(at), Some(b'+' | b'-' | b'Z')) {
        return false;
    }
    at += 1;
    if at == bytes.len() {
        return true;
    }
    if field(bytes, at, 0, 23).is_none() {
        return false;
    }
    at += 2;
    if at == bytes.len() {
        return true;
    }
    if bytes.get(at) != Some(&b'\'') {
        return false;
    }
    at += 1;
    if at == bytes.len() {
        return true;
    }
    field(bytes, at, 0, 59).is_some() && at + 2 == bytes.len()
}

/// What a `/ModDate` edit wrote, or `None` where the edit was refused.
fn written(input: &str) -> Option<String> {
    let sinks = MemorySinks::new();
    apply(
        &Plan::Update(UpdatePlan {
            source: 0,
            edit: Edit::SetInformation {
                entries: vec![InfoEntry {
                    key: "ModDate".into(),
                    value: Some(input.into()),
                }],
            },
            names: "out.pdf".parse().expect("output pattern"),
        }),
        &[Source::new(support::with_metadata("1.7", None, None))],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .ok()?;
    let (_, bytes) = sinks.into_outputs().pop().expect("one output");
    let document = Document::open(bytes).expect("written PDF parses");
    Information::read(&document).modified
}

#[test]
fn the_independent_reading_tells_the_clauses_example_from_its_violations() {
    // The clause's own example, and its partial forms.
    for valid in [
        "D:199812231952-08'00",
        "D:1998",
        "D:199812",
        "D:20260101000000Z",
    ] {
        assert!(conforms(valid), "{valid}");
    }
    for invalid in [
        "1998",
        "D:199",
        "D:19981301",
        "D:19981232",
        "D:1998122324",
        "D:199812231960",
        "D:2026010",
        "D:19981223195200-24'00",
        "D:19981223195200-08'60",
        "D:19981223195200-0800",
        "D:19981223195200X",
    ] {
        assert!(!conforms(invalid), "{invalid}");
    }
}

#[test]
fn every_date_metadata_editing_writes_conforms_to_the_clause() {
    let inputs = [
        "D:1998",
        "D:199812231952-08'00",
        "D:20260101000000+23'59",
        "D:20261301",
        "D:20260132",
        "D:2026010124",
        "D:2026010",
        "D:20260101000000-24'00",
        "D:20260101000000+05'60",
    ];
    for input in inputs {
        let Some(date) = written(input) else {
            continue;
        };
        assert!(
            conforms(&date),
            "{input:?} was written as {date:?}, which §7.9.4 does not admit"
        );
        if conforms(input) {
            assert_eq!(date, input, "a conforming date is written as given");
        }
    }
}
