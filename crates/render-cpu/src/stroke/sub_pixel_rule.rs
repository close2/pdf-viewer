//! A stroke thinner than one device pixel, drawn at the coverage its own area implies — ISO 32000-2
//! §10.7.4 — by the exact construction where it applies (ADR 0226) and at one pixel with the width
//! carried in the alpha where it does not (ADR 0268).

use pdf_render::Path;

use crate::compose::carries_coverage_as_alpha;
use crate::stroke::SolidOutline;
use crate::surface::Placed;
use crate::{convert, scan};

/// Draws a stroke the raster cannot state the coverage of, ISO 32000-2 §10.7.4, and reports
/// whether it did.
///
/// # What is owed, and how far down
///
/// The clause's stated purpose covers strokes as well as fills — "\[t]his rule applies both to fill
/// operations and to strokes with non-zero width":
///
/// > This ensures that no shape ever disappears as a result of unfavourable placement relative to
/// > the device pixel grid, as might happen with other possible scan conversion rules.
///
/// A coverage that rounds to nothing is that disappearance reached by another route, so a mark too
/// thin for the raster's depth is restated as one it can hold. **What is owed is bounded by what
/// the device can express, and this backend's device states the winding integral over a pixel
/// exactly** (`render_cpu::area`, ADR 1082), down to the eight bits the raster itself carries. So
/// the boundary is [`pdf_render::unmeasurable_width`] — one *level* of a device pixel — and above
/// it nothing is widened. What is above it is drawn as the shape the document states: by the exact
/// construction below where that applies, and otherwise by returning `false` so that the stroke's
/// own outline reaches [`draw_stroked_outline`], which is the shape `render_raster` builds from the
/// same stated width (ADR 1102).
///
/// # Two constructions, exact first
///
/// [`draw_rule_as_bands`] is ADR 0226's and is exact: the outline of a straight axis-aligned rule
/// is the rectangle its width and length state, and [`pdf_render::sub_pixel_bands`] draws that at
/// the coverage its own area implies, including the part that is off the raster, which is then
/// clipped away rather than folded back in. It is tried for every rule thinner than a device pixel
/// rather than only for one under a level, because a closed form costs neither a stroker nor a
/// path conversion and lands on the same coverage the general converter would compute.
///
/// [`draw_rule_at_one_pixel`] is ADR 0268's, and it is the last resort: **the same path stroked one
/// device pixel wide, with the width it gave up carried in the paint's alpha.** That conserves the
/// ink exactly at every angle, because widening by a factor and dividing the alpha by it cancel.
/// What it costs is where the ink lands — a band one device pixel wide spreads it over a pixel
/// where a band of the true width spreads it over `w`, at the raster's edge it loses what falls
/// outside, and a coverage carried as alpha no longer composes like coverage, so two draws of one
/// rule come out heavier than the rule. Under [`pdf_render::unmeasurable_width`] that price buys
/// the mark's existence; above it, it buys nothing.
///
/// Nothing is snapped by either. A 0.1-unit rule draws 0.1 of a row at the fractional position the
/// document put it, which is the sentence `doc/todo/_scan-conversion.md` demands of anything that
/// touches the pixel grid; what §10.7.5 would do under `/SA` is multiply the *ink* by
/// `one_pixel / w` rather than divide the alpha by it.
///
/// # What it still declines
///
/// The whole rule is conditioned on [`carries_coverage_as_alpha`], which is where its warrant
/// lives. Beyond that, only a stroke the stroker itself refuses — and the exact construction
/// declines a great deal more, each case argued in `pdf_render::sub_pixel`'s module comment.
/// A dashed rule is taken: the dashes are dispensed by `tiny_skia::Path::dash`, which is the
/// same function `stroke_path` would have called, rather than by a second implementation of
/// §8.4.3.6 in this file.
///
/// [`draw_stroked_outline`]: super::draw_stroked_outline
pub(crate) fn draw_sub_pixel_rule(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    geometry: (&Path, &tiny_skia::Path),
    (style, cap): (&tiny_skia::Stroke, pdf_render::LineCap),
    (placed, outline): (Placed, &SolidOutline<'_>),
    brush: &tiny_skia::Paint<'_>,
    clip: scan::Clip<'_>,
) -> bool {
    let at = placed.at;
    let Some(one_pixel) = pdf_render::thinnest_line(at) else {
        return false;
    };
    if !at_or_under_the_quantum(style.width, one_pixel)
        || !carries_coverage_as_alpha(brush.anti_alias, brush.blend_mode)
    {
        return false;
    }
    // The resolution scale bounds how finely a curve is measured while it is offset, and is what
    // `stroke_path` would have used had the hairline test not fired.
    let scale = outline.scale;
    // The cheap questions come first: a path of straight axis-aligned rules is one walk over the
    // commands, and only such a path can reach the exact construction at all.
    // **The exact construction stays strictly under the quantum.** A rule *thinner* than a device
    // pixel lies inside one pixel line and `pdf_render::sub_pixel_bands` draws that line at the
    // coverage its area implies; a rule that is exactly one pixel wide has no such line — it
    // spans two of them at a fractional offset — and snapping it onto one would be §10.7.5's
    // automatic stroke adjustment performed without `/SA`, which ADR 0208 forbids and
    // `render-cpu/tests/zero_area_fill.rs` pins by making a `0 w` stroke and a zero-height fill
    // land in *different* places. The general construction below moves nothing.
    if style.width < one_pixel
        && style.dash.is_none()
        && at.preserves_axes()
        && pdf_render::only_flat_subpaths(geometry.0)
        && draw_rule_as_bands(pixmap, outline, placed, brush, clip)
    {
        return true;
    }
    // **The widened band is the last resort and not the general answer, which is what ADR 1102
    // moved.** A mark whose coverage the raster can state is drawn as the shape the document
    // states — by the exact construction above where it applies, and by the stroke's own outline
    // filled through `render_cpu::area` below where it does not (ADR 1082). Only under
    // `pdf_render::unmeasurable_width` is there no coverage left to state, and there the alpha is
    // the one place the mark can go.
    if !pdf_render::unmeasurable_width(at).is_some_and(|floor| style.width <= floor) {
        return false;
    }
    draw_rule_at_one_pixel(pixmap, (geometry, cap), style, placed, scale, brush, clip)
}

/// Whether a mark this wide is one ISO 32000-2 §10.7.4 may restate, at a quantum of `one_pixel`.
///
/// Asked twice, of two different quanta. [`draw_sub_pixel_rule`] asks it of
/// [`pdf_render::thinnest_line`], which is the width above which nothing in this module applies at
/// all; [`draw_rule_at_one_pixel`] asks it of [`pdf_render::band_substitute_width`], which is the
/// width its own widening would state. **Neither is the width at which a substitute is *owed*** —
/// that is [`pdf_render::unmeasurable_width`], one level of a device pixel, and it is asked
/// separately (ADR 1102).
///
/// **The boundary is inclusive, and that is a decision rather than a reading.** §10.7.4 states
/// the rule and one exemption:
///
/// > The area covered by painted pixels shall always be at least as large as the area of the
/// > original shape. This rule applies both to fill operations and to strokes with non-zero
/// > width. Zero-width strokes may be done in an implementation-defined manner that may include
/// > fewer pixels than the rule implies.
///
/// A `0 w` rule is the case the inclusive end is about. §10.7.4's exemption is a `may`, so
/// drawing it short is allowed; §8.4.3.2 says "\[a] line width of 0 shall denote the thinnest line
/// that can be rendered at device resolution: 1 device pixel wide". `pdf_render::Stroke::
/// device_width` resolves the zero to one device pixel **in the shared crate**, so both backends
/// draw one mark from one number, and a boundary that excluded its own value would put that mark
/// on a different path from the `1 w` rule beside it for no reason a clause states.
///
/// `<=` rather than an equality test on purpose: a page transform of 1.0037 leaves the two sides
/// near rather than equal, and a rule that only fired on the exact float would be a rule about
/// one scale.
fn at_or_under_the_quantum(width: f32, one_pixel: f32) -> bool {
    width <= one_pixel
}

/// ADR 0226's exact construction: the rule's own outline, at the coverage its area implies.
///
/// See [`draw_sub_pixel_rule`], whose first choice this is. Returns `false` — leaving the caller
/// to the construction below — for an outline [`pdf_render::sub_pixel_bands`] cannot measure
/// exactly, which is every case its module comment argues: a round cap's arc, two rules meeting in
/// one pixel row, a subpath that is not a rectangle.
///
/// Asked only of an undashed rule, so the outline it measures is the solid one
/// [`draw_stroked_outline`] fills when this declines — and [`SolidOutline`] builds it once for both.
///
/// [`draw_stroked_outline`]: super::draw_stroked_outline
fn draw_rule_as_bands(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    outline: &SolidOutline<'_>,
    placed: Placed,
    brush: &tiny_skia::Paint<'_>,
    clip: scan::Clip<'_>,
) -> bool {
    let Some(outline) = outline.get() else {
        return false;
    };
    // The pixel lines are the page's, decided under its own matrix, and each band is measured on
    // its grid and written at the surface's corner: so a window's or a strip's bands are the
    // page's (ADR 2148).
    let Some(bands) = pdf_render::sub_pixel_bands(
        &convert::from_skia_path(outline),
        placed.page,
        pdf_render::FillRule::NonZero,
    ) else {
        return false;
    };
    // Converted before anything is drawn, so a shape the rasteriser rejects leaves the whole rule
    // to the construction below rather than drawing half of it twice.
    let Some(shapes) = bands
        .iter()
        .map(|band| Some((convert::path(&band.shape)?, band.coverage)))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    for (shape, coverage) in &shapes {
        let mut faint = brush.clone();
        faint.shader.apply_opacity(*coverage);
        scan::fill_stated(
            pixmap,
            (shape, &scan::Exact::Unknown),
            &faint,
            tiny_skia::FillRule::Winding,
            placed,
            clip,
        );
    }
    true
}

/// ADR 0268's general construction: the rule stroked one device pixel wide, at the alpha its own
/// width implies, with §8.4.3.3's cap as a second mark at the alpha *its* own area implies.
///
/// See [`draw_sub_pixel_rule`] for why this is owed and [`pdf_render::band_substitute_width`] for
/// where the width comes from. The two moves are one arithmetic identity: the substitute's device
/// area is `width / style.width` times the rule's, and the alpha is `style.width / width`, so the
/// ink is the rule's own area whatever the transform and whatever the angle.
///
/// **The width is the *band's* rather than the mark's, and under an uneven placement those are
/// two different quantities.** A swept body is thin along one direction only, so what makes it a
/// device pixel thick depends on which way it runs, where [`pdf_render::substitute_width`] answers
/// the question §8.5.3.2's dot asks — a pixel across whichever way the mark runs. Asking the
/// first with the second's answer is what drew `issue12295.pdf`'s traces across twenty-five device
/// pixels at half a percent each, on a placement whose two stretches are a factor of 25 apart.
/// §8.4.3.3's cap then follows the body it caps rather than keeping a width of its own, or the
/// mark projecting past a stroke one pixel wide is twenty-five. ADR 0945.
///
/// The dashes are dispensed first, by the same `tiny_skia::Path::dash` that `stroke_path` would
/// have called, because widening a dashed stroke must not widen its dashes: §8.4.3.6's pattern is
/// measured along the path and has nothing to do with the width. The caps are taken from the
/// dashed path for the same clause's sake: §8.4.3.3 states the cap style "shall be used at both
/// ends of open subpaths (and dashes 8.4.3.6, "Line dash pattern") when they are stroked".
fn draw_rule_at_one_pixel(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    (geometry, cap): ((&Path, &tiny_skia::Path), pdf_render::LineCap),
    style: &tiny_skia::Stroke,
    placed: Placed,
    scale: f32,
    brush: &tiny_skia::Paint<'_>,
    clip: scan::Clip<'_>,
) -> bool {
    let Some(width) = pdf_render::band_substitute_width(geometry.0, placed.at) else {
        return false;
    };
    // A width above the substitute's is not this rule's business: `draw_sub_pixel_rule` has
    // already established that the stroke is at or under one device pixel by §8.4.3.2's reading.
    // Kept although `band_substitute_width` is never under `thinnest_line` — a floor that
    // function states rather than an accident of the arithmetic, and ADR 1095 is what made it
    // one — so that the test guarding the widening below is the widening's own rather than a
    // property of a caller.
    if !at_or_under_the_quantum(style.width, width) {
        return false;
    }
    let dashed;
    // The caps are stated from the *path's* own commands, and where the dasher has cut new ends
    // they have to be read back from what it produced — which is the one case that pays for a
    // conversion. An undashed rule already has its geometry in both forms.
    let mut cut_ends = None;
    let path = match style.dash.as_ref() {
        None => geometry.1,
        Some(dash) => {
            let Some(cut) = geometry.1.dash(dash, scale) else {
                return false;
            };
            dashed = cut;
            if cap != pdf_render::LineCap::Butt {
                cut_ends = Some(convert::from_skia_path(&dashed));
            }
            &dashed
        }
    };
    let mut widened = style.clone();
    widened.width = width;
    widened.dash = None;
    // The identity above is the *body's*: a cap's area goes as the square of the width, so
    // widening multiplies it by `(width / style.width)²` where the alpha divides it back only
    // once, and the end would be overstated by that factor — which is why the widened stroke is
    // butt-capped and the cap is a mark of its own below, at the alpha its own square implies.
    // **Except where nothing was widened.** The factor above is `width / style.width`, and at
    // the quantum itself it is exactly 1: a rule already one device pixel wide overstates its own
    // cap by nothing, so the stroker's own cap is exact and there is nothing to state twice.
    let widening = style.width < width;
    if widening {
        widened.line_cap = tiny_skia::LineCap::Butt;
    }
    let Some(outline) = path.stroke(&widened, scale) else {
        return false;
    };
    let mut faint = brush.clone();
    // `width` is stated in the path's own space and is finite and above zero. The floor is
    // §10.7.4's "no shape ever disappears" reaching the *alpha* the substitute rides in, which is
    // eight bits and runs out below 1/255 of a device pixel of thickness.
    faint
        .shader
        .apply_opacity(pdf_render::expressible_coverage(style.width / width));
    // The band and its caps on the page's grid, written at the surface's corner (ADR 2148).
    scan::fill_stated(
        pixmap,
        (&outline, &scan::Exact::Unknown),
        &faint,
        tiny_skia::FillRule::Winding,
        placed,
        clip,
    );
    // A butt cap projects nothing, which is the overwhelmingly common case and the one that must
    // not pay for a path conversion to be told so.
    if widening
        && cap != pdf_render::LineCap::Butt
        && let Some(mark) = pdf_render::enlarged_mark_at(style.width, width)
        && let Some(caps) =
            pdf_render::sub_pixel_caps(cut_ends.as_ref().unwrap_or(geometry.0), cap, mark)
        && let Some(converted) = convert::path(&caps)
    {
        let mut faint = brush.clone();
        faint.shader.apply_opacity(mark.coverage);
        scan::fill_stated(
            pixmap,
            (&converted, &scan::Exact::Unknown),
            &faint,
            tiny_skia::FillRule::Winding,
            placed,
            clip,
        );
    }
    true
}
