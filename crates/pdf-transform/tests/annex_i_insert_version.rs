//! Annex I.2 recommends the newer input version when a PDF's contents are inserted.

#![expect(
    clippy::expect_used,
    reason = "the fixtures must reach insertion, and unmet behavior must fail loudly"
)]

mod support;

use pdf_model::Pages;
use pdf_syntax::{Document, Version};
use pdf_transform::update::{Edit, UpdatePlan};
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Source, apply};

fn one_page_pdf(version: &str) -> Vec<u8> {
    let mut bytes = support::with_metadata(version, None, None);
    let short_id = b"/ID [<0102> <0304>]";
    let at = bytes
        .windows(short_id.len())
        .position(|window| window == short_id)
        .expect("fixture trailer has an /ID entry");
    let full_id = b"/ID [<00112233445566778899AABBCCDDEEFF> <FFEEDDCCBBAA99887766554433221100>]";
    bytes.splice(
        at..at.saturating_add(short_id.len()),
        full_id.iter().copied(),
    );
    bytes
}

fn insert_page(destination: &[u8], incoming: &[u8]) -> Vec<u8> {
    let sinks = MemorySinks::new();
    let sources = vec![
        Source::new(destination.to_vec()),
        Source::new(incoming.to_vec()),
    ];
    apply(
        &Plan::Update(UpdatePlan {
            source: 0,
            edit: Edit::InsertPages { from: 1, at: 2 },
            names: "out.pdf".parse().expect("a valid output name pattern"),
        }),
        &sources,
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("one page from the incoming PDF is inserted");
    sinks
        .into_outputs()
        .into_iter()
        .next()
        .expect("one updated PDF is written")
        .1
}

fn inserted_version(destination_version: &str, incoming_version: &str) -> Option<Version> {
    let destination = one_page_pdf(destination_version);
    let incoming = one_page_pdf(incoming_version);
    let destination_document =
        Document::open(destination.clone()).expect("the destination fixture is valid");
    let incoming_document =
        Document::open(incoming.clone()).expect("the incoming fixture is valid");
    assert!(!destination_document.was_recovered());
    assert!(!incoming_document.was_recovered());
    assert_eq!(Pages::new(&destination_document).len(), 1);
    assert_eq!(Pages::new(&incoming_document).len(), 1);

    let output = insert_page(&destination, &incoming);
    let updated = Document::open(output).expect("the update reads back");
    assert_eq!(
        Pages::new(&updated).len(),
        2,
        "the incoming page is present for destination {destination_version} and incoming {incoming_version}"
    );
    updated.version()
}

#[test]
fn inserting_an_older_pdf_keeps_the_destination_version() {
    // Annex I.2 says the inserted document's version is the maximum: a lower incoming version
    // must not lower the destination's stated version.
    assert_eq!(
        inserted_version("2.0", "1.7"),
        Some(Version { major: 2, minor: 0 })
    );
}

#[test]
fn inserting_a_newer_pdf_uses_the_newer_version() {
    // Annex I.2 recommends the maximum after insertion. This currently exposes the missing
    // upgrade from a 1.7 destination when a PDF 2.0 page is inserted.
    assert_eq!(
        inserted_version("1.7", "2.0"),
        Some(Version { major: 2, minor: 0 })
    );
}
