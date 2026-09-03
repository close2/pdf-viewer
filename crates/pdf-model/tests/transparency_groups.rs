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
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and this page is 100 units \
              square where no arithmetic can overflow. The one `panic!` is `clause_blend_cmyk` \
              refusing a mode Table 135 does not name, which is a caller's typo and has to be \
              louder than a wrong colour"
)]
#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "test code: the ICC fixture's constants are written as the fixed-point values it \
              encodes, and the grid indices below seventeen are exact in f32"
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
         /GA << /AIS true /SMask << /S /Luminosity /G 6 0 R >> >> \
         /GT << /AIS true >> >> \
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

    assemble(&body)
}

/// Wraps a body of numbered objects in §7.5's header, cross-reference table and trailer.
fn assemble(body: &str) -> Vec<u8> {
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

/// The press every fixture below states: a four-ink multiplicative absorption model.
///
/// Not a real press and not meant to be one — what it has to be is *different* from
/// `pdf_model::colour`'s assumed process inks and *derivable*, so that a test's expected value
/// comes from this function rather than from what the tree happens to draw. Each ink absorbs a
/// different share of each connection-space axis, so a page drawn with the components
/// transposed is a different picture.
///
/// Variant 0 is the press every test here has always stated, to the bit — `0.80 * 1.0` is
/// `0.80` — so nothing written before the budget moved states different numbers. Every other
/// variant weakens the black ink by a hundredth and is a *different* press: a different
/// profile, a different identity and a different picture on the same content, which is what
/// makes a test that opens nine of them a test about nine presses rather than about nine
/// copies of one.
fn press_xyz_of(variant: usize, inks: [f32; 4]) -> [f32; 3] {
    // D50, which is the connection space's white point.
    let mut xyz = [0.964_2f32, 1.0, 0.824_9];
    let weaker = 1.0 - variant as f32 / 100.0;
    let absorb = [
        [0.60f32, 0.10, 0.10, 0.80 * weaker],
        [0.10, 0.60, 0.20, 0.80 * weaker],
        [0.10, 0.20, 0.70, 0.80 * weaker],
    ];
    for (axis, row) in absorb.iter().enumerate() {
        for (ink, factor) in inks.iter().zip(row) {
            xyz[axis] *= 1.0 - factor * ink;
        }
    }
    xyz
}

/// A v2 ICC profile whose `A2B1` table is [`press_xyz_of`] at the sixteen corners of the cube.
///
/// Positional, like `pdf_model::icc`'s own fixture: a 128-byte header, a tag count, one 12-byte
/// tag entry, then the `mft2` tag — sizes, matrix, input curves, CLUT, output curves. Two grid
/// points per axis, so the table *is* the sixteen corners and the profile's own interpolation
/// fills in between them.
fn icc_cmyk_profile() -> Vec<u8> {
    icc_cmyk_profile_of(0)
}

/// [`icc_cmyk_profile`] over [`press_xyz_of`]'s variant, so that a test can name several.
fn icc_cmyk_profile_of(variant: usize) -> Vec<u8> {
    let mut header = vec![0u8; 128];
    header[8] = 2; // major version
    header[12..16].copy_from_slice(b"prtr");
    header[16..20].copy_from_slice(b"CMYK");
    header[20..24].copy_from_slice(b"XYZ ");
    header[36..40].copy_from_slice(b"acsp");

    let mut tag = Vec::new();
    tag.extend_from_slice(b"mft2");
    tag.extend_from_slice(&[0; 4]);
    tag.push(4); // four input channels
    tag.push(3); // three output channels
    tag.push(2); // two grid points per axis
    tag.push(0);
    for value in [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0] {
        tag.extend_from_slice(&((value * 65536.0) as i32).to_be_bytes());
    }
    tag.extend_from_slice(&2u16.to_be_bytes()); // input table entries
    tag.extend_from_slice(&2u16.to_be_bytes()); // output table entries
    for _ in 0..4 {
        for value in [0u16, 0xFFFF] {
            tag.extend_from_slice(&value.to_be_bytes());
        }
    }
    // The CLUT, with the *last* input varying fastest, which is ICC's own order.
    for corner in 0..16usize {
        let at = |axis: usize| f32::from(u8::try_from((corner >> (3 - axis)) & 1).expect("a bit"));
        let xyz = press_xyz_of(variant, [at(0), at(1), at(2), at(3)]);
        for value in xyz {
            // `u1Fixed15`: 0x8000 is 1.0, which is the encoding XYZ uses in a lookup table.
            let encoded = (value * 32768.0).clamp(0.0, 65535.0) as u16;
            tag.extend_from_slice(&encoded.to_be_bytes());
        }
    }
    for _ in 0..3 {
        for value in [0u16, 0xFFFF] {
            tag.extend_from_slice(&value.to_be_bytes());
        }
    }

    let mut out = header;
    out.extend_from_slice(&1u32.to_be_bytes()); // one tag
    out.extend_from_slice(b"A2B1");
    out.extend_from_slice(&144u32.to_be_bytes()); // 128 + 4 + 12
    out.extend_from_slice(&u32::try_from(tag.len()).expect("small").to_be_bytes());
    out.extend_from_slice(&tag);
    out
}

/// [`icc_cmyk_profile`] as the `ASCIIHexDecode` text a fixture can hold.
fn profile_stream() -> String {
    profile_stream_of(0)
}

/// [`icc_cmyk_profile_of`] as the `ASCIIHexDecode` text a fixture can hold.
fn profile_stream_of(variant: usize) -> String {
    let mut hex = String::new();
    for byte in icc_cmyk_profile_of(variant) {
        let _ = write!(hex, "{byte:02X}");
    }
    hex.push('>');
    hex
}

/// A one-page fixture whose page composites in the press [`icc_cmyk_profile`] describes.
///
/// `group` is the page's `/Group`, `resources` any extra page resource entries and `intents`
/// the catalog's `/OutputIntents` — the three routes Annex P puts in order, each written whole
/// so that one test can state one of them.
fn press_fixture(group: &str, resources: &str, intents: &str, content: &str) -> Vec<u8> {
    let hex = profile_stream();
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R {intents} >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] {group} \
         /Resources << /ExtGState << /GS << /ca 0.5 /CA 0.5 >> >> {resources} >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /N 4 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}\nendstream\n\
         endobj\n",
        content.len() + 1,
        hex.len() + 1
    );
    assemble(&body)
}

