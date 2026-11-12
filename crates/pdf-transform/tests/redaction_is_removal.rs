//! Redaction is removal, never a cover (`doc/questions/A834`, item 3).
//!
//! ISO 32000-2 §12.5.6.23: applying a redaction is to "remove all content identified by the
//! redaction annotation, as well as the annotation itself", to "remove all traces of the specified
//! content", and "\[i]f a portion of an image is contained in a redaction region, that portion of
//! the image data shall be destroyed; clipping or image masks shall not be used to hide that
//! data." The owner's word on it: "If a redaction only places something over someting else, it
//! will lead to sensitive information exposure."
//!
//! So every proof here is made by **extraction from the written file**, never by rendering: a
//! page drawn with a black box over unremoved text looks redacted and is not. Every object of the
//! file is opened and every stream decoded, and the text, the path coordinates and the image
//! samples the region held are looked for in all of them; Table 195's `/RO`, `/IC` and
//! `/OverlayText` are not drawn at all (`doc/questions/Q842` asks whether `/RO` should be, after
//! the removal). ADR 2841.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that cannot be built must fail loudly, and its byte offsets \
              are small integers a panic on overflow would only make louder"
)]

use std::fmt::Write as _;

use pdf_syntax::{Document, Limits, Object, ObjectId};
use pdf_transform::redact::RedactPlan;
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Source, apply};

/// The page's content: `SECRET` shown, a path stroked and a 2×1 image painted, all inside the
/// region `[10 40 130 66]`, and `KEEP` shown outside it.
const CONTENT: &str = "BT /F1 12 Tf 20 150 Td (KEEP) Tj ET\n\
                       BT /F1 12 Tf 20 50 Td (SECRET) Tj ET\n\
                       0 0 1 RG 31.5 47.25 m 97.75 58.5 l S\n\
                       q 20 0 0 10 100 45 cm /Im1 Do Q";

/// The image's two samples, `/DeviceRGB` at eight bits, written as hexadecimal by
/// `ASCIIHexDecode` so the fixture is text; any of them surviving is the image not destroyed.
const SAMPLES: [u8; 6] = [0xC3, 0x5A, 0x96, 0xC3, 0x5A, 0x96];

/// The overlay form the producer wrote: a black rectangle over its own `/BBox`.
const OVERLAY: &str = "0 g 0 0 120 26 re f";

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

/// A one-page document holding [`CONTENT`] and one `/Redact` annotation whose extra entries are
/// `entries`; object 8 is a form `XObject` drawing [`OVERLAY`], which `entries` may name as `/RO`.
fn document(entries: &str) -> Vec<u8> {
    let hex = SAMPLES.iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02X}");
        hex
    });
    let objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /Font << /F1 5 0 R \
         >> /XObject << /Im1 6 0 R >> >> /Contents 4 0 R /Annots [7 0 R] >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{CONTENT}\nendstream",
            CONTENT.len() + 1
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        format!(
            "<< /Type /XObject /Subtype /Image /Width 2 /Height 1 /ColorSpace /DeviceRGB \
             /BitsPerComponent 8 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}>\nendstream",
            hex.len() + 2
        ),
        format!("<< /Type /Annot /Subtype /Redact /Rect [10 40 130 66] {entries} >>"),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 120 26] /Length {} >>\nstream\n\
             {OVERLAY}\nendstream",
            OVERLAY.len() + 1
        ),
    ];
    assemble(&objects)
}

/// Applies every redaction the bytes state, returning the report and the one output file.
fn redact(bytes: &[u8]) -> (pdf_transform::Report, Vec<u8>) {
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Redact(RedactPlan {
            source: 0,
            names: "out.pdf".parse().expect("a pattern"),
            image_samples: None,
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

/// Every stream of the written file, decoded — every object the cross-reference table numbers,
/// not only those the page reaches, so an orphan holding the removed bytes is found too.
fn every_stream(bytes: &[u8]) -> Vec<(Dictionary, Vec<u8>)> {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let size = document
        .trailer()
        .get("Size")
        .and_then(Object::as_integer)
        .and_then(|size| u32::try_from(size).ok())
        .expect("a /Size");
    (1..size)
        .filter_map(|number| {
            let object = document.get(ObjectId {
                number,
                generation: 0,
            });
            let stream = object.as_stream()?;
            let data = document
                .decoded_stream_data(stream)
                .expect("every stream decodes");
            Some((stream.dict.clone(), data.to_vec()))
        })
        .collect()
}

type Dictionary = pdf_syntax::object::Dictionary;

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// The text a reader extracts from page one.
fn page_text(bytes: &[u8]) -> String {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page).text
}

/// The proof of removal, made by extraction: the text, the path's coordinates and the image's
/// samples the region held are in no stream of the file and in none of its raw bytes, and the
/// text outside the region is.
fn removed(bytes: &[u8], out: &[u8]) {
    assert!(contains(bytes, b"SECRET"), "the fixture holds the secret");
    let text = page_text(out);
    assert!(text.contains("KEEP"), "the outside text survives: {text:?}");
    assert!(!text.contains("SECRET"), "the text is extracted: {text:?}");
    assert!(!contains(out, b"SECRET"), "the text is in the file's bytes");
    assert!(
        !contains(out, b"/Redact"),
        "§12.5.6.23 removes the annotation itself"
    );
    let streams = every_stream(out);
    for (dict, data) in &streams {
        assert!(!contains(data, b"SECRET"), "a stream holds the text");
        assert!(
            !contains(data, b"31.5 47.25") && !contains(data, b"97.75 58.5"),
            "a stream holds the path's coordinates: {}",
            String::from_utf8_lossy(data)
        );
        let is_image = dict
            .get("Subtype")
            .and_then(Object::as_name)
            .is_some_and(|name| name.as_bytes() == b"Image");
        if is_image {
            assert!(
                data.iter().all(|sample| *sample == 0),
                "the image's samples are destroyed, not covered: {data:?}"
            );
        }
    }
    assert!(
        streams.iter().any(|(dict, _)| dict
            .get("Subtype")
            .and_then(Object::as_name)
            .is_some_and(|name| name.as_bytes() == b"Image")),
        "the image is written, destroyed"
    );
    assert_eq!(
        out.windows(5).filter(|window| window == b"%%EOF").count(),
        1,
        "a new file with no earlier revision the removed bytes could stay in"
    );
    assert!(
        !contains(out, b"/Prev"),
        "no earlier cross-reference section"
    );
}

/// Table 195 with `/RO`, `/IC` and `/OverlayText`: the region is removed, nothing is drawn over
/// it — not the producer's form, not a fill, not the text — and the departure says so.
#[test]
fn the_region_is_removed_and_no_overlay_stands_in_for_the_removal() {
    let bytes = document("/RO 8 0 R /IC [1 0 0] /OverlayText (NOT DRAWN) /DA (/Helv 9 Tf 0 g)");
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    removed(&bytes, &out);
    assert!(
        !contains(&out, b"NOT DRAWN"),
        "/OverlayText is not composed"
    );
    assert!(
        every_stream(&out)
            .iter()
            .all(|(_, data)| !contains(data, OVERLAY.as_bytes())),
        "the /RO form is not carried into the file"
    );
    assert_eq!(report.departures.len(), 1, "{:?}", report.departures);
    assert!(report.departures[0].detail.contains("overlay"));
}

/// The control: a redaction stating no overlay is removed the same way and departs from nothing.
#[test]
fn a_redaction_without_an_overlay_is_removed_the_same_way() {
    let bytes = document("");
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    removed(&bytes, &out);
    assert!(report.departures.is_empty(), "{:?}", report.departures);
}
