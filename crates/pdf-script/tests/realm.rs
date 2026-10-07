//! One document's realm: what the open defines is what a field's script calls later, every field the
//! realm was told of is reachable through `getField`, and a script's write is an edit (ADRs 1602 and
//! 1603).
//!
//! Every expected value is either one the script itself computes from what it was handed, or one a
//! field state this test wrote states; the reference's spellings — `display.hidden` is 1, a colour
//! is `["RGB", r, g, b]`, a field's `type` is `"text"` — are the documented choices
//! `pdf_model::view`'s types record, read back here.

#![cfg(feature = "engine")]
#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use pdf_model::action::{PageTrigger, Trigger as AnnotationTrigger};
use pdf_model::aform::Trigger;
use pdf_model::view::{
    Alignment, BorderStyle, Colour, Display, FieldState, FieldType, Property, ScriptEdit,
    ScriptSite,
};
use pdf_script::{Budget, Ending, Engine, Event, Outcome, Realm, RefusalKind, Request};

/// A text field's state.
fn field(name: &str, value: &str) -> FieldState {
    FieldState {
        name: name.to_owned(),
        kind: FieldType::Text,
        value: value.to_owned(),
        flags: 0,
        display: Display::Visible,
        text_color: None,
        fill_color: None,
        stroke_color: None,
        border_style: BorderStyle::Solid,
        alignment: Alignment::Left,
        char_limit: None,
        page: Some(0),
        rect: [10.0, 10.0, 210.0, 40.0],
        captions: Default::default(),
    }
}

/// The form every test here tells its realm of.
fn form() -> Vec<FieldState> {
    vec![
        field("Line.1", "2"),
        field("Line.2", "3.5"),
        FieldState {
            flags: 1 << 1,
            char_limit: Some(6),
            alignment: Alignment::Right,
            border_style: BorderStyle::Beveled,
            text_color: Some(Colour::Rgb([0.0, 0.0, 1.0])),
            page: Some(2),
            ..field("Total", "")
        },
        FieldState {
            kind: FieldType::CheckBox,
            ..field("Agree", "Off")
        },
    ]
}

/// A request for `script` at `site`, on `field`, telling the realm of `fields`.
fn request(site: ScriptSite, field: &str, script: &str, fields: Vec<FieldState>) -> Request {
    Request {
        site,
        field: field.to_owned(),
        label: "library".to_owned(),
        script: script.to_owned(),
        event: Event {
            value: String::new(),
            change: String::new(),
            selection_start: 0,
            selection_end: 0,
            will_commit: false,
            commit_key: 0,
            field_full: false,
            change_ex: String::new(),
            source: String::new(),
        },
        fields,
        page: 1,
        pages: 4,
        dirty: false,
        document: None,
        moment: 1_704_465_015_000,
        utc_offset_seconds: 0,
    }
}

/// A realm told of the form.
fn realm() -> Realm {
    let mut realm = Realm::new(Budget::FIELD_EVENT).expect("a realm");
    let told = realm.run(&request(ScriptSite::Library, "", "", form()));
    assert_eq!(told.ending, Ending::Finished);
    realm
}

/// Runs a calculate script on `Total`, telling the realm nothing new.
fn calculate(realm: &mut Realm, script: &str) -> Outcome {
    realm.run(&request(
        ScriptSite::Field(Trigger::Calculate),
        "Total",
        script,
        Vec::new(),
    ))
}

#[test]
fn a_function_the_name_tree_defines_is_what_a_later_field_script_calls() {
    let mut realm = realm();
    let defined = realm.run(&request(
        ScriptSite::Library,
        "",
        "function twice(x) { return 2 * x; } var opened = event.type + '/' + event.name;",
        Vec::new(),
    ));
    assert_eq!(defined.ending, Ending::Finished, "{defined:?}");
    let ran = calculate(&mut realm, "event.value = twice(21) + ' ' + opened;");
    assert_eq!(ran.value.as_deref(), Some("42 Doc/Open"), "{ran:?}");
    let fresh = pdf_script::run(
        &request(
            ScriptSite::Field(Trigger::Calculate),
            "Total",
            "event.value = typeof twice;",
            form(),
        ),
        &Budget::FIELD_EVENT,
    );
    assert_eq!(
        fresh.value.as_deref(),
        Some("undefined"),
        "a one-shot realm holds nothing of another"
    );
}

