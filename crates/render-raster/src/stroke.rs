//! Strokes: what raster draws, and what is settled here first.
//!
//! raster expands caps, joins and miters itself, and — since the amendment of the
//! brief's section 4.5 on 2026-08-27 (ADR 0701, raster's ADR 0085) — resolves the
//! device width itself, per
//! placement, from the scene-space width and §10.7.5's `adjust` flag this module now
//! passes through unresolved. It does not dash, and it does not re-take the other
//! decisions `pdf-render` owns. So this module runs the same shared machinery the
//! other backends run, in the same order:
//!
//! 1. [`pdf_render::split_degenerate`]: ISO 32000-2 §8.5.3.2's zero-length
//!    subpaths become dots (or nothing), decided by one implementation.
//! 2. [`kurbo::dash`]: the dash pattern is cut in path space, by the dasher
//!    `render-gpu` uses through vello — identical dashes by construction — with
//!    §8.5.3.2's zero-length *dashes* turned into marks through
//!    [`pdf_render::dashes_showing_direction`] and
//!    [`pdf_render::split_dash_marks`].
//! 3. raster strokes the surviving subpaths solid and fills the dots.
//!
//! **And one decision taken before any of that: §8.3.4 NOTE 3's matrix with no inverse.** raster
//! needs no inverse to place a paint — it anchors one in page space, where `tiny-skia` and Vello
//! each want the draw transform undone — so what [`pdf_render::paint_space`] gives this module is
//! only the *refusal*, and taking it from there is what keeps the three backends refusing the same
//! marks (viewer trap 2). Without it such a transform reached `path_width * max_stretch` below,
//! which is a width of zero, and the **scene** refused with `InvalidStroke`: the whole frame lost
//! for one mark whose path has collapsed onto a line or a point and covers no area anyway. That is
//! `doc/todo/11` item 8 in raster's own spelling, and ADR 0482 has the reading.

use std::sync::Arc;

use pdf_render::{
    BlendMode, ClipId, LineCap, LineJoin, Paint, Path, PathCommand, Point, SoftMaskId, Stroke,
    Transform,
};
use raster_scene::SceneBuilder;

use crate::QuorraRasterError;
use crate::scene::{Encoder, blend_mode, colour, compose, overprinted};

/// Encodes one stroke command.
pub(crate) fn encode(
    enc: &mut Encoder<'_>,
    builder: &mut SceneBuilder,
    path: &Arc<Path>,
    transform: Transform,
    s: &Stroke,
    paint: &Paint,
    (clip, mask, blend): (Option<ClipId>, Option<SoftMaskId>, BlendMode),
) -> Result<(), QuorraRasterError> {
    // ISO 32000-2 §8.3.4 NOTE 3's matrix with no inverse, asked beside the empty path because the
    // two are one sentence — this command marks nothing, so it is refused and the page is drawn.
    // Stated for all three backends in `pdf-render`; the module comment says what it cost raster.
    if path.is_empty() || pdf_render::paint_space(transform).is_none() {
        return Ok(());
    }
    // A stroke `pdf-render` sized as thin for every backend (trap 2) is one raster line (ADR 1835).
    if thin_line(enc, builder, path, transform, s, paint, (clip, mask, blend))? {
        return Ok(());
    }
    // A pattern past the dashes `pdf-render` lets a stroke be cut into is drawn solid at the
    // coverage it implies, decided there for all three backends (trap 2, ADR 3334): `kurbo`'s
    // dasher has no bound of its own, and would build every dash. The pattern is a statement in
    // the path's own space, so its count asks nothing of the view.
    let to_device = transform.then(enc.target().transform);
    let rest_coverage = pdf_render::past_the_dash_bound(path, s, s.device_width(to_device));
    let solid_stroke;
    let s: &Stroke = if rest_coverage.is_some() {
        solid_stroke = Stroke {
            dash_array: Vec::new(),
            dash_phase: 0.0,
            ..s.clone()
        };
        &solid_stroke
    } else {
        s
    };
    let rest_coverage = rest_coverage.unwrap_or(1.0);
    let fitted = fitted_for_dashing(enc, path, transform, s);
    let fitted_path = fitted.as_ref().map(|fit| Arc::new(fit.path.clone()));
    let path = fitted_path.as_ref().unwrap_or(path);
    let s = fitted.as_ref().map_or(s, |fit| &fit.stroke);
    let mask = enc.mask_id(builder, mask)?;
    let Some(raster_paint) = raster_paint(enc, paint, (path, transform, s))? else {
        return Ok(()); // nothing visible to paint with
    };

    // §8.4.3.2 with §10.7.5, resolved by the shared method — which answers in
    // *path* units despite its name (the other backends scale it through the draw
    // transform). raster expands on device-space geometry with one scalar width,
    // which is exact for a similarity transform and exactly wrong for any other:
    // a sheared or unevenly-scaled stroke must vary its device width with
    // direction (§8.4.3.2's own note), and a scalar cannot. So the transform's
    // anisotropy decides the route below.
    let path_width = s.device_width(to_device);

    // §10.7.4's substitution for a mark whose area is under what the raster can hold, decided
    // in `pdf-render` for all three backends (trap 2). Two conditions withhold it, and both are
    // about the coverage having nowhere but the alpha to go: inside §11.4.6's knockout an
    // element replaces its backdrop within its own *shape*, which raster reads off the coverage
    // a mark is drawn with; and this builder's `fill` takes no alpha of its own, so a coverage
    // can only be carried in a solid colour. A mark painted by a shading therefore keeps the
    // shape §8.5.3.2 states, and the cross-backend gate is what would show such a page.
    let substitute =
        (!enc.inside_knockout() && matches!(paint, Paint::Solid(_))).then_some(to_device);
    // §8.5.3.2's zero-length subpaths, split by the shared rule — in path space,
    // with the path-space width, as the shared helpers expect.
    let split = pdf_render::split_degenerate(path, s.cap, path_width, substitute);
    let geometry: &Path = split.as_ref().map_or(path.as_ref(), |d| &d.stroked);
    // §8.4.3.6's "the end cap is painted before the corner" at the vertex a closed subpath's own
    // close makes, decided in `pdf-render` for all three backends (trap 2): a dasher that merges
    // the first and last dash of a closed contour cannot tell a dash the close cut short — which
    // §8.4.3.4 joins — from one that finished there.
    let opened =
        pdf_render::opened_where_a_dash_ends_at_the_close(geometry, &s.dash_array, s.dash_phase);
    let geometry: &Path = opened.as_ref().unwrap_or(geometry);
    let mut dots: Path = split.as_ref().map_or_else(Path::new, |d| d.dots.clone());
    // The two rules make marks of one width under one cap, so they share one coverage;
    // whichever produced marks answers for both.
    let mut coverage = split.as_ref().map_or(1.0, |d| d.coverage);

    let dashed = dash(
        geometry,
        s,
        (path_width, substitute),
        &mut dots,
        &mut coverage,
    );
    // Asked after the splits and not before: what decides whether the clip can be left off this
    // mark is how far the mark reaches, and `reach` is where that is settled.
    let reach = reach(path, transform, s, (&dots, coverage));
    let crate::scene::Admitted::Chain(clip) = enc.clip_chain(builder, clip, reach)? else {
        return Ok(()); // the clip admits nothing
    };
    let site = (path, transform, s, paint, blend);
    keep_marks(enc, site, (path_width, substitute), (&dots, coverage));
    let solid: &Path = dashed.as_ref().unwrap_or(geometry);

    let at = enc.placed(transform);
    let marks = Marks {
        source: path,
        solid,
        stroke: s,
        path_width,
        to_device,
        from_source: dashed.is_none() && split.is_none(),
        at,
        paint: raster_paint,
        dots: &dots,
        coverage,
        rest_coverage,
    };
    // §11.6.2: "\[p]ortions of an object shall not be composited with one another", and the
    // stroked rest and its dots are portions of one stroke. Where drawing one over the other
    // would apply the paint twice, both are elements of an isolated knockout group of their own,
    // so the dots replace the rest within their shape and the group meets the backdrop once under
    // the stroke's clip, mask and blend (ADR 3006). Two cases keep the separate draw: a
    // substituted dot carries part of its coverage in its alpha, which a knockout would read as
    // shape; and a stroke that is itself an element of §11.4.6's knockout group cannot be a group,
    // whose shape a layer does not carry (`doc/adr/1295`).
    let applied_once = !solid.is_empty()
        && !dots.is_empty()
        && coverage >= 1.0
        && rest_coverage >= 1.0
        && !enc.knockout_element()
        && !composites_as_one(paint, blend);
    if applied_once {
        enc.one_object(builder, (clip, mask, blend), |enc, builder| {
            draw_marks(enc, builder, &marks, (None, None, BlendMode::Normal))
        })
    } else {
        draw_marks(enc, builder, &marks, (clip, mask, blend))
    }
}