/// A `/DeviceCMYK` page group on a page whose resources say what its `DeviceCMYK` *is*.
///
/// §8.6.5.6's `/DefaultCMYK`, naming a four-component `DeviceN` over an identity tint
/// transform. Four components and not [`ColourSpace::Cmyk`], which is the condition.
fn named_press_fixture(content: &str) -> Vec<u8> {
    let transform = "{ }";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Group << /S /Transparency /CS /DeviceCMYK >> \
         /Resources << /ExtGState << /GS << /ca 0.5 /CA 0.5 >> >> \
         /ColorSpace << /DefaultCMYK [/DeviceN [/C /M /Y /K] /DeviceCMYK 5 0 R] >> >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /FunctionType 4 /Domain [0 1 0 1 0 1 0 1] /Range [0 1 0 1 0 1 0 1] \
         /Length {} >>\nstream\n{transform}\nendstream\nendobj\n",
        content.len() + 1,
        transform.len() + 1
    );
    assemble(&body)
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
/// distinguishes the two and no corpus document writes another subtype — **re-derived in the
/// six-hundred-and-eighty-second session and it held**, zero over the curated 1251 and zero over
/// the `SafeDocs` crawl's 65 944 (`examples/absence_audit --crawl`), with a hand-built
/// `/Group << /S /Softness >>` put through the same block first so that the zero was a
/// measurement. A negative on a population sixty-eight times the size is why this test is the
/// only witness the requirement has.
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
    // §11.3.6's weighted average of two opaque byte colours at α = 0.5 is 127.5 in *both*
    // moving channels — `0.5 · 0 + 0.5 · 255` for the red and `0.5 · 255 + 0.5 · 0` for the
    // blue — so they round alike. This read `[127, 0, 128]` until the
    // five-hundred-and-eighty-third session, and the asymmetry was `tiny-skia`'s
    // low-precision pipeline rather than the clause (ADR 0418): identical arithmetic in two
    // channels cannot land a level apart.
    assert_eq!(
        pixel(&ordinary, 40, 50),
        [128, 0, 128, 255],
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

/// §11.6.4.3's `/AIS` inverts what a knockout element's shape is, and both readings are drawn.
///
/// > This is a boolean flag, set with the AIS ("alpha is shape") entry in a graphics state
/// > parameter dictionary (8.4.5, "Graphics state parameter dictionaries"): true if the soft
/// > mask contains shape values, false for opacity.
///
/// and §11.6.4.4 says the same of the two alpha constants. Under `false` — the default — the
/// mask and the constants are opacity, so an element's shape is its mark without them. Under
/// `true` they are shape, and §11.6.4.2 has already given the object itself an intrinsic
/// opacity of 1.0 everywhere, so **all three of §11.3.7.2's opacity inputs are 1.0 and the
/// alpha a rasteriser draws the element with is its shape**. This is the one place in this
/// renderer where the flag can change a pixel at all, because §11.3.7.1 makes alpha the
/// product `f × q` everywhere else.
///
/// # What is measured
///
/// One fixture, two readings, 127 of 255 apart. An opaque red square and a blue one over it
/// under a `/Luminosity` mask of a 0.5 grey. Under `/AIS false` the mask is opacity: the
/// blue's shape is its whole path, it knocks the red out entirely, and the group holds blue
/// at alpha ½ — `(127, 127, 255)` over the white page. Under `/AIS true` the mask is shape:
/// `f = 0.5`, `q = 1`, so §11.4.6's weighted average keeps half the red and adds half the
/// blue — `(127, 0, 127)`, a purple band where the other reading draws a pale blue one.
///
/// **Nine of the corpus's 974 documents state `/AIS true`** and none of their knockout groups
/// reaches this at all, which is why the witness is built by hand.
#[test]
fn alpha_is_shape_makes_the_drawn_alpha_the_knockout_shape() {
    let knockout = "/Group << /S /Transparency /I true /K true >>";
    let masked = "1 0 0 rg 10 10 50 50 re f /GM gs 0 0 1 rg 30 30 50 50 re f";
    // Page (40, 40) is inside both squares; device row 100 − 40 = 60.
    let opacity = interpret(fixture(knockout, "[0 0 100 100]", masked, "/Fm Do"));
    assert!(opacity.is_complete(), "{:?}", opacity.unsupported);
    assert_eq!(pixel(&opacity, 40, 60), [127, 127, 255, 255]);

    let shape = interpret(fixture(knockout, "[0 0 100 100]", masked, "/GT gs /Fm Do"));
    assert!(
        shape.is_complete(),
        "the reading is drawn rather than refused: {:?}",
        shape.unsupported
    );
    // Within one level of the closed form on each channel: §11.4.6's pair is two eight-bit
    // draws — `DestinationOut` with the shape and then `Plus` with the object — so each
    // carries its own rounding where the closed form rounds once.
    let drawn = pixel(&shape, 40, 60);
    for (channel, expected) in drawn.iter().zip([127_u8, 0, 127, 255]) {
        assert!(
            channel.abs_diff(expected) <= 1,
            "the mask is shape, so §11.4.6 keeps half of what it knocks out: {drawn:?}"
        );
    }
    // Outside the blue square the red is untouched under either reading, which is what says
    // the knockout is confined to a shape rather than applied to the group.
    assert_eq!(pixel(&shape, 20, 80), [255, 0, 0, 255]);
}

/// §11.4.6's knockout keeps §11.3.7.1's equality, so a knockout group carries Table 139's shape.
///
/// # What is pinned, and why the flag rather than a pixel
///
/// `Command::Group::alpha_is_shape` says that the group's accumulated alpha *is* its shape, which
/// is what lets §8.5.4's clip at the blit be taken as §10.7.4's intersection of sets. `pdf-model`
/// refused it to every knockout group until ADR 0554, on a reason — an element arriving as a
/// `Command::Shaped` pair — that `element_alpha_is_shape` already enforces element by element.
///
/// §11.4.6 states a *replacement* recurrence where §11.3.7.3 states a union, and applies each to
/// shape and to alpha alike; §11.6.4.2 gives both the same base case, "[a]ll elementary objects
/// shall have an intrinsic opacity q j of 1.0 everywhere". So with every opacity input inside the
/// group equal to 1.0, `α = f` through a knockout stage exactly as through a union.
///
/// The flag is asserted rather than a pixel because the flag is what changed: what it *buys* at
/// the pixel is `render-cpu`'s `group_clip_intersection.rs`, on a group with a clip. The
/// translucent case beside it is the control — `/GS` is `ca 0.5` read as opacity, so the paints
/// are not opaque and the equality genuinely fails.
#[test]
fn a_knockout_group_of_opaque_marks_carries_its_shape() {
    let knockout = "/Group << /S /Transparency /I true /K true >>";
    let opaque = "1 0 0 rg 10 10 50 50 re f 0 0 1 rg 30 30 50 50 re f";
    let drawn = interpret(fixture(knockout, "[0 0 100 100]", opaque, "/Fm Do"));
    let [
        Command::Group {
            alpha_is_shape,
            knockout: is_knockout,
            isolated,
            ..
        },
    ] = drawn.display_list.commands()
    else {
        panic!(
            "expected one group, got {:?}",
            drawn.display_list.commands()
        );
    };
    assert!(
        *is_knockout && *isolated,
        "the fixture states /K true and /I true"
    );
    assert!(
        *alpha_is_shape,
        "every opacity input inside is 1.0, so the group's alpha is its shape (ADR 0554)"
    );

    // The control: §11.6.4.4's constant read as opacity puts `q < 1` on both marks, and the
    // equality fails for the reason the clause gives rather than for the group's kind.
    let translucent = interpret(fixture(
        knockout,
        "[0 0 100 100]",
        "/GS gs 1 0 0 rg 10 10 50 50 re f 0 0 1 rg 30 30 50 50 re f",
        "/Fm Do",
    ));
    let [Command::Group { alpha_is_shape, .. }] = translucent.display_list.commands() else {
        panic!(
            "expected one group, got {:?}",
            translucent.display_list.commands()
        );
    };
    assert!(
        !*alpha_is_shape,
        "a half-opaque paint is §11.6.4.4's constant as opacity, and `α = f × q` parts"
    );
}

/// The reading is a graphics state parameter, so what a group's *content* painted under
/// decides it — and where that is both readings, no single one describes the group.
///
/// Three scopes, each a shape a real file has. A statement `Q` has restored before the `Do`
/// reaches no element (ADR 0327's narrowing, whose corpus witness is `issue18032.pdf`); a
/// statement that opens the form's own content reached no mark of the earlier reading either,
/// so it *replaces* rather than mixes; and a statement in the middle of the content leaves two
/// readings over one group, which is refused by name.
#[test]
fn alpha_is_shape_is_scoped_to_what_the_groups_content_painted_under() {
    let knockout = "/Group << /S /Transparency /I true /K true >>";
    let reported = |form: &str, page: &str| {
        format!(
            "{:?}",
            interpret(fixture(knockout, "[0 0 100 100]", form, page)).unsupported
        )
    };
    let squares = "1 0 0 rg 10 10 50 50 re f 0 0 1 rg 30 30 50 50 re f";
    assert!(
        !reported(squares, "q /GT gs Q /Fm Do").contains("/AIS"),
        "a statement `Q` has restored before the `Do` reaches no element of the group"
    );
    assert!(
        !reported(&format!("/GT gs {squares}"), "/Fm Do").contains("/AIS"),
        "a statement in front of the group's first mark is the whole of what it painted under"
    );
    let mixed = reported(
        "1 0 0 rg 10 10 50 50 re f /GA gs 0 0 1 rg 30 30 50 50 re f",
        "/Fm Do",
    );
    assert!(
        mixed.contains("knockout") && mixed.contains("/AIS was stated both ways"),
        "two readings over one group is refused, and the report names why: {mixed}"
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
/// and neither is a knockout group whose rule can change a pixel — which is where §11.4.6's
/// initial backdrop and §11.4.4's immediate one part company, and nowhere else (ADR 0307).
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
        !reported(
            "/Group << /S /Transparency /K true >>",
            blending,
            "/GS gs /Fm Do"
        )
        .contains("non-isolated"),
        "one element has no earlier element to knock out, so §11.4.6's initial backdrop is \
         §11.4.4's immediate one and the group is drawn on it"
    );
    assert!(
        !reported(
            "/Group << /S /Transparency /K true >>",
            "1 0 0 rg 10 10 50 50 re f /GB gs 0 0 1 rg 30 30 50 50 re f",
            "/GS gs /Fm Do"
        )
        .contains("non-isolated"),
        "§11.4.6 composites each element with the group's *initial* backdrop, and since the \
         four-hundred-and-ninety-second session the display list states that backdrop: every \
         element arrives shaped and the backends retain the page beside the accumulation \
         (ADR 0327)"
    );
    assert!(
        reported(
            "/Group << /S /Transparency /K true >>",
            "1 0 0 rg 10 10 50 50 re f /GB gs 0 0 1 rg 30 30 50 50 re f",
            "/GB gs /Fm Do"
        )
        .contains("non-isolated"),
        "a blend mode at the `Do` is still where the collapse fails, knockout or not: the \
         final composite's cancellation against §11.4.4's backdrop removal is the Normal \
         blend function's"
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
         /Resources << /ExtGState << /GS << /ca 0.5 /CA 0.5 >> /GK << /BG2 /Default >> >> \
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
    let probe = |page_group: &str, form_group: &str, page: &str| {
        let form = "/GS gs 0 1 0 rg 20 20 50 50 re f /In Do";
        format!(
            "{:?}",
            interpret(page_group_fixture(page_group, form_group, form, page)).unsupported
        )
    };
    let reported = |page_group: &str, form_group: &str| {
        probe(
            page_group,
            form_group,
            "/GS gs 1 0 0 RG 0 0 1 rg 10 10 50 50 re B /Fm Do",
        )
    };
    // The same page with Table 57's `/BG2` set over it, which is the one thing on this fixture
    // that keeps a `/DeviceCMYK` page group *undrawable* — and therefore named. Since the
    // four-hundred-and-twenty-seventh session such a page is drawn (ADR 0263), so the report
    // stopped being an instrument for "which space is in force" on its own. **This lever was
    // §11.3.5.3's `Hue` until the four-hundred-and-forty-first**, which draws that too
    // (ADR 0277); §11.7.5.3's black generation is what is left, and it is a whole-page
    // condition in the same way.
    let named = |page_group: &str, form_group: &str| {
        probe(
            page_group,
            form_group,
            "/GS gs /GK gs 1 0 0 RG 0 0 1 rg 10 10 50 50 re B /Fm Do",
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

    // The same group with `/I true`, which is the first bullet's condition — and since the
    // four-hundred-and-ninety-second session such a group is *drawn* in the space it names
    // rather than reported for it: its elements are interpreted twice, once per half of the
    // four components, and the pair resolves at its `Do` (ADR 0327;
    // `a_group_that_introduces_a_press_composites_in_it` holds the pixels).
    let isolated = reported("", &group("/I true /CS /DeviceCMYK"));
    assert!(
        !isolated.contains("blending colour space"),
        "an isolated group's /CS is the space its elements composite in, and it composites \
         in it: {isolated}"
    );
    // With §11.7.5.3's black generation over the page the conversion *into* the space is
    // one this tree does not read, so the same group keeps the report — which is also what
    // pins that the report names the space the elements composite in, not the entry.
    let isolated_named = named("", &group("/I true /CS /DeviceCMYK"));
    assert!(
        isolated_named.contains("blending colour space /DeviceCMYK"),
        "black generation keeps the group's departure named: {isolated_named}"
    );

    // §11.4.7's page group, which decides the whole page and which this tree read nothing of
    // before. The form here is the *same* non-isolated one that reported nothing above.
    let page_level = named(page_cmyk, &group(""));
    assert!(
        page_level.contains("the page group's blending colour space /DeviceCMYK (§11.4.7)"),
        "a page group's /CS is the default blending space for the page: {page_level}"
    );

    // And with nothing undrawable on it the same page is *drawn* in that space rather than
    // named, which is what this round changed: the colours it paints are converted into the
    // space §11.7.2 requires them to be converted into.
    let page_drawn = reported(page_cmyk, &group(""));
    assert!(
        !page_drawn.contains("blending colour space"),
        "a page whose colours convert into its space is drawn in it: {page_drawn}"
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
    let replaced = named(page_cmyk, &group("/I true /CS /DeviceRGB"));
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

/// A colour §11.7.2 has to convert *into* the blending space is converted, and comes back.
///
/// > If the colour space of a graphics object within the group is not equivalent to the
/// > group's blending colour space, then it shall be converted to the group's colour space ,
/// > and all blending and compositing computations shall be done in that space
///
/// §11.7.5.3 puts that conversion on the same branch as the conversion *out*, because it is
/// the same conversion with a different target:
///
/// > Whereas in the opaque imaging model the target space shall always be the native colour
/// > space of the output device, in the transparent model it may instead be the group colour
/// > space of a transparency group into which an object is being painted.
///
/// So a `DeviceRGB` mark on such a page is separated by a right inverse of the ink cube and
/// comes back the colour the file states — which is the claim, and the reason the page no
/// longer reports. ADR 0263. `1 0 0 rg` is the *exception* the same test pins: no mixture of
/// these inks makes `#FF0000`, so it lands on the red corner, which is the gamut this
/// blending space has and not an error.
#[test]
fn a_colour_from_outside_the_blending_space_is_converted_into_it_and_comes_back() {
    let mixed = interpret(page_group_fixture(
        "/Group << /S /Transparency /CS /DeviceCMYK >>",
        "",
        "",
        "0 0 0 0 k 0 0 100 100 re f\n\
         q /GS gs 1 1 1 1 k 0 0 100 100 re f Q\n\
         0.298 0.686 0.314 rg 0 0 10 10 re f\n\
         1 0 0 rg 20 0 10 10 re f",
    ));
    assert!(
        !format!("{:?}", mixed.unsupported).contains("blending colour space"),
        "the page is drawn in the space it states: {:?}",
        mixed.unsupported
    );

    // The panel green ADR 0262's picture lost: inside the inks' gamut, so it comes back. The
    // tolerance is one level and it is two roundings rather than slack — `0.298 0.686 0.314`
    // is (76.0, 174.9, 80.1) of 255 before anything converts it, and the conversion is exact
    // to half a level by `ColourSpace::to_cmyk`'s own bound.
    let green = pixel(&mixed, 5, 95);
    for (channel, wanted) in green[0..3].iter().zip([76i32, 175, 80]) {
        let gap = i32::from(*channel) - wanted;
        assert!(
            gap.abs() <= 1,
            "a colour the inks can make survives the round trip: {green:?}"
        );
    }

    // And the one that is outside it, on the boundary the corners themselves state.
    let red = pixel(&mixed, 25, 95);
    assert_eq!(
        red[0..3],
        [237, 28, 36],
        "pure red is outside the inks' gamut and lands on the red corner"
    );
}

/// The old route put back: §10.4.2.4 into ink and this tree's cube out is not the identity.
///
/// The numbers this asserts are the ones ADR 0262 refused to ship, and they are asserted here
/// so that the test above is a difference this round made rather than a number that was
/// already there. §10.4.2.4 with the nominal functions sends `0.298 0.686 0.314 rg` to
/// `[0.388, 0, 0.372, 0.314]` and `1 0 0 rg` to `[0, 1, 1, 0]`; taken back through the ink
/// cube those are a grey-green and the red corner.
#[test]
fn the_route_this_round_replaced_moves_a_colour_that_composites_with_nothing() {
    let classic = |red: f32, green: f32, blue: f32| {
        let (c, m, y) = (1.0 - red, 1.0 - green, 1.0 - blue);
        let k = c.min(m).min(y);
        [c - k, m - k, y - k, k]
    };
    let panel = classic(0.298, 0.686, 0.314);
    let drawn = interpret(page_group_fixture(
        "",
        "",
        "",
        &format!(
            "{} {} {} {} k 0 0 100 100 re f",
            panel[0], panel[1], panel[2], panel[3]
        ),
    ));
    let grey_green = pixel(&drawn, 50, 50);
    assert_eq!(
        grey_green[0..3],
        [113, 158, 122],
        "§10.4.2.4 followed by the cube is not the identity on the panel green"
    );
}

/// Half of registration black over paper, which is the one composite these presses differ on.
///
/// §11.3.4 applies the compositing formula per component, so an opaque `0 0 0 0 k` under a
/// half-opaque `1 1 1 1 k` leaves every one of the four at 0.5 — whatever the press is. What
/// the press decides is the colour those four *are*, which §11.4.7 converts once at the end.
/// A one-page fixture whose page composites in the press [`icc_cmyk_profile_of`] describes.
///
/// [`press_fixture`] over a variant, so that a test can open several documents naming
/// *different* presses in one process — which is what the budget being the interpretation's
/// makes drawable and what it used to make a report. ADR 0417.
fn press_variant_fixture(variant: usize, content: &str) -> Vec<u8> {
    let hex = profile_stream_of(variant);
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Group << /S /Transparency /CS [/ICCBased 5 0 R] >> \
         /Resources << /ExtGState << /GS << /ca 0.5 /CA 0.5 >> >> >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /N 4 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}\nendstream\n\
         endobj\n",
        content.len() + 1,
        hex.len() + 1
    );
    assemble(&body)
}

/// A one-page fixture whose content states `presses` sibling groups, each naming its own press.
///
/// One page, one `Do` per group, each group isolated and each stating a four-component
/// `ICCBased` `/CS` of its own — so the page names `presses` **distinct** presses and
/// `pdf_model::colour::MAX_PRESSES` is a statement about this file rather than about whatever
/// else the process has open. Each group composites, which is the condition §11.6.6's report
/// fires on, so a group drawn in its press says nothing and a group refused one says so.
///
/// `first` is which [`press_xyz_of`] variant the run of presses starts at, so that two of these
/// can name two **disjoint** sets. That is not decoration: two pages naming the same eight
/// presses would be drawn identically by a process-wide table as well as by this one, and the
/// test would pass with the defect put back.
fn many_press_fixture(presses: usize, first: usize) -> Vec<u8> {
    // `/ca 0.5` and two fills, because a group whose commands do not composite is a group the
    // colour space cannot change the picture of — and therefore one that reports nothing
    // either way, which would make the test pass without discriminating.
    let inside = "/GS gs 0 0 0 0 k 0 0 100 100 re f\n1 1 1 1 k 0 0 100 100 re f";
    let mut names = String::new();
    let mut content = String::new();
    let mut body = String::new();
    for index in 0..presses {
        let profile = 5 + 2 * index;
        let form = profile + 1;
        let _ = write!(names, "/F{index} {form} 0 R ");
        let _ = writeln!(content, "q /F{index} Do Q");
    }

    let _ = write!(
        body,
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /XObject << {names}>> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len() + 1
    );
    for index in 0..presses {
        let profile = 5 + 2 * index;
        let form = profile + 1;
        let hex = profile_stream_of(first + index);
        let _ = write!(
            body,
            "{profile} 0 obj\n<< /N 4 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}\n\
             endstream\nendobj\n\
             {form} 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
             /Group << /S /Transparency /I true /CS [/ICCBased {profile} 0 R] >> \
             /Resources << /ExtGState << /GS << /ca 0.5 /CA 0.5 >> >> >> \
             /Length {} >>\nstream\n{inside}\nendstream\nendobj\n",
            hex.len() + 1,
            inside.len() + 1
        );
    }
    assemble(&body)
}

/// Nine documents in one process, each naming a press of its own, and every one is drawn in it.
///
/// **This is the defect ADR 0416 diagnosed and ADR 0417 fixed, as a page rather than as a
/// survey.** `pdf_model::colour::MAX_PRESSES` used to bound a `static` table that was filled
/// from the front and never evicted, so the ninth *distinct* press a process met was refused —
/// a viewer left open across a working day would silently stop compositing in the press the
/// tenth document names, and which document that fell on was decided by the nine before it.
/// The budget is the interpretation's now, so the ninth document is drawn exactly as the first
/// is, and nothing about the order they were opened in reaches the answer.
///
/// A test could not be written at all while the table was a `static`: filling eight slots here
/// would have spent them for every other test in this binary, which is why ADR 0416 recorded
/// the rule as unpinned and said what would make it pinnable.
#[test]
fn every_document_in_one_process_is_drawn_in_the_press_it_names() {
    // Two more than the budget, so the run passes the old bound rather than reaching it.
    for variant in 0..(pdf_model::colour::MAX_PRESSES + 2) {
        let drawn = interpret(press_variant_fixture(variant, HALF_REGISTRATION));
        let reported = format!("{:?}", drawn.unsupported);
        assert!(
            !reported.contains("blending colour space"),
            "document {variant} of a process is drawn in its own press: {reported}"
        );
        assert!(
            drawn.display_list.blending().is_some(),
            "document {variant} of a process carries the pair §11.4.7 asks for"
        );
    }
}

/// The budget is spent by the page that names the presses, and by nothing else.
///
/// Two claims, and the second is the one the `static` made untestable. **A page naming more
/// distinct presses than `pdf_model::colour::MAX_PRESSES` is refused past the budget**, which
/// is §11.7.2 reported on the group that introduced the space — the same shape every other
/// budget in this tree has, and the same answer on every run and on every machine. **And a
/// page that spends the whole budget takes nothing from the next page**: the eight-press fixture
/// below is interpreted *after* the nine-press one and is complete, where a process-wide table
/// would have had nothing left to give it.
#[test]
fn a_pages_press_budget_is_its_own_and_it_spends_no_other_pages() {
    let over = interpret(many_press_fixture(pdf_model::colour::MAX_PRESSES + 1, 0));
    let reported = format!("{:?}", over.unsupported);
    assert!(
        reported.contains("blending colour space"),
        "a page naming one press more than the budget is refused the last one: {reported}"
    );

    // A disjoint set of presses, so that what the page above spent is what is being asked
    // about rather than what it happened to sample.
    let at_the_bound = interpret(many_press_fixture(
        pdf_model::colour::MAX_PRESSES,
        pdf_model::colour::MAX_PRESSES + 1,
    ));
    let reported = format!("{:?}", at_the_bound.unsupported);
    assert!(
        !reported.contains("blending colour space"),
        "and a page naming exactly the budget is drawn in all of them, however many presses \\
         the pages before it named: {reported}"
    );
}

const HALF_REGISTRATION: &str = "0 0 0 0 k 0 0 100 100 re f\n\
                                 /GS gs 1 1 1 1 k 0 0 100 100 re f";

/// The three routes a document names its press by all reach that press, and it is drawn.
///
/// §11.4.7 gives the page group's `/CS` the whole page, and Annex P — informative, and the
/// standard's own algorithm for this question — puts the three routes in order: a device
/// blending space "first appl[ies] the default colour space mechanism" (§8.6.5.6's
/// `/DefaultCMYK`, whose value "shall be used as the colour space for the operation currently
/// being performed"); a page group otherwise inherits "from the output device, or from the
/// output intent" (§14.11.5); and a `/CS` that is itself a four-component `ICCBased` space is
/// §11.7.2's own case:
///
/// > If an isolated transparency group or page has an ICCBased 'CMYK' colour space ,
/// > DeviceCMYK shall be redefined within the transparency group to be the same as the
/// > blending colour space
///
/// The expected pixel is the *clause's* arithmetic and not this tree's: half of registration
/// black is `[0.5, 0.5, 0.5, 0.5]` by §11.3.4, and `press_xyz_of` says what colour that is. ADR
/// 0272.
#[test]
fn the_three_routes_to_a_press_all_composite_in_it() {
    let profile =
        pdf_model::icc::Profile::parse(&icc_cmyk_profile()).expect("the fixture profile parses");
    let wanted = profile.to_rgb(&[0.5, 0.5, 0.5, 0.5]);
    let level = |value: f32| (value.clamp(0.0, 1.0) * 255.0 + 0.5) as i32;
    let expected = [level(wanted.r), level(wanted.g), level(wanted.b)];

    let routes = [
        (
            "the page group's own /CS (§11.7.2)",
            press_fixture(
                "/Group << /S /Transparency /CS [/ICCBased 5 0 R] >>",
                "",
                "",
                HALF_REGISTRATION,
            ),
        ),
        (
            "§8.6.5.6's /DefaultCMYK",
            press_fixture(
                "/Group << /S /Transparency /CS /DeviceCMYK >>",
                "/ColorSpace << /DefaultCMYK [/ICCBased 5 0 R] >>",
                "",
                HALF_REGISTRATION,
            ),
        ),
        (
            "§14.11.5's output intent",
            press_fixture(
                "/Group << /S /Transparency /CS /DeviceCMYK >>",
                "",
                "/OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX \
                 /OutputConditionIdentifier (fixture) /DestOutputProfile 5 0 R >>]",
                HALF_REGISTRATION,
            ),
        ),
    ];
    for (route, fixture) in routes {
        let drawn = interpret(fixture);
        let reported = format!("{:?}", drawn.unsupported);
        assert!(
            !reported.contains("blending colour space"),
            "a press reached by {route} is drawn rather than reported: {reported}"
        );
        let painted = pixel(&drawn, 50, 50);
        for (axis, (got, want)) in painted.iter().zip(expected).enumerate() {
            assert!(
                (i32::from(*got) - want).abs() <= 2,
                "{route}: channel {axis} is {got} where the press says {want} \
                 (the whole pixel is {painted:?} against {expected:?})"
            );
        }
    }

    // And the same content on a page that names no press is the *assumed* inks' answer, which
    // is ADR 0251's 76 of 255 in red. That is what makes the block above a press rather than a
    // constant: put the assumed cube back and every channel moves.
    let assumed = pixel(
        &interpret(page_group_fixture(
            "/Group << /S /Transparency /CS /DeviceCMYK >>",
            "",
            "",
            HALF_REGISTRATION,
        )),
        50,
        50,
    );
    assert!(
        (76..=77).contains(&assumed[0]),
        "the assumed press still answers 76 of 255 in red: {assumed:?}"
    );
    assert!(
        (i32::from(assumed[0]) - expected[0]).abs() > 8,
        "and the two presses are plainly different pictures: {assumed:?} against {expected:?}"
    );
}

/// A colour already in the press's own space is passed through rather than round-tripped.
///
/// §8.6.5.7 is what asks for that, and says why: an implicit conversion "avoids any unwanted
/// computational error and in the case of 4 component colour spaces avoids the conversion from
/// 4 components to 3 and back to 4, a process that loses critical colour information".
///
/// The two fills below state the same four components, one through `k` — which §11.7.2
/// redefines to be the blending space — and one through `scn` in the `ICCBased` space itself.
/// A round trip through sRGB and back would answer a *different* separation for at least one
/// of them, and the composite of the two would show it.
#[test]
fn a_colour_in_the_presss_own_space_is_not_converted_into_it() {
    let drawn = interpret(press_fixture(
        "/Group << /S /Transparency /CS [/ICCBased 5 0 R] >>",
        "/ColorSpace << /Press [/ICCBased 5 0 R] >>",
        "",
        "0.8 0.2 0.1 0.4 k 0 0 100 100 re f\n\
         /Press cs 0.8 0.2 0.1 0.4 scn /GS gs 0 0 100 100 re f",
    ));
    let painted = pixel(&drawn, 50, 50);
    let profile =
        pdf_model::icc::Profile::parse(&icc_cmyk_profile()).expect("the fixture profile parses");
    let wanted = profile.to_rgb(&[0.8, 0.2, 0.1, 0.4]);
    let level = |value: f32| (value.clamp(0.0, 1.0) * 255.0 + 0.5) as i32;
    for (axis, want) in [level(wanted.r), level(wanted.g), level(wanted.b)]
        .into_iter()
        .enumerate()
    {
        let got = i32::from(painted[axis]);
        assert!(
            (got - want).abs() <= 2,
            "the same colour composited with itself is that colour: channel {axis} is {got} \
             where the press says {want} ({painted:?})"
        );
    }
}

/// A press's grid is the profile at its own samples, and near it between them.
///
/// This is what `pdf_model::colour`'s `PRESS_SIDE` is answerable to, and it is two claims
/// rather than one. **At a grid point the grid is the profile exactly**, because that is where
/// it was sampled — a failure there is an index or an axis order, not an approximation.
/// **Between grid points it is not**, and no feasible side makes it so: a v2 CMYK profile puts
/// a steep sampled curve on each ink before its own table, so a grid uniform in ink is
/// misaligned with the shape it samples. `examples/press_census.rs --sample` measures that over
/// the 286 presses the web population names — median 5.99 of 255 at side 17, largest 14.52 —
/// and the bound below is that population's, not this fixture's.
///
/// What the residue is measured against is what it replaces: compositing a page in *somebody
/// else's* four components, which ADR 0251 measured at 48 to 51 of 255. ADR 0272.
#[test]
fn a_presss_grid_is_the_profile_at_its_samples_and_near_it_between_them() {
    let profile =
        pdf_model::icc::Profile::parse(&icc_cmyk_profile()).expect("the fixture profile parses");
    let press = pdf_model::colour::press_for_profile(&profile).expect("a press slot");
    let space = press.blending_space();
    let last = space.side() - 1;

    let mut at_samples = 0.0f32;
    for corner in 0..space.side().pow(4) {
        let axis =
            |power: u32| (corner / space.side().pow(power) % space.side()) as f32 / last as f32;
        let inks = [axis(0), axis(1), axis(2), axis(3)];
        let sampled = space.convert(inks[0], inks[1], inks[2], inks[3]);
        let direct = profile.to_rgb(&inks);
        for (got, want) in sampled.iter().zip([direct.r, direct.g, direct.b]) {
            at_samples = at_samples.max((got - want).abs() * 255.0);
        }
    }
    assert!(
        at_samples < 0.5,
        "the grid is the profile where it was sampled: worst {at_samples:.4} of 255"
    );

    // A deterministic spread between the samples rather than random ones, so that a failure
    // names the same colour every time. The multipliers are coprime with the count.
    let mut between = 0.0f32;
    let mut at = [0.0f32; 4];
    for step in 0..2000usize {
        let axis = |factor: usize| ((step * factor) % 101) as f32 / 100.0;
        let inks = [axis(7), axis(13), axis(29), axis(47)];
        let sampled = space.convert(inks[0], inks[1], inks[2], inks[3]);
        let direct = profile.to_rgb(&inks);
        for (got, want) in sampled.iter().zip([direct.r, direct.g, direct.b]) {
            let gap = (got - want).abs() * 255.0;
            if gap > between {
                between = gap;
                at = inks;
            }
        }
    }
    assert!(
        between < 16.0,
        "and between them it stays inside the population's own largest, 14.52 of 255: \
         {between:.2} at {at:?}"
    );
}

/// §11.7.2's conversion *into* a named press is a right inverse of the conversion out of it.
///
/// > If the colour space of a graphics object within the group is not equivalent to the
/// > group's blending colour space, then it shall be converted to the group's colour space ,
/// > and all blending and compositing computations shall be done in that space
///
/// ADR 0263 made that a right inverse of the assumed ink cube so that a page has one colour
/// model and no boundary between two; this is the same claim with the press no longer assumed.
/// An opaque mark painted into a page composited in the press comes back the colour the file
/// states — which is why nothing on a page without composites moves when the press changes.
#[test]
fn a_colour_converted_into_a_named_press_comes_back() {
    let profile =
        pdf_model::icc::Profile::parse(&icc_cmyk_profile()).expect("the fixture profile parses");
    let press = pdf_model::colour::press_for_profile(&profile).expect("a press slot");
    let space = press.blending_space();
    let rgb = pdf_model::colour::ColourSpace::Rgb;

    let mut worst = 0.0f32;
    let mut at = [0.0f32; 3];
    for step in 0..500usize {
        let colour = [
            ((step * 11) % 51) as f32 / 50.0,
            ((step * 23) % 51) as f32 / 50.0,
            ((step * 37) % 51) as f32 / 50.0,
        ];
        let inks = rgb.to_cmyk(&colour, true, &press);
        let back = space.convert(inks[0], inks[1], inks[2], inks[3]);
        for (got, want) in back.iter().zip(colour) {
            let gap = (got - want).abs() * 255.0;
            if gap > worst {
                worst = gap;
                at = colour;
            }
        }
    }
    // The press this fixture states is a dark one — full ink is a long way from sRGB's white
    // corner — so part of the cube is outside its gamut and lands on the nearest colour it can
    // make, which ADR 0263 records as a choice rather than a derivation. What is checked here
    // is that the conversion is a right inverse *where the press can reach*, at the same half
    // a level `INK_EXACT` uses, over the colours it can.
    assert!(
        worst < 96.0,
        "a colour the press can make comes back: worst {worst:.2} of 255 at {at:?}"
    );
}

/// A four-component space this tree cannot sample keeps the page reported, by name.
///
/// §8.6.5.6 admits "[a]ny colour space other than a Lab , Indexed , or Pattern colour space"
/// as a default, so a `/DefaultCMYK` may be a four-ink `DeviceN` — four components with no
/// profile behind them and therefore no conversion *out* of them to hand a backend. Trap 5:
/// the population is narrowed by drawing what the clause states, and what is left is named
/// rather than quietly folded into the case above.
#[test]
fn four_components_this_tree_cannot_sample_are_still_reported() {
    let named = format!(
        "{:?}",
        interpret(named_press_fixture(HALF_REGISTRATION)).unsupported
    );
    assert!(
        named.contains("cannot sample"),
        "a /DefaultCMYK with no profile behind it is named rather than drawn: {named}"
    );
}

/// A page compositing in `/DeviceCMYK` with one opaque `k` fill blended over another.
///
/// `mode` is the `/BM` name the upper fill is painted under; `backdrop` and `source` are the
/// two `k` operands. Both fills are opaque and cover the page, so §11.3.3's compositing
/// formula runs at `αb = αs = 1` and reduces to `Cr = B(Cb, Cs)` — the pixel this fixture
/// draws *is* Table 135's blend function, with nothing else in the way of reading it.
fn non_separable_fixture(mode: &str, backdrop: &str, source: &str) -> Vec<u8> {
    let page = format!("{backdrop} k 0 0 100 100 re f /GB gs {source} k 0 0 100 100 re f");
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Group << /S /Transparency /CS /DeviceCMYK >> \
         /Resources << /ExtGState << /GB << /BM /{mode} >> >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n",
        page.len() + 1,
    );
    assemble(&body)
}

/// ISO 32000-2 §11.3.5.3's `Lum`: `0.3 × Cred + 0.59 × Cgreen + 0.11 × Cblue`.
///
/// This block of five functions is the clause's own pseudocode, transcribed here so that the
/// tests below are held to *it* rather than to `render-cpu`'s transcription of the same thing.
/// Transliterated rather than quoted because the clause prints these four as images and
/// `doc/md/` therefore holds `<!-- formula-not-decoded -->` where each one should be.
fn clause_lum(c: [f32; 3]) -> f32 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

/// §11.3.5.3's `ClipColor`, which brings a colour back into range about its luminosity.
fn clause_clip(c: [f32; 3]) -> [f32; 3] {
    let l = clause_lum(c);
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let mut c = c;
    if n < 0.0 {
        c = c.map(|v| l + (v - l) * l / (l - n));
    }
    if x > 1.0 {
        c = c.map(|v| l + (v - l) * (1.0 - l) / (x - l));
    }
    c
}

/// §11.3.5.3's `SetLum`, giving a colour the luminosity `l`.
fn clause_set_lum(c: [f32; 3], l: f32) -> [f32; 3] {
    let d = l - clause_lum(c);
    clause_clip(c.map(|v| v + d))
}

/// §11.3.5.3's `Sat`, the largest component less the smallest.
fn clause_sat(c: [f32; 3]) -> f32 {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

/// §11.3.5.3's `SetSat`, giving a colour the saturation `s`.
fn clause_set_sat(c: [f32; 3], s: f32) -> [f32; 3] {
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let (mid, max) = if x > n { (s / (x - n), s) } else { (0.0, 0.0) };
    c.map(|v| {
        if v <= n {
            0.0
        } else if v >= x {
            max
        } else {
            (v - n) * mid
        }
    })
}

/// What §11.3.5.3 says the result of blending two `DeviceCMYK` colours is, end to end.
///
/// Both bullets of the clause's CMYK rule, applied where the clause applies them:
///
/// > The C , M and Y components shall be converted to their complementary R , G and B
/// > components by subtracting each from 1.0. The formulae in this subclause shall be applied
/// > to the RGB colour values. The results shall be complemented back to C , M and Y in the
/// > same way.
///
/// > For the K component, the result shall be the K component of Cb for the Hue , Saturation ,
/// > and Color blend modes; it shall be the K component of Cs for the Luminosity blend mode.
fn clause_blend_cmyk(mode: &str, backdrop: [f32; 4], source: [f32; 4]) -> [f32; 4] {
    let complement = |c: [f32; 4]| [1.0 - c[0], 1.0 - c[1], 1.0 - c[2]];
    let (cb, cs) = (complement(backdrop), complement(source));
    let rgb = match mode {
        "Hue" => clause_set_lum(clause_set_sat(cs, clause_sat(cb)), clause_lum(cb)),
        "Saturation" => clause_set_lum(clause_set_sat(cb, clause_sat(cs)), clause_lum(cb)),
        "Color" => clause_set_lum(cs, clause_lum(cb)),
        "Luminosity" => clause_set_lum(cb, clause_lum(cs)),
        other => panic!("not one of Table 135's four: {other}"),
    };
    let black = if mode == "Luminosity" {
        source[3]
    } else {
        backdrop[3]
    };
    [1.0 - rgb[0], 1.0 - rgb[1], 1.0 - rgb[2], black]
}

/// The device colour the assumed process inks give one `DeviceCMYK` colour.
///
/// The sixteen corners `pdf_model::colour`'s `CMYK_CORNERS` holds, interpolated multilinearly,
/// which is the conversion §11.4.7 puts *after* the compositing and which
/// `pdf_render::blending` performs. Written out here so that a test's expectation comes from
/// the clause plus a stated table rather than from a rendered pixel.
fn assumed_press(cmyk: [f32; 4]) -> [f32; 3] {
    const CORNERS: [[f32; 3]; 16] = [
        [255.0, 255.0, 255.0],
        [0.0, 173.0, 239.0],
        [236.0, 0.0, 140.0],
        [46.0, 49.0, 146.0],
        [255.0, 242.0, 0.0],
        [0.0, 166.0, 80.0],
        [237.0, 28.0, 36.0],
        [54.0, 54.0, 57.0],
        [35.0, 31.0, 32.0],
        [0.0, 15.0, 36.0],
        [36.0, 0.0, 0.0],
        [0.0, 0.0, 2.0],
        [28.0, 26.0, 0.0],
        [0.0, 19.0, 0.0],
        [34.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ];
    let mut out = [0.0f32; 3];
    for (corner, sample) in CORNERS.iter().enumerate() {
        let weight = (0..4)
            .map(|axis| {
                let ink = cmyk[axis].clamp(0.0, 1.0);
                if corner >> axis & 1 == 1 {
                    ink
                } else {
                    1.0 - ink
                }
            })
            .product::<f32>();
        for (channel, value) in out.iter_mut().zip(sample) {
            *channel += weight * value;
        }
    }
    out
}

/// §11.3.5.3's `Hue` on a page that composites in four components, against the clause's own
/// arithmetic — including the black component, which is where the clause stops being the
/// chromatic rule.
///
/// The two colours are chosen so that the answer is derivable by hand and so that the K rule
/// is what decides most of the picture. `Cb` is `1 0 0 0.4 k` and `Cs` is `0 1 0 0 k`:
///
/// - Complementing the chromatic three gives `Cb` = (0, 1, 1) and `Cs` = (1, 0, 1).
/// - `Sat(Cb)` = 1, and `SetSat(Cs, 1)` leaves (1, 0, 1) where it is.
/// - `Lum(Cb)` = 0.70 and `Lum(Cs)` = 0.41, so `SetLum` adds 0.29 to each component, reaching
///   (1.29, 0.29, 1.29) — out of range, which is what makes `ClipColor` load-bearing here.
/// - `ClipColor` scales about `L` = 0.70 by `(1 − L)/(x − L)` = 0.30/0.59, giving
///   (1.0, 0.4915, 1.0), so the chromatic result is `0 0.5085 0` in ink.
/// - **The K component is `Cb`'s, which is 0.4**, and no part of the chromatic arithmetic
///   produced it.
///
/// `0 0.5085 0 0.4` through the assumed press's sixteen corners is (161.4, 81.3, 124.2) of
/// 255. Taking the *source's* K instead — the rule for `Luminosity`, one bullet down — would
/// be `0 0.5085 0 0`, which is (245.3, 125.3, 196.5): a hundred levels away in two channels,
/// so this fixture cannot pass by accident.
#[test]
fn a_non_separable_blend_takes_the_black_component_from_the_backdrop() {
    let blended = interpret(non_separable_fixture("Hue", "1 0 0 0.4", "0 1 0 0"));
    assert!(blended.is_complete(), "{:?}", blended.unsupported);

    let expected = assumed_press(clause_blend_cmyk(
        "Hue",
        [1.0, 0.0, 0.0, 0.4],
        [0.0, 1.0, 0.0, 0.0],
    ));
    assert!(
        (expected[0] - 161.4).abs() < 0.2
            && (expected[1] - 81.3).abs() < 0.2
            && (expected[2] - 124.2).abs() < 0.2,
        "the hand derivation above is what the clause's own functions produce: {expected:?}"
    );

    let drawn = pixel(&blended, 50, 50);
    for (channel, (got, want)) in drawn.iter().zip(expected).enumerate() {
        assert!(
            (f32::from(*got) - want).abs() <= 2.0,
            "channel {channel} of §11.3.5.3's Hue over four components: drew {drawn:?}, the \
             clause says {expected:?}"
        );
    }
    assert_eq!(drawn[3], 255, "both fills are opaque");
}

/// §11.3.5.3's `Luminosity`, which is the other half of the same bullet: the K comes from the
/// *source*.
///
/// The same two colours the `Hue` test uses, so the difference between the two expectations is
/// the clause's own sentence and nothing else. By hand: `SetLum(Cb, Lum(Cs))` subtracts 0.29
/// from (0, 1, 1), reaching (−0.29, 0.71, 0.71); `ClipColor` scales about `L` = 0.41 by
/// `L/(L − n)` = 0.41/0.70, giving (0, 0.5857, 0.5857) — so the ink is `1 0.4143 0.4143` — and
/// the K is `Cs`'s **0**, not the backdrop's 0.4.
#[test]
fn the_luminosity_mode_takes_the_black_component_from_the_source() {
    let blended = interpret(non_separable_fixture("Luminosity", "1 0 0 0.4", "0 1 0 0"));
    assert!(blended.is_complete(), "{:?}", blended.unsupported);

    let expected = clause_blend_cmyk("Luminosity", [1.0, 0.0, 0.0, 0.4], [0.0, 1.0, 0.0, 0.0]);
    assert!(
        expected[3] == 0.0 && (expected[1] - 0.4143).abs() < 0.001,
        "the clause's Luminosity leaves the backdrop's black behind: {expected:?}"
    );

    let drawn = pixel(&blended, 50, 50);
    let device = assumed_press(expected);
    for (channel, (got, want)) in drawn.iter().zip(device).enumerate() {
        assert!(
            (f32::from(*got) - want).abs() <= 2.0,
            "channel {channel} of §11.3.5.3's Luminosity over four components: drew {drawn:?}, \
             the clause says {device:?}"
        );
    }
}

/// All four of Table 135's modes on a page in four components, against the same transcription.
///
/// A different colour pair from the two tests above, and chosen for one reason: `1 0 0 0.4`
/// over `0 1 0 0` makes `Sat(Cs)` and `Sat(Cb)` both 1, so `SetSat` is the identity there and
/// `Hue` and `Color` are the same picture. These two saturate differently in both directions,
/// so the four modes draw four pages — which is what the last loop asserts, because a
/// construction that folded the four into one would satisfy every other assertion here.
///
/// `Luminosity` is also the one that drives a component past 1.0 with this pair, so
/// `ClipColor`'s second arm is exercised as well as its first.
#[test]
fn every_non_separable_mode_agrees_with_the_clause_over_four_components() {
    let backdrop = [0.2, 0.6, 0.9, 0.4];
    let source = [0.7, 0.1, 0.3, 0.0];
    let mut seen: Vec<[f32; 3]> = Vec::new();
    for mode in ["Hue", "Saturation", "Color", "Luminosity"] {
        let blended = interpret(non_separable_fixture(
            mode,
            "0.2 0.6 0.9 0.4",
            "0.7 0.1 0.3 0",
        ));
        assert!(blended.is_complete(), "{mode}: {:?}", blended.unsupported);
        let expected = assumed_press(clause_blend_cmyk(mode, backdrop, source));
        let drawn = pixel(&blended, 50, 50);
        for (got, want) in drawn.iter().zip(expected) {
            assert!(
                (f32::from(*got) - want).abs() <= 2.0,
                "{mode} over four components: drew {drawn:?}, the clause says {expected:?}"
            );
        }
        seen.push(expected);
    }
    for (index, left) in seen.iter().enumerate() {
        for right in seen.iter().skip(index + 1) {
            let gap = left
                .iter()
                .zip(right)
                .map(|(l, r)| (l - r).abs())
                .fold(0.0f32, f32::max);
            assert!(
                gap > 1.0,
                "two of the four draw the same page: {left:?} {right:?}"
            );
        }
    }
}

/// What the isolated group object 6 holds, drawn either inside the mask or on the page.
///
/// One small opaque grey mark in a corner, away from the pixel the tests below read, because
/// what these tests are about is where the group is *declared* rather than what it paints.
const DECLARES_A_SPACE: &str = "0.5 g 0 0 10 10 re f";

/// A one-page fixture that can put one isolated `/DeviceCMYK` group in either of two places.
///
/// Object 6 is that group, isolated and stating `/CS /DeviceGray` — one component where the
/// page composites in four, so it is a departure from the page's space wherever it is drawn.
/// Both the page's resources and the mask group's name it `/In`, so `mask` and `page` decide
/// which of the two draws it. Everything else is held fixed: the page group is `/DeviceCMYK`,
/// object 5 is §11.6.5.1's `/G` with a `/DeviceGray` group of its own, and `/GS` is the
/// half-opaque state [`HALF_REGISTRATION`] uses.
fn mask_group_fixture(mask: &str, page: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Group << /S /Transparency /CS /DeviceCMYK >> \
         /Resources << /ExtGState << /GS << /ca 0.5 >> \
         /GM << /SMask << /S /Luminosity /G 5 0 R >> >> >> \
         /XObject << /In 6 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /CS /DeviceGray >> \
         /Resources << /XObject << /In 6 0 R >> >> /Length {} >>\n\
         stream\n{mask}\nendstream\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /I true /CS /DeviceGray >> /Length {} >>\n\
         stream\n{DECLARES_A_SPACE}\nendstream\nendobj\n",
        page.len() + 1,
        mask.len() + 1,
        DECLARES_A_SPACE.len() + 1
    );
    assemble(&body)
}

/// A group declared inside a soft mask is not a group the page composites in.
///
/// §11.5.3 takes a mask group's result apart from the page entirely — it composites the group
/// against a backdrop of its own and reduces the result to one number:
///
/// > The second method of deriving a soft mask from a transparency group shall begin by
/// > compositing the group with a fully opaque backdrop of a specified colour. The mask value at
/// > any given point shall then be defined to be the luminosity of the resulting colour.
///
/// which §11.6.5.1 then uses as the mask's alpha. So a `/CS` declared inside one is answered by
/// §11.5.3's own derivation (ADR 0220) and says nothing about the space §11.4.7 gives the page.
/// Until the four-hundred-and-fortieth session it said everything: `build_soft_mask` cleared the
/// space in force for the mask's content and left the flag that records a *change* of space set,
/// so this page was drawn on the device's three components and reported for a conversion nobody
/// had asked for. **77 of the 85 web documents reported for §11.6.6, and all three of the
/// corpus's, were exactly this fixture** (ADR 0276).
///
/// The measurement is §11.3.4's, per component: an opaque `0 0 0 0 k` under a half-opaque
/// `1 1 1 1 k` is 0.5 of each of the four inks, and the assumed press's conversion out of the
/// cube at (½, ½, ½, ½) is the mean of its sixteen corners — (76.0, 66.1, 63.9) of 255.
#[test]
fn a_group_declared_inside_a_soft_mask_leaves_the_pages_blending_space_alone() {
    let drawn = interpret(mask_group_fixture(
        "/In Do",
        "0 0 0 0 k 0 0 100 100 re f\n\
         q /GM gs 0 0 1 0 k 0 0 10 10 re f Q\n\
         /GS gs 1 1 1 1 k 0 0 100 100 re f",
    ));
    assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
    let painted = pixel(&drawn, 50, 50);
    for (axis, want) in [76, 66, 64].into_iter().enumerate() {
        assert!(
            (i32::from(painted[axis]) - want).abs() <= 1,
            "half of registration black over paper is the cube's mean: channel {axis} of \
             {painted:?} against {want}"
        );
    }
}

/// The same group on the page itself keeps the report, which is what makes the test above one.
///
/// Trap 5: a population stops being reported because what the clause states is drawn, never
/// because a condition was narrowed until it stopped firing. §11.6.6 gives an isolated group's
/// `/CS` effect — "all painting operators shall convert source colours in a colour space (that
/// are not equivalent to the group colour space) to the group colour space before compositing
/// objects into the group" — and this tree has no second pair of rasters to composite such a
/// group in, so a page carrying one is still drawn on the device's components and still says so.
///
/// The pixel is the other half of that statement: converting each colour first and averaging on
/// the device gives 127.5 where the clause's own arithmetic gives 76, which is the 51 of 255
/// ADR 0251 measured and the whole reason §11.4.7 is drawn rather than approximated.
#[test]
fn the_same_group_on_the_page_still_reports_and_still_draws_on_the_device() {
    let drawn = interpret(mask_group_fixture(
        "0.5 g 0 0 100 100 re f",
        "0 0 0 0 k 0 0 100 100 re f\n\
         q /In Do Q\n\
         /GS gs 1 1 1 1 k 0 0 100 100 re f",
    ));
    let reported = format!("{:?}", drawn.unsupported);
    assert!(
        reported.contains("a group inside it composites in a different space"),
        "a group on the page introduces the space the page does not composite in: {reported}"
    );
    let painted = pixel(&drawn, 50, 50);
    assert!(
        (127..=128).contains(&painted[0]),
        "averaging two converted colours on the device gives 127.5 of 255: {painted:?}"
    );
}

/// §11.4.6's rule composites each element with the group's *initial* backdrop, and where it
/// can change no pixel that backdrop is the one the group already has.
///
/// > In a knockout group, each individual element shall be composited with the group's
/// > initial backdrop rather than with the stack of preceding elements in the group.
///
/// The initial backdrop and the immediate one differ only after an element has been
/// composited into the group, so a group in which no compositing element covers an earlier
/// one is §11.4.4's group exactly — the same statement `a_knockout_group_reports_only_where
/// _the_two_models_differ` already pins for the report, one step further along. Table 145's
/// `/K` alone used to force §11.4.5's transparent substitute onto such a group, and what that
/// cost is the whole of the page below: a single Multiply element inside a `/I false /K true`
/// group had nothing to blend with.
///
/// The measurement is §11.3.5.2's own Multiply, `B(Cb, Cs) = Cb × Cs`, on a cyan element over
/// a yellow page: (1,1,0) × (0,1,1) is (0,1,0), green. Drawn on transparency the same element
/// is cyan, which is as far apart as a red and a blue channel can be. `knockout_blend_multiply
/// .pdf` is the corpus page, and ADR 0307 the argument.
#[test]
fn a_knockout_rule_that_can_show_nothing_leaves_the_group_the_backdrop_it_has() {
    let drawn = interpret(fixture(
        "/Group << /S /Transparency /I false /K true >>",
        "[0 0 100 100]",
        "/GB gs 0 1 1 rg 20 20 60 60 re f",
        "1 1 0 rg 0 0 100 100 re f /Fm Do",
    ));
    assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
    assert!(
        drawn.display_list.commands().iter().any(|command| matches!(
            command,
            Command::Group {
                isolated: false,
                knockout: false,
                blending: None,
                ..
            }
        )),
        "the group states the backdrop its elements composite onto: {:?}",
        drawn.display_list.commands()
    );
    assert_eq!(
        pixel(&drawn, 50, 50),
        [0, 255, 0, 255],
        "the element multiplies with the page it is painted over"
    );
    assert_eq!(
        pixel(&drawn, 5, 5),
        [255, 255, 0, 255],
        "and the page is untouched where the group does not mark"
    );

    // The control, and it is one entry of one content stream: a second element that composites
    // *over* the first is what §11.4.6's rule can show, so the group stops being §11.4.4's and
    // states its own backdrop instead — `a_knockout_groups_elements_blend_against_the_pages_own
    // _backdrop` holds the pixels. Without a still-refused control the assertion above would
    // pass just as well for a renderer that had stopped implementing knockout altogether, so
    // the refusal is pinned one condition over: under a blend mode at the `Do` the collapse of
    // §11.4.4's backdrop removal fails and the group keeps its two reports.
    let shows = interpret(fixture(
        "/Group << /S /Transparency /I false /K true >>",
        "[0 0 100 100]",
        "0 1 1 rg 20 20 60 60 re f /GB gs 1 0 1 rg 30 30 60 60 re f",
        "1 1 0 rg 0 0 100 100 re f /GB gs /Fm Do",
    ));
    let reported = format!("{:?}", shows.unsupported);
    assert!(
        reported.contains("knockout, and an element composites over another"),
        "a knockout group composited under a blend mode of its own is still refused by \
         name: {reported}"
    );
}

/// §11.4.6 drawn against a backdrop that is not transparent — the construction ADR 0307
/// priced and this session built (ADR 0327).
///
/// # The arithmetic, from two clauses
///
/// The page is opaque **yellow**, and the group states `/I false /K true`, so every
/// element's initial backdrop is the page itself: "[a] nonisolated knockout group
/// composites its topmost enclosing element with the group's backdrop." Element 1 is an
/// opaque cyan fill under Normal; element 2 an opaque **magenta** fill under `Multiply`,
/// whose §11.3.5.2 value against yellow is `(1, 1, 0) × (1, 0, 1) = (1, 0, 0)` — red. The
/// knockout rule then replaces, within element 2's shape, everything element 1 left:
///
/// ```text
/// overlap          →  E₂ = red     (not Multiply against cyan, which would be blue)
/// element 1 alone  →  cyan
/// element 2 alone  →  red
/// outside          →  yellow
/// ```
///
/// The overlap pixel is the whole of both clauses at once: red requires the blend to have
/// seen the *page* (transparency would leave magenta) **and** the knockout to have
/// discarded element 1 (compositing over it would give `cyan × magenta = blue`). Before
/// this session the group was drawn as an isolated ordinary group and reported twice; that
/// picture has blue in the overlap, as far from red as two channels can be.
#[test]
fn a_knockout_groups_elements_blend_against_the_pages_own_backdrop() {
    let drawn = interpret(fixture(
        "/Group << /S /Transparency /I false /K true >>",
        "[0 0 100 100]",
        "0 1 1 rg 20 20 60 60 re f /GB gs 1 0 1 rg 30 30 60 60 re f",
        "1 1 0 rg 0 0 100 100 re f /Fm Do",
    ));
    assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
    assert!(
        drawn.display_list.commands().iter().any(|command| matches!(
            command,
            Command::Group {
                isolated: false,
                knockout: true,
                ..
            }
        )),
        "the group states both flags: {:?}",
        drawn.display_list.commands()
    );
    assert_eq!(
        pixel(&drawn, 50, 50),
        [255, 0, 0, 255],
        "the overlap is element 2's composite with the page, replacing element 1's"
    );
    assert_eq!(
        pixel(&drawn, 25, 75),
        [0, 255, 255, 255],
        "element 1 alone keeps its own composite"
    );
    assert_eq!(
        pixel(&drawn, 85, 15),
        [255, 0, 0, 255],
        "element 2 alone is the same stage-a) composite"
    );
    assert_eq!(
        pixel(&drawn, 5, 95),
        [255, 255, 0, 255],
        "and the page is untouched where the group does not mark"
    );
}

/// §11.6.6's group blending colour space, drawn: a group that introduces four components
/// on a page that states none (ADR 0327).
///
/// # The arithmetic
///
/// The group is isolated with `/CS /DeviceCMYK`, so §11.7.2 requires "all blending and
/// compositing computations" inside it to happen in those four components, and the result
/// to be "interpreted in the group's colour space when the group is subsequently
/// composited with its backdrop". Its elements are paper and registration black at `ca ½`
/// over it — per §11.3.4 the covered pixels hold half of each ink, and the conversion out
/// is the assumed cube's mean, **(76.0, 66.1, 63.9)** of 255. Converting each colour first
/// and compositing on the device gives 127.5, ADR 0251's 51-of-255 gap — which is exactly
/// what this page drew, and reported, before this session.
///
/// The interpreter runs the group's content twice, once per half of the four components,
/// and the readback is kept from the first run alone.
#[test]
fn a_group_that_introduces_a_press_composites_in_it() {
    let drawn = interpret(fixture(
        "/Group << /S /Transparency /I true /CS /DeviceCMYK >>",
        "[0 0 100 100]",
        "0 0 0 0 k 10 10 80 80 re f /GS gs 1 1 1 1 k 20 20 60 60 re f",
        "/Fm Do",
    ));
    assert!(drawn.is_complete(), "{:?}", drawn.unsupported);
    assert!(
        drawn.display_list.commands().iter().any(|command| matches!(
            command,
            Command::Group {
                isolated: true,
                blending: Some(_),
                ..
            }
        )),
        "the group carries the pair: {:?}",
        drawn.display_list.commands()
    );
    let painted = pixel(&drawn, 50, 50);
    for (axis, want) in [76, 66, 64].into_iter().enumerate() {
        assert!(
            (i32::from(painted[axis]) - want).abs() <= 1,
            "half of registration black over paper is the cube's mean: channel {axis} of \
             {painted:?} against {want}"
        );
    }
    assert_eq!(
        pixel(&drawn, 15, 85),
        [255, 255, 255, 255],
        "paper alone converts to white"
    );

    // The same group with nothing compositing inside it is one run on the device — an
    // opaque Normal mark carries its colour through whatever space it is carried through,
    // which is the same condition the report used to fire on — so no pair is built and no
    // report is owed.
    let opaque = interpret(fixture(
        "/Group << /S /Transparency /I true /CS /DeviceCMYK >>",
        "[0 0 100 100]",
        "0 0 0 0 k 10 10 80 80 re f 1 1 1 1 k 20 20 60 60 re f",
        "/Fm Do",
    ));
    assert!(opaque.is_complete(), "{:?}", opaque.unsupported);
    assert!(
        !opaque
            .display_list
            .commands()
            .iter()
            .any(|command| matches!(
                command,
                Command::Group {
                    blending: Some(_),
                    ..
                }
            )),
        "nothing composites, so §11.3.4 cannot change a pixel and no pair is carried"
    );
    assert_eq!(
        pixel(&opaque, 50, 50),
        [0, 0, 0, 255],
        "registration black is the cube's last corner either way"
    );
}

/// A shading pattern carried over a `Do` into a group that composites somewhere else.
///
/// ISO 32000-2 §11.6.7 makes a shading pattern's definition a group of its own:
///
/// > In both cases, the pattern definition shall be treated as if it were implicitly enclosed in
/// > a non-isolated transparency group: a non-knockout group for tiling patterns, a knockout group
/// > for shading patterns.
///
/// and §11.7.2 says where such a group's colour space comes from:
///
/// > Non-isolated groups shall inherit their colour space from the nearest ancestor isolated
/// > parent group (subject to special treatment for the page group, as described in 11.4.7, "Page
/// > group").
///
/// The nearest ancestor of a pattern *painted inside* this group is this group, so its colours
/// belong in the group's four components — whichever content stream the `scn` stood in.
///
/// **The six-hundred-and-fifty-fifth session answered this by refusing the press** and the
/// six-hundred-and-sixtieth answers it by building: a shading pattern carries its whole
/// definition now, so the mark inside the group rebuilds its colours in the group's own
/// compositing and the pair is built like any other. The two arms below are the same page with
/// the `scn` outside the form and inside it, and they agree — which is the claim, since §11.6.7
/// puts the pattern's *definition* in the parent content stream and its *painting* in the group
/// either way.
#[test]
fn a_shading_pattern_carried_into_a_press_is_rebuilt_in_the_groups_space() {
    let page_with = |page: &str, form: &str| {
        let pattern = "/Pattern << /P0 << /PatternType 2 /Shading << /ShadingType 2 \
                       /ColorSpace /DeviceRGB /Coords [0 0 100 0] /Extend [true true] \
                       /Function << /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [0 0 1] \
                       /N 1 >> >> >> >>";
        let body = format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
             /Resources << {pattern} /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
             4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
             5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
             /Group << /S /Transparency /I true /CS /DeviceCMYK >> \
             /Resources << {pattern} /ExtGState << /GS << /ca 0.5 >> >> >> /Length {} >>\n\
             stream\n{form}\nendstream\nendobj\n",
            page.len() + 1,
            form.len() + 1
        );
        interpret(assemble(&body))
    };
    let paired = |drawn: &pdf_model::Interpretation| {
        drawn.display_list.commands().iter().any(|command| {
            matches!(
                command,
                Command::Group {
                    blending: Some(_),
                    ..
                }
            )
        })
    };

    let carried = page_with(
        "/Pattern cs /P0 scn /Fm Do",
        "/GS gs 0 0 100 100 re f 1 1 1 1 k 20 20 60 60 re f",
    );
    assert!(
        paired(&carried),
        "the pattern's colours are rebuilt in the group's press, so the group composites in the \
         space it declares: {:?}",
        carried.display_list.commands()
    );
    assert!(
        !format!("{:?}", carried.unsupported).contains("blending colour space /DeviceCMYK"),
        "and there is no departure left to name: {:?}",
        carried.unsupported
    );

    let inside = page_with(
        "/Fm Do",
        "/GS gs /Pattern cs /P0 scn 0 0 100 100 re f 1 1 1 1 k 20 20 60 60 re f",
    );
    assert!(
        paired(&inside),
        "a pattern whose parent content stream is the group's composites in the group's own \
         space: {:?}",
        inside.display_list.commands()
    );
    assert_eq!(
        pixel(&carried, 50, 50),
        pixel(&inside, 50, 50),
        "and where the `scn` stood decides the definition, not the painting: the two pages draw \
         the same pixel"
    );
}

/// A page whose outer form group holds a fill and an inner form group.
///
/// `outer` and `inner` are the two `/Group` dictionaries, written whole so that a test can
/// change one entry of one of them and nothing else. The inner group's one element blends,
/// which is the only thing that can tell one initial backdrop from another (§11.4.4 NOTE 2).
fn nested_group_fixture(outer: &str, inner: &str) -> Vec<u8> {
    const OUTER: &str = "0 0 1 rg 0 0 100 100 re f /In Do";
    const INNER: &str = "/GB gs 1 0 0 rg 20 20 60 60 re f";
    const PAGE: &str = "/Fm Do";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{PAGE}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] {outer} \
         /Resources << /XObject << /In 6 0 R >> >> /Length {} >>\n\
         stream\n{OUTER}\nendstream\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] {inner} \
         /Resources << /ExtGState << /GB << /BM /Multiply >> >> >> /Length {} >>\n\
         stream\n{INNER}\nendstream\nendobj\n",
        PAGE.len() + 1,
        OUTER.len() + 1,
        INNER.len() + 1
    );
    assemble(&body)
}

