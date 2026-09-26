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

/// The graphics states the fixtures overprint under, and one that composites under Multiply.
const STATES: &str = "/ExtGState << /OP << /OP true /op true >> \
     /OPM << /OP true /op true /OPM 1 >> /MUL << /BM /Multiply >> >>";

/// A 40-unit page on the device's components naming LogoGreen, with `content` its stream.
fn page(content: &str) -> Vec<u8> {
    page_in("", &[], content)
}

/// A 40-unit page naming LogoGreen whose page dictionary also states `group` — a `/Group` entry
/// or nothing — with `extra` as objects 6 onwards, reachable from the page's resources as the
/// form XObject `/Fm` where there is one, and `content` its stream.
fn page_in(group: &str, extra: &[String], content: &str) -> Vec<u8> {
    let form = if extra.is_empty() {
        ""
    } else {
        "/XObject << /Fm 6 0 R >>"
    };
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] {group} \
             /Resources << /ColorSpace << /LG {LOGO_GREEN} /Profiled [/ICCBased 7 0 R] >> \
             {STATES} {form} >> /Contents 4 0 R >>"
        ),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len() + 1
        ),
        LOGO_GREEN_TINT.to_owned(),
    ];
    objects.extend(extra.iter().cloned());
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
    pressed_page(page(content))
}

