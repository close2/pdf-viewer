//! Specification audit of ISO 32000-2 §10.5 to §10.8 (review 2248): §10.5's transfer function on
//! §10.8.3's simulated press, whose device is a subtractive one, and §10.7.5's stroke adjustment
//! of a rule one and a half device pixels wide or wider.
//!
//! Each test is one behaviour finding of `doc/reviews/2248-specification-audit-10-5-10-8.md`,
//! with a control the same fixture passes on main beside the failing half. The fixtures are
//! exact-xref documents built here and drawn on the CPU oracle at one device pixel a point.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::doc_markdown,
    reason = "exact-xref test fixtures, their assertions, a small raster's pixel index, and \
              verbatim quotations of the standard, which may not gain backticks"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// A classic-xref PDF whose catalog is object 1 and states `catalog`, whose page is object 3 of
/// `size` points square with `resources` and the content stream `content` (object 4), and whose
/// further objects `extra` are numbered from 5. A body holding `@STREAM@` is a stream: what
/// precedes the marker is the dictionary's entries, what follows it the data.
fn pdf(size: u32, catalog: &str, resources: &str, content: &str, extra: &[String]) -> Vec<u8> {
    let mut objects = vec![
        format!("<< /Type /Catalog /Pages 2 0 R {catalog} >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {size} {size}] \
             /Resources << {resources} >> /Contents 4 0 R >>"
        ),
        format!("@STREAM@{content}"),
    ];
    objects.extend_from_slice(extra);
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let body = match body.split_once("@STREAM@") {
            Some((dict, data)) => format!(
                "<< {dict} /Length {} >>\nstream\n{data}\nendstream",
                data.len().saturating_add(1)
            ),
            None => body.clone(),
        };
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

/// The colour of the pixel at (`x`, `y`) from the top left, drawn at 72 dpi on the CPU oracle,
/// with §10.8.3's simulation asked for when `simulate`.
fn pixel(bytes: Vec<u8>, simulate: bool, x: u32, y: u32) -> [u8; 3] {
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
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("a small target");
    let raster = CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("the fixture rasterizes");
    let at = usize::try_from(y * raster.width + x).expect("a pixel index") * 4;
    [raster.data[at], raster.data[at + 1], raster.data[at + 2]]
}

/// A type 2 function from 0 to 1: the identity on 0..1.
const IDENTITY: &str = "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >>";
/// A type 2 function that answers 0 everywhere.
const ZERO: &str = "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [0] /N 1 >>";

/// Resources whose `/GS` states Table 57's `/TR` as "an array of four functions (which apply to
/// the process colourants individually)", red, green, blue and gray in that order (§10.5).
fn four_transfers(four: [&str; 4]) -> String {
    format!(
        "/ExtGState << /GS << /TR [{} {} {} {}] >> >>",
        four[0], four[1], four[2], four[3]
    )
}

/// ISO 32000-2 §10.5: "An RGB device shall use the first three, a monochrome device shall use
/// the gray transfer function only, and a CMYK device shall use all four." and "If the component
/// is subtractive (cyan, magenta, yellow, black, or a spot colour), it shall be converted to
/// additive form by subtracting it from 1.0 before it is passed to the transfer function."
/// §10.8.3 a): "Process the PDF as if separations were to be created for a simulated device that
/// supports subtractive process colourants".
///
/// `/TR` is the identity for red, green and blue and zero for gray, and the page fills with
/// `0 0 0 0 k`. On the screen, an RGB device, the first three apply and the page stays white —
/// the control. On the simulated press the device is CMYK: black 0 is additive 1, the fourth
/// function maps it to 0, and subtracted from 1.0 again that is full black ink, so the separation
/// the simulation blends is black. Applying the RGB triple to the blended result instead leaves
/// the page white.
#[test]
fn the_simulated_press_takes_all_four_transfer_functions_spec_audit() {
    let page = || {
        pdf(
            100,
            "",
            &four_transfers([IDENTITY, IDENTITY, IDENTITY, ZERO]),
            "/GS gs 0 0 0 0 k 0 0 100 100 re f",
            &[],
        )
    };
    let screen = pixel(page(), false, 50, 50);
    assert!(
        screen.iter().all(|&level| level > 247),
        "control: the screen is an RGB device and uses the first three, the identity: {screen:?}"
    );
    let press = pixel(page(), true, 50, 50);
    assert!(
        press.iter().all(|&level| level < 100),
        "the simulated CMYK device passes black through the fourth function, which makes it \
         full black ink: {press:?}"
    );
}

/// ISO 32000-2 §10.5: "When the current colour space is DeviceGray and the output device's native
/// colour space is DeviceCMYK , a PDF processor shall use only the gray transfer function. The
/// normal conversion from DeviceGray to DeviceCMYK produces 0.0 for the cyan, magenta, and yellow
/// components. These components shall not be passed through their respective transfer functions
/// but are rendered directly, producing output containing no coloured inks."
///
/// `/TR` is zero for red, green and blue — cyan, magenta and yellow on a CMYK device — and the
/// identity for gray, and the page fills with `1 g`. On the screen the first three apply and the
/// white goes black, the control. On the simulated press only the gray function applies: black
/// ink 0 is additive 1, mapped to 1, no ink; cyan, magenta and yellow 0 are rendered directly, no
/// ink — the page stays white.
#[test]
fn a_grey_on_the_simulated_press_takes_the_gray_transfer_function_alone_spec_audit() {
    let page = || {
        pdf(
            100,
            "",
            &four_transfers([ZERO, ZERO, ZERO, IDENTITY]),
            "/GS gs 1 g 0 0 100 100 re f",
            &[],
        )
    };
    let screen = pixel(page(), false, 50, 50);
    assert!(
        screen.iter().all(|&level| level < 8),
        "control: on the RGB screen the first three functions, zero, apply: {screen:?}"
    );
    let press = pixel(page(), true, 50, 50);
    assert!(
        press.iter().all(|&level| level > 240),
        "on the simulated CMYK device a DeviceGray white takes the gray function alone and no \
         coloured ink: {press:?}"
    );
}

/// The red level of column 50, rows 44 to 56 from the top.
fn column(bytes: &[u8]) -> Vec<u8> {
    (44..56)
        .map(|y| pixel(bytes.to_vec(), false, 50, y)[0])
        .collect()
}

/// ISO 32000-2 §10.7.5: "When stroke adjustment is enabled, the line width and the coordinates of
/// a stroke shall automatically be adjusted as necessary to produce lines of uniform thickness.
/// The thickness shall be as near as possible to the requested line width -no more than half a
/// pixel different." The paragraph before it names the defect: "A line of a given width can
/// intersect with different numbers of device pixels, depending on where it is positioned."
///
/// Under `/SA true` a horizontal rule along the device axis is centred at device y = 49.75. One
/// point wide it is fitted to one whole row, the control. Two points wide it is not adjusted: it
/// covers a quarter of row 48, row 49 and three quarters of row 50, three rows where the same
/// rule placed on a pixel boundary covers two. Two whole rows are within half a pixel of the
/// requested width; the clause asks for that adjustment at every width it enables.
#[test]
fn stroke_adjustment_reaches_a_rule_two_pixels_wide_spec_audit() {
    let resources = "/ExtGState << /GS << /SA true >> >>";
    let rule = |width: &str| {
        pdf(
            100,
            "",
            resources,
            &format!("/GS gs {width} w 10 50.25 m 90 50.25 l S"),
            &[],
        )
    };
    let partial = |levels: &[u8]| levels.iter().filter(|&&l| l > 8 && l < 247).count();
    let dark = |levels: &[u8]| levels.iter().filter(|&&l| l <= 8).count();

    let one = column(&rule("1"));
    assert!(
        partial(&one) == 0 && dark(&one) == 1,
        "control: a one-pixel rule under /SA true is one whole row: {one:?}"
    );
    let two = column(&rule("2"));
    assert!(
        partial(&two) == 0 && dark(&two) == 2,
        "a two-pixel rule under /SA true is two whole rows, not a partly covered row either \
         side of one: {two:?}"
    );
}
