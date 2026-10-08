//! One field's script run through the engine: what runs, what is refused by name, and what a budget
//! stops (ADRs 1590, 1591 and 1603).
//!
//! Every expected value is either the event the script itself sets — `event.rc = false` is false —
//! or `pdf_model::aform`'s own output for the same call, asked directly: the bridge's claim is
//! that a format called from a script writes what Tier 0 writes, so the test compares the two
//! rather than hand-writing either.

#![cfg(feature = "engine")]
#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::time::Instant;

use pdf_model::aform::{Call, Trigger};
use pdf_model::view::{
    Alignment, BorderStyle, Display, FieldState, FieldType, ScriptSite, WidgetState,
};
use pdf_script::{Budget, Ending, Event, Exceeded, Outcome, RefusalKind, Request, run};

/// A text field's state as a view state tells a realm of it.
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

/// A request for `script` at `trigger` on the field `Amount`, holding `value`.
fn request(
    trigger: Trigger,
    script: &str,
    value: &str,
    change: &str,
    will_commit: bool,
) -> Request {
    let at = u32::try_from(value.encode_utf16().count()).expect("a short value");
    Request {
        site: ScriptSite::Field(trigger),
        field: "Amount".to_owned(),
        label: String::new(),
        script: script.to_owned(),
        event: Event {
            value: value.to_owned(),
            change: change.to_owned(),
            selection_start: at,
            selection_end: at,
            will_commit,
            commit_key: 0,
            field_full: false,
            change_ex: String::new(),
            source: String::new(),
        },
        fields: vec![field("Amount", value)],
        page: 0,
        pages: 1,
        dirty: false,
        document: None,
        moment: 1_704_465_015_000,
        utc_offset_seconds: 0,
    }
}

/// Runs a request under the field-event budget.
fn outcome(request: &Request) -> Outcome {
    run(request, &Budget::FIELD_EVENT)
}

#[test]
fn a_keystroke_script_that_sets_rc_false_rejects_the_character() {
    let ran = outcome(&request(
        Trigger::Keystroke,
        "if (!/^[0-9]*$/.test(event.change)) event.rc = false;",
        "12",
        "x",
        false,
    ));
    assert_eq!(ran.ending, Ending::Finished);
    assert!(!ran.rc, "a letter is rejected: {ran:?}");
    let ran = outcome(&request(
        Trigger::Keystroke,
        "if (!/^[0-9]*$/.test(event.change)) event.rc = false;",
        "12",
        "3",
        false,
    ));
    assert!(ran.rc, "a digit stands: {ran:?}");
}

#[test]
fn a_keystroke_script_may_rewrite_the_change() {
    let ran = outcome(&request(
        Trigger::Keystroke,
        "event.change = event.change.toUpperCase();",
        "",
        "ab",
        false,
    ));
    assert_eq!(ran.change.as_deref(), Some("AB"));
    assert!(ran.rc);
}

