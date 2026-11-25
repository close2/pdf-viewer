//! Specification audit of ISO 32000-2 §8.1 to §8.3, §8.8 and §10.1 to §10.4 (review 2237): the
//! coordinate space a pattern inside a Type 3 glyph description is defined against, and what a
//! `DeviceGray` colour becomes on §10.8.3's simulated press.
//!
//! Each test is one behaviour finding of
//! `doc/reviews/2237-specification-audit-8-1-8-3-8-8-10-1-10-4.md`, with a control the same
//! fixture passes on main beside the failing half. The fixtures are exact-xref documents built
//! here and drawn on the CPU oracle.

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

/// An axial shading pattern from red at pattern-space x = 0 to blue at x = 1000, extended both
/// ways, so a pixel's blue says where in pattern space it fell.
fn red_to_blue_pattern() -> String {
    "<< /PatternType 2 /Shading << /ShadingType 2 /ColorSpace /DeviceRGB \
     /Coords [0 0 1000 0] /Extend [true true] /Function << /FunctionType 2 /Domain [0 1] \
     /C0 [1 0 0] /C1 [0 0 1] /N 1 >> >> >>"
        .to_owned()
}

/// ISO 32000-2 §8.7.2, which §8.3.2.4's pattern-space bullet points to: "Every pattern has a
/// pattern matrix , a transformation matrix that maps the pattern's internal coordinate system
/// to the default coordinate system of the pattern's parent content stream (the content stream
/// in which the pattern is defined as a resource)." A Type 3 font's `/Resources` are, by Table
/// 110, "the named resources … required by the glyph descriptions in this font", so a pattern
/// stated there is defined as a resource of the glyph description, and §9.6.4 fixes that
/// stream's initial coordinate system: "When the glyph description begins execution, the current
/// transformation matrix (CTM) shall be the concatenation of the font matrix … and the text space
/// that was in effect at the time the text-showing operator was invoked".
///
/// The glyph fills its 1000-unit square with a shading pattern running red to blue over pattern
/// x 0 to 1000; at 100 points a text unit and a font matrix of 1/1000 the glyph covers the
/// 100-point page, so the page's centre is pattern x 500 — half blue. Against the page's default
/// space it would be pattern x 50, almost pure red. The control draws the same square and the
/// same pattern through a form `XObject` whose `/Matrix` is the same 1/10 scale, which
/// §8.7.2's own sentence about forms covers and the interpreter honours.
#[test]
fn a_pattern_in_a_type3_glyph_is_placed_in_the_glyphs_space_spec_audit() {
    let square = "/Pattern cs /P0 scn 0 0 1000 1000 re f";

    let form = pdf(
        100,
        "",
        "/XObject << /Fm0 5 0 R >>",
        "/Fm0 Do",
        &[
            format!(
                "/Type /XObject /Subtype /Form /BBox [0 0 1000 1000] \
                 /Matrix [0.1 0 0 0.1 0 0] /Resources << /Pattern << /P0 6 0 R >> >> \
                 @STREAM@{square}"
            ),
            red_to_blue_pattern(),
        ],
    );
    let in_form = pixel(form, false, 50, 50);
    assert!(
        (100..=160).contains(&in_form[2]),
        "control: inside a form the pattern is placed in the form's space, half blue: \
         {in_form:?}"
    );

    let glyph = pdf(
        100,
        "",
        "/Font << /F1 5 0 R >>",
        "BT /F1 100 Tf 0 0 Td (a) Tj ET",
        &[
            "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1000 1000] \
             /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /a 6 0 R >> \
             /Encoding << /Type /Encoding /Differences [97 /a] >> /FirstChar 97 /LastChar 97 \
             /Widths [1000] /Resources << /Pattern << /P0 7 0 R >> >> >>"
                .to_owned(),
            format!("@STREAM@1000 0 d0 {square}"),
            red_to_blue_pattern(),
        ],
    );
    let in_glyph = pixel(glyph, false, 50, 50);
    assert!(
        (100..=160).contains(&in_glyph[2]),
        "a pattern the glyph description's resources state is placed in the glyph's own \
         default space, half blue at the page's centre, not in the page's: {in_glyph:?}"
    );
}

