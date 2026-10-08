//! The members ADRs 1626 and 1627 bridge: `global`, `event.commitKey`, `fieldFull` and `changeEx`,
//! `this.dirty`, `this.info`, `this.getOCGs` and the `OCG` object, `util.printf`, a button's
//! captions, `app.alert` and `app.response`, and the depth a script may nest before it is parsed.
//!
//! Where Adobe's reference prints an example, the example is the fixture and its printed result
//! the expected value (habit 39): `util.printf`'s one hundred π, `global`'s sphere of radius 8, and
//! `this.dirty`'s saved-and-restored mark. Every other expected value is this program's documented
//! choice, stated in ADR 1626 or 1627, or the clause a refusal cites — Table 192's captions,
//! Table 99's `/Locked`.

#![cfg(feature = "engine")]
#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::rc::Rc;

use pdf_model::aform::Trigger;
use pdf_model::view::{
    Alignment, BorderStyle, Display, DocumentState, Face, FieldState, FieldType, InfoEntry, Layer,
    Property, ScriptEdit, ScriptSite, WidgetState,
};
use pdf_script::{
    Answer, Asker, Budget, Button, Buttons, Ending, Event, Exceeded, Icon, Outcome, Question,
    Realm, RefusalKind, Request,
};

/// A field's state.
fn field(name: &str, kind: FieldType) -> FieldState {
    FieldState {
        name: name.to_owned(),
        kind,
        value: String::new(),
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
            rect: [0.0, 0.0, 100.0, 20.0],
            captions: ["Send".to_owned(), String::new(), String::new()],
            on_state: None,
        }],
        options: Vec::new(),
        selected: Vec::new(),
    }
}

/// The document as a whole: a title, a creation date, and two groups, one of them locked.
fn document() -> DocumentState {
    DocumentState {
        info: vec![
            InfoEntry {
                key: "Title".to_owned(),
                text: "Form".to_owned(),
                moment: None,
            },
            InfoEntry {
                key: "CreationDate".to_owned(),
                text: "D:20000612145409Z".to_owned(),
                moment: Some(960_821_649_000),
            },
            InfoEntry {
                key: "eec".to_owned(),
                text: "custom".to_owned(),
                moment: None,
            },
        ],
        layers: vec![
            Layer {
                number: 9,
                generation: 0,
                name: "Watermark".to_owned(),
                on: true,
                initially_on: true,
                locked: false,
            },
            Layer {
                number: 4,
                generation: 0,
                name: "English".to_owned(),
                on: false,
                initially_on: false,
                locked: true,
            },
        ],
        annotations: Vec::new(),
        pages: Vec::new(),
    }
}

/// A request at `site` on `Total`, the form and the document told.
fn request(site: ScriptSite, script: &str) -> Request {
    Request {
        site,
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
        fields: vec![
            field("Total", FieldType::Text),
            field("Send", FieldType::PushButton),
            field("Agree", FieldType::CheckBox),
        ],
        page: 0,
        pages: 1,
        dirty: true,
        document: Some(document()),
        moment: 1_704_465_015_000,
        utc_offset_seconds: 0,
    }
}

/// Runs a calculate script in a realm of its own.
fn run(script: &str) -> Outcome {
    let mut realm = Realm::new(Budget::FIELD_EVENT).expect("a realm");
    realm.run(&request(ScriptSite::Field(Trigger::Calculate), script))
}

/// The refusal a run met, by member.
fn refused(ran: &Outcome) -> Vec<&str> {
    ran.refusals
        .iter()
        .map(|refusal| refusal.member.as_str())
        .collect()
}

