//! Translation from a resolved display list to a Vello scene.
//!
//! # Why this is not a straight loop
//!
//! `pdf-render`'s [`DisplayList`] is *flat*: every command names its clip by id, so
//! commands are independent and can be reordered or parallelised. Vello's scene is
//! *nested*: clips are a layer stack, pushed and popped around the commands they
//! affect.
//!
//! Translating therefore means re-nesting — walking the commands in painting order
//! and pushing or popping clip layers as the chain changes, like diffing two trees.
//! `render-cpu` faces the opposite problem: `tiny-skia` wants a flat coverage mask
//! per clip, not a stack.
//!
//! That neither backend consumes the display list directly is the evidence that the
//! flat form is the right neutral representation. Had the display list been shaped
//! like either library's model, the other backend would have paid for it, and the
//! two would no longer be comparable on identical input.

use pdf_render::display_list::Clip;
use pdf_render::{
    BackendError, BlendMode, ClipId, Color, Command, DisplayList, FillRule, LineCap, LineJoin,
    MAX_GROUP_DEPTH, Paint, Path, PathCommand, Stroke, Transform,
};
use vello::kurbo;
use vello::peniko;

use crate::GpuRasterError;

/// Converts a PDF matrix to a `kurbo` affine transform.
///
/// `kurbo::Affine::new` takes `[a, b, c, d, e, f]` in the same order and with the
/// same meaning as a PDF matrix, so this is a direct widening to `f64`. Pinned by a
/// test, because a transposition would misplace all geometry.
pub(crate) fn affine(t: Transform) -> kurbo::Affine {
    kurbo::Affine::new([
        f64::from(t.a),
        f64::from(t.b),
        f64::from(t.c),
        f64::from(t.d),
        f64::from(t.e),
        f64::from(t.f),
    ])
}

/// Converts a path to a `kurbo` Bézier path.
fn bez_path(p: &Path) -> kurbo::BezPath {
    let mut out = kurbo::BezPath::new();
    for command in p.commands() {
        match *command {
            PathCommand::MoveTo(p) => out.move_to((f64::from(p.x), f64::from(p.y))),
            PathCommand::LineTo(p) => out.line_to((f64::from(p.x), f64::from(p.y))),
            PathCommand::CurveTo(c1, c2, end) => out.curve_to(
                (f64::from(c1.x), f64::from(c1.y)),
                (f64::from(c2.x), f64::from(c2.y)),
                (f64::from(end.x), f64::from(end.y)),
            ),
            PathCommand::Close => out.close_path(),
        }
    }
    out
}

/// Converts a fill rule.
fn fill_rule(rule: FillRule) -> peniko::Fill {
    match rule {
        FillRule::NonZero => peniko::Fill::NonZero,
        FillRule::EvenOdd => peniko::Fill::EvenOdd,
    }
}

/// Converts a colour. Both sides use straight alpha with components in `0.0..=1.0`.
pub(crate) fn color(c: Color) -> peniko::Color {
    peniko::Color::new([c.r, c.g, c.b, c.a])
}

/// Converts a blend mode.
///
/// `peniko::Mix` carries exactly the sixteen PDF blend modes, so this mapping is
/// total. Composition is always source-over: PDF's blend mode selects the *mix*
/// function, while the Porter-Duff composite operator stays source-over.
fn blend_mode(mode: BlendMode) -> peniko::BlendMode {
    let mix = match mode {
        BlendMode::Normal => peniko::Mix::Normal,
        BlendMode::Multiply => peniko::Mix::Multiply,
        BlendMode::Screen => peniko::Mix::Screen,
        BlendMode::Overlay => peniko::Mix::Overlay,
        BlendMode::Darken => peniko::Mix::Darken,
        BlendMode::Lighten => peniko::Mix::Lighten,
        BlendMode::ColorDodge => peniko::Mix::ColorDodge,
        BlendMode::ColorBurn => peniko::Mix::ColorBurn,
        BlendMode::HardLight => peniko::Mix::HardLight,
        BlendMode::SoftLight => peniko::Mix::SoftLight,
        BlendMode::Difference => peniko::Mix::Difference,
        BlendMode::Exclusion => peniko::Mix::Exclusion,
        BlendMode::Hue => peniko::Mix::Hue,
        BlendMode::Saturation => peniko::Mix::Saturation,
        BlendMode::Color => peniko::Mix::Color,
        BlendMode::Luminosity => peniko::Mix::Luminosity,
    };
    peniko::BlendMode::new(mix, peniko::Compose::SrcOver)
}

