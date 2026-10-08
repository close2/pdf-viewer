//! The members ADR 1615 bridges: `util.printx`, `util.printd`, `app`'s six properties naming the
//! viewer, and a field's `getArray`, `setFocus` and text flags.
//!
//! Where Adobe's reference prints an example, the example is the fixture and its printed result
//! the expected value (habit 39): `printx`'s telephone number, and `printd`'s three numbered
//! formats, whose examples all write 1 August 2000 at 14:56:05 seven hours east of Universal Time.
//! Every other expected value is this program's documented choice, stated in ADR 1615, or the
//! clause a refusal cites — Table 231's `Comb`.

#![cfg(feature = "engine")]
#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use pdf_model::aform::Trigger;
use pdf_model::view::{
    Alignment, BorderStyle, Display, FieldState, FieldType, Property, ScriptEdit, ScriptSite,
    TextFlag, WidgetState,
};
use pdf_script::{Budget, Ending, Event, Outcome, Realm, RefusalKind, Request};

/// A text field's state.
fn field(name: &str, kind: FieldType, flags: u32, char_limit: Option<u32>) -> FieldState {
    FieldState {
        name: name.to_owned(),
        kind,
        value: String::new(),
        flags,
        char_limit,
        page: Some(0),
        widgets: vec![WidgetState {
            display: Display::Visible,
            text_color: None,
            fill_color: None,
            stroke_color: None,
            border_style: BorderStyle::Solid,
            alignment: Alignment::Left,
            rect: [0.0, 0.0, 100.0, 20.0],
            captions: Default::default(),
            on_state: None,
        }],
    }
}

/// A calculate script on `Total`, telling the realm of `fields`, at seven hours east of UT.
fn request(script: &str, fields: Vec<FieldState>) -> Request {
    Request {
        site: ScriptSite::Field(Trigger::Calculate),
        field: "Total".to_owned(),
        label: String::new(),
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
        page: 0,
        pages: 1,
        dirty: false,
        document: None,
        moment: 1_704_465_015_000,
        utc_offset_seconds: 7 * 3600,
    }
}

/// Runs `script` in a realm told of a form: `Total`, `Code` (a text field with a `/MaxLen` of 6),
/// `Lines` (multiline), `Agree` (a check box) and `Group.a`, `Group.b`.
fn run(script: &str) -> Outcome {
    let mut realm = Realm::new(Budget::FIELD_EVENT).expect("a realm");
    realm.run(&request(
        script,
        vec![
            field("Total", FieldType::Text, 0, None),
            field("Code", FieldType::Text, 0, Some(6)),
            field("Lines", FieldType::Text, TextFlag::Multiline.bit(), Some(6)),
            field("Agree", FieldType::CheckBox, 0, None),
            field("Group.a", FieldType::Text, 0, None),
            field("Group.b", FieldType::Text, 0, None),
        ],
    ))
}

