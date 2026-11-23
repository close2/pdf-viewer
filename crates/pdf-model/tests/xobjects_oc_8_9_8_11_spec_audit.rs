//! Specification audit of ISO 32000-2 §8.9 to §8.11 (images, form XObjects and optional
//! content): `doc/reviews/2195-specification-audit-8-9-8-11.md`.
//!
//! Each `*_spec_audit` test is one behaviour finding and fails on the tree the audit read; the
//! control beside it passes there, so the failure is the finding and not the fixture.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::doc_markdown,
    reason = "small exact-xref fixtures, a pixel index, and the standard quoted verbatim"
)]

use pdf_model::optional_content::Purpose;
use pdf_model::reference::Supply;
use pdf_model::view::ViewState;
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use std::fmt::Write as _;

const WHITE: [u8; 3] = [255, 255, 255];
const BLUE: [u8; 3] = [0, 0, 255];

/// A document of `objects` (numbered from 1; object 1 the catalog) with exact cross-reference
/// offsets and, where `id` is given, a trailer `/ID` of those two byte strings.
fn document(objects: &[String], id: Option<(&str, &str)>) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{object}\nendobj\n", index + 1);
    }
    let xref = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let id = id.map_or_else(String::new, |(first, second)| {
        format!(" /ID [<{first}> <{second}>]")
    });
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R{id} >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

fn stream(dictionary: &str, data: &str) -> String {
    format!(
        "<< {dictionary} /Length {} >>\nstream\n{data}\nendstream",
        data.len()
    )
}

/// The centre pixel of an interpretation's display list at 72 dpi on the CPU oracle.
fn centre_of(interpretation: &pdf_model::Interpretation) -> [u8; 3] {
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("100-point target");
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("fixture rasterizes");
    assert_eq!((raster.width, raster.height), (100, 100));
    let at = ((50 * raster.width + 50) * 4) as usize;
    [raster.data[at], raster.data[at + 1], raster.data[at + 2]]
}

/// The centre pixel of the first page of `bytes`, interpreted under the document's own view.
fn centre(bytes: Vec<u8>) -> [u8; 3] {
    let document = Document::open(bytes).expect("fixture parses");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    centre_of(&pdf_model::interpret(&document, &page))
}

// ---------------------------------------------------------------------------------------------
// 2195-B01: a group with no effect on visibility, inside a visibility expression.
// ---------------------------------------------------------------------------------------------

/// A page filled blue inside `/OC /m BDC … EMC`, where `/m` is the membership dictionary
/// `ocmd` (object 5). Object 6 is a group the default configuration turns OFF, object 7 a group
/// that is ON and whose `/Intent` is `/Design`, which the default configuration's `/View` intent
/// does not cover.
fn layered(ocmd: &str) -> Vec<u8> {
    document(
        &[
            "<< /Type /Catalog /Pages 2 0 R \
             /OCProperties << /OCGs [6 0 R 7 0 R] /D << /OFF [6 0 R] >> >> >>"
                .to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
             /Resources << /Properties << /m 5 0 R >> >> /Contents 4 0 R >>"
                .to_owned(),
            stream("", "/OC /m BDC 0 0 1 rg 0 0 100 100 re f EMC"),
            ocmd.to_owned(),
            "<< /Type /OCG /Name (Off) >>".to_owned(),
            "<< /Type /OCG /Name (Design) /Intent /Design >>".to_owned(),
        ],
        None,
    )
}

/// ISO 32000-2 §8.11.2.3: "If one or more of a group's intents is contained in the current
/// configuration's set of intents, the group shall be used in determining visibility. If there
/// is no match, the group shall have no effect on visibility." So `[/Or off design]` is decided
/// by the OFF group alone, and the content is hidden — as `/OCGs [off design] /P /AnyOn` hides
/// it, which is the control.
#[test]
fn a_visibility_expression_or_ignores_a_group_outside_the_intent_spec_audit() {
    assert_eq!(
        centre(layered("<< /Type /OCMD /OCGs [6 0 R 7 0 R] /P /AnyOn >>")),
        WHITE,
        "control: the /P route leaves the Design group out and hides the content"
    );
    assert_eq!(
        centre(layered("<< /Type /OCMD /VE [/Or 6 0 R 7 0 R] >>")),
        WHITE,
        "§8.11.2.3: a group outside the configuration's intent has no effect, so /Or of an OFF \
         group and that group is the OFF group's answer"
    );
}

/// The same clause through `/Not`: a group with "no effect on visibility" cannot hide content
/// by being negated. The control is the `/P` route's `/AllOff` over the same group, which shows.
#[test]
fn a_visibility_expression_not_of_a_group_outside_the_intent_shows_spec_audit() {
    assert_eq!(
        centre(layered("<< /Type /OCMD /OCGs [7 0 R] /P /AllOff >>")),
        BLUE,
        "control: the /P route gives the Design group no effect and shows the content"
    );
    assert_eq!(
        centre(layered("<< /Type /OCMD /VE [/Not 7 0 R] >>")),
        BLUE,
        "§8.11.2.3: [/Not g] of a group with no effect on visibility does not hide content"
    );
}

// ---------------------------------------------------------------------------------------------
// 2195-B02: an imported page's optional content under the operation's event.
// ---------------------------------------------------------------------------------------------

const TARGET_ID: (&str, &str) = ("0102", "0304");