/// Whether drawing a stroke's portions one after another composites them exactly as drawing their
/// union once would: an opaque colour under the Normal blend, for which `(1 − a)(1 − b)` of the
/// backdrop survives either way (ADR 3006).
fn composites_as_one(paint: &Paint, blend: BlendMode) -> bool {
    blend == BlendMode::Normal && matches!(paint, Paint::Solid(colour) if colour.a >= 1.0)
}

/// The marks one stroke is drawn as once `pdf-render` has split and dashed it: the solid rest,
/// stroked, and §8.5.3.2's dots, filled beside it at `coverage`.
struct Marks<'a> {
    /// The display list's own path, which the expansion cache is keyed by.
    source: &'a Arc<Path>,
    /// What is stroked solid.
    solid: &'a Path,
    /// The stroke's parameters.
    stroke: &'a Stroke,
    /// §8.4.3.2's width in path units.
    path_width: f32,
    /// Path space to the device.
    to_device: Transform,
    /// Whether `solid` *is* the display list's own path.
    from_source: bool,
    /// Where the scene places the marks.
    at: raster_scene::Affine,
    /// The stroke's paint.
    paint: raster_scene::Paint,
    /// The filled dots.
    dots: &'a Path,
    /// The coverage the dots carry in their alpha.
    coverage: f32,
    /// The coverage the rest carries in its alpha: below one only for a dash pattern past
    /// `pdf_render::MAX_DASHES`, drawn solid at the coverage it implies (ADR 3334). A paint that is
    /// not a solid colour has no alpha this builder can carry it in, and is drawn at its whole
    /// coverage, as §10.7.4's substituted dots are.
    rest_coverage: f32,
}

