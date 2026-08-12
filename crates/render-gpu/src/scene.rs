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
    MAX_GROUP_DEPTH, Paint, Path, PathCommand, Point, Stroke, TargetSpec, Transform,
};
use vello::kurbo;
use vello::peniko;

use crate::GpuRasterError;
use crate::soft_mask::SoftMaskRasters;

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

/// How an element of a group combines with the elements drawn before it.
///
/// ISO 32000-2 §11.4.6's two answers, and the same type as `render-cpu`'s `Compose` because
/// the display list states one rule and two backends have to reach it. A knockout element
/// "shall be composited with the group's initial backdrop rather than with the stack of
/// preceding elements in the group", and the backdrop a group is built on is transparent —
/// so the element replaces what is under it within its own shape, which is Porter-Duff
/// Source, and its own blend mode has nothing to blend against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Compose {
    /// §11.4.5: an element paints over the group's accumulated result.
    Over,
    /// §11.4.6: an element replaces it within its own coverage.
    Knockout,
}

impl Compose {
    /// The layer an element is drawn inside, or `None` to draw it straight into the scene.
    ///
    /// Vello composites a *layer* with its backdrop and has no per-draw blend parameter, so
    /// a blend mode needs one. A knockout element needs none: what it needs is [`knock_out`]
    /// run before it, after which it is painted over an area that has been emptied for it
    /// and its own blend mode is irrelevant — its backdrop is transparent.
    fn layer(self, blend: BlendMode) -> Option<peniko::BlendMode> {
        match self {
            Self::Knockout => None,
            Self::Over if blend == BlendMode::Normal => None,
            Self::Over => Some(blend_mode(blend)),
        }
    }
}

/// The refusal a group inside a knockout group meets.
///
/// Such a group would have to be composited by its *shape*, which arrives at a backend as
/// one alpha channel and cannot be told from its opacity. `pdf-model` does not build that
/// display list — see `Command::Group`'s `knockout` — and erroring keeps the assumption from
/// becoming a silent approximation if it ever does.
fn nested_group_in_knockout() -> GpuRasterError {
    GpuRasterError::UnsupportedCommand(
        "a group inside a knockout group has a shape this backend cannot separate from its \
         opacity (ISO 32000-2 §11.4.6)"
            .to_owned(),
    )
}

/// §11.4.4's non-isolated group, which this backend cannot build.
///
/// A Vello layer always begins fully transparent — §11.4.5's initial backdrop — and a scene
/// cannot read what it has drawn so far, so there is no way to seed one with the page a
/// non-isolated group's elements have to composite onto. The frame goes to the CPU backend;
/// drawing the isolated group this is not would be a plausible wrong picture, which is what
/// [`GpuRasterError`] exists to prevent.
fn non_isolated_group() -> GpuRasterError {
    GpuRasterError::UnsupportedCommand(
        "a non-isolated transparency group: a Vello layer begins transparent and its \
         elements have to composite onto the page behind it (ISO 32000-2 §11.4.4, §11.4.5)"
            .to_owned(),
    )
}

/// Empties the area an element is about to knock out (§11.4.6).
///
/// # Why not `Compose::Copy`, which is the rule itself
///
/// Porter-Duff Source *is* what a knockout element does, and `render-cpu` says exactly that
/// to `tiny-skia`. Vello cannot: a layer's compose runs over the layer's whole bounding box
/// with the clip's coverage applied to the *source*, so `Compose::Copy` writes `area × src`
/// everywhere in the box — which erases the destination wherever the shape does not reach,
/// out to the box's edge. Measured, not assumed: the first version of this differed from the
/// CPU backend along a whole rectangle edge, one row outside the shape.
///
/// `Compose::DestOut` is safe in the same place — at zero coverage it leaves the destination
/// exactly — so the knockout is done in two steps: empty the shape, then paint the element
/// over the emptied area with ordinary source-over.
///
/// # What the two steps cost, exactly
///
/// Where coverage is 0 or 1 the pair is the clause's arithmetic to the bit. At a partially
/// covered pixel the destination keeps `(1 − a)(1 − a·αs)` of itself where §11.4.6 asks for
/// `(1 − a)`, `a` being the coverage and `αs` the element's own alpha — the second factor is
/// the source-over of the second step, which the one-step Source form does not have. It
/// vanishes at both ends of the range and is bounded by a quarter of the destination at
/// `a = αs = ½`, so it is an antialiasing difference along an element's outline and nothing
/// more. The cross-backend scene draws a diagonal edge inside a knockout group precisely so
/// that this is measured rather than assumed.
fn knock_out(
    scene: &mut vello::Scene,
    at: kurbo::Affine,
    rule: peniko::Fill,
    shape: &impl kurbo::Shape,
) {
    scene.push_layer(
        rule,
        peniko::BlendMode::new(peniko::Mix::Normal, peniko::Compose::DestOut),
        1.0,
        at,
        shape,
    );
    scene.fill(
        rule,
        at,
        &peniko::Brush::Solid(peniko::color::AlphaColor::new([0.0, 0.0, 0.0, 1.0])),
        None,
        shape,
    );
    scene.pop_layer();
}

