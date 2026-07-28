//! Optional content, checked one rule of ISO 32000-2 §8.11 at a time.
//!
//! The corpus has five first pages that draw a hidden layer, and between them they exercise
//! perhaps three of this clause's rules — which is trap 8 in `doc/HANDOVER.md`: a corpus
//! finds what documents contain, not what the specification says. Each fixture here carries
//! exactly one construction, so a rule that stops working fails one test by name rather than
//! moving a pixel count on a page that has four other things wrong with it.
//!
//! Two of them exist because getting the rule backwards is *plausible*.
//! [`a_membership_dictionary_can_show_content_when_its_group_is_off`] is the one that
//! matters most: a reader that implements "hide what `/OFF` lists" passes every simple case
//! and inverts this one, and a document uses `/AllOff` precisely when it wants content that
//! appears while a layer is hidden. And
//! [`a_single_group_reference_is_read_without_losing_which_object_it_is`] pins a defect this
//! module shipped for an afternoon: `/OCGs` may be one reference rather than an array, and
//! resolving it to see what it is throws away the identity the group is recognised by.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture or an out-of-range pixel should fail loudly, \
              and the fixtures are 100x100 pages where no index can overflow"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the 100×100 pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// The page's content stream in every fixture that does not supply its own.
///
/// A black square from (20, 20) to (80, 80), inside a marked-content section governed by the
/// `/oc` entry of the page's `/Properties`. Nothing else marks the page, so a pixel at the
/// centre is painted exactly when the section is visible.
const MARKED_SQUARE: &str = "/OC /oc BDC 20 20 60 60 re f EMC";

/// Assembles a one-page PDF around an optional content configuration.
///
/// Object numbering is fixed so that a fixture can refer to its own objects: 1 catalog,
/// 2 pages, 3 page, 4 contents, and 5 onwards whatever `extra` defines.
fn pdf(properties: &str, resources: &str, content: &str, page_extra: &str, extra: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R {properties} >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << {resources} >> /Contents 4 0 R {page_extra} >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n{extra}",
        content.len().saturating_add(1)
    );

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

/// Renders a fixture at one pixel per unit onto a transparent background.
fn render(bytes: Vec<u8>) -> pdf_render::Raster {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the fixture should draw completely: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .with_background(pdf_render::Color::TRANSPARENT)
        .rasterize(&list, target)
        .expect("supported")
}

/// The RGBA at a point given in PDF coordinates, whose y runs the other way from a raster's.
fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let at = ((row.saturating_mul(raster.width)).saturating_add(x) as usize).saturating_mul(4);
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

/// Whether the middle of the fixture square was marked at all.
fn drew(raster: &pdf_render::Raster) -> bool {
    pixel(raster, 50, 50)[3] > 0
}

/// One optional content group, listed in the properties dictionary and turned off.
///
/// §8.11.4.5: the base state — `ON` by default (Table 99) — reaches every group, and then
/// the `/OFF` array adjusts the ones it names.
const ONE_GROUP_OFF: &str = "/OCProperties << /OCGs [5 0 R] /D << /OFF [5 0 R] >> >>";

/// The same group, left on, so that a test can show its fixture draws when nothing hides it.
const ONE_GROUP_ON: &str = "/OCProperties << /OCGs [5 0 R] /D << >> >>";

/// The group object itself. Table 96: `/Type` and `/Name` are the required entries.
const GROUP: &str = "5 0 obj\n<< /Type /OCG /Name (Layer) >>\nendobj\n";

/// A group the default configuration turns off hides the content marked with it.
///
/// The base case, and the one that decides whether §6.3.2.2's third obligation on a
/// rendering processor is met at all.
#[test]
fn a_marked_section_belonging_to_a_group_that_is_off_is_not_drawn() {
    let visible = render(pdf(
        ONE_GROUP_ON,
        "/Properties << /oc 5 0 R >>",
        MARKED_SQUARE,
        "",
        GROUP,
    ));
    assert!(drew(&visible), "the group is on, so the square is drawn");

    let hidden = render(pdf(
        ONE_GROUP_OFF,
        "/Properties << /oc 5 0 R >>",
        MARKED_SQUARE,
        "",
        GROUP,
    ));
    assert!(!drew(&hidden), "the group is off, so nothing is drawn");
}

