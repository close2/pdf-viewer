//! Clause 11: transparency groups, knockout, soft masks and blending colour spaces.
//!
//! Both halves of this tree's §11 story are here — the constructions the display list can
//! carry (§11.4.5's isolation, §11.4.6's knockout, §11.6.5's soft masks) and the analysis
//! that decides, per group, whether drawing or reporting is the honest answer.

use std::sync::Arc;

use pdf_render::{
    BlendMode, Color, Command, Paint, Path, PathCommand, Point, Rect, SoftMaskId, Transform,
};
use pdf_syntax::{Dictionary, Document, Name, Object};

use crate::colour::{ColourSpace, Compositing, InkScale, Press, Presses};
use crate::page::Page;

use super::colour::output_intent_space;
use super::reader::NestedContent;
use super::report::Unsupported;
use super::{GraphicsState, Interpreter, MAX_SOFT_MASK_DEPTH};

/// What a form `XObject`'s `/Group` asks for (ISO 32000-2 §11.6.6 Table 145).
///
/// Only the three entries that change what is drawn. `/S` is not carried because a
/// dictionary whose subtype is not `/Transparency` never becomes one of these.
#[derive(Debug, Clone)]
pub(super) struct TransparencyGroup {
    /// `/I`: whether the elements are composited onto a transparent initial backdrop
    /// (§11.4.5) rather than onto the group's backdrop.
    isolated: bool,
    /// `/K`: whether each element is composited with the initial backdrop rather than with
    /// the elements below it (§11.4.6).
    knockout: bool,
    /// `/CS`: the group's blending colour space, unresolved, or `Null` where absent.
    colour_space: Object,
}

/// A glyph outline's bounding box in page space, for §9.3.8's overlap test.
///
/// Built from the control points rather than from the curves' extremes, so it contains the
/// outline rather than hugging it. Both approximations run the same way — the box is a
/// superset of the ink — which is what the caller needs.
pub(super) fn outline_bounds(outline: &Path, transform: Transform) -> Option<Rect> {
    let mut bounds: Option<Rect> = None;
    let mut add = |point: Point| {
        let mapped = transform.apply(point);
        bounds = Some(match bounds {
            Some(rect) => rect.union(Rect::from_corners(mapped, mapped)),
            None => Rect::from_corners(mapped, mapped),
        });
    };
    for command in outline.commands() {
        match *command {
            PathCommand::MoveTo(point) | PathCommand::LineTo(point) => add(point),
            PathCommand::CurveTo(first, second, end) => {
                add(first);
                add(second);
                add(end);
            }
            PathCommand::Close => {}
        }
    }
    bounds
}

/// Whether any command in a group, at any depth, satisfies `wanted`.
///
/// Recursive because a group's elements may themselves be groups: §11.4.3 calls both an
/// *element*, and a question about what a group contains is a question about its tree.
pub(super) fn any_command(commands: &[Command], wanted: &dyn Fn(&Command) -> bool) -> bool {
    commands.iter().any(|command| {
        wanted(command)
            || match command {
                Command::Group { commands, .. } => any_command(commands, wanted),
                _ => false,
            }
    })
}

/// Whether a command asks to be blended with what is under it, rather than painted over it.
pub(super) fn command_blends(command: &Command) -> bool {
    match command {
        Command::Fill { blend, .. }
        | Command::Stroke { blend, .. }
        | Command::Image { blend, .. }
        | Command::Group { blend, .. } => *blend != BlendMode::Normal,
        // `Command` is non-exhaustive. A command whose blending is unknown counts as
        // blending, because both callers decide whether to *report*: an unnecessary report
        // is recoverable, a missed one is a page drawn wrong in silence.
        _ => true,
    }
}

/// Whether a command's result can let what is under it show through.
///
/// The union of blending and of any transparency at all — a constant alpha below one, a
/// colour or shading carrying alpha, an image with a soft mask or a stencil, and a soft mask
/// in the graphics state. A group is transparent if its own alpha is, or if anything inside
/// it is: a group of half-opaque objects produces a half-opaque result.
///
/// Scanning an image's samples is linear in them, which is why the one caller asks this only
/// for a knockout group.
fn command_composites(command: &Command) -> bool {
    if command_blends(command) {
        return true;
    }
    // A soft mask is §11.6.4.1's third source of opacity, so an object painted through one
    // composites however opaque its colour is. `knockout_smask.pdf` is why this line is
    // here: its knockout group paints an opaque blue over an opaque red *under a mask*, and
    // without this the §11.4.6 report saw two opaque fills and stayed quiet about a page
    // three references draw the other way.
    if command.mask().is_some() {
        return true;
    }
    match command {
        Command::Fill { paint, .. } | Command::Stroke { paint, .. } => match paint {
            Paint::Solid(colour) => colour.a < 1.0,
            Paint::Shading(shading) => !shading.is_opaque(),
            // `Paint` is non-exhaustive, and a paint whose opacity is unknown is treated as
            // compositing: this decides whether to *report*, and an unnecessary report is
            // recoverable where a missed one is a page drawn wrong in silence.
            _ => true,
        },
        Command::Image { image, alpha, .. } => *alpha < 1.0 || !image.is_opaque(),
        Command::Group {
            commands, alpha, ..
        } => *alpha < 1.0 || any_command(commands, &command_composites),
        _ => true,
    }
}

/// Names a colour space if compositing in it is not compositing on the device's components.
///
/// This tree composites on the three components of the device raster, so the spaces that ask
/// for what already happens are the three-component RGB ones: `/DeviceRGB`, `CalRGB`, and an
/// ICC profile of three components, each of which this tree already resolves *to* device RGB
/// one colour at a time. Those are a colorimetric difference this renderer takes page-wide and
/// records as a choice.
///
/// What is named is a space whose components are not those: `/DeviceGray`, `/DeviceCMYK`,
/// `Separation` and `DeviceN` blend a different number of components, and `Lab` blends three
/// that are not a linear map of these. §11.3.4 is why that is a difference rather than a
/// notation:
///
/// > The result of the computation thus depends on the colour space in which the colours are
/// > represented.
///
/// Honouring one means compositing in its own components and converting once at the end, which
/// is a second raster format rather than a colour conversion — ADR 0251 measures how far apart
/// the two orders of operation are, and it is up to 48 of 255 for `/DeviceCMYK`.
///
/// `None` for an absent entry as well as for an RGB one, which is why a caller deciding
/// §11.6.6's inheritance tests the entry's presence itself rather than reading it off this.
fn space_departure(document: &Document, entry: &Object) -> Option<String> {
    let object = document.resolve(entry);
    if matches!(object, Object::Null) {
        return None;
    }
    // Named before it is parsed, because what a report has to say is what the file
    // asked for, and a space this crate cannot read has no other description.
    let described = match &object {
        Object::Name(name) => format!("/{}", String::from_utf8_lossy(name.as_bytes())),
        _ => "an array-formed space".to_owned(),
    };
    match ColourSpace::parse(document, &object, &Dictionary::new()) {
        Some(ColourSpace::Rgb | ColourSpace::CalRgb { .. }) => None,
        Some(ColourSpace::Icc { profile }) if profile.channels() == 3 => None,
        _ => Some(described),
    }
}

/// The blending colour space §11.4.7 gives a page, named where it is one this tree departs from.
///
/// The root of the inheritance §11.6.6 states, and §11.4.7 says where its space comes from:
///
/// > That initial colour space shall serve as the default blending colour space for each page,
/// > unless the page explicitly specifies an alternative default by means of its page
/// > dictionary containing a Group key that contains a CS key whose value represents a
/// > different colour space from the initial blending colour space.
///
/// The initial one "is inherited from the native colour space of the actual, assumed or
/// simulated output device", which for this processor is the device raster's three components.
/// So a page that states nothing composites in what this tree composites in, and a page that
/// states a `/Group /CS` is judged by [`space_departure`] — and that entry decides the whole
/// page, because §11.4.7 also says "[a]ll page-level compositing shall be done in the default
/// blending colour space of the page".
pub(super) fn page_blending_space(document: &Document, page: &Page) -> Option<String> {
    let attributes = document.get_key(&page.dict, "Group");
    let attributes = attributes.as_dict()?;
    // §8.10.3 Table 94's `/S`, the same required entry a form's group carries.
    if document.get_key(attributes, "S").as_name()?.as_bytes() != b"Transparency" {
        return None;
    }
    space_departure(document, &document.get_key(attributes, "CS"))
}

/// The blending colour space in force *inside* a group, given the one in force outside it.
///
/// §11.6.6 states the two cases, and the second is the one this tree used to ignore:
///
/// > For isolated groups, if a group colour space ( CS ) is specified in the group attributes
/// > dictionary, all painting operators shall convert source colours in a colour space (that
/// > are not equivalent to the group colour space) to the group colour space before compositing
/// > objects into the group.
///
/// > For non-isolated groups, or if no group colour space is specified, the group colour space
/// > shall be inherited from the parent group or page.
///
/// §11.7.2 says it a second time and gives the reason — "the use of an explicit colour space in
/// a non-isolated group would require converting colours from the backdrop's colour space to
/// that of the group in order to perform the compositing computations" — so a `/CS` on a
/// non-isolated group is not the space anything composites in, and reporting it as one is
/// reporting a departure that is not there.
fn group_blending(
    document: &Document,
    group: &TransparencyGroup,
    inherited: Option<&str>,
) -> Option<String> {
    let entry = document.resolve(&group.colour_space);
    if !group.isolated || matches!(entry, Object::Null) {
        return inherited.map(str::to_owned);
    }
    space_departure(document, &entry)
}

