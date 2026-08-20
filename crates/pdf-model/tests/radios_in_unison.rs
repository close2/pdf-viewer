//! Table 229 bit 26, in the direction that needs code.
//!
//! ISO 32000-2 §12.7.5.2.1 makes the flag binding on a *reader*:
//!
//! > For button fields, bits 15, 16, 17, and 26 shall indicate the intended behaviour of the button
//! > field. An interactive PDF processor shall follow the intended behaviour, as defined in "Table
//! > 229 -Field flags specific to button fields" and clauses 12.7.5.2.2, "Push-buttons", and
//! > 12.7.5.2.4, "Radio buttons".
//!
//! **The half everyone expects was already obeyed by accident, and the other half was not.** A
//! widget is on when its `/AP /N` states a stream under the name `/V` holds, so two widgets sharing
//! an on-state name go on together by construction — which is Table 229 bit 26 *set*, obeyed by an
//! implementation that had never read the bit. What was not obeyed is §12.7.5.2.3's sentence for
//! the bit **clear**:
//!
//! > For radio buttons, the same behaviour shall occur only if the RadiosInUnison flag is set. If
//! > it is not set, at most one radio button in a field shall be set at a time.
//!
//! and §12.7.5.2.4's NOTE says the same thing from the other end: "[a]n exception occurs when
//! multiple radio buttons in a field have the same on state and the RadiosInUnison flag is set. In
//! that case, turning on one of the buttons turns on all of them."
//!
//! # Why this is a hand-written pair and not a corpus document
//!
//! Trap 8. `cargo run --release -p pdf-model --example field_flag_census` over all 1293 documents
//! this tree can reach counts **no** widget setting bit 26 on a `Btn`, and — the population that
//! actually matters here, which no flag count would have found — **no** radio field at all whose
//! widgets share an `/AP /N` on-state name, with the flag or without it. So the corpus cannot see
//! this clause in either direction, and the instrument is a pair of files differing in one bit,
//! which is the shape `pdf-syntax/tests/cross_references.rs` uses for the same reason.
//!
//! # Which button stays on is a documented choice
//!
//! The clause states none, and the *file* cannot: `/V` is a name, and a producer that gave two
//! buttons the same name has written a document whose own value cannot tell them apart. Table 230
//! is the standard's own instrument for a producer that wants them distinguishable — the `/AP`
//! names "may use numerical position (starting with 0) of the annotation in the Kids array" — so
//! this reader takes the **first** kid answering to the name, which is the field's own order and
//! the order `/Opt` is indexed by. Asserted here so that the choice is visible rather than implied.

#![expect(
    clippy::expect_used,
    reason = "a test's failure is its purpose, and these helpers run outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]
#![expect(
    clippy::doc_markdown,
    reason = "the module comment quotes §12.7.5.2.4's NOTE, which spells `RadiosInUnison` without \
              backticks because the standard does. `CLAUDE.md`'s rule is that quotation marks mean \
              verbatim, and the conformance checker reads these blockquotes against `doc/md/` — so \
              a backtick added to satisfy a style lint would turn a quotation into a paraphrase \
              claiming to be one"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

use pdf_model::form::Control;
use pdf_model::view::{Entered, ViewState};

/// Pixel budget, far above the 300×100 page this fixture builds.
const GENEROUS: u64 = 1 << 30;

/// The page the fixture draws on, in points.
const PAGE: (u32, u32) = (300, 100);

/// Table 229's bit 16, `Radio`, one-based as the table numbers them.
const RADIO: u32 = 1 << 15;
/// Table 229's bit 26, `RadiosInUnison`.
const IN_UNISON: u32 = 1 << 25;

/// The three buttons' rectangles, in the page's own coordinates.
const FIRST: [u32; 4] = [10, 10, 50, 50];
const SECOND: [u32; 4] = [110, 10, 150, 50];
const THIRD: [u32; 4] = [210, 10, 250, 50];

