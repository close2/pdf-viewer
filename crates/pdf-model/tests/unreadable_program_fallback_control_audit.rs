//! Public integration test: an unreadable `FontFile` uses the same fallback layout as no `FontFile`,
//! while the page still reports that the embedded program was rejected. This is policy coverage
//! for ADR 2870/A837; it does not assert that ISO mandates a specific substitute face.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test fixture and command assertions should fail loudly"
)]

use pdf_render::{Command, Medium, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

fn stream(data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< /Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (n, body) in (1_usize..).zip(objects) {
        offsets.push(out.len());
        out.extend_from_slice(format!("{n} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    let size = offsets.len() + 1;
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    out
}

fn fixture(unreadable: bool) -> Vec<u8> {
    let content = b"BT /F1 36 Tf 10 25 Td (AB) Tj ET";
    let font = b"<< /Type /Font /Subtype /TrueType /BaseFont /Arial /FirstChar 65 /LastChar 66 \
       /Widths [667 667] /Encoding /WinAnsiEncoding /FontDescriptor 6 0 R >>"
        .to_vec();
    let descriptor = if unreadable {
        b"<< /Type /FontDescriptor /FontName /Arial /Flags 32 /FontBBox [-665 -325 2000 1006] \
           /ItalicAngle 0 /Ascent 905 /Descent -212 /CapHeight 716 /StemV 80 /FontFile2 7 0 R >>"
            .to_vec()
    } else {
        b"<< /Type /FontDescriptor /FontName /Arial /Flags 32 /FontBBox [-665 -325 2000 1006] \
           /ItalicAngle 0 /Ascent 905 /Descent -212 /CapHeight 716 /StemV 80 >>"
            .to_vec()
    };
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 80] \
           /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
        stream(content),
        font,
        descriptor,
    ];
    if unreadable {
        // Table125 requires Length1 for TrueType. Only the program bytes are malformed.
        let program = b"not a TrueType font";
        let mut encoded = format!(
            "<< /Length {} /Length1 {} >>\nstream\n",
            program.len(),
            program.len()
        )
        .into_bytes();
        encoded.extend_from_slice(program);
        encoded.extend_from_slice(b"\nendstream");
        objects.push(encoded);
    }
    assemble(&objects)
}

struct Rendered {
    pixels: Vec<u8>,
    text: String,
    origins: Vec<f32>,
    reports: String,
    complete: bool,
    marked_pixels: usize,
}

fn rendered(bytes: Vec<u8>) -> Rendered {
    let doc = Document::open(bytes).expect("fixture parses");
    assert!(!doc.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&doc).get(0).expect("one page");
    let result = pdf_model::interpret(&doc, &page);
    let list = &result.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 22).expect("bounded target");
    let raster = CpuRasterizer::new()
        .with_medium(Medium::NONE)
        .rasterize(list, target)
        .expect("page rasterizes");
    let origins = list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill { transform, .. } => Some(transform.e),
            _ => None,
        })
        .collect();
    let reports = format!("{:?}", result.unsupported);
    let complete = result.is_complete();
    let marked_pixels = raster
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel[3] != 0)
        .count();
    Rendered {
        pixels: raster.data,
        text: result.text,
        origins,
        reports,
        complete,
        marked_pixels,
    }
}

#[test]
fn unreadable_program_falls_back_like_absent_program_and_keeps_its_report() {
    let absent = rendered(fixture(false));
    let unreadable = rendered(fixture(true));

    assert!(
        absent.complete,
        "ordinary absent-program substitution is not a refusal"
    );
    assert_eq!(absent.reports, "[]", "control has no refusal");
    assert_eq!(absent.text.trim(), "AB");
    assert!(absent.marked_pixels > 0, "control produces visible ink");

    assert_eq!(
        unreadable.pixels, absent.pixels,
        "same host face and dictionary values yield same pixels"
    );
    assert_eq!(
        unreadable.text, absent.text,
        "both routes expose the same extracted text"
    );
    assert_eq!(
        unreadable.origins.len(),
        2,
        "each shown glyph has a public fill command"
    );
    assert_eq!(unreadable.origins.len(), absent.origins.len());
    assert!((unreadable.origins[0] - 10.0).abs() < 1e-5);
    assert!(
        (unreadable.origins[1] - 34.012).abs() < 1e-4,
        "10 + 667/1000 × 36 pt"
    );
    assert_eq!(
        unreadable.origins, absent.origins,
        "same stated advances place both glyphs"
    );
    assert!(
        unreadable.marked_pixels > 0,
        "fallback actually paints the page"
    );
    assert!(
        unreadable.reports.contains("could not be parsed"),
        "original failure remains: {}",
        unreadable.reports
    );
    assert!(
        unreadable
            .reports
            .contains("drawn from a substitute face instead"),
        "fallback is identified: {}",
        unreadable.reports
    );
    assert!(
        !unreadable.complete,
        "the fallback drawing remains reported as incomplete"
    );
}
