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
use pdf_model::view::CommitKey;
use pdf_model::view::{Entered, ScriptEdit, ViewState};
use pdf_model::xfa::{
    Activation, Change, Ended, Exited, Level, Limits, MAX_SUBMITS, Resource, Scalar, Stimulus,
    Test, Typing, Validity,
};
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
    configured(template, values, None)
}

/// The `template` element's start tag of an XFA 3.3 template.
const HEAD: &str = "<template xmlns=\"http://www.xfa.org/schema/xfa-template/3.3/\">";

/// [`document`], with a `config` packet of the text `config` where it is given, the last object.
fn configured(template: &str, values: &[(&str, &str)], config: Option<&str>) -> Document {
    headed(HEAD, template, values, config)
}

/// [`configured`], with the `template` element's start tag and what follows it before the root
/// subform `head`.
fn headed(head: &str, template: &str, values: &[(&str, &str)], config: Option<&str>) -> Document {
    use std::fmt::Write as _;
    let stream = |text: &str| {
        format!(
            "<< /Length {} >>\nstream\n{text}\nendstream",
            text.len().saturating_add(1)
        )
    };
    let template = format!("{head}<subform name=\"form1\">{template}</subform></template>");
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
    let config_packet = config.map_or_else(String::new, |_| {
        format!(" (config) {} 0 R", values.len().saturating_add(7))
    });
    let mut objects = vec![
        format!(
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] \
             /XFA [(template) 5 0 R (datasets) 6 0 R{config_packet}] >> >>"
        ),
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
    if let Some(config) = config {
        objects.push(stream(&format!(
            "<config xmlns=\"http://www.xfa.org/schema/xci/3.1/\">{config}</config>"
        )));
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
            typing: &[],
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
            typing: &[],
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
            left_button: None,
            reaching_button: None,
            exited_at_selection: false,
            committed: None,
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
            reached_button: None,
            leaving_button: None,
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
            left_button: None,
            reaching_button: None,
            exited_at_selection: false,
            committed: None,
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
            reached_button: None,
            leaving_button: None,
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
            left_button: None,
            reaching_button: None,
            exited_at_selection: false,
            committed: None,
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
            reached_button: None,
            leaving_button: None,
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
            reached_button: None,
            leaving_button: None,
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
            left_button: None,
            reaching_button: None,
            exited_at_selection: false,
            committed: None,
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
            left_button: None,
            reaching_button: None,
            exited_at_selection: false,
            committed: None,
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
        Stimulus::Click {
            clicked: &clicked,
            button: None,
        },
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

/// A field whose `click` event holds `property` in place of a script, beside a second `click`
/// event appending `Send` to `Log`.
fn sends(property: &str) -> String {
    field(
        "Send",
        &format!(
            "<event activity=\"click\">{property}</event>{}",
            clicks("Send")
        ),
    )
}

/// Clicks `Send` in a form of [`sends`]`(property)` and a `Log`.
fn click_send(property: &str) -> Activation {
    let template = [sends(property), field("Log", "")].concat();
    click(&template, &[("Send", ""), ("Log", "")], "Send")
}

/// Whether the activation's report holds a sentence containing `words`.
fn says(activation: &Activation, words: &str) -> bool {
    activation
        .report
        .iter()
        .any(|sentence| sentence.contains(words))
}

/// Table 240's bits, numbered from 1 as the table numbers them.
fn bits(numbers: &[u32]) -> u32 {
    numbers.iter().fold(0, |all, bit| {
        all | 1u32.checked_shl(bit.saturating_sub(1)).unwrap_or(0)
    })
}

/// Page 703's `submit` is held for the host as the submit-form action composing the body page 859's
/// `format` asks for: `pdf` is Table 240 bit 9's whole document, `xfd` bit 6's XFDF and
/// `urlencoded` bit 3's HTML Form format; each by POST (page 858), so bit 4 is clear, and each with
/// bit 2, since page 445 sends the form's variable data, an empty field's included. `/F` is the
/// `target`, and the field clicked is what the `preSubmit` events read (page 398). A `script` on
/// another event of the same click still runs.
#[test]
fn a_click_event_that_submits_holds_the_submit_for_the_host_in_the_format_it_names() {
    for (format, bit) in [("pdf", 9), ("xfd", 6), ("urlencoded", 3)] {
        let activation = click_send(&format!(
            "<submit format=\"{format}\" target=\"https://example.invalid/form\"/>"
        ));
        assert_eq!(log(&activation).as_deref(), Some("Send!"), "{format}");
        assert!(
            activation.report.is_empty(),
            "{format}: {:?}",
            activation.report
        );
        let [submit] = activation.submits.as_slice() else {
            panic!("{format}: one submit is held: {:?}", activation.submits);
        };
        assert_eq!(submit.action.url, "https://example.invalid/form");
        assert_eq!(submit.action.flags.0, bits(&[bit, 2]), "{format}");
        assert!(submit.action.fields.is_empty());
        assert_eq!(submit.clicked, Some((name("Send"), None)));
        assert!(submit.owed.is_empty(), "{format}: {:?}", submit.owed);
    }
}

/// Page 597 makes the underlined value the one an omitted attribute takes, and page 858 underlines
/// `xdp`: a `submit` stating no `format` asks for chapter 23's XDP packaging, which this reader does
/// not write, as it does not write `xml`'s; page 859 describes `formdata` only as emulating legacy
/// software it does not name. Each is said, and nothing is held.
#[test]
fn a_submit_in_a_packaging_this_reader_does_not_write_is_said_and_not_held() {
    for (attribute, words) in [
        ("", "the xdp packaging"),
        ("format=\"xml\"", "the xml packaging"),
        ("format=\"formdata\"", "formdata packaging"),
        (
            "format=\"html\"",
            "a format html that page 858 does not list",
        ),
    ] {
        let activation = click_send(&format!(
            "<submit {attribute} target=\"https://example.invalid/\"/>"
        ));
        assert!(activation.submits.is_empty(), "{attribute}");
        assert!(
            says(&activation, words),
            "{attribute}: {:?}",
            activation.report
        );
        assert_eq!(log(&activation).as_deref(), Some("Send!"));
    }
}

/// Page 453's client signs the content (step 3) and encrypts it (step 4) before posting it, and
/// this reader does neither, so a `submit` holding `signData`, `encrypt` or `encryptData` is said
/// and not held rather than sent unsigned or in the clear; one with no `target` has nowhere to go,
/// since page 860 leaves the URI to a product's own technique.
#[test]
fn a_submit_that_asks_to_be_signed_or_encrypted_or_names_no_target_is_not_held() {
    for (property, words) in [
        (
            "<submit format=\"pdf\" target=\"https://example.invalid/\"><signData operation=\"sign\"/></submit>",
            "holds a signData",
        ),
        (
            "<submit format=\"pdf\" target=\"https://example.invalid/\"><encrypt/></submit>",
            "holds an encrypt or an encryptData",
        ),
        (
            "<submit format=\"xfd\" target=\"https://example.invalid/\"><encryptData/></submit>",
            "holds an encrypt or an encryptData",
        ),
        ("<submit format=\"pdf\"/>", "states no target"),
    ] {
        let activation = click_send(property);
        assert!(activation.submits.is_empty(), "{property}");
        assert!(
            says(&activation, words),
            "{property}: {:?}",
            activation.report
        );
    }
}

/// Page 860's `textEncoding` decides the encoding of the text of a URL-encoded body, and the body is
/// written in UTF-8: another encoding is named on what the submit owes, and the submit is held.
#[test]
fn a_text_encoding_other_than_utf_8_is_named_on_what_the_submit_owes() {
    let activation = click_send(
        "<submit format=\"urlencoded\" textEncoding=\"ISO-8859-1\" \
         target=\"https://example.invalid/\"/>",
    );
    let [submit] = activation.submits.as_slice() else {
        panic!("one submit is held: {:?}", activation.report);
    };
    assert!(
        submit.owed.iter().any(|owed| owed.contains("ISO-8859-1")),
        "{:?}",
        submit.owed
    );
    let utf8 = click_send(
        "<submit format=\"urlencoded\" textEncoding=\"utf-8\" target=\"https://example.invalid/\"/>",
    );
    assert!(
        utf8.submits
            .first()
            .is_some_and(|submit| submit.owed.is_empty())
    );
}

/// Page 722: an `execute` whose `runAt` is `server` is one a client ignores, so it runs nothing and
/// says nothing; one the client is to run calls a WSDL web service, which this reader does not,
/// and says so; and one naming no connection is the error page 722 names.
#[test]
fn an_execute_is_ignored_where_the_server_runs_it_and_said_where_the_client_would() {
    let server = click_send("<execute connection=\"Lookup\" runAt=\"server\"/>");
    assert!(server.report.is_empty(), "{:?}", server.report);
    assert!(server.submits.is_empty());
    let client = click_send("<execute connection=\"Lookup\"/>");
    assert!(
        says(&client, "web service of the connection Lookup"),
        "{:?}",
        client.report
    );
    let unnamed = click_send("<execute runAt=\"both\"/>");
    assert!(
        says(&unnamed, "names no connection"),
        "{:?}",
        unnamed.report
    );
}

/// Page 446 pairs each pre-event with its post-event around one transaction: a `submit` a `preSubmit`
/// or `postSubmit` event performs would begin a submit inside the one under way, so it is said and
/// not held — a choice where the pages are silent (ADR 2738).
#[test]
fn a_submit_under_a_submits_own_events_is_said_and_not_held() {
    let submit = "<submit format=\"pdf\" target=\"https://example.invalid/\"/>";
    let template = [
        field(
            "Send",
            &format!(
                "<event activity=\"preSubmit\" ref=\"$form\">{submit}</event>\
                 <event activity=\"postSubmit\" ref=\"$form\">{submit}</event>"
            ),
        ),
        field("Log", ""),
    ]
    .concat();
    for stimulus in [
        Stimulus::PreSubmit {
            clicked: None,
            button: None,
        },
        Stimulus::PostSubmit {
            clicked: None,
            button: None,
        },
    ] {
        let activation = activate(
            &template,
            &[("Send", ""), ("Log", "")],
            stimulus,
            Limits::DEFAULT,
        );
        assert!(activation.submits.is_empty(), "{stimulus:?}");
        assert!(
            says(&activation, "would begin a submit inside the one under way"),
            "{:?}",
            activation.report
        );
    }
}

/// One activation hands the host at most `MAX_SUBMITS`, and says that the rest are not performed.
#[test]
fn an_activation_holds_at_most_its_bound_of_submits() {
    let events = "<event activity=\"click\"><submit format=\"pdf\" target=\"https://example.invalid/\"/></event>"
        .repeat(MAX_SUBMITS.saturating_add(3));
    let template = [field("f", &events), field("Log", "")].concat();
    let activation = click(&template, &[("f", ""), ("Log", "")], "f");
    assert_eq!(activation.submits.len(), MAX_SUBMITS);
    assert!(
        says(&activation, "the most it sends"),
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
            button: None,
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

/// A field carrying a hundred thousand `click` events that each `submit` in the default `xdp`
/// packaging runs no script and holds none of them; the report keeps one sentence for them, since they say the same
/// thing, and the walk over them is held to the template's elements, which the XML reader bounds.
#[test]
fn a_hundred_thousand_click_events_that_submit_are_reported_once() {
    let events = "<event activity=\"click\"><submit/></event>".repeat(100_000);
    let template = [field("f", &events), field("Log", "")].concat();
    let started = std::time::Instant::now();
    let activation = click(&template, &[("f", ""), ("Log", "")], "f");
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
    assert_eq!(activation.ended, Ended::Settled);
    assert_eq!(activation.activations, 0);
    assert!(activation.submits.is_empty());
    assert_eq!(
        activation
            .report
            .iter()
            .filter(|sentence| sentence.contains("submit asks for the xdp packaging"))
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
        Stimulus::Click {
            clicked: &clicked,
            button: None,
        },
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
                typing: &[],
            },
            "change!A^",
        ),
        (
            Stimulus::Exit {
                left: &a1,
                reaching: Some(&a2),
                left_button: None,
                reaching_button: None,
                exited_at_selection: false,
                committed: None,
            },
            "exit!A^",
        ),
        (
            Stimulus::Enter {
                reached: &a1,
                leaving: Some(&a2),
                reached_button: None,
                leaving_button: None,
            },
            "enter!A^",
        ),
        (
            Stimulus::MouseDown {
                pressed: &a1,
                button: None,
            },
            "mouseDown!A^",
        ),
        (
            Stimulus::MouseUp {
                released: &a1,
                button: None,
            },
            "mouseUp!A^",
        ),
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
        Stimulus::MouseDown {
            pressed: &button,
            button: None,
        },
        Stimulus::MouseUp {
            released: &button,
            button: None,
        },
    ] {
        let activation = activate(&template, &values, stimulus, Limits::DEFAULT);
        assert_eq!(log(&activation).as_deref(), Some("Button!"), "{stimulus:?}");
    }
    let choice = name("Choice");
    let activation = activate(
        &template,
        &values,
        Stimulus::MouseUp {
            released: &choice,
            button: None,
        },
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
            typing: &[],
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
        (
            Stimulus::MouseEnter {
                entered: &button,
                button: None,
            },
            "in:bA^",
        ),
        (
            Stimulus::MouseExit {
                exited: &button,
                button: None,
            },
            "out:bA^",
        ),
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

/// A radio button of an exclusion group: a check button whose on value, the first of its `items`,
/// is `on` (page 759's `checkButton`, pages 195–197), carrying `inside`.
fn radio(name: &str, on: &str, inside: &str) -> String {
    format!(
        "<field name=\"{name}\"><ui><checkButton/></ui><items><text>{on}</text></items>{inside}\
         </field>"
    )
}

/// An exclusion group `Choice` of the buttons `Agree` (on value `y`) and `Refuse` (on value `n`),
/// each with a `click` event appending its name, and a group event that hears a click from below
/// (page 387).
fn choice_group() -> (String, Vec<(&'static str, &'static str)>) {
    let template = [
        format!(
            "<exclGroup name=\"Choice\"><event activity=\"click\" listen=\"refAndDescendents\">\
             <script>Log = Concat(Log, \"Group^\")</script></event>{}{}</exclGroup>",
            radio("Agree", "y", &clicks("Agree")),
            radio("Refuse", "n", &clicks("Refuse"))
        ),
        field("Log", ""),
    ]
    .concat();
    (template, vec![("Choice", ""), ("Log", "")])
}

/// Page 391: a click fires the `click` events of the field it occurs within. An exclusion group's
/// PDF field is one for all its buttons, and the widget clicked names the button by its on value,
/// which Annex K's consistency of the XFA values with the `/V` entries makes the first `items`
/// value of the group's field (pages 195–197): the on value `n` is `Refuse`'s, so `Refuse`'s
/// event runs, and not `Agree`'s, and the group, listening, hears it after (page 387).
#[test]
fn a_click_on_a_button_of_an_exclusion_group_runs_the_click_events_of_that_field() {
    let (template, values) = choice_group();
    let group = name("Choice");
    for (on, expected) in [("n", "Refuse!Group^"), ("y", "Agree!Group^")] {
        let activation = activate(
            &template,
            &values,
            Stimulus::Click {
                clicked: &group,
                button: Some(on),
            },
            Limits::DEFAULT,
        );
        assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
        assert_eq!(log(&activation).as_deref(), Some(expected), "{on}");
        assert!(activation.report.is_empty(), "{:?}", activation.report);
    }
}

/// Pages 391–392: the press, the release and the pointer crossing are a field's events as the click
/// is, so a button of an exclusion group runs its own `mouseDown`, `mouseUp`, `mouseEnter` and
/// `mouseExit` by the on value the widget names.
#[test]
fn the_pointer_on_a_button_of_an_exclusion_group_runs_that_fields_mouse_events() {
    let mouse = |activity: &str| {
        format!(
            "<event activity=\"{activity}\"><script>Log = Concat(Log, \"{activity}!\")</script>\
             </event>"
        )
    };
    let events = ["mouseDown", "mouseUp", "mouseEnter", "mouseExit"]
        .map(mouse)
        .concat();
    let template = [
        format!(
            "<exclGroup name=\"Choice\">{}{}</exclGroup>",
            radio("Agree", "y", ""),
            radio("Refuse", "n", &events)
        ),
        field("Log", ""),
    ]
    .concat();
    let values = [("Choice", ""), ("Log", "")];
    let group = name("Choice");
    let on = Some("n");
    for (stimulus, expected) in [
        (
            Stimulus::MouseDown {
                pressed: &group,
                button: on,
            },
            "mouseDown!",
        ),
        (
            Stimulus::MouseUp {
                released: &group,
                button: on,
            },
            "mouseUp!",
        ),
        (
            Stimulus::MouseEnter {
                entered: &group,
                button: on,
            },
            "mouseEnter!",
        ),
        (
            Stimulus::MouseExit {
                exited: &group,
                button: on,
            },
            "mouseExit!",
        ),
    ] {
        let activation = activate(&template, &values, stimulus, Limits::DEFAULT);
        assert_eq!(log(&activation).as_deref(), Some(expected), "{stimulus:?}");
    }
}

/// A widget whose on value no button of the group holds names no field the click was within, so
/// nothing runs and the report says which value it was.
#[test]
fn a_click_on_a_button_whose_on_value_no_field_holds_runs_nothing_and_says_so() {
    let (template, values) = choice_group();
    let group = name("Choice");
    let activation = activate(
        &template,
        &values,
        Stimulus::Click {
            clicked: &group,
            button: Some("maybe"),
        },
        Limits::DEFAULT,
    );
    assert!(activation.edits.is_empty(), "{:?}", activation.edits);
    assert!(
        activation
            .report
            .iter()
            .any(|sentence| sentence.contains("none of its fields has the on value \"maybe\"")),
        "{:?}",
        activation.report
    );
}

/// Pages 706–707: an event's `ref` names the object that generates the event, a "come-from" (page
/// 404), so an event placed in `Watcher` bound to `Button` runs when `Button` is clicked, as
/// `Watcher`'s script — `$` is `Watcher` — with `$event.target` the button (page 404). A click
/// on `Watcher` itself runs nothing, its event being bound elsewhere.
#[test]
fn an_event_bound_by_ref_to_another_field_runs_when_that_field_is_clicked() {
    let template = [
        field("Button", ""),
        field(
            "Watcher",
            "<event activity=\"click\" ref=\"Button\"><script>\
             Log = Concat(Log, $event.target, \"/\", $, \"!\")</script></event>",
        ),
        field("Log", ""),
    ]
    .concat();
    let values = [("Button", "pressed"), ("Watcher", "w"), ("Log", "")];
    let activation = click(&template, &values, "Button");
    assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
    assert_eq!(log(&activation).as_deref(), Some("pressed/w!"));
    assert!(activation.report.is_empty(), "{:?}", activation.report);
    assert_eq!(log(&click(&template, &values, "Watcher")), None);
}

/// Pages 387 and 706: an event bound by `ref` to a subform with `listen="refAndDescendents"`
/// responds to the activity on every object the subform contains, from wherever the event is
/// placed; a field outside the subform reaches it not at all. And a `change` bound by `ref` runs
/// at a person's entry into the field it names (page 391).
#[test]
fn an_event_bound_by_ref_to_a_subform_hears_what_the_subform_contains() {
    let template = [
        format!("<subform name=\"Panel\">{}</subform>", field("Inner", "")),
        field("Outside", ""),
        field(
            "Watcher",
            "<event activity=\"click\" ref=\"Panel\" listen=\"refAndDescendents\"><script>\
             Log = Concat(Log, \"heard^\")</script></event>\
             <event activity=\"change\" ref=\"Outside\"><script>\
             Log = Concat(Log, \"changed^\")</script></event>",
        ),
        field("Log", ""),
    ]
    .concat();
    let values = [
        ("Panel.Inner", ""),
        ("Outside", "x"),
        ("Watcher", ""),
        ("Log", ""),
    ];
    assert_eq!(
        log(&click(&template, &values, "Panel.Inner")).as_deref(),
        Some("heard^")
    );
    assert_eq!(log(&click(&template, &values, "Outside")), None);
    assert_eq!(
        log(&entered(&template, &values, &["Outside"])).as_deref(),
        Some("changed^")
    );
}

/// Page 707: a `ref` that names no field, subform or exclusion group binds the event to nothing
/// that generates it, so it never runs, and the end of the open sequence names it.
#[test]
fn an_event_bound_by_ref_to_nothing_is_named_and_not_run() {
    let template = [
        field("Button", &bound("click", "Nowhere", "lost")),
        field("Log", ""),
    ]
    .concat();
    let values = [("Button", ""), ("Log", "")];
    assert_eq!(log(&click(&template, &values, "Button")), None);
    let activation = ready(&template, &values);
    assert!(
        activation.report.iter().any(|sentence| sentence.contains(
            "its click event is bound to Nowhere, which names no field, subform or exclusion group"
        )),
        "{:?}",
        activation.report
    );
}

/// Pages 390–391: a field's `enter` and `exit` fire as it gains and loses the focus, and an
/// exclusion group's only for focus that comes in from outside the group or goes out of it. The
/// focus on a radio button's widget is on the group's field its on value names, so between two
/// buttons of one group only the buttons' events run, and leaving the group for another field runs
/// the button's `exit` and then the group's, innermost first (ADR 2566's order).
#[test]
fn the_focus_on_a_button_of_an_exclusion_group_runs_that_fields_enter_and_exit() {
    let focus = |tag: &str| {
        format!(
            "<event activity=\"enter\"><script>Log = Concat(Log, \"{tag}+\")</script></event>\
             <event activity=\"exit\"><script>Log = Concat(Log, \"{tag}-\")</script></event>"
        )
    };
    let template = [
        format!(
            "<exclGroup name=\"Choice\">{}{}{}</exclGroup>",
            focus("Group"),
            radio("Agree", "y", &focus("Agree")),
            radio("Refuse", "n", &focus("Refuse"))
        ),
        field("Other", ""),
        field("Log", ""),
    ]
    .concat();
    let values = [("Choice", ""), ("Other", ""), ("Log", "")];
    let (group, other) = (name("Choice"), name("Other"));
    let exit = |reaching: Option<(&str, Option<&str>)>| {
        activate(
            &template,
            &values,
            Stimulus::Exit {
                left: &group,
                reaching: reaching.map(|(name, _)| name),
                left_button: Some("y"),
                reaching_button: reaching.and_then(|(_, on)| on),
                exited_at_selection: false,
                committed: None,
            },
            Limits::DEFAULT,
        )
    };
    assert_eq!(
        log(&exit(Some((&group, Some("n"))))).as_deref(),
        Some("Agree-")
    );
    assert_eq!(
        log(&exit(Some((&other, None)))).as_deref(),
        Some("Agree-Group-")
    );
    let enter = activate(
        &template,
        &values,
        Stimulus::Enter {
            reached: &group,
            leaving: Some(&group),
            reached_button: Some("n"),
            leaving_button: Some("y"),
        },
        Limits::DEFAULT,
    );
    assert_eq!(log(&enter).as_deref(), Some("Refuse+"));
}

/// Principle 3: each `ref` that searches the template is charged a step for every container the
/// template holds, so a template of many containers each naming another ends on the activation's
/// steps limit and keeps nothing (page 380's means of ending what does not settle), rather than
/// costing the square of its size; the same events within the limit run.
#[test]
fn events_bound_by_ref_are_charged_against_the_steps_limit() {
    let watchers: String = (0..100)
        .map(|at| {
            field(
                &format!("w{at}"),
                "<event activity=\"click\" ref=\"Button\"><script>Log = \"seen\"</script></event>",
            )
        })
        .collect();
    let template = [field("Button", ""), watchers, field("Log", "")].concat();
    let values = [("Button", ""), ("Log", "")];
    let clicked = name("Button");
    let stimulus = Stimulus::Click {
        clicked: &clicked,
        button: None,
    };
    let tight = Limits {
        steps: 1_000,
        ..Limits::DEFAULT
    };
    let ended = activate(&template, &values, stimulus, tight);
    assert_eq!(ended.ended, Ended::Budget("steps"), "{:?}", ended.report);
    assert!(ended.edits.is_empty(), "{:?}", ended.edits);
    let settled = activate(&template, &values, stimulus, Limits::DEFAULT);
    assert_eq!(log(&settled).as_deref(), Some("seen"));
}

/// Activates `stimulus` over a form of `template` holding `values`, whose `config` packet is
/// `config` where it is given.
fn act(
    template: &str,
    values: &[(&str, &str)],
    config: Option<&str>,
    stimulus: Stimulus<'_>,
) -> Activation {
    let document = configured(template, values, config);
    let resource = Resource::read(&document).unwrap().unwrap();
    resource.activate(
        &fields(values),
        stimulus,
        &mut FormCalcRunner,
        Limits::DEFAULT,
    )
}

/// A submit a click on the field `short` raised (XFA 3.3 page 389), its `preSubmit` half.
fn pre_submit(
    template: &str,
    values: &[(&str, &str)],
    config: Option<&str>,
    short: &str,
) -> Activation {
    let clicked = name(short);
    act(
        template,
        values,
        config,
        Stimulus::PreSubmit {
            clicked: Some(&clicked),
            button: None,
        },
    )
}

/// XFA 3.3 page 388: `preSave` and `postSave` are application events, which a script binds to by
/// the `ref` `$host` (page 387), fired "just before" and "just after" the form is written out. So
/// a save's first half runs the `preSave` bound to `$host` and nothing else, and its second half
/// the `postSave`; one bound to the Form DOM or to its own field names an object that generates no
/// save, and does not run.
#[test]
fn a_save_runs_the_applications_pre_save_and_then_its_post_save_events() {
    let template = [
        field("Pre", &bound("preSave", "$host", "pre")),
        field("Alias", &bound("preSave", "xfa.host", "alias")),
        field("Post", &bound("postSave", "$host", "post")),
        field("OnForm", &bound("preSave", "$form", "form")),
        field("Own", &bound("preSave", "", "own")),
        field("Log", ""),
    ]
    .concat();
    let values = [
        ("Pre", ""),
        ("Alias", ""),
        ("Post", ""),
        ("OnForm", ""),
        ("Own", ""),
        ("Log", ""),
    ];
    let before = act(&template, &values, None, Stimulus::PreSave);
    assert_eq!(before.ended, Ended::Settled, "{:?}", before.report);
    assert_eq!(log(&before).as_deref(), Some("pre!alias!"));
    assert!(!before.cancelled);
    let after = act(&template, &values, None, Stimulus::PostSave);
    assert_eq!(log(&after).as_deref(), Some("post!"));
}

/// XFA 3.3 page 389: "The preSubmit event applies only to the Form DOM ($form)", and its example
/// 10.7 tells two submit buttons apart by `$event.target`: page 398 has a secondary event leave the
/// primary properties as the primary event set them, so the script reads the click's `name` and
/// `target`. A `preSubmit` bound to `$host` is not run; `postSubmit` follows under the same click.
#[test]
fn a_submit_runs_the_form_doms_pre_submit_under_the_click_that_raised_it() {
    let reads = |activity: &str, reference: &str, tag: &str| {
        format!(
            "<event activity=\"{activity}\" ref=\"{reference}\"><script>Log = Concat(Log, \
             \"{tag}:\", $event.name, \"=\", $event.target, \"!\")</script></event>"
        )
    };
    let template = [
        field("One", &reads("preSubmit", "$form", "pre")),
        field("Two", &reads("postSubmit", "$form", "post")),
        field("Host", &reads("preSubmit", "$host", "host")),
        field("Log", ""),
    ]
    .concat();
    let values = [("One", "a"), ("Two", "b"), ("Host", ""), ("Log", "")];
    for (clicked, shown) in [("One", "a"), ("Two", "b")] {
        let before = pre_submit(&template, &values, None, clicked);
        assert_eq!(before.ended, Ended::Settled, "{:?}", before.report);
        assert_eq!(
            log(&before).as_deref(),
            Some(format!("pre:click={shown}!").as_str()),
            "{:?}",
            before.report
        );
        let named = name(clicked);
        let after = act(
            &template,
            &values,
            None,
            Stimulus::PostSubmit {
                clicked: Some(&named),
                button: None,
            },
        );
        assert_eq!(
            log(&after).as_deref(),
            Some(format!("post:click={shown}!").as_str())
        );
    }
}

/// XFA 3.3 page 402: `cancelAction` "is cleared by the XFA processor at the beginning of an action
/// such as submitting data", "[s]etting this flag at any time before the action is consumated
/// causes the action to abort", and "[s]cripts must not attempt to clear this flag". So the first
/// `preSubmit` reads it clear and sets it, the second reads it set, and the activation says the
/// submit is cancelled; a script that writes a false value is stopped, and cancels nothing.
#[test]
fn a_pre_submit_script_that_sets_cancel_action_cancels_the_submit() {
    let template = [
        field(
            "First",
            "<event activity=\"preSubmit\" ref=\"$form\"><script>Log = Concat(Log, \
             $event.cancelAction, \"!\")\n$event.cancelAction = 1</script></event>",
        ),
        field(
            "Second",
            "<event activity=\"preSubmit\" ref=\"$form\"><script>Log = Concat(Log, \
             $event.cancelAction, \"!\")</script></event>",
        ),
        field("Log", ""),
    ]
    .concat();
    let values = [("First", ""), ("Second", ""), ("Log", "")];
    let cancelled = pre_submit(&template, &values, None, "First");
    assert_eq!(
        log(&cancelled).as_deref(),
        Some("0!1!"),
        "{:?}",
        cancelled.report
    );
    assert!(cancelled.cancelled);

    let clears = [
        field(
            "First",
            "<event activity=\"preSubmit\" ref=\"$form\"><script>$event.cancelAction = 0\n\
             Log = \"ran\"</script></event>",
        ),
        field("Log", ""),
    ]
    .concat();
    let refused = pre_submit(&clears, &[("First", ""), ("Log", "")], None, "First");
    assert!(!refused.cancelled);
    assert_eq!(log(&refused), None);
    assert!(
        refused
            .report
            .iter()
            .any(|sentence| sentence.contains("must not attempt to clear it")
                && sentence.contains("page 402")),
        "{:?}",
        refused.report
    );
}

/// XFA 3.3 page 398: using a property the current event did not set is an error, and a click is no
/// action such as a save or a submit, so it sets no `cancelAction` to read or to write.
#[test]
fn a_click_sets_no_cancel_action() {
    for script in [
        "Log = $event.cancelAction",
        "$event.cancelAction = 1\nLog = \"ran\"",
    ] {
        let template = [
            field(
                "Button",
                &format!("<event activity=\"click\"><script>{script}</script></event>"),
            ),
            field("Log", ""),
        ]
        .concat();
        let activation = click(&template, &[("Button", ""), ("Log", "")], "Button");
        assert_eq!(log(&activation), None, "{script}");
        assert!(!activation.cancelled);
        assert!(
            activation.report.iter().any(
                |sentence| sentence.contains("$event.cancelAction") && sentence.contains("398")
            ),
            "{script}: {:?}",
            activation.report
        );
    }
}

/// XFA 3.3 page 1340: validations run "automatically ... before specified activities", `preSubmit`
/// alone by default and an empty element preventing even that, and page 382 makes the script test
/// one of them. So a submit tests every container's script — a field nobody entered into among
/// them — where the `config` lists `preSubmit` or states nothing, and a save only where it lists
/// `preSave`. Page 1340 has an error cancel a submit; the tree names the failure and leaves the
/// cancelling to the host's submit policy (ADR 2475), so the activation is not cancelled.
#[test]
fn the_script_tests_run_before_the_activities_the_config_lists() {
    let template = [
        field(
            "Amount",
            &validate("Amount &lt;= 100", "", Some("At most 100.")),
        ),
        field("Log", ""),
    ]
    .concat();
    let values = [("Amount", "150"), ("Log", "")];
    let listed = |activities: &str| format!("<acrobat><validate>{activities}</validate></acrobat>");
    let failing = vec![vec![(name("Amount"), Level::Error)]];
    for (config, at_save, at_submit) in [
        (None, false, true),
        (Some(listed("preSubmit")), false, true),
        (Some(listed("preSave")), true, false),
        (Some(listed("preSave preSubmit")), true, true),
        (Some(listed("")), false, false),
    ] {
        let saving = act(&template, &values, config.as_deref(), Stimulus::PreSave);
        let submitting = pre_submit(&template, &values, config.as_deref(), "Log");
        let expect = |tested: bool| if tested { failing.clone() } else { Vec::new() };
        assert_eq!(invalid(&saving), expect(at_save), "{config:?} at a save");
        assert_eq!(
            invalid(&submitting),
            expect(at_submit),
            "{config:?} at a submit"
        );
        assert!(!submitting.cancelled);
        let after = act(&template, &values, config.as_deref(), Stimulus::PostSave);
        assert!(invalid(&after).is_empty(), "no post-event validates");
    }
}

/// An event of `activity` whose script appends `tag` to `Log`.
fn logs(activity: &str, tag: &str) -> String {
    format!("<event activity=\"{activity}\"><script>Log = Concat(Log, \"{tag}\")</script></event>")
}

/// Activates each stimulus in turn over one form of `template`, each over its own values, with
/// the validation state kept from one to the next as a window keeps it.
fn in_turn(template: &str, steps: &[(&[(&str, &str)], Stimulus<'_>)]) -> Vec<Activation> {
    let document = document(
        template,
        steps.first().map_or(&[][..], |(values, _)| values),
    );
    let resource = Resource::read(&document).unwrap().unwrap();
    let mut kept = Validity::default();
    steps
        .iter()
        .map(|(values, stimulus)| {
            resource.activate_kept(
                &fields(values),
                *stimulus,
                &mut FormCalcRunner,
                Limits::DEFAULT,
                &mut kept,
            )
        })
        .collect()
}

/// XFA 3.3 page 396: the `validationState` event is triggered for each field, exclusion group and
/// subform when the object is first initialized, which for a form is the merge; page 398's table
/// has it set `name` and `target` on all three. Its place after the `initialize` events, and the
/// document order among the objects, are ADR 2706's choices.
#[test]
fn every_object_announces_its_validation_state_when_the_form_is_initialized() {
    let named = "<event activity=\"validationState\"><script>Log = Concat(Log, \"S:\", \
                 $event.name, \";\")</script></event>";
    let template = [
        format!(
            "<subform name=\"Part\">{}{named}{}</subform>",
            logs("initialize", "i;"),
            field("Amount", &logs("validationState", "F;"))
        ),
        format!(
            "<exclGroup name=\"Choice\">{}{}</exclGroup>",
            logs("validationState", "G;"),
            field("Yes", "")
        ),
        field("Log", ""),
    ]
    .concat();
    let activation = merge(&template, &[("Part.Amount", "5"), ("Log", "")]);
    assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
    assert_eq!(
        log(&activation).as_deref(),
        Some("i;S:validationState;F;G;")
    );
}

/// Page 396: after the initialization the event fires only where the object changes from valid to
/// invalid or back; page 397: a subform is invalid where any of its content fails a validation.
/// The script test is re-evaluated at each entry (page 396), so an entry of 50 into a field whose
/// script test is `Amount < 10` makes it and its subform invalid, a second invalid entry changes
/// neither state, and a valid one makes both valid again.
#[test]
fn a_validation_state_event_fires_where_an_entry_changes_the_state_and_nowhere_else() {
    let template = [
        format!(
            "<subform name=\"Part\">{}{}</subform>",
            logs("validationState", "S;"),
            field(
                "Amount",
                &[
                    validate("Amount &lt; 10", "", None),
                    logs("validationState", "F;"),
                ]
                .concat()
            )
        ),
        field("Log", ""),
    ]
    .concat();
    let typed = [name("Part.Amount")];
    let entry = Stimulus::Entry {
        entered: &typed,
        before: &[],
        typing: &[],
    };
    let steps: [(&[(&str, &str)], Stimulus<'_>); 4] = [
        (&[("Part.Amount", "5"), ("Log", "")], Stimulus::Merge),
        (&[("Part.Amount", "50"), ("Log", "")], entry),
        (&[("Part.Amount", "60"), ("Log", "")], entry),
        (&[("Part.Amount", "5"), ("Log", "")], entry),
    ];
    let logged: Vec<Option<String>> = in_turn(&template, &steps)
        .iter()
        .map(|activation| {
            assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
            log(activation)
        })
        .collect();
    assert_eq!(
        logged,
        [Some("S;F;"), Some("S;F;"), None, Some("S;F;")].map(|said| said.map(str::to_owned))
    );
}

/// Page 396's own case: a field holding a value is valid, a person clears it, and that does not run
/// its null test, so the field still registers a valid state; only the null test a submit's
/// pre-event applies (page 1340's default, `preSubmit`) makes it invalid, and the event fires
/// then. A second submit finds the same state and fires nothing.
#[test]
fn a_cleared_field_turns_invalid_only_at_the_pre_event_that_runs_its_null_test() {
    let template = [
        field(
            "Amount",
            &[
                validate("Amount &lt; 10", "nullTest=\"error\"", None),
                logs("validationState", "F;"),
            ]
            .concat(),
        ),
        field("Log", ""),
    ]
    .concat();
    let typed = [name("Amount")];
    let submit = Stimulus::PreSubmit {
        clicked: None,
        button: None,
    };
    let steps: [(&[(&str, &str)], Stimulus<'_>); 4] = [
        (&[("Amount", "5"), ("Log", "")], Stimulus::Merge),
        (
            &[("Amount", ""), ("Log", "")],
            Stimulus::Entry {
                entered: &typed,
                before: &[],
                typing: &[],
            },
        ),
        (&[("Amount", ""), ("Log", "")], submit),
        (&[("Amount", ""), ("Log", "")], submit),
    ];
    let logged: Vec<Option<String>> = in_turn(&template, &steps)
        .iter()
        .map(|activation| {
            assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
            log(activation)
        })
        .collect();
    assert_eq!(
        logged,
        [Some("F;"), None, Some("F;"), None].map(|said| said.map(str::to_owned))
    );
}

/// A choice list whose `enter` logs `p+` and `$event.reenter`, and whose `exit` logs `p-`, in a
/// subform `Part` logging its own focus events, and the log.
fn choice_form() -> (String, Vec<(&'static str, &'static str)>) {
    let pick = "<field name=\"pick\"><ui><choiceList/></ui><value><text/></value>\
                <event activity=\"enter\"><script>Log = Concat(Log, \"p+\", $event.reenter)\
                </script></event>\
                <event activity=\"exit\"><script>Log = Concat(Log, \"p-\")</script></event>\
                </field>";
    let template = [
        format!("<subform name=\"Part\">{}{pick}</subform>", logged("P")),
        field("Log", ""),
    ]
    .concat();
    (template, vec![("Part.pick", "one"), ("Log", "")])
}

/// XFA 3.3 page 392, from XFA 2.4: a person committing a selection in a choice list fires its
/// `exit` though the focus remains; a later selection while it keeps the focus fires its `enter`
/// again, and then its `exit`; page 403 makes `reenter` false for the first `enter` after the list
/// gains the focus and true for the later ones. Since the commit's `exit` paired the last `enter`,
/// the focus then leaving fires the `exit` of the subform it goes out of and not the list's again
/// (ADR 2707's reading of the page).
#[test]
fn a_choice_list_exits_at_each_selection_and_reenters_before_the_next() {
    let (template, values) = choice_form();
    let pick = name("Part.pick");
    let exit = |exited_at_selection| Stimulus::Exit {
        left: &pick,
        reaching: None,
        left_button: None,
        reaching_button: None,
        exited_at_selection,
        committed: None,
    };
    for (stimulus, expected) in [
        (
            Stimulus::Enter {
                reached: &pick,
                leaving: None,
                reached_button: None,
                leaving_button: None,
            },
            "P+p+0",
        ),
        (
            Stimulus::Selected {
                field: &pick,
                committed: None,
            },
            "p-",
        ),
        (Stimulus::Reentered { field: &pick }, "p+1"),
        (exit(true), "P-"),
        (exit(false), "p-P-"),
    ] {
        let activation = activate(&template, &values, stimulus, Limits::DEFAULT);
        assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
        assert_eq!(log(&activation).as_deref(), Some(expected), "{stimulus:?}");
    }
}

/// XFA 3.3 pages 392 and 402: a choice list's selection commit fires its `exit`, which reads
/// `$event.commitKey` as what happened to the value — 2 where the window says Enter committed the
/// selection. A click on an item is within the field, none of page 402's "click of the mouse
/// outside the field", so a window that cannot say more leaves the property unoffered and the
/// script's read is refused by name (ADR 2755).
#[test]
fn a_choice_lists_selection_exit_reads_enter_as_commit_key_2() {
    let pick = "<field name=\"pick\"><ui><choiceList/></ui><value><text/></value>\
                <event activity=\"exit\"><script>Log = Concat(Log, \"k\", $event.commitKey)\
                </script></event></field>";
    let template = [
        format!("<subform name=\"Part\">{pick}</subform>"),
        field("Log", ""),
    ]
    .concat();
    let values = [("Part.pick", "one"), ("Log", "")];
    let field_name = name("Part.pick");
    let selected = |committed| {
        activate(
            &template,
            &values,
            Stimulus::Selected {
                field: &field_name,
                committed,
            },
            Limits::DEFAULT,
        )
    };
    let enter = selected(Some(CommitKey::Enter));
    assert_eq!(log(&enter).as_deref(), Some("k2"), "{:?}", enter.report);
    let clicked = selected(None);
    assert_eq!(log(&clicked), None, "{:?}", clicked.report);
    assert!(
        clicked.report.iter().any(|said| said
            .contains("$event.commitKey: the window did not say how the value was committed")),
        "{:?}",
        clicked.report
    );
}

/// Pages 392–393: a template of XFA 3.0 whose `originalXFAVersion` processing instruction names
/// 2.1 and states `LegacyEventModel:1` follows the behaviour before XFA 2.4, which fired an `exit`
/// at every selection and an `enter` only on the first, so no re-entry runs and the list's `exit`
/// fires again when the focus leaves.
#[test]
fn a_template_that_asks_for_the_legacy_event_model_fires_no_reentry() {
    let (template, values) = choice_form();
    let head = "<template xmlns=\"http://www.xfa.org/schema/xfa-template/3.0/\">\
                <?originalXFAVersion http://www.xfa.org/schema/xfa-template/2.1/ \
                LegacyEventModel:1 ?>";
    let document = headed(head, &template, &values, None);
    let resource = Resource::read(&document).unwrap().unwrap();
    let pick = name("Part.pick");
    for (stimulus, expected) in [
        (
            Stimulus::Selected {
                field: &pick,
                committed: None,
            },
            Some("p-"),
        ),
        (Stimulus::Reentered { field: &pick }, None),
        (
            Stimulus::Exit {
                left: &pick,
                reaching: None,
                left_button: None,
                reaching_button: None,
                exited_at_selection: true,
                committed: None,
            },
            Some("p-P-"),
        ),
    ] {
        let activation = resource.activate(
            &fields(&values),
            stimulus,
            &mut FormCalcRunner,
            Limits::DEFAULT,
        );
        assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
        assert_eq!(log(&activation).as_deref(), expected, "{stimulus:?}");
    }
}

/// Page 403: `reenter` is false for a field that is no choice list, which page 398's table has a
/// field's `click` set; of a choice list's `click` the definition says nothing, so reading it is
/// refused by name rather than answered.
#[test]
fn reenter_is_false_on_a_text_field_and_refused_on_a_choice_list_s_click() {
    let reads = "<event activity=\"click\"><script>Log = Concat(Log, \"c\", $event.reenter)\
                 </script></event>";
    let template = [
        field("Text", reads),
        format!("<field name=\"pick\"><ui><choiceList/></ui><value><text/></value>{reads}</field>"),
        field("Log", ""),
    ]
    .concat();
    let values = [("Text", "a"), ("pick", "one"), ("Log", "")];
    let (text, pick) = (name("Text"), name("pick"));
    let clicked = activate(
        &template,
        &values,
        Stimulus::Click {
            clicked: &text,
            button: None,
        },
        Limits::DEFAULT,
    );
    assert_eq!(log(&clicked).as_deref(), Some("c0"), "{:?}", clicked.report);
    let refused = activate(
        &template,
        &values,
        Stimulus::Click {
            clicked: &pick,
            button: None,
        },
        Limits::DEFAULT,
    );
    assert_eq!(log(&refused), None);
    assert!(
        refused
            .report
            .iter()
            .any(|said| said.contains("$event.reenter")),
        "{:?}",
        refused.report
    );
}

/// XFA 3.3 pages 398 and 402: a field's `exit` sets `$event.commitKey`, how the value was
/// committed as the focus left the field — 3 for a tab to a new field, 1 for a click of the mouse
/// outside it. Where the window does not say, the property is refused by name rather than
/// answered; a subform's `exit` and a field's `enter` are rows of page 398's table that do not set
/// it at all (ADR 2726).
#[test]
fn a_fields_exit_reads_how_its_value_was_committed() {
    let template = [
        format!(
            "<subform name=\"Part\">{}{}</subform>",
            "<event activity=\"exit\"><script>Log = Concat(Log, \"P\", $event.commitKey)\
             </script></event>",
            field(
                "Amount",
                "<event activity=\"exit\"><script>Log = Concat(Log, \"k\", $event.commitKey, \
                 \";\")</script></event><event activity=\"enter\"><script>Log = Concat(Log, \
                 \"e\", $event.commitKey)</script></event>"
            )
        ),
        field("Other", ""),
        field("Log", ""),
    ]
    .concat();
    let values = [("Part.Amount", "5"), ("Other", ""), ("Log", "")];
    let (left, reached) = (name("Part.Amount"), name("Other"));
    let exit = |committed| {
        activate(
            &template,
            &values,
            Stimulus::Exit {
                left: &left,
                reaching: Some(&reached),
                left_button: None,
                reaching_button: None,
                exited_at_selection: false,
                committed,
            },
            Limits::DEFAULT,
        )
    };
    // Page 402's 0, "[t]he value was not committed (for example, the escape key was pressed)",
    // is what a window says of a field a person left with Escape (ADR 2756).
    for (committed, expected) in [
        (Some(Exited::Committed(CommitKey::Tab)), "k3;"),
        (Some(Exited::Committed(CommitKey::Click)), "k1;"),
        (Some(Exited::Committed(CommitKey::Enter)), "k2;"),
        (Some(Exited::NotCommitted), "k0;"),
    ] {
        let activation = exit(committed);
        assert_eq!(
            log(&activation).as_deref(),
            Some(expected),
            "{:?}",
            activation.report
        );
        assert!(
            activation
                .report
                .iter()
                .any(|said| said.contains("$event.commitKey: the exit event does not set it")),
            "the subform's exit: {:?}",
            activation.report
        );
    }
    let untold = exit(None);
    assert_eq!(log(&untold), None, "{:?}", untold.report);
    assert!(
        untold.report.iter().any(|said| said
            .contains("$event.commitKey: the window did not say how the value was committed")),
        "{:?}",
        untold.report
    );
    let enter = activate(
        &template,
        &values,
        Stimulus::Enter {
            reached: &left,
            leaving: Some(&reached),
            reached_button: None,
            leaving_button: None,
        },
        Limits::DEFAULT,
    );
    assert_eq!(log(&enter), None);
    assert!(
        enter
            .report
            .iter()
            .any(|said| said.contains("$event.commitKey: the enter event does not set it")),
        "{:?}",
        enter.report
    );
}

/// XFA 3.3 pages 398 and 403: a field's `change` sets `$event.newContentType` and
/// `$event.prevContentType`, which are always the same in this version: `allowRichText` where the
/// field supports rich text and `plainTextOnly` where it does not. Page 865: a `textEdit`'s
/// `allowRichText` is 1 or 0 as stated, and 0 where it is not stated; a field whose `ui` is no
/// `textEdit` takes no styled text. A click sets neither.
#[test]
fn a_change_reads_whether_the_field_takes_rich_text() {
    let reads = "<event activity=\"change\"><script>Log = Concat($event.newContentType, \"|\", \
                 $event.prevContentType)</script></event>";
    for (ui, expected) in [
        (
            "<textEdit allowRichText=\"1\"/>",
            "allowRichText|allowRichText",
        ),
        (
            "<textEdit allowRichText=\"0\"/>",
            "plainTextOnly|plainTextOnly",
        ),
        ("<textEdit/>", "plainTextOnly|plainTextOnly"),
        ("<numericEdit/>", "plainTextOnly|plainTextOnly"),
    ] {
        let template = [
            format!("<field name=\"Note\"><ui>{ui}</ui><value><text/></value>{reads}</field>"),
            field("Log", ""),
        ]
        .concat();
        let activation = entry(&template, &[("Note", "7"), ("Log", "")], "Note", None);
        assert_eq!(
            log(&activation).as_deref(),
            Some(expected),
            "{ui}: {:?}",
            activation.report
        );
    }
    let template = [
        field(
            "Button",
            "<event activity=\"click\"><script>Log = $event.newContentType</script></event>",
        ),
        field("Log", ""),
    ]
    .concat();
    let activation = click(&template, &[("Button", ""), ("Log", "")], "Button");
    assert_eq!(log(&activation), None);
    assert!(
        activation
            .report
            .iter()
            .any(|said| said.contains("$event.newContentType: the click event does not set it")),
        "{:?}",
        activation.report
    );
}

/// A text field `Code` whose `validate` picture is `AAA-999` — three letters, a hyphen, three
/// digits — at the `formatTest` level `attributes` states, with the designer's message for the
/// format test (XFA 3.3 page 383).
fn coded(attributes: &str) -> String {
    format!(
        "<field name=\"Code\"><ui><textEdit/></ui><validate {attributes}><message><text \
         name=\"formatTest\">Three letters and three digits.</text></message><picture>AAA-999\
         </picture></validate><value><text/></value>\
         <event activity=\"validationState\"><script>Log = Concat(Log, \"S;\")</script></event>\
         </field>"
    )
}

/// The format test failures of an activation: each container, its test and its level.
fn format_failures(activation: &Activation) -> Vec<(String, Level, Option<String>)> {
    activation
        .script_tests
        .iter()
        .flatten()
        .filter(|failed| failed.test == Test::Format)
        .map(|failed| {
            (
                failed.container.clone(),
                failed.level,
                failed.message.clone(),
            )
        })
        .collect()
}

/// XFA 3.3 page 396: the format test is re-evaluated when content changes, and is applied to no
/// field holding null or empty content; page 382: the `validate` picture must format the
/// canonical value; pages 384–385: `formatTest` is `warning` unless it says `error` or
/// `disabled`; page 383: the designer's message is the `message` element's `text` named
/// `formatTest`. The picture formats the canonical value, so `ABC123` passes and the text it
/// would display, `ABC-123`, typed with no edit picture to parse it, is canonical as typed and
/// fails. An entry tests the field entered and no field it did not change (ADR 2727).
#[test]
fn an_entry_puts_the_field_to_its_format_test() {
    let said = Some("Three letters and three digits.".to_owned());
    for (attributes, value, expected) in [
        ("", "12", Some(Level::Warning)),
        ("formatTest=\"warning\"", "12", Some(Level::Warning)),
        ("formatTest=\"error\"", "12", Some(Level::Error)),
        ("formatTest=\"disabled\"", "12", None),
        ("formatTest=\"error\"", "ABC123", None),
        ("formatTest=\"error\"", "ABC-123", Some(Level::Error)),
        ("formatTest=\"error\"", "", None),
    ] {
        let template = [coded(attributes), field("Log", "")].concat();
        let activation = entry(&template, &[("Code", value), ("Log", "")], "Code", None);
        assert_eq!(activation.ended, Ended::Settled, "{:?}", activation.report);
        assert_eq!(
            format_failures(&activation),
            expected
                .map(|level| (name("Code"), level, said.clone()))
                .into_iter()
                .collect::<Vec<_>>(),
            "{attributes:?} holding {value:?}"
        );
    }
    // An entry into another field leaves the code's failing value untested.
    let template = [
        coded("formatTest=\"error\""),
        field("Other", ""),
        field("Log", ""),
    ]
    .concat();
    let activation = entry(
        &template,
        &[("Code", "12"), ("Other", "x"), ("Log", "")],
        "Other",
        None,
    );
    assert!(
        format_failures(&activation).is_empty(),
        "{:?}",
        activation.script_tests
    );
}

/// XFA 3.3 pages 382 and 396: a merge initializes the form and runs its format tests, and the
/// format test's verdict is the validation state the `validationState` event follows: a code
/// that conforms at the merge and fails after an entry changes state once, and an entry that
/// leaves it failing changes nothing.
#[test]
fn a_format_test_that_fails_after_an_entry_announces_the_change_of_state() {
    let template = [coded("formatTest=\"error\""), field("Log", "")].concat();
    let code = name("Code");
    let typed = [code.clone()];
    let entry = Stimulus::Entry {
        entered: &typed,
        before: &[],
        typing: &[],
    };
    let ran = in_turn(
        &template,
        &[
            (&[("Code", "ABC123"), ("Log", "")], Stimulus::Merge),
            (&[("Code", "12"), ("Log", "")], entry),
            (&[("Code", "34"), ("Log", "")], entry),
        ],
    );
    assert!(format_failures(&ran[0]).is_empty(), "{:?}", ran[0].report);
    assert_eq!(
        log(&ran[0]).as_deref(),
        Some("S;"),
        "the merge announces every state"
    );
    assert_eq!(format_failures(&ran[1]).len(), 1, "{:?}", ran[1].report);
    assert_eq!(log(&ran[1]).as_deref(), Some("S;"), "valid to invalid");
    assert_eq!(format_failures(&ran[2]).len(), 1);
    assert_eq!(log(&ran[2]), None, "invalid for the same reason");

    let failing = merge(&template, &[("Code", "12"), ("Log", "")]);
    assert_eq!(
        format_failures(&failing),
        vec![(
            code,
            Level::Error,
            Some("Three letters and three digits.".to_owned())
        )]
    );
}

/// Activates `stimulus` over a form of `template` holding `values`, the window having held `keys`
/// during the person's action.
fn held(
    template: &str,
    values: &[(&str, &str)],
    stimulus: Stimulus<'_>,
    keys: Option<pdf_model::view::Keys>,
) -> Activation {
    let document = document(template, values);
    let resource = Resource::read(&document).unwrap().unwrap();
    resource.activate_held(
        &fields(values),
        stimulus,
        keys,
        &mut FormCalcRunner,
        Limits::DEFAULT,
        &mut Validity::default(),
    )
}

/// A subform `Part` whose `enter` reads `$event.shift`, around a field `Amount` whose `click`,
/// `enter`, `exit`, `mouseDown` and `change` log both keys; a field `Other` whose `click` writes
/// `$event.shift`; and the log.
fn keys_form() -> (String, [(&'static str, &'static str); 3]) {
    let reads = |activity: &str| {
        format!(
            "<event activity=\"{activity}\"><script>Log = Concat(Log, \"{activity}:s\", \
             $event.shift, \"m\", $event.modifier, \";\")</script></event>"
        )
    };
    let template = [
        format!(
            "<subform name=\"Part\"><event activity=\"enter\"><script>Log = Concat(Log, \
             $event.shift)</script></event>{}</subform>",
            field(
                "Amount",
                &["click", "enter", "exit", "mouseDown", "change"]
                    .map(reads)
                    .concat()
            )
        ),
        field(
            "Other",
            "<event activity=\"click\"><script>$event.shift = 1</script></event>",
        ),
        field("Log", ""),
    ]
    .concat();
    let values = [("Part.Amount", "5"), ("Other", ""), ("Log", "")];
    (template, values)
}

/// XFA 3.3 pages 398 and 403: a field's `click`, `enter`, `exit`, `mouseDown`, `mouseEnter`,
/// `mouseExit`, `mouseUp` and `change` set `$event.shift`, true if and only if the shift key was
/// held down during the event, and `$event.modifier`, the same of the modifier key, Control on
/// Microsoft Windows; a script reads a boolean as 1 or 0. Where the window did not say which keys
/// it held both are refused by name (ADR 2732).
#[test]
fn a_fields_events_read_the_keys_the_window_held() {
    let (template, values) = keys_form();
    let (amount, other) = (name("Part.Amount"), name("Other"));
    let shift = pdf_model::view::Keys {
        shift: true,
        modifier: false,
        arrows: false,
    };
    let control = pdf_model::view::Keys {
        shift: false,
        modifier: true,
        arrows: false,
    };
    let typed = [amount.clone()];
    for (stimulus, activity) in [
        (
            Stimulus::Click {
                clicked: &amount,
                button: None,
            },
            "click",
        ),
        (
            Stimulus::MouseDown {
                pressed: &amount,
                button: None,
            },
            "mouseDown",
        ),
        (
            Stimulus::Exit {
                left: &amount,
                reaching: Some(&other),
                left_button: None,
                reaching_button: None,
                exited_at_selection: false,
                committed: None,
            },
            "exit",
        ),
        (
            Stimulus::Entry {
                entered: &typed,
                before: &[],
                typing: &[],
            },
            "change",
        ),
    ] {
        for (keys, expected) in [
            (shift, format!("{activity}:s1m0;")),
            (control, format!("{activity}:s0m1;")),
        ] {
            let activation = held(&template, &values, stimulus, Some(keys));
            assert_eq!(
                log(&activation),
                Some(expected),
                "{activity}: {:?}",
                activation.report
            );
        }
        let untold = held(&template, &values, stimulus, None);
        assert_eq!(log(&untold), None, "{activity}: {:?}", untold.report);
        assert!(
            untold.report.iter().any(|said| said.contains(&format!(
                "$event.shift: the window did not say which keys it held during the {activity} \
                 event"
            ))),
            "{activity}: {:?}",
            untold.report
        );
    }
}

/// XFA 3.3 pages 398 and 399: a subform's `enter` is a row of page 398's table that sets neither
/// `shift` nor `modifier`, so its script is refused while the field's `enter` reads them, and a
/// script may write neither (ADR 2732).
#[test]
fn a_subforms_enter_reads_no_keys_and_no_script_writes_them() {
    let (template, values) = keys_form();
    let (amount, other) = (name("Part.Amount"), name("Other"));
    let shift = pdf_model::view::Keys {
        shift: true,
        modifier: false,
        arrows: false,
    };
    let entering = held(
        &template,
        &values,
        Stimulus::Enter {
            reached: &amount,
            leaving: Some(&other),
            reached_button: None,
            leaving_button: None,
        },
        Some(shift),
    );
    assert_eq!(log(&entering).as_deref(), Some("enter:s1m0;"));
    assert!(
        entering
            .report
            .iter()
            .any(|said| said.contains("$event.shift: the enter event does not set it")),
        "the subform's enter: {:?}",
        entering.report
    );
    let written = held(
        &template,
        &values,
        Stimulus::Click {
            clicked: &other,
            button: None,
        },
        Some(shift),
    );
    assert!(
        written
            .report
            .iter()
            .any(|said| said.contains("$event.shift: read-only")),
        "{:?}",
        written.report
    );
}

/// XFA 3.3 pages 398 and 402: a field's `change` sets `$event.fullText`, the full (untruncated)
/// value where the field truncated what the person entered, and `$event.keyDown`, true if the
/// arrow key was used to make a selection and otherwise false. The window handed `123456` to a
/// field that kept `1234`, so `change` is `1234` and `fullText` `123456`. A `click` sets neither,
/// and a commit whose typing the window did not describe offers neither (ADR 2732).
#[test]
fn a_change_reads_the_whole_text_and_whether_an_arrow_made_it() {
    let template = [
        field(
            "Amount",
            "<event activity=\"change\"><script>Log = Concat($event.fullText, \"|\", \
             $event.change, \"|\", $event.keyDown)</script></event>\
             <event activity=\"click\"><script>Log = $event.fullText</script></event>",
        ),
        field("Log", ""),
    ]
    .concat();
    let values = [("Amount", "1234"), ("Log", "")];
    let typed = [name("Amount")];
    for (arrows, expected) in [(false, "123456|1234|0"), (true, "123456|1234|1")] {
        let typing = [Some(Typing {
            whole: "123456".to_owned(),
            arrows,
        })];
        let activation = held(
            &template,
            &values,
            Stimulus::Entry {
                entered: &typed,
                before: &[],
                typing: &typing,
            },
            None,
        );
        assert_eq!(
            log(&activation).as_deref(),
            Some(expected),
            "{:?}",
            activation.report
        );
    }
    let untold = entered(&template, &values, &["Amount"]);
    assert_eq!(log(&untold), None, "{:?}", untold.report);
    assert!(
        untold
            .report
            .iter()
            .any(|said| said
                .contains("$event.fullText: the window did not say what the person typed")),
        "{:?}",
        untold.report
    );
    let clicked = click(&template, &values, "Amount");
    assert!(
        clicked
            .report
            .iter()
            .any(|said| said.contains("$event.fullText: the click event does not set it")),
        "{:?}",
        clicked.report
    );
}
