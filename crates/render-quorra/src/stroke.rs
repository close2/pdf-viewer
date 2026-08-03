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
    if path.is_empty() {
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

    // §8.5.3.2's zero-length subpaths, split by the shared rule — in path space,
    // with the path-space width, as the shared helpers expect.
    let split = pdf_render::split_degenerate(path, s.cap, path_width);
    let geometry: &Path = split.as_ref().map_or(path.as_ref(), |d| &d.stroked);
    let mut dots: Path = split.as_ref().map_or_else(Path::new, |d| d.dots.clone());

    let dashed = dash(geometry, s, path_width, &mut dots);
    let solid: &Path = dashed.as_ref().unwrap_or(geometry);

    let at = enc.placed(transform);
    if !solid.is_empty() {
        if anisotropy(to_device) > MAX_ISOTROPY_ERROR {
            // The stroke is expanded here, in path space, where the transform
            // scales its width per direction as §8.4.3.2 asks — through the same
            // `kurbo::stroke` the Vello backend outlines with, so the corpus's
            // sheared pattern marks (issue2177, issue6769) get the CPU oracle's
            // geometry rather than the widest direction everywhere. quorra then
            // fills the outline; its own stroke lane is for the scalar case.
            let style = kurbo::Stroke::new(f64::from(path_width))
                .with_caps(kurbo_cap(s.cap))
                .with_join(kurbo_join(s.join))
                .with_miter_limit(f64::from(s.miter_limit.max(1.0)));
            // A quarter device pixel, expressed in path units — the same
            // flattening tolerance quorra itself draws with.
            let tolerance = f64::from((0.25 / to_device.max_stretch()).clamp(1e-4, 1.0));
            let outlined = from_bez(
                kurbo::stroke(
                    bez(solid).iter(),
                    &style,
                    &kurbo::StrokeOpts::default(),
                    tolerance,
                )
                .iter(),
            );
            let outline = enc.transient_outline(&outlined)?;
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
            quorra_paint,
            clip,
            blend_mode(blend),
            quorra_scene::Compose::SrcOver,
            mask,
        )?;
    }
    Ok(())
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
fn dash(geometry: &Path, s: &Stroke, width: f32, dots: &mut Path) -> Option<Path> {
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
        let marks = pdf_render::split_dash_marks(&cut, s.cap, width);
        dots.extend(marks.dots.commands());
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
