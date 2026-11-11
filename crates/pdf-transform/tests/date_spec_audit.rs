//! ISO 32000-2 §7.9.4 applies to dates newly written by metadata editing.

#![expect(
    clippy::expect_used,
    reason = "valid fixtures and successful controls are prerequisites"
)]

mod support;

use pdf_model::metadata::Information;
use pdf_syntax::{Document, date::Date};
use pdf_transform::update::{Edit, InfoEntry, UpdatePlan};
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Source, apply};

fn source() -> Vec<u8> {
    let mut bytes = support::with_metadata("1.7", None, None);
    let short = b"/ID [<0102> <0304>]";
    let at = bytes
        .windows(short.len())
        .position(|part| part == short)
        .expect("fixture IDs");
    bytes.splice(
        at..at.saturating_add(short.len()),
        b"/ID [<00112233445566778899AABBCCDDEEFF> <FFEEDDCCBBAA99887766554433221100>]"
            .iter()
            .copied(),
    );
    let document = Document::open(bytes.clone()).expect("valid fixture");
    assert!(!document.was_recovered());
    bytes
}

/// A successful edit must not publish a date outside the clause's grammar and field ranges.
/// Refusing invalid input is allowed; this assertion does not prescribe a particular error type.
fn check_written_date(input: &str) -> bool {
    let sinks = MemorySinks::new();
    let result = apply(
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
        &[Source::new(source())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    );
    if result.is_err() {
        return false;
    }
    let outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1);
    let (_, bytes) = outputs.into_iter().next().expect("one output");
    let document = Document::open(bytes).expect("written PDF parses");
    assert!(!document.was_recovered());
    let information = Information::read(&document);
    let written = information
        .modified
        .expect("requested modification date is present");
    assert!(
        Date::parse(&written).is_some(),
        "metadata editing published an invalid date: {written:?}"
    );
    if Date::parse(input).is_some() {
        assert_eq!(
            written, input,
            "a valid date must retain the requested value, not merely become some valid date"
        );
    }
    true
}

#[test]
fn valid_partial_and_zoned_dates_can_be_written() {
    for date in ["D:1998", "D:199812231952-08'00", "D:20260101000000+23'59"] {
        assert!(
            check_written_date(date),
            "valid date {date} must reach the writer"
        );
    }
}

#[test]
fn metadata_editing_does_not_publish_an_out_of_range_month() {
    let _ = check_written_date("D:20261301");
}

#[test]
fn metadata_editing_does_not_publish_an_incomplete_numeric_field() {
    let _ = check_written_date("D:2026010");
}
