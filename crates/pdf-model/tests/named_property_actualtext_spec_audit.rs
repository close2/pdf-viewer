//! Table 352 and §14.6.2 named property lists reach §14.9.4 replacement text.
//! An indirect property value is stored outside the content stream, through Resources/Properties.

#![expect(clippy::arithmetic_side_effects, reason = "small exact-xref fixture")]

use pdf_syntax::Document;
use std::fmt::Write as _;

fn fixture(named: bool) -> Vec<u8> {
    let properties = if named {
        "/P"
    } else {
        "<< /ActualText (replacement) >>"
    };
    let content = format!("BT /F1 12 Tf 10 50 Td /Span {properties} BDC (drawn) Tj EMC ET");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << /Font << /F1 5 0 R >> /Properties << /P 6 0 R >> >> /Contents 4 0 R >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        "<< /ActualText 7 0 R >>".to_owned(),
        "(replacement)".to_owned(),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (i, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{object}\nendobj\n", i + 1);
    }
    let xref = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    );
    out.into_bytes()
}

#[test]
fn named_properties_resolve_indirect_actualtext_for_extraction_and_speech() {
    for named in [false, true] {
        let document = Document::open(fixture(named)).expect("valid exact-xref PDF1.7");
        assert!(!document.was_recovered());
        assert!(!document.cross_reference_chain_was_cut());
        let page = pdf_model::Pages::new(&document).get(0).expect("one page");
        let drawn = pdf_model::interpret(&document, &page);
        assert!(drawn.glyphs > 0, "fixture must draw real glyphs");
        assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
        assert_eq!(drawn.text.trim_end(), "replacement", "named={named}");
        let speech = drawn.speech();
        let runs: Vec<_> = speech
            .iter()
            .map(|r| (r.text.as_str(), r.language.as_deref()))
            .collect();
        assert_eq!(runs, vec![("replacement", None)], "named={named}");
    }
}
