//! Independent Annex Q checks. Kept in one file so the coverage matrix does not get mixed into
//! the survey's older tests as requirements are added to it.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use pdf_syntax::{Document, Object, ObjectId};

use super::super::Survey;

/// Opens a small, real PDF with one page, one content stream and the supplied extra objects.
fn page(content: &str, page_entries: &str, objects: &[String]) -> Document {
    use std::fmt::Write as _;

    let stream = format!(
        "<< /Length {} >> stream\n{content}\nendstream",
        content.len()
    );
    let resources = if page_entries.contains("/Resources") {
        ""
    } else {
        "/Resources << >>"
    };
    let page = format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R {resources} {page_entries} >>"
    );
    let mut bodies = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        page,
        stream,
    ];
    bodies.extend_from_slice(objects);

    let mut bytes = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(bytes.len());
        let object_number = index.saturating_add(1);
        let _ = writeln!(bytes, "{object_number} 0 obj {body} endobj");
    }
    let xref = bytes.len();
    let size = bodies.len().saturating_add(1);
    let _ = write!(bytes, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer << /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    let document =
        Document::open(bytes.into_bytes()).expect("the locally assembled PDF fixture is valid");
    assert!(
        !document.was_recovered(),
        "the fixture has a valid cross-reference table"
    );
    document
}

fn ext_gstate(state: &str) -> String {
    format!("/Resources << /ExtGState << /GS0 << {state} >> >> >>")
}

/// Annex Q.2's state tests use dictionary type, both alpha constants use a strict less-than-one
/// bound, and the name form of BM distinguishes Normal from another mode.
#[test]
fn each_q2_graphics_state_predicate_has_a_discriminating_pair() {
    let cases = [
        ("/ca 0.999", true, "nonstroking alpha below one"),
        ("/ca 1", false, "nonstroking alpha at one"),
        ("/CA 0.999", true, "stroking alpha below one"),
        ("/CA 1", false, "stroking alpha at one"),
        ("/BM /Multiply", true, "non-Normal blend mode"),
        ("/BM /Normal", false, "Normal blend mode"),
        ("/BM /Compatible", false, "legacy name for Normal"),
    ];

    for (state, expected, label) in cases {
        let document = page("/GS0 gs 0 0 1 1 re f", &ext_gstate(state), &[]);
        assert_eq!(
            Survey::of(&document).page_is_transparent(0),
            expected,
            "Annex Q.2: {label} ({state})"
        );
    }
}

/// This isolates Q.2's dictionary-type check from the Q.3 transparency group that a valid soft
/// mask would also require. The value test is intentionally at the predicate boundary: the
/// surrounding PDF/A rules report malformed mask dictionaries separately.
#[test]
fn q2_soft_mask_state_requires_a_dictionary_value() {
    for (value, expected) in [("<< /S /Alpha >>", true), ("null", false), ("/None", false)] {
        let document = page("", "", &[format!("<< /SMask {value} >>")]);
        let dict = match document.get(ObjectId::new(5, 0)) {
            Object::Dictionary(dict) => dict,
            other => panic!("the fixture object is not a dictionary: {other:?}"),
        };
        let walk = super::Walk {
            document: &document,
            visited: BTreeMap::default(),
            opened: 0,
            budget: super::super::TOKEN_BUDGET,
            shown_budget: super::super::SHOWN_BUDGET,
            actual_text_budget: super::super::ACTUAL_TEXT_BUDGET,
            marks: Vec::new(),
            seen: BTreeSet::default(),
            page_resources: Arc::default(),
            displacing: None,
            type3_codes: BTreeMap::default(),
            ledger: None,
            survey: Survey::default(),
        };
        assert_eq!(
            walk.is_transparent_state(&dict),
            expected,
            "Annex Q.2 SMask dictionary condition for {value}"
        );
    }
}

/// §11.6.3 requires the first recognised mode. The normal-mode first entry wins even though a
/// later array entry names Multiply.
#[test]
fn q2_blend_mode_arrays_use_the_first_recognised_mode() {
    let document = page(
        "/GS0 gs 0 0 1 1 re f",
        &ext_gstate("/BM [/Normal /Multiply]"),
        &[],
    );
    assert!(
        !Survey::of(&document).page_is_transparent(0),
        "the effective mode is the first recognised entry, /Normal"
    );
}

/// A graphics-state change inside q/Q is restored before the later path is painted. Q.2 asks
/// about the state associated with a rendered element, so the unpainted half-alpha state alone
/// must not make this otherwise opaque page transparent.
#[test]
fn a_q2_transparent_state_restored_before_painting_does_not_mark_the_page() {
    let document = page("q /GS0 gs Q 0 0 1 1 re f", &ext_gstate("/ca 0.5"), &[]);
    assert!(
        !Survey::of(&document).page_is_transparent(0),
        "the only element is painted after Q restores the opaque state"
    );
}

/// Q.2 requires each annotation appearance stream; `/AP /N` can be a state dictionary, not just
/// one stream. Transparency only in an alternate state must still be observed by this method.
#[test]
fn q2_walks_every_appearance_state_in_an_annotation() {
    let opaque =
        "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] /Length 0 >> stream\n\nendstream";
    let marks = "/GS0 gs 0 0 1 1 re f";
    let transparent = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] \
         /Resources << /ExtGState << /GS0 << /CA 0.5 >> >> >> /Length {} >>\n\
         stream\n{marks}\nendstream",
        marks.len()
    );
    let annotation = "<< /Type /Annot /Subtype /Widget /Rect [0 0 1 1] /AP << /N << /On 6 0 R /Off 7 0 R >> >> >>";
    let document = page(
        "",
        "/Annots [5 0 R]",
        &[annotation.to_owned(), opaque.to_owned(), transparent],
    );
    assert!(Survey::of(&document).page_is_transparent(0));
}

