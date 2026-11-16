//! Annex O integration control: percent-encoded structelem byte ID reaches the public opener.
//! Exact-xref structure with coherent parent links and first-page controls.

#![expect(clippy::panic, clippy::arithmetic_side_effects)]

use std::fmt::Write as _;
use viewer_core::{Answer, Command, DocumentId, Query, Viewer};

fn exact_pdf(objects: &[String]) -> Vec<u8> {
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref_at = out.len();
    let size = objects.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<00112233445566778899AABBCCDDEEFF> <00112233445566778899AABBCCDDEEFF>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn fixture() -> Vec<u8> {
    let content = "/P << /MCID 0 >> BDC 0 0 10 10 re f EMC";
    let stream = format!(
        "<< /Length {} >>\nstream\n{content}\nendstream",
        content.len()
    );
    let mut objects: Vec<String> = [
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R /MarkInfo << /Marked true >> >>",
        "<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 12 0 R /StructParents 0 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 13 0 R /StructParents 1 >>",
        "<< /Type /StructTreeRoot /K [6 0 R] /IDTree 9 0 R /ParentTree 14 0 R >>",
        "<< /Type /StructElem /S /Document /P 5 0 R /ID (A&B) /K [7 0 R 8 0 R] >>",
        "<< /Type /StructElem /S /P /P 6 0 R /K [10 0 R] >>",
        "<< /Type /StructElem /S /P /P 6 0 R /K [11 0 R] >>",
        "<< /Names [(A&B) 6 0 R] >>",
        "<< /Type /MCR /Pg 4 0 R /MCID 0 >>",
        "<< /Type /MCR /Pg 3 0 R /MCID 0 >>",
        "",
        "",
        "<< /Nums [0 [8 0 R] 1 [7 0 R]] >>",
    ].into_iter().map(str::to_owned).collect();
    objects[11].clone_from(&stream);
    objects[12] = stream;
    exact_pdf(&objects)
}

fn opened_page(bytes: Vec<u8>, fragment: &str) -> usize {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let _ = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: Some(fragment.to_owned()),
        })
        .collect::<Vec<_>>();
    let Answer::Page { index, .. } = viewer.query(Query::CurrentPage) else {
        panic!("the viewer has an open page");
    };
    index
}

#[test]
fn uri_encoded_structelem_id_matches_raw_idtree_bytes_and_selects_first_content_page() {
    let bytes = fixture();
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("valid xref fixture");
    assert!(!parsed.was_recovered(), "the exact xref is used");
    for index in 0..2 {
        let page = pdf_model::Pages::new(&parsed)
            .get(index)
            .expect("two pages");
        assert!(pdf_model::interpret(&parsed, &page).is_complete());
    }
    assert_eq!(
        opened_page(bytes.clone(), ""),
        0,
        "no fragment starts on page one"
    );
    assert_eq!(
        opened_page(bytes.clone(), "page=2"),
        1,
        "positive page-number control reaches the second page"
    );
    assert_eq!(
        opened_page(bytes.clone(), "page=1"),
        0,
        "first-page control reaches the initial page"
    );
    assert_eq!(
        opened_page(bytes.clone(), "structelem=unknown"),
        0,
        "an absent ID does not move the initial page"
    );
    assert_eq!(
        opened_page(bytes, "structelem=A%26B"),
        1,
        "decoded bytes A&B match the StructElem ID and its first MCR is on page two"
    );
}
