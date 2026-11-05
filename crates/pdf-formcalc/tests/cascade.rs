//! A stimulus reaches a hybrid form and its automation objects run in the order of XFA 3.3 page
//! 379, the calculations cascade by dependency (pages 379–380) and end under a limit, and what
//! they computed reaches the save as an edit beside a person's typing (ADR 2506).

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test asserts, and a failed assertion is its report"
)]

mod common;

use common::hosted::FormCalcRunner;
use pdf_model::view::{Entered, ScriptEdit, ViewState};
use pdf_model::xfa::{Activation, Change, Ended, Level, Limits, Resource, Scalar, Stimulus};
use pdf_syntax::Document;

/// A field with a text value, a text edit, and the automation elements `inside` states.
fn field(name: &str, inside: &str) -> String {
    format!("<field name=\"{name}\"><ui><textEdit/></ui><value><text/></value>{inside}</field>")
}

/// A `calculate` element running `script`, with the `override` the attribute text states.
fn calculate(script: &str, attributes: &str) -> String {
    format!("<calculate {attributes}><script>{script}</script></calculate>")
}

/// A hybrid form: one subform `form1` of `template`, a widget and a data value for every field
/// of `values`, each at `form1[0].<name>[0]`.
fn document(template: &str, values: &[(&str, &str)]) -> Document {
    use std::fmt::Write as _;
    let stream = |text: &str| {
        format!(
            "<< /Length {} >>\nstream\n{text}\nendstream",
            text.len().saturating_add(1)
        )
    };
    let template = format!(
        "<template xmlns=\"http://www.xfa.org/schema/xfa-template/3.3/\">\
         <subform name=\"form1\">{template}</subform></template>"
    );
    let mut data = String::new();
    for (name, value) in values {
        let _ = write!(data, "<{name}>{value}</{name}>");
    }
    let datasets = format!(
        "<xfa:datasets xmlns:xfa=\"http://www.xfa.org/schema/xfa-data/1.0/\"><xfa:data><form1>\
         {data}</form1></xfa:data></xfa:datasets>"
    );
    // The datasets stream is object 6 and the widgets follow it.
    let widgets: Vec<String> = (0..values.len())
        .map(|i| format!("{} 0 R", i.saturating_add(7)))
        .collect();
    let widgets = widgets.join(" ");
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] \
         /XFA [(template) 5 0 R (datasets) 6 0 R] >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [{widgets}] >>"),
        format!("<< /T (form1[0]) /Kids [{widgets}] >>"),
        stream(&template),
        stream(&datasets),
    ];
    for (index, (name, value)) in values.iter().enumerate() {
        let y = 700_usize.saturating_sub(30_usize.saturating_mul(index));
        objects.push(format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /Parent 4 0 R /T ({name}[0]) /P 3 0 R \
             /V ({value}) /Rect [72 {y} 300 {}] /DA (/Helv 10 Tf 0 g) >>",
            y.saturating_add(20)
        ));
    }
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

/// The PDF field name of `short`, a dotted path of template names from `form1`'s children down.
fn name(short: &str) -> String {
    let path: Vec<String> = short.split('.').map(|part| format!("{part}[0]")).collect();
    format!("form1[0].{}", path.join("."))
}

fn fields(values: &[(&str, &str)]) -> Vec<(String, Change)> {
    values
        .iter()
        .map(|(short, value)| (name(short), Change::Text((*value).to_owned())))
        .collect()
}

/// Activates `stimulus` over a form of `template` holding `values`.
fn activate(
    template: &str,
    values: &[(&str, &str)],
    stimulus: Stimulus<'_>,
    limits: Limits,
) -> Activation {
    let document = document(template, values);
    let resource = Resource::read(&document).unwrap().unwrap();
    resource.activate(&fields(values), stimulus, &mut FormCalcRunner, limits)
}

fn merge(template: &str, values: &[(&str, &str)]) -> Activation {
    activate(template, values, Stimulus::Merge, Limits::DEFAULT)
}

fn entered(template: &str, values: &[(&str, &str)], typed: &[&str]) -> Activation {
    let typed: Vec<String> = typed.iter().map(|short| name(short)).collect();
    activate(
        template,
        values,
        Stimulus::Entry {
            entered: &typed,
            before: &[],
        },
        Limits::DEFAULT,
    )
}

fn edit(short: &str, value: Scalar) -> (String, Scalar) {
    (name(short), value)
}

/// Page 379's cascading value changes: `Total` is first in the template and reads `Subtotal`,
/// which is calculated after it, so the calculation that is read runs first and `Total` runs
/// once, on the value it reads. Two runs find what each reads and two compute.
#[test]
fn a_calculation_runs_after_the_ones_it_reads() {
    let template = [
        field("Units", ""),
        field("Price", ""),
        field("Total", &calculate("Subtotal + 1", "")),
        field("Subtotal", &calculate("Units * Price", "")),
    ]
    .concat();
    let values = [
        ("Units", "3"),
        ("Price", "2"),
        ("Total", ""),
        ("Subtotal", ""),
    ];
    let activation = merge(&template, &values);
    assert_eq!(activation.ended, Ended::Settled);
    assert_eq!(
        activation.edits,
        vec![
            edit("Total", Scalar::Number(7.0)),
            edit("Subtotal", Scalar::Number(6.0))
        ]
    );
    assert_eq!(activation.activations, 4);
}

/// Page 379: a person's entry activates the calculations that depend directly on that
/// container's value, and the ones that depend on those. `Rate` reads nothing a person typed
/// and is not run.
#[test]
fn an_entry_runs_the_calculations_that_depend_on_it_and_no_others() {
    let template = [
        field("Units", ""),
        field("Subtotal", &calculate("Units * 2", "")),
        field("Total", &calculate("Subtotal + 1", "")),
        field("Rate", &calculate("9", "")),
    ]
    .concat();
    let values = [
        ("Units", "4"),
        ("Subtotal", "6"),
        ("Total", "7"),
        ("Rate", ""),
    ];
    let activation = entered(&template, &values, &["Units"]);
    assert_eq!(activation.ended, Ended::Settled);
    assert_eq!(
        activation.edits,
        vec![
            edit("Subtotal", Scalar::Number(8.0)),
            edit("Total", Scalar::Number(9.0))
        ],
        "Rate, which depends on nothing typed, keeps its value"
    );
    assert!(activation.report.is_empty(), "{:?}", activation.report);
}

/// Page 380: two calculations that read one another and never stop changing are the
/// seemingly infinite loops of the recommendation, and the activation ends on its limit with
/// nothing kept.
#[test]
fn a_cycle_that_never_settles_ends_on_the_activation_limit_and_keeps_nothing() {
    let template = [
        field("A", &calculate("B + 1", "")),
        field("B", &calculate("A + 1", "")),
    ]
    .concat();
    let values = [("A", ""), ("B", "")];
    for bound in [1_usize, 2, 50, 51] {
        let limits = Limits {
            activations: bound,
            ..Limits::DEFAULT
        };
        let activation = activate(&template, &values, Stimulus::Merge, limits);
        assert_eq!(activation.ended, Ended::Budget("activations"), "{bound}");
        assert_eq!(
            activation.activations, bound,
            "it stops at the bound exactly"
        );
        assert!(activation.edits.is_empty(), "nothing half-done is kept");
        assert!(
            activation
                .report
                .iter()
                .any(|s| s.contains("nothing it calculated is kept")),
            "{:?}",
            activation.report
        );
    }
    let default = merge(&template, &values);
    assert_eq!(default.ended, Ended::Budget("activations"));
    assert_eq!(default.activations, Limits::DEFAULT.activations);
}

/// The steps the scripts take between them are the other end of a cascade.
#[test]
fn a_cycle_is_also_ended_by_the_steps_the_scripts_take_between_them() {
    let template = [
        field("A", &calculate("B + 1", "")),
        field("B", &calculate("A + 1", "")),
    ]
    .concat();
    let limits = Limits {
        steps: 40,
        ..Limits::DEFAULT
    };
    let activation = activate(&template, &[("A", ""), ("B", "")], Stimulus::Merge, limits);
    assert_eq!(activation.ended, Ended::Budget("steps"));
    assert!(activation.edits.is_empty());
    assert!(activation.activations < 40, "{}", activation.activations);
}

