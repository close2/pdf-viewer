//! Independent Indexed-to-ICC source controls for §§8.6.6.3 and 11.7.5.3.
//! Expected pixels use encoded fixture knots, lookup scaling and the published sRGB matrix.
use super::{
    content_stream, decoded_xyz_to_srgb, fixture_with_default, fixture_with_supplements, near,
    patch, pixel, render_bytes,
};

// Literal u1Fixed15 knots emitted by the independently generated profile fixture.
pub(super) const RED: [u32; 3] = [14745, 8192, 1638];
const BLUE: [u32; 3] = [3932, 8192, 24576];
pub(super) const NEUTRAL: [u32; 3] = [8192, 8192, 8192];

const RESOURCES: &str = "/IA [/Indexed [/ICCBased 5 0 R] 2 <4080C0>] /IB [/Indexed [/ICCBased 6 0 R] 2 <4080C0>] /IT [/Indexed [/ICCBased 5 0 R] 2 <C08040>] /Initial [/Indexed [/ICCBased 5 0 R] 0 <80>]";

fn render(content: &str) -> Vec<u8> {
    render_bytes(fixture_with_default(content, RESOURCES))
}

// Identity input/output tables preserve byte/255. Quantize neither XYZ nor
// straight RGB again before the final buffer boundary. At 128 the second segment
// fraction is 1/255, not zero; 64/192 give stronger lookup discrimination.
fn expected_entry_unrounded(mid: [u32; 3], byte: u8) -> [f64; 3] {
    let white = [31594, 32768, 27030];
    let last = [3159, 3276, 2703];
    let (low, high, numerator) = if byte < 128 {
        (white, mid, 2 * u16::from(byte))
    } else {
        (mid, last, 2 * u16::from(byte) - 255)
    };
    let xyz = std::array::from_fn(|i| {
        (f64::from(255 - numerator) * f64::from(low[i]) + f64::from(numerator) * f64::from(high[i]))
            / (255.0 * 32768.0)
    });
    decoded_xyz_to_srgb(xyz)
}
pub(super) fn expected_entry(mid: [u32; 3], byte: u8) -> [u8; 3] {
    expected_entry_unrounded(mid, byte).map(|v| v.round() as u8)
}

#[test]
fn indexed_late_intent_and_initial_index_decode_the_lookup_entry() {
    for selection in ["/Initial cs", "/IA cs 1 scn"] {
        for intent in ["/Perceptual ri", "/Red gs"] {
            let actual = render(&format!("{selection} {intent} {}", patch(0)));
            near(pixel(&actual, 30, 50), expected_entry(RED, 128));
            let cold = render(&format!("{intent} {selection} {}", patch(0)));
            near(pixel(&cold, 30, 50), expected_entry(RED, 128));
        }
    }
}

#[test]
fn indexed_restore_keeps_entry_table_and_profile_separate_from_context() {
    let actual = render(&format!(
        "/IA cs 0 scn {} q /Perceptual ri {} q 2 scn {} /IB cs 2 scn {} Q {} Q {}",
        patch(0),
        patch(100),
        patch(200),
        patch(300),
        patch(400),
        patch(500)
    ));
    for (x, mid, byte) in [
        (30, NEUTRAL, 64),
        (130, RED, 64),
        (230, RED, 192),
        (330, BLUE, 192),
        (430, RED, 64),
        (530, NEUTRAL, 64),
    ] {
        near(pixel(&actual, x, 50), expected_entry(mid, byte));
    }
    let tables = render(&format!(
        "/Perceptual ri /IA cs 0 scn {} /IT cs 0 scn {} /IA cs 2 scn {}",
        patch(0),
        patch(100),
        patch(200)
    ));
    for (x, byte) in [(30, 64), (130, 192), (230, 192)] {
        near(pixel(&tables, x, 50), expected_entry(RED, byte));
    }
}