/// The centre pixel of any fixture page drawn under the simulation, which must separate it.
fn pressed_page(bytes: Vec<u8>) -> [u8; 3] {
    let interpretation = interpreted(bytes, true);
    assert!(
        interpretation.display_list.separation().is_some(),
        "a page naming LogoGreen is separated under the simulation: {:?}",
        interpretation.unsupported
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

/// The page group ISO 32000-2 §11.4.7 composites in, where it is not a press.
const RGB_GROUP: &str = "/Group << /S /Transparency /CS /DeviceRGB >>";

/// The same, in one component.
const GREY_GROUP: &str = "/Group << /S /Transparency /CS /DeviceGray >>";

/// The same, in the three components of [`linear_srgb_profile`].
const PROFILED_GROUP: &str = "/Group << /S /Transparency /CS [/ICCBased 7 0 R] >>";

/// Object 6, a placeholder where a fixture needs object 7 alone, and object 7: an `ICCBased`
/// stream over [`linear_srgb_profile`].
fn profile_objects() -> Vec<String> {
    let mut hex = String::new();
    for byte in linear_srgb_profile() {
        let _ = write!(hex, "{byte:02X}");
    }
    vec![
        "null".to_owned(),
        format!(
            "<< /N 3 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}>\nendstream",
            hex.len() + 1
        ),
    ]
}

/// A v2 'RGB ' display profile in the matrix form whose components are linear sRGB: sRGB's
/// D50-adapted colourants as its three `XYZ ` columns and a gamma of 1.0 on each tone curve —
/// `transparency_groups.rs`'s `icc_rgb_profile`, so its components are linear light and a
/// component of 0.5 is half of D50.
fn linear_srgb_profile() -> Vec<u8> {
    let mut header = vec![0u8; 128];
    header[8] = 2;
    header[12..16].copy_from_slice(b"mntr");
    header[16..20].copy_from_slice(b"RGB ");
    header[20..24].copy_from_slice(b"XYZ ");
    header[36..40].copy_from_slice(b"acsp");
    let xyz_tag = |column: [f32; 3]| {
        let mut tag = Vec::new();
        tag.extend_from_slice(b"XYZ ");
        tag.extend_from_slice(&[0; 4]);
        for value in column {
            #[expect(clippy::cast_possible_truncation, reason = "a colourant below 1.0")]
            tag.extend_from_slice(&((value * 65536.0) as i32).to_be_bytes());
        }
        tag
    };
    let mut curve = Vec::new();
    curve.extend_from_slice(b"curv");
    curve.extend_from_slice(&[0; 4]);
    curve.extend_from_slice(&1u32.to_be_bytes());
    curve.extend_from_slice(&0x0100u16.to_be_bytes());
    curve.extend_from_slice(&[0; 2]);
    let tags: [(&[u8; 4], Vec<u8>); 6] = [
        (b"rXYZ", xyz_tag([0.4361, 0.2225, 0.0139])),
        (b"gXYZ", xyz_tag([0.3851, 0.7169, 0.0971])),
        (b"bXYZ", xyz_tag([0.1431, 0.0606, 0.7141])),
        (b"rTRC", curve.clone()),
        (b"gTRC", curve.clone()),
        (b"bTRC", curve),
    ];
    let mut out = header;
    out.extend_from_slice(&6u32.to_be_bytes());
    let mut offset = 128 + 4 + 6 * 12;
    for (name, tag) in &tags {
        out.extend_from_slice(*name);
        out.extend_from_slice(&u32::try_from(offset).expect("small").to_be_bytes());
        out.extend_from_slice(&u32::try_from(tag.len()).expect("small").to_be_bytes());
        offset += tag.len();
    }
    for (_, tag) in &tags {
        out.extend_from_slice(tag);
    }
    out
}

/// A page whose group is `/DeviceRGB` is separated too, and its spot colourant passes through the
/// page group untouched — §11.7.3: "A spot colour retains its own identity; it shall not be subject
/// to conversion to or from the colour space of the enclosing transparency group or page." The
/// process separation is the page in its own three components, where LogoGreen painted
/// "an additive value of 1.0", white, so LogoGreen at 0.5 alone is (134, 198, 183) as on a page with
/// no group. And over an RGB yellow under overprint, Table 146's "Separation or DeviceN" row keeps
/// the process components — 𝐶𝑏 under `OP true` — so step c) multiplies the two: RGB yellow is linear
/// (1, 1, 0), whose flat XYZ over D50 is (0.8516, 0.9394, 0.1346); times LogoGreen's (0.1612,
/// 0.2309, 0.2429) and D50 it is XYZ (0.1324, 0.2169, 0.0270), which step d) takes to linear sRGB
/// (0.0509, 0.2870, −0.0022) and to eight bits (64, 146, 0). Drawn one operation at a time, as the
/// page was before, the last mark is LogoGreen's alternate over the yellow, (33, 148, 134).
#[test]
fn a_device_rgb_page_group_is_separated() {
    assert_near(
        pressed_page(page_in(RGB_GROUP, &[], "/LG cs 0.5 scn 0 0 40 40 re f")),
        [134, 198, 183],
        "LogoGreen at 0.5 on a /DeviceRGB page",
    );
    assert_near(
        pressed_page(page_in(
            RGB_GROUP,
            &[],
            "1 1 0 rg 0 0 40 40 re f /OP gs /LG cs 1 scn 0 0 40 40 re f",
        )),
        [64, 146, 0],
        "RGB yellow times LogoGreen",
    );
}

/// A process mark over LogoGreen on a `/DeviceRGB` page group: without overprint it erases the
/// spot — §11.7.4.2's "additive value of 1.0 … shall have the effect of erasing those components" —
/// and the page is RGB yellow alone, (255, 255, 0); under overprint Table 146's "Any process colour
/// space" row keeps the spot colourant, 𝐶𝑏, and the page is the product (64, 146, 0) again.
#[test]
fn a_process_mark_on_an_rgb_page_keeps_the_spot_only_where_it_overprints() {
    assert_near(
        pressed_page(page_in(
            RGB_GROUP,
            &[],
            "/LG cs 1 scn 0 0 40 40 re f 1 1 0 rg 0 0 40 40 re f",
        )),
        [255, 255, 0],
        "no overprint: yellow erases LogoGreen",
    );
    assert_near(
        pressed_page(page_in(
            RGB_GROUP,
            &[],
            "/LG cs 1 scn 0 0 40 40 re f /OP gs 1 1 0 rg 0 0 40 40 re f",
        )),
        [64, 146, 0],
        "overprint: LogoGreen kept under the yellow",
    );
}

/// The same on a three-component `ICCBased` page group, whose components are linear sRGB: a
/// `DeviceRGB` grey of 0.5 is the group's own components by §11.7.2 — "the CIE-based space of the
/// nearest such ancestor" — so the process separation is half of D50, (0.5, 0.5, 0.5) of the matte,
/// where the same operands on a `/DeviceRGB` group would be sRGB 0.5. Times LogoGreen at 1 and D50
/// it is XYZ (0.0777, 0.1155, 0.1002), linear sRGB (0.0077, 0.1485, 0.1195), and (21, 108, 97).
#[test]
fn an_icc_based_page_group_is_separated_in_its_components() {
    assert_near(
        pressed_page(page_in(
            PROFILED_GROUP,
            &profile_objects(),
            "/LG cs 0.5 scn 0 0 40 40 re f",
        )),
        [134, 198, 183],
        "LogoGreen at 0.5 on an ICCBased page",
    );
    assert_near(
        pressed_page(page_in(
            PROFILED_GROUP,
            &profile_objects(),
            "0.5 0.5 0.5 rg 0 0 40 40 re f /OP gs /LG cs 1 scn 0 0 40 40 re f",
        )),
        [21, 108, 97],
        "linear half grey times LogoGreen",
    );
}

/// A `/DeviceGray` page group, one component: LogoGreen alone paints the grey white and is its
/// own colour, (134, 198, 183); and a grey of 0.5 is sRGB 0.5 on the device, linear 0.2140 of D50,
/// which times LogoGreen at 0.5 — flat (0.4036, 0.4866, 0.4804) — and D50 is XYZ (0.0833, 0.1041,
/// 0.0848), linear sRGB (0.0510, 0.1209, 0.1014), and (64, 98, 90).
#[test]
fn a_device_gray_page_group_is_separated() {
    assert_near(
        pressed_page(page_in(GREY_GROUP, &[], "/LG cs 0.5 scn 0 0 40 40 re f")),
        [134, 198, 183],
        "LogoGreen at 0.5 on a /DeviceGray page",
    );
    assert_near(
        pressed_page(page_in(
            GREY_GROUP,
            &[],
            "0.5 g 0 0 40 40 re f /OP gs /LG cs 0.5 scn 0 0 40 40 re f",
        )),
        [64, 98, 90],
        "grey times LogoGreen",
    );
}

/// A spot mark inside a `/DeviceRGB` group nested in a `/DeviceCMYK` page: §11.7.3's first bullet
/// passes it through the group — "the spot colour passes directly through the group hierarchy to
/// the device, with no colour conversions performed" — while the group's process components are
/// composited in RGB and converted into the page's CMYK at the group's end. The group's one mark
/// is LogoGreen at full tint and half opacity, so something composites in it and it is drawn in
/// its own space rather than folded into the page's: its process components are white at 0.5 and
/// its spot plane full LogoGreen at 0.5. It is composited onto process yellow under Multiply,
/// which is separable and white-preserving and so applies to the spot colourant too (§11.7.4.2):
/// white times yellow leaves the yellow, and over no ink the spot plane is `½ × 1 + ½ × (1 × 0)`,
/// a tint of 0.5. Step c) multiplies the two separations — process yellow, flat (0.8068, 0.8590,
/// 0.1214), and LogoGreen at 0.5, (0.4036, 0.4866, 0.4804) — to XYZ (0.3140, 0.4180, 0.0481),
/// linear sRGB (0.2846, 0.4952, −0.0055), and (145, 187, 0).
#[test]
fn a_spot_mark_inside_an_rgb_group_passes_through_to_a_cmyk_page() {
    let form = "/HALF gs /LG cs 1 scn 0 0 40 40 re f";
    let extra = vec![format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 40 40] \
         /Group << /S /Transparency /CS /DeviceRGB /I true >> \
         /Resources << /ColorSpace << /LG {LOGO_GREEN} >> \
         /ExtGState << /HALF << /ca 0.5 >> >> >> /Length {} >>\nstream\n{form}\n\
         endstream",
        form.len() + 1
    )];
    let bytes = page_in(
        "/Group << /S /Transparency /CS /DeviceCMYK >>",
        &extra,
        "0 0 1 0 k 0 0 40 40 re f /MUL gs /Fm Do",
    );
    let interpretation = interpreted(bytes.clone(), true);
    assert!(
        interpretation
            .display_list
            .commands()
            .iter()
            .any(|command| matches!(
                command,
                pdf_render::Command::Group {
                    blending: Some(_),
                    ..
                }
            )),
        "the RGB group is drawn in its own space, with a conversion into the page's"
    );
    assert_near(
        pressed_page(bytes),
        [145, 187, 0],
        "yellow times the RGB group's LogoGreen at half",
    );
}