/// Draws `marks` under one clip, mask and blend.
fn draw_marks(
    enc: &mut Encoder<'_>,
    builder: &mut SceneBuilder,
    marks: &Marks<'_>,
    (clip, mask, blend): (
        Option<raster_scene::ClipId>,
        Option<raster_scene::MaskId>,
        BlendMode,
    ),
) -> Result<(), QuorraRasterError> {
    let Marks {
        source: path,
        solid,
        stroke: s,
        path_width,
        to_device,
        from_source,
        at,
        paint: raster_paint,
        dots,
        coverage,
        rest_coverage,
    } = *marks;
    let rest_paint = faint(raster_paint, rest_coverage);
    if !solid.is_empty() {
        // §8.4.3.2's thinnest line is one device pixel in every direction, which is what raster's
        // own expansion draws from the width it resolves per placement (ADR 0085), and what no
        // path-space width draws under unequal scales (ADR 3333).
        let in_path_space =
            !s.is_thinnest_line(to_device) && !scalar_width_holds(to_device, path_width);
        if in_path_space {
            // The expansion below happens in device space, on this view's transform
            // (ADR 0702).
            enc.consume_view();
            let outline = expanded(
                enc,
                Expansion {
                    source: path,
                    solid,
                    stroke: s,
                    path_width,
                    to_device,
                    // Whether `solid` *is* the display list's own path rather than geometry this
                    // frame computed from it — which is what decides whether the expansion can be
                    // cached, below.
                    from_source,
                },
            )?;
            builder.fill(
                outline,
                at,
                raster_scene::FillRule::NonZero,
                rest_paint,
                clip,
                blend_mode(blend),
                compose(blend),
                mask,
            )?;
        } else {
            // Under a transform that is not a similarity the choice above read this view's
            // magnification through [`MAX_SCALAR_WIDTH_ERROR`], so the scene it built is only
            // true at this target (ADR 0702).
            if anisotropy(to_device) > 1.0 + SIMILARITY_ROUNDING {
                enc.consume_view();
            }
            // The untouched common case keeps the path's `Arc` identity, so a
            // glyph stroked a thousand times uploads once; computed geometry is
            // per-frame.
            let outline = if from_source {
                enc.outline(path)?
            } else {
                enc.transient_outline(solid)?
            };
            let stroke = raster_scene::Stroke {
                // Scene-space since raster's ADR 0085 (this tree's ADR 0701): the
                // width travels as the file stated it, and the encode applies
                // §8.4.3.2's zero and §10.7.5's adjustment per placement — the
                // amendment that lets one scene be true at every magnification.
                width: s.width,
                adjust: s.adjust,
                cap: cap(s.cap),
                join: join(s.join),
                // §8.4.3.5 defines the limit as a ratio of at least 1; a
                // smaller value from a malformed file behaves as the smallest
                // legal one.
                miter_limit: s.miter_limit.max(1.0),
            };
            // A raster stroke carries no compositing operator, so §11.7.4.3's mode puts
            // it in a group of one (`crate::scene::overprinted`).
            overprinted(builder, blend, enc.knockout_element(), |builder| {
                builder.stroke(
                    outline,
                    at,
                    stroke,
                    rest_paint,
                    clip,
                    blend_mode(blend),
                    mask,
                )
            })?;
        }
    }
    fill_dots(
        enc,
        builder,
        dots,
        at,
        faint(raster_paint, coverage),
        (clip, mask, blend),
    )
}

/// ISO 32000-2 §10.7.5's fit of a stroke whose dash pattern is to be cut, or `None` for a solid
/// stroke and for one the fit does not take.
///
/// `render-cpu` fits the command's path at its stroker's entry and dashes the fitted path, so
/// every dash — and every zero-length dash §8.5.3.2 makes a mark of — lies where the fitted rule
/// runs. raster fits a stroke at its own encode, per placement (ADR 2553), but the dashes are cut
/// here in path space, so a dashed stroke the fit takes is fitted here as the oracle fits it, and
/// the scene that holds it is this view's.
///
/// **A decline is an answer of this view too** (ADR 2902). The fit's bounds are device measures —
/// a width under [`pdf_render::FIT_BELOW`] device pixels, a vertex not already on a pixel centre —
/// so the dashes a stroke under `/SA` keeps unfitted here are cut where another view's build would
/// cut them only where that view declines as well: the scene keeps the question as an
/// [`UnfittedDashes`] and another view asks it again before it keeps the scene. A stroke without
/// `/SA` is declined at every view by the fit's first test, and asks nothing.
fn fitted_for_dashing(
    enc: &mut Encoder<'_>,
    path: &Arc<Path>,
    transform: Transform,
    s: &Stroke,
) -> Option<pdf_render::GridFit> {
    if s.dash_array.is_empty() || !s.adjust {
        return None;
    }
    let Some(fitted) = pdf_render::grid_fit(path, s, transform.then(enc.target().transform)) else {
        enc.keep_mark(KeptMark::Unfitted(UnfittedDashes {
            path: Arc::clone(path),
            transform,
            stroke: s.clone(),
        }));
        return None;
    };
    enc.consume_view();
    Some(fitted)
}

/// A dashed stroke under `/SA` whose §10.7.5 fit the scene's view declined, kept so that another
/// view asks the fit again before it keeps the scene (ADR 2902).
///
/// Where [`pdf_render::grid_fit`] declines at another view too, a build there dashes the command's
/// own path in path space exactly as this scene did, so the kept scene is that build; where it
/// takes, a build there dashes the fitted path, whose corners have moved along the segments they
/// end and carried every dash after them, and the scene is rebuilt.
///
/// [`pdf_render::grid_fit`]: fn@pdf_render::grid_fit
#[derive(Debug, Clone)]
pub(crate) struct UnfittedDashes {
    path: Arc<Path>,
    transform: Transform,
    /// The command's stroke: its width, `/SA` and dash pattern.
    stroke: Stroke,
}

impl UnfittedDashes {
    /// Whether the fit declines at a view placing the page by `view` as well.
    fn declines_at(&self, view: Transform) -> bool {
        pdf_render::grid_fit(&self.path, &self.stroke, self.transform.then(view)).is_none()
    }
}

