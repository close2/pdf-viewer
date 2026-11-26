//! ISO 32000-2 §10.5's transfer functions on §10.8.3's simulated press, a CMYK device, on every
//! lane that draws it: the CPU oracle, `rasterize`, and the window's present
//! (`rasterize_frame`, ADR 3701). `render-gpu` refuses a list with a transfer channel and hands
//! the frame to the CPU oracle, so it has no picture of its own to compare. ADR 3769, ADR 3770.
//!
//! `pdf-model`'s `rendering_10_5_10_8_spec_audit` asks the oracle alone; a test of one lane is not
//! a test of the window (review 2248).

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "exact-xref test fixtures, their assertions and a small raster's pixel index"
)]

#[path = "../../../tools/conformance/test-support/gpu_lock.rs"]
mod gpu_lock;
#[path = "../../../tools/conformance/test-support/gpu_lock_hook.rs"]
mod gpu_lock_hook;

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_render::{DisplayList, Raster, Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_raster::{PresentFrame, QuorraRasterizer};

/// A type 2 function from 0 to 1: the identity on 0..1.
const IDENTITY: &str = "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>";
/// A type 2 function that answers 0 everywhere.
const ZERO: &str = "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [0] /N 1 >>";

/// A one-page classic-xref PDF, 40 points square, with `resources` and the content `data`.
fn page(resources: &str, data: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] /Resources << {resources} >> \
             /Contents 4 0 R >>"
        ),
        format!(
            "<< /Length {} >>\nstream\n{data}\nendstream",
            data.len().saturating_add(1)
        ),
    ];
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{body}\nendobj", index.saturating_add(1));
    }
    let xref_at = out.len();
    let count = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {count}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {count} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// `bytes`' one page as `pdf_model` interprets it, on §10.8.3's simulated press when `simulate`.