/// A cycle whose values stop changing settles: each calculation is run again only when what it
/// reads has changed.
#[test]
fn a_cycle_whose_values_stop_changing_settles() {
    let template = [
        field("A", &calculate("B * 0 + 1", "")),
        field("B", &calculate("A * 0 + 2", "")),
    ]
    .concat();
    let activation = merge(&template, &[("A", ""), ("B", "")]);
    assert_eq!(activation.ended, Ended::Settled);
    assert_eq!(
        activation.edits,
        vec![
            edit("A", Scalar::Number(1.0)),
            edit("B", Scalar::Number(2.0))
        ]
    );
    // Two runs find what each reads; then A, B, and A again because B changed.
    assert_eq!(activation.activations, 5);
}

/// Page 380: a calculation that reads its own value is a circular reference the form's creator
/// prevents; it does not wake itself.
#[test]
fn a_calculation_does_not_wake_itself() {
    let template = field("S", &calculate("$ = Replace($, \"a\", \"b\")", ""));
    let activation = merge(&template, &[("S", "aaa")]);
    assert_eq!(
        activation.activations, 2,
        "one run to find what it reads, one to compute"
    );
    assert_eq!(
        activation.edits,
        vec![edit("S", Scalar::Text("bbb".to_owned()))]
    );
}

/// A long chain in the order that makes each link stale when it first runs.
#[test]
fn a_long_chain_in_the_wrong_order_runs_each_link_once_on_its_inputs() {
    const LINKS: usize = 100;
    let template: String = (1..=LINKS)
        .map(|i| {
            let script = if i == LINKS {
                "1".to_owned()
            } else {
                format!("C{} + 1", i.saturating_add(1))
            };
            field(&format!("C{i}"), &calculate(&script, ""))
        })
        .collect();
    let names: Vec<String> = (1..=LINKS).map(|i| format!("C{i}")).collect();
    let values: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), "")).collect();
    let activation = merge(&template, &values);
    assert_eq!(activation.ended, Ended::Settled);
    assert_eq!(
        activation.edits.first(),
        Some(&edit("C1", Scalar::Number(100.0)))
    );
    // Run in the template's order the chain takes a run per link for every link after it.
    assert_eq!(activation.activations, LINKS.saturating_mul(2));
}

/// Page 641: `disabled` switches a calculation off whatever happens; `error` and `ignore` keep
/// the calculated value over a person's entry; `warning` lets the person's stand.
#[test]
fn an_override_decides_whether_a_calculation_replaces_what_a_person_entered() {
    for (attributes, expected) in [
        ("", Some(6.0)),
        ("override=\"error\"", Some(6.0)),
        ("override=\"ignore\"", Some(6.0)),
        ("override=\"warning\"", None),
        ("override=\"disabled\"", None),
    ] {
        let template = [
            field("Units", ""),
            field("Total", &calculate("Units * 2", attributes)),
        ]
        .concat();
        let values = [("Units", "3"), ("Total", "99")];
        let activation = entered(&template, &values, &["Total"]);
        assert_eq!(
            activation.edits,
            expected
                .map(|n| edit("Total", Scalar::Number(n)))
                .into_iter()
                .collect::<Vec<_>>(),
            "{attributes:?}"
        );
    }
    let template = field("Total", &calculate("5", "override=\"disabled\""));
    assert!(merge(&template, &[("Total", "")]).edits.is_empty());
    assert_eq!(merge(&template, &[("Total", "")]).activations, 0);
}

/// Page 379 puts events first; page 391 says a `change` event is the person's own. The event
/// writes `Log` before the calculation appends to it.
#[test]
fn on_an_entry_the_change_event_runs_before_the_calculation() {
    let template = [
        field(
            "Units",
            "<event activity=\"change\"><script>Log = \"event\"</script></event>",
        ),
        field("Log", ""),
        field(
            "Total",
            &calculate("Log = Concat(Log, \",calc\"); Units", ""),
        ),
    ]
    .concat();
    let values = [("Units", "5"), ("Log", ""), ("Total", "")];
    let activation = entered(&template, &values, &["Units"]);
    let log = activation
        .edits
        .iter()
        .find(|(field, _)| *field == name("Log"));
    assert_eq!(
        log,
        Some(&edit("Log", Scalar::Text("event,calc".to_owned())))
    );
}

/// Page 407, rule 3: a merge's calculations are done, then the validations, then the initialize
/// events — so the event sees the calculated value.
#[test]
fn on_a_merge_the_initialize_event_runs_after_the_calculations() {
    let template = [
        field(
            "Units",
            "<event activity=\"initialize\"><script>Seen = Total</script></event>",
        ),
        field("Seen", ""),
        field("Total", &calculate("Units * 2", "")),
    ]
    .concat();
    let activation = merge(&template, &[("Units", "4"), ("Seen", ""), ("Total", "")]);
    assert_eq!(
        activation.edits,
        vec![
            edit("Seen", Scalar::Number(8.0)),
            edit("Total", Scalar::Number(8.0))
        ]
    );
}

/// Page 405, rule 1 item 4 and page 379: a subform has no value of its own, counts as changed
/// whenever a container inside it does, and its calculation is a property calculation, run after
/// the value calculations.
#[test]
fn a_subform_calculation_is_run_when_a_container_inside_it_changes() {
    let template = [
        "<subform name=\"Sub\">".to_owned(),
        field("A", ""),
        calculate("Seen = 1", ""),
        "</subform>".to_owned(),
        field("Seen", ""),
    ]
    .concat();
    let values = [("Sub.A", "2"), ("Seen", "")];
    let activation = entered(&template, &values, &["Sub.A"]);
    assert_eq!(activation.edits, vec![edit("Seen", Scalar::Number(1.0))]);
}

/// Steps 5's scripts are named, not run and not silently dropped.
#[test]
fn a_script_in_javascript_or_outside_the_subset_is_refused_by_name() {
    let template = [
        field(
            "Js",
            "<calculate><script contentType=\"application/x-javascript\">1;</script></calculate>",
        ),
        field(
            "Outside",
            "<event activity=\"change\"><script>Abs(1)</script></event>",
        ),
    ]
    .concat();
    let values = [("Js", ""), ("Outside", "")];
    let merged = merge(&template, &values);
    assert!(
        merged
            .report
            .iter()
            .any(|s| s.contains("Js") && s.contains("JavaScript")),
        "{:?}",
        merged.report
    );
    let typed = entered(&template, &values, &["Outside"]);
    assert!(
        typed
            .report
            .iter()
            .any(|s| s.contains("Outside") && s.contains("change event")),
        "{:?}",
        typed.report
    );
    assert!(merged.edits.is_empty() && typed.edits.is_empty());
}

/// A form of three hundred refused scripts reports the first 256 sentences and the count of the
/// rest, so a hostile template cannot make the report as large as itself.
#[test]
fn the_report_is_capped_and_says_how_many_it_left_out() {
    let template: String = (0..300)
        .map(|i| {
            field(
                &format!("Js{i}"),
                "<calculate><script contentType=\"application/x-javascript\">1;</script></calculate>",
            )
        })
        .collect();
    let merged = merge(&template, &[("Js0", "")]);
    assert_eq!(merged.report.len(), 257, "{:?}", merged.report.last());
    assert!(
        merged
            .report
            .last()
            .is_some_and(|s| s.contains("44 more sentences")),
        "{:?}",
        merged.report.last()
    );
}

/// A sentence names the field it is about, and a name as long as the template is cut where the
/// report keeps it (ADR 2507).
#[test]
fn a_reported_sentence_is_cut_to_its_cap() {
    let long = "N".repeat(5_000);
    let template = field(
        &long,
        "<calculate><script contentType=\"application/x-javascript\">1;</script></calculate>",
    );
    let merged = merge(&template, &[(long.as_str(), "")]);
    assert!(!merged.report.is_empty());
    assert!(
        merged.report.iter().all(|s| s.chars().count() <= 512),
        "{:?}",
        merged.report.iter().map(String::len).collect::<Vec<_>>()
    );
}