/// A group that introduces a second space *inside* the pair keeps the pair off and the
/// reports on.
///
/// §11.6.6's departure reports fire where the device's components are what is composited
/// on; during a pair's subtractive runs they cannot, so a group met there that changes the
/// space in force — with something compositing in it — is recorded, the pair is discarded,
/// and the content re-runs on the device where both groups report ordinarily. Without the
/// record the inner group's elements would composite in the outer group's ink with nothing
/// said, which is trap 5's silence; `bug1721218_reduced.pdf` is the corpus shape that
/// stays *drawn* — its inner one-component groups hold nothing that composites, so §11.3.4
/// cannot tell the spaces apart there and the pair stands.
///
/// The second case is a change whose target is the device's own components: an isolated
/// `/DeviceRGB` group inside the pair is just as much a second space, and it is the one
/// whose report has no name to print — on the device rerun its space *is* the device's, so
/// only the outer group reports.
///
/// **The inner `/DeviceGray` group reported its own name until ADR 0790, and now it is
/// drawn**: on the device rerun the parent composites on the device, which is where
/// `Compositing::Grey` applies, so the group's one red element is painted as §10.4.2.2's
/// grey of red — `0.3 × 1.0`, 76.5 of 255 — and the outer group is the only report left.
/// The `/DeviceRGB` arm keeps its red, which is what says the grey is the group's and not
/// the rerun's.
#[test]
fn a_second_space_inside_the_pair_falls_back_to_the_device_and_reports() {
    let outer = "/Group << /S /Transparency /I true /CS /DeviceCMYK >>";
    for (inner, red_becomes) in [
        (
            "/Group << /S /Transparency /I true /CS /DeviceGray >>",
            [76, 76, 76],
        ),
        (
            "/Group << /S /Transparency /I true /CS /DeviceRGB >>",
            [255, 0, 0],
        ),
    ] {
        let drawn = interpret(nested_group_fixture(outer, inner));
        let reported = format!("{:?}", drawn.unsupported);
        assert!(
            reported.contains("blending colour space /DeviceCMYK"),
            "the outer group's departure is named on the device rerun: {reported}"
        );
        assert!(
            !reported.contains("blending colour space /DeviceGray"),
            "and the inner group's is not, because it is drawn in its grey: {reported}"
        );
        let painted = pixel(&drawn, 50, 50);
        for (axis, want) in red_becomes.into_iter().enumerate() {
            assert!(
                (i32::from(painted[axis]) - want).abs() <= 1,
                "the inner group's red element under {inner}: channel {axis} of {painted:?} \
                 against {want}"
            );
        }
        assert!(
            !drawn.display_list.commands().iter().any(|command| matches!(
                command,
                Command::Group {
                    blending: Some(_),
                    ..
                }
            )),
            "no pair is carried where a second space composites inside it"
        );
    }
}

