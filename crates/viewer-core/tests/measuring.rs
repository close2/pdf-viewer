//! ISO 32000-2 §12.9's measurement, driven from two points and no window at all.
//!
//! §12.9.1 says what the clause is for, and it is a viewer rather than a page:
//!
//! > This information enables users of interactive PDF processors to perform measurements that
//! > yield results in the units intended by the creator of the document.
//!
//! `Query::Measure` is the half of that a host cannot do for itself. The points are the host's —
//! a rubber band is chrome, drawn where a selection highlight is drawn — and everything between
//! them and the string is this crate's and `pdf_model::measurement`'s: the viewport point becomes
//! default user space through the transform the frame was drawn with (ADR 0118), §12.9.1 chooses
//! the viewport of the *first* point, Table 267's conversions carry each axis into the measuring
//! system, and §12.9.2's five steps format the result.
//!
//! **Not one page of the 974 curated documents states a rectilinear `/VP`** — the one viewport
//! between `doc/pdf.js` and `doc/corpora` is `GEO`, which belongs to §12.10 — so the fixture is
//! written here and trap 8 is why: a corpus finds what documents contain, and this is a clause the
//! corpus cannot reach. The numbers are the standard's own EXAMPLE, so what is asserted below is
//! the clause's arithmetic rather than this tree's.
//!
//! ADR 1191.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "test code: the object numbers below are the small integers this file wrote"
)]
#![expect(
    clippy::panic,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::fmt::Write as _;

use viewer_core::{Answer, Command, DocumentId, Event, Query, Viewer};

/// One page, 400 by 400, with two viewports at two scales side by side.
///
/// The left half is §12.9.2's own EXAMPLE measure dictionary — `/X` converting user space to
/// miles at `.00139`, `/D` walking miles, feet and inches, `/A` in acres — and the right half is
/// the same drawing at ten times the scale, so that a path crossing the join measures differently
/// depending on which viewport §12.9.1 chooses. A reader taking the *last* point's viewport, or
/// the innermost, answers the other number.
fn plan() -> Vec<u8> {
    let objects: [String; 7] = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Contents 4 0 R \
         /VP [5 0 R 6 0 R] >>"
            .to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        "<< /Type /Viewport /BBox [0 0 200 400] /Name (Plan) /Measure 7 0 R >>".to_owned(),
        "<< /Type /Viewport /BBox [200 0 400 400] /Name (Inset) /Measure 7 0 R >>".to_owned(),
        "<< /Type /Measure /Subtype /RL /R (1in = 0.1 mi) \
         /X [<< /U (mi) /C .00139 /D 100000 >>] \
         /D [<< /U (mi) /C 1 >> << /U (ft) /C 5280 >> << /U (in) /C 12 /F /F /D 8 >>] \
         /A [<< /U (acres) /C 640 >>] >>"
            .to_owned(),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref_at = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len() + 1
    );
    out.into_bytes()
}

/// A viewer with that page open, fitted to a 400 by 400 viewport at scale 1.
///
/// The page and the viewport are the same size on purpose: the magnification is then 1 and a
/// device pixel is a user space unit, so the numbers below are the clause's rather than a
/// rounding of this crate's layout.
fn opened() -> Viewer {
    let mut viewer = Viewer::new(400, 400, 1.0);
    let opened = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: plan().into(),
            password: None,
            fragment: None,
        })
        .any(|event| matches!(event, Event::Opened { .. }));
    assert!(opened, "the fixture is a valid PDF");
    viewer
}

/// What the viewer makes of a path, or nothing.
fn measured(viewer: &Viewer, points: &[[f32; 2]]) -> Option<pdf_model::measurement::Traced> {
    match viewer.query(Query::Measure(points)) {
        Answer::Measured(traced) => Some(traced),
        Answer::None => None,
        other => panic!("Query::Measure answered {other:?}"),
    }
}

/// §12.9.1's rule, end to end: the viewport of the **first** point decides the units.
///
/// > Any measurement that potentially involves multiple viewports, such as one specifying the
/// > distance between two points, shall use the information specified in the viewport of the
/// > first point.
///
/// Both paths below are the same hundred pixels of the same page and they cross the same join;
/// what differs is which end they start at, and the clause says that is what decides. Naming the
/// viewport in the answer is what makes the assertion legible rather than a number nobody can
/// place.
#[test]
fn a_path_is_measured_in_the_viewport_of_its_first_point() {
    let viewer = opened();
    let left = measured(&viewer, &[[50.0, 200.0], [250.0, 200.0]]).expect("a path from the plan");
    assert_eq!(left.viewport.as_deref(), Some("Plan"));
    assert_eq!(left.ratio.as_deref(), Some("1in = 0.1 mi"));

    let right = measured(&viewer, &[[250.0, 200.0], [50.0, 200.0]]).expect("a path from the inset");
    assert_eq!(right.viewport.as_deref(), Some("Inset"));
}

/// §12.9.2's algorithm reaching a host, over the clause's own numbers.
///
/// The EXAMPLE's `/X` converts a user space unit to `.00139` miles, so 1043.52 of them along one
/// axis is 1.4505 miles — the very value the clause walks — and the string its three number
/// format dictionaries produce is stated in the standard: `1 mi 2,378 ft 7 5/8 in`. Measured here
/// over two points the distance is shorter, and what the assertion is about is that every part of
/// the walk ran: a unit label, `/RT` between orders of thousands, and `/F /F` with `/D 8` showing
/// eighths of an inch.
#[test]
fn the_clauses_own_formatting_reaches_a_host_through_the_query() {
    let viewer = opened();
    // A hundred user space units along x, which the EXAMPLE's `/X` makes 0.139 miles: no whole
    // mile, 733 feet and a fraction of an inch — the algorithm's steps c), d) and e) all in one
    // string.
    let traced = measured(&viewer, &[[50.0, 200.0], [150.0, 200.0]]).expect("a path in the plan");
    let length = traced.length.expect("Table 267's /D states three units");
    assert!(length.starts_with("0 mi "), "{length}");
    assert!(length.contains(" ft "), "{length}");
    assert!(length.ends_with(" in"), "{length}");
    assert!(
        traced.slope.is_none(),
        "this drawing states no /S, so there is no slope to show"
    );

    // Three points close the shape, and `/A` is the one array that states acres.
    let corner = measured(&viewer, &[[50.0, 100.0], [150.0, 100.0], [150.0, 200.0]])
        .expect("a path in the plan");
    let area = corner.area.expect("Table 267's /A states acres");
    assert!(area.ends_with(" acres"), "{area}");
}

/// A point on no viewport is the page saying nothing, and it answers nothing.
///
/// §12.9.1 chooses "the viewport of the first point", so a first point inside no `/BBox` leaves
/// the clause with no measuring system to apply — which is an answer about the document rather
/// than a failure of this query.
#[test]
fn a_page_with_no_viewport_under_the_first_point_answers_nothing() {
    let mut viewer = Viewer::new(400, 400, 1.0);
    assert!(
        measured(&viewer, &[[10.0, 10.0], [20.0, 20.0]]).is_none(),
        "no document is open"
    );

    viewer = opened();
    assert!(
        measured(&viewer, &[]).is_none(),
        "an empty path measures nothing"
    );
    // Far below the page: `user_space` still names a position in the page's own coordinates, and
    // neither `/BBox` contains it.
    assert!(
        measured(&viewer, &[[50.0, 4000.0], [60.0, 4000.0]]).is_none(),
        "the page states no units there"
    );
}
