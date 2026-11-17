//! Table 232 regression cases for a live script `charLimit` and a subsequent text value.
//! The `ScriptRunner` supplies edit results; this does not claim to execute ECMAScript.

#![expect(
    clippy::expect_used,
    reason = "exact fixture assertions explain failures"
)]

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_model::view::{
    Entered, Property, ScriptEdit, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite, ViewState,
};
use pdf_syntax::{Document, ObjectId};

fn text_field_pdf(stored_limit: u32) -> Document {
    let objects = [
        r"<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /JavaScript /JS (applyEdits\(\);) >> /AcroForm << /Fields [4 0 R] /NeedAppearances true /DA (/Helv 10 Tf 0 g) /DR << /Font << /Helv 5 0 R >> >> >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> /Annots [4 0 R] >>".to_owned(),
        format!("<< /Type /Annot /Subtype /Widget /FT /Tx /T (Field) /V (old) /MaxLen {stored_limit} /Rect [10 10 190 40] /P 3 0 R >>"),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = write!(bytes, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref = bytes.len();
    let _ = write!(
        bytes,
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len().saturating_add(1)
    );
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len().saturating_add(1)
    );
    Document::open(bytes.into_bytes()).expect("valid exact-xref PDF 1.7 fixture")
}

#[derive(Debug)]
struct OrderedEdits(Vec<ScriptEdit>);

impl ScriptRunner for OrderedEdits {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits: if event.site == ScriptSite::OpenAction {
                self.0.clone()
            } else {
                Vec::new()
            },
            report: Vec::new(),
        }
    }
}

fn property(limit: u32) -> ScriptEdit {
    ScriptEdit::Property {
        field: "Field".to_owned(),
        widget: None,
        property: Property::CharLimit(limit),
    }
}

fn value(text: &str) -> ScriptEdit {
    ScriptEdit::Value {
        field: "Field".to_owned(),
        value: text.to_owned(),
    }
}

fn run(stored: u32, edits: Vec<ScriptEdit>) -> (ViewState, Document) {
    let document = text_field_pdf(stored);
    assert!(!document.was_recovered(), "the exact xref is authoritative");
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(Arc::new(OrderedEdits(edits))));
    view.run_open_scripts(&document, 0);
    (view, document)
}

#[derive(Debug, PartialEq)]
struct Observation {
    live_value: String,
    live_char_limit: Option<u32>,
    saved_value: String,
    saved_max_len: i64,
}

fn observe(document: &Document, view: &ViewState) -> Observation {
    let live_value = view
        .field_value(document, "Field")
        .map(|value| value.text)
        .expect("live field value");
    let live_char_limit = view.script_properties("Field").iter().find_map(|property| {
        if let Property::CharLimit(limit) = property {
            Some(*limit)
        } else {
            None
        }
    });
    let saved =
        Document::open(view.save(document).expect("save").bytes).expect("saved PDF reopens");
    assert!(!saved.was_recovered(), "the saved xref is authoritative");
    let field_object = saved.get(ObjectId::new(4, 0));
    let field = field_object.as_dict().expect("field dictionary");
    let value = pdf_syntax::text_string::text_string(
        saved.get_key(field, "V").as_string().expect("text /V"),
    );
    let max_len = saved
        .get_key(field, "MaxLen")
        .as_integer()
        .expect("saved /MaxLen");
    Observation {
        live_value,
        live_char_limit,
        saved_value: value,
        saved_max_len: max_len,
    }
}

#[test]
fn stored_max_len_control_limits_a_script_value_without_a_property_edit() {
    let (view, document) = run(3, vec![value("abcdef")]);
    assert_eq!(
        observe(&document, &view),
        Observation {
            live_value: "abc".to_owned(),
            live_char_limit: None,
            saved_value: "abc".to_owned(),
            saved_max_len: 3,
        }
    );
}

#[test]
fn a_value_within_the_stored_limit_stays_whole() {
    let (view, document) = run(3, vec![value("abc")]);
    assert_eq!(
        observe(&document, &view),
        Observation {
            live_value: "abc".to_owned(),
            live_char_limit: None,
            saved_value: "abc".to_owned(),
            saved_max_len: 3,
        }
    );
}

#[test]
fn a_reduced_live_char_limit_applies_to_a_later_value_edit() {
    let (view, document) = run(5, vec![property(3), value("abcdef")]);
    assert_eq!(
        observe(&document, &view),
        Observation {
            live_value: "abc".to_owned(),
            live_char_limit: Some(3),
            saved_value: "abc".to_owned(),
            saved_max_len: 3,
        }
    );
}

#[test]
fn a_raised_live_char_limit_applies_to_a_later_value_edit() {
    let (view, document) = run(3, vec![property(5), value("abcdef")]);
    assert_eq!(
        observe(&document, &view),
        Observation {
            live_value: "abcde".to_owned(),
            live_char_limit: Some(5),
            saved_value: "abcde".to_owned(),
            saved_max_len: 5,
        }
    );
}

#[test]
fn an_open_script_lowering_char_limit_applies_to_later_public_input() {
    let (mut view, document) = run(5, vec![property(3)]);
    let accepted = view.set_field(&document, "Field", &Entered::Text("abcdef".to_owned()));
    let observed = observe(&document, &view);
    assert_eq!(
        (accepted, observed),
        (
            1,
            Observation {
                live_value: "abc".to_owned(),
                live_char_limit: Some(3),
                saved_value: "abc".to_owned(),
                saved_max_len: 3,
            }
        )
    );
}

#[test]
fn an_open_script_raising_char_limit_applies_to_later_public_input() {
    let (mut view, document) = run(3, vec![property(5)]);
    let accepted = view.set_field(&document, "Field", &Entered::Text("abcdef".to_owned()));
    let observed = observe(&document, &view);
    assert_eq!(
        (accepted, observed),
        (
            1,
            Observation {
                live_value: "abcde".to_owned(),
                live_char_limit: Some(5),
                saved_value: "abcde".to_owned(),
                saved_max_len: 5,
            }
        )
    );
}