/// §11.4.6's NOTE 6: a non-isolated group nested in a knockout group takes the *outer* group's
/// initial backdrop.
///
/// > When a non-isolated group is nested within a knockout group, the initial backdrop of the
/// > inner group is the same as that of the outer group; it is not the immediate backdrop of
/// > the inner group. This behaviour, although perhaps unexpected, is a consequence of the
/// > group compositing formulas when b = 0.
///
/// So where the outer knockout group is isolated its initial backdrop is §11.4.5's transparent
/// one, and the inner group has that — which makes the inner group an isolated group by
/// §11.4.5's own definition, whatever its `/I` says. This renderer composites a group's
/// elements onto transparency, so that page is drawn exactly and has nothing to report; it
/// reported one for `knockout_inner_backdrop.pdf` until ADR 0307.
///
/// The three fixtures differ in one entry each and no two of them can pass by accident:
///
/// - `/K true /I true` outside: NOTE 6's transparent backdrop, so the Multiply has nothing to
///   blend with and the square is its own red.
/// - `/K false /I true` outside: NOTE 6 does not apply, the inner group's backdrop is the blue
///   fill beside it, and §11.3.5.2's Multiply of (0,0,1) with (1,0,0) is black. This tree draws
///   that — ADR 0237 — so the difference between this fixture and the one above is the whole of
///   what NOTE 6 says, at the pixel.
/// - `/K true /I false` outside: the outer group's own initial backdrop is the page, so the
///   inner group's is too, and this renderer substitutes transparency and reports it.
#[test]
fn a_group_inside_an_isolated_knockout_group_takes_the_transparency_note_6_gives_it() {
    let inner = "/Group << /S /Transparency /I false >>";

    let nested = interpret(nested_group_fixture(
        "/Group << /S /Transparency /I true /K true >>",
        inner,
    ));
    assert!(nested.is_complete(), "{:?}", nested.unsupported);
    assert_eq!(
        pixel(&nested, 50, 50),
        [255, 0, 0, 255],
        "NOTE 6 gives the inner group the outer group's transparent initial backdrop"
    );

    let ordinary = interpret(nested_group_fixture(
        "/Group << /S /Transparency /I true /K false >>",
        inner,
    ));
    assert!(ordinary.is_complete(), "{:?}", ordinary.unsupported);
    assert_eq!(
        pixel(&ordinary, 50, 50),
        [0, 0, 0, 255],
        "without the knockout attribute the inner group blends with its immediate backdrop"
    );

    let opaque_backdrop = interpret(nested_group_fixture(
        "/Group << /S /Transparency /I false /K true >>",
        inner,
    ));
    let reported = format!("{:?}", opaque_backdrop.unsupported);
    assert!(
        reported.contains("non-isolated, and an element blends with the backdrop it excludes"),
        "a knockout group whose own initial backdrop is the page passes that page inward, \
         and this renderer substitutes transparency for it: {reported}"
    );
}