/// Where a command marks the page, as a box containing its ink.
///
/// A superset rather than a tight fit — control points rather than curve extremes, and a
/// stroke widened by its whole line width rather than by half of it — because the one caller
/// is an overlap test for a *report*, where saying two elements might overlap when they do
/// not is recoverable and the reverse is a missed gap.
fn command_bounds(command: &Command) -> Option<Rect> {
    match command {
        Command::Fill {
            path, transform, ..
        } => outline_bounds(path, *transform),
        Command::Stroke {
            path,
            transform,
            stroke,
            ..
        } => {
            let bounds = outline_bounds(path, *transform)?;
            // The width is in the path's space, so it reaches the page scaled by the
            // transform — by the *largest* factor the transform stretches a length, since the
            // margin has to hold in every direction. This used to be the determinant's square
            // root, described as "an over-estimate for a sheared one", which is the wrong way
            // round: a shear can leave the determinant at 1 while tripling a length, so the
            // margin was too small and an overlap could be missed. `Transform::max_stretch`
            // is the bound the comment claimed.
            let margin = stroke.width * transform.max_stretch();
            Some(Rect::from_corners(
                Point::new(bounds.min.x - margin, bounds.min.y - margin),
                Point::new(bounds.max.x + margin, bounds.max.y + margin),
            ))
        }
        Command::Image { transform, .. } => {
            // An image occupies the unit square, which the command's transform places.
            let mut square = Path::new();
            square.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
            square.push(PathCommand::LineTo(Point::new(1.0, 0.0)));
            square.push(PathCommand::LineTo(Point::new(1.0, 1.0)));
            square.push(PathCommand::LineTo(Point::new(0.0, 1.0)));
            outline_bounds(&square, *transform)
        }
        Command::Group { commands, .. } => commands
            .iter()
            .filter_map(command_bounds)
            .reduce(Rect::union),
        // Unbounded, which the overlap test reads as covering everything.
        _ => None,
    }
}

/// What §11.6.4.3's alpha source parameter makes of a soft mask and the two alpha constants.
///
/// Table 57's `/AIS` is one boolean and it decides which of §11.3.7.2's two products a value
/// enters. §11.6.4.3's NOTE 1 states it of the mask:
///
/// > This is a boolean flag, set with the AIS ("alpha is shape") entry in a graphics state
/// > parameter dictionary (8.4.5, "Graphics state parameter dictionaries"): true if the soft
/// > mask contains shape values, false for opacity.
///
/// and §11.6.4.4 states it again of the constants:
///
/// > As described previously for the soft mask, the AIS ('alpha is shape') entry in a
/// > graphics state parameter dictionary shall determine whether the alpha constants are
/// > interpreted as shape values ( true ) or opacity values ( false ).
///
/// Alpha is their product either way (§11.3.7.1), so the flag changes no pixel outside a
/// knockout group — and inside one it changes which quantity §11.4.6's weighted average is
/// taken with. The two readings therefore build a knockout element's shape differently, and
/// the difference is the whole of what this type is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AlphaSource {
    /// `/AIS false`, Table 57's default: the mask and both constants are opacity.
    Opacity,
    /// `/AIS true`: they are shape, so §11.3.7.2's source opacity is 1.0 everywhere.
    Shape,
}

/// Which readings of §11.6.4.3's alpha source parameter the content being run painted under.
///
/// `/AIS` is a graphics state parameter, so one content stream may paint some of its marks
/// under each reading. A knockout group's construction differs between them, so a group whose
/// content stated both is refused rather than drawn under either — which is what [`Mixed`]
/// says and what [`Self::settled`] answers `None` for.
///
/// [`Mixed`]: Self::Mixed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AlphaSourcesSeen {
    /// Only §11.6.4.3's `false`.
    Opacity,
    /// Only its `true`.
    Shape,
    /// Both, so no single reading describes what the content painted.
    Mixed,
}

impl AlphaSourcesSeen {
    /// What a graphics state whose `/AIS` is `alpha_is_shape` paints under.
    pub(super) fn of(alpha_is_shape: bool) -> Self {
        if alpha_is_shape {
            Self::Shape
        } else {
            Self::Opacity
        }
    }

    /// What content that painted under both `self` and `other` painted under.
    pub(super) fn with(self, other: Self) -> Self {
        if self == other { self } else { Self::Mixed }
    }

    /// The one reading that describes all of it, or `None` where there is no such reading.
    pub(super) fn settled(self) -> Option<AlphaSource> {
        match self {
            Self::Opacity => Some(AlphaSource::Opacity),
            Self::Shape => Some(AlphaSource::Shape),
            Self::Mixed => None,
        }
    }
}

