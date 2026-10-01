//! ISO 32000-2 §8.6.6.6's multitone examples, run as a document (habit 39).
//!
//! The clause states no requirement of its own: it composes §8.6.6.3's `Indexed` space over
//! §8.6.6.5's `DeviceN`, and both are implemented. What it does ship is three complete colour
//! spaces with their tint transformation functions, and an example the standard prints is a
//! claim about what those two clauses produce together — so each is drawn here beside a
//! reference that states the same colour directly, and the two must agree.
//!
//! - **Example 1**, a cyan-and-black duotone over `DeviceCMYK`, whose function "maps the two
//!   tint components for cyan and black to the four components for a `DeviceCMYK` colour space by
//!   supplying zero values for the other two components". Its lookup table is the five entries
//!   the clause prints, so `hival` is 4 where the example's elided table has 255.
//! - **Examples 3 and 4**, black and gold with the three `CalRGB` components carried as `None`
//!   colourants in the same table entry, and the Type 4 function that "can merely discard the
//!   first two components and return the last three". Gold is a spot colourant a screen does not
//!   have, so the tint transformation is what draws it.
//! - **Example 5**, the quadtone: black and three PANTONE spots, three `None` components, and a
//!   function that discards the first four. Its lookup table is a stream rather than a string.
//!
//! Example 2 states its function only as a formula left to the reader, so it has nothing to run.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and this page is 100 units \
              square where no arithmetic can overflow"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;

/// The clause's `CalRGB` dictionary, shared by Examples 3 to 5 and their references.
const CAL_RGB: &str = "[/CalRGB << /WhitePoint [1.0 1.0 1.0] /Gamma [2.2 2.2 2.2] >>]";

/// Example 1's tint transformation function, object 15 in the clause.
const DUOTONE: &str = "{ 0 0 3 -1 roll }";

/// Example 4's, which Example 3's black-and-gold space names.
const BLACK_GOLD: &str = "{ 5 3 roll pop pop }";

/// Example 5's, object 20 there: "Just discard first four values".
const QUADTONE: &str = "{ 7 3 roll pop pop pop pop }";

