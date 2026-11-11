//! §9.3 page defaults, vertical text parameters, and hidden optional-content clipping.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test fixtures are valid constructed PDFs and all expected positions are bounded"
)]

use pdf_render::{Command, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const PROGRAM: &[u8] = include_bytes!("../../../data/standard-fonts/LiberationSans-Regular.ttf");
const PIXEL_BUDGET: u64 = 1 << 20;

fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

fn stream(data: &[u8], extra: &str) -> Vec<u8> {
    let mut out = format!("<< /Length {} {extra} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

fn simple_page(content: &str) -> Vec<u8> {
    let content = format!("{content}\n");
    assemble(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_vec(),
        stream(content.as_bytes(), ""),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ])
}

fn two_page_state_pdf() -> Vec<u8> {
    let first = b"BT 20 Tc 40 Tw 50 Tz 12 TL 9 Ts 3 Tr /F1 10 Tf 10 50 Td ( A) Tj T* (A) Tj ET\n";
    let second = b"BT /F1 10 Tf 10 50 Td ( A) Tj T* (A) Tj ET\n";
    assemble(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Font << /F1 7 0 R >> >> /Contents 5 0 R >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Font << /F1 7 0 R >> >> /Contents 6 0 R >>".to_vec(),
        stream(first, ""),
        stream(second, ""),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ])
}

fn interpretation_in(document: &Document, page_number: usize) -> pdf_model::Interpretation {
    let page = pdf_model::Pages::new(document)
        .get(page_number)
        .expect("fixture page exists");
    let result = pdf_model::interpret(document, &page);
    assert!(
        result.is_complete(),
        "fixture features are supported: {:?}",
        result.unsupported
    );
    result
}

fn interpretation(bytes: Vec<u8>, page_number: usize) -> pdf_model::Interpretation {
    let document = Document::open(bytes).expect("valid PDF fixture");
    assert!(!document.was_recovered(), "fixture has a complete xref");
    interpretation_in(&document, page_number)
}

fn transforms(interpretation: &pdf_model::Interpretation) -> Vec<(f32, f32, f32, f32)> {
    interpretation
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill { transform, .. } => {
                Some((transform.e, transform.f, transform.a, transform.d))
            }
            _ => None,
        })
        .collect()
}

/// Page one changes every operator-settable default before page two. Page two must still begin
/// with Tc=Tw=TL=Ts=0, Tz=100%, and Tr=0. Explicit-default control is the same text stream in
/// a separate valid one-page PDF; both placement and command-kind behavior are checked. Both
/// pages are interpreted from the same parsed Document, first page one and then page two.
#[test]
fn text_state_defaults_are_restored_at_each_page() {
    let document = Document::open(two_page_state_pdf()).expect("valid two-page PDF fixture");
    assert!(
        !document.was_recovered(),
        "two-page fixture has a complete xref"
    );
    let page_one = interpretation_in(&document, 0);
    let page_two = interpretation_in(&document, 1);
    let explicit = interpretation(
        simple_page("0 Tc 0 Tw 100 Tz 0 TL 0 Ts 0 Tr BT /F1 10 Tf 10 50 Td ( A) Tj T* (A) Tj ET"),
        0,
    );
    assert!(
        page_two
            .display_list
            .commands()
            .iter()
            .all(|command| matches!(command, Command::Fill { .. })),
        "default rendering mode fills without stroking or clipping"
    );
    let actual = transforms(&page_two);
    let control = transforms(&explicit);
    assert_eq!(actual.len(), control.len());
    for (actual, control) in actual.iter().zip(&control) {
        assert!(
            (actual.0 - control.0).abs() < 0.001
                && (actual.1 - control.1).abs() < 0.001
                && (actual.2 - control.2).abs() < 0.001
                && (actual.3 - control.3).abs() < 0.001,
            "page two must behave exactly as explicit initial defaults: {actual:?} vs {control:?}"
        );
    }
    assert_eq!(actual.len(), 2, "default Tr=0 paints both A glyphs");
    assert!(
        (actual[0].0 - 12.78).abs() < 0.01,
        "default Tc/Tw leave A at x=12.78: {actual:?}"
    );
    assert!(
        (actual[0].1 - 50.0).abs() < 0.01 && (actual[1].1 - 50.0).abs() < 0.01,
        "default Ts=0 and TL=0 leave both baselines at y=50: {actual:?}"
    );
    let prior_page = transforms(&page_one);
    assert!(
        prior_page.is_empty(),
        "page one's explicit Tr=3 makes text invisible"
    );
    let prior_a: Vec<_> = page_one
        .text_layer
        .iter()
        .filter_map(|placed| {
            (page_one.text.get(placed.span.clone()) == Some("A")).then_some(placed.quad)
        })
        .collect();
    let default_a: Vec<_> = page_two
        .text_layer
        .iter()
        .filter_map(|placed| {
            (page_two.text.get(placed.span.clone()) == Some("A")).then_some(placed.quad)
        })
        .collect();
    assert_eq!(
        prior_a.len(),
        2,
        "page one executes its parameter-setting show operations"
    );
    assert_eq!(default_a.len(), 2);
    let center = |quad: &[f32; 8]| {
        (
            (quad[0] + quad[2] + quad[4] + quad[6]) / 4.0,
            (quad[1] + quad[3] + quad[5] + quad[7]) / 4.0,
        )
    };
    let prior_first = center(&prior_a[0]);
    let prior_second = center(&prior_a[1]);
    let default_first = center(&default_a[0]);
    let default_second = center(&default_a[1]);
    assert!(
        prior_first.0 > default_first.0 + 20.0,
        "page one control reflects its large Tc/Tw values: {prior_first:?} vs {default_first:?}"
    );
    assert!(
        (prior_first.1 - default_first.1 - 9.0).abs() < 0.02,
        "page one Ts=9 raises its first A relative to the default page: {prior_first:?} vs {default_first:?}"
    );
    assert!(
        (prior_second.1 - default_second.1 + 3.0).abs() < 0.02,
        "page one's Ts=9 and TL=12 put its second A 3 units below the default page: {prior_second:?} vs {default_second:?}"
    );
}

