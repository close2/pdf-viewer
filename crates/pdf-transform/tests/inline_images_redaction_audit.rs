//! §8.9.7 redaction preserves direct Indexed dictionaries and stencil semantics.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "complete fixtures must open and bounded sample grids must decode"
)]
use pdf_syntax::Document;
use pdf_transform::redact::RedactPlan;
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Source, apply};
use std::fmt::Write as _;

fn redaction_fixture(image: &[u8]) -> Vec<u8> {
    let mut content = b"q 100 0 0 100 50 50 cm\n".to_vec();
    content.extend_from_slice(image);
    content.extend_from_slice(b"\nQ");
    let mut body = Vec::new();
    body.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    body.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    body.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n");
    body.extend_from_slice(
        format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes(),
    );
    body.extend_from_slice(&content);
    body.extend_from_slice(b"\nendstream\nendobj\n");
    body.extend_from_slice(b"5 0 obj\n<< /Type /Annot /Subtype /Redact /Rect [50 50 100 150] /QuadPoints [50 150 100 150 100 50 50 50] >>\nendobj\n");

    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    let mut rest = body.as_slice();
    while let Some(end) = rest.windows(7).position(|window| window == b"endobj\n") {
        let stop = end + 7;
        offsets.push(bytes.len());
        bytes.extend_from_slice(&rest[..stop]);
        rest = &rest[stop..];
    }
    assert!(rest.is_empty(), "fixture body ends at an object boundary");
    let xref_at = bytes.len();
    let size = offsets.len() + 1;
    let mut trailer = String::new();
    let _ = writeln!(trailer, "xref\n0 {size}");
    trailer.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    bytes.extend_from_slice(trailer.as_bytes());
    bytes
}

fn redact(bytes: Vec<u8>) -> (pdf_transform::Report, Document) {
    let source = Document::open(bytes.clone()).expect("complete redaction source opens");
    assert!(
        !source.was_recovered(),
        "source fixture has a complete xref"
    );
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Redact(RedactPlan {
            source: 0,
            names: "redacted.pdf".parse().expect("output name parses"),
            image_samples: None,
        }),
        &[Source::new(bytes)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("public redaction plan runs");
    let mut outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1);
    let output = Document::open(outputs.remove(0).1).expect("redaction output opens");
    assert!(!output.was_recovered(), "writer emits a complete xref");
    (report, output)
}

#[test]
fn redaction_preserves_a_direct_indexed_inline_color_space_array() {
    let samples: Vec<u8> = (0..8u8).flat_map(|_| [0, 1, 0, 1, 0, 1, 0, 1]).collect();
    let mut image = b"BI /W 8 /H 8 /BPC 8 /CS [/I /RGB 1 <ff000000ff00>] ID\n".to_vec();
    image.extend_from_slice(&samples);
    image.extend_from_slice(b"\nEI");
    let (report, output) = redact(redaction_fixture(&image));
    assert!(
        report.refused.is_empty(),
        "direct Indexed array is spliced: {:?}",
        report.refused
    );
    let page = pdf_model::Pages::new(&output).get(0).expect("output page");
    let content = page.content(&output);
    let bi = content
        .windows(2)
        .position(|window| window == b"BI")
        .expect("rewritten BI");
    let image = pdf_model::inline_image::scan(&output, &content, bi + 2, &page.resources, true)
        .image
        .expect("rewritten Indexed inline image scans");
    let space = image
        .dict
        .get("ColorSpace")
        .and_then(pdf_syntax::Object::as_array)
        .expect("direct color array");
    assert_eq!(
        space
            .first()
            .and_then(pdf_syntax::Object::as_name)
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"Indexed"[..])
    );
    assert_eq!(
        space
            .get(1)
            .and_then(pdf_syntax::Object::as_name)
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"DeviceRGB"[..])
    );
    assert_eq!(
        space.get(2).and_then(pdf_syntax::Object::as_integer),
        Some(1)
    );
    assert!(
        matches!(space.get(3), Some(pdf_syntax::Object::String(bytes)) if bytes.as_ref() == [255, 0, 0, 0, 255, 0])
    );
    let samples = output
        .image_stream(&image)
        .expect("writer output samples decode")
        .data;
    assert_eq!(samples.len(), 64);
    for row in 0..8usize {
        for column in 0..8usize {
            let expected = if column < 4 {
                0
            } else {
                u8::try_from(column % 2).expect("small index")
            };
            assert_eq!(
                samples[row * 8 + column],
                expected,
                "row {row}, column {column}"
            );
        }
    }
}

#[test]
fn redaction_preserves_inline_image_mask_without_a_color_space() {
    let mut image = b"BI /W 8 /H 8 /BPC 1 /IM true ID\n".to_vec();
    image.extend_from_slice(&[0xaa; 8]);
    image.extend_from_slice(b"\nEI");
    let (report, output) = redact(redaction_fixture(&image));
    assert!(
        report.refused.is_empty(),
        "inline ImageMask is spliced: {:?}",
        report.refused
    );
    let page = pdf_model::Pages::new(&output).get(0).expect("output page");
    let content = page.content(&output);
    let bi = content
        .windows(2)
        .position(|window| window == b"BI")
        .expect("rewritten BI");
    let image = pdf_model::inline_image::scan(&output, &content, bi + 2, &page.resources, true)
        .image
        .expect("rewritten mask scans");
    assert!(matches!(
        image.dict.get("ImageMask"),
        Some(pdf_syntax::Object::Boolean(true))
    ));
    assert!(
        image.dict.get("ColorSpace").is_none(),
        "mask still has no ColorSpace"
    );
    assert_eq!(
        image
            .dict
            .get("BitsPerComponent")
            .and_then(pdf_syntax::Object::as_integer),
        Some(1)
    );
    let samples = output
        .image_stream(&image)
        .expect("writer output samples decode")
        .data;
    assert_eq!(
        samples.as_ref(),
        &[0x0a; 8],
        "the left four redacted mask bits are zeroed"
    );
}
