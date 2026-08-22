//! Where ISO 32000-2 §10.5's transfer function applies, and where §11.7.5.2 says it does not.
//!
//! # Why this file exists
//!
//! The function itself has been implemented since the three-hundred-and-fifty-eighth session and
//! the corpus cannot say anything about it: `examples/transfer_function_census` finds one document
//! in the whole of `doc/pdf.js` that states a `/TR` which is not `/Identity` or `/Default`, and
//! that one draws a single image at full alpha under the Normal blend mode with no mask anywhere.
//! So every rule below is defended by a fixture or by nothing — trap 8 — and the two departures
//! these tests pin were both *silent* until the six-hundred-and-thirty-seventh session.
//!
//! The two are different clauses and are tested apart:
//!
//! - **§11.7.5.2** makes the parameter a property of a *region*. The transfer function at a point
//!   is the topmost object's "but only if the object is fully opaque", and the page's default
//!   otherwise — so a page that states one and paints anything translucent over it is drawn with a
//!   function the clause does not put there. This tree applies each object's own function to its
//!   own colour before compositing, which agrees with the clause exactly on the fully opaque case.
//! - **§10.5** applies the function to every component value on its way to the device, and a
//!   shading's colours reach the backend as a ramp, a mesh or a sampled grid. Those are built in
//!   `shading::kind_of`, so that is where the function is applied — *inside* the sampling rather
//!   than to the samples, which is what `a_ramp_is_the_composition_and_not_its_endpoints` is
//!   about and what nothing else in this tree can see.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly and say what it found instead, \
              and these pages are 100 units square where no arithmetic can overflow"
)]

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_render::{Command, Corners, Paint, Ramp, Shading, ShadingKind, Transform};
use pdf_syntax::Document;

/// One eight-bit level, which is the resolution every colour below is finally drawn at.
const LEVEL: f32 = 1.0 / 255.0;

/// An inverting §7.10.3 exponential function, which no reader can mistake for the identity.
///
/// `/C0 [1] /C1 [0] /N 1` maps 0 to 1 and 1 to 0. It is one component, which
/// `ext_gstate::Transfer::read` gives to all three of an RGB device's channels — Table 57's
/// "a single function" case rather than its array of four.
const INVERT: &str = "<< /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >>";

/// A one-page fixture: `resources` goes in the page's resource dictionary, `extra` after object 4.
fn fixture(resources: &str, content: &str, extra: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << {resources} >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n{extra}",
        content.len() + 1
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        if object.trim().is_empty() {
            continue;
        }
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Every [`pdf_model::Unsupported::TransferFunction`] this page raises, in the order they sort.
fn transfer_reports(bytes: Vec<u8>) -> Vec<String> {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    interpretation
        .unsupported
        .iter()
        .filter_map(|item| match item {
            pdf_model::Unsupported::TransferFunction { detail } => Some(detail.clone()),
            _ => None,
        })
        .collect()
}

/// A page stating a transfer function and painting one translucent mark under it.
///
/// ISO 32000-2 §11.7.5.2:
///
/// > For portions of the page whose topmost object is not fully opaque or that are never painted
/// > at all, the default halftone and transfer function for the page shall be used
///
/// One stated function and one `ca` below 1.0 is the whole of the condition — no second function
/// competing with a first, which is what the ledger's row claimed for two hundred sessions. The
/// second half of the test is the mutation: the same page painted opaque agrees with the clause
/// exactly, because the six conditions "ensure that only the object itself shall contribute to the
/// colour at the given point".
#[test]
fn a_translucent_mark_under_a_transfer_function_is_reported() {
    let resources = format!("/ExtGState << /Solid << /TR {INVERT} >> /Half << /ca 0.5 >> >>");
    let translucent = transfer_reports(fixture(
        &resources,
        "/Solid gs 1 0 0 rg 0 0 50 50 re f /Half gs 0 0 1 rg 10 10 50 50 re f",
        "",
    ));
    assert_eq!(
        translucent.len(),
        1,
        "one report for the page, not one per mark: {translucent:?}"
    );
    let detail = translucent.first().expect("the report just counted");
    assert!(
        detail.contains("§11.7.5.2") && detail.contains("non-stroking alpha constant is below 1.0"),
        "the report names the clause and the condition that matched: {detail}"
    );

    let opaque = transfer_reports(fixture(
        &resources,
        "/Solid gs 1 0 0 rg 0 0 50 50 re f 0 0 1 rg 10 10 50 50 re f",
        "",
    ));
    assert!(
        opaque.is_empty(),
        "a fully opaque page is what the clause and this tree agree on: {opaque:?}"
    );
}

/// The clause's fifth condition, which a mark inside a group cannot see for itself.
///
/// ISO 32000-2 §11.7.5.2:
///
/// > The foregoing four conditions were also true at the time the Do operator was invoked for the
/// > group containing the object, as well as for any direct ancestor groups.
///
/// §11.6.6 resets the blend mode, both alpha constants and the soft mask before a transparency
/// group's content runs, so the mark below is fully opaque by its own graphics state and is not
/// fully opaque by the clause. A flag reading the mark alone would report nothing here, which is
/// exactly the nested case §11.7.5.2 spends four of its six conditions on.
#[test]
fn a_group_invoked_translucently_carries_its_opacity_to_the_marks_inside() {
    let resources = "/ExtGState << /Half << /ca 0.5 >> >> /XObject << /Fm 5 0 R >>";
    let inner = "/Inner gs 1 0 0 rg 0 0 50 50 re f";
    let form = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /CS /DeviceRGB >> \
         /Resources << /ExtGState << /Inner << /TR {INVERT} >> >> >> /Length {} >>\n\
         stream\n{inner}\nendstream\nendobj\n",
        inner.len() + 1
    );

    let translucent = transfer_reports(fixture(resources, "/Half gs /Fm Do", &form));
    assert_eq!(
        translucent.len(),
        1,
        "the ancestry is what makes this one reportable: {translucent:?}"
    );
    let detail = translucent.first().expect("the report just counted");
    assert!(
        detail.contains("§11.7.5.2") && detail.contains("enclosing group's Do"),
        "the report names the condition the ancestry failed: {detail}"
    );

    let opaque = transfer_reports(fixture(resources, "/Fm Do", &form));
    assert!(
        opaque.is_empty(),
        "the same group invoked opaquely satisfies every one of the six: {opaque:?}"
    );
}