#[test]
fn indexed_fill_stroke_and_shared_stencil_use_current_intent() {
    let actual = render(
        "/IA cs 0 scn /IB CS 2 SCN /Perceptual ri 0 10 70 80 re f 20 w 100 10 m 100 90 l S q 70 0 0 80 200 10 cm /Mask Do Q /Saturation ri q 70 0 0 80 300 10 cm /Mask Do Q",
    );
    for (x, mid, byte) in [
        (30, RED, 64),
        (100, BLUE, 192),
        (230, RED, 64),
        (330, BLUE, 64),
    ] {
        near(pixel(&actual, x, 50), expected_entry(mid, byte));
    }
}

#[test]
fn indexed_fractional_rounding_and_clamping_are_lookup_semantics() {
    let actual = render(&format!(
        "/IA cs -5 scn /Perceptual ri {} 0.49 scn {} 0.5 scn {} 1.5 scn {} 99 scn {}",
        patch(0),
        patch(100),
        patch(200),
        patch(300),
        patch(400)
    ));
    for (x, byte) in [(30, 64), (130, 64), (230, 128), (330, 192), (430, 192)] {
        near(pixel(&actual, x, 50), expected_entry(RED, byte));
    }
}

#[test]
fn indexed_profile_and_component_keys_have_independent_restore_controls() {
    for (outer, inner, bytes, mids) in [
        (
            "/IA cs 2 scn",
            "/IB cs 2 scn",
            [192, 192, 192],
            [RED, BLUE, RED],
        ),
        (
            "/IA cs 0 scn",
            "/IA cs 2 scn",
            [64, 192, 64],
            [RED, RED, RED],
        ),
        (
            "/IA cs 0 scn",
            "/IT cs 0 scn",
            [64, 192, 64],
            [RED, RED, RED],
        ),
    ] {
        let actual = render(&format!(
            "/Perceptual ri {outer} {} q {inner} {} Q {}",
            patch(0),
            patch(100),
            patch(200)
        ));
        for (index, x) in [30, 130, 230].into_iter().enumerate() {
            near(
                pixel(&actual, x, 50),
                expected_entry(mids[index], bytes[index]),
            );
        }
    }
}

#[test]
fn indexed_text_fill_stroke_and_both_have_independent_colour_witnesses() {
    for mode in [0, 1, 2] {
        let mark = format!("4 w BT /F 80 Tf {mode} Tr 1 0 0 1 10 10 Tm (H) Tj ET");
        let actual = render(&format!("/IA cs 0 scn /IA CS 0 SCN /Perceptual ri {mark}"));
        let cold = render(&format!("/Perceptual ri /IA cs 0 scn /IA CS 0 SCN {mark}"));
        let first_difference = actual
            .iter()
            .zip(&cold)
            .enumerate()
            .find_map(|(index, (a, b))| (a != b).then_some((index, *a, *b)));
        let wanted = expected_entry(RED, 64);
        assert!(
            (10..90).any(|y| (10..80).any(|x| pixel(&actual, x, y)
                .into_iter()
                .zip(wanted)
                .all(|(a, b)| a.abs_diff(b) <= 1))),
            "missing independently coloured glyph interior in mode {mode}"
        );
        assert!(
            first_difference.is_none(),
            "text mode {mode}: {first_difference:?}"
        );
    }
}

#[test]
fn indexed_endpoint_bpc_and_absolute_are_current_paint_context() {
    let actual = render_bytes(fixture_with_default(
        &format!(
            "/End cs 0 scn {} /On gs {} /AbsoluteColorimetric ri {} /RelativeColorimetric ri {}",
            patch(0),
            patch(100),
            patch(200),
            patch(300)
        ),
        &format!("{RESOURCES} /End [/Indexed [/ICCBased 5 0 R] 0 <FF>]"),
    ));
    for (x, wanted) in [
        (30, expected_entry(NEUTRAL, 255)),
        (130, [0, 0, 0]),
        (230, expected_entry(NEUTRAL, 255)),
        (330, [0, 0, 0]),
    ] {
        near(pixel(&actual, x, 50), wanted);
    }
}

