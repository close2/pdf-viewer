//! Independent `Separation`/`DeviceN` and calibrated/Lab paint controls for §§8.6.6.4–5 and
//! 11.7.5.3: a colour selected under one state and painted under another converts in the
//! state at the paint.
//!
//! Expected pixels come from the fixture profile's encoded knots, §8.6.5.3's and §8.6.5.4's
//! own formulas on a D50 white, the published D50-adapted XYZ-to-sRGB matrix and §10.4.2.2's
//! grey weights, independently of the interpreter. Cold selections inside the same group are a
//! supplementary consistency check, not the oracle.
use super::{
    BLUE, NEUTRAL, RED, content_stream, decoded_xyz_to_srgb, expected, fixture_with_supplements,
    near, patch, pixel, render_bytes,
};

/// The page's colour spaces. `/S` and `/SB` are one identity tint transform into profiles A
/// and B; `/N` is a `DeviceN` whose type 4 transform keeps its first tint; `/SG` and `/NG`
/// paint pure green through `DeviceRGB`; `/C`, `/L` and `/CG` are calibrated on D50.
const SPACES: &str = "/S [/Separation /Spot [/ICCBased 5 0 R] << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>] \
/SB [/Separation /Spot [/ICCBased 6 0 R] << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>] \
/N [/DeviceN [/Spot /Other] [/ICCBased 5 0 R] 17 0 R] \
/SG [/Separation /Green /DeviceRGB << /FunctionType 2 /Domain [0 1] /C0 [1 1 1] /C1 [0 1 0] /N 1 >>] \
/NG [/DeviceN [/Spot /Other] /DeviceRGB 18 0 R] \
/C [/CalRGB << /WhitePoint [0.9642 1 0.8249] /Matrix [1 0 0 0 1 0 0 0 1] >>] \
/L [/Lab << /WhitePoint [0.9642 1 0.8249] /Range [-100 100 -100 100] >>] \
/CG [/CalGray << /WhitePoint [0.9642 1 0.8249] /Gamma 2.2 >>] ";

const XOBJECTS: &str = "/GInherit 19 0 R /GCold 20 0 R /CInherit 21 0 R /CCold 22 0 R ";

/// Objects 17–22: the two `DeviceN` transforms, then an opaque `DeviceGray` group and a
/// `CalGray` group of gamma 1, each once inheriting the page's colour and once selecting
/// `cold` inside itself.
fn render(content: &str, cold: &str) -> Vec<u8> {
    let gray = "/Type /XObject /Subtype /Form /BBox [0 0 80 80] /Group << /S /Transparency /I true /CS /DeviceGray >>";
    let calibrated = "/Type /XObject /Subtype /Form /BBox [0 0 80 80] /Group << /S /Transparency /I true /CS [/CalGray << /WhitePoint [0.9642 1 0.8249] >>] >>";
    let resources = format!("/Resources << /ColorSpace << {SPACES}>> >>");
    let extra = [
        content_stream("/FunctionType 4 /Domain [0 1 0 1] /Range [0 1]", "{ pop }"),
        content_stream(
            "/FunctionType 4 /Domain [0 1 0 1] /Range [0 1 0 1 0 1]",
            "{ pop 0 exch 0 }",
        ),
        content_stream(&format!("{gray} {resources}"), "0 0 60 80 re f"),
        content_stream(
            &format!("{gray} {resources}"),
            &format!("{cold} 0 0 60 80 re f"),
        ),
        content_stream(&format!("{calibrated} {resources}"), "0 0 60 80 re f"),
        content_stream(
            &format!("{calibrated} {resources}"),
            &format!("{cold} 0 0 60 80 re f"),
        ),
    ];
    render_bytes(fixture_with_supplements(content, SPACES, XOBJECTS, &extra))
}

/// §8.6.5.4's second stage on the fixture's D50 white, from an `L*a*b*` triple inside `/Range`.
fn lab_xyz([lightness, a, b]: [f64; 3]) -> [f64; 3] {
    let g = |x: f64| {
        if x >= 6.0 / 29.0 {
            x * x * x
        } else {
            108.0 / 841.0 * (x - 4.0 / 29.0)
        }
    };
    let m = (lightness + 16.0) / 116.0;
    [0.9642 * g(m + a / 500.0), g(m), 0.8249 * g(m - b / 200.0)]
}

/// §10.4.2.2's grey of the sRGB a colour converts to, which is what a `DeviceGray` group holds.
fn grey_of(srgb: [f64; 3]) -> [u8; 3] {
    let grey = (0.3 * srgb[0] + 0.59 * srgb[1] + 0.11 * srgb[2]).round() as u8;
    [grey; 3]
}

const LAB: [f64; 3] = [50.0, 60.0, -20.0];