/// The one shading the display list carries, or a panic naming what it found instead.
fn painted_shading(bytes: Vec<u8>) -> Arc<Shading> {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    let shading = interpretation
        .display_list
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Fill {
                paint: Paint::Shading(shading),
                ..
            } => Some(Arc::clone(shading)),
            _ => None,
        });
    shading.unwrap_or_else(|| {
        panic!(
            "the page paints no shading: {:?}",
            interpretation.unsupported
        )
    })
}

/// An axial shading whose grey runs from black to white along the page's diagonal.
const RAMP: &str = "/Shading << /Sh << /ShadingType 2 /ColorSpace /DeviceRGB \
                    /Coords [0 0 100 100] /Function << /FunctionType 2 /Domain [0 1] \
                    /C0 [0 0 0] /C1 [1 1 1] /N 1 >> /Extend [true true] >> >>";

/// The ramp of the shading a page paints, or a panic if it paints another kind.
fn painted_ramp(bytes: Vec<u8>) -> Ramp {
    let shading = painted_shading(bytes);
    match shading.kind.as_ref() {
        ShadingKind::Axial { ramp, .. } | ShadingKind::Radial { ramp, .. } => ramp.clone(),
        other => panic!("expected an axial shading, got {other:?}"),
    }
}

/// §10.5 applies to every component value, and an `sh`'s ramp is made of component values.
///
/// > In the sequence of steps for processing colours, the PDF processor shall apply the transfer
/// > function after performing any needed conversions between colour spaces.
///
/// > The input shall be the value of a colour component in the device's native colour space,
/// > either specified directly or produced by conversion from some other colour space.
///
/// Nothing in the clause distinguishes a colour a `rg` operator states from a colour a shading's
/// function produces, so the ramp below is inverted end for end. The mutation is the same `sh`
/// with `/Identity` in force, which Table 57 makes the way to state no function at all.
#[test]
fn an_axial_ramp_is_sampled_through_the_transfer_function() {
    let inverted = painted_ramp(fixture(
        &format!("/ExtGState << /Solid << /TR {INVERT} >> >> {RAMP}"),
        "/Solid gs /Sh sh",
        "",
    ));
    assert!(
        (inverted.colour_at(0.0).r - 1.0).abs() < LEVEL,
        "the ramp's black end is white under an inverting function: {:?}",
        inverted.colour_at(0.0)
    );
    assert!(
        inverted.colour_at(1.0).r < LEVEL,
        "and its white end is black: {:?}",
        inverted.colour_at(1.0)
    );

    let plain = painted_ramp(fixture(
        &format!("/ExtGState << /Solid << /TR /Identity >> >> {RAMP}"),
        "/Solid gs /Sh sh",
        "",
    ));
    assert!(
        plain.colour_at(0.0).r < LEVEL && (plain.colour_at(1.0).r - 1.0).abs() < LEVEL,
        "/Identity leaves the shading's own colours: {:?} {:?}",
        plain.colour_at(0.0),
        plain.colour_at(1.0)
    );
}

