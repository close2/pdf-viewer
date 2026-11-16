//! Public-model coverage for §14.13.5 MCAF nested in a form, Type 3 glyph, or tiling
//! cell. The source lists all these as graphics-content sites. Every file uses a complete xref;
//! each target has a literal name, relationship, and stream distinct from the control.
#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::panic,
    reason = "standalone generated-fixture tests fail loudly when the fixture misses the feature"
)]
use pdf_model::attachment::Relationship;
use pdf_model::{Pages, interpret};
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use std::fmt::Write as _;

// Helvetica.afm gives x and y widths 500; explicit metrics keep these PDF 2.0
// fixtures valid without the older Standard14 omission exception.
fn pdf(objects: &[&str]) -> Document {
    let mut b = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (i, obj) in objects.iter().enumerate() {
        offsets.push(b.len());
        let _ = writeln!(b, "{} 0 obj\n{obj}\nendobj", i + 1);
    }
    let x = b.len();
    let _ = writeln!(b, "xref\n0 {}\n0000000000 65535 f ", objects.len() + 1);
    for o in offsets {
        let _ = writeln!(b, "{o:010} 00000 n ");
    }
    let _ = write!(
        b,
        "trailer\n<< /Size {} /Root 1 0 R /ID [<00112233445566778899AABBCCDDEEFF> <00112233445566778899AABBCCDDEEFF>] >>\nstartxref\n{x}\n%%EOF\n",
        objects.len() + 1
    );
    let d = Document::open(b.into_bytes()).expect("exact xref PDF");
    assert!(!d.was_recovered());
    d
}
fn assert_one(
    d: &Document,
    expected: &str,
    description: &str,
    relation: &Relationship,
    expected_text: &str,
    expected_payload: &[u8],
) {
    let page = Pages::new(d).get(0).expect("fixture contains page 0");
    let out = interpret(d, &page);
    assert!(
        out.is_complete(),
        "complete exact-xref interpretation: {out:?}"
    );
    assert!(
        out.text.contains(expected_text),
        "independent ordinary/nested text control: {:?}",
        out.text
    );
    let [(range, file)] = out.associated_files.as_slice() else {
        panic!("one MCAF association: {:?}", out.associated_files)
    };
    assert_eq!(file.file_name.as_deref(), Some(expected));
    assert_eq!(file.description.as_deref(), Some(description));
    assert_eq!(&file.relationship, relation);
    let payload = d
        .decoded_stream_data(&file.stream)
        .expect("associated payload decodes");
    assert_eq!(payload.as_ref(), expected_payload);
    assert!(range.end <= out.text.len());
}
#[test]
fn mcaf_inside_form_is_returned_by_the_enclosing_page_interpretation() {
    let content = "/Span /NoAF BDC BT /H 12 Tf 10 50 Td (y) Tj ET EMC /Fx Do";
    let form = "/AF /P BDC BT /H 10 Tf 0 0 Td (x) Tj ET EMC";
    let d = pdf(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Properties << /NoAF << /MCAF [7 0 R] >> >> /Font << /H << /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding /FirstChar 120 /LastChar 121 /Widths [500 500] /FontDescriptor 10 0 R >> >> /XObject << /Fx 5 0 R >> >> /Contents 4 0 R >>",
        &format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ),
        &format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 40 20] /Resources << /Properties << /P << /MCAF [6 0 R] >> >> /Font << /H << /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding /FirstChar 120 /LastChar 121 /Widths [500 500] /FontDescriptor 10 0 R >> >> >> /Length {} >>\nstream\n{form}\nendstream",
            form.len()
        ),
        "<< /Type /Filespec /F (form-source.xml) /UF (form-source.xml) /Desc (form source data) /AFRelationship /Source /EF << /F 8 0 R /UF 8 0 R >> >>",
        "<< /Type /Filespec /F (decoy.bin) /UF (decoy.bin) /AFRelationship /Data /EF << /F 9 0 R /UF 9 0 R >> >>",
        "<< /Type /EmbeddedFile /Subtype /application#2Fxml /Params << /ModDate (D:20240101000000Z) >> /Length 4 >>\nstream\nform\nendstream",
        "<< /Type /EmbeddedFile /Subtype /application#2Foctet-stream /Params << /ModDate (D:20240101000000Z) >> /Length 4 >>\nstream\nfake\nendstream",
        "<< /Type /FontDescriptor /FontName /Helvetica /Flags 32 /FontBBox [-166 -225 1000 931] /ItalicAngle 0 /Ascent 718 /Descent -207 /CapHeight 523 /StemV 88 >>",
    ]);
    assert_one(
        &d,
        "form-source.xml",
        "form source data",
        &Relationship::Source,
        "y",
        b"form",
    );
    let page = Pages::new(&d).get(0).expect("fixture contains page 0");
    let out = interpret(&d, &page);
    assert!(
        out.is_complete(),
        "complete exact-xref interpretation: {out:?}"
    );
    let [(range, _)] = out.associated_files.as_slice() else {
        panic!("one MCAF association: {:?}", out.associated_files)
    };
    assert_eq!(out.text.get(range.clone()).map(str::trim), Some("x"));
}

