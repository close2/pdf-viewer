//! ISO 32000-2 Annex Q.2's population is "each graphical element to be rendered on the page", and
//! a soft mask's transparency group is not one: it is evaluated for the elements painted while
//! the mask is in force, each of which Q.2's first condition — an `SMask` entry whose value is a
//! dictionary — already finds. So a page that sets a mask and paints nothing under it
//! contains no transparency, and the same page painting one rectangle under it does.

#![expect(clippy::expect_used, reason = "standalone exact-xref fixture")]

use std::fmt::Write as _;

use pdf_archive::survey::Survey;
use pdf_syntax::Document;

/// One page whose resources name a graphics state setting a luminosity soft mask whose group is a
/// transparency group, with `content` as the page's content stream.
fn masked_page(content: &str) -> Document {
    let group = "0 0 10 10 re f";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R \
         /Resources << /ExtGState << /M 5 0 R >> >> >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len().saturating_add(1)
        ),
        "<< /Type /ExtGState /SMask << /Type /Mask /S /Luminosity /G 6 0 R >> >>".to_owned(),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources << >> \
             /Group << /S /Transparency /CS /DeviceGray >> /Length {} >>\nstream\n{group}\nendstream",
            group.len().saturating_add(1)
        ),
    ];
    let mut bytes = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj\n{body}\nendobj", index.saturating_add(1));
    }
    let xref = bytes.len();
    let size = objects.len().saturating_add(1);
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

#[test]
fn a_soft_mask_set_with_nothing_painted_under_it_is_no_transparency() {
    let survey = Survey::of(&masked_page("/M gs"));
    assert!(!survey.stopped_short());
    assert!(!survey.page_is_transparent(0));
}

#[test]
fn an_element_painted_under_a_soft_mask_is_transparency() {
    let survey = Survey::of(&masked_page("/M gs 0 0 5 5 re f"));
    assert!(!survey.stopped_short());
    assert!(survey.page_is_transparent(0));
}