/// The text the scripts assign is bounded: ten calculations of a hundred bytes each store a
/// thousand bytes on the form the activation keeps and a thousand on the copy that discovery runs
/// on, and the room is charged on both (ADR 2507).
#[test]
fn the_text_the_scripts_assign_is_bounded_and_discovery_is_charged() {
    let script = format!("\"{}\"", "a".repeat(100));
    let template: String = (0..10)
        .map(|i| field(&format!("T{i}"), &calculate(&script, "")))
        .collect();
    let names: Vec<String> = (0..10).map(|i| format!("T{i}")).collect();
    let values: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), "")).collect();
    let run = |bytes| {
        let limits = Limits {
            bytes,
            ..Limits::DEFAULT
        };
        activate(&template, &values, Stimulus::Merge, limits)
    };
    let enough = run(2_000);
    assert_eq!(enough.ended, Ended::Settled);
    assert_eq!(enough.edits.len(), 10);
    // One byte short of the two thousand: the last assignment of the activation is refused.
    for bytes in [0, 500, 1_000, 1_999] {
        let short = run(bytes);
        assert_eq!(short.ended, Ended::Budget("bytes"), "{bytes}");
        assert!(short.edits.is_empty(), "{bytes}: nothing half-done is kept");
        assert!(
            short
                .report
                .iter()
                .any(|s| s.contains("bytes limit") && s.contains("nothing it calculated")),
            "{:?}",
            short.report
        );
    }
}

/// The dependencies the cascade keeps are bounded: thirty fields of one name and five
/// calculations each naming all of them is a hundred and fifty names.
#[test]
fn the_names_the_calculations_hold_are_bounded() {
    let mut template: String = (0..30).map(|_| field("F", "")).collect();
    template.push_str(
        &(1..=5)
            .map(|i| field(&format!("G{i}"), &calculate("Sum(F[*])", "")))
            .collect::<String>(),
    );
    let document = document(&template, &[("G1", "")]);
    let resource = Resource::read(&document).unwrap().unwrap();
    let all: Vec<(String, Change)> = (1..=5)
        .map(|i| (name(&format!("G{i}")), Change::Text(String::new())))
        .collect();
    let limits = Limits {
        names: 100,
        ..Limits::DEFAULT
    };
    let activation = resource.activate(&all, Stimulus::Merge, &mut FormCalcRunner, limits);
    assert_eq!(activation.ended, Ended::Budget("names"));
    assert!(activation.edits.is_empty());
    let roomy = resource.activate(&all, Stimulus::Merge, &mut FormCalcRunner, Limits::DEFAULT);
    assert_eq!(roomy.ended, Ended::Settled);
}

/// ADR 2396's save: what the calculation computed is an edit beside the person's typing, and the
/// `/V` and the `datasets` packet it reaches hold it.
#[test]
fn a_calculated_value_reaches_the_value_and_the_datasets_packet_through_the_save() {
    let template = [
        field("Units", ""),
        field("Total", &calculate("Units * 2", "")),
    ]
    .concat();
    let values = [("Units", "1"), ("Total", "2")];
    let document = document(&template, &values);
    let resource = Resource::read(&document).unwrap().unwrap();
    let mut view = ViewState::of(&document);
    assert!(view.set_field(&document, &name("Units"), &Entered::Text("3".to_owned())) > 0);
    let now = fields(&[("Units", "3"), ("Total", "2")]);
    let typed = [name("Units")];
    let activation = resource.activate(
        &now,
        Stimulus::Entry {
            entered: &typed,
            before: &[],
        },
        &mut FormCalcRunner,
        Limits::DEFAULT,
    );
    let result = activation.result();
    assert_eq!(
        result.edits,
        vec![ScriptEdit::Value {
            field: name("Total"),
            value: "6".to_owned()
        }]
    );
    for edit in &result.edits {
        let ScriptEdit::Value { field, value } = edit else {
            unreachable!("the activation makes only value edits");
        };
        assert!(view.set_field(&document, field, &Entered::Text(value.clone())) > 0);
    }
    let written = view.save(&document).expect("the form saves");
    assert!(written.xfa_unkept.is_empty(), "{:?}", written.xfa_unkept);
    let saved = Document::open(written.bytes).expect("the save opens");
    let after = Resource::read(&saved).unwrap().unwrap();
    let shown = ViewState::of(&saved).field_value(&saved, &name("Total"));
    assert_eq!(shown.map(|v| v.text).as_deref(), Some("6"), "the /V");
    assert_eq!(
        after
            .datasets
            .value(&after.template, &name("Total"))
            .as_deref(),
        Some("6")
    );
    assert_eq!(
        after
            .datasets
            .value(&after.template, &name("Units"))
            .as_deref(),
        Some("3")
    );
}

/// The per-script path of ADR 2482's hook and the activation give one form the same value.
#[test]
fn one_calculation_gives_the_same_value_alone_and_in_an_activation() {
    use common::hosted::calculate as alone;
    let template = field("Total", &calculate("6 * 7", ""));
    let document = document(&template, &[("Total", "")]);
    let resource = Resource::read(&document).unwrap().unwrap();
    let all = fields(&[("Total", "")]);
    let mut form = resource.form(&all);
    let calculation = form.calculations().remove(0);
    alone(&mut form, &calculation).unwrap();
    let activation = resource.activate(&all, Stimulus::Merge, &mut FormCalcRunner, Limits::DEFAULT);
    assert_eq!(activation.edits, form.edits());
}

/// A container whose `enter` and `exit` events each append `tag` and a sign to `Log`.
fn logged(tag: &str) -> String {
    format!(
        "<event activity=\"enter\"><script>Log = Concat(Log, \"{tag}+\")</script></event>\
         <event activity=\"exit\"><script>Log = Concat(Log, \"{tag}-\")</script></event>"
    )
}

/// Two subforms of two fields and one, every container logging its focus events, and the log.
fn focus_form() -> (String, Vec<(&'static str, &'static str)>) {
    let template = [
        format!(
            "<subform name=\"A\">{}{}{}</subform>",
            logged("A"),
            field("a1", &logged("a1")),
            field("a2", &logged("a2"))
        ),
        format!(
            "<subform name=\"B\">{}{}</subform>",
            logged("B"),
            field("b1", &logged("b1"))
        ),
        field("Log", ""),
    ]
    .concat();
    let values = vec![("A.a1", ""), ("A.a2", ""), ("B.b1", ""), ("Log", "")];
    (template, values)
}

/// The text `Log` holds after the activation, where a script set it.
fn log(activation: &Activation) -> Option<String> {
    activation
        .edits
        .iter()
        .find(|(field, _)| *field == name("Log"))
        .map(|(_, value)| match value {
            Scalar::Text(text) => text.clone(),
            other => format!("{other:?}"),
        })
}

/// Pages 390–391: focus going from `a1` to `b1` leaves the field and its subform `A` and comes into
/// the subform `B` and the field. The fields' events are page 391's; the subforms' page 390's,
/// which fire only for focus that crosses the subform's edge. The order within each half is the
/// choice ADR 2566 writes down: an `exit` innermost first, an `enter` outermost first.
#[test]
fn focus_crossing_two_subforms_exits_the_field_then_its_subform_and_enters_the_other_way() {
    let (template, values) = focus_form();
    let (left, reached) = (name("A.a1"), name("B.b1"));
    let exit = activate(
        &template,
        &values,
        Stimulus::Exit {
            left: &left,
            reaching: Some(&reached),
        },
        Limits::DEFAULT,
    );
    assert_eq!(exit.ended, Ended::Settled, "{:?}", exit.report);
    assert_eq!(log(&exit).as_deref(), Some("a1-A-"));
    let enter = activate(
        &template,
        &values,
        Stimulus::Enter {
            reached: &reached,
            leaving: Some(&left),
        },
        Limits::DEFAULT,
    );
    assert_eq!(log(&enter).as_deref(), Some("B+b1+"));
}

