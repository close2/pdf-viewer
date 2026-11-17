//! Table 226 defines `/TM` on a field without marking it inheritable. Table 240
//! uses that field's mapping name for click coordinates, falling back to its name.
//! A separate widget must read its own terminal field, without inheriting from a
//! different field above it. The literal bodies distinguish these cases.
#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "small exact-xref fixture"
)]

use pdf_model::action::Action;
use pdf_model::submission::{Click, Format, compose};
use pdf_model::view::ViewState;
use pdf_syntax::{Document, FileBytes, Object, ObjectId};
use std::fmt::Write as _;

fn assembled(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn composed(
    parent_mapping: Option<&str>,
    terminal_mapping: Option<&str>,
    separate: bool,
) -> String {
    let tm = parent_mapping.map_or_else(String::new, |name| format!("/TM ({name}) "));
    let terminal_tm = terminal_mapping.map_or_else(String::new, |name| format!("/TM ({name}) "));
    let widget_id = if separate { 8 } else { 5 };
    let widget_entries =
        "/Type /Annot /Subtype /Widget /Rect [20 30 100 60] /F 4 /A 6 0 R /AP << /N 7 0 R >>";
    let terminal_entries = if separate {
        "/Kids [8 0 R]"
    } else {
        widget_entries
    };
    let mut body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> /Annots [{widget_id} 0 R] >>\nendobj\n\
         4 0 obj\n<< /T (group) {tm}/Kids [5 0 R] >>\nendobj\n\
         5 0 obj\n<< /FT /Btn /Ff 65536 /Parent 4 0 R /T (submit) {terminal_tm}{terminal_entries} >>\nendobj\n\
         6 0 obj\n<< /S /SubmitForm /F << /FS /URL /F (https://example.invalid/send) >> /Flags 20 >>\nendobj\n\
         7 0 obj\n<< /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 80 30] /Resources << >> /Length 25 >>\nstream\nq 0.2 g 0 0 80 30 re f Q\nendstream\nendobj\n"
    );
    if separate {
        let _ = write!(
            body,
            "8 0 obj\n<< {widget_entries} /Parent 5 0 R >>\nendobj\n"
        );
    }
    let document = Document::open(FileBytes::from(assembled(&body))).expect("complete source PDF");
    assert!(!document.was_recovered(), "exact source xref");
    let action = pdf_model::action::read(&document, &Object::Reference(ObjectId::new(6, 0)));
    let [Action::SubmitForm(submit)] = action.as_slice() else {
        panic!("SubmitForm: {action:?}")
    };
    let submission = compose(
        &document,
        &ViewState::of(&document),
        submit,
        Some(&Click {
            point: (35.0, 45.0),
            widget: Some(ObjectId::new(widget_id, 0)),
        }),
    )
    .expect("click submission composes");
    assert_eq!(submission.format, Format::HtmlForm);
    String::from_utf8(submission.body).expect("URL-encoded body")
}

#[test]
fn nonterminal_mapping_is_not_inherited_by_a_terminal_field_or_its_widget() {
    // Both representations have the same terminal-field semantics. Resolve widget-to-field
    // association, but do not inherit from a different field above the terminal one.
    let expected = [false, true].map(|separate| {
        let fallback = composed(None, None, separate);
        // Both spellings encode the same ASCII field name; encoding the period is optional.
        assert!(matches!(
            fallback.as_str(),
            "group.submit.x=15&group.submit.y=15" | "group%2Esubmit.x=15&group%2Esubmit.y=15"
        ));
        assert_eq!(
            composed(Some("parentmap"), Some("terminalmap"), separate),
            "terminalmap.x=15&terminalmap.y=15"
        );
        assert_eq!(composed(None, Some(" "), separate), "x=15&y=15");
        fallback
    });
    let actual = [
        composed(Some("parentmap"), None, false),
        composed(Some("parentmap"), None, true),
    ];
    assert_eq!(
        actual, expected,
        "Table 226 does not designate TM inheritable; both merged and separate widgets must use their own terminal field's fallback name"
    );
}
