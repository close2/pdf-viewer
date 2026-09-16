//! Applying a redaction, ISO 32000-2 §12.5.6.23 (`doc/questions/A64`).
//!
//! The calibration trap 13 asks for: a page with text inside and outside a `/QuadPoints`
//! region, where after application the inside text is **gone from the content stream** and
//! unrecoverable by extraction, the outside byte-identical, and the file re-opens and
//! re-extracts. Plus the overlay departure, the refusals, and the census — a `/Redact` planted
//! inside an object stream (§7.5.7), which a byte-grep cannot see and the parsed census must.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that cannot be built must fail loudly, and its byte offsets \
              are small integers a panic on overflow would only make louder"
)]

use std::fmt::Write as _;

use pdf_syntax::serialize::{ObjectStreams, Streams, flate_encode};
use pdf_syntax::{Document, Limits, Object};
use pdf_transform::optimize::OptimizePlan;
use pdf_transform::redact::RedactPlan;
use pdf_transform::{
    Budget, Declined, Departure, MemorySinks, Origin, Plan, Policy, Source, apply,
};

mod support;

/// Assembles a one-page PDF: Helvetica in `/F1`, a 200×200 media box, the given content stream
/// and the given annotation dictionaries in `/Annots`.
fn build(content: &str, annotations: &[&str]) -> Vec<u8> {
    let mut objects: Vec<String> = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        String::new(), // page, filled once the annotation object numbers are known
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len() + 1
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    let mut annot_refs = String::new();
    for annotation in annotations {
        let number = objects.len() + 1;
        let _ = write!(annot_refs, "{number} 0 R ");
        objects.push((*annotation).to_owned());
    }
    let annots = if annotations.is_empty() {
        String::new()
    } else {
        format!(" /Annots [{}]", annot_refs.trim_end())
    };
    objects[2] = format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /Font << /F1 5 0 R \
         >> >> /Contents 4 0 R{annots} >>"
    );
    assemble(&objects)
}

/// Writes a flat object list into a §7.5.4 cross-referenced file, objects numbered from one.
fn assemble(objects: &[String]) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj {object} endobj", index + 1);
    }
    let at = out.len();
    let size = objects.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Applies a redaction to the bytes, returning the report and the one output file.
fn redact(bytes: &[u8]) -> (pdf_transform::Report, Vec<u8>) {
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Redact(RedactPlan {
            source: 0,
            names: "out.pdf".parse().expect("a pattern"),
        }),
        &[Source::new(bytes.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the redaction applies");
    let mut outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1, "one input, one output");
    (report, outputs.remove(0).1)
}

/// The extracted text of a document's first page — the same string
/// `pdf_model::interpret(...).text` gives, which is what `pdf-retrieve` returns by default.
fn page_text(bytes: &[u8]) -> String {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page).text
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// The calibration: inside a `/QuadPoints` region the text is gone from the content stream and
/// unrecoverable; outside it the bytes are identical; the file re-opens and re-extracts.
#[test]
fn text_inside_a_quadpoints_region_is_removed_and_outside_it_is_byte_identical() {
    // "KEEP" sits at y = 150, "SECRET" at y = 50; the region is the band [10,40]–[130,66].
    let content = "BT /F1 12 Tf 20 150 Td (KEEP) Tj ET\nBT /F1 12 Tf 20 50 Td (SECRET) Tj ET";
    let bytes = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [10 40 130 66] \
           /QuadPoints [10 66 130 66 10 40 130 40] >>"],
    );
    assert!(contains(&bytes, b"SECRET"), "the fixture holds the secret");

    let (report, out) = redact(&bytes);

    assert!(
        report.refused.is_empty(),
        "nothing was refused: {:?}",
        report.refused
    );
    assert!(
        !contains(&out, b"SECRET"),
        "the removed text is gone from the file, not merely covered"
    );
    assert!(
        contains(&out, b"(KEEP)"),
        "the surviving show operator is byte-identical"
    );
    assert!(
        !contains(&out, b"/Redact"),
        "§12.5.6.23: the redaction annotation is removed too"
    );

    let text = page_text(&out);
    assert!(
        text.contains("KEEP"),
        "the outside text re-extracts: {text:?}"
    );
    assert!(
        !text.contains("SECRET"),
        "the removed text is unrecoverable by extraction: {text:?}"
    );

    let Some(Origin::Redacted {
        annotations,
        glyphs,
        ..
    }) = report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("the report states a redacted origin");
    };
    assert_eq!(annotations, 1, "one annotation applied");
    assert_eq!(glyphs, 6, "the six glyphs of SECRET were removed");
}

