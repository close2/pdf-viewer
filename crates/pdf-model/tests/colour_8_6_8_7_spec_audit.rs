//! Specification audit of ISO 32000-2 §8.6 and §8.7 (colour spaces and patterns):
//! `doc/reviews/2184-specification-audit-8-6-8-7.md`.
//!
//! Each `*_spec_audit` test is one behaviour finding and fails on the tree the audit read; the
//! control beside it passes there, so the failure is the finding and not the fixture.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::doc_markdown,
    reason = "small exact-xref fixtures, a centre pixel index, and the standard quoted verbatim"
)]

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use std::fmt::Write as _;

/// A one-page document of `objects` (numbered from 1; object 1 the catalog, 2 the page tree,
/// 3 the page) with exact cross-reference offsets.
fn document(objects: &[String]) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{object}\nendobj\n", index + 1);
    }
    let xref = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

fn stream(dictionary: &str, data: &str) -> String {
    format!(
        "<< {dictionary} /Length {} >>\nstream\n{data}\nendstream",
        data.len()
    )
}

/// A page of 100 by 100 points with `resources`, `group` (empty for none) and `content`, plus
/// the further objects `extra`, numbered from 5.
fn page(group: &str, resources: &str, content: &str, extra: &[String]) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] {group} \
             /Resources {resources} /Contents 4 0 R >>"
        ),
        stream("", content),
    ];
    objects.extend_from_slice(extra);
    document(&objects)
}

/// The centre pixel of the page at 72 dpi on the CPU oracle.
fn centre(bytes: Vec<u8>) -> [u8; 3] {
    let document = Document::open(bytes).expect("fixture parses");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("100-point target");
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("fixture rasterizes");
    assert_eq!((raster.width, raster.height), (100, 100));
    let at = ((50 * raster.width + 50) * 4) as usize;
    [raster.data[at], raster.data[at + 1], raster.data[at + 2]]
}

// ---------------------------------------------------------------------------------------------
// 2184-B01: §8.6.8's graphics state entries inside a `d1` glyph description.
// ---------------------------------------------------------------------------------------------

const BG_INVERT: &str = "<< /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >>";
const UCR_HALF: &str = "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [0.5] /N 1 >>";

/// A page whose `DeviceCMYK` page group converts a `DeviceRGB` fill through Table 57's black
/// generation and undercolour removal (§10.4.2.4, §11.7.5.3), painting one Type 3 glyph that
/// covers the page. `glyph_gs` is the `gs` the glyph description runs after its `d1`, `page_gs`
/// the one the page runs before the text.
fn type3_page(page_gs: &str, glyph_gs: &str) -> Vec<u8> {
    let pair = format!("/BG2 {BG_INVERT} /UCR2 {UCR_HALF}");
    let glyph = format!("1000 0 0 0 1000 1000 d1 {glyph_gs} 0 0 1000 1000 re f");
    page(
        "/Group << /S /Transparency /CS /DeviceCMYK >>",
        &format!("<< /Font << /F1 5 0 R >> /ExtGState << /G << {pair} >> >> >>"),
        &format!("{page_gs} 0.2 0.4 0.6 rg BT /F1 100 Tf 0 0 Td (a) Tj ET"),
        &[
            format!(
                "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1000 1000] \
                 /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /a 6 0 R >> \
                 /Encoding << /Differences [97 /a] >> /FirstChar 97 /LastChar 97 \
                 /Widths [1000] /Resources << /ExtGState << /G << {pair} >> >> >> >>"
            ),
            stream("", &glyph),
        ],
    )
}

/// ISO 32000-2 §8.6.8, of "any glyph description that uses the d1 operator": "All of the
/// following entries, if present in the graphics state parameter dictionary of a gs operator
/// shall be ignored" — `TR`, `TR2`, `HT`, `BG`, `BG2`, `UCR`, `UCR2` and `UseBlackPtComp`. The
/// glyph's colour is the text's, "specified separately each time [it is] used", so a `/BG2` and
/// `/UCR2` stated inside the glyph must leave the converted colour as it was.
#[test]
fn a_d1_glyphs_black_generation_is_ignored_spec_audit() {
    let without = centre(type3_page("", ""));
    // Control: the same pair stated by the page changes the colour, so the entries are
    // observable here at all.
    let by_the_page = centre(type3_page("/G gs", ""));
    assert_ne!(
        by_the_page, without,
        "Table 57's pair converts the RGB fill differently in a DeviceCMYK group"
    );
    let by_the_glyph = centre(type3_page("", "/G gs"));
    assert_eq!(
        by_the_glyph, without,
        "§8.6.8: /BG2 and /UCR2 in a d1 glyph's gs shall be ignored"
    );
}

