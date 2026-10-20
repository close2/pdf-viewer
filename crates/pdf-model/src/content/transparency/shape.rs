//! §11.6.4.2: the shape of an element, stated as a command whose alpha is that shape, and whether a
//! group's raster carries Table 139's shape as well as its alpha (ADR 2080).

use std::sync::Arc;

use pdf_render::{
    BlendMode, ClipId, Color, Command, FillRule, Paint, Path, PathCommand, Point, SampleAlpha,
    SoftMaskId,
};

use crate::image::ShapeMasks;

use super::commands::any_command;
use super::readings::AlphaSource;

/// Whether every element of a knockout group has a shape a rasteriser can draw bare.
///
/// §11.4.6 replaces the accumulated group result "by only a fraction of the result of
/// compositing the object with the initial backdrop", and that fraction is the element's
/// *shape*. A rasteriser draws with one number per pixel, the coverage, and the clause says
/// in as many words why that is not enough in general:
///
/// > The existence of the knockout feature is the main reason for maintaining a separate
/// > shape value rather than only a single alpha that combines shape and opacity.
///
/// So this is the condition under which the two coincide, and it is where the display list
/// may carry `knockout` rather than a report:
///
/// - **No soft mask.** §11.6.4.1 makes a mask a source of *opacity*, and this renderer
///   applies it as coverage — the one place the conflation is visible, and
///   `knockout_smask.pdf` is the page that shows it.
/// - **No per-sample alpha.** An image's transparency may be §8.9.6.2's stencil, which is
///   shape, or §11.6.5.2's `/SMask`, which is opacity, and one RGBA raster's *samples* cannot
///   say which — the raster's own [`pdf_render::Image::sample_alpha`] does, and
///   [`stated_shape`] reads it, but a bare draw reads the samples. A shading that does not
///   extend leaves its region
///   unpainted, which is a shape of zero. A *constant* alpha is unambiguously opacity, so it
///   is allowed.
/// - **No nested group.** A group's result reaches the backends as a raster, so its shape
///   would be its alpha by construction — the same conflation one level down.
///
/// All three are [`AlphaSource::Opacity`]'s. Under [`AlphaSource::Shape`] there is no bare
/// draw at all, for the reason [`element_shape_is_coverage`] gives.
///
/// What is left is [`stated_shape`], which states the shape separately for the elements
/// this refuses, and a report for the elements *that* refuses.
pub(super) fn knockout_shape_is_coverage(commands: &[Command], alpha: AlphaSource) -> bool {
    commands
        .iter()
        .all(|command| element_shape_is_coverage(command, alpha))
}

