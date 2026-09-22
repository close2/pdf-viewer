//! ISO 32000-2 §8.6.7's overprint parameters and §11.7.4's special blend mode.
//!
//! # What these fixtures are measuring
//!
//! Table 146 has one cell whose value is not the source colour and that this renderer can
//! reach: its first row's `OP true, OPM 1`, which applies to "DeviceCMYK , specified directly,
//! not in a sampled image" affecting a "C, M, Y, or K" component **of the group space**. The
//! group space is `DeviceCMYK` only inside a page or isolated group that says so, which is the
//! four-component pair `pdf_render::blending` puts back together — so the whole of what is
//! owed lives on such a page, and every fixture below states one.
//!
//! **Which spaces are in that row is §11.7.4.3's NOTE 2's answer, not the row's own wording.**
//! A `Separation` or `DeviceN` that reverts has its alternate for a current colour space, so
//! one over `DeviceCMYK` is in the first row on the four components the alternate receives;
//! §8.6.7's EXAMPLE calls such an `scn` equivalent to the matching `k`, and the last two
//! fixtures are that EXAMPLE and §10.8.2's (ADR 1241).
//!
//! The expected values are derived from the clause rather than measured from this tree. Under
//! `OP true` and `OPM 1` the blend function is "the source component C s for any process
//! ( DeviceCMYK ) colour component whose (subtractive) colour value is nonzero; otherwise it
//! shall be the backdrop component 𝐶𝑏" (§11.7.4.3), and both fixtures paint opaquely, where
//! §11.3.6's compositing formula reduces to `B(Cb, Cs)` exactly. So the four components after
//! the second fill are a colour that can be *named*, and each assertion below is an equality
//! with a page that paints that colour directly in the same space through the same press.
//! Nothing here depends on what the ink cube converts those components to.
//!
//! ADR 1157 has the reading, ADR 1158 what it costs, and ADR 0028 is what it supersedes.

#![expect(
    clippy::doc_markdown,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "a test asserts, a fixture that will not load is a failed test, and the \
              prose here is largely quotation, which may not gain backticks"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;

/// A one-page fixture whose page group composites in `DeviceCMYK`.
///
/// `resources` carries the page's `/ExtGState` entries and `content` its stream. The page is
/// 40 units square and rasterises one pixel per unit.
fn cmyk_page(resources: &str, content: &str) -> Vec<u8> {
    cmyk_page_with(resources, content, "")
}

/// [`cmyk_page`] with further objects appended, numbered from 5.
///
/// `extra` is the literal body of those objects, each ending in `endobj\n`, so that a fixture
/// can name a colour space array and a tint transform by object number. A type 4 function is
/// a stream and there is no other place to put one.
fn cmyk_page_with(resources: &str, content: &str, extra: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
         /Group << /S /Transparency /CS /DeviceCMYK >> \
         /Resources << {resources} >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n{extra}",
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

/// One interpretation of a fixture, which must be complete.
fn interpret(bytes: Vec<u8>) -> pdf_model::Interpretation {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the fixture should draw completely: {:?}",
        interpretation.unsupported
    );
    interpretation
}

/// The centre pixel of a fixture, rendered by the backend `CLAUDE.md` keeps as the oracle.
fn centre(interpretation: &pdf_model::Interpretation) -> (u8, u8, u8) {
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("a 40x40 target");
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the fixture rasterises");
    let at = ((20 * raster.width) + 20) as usize * 4;
    (raster.data[at], raster.data[at + 1], raster.data[at + 2])
}

/// Paints `backdrop`, then `source` under the graphics state `gs` names.
fn over(gs: &str, backdrop: &str, source: &str) -> String {
    format!("{backdrop} k 0 0 40 40 re f {gs} {source} k 0 0 40 40 re f")
}

/// One pixel of a fixture, rendered by the backend `CLAUDE.md` keeps as the oracle.
fn pixel(interpretation: &pdf_model::Interpretation, x: u32, y: u32) -> (u8, u8, u8) {
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("a 40x40 target");
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the fixture rasterises");
    let at = ((y * raster.width) + x) as usize * 4;
    (raster.data[at], raster.data[at + 1], raster.data[at + 2])
}

