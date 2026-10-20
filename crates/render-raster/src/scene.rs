//! Display list → raster scene: the translation the two vocabularies were shaped
//! to make small.
//!
//! One walk over the commands, mirroring the recursive structure (groups and soft
//! masks carry command lists of the same display list). Resources upload through
//! the [`Encoder`]'s caches — outlines and images by their `Arc` identity, exactly
//! the keying `RENDER_LIBRARY.md` section 2.2 asks for — or transiently for the
//! per-frame forms (clips, dashed strokes, meshes, sampled grids, area-averaged
//! images), which the caller releases after the frame.
//!
//! **One resource is neither, and it is the one that was got wrong**: a stroke this crate
//! expands itself under an anisotropic placement is computed geometry that a *stable* source
//! determines, so it is cached under [`crate::cache::StrokeKey`] — the source path's identity
//! plus the arguments — rather than uploaded fresh every frame. A transient's identifier moves
//! between two renders of one unchanged page, and raster keys its glyph atlas on that (ADR 0402).

use std::collections::HashMap;

use pdf_render::{
    BlendMode, ClipId, Command, DisplayList, FillRule, Rect, SoftMaskId, TargetSpec, Transform,
};
use raster_scene::{ResourceId, SceneBuilder};

use crate::QuorraRasterError;
use crate::cache::ResourceCaches;

mod clips;
mod fills;
mod groups;
mod images;
mod outlines;
mod own_space;
mod shadings;
mod vocabulary;

// What the other modules of this crate, and the concerns above, name by the scene's own path.
use groups::refuse_untranslatable_group;
pub(crate) use vocabulary::{affine, blend_mode, byte_colour, colour, compose, overprinted};
use vocabulary::{fill_rule, point, segments, spec};

/// The walk's state: the device and caches it uploads through, and the per-list
/// clip and mask tables it resolves against.
pub(crate) struct Encoder<'a> {
    device: &'a mut raster_gpu::Device,
    list: &'a DisplayList,
    /// The real device target, read by every *resolution* decision — mesh and function
    /// sampling, an image's filter, a stroke's split — each of which marks the scene
    /// view-consuming ([`Encoder::consumed_view`], ADR 0702).
    target: TargetSpec,
    /// What [`Encoder::placed`] composes geometry with: the real target where the
    /// placement is baked (the multi-page path), the identity where the scene is built
    /// in page space and the viewport carries the placement (ADR 0702).
    place: Transform,
    /// Whether anything in this scene read the view — a device-resolution decision
    /// whose answer is baked into the built scene. A scene that never did is true at
    /// every viewport and survives view changes (ADR 0702); one that did is rebuilt
    /// per view, exactly as every scene was before that ADR.
    consumed_view: bool,
    /// The round dots this scene drew as lines, each true at another view only where that view
    /// would draw it as a line too ([`crate::stroke::DotAsLine`], ADR 1877) — the one question a
    /// scene of lines still asks of the view.
    dots: Vec<crate::stroke::DotAsLine>,
    /// Whether the scene is built in page space (ADR 0702): geometry placed without
    /// the target, the viewport carrying it — and the one behavioural fork, §14.11.2.1's
    /// crop, emitted unconditionally because its early-out reads the view.
    page_space: bool,
    caches: &'a mut ResourceCaches,
    transient: &'a mut Vec<ResourceId>,
    functions: &'a mut FunctionPaints,
    clips: HashMap<usize, ResolvedClip>,
    /// Each clip's region as uploaded for this frame's target, by the list's clip id, and
    /// handed to every frame of the group drawn as frames of their own on the same target
    /// (ADR 1529): the region is a function of the clip and the target alone, so the second
    /// frame of a four-component group states its clips by the first frame's outlines.
    clip_outlines: HashMap<usize, ClipOutline>,
    /// The page-space rectangle each chain **admits**, where it is one — see
    /// [`Encoder::admitted_rect`], which is what decides whether a chain can be left off a
    /// mark it does not cut.
    admits: HashMap<usize, Option<Rect>>,
    masks: HashMap<usize, raster_scene::MaskId>,
    /// How many enclosing groups are ISO 32000-2 §11.4.6's knockout groups.
    ///
    /// Read by [`Self::inside_knockout`], and there for one question: inside such a group an
    /// element replaces its backdrop within its own **shape**, which raster reads off the
    /// coverage a mark is drawn with — so the coverage-as-alpha substitutions §10.7.4 asks for
    /// are withheld there, exactly as `render-cpu` withholds them under Porter-Duff Source.
    knockouts: u32,
    /// Whether the commands being translated are the **direct** elements of a §11.4.6
    /// knockout group, which is where [`overprinted`] leaves a mark unwrapped: see
    /// [`Self::knockout_element`].
    element_of_knockout: bool,
    /// §14.11.2.1's boundary as a clip every chain in this list hangs from, or `None`.
    ///
    /// See [`Encoder::crop_to_page`], which is the only thing that sets it and says why this
    /// backend states the clause in the scene where the other two state it in the pixels.
    root: Option<raster_scene::ClipId>,
    /// §14.11.2.1's boundary as a page-space rectangle, set with [`Encoder::root`] and read
    /// by [`Encoder::admitted_rect`]: the outermost link of every chain, and the only link of
    /// a command that states none.
    root_rect: Option<Rect>,
}