/// A radio button field of three kids, two of which share the on state `Yes`.
///
/// The third answers to `No`, so it is the control: nothing this clause says should ever turn it on
/// while `/V` is `Yes`, whichever way bit 26 is set, and a rule that turned *every* button on would
/// show up here rather than in a count.
///
/// Each on state's appearance fills its whole `/BBox`, so "is this button drawn on" is a question
/// about a rectangle of the raster and needs no font. The off state draws nothing at all, which is
/// what makes the two states tell each other apart in ink.
fn fixture(flags: u32) -> Vec<u8> {
    let (width, height) = PAGE;
    let on = |number: u32| {
        let contents = "0 0 1 rg 0 0 40 40 re f";
        format!(
            "{number} 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 40 40] /Length {} >>\n\
             stream\n{contents}\nendstream\nendobj\n",
            contents.len().saturating_add(1)
        )
    };
    let off = |number: u32| {
        format!(
            "{number} 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 40 40] /Length 1 >>\n\
             stream\n\nendstream\nendobj\n"
        )
    };
    let button = |number: u32, rect: [u32; 4], state: &str, on_stream: u32, off_stream: u32| {
        format!(
            "{number} 0 obj\n<< /Type /Annot /Subtype /Widget /Parent 5 0 R \
             /Rect [{} {} {} {}] /F 4 /AS /Off \
             /AP << /N << /{state} {on_stream} 0 R /Off {off_stream} 0 R >> >> >>\nendobj\n",
            rect[0], rect[1], rect[2], rect[3]
        )
    };

    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] \
         /Resources << >> /Contents 4 0 R /Annots [6 0 R 7 0 R 8 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /FT /Btn /T (choice) /Ff {flags} /V /Off /Kids [6 0 R 7 0 R 8 0 R] >>\nendobj\n\
         {}{}{}{}{}",
        button(6, FIRST, "Yes", 9, 10),
        button(7, SECOND, "Yes", 9, 10),
        button(8, THIRD, "No", 9, 10),
        on(9),
        off(10),
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

/// Which of the three buttons this reader says are on, after `/V` has been set to `Yes`.
///
/// In `/Kids` order, which is `/Annots` order here, so the three booleans line up with the three
/// rectangles above.
fn switched_on(flags: u32) -> Vec<bool> {
    let document = Document::open(fixture(flags)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let mut view = ViewState::of(&document);
    let touched = view.set_field(&document, "choice", &Entered::Text("Yes".to_owned()));
    assert_eq!(touched, 3, "the field owns three widgets");

    let fields = pdf_model::form::fields(&document, &page, &view);
    assert_eq!(fields.len(), 1, "one field, whatever its buttons do");
    assert!(
        matches!(fields[0].control, Control::RadioButton { .. }),
        "Table 229 bit 16 makes it a radio set: {:?}",
        fields[0].control
    );
    fields[0].widgets.iter().map(|widget| widget.on).collect()
}

/// How many opaque pixels a user-space rectangle of the page holds, after the same edit.
fn inked(flags: u32) -> Vec<usize> {
    let document = Document::open(fixture(flags)).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let mut view = ViewState::of(&document);
    let _ = view.set_field(&document, "choice", &Entered::Text("Yes".to_owned()));
    let list = pdf_model::content::interpret_with(&document, &page, &view).display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported");
    [FIRST, SECOND, THIRD]
        .iter()
        .map(|rect| ink(&raster, *rect))
        .collect()
}

/// Opaque pixels inside one user-space rectangle. The raster's rows run down from the page's top.
fn ink(raster: &pdf_render::Raster, rect: [u32; 4]) -> usize {
    let (top, bottom) = (
        PAGE.1.saturating_sub(rect[3]),
        PAGE.1.saturating_sub(rect[1]),
    );
    (rect[0]..rect[2])
        .flat_map(|x| (top..bottom).map(move |y| (x, y)))
        .filter(|(x, y)| {
            let index = (y.saturating_mul(raster.width).saturating_add(*x) as usize)
                .saturating_mul(4)
                .saturating_add(3);
            raster.data.get(index).copied().unwrap_or_default() > 0
        })
        .count()
}

/// Bit 26 set: "if one is checked, they are all checked."
#[test]
fn with_the_flag_set_both_buttons_that_share_an_on_state_go_on() {
    assert_eq!(
        switched_on(RADIO | IN_UNISON),
        vec![true, true, false],
        "§12.7.5.2.4's NOTE: turning on one of them turns on all of them, and the button whose \
         on state is a different name is not one of them"
    );
}

/// Bit 26 clear: "at most one radio button in a field shall be set at a time."
#[test]
fn with_the_flag_clear_only_the_first_of_them_does() {
    assert_eq!(
        switched_on(RADIO),
        vec![true, false, false],
        "§12.7.5.2.3 makes the buttons mutually exclusive, and the first in /Kids is this \
         reader's documented choice for which one the value names"
    );
}

/// And the page draws what the description says, which is the half a host cannot check.
///
/// ADR 0235's finding was that these two paths can be wrong differently — the constructed
/// appearance and the description of the field are computed by different code and were, at the
/// time, disagreeing about whether an edited check box was on. Both go through
/// `Field::replacement_state` now, and this is what says so in pixels.
#[test]
fn the_page_draws_the_same_buttons_the_description_names() {
    let unison = inked(RADIO | IN_UNISON);
    let exclusive = inked(RADIO);

    assert!(unison[0] > 0 && unison[1] > 0, "both share the on state");
    assert_eq!(unison[2], 0, "and the third answers to a different name");

    assert!(exclusive[0] > 0, "the first is the one that goes on");
    assert_eq!(
        exclusive[1], 0,
        "and the second is off, which is the whole of what bit 26 being clear requires"
    );
    assert_eq!(exclusive[2], 0);
    assert_eq!(
        unison[0], exclusive[0],
        "the button that is on is drawn identically either way"
    );
}

/// The flag reaches a host either way, which is what it was already doing before it was obeyed.
#[test]
fn the_flag_crosses_to_a_host_as_well_as_being_obeyed() {
    for (flags, expected) in [(RADIO, false), (RADIO | IN_UNISON, true)] {
        let document = Document::open(fixture(flags)).expect("the fixture is a valid PDF");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        let view = ViewState::of(&document);
        let fields = pdf_model::form::fields(&document, &page, &view);
        let Control::RadioButton { in_unison, .. } = fields[0].control else {
            panic!("Table 229 bit 16 makes it a radio set");
        };
        assert_eq!(
            in_unison, expected,
            "Table 229 bit 26, as the file states it"
        );
    }
}

/// The file's own `/AS` is left alone, which is the boundary of the rule rather than a gap.
///
/// §12.7.5.2.3: "[t]he value of the V key shall also be the value of the AS key. If they are not
/// equal, then the value of the AS key shall be used instead of the V key to determine which
/// appearance to use." A producer that wrote two widgets on has stated which of *its* buttons are
/// on, per widget, and bit 26 is a rule about what happens when a button is turned on — which is
/// something this program does and that file did not ask it to do. Correcting the document here
/// would be trap 5's opposite: inventing a repair nobody can see.
#[test]
fn a_file_that_states_two_buttons_on_is_drawn_as_it_wrote_itself() {
    let stated = String::from_utf8(fixture(RADIO))
        .expect("the fixture is ASCII")
        .replace("/AS /Off\n", "/AS /Yes\n")
        .replace("/AS /Off ", "/AS /Yes ");
    let document = Document::open(stated.into_bytes()).expect("still a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let view = ViewState::of(&document);
    let fields = pdf_model::form::fields(&document, &page, &view);
    let on: Vec<bool> = fields[0].widgets.iter().map(|widget| widget.on).collect();
    assert_eq!(
        on,
        vec![true, true, true],
        "nothing has been edited, so §12.7.5.2.3's `/AS` decides and the file said all three"
    );
}
