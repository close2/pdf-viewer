//! §11.6.5.1's soft-mask spot-colour substitution, exercised through a full page render.
//!
//! The fixture keeps the named Separation as an ordinary page mark and also uses it inside a
//! luminosity-mask group. Its tint transform maps tint .25 to gray .75 in `DeviceGray` or `DeviceRGB`. Each mask must match
//! a control group that paints that alternate gray directly. Under spot simulation the ordinary page mark establishes
//! availability of the native colourant; the two mask consumers must still match.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test fixture is a complete numbered PDF and each pixel index is on its 40x40 raster"
)]

use std::fmt::Write as _;

use pdf_model::colour::Plane;
use pdf_render::{Command, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const PIXELS: u64 = 1 << 20;

fn stream_object(number: usize, dictionary: &str, content: &[u8]) -> Vec<u8> {
    let mut object = format!(
        "{number} 0 obj\n<< {dictionary} /Length {} >>\nstream\n",
        content.len()
    )
    .into_bytes();
    object.extend_from_slice(content);
    object.extend_from_slice(b"\nendstream\nendobj\n");
    object
}

fn fixture() -> Vec<u8> {
    let page_content = b"/Ink cs 0.25 scn 0 0 10 40 re f \
                         /SpotMask gs 0 g 10 0 10 40 re f \
                         /GrayMask gs 0 g 20 0 10 40 re f";
    let spot_group = b"/Ink cs 0.25 scn 0 0 40 40 re f";
    let gray_group = b"0.75 g 0 0 40 40 re f";
    let objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Resources << /ColorSpace << /Ink [/Separation /Ink /DeviceGray 7 0 R] >> \
          /ExtGState << /SpotMask 5 0 R /GrayMask 8 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        stream_object(4, "", page_content),
        b"5 0 obj\n<< /Type /ExtGState /SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [1] >> >>\nendobj\n".to_vec(),
        stream_object(
            6,
            "/Type /XObject /Subtype /Form /BBox [0 0 40 40] \
             /Resources << /ColorSpace << /Ink [/Separation /Ink /DeviceGray 7 0 R] >> >> \
             /Group << /Type /Group /S /Transparency /CS /DeviceGray >>",
            spot_group,
        ),
        b"7 0 obj\n<< /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >>\nendobj\n".to_vec(),
        b"8 0 obj\n<< /Type /ExtGState /SMask << /Type /Mask /S /Luminosity /G 9 0 R /BC [1] >> >>\nendobj\n".to_vec(),
        stream_object(
            9,
            "/Type /XObject /Subtype /Form /BBox [0 0 40 40] /Resources << >> \
             /Group << /Type /Group /S /Transparency /CS /DeviceGray >>",
            gray_group,
        ),
    ];
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in &objects {
        offsets.push(bytes.len());
        bytes.extend_from_slice(object);
    }
    let xref = bytes.len();
    let size = offsets.len().saturating_add(1);
    let mut trailer = String::new();
    let _ = writeln!(trailer, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    bytes.extend_from_slice(trailer.as_bytes());
    bytes
}

fn rgb_fixture() -> Vec<u8> {
    let page_content = b"/Ink cs 0.25 scn 0 0 10 40 re f \
                         /SpotMask gs 0 g 10 0 10 40 re f \
                         /GrayMask gs 0 g 20 0 10 40 re f";
    let spot_group = b"/Ink cs 0.25 scn 0 0 40 40 re f";
    let rgb_group = b"0.75 0.75 0.75 rg 0 0 40 40 re f";
    let objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Resources << /ColorSpace << /Ink [/Separation /Ink /DeviceRGB 7 0 R] >> \
          /ExtGState << /SpotMask 5 0 R /GrayMask 8 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        stream_object(4, "", page_content),
        b"5 0 obj\n<< /Type /ExtGState /SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [1 1 1] >> >>\nendobj\n".to_vec(),
        stream_object(
            6,
            "/Type /XObject /Subtype /Form /BBox [0 0 40 40] \
             /Resources << /ColorSpace << /Ink [/Separation /Ink /DeviceRGB 7 0 R] >> >> \
             /Group << /Type /Group /S /Transparency /CS /DeviceRGB >>",
            spot_group,
        ),
        b"7 0 obj\n<< /FunctionType 2 /Domain [0 1] /C0 [1 1 1] /C1 [0 0 0] /N 1 >>\nendobj\n".to_vec(),
        b"8 0 obj\n<< /Type /ExtGState /SMask << /Type /Mask /S /Luminosity /G 9 0 R /BC [1 1 1] >> >>\nendobj\n".to_vec(),
        stream_object(
            9,
            "/Type /XObject /Subtype /Form /BBox [0 0 40 40] /Resources << >> \
             /Group << /Type /Group /S /Transparency /CS /DeviceRGB >>",
            rgb_group,
        ),
    ];
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in &objects {
        offsets.push(bytes.len());
        bytes.extend_from_slice(object);
    }
    let xref = bytes.len();
    let size = offsets.len().saturating_add(1);
    let mut trailer = String::new();
    let _ = writeln!(trailer, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    bytes.extend_from_slice(trailer.as_bytes());
    bytes
}

fn interpreted(bytes: Vec<u8>, simulate_spots: bool) -> pdf_model::Interpretation {
    let document = Document::open(bytes).expect("valid complete PDF fixture");
    assert!(!document.was_recovered(), "exact fixture xref");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let mut state = pdf_model::view::ViewState::of(&document);
    state.set_separation_simulation(simulate_spots);
    pdf_model::content::interpret_with(&document, &page, &state)
}

fn fill_colours(commands: &[Command], output: &mut Vec<[f32; 3]>) {
    for command in commands {
        match command {
            Command::Fill {
                paint: pdf_render::Paint::Solid(color),
                ..
            } => {
                output.push([color.r, color.g, color.b]);
            }
            Command::Group { commands, .. } => fill_colours(commands, output),
            _ => {}
        }
    }
}

fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    assert!(x < raster.width && y < raster.height);
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let at = usize::try_from(row.saturating_mul(raster.width).saturating_add(x))
        .expect("small fixture")
        .saturating_mul(4);
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

fn gray(raster: &pdf_render::Raster, x: u32, y: u32, expected: u8, label: &str) {
    let got = pixel(raster, x, y);
    for channel in &got[..3] {
        assert!(
            channel.abs_diff(expected) <= 2,
            "{label}: got {got:?}, expected {expected}"
        );
    }
    assert_eq!(got[3], 255, "opaque page");
}

fn check_spot_mask(bytes: &[u8]) {
    for simulate_spots in [false, true] {
        let interpretation = interpreted(bytes.to_vec(), simulate_spots);
        assert!(
            interpretation.is_complete(),
            "unsupported: {:?}",
            interpretation.unsupported
        );
        let target = TargetSpec::for_page(&interpretation.display_list, 1.0, PIXELS)
            .expect("valid raster target");
        let raster = CpuRasterizer::new()
            .rasterize(&interpretation.display_list, target)
            .expect("CPU rasterizer supports fixture");

        // Without press simulation, black through luminosity .75 leaves one quarter white.
        // With simulation, compare equivalent masks after the same press conversion.
        if simulate_spots {
            let direct = pixel(&raster, 25, 20);
            let spot = pixel(&raster, 15, 20);
            assert_eq!(direct[3], 255);
            assert_eq!(spot[3], 255);
            for (actual, expected) in spot.iter().zip(direct) {
                assert!(
                    actual.abs_diff(expected) <= 2,
                    "spot mask {spot:?} must match direct alternate mask {direct:?}"
                );
            }
        } else {
            gray(&raster, 25, 20, 64, "direct alternate soft-mask control");
            gray(&raster, 15, 20, 64, "spot mask alternate-space control");
            gray(&raster, 5, 20, 191, "ordinary spot alternate-space control");
            gray(&raster, 35, 20, 255, "unpainted page control");
        }

        if simulate_spots {
            let separation = interpretation
                .separation
                .expect("the ordinary page mark names /Ink");
            assert_eq!(separation.colourants().len(), 1);
            let spot_plane =
                pdf_model::colourants::plane(&interpretation.display_list, Plane::Spot(0))
                    .expect("ordinary /Ink mark creates the simulated spot plane");
            let mut colours = Vec::new();
            fill_colours(spot_plane.commands(), &mut colours);
            let native_mark = colours
                .iter()
                .filter(|colour| {
                    (colour[0] - 0.75).abs() < 1.0 / 4096.0
                        && (colour[1] - 1.0).abs() < 1.0 / 4096.0
                        && (colour[2] - 1.0).abs() < 1.0 / 4096.0
                })
                .count();
            assert_eq!(
                native_mark, 1,
                "the ordinary /Ink mark keeps native tint .25 (additive plane .75), confirming availability of the native spot colourant: {colours:?}"
            );
        }
    }
}

#[test]
fn soft_mask_group_uses_spot_alternate_when_native_colourant_is_available() {
    check_spot_mask(&fixture());
}

// The RGB compositing route consults available spot planes; the gray luminosity route does not.
// Both paths must substitute the alternate in a mask-defining group.
#[test]
fn rgb_soft_mask_group_uses_spot_alternate_when_native_colourant_is_available() {
    check_spot_mask(&rgb_fixture());
}
