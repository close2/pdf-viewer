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
    let fitted = fitted_for_dashing(enc, path, transform, s);
    let fitted_path = fitted.as_ref().map(|fit| Arc::new(fit.path.clone()));
    let path = fitted_path.as_ref().unwrap_or(path);
    let s = fitted.as_ref().map_or(s, |fit| &fit.stroke);
    let mask = enc.mask_id(builder, mask)?;
    let Some(raster_paint) = raster_paint(enc, paint)? else {
        return Ok(()); // nothing visible to paint with
    };

    // §8.4.3.2 with §10.7.5, resolved by the shared method — which answers in
    // *path* units despite its name (the other backends scale it through the draw
    // transform). raster expands on device-space geometry with one scalar width,
    // which is exact for a similarity transform and exactly wrong for any other:
    // a sheared or unevenly-scaled stroke must vary its device width with
    // direction (§8.4.3.2's own note), and a scalar cannot. So the transform's
    // anisotropy decides the route below.
    let to_device = transform.then(enc.target().transform);
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
    if !dots.is_empty() || coverage < 1.0 {
        // §8.5.3.2's dots and zero-length dash marks are sized — and §10.7.4's
        // substitution for them decided — on this view's pixel grid, so a scene
        // holding any is true only here (ADR 0702). **The splits alone are not that**
        // (ADR 0705): which subpaths are degenerate is the path's own geometry, the
        // dash pattern cuts in path space with no width in the arithmetic, and the
        // stroke that survives both carries its scene-space width to raster, which
        // resolves §8.4.3.2 and §10.7.5 per placement (ADR 0701). So a dashed table
        // border or a stroke that merely *lost* its degenerate subpaths keeps its
        // scene across every zoom, and only marks actually made consume the view.
        enc.consume_view();
    }
    let solid: &Path = dashed.as_ref().unwrap_or(geometry);

    let at = enc.placed(transform);
    if !solid.is_empty() {
        if anisotropy(to_device) > MAX_ISOTROPY_ERROR {
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
                    from_source: dashed.is_none() && split.is_none(),
                },
            )?;
            builder.fill(
                outline,
                at,
                raster_scene::FillRule::NonZero,
                raster_paint,
                clip,
                blend_mode(blend),
                compose(blend),
                mask,
            )?;
        } else {
            // The untouched common case keeps the path's `Arc` identity, so a
            // glyph stroked a thousand times uploads once; computed geometry is
            // per-frame.
            let outline = if dashed.is_none() && split.is_none() {
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
                    raster_paint,
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
        &dots,
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
fn fitted_for_dashing(
    enc: &mut Encoder<'_>,
    path: &Path,
    transform: Transform,
    s: &Stroke,
) -> Option<pdf_render::GridFit> {
    if s.dash_array.is_empty() {
        return None;
    }
    let fitted = pdf_render::grid_fit(path, s, transform.then(enc.target().transform))?;
    enc.consume_view();
    Some(fitted)
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
        enc.keep_dot(dot);
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
fn raster_paint(
    enc: &mut Encoder<'_>,
    paint: &Paint,
) -> Result<Option<raster_scene::Paint>, QuorraRasterError> {
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
/// route takes over: the ratio of the transform's two singular values, allowed to
/// reach 1% — under the antialiasing floor every gate already tolerates, and far
/// under the 1.9× that made issue6769's bar visibly fat.
const MAX_ISOTROPY_ERROR: f32 = 1.01;

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
/// colour is reached: [`encode`] withholds the substitution for every other paint, so `coverage`
/// is 1 here whenever the paint is not one.
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
