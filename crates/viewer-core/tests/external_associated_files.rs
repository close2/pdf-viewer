//! §14.13.2's associated files outside the document, answered for the attachments panel.
//!
//! The clause gives an associated file two forms in one sentence:
//!
//! > The file specification for an associated file represents either a file external to the PDF
//! > file or an embedded file stream (see 7.11.4, "Embedded file streams") within the PDF file.
//!
//! `Query::Attachments` lists the second form; `Query::ExternalFiles` answers with the first, from
//! the same two homes — §14.13.3's catalog `/AF` and §14.13.4's `/AF` on the page in view — so the
//! panel can list it as a row with no save action (`doc/questions/A45`, ADR 2799).
//!
//! No corpus document states an external associated file (ADR 0918's census), so the fixture is
//! written here (trap 8).

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

use pdf_model::attachment::{ExternalAssociatedFile, Relationship};
use viewer_core::{Answer, Command, DocumentId, PageTarget, Query, Viewer};

/// The document under test.
const DOCUMENT: DocumentId = DocumentId(1);

/// Two pages. The catalog's `/AF` holds one embedded and one external specification; the second
/// page's holds a second external one, the catalog's external one again, and one that names no
/// file.
///
/// An external specification is §7.11.3's dictionary with no `/EF`: Table 43 makes `/EF`
/// "\[r]equired if RF is present" and otherwise optional, and its absence is what makes the
/// specification name a file rather than carry one.
fn two_pages_with_external_associated_files() -> Vec<u8> {
    let objects: [&str; 10] = [
        "<< /Type /Catalog /Pages 2 0 R /AF [6 0 R 8 0 R] >>",
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 800] /Contents 5 0 R >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 800] /Contents 5 0 R \
          /AF [9 0 R 10 0 R 8 0 R] >>",
        "<< /Length 0 >>\nstream\n\nendstream",
        "<< /Type /Filespec /F (source.txt) /UF (source.txt) /AFRelationship /Source \
          /EF << /F 7 0 R >> >>",
        "<< /Type /EmbeddedFile /Subtype /text#2Fplain /Length 6 >>\nstream\nwhole\nendstream",
        "<< /Type /Filespec /F (survey.csv) /UF (survey.csv) /AFRelationship /Data >>",
        "<< /Type /Filespec /F (scan.tif) /UF (scan.tif) /AFRelationship /Alternative >>",
        "<< /Type /Filespec /AFRelationship /Supplement >>",
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

fn external(viewer: &Viewer) -> Vec<ExternalAssociatedFile> {
    match viewer.query(Query::ExternalFiles) {
        Answer::ExternalFiles(files) => files,
        other => panic!("the query answers external files: {other:?}"),
    }
}

fn file(name: &str, relationship: Relationship) -> ExternalAssociatedFile {
    ExternalAssociatedFile {
        name: name.to_owned(),
        relationship,
    }
}

/// Each external specification is answered once, by name and relationship, from the catalog and
/// the page in view, and none of them reaches the embedded list.
///
/// Four sentences, one assertion each. §14.13.2's first form is answered with what the producer
/// stated of it, Table 43's `/AFRelationship` included. §14.13.4's page array joins on the page it
/// is on and not before. A specification that names no file is still answered, since its
/// relationship is a fact about the document. And the form decides the list: the embedded file is
/// `Query::Attachments`'s and the external ones are not.
#[test]
fn a_file_associated_but_not_carried_is_answered_from_the_catalog_and_the_page_in_view() {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: two_pages_with_external_associated_files().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);

    assert_eq!(
        external(&viewer),
        [file("survey.csv", Relationship::Data)],
        "§14.13.3's catalog array's external specification, and no page's"
    );

    let _ = viewer.handle(Command::GoTo(PageTarget::Index(1))).count();
    assert_eq!(
        external(&viewer),
        [
            file("survey.csv", Relationship::Data),
            file("scan.tif", Relationship::Alternative),
            file("", Relationship::Supplement),
        ],
        "§14.13.4's array on the page in view after the catalog's, the catalog's file named \
         again on the page listed once, and a specification naming no file kept"
    );

    let Answer::Attachments(embedded) = viewer.query(Query::Attachments) else {
        panic!("the query answers attachments")
    };
    assert_eq!(
        embedded
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        ["source.txt"],
        "§14.13.2's embedded form is the attachment list's, and the external form is not in it"
    );
}

/// A document that associates nothing answers with nothing, which is what lets a panel say it
/// holds no files.
#[test]
fn a_document_with_no_associated_files_answers_with_none() {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let bytes = String::from_utf8(two_pages_with_external_associated_files())
        .expect("the fixture is ASCII")
        // The same length, so that the cross-reference table's offsets still hold.
        .replace("/AF [6 0 R 8 0 R]", &" ".repeat("/AF [6 0 R 8 0 R]".len()))
        .into_bytes();
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    assert!(
        external(&viewer).is_empty(),
        "a catalog with no /AF, on a page with none"
    );
}
