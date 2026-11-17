//! Live JavaScript field flags must reach the model's form-description and submit consumers.
#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "standalone exact-xref spec-audit fixtures"
)]

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_model::action::{Action, read as read_action};
use pdf_model::form::fields as form_fields;
use pdf_model::view::{
    Entered, Property, ScriptEdit, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite, ViewState,
};
use pdf_syntax::{Document, FileBytes, Object, ObjectId};

fn pdf(flags: u32, value: Option<&str>) -> Document {
    let value = value.map_or_else(String::new, |value| format!("/V ({value})"));
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /JavaScript /JS (noop) >> \
         /AcroForm << /Fields [4 0 R] /NeedAppearances true /DA (/Helv 12 Tf 0 g) \
         /DR << /Font << /Helv 5 0 R >> >> >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Annots [4 0 R] >>"
            .to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Widget /Rect [10 10 180 40] /P 3 0 R /F 4 \
             /FT /Tx /T (Field) /Ff {flags} {value} /DA (/Helv 12 Tf 0 g) >>"
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_owned(),
        "<< /S /SubmitForm /F << /FS /URL /F (https://example.invalid/submit) >> \
         /Fields [(Field)] >>"
            .to_owned(),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = vec![0_usize];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        writeln!(out, "{} 0 obj\n{object}\nendobj", index.saturating_add(1))
            .expect("in-memory object write succeeds");
    }
    let xref = out.len();
    writeln!(out, "xref\n0 {}\n0000000000 65535 f ", offsets.len())
        .expect("in-memory xref header write succeeds");
    for offset in offsets.iter().skip(1) {
        writeln!(out, "{offset:010} 00000 n ").expect("in-memory xref entry write succeeds");
    }
    write!(out, "trailer\n<< /Size {} /Root 1 0 R /ID [<01020304> <05060708>] >>\nstartxref\n{xref}\n%%EOF\n", offsets.len())
        .expect("in-memory trailer write succeeds");
    let document =
        Document::open(FileBytes::from(out.into_bytes())).expect("the exact-xref fixture parses");
    assert!(!document.was_recovered(), "the fixture xref is exact");
    document
}

#[derive(Debug)]
struct SetProperty(Property);

impl ScriptRunner for SetProperty {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        let edits = if event.site == ScriptSite::OpenAction {
            vec![ScriptEdit::Property {
                field: "Field".to_owned(),
                widget: None,
                property: self.0.clone(),
            }]
        } else {
            Vec::new()
        };
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits,
            report: Vec::new(),
        }
    }
}

fn opened(document: &Document, property: Property) -> ViewState {
    let mut view = ViewState::of(document);
    view.run_scripts_with(Some(Arc::new(SetProperty(property))));
    view.run_open_scripts(document, 0);
    view
}

fn first_form_field(document: &Document, view: &ViewState) -> pdf_model::form::FormField {
    let page = pdf_model::Pages::new(document).get(0).expect("one page");
    let mut fields = form_fields(document, &page, view);
    assert_eq!(fields.len(), 1, "one field is reachable on the page");
    fields.remove(0)
}

fn submit(document: &Document, view: &ViewState) -> pdf_model::submission::Submission {
    let actions = read_action(document, &Object::Reference(ObjectId::new(6, 0)));
    let [Action::SubmitForm(action)] = actions.as_slice() else {
        panic!("the fixture contains one SubmitForm action: {actions:?}");
    };
    pdf_model::submission::compose(document, view, action, None)
        .expect("the form submission composes without network I/O")
}

fn has_property(view: &ViewState, property: &Property) -> bool {
    view.script_properties("Field").contains(property)
}

#[test]
fn live_required_override_reaches_the_form_description() {
    let stored_clear = pdf(0, None);
    let clear_view = ViewState::of(&stored_clear);
    let stored_required = pdf(2, None);
    let required_view = ViewState::of(&stored_required);
    let live_true = opened(&stored_clear, Property::Required(true));
    let live_false = opened(&stored_required, Property::Required(false));
    let observations = (
        first_form_field(&stored_clear, &clear_view).required,
        first_form_field(&stored_required, &required_view).required,
        first_form_field(&stored_clear, &live_true).required,
        has_property(&live_true, &Property::Required(true)),
        first_form_field(&stored_required, &live_false).required,
        has_property(&live_false, &Property::Required(false)),
    );
    assert_eq!(observations, (false, true, true, true, false, true));
}