/// Hiding stops the marking and nothing else.
///
/// §8.11.3.1: when optional content is hidden, "graphics state operations, such as setting
/// the colour, transformation matrix, and clipping, shall still be applied … graphics state
/// parameters that persist past the end of a marked-content section shall be the same
/// whether the optional content is visible or not."
///
/// The fixture sets blue inside the hidden section and fills a square *after* `EMC` without
/// setting a colour again. A reader that skips the section's operators paints it black.
#[test]
fn a_hidden_section_still_changes_the_graphics_state_it_leaves_behind() {
    let raster = render(pdf(
        ONE_GROUP_OFF,
        "/Properties << /oc 5 0 R >>",
        "/OC /oc BDC 0 0 1 rg 0 0 10 10 re f EMC 20 20 60 60 re f",
        "",
        GROUP,
    ));
    assert!(!drew(&render(pdf(
        ONE_GROUP_OFF,
        "/Properties << /oc 5 0 R >>",
        "/OC /oc BDC 0 0 1 rg 0 0 10 10 re f EMC",
        "",
        GROUP,
    ))));
    assert_eq!(
        pixel(&raster, 50, 50),
        [0, 0, 255, 255],
        "the colour set inside the hidden section must outlive it"
    );
}

/// A membership dictionary can make content visible *because* a group is off.
///
/// §8.11.2.2 Table 97: `/P /AllOff` means "visible only if all of the entries in OCGs are
/// OFF". This is the case a reader that implements "skip what `/OFF` lists" gets exactly
/// backwards, and it is why membership dictionaries exist — the clause names it first among
/// the cases they are useful for.
#[test]
fn a_membership_dictionary_can_show_content_when_its_group_is_off() {
    let raster = render(pdf(
        ONE_GROUP_OFF,
        "/Properties << /oc 6 0 R >>",
        MARKED_SQUARE,
        "",
        &format!("{GROUP}6 0 obj\n<< /Type /OCMD /OCGs [5 0 R] /P /AllOff >>\nendobj\n"),
    ));
    assert!(drew(&raster), "/AllOff shows content whose group is off");

    let inverted = render(pdf(
        ONE_GROUP_ON,
        "/Properties << /oc 6 0 R >>",
        MARKED_SQUARE,
        "",
        &format!("{GROUP}6 0 obj\n<< /Type /OCMD /OCGs [5 0 R] /P /AllOff >>\nendobj\n"),
    ));
    assert!(!drew(&inverted), "and hides it when the group is on");
}

/// `/OCGs` may be a single group rather than an array, and the reference is the identity.
///
/// §8.11.2.2 Table 97 types the entry as "dictionary or array of dictionaries". Resolving it
/// to find out which it is loses the reference, and a group with no reference matches nothing
/// in `/OCProperties /OCGs` — so every layer of `issue12007_reduced.pdf`, all of which are
/// written `<< /Type /OCMD /OCGs 38 0 R >>`, drew anyway. Found by the reference oracle after
/// this module was already in the tree and passing its other tests.
#[test]
fn a_single_group_reference_is_read_without_losing_which_object_it_is() {
    let raster = render(pdf(
        ONE_GROUP_OFF,
        "/Properties << /oc 6 0 R >>",
        MARKED_SQUARE,
        "",
        &format!("{GROUP}6 0 obj\n<< /Type /OCMD /OCGs 5 0 R >>\nendobj\n"),
    ));
    assert!(!drew(&raster), "a lone /OCGs reference names a real group");
}

/// A visibility expression is used in preference to `/OCGs` and `/P`, and it nests.
///
/// §8.11.2.2: "If the VE key is present it shall be used in preference to the OCGs and P
/// keys", and "In evaluating a visibility expression, the ON state of an optional content
/// group shall be equated to the boolean value true; OFF shall be equated to false."
///
/// The fixture's `/OCGs` and `/P` say the opposite of its `/VE`, so a reader that evaluates
/// the wrong one fails rather than agreeing by accident. `visibility_expressions.pdf` is the
/// corpus's version of this, and it is worth knowing that `mupdf` and `ghostscript` both
/// draw all five of its lines: neither implements `/VE`, and this is one of the few pages
/// where the reference consensus is wrong and the clause is unambiguous.
#[test]
fn a_visibility_expression_outranks_the_policy_beside_it() {
    let extra = format!(
        "{GROUP}6 0 obj\n<< /Type /OCMD /OCGs [5 0 R] /P /AllOff /VE [/Not [/Or 5 0 R 5 0 R]] >>\nendobj\n"
    );
    // The group is *on*, so `/VE` — not(on or on) — hides, while `/P /AllOff` would show.
    let raster = render(pdf(
        ONE_GROUP_ON,
        "/Properties << /oc 6 0 R >>",
        MARKED_SQUARE,
        "",
        &extra,
    ));
    assert!(!drew(&raster), "/VE decides, and it says hidden");
}