/// Per-glyph removal inside one show operator: `[(A) -5000 (B)] TJ` places B far to the right,
/// where a narrow region catches it and leaves A. The advance restoration (§9.4.4's `w0`, read
/// from the placed quad) keeps A where it was.
#[test]
fn one_glyph_is_removed_from_the_middle_of_a_show_operator() {
    let content = "BT /F1 12 Tf 20 100 Td [(A) -5000 (B)] TJ ET";
    let bytes = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [78 94 130 112] >>"],
    );
    let (report, out) = redact(&bytes);

    assert!(
        report.refused.is_empty(),
        "nothing refused: {:?}",
        report.refused
    );
    let Some(Origin::Redacted { glyphs, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("a redacted origin");
    };
    assert_eq!(glyphs, 1, "exactly the one glyph in the region went");
    let text = page_text(&out);
    assert_eq!(text.trim(), "A", "B is gone and A survives: {text:?}");
    assert!(contains(&out, b"TJ"), "the operator was rebuilt as a TJ");
}

/// A redaction that states `/OverlayText` gets the removal and a reported departure: the content
/// is gone, the overlay is not composed (A64/A65's provenance fence).
#[test]
fn an_overlay_is_a_reported_departure_not_a_drawn_mark() {
    let content = "BT /F1 12 Tf 20 50 Td (SECRET) Tj ET";
    let bytes = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [10 40 130 66] \
           /IC [0 0 0] /OverlayText (REDACTED) /DA (/Helv 0 Tf 0 g) >>"],
    );
    let (report, out) = redact(&bytes);

    assert!(!contains(&out, b"SECRET"), "the content is still removed");
    assert!(
        !contains(&out, b"REDACTED"),
        "the overlay text is not composed onto the page"
    );
    let departures: Vec<&Departure> = report.departures.iter().collect();
    assert_eq!(departures.len(), 1, "the departure is reported once");
    assert_eq!(departures[0].page, Some(1), "on the page it happened");
    assert!(
        departures[0].detail.contains("overlay"),
        "the departure says what was not composed: {:?}",
        departures[0]
    );
}

/// A page whose region also holds a painted path is refused by name — the content and the
/// annotation are left as the file wrote them, never cut wrong (trap 5).
#[test]
fn a_painted_path_in_the_region_refuses_the_page() {
    let content = "20 40 100 20 re f\nBT /F1 12 Tf 20 50 Td (SECRET) Tj ET";
    let bytes = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [10 40 130 66] >>"],
    );
    let (report, out) = redact(&bytes);

    let refused: Vec<&Declined> = report.refused.iter().collect();
    assert_eq!(
        refused.len(),
        1,
        "the page is declined: {:?}",
        report.refused
    );
    assert_eq!(refused[0].page, Some(1));
    assert!(
        contains(&out, b"SECRET"),
        "a refused page keeps its content, so nothing was cut wrong"
    );
    assert!(
        contains(&out, b"/Redact"),
        "and keeps its unapplied redaction annotation"
    );
}

/// A document with no `/Redact` annotation is written unchanged, and its text re-extracts.
#[test]
fn a_document_without_redactions_is_written_and_unchanged() {
    let content = "BT /F1 12 Tf 20 100 Td (HELLO) Tj ET";
    let bytes = build(content, &[]);
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty());
    assert!(report.departures.is_empty());
    assert_eq!(page_text(&out).trim(), "HELLO");
    let Some(Origin::Redacted { glyphs, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("a redacted origin");
    };
    assert_eq!(glyphs, 0, "nothing was removed");
}

/// The census: a `/Redact` planted inside an object stream (§7.5.7) is invisible to a byte-grep
/// and must be seen by the parsed census — and applied.
#[test]
fn a_redaction_hidden_in_an_object_stream_is_found_and_applied() {
    let bytes = object_stream_fixture();
    assert!(
        !contains(&bytes, b"/Redact"),
        "the planted annotation is not visible to a byte-grep"
    );
    // The parsed reader sees it: it resolves the annotation and its region.
    let document = Document::open_with_limits(bytes.clone(), Limits::DEFAULT).expect("opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let annots = document.get_key(&page.dict, "Annots");
    let has_redact = annots.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            document
                .resolve(item)
                .as_dict()
                .and_then(|dict| document.get_key(dict, "Subtype").as_name().cloned())
                .is_some_and(|name| name.as_bytes() == b"Redact")
        })
    });
    assert!(
        has_redact,
        "the census resolves the object-stream-hidden /Redact"
    );

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "it applies: {:?}",
        report.refused
    );
    assert!(
        !contains(&out, b"SECRET"),
        "and the hidden redaction removes the content"
    );
}