/// Converts stroke parameters.
///
/// As in `render-cpu`, the miter limit is always set explicitly: `kurbo`'s default is
/// `4.0` where PDF's initial value is `10.0`.
fn stroke(s: &Stroke) -> kurbo::Stroke {
    let mut out = kurbo::Stroke::new(f64::from(s.width))
        .with_caps(match s.cap {
            LineCap::Butt => kurbo::Cap::Butt,
            LineCap::Round => kurbo::Cap::Round,
            LineCap::Square => kurbo::Cap::Square,
        })
        .with_join(match s.join {
            LineJoin::Miter => kurbo::Join::Miter,
            LineJoin::Round => kurbo::Join::Round,
            LineJoin::Bevel => kurbo::Join::Bevel,
        })
        .with_miter_limit(f64::from(s.miter_limit));

    if !s.dash_array.is_empty() {
        out = out.with_dashes(
            f64::from(s.dash_phase),
            s.dash_array.iter().copied().map(f64::from),
        );
    }
    out
}

/// The three spaces a command's geometry and its paint are stated in.
///
/// Grouped because a paint and the shape it fills are positioned differently — that is
/// the whole point of [`brush_for`] — so both mappings travel together everywhere a
/// command is encoded, and passing them separately made the call a row of unlabelled
/// transforms.
#[derive(Debug, Clone, Copy)]
struct Spaces {
    /// Path space to device space: what the shape is drawn under.
    at: kurbo::Affine,
    /// Page space to device space, for geometry a paint resolves to directly.
    to_device: Transform,
    /// Page space to path space, which is where a brush transform is read.
    page_to_path: Transform,
}

impl Spaces {
    /// Builds the three spaces for a command drawn under `transform`.
    ///
    /// # Errors
    ///
    /// Returns [`GpuRasterError::UnsupportedPaint`] when `transform` is singular. A path
    /// under a singular transform has collapsed to a line or a point, so there is no
    /// space left to position a paint in, and reporting beats placing a gradient
    /// arbitrarily.
    fn new(transform: Transform, to_device: Transform) -> Result<Self, GpuRasterError> {
        Ok(Self {
            at: affine(transform.then(to_device)),
            to_device,
            page_to_path: transform.invert().ok_or_else(|| {
                GpuRasterError::UnsupportedPaint(format!("singular transform {transform:?}"))
            })?,
        })
    }
}

/// Encodes one fill command.
///
/// Separate from the dispatch loop because a fill has three shapes: an ordinary brush, a
/// brush under a blend layer, and a mesh, which is not a brush at all.
fn encode_fill(
    scene: &mut vello::Scene,
    shape: &kurbo::BezPath,
    spaces: Spaces,
    rule: peniko::Fill,
    paint: &Paint,
    blend: BlendMode,
) -> Result<(), GpuRasterError> {
    let at = spaces.at;

    // A mesh carries a colour per triangle corner, which no brush can express, so it is
    // drawn triangle by triangle inside a layer clipped to the shape.
    if let Paint::Shading(shading) = paint
        && let pdf_render::ShadingKind::Mesh { triangles } = &shading.kind
    {
        scene.push_layer(rule, blend_mode(blend), 1.0, at, shape);
        crate::shading::fill_mesh(scene, triangles, shading.transform.then(spaces.to_device));
        scene.pop_layer();
        return Ok(());
    }

    let (brush, brush_at) = brush_for(paint, spaces.page_to_path)?;

    // A non-normal blend mode needs its own layer: Vello composites a layer against its
    // backdrop with the layer's blend mode, and there is no per-draw blend parameter. The
    // layer is clipped to the shape being drawn, so the blend applies exactly where paint
    // lands.
    if blend == BlendMode::Normal {
        scene.fill(rule, at, &brush, brush_at, shape);
    } else {
        scene.push_layer(rule, blend_mode(blend), 1.0, at, shape);
        scene.fill(rule, at, &brush, brush_at, shape);
        scene.pop_layer();
    }
    Ok(())
}

