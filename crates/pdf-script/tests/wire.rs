//! The bytes a request and an outcome cross a process boundary as (RFC 0008 section 6.2).

use std::time::Duration;

use pdf_model::aform::Trigger;
use pdf_script::wire::{WireError, decode_outcome, decode_request, encode_outcome, encode_request};
use pdf_script::{Ending, Event, Exceeded, Outcome, Refusal, RefusalKind, Request};

fn request() -> Request {
    Request {
        trigger: Trigger::Keystroke,
        field: "Total.Amount".to_owned(),
        script: "if (event.willCommit) event.value = 'é';".to_owned(),
        event: Event {
            value: "12".to_owned(),
            change: "€".to_owned(),
            selection_start: 1,
            selection_end: 2,
            will_commit: true,
        },
        moment: 1_704_465_015_000,
        utc_offset_seconds: -3600,
    }
}

fn outcome() -> Outcome {
    Outcome {
        rc: false,
        value: Some("$1.00".to_owned()),
        change: None,
        ending: Ending::Exceeded(Exceeded::Wall(Duration::from_millis(100))),
        refusals: vec![
            Refusal {
                member: "app.launchURL".to_owned(),
                kind: RefusalKind::Excluded("leaves the machine".to_owned()),
            },
            Refusal {
                member: "event.commitKey".to_owned(),
                kind: RefusalKind::NotBridged,
            },
        ],
        log: vec!["one".to_owned(), "two".to_owned()],
    }
}

#[test]
fn a_request_and_an_outcome_come_back_as_they_went() {
    assert_eq!(
        decode_request(&encode_request(&request())).expect("decodes"),
        request()
    );
    assert_eq!(
        decode_outcome(&encode_outcome(&outcome())).expect("decodes"),
        outcome()
    );
}

#[test]
fn every_cut_of_an_encoding_is_refused_and_nothing_follows_one() {
    let bytes = encode_outcome(&outcome());
    for end in 0..bytes.len() {
        assert!(decode_outcome(bytes.get(..end).expect("a prefix")).is_err());
    }
    let mut longer = bytes;
    longer.push(0);
    assert_eq!(decode_outcome(&longer), Err(WireError::Trailing(1)));
}

#[test]
fn another_version_is_refused_rather_than_misread() {
    let mut bytes = encode_request(&request());
    if let Some(first) = bytes.first_mut() {
        *first = 2;
    }
    assert_eq!(decode_request(&bytes), Err(WireError::Version(2)));
}

#[test]
fn a_hostile_length_asks_for_nothing() {
    let mut bytes = vec![pdf_script::wire::VERSION, 0];
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(decode_request(&bytes), Err(WireError::Truncated));
}
