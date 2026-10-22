//! Painting-time ICC state controls for ISO 32000-2 §§8.6.5.8–9 and 11.7.5.3.
//!
//! Generated intent tables have different exact middle knots. Expected colours below
//! come from those knots and the published D50-adapted XYZ-to-sRGB matrix, independently
//! of the interpreter's conversion and memoization. Cold comparisons supplement that oracle.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "bounded generated fixtures and explicit ICC fixed-point encoding"
)]
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use std::fmt::Write as _;

fn mid_table(mid: [f32; 3]) -> Vec<u8> {
    let white = [0.964_2f32, 1.0, 0.824_9];
    let fixed = |value: f32| ((value * 32768.0) as u16).to_be_bytes();

    let mut tag = Vec::new();
    tag.extend_from_slice(b"mft2");
    tag.extend_from_slice(&[0; 4]);
    tag.extend_from_slice(&[1, 3, 3, 0]); // one in, three out, three grid points
    for value in [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0] {
        tag.extend_from_slice(&((value * 65536.0) as i32).to_be_bytes());
    }
    tag.extend_from_slice(&2u16.to_be_bytes()); // input table entries
    tag.extend_from_slice(&2u16.to_be_bytes()); // output table entries
    for value in [0u16, 0xFFFF] {
        tag.extend_from_slice(&value.to_be_bytes());
    }
    for point in [
        white,
        mid,
        [white[0] / 10.0, white[1] / 10.0, white[2] / 10.0],
    ] {
        for value in point {
            tag.extend_from_slice(&fixed(value));
        }
    }
    for _ in 0..3 {
        for value in [0u16, 0xFFFF] {
            tag.extend_from_slice(&value.to_be_bytes());
        }
    }
    tag
}

/// An ICC profile that answers a mid-tone in three different colours, one per rendering intent.
///
/// ISO 15076-1:2010 gives `A2B0`, `A2B1` and `A2B2` the device-to-connection transform of one
/// rendering intent each (its tag listing, clause 9.2), and §8.6.5.8 says the names PDF uses
/// "have been chosen to correspond to those defined by the International Color Consortium
/// (ICC)". So this profile is three answers to one question, and which one comes back says which
/// intent was in force:
///
/// - `A2B0`, perceptual: a red mid-tone.
/// - `A2B1`, colorimetric: a neutral mid-tone — the one Table 51 makes the initial value.
/// - `A2B2`, saturation: a blue mid-tone.
///
/// No real profile is built this way; that is the point of a fixture (trap 8). A profile whose
/// tables agreed to within a few levels, as real ones tend to, could pass these tests with the
/// selection unimplemented.
fn three_table_profile() -> Vec<u8> {
    let tables: [(&[u8; 4], [f32; 3]); 3] = [
        (b"A2B0", [0.45, 0.25, 0.05]),
        (b"A2B1", [0.25, 0.25, 0.25]),
        (b"A2B2", [0.12, 0.25, 0.75]),
    ];

    let mut header = vec![0u8; 128];
    header[8] = 2;
    header[16..20].copy_from_slice(b"GRAY");
    header[20..24].copy_from_slice(b"XYZ ");
    header[36..40].copy_from_slice(b"acsp");

    let mut out = header;
    out.extend_from_slice(&(tables.len() as u32).to_be_bytes());
    let mut offset = 128 + 4 + 12 * tables.len();
    let mut data = Vec::new();
    for (name, mid) in tables {
        let tag = mid_table(mid);
        out.extend_from_slice(name);
        out.extend_from_slice(&(offset as u32).to_be_bytes());
        out.extend_from_slice(&(tag.len() as u32).to_be_bytes());
        offset += tag.len();
        data.extend_from_slice(&tag);
    }
    out.extend_from_slice(&data);
    out
}

fn stream(dictionary: &str, data: &[u8]) -> String {
    let mut hex = String::new();
    for byte in data {
        write!(hex, "{byte:02X}").expect("write string");
    }
    format!(
        "<< {dictionary} /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}>\nendstream",
        hex.len() + 1
    )
}
fn content_stream(dictionary: &str, content: &str) -> String {
    format!(
        "<< {dictionary} /Length {} >>\nstream\n{content}\nendstream",
        content.len() + 1
    )
}
/// Profiles A and B share a component count and values but have different tables.
/// The same indirect stencil and font are reused throughout every page.
fn fixture(content: &str) -> Vec<u8> {
    fixture_with_default(content, "")
}
fn fixture_with_default(content: &str, default_gray: &str) -> Vec<u8> {
    fixture_with_supplements(content, default_gray, "", &[])
}