/// The one cell of Table 146 this renderer can reach, at the pixel.
///
/// Backdrop `0 0 0 1 k`, source `0.9 0 0 0 k`, `/OP true /op true /OPM 1`. §11.7.4.3's first
/// bullet decides each of the group space's four components on that component's own value:
///
/// - cyan `0.9` is nonzero, so `B = C_s = 0.9`;
/// - magenta, yellow and black are `0.0`, so `B = C_b`, which the backdrop set to `0`, `0`
///   and `1`.
///
/// So the page is `0.9 0 0 1 k`, and that is what the assertion names. The second assertion is
/// what makes the first discriminate: without the overprint the same pair of fills is the
/// source colour in every component, which is the `OP false` column and a different picture.
#[test]
fn a_zero_component_under_overprint_mode_one_keeps_the_backdrops() {
    let overprinting = interpret(cmyk_page(
        "/ExtGState << /GS << /OP true /op true /OPM 1 >> >>",
        &over("/GS gs", "0 0 0 1", "0.9 0 0 0"),
    ));
    let named = interpret(cmyk_page("", "0.9 0 0 1 k 0 0 40 40 re f"));
    let erasing = interpret(cmyk_page("", &over("", "0 0 0 1", "0.9 0 0 0")));

    assert_eq!(
        centre(&overprinting),
        centre(&named),
        "§11.7.4.3's first bullet makes the page 0.9 0 0 1 in the group's own components"
    );
    assert_ne!(
        centre(&overprinting),
        centre(&erasing),
        "which is not the colour the same two fills paint with overprinting off"
    );
}

/// Overprint mode 0 is the `OP true, OPM 0` column, which Table 146 gives as `C_s`.
///
/// §8.6.7 states the same thing in words: the overprint mode "shall affect the interpretation
/// of a tint value of 0.0 for a colour component in a DeviceCMYK colour space when
/// overprinting is enabled", and mode 0 is the one where "each source colour component value
/// replaces the value previously painted for the corresponding device colourant, no matter
/// what the new value is".
#[test]
fn overprint_mode_zero_erases_the_components_the_source_does_not_name() {
    let mode_zero = interpret(cmyk_page(
        "/ExtGState << /GS << /OP true /op true /OPM 0 >> >>",
        &over("/GS gs", "0 0 0 1", "0.9 0 0 0"),
    ));
    let plain = interpret(cmyk_page("", &over("", "0 0 0 1", "0.9 0 0 0")));
    assert_eq!(centre(&mode_zero), centre(&plain));
    assert!(
        !mode_zero.display_list.overprints(),
        "no command composites under the special mode"
    );
}

/// The mode "shall have an effect only when the overprint parameter is true" (§8.6.7).
#[test]
fn overprint_mode_one_alone_changes_nothing() {
    let mode_only = interpret(cmyk_page(
        "/ExtGState << /GS << /OPM 1 >> >>",
        &over("/GS gs", "0 0 0 1", "0.9 0 0 0"),
    ));
    let plain = interpret(cmyk_page("", &over("", "0 0 0 1", "0.9 0 0 0")));
    assert_eq!(centre(&mode_only), centre(&plain));
    assert!(!mode_only.display_list.overprints());
}