/// A stroke of one straight segment, drawn as one [`raster_scene::Command::Line`] — its exact area
/// in the shader, with no outline behind it (raster's ADR 0097) — and `true`; or `false`, and
/// nothing drawn, for every stroke that is not one.
///
/// **Which strokes are lines is decided without the view** (ADR 1877). Raster's line is
/// §8.4.3.2's set with Table 53's end by its exact area at every size, and it resolves the
/// width per placement exactly as it resolves a stroke's (its ADR 0085: §8.4.3.2's zero and
/// §10.7.5's adjustment), so nothing about a segment of two distinct points needs this view to be
/// drawn right — and a scene holding only such lines is true at every view (ADR 0702), which is
/// what lets a zoom step keep it. The processor keeps its own population, the segments whose ink
/// fits [`pdf_render::thin_segment`]'s block at this view (ADR 1829): the two backends measure
/// one set by two constructions past that block, which raster ADR 0082's bound covers.
///
/// §8.5.3.2's round dot is the one line still decided at the view, by
/// [`pdf_render::thin_segment`] as the processor decides it: whether its area reaches a level is
/// what keeps it from §10.7.4's substitution in `split_degenerate`, and the device and the
/// processor then measure it by the one closed form (ADR 1857). So the scene keeps each such dot
/// as a [`DotAsLine`] instead of reading the view, and is kept at another view only where every
/// one of them is a line there too (ADR 1877).
///
/// [`pdf_render::thin_segment`]: fn@pdf_render::thin_segment
fn thin_line(
    enc: &mut Encoder<'_>,
    builder: &mut SceneBuilder,
    path: &Arc<Path>,
    transform: Transform,
    s: &Stroke,
    paint: &Paint,
    (clip, mask, blend): (Option<ClipId>, Option<SoftMaskId>, BlendMode),
) -> Result<bool, QuorraRasterError> {
    // One straight segment, or §8.5.3.2's single point however the path states it (`m l`, `m h`,
    // a curve that stays at its point), read by the shared rule the processor reads it by.
    let (Paint::Solid(color), Some((from, to))) = (paint, pdf_render::segment_ends(path)) else {
        return Ok(false);
    };
    if !is_view_free_line(transform, s, blend, (from, to)) {
        return Ok(false);
    }
    if from == to {
        // Under the other two caps a dot paints nothing (§8.5.3.2), which `split_degenerate`
        // states; a line of no length has no direction to put such an end along.
        if s.cap != LineCap::Round {
            return Ok(false);
        }
        let dot = DotAsLine {
            path: Arc::clone(path),
            transform,
            stroke: s.clone(),
            paint: paint.clone(),
        };
        if !dot.is_line_at(enc.target().transform) {
            return Ok(false);
        }
        enc.keep_mark(KeptMark::Line(dot));
    }
    let mask = enc.mask_id(builder, mask)?;
    let reach = reach(path, transform, s, (&Path::new(), 1.0));
    let crate::scene::Admitted::Chain(clip) = enc.clip_chain(builder, clip, reach)? else {
        return Ok(true); // the clip admits nothing
    };
    builder.line(
        (
            raster_scene::Point::new(from.x, from.y),
            raster_scene::Point::new(to.x, to.y),
        ),
        enc.placed(transform),
        (s.width, s.adjust),
        cap(s.cap),
        colour(*color),
        clip,
        mask,
    )?;
    Ok(true)
}

/// Keeps the answers §8.5.3.2's dots and zero-length-dash marks were stated by, or consumes the
/// view where no other view can give them again; a stroke that made no mark asks nothing.
///
/// The marks are stated by two answers this view gives — the width §8.4.3.2 and §10.7.5 resolve
/// to, and §10.7.4's [`pdf_render::stated_point_mark`] at it — and by nothing else of it: which
/// subpaths are degenerate is the path's own geometry and the dash pattern cuts in path space with
/// no width in the arithmetic (ADR 0705). So the scene keeps the answers and another view asks
/// them again before it keeps the scene (ADR 2884). A mark stated as the device pixel its centre
/// lies in is this grid's pixel, which no other placement names, so that answer alone still
/// consumes the view.
fn keep_marks(
    enc: &mut Encoder<'_>,
    (path, transform, s, paint, blend): (&Arc<Path>, Transform, &Stroke, &Paint, BlendMode),
    (width, substitute): (f32, Option<Transform>),
    (dots, coverage): (&Path, f32),
) {
    if dots.is_empty() && coverage >= 1.0 {
        return;
    }
    let marks = StatedMarks {
        transform,
        stroke: s.clone(),
        substitute: substitute.is_some(),
        width,
        mark: pdf_render::stated_point_mark(s.cap, width, substitute),
        lone_dot: lone_dot(path, transform, s, paint, blend),
    };
    if marks.mark.in_its_pixel {
        enc.consume_view();
    } else {
        enc.keep_mark(KeptMark::Stated(marks));
    }
}

/// A question a scene answered at the view it was built at, kept so that another view asks it
/// again before it keeps the scene (ADRs 1877, 2884, 2902).
#[derive(Debug, Clone)]
pub(crate) enum KeptMark {
    /// A round dot the scene drew as a line.
    Line(DotAsLine),
    /// The marks §8.5.3.2's splits made, as the scene stated them.
    Stated(StatedMarks),
    /// A dashed stroke whose §10.7.5 fit the scene's view declined.
    Unfitted(UnfittedDashes),
}

impl KeptMark {
    /// Whether a build at a view placing the page by `view` makes this mark as the kept scene
    /// holds it.
    pub(crate) fn holds_at(&self, view: Transform) -> bool {
        match self {
            Self::Line(dot) => dot.is_line_at(view),
            Self::Stated(marks) => marks.holds_at(view),
            Self::Unfitted(dashes) => dashes.declines_at(view),
        }
    }
}