/// Whether the raster a group accumulates carries Table 139's shape `f` as well as its alpha
/// `α` — `Command::Group`'s `alpha_is_shape` (ISO 32000-2 §11.4.4, §11.3.7.1, §11.6.4.2).
///
/// §11.3.7.1 gives alpha as "the product of shape and opacity", so the two coincide exactly
/// where the group's opacity is 1.0 at every point, and §11.6.4.2 says where that is:
///
/// > All elementary objects shall have an intrinsic opacity q j of 1.0 everywhere. Any desired
/// > opacity less than 1.0 shall be applied by means of an opacity mask or constant, as
/// > described in ' 11.6.4.3 , ' Mask shape and opacity ' and 11.6.4.4 , ' Constant shape and
/// > opacity ' .
///
/// So the question is only whether anything inside the group states one of those two **as
/// opacity**, and §11.6.4.3's NOTE 1 gives the flag that decides it: `/AIS` is "true if the
/// soft mask contains shape values, false for opacity", and §11.6.4.4 says the same of `CA`
/// and `ca`. Under [`AlphaSource::Shape`] every mask and constant inside the group is *shape*,
/// so §11.3.7.2's three opacity inputs are all 1.0 and an elementary element's drawn alpha is
/// its shape whatever it carries — the argument [`shape_the_alpha_already_is`] already makes.
/// Under [`AlphaSource::Opacity`] each element has to be opaque on its own terms, which is the
/// same list [`shape_without_the_mask_and_the_constants`] removes.
///
/// A nested group answers with the flag it was given, so each group is asked under the `/AIS`
/// reading **its own** content ran under rather than its parent's; what the parent adds is
/// §11.6.4.3's and §11.6.4.4's inputs at the nested blit, which are opacity under
/// [`AlphaSource::Opacity`] and shape under the other.
///
/// # What the answer is for
///
/// §8.5.4 constrains a group's shape by the clip in force where the group is painted — "the
/// shape of a transparency group … shall be influenced … by the one in effect at the time the
/// group's results are painted onto its backdrop" — and §10.7.4 makes that an intersection of
/// sets. A backend can only take it against a shape it holds, so this is the condition under
/// which a group's raster is that shape and the composition is exact rather than a product.
/// ADR 0492.
///
/// # The two it declines, neither of them because the equality fails
///
/// - **A [`Command::Shaped`] element.** Its two halves are drawn by two separate composites,
///   and proving the alpha those leave is the shape is a separate argument from this one.
///   Declining costs the exact composition and never correctness.
/// - **A command whose kind is unknown**, because [`Command`] is non-exhaustive.
///
/// # A knockout group is not one of them, and used to be
///
/// §11.4.6's accumulation is `(1 − f) × P + f × E` rather than §11.4.4's union, and the
/// clause applies that recurrence to shape and to alpha alike — differing, as §11.3.7.3's
/// union does, only in the opacity inputs it carries. So where every element's opacity is
/// 1.0 the equality `α = f` holds through a knockout stage exactly as it holds through a
/// union, and §11.6.4.2 is the same base case for both.
///
/// The caller declined every knockout group outright until ADR 0554, on the reason stated in
/// the first bullet above — and that reason is enforced by the first bullet, element by
/// element, since a [`Command::Shaped`] element is refused wherever it appears. A blanket
/// guard on the group therefore refused the case it named twice and every other case once,
/// which is `doc/todo/02` §1's *capability that arrived and announced nothing*:
/// [`Command::Shaped`] arrived, and the guard in front of it stayed. Ten clipped knockout
/// groups on one corpus drawing were being composited by a product where the clause asks for
/// a set, and raster's own proof of the same condition had already stopped declining them.
///
/// Isolation is *not* asked here and is not this function's to ask: a non-isolated group's
/// buffer starts as a copy of its backdrop, so the alpha in it is not the group's shape at
/// all — which every backend that reads this flag already tests for itself, because it is a
/// fact about the buffer rather than about the commands.
pub(in crate::content) fn group_alpha_is_shape(
    commands: &[Command],
    alpha: Option<AlphaSource>,
) -> bool {
    let Some(alpha) = alpha else {
        // §11.6.4.3's flag decides which of the two arguments above applies, and the caller
        // has already asked whether it decides anything here at all
        // ([`AlphaSourcesSeen::settled_over`]): `None` means the content painted under both
        // readings *and* states one of the two inputs the flag reinterprets, so neither
        // argument describes it.
        return false;
    };
    commands
        .iter()
        .all(|command| element_alpha_is_shape(command, alpha))
}

/// One element's half of [`group_alpha_is_shape`].
pub(super) fn element_alpha_is_shape(command: &Command, alpha: AlphaSource) -> bool {
    match command {
        Command::Fill { paint, .. } | Command::Stroke { paint, .. } => {
            alpha == AlphaSource::Shape || (command.mask().is_none() && paint_is_opaque(paint))
        }
        Command::Image {
            image, alpha: at, ..
        } => {
            alpha == AlphaSource::Shape
                || (command.mask().is_none() && *at >= 1.0 && image.is_opaque())
        }
        // A nested group's own alpha and mask are §11.6.4.4's constant and §11.6.4.3's mask
        // applied to it as an object, so under `Opacity` they lower its alpha below its shape
        // and under `Shape` they lower both together. Isolation is the third condition and it
        // is about the *buffer* rather than about the clause: a non-isolated group is
        // accumulated on a copy of its backdrop (ADR 0237), so what its raster holds is not
        // the group's own alpha at all — the sentence `stated_shape` already writes.
        Command::Group {
            alpha: at,
            isolated,
            knockout,
            alpha_is_shape,
            ..
        } => {
            *alpha_is_shape
                && *isolated
                && !*knockout
                && (alpha == AlphaSource::Shape || (command.mask().is_none() && *at >= 1.0))
        }
        _ => false,
    }
}