// ---------------------------------------------------------------------------------------------
// 2184-B02: an `ICCBased` space this reader cannot apply and which states no `/Alternate`.
// ---------------------------------------------------------------------------------------------

/// A page selecting `[/ICCBased 5 0 R]` as `/CS0`, whose stream holds no ICC profile.
fn icc_page(stream_entries: &str, content: &str) -> Vec<u8> {
    page(
        "",
        "<< /ColorSpace << /CS0 [/ICCBased 5 0 R] >> >>",
        content,
        &[stream(stream_entries, "not an ICC profile")],
    )
}

/// ISO 32000-2 §8.6.8: "In a Lab or ICCBased colour space, the initial colour shall have all
/// components equal to 0.0 unless that falls outside the intervals specified by the space's
/// Range entry". Table 65 makes `DeviceCMYK` the space "that shall be used" where `/Alternate`
/// is omitted and the profile is not understood — the space the values are *drawn* in — but the
/// selected space is still the `ICCBased` one, whose initial colour is no ink at all.
#[test]
fn an_unreadable_icc_space_without_alternate_starts_at_zero_spec_audit() {
    let fill = "/CS0 cs 0 0 100 100 re f";
    // Control: the same profile with its `/Alternate` stated starts at no ink, which is white.
    let stated = centre(icc_page("/N 4 /Alternate /DeviceCMYK", fill));
    assert_eq!(stated, [255, 255, 255], "ICCBased initial colour [0 0 0 0]");
    let omitted = centre(icc_page("/N 4", fill));
    assert_eq!(
        omitted, stated,
        "§8.6.8: an ICCBased space's initial colour is all zero whichever alternate draws it"
    );
}

/// Table 65's `/Alternate` row: "Colour values within the range of the ICCBased colour space
/// might not be within the range of the alternate colour space. In this case and after
/// constraining to the ICCBased range, the nearest values within the range of the alternate
/// space shall be substituted without error indication." The alternate the row names includes
/// the one it supplies where the entry is omitted, so a value is held to `/Range` first there
/// too.
#[test]
fn an_unreadable_icc_space_without_alternate_keeps_its_range_spec_audit() {
    let fill = "/CS0 cs 0.8 sc 0 0 100 100 re f";
    let half = centre(page(
        "",
        "<< >>",
        "/DeviceGray cs 0.5 sc 0 0 100 100 re f",
        &[],
    ));
    // Control: the same `/Range` with `/Alternate` stated is held to 0.5 (ADR 2973).
    let stated = centre(icc_page("/N 1 /Range [0 0.5] /Alternate /DeviceGray", fill));
    assert_eq!(stated, half, "held to /Range before the stated alternate");
    let omitted = centre(icc_page("/N 1 /Range [0 0.5]", fill));
    assert_eq!(
        omitted, half,
        "Table 65: constrained to the ICCBased range before the N-implied alternate"
    );
}

// ---------------------------------------------------------------------------------------------
// 2184-B03: §8.6.7's nonzero overprint mode, of an `Indexed` space whose base is `DeviceCMYK`.
// ---------------------------------------------------------------------------------------------

/// A `DeviceCMYK` page group painted `0 0 0 1 k` and then filled under `/OP true /op true
/// /OPM 1` by `paint`, with `/CS0` an `Indexed` space whose one entry is `0.8 0 0 0`.
fn overprint_page(paint: &str) -> Vec<u8> {
    page(
        "/Group << /S /Transparency /CS /DeviceCMYK >>",
        "<< /ExtGState << /GS << /OP true /op true /OPM 1 >> >> \
         /ColorSpace << /CS0 [/Indexed /DeviceCMYK 0 <CC000000>] >> >>",
        &format!("0 0 0 1 k 0 0 100 100 re f /GS gs {paint} 0 0 100 100 re f"),
        &[],
    )
}

