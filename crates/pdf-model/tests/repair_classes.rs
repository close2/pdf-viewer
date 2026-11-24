//! `pdf_model::repair`'s classes reach the document's one channel where the reader makes them,
//! for a viewer as much as for a validator (ADR 3477, ADR 3541).
//!
//! Each fixture is one page whose single departure is the one named, beside a twin that states
//! the same page as ISO 32000-2 asks; the twin logs nothing, the damaged page logs its class at
//! the site the class belongs to.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that does not open should fail loudly, and every offset is a \
              few hundred bytes into a file built here"
)]

use std::fmt::Write as _;

use pdf_model::repair;
use pdf_model::{Pages, content};
use pdf_syntax::{Document, ObjectId, RepairClass, Site};

/// A whole file from object bodies, numbered from 1, with an exact cross-reference table.
fn file(objects: &[String]) -> Document {
    let mut bytes: Vec<u8> = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
    }
    let xref = bytes.len();
    let size = objects.len() + 1;
    let mut table = format!("xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(table, "{offset:010} 00000 n ");
    }
    let _ = write!(
        table,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    bytes.extend_from_slice(table.as_bytes());
    Document::open(bytes).expect("the fixture opens")
}

/// One page, its dictionary carrying `extra` and its content stream `operators`.
fn page(operators: &str, extra: &str) -> Document {
    file(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R \
             /Resources << >> {extra} >>"
        ),
        format!(
            "<< /Length {} >>\nstream\n{operators}\nendstream",
            operators.len()
        ),
    ])
}

const PAGE: ObjectId = ObjectId::new(3, 0);

/// The page read and drawn, and the classes its document logged, with their sites.
fn logged(document: &Document) -> Vec<(&'static str, Site)> {
    let pages = Pages::new(document);
    let page = pages.get(0).expect("the page is read");
    let _ = content::interpret(document, &page);
    document
        .repairs()
        .into_iter()
        .map(|repair| (repair.class.name, repair.at))
        .collect()
}

fn has(document: &Document, class: &RepairClass, at: Site) {
    let log = logged(document);
    assert!(
        log.contains(&(class.name, at)),
        "{} at {at:?} is in the log, which holds {log:?}",
        class.name
    );
}

/// A page stating every entry as Table 31 asks needs no repair.
#[test]
fn a_well_formed_page_logs_nothing() {
    assert_eq!(logged(&page("0 0 m 1 1 l S", "")), []);
}

/// Table 31: `/Rotate` "shall be a multiple of 90".
#[test]
fn a_rotation_of_forty_five_is_logged_at_the_page() {
    has(
        &page("0 0 m 1 1 l S", "/Rotate 45"),
        &repair::ROTATE_NOT_A_MULTIPLE_OF_NINETY,
        Site::Object(PAGE),
    );
}

/// Table 31: `/MediaBox` is a rectangle.
#[test]
fn a_media_box_that_is_a_name_is_logged_at_the_page() {
    let document = file(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox /Letter /Contents 4 0 R /Resources << >> >>"
            .to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
    ]);
    has(
        &document,
        &repair::MEDIA_BOX_SUBSTITUTED,
        Site::Object(PAGE),
    );
}

/// §8.4.2: "Occurrences of the q and Q operators shall be balanced within a given content
/// stream". The interpreter ignores the `Q` and logs it at the page's content.
#[test]
fn a_restore_with_nothing_saved_is_logged_at_the_content() {
    has(
        &page("Q 0 0 m 1 1 l S", ""),
        &repair::RESTORE_WITH_NOTHING_SAVED,
        Site::Content(Some(PAGE)),
    );
}

/// §8.6.8: three `sc` operands under `DeviceGray`.
#[test]
fn colour_operands_that_do_not_fit_are_logged_at_the_content() {
    has(
        &page("/DeviceGray cs 1 0 0 sc 0 0 m 1 1 l S", ""),
        &repair::COLOUR_OPERANDS_DO_NOT_FIT_THE_SPACE,
        Site::Content(Some(PAGE)),
    );
}

/// The content reader a viewer draws through logs what lexing needed, so the viewer is not
/// silent about a string that never ends.
#[test]
fn the_page_reader_logs_its_lexical_repairs() {
    has(
        &page("0 0 m 1 1 l S (open", ""),
        &pdf_syntax::repair::STRING_UNTERMINATED,
        Site::Content(Some(PAGE)),
    );
}

/// §7.8.2: "operands shall not be left over when an operator finishes execution". The `5` in
/// front of `m`'s two is dropped, and logged at the page's content.
#[test]
fn operands_left_over_are_logged_at_the_content() {
    has(
        &page("5 0 0 m 1 1 l S", ""),
        &repair::OPERANDS_LEFT_OVER,
        Site::Content(Some(PAGE)),
    );
}