/// What was decided about a group, for the reports that ask what it did not get.
///
/// Grouped because these four always travel together and a report reads three of them
/// against one entry of Table 145 apiece.
#[derive(Debug, Clone, Copy)]
struct GroupDrawn {
    /// Whether §11.4.6's rule reached the display list rather than a report.
    knockout: bool,
    /// What the elements were composited onto — see `Command::Group`'s `isolated`.
    isolated: bool,
    /// §11.4.6's NOTE 6 — see [`Interpreter::transparent_initial_backdrop`].
    backdrop_transparent: bool,
    /// Which readings of §11.6.4.3's `/AIS` the group's content painted under.
    alpha_sources: AlphaSourcesSeen,
}

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
///   shape, or §11.6.5.2's `/SMask`, which is opacity, and one RGBA raster cannot say which.
///   A shading that does not extend leaves its region unpainted, which is a shape of zero.
///   A *constant* alpha is unambiguously opacity, so it is allowed.
/// - **No nested group.** A group's result reaches the backends as a raster, so its shape
///   would be its alpha by construction — the same conflation one level down.
///
/// All three are [`AlphaSource::Opacity`]'s. Under [`AlphaSource::Shape`] there is no bare
/// draw at all, for the reason [`element_shape_is_coverage`] gives.
///
/// What is left is [`stated_shape`], which states the shape separately for the elements
/// this refuses, and a report for the elements *that* refuses.
fn knockout_shape_is_coverage(commands: &[Command], alpha: AlphaSource) -> bool {
    commands
        .iter()
        .all(|command| element_shape_is_coverage(command, alpha))
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
fn element_shape_is_coverage(command: &Command, alpha: AlphaSource) -> bool {
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
fn stated_shape(command: &Command, alpha: AlphaSource) -> Option<Command> {
    match alpha {
        AlphaSource::Opacity => shape_without_the_mask_and_the_constants(command),
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
/// `None` for a nested group, which is the one element whose alpha is not its shape even here:
/// §11.3.7.2 makes a group object's opacity "the result of the opacity computations for all of
/// the objects it contains" rather than 1.0, and a non-isolated one accumulates its backdrop's
/// alpha besides. That is the shape channel §11.3.7.2's row owes, and it stays owed.
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
        // A group's shape is the union of its elements' (§11.3.7.2), and under this reading
        // each element's shape is its own alpha — which is exactly what an isolated group
        // accumulates: compositing by Over takes the union of the alphas whatever blend
        // function the *colours* go through, and §11.4.6's `(1 − f) × F + f` is that same
        // union. Its opacity is 1.0 because every one of §11.3.7.2's opacity inputs inside it
        // is 1.0, so the group's alpha is its shape and the shape command is the group itself.
        //
        // **Isolated only**, and that is the one place this reading does not simplify: a
        // non-isolated group is drawn on a buffer seeded from its backdrop (ADR 0237), so what
        // it accumulates carries the backdrop's alpha beside its own and is not a shape at all.
        Command::Group {
            commands,
            alpha,
            clip,
            mask,
            isolated: true,
            knockout,
            blending,
            ..
        } => Some(Command::Group {
            commands: commands.clone(),
            alpha: *alpha,
            clip: *clip,
            mask: *mask,
            blend: BlendMode::Normal,
            isolated: true,
            knockout: *knockout,
            blending: blending.clone(),
        }),
        // An inner knockout group's elements arrive with their shape already stated, under
        // whichever reading that group's own content ran — which is this one, because the
        // reading propagates outward (see `Interpreter::alpha_sources`).
        Command::Shaped { shape, .. } => Some((**shape).clone()),
        _ => None,
    }
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
/// `None` where this renderer cannot separate the two, which is where the report stands:
///
/// - **A shading that is not opaque.** §11.6.4.2 constrains such an object's shape by "the
///   objects that define the pattern", and this tree folds §11.6.4.4's constant alpha into
///   the shading's own colours (`Shading::with_alpha`), so a translucent colour and an
///   unpainted region are the same number by the time a command holds them.
/// - **An image whose samples are not opaque.** Its alpha is §11.6.5.2's `/SMask`, which is
///   opacity, or §8.9.6.2's stencil and §8.9.6.3's explicit mask, which are shape — and one
///   RGBA raster cannot say which.
///
/// A [`Command::Shaped`] answers with the shape it already carries: an inner knockout group's
/// elements arrive stated.
fn shape_without_the_mask_and_the_constants(command: &Command) -> Option<Command> {
    match command {
        Command::Fill {
            path,
            transform,
            fill_rule,
            paint,
            clip,
            ..
        } => Some(Command::Fill {
            path: Arc::clone(path),
            transform: *transform,
            fill_rule: *fill_rule,
            paint: opaque_paint(paint)?,
            clip: *clip,
            mask: None,
            blend: BlendMode::Normal,
        }),
        Command::Stroke {
            path,
            transform,
            stroke,
            paint,
            clip,
            ..
        } => Some(Command::Stroke {
            path: Arc::clone(path),
            transform: *transform,
            stroke: stroke.clone(),
            paint: opaque_paint(paint)?,
            clip: *clip,
            mask: None,
            blend: BlendMode::Normal,
        }),
        Command::Image {
            image,
            transform,
            clip,
            ..
        } => image.is_opaque().then(|| Command::Image {
            image: image.clone(),
            transform: *transform,
            alpha: 1.0,
            clip: *clip,
            mask: None,
            blend: BlendMode::Normal,
        }),
        // A group's shape is the union of its elements', which is what drawing their shapes
        // onto transparency accumulates. **Knockout or not makes no difference to a shape**
        // and that is arithmetic rather than a simplification: §11.4.6 accumulates
        // `(1 − f) × F + f`, §11.4.4 accumulates `Union(F, f) = F + f − F × f`, and the two
        // expressions are equal.
        Command::Group { commands, clip, .. } => Some(Command::Group {
            commands: commands
                .iter()
                .map(shape_without_the_mask_and_the_constants)
                .collect::<Option<_>>()?,
            alpha: 1.0,
            clip: *clip,
            mask: None,
            blend: BlendMode::Normal,
            // A shape is accumulated on transparency by definition — §11.6.4.2 gives it
            // from geometry alone — so the backdrop this is drawn over states nothing.
            isolated: true,
            knockout: false,
            // And a shape has no colour, so it composites in no space: a pair carried by
            // the object states which components its *colours* resolve to, and the
            // chromatic half's geometry is the group's whole shape already.
            blending: None,
        }),
        Command::Shaped { shape, .. } => Some((**shape).clone()),
        _ => None,
    }
}

/// A paint that marks where its argument marks, at full opacity, or `None` where the two
/// cannot be told apart. See [`shape_without_the_mask_and_the_constants`].
fn opaque_paint(paint: &Paint) -> Option<Paint> {
    match paint {
        Paint::Solid(_) => Some(Paint::Solid(Color::WHITE)),
        Paint::Shading(shading) => shading.is_opaque().then(|| Paint::Shading(shading.clone())),
        _ => None,
    }
}

/// A knockout group's elements, each carrying the shape it knocks out with (§11.4.6).
///
/// `None` where one element's shape cannot be stated, which leaves the whole group an
/// ordinary one with the report [`Interpreter::note_group_structure`] gives it —
/// per group rather than per element, because the model the clause states is the group's.
fn knockout_elements(commands: &[Command], alpha: AlphaSource) -> Option<Vec<Command>> {
    commands
        .iter()
        .map(|command| {
            if element_shape_is_coverage(command, alpha)
                || matches!(command, Command::Shaped { .. })
            {
                return Some(command.clone());
            }
            Some(Command::Shaped {
                object: Box::new(command.clone()),
                shape: Box::new(stated_shape(command, alpha)?),
            })
        })
        .collect()
}

/// Every element of a knockout group as a [`Command::Shaped`], for a group whose initial
/// backdrop is **not** transparent (§11.4.6).
///
/// [`knockout_elements`] leaves an element whose shape *is* its coverage bare, because on a
/// transparent backdrop one draw carries both quantities. Against the group's own backdrop a
/// backend needs the shape of **every** element per pixel — the weighted average's factor —
/// so each is stated, even where a single draw could have carried it. `None` where any
/// element's shape cannot be stated, which leaves the group the report it has.
fn stated_elements(commands: &[Command], alpha: AlphaSource) -> Option<Vec<Command>> {
    commands
        .iter()
        .map(|command| {
            if matches!(command, Command::Shaped { .. }) {
                return Some(command.clone());
            }
            Some(Command::Shaped {
                object: Box::new(command.clone()),
                shape: Box::new(stated_shape(command, alpha)?),
            })
        })
        .collect()
}

/// Whether two interpretations of one content stream drew the same structure.
///
/// The guard [`pdf_render::DisplayList::geometry_digest`] provides for §11.4.7's page pair, asked of a
/// *group's* pair: the two lists differ only in what each colour resolved to, so their
/// variants, nesting, clips, paths and blend modes must agree — the halves are resolved per
/// pixel, and a command in one and not the other would be converted against a shape that
/// never drew it. Soft masks are compared by *presence* rather than identity, because the
/// second run registers its own copies of the same masks and the identifiers differ by
/// construction.
fn paired(first: &[Command], second: &[Command]) -> bool {
    first.len() == second.len()
        && first.iter().zip(second).all(|(left, right)| {
            if std::mem::discriminant(left) != std::mem::discriminant(right)
                || left.clip() != right.clip()
                || left.mask().is_some() != right.mask().is_some()
                || std::mem::discriminant(&left.blend()) != std::mem::discriminant(&right.blend())
                || left.path().map(|path| path.commands().len())
                    != right.path().map(|path| path.commands().len())
            {
                return false;
            }
            match (left, right) {
                (
                    Command::Group {
                        commands: ours,
                        blending: pair,
                        ..
                    },
                    Command::Group {
                        commands: theirs,
                        blending: other,
                        ..
                    },
                ) => pair.is_none() && other.is_none() && paired(ours, theirs),
                (
                    Command::Shaped {
                        object: left_object,
                        shape: left_shape,
                    },
                    Command::Shaped {
                        object: right_object,
                        shape: right_shape,
                    },
                ) => {
                    paired(
                        std::slice::from_ref(left_object),
                        std::slice::from_ref(right_object),
                    ) && paired(
                        std::slice::from_ref(left_shape),
                        std::slice::from_ref(right_shape),
                    )
                }
                _ => true,
            }
        })
}

/// The first element of a knockout group whose shape this renderer cannot state, named for
/// the report. See [`stated_shape`] for why each is refused.
fn unstatable_shape(commands: &[Command], alpha: AlphaSource) -> Option<&'static str> {
    commands.iter().find_map(|command| {
        if element_shape_is_coverage(command, alpha) || stated_shape(command, alpha).is_some() {
            return None;
        }
        Some(match (command, alpha) {
            (Command::Group { .. }, AlphaSource::Shape) => {
                "a non-isolated group, whose accumulated alpha carries its backdrop's (§11.4.4)"
            }
            (Command::Image { .. }, _) => "an image whose samples state either shape or opacity",
            (Command::Fill { .. } | Command::Stroke { .. }, _) => "a shading that is not opaque",
            _ => "an element this renderer cannot describe the shape of",
        })
    })
}

/// §11.4.6's elements for a group the specification itself makes a knockout group out of,
/// or `None` where the rule cannot be drawn for them at all.
///
/// Two conditions, and the clause states both. The first is
/// [`knockout_shape_is_coverage`]: a rasteriser has one number per pixel where the clause
/// wants shape and opacity separately. The second is isolation, which §11.4.6 makes an
/// independent attribute — "[a] non-isolated knockout group composites its topmost enclosing
/// element with the group's backdrop" — and this renderer composites a group's elements onto
/// transparency. The two coincide by §11.4.4's NOTE 3 wherever no element blends: the
/// backdrop is composited in and removed again exactly, so it cancels. Where one blends it
/// does not, and the caller reports instead.
///
/// Under `/AIS true` the first condition is never met and never has to be: every element's
/// drawn alpha *is* its shape there ([`shape_the_alpha_already_is`]), so each states the pair
/// §11.4.6's two stages ask for and the group is drawn rather than reported.
///
/// The two callers are the places the specification itself makes a knockout group out of
/// something that is not one: §9.3.8's text object and §11.6.2's one object in parts.
pub(super) fn knockout_group_elements(
    commands: &[Command],
    alpha: Option<AlphaSource>,
) -> Option<Vec<Command>> {
    let alpha = alpha?;
    if any_command(commands, &command_blends) {
        return None;
    }
    match alpha {
        AlphaSource::Opacity => {
            knockout_shape_is_coverage(commands, alpha).then(|| commands.to_vec())
        }
        AlphaSource::Shape => stated_elements(commands, alpha),
    }
}

/// Whether §11.4.6's knockout could change a pixel of this group.
///
/// True when an element that composites overlaps an element painted before it. Where the
/// upper element is opaque and blends Normal it overwrites the lower one under either model,
/// and where two elements do not overlap there is nothing to knock out.
///
/// An element whose ink cannot be bounded is taken to overlap everything, which is the same
/// direction [`TextObject::note_knockout`] errs in and for the same reason.
fn knockout_can_show(commands: &[Command]) -> bool {
    let mut painted: Vec<Option<Rect>> = Vec::with_capacity(commands.len());
    for command in commands {
        let bounds = command_bounds(command);
        if command_composites(command)
            && painted.iter().any(|below| match (below, bounds) {
                (Some(first), Some(second)) => {
                    first.min.x < second.max.x
                        && second.min.x < first.max.x
                        && first.min.y < second.max.y
                        && second.min.y < first.max.y
                }
                _ => true,
            })
        {
            return true;
        }
        painted.push(bounds);
    }
    false
}

/// What §11.4.7's page group asks a page to composite in.
#[derive(Debug, Clone)]
pub(super) enum PagePress {
    /// The device's three components: no page group, or one whose space is a three-component
    /// RGB space, which is what this raster already holds.
    Device,
    /// Four components, and this is whose.
    In(Arc<Press>),
    /// Four components this page cannot be drawn in, and this says why.
    Beyond(BeyondPress),
}

/// Why four components were not sampled into a press.
///
/// **Every reason here is the document's**, which it was not until ADR 0417: one of them used
/// to be [`crate::colour::MAX_PRESSES`] being spent by other files, so the same page carried a
/// different verdict depending on what the process had opened first, and ADR 0416 had to give
/// a caller a way to tell the two apart. The budget is the interpretation's now, so the
/// distinction has nothing left to separate and the sentence is a fact about the file again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BeyondPress {
    /// The sentence the report carries.
    pub(super) why: &'static str,
}

impl BeyondPress {
    /// A refusal the document decides: the same answer on every run and on every machine.
    const fn stated(why: &'static str) -> Self {
        Self { why }
    }
}

/// The press §11.4.7's page group composites in, and where its four components come from.
///
/// > That initial colour space shall serve as the default blending colour space for each page,
/// > unless the page explicitly specifies an alternative default by means of its page
/// > dictionary containing a Group key that contains a CS key whose value represents a
/// > different colour space from the initial blending colour space.
///
/// Three routes reach a press, and Annex P — informative, and the standard's own algorithm for
/// this question — puts them in this order. A device blending space "first appl[ies] the
/// default colour space mechanism" (§8.6.5.6's `/DefaultCMYK`, whose value "shall be used as
/// the colour space for the operation currently being performed"); a page group with no parent
/// otherwise inherits "from the output device, or from the output intent" (§14.11.5), which is
/// also §8.6.5.7 NOTE 3's "an output intent dictionary, if present, can suggest such a
/// calibration"; and a `/CS` that is itself a four-component `ICCBased` space names the press
/// outright, which is §11.7.2's paragraph about `DeviceCMYK` being redefined inside the group.
///
/// Where none of the three names one, the four components are the assumed inks of ADR 0263 and
/// this is [`crate::colour::assumed_press`], which no budget applies to because no file asked
/// for it. Where one names a space this tree cannot sample — four components that are not an ICC
/// profile, or a press past this *page's* [`crate::colour::MAX_PRESSES`] —
/// [`PagePress::Beyond`] carries the reason into the report.
pub(super) fn page_press(document: &Document, page: &Page, presses: &Presses) -> PagePress {
    let attributes = document.get_key(&page.dict, "Group");
    let Some(attributes) = attributes.as_dict() else {
        return PagePress::Device;
    };
    // §8.10.3 Table 94's `/S`, as [`page_blending_space`] reads it.
    if document
        .get_key(attributes, "S")
        .as_name()
        .is_none_or(|name| name.as_bytes() != b"Transparency")
    {
        return PagePress::Device;
    }
    let entry = document.get_key(attributes, "CS");
    press_for_entry(document, &entry, &page.resources, presses)
}

/// The press a group or page `/CS` entry names, resolved against `resources`.
///
/// The tail of [`page_press`], split out because a *group's* `/CS` asks the same question one
/// scope down (§11.7.2 redefines `DeviceCMYK` "within the transparency group" for an
/// `ICCBased` CMYK space, and Table 145 subjects a device space here to §8.6.5.6's
/// remapping) — with the
/// resource dictionary in force at the group rather than the page's.
fn press_for_entry(
    document: &Document,
    entry: &Object,
    resources: &Dictionary,
    presses: &Presses,
) -> PagePress {
    match ColourSpace::parse(document, entry, &Dictionary::new()) {
        Some(ColourSpace::Cmyk) => named_press(document, resources, presses),
        Some(ColourSpace::Icc { profile }) if profile.channels() == 4 => {
            press_or_beyond(&profile, presses)
        }
        // A four-component space that is not a profile — a `DeviceN` of four inks, say — names
        // components this tree has no conversion out of, so it keeps its report.
        Some(space) if space.components() == 4 => PagePress::Beyond(BeyondPress::stated(
            "its four components are named by a space this tree cannot sample (§11.7.2), \
             so there is no conversion out of them",
        )),
        _ => PagePress::Device,
    }
}

/// The press a `/DeviceCMYK` group or page's four components belong to.
///
/// `/DefaultCMYK` first, because §8.6.5.6 says "shall" about the operation being performed
/// while §8.6.5.7 NOTE 3 says an output intent "can suggest" a calibration — the nearer and
/// stronger statement wins, which is the ranking ADR 0009 recorded for a colour on its way to a
/// pixel and this is the same ranking one clause up.
fn named_press(document: &Document, resources: &Dictionary, presses: &Presses) -> PagePress {
    // `None` cannot happen for a literal device name — `ColourSpace::by_name` falls back on the
    // device space when a `/DefaultCMYK` will not parse — and it is grouped with the plain
    // answer because a space that did not parse names no press.
    match ColourSpace::parse(
        document,
        &Object::Name(Name::new(b"DeviceCMYK".to_vec())),
        resources,
    ) {
        Some(ColourSpace::Cmyk) | None => {}
        Some(ColourSpace::Icc { profile }) if profile.channels() == 4 => {
            return press_or_beyond(&profile, presses);
        }
        Some(space) if space.components() == 4 => {
            return PagePress::Beyond(BeyondPress::stated(
                "its /DefaultCMYK names a space this tree cannot sample (§8.6.5.6), so there \
                 is no conversion out of its four components",
            ));
        }
        Some(_) => {}
    }
    match output_intent_space(document) {
        Some(ColourSpace::Icc { profile }) if profile.channels() == 4 => {
            press_or_beyond(&profile, presses)
        }
        _ => PagePress::In(crate::colour::assumed_press()),
    }
}

/// A profile sampled into a press, or the reason this page is not drawn in it.
///
/// The refusal is [`crate::colour::MAX_PRESSES`], and since ADR 0417 it is this page's own: a
/// page naming a ninth distinct press is refused the ninth on every run and on every machine,
/// where before the ninth press of the *process* was refused and which page that fell on
/// depended on the order the scheduler ran the others in.
fn press_or_beyond(profile: &crate::icc::Profile, presses: &Presses) -> PagePress {
    presses.press_for_profile(profile).map_or(
        PagePress::Beyond(BeyondPress::stated(
            "it is the ninth distinct press this page names and eight is the budget \
             (§11.7.2), so its four components are not converted out",
        )),
        PagePress::In,
    )
}

/// Where the interpreter's readback stood before a content stream was run a second time.
///
/// A group that composites in a four-component space is one content stream interpreted
/// twice — §11.4.7's construction one scope down — and the second run resolves colours
/// differently while placing the same glyphs in the same places. Everything a *reader* gets
/// out of the page would therefore arrive twice; this records where each accumulator stood
/// so that [`Interpreter::rewind_readback`] can put the second run's copy back off. What is
/// deliberately **not** rewound: `operations` (the second run is real work the budget has to
/// count), `unsupported` (keyed, so a duplicate report collapses), and the display list's
/// clip and soft-mask tables (the second run's commands reference the entries it registered).
struct ReadbackMark {
    text: usize,
    text_layer: usize,
    described: usize,
    artifacts: usize,
    marked: usize,
    associated: usize,
    glyphs: usize,
    codes_without_a_glyph: usize,
    codes_reaching_a_blank_glyph: usize,
    codes_without_a_character: super::UnnamedCodes,
    glyph_coverage: std::collections::BTreeMap<String, super::Coverage>,
    inferred_separators: usize,
    text_operations: usize,
    reversed_chars: usize,
    text_cursor: Option<(f32, f32)>,
}

impl Interpreter<'_> {
    /// Records where every readback accumulator stands. See [`ReadbackMark`].
    fn readback_mark(&self) -> ReadbackMark {
        ReadbackMark {
            text: self.text.len(),
            text_layer: self.text_layer.len(),
            described: self.described.len(),
            artifacts: self.artifacts.len(),
            marked: self.marked.len(),
            associated: self.associated.len(),
            glyphs: self.glyphs,
            codes_without_a_glyph: self.codes_without_a_glyph,
            codes_reaching_a_blank_glyph: self.codes_reaching_a_blank_glyph,
            codes_without_a_character: self.codes_without_a_character,
            glyph_coverage: self.glyph_coverage.clone(),
            inferred_separators: self.inferred_separators,
            text_operations: self.text_operations,
            reversed_chars: self.reversed_chars,
            text_cursor: self.text_cursor,
        }
    }

    /// Puts the readback back where [`Interpreter::readback_mark`] recorded it.
    fn rewind_readback(&mut self, mark: ReadbackMark) {
        self.text.truncate(mark.text);
        self.text_layer.truncate(mark.text_layer);
        self.described.truncate(mark.described);
        self.artifacts.truncate(mark.artifacts);
        self.marked.truncate(mark.marked);
        self.associated.truncate(mark.associated);
        self.glyphs = mark.glyphs;
        self.codes_without_a_glyph = mark.codes_without_a_glyph;
        self.codes_reaching_a_blank_glyph = mark.codes_reaching_a_blank_glyph;
        self.codes_without_a_character = mark.codes_without_a_character;
        self.glyph_coverage = mark.glyph_coverage;
        self.inferred_separators = mark.inferred_separators;
        self.text_operations = mark.text_operations;
        self.reversed_chars = mark.reversed_chars;
        self.text_cursor = mark.text_cursor;
    }

    /// The press an isolated group's own `/CS` asks its elements to composite in, where
    /// this run can honour it (§11.6.6, §11.7.2).
    ///
    /// `None` is "composite in what the parent composites in", and it is the answer unless
    /// **all** of the following hold, each for a stated reason:
    ///
    /// - **The group is isolated.** §11.6.6: "[f]or non-isolated groups, or if no group
    ///   colour space is specified, the group colour space shall be inherited from the
    ///   parent group or page."
    /// - **The parent composites on the device's three components.** A group inside
    ///   §11.4.7's four-component page, or inside another such group, already composites in
    ///   a press — restating it is not a departure, and naming a *different* press there
    ///   would need a per-pixel conversion between two presses this tree does not have, so
    ///   that case keeps its report. A soft mask's group never reaches here with `Device`
    ///   compositing unless its own derivation chose it, in which case the same
    ///   construction answers.
    /// - **The `/CS` names four components this tree can sample** — `/DeviceCMYK` through
    ///   §8.6.5.6's and §14.11.5's ranking, or a four-component `ICCBased` profile
    ///   (§11.7.2). Table 145 subjects a device space here to remapping through the
    ///   `DefaultCMYK` entry of the *current* resource dictionary's `ColorSpace`
    ///   subdictionary, which is why the group's resources are consulted rather than the
    ///   page's.
    /// - **No figure is supplying this content's colour from outside it** (§8.6.8's
    ///   `uncoloured`): the marks inside carry a colour resolved for the *parent's*
    ///   compositing, and reinterpreting them in ink would convert a colour that was never
    ///   stated here.
    /// - **No `/ExtGState` has stated Table 57's black generation**, which §11.7.5.3 puts
    ///   inside the conversion into the space and this conversion does not read. Checked
    ///   again after the run, since a `gs` inside the group can state one.
    /// - **Not a knockout group.** §11.4.6's staged rewrites edit the element list after
    ///   the runs, and editing one half of a pair would leave the other describing a
    ///   different construction. Such a group keeps the report it has; no corpus document
    ///   states the combination.
    fn group_press(
        &mut self,
        group: &TransparencyGroup,
        resources: &Dictionary,
    ) -> Option<Arc<Press>> {
        if self.compositing != Compositing::Device
            || !group.isolated
            || group.knockout
            || self.uncoloured
            || self.black_generation_stated
        {
            return None;
        }
        match press_for_entry(self.document, &group.colour_space, resources, self.presses) {
            PagePress::In(press) => Some(press),
            // A group whose press this page has no budget left for is drawn in the parent's
            // space and keeps §11.6.6's report, which is the answer that was right before this
            // construction existed and is still right.
            PagePress::Device | PagePress::Beyond(_) => None,
        }
    }

    /// Reports a blend mode inside a mask group whose channel is more than one component.
    ///
    /// §11.3.5.2 applies a separable blend function "separately to each set of corresponding
    /// components", and it says which components:
    ///
    /// > where the lowercase variables 𝑐 𝑟 , 𝑐 𝑏 , and 𝑐 𝑠 denote corresponding components of
    /// > the colours 𝐶𝑟 , 𝐶𝑏 , and 𝐶𝑠 , expressed in additive form.
    ///
    /// A subtractive group's components in additive form are the complements of its ink, and
    /// what this tree paints such a group in is one *weighted average* of those complements:
    /// `1 − ink ÷ scale` is `(0.3(1 − c) + 0.59(1 − m) + 0.11(1 − y) + (1 − k)) ÷ 2` for
    /// `DeviceCMYK`, whose weights sum to 1. Source-over is affine and passes through an
    /// average unchanged, which is what makes the whole construction exact; no other blend
    /// function does, because `B` of an average is not the average of `B`.
    ///
    /// So the condition is a scale of more than one component, which is
    /// [`crate::colour::InkScale::Double`] and nothing else: a `DeviceGray` group's channel
    /// *is* its one component in additive form, so every blend mode is exact there.
    ///
    /// **This is a silence the three-hundred-and-eightieth session left behind**, and finding
    /// it is why a removed report is worth re-deriving rather than deleting. Until ADR 0217
    /// every `DeviceCMYK` mask group was reported for being composited in device RGB, which
    /// covered this case without naming it; that sentence now fires only for `Lab`, and this
    /// one says the part of it that is still true.
    fn note_blended_luminosity(&mut self, compositing: &Compositing, commands: &[Command]) {
        if *compositing != Compositing::Luminosity(InkScale::Double)
            || !any_command(commands, &|command| command_blends(command))
        {
            return;
        }
        self.note(Unsupported::TransparencyGroup {
            detail: "a soft mask's group blends in a space of four components, which this \
                     composites on one weighted average of them (§11.3.5.2, §11.5.3)"
                .to_owned(),
        });
    }

    /// Evaluates a soft mask's transparency group and registers it (§11.5, §11.6.5.1).
    ///
    /// Returns `None` when the group draws nothing at all, which §11.5.2's NOTE 2 makes a
    /// mask of zero — but only for the alpha derivation, where an empty group masks
    /// everything away; a luminosity mask over a white backdrop is a mask of *one*. Both
    /// answers are the mask's own, so the group is registered either way and only an
    /// unreadable one gives up here.
    pub(super) fn build_soft_mask(
        &mut self,
        request: &crate::soft_mask::SoftMaskRequest,
        state: &GraphicsState,
    ) -> Option<SoftMaskId> {
        if self.soft_mask_depth >= MAX_SOFT_MASK_DEPTH {
            self.note(Unsupported::LimitReached {
                limit: "MAX_SOFT_MASK_DEPTH",
            });
            return None;
        }
        // §11.6.5.1: "The group shall be defined by a transparency group XObject (see 11.6.6,
        // 'Transparency group XObjects') designated by the G entry in the soft-mask
        // dictionary" — a form `XObject`, so §7.8.2's rule for a damaged prefix reaches here
        // too. **What makes it the same rule and not the sampled function's** (ADR 0356) is
        // that this clause states the mask's value where the group painted nothing: the
        // transfer function of 0.0 for `Alpha`, `/BC`'s luminosity for `Luminosity`. A place
        // the damage took is a place the group did not paint, and the clause already answers
        // for one of those — where a sampled function's missing samples are values it has no
        // answer for and interpolates from. Places, not values, so the prefix is drawn.
        let Some(content) = self.content_stream(
            &request.group,
            "a soft mask's transparency group /G (§11.6.5.1)",
        ) else {
            self.note(Unsupported::SoftMask {
                detail: "/SMask names an undecodable /G".to_owned(),
            });
            return None;
        };

        // §11.6.5.1: "The mask's coordinate system shall be defined by concatenating the
        // transformation matrix specified by the Matrix entry in the transparency group's
        // form dictionary … with the current transformation matrix at the moment the soft
        // mask is established in the graphics state with the gs operator." The mask is
        // therefore fixed here, at the `gs`, and does not move with whatever transform is in
        // force when it is finally used.
        let mut inner = GraphicsState::initial(state.transform);
        if let Some(matrix) = self.matrix(&request.group.dict) {
            inner.transform = matrix.then(inner.transform);
        }
        // The clip chain starts fresh rather than inheriting the caller's. A mask is not
        // painted, so nothing about it is clipped by the path in force; its own `/BBox` is
        // the whole of its extent, and §11.6.5.1 gives the mask a value everywhere outside
        // that box — the transfer function of 0.0, or of the backdrop's luminosity — which
        // an inherited clip would have no way to express.
        if let Some(bbox) = self.rectangle(&request.group.dict, "BBox") {
            inner.clip = self.rect_clip(bbox, inner.transform, None);
            if inner.clip.is_none() {
                self.note(Unsupported::LimitReached { limit: "max_clips" });
                return None;
            }
        }

        let resources = self
            .document
            .get_key(&request.group.dict, "Resources")
            .as_dict()
            .cloned()
            .unwrap_or_default();

        for detail in &request.departures {
            self.note(Unsupported::TransparencyGroup {
                detail: detail.clone(),
            });
        }

        let mark = self.list.command_count();
        self.soft_mask_depth = self.soft_mask_depth.saturating_add(1);
        // §8.6.8's restriction does **not** reach in here, and until the
        // two-hundred-and-thirty-seventh session it did. The clause applies "[i]n any glyph
        // description that uses the d1 operator … and to all other content streams invoked
        // from within the same glyph description", and it says why in the sentence before:
        // "when defining graphical figures whose colours shall be specified separately each
        // time they are used". A soft mask is not such a figure. It carries no colour to the
        // page at all — §11.6.5.2 turns the group's result into a luminosity and uses it as
        // *alpha* — so NOTE 1's own reason for exempting a stencil applies verbatim: it "does
        // not specify colours; instead, it designates places where the current colour is
        // painted". Worse, the restriction is actively destructive here: a `/Luminosity`
        // mask's values *are* the group's colours, so ignoring `rg` inside it changes the
        // mask, and ignoring the group's images leaves a mask of zero that erases the very
        // marks the glyph exists to make.
        //
        // `issue19634.pdf` is the witness — a Skia blur test whose red text is a Type 3 font
        // whose glyph procedure is `d1`, a `gs` naming a `/Luminosity` mask, and one `re f`.
        // The mask group draws a blurred greyscale image; with the flag leaking in, the image
        // was skipped by §8.6.8's image rule, the mask came out zero and the text vanished.
        // Ink 2.87 against `mupdf`'s 7.63 and `hayro`'s 8.11 (ADR 0173).
        let saved_uncoloured = std::mem::replace(&mut self.uncoloured, false);
        // §11.6.5.1 makes the group's `/CS` "the colour space in which the compositing
        // computation is to be performed", and `crate::soft_mask` has already decided which of
        // this tree's two routes answers that space. A mask group nested inside another one
        // may name a different space, so this is saved and restored like `uncoloured` rather
        // than set once.
        let saved_compositing =
            std::mem::replace(&mut self.compositing, request.compositing.clone());
        // And §11.6.6's blending space stops being a departure in here, which is ADR 0220's
        // finding rather than a simplification: a mask group whose space is subtractive is
        // painted in the ink §10.4.2.3 weighs, that weighting is linear in the components, and
        // a linear functional of a convex combination is the convex combination of the
        // functional. So the compositing this tree performs inside such a group *is* the
        // compositing the clause asks for, and the one thing that is not — a blend function,
        // which is not affine — is `note_blended_luminosity`'s report and not this one.
        let saved_blending = self.blending.take();
        // **And so does the flag that records one**, which it did not until the
        // four-hundred-and-fortieth session. [`Interpreter::blending_changed`] answers exactly
        // one question — whether the *page* may be composited in the space §11.4.7 gives it —
        // and the line above makes every group inside a mask compare its space against `None`,
        // so a mask group holding an isolated `/DeviceCMYK` group set the flag on a page that
        // composites in `/DeviceCMYK` and departs from nothing. **77 of the 85 web documents
        // reported for §11.6.6 and all three of the corpus's were that**, measured by asking
        // each change its `soft_mask_depth` (ADR 0276). A mask's group is not painted onto the
        // page at all — §11.5.3 turns its result into one luminosity — so no space inside it
        // is a space the page composites in.
        let saved_change = std::mem::replace(&mut self.blending_changed, false);
        // And so does §11.4.6's NOTE 6, for the reason the `false` below it states: the mask's
        // group is not an element of the knockout group the `gs` appears in, so nothing inside
        // it inherits that group's initial backdrop.
        let saved_backdrop = std::mem::replace(&mut self.transparent_initial_backdrop, false);
        // And §11.6.4.3's `/AIS`, restored exactly rather than folded back: the record answers
        // "which readings did the elements of the group being built paint under", and a
        // mask's content is not an element of anything — its marks become one alpha per pixel.
        let saved_ais = std::mem::replace(
            &mut self.alpha_sources,
            AlphaSourcesSeen::of(state.alpha_is_shape),
        );
        let saved_ais_mark = std::mem::replace(&mut self.alpha_sources_mark, mark);
        // And the record of a space departure met on a subtractive run, for ADR 0276's
        // reason one construction over: a space declared inside a mask is answered by
        // §11.5.3's own derivation and says nothing about the group the `gs` sits in.
        let saved_departed = std::mem::replace(&mut self.nested_space_departed, false);
        self.run(&content, &resources, &inner, 0);
        self.nested_space_departed = saved_departed;
        let mask_alpha_sources = std::mem::replace(&mut self.alpha_sources, saved_ais);
        self.alpha_sources_mark = saved_ais_mark;
        self.transparent_initial_backdrop = saved_backdrop;
        self.blending_changed = saved_change;
        self.blending = saved_blending;
        self.compositing = saved_compositing;
        self.uncoloured = saved_uncoloured;
        self.soft_mask_depth = self.soft_mask_depth.saturating_sub(1);
        let commands = self.list.split_off_commands(mark);

        // §11.5.3, of the group a mask is derived from: "G may be any kind of group -
        // isolated or not, knockout or not - producing various effects on the C result in
        // each case." So Table 145's two flags mean here what they mean anywhere, and the
        // group is evaluated as the isolated non-knockout one either way — the same
        // departure, reported on the same conditions, rather than a second reading of the
        // same table.
        if let Some(group) = self.transparency_group(&request.group.dict) {
            // `false`: a mask's group is evaluated into a mask raster by
            // `pdf_render::SoftMask`, which carries no knockout flag, so a knockout here is
            // a departure whatever its elements are.
            // `true` for the second: a mask raster is built on transparency, so a
            // non-isolated mask group is drawn as §11.4.5's isolated one and reports on the
            // same condition a painted group used to.
            // `false` for the third: §11.4.6's NOTE 6 gives its backdrop to the *elements* of
            // a knockout group, and a soft mask is named by an `/ExtGState` rather than being
            // an element of anything — so a mask group's non-isolation is a departure however
            // it was reached.
            self.note_group_structure(
                &group,
                &commands,
                GroupDrawn {
                    knockout: false,
                    isolated: true,
                    backdrop_transparent: false,
                    alpha_sources: mask_alpha_sources,
                },
            );
        }
        self.note_blended_luminosity(&request.compositing, &commands);

        let evaluated = pdf_render::SoftMask {
            commands,
            kind: request.kind,
            transfer: request.transfer.clone(),
        };
        let Ok(id) = self.list.add_soft_mask(evaluated) else {
            self.note(Unsupported::LimitReached {
                limit: "max_soft_masks",
            });
            return None;
        };
        Some(id)
    }

    /// Runs a transparency group `XObject`'s content and composites it as one object.
    ///
    /// `inner` is the state its content runs under — the form's matrix and `/BBox` clip
    /// already applied — and `outer` the state at the `Do`, which is what the group as an
    /// object is painted with.
    pub(super) fn run_transparency_group(
        &mut self,
        group: &TransparencyGroup,
        content: &NestedContent,
        resources: &Dictionary,
        inner: &GraphicsState,
        outer: &GraphicsState,
        form_depth: usize,
    ) {
        let mut inner = inner.clone();
        // §11.6.6, of what `Do` adds for a transparency group XObject:
        //
        // > Before execution of the transparency group XObject's content stream, the current
        // > blend mode in the graphics state shall be initialised to Normal , the current
        // > stroking and nonstroking alpha constants to 1.0, and the current soft mask to
        // > None .
        //
        // Its NOTE 1 gives the reason: those parameters apply to the *group*, once, when it
        // is composited into its parent, and leaving them in force would apply them a second
        // time to every element inside. All four are reset here, the soft mask included —
        // the group carries it instead, on the command below.
        inner.blend = BlendMode::Normal;
        inner.fill_alpha = 1.0;
        inner.stroke_alpha = 1.0;
        inner.soft_mask = None;

        let enclosing_knockout = self.inside_knockout;
        // §11.4.6's NOTE 6, which decides what *this* group's elements composite onto: this
        // group's own initial backdrop is transparent when it says so or when NOTE 6 hands it
        // the transparent one an enclosing knockout group has, and its elements inherit that
        // in turn only if this group is itself a knockout group.
        let backdrop_transparent = self.transparent_initial_backdrop;
        let enclosing_transparent = std::mem::replace(
            &mut self.transparent_initial_backdrop,
            group.knockout && (group.isolated || backdrop_transparent),
        );
        self.inside_knockout = enclosing_knockout || group.knockout;
        // §11.6.6's group colour space, which the elements composite in and which
        // [`group_blending`] resolves against §11.7.2's inheritance rule. Saved and restored
        // rather than set once, because a group is a scope: what is in force after the `Do` is
        // what was in force before it.
        let entered = group_blending(self.document, group, self.blending.as_deref());
        // A group *introduces* a departure only where the space it composites in is not the
        // one its parent was already composited in. Where it inherits, or where it restates
        // the space it inherited, the parent's report is the report — one departure named at
        // the point the file introduces it, rather than once per group that lives inside it.
        let changed = entered != self.blending;
        let introduced = changed.then(|| entered.clone()).flatten();
        self.blending_changed |= changed;
        let outside = std::mem::replace(&mut self.blending, entered);
        let (commands, pair, ais_inside) =
            self.group_commands(group, content, resources, &inner, form_depth);
        self.blending = outside;
        self.inside_knockout = enclosing_knockout;
        self.transparent_initial_backdrop = enclosing_transparent;
        // A group that changes the space in force, with something compositing in it, is a
        // departure the reports can only name on the device's components — so on any other
        // compositing it is *recorded* instead, and the enclosing pair run reads the record
        // and falls back to the device, where this same group will report ordinarily. Both
        // directions of change count, including one whose `entered` is `None`: a group
        // returning to the device's three components inside a four-component pair
        // composites in the wrong space just as surely as one leaving them.
        if changed
            && self.compositing != Compositing::Device
            && any_command(&commands, &command_composites)
        {
            self.nested_space_departed = true;
        }
        if commands.is_empty() {
            return;
        }

        // §11.4.4's NOTE 5 states, in full, when a group need not be built at all:
        //
        // > As a result of these corrections, the effect of compositing objects as a group is
        // > the same as that of compositing them separately (without grouping) if the following
        // > conditions hold:
        // >
        // > The group is non-isolated and has the same knockout attribute as its parent group
        // > (see 11.4.5, "Isolated groups" and 11.4.6 , 'Knockout groups').
        // >
        // > When compositing the group's results with the group backdrop, the Normal blend mode
        // > is used, and the shape and opacity inputs are always 1.0.
        //
        // Both conditions are decidable here, and together they are the *whole* of what a
        // non-isolated group's correctness needed. §11.4.4's result step removes the backdrop
        // from the group's accumulated colour — `C = Cn + (Cn − C0) × (α0/αgn − α0)` — which
        // this tree cannot compute on one raster, because NOTE 4 says the group alpha `αgn` has
        // to be accumulated *separately* from the composite alpha and an opaque backdrop
        // destroys the difference. Flattening sidesteps the arithmetic entirely by never
        // introducing the backdrop that would have to be removed: the elements composite onto
        // the page they were always going to composite onto, and every blend mode inside the
        // group then sees the backdrop §11.4.4 says it should see.
        //
        // The clip is not a condition. It reaches every element already — PDF's clipping is
        // cumulative in the graphics state, so an element inside the form carries the clip in
        // force at the `Do` — and applying it once per element is applying it once.
        //
        // This is also strictly less work than the group it replaces: no page-sized buffer, no
        // second composite. A correctness fix that is faster means the old code was doing work
        // that was worse than useless, which is this project's own name for the shape.
        if !group.isolated
            && !group.knockout
            && !enclosing_knockout
            && outer.fill_alpha >= 1.0
            && outer.blend == BlendMode::Normal
            && outer.soft_mask.is_none()
        {
            // No `/CS` question here, and that is §11.6.6 rather than an omission: this branch
            // is reached only where the group is non-isolated, and a non-isolated group's own
            // `/CS` is not the space anything composites in — "the group colour space shall be
            // inherited from the parent group or page". Whatever it inherited is already
            // reported where it was introduced, and this branch does not change it.
            for command in commands {
                self.list.push(command);
            }
            return;
        }

        // §11.4.6's rule reaches the backends where every element's shape is the coverage a
        // rasteriser draws it with, and stays a report where it does not.
        //
        // Isolation is a second condition and a different one. "[A] knockout group may be
        // isolated or non-isolated; that is, isolated and knockout are independent
        // attributes", and what this backend can composite onto is a transparent backdrop —
        // §11.4.5's. For a *non-isolated* knockout group the initial backdrop is the
        // group's own, and the two coincide by exactly the argument §11.4.4's NOTE 3 makes
        // and this tree already relies on: with every element blending Normal the backdrop
        // is composited in and removed again exactly, so it cancels. Where an element
        // blends it does not, and that group is the one `note_group_structure` already
        // names — so it keeps both reports rather than gaining a second departure.
        // `backdrop_transparent` is the third answer and it is the clause's rather than an
        // approximation: NOTE 6 hands a direct element of a knockout group that group's
        // initial backdrop, so a `/I false` knockout group nested in an isolated one has
        // §11.4.5's transparent backdrop and the pair below is exact for it.
        //
        // Where an element's shape is *not* its coverage the display list states the two
        // separately, which is `Command::Shaped` and ADR 0234. The coverage case is left
        // alone rather than folded into it: it is the same arithmetic in one draw instead
        // of two, and it is what §9.3.8's text objects are made of.

        // Whether §11.4.6's rule can change a pixel of this group, which decides below whether
        // its initial backdrop and §11.4.4's immediate one are the same thing. Asked of the
        // file's own elements, before the rewrite that turns any of them into a
        // `Command::Shaped` whose bounds and blending this predicate cannot read.
        let knockout_shows = group.knockout && knockout_can_show(&commands);
        let mut commands = commands;
        let mut knockout = false;
        // §11.4.6 on the group's own backdrop: "[a] nonisolated knockout group composites
        // its topmost enclosing element with the group's backdrop." Where an element blends,
        // that backdrop cannot be substituted by §11.4.5's transparent one — and since the
        // four-hundred-and-ninety-second session it is *stated* instead: the group goes to
        // the backends with `isolated: false` beside `knockout: true`, every element a
        // `Command::Shaped`, and a backend retains the initial backdrop beside the
        // accumulation (ADR 0327). Three conditions bound it, each the clause's:
        // `outer.blend == Normal` because the final composite's cancellation against
        // §11.4.4's backdrop removal is the Normal blend function's (ADR 0237's argument,
        // unchanged by knockout); `!enclosing_knockout` because an element of a knockout
        // group is weighted by its own shape, which `Command::Group` does not carry; and
        // every element's shape statable, because the weighted average's factor has to
        // come from somewhere.
        //
        // And a fourth, which is §11.6.4.3's: the shape §11.4.6 weights by is built one way
        // under each reading of `/AIS`, so a group whose content painted under both is
        // refused. `ais_inside.settled()` is that question, and both of its answers draw.
        let mut backdrop_composited = false;
        if group.knockout
            && let Some(alpha) = ais_inside.settled()
        {
            if group.isolated || backdrop_transparent || !any_command(&commands, &command_blends) {
                if knockout_shape_is_coverage(&commands, alpha) {
                    knockout = true;
                } else if let Some(elements) = knockout_elements(&commands, alpha) {
                    commands = elements;
                    knockout = true;
                }
            } else if knockout_shows
                && !enclosing_knockout
                && outer.blend == BlendMode::Normal
                && let Some(elements) = stated_elements(&commands, alpha)
            {
                commands = elements;
                knockout = true;
                backdrop_composited = true;
            }
        }
        // §11.4.4's own model, for the group NOTE 5 could not flatten: the elements
        // composite onto the backdrop the group is painted over, and the display list says
        // so rather than substituting §11.4.5's transparent one. The three conditions are
        // what makes the clause's backdrop removal cancel against §11.3.3's re-compositing
        // — see `Command::Group`'s `isolated` and ADR 0237 — and each is load-bearing:
        //
        // - **Normal at the `Do`.** The cancellation is of a division by Table 140's group
        //   alpha against a multiplication by it, and only the Normal blend function
        //   performs the second. Under any other the group's own colour is needed.
        // - **Not a knockout group whose rule can change a pixel.** §11.4.6 composites each
        //   element with the group's *initial* backdrop, which here is the page rather than
        //   transparency, so the two stages are not the pair `Command::Shaped` states. But
        //   the two backdrops are the same wherever the knockout rule can show nothing —
        //   `knockout_can_show` is that condition, and where it holds the immediate backdrop
        //   *is* the initial one at every point an element marks, which makes such a group
        //   §11.4.4's group exactly rather than a substitution for it. `/K true` alone used
        //   to stand here, and it cost `knockout_blend_multiply.pdf`'s single Multiply
        //   element the page it blends with (ADR 0307).
        // - **Not drawn as a knockout group.** The pair `Command::Shaped` states is
        //   `P' = (1 − f) × P + S` on the transparent start §11.4.5 gives, and seeding `P`
        //   from the page would put the backdrop in twice. Every group that reaches the
        //   condition above already satisfies this one — a knockout group is drawn only when
        //   it is isolated or when nothing in it blends — and it is stated rather than
        //   derived because a `Command::Shaped` reads as blending to `command_blends`.
        // - **Not inside a knockout group.** A knockout group's element is weighted by its
        //   own shape, which is a quantity this command does not carry.
        //
        // And a fourth condition, which is not about correctness but about *cost*: with
        // every element painting Normal the backdrop is composited in and removed again
        // exactly, so the two models are the same page and the isolated one is what every
        // rasteriser already has. Stating the harder construction there would buy nothing
        // and would cost a surface-sized copy per group — and two of the three backends
        // cannot draw it at all, so it would take pages off the cross-backend comparison
        // for a difference that provably does not exist. This is the same condition the
        // report fired on before the construction existed, and for the same reason.
        //
        // Isolation is otherwise §11.4.5's, which is what a rasteriser's layer is —
        // except where the two flags together state §11.4.6's own backdrop, which is the
        // one shape of the command in which `false` accompanies `knockout`.
        let isolated = !backdrop_composited
            && (group.isolated
                || knockout_shows
                || knockout
                || enclosing_knockout
                || outer.blend != BlendMode::Normal
                || !any_command(&commands, &command_blends));
        self.note_group_departures(
            group,
            &commands,
            GroupDrawn {
                knockout,
                isolated,
                backdrop_transparent,
                alpha_sources: ais_inside,
            },
            // A group drawn in its own space has no colour-space departure to report.
            if pair.is_some() {
                None
            } else {
                introduced.as_deref()
            },
        );
        // §11.6.6's final compositing: the group's shape "shall then be painted into the
        // parent group or page, using the group's accumulated colour and opacity at each
        // point" — under the state in force at `Do`, which is where `ca` and `/BM` were left
        // by the caller. `ca` and not `CA`, because painting a form is not a stroking
        // operation and §11.6.4.4 gives `CA` to those alone.
        self.list.push(Command::Group {
            commands,
            alpha: outer.fill_alpha,
            clip: inner.clip,
            isolated,
            // The mask in force at the `Do`, applied to the group as one object — which is
            // §11.6.4.3's NOTE 2 recommending exactly this construction: "[t]o apply a soft
            // mask to multiple objects, it is usually best to define the objects as a
            // transparency group and apply the mask to the group as a whole."
            mask: outer.soft_mask,
            blend: outer.blend,
            knockout,
            blending: pair.map(Box::new),
        });
    }

    /// Runs a group's content and collects its commands — twice where the group states a
    /// blending colour space of four components, once more where the two runs diverge.
    ///
    /// The first run is the one whose readback is kept, because a colour changes no glyph's
    /// place; every run after it is rewound (see [`ReadbackMark`]). Where
    /// [`Interpreter::group_press`] names a press, the first run resolves colours into its
    /// chromatic half and the second into its black half — §11.4.7's construction one scope
    /// down, see `pdf_render::GroupBlending` — and the two are paired only if their
    /// structures agree. Two cases fall back to one run on the device's components, each
    /// with the report it always had: a group in which **nothing composites**, where
    /// §11.3.4's per-component question cannot change a pixel (an opaque Normal mark
    /// carries its colour through whatever space it is carried through, which is the same
    /// condition the report fires on), and a run that stated §11.7.5.3's black generation,
    /// which the conversion into the space does not read.
    ///
    /// The third value is which of §11.6.4.3's `/AIS` readings the content ran under —
    /// scoped here, seeded from the state at the `Do`, because the entry is a graphics
    /// state parameter and the page-wide flag it used to be refused knockout groups whole
    /// forms away from any statement of it.
    fn group_commands(
        &mut self,
        group: &TransparencyGroup,
        content: &NestedContent,
        resources: &Dictionary,
        inner: &GraphicsState,
        form_depth: usize,
    ) -> (
        Vec<Command>,
        Option<pdf_render::GroupBlending>,
        AlphaSourcesSeen,
    ) {
        let mark = self.list.command_count();
        let outer_ais = std::mem::replace(
            &mut self.alpha_sources,
            AlphaSourcesSeen::of(inner.alpha_is_shape),
        );
        let outer_ais_mark = std::mem::replace(&mut self.alpha_sources_mark, mark);
        let ink = self.group_press(group, resources);
        let saved = self.compositing.clone();
        // Scoped only where a pair is being attempted: everywhere else the record has to
        // propagate *up* to whatever pair run this group may be inside.
        let saved_departed = if ink.is_some() {
            std::mem::replace(&mut self.nested_space_departed, false)
        } else {
            self.nested_space_departed
        };
        if let Some(press) = ink.clone() {
            self.compositing = Compositing::Subtractive(crate::colour::Half::Chromatic, press);
        }
        self.run(content, resources, inner, form_depth.saturating_add(1));
        self.compositing = saved.clone();
        let mut commands = self.list.split_off_commands(mark);
        let mut pair = None;
        if let Some(press) = ink.clone()
            && !commands.is_empty()
        {
            if !self.nested_space_departed
                && any_command(&commands, &command_composites)
                && !self.black_generation_stated
            {
                let rewind = self.readback_mark();
                self.compositing =
                    Compositing::Subtractive(crate::colour::Half::Black, Arc::clone(&press));
                self.run(content, resources, inner, form_depth.saturating_add(1));
                self.compositing = saved.clone();
                self.rewind_readback(rewind);
                let black = self.list.split_off_commands(mark);
                if paired(&commands, &black) {
                    pair = Some(pdf_render::GroupBlending {
                        space: press.blending_space(),
                        black,
                    });
                } else {
                    // The halves diverged structurally, which no valid content stream
                    // does; the device's components and the standing report are the
                    // answer that was right before this construction and is still right.
                    commands = self.rerun_on_device(content, resources, inner, form_depth, mark);
                }
            } else {
                // Nothing composites, §11.7.5.3's black generation is in force, or a group
                // inside introduced a space of its own — the last recorded rather than
                // reported, because a report about a space can only be made where the
                // device's components are what is being composited on, which the rerun is.
                commands = self.rerun_on_device(content, resources, inner, form_depth, mark);
            }
        }
        if ink.is_some() {
            self.nested_space_departed = saved_departed;
        }
        let ais_inside = self.alpha_sources;
        // Folded into the enclosing scope's record, unless that scope had painted nothing
        // before this group — in which case the enclosing reading reached no mark either and
        // this group's is the whole of what the enclosing content has painted under so far.
        self.alpha_sources = if mark == outer_ais_mark {
            ais_inside
        } else {
            outer_ais.with(ais_inside)
        };
        self.alpha_sources_mark = outer_ais_mark;
        (commands, pair, ais_inside)
    }

    /// One more run of a group's content with colours resolved for the device, replacing
    /// what a subtractive run drew. See [`Interpreter::group_commands`] for the two cases.
    fn rerun_on_device(
        &mut self,
        content: &NestedContent,
        resources: &Dictionary,
        inner: &GraphicsState,
        form_depth: usize,
        mark: usize,
    ) -> Vec<Command> {
        let rewind = self.readback_mark();
        self.run(content, resources, inner, form_depth.saturating_add(1));
        self.rewind_readback(rewind);
        self.list.split_off_commands(mark)
    }

    /// Reads a form `XObject`'s `/Group`, if it is a transparency group (§8.10.3, §11.6.6).
    ///
    /// `None` for a form with no `/Group` at all and for one whose group subtype is not
    /// `/Transparency`, which §11.6.6 makes the same case:
    ///
    /// > An ordinary form XObject -one having no Group entry -or having a Group entry with a
    /// > subtype other than Transparency -shall not be subject to any grouping behaviour for
    /// > transparency purposes.
    pub(super) fn transparency_group(&mut self, dict: &Dictionary) -> Option<TransparencyGroup> {
        let group = self.document.get_key(dict, "Group");
        let group = group.as_dict()?;
        // §8.10.3 Table 94: `/S` is required and "identifies the type of group whose
        // attributes this dictionary describes"; `/Transparency` is the only subtype the
        // specification defines.
        if self.document.get_key(group, "S").as_name()?.as_bytes() != b"Transparency" {
            return None;
        }
        // Table 145's `/I` and `/K`, both booleans defaulting to false.
        Some(TransparencyGroup {
            isolated: matches!(self.document.get_key(group, "I"), Object::Boolean(true)),
            knockout: matches!(self.document.get_key(group, "K"), Object::Boolean(true)),
            colour_space: self.document.get_key(group, "CS"),
        })
    }

    /// Reports §11.4.7's page group where its blending space is not the device's.
    ///
    /// > All page-level compositing shall be done in the default blending colour space of the
    /// > page, and the entire result shall then, if the colour spaces are not equivalent, be
    /// > converted to the native colour space of the output device before being composited
    /// > with the context-dependent backdrop.
    ///
    /// A page whose blending space has four components is drawn in those two orders' *agreeing*
    /// form since ADR 0262: the page is interpreted twice, once per half of the four, and
    /// `pdf_render::blending` converts the pair where the clause puts the conversion. Whose
    /// four they are is the document's since ADR 0272 — [`page_press`] is that reading. This
    /// fires for what is left — a page whose space is not four components this tree can sample,
    /// or one where the four would not answer the question — and only on the pass that
    /// composites on the device, since the subtractive passes are the ones that do not depart.
    ///
    /// Conditioned on something compositing, for [`Interpreter::note_group_departures`]'
    /// reason: an opaque `Normal` paint carries its colour through whatever space it is
    /// carried through, so a page of them is the same page in either.
    pub(super) fn note_page_blending_space(&mut self) {
        if self.compositing != Compositing::Device {
            return;
        }
        let Some(name) = self.blending.clone() else {
            return;
        };
        if !any_command(self.list.commands(), &command_composites) {
            return;
        }
        let because = self.blending_undrawable().unwrap_or(BeyondPress::stated(
            "its components are not four this tree can sample into a press, so §11.3.4 has \
                 no per-component formula to apply and no conversion back out",
        ));
        self.note(Unsupported::TransparencyGroup {
            detail: format!(
                "the page group's blending colour space {name} (§11.4.7): {}",
                because.why
            ),
        });
    }

    /// Why this page cannot be drawn in the blending space it states, or `None` if it can.
    ///
    /// Three conditions, each of which is a *different* clause asking for something the pair of
    /// rasters does not carry, and each named rather than folded into the others. All three
    /// want a **second colour space** — one the document names, one a group introduces, one
    /// whose black generation the file states.
    ///
    /// **A fourth was here until the four-hundred-and-forty-first session and it was not a
    /// second colour space at all**: §11.3.5.3's rule for the black component under Table 135's
    /// four modes, which this reported as "a blend function neither raster has". It is drawn
    /// rather than reported since ADR 0277, and nothing was written for it — the clause splits
    /// a subtractive space's four components along the same line the two rasters already are,
    /// its chromatic bullet is what [`crate::colour::Half::Chromatic`] holds, and the rule it
    /// gives the black component is what its own four functions return on the neutral colour
    /// [`crate::colour::Half::Black`] holds. `render-cpu`'s `blend` module has the derivation.
    /// **The order among them is not a reading rule any more, and ADR 0417 is why.** ADR 0416
    /// had to put a file-stated reason in front of the one [`crate::colour::MAX_PRESSES`]
    /// supplied, because that one was a fact about the process and reporting it in place of
    /// what the page says about itself made a verdict that moved between runs. Every reason
    /// here is the document's now, so the order is back to the plain one — the space itself
    /// first, then what a group inside did to it, then what an `/ExtGState` said about the
    /// conversion — and any of them is the same answer on every run.
    pub(super) fn blending_undrawable(&self) -> Option<BeyondPress> {
        if let Some(beyond) = self.blending_beyond {
            return Some(beyond);
        }
        if self.blending_changed {
            return Some(BeyondPress::stated(
                "a group inside it composites in a different space (§11.6.6), which needs a \
                 conversion between the two at its Do",
            ));
        }
        if self.black_generation_stated {
            return Some(BeyondPress::stated(
                "an /ExtGState states Table 57's black generation or undercolour removal, which \
                 §11.7.5.3 puts inside the conversion into the space",
            ));
        }
        self.blending_beyond
    }

    /// Reports the parts of §11.4 this group asks for and does not get.
    ///
    /// A group is composited here under its own constant alpha and blend mode, onto the
    /// backdrop [`GroupDrawn::isolated`] names. Three of Table 145's answers can ask for more
    /// than the display list carries, and each is reported only where it can change a pixel —
    /// a report that fires where the output is provably identical costs the page its place
    /// in the oracle's comparison and buys nothing.
    fn note_group_departures(
        &mut self,
        group: &TransparencyGroup,
        commands: &[Command],
        drawn: GroupDrawn,
        introduced: Option<&str>,
    ) {
        self.note_group_structure(group, commands, drawn);

        // §11.6.6: for an isolated group, a `/CS` means "all painting operators shall
        // convert source colours ... to the group colour space before compositing objects
        // into the group", and the result is interpreted in that space. Compositing here
        // happens on the device's RGB components, so a group asking for any other space is
        // blended with different arithmetic — visible only where something composites at
        // all, since an opaque Normal paint carries its colour through unchanged.
        //
        // `introduced` is `run_transparency_group`'s answer to *which* space, after
        // §11.6.6's inheritance and §11.7.2's rule about a non-isolated group. A group that
        // inherits a departing space is drawn no better and no worse than the page or group
        // that introduced it, and that one carries the report.
        if let Some(name) = introduced
            && self.compositing == Compositing::Device
            && any_command(commands, &command_composites)
        {
            self.note(Unsupported::TransparencyGroup {
                detail: format!("blending colour space {name}"),
            });
        }
    }

    /// Reports Table 145's `/I` and `/K`, which mean the same thing wherever a group is used.
    ///
    /// Split from [`Interpreter::note_group_departures`] because a *soft mask's* group asks
    /// the same two questions and a different colour-space question: §11.6.5.1 makes its
    /// `/CS` the space the mask's luminosity is computed in, where §11.6.6 makes a painted
    /// group's the space its elements are composited in. `crate::soft_mask` decides the
    /// first; this decides what the two share.
    ///
    /// [`GroupDrawn::backdrop_transparent`] is §11.4.6's NOTE 6: a group that is a direct
    /// element of a knockout group whose initial backdrop is transparent **is** §11.4.5's
    /// isolated group by that clause's own definition, whatever Table 145's `/I` says here,
    /// so both questions below are asked of that rather than of the entry. See
    /// [`Interpreter::transparent_initial_backdrop`], and `knockout_inner_backdrop.pdf` is
    /// the page that showed the difference: its inner group states `/I false` inside an
    /// isolated knockout group, is drawn on the transparency the clause asks for, and was
    /// reported as departing from it (ADR 0307).
    fn note_group_structure(
        &mut self,
        group: &TransparencyGroup,
        commands: &[Command],
        drawn: GroupDrawn,
    ) {
        let GroupDrawn {
            knockout: knockout_drawn,
            isolated: isolated_drawn,
            backdrop_transparent,
            alpha_sources,
        } = drawn;
        let isolated_by_clause = group.isolated || backdrop_transparent;
        // §11.4.4 composites a non-isolated group's elements onto the group's backdrop and
        // then removes that backdrop's contribution again (its NOTE 3). Under the Normal
        // blend mode the removal is exact and the backdrop cancels, which is what §11.6.7's
        // NOTE 1 states for the same computation applied to a pattern cell: "in the common
        // case in which the pattern consists entirely of objects painted with the Normal
        // blend mode, this behaviour can be optimised by treating the pattern cell as if it
        // were an isolated group. Since in this case the results depend only on the colour,
        // shape, and opacity of the pattern cell and not on those of the backdrop". So a
        // group all of whose elements paint Normal is drawn as an isolated one whatever it
        // says, and only a blend mode inside it can tell the difference — the same sentence
        // §11.4.4's NOTE 2 gives as the reason the two kinds of group differ at all.
        //
        // Where one does blend, the display list states the group's backdrop instead of
        // substituting §11.4.5's (ADR 0237), and `isolated_drawn` is false. What is left
        // here is the population that construction refuses: a knockout group, an element of
        // one, and a group composited under a blend mode of its own — plus a *mask* group,
        // which is evaluated into a raster built on transparency whatever it declares.
        if !isolated_by_clause
            && isolated_drawn
            && any_command(commands, &|command| command_blends(command))
        {
            self.note(Unsupported::TransparencyGroup {
                detail: "non-isolated, and an element blends with the backdrop it excludes"
                    .to_owned(),
            });
        }

        // §11.4.6: "In a knockout group, each individual element shall be composited with
        // the group's initial backdrop rather than with the stack of preceding elements in
        // the group." Where the upper of two overlapping elements is opaque and blends
        // Normal it overwrites either way, so the two models differ only where a later
        // element that composites covers an earlier one — which is the condition below,
        // and the same shape as §9.3.8's for a text object.
        // Since the seventy-first session the display list can carry the rule itself, for
        // the groups whose elements have a shape a rasteriser can draw — see
        // [`knockout_shape_is_coverage`] — and since ADR 0234 for those whose shape it can
        // *state*, see [`stated_shape`]. `knockout_drawn` answers both. What is left here
        // is the population they refuse, and the report says which of the two refused it:
        // a report that names its condition is worth more than one that names its clause,
        // and this one had named neither.
        if group.knockout && !knockout_drawn && knockout_can_show(commands) {
            // Ordered by how precisely each condition is known, not by how it is tested:
            // the first names an element, the second names this group, and the third names
            // a history of the group's whole run and over-approximates within it. More than
            // one can hold, and the most precise true statement is the one worth printing.
            // The first is asked only where there *is* a settled reading of `/AIS`, because
            // which elements have a statable shape is a different question under each — and
            // where there is not, the third is the answer by construction.
            let refusal = if let Some(element) = alpha_sources
                .settled()
                .and_then(|alpha| unstatable_shape(commands, alpha))
            {
                element
            } else if !isolated_by_clause && any_command(commands, &command_blends) {
                "non-isolated, and an element blends with the backdrop it excludes"
            } else {
                "/AIS was stated both ways while the content ran, and §11.6.4.3 gives the \
                 mask and the alpha constants a different meaning under each"
            };
            self.note(Unsupported::TransparencyGroup {
                detail: format!("knockout, and an element composites over another ({refusal})"),
            });
        }
    }
}