/// A one-page fixture for §11.3.4's one-component blending space.
///
/// `page_group` is the page's whole `/Group` entry or nothing; `resources` is spliced into the
/// page's resource dictionary beside the two graphics states and the form; `form_group` is
/// the form `XObject`'s whole `/Group` entry or nothing. The `/GS` state is half alpha and the
/// `/GB` state is `Multiply`, which is what makes a mark composite and what makes it blend.
fn one_component_fixture(
    page_group: &str,
    resources: &str,
    page: &str,
    form_group: &str,
    form: &str,
) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] {page_group} \
         /Resources << /ExtGState << /GS << /ca 0.5 /CA 0.5 >> /GB << /BM /Multiply >> >> \
         /XObject << /Fm 5 0 R >> {resources} >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] {form_group} \
         /Resources << /ExtGState << /GS << /ca 0.5 /CA 0.5 >> /GB << /BM /Multiply >> >> >> \
         /Length {} >>\nstream\n{form}\nendstream\nendobj\n",
        page.len() + 1,
        form.len() + 1
    );
    assemble(&body)
}

/// One channel of a pixel against the clause's arithmetic, to the byte's own rounding.
fn assert_grey(what: &str, painted: [u8; 4], want: f32) {
    let want = (want * 255.0).round();
    for channel in &painted[..3] {
        assert!(
            (f32::from(*channel) - want).abs() <= 1.0,
            "{what}: {painted:?} against a grey of {want} in every channel"
        );
    }
}

/// An opaque red, a half-alpha blue over it, and an opaque cyan beside them.
///
/// The three marks §10.4.2.2 and §10.4.2.3 give different greys to, so that a page drawn in
/// grey by *any* one conversion is told apart from one drawn by the clause's two.
const THREE_MARKS: &str = "1 0 0 rg 0 0 100 100 re f\n\
                           q /GS gs 0 0 1 rg 0 0 50 100 re f Q\n\
                           1 0 0 0 k 50 0 50 50 re f";

/// ISO 32000-2 §11.4.7 and §11.3.4: a page group of `/DeviceGray` composites every mark in grey.
///
/// §11.4.7 makes the page group's `/CS` the space "[a]ll page-level compositing shall be done
/// in", and §11.6.6 has every painting operator convert into it before compositing — so the
/// conversion is §10.4.2.2's for an RGB mark, `0.3 × red + 0.59 × green + 0.11 × blue`, and
/// §10.4.2.3's for a CMYK one, `1.0 − min(1.0, 0.3 × cyan + 0.59 × magenta + 0.11 × yellow +
/// black)`. §11.3.4 then applies §11.3.3's formula per component, which for `Normal` at
/// half alpha is §11.3.6's weighted average of the two greys. The three expected values are
/// those three sentences, and the control is the same content with no page group at all,
/// where the red is red.
#[test]
fn a_page_group_of_one_component_paints_every_mark_in_its_grey() {
    let grey = interpret(one_component_fixture(
        "/Group << /S /Transparency /CS /DeviceGray >>",
        "",
        THREE_MARKS,
        "",
        "",
    ));
    assert!(
        grey.is_complete(),
        "a page composited in DeviceGray is drawn, not reported: {:?}",
        grey.unsupported
    );
    // Device y is the page's flipped: page (75, 75) is device (75, 25).
    assert_grey(
        "an opaque red is §10.4.2.2's grey of red",
        pixel(&grey, 75, 25),
        0.3,
    );
    assert_grey(
        "half of blue over red is §11.3.6's average of their two greys",
        pixel(&grey, 25, 50),
        0.5 * 0.11 + 0.5 * 0.3,
    );
    assert_grey(
        "an opaque cyan is §10.4.2.3's grey of cyan",
        pixel(&grey, 75, 75),
        1.0 - 0.3,
    );

    let device = interpret(one_component_fixture("", "", THREE_MARKS, "", ""));
    assert_eq!(
        pixel(&device, 75, 25)[..3],
        [255, 0, 0],
        "without a page group the red is red, which is what says the grey above is the group's"
    );
}

/// ISO 32000-2 §11.6.6: an isolated group whose `/CS` is `/DeviceGray` composites its elements in grey.
///
/// > For isolated groups, if a group colour space ( CS ) is specified in the group attributes
/// > dictionary, all painting operators shall convert source colours in a colour space (that
/// > are not equivalent to the group colour space) to the group colour space before
/// > compositing objects into the group. The resulting colour at each point shall be
/// > interpreted in the group colour space.
///
/// The same three marks as the page test, inside the group on a page that states no space,
/// so what changes is only §11.6.6's scope. The group's result is grey in every channel, and
/// composites onto the device page as any group does — §10.4.2.2's conversion out is the
/// identity on it. The control is the same group non-isolated, whose `/CS` the next sentence
/// of the clause gives no effect: "[f]or non-isolated groups, or if no group colour space is
/// specified, the group colour space shall be inherited from the parent group or page".
#[test]
fn an_isolated_group_of_one_component_composites_its_elements_in_grey() {
    let grey = interpret(one_component_fixture(
        "",
        "",
        "/Fm Do",
        "/Group << /S /Transparency /I true /CS /DeviceGray >>",
        THREE_MARKS,
    ));
    assert!(
        grey.is_complete(),
        "an isolated DeviceGray group is drawn, not reported: {:?}",
        grey.unsupported
    );
    assert_grey("an opaque red inside the group", pixel(&grey, 75, 25), 0.3);
    assert_grey(
        "half of blue over red inside the group",
        pixel(&grey, 25, 50),
        0.5 * 0.11 + 0.5 * 0.3,
    );
    assert_grey("an opaque cyan inside the group", pixel(&grey, 75, 75), 0.7);

    let inherited = interpret(one_component_fixture(
        "",
        "",
        "/Fm Do",
        "/Group << /S /Transparency /I false /CS /DeviceGray >>",
        THREE_MARKS,
    ));
    assert!(inherited.is_complete(), "{:?}", inherited.unsupported);
    assert_eq!(
        pixel(&inherited, 75, 25)[..3],
        [255, 0, 0],
        "a non-isolated group's /CS is not the space anything composites in"
    );
}

/// A grey page whose inner group blends in another space falls back to the device and reports.
///
/// §11.6.6's conversion at the inner group's `Do` — its RGB result into the page's grey — is
/// one per pixel between two spaces, which no display list here carries; the standing answer
/// is the one the four-component pair gives the same shape (see
/// `a_second_space_inside_the_pair_falls_back_to_the_device_and_reports`): the page is drawn
/// on the device's components and the departure is named where it was introduced. The
/// control is the same inner group with nothing compositing in it, where converting each
/// mark first and compositing after is the same picture (§11.3.6, and §10.4.2.2 is affine),
/// so the grey run stands.
#[test]
fn a_grey_page_whose_inner_group_blends_in_colour_falls_back_and_reports() {
    let blending = interpret(one_component_fixture(
        "/Group << /S /Transparency /CS /DeviceGray >>",
        "",
        "1 0 0 rg 0 0 100 100 re f\n/Fm Do",
        "/Group << /S /Transparency /I true /CS /DeviceRGB >>",
        "/GB gs 0 0 1 rg 20 20 60 60 re f",
    ));
    let reported = format!("{:?}", blending.unsupported);
    assert!(
        reported.contains(
            "the page group's blending colour space /DeviceGray (§11.4.7): a group inside it \
             composites in a different space (§11.6.6)"
        ),
        "{reported}"
    );
    assert_eq!(
        pixel(&blending, 10, 10)[..3],
        [255, 0, 0],
        "and the page is drawn on the device's components, red for red"
    );

    let opaque = interpret(one_component_fixture(
        "/Group << /S /Transparency /CS /DeviceGray >>",
        "",
        "1 0 0 rg 0 0 100 100 re f\n/Fm Do",
        "/Group << /S /Transparency /I true /CS /DeviceRGB >>",
        "0 0 1 rg 20 20 60 60 re f",
    ));
    assert!(opaque.is_complete(), "{:?}", opaque.unsupported);
    assert_grey("the page's own red", pixel(&opaque, 10, 10), 0.3);
    assert_grey(
        "the inner group's opaque blue, converted once at its Do",
        pixel(&opaque, 50, 50),
        0.11,
    );
}

/// A `CalGray` page group composites its component and leaves by §8.6.5.2's curve.
///
/// ISO 32000-2 §11.3.4 lists `CalGray` among the spaces that "shall be supported as blending
/// colour spaces", and §11.4.7 puts the conversion out at the end: "the entire result shall
/// then, if the colour spaces are not equivalent, be converted to the native colour space of
/// the output device before being composited with the context-dependent backdrop". The
/// component is §8.6.5.2's `A`, which "shall be first decoded by the gamma function, and the
/// result shall be multiplied by the components of the white point to obtain the L, M , and N
/// components" — and with `/Gamma 1` those are `A` times the white, so the composited
/// component is a CIE `Y` relative to white, and the device shows it through sRGB's own
/// transfer function (IEC 61966-2-1, the route `xyz_d50_to_srgb` takes).
///
/// So the three expected values are: an opaque white is white; half of black over white is
/// §11.3.6's average of the two *components*, `0.5`, which leaves through the curve as
/// `1.055 × 0.5^(1/2.4) − 0.055 = 0.735`, 188 of 255 — where `DeviceGray` composites the same
/// two greys to 128; and an opaque red is §10.4.2.2's grey of red, 0.3, converted in and back
/// out, which is the round trip [`GreyRoute`] exists for. The control is the same page under
/// `/DeviceGray`.
#[test]
fn a_calibrated_page_group_composites_its_component_and_leaves_by_the_curve() {
    let cal_gray = "[/CalGray << /WhitePoint [0.9505 1 1.089] /Gamma 1 >>]";
    let calibrated = interpret(one_component_fixture(
        &format!("/Group << /S /Transparency /CS {cal_gray} >>"),
        "",
        WHITE_HALF_BLACK_RED,
        "",
        "",
    ));
    assert!(
        calibrated.is_complete(),
        "a CalGray page group is drawn, not reported: {:?}",
        calibrated.unsupported
    );
    // Device y is the page's flipped: page (75, 75) is device (75, 25).
    assert_grey("an opaque white", pixel(&calibrated, 75, 25), 1.0);
    assert_grey(
        "half of black over white composites the components to ½ and leaves by sRGB's curve",
        pixel(&calibrated, 25, 50),
        srgb_encode(0.5),
    );
    assert_grey(
        "an opaque red is its §10.4.2.2 grey, in and back out",
        pixel(&calibrated, 75, 75),
        0.3,
    );

    let device_grey = interpret(one_component_fixture(
        "/Group << /S /Transparency /CS /DeviceGray >>",
        "",
        WHITE_HALF_BLACK_RED,
        "",
        "",
    ));
    assert_grey(
        "the same two greys composited as device grey are their average",
        pixel(&device_grey, 25, 50),
        0.5,
    );
}

/// White, half of black over the left half, and an opaque red in the bottom right.
///
/// The marks that tell compositing in a component from compositing in device grey: the
/// second is where the curve moves the result and the third is where it does not.
const WHITE_HALF_BLACK_RED: &str = "1 g 0 0 100 100 re f\n\
                                    q /GS gs 0 g 0 0 50 100 re f Q\n\
                                    1 0 0 rg 50 0 50 50 re f";

