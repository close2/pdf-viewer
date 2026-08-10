//! Transparency group `XObject`s: ISO 32000-2 §11.4 and §11.6.6.
//!
//! # Why this file exists
//!
//! A group is the one construction in clause 11 whose effect cannot be seen in a single
//! command. `ca` on an ordinary form reaches every object the form paints; `ca` on a
//! *group* reaches the group's composited result, once — so two overlapping opaque
//! rectangles under `/ca 0.5` are a single translucent shape rather than two, and the
//! difference is a band of doubled colour where they overlap. That band is what these
//! tests look for, at the pixel, because a display list holding the right commands under
//! the wrong nesting is indistinguishable from a correct one until something composites.
//!
//! The other half is §8.10.1's step c), which clips a form to its own `/BBox`, and which
//! this tree did for an annotation's appearance and not for a form invoked by `Do`. §11.6.6
//! needs it — a group's shape is what it painted, clipped by the group's bounding box — and
//! it is a requirement of every form either way.
//!
//! §11.4.6's knockout is drawn rather than reported — with the element's shape stated apart
//! from its alpha where the two differ (ADR 0234) — and the tests below measure it against
//! the clause's own two-stage arithmetic at the pixel. What is still *not* implemented is
//! reported: a non-isolated group whose elements blend, a group blending colour space that is
//! not the device's, and the knockout elements whose shape one alpha channel cannot be
//! separated from. The conditions those reports fire on are pinned below, because a report
//! that names a page where the output cannot differ costs that page its place in the oracle's
//! comparison (see `doc/HANDOVER.md`, trap 11).

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and this page is 100 units \
              square where no arithmetic can overflow"
)]

use std::fmt::Write as _;

use pdf_render::{Command, Paint, Rasterizer, TargetSpec};
use pdf_syntax::Document;

/// A one-page fixture drawing a form `XObject` named `/Fm`.
///
/// `group` is the form's `/Group` entry, written whole so a test can leave it out; `bbox`
/// is the form's `/BBox`; `form` is the form's content stream and `page` the page's.
fn fixture(group: &str, bbox: &str, form: &str, page: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /ExtGState << /GS << /ca 0.5 /CA 0.5 >> /GB << /BM /Multiply >> \
         /GM << /SMask << /S /Luminosity /G 6 0 R >> >> \
         /GA << /AIS true /SMask << /S /Luminosity /G 6 0 R >> >> >> \
         /Shading << /Sh 8 0 R >> \
         /XObject << /Fm 5 0 R /In 7 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox {bbox} {group} /Length {} >>\n\
         stream\n{form}\nendstream\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /CS /DeviceGray >> /Length {} >>\n\
         stream\n{MASK}\nendstream\nendobj\n\
         7 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /I true >> /Length {} >>\n\
         stream\n{INNER}\nendstream\nendobj\n\
         8 0 obj\n<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 0 100 0] \
         /Extend [true true] /Function << /FunctionType 2 /Domain [0 1] \
         /C0 [0 0 1] /C1 [0 0 1] /N 1 >> >>\nendobj\n",
        page.len() + 1,
        form.len() + 1,
        MASK.len() + 1,
        INNER.len() + 1
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
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

/// Interprets a fixture, returning its display list and what it could not draw.
fn interpret(bytes: Vec<u8>) -> pdf_model::Interpretation {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page)
}

/// The RGBA pixel at `(x, y)` of the fixture rendered at one pixel per unit.
///
/// The page is 100 units square and the raster is 100 pixels square, with PDF's y-up
/// origin flipped by the target transform, so a caller states device coordinates.
fn pixel(interpretation: &pdf_model::Interpretation, x: u32, y: u32) -> [u8; 4] {
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("a 100x100 target");
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the fixture rasterises");
    let at = ((y * raster.width + x) * 4) as usize;
    let bytes = &raster.data[at..at + 4];
    [bytes[0], bytes[1], bytes[2], bytes[3]]
}

/// The luminosity group object 6 holds, for the `/GM` graphics state's soft mask.
///
/// A grey wedge rather than a constant, so that a mask applied where none was intended
/// changes the picture: §11.6.5.1 makes the mask value the group's luminosity.
const MASK: &str = "0.5 g 0 0 100 100 re f";

/// The isolated transparency group object 7 holds, drawn by the `/In` name.
///
/// One opaque blue square, so that a group used as an *element* of a knockout group has a
/// shape — the union of its elements' — that is plainly not the alpha it is painted at.
const INNER: &str = "0 0 1 rg 30 30 50 50 re f";

/// Two overlapping opaque red squares, drawn inside the form.
///
/// Opaque and the same colour, so that any difference between the models is *only* about
/// how the two are combined: under the group model the pair is one shape painted at half
/// alpha, and under the per-object one the overlap is painted twice.
const TWO_SQUARES: &str = "1 0 0 rg 10 10 50 50 re f 30 30 50 50 re f";

/// §11.4.1: a group is composited to one colour and opacity, then painted once.
///
/// > A transparency group is a sequence of consecutive objects in a transparency stack that
/// > shall be collected together and composited to produce a single colour, shape, and
/// > opacity at each point.
///
/// The measurement is the overlap. Two opaque red squares under `/ca 0.5` cover white: as
/// one object the whole union is `0.5` red over white, and as two objects the band they
/// share is painted twice — `0.75` — which is a visibly darker rectangle in the middle of
/// the shape. Nothing short of a pixel can tell the two apart, which is why this test
/// rasterises rather than reading the display list.
#[test]
fn a_group_is_composited_once_rather_than_object_by_object() {
    let grouped = interpret(fixture(
        "/Group << /S /Transparency /I true >>",
        "[0 0 100 100]",
        TWO_SQUARES,
        "/GS gs /Fm Do",
    ));
    assert!(grouped.is_complete(), "{:?}", grouped.unsupported);

    // Device y = 55 is page y = 45, inside the lower square only; y = 35 is page y = 65,
    // inside the upper square only; y = 45 is page y = 55, inside both.
    let alone = pixel(&grouped, 20, 55);
    let overlap = pixel(&grouped, 40, 45);
    assert_eq!(
        alone, overlap,
        "the overlap of two objects of one group is painted once, not twice"
    );
    assert_eq!(
        alone,
        [255, 127, 127, 255],
        "half of an opaque red over white is a light red"
    );

    // The same content with no `/Group` is an ordinary form, and there the constant does
    // reach each object: this is the behaviour the group changes, stated as a control so
    // that a fixture drawing nothing at all cannot pass the assertion above.
    let plain = interpret(fixture("", "[0 0 100 100]", TWO_SQUARES, "/GS gs /Fm Do"));
    assert_ne!(
        pixel(&plain, 20, 55),
        pixel(&plain, 40, 45),
        "without a group each object carries the constant separately"
    );
}

