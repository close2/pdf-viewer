//! A hybrid form's `calculate` scripts run against the form they belong to, through
//! `pdf-model`'s hook and the adapter in `common/hosted.rs` (ADR 2482; XFA 3.3 pages 114, 115
//! and 379).

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test asserts, and a failed assertion is its report"
)]

mod common;

use common::hosted::calculate;
use pdf_formcalc::{Stop, Value};
use pdf_model::xfa::{Change, Language, Resource, Scalar};
use pdf_syntax::Document;

/// A document whose `/XFA` array holds a template and an empty `datasets` packet.
fn document(template: &str) -> Document {
    use std::fmt::Write as _;
    let stream = |text: &str| {
        format!(
            "<< /Length {} >>\nstream\n{text}\nendstream",
            text.len().saturating_add(1)
        )
    };
    let template = format!(
        "<template xmlns=\"http://www.xfa.org/schema/xfa-template/3.3/\">{template}</template>"
    );
    let datasets = "<xfa:datasets xmlns:xfa=\"http://www.xfa.org/schema/xfa-data/1.0/\">\
                    <xfa:data><form1/></xfa:data></xfa:datasets>";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [] \
         /XFA [(template) 4 0 R (datasets) 5 0 R] >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>".to_owned(),
        stream(&template),
        stream(datasets),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let _ = write!(
        out,
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len().saturating_add(1)
    );
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len().saturating_add(1)
    );
    Document::open(out.into_bytes()).expect("the fixture opens")
}

/// What running one calculation gave.
type Outcome = Result<Value, Stop>;

/// The values of the fields of the form, by PDF name, after every `FormCalc` calculation ran once
/// in document order; each calculation's outcome beside it.
fn run_all(template: &str, fields: &[(&str, &str)]) -> (Vec<(String, Scalar)>, Vec<Outcome>) {
    let document = document(template);
    let resource = Resource::read(&document).unwrap().unwrap();
    let fields: Vec<(String, Change)> = fields
        .iter()
        .map(|(name, value)| ((*name).to_owned(), Change::Text((*value).to_owned())))
        .collect();
    let mut form = resource.form(&fields);
    let mut outcomes = Vec::new();
    for calculation in form.calculations() {
        if calculation.language == Language::FormCalc {
            outcomes.push(calculate(&mut form, &calculation));
        }
    }
    (form.edits(), outcomes)
}

#[test]
fn a_total_is_computed_from_its_siblings() {
    // Page 114: Units and Price are siblings of Total, so the unqualified names reach them.
    let (edits, outcomes) = run_all(
        "<subform name=\"form1\">\
         <field name=\"Units\"><value><integer/></value></field>\
         <field name=\"Price\"><value><decimal/></value></field>\
         <field name=\"Total\"><value><decimal/></value>\
         <calculate><script>Units * Price</script></calculate></field>\
         </subform>",
        &[
            ("form1[0].Units[0]", "4"),
            ("form1[0].Price[0]", "2.5"),
            ("form1[0].Total[0]", ""),
        ],
    );
    assert_eq!(outcomes, vec![Ok(Value::Number(10.0))]);
    assert_eq!(
        edits,
        vec![("form1[0].Total[0]".to_owned(), Scalar::Number(10.0))]
    );
}

#[test]
fn a_dollar_assignment_rewrites_the_fields_own_value() {
    // The state-name script of the census: `$ = Replace($, "AL", "Alabama")`, page 1132's
    // Replace over the field's own value, page 1066's assignment.
    let (edits, outcomes) = run_all(
        "<subform name=\"form1\"><field name=\"State\"><calculate>\
         <script>$ = Replace($, \"AL\", \"Alabama\");\n$ = Replace($, \"AK\", \"Alaska\");</script>\
         </calculate></field></subform>",
        &[("form1[0].State[0]", "AK")],
    );
    assert_eq!(outcomes, vec![Ok(Value::String("Alaska".to_owned()))]);
    assert_eq!(
        edits,
        vec![(
            "form1[0].State[0]".to_owned(),
            Scalar::Text("Alaska".to_owned())
        )]
    );
}

#[test]
fn a_dotted_name_is_reached_through_the_nearest_ancestor_that_holds_its_first_term() {
    // Page 114: `Page1.Closing` starts at Page1, a sibling of the subform the script is in.
    let (edits, outcomes) = run_all(
        "<subform name=\"form1\"><subform name=\"Page1\">\
         <field name=\"Closing\"><value><decimal/></value></field></subform>\
         <subform name=\"Page2\"><field name=\"Value\"><value><decimal/></value>\
         <calculate><script>Sum(Page1.Closing * 3)</script></calculate></field></subform>\
         </subform>",
        &[
            ("form1[0].Page1[0].Closing[0]", "7"),
            ("form1[0].Page2[0].Value[0]", ""),
        ],
    );
    assert_eq!(outcomes, vec![Ok(Value::Number(21.0))]);
    assert_eq!(
        edits,
        vec![(
            "form1[0].Page2[0].Value[0]".to_owned(),
            Scalar::Number(21.0)
        )]
    );
}

#[test]
fn a_name_the_form_does_not_hold_is_an_error_not_a_zero() {
    // Page 1062 lists an unknown accessor among the error exceptions.
    let (edits, outcomes) = run_all(
        "<subform name=\"form1\"><field name=\"Total\"><calculate>\
         <script>Missing + 1</script></calculate></field></subform>",
        &[("form1[0].Total[0]", "")],
    );
    assert!(
        matches!(outcomes.as_slice(), [Err(Stop::Exception(_))]),
        "{outcomes:?}"
    );
    assert!(edits.is_empty());
}

#[test]
fn a_construct_outside_the_subset_is_refused_and_assigns_nothing() {
    let (edits, outcomes) = run_all(
        "<subform name=\"form1\"><field name=\"Total\"><calculate>\
         <script>Abs(0 - 3)</script></calculate></field></subform>",
        &[("form1[0].Total[0]", "")],
    );
    assert!(
        matches!(outcomes.as_slice(), [Err(Stop::Unsupported(_))]),
        "{outcomes:?}"
    );
    assert!(edits.is_empty());
}

#[test]
fn the_activation_of_a_merge_agrees_with_running_each_script_alone() {
    // ADR 2506: the activation runs the scripts of ADR 2482's hook in an order of its own, and a
    // form whose calculations do not read one another gets the values each gives run alone.
    use common::hosted::FormCalcRunner;
    use pdf_model::xfa::{Limits, Stimulus};
    let template = "<subform name=\"form1\">\
         <field name=\"Units\"><value><integer/></value></field>\
         <field name=\"Price\"><value><decimal/></value></field>\
         <field name=\"Total\"><value><decimal/></value>\
         <calculate><script>Units * Price</script></calculate></field>\
         </subform>";
    let values = [
        ("form1[0].Units[0]", "4"),
        ("form1[0].Price[0]", "2.5"),
        ("form1[0].Total[0]", ""),
    ];
    let (alone, _) = run_all(template, &values);
    let document = document(template);
    let resource = Resource::read(&document).unwrap().unwrap();
    let fields: Vec<(String, Change)> = values
        .iter()
        .map(|(name, value)| ((*name).to_owned(), Change::Text((*value).to_owned())))
        .collect();
    let activation = resource.activate(
        &fields,
        Stimulus::Merge,
        &mut FormCalcRunner,
        Limits::DEFAULT,
    );
    assert_eq!(activation.edits, alone);
}