#[test]
fn get_field_reaches_every_field_and_reads_its_properties() {
    let mut realm = realm();
    let ran = calculate(
        &mut realm,
        r#"var t = this.getField("Total"), l = this.getField("Line.1");
           event.value = [typeof l.value, l.value + 1, l.valueAsString, t.type,
             this.getField("Agree").type, t.display, t.readonly, t.required, t.charLimit,
             t.alignment, t.borderStyle, t.textColor.join(), l.textColor.join(),
             l.fillColor.join(), t.page, l.rect.join(), t.name, t.doc === this,
             this.numFields, this.getNthFieldName(0), this.numPages, this.pageNum,
             this.getField("Nothing")].join("|");"#,
    );
    assert_eq!(ran.ending, Ending::Finished, "{ran:?}");
    assert_eq!(
        ran.value.as_deref(),
        Some(
            "number|3|2|text|checkbox|0|false|true|6|right|beveled|RGB,0,0,1|G,0|T|2|\
             10,40,210,10|Total|true|4|Agree|4|1|"
        ),
        "{ran:?}"
    );
}

#[test]
fn a_write_to_another_field_is_an_edit_the_script_reads_back() {
    let mut realm = realm();
    let ran = calculate(
        &mut realm,
        r#"this.getField("Line.1").value = 5; event.value = this.getField("Line.1").value * 10;"#,
    );
    assert_eq!(ran.value.as_deref(), Some("50"), "{ran:?}");
    assert_eq!(
        ran.edits,
        vec![ScriptEdit::Value {
            field: "Line.1".to_owned(),
            value: "5".to_owned()
        }]
    );
    let later = calculate(
        &mut realm,
        r#"event.value = this.getField("Line.1").value;"#,
    );
    assert_eq!(
        later.value.as_deref(),
        Some("5"),
        "the realm's table keeps what a finished run wrote"
    );
}

#[test]
fn every_property_write_is_an_edit_by_member() {
    let mut realm = realm();
    let ran = calculate(
        &mut realm,
        r#"var f = this.getField("Total");
           f.display = display.noView; f.readonly = true; f.required = false;
           f.textColor = color.red; f.fillColor = ["G", 0.5]; f.strokeColor = ["CMYK", 0, 0, 0, 1];
           f.borderStyle = border.u; f.alignment = "center"; f.charLimit = 3;
           this.getField("Line.2").hidden = true;"#,
    );
    assert_eq!(ran.ending, Ending::Finished, "{ran:?}");
    let total = |property| ScriptEdit::Property {
        field: "Total".to_owned(),
        property,
    };
    assert_eq!(
        ran.edits,
        vec![
            total(Property::Display(Display::NoView)),
            total(Property::ReadOnly(true)),
            total(Property::Required(false)),
            total(Property::TextColor(Colour::Rgb([1.0, 0.0, 0.0]))),
            total(Property::FillColor(Colour::Gray(0.5))),
            total(Property::StrokeColor(Colour::Cmyk([0.0, 0.0, 0.0, 1.0]))),
            total(Property::BorderStyle(BorderStyle::Underline)),
            total(Property::Alignment(Alignment::Center)),
            total(Property::CharLimit(3)),
            ScriptEdit::Property {
                field: "Line.2".to_owned(),
                property: Property::Display(Display::Hidden),
            },
        ]
    );
    let read = calculate(
        &mut realm,
        r#"var f = this.getField("Total");
           event.value = [f.display, f.readonly, f.textColor.join(), f.alignment].join();"#,
    );
    assert_eq!(read.value.as_deref(), Some("3,true,RGB,1,0,0,center"));
}

#[test]
fn a_name_with_fields_below_it_stands_for_all_of_them() {
    let mut realm = realm();
    let ran = calculate(
        &mut realm,
        r#"this.getField("Line").display = display.hidden;"#,
    );
    let fields: Vec<&str> = ran
        .edits
        .iter()
        .map(|edit| match edit {
            ScriptEdit::Property { field, .. } => field.as_str(),
            _ => "",
        })
        .collect();
    assert_eq!(fields, vec!["Line.1", "Line.2"]);
}

#[test]
fn a_value_that_is_no_display_constant_is_refused_by_name() {
    let mut realm = realm();
    let ran = calculate(
        &mut realm,
        r#"try { this.getField("Total").display = 9; } catch (e) { console.println(e.name); }"#,
    );
    assert_eq!(ran.log, vec!["NotAllowedError".to_owned()]);
    assert!(ran.edits.is_empty());
    let refusal = ran.refusals.first().expect("the refusal");
    assert_eq!(refusal.member, "Field.display=");
    assert!(matches!(refusal.kind, RefusalKind::Unreachable(_)));
}

#[test]
fn a_keystroke_script_changes_its_own_field_through_the_event_only() {
    let mut realm = realm();
    let ran = realm.run(&request(
        ScriptSite::Field(Trigger::Keystroke),
        "Total",
        r#"this.getField("Total").value = "9";"#,
        Vec::new(),
    ));
    assert!(matches!(ran.ending, Ending::Threw(_)), "{ran:?}");
    assert_eq!(
        ran.refusals.first().map(|refusal| refusal.member.as_str()),
        Some("Field.value=")
    );
    assert!(ran.edits.is_empty());
}

