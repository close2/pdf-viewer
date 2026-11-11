//! Regressions for effective ISO 32000-2 §8.9.5.4 alternate-image selection.
//!
//! Fixtures use exact stream lengths and a real
//! classic xref table. Black base / white alternate samples make selected branches visible.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test fixtures are fully controlled and malformed output should fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::optional_content::Purpose;
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 30;
const GROUPS: &str = "/OCProperties << /OCGs [5 0 R 6 0 R] /D << /OFF [5 0 R] >> >>";
const DRAW: &str = "q 60 0 0 60 20 20 cm /Im Do Q";

fn image(number: u32, sample: u8, extra: &str) -> String {
    format!(
        "{number} 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1\n         /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /ASCIIHexDecode {extra}\n         /Length 3 >>\nstream\n{sample:02X}>\nendstream\nendobj\n"
    )
}

fn pdf(base: &str, alternates: &str, alternate: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R {GROUPS} >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100]\n          /Resources << /XObject << /Im 7 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{DRAW}\nendstream\nendobj\n{GROUPS_OBJECTS}{base}{alternates}{alternate}",
        DRAW.len().saturating_add(1),
    );
    assemble(&body)
}

const GROUPS_OBJECTS: &str = "5 0 obj\n<< /Type /OCG /Name (Hidden) >>\nendobj\n\
                               6 0 obj\n<< /Type /OCG /Name (Visible) >>\nendobj\n";

fn assemble(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn render(bytes: Vec<u8>, purpose: Purpose) -> pdf_render::Raster {
    let document = Document::open(bytes).expect("valid complete one-page fixture");
    assert!(!document.was_recovered(), "fixture xref is exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let mut state = pdf_model::view::ViewState::of(&document);
    state.set_purpose(purpose);
    let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
    assert!(
        interpretation.is_complete(),
        "fixture is fully supported: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported rasterization")
}

fn center(raster: &pdf_render::Raster) -> [u8; 4] {
    let row = raster.height.saturating_sub(1).saturating_sub(50);
    let at = ((row.saturating_mul(raster.width)).saturating_add(50) as usize).saturating_mul(4);
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

#[test]
fn print_without_designated_or_visible_alternate_falls_back_to_base() {
    let base = image(7, 0x00, "/Alternates 8 0 R");

    // Positive control: a true print designation makes the image stream, rather than the base,
    // the observed pixel under the same Print purpose and otherwise identical PDF structure.
    let designated = "8 0 obj\n[ << /Image 9 0 R /DefaultForPrinting true >> ]\nendobj\n";
    let alternate = image(9, 0xFF, "");
    let control = render(pdf(&base, designated, &alternate), Purpose::Print);
    assert_eq!(
        center(&control),
        [255, 255, 255, 255],
        "print control selects alternate"
    );

    // No /DefaultForPrinting (the Table 89 default is false) and no /OC on the dictionary, so
    // c) and d) identify none; EC3 step e) requires the black base image.
    let undesignated = "8 0 obj\n[ << /Image 9 0 R >> ]\nendobj\n";
    let actual = render(pdf(&base, undesignated, &alternate), Purpose::Print);
    assert_eq!(
        center(&actual),
        [0, 0, 0, 255],
        "step e) renders the base when c) and d) select no alternate, even for Print"
    );
}

#[test]
fn the_first_of_two_visible_alternates_wins_in_array_order() {
    // EC3#79 step d selects the FIRST visible alternate. Hidden-first/visible-second alone
    // cannot distinguish first-visible selection from last-visible selection.
    let base = image(7, 0, "/Alternates 8 0 R");
    let images = format!("{}{}", image(9, 64, ""), image(10, 255, ""));
    for purpose in [Purpose::View, Purpose::Print, Purpose::Export] {
        let forward =
            "8 0 obj\n[ << /Image 9 0 R /OC 6 0 R >> << /Image 10 0 R /OC 6 0 R >> ]\nendobj\n";
        let reverse =
            "8 0 obj\n[ << /Image 10 0 R /OC 6 0 R >> << /Image 9 0 R /OC 6 0 R >> ]\nendobj\n";
        assert_eq!(
            center(&render(pdf(&base, forward, &images), purpose)),
            [64, 64, 64, 255]
        );
        assert_eq!(
            center(&render(pdf(&base, reverse, &images), purpose)),
            [255, 255, 255, 255]
        );
    }
}