/// Turns a paint into a Vello brush and the transform that positions it.
///
/// A shading's transform goes on the *brush*, not on the shape: a pattern is anchored to
/// the page rather than to the path being filled. `page_to_path` carries the shading into
/// the path's space, which is the space Vello reads a brush transform in — see
/// [`crate::shading::brush`].
fn brush_for(
    paint: &Paint,
    page_to_path: Transform,
) -> Result<(peniko::Brush, Option<kurbo::Affine>), GpuRasterError> {
    match paint {
        Paint::Solid(colour) => Ok((peniko::Brush::Solid(color(*colour)), None)),
        Paint::Shading(shading) => crate::shading::brush(shading, page_to_path).ok_or_else(|| {
            // A mesh is drawn by the caller; anything else reaching here is a kind this
            // backend cannot express. Reporting keeps the two backends honestly different
            // rather than quietly so: the comparison harness excludes a page a backend
            // says it cannot draw, instead of blaming the difference on the GPU.
            GpuRasterError::UnsupportedPaint(format!("{:?}", shading.kind))
        }),
        other => Err(GpuRasterError::UnsupportedPaint(format!("{other:?}"))),
    }
}

/// Builds a Vello scene from a display list.
///
/// `to_device` maps page space to device space and is applied on top of each
/// command's own transform.
///
/// # Errors
///
/// Returns [`GpuRasterError`] for a command or paint variant this backend does not
/// implement, or for a clip chain that is dangling or cyclic. Unsupported input is an
/// error rather than a skipped command: silently omitting geometry would hand the
/// comparison harness a plausible-looking wrong image instead of a failure.
pub(crate) fn build(
    list: &DisplayList,
    to_device: Transform,
) -> Result<vello::Scene, GpuRasterError> {
    let mut scene = vello::Scene::new();
    encode(&mut scene, list, list.commands(), to_device, 0)?;
    Ok(scene)
}