/// Page 390: focus moving to another field within the same subform fires neither the subform's
/// `exit` nor its `enter`, only the two fields' own (page 391).
#[test]
fn focus_within_one_subform_fires_only_the_fields_events() {
    let (template, values) = focus_form();
    let (left, reached) = (name("A.a1"), name("A.a2"));
    let exit = activate(
        &template,
        &values,
        Stimulus::Exit {
            left: &left,
            reaching: Some(&reached),
        },
        Limits::DEFAULT,
    );
    assert_eq!(log(&exit).as_deref(), Some("a1-"));
    let enter = activate(
        &template,
        &values,
        Stimulus::Enter {
            reached: &reached,
            leaving: Some(&left),
        },
        Limits::DEFAULT,
    );
    assert_eq!(log(&enter).as_deref(), Some("a2+"));
}

/// Focus that leaves the form for nothing it holds goes out of every container around the field.
#[test]
fn focus_leaving_the_form_exits_every_container_around_the_field() {
    let (template, values) = focus_form();
    let left = name("B.b1");
    let exit = activate(
        &template,
        &values,
        Stimulus::Exit {
            left: &left,
            reaching: None,
        },
        Limits::DEFAULT,
    );
    assert_eq!(log(&exit).as_deref(), Some("b1-B-"));
}

/// Page 379: a value an `enter` script sets is a change like any other, and the calculations
/// that read it run after the event.
#[test]
fn a_value_an_enter_event_sets_wakes_the_calculations_that_read_it() {
    let template = [
        field(
            "Units",
            "<event activity=\"enter\"><script>Units = 3</script></event>",
        ),
        field("Total", &calculate("Units * 2", "")),
    ]
    .concat();
    let units = name("Units");
    let activation = activate(
        &template,
        &[("Units", ""), ("Total", "")],
        Stimulus::Enter {
            reached: &units,
            leaving: None,
        },
        Limits::DEFAULT,
    );
    assert_eq!(
        activation.edits,
        vec![
            edit("Units", Scalar::Number(3.0)),
            edit("Total", Scalar::Number(6.0))
        ]
    );
}

/// A field the form's field list does not name crosses nothing, and the report says so.
#[test]
fn focus_on_a_field_the_form_does_not_name_runs_nothing_and_says_so() {
    let (template, values) = focus_form();
    let activation = activate(
        &template,
        &values,
        Stimulus::Enter {
            reached: "elsewhere[0]",
            leaving: None,
        },
        Limits::DEFAULT,
    );
    assert!(activation.edits.is_empty());
    assert!(
        activation
            .report
            .iter()
            .any(|sentence| sentence.contains("elsewhere[0]")),
        "{:?}",
        activation.report
    );
}

/// The walk out of a field is as long as the template nests, which the XML reader bounds (256
/// elements deep): a field inside a hundred subforms, each with an `exit` event, runs all hundred
/// and the field's, innermost first.
#[test]
fn focus_leaving_a_deeply_nested_field_exits_every_level_once() {
    const LEVELS: usize = 100;
    let exit = |tag: &str| {
        format!("<event activity=\"exit\"><script>Log = Concat(Log, \"{tag},\")</script></event>")
    };
    let mut template = field("f", &exit("f"));
    for level in (0..LEVELS).rev() {
        template = format!(
            "<subform name=\"S\">{}{template}</subform>",
            exit(&level.to_string())
        );
    }
    template.push_str(&field("Log", ""));
    let short = format!("{}f", "S.".repeat(LEVELS));
    let left = name(&short);
    let activation = activate(
        &template,
        &[(short.as_str(), ""), ("Log", "")],
        Stimulus::Exit {
            left: &left,
            reaching: None,
        },
        Limits::DEFAULT,
    );
    let mut expected = String::from("f,");
    for level in (0..LEVELS).rev() {
        expected.push_str(&level.to_string());
        expected.push(',');
    }
    assert_eq!(log(&activation).as_deref(), Some(expected.as_str()));
}

/// A field carrying more `exit` events than an activation may run ends on the activation limit
/// (page 380's means of ending a seemingly infinite loop) and keeps nothing its scripts set.
#[test]
fn more_exit_events_than_an_activation_runs_end_on_its_limit_and_keep_nothing() {
    let events = "<event activity=\"exit\"><script>Log = 1</script></event>"
        .repeat(Limits::DEFAULT.activations.saturating_add(1));
    let template = [field("f", &events), field("Log", "")].concat();
    let left = name("f");
    let activation = activate(
        &template,
        &[("f", ""), ("Log", "")],
        Stimulus::Exit {
            left: &left,
            reaching: None,
        },
        Limits::DEFAULT,
    );
    assert_eq!(activation.ended, Ended::Budget("activations"));
    assert!(activation.edits.is_empty(), "{:?}", activation.edits);
}

/// A container whose `click` event appends `tag` to `Log`.
fn clicks(tag: &str) -> String {
    format!("<event activity=\"click\"><script>Log = Concat(Log, \"{tag}!\")</script></event>")
}

/// Activates a click within the field `short`.
fn click(template: &str, values: &[(&str, &str)], short: &str) -> Activation {
    let clicked = name(short);
    activate(
        template,
        values,
        Stimulus::Click { clicked: &clicked },
        Limits::DEFAULT,
    )
}

/// Page 391: a click within a field fires the field's own `click` event. The subform around it
/// stating one fires nothing, because page 390 gives a subform no `click` and page 387's default
/// `listen` is `refOnly`; nor does a field beside it, which the click was not within.
#[test]
fn a_click_fires_the_fields_own_click_event_and_no_other() {
    let template = [
        format!(
            "<subform name=\"A\">{}{}{}</subform>",
            clicks("A"),
            field("Button", &clicks("Button")),
            field("Beside", &clicks("Beside"))
        ),
        field("Log", ""),
    ]
    .concat();
    let values = [("A.Button", ""), ("A.Beside", ""), ("Log", "")];
    let activation = click(&template, &values, "A.Button");
    assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
    assert_eq!(log(&activation).as_deref(), Some("Button!"));
    assert!(activation.report.is_empty(), "{:?}", activation.report);
}

/// Page 379: a value a `click` script sets is a change like any other, and the calculations that
/// read it run after the event.
#[test]
fn a_value_a_click_event_sets_wakes_the_calculations_that_read_it() {
    let template = [
        field(
            "Add",
            "<event activity=\"click\"><script>Units = Units + 1</script></event>",
        ),
        field("Units", ""),
        field("Total", &calculate("Units * 2", "")),
    ]
    .concat();
    let activation = click(
        &template,
        &[("Add", ""), ("Units", "2"), ("Total", "")],
        "Add",
    );
    assert_eq!(
        activation.edits,
        vec![
            edit("Units", Scalar::Number(3.0)),
            edit("Total", Scalar::Number(6.0))
        ]
    );
}

/// Page 703's `submit`, one of an event's one-of properties this reader does not perform, is said
/// where the click fires it rather than passing as an event that did nothing; a `script` beside it
/// on another event still runs.
#[test]
fn a_click_event_that_submits_is_reported_as_not_performed() {
    let template = [
        field(
            "Send",
            &format!(
                "<event activity=\"click\"><submit target=\"https://example.invalid/\"/></event>{}",
                clicks("Send")
            ),
        ),
        field("Log", ""),
    ]
    .concat();
    let activation = click(&template, &[("Send", ""), ("Log", "")], "Send");
    assert_eq!(log(&activation).as_deref(), Some("Send!"));
    assert!(
        activation
            .report
            .iter()
            .any(|sentence| sentence.contains("click event's submit is not performed")),
        "{:?}",
        activation.report
    );
}

/// An exclusion group's PDF field is one for every button in it, so a click on it does not say
/// which of its fields it was within: nothing runs, and the report says why.
#[test]
fn a_click_on_an_exclusion_group_runs_nothing_and_says_so() {
    let template = [
        format!(
            "<exclGroup name=\"Choice\">{}{}</exclGroup>",
            field("Yes", &clicks("Yes")),
            field("No", &clicks("No"))
        ),
        field("Log", ""),
    ]
    .concat();
    let activation = click(&template, &[("Choice", ""), ("Log", "")], "Choice");
    assert!(activation.edits.is_empty(), "{:?}", activation.edits);
    assert!(
        activation
            .report
            .iter()
            .any(|sentence| sentence.contains("it is an exclusion group")),
        "{:?}",
        activation.report
    );
}

