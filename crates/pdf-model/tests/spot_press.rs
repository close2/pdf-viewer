//! ISO 32000-2 §10.8.3's simulated press drawn: the process pair and the spot planes put together
//! by steps b) to d) on the backend `CLAUDE.md` keeps as the oracle (ADR 1317).
//!
//! > - b) Convert each separation into "flat XYZ" (no gamma) and using a background matte of all
//! >   white.
//! > - c) Blend the resulting separations into a single result using a multiply blend (see
//! >   "Table 133 -Variables used in the basic compositing formula").
//! > - d) Convert the result to the actual device colour space and output it.
//!
//! # How each expected pixel is worked
//!
//! Every page here is drawn on the device's components, so its separation is made on step a)'s
//! press — "A default DestOutputProfile , if available for a subtractive device" — and with no
//! output intent that is the assumed inks, the sixteen sRGB corners of ADR 0263 interpolated
//! multilinearly. A separation's flat XYZ is then the corners' colour decoded by the sRGB transfer
//! function and taken through the sRGB-to-D50 matrix (§10.3.2 has a processor give its device
//! spaces a CIE definition, and this processor's is sRGB, ADR 0009); each enters step c) divided by
//! D50, the matte's white, because Table 136's NOTE 3 makes white the unit of the multiply; and
//! step d) is the matrix back and the transfer function, to eight bits. `LogoGreen` is §8.6.6.4's
//! EXAMPLE 2, whose calculator function at a tint `t` is CMYK `(0.84 t, 0, 0.44 t, 0.21 t)`.
//! The arithmetic, in the two separations the fixtures multiply:
//!
//! - process yellow, `0 0 1 0`, is the corner (255, 242, 0), whose flat XYZ over D50 is
//!   (0.8068, 0.8590, 0.1214);
//! - process black, `0 0 0 1`, is the corner (35, 31, 32): (0.01522, 0.01444, 0.01440);
//! - `LogoGreen` at 1, CMYK (0.84, 0, 0.44, 0.21), interpolates to sRGB (33, 148, 134) and is
//!   (0.1612, 0.2309, 0.2429).
//!
//! Yellow times `LogoGreen` is (0.1300, 0.1983, 0.0295) of D50, which is sRGB (69, 139, 0) — the
//! blue channel falls below zero in linear light and is clamped by step d)'s conversion — and black
//! times `LogoGreen` is sRGB (2, 13, 11). Each assertion allows one level for the sampled curves
//! the backend interpolates.
#![expect(
    clippy::doc_markdown,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "a test asserts, a fixture that will not load is a failed test, and the prose here \
              is largely quotation, which may not gain backticks"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;

/// §8.6.6.4's EXAMPLE 2, object 12.
const LOGO_GREEN: &str = "[/Separation /LogoGreen /DeviceCMYK 5 0 R]";

/// Object 5: EXAMPLE 2's PostScript calculator function, byte for byte.
const LOGO_GREEN_TINT: &str = "<< /FunctionType 4 /Domain [0.0 1.0] \
     /Range [0.0 1.0 0.0 1.0 0.0 1.0 0.0 1.0] /Length 62 >>\nstream\n\
     {dup 0.84 mul exch 0.00 exch dup 0.44 mul exch 0.21 mul}\nendstream";

/// The graphics states the fixtures overprint under.
const STATES: &str = "/ExtGState << /OP << /OP true /op true >> \
     /OPM << /OP true /op true /OPM 1 >> >>";

/// A 40-unit page on the device's components naming LogoGreen, with `content` its stream.
fn page(content: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
             /Resources << /ColorSpace << /LG {LOGO_GREEN} >> {STATES} >> /Contents 4 0 R >>"
        ),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len() + 1
        ),
        LOGO_GREEN_TINT.to_owned(),
    ];
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Page one of `bytes`, interpreted with §10.8.3's simulation on or off.
fn interpreted(bytes: Vec<u8>, simulate: bool) -> pdf_model::Interpretation {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let mut state = pdf_model::view::ViewState::of(&document);
    state.set_separation_simulation(simulate);
    pdf_model::content::interpret_with(&document, &page, &state)
}

/// The whole raster of an interpretation, from the CPU backend.
fn raster(interpretation: &pdf_model::Interpretation) -> pdf_render::Raster {
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("a 40x40 target");
    render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the fixture rasterises")
}

/// The centre pixel of a fixture drawn under the simulation, which must separate it.
fn pressed(content: &str) -> [u8; 3] {
    let interpretation = interpreted(page(content), true);
    assert!(
        interpretation.display_list.separation().is_some(),
        "a page naming LogoGreen is separated under the simulation"
    );
    let drawn = raster(&interpretation);
    let at = ((20 * drawn.width) + 20) as usize * 4;
    [drawn.data[at], drawn.data[at + 1], drawn.data[at + 2]]
}