/// The marks one stroke's §8.5.3.2 splits made, kept with the two answers they were stated by
/// (ADR 2884).
///
/// [`pdf_render::split_degenerate`] and [`pdf_render::split_dash_marks`] read the view through
/// two values and nothing else: the width [`Stroke::device_width`] resolves (§8.4.3.2's zero and
/// §10.7.5's adjustment) and the [`pdf_render::PointMark`] [`pdf_render::stated_point_mark`]
/// decides at it (§10.7.4). Where a view gives both again, the marks a build there makes are
/// these marks to the bit — the circles and squares are the path's own points at that width, the
/// coverage the mark's — so the kept scene is that build.
///
/// **What keeping them buys** (`examples/dotted_zoom`, ADRs 2884 and 2902): on a page of 150 dotted
/// leaders, 27 750 marks, a zoom step kept its scene at 6.8–13.6 ms a frame where the same step
/// rebuilt it at 28.7–30.2 ms, the scene phase 0.01 ms against 7.6–10.0 (release, five rounds,
/// minimum of each, on the RX 580 through RADV at a load of 2.4). The cost is this record per
/// stroke that made marks, and asking it again at each view.
#[derive(Debug, Clone)]
pub(crate) struct StatedMarks {
    transform: Transform,
    /// The command's stroke, whose width and flag the view resolves.
    stroke: Stroke,
    /// Whether the encode handed the splits the transform, which is a property of the paint and
    /// the group and not of the view.
    substitute: bool,
    /// The width the marks were stated at, in the path's own space.
    width: f32,
    mark: pdf_render::PointMark,
    /// The stroke as [`thin_line`] would take it, where it is a lone round dot the build's view
    /// left to the splits: another view that draws it as a line draws something else.
    lone_dot: Option<DotAsLine>,
}

impl StatedMarks {
    /// Whether a build at a view placing the page by `view` states these marks.
    fn holds_at(&self, view: Transform) -> bool {
        let to_device = self.transform.then(view);
        let width = self.stroke.device_width(to_device);
        width.to_bits() == self.width.to_bits()
            && pdf_render::stated_point_mark(
                self.stroke.cap,
                width,
                self.substitute.then_some(to_device),
            ) == self.mark
            && !self
                .lone_dot
                .as_ref()
                .is_some_and(|dot| dot.is_line_at(view))
    }
}

/// The stroke as a [`DotAsLine`], where [`thin_line`] would take it at a view that draws it as a
/// line: a lone round dot of a solid paint, under the conditions every line keeps.
fn lone_dot(
    path: &Arc<Path>,
    transform: Transform,
    s: &Stroke,
    paint: &Paint,
    blend: BlendMode,
) -> Option<DotAsLine> {
    let (from, to) = pdf_render::segment_ends(path)?;
    (matches!(paint, Paint::Solid(_))
        && s.cap == LineCap::Round
        && from == to
        && is_view_free_line(transform, s, blend, (from, to)))
    .then(|| DotAsLine {
        path: Arc::clone(path),
        transform,
        stroke: s.clone(),
        paint: paint.clone(),
    })
}

/// A round dot a scene drew as a line, kept so that another view can ask whether it would draw
/// it as a line too (ADR 1877).
///
/// Whether §8.5.3.2's dot is the line's disc or §10.7.4's substitution is a question about the
/// pixels it lands in, so it is the one answer a scene of lines carries from the view it was
/// built at. A fresh build at another view asks it again of every dot, and a kept scene is that
/// build only where every answer is the same — which this asks, by the same call.
#[derive(Debug, Clone)]
pub(crate) struct DotAsLine {
    /// The dot's own path: a single point, however the path states it.
    path: Arc<Path>,
    transform: Transform,
    /// The command's stroke, whose dash array a line's is empty.
    stroke: Stroke,
    paint: Paint,
}

impl DotAsLine {
    /// Whether this dot is drawn as a line at a view placing the page by `view`.
    ///
    /// Where the processor's block admits it, by the processor's own call; and past the block,
    /// wherever the dot is at least a device pixel across both ways: its disc then covers more
    /// than π/4 of a pixel, so §10.7.4 substitutes nothing for it, and the line's disc is the
    /// clause's filled circle by its exact area at any size.
    pub(crate) fn is_line_at(&self, view: Transform) -> bool {
        let to_device = self.transform.then(view);
        let across = self.stroke.device_width(to_device) * to_device.min_stretch();
        (self.stroke.cap == LineCap::Round && to_device.preserves_axes() && across >= 1.0)
            || pdf_render::thin_segment(
                &self.path,
                &self.stroke,
                &self.paint,
                BlendMode::Normal,
                to_device,
            )
            .is_some()
    }
}

/// Whether a solid stroke of the one segment `from`–`to` is a raster line at every view: no dash
/// (the dash pattern is this crate's to cut), `Normal` (raster's line composites over), a path
/// transform that keeps the axes and collapses nothing — the line's pen is an ellipse on the
/// device's axes only there, and a page placement keeps the axes — and coordinates and a width
/// raster's scene admits, so that a line it would refuse takes the stroke road rather than losing
/// the frame.
fn is_view_free_line(
    transform: Transform,
    s: &Stroke,
    blend: BlendMode,
    (from, to): (Point, Point),
) -> bool {
    let inside = |v: f32| v.is_finite() && v.abs() <= raster_scene::MAX_COORDINATE;
    let determinant = transform.determinant();
    blend == BlendMode::Normal
        && s.dash_array.is_empty()
        && transform.preserves_axes()
        && determinant.is_finite()
        && determinant != 0.0
        && [from.x, from.y, to.x, to.y].into_iter().all(inside)
        && inside(s.width)
        && s.width >= 0.0
}

