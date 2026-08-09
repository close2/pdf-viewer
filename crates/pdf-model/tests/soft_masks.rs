//! A soft mask in the graphics state: ISO 32000-2 §11.5 and §11.6.5.1, from the file down.
//!
//! `render-cpu/tests/soft_mask.rs` checks the arithmetic a mask value produces once it
//! exists. This file checks the half above it — the dictionary, the group, and where the
//! mask lands on the page — on documents written here rather than taken from a corpus, for
//! trap 8's reason: the corpus can only exercise what somebody happened to write, and three
//! of the four rules below are ones no corpus document distinguishes.
//!
//! Every expected value is derived from the clause and stated in the test that uses it. The
//! fixtures are 40-unit pages drawn at one pixel per unit, so a coordinate in the content
//! stream is a pixel.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture or an out-of-range pixel should fail loudly, \
              and the fixtures are 40x40 pages where no index can overflow"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the 40×40 pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// A one-page PDF whose content stream is `content`, with one `/ExtGState` named `/GS`.
///
/// `gstate` is what goes inside that dictionary and `group` is the content stream of the
/// form `XObject` its `/SMask` names, which is object 6 and always a transparency group
/// covering the left half of the page, blending in `/DeviceGray`.
fn page(gstate: &str, content: &str, group: &str) -> Vec<u8> {
    page_blending_in("/DeviceGray", gstate, content, group)
}

/// [`page`], with the mask group's own blending colour space named.
///
/// §11.6.5.1 makes that entry required for a luminosity mask — "the group attributes
/// dictionary shall contain a CS entry defining the colour space in which the compositing
/// computation is to be performed" — and §11.5.3 makes it decide the arithmetic, so it is
/// the parameter these fixtures vary.
fn page_blending_in(space: &str, gstate: &str, content: &str, group: &str) -> Vec<u8> {
    page_with_group_resources(space, gstate, content, "<< >>", group, &[])
}

/// [`page_blending_in`], with resources and extra objects for the mask group to name.
///
/// A group that draws an image or a shading needs both, and they are the two things §11.5.3's
/// second residue was about — colour that reaches the group already rasterised. `extra` is
/// appended after object 6, so the first of them is object 7.
fn page_with_group_resources(
    space: &str,
    gstate: &str,
    content: &str,
    group_resources: &str,
    group: &str,
    extra: &[Vec<u8>],
) -> Vec<u8> {
    let mut objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Resources << /ExtGState << /GS 5 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        stream_object(4, "", content.as_bytes()),
        format!("5 0 obj\n<< /Type /ExtGState {gstate} >>\nendobj\n").into_bytes(),
        stream_object(
            6,
            &format!(
                "/Type /XObject /Subtype /Form /BBox [0 0 20 40] /Resources {group_resources} \
                 /Group << /Type /Group /S /Transparency /CS {space} >>"
            ),
            group.as_bytes(),
        ),
    ];
    objects.extend_from_slice(extra);
    assemble(&objects)
}