/// §11.6.6: the transparency parameters are reset for the group's own content.
///
/// > Before execution of the transparency group XObject's content stream, the current blend
/// > mode in the graphics state shall be initialised to Normal , the current stroking and
/// > nonstroking alpha constants to 1.0, and the current soft mask to None .
///
/// Its NOTE 1 says why: those parameters belong to the group as a whole, and leaving them in
/// force would apply them twice. So the elements inside come out opaque and Normal however
/// the caller had set them, and the `Command::Group` carries the caller's values instead.
#[test]
fn a_group_resets_the_alpha_constants_and_the_blend_mode_for_its_elements() {
    let interpretation = interpret(fixture(
        "/Group << /S /Transparency /I true >>",
        "[0 0 100 100]",
        TWO_SQUARES,
        "/GS gs /GB gs /Fm Do",
    ));

    let [
        Command::Group {
            commands,
            alpha,
            blend,
            ..
        },
    ] = interpretation.display_list.commands()
    else {
        panic!(
            "expected one group command, got {:?}",
            interpretation.display_list.commands()
        );
    };

    assert!(
        (alpha - 0.5).abs() < 1e-6,
        "the group carries `ca` from `Do`"
    );
    assert_eq!(
        *blend,
        pdf_render::BlendMode::Multiply,
        "and the blend mode from `Do`"
    );
    assert_eq!(commands.len(), 2, "both squares are elements of the group");
    for command in commands {
        let Command::Fill { paint, blend, .. } = command else {
            panic!("expected fills inside the group, got {command:?}");
        };
        assert_eq!(
            *blend,
            pdf_render::BlendMode::Normal,
            "an element's blend mode is reset"
        );
        let Paint::Solid(colour) = paint else {
            panic!("expected a solid paint");
        };
        assert!(
            (colour.a - 1.0).abs() < 1e-6,
            "an element's alpha constant is reset to 1.0, was {}",
            colour.a
        );
    }
}

/// §11.6.6: only a `/Group` whose subtype is `/Transparency` groups anything.
///
/// > An ordinary form XObject -one having no Group entry -or having a Group entry with a
/// > subtype other than Transparency -shall not be subject to any grouping behaviour for
/// > transparency purposes.
///
/// The second half is the one an implementation drops, because `/S` is the only entry that
/// distinguishes the two and no corpus document writes another subtype.
#[test]
fn a_form_becomes_a_group_only_for_the_transparency_subtype() {
    let groups = |group: &str| {
        interpret(fixture(group, "[0 0 100 100]", TWO_SQUARES, "/Fm Do"))
            .display_list
            .commands()
            .iter()
            .filter(|command| matches!(command, Command::Group { .. }))
            .count()
    };

    assert_eq!(groups("/Group << /S /Transparency /I true >>"), 1);
    assert_eq!(groups("/Group << /S /Fictional >>"), 0);
    assert_eq!(groups("/Group << /I true >>"), 0, "no subtype, no group");
    assert_eq!(groups(""), 0, "an ordinary form is not a group");
    // §11.4.4's NOTE 5: a *non-isolated* group composited with Normal, alpha 1.0 and no
    // mask is "the same as … compositing them separately (without grouping)", so there is
    // no group in the list — which is a different reason for zero from the three above and
    // is why the positive case now states `/I true`.
    assert_eq!(
        groups("/Group << /S /Transparency >>"),
        0,
        "a non-isolated group with a trivial composite is flattened"
    );
}

/// §8.10.1 lists what `Do` performs on a form `XObject`, and step c) is:
///
/// > Clips according to the form dictionary's BBox entry
///
/// §8.10.2's Table 93 says the same of the entry itself:
///
/// > These boundaries shall be used to clip the form XObject and to determine its size for
/// > caching.
///
/// This tree honoured it for an annotation's appearance only, where §12.5.5's placement
/// algorithm made it unavoidable. A form invoked by `Do` drew outside its own box, which is
/// a page marked where the producer said nothing would be.
#[test]
fn a_forms_bounding_box_clips_what_it_draws() {
    let clipped = interpret(fixture(
        "",
        "[0 0 50 50]",
        "1 0 0 rg 0 0 100 100 re f",
        "/Fm Do",
    ));
    assert!(clipped.is_complete(), "{:?}", clipped.unsupported);

    // Device (10, 90) is page (10, 10) — inside the box — and (60, 40) is page (60, 60),
    // outside it but inside the rectangle the form fills.
    assert_eq!(
        pixel(&clipped, 10, 90),
        [255, 0, 0, 255],
        "inside the box the form paints"
    );
    assert_eq!(
        pixel(&clipped, 60, 40),
        [255, 255, 255, 255],
        "outside the box it does not"
    );
}

/// §11.4.6's knockout is *drawn*: only the topmost element contributes.
///
/// > In a knockout group, each individual element shall be composited with the group's
/// > initial backdrop rather than with the stack of preceding elements in the group. …
/// > At any given point, only the topmost object enclosing the point shall contribute to
/// > the result colour and opacity of the group as a whole.
///
/// The measurement is the overlap of a half-transparent blue over an opaque red, drawn onto
/// a white page. Under knockout the red is not there for the blue to composite with, so the
/// overlap is half blue over *white*; under the ordinary model it is half blue over red.
/// The two are as far apart as a red channel can be, and nothing but a pixel distinguishes
/// them — the display list holds the same four commands either way.
#[test]
fn a_knockout_group_paints_only_its_topmost_element() {
    let form = "1 0 0 rg 10 10 50 50 re f /GS gs 0 0 1 rg 30 30 50 50 re f";
    let knocked = interpret(fixture(
        "/Group << /S /Transparency /I true /K true >>",
        "[0 0 100 100]",
        form,
        "/Fm Do",
    ));
    assert!(knocked.is_complete(), "{:?}", knocked.unsupported);
    let ordinary = interpret(fixture(
        "/Group << /S /Transparency /I true >>",
        "[0 0 100 100]",
        form,
        "/Fm Do",
    ));

    // Device (40, 50) is page (40, 50): inside both squares.
    assert_eq!(
        pixel(&knocked, 40, 50),
        [127, 127, 255, 255],
        "half blue over the page, because the red under it was knocked out"
    );
    assert_eq!(
        pixel(&ordinary, 40, 50),
        [127, 0, 128, 255],
        "and half blue over the red, when it is not"
    );
    // Where only the lower element is, both models paint it.
    assert_eq!(pixel(&knocked, 15, 85), [255, 0, 0, 255]);
    assert_eq!(pixel(&ordinary, 15, 85), [255, 0, 0, 255]);
}

