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

    assemble(&body)
}

/// The same page with Table 31's *array* form of `/Contents`, in two parts.
///
/// Object numbering keeps `pdf`'s so that [`GROUP`] and the configurations above are reusable:
/// 1 catalog, 2 pages, 3 page, 4 the first part, 5 onwards whatever `extra` defines, and the
/// second part last — its number is one past `extra`'s, which every caller here knows because
/// `extra` is [`GROUP`] alone.
fn pdf_of_two_parts(properties: &str, resources: &str, parts: [&str; 2], extra: &str) -> Vec<u8> {
    let second = 6;
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R {properties} >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << {resources} >> /Contents [4 0 R {second} 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n{extra}\
         {second} 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        parts[0].len().saturating_add(1),
        parts[0],
        parts[1].len().saturating_add(1),
        parts[1],
    );

    assemble(&body)
}

/// Wraps a body of objects in a header, a cross-reference table and a trailer.
fn assemble(body: &str) -> Vec<u8> {
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
        .with_medium(pdf_render::Medium::NONE)
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

/// A section may open in one part of `/Contents` and close in the next.
///
/// ISO 32000-2 §14.6.1 states the one requirement that clause puts on a *reader* — every other
/// `shall` in it is addressed to whoever writes the file:
///
/// > The Contents entry of a page object (see 7.7.3.3, "Page objects"), whether a single stream
/// > or an array of streams, is considered a single stream with respect to marked-content
/// > sequences.
///
/// The fixture is built so that a reader treating each part as a stream of its own fails it
/// rather than merely reporting differently: the `BDC` is the whole of part one, so a
/// per-part reader would discard the open section at the end of it and paint the square that
/// part two draws before the `EMC`. The rectangle *after* the `EMC` is the control — it says
/// the section ended where the second part closed it, rather than running to the end of the
/// page.
#[test]
fn a_marked_content_section_may_span_two_parts_of_the_contents_array() {
    let parts = ["/OC /oc BDC", "20 20 60 60 re f EMC 0 0 10 10 re f"];

    let visible = render(pdf_of_two_parts(
        ONE_GROUP_ON,
        "/Properties << /oc 5 0 R >>",
        parts,
        GROUP,
    ));
    assert!(drew(&visible), "the group is on, so the square is drawn");
    assert!(
        pixel(&visible, 5, 5)[3] > 0,
        "and so is the rectangle after the EMC"
    );

    let hidden = render(pdf_of_two_parts(
        ONE_GROUP_OFF,
        "/Properties << /oc 5 0 R >>",
        parts,
        GROUP,
    ));
    assert!(
        !drew(&hidden),
        "the section opened in the first part governs the square in the second"
    );
    assert!(
        pixel(&hidden, 5, 5)[3] > 0,
        "and the EMC in the second part ends it, so what follows is drawn"
    );
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

/// §8.11.4.4's `/AS` array switches a group off after the base configuration set it on.
///
/// > For each of the groups in OCGs , the entries in its usage dictionary … specified by
/// > Category shall be examined to yield a recommended state for the group. If all the
/// > entries yield a recommended state of ON , the group's state shall be set to ON ;
/// > otherwise, its state shall be set to OFF .
///
/// The corpus cannot test this: eight of its documents carry an `/AS`, every one of them
/// pairs a `View` event with the `View` category, and not one of their groups states a
/// `/ViewState` of `OFF`. So the mechanism runs on every one of them and changes nothing,
/// which is exactly the shape trap 8 describes.
#[test]
fn a_usage_application_dictionary_turns_a_group_off() {
    let visible = render(pdf(
        "/OCProperties << /OCGs [5 0 R] /D << /AS [<< /Event /View /Category [/View] \
         /OCGs [5 0 R] >>] >> >>",
        "/Properties << /oc 5 0 R >>",
        MARKED_SQUARE,
        "",
        "5 0 obj\n<< /Type /OCG /Name (Layer) /Usage << /View << /ViewState /ON >> >> >>\nendobj\n",
    ));
    let hidden = render(pdf(
        "/OCProperties << /OCGs [5 0 R] /D << /AS [<< /Event /View /Category [/View] \
         /OCGs [5 0 R] >>] >> >>",
        "/Properties << /oc 5 0 R >>",
        MARKED_SQUARE,
        "",
        "5 0 obj\n<< /Type /OCG /Name (Layer) /Usage << /View << /ViewState /OFF >> >> >>\nendobj\n",
    ));

    assert!(drew(&visible), "an ON view state leaves the group on");
    assert!(!drew(&hidden), "an OFF view state turns it off");
}

/// Only the `View` event applies here, because only viewing is what this is.
///
/// §8.11.4.5: an interactive processor "shall examine the AS array for usage application
/// dictionaries that have an Event of type View", and a `Print` one applies "for the duration
/// of the print operation". A viewer that applied `Print` would hide a watermark the document
/// means to show on screen — or show one it means only to print.
#[test]
fn a_print_event_does_not_apply_to_a_screen() {
    let raster = render(pdf(
        "/OCProperties << /OCGs [5 0 R] /D << /AS [<< /Event /Print /Category [/Print] \
         /OCGs [5 0 R] >>] >> >>",
        "/Properties << /oc 5 0 R >>",
        MARKED_SQUARE,
        "",
        "5 0 obj\n<< /Type /OCG /Name (Layer) /Usage << /Print << /PrintState /OFF >> >> >>\n\
         endobj\n",
    ));

    assert!(drew(&raster), "a Print event has no say over a screen");
}

/// The categories are an AND, and a group named twice is an AND across both dictionaries.
///
/// §8.11.4.4:
///
/// > If a given optional content group appears in more than one OCGs array, its state shall
/// > be ON only if all categories in all the usage application dictionaries it appears in
/// > have a state of ON .
#[test]
fn every_category_of_every_dictionary_has_to_agree() {
    let both = "/OCProperties << /OCGs [5 0 R] /D << /AS [\
        << /Event /View /Category [/View] /OCGs [5 0 R] >> \
        << /Event /View /Category [/Export] /OCGs [5 0 R] >>] >> >>";
    let raster = render(pdf(
        both,
        "/Properties << /oc 5 0 R >>",
        MARKED_SQUARE,
        "",
        "5 0 obj\n<< /Type /OCG /Name (Layer) /Usage << /View << /ViewState /ON >> \
         /Export << /ExportState /OFF >> >> >>\nendobj\n",
    ));

    assert!(
        !drew(&raster),
        "the second dictionary's OFF has to survive the first's ON"
    );
}

/// A category about *this machine* leaves the state alone and says so.
///
/// `User` matches "the user's identification" and `Language` "the language and locale of the
/// application", and neither is a question about the document. The clause's "otherwise OFF"
/// would hide content on the strength of a question nobody asked, so the configuration's own
/// state stands and the page reports what it could not answer.
#[test]
fn a_category_about_this_machine_is_reported_rather_than_guessed() {
    let bytes = pdf(
        "/OCProperties << /OCGs [5 0 R] /D << /AS [<< /Event /View /Category [/Language] \
         /OCGs [5 0 R] >>] >> >>",
        "/Properties << /oc 5 0 R >>",
        MARKED_SQUARE,
        "",
        "5 0 obj\n<< /Type /OCG /Name (Layer) /Usage << /Language << /Lang (es-MX) >> >> >>\n\
         endobj\n",
    );
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);

    assert!(
        !interpretation.is_complete(),
        "the Language category must be reported"
    );
    assert!(
        !interpretation.display_list.commands().is_empty(),
        "and the group's own state must stand, which here draws"
    );
}

/// `Zoom` is answered at a magnification of 1.0, which is a choice this file records.
///
/// A display list carries no magnification — it is built once and rasterised at whatever
/// scale the caller asks for — so the clause's "current magnification level of the document"
/// has to be supplied from somewhere. It is 1.0, the magnification at which a page is its
/// stated size. A group asking for more than that is off; a group asking for less is on.
#[test]
fn a_zoom_category_is_answered_at_the_pages_stated_size() {
    let with = |zoom: &str| {
        render(pdf(
            "/OCProperties << /OCGs [5 0 R] /D << /AS [<< /Event /View /Category [/Zoom] \
             /OCGs [5 0 R] >>] >> >>",
            "/Properties << /oc 5 0 R >>",
            MARKED_SQUARE,
            "",
            &format!(
                "5 0 obj\n<< /Type /OCG /Name (Layer) /Usage << /Zoom << {zoom} >> >> >>\nendobj\n"
            ),
        ))
    };

    assert!(drew(&with("/min 0.5 /max 2.0")), "1.0 is inside 0.5..2.0");
    assert!(!drew(&with("/min 2.0")), "and below a minimum of 2.0");
    // "greater than or equal to min and less than max", so the maximum is exclusive.
    assert!(
        !drew(&with("/max 1.0")),
        "and not less than a maximum of 1.0"
    );
    assert!(
        drew(&with("/min 1.0")),
        "and not less than a minimum of 1.0"
    );
}

/// Interprets a fixture without demanding that it be complete, and answers what it reported.
///
/// [`render`] asserts completeness, which is right for every test above and wrong for
/// §8.9.5.4: an alternate image dictionary stating no `/Image` is a document defect the clause
/// has no step for, and naming it is what the tests below check.
fn interpret(bytes: Vec<u8>) -> (pdf_render::Raster, Vec<String>) {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    let reports = interpretation
        .unsupported
        .iter()
        .map(|item| format!("{item:?}"))
        .collect();
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported");
    (raster, reports)
}

/// Two optional content groups: object 5 is off and object 9 is on.
const TWO_GROUPS: &str = "/OCProperties << /OCGs [5 0 R 6 0 R] /D << /OFF [5 0 R] >> >>";

/// Object 9, the group the configuration leaves on.
const SECOND_GROUP: &str = "6 0 obj\n<< /Type /OCG /Name (Shown) >>\nendobj\n";

/// A one-pixel greyscale image object, `number`, of the given sample byte.
///
/// Greyscale rather than RGB so that the sample *is* the colour: 0x00 draws black and 0xFF
/// white, and a test can say which image reached the page by reading one pixel. The sample goes
/// in through `ASCIIHexDecode` so that every byte of the fixture stays printable — `pdf` builds
/// its offsets over a `String`, and a raw 0xFF would be two bytes of UTF-8 rather than one
/// sample.
fn one_pixel_image(number: u32, sample: u8, extra: &str) -> String {
    format!(
        "{number} 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 \
         /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /ASCIIHexDecode {extra} \
         /Length 3 >>\nstream\n{sample:02X}>\nendstream\nendobj\n"
    )
}

/// Draws object 6 as an image across the middle of the page.
const DRAW_IMAGE: &str = "q 60 0 0 60 20 20 cm /Im Do Q";

/// §8.9.5.4 step a), as Errata Collection 3 amends it: "[i]f the base image contains an OC
/// entry that specifies that the content is not visible, then nothing shall be shown."
///
/// Terminal, so the `/Alternates` are not examined — which is where this differs from the 2020
/// algorithm, whose step c) sent a hidden base image to its alternates.
#[test]
fn a_hidden_base_image_shows_nothing_and_does_not_reach_its_alternates() {
    let base = one_pixel_image(7, 0x00, "/OC 5 0 R /Alternates 8 0 R");
    let alternates = "8 0 obj\n[ << /Image 9 0 R /OC 6 0 R >> ]\nendobj\n";
    let alternate = one_pixel_image(9, 0xFF, "");
    let (raster, reports) = interpret(pdf(
        TWO_GROUPS,
        "/XObject << /Im 7 0 R >>",
        DRAW_IMAGE,
        "",
        &format!("{GROUP}{SECOND_GROUP}{base}{alternates}{alternate}"),
    ));
    assert!(!drew(&raster), "nothing shall be shown");
    assert!(reports.is_empty(), "a decision is not a gap: {reports:?}");
}

/// Step d): "the first alternate containing an OC entry specifying that its content is visible
/// shall be shown" — examined in the array's order, so an alternate whose group is off is
/// passed over for the next one.
#[test]
fn a_base_image_with_no_group_shows_the_first_visible_alternate() {
    let base = one_pixel_image(7, 0x00, "/Alternates 8 0 R");
    let alternates =
        "8 0 obj\n[ << /Image 9 0 R /OC 5 0 R >> << /Image 10 0 R /OC 6 0 R >> ]\nendobj\n";
    let hidden = one_pixel_image(9, 0x40, "");
    let shown = one_pixel_image(10, 0xFF, "");
    let (raster, reports) = interpret(pdf(
        TWO_GROUPS,
        "/XObject << /Im 7 0 R >>",
        DRAW_IMAGE,
        "",
        &format!("{GROUP}{SECOND_GROUP}{base}{alternates}{hidden}{shown}"),
    ));
    assert_eq!(
        pixel(&raster, 50, 50),
        [255, 255, 255, 255],
        "the second alternate, whose group is on"
    );
    assert!(reports.is_empty(), "{reports:?}");
}

/// Step d)'s parenthesis — "(Alternates that have no OC entry shall not be shown.)" — with
/// step e) behind it: "[i]f steps c and d above do not identify an alternate to be rendered
/// then the base image shall be rendered."
///
/// This is the sentence the 2020 clause contradicted itself about and the one this tree used to
/// resolve the other way, drawing the alternate and reporting the choice.
#[test]
fn an_alternate_with_no_group_is_not_shown_and_the_base_image_is() {
    let base = one_pixel_image(7, 0x00, "/Alternates 8 0 R");
    let alternates = "8 0 obj\n[ << /Image 9 0 R >> ]\nendobj\n";
    let alternate = one_pixel_image(9, 0xFF, "");
    let (raster, reports) = interpret(pdf(
        TWO_GROUPS,
        "/XObject << /Im 7 0 R >>",
        DRAW_IMAGE,
        "",
        &format!("{GROUP}{SECOND_GROUP}{base}{alternates}{alternate}"),
    ));
    assert_eq!(
        pixel(&raster, 50, 50),
        [0, 0, 0, 255],
        "the base image, by step e)"
    );
    assert!(reports.is_empty(), "a decision is not a gap: {reports:?}");
}

/// Step e) again, from the other side: every alternate states a group and every group is off.
#[test]
fn a_base_image_whose_alternates_are_all_hidden_is_drawn_itself() {
    let base = one_pixel_image(7, 0x00, "/Alternates 8 0 R");
    let alternates = "8 0 obj\n[ << /Image 9 0 R /OC 5 0 R >> ]\nendobj\n";
    let alternate = one_pixel_image(9, 0xFF, "");
    let (raster, reports) = interpret(pdf(
        TWO_GROUPS,
        "/XObject << /Im 7 0 R >>",
        DRAW_IMAGE,
        "",
        &format!("{GROUP}{SECOND_GROUP}{base}{alternates}{alternate}"),
    ));
    assert_eq!(pixel(&raster, 50, 50), [0, 0, 0, 255], "the base image");
    assert!(reports.is_empty(), "{reports:?}");
}

/// Step b): "[i]f the base image contains an OC entry that specifies that the base image is
/// visible, then the base image shall be rendered" — `/Alternates` beside it changes nothing.
#[test]
fn a_visible_base_image_is_drawn_whatever_its_alternates_say() {
    let base = one_pixel_image(7, 0x00, "/OC 6 0 R /Alternates 8 0 R");
    let alternates = "8 0 obj\n[ << /Image 9 0 R /OC 6 0 R >> ]\nendobj\n";
    let alternate = one_pixel_image(9, 0xFF, "");
    let (raster, reports) = interpret(pdf(
        TWO_GROUPS,
        "/XObject << /Im 7 0 R >>",
        DRAW_IMAGE,
        "",
        &format!("{GROUP}{SECOND_GROUP}{base}{alternates}{alternate}"),
    ));
    assert_eq!(pixel(&raster, 50, 50), [0, 0, 0, 255], "the base image");
    assert!(reports.is_empty(), "{reports:?}");
}

/// Step d)'s closing sentence, which inverts what the 2020 clause asked for: "[f]urthermore if
/// the image dictionary that forms the value of the Image key of the selected alternate
/// contains an OC entry, then that OC in the image dictionary shall not be examined."
///
/// So the selected alternate is drawn even though its own image `XObject` carries Table 87's
/// `/OC` naming a group that is off.
#[test]
fn the_selected_alternates_own_image_group_is_not_examined() {
    let base = one_pixel_image(7, 0x00, "/Alternates 8 0 R");
    let alternates = "8 0 obj\n[ << /Image 9 0 R /OC 6 0 R >> ]\nendobj\n";
    let alternate = one_pixel_image(9, 0xFF, "/OC 5 0 R");
    let (raster, reports) = interpret(pdf(
        TWO_GROUPS,
        "/XObject << /Im 7 0 R >>",
        DRAW_IMAGE,
        "",
        &format!("{GROUP}{SECOND_GROUP}{base}{alternates}{alternate}"),
    ));
    assert_eq!(
        pixel(&raster, 50, 50),
        [255, 255, 255, 255],
        "the alternate the dictionary selected"
    );
    assert!(reports.is_empty(), "{reports:?}");
}

/// Table 89 makes `/Image` required. An alternate that states none identifies no alternate to
/// be rendered, which is step e)'s condition — and the document's defect is named rather than
/// swallowed by it.
#[test]
fn an_alternate_that_states_no_image_is_reported_and_the_base_is_drawn() {
    let base = one_pixel_image(7, 0x00, "/Alternates 8 0 R");
    let alternates = "8 0 obj\n[ << /OC 6 0 R >> ]\nendobj\n";
    let (raster, reports) = interpret(pdf(
        TWO_GROUPS,
        "/XObject << /Im 7 0 R >>",
        DRAW_IMAGE,
        "",
        &format!("{GROUP}{SECOND_GROUP}{base}{alternates}"),
    ));
    assert_eq!(pixel(&raster, 50, 50), [0, 0, 0, 255], "the base image");
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(reports[0].contains("states no /Image"), "{reports:?}");
}

/// §8.11.4.3's Table 99 `/ListMode` `VisiblePages` needs one question answered about the file:
///
/// > Display only those groups in the Order array that are referenced by one or more visible
/// > pages.
///
/// The clause does not say what *referenced by* means, so `groups_referenced_by` takes the three
/// places §8.11 puts an `/OC` — the page's `/Resources /Properties`, which a `BDC /OC` names; an
/// `XObject`'s own entry; and an annotation's — and every group a membership dictionary's
/// `/OCGs` or `/VE` mentions. A group nothing on the page names is what the entry exists to
/// hide, and that is the case with no witness in the corpus: `visibility_expressions.pdf` is the
/// one document that states the entry, and its page reaches all three of its groups.
/// §8.11.3.2's `DP` form: a group referenced by a page that draws nothing in it.
///
/// > In addition, a DP marked-content operator may be placed in a page's content stream to force
/// > a reference to an optional content group or groups on the page, even when the page has no
/// > current content in that layer.
///
/// The sentence's only consequence is the *reference*: a marked-content point marks nothing, so
/// nothing is drawn either way, and the one entry that asks who references what is Table 99's
/// `/ListMode /VisiblePages`. `groups_referenced_by` answers it from the page's resources rather
/// than by interpreting the stream, so a `DP` naming a property in `/Properties` is counted by
/// construction — which is what this test pins, because a walk that ever became an
/// interpretation would drop the case silently.
#[test]
fn a_group_named_only_by_a_marked_content_point_is_still_referenced() {
    let bytes = pdf(
        "/OCProperties << /OCGs [5 0 R 6 0 R] /D << >> >>",
        "/Properties << /forced 5 0 R >>",
        // No `BDC` at all: the page draws one square in no layer, and states the group with a
        // marked-content *point*, which is the clause's own "no current content in that layer".
        "/OC /forced DP 20 20 60 60 re f",
        "",
        "5 0 obj\n<< /Type /OCG /Name (forced by DP) >>\nendobj\n\
         6 0 obj\n<< /Type /OCG /Name (named by nothing on this page) >>\nendobj\n",
    );
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let found = pdf_model::optional_content::groups_referenced_by(&document, &page);
    let numbers: Vec<u32> = found.iter().map(|id| id.number).collect();
    assert_eq!(
        numbers,
        vec![5],
        "the DP'd group is referenced by the page and 6, which nothing names, is not"
    );

    // And it is still a page that draws: a marked-content point governs nothing.
    assert!(
        drew(&render(pdf(
            "/OCProperties << /OCGs [5 0 R] /D << /OFF [5 0 R] >> >>",
            "/Properties << /forced 5 0 R >>",
            "/OC /forced DP 20 20 60 60 re f",
            "",
            "5 0 obj\n<< /Type /OCG /Name (forced by DP) >>\nendobj\n",
        ))),
        "a DP is a point, so switching its group off must not hide the square beside it"
    );
}

#[test]
fn a_page_names_the_groups_its_content_annotations_and_forms_reach() {
    let bytes = pdf(
        "/OCProperties << /OCGs [5 0 R 6 0 R 7 0 R 8 0 R 9 0 R] /D << >> >>",
        "/Properties << /oc 5 0 R /ocmd 10 0 R >> /XObject << /Fx 11 0 R >>",
        MARKED_SQUARE,
        "/Annots [12 0 R]",
        "5 0 obj\n<< /Type /OCG /Name (named by the page) >>\nendobj\n\
         6 0 obj\n<< /Type /OCG /Name (named by a membership dictionary) >>\nendobj\n\
         7 0 obj\n<< /Type /OCG /Name (named by a visibility expression) >>\nendobj\n\
         8 0 obj\n<< /Type /OCG /Name (named by a form XObject) >>\nendobj\n\
         9 0 obj\n<< /Type /OCG /Name (named by nothing on this page) >>\nendobj\n\
         10 0 obj\n<< /Type /OCMD /OCGs [6 0 R] /VE [/Not 7 0 R] >>\nendobj\n\
         11 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /OC 8 0 R \
         /Length 0 >>\nstream\n\nendstream\nendobj\n\
         12 0 obj\n<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] /OC 6 0 R >>\nendobj\n",
    );
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let found = pdf_model::optional_content::groups_referenced_by(&document, &page);
    let numbers: Vec<u32> = found.iter().map(|id| id.number).collect();
    assert_eq!(
        numbers,
        vec![5, 6, 7, 8],
        "every group the page reaches, and only those: 9 is in /OCGs and on no page"
    );
}
