//! Integration tests for §7.11.1 and Table 43 parsing.
//!
//! Complete fixtures exercise indirect string and dictionary forms.
#![expect(clippy::expect_used, reason = "complete generated fixtures must open")]

use pdf_model::file_spec::{FileSpec, FileSystem};
use pdf_syntax::{Document, Object, ObjectId};
use std::fmt::Write as _;

/// Build a complete, non-recovered PDF with a catalog, page tree, page, and two file specs.
fn valid_document(specification: &str) -> Document {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> >>",
        "(images/figure.bin)",
        specification,
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        writeln!(bytes, "{} 0 obj\n{body}\nendobj", index.saturating_add(1))
            .expect("String writes cannot fail");
    }
    let xref_offset = bytes.len();
    writeln!(
        bytes,
        "xref\n0 {}\n0000000000 65535 f ",
        objects.len().saturating_add(1)
    )
    .expect("String writes cannot fail");
    for offset in offsets {
        writeln!(bytes, "{offset:010} 00000 n ").expect("String writes cannot fail");
    }
    write!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
        objects.len().saturating_add(1)
    )
    .expect("String writes cannot fail");

    let document = Document::open(bytes.into_bytes()).expect("the fixture is a valid PDF");
    assert!(
        !document.was_recovered(),
        "the xref must be used as written"
    );
    assert_eq!(
        pdf_model::Pages::new(&document).len(),
        1,
        "the page-tree traversal finds one leaf"
    );
    document
}

#[test]
fn file_specification_forms_and_table_entries_are_read_from_a_valid_document() {
    let document = valid_document(
        "<< /Type /Filespec /F (legacy-name.bin) \
         /UF <FEFF007200E900730075006D00E9002D732B002E007000640066> \
         /Desc (diagram data) /V true \
         /ID [<000102030405060708090A0B0C0D0E0F> \
              <101112131415161718191A1B1C1D1E1F>] >>",
    );

    // Positive control first: §7.11.1's simple string form survives indirect-reference parsing.
    let simple = FileSpec::parse(&document, &Object::Reference(ObjectId::new(4, 0)))
        .expect("a string is a simple file specification");
    assert_eq!(
        simple.bytes.as_deref(),
        Some(b"images/figure.bin".as_slice())
    );
    assert_eq!(
        simple.components(),
        vec![b"images".to_vec(), b"figure.bin".to_vec()]
    );

    // The dictionary form carries the preferred Unicode spelling plus its optional metadata.
    let full = FileSpec::parse(&document, &Object::Reference(ObjectId::new(5, 0)))
        .expect("a dictionary is a simple file specification too");
    assert_eq!(full.system, FileSystem::Standard);
    assert_eq!(full.display_name().as_deref(), Some("résumé-猫.pdf"));
    assert_eq!(full.bytes.as_deref(), Some(b"legacy-name.bin".as_slice()));
    assert_eq!(full.description.as_deref(), Some("diagram data"));
    assert!(
        full.volatile,
        "Table 43's /V true is retained for the eventual fetcher"
    );
    assert_eq!(
        full.id,
        Some([
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
            vec![
                16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31
            ],
        ]),
        "Table 43 ID retains both distinct identifier strings"
    );
}

/// Optional metadata defaults stay distinct from explicitly stated true /V and /FS.
#[test]
fn omitted_file_specification_entries_have_their_defined_defaults() {
    let document = valid_document("<< /Type /Filespec /F (directory/) >>");
    let specification = FileSpec::parse(&document, &Object::Reference(ObjectId::new(5, 0)))
        .expect("valid dictionary form");
    assert_eq!(specification.display_name().as_deref(), Some("directory/"));
    assert_eq!(
        specification.components(),
        vec![b"directory".to_vec(), Vec::new()]
    );
    assert!(!specification.is_absolute());
    assert_eq!(specification.system, FileSystem::Standard);
    assert!(!specification.volatile);
    assert_eq!(specification.id, None);
    assert_eq!(specification.description, None);
    assert_eq!(specification.unicode_name, None);
}

/// /FS is independent of the name keys; an application-defined system retains its identity.
#[test]
fn explicit_filesystem_and_legacy_filename_are_retained() {
    let document =
        valid_document("<< /Type /Filespec /FS /ExampleFS /Unix (legacy/path.bin) /V false >>");
    let specification = FileSpec::parse(&document, &Object::Reference(ObjectId::new(5, 0)))
        .expect("valid legacy dictionary form");
    assert_eq!(specification.system, FileSystem::Other("ExampleFS".into()));
    assert_eq!(
        specification.bytes.as_deref(),
        Some(b"legacy/path.bin".as_slice())
    );
    assert!(specification.platform_specific);
    assert!(!specification.volatile);
}