/// §11.6.2: a path filled *and* stroked by one operator composites once.
///
/// > Single graphics objects … shall be treated as elementary objects for transparency
/// > compositing purposes … Portions of an object shall not be composited with one another,
/// > even if they are described in a way that would seem to cause overlaps (such as a
/// > self-intersecting path, combined fill and stroke of a path, or a shading pattern
/// > containing an overlap or fold-over).
///
/// `B` fills and then strokes, and a stroke straddles its path — so the inner half of the
/// stroke covers the fill. At half alpha that band came out at 0.75 where the clause asks
/// for 0.5, which is the whole difference and is invisible to everything but a pixel.
///
/// The construction is §11.4.6's, and it is the clause's own: "[a]t any given point, only
/// the topmost object enclosing the point shall contribute", the topmost portion here being
/// the stroke, because `B` strokes second.
#[test]
fn a_filled_and_stroked_path_is_one_object() {
    let painted = interpret(fixture(
        "",
        "[0 0 100 100]",
        "",
        "/GS gs 1 0 0 rg 1 0 0 RG 10 w 30 30 40 40 re B",
    ));
    assert!(painted.is_complete(), "{:?}", painted.unsupported);

    // Device (35, 65) is page (35, 35): inside the stroke's inner half, which the fill also
    // covers. Once at 0.5 over white is (255, 127, 127); twice would be (255, 63, 63).
    assert_eq!(
        pixel(&painted, 35, 65),
        [255, 127, 127, 255],
        "the band the two portions share is painted once"
    );
    // The fill's interior, which only one portion covers, is the same colour.
    assert_eq!(pixel(&painted, 50, 50), [255, 127, 127, 255]);
}

/// §11.4.6's knockout is reported where this renderer cannot *state* an element's shape.
///
/// The clause states the reason itself:
///
/// > The existence of the knockout feature is the main reason for maintaining a separate
/// > shape value rather than only a single alpha that combines shape and opacity.
///
/// A raster of premultiplied samples has one alpha and no shape, so knockout used to reach
/// the display list only where the two coincide. Since ADR 0234 it also reaches it where the
/// shape can be stated *beside* the object — a soft mask and a constant alpha are
/// §11.6.4.3's and §11.6.4.4's opacity, so the shape is the mark without them, and a group's
/// is the union of its elements'. What is left is where one alpha genuinely carries both:
/// an image's samples, which may be §8.9.6.2's stencil or §11.6.5.2's `/SMask`, and a shading
/// whose colours already carry §11.6.4.4's constant.
///
/// The second half of the condition is older and still stands: where the upper of two
/// elements is opaque and blends Normal it overwrites the lower one under either model, and
/// where two elements do not overlap there is nothing to knock out. A report on `/K true`
/// alone would name every knockout group in the corpus for a difference most of them cannot
/// show, which is the mistake §9.3.8's first draft made.
#[test]
fn a_knockout_group_reports_only_where_the_two_models_differ() {
    let reported = |group: &str, form: &str| {
        format!(
            "{:?}",
            interpret(fixture(group, "[0 0 100 100]", form, "/Fm Do")).unsupported
        )
    };
    let knockout = "/Group << /S /Transparency /I true /K true >>";

    assert!(
        !reported(
            knockout,
            "/GS gs 1 0 0 rg 10 10 50 50 re f 30 30 50 50 re f"
        )
        .contains("knockout"),
        "two fills have a shape a rasteriser draws, so the clause's rule is drawn"
    );
    assert!(
        !reported(knockout, TWO_SQUARES).contains("knockout"),
        "an opaque element under the Normal blend mode overwrites either way"
    );
    assert!(
        !reported(
            knockout,
            "/GS gs 1 0 0 rg 10 10 20 20 re f 60 60 20 20 re f"
        )
        .contains("knockout"),
        "elements that do not overlap have nothing to knock out"
    );
    assert!(
        !reported(
            "/Group << /S /Transparency /I true >>",
            "/GS gs 1 0 0 rg 10 10 50 50 re f 30 30 50 50 re f"
        )
        .contains("knockout"),
        "a group that is not a knockout group is drawn as it asks to be"
    );
    assert!(
        !reported(
            knockout,
            "1 0 0 rg 10 10 50 50 re f /GM gs 0 0 1 rg 30 30 50 50 re f"
        )
        .contains("knockout"),
        "a soft mask is opacity, so the element's shape is its path and is stated"
    );
    assert!(
        !reported(knockout, "1 0 0 rg 10 10 50 50 re f /GS gs /In Do").contains("knockout"),
        "a nested group's shape is the union of its elements' and is stated"
    );
    // A shading painted under `/GS gs` carries the constant alpha in its own colours
    // (`Shading::with_alpha`), where an unpainted region would carry the same number.
    let refusal = reported(
        knockout,
        "1 0 0 rg 10 10 50 50 re f /GS gs q 30 30 50 50 re W n /Sh sh Q",
    );
    assert!(
        refusal.contains("knockout") && refusal.contains("shading that is not opaque"),
        "a translucent shading keeps the report, and the report names why: {refusal}"
    );
}

/// §11.6.4.3's `/AIS` inverts what a knockout element's shape is, and is refused by name.
///
/// > This is a boolean flag, set with the AIS ("alpha is shape") entry in a graphics state
/// > parameter dictionary (8.4.5, "Graphics state parameter dictionaries"): true if the soft
/// > mask contains shape values, false for opacity.
///
/// Under `false` — the default, and what `stated_shape` builds — the mask and the two alpha
/// constants are opacity, so an element's shape is its mark without them. Under `true` they
/// are the shape, and removing them states exactly the wrong quantity. This is the one place
/// in this renderer where the flag can change a pixel at all, because §11.3.7.1 makes alpha
/// the product `f × q` everywhere else.
///
/// **Nine of the corpus's 974 documents state `/AIS true`**, against a ledger row that said
/// none did, and the flag costs no page: the corpus's incomplete list is the same 67 with it
/// and without it.
#[test]
fn alpha_is_shape_refuses_the_knockout_it_would_invert() {
    let knockout = "/Group << /S /Transparency /I true /K true >>";
    let reported = |form: &str| {
        format!(
            "{:?}",
            interpret(fixture(knockout, "[0 0 100 100]", form, "/Fm Do")).unsupported
        )
    };
    let under_ais = reported("1 0 0 rg 10 10 50 50 re f /GA gs 0 0 1 rg 30 30 50 50 re f");
    assert!(
        under_ais.contains("knockout") && under_ais.contains("/AIS"),
        "the report names the entry that inverts the two quantities: {under_ais}"
    );
    // The same content under a mask that does *not* set the flag is drawn, which is what
    // says the refusal is about `/AIS` rather than about the mask.
    assert!(
        !reported("1 0 0 rg 10 10 50 50 re f /GM gs 0 0 1 rg 30 30 50 50 re f")
            .contains("knockout")
    );
}