/// The raster paint this stroke draws with, or `None` where what it names paints nothing.
///
/// `None` is a shading whose raster is empty, which marks nothing on the sibling backends either
/// — see `crate::scene::ShadedPaint`. A sampled shading and every other paint are refused by
/// name, so the frame goes to the CPU backend rather than being drawn with the wrong colour.
///
/// A shading stating Table 77's `/AntiAlias` is the one stroking paint that is not this device's
/// own: §8.7.4.1 lets a shading pattern paint "with painting operators such as f (fill), S
/// (stroke)", and the table asks "to filter the shading function" whichever paints it. So it is
/// the raster a fill takes ([`Encoder::background_raster`], ADR 3024), built over the pixels the
/// stroke can reach ([`ink_pixels`]) and drawn through the stroke as the mesh paint is (ADR 3074).
fn raster_paint(
    enc: &mut Encoder<'_>,
    paint: &Paint,
    (path, transform, s): (&Path, Transform, &Stroke),
) -> Result<Option<raster_scene::Paint>, QuorraRasterError> {
    if let Paint::Shading(shading) = paint
        && shading.is_rastered()
    {
        let within = ink_pixels(enc, path, transform, s);
        return Ok(enc
            .background_raster(shading, within)?
            .map(raster_scene::Paint::Mesh));
    }
    Ok(Some(match paint {
        Paint::Solid(c) => raster_scene::Paint::Solid(colour(*c)),
        Paint::Shading(shading) => match enc.shading_paint(shading)? {
            crate::scene::ShadedPaint::Ready(paint) => paint,
            crate::scene::ShadedPaint::Sampled => {
                return Err(QuorraRasterError::Unsupported(
                    "a sampled shading painting a stroke".into(),
                ));
            }
            crate::scene::ShadedPaint::Nothing => return Ok(None),
        },
        other => {
            return Err(QuorraRasterError::Unsupported(format!("paint {other:?}")));
        }
    }))
}

/// The device pixels a stroke's marks can paint, as `(left, top, right, bottom)` clamped to the
/// target: `render-cpu`'s ink extent, for the same marks (ADR 2404). `pdf_render::stroked_bounds`
/// bounds the outline at the stated width term by term (§8.4.3), and the marks drawn one device
/// pixel wide — §8.4.3.2's zero width, §10.7.4's enlargement — reach half that pixel along the
/// path, `√2` halves at a projecting square cap's corner (§8.4.3.3) and the mitre limit's halves
/// at a mitre's tip (§8.4.3.5); a pixel whose centre lies half a pixel past that is still painted.
/// The whole target where the path names no point.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "each cast is between a device pixel index and its coordinate, clamped to the \
              target's extent"
)]
fn ink_pixels(
    enc: &Encoder<'_>,
    path: &Path,
    transform: Transform,
    s: &Stroke,
) -> (u32, u32, u32, u32) {
    /// A projecting square cap's far corner sits `√2` half-widths from the endpoint.
    const SQUARE_CAP_CORNER: f32 = std::f32::consts::SQRT_2;
    let target = enc.target();
    let (width, height) = (target.width as f32, target.height as f32);
    let Some(ink) = pdf_render::stroked_bounds(path, s, transform.then(target.transform)) else {
        return (0, 0, target.width, target.height);
    };
    let half_widths = match s.join {
        LineJoin::Miter => s.miter_limit.max(SQUARE_CAP_CORNER),
        LineJoin::Round | LineJoin::Bevel => SQUARE_CAP_CORNER,
    };
    let reach = 0.5 + half_widths / 2.0;
    // `max` before `min`, so a coordinate that is not a number lands on the target's edge and
    // the extent is empty rather than the whole target.
    let clamp = |at: f32, high: f32| at.max(0.0).min(high) as u32;
    (
        clamp((ink.min.x - reach).floor(), width),
        clamp((ink.min.y - reach).floor(), height),
        clamp((ink.max.x + reach).ceil(), width),
        clamp((ink.max.y + reach).ceil(), height),
    )
}

/// How far this stroke's marks reach, in the path's own page space, or `None` where they can
/// reach past the outline §8.4.3 gives them.
///
/// What `crate::scene::Encoder::cuts_nothing` needs is a bound the *document* states rather than
/// one this view does, and §10.7.4's substitutions are what break that: a stroke that made
/// §8.5.3.2's dot or gave up width to the clause's enlargement marks one device pixel across,
/// which is a size the magnification decides. So is a width the device will resolve to a whole
/// pixel — §8.4.3.2's zero and §10.7.5's adjustment, `raster::resolve_width` — and each of those
/// answers `None`, which keeps the clip.
///
/// What is left is a stroke whose outline is the one `pdf_render::stroked_bounds` bounds, in the
/// path's own space where the width is stated, mapped from there.
fn reach(
    path: &Path,
    transform: Transform,
    s: &Stroke,
    (dots, coverage): (&Path, f32),
) -> Option<pdf_render::Rect> {
    if s.width <= 0.0 || s.adjust || !dots.is_empty() || coverage < 1.0 {
        return None;
    }
    pdf_render::stroked_bounds(path, s, transform)
}

/// Fills §8.5.3.2's dots — the marks the degenerate and zero-length-dash splits made —
/// with the stroke's paint at the coverage the split answered for.
fn fill_dots(
    enc: &mut Encoder<'_>,
    builder: &mut SceneBuilder,
    dots: &Path,
    at: raster_scene::Affine,
    paint: raster_scene::Paint,
    (clip, mask, blend): (
        Option<raster_scene::ClipId>,
        Option<raster_scene::MaskId>,
        BlendMode,
    ),
) -> Result<(), QuorraRasterError> {
    if dots.is_empty() {
        return Ok(());
    }
    let outline = enc.transient_outline(dots)?;
    builder.fill(
        outline,
        at,
        raster_scene::FillRule::NonZero,
        paint,
        clip,
        blend_mode(blend),
        compose(blend),
        mask,
    )?;
    Ok(())
}