// The shared fixture occupies objects 1–16; these Form resources start at 17.
fn render_forms(content: &str) -> Vec<u8> {
    let shadow = "/Type /XObject /Subtype /Form /BBox [0 0 80 80] /Resources << /ColorSpace << /IA [/Indexed [/ICCBased 6 0 R] 2 <4080C0>] >> >>";
    let objects = [
        content_stream(shadow, "0 0 30 80 re f"),
        content_stream(shadow, "0 0 30 80 re f /IA cs 0 scn 40 0 30 80 re f"),
        content_stream(
            "/Type /XObject /Subtype /Form /BBox [0 0 80 80] /Group << /S /Transparency /I true /CS /DeviceGray >> /Resources << /ColorSpace << /IA [/Indexed [/ICCBased 5 0 R] 2 <4080C0>] >> /ExtGState << /Half << /ca 0.5 >> >> >>",
            "/IA cs 1 scn /Half gs 0 0 60 80 re f 20 0 60 80 re f",
        ),
    ];
    render_bytes(fixture_with_supplements(
        content,
        RESOURCES,
        "/IndexedInherit 17 0 R /IndexedChange 18 0 R /IndexedGrayCold 19 0 R",
        &objects,
    ))
}

#[test]
fn indexed_forms_shadow_names_without_rebinding_inherited_sources() {
    let actual = render_forms(&format!(
        "/IA cs 0 scn /Perceptual ri q 1 0 0 1 0 10 cm /IndexedInherit Do Q q 1 0 0 1 100 10 cm /IndexedChange Do Q {}",
        patch(200)
    ));
    for (x, mid) in [(15, RED), (115, RED), (155, BLUE), (230, RED)] {
        near(pixel(&actual, x, 50), expected_entry(mid, 64));
    }
}

fn half_over_white(source: [f64; 3], twice: bool) -> [u8; 3] {
    let source = source.map(|v| (v * 0.5).round() as u16);
    let over = |s: u16, d: u16| s + ((d * 127 + 255) >> 8);
    let alpha = if twice { over(128, 128) } else { 128 };
    source.map(|v| ((if twice { over(v, v) } else { v }) + 255 - alpha) as u8)
}

#[test]
fn indexed_alpha_and_rgb_group_return_keep_pre_alpha_source() {
    let actual = render(&format!(
        "/IA cs 1 scn /Perceptual ri {} q /Half gs {} Q q 1 0 0 1 200 10 cm /Group Do Q {}",
        patch(0),
        patch(100),
        patch(300)
    ));
    near(pixel(&actual, 30, 50), expected_entry(RED, 128));
    near(
        pixel(&actual, 130, 50),
        half_over_white(expected_entry_unrounded(RED, 128), false),
    );
    near(
        pixel(&actual, 230, 50),
        half_over_white(expected_entry_unrounded(RED, 128), true),
    );
    near(pixel(&actual, 330, 50), expected_entry(RED, 128));
}

#[test]
fn indexed_gray_target_and_parent_return_agree_with_cold_group_selection() {
    let actual = render_forms(&format!(
        "/Perceptual ri /IA cs 1 scn {} q 1 0 0 1 100 10 cm /Gray Do Q {}",
        patch(0),
        patch(200)
    ));
    let cold = render_forms("/Perceptual ri q 1 0 0 1 100 10 cm /IndexedGrayCold Do Q");
    near(pixel(&actual, 30, 50), expected_entry(RED, 128));
    near(pixel(&actual, 230, 50), expected_entry(RED, 128));
    for x in [110, 130, 170] {
        let gray = pixel(&actual, x, 50);
        assert!(
            gray[0].abs_diff(gray[1]) <= 1 && gray[1].abs_diff(gray[2]) <= 1,
            "group must be neutral: {gray:?}"
        );
        assert!(gray[0] < 250, "group must actually paint");
        near(gray, pixel(&cold, x, 50));
    }
}