/// §11.4.6's arithmetic, at the pixel, for an element whose shape is not its coverage.
///
/// # What is measured, and against what
///
/// The clause's own two stages. Composite the object with the group's initial backdrop
/// "disregarding the object's shape and using a source shape value of 1.0 everywhere", then
/// take a "weighted average of this result with the object's immediate backdrop, using the
/// source shape as the weighting factor". On the transparent backdrop of §11.4.5's isolated
/// group that is `P' = (1 − f) × P + S` in premultiplied form, `f` the shape and `S` the
/// object's premultiplied colour.
///
/// Both fixtures paint an opaque **red** square and then a **blue** one over it whose shape
/// is 1 and whose opacity is a half — once by a `/Luminosity` soft mask of a 0.5 grey, once
/// by a nested group painted under `ca 0.5`. So in the overlap `f = 1`: the red is knocked
/// out entirely and the group holds blue at alpha ½, which over the white page is
/// `(127, 127, 255)`.
///
/// **The number the old route drew is 127 of 255 away**, and it is the reason the fixture is
/// worth the words: with the shape read off the alpha the blue composites *over* the red at
/// a half, giving `(127, 0, 127)` — a purple band where the clause asks for a pale blue one.
/// Putting the old route back makes both assertions fail on the green channel.
#[test]
fn a_stated_shape_knocks_the_element_under_it_out_entirely() {
    let knockout = "/Group << /S /Transparency /I true /K true >>";
    // Page (40, 40) is inside both squares; device row 100 − 40 = 60.
    let masked = interpret(fixture(
        knockout,
        "[0 0 100 100]",
        "1 0 0 rg 10 10 50 50 re f /GM gs 0 0 1 rg 30 30 50 50 re f",
        "/Fm Do",
    ));
    assert!(masked.is_complete(), "{:?}", masked.unsupported);
    assert_eq!(
        pixel(&masked, 40, 60),
        [127, 127, 255, 255],
        "the mask is opacity, so the shape knocks the red out whole"
    );
    // Outside the blue square the red is untouched, which is what says the knockout is
    // confined to a shape rather than applied to the group.
    assert_eq!(pixel(&masked, 20, 80), [255, 0, 0, 255]);

    let nested = interpret(fixture(
        knockout,
        "[0 0 100 100]",
        "1 0 0 rg 10 10 50 50 re f /GS gs /In Do",
        "/Fm Do",
    ));
    assert!(nested.is_complete(), "{:?}", nested.unsupported);
    assert_eq!(
        pixel(&nested, 40, 60),
        [127, 127, 255, 255],
        "a nested group's shape is where it marks, not the alpha it is painted at"
    );
    assert_eq!(pixel(&nested, 20, 80), [255, 0, 0, 255]);
}

/// §11.4.4's NOTE 5: a non-isolated group whose result composites trivially is not built.
///
/// > the effect of compositing objects as a group is the same as that of compositing them
/// > separately (without grouping) if the following conditions hold:
/// >
/// > The group is non-isolated and has the same knockout attribute as its parent group
///
/// > When compositing the group's results with the group backdrop, the Normal blend mode is
/// > used, and the shape and opacity inputs are always 1.0.
///
/// The measurement is the *blend*, because that is the whole of what isolation changes: an
/// element blending Multiply inside a non-isolated group multiplies against the page. The
/// fixture paints a mid-grey page and then a red square inside the group under `/BM
/// /Multiply`; flattened, the red multiplies with the grey and darkens.
#[test]
fn a_non_isolated_group_blends_with_the_page_behind_it() {
    let inside_group = interpret(fixture(
        "/Group << /S /Transparency >>",
        "[0 0 100 100]",
        "/GB gs 1 0 0 rg 10 10 50 50 re f",
        "0.5 g 0 0 100 100 re f /Fm Do",
    ));
    assert!(
        inside_group.is_complete(),
        "nothing is owed: {:?}",
        inside_group.unsupported
    );
    let blended = pixel(&inside_group, 30, 70);

    // The same content with no group at all, which NOTE 5 says must produce the same page.
    let ungrouped = interpret(fixture(
        "",
        "[0 0 100 100]",
        "/GB gs 1 0 0 rg 10 10 50 50 re f",
        "0.5 g 0 0 100 100 re f /Fm Do",
    ));
    assert_eq!(
        blended,
        pixel(&ungrouped, 30, 70),
        "NOTE 5: grouping and not grouping are the same page"
    );

    // And an *isolated* group is the case that genuinely differs: its element multiplies
    // against a transparent backdrop rather than against the grey.
    let isolated = interpret(fixture(
        "/Group << /S /Transparency /I true >>",
        "[0 0 100 100]",
        "/GB gs 1 0 0 rg 10 10 50 50 re f",
        "0.5 g 0 0 100 100 re f /Fm Do",
    ));
    assert_ne!(
        blended,
        pixel(&isolated, 30, 70),
        "isolation is what the flag means, and it shows"
    );
}

/// A one-page fixture nesting a group inside an *isolated* one, so the inner group's
/// backdrop is partly transparent.
///
/// [`fixture`]'s page is opaque, and an opaque backdrop hides half of §11.4.4: the
/// interpolation's two weights are then the only thing acting. Here the outer form fills
/// with `/GS`'s `ca 0.5` before invoking the inner one, so the inner group's backdrop has an
/// alpha of one half and the arithmetic has an `alpha0` to carry.
///
/// `inner` is the inner form's `/Group`; `content` is what it draws; `outer_body` is the
/// outer isolated group's content, which invokes it.
fn nested_fixture(inner: &str, content: &str, outer_body: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /XObject << /Ou 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length 8 >>\nstream\n/Ou Do\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /I true >> \
         /Resources << /ExtGState << /GS << /ca 0.5 >> /GB << /BM /Multiply >> >> \
         /XObject << /Fm 6 0 R >> >> /Length {} >>\nstream\n{outer_body}\nendstream\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] {inner} \
         /Resources << /ExtGState << /GB << /BM /Multiply >> >> >> /Length {} >>\n\
         stream\n{content}\nendstream\nendobj\n",
        outer_body.len() + 1,
        content.len() + 1
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
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