fn vertical_fixture(content: &str) -> Vec<u8> {
    let encoding = b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n/CMapName /AuditV def\n/CMapType 1 def\n/WMode 1 def\n1 begincodespacerange\n<20> <7E>\nendcodespacerange\n2 begincidchar\n<20> 3\n<41> 36\nendcidchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend";
    let unicode = b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n/CMapName /AuditU def\n/CMapType 2 def\n1 begincodespacerange\n<20> <7E>\nendcodespacerange\n2 beginbfchar\n<20> <0020>\n<41> <0041>\nendbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend";
    let content = format!("{content}\n");
    let mut font_program = format!(
        "<< /Length {} /Length1 {} >>\nstream\n",
        PROGRAM.len(),
        PROGRAM.len()
    )
    .into_bytes();
    font_program.extend_from_slice(PROGRAM);
    font_program.extend_from_slice(b"\nendstream");
    assemble(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_vec(),
        stream(content.as_bytes(), ""),
        b"<< /Type /Font /Subtype /Type0 /BaseFont /LiberationSans /Encoding 8 0 R /ToUnicode 9 0 R /DescendantFonts [6 0 R] >>".to_vec(),
        b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /LiberationSans /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 7 0 R /CIDToGIDMap /Identity /DW 1000 /W [3 [250] 36 [700]] /DW2 [880 -1000] /W2 [3 [-1000 125 880] 36 [-1000 350 880]] >>".to_vec(),
        b"<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 /FontBBox [-203 -303 1050 910] /ItalicAngle 0 /Ascent 905 /Descent -212 /CapHeight 729 /StemV 80 /FontFile2 10 0 R >>".to_vec(),
        stream(encoding, "/Type /CMap /CMapName /AuditV /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /WMode 1"),
        stream(unicode, "/Type /CMap /CMapName /AuditU /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >>"),
        font_program,
    ])
}

fn quad_for_a(interpretation: &pdf_model::Interpretation) -> [f32; 8] {
    interpretation
        .text_layer
        .iter()
        .find_map(|placed| {
            (interpretation.text.get(placed.span.clone()) == Some("A")).then_some(placed.quad)
        })
        .expect("the CMap's A code produces a text-layer glyph")
}

fn bounds(quad: &[f32; 8]) -> (f32, f32, f32, f32) {
    let xs = [quad[0], quad[2], quad[4], quad[6]];
    let ys = [quad[1], quad[3], quad[5], quad[7]];
    (
        xs.iter().copied().fold(f32::INFINITY, f32::min),
        ys.iter().copied().fold(f32::INFINITY, f32::min),
        xs.iter().copied().fold(f32::NEG_INFINITY, f32::max),
        ys.iter().copied().fold(f32::NEG_INFINITY, f32::max),
    )
}