#[test]
fn a_format_script_formats_through_af_number_format_as_tier_0_does() {
    let tier_0 = Call::parse(r#"AFNumber_Format(2, 0, 0, 0, "$", true)"#)
        .expect("one call")
        .format("1234.5")
        .expect("formats")
        .text;
    let ran = outcome(&request(
        Trigger::Format,
        r#"if (event.value !== "") AFNumber_Format(2, 0, 0, 0, "$", true);"#,
        "1234.5",
        "",
        false,
    ));
    assert_eq!(ran.ending, Ending::Finished, "{ran:?}");
    assert_eq!(ran.value.as_deref(), Some(tier_0.as_str()));
    assert!(ran.refusals.is_empty(), "{:?}", ran.refusals);
}

#[test]
fn a_keystroke_script_calls_the_library_s_keystroke_function() {
    let ran = outcome(&request(
        Trigger::Keystroke,
        "AFNumber_Keystroke(2, 0, 0, 0, \"\", true);",
        "12",
        "q",
        false,
    ));
    assert!(!ran.rc, "AFNumber_Keystroke rejects a letter: {ran:?}");
}

#[test]
fn a_runaway_loop_is_stopped_by_name_and_the_process_lives() {
    let started = Instant::now();
    let ran = outcome(&request(Trigger::Format, "while (true) {}", "1", "", false));
    let took = started.elapsed();
    match ran.ending {
        Ending::Exceeded(Exceeded::Wall(_) | Exceeded::Steps(_) | Exceeded::LoopIterations(_)) => {}
        other => panic!("a runaway loop ends over a budget, not {other:?}"),
    }
    assert_eq!(ran.value, None, "a run that did not finish changes nothing");
    assert!(ran.rc);
    println!("while (true) {{}}: {:?} after {took:?}", ran.ending);
    // A second run in the same process: the first left nothing running.
    let after = outcome(&request(
        Trigger::Format,
        "event.value = 'ok';",
        "1",
        "",
        false,
    ));
    assert_eq!(after.value.as_deref(), Some("ok"));
}

#[test]
fn a_loop_spread_across_calls_is_stopped_by_the_step_budget() {
    // Each call frame's loop stays under the loop-iteration limit, so only the steps count.
    let script = "function g() { for (var i = 0; i < 99999; i++) {} } \
                  for (var k = 0; k < 100000; k++) g();";
    let unhurried = Budget {
        wall: std::time::Duration::from_mins(10),
        ..Budget::FIELD_EVENT
    };
    let started = Instant::now();
    let ran = run(
        &request(Trigger::Format, script, "1", "", false),
        &unhurried,
    );
    let took = started.elapsed();
    assert_eq!(
        ran.ending,
        Ending::Exceeded(Exceeded::Steps(Budget::FIELD_EVENT.steps))
    );
    println!(
        "{} engine steps spent in {took:?}",
        Budget::FIELD_EVENT.steps
    );
    let ran = outcome(&request(Trigger::Format, script, "1", "", false));
    assert!(
        matches!(
            ran.ending,
            Ending::Exceeded(Exceeded::Steps(_) | Exceeded::Wall(_))
        ),
        "{:?}",
        ran.ending
    );
}

#[test]
fn a_loop_that_catches_everything_is_still_stopped() {
    let ran = outcome(&request(
        Trigger::Format,
        "for (;;) { try { new Array(5000000).fill(0); } catch (e) {} }",
        "1",
        "",
        false,
    ));
    assert!(
        matches!(ran.ending, Ending::Exceeded(Exceeded::Elements { .. })),
        "{:?}",
        ran.ending
    );
}

#[test]
fn a_five_million_element_array_is_stopped_by_name_and_the_process_lives() {
    let started = Instant::now();
    let ran = outcome(&request(
        Trigger::Format,
        "var a = new Array(5000000).fill(0); event.value = String(a.length);",
        "1",
        "",
        false,
    ));
    let took = started.elapsed();
    assert_eq!(
        ran.ending,
        Ending::Exceeded(Exceeded::Elements {
            asked: 5_000_000,
            ceiling: Budget::FIELD_EVENT.elements
        })
    );
    assert_eq!(ran.value, None);
    let sentence = ran.ending.sentence().expect("a sentence");
    assert!(sentence.contains("5000000"), "{sentence}");
    println!("new Array(5e6).fill(0): {sentence} after {took:?}");
    let after = outcome(&request(
        Trigger::Format,
        "event.value = 'ok';",
        "1",
        "",
        false,
    ));
    assert_eq!(after.value.as_deref(), Some("ok"));
}

#[test]
fn unbounded_recursion_is_stopped_by_the_engine_s_own_limit() {
    let ran = outcome(&request(
        Trigger::Format,
        "function f() { return f(); } f();",
        "1",
        "",
        false,
    ));
    assert!(
        matches!(
            ran.ending,
            Ending::Exceeded(Exceeded::Recursion(_) | Exceeded::Stack(_))
        ),
        "{:?}",
        ran.ending
    );
}

#[test]
fn a_string_asked_for_by_an_argument_is_held_to_the_budget() {
    let ran = outcome(&request(
        Trigger::Format,
        "event.value = 'x'.padStart(1e9);",
        "1",
        "",
        false,
    ));
    assert!(
        matches!(ran.ending, Ending::Exceeded(Exceeded::StringUnits { .. })),
        "{:?}",
        ran.ending
    );
}

#[test]
fn get_field_of_a_field_that_does_not_exist_answers_null() {
    // The reference's `getField` answers `null` for a name no field has; reading a property of
    // it is then the script's own `TypeError`, which ends the run as a throw.
    let ran = outcome(&request(
        Trigger::Format,
        r#"var f = this.getField("NoSuchField"); event.value = String(f === null); f.value;"#,
        "1",
        "",
        false,
    ));
    assert!(ran.refusals.is_empty(), "{:?}", ran.refusals);
    assert!(matches!(ran.ending, Ending::Threw(_)), "{:?}", ran.ending);
    assert_eq!(ran.value, None, "a run that did not finish changes nothing");
}

#[test]
fn one_run_s_cost_is_the_construction_and_the_script() {
    let mut fastest = std::time::Duration::MAX;
    for _ in 0..20 {
        let started = Instant::now();
        let ran = outcome(&request(
            Trigger::Format,
            "event.value = '1';",
            "0",
            "",
            false,
        ));
        fastest = fastest.min(started.elapsed());
        assert_eq!(ran.value.as_deref(), Some("1"));
    }
    println!("one run of a one-statement script, fastest of 20: {fastest:?}");
}

#[test]
fn get_field_of_the_event_s_own_field_reads_its_value() {
    let ran = outcome(&request(
        Trigger::Format,
        r#"event.value = this.getField("Amount").value + "!";"#,
        "7",
        "",
        false,
    ));
    assert_eq!(ran.value.as_deref(), Some("7!"), "{ran:?}");
}

#[test]
fn launch_url_is_refused_by_name_and_logged_even_when_caught() {
    let ran = outcome(&request(
        Trigger::Format,
        r#"try { app.launchURL("https://example.org/"); } catch (e) { console.println(e.name); }
           event.value = "after";"#,
        "1",
        "",
        false,
    ));
    assert_eq!(ran.ending, Ending::Finished);
    assert_eq!(ran.value.as_deref(), Some("after"));
    let refusal = ran.refusals.first().expect("the refusal is recorded");
    assert_eq!(refusal.member, "app.launchURL");
    assert!(matches!(refusal.kind, RefusalKind::Excluded(_)));
    assert_eq!(ran.log, vec!["NotAllowedError".to_owned()]);
}

#[test]
fn submit_form_is_refused_by_name_and_stops_an_uncaught_script() {
    let ran = outcome(&request(
        Trigger::Keystroke,
        r#"this.submitForm("https://example.org/"); event.rc = false;"#,
        "1",
        "",
        true,
    ));
    assert!(ran.rc, "the line after the refused call did not run");
    let refusal = ran.refusals.first().expect("the refusal is recorded");
    assert_eq!(refusal.member, "this.submitForm");
    assert!(
        refusal.sentence().contains("Tier 2"),
        "{}",
        refusal.sentence()
    );
    assert!(matches!(ran.ending, Ending::Threw(_)), "{:?}", ran.ending);
}

#[test]
fn an_admitted_member_this_bridge_does_not_carry_says_so() {
    let ran = outcome(&request(
        Trigger::Keystroke,
        "if (event.keyDown) event.rc = false;",
        "1",
        "",
        true,
    ));
    let refusal = ran.refusals.first().expect("one refusal");
    assert_eq!(refusal.member, "event.keyDown");
    assert_eq!(refusal.kind, RefusalKind::NotBridged);
}

#[test]
fn validate_and_calculate_hand_back_rc_and_the_value() {
    let ran = outcome(&request(
        Trigger::Validate,
        "if (event.value > 10) event.rc = false;",
        "12",
        "",
        true,
    ));
    assert_eq!(ran.ending, Ending::Finished, "{:?}", ran.ending);
    assert!(!ran.rc, "a validate script's rc false is handed back");
    let ran = outcome(&request(
        Trigger::Calculate,
        "event.value = 6 * 7;",
        "1",
        "",
        false,
    ));
    assert_eq!(ran.value.as_deref(), Some("42"), "{ran:?}");
    assert!(ran.rc);
}

#[test]
fn a_script_that_does_not_parse_says_so() {
    let ran = outcome(&request(Trigger::Format, "event.value = ;", "1", "", false));
    assert!(
        matches!(ran.ending, Ending::Unparsed(_)),
        "{:?}",
        ran.ending
    );
}

#[test]
fn date_answers_the_request_s_moment() {
    let ran = outcome(&request(
        Trigger::Format,
        "event.value = String(new Date().getTime() === new Date().getTime() && Date.now());",
        "",
        "",
        false,
    ));
    assert_eq!(ran.value.as_deref(), Some("1704465015000"), "{ran:?}");
}

#[test]
fn every_refused_member_throws_not_allowed_error_by_name() {
    let kept = pdf_script::surface::REFUSED;
    for row in pdf_script::surface::EXCLUDED.iter().chain(kept) {
        for member in row.members {
            let spelled = format!("{}{member}", row.holder.prefix());
            let reached = match row.holder {
                pdf_script::surface::Holder::Field => format!("event.target.{member}"),
                pdf_script::surface::Holder::Layer => format!("this.getOCGs()[0].{member}"),
                _ => spelled.clone(),
            };
            let mut asked = request(
                Trigger::Format,
                &format!("try {{ var x = {reached}; }} catch (e) {{ console.println(e.name); }}"),
                "1",
                "",
                false,
            );
            asked.document = Some(pdf_model::view::DocumentState {
                info: Vec::new(),
                layers: vec![pdf_model::view::Layer {
                    number: 7,
                    generation: 0,
                    name: "Watermark".to_owned(),
                    on: true,
                    initially_on: true,
                    locked: false,
                }],
                annotations: Vec::new(),
                pages: Vec::new(),
            });
            let ran = outcome(&asked);
            assert_eq!(ran.ending, Ending::Finished, "{spelled}: {:?}", ran.ending);
            assert_eq!(
                ran.log,
                vec!["NotAllowedError".to_owned()],
                "{spelled} throws a NotAllowedError"
            );
            assert_eq!(
                ran.refusals.first().map(|refusal| refusal.member.as_str()),
                Some(spelled.as_str())
            );
            // A Tier 1 member this program keeps out says its own reason, not Tier 2's (ADR 1724).
            let kind = ran.refusals.first().map(|refusal| refusal.kind.clone());
            let expected = if kept.contains(row) {
                RefusalKind::Unreachable(format!("it {}", row.reason))
            } else {
                RefusalKind::Excluded(row.reason.to_owned())
            };
            assert_eq!(kind, Some(expected), "{spelled}'s reason");
        }
    }
}

#[test]
fn a_script_nested_past_the_parser_s_budget_is_stopped_before_it_is_parsed() {
    // Boa 0.22's parser has no depth limit: five hundred nested parentheses overflow an 8 MiB
    // stack. The budget stops the script by name before the parser sees it (ADR 1602).
    let deep = format!("event.value = {}1{};", "(".repeat(500), ")".repeat(500));
    let ran = outcome(&request(Trigger::Format, &deep, "1", "", false));
    assert_eq!(
        ran.ending,
        Ending::Exceeded(Exceeded::Nesting(Budget::FIELD_EVENT.nesting))
    );
    assert_eq!(ran.value, None);
    // At the budget the parser runs, on the realm thread `Engine` gives it: a test's own thread
    // has 2 MiB, which holds about sixty levels.
    let at = format!("event.value = {}2{};", "(".repeat(128), ")".repeat(128));
    let ran = pdf_script::Engine::new(Budget::FIELD_EVENT).run_request(&request(
        Trigger::Format,
        &at,
        "1",
        "",
        false,
    ));
    assert_eq!(ran.value.as_deref(), Some("2"), "{ran:?}");
}
