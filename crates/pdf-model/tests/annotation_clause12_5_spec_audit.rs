//! Specification audit of ISO 32000-2 §12.5 (annotations): the behaviour findings `2175-B01` to
//! `2175-B06` of `doc/reviews/2175-specification-audit-12-5.md`, each a failing test beside a
//! control that passes.
//!
//! Every fixture is a complete one-page PDF with an exact cross-reference table, so nothing here
//! is recovery; each expected value is derived from the clause its test names.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::doc_markdown,
    reason = "a fixture or rendering failure should fail at its source; raster coordinates are \
              bounded by these 100-unit fixtures; the standard's sentences are quoted verbatim, \
              flag names as it spells them"
)]

use std::fmt::Write as _;

use pdf_model::restriction::{Operation, Restriction};
use pdf_model::view::{ViewState, annotation_at, annotation_interacts, field_at};
use pdf_model::{Pages, content};
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::{Document, ObjectId};
use render_cpu::CpuRasterizer;

const BUDGET: u64 = 1 << 24;

/// The annotation every fixture puts in `/Annots`.
const ANNOTATION: ObjectId = ObjectId::new(5, 0);

fn assemble(body: &str) -> Vec<u8> {
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let xref = bytes.len();
    let size = offsets.len() + 1;
    let _ = writeln!(bytes, "xref\n0 {size}");
    bytes.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    bytes.into_bytes()
}

/// A stream object with an exact `/Length`.
fn stream(number: u32, dict: &str, data: &str) -> String {
    format!(
        "{number} 0 obj\n<< {dict} /Length {} >>\nstream\n{data}\nendstream\nendobj\n",
        data.len() + 1
    )
}

/// One 100-by-100 page whose `/Annots` is `[5 0 R]`: `catalog` and `page` are added to those two
/// dictionaries, and `rest` is object 5 and anything after it.
fn pdf(catalog: &str, page: &str, rest: &str) -> Vec<u8> {
    assemble(&format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R {catalog} >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] {page} \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n{rest}"
    ))
}

/// A `/Square` with a stored appearance that fills its whole `/BBox` black.
fn filled_square(rect: &str, flags: i64, extra: &str, width: u32, height: u32) -> String {
    format!(
        "5 0 obj\n<< /Type /Annot /Subtype /Square /Rect [{rect}] /F {flags} {extra} \
         /AP << /N 6 0 R >> >>\nendobj\n{}",
        stream(
            6,
            &format!("/Type /XObject /Subtype /Form /BBox [0 0 {width} {height}]"),
            &format!("0 0 0 rg 0 0 {width} {height} re f"),
        )
    )
}

/// A form whose only field is the text field `f`, whose only widget is object 5.
fn one_field(widget: &str) -> Vec<u8> {
    pdf(
        "/AcroForm << /Fields [5 0 R] >>",
        "",
        &format!("5 0 obj\n<< /Type /Annot /Subtype /Widget /T (f) /P 3 0 R {widget} >>\nendobj\n"),
    )
}

fn open(bytes: Vec<u8>) -> Document {
    let document = Document::open(bytes).expect("the fixture opens");
    assert!(!document.was_recovered(), "the exact xref is used");
    document
}

