//! XFDF's `<annots>`, one fixture per annotation element the XFDF 3.0 text defines.
//!
//! Each fixture under `tests/xfdf/` is a small file written for this test, in the grammar of
//! Adobe's *XML Forms Data Format Specification* 3.0 (chapter 2, *Annotation Elements*, pages 37
//! to 54) with values of the test's own. Each test states which dictionary entries the element's
//! attributes land in and the ISO 32000-2 table that defines each entry, because the key is ISO
//! 32000-2's even where the attribute's name is the XFDF text's (ADR 1297). The XFDF text is cited
//! by chapter and page and paraphrased, never quoted: it is a licensed text held under ADR 0187's
//! discipline (`doc/third-party-data.md`).

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a test's failure is its purpose, and these helpers run outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]

use std::fmt::Write as _;

use pdf_model::forms_data::{FdfAnnotation, FormsData};
use pdf_model::view::ViewState;
use pdf_syntax::{Dictionary, Document, Object};

/// Reads one fixture.
fn read(fixture: &[u8]) -> FormsData {
    pdf_model::xfdf::read(fixture).expect("a well-formed XFDF fixture")
}

/// The first annotation a fixture states, with its dictionary.
fn first(data: &FormsData) -> (&FdfAnnotation, &Dictionary) {
    let annotation = data.annotations.first().expect("one annotation");
    let dict = annotation.dictionary.as_ref().expect("a dictionary");
    (annotation, dict)
}

/// An entry's name, as bytes.
fn name<'a>(dict: &'a Dictionary, key: &str) -> &'a [u8] {
    dict.get(key)
        .and_then(Object::as_name)
        .unwrap_or_else(|| panic!("/{key} is a name: {dict:?}"))
        .as_bytes()
}

/// An entry that is a text string, decoded.
fn text(dict: &Dictionary, key: &str) -> String {
    pdf_syntax::text_string(
        dict.get(key)
            .and_then(Object::as_string)
            .unwrap_or_else(|| panic!("/{key} is a string: {dict:?}")),
    )
}

/// An entry's bytes, for a byte string.
fn bytes<'a>(dict: &'a Dictionary, key: &str) -> &'a [u8] {
    dict.get(key)
        .and_then(Object::as_string)
        .unwrap_or_else(|| panic!("/{key} is a string: {dict:?}"))
}

/// A number, integer or real.
fn number(object: &Object) -> f64 {
    match object {
        Object::Real(value) => *value,
        Object::Integer(value) => f64::from(i32::try_from(*value).expect("a small integer")),
        other => panic!("{other:?} is not a number"),
    }
}

/// An array of numbers.
fn numbers(dict: &Dictionary, key: &str) -> Vec<f64> {
    dict.get(key)
        .and_then(Object::as_array)
        .unwrap_or_else(|| panic!("/{key} is an array: {dict:?}"))
        .iter()
        .map(number)
        .collect()
}

/// A sub-dictionary.
fn sub<'a>(dict: &'a Dictionary, key: &str) -> &'a Dictionary {
    dict.get(key)
        .and_then(Object::as_dict)
        .unwrap_or_else(|| panic!("/{key} is a dictionary: {dict:?}"))
}

/// Whether `owed` names something starting with this.
fn owes(data: &FormsData, start: &str) -> bool {
    data.owed.iter().any(|owed| owed.starts_with(start))
}

