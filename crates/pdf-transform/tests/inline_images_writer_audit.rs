//! Focused §8.9.7 public redaction-writer regression.

#![expect(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "the self-contained PDF fixture has fixed, checked object layouts"
)]

use std::fmt::Write as _;

use pdf_model::Pages;
use pdf_syntax::{Document, Limits, filter};
use pdf_transform::redact::RedactPlan;
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Source, apply};

fn source_fixture(samples: &[u8]) -> Vec<u8> {
    let mut content = b"q 100 0 0 100 50 50 cm\nBI /W 8 /H 8 /BPC 8 /CS /G ID\n".to_vec();
    content.extend_from_slice(samples);
    content.extend_from_slice(b"\nEI\nQ");

    let mut body = Vec::new();
    body.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    body.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    body.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n",
    );
    body.extend_from_slice(
        format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes(),
    );
    body.extend_from_slice(&content);
    body.extend_from_slice(b"\nendstream\nendobj\n");
    body.extend_from_slice(b"5 0 obj\n<< /Type /Annot /Subtype /Redact /Rect [50 50 100 150] /QuadPoints [50 150 100 150 100 50 50 50] >>\nendobj\n");

    // PDF 1.7 avoids a PDF 2.0 trailer ID; all five objects and the source stream length are exact.
    let mut source_bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    let mut remaining = body.as_slice();
    while let Some(end) = remaining
        .windows(7)
        .position(|window| window == b"endobj\n")
    {
        let stop = end + 7;
        offsets.push(source_bytes.len());
        source_bytes.extend_from_slice(&remaining[..stop]);
        remaining = &remaining[stop..];
    }
    assert!(
        remaining.is_empty(),
        "fixture body ends at an object boundary"
    );
    let xref_at = source_bytes.len();
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
    source_bytes.extend_from_slice(trailer.as_bytes());

    source_bytes
}

#[expect(clippy::expect_used, reason = "the valid fixture must reopen")]
fn assert_source_samples(source_bytes: &[u8], samples: &[u8]) {
    let source = Document::open(source_bytes.to_vec()).expect("valid source PDF opens");
    assert!(
        !source.was_recovered(),
        "source fixture has a complete xref"
    );
    let source_page = Pages::new(&source).get(0).expect("source page one");
    let source_content = source_page.content(&source);
    let source_bi = source_content
        .windows(2)
        .position(|window| window == b"BI")
        .expect("source inline image");
    let source_image = pdf_model::inline_image::scan(
        &source,
        &source_content,
        source_bi + 2,
        &source_page.resources,
        true,
    )
    .image
    .expect("source image is readable");
    assert_eq!(source_image.data.as_ref(), samples);
}

#[test]
fn redaction_writer_length_matches_independent_span_and_decoded_redaction_samples() {
    let mut samples = Vec::with_capacity(64);
    for row in 0..8u8 {
        for column in 0..8u8 {
            samples.push(column * 8 + row + 1);
        }
    }
    let source_bytes = source_fixture(&samples);
    assert_source_samples(&source_bytes, &samples);

    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Redact(RedactPlan {
            source: 0,
            names: "redacted.pdf".parse().expect("output name parses"),
            image_samples: None,
        }),
        &[Source::new(source_bytes)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("public redaction plan writes output");
    assert!(
        report.refused.is_empty(),
        "the valid inline image is spliced"
    );
    let mut outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1);
    let output = Document::open(outputs.remove(0).1).expect("redacted output opens");
    assert!(!output.was_recovered(), "writer output has a complete xref");
    let page = Pages::new(&output).get(0).expect("output page one");
    let rewritten = page.content(&output);
    let bi = rewritten
        .windows(2)
        .position(|window| window == b"BI")
        .expect("rewritten inline image remains in its content stream");
    // Independently measure bytes after ID through the writer's retained LF-before-EI
    // boundary. This deliberately does not use the inline-image scanner's /Length logic.
    let image_start = &rewritten[bi..];
    let id_rel = image_start
        .windows(3)
        .position(|window| window == b"ID\n")
        .expect("writer emits ID followed by its data separator");
    let id_end = bi + id_rel + 3;
    let ei_start = rewritten[id_end..]
        .windows(3)
        .rposition(|window| window == b"\nEI")
        .map(|offset| id_end + offset)
        .expect("writer emits an EI after the encoded image");
    let encoded = &rewritten[id_end..ei_start];
    assert!(
        !encoded.is_empty(),
        "Flate data is present between ID and EI"
    );

    let dictionary = &rewritten[bi..bi + id_rel];
    let length_key = dictionary
        .windows(7)
        .position(|window| window == b"/Length")
        .expect("redaction writer emits full /Length");
    let length_value = dictionary[length_key + 7..]
        .iter()
        .copied()
        .skip_while(u8::is_ascii_whitespace)
        .take_while(u8::is_ascii_digit)
        .collect::<Vec<_>>();
    let stated = std::str::from_utf8(&length_value)
        .expect("decimal Length is ASCII")
        .parse::<usize>()
        .expect("Length is an integer");
    assert_eq!(stated, encoded.len(), "Length counts encoded bytes only");
    assert!(
        dictionary
            .windows(b"/FlateDecode".len())
            .any(|window| window == b"/FlateDecode")
    );

    let decoded = filter::decode(b"FlateDecode", encoded, None, Limits::DEFAULT)
        .expect("writer output is valid Flate data");
    assert_eq!(decoded.len(), 64);
    let expected = (0..8u8)
        .flat_map(|row| {
            (0..8u8).map(move |column| if column < 4 { 0 } else { column * 8 + row + 1 })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        decoded.as_ref(),
        expected.as_slice(),
        "edited samples are zeroed while untouched samples survive"
    );
    assert!(
        rewritten[ei_start..].starts_with(b"\nEI\nQ"),
        "writer retains the EI delimiter and balanced graphics-state restore"
    );
}