#[test]
fn separation_and_devicen_tints_convert_under_the_intent_at_the_paint() {
    for selection in ["/S cs 0.5 scn", "/N cs 0.5 0.2 scn"] {
        for intent in ["/Perceptual ri", "/Red gs"] {
            let actual = render(
                &format!("{selection} {intent} {} {}", patch(0), patch(100)),
                "",
            );
            near(pixel(&actual, 30, 50), expected(RED));
            near(pixel(&actual, 130, 50), expected(RED));
            let cold = render(&format!("{intent} {selection} {}", patch(0)), "");
            near(pixel(&cold, 30, 50), expected(RED));
        }
    }
}

#[test]
fn an_initial_tint_follows_late_black_point_compensation() {
    for selection in ["/S cs", "/N cs"] {
        let actual = render(
            &format!(
                "{selection} {} /On gs {} /Off gs {}",
                patch(0),
                patch(100),
                patch(200)
            ),
            "",
        );
        let dark = pixel(&actual, 30, 50);
        assert!(
            dark.into_iter().all(|v| v > 60),
            "uncompensated full tint must remain above black: {dark:?}"
        );
        near(pixel(&actual, 130, 50), [0, 0, 0]);
        near(pixel(&actual, 230, 50), dark);
    }
}

#[test]
fn saves_restore_the_tint_source_beside_the_current_intent() {
    let actual = render(
        &format!(
            "/S cs 0.5 scn {} q /Perceptual ri {} q /SB cs 0.5 scn {} Q {} Q {}",
            patch(0),
            patch(100),
            patch(200),
            patch(300),
            patch(400)
        ),
        "",
    );
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
fn stroke_stencil_and_alpha_paint_the_tint_under_the_current_intent() {
    let actual = render(
        &format!(
            "/S CS 0.5 SCN /N cs 0.5 1 scn /Perceptual ri 20 w 30 10 m 30 90 l S q 70 0 0 80 100 10 cm /Mask Do Q q /Half gs {} Q",
            patch(200)
        ),
        "",
    );
    let red = expected(RED);
    near(pixel(&actual, 30, 50), red);
    near(pixel(&actual, 130, 50), red);
    near(
        pixel(&actual, 230, 50),
        red.map(|v| f64::midpoint(f64::from(v), 255.0).round() as u8),
    );
}

#[test]
fn a_tint_or_cie_colour_inherited_into_a_gray_group_converts_there() {
    let green = [0.0, 255.0, 0.0];
    for (selection, srgb) in [
        ("/SG cs 1 scn", green),
        ("/NG cs 1 0.3 scn", green),
        ("/C cs 0.45 0.25 0.05 scn", decoded_xyz_to_srgb(RED)),
        ("/L cs 50 60 -20 scn", decoded_xyz_to_srgb(lab_xyz(LAB))),
    ] {
        let on_page = srgb.map(|v| v.round() as u8);
        let content = format!(
            "{selection} {} q 1 0 0 1 100 10 cm /GInherit Do Q {}",
            patch(0),
            patch(200)
        );
        let actual = render(&content, selection);
        near(pixel(&actual, 30, 50), on_page);
        near(pixel(&actual, 230, 50), on_page);
        let cold = render("q 1 0 0 1 100 10 cm /GCold Do Q", selection);
        for x in [110, 140] {
            near(pixel(&actual, x, 50), grey_of(srgb));
            near(pixel(&cold, x, 50), grey_of(srgb));
        }
    }
}

#[test]
fn a_calgray_colour_inherited_into_a_calibrated_group_keeps_its_colorimetry() {
    // §8.6.5.2: A = 0.6 decodes to Y = 0.6^2.2 on the D50 white, whose sRGB is neutral.
    let luminance = 0.6f64.powf(2.2);
    let srgb = decoded_xyz_to_srgb([0.9642 * luminance, luminance, 0.8249 * luminance]);
    let wanted = srgb.map(|v| v.round() as u8);
    let selection = "/CG cs 0.6 scn";
    let actual = render(
        &format!(
            "{selection} {} q 1 0 0 1 100 10 cm /CInherit Do Q",
            patch(0)
        ),
        selection,
    );
    let cold = render("q 1 0 0 1 100 10 cm /CCold Do Q", selection);
    near(pixel(&actual, 30, 50), wanted);
    near(pixel(&actual, 140, 50), wanted);
    near(pixel(&cold, 140, 50), wanted);
}

#[test]
fn calibrated_and_lab_colours_on_the_device_read_no_intent() {
    for (selection, srgb) in [
        ("/C cs 0.45 0.25 0.05 scn", decoded_xyz_to_srgb(RED)),
        ("/L cs 50 60 -20 scn", decoded_xyz_to_srgb(lab_xyz(LAB))),
    ] {
        let actual = render(
            &format!("{selection} {} /Saturation ri {}", patch(0), patch(100)),
            "",
        );
        let wanted = srgb.map(|v| v.round() as u8);
        near(pixel(&actual, 30, 50), wanted);
        near(pixel(&actual, 130, 50), wanted);
    }
}
