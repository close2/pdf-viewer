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

use pdf_model::colour::Conversion;
use pdf_render::Color;
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

/// An image behind `JPXDecode` meeting the region is refused by name, never cleared — a
/// codestream over the decoder's budget comes back at a reduced resolution level (§7.4.9 NOTE 3),
/// so its raster is not the image's grid and the redaction cannot be proven to replace the
/// full-resolution content (trap 5, principle 1). `JBIG2Decode` is now cleared, not refused
/// (`a_jbig2_image_in_the_region_is_decoded_cleared_and_reencoded_as_flate`).
#[test]
fn an_image_behind_jpx_is_refused_by_name() {
    // The bytes are never decoded: the codec is refused from `/Filter` before any decode.
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
        image_object("JPXDecode", b"\x00\x00\x00\x0cjP  "),
    ];
    let bytes = assemble_bytes(&objects);

    let (report, _out) = redact(&bytes);
    let refused = report
        .refused
        .iter()
        .find(|declined| declined.page == Some(1))
        .expect("the page is refused");
    assert!(
        refused.detail.contains("JPXDecode") && refused.detail.contains("codec"),
        "the refusal names the codec: {}",
        refused.detail
    );
}

/// The one image `XObject` as a byte object with a stated grid, colour space, bit depth, filter
/// and (optional) decode parameters — for the codec fixtures whose grid is not the 8×8 default.
fn codec_image_object(
    width: u32,
    height: u32,
    colour_space: &str,
    bits: u32,
    filter: &str,
    parms: &str,
    data: &[u8],
) -> Vec<u8> {
    let mut object = format!(
        "<< /Type /XObject /Subtype /Image /Width {width} /Height {height} /ColorSpace \
         /{colour_space} /BitsPerComponent {bits} /Filter /{filter}{parms} /Length {} >>\n\
         stream\n",
        data.len()
    )
    .into_bytes();
    object.extend_from_slice(data);
    object.extend_from_slice(b"\nendstream");
    object
}

/// The output's one image `XObject`, decoded to straight-alpha `RGBA8` the interpreter's own way,
/// with whether it is a `FlateDecode` `DeviceRGB` image (the codec re-encode's shape).
fn read_back_codec_image(bytes: &[u8]) -> (u32, u32, Vec<u8>, bool) {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let entry = document
        .get_key(&page.resources, "XObject")
        .as_dict()
        .and_then(|dict| dict.get("Im1"))
        .cloned()
        .expect("/Im1 in the resources");
    let image = document.resolve(&entry);
    let stream = image.as_stream().expect("the image is a stream");
    let is_flate_rgb = {
        let filter = document.get_key(&stream.dict, "Filter");
        let space = document.get_key(&stream.dict, "ColorSpace");
        let flate = matches!(filter, Object::Name(name) if name.as_bytes() == b"FlateDecode");
        let rgb = matches!(space, Object::Name(name) if name.as_bytes() == b"DeviceRGB");
        flate && rgb
    };
    // The redacted page must still draw (its interpretation must not fault on the new image).
    let _ = pdf_model::interpret(&document, &page);
    let flattened = pdf_model::image::decode(
        &document,
        stream,
        &page.resources,
        Color::BLACK,
        &Conversion::device(),
    )
    .expect("the output image decodes");
    (
        flattened.image.width,
        flattened.image.height,
        flattened.image.data.to_vec(),
        is_flate_rgb,
    )
}