fn fixture_with_supplements(
    content: &str,
    default_gray: &str,
    extra_xobjects: &str,
    extra_objects: &[String],
) -> Vec<u8> {
    let a = three_table_profile();
    let mut b = a.clone();
    // Exchange A2B0/A2B2 tag signatures, leaving the encoded tables untouched.
    b[132..136].copy_from_slice(b"A2B2");
    b[156..160].copy_from_slice(b"A2B0");
    let resources = "/ColorSpace << /A [/ICCBased 5 0 R] /B [/ICCBased 6 0 R] /P [/Pattern [/ICCBased 5 0 R]] >> /XObject << /Mask 7 0 R /Inherit 11 0 R /Change 12 0 R /Group 13 0 R /Gray 15 0 R /GrayCold 16 0 R >> /Font << /F 8 0 R >> /Pattern << /Tile 14 0 R >> /ExtGState << /Off << /UseBlackPtComp /OFF >> /On << /UseBlackPtComp /ON >> /Red << /RI /Perceptual >> /Half << /ca 0.5 /CA 0.5 >> >>";
    let resources = resources.replacen(
        "/ColorSpace <<",
        &format!("/ColorSpace << {default_gray}"),
        1,
    );
    let resources = if extra_xobjects.is_empty() {
        resources
    } else {
        resources.replacen("/XObject <<", &format!("/XObject << {extra_xobjects}"), 1)
    };
    let font = include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 100] /Resources << {resources} >> /Contents 4 0 R >>"),
        content_stream("", &format!("/Off gs {content}")),
        stream("/N 1", &a), stream("/N 1", &b),
        stream("/Type /XObject /Subtype /Image /Width 1 /Height 1 /ImageMask true /BitsPerComponent 1", &[0]),
        "<< /Type /Font /Subtype /TrueType /BaseFont /LiberationSans /FirstChar 72 /LastChar 72 /Widths [722] /Encoding /WinAnsiEncoding /FontDescriptor 9 0 R >>".to_owned(),
        "<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 /FontBBox [-203 -303 1050 910] /ItalicAngle 0 /Ascent 905 /Descent -212 /CapHeight 729 /StemV 80 /FontFile2 10 0 R >>".to_owned(),
        stream(&format!("/Length1 {}", font.len()), font),
        content_stream("/Type /XObject /Subtype /Form /BBox [0 0 80 80] /Resources << /ColorSpace << /A [/ICCBased 6 0 R] >> >>", "0 0 30 80 re f"),
        content_stream("/Type /XObject /Subtype /Form /BBox [0 0 80 80] /Resources << /ColorSpace << /A [/ICCBased 6 0 R] >> >>", "0 0 30 80 re f /A cs 0.5 scn 40 0 30 80 re f"),
        content_stream("/Type /XObject /Subtype /Form /BBox [0 0 80 80] /Group << /S /Transparency /I true /CS /DeviceRGB >> /Resources << /ExtGState << /Half << /ca 0.5 >> >> >>", "/Half gs 0 0 60 80 re f 20 0 60 80 re f"),
        content_stream("/Type /Pattern /PatternType 1 /PaintType 2 /TilingType 1 /BBox [0 0 10 10] /XStep 10 /YStep 10 /Resources << >>", "0 0 10 10 re f"),
        content_stream("/Type /XObject /Subtype /Form /BBox [0 0 80 80] /Group << /S /Transparency /I true /CS /DeviceGray >> /Resources << /ExtGState << /Half << /ca 0.5 >> >> >>", "/Half gs 0 0 60 80 re f 20 0 60 80 re f"),
        content_stream("/Type /XObject /Subtype /Form /BBox [0 0 80 80] /Group << /S /Transparency /I true /CS /DeviceGray >> /Resources << /ColorSpace << /A [/ICCBased 5 0 R] >> /ExtGState << /Half << /ca 0.5 >> >> >>", "/A cs 0.5 scn /Half gs 0 0 60 80 re f 20 0 60 80 re f"),
    ];
    objects.extend_from_slice(extra_objects);
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        writeln!(out, "{} 0 obj\n{object}\nendobj", index + 1).expect("write object");
    }
    let xref = out.len();
    writeln!(out, "xref\n0 {}\n0000000000 65535 f ", objects.len() + 1).expect("xref");
    for offset in offsets {
        writeln!(out, "{offset:010} 00000 n ").expect("offset");
    }
    write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    )
    .expect("trailer");
    out.into_bytes()
}
fn render(content: &str) -> Vec<u8> {
    render_bytes(fixture(content))
}
fn render_bytes(bytes: Vec<u8>) -> Vec<u8> {
    let doc = Document::open(bytes).expect("valid fixture");
    let page = pdf_model::Pages::new(&doc).get(0).expect("page");
    let interpreted = pdf_model::interpret(&doc, &page);
    assert!(
        interpreted.is_complete(),
        "unsupported: {:?}",
        interpreted.unsupported
    );
    let target = TargetSpec::for_page(&interpreted.display_list, 1.0, 1 << 24).expect("target");
    CpuRasterizer::new()
        .rasterize(&interpreted.display_list, target)
        .expect("raster")
        .data
}
fn pixel(raster: &[u8], x: usize, y: usize) -> [u8; 3] {
    let at = (y * 600 + x) * 4;
    [raster[at], raster[at + 1], raster[at + 2]]
}
/// IEC sRGB transfer function on the independently encoded PCS midpoint.
fn expected(mid: [f64; 3]) -> [u8; 3] {
    expected_unrounded(mid).map(|v| v.round() as u8)
}
fn expected_unrounded(mid: [f64; 3]) -> [f64; 3] {
    let xyz = mid.map(|v| (v * 32768.0).floor() / 32768.0);
    decoded_xyz_to_srgb(xyz)
}

