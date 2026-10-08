//! `app.setInterval`, `app.setTimeOut`, their two `clear` methods and `app.beep`: each a request the
//! view state carries out, recorded as an edit and crossing the wire as one (ADR 1702).
//!
//! The expected values are the reference's own shapes — an interval runs every period and a timeout
//! once, `clearInterval` of a value that is no timer object throws, `beep`'s numbers 0 to 4 name its
//! five sounds and an absent one is the default — read from Adobe's "app methods" page, cited and
//! never quoted.

#![cfg(feature = "engine")]
#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use pdf_model::view::{ScriptEdit, ScriptSite, Sound};
use pdf_script::{Budget, Ending, Event, Outcome, Realm, Request};

/// A request for `script` at `site`, telling the realm of no field.
fn request(site: ScriptSite, script: &str) -> Request {
    Request {
        site,
        field: String::new(),
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
        fields: Vec::new(),
        page: 0,
        pages: 3,
        dirty: false,
        document: None,
        moment: 1_704_465_015_000,
        utc_offset_seconds: 0,
    }
}

/// Runs `script` as the document's open action in a fresh realm.
fn opened(script: &str) -> (Realm, Outcome) {
    let mut realm = Realm::new(Budget::FIELD_EVENT).expect("a realm");
    let outcome = realm.run(&request(ScriptSite::OpenAction, script));
    (realm, outcome)
}

/// Both setters answer an object a script may add to — the reference's own example keeps a count
/// on it — and record the expression, the period and whether it repeats; clearing it records its
/// number, which no property the script added can change.
#[test]
fn a_timer_is_an_object_the_script_holds_and_an_edit_the_host_carries_out() {
    let (_, outcome) = opened(
        "var t = app.setInterval('tick()', 250); t.count = 0; t.timer = 'x'; \
         var o = app.setTimeOut('once()', 1000.9); app.clearInterval(t); \
         console.println(typeof t + ' ' + t.count);",
    );
    assert_eq!(outcome.ending, Ending::Finished, "{outcome:?}");
    assert_eq!(
        outcome.edits,
        vec![
            ScriptEdit::Timer {
                id: 0,
                script: "tick()".to_owned(),
                period: 250,
                repeat: true,
            },
            ScriptEdit::Timer {
                id: 1,
                script: "once()".to_owned(),
                period: 1000,
                repeat: false,
            },
            ScriptEdit::ClearTimer { id: 0 },
        ],
        "{outcome:?}"
    );
    assert_eq!(outcome.log, vec!["object 0".to_owned()], "{outcome:?}");
}

/// The realm numbers its timers for its whole life, so a timer a later run sets is not confused
/// with one an earlier run set; and a timer's own run is a document event, `this` the document.
#[test]
fn a_later_run_s_timer_has_a_number_of_its_own_and_runs_as_the_document() {
    let (mut realm, first) = opened("var a = app.setTimeOut('x()', 10);");
    assert_eq!(first.ending, Ending::Finished, "{first:?}");
    let later = realm.run(&request(
        ScriptSite::Timer,
        "var b = app.setTimeOut('y()', 10); console.println(event.type + '/' + event.name + ' ' + \
         (event.target === this) + ' ' + this.numPages);",
    ));
    assert_eq!(later.ending, Ending::Finished, "{later:?}");
    assert!(
        matches!(later.edits.as_slice(), [ScriptEdit::Timer { id: 1, .. }]),
        "{later:?}"
    );
    assert_eq!(later.log, vec!["App/Timer true 3".to_owned()], "{later:?}");
}

/// The reference's Stop button guards its `clear` with `try`, because a stop pressed before the
/// start passes `undefined`: that throws, and the run records nothing for it.
#[test]
fn clearing_what_is_no_timer_object_throws() {
    let (_, outcome) = opened(
        "var caught = ''; try { app.clearInterval(undefined); } catch (e) { caught = e.name; } \
         try { app.clearTimeOut({}); } catch (e) { caught += ' ' + e.name; } \
         console.println(caught);",
    );
    assert_eq!(outcome.ending, Ending::Finished, "{outcome:?}");
    assert!(outcome.edits.is_empty(), "{outcome:?}");
    assert_eq!(outcome.log, vec!["TypeError TypeError".to_owned()]);
}

/// `beep`'s five numbers, an absent one as the default and one the reference does not number as
/// the default too.
#[test]
fn a_beep_names_one_of_the_reference_s_five_sounds() {
    let (_, outcome) = opened("app.beep(0); app.beep(3); app.beep(); app.beep(9);");
    assert_eq!(outcome.ending, Ending::Finished, "{outcome:?}");
    let sounds: Vec<Sound> = outcome
        .edits
        .iter()
        .filter_map(|edit| match edit {
            ScriptEdit::Beep { sound } => Some(*sound),
            _ => None,
        })
        .collect();
    assert_eq!(
        sounds,
        vec![Sound::Error, Sound::Status, Sound::Default, Sound::Default]
    );
}

/// A run that throws after setting a timer sets none: every edit of a run is all or nothing.
#[test]
fn a_run_that_throws_sets_no_timer() {
    let (_, outcome) = opened("app.setInterval('tick()', 100); app.beep(1); throw 'stop';");
    assert!(matches!(outcome.ending, Ending::Threw(_)), "{outcome:?}");
    assert!(outcome.edits.is_empty(), "{outcome:?}");
}

/// The three edits and the timer's site cross the wire as they went in.
#[test]
fn the_timer_s_edits_and_site_cross_the_wire() {
    let (_, outcome) =
        opened("var t = app.setInterval('tick()', 250); app.clearInterval(t); app.beep(2);");
    let decoded = pdf_script::wire::decode_outcome(&pdf_script::wire::encode_outcome(&outcome))
        .expect("an outcome this crate encoded decodes");
    assert_eq!(decoded, outcome);
    let timer = request(ScriptSite::Timer, "tick()");
    let decoded = pdf_script::wire::decode_request(&pdf_script::wire::encode_request(&timer))
        .expect("a request this crate encoded decodes");
    assert_eq!(decoded, timer);
}
