//! The bytes a request and an outcome cross a process boundary as (RFC 0008 section 6.2).

use std::time::Duration;

use pdf_model::action::Trigger as AnnotationTrigger;
use pdf_model::aform::Trigger;
use pdf_model::view::{
    Alignment, BorderStyle, Colour, Display, DocumentTrigger, FieldState, FieldType, Property,
    ScriptEdit, ScriptSite, TextFlag,
};
use pdf_script::wire::{WireError, decode_outcome, decode_request, encode_outcome, encode_request};
use pdf_script::{Ending, Event, Exceeded, Outcome, Refusal, RefusalKind, Request};

fn request(site: ScriptSite) -> Request {
    Request {
        site,
        field: "Total.Amount".to_owned(),
        label: "library".to_owned(),
        script: "if (event.willCommit) event.value = 'é';".to_owned(),
        event: Event {
            value: "12".to_owned(),
            change: "€".to_owned(),
            selection_start: 1,
            selection_end: 2,
            will_commit: true,
            commit_key: 0,
            field_full: false,
            change_ex: String::new(),
            source: "Line.1".to_owned(),
        },
        fields: vec![FieldState {
            name: "Total.Amount".to_owned(),
            kind: FieldType::ComboBox,
            value: "12".to_owned(),
            flags: 1 << 17,
            display: Display::NoPrint,
            text_color: Some(Colour::Rgb([1.0, 0.0, 0.5])),
            fill_color: Some(Colour::Transparent),
            stroke_color: None,
            border_style: BorderStyle::Underline,
            alignment: Alignment::Right,
            char_limit: Some(8),
            page: None,
            rect: [1.0, 2.5, 3.0, -4.0],
            captions: Default::default(),
        }],
        page: 3,
        pages: 9,
        dirty: false,
        document: None,
        moment: 1_704_465_015_000,
        utc_offset_seconds: -3600,
    }
}

fn outcome() -> Outcome {
    Outcome {
        rc: false,
        value: Some("$1.00".to_owned()),
        change: None,
        edits: vec![
            ScriptEdit::Value {
                field: "Total".to_owned(),
                value: "3".to_owned(),
            },
            ScriptEdit::Property {
                field: "Total".to_owned(),
                property: Property::TextColor(Colour::Cmyk([0.0, 0.1, 0.2, 0.3])),
            },
            ScriptEdit::Property {
                field: "Total".to_owned(),
                property: Property::Display(Display::Hidden),
            },
            ScriptEdit::Reset {
                fields: vec!["A".to_owned(), "B".to_owned()],
            },
            ScriptEdit::Calculate,
            ScriptEdit::Property {
                field: "Total".to_owned(),
                property: Property::TextFlag(TextFlag::Comb, true),
            },
            ScriptEdit::Focus {
                field: "Total".to_owned(),
            },
            ScriptEdit::GoTo { page: 3 },
        ],
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
        notes: vec!["a note".to_owned()],
    }
}

#[test]
fn a_request_and_an_outcome_come_back_as_they_went() {
    for site in [
        ScriptSite::Field(Trigger::Calculate),
        ScriptSite::Annotation(AnnotationTrigger::PageInvisible),
        ScriptSite::Page(pdf_model::action::PageTrigger::Close),
        ScriptSite::OpenAction,
        ScriptSite::Library,
    ]
    .into_iter()
    .chain(DocumentTrigger::ALL.map(ScriptSite::Document))
    {
        assert_eq!(
            decode_request(&encode_request(&request(site))).expect("decodes"),
            request(site)
        );
    }
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
    let mut bytes = encode_request(&request(ScriptSite::Library));
    if let Some(first) = bytes.first_mut() {
        *first = 1;
    }
    assert_eq!(decode_request(&bytes), Err(WireError::Version(1)));
}

#[test]
fn a_hostile_length_asks_for_nothing() {
    let mut bytes = vec![pdf_script::wire::VERSION, 4];
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(decode_request(&bytes), Err(WireError::Truncated));
}

#[test]
fn every_budget_comes_back_as_it_went() {
    for exceeded in [
        Exceeded::Wall(Duration::from_millis(100)),
        Exceeded::Steps(7),
        Exceeded::LoopIterations(8),
        Exceeded::Recursion(9),
        Exceeded::Stack(10),
        Exceeded::Nesting(256),
        Exceeded::Elements {
            asked: 3,
            ceiling: 2,
        },
        Exceeded::StringUnits {
            asked: 5,
            ceiling: 4,
        },
    ] {
        let sent = Outcome {
            ending: Ending::Exceeded(exceeded),
            ..outcome()
        };
        assert_eq!(
            decode_outcome(&encode_outcome(&sent)).expect("decodes"),
            sent
        );
    }
}
