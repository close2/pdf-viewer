//! Independent `DeviceGray`/`DeviceCMYK` and `Indexed` paint controls for §§8.6.6.3 and
//! 11.7.5.3: a device colour, or an `Indexed` entry over any base, selected outside a group
//! and painted inside it converts into that group at the paint.
//!
//! Expected pixels come from the clauses' own arithmetic: §10.4.2.3's grey of a `DeviceCMYK`
//! colour, §10.4.2.2's of an sRGB one, §8.6.6.3's lookup scaling, §8.6.5.3's `CalRGB` formula
//! on a D50 white, and, on the device and out of a `DeviceCMYK` group, ADR 0263's assumed
//! press, whose process-magenta corner is the colour `0 1 0 0 k` is drawn in. A one-component
//! `ICCBased` group takes a device grey in as the component that gives it back (ADR 0790), so
//! the grey a colour has is what comes out of it. Cold selections inside the same group are a
//! supplementary consistency check, not the oracle.
use super::{
    content_stream, decoded_xyz_to_srgb, fixture_with_supplements, near, patch, pixel,
    render_bytes, stream,
};

/// The page's colour spaces: `Indexed` over `DeviceCMYK`, `DeviceGray`, `CalRGB` and a
/// `Separation` into profile A, each with entry 1 the colour a control paints.
const SPACES: &str = "/IK [/Indexed /DeviceCMYK 1 <00000000 00FF0000>] \
/IG [/Indexed /DeviceGray 1 <FF99>] \
/IC [/Indexed [/CalRGB << /WhitePoint [0.9642 1 0.8249] /Matrix [1 0 0 0 1 0 0 0 1] >>] 1 <000000 73400D>] \
/IS [/Indexed [/Separation /Spot [/ICCBased 5 0 R] << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>] 1 <0080>] \
/IF [/Indexed [/Separation /Spot [/ICCBased 5 0 R] << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>] 1 <00FF>] ";

const XOBJECTS: &str = "/GInherit 18 0 R /GCold 19 0 R /KInherit 20 0 R /KCold 21 0 R \
/IInherit 22 0 R /ICold 23 0 R /GMarks 24 0 R ";

/// `0 1 0 0 k` on the device: ADR 0263's process-magenta corner.
const MAGENTA: [u8; 3] = [236, 0, 140];

/// `0 1 0 0 k` at half alpha over `0 0 1 0 k` in a `DeviceCMYK` group: §11.3.3's basic
/// compositing formula on each component gives `0 0.5 0.5 0`, which ADR 0263's press
/// interpolates as the mean of its paper, magenta, yellow and red corners — [255, 255, 255],
/// [236, 0, 140], [255, 242, 0] and [237, 28, 36].
const MAGENTA_HALF_OVER_YELLOW: [u8; 3] = [246, 131, 108];

/// §10.4.2.3's grey of `0 1 0 0`: `1 − min(1, 0.59)`, on eight bits.
const MAGENTA_GREY: [u8; 3] = [105; 3];

/// `0.6 g`, and entry 1 of `/IG` (`0x99`).
const GREY_60: [u8; 3] = [153; 3];

/// A v2 'GRAY' profile whose one tag is a `kTRC` gamma of 1.0: a group in it has a curve
/// with an inverse, which the three-table profile (whose greys fall) does not.
fn gamma_one_gray_profile() -> Vec<u8> {
    let mut header = vec![0u8; 128];
    header[8] = 2;
    header[12..16].copy_from_slice(b"mntr");
    header[16..20].copy_from_slice(b"GRAY");
    header[20..24].copy_from_slice(b"XYZ ");
    header[36..40].copy_from_slice(b"acsp");
    let mut out = header;
    out.extend_from_slice(&1u32.to_be_bytes());
    out.extend_from_slice(b"kTRC");
    out.extend_from_slice(&144u32.to_be_bytes());
    out.extend_from_slice(&16u32.to_be_bytes());
    out.extend_from_slice(b"curv");
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&1u32.to_be_bytes());
    out.extend_from_slice(&0x0100u16.to_be_bytes());
    out.extend_from_slice(&[0; 2]);
    out
}