/// One clip's region as [`Encoder::clip_outline`] uploaded it: the outline, the rule it is
/// filled by, and whether building it read the view (ADR 0702), which a frame handed the
/// outline must record as its own.
#[derive(Debug, Clone, Copy)]
struct ClipOutline {
    outline: raster_scene::OutlineId,
    rule: FillRule,
    read_view: bool,
}

/// What this frame's §8.7.4.5.2 type 1 shadings did: how many the device evaluated, and the
/// ground on which it declined each of the rest.
///
/// **A count and a name rather than an error, because nothing failed.** A program the device
/// declines is drawn from the grid `pdf_render::Shading::sampled_at` produces (ADR 0364) —
/// the same construction every backend used before ADR 0376 and the one the correctness
/// oracle still uses — so the page is right either way and refusing the frame would be
/// refusing a page that draws. What would be wrong is drawing it *silently*: the two paths
/// cost four orders of magnitude apart, and a document that quietly stopped taking the fast
/// one is exactly the regression no timing on its own can attribute. So the ground is carried
/// out by name, `--trace` prints it beside the frame, and a test can assert on it.
#[derive(Debug, Clone, Default)]
pub struct FunctionPaints {
    evaluated: u32,
    refused: Vec<String>,
}

impl FunctionPaints {
    /// How many type 1 shadings this frame painted with a device-evaluated program.
    #[must_use]
    pub fn evaluated(&self) -> u32 {
        self.evaluated
    }

    /// One ground per shading the device declined, each of which drew from the grid instead.
    #[must_use]
    pub fn refusals(&self) -> &[String] {
        &self.refused
    }

    /// Forgets the last frame's answers.
    pub(crate) fn clear(&mut self) {
        self.evaluated = 0;
        self.refused.clear();
    }

    fn refuse(&mut self, ground: String) {
        self.refused.push(ground);
    }
}

/// What a non-solid paint resolved to.
pub(crate) enum ShadedPaint {
    /// A raster paint, ready to fill or stroke with.
    Ready(raster_scene::Paint),
    /// A sampled grid: not a raster paint — the caller draws it as an image
    /// clipped to the shape.
    Sampled,
    /// A shading with nothing visible to paint — a mesh whose raster is empty.
    /// pdf.js calls the document defective there (issue #17848, PR #17858 rejects
    /// the zero-extent bounds) and both sibling backends draw nothing, so nothing
    /// is what this draws too.
    Nothing,
}

/// A display-list clip chain, resolved once: either it admits nothing anywhere —
/// so every command under it is skipped whole, the same answer `render-gpu`
/// gives — or it is a raster clip chain.
#[derive(Debug, Clone, Copy)]
enum ResolvedClip {
    AdmitsNothing,
    Chain(raster_scene::ClipId),
}

/// What a command's clip admits: nothing at all (the command is skipped whole),
/// or a raster chain to draw under (`None` = unclipped).
#[derive(Debug, Clone, Copy)]
pub(crate) enum Admitted {
    Nothing,
    Chain(Option<raster_scene::ClipId>),
}

/// One transparency group's parameters, as [`Encoder::group`] takes them.
///
/// A struct rather than seven arguments because two callers state them: the page's own
/// groups, and §11.4.6's staged halves, which differ from the first only in the operator
/// the finished group composites with.
#[derive(Clone, Copy)]
struct GroupParts<'a> {
    /// The group's elements.
    commands: &'a [Command],
    /// §11.4.5's group alpha.
    alpha: f32,
    /// How the finished group combines with its backdrop (§11.3.5).
    blend: BlendMode,
    /// Active clip, or `None` for unclipped.
    clip: Option<ClipId>,
    /// Soft mask applied to the composited group, or `None` (§11.6.4.3).
    mask: Option<SoftMaskId>,
    /// Table 145's `/I`.
    isolated: bool,
    /// Whether the group is a knockout group (§11.4.6).
    knockout: bool,
    /// The blending colour space the group composites in, where it states one of its own
    /// (§11.6.6, §11.7.2).
    blending: Option<&'a pdf_render::GroupBlending>,
}

impl<'a> Encoder<'a> {
    pub(crate) fn new(
        device: &'a mut raster_gpu::Device,
        list: &'a DisplayList,
        target: TargetSpec,
        caches: &'a mut ResourceCaches,
        transient: &'a mut Vec<ResourceId>,
        functions: &'a mut FunctionPaints,
    ) -> Self {
        Self::with_placement(device, list, target, false, caches, transient, functions)
    }