/// A click within a field the form's field list does not name runs nothing, and says so.
#[test]
fn a_click_on_a_field_the_form_does_not_name_runs_nothing_and_says_so() {
    let template = [field("Button", &clicks("Button")), field("Log", "")].concat();
    let activation = activate(
        &template,
        &[("Button", ""), ("Log", "")],
        Stimulus::Click {
            clicked: "elsewhere[0]",
        },
        Limits::DEFAULT,
    );
    assert!(activation.edits.is_empty());
    assert!(
        activation
            .report
            .iter()
            .any(|sentence| sentence.contains("elsewhere[0]")),
        "{:?}",
        activation.report
    );
}

/// A field carrying more `click` events than an activation may run ends on the activation limit
/// and keeps nothing its scripts set.
#[test]
fn more_click_events_than_an_activation_runs_end_on_its_limit_and_keep_nothing() {
    let events = "<event activity=\"click\"><script>Log = 1</script></event>"
        .repeat(Limits::DEFAULT.activations.saturating_add(1));
    let template = [field("f", &events), field("Log", "")].concat();
    let activation = click(&template, &[("f", ""), ("Log", "")], "f");
    assert_eq!(activation.ended, Ended::Budget("activations"));
    assert!(activation.edits.is_empty(), "{:?}", activation.edits);
}

/// A field carrying a hundred thousand `click` events that each `submit` runs no script and
/// performs none of them; the report keeps one sentence for them, since they say the same thing,
/// and the walk over them is held to the template's elements, which the XML reader bounds.
#[test]
fn a_hundred_thousand_click_events_that_submit_are_reported_once() {
    let events = "<event activity=\"click\"><submit/></event>".repeat(100_000);
    let template = [field("f", &events), field("Log", "")].concat();
    let started = std::time::Instant::now();
    let activation = click(&template, &[("f", ""), ("Log", "")], "f");
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
    assert_eq!(activation.ended, Ended::Settled);
    assert_eq!(activation.activations, 0);
    assert_eq!(
        activation
            .report
            .iter()
            .filter(|sentence| sentence.contains("submit is not performed"))
            .count(),
        1,
        "{:?}",
        activation.report
    );
}

/// An event of `activity` bound by `ref` to `reference` that appends `tag` to `Log`; no `ref`
/// where `reference` is empty.
fn bound(activity: &str, reference: &str, tag: &str) -> String {
    let reference = if reference.is_empty() {
        String::new()
    } else {
        format!(" ref=\"{reference}\"")
    };
    format!(
        "<event activity=\"{activity}\"{reference}><script>Log = Concat(Log, \"{tag}!\")</script>\
         </event>"
    )
}

/// Activates the end of the open sequence.
fn ready(template: &str, values: &[(&str, &str)]) -> Activation {
    activate(template, values, Stimulus::Ready, Limits::DEFAULT)
}

/// Page 388: the Form DOM's `ready` fires once its merge is done, and the application's `docReady`
/// after it, whatever the document order of the fields carrying them. A `ready` bound to the field
/// itself names an object that generates no `ready`, which page 703 requires its `ref` to name, so
/// it fires nothing; one bound to `$layout` is not raised by this reader and is named in the report
/// as not run.
#[test]
fn ready_runs_the_form_doms_events_and_then_the_applications() {
    let template = [
        field("Doc", &bound("docReady", "$host", "doc")),
        field("Form", &bound("ready", "$form", "form")),
        field("Alias", &bound("ready", "xfa.form", "alias")),
        field("Own", &bound("ready", "", "own")),
        field("Layout", &bound("ready", "$layout", "layout")),
        field("Log", ""),
    ]
    .concat();
    let values = [
        ("Doc", ""),
        ("Form", ""),
        ("Alias", ""),
        ("Own", ""),
        ("Layout", ""),
        ("Log", ""),
    ];
    let activation = ready(&template, &values);
    assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
    assert_eq!(log(&activation).as_deref(), Some("form!alias!doc!"));
    assert!(
        activation.report.iter().any(|sentence| sentence.contains(
            "its ready event is bound to $layout, which this reader raises no event for"
        )),
        "{:?}",
        activation.report
    );
    assert_eq!(activation.report.len(), 1, "{:?}", activation.report);
}

/// No other stimulus raises `ready`: a merge runs the calculations and the `initialize` events, and
/// the Form DOM's `ready` waits for the end of the open sequence.
#[test]
fn a_merge_alone_does_not_run_ready() {
    let template = [
        field("Form", &bound("ready", "$form", "form")),
        field("Log", ""),
    ]
    .concat();
    let activation = merge(&template, &[("Form", ""), ("Log", "")]);
    assert_eq!(log(&activation), None, "{:?}", activation.edits);
}

/// Page 379: a value a `ready` script sets is a change like any other, and the calculations that
/// read it run after the event.
#[test]
fn a_value_a_ready_event_sets_wakes_the_calculations_that_read_it() {
    let template = [
        field(
            "Start",
            "<event activity=\"ready\" ref=\"$form\"><script>Units = 4</script></event>",
        ),
        field("Units", ""),
        field("Total", &calculate("Units * 2", "")),
    ]
    .concat();
    let activation = ready(&template, &[("Start", ""), ("Units", ""), ("Total", "")]);
    assert_eq!(
        activation.edits,
        vec![
            edit("Units", Scalar::Number(4.0)),
            edit("Total", Scalar::Number(8.0))
        ]
    );
}

/// An event of `activity` whose `listen` is `refAndDescendents`, appending `tag` and `^` to `Log`.
fn listens(activity: &str, tag: &str) -> String {
    format!(
        "<event activity=\"{activity}\" listen=\"refAndDescendents\"><script>Log = Concat(Log, \
         \"{tag}^\")</script></event>"
    )
}

/// Page 387: an event whose `listen` is `refAndDescendents` responds to the activity on what its
/// `ref` contains. A click within a field runs the field's own event, then that of each enclosing
/// subform that listens, innermost first (ADR 2626's choice), and then the Form DOM's listening
/// event. A subform's `click` that does not listen, one whose `listen` says `refOnly`, and a
/// listening subform beside the field rather than around it hear nothing.
#[test]
fn a_click_reaches_every_listening_container_around_the_field_innermost_first() {
    let template = [
        format!(
            "<subform name=\"Outer\">{}{}<subform name=\"Middle\">{}{}{}</subform></subform>",
            listens("click", "Outer"),
            bound("click", "", "Plain"),
            listens("click", "Middle"),
            "<event activity=\"click\" listen=\"refOnly\"><script>Log = Concat(Log, \
             \"refOnly!\")</script></event>",
            field("Button", &clicks("Button")),
        ),
        format!(
            "<subform name=\"Beside\">{}{}</subform>",
            listens("click", "Beside"),
            field("Other", "")
        ),
        format!(
            "<subform name=\"Root\"><event activity=\"click\" ref=\"$form\" \
             listen=\"refAndDescendents\"><script>Log = Concat(Log, \"Form^\")</script>\
             </event></subform>{}",
            field("Log", "")
        ),
    ]
    .concat();
    let values = [
        ("Outer.Middle.Button", ""),
        ("Beside.Other", ""),
        ("Log", ""),
    ];
    let clicked = name("Outer.Middle.Button");
    let document = document(&template, &values);
    let resource = Resource::read(&document).unwrap().unwrap();
    let activation = resource.activate(
        &fields(&values),
        Stimulus::Click { clicked: &clicked },
        &mut FormCalcRunner,
        Limits::DEFAULT,
    );
    assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
    let logged = activation
        .edits
        .iter()
        .find(|(field, _)| *field == name("Log"))
        .map(|(_, value)| format!("{value:?}"));
    assert_eq!(
        logged.as_deref(),
        Some("Text(\"Button!Middle^Outer^Form^\")"),
        "{:?}",
        activation.report
    );
    assert!(activation.report.is_empty(), "{:?}", activation.report);
}

