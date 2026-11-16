//! Q.3 -> Q.2 legacy resource fallback through nested forms.
//! This deliberately omits a form resource dictionary to test compatibility lookup;
//! it is not a producer-conformance witness for PDF 1.7 or PDF 2.0.

#![expect(clippy::expect_used, reason = "standalone exact-xref fixture")]

use std::fmt::Write as _;

use pdf_archive::survey::Survey;
use pdf_syntax::Document;

fn page(inherited_fill_alpha: &str) -> Document {
    let page_do = "/Outer Do";
    let inner = "/GS0 gs 0 0 10 10 re f";
    let outer = "/Inner Do";
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        format!(
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /Resources << \
             /XObject << /Outer 5 0 R >> /ExtGState << /GS0 << /ca {inherited_fill_alpha} >> >> >> >>"
        ),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R >>".to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{page_do}\nendstream",
            page_do.len().saturating_add(1)
        ),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] \
             /Resources << /XObject << /Inner 6 0 R >> >> /Length {} >>\n\
             stream\n{outer}\nendstream",
            outer.len().saturating_add(1)
        ),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Length {} >>\n\
             stream\n{inner}\nendstream",
            inner.len().saturating_add(1)
        ),
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj\n{body}\nendobj", index.saturating_add(1));
    }
    let xref = bytes.len();
    let size = bodies.len().saturating_add(1);
    let _ = write!(bytes, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    let document = Document::open(bytes.into_bytes()).expect("the exact-xref PDF opens");
    assert!(
        !document.was_recovered(),
        "the fixture xref is authoritative"
    );
    document
}

/// The page inherits its resource dictionary. The outer form has its own dictionary that names
/// only the inner form. The inner form has no /Resources, so §7.8.3's clarified fallback uses the
/// page's dictionary; the /GS0 it names sets the Q.2 alpha condition on the inner painted path.
#[test]
fn q3_nested_form_without_resources_uses_inherited_page_resources() {
    assert!(Survey::of(&page("0.5")).page_is_transparent(0));
    assert!(
        !Survey::of(&page("1")).page_is_transparent(0),
        "the resource lookup is exercised but the Q.2 alpha is at its opaque boundary"
    );
}
