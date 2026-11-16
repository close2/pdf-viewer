//! Availability probe for the pinned Adobe reference's `Doc.addLink` Example 2.
//!
//! A193 makes safe document/viewer reference APIs the target. The official sample constructs
//! `Matrix2D` and calls `fromRotated`, `invert`, and `transform`. This tests only that surface;
//! it neither assumes undocumented constructor overloads nor certifies method semantics.
//! Source: <https://raw.githubusercontent.com/adobe/dc-acrobat-sdk-docs/ab3b42a7/docs/library/jsapiref/doc.html>
//!
#![cfg(feature = "engine")]

// The ordinary-array control checks this realm and event-value readback before construction.

use pdf_model::aform::Trigger;
use pdf_model::view::{ScriptSite, WindowView};
use pdf_script::{Budget, Ending, Event, Realm, Request};

fn request(script: &str) -> Request {
    Request {
        site: ScriptSite::Field(Trigger::Calculate),
        field: "MatrixProbe".to_owned(),
        label: "Matrix2D reference sample".to_owned(),
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
            shift: false,
            modifier: false,
            key_down: false,
            rich_value: String::new(),
        },
        fields: Vec::new(),
        page: 0,
        pages: 1,
        dirty: false,
        document: None,
        view: WindowView::default(),
        moment: 1_704_465_015_000,
        utc_offset_seconds: 0,
    }
}

#[test]
fn matrix2d_constructor_and_reference_methods_are_available_to_a_document_realm() {
    let mut realm = Realm::new(Budget::FIELD_EVENT).expect("one bounded realm");

    let ordinary = realm.run(&request("event.value = [1,2,3].join(',');"));
    assert_eq!(
        ordinary.ending,
        Ending::Finished,
        "ordinary JS control: {ordinary:?}"
    );
    assert_eq!(
        ordinary.value.as_deref(),
        Some("1,2,3"),
        "control is independent"
    );

    let matrix = realm.run(&request(
        "var m = new Matrix2D(); \
         event.value = [typeof m.fromRotated, typeof m.invert, typeof m.transform].join('|');",
    ));
    assert_eq!(
        matrix.ending,
        Ending::Finished,
        "reference API call: {matrix:?}"
    );
    assert_eq!(
        matrix.value.as_deref(),
        Some("function|function|function"),
        "the official example's constructor and methods are present"
    );
}