/// Pure matrix/transfer stage; input XYZ is already decoded, with no further quantization.
fn decoded_xyz_to_srgb(xyz: [f64; 3]) -> [f64; 3] {
    let matrix = [
        [3.134_136, -1.617_036, -0.490_662],
        [-0.978_755, 1.916_142, 0.033_454],
        [0.071_950, -0.228_988, 1.405_386],
    ];
    matrix.map(|row| {
        let linear = (row[0] * xyz[0] + row[1] * xyz[1] + row[2] * xyz[2]).clamp(0.0, 1.0);
        let encoded = if linear <= 0.003_130_8 {
            12.92 * linear
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        };
        255.0 * encoded
    })
}
/// Two half-alpha paints into an eight-bit isolated group, then over white.
/// Quantize the independently calculated *premultiplied* source at each buffer boundary.
/// tiny-skia 0.12's lowp source-over divides products as (product + 255) >> 8.
/// Rounding straight RGB before ideal floating composition misses these storage boundaries.
fn expected_half_overlap(mid: [f64; 3]) -> [u8; 3] {
    let source = expected_unrounded(mid).map(|v| (v * 0.5).round() as u16);
    let over = |source: u16, destination: u16| source + ((destination * 127 + 255) >> 8);
    let group_alpha = over(128, 128);
    source.map(|v| (over(v, v) + 255 - group_alpha) as u8)
}
const RED: [f64; 3] = [0.45, 0.25, 0.05];
const BLUE: [f64; 3] = [0.12, 0.25, 0.75];
const NEUTRAL: [f64; 3] = [0.25, 0.25, 0.25];
fn near(actual: [u8; 3], wanted: [u8; 3]) {
    assert!(
        actual
            .into_iter()
            .zip(wanted)
            .all(|(a, b)| a.abs_diff(b) <= 1),
        "actual {actual:?}, expected {wanted:?}"
    );
}
fn patch(x: usize) -> String {
    format!("{x} 10 70 80 re f ")
}

