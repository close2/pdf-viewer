//! What a redaction drops beside the content it removes, and how every rewriting writer carries a
//! direct information dictionary (ISO 32000-2 §14.5, §12.3.4, §12.5.6.23, §14.3.3 with Errata
//! Collection 3's Issue #106; ADRs 3717 and 3719).

#![expect(
    clippy::expect_used,
    reason = "tests use complete self-contained fixtures and fail loudly on an invalid writer result"
)]

use std::fmt::Write as _;

use pdf_syntax::{Dictionary, Document, Object};
use pdf_transform::range::Selection;
use pdf_transform::redact::RedactPlan;
use pdf_transform::split::{Pieces, SplitPlan};
use pdf_transform::update::{Edit, InfoEntry, UpdatePlan};
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Report, Source, apply};

/// Writes `objects` (numbered from one) into a §7.5.4 cross-referenced file with `trailer` added
/// to the trailer dictionary.
fn assemble(objects: &[String], trailer: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{object}\nendobj", index.saturating_add(1));
    }
    let at = out.len();
    let size = objects.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R {trailer} >>\nstartxref\n{at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// A stream object's text with its `/Length` stated.
fn stream(entries: &str, data: &str) -> String {
    format!(
        "<< {entries} /Length {} >>\nstream\n{data}\nendstream",
        data.len()
    )
}

/// The report and the one output of `plan` over `source`.
fn run(plan: &Plan, source: Vec<u8>) -> (Report, Document) {
    let sinks = MemorySinks::new();
    let report = apply(
        plan,
        &[Source::new(source)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the plan applies to a valid fixture");
    let mut outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1, "one source, one output");
    let document = Document::open(outputs.remove(0).1).expect("the output opens");
    (report, document)
}

/// Two pages under a catalog with private data; the first holds the text a `/Redact` annotation
/// covers, private data, its date and a thumbnail, and draws a form with private data of its own;
/// the second states private data and is not redacted.
fn two_pages() -> Vec<u8> {
    assemble(
        &[
            "<< /Type /Catalog /Pages 2 0 R /PieceInfo << /Editor << \
             /LastModified (D:20250101000000Z) /Private (SECRET) >> >> >>"
                .to_owned(),
            "<< /Type /Pages /Kids [3 0 R 8 0 R] /Count 2 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] \
             /Resources << /Font << /F1 5 0 R >> /XObject << /Fm 10 0 R >> >> \
             /Contents 4 0 R /Annots [6 0 R] /Thumb 7 0 R /LastModified (D:20250101000000Z) \
             /PieceInfo << /Editor << /LastModified (D:20250101000000Z) /Private (SECRET) >> >> >>"
                .to_owned(),
            stream("", "BT /F1 12 Tf 20 50 Td (SECRET) Tj ET /Fm Do"),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
            "<< /Type /Annot /Subtype /Redact /Rect [10 40 130 66] >>".to_owned(),
            stream(
                "/Width 1 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8",
                "\u{7f}",
            ),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
             /Contents 9 0 R /LastModified (D:20250101000000Z) /PieceInfo << /Editor << \
             /LastModified (D:20250101000000Z) >> >> >>"
                .to_owned(),
            stream("", "0 0 5 5 re f"),
            stream(
                "/Type /XObject /Subtype /Form /BBox [0 0 200 200] \
                 /Resources << /Font << /F1 5 0 R >> >> /LastModified (D:20250101000000Z) \
                 /PieceInfo << /Editor << /LastModified (D:20250101000000Z) >> >>",
                "BT /F1 12 Tf 20 45 Td (SECRET) Tj ET",
            ),
        ],
        "",
    )
}

fn redacted() -> (Report, Document) {
    run(
        &Plan::Redact(RedactPlan {
            source: 0,
            names: "redacted.pdf".parse().expect("output name parses"),
            image_samples: None,
        }),
        two_pages(),
    )
}

fn page(document: &Document, index: usize) -> Dictionary {
    pdf_model::Pages::new(document)
        .get(index)
        .expect("the output has the page")
        .dict
        .clone()
}

/// §14.5's private data and the date that calls it current, and §12.3.4's thumbnail, describe the
/// redacted page's content before the removal: none is written, on the page, on the form the
/// removal edited, or in the catalog, and every one is a departure in the report.
#[test]
fn a_redaction_drops_and_reports_what_described_the_removed_content() {
    let (report, document) = redacted();
    let first = page(&document, 0);
    for key in ["PieceInfo", "LastModified", "Thumb"] {
        assert!(first.get(key).is_none(), "the redacted page states /{key}");
    }
    let catalog = document.catalog().expect("the output has a catalog");
    assert!(
        catalog.get("PieceInfo").is_none(),
        "the catalog kept /PieceInfo"
    );
    let resources = document.get_key(&first, "Resources");
    let form = resources
        .as_dict()
        .map(|resources| document.get_key(resources, "XObject"))
        .and_then(|xobjects| {
            xobjects
                .as_dict()
                .map(|xobjects| document.get_key(xobjects, "Fm"))
        })
        .expect("the page names its form");
    let form = form.as_stream().expect("the form is a stream");
    assert!(!form.data.windows(6).any(|window| window == b"SECRET"));
    for key in ["PieceInfo", "LastModified"] {
        assert!(
            form.dict.get(key).is_none(),
            "the edited form states /{key}"
        );
    }

    let details: Vec<(Option<usize>, &str)> = report
        .departures
        .iter()
        .map(|departure| (departure.page, departure.detail.as_str()))
        .collect();
    let said = |page: Option<usize>, phrase: &str| {
        details.iter().any(|(at, detail)| {
            *at == page && detail.starts_with("§14.5") && detail.contains(phrase)
        })
    };
    assert!(
        said(
            Some(1),
            "the page's /PieceInfo and /LastModified and /Thumb not written"
        ),
        "{details:?}"
    );
    assert!(
        said(Some(1), "form XObject 10's /PieceInfo and /LastModified"),
        "{details:?}"
    );
    assert!(said(None, "the catalog's /PieceInfo"), "{details:?}");
    assert!(!details.iter().any(|(at, _)| *at == Some(2)), "{details:?}");
}

/// The page the redaction did not touch keeps its private data and its date: its content is the
/// content they describe.
#[test]
fn a_page_no_redaction_touched_keeps_its_private_data() {
    let (_, document) = redacted();
    let second = page(&document, 1);
    assert!(second.get("PieceInfo").is_some());
    assert_eq!(
        document.get_key(&second, "LastModified").as_string(),
        Some(&b"D:20250101000000Z"[..])
    );
}

/// A one-page file whose trailer states `/Info` directly, as Issue #106 allows.
fn direct_information() -> Vec<u8> {
    assemble(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> \
             /Contents 4 0 R >>"
                .to_owned(),
            stream("", "0 0 5 5 re f"),
            "(Annual report)".to_owned(),
        ],
        "/Info << /Title 5 0 R /Author (Ada) >>",
    )
}

/// The output's information dictionary, entry `key` as a string.
fn information(document: &Document, key: &str) -> Option<Vec<u8>> {
    let info = document.get_key(document.trailer(), "Info");
    let info = info.as_dict()?;
    document.get_key(info, key).as_string().map(<[u8]>::to_vec)
}

/// `split` carries a direct `/Info` whole, the indirect string it names included.
#[test]
fn a_split_piece_carries_a_direct_information_dictionary() {
    let (_, piece) = run(
        &Plan::Split(SplitPlan {
            source: 0,
            pages: "1".parse::<Selection>().expect("a selection"),
            pieces: Pieces::EachPage,
            names: "piece-%d.pdf".parse().expect("output pattern parses"),
        }),
        direct_information(),
    );
    assert_eq!(
        information(&piece, "Title").as_deref(),
        Some(&b"Annual report"[..])
    );
    assert_eq!(information(&piece, "Author").as_deref(), Some(&b"Ada"[..]));
}

/// `SetInformation` over a direct `/Info` writes the dictionary as an object of its own, keeps
/// what it did not change, and warns of nothing: the file was valid (Issue #106).
#[test]
fn an_update_restates_a_direct_information_dictionary_without_a_warning() {
    let (report, updated) = run(
        &Plan::Update(UpdatePlan {
            source: 0,
            edit: Edit::SetInformation {
                entries: vec![InfoEntry {
                    key: "Subject".to_owned(),
                    value: Some("Accounts".to_owned()),
                }],
            },
            names: "updated.pdf".parse().expect("output name parses"),
        }),
        direct_information(),
    );
    assert!(
        updated
            .trailer()
            .get("Info")
            .and_then(Object::as_reference)
            .is_some()
    );
    assert_eq!(
        information(&updated, "Subject").as_deref(),
        Some(&b"Accounts"[..])
    );
    assert_eq!(
        information(&updated, "Author").as_deref(),
        Some(&b"Ada"[..])
    );
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}
