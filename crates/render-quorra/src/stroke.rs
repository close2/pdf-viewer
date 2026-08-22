//! Strokes: what quorra draws, and what is settled here first.
//!
//! quorra expands caps, joins and miters itself from a resolved **device** width —
//! `RENDER_LIBRARY.md` section 4.5's contract — but it does not dash, and it does not
//! re-take the decisions `pdf-render` already owns. So this module runs the same
//! shared machinery the other backends run, in the same order:
//!
//! 1. [`pdf_render::split_degenerate`]: ISO 32000-2 §8.5.3.2's zero-length
//!    subpaths become dots (or nothing), decided by one implementation.
//! 2. [`kurbo::dash`]: the dash pattern is cut in path space, by the dasher
//!    `render-gpu` uses through vello — identical dashes by construction — with
//!    §8.5.3.2's zero-length *dashes* turned into marks through
//!    [`pdf_render::dashes_showing_direction`] and
//!    [`pdf_render::split_dash_marks`].
//! 3. quorra strokes the surviving subpaths solid and fills the dots.
//!
//! **And one decision taken before any of that: §8.3.4 NOTE 3's matrix with no inverse.** quorra
//! needs no inverse to place a paint — it anchors one in page space, where `tiny-skia` and Vello
//! each want the draw transform undone — so what [`pdf_render::paint_space`] gives this module is
//! only the *refusal*, and taking it from there is what keeps the three backends refusing the same
//! marks (viewer trap 2). Without it such a transform reached `path_width * max_stretch` below,
//! which is a width of zero, and the **scene** refused with `InvalidStroke`: the whole frame lost
//! for one mark whose path has collapsed onto a line or a point and covers no area anyway. That is
//! `doc/todo/11` item 8 in quorra's own spelling, and ADR 0482 has the reading.

use std::sync::Arc;

use pdf_render::{
    BlendMode, ClipId, LineCap, LineJoin, Paint, Path, PathCommand, Point, SoftMaskId, Stroke,
    Transform,
};
use quorra_scene::SceneBuilder;