/// Whether a paint marks at full opacity wherever it marks at all.
///
/// §11.6.4.4's constant is folded into a solid colour's alpha and into a shading's own colours
/// by the time a command carries them, which is why this asks the paint rather than the state.
fn paint_is_opaque(paint: &Paint) -> bool {
    match paint {
        Paint::Solid(colour) => colour.a >= 1.0,
        Paint::Shading(shading) => shading.is_opaque(),
        // `Paint` is non-exhaustive; a paint whose opacity is unknown is not proven to be 1.0.
        _ => false,
    }
}

/// Whether one element's shape is the coverage a rasteriser draws it with.
///
/// The three conditions are [`knockout_shape_is_coverage`]'s, asked of one command.
///
/// Never under [`AlphaSource::Shape`], and that is the flag inverting the question rather
/// than a gap. A bare knockout draw is Porter-Duff Source modulated by coverage, so it
/// computes `(1 − cov) × P + cov × α × C` — the paint's own alpha read as §11.6.4.4's
/// *opacity*, which is exactly what `/AIS true` says it is not. The shape there is the whole
/// drawn alpha, which a second command states exactly; see [`shape_the_alpha_already_is`].
pub(super) fn element_shape_is_coverage(command: &Command, alpha: AlphaSource) -> bool {
    if alpha == AlphaSource::Shape || command.mask().is_some() {
        return false;
    }
    match command {
        // A *constant* alpha is unambiguously opacity, so the coverage a rasteriser draws
        // this with is still the shape — which is why a translucent solid is here.
        Command::Fill { paint, .. } | Command::Stroke { paint, .. } => match paint {
            Paint::Solid(_) => true,
            Paint::Shading(shading) => shading.is_opaque(),
            // `Paint` is non-exhaustive; a paint whose shape is unknown is refused,
            // which leaves the report standing.
            _ => false,
        },
        Command::Image { image, .. } => image.is_opaque(),
        _ => false,
    }
}

/// §11.6.4.2's shape of one element, as a command whose alpha *is* that shape.
///
/// Which of the two constructions below applies is §11.6.4.3's and §11.6.4.4's `/AIS`.
pub(super) fn stated_shape(
    command: &Command,
    alpha: AlphaSource,
    shape_masks: &ShapeMasks,
) -> Option<Command> {
    match alpha {
        AlphaSource::Opacity => shape_without_the_mask_and_the_constants(command, shape_masks),
        AlphaSource::Shape => shape_the_alpha_already_is(command),
    }
}

