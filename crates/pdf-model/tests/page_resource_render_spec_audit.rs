//! Integration test for §§7.7.3.4 and 7.8.3.
//! It exercises the page's actual public rendering consumer, not only `Page.resources`.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use pdf_render::{Command, Paint};
use pdf_syntax::Document;
use std::fmt::Write as _;

fn document(body: &str) -> Document {
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let xref_at = bytes.len();
    let size = offsets.len() + 1;
    let _ = writeln!(bytes, "xref\n0 {size}");
    bytes.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    let document = Document::open(bytes.into_bytes()).expect("valid exact-xref fixture");
    assert!(!document.was_recovered(), "all xref offsets are exact");
    document
}

fn fill(bytes: &str) -> [u32; 4] {
    let document = document(bytes);
    let pages = pdf_model::Pages::new(&document);
    assert_eq!(pages.len(), 1);
    let page = pages.get(0).expect("page tree yields page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "valid fixture is fully interpreted: {:?}",
        interpretation.unsupported
    );
    let fills: Vec<_> = interpretation
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill {
                paint: Paint::Solid(colour),
                ..
            } => Some([
                colour.r.to_bits(),
                colour.g.to_bits(),
                colour.b.to_bits(),
                colour.a.to_bits(),
            ]),
            _ => None,
        })
        .collect();
    assert_eq!(fills.len(), 1, "the named form produces one solid fill");
    fills[0]
}

fn fixture(intermediate_resource: bool) -> String {
    let inner_resources = if intermediate_resource {
        "/Resources << /XObject << /Mark 7 0 R >> >>"
    } else {
        ""
    };
    let stream = "/Mark Do";
    format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 \
         /Resources << /XObject << /Mark 5 0 R >> >> >>\nendobj\n\
         3 0 obj\n<< /Type /Pages /Parent 2 0 R /Kids [4 0 R] /Count 1 {inner_resources} >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 3 0 R /MediaBox [0 0 100 100] /Contents 6 0 R >>\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Length {} >>\nstream\n1 0 0 rg 0 0 100 100 re f\nendstream\nendobj\n\
         6 0 obj\n<< /Length {} >>\nstream\n{stream}\nendstream\nendobj\n\
         7 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Length {} >>\nstream\n0 0 1 rg 0 0 100 100 re f\nendstream\nendobj\n",
        "1 0 0 rg 0 0 100 100 re f\n".len(),
        stream.len() + 1,
        "0 0 1 rg 0 0 100 100 re f\n".len(),
    )
}

/// Table 31/§7.7.3.4 choose the nearest inherited Resources dictionary.
/// A root-only rendering control paints red; inserting
/// an intermediate /Resources with the same /Mark name changes the page to blue.
#[test]
fn inherited_resources_choose_the_nearest_ancestor_in_page_rendering() {
    let root_only = fill(&fixture(false));
    assert_eq!(
        root_only,
        [1.0_f32, 0.0, 0.0, 1.0].map(f32::to_bits),
        "root resource resolves when it is the nearest one"
    );

    let shadowed = fill(&fixture(true));
    assert_eq!(
        shadowed,
        [0.0_f32, 0.0, 1.0, 1.0].map(f32::to_bits),
        "the intermediate resource dictionary shadows the root dictionary"
    );
}
