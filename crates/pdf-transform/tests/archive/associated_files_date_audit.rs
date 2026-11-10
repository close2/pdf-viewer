//! Regression for ISO 32000-2 §7.11.4.1 Table 45 and §14.13.2.
//!
//! The profile's `original =
//! "attach"` route is admitted only by PDF/A-4f/4e, so this exercises the PDF 2.0 archive writer
//! against the actual source-to-output path rather than the generic non-associated attachment
//! writer. The source is a syntactically valid PDF 1.7 with deliberately malformed XMP (so it is
//! not a conforming PDF/A source); the tested obligation is on the associated file the repair
//! writer emits, independent of the source defect that caused it to run.
#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a malformed fixture or an omitted associated-file date must fail loudly"
)]

use super::*;

const PROFILE: &str = include_str!("../../../../doc/profiles/only-metadata-loss.toml");
const PACKET: &str = "this is deliberately not an XMP packet";
const EXPECTED_SOURCE_DATE: &str = "D:20260101000000Z";

fn source(info: Option<&str>) -> Vec<u8> {
    Conforming {
        metadata: Packet::Stated(PACKET.to_owned()),
        // The deliberately malformed metadata is the defect this archival repair preserves.
        header: Some("%PDF-1.7"),
        info: info.map(str::to_owned),
        ..Conforming::default()
    }
    .build()
}

fn attached_mod_date(output: &[u8]) -> Option<String> {
    let document =
        Document::open_with_limits(output.to_vec(), Limits::DEFAULT).expect("output opens");
    assert!(
        !document.was_recovered(),
        "written output has a usable xref"
    );
    let catalog = document.catalog().expect("catalog");
    let associated = pdf_model::attachment::associated(&document, &catalog);
    let [file] = associated.as_slice() else {
        panic!("the original packet is associated with the catalog: {associated:?}");
    };
    assert_eq!(
        document
            .decoded_stream_data(&file.stream)
            .expect("preserved bytes decode")
            .as_ref(),
        PACKET.as_bytes()
    );
    assert_eq!(file.media_type.as_deref(), Some("application/octet-stream"));
    file.modified.clone()
}

fn convert_source(info: Option<&str>) -> Option<Vec<u8>> {
    let target = Target::Four(Flavour::F);
    let source = source(info);
    let parsed =
        Document::open_with_limits(source.clone(), Limits::DEFAULT).expect("valid PDF syntax");
    assert!(
        !parsed.was_recovered(),
        "the fixture has a valid xref table"
    );
    let plan = plan_from(PROFILE, target);
    let (_report, output) = convert_with_plan(&source, &plan);
    let Some(output) = output else {
        // A producer may decline this preservation route if it cannot supply a required date.
        return None;
    };
    assert!(output.starts_with(b"%PDF-2.0"), "the target writes PDF 2.0");
    Some(output)
}

#[test]
fn associated_archive_file_does_not_omit_the_table45_required_modification_date() {
    // Positive control before the disputed branch: the same repair route has a source date.
    let output = convert_source(Some(&format!("/ModDate ({EXPECTED_SOURCE_DATE})")))
        .expect("the writer has a source date and can preserve the packet");
    let written = attached_mod_date(&output).expect("known modification date is preserved");
    assert_eq!(
        pdf_syntax::Date::parse(&written).expect("written control date parses"),
        pdf_syntax::Date::parse(EXPECTED_SOURCE_DATE).expect("source control date parses")
    );

    // A producer may refuse when it cannot supply the required date; published output must
    // contain a valid date. This does not choose a source or invent a fallback timestamp.
    let Some(output) = convert_source(None) else {
        return;
    };
    let written =
        attached_mod_date(&output).expect("Table 45 requires ModDate for the catalog AF stream");
    assert!(
        pdf_syntax::Date::parse(&written).is_some(),
        "written ModDate obeys its date grammar"
    );
}