/// §11.6.4.2's shape under `/AIS true`, where the element's own drawn alpha is it.
///
/// Three sentences settle this and none of them is about a rasteriser. §11.6.4.2 gives every
/// elementary object its intrinsic opacity:
///
/// > All elementary objects shall have an intrinsic opacity q j of 1.0 everywhere.
///
/// §11.6.4.3's NOTE 1 and §11.6.4.4 then hand the soft mask and both alpha constants to
/// *shape* while the flag is set — see [`AlphaSource`] for both quotations. So all three of
/// §11.3.7.2's opacity inputs are 1.0, their product is 1.0, and §11.3.7.1's alpha is the
/// source shape and nothing else. **The number a rasteriser already draws the element with is
/// the shape**, so the shape command is the element itself.
///
/// Only the blend mode is dropped, for the reason [`shape_without_the_mask_and_the_constants`]
/// drops it: §11.4.6 leaves a knockout element nothing to blend against.
///
/// A nested group answers with the union §11.3.7.2 makes its shape, accumulated on
/// transparency — see the arm's own comment for why isolation does not enter into it.
fn shape_the_alpha_already_is(command: &Command) -> Option<Command> {
    match command {
        Command::Fill {
            path,
            transform,
            fill_rule,
            paint,
            clip,
            mask,
            ..
        } => Some(Command::Fill {
            path: Arc::clone(path),
            transform: *transform,
            fill_rule: *fill_rule,
            paint: paint.clone(),
            clip: *clip,
            mask: *mask,
            blend: BlendMode::Normal,
        }),
        Command::Stroke {
            path,
            transform,
            stroke,
            paint,
            clip,
            mask,
            ..
        } => Some(Command::Stroke {
            path: Arc::clone(path),
            transform: *transform,
            stroke: stroke.clone(),
            paint: paint.clone(),
            clip: *clip,
            mask: *mask,
            blend: BlendMode::Normal,
        }),
        Command::Image {
            image,
            transform,
            alpha,
            clip,
            mask,
            ..
        } => Some(Command::Image {
            image: image.clone(),
            transform: *transform,
            alpha: *alpha,
            clip: *clip,
            mask: *mask,
            blend: BlendMode::Normal,
        }),
        // §11.3.7.2 gives a group's shape without reference to its backdrop:
        //
        // > The shape of a group object shall be the union (as defined in 11.3.7.3, "Result
        // > shape and opacity") of the shapes of the objects it contains.
        //
        // Under this reading each element's shape is its own drawn alpha, and compositing by
        // Over takes the union of the alphas whatever blend function the *colours* go through
        // — §11.4.6's `(1 − f) × F + f` being that same union — so a group accumulated on
        // transparency accumulates exactly that union. Its opacity is 1.0 because every one of
        // §11.3.7.2's opacity inputs inside it is 1.0, so its alpha is its shape.
        //
        // **Emitted isolated whatever the element was**, which is the clause rather than a
        // substitution: a non-isolated group is *drawn* on a buffer seeded from its backdrop
        // (ADR 0237), so what it accumulates carries the backdrop's alpha beside its own — but
        // the sentence above makes the group's shape the union of its elements' either way, and
        // accumulating that union on transparency is the one way to hold it alone. ADR 1205.
        //
        // An element that states its shape is drawn *as* that shape, and the group then unions
        // its elements by Over rather than by §11.4.6's stages, because the two are one union
        // and only Over reads a fractional shape as the weight (`shapes_where_stated`).
        Command::Group {
            commands,
            alpha,
            clip,
            mask,
            knockout,
            blending,
            ..
        } => Some(Command::Group {
            commands: shapes_where_stated(commands),
            alpha: *alpha,
            clip: *clip,
            mask: *mask,
            blend: BlendMode::Normal,
            isolated: true,
            knockout: *knockout && !states_a_shape(commands),
            // What this function returns *is* the shape, by its own contract, so the alpha
            // drawing it leaves is that shape whatever the original group's flag said.
            alpha_is_shape: true,
            blending: blending.clone(),
        }),
        // An element that arrives with its shape already stated — an inner knockout group's,
        // or one sealed under the reading it was painted under (`seal`) — is that shape.
        Command::Shaped { shape, .. } => Some((**shape).clone()),
        _ => None,
    }
}

/// A group's elements as [`shape_the_alpha_already_is`] draws them for the group's shape: each
/// as it is, except that an element stating its shape is that shape, at any depth.
///
/// The drawn alpha of every other element is its shape under the reading this function is
/// asked under; a [`Command::Shaped`] element's object may have been painted under the other
/// one (`seal`), and its drawn alpha is then shape times opacity — so its stated shape stands in
/// for it, which is also what the object's alpha is wherever the two readings agree. A group
/// among them whose elements were so replaced unions them by Over: a knockout group's bare
/// element weights what is under it by its *coverage*, so a fractional shape drawn bare would
/// replace the union rather than join it, and §11.4.6's `(1 − f) × F + f` is Over's union
/// term for term.
fn shapes_where_stated(commands: &[Command]) -> Vec<Command> {
    commands
        .iter()
        .map(|command| match command {
            Command::Shaped { shape, .. } => (**shape).clone(),
            Command::Group {
                commands: inner, ..
            } if states_a_shape(inner) => {
                let mut group = command.clone();
                if let Command::Group {
                    commands, knockout, ..
                } = &mut group
                {
                    *commands = shapes_where_stated(inner);
                    *knockout = false;
                }
                group
            }
            _ => command.clone(),
        })
        .collect()
}

/// Whether any element of `commands`, at any depth, states its own shape.
fn states_a_shape(commands: &[Command]) -> bool {
    any_command(commands, &|element| {
        matches!(element, Command::Shaped { .. })
    })
}