/// §11.4.5 against §11.4.4: what is left to report once §11.4.4's own model is drawn.
///
/// A non-isolated group composites its elements onto the group's backdrop and then removes
/// that backdrop's contribution again (§11.4.4 NOTE 3). Under the Normal blend mode the
/// removal is exact and the two computations agree, which is what §11.6.7's NOTE 1 states
/// for the same computation applied to a pattern cell — so a group whose every element
/// paints Normal is drawn as an isolated one whatever it declares, and says nothing.
///
/// Where an element *does* blend, the display list states the group's own backdrop (ADR
/// 0237) on three conditions, and this pins what happens either side of each of them: a
/// constant alpha and a soft mask at the `Do` are drawn, a blend mode at the `Do` is not,
/// and neither is a knockout group or an element of one.
#[test]
fn a_non_isolated_group_reports_only_where_the_backdrop_cannot_be_stated() {
    let reported = |group: &str, form: &str, page: &str| {
        format!(
            "{:?}",
            interpret(fixture(group, "[0 0 100 100]", form, page)).unsupported
        )
    };
    let non_isolated = "/Group << /S /Transparency >>";
    let blending = "/GB gs 1 0 0 rg 10 10 50 50 re f";

    assert!(
        !reported(non_isolated, blending, "/GS gs /Fm Do").contains("non-isolated"),
        "a constant alpha at the `Do` is the weight §11.4.4's two steps collapse to"
    );
    assert!(
        !reported(non_isolated, blending, "/GM gs /Fm Do").contains("non-isolated"),
        "and so is a soft mask, which is what every corpus witness states"
    );
    assert!(
        reported(non_isolated, blending, "/GB gs /Fm Do").contains("non-isolated"),
        "a blend mode at the `Do` is where the collapse fails: the group's own colour is \
         needed, and with it Table 140's group alpha"
    );
    assert!(
        !reported(non_isolated, TWO_SQUARES, "/GS gs /Fm Do").contains("non-isolated"),
        "with every element Normal the backdrop cancels and the two agree"
    );
    assert!(
        !reported(
            "/Group << /S /Transparency /I true >>",
            blending,
            "/GS gs /Fm Do"
        )
        .contains("non-isolated"),
        "an isolated group is what a rasteriser's layer already is"
    );
    assert!(
        reported(
            "/Group << /S /Transparency /K true >>",
            blending,
            "/GS gs /Fm Do"
        )
        .contains("non-isolated"),
        "§11.4.6 composites each element with the group's *initial* backdrop, which is not \
         the pair of draws this states"
    );
}

/// §11.4.4's own model, drawn: a non-isolated group's element blends with the page.
///
/// # The arithmetic, from the clause and not from a renderer
///
/// §11.4.4 composites the elements onto the group's backdrop and then removes it again
/// (NOTE 3), and §11.3.3 composites the group's result back onto that same backdrop. The
/// removal divides by Table 140's group alpha and the re-compositing multiplies by it, so
/// with the Normal blend function at the `Do` the pair collapses to an interpolation
/// between the backdrop `B` and the elements composited onto it, `E(B)`, by the group's
/// constant alpha times its soft mask. See `Command::Group`'s `isolated` and ADR 0237.
///
/// The page is opaque red and the group's one element is an opaque **blue** under `Multiply`.
/// §11.3.5.2's Multiply is the componentwise product, so `E(B)` is `(1,0,0) × (0,0,1)` =
/// black inside the element and the red page outside it. With `w = ½` from `/GS`'s `ca`,
///
/// ```text
/// (1 − ½) × (1, 0, 0) + ½ × (0, 0, 0) = (½, 0, 0)     inside
/// (1 − ½) × (1, 0, 0) + ½ × (1, 0, 0) = (1,  0, 0)    outside
/// ```
///
/// which is **(128, 0, 0)** and **(255, 0, 0)**. Drawing the group on §11.4.5's transparent
/// backdrop instead — what this tree did until ADR 0237, and what it reported — leaves the
/// blue unblended and gives `(128, 0, 128)`: the whole blue channel.
///
/// The second half doubles the weight through a soft mask as well as the constant, because
/// a soft mask at the `Do` is what all four corpus witnesses actually state: `w = ¼` gives
/// `(¾, 0, 0)` = **(191, 0, 0)**.
#[test]
fn a_non_isolated_groups_element_blends_with_the_backdrop_the_group_is_painted_over() {
    let blend_over_red = |page: &str| {
        interpret(fixture(
            "/Group << /S /Transparency >>",
            "[0 0 100 100]",
            "/GB gs 0 0 1 rg 10 10 50 50 re f",
            page,
        ))
    };

    let by_alpha = blend_over_red("1 0 0 rg 0 0 100 100 re f /GS gs /Fm Do");
    assert!(by_alpha.is_complete(), "{:?}", by_alpha.unsupported);
    // Device y = 60 is page y = 40, inside the element; x = 70 is outside it and inside
    // the form's `/BBox`, which is the whole page.
    assert_eq!(
        pixel(&by_alpha, 30, 60),
        [128, 0, 0, 255],
        "Multiply against the page the group does not exclude leaves no blue at all"
    );
    assert_eq!(
        pixel(&by_alpha, 70, 60),
        [255, 0, 0, 255],
        "and where the group marks nothing the weight puts the page back unchanged"
    );

    let by_mask = blend_over_red("1 0 0 rg 0 0 100 100 re f /GS gs /GM gs /Fm Do");
    assert!(by_mask.is_complete(), "{:?}", by_mask.unsupported);
    assert_eq!(
        pixel(&by_mask, 30, 60),
        [191, 0, 0, 255],
        "the soft mask and the constant are one weight, and it is their product"
    );
    assert_eq!(pixel(&by_mask, 70, 60), [255, 0, 0, 255]);
}