/// Table 57's two parameters are two: `/op` false leaves a *fill* erasing while `/OP` stands.
///
/// Table 57 (§8.4.5) gives `/OP` and `/op` a precedence rather than one meaning each: an `/OP`
/// entry sets both parameters unless the same dictionary also carries `/op`, in which case
/// `/OP` sets only the stroking one, and an absent `/op` takes `/OP`'s value. So
/// `<< /OP true /op false >>` is stroking-only, and the fill below is the `OP false`
/// column. The fixture fills rather than strokes, which is what makes the two distinguishable
/// at all.
#[test]
fn the_non_stroking_parameter_is_its_own() {
    let stroking_only = interpret(cmyk_page(
        "/ExtGState << /GS << /OP true /op false /OPM 1 >> >>",
        &over("/GS gs", "0 0 0 1", "0.9 0 0 0"),
    ));
    let plain = interpret(cmyk_page("", &over("", "0 0 0 1", "0.9 0 0 0")));
    assert_eq!(centre(&stroking_only), centre(&plain));

    // And `/OP` on its own sets both, so the same fill overprints.
    let both = interpret(cmyk_page(
        "/ExtGState << /GS << /OP true /OPM 1 >> >>",
        &over("/GS gs", "0 0 0 1", "0.9 0 0 0"),
    ));
    let named = interpret(cmyk_page("", "0.9 0 0 1 k 0 0 40 40 re f"));
    assert_eq!(centre(&both), centre(&named));
}

/// A source with no zero component is the source colour in every one, which is Normal.
///
/// The bullet's value differs from `C_s` only "for any process ( DeviceCMYK ) colour component
/// whose (subtractive) colour value is nonzero; otherwise it shall be the backdrop component",
/// so a colour with four nonzero components asks for nothing and the list says so — which is
/// what keeps the two refusing backends from being handed a page that did not need them.
#[test]
fn a_colour_with_no_zero_component_asks_for_nothing() {
    let dense = interpret(cmyk_page(
        "/ExtGState << /GS << /OP true /op true /OPM 1 >> >>",
        &over("/GS gs", "0 0 0 1", "0.9 0.1 0.2 0.3"),
    ));
    let plain = interpret(cmyk_page("", &over("", "0 0 0 1", "0.9 0.1 0.2 0.3")));
    assert_eq!(centre(&dense), centre(&plain));
    assert!(!dense.display_list.overprints());
}

/// The verdict is a command under the mode, not a graphics state that would have chosen one.
///
/// §8.6.7 gives stroking and non-stroking operations an overprint parameter each, so
/// `Interpreter::overprint_blend` is asked twice per painting operator — before the operator
/// says which of the two parts marks the page. Here the non-stroking colour `0.9 0 0 0` has
/// three zero tints and the stroking colour `0.9 0.1 0.2 0.3` has none, and the operator is
/// `S`: the mode is chosen for a fill that never happens and no command on the page carries
/// it. Two backends refuse a whole page on this flag, so a page in this shape would lose its
/// backend for a mark that is not on it (ADR 1181).
///
/// The second half is what makes the first discriminate: the same two colours under `f`, where
/// the fill *is* painted, must carry the verdict.
#[test]
fn a_mode_chosen_for_a_part_that_never_paints_is_not_the_verdict() {
    let resources = "/ExtGState << /GS << /OP true /op true /OPM 1 >> >>";
    let colours = "/GS gs 0.9 0 0 0 k 0.9 0.1 0.2 0.3 K";
    let stroked = interpret(cmyk_page(
        resources,
        &format!("0 0 0 1 k 0 0 40 40 re f {colours} 4 w 10 10 20 20 re S"),
    ));
    assert!(
        !stroked.display_list.overprints(),
        "no command carries the mode, so the page does not"
    );
    let filled = interpret(cmyk_page(
        resources,
        &format!("0 0 0 1 k 0 0 40 40 re f {colours} 10 10 20 20 re f"),
    ));
    assert!(
        filled.display_list.overprints(),
        "the same colours under an operator that fills do carry it"
    );
}

/// §8.6.7 excludes images and shadings, and Table 146's first row excludes a sampled image.
///
/// The clause's sentence is that non-zero overprint mode shall not apply to the painting of
/// images or shadings, and Table 146's own first row says "not in a sampled image".
///
/// A shading pattern set as the fill colour is the reachable half of that here, since the
/// interpreter never puts a colour's tints on a patterned paint.
#[test]
fn a_shading_is_not_the_current_colour() {
    let resources = "/ExtGState << /GS << /OP true /op true /OPM 1 >> >> \
                     /Pattern << /P << /PatternType 2 /Shading << /ShadingType 2 \
                     /ColorSpace /DeviceCMYK /Coords [0 0 40 0] /Function << /FunctionType 2 \
                     /Domain [0 1] /C0 [0.9 0 0 0] /C1 [0.9 0 0 0] /N 1 >> >> >> >>";
    let content = "0 0 0 1 k 0 0 40 40 re f /GS gs /Pattern cs /P scn 0 0 40 40 re f";
    let shaded = interpret(cmyk_page(resources, content));
    assert!(
        !shaded.display_list.overprints(),
        "a shading's colours are not the current colour in the graphics state"
    );
}