use crate::QuorraRasterError;
use crate::scene::{Encoder, blend_mode, colour};

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
    // Stated for all three backends in `pdf-render`; the module comment says what it cost quorra.
    if path.is_empty() || pdf_render::paint_space(transform).is_none() {
        return Ok(());
    }
    let crate::scene::Admitted::Chain(clip) = enc.clip_chain(builder, clip)? else {
        return Ok(()); // the clip admits nothing
    };
    let mask = enc.mask_id(builder, mask)?;
    let quorra_paint = match paint {
        Paint::Solid(c) => quorra_scene::Paint::Solid(colour(*c)),
        Paint::Shading(shading) => match enc.shading_paint(shading)? {
            crate::scene::ShadedPaint::Ready(paint) => paint,
            crate::scene::ShadedPaint::Sampled => {
                return Err(QuorraRasterError::Unsupported(
                    "a sampled shading painting a stroke".into(),
                ));
            }
            // Nothing visible to paint with — the stroke marks nothing, as on
            // the sibling backends.
            crate::scene::ShadedPaint::Nothing => return Ok(()),
        },
        other => {
            return Err(QuorraRasterError::Unsupported(format!("paint {other:?}")));
        }
    };

    // §8.4.3.2 with §10.7.5, resolved by the shared method — which answers in
    // *path* units despite its name (the other backends scale it through the draw
    // transform). quorra expands on device-space geometry with one scalar width,
    // which is exact for a similarity transform and exactly wrong for any other:
    // a sheared or unevenly-scaled stroke must vary its device width with
    // direction (§8.4.3.2's own note), and a scalar cannot. So the transform's
    // anisotropy decides the route below.
    let to_device = transform.then(enc.target().transform);
    let path_width = s.device_width(to_device);
    let width = path_width * to_device.max_stretch();

    // §10.7.4's substitution for a mark whose area is under what the raster can hold, decided
    // in `pdf-render` for all three backends (trap 2). Two conditions withhold it, and both are
    // about the coverage having nowhere but the alpha to go: inside §11.4.6's knockout an
    // element replaces its backdrop within its own *shape*, which quorra reads off the coverage
    // a mark is drawn with; and this builder's `fill` takes no alpha of its own, so a coverage
    // can only be carried in a solid colour. A mark painted by a shading therefore keeps the
    // shape §8.5.3.2 states, and the cross-backend gate is what would show such a page.
    let substitute =
        (!enc.inside_knockout() && matches!(paint, Paint::Solid(_))).then_some(to_device);
    // §8.5.3.2's zero-length subpaths, split by the shared rule — in path space,
    // with the path-space width, as the shared helpers expect.
    let split = pdf_render::split_degenerate(path, s.cap, path_width, substitute);
    let geometry: &Path = split.as_ref().map_or(path.as_ref(), |d| &d.stroked);
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
    let solid: &Path = dashed.as_ref().unwrap_or(geometry);

    let at = enc.placed(transform);
    if !solid.is_empty() {
        if anisotropy(to_device) > MAX_ISOTROPY_ERROR {
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
                quorra_scene::FillRule::NonZero,
                quorra_paint,
                clip,
                blend_mode(blend),
                quorra_scene::Compose::SrcOver,
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
            builder.stroke(
                outline,
                at,
                quorra_scene::Stroke {
                    width,
                    cap: cap(s.cap),
                    join: join(s.join),
                    // §8.4.3.5 defines the limit as a ratio of at least 1; a
                    // smaller value from a malformed file behaves as the smallest
                    // legal one.
                    miter_limit: s.miter_limit.max(1.0),
                },
                quorra_paint,
                clip,
                blend_mode(blend),
                mask,
            )?;
        }
    }
    if !dots.is_empty() {
        let outline = enc.transient_outline(&dots)?;
        builder.fill(
            outline,
            at,
            quorra_scene::FillRule::NonZero,
            faint(quorra_paint, coverage),
            clip,
            blend_mode(blend),
            quorra_scene::Compose::SrcOver,
            mask,
        )?;
    }
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
/// quorra widens from one scalar device width, which cannot. So the expansion happens here,
/// through the same `kurbo::stroke` the Vello backend outlines with — which is what gives the
/// corpus's sheared pattern marks (issue2177, issue6769) the CPU oracle's geometry rather than the
/// widest direction everywhere — and quorra fills the result.
///
/// **Cached when the geometry going in is the display list's own path, transient when this frame
/// computed it.** That is the same division the scalar branch makes, and it is here for a reason
/// stated the other way round: an identifier that moves between two renders of one unchanged page
/// makes every atlas key quorra derives from it foreign, and its atlas then repacks at period two
/// for ever (ADR 0402). A dashed or degenerate stroke has no stable source to key on, so it stays
/// a transient — and the expansion is a closure so that a cache hit skips `kurbo::stroke` as well
/// as the upload.
fn expanded(
    enc: &mut Encoder<'_>,
    parts: Expansion<'_>,
) -> Result<quorra_scene::OutlineId, QuorraRasterError> {
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
    // A quarter device pixel, expressed in path units — the same flattening tolerance quorra
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
    let source = bez(geometry);
    let phase = f64::from(s.dash_phase);
    if let Some(showing) = pdf_render::dashes_showing_direction(&s.dash_array, s.cap) {
        let pattern: Vec<f64> = showing.iter().copied().map(f64::from).collect();
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
fn faint(paint: quorra_scene::Paint, coverage: f32) -> quorra_scene::Paint {
    match paint {
        quorra_scene::Paint::Solid(c) if coverage < 1.0 => {
            quorra_scene::Paint::Solid(quorra_scene::Color::new(c.r, c.g, c.b, c.a * coverage))
        }
        other => other,
    }
}

fn cap(cap: LineCap) -> quorra_scene::LineCap {
    match cap {
        LineCap::Butt => quorra_scene::LineCap::Butt,
        LineCap::Round => quorra_scene::LineCap::Round,
        LineCap::Square => quorra_scene::LineCap::Square,
    }
}

fn join(join: LineJoin) -> quorra_scene::LineJoin {
    match join {
        LineJoin::Miter => quorra_scene::LineJoin::Miter,
        LineJoin::Round => quorra_scene::LineJoin::Round,
        LineJoin::Bevel => quorra_scene::LineJoin::Bevel,
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