/// The second of §11.4.6's two stages is an *addition*, and one pixel says so.
///
/// The clause's stage b) is a "weighted average of this result with the object's immediate
/// backdrop, using the source shape as the weighting factor" — the shape weights the
/// backdrop and the object arrives whole. Drawing the object with ordinary source-over after
/// emptying the shape would weight the backdrop a second time, by `1 − shape × opacity`,
/// which is right only where the object is opaque or its shape is 0 or 1. Every other test
/// here has a shape of 1 in the overlap and cannot see the difference.
///
/// So this one puts a **half-covered** pixel under a **half-opaque** object. The blue
/// rectangle starts at page x = 10.5, so device column 10 has shape `f = ½`; the `/GM` soft
/// mask is a 0.5 grey, so its opacity is `q = ½`. Over an opaque red page inside an isolated
/// knockout group, §11.4.6 gives, premultiplied,
///
/// ```text
/// P' = (1 − f) × P + f × q × blue = ½ × (1, 0, 0; 1) + ¼ × (0, 0, 1; 1)
///    = (0.5, 0, 0.25; 0.75)
/// ```
///
/// and compositing that onto the white page adds `1 − 0.75` of white to each component:
/// `(0.75, 0.25, 0.5)`, which is **(191, 64, 128)**. Source-over in the second stage gives
/// `(191, 96, 160)` instead — 32 of 255 on two channels — and that is what this fails with
/// when `Compose::Add` is changed to over.
#[test]
fn the_object_is_added_to_the_backdrop_the_shape_left_behind() {
    let painted = interpret(fixture(
        "/Group << /S /Transparency /I true /K true >>",
        "[0 0 100 100]",
        "1 0 0 rg 0 0 100 100 re f /GM gs 0 0 1 rg 10.5 10 50 50 re f",
        "/Fm Do",
    ));
    assert!(painted.is_complete(), "{:?}", painted.unsupported);
    assert_eq!(
        pixel(&painted, 10, 60),
        [191, 64, 128, 255],
        "half a shape keeps half the backdrop and the object is added to it"
    );
    // The columns either side, which are the two ends of the same formula: shape 0 keeps the
    // red whole, and shape 1 replaces it with the half-opaque blue.
    assert_eq!(pixel(&painted, 9, 60), [255, 0, 0, 255]);
    assert_eq!(pixel(&painted, 11, 60), [127, 127, 255, 255]);
}

/// §11.4.4 with a backdrop that is not opaque, which is where the interpolation shows.
///
/// # Why the page above cannot ask this
///
/// A page is opaque, and over an opaque backdrop the two weights of
/// `(1 − w) × B + w × E(B)` are the whole of the arithmetic. Nest the group inside an
/// *isolated* one and its backdrop is that group's buffer — half-transparent here — and two
/// further things become visible: the blend function inside the group sees a backdrop alpha,
/// and the region the group does **not** mark has to come back exactly as it was. Ordinary
/// source-over of the buffer would not do the second: it weights the backdrop by
/// `1 − w × alpha_buffer` where §11.4.4 weights it by `1 − w`, and at `alpha_buffer = ½` and
/// `w = ½` that is **(255, 95, 95)** against the backdrop's own (255, 127, 127) — 32 of 255
/// on two channels, the same magnitude ADR 0234 measured for §11.4.6's second stage.
///
/// # The arithmetic
///
/// The outer group fills opaque red at `ca 0.5`, so its buffer holds premultiplied
/// `(128, 0, 0; 128)` — one half of full scale as eight bits carry it. The inner
/// non-isolated group draws opaque blue under `Multiply`, whose §11.3.5.2 value is the
/// componentwise product `(1,0,0) × (0,0,1) = (0,0,0)`, so §11.3.3 gives
///
/// ```text
/// alpha = 1,  C = Cb + ((1 - alpha0) Cs + alpha0 B(Cb, Cs) - Cb) = (0, 0, 1 - 128/255)
/// ```
///
/// premultiplied `(0, 0, 127; 255)`. Interpolating at `w = ½`:
///
/// ```text
/// inside   (64, 0, 63.5; 191.5) -> (64, 0, 64; 192)
/// outside  the buffer is the backdrop, so (128, 0, 0; 128) comes back unchanged
/// ```
///
/// and the outer group over the white page adds `255 - alpha` of white to each component:
/// **(127, 63, 127)** and **(255, 127, 127)**.
///
/// Drawn on §11.4.5's transparent backdrop instead, the blue never meets the red at all and
/// the same pixel is **(127, 63, 191)**.
#[test]
fn a_non_isolated_group_inside_another_keeps_the_backdrop_alpha_it_composites_onto() {
    let content = "/GB gs 0 0 1 rg 10 10 50 50 re f";
    let invoking = "/GS gs 1 0 0 rg 0 0 100 100 re f /Fm Do";
    let backdrop_alone = "/GS gs 1 0 0 rg 0 0 100 100 re f";

    let drawn = interpret(nested_fixture(
        "/Group << /S /Transparency >>",
        content,
        invoking,
    ));
    assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
    assert_eq!(
        pixel(&drawn, 30, 60),
        [127, 63, 127, 255],
        "Multiply against a half-opaque red leaves half the blue, not all of it"
    );

    // The same page with the inner form never invoked. Compared rather than predicted: this
    // is §11.4.4 with the group's own marks taken out, and it is the assertion that fails if
    // the buffer is composited back with source-over instead of interpolated.
    let backdrop = interpret(nested_fixture(
        "/Group << /S /Transparency >>",
        content,
        backdrop_alone,
    ));
    assert_eq!(
        pixel(&drawn, 70, 60),
        pixel(&backdrop, 70, 60),
        "where the group marks nothing, its backdrop comes back exactly as it was"
    );
    assert_eq!(pixel(&backdrop, 70, 60), [255, 127, 127, 255]);

    // And §11.4.5's transparent initial backdrop, which is what this tree drew until the
    // four-hundredth session and reported by name while it did.
    let flattened_wrongly = interpret(nested_fixture(
        "/Group << /S /Transparency /I true >>",
        content,
        invoking,
    ));
    assert_eq!(
        pixel(&flattened_wrongly, 30, 60),
        [127, 63, 191, 255],
        "an isolated group's element multiplies against nothing and keeps its blue"
    );
}

/// A one-page fixture whose *page* states a `/Group`, with one form group inside it.
///
/// §11.4.7 makes the page group the root of §11.6.6's inheritance, and no other fixture in
/// this file states one — so until this existed there was no way to write a test about the
/// space a page composites in at all.
///
/// `page_group` is the page dictionary's `/Group` entry, written whole so a test can leave it
/// out; `form_group` is the form's; `form` is what the form draws and `page` what the page
/// draws around it.
fn page_group_fixture(page_group: &str, form_group: &str, form: &str, page: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] {page_group} \
         /Resources << /ExtGState << /GS << /ca 0.5 /CA 0.5 >> /GH << /BM /Hue >> >> \
         /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] {form_group} \
         /Resources << /ExtGState << /GS << /ca 0.5 /CA 0.5 >> >> \
         /XObject << /In 6 0 R >> >> /Length {} >>\n\
         stream\n{form}\nendstream\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /CS /DeviceCMYK >> \
         /Resources << /ExtGState << /GS << /ca 0.5 >> >> >> /Length {} >>\n\
         stream\n{NESTED}\nendstream\nendobj\n",
        page.len() + 1,
        form.len() + 1,
        NESTED.len() + 1
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
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