/// The parameters are graphics state parameters, so `q`/`Q` restores them (§8.4).
#[test]
fn the_overprint_parameters_are_saved_and_restored() {
    let restored = interpret(cmyk_page(
        "/ExtGState << /GS << /OP true /op true /OPM 1 >> >>",
        "0 0 0 1 k 0 0 40 40 re f q /GS gs Q 0.9 0 0 0 k 0 0 40 40 re f",
    ));
    let plain = interpret(cmyk_page("", &over("", "0 0 0 1", "0.9 0 0 0")));
    assert_eq!(centre(&restored), centre(&plain));
    assert!(!restored.display_list.overprints());
}

/// Outside a group compositing in four components, §8.6.7's own condition keeps it off.
///
/// > It also shall not apply if the native colour space of the output device does not include
/// > CMYK device colourants; in that case, source colours shall be converted to the device's
/// > native colour space, and all components participate in the conversion, whatever their
/// > values.
///
/// This device's three additive colourants are that case, and §11.7.4.5's NOTE 1 says the
/// transparent model differs from the opaque one only "within a transparency group (including
/// the page group, if its colour space is different from the native colour space of the output
/// device)" — so a page with no such group is the opaque reading, unchanged. The same fixture
/// as the first test, less its `/Group`.
#[test]
fn a_page_on_the_devices_own_components_is_unchanged() {
    let body = |resources: &str, content: &str| {
        let body = format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
             /Resources << {resources} >> /Contents 4 0 R >>\nendobj\n\
             4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
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
    };
    let overprinting = interpret(body(
        "/ExtGState << /GS << /OP true /op true /OPM 1 >> >>",
        &over("/GS gs", "0 0 0 1", "0.9 0 0 0"),
    ));
    let plain = interpret(body("", &over("", "0 0 0 1", "0.9 0 0 0")));
    assert_eq!(centre(&overprinting), centre(&plain));
    assert!(!overprinting.display_list.overprints());
}

/// A colour whose only zero tint is black keeps the page's four-component pair together.
///
/// §11.4.7's page group composites in four components and this renderer carries them as two
/// rasters of three channels — cyan, magenta and yellow in one and the black component in all
/// three of the other. §11.7.4.3's first bullet decides each of the *four*, so a source of
/// `0.9 0.1 0.2 0` leaves every channel of the black raster to the backdrop and none of the
/// chromatic one. Asked of a half's own three tints the question answers differently in the two
/// runs, `DisplayList::geometry_digest` refuses a pair whose commands differ in structure, and
/// the page falls back to the device's three components with nothing said. Asked of the four
/// the clause decides, it answers once. ADR 1169.
///
/// The expected colour is the clause's: cyan, magenta and yellow are nonzero and take the
/// source, black is zero and takes the backdrop's `1`.
#[test]
fn a_source_whose_only_zero_tint_is_black_keeps_the_pages_pair() {
    let overprinting = interpret(cmyk_page(
        "/ExtGState << /GS << /OP true /op true /OPM 1 >> >>",
        &over("/GS gs", "0 0 0 1", "0.9 0.1 0.2 0"),
    ));
    let named = interpret(cmyk_page("", "0.9 0.1 0.2 1 k 0 0 40 40 re f"));
    let erasing = interpret(cmyk_page("", &over("", "0 0 0 1", "0.9 0.1 0.2 0")));

    assert!(
        overprinting.display_list.blending().is_some(),
        "the page still composites in the four components its group names"
    );
    assert_eq!(
        centre(&overprinting),
        centre(&named),
        "§11.7.4.3's first bullet leaves the black component to the backdrop"
    );
    assert_ne!(centre(&overprinting), centre(&erasing));
}