/// Asserts a pixel to within one level of the worked value.
#[track_caller]
fn assert_near(found: [u8; 3], expected: [u8; 3], what: &str) {
    assert!(
        found
            .iter()
            .zip(expected)
            .all(|(found, expected)| found.abs_diff(expected) <= 1),
        "{what}: {found:?}, worked by hand as {expected:?}"
    );
}

/// One separation alone is its own colour: `LogoGreen` at 0.5 is CMYK (0.42, 0, 0.22, 0.105),
/// sRGB (134, 198, 183), and every other separation is the matte, the identity of the multiply.
#[test]
fn a_spot_alone_is_its_separations_colour() {
    assert_near(
        pressed("/LG cs 0.5 scn 0 0 40 40 re f"),
        [134, 198, 183],
        "LogoGreen at 0.5",
    );
}

/// §10.8.2's example is the reason the simulation exists — "that area will appear green when
/// produced using separations, overprinting of the cyan and yellow inks" — and on a page that also
/// names a spot colourant it is drawn by the separation's process pair: the green corner, (0, 166,
/// 80), with the empty spot plane's matte multiplied in.
#[test]
fn cyan_overprinting_yellow_is_green_on_the_press() {
    assert_near(
        pressed("1 0 0 0 k 0 0 40 40 re f /OPM gs 0 0 1 0 k 0 0 40 40 re f"),
        [0, 166, 80],
        "cyan then yellow under OPM 1",
    );
}

/// LogoGreen overprinting yellow: §11.7.4.3's second bullet keeps the backdrop's yellow — "𝐶𝑠 for
/// all colour components specified in the current colour space, otherwise 𝐶𝑏" — so the two inks
/// lie on one area of the press and step c) multiplies them: (69, 139, 0).
#[test]
fn logo_green_overprinting_yellow_is_the_product_of_the_two() {
    assert_near(
        pressed("0 0 1 0 k 0 0 40 40 re f /OP gs /LG cs 1 scn 0 0 40 40 re f"),
        [69, 139, 0],
        "yellow times LogoGreen",
    );
}

/// LogoGreen overprinting process black is darker than either: (2, 13, 11).
#[test]
fn logo_green_overprinting_black_is_the_product_of_the_two() {
    assert_near(
        pressed("0 0 0 1 k 0 0 40 40 re f /OP gs /LG cs 1 scn 0 0 40 40 re f"),
        [2, 13, 11],
        "black times LogoGreen",
    );
}

/// A process mark over LogoGreen, and whether it knocks the spot out. Without overprint §11.7.4.2
/// erases it: "For colour components whose value has not been specified, a source colour value of
/// 1.0 shall be assumed; when objects are fully opaque and the Normal blend mode is used, this shall
/// have the effect of erasing those components" — the page is yellow alone, (255, 242, 0). And
/// §11.7.4.3's first bullet keeps it where the mode is 1 — "For spot colour components, the value
/// shall always be 𝐶𝑏" — and the page is the product again.
#[test]
fn a_process_mark_knocks_the_spot_out_unless_it_overprints() {
    assert_near(
        pressed("/LG cs 1 scn 0 0 40 40 re f 0 0 1 0 k 0 0 40 40 re f"),
        [255, 242, 0],
        "no overprint: yellow erases LogoGreen",
    );
    assert_near(
        pressed("/LG cs 1 scn 0 0 40 40 re f /OPM gs 0 0 1 0 k 0 0 40 40 re f"),
        [69, 139, 0],
        "OPM 1: LogoGreen kept under the yellow",
    );
}

/// With the simulation off the same page draws the way §8.6.6.4 asks of a device without the
/// colourant, through the alternate: LogoGreen's mark is its CMYK, and the yellow under it is gone
/// (§10.8.2: on a screen "ignoring the overprint controls, will generally produce yellow, the last
/// colour painted" — here the last colour is LogoGreen's alternate, (33, 148, 134)).
#[test]
fn off_the_page_draws_in_the_alternate() {
    let interpretation = interpreted(
        page("0 0 1 0 k 0 0 40 40 re f /OP gs /LG cs 1 scn 0 0 40 40 re f"),
        false,
    );
    assert!(interpretation.display_list.separation().is_none());
    let drawn = raster(&interpretation);
    let at = ((20 * drawn.width) + 20) as usize * 4;
    assert_near(
        [drawn.data[at], drawn.data[at + 1], drawn.data[at + 2]],
        [33, 148, 134],
        "LogoGreen's alternate, painted last",
    );
}