/// The nested non-isolated group object 6 holds, which restates `/DeviceCMYK`.
///
/// Two half-opaque fills, so that whatever space is in force is a space something composites
/// in: the report every test below reads is conditioned on that.
const NESTED: &str = "/GS gs 0 0 0 1 k 0 0 60 60 re f 1 1 0 0 k 40 40 60 60 re f";

/// §11.6.6's inheritance and §11.4.7's root of it, which decide *which* space is departed from.
///
/// The entry is not read where the file writes it. §11.6.6 gives a group's `/CS` effect "[f]or
/// isolated groups" and then says of the rest:
///
/// > For non-isolated groups, or if no group colour space is specified, the group colour space
/// > shall be inherited from the parent group or page.
///
/// and §11.7.2 repeats it — "[n]on-isolated groups shall inherit their colour space from the
/// nearest ancestor isolated parent group". So the six cases below are one clause read in one
/// direction, and until the four-hundred-and-fifteenth session this tree reported the declared
/// entry: a non-isolated `/DeviceCMYK` group was named on a page that composites in RGB, and a
/// page group of `/DeviceCMYK` — which decides every mark on the page — was not named at all.
#[test]
fn the_blending_space_is_the_one_in_force_rather_than_the_one_declared() {
    let reported = |page_group: &str, form_group: &str| {
        let page = "/GS gs 1 0 0 RG 0 0 1 rg 10 10 50 50 re B /Fm Do";
        let form = "/GS gs 0 1 0 rg 20 20 50 50 re f /In Do";
        format!(
            "{:?}",
            interpret(page_group_fixture(page_group, form_group, form, page)).unsupported
        )
    };
    let page_cmyk = "/Group << /S /Transparency /CS /DeviceCMYK >>";
    let group = |entry: &str| format!("/Group << /S /Transparency {entry} >>");

    // A non-isolated group naming `/DeviceCMYK` on a page that states no group at all. The
    // clause hands the space to the parent, and the parent is §11.4.7's page group, whose
    // space "is inherited from the native colour space of the actual, assumed or simulated
    // output device" — this device's three components. Nothing departs.
    let inherited = reported("", &group("/CS /DeviceCMYK"));
    assert!(
        !inherited.contains("blending colour space"),
        "a non-isolated group's own /CS is not the space anything composites in: {inherited}"
    );

    // The same group with `/I true`, which is the first bullet's condition.
    let isolated = reported("", &group("/I true /CS /DeviceCMYK"));
    assert!(
        isolated.contains("blending colour space /DeviceCMYK"),
        "an isolated group's /CS is the space its elements composite in: {isolated}"
    );

    // §11.4.7's page group, which decides the whole page and which this tree read nothing of
    // before. The form here is the *same* non-isolated one that reported nothing above.
    let page_level = reported(page_cmyk, &group(""));
    assert!(
        page_level.contains("the page group's blending colour space /DeviceCMYK (§11.4.7)"),
        "a page group's /CS is the default blending space for the page: {page_level}"
    );

    // And it is reported once, at the point the file introduces it, rather than again at every
    // group that inherits it — the fixture nests three groups inside that page.
    assert_eq!(
        page_level.matches("blending colour space").count(),
        1,
        "one departure named where it is introduced: {page_level}"
    );

    // An isolated group *replaces* the inherited space, which the first bullet says outright:
    // its elements are converted to the group's space, not to the page's. So an RGB group
    // inside a `/DeviceCMYK` page reports the page and not itself — and the nested
    // `/DeviceCMYK` group inside *it* is non-isolated, so it inherits the RGB one.
    let replaced = reported(page_cmyk, &group("/I true /CS /DeviceRGB"));
    assert_eq!(
        replaced.matches("blending colour space").count(),
        1,
        "an isolated RGB group inside a CMYK page departs only above itself: {replaced}"
    );

    // A page group of `/DeviceRGB` is what this tree already composites in, so it is not a
    // departure — the entry being *present* is not the condition.
    let rgb_page = reported("/Group << /S /Transparency /CS /DeviceRGB >>", &group(""));
    assert!(
        !rgb_page.contains("blending colour space"),
        "a page group naming the device's own components asks for what happens: {rgb_page}"
    );

    // And the page-level report is conditioned on something compositing, for the reason every
    // other report in this file is: an opaque Normal paint carries its colour through
    // unchanged whatever space it is carried through, so a page of them is the same page.
    let opaque = format!(
        "{:?}",
        interpret(page_group_fixture(
            page_cmyk,
            &group(""),
            "0 1 0 rg 20 20 50 50 re f",
            "0 0 1 rg 10 10 50 50 re f",
        ))
        .unsupported
    );
    assert!(
        !opaque.contains("blending colour space"),
        "nothing composites, so the space cannot change a pixel: {opaque}"
    );
}

/// What compositing in `/DeviceCMYK` costs, against compositing in the device's components.
///
/// This is the arithmetic behind the report above, and it is why the report is a report rather
/// than a construction. §11.3.3 composites two colours as a weighted average and §11.3.6 says
/// so — "the compositing formula collapses to a simple weighted average of the backdrop and
/// source colours" under `Normal` — so doing it in the blending space and converting once at
/// the end agrees with converting first and compositing in the device's components **exactly
/// when the conversion is affine over the colours involved**, and only then.
///
/// This tree's `DeviceCMYK` conversion is multilinear over the ink cube (ADRs 0009, 0042),
/// which is not affine: the interpolation carries products of the four inks. The two orders of
/// operation therefore differ, and the fixture below is the simplest case there is — a
/// half-opaque registration black over paper, which is 51.5 of 255 apart. That number is what
/// makes §11.6.6's blending space a second raster format rather than a colour conversion, and
/// ADR 0251 has the 300 000-case measurement it is the head of.
#[test]
fn compositing_in_cmyk_is_not_compositing_in_the_device_and_this_is_the_gap() {
    use pdf_model::colour::ColourSpace;

    // §11.3.3 under `Normal`, over an opaque backdrop: `Cr = (1 − as) × Cb + as × Cs`.
    let mix = |backdrop: &[f32], source: &[f32], alpha: f32| -> Vec<f32> {
        backdrop
            .iter()
            .zip(source)
            .map(|(b, s)| (1.0 - alpha) * b + alpha * s)
            .collect()
    };
    let rgb = |values: &[f32]| {
        let colour = ColourSpace::Cmyk.to_rgb(values);
        [colour.r, colour.g, colour.b]
    };

    let paper = [0.0, 0.0, 0.0, 0.0];
    let registration = [1.0, 1.0, 1.0, 1.0];

    // §11.6.6's order: composite in the blending space, convert the result once.
    let in_cmyk = rgb(&mix(&paper, &registration, 0.5));
    // This tree's order: convert each colour, composite on the device's components.
    let in_device = mix(&rgb(&paper), &rgb(&registration), 0.5);

    let gap = in_cmyk
        .iter()
        .zip(&in_device)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f32, f32::max);
    assert!(
        gap * 255.0 > 51.0,
        "half of registration black over paper is {in_cmyk:?} in CMYK and {in_device:?} on \
         the device, {:.1} of 255 apart",
        gap * 255.0
    );

    // And the direction is not an accident: compositing in ink makes the half-covered pixel
    // *darker* than mixing the two converted colours, because half the ink of registration
    // black is still most of the way to black.
    assert!(
        in_cmyk[0] < in_device[0] && in_cmyk[1] < in_device[1] && in_cmyk[2] < in_device[2],
        "compositing in ink is darker: {in_cmyk:?} against {in_device:?}"
    );

    // Where the conversion *is* affine the two orders agree, which is the other half of the
    // claim and is what ADR 0220 relied on one clause over: a `DeviceGray` colour taken into
    // CMYK moves along one edge of the cube, and multilinear interpolation is affine on an
    // edge.
    let dark = [0.0, 0.0, 0.0, 0.8];
    let light = [0.0, 0.0, 0.0, 0.2];
    let along_the_edge = rgb(&mix(&dark, &light, 0.5));
    let converted_first = mix(&rgb(&dark), &rgb(&light), 0.5);
    for (a, b) in along_the_edge.iter().zip(&converted_first) {
        assert!(
            (a - b).abs() * 255.0 < 0.5,
            "on one edge of the ink cube the two orders agree: {along_the_edge:?} against \
             {converted_first:?}"
        );
    }
}