#[test]
fn late_ri_and_extgstate_intent_change_the_selected_source() {
    for intent in ["/Perceptual ri", "/Red gs"] {
        let actual = render(&format!(
            "/A cs 0.5 scn {intent} {} {}",
            patch(0),
            patch(100)
        ));
        let cold = render(&format!("{intent} /A cs 0.5 scn {}", patch(0)));
        near(pixel(&cold, 30, 50), expected(RED));
        near(pixel(&actual, 30, 50), expected(RED));
        near(pixel(&actual, 130, 50), expected(RED));
    }
}
#[test]
fn nested_saves_restore_profile_and_current_intent() {
    let actual = render(&format!(
        "/A cs 0.5 scn {} q /Perceptual ri {} q /B cs 0.5 scn {} Q {} Q {}",
        patch(0),
        patch(100),
        patch(200),
        patch(300),
        patch(400)
    ));
    for (x, mid) in [
        (30, NEUTRAL),
        (130, RED),
        (230, BLUE),
        (330, RED),
        (430, NEUTRAL),
    ] {
        near(pixel(&actual, x, 50), expected(mid));
    }
}
#[test]
fn fill_and_stroke_keep_independent_sources_at_paint_time() {
    let actual = render(
        "/A cs 0.5 scn /B CS 0.5 SCN /Perceptual ri 0 10 70 80 re f 20 w 100 10 m 100 90 l S /A CS 0.5 SCN /Saturation ri 200 10 70 80 re f 300 10 m 300 90 l S",
    );
    for (x, mid) in [(30, RED), (100, BLUE), (230, BLUE), (300, BLUE)] {
        near(pixel(&actual, x, 50), expected(mid));
    }
}
#[test]
fn one_stencil_reuses_the_current_source_across_intent_and_restore() {
    let stamp = |x| format!("q 70 0 0 80 {x} 10 cm /Mask Do Q ");
    let actual = render(&format!(
        "/A cs 0.5 scn {} /Perceptual ri {} q /B cs 0.5 scn {} Q {}",
        stamp(0),
        stamp(100),
        stamp(200),
        stamp(300)
    ));
    for (x, mid) in [(30, NEUTRAL), (130, RED), (230, BLUE), (330, RED)] {
        near(pixel(&actual, x, 50), expected(mid));
    }
}
#[test]
fn embedded_text_fill_stroke_and_both_follow_the_current_intent() {
    for mode in [0, 1, 2] {
        let mark = format!("4 w BT /F 80 Tf {mode} Tr 1 0 0 1 10 10 Tm (H) Tj ET");
        let actual = render(&format!(
            "/A cs 0.5 scn /A CS 0.5 SCN /Perceptual ri {mark}"
        ));
        let cold = render(&format!(
            "/Perceptual ri /A cs 0.5 scn /A CS 0.5 SCN {mark}"
        ));
        let first_difference = actual
            .iter()
            .zip(&cold)
            .enumerate()
            .find_map(|(index, (a, b))| (a != b).then_some((index, *a, *b)));
        assert!(
            first_difference.is_none(),
            "text mode {mode}, first differing byte: {first_difference:?}"
        );
        let red = expected(RED);
        assert!(
            (10..90).any(|y| (10..80).any(|x| pixel(&actual, x, y)
                .into_iter()
                .zip(red)
                .all(|(a, b)| a.abs_diff(b) <= 1))),
            "missing solid glyph interior in mode {mode}"
        );
    }
}
#[test]
fn explicit_resets_replace_the_retained_source_and_pattern_tint() {
    let actual = render(&format!(
        "0 0 0 rg 400 10 70 80 re f /Perceptual ri /A cs 0.5 scn {} 0 1 0 rg {} /B cs 0.5 scn {} /P cs 0.5 /Tile scn {} /A cs {}",
        patch(0),
        patch(100),
        patch(200),
        patch(300),
        patch(400)
    ));
    near(pixel(&actual, 30, 50), expected(RED));
    near(pixel(&actual, 130, 50), [0, 255, 0]);
    near(pixel(&actual, 230, 50), expected(BLUE));
    near(pixel(&actual, 330, 50), expected(RED));
    near(pixel(&actual, 430, 50), [255, 255, 255]);
}
#[test]
fn form_resource_shadowing_does_not_rebind_an_inherited_source() {
    let actual = render(&format!(
        "/A cs 0.5 scn /Perceptual ri q 1 0 0 1 0 10 cm /Inherit Do Q q 1 0 0 1 100 10 cm /Change Do Q {}",
        patch(200)
    ));
    for (x, mid) in [(15, RED), (115, RED), (155, BLUE), (230, RED)] {
        near(pixel(&actual, x, 50), expected(mid));
    }
}
#[test]
fn warm_alpha_and_group_return_do_not_reuse_premultiplied_colours() {
    let actual = render(&format!(
        "/A cs 0.5 scn /Perceptual ri {} q /Half gs {} Q q 1 0 0 1 200 10 cm /Group Do Q {}",
        patch(0),
        patch(100),
        patch(300)
    ));
    let red = expected(RED);
    near(pixel(&actual, 30, 50), red);
    near(
        pixel(&actual, 130, 50),
        red.map(|v| f64::midpoint(f64::from(v), 255.0).round() as u8),
    );
    near(pixel(&actual, 230, 50), expected_half_overlap(RED));
    near(pixel(&actual, 330, 50), red);
}

