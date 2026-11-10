//! Selected Projection appearance and Redact overlay-reporting consumer controls.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "each complete fixture and observable is required for the audit assertion"
)]

use std::fmt::Write as _;

use pdf_model::Pages;
use pdf_syntax::Document;
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Source, apply};

fn assemble(objects: &[String], version: &str) -> Vec<u8> {
    let mut out = format!("%PDF-{version}\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{object}\nendobj", index + 1);
    }
    let xref_at = out.len();
    let size = objects.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn stream_object(dictionary: &str, data: &str) -> String {
    format!(
        "{dictionary}\nstream\n{data}\nendstream",
        dictionary = dictionary.replace("/Length __LENGTH__", &format!("/Length {}", data.len())),
        data = data
    )
}

fn projection_pdf(with_appearance: bool) -> Vec<u8> {
    let appearance = if with_appearance {
        " /AP << /N 6 0 R >>"
    } else {
        ""
    };
    assemble(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
         /Contents 4 0 R /Annots [5 0 R] >>"
                .to_owned(),
            stream_object("<< /Length __LENGTH__ >>", ""),
            format!("<< /Type /Annot /Subtype /Projection /Rect [20 20 60 60] /F 4{appearance} >>"),
            stream_object(
                "<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources << >> /Length __LENGTH__ >>",
                "1 0 0 rg 0 0 10 10 re f",
            ),
        ],
        "2.0",
    )
}

#[test]
fn projection_without_runtime_has_no_constructed_mark_but_keeps_a_stored_appearance() {
    // A64's clause-13 boundary means this reader has no activated 3D measurement runtime.
    // EC3 Issue #94 adds that an AP dictionary is not required in this context; the original
    // zero-Rect prohibition was struck out by Issue #42 (both on physical p. 520 / printed 505).
    let without = projection_pdf(false);
    let parsed = Document::open(without).expect("complete projection fixture");
    assert!(!parsed.was_recovered(), "the xref offsets are exact");
    let page = Pages::new(&parsed).get(0).expect("one page");
    let absent = pdf_model::interpret(&parsed, &page);
    assert!(absent.display_list.commands().is_empty());
    assert_eq!(pixel(&absent, 40, 40), [0, 0, 0, 0]);
    assert!(
        absent.is_complete(),
        "there is no default projection artwork or 3D runtime to render: {:?}",
        absent.unsupported
    );

    // Positive control: Projection is not blanket-dropped; a producer's stored /AP goes through
    // the common §12.5.5 path even though this reader does not render the 3D runtime itself.
    let with = projection_pdf(true);
    let parsed = Document::open(with).expect("complete projection with appearance");
    assert!(!parsed.was_recovered(), "the xref offsets are exact");
    let page = Pages::new(&parsed).get(0).expect("one page");
    let stated = pdf_model::interpret(&parsed, &page);
    assert!(stated.is_complete(), "{:?}", stated.unsupported);
    assert_eq!(pixel(&stated, 40, 40), [255, 0, 0, 255]);
    assert!(
        !stated.display_list.commands().is_empty(),
        "the stored normal appearance remains drawable"
    );
}

fn redaction_pdf(with_ro: bool) -> Vec<u8> {
    let content = "BT /F1 12 Tf 20 150 Td (KEEP) Tj ET BT /F1 12 Tf 20 50 Td (SECRET) Tj ET";
    let length = content.len();
    let ro_entry = if with_ro { " /RO 7 0 R" } else { "" };
    let objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /Font << /F1 5 0 R >> >> \
         /Contents 4 0 R /Annots [6 0 R] >>"
            .to_owned(),
        stream_object(&format!("<< /Length {length} >>"), content),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Redact /Rect [10 40 130 66] /F 4 /AP << /N 8 0 R >>{ro_entry} >>"
        ),
        stream_object(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources << >> /Length __LENGTH__ >>",
            "0 1 0 rg 0 0 10 10 re f",
        ),
        stream_object(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 120 26] /Resources << >> /Length __LENGTH__ >>",
            "1 0 0 RG 1 w 1 1 118 24 re S",
        ),
    ];
    assemble(&objects, "1.7")
}

fn apply_redaction(bytes: &[u8]) -> (pdf_transform::Report, Vec<u8>) {
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Redact(pdf_transform::redact::RedactPlan {
            source: 0,
            names: "out.pdf".parse().expect("valid name pattern"),
            image_samples: None,
        }),
        &[Source::new(bytes.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the redaction should apply");
    let mut outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1);
    (report, outputs.remove(0).1)
}

#[test]
fn a_redaction_ro_is_reported_while_removal_proceeds_without_composing_the_overlay() {
    // Positive control first: ordinary removal succeeds and reports no overlay departure.
    let plain = redaction_pdf(false);
    let parsed = Document::open(plain.clone()).expect("complete plain redaction fixture");
    assert!(!parsed.was_recovered(), "the xref offsets are exact");
    let (plain_report, plain_out) = apply_redaction(&plain);
    assert_redacted(&plain_out);
    assert!(plain_report.refused.is_empty());
    assert!(plain_report.departures.is_empty(), "{plain_report:?}");

    // RO is producer-written content independent of AP. This assertion records the current
    // declined recommendation; it does not equate RO with invented overlay text/fill. The reader
    // reports the declined overlay once. This checks the /RO-only detection path (the existing
    // OverlayText test also states /IC and therefore cannot prove /RO is recognized).
    let with_ro = redaction_pdf(true);
    let parsed = Document::open(with_ro.clone()).expect("complete /RO redaction fixture");
    assert!(!parsed.was_recovered(), "the xref offsets are exact");
    let (report, out) = apply_redaction(&with_ro);
    assert_redacted(&out);
    assert!(report.refused.is_empty());
    assert_eq!(report.departures.len(), 1, "{report:?}");
    assert_eq!(report.departures[0].page, Some(1));
    assert!(report.departures[0].detail.contains("overlay"));
}

fn pixel(interpretation: &pdf_model::Interpretation, x: u32, y: u32) -> [u8; 4] {
    use pdf_render::{Rasterizer, TargetSpec};
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("small page");
    let raster = render_cpu::CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(list, target)
        .expect("supported page");
    let offset = ((raster.height - 1 - y) * raster.width + x) as usize * 4;
    raster.data[offset..offset + 4]
        .try_into()
        .expect("one pixel")
}

fn assert_redacted(bytes: &[u8]) {
    assert!(!bytes.windows(6).any(|word| word == b"SECRET"));
    assert!(!bytes.windows(7).any(|word| word == b"/Redact"));
    let document = Document::open(bytes.to_vec()).expect("result reopens");
    assert!(!document.was_recovered());
    let page = Pages::new(&document).get(0).expect("result page retained");
    let interpreted = pdf_model::interpret(&document, &page);
    assert!(interpreted.is_complete(), "{:?}", interpreted.unsupported);
    assert!(interpreted.text.contains("KEEP"), "unrelated text survives");
    assert!(!interpreted.text.contains("SECRET"));
    assert_eq!(
        pixel(&interpreted, 15, 45),
        [0, 0, 0, 0],
        "RO was not composed at its stated origin"
    );
}