/// A §7.5.7 file whose `/Redact` annotation lives in an object stream, so the annotation is
/// compressed out of a byte-grep's sight.
///
/// Built by writing a plain fixture and running it through this suite's own `optimize` verb with
/// [`ObjectStreams::DEFAULT`], which is §7.5.8's cross-reference stream and §7.5.7's object
/// streams — a real object-stream file rather than one this test hand-rolls.
fn object_stream_fixture() -> Vec<u8> {
    let content = "BT /F1 12 Tf 20 50 Td (SECRET) Tj ET";
    let plain = build(
        content,
        &["<< /Type /Annot /Subtype /Redact /Rect [10 40 130 66] \
           /QuadPoints [10 66 130 66 10 40 130 40] >>"],
    );
    let sinks = MemorySinks::new();
    apply(
        &Plan::Optimize(OptimizePlan {
            source: 0,
            names: "objstm.pdf".parse().expect("a pattern"),
            prune: true,
            object_streams: ObjectStreams::DEFAULT,
            streams: Streams::DEFAULT,
        }),
        &[Source::new(plain)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the object-stream rewrite applies");
    sinks.into_outputs().remove(0).1
}

/// The Isartor `6.5.2` witness — a `-fail-` PDF/A-1b fixture carrying a `/Redact` inside an
/// object stream — opens, its region resolves, and `quorra-transform`'s verb runs on it.
#[test]
fn the_isartor_witness_opens_and_its_region_resolves() {
    let path = support::committed(
        "veraPDF-corpus/Isartor test files/PDFA-1b/6.5 Annotations/6.5.2 Annotation \
         types/isartor-6-5-2-t01-fail-h.pdf",
    );
    let Ok(bytes) = std::fs::read(&path) else {
        // The corpus submodule is not checked out in this tree; the census's other half stands.
        return;
    };
    let document =
        Document::open_with_limits(bytes.clone(), Limits::DEFAULT).expect("the witness opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let annots = document.get_key(&page.dict, "Annots");
    let region_resolves = annots.as_array().is_some_and(|items| {
        items.iter().any(|item| {
            let resolved = document.resolve(item);
            let Some(dict) = resolved.as_dict() else {
                return false;
            };
            document
                .get_key(dict, "Subtype")
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"Redact")
                && !matches!(document.get_key(dict, "Rect"), Object::Null)
        })
    });
    assert!(region_resolves, "the witness's /Redact region resolves");

    // End to end through `apply`: it either applies or refuses by name, never panics or leaks.
    let (report, _out) = redact(&bytes);
    assert!(
        report.outputs.len() == 1,
        "the verb wrote a document for the witness"
    );
}

/// Assembles a §7.5.4 cross-referenced file from byte objects, numbered from one — the binary
/// counterpart of [`assemble`], for a fixture whose image stream is not valid UTF-8.
fn assemble_bytes(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::from(&b"%PDF-1.7\n"[..]);
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj ", index + 1).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b" endobj\n");
    }
    let at = out.len();
    let size = objects.len() + 1;
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{at}\n%%EOF\n").as_bytes(),
    );
    out
}

/// An 8×8 `DeviceGray` image whose sample `(col, row)` is `col*8 + row + 1` — every sample
/// distinct and non-zero, so a cleared sample (zero) is unmistakable and a mixed-up row would be
/// caught.
fn distinct_samples() -> Vec<u8> {
    let mut samples = Vec::with_capacity(64);
    for row in 0..8u8 {
        for col in 0..8u8 {
            samples.push(col * 8 + row + 1);
        }
    }
    samples
}