#[test]
fn nested_saves_restore_components_without_reselecting_after_q() {
    let actual = render(&format!(
        "0 0 0 rg 100 10 70 80 re f /A cs 0.5 scn /Perceptual ri {} q 0 scn {} Q {}",
        patch(0),
        patch(100),
        patch(200)
    ));
    near(pixel(&actual, 30, 50), expected(RED));
    near(pixel(&actual, 130, 50), [255, 255, 255]);
    near(pixel(&actual, 230, 50), expected(RED));
}
#[test]
fn late_black_point_compensation_and_absolute_intent_use_current_parameters() {
    let actual = render(&format!(
        "/A cs 1 scn {} /On gs {} /AbsoluteColorimetric ri {} /RelativeColorimetric ri {}",
        patch(0),
        patch(100),
        patch(200),
        patch(300)
    ));
    let dark = pixel(&actual, 30, 50);
    assert!(
        dark.into_iter().all(|v| v > 60),
        "uncompensated source must remain above black: {dark:?}"
    );
    near(pixel(&actual, 130, 50), [0, 0, 0]);
    near(pixel(&actual, 230, 50), dark);
    near(pixel(&actual, 330, 50), [0, 0, 0]);
}

#[test]
fn a_different_group_target_cannot_poison_the_parent_colour() {
    let actual = render(&format!(
        "/Perceptual ri /A cs 0.5 scn {} q 1 0 0 1 100 10 cm /Gray Do Q {}",
        patch(0),
        patch(200)
    ));
    let cold = render("/Perceptual ri q 1 0 0 1 100 10 cm /GrayCold Do Q");
    near(pixel(&actual, 30, 50), expected(RED));
    near(pixel(&actual, 230, 50), expected(RED));
    for x in [110, 130, 170] {
        let gray = pixel(&actual, x, 50);
        near(gray, pixel(&cold, x, 50));
        assert!(
            gray[0].abs_diff(gray[1]) <= 1 && gray[1].abs_diff(gray[2]) <= 1,
            "group must actually be gray: {gray:?}"
        );
        assert!(gray[0] < 250, "group must draw nonwhite ink");
    }
}
#[test]
fn mismatched_operand_recovery_keeps_the_actual_source_space() {
    let actual = render(&format!(
        "/A cs 0.5 scn 1 0 0 scn /Saturation ri {} /B CS 0.5 SCN 0 1 0 SCN 20 w 100 10 m 100 90 l S",
        patch(0)
    ));
    near(pixel(&actual, 30, 50), [255, 0, 0]);
    near(pixel(&actual, 100, 50), [0, 255, 0]);
}

#[test]
fn device_gray_operators_retain_the_resolved_default_icc_source() {
    for selection in ["0.5 g", "/DeviceGray cs 0.5 scn"] {
        let content = format!(
            "{selection} /Perceptual ri {} /Saturation ri {}",
            patch(0),
            patch(100)
        );
        let actual = render_bytes(fixture_with_default(
            &content,
            "/DefaultGray [/ICCBased 5 0 R]",
        ));
        near(pixel(&actual, 30, 50), expected(RED));
        near(pixel(&actual, 130, 50), expected(BLUE));
        let device = render(&content);
        near(pixel(&device, 30, 50), [128, 128, 128]);
        near(pixel(&device, 130, 50), [128, 128, 128]);
    }
}

#[path = "icc_paint_context/indexed.rs"]
mod indexed;