/// A form `XObject` may be made optional in its entirety by an `/OC` entry.
///
/// §8.11.3.3, and the entry point `issue12007_reduced.pdf` uses. A reader that implements
/// only the marked-content form of §8.11.3.2 draws that document's hidden screenshot in full
/// while passing every test above.
#[test]
fn a_form_xobject_carrying_oc_is_skipped_when_its_group_is_off() {
    let form = "6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /OC 5 0 R \
                /Length 20 >>\nstream\n20 20 60 60 re f\nendstream\nendobj\n";
    let hidden = render(pdf(
        ONE_GROUP_OFF,
        "/XObject << /Fx 6 0 R >>",
        "/Fx Do",
        "",
        &format!("{GROUP}{form}"),
    ));
    assert!(!drew(&hidden), "the form's own /OC hides it");

    let visible = render(pdf(
        ONE_GROUP_ON,
        "/XObject << /Fx 6 0 R >>",
        "/Fx Do",
        "",
        &format!("{GROUP}{form}"),
    ));
    assert!(visible.data.iter().any(|byte| *byte > 0), "and shows it");
}

/// An annotation carrying `/OC` is not drawn when its group is off.
///
/// §8.11.3.3: an annotation with an `/OC` entry "shall be visible for screen or print only if
/// the flags have the appropriate settings and the group or membership dictionary indicates
/// it shall be visible". The flags are §12.5.3's business; this is the other half.
#[test]
fn an_annotation_carrying_oc_is_not_drawn_when_its_group_is_off() {
    let annotation = "6 0 obj\n<< /Type /Annot /Subtype /Square /Rect [20 20 80 80] /F 4 \
                      /OC 5 0 R /AP << /N 7 0 R >> >>\nendobj\n";
    let appearance = "7 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 60 60] \
                      /Length 18 >>\nstream\n0 0 60 60 re f\nendstream\nendobj\n";
    let hidden = render(pdf(
        ONE_GROUP_OFF,
        "",
        "",
        "/Annots [6 0 R]",
        &format!("{GROUP}{annotation}{appearance}"),
    ));
    assert!(!drew(&hidden), "the annotation's /OC hides it");

    let visible = render(pdf(
        ONE_GROUP_ON,
        "",
        "",
        "/Annots [6 0 R]",
        &format!("{GROUP}{annotation}{appearance}"),
    ));
    assert!(drew(&visible), "and shows it when the group is on");
}

/// A group the properties dictionary never listed governs nothing.
///
/// §8.11.3.2: content is optional content "only if the tag is OC and the dictionary operand
/// is a valid optional content group that is included in the OCGs array of the optional
/// content properties dictionary … or a valid optional content membership dictionary". The
/// fixture's group is written exactly like a real one and turned off by an `/OFF` array that
/// names it — but `/OCGs` does not, so it is not one of the document's groups.
#[test]
fn a_group_missing_from_the_properties_dictionary_hides_nothing() {
    let raster = render(pdf(
        "/OCProperties << /OCGs [] /D << /OFF [5 0 R] >> >>",
        "/Properties << /oc 5 0 R >>",
        MARKED_SQUARE,
        "",
        GROUP,
    ));
    assert!(drew(&raster), "an undeclared group is not a group");
}

/// Without `/OCProperties`, optional content structures are ignored entirely.
///
/// §8.11.4.2: the dictionary "shall be present if the PDF file contains any optional content;
/// if it is missing, a PDF processor shall ignore any optional content structures in the
/// document". So a `/OC` marked section in such a file is drawn, and there is nothing to
/// decide.
#[test]
fn a_document_with_no_properties_dictionary_draws_everything() {
    let raster = render(pdf(
        "",
        "/Properties << /oc 5 0 R >>",
        MARKED_SQUARE,
        "",
        GROUP,
    ));
    assert!(drew(&raster), "no /OCProperties means no optional content");
}