/// One numbered stream object, with the `/Length` its data actually has.
fn stream_object(number: usize, dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!(
        "{number} 0 obj\n<< {dict} /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    out
}

/// Assembles numbered objects into a file with a cross-reference table that points at them.
fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in objects {
        offsets.push(out.len());
        out.extend_from_slice(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let mut trailer = String::new();
    let _ = writeln!(trailer, "xref\n0 {size}");
    trailer.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(trailer.as_bytes());
    out
}

/// Renders a fixture at one pixel per unit onto white, and requires it to draw completely.
fn render(bytes: Vec<u8>) -> pdf_render::Raster {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the fixture should draw completely: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported")
}

/// The RGBA at a point given in PDF coordinates, whose y runs the other way from a raster's.
fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let at = ((row.saturating_mul(raster.width)).saturating_add(x) as usize).saturating_mul(4);
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

/// The red channel at a point, which is what a black object over white says about its mask.
///
/// Painting black through a mask of value `v` over white leaves `255 × (1 − v)` in every
/// channel, so one channel is the whole answer and reading it as a number keeps the
/// assertions in the units the clause is written in.
fn level(raster: &pdf_render::Raster, x: u32, y: u32) -> i32 {
    i32::from(pixel(raster, x, y)[0])
}

/// Asserts a level within one, which is what an eight-bit mask value can promise.
fn near(got: i32, want: i32, what: &str) {
    assert!(
        (got - want).abs() <= 1,
        "{what}: level {got} is more than one away from the clause's {want}"
    );
}

/// A luminosity mask masks by the grey its group paints, and by `/BC` outside its box.
///
/// §11.5.3 derives the mask from "the luminosity of the resulting colour" of the group
/// composited onto the `/BC` backdrop, and §11.6.5.1 gives the area outside the group's
/// bounding box the same treatment: "the mask value shall be derived by transforming the BC
/// colour to luminosity". The fixture's group is a `/DeviceGray` form whose `/BBox` covers
/// the left half of the page and which paints 0.25 grey over its left quarter and 1.0 over
/// the rest of the box; `/BC` is 0.5.
///
/// So a black rectangle drawn over the whole page through that mask leaves, left to right:
/// 75% of the white showing where the mask is 0.25, none where it is 1.0, and half where the
/// backdrop decides it.
#[test]
fn a_luminosity_mask_takes_the_groups_grey_and_the_backdrops_outside_it() {
    let raster = render(page(
        "/SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [0.5] >>",
        "q /GS gs 0 g 0 0 40 40 re f Q",
        "0.25 g 0 0 10 40 re f 1 g 10 0 10 40 re f",
    ));

    near(
        level(&raster, 5, 20),
        191,
        "a mask of 0.25 leaves 75% of white",
    );
    near(level(&raster, 15, 20), 0, "a mask of 1.0 paints the object");
    near(
        level(&raster, 30, 20),
        128,
        "outside the group's /BBox the mask is the backdrop's luminosity",
    );
}

/// `/TR` maps the derived value, and the identity is what its absence means.
///
/// §11.6.5.1, of `/TR`: "The value of the TR key in the `SoftMask` dictionary, which is a
/// transfer function, shall then be applied to the computed group alpha to produce the mask
/// values."
/// The function here is `1 − x`, written as a type 2 exponential with `C0 = 1` and `C1 = 0`,
/// so every expectation of the test above is inverted — including the one outside the
/// bounding box, which is the case §11.6.5.1 states separately and the one an implementation
/// forgets.
#[test]
fn a_transfer_function_maps_every_mask_value_including_the_one_outside_the_box() {
    let raster = render(page(
        "/SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [0.5] \
         /TR << /FunctionType 2 /Domain [0 1] /Range [0 1] /N 1 /C0 [1] /C1 [0] >> >>",
        "q /GS gs 0 g 0 0 40 40 re f Q",
        "0.25 g 0 0 10 40 re f 1 g 10 0 10 40 re f",
    ));

    near(level(&raster, 5, 20), 64, "1 - 0.25 paints three quarters");
    near(level(&raster, 15, 20), 255, "1 - 1.0 paints nothing at all");
    near(
        level(&raster, 30, 20),
        128,
        "1 - 0.5 outside the box is still half",
    );
}

/// An alpha mask reads the group's alpha and ignores its colour (§11.5.2).
///
/// The group paints two rectangles of the *same* black at two different constant alphas, so
/// a derivation that looked at colour or at luminosity would make both halves identical —
/// and would mask everything away, since black has no luminosity. `/BC` is deliberately
/// present and deliberately irrelevant: Table 142 says it "shall be consulted only if the
/// subtype S is Luminosity".
///
/// **The fixture did not have those two alphas until the four-hundred-and-nineteenth
/// session.** It named `/GA` and `/GB` in a group whose `/Resources` defined neither, so both
/// fills were opaque, both halves masked identically at 1.0, and the test asserted the same
/// number twice about a difference that was not in the file — with a comment inside it saying
/// so and a doc comment above it saying the opposite. Nothing could see that: the interpreter
/// answered a `gs` naming an undefined `/ExtGState` in silence, which is what this round made
/// loud (ADR 0255), and the assertion this test exists for was the one it was not making.
#[test]
fn an_alpha_mask_reads_the_groups_alpha_and_ignores_its_colour() {
    let raster = render(page_with_group_resources(
        "/DeviceGray",
        "/SMask << /Type /Mask /S /Alpha /G 6 0 R /BC [1] >>",
        "q /GS gs 0 g 0 0 40 40 re f Q",
        "<< /ExtGState << /GA 7 0 R /GB 8 0 R >> >>",
        "/GA gs 0 g 0 0 10 40 re f /GB gs 0 g 10 0 10 40 re f",
        &[
            b"7 0 obj\n<< /Type /ExtGState /ca 0.25 >>\nendobj\n".to_vec(),
            b"8 0 obj\n<< /Type /ExtGState /ca 0.75 >>\nendobj\n".to_vec(),
        ],
    ));

    // §11.5.2: the mask value is the group's alpha, so black painted through it leaves
    // 255 × (1 − α) of the white behind it. Both fills are the same colour, so a derivation
    // reading colour or luminosity would give one number for both.
    near(
        level(&raster, 5, 20),
        191,
        "a group alpha of 0.25 leaves three quarters of the white",
    );
    near(
        level(&raster, 15, 20),
        64,
        "and 0.75 leaves a quarter of it, from the same black",
    );
    near(
        level(&raster, 30, 20),
        255,
        "outside the box an alpha mask is the transfer function of 0.0, whatever /BC says",
    );
}

/// `/SMask /None` removes the mask, and `q`/`Q` restores it.
///
/// §11.6.4.3: "The name None may be specified in place of a soft-mask dictionary, denoting
/// the absence of a soft mask. It shall also mean that any existing mask shall be removed
/// from the current graphics state." The fixture sets the mask, paints, sets `/None`, paints
/// again over the same place — and the second rectangle must be solid.
#[test]
fn none_removes_the_mask() {
    let mut objects = page(
        "/SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [0] >>",
        "q /GS gs /Off gs 0 g 0 0 40 40 re f Q",
        "1 g 0 0 20 40 re f",
    );
    // A second `/ExtGState`, `/Off`, added to the page's resources by rewriting the one
    // place they are written. Cheaper than a second builder and impossible to get subtly
    // wrong: if the name does not resolve, `gs` reports and `render` fails the fixture.
    let with_off = String::from_utf8(objects)
        .expect("the fixture is ASCII")
        .replace(
            "/ExtGState << /GS 5 0 R >>",
            "/ExtGState << /GS 5 0 R /Off 7 0 R >>",
        )
        .replace(
            "trailer\n<< /Size 7",
            "7 0 obj\n<< /Type /ExtGState /SMask /None >>\nendobj\ntrailer\n<< /Size 8",
        );
    objects = with_off.into_bytes();

    // The cross-reference table now names an object it has no offset for, which is exactly
    // the malformation `pdf-syntax` recovers from by scanning; the fixture is valid enough
    // to open and that is all this test needs it to be.
    let raster = render(objects);
    near(
        level(&raster, 5, 20),
        0,
        "with the mask removed the black rectangle is solid",
    );
    near(level(&raster, 30, 20), 0, "everywhere, not only in the box");
}

/// A mask is fixed where the `gs` established it, not where the object is painted.
///
/// §11.6.5.1: "The mask's coordinate system shall be defined by concatenating the
/// transformation matrix specified by the Matrix entry in the transparency group's form
/// dictionary … with the current transformation matrix at the moment the soft mask is
/// established in the graphics state with the gs operator." The fixture translates the
/// coordinate system by 20 units *after* the `gs`, so a mask that moved with the object
/// would cover the right half of the page instead of the left.
#[test]
fn the_masks_coordinate_system_is_the_one_in_force_at_the_gs() {
    let raster = render(page(
        "/SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [0] >>",
        "q /GS gs 1 0 0 1 20 0 cm 0 g -20 0 40 40 re f Q",
        "1 g 0 0 20 40 re f",
    ));

    near(
        level(&raster, 5, 20),
        0,
        "the mask's white half is where the gs left it",
    );
    near(
        level(&raster, 30, 20),
        255,
        "and the black backdrop still masks the other half away",
    );
}

/// §8.6.8's uncoloured restriction does not reach inside a soft mask's group.
///
/// The clause restricts colour operators "[i]n any glyph description that uses the d1
/// operator … and to all other content streams invoked from within the same glyph
/// description", and says why one sentence earlier: the restriction is for "graphical figures
/// whose colours shall be specified separately each time they are used". A soft mask is not
/// one. It carries no colour to the page — §11.6.5.2 turns the group's result into a
/// luminosity and uses it as *alpha* — so NOTE 1's reason for exempting a stencil applies
/// word for word, and the restriction is destructive rather than neutral here: a
/// `/Luminosity` mask's values **are** its group's colours.
///
/// The fixture is the shape `issue19634.pdf` uses: a `d1` glyph description whose whole body
/// is a `gs` naming a luminosity mask and one filled rectangle. The mask's group is stated in
/// *glyph* space, because §11.6.5.1 fixes the mask's coordinate system at the `gs` and the
/// `gs` is inside the glyph; it paints mid grey, so the glyph's black must come out about
/// half strength. With the flag leaking in,
/// the group's `0.5 g` was ignored, the group painted its initial black, and the mask came
/// out **zero** — the glyph vanished, in silence.
#[test]
fn a_soft_mask_group_is_not_bound_by_an_uncoloured_glyphs_restriction() {
    let objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Resources << /Font << /FT3 7 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        stream_object(4, "", b"BT /FT3 40 Tf 0 0 0 rg 0 10 Td (a) Tj ET"),
        b"5 0 obj\n<< /Type /ExtGState /SMask << /Type /Mask /S /Luminosity /G 6 0 R >> \
          >>\nendobj\n"
            .to_vec(),
        stream_object(
            6,
            "/Type /XObject /Subtype /Form /BBox [0 0 1000 1000] \
             /Group << /Type /Group /S /Transparency /CS /DeviceGray >>",
            b"0.5 g 0 0 1000 1000 re f",
        ),
        b"7 0 obj\n<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1000 1000] \
          /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs 9 0 R /Encoding 8 0 R \
          /FirstChar 97 /LastChar 97 /Widths [1000] \
          /Resources << /ExtGState << /GS 5 0 R >> >> >>\nendobj\n"
            .to_vec(),
        b"8 0 obj\n<< /Type /Encoding /Differences [97 /box] >>\nendobj\n".to_vec(),
        b"9 0 obj\n<< /box 10 0 R >>\nendobj\n".to_vec(),
        stream_object(
            10,
            "",
            b"1000 0 0 0 1000 1000 d1\n/GS gs\n0 0 1000 1000 re f",
        ),
    ];
    let raster = render(assemble(&objects));

    // Black through a mask of 0.5 over white is 128. Before the fix this pixel was 255: the
    // mask group's fill was ignored, so the mask was zero and the glyph painted nothing.
    near(
        level(&raster, 20, 30),
        128,
        "the glyph is painted at the mask's own value",
    );
}

/// §11.5.3's luminosity is computed in the space the group's `/CS` names, not in the device's.
///
/// §11.6.5.1 makes that entry "the colour space in which the compositing computation is to be
/// performed", and §11.5.3 then converts the result: "For device colour spaces, convert the
/// colour to `DeviceGray` by implementation-defined means and use the resulting gray value as
/// the luminosity". §10.4.2.3 states those means for `DeviceCMYK`, and its formula is the one
/// §11.5.3's EXAMPLE 2 prints —
/// `gray = 1 − min(1, 0.3 × cyan + 0.59 × magenta + 0.11 × yellow + black)`.
///
/// The fixture's mask group blends in `/DeviceCMYK` and paints three colours whose grey levels
/// the clause fixes exactly:
///
/// | `k` operands | ink | `gray` | black through it, over white |
/// |---|---|---|---|
/// | `0 0 0 1` (process black) | 1.00 | 0.00 | 255 — nothing painted |
/// | `0 0 0 0` (no ink at all)  | 0.00 | 1.00 | 0 — painted solid |
/// | `0 0 0 0.5`                | 0.50 | 0.50 | 128 |
///
/// **The first row is what this test exists for.** Until the three-hundred-and-eightieth
/// session the group was composited on the device's three components, so process black went
/// through this tree's `DeviceCMYK` → RGB table (ADR 0009) to `(35, 31, 32)` and came back as
/// a mask value of 32 — content its producer had masked away, faintly there at 12.5%. The
/// clause's own arithmetic gives 0.
#[test]
fn a_cmyk_mask_group_takes_its_luminosity_from_the_clauses_own_formula() {
    let raster = render(page_blending_in(
        "/DeviceCMYK",
        "/SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [0 0 0 1] >>",
        "q /GS gs 0 g 0 0 40 40 re f Q",
        "0 0 0 1 k 0 0 7 40 re f  0 0 0 0 k 7 0 6 40 re f  0 0 0 0.5 k 13 0 7 40 re f",
    ));

    near(
        level(&raster, 3, 20),
        255,
        "1 - min(1, 1) = 0 masks process black away entirely",
    );
    near(level(&raster, 10, 20), 0, "1 - min(1, 0) = 1 paints it all");
    near(level(&raster, 16, 20), 128, "1 - min(1, 0.5) = 0.5");
    near(
        level(&raster, 30, 20),
        255,
        "outside the /BBox, /BC [0 0 0 1] is the same process black and the same 0",
    );
}

/// An absent `/BC` in a `DeviceCMYK` group is process black, and process black masks fully.
///
/// Table 142 gives `/BC` the default "the colour space's initial value, representing black",
/// and §8.6.8 makes that `[0.0 0.0 0.0 1.0]` for `DeviceCMYK` — one whole unit of §10.4.2.3's
/// ink, so a grey level of exactly 0. `issue14200.pdf` is the corpus's witness: a mask group
/// whose content stream is `q Q` and whose `/BC` is absent, so its mask is that one number
/// everywhere.
#[test]
fn an_absent_backdrop_in_a_cmyk_group_masks_everything_away() {
    let raster = render(page_blending_in(
        "/DeviceCMYK",
        "/SMask << /Type /Mask /S /Luminosity /G 6 0 R >>",
        "q /GS gs 0 g 0 0 40 40 re f Q",
        "",
    ));

    for x in [3, 10, 16, 30] {
        near(
            level(&raster, x, 20),
            255,
            "an empty group over the default backdrop masks the whole page",
        );
    }
}

/// A grey painted in a `DeviceCMYK` group is the same mask value as in a `DeviceGray` one.
///
/// Worth a test rather than an assumption, because it is the reason this tree needs no
/// conversion from a painted colour's space into the group's. §10.4.2.3 sends a grey `g` to
/// `(0, 0, 0, 1 − g)` and back to `1 − min(1, 1 − g) = g`; §10.4.2.4 sends an RGB colour
/// through a black generation whose every term cancels, because §10.4.2.3's three weights sum
/// to 1. So the two fixtures below differ in one name and in nothing a mask can see.
///
/// **To within one level of 255, which is the whole of what the scaled channel costs.** A
/// `DeviceCMYK` group's channel carries `1 − ink ÷ 2` so that §10.4.2.3's `min` can wait for
/// the compositing (`InkScale`, ADR 0220), and eight bits of that recover the ink in steps of
/// `2 ÷ 255`; a `DeviceGray` group's channel is unscaled because §11.6.6's conversion *into*
/// that space has already applied the `min`. `near` is the assertion, and it is the same one
/// level every other line in this file allows an eight-bit mask.
#[test]
fn a_grey_masks_the_same_whichever_of_the_two_device_spaces_the_group_blends_in() {
    let group = "0.25 g 0 0 10 40 re f 1 g 10 0 10 40 re f";
    let content = "q /GS gs 0 g 0 0 40 40 re f Q";
    let state = "/SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [0.5] >>";
    let grey = render(page_blending_in("/DeviceGray", state, content, group));
    let cmyk = render(page_blending_in(
        "/DeviceCMYK",
        "/SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [0 0 0 0.5] >>",
        content,
        group,
    ));

    for x in [5, 15, 30] {
        near(
            level(&cmyk, x, 20),
            level(&grey, x, 20),
            &format!("the same grey artwork masks the same at x = {x}"),
        );
    }
    near(
        level(&cmyk, 5, 20),
        191,
        "a mask of 0.25 leaves 75% of white",
    );
    near(
        level(&cmyk, 30, 20),
        128,
        "/BC [0 0 0 0.5] is a grey of 0.5",
    );
}

/// Half-covered artwork over registration black: §10.4.2.3's `min` waits for the compositing.
///
/// §11.5.3 composites the group with its backdrop *first* and takes the luminosity of the
/// result, and §10.4.2.3's conversion to a grey level ends in `min(1.0, …)`. A `DeviceCMYK`
/// colour can weigh two whole units of ink — `/BC [1 1 1 1]`, registration black, is exactly
/// that — so a rendered channel holding `0..=1` cannot carry the group's arithmetic unless it
/// is scaled, which `pdf_model::colour::InkScale` is (ADR 0220).
///
/// The fixture is the whole of the departure in one page. The group paints `0 0 0 0 k`, no
/// ink at all, at a constant alpha of 0.5 over a backdrop weighing 2.0, so:
///
/// - the clause gives `1 − min(1, 0.5 × 0 + 0.5 × 2) = 0`, and the page keeps its white;
/// - clamping each colour before the mixing gives `0.5 × 1 + 0.5 × 0 = 0.5`, a mask of 128,
///   and half the page's black — which is what this tree drew until this round, and which
///   putting the old route back still draws.
///
/// That is the closed form `(1 − α) · e` at its widest: `e` is one whole unit of ink and `α`
/// is a half, so the two answers are 127 of 255 apart.
#[test]
fn half_covered_artwork_over_registration_black_is_clamped_after_the_compositing() {
    let raster = render(page_with_group_resources(
        "/DeviceCMYK",
        "/SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [1 1 1 1] >>",
        "q /GS gs 0 g 0 0 40 40 re f Q",
        "<< /ExtGState << /GA 7 0 R >> >>",
        "q /GA gs 0 0 0 0 k 0 0 20 40 re f Q",
        &[b"7 0 obj\n<< /Type /ExtGState /ca 0.5 >>\nendobj\n".to_vec()],
    ));

    for x in [5, 15] {
        near(
            level(&raster, x, 20),
            255,
            "half of no ink over two units of it still weighs one whole unit",
        );
    }
    near(
        level(&raster, 30, 20),
        255,
        "and outside the /BBox the backdrop's own two units mask everything away",
    );
}

/// A `DeviceCMYK` image inside a mask group is decoded into the group's own components.
///
/// §11.6.6 makes the group's `/CS` "[t]he colour space into which colours shall be converted
/// when painted into the group", and §11.6.2 is explicit that an image's samples are colours
/// like any other: an object's source colour "shall be specified in the same way as in the
/// opaque imaging model: by means of the current colour in the graphics state or the source
/// samples in an image". So a sample has to reach the mask in the quantity §11.5.3
/// composites, and until this round it reached it as RGB — which for `DeviceCMYK` is a
/// different number, because this tree's route to a pixel is a table of ink appearances
/// (ADR 0009) and §10.4.2.3 weighs the four components directly.
///
/// The image is two samples wide over the group's whole box: pure cyan, then no ink at all.
/// §10.4.2.3 weighs cyan at `0.3` and gives it the grey level `0.7`, so the left half of the
/// page keeps 30% of its black — `255 − 179`, and the mask arrives at 179 rather than 178
/// because the group's channel carries `1 − ink ÷ 2` and eight bits of that is a step of two.
/// The right half is a whole unit of no ink at all: `1 − min(1, 0) = 1`, and nothing masked.
///
/// **Cyan rather than process black, and the reason is worth the sentence.** Both routes send
/// process black to a mask of zero — this tree renders it as `(35, 31, 32)`, whose §10.4.2.2
/// grey is 32, and a `DeviceCMYK` group's scaled channel then crushes 32 to nothing — so it
/// would pass whichever route drew it. Cyan separates them: the RGB this tree renders it as
/// has a grey level of 0.516 against the clause's 0.7, and putting the old route back draws
/// this fixture's left half at 254 instead of 76 — the scaled channel then crushing what is
/// left of the difference, which is why the check is worth having on a colour that survives
/// both steps.
#[test]
fn a_cmyk_image_in_a_mask_group_is_decoded_into_the_groups_own_components() {
    let mut image = b"7 0 obj\n<< /Type /XObject /Subtype /Image /Width 2 /Height 1 \
          /ColorSpace /DeviceCMYK /BitsPerComponent 8 /Length 8 >>\nstream\n"
        .to_vec();
    image.extend_from_slice(&[255, 0, 0, 0, 0, 0, 0, 0]);
    image.extend_from_slice(b"\nendstream\nendobj\n");

    let raster = render(page_with_group_resources(
        "/DeviceCMYK",
        "/SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [0 0 0 1] >>",
        "q /GS gs 0 g 0 0 40 40 re f Q",
        "<< /XObject << /Im 7 0 R >> >>",
        "q 20 0 0 40 0 0 cm /Im Do Q",
        &[image],
    ));

    near(
        level(&raster, 5, 20),
        76,
        "cyan weighs 0.3 of a unit of ink and leaves a mask of 0.7",
    );
    near(
        level(&raster, 15, 20),
        0,
        "and a sample of no ink at all masks nothing",
    );
    near(
        level(&raster, 30, 20),
        255,
        "while /BC's one whole unit masks everything outside the box",
    );
}

/// A `DeviceCMYK` shading inside a `/DeviceGray` mask group is sampled the same way.
///
/// The second half of the same rule, and the corpus's own shape:
/// `bug1703683_page2_reduced.pdf` draws a `/DeviceN` shading whose alternate rests on
/// `DeviceCMYK` inside a `/DeviceGray` luminosity mask group, and `issue13520.pdf` draws two
/// more. A shading's colours reach a display list as a [`pdf_render::Ramp`] — 256 evaluations
/// of §8.7.4.5.3's function — so the conversion happens once per stop and has to happen in
/// the group's quantity for the same reason an image's samples do.
///
/// The ramp runs from no ink to pure cyan across the group's twenty units, so at a pixel whose
/// centre is at parameter `t` the ink is `0.3 × t` and §10.4.2.3's grey level is `1 − 0.3 t`.
/// A black page through that mask therefore keeps `255 × 0.3 t` of nothing — `76.5 t` — and
/// the three columns below are that arithmetic at `t = 0.025`, `0.525` and `0.975`.
///
/// **The old route is far away rather than nearby**, which is what makes this a measurement:
/// cyan renders as `(0.025, 0.687, 0.939)` at full tint, whose §10.4.2.2 grey is 0.516
/// against the clause's 0.7, and putting it back draws the middle column at 66 instead of 40.
#[test]
fn a_cmyk_shading_in_a_grey_mask_group_is_sampled_into_the_groups_own_components() {
    let extra = vec![
        b"7 0 obj\n<< /ShadingType 2 /ColorSpace /DeviceCMYK /Coords [0 0 20 0] \
          /Extend [true true] /Function 8 0 R >>\nendobj\n"
            .to_vec(),
        b"8 0 obj\n<< /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [1 0 0 0] /N 1 >>\nendobj\n"
            .to_vec(),
    ];
    let raster = render(page_with_group_resources(
        "/DeviceGray",
        "/SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [0] >>",
        "q /GS gs 0 g 0 0 40 40 re f Q",
        "<< /Shading << /Sh 7 0 R >> >>",
        "q 0 0 20 40 re W n /Sh sh Q",
        &extra,
    ));

    for (x, expected) in [(0, 2), (10, 40), (19, 75)] {
        near(
            level(&raster, x, 20),
            expected,
            &format!("76.5 × t of the page survives a mask of 1 − 0.3 t at x = {x}"),
        );
    }
    near(
        level(&raster, 30, 20),
        255,
        "and /BC [0] is a whole unit of ink, which masks everything outside the box",
    );
}

/// A blend mode inside a `DeviceCMYK` mask group is reported, because one channel cannot hold
/// four separable ones.
///
/// The condition `note_blended_luminosity` derives from §11.3.5.2, on the fixture that makes
/// it fire: two overlapping marks inside the group, the upper one `/Multiply`. **No corpus
/// document states one** — the whole population was checked when the report was written — so
/// this is the only thing in the tree that can show the sentence is reachable, which is
/// exactly why it is here (trap 11's other edge: a report nothing exercises is a report
/// nobody has read).
///
/// The same group blending in `/DeviceGray` is *not* reported, and that is the load-bearing
/// half: there the channel is the group's one component in additive form, so a separable
/// blend function applied to it is the clause's own arithmetic.
#[test]
fn a_blend_mode_in_a_four_component_mask_group_is_reported_and_in_a_one_component_one_is_not() {
    let group = "q 0 0 0 0.2 k 0 0 20 40 re f /GB gs 0 0 0 0.8 k 0 0 20 20 re f Q";
    let grey_group = "q 0.8 g 0 0 20 40 re f /GB gs 0.2 g 0 0 20 20 re f Q";
    let multiply = b"7 0 obj\n<< /Type /ExtGState /BM /Multiply >>\nendobj\n".to_vec();

    let cmyk = reports(page_with_group_resources(
        "/DeviceCMYK",
        "/SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [0 0 0 1] >>",
        "q /GS gs 0 g 0 0 40 40 re f Q",
        "<< /ExtGState << /GB 7 0 R >> >>",
        group,
        std::slice::from_ref(&multiply),
    ));
    assert!(
        cmyk.iter()
            .any(|detail| detail.contains("blends in a space of four components")),
        "a /Multiply inside a /DeviceCMYK mask group is a departure, and got {cmyk:?}"
    );

    let grey = reports(page_with_group_resources(
        "/DeviceGray",
        "/SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [0] >>",
        "q /GS gs 0 g 0 0 40 40 re f Q",
        "<< /ExtGState << /GB 7 0 R >> >>",
        grey_group,
        &[multiply],
    ));
    assert!(
        !grey
            .iter()
            .any(|detail| detail.contains("blends in a space of")),
        "one component in additive form blends exactly, and got {grey:?}"
    );
}

/// Every departure a fixture's page one reports, as the sentences they are worded in.
fn reports(bytes: Vec<u8>) -> Vec<String> {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page)
        .unsupported
        .iter()
        .map(|item| format!("{item:?}"))
        .collect()
}
