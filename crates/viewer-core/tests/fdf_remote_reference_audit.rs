//! Table 253 /F accepts string and dictionary file-specification forms on the public
//! `ImportData` -> `NamedPage` host-supply path.
#![expect(
    clippy::expect_used,
    reason = "complete exact-xref fixtures and expected host requests must fail loudly"
)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Event, Purpose, Query, Viewer};

fn file(header: &str, root: &str, objects: &[String]) -> Vec<u8> {
    let mut out = String::from(header);
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let number = index.saturating_add(1);
        let _ = writeln!(out, "{number} 0 obj\n{object}\nendobj");
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root {root} /ID [<30313233343536373839616263646566> <66656463626139383736353433323130>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    let bytes = out.into_bytes();
    let document = pdf_syntax::Document::open(bytes.clone()).expect("complete fixture");
    assert!(!document.was_recovered(), "exact fixture xref");
    bytes
}

fn target() -> Vec<u8> {
    file(
        "%PDF-2.0\n",
        "1 0 R",
        &[
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [4 0 R] >>".to_owned(),
            "<< /Type /Annot /Subtype /Link /Rect [5 5 30 30] /A << /Type /Action /S /ImportData /F (pages.fdf) >> >>".to_owned(),
        ],
    )
}

fn fdf(file_specification: &str) -> Vec<u8> {
    file(
        "%FDF-1.2\n",
        "1 0 R",
        &[format!(
            "<< /Version /1.7 /FDF << /Pages [<< /Templates [<< /TRef << /Name (stamp) /F {file_specification} >> >>] >>] >> >>"
        )],
    )
}

fn library() -> Vec<u8> {
    file(
        "%PDF-1.7\n",
        "1 0 R",
        &[
            "<< /Type /Catalog /Pages 2 0 R /Names << /Templates << /Names [(stamp) 4 0 R] >> >> >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_owned(),
            "<< /Type /Template /MediaBox [0 0 100 100] /Resources << >> >>".to_owned(),
        ],
    )
}

fn import_with_specification(file_specification: &str, expected_file: &str) {
    let mut viewer = Viewer::new(200, 200, 1.0);
    viewer
        .handle(Command::Open {
            id: DocumentId(92),
            bytes: target().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let action = viewer
        .handle(Command::Activate(pdf_syntax::ObjectId::new(4, 0)))
        .collect::<Vec<_>>();
    assert!(
        action.iter().any(|event| matches!(event, Event::NeedsFile {
            purpose: Purpose::ImportData,
            name,
            ..
        } if name == "pages.fdf")),
        "reachable ImportData action asks for the FDF: {action:?}"
    );

    let imported = viewer
        .handle(Command::Supply {
            purpose: Purpose::ImportData,
            bytes: Some(fdf(file_specification)),
        })
        .collect::<Vec<_>>();
    assert!(
        imported
            .iter()
            .any(|event| matches!(event, Event::NeedsFile {
            purpose: Purpose::NamedPage,
            name,
            ..
        } if name == expected_file)),
        "parsed Table 253 file specification drives the next host request: {imported:?}"
    );
    assert!(
        matches!(viewer.query(Query::PageCount), Answer::Count(1)),
        "page is not appended before the named PDF is supplied"
    );

    let supplied = viewer
        .handle(Command::Supply {
            purpose: Purpose::NamedPage,
            bytes: Some(library()),
        })
        .collect::<Vec<_>>();
    assert!(
        supplied
            .iter()
            .any(|event| matches!(event, Event::Reported { notes, .. }
            if notes.iter().any(|note| note.contains("gave 0 appearance(s) and 1 page(s)")))),
        "the named Template is supplied and carried into the open viewer: {supplied:?}"
    );
    assert!(
        matches!(viewer.query(Query::PageCount), Answer::Count(2)),
        "the supplied template page is now in the viewer's page count"
    );
}

/// Positive control: the existing, simple file-specification spelling still follows the normal
/// host request and supply path.
#[test]
fn table253_string_file_specification_requests_and_adds_remote_template() {
    import_with_specification("(library.pdf)", "library.pdf");
}

/// The dictionary file-specification form is authorized by §7.11.1. With both names present,
/// Table 43 says a reader shall use /UF instead of /F. The host request and successful supply
/// make that precedence observable beyond parsing the dictionary.
#[test]
fn table253_dictionary_file_spec_uses_uf_on_public_host_path() {
    import_with_specification(
        "<< /Type /Filespec /F (wrong-fallback.pdf) /UF (library.pdf) >>",
        "library.pdf",
    );
}