/// Errata Collection 3, Issue #51 adds the annotation dictionary's own `/BM` to the transparency
/// method. This route is separate from the form appearance stream.
#[test]
fn q3_erratum_51_counts_annotation_blend_modes() {
    let annotation = "<< /Type /Annot /Subtype /Square /Rect [0 0 1 1] /BM /Multiply >>";
    let document = page("", "/Annots [5 0 R]", &[annotation.to_owned()]);
    let dict = match document.get(ObjectId::new(5, 0)) {
        Object::Dictionary(dict) => dict,
        other => panic!("annotation fixture is not a dictionary: {other:?}"),
    };
    assert!(
        document
            .get_key(&dict, "BM")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"Multiply")
    );
    assert!(
        Survey::of(&document).page_is_transparent(0),
        "Errata Collection 3, Issue #51: /BM /Multiply on an annotation"
    );
}

#[test]
fn q3_erratum_51_normal_or_absent_annotation_blend_mode_is_opaque() {
    for (blend, expected_name) in [("/BM /Normal", Some(b"Normal".as_slice())), ("", None)] {
        let annotation = format!("<< /Type /Annot /Subtype /Square /Rect [0 0 1 1] {blend} >>");
        let document = page("", "/Annots [5 0 R]", &[annotation]);
        let dict = match document.get(ObjectId::new(5, 0)) {
            Object::Dictionary(dict) => dict,
            other => panic!("annotation fixture is not a dictionary: {other:?}"),
        };
        let bm = document.get_key(&dict, "BM");
        let parsed_name = bm.as_name().map(pdf_syntax::Name::as_bytes);
        assert_eq!(parsed_name, expected_name, "fixture BM entry: {blend}");
        assert!(
            !Survey::of(&document).page_is_transparent(0),
            "Errata Collection 3, Issue #51: {blend}"
        );
    }
}

/// Q.3 has two distinct ways to classify a form: a transparency `/Group` marks the page, and the
/// form's own stream is still processed by Q.2 even when it has no group.
#[test]
fn q3_checks_form_group_subtype_and_walks_form_content() {
    let form = |extra: &str, content: &str| {
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 1 1] {extra} /Length {} >>\n\
             stream\n{content}\nendstream",
            content.len()
        )
    };

    let grouped = form("/Group << /S /Transparency >>", "");
    let document = page(
        "/Fm0 Do",
        "/Resources << /XObject << /Fm0 5 0 R >> >>",
        &[grouped],
    );
    assert!(Survey::of(&document).page_is_transparent(0));

    let other_group = form("/Group << /S /Form >>", "");
    let document = page(
        "/Fm0 Do",
        "/Resources << /XObject << /Fm0 5 0 R >> >>",
        &[other_group],
    );
    assert!(!Survey::of(&document).page_is_transparent(0));

    let content_transparency = form(
        "/Resources << /ExtGState << /GS0 << /ca 0.5 >> >> >>",
        "/GS0 gs 0 0 1 1 re f",
    );
    let document = page(
        "/Fm0 Do",
        "/Resources << /XObject << /Fm0 5 0 R >> >>",
        &[content_transparency],
    );
    assert!(Survey::of(&document).page_is_transparent(0));
}

/// Q.4's `SMask` condition is about a stream value, not the key alone or any other object type.
#[test]
fn q4_an_image_smask_counts_only_when_its_value_is_a_stream() {
    let image = |mask: &str| {
        format!(
            "<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray \
             /BitsPerComponent 8 {mask} /Length 1 >>\nstream\n\x00\nendstream"
        )
    };
    for (mask, expected) in [
        ("/SMask 6 0 R", true),
        ("/SMask << /Type /XObject >>", false),
        ("/SMask null", false),
    ] {
        let stream = image(mask);
        let mask_stream = image("");
        let document = page(
            "/Im0 Do",
            "/Resources << /XObject << /Im0 5 0 R >> >>",
            &[stream, mask_stream],
        );
        assert_eq!(
            Survey::of(&document).page_is_transparent(0),
            expected,
            "Annex Q.4 with {mask}"
        );
    }
}

/// Q.5 says to process each Type 3 `CharProc` once text is drawn with that font. A procedure that
/// is present in the dictionary but is not the glyph shown remains in this population.
#[test]
fn q5_processes_all_type3_charprocs_after_text_is_drawn() {
    let opaque = "1 0 0 0 1 1 d1 0 0 1 1 re f";
    let transparent = "/GS0 gs 1 0 0 0 1 1 d1 0 0 1 1 re f";
    let proc = |contents: &str| {
        format!(
            "<< /Length {} >> stream\n{contents}\nendstream",
            contents.len()
        )
    };
    let font = "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1 1] \
                /FontMatrix [1 0 0 1 0 0] /CharProcs << /a 6 0 R /b 7 0 R >> \
                /Encoding << /Differences [97 /a 98 /b] >> /FirstChar 97 /LastChar 98 \
                /Widths [1 1] /Resources << /ExtGState << /GS0 << /BM /Multiply >> >> >> >>";
    let document = page(
        "BT /F1 1 Tf (a) Tj ET",
        "/Resources << /Font << /F1 5 0 R >> >>",
        &[font.to_owned(), proc(opaque), proc(transparent)],
    );
    assert!(
        Survey::of(&document).page_is_transparent(0),
        "the Type 3 font's unshown /b procedure is still processed by Q.5"
    );
}
