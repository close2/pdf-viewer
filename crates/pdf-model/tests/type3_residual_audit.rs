//! Remaining independent PDF-input discriminators for ISO 32000-2 §9.6.4 and §9.6.5.3.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "these tests construct small, deliberately valid PDF fixtures"
)]

use std::fmt::Write as _;

use pdf_render::{Command, Paint};
use pdf_syntax::Document;

fn page(
    font_matrix: &str,
    glyph: &str,
    content: &str,
    font_entries: &str,
    encoding_entries: &str,
    glyph_stream_entries: &str,
    extra_objects: &str,
) -> pdf_model::Interpretation {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] \
         /Resources << /Font << /F3 5 0 R >> /ExtGState << /Half 8 0 R >> >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type3 /FontBBox [0 0 750 750] \
         /FontMatrix {font_matrix} /CharProcs << /square 7 0 R /triangle 7 0 R \
         /A 7 0 R /b 9 0 R /.notdef 9 0 R >> \
         /Encoding 6 0 R /FirstChar 97 /LastChar 98 /Widths [1000 1000] {font_entries} >>\nendobj\n\
         6 0 obj\n<< /Type /Encoding {encoding_entries} >>\nendobj\n\
         7 0 obj\n<< /Length {} {glyph_stream_entries} >>\nstream\n{glyph}\nendstream\nendobj\n\
         8 0 obj\n<< /Type /ExtGState /ca 0.5 /CA 0.5 >>\nendobj\n\
         9 0 obj\n<< /Length 27 >>\nstream\n1000 0 d0\n0 0 750 750 re f\nendstream\nendobj\n\
         {extra_objects}",
        content.len() + 1,
        glyph.len() + 1,
    );

    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let xref_at = bytes.len();
    let size = offsets.len() + 1;
    let _ = writeln!(bytes, "xref\n0 {size}");
    bytes.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );

    let document = Document::open(bytes.into_bytes()).expect("the PDF fixture is valid");
    assert!(!document.was_recovered(), "the fixture has a valid xref");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("page one exists");
    pdf_model::interpret(&document, &page)
}

fn fills(interpretation: &pdf_model::Interpretation) -> Vec<(pdf_render::Transform, [f32; 4])> {
    interpretation
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill {
                transform,
                paint: Paint::Solid(colour),
                ..
            } => Some((*transform, [colour.r, colour.g, colour.b, colour.a])),
            _ => None,
        })
        .collect()
}

/// Table 110's optional `/ToUnicode` `CMap` controls the text a PDF consumer reads for a shown
/// code, even when its Encoding glyph name has a different Adobe Glyph List meaning.
#[test]
fn type3_to_unicode_controls_the_pdf_text_readback() {
    let cmap = "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
        /CIDSystemInfo 3 dict dup begin /Registry (Adobe) def /Ordering (UCS) def \
        /Supplement 0 def end def\n/CMapName /Type3Residual def\n/CMapType 2 def\n\
        1 begincodespacerange\n<00> <FF>\nendcodespacerange\n\
        1 beginbfchar\n<61> <2603>\nendbfchar\nendcmap\n\
        CMapName currentdict /CMap defineresource pop\nend\nend";
    let extra = format!(
        "10 0 obj\n<< /Length {} >>\nstream\n{cmap}\nendstream\nendobj\n",
        cmap.len() + 1
    );
    let result = page(
        "[0.001 0 0 0.001 0 0]",
        "1000 0 0 0 750 750 d1\n0 0 750 750 re f",
        "BT /F3 10 Tf (a) Tj ET",
        "/ToUnicode 10 0 R",
        "/Differences [97 /A]",
        "",
        &extra,
    );
    assert!(result.is_complete(), "{:?}", result.unsupported);
    assert_eq!(result.text, "☃");
    assert_eq!(fills(&result).len(), 1, "the mapped glyph is painted");

    let without_to_unicode = page(
        "[0.001 0 0 0.001 0 0]",
        "1000 0 0 0 750 750 d1\n0 0 750 750 re f",
        "BT /F3 10 Tf (a) Tj ET",
        "",
        "/Differences [97 /A]",
        "",
        "",
    );
    assert!(
        without_to_unicode.is_complete(),
        "{:?}",
        without_to_unicode.unsupported
    );
    assert_eq!(without_to_unicode.text, "A");
    assert_eq!(fills(&without_to_unicode).len(), 1);
}