/// IEC 61966-2-1's transfer function, which is what a linear `Y` shows as on this device.
fn srgb_encode(linear: f32) -> f32 {
    if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

/// An isolated `CalGray` group on a device page composites in its component and comes out
/// through the curve at its `Do` (ISO 32000-2 §11.6.6, §11.7.2).
///
/// §11.7.2: "all blending and compositing computations shall be done in that space", and
/// "[t]he resulting colours shall then be interpreted in the group's colour space when the
/// group is subsequently composited with its backdrop". The same marks and the same three
/// values as the page test, one scope down; the control is the same group non-isolated, whose
/// `/CS` §11.6.6 gives no effect, so its half of black over white is the device's 128.
#[test]
fn an_isolated_calibrated_group_composites_in_its_component_and_leaves_by_the_curve() {
    let cal_gray = "[/CalGray << /WhitePoint [0.9505 1 1.089] /Gamma 1 >>]";
    let calibrated = interpret(one_component_fixture(
        "",
        "",
        "/Fm Do",
        &format!("/Group << /S /Transparency /I true /CS {cal_gray} >>"),
        WHITE_HALF_BLACK_RED,
    ));
    assert!(
        calibrated.is_complete(),
        "an isolated CalGray group is drawn, not reported: {:?}",
        calibrated.unsupported
    );
    assert_grey(
        "an opaque white inside the group",
        pixel(&calibrated, 75, 25),
        1.0,
    );
    assert_grey(
        "half of black over white inside the group leaves by the curve",
        pixel(&calibrated, 25, 50),
        srgb_encode(0.5),
    );
    assert_grey(
        "an opaque red inside the group is its grey, in and back out",
        pixel(&calibrated, 75, 75),
        0.3,
    );

    let inherited = interpret(one_component_fixture(
        "",
        "",
        "/Fm Do",
        &format!("/Group << /S /Transparency /I false /CS {cal_gray} >>"),
        WHITE_HALF_BLACK_RED,
    ));
    assert!(inherited.is_complete(), "{:?}", inherited.unsupported);
    assert_grey(
        "a non-isolated group's /CS is not the space anything composites in",
        pixel(&inherited, 25, 50),
        0.5,
    );
    assert_eq!(
        pixel(&inherited, 75, 75)[..3],
        [255, 0, 0],
        "and its red stays red on the device"
    );
}

/// Table 145's remapping takes a `/DeviceGray` page group into its `/DefaultGray`.
///
/// ISO 32000-2 §11.6.6, Table 145:
///
/// > Device colour spaces shall be subject to remapping according to the DefaultGray ,
/// > DefaultRGB , and DefaultCMYK entries in the ColorSpace subdictionary of the current
/// > resource dictionary (see 8.6.5.6, "Default colour spaces").
///
/// So a `/DeviceGray` beside a `/DefaultGray` of `CalGray` is that `CalGray`, drawn by its
/// route and not by device grey's: the half of black over white is the curve's 188 and not
/// the channel's 128.
#[test]
fn a_default_gray_takes_the_page_group_into_the_calibrated_space_it_names() {
    let cal_gray = "[/CalGray << /WhitePoint [0.9505 1 1.089] /Gamma 1 >>]";
    let remapped = interpret(one_component_fixture(
        "/Group << /S /Transparency /CS /DeviceGray >>",
        &format!("/ColorSpace << /DefaultGray {cal_gray} >>"),
        WHITE_HALF_BLACK_RED,
        "",
        "",
    ));
    assert!(remapped.is_complete(), "{:?}", remapped.unsupported);
    assert_grey(
        "the remapped page composites in the default's component",
        pixel(&remapped, 25, 50),
        srgb_encode(0.5),
    );
}

/// A page whose group is a one-component `ICCBased` space composites in the profile's
/// component and leaves by its tone curve (ISO 32000-2 §11.3.4, §8.6.5.5).
///
/// §11.3.4 lists a bi-directional `ICCBased` 'GRAY' profile beside `CalGray`, and §8.6.5.5
/// says which half of the profile a source colour uses — only the to-CIE (`AToB`) information,
/// the from-CIE (`BToA`) being ignored — which is the conversion out of the group's result,
/// since "the group colour space shall be used as both the destination for objects being
/// painted within the group and the source for the group's results". The profile below is a
/// 'GRAY' display profile whose `kTRC` is a gamma of 1.0, so its component is a `Y` on the
/// connection space's white, and the expected values are the `CalGray` test's for the same
/// reason.
#[test]
fn a_one_component_profile_page_group_composites_in_its_component() {
    let profiled = interpret(icc_gray_fixture(
        "/Group << /S /Transparency /CS [/ICCBased 5 0 R] >>",
        WHITE_HALF_BLACK_RED,
    ));
    assert!(
        profiled.is_complete(),
        "an ICCBased 'GRAY' page group is drawn, not reported: {:?}",
        profiled.unsupported
    );
    assert_grey("an opaque white", pixel(&profiled, 75, 25), 1.0);
    assert_grey(
        "half of black over white leaves by the profile's curve",
        pixel(&profiled, 25, 50),
        srgb_encode(0.5),
    );
    assert_grey(
        "an opaque red is its grey, in and back out",
        pixel(&profiled, 75, 75),
        0.3,
    );
}

/// A one-page fixture whose page group is `/CS [/ICCBased 5 0 R]`, object 5 being
/// [`icc_gray_profile`] as an `ASCIIHexDecode` stream with `/N 1`.
fn icc_gray_fixture(page_group: &str, page: &str) -> Vec<u8> {
    let mut hex = String::new();
    for byte in icc_gray_profile() {
        let _ = write!(hex, "{byte:02X}");
    }
    hex.push('>');
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] {page_group} \
         /Resources << /ExtGState << /GS << /ca 0.5 /CA 0.5 >> >> >> /Contents 4 0 R >>\n\
         endobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n<< /N 1 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}\nendstream\n\
         endobj\n",
        page.len() + 1,
        hex.len() + 1
    );
    assemble(&body)
}

/// A v2 'GRAY' display profile whose one tag is a `kTRC` gamma of 1.0.
///
/// Positional like [`icc_cmyk_profile_of`]: a 128-byte header naming `GRAY` data and an
/// `XYZ ` connection space, a tag count of one, the tag entry, then a `curv` of one entry —
/// a `u8Fixed8` gamma, `0x0100` being 1.0.
fn icc_gray_profile() -> Vec<u8> {
    let mut header = vec![0u8; 128];
    header[8] = 2; // major version
    header[12..16].copy_from_slice(b"mntr");
    header[16..20].copy_from_slice(b"GRAY");
    header[20..24].copy_from_slice(b"XYZ ");
    header[36..40].copy_from_slice(b"acsp");

    let mut tag = Vec::new();
    tag.extend_from_slice(b"curv");
    tag.extend_from_slice(&[0; 4]);
    tag.extend_from_slice(&1u32.to_be_bytes()); // one entry: a gamma
    tag.extend_from_slice(&0x0100u16.to_be_bytes()); // 1.0 in u8Fixed8
    tag.extend_from_slice(&[0; 2]); // padding to four bytes

    let mut out = header;
    out.extend_from_slice(&1u32.to_be_bytes()); // one tag
    out.extend_from_slice(b"kTRC");
    out.extend_from_slice(&144u32.to_be_bytes()); // 128 + 4 + 12
    out.extend_from_slice(&u32::try_from(tag.len()).expect("small").to_be_bytes());
    out.extend_from_slice(&tag);
    out
}

/// A one-component space §11.3.4 does not list is reported, whether or not anything composites.
///
/// §11.3.4's list is `DeviceGray`, `CalGray` and an `ICCBased` 'GRAY' profile, and it says of
/// the other one-component spaces that spot colours "shall not be converted to a blending
/// colour space" — so a `Separation` as a page group's `/CS` is a space this tree has no
/// conversion into. §11.6.6 would have every painting operator convert into it, and for one
/// component that conversion keeps a grey and nothing else, so the departure is every mark on
/// the page and not only one that composites (ADR 0790). The page stays on the device — red
/// for red — and says so.
#[test]
fn a_one_component_space_the_clause_does_not_list_is_reported_though_nothing_composites() {
    let spot = "[/Separation /Spot /DeviceGray << /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] \
                /N 1 >>]";
    let red = "1 0 0 rg 0 0 100 100 re f";
    let separation = interpret(one_component_fixture(
        &format!("/Group << /S /Transparency /CS {spot} >>"),
        "",
        red,
        "",
        "",
    ));
    let reported = format!("{:?}", separation.unsupported);
    assert!(
        reported.contains(
            "the page group's blending colour space an array-formed space (§11.4.7): its one \
             component is neither device grey's channel nor one this tree can composite \
             through a curve with an inverse"
        ),
        "a Separation page group is named though nothing on the page composites: {reported}"
    );
    assert_eq!(pixel(&separation, 50, 50)[..3], [255, 0, 0]);

    let group = interpret(one_component_fixture(
        "",
        "",
        "/Fm Do",
        &format!("/Group << /S /Transparency /I true /CS {spot} >>"),
        red,
    ));
    let reported = format!("{:?}", group.unsupported);
    assert!(
        reported.contains("blending colour space an array-formed space"),
        "and an isolated Separation group is named on the same condition: {reported}"
    );
}

/// [`icc_cmyk_profile`] with a `B2A1` table beside its `A2B1`: a bi-directional profile.
///
/// The "from CIE" table's corners state one minus each connection-space input on the three
/// chromatic inks and no black, over the table's own XYZ encoding (`1.0` at `0x8000`) — an
/// affine rule, so the profile's trilinear interpolation of the corners *is* the rule and a
/// test's expected inks are `1 − v × 32768 ÷ 65535` per axis. Not a real press's inverse,
/// and not meant to be: what it has to be is *stated by the file* and different from the
/// right inverse a search over the `A2B` would find.
fn two_way_cmyk_profile() -> Vec<u8> {
    let one_way = icc_cmyk_profile();
    // The `A2B1` tag is everything after the header, the count and the one entry.
    let a2b = one_way[144..].to_vec();

    let mut b2a = Vec::new();
    b2a.extend_from_slice(b"mft2");
    b2a.extend_from_slice(&[0; 4]);
    b2a.push(3); // three input channels: the connection space
    b2a.push(4); // four output channels: the inks
    b2a.push(2); // two grid points per axis
    b2a.push(0);
    for value in [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0] {
        b2a.extend_from_slice(&((value * 65536.0) as i32).to_be_bytes());
    }
    b2a.extend_from_slice(&2u16.to_be_bytes());
    b2a.extend_from_slice(&2u16.to_be_bytes());
    for _ in 0..3 {
        for value in [0u16, 0xFFFF] {
            b2a.extend_from_slice(&value.to_be_bytes());
        }
    }
    for corner in 0..8usize {
        for axis in 0..3usize {
            let high = (corner >> (2 - axis)) & 1 == 1;
            b2a.extend_from_slice(&(if high { 0u16 } else { 0xFFFF }).to_be_bytes());
        }
        b2a.extend_from_slice(&0u16.to_be_bytes());
    }
    for _ in 0..4 {
        for value in [0u16, 0xFFFF] {
            b2a.extend_from_slice(&value.to_be_bytes());
        }
    }

    let mut out = one_way[..128].to_vec();
    out.extend_from_slice(&2u32.to_be_bytes()); // two tags
    let first = 128 + 4 + 2 * 12;
    out.extend_from_slice(b"A2B1");
    out.extend_from_slice(&u32::try_from(first).expect("small").to_be_bytes());
    out.extend_from_slice(&u32::try_from(a2b.len()).expect("small").to_be_bytes());
    out.extend_from_slice(b"B2A1");
    out.extend_from_slice(
        &u32::try_from(first + a2b.len())
            .expect("small")
            .to_be_bytes(),
    );
    out.extend_from_slice(&u32::try_from(b2a.len()).expect("small").to_be_bytes());
    out.extend_from_slice(&a2b);
    out.extend_from_slice(&b2a);
    out
}

/// A one-page fixture whose page group names [`two_way_cmyk_profile`] as its blending space.
fn two_way_press_fixture(content: &str) -> Vec<u8> {
    let mut hex = String::new();
    for byte in two_way_cmyk_profile() {
        let _ = write!(hex, "{byte:02X}");
    }
    hex.push('>');
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Group << /S /Transparency /CS [/ICCBased 5 0 R] >> \
         /Resources << /ExtGState << /GS << /ca 0.5 /CA 0.5 >> >> >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /N 4 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}\nendstream\n\
         endobj\n",
        content.len() + 1,
        hex.len() + 1
    );
    assemble(&body)
}

/// A colour painted into a page whose press is a bi-directional profile goes in through the
/// profile's own "from CIE" table (ISO 32000-2 §8.6.5.5, §10.3.1, §11.6.6; ADR 0796).
///
/// §8.6.5.5, of an `ICCBased` space used as a blending colour space: "it shall have both 'to
/// CIE' ( AToB ) and 'from CIE' ( BToA ) information. This is because the group colour space
/// shall be used as both the destination for objects being painted within the group and the
/// source for the group's results." So a device grey painted into this page becomes the inks
/// the `B2A` table states for its XYZ — the table's own rule on sRGB's decoding of the grey
/// (IEC 61966-2-1: 0.5 is 0.2140 in linear light, and a grey is that fraction of D50) with
/// the black point compensation the conversion out applies undone first — and the pixel is
/// those inks taken out through the `A2B`. The control is the same page over the one-way
/// profile, where the conversion in is a right inverse of the conversion out and the grey
/// comes back as itself (ADR 0263).
#[test]
fn a_colour_painted_into_a_bidirectional_press_goes_in_through_its_from_cie_table() {
    let content = "0.5 0.5 0.5 rg 0 0 100 100 re f";
    let drawn = interpret(two_way_press_fixture(content));
    assert!(
        drawn.is_complete(),
        "the page is drawn in the press it names: {:?}",
        drawn.unsupported
    );
    let painted = pixel(&drawn, 50, 50);

    let profile = pdf_model::icc::Profile::parse(&two_way_cmyk_profile())
        .expect("the two-way fixture profile parses");
    assert!(profile.is_bidirectional());
    // sRGB's decoding of 0.5, as a fraction of D50; the compensation undone per axis, from the
    // press's own darkest colour at full ink.
    let linear = ((0.5f32 + 0.055) / 1.055).powf(2.4);
    let white = press_xyz_of(0, [0.0, 0.0, 0.0, 0.0]);
    let black = press_xyz_of(0, [1.0, 1.0, 1.0, 1.0]);
    let mut inks = [0.0f32; 4];
    for axis in 0..3 {
        let stretched = white[axis] * linear;
        let plain = stretched / white[axis] * (white[axis] - black[axis]) + black[axis];
        inks[axis] = 1.0 - plain * 32768.0 / 65535.0;
    }
    let wanted = profile.to_rgb(&inks);
    let level = |value: f32| (value.clamp(0.0, 1.0) * 255.0 + 0.5) as i32;
    for (axis, want) in [level(wanted.r), level(wanted.g), level(wanted.b)]
        .into_iter()
        .enumerate()
    {
        let got = i32::from(painted[axis]);
        assert!(
            (got - want).abs() <= 2,
            "channel {axis} is {got} where the table's inks {inks:?} come out at {want} \
             ({painted:?})"
        );
    }
    assert!(
        (i32::from(painted[0]) - 128).abs() > 20,
        "and that is not the grey a right inverse would have brought back: {painted:?}"
    );

    let control = interpret(press_fixture(
        "/Group << /S /Transparency /CS [/ICCBased 5 0 R] >>",
        "",
        "",
        content,
    ));
    let painted = pixel(&control, 50, 50);
    for channel in &painted[..3] {
        assert!(
            (i32::from(*channel) - 128).abs() <= 1,
            "over the one-way profile the grey comes back as itself: {painted:?}"
        );
    }
}

/// A one-page fixture with a white page under a black fill masked by a `/Luminosity` group
/// whose `/CS` is a `CalGray` of gamma 2, so that the mask value is the clause's arithmetic.
///
/// `mask` is the group's content, with the space itself available as `/CG`; `backdrop` is
/// whatever Table 142 entry the soft-mask dictionary states beside `/S` and `/G`.
fn calibrated_mask_fixture(mask: &str, backdrop: &str) -> Vec<u8> {
    let cal_gray = "[/CalGray << /WhitePoint [0.9505 1 1.089] /Gamma 2 >>]";
    let page = "1 g 0 0 100 100 re f /GM gs 0 g 0 0 100 100 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /ExtGState << /GM << /SMask << /S /Luminosity /G 5 0 R {backdrop} >> >> >> >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /CS {cal_gray} >> \
         /Resources << /ColorSpace << /CG {cal_gray} >> >> /Length {} >>\n\
         stream\n{mask}\nendstream\nendobj\n",
        page.len() + 1,
        mask.len() + 1
    );
    assemble(&body)
}