/// §11.7.4.3's last paragraph: an implicit non-isolated, non-knockout group under a blend mode.
///
/// > If the current blend mode is any mode other than Normal when invoking this special
/// > overprinting blend mode, the object being painted shall be implicitly treated as if it
/// > were defined in a non-isolated, non-knockout transparency group, and painted using the
/// > this special blend mode. The group's results shall then be painted using the current blend
/// > mode in the graphics state.
///
/// The fixture paints `0.9 0 0 0` over an opaque `0.2 0 0 0.5` backdrop with `/OP true /op true
/// /OPM 1` under `/BM /Multiply`, at an alpha of 1.0 and full coverage. The expected value is
/// derived and not measured:
///
/// - Inside the group the object composites onto the group's backdrop — the page, since the
///   group is non-isolated — under the special mode, and §11.3.6's formula at `αb = 1` and
///   `αs = 1` is `B(C_b, C_s)` exactly. Component by component that is `0.9` (cyan is nonzero,
///   so the source), `0` and `0` (magenta and yellow are zero, so the backdrop's) and `0.5`
///   (black is zero, so the backdrop's): the colour `0.9 0 0 0.5`.
/// - §11.4.4's backdrop removal divides by Table 140's group alpha and the initial backdrop's,
///   both 1.0 here, so the factor `α0 ÷ αgn − α0` is zero and the group's colour is that same
///   value.
/// - The group's result is then painted under Multiply, at an alpha of 1.0, onto the same
///   backdrop — which is §11.3.3 with the Multiply blend function over `0.9 0 0 0.5`, and that
///   is what a page painting that colour under Multiply does with one fill.
///
/// So the assertion names a page rather than a number, and it holds whatever the arithmetic of
/// Multiply over subtractive components turns out to be — the same function is applied to the
/// same two colours on both sides. The second assertion is what makes it discriminate: without
/// the special mode the same fill under Multiply carries `0.9 0 0 0`, whose black component is
/// the source's and not the backdrop's.
#[test]
fn a_non_normal_blend_mode_puts_the_object_in_an_implicit_group() {
    let multiplying = interpret(cmyk_page(
        "/ExtGState << /GS << /OP true /op true /OPM 1 /BM /Multiply >> >>",
        &over("/GS gs", "0.2 0 0 0.5", "0.9 0 0 0"),
    ));
    let group_result = interpret(cmyk_page(
        "/ExtGState << /GS << /BM /Multiply >> >>",
        &over("/GS gs", "0.2 0 0 0.5", "0.9 0 0 0.5"),
    ));
    let without_the_mode = interpret(cmyk_page(
        "/ExtGState << /GS << /BM /Multiply >> >>",
        &over("/GS gs", "0.2 0 0 0.5", "0.9 0 0 0"),
    ));

    assert_eq!(
        centre(&multiplying),
        centre(&group_result),
        "the group's result is B(C_b, C_s) and it is painted under the document's own mode"
    );
    assert_ne!(
        centre(&multiplying),
        centre(&without_the_mode),
        "which is not the same page as the source colour painted under that mode"
    );
}

