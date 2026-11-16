//! A valid page-level intent cannot silently become the catalog's different press.
//!
//! Section 14.11.5 makes the page's intent win when intents are respected. Table 401
//! permits a registered production condition to omit its profile. The page names the
//! standard's CGATS TR 001 example; no network/profile lookup is requested by this test.
//! Compare the page alone with the same page under a different catalog intent. A reader
//! may decline the unavailable transformation, but the catalog cannot redefine that page.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "small exact-xref test fixture with a shipped independent ICC profile"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

fn fixture(catalog_intent: bool, page_intent: bool) -> Document {
    let profile = include_bytes!("../../../data/icc/GRACoL2006_Coated1v2.icc");
    let mut hex = String::new();
    for byte in profile {
        let _ = write!(hex, "{byte:02x}");
    }
    hex.push('>');
    let paint = "0.25 g 0 0 10 10 re f";
    let catalog_entry = if catalog_intent {
        "/OutputIntents [5 0 R]"
    } else {
        ""
    };
    let page_entry = if page_intent {
        "/OutputIntents [7 0 R]"
    } else {
        ""
    };
    let objects = [
        format!("<< /Type /Catalog /Pages 2 0 R {catalog_entry} >>"),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".into(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> /Contents 4 0 R {page_entry} >>"),
        format!("<< /Length {} >>\nstream\n{paint}\nendstream", paint.len()),
        "<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (Catalog GRACoL) /OutputConditionIdentifier (Custom) /Info (GRACoL2006 Coated1 v2) /DestOutputProfile 6 0 R >>".into(),
        format!("<< /N 4 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}\nendstream", hex.len()),
        "<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (Page SWOP) /OutputConditionIdentifier (CGATS TR 001) /RegistryName (http://www.color.org) >>".into(),
    ];
    let mut bytes = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj\n{object}\nendobj", index + 1);
    }
    let xref = bytes.len();
    let _ = writeln!(bytes, "xref\n0 {}\n0000000000 65535 f ", offsets.len() + 1);
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size 8 /Root 1 0 R /ID [<00112233445566778899aabbccddeeff> <00112233445566778899aabbccddeeff>] >>\nstartxref\n{xref}\n%%EOF\n"
    );
    let document = Document::open(bytes.into_bytes()).expect("exact-xref PDF 2.0");
    assert!(!document.was_recovered());
    document
}

fn pixel(document: &Document) -> [u8; 4] {
    let page = pdf_model::Pages::new(document).get(0).expect("one page");
    let interpreted = pdf_model::interpret(document, &page);
    assert!(interpreted.is_complete(), "{:?}", interpreted.unsupported);
    let target = TargetSpec::for_page(&interpreted.display_list, 1.0, 1 << 20).expect("small page");
    let raster = CpuRasterizer::new()
        .rasterize(&interpreted.display_list, target)
        .expect("simple fill");
    let offset = ((5 * raster.width + 5) * 4) as usize;
    raster.data[offset..offset + 4]
        .try_into()
        .expect("interior pixel")
}

#[test]
fn a_registered_page_intent_without_embedded_profile_does_not_borrow_the_catalog_press() {
    let page_only = pixel(&fixture(false, true));
    let catalog_only = pixel(&fixture(true, false));
    assert_ne!(
        page_only, catalog_only,
        "the real catalog profile is a discriminating control"
    );
    assert_eq!(
        pixel(&fixture(true, true)),
        page_only,
        "the page's effective output condition is unchanged by the catalog's different press"
    );
}

#[test]
fn an_unavailable_page_profile_is_not_reported_as_the_catalogs_output_condition() {
    let catalog_only = pdf_model::output_intent::in_force_on_pages(&fixture(true, false));
    assert!(
        catalog_only
            .iter()
            .any(|intent| intent.condition.as_deref() == Some("Catalog GRACoL"))
    );
    let page_only = pdf_model::output_intent::in_force_on_pages(&fixture(false, true));
    assert!(
        !page_only
            .iter()
            .any(|intent| intent.condition.as_deref() == Some("Catalog GRACoL"))
    );
    let both = pdf_model::output_intent::in_force_on_pages(&fixture(true, true));
    assert!(
        !both
            .iter()
            .any(|intent| intent.condition.as_deref() == Some("Catalog GRACoL")),
        "the catalog condition is not in force on this page: {both:?}"
    );
}
