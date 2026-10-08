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
    Alignment, BorderStyle, Colour, Display, FieldState, FieldType, Glyph, Property, ScriptEdit,
    ScriptSite, WidgetState,
};
use pdf_script::{Budget, Ending, Engine, Event, Outcome, Realm, RefusalKind, Request};

/// A text field's state.
fn field(name: &str, value: &str) -> FieldState {
    FieldState {
        name: name.to_owned(),
        kind: FieldType::Text,
        value: value.to_owned(),
        flags: 0,
        char_limit: None,
        page: Some(0),
        widgets: vec![WidgetState {
            display: Display::Visible,
            text_color: None,
            fill_color: None,
            stroke_color: None,
            border_style: BorderStyle::Solid,
            alignment: Alignment::Left,
            rect: [10.0, 10.0, 210.0, 40.0],
            captions: Default::default(),
            on_state: None,
        }],
        options: Vec::new(),
        selected: Vec::new(),
    }
}

/// The one widget of a field of [`field`].
fn widget() -> WidgetState {
    field("", "").widgets.remove(0)
}

/// The form every test here tells its realm of.
fn form() -> Vec<FieldState> {
    vec![
        field("Line.1", "2"),
        field("Line.2", "3.5"),
        FieldState {
            flags: 1 << 1,
            char_limit: Some(6),
            page: Some(2),
            widgets: vec![WidgetState {
                alignment: Alignment::Right,
                border_style: BorderStyle::Beveled,
                text_color: Some(Colour::Rgb([0.0, 0.0, 1.0])),
                ..widget()
            }],
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
        widget: None,
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
                widget: None,
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

/// Adobe's "Doc properties" page: `pageNum` is the current page, zero-based, read and written, with
/// `this.pageNum = 0` and `this.pageNum++` as its two examples. The requests here are on page 1 of
/// four; a value that names no page is this tree's documented choice (ADR 1640).
#[test]
fn a_write_to_page_num_is_a_page_turn_the_script_reads_back() {
    let mut realm = realm();
    let first = calculate(&mut realm, "this.pageNum = 0; event.value = this.pageNum;");
    assert_eq!(first.ending, Ending::Finished, "{first:?}");
    assert_eq!(first.edits, vec![ScriptEdit::GoTo { page: 0 }]);
    assert_eq!(first.value.as_deref(), Some("0"));
    assert!(first.refusals.is_empty(), "{first:?}");

    let next = calculate(&mut realm, "this.pageNum++; event.value = this.pageNum;");
    assert_eq!(next.edits, vec![ScriptEdit::GoTo { page: 2 }]);
    assert_eq!(next.value.as_deref(), Some("2"));

    let twice = calculate(&mut realm, "this.pageNum = 3; this.pageNum = '1.7';");
    assert_eq!(
        twice.edits,
        vec![ScriptEdit::GoTo { page: 1 }],
        "the latest turn of a run is carried, its number truncated"
    );

    let nowhere = calculate(
        &mut realm,
        "this.pageNum = 4; this.pageNum = -1; this.pageNum = 'x'; event.value = this.pageNum;",
    );
    assert_eq!(nowhere.ending, Ending::Finished, "{nowhere:?}");
    assert!(nowhere.edits.is_empty(), "{nowhere:?}");
    assert_eq!(nowhere.value.as_deref(), Some("1"), "the page stays");
    assert_eq!(
        nowhere
            .notes
            .iter()
            .filter(|note| note.contains("names no page of this document's 4"))
            .count(),
        3,
        "{nowhere:?}"
    );

    let thrown = calculate(&mut realm, "this.pageNum = 2; throw new Error('late');");
    assert!(
        thrown.edits.is_empty(),
        "a run that throws turns nothing: {thrown:?}"
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
fn the_glyph_styles_and_the_pointer_behaviours_are_the_reference_s_tables() {
    // The "Field properties" page's six keywords, each the style's own name in its table, and the
    // "FullScreen properties" page's three, numbered in its table's order (ADR 1652).
    let mut realm = realm();
    let ran = calculate(
        &mut realm,
        r"event.value = [style.ch, style.cr, style.di, style.ci, style.st, style.sq,
           cursor.hidden, cursor.delay, cursor.visible].join();",
    );
    assert_eq!(
        ran.value.as_deref(),
        Some("check,cross,diamond,circle,star,square,0,1,2"),
        "{ran:?}"
    );
}

#[test]
fn a_style_is_the_glyph_code_of_the_normal_caption() {
    // The "Field properties" page's `style`, read and written as Table 192's `/CA` holding the
    // ZapfDingbats code of the glyph (ADR 1665): `Agree` states no caption, so it reads as no
    // style; a text field has no glyph to set.
    let mut realm = realm();
    let ran = calculate(
        &mut realm,
        r#"var box = this.getField("Agree"), before = typeof box.style;
           box.style = style.cr;
           var refused = "";
           try { this.getField("Total").style = style.ch; } catch (e) { refused = e.name; }
           event.value = [before, box.style, box.buttonGetCaption(), refused].join();"#,
    );
    assert_eq!(
        ran.value.as_deref(),
        Some("undefined,cross,8,NotAllowedError"),
        "{ran:?}"
    );
    assert_eq!(
        ran.edits,
        vec![ScriptEdit::Property {
            field: "Agree".to_owned(),
            widget: None,
            property: Property::Style(Glyph::Cross),
        }]
    );
}

#[test]
fn the_full_screen_preferences_are_refused_by_name() {
    // `app.fs` is the application's full-screen preferences, which RFC 0008 section 4.2 does not
    // admit: a script that sets its cursor meets a `NotAllowedError` naming it (ADR 1665).
    let mut realm = realm();
    let ran = calculate(
        &mut realm,
        r"try { app.fs.cursor = cursor.hidden; } catch (e) { event.value = e.name; }",
    );
    assert_eq!(ran.value.as_deref(), Some("NotAllowedError"), "{ran:?}");
    assert_eq!(ran.refusals.len(), 1, "{ran:?}");
    assert_eq!(ran.refusals[0].member, "app.fs");
    assert!(
        matches!(
            &ran.refusals[0].kind,
            RefusalKind::Excluded(reason) if reason.contains("/PageMode")
        ),
        "{ran:?}"
    );
}

#[test]
fn an_exact_match_is_of_the_whole_string_and_answers_a_position_from_one() {
    // ADR 1652: the first pattern whose first match is the whole string, counted from one; 0 for
    // none; one pattern is a list of one.
    let mut realm = realm();
    let ran = calculate(
        &mut realm,
        r#"var zip = [/\d{5}/, /\d{5}-\d{4}/];
           event.value = [AFExactMatch(zip, "12345"), AFExactMatch(zip, "12345-6789"),
           AFExactMatch(zip, "123456"), AFExactMatch(/\d+/, "42"), AFExactMatch(/\d+/, "4x"),
           AFExactMatch(/a/g, "aa"), AFExactMatch([], "")].join();"#,
    );
    assert_eq!(ran.ending, Ending::Finished, "{ran:?}");
    assert_eq!(ran.value.as_deref(), Some("1,2,0,1,0,0,0"), "{ran:?}");
}

#[test]
fn a_name_that_matches_no_field_is_read_again_without_its_spaces_and_trailing_periods() {
    // ADR 1652: the exact name first; only a name that matches nothing is cut, and the cut is said.
    let mut realm = realm();
    let ran = calculate(
        &mut realm,
        r#"event.value = [this.getField("Total   ").name, this.getField(" Total").name,
           this.getField("Total.").name, this.getField("Line.1").value,
           this.getField("Line . ").name, this.getField("None. ") === null,
           this.getField("   ") === null].join();"#,
    );
    assert_eq!(ran.ending, Ending::Finished, "{ran:?}");
    assert_eq!(
        ran.value.as_deref(),
        Some("Total,Total,Total,2,Line,true,true")
    );
    assert!(
        ran.notes.iter().any(|note| note.contains(r#""Total   ""#)
            && note.contains(r#"given "Total""#)
            && note.contains("ADR 1652")),
        "{:?}",
        ran.notes
    );
    let exact = calculate(
        &mut realm,
        r#"event.value = this.getField("Line.1").value;"#,
    );
    assert!(exact.notes.is_empty(), "{:?}", exact.notes);
}

/// A radio button field with two widgets, the second drawn elsewhere, hidden and filled grey.
fn two_widgets() -> FieldState {
    FieldState {
        kind: FieldType::RadioButton,
        widgets: vec![
            widget(),
            WidgetState {
                display: Display::Hidden,
                fill_color: Some(Colour::Gray(0.5)),
                rect: [300.0, 10.0, 320.0, 30.0],
                ..widget()
            },
        ],
        ..field("Choice", "Off")
    }
}

#[test]
fn one_widget_of_a_field_answers_its_own_widget_members_and_the_field_s_value() {
    // The reference's "Field" page: `getField("name.N")` is the Nth widget from zero; a widget
    // member reads and writes that widget, a field member the field (ADR 1664).
    let mut realm = Realm::new(Budget::FIELD_EVENT).expect("a realm");
    let mut fields = form();
    fields.push(two_widgets());
    let told = realm.run(&request(ScriptSite::Library, "", "", fields));
    assert_eq!(told.ending, Ending::Finished, "{told:?}");
    let ran = calculate(
        &mut realm,
        r#"var all = this.getField("Choice"), second = this.getField("Choice.1");
           var read = [second.name, second.rect.join(), second.display, second.fillColor.join(),
                       all.rect.join(), all.display, second === this.getField("Choice.1"),
                       this.getField("Choice.2") === null, this.getField("Total.0").name];
           second.strokeColor = color.red;
           second.readonly = true;
           second.value = "B";
           read.push(this.getField("Choice.0").strokeColor.join(), second.strokeColor.join(),
                     all.readonly, all.value);
           event.value = read.join(" ");"#,
    );
    assert_eq!(ran.ending, Ending::Finished, "{ran:?}");
    assert_eq!(
        ran.value.as_deref(),
        Some(
            "Choice 300,30,320,10 1 G,0.5 10,40,210,10 0 true true Total \
             T RGB,1,0,0 true B"
        ),
        "{ran:?}"
    );
    assert_eq!(
        ran.edits,
        vec![
            ScriptEdit::Property {
                field: "Choice".to_owned(),
                widget: Some(1),
                property: Property::StrokeColor(Colour::Rgb([1.0, 0.0, 0.0])),
            },
            ScriptEdit::Property {
                field: "Choice".to_owned(),
                widget: None,
                property: Property::ReadOnly(true),
            },
            ScriptEdit::Value {
                field: "Choice".to_owned(),
                value: "B".to_owned(),
            },
        ]
    );
}

#[test]
fn set_focus_through_one_widget_asks_for_that_widget() {
    // The reference makes `setFocus` a widget's method: a `Field` of the second widget asks for
    // the second, and a `Field` of every widget for the first (ADR 1688).
    let mut realm = Realm::new(Budget::FIELD_EVENT).expect("a realm");
    let mut fields = form();
    fields.push(two_widgets());
    let told = realm.run(&request(ScriptSite::Library, "", "", fields));
    assert_eq!(told.ending, Ending::Finished, "{told:?}");
    let ran = calculate(
        &mut realm,
        r#"this.getField("Choice.1").setFocus(); this.getField("Choice").setFocus();"#,
    );
    assert_eq!(ran.ending, Ending::Finished, "{ran:?}");
    assert!(ran.refusals.is_empty(), "{ran:?}");
    assert_eq!(
        ran.edits,
        vec![
            ScriptEdit::Focus {
                field: "Choice".to_owned(),
                widget: Some(1),
            },
            ScriptEdit::Focus {
                field: "Choice".to_owned(),
                widget: None,
            },
        ]
    );
}

#[test]
fn a_box_is_checked_where_the_value_names_its_on_state_and_checking_it_sets_that_value() {
    // The reference's "Field methods": `isBoxChecked(nWidget)` and `checkThisBox(nWidget,
    // bCheckIt)`, a widget counted from zero; a radio button is not unchecked this way (ADR 1689).
    let mut realm = Realm::new(Budget::FIELD_EVENT).expect("a realm");
    let on = |name: &str| WidgetState {
        on_state: Some(name.to_owned()),
        ..widget()
    };
    let mut fields = form();
    fields.push(FieldState {
        kind: FieldType::RadioButton,
        widgets: vec![on("A"), on("B")],
        ..field("Choice", "B")
    });
    fields.push(FieldState {
        kind: FieldType::CheckBox,
        widgets: vec![on("Yes")],
        ..field("Box", "Off")
    });
    let told = realm.run(&request(ScriptSite::Library, "", "", fields));
    assert_eq!(told.ending, Ending::Finished, "{told:?}");
    let ran = calculate(
        &mut realm,
        r#"var b = this.getField("Box"), c = this.getField("Choice");
           var read = [b.isBoxChecked(0), c.isBoxChecked(0), c.isBoxChecked(1), c.isBoxChecked(5)];
           b.checkThisBox(0); c.checkThisBox(0, true); c.checkThisBox(0, false);
           read.push(b.isBoxChecked(0), b.value, c.value, c.isBoxChecked(0), c.isBoxChecked(1));
           b.checkThisBox(0, false);
           read.push(b.value);
           event.value = read.join(" ");"#,
    );
    assert_eq!(ran.ending, Ending::Finished, "{ran:?}");
    assert_eq!(
        ran.value.as_deref(),
        Some("false false true false true Yes A true false Off"),
        "{ran:?}"
    );
    let value = |field: &str, value: &str| ScriptEdit::Value {
        field: field.to_owned(),
        value: value.to_owned(),
    };
    assert_eq!(
        ran.edits,
        vec![
            value("Box", "Yes"),
            value("Choice", "A"),
            value("Box", "Off")
        ]
    );
    let text = calculate(&mut realm, r#"this.getField("Line.1").checkThisBox(0);"#);
    assert!(matches!(text.ending, Ending::Threw(_)), "{text:?}");
    assert_eq!(
        text.refusals.first().map(|refusal| refusal.member.as_str()),
        Some("Field.checkThisBox")
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
