//! Table 15 Issue #106 regression: `/Info` may be direct or indirect.
//! The direct form is the target; the indirect form is an equivalent valid control.
//! Exact offsets and PDF 2.0 trailer ID keep the source premise valid.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "small deterministic exact-xref fixture"
)]

use std::fmt::Write as _;

use pdf_model::metadata::Information;
use pdf_syntax::Document;

fn pdf(info_is_direct: bool) -> Vec<u8> {
    let info_object = if info_is_direct {
        ""
    } else {
        "4 0 obj\n<< /Title (Indirect title) /Author (Indirect author) >>\nendobj\n"
    };
    let info_entry = if info_is_direct {
        "/Info << /Title (Direct title) /Author (Direct author) >>"
    } else {
        "/Info 4 0 R"
    };
    let mut bytes = b"%PDF-2.0\n".to_vec();
    let mut offsets = Vec::new();
    for body in [
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
        "2 0 obj\n<< /Type /Pages /Count 1 /Kids [3 0 R] >>\nendobj\n",
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> >>\nendobj\n",
    ] {
        offsets.push(bytes.len());
        bytes.extend_from_slice(body.as_bytes());
    }
    if !info_object.is_empty() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(info_object.as_bytes());
    }
    let xref = bytes.len();
    let size = offsets.len() + 1;
    let _ = writeln!(StringWriter(&mut bytes), "xref\n0 {size}");
    let _ = writeln!(StringWriter(&mut bytes), "0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(StringWriter(&mut bytes), "{offset:010} 00000 n ");
    }
    let _ = write!(
        StringWriter(&mut bytes),
        "trailer\n<< /Size {size} /Root 1 0 R {info_entry} \
         /ID [<00112233445566778899AABBCCDDEEFF> <00112233445566778899AABBCCDDEEFF>] >>\n\
         startxref\n{xref}\n%%EOF\n"
    );
    bytes
}

struct StringWriter<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for StringWriter<'_> {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        self.0.extend_from_slice(value.as_bytes());
        Ok(())
    }
}

#[test]
fn table15_info_accepts_direct_and_indirect_dictionaries() {
    let direct = Document::open(pdf(true)).expect("direct Info PDF opens");
    let indirect = Document::open(pdf(false)).expect("indirect Info PDF opens");
    for document in [&direct, &indirect] {
        assert!(!document.was_recovered(), "xref is exact");
        assert!(!document.cross_reference_chain_was_cut());
        assert!(document.catalog().is_ok(), "catalog resolves normally");
    }

    let direct_info = Information::read(&direct);
    assert_eq!(direct_info.title.as_deref(), Some("Direct title"));
    assert_eq!(direct_info.author.as_deref(), Some("Direct author"));
    let indirect_info = Information::read(&indirect);
    assert_eq!(indirect_info.title.as_deref(), Some("Indirect title"));
    assert_eq!(indirect_info.author.as_deref(), Some("Indirect author"));
}