/// §11.6.4.2's shape under `/AIS false`, which is the element with the mask and the
/// constants removed.
///
/// The clause gives the shape from the object's geometry and nothing else — for a path
///
/// > the shape shall always be 1.0 inside and 0.0 outside the path
///
/// — while §11.6.4.3's soft mask and §11.6.4.4's constant are opacity. So a shape command is
/// the element with those two removed: an opaque paint in place of a translucent one, no soft
/// mask, and the blend mode dropped because §11.4.6 leaves a knockout element nothing to
/// blend against. The clip stays: a clip constrains a shape as much as it constrains a mark.
///
/// Two paints carry the constant *inside* them, and each has a way back:
///
/// - **A shading.** This tree folds §11.6.4.4's constant into the shading's own colours
///   (`Shading::with_alpha`), and nothing else in a shading's colours is below 1.0 but
///   §8.6.6.4's `/None` colourant, which this tree reads as opacity 0 the way it reads a
///   `/None` fill. Where the shading paints is not in its colours at all — an axial ramp
///   that does not extend, a mesh's triangles, a sampled grid's cover — so
///   `Shading::opaque` is the shape §11.6.4.2 states for a `sh`: "1.0 inside and 0.0 outside
///   the bounds of the shading's painti ng geometry".
/// - **An image.** Its alpha is §8.9.6.2's stencil or §8.9.6.3's explicit mask, which are
///   shape, or §11.6.5.2's `/SMask`, which is opacity, and the samples cannot say which — the
///   raster carries the kind as a field, decided where the masks were applied
///   (`pdf_render::Image::sample_alpha`, ADR 1022). For a *shape*
///   the image is its own shape, drawn at alpha 1.0; for an *opacity* the shape is the
///   clause's "1.0 inside the image rectangle and 0.0 outside it", which is the unit square
///   under the image's transform, filled — the same path and the same coverage every
///   backend draws the image itself through.
///
/// `None` where this renderer cannot state the shape, which is where the report stands: a paint of
/// a kind this crate does not know. A stencil under an `/SMask` of its own is not among them — its
/// producer keeps the stencil apart from the opacity on every route (ADRs 1218, 1279). **An image
/// drawn outside this run used to be a third**, because the kind was a record the interpreter wrote
/// as it drew; a raster carries its own now, wherever it came from (ADR 1022).
///
/// A [`Command::Shaped`] answers with the shape it already carries: an inner knockout group's
/// elements arrive stated. A soft mask the interpreter built out of a stencil is kept rather
/// than removed, for the reason [`ShapeMasks::shape_of`] gives: that mask *is* shape — or,
/// for a stencil under an `/SMask` of its own, the mask beside it that holds the stencil alone.
fn shape_without_the_mask_and_the_constants(
    command: &Command,
    shape_masks: &ShapeMasks,
) -> Option<Command> {
    let shape_mask = |mask: &Option<SoftMaskId>| mask.and_then(|id| shape_masks.shape_of(id));
    match command {
        Command::Fill {
            path,
            transform,
            fill_rule,
            paint,
            clip,
            mask,
            ..
        } => Some(Command::Fill {
            path: Arc::clone(path),
            transform: *transform,
            fill_rule: *fill_rule,
            paint: opaque_paint(paint)?,
            clip: *clip,
            mask: shape_mask(mask),
            blend: BlendMode::Normal,
        }),
        Command::Stroke {
            path,
            transform,
            stroke,
            paint,
            clip,
            mask,
            ..
        } => Some(Command::Stroke {
            path: Arc::clone(path),
            transform: *transform,
            stroke: stroke.clone(),
            paint: opaque_paint(paint)?,
            clip: *clip,
            mask: shape_mask(mask),
            blend: BlendMode::Normal,
        }),
        Command::Image {
            image,
            transform,
            clip,
            ..
        } => match image.sample_alpha() {
            SampleAlpha::Shape => Some(Command::Image {
                image: image.clone(),
                transform: *transform,
                alpha: 1.0,
                clip: *clip,
                mask: None,
                blend: BlendMode::Normal,
            }),
            SampleAlpha::Opacity => Some(Command::Fill {
                path: Arc::new(unit_square()),
                transform: *transform,
                fill_rule: FillRule::NonZero,
                paint: Paint::Solid(Color::WHITE),
                clip: *clip,
                mask: None,
                blend: BlendMode::Normal,
            }),
            // A stencil under a soft mask of its own: §11.6.4.2's shape is the stencil's
            // painted areas and §11.6.4.3's opacity is the mask, and the raster a device draws
            // holds their product. The pair is kept apart on the way here for exactly this
            // question, so the shape is the stencil's own samples (ADR 1218) — on every route,
            // an `/SMask` the device-scale route declines and an `/SMaskInData` opacity included
            // (ADR 1279), so `image::decode_parts` hands no command a raster that multiplied the
            // two.
            SampleAlpha::Both => image.shape().map(|shape| Command::Image {
                image: shape,
                transform: *transform,
                alpha: 1.0,
                clip: *clip,
                mask: None,
                blend: BlendMode::Normal,
            }),
        },
        // A group's shape is the union of its elements', which is what drawing their shapes
        // onto transparency accumulates. **Knockout or not makes no difference to a shape**
        // and that is arithmetic rather than a simplification: §11.4.6 accumulates
        // `(1 − f) × F + f`, §11.4.4 accumulates `Union(F, f) = F + f − F × f`, and the two
        // expressions are equal.
        //
        // The group's own mask comes off for the reason a fill's does, unless it is a
        // stencil's: a stencil painted through a tiling pattern reaches the page as the
        // cells' group under the stencil (`Interpreter::tile`), and §11.6.4.2 makes that mask
        // the group's shape.
        //
        // A group whose raster is its shape (`alpha_is_shape`) keeps its elements' own
        // alphas, each with its blend mode dropped, and loses its own constant and mask: under
        // the reading its content ran under nothing inside it is below its shape, and that
        // reading need not be this one — a group states its own shape and its readings stay
        // out of the enclosing record (`run_transparency_group`), so stripping constants that
        // reading made shape would take the shape with them (ADR 1306).
        Command::Group {
            commands,
            clip,
            mask,
            knockout,
            alpha_is_shape: true,
            ..
        } => Some(group_shape(
            commands
                .iter()
                .map(shape_the_alpha_already_is)
                .collect::<Option<_>>()?,
            (*clip, shape_mask(mask)),
            *knockout,
        )),
        Command::Group {
            commands,
            clip,
            mask,
            ..
        } => Some(group_shape(
            commands
                .iter()
                .map(|command| shape_without_the_mask_and_the_constants(command, shape_masks))
                .collect::<Option<_>>()?,
            (*clip, shape_mask(mask)),
            false,
        )),
        Command::Shaped { shape, .. } => Some((**shape).clone()),
        _ => None,
    }
}