/// Page 387 states no activity that does not propagate: a person's entry (`change`), the focus
/// leaving and reaching a field within one subform (`exit`, `enter`), a press and a release
/// (`mouseDown`, `mouseUp`) and the merge's `initialize` each reach the listening subform around
/// the field, after the field's own event. The subform's own `enter` and `exit` do not fire,
/// since the focus stays inside it (page 390), so what it logs is what it heard.
#[test]
fn each_built_activity_reaches_the_listening_subform_after_the_fields_own() {
    let activities = [
        "change",
        "exit",
        "enter",
        "mouseDown",
        "mouseUp",
        "initialize",
    ];
    let own: String = activities
        .iter()
        .map(|activity| bound(activity, "", activity))
        .collect();
    let heard: String = activities
        .iter()
        .map(|activity| listens(activity, "A"))
        .collect();
    let template = [
        format!(
            "<subform name=\"A\">{heard}{}{}</subform>",
            field("a1", &own),
            field("a2", "")
        ),
        field("Log", ""),
    ]
    .concat();
    let values = [("A.a1", ""), ("A.a2", ""), ("Log", "")];
    let (a1, a2) = (name("A.a1"), name("A.a2"));
    let entered = [a1.clone()];
    let cases = [
        (
            Stimulus::Entry {
                entered: &entered,
                before: &[],
            },
            "change!A^",
        ),
        (
            Stimulus::Exit {
                left: &a1,
                reaching: Some(&a2),
            },
            "exit!A^",
        ),
        (
            Stimulus::Enter {
                reached: &a1,
                leaving: Some(&a2),
            },
            "enter!A^",
        ),
        (Stimulus::MouseDown { pressed: &a1 }, "mouseDown!A^"),
        (Stimulus::MouseUp { released: &a1 }, "mouseUp!A^"),
    ];
    for (stimulus, expected) in cases {
        let activation = activate(&template, &values, stimulus, Limits::DEFAULT);
        assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
        assert_eq!(log(&activation).as_deref(), Some(expected), "{stimulus:?}");
    }
    // The merge fires every container's `initialize` in document order: `A`'s own, to which its
    // listening event responds as to its `ref` (page 706: the target and what it contains), then
    // `a1`'s own and what `A` heard of it, then what `A` heard of `a2`'s.
    let activation = merge(&template, &values);
    assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
    assert_eq!(log(&activation).as_deref(), Some("A^initialize!A^A^"));
}

/// Page 388 applies `ready` to the Form DOM, which no container contains: an event bound to it that
/// listens runs once, as one that does not, and a listening subform's `ready` names an object that
/// generates none and hears none from what it contains. Neither is named as not followed.
#[test]
fn a_listening_ready_runs_once_for_the_form_dom_and_never_for_a_subform() {
    let template = [
        format!(
            "<subform name=\"A\">{}<event activity=\"ready\" ref=\"$form\" \
             listen=\"refAndDescendents\"><script>Log = Concat(Log, \"Form^\")</script></event>\
             {}</subform>",
            listens("ready", "A"),
            field("a1", "")
        ),
        field("Log", ""),
    ]
    .concat();
    let activation = ready(&template, &[("A.a1", ""), ("Log", "")]);
    assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
    assert_eq!(log(&activation).as_deref(), Some("Form^"));
    assert!(activation.report.is_empty(), "{:?}", activation.report);
}

/// Pages 391–392 give `mouseDown` and `mouseUp` to a field and page 390 to no subform: a press and
/// a release run the field's own events and not a subform's that does not listen, and a name that
/// reaches an exclusion group runs nothing and says which event it did not run.
#[test]
fn a_press_and_a_release_run_the_fields_own_mouse_events() {
    let mouse = |tag: &str| [bound("mouseDown", "", tag), bound("mouseUp", "", tag)].concat();
    let template = [
        format!(
            "<subform name=\"A\">{}{}</subform>",
            mouse("A"),
            field("Button", &mouse("Button"))
        ),
        format!(
            "<exclGroup name=\"Choice\">{}</exclGroup>",
            field("Yes", &mouse("Yes"))
        ),
        field("Log", ""),
    ]
    .concat();
    let values = [("A.Button", ""), ("Choice", ""), ("Log", "")];
    let button = name("A.Button");
    for stimulus in [
        Stimulus::MouseDown { pressed: &button },
        Stimulus::MouseUp { released: &button },
    ] {
        let activation = activate(&template, &values, stimulus, Limits::DEFAULT);
        assert_eq!(log(&activation).as_deref(), Some("Button!"), "{stimulus:?}");
    }
    let choice = name("Choice");
    let activation = activate(
        &template,
        &values,
        Stimulus::MouseUp { released: &choice },
        Limits::DEFAULT,
    );
    assert!(activation.edits.is_empty(), "{:?}", activation.edits);
    assert!(
        activation.report.iter().any(|sentence| sentence
            .contains("its mouseUp events are not run: it is an exclusion group, and the pointer")),
        "{:?}",
        activation.report
    );
}

/// The deepest nesting the XML reader admits, every level listening: one click runs the field's
/// event and one event per level, each once, innermost first.
#[test]
fn a_click_at_the_deepest_nesting_reaches_every_level_once() {
    let levels = 250;
    let mut template = field("Button", &clicks("f"));
    for level in (0..levels).rev() {
        template = format!(
            "<subform name=\"L{level}\">{}{template}</subform>",
            listens("click", "")
        );
    }
    let template = [template, field("Log", "")].concat();
    let path: Vec<String> = (0..levels).map(|level| format!("L{level}")).collect();
    let short = format!("{}.Button", path.join("."));
    let values = [(short.as_str(), ""), ("Log", "")];
    let activation = click(&template, &values, &short);
    assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
    assert_eq!(activation.activations, levels + 1);
    let expected = format!("f!{}", "^".repeat(levels));
    assert_eq!(log(&activation).as_deref(), Some(expected.as_str()));
}

/// A subform holding more listening events that perform nothing than an activation may run: each
/// heard from below is charged one run, so a click ends on the activation limit, keeps nothing,
/// and the walk is held to the limit rather than to the events times the fields.
#[test]
fn more_heard_events_than_an_activation_runs_end_on_its_limit_and_keep_nothing() {
    let heard = "<event activity=\"click\" listen=\"refAndDescendents\"><submit/></event>"
        .repeat(Limits::DEFAULT.activations.saturating_add(1));
    let template = [
        format!(
            "<subform name=\"A\">{heard}{}</subform>",
            field(
                "Button",
                "<event activity=\"click\"><script>Log = 1</script></event>"
            )
        ),
        field("Log", ""),
    ]
    .concat();
    let started = std::time::Instant::now();
    let activation = click(&template, &[("A.Button", ""), ("Log", "")], "A.Button");
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
    assert_eq!(activation.ended, Ended::Budget("activations"));
    assert!(activation.edits.is_empty(), "{:?}", activation.edits);
}

/// A field carrying more `ready` events than an activation may run ends on the activation limit
/// and keeps nothing its scripts set.
#[test]
fn more_ready_events_than_an_activation_runs_end_on_its_limit_and_keep_nothing() {
    let events = "<event activity=\"ready\" ref=\"$form\"><script>Log = 1</script></event>"
        .repeat(Limits::DEFAULT.activations.saturating_add(1));
    let template = [field("f", &events), field("Log", "")].concat();
    let activation = ready(&template, &[("f", ""), ("Log", "")]);
    assert_eq!(activation.ended, Ended::Budget("activations"));
    assert!(activation.edits.is_empty(), "{:?}", activation.edits);
}

/// A hundred thousand events that listen to a subform's descendants, and a hundred thousand fields
/// each bound to an object this reader raises nothing for, run no script at the open sequence's
/// end: the listening events are followed and hear no `ready`, so nothing names them, the walk is
/// one pass over the template's elements, which the XML reader bounds, and the report is capped
/// and says how many it left out.
#[test]
fn a_hundred_thousand_unfollowed_events_are_reported_within_the_cap() {
    let listening =
        "<event activity=\"click\" listen=\"refAndDescendents\"><submit/></event>".repeat(100_000);
    let fields: String = (0..100_000)
        .map(|at| field(&format!("f{at}"), &bound("ready", "$layout", "x")))
        .collect();
    let template = [
        format!("<subform name=\"A\">{listening}</subform>"),
        fields,
        field("Log", ""),
    ]
    .concat();
    let started = std::time::Instant::now();
    let activation = ready(&template, &[("Log", "")]);
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
    assert_eq!(activation.activations, 0);
    assert_eq!(
        activation
            .report
            .iter()
            .filter(|sentence| sentence.contains("refAndDescendents"))
            .count(),
        0,
        "{:?}",
        activation.report
    );
    assert_eq!(activation.report.len(), 257, "{}", activation.report.len());
    assert!(
        activation
            .report
            .last()
            .is_some_and(|sentence| sentence.contains("more sentences, past the")),
        "{:?}",
        activation.report.last()
    );
}