#[test]
fn submit_required_diagnostic_uses_the_live_flag_value() {
    // Stored flag controls make sure the oracle is wired: static clear is not reported, and static
    // Required is. Live overrides then have to move that result in both directions.
    let stored_clear = pdf(0, None);
    let clear_view = ViewState::of(&stored_clear);
    let clear_owed = submit(&stored_clear, &clear_view)
        .owed
        .iter()
        .any(|sentence| sentence.contains("bit 2") && sentence.contains("Field"));

    let stored_required = pdf(2, None);
    let required_view = ViewState::of(&stored_required);
    let stored_owed = submit(&stored_required, &required_view)
        .owed
        .iter()
        .any(|sentence| sentence.contains("bit 2") && sentence.contains("Field"));

    let live_true = opened(&stored_clear, Property::Required(true));
    let live_true_owed = submit(&stored_clear, &live_true)
        .owed
        .iter()
        .any(|sentence| sentence.contains("bit 2") && sentence.contains("Field"));
    let live_false = opened(&stored_required, Property::Required(false));
    let live_false_owed = submit(&stored_required, &live_false)
        .owed
        .iter()
        .any(|sentence| sentence.contains("bit 2") && sentence.contains("Field"));
    let recorded_overrides = (
        has_property(&live_true, &Property::Required(true)),
        has_property(&live_false, &Property::Required(false)),
    );
    assert_eq!(
        (
            clear_owed,
            stored_owed,
            live_true_owed,
            live_false_owed,
            recorded_overrides
        ),
        (false, true, true, false, (true, true))
    );
}

#[test]
fn live_readonly_override_is_described_and_blocks_user_editing() {
    let editable = pdf(0, Some("Paid"));
    let editable_view = ViewState::of(&editable);
    let editable_description = first_form_field(&editable, &editable_view).read_only;
    let mut editable_control = ViewState::of(&editable);
    let accepted =
        editable_control.set_field(&editable, "Field", &Entered::Text("Before".to_owned()));

    let stored_readonly = pdf(1, Some("Paid"));
    let stored_view = ViewState::of(&stored_readonly);
    let stored_description = first_form_field(&stored_readonly, &stored_view).read_only;
    let mut stored_control = ViewState::of(&stored_readonly);
    let stored_refused = stored_control.set_field(
        &stored_readonly,
        "Field",
        &Entered::Text("Changed".to_owned()),
    );

    let live_true = opened(&editable, Property::ReadOnly(true));
    let true_description = first_form_field(&editable, &live_true).read_only;
    let mut true_control = live_true.clone();
    let live_true_refused =
        true_control.set_field(&editable, "Field", &Entered::Text("Changed".to_owned()));
    let true_value = true_control
        .field_value(&editable, "Field")
        .map(|value| value.text);

    let live_false = opened(&stored_readonly, Property::ReadOnly(false));
    let false_description = first_form_field(&stored_readonly, &live_false).read_only;
    let mut false_control = live_false.clone();
    let live_false_taken = false_control.set_field(
        &stored_readonly,
        "Field",
        &Entered::Text("Changed".to_owned()),
    );
    let false_value = false_control
        .field_value(&stored_readonly, "Field")
        .map(|value| value.text);
    let recorded_overrides = (
        has_property(&live_true, &Property::ReadOnly(true)),
        has_property(&live_false, &Property::ReadOnly(false)),
    );
    assert_eq!(
        (
            editable_description,
            accepted,
            stored_description,
            stored_refused,
            true_description,
            live_true_refused,
            true_value,
            false_description,
            live_false_taken,
            false_value,
            recorded_overrides,
        ),
        (
            false,
            1,
            true,
            0,
            true,
            0,
            Some("Paid".to_owned()),
            false,
            1,
            Some("Changed".to_owned()),
            (true, true),
        )
    );
}