/// Converts stroke parameters, resolving the width against the device.
///
/// The width is [`Stroke::device_width`]'s rather than the field's, which is not a
/// refinement here but the whole of ISO 32000-2 §8.4.3.2 on this backend: `kurbo` expands
/// a zero-width stroke into an empty outline, so a `0 w` line — the standard's "thinnest
/// line that can be rendered at device resolution" — drew nothing at all until the
/// nineteenth session. §10.7.5's stroke adjustment arrives through the same call.
///
/// As in `render-cpu`, the miter limit is always set explicitly: `kurbo`'s default is
/// `4.0` where PDF's initial value is `10.0`.
fn stroke(s: &Stroke, to_device: Transform) -> kurbo::Stroke {
    let mut out = kurbo::Stroke::new(f64::from(s.device_width(to_device)))
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

/// Turns a dash pattern's zero-length dashes into marks, ISO 32000-2 §8.5.3.2.
///
/// The clause is explicit that a dash of no length is *not* the degenerate subpath of
/// [`pdf_render::split_degenerate`], and gets the opposite answer under a projecting square
/// cap:
///
/// > This rule shall apply only to zero-length subpaths of the path being stroked, and not
/// > to zero-length dashes in a dash pattern of a non-degenerate subpath. In the latter
/// > case, the line caps shall always be painted, since their orientation is determined by
/// > the direction of the underlying path except in the case of a degenerate subpath.
///
/// So `[0 6] 0 d 1 J S` is a dotted line, and `kurbo` expands a dash of no length into an
/// empty outline — the `0 w` hairline defect of the nineteenth session in a second place, a
/// rasteriser convention standing in for a clause nobody had written down.
///
/// Returns `None` — leaving the caller to hand Vello the pattern as usual — unless it holds a
/// zero-length dash whose cap would show. Where it does, the path is dashed here with the
/// same `kurbo::dash` Vello uses internally, the marks go into `dots`, and what comes back is
/// the remaining dashes, to be stroked solid.
fn zero_length_dashes(
    shape: &kurbo::BezPath,
    s: &Stroke,
    width: f32,
    dots: &mut Path,
) -> Option<kurbo::BezPath> {
    let pattern = pdf_render::dashes_showing_direction(&s.dash_array, s.cap)?;
    let pattern: Vec<f64> = pattern.iter().copied().map(f64::from).collect();
    let dashed = kurbo::dash(shape.iter(), f64::from(s.dash_phase), &pattern);
    let split = pdf_render::split_dash_marks(&from_bez_path(dashed), s.cap, width);
    dots.extend(split.dots.commands());
    Some(bez_path(&split.stroked))
}

/// Converts a `kurbo` path back to the display list's own, for [`zero_length_dashes`].
///
/// The one place geometry travels back out of the rasteriser's library, because the rule
/// applied to a dashed path is stated in `pdf-render` so that both backends apply the same
/// one. `kurbo` emits quadratics only for input that had them, and this pipeline never
/// produces any, so a quadratic is elevated to the cubic through the same curve.
#[expect(
    clippy::cast_possible_truncation,
    reason = "every coordinate here originated as an f32 in this display list"
)]
fn from_bez_path(elements: impl Iterator<Item = kurbo::PathEl>) -> Path {
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

/// Encodes a stroked path, including the marks its own geometry has no length to make.
///
/// Split out of [`encode`] because ISO 32000-2 §8.5.3.2 turns one command into two draws —
/// the subpaths that span a distance are stroked, and the ones that do not are *filled*, as
/// circles this crate states rather than as whatever `kurbo`'s caps would have produced.
fn encode_stroke(
    scene: &mut vello::Scene,
    path: &Path,
    spaces: Spaces,
    s: &Stroke,
    to_path: Transform,
    paint: &Paint,
    (compose, blend): (Compose, BlendMode),
) -> Result<(), GpuRasterError> {
    let at = spaces.at;
    let layer = compose.layer(blend);
    let (brush, brush_at) = brush_for(paint, spaces.page_to_path)?;
    let width = s.device_width(to_path);

    // §8.5.3.2's two rules about a stroke with no length, neither of which `kurbo` answers:
    // it drops a contour that expanded to nothing, so a dot and a dotted line both came out
    // blank on this backend.
    let split = pdf_render::split_degenerate(path, s.cap, width);
    let geometry = split.as_ref().map_or(path, |d| &d.stroked);
    let mut dots = split.as_ref().map_or_else(Path::new, |d| d.dots.clone());
    let shape = bez_path(geometry);
    let style = stroke(s, to_path);
    let (shape, style) = match zero_length_dashes(&shape, s, width, &mut dots) {
        // The dashes have already been dispensed, so what is left is stroked solid. The
        // width is the resolved one either way.
        Some(remainder) => {
            let mut solid = style;
            solid.dash_pattern = kurbo::Dashes::new();
            (remainder, solid)
        }
        None => (shape, style),
    };
    let dots = bez_path(&dots);

    // The stroke width is in the command's own coordinate space, and the transform scales it
    // along with the geometry, as PDF specifies.
    let draw = |scene: &mut vello::Scene| {
        if !shape.is_empty() {
            scene.stroke(&style, at, &brush, brush_at, &shape);
        }
        if !dots.is_empty() {
            scene.fill(peniko::Fill::NonZero, at, &brush, brush_at, &dots);
        }
    };
    // Clipping to the *unstroked* path would cut the stroke in half, since a stroke
    // straddles its path. The stroke's own outline is the shape both a blend layer and
    // §11.4.6's knockout need, with the dots — which are already outlines — beside it, so
    // that one shape carries the whole object and §11.6.2 composites it once.
    let outline = |shape: &kurbo::BezPath, dots: &kurbo::BezPath| {
        let mut outline = kurbo::stroke(shape.iter(), &style, &kurbo::StrokeOpts::default(), 0.1);
        outline.extend(dots.iter());
        outline
    };
    if compose == Compose::Knockout {
        knock_out(scene, at, peniko::Fill::NonZero, &outline(&shape, &dots));
    }
    if let Some(mode) = layer {
        scene.push_layer(
            peniko::Fill::NonZero,
            mode,
            1.0,
            at,
            &outline(&shape, &dots),
        );
        draw(scene);
        scene.pop_layer();
    } else {
        draw(scene);
    }
    Ok(())
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

/// Encodes one fill command, including the marks a shape with no area cannot make itself.
///
/// Split out of [`encode`] because ISO 32000-2 §10.7.4 turns one command into two draws: a
/// subpath with no extent along one axis covers no pixel any rasteriser can measure, and the
/// clause says no shape may disappear. What it marks instead is `pdf-render`'s own geometry
/// rather than whatever a hairline would be on this backend, so that the two backends cannot
/// answer it differently — and it is filled under the non-zero rule whatever the command's own
/// rule is, because a mark is a shape in its own right rather than part of the path's winding.
fn encode_fill_command(
    scene: &mut vello::Scene,
    (path, rule): (&Path, peniko::Fill),
    spaces: Spaces,
    path_to_device: Transform,
    paint: &Paint,
    how: (Compose, BlendMode),
    target: TargetSpec,
) -> Result<(), GpuRasterError> {
    let split = pdf_render::split_collapsed_fill(path, path_to_device);
    let Some(split) = split else {
        return encode_fill(scene, &bez_path(path), spaces, rule, paint, how, target);
    };
    if !split.marks.is_empty() {
        encode_fill(
            scene,
            &bez_path(&split.marks),
            spaces,
            peniko::Fill::NonZero,
            paint,
            how,
            target,
        )?;
    }
    if split.filled.is_empty() {
        return Ok(());
    }
    encode_fill(
        scene,
        &bez_path(&split.filled),
        spaces,
        rule,
        paint,
        how,
        target,
    )
}

/// Encodes one shape of a fill command.
///
/// Separate from the dispatch loop because a fill has three shapes: an ordinary brush, a
/// brush under a blend layer, and a mesh, which is not a brush at all.
fn encode_fill(
    scene: &mut vello::Scene,
    shape: &kurbo::BezPath,
    spaces: Spaces,
    rule: peniko::Fill,
    paint: &Paint,
    (compose, blend): (Compose, BlendMode),
    target: TargetSpec,
) -> Result<(), GpuRasterError> {
    let at = spaces.at;
    let layer = compose.layer(blend);
    if compose == Compose::Knockout {
        knock_out(scene, at, rule, shape);
    }

    // A mesh carries a colour — or §8.7.4.5.5's parametric value — per triangle corner,
    // which no brush can express, so it is rasterised and drawn inside a layer clipped to
    // the shape.
    if let Paint::Shading(shading) = paint
        && let pdf_render::ShadingKind::Mesh { triangles, ramp } = shading.kind.as_ref()
    {
        // A mesh always needs a layer, because its raster is clipped to the shape;
        // source-over is what an unblended one composites through.
        let mode = layer.unwrap_or(peniko::BlendMode::new(
            peniko::Mix::Normal,
            peniko::Compose::SrcOver,
        ));
        scene.push_layer(rule, mode, 1.0, at, shape);
        crate::shading::fill_mesh(
            scene,
            triangles,
            ramp.as_ref(),
            shading.transform.then(spaces.to_device),
            target,
        );
        scene.pop_layer();
        return Ok(());
    }

    // §8.7.4.5.4's cone: a point can lie on two blend circles and the clause's "greatest
    // value of s" decides between them, which no two-point conical gradient expresses. The
    // exact evaluation is drawn as an image inside a layer clipped to the shape, exactly as
    // a mesh is; every other radial keeps Vello's gradient. See `shading::is_a_cone`.
    if let Paint::Shading(shading) = paint
        && let pdf_render::ShadingKind::Radial {
            start,
            start_radius,
            end,
            end_radius,
            ramp,
            extend,
        } = shading.kind.as_ref()
        && crate::shading::is_a_cone(*start, *start_radius, *end, *end_radius)
    {
        let mode = layer.unwrap_or(peniko::BlendMode::new(
            peniko::Mix::Normal,
            peniko::Compose::SrcOver,
        ));
        scene.push_layer(rule, mode, 1.0, at, shape);
        let drawn = crate::shading::fill_radial(
            scene,
            pdf_render::Radial {
                start: *start,
                start_radius: *start_radius,
                end: *end,
                end_radius: *end_radius,
                ramp,
                extend: *extend,
            },
            shading.transform.then(spaces.to_device),
            device_bounds(shape, at, target),
        );
        scene.pop_layer();
        if drawn {
            return Ok(());
        }
    }

    let (brush, brush_at) = brush_for(paint, spaces.page_to_path)?;

    // A non-normal blend mode needs its own layer: Vello composites a layer against its
    // backdrop with the layer's blend mode, and there is no per-draw blend parameter. The
    // layer is clipped to the shape being drawn, so the blend applies exactly where paint
    // lands.
    if let Some(mode) = layer {
        scene.push_layer(rule, mode, 1.0, at, shape);
        scene.fill(rule, at, &brush, brush_at, shape);
        scene.pop_layer();
    } else {
        scene.fill(rule, at, &brush, brush_at, shape);
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
    target: TargetSpec,
    masks: &SoftMaskRasters,
) -> Result<vello::Scene, GpuRasterError> {
    build_commands(list, list.commands(), target, masks)
}

/// Builds a Vello scene from one sequence of commands of a display list.
///
/// Separate from [`build`] because a soft mask's group is exactly that — a command list of
/// the same display list, drawn onto transparency at the same target — and evaluating one
/// must go through the same translation as the page, or the mask would be built by code the
/// cross-backend tests never exercise.
///
/// # Errors
///
/// As [`build`].
pub(crate) fn build_commands(
    list: &DisplayList,
    commands: &[Command],
    target: TargetSpec,
    masks: &SoftMaskRasters,
) -> Result<vello::Scene, GpuRasterError> {
    let mut scene = vello::Scene::new();
    encode(
        &mut scene,
        list,
        commands,
        Spec {
            target,
            masks,
            depth: 0,
            compose: Compose::Over,
        },
    )?;
    Ok(scene)
}

/// What every level of [`encode`] needs and none of it changes except the depth.
///
/// Grouped because threading three more arguments through a recursive encoder made every
/// call site a row of unlabelled values, and because the target and the masks travel
/// together by necessity: a mask raster is only valid for the target it was rendered at.
#[derive(Debug, Clone, Copy)]
struct Spec<'a> {
    target: TargetSpec,
    masks: &'a SoftMaskRasters,
    /// How many enclosing transparency groups; see [`MAX_GROUP_DEPTH`].
    depth: usize,
    /// How the commands being encoded combine with each other (§11.4.6).
    compose: Compose,
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
    spec: Spec<'_>,
) -> Result<(), GpuRasterError> {
    let to_device = spec.target.transform;
    // The clip chain currently pushed as layers, root-first. A group's elements start with
    // an empty one rather than inheriting the caller's: reconciling against a chain opened
    // outside the group's own layer could pop that layer, and re-pushing a clip already in
    // force costs an intersection with itself and changes nothing.
    let mut open: Vec<ClipId> = Vec::new();

    for command in commands {
        let wanted = resolve_chain(list, command.clip())?;
        // §8.5.4 with §8.5.3.3.1: a clip whose path is empty admits nothing, so the command
        // — a group included — marks no pixel and is not encoded at all. Vello would clip
        // an empty path to an empty region and reach the same page, but by its own
        // convention rather than by the clause; `Clip::admits_nothing` is where the two
        // backends are held to one answer.
        if wanted
            .iter()
            .filter_map(|&id| list.clip(id))
            .any(Clip::admits_nothing)
        {
            continue;
        }
        reconcile_layers(scene, list, &mut open, &wanted, to_device)?;

        // §11.4.6's two stages, for an element whose shape the display list states apart
        // from its alpha. Handled before the mask layer below rather than beside the other
        // commands, because the object carries the mask and the recursion applies it: doing
        // it here as well would multiply the mask into the object twice, which is the very
        // thing §11.6.4.3's NOTE 2 warns a *file* against.
        if let Command::Shaped { object, shape } = command {
            encode_shaped(scene, list, (object, shape), spec)?;
            continue;
        }

        // §11.6.4.3's soft mask applies to one object at a time, so the object is isolated
        // in a layer of its own and the mask's alpha then multiplied into it. Opened before
        // the command and closed after, both inside the clip layers, because a clip and a
        // mask intersect and the order two coverages multiply in does not matter.
        let masked = command.mask();
        if masked.is_some() {
            open_layer(scene, spec.target, peniko::Compose::SrcOver);
        }

        match command {
            Command::Fill {
                path,
                transform,
                fill_rule: rule,
                paint,
                blend,
                ..
            } => {
                encode_fill_command(
                    scene,
                    (path, fill_rule(*rule)),
                    Spaces::new(*transform, to_device)?,
                    transform.then(to_device),
                    paint,
                    (spec.compose, *blend),
                    spec.target,
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
                encode_stroke(
                    scene,
                    path,
                    Spaces::new(*transform, to_device)?,
                    s,
                    transform.then(to_device),
                    paint,
                    (spec.compose, *blend),
                )?;
            }
            Command::Image {
                image,
                transform,
                alpha,
                blend,
                ..
            } => draw_image(
                scene,
                image,
                transform.then(to_device),
                *alpha,
                (spec.compose, *blend),
            )?,
            Command::Group {
                commands,
                alpha,
                blend,
                isolated,
                knockout,
                ..
            } => {
                if spec.compose == Compose::Knockout {
                    return Err(nested_group_in_knockout());
                }
                if !*isolated {
                    return Err(non_isolated_group());
                }
                encode_group(scene, list, commands, (*alpha, *blend), *knockout, spec)?;
            }
            other => {
                return Err(GpuRasterError::UnsupportedCommand(format!("{other:?}")));
            }
        }

        if let Some(id) = masked {
            apply_mask(scene, spec, id)?;
            scene.pop_layer();
        }
    }

    // Close every layer left open, or the scene is malformed.
    for _ in 0..open.len() {
        scene.pop_layer();
    }

    Ok(())
}

/// Encodes §11.4.6's two stages for an element that states its shape (`Command::Shaped`).
///
/// The clause's second stage is a weighted average of the object's composite with the
/// element's immediate backdrop, "using the source shape as the weighting factor", which on
/// the transparent initial backdrop a group is built on comes to `P' = (1 − f) × P + S` in
/// premultiplied form — the backdrop scaled by one minus the shape, plus the object.
///
/// Both operators are safe where Vello's layers are not: `Compose::Copy` writes the source
/// over the layer's whole bounding box and so erases outside the shape (see [`knock_out`]),
/// while `DestOut` leaves the destination exactly where the layer contributes nothing and
/// `Plus` adds nothing there. The sum of the two draws is bounded by 1 at every pixel — the
/// backdrop keeps `1 − f` and the object brings `f × opacity` — so `Plus`'s saturation never
/// engages and the pair is the clause's arithmetic rather than an approximation of it.
///
/// Outside a knockout group the shape is unused, so the object is encoded alone.
///
/// # Errors
///
/// As [`encode`].
fn encode_shaped(
    scene: &mut vello::Scene,
    list: &DisplayList,
    (object, shape): (&Command, &Command),
    spec: Spec<'_>,
) -> Result<(), GpuRasterError> {
    // Inside either layer an element draws ordinarily: the compositing operator is the
    // layer's, and the element's own is §11.4.5's over.
    let inside = Spec {
        compose: Compose::Over,
        ..spec
    };
    if spec.compose != Compose::Knockout {
        return encode(scene, list, std::slice::from_ref(object), inside);
    }
    open_layer(scene, spec.target, peniko::Compose::DestOut);
    let erased = encode(scene, list, std::slice::from_ref(shape), inside);
    scene.pop_layer();
    erased?;
    open_layer(scene, spec.target, peniko::Compose::Plus);
    let added = encode(scene, list, std::slice::from_ref(object), inside);
    scene.pop_layer();
    added
}

/// Opens a layer over the whole target, whose contents composite by `compose`.
///
/// Vello has no per-draw compositing operator, so every operator this backend needs beyond
/// source-over is a layer: §11.6.4.3's mask multiplied in, and §11.4.6's two stages. The
/// shape is the whole target rather than the mark's, because an operator that changes the
/// destination where the source contributes nothing would then be confined to a box, which
/// is the defect [`knock_out`] records.
fn open_layer(scene: &mut vello::Scene, target: TargetSpec, compose: peniko::Compose) {
    scene.push_layer(
        peniko::Fill::NonZero,
        peniko::BlendMode::new(peniko::Mix::Normal, compose),
        1.0,
        kurbo::Affine::IDENTITY,
        &device_rect(target),
    );
}

/// Encodes one transparency group as a Vello layer (§11.4.1).
///
/// A Vello layer *is* a transparency group: its contents are composited onto a transparent
/// surface and the result is painted once, under the layer's alpha and blend mode. That is
/// §11.4.1's definition, so this command translates rather than being implemented.
///
/// The layer's shape is the page, because the group itself clips nothing — the clip in force
/// is already open as a layer of its own, and nothing outside the page bounds can reach a
/// target built from them.
///
/// # Errors
///
/// As [`encode`], plus [`BackendError::GroupsTooDeep`] past [`MAX_GROUP_DEPTH`].
fn encode_group(
    scene: &mut vello::Scene,
    list: &DisplayList,
    commands: &[Command],
    (alpha, blend): (f32, BlendMode),
    knockout: bool,
    spec: Spec<'_>,
) -> Result<(), GpuRasterError> {
    let depth = spec.depth.saturating_add(1);
    if depth > MAX_GROUP_DEPTH {
        return Err(BackendError::GroupsTooDeep {
            depth,
            limit: MAX_GROUP_DEPTH,
        }
        .into());
    }
    scene.push_layer(
        peniko::Fill::NonZero,
        blend_mode(blend),
        alpha.clamp(0.0, 1.0),
        affine(spec.target.transform),
        &kurbo::Rect::new(
            f64::from(list.page_bounds().min.x),
            f64::from(list.page_bounds().min.y),
            f64::from(list.page_bounds().max.x),
            f64::from(list.page_bounds().max.y),
        ),
    );
    let compose = if knockout {
        Compose::Knockout
    } else {
        Compose::Over
    };
    encode(
        scene,
        list,
        commands,
        Spec {
            depth,
            compose,
            ..spec
        },
    )?;
    scene.pop_layer();
    Ok(())
}

/// The whole target, in device pixels.
///
/// A mask has a value everywhere — §11.6.5.1 gives one for the area outside its group's
/// bounding box — so the layer it is applied through covers everything the target can show.
fn device_rect(target: TargetSpec) -> kurbo::Rect {
    kurbo::Rect::new(0.0, 0.0, f64::from(target.width), f64::from(target.height))
}

/// Multiplies the layer in progress by a soft mask's values.
///
/// `Compose::DestIn` is "the parts of the destination that overlap with the source", which
/// for a source of no colour and this much alpha is the destination scaled by the mask —
/// §11.3.7.1's `α = f × q` with the mask as one of the two factors. The mask image is in
/// device pixels and drawn under the identity, since it was rendered at exactly this target.
///
/// # Errors
///
/// [`GpuRasterError::UnknownSoftMask`] if the mask was not evaluated for this target.
fn apply_mask(
    scene: &mut vello::Scene,
    spec: Spec<'_>,
    id: pdf_render::SoftMaskId,
) -> Result<(), GpuRasterError> {
    open_layer(scene, spec.target, peniko::Compose::DestIn);
    scene.draw_image(spec.masks.get(id)?, kurbo::Affine::IDENTITY);
    scene.pop_layer();
    Ok(())
}

/// Draws one image over the unit square its command names.
///
/// Split out of [`build`] because Vello wants an image's brush, its sampler and its blend
/// layer set up together, and because it is the one command whose *sampler* is a decision
/// rather than a translation — see [`pdf_render::Image::is_smoothed`].
fn draw_image(
    scene: &mut vello::Scene,
    source: &pdf_render::ImageSource,
    placement: Transform,
    alpha: f32,
    (compose, blend): (Compose, BlendMode),
) -> Result<(), GpuRasterError> {
    let layer = compose.layer(blend);
    // Samples the display list deferred — §11.6.5.2's mask on a grid of its own — are produced
    // here, at the grid `pdf_render` derives from the placement, so that the three backends ask
    // for the same one. An ordinary image borrows.
    let resolved = source.at(placement);
    let image: &pdf_render::Image = &resolved;
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

    let unit = kurbo::Rect::new(0.0, 0.0, width, height);
    // An image's shape is the rectangle its samples cover, which is the unit square the
    // command's transform places — the same shape a blend layer is clipped to below.
    if compose == Compose::Knockout {
        knock_out(scene, at, peniko::Fill::NonZero, &unit);
    }
    if let Some(mode) = layer {
        scene.push_layer(peniko::Fill::NonZero, mode, 1.0, at, &unit);
        scene.draw_image(&brush, at);
        scene.pop_layer();
    } else {
        scene.draw_image(&brush, at);
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

/// The device pixels a shape covers, clamped to the target.
///
/// What a [`pdf_render::RadialRaster`] is worth evaluating over: an extended radial covers
/// everything, and `radial_gradients.pdf` puts twenty-four of them on one page, so a
/// page-sized raster apiece is a cost the shape already rules out. Half a pixel of margin on
/// each side, because a pixel is sampled at its centre and a shape ending at x = 10.0 still
/// covers the sample at 9.5 — `MeshRaster::build`'s own margin.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "each cast is between a device pixel index and its coordinate, both bounded by \
              the target's extent"
)]
fn device_bounds(
    shape: &kurbo::BezPath,
    at: kurbo::Affine,
    target: TargetSpec,
) -> (u32, u32, u32, u32) {
    // The bounding box of the transformed box rather than of the transformed path: a
    // conservative outer bound is all a raster's extent needs, and it avoids rebuilding the
    // whole path in device space to measure it.
    let bounds = at.transform_rect_bbox(kurbo::Shape::bounding_box(shape));
    (
        (bounds.x0 - 0.5).floor().max(0.0) as u32,
        (bounds.y0 - 0.5).floor().max(0.0) as u32,
        (bounds.x1 + 0.5)
            .ceil()
            .max(0.0)
            .min(f64::from(target.width)) as u32,
        (bounds.y1 + 0.5)
            .ceil()
            .max(0.0)
            .min(f64::from(target.height)) as u32,
    )
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