/// Encodes one sequence of commands, opening and closing clip layers as the chain changes.
///
/// `depth` counts enclosing transparency groups; see [`MAX_GROUP_DEPTH`].
///
/// # Errors
///
/// As [`build`], plus [`GpuRasterError::Target`] for groups nested past the bound.
fn encode(
    scene: &mut vello::Scene,
    list: &DisplayList,
    commands: &[Command],
    to_device: Transform,
    depth: usize,
) -> Result<(), GpuRasterError> {
    // The clip chain currently pushed as layers, root-first. A group's elements start with
    // an empty one rather than inheriting the caller's: reconciling against a chain opened
    // outside the group's own layer could pop that layer, and re-pushing a clip already in
    // force costs an intersection with itself and changes nothing.
    let mut open: Vec<ClipId> = Vec::new();

    for command in commands {
        let wanted = resolve_chain(list, command.clip())?;
        reconcile_layers(scene, list, &mut open, &wanted, to_device)?;

        match command {
            Command::Fill {
                path,
                transform,
                fill_rule: rule,
                paint,
                blend,
                ..
            } => {
                encode_fill(
                    scene,
                    &bez_path(path),
                    Spaces::new(*transform, to_device)?,
                    fill_rule(*rule),
                    paint,
                    *blend,
                )?;
            }
            Command::Stroke {
                path,
                transform,
                stroke: s,
                paint,
                blend,
                ..
            } => {
                let shape = bez_path(path);
                let spaces = Spaces::new(*transform, to_device)?;
                let at = spaces.at;
                let (brush, brush_at) = brush_for(paint, spaces.page_to_path)?;
                let style = stroke(s);

                // The stroke width is in the command's own coordinate space, and the
                // transform scales it along with the geometry, as PDF specifies.
                if *blend == BlendMode::Normal {
                    scene.stroke(&style, at, &brush, brush_at, &shape);
                } else {
                    // Clipping the layer to the *unstroked* path would cut the stroke
                    // in half, since a stroke straddles its path. The stroke outline
                    // is used as the layer's clip shape instead.
                    let outline =
                        kurbo::stroke(shape.iter(), &style, &kurbo::StrokeOpts::default(), 0.1);
                    scene.push_layer(peniko::Fill::NonZero, blend_mode(*blend), 1.0, at, &outline);
                    scene.stroke(&style, at, &brush, brush_at, &shape);
                    scene.pop_layer();
                }
            }
            Command::Image {
                image,
                transform,
                alpha,
                blend,
                ..
            } => draw_image(scene, image, transform.then(to_device), *alpha, *blend)?,
            Command::Group {
                commands,
                alpha,
                blend,
                ..
            } => {
                let depth = depth.saturating_add(1);
                if depth > MAX_GROUP_DEPTH {
                    return Err(BackendError::GroupsTooDeep {
                        depth,
                        limit: MAX_GROUP_DEPTH,
                    }
                    .into());
                }
                // A Vello layer *is* a transparency group: its contents are composited
                // onto a transparent surface and the result is painted once, under the
                // layer's alpha and blend mode. That is §11.4.1's definition, so this
                // command translates rather than being implemented.
                //
                // The layer's shape is the page, because the group itself clips nothing —
                // the clip in force is already open as a layer of its own, and nothing
                // outside the page bounds can reach a target built from them.
                scene.push_layer(
                    peniko::Fill::NonZero,
                    blend_mode(*blend),
                    alpha.clamp(0.0, 1.0),
                    affine(to_device),
                    &kurbo::Rect::new(
                        f64::from(list.page_bounds().min.x),
                        f64::from(list.page_bounds().min.y),
                        f64::from(list.page_bounds().max.x),
                        f64::from(list.page_bounds().max.y),
                    ),
                );
                encode(scene, list, commands, to_device, depth)?;
                scene.pop_layer();
            }
            other => {
                return Err(GpuRasterError::UnsupportedCommand(format!("{other:?}")));
            }
        }
    }

    // Close every layer left open, or the scene is malformed.
    for _ in 0..open.len() {
        scene.pop_layer();
    }

    Ok(())
}

/// Draws one image over the unit square its command names.
///
/// Split out of [`build`] because Vello wants an image's brush, its sampler and its blend
/// layer set up together, and because it is the one command whose *sampler* is a decision
/// rather than a translation — see [`pdf_render::Image::is_smoothed`].
fn draw_image(
    scene: &mut vello::Scene,
    image: &pdf_render::Image,
    placement: Transform,
    alpha: f32,
    blend: BlendMode,
) -> Result<(), GpuRasterError> {
    if !image.is_consistent() {
        return Err(GpuRasterError::InvalidImage {
            width: image.width,
            height: image.height,
            bytes: image.data.len(),
        });
    }

    // The same area averaging the CPU backend applies, from the same function, so the two
    // cannot disagree about a deeply reduced image — Vello's own filters read a fixed
    // neighbourhood exactly as `tiny-skia`'s do. ADR 0025.
    let reduced = image.area_averaged(placement);
    let image = reduced.as_ref().unwrap_or(image);

    // Vello draws an image over a rectangle in *pixel* units, so the transform must map
    // pixel space onto the unit square before the command's own transform. The vertical flip
    // is because PDF's y-up space puts the image's first row at the top of the unit square.
    let width = f64::from(image.width);
    let height = f64::from(image.height);
    let to_unit = kurbo::Affine::new([1.0 / width, 0.0, 0.0, -1.0 / height, 0.0, 1.0]);
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "composing two affine transforms is floating-point multiplication, which \
                  cannot overflow"
    )]
    let at = affine(placement) * to_unit;

    let data = peniko::ImageData {
        data: peniko::Blob::new(std::sync::Arc::new(image.data.to_vec())),
        format: peniko::ImageFormat::Rgba8,
        // Straight alpha, matching `pdf_render::Image`'s documented format.
        alpha_type: peniko::ImageAlphaType::Alpha,
        width: image.width,
        height: image.height,
    };
    // §8.9.5.3, through the same decision the CPU backend makes: `Medium` is bilinear and
    // `Low` is nearest-neighbour, and which one an image gets is `is_smoothed`'s business so
    // that the two backends cannot disagree about a magnified image the document asked not
    // to smooth.
    let quality = if image.is_smoothed(placement) {
        peniko::ImageQuality::Medium
    } else {
        peniko::ImageQuality::Low
    };
    let brush = peniko::ImageBrush::new(data)
        .with_alpha(alpha.clamp(0.0, 1.0))
        .with_quality(quality);

    if blend == BlendMode::Normal {
        scene.draw_image(&brush, at);
    } else {
        let unit = kurbo::Rect::new(0.0, 0.0, width, height);
        scene.push_layer(peniko::Fill::NonZero, blend_mode(blend), 1.0, at, &unit);
        scene.draw_image(&brush, at);
        scene.pop_layer();
    }
    Ok(())
}