/// One stroke to expand in path space, as [`expanded`] takes it.
///
/// `Copy`, so that it is passed by value like the other small parameter bundles here: every field
/// is a reference or a scalar and there is nothing to move.
#[derive(Clone, Copy)]
struct Expansion<'a> {
    /// The display list's own path — the identity the cache is keyed and pinned by, whether or
    /// not it is the geometry being expanded.
    source: &'a Arc<Path>,
    /// What is actually expanded: [`Self::source`] itself, or what dashing and §8.5.3.2's
    /// degenerate split left of it.
    solid: &'a Path,
    stroke: &'a Stroke,
    /// The width in the path's own space, already resolved through §8.4.3.2 and §10.7.5.
    path_width: f32,
    to_device: Transform,
    /// Whether [`Self::solid`] is [`Self::source`] rather than geometry this frame computed.
    from_source: bool,
}

/// Expands a stroke into a fillable outline, in **path** space, and uploads it.
///
/// This is the route §8.4.3.2's own note forces wherever the placement is anisotropic: "the
/// thickness of stroked lines in device space shall vary according to their orientation", and
/// raster widens from one scalar device width, which cannot. So the expansion happens here,
/// through the same `kurbo::stroke` the Vello backend outlines with — which is what gives the
/// corpus's sheared pattern marks (issue2177, issue6769) the CPU oracle's geometry rather than the
/// widest direction everywhere — and raster fills the result.
///
/// **Cached when the geometry going in is the display list's own path, transient when this frame
/// computed it.** That is the same division the scalar branch makes, and it is here for a reason
/// stated the other way round: an identifier that moves between two renders of one unchanged page
/// makes every atlas key raster derives from it foreign, and its atlas then repacks at period two
/// for ever (ADR 0402). A dashed or degenerate stroke has no stable source to key on, so it stays
/// a transient — and the expansion is a closure so that a cache hit skips `kurbo::stroke` as well
/// as the upload.
fn expanded(
    enc: &mut Encoder<'_>,
    parts: Expansion<'_>,
) -> Result<raster_scene::OutlineId, QuorraRasterError> {
    let Expansion {
        source,
        solid,
        stroke: s,
        path_width,
        to_device,
        from_source,
    } = parts;
    let style = kurbo::Stroke::new(f64::from(path_width))
        .with_caps(kurbo_cap(s.cap))
        .with_join(kurbo_join(s.join))
        .with_miter_limit(f64::from(s.miter_limit.max(1.0)));
    // A quarter device pixel, expressed in path units — the same flattening tolerance raster
    // itself draws with.
    let tolerance = (0.25 / to_device.max_stretch()).clamp(1e-4, 1.0);
    let expand = || {
        from_bez(
            kurbo::stroke(
                bez(solid).iter(),
                &style,
                &kurbo::StrokeOpts::default(),
                f64::from(tolerance),
            )
            .iter(),
        )
    };
    if from_source {
        enc.expanded_stroke(
            source,
            crate::cache::StrokeKey::new(source, s, path_width, tolerance),
            expand,
        )
    } else {
        enc.transient_outline(&expand())
    }
}

/// How far a scalar device width may sit from the truth before the path-space
/// route takes over, as a ratio: the transform's two singular values, allowed to
/// reach 1% — far under the 1.9× that made issue6769's bar visibly fat.
const MAX_ISOTROPY_ERROR: f32 = 1.01;

/// The same bound in device pixels: a quarter of one.
///
/// A ratio alone does not bound a thick line. ISO 32000-2 §8.4.3.2 has the device thickness
/// "vary according to their orientation" under unequal scales, and the two device pixels the
/// clause allows are for "the positions of lines with respect to the pixel grid", not for a
/// width read off the wrong direction — a 1000-unit line under a 1.009 scale was nine pixels
/// wide across the axis the scale leaves alone. The clause states no tolerance for that error,
/// so this is a choice and not a derivation: a quarter pixel is under what anti-aliased
/// coverage shows at an edge, and it keeps every stroke thinner than 25 pixels under a 1% ratio
/// on the scalar route, where the scene stays true at every magnification (ADR 3007).
const MAX_SCALAR_WIDTH_ERROR: f32 = 0.25;

/// How far above 1 [`anisotropy`] reads for a similarity transform, from the rounding of its
/// determinant and stretch in `f32`: a few units in the last place.
const SIMILARITY_ROUNDING: f32 = 1.0e-5;

/// Whether one scalar device width states a stroke `path_width` path units wide under `t`
/// within both bounds: [`MAX_ISOTROPY_ERROR`] on the ratio of the singular values and
/// [`MAX_SCALAR_WIDTH_ERROR`] on the difference of the widths along them.
fn scalar_width_holds(t: Transform, path_width: f32) -> bool {
    let ratio = anisotropy(t);
    if ratio > MAX_ISOTROPY_ERROR {
        return false;
    }
    // The scalar width is the wider one, `path_width · max`, and the narrowest direction's
    // true width is `path_width · max / ratio`.
    let widest = path_width * t.max_stretch();
    let error = widest - widest / ratio;
    !error.is_finite() || error <= MAX_SCALAR_WIDTH_ERROR
}

/// The ratio of the transform's larger singular value to its smaller — 1 exactly
/// for a similarity transform, growing with shear or uneven scale.
fn anisotropy(t: Transform) -> f32 {
    let max = t.max_stretch();
    let determinant = t.determinant().abs();
    if !max.is_finite() || max <= 0.0 || !determinant.is_finite() || determinant <= 0.0 {
        // Degenerate: no direction to vary a width along; the scalar route is as
        // good as any.
        return 1.0;
    }
    // |det| = product of the singular values, so min = |det| / max.
    (max * max) / determinant
}

fn kurbo_cap(cap: LineCap) -> kurbo::Cap {
    match cap {
        LineCap::Butt => kurbo::Cap::Butt,
        LineCap::Round => kurbo::Cap::Round,
        LineCap::Square => kurbo::Cap::Square,
    }
}