/// A one-component `Separation` space named `name` whose alternate is pure cyan at full tint,
/// in the page's resources, and a mark painted in it at tint 0 in the page's corner so the page
/// names the colourant without inking the probe.
fn cyan_spot_resources(name: &str) -> (String, &'static str) {
    (
        format!(
            "/ColorSpace << /CS0 [/Separation /{name} /DeviceCMYK << /FunctionType 2 \
             /Domain [0 1] /C0 [0 0 0 0] /C1 [1 0 0 0] /N 1 >>] >>"
        ),
        "/CS0 cs 0 sc 0 0 1 1 re f ",
    )
}

/// An output intent naming the assumed inks as a four-component `DestOutputProfile`, objects 5
/// and 6, and the catalog entry that selects it.
fn cmyk_intent() -> (Vec<String>, &'static str) {
    let mut hex = String::new();
    for byte in pdf_colour::colour::assumed_inks_profile() {
        let _ = write!(hex, "{byte:02X}");
    }
    (
        vec![
            "<< /Type /OutputIntent /S /GTS_PDFX /OutputConditionIdentifier (test) \
             /DestOutputProfile 6 0 R >>"
                .to_owned(),
            format!("/N 4 /Filter /ASCIIHexDecode @STREAM@{hex}>"),
        ],
        "/OutputIntents [5 0 R]",
    )
}

/// ISO 32000-2 §10.2: "A device may also support additional spot colourants, which shall be
/// painted only by means of Separation or DeviceN colour spaces that define such spot
/// colourants", and before it, "Colours specified in any device or CIE-based colour space shall
/// be rendered as process colours."
///
/// On §10.8.3's simulated press under a CMYK output intent this tree reads `DeviceGray` as a
/// one-component `Separation` whose colourant is named `DeviceGray`, so that §10.3.2's
/// conversion (black `1 − gray`, §10.4.2.3) reaches every route. A page may name a spot
/// colourant `/DeviceGray` — a colourant name is any name — and once it does, the simulated
/// device has a plane of that name and the grey's construction names an available colourant:
/// `0.5 g` paints the page's spot plate instead of the process black. The control is the same
/// page with the spot named `/Gold`, where `0.5 g` and `0 0 0 0.5 k` draw alike.
#[test]
fn a_device_gray_colour_never_paints_a_spot_colourant_spec_audit() {
    let (objects, catalog) = cmyk_intent();
    let drawn = |name: &str, colour: &str| {
        let (resources, naming) = cyan_spot_resources(name);
        pixel(
            pdf(
                20,
                catalog,
                &resources,
                &format!("{naming}{colour} 2 2 16 16 re f"),
                &objects,
            ),
            true,
            10,
            10,
        )
    };

    let gold_grey = drawn("Gold", "0.5 g");
    assert_eq!(
        gold_grey,
        drawn("Gold", "0 0 0 0.5 k"),
        "control: beside a spot named /Gold the grey is the press's half-black ink"
    );
    assert_eq!(
        drawn("DeviceGray", "0.5 g"),
        gold_grey,
        "a DeviceGray colour is a process colour whatever the page names its spot colourants"
    );
}

/// ISO 32000-2 §10.3.2: "If the native device colour space is CMYK, then converting colours in
/// the DeviceGray colour space to that CMYK should follow the method described in 10.4.2.3",
/// whose formula is cyan, magenta and yellow `0.0` and black `1.0 − gray`.
///
/// §10.8.3 step a) processes the page "as if separations were to be created for a simulated
/// device that supports subtractive process colourants", and "\[t]he PDF processor determines
/// what process colours … the simulated device is to have": with no output intent this tree
/// simulates its assumed inks, a device whose native colour space is CMYK as much as an intent's
/// is. The control is the same grey under a CMYK output intent, where ADR 2758 already reads it
/// as the press's black ink; without one, `0.5 g` is separated into all four inks as a neutral.
#[test]
fn a_grey_is_black_ink_alone_on_the_assumed_simulated_press_spec_audit() {
    let (objects, catalog) = cmyk_intent();
    let with_intent = |colour: &str| pixel(pdf(20, catalog, "", colour, &objects), true, 10, 10);
    assert_eq!(
        with_intent("0.5 g 0 0 20 20 re f"),
        with_intent("0 0 0 0.5 k 0 0 20 20 re f"),
        "control: under a CMYK intent the simulated grey is black 1 − gray"
    );

    let without = |colour: &str| pixel(pdf(20, "", "", colour, &[]), true, 10, 10);
    assert_eq!(
        without("0.5 g 0 0 20 20 re f"),
        without("0 0 0 0.5 k 0 0 20 20 re f"),
        "on the assumed simulated press, too, §10.4.2.3's black 1 − gray and no other ink"
    );
}