/// EC3 Issue #128 points to §7.8.3: the glyph stream dictionary is searched before the font
/// dictionary. The same `/X1` name selects blue from the stream and red from the font.
#[test]
fn glyph_stream_resources_win_a_name_collision_with_font_resources() {
    let form = |paint: &str| {
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 750 750] /Resources << >> /Length {} >>\nstream\n{paint}\nendstream",
            paint.len() + 1
        )
    };
    let objects = format!(
        "10 0 obj\n{}\nendobj\n11 0 obj\n{}\nendobj\n",
        form("1 0 0 rg 0 0 750 750 re f"),
        form("0 0 1 rg 0 0 750 750 re f"),
    );
    let result = page(
        "[0.001 0 0 0.001 0 0]",
        "1000 0 d0\n/X1 Do",
        "BT /F3 10 Tf (a) Tj ET",
        "/Resources << /XObject << /X1 10 0 R >> >>",
        "/Differences [97 /square]",
        "/Resources << /XObject << /X1 11 0 R >> >>",
        &objects,
    );
    assert!(result.is_complete(), "{:?}", result.unsupported);
    let rendered = fills(&result);
    assert_eq!(
        rendered.len(),
        1,
        "the glyph stream's /X1 form paints one fill"
    );
    assert_eq!(
        rendered[0].1.map(f32::to_bits),
        [0.0_f32, 0.0, 1.0, 1.0].map(f32::to_bits),
        "the glyph-stream form is blue; the font-level form is red"
    );
}

/// §9.6.5.3's NOTE excludes an implicit `/.notdef`: code 98 is not named by this Encoding,
/// although `/.notdef`, `/b`, `/triangle`, and a second Widths entry exist. An incorrect
/// default-name fallback would therefore visibly paint the fallback stream.
#[test]
fn an_encoding_omission_does_not_select_notdef_or_another_charproc() {
    let result = page(
        "[0.001 0 0 0.001 0 0]",
        "1000 0 0 0 750 750 d1\n0 0 750 750 re f",
        "BT /F3 10 Tf 0 0 Td (ba) Tj ET",
        "",
        "/Differences [97 /square]",
        "",
        "",
    );
    assert!(result.is_complete(), "{:?}", result.unsupported);
    let rendered = fills(&result);
    assert_eq!(rendered.len(), 1, "only code 97 has an Encoding name");
    assert!((rendered[0].0.e - 10.0).abs() < 1e-4);
}

/// §9.6.4 invokes the glyph under the CTM formed from `FontMatrix` and the text space active at
/// the show operator; the glyph's coordinates remain based at its own (0, 0) origin.
#[test]
fn glyph_ctm_uses_the_nonzero_text_origin() {
    let result = page(
        "[0.001 0 0 0.001 0 0]",
        "1000 0 0 0 750 750 d1\n0 0 750 750 re f",
        "BT /F3 10 Tf 25 40 Td (a) Tj ET",
        "",
        "/Differences [97 /square]",
        "",
        "",
    );
    let rendered = fills(&result);
    assert_eq!(rendered.len(), 1);
    assert!((rendered[0].0.e - 25.0).abs() < 1e-4);
    assert!((rendered[0].0.f - 40.0).abs() < 1e-4);
}

/// Table 111 discourages a producer from setting transparency in d1, then specifies the reader
/// behavior: ignore the color-related use and continue the glyph. The paired d0 control proves
/// `/Half` resolves and changes alpha when the glyph is allowed to set its own color state.
#[test]
fn d1_ignores_color_related_extgstate_changes_and_continues() {
    let result = page(
        "[0.001 0 0 0.001 0 0]",
        "1000 0 0 0 750 750 d1\n/Half gs\n0 0 750 750 re f",
        "BT /F3 10 Tf 0 0 Td (a) Tj ET",
        "",
        "/Differences [97 /square]",
        "",
        "",
    );
    assert!(result.is_complete(), "{:?}", result.unsupported);
    let rendered = fills(&result);
    assert_eq!(rendered.len(), 1, "processing resumes after the ignored gs");
    let coloured = page(
        "[0.001 0 0 0.001 0 0]",
        "1000 0 d0\n/Half gs\n0 0 750 750 re f",
        "BT /F3 10 Tf 0 0 Td (a) Tj ET",
        "",
        "/Differences [97 /square]",
        "",
        "",
    );
    assert!(coloured.is_complete(), "{:?}", coloured.unsupported);
    let coloured_fills = fills(&coloured);
    assert_eq!(coloured_fills.len(), 1);
    assert_eq!(
        coloured_fills[0].1[3].to_bits(),
        0.5_f32.to_bits(),
        "the same resolved /Half entry changes alpha in a d0 glyph"
    );
    assert_eq!(
        rendered[0].1[3].to_bits(),
        1.0_f32.to_bits(),
        "the attempted ca 0.5 is ignored"
    );
}
