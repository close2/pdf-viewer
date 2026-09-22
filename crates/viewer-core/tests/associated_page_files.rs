//! §14.13.4's associated files, listed by the panel that lists a document's.
//!
//! The clause states the entry and states it of a *page*:
//!
//! > One or more files may be associated with any PDF page by including a file specification
//! > dictionary (7.11.3, "File specification dictionaries") for each file as one of the members
//! > of the array value of the AF key in the appropriate page dictionary (7.7.3.3, "Page
//! > objects"). The relationship that the associated files have to the page is supplied by the
//! > AFRelationship key in each file specification dictionary.
//!
//! `pdf_model::attachment::associated` has read that array for any carrier since §14.13 was
//! implemented, and no caller in this tree handed it a page dictionary — the shape §14.13.3's
//! catalog `/AF` and §12.5.6.15's `/FS` were both in before a consumer reached them (ADR 0295):
//! a reading that exists, a row that calls it implemented, and a payload no panel could list.
//!
//! **Which page, and why not all of them.** `CLAUDE.md` principle 2 forbids a full page-tree walk
//! on the launch path, and every host asks `Query::Attachments` when a document opens. So the
//! answer carries the page the reader is *on*, which this crate has already built. ADR 1186.
//!
//! Not one corpus document states a page `/AF` — the clause's own NOTE says why a producer would
//! — so the fixture is written here (trap 8).

#![expect(
    clippy::arithmetic_side_effects,
    reason = "test code: the object numbers below are the small integers this file wrote"
)]
#![expect(
    clippy::panic,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, PageTarget, Query, Viewer};

/// The document under test.
const DOCUMENT: DocumentId = DocumentId(1);

/// Two pages, an `/AF` on the catalog and a different one on the second page.
///
/// Both file specifications are §14.13.2's recommended form — "the embedded form is recommended"
/// — so each carries an `/EF` whose stream is the payload, and each states the `/AFRelationship`
/// the clause asks for.
fn two_pages_one_with_an_associated_file() -> Vec<u8> {
    let objects: [&str; 9] = [
        "<< /Type /Catalog /Pages 2 0 R /AF [8 0 R] >>",
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 800] /Contents 5 0 R >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 800] /Contents 5 0 R /AF [6 0 R] >>",
        "<< /Length 0 >>\nstream\n\nendstream",
        "<< /Type /Filespec /F (chart.csv) /UF (chart.csv) /AFRelationship /Data \
          /EF << /F 7 0 R >> >>",
        "<< /Type /EmbeddedFile /Subtype /text#2Fcsv /Length 12 >>\nstream\nyear,value\n\nendstream",
        "<< /Type /Filespec /F (source.txt) /UF (source.txt) /AFRelationship /Source \
          /EF << /F 9 0 R >> >>",
        "<< /Type /EmbeddedFile /Subtype /text#2Fplain /Length 6 >>\nstream\nwhole\nendstream",
    ];
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref_at = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len() + 1
    );
    out.into_bytes()
}

/// The names `Query::Attachments` answers with, in the order it answers them.
fn listed(viewer: &Viewer) -> Vec<String> {
    match viewer.query(Query::Attachments) {
        Answer::Attachments(files) => files.into_iter().map(|file| file.name).collect(),
        other => panic!("the query answers attachments: {other:?}"),
    }
}

/// §14.13.4's array reaches the list, and §14.13.3's stays in it.
///
/// Three assertions and each is a different sentence. On page one the answer is the catalog's
/// file alone, which is §14.13.3 and was already true. On page two it is that file **and** the
/// page's, which is §14.13.4 and is what had no consumer. And the relationship each specification
/// asserts arrives with it, because §14.13.1 makes identifying it half of what an associated file
/// is for.
#[test]
fn a_pages_associated_file_is_listed_while_the_reader_is_on_that_page() {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: two_pages_one_with_an_associated_file().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);

    assert_eq!(
        listed(&viewer),
        ["source.txt"],
        "§14.13.3's catalog array, and no page's"
    );

    let _ = viewer.handle(Command::GoTo(PageTarget::Index(1))).count();
    assert_eq!(
        listed(&viewer),
        ["source.txt", "chart.csv"],
        "§14.13.4's array on the page the reader is on, after the document's own"
    );

    let Answer::Attachments(files) = viewer.query(Query::Attachments) else {
        panic!("the query answers attachments")
    };
    let page_file = files
        .iter()
        .find(|file| file.name == "chart.csv")
        .expect("the page's file");
    assert_eq!(
        page_file.relationship,
        pdf_model::attachment::Relationship::Data,
        "Table 43's /AFRelationship, which §14.13.4 says supplies the relationship to the page"
    );
    assert_eq!(page_file.media_type.as_deref(), Some("text/csv"));

    let _ = viewer.handle(Command::GoTo(PageTarget::Index(0))).count();
    assert_eq!(
        listed(&viewer),
        ["source.txt"],
        "the entry belongs to a page, so the list follows the reader"
    );
}