fn kurbo_join(join: LineJoin) -> kurbo::Join {
    match join {
        LineJoin::Miter => kurbo::Join::Miter,
        LineJoin::Round => kurbo::Join::Round,
        LineJoin::Bevel => kurbo::Join::Bevel,
    }
}

/// Cuts the dash pattern, in path space, or returns `None` for a solid stroke.
///
/// ISO 32000-2 §8.4.3.6: an empty dash array — or one whose lengths sum to
/// nothing — is a solid line. Zero-length dashes whose caps would show become
/// marks appended to `dots`, by the shared §8.5.3.2 rule.
fn dash(
    geometry: &Path,
    s: &Stroke,
    (width, substitute): (f32, Option<Transform>),
    dots: &mut Path,
    coverage: &mut f32,
) -> Option<Path> {
    if s.dash_array.is_empty() || s.dash_array.iter().sum::<f32>() <= 0.0 {
        return None;
    }
    let phase = f64::from(s.dash_phase);
    if let Some(showing) = pdf_render::dashes_showing_direction(&s.dash_array, s.cap) {
        let pattern: Vec<f64> = showing.iter().copied().map(f64::from).collect();
        // A zero-length dash the pattern places exactly at an open subpath's end is dispensed on
        // the extension `pdf-render` adds, decided there for all three backends (trap 2).
        let extended =
            pdf_render::extended_to_a_zero_dash_at_the_end(geometry, &s.dash_array, s.dash_phase);
        let source = bez(extended.as_ref().unwrap_or(geometry));
        let cut = from_bez(kurbo::dash(
            source.elements().iter().copied(),
            phase,
            &pattern,
        ));
        let marks = pdf_render::split_dash_marks(&cut, s.cap, width, substitute);
        dots.extend(marks.dots.commands());
        *coverage = marks.coverage;
        Some(marks.stroked)
    } else {
        let source = bez(geometry);
        let pattern: Vec<f64> = s.dash_array.iter().copied().map(f64::from).collect();
        Some(from_bez(kurbo::dash(
            source.elements().iter().copied(),
            phase,
            &pattern,
        )))
    }
}

/// A paint at `coverage` of its own alpha, for a mark ISO 32000-2 §10.7.4 states wider than the
/// document's own width.
///
/// The coverage a substituted mark gave up rides in the paint's alpha — §11.3.7.1 makes shape and
/// opacity one product — and this builder's `fill` takes no alpha beside the paint. Only a solid
/// colour is carried: [`encode`] withholds §10.7.4's substitution for every other paint, and a
/// dash pattern past `pdf_render::MAX_DASHES` under another paint is drawn at its whole coverage
/// (ADR 3334), which is what this returns for it.
fn faint(paint: raster_scene::Paint, coverage: f32) -> raster_scene::Paint {
    match paint {
        raster_scene::Paint::Solid(c) if coverage < 1.0 => {
            raster_scene::Paint::Solid(raster_scene::Color::new(c.r, c.g, c.b, c.a * coverage))
        }
        other => other,
    }
}

fn cap(cap: LineCap) -> raster_scene::LineCap {
    match cap {
        LineCap::Butt => raster_scene::LineCap::Butt,
        LineCap::Round => raster_scene::LineCap::Round,
        LineCap::Square => raster_scene::LineCap::Square,
    }
}

fn join(join: LineJoin) -> raster_scene::LineJoin {
    match join {
        LineJoin::Miter => raster_scene::LineJoin::Miter,
        LineJoin::Round => raster_scene::LineJoin::Round,
        LineJoin::Bevel => raster_scene::LineJoin::Bevel,
    }
}

fn bez(path: &Path) -> kurbo::BezPath {
    let mut out = kurbo::BezPath::new();
    let point = |p: Point| kurbo::Point::new(f64::from(p.x), f64::from(p.y));
    for command in path.commands() {
        match *command {
            PathCommand::MoveTo(p) => out.move_to(point(p)),
            PathCommand::LineTo(p) => out.line_to(point(p)),
            PathCommand::CurveTo(c1, c2, to) => out.curve_to(point(c1), point(c2), point(to)),
            PathCommand::Close => out.close_path(),
        }
    }
    out
}

/// Converts kurbo elements back to a display-list path. `kurbo` emits quadratics
/// only for input that had them, and this pipeline never produces any, so a
/// quadratic is elevated to the cubic through the same curve — the same handling
/// as `render-gpu`'s.
#[expect(
    clippy::cast_possible_truncation,
    reason = "every coordinate here originated as an f32 in this display list"
)]
fn from_bez(elements: impl Iterator<Item = kurbo::PathEl>) -> Path {
    let mut out = Path::new();
    let point = |p: kurbo::Point| Point::new(p.x as f32, p.y as f32);
    let mut current = kurbo::Point::ZERO;
    for element in elements {
        match element {
            kurbo::PathEl::MoveTo(p) => {
                current = p;
                out.push(PathCommand::MoveTo(point(p)));
            }
            kurbo::PathEl::LineTo(p) => {
                current = p;
                out.push(PathCommand::LineTo(point(p)));
            }
            kurbo::PathEl::QuadTo(c, p) => {
                let quad = kurbo::QuadBez::new(current, c, p).raise();
                current = p;
                out.push(PathCommand::CurveTo(
                    point(quad.p1),
                    point(quad.p2),
                    point(quad.p3),
                ));
            }
            kurbo::PathEl::CurveTo(c1, c2, p) => {
                current = p;
                out.push(PathCommand::CurveTo(point(c1), point(c2), point(p)));
            }
            kurbo::PathEl::ClosePath => out.push(PathCommand::Close),
        }
    }
    out
}