/// §11.7.4.4's first bullet, where its group is not the two commands as they stand.
///
/// > If overprinting is enabled (the overprint parameters for both stroking and non-stroking
/// > operations in the graphics state are true ) and the current stroking and nonstroking alpha
/// > constants are equal, a non-isolated, non-knockout transparency group shall be established.
/// > Within the group, the fill and stroke shall be performed with an alpha value of 1.0 but
/// > with the special overprinting blend mode described in 11.7.4.3 , ' Compatibility with
/// > opaque overprinting ' . The group results shall then be composited with the backdrop,
/// > using the originally specified alpha and blend mode.
///
/// The fixture is a `B` on a 20-unit square stroked four units wide, filled `0.9 0 0 0` and
/// stroked `0 0.7 0 0`, over an opaque `0 0 0 0.5` backdrop, with both alpha constants 0.5.
/// Two pixels are asked about and both expected values are derived:
///
/// - Inside the group the parts paint at an alpha of 1.0 and full coverage, so each is
///   `B(C_b, C_s)` exactly. The fill leaves magenta, yellow and black to the backdrop and takes
///   cyan from the source: `0.9 0 0 0.5`. The stroke paints over that and leaves cyan, yellow
///   and black to *it* while taking magenta: `0.9 0.7 0 0.5`.
/// - The group is non-isolated with both alphas 1.0, so §11.4.4's removal factor is zero again
///   and the group's colour is those two values.
/// - The group composites once "using the originally specified alpha and blend mode" — 0.5 and
///   Normal — which is what a page painting each of those colours at `/ca 0.5` does.
///
/// The band and the centre must also differ, or the fixture would be a stroke that never drew.
#[test]
fn a_pair_under_a_constant_alpha_gets_the_first_bullets_group() {
    let stroked = interpret(cmyk_page(
        "/ExtGState << /GS << /OP true /op true /OPM 1 /ca 0.5 /CA 0.5 >> >>",
        "0 0 0 0.5 k 0 0 40 40 re f /GS gs 0.9 0 0 0 k 0 0.7 0 0 K 4 w 10 10 20 20 re B",
    ));
    let half = |colour: &str| {
        interpret(cmyk_page(
            "/ExtGState << /GS << /ca 0.5 >> >>",
            &format!("0 0 0 0.5 k 0 0 40 40 re f /GS gs {colour} k 0 0 40 40 re f"),
        ))
    };
    let filled = half("0.9 0 0 0.5");
    let banded = half("0.9 0.7 0 0.5");

    assert_eq!(
        pixel(&stroked, 20, 20),
        pixel(&filled, 20, 20),
        "the fill's part of the group is B(C_b, C_s), composited once at the stated alpha"
    );
    assert_eq!(
        pixel(&stroked, 11, 20),
        pixel(&banded, 11, 20),
        "and the stroke's part is B over what the fill left, composited in the same one step"
    );
    assert_ne!(pixel(&stroked, 11, 20), pixel(&stroked, 20, 20));
}

/// §10.8.2's own example: cyan then yellow through two `Separation` spaces is green.
///
/// > For example, if two separate painting operations are performed on the same area of the
/// > page with the overprinting controls turned on, one using a Cyan Separation colour space
/// > and the second using a Yellow Separation colour space, then that area will appear green
/// > when produced using separations, overprinting of the cyan and yellow inks, whereas
/// > displaying it on the screen, ignoring the overprint controls, will generally produce
/// > yellow, the last colour painted.
///
/// The clause's second half describes a processor that *ignores* the overprint controls; this
/// one does not, because §11.7.4.3's special blend mode is built. So inside a page group
/// compositing in `DeviceCMYK` the area is green — `1 0 1 0` in the group's four components,
/// cyan kept because yellow's other three tints are zero — and that is a colour the same page
/// can name with `k`. §11.7.4.3's NOTE 2 is what carries the two `Separation` spaces into the
/// first bullet: each reverts to `DeviceCMYK`, so the current colour space *is* `DeviceCMYK`
/// and the zero test is asked of the four components the alternate receives. ADR 1241.
///
/// The second assertion is what makes the first discriminate: with the overprint controls off
/// the same two fills are the clause's other outcome, yellow.
#[test]
fn two_separation_spaces_overprinting_are_the_colour_the_alternate_names() {
    let spaces = "/ColorSpace << \
                  /Cy [/Separation /Cyan /DeviceCMYK \
                  << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [1 0 0 0] /N 1 >>] \
                  /Ye [/Separation /Yellow /DeviceCMYK \
                  << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 0 1 0] /N 1 >>] >>";
    let resources = format!("/ExtGState << /GS << /OP true /op true /OPM 1 >> >> {spaces}");
    let fills = "/Cy cs 1 scn 0 0 40 40 re f /Ye cs 1 scn 0 0 40 40 re f";

    let separations = interpret(cmyk_page(&resources, &format!("/GS gs {fills}")));
    let green = interpret(cmyk_page("", "1 0 1 0 k 0 0 40 40 re f"));
    assert!(
        separations.display_list.overprints(),
        "a Separation reverting to DeviceCMYK is the first bullet's current colour space"
    );
    assert_eq!(
        centre(&separations),
        centre(&green),
        "cyan overprinted by yellow is 1 0 1 0 in the group's own components"
    );

    let ignored = interpret(cmyk_page(&resources, fills));
    let yellow = interpret(cmyk_page("", "0 0 1 0 k 0 0 40 40 re f"));
    assert_eq!(
        centre(&ignored),
        centre(&yellow),
        "with the controls off the clause's other outcome is the last colour painted"
    );
    assert_ne!(centre(&separations), centre(&ignored));
}