/// The one image `XObject` as a byte object, `/Filter` and data given.
fn image_object(filter: &str, data: &[u8]) -> Vec<u8> {
    let mut object = format!(
        "<< /Type /XObject /Subtype /Image /Width 8 /Height 8 /ColorSpace /DeviceGray \
         /BitsPerComponent 8 /Filter /{filter} /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    object.extend_from_slice(data);
    object.extend_from_slice(b"\nendstream");
    object
}

/// The output's one image `XObject` decoded back to its packed samples.
fn read_back_samples(bytes: &[u8]) -> Vec<u8> {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let xobjects = document.get_key(&page.resources, "XObject");
    let entry = xobjects
        .as_dict()
        .and_then(|dict| dict.get("Im1"))
        .cloned()
        .expect("/Im1 in the resources");
    let image = document.resolve(&entry);
    let stream = image.as_stream().expect("the image is a stream");
    // Re-interpret the page too: the destroyed image must not stop it drawing (it renders).
    let _ = pdf_model::interpret(&document, &page);
    document
        .image_stream(stream)
        .expect("the image decodes to samples")
        .data
        .to_vec()
}

/// The calibration trap 13 asks for on the image case (§12.5.6.23, "that portion of the image
/// data shall be destroyed"): an 8×8 image is placed over the page's [50,150]² square, and a
/// `/QuadPoints` region covers its left half. After application the left four columns are zero in
/// the decoded image and unrecoverable, the right four columns are byte-identical, and the file
/// re-opens and its image decodes.
#[test]
fn image_samples_inside_a_quadpoints_region_are_destroyed_and_outside_intact() {
    let samples = distinct_samples();
    let encoded = flate_encode(&samples, 6).expect("the fixture image deflates");
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject << /Im1 6 \
          0 R >> >> /Contents 4 0 R /Annots [5 0 R] >>"
            .to_vec(),
        b"<< /Length 33 >>\nstream\nq 100 0 0 100 50 50 cm /Im1 Do Q\nendstream".to_vec(),
        b"<< /Type /Annot /Subtype /Redact /Rect [50 50 100 150] \
          /QuadPoints [50 150 100 150 100 50 50 50] >>"
            .to_vec(),
        image_object("FlateDecode", &encoded),
    ];
    let bytes = assemble_bytes(&objects);

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "the image is cleared, not refused: {:?}",
        report.refused
    );
    // Principle 1: the original stream bytes are gone from the file, not left as an orphan the
    // new one sits beside — the destruction is a destruction.
    assert!(
        !contains(&out, &encoded),
        "the original image stream is not left in the file"
    );

    let out_samples = read_back_samples(&out);
    assert_eq!(out_samples.len(), 64, "the grid survives");
    for row in 0..8usize {
        for col in 0..8usize {
            let got = out_samples[row * 8 + col];
            if col < 4 {
                assert_eq!(
                    got, 0,
                    "col {col} row {row} is in the region: its sample is destroyed"
                );
            } else {
                let want = u8::try_from(col * 8 + row + 1).expect("small");
                assert_eq!(
                    got, want,
                    "col {col} row {row} is outside the region: its sample is byte-identical"
                );
            }
        }
    }

    let Some(Origin::Redacted { images, glyphs, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("the report states a redacted origin");
    };
    assert_eq!(images, 1, "one image had samples destroyed");
    assert_eq!(glyphs, 0, "no text was in this fixture");
}

/// An image behind a lossy codec (`DCTDecode`) meeting the region is refused by name, never
/// cleared — its samples are behind a codec this removal does not re-encode (trap 5, principle 1).
#[test]
fn an_image_behind_a_codec_is_refused_by_name() {
    // The bytes are never decoded: the codec is detected from `/Filter` before any decode.
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject << /Im1 6 \
          0 R >> >> /Contents 4 0 R /Annots [5 0 R] >>"
            .to_vec(),
        b"<< /Length 33 >>\nstream\nq 100 0 0 100 50 50 cm /Im1 Do Q\nendstream".to_vec(),
        b"<< /Type /Annot /Subtype /Redact /Rect [50 50 100 150] \
          /QuadPoints [50 150 100 150 100 50 50 50] >>"
            .to_vec(),
        image_object("DCTDecode", b"\xff\xd8\xff\xd9"),
    ];
    let bytes = assemble_bytes(&objects);

    let (report, _out) = redact(&bytes);
    let refused = report
        .refused
        .iter()
        .find(|declined| declined.page == Some(1))
        .expect("the page is refused");
    assert!(
        refused.detail.contains("DCTDecode") && refused.detail.contains("codec"),
        "the refusal names the codec: {}",
        refused.detail
    );
}

/// A shared image — placed on a second page as well — is refused rather than cleared, because
/// overwriting its samples would destroy the other page's picture (the single-referrer guard).
#[test]
fn a_shared_image_is_refused_rather_than_cleared() {
    let samples = distinct_samples();
    let encoded = flate_encode(&samples, 6).expect("the fixture image deflates");
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R 7 0 R] /Count 2 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject << /Im1 6 \
          0 R >> >> /Contents 4 0 R /Annots [5 0 R] >>"
            .to_vec(),
        b"<< /Length 33 >>\nstream\nq 100 0 0 100 50 50 cm /Im1 Do Q\nendstream".to_vec(),
        b"<< /Type /Annot /Subtype /Redact /Rect [50 50 100 150] \
          /QuadPoints [50 150 100 150 100 50 50 50] >>"
            .to_vec(),
        image_object("FlateDecode", &encoded),
        // A second page placing the very same image object 6.
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /XObject << /Im1 6 \
          0 R >> >> /Contents 8 0 R >>"
            .to_vec(),
        b"<< /Length 33 >>\nstream\nq 100 0 0 100 50 50 cm /Im1 Do Q\nendstream".to_vec(),
    ];
    let bytes = assemble_bytes(&objects);

    let (report, out) = redact(&bytes);
    let refused = report
        .refused
        .iter()
        .find(|declined| declined.page == Some(1))
        .expect("the shared-image page is refused");
    assert!(
        refused.detail.contains("shared"),
        "the refusal says the image is shared: {}",
        refused.detail
    );
    // The image is left whole: the other page's picture is intact.
    let out_samples = read_back_samples(&out);
    assert_eq!(
        out_samples,
        distinct_samples(),
        "the shared image is untouched"
    );
}