#[test]
fn printf_writes_the_reference_s_four_conversions_of_one_hundred_pi() {
    let ran = run(r#"var n = Math.PI * 100;
        event.value = [util.printf("Decimal format: %d", n), util.printf("Hex format: %x", n),
                       util.printf("Float format: %.2f", n), util.printf("String format: %s", n)]
                      .join("|");"#);
    assert_eq!(
        ran.value.as_deref(),
        Some(
            "Decimal format: 314|Hex format: 13A|Float format: 314.16|String format: 314.159265358979"
        ),
        "{ran:?}"
    );
    let ran = run(r#"util.printf("%d %d", 1);"#);
    assert_eq!(refused(&ran), ["util.printf"]);
}

#[test]
fn global_holds_a_value_across_runs_of_one_realm_and_no_further() {
    // The reference's sphere: `global.radius = 8` in one script, the volume in another.
    let mut realm = Realm::new(Budget::FIELD_EVENT).expect("a realm");
    let set = realm.run(&request(ScriptSite::Library, "global.radius = 8;"));
    assert_eq!(set.ending, Ending::Finished, "{set:?}");
    let read = realm.run(&request(
        ScriptSite::Field(Trigger::Calculate),
        "var V = (4/3) * Math.PI * Math.pow(global.radius, 3); event.value = util.printf('%.5f', V);",
    ));
    assert_eq!(read.value.as_deref(), Some("2144.66058"), "{read:?}");
    let fresh = run("event.value = String(global.radius);");
    assert_eq!(
        fresh.value.as_deref(),
        Some("undefined"),
        "another realm holds none"
    );
    let ran = run(r#"global.setPersistent("radius", true);"#);
    assert_eq!(refused(&ran), ["global.setPersistent"]);
    assert!(
        matches!(&ran.refusals[0].kind, RefusalKind::Excluded(why) if why.contains("between files"))
    );
}

#[test]
fn the_event_carries_a_commit_s_key_and_a_full_field_s_two_changes() {
    let mut asked = request(
        ScriptSite::Field(Trigger::Keystroke),
        "event.value = [event.commitKey, event.fieldFull, event.change, event.changeEx].join();",
    );
    asked.event.commit_key = 3;
    asked.event.field_full = true;
    asked.event.change = "abc".to_owned();
    asked.event.change_ex = "abcdef".to_owned();
    let ran = Realm::new(Budget::FIELD_EVENT)
        .expect("a realm")
        .run(&asked);
    assert_eq!(ran.value.as_deref(), Some("3,true,abc,abcdef"), "{ran:?}");
}

#[test]
fn dirty_reads_the_view_s_mark_and_a_write_is_read_back_for_the_run() {
    // The reference's second example: the mark saved, a field written, the mark put back.
    let ran = run(
        "var b = this.dirty; this.getField('Total').value = 'Please fill in the fields below.'; \
         this.dirty = false; var cleared = this.dirty; this.dirty = b; \
         event.value = [b, cleared, this.dirty].join();",
    );
    assert_eq!(ran.value.as_deref(), Some("true,false,true"), "{ran:?}");
    assert!(
        ran.notes
            .iter()
            .any(|note| note.contains("this.dirty") && note.contains("ADR 1626")),
        "{:?}",
        ran.notes
    );
}

#[test]
fn info_is_the_information_dictionary_read_only() {
    let ran = run(
        "var i = this.info; event.value = [i.Title, i.title, i.eec, i.CreationDate instanceof Date, \
         i.CreationDate.getTime()].join();",
    );
    assert_eq!(
        ran.value.as_deref(),
        Some("Form,Form,custom,true,960821649000"),
        "{ran:?}"
    );
    let ran = run("this.info.Title = 'Another';");
    assert_eq!(refused(&ran), ["this.info.Title="]);
}

#[test]
fn get_ocgs_answers_the_groups_by_name_and_a_switch_is_a_layer_edit() {
    let ran = run(
        "var g = this.getOCGs(); event.value = g.map(function (o) { return o.name + ':' + o.state \
         + ':' + o.locked; }).join(); g[1].state = false;",
    );
    assert_eq!(
        ran.value.as_deref(),
        Some("English:false:true,Watermark:true:false"),
        "{ran:?}"
    );
    assert_eq!(
        ran.edits,
        vec![ScriptEdit::Layer {
            number: 9,
            generation: 0,
            on: false
        }]
    );
    // 6204065.pdf's library sets `initState` too, which a reader does not write.
    let ran = run("var g = this.getOCGs(); g[0].state = true; g[0].initState = false;");
    assert_eq!(refused(&ran), ["OCG.initState="]);
    assert!(ran.edits.is_empty(), "a run that threw changes nothing");
    let ran = run("this.getOCGs(0);");
    assert_eq!(refused(&ran), ["this.getOCGs(nPage)"]);
}

#[test]
fn a_caption_is_table_192_s_entry_and_a_text_field_has_none() {
    let ran = run(
        "var b = this.getField('Send'); var was = b.buttonGetCaption(); \
         b.buttonSetCaption('Formular drucken'); b.buttonSetCaption('=> drucken <=', 2); \
         event.value = [was, b.buttonGetCaption(), b.buttonGetCaption(2)].join('|');",
    );
    assert_eq!(
        ran.value.as_deref(),
        Some("Send|Formular drucken|=> drucken <="),
        "{ran:?}"
    );
    assert_eq!(
        ran.edits,
        vec![
            ScriptEdit::Property {
                field: "Send".to_owned(),
                widget: None,
                property: Property::Caption(Face::Normal, "Formular drucken".to_owned()),
            },
            ScriptEdit::Property {
                field: "Send".to_owned(),
                widget: None,
                property: Property::Caption(Face::Rollover, "=> drucken <=".to_owned()),
            },
        ]
    );
    for (script, member) in [
        (
            "this.getField('Total').buttonSetCaption('x');",
            "Field.buttonSetCaption",
        ),
        (
            "this.getField('Agree').buttonSetCaption('x', 1);",
            "Field.buttonSetCaption",
        ),
        (
            "this.getField('Send').buttonSetCaption('x', 7);",
            "Field.buttonSetCaption",
        ),
    ] {
        assert_eq!(refused(&run(script)), [member], "{script}");
    }
}

/// An asker that answers every question with one answer, and remembers what it was asked.
#[derive(Debug)]
struct Answering {
    /// What it answers.
    answer: Answer,
    /// What it was asked.
    asked: std::cell::RefCell<Vec<Question>>,
}

impl Asker for Answering {
    fn ask(&self, question: &Question) -> Answer {
        self.asked.borrow_mut().push(question.clone());
        self.answer.clone()
    }
}

/// Runs `script` in a realm whose questions `answer` answers, and says what was asked.
fn asking(script: &str, answer: Answer) -> (Outcome, Vec<Question>) {
    let asker = Rc::new(Answering {
        answer,
        asked: std::cell::RefCell::new(Vec::new()),
    });
    let mut realm = Realm::with_asker(Budget::FIELD_EVENT, asker.clone()).expect("a realm");
    let ran = realm.run(&request(ScriptSite::Field(Trigger::Calculate), script));
    let questions = asker.asked.borrow().clone();
    (ran, questions)
}

#[test]
fn an_alert_is_put_once_per_run_and_answers_the_button_pressed() {
    // The reference's second example: a question with Yes and No, Yes answered as 4.
    let (ran, asked) = asking(
        r#"var n = app.alert({cMsg: "Do you want to close this document?",
                              cTitle: "A message from A. C. Robat", nIcon: 2, nType: 2});
           var m = app.alert("Again?", 1, 1);
           event.value = n + "," + m;"#,
        Answer::Pressed(Button::Yes),
    );
    assert_eq!(ran.value.as_deref(), Some("4,2"), "{ran:?}");
    assert_eq!(
        asked,
        vec![Question::Alert {
            message: "Do you want to close this document?".to_owned(),
            icon: Icon::Question,
            buttons: Buttons::YesNo,
            title: Some("A message from A. C. Robat".to_owned()),
        }],
        "the second alert of the run is not put"
    );
    assert!(
        ran.notes
            .iter()
            .any(|note| note.contains("1 further question")),
        "{:?}",
        ran.notes
    );
    // A button the set does not offer is not believed.
    let (ran, _) = asking(
        "event.value = app.alert('x');",
        Answer::Pressed(Button::Yes),
    );
    assert_eq!(ran.value.as_deref(), Some("1"), "{ran:?}");
}

#[test]
fn a_question_nobody_can_answer_is_answered_as_a_closed_dialogue_and_said() {
    let ran = run("event.value = app.alert('Sure?', 2, 3) + ',' + app.response('Name?');");
    assert_eq!(ran.value.as_deref(), Some("2,null"), "{ran:?}");
    assert!(
        ran.notes
            .iter()
            .any(|note| note.contains("could not be put")),
        "{:?}",
        ran.notes
    );
}

#[test]
fn a_response_answers_the_text_or_null_and_a_password_is_never_said_back() {
    let (ran, asked) = asking(
        r#"var r = app.response({cQuestion: "How are you today?", cTitle: "Your Health Status",
                                 cDefault: "Fine", cLabel: "Response:"});
           event.value = String(r);"#,
        Answer::Typed(Some("Fine".to_owned())),
    );
    assert_eq!(ran.value.as_deref(), Some("Fine"), "{ran:?}");
    assert_eq!(
        asked,
        vec![Question::Response {
            question: "How are you today?".to_owned(),
            title: Some("Your Health Status".to_owned()),
            default: "Fine".to_owned(),
            label: Some("Response:".to_owned()),
            password: false,
        }]
    );
    let (ran, _) = asking(
        "event.value = String(app.response('PIN?', 'Bank', '', true));",
        Answer::Typed(Some("1234".to_owned())),
    );
    assert_eq!(ran.value.as_deref(), Some("1234"));
    assert!(
        ran.notes.iter().all(|note| !note.contains("1234")),
        "{:?}",
        ran.notes
    );
    let (ran, _) = asking(
        "event.value = String(app.response('?'));",
        Answer::Typed(None),
    );
    assert_eq!(ran.value.as_deref(), Some("null"));
}