#[test]
fn a_stopped_run_hands_back_no_edit_and_leaves_the_realm_as_it_was() {
    let mut realm = realm();
    let ran = calculate(
        &mut realm,
        r#"this.getField("Line.1").value = 9; while (true) {}"#,
    );
    assert!(matches!(ran.ending, Ending::Exceeded(_)), "{ran:?}");
    assert!(ran.edits.is_empty());
    let after = calculate(
        &mut realm,
        r#"event.value = this.getField("Line.1").value;"#,
    );
    assert_eq!(after.value.as_deref(), Some("2"));
}

#[test]
fn simple_calculate_called_from_a_script_reads_the_realm_s_fields() {
    let mut realm = realm();
    let ran = calculate(
        &mut realm,
        r#"AFSimple_Calculate("SUM", ["Line.1", "Line.2"]); var listed = event.value;
           AFSimple_Calculate("SUM", "Line"); event.value = listed + "/" + event.value;"#,
    );
    assert_eq!(ran.ending, Ending::Finished, "{ran:?}");
    assert_eq!(ran.value.as_deref(), Some("5.5/5.5"), "{ran:?}");
}

#[test]
fn calculate_now_and_reset_form_are_edits() {
    let mut realm = realm();
    let ran = calculate(
        &mut realm,
        r#"this.calculateNow(); this.resetForm(["Line.1"]); this.resetForm();"#,
    );
    assert_eq!(
        ran.edits,
        vec![
            ScriptEdit::Calculate,
            ScriptEdit::Reset {
                fields: vec!["Line.1".to_owned()]
            },
            ScriptEdit::Reset { fields: Vec::new() },
        ]
    );
}

#[test]
fn the_event_names_its_site_its_target_and_its_source() {
    let mut realm = realm();
    let mut asked = request(
        ScriptSite::Field(Trigger::Calculate),
        "Total",
        r"event.value = [event.type, event.name, event.target.name, event.targetName,
           event.source.name].join();",
        Vec::new(),
    );
    asked.event.source = "Line.2".to_owned();
    let ran = realm.run(&asked);
    assert_eq!(
        ran.value.as_deref(),
        Some("Field,Calculate,Total,Total,Line.2")
    );
    for (site, field, expected) in [
        (
            ScriptSite::Annotation(AnnotationTrigger::Up),
            "Agree",
            "Field,Mouse Up,Agree",
        ),
        (
            ScriptSite::Annotation(AnnotationTrigger::Up),
            "",
            "Link,Mouse Up,",
        ),
        (
            ScriptSite::Annotation(AnnotationTrigger::PageVisible),
            "",
            "Screen,InView,",
        ),
        (ScriptSite::Page(PageTrigger::Close), "", "Page,Close,"),
        (ScriptSite::OpenAction, "", "Doc,Open,"),
        (ScriptSite::Library, "", "Doc,Open,library"),
    ] {
        let ran = realm.run(&request(
            site,
            field,
            "this.getField('Total').value = [event.type, event.name, event.targetName].join();",
            Vec::new(),
        ));
        assert_eq!(
            ran.edits,
            vec![ScriptEdit::Value {
                field: "Total".to_owned(),
                value: expected.to_owned()
            }],
            "{site:?}"
        );
    }
}

#[test]
fn the_constants_are_the_reference_s() {
    let mut realm = realm();
    let ran = calculate(
        &mut realm,
        r#"event.value = [display.visible, display.hidden, display.noPrint, display.noView,
           border.s, border.u, color.equal(color.red, ["RGB", 1, 0, 0]),
           color.equal(color.red, color.blue), color.transparent.join()].join();"#,
    );
    assert_eq!(
        ran.value.as_deref(),
        Some("0,1,2,3,solid,underline,true,false,T")
    );
}

#[test]
fn an_engine_keeps_one_realm_across_its_runs() {
    let engine = Engine::new(Budget::FIELD_EVENT);
    let defined = engine.run_request(&request(
        ScriptSite::Library,
        "",
        "var count = 0; function next() { count += 1; return count; }",
        form(),
    ));
    assert_eq!(defined.ending, Ending::Finished, "{defined:?}");
    for expected in ["1", "2", "3"] {
        let ran = engine.run_request(&request(
            ScriptSite::Field(Trigger::Calculate),
            "Total",
            "event.value = next();",
            Vec::new(),
        ));
        assert_eq!(ran.value.as_deref(), Some(expected), "{ran:?}");
    }
}