/// The calibration trap 13 asks for on the codec case: a `DCTDecode` image (§7.4.8) over the
/// page's [50,150]² square, a `/QuadPoints` region on its left half. After application the region
/// pixels are the zero constant in the re-read image, the rest byte-identical to the original
/// decode, and the output is a `FlateDecode` `DeviceRGB` image — no codec, so the cleared region
/// cannot round-trip back through the lossy filter that would leak it.
#[test]
fn a_dct_image_in_the_region_is_decoded_cleared_and_reencoded_as_flate() {
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
        codec_image_object(16, 16, "DeviceRGB", 8, "DCTDecode", "", DCT_JPEG_16X16),
    ];
    let bytes = assemble_bytes(&objects);

    // The original image decoded the interpreter's way: what "the rest is intact" is measured
    // against. A JPEG decodes deterministically, so the output's untouched pixels are these.
    let original = decode_fixture_rgba(&bytes);

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "the codec image is cleared, not refused: {:?}",
        report.refused
    );
    // Principle 1: the original codec bytes are gone from the file — no orphan to recover.
    assert!(
        !contains(&out, DCT_JPEG_16X16),
        "the original DCT stream is not left in the file"
    );

    let (width, height, rgba, is_flate_rgb) = read_back_codec_image(&out);
    assert_eq!((width, height), (16, 16), "the grid survives the re-encode");
    assert!(
        is_flate_rgb,
        "the output image is a FlateDecode DeviceRGB stream, not a codec"
    );
    for row in 0..16usize {
        for col in 0..16usize {
            let at = (row * 16 + col) * 4;
            if col < 8 {
                assert_eq!(
                    &rgba[at..at + 3],
                    &[0, 0, 0],
                    "col {col} row {row} is in the region: its sample is the zero constant"
                );
            } else {
                assert_eq!(
                    &rgba[at..at + 3],
                    &original[at..at + 3],
                    "col {col} row {row} is outside the region: byte-identical to the decode"
                );
            }
        }
    }
    // The two halves of the original differ, so a preserved right half is not a cleared one.
    assert_ne!(
        &original[8 * 4..8 * 4 + 3],
        &original[0..3],
        "the fixture's halves differ, so 'intact' is a real assertion"
    );

    let Some(Origin::Redacted { images, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("the report states a redacted origin");
    };
    assert_eq!(images, 1, "one image had samples destroyed");
}

/// The same calibration for a `CCITTFaxDecode` image (§7.4.6). The bilevel image is decoded
/// through the codec (in-process here, the confined worker in the program — principle 3), its
/// region cleared, and the output written as a `FlateDecode` `DeviceRGB` raster.
#[test]
fn a_ccitt_image_in_the_region_is_decoded_cleared_and_reencoded_as_flate() {
    // In-process decode: the same routine the confined worker runs, with no worker binary needed.
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
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
        codec_image_object(
            16,
            16,
            "DeviceGray",
            1,
            "CCITTFaxDecode",
            " /DecodeParms << /K -1 /Columns 16 /Rows 16 >>",
            CCITT_G4_16X16,
        ),
    ];
    let bytes = assemble_bytes(&objects);
    let original = decode_fixture_rgba(&bytes);

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "the CCITT image is cleared, not refused: {:?}",
        report.refused
    );
    assert!(
        !contains(&out, CCITT_G4_16X16),
        "the original CCITT stream is not left in the file"
    );

    let (width, height, rgba, is_flate_rgb) = read_back_codec_image(&out);
    assert_eq!((width, height), (16, 16), "the grid survives the re-encode");
    assert!(
        is_flate_rgb,
        "the output image is a FlateDecode DeviceRGB stream, not a codec"
    );
    for row in 0..16usize {
        for col in 0..16usize {
            let at = (row * 16 + col) * 4;
            if col < 8 {
                assert_eq!(
                    &rgba[at..at + 3],
                    &[0, 0, 0],
                    "col {col} row {row} is in the region: the zero constant"
                );
            } else {
                assert_eq!(
                    &rgba[at..at + 3],
                    &original[at..at + 3],
                    "col {col} row {row} is outside the region: byte-identical to the decode"
                );
            }
        }
    }
    // Left half and right half of the fixture differ (black vs white), so the intact assertion
    // is real rather than vacuous.
    assert_ne!(
        &original[8 * 4..8 * 4 + 3],
        &original[0..3],
        "the fixture's halves differ, so 'intact' is a real assertion"
    );
}