/// The page drawn at one pixel per unit, and what the interpretation reported.
fn render(document: &Document, state: &ViewState) -> (pdf_render::Raster, Vec<String>) {
    let page = Pages::new(document).get(0).expect("page one");
    let interpreted = content::interpret_with(document, &page, state);
    let reported = interpreted
        .unsupported
        .iter()
        .map(|unsupported| format!("{unsupported:?}"))
        .collect();
    let target =
        TargetSpec::for_page(&interpreted.display_list, 1.0, BUDGET).expect("a bounded target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpreted.display_list, target)
        .expect("rasterization");
    (raster, reported)
}

/// The alpha of the device pixel `(x, y)`, counting `y` up from the bottom row.
fn alpha(raster: &pdf_render::Raster, x: u32, y: u32) -> u8 {
    let row = raster.height - 1 - y;
    let index = usize::try_from((row * raster.width + x) * 4 + 3).expect("small index");
    raster.data[index]
}

fn hit(document: &Document, state: &ViewState, x: f32, y: f32) -> Option<ObjectId> {
    let page = Pages::new(document).get(0).expect("page one");
    annotation_at(document, &page, state, x, y)
}

/// 2175-B01, `NoZoom`. ISO 32000-2 §12.5.3 (after Table 167): "If the NoZoom flag is set, the
/// annotation shall always maintain the same fixed size on the screen", pivoting on the upper-left
/// corner of `/Rect`, and "\[t]he PDF processor shall perform this alteration automatically" —
/// "the positions of the other three corners of the annotation rectangle are different in the
/// altered user space". §12.5.5's rollover and down appearances, and §12.6.3's triggers, are about
/// "the annotation's active area", which is the rectangle where it is shown.
///
/// `/Rect [10 70 30 90]` at magnification 0.5 is drawn twice as large in default user space about
/// (10, 90): over [10, 50] × [50, 90]. The point (40, 55) is inked and is not in `/Rect`.
#[test]
fn a_no_zoom_annotations_active_area_is_where_it_is_drawn_spec_audit() {
    let document = open(pdf("", "", &filled_square("10 70 30 90", 8, "", 20, 20)));
    let mut state = ViewState::of(&document);
    state.set_magnification(Some(0.5));
    let (raster, reported) = render(&document, &state);
    assert!(reported.is_empty(), "nothing to report: {reported:?}");
    // Control: the drawing honours §12.5.3, and the unaltered rectangle is still a hit.
    assert_eq!(
        alpha(&raster, 40, 55),
        255,
        "NoZoom draws the square at twice the size"
    );
    assert_eq!(hit(&document, &state, 20.0, 80.0), Some(ANNOTATION));
    assert_eq!(
        hit(&document, &state, 40.0, 55.0),
        Some(ANNOTATION),
        "a pointer on the drawn annotation is on its active area"
    );
}

/// 2175-B01, `NoRotate`. §12.5.3: "if the NoRotate flag is set, the annotation shall retain its
/// original orientation on the screen when the page is rotated", about the upper-left corner of
/// `/Rect`.
///
/// `/Rect [10 30 50 50]` on a `/Rotate 90` page is turned about (10, 50) and drawn over
/// [10, 30] × [50, 90] of default user space; `/Rotate 90` shows the user-space point (20, 80) at
/// device pixel (80, 80) counted from the bottom. The point is inked and is not in `/Rect`.
#[test]
fn a_no_rotate_annotations_active_area_is_where_it_is_drawn_spec_audit() {
    let document = open(pdf(
        "",
        "/Rotate 90",
        &filled_square("10 30 50 50", 16, "", 40, 20),
    ));
    let state = ViewState::of(&document);
    let (raster, reported) = render(&document, &state);
    assert!(reported.is_empty(), "nothing to report: {reported:?}");
    assert_eq!(
        alpha(&raster, 80, 80),
        255,
        "NoRotate draws the square there"
    );
    assert_eq!(
        hit(&document, &state, 20.0, 80.0),
        Some(ANNOTATION),
        "a pointer on the drawn annotation is on its active area"
    );
}

/// 2175-B02. Table 167, bit 2 `Hidden`: "If set, do not render the annotation or allow it to
/// interact with the user"; bit 6 `NoView`: "do not render the annotation on the screen or allow
/// it to interact with the user". `field_at` is what a host's click on a field asks
/// (`viewer_host::form::clicked`), and it answers for a hidden widget.
#[test]
fn a_hidden_widget_is_not_a_field_under_the_pointer_spec_audit() {
    let field = |flags: i64| {
        let document = open(one_field(&format!(
            "/FT /Tx /Rect [10 10 90 90] /F {flags}"
        )));
        let page = Pages::new(&document).get(0).expect("page one");
        field_at(&document, &page, 50.0, 50.0).map(|name| name.qualified)
    };
    assert_eq!(
        field(0),
        Some("f".to_owned()),
        "control: a shown widget is found"
    );
    assert_eq!(field(2), None, "Hidden: no interaction");
    assert_eq!(field(32), None, "NoView: no interaction on the screen");
}

/// 2175-B03. Table 167, bit 7 `ReadOnly`: "This flag shall be ignored for widget annotations; its
/// function is subsumed by the ReadOnly flag of the associated form field". Table 227, bit 1
/// `ReadOnly`: "any associated widget annotations should not interact with the user; that is, they
/// should not respond to mouse clicks nor change their appearance in response to mouse motions."
#[test]
fn a_widget_of_a_read_only_field_does_not_interact_spec_audit() {
    let interacts = |field_flags: i64| {
        let document = open(one_field(&format!(
            "/FT /Tx /Ff {field_flags} /Rect [10 10 90 90] /F 4"
        )));
        let state = ViewState::of(&document);
        (
            annotation_interacts(&document, ANNOTATION, &state),
            hit(&document, &state, 50.0, 50.0),
        )
    };
    assert_eq!(
        interacts(0),
        (true, Some(ANNOTATION)),
        "control: a writable field"
    );
    assert_eq!(interacts(1), (false, None), "the field's ReadOnly flag");
}

/// 2175-B04. Table 167, bit 10 `LockedContents`: "If set, do not allow the contents of the
/// annotation to be modified by the user." Bit 8's row says what a widget's contents are: "this
/// flag does not restrict changes to the annotation's contents, such as the value of a form
/// field". The restriction is asked only of `Operation::Annotate`.
#[test]
fn locked_contents_on_a_widget_restricts_filling_its_field_spec_audit() {
    let document = open(one_field("/FT /Tx /Rect [10 10 90 90] /F 516"));
    // Control: the flag is read off the same widget where the operation is annotating.
    assert!(
        pdf_model::restriction::asserted(&document, Operation::Annotate, None, Some(ANNOTATION))
            .contains(&Restriction::AnnotationLocked)
    );
    assert!(
        pdf_model::restriction::asserted(&document, Operation::FillInForm, Some("f"), None)
            .contains(&Restriction::AnnotationLocked),
        "the value of the field is the widget's contents"
    );
}

/// 2175-B05. Table 166, `/AS`: "(Required if the appearance dictionary AP contains one or more
/// subdictionaries)". §12.5.5's "reasonable behaviour (such as displaying nothing)" is for an `/AS`
/// that "designates an appearance state for which no appearance is defined"; an absent `/AS` is a
/// third case, a file that breaks a requirement, and nothing is drawn without a word.
#[test]
fn an_appearance_subdictionary_without_an_as_is_reported_spec_audit() {
    let square = |state: &str| {
        let document = open(pdf(
            "",
            "",
            &format!(
                "5 0 obj\n<< /Type /Annot /Subtype /Square /Rect [10 10 90 90] {state} \
                 /AP << /N << /On 6 0 R >> >> >>\nendobj\n{}",
                stream(
                    6,
                    "/Type /XObject /Subtype /Form /BBox [0 0 80 80]",
                    "0 0 0 rg 0 0 80 80 re f"
                )
            ),
        ));
        let (raster, reported) = render(&document, &ViewState::of(&document));
        (alpha(&raster, 50, 50), reported)
    };
    let (ink, reported) = square("/AS /On");
    assert_eq!((ink, reported.len()), (255, 0), "control: /AS selects /On");
    let (ink, reported) = square("");
    assert_eq!(ink, 0, "no state is chosen, so nothing is drawn");
    assert!(
        !reported.is_empty(),
        "an /N subdictionary with no /AS is said, not drawn as nothing in silence"
    );
}

/// 2175-B06, `/Rect`. Table 166: `/Rect` is a rectangle, which §7.9.5 makes "a rectangle shall be
/// written as an array of four numbers". `[10 /x 10 50 50]` is read as `[10 10 50 50]` — the name
/// dropped and the numbers after it moved one place — and drawn there without a report.
#[test]
fn a_rect_that_is_not_four_numbers_is_reported_spec_audit() {
    let drawn = |rect: &str| {
        let document = open(pdf("", "", &filled_square(rect, 0, "", 40, 40)));
        let (raster, reported) = render(&document, &ViewState::of(&document));
        (alpha(&raster, 30, 30), reported)
    };
    let (ink, reported) = drawn("10 10 50 50");
    assert_eq!(
        (ink, reported.len()),
        (255, 0),
        "control: a well-formed /Rect"
    );
    let (_, reported) = drawn("10 /x 10 50 50");
    assert!(
        !reported.is_empty(),
        "a /Rect holding a name is malformed and is said"
    );
}

/// 2175-B06, `/Matrix`. §12.5.5 maps the appearance "as defined by its Matrix entry; see "Table 93
/// -Additional entries specific to a Type 1 form dictionary"", and Table 93's `/Matrix` is "\[a]n
/// array of six numbers". A five-number `/Matrix` is taken as the identity in silence.
#[test]
fn an_appearance_matrix_that_is_not_six_numbers_is_reported_spec_audit() {
    let drawn = |matrix: &str| {
        let document = open(pdf(
            "",
            "",
            &format!(
                "5 0 obj\n<< /Type /Annot /Subtype /Square /Rect [10 10 50 50] \
                 /AP << /N 6 0 R >> >>\nendobj\n{}",
                stream(
                    6,
                    &format!("/Type /XObject /Subtype /Form /BBox [0 0 40 40] /Matrix [{matrix}]"),
                    "0 0 0 rg 0 0 40 40 re f"
                )
            ),
        ));
        render(&document, &ViewState::of(&document)).1
    };
    assert!(drawn("1 0 0 1 0 0").is_empty(), "control: six numbers");
    assert!(
        !drawn("1 0 0 1 0").is_empty(),
        "a five-number /Matrix is malformed and is said"
    );
}
