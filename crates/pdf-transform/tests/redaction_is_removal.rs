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
//! samples the region held are looked for in all of them. Table 195's `/RO` is drawn **after**
//! that removal and never in its place, and `/IC` and `/OverlayText` are not composed
//! (`doc/questions/A842`, ADRs 2841 and 2868): so the order is proven by extraction too — the
//! page's written content stream holds the producer's form invoked once, behind the edited
//! content, and none of what the region held.

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
    document_with(
        "/Rect [10 40 130 66]",
        entries,
        "/Type /XObject /Subtype /Form /BBox [0 0 120 26]",
    )
}

/// [`document`] with the annotation's `/Rect` and object 8's dictionary entries stated.
fn document_with(rect: &str, entries: &str, form: &str) -> Vec<u8> {
    document_drawing(CONTENT, rect, entries, form)
}

/// [`document_with`] with the page's content stated as well.
fn document_drawing(content: &str, rect: &str, entries: &str, form: &str) -> Vec<u8> {
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
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len() + 1
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        format!(
            "<< /Type /XObject /Subtype /Image /Width 2 /Height 1 /ColorSpace /DeviceRGB \
             /BitsPerComponent 8 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}>\nendstream",
            hex.len() + 2
        ),
        format!("<< /Type /Annot /Subtype /Redact {rect} {entries} >>"),
        format!(
            "<< {form} /Length {} >>\nstream\n{OVERLAY}\nendstream",
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

/// Page one's written content stream, decoded.
fn page_content(bytes: &[u8]) -> String {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    String::from_utf8_lossy(&page.content(&document)).into_owned()
}

/// Page one's `/XObject` resource `name`, resolved: its dictionary and its decoded data.
fn page_xobject(bytes: &[u8], name: &str) -> Option<(Dictionary, Vec<u8>)> {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let table = document.get_key(&page.resources, "XObject");
    let entry = table.as_dict()?.get(name)?.clone();
    let resolved = document.resolve(&entry);
    let stream = resolved.as_stream()?;
    let data = document.decoded_stream_data(stream)?;
    Some((stream.dict.clone(), data.to_vec()))
}

/// The proof that `/RO` was drawn strictly after the removal: the producer's form, unaltered, is
/// what the page names `OverlayRO1`; the page invokes it once; the invocation comes after every
/// operator of the edited content (the text outside the region among them); and it is placed by
/// the translation `at`, which is Table 195's "origin lines up with the lower-left corner of the
/// annotation rectangle".
fn drawn_after_removal(out: &[u8], at: &str) {
    let content = page_content(out);
    let invocation = format!("q 1 0 0 1 {at} cm /OverlayRO1 Do Q");
    assert_eq!(
        content.matches("/OverlayRO1 Do").count(),
        1,
        "the form is invoked once: {content}"
    );
    assert!(
        content.contains(&invocation),
        "the form is placed at {at}: {content}"
    );
    let drawn = content.find(&invocation).expect("asserted above");
    let keep = content
        .find("(KEEP)")
        .expect("the outside text is in the edited content");
    assert!(keep < drawn, "the edited content comes first: {content}");
    assert_eq!(
        content[drawn + invocation.len()..].trim(),
        "",
        "nothing is drawn after the overlay: {content}"
    );
    assert!(
        content[..drawn].trim_end().ends_with('Q'),
        "the edited content is closed in its own q…Q before the form: {content}"
    );
    let (dict, data) = page_xobject(out, "OverlayRO1").expect("the page names the form");
    assert_eq!(
        String::from_utf8_lossy(&data).trim(),
        OVERLAY,
        "the producer's form is carried as it was written, not cut by the removal"
    );
    assert!(
        dict.get("Subtype")
            .and_then(Object::as_name)
            .is_some_and(|name| name.as_bytes() == b"Form")
    );
}

/// Table 195 with `/RO`, `/IC` and `/OverlayText`: the region is removed, and then the producer's
/// `/RO` form is drawn over it at the annotation rectangle's lower-left corner, `(10, 40)`. The
/// table says `/IC` and `/OverlayText` are "ignored if the RO entry is present", so neither is
/// composed and nothing departs (§12.5.6.23, `doc/questions/A842`).
#[test]
fn the_region_is_removed_and_then_the_producers_ro_is_drawn_over_it() {
    let bytes = document("/RO 8 0 R /IC [1 0 0] /OverlayText (NOT DRAWN) /DA (/Helv 9 Tf 0 g)");
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    removed(&bytes, &out);
    drawn_after_removal(&out, "10 40");
    assert!(
        !contains(&out, b"NOT DRAWN"),
        "/OverlayText is not composed"
    );
    assert!(report.departures.is_empty(), "{:?}", report.departures);
}

/// "Its origin" is form space's: §8.10.1's `/Matrix` carries the form's origin to `(e, f)`, so the
/// translation in front of the `Do` is the corner less `(e, f)` — here `(10 - 5, 40 - 7)`. The
/// corner is the lower-left of `/Rect` however its corners are written (§7.9.5).
#[test]
fn the_ro_forms_own_origin_lines_up_with_the_rectangles_lower_left_corner() {
    let bytes = document_with(
        "/Rect [130 66 10 40]",
        "/RO 8 0 R",
        "/Type /XObject /Subtype /Form /BBox [0 0 120 26] /Matrix [1 0 0 1 5 7]",
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    removed(&bytes, &out);
    drawn_after_removal(&out, "5 33");
    assert!(report.departures.is_empty(), "{:?}", report.departures);
}

/// A `/RO` that is not a form `XObject` with a `/BBox` (Table 93) is not drawn, the removal is
/// the same, and the departure names `/RO`.
#[test]
fn a_ro_that_is_no_form_is_not_drawn_and_the_removal_stands() {
    let bytes = document_with(
        "/Rect [10 40 130 66]",
        "/RO 8 0 R",
        "/Type /XObject /Subtype /Form",
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    removed(&bytes, &out);
    assert!(
        !page_content(&out).contains("OverlayRO"),
        "nothing is drawn"
    );
    assert_eq!(report.departures.len(), 1, "{:?}", report.departures);
    assert!(report.departures[0].detail.contains("/RO"));
}

/// A `/RO` form whose references reach the page's original content stream would carry the removed
/// bytes back into the file with it, so it is not drawn: §12.5.6.23's "remove all traces of the
/// specified content" outranks the table's "should be drawn".
#[test]
fn a_ro_that_reaches_the_removed_content_is_not_drawn() {
    let bytes = document_with(
        "/Rect [10 40 130 66]",
        "/RO 8 0 R",
        "/Type /XObject /Subtype /Form /BBox [0 0 120 26] /Kept 4 0 R",
    );
    let (report, out) = redact(&bytes);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    removed(&bytes, &out);
    assert!(
        !page_content(&out).contains("OverlayRO"),
        "nothing is drawn"
    );
    assert_eq!(report.departures.len(), 1, "{:?}", report.departures);
    assert!(report.departures[0].detail.contains("/RO"));
}

/// The overlay is placed in the state the page began in, which the edited content is closed back
/// to — a `q` it leaves open is closed before the `Do` — and a content stream that pops a state it
/// never pushed breaks §8.4.2's balance ("Occurrences of the q and Q operators shall be balanced
/// within a given content stream"), so no state is left to place the form in and it is not drawn.
#[test]
fn the_overlay_is_drawn_in_the_pages_first_state_or_not_at_all() {
    let form = "/Type /XObject /Subtype /Form /BBox [0 0 120 26]";
    let rect = "/Rect [10 40 130 66]";
    let open = document_drawing(&format!("q q\n{CONTENT}"), rect, "/RO 8 0 R", form);
    let (report, out) = redact(&open);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    removed(&open, &out);
    drawn_after_removal(&out, "10 40");
    let content = page_content(&out);
    let drawn = content.find("q 1 0 0 1 10 40 cm").expect("drawn");
    assert!(
        content[..drawn].trim_end().ends_with("Q\nQ\nQ"),
        "the two states left open and the opening one are closed first: {content}"
    );

    let popped = document_drawing(&format!("Q\n{CONTENT}"), rect, "/RO 8 0 R", form);
    let (report, out) = redact(&popped);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    removed(&popped, &out);
    assert!(
        !page_content(&out).contains("OverlayRO"),
        "nothing is drawn"
    );
    assert_eq!(report.departures.len(), 1, "{:?}", report.departures);
    assert!(report.departures[0].detail.contains("§8.4.2"));
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
