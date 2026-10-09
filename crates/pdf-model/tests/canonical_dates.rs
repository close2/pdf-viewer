//! ISO 32000-2 Table 240 bit 10, `CanonicalFormat`, end to end through §12.7.6.2's composition.
//!
//! > (PDF 1.4) If set, any submitted field values representing dates shall be converted to the
//! > standard format described in 7.9.4, "Dates".
//!
//! The bit's NOTE 1 says which values those are: "[t]he interpretation of a form field as a date is
//! not specified explicitly in the field itself but only in the ECMAScript code that processes it".
//! So each fixture field states its date, or its lack of one, in a Table 199 script, and the
//! expected strings are §7.9.4's form cut to the fields each picture states: "the year field
//! (YYYY) shall be present and all other fields may be present but only if all of their preceding
//! fields are also present". ADR 1796.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a test's failure is its purpose, and these helpers run outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]

use std::fmt::Write as _;

use pdf_model::action::Action;
use pdf_model::forms_data::FormsData;
use pdf_model::submission::{Submission, compose};
use pdf_model::view::ViewState;
use pdf_syntax::{Document, Object, ObjectId};

/// One text widget per case, each `/V` a value and each `/AA` the script that says what it is.
///
/// `born` is `AFDate_FormatEx` over a date; `met` is `AFDate_Format`'s menu entry 12, `m/d/yy h:MM
/// tt`, which states a time; `typed` has only a keystroke script; `birthday` has a picture with no
/// year; `due` has a value its picture does not read; `scripted` has a script that is not one call;
/// and `plain` has none at all, which NOTE 1 makes a field no one has said holds a date.
fn form() -> Vec<u8> {
    let fields = [
        (
            "born",
            "01/02/2024",
            "/F",
            "AFDate_FormatEx(\"mm/dd/yyyy\");",
        ),
        ("met", "1/2/24 3:04 pm", "/F", "AFDate_Format(12);"),
        (
            "typed",
            "2024-03-09",
            "/K",
            "AFDate_KeystrokeEx(\"yyyy-mm-dd\");",
        ),
        ("birthday", "3/9", "/F", "AFDate_FormatEx(\"m/d\");"),
        ("due", "soon", "/F", "AFDate_FormatEx(\"mm/dd/yyyy\");"),
        (
            "scripted",
            "01/02/2024",
            "/F",
            "var d = 1; AFDate_FormatEx(\"mm/dd/yyyy\");",
        ),
        ("plain", "01/02/2024", "", ""),
    ];
    // Objects 4 onwards, one per field, each 40 units above the last.
    let numbered: Vec<(usize, usize)> = (4_usize..)
        .zip((0_usize..).step_by(40))
        .take(fields.len())
        .collect();
    let mut references = String::new();
    for (number, _) in &numbered {
        let _ = write!(references, "{number} 0 R ");
    }
    let mut body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [{references}] >> >>\n\
         endobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 400] /Annots [{references}] >>\n\
         endobj\n"
    );
    for ((number, bottom), (name, value, trigger, script)) in numbered.iter().zip(&fields) {
        let additional = if trigger.is_empty() {
            String::new()
        } else {
            format!("/AA << {trigger} << /S /JavaScript /JS ({script}) >> >>")
        };
        let top = bottom.saturating_add(30);
        let _ = write!(
            body,
            "{number} 0 obj\n<< /Type /Annot /Subtype /Widget /Rect [10 {bottom} 190 {top}] /F 4 \
             /FT /Tx /T ({name}) /V ({value}) {additional} >>\nendobj\n"
        );
    }

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

/// The fixture's fields submitted as FDF under these Table 240 flags.
fn submitted(flags: u32) -> Submission {
    let document = Document::open(pdf_syntax::FileBytes::from(form())).expect("the fixture parses");
    let view = ViewState::of(&document);
    let source = format!(
        "%PDF-2.0\n1 0 obj\n<< /Type /Catalog >>\nendobj\n\
         2 0 obj\n<< /S /SubmitForm /F << /FS /URL /F (https://example.invalid/cgi) >> \
         /Flags {flags} >>\nendobj\n\
         trailer\n<< /Root 1 0 R /Size 3 >>\n"
    );
    let holder = Document::open(pdf_syntax::FileBytes::from(source.into_bytes()))
        .expect("the action parses");
    let action = match pdf_model::action::read(&holder, &Object::Reference(ObjectId::new(2, 0)))
        .into_iter()
        .next()
    {
        Some(Action::SubmitForm(submit)) => submit,
        other => panic!("Table 239's action is read whole: {other:?}"),
    };
    compose(&document, &view, &action, None).expect("the composition succeeds")
}

/// The FDF body's value for one field, read back by §12.7.8's own reader.
fn value_of(submission: &Submission, name: &str) -> Option<String> {
    let document = Document::open(pdf_syntax::FileBytes::from(submission.body.clone()))
        .expect("the composed FDF parses");
    FormsData::read(&document)
        .expect("an FDF catalog")
        .fields
        .into_iter()
        .find(|field| field.name == name)
        .and_then(|field| field.value)
        .and_then(|value| value.as_string().map(pdf_syntax::text_string))
}

/// A date the library's call states is written as §7.9.4's date, to the fields its picture states.
#[test]
fn a_date_a_fields_script_states_is_submitted_in_the_standard_format() {
    let submission = submitted(512);
    assert_eq!(value_of(&submission, "born").as_deref(), Some("D:20240102"));
    // Two digits under `yy` are the twenty-first century below 50, `scand`'s horizon.
    assert_eq!(
        value_of(&submission, "met").as_deref(),
        Some("D:202401021504")
    );
    assert_eq!(
        value_of(&submission, "typed").as_deref(),
        Some("D:20240309")
    );
    let owed = submission.owed.join("\n");
    assert!(
        owed.contains("GMT"),
        "a time with no offset is read as GMT, and that is said: {owed}"
    );
}

/// What the clause cannot convert is sent as it is and named, and a field no script calls a date
/// is not one.
#[test]
fn a_value_that_is_not_a_date_under_its_picture_is_named_and_sent_as_it_is() {
    let submission = submitted(512);
    for (name, value) in [
        ("birthday", "3/9"),
        ("due", "soon"),
        ("scripted", "01/02/2024"),
        ("plain", "01/02/2024"),
    ] {
        assert_eq!(
            value_of(&submission, name).as_deref(),
            Some(value),
            "{name}"
        );
    }
    let owed = submission.owed.join("\n");
    for (name, why) in [
        ("birthday", "states no year"),
        ("due", "is not a date"),
        ("scripted", "not one call"),
    ] {
        assert!(
            owed.lines()
                .any(|line| line.contains(&format!("{name}'s")) && line.contains(why)),
            "{name}: {owed}"
        );
    }
    assert!(!owed.contains("plain"), "{owed}");
}

/// The control: with the bit clear every value is sent as the field holds it, and nothing is owed.
#[test]
fn with_the_bit_clear_no_value_is_converted() {
    let submission = submitted(0);
    assert_eq!(value_of(&submission, "born").as_deref(), Some("01/02/2024"));
    assert!(
        !submission
            .owed
            .iter()
            .any(|owed| owed.contains("CanonicalFormat")),
        "{:?}",
        submission.owed
    );
}

/// The bit is restricted to no format, so XFDF's body carries the converted date too.
#[test]
fn an_xfdf_submission_carries_the_converted_date() {
    let submission = submitted(512 | 32);
    let body = String::from_utf8_lossy(&submission.body);
    assert!(body.contains("D:20240102"), "{body}");
}