/// Runs `script` on a thread with the confined worker's 8 MiB of stack.
fn on_the_worker_s_stack(script: String) -> Outcome {
    std::thread::Builder::new()
        .stack_size(8 << 20)
        .spawn(move || run(&script))
        .expect("a thread")
        .join()
        .expect("the run returns")
}

#[test]
fn an_expression_too_deep_to_parse_is_stopped_by_name_before_the_parser_sees_it() {
    // A chain of 20 000 terms overflowed the worker's 8 MiB at about 11 000 (ADR 1626).
    for deep in [
        format!("event.value = 1{};", "+1".repeat(20_000)),
        format!("event.value = {}1;", "!".repeat(3_000)),
        format!("event.value = {}1;", "x=>".repeat(2_000)),
        format!("if (1) {{}}{}", " else if (1) {}".repeat(3_000)),
    ] {
        let ran = on_the_worker_s_stack(deep);
        assert!(
            matches!(ran.ending, Ending::Exceeded(Exceeded::Depth { .. })),
            "{:?}",
            ran.ending
        );
        assert!(ran.edits.is_empty() && ran.value.is_none());
    }
    // A string compiled at run time is held to the same budget.
    let ran = on_the_worker_s_stack("event.value = eval('1' + '+1'.repeat(20000));".to_owned());
    assert!(
        matches!(ran.ending, Ending::Exceeded(Exceeded::Depth { .. })),
        "{:?}",
        ran.ending
    );
    // And what a form's script nests runs.
    let ran = on_the_worker_s_stack(format!("event.value = 1{};", "+1".repeat(1_000)));
    assert_eq!(ran.value.as_deref(), Some("1001"), "{ran:?}");
}