/// Objects 17–24: the gamma-one profile, then a `DeviceGray`, a `DeviceCMYK` and an
/// `ICCBased` group, each once inheriting the page's colour and once selecting `cold` inside
/// itself, and a `DeviceGray` group drawing a stroke, a stencil and a half-alpha fill.
///
/// The `DeviceCMYK` group paints its colour at half alpha over an opaque yellow it selects and
/// restores from: an opaque fill alone composites nothing, and the group gives its own space up
/// for its parent's (`group_runs.rs`), where the colour converted outside is the right one.
fn render(content: &str, cold: &str) -> Vec<u8> {
    let form = "/Type /XObject /Subtype /Form /BBox [0 0 80 80]";
    let resources =
        format!("/Resources << /ColorSpace << {SPACES}>> /ExtGState << /Half << /ca 0.5 >> >> >>");
    let group = |space: &str| {
        format!("{form} /Group << /S /Transparency /I true /CS {space} >> {resources}")
    };
    let mut extra = vec![stream("/N 1", &gamma_one_gray_profile())];
    for (space, before, paint) in [
        ("/DeviceGray", "", "0 0 60 80 re f"),
        (
            "/DeviceCMYK",
            "q 0 0 1 0 k 0 0 60 80 re f Q ",
            "/Half gs 0 0 60 80 re f",
        ),
        ("[/ICCBased 17 0 R]", "", "0 0 60 80 re f"),
    ] {
        extra.push(content_stream(&group(space), &format!("{before}{paint}")));
        extra.push(content_stream(
            &group(space),
            &format!("{before}{cold} {paint}"),
        ));
    }
    extra.push(content_stream(
        &format!(
            "{form} /Group << /S /Transparency /I true /CS /DeviceGray >> /Resources << /XObject << /Mask 7 0 R >> /ExtGState << /Half << /ca 0.5 >> >> >>"
        ),
        "20 w 10 0 m 10 80 l S q 20 0 0 80 25 0 cm /Mask Do Q /Half gs 50 0 25 80 re f",
    ));
    render_bytes(fixture_with_supplements(content, SPACES, XOBJECTS, &extra))
}

/// The page with `selection` painted on the device at 0 and inherited into `group` at 100,
/// and the same group selecting `selection` cold.
fn inherited_and_cold(selection: &str, group: &str, cold_group: &str) -> (Vec<u8>, Vec<u8>) {
    let actual = render(
        &format!(
            "{selection} {} q 1 0 0 1 100 10 cm /{group} Do Q {}",
            patch(0),
            patch(200)
        ),
        selection,
    );
    let cold = render(
        &format!("q 1 0 0 1 100 10 cm /{cold_group} Do Q"),
        selection,
    );
    (actual, cold)
}

/// §10.4.2.2's grey of an sRGB colour, which is what a `DeviceGray` group holds.
fn grey_of(srgb: [f64; 3]) -> [u8; 3] {
    let grey = (0.3 * srgb[0] + 0.59 * srgb[1] + 0.11 * srgb[2]).round() as u8;
    [grey; 3]
}

#[test]
fn a_cmyk_colour_inherited_into_each_group_converts_there() {
    for selection in ["0 1 0 0 k", "/IK cs 1 scn"] {
        for (group, cold_group, wanted) in [
            ("GInherit", "GCold", MAGENTA_GREY),
            ("KInherit", "KCold", MAGENTA_HALF_OVER_YELLOW),
            ("IInherit", "ICold", MAGENTA_GREY),
        ] {
            let (actual, cold) = inherited_and_cold(selection, group, cold_group);
            near(pixel(&actual, 30, 50), MAGENTA);
            near(pixel(&actual, 230, 50), MAGENTA);
            for x in [110, 140] {
                near(pixel(&actual, x, 50), wanted);
                near(pixel(&cold, x, 50), wanted);
            }
        }
    }
}