/// ISO 32000-2 §8.6.7: "Non-zero overprint mode shall apply only to painting operations that use
/// the current colour in the graphics state when the current colour space is DeviceCMYK", and
/// §11.7.4.3 NOTE 2 says what the current colour space is for an `Indexed` one: "In the case of
/// an Indexed space, it refers to the base colour space". So an `Indexed` colour whose base is
/// `DeviceCMYK` and whose entry is `0.8 0 0 0` keeps the backdrop's black exactly as `0.8 0 0 0
/// k` does — the page is `0.8 0 0 1` in the group's components.
#[test]
fn an_indexed_cmyk_colour_takes_nonzero_overprint_mode_spec_audit() {
    let direct = centre(overprint_page("0.8 0 0 0 k"));
    let named = centre(page(
        "/Group << /S /Transparency /CS /DeviceCMYK >>",
        "<< >>",
        "0.8 0 0 1 k 0 0 100 100 re f",
        &[],
    ));
    // Control: the device colour keeps the backdrop's black.
    assert_eq!(direct, named, "§11.7.4.3's first bullet for 0.8 0 0 0 k");
    let indexed = centre(overprint_page("/CS0 cs 0 sc"));
    assert_eq!(
        indexed, named,
        "§11.7.4.3 NOTE 2: the current colour space of an Indexed space is its DeviceCMYK base"
    );
}

// ---------------------------------------------------------------------------------------------
// 2184-B04: a `Pattern` space where a clause forbids one, read as a colour.
// ---------------------------------------------------------------------------------------------

/// Whether the page's interpretation says it drew everything the file states.
fn complete(bytes: Vec<u8>) -> bool {
    let document = Document::open(bytes).expect("fixture parses");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    pdf_model::interpret(&document, &page).is_complete()
}

/// ISO 32000-2 Table 77 makes a shading's `/ColorSpace` "any colour space except a Pattern
/// space", and §8.7.1 says what a `Pattern` space's values are: "These spaces shall use pattern
/// objects as the equivalent of colour values instead of the numeric component values used with
/// other spaces." So no numeric value a shading's function returns names a colour in one, and a
/// page that paints such a shading in some colour and calls itself complete has drawn a colour no
/// clause gives it. Control: the same shading in `DeviceGray` is complete.
#[test]
fn a_shading_in_a_pattern_space_is_not_drawn_as_complete_spec_audit() {
    let shading = |space: &str| {
        page(
            "",
            &format!(
                "<< /Shading << /Sh << /ShadingType 2 /ColorSpace {space} \
                 /Coords [0 0 100 0] /Function << /FunctionType 2 /Domain [0 1] \
                 /C0 [0] /C1 [1] /N 1 >> /Extend [true true] >> >> >>"
            ),
            "/Sh sh",
            &[],
        )
    };
    assert!(complete(shading("/DeviceGray")), "a grey axial shading");
    assert!(
        !complete(shading("/Pattern")),
        "Table 77: a Pattern space states no colour for a shading's values"
    );
}

/// §8.6.6.3: an `Indexed` space's base "shall be any device or CIE-based colour space or (PDF
/// 1.3) a Separation or DeviceN space, but shall not be a Pattern space or another Indexed
/// space", for §8.7.1's reason above: the table's bytes are numeric components and a `Pattern`
/// space has none. Control: the same table over `DeviceGray` is complete.
#[test]
fn an_indexed_space_over_a_pattern_base_is_not_drawn_as_complete_spec_audit() {
    let indexed = |base: &str| {
        page(
            "",
            &format!("<< /ColorSpace << /CS0 [/Indexed {base} 1 <00FF>] >> >>"),
            "/CS0 cs 1 sc 0 0 100 100 re f",
            &[],
        )
    };
    assert!(complete(indexed("/DeviceGray")), "an Indexed grey");
    assert!(
        !complete(indexed("/Pattern")),
        "§8.6.6.3: a Pattern base states no colour for a table entry"
    );
}

/// §8.6.8 of `SCN` and `scn` in a `Pattern` space: "name shall be the name of an entry in the
/// Pattern subdictionary of the current resource dictionary", and §8.7.1 again: a `Pattern`
/// space uses "pattern objects as the equivalent of colour values instead of the numeric
/// component values used with other spaces". A bare number therefore selects no pattern and
/// names no colour; the colour `cs` left is §8.6.8's "pattern object that causes nothing to be
/// painted". Control: the same fill with nothing after `cs` paints nothing.
#[test]
fn a_number_in_a_pattern_space_paints_no_colour_spec_audit() {
    let filled = |colour: &str| {
        centre(page(
            "",
            "<< >>",
            &format!("/Pattern cs {colour} 0 0 100 100 re f"),
            &[],
        ))
    };
    assert_eq!(
        filled(""),
        [255, 255, 255],
        "the initial pattern paints nothing"
    );
    assert_eq!(
        filled("0.5 scn"),
        [255, 255, 255],
        "§8.6.8: a Pattern space's operand is a pattern name, and a number names no colour"
    );
}