/// The fixture: six named colour spaces, and `content` drawing with them on a 100-unit page.
fn fixture(content: &str) -> Vec<u8> {
    // Example 5's lookup table: index 0 is a seven-component entry whose last three bytes are
    // the CalRGB colour, and index 1 another.
    let quadtone_table: &[u8] = &[
        0x10, 0x20, 0x30, 0x40, 0x33, 0x66, 0x99, //
        0xFF, 0x00, 0x80, 0x00, 0xC0, 0x40, 0x20,
    ];
    let mut quadtone_hex = String::new();
    for byte in quadtone_table {
        let _ = write!(quadtone_hex, "{byte:02X}");
    }
    let quadtone_hex = format!("{quadtone_hex}>");
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /ColorSpace << \
         /Duotone [/Indexed [/DeviceN [/Cyan /Black] /DeviceCMYK 6 0 R] 4 \
         <6605 6806 6907 6B09 6C0A>] \
         /DuotoneReference [/Indexed /DeviceCMYK 4 <66000005 68000006 69000007 6B000009 \
         6C00000A>] \
         /BlackGold [/Indexed [/DeviceN [/Black /Gold /None /None /None] {CAL_RGB} 7 0 R] 1 \
         <FF80336699 00FFC04020>] \
         /Quadtone [/Indexed 8 0 R 1 9 0 R] \
         /CalReference [/Indexed {CAL_RGB} 1 <336699 C04020>] \
         /QuadtoneReference 12 0 R >> >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\nnull\nendobj\n\
         6 0 obj\n<< /FunctionType 4 /Domain [0.0 1.0 0.0 1.0] \
         /Range [0.0 1.0 0.0 1.0 0.0 1.0 0.0 1.0] /Length {} >>\n\
         stream\n{DUOTONE}\nendstream\nendobj\n\
         7 0 obj\n<< /FunctionType 4 /Domain [0.0 1.0 0.0 1.0 0.0 1.0 0.0 1.0 0.0 1.0] \
         /Range [0.0 1.0 0.0 1.0 0.0 1.0] /Length {} >>\n\
         stream\n{BLACK_GOLD}\nendstream\nendobj\n\
         8 0 obj\n[/DeviceN [/Black /PANTONE#20216#20CVC /PANTONE#20409#20CVC \
         /PANTONE#202985#20CVC /None /None /None] 10 0 R 11 0 R]\nendobj\n\
         9 0 obj\n<< /Filter /ASCIIHexDecode /Length {} >>\nstream\n{quadtone_hex}\n\
         endstream\nendobj\n\
         10 0 obj\n[/CalRGB << /WhitePoint [1.0 1.0 1.0] >>]\nendobj\n\
         11 0 obj\n<< /FunctionType 4 \
         /Domain [0.0 1.0 0.0 1.0 0.0 1.0 0.0 1.0 0.0 1.0 0.0 1.0 0.0 1.0] \
         /Range [0.0 1.0 0.0 1.0 0.0 1.0] /Length {} >>\n\
         stream\n{QUADTONE}\nendstream\nendobj\n\
         12 0 obj\n[/Indexed [/CalRGB << /WhitePoint [1.0 1.0 1.0] >>] 1 <336699 C04020>]\n\
         endobj\n",
        content.len() + 1,
        DUOTONE.len() + 1,
        BLACK_GOLD.len() + 1,
        quadtone_hex.len() + 1,
        QUADTONE.len() + 1,
    );
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// The page rendered at one pixel per unit, and the interpretation's report alongside.
fn render(content: &str) -> pdf_render::Raster {
    let document = Document::open(fixture(content)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "{:?}",
        interpretation.unsupported
    );
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("a 100x100 target");
    render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the fixture rasterises")
}

/// The RGBA pixel at the centre of the 10-unit square whose lower-left corner is page `(x, 0)`.
fn square(raster: &pdf_render::Raster, x: u32) -> [u8; 4] {
    let (column, row) = (x + 5, 95);
    let at = usize::try_from((row * raster.width + column) * 4).expect("a small raster");
    let bytes = &raster.data[at..at + 4];
    [bytes[0], bytes[1], bytes[2], bytes[3]]
}

/// Paints index `index` of `space` in the square at `x`, and of `reference` in the one at `x + 10`.
fn pair(space: &str, reference: &str, index: u8, x: u32) -> String {
    format!(
        "/{space} cs {index} sc {x} 0 10 10 re f /{reference} cs {index} sc {} 0 10 10 re f ",
        x + 10
    )
}

/// Example 1: an index of the duotone is the cyan and black of its table entry, carried into
/// `DeviceCMYK` with zero magenta and yellow — the same colour a `DeviceCMYK` table states.
#[test]
fn example_1s_duotone_is_its_entrys_cyan_and_black_in_device_cmyk() {
    let mut content = String::new();
    for index in 0..5 {
        content.push_str(&pair(
            "Duotone",
            "DuotoneReference",
            index,
            u32::from(index) * 20,
        ));
    }
    let raster = render(&content);
    for index in 0..5u32 {
        let (drawn, reference) = (
            square(&raster, index * 20),
            square(&raster, index * 20 + 10),
        );
        assert_eq!(drawn, reference, "index {index}");
        assert_ne!(drawn, [255, 255, 255, 255], "index {index} painted nothing");
    }
}

/// Examples 3 and 4: with gold unavailable, the tint transformation returns the entry's last
/// three components as the `CalRGB` colour, and the two colourant tints are discarded.
#[test]
fn example_3s_black_and_gold_draw_as_the_calrgb_components_their_entry_carries() {
    let raster = render(&format!(
        "{}{}",
        pair("BlackGold", "CalReference", 0, 0),
        pair("BlackGold", "CalReference", 1, 20)
    ));
    for x in [0, 20] {
        assert_eq!(square(&raster, x), square(&raster, x + 10), "at {x}");
    }
    assert_ne!(
        square(&raster, 0),
        square(&raster, 20),
        "two entries, two colours"
    );
}

/// Example 5: the quadtone's seven components, of which the tint transformation keeps the last
/// three, read from a lookup table that is a stream.
#[test]
fn example_5s_quadtone_draws_as_the_last_three_components_of_its_entry() {
    let raster = render(&format!(
        "{}{}",
        pair("Quadtone", "QuadtoneReference", 0, 0),
        pair("Quadtone", "QuadtoneReference", 1, 20)
    ));
    for x in [0, 20] {
        assert_eq!(square(&raster, x), square(&raster, x + 10), "at {x}");
    }
    assert_ne!(
        square(&raster, 0),
        square(&raster, 20),
        "two entries, two colours"
    );
}