/// The output's one image `XObject`, decoded to straight-alpha `RGBA8`, with whether it is the
/// bilevel re-encode's shape: a `FlateDecode` 1-bit `DeviceGray` image (ADR 1143).
fn read_back_bilevel_image(bytes: &[u8]) -> (u32, u32, Vec<u8>, bool) {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let entry = document
        .get_key(&page.resources, "XObject")
        .as_dict()
        .and_then(|dict| dict.get("Im1"))
        .cloned()
        .expect("/Im1 in the resources");
    let image = document.resolve(&entry);
    let stream = image.as_stream().expect("the image is a stream");
    let is_flate_gray_1bit = {
        let filter = document.get_key(&stream.dict, "Filter");
        let space = document.get_key(&stream.dict, "ColorSpace");
        let bits = document.get_key(&stream.dict, "BitsPerComponent");
        let flate = matches!(filter, Object::Name(name) if name.as_bytes() == b"FlateDecode");
        let gray = matches!(space, Object::Name(name) if name.as_bytes() == b"DeviceGray");
        flate && gray && matches!(bits, Object::Integer(1))
    };
    // The redacted page must still draw: its interpretation must not fault on the new image.
    let _ = pdf_model::interpret(&document, &page);
    let flattened = pdf_model::image::decode(
        &document,
        stream,
        &page.resources,
        Color::BLACK,
        &Conversion::device(),
    )
    .expect("the output image decodes");
    (
        flattened.image.width,
        flattened.image.height,
        flattened.image.data.to_vec(),
        is_flate_gray_1bit,
    )
}

/// The calibration trap 13 asks for on the bilevel-codec case: the ISO 32000-2 §7.4.7 worked
/// example (a 52×66 `JBIG2Decode` image, a letter C drawn twice) over the page's [50,150]² square,
/// a `/QuadPoints` region on its left half. The image is decoded through the codec (in-process
/// here, the confined worker in the program — principle 3), its region cleared, and the output
/// written as a `FlateDecode` **1-bit `DeviceGray`** raster (ADR 1143): a lossless, non-codec
/// filter at the bilevel image's own depth, so the cleared region is exactly the one-bit zero
/// constant and cannot round-trip back through the codec. The bytes are the specification's own,
/// never another implementation's output (principle 5).
#[test]
fn a_jbig2_image_in_the_region_is_decoded_cleared_and_reencoded_as_flate() {
    // In-process decode: the same routine the confined worker runs, with no worker binary needed.
    pdf_sandbox::set_isolation(pdf_sandbox::Isolation::InProcess);
    let mut image = format!(
        "<< /Type /XObject /Subtype /Image /Width 52 /Height 66 /ColorSpace /DeviceGray \
         /BitsPerComponent 1 /Filter /JBIG2Decode /DecodeParms << /JBIG2Globals 7 0 R >> \
         /Length {} >>\nstream\n",
        JBIG2_IMAGE.len()
    )
    .into_bytes();
    image.extend_from_slice(JBIG2_IMAGE);
    image.extend_from_slice(b"\nendstream");
    let mut globals = format!("<< /Length {} >>\nstream\n", JBIG2_GLOBALS.len()).into_bytes();
    globals.extend_from_slice(JBIG2_GLOBALS);
    globals.extend_from_slice(b"\nendstream");
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
        image,
        globals,
    ];
    let bytes = assemble_bytes(&objects);
    let original = decode_fixture_rgba(&bytes);

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "the JBIG2 image is cleared, not refused: {:?}",
        report.refused
    );
    // Principle 1: the original JBIG2 segments are gone from the file — no orphan to recover.
    assert!(
        !contains(&out, JBIG2_IMAGE),
        "the original JBIG2 image stream is not left in the file"
    );
    assert!(
        !contains(&out, JBIG2_GLOBALS),
        "the orphaned JBIG2 globals stream is not left in the file"
    );

    let (width, height, rgba, is_flate_gray_1bit) = read_back_bilevel_image(&out);
    assert_eq!((width, height), (52, 66), "the grid survives the re-encode");
    assert!(
        is_flate_gray_1bit,
        "the output image is a FlateDecode 1-bit DeviceGray stream, not a codec"
    );
    // The region is the left half of the placement: sample centres whose column maps into [50,100]
    // in user space, which is columns 0..26 of the 52-wide image (all rows).
    let (mut cleared_had_white, mut intact_had_white) = (false, false);
    for row in 0..66usize {
        for col in 0..52usize {
            let at = (row * 52 + col) * 4;
            if col < 26 {
                assert_eq!(
                    &rgba[at..at + 3],
                    &[0, 0, 0],
                    "col {col} row {row} is in the region: the one-bit zero constant (black)"
                );
                cleared_had_white |= original[at] >= 0x80;
            } else {
                assert_eq!(
                    &rgba[at..at + 3],
                    &original[at..at + 3],
                    "col {col} row {row} is outside the region: byte-identical to the decode"
                );
                intact_had_white |= original[at] >= 0x80;
            }
        }
    }
    // The cleared region held white (background) pixels the redaction turned black, so clearing
    // changed real content; the intact region held white pixels too, so 'byte-identical' is not a
    // vacuous statement about an all-black image (a bilevel page is mostly white).
    assert!(
        cleared_had_white,
        "the cleared region held white pixels the redaction destroyed"
    );
    assert!(
        intact_had_white,
        "the intact region held white pixels, so 'intact' is a real assertion"
    );

    let Some(Origin::Redacted { images, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("the report states a redacted origin");
    };
    assert_eq!(images, 1, "one image had samples destroyed");
}

