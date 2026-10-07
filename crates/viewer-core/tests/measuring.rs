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
        Answer::Measured { traced, .. } => Some(traced),
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

/// A document under `doc/`, or `None` where the gitignored corpora are not on this disk.
fn on_disk(relative: &str) -> Option<Vec<u8>> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative),
    )
    .ok()
}

/// One page of `bytes`, shown, with the device point of a point in its default user space.
fn shown_at(bytes: Vec<u8>, page: usize, user: (f32, f32)) -> (Viewer, [f32; 2]) {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
        .handle(Command::GoTo(viewer_core::PageTarget::Index(page)))
        .for_each(drop);
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(page)) else {
        panic!("the page on the screen has a geometry");
    };
    let point = [
        geometry.origin.0 + user.0 * geometry.scale,
        geometry.origin.1 + (geometry.page.height - user.1) * geometry.scale,
    ];
    (viewer, point)
}

/// §12.10's one geographic map among the curated documents: a point in the middle of its
/// viewport is located by the affine map its four corners determine (ADR 1593).
///
/// `bug1146106.pdf` registers a 715 by 522 viewport by its four corners, a north-up rectangle of
/// degrees from 9.43386° S to 17.71438° S and 165.52069° E to 176.86596° E, so its middle is the
/// rectangle's — 13.57412° S, 171.193325° E — and the fit is exact. `/LPTS` writes the upper
/// corners at 1.00001 rather than 1, which is why the middle is checked to five places.
#[test]
fn the_middle_of_a_geographic_map_is_the_middle_of_its_degrees() {
    let Some(bytes) = on_disk("doc/pdf.js/test/pdfs/bug1146106.pdf") else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let (viewer, point) = shown_at(bytes, 0, (715.146_56 / 2.0, 261.0));
    let Answer::Measured {
        traced,
        located:
            Some(viewer_core::Located::At {
                latitude,
                longitude,
                display,
                departure,
            }),
    } = viewer.query(Query::Measure(&[point]))
    else {
        panic!("the map's middle is located");
    };
    assert!(traced.geospatial.is_some());
    assert!((latitude - (-13.574_12)).abs() < 1e-4, "{latitude}");
    assert!((longitude - 171.193_325).abs() < 1e-4, "{longitude}");
    assert_eq!(display, None, "the file names no /DCS");
    assert!(departure < 1e-3, "{departure}");
}

/// A projected map whose `/GPTS` are degrees — the shape every projected map of the crawl takes
/// — is refused by name until the owner answers `doc/questions/Q271`, and the refusal is the
/// position's sentence.
///
/// Written here because no curated document carries that shape with a method this tree
/// evaluates: the one projected map among them is refused by its method first, below.
#[test]
fn a_projected_map_with_degrees_for_eastings_gives_no_position() {
    let wkt = "PROJCS[\"ETRS89_UTM_zone_32N\",GEOGCS[\"GCS_ETRS_1989\",DATUM[\"D_ETRS_1989\",\
               SPHEROID[\"GRS_1980\",6378137,298.257222101]],PRIMEM[\"Greenwich\",0],\
               UNIT[\"Degree\",0.017453292519943295]],PROJECTION[\"Transverse_Mercator\"],\
               PARAMETER[\"latitude_of_origin\",0],PARAMETER[\"central_meridian\",9],\
               PARAMETER[\"scale_factor\",0.9996],PARAMETER[\"false_easting\",500000],\
               PARAMETER[\"false_northing\",0],UNIT[\"Meter\",1]]";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /VP [4 0 R] >>".to_owned(),
        "<< /Type /Viewport /BBox [0 0 400 400] /Measure 5 0 R >>".to_owned(),
        format!(
            "<< /Type /Measure /Subtype /GEO /GCS << /Type /PROJCS /WKT ({wkt}) >> \
             /GPTS [47 8 48 8 48 10 47 10] /LPTS [0 0 0 1 1 1 1 0] >>"
        ),
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
    let (viewer, point) = shown_at(out.into_bytes(), 0, (200.0, 200.0));
    let Answer::Measured {
        located: Some(viewer_core::Located::Refused(why)),
        ..
    } = viewer.query(Query::Measure(&[point]))
    else {
        panic!("a projected map with degree-shaped points is refused");
    };
    assert!(why.contains("shaped as degrees"), "{why}");
}

/// The one projected map among the curated documents names a projection method this tree does
/// not evaluate, and is refused by that method's name.
#[test]
fn a_projected_map_states_why_it_gives_no_position() {
    let Some(bytes) = on_disk(
        "doc/corpora/format-corpus/jhove-errors/PDF-HUL-117/\
         363_Risk-Based Management of Groundwater Contamination _Paper.pdf",
    ) else {
        eprintln!("skipped: doc/corpora is not on this disk");
        return;
    };
    let needle = "Hotine_Oblique_Mercator_Azimuth_Natural_Origin";
    for page in [87, 91] {
        let (viewer, point) = shown_at(bytes.clone(), page, (300.0, 400.0));
        let Answer::Measured {
            located: Some(viewer_core::Located::Refused(why)),
            ..
        } = viewer.query(Query::Measure(&[point]))
        else {
            panic!("page {} is a projected map, refused", page + 1);
        };
        assert!(why.contains(needle), "page {}: {why}", page + 1);
    }
}