/// A group stating a shape: what [`shape_without_the_mask_and_the_constants`] makes of a group,
/// given its elements' shapes, the clip it keeps and the mask that is shape rather than
/// opacity, and whether its elements knock one another out — which changes no shape, for the
/// arithmetic reason that function gives, and is kept where the elements were stated for it.
fn group_shape(
    commands: Vec<Command>,
    (clip, mask): (Option<ClipId>, Option<SoftMaskId>),
    knockout: bool,
) -> Command {
    Command::Group {
        commands,
        alpha: 1.0,
        clip,
        mask,
        blend: BlendMode::Normal,
        // A shape is accumulated on transparency by definition — §11.6.4.2 gives it from
        // geometry alone — so the backdrop this is drawn over states nothing.
        isolated: true,
        knockout,
        // Every element has had §11.6.4.3's mask and §11.6.4.4's constants removed, or reads
        // them as shape, so each draws at its own shape and their union is this group's —
        // Table 139's `f` and `α` are one number in this raster because the opacities are
        // all 1.0.
        alpha_is_shape: true,
        // And a shape has no colour, so it composites in no space: a pair carried by the
        // object states which components its *colours* resolve to, and the chromatic half's
        // geometry is the group's whole shape already.
        blending: None,
    }
}

/// A paint that marks where its argument marks, at full opacity, or `None` for a paint of a
/// kind this crate does not know. See [`shape_without_the_mask_and_the_constants`].
fn opaque_paint(paint: &Paint) -> Option<Paint> {
    match paint {
        Paint::Solid(_) => Some(Paint::Solid(Color::WHITE)),
        // Shared where it is opaque already — a pattern paints many paths — and made so
        // where §11.6.4.4's constant was folded into it.
        Paint::Shading(shading) => Some(Paint::Shading(if shading.is_opaque() {
            Arc::clone(shading)
        } else {
            Arc::new(shading.opaque())
        })),
        _ => None,
    }
}

/// The unit square an image occupies (ISO 32000-2 §8.9.4), as a path: an image's shape where
/// its alpha is opacity, and its region wherever a command has to name one.
fn unit_square() -> Path {
    let mut path = Path::new();
    path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
    path.push(PathCommand::LineTo(Point::new(1.0, 0.0)));
    path.push(PathCommand::LineTo(Point::new(1.0, 1.0)));
    path.push(PathCommand::LineTo(Point::new(0.0, 1.0)));
    path.push(PathCommand::Close);
    path
}