/// Paper laid down opaque, then half of registration black over it.
///
/// The backdrop has to be *painted* rather than left to the medium: §11.4.7 makes the page
/// group isolated, so an unpainted pixel is transparent and the medium is composited with the
/// page's result **after** the conversion out of the blending space — where both orders of
/// operation agree. What tells them apart is a composite that happens inside the page, and
/// `0 0 0 0 k` is the paper written as ink.
const INK_OVER_PAPER: &str = "0 0 0 0 k 0 0 100 100 re f\n\
                              /GS gs 1 1 1 1 k 0 0 100 100 re f";

/// §11.4.7's page group, drawn in the space it names rather than on the device's components.
///
/// > All page-level compositing shall be done in the default blending colour space of the
/// > page, and the entire result shall then, if the colour spaces are not equivalent, be
/// > converted to the native colour space of the output device before being composited with
/// > the context-dependent backdrop.
///
/// Half of registration black over paper is the fixture ADR 0251 priced this population with:
/// composited in ink it is `[0.5, 0.5, 0.5, 0.5]`, which is the average of the ink cube's
/// sixteen corners and **76 of 255** in red; composited on the device it is the average of
/// black and white, **127.5**. The old route is put back by the second half of the test, which
/// states no `/CS` at all and gets 127 — so the number this asserts is a difference this round
/// made rather than a number that was already there.
#[test]
fn a_page_group_in_ink_composites_in_ink() {
    let ink = interpret(page_group_fixture(
        "/Group << /S /Transparency /CS /DeviceCMYK >>",
        "",
        "",
        INK_OVER_PAPER,
    ));
    assert!(
        !format!("{:?}", ink.unsupported).contains("blending colour space"),
        "the page is drawn in the space it states: {:?}",
        ink.unsupported
    );
    let drawn = pixel(&ink, 50, 50);
    assert_eq!(
        drawn[3], 255,
        "the paper underneath is opaque, so the pixel is"
    );
    assert!(
        (75..=77).contains(&drawn[0]),
        "half of registration black over paper is the cube's own average, 76 of 255: {drawn:?}"
    );

    // The old route, stated by a page that names no blending space: the two colours are
    // converted first and averaged on the device's components, which is 127.5.
    let device = interpret(page_group_fixture("", "", "", INK_OVER_PAPER));
    let plain = pixel(&device, 50, 50);
    assert!(
        (127..=128).contains(&plain[0]),
        "converting first and averaging gives 127.5 of 255: {plain:?}"
    );
}

/// A colour §11.7.2 would have to convert *into* the blending space keeps the page reported.
///
/// > If the colour space of a graphics object within the group is not equivalent to the
/// > group's blending colour space, then it shall be converted to the group's colour space ,
/// > and all blending and compositing computations shall be done in that space
///
/// §11.7.5.3 names §10.4.2.4 as that conversion, and §10.4.2.1 packages §10.4.2.2 to §10.4.2.5
/// as what a processor uses *instead of* §10.3 — which is the branch ADRs 0009 and 0042 put
/// this tree's conversion out of `DeviceCMYK` on. Composing the two moves a colour the clause
/// never asked to move, so the page is drawn as before and says so. ADR 0262.
#[test]
fn a_colour_from_outside_the_blending_space_is_reported_rather_than_converted_into_it() {
    let mixed = interpret(page_group_fixture(
        "/Group << /S /Transparency /CS /DeviceCMYK >>",
        "",
        "",
        "0 0 0 0 k 0 0 100 100 re f\n\
         q /GS gs 1 1 1 1 k 0 0 100 100 re f Q\n\
         1 0 0 rg 0 0 10 10 re f",
    ));
    let reported = format!("{:?}", mixed.unsupported);
    assert!(
        reported.contains("a colour outside it is painted into it"),
        "the conversion into the space is what is missing: {reported}"
    );
    // And the red is the red the file states, not the red process inks print for it.
    assert_eq!(pixel(&mixed, 5, 95)[0..3], [255, 0, 0]);
}

/// §11.3.5.3's non-separable modes give the black component a rule of its own.
///
/// > For the K component, the result shall be the K component of Cb for the Hue , Saturation ,
/// > and Color blend modes; it shall be the K component of Cs for the Luminosity blend mode.
///
/// Which is a blend function neither raster has: the pair of passes carries three components
/// in one and the fourth in the other, and each is composited by the backend's own separable
/// arithmetic. Reported by name rather than drawn with the chromatic rule applied to black.
#[test]
fn a_non_separable_blend_keeps_a_page_group_on_the_devices_components() {
    let blended = interpret(page_group_fixture(
        "/Group << /S /Transparency /CS /DeviceCMYK >>",
        "",
        "",
        "0 0 0 0 k 0 0 100 100 re f\n\
         /GH gs 1 1 1 1 k 0 0 100 100 re f",
    ));
    let reported = format!("{:?}", blended.unsupported);
    assert!(
        reported.contains("non-separable blend mode"),
        "§11.3.5.3's rule for the black component is what is missing: {reported}"
    );
}