/// A one-page document to place annotations on, holding one annotation of its own named
/// `on-the-page` for Table 172's `/IRT` to find.
fn target() -> Vec<u8> {
    let body = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Text /Rect [150 60 170 80] /NM (on-the-page) \
         /Contents (already here) >>\nendobj\n";
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// `<text>` (page 37): Table 166's common entries, Table 172's markup entries and Table 175's
/// own, and a `<popup>` that becomes Table 186's popup annotation.
#[test]
fn a_text_element_carries_the_common_markup_and_text_entries() {
    let data = read(include_bytes!("xfdf/text.xfdf"));
    assert!(data.owed.is_empty(), "{:?}", data.owed);
    assert_eq!(data.annotations.len(), 2, "the note and its popup");
    let (annotation, dict) = first(&data);
    // Table 254's `/Page`, from the `page` attribute.
    assert_eq!(annotation.page, Some(0));
    assert_eq!(name(dict, "Subtype"), b"Text");
    // Table 166.
    assert_eq!(numbers(dict, "Rect"), [20.0, 60.0, 40.0, 80.0]);
    let colour = numbers(dict, "C");
    assert_eq!(colour.len(), 3, "DeviceRGB, by Table 166's element count");
    assert!(colour[0].abs() < 1e-9 && (colour[1] - 1.0).abs() < 1e-9);
    assert!((colour[2] - 128.0 / 255.0).abs() < 1e-9);
    assert_eq!(text(dict, "M"), "D:20260922101500Z");
    // Table 167: Print is bit 3, NoZoom bit 4, NoRotate bit 5.
    assert_eq!(dict.get("F"), Some(&Object::Integer(4 | 8 | 16)));
    assert_eq!(text(dict, "NM"), "note-one");
    assert_eq!(text(dict, "Contents"), "Is this figure current?");
    // Table 166's `/CA`, a number.
    assert_eq!(dict.get("CA").map(number), Some(0.5));
    // Table 172.
    assert_eq!(text(dict, "T"), "Reviewer");
    assert_eq!(text(dict, "CreationDate"), "D:20260922101000Z");
    assert_eq!(text(dict, "Subj"), "A question");
    // Table 175: `/Name` is a name; `/State` and `/StateModel` are text strings there, whatever
    // the XFDF text calls them.
    assert_eq!(name(dict, "Name"), b"Help");
    assert_eq!(text(dict, "State"), "Accepted");
    assert_eq!(text(dict, "StateModel"), "Review");
    // Table 172's `/Popup` and Table 186's `/Parent` are links by position until an import
    // gives both objects numbers.
    assert_eq!(annotation.popup, Some(1));
    let popup = &data.annotations[1];
    assert_eq!(popup.parent, Some(0));
    assert_eq!(popup.subtype.as_deref(), Some("Popup"));
    let popup = popup.dictionary.as_ref().expect("a popup dictionary");
    assert_eq!(name(popup, "Subtype"), b"Popup");
    assert_eq!(popup.get("Open"), Some(&Object::Boolean(true)));
    assert_eq!(numbers(popup, "Rect"), [120.0, 40.0, 190.0, 95.0]);
    assert!(dict.get("Popup").is_none() && popup.get("Parent").is_none());
}

/// `inreplyto` and `replyType` (pages 72 and 73): Table 172's `/IRT` in the FDF file's own form, a
/// text string of the replied-to `/NM`, which an import turns into a reference on the same page.
#[test]
fn a_reply_is_linked_to_the_annotation_its_name_names_on_the_same_page() {
    let data = read(include_bytes!("xfdf/text-reply.xfdf"));
    let answer = data.annotations[1].dictionary.as_ref().expect("the answer");
    assert_eq!(text(answer, "IRT"), "question");
    // Table 172's `/RT`: `Group`, from the text's `group`.
    assert_eq!(name(answer, "RT"), b"Group");

    let document = Document::open(target()).expect("the target is a valid PDF");
    let mut view = ViewState::of(&document);
    let outcome = view.import(&document, &data);
    assert_eq!(outcome.annotations, 4, "{outcome:?}");
    let added = view.additions();
    let (question, answer) = (&added[0], &added[1]);
    assert_eq!(
        answer.dict.get("IRT"),
        Some(&Object::Reference(question.id)),
        "one annotation of the same import"
    );
    let late = &added[2];
    assert!(
        matches!(late.dict.get("IRT"), Some(Object::Reference(id)) if id.number == 5),
        "the page's own annotation: {:?}",
        late.dict
    );
    // One that names an annotation the page has nowhere is refused by name, `/RT` with it.
    let lost = &added[3];
    assert!(lost.dict.get("IRT").is_none() && lost.dict.get("RT").is_none());
    assert_eq!(outcome.refused.len(), 1, "{:?}", outcome.refused);
    assert!(
        outcome.refused[0].contains("\"nowhere\""),
        "{:?}",
        outcome.refused
    );
}

/// `<highlight>` (page 38): Table 182's `/QuadPoints` from `coords`, and a `<contents-richtext>`
/// holding markup, which is Table 172's `/RC` (chapter 1, page 30).
#[test]
fn a_highlight_element_carries_its_quadrilaterals_and_its_rich_text() {
    let data = read(include_bytes!("xfdf/highlight.xfdf"));
    assert!(data.owed.is_empty(), "{:?}", data.owed);
    let (_, dict) = first(&data);
    assert_eq!(name(dict, "Subtype"), b"Highlight");
    assert_eq!(
        numbers(dict, "QuadPoints"),
        [10.0, 22.0, 110.0, 22.0, 10.0, 10.0, 110.0, 10.0]
    );
    let rich = text(dict, "RC");
    assert!(
        rich.starts_with("<body") && rich.contains("<b>here</b>"),
        "{rich:?}"
    );
    assert!(dict.get("Contents").is_none());
}

/// `<underline>` (page 39): two quadrilaterals, Table 172's `/IT`, and a `<contents-richtext>`
/// holding plain text, which the text maps to `/Contents` rather than `/RC` (page 30).
#[test]
fn an_underline_element_with_plain_rich_text_carries_contents() {
    let data = read(include_bytes!("xfdf/underline.xfdf"));
    assert!(data.owed.is_empty(), "{:?}", data.owed);
    let (_, dict) = first(&data);
    assert_eq!(name(dict, "Subtype"), b"Underline");
    assert_eq!(numbers(dict, "QuadPoints").len(), 16);
    assert_eq!(name(dict, "IT"), b"Emphasis");
    assert_eq!(text(dict, "Contents"), "Plain words, not markup");
    assert!(dict.get("RC").is_none());
}

/// `<strikeout>` (page 40): Table 171 spells the subtype `StrikeOut`.
#[test]
fn a_strikeout_element_is_a_strikeout_annotation() {
    let data = read(include_bytes!("xfdf/strikeout.xfdf"));
    assert!(data.owed.is_empty(), "{:?}", data.owed);
    let (_, dict) = first(&data);
    assert_eq!(name(dict, "Subtype"), b"StrikeOut");
    assert_eq!(numbers(dict, "QuadPoints").len(), 8);
    assert_eq!(text(dict, "T"), "Editor");
}

/// `<squiggly>` (page 41).
#[test]
fn a_squiggly_element_is_a_squiggly_annotation() {
    let data = read(include_bytes!("xfdf/squiggly.xfdf"));
    assert!(data.owed.is_empty(), "{:?}", data.owed);
    let (_, dict) = first(&data);
    assert_eq!(name(dict, "Subtype"), b"Squiggly");
    assert_eq!(numbers(dict, "QuadPoints").len(), 8);
    assert_eq!(numbers(dict, "C"), [1.0, 0.0, 0.0]);
}

/// `<line>` (pages 41 to 43): Table 178's entries, two of them made of two attributes each, and
/// Table 168's `/BS`.
#[test]
fn a_line_element_carries_table_178_and_its_border_style() {
    let data = read(include_bytes!("xfdf/line.xfdf"));
    assert!(data.owed.is_empty(), "{:?}", data.owed);
    let (_, dict) = first(&data);
    assert_eq!(name(dict, "Subtype"), b"Line");
    // `/L` is `start` then `end`; `/LE` is `head` then `tail`.
    assert_eq!(numbers(dict, "L"), [10.0, 50.0, 190.0, 50.0]);
    let ends: Vec<&[u8]> = dict
        .get("LE")
        .and_then(Object::as_array)
        .expect("/LE")
        .iter()
        .filter_map(Object::as_name)
        .map(pdf_syntax::Name::as_bytes)
        .collect();
    assert_eq!(ends, [&b"OpenArrow"[..], &b"Butt"[..]]);
    assert_eq!(numbers(dict, "IC"), [0.0, 0.0, 1.0]);
    assert_eq!(dict.get("LL").map(number), Some(12.0));
    assert_eq!(dict.get("LLE").map(number), Some(3.0));
    assert_eq!(dict.get("LLO").map(number), Some(2.0));
    assert_eq!(dict.get("Cap"), Some(&Object::Boolean(true)));
    assert_eq!(name(dict, "CP"), b"Top");
    // `/CO` defaults to no offset, so the vertical half the file left out is zero.
    assert_eq!(numbers(dict, "CO"), [4.0, 0.0]);
    assert_eq!(name(dict, "IT"), b"LineDimension");
    let border = sub(dict, "BS");
    assert_eq!(border.get("W").map(number), Some(2.0));
    assert_eq!(numbers(border, "D"), [3.0, 1.0]);
    assert_eq!(name(border, "S"), b"D");
}

/// `<circle>` (pages 43 and 44): Table 180's `/IC` and `/RD`, and `style="cloudy"`, which is
/// Table 169's `/BE` rather than Table 168's `/BS` (page 84).
#[test]
fn a_circle_element_carries_its_interior_colour_and_border_effect() {
    let data = read(include_bytes!("xfdf/circle.xfdf"));
    assert!(data.owed.is_empty(), "{:?}", data.owed);
    let (_, dict) = first(&data);
    assert_eq!(name(dict, "Subtype"), b"Circle");
    assert_eq!(numbers(dict, "IC").len(), 3);
    assert_eq!(numbers(dict, "RD"), [2.0, 2.0, 2.0, 2.0]);
    assert_eq!(sub(dict, "BS").get("W").map(number), Some(1.5));
    let effect = sub(dict, "BE");
    assert_eq!(name(effect, "S"), b"C");
    assert_eq!(effect.get("I").map(number), Some(1.0));
}

/// `<square>` (pages 44 and 45): an empty `interior-color`, which the text makes the transparent
/// default (page 77), is an absent `/IC`.
#[test]
fn a_square_element_with_an_empty_interior_colour_states_none() {
    let data = read(include_bytes!("xfdf/square.xfdf"));
    assert!(data.owed.is_empty(), "{:?}", data.owed);
    let (_, dict) = first(&data);
    assert_eq!(name(dict, "Subtype"), b"Square");
    assert!(dict.get("IC").is_none());
    assert_eq!(numbers(dict, "RD"), [1.0, 1.0, 1.0, 1.0]);
    let border = sub(dict, "BS");
    assert_eq!(numbers(border, "D"), [4.0, 2.0]);
    assert_eq!(name(border, "S"), b"D");
}

/// `<caret>` (page 45): Table 183's `/RD` and `/Sy`; the `<defaultappearance>` the text allows
/// under it is refused, because Table 183 states no `/DA`.
#[test]
fn a_caret_element_carries_table_183_and_refuses_a_default_appearance() {
    let data = read(include_bytes!("xfdf/caret.xfdf"));
    let (_, dict) = first(&data);
    assert_eq!(name(dict, "Subtype"), b"Caret");
    assert_eq!(numbers(dict, "RD"), [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(name(dict, "Sy"), b"P");
    assert!(dict.get("DA").is_none());
    assert!(
        owes(&data, "<defaultappearance> under <caret>"),
        "{:?}",
        data.owed
    );
}

/// `<polygon>` (page 46): Table 181's `/Vertices` from `<vertices>`, and its dimension intent
/// spelled as Table 181 spells it.
#[test]
fn a_polygon_element_carries_its_vertices_and_table_181s_intent() {
    let data = read(include_bytes!("xfdf/polygon.xfdf"));
    assert!(data.owed.is_empty(), "{:?}", data.owed);
    let (_, dict) = first(&data);
    assert_eq!(name(dict, "Subtype"), b"Polygon");
    assert_eq!(
        numbers(dict, "Vertices"),
        [10.0, 10.0, 90.0, 10.0, 50.0, 90.0]
    );
    assert_eq!(numbers(dict, "IC"), [0.0, 1.0, 0.0]);
    assert_eq!(name(dict, "IT"), b"PolygonDimension");
    assert_eq!(sub(dict, "BE").get("I").map(number), Some(2.0));
}

/// `<polyline>` (page 47): Table 171 spells the subtype `PolyLine`; `/LE` defaults its tail to
/// `/None`; and the border effect the text lists for it is refused, because Table 181 makes
/// `/BE` meaningful only for a polygon.
#[test]
fn a_polyline_element_is_spelled_as_table_171_spells_it() {
    let data = read(include_bytes!("xfdf/polyline.xfdf"));
    let (annotation, dict) = first(&data);
    assert_eq!(annotation.subtype.as_deref(), Some("PolyLine"));
    assert_eq!(name(dict, "Subtype"), b"PolyLine");
    assert_eq!(numbers(dict, "Vertices").len(), 6);
    assert_eq!(name(dict, "IT"), b"PolyLineDimension");
    let ends = dict.get("LE").and_then(Object::as_array).expect("/LE");
    assert_eq!(ends.len(), 2);
    assert!(dict.get("BE").is_none());
    assert!(owes(&data, "style=\"cloudy\""), "{:?}", data.owed);
}

/// `<stamp>` (page 48): Table 184's `/Name`; `rotation`, for which no annotation table of ISO
/// 32000-2 states a `/Rotate`, and `<appearance>`, whose decoded bytes the text never describes,
/// are both refused by name.
#[test]
fn a_stamp_element_carries_its_icon_and_refuses_what_has_nowhere_to_go() {
    let data = read(include_bytes!("xfdf/stamp.xfdf"));
    let (annotation, dict) = first(&data);
    assert_eq!(name(dict, "Subtype"), b"Stamp");
    assert_eq!(name(dict, "Name"), b"Approved");
    assert!(dict.get("Rotate").is_none() && dict.get("AP").is_none());
    assert!(owes(&data, "rotation:"), "{:?}", data.owed);
    assert!(owes(&data, "<appearance>:"), "{:?}", data.owed);
    // The popup states no page of its own and takes its parent's.
    assert_eq!(data.annotations[1].page, annotation.page);
}

/// `<ink>` (page 49): Table 185's `/InkList`, one array per `<gesture>` (page 64).
#[test]
fn an_ink_element_carries_one_path_per_gesture() {
    let data = read(include_bytes!("xfdf/ink.xfdf"));
    assert!(data.owed.is_empty(), "{:?}", data.owed);
    let (_, dict) = first(&data);
    assert_eq!(name(dict, "Subtype"), b"Ink");
    let paths: Vec<Vec<f64>> = dict
        .get("InkList")
        .and_then(Object::as_array)
        .expect("/InkList")
        .iter()
        .map(|path| {
            path.as_array()
                .expect("a path")
                .iter()
                .map(number)
                .collect()
        })
        .collect();
    assert_eq!(
        paths,
        [
            vec![10.0, 10.0, 20.0, 30.0, 30.0, 20.0],
            vec![60.0, 60.0, 70.0, 80.0]
        ]
    );
    assert_eq!(sub(dict, "BS").get("W").map(number), Some(2.0));
}

/// `<freetext>` (page 50): Table 177's `/DA`, `/DS`, `/Q`, `/IT` and `/RC`, and the legacy
/// `<border>` the text maps onto both `/Border` and `/BS` (page 29).
#[test]
fn a_freetext_element_carries_table_177() {
    let data = read(include_bytes!("xfdf/freetext.xfdf"));
    assert!(data.owed.is_empty(), "{:?}", data.owed);
    let (_, dict) = first(&data);
    assert_eq!(name(dict, "Subtype"), b"FreeText");
    assert_eq!(bytes(dict, "DA"), b"/Helv 10 Tf 0 0 1 rg");
    assert_eq!(text(dict, "DS"), "font: Helvetica 10pt; color:#0000FF");
    assert_eq!(dict.get("Q"), Some(&Object::Integer(1)));
    assert_eq!(name(dict, "IT"), b"FreeTextCallout");
    assert!(text(dict, "RC").contains("Typed on the page"));
    assert_eq!(text(dict, "Contents"), "Typed on the page");
    assert_eq!(numbers(dict, "Border"), [0.0, 0.0, 2.0]);
    assert_eq!(sub(dict, "BS").get("W").map(number), Some(2.0));
}

/// `<fileattachment>` (page 51): Table 187's `/FS`, a file specification whose `/EF` holds the
/// `<data>` as §7.11.4's embedded file stream with Table 44's and Table 45's entries.
#[test]
fn a_fileattachment_element_carries_its_file() {
    let data = read(include_bytes!("xfdf/fileattachment.xfdf"));
    assert!(data.owed.is_empty(), "{:?}", data.owed);
    let (_, dict) = first(&data);
    assert_eq!(name(dict, "Subtype"), b"FileAttachment");
    assert_eq!(name(dict, "Name"), b"Paperclip");
    assert_eq!(text(dict, "Contents"), "The notes");
    let spec = sub(dict, "FS");
    assert_eq!(bytes(spec, "F"), b"notes.txt");
    assert_eq!(text(spec, "UF"), "notes.txt");
    let Some(Object::Stream(file)) = sub(spec, "EF").get("F") else {
        panic!("an embedded file stream: {spec:?}");
    };
    assert_eq!(&*file.data, b"hello");
    assert_eq!(name(&file.dict, "Type"), b"EmbeddedFile");
    assert_eq!(name(&file.dict, "Subtype"), b"text/plain");
    let params = sub(&file.dict, "Params");
    assert_eq!(params.get("Size"), Some(&Object::Integer(5)));
    assert_eq!(text(params, "CreationDate"), "D:20260901000000Z");
    assert_eq!(text(params, "ModDate"), "D:20260902000000Z");
}

/// `<sound>` (page 52): Table 188's `/Sound`, Table 305's sound object with `/R`, `/B`, `/C` and
/// `/E`, from `<data>` in the text's filtered ASCII form (page 31).
#[test]
fn a_sound_element_carries_its_sound_object() {
    let data = read(include_bytes!("xfdf/sound.xfdf"));
    assert!(data.owed.is_empty(), "{:?}", data.owed);
    let (_, dict) = first(&data);
    assert_eq!(name(dict, "Subtype"), b"Sound");
    assert_eq!(name(dict, "Name"), b"Mic");
    let Some(Object::Stream(sound)) = dict.get("Sound") else {
        panic!("a sound object: {dict:?}");
    };
    assert_eq!(&*sound.data, b"ABCD");
    assert_eq!(sound.dict.get("R").map(number), Some(8000.0));
    assert_eq!(sound.dict.get("B"), Some(&Object::Integer(8)));
    assert_eq!(sound.dict.get("C"), Some(&Object::Integer(1)));
    assert_eq!(name(&sound.dict, "E"), b"Signed");
}

/// `<link>` (page 53): read, and refused by the import, because Table 246 excludes a Link from an
/// FDF file's annotations and the text maps `<annots>` onto that array (page 37).
#[test]
fn a_link_element_is_refused_by_the_import_by_name() {
    let data = read(include_bytes!("xfdf/link.xfdf"));
    let (annotation, dict) = first(&data);
    assert_eq!(annotation.subtype.as_deref(), Some("Link"));
    // Table 176's `/H` and `/QuadPoints`, which need no page lookup.
    assert_eq!(name(dict, "H"), b"O");
    assert_eq!(numbers(dict, "QuadPoints").len(), 8);
    assert!(
        owes(&data, "<link>'s Dest, OnActivation"),
        "{:?}",
        data.owed
    );

    let document = Document::open(target()).expect("the target is a valid PDF");
    let mut view = ViewState::of(&document);
    let outcome = view.import(&document, &data);
    assert_eq!(outcome.annotations, 0);
    assert!(
        outcome
            .refused
            .iter()
            .any(|refused| refused.contains("Link")),
        "{:?}",
        outcome.refused
    );
}

/// `<redact>` (pages 53 and 54): Table 195's entries; `page` and `rect`, which the element's own
/// list leaves out and Tables 254 and 166 require; and `<overlayappearance>`, refused because
/// `/RO` is a form `XObject` the text states only as a text string.
#[test]
fn a_redact_element_carries_table_195() {
    let data = read(include_bytes!("xfdf/redact.xfdf"));
    let (annotation, dict) = first(&data);
    assert_eq!(annotation.page, Some(0));
    assert_eq!(name(dict, "Subtype"), b"Redact");
    assert_eq!(numbers(dict, "QuadPoints").len(), 8);
    assert_eq!(numbers(dict, "IC"), [0.0, 0.0, 0.0]);
    assert_eq!(text(dict, "OverlayText"), "REMOVED");
    assert_eq!(dict.get("Repeat"), Some(&Object::Boolean(true)));
    assert_eq!(dict.get("Q"), Some(&Object::Integer(2)));
    assert_eq!(bytes(dict, "DA"), b"/Helv 8 Tf 1 g");
    assert!(dict.get("RO").is_none());
    assert!(owes(&data, "<overlayappearance>:"), "{:?}", data.owed);
}

/// `<projection>` (page 54): §12.5.6.24 gives it Table 166's and Table 172's entries and no
/// others, so its `rotation` is refused like every other.
#[test]
fn a_projection_element_carries_the_markup_entries() {
    let data = read(include_bytes!("xfdf/projection.xfdf"));
    let (_, dict) = first(&data);
    assert_eq!(name(dict, "Subtype"), b"Projection");
    assert_eq!(text(dict, "Subj"), "A measurement");
    assert_eq!(text(dict, "CreationDate"), "D:20260922000000Z");
    assert!(dict.get("Rotate").is_none());
    assert!(owes(&data, "rotation:"), "{:?}", data.owed);
}

/// An element missing an entry ISO 32000-2 requires of its subtype is not placed, and the
/// sentence says which: Table 166's `/Rect` for every annotation, Table 182's `/QuadPoints` for a
/// text markup one, Table 254's `/Page` for every annotation of an FDF file.
#[test]
fn an_element_missing_a_required_entry_is_named_and_not_placed() {
    let data = read(
        b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
          <xfdf xmlns=\"http://ns.adobe.com/xfdf/\" xml:space=\"preserve\"><annots>\
          <text page=\"0\"/>\
          <highlight page=\"0\" rect=\"0,0,10,10\"/>\
          <square rect=\"0,0,10,10\"/>\
          </annots></xfdf>",
    );
    assert!(data.annotations.is_empty(), "{:?}", data.annotations);
    assert!(
        owes(&data, "<annots>: an annotation with no rect"),
        "{:?}",
        data.owed
    );
    assert!(owes(&data, "<highlight> without coords"), "{:?}", data.owed);
    assert!(
        owes(&data, "<annots>: an annotation with no page"),
        "{:?}",
        data.owed
    );
}

/// Placed and saved: the annotation and its popup become two objects of §7.5.6's update, whose
/// Table 172 `/Popup` and Table 186 `/Parent` refer to each other, and the attachment's embedded
/// file stream is an indirect object as §7.3.8.1 requires of every stream.
#[test]
fn a_placed_annotation_and_its_popup_are_saved_as_objects_that_name_each_other() {
    let document = Document::open(target()).expect("the target is a valid PDF");
    let mut view = ViewState::of(&document);
    let mut data = read(include_bytes!("xfdf/text.xfdf"));
    data.annotations
        .extend(read(include_bytes!("xfdf/fileattachment.xfdf")).annotations);
    let outcome = view.import(&document, &data);
    assert_eq!(outcome.annotations, 3, "{outcome:?}");
    assert!(outcome.refused.is_empty(), "{:?}", outcome.refused);

    let saved = view.save(&document).expect("the target can be updated");
    let saved = Document::open(saved.bytes).expect("the update is a valid PDF");
    let pages = pdf_model::Pages::new(&saved);
    let page = pages.get(0).expect("page one");
    let annots = saved.get_key(&page.dict, "Annots");
    let annots: Vec<Dictionary> = annots
        .as_array()
        .expect("/Annots")
        .iter()
        .filter_map(|entry| saved.resolve(entry).as_dict().cloned())
        .collect();
    assert_eq!(
        annots.len(),
        4,
        "the page's own annotation and the three placed"
    );
    let note = &annots[1];
    let popup_id = note
        .get("Popup")
        .and_then(Object::as_reference)
        .expect("/Popup");
    let popup = saved.get(popup_id);
    let popup = popup.as_dict().expect("the popup annotation");
    assert_eq!(name(popup, "Subtype"), b"Popup");
    assert!(popup.get("Parent").and_then(Object::as_reference).is_some());
    let attachment = &annots[3];
    let spec = saved.resolve(attachment.get("FS").expect("/FS"));
    let spec = spec.as_dict().expect("a file specification");
    let embedded = saved.resolve(spec.get("EF").expect("/EF"));
    let file = embedded
        .as_dict()
        .and_then(|ef| ef.get("F"))
        .expect("/EF /F");
    assert!(
        file.as_reference().is_some(),
        "an indirect stream: {file:?}"
    );
    let Object::Stream(file) = saved.resolve(file) else {
        panic!("the embedded file is a stream");
    };
    let decoded = saved
        .decoded_stream_data(&file)
        .expect("the embedded file decodes");
    assert_eq!(&*decoded, b"hello");
}