#[test]
fn printx_writes_the_reference_s_telephone_number() {
    let ran = run(r#"event.value = util.printx("9 (999) 999-9999", "aaa14159697489zzz");"#);
    assert_eq!(ran.value.as_deref(), Some("1 (415) 969-7489"), "{ran:?}");
}

#[test]
fn printd_writes_the_reference_s_three_numbered_formats_and_a_picture() {
    // ECMA-262's months count from zero: 7 is August. The reference's example for format 0 ends
    // in an apostrophe after the minutes, PDF 1.7's spelling; §7.9.4 now writes none, and its
    // NOTE 2 is what keeps the older form readable, so the standard's spelling is the one written.
    let ran = run(r#"var d = new Date(2000, 7, 1, 14, 56, 5);
           event.value = [util.printd(0, d), util.printd(1, d), util.printd(2, d),
                          util.printd("mmmm dd, yyyy", d)].join("|");"#);
    assert_eq!(
        ran.value.as_deref(),
        Some("D:20000801145605+07'00|D:20000801075605Z|2000/08/01 14:56:05|August 01, 2000"),
        "{ran:?}"
    );
}

#[test]
fn printd_refuses_an_xfa_picture_and_a_value_that_is_no_date_by_name() {
    for (script, says) in [
        (
            "util.printd('YYYY', new Date(), true);",
            "XFA picture clause",
        ),
        ("util.printd('yyyy', 5);", "is not a Date"),
        (
            "util.printd(7, new Date());",
            "numbered formats are 0, 1 and 2",
        ),
        ("util.printd('yyyy', new Date(NaN));", "not a moment"),
    ] {
        let ran = run(script);
        assert!(matches!(ran.ending, Ending::Threw(_)), "{script}: {ran:?}");
        let refusal = ran.refusals.first().expect("one refusal");
        assert_eq!(refusal.member, "util.printd");
        assert!(
            matches!(&refusal.kind, RefusalKind::Library(why) if why.contains(says)),
            "{script}: {refusal:?}"
        );
    }
}

#[test]
fn app_names_this_program_and_refuses_a_write() {
    let ran = run(
        "event.value = [app.viewerType, app.viewerVariation, app.viewerVersion, \
         app.formsVersion, app.platform, app.language, typeof app.viewerVersion].join();",
    );
    let version = pdf_script::viewer::version();
    assert_eq!(
        ran.value,
        Some(format!(
            "quorra,quorra,{version},{version},{},ENU,number",
            pdf_script::viewer::platform()
        )),
        "{ran:?}"
    );
    let ran = run("app.viewerVersion = 9;");
    assert_eq!(
        ran.refusals.first().map(|refusal| refusal.member.as_str()),
        Some("app.viewerVersion=")
    );
}

#[test]
fn get_array_answers_the_terminal_fields_below_a_name() {
    let ran = run(
        "event.value = this.getField('Group').getArray().map(function (f) { return f.name; }) \
         .join() + '/' + this.getField('Code').getArray().length;",
    );
    assert_eq!(ran.value.as_deref(), Some("Group.a,Group.b/1"), "{ran:?}");
}

#[test]
fn set_focus_is_an_edit_the_host_carries_out() {
    let ran = run("this.getField('Group').setFocus();");
    assert_eq!(ran.ending, Ending::Finished, "{ran:?}");
    assert_eq!(
        ran.edits,
        vec![ScriptEdit::Focus {
            field: "Group.a".to_owned(),
            widget: None,
        }]
    );
}

#[test]
fn comb_is_set_where_table_231_lets_it_be_and_sets_do_not_scroll_beside_it() {
    let ran = run(
        "var f = this.getField('Code'); f.comb = true; event.value = [f.comb, f.doNotScroll].join();",
    );
    assert_eq!(ran.value.as_deref(), Some("true,true"), "{ran:?}");
    assert_eq!(
        ran.edits,
        vec![
            ScriptEdit::Property {
                field: "Code".to_owned(),
                widget: None,
                property: Property::TextFlag(TextFlag::Comb, true),
            },
            ScriptEdit::Property {
                field: "Code".to_owned(),
                widget: None,
                property: Property::TextFlag(TextFlag::DoNotScroll, true),
            },
        ]
    );
    for (name, says) in [
        (
            "Lines",
            "multiline, password and file-select flags are clear",
        ),
        ("Total", "/MaxLen"),
        ("Agree", "is a checkbox field"),
    ] {
        let ran = run(&format!("this.getField('{name}').comb = true;"));
        let refusal = ran.refusals.first().expect("one refusal");
        assert_eq!(refusal.member, "Field.comb=", "{name}");
        assert!(
            matches!(&refusal.kind, RefusalKind::Unreachable(why) if why.contains(says)),
            "{name}: {refusal:?}"
        );
        assert!(
            ran.edits.is_empty(),
            "{name}: a refused write changes no field"
        );
    }
    let ran = run("this.getField('Lines').multiline = false; this.getField('Lines').comb = true;");
    assert_eq!(ran.ending, Ending::Finished, "{ran:?}");
}