/// The target document: one page filled blue inside `/OC /g BDC … EMC`, its group ON by the
/// default configuration and OFF by its `/Usage /Print`, which an `/AS` usage application
/// dictionary of event `/Print` applies (§8.11.4.4).
fn target() -> Vec<u8> {
    document(
        &[
            "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [5 0 R] /D << \
             /AS [<< /Event /Print /Category [/Print] /OCGs [5 0 R] >>] >> >> >>"
                .to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
             /Resources << /Properties << /g 5 0 R >> >> /Contents 4 0 R >>"
                .to_owned(),
            stream("", "/OC /g BDC 0 0 1 rg 0 0 100 100 re f EMC"),
            "<< /Type /OCG /Name (Screen only) /Usage << /Print << /PrintState /OFF >> >> >>"
                .to_owned(),
        ],
        Some(TARGET_ID),
    )
}

/// A containing document whose page is one reference `XObject` over the whole page naming
/// `target.pdf`'s first page; its proxy paints nothing.
fn containing() -> Vec<u8> {
    let (first, second) = TARGET_ID;
    document(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
             /Resources << /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>"
                .to_owned(),
            stream("", "/Fm Do"),
            stream(
                &format!(
                    "/Type /XObject /Subtype /Form /BBox [0 0 100 100] /Ref << /F << /Type \
                     /Filespec /F (target.pdf) /UF (target.pdf) >> /Page 0 \
                     /ID [<{first}> <{second}>] >>"
                ),
                "",
            ),
        ],
        Some(("1111", "2222")),
    )
}

fn printing(document: &Document) -> ViewState {
    let mut view = ViewState::of(document);
    view.set_purpose(Purpose::Print);
    view
}

/// ISO 32000-2 §8.11.4.5: "When a document is printed by an interactive PDF processor, usage
/// application dictionaries with an event type Print shall be applied over the current states of
/// optional content groups." The imported page is drawn as part of the printed page (§8.10.4.1),
/// and its groups are the target document's, so the target's `/Print` usage applies to them —
/// the control is the target printed directly, whose layer the event hides.
#[test]
fn an_imported_pages_layers_take_the_print_event_spec_audit() {
    let target_document = Document::open(target()).expect("target parses");
    let page = pdf_model::Pages::new(&target_document)
        .get(0)
        .expect("the target's page");
    let direct = pdf_model::content::interpret_importing(
        &target_document,
        &page,
        &printing(&target_document),
        &pdf_model::FontCache::new(),
        &Supply::NONE,
    );
    assert_eq!(
        centre_of(&direct),
        WHITE,
        "control: printing the target hides its screen-only layer"
    );

    let (supply, refused) = Supply::read([("target.pdf", target())], "the test's own bytes");
    assert!(refused.is_empty(), "the target is a PDF: {refused:?}");
    let containing_document = Document::open(containing()).expect("containing parses");
    let page = pdf_model::Pages::new(&containing_document)
        .get(0)
        .expect("the containing page");
    let viewed = pdf_model::content::interpret_importing(
        &containing_document,
        &page,
        &ViewState::of(&containing_document),
        &pdf_model::FontCache::new(),
        &supply,
    );
    assert_eq!(
        centre_of(&viewed),
        BLUE,
        "control: on screen the imported page's layer is ON and drawn"
    );
    let printed = pdf_model::content::interpret_importing(
        &containing_document,
        &page,
        &printing(&containing_document),
        &pdf_model::FontCache::new(),
        &supply,
    );
    assert_eq!(
        centre_of(&printed),
        WHITE,
        "§8.11.4.5: the print event's usage applies to the imported page's groups too"
    );
}

// ---------------------------------------------------------------------------------------------
// 2195-B03: an appearance stream's own /OC.
// ---------------------------------------------------------------------------------------------

/// A page with one `Square` annotation covering it, printable, whose normal appearance paints
/// black; `annotation_oc` and `appearance_oc` are stated on the annotation dictionary and on the
/// appearance stream's form dictionary. Object 6 is a group the default configuration turns OFF.
fn annotated(annotation_oc: &str, appearance_oc: &str) -> Vec<u8> {
    document(
        &[
            "<< /Type /Catalog /Pages 2 0 R \
             /OCProperties << /OCGs [6 0 R] /D << /OFF [6 0 R] >> >> >>"
                .to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
             /Contents 4 0 R /Annots [5 0 R] >>"
                .to_owned(),
            stream("", ""),
            format!(
                "<< /Type /Annot /Subtype /Square /Rect [0 0 100 100] /F 4 {annotation_oc} \
                 /AP << /N 7 0 R >> >>"
            ),
            "<< /Type /OCG /Name (Off) >>".to_owned(),
            stream(
                &format!("/Type /XObject /Subtype /Form /BBox [0 0 100 100] {appearance_oc}"),
                "0 g 0 0 100 100 re f",
            ),
        ],
        None,
    )
}

/// ISO 32000-2 §12.5.5: "Each appearance stream is a form XObject (see 8.10, "Form XObjects")",
/// and Table 93's `/OC` of a form XObject: "Before the form is processed, its visibility shall
/// be determined based on this entry. If it is determined to be invisible, the entire form shall
/// be skipped". The control is the same group stated on the annotation dictionary (§8.11.3.3),
/// which hides it.
#[test]
fn an_appearance_streams_own_oc_hides_it_spec_audit() {
    assert_eq!(centre(annotated("", "")), [0, 0, 0], "control: drawn");
    assert_eq!(
        centre(annotated("/OC 6 0 R", "")),
        WHITE,
        "control: the annotation's /OC hides it"
    );
    assert_eq!(
        centre(annotated("", "/OC 6 0 R")),
        WHITE,
        "Table 93: an appearance stream is a form XObject, and an invisible one is skipped"
    );
}