/// The value the activation left in the field `short`, where a script or `$event.newText` set it.
fn edited(activation: &Activation, short: &str) -> Option<Scalar> {
    activation
        .edits
        .iter()
        .find(|(field, _)| *field == name(short))
        .map(|(_, value)| value.clone())
}

/// Activates a person's entry into the field `short`, which showed `before` before it.
fn entry(template: &str, values: &[(&str, &str)], short: &str, before: Option<&str>) -> Activation {
    let typed = [name(short)];
    let before = [before.map(str::to_owned)];
    activate(
        template,
        values,
        Stimulus::Entry {
            entered: &typed,
            before: &before,
        },
        Limits::DEFAULT,
    )
}

/// XFA 3.3 pages 398 and 404: `$event.target` is the object the activity happened on, so a
/// subform that hears a click from below (page 387) reads the field that was clicked and not
/// itself, and `$event.name` is the activity. Two clicks on two fields name each in turn.
#[test]
fn a_listening_subform_reads_the_clicked_field_from_event_target() {
    let template = [
        format!(
            "<subform name=\"A\"><event activity=\"click\" listen=\"refAndDescendents\"><script>\
             Log = Concat(Log, $event.name, \"=\", $event.target, \"!\")</script></event>{}{}\
             </subform>",
            field("One", ""),
            field("Two", "")
        ),
        field("Log", ""),
    ]
    .concat();
    let values = [("A.One", "first"), ("A.Two", "second"), ("Log", "")];
    for (short, expected) in [("A.One", "click=first!"), ("A.Two", "click=second!")] {
        let activation = click(&template, &values, short);
        assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
        assert_eq!(log(&activation).as_deref(), Some(expected));
        assert!(activation.report.is_empty(), "{:?}", activation.report);
    }
}

/// Page 399: a script may write `change`, `selStart`, `selEnd` and `cancelAction` and no other
/// property, so a click script writing `$event.name` is stopped and the sentence says why; the
/// script's later statement does not run.
#[test]
fn a_script_writing_a_read_only_event_property_is_reported() {
    let template = [
        field(
            "Button",
            "<event activity=\"click\"><script>$event.name = \"other\"\nLog = \"ran\"</script>\
             </event>",
        ),
        field("Log", ""),
    ]
    .concat();
    let values = [("Button", ""), ("Log", "")];
    let activation = click(&template, &values, "Button");
    assert_eq!(log(&activation), None, "{:?}", activation.report);
    assert!(
        activation
            .report
            .iter()
            .any(|sentence| sentence.contains("$event.name: read-only")
                && sentence.contains("page 399")),
        "{:?}",
        activation.report
    );
}

/// Page 398: using a property the current event did not set is an error. A click sets no
/// `change`, and a merge's `initialize` is no person's action, so neither script is handed a null.
#[test]
fn a_property_the_event_did_not_set_is_refused_by_name() {
    let reads_change = field(
        "Button",
        "<event activity=\"click\"><script>Log = $event.change</script></event>",
    );
    let template = [reads_change, field("Log", "")].concat();
    let values = [("Button", ""), ("Log", "")];
    let activation = click(&template, &values, "Button");
    assert_eq!(log(&activation), None);
    assert!(
        activation
            .report
            .iter()
            .any(|sentence| sentence.contains("$event.change: the click event does not set it")),
        "{:?}",
        activation.report
    );

    let reads_name = field(
        "Button",
        "<event activity=\"initialize\"><script>Log = $event.name</script></event>",
    );
    let template = [reads_name, field("Log", "")].concat();
    let activation = merge(&template, &values);
    assert_eq!(log(&activation), None);
    assert!(
        activation
            .report
            .iter()
            .any(|sentence| sentence.contains("$event.name: no person's action is being handled")),
        "{:?}",
        activation.report
    );
}

/// Page 400: a `change` script reads what the field showed (`prevText`), the entry (`change`)
/// and what the field will hold (`newText`), and a write to `change` changes `newText`, which is
/// placed into the field once the event's scripts have run; the calculation that reads the
/// field, a secondary event, sees the placed text and the primary event's `$event.name`.
#[test]
fn a_change_script_reads_the_entry_and_rewrites_it_through_event_change() {
    let template = [
        field(
            "Name",
            "<event activity=\"change\"><script>Log = Concat($event.prevText, \">\", \
             $event.newText, \"@\", $event.selStart, \"-\", $event.selEnd)\n\
             $event.change = Concat($event.change, \"!\")</script></event>",
        ),
        field("Echo", &calculate("Concat(Name, \"/\", $event.name)", "")),
        field("Log", ""),
    ]
    .concat();
    let values = [("Name", "smith"), ("Echo", ""), ("Log", "")];
    let activation = entry(&template, &values, "Name", Some("jones"));
    assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
    assert_eq!(log(&activation).as_deref(), Some("jones>smith@0-5"));
    assert_eq!(
        edited(&activation, "Name"),
        Some(Scalar::Text("smith!".to_owned())),
        "{:?} {:?}",
        activation.edits,
        activation.report
    );
    assert_eq!(
        edited(&activation, "Echo"),
        Some(Scalar::Text("smith!/change".to_owned()))
    );
    assert!(activation.report.is_empty(), "{:?}", activation.report);

    // Where the window did not say what the field showed, `prevText` is not offered.
    let reads_previous = field(
        "Name",
        "<event activity=\"change\"><script>Log = $event.prevText</script></event>",
    );
    let template = [reads_previous, field("Log", "")].concat();
    let activation = entry(&template, &[("Name", "a"), ("Log", "")], "Name", None);
    assert_eq!(log(&activation), None);
    assert!(
        activation.report.iter().any(|sentence| sentence
            .contains("$event.prevText: the window did not say what the field held")),
        "{:?}",
        activation.report
    );
}

/// Page 403: `selStart` is never past `selEnd`, and a position lies within `prevText`; a script
/// that moves the span outside it is stopped and the entry stands as the person made it.
#[test]
fn a_selection_moved_outside_the_previous_text_is_refused() {
    let template = [
        field(
            "Name",
            "<event activity=\"change\"><script>$event.selEnd = 99</script></event>",
        ),
        field("Log", ""),
    ]
    .concat();
    let values = [("Name", "abc"), ("Log", "")];
    let activation = entry(&template, &values, "Name", Some("xy"));
    assert_eq!(edited(&activation, "Name"), None, "{:?}", activation.edits);
    assert!(
        activation
            .report
            .iter()
            .any(|sentence| sentence
                .contains("$event.selEnd: a position in prevText's 2 characters")),
        "{:?}",
        activation.report
    );
}

/// Page 391: the pointer moving into a field's region and out of it fire the field's own
/// `mouseEnter` and `mouseExit`, which page 390 gives no subform; a listening subform hears both
/// (page 387) and reads the field from `$event.target`.
#[test]
fn the_pointer_crossing_a_field_runs_its_mouse_enter_and_mouse_exit() {
    let template = [
        format!(
            "<subform name=\"A\">{}{}{}</subform>",
            listens("mouseEnter", "A"),
            listens("mouseExit", "A"),
            field(
                "Button",
                "<event activity=\"mouseEnter\"><script>Log = Concat(Log, \"in:\", \
                 $event.target)</script></event><event activity=\"mouseExit\"><script>\
                 Log = Concat(Log, \"out:\", $event.target)</script></event>"
            )
        ),
        field("Log", ""),
    ]
    .concat();
    let values = [("A.Button", "b"), ("Log", "")];
    let button = name("A.Button");
    let cases = [
        (Stimulus::MouseEnter { entered: &button }, "in:bA^"),
        (Stimulus::MouseExit { exited: &button }, "out:bA^"),
    ];
    for (stimulus, expected) in cases {
        let activation = activate(&template, &values, stimulus, Limits::DEFAULT);
        assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
        assert_eq!(log(&activation).as_deref(), Some(expected));
    }
}