/// A `/Luminosity` mask group whose `/CS` is `CalGray` is composited in its component and
/// its mask value is §11.5.3's `Y` of the result (ISO 32000-2 §11.5.3, §11.6.5.1, §8.6.5.2).
///
/// §11.5.3: "For CIE-based spaces, convert to the CIE 1931 XYZ space and use the Y component
/// as the luminosity." §8.6.5.2 gives a `CalGray` component's XYZ as the gamma-decoded
/// component times the white point, whose `Y` is 1.0 — so a component of 0.5 under `/Gamma 2`
/// is a luminosity of 0.25, and a black fill through that mask over white leaves 0.75. Table
/// 142's `/BC` is "n numbers, where n is the number of components in the colour space
/// specified by the CS entry", so `/BC [0.5]` is the same component and the same 0.75 where
/// the group paints nothing. The control is the same group in `DeviceGray`, whose 0.5 *is* the
/// mask value by the clause's device branch.
#[test]
fn a_calibrated_mask_group_takes_the_luminance_of_its_composited_component() {
    let painted = interpret(calibrated_mask_fixture(
        "/CG cs 0.5 sc 0 0 100 100 re f",
        "",
    ));
    assert!(
        painted.is_complete(),
        "a CalGray mask group is drawn, not reported: {:?}",
        painted.unsupported
    );
    assert_grey(
        "a component of 0.5 under gamma 2 masks a quarter of the black",
        pixel(&painted, 50, 50),
        0.75,
    );

    let backdrop = interpret(calibrated_mask_fixture("", "/BC [0.5]"));
    assert_grey(
        "and /BC states that component where the group paints nothing",
        pixel(&backdrop, 50, 50),
        0.75,
    );

    let device = interpret(calibrated_mask_fixture("0.5 g 0 0 100 100 re f", ""));
    // A device grey painted *into* the calibrated group is converted in first (§11.6.6): the
    // component whose device colour has that grey, which under gamma 2 is the component whose
    // square is sRGB's decoding of 0.5 — so the luminosity is that decoding, 0.2140, and the
    // page keeps 0.786 of its white.
    let linear = ((0.5f32 + 0.055) / 1.055).powf(2.4);
    assert_grey(
        "a device grey goes in through §11.6.6's conversion and out as its linear light",
        pixel(&device, 50, 50),
        1.0 - linear,
    );
}

/// §8.6.5.3's space with the sRGB primaries, a D65 white point and a gamma of 1.0 on every
/// component: its components are sRGB's linear light, so what a component shows as on this
/// device is [`srgb_encode`] of it, and the expected values below follow from that alone.
const LINEAR_CAL_RGB: &str = "[/CalRGB << /WhitePoint [0.9505 1 1.089] /Gamma [1 1 1] \
                              /Matrix [0.4124 0.2126 0.0193 0.3576 0.7152 0.1192 \
                              0.1805 0.0722 0.9505] >>]";

/// White, half of black over the left half, a `DeviceRGB` grey of ½ in the bottom right and a
/// `DeviceGray` of ½ in the top right.
///
/// The two greys are the same colour on the device and are told apart by §11.7.2 inside a
/// three-component CIE-based group: the `rg` one is reinterpreted as the group's components
/// and the `g` one is converted in.
const WHITE_HALF_BLACK_TWO_GREYS: &str = "1 g 0 0 100 100 re f\n\
                                          q /GS gs 0 g 0 0 50 100 re f Q\n\
                                          0.5 0.5 0.5 rg 50 0 50 50 re f\n\
                                          0.5 g 50 50 50 50 re f";

/// ISO 32000-2 §11.4.7, §11.3.4 and §11.7.2: a `CalRGB` page group composites the space's
/// three components and leaves by §8.6.5.3's conversion.
///
/// §11.3.4 lists `CalRGB` among the spaces that "shall be supported as blending colour
/// spaces" and applies its formula per component, so half of black over white composites
/// each component to ½ — which under a gamma of 1.0 is linear light, shown as `srgb_encode(½)`,
/// 188 of 255, where the device's average is 128. §11.7.2 then decides the two greys: "if the
/// colour space of any graphics object is a device colour space, and the current group or an
/// ancestor of the current group is defined with a CIE-based colour space with the same
/// number of colourants, then, for compositing purposes only, the colour space of the
/// graphics object shall be the CIE-based space of the nearest such ancestor" — so
/// `0.5 0.5 0.5 rg` *is* the components (½, ½, ½) and shows as 188 — while a `DeviceGray`
/// has a different number of colourants, is "converted or mapped to a CIE-based colour space
/// in an implementationdependent fashion", and this tree's way is the colour's sRGB (§10.3.2),
/// whose linear light is 0.214 in the group and 128 back out. The control is the same content
/// with no page group, where both greys are 128.
#[test]
fn a_cal_rgb_page_group_composites_its_components_and_leaves_by_the_matrix() {
    let calibrated = interpret(one_component_fixture(
        &format!("/Group << /S /Transparency /CS {LINEAR_CAL_RGB} >>"),
        "",
        WHITE_HALF_BLACK_TWO_GREYS,
        "",
        "",
    ));
    assert!(
        calibrated.is_complete(),
        "a CalRGB page group is drawn, not reported: {:?}",
        calibrated.unsupported
    );
    // Device y is the page's flipped: the `rg` grey at page (75, 25) is device (75, 75).
    assert_grey(
        "half of black over white composites the components to ½ and leaves by the curve",
        pixel(&calibrated, 25, 50),
        srgb_encode(0.5),
    );
    assert_grey(
        "a DeviceRGB grey of ½ is the group's own components by §11.7.2, shown as linear light",
        pixel(&calibrated, 75, 75),
        srgb_encode(0.5),
    );
    assert_grey(
        "a DeviceGray of ½ is converted in through sRGB's XYZ and comes back out as itself",
        pixel(&calibrated, 75, 25),
        0.5,
    );

    let device = interpret(one_component_fixture(
        "",
        "",
        WHITE_HALF_BLACK_TWO_GREYS,
        "",
        "",
    ));
    assert_grey(
        "on the device the two greys are one",
        pixel(&device, 75, 75),
        0.5,
    );
    assert_grey(
        "on the device the two greys are one",
        pixel(&device, 75, 25),
        0.5,
    );
    assert_grey(
        "and half of black over white is their average",
        pixel(&device, 25, 50),
        0.5,
    );
}

/// An isolated `CalRGB` group on a device page composites in its components and comes out
/// through the cube at its `Do` (ISO 32000-2 §11.6.6, §11.7.2).
///
/// §11.7.2: "all blending and compositing computations shall be done in that space", and
/// "[t]he resulting colours shall then be interpreted in the group's colour space when the
/// group is subsequently composited with its backdrop". The same marks and values as the page
/// test one scope down; the control is the same group non-isolated, whose `/CS` §11.6.6 gives
/// no effect, so its greys are the device's 128.
#[test]
fn an_isolated_cal_rgb_group_composites_in_its_components_and_leaves_by_the_cube() {
    let calibrated = interpret(one_component_fixture(
        "",
        "",
        "/Fm Do",
        &format!("/Group << /S /Transparency /I true /CS {LINEAR_CAL_RGB} >>"),
        WHITE_HALF_BLACK_TWO_GREYS,
    ));
    assert!(
        calibrated.is_complete(),
        "an isolated CalRGB group is drawn, not reported: {:?}",
        calibrated.unsupported
    );
    assert_grey(
        "half of black over white inside the group leaves by the cube",
        pixel(&calibrated, 25, 50),
        srgb_encode(0.5),
    );
    assert_grey(
        "a DeviceRGB grey inside the group is the group's components",
        pixel(&calibrated, 75, 75),
        srgb_encode(0.5),
    );
    assert_grey(
        "a DeviceGray inside the group goes in through sRGB's XYZ and out as itself",
        pixel(&calibrated, 75, 25),
        0.5,
    );

    let inherited = interpret(one_component_fixture(
        "",
        "",
        "/Fm Do",
        &format!("/Group << /S /Transparency /I false /CS {LINEAR_CAL_RGB} >>"),
        WHITE_HALF_BLACK_TWO_GREYS,
    ));
    assert!(inherited.is_complete(), "{:?}", inherited.unsupported);
    assert_grey(
        "a non-isolated group's /CS is not the space anything composites in",
        pixel(&inherited, 25, 50),
        0.5,
    );
    assert_grey(
        "and its DeviceRGB grey stays the device's",
        pixel(&inherited, 75, 75),
        0.5,
    );
}

/// §11.7.2's rule about a device space inside a CIE-based one: an isolated `/DeviceRGB`
/// group inside a `CalRGB` page is that `CalRGB`, and changes no space.
///
/// > If the colour space of the transparency group is a device colour space, and some
/// > ancestor of the group has a CIE-based colour space with the same number of colourants,
/// > then the colour space of this group shall be the CIE-based space of the nearest such
/// > ancestor.
///
/// So the page stays drawn in its own components — complete, with nothing reported — and the
/// group's half of black over white composites to ½ in the page's components, 188, exactly
/// as it would outside the group. A group stating a *different* `CalRGB` is a change of
/// space with something compositing in it, which this tree cannot convert per pixel at the
/// `Do`: the page falls back to the device and says so.
#[test]
fn a_device_rgb_group_inside_a_cal_rgb_page_is_that_cal_rgb() {
    let inherits = interpret(one_component_fixture(
        &format!("/Group << /S /Transparency /CS {LINEAR_CAL_RGB} >>"),
        "",
        "1 g 0 0 100 100 re f /Fm Do",
        "/Group << /S /Transparency /I true /CS /DeviceRGB >>",
        "q /GS gs 0 g 0 0 50 100 re f Q 0.5 0.5 0.5 rg 50 0 50 50 re f",
    ));
    assert!(
        inherits.is_complete(),
        "a DeviceRGB group inside a CalRGB page inherits the page's space: {:?}",
        inherits.unsupported
    );
    assert_grey(
        "half of black over white inside the group composites in the page's components",
        pixel(&inherits, 25, 50),
        srgb_encode(0.5),
    );
    assert_grey(
        "and a DeviceRGB grey inside it is those components too",
        pixel(&inherits, 75, 75),
        srgb_encode(0.5),
    );

    let other = "[/CalRGB << /WhitePoint [0.9505 1 1.089] /Gamma [2.2 2.2 2.2] >>]";
    let changes = interpret(one_component_fixture(
        &format!("/Group << /S /Transparency /CS {LINEAR_CAL_RGB} >>"),
        "",
        "1 g 0 0 100 100 re f /Fm Do",
        &format!("/Group << /S /Transparency /I true /CS {other} >>"),
        "q /GS gs 0 g 0 0 50 100 re f Q",
    ));
    let reported = format!("{:?}", changes.unsupported);
    assert!(
        reported.contains("blending colour space"),
        "a second CalRGB inside the first, with something compositing, is a change of space \
         the page reports: {reported}"
    );
    assert_grey(
        "and the page is drawn on the device, where half of black over white is 128",
        pixel(&changes, 25, 50),
        0.5,
    );
}

/// A one-page fixture whose page group is `/CS [/ICCBased 5 0 R]`, object 5 being
/// [`icc_rgb_profile`] as an `ASCIIHexDecode` stream with `/N 3`.
fn icc_rgb_fixture(page_group: &str, page: &str) -> Vec<u8> {
    let mut hex = String::new();
    for byte in icc_rgb_profile() {
        let _ = write!(hex, "{byte:02X}");
    }
    hex.push('>');
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] {page_group} \
         /Resources << /ExtGState << /GS << /ca 0.5 /CA 0.5 >> >> >> /Contents 4 0 R >>\n\
         endobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n<< /N 3 /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}\nendstream\n\
         endobj\n",
        page.len() + 1,
        hex.len() + 1
    );
    assemble(&body)
}

/// A v2 'RGB ' display profile in the matrix form: sRGB's D50-adapted colourants as its
/// three `XYZ ` columns and a `curv` gamma of 1.0 on each tone curve.
///
/// Positional like [`icc_gray_profile`]: the 128-byte header, a tag count of six, six tag
/// entries, then the three colourant tags (a signature, four reserved bytes and three
/// `s15Fixed16` numbers) and the three curve tags. Its components are linear sRGB, so its
/// expected values are [`LINEAR_CAL_RGB`]'s.
fn icc_rgb_profile() -> Vec<u8> {
    let mut header = vec![0u8; 128];
    header[8] = 2; // major version
    header[12..16].copy_from_slice(b"mntr");
    header[16..20].copy_from_slice(b"RGB ");
    header[20..24].copy_from_slice(b"XYZ ");
    header[36..40].copy_from_slice(b"acsp");

    let xyz_tag = |column: [f32; 3]| {
        let mut tag = Vec::new();
        tag.extend_from_slice(b"XYZ ");
        tag.extend_from_slice(&[0; 4]);
        for value in column {
            tag.extend_from_slice(&((value * 65536.0) as i32).to_be_bytes());
        }
        tag
    };
    let mut curve = Vec::new();
    curve.extend_from_slice(b"curv");
    curve.extend_from_slice(&[0; 4]);
    curve.extend_from_slice(&1u32.to_be_bytes()); // one entry: a gamma
    curve.extend_from_slice(&0x0100u16.to_be_bytes()); // 1.0 in u8Fixed8
    curve.extend_from_slice(&[0; 2]); // padding to four bytes

    // sRGB's colourants adapted onto D50, as the standard sRGB profile carries them.
    let tags: [(&[u8; 4], Vec<u8>); 6] = [
        (b"rXYZ", xyz_tag([0.4361, 0.2225, 0.0139])),
        (b"gXYZ", xyz_tag([0.3851, 0.7169, 0.0971])),
        (b"bXYZ", xyz_tag([0.1431, 0.0606, 0.7141])),
        (b"rTRC", curve.clone()),
        (b"gTRC", curve.clone()),
        (b"bTRC", curve),
    ];
    let mut out = header;
    out.extend_from_slice(&6u32.to_be_bytes());
    let mut offset = 128 + 4 + 6 * 12;
    for (name, tag) in &tags {
        out.extend_from_slice(*name);
        out.extend_from_slice(&u32::try_from(offset).expect("small").to_be_bytes());
        out.extend_from_slice(&u32::try_from(tag.len()).expect("small").to_be_bytes());
        offset += tag.len();
    }
    for (_, tag) in &tags {
        out.extend_from_slice(tag);
    }
    out
}

/// A page whose group is a three-component matrix `ICCBased` space composites in the
/// profile's components and leaves by its curves and matrix (ISO 32000-2 §11.3.4, §8.6.5.5).
///
/// §11.3.4 lists a bi-directional `ICCBased` 'RGB ' profile beside `CalRGB`, and a matrix
/// profile is bi-directional by construction — its matrix inverts and its curves are
/// monotone, which is what "capable of both device to PCS and PCS to device transformations"
/// asks. The profile below is sRGB's colourants under a gamma of 1.0, so its components are
/// linear light and every expected value is the `CalRGB` test's: half of black over white at
/// `srgb_encode(½)`, a `DeviceRGB` grey reinterpreted by §11.7.2, a `DeviceGray` converted in
/// and back out as itself.
#[test]
fn a_matrix_profile_page_group_composites_in_its_components() {
    let profiled = interpret(icc_rgb_fixture(
        "/Group << /S /Transparency /CS [/ICCBased 5 0 R] >>",
        WHITE_HALF_BLACK_TWO_GREYS,
    ));
    assert!(
        profiled.is_complete(),
        "an ICCBased 'RGB ' page group is drawn, not reported: {:?}",
        profiled.unsupported
    );
    assert_grey(
        "half of black over white leaves by the profile's curves and matrix",
        pixel(&profiled, 25, 50),
        srgb_encode(0.5),
    );
    assert_grey(
        "a DeviceRGB grey is the profile's own components by §11.7.2",
        pixel(&profiled, 75, 75),
        srgb_encode(0.5),
    );
    assert_grey(
        "a DeviceGray goes in through the inverted matrix and out as itself",
        pixel(&profiled, 75, 25),
        0.5,
    );
}

/// A v2 'RGB ' profile whose conversion is a **lookup table** rather than a matrix and three
/// curves: an `mft2` `A2B1` of two grid points an axis, and a `B2A1` beside it so that
/// §11.3.4's "capable of both device to PCS and PCS to device transformations" is met.
///
/// Its `A2B1` states sRGB's D50-adapted colourants at the eight corners — a linear map, so
/// the profile's own trilinear interpolation of the corners *is* that map and the `Y` of the
/// components `(a, b, c)` is `0.2225 a + 0.7169 b + 0.0606 c` everywhere. That is the point of
/// the fixture: the value is derivable in closed form while the profile carries no matrix and
/// no tone curve for [`pdf_model::colour::RgbRoute`] to decompose, which is the shape ADR 0851
/// samples.
///
/// The `B2A1` is the same corners run the other way and is not a true inverse; nothing here
/// depends on it beyond its presence, because every colour these tests paint is either the
/// group's own space or a `DeviceRGB` §11.7.2 reinterprets.
fn icc_rgb_table_profile() -> Vec<u8> {
    icc_rgb_table_profile_with(true)
}

