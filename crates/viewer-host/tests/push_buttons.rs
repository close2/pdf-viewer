//! ISO 32000-2 §12.7.5.2.2's push-button in a host that places controls over the page.
//!
//! A push-button "responds immediately to user input without retaining a permanent value", so the
//! only thing it shows is its widget's appearance — and §6.3.2.2 makes drawing that appearance the
//! processor's obligation. So no host places a toolkit button over one (ADR 1357): the page draws
//! the producer's `/AP`, the pointer reaches the widget as it reaches every page-drawn widget, and
//! the keyboard reaches it through [`viewer_host::pressed`]. These are the two decisions the native
//! windows are built on, checked without a display.

#![expect(
    clippy::panic,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use pdf_model::form::{ChoiceControl, Control, TextControl};
use pdf_syntax::ObjectId;
use viewer_core::{Answer, Command, DocumentId, FocusMove, Query, Viewer};
use viewer_host::{Key, Modifiers, Pressed, control_kind, pressed};

/// The identity the one document is opened under.
const DOCUMENT: DocumentId = DocumentId(1);

/// The push-button whose `/A` goes to the second page.
const BUTTON: ObjectId = ObjectId::new(5, 0);
/// The push-button Table 227 bit 1 locks.
const LOCKED: ObjectId = ObjectId::new(6, 0);
/// A text field, which a host places an entry over.
const TYPED: ObjectId = ObjectId::new(7, 0);

/// Writes object bodies numbered from 1 as a file with §7.5.4's table and a trailer.
fn assembled(objects: &[&str]) -> Vec<u8> {
    use std::fmt::Write as _;
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Two pages; the first holds a push-button going to the second, a locked one, and a text field.
fn form() -> Vec<u8> {
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R 6 0 R 7 0 R] >> >>",
        "<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Tabs /R \
         /Annots [5 0 R 6 0 R 7 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] >>",
        "<< /Type /Annot /Subtype /Widget /FT /Btn /Ff 65536 /T (next) /Rect [20 150 120 180] \
         /F 4 /AP << /N 8 0 R >> /A << /S /GoTo /D [4 0 R /Fit] >> >>",
        "<< /Type /Annot /Subtype /Widget /FT /Btn /Ff 65537 /T (locked) /Rect [20 100 120 130] \
         /F 4 /AP << /N 8 0 R >> /A << /S /GoTo /D [4 0 R /Fit] >> >>",
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (typed) /Rect [20 50 220 80] /F 4 \
         /AP << /N 8 0 R >> >>",
        "<< /Type /XObject /Subtype /Form /BBox [0 0 100 30] /Length 24 >>\nstream\n\
         0 0 1 rg 0 0 100 30 re f\nendstream",
    ])
}

/// A viewer with [`form`] open and its events drained.
fn opened() -> Viewer {
    let mut viewer = Viewer::new(600, 400, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: form().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
}

/// Moves §12.5.1's focus until it is on `wanted`, which the fixture's three widgets reach in turn.
fn focus_on(viewer: &mut Viewer, wanted: ObjectId) {
    for _ in 0..4 {
        viewer
            .handle(Command::Focused(FocusMove::Next))
            .for_each(drop);
        if let Answer::Focus { object, .. } = viewer.query(Query::Focus)
            && object == wanted
        {
            return;
        }
    }
    panic!("§12.5.1's walk never reached {wanted:?}");
}

/// Which page is showing.
fn showing(viewer: &Viewer) -> usize {
    match viewer.query(Query::CurrentPage) {
        Answer::Page { index, .. } => index,
        other => panic!("the current page is answered: {other:?}"),
    }
}

/// Every §12.7.5 control a host is handed: placed exactly where the page leaves the appearance off.
///
/// [`viewer_host::ControlKind::is_placed`] decides where a native window builds a widget and
/// [`pdf_model::form::Control::is_delegable`] decides which appearances the page leaves out for
/// one. If the two disagree, a field is either shown twice or not at all, so every variant is
/// asked of both.
#[test]
fn the_controls_a_host_places_are_the_appearances_it_was_given() {
    let every = [
        Control::PushButton,
        Control::CheckBox { on: false },
        Control::RadioButton {
            on: true,
            no_toggle_to_off: true,
            in_unison: false,
        },
        Control::Text(TextControl::default()),
        Control::Choice(ChoiceControl::default()),
        Control::Choice(ChoiceControl {
            combo: true,
            ..ChoiceControl::default()
        }),
        Control::Signature,
        Control::Unstated,
    ];
    for control in &every {
        assert_eq!(
            control_kind(control).is_placed(),
            control.is_delegable(),
            "{control:?}: a control is placed exactly where the page leaves the appearance off"
        );
    }
    assert!(
        !control_kind(&Control::PushButton).is_placed(),
        "§12.7.5.2.2's push-button shows its own appearance and no toolkit button"
    );
}

/// Space and Enter press the push-button §12.5.1's focus is on, and the press is its `/A`.
#[test]
fn space_or_enter_on_a_focused_push_button_activates_it() {
    let mut viewer = opened();
    focus_on(&mut viewer, BUTTON);
    for key in [Key::Space, Key::Enter] {
        match pressed(&viewer, key, Modifiers::NONE) {
            Some(Pressed::Activates { name, annotation }) => {
                assert_eq!(annotation, BUTTON);
                assert_eq!(name.qualified, "next");
            }
            other => panic!("{key:?} on the focused push-button presses it: {other:?}"),
        }
    }
    // Every other key is the key table's, and so is a press with Control held.
    assert_eq!(pressed(&viewer, Key::Tab, Modifiers::NONE), None);
    assert_eq!(pressed(&viewer, Key::Right, Modifiers::NONE), None);
    let ctrl = Modifiers {
        shift: false,
        ctrl: true,
    };
    assert_eq!(pressed(&viewer, Key::Enter, ctrl), None);

    assert_eq!(showing(&viewer), 0);
    viewer.handle(Command::Activate(BUTTON)).for_each(drop);
    assert_eq!(
        showing(&viewer),
        1,
        "the button's /A is §12.6.4.2's go-to, and the press performs it"
    );
}

/// Table 227 bit 1: "any associated widget annotations should not interact with the user".
#[test]
fn a_locked_push_button_is_refused_by_name() {
    let mut viewer = opened();
    focus_on(&mut viewer, LOCKED);
    let refused = pressed(&viewer, Key::Space, Modifiers::NONE);
    assert!(
        matches!(&refused, Some(Pressed::ReadOnly { name }) if name.qualified == "locked"),
        "{refused:?}"
    );
    assert!(
        refused
            .and_then(|refused| refused.note())
            .is_some_and(|said| said.contains("Table 227")),
        "the refusal says which flag"
    );
}

/// A key on any other widget is the page's: Space still turns the page from a text field's focus.
#[test]
fn a_key_on_any_other_widget_is_the_key_tables() {
    let mut viewer = opened();
    focus_on(&mut viewer, TYPED);
    assert_eq!(pressed(&viewer, Key::Space, Modifiers::NONE), None);
    assert_eq!(pressed(&viewer, Key::Enter, Modifiers::NONE), None);
    // And with nothing focused at all.
    assert_eq!(pressed(&opened(), Key::Enter, Modifiers::NONE), None);
}