/// Returns the clip chain for a command, root-first, or an empty chain if unclipped.
///
/// Walked iteratively with a seen-set: a deep or cyclic chain is reachable from a
/// malformed document, and recursion would exhaust the stack.
fn resolve_chain(list: &DisplayList, clip: Option<ClipId>) -> Result<Vec<ClipId>, GpuRasterError> {
    let mut chain = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut current = clip;

    while let Some(this) = current {
        if !seen.insert(this) {
            return Err(GpuRasterError::CyclicClip(this));
        }
        let clip = list.clip(this).ok_or(GpuRasterError::UnknownClip(this))?;
        chain.push(this);
        current = clip.parent;
    }

    chain.reverse();
    Ok(chain)
}

/// Pops and pushes clip layers so that `open` ends up equal to `wanted`.
///
/// Only the differing suffix is touched. Because clips are hierarchical and commands
/// arrive in painting order, consecutive commands usually share a long prefix, so in
/// practice this pushes nothing at all for most commands.
fn reconcile_layers(
    scene: &mut vello::Scene,
    list: &DisplayList,
    open: &mut Vec<ClipId>,
    wanted: &[ClipId],
    to_device: Transform,
) -> Result<(), GpuRasterError> {
    let shared = open.iter().zip(wanted).take_while(|(a, b)| a == b).count();

    for _ in shared..open.len() {
        scene.pop_layer();
    }
    open.truncate(shared);

    for &id in wanted.iter().skip(shared) {
        let clip: &Clip = list.clip(id).ok_or(GpuRasterError::UnknownClip(id))?;
        scene.push_clip_layer(
            fill_rule(clip.fill_rule),
            affine(clip.transform.then(to_device)),
            &bez_path(&clip.path),
        );
        open.push(id);
    }

    Ok(())
}

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "the matrix mapping must be exact; an approximate comparison would not \
              catch a transposition"
)]
mod tests {
    use super::affine;
    use pdf_render::{Point, Transform};

    /// Asserts that a PDF matrix and `kurbo::Affine` agree on where a point lands.
    /// A shear is used because a pure scale or translation would pass even under a
    /// transposition.
    #[test]
    fn affine_agrees_with_our_transform_on_a_sheared_point() {
        let ours = Transform::new(2.0, 0.5, -0.25, 3.0, 10.0, -4.0);
        let point = Point::new(7.0, 11.0);

        let expected = ours.apply(point);
        let actual = affine(ours) * vello::kurbo::Point::new(7.0, 11.0);

        // Compared in f64: our transform computes in f32, so widening the expected
        // value is exact, whereas narrowing kurbo's f64 result would discard the very
        // precision difference being checked.
        assert_eq!(actual.x, f64::from(expected.x));
        assert_eq!(actual.y, f64::from(expected.y));
    }
}