/// §7.8.2: `BX` and `EX` "shall occur in pairs" — an `EX` ending nothing, and a `BX` left open.
#[test]
fn unbalanced_compatibility_sections_are_logged_at_the_content() {
    for operators in ["EX 0 0 m 1 1 l S", "BX 0 0 m 1 1 l S"] {
        has(
            &page(operators, ""),
            &repair::COMPATIBILITY_SECTIONS_UNBALANCED,
            Site::Content(Some(PAGE)),
        );
    }
    assert_eq!(logged(&page("BX 0 0 m 1 1 l S EX", "")), []);
}

/// Table 352: `BMC` is "terminated by a balancing EMC operator" — an `EMC` balancing nothing,
/// and a sequence left open.
#[test]
fn unbalanced_marked_content_is_logged_at_the_content() {
    for operators in ["EMC 0 0 m 1 1 l S", "/Span BMC 0 0 m 1 1 l S"] {
        has(
            &page(operators, ""),
            &repair::MARKED_CONTENT_UNBALANCED,
            Site::Content(Some(PAGE)),
        );
    }
    assert_eq!(logged(&page("/Span BMC 0 0 m 1 1 l S EMC", "")), []);
}

/// §14.6.2: a named property list is "a named resource in the Properties subdictionary"; this
/// page's resources hold none.
#[test]
fn a_property_list_the_resources_do_not_hold_is_logged_at_the_content() {
    has(
        &page("/OC /Missing BDC 0 0 m 1 1 l S EMC", ""),
        &repair::PROPERTY_LIST_UNRESOLVED,
        Site::Content(Some(PAGE)),
    );
}

/// One page whose resources hold `/GS0` as `state` and `/Fm0` as a form stating `form`.
fn page_with_resources(operators: &str, state: &str, form: &str) -> Document {
    file(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R \
             /Resources << /ExtGState << /GS0 {state} >> /XObject << /Fm0 5 0 R >> >> >>"
        ),
        format!(
            "<< /Length {} >>\nstream\n{operators}\nendstream",
            operators.len()
        ),
        format!(
            "<< /Type /XObject /Subtype /Form {form} /Length 13 >>\nstream\n0 0 m 1 1 l S\nendstream"
        ),
    ])
}

/// Table 57: `/LW` is a number and `/SA` a boolean; `/TR` is a function, an array of four or
/// `/Identity`. Each of the wrong type is ignored, and logged at the content that set it.
#[test]
fn graphics_state_entries_of_the_wrong_type_are_logged_at_the_content() {
    let box_ = "/BBox [0 0 1 1]";
    has(
        &page_with_resources("/GS0 gs 0 0 m 1 1 l S", "<< /LW /Thick >>", box_),
        &repair::GRAPHICS_STATE_ENTRY_OF_THE_WRONG_TYPE,
        Site::Content(Some(PAGE)),
    );
    has(
        &page_with_resources("/GS0 gs 0 0 m 1 1 l S", "<< /SA 1 >>", box_),
        &repair::GRAPHICS_STATE_ENTRY_OF_THE_WRONG_TYPE,
        Site::Content(Some(PAGE)),
    );
    has(
        &page_with_resources("/GS0 gs 0 0 m 1 1 l S", "<< /TR /Inverted >>", box_),
        &repair::TRANSFER_FUNCTION_UNREADABLE,
        Site::Content(Some(PAGE)),
    );
    assert_eq!(
        logged(&page_with_resources(
            "/GS0 gs 0 0 m 1 1 l S",
            "<< /LW 2 /SA true /TR /Identity >>",
            box_
        )),
        []
    );
}

/// Table 93: `/BBox` is required and four numbers; `/Matrix` is six. A form missing the first
/// and stating five numbers for the second is drawn, and both are logged at the content that
/// invoked it.
#[test]
fn a_form_with_no_bbox_and_a_short_matrix_is_logged_at_the_content() {
    let document = page_with_resources("/Fm0 Do", "<< >>", "/Matrix [1 0 0 1 0]");
    has(
        &document,
        &repair::FORM_BBOX_UNUSABLE,
        Site::Content(Some(PAGE)),
    );
    has(
        &document,
        &repair::FORM_MATRIX_MALFORMED,
        Site::Content(Some(PAGE)),
    );
    assert_eq!(
        logged(&page_with_resources(
            "/Fm0 Do",
            "<< >>",
            "/BBox [0 0 1 1] /Matrix [1 0 0 1 0 0]"
        )),
        []
    );
}

/// Table 93: a form's `/Resources` is a dictionary. An integer there is read as absent, the form's
/// names then looked up in the page's, and logged at the content that invoked it.
#[test]
fn a_form_with_resources_of_the_wrong_type_is_logged_at_the_content() {
    let box_ = "/BBox [0 0 1 1]";
    has(
        &page_with_resources("/Fm0 Do", "<< >>", &format!("{box_} /Resources 7")),
        &repair::FORM_RESOURCES_NOT_A_DICTIONARY,
        Site::Content(Some(PAGE)),
    );
    assert_eq!(
        logged(&page_with_resources(
            "/Fm0 Do",
            "<< >>",
            &format!("{box_} /Resources << >>")
        )),
        []
    );
}