#[test]
fn mcaf_inside_type3_glyph_description_reaches_the_page_report() {
    let content =
        "BT /T3 20 Tf 10 40 Td (a) Tj ET /Span /NoAF BDC BT /H 12 Tf 10 10 Td (y) Tj ET EMC";
    let glyph = "500 0 0 0 500 700 d1 /AF /P BDC BT /H 100 Tf 0 250 Td (x) Tj ET EMC";
    let d = pdf(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Font << /H << /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding /FirstChar 120 /LastChar 121 /Widths [500 500] /FontDescriptor 11 0 R >> /T3 6 0 R >> /Properties << /P << /MCAF [5 0 R] >> /NoAF << /MCAF [7 0 R] >> >> >> /Contents 4 0 R >>",
        &format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ),
        "<< /Type /Filespec /F (glyph-equation.xml) /UF (glyph-equation.xml) /Desc (glyph equation source) /AFRelationship /Supplement /EF << /F 9 0 R /UF 9 0 R >> >>",
        "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] /FontMatrix [0.001 0 0 0.001 0 0] /FirstChar 97 /LastChar 97 /Widths [500] /Encoding << /Differences [97 /a] >> /CharProcs << /a 8 0 R >> >>",
        "<< /Type /Filespec /F (decoy.bin) /UF (decoy.bin) /AFRelationship /Data /EF << /F 10 0 R /UF 10 0 R >> >>",
        &format!("<< /Length {} >>\nstream\n{glyph}\nendstream", glyph.len()),
        "<< /Type /EmbeddedFile /Subtype /application#2Fmathml+xml /Params << /ModDate (D:20240101000000Z) >> /Length 7 >>\nstream\n<math/>\nendstream",
        "<< /Type /EmbeddedFile /Subtype /application#2Foctet-stream /Params << /ModDate (D:20240101000000Z) >> /Length 4 >>\nstream\nfake\nendstream",
        "<< /Type /FontDescriptor /FontName /Helvetica /Flags 32 /FontBBox [-166 -225 1000 931] /ItalicAngle 0 /Ascent 718 /Descent -207 /CapHeight 523 /StemV 88 >>",
    ]);
    assert_one(
        &d,
        "glyph-equation.xml",
        "glyph equation source",
        &Relationship::Supplement,
        "y",
        b"<math/>",
    );
    let out = interpret(&d, &Pages::new(&d).get(0).expect("fixture contains page 0"));
    assert!(
        out.is_complete(),
        "complete exact-xref interpretation: {out:?}"
    );
    let [(range, _)] = out.associated_files.as_slice() else {
        panic!("one MCAF association: {:?}", out.associated_files)
    };
    // Nested glyph-description text paints the glyph but is deliberately not extracted
    // as a second character. The association is still reported with an empty text range.
    assert!(range.is_empty());
    assert!(
        out.text.contains('a'),
        "the enclosing Type3 character is extracted"
    );
    assert!(
        !out.text.contains('x'),
        "glyph construction does not duplicate text"
    );
}

#[test]
fn mcaf_inside_tiling_cell_is_reported_alongside_unmarked_page_text() {
    let content =
        "/Span /NoAF BDC BT /H 12 Tf 10 80 Td (y) Tj ET EMC /Pat cs /Tile scn 0 0 80 60 re f";
    let cell = "/AF /P BDC 1 0 0 rg 0 0 10 10 re f EMC";
    let embedded = "a,b\n1,2";
    let d = pdf(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Font << /H << /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding /FirstChar 120 /LastChar 121 /Widths [500 500] /FontDescriptor 10 0 R >> >> /ColorSpace << /Pat /Pattern >> /Pattern << /Tile 5 0 R >> /Properties << /NoAF << /MCAF [7 0 R] >> >> >> /Contents 4 0 R >>",
        &format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ),
        &format!(
            "<< /Type /Pattern /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 10 10] /XStep 10 /YStep 10 /Resources << /Properties << /P << /MCAF [6 0 R] >> >> >> /Length {} >>\nstream\n{cell}\nendstream",
            cell.len()
        ),
        "<< /Type /Filespec /F (tile-source.csv) /UF (tile-source.csv) /Desc (tile source data) /AFRelationship /Data /EF << /F 8 0 R /UF 8 0 R >> >>",
        "<< /Type /Filespec /F (decoy.bin) /UF (decoy.bin) /AFRelationship /Source /EF << /F 9 0 R /UF 9 0 R >> >>",
        &format!(
            "<< /Type /EmbeddedFile /Subtype /text#2Fcsv /Params << /ModDate (D:20240101000000Z) >> /Length {} >>\nstream\n{}\nendstream",
            embedded.len(),
            embedded
        ),
        "<< /Type /EmbeddedFile /Subtype /application#2Foctet-stream /Params << /ModDate (D:20240101000000Z) >> /Length 4 >>\nstream\nfake\nendstream",
        "<< /Type /FontDescriptor /FontName /Helvetica /Flags 32 /FontBBox [-166 -225 1000 931] /ItalicAngle 0 /Ascent 718 /Descent -207 /CapHeight 523 /StemV 88 >>",
    ]);
    assert_one(
        &d,
        "tile-source.csv",
        "tile source data",
        &Relationship::Data,
        "y",
        b"a,b\n1,2",
    );
    let out = interpret(&d, &Pages::new(&d).get(0).expect("fixture contains page 0"));
    assert!(
        out.is_complete(),
        "complete exact-xref interpretation: {out:?}"
    );
    let [(range, _)] = out.associated_files.as_slice() else {
        panic!("one MCAF association: {:?}", out.associated_files)
    };
    assert!(
        range.is_empty(),
        "the cell paints geometry but emits no text"
    );
    let target = TargetSpec::for_page(&out.display_list, 1.0, 1 << 20).expect("small target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&out.display_list, target)
        .expect("valid pattern paints");
    // PDF (5,5) is inside the red patterned rectangle, clear of the ordinary text.
    let at = 94usize * 100 * 4 + 5 * 4;
    assert_eq!(
        raster.data.get(at..at + 4),
        Some([255, 0, 0, 255].as_slice())
    );
}
