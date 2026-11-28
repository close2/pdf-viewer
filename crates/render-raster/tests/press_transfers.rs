//! ISO 32000-2 §10.5's transfer functions on §10.8.3's simulated press, a CMYK device, on every
//! lane that draws it: the CPU oracle, `rasterize`, and the window's present
//! (`rasterize_frame`, ADR 3701). `render-gpu` refuses a list with a transfer channel, and a
//! separated page, by name and hands the frame to the CPU oracle, so it has no picture of its own
//! to compare; [`agree`] asks it for the refusal. ADR 3769, ADR 3770, ADR 3806 to ADR 3808, and
//! ADR 3857 for a page whose group composites in one or three components.
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
    page_with("", resources, data)
}

/// [`page`] whose page group states the blending colour space `space` (§11.4.7).
fn grouped(space: &str, resources: &str, data: &str) -> Vec<u8> {
    page_with(
        &format!("/Group << /S /Transparency /CS {space} >>"),
        resources,
        data,
    )
}

/// [`page`] with `entries` more in the page dictionary.
fn page_with(entries: &str, resources: &str, data: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] /Resources << {resources} >> \
             {entries} /Contents 4 0 R >>"
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

/// Every lane within a level of rounding of the oracle, and the oracle as `expected` says; and
/// `render-gpu`, which has no pass over its result on either device, refusing the page by name.
fn agree(list: &Arc<DisplayList>, expected: impl Fn([u8; 3]) -> bool, what: &str) {
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("a 40 by 40 target");
    {
        // `render-gpu` makes its device outside `raster_gpu`'s hook, so the lock is held around
        // its whole life here and released before the hooked lanes take it.
        let _gpu = gpu_lock::Hold::unless_software("vello-device");
        let mut vello = render_gpu::GpuRasterizer::new_headless().expect("a device for render-gpu");
        let refused = vello.rasterize(list, target);
        assert!(
            matches!(
                &refused,
                Err(render_gpu::GpuRasterError::UnsupportedCommand(_))
            ),
            "{what}: render-gpu draws a page it has no pass for: {refused:?}"
        );
    }
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

/// A Type 5 halftone whose cyan and `/Default` take [`ZERO`] and whose magenta, yellow and black
/// take the identity: on the press a component that takes its function is full ink, and one left
/// to the page's default keeps its value.
fn cyan_and_spots_zero() -> String {
    let screen = "/HalftoneType 1 /Frequency 60 /Angle 0 /SpotFunction /Round";
    format!(
        "<< /HalftoneType 5 \
         /Cyan << {screen} /TransferFunction {ZERO} >> \
         /Magenta << {screen} /TransferFunction /Identity >> \
         /Yellow << {screen} /TransferFunction /Identity >> \
         /Black << {screen} /TransferFunction /Identity >> \
         /Default << {screen} /TransferFunction {ZERO} >> >>"
    )
}

/// A spot colourant `/Spot` whose alternate at full tint is red.
const SPOT: &str = "/ColorSpace << /CS0 [/Separation /Spot /DeviceRGB << /FunctionType 2 \
                    /Domain [0 1] /C0 [1 1 1] /C1 [1 0 0] /N 1 >>] >>";

/// §11.7.5.2: "If a graphics object is painted with overprinting enabled … the halftone and
/// transfer function to use at a given point shall be determined independently for each colour
/// component. An object is opaque for a given component only if overprinting yields the source
/// colour (not the backdrop colour) for that component." A process colour over a spot one: Table
/// 146 gives `0 0 0 0 k` under `OP` true and `OPM` 0 `C_s` on the four process components and `C_b`
/// on the spot, so on the press its four take their functions — cyan's [`ZERO`] is full cyan ink —
/// and the spot colourant takes the page's default and keeps its tint 0. Cyan, where the function
/// on every component would add full red spot ink and the default on every component would leave
/// the page white.
#[test]
fn every_lane_gives_a_process_overprint_its_function_on_the_process_components_alone() {
    gpu_lock_hook::install();
    let resources = format!(
        "/ExtGState << /GS << /OP true /op true /OPM 0 /HT {} >> >> {SPOT}",
        cyan_and_spots_zero()
    );
    let content = "/CS0 cs 0 scn 0 0 40 40 re f /GS gs 0 0 0 0 k 0 0 40 40 re f";
    agree(
        &interpreted(page(&resources, content), true),
        |[red, green, blue]| red < 100 && green > 120 && blue > 120,
        "a process mark overprinting a spot takes its function on cyan alone",
    );
}

/// The same paragraph the other way round: a spot colour at tint 0 overprinting a white page.
/// Table 146 gives it `C_s` on its own spot colourant and `C_b` on the four process components, so
/// on the press the spot takes `/Default`'s [`ZERO`] — full spot ink, the alternate's red — and
/// cyan takes the page's default and stays without ink. Red, where the function on every component
/// would add full cyan and the default on every component would leave the page white.
#[test]
fn every_lane_gives_a_spot_overprint_its_function_on_the_spot_alone() {
    gpu_lock_hook::install();
    let resources = format!(
        "/ExtGState << /GS << /OP true /op true /OPM 0 /HT {} >> >> {SPOT}",
        cyan_and_spots_zero()
    );
    let content = "/GS gs /CS0 cs 0 scn 0 0 40 40 re f";
    agree(
        &interpreted(page(&resources, content), true),
        |[red, green, blue]| red > 200 && green < 100 && blue < 100,
        "a spot mark overprinting takes its function on the spot alone",
    );
}

/// §10.5's special case — "When the current colour space is DeviceGray and the output device's
/// native colour space is DeviceCMYK , a PDF processor shall use only the gray transfer function"
/// — for "colours specified in the DeviceGray colour space" by an image's samples. A one-sample
/// `DeviceGray` image of white under `/TR [zero zero zero identity]`: black on the screen, which
/// takes the first three, and white on the press, whose cyan, magenta and yellow are 0 and
/// "rendered directly".
#[test]
#[expect(
    clippy::doc_markdown,
    reason = "the paragraph quotes §10.5 verbatim, and a quotation may not gain backticks"
)]
fn every_lane_gives_a_grey_images_samples_the_gray_function_alone_on_the_press() {
    gpu_lock_hook::install();
    let resources = format!("/ExtGState << /GS << /TR [{ZERO} {ZERO} {ZERO} {IDENTITY}] >> >>");
    let content = "/GS gs q 40 0 0 40 0 0 cm BI /W 1 /H 1 /CS /G /BPC 8 /F /AHx ID ff> EI Q";
    agree(
        &interpreted(page(&resources, content), false),
        |colour| colour.iter().all(|&level| level < 8),
        "control: the screen takes the three zero functions",
    );
    agree(
        &interpreted(page(&resources, content), true),
        |colour| colour.iter().all(|&level| level > 240),
        "a DeviceGray image's white on the press takes the gray function alone",
    );
}