#[test]
fn a_gray_colour_inherited_into_each_group_converts_there() {
    for selection in ["0.6 g", "/IG cs 1 scn"] {
        for (group, cold_group) in [
            ("GInherit", "GCold"),
            ("KInherit", "KCold"),
            ("IInherit", "ICold"),
        ] {
            let (actual, cold) = inherited_and_cold(selection, group, cold_group);
            near(pixel(&actual, 30, 50), GREY_60);
            near(pixel(&actual, 230, 50), GREY_60);
            for x in [110, 140] {
                // A `DeviceCMYK` group takes a grey in by ADR 0263's search, so the ink it
                // mixes with the yellow is the search's; the cold selection is that ink.
                let wanted = if group == "KInherit" {
                    pixel(&cold, x, 50)
                } else {
                    GREY_60
                };
                near(pixel(&actual, x, 50), wanted);
                near(pixel(&cold, x, 50), wanted);
            }
        }
    }
}

#[test]
fn an_indexed_calrgb_entry_inherited_into_a_gray_group_converts_there() {
    // §8.6.6.3 scales entry 1's bytes by 1/255; §8.6.5.3 with a unit matrix and gammas makes
    // them XYZ.
    let srgb = decoded_xyz_to_srgb([115.0 / 255.0, 64.0 / 255.0, 13.0 / 255.0]);
    let (actual, cold) = inherited_and_cold("/IC cs 1 scn", "GInherit", "GCold");
    near(pixel(&actual, 30, 50), srgb.map(|v| v.round() as u8));
    for x in [110, 140] {
        near(pixel(&actual, x, 50), grey_of(srgb));
        near(pixel(&cold, x, 50), grey_of(srgb));
    }
}

#[test]
fn an_indexed_separation_entry_converts_under_the_intent_and_black_point_at_the_paint() {
    use super::indexed::{NEUTRAL, RED, expected_entry};
    for intent in ["/Perceptual ri", "/Red gs"] {
        let actual = render(
            &format!("/IS cs 1 scn {} {intent} {}", patch(0), patch(100)),
            "",
        );
        near(pixel(&actual, 30, 50), expected_entry(NEUTRAL, 128));
        near(pixel(&actual, 130, 50), expected_entry(RED, 128));
    }
    let actual = render(
        &format!(
            "/IF cs 1 scn {} /On gs {} /Off gs {}",
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

#[test]
fn saves_restore_the_device_source_into_the_group_painted_after_them() {
    // §10.4.2.3's grey of `1 0 0 0`: `1 − min(1, 0.3)`, 178.5 on eight bits.
    let cyan_grey = [178; 3];
    for (outer, inner, inner_grey) in [
        ("0 1 0 0 k", "0.6 g", GREY_60),
        ("0 1 0 0 k", "1 0 0 0 k", cyan_grey),
        ("/IK cs 1 scn", "/IG cs 1 scn", GREY_60),
    ] {
        let actual = render(
            &format!(
                "{outer} q {inner} q 1 0 0 1 100 10 cm /GInherit Do Q Q q 1 0 0 1 200 10 cm /GInherit Do Q {}",
                patch(300)
            ),
            "",
        );
        near(pixel(&actual, 130, 50), inner_grey);
        near(pixel(&actual, 230, 50), MAGENTA_GREY);
        near(pixel(&actual, 330, 50), MAGENTA);
    }
}

#[test]
fn stroke_stencil_and_alpha_paint_the_inherited_cmyk_colour_in_the_group() {
    for selection in ["0 1 0 0 k 0 1 0 0 K", "/IK cs 1 scn /IK CS 1 SCN"] {
        let actual = render(&format!("{selection} q 1 0 0 1 100 10 cm /GMarks Do Q"), "");
        near(pixel(&actual, 110, 50), MAGENTA_GREY);
        near(pixel(&actual, 135, 50), MAGENTA_GREY);
        near(
            pixel(&actual, 162, 50),
            MAGENTA_GREY.map(|v| f64::midpoint(f64::from(v), 255.0).round() as u8),
        );
    }
}

#[test]
fn device_solids_on_the_device_read_no_intent_or_black_point() {
    for (selection, wanted) in [("0 1 0 0 k", MAGENTA), ("0.6 g", GREY_60)] {
        let actual = render(
            &format!(
                "{selection} {} /Saturation ri {} /On gs {}",
                patch(0),
                patch(100),
                patch(200)
            ),
            "",
        );
        for x in [30, 130, 230] {
            near(pixel(&actual, x, 50), wanted);
        }
    }
}