/// §8.6.7's EXAMPLE, which is an equivalence between a `k` and a `DeviceN` `scn`.
///
/// > EXAMPLE If the overprint parameter is true and the overprint mode is 1, the operation 0.2
/// > 0.3 0.0 1.0 k is equivalent to 0.2 0.3 1.0 scn in the colour space shown in this example.
///
/// The space the EXAMPLE shows is `[/DeviceN [/Cyan /Magenta /Black] /DeviceCMYK 6 0 R]` with
/// the tint transform `{ 0 exch }`, which inserts the zero yellow the `k` operator wrote. Two
/// operators a clause calls equivalent may not take different blend functions, so the `scn`
/// takes the first bullet exactly as the `k` does — which is only true because §11.7.4.3's
/// NOTE 2 makes the `DeviceN` space's alternate the current colour space (ADR 1241).
///
/// The backdrop states a yellow of `0.6` so that the component both operators leave alone is
/// visible, and the expected colour is the clause's: `0.2 0.3 0.6 1.0`, yellow from the
/// backdrop and the other three from the source.
#[test]
fn the_overprint_clauses_own_example_is_an_equivalence() {
    let extra = "5 0 obj\n[/DeviceN [/Cyan /Magenta /Black] /DeviceCMYK 6 0 R]\nendobj\n\
                 6 0 obj\n<< /FunctionType 4 /Domain [0 1 0 1 0 1] \
                 /Range [0 1 0 1 0 1 0 1] /Length 10 >>\nstream\n{ 0 exch }\nendstream\nendobj\n";
    let resources =
        "/ExtGState << /GS << /OP true /op true /OPM 1 >> >> /ColorSpace << /DN 5 0 R >>";
    let backdrop = "0 0 0.6 0 k 0 0 40 40 re f /GS gs";

    let through_devicen = interpret(cmyk_page_with(
        resources,
        &format!("{backdrop} /DN cs 0.2 0.3 1.0 scn 0 0 40 40 re f"),
        extra,
    ));
    let through_k = interpret(cmyk_page(
        resources,
        &format!("{backdrop} 0.2 0.3 0.0 1.0 k 0 0 40 40 re f"),
    ));
    let named = interpret(cmyk_page("", "0.2 0.3 0.6 1.0 k 0 0 40 40 re f"));

    assert_eq!(
        centre(&through_devicen),
        centre(&named),
        "the EXAMPLE's scn leaves yellow to the backdrop, as its k does"
    );
    assert_eq!(
        centre(&through_devicen),
        centre(&through_k),
        "which is the equivalence §8.6.7's EXAMPLE states"
    );

    let erasing = interpret(cmyk_page_with(
        resources,
        "0 0 0.6 0 k 0 0 40 40 re f /DN cs 0.2 0.3 1.0 scn 0 0 40 40 re f",
        extra,
    ));
    assert_ne!(
        centre(&through_devicen),
        centre(&erasing),
        "and not the picture the same fill paints with the controls off"
    );
}

