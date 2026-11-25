//! Specification audit of ISO 32000-2 §12.9's measurement: Table 268's number format entries and
//! §12.9.2's algorithm, and Table 265's measuring axes, read through a page's `/VP` (review 2239).
//!
//! Every expected string is derived from the clause's own words, quoted beside each assertion;
//! no other processor is consulted. Each audited test has a control that passes on the tree it
//! was written against, so the failure is the audited sentence and not the fixture.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    reason = "bounded exact-xref fixture; a fixture that does not read must fail the test"
)]

use std::fmt::Write as _;

use pdf_model::measurement::Viewports;
use pdf_syntax::Document;

/// A one-page document whose page states one viewport over `bbox` with `measure` as its
/// `/Measure`.
fn page_viewports(bbox: &str, measure: &str) -> Viewports {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Resources << >> /VP [4 0 R] >>"
            .to_owned(),
        format!("<< /Type /Viewport /BBox {bbox} /Measure 5 0 R >>"),
        measure.to_owned(),
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(bytes, "{} 0 obj\n{body}\nendobj", index + 1);
    }
    let xref = bytes.len();
    let _ = write!(
        bytes,
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len() + 1
    );
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    );
    let document = Document::open(bytes.into_bytes()).expect("valid exact-xref fixture");
    assert!(!document.was_recovered());
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("the fixture has one page");
    Viewports::read(&document, &page.dict)
}

/// A rectilinear measure of one user space unit to one `u`, with `distance` as the one number
/// format dictionary of its `/D` array.
fn distance_in(distance: &str) -> Viewports {
    page_viewports(
        "[0 0 400 400]",
        &format!(
            "<< /Type /Measure /Subtype /RL /R (1 u = 1 u) /X [<< /U (u) /C 1 >>] \
             /D [{distance}] /A [<< /U (u2) /C 1 >>] >>"
        ),
    )
}

/// Table 268's `/RD`: "Text that shall be used as the decimal position in displaying numerical
/// values. An empty string indicates that the default shall be used. Default value: PERIOD
/// (2Eh)." So `/RD ()` is the period, unlike `/RT`, `/PS` and `/SS`, whose empty string is
/// "no text shall be added". 12.5 units is exactly representable.
#[test]
fn an_empty_decimal_position_is_the_default_period_spec_audit() {
    let measured = |format: &str| {
        distance_in(format)
            .distance((100.0, 200.0), (112.5, 200.0))
            .expect("the first point is inside the viewport and /D is stated")
    };
    // Control: a stated separator is used as stated.
    assert_eq!(measured("<< /U (u) /C 1 /RD (,) >>"), "12,5 u");
    // Audited: Table 268's own sentence for the empty string.
    assert_eq!(
        measured("<< /U (u) /C 1 /RD () >>"),
        "12.5 u",
        "Table 268 /RD: \"An empty string indicates that the default shall be used\""
    );
}

/// Table 268's `/F` `D`: "Show as decimal to the precision specified by the D entry", and `/D`
/// 100 is "hundredths, corresponding to two decimal digits". 2.999 shown to hundredths is 3.00,
/// whose low-order zeros "may be truncated unless FD is true" — so "3 u" or "3.00 u", and never
/// a value a whole unit short of the measurement.
#[test]
fn a_decimal_that_rounds_up_to_a_whole_unit_carries_into_it_spec_audit() {
    let viewports = distance_in("<< /U (u) /C 1 >>");
    // Control: a fraction that does not round to a whole unit.
    let control = viewports
        .distance((100.0, 200.0), (102.5, 200.0))
        .expect("the first point is inside the viewport and /D is stated");
    assert_eq!(control, "2.5 u");
    // Audited: 2.999 to two decimal digits.
    let audited = viewports
        .distance((100.0, 200.0), (102.999, 200.0))
        .expect("the first point is inside the viewport and /D is stated");
    assert!(
        audited == "3 u" || audited == "3.00 u",
        "2.999 to hundredths is 3.00 (Table 268 /F D, /D 100), got {audited:?}"
    );
    // The same with FD true, which keeps the zeros: "3.00 u".
    let kept = distance_in("<< /U (u) /C 1 /FD true >>")
        .distance((100.0, 200.0), (102.999, 200.0))
        .expect("the first point is inside the viewport and /D is stated");
    assert_eq!(
        kept, "3.00 u",
        "Table 268 /FD true keeps the two digits /D 100 asks for"
    );
}

/// Table 265's `/BBox`: "The two coordinate pairs of the rectangle shall be specified in
/// normalised form; that is, lower-left followed by upper-right, relative to the measuring
/// coordinate system. This ordering shall determine the orientation of the measuring coordinate
/// system (that is, the direction of the positive x and y axes) in this viewport". Table 267's
/// `/X`: "The directions of the x and y axes are in the measuring coordinate system … These
/// directions shall be determined by the `BBox` entry of the containing viewport". A viewport
/// stated as `[0 400 400 0]` has its lower-left at the page's top-left, so its positive y runs
/// down the page: a line rising one unit on the page per unit to the right falls one measuring
/// unit, and its slope (`/S`, Y units over X units) is -1 there and +1 in `[0 0 400 400]`.
#[test]
fn a_slope_is_taken_along_the_axes_the_viewport_orders_spec_audit() {
    let measure = "<< /Type /Measure /Subtype /RL /R (1 u = 1 u) /X [<< /U (u) /C 1 >>] \
                   /D [<< /U (u) /C 1 >>] /A [<< /U (u2) /C 1 >>] /S [<< /U (rise) /C 1 >>] >>";
    let slope = |bbox: &str| {
        page_viewports(bbox, measure)
            .traced(&[[100.0, 100.0], [200.0, 200.0]])
            .and_then(|traced| traced.slope)
            .expect("a two-point path inside the viewport with /S stated has a slope")
    };
    // Control: measuring axes that run as the page's do.
    assert_eq!(slope("[0 0 400 400]"), "1 rise");
    // Audited: the measuring y axis runs down the page.
    assert_eq!(
        slope("[0 400 400 0]"),
        "-1 rise",
        "Table 265: the /BBox corner order determines the direction of the positive y axis"
    );
}