/// ISO 32000-2 §11.3.7.2: "All of the shape and opacity inputs shall have values in the range
/// 0.0 to 1.0 (inclusive)". A `/CA` of 2 is read as 1, and logged.
#[test]
fn an_alpha_constant_out_of_range_is_logged_at_the_content() {
    let box_ = "/BBox [0 0 1 1]";
    has(
        &page_with_resources("/GS0 gs 0 0 m 1 1 l S", "<< /CA 2 >>", box_),
        &repair::ALPHA_CONSTANT_OUT_OF_RANGE,
        Site::Content(Some(PAGE)),
    );
    assert_eq!(
        logged(&page_with_resources(
            "/GS0 gs 0 0 m 1 1 l S",
            "<< /CA 0.5 /ca 1 >>",
            box_
        )),
        []
    );
}

/// Table 57's `/HT` is a dictionary, a stream or the name `/Default`, and Table 128's
/// `/TransferFunction` a function or `/Identity`. Each of the wrong kind is logged.
#[test]
fn an_unreadable_halftone_and_its_transfer_function_are_logged_at_the_content() {
    let box_ = "/BBox [0 0 1 1]";
    has(
        &page_with_resources("/GS0 gs 0 0 m 1 1 l S", "<< /HT 5 >>", box_),
        &repair::HALFTONE_UNREADABLE,
        Site::Content(Some(PAGE)),
    );
    has(
        &page_with_resources(
            "/GS0 gs 0 0 m 1 1 l S",
            "<< /HT << /HalftoneType 1 /TransferFunction /Inverted >> >>",
            box_,
        ),
        &repair::HALFTONE_TRANSFER_FUNCTION_UNREADABLE,
        Site::Content(Some(PAGE)),
    );
    assert_eq!(
        logged(&page_with_resources(
            "/GS0 gs 0 0 m 1 1 l S",
            "<< /HT << /HalftoneType 1 /TransferFunction /Identity >> >>",
            box_
        )),
        []
    );
}

/// A document whose catalog states `extra`.
fn catalog(extra: &str) -> Document {
    file(&[
        format!("<< /Type /Catalog /Pages 2 0 R {extra} >>"),
        "<< /Type /Pages /Kids [] /Count 0 >>".to_owned(),
    ])
}

fn logged_alone(document: &Document) -> Vec<(&'static str, Site)> {
    document
        .repairs()
        .into_iter()
        .map(|repair| (repair.class.name, repair.at))
        .collect()
}

/// Table 29's `/PageLayout` names one of six layouts and Table 147's `/HideToolbar` is a boolean:
/// a name outside the table and a value of another type are read as the defaults, and logged at
/// the catalog. `/PrintScaling`'s unrecognised value is the clause's own `AppDefault`.
#[test]
fn display_entries_and_viewer_preferences_of_the_wrong_kind_are_logged_at_the_catalog() {
    let document = catalog("/PageLayout /Sideways");
    let _ = pdf_model::viewer_preferences::Opening::read(&document);
    assert_eq!(
        logged_alone(&document),
        [(
            repair::CATALOG_DISPLAY_ENTRY_UNUSABLE.name,
            Site::Object(ObjectId::new(1, 0))
        )]
    );
    let document = catalog("/ViewerPreferences << /HideToolbar 1 >>");
    let _ = pdf_model::viewer_preferences::ViewerPreferences::read(&document);
    assert_eq!(
        logged_alone(&document),
        [(
            repair::VIEWER_PREFERENCE_UNUSABLE.name,
            Site::Object(ObjectId::new(1, 0))
        )]
    );
    let document = catalog(
        "/PageLayout /TwoPageLeft /ViewerPreferences << /HideToolbar true /PrintScaling /Fit >>",
    );
    let _ = pdf_model::viewer_preferences::Opening::read(&document);
    let _ = pdf_model::viewer_preferences::ViewerPreferences::read(&document);
    assert_eq!(logged_alone(&document), []);
}

/// Table 215's `/H` is a boolean: one of another type is read as its default, and logged.
#[test]
fn an_action_entry_of_the_wrong_type_is_logged() {
    let document = catalog("");
    let action = pdf_syntax::Object::Dictionary(
        pdf_syntax::Parser::new(b"<< /S /Hide /T (a) /H 1 >>")
            .parse_object()
            .expect("the action parses")
            .as_dict()
            .expect("a dictionary")
            .clone(),
    );
    let _ = pdf_model::action::read(&document, &action);
    assert_eq!(
        logged_alone(&document),
        [(repair::ACTION_ENTRY_UNUSABLE.name, Site::WholeFile)]
    );
}
