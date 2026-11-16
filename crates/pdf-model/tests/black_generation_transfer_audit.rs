//! Integration assertions: §10.4.2.4's BG/UCR are used for RGB -> CMYK group conversion;
//! §10.5's transfer is applied later in the native RGB device space.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "bounded 100-unit exact-xref fixture and center pixel index"
)]

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use std::fmt::Write as _;

fn page(extgstate: &str, paint: &str) -> Vec<u8> {
    let content = format!("/G gs {paint}");
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Group << /S /Transparency /CS /DeviceCMYK >> \
         /Resources << /ExtGState << /G << /Type /ExtGState {extgstate} >> >> >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len() + 1
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

fn center(bytes: Vec<u8>) -> [u8; 3] {
    let document = Document::open(bytes).expect("fixture parses");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "{:?}",
        interpretation.unsupported
    );
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("100-point target");
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("fixture rasterizes");
    assert_eq!((raster.width, raster.height), (100, 100));
    let at = ((50 * raster.width + 50) * 4) as usize;
    assert_eq!(raster.data[at + 3], 255);
    [raster.data[at], raster.data[at + 1], raster.data[at + 2]]
}

const BG_INVERT: &str = "<< /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >>";
const UCR_HALF: &str = "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [0.5] /N 1 >>";
const TRANSFER_INVERT: &str = "<< /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >>";

#[test]
fn rgb_to_cmyk_group_uses_bg_ucr_before_native_rgb_transfer() {
    let rgb = "0.2 0.4 0.6 rg 0 0 100 100 re f";
    let no_pair = center(page("", rgb));
    let explicit_default = center(page("/BG2 /Default /UCR2 /Default /TR2 /Default", rgb));
    assert_eq!(
        explicit_default, no_pair,
        "all three /Default entries restore page-start defaults"
    );

    // Independent §10.4.2.4 calculation:
    // c=.8, m=.6, y=.4, k=.4; BG(k)=1-k=.6 and UCR(k)=k/2=.2,
    // hence final DeviceCMYK=(.6,.4,.2,.6).
    let pair = format!("/BG2 {BG_INVERT} /UCR2 {UCR_HALF} /TR2 /Identity");
    let generated = center(page(&pair, rgb));
    let explicit_inks = center(page(&pair, "0.6 0.4 0.2 0.6 k 0 0 100 100 re f"));
    assert_eq!(
        generated, explicit_inks,
        "the independently stated CMYK values are the control"
    );

    // The screen is native RGB. §10.5 transfer follows conversion, so each final RGB channel is
    // complemented. Applying transfer to RGB first would produce a different CMYK group input.
    let with_transfer = format!("/BG2 {BG_INVERT} /UCR2 {UCR_HALF} /TR2 {TRANSFER_INVERT}");
    let transferred = center(page(&with_transfer, rgb));
    assert_eq!(
        transferred,
        generated.map(|component| 255 - component),
        "transfer is after conversion into and out of the CMYK transparency group"
    );
}

#[test]
fn negative_ucr_adds_colourant_and_the_final_channels_are_clamped() {
    let negative_ucr = "<< /FunctionType 2 /Domain [0 1] /Range [-1 1] \
        /C0 [-0.5] /C1 [-0.5] /N 1 >>";
    // For RGB=(.2,.7,.4), c=.8, m=.3, y=.6, k=.3. UCR(k)=-.5
    // therefore yields (1.3,.8,1.1,.3), clamped by §10.4.2.4 to (1,.8,1,.3).
    let from_rgb = center(page(
        &format!("/UCR2 {negative_ucr}"),
        "0.2 0.7 0.4 rg 0 0 100 100 re f",
    ));
    let stated_cmyk = center(page("", "1 0.8 1 0.3 k 0 0 100 100 re f"));
    assert_eq!(
        from_rgb, stated_cmyk,
        "negative UCR adds ink; output channels clamp to [0,1]"
    );
}