/// §11.7.4.4's first-bullet group inside a knockout group of the stated isolation.
///
/// The page paints an opaque `0 0 0 0.5` backdrop and invokes a form whose `/Group` is a
/// knockout group. Inside it a translucent fill is covered by the `B` of
/// [`a_pair_under_a_constant_alpha_gets_the_first_bullets_group`], so §11.4.6's rule has
/// something to knock out: with one element a knockout group and §11.4.4's group are the same
/// picture.
fn pair_in_a_knockout_group(isolated: bool) -> Vec<u8> {
    const FORM: &str = "/GH gs 0 0 0.6 0 k 0 0 40 40 re f \
                        /GS gs 0.9 0 0 0 k 0 0.7 0 0 K 4 w 10 10 20 20 re B";
    let form = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 40 40] \
         /Group << /S /Transparency /K true /I {} >> \
         /Resources << /ExtGState << /GS << /OP true /op true /OPM 1 /ca 0.5 /CA 0.5 >> \
         /GH << /ca 0.5 >> >> >> /Length {} >>\nstream\n{FORM}\nendstream\nendobj\n",
        if isolated { "true" } else { "false" },
        FORM.len() + 1
    );
    cmyk_page_with(
        "/XObject << /Fm 5 0 R >>",
        "0 0 0 0.5 k 0 0 40 40 re f /Fm Do",
        &form,
    )
}

/// §11.7.4.4's first bullet is built inside a knockout group of either isolation, and the two
/// draw the two backdrops §11.4.6 names.
///
/// > A knockout group may be isolated or non-isolated; that is, isolated and knockout are
/// > independent attributes. A nonisolated knockout group composites its topmost enclosing
/// > element with the group's backdrop. An isolated knockout group composites the element with
/// > a transparent backdrop.
///
/// The bullet's group is "a non-isolated, non-knockout transparency group", and §11.4.6's
/// NOTE 6 says what that means for a direct element of a knockout group: "the initial backdrop
/// of the inner group is the same as that of the outer group". So the pair's own backdrop is
/// the enclosing group's initial one, and the two isolations give two derivable colours:
///
/// - **Non-isolated**: the initial backdrop is the page's `0 0 0 0.5`, so the fill's part is
///   §11.7.4.3's `B(C_b, C_s)` component by component — cyan from the source, the other three
///   from the backdrop — which is `0.9 0 0 0.5`, composited once at the stated alpha of 0.5.
/// - **Isolated**: the initial backdrop is transparent, where §11.3.6 leaves the blend function
///   nothing to do — "[a]n alpha value of αs = 0.0 or αb = 0.0 results in no blend mode effect"
///   — so the fill's part is the source colour `0.9 0 0 0`, and the group is composited over
///   the page at 0.5.
///
/// Each assertion is an equality with a page that paints that colour directly at `/ca 0.5`
/// over the same backdrop, so nothing here depends on what the ink cube makes of the four
/// components. ADR 1265.
#[test]
fn a_pairs_group_takes_the_backdrop_its_enclosing_knockout_group_has() {
    let half = |colour: &str| {
        interpret(cmyk_page(
            "/ExtGState << /GS << /ca 0.5 >> >>",
            &format!("0 0 0 0.5 k 0 0 40 40 re f /GS gs {colour} k 0 0 40 40 re f"),
        ))
    };
    let non_isolated = interpret(pair_in_a_knockout_group(false));
    let isolated = interpret(pair_in_a_knockout_group(true));

    assert_eq!(
        pixel(&non_isolated, 20, 20),
        pixel(&half("0.9 0 0 0.5"), 20, 20),
        "a non-isolated knockout group composites its element with the group's backdrop, so \
         the overprint keeps the backdrop's other three components"
    );
    assert_eq!(
        pixel(&isolated, 20, 20),
        pixel(&half("0.9 0 0 0"), 20, 20),
        "an isolated knockout group composites the element with a transparent backdrop, where \
         the special blend mode has no backdrop to keep"
    );
    assert_ne!(
        pixel(&non_isolated, 20, 20),
        pixel(&isolated, 20, 20),
        "which are two pictures, or the fixture would be about neither"
    );
}