/// In vertical writing Tc and the single-byte code-32 Tw are added to y displacement;
/// Tz changes glyph width but not that vertical advance; Ts moves the baseline in y.
#[test]
fn vertical_spacing_rise_and_horizontal_scale_keep_distinct_axes() {
    let base = interpretation(vertical_fixture("BT /F1 10 Tf 20 50 Td ( A) Tj ET"), 0);
    let tc = interpretation(
        vertical_fixture("BT /F1 10 Tf 0 Tc 20 50 Td 3 Tc ( A) Tj ET"),
        0,
    );
    let tw = interpretation(vertical_fixture("BT /F1 10 Tf 20 50 Td 4 Tw ( A) Tj ET"), 0);
    let half = interpretation(
        vertical_fixture("BT /F1 10 Tf 20 50 Td 3 Tc 4 Tw 50 Tz ( A) Tj ET"),
        0,
    );
    let risen = interpretation(vertical_fixture("BT /F1 10 Tf 20 50 Td 5 Ts ( A) Tj ET"), 0);
    let (bx0, by0, bx1, by1) = bounds(&quad_for_a(&base));
    let (_, character_bottom, _, character_top) = bounds(&quad_for_a(&tc));
    let (_, word_bottom, _, word_top) = bounds(&quad_for_a(&tw));
    let (hx0, hy0, hx1, hy1) = bounds(&quad_for_a(&half));
    let (_, ry0, _, ry1) = bounds(&quad_for_a(&risen));
    assert!(
        (((character_bottom + character_top) - (by0 + by1)) / 2.0 - 3.0).abs() < 0.02,
        "vertical Tc=3 raises the later glyph's text position by 3: {by0}..{by1} vs {character_bottom}..{character_top}"
    );
    assert!(
        (((word_bottom + word_top) - (by0 + by1)) / 2.0 - 4.0).abs() < 0.02,
        "one-byte code 32 receives vertical Tw=4: {by0}..{by1} vs {word_bottom}..{word_top}"
    );
    assert!(
        (((hy0 + hy1) - (by0 + by1)) / 2.0 - 7.0).abs() < 0.02,
        "Tz=50 does not scale vertical Tc+Tw displacement: {by0}..{by1} vs {hy0}..{hy1}"
    );
    assert!(
        ((hx1 - hx0) - (bx1 - bx0) / 2.0).abs() < 0.03,
        "Tz=50 halves horizontal extent in vertical writing mode"
    );
    assert!(
        (((ry0 + ry1) - (by0 + by1)) / 2.0 - 5.0).abs() < 0.02,
        "Ts=5 raises baseline five units independent of writing direction"
    );
}

fn optional_content_pdf(visible: bool, mode: u8) -> Vec<u8> {
    let off = if visible { "" } else { "/OFF [5 0 R]" };
    let catalog = format!(
        "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [5 0 R] /D << {off} >> >> >>"
    );
    let content =
        format!("/OC /oc BDC BT /F1 48 Tf {mode} Tr 10 20 Td (A) Tj ET EMC 0 0 80 80 re f\n");
    assemble(&[
        catalog.into_bytes(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Font << /F1 6 0 R >> /Properties << /oc 5 0 R >> >> /Contents 4 0 R >>".to_vec(),
        stream(content.as_bytes(), ""),
        b"<< /Type /OCG /Name (Layer) >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ])
}

fn raster(bytes: Vec<u8>) -> pdf_render::Raster {
    let result = interpretation(bytes, 0);
    assert!(
        result.is_complete(),
        "text clipping fixture is well-formed: {:?}",
        result.unsupported
    );
    let list = result.display_list;
    let target = TargetSpec::for_page(&list, 1.0, PIXEL_BUDGET).expect("raster target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("rasterize")
}

/// A hidden OCG suppresses glyph marks while text clipping survives ET. Mode 7 has no text
/// paint, so visible and hidden configurations should leave the later rectangle pixel-identical;
/// a missing clip in the hidden case makes it a full rectangle instead.
#[test]
fn hidden_optional_content_still_contributes_text_clip() {
    // Prove that the configurations really differ in visibility before comparing clips.
    let visible_paint = interpretation(optional_content_pdf(true, 0), 0);
    let hidden_paint = interpretation(optional_content_pdf(false, 0), 0);
    assert_eq!(transforms(&visible_paint).len(), 2, "glyph plus rectangle");
    assert_eq!(
        transforms(&hidden_paint).len(),
        1,
        "hidden glyph; visible rectangle"
    );
    let no_clip = raster(optional_content_pdf(false, 3));
    assert_eq!(
        no_clip.data.chunks_exact(4).filter(|px| px[3] > 0).count(),
        80 * 80
    );
    let visible = raster(optional_content_pdf(true, 7));
    let hidden = raster(optional_content_pdf(false, 7));
    assert_eq!(
        hidden.data, visible.data,
        "OCG visibility must not remove the text clip"
    );
    let opaque = hidden.data.chunks_exact(4).filter(|px| px[3] > 0).count();
    assert!(opaque > 0, "A's clipping path admits some rectangle pixels");
    assert!(
        opaque < 80 * 80,
        "A's clipping path excludes the rectangle outside its outline; found {opaque}"
    );
}