/// A group outside the configuration's intent has no effect on visibility.
///
/// §8.11.2.3: "If one or more of a group's intents is contained in the current
/// configuration's set of intents, the group shall be used in determining visibility. If
/// there is no match, the group shall have no effect on visibility." The configuration here
/// takes the default — `View` — and the group declares `Design`, which is what a drawing
/// application writes for a layer that organises artwork rather than presentation.
///
/// So the square is drawn *even though its group is off*, which is the rule's whole point and
/// is indistinguishable from a bug unless the reason is written down.
#[test]
fn a_group_whose_intent_the_configuration_ignores_does_not_hide_anything() {
    let raster = render(pdf(
        ONE_GROUP_OFF,
        "/Properties << /oc 5 0 R >>",
        MARKED_SQUARE,
        "",
        "5 0 obj\n<< /Type /OCG /Name (Layer) /Intent /Design >>\nendobj\n",
    ));
    assert!(
        drew(&raster),
        "a Design-only group is not consulted by a View configuration"
    );

    // And the same group *is* consulted when the configuration asks for every intent.
    let all = render(pdf(
        "/OCProperties << /OCGs [5 0 R] /D << /OFF [5 0 R] /Intent /All >> >>",
        "/Properties << /oc 5 0 R >>",
        MARKED_SQUARE,
        "",
        "5 0 obj\n<< /Type /OCG /Name (Layer) /Intent /Design >>\nendobj\n",
    ));
    assert!(
        !all.data.iter().any(|byte| *byte > 0),
        "/Intent /All covers every group"
    );
}

/// `/BaseState /OFF` hides every group, and `/ON` names the exceptions.
///
/// §8.11.4.5 a) and b), and Table 99's note that with a base state of `OFF` the `/OFF` array
/// is redundant. A reader that only ever consults `/OFF` shows a document written this way
/// in full.
#[test]
fn a_base_state_of_off_hides_the_groups_the_on_array_does_not_name() {
    let hidden = render(pdf(
        "/OCProperties << /OCGs [5 0 R] /D << /BaseState /OFF >> >>",
        "/Properties << /oc 5 0 R >>",
        MARKED_SQUARE,
        "",
        GROUP,
    ));
    assert!(!drew(&hidden), "/BaseState /OFF reaches every group");

    let restored = render(pdf(
        "/OCProperties << /OCGs [5 0 R] /D << /BaseState /OFF /ON [5 0 R] >> >>",
        "/Properties << /oc 5 0 R >>",
        MARKED_SQUARE,
        "",
        GROUP,
    ));
    assert!(drew(&restored), "and /ON names the exceptions");
}

/// A visibility expression nested past the interpreter's bound is reported, and drawn.
///
/// `/VE` is a tree the document supplies, so it is untrusted input with a natural recursion
/// in it, and the bound is the answer to that. Two things are being pinned here, and the
/// second is the one that matters:
///
/// - **The bound holds.** A 40-deep chain of `/Not` does not recurse to the bottom.
/// - **Reaching it is loud.** `Unsupported::OptionalContent` is reported, so the page stops
///   claiming to be complete and leaves the oracle's gate rather than being compared while
///   wrong. Silent caps are defects, not safety — the interpreter's operand cap taught this
///   project that once already, by truncating a `TJ` array mid-sentence with
///   `unsupported: []`.
///
/// The content is *drawn* rather than hidden, which is the deliberate choice between the two
/// ways to be wrong: something visible that should be hidden is on the page where a reader
/// can see it, where something hidden that should be visible is gone without a trace.
#[test]
fn a_visibility_expression_past_the_bound_is_reported_rather_than_recursed() {
    let mut expression = String::from("5 0 R");
    for _ in 0..40 {
        expression = format!("[/Not {expression}]");
    }
    let bytes = pdf(
        ONE_GROUP_OFF,
        "/Properties << /oc 6 0 R >>",
        MARKED_SQUARE,
        "",
        &format!("{GROUP}6 0 obj\n<< /Type /OCMD /VE {expression} >>\nendobj\n"),
    );

    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);

    let reported = format!("{:?}", interpretation.unsupported);
    assert!(
        reported.contains("OptionalContent"),
        "a bound reached must be said out loud: {reported}"
    );
}