/// §10.6.5.6, Table 132's "any colourant name": "The halftone corresponding to the colourant or
/// colour component named by the key", whose `TransferFunction` overrides the current transfer
/// function "for the same component" (Table 128). A Type 5 halftone gives `/Spot` its own
/// [`ZERO`] and `/Default` the identity, so a spot colour at tint 0 is full spot ink on the press —
/// red — where `/Default`'s function would leave it white; and the screen, whose red, green and
/// blue have no entry and take `/Default`'s identity, is white.
#[test]
fn every_lane_puts_a_type5_entry_on_the_spot_colourant_it_names() {
    gpu_lock_hook::install();
    let screen = "/HalftoneType 1 /Frequency 60 /Angle 0 /SpotFunction /Round";
    let halftone = format!(
        "<< /HalftoneType 5 \
         /Spot << {screen} /TransferFunction {ZERO} >> \
         /Default << {screen} /TransferFunction /Identity >> >>"
    );
    let resources = format!("/ExtGState << /GS << /HT {halftone} >> >> {SPOT}");
    let content = "/GS gs /CS0 cs 0 scn 0 0 40 40 re f";
    agree(
        &interpreted(page(&resources, content), false),
        |colour| colour.iter().all(|&level| level > 247),
        "control: the screen takes the default's identity",
    );
    agree(
        &interpreted(page(&resources, content), true),
        |[red, green, blue]| red > 200 && green < 100 && blue < 100,
        "the press's spot colourant takes the entry that names it",
    );
}

/// §11.7.4.2: "If the group colour space is different from the native colour space of the output
/// device, … the blending computations shall affect the process colour ants only after the
/// group's results have been converted to the device colour space", and §10.5 puts the functions
/// in that space. `/TR [identity identity identity zero]` over `0 0 0 0 k` on a page compositing
/// in `DeviceRGB`: the group's white is no ink on the press, whose black 0 is additive 1, and the
/// fourth function makes it full black ink. The screen takes the first three and stays white.
#[test]
fn every_lane_puts_the_press_functions_on_an_rgb_page_groups_result() {
    gpu_lock_hook::install();
    let resources =
        format!("/ExtGState << /GS << /TR [{IDENTITY} {IDENTITY} {IDENTITY} {ZERO}] >> >>");
    let content = "/GS gs 0 0 0 0 k 0 0 40 40 re f";
    agree(
        &interpreted(grouped("/DeviceRGB", &resources, content), false),
        |colour| colour.iter().all(|&level| level > 247),
        "control: the screen is an RGB device",
    );
    agree(
        &interpreted(grouped("/DeviceRGB", &resources, content), true),
        |colour| colour.iter().all(|&level| level < 100),
        "the simulated press's black takes the fourth function after the group's conversion",
    );
}