/// A `validate` element whose script is `script`, with the attribute text `attributes` and, where
/// `message` is given, the designer's message for the script test (XFA 3.3 page 383's example).
fn validate(script: &str, attributes: &str, message: Option<&str>) -> String {
    let message = message.map_or_else(String::new, |said| {
        format!("<message><text name=\"scriptTest\">{said}</text></message>")
    });
    format!("<validate {attributes}>{message}<script>{script}</script></validate>")
}

/// The containers each message of an activation's script tests names, with their levels.
fn invalid(activation: &Activation) -> Vec<Vec<(String, Level)>> {
    activation
        .script_tests
        .iter()
        .map(|group| {
            group
                .iter()
                .map(|failed| (failed.container.clone(), failed.level))
                .collect()
        })
        .collect()
}

/// Page 382: the script test succeeds where the script returns a true value, and a merge, the
/// form's initialization, executes every validation test. Page 881: `scriptTest` is `error` by
/// default, `warning` and `disabled` its other levels. Page 383: the designer's message for the
/// test is the `message` element's `text` named `scriptTest`. Page 386's default
/// `validationMessaging`, `allMessagesIndividually`, makes each failure its own message.
#[test]
fn a_script_test_fails_where_its_script_returns_no_true_value_at_its_level() {
    for (attributes, value, expected) in [
        ("", "150", Some(Level::Error)),
        ("scriptTest=\"error\"", "150", Some(Level::Error)),
        ("scriptTest=\"warning\"", "150", Some(Level::Warning)),
        ("scriptTest=\"disabled\"", "150", None),
        ("", "50", None),
    ] {
        let template = field(
            "Amount",
            &validate("Amount &lt;= 100", attributes, Some("At most 100.")),
        );
        let activation = merge(&template, &[("Amount", value)]);
        assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
        assert_eq!(
            invalid(&activation),
            expected
                .map(|level| vec![(name("Amount"), level)])
                .into_iter()
                .collect::<Vec<_>>(),
            "{attributes:?} holding {value}"
        );
        if expected.is_some() {
            assert_eq!(
                activation.script_tests[0][0].message.as_deref(),
                Some("At most 100.")
            );
        }
    }
}

/// Page 1061: a value is true where it can be fully converted to a nonzero value, and null is
/// false; so a script answering a text that is no number, or zero, fails the test, and one
/// answering a nonzero number passes it.
#[test]
fn a_script_test_reads_its_value_as_formcalc_promotes_a_boolean() {
    for (script, fails) in [
        ("1", false),
        ("-2.5", false),
        ("\"7\"", false),
        ("0", true),
        ("\"abc\"", true),
        ("Null", true),
    ] {
        let template = field("Amount", &validate(script, "", None));
        let activation = merge(&template, &[("Amount", "1")]);
        assert_eq!(!activation.script_tests.is_empty(), fails, "{script}");
        if fails {
            assert_eq!(activation.script_tests[0][0].message, None);
        }
    }
}

/// Page 396: neither the script test nor the format test is applied to a field holding null or
/// empty content, which the null test answers for.
#[test]
fn an_empty_field_is_not_put_to_its_script_test() {
    let template = field("Amount", &validate("0", "", None));
    assert!(merge(&template, &[("Amount", "")]).script_tests.is_empty());
    assert_eq!(invalid(&merge(&template, &[("Amount", "x")])).len(), 1);
}

/// Page 382: on a person's entry the tests run are the field's, a subform's where a value was
/// entered into a field it contains (`Inner`), and the others the processor infers the change may
/// affect by dependency tracking — here `Check`, whose script reads `A`. `B`'s test and the subform
/// `Other`'s, which the entry reaches neither way, are not answered.
#[test]
fn an_entry_answers_the_script_tests_it_reaches_and_no_others() {
    let template = [
        format!(
            "<subform name=\"Inner\">{}{}</subform>",
            validate("0", "", None),
            field("A", &validate("A &lt; 10", "", None))
        ),
        format!(
            "<subform name=\"Other\">{}{}</subform>",
            validate("0", "", None),
            field("B", &validate("B &lt; 10", "", None))
        ),
        field("Check", &validate("Inner.A &lt; 10", "", None)),
    ]
    .concat();
    let values = [("Inner.A", "50"), ("Other.B", "50"), ("Check", "1")];
    let activation = entered(&template, &values, &["Inner.A"]);
    let mut answered: Vec<String> = invalid(&activation)
        .into_iter()
        .flatten()
        .map(|(container, _)| container)
        .collect();
    answered.sort();
    assert_eq!(
        answered,
        ["Inner".to_owned(), name("Check"), name("Inner.A")],
        "{:?}",
        activation.report
    );
    assert_eq!(invalid(&merge(&template, &values)).len(), 5);
}

/// Pages 372 and 385: a validation script must not alter the form. One that assigns has its
/// assignment put back before anything reads it — `Probe`'s test, run after it, reads `Note` as
/// the calculation left it — the activation keeps no such value, and the report says so.
#[test]
fn a_validation_script_that_assigns_changes_nothing_and_is_reported() {
    let template = [
        field("Amount", &validate("Note = \"touched\"\n1", "", None)),
        field("Note", &calculate("Concat(\"seen \", Note)", "")),
        field("Probe", &validate("Note &lt;&gt; \"touched\"", "", None)),
    ]
    .concat();
    let values = [("Amount", "5"), ("Note", ""), ("Probe", "1")];
    let activation = merge(&template, &values);
    assert_eq!(
        activation.edits,
        vec![edit("Note", Scalar::Text("seen ".to_owned()))]
    );
    assert!(activation.script_tests.is_empty());
    assert!(
        activation
            .report
            .iter()
            .any(|said| said.contains("validation script assigned a value")),
        "{:?}",
        activation.report
    );
}

/// Page 407's rule 3: on a merge the validations run after the calculations and before the
/// `initialize` events. The calculation makes `Amount` valid before its test; the `initialize`
/// event makes `Late` valid only after its test, so `Late` alone fails.
#[test]
fn on_a_merge_the_script_tests_run_between_the_calculations_and_the_initialize_events() {
    let template = [
        field(
            "Amount",
            &[calculate("5", ""), validate("Amount &lt; 10", "", None)].concat(),
        ),
        field(
            "Late",
            &[
                "<event activity=\"initialize\"><script>Late = 5</script></event>".to_owned(),
                validate("Late &lt; 10", "", None),
            ]
            .concat(),
        ),
    ]
    .concat();
    let activation = merge(&template, &[("Amount", "50"), ("Late", "50")]);
    assert_eq!(
        invalid(&activation),
        vec![vec![(name("Late"), Level::Error)]],
        "{:?}",
        activation.report
    );
}

/// Page 642: a person who dismisses the warning an `override` of `warning` presents leaves the
/// calculated value undisturbed, so the calculation replaces the entry (`99`) with its value
/// (`Units * 2`), and `Double`, which reads `Total`, follows (page 379's cascade). No event fires:
/// `Total`'s `change` is a person's entry (page 391), and a dismissal enters nothing.
#[test]
fn a_dismissed_warning_runs_the_calculation_over_the_entry_and_what_reads_it() {
    let template = [
        field("Units", ""),
        field(
            "Total",
            &[
                calculate("Units * 2", "override=\"warning\""),
                "<event activity=\"change\"><script>Log = \"fired\"</script></event>".to_owned(),
            ]
            .concat(),
        ),
        field("Double", &calculate("Total * 2", "")),
        field("Log", ""),
    ]
    .concat();
    let values = [
        ("Units", "3"),
        ("Total", "99"),
        ("Double", "198"),
        ("Log", ""),
    ];
    let field = name("Total");
    let activation = activate(
        &template,
        &values,
        Stimulus::Dismissed { field: &field },
        Limits::DEFAULT,
    );
    assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
    assert_eq!(
        activation.edits,
        vec![
            edit("Total", Scalar::Number(6.0)),
            edit("Double", Scalar::Number(12.0)),
        ]
    );
    assert!(activation.overridden.is_empty());
}
