//! `Matrix2D`'s members do the arithmetic ADR 3010 states for them: ISO 32000-2 §8.3.3's six numbers
//! with `x' = a·x + c·y + h` and `y' = b·x + d·y + v`, an inverse, and a pairwise `transform`.
#![cfg(feature = "engine")]
#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use pdf_model::aform::Trigger;
use pdf_model::view::{ScriptSite, WindowView};
use pdf_script::{Budget, Ending, Event, Realm, Request};

fn request(script: &str) -> Request {
    Request {
        site: ScriptSite::Field(Trigger::Calculate),
        field: "MatrixProbe".to_owned(),
        label: "Matrix2D arithmetic".to_owned(),
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

fn value_of(script: &str) -> String {
    let mut realm = Realm::new(Budget::FIELD_EVENT).expect("one bounded realm");
    let run = realm.run(&request(script));
    assert_eq!(run.ending, Ending::Finished, "{script}: {run:?}");
    run.value.unwrap_or_default()
}

#[test]
fn the_default_matrix_is_the_identity_and_a_stated_one_maps_by_section_8_3_3() {
    assert_eq!(
        value_of("event.value = new Matrix2D().transform([3, 4]).join(',');"),
        "3,4"
    );
    // [2 1 0 3 5 7]: x' = 2·1 + 0·2 + 5 = 7, y' = 1·1 + 3·2 + 7 = 14.
    assert_eq!(
        value_of("event.value = new Matrix2D(2, 1, 0, 3, 5, 7).transform([1, 2]).join(',');"),
        "7,14"
    );
}

#[test]
fn the_inverse_undoes_the_matrix_and_a_quad_list_is_mapped_member_by_member() {
    assert_eq!(
        value_of(
            // Every entry of [2 0 0 4 5 7] and of its inverse is a binary fraction, so the round trip
            // is exact.
            "var m = new Matrix2D(2, 0, 0, 4, 5, 7); \
             var q = m.invert().transform(m.transform([[1, 2, 3, 4], [5, 6, 7, 8]])); \
             event.value = q.map(function (r) { return r.join(','); }).join(';');"
        ),
        "1,2,3,4;5,6,7,8"
    );
}

#[test]
fn a_matrix_with_no_inverse_is_refused_by_name() {
    assert_eq!(
        value_of(
            "try { new Matrix2D(1, 2, 2, 4, 0, 0).invert(); event.value = 'inverted'; } \
             catch (e) { event.value = e.name; }"
        ),
        "RangeError"
    );
}