/// Decodes the fixture's one image `XObject` to straight-alpha `RGBA8` from the input bytes,
/// giving the pixels the redaction's untouched region must still equal.
fn decode_fixture_rgba(bytes: &[u8]) -> Vec<u8> {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let entry = document
        .get_key(&page.resources, "XObject")
        .as_dict()
        .and_then(|dict| dict.get("Im1"))
        .cloned()
        .expect("/Im1 in the resources");
    let image = document.resolve(&entry);
    let stream = image.as_stream().expect("the image is a stream");
    pdf_model::image::decode(
        &document,
        stream,
        &page.resources,
        Color::BLACK,
        &Conversion::device(),
    )
    .expect("the fixture image decodes")
    .image
    .data
    .to_vec()
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

/// The first offset of `needle` in `haystack`, or `None`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// A one-page fixture whose content places an 8×8 `DeviceGray` inline image over the page's
/// [50,150]² square, with the given filter/data spelling written between `ID` and `EI`. `prefix`
/// and `suffix` bracket the `BI`…`EI` run so a test can prove they cross the output byte for byte.
fn inline_fixture(image: &[u8], quadpoints: &str) -> Vec<u8> {
    let mut content = b"q 100 0 0 100 50 50 cm\n".to_vec();
    content.extend_from_slice(image);
    content.extend_from_slice(b"\nQ");

    let mut content_object = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    content_object.extend_from_slice(&content);
    content_object.extend_from_slice(b"\nendstream");

    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> /Contents 4 0 R \
          /Annots [5 0 R] >>"
            .to_vec(),
        content_object,
        format!(
            "<< /Type /Annot /Subtype /Redact /Rect [50 50 100 150] /QuadPoints [{quadpoints}] >>"
        )
        .into_bytes(),
    ];
    assemble_bytes(&objects)
}

/// The first-page content stream of `bytes`, the inline image's decoded samples, and the content
/// split around the `BI`\u{2026}`EI` run: everything before `BI` and everything after `EI`.
fn read_back_inline(bytes: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let content = page.content(&document);
    // The spliced image must still draw: interpreting the page must not stumble on the new run.
    let _ = pdf_model::interpret(&document, &page);
    let bi = find(&content, b"BI").expect("a BI operator in the content");
    let scan = pdf_model::inline_image::scan(&document, &content, bi + 2, &page.resources, true);
    let stream = scan.image.expect("the inline image reads back");
    let samples = document
        .image_stream(&stream)
        .expect("the inline image decodes to samples")
        .data
        .to_vec();
    let before = content[..bi].to_vec();
    let after = content[scan.resume.min(content.len())..].to_vec();
    (content, samples, before, after)
}