/// The same on a page compositing in `DeviceGray`: its white `1 g` is no ink on the press, and the
/// fourth function makes the black full ink.
#[test]
fn every_lane_puts_the_press_functions_on_a_grey_page_groups_result() {
    gpu_lock_hook::install();
    let resources =
        format!("/ExtGState << /GS << /TR [{IDENTITY} {IDENTITY} {IDENTITY} {ZERO}] >> >>");
    let content = "/GS gs 1 g 0 0 40 40 re f";
    agree(
        &interpreted(grouped("/DeviceGray", &resources, content), false),
        |colour| colour.iter().all(|&level| level > 247),
        "control: the screen is an RGB device",
    );
    agree(
        &interpreted(grouped("/DeviceGray", &resources, content), true),
        |colour| colour.iter().all(|&level| level < 100),
        "the simulated press's black takes the fourth function after the group's conversion",
    );
}

/// A `CalRGB` page group, the three-component CIE-based case: its result leaves through the cube
/// onto the press rather than its own cube to the screen, so the press's fourth function reaches
/// it as it reaches a `DeviceRGB` group's.
#[test]
fn every_lane_puts_the_press_functions_on_a_calrgb_page_groups_result() {
    gpu_lock_hook::install();
    let resources =
        format!("/ExtGState << /GS << /TR [{IDENTITY} {IDENTITY} {IDENTITY} {ZERO}] >> >>");
    let content = "/GS gs 1 1 1 rg 0 0 40 40 re f";
    let space = "[/CalRGB << /WhitePoint [0.9505 1 1.089] /Gamma [2.2 2.2 2.2] >>]";
    agree(
        &interpreted(grouped(space, &resources, content), false),
        |colour| colour.iter().all(|&level| level > 230),
        "control: the screen is an RGB device, and the space's white near the device's",
    );
    agree(
        &interpreted(grouped(space, &resources, content), true),
        |colour| colour.iter().all(|&level| level < 100),
        "the simulated press's black takes the fourth function after the group's conversion",
    );
}

/// A spot colourant on a page compositing in `DeviceRGB`: the spot plane takes a Type 5
/// halftone's `/Default` and the process plates, converted onto the press, its four named
/// entries. Tint 0 is then full red spot ink over an unprinted process separation; the screen,
/// whose red, green and blue take `/Default`'s zero, is black.
#[test]
fn every_lane_puts_the_spot_function_on_an_rgb_page_groups_spot_plane() {
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
    let resources = format!("/ExtGState << /GS << /HT {halftone} >> >> {SPOT}");
    let content = "/GS gs /CS0 cs 0 scn 0 0 40 40 re f";
    agree(
        &interpreted(grouped("/DeviceRGB", &resources, content), false),
        |colour| colour.iter().all(|&level| level < 8),
        "control: the screen's red, green and blue take the halftone's default",
    );
    agree(
        &interpreted(grouped("/DeviceRGB", &resources, content), true),
        |[red, green, blue]| red > 200 && green < 100 && blue < 100,
        "the spot plane takes the default and the converted process plates the identity",
    );
}

/// What the round trip costs a page stating no transfer: a mid grey on a page compositing in
/// `DeviceRGB` is inside the press's gamut, and the press's conversion in is a right inverse of
/// its conversion out (`pdf_colour`'s ink search), so it comes back within the cube's
/// interpolation and the planes' quantisation. The press's picture of an in-gamut colour is that
/// colour.
#[test]
fn every_lane_returns_an_in_gamut_grey_through_the_press() {
    gpu_lock_hook::install();
    let content = "0.5 0.5 0.5 rg 0 0 40 40 re f";
    let list = interpreted(grouped("/DeviceRGB", "", content), true);
    assert!(
        list.press_conversion().is_some(),
        "the page is carried onto the press"
    );
    let screen = centre(
        &render_cpu::CpuRasterizer::new()
            .rasterize(
                &interpreted(grouped("/DeviceRGB", "", content), false),
                TargetSpec::for_page(&list, 1.0, 1 << 20).expect("a 40 by 40 target"),
            )
            .expect("the oracle draws the page"),
    );
    agree(
        &list,
        |colour| colour.iter().zip(screen).all(|(&a, b)| a.abs_diff(b) <= 3),
        "a mid grey through the press",
    );
}