    /// As [`Encoder::new`], but `page_space` builds the scene in page space (ADR 0702):
    /// geometry placed without the target, the viewport carrying the placement.
    pub(crate) fn with_placement(
        device: &'a mut raster_gpu::Device,
        list: &'a DisplayList,
        target: TargetSpec,
        page_space: bool,
        caches: &'a mut ResourceCaches,
        transient: &'a mut Vec<ResourceId>,
        functions: &'a mut FunctionPaints,
    ) -> Self {
        Self {
            device,
            list,
            target,
            place: if page_space {
                Transform::IDENTITY
            } else {
                target.transform
            },
            consumed_view: false,
            dots: Vec::new(),
            page_space,
            caches,
            transient,
            functions,
            clips: HashMap::new(),
            clip_outlines: HashMap::new(),
            admits: HashMap::new(),
            masks: HashMap::new(),
            knockouts: 0,
            element_of_knockout: false,
            root: None,
            root_rect: None,
        }
    }

    /// Whether this scene read the view while it was built (ADR 0702): true means the
    /// built scene is only correct at the target it was built for.
    pub(crate) fn consumed_view(&self) -> bool {
        self.consumed_view
    }

    /// The round dots this scene drew as lines (ADR 1877), handed to the scene that keeps them.
    pub(crate) fn take_dots(&mut self) -> Vec<crate::stroke::DotAsLine> {
        std::mem::take(&mut self.dots)
    }

    /// Notes a round dot drawn as a line, which a later view asks again before it keeps the scene.
    pub(crate) fn keep_dot(&mut self, dot: crate::stroke::DotAsLine) {
        self.dots.push(dot);
    }

    /// Translates one command list into the builder — the page's, a window's selection of it
    /// (ADR 1980), a group's or a soft mask's, which all share this walk so the cross-backend
    /// tests exercise every path a mask can take.
    pub(crate) fn commands<'c>(
        &mut self,
        builder: &mut SceneBuilder,
        commands: impl IntoIterator<Item = &'c Command>,
    ) -> Result<(), QuorraRasterError> {
        for command in commands {
            match command {
                Command::Fill {
                    path,
                    transform,
                    fill_rule,
                    paint,
                    clip,
                    mask,
                    blend,
                } => self.fill(
                    builder,
                    path,
                    *transform,
                    *fill_rule,
                    paint,
                    (*clip, *mask, *blend),
                )?,
                Command::Stroke {
                    path,
                    transform,
                    stroke,
                    paint,
                    clip,
                    mask,
                    blend,
                } => crate::stroke::encode(
                    self,
                    builder,
                    path,
                    *transform,
                    stroke,
                    paint,
                    (*clip, *mask, *blend),
                )?,
                Command::Image {
                    image,
                    transform,
                    alpha,
                    clip,
                    mask,
                    blend,
                } => self.image(builder, image, *transform, *alpha, (*clip, *mask, *blend))?,
                Command::Group {
                    commands,
                    alpha,
                    clip,
                    mask,
                    blend,
                    isolated,
                    knockout,
                    // §8.5.4's intersection of a group's shape with the clip at its blit is
                    // arithmetic this backend does not perform: the composite is
                    // `raster_scene::GroupSpec`'s, and the library multiplies the clip's
                    // coverage into the group's weight (`composite.wgsl`). So the flag is
                    // read by nothing here and the ask is `doc/QUORRA_FEEDBACK.md`
                    // section 36.
                    alpha_is_shape: _,
                    blending,
                } => {
                    refuse_untranslatable_group(*isolated, *knockout, blending.as_deref())?;
                    self.group(
                        builder,
                        GroupParts {
                            commands,
                            alpha: *alpha,
                            blend: *blend,
                            clip: *clip,
                            mask: *mask,
                            isolated: *isolated,
                            knockout: *knockout,
                            blending: blending.as_deref(),
                        },
                        raster_scene::Compose::SrcOver,
                    )?;
                }
                Command::Shaped { object, shape } => self.shaped(builder, object, shape)?,
                // `Command` is non-exhaustive: a variant added upstream must fail
                // loudly here, never fall through as a hole in the page.
                other => {
                    return Err(QuorraRasterError::Unsupported(format!(
                        "display-list command {other:?}"
                    )));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn target(&self) -> TargetSpec {
        self.target
    }

    /// A page-space transform placed onto the scene: the target's transform is
    /// baked into every command here, and the raster viewport stays identity —
    /// which is what lets one scene carry the page, a fallback raster and
    /// window-pixel overlays at their own placements ([`crate::QuorraWindowRenderer`]).
    pub(crate) fn placed(&self, t: Transform) -> raster_scene::Affine {
        affine(t.then(self.place))
    }

    /// Marks this scene as having read the view (ADR 0702): a device-resolution
    /// decision — sampling, a filter, a split — whose answer the built scene now
    /// carries, so the scene is only true at the target it was built for.
    pub(crate) fn consume_view(&mut self) {
        self.consumed_view = true;
    }
}