fn interpreted(bytes: Vec<u8>, simulate: bool) -> Arc<DisplayList> {
    let document = Document::open(bytes).expect("the exact-xref PDF opens");
    assert!(!document.was_recovered(), "xref offsets are exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let mut state = pdf_model::view::ViewState::of(&document);
    state.set_separation_simulation(simulate);
    let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
    assert!(
        interpretation.is_complete(),
        "the fixture draws completely: {:?}",
        interpretation.unsupported
    );
    Arc::new(interpretation.display_list)
}

/// The colour at the raster's centre.
fn centre(raster: &Raster) -> [u8; 3] {
    let at = usize::try_from((raster.height / 2) * raster.width + raster.width / 2)
        .expect("a pixel index")
        * 4;
    [raster.data[at], raster.data[at + 1], raster.data[at + 2]]
}

/// The page's centre on the CPU oracle, through `rasterize` and through the window's frame.
fn three_lanes(list: &Arc<DisplayList>) -> ([u8; 3], [u8; 3], [u8; 3], String) {
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("a 40 by 40 target");
    let oracle = render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the oracle draws the page");
    let mut gpu = QuorraRasterizer::new_headless().expect("a device for the window's lane");
    let whole = gpu
        .rasterize(list, target)
        .expect("the device draws the page");
    let frame = gpu
        .rasterize_frame(&PresentFrame {
            width: target.width,
            height: target.height,
            pages: &[(list, target)],
            raster: None,
            overlays: &[],
        })
        .expect("the window's present draws the page");
    (
        centre(&oracle),
        centre(&whole),
        centre(&frame),
        gpu.adapter_description().to_owned(),
    )
}

/// Every lane within a level of rounding of the oracle, and the oracle as `expected` says.
fn agree(list: &Arc<DisplayList>, expected: impl Fn([u8; 3]) -> bool, what: &str) {
    let (oracle, drawn, shown, adapter) = three_lanes(list);
    assert!(expected(oracle), "{what}: the oracle draws {oracle:?}");
    for (lane, colour) in [("rasterize", drawn), ("the window's frame", shown)] {
        assert!(
            colour.iter().zip(oracle).all(|(&a, b)| a.abs_diff(b) <= 8),
            "{what}: {lane} draws {colour:?} where the oracle draws {oracle:?}, on {adapter}"
        );
    }
}

/// §10.5: "a CMYK device shall use all four", on the subtractive component "converted to additive
/// form by subtracting it from 1.0". `/TR [identity identity identity zero]` over `0 0 0 0 k`:
/// the screen takes the first three and stays white; the press's black 0 is additive 1, the fourth
/// function makes it 0, and that is full black ink.
#[test]
fn every_lane_puts_the_fourth_function_on_the_press_black_plane() {
    gpu_lock_hook::install();
    let resources =
        format!("/ExtGState << /GS << /TR [{IDENTITY} {IDENTITY} {IDENTITY} {ZERO}] >> >>");
    let content = "/GS gs 0 0 0 0 k 0 0 40 40 re f";
    agree(
        &interpreted(page(&resources, content), false),
        |colour| colour.iter().all(|&level| level > 247),
        "control: the screen is an RGB device",
    );
    agree(
        &interpreted(page(&resources, content), true),
        |colour| colour.iter().all(|&level| level < 100),
        "the simulated press's black takes the fourth function",
    );
}

/// §10.5: "When the current colour space is `DeviceGray` and the output device's native colour
/// space is `DeviceCMYK` , a PDF processor shall use only the gray transfer function."
/// `/TR [zero zero zero identity]` over `1 g`: black on the screen, white on the press.
#[test]
fn every_lane_gives_a_grey_the_gray_function_alone_on_the_press() {
    gpu_lock_hook::install();
    let resources = format!("/ExtGState << /GS << /TR [{ZERO} {ZERO} {ZERO} {IDENTITY}] >> >>");
    let content = "/GS gs 1 g 0 0 40 40 re f";
    agree(
        &interpreted(page(&resources, content), false),
        |colour| colour.iter().all(|&level| level < 8),
        "control: the screen takes the three zero functions",
    );
    agree(
        &interpreted(page(&resources, content), true),
        |colour| colour.iter().all(|&level| level > 240),
        "a DeviceGray white on the press takes the gray function alone",
    );
}

/// §10.5: a halftone dictionary "is the only way to set transfer functions for nonprimary colour
/// components", and §10.6.5.6 names a Type 5 halftone's primaries per device — "Red , Green , and
/// Blue for `DeviceRGB` ; Cyan , Magenta , Yellow , and Black for `DeviceCMYK`". The halftone below
/// gives the press's four the identity and everything else, through `/Default`, zero; a spot
/// colourant painted at tint 0 is then full spot ink on the press — its alternate's red — while
/// the screen, whose red, green and blue take `/Default`'s zero, is black.
#[test]
fn every_lane_puts_a_halftones_default_on_the_press_spot_plane() {
    gpu_lock_hook::install();
    let screen = "/HalftoneType 1 /Frequency 60 /Angle 0 /SpotFunction /Round";
    let halftone = format!(
        "<< /HalftoneType 5 \
         /Cyan << {screen} /TransferFunction /Identity >> \
         /Magenta << {screen} /TransferFunction /Identity >> \
         /Yellow << {screen} /TransferFunction /Identity >> \
         /Black << {screen} /TransferFunction /Identity >> \
         /Default << {screen} /TransferFunction {ZERO} >> >>"
    );
    let resources = format!(
        "/ExtGState << /GS << /HT {halftone} >> >> /ColorSpace << /CS0 [/Separation /Spot \
         /DeviceRGB << /FunctionType 2 /Domain [0 1] /C0 [1 1 1] /C1 [1 0 0] /N 1 >>] >>"
    );
    let content = "/GS gs /CS0 cs 0 scn 0 0 40 40 re f";
    agree(
        &interpreted(page(&resources, content), false),
        |colour| colour.iter().all(|&level| level < 8),
        "control: the screen's red, green and blue take the halftone's default",
    );
    agree(
        &interpreted(page(&resources, content), true),
        |[red, green, blue]| red > 200 && green < 100 && blue < 100,
        "the press's spot plane takes the halftone's default and its process plates the identity",
    );
}
