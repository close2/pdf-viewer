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
//! What is *not* implemented is reported rather than tested here: §11.4.6's knockout
//! groups, a non-isolated group whose elements blend, and a group blending colour space
//! that is not the device's. The conditions those reports fire on are pinned below,
//! because a report that names a page where the output cannot differ costs that page its
//! place in the oracle's comparison (see `doc/HANDOVER.md`, trap 11).

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
         /GM << /SMask << /S /Luminosity /G 6 0 R >> >> >> \
         /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox {bbox} {group} /Length {} >>\n\
         stream\n{form}\nendstream\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /CS /DeviceGray >> /Length {} >>\n\
         stream\n{MASK}\nendstream\nendobj\n",
        page.len() + 1,
        form.len() + 1,
        MASK.len() + 1
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

    assert_eq!(groups("/Group << /S /Transparency >>"), 1);
    assert_eq!(groups("/Group << /S /Fictional >>"), 0);
    assert_eq!(groups("/Group << /I true >>"), 0, "no subtype, no group");
    assert_eq!(groups(""), 0, "an ordinary form is not a group");
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

/// §11.4.6's knockout is reported where a rasteriser cannot draw an element's *shape*.
///
/// The clause states the reason itself:
///
/// > The existence of the knockout feature is the main reason for maintaining a separate
/// > shape value rather than only a single alpha that combines shape and opacity.
///
/// A raster of premultiplied samples has one alpha and no shape, so knockout reaches the
/// display list only where the two coincide — every element's transparency being an opacity
/// rather than a shape. A soft mask is §11.6.4.1's opacity applied here as coverage, an
/// image's own alpha may be either §8.9.6.2's stencil or §11.6.5.2's `/SMask`, and a nested
/// group arrives as a raster. Those keep the report; the rest is drawn.
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
        reported(
            knockout,
            "1 0 0 rg 10 10 50 50 re f /GM gs 0 0 1 rg 30 30 50 50 re f"
        )
        .contains("knockout"),
        "an element under a soft mask has an opacity this backend applies as a shape"
    );
}

/// §11.4.5 against §11.4.4: a non-isolated group is reported only where isolation shows.
///
/// A non-isolated group composites its elements onto the group's backdrop and then removes
/// that backdrop's contribution again (§11.4.4 NOTE 3). Under the Normal blend mode the
/// removal is exact and the two computations agree, which is what §11.6.7's NOTE 1 states
/// for the same computation applied to a pattern cell. What makes them differ is a blend
/// mode inside the group — §11.4.4's NOTE 2 gives that as the whole reason the two kinds of
/// group exist — so that, and not the flag, is what the report fires on.
#[test]
fn a_non_isolated_group_reports_only_when_an_element_blends() {
    let reported = |group: &str, form: &str| {
        format!(
            "{:?}",
            interpret(fixture(group, "[0 0 100 100]", form, "/Fm Do")).unsupported
        )
    };
    let non_isolated = "/Group << /S /Transparency >>";

    assert!(
        reported(non_isolated, "/GB gs 1 0 0 rg 10 10 50 50 re f").contains("non-isolated"),
        "an element blending Multiply sees a backdrop the isolated computation excludes"
    );
    assert!(
        !reported(non_isolated, TWO_SQUARES).contains("non-isolated"),
        "with every element Normal the backdrop cancels and the two agree"
    );
    assert!(
        !reported(
            "/Group << /S /Transparency /I true >>",
            "/GB gs 1 0 0 rg 10 10 50 50 re f"
        )
        .contains("non-isolated"),
        "an isolated group is what this composites, blend modes and all"
    );
}