/// **The test that decides where in the pipeline the function belongs.**
///
/// A ramp is a *sampling* of the shading's colour function, and ADR 0068's simplifier then drops
/// every stop within half an eight-bit level of the line its neighbours draw — so the linear grey
/// ramp above reaches the display list as two stops and nothing else. Applying a transfer function
/// to a finished ramp would therefore map two colours and let the rasteriser interpolate between
/// them, drawing a straight line wherever the clause asks for the function's own curve.
///
/// `/N 2` is that curve: §7.10.3 makes an exponential function `C0 + x^N × (C1 − C0)`, so this one
/// squares its input. The composition at the ramp's midpoint is 0.5² = 0.25; the chord between the
/// mapped endpoints is 0.5. A quarter of full scale is 64 levels of 255 — no rounding question.
#[test]
fn a_ramp_is_the_composition_and_not_its_endpoints() {
    let square = "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 2 >>";
    let ramp = painted_ramp(fixture(
        &format!("/ExtGState << /Curve << /TR {square} >> >> {RAMP}"),
        "/Curve gs /Sh sh",
        "",
    ));
    let middle = ramp.colour_at(0.5).r;
    assert!(
        (middle - 0.25).abs() < 8.0 * LEVEL,
        "the ramp samples the transfer of the shading's colour, not the chord between its \
         transferred ends: {middle} is not 0.25"
    );
    assert!(
        ramp.stops.len() > 2,
        "a curve needs more than the two stops the linear shading simplified to: {}",
        ramp.stops.len()
    );
}

/// §8.7.4.5.5's mesh states its colours at the vertices, and those are component values too.
///
/// The fixture is one type 4 triangle with `/BitsPerFlag 8`, `/BitsPerCoordinate 8` and
/// `/BitsPerComponent 8`, so a vertex is six bytes with no padding to compute. Under the inverting
/// function its black corner is white and its mid-grey corners are their complement.
#[test]
fn a_mesh_corner_colour_goes_through_the_transfer_function() {
    // flag, x, y, r, g, b — `/Decode` maps the coordinates one-for-one and the components to 0..1.
    // Every byte is below 0x80 so that the stream survives being written as text: `char::from`
    // above that is two UTF-8 bytes, which shifts every vertex after it.
    let vertices: [u8; 18] = [
        0, 0, 0, 0, 0, 0, //
        0, 100, 0, 127, 127, 127, //
        0, 0, 100, 127, 127, 127,
    ];
    let stream: String = vertices.iter().map(|byte| char::from(*byte)).collect();
    let mesh = format!(
        "5 0 obj\n<< /ShadingType 4 /ColorSpace /DeviceRGB /BitsPerCoordinate 8 \
         /BitsPerComponent 8 /BitsPerFlag 8 /Decode [0 255 0 255 0 1 0 1 0 1] /Length {} >>\n\
         stream\n{stream}\nendstream\nendobj\n",
        vertices.len()
    );
    let shading = painted_shading(fixture(
        &format!("/ExtGState << /Solid << /TR {INVERT} >> >> /Shading << /Sh 5 0 R >>"),
        "/Solid gs /Sh sh",
        &mesh,
    ));
    let ShadingKind::Mesh { triangles, .. } = shading.kind.as_ref() else {
        panic!("expected a mesh, got {:?}", shading.kind);
    };
    let corners = triangles
        .iter()
        .flat_map(|triangle| match triangle.corners {
            Corners::Colours(colours) => colours.into_iter(),
            Corners::Parameters(_) => panic!("a mesh with no /Function states colours"),
        })
        .map(|colour| colour.r)
        .collect::<Vec<_>>();
    assert!(
        corners.iter().any(|red| (red - 1.0).abs() < LEVEL),
        "the black corner is white under an inverting function: {corners:?}"
    );
    assert_eq!(corners.len(), 3, "one triangle, three corners: {corners:?}");
    assert!(
        corners
            .iter()
            .filter(|red| (*red - 1.0).abs() >= LEVEL)
            .all(|red| (red - (1.0 - 127.0 / 255.0)).abs() < LEVEL),
        "and the mid-grey corners are their own complement: {corners:?}"
    );
}

