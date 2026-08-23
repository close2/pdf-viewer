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
//!   otherwise. Half of that is a per-object rule in disguise and is implemented: an object the
//!   clause does not call fully opaque is never the one whose function is chosen anywhere, so it
//!   is handed the page's default — which is the identity on this device — and the tests below
//!   pin that for a `ca`, for an ancestry and for a soft mask's group. What is left needs a
//!   *point*: a fully opaque transferred mark seen through a later translucent one, where the
//!   clause maps the whole composite and this tree has already mapped the colour underneath.
//!   That one is reported rather than drawn, and `doc/todo/13` prices it.
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

/// Every solid fill colour the page paints, in the order the display list holds them.
///
/// Recursive, because a transparency group's marks are a [`Command::Group`]'s elements rather than
/// commands of the page — and the clause under test is precisely about what a group does to the
/// marks inside it.
fn filled_colours(bytes: Vec<u8>) -> Vec<pdf_render::Color> {
    fn walk(commands: &[Command], into: &mut Vec<pdf_render::Color>) {
        for command in commands {
            match command {
                Command::Fill {
                    paint: Paint::Solid(colour),
                    ..
                } => into.push(*colour),
                Command::Group { commands, .. } => walk(commands, into),
                _ => {}
            }
        }
    }

    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    let mut colours = Vec::new();
    walk(interpretation.display_list.commands(), &mut colours);
    colours
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

/// A mark the clause does not call fully opaque is drawn with the page's **default** function.
///
/// ISO 32000-2 §11.7.5.2 says which object decides, and what happens when none does:
///
/// > The halftone and transfer function to be used at any given point on the page shall be those
/// > in effect at the time of painting the last (topmost) elementary graphics object enclosing
/// > that point, but only if the object is fully opaque.
///
/// > For portions of the page whose topmost object is not fully opaque or that are never painted
/// > at all, the default halftone and transfer function for the page shall be used
///
/// Those two sentences decide this test between them and the deduction is three lines. At a point,
/// the function is the topmost enclosing object's — and only where that object is fully opaque —
/// or else the page's default. Take an object *O* that is not fully opaque and any point it
/// encloses: either *O* is topmost there, and the first sentence withholds its function in favour
/// of the second's default, or something else is topmost there, and that object's function or the
/// default is chosen instead. Neither branch can choose *O*'s. **So a mark that is not fully
/// opaque has its own transfer function used at no point on the page at all**, and the colour it
/// contributes reaches the device unmapped.
///
/// The page's default is Table 52's "a PDF reader shall initialise this to a suitable device
/// dependent value", and for this device it is the identity: §10.5's own subject is a function
/// that "adjusts device gray or colour component levels to compensate for nonlinear response in a
/// particular output device", and nothing this program puts between a `Command::Fill`'s colour and
/// the raster is such a response. That is the documented choice `doc/todo/13` records, not a
/// silence.
///
/// The mutation is the same page with the `ca` taken off, where the mark *is* fully opaque and the
/// first sentence hands it its own function.
#[test]
fn a_translucent_marks_own_transfer_function_is_used_at_no_point() {
    let resources =
        format!("/ExtGState << /Solid << /TR {INVERT} >> /Half << /TR {INVERT} /ca 0.5 >> >>");

    let translucent = filled_colours(fixture(
        resources.as_str(),
        "/Half gs 1 0 0 rg 0 0 50 50 re f",
        "",
    ));
    let painted = *translucent.first().expect("the page paints one fill");
    assert!(
        (painted.r - 1.0).abs() < LEVEL && painted.g < LEVEL && painted.b < LEVEL,
        "the clause puts the page's default — the identity — on a mark that is not fully \
         opaque, so red stays red: {painted:?}"
    );
    assert!(
        (painted.a - 0.5).abs() < LEVEL,
        "§11.6.4.4's constant alpha is a different parameter and is untouched: {painted:?}"
    );

    let opaque = filled_colours(fixture(
        resources.as_str(),
        "/Solid gs 1 0 0 rg 0 0 50 50 re f",
        "",
    ));
    let painted = *opaque.first().expect("the page paints one fill");
    assert!(
        painted.r < LEVEL && (painted.g - 1.0).abs() < LEVEL && (painted.b - 1.0).abs() < LEVEL,
        "and a fully opaque mark is handed its own function, which inverts red to cyan: \
         {painted:?}"
    );
}

/// A page stating a transfer function and painting one translucent mark **over** an opaque one.
///
/// ISO 32000-2 §11.7.5.2:
///
/// > For portions of the page whose topmost object is not fully opaque or that are never painted
/// > at all, the default halftone and transfer function for the page shall be used
///
/// One stated function and one `ca` below 1.0 is the whole of the condition — no second function
/// competing with a first, which is what the ledger's row claimed for two hundred sessions.
///
/// **What is left to report is one shape, and it takes two marks.** The clause composites raw
/// colours and maps the result once, per point, with the topmost object's function; this tree maps
/// each fully opaque contributor's colour before compositing. Where the topmost object is fully
/// opaque the two are the same picture, because the six conditions "ensure that only the object
/// itself shall contribute to the colour at the given point". Where it is not, this tree's
/// composite is right unless something *underneath* it was fully opaque and carried a function —
/// which is the red rectangle below, seen through the blue one. The colours the two marks carry
/// are asserted beside the report, because the report is only worth what the picture under it is.
///
/// The mutation is the same page painted opaque throughout, where the clause and this tree agree.
#[test]
fn a_transferred_opaque_mark_seen_through_a_translucent_one_is_reported() {
    let resources = format!("/ExtGState << /Solid << /TR {INVERT} >> /Half << /ca 0.5 >> >>");
    let content = "/Solid gs 1 0 0 rg 0 0 50 50 re f /Half gs 0 0 1 rg 10 10 50 50 re f";

    let translucent = transfer_reports(fixture(&resources, content, ""));
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

    let colours = filled_colours(fixture(&resources, content, ""));
    let [under, over] = colours.as_slice() else {
        panic!("the page paints two fills: {colours:?}");
    };
    assert!(
        under.r < LEVEL && (under.g - 1.0).abs() < LEVEL && (under.b - 1.0).abs() < LEVEL,
        "the opaque mark is topmost where the other misses it, so it keeps its own \
         function: {under:?}"
    );
    assert!(
        over.r < LEVEL && over.g < LEVEL && (over.b - 1.0).abs() < LEVEL,
        "and the translucent mark over it is drawn with the page's default: {over:?}"
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
/// fully opaque by the clause. A rule reading the mark alone would hand it its own function here,
/// which is exactly the nested case §11.7.5.2 spends four of its six conditions on — and the mark
/// is the only object on the page, so the clause's answer at every point it encloses is the page's
/// default and there is nothing left to report.
///
/// The mutation is the same group invoked with no `ca`, where all six conditions hold.
#[test]
fn a_group_invoked_translucently_takes_the_transfer_function_off_the_marks_inside() {
    let resources = "/ExtGState << /Half << /ca 0.5 >> >> /XObject << /Fm 5 0 R >>";
    let inner = "/Inner gs 1 0 0 rg 0 0 50 50 re f";
    let form = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /CS /DeviceRGB >> \
         /Resources << /ExtGState << /Inner << /TR {INVERT} >> >> >> /Length {} >>\n\
         stream\n{inner}\nendstream\nendobj\n",
        inner.len() + 1
    );

    let translucent = filled_colours(fixture(resources, "/Half gs /Fm Do", &form));
    let painted = *translucent.first().expect("the group paints one fill");
    assert!(
        (painted.r - 1.0).abs() < LEVEL && painted.g < LEVEL && painted.b < LEVEL,
        "the ancestry is what makes this mark not fully opaque, so its own function is not \
         the one the clause chooses: {painted:?}"
    );
    assert!(
        transfer_reports(fixture(resources, "/Half gs /Fm Do", &form)).is_empty(),
        "and with the page drawn as the clause asks there is nothing to report"
    );

    let opaque = filled_colours(fixture(resources, "/Fm Do", &form));
    let painted = *opaque.first().expect("the group paints one fill");
    assert!(
        painted.r < LEVEL && (painted.g - 1.0).abs() < LEVEL && (painted.b - 1.0).abs() < LEVEL,
        "the same group invoked opaquely satisfies every one of the six: {painted:?}"
    );
}

/// A mark inside a **soft mask's** group is not painted at a point on the page at all.
///
/// ISO 32000-2 §11.7.5.2 is addressed throughout to "any given point on the page", and §11.5.3
/// says what a mask's group becomes instead:
///
/// > The mask value at any given point shall then be defined to be the luminosity of the resulting
/// > colour.
///
/// So no mark inside it is ever "the topmost elementary object in the entire page stack" at any
/// point of the page, and by the same deduction as the translucent mark above, its own transfer
/// function is chosen nowhere. Two more sentences say the same thing from other directions:
/// §11.5.3 makes the luminosity of a device colour a conversion "with no compensation for gamma or
/// other colour calibration", which is precisely what Table 52 calls a transfer function; and
/// §11.7.5.3's NOTE puts the moment beyond reach as well as the place — it says of the current
/// halftone and transfer function that they are the parameters "whose values are used only when
/// all colour compositing has been completed and rasterization is being performed", where a mask's
/// group is resolved before the page's compositing begins.
///
/// The mutation is the same group painted by a `Do` instead of named by an `/SMask`, where its
/// mark *is* an object on the page and takes the function it states.
#[test]
fn a_mark_inside_a_soft_masks_group_is_not_transferred() {
    let inner = "/Inner gs 1 0 0 rg 0 0 100 100 re f";
    let form = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /CS /DeviceRGB >> \
         /Resources << /ExtGState << /Inner << /TR {INVERT} >> >> >> /Length {} >>\n\
         stream\n{inner}\nendstream\nendobj\n",
        inner.len() + 1
    );

    let document = Document::open(fixture(
        "/ExtGState << /M << /SMask << /S /Luminosity /G 5 0 R >> >> >> /XObject << /Fm 5 0 R >>",
        "/M gs 0 0 1 rg 0 0 100 100 re f",
        &form,
    ))
    .expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    let list = &interpretation.display_list;
    let mask = list
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Fill { mask: Some(id), .. } => list.soft_mask(*id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the page paints through a mask: {:?}", list.commands()));
    let inside = mask
        .commands
        .iter()
        .find_map(|command| match command {
            Command::Fill {
                paint: Paint::Solid(colour),
                ..
            } => Some(*colour),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the mask's group paints one fill: {:?}", mask.commands));
    assert!(
        (inside.r - 1.0).abs() < LEVEL && inside.g < LEVEL && inside.b < LEVEL,
        "the mask's own luminosity is computed from the colours the group states, unmapped: \
         {inside:?}"
    );

    let painted = filled_colours(fixture("/XObject << /Fm 5 0 R >>", "/Fm Do", &form));
    let painted = *painted.first().expect("the group paints one fill");
    assert!(
        painted.r < LEVEL && (painted.g - 1.0).abs() < LEVEL && (painted.b - 1.0).abs() < LEVEL,
        "the same group painted onto the page takes the function it states: {painted:?}"
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

/// The colour of the one solid fill a page paints, or a panic if it paints anything else.
fn filled_colour(bytes: Vec<u8>) -> pdf_render::Color {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    interpretation
        .display_list
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Fill {
                paint: Paint::Solid(colour),
                ..
            } => Some(*colour),
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!(
                "the page paints no solid fill: {:?}",
                interpretation.unsupported
            )
        })
}

/// A §7.10.3 exponential that halves its input: `/C0 [0] /C1 [0.5] /N 1`.
///
/// Distinguishable from [`INVERT`] at every input, which is what makes an override legible: 1.0
/// through this is 0.5, and through [`INVERT`] it is 0.0.
const HALVE: &str = "<< /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [0.5] /N 1 >>";

/// ISO 32000-2 §10.5's **second** bullet: a halftone dictionary's `TransferFunction` wins.
///
/// > The current halftone parameter in the graphics state may specify transfer functions as
/// > optional entries in halftone dictionaries (see 10.6.5, "Halftone dictionaries"). … A transfer
/// > function specified in a halftone dictionary shall override the corresponding one specified by
/// > the current transfer function parameter in the graphics state.
///
/// **This is a screen's business even though §10.6's screens are not**, and the standard separates
/// them itself. §10.1 lists the rendering steps with only one of the two conditional on the device
/// — "[i]f the raster output device supports PDF-defined halftoning, apply halftoning according to
/// 10.6" against an unqualified "[f]or any object for which transfer functions are in effect, apply
/// those transfer functions" — and §10.6.1 says what a device needing no screen still owes:
/// "[h]alftoning is not required for such devices; after gamma correction by the transfer
/// functions, the colour components shall be transmitted directly to the device."
///
/// The expected values are the functions' own arithmetic. §7.10.3 makes an exponential
/// `C0 + x^N × (C1 − C0)`, so white through `HALVE` is 0.5 and white through `INVERT` is 0.0; the
/// page states both, one in `/TR` and one in the halftone, and 0.5 is the only answer that says the
/// halftone's won. The mutation is the same page with the halftone's entry removed, which leaves
/// `/TR` alone and gives 0.0.
#[test]
fn a_halftone_dictionarys_transfer_function_overrides_the_graphics_states() {
    let halftone = format!(
        "/HT << /Type /Halftone /HalftoneType 1 /Frequency 60 /Angle 45 \
         /SpotFunction /Round /TransferFunction {HALVE} >>"
    );
    let overridden = filled_colour(fixture(
        &format!("/ExtGState << /G << /TR {INVERT} {halftone} >> >>"),
        "/G gs 1 1 1 rg 0 0 100 100 re f",
        "",
    ));
    assert!(
        (overridden.r - 0.5).abs() < LEVEL
            && (overridden.g - 0.5).abs() < LEVEL
            && (overridden.b - 0.5).abs() < LEVEL,
        "the halftone's function decides every component: {overridden:?}"
    );

    let alone = filled_colour(fixture(
        &format!("/ExtGState << /G << /TR {INVERT} >> >>"),
        "/G gs 1 1 1 rg 0 0 100 100 re f",
        "",
    ));
    assert!(
        alone.r < LEVEL,
        "with no halftone the graphics state's function stands: {alone:?}"
    );
}

/// `TransferFunction /Identity` is an override, not a silence — and it is what the crawl states.
///
/// Table 128: "( Optional ) A transfer function, which overrides the current transfer function in
/// the graphics state for the same component. … The name Identity may be used to specify the
/// identity function." So a halftone naming `Identity` **replaces** the `/TR` in force with the
/// identity, which leaves the colour alone; a halftone with no `TransferFunction` at all leaves the
/// `/TR` running. The two are one keyword apart in the file and a level apart on the page.
///
/// `examples/transfer_function_census` over the `SafeDocs` crawl finds three documents carrying a
/// `TransferFunction` and every one of them names `Identity`, so this is the shape that exists
/// rather than the one that is convenient — none of the three also states a `/TR`, which is why the
/// fixture has to state both (trap 8).
#[test]
fn a_halftone_naming_identity_turns_the_graphics_states_function_off() {
    let content = "/On gs /Half gs 1 1 1 rg 0 0 100 100 re f";
    let off = filled_colour(fixture(
        &format!(
            "/ExtGState << /On << /TR {INVERT} >> \
             /Half << /HT << /HalftoneType 1 /TransferFunction /Identity >> >> >>"
        ),
        content,
        "",
    ));
    assert!(
        (off.r - 1.0).abs() < LEVEL,
        "an identity override leaves white white: {off:?}"
    );

    // The mutation: the same halftone with no `TransferFunction`, which says nothing about any
    // component and leaves the `/TR` in force.
    let silent = filled_colour(fixture(
        &format!(
            "/ExtGState << /On << /TR {INVERT} >> \
             /Half << /HT << /HalftoneType 1 /Frequency 60 >> >> >>"
        ),
        content,
        "",
    ));
    assert!(
        silent.r < LEVEL,
        "a halftone that says nothing does not override: {silent:?}"
    );
}

/// A Type 5 halftone names its colourants, and each one governs its own component.
///
/// §10.6.5.6 lists the primaries of each native space — "Red , Green , and Blue for `DeviceRGB` " —
/// and Table 132 makes `Default` "[a] halftone that shall be used for any colourant or colour
/// component that does not have an entry of its own". So a dictionary naming `/Red` and `/Default`
/// governs red by the first and green and blue by the second, and §10.5's "[e]ach colour component
/// shall have its own separate transfer function; there shall not be interaction between
/// components" is what makes reading the three separately correct rather than merely possible.
///
/// White in: red through `HALVE` is 0.5, green and blue through `INVERT` are 0.0.
#[test]
fn a_type_5_halftone_gives_each_colourant_its_own_function() {
    let painted = filled_colour(fixture(
        &format!(
            "/ExtGState << /G << /HT << /Type /Halftone /HalftoneType 5 \
             /Red << /HalftoneType 1 /TransferFunction {HALVE} >> \
             /Default << /HalftoneType 1 /TransferFunction {INVERT} >> >> >> >>"
        ),
        "/G gs 1 1 1 rg 0 0 100 100 re f",
        "",
    ));
    assert!(
        (painted.r - 0.5).abs() < LEVEL,
        "/Red governs the red component: {painted:?}"
    );
    assert!(
        painted.g < LEVEL && painted.b < LEVEL,
        "/Default governs the two with no entry of their own: {painted:?}"
    );
}

/// `/HT /Default` restores the device's halftone, and with it the graphics state's own function.
///
/// Table 57 defines the name: "the halftone that was in effect at the start of the page". Table 52
/// says what that is — "a PDF reader shall initialise this to a suitable device dependent value" —
/// and a halftone belonging to the device is not one of the "halftone dictionaries" §10.5's second
/// bullet reads a `TransferFunction` out of. So the override comes off and the first bullet's `/TR`,
/// which no `gs` here has touched, is in force again: white through `INVERT` is black.
///
/// The two parameters are independent, which is the whole reason this reader keeps them apart —
/// `/HT /Default` must not clear a `/TR`, and `/TR /Identity` must not clear an `/HT`.
#[test]
fn a_default_halftone_takes_its_override_off_and_leaves_the_transfer_function() {
    let resources = format!(
        "/ExtGState << /On << /TR {INVERT} >> \
         /Half << /HT << /HalftoneType 1 /TransferFunction {HALVE} >> >> \
         /Device << /HT /Default >> \
         /Off << /TR /Identity >> >>"
    );
    let restored = filled_colour(fixture(
        &resources,
        "/On gs /Half gs /Device gs 1 1 1 rg 0 0 100 100 re f",
        "",
    ));
    assert!(
        restored.r < LEVEL,
        "the halftone's override is gone and /TR is not: {restored:?}"
    );

    // And the other independence: `/TR /Identity` clears the first bullet and leaves the second.
    let halftone_only = filled_colour(fixture(
        &resources,
        "/On gs /Half gs /Off gs 1 1 1 rg 0 0 100 100 re f",
        "",
    ));
    assert!(
        (halftone_only.r - 0.5).abs() < LEVEL,
        "the halftone's function survives /TR /Identity: {halftone_only:?}"
    );
}
