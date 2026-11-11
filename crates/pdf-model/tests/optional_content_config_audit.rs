//! Valid, self-contained tests for §8.11.4 configuration initialization.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "the constructed PDF fixtures are bounded and malformed fixtures should fail loudly"
)]

use std::io::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const PIXEL_BUDGET: u64 = 1 << 20;

fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        writeln!(&mut out, "{offset:010} 00000 n ").expect("writing to Vec cannot fail");
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

fn stream(data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< /Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

/// One valid PDF 1.7 page with a named OC group, the stated default configuration, and an
/// optional alternate configuration array. Every state-reference points to declared group 5.
fn fixture(default: &str, configs: &str, group_intent: &str) -> Vec<u8> {
    let config_entry = if configs.is_empty() {
        String::new()
    } else {
        format!("/Configs {configs}")
    };
    let catalog = format!(
        "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [5 0 R] /D {default} {config_entry} >> >>"
    );
    let group = format!("<< /Type /OCG /Name (Layer) {group_intent} >>");
    assemble(&[
        catalog.into_bytes(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Properties << /oc 5 0 R >> >> /Contents 4 0 R >>".to_vec(),
        stream(b"/OC /oc BDC 20 20 60 60 re f EMC"),
        group.into_bytes(),
    ])
}

fn center_alpha(bytes: Vec<u8>) -> u8 {
    let document = Document::open(bytes).expect("valid PDF fixture");
    assert!(!document.was_recovered(), "fixture has a complete xref");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "fixture uses supported optional content: {:?}",
        interpretation.unsupported
    );
    let display = interpretation.display_list;
    let target = TargetSpec::for_page(&display, 1.0, PIXEL_BUDGET).expect("100 by 100 target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&display, target)
        .expect("supported square mark");
    let row = raster.height - 1 - 50;
    let index = ((row * raster.width + 50) as usize) * 4 + 3;
    raster.data[index]
}

/// A well-formed alternate configuration is available, but the default D configuration is the
/// required initial state when opening the file. Controls run first and prove both states render.
#[test]
fn default_configuration_supplies_initial_state_despite_alternate() {
    let visible_control = center_alpha(fixture(
        "<< /BaseState /ON /ON [5 0 R] >>",
        "[<< /Name (Alternate) /BaseState /ON /OFF [5 0 R] >>]",
        "",
    ));
    assert!(
        visible_control > 0,
        "default D ON state is visible while the alternate is OFF"
    );

    let hidden = center_alpha(fixture(
        "<< /BaseState /ON /OFF [5 0 R] >>",
        "[<< /Name (Alternate) /BaseState /ON /ON [5 0 R] >>]",
        "",
    ));
    assert_eq!(
        hidden, 0,
        "opening uses D's OFF state; the available alternate does not replace it"
    );
}

/// Table 99 defaults the default configuration's Intent to View, and says any explicit value
/// there shall be View. The group-level Design intent consequently leaves it outside visibility.
#[test]
fn explicit_default_view_intent_filters_design_group() {
    let view_control = center_alpha(fixture(
        "<< /BaseState /ON /OFF [5 0 R] /Intent /View >>",
        "",
        "/Intent /View",
    ));
    assert_eq!(
        view_control, 0,
        "a View group participates and D turns it off"
    );

    let design_group = center_alpha(fixture(
        "<< /BaseState /ON /OFF [5 0 R] /Intent /View >>",
        "",
        "/Intent /Design",
    ));
    assert!(
        design_group > 0,
        "a Design-only group is ignored by the valid View configuration"
    );
}

#[test]
fn initial_view_usage_can_turn_on_an_off_group_without_affecting_missing_usage() {
    let on_usage = "/Usage << /View << /ViewState /ON >> >>";
    assert_eq!(
        center_alpha(fixture("<< /OFF [5 0 R] >>", "", on_usage)),
        0,
        "a usage dictionary alone does not override the default configuration"
    );
    let with_view = "<< /OFF [5 0 R] /AS [<< /Event /View /Category [/View] /OCGs [5 0 R] >>] >>";
    assert_eq!(
        center_alpha(fixture(with_view, "", on_usage)),
        255,
        "the initial View event assigns ON, rather than ANDing with initial OFF"
    );
    assert_eq!(
        center_alpha(fixture(with_view, "", "")),
        0,
        "a group without the requested usage category receives no recommendation"
    );
    let with_print = "<< /OFF [5 0 R] /AS [<< /Event /Print /Category [/View] /OCGs [5 0 R] >>] >>";
    assert_eq!(
        center_alpha(fixture(with_print, "", on_usage)),
        0,
        "the event, rather than its category name, decides whether the dictionary applies on opening"
    );
}