/// §8.7.4.5.2's function of two variables is evaluated per cell, and each cell is a colour.
///
/// The colours of a type 1 shading do not exist until a device says how large the domain will be
/// drawn, so the transfer travels with the producer and is applied as each cell is made. The
/// second half of the test is the reason it has to: a device *program* — §7.10.5's function lowered
/// to instructions `render-quorra` evaluates on the GPU — computes the colour and nothing else, so
/// there is nowhere on that path to put the function, and it is withdrawn.
#[test]
fn a_function_based_shading_maps_every_cell_and_withdraws_its_device_program() {
    // `{ pop dup dup }`: discard y, and give the x coordinate to all three components.
    let grey = "<< /FunctionType 4 /Domain [0 1 0 1] /Range [0 1 0 1 0 1] /Length 16 >>\n\
                stream\n{ pop dup dup }\nendstream";
    let shading = |state: &str| {
        format!(
            "/ExtGState << /Solid << /TR {state} >> >> \
             /Shading << /Sh << /ShadingType 1 /ColorSpace /DeviceRGB /Domain [0 1 0 1] \
             /Matrix [100 0 0 100 0 0] /Function 5 0 R >> >>"
        )
    };
    let extra = format!("5 0 obj\n{grey}\nendobj\n");

    let mapped = painted_shading(fixture(&shading(INVERT), "/Solid gs /Sh sh", &extra));
    let grid = mapped
        .sampled_at(Transform::IDENTITY, (16, 16))
        .expect("a type 1 shading resolves to a grid");
    let first = grid.pixels.first().expect("a grid has cells");
    let last = grid.pixels.last().expect("a grid has cells");
    assert!(
        first.r > last.r,
        "the function rises with x and the transfer inverts it, so the grid falls: \
         {first:?} then {last:?}"
    );
    assert!(
        mapped.device_program().is_none(),
        "a device evaluating the program would draw the untransferred colours"
    );

    let plain = painted_shading(fixture(&shading("/Identity"), "/Solid gs /Sh sh", &extra));
    let grid = plain
        .sampled_at(Transform::IDENTITY, (16, 16))
        .expect("a type 1 shading resolves to a grid");
    let first = grid.pixels.first().expect("a grid has cells");
    let last = grid.pixels.last().expect("a grid has cells");
    assert!(
        first.r < last.r,
        "with no function in force the grid rises: {first:?} then {last:?}"
    );
    assert!(
        plain.device_program().is_some(),
        "and the device may state the same colours for itself"
    );
}

/// §8.7.2 resolves a pattern's colour at the `scn`, and §10.5's function belongs to the mark.
///
/// The two sentences pull in opposite directions and ISO 32000-2 §11.6.7 settles it. A shading
/// pattern's *definition* is fixed before the `scn`:
///
/// > The definition shall not inherit the current values of the graphics state parameters at the
/// > time it is evaluated; those parameters shall take effect only when the resulting pattern is
/// > later used to paint an object.
///
/// but the painting operation is not the definition:
///
/// > This painting operation is subject to the values of the graphics state parameters in effect
/// > at the time, just as in painting an object with a constant colour.
///
/// and §11.7.5.2 puts the transfer function at "the last (topmost) elementary graphics object
/// enclosing that point" — the mark. So a file that states one function at the `scn` and another
/// at the `f` is painted with the **second**, and the ramp below is read both ways round to say
/// so. Reported rather than drawn until the six-hundred-and-sixtieth session.
#[test]
fn a_pattern_is_painted_under_the_transfer_function_the_mark_states() {
    let resources = format!(
        "/ExtGState << /On << /TR {INVERT} >> /Off << /TR /Identity >> >> \
         /Pattern << /P << /PatternType 2 /Shading << /ShadingType 2 /ColorSpace /DeviceRGB \
         /Coords [0 0 100 100] /Function << /FunctionType 2 /Domain [0 1] /C0 [0 0 0] \
         /C1 [1 1 1] /N 1 >> /Extend [true true] >> >> >>"
    );

    // Selected under the inverting function and painted with none: the mark's answer is the
    // shading's own colours, black at one end and white at the other.
    let dropped = painted_ramp(fixture(
        &resources,
        "/On gs /Pattern cs /P scn /Off gs 0 0 100 100 re f",
        "",
    ));
    assert!(
        dropped.colour_at(0.0).r < LEVEL && (dropped.colour_at(1.0).r - 1.0).abs() < LEVEL,
        "the function was turned off before the mark, so the pattern paints its own colours: \
         {:?} {:?}",
        dropped.colour_at(0.0),
        dropped.colour_at(1.0)
    );

    // And the other way round: selected under no function and painted under the inverting one.
    let gained = painted_ramp(fixture(
        &resources,
        "/Off gs /Pattern cs /P scn /On gs 0 0 100 100 re f",
        "",
    ));
    assert!(
        (gained.colour_at(0.0).r - 1.0).abs() < LEVEL && gained.colour_at(1.0).r < LEVEL,
        "the function stated at the mark reaches the pattern's colours: {:?} {:?}",
        gained.colour_at(0.0),
        gained.colour_at(1.0)
    );

    // Nothing is left for §10.5 to report about a pattern, and the page below is the one that
    // used to raise it.
    let reports = transfer_reports(fixture(
        &resources,
        "/On gs /Pattern cs /P scn /Off gs 0 0 100 100 re f",
        "",
    ));
    assert!(
        reports.is_empty(),
        "the departure is closed rather than named: {reports:?}"
    );
}
