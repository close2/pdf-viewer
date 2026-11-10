//! Focused §8.9.7 reader tests for Table 91 key precedence and Table 92 Indexed expansion.

#![expect(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "the self-contained PDF fixtures have fixed, checked object layouts"
)]

use std::fmt::Write as _;

use pdf_model::inline_image;
use pdf_syntax::{Dictionary, Document, Object};

fn one_page_pdf(content: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    body.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    body.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> /Contents 4 0 R >>\nendobj\n",
    );
    body.extend_from_slice(
        format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes(),
    );
    body.extend_from_slice(content);
    body.extend_from_slice(b"\nendstream\nendobj\n");

    // PDF 1.7 avoids PDF 2.0's trailer /ID requirement; this fixture still has a complete xref.
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    let mut remaining = body.as_slice();
    while let Some(end) = remaining
        .windows(7)
        .position(|window| window == b"endobj\n")
    {
        let stop = end + 7;
        offsets.push(bytes.len());
        bytes.extend_from_slice(&remaining[..stop]);
        remaining = &remaining[stop..];
    }
    assert!(
        remaining.is_empty(),
        "fixture body ends at an object boundary"
    );

    let xref_at = bytes.len();
    let size = offsets.len() + 1;
    let mut trailer = String::new();
    let _ = writeln!(trailer, "xref\n0 {size}");
    trailer.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    bytes.extend_from_slice(trailer.as_bytes());
    bytes
}

fn scan(document: &Document, content: &[u8]) -> inline_image::Scan {
    assert_eq!(content.get(..2), Some(&b"BI"[..]));
    inline_image::scan(document, content, 2, &Dictionary::new(), true)
}

#[test]
fn abbreviated_table91_keys_win_in_either_order_and_unlisted_keys_are_ignored() {
    // Positive control: the full spelling alone states a one-sample grid and consumes one byte.
    let full_content =
        b"BI /Width 1 /Height 1 /BitsPerComponent 8 /ColorSpace /DeviceGray ID \xff EI q Q";
    let full_doc =
        Document::open(one_page_pdf(full_content)).expect("complete PDF 1.7 fixture opens");
    assert!(
        !full_doc.was_recovered(),
        "positive control has a complete xref"
    );
    let full = scan(&full_doc, full_content)
        .image
        .expect("full-key image scans");
    assert_eq!(full.dict.get("Width").and_then(Object::as_integer), Some(1));
    assert_eq!(full.data.as_ref(), b"\xff");

    // EC3 Issue #3: abbreviation wins even when the contradictory full key appears later.
    // Uppercase I is Table 91's Interpolate key (Issue #306), and /SMask is not listed.
    for content in [
        &b"BI /W 1 /Width 2 /H 1 /BPC 8 /CS /G /I true /Interpolate false /SMask true ID \xff EI q Q"
            [..],
        &b"BI /Width 2 /W 1 /H 1 /BPC 8 /CS /G /Interpolate false /I true /SMask true ID \xff EI q Q"
            [..],
    ] {
        let document = Document::open(one_page_pdf(content)).expect("complete fixture opens");
        assert!(!document.was_recovered(), "fixture has a complete xref");
        let scanned = scan(&document, content);
        let image = scanned.image.expect("abbreviated-key image scans");
        assert_eq!(
            image.dict.get("Width").and_then(Object::as_integer),
            Some(1),
            "the abbreviation /W overrides the conflicting /Width in either order"
        );
        assert_eq!(
            image.dict.get("Height").and_then(Object::as_integer),
            Some(1)
        );
        assert_eq!(
            image
                .dict
                .get("Interpolate")
                .map(|value| matches!(value, Object::Boolean(true))),
            Some(true)
        );
        assert!(
            image.dict.get("SMask").is_none(),
            "unlisted entries are ignored"
        );
        assert_eq!(image.data.as_ref(), b"\xff");
        assert_eq!(
            content.get(scanned.resume..),
            Some(&b" q Q"[..]),
            "the scan resumes immediately after the EI terminator"
        );
    }

    // Table 92's I is the Indexed family when used as a ColorSpace value, while the same
    // spelling in a Table 91 key position is Interpolate.
    let indexed_content = b"BI /W 1 /H 1 /BPC 8 /CS [/I /G 0 <ff>] ID \x00 EI q Q";
    let indexed_doc =
        Document::open(one_page_pdf(indexed_content)).expect("complete fixture opens");
    assert!(
        !indexed_doc.was_recovered(),
        "Indexed control has a complete xref"
    );
    let indexed = scan(&indexed_doc, indexed_content)
        .image
        .expect("Indexed abbreviation scans");
    let space = indexed
        .dict
        .get("ColorSpace")
        .and_then(Object::as_array)
        .expect("ColorSpace array");
    assert_eq!(
        space
            .first()
            .and_then(Object::as_name)
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"Indexed"[..])
    );
}