/// The calibration trap 13 asks for on the inline-image case (§8.9.7, §12.5.6.23): an 8×8
/// unfiltered inline image is placed over the page's [50,150]² square and a `/QuadPoints` region
/// covers its left half. After application the left four columns are zero in the spliced image
/// and unrecoverable, the right four columns are byte-identical, every byte of the content stream
/// outside the `BI`…`EI` run is unchanged, and the file re-opens and the image decodes and draws.
#[test]
fn inline_image_samples_inside_a_quadpoints_region_are_destroyed_and_the_stream_is_spliced() {
    let samples = distinct_samples();
    let mut image = b"BI /W 8 /H 8 /BPC 8 /CS /G ID\n".to_vec();
    image.extend_from_slice(&samples);
    image.extend_from_slice(b"\nEI");
    let bytes = inline_fixture(&image, "50 150 100 150 100 50 50 50");

    let (report, out) = redact(&bytes);
    // The same fixture, read before redaction, gives the surrounding bytes to compare against.
    let (_before_content, in_samples, in_before, in_after) = read_back_inline(&bytes);
    assert_eq!(
        in_samples,
        distinct_samples(),
        "the input image reads back whole"
    );

    let (content, out_samples, out_before, out_after) = read_back_inline(&out);
    assert!(
        report.refused.is_empty(),
        "the inline image is spliced, not refused: {:?}",
        report.refused
    );
    assert_eq!(out_samples.len(), 64, "the grid survives the splice");
    for row in 0..8usize {
        for col in 0..8usize {
            let got = out_samples[row * 8 + col];
            if col < 4 {
                assert_eq!(got, 0, "col {col} row {row} is in the region: destroyed");
            } else {
                let want = u8::try_from(col * 8 + row + 1).expect("small");
                assert_eq!(got, want, "col {col} row {row} is outside: byte-identical");
            }
        }
    }

    // The surrounding stream is byte-identical: everything before BI, and everything after EI up
    // to the trailing white space the content reader appends afresh on each read (which a
    // redaction round-trip bakes into the stored stream, ADR 1124's apply-on-reader-output).
    assert_eq!(
        out_before, in_before,
        "the content before the run is byte-identical"
    );
    let trim = |b: &[u8]| {
        let end = b
            .iter()
            .rposition(|c| !c.is_ascii_whitespace())
            .map_or(0, |i| i + 1);
        b[..end].to_vec()
    };
    assert_eq!(
        trim(&out_after),
        trim(&in_after),
        "the content after the run is byte-identical but for reader trailing white space"
    );
    // Principle 1: the original sample block is gone from the decoded content, not covered \u2014 the
    // left columns are the constant and the block never appears whole again.
    assert!(
        find(&content, &samples).is_none(),
        "the original inline sample block is not left in the spliced content"
    );

    let Some(Origin::Redacted { images, glyphs, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("the report states a redacted origin");
    };
    assert_eq!(images, 1, "one inline image had its samples destroyed");
    assert_eq!(glyphs, 0, "no text was in this fixture");
}

/// An inline image behind a lossy codec (`DCTDecode`) meeting the region is refused by name, never
/// spliced — its samples are behind a codec this removal does not re-encode (trap 5, principle 1).
#[test]
fn an_inline_image_behind_a_codec_is_refused_by_name() {
    // The four bytes are never decoded: the codec is seen from `/F` before any decode is tried.
    let image = b"BI /W 8 /H 8 /BPC 8 /CS /G /F /DCT /L 4 ID\n\xff\xd8\xff\xd9\nEI".to_vec();
    let bytes = inline_fixture(&image, "50 150 100 150 100 50 50 50");

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

/// An inline image the redaction does not touch crosses the output byte for byte: the run is
/// skipped, never spliced, when its placement is nowhere near a region.
#[test]
fn an_inline_image_clear_of_the_region_is_left_untouched() {
    let samples = distinct_samples();
    let mut image = b"BI /W 8 /H 8 /BPC 8 /CS /G ID\n".to_vec();
    image.extend_from_slice(&samples);
    image.extend_from_slice(b"\nEI");
    // The region is the page's top-right corner [150,190]², clear of the image's [50,150]² square.
    let bytes = inline_fixture(&image, "150 190 190 190 190 150 150 150");

    let (report, out) = redact(&bytes);
    assert!(
        report.refused.is_empty(),
        "no page is refused: {:?}",
        report.refused
    );
    let (_content, out_samples, _before, _after) = read_back_inline(&out);
    assert_eq!(
        out_samples, samples,
        "the untouched inline image is byte-identical"
    );
    let Some(Origin::Redacted { images, .. }) =
        report.outputs.first().map(|output| output.origin.clone())
    else {
        panic!("the report states a redacted origin");
    };
    assert_eq!(images, 0, "no image was destroyed");
}

// --- Codec fixtures (generated with PIL; see doc/history/1136) ---
// There is no JPEG or CCITT encoder in the tree, so a valid codec stream cannot be built from
// pixels in-test; these are captured bytes. Each is decoded through the real codec, and the test
// asserts against that decode — never against a hand-predicted pixel — so what they encode is
// self-checking. Both are 16×16, left half distinct from the right so an intact right half is not
// a cleared one.

const DCT_JPEG_16X16: &[u8] = &[
    // 653 bytes  16x16 RGB baseline JPEG (left red, right blue), q90 4:4:4
    0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
    0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x03, 0x02, 0x02, 0x03, 0x02, 0x02, 0x03,
    0x03, 0x03, 0x03, 0x04, 0x03, 0x03, 0x04, 0x05, 0x08, 0x05, 0x05, 0x04, 0x04, 0x05, 0x0A, 0x07,
    0x07, 0x06, 0x08, 0x0C, 0x0A, 0x0C, 0x0C, 0x0B, 0x0A, 0x0B, 0x0B, 0x0D, 0x0E, 0x12, 0x10, 0x0D,
    0x0E, 0x11, 0x0E, 0x0B, 0x0B, 0x10, 0x16, 0x10, 0x11, 0x13, 0x14, 0x15, 0x15, 0x15, 0x0C, 0x0F,
    0x17, 0x18, 0x16, 0x14, 0x18, 0x12, 0x14, 0x15, 0x14, 0xFF, 0xDB, 0x00, 0x43, 0x01, 0x03, 0x04,
    0x04, 0x05, 0x04, 0x05, 0x09, 0x05, 0x05, 0x09, 0x14, 0x0D, 0x0B, 0x0D, 0x14, 0x14, 0x14, 0x14,
    0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14,
    0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14,
    0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0x14, 0xFF, 0xC0,
    0x00, 0x11, 0x08, 0x00, 0x10, 0x00, 0x10, 0x03, 0x01, 0x11, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11,
    0x01, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09,
    0x0A, 0x0B, 0xFF, 0xC4, 0x00, 0xB5, 0x10, 0x00, 0x02, 0x01, 0x03, 0x03, 0x02, 0x04, 0x03, 0x05,
    0x05, 0x04, 0x04, 0x00, 0x00, 0x01, 0x7D, 0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21,
    0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08, 0x23,
    0x42, 0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0A, 0x16, 0x17,
    0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A,
    0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A,
    0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A,
    0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99,
    0x9A, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7,
    0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5,
    0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF1,
    0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA, 0xFF, 0xC4, 0x00, 0x1F, 0x01, 0x00, 0x03,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
    0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0xFF, 0xC4, 0x00, 0xB5, 0x11, 0x00,
    0x02, 0x01, 0x02, 0x04, 0x04, 0x03, 0x04, 0x07, 0x05, 0x04, 0x04, 0x00, 0x01, 0x02, 0x77, 0x00,
    0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71, 0x13,
    0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91, 0xA1, 0xB1, 0xC1, 0x09, 0x23, 0x33, 0x52, 0xF0, 0x15,
    0x62, 0x72, 0xD1, 0x0A, 0x16, 0x24, 0x34, 0xE1, 0x25, 0xF1, 0x17, 0x18, 0x19, 0x1A, 0x26, 0x27,
    0x28, 0x29, 0x2A, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49,
    0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
    0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88,
    0x89, 0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6,
    0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4,
    0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE2,
    0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9,
    0xFA, 0xFF, 0xDA, 0x00, 0x0C, 0x03, 0x01, 0x00, 0x02, 0x11, 0x03, 0x11, 0x00, 0x3F, 0x00, 0xF0,
    0x2A, 0xFC, 0xC8, 0xFE, 0xE3, 0x3C, 0xA6, 0xBF, 0xD3, 0x03, 0xFC, 0xF7, 0x3D, 0x5A, 0xBF, 0xCC,
    0xF3, 0xFD, 0x08, 0x3C, 0xA6, 0xBF, 0xD3, 0x03, 0xFC, 0xF7, 0x3F, 0xFF, 0xD9,
];

const CCITT_G4_16X16: &[u8] = &[
    // 9 bytes  16x16 CCITT G4, photometric=1 (0=WhiteIsZero)
    0x33, 0x17, 0xFF, 0xFF, 0xFF, 0xF0, 0x01, 0x00, 0x10,
];

// --- JBIG2 fixture (ISO 32000-2 §7.4.7's worked example; see doc/history/1143) ---
// There is no JBIG2 encoder in the tree, so a valid embedded stream cannot be built from pixels
// in-test. These are the specification's own worked example, split exactly where §7.4.7 splits it:
// a 52×66 bilevel image — a letter C drawn twice, upper half and lower — whose symbol dictionary is
// the globals stream and whose page-information and text-region segments are the image stream.
// Nothing here was taken from another implementation's output (principle 5); it is the same
// bitstream `pdf-sandbox`'s own §7.4.7 test decodes. The test decodes it through the real codec and
// asserts against that decode, never a hand-predicted pixel, so what it encodes is self-checking.

/// The `/JBIG2Globals` stream: segment 0, a symbol dictionary (ISO 32000-2 §7.4.7, part (b)).
const JBIG2_GLOBALS: &[u8] = &[
    0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x32, 0x00, 0x00, 0x03, 0xFF, 0xFD,
    0xFF, 0x02, 0xFE, 0xFE, 0xFE, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x2A, 0xE2, 0x25,
    0xAE, 0xA9, 0xA5, 0xA5, 0x38, 0xB4, 0xD9, 0x99, 0x9C, 0x5C, 0x8E, 0x56, 0xEF, 0x0F, 0x87, 0x27,
    0xF2, 0xB5, 0x3D, 0x4E, 0x37, 0xEF, 0x79, 0x5C, 0xC5, 0x50, 0x6D, 0xFF, 0xAC,
];

/// The image stream: segment 1, page information, and segment 2, an immediate text region
/// (ISO 32000-2 §7.4.7, part (c)).
const JBIG2_IMAGE: &[u8] = &[
    0x00, 0x00, 0x00, 0x01, 0x30, 0x00, 0x01, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00, 0x34, 0x00,
    0x00, 0x00, 0x42, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x02, 0x06, 0x20, 0x00, 0x01, 0x00, 0x00, 0x00, 0x1E, 0x00, 0x00, 0x00, 0x34, 0x00, 0x00,
    0x00, 0x42, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x10, 0x00, 0x00, 0x00,
    0x02, 0x31, 0xDB, 0x51, 0xCE, 0x51, 0xFF, 0xAC,
];