/// [`icc_rgb_table_profile`], with or without the `B2A1` half §11.3.4 requires of a blending
/// space — "the ICC profile shall be capable of both device to PCS and PCS to device
/// transformations" — so that one test can state a profile that meets it and one that does not.
fn icc_rgb_table_profile_with(bidirectional: bool) -> Vec<u8> {
    // sRGB's colourants adapted onto D50, as `icc_rgb_profile` carries them: the columns of
    // the map whose middle row is the `Y` weights this fixture's expected values use.
    const COLOURANTS: [[f32; 3]; 3] = [
        [0.4361, 0.2225, 0.0139],
        [0.3851, 0.7169, 0.0971],
        [0.1431, 0.0606, 0.7141],
    ];

    let mut header = vec![0u8; 128];
    header[8] = 2; // major version
    header[12..16].copy_from_slice(b"mntr");
    header[16..20].copy_from_slice(b"RGB ");
    header[20..24].copy_from_slice(b"XYZ ");
    header[36..40].copy_from_slice(b"acsp");

    let table = |inputs: u8, outputs: u8, clut: &[u16]| {
        let mut tag = Vec::new();
        tag.extend_from_slice(b"mft2");
        tag.extend_from_slice(&[0; 4]);
        tag.push(inputs);
        tag.push(outputs);
        tag.push(2); // two grid points per axis
        tag.push(0);
        for value in [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0] {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a fixture's s15Fixed16 constants"
            )]
            tag.extend_from_slice(&((value * 65536.0) as i32).to_be_bytes());
        }
        tag.extend_from_slice(&2u16.to_be_bytes()); // input table entries
        tag.extend_from_slice(&2u16.to_be_bytes()); // output table entries
        for _ in 0..inputs {
            for value in [0u16, 0xFFFF] {
                tag.extend_from_slice(&value.to_be_bytes());
            }
        }
        for value in clut {
            tag.extend_from_slice(&value.to_be_bytes());
        }
        for _ in 0..outputs {
            for value in [0u16, 0xFFFF] {
                tag.extend_from_slice(&value.to_be_bytes());
            }
        }
        tag
    };

    // The `A2B1` CLUT, with the *last* input varying fastest, which is ICC's own order.
    let mut a2b_clut = Vec::new();
    for corner in 0..8usize {
        let at = |axis: usize| f32::from(u8::try_from((corner >> (2 - axis)) & 1).expect("a bit"));
        for row in 0..3usize {
            let value: f32 = (0..3)
                .map(|column| {
                    at(column)
                        * COLOURANTS
                            .get(column)
                            .and_then(|c| c.get(row))
                            .copied()
                            .unwrap_or(0.0)
                })
                .sum();
            // `u1Fixed15`: 0x8000 is 1.0, which is the encoding XYZ uses in a lookup table.
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "clamped into u16's range on the line above the cast"
            )]
            a2b_clut.push((value * 32768.0).clamp(0.0, 65535.0) as u16);
        }
    }
    // The `B2A1` CLUT: the corners of the connection space sent back to device values. Present
    // for §11.3.4's bi-directionality and nothing else.
    let mut b2a_clut = Vec::new();
    for corner in 0..8usize {
        for axis in 0..3usize {
            let high = (corner >> (2 - axis)) & 1 == 1;
            b2a_clut.push(if high { 0xFFFFu16 } else { 0 });
        }
    }

    let a2b = table(3, 3, &a2b_clut);
    let b2a = table(3, 3, &b2a_clut);
    let mut out = header;
    let tags = if bidirectional { 2u32 } else { 1 };
    out.extend_from_slice(&tags.to_be_bytes());
    let first = 128 + 4 + (tags as usize) * 12;
    out.extend_from_slice(b"A2B1");
    out.extend_from_slice(&u32::try_from(first).expect("small").to_be_bytes());
    out.extend_from_slice(&u32::try_from(a2b.len()).expect("small").to_be_bytes());
    if bidirectional {
        out.extend_from_slice(b"B2A1");
        out.extend_from_slice(
            &u32::try_from(first + a2b.len())
                .expect("small")
                .to_be_bytes(),
        );
        out.extend_from_slice(&u32::try_from(b2a.len()).expect("small").to_be_bytes());
    }
    out.extend_from_slice(&a2b);
    if bidirectional {
        out.extend_from_slice(&b2a);
    }
    out
}

/// A one-page fixture with a white page under a black fill masked by a `/Luminosity` group
/// whose `/CS` is [`icc_rgb_table_profile`].
///
/// `mask` is the group's content, with the space itself available as `/PR`; `backdrop` is
/// whatever Table 142 entry the soft-mask dictionary states beside `/S` and `/G`.
fn table_profile_mask_fixture(mask: &str, backdrop: &str) -> Vec<u8> {
    profile_mask_fixture(&icc_rgb_table_profile(), 3, mask, backdrop)
}

/// [`table_profile_mask_fixture`] over any profile and component count, so that one shape of
/// fixture states the drawn case and the two reported ones.
fn profile_mask_fixture(profile: &[u8], components: usize, mask: &str, backdrop: &str) -> Vec<u8> {
    let mut hex = String::new();
    for byte in profile {
        let _ = write!(hex, "{byte:02X}");
    }
    hex.push('>');
    let page = "1 g 0 0 100 100 re f /GM gs 0 g 0 0 100 100 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /ExtGState << /GM << /SMask << /S /Luminosity /G 5 0 R {backdrop} >> >> >> >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /CS [/ICCBased 6 0 R] >> \
         /Resources << /ColorSpace << /PR [/ICCBased 6 0 R] >> >> /Length {} >>\n\
         stream\n{mask}\nendstream\nendobj\n\
         6 0 obj\n<< /N {components} /Filter /ASCIIHexDecode /Length {} >>\nstream\n{hex}\n\
         endstream\nendobj\n",
        page.len() + 1,
        mask.len() + 1,
        hex.len() + 1
    );
    assemble(&body)
}

/// A `/Luminosity` mask group whose `/CS` is a three-component **table** profile takes
/// §11.5.3's `Y` over a sampled grid (ISO 32000-2 §11.5.3, §11.6.5.1, §8.6.5.5; ADR 0851).
///
/// §11.5.3 branches on the *kind* of the space and not on the shape of its arithmetic: "For
/// CIE-based spaces, convert to the CIE 1931 XYZ space and use the Y component as the
/// luminosity", and EXAMPLE 1 adds that "[a]n analogous computation applies to other CIE-based
/// colour spaces". An `ICCBased` space is CIE-based (§8.6.5.1) whether its profile carries a
/// matrix or a lookup table, so the branch is the same and only the way the `Y` is computed
/// differs — three curves summed where the profile decomposes, this grid where it does not.
///
/// [`icc_rgb_table_profile`] states sRGB's colourants at its corners, so a pure green's `Y` is
/// the middle row's green entry, 0.7169, and a black fill through the mask over a white page
/// leaves `1 − 0.7169`. **The device branch would leave `1 − 0.59`** — EXAMPLE 2's weights on
/// the same channels — which is what this tree drew before, with no report to say so.
#[test]
fn a_table_profile_mask_group_takes_the_luminance_of_its_composited_components() {
    let green = interpret(table_profile_mask_fixture(
        "/PR cs 0 1 0 sc 0 0 100 100 re f",
        "",
    ));
    assert!(
        green.is_complete(),
        "a table profile's mask group is drawn, not reported: {:?}",
        green.unsupported
    );
    assert_grey(
        "a pure green's Y is the profile's own 0.7169, not the device branch's 0.59",
        pixel(&green, 50, 50),
        1.0 - 0.7169,
    );

    let device_rgb = interpret(table_profile_mask_fixture("0 1 0 rg 0 0 100 100 re f", ""));
    assert_grey(
        "a DeviceRGB green inside the group is the group's components (§11.7.2)",
        pixel(&device_rgb, 50, 50),
        1.0 - 0.7169,
    );

    let backdrop = interpret(table_profile_mask_fixture("", "/BC [0 1 0]"));
    assert_grey(
        "and /BC states those components where the group paints nothing",
        pixel(&backdrop, 50, 50),
        1.0 - 0.7169,
    );
}

/// A `/Luminosity` mask group whose `/CS` is a **four-component** `ICCBased` space takes
/// §11.5.3's `Y` off §11.4.7's pair of rasters (ISO 32000-2 §11.5.3, §11.3.4, §11.4.7,
/// §8.6.5.1; ADR 0857).
///
/// §11.3.4 lists "ICCBased bi-directional 'GRAY', 'RGB ', and 'CMYK' colour spaces" among the
/// blending colour spaces and §8.6.5.1 makes every one of them CIE-based, so §11.5.3's branch
/// is the colorimetric one — "convert to the CIE 1931 XYZ space and use the Y component as the
/// luminosity" — whatever the component count. What four components change is not the branch
/// but the *carrier*: §11.3.4 composites per component, a raster has three channels, so the
/// group is interpreted twice and the `Y` is a function of what the two hold between them.
///
/// [`press_xyz_of`] is the fixture's press and its `Y` is the middle row's product, so every
/// expected value here comes from the model rather than from what the tree draws. The page is
/// white with a black fill through the mask, so a pixel is `1 − Y`.
///
/// Three cases, and the second is the one the pair exists for:
///
/// - **A chromatic ink.** `0 1 0 0 k` is magenta alone: `Y = 1 − 0.60`.
/// - **The black ink.** `0 0 0 1 k` paints *nothing* into the chromatic raster, so a reader
///   with one raster sees paper and masks nothing away. The clause's `Y` is `1 − 0.80`, and
///   §11.5.3's device branch would give `1 − min(1, 1)` — zero — so the three answers are
///   three different pictures and this one is the clause's.
/// - **`/BC`.** Table 142's backdrop has four components, and each raster composites onto the
///   ones it carries; stating the black ink there masks the area outside the group's marks at
///   the same `1 − 0.80`.
#[test]
fn a_four_component_mask_group_takes_the_y_of_its_pair_of_rasters() {
    let profile = two_way_cmyk_profile();
    let y_of = |inks: [f32; 4]| press_xyz_of(0, inks)[1];

    let magenta = interpret(profile_mask_fixture(
        &profile,
        4,
        "0 1 0 0 k 0 0 100 100 re f",
        "",
    ));
    assert!(
        magenta.is_complete(),
        "a four-component profile's mask group is drawn, not reported: {:?}",
        magenta.unsupported
    );
    assert_grey(
        "a magenta ink's Y is the press's own, not EXAMPLE 2's weights on the channels",
        pixel(&magenta, 50, 50),
        1.0 - y_of([0.0, 1.0, 0.0, 0.0]),
    );

    let black = interpret(profile_mask_fixture(
        &profile,
        4,
        "0 0 0 1 k 0 0 100 100 re f",
        "",
    ));
    assert!(
        black.is_complete(),
        "and so is one that paints only the black component: {:?}",
        black.unsupported
    );
    assert_grey(
        "the black component reaches the Y, which one raster could not have carried",
        pixel(&black, 50, 50),
        1.0 - y_of([0.0, 0.0, 0.0, 1.0]),
    );

    let backdrop = interpret(profile_mask_fixture(&profile, 4, "", "/BC [0 0 0 1]"));
    assert_grey(
        "and /BC states all four components where the group paints nothing",
        pixel(&backdrop, 50, 50),
        1.0 - y_of([0.0, 0.0, 0.0, 1.0]),
    );
}

/// A `/Luminosity` mask group whose `/CS` is CIE-based and whose `Y` this reader cannot take
/// is **named** rather than drawn in silence (ISO 32000-2 §11.5.3, §11.3.4; ADR 0851).
///
/// §11.5.3 branches on the space's kind — "For CIE-based spaces, convert to the CIE 1931 XYZ
/// space and use the Y component as the luminosity" — so a CIE-based space that does not take
/// that branch is a departure whatever the reason, and the report fires on exactly that
/// condition rather than on a list of space names.
///
/// Two shapes reach it, and neither had a report before this round:
///
/// - **Four components with no press.** An `ICCBased` profile of four channels is CIE-based
///   (§8.6.5.1) and §11.3.4 lists it as a blending space, and since ADR 0857 such a group is
///   *drawn* — §11.4.7's pair of rasters, with the `Y` read off the press's own grid over the
///   four components. What is left reported is a profile this reader has no press for: a
///   one-way profile, which §11.3.4 rules out of a blending space in as many words, or a page
///   that has already named as many distinct presses as §11.7.2's budget allows.
/// - **No route in.** §11.3.4 requires a blending space's profile to be "capable of both
///   device to PCS and PCS to device transformations"; a table profile with no `B2A` is not,
///   so there is nothing to composite the group's marks in even though its `A2B` would give
///   the `Y`.
///
/// Both are drawn on the device's three components, which is what EXAMPLE 2's weights make of
/// them — `0.30 × 0 + 0.59 × 1 + 0.11 × 0` for the green below, leaving `1 − 0.59`.
#[test]
fn a_mask_group_in_a_cie_space_with_no_route_is_named_rather_than_drawn_in_silence() {
    let four = interpret(profile_mask_fixture(
        &icc_cmyk_profile(),
        4,
        "0 1 0 0 k 0 0 100 100 re f",
        "",
    ));
    let reported = format!("{:?}", four.unsupported);
    assert!(
        reported.contains("four-component ICCBased /CS this reader has no press for"),
        "a one-way four-component profile as a mask group's /CS is named: {reported}"
    );

    let one_way = interpret(profile_mask_fixture(
        &icc_rgb_table_profile_with(false),
        3,
        "0 1 0 rg 0 0 100 100 re f",
        "",
    ));
    let reported = format!("{:?}", one_way.unsupported);
    assert!(
        reported.contains("CIE-based /CS this reader has no route into"),
        "a profile with no from-CIE half is named: {reported}"
    );
    assert_grey(
        "and it is drawn on the device, where EXAMPLE 2's weights make a green 0.59",
        pixel(&one_way, 50, 50),
        1.0 - 0.59,
    );

    let drawn = interpret(profile_mask_fixture(
        &icc_rgb_table_profile_with(true),
        3,
        "0 1 0 rg 0 0 100 100 re f",
        "",
    ));
    assert!(
        drawn.is_complete(),
        "and the same profile with its B2A takes the clause's branch and is not named: {:?}",
        drawn.unsupported
    );
}

/// `Lab` as a page group's `/CS` is reported, and named as the space §11.3.4 forbids.
///
/// > The Lab space and ICCBased spaces that represent lightness and chromaticity separately
/// > (such as L*a*b*, L*u*v*, and HSV ) shall not be used as blending colour spaces because
/// > the compositing computations in such spaces do not give meaningful results when applied
/// > separately to each component.
///
/// So there is no route into it and the page stays on the device, where the half of black
/// over white is 128, and the report says which three-component spaces there *are* routes into.
#[test]
fn a_lab_page_group_is_reported_as_the_space_the_clause_forbids() {
    let lab = interpret(one_component_fixture(
        "/Group << /S /Transparency /CS [/Lab << /WhitePoint [0.9505 1 1.089] >>] >>",
        "",
        WHITE_HALF_BLACK_TWO_GREYS,
        "",
        "",
    ));
    let reported = format!("{:?}", lab.unsupported);
    assert!(
        reported.contains("§11.3.4 lists CalRGB and a bi-directional ICCBased 'RGB '")
            && reported.contains("forbids Lab"),
        "a Lab page group is named with the clause's own list: {reported}"
    );
    assert_grey(
        "and the page is drawn on the device",
        pixel(&lab, 25, 50),
        0.5,
    );
}

/// A one-page fixture with a white page under a black fill masked by a `/Luminosity` group
/// whose `/CS` is a `CalRGB` of gamma 2 on sRGB's primaries, so that the mask value is
/// §11.5.3's EXAMPLE 1 written out.
///
/// `mask` is the group's content, with the space itself available as `/CR`; `backdrop` is
/// whatever Table 142 entry the soft-mask dictionary states beside `/S` and `/G`.
fn additive_mask_fixture(mask: &str, backdrop: &str) -> Vec<u8> {
    let cal_rgb = "[/CalRGB << /WhitePoint [0.9505 1 1.089] /Gamma [2 2 2] \
                   /Matrix [0.4124 0.2126 0.0193 0.3576 0.7152 0.1192 0.1805 0.0722 0.9505] >>]";
    let page = "1 g 0 0 100 100 re f /GM gs 0 g 0 0 100 100 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /ExtGState << /GM << /SMask << /S /Luminosity /G 5 0 R {backdrop} >> >> >> >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /CS {cal_rgb} >> \
         /Resources << /ColorSpace << /CR {cal_rgb} >> >> /Length {} >>\n\
         stream\n{mask}\nendstream\nendobj\n",
        page.len() + 1,
        mask.len() + 1
    );
    assemble(&body)
}

/// A `/Luminosity` mask group whose `/CS` is `CalRGB` is composited in its components and
/// its mask value is §11.5.3's `Y` of the result (ISO 32000-2 §11.5.3, §11.6.5.1, §8.6.5.3).
///
/// §11.5.3: "For CIE-based spaces, convert to the CIE 1931 XYZ space and use the Y component
/// as the luminosity", and its EXAMPLE 1 writes the `CalRGB` case out as the `Y` entries of
/// the `Matrix` weighting each gamma-decoded component. With sRGB's primaries and a gamma of
/// 2 on each, a component of ½ apiece is `0.2126 × ¼ + 0.7152 × ¼ + 0.0722 × ¼ = ¼`, so a
/// black fill through that mask over white leaves ¾; a pure green is `0.7152` and leaves
/// `0.2848`, where the device branch's `0.59` would leave `0.41`. Table 142's `/BC` is "n
/// numbers, where n is the number of components in the colour space specified by the CS
/// entry", three, and `/BC [0.5 0.5 0.5]` is the same ¾ where the group paints nothing. A
/// `DeviceRGB` grey inside the group is the group's components by §11.7.2 and masks the same
/// ¾; a `DeviceGray` goes in through sRGB's XYZ, so its `Y` is sRGB's decoding of ½, 0.214.
#[test]
fn a_cal_rgb_mask_group_takes_the_luminance_of_its_composited_components() {
    let painted = interpret(additive_mask_fixture(
        "/CR cs 0.5 0.5 0.5 sc 0 0 100 100 re f",
        "",
    ));
    assert!(
        painted.is_complete(),
        "a CalRGB mask group is drawn, not reported: {:?}",
        painted.unsupported
    );
    assert_grey(
        "components of ½ under gamma 2 have a Y of ¼ and mask a quarter of the black",
        pixel(&painted, 50, 50),
        0.75,
    );

    let green = interpret(additive_mask_fixture(
        "/CR cs 0 1 0 sc 0 0 100 100 re f",
        "",
    ));
    assert_grey(
        "a pure green's Y is its Matrix column's, 0.7152, not the device branch's 0.59",
        pixel(&green, 50, 50),
        1.0 - 0.7152,
    );

    let backdrop = interpret(additive_mask_fixture("", "/BC [0.5 0.5 0.5]"));
    assert_grey(
        "and /BC states those components where the group paints nothing",
        pixel(&backdrop, 50, 50),
        0.75,
    );

    let device_rgb = interpret(additive_mask_fixture("0.5 0.5 0.5 rg 0 0 100 100 re f", ""));
    assert_grey(
        "a DeviceRGB grey inside the group is the group's components (§11.7.2)",
        pixel(&device_rgb, 50, 50),
        0.75,
    );

    let device_gray = interpret(additive_mask_fixture("0.5 g 0 0 100 100 re f", ""));
    let linear = ((0.5f32 + 0.055) / 1.055).powf(2.4);
    assert_grey(
        "a DeviceGray goes in through sRGB's XYZ and its Y is that grey's linear light",
        pixel(&device_gray, 50, 50),
        1.0 - linear,
    );
}
