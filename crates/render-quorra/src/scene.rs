//! Display list → quorra scene: the translation the two vocabularies were shaped
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
//! between two renders of one unchanged page, and quorra keys its glyph atlas on that (ADR 0402).

use std::collections::HashMap;
use std::sync::Arc;

use pdf_render::{
    BlendMode, ClipId, Color, Command, DisplayList, FillRule, Image, ImageSource, Paint, Path,
    PathCommand, Point, ProgramOperator, ProgramRange, ProgramStep, Shading, ShadingKind,
    SoftMaskId, SoftMaskKind, TargetSpec, Transform,
};
use quorra_scene::{ResourceId, SceneBuilder};

use crate::QuorraRasterError;
use crate::cache::ResourceCaches;

/// The factor pair that stands for an image uploaded whole, as its own key in the
/// resource cache: one source sample per output sample, in both axes.
const WHOLE: (u32, u32) = (1, 1);

/// The walk's state: the device and caches it uploads through, and the per-list
/// clip and mask tables it resolves against.
pub(crate) struct Encoder<'a> {
    device: &'a mut quorra_gpu::Device,
    list: &'a DisplayList,
    target: TargetSpec,
    caches: &'a mut ResourceCaches,
    transient: &'a mut Vec<ResourceId>,
    functions: &'a mut FunctionPaints,
    clips: HashMap<usize, ResolvedClip>,
    masks: HashMap<usize, quorra_scene::MaskId>,
    /// How many enclosing groups are ISO 32000-2 §11.4.6's knockout groups.
    ///
    /// Read by [`Self::inside_knockout`], and there for one question: inside such a group an
    /// element replaces its backdrop within its own **shape**, which quorra reads off the
    /// coverage a mark is drawn with — so the coverage-as-alpha substitutions §10.7.4 asks for
    /// are withheld there, exactly as `render-cpu` withholds them under Porter-Duff Source.
    knockouts: u32,
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
    /// A quorra paint, ready to fill or stroke with.
    Ready(quorra_scene::Paint),
    /// A sampled grid: not a quorra paint — the caller draws it as an image
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
/// gives — or it is a quorra clip chain.
#[derive(Debug, Clone, Copy)]
enum ResolvedClip {
    AdmitsNothing,
    Chain(quorra_scene::ClipId),
}

/// What a command's clip admits: nothing at all (the command is skipped whole),
/// or a quorra chain to draw under (`None` = unclipped).
#[derive(Debug, Clone, Copy)]
pub(crate) enum Admitted {
    Nothing,
    Chain(Option<quorra_scene::ClipId>),
}

/// Chains are acyclic by the display list's construction; the cap exists so a
/// hostile or corrupted list is an error rather than a stack overflow. 4096 links
/// is far beyond any real chain (the corpus's worst page nests clips a handful
/// deep across 3 608 chains).
const MAX_CLIP_DEPTH: usize = 4096;

/// One transparency group's parameters, as [`Encoder::group`] takes them.
///
/// A struct rather than seven arguments because two callers state them: the page's own
/// groups, and §11.4.6's staged halves, which differ from the first only in the operator
/// the finished group composites with.
/// Refuses the two shapes of `Command::Group` quorra's vocabulary cannot state.
///
/// - **A non-isolated knockout group** (`isolated: false` beside `knockout: true`):
///   §11.4.6 composites each element with the group's *initial* backdrop — here the
///   group's own — which needs that backdrop retained beside the accumulation and a
///   scratch per element. A `GroupSpec` carries the two flags but a scene states no
///   per-element backdrop, and quorra's staged `DestOut`/`Plus` pair is written on the
///   transparent start §11.4.5 gives (its ADRs 0025, 0032). Passing the flags through
///   would substitute one backdrop for the other in silence.
/// - **A group compositing in a four-component blending colour space** (§11.6.6, §11.7.2,
///   `blending: Some`): the pair's colours are ink complements resolved per pixel after
///   the group composites, and a scene under composition cannot be read back. The page
///   -level pair is drawn by two whole `render` passes (ADR 0275); a group-scoped one has
///   no lane.
///
/// Both go to the CPU backend, which draws them; refusing here is what keeps either from
/// becoming a wrong picture in silence, which is trap 5.
fn refuse_untranslatable_group(
    isolated: bool,
    knockout: bool,
    in_own_space: bool,
) -> Result<(), QuorraRasterError> {
    if knockout && !isolated {
        return Err(QuorraRasterError::Unsupported(
            "a non-isolated knockout group: each element composites with the group's own \
             initial backdrop, which a scene cannot retain beside the accumulation \
             (ISO 32000-2 §11.4.6)"
                .to_owned(),
        ));
    }
    if in_own_space {
        return Err(QuorraRasterError::Unsupported(
            "a group compositing in a blending colour space of four components: the pair \
             resolves per pixel after the group composites (ISO 32000-2 §11.6.6, §11.7.2)"
                .to_owned(),
        ));
    }
    Ok(())
}

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
}

impl<'a> Encoder<'a> {
    pub(crate) fn new(
        device: &'a mut quorra_gpu::Device,
        list: &'a DisplayList,
        target: TargetSpec,
        caches: &'a mut ResourceCaches,
        transient: &'a mut Vec<ResourceId>,
        functions: &'a mut FunctionPaints,
    ) -> Self {
        Self {
            device,
            list,
            target,
            caches,
            transient,
            functions,
            clips: HashMap::new(),
            masks: HashMap::new(),
            knockouts: 0,
        }
    }

    /// Whether this command is being encoded inside ISO 32000-2 §11.4.6's knockout group.
    ///
    /// See [`Self::knockouts`]: it is the one place a coverage folded into a paint's alpha would
    /// be read back as opacity, so the substitutions §10.7.4 asks for are withheld under it.
    pub(crate) fn inside_knockout(&self) -> bool {
        self.knockouts > 0
    }

    /// Translates one command list into the builder — the page's, a group's or a
    /// soft mask's, which all share this walk so the cross-backend tests exercise
    /// every path a mask can take.
    pub(crate) fn commands(
        &mut self,
        builder: &mut SceneBuilder,
        commands: &[Command],
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
                    blending,
                } => {
                    refuse_untranslatable_group(*isolated, *knockout, blending.is_some())?;
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
                        },
                        quorra_scene::Compose::SrcOver,
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

    /// One transparency group, composited by `compose` (ISO 32000-2 §11.4.1).
    ///
    /// See [`refuse_untranslatable_group`] for the two shapes of the command every caller
    /// screens out before building a [`quorra_scene::GroupSpec`].
    ///
    /// `compose` is [`quorra_scene::Compose::SrcOver`] for every group a page states, and one
    /// of §11.4.6's two staged operators where this group is one half of a
    /// [`Command::Shaped`] — see [`Self::shaped`].
    fn group(
        &mut self,
        builder: &mut SceneBuilder,
        parts: GroupParts<'_>,
        compose: quorra_scene::Compose,
    ) -> Result<(), QuorraRasterError> {
        let Admitted::Chain(clip) = self.clip_chain(builder, parts.clip)? else {
            return Ok(()); // the clip admits nothing: the group draws nothing
        };
        let mask = self.mask_id(builder, parts.mask)?;
        let spec = quorra_scene::GroupSpec {
            alpha: parts.alpha,
            blend: blend_mode(parts.blend),
            clip,
            knockout: parts.knockout,
            mask,
            compose,
            // Table 145's `/I`, straight through. §11.4.5's isolated group is
            // what a layer in any rasterising library is; §11.4.4's other
            // initial backdrop — "the group's backdrop" — is the one quorra
            // gained in its ADR 0019, and the flag is how a scene asks for it.
            //
            // **The three conditions are not re-checked here, and that is
            // deliberate.** `pdf-model` emits `isolated: false` only where the
            // group's own blend is Normal, it is not a knockout group and no
            // enclosing group is one (ADR 0237, and `Command::Group`'s
            // `isolated` states the guarantee); quorra accepts exactly that set
            // and refuses the rest at `SceneBuilder::group` as
            // `SceneError::NonIsolatedGroupUnsupported`, which arrives below as
            // a typed `QuorraRasterError::Scene` naming which condition broke.
            // A copy of the condition here would be a second reading of §11.4.4
            // free to drift from the one that decides the picture.
            isolated: parts.isolated,
        };
        let mut walked = Ok(());
        // §11.4.6's group is counted around the walk rather than tested inside it, because what
        // its elements may not do — carry a coverage in the paint's alpha — is true of every
        // element at every depth under it, not only of its immediate children.
        let outer = self.knockouts;
        self.knockouts = outer.saturating_add(u32::from(parts.knockout));
        builder.group(spec, |body| {
            walked = self.commands(body, parts.commands);
            // The builder's own error channel carries scene refusals;
            // upload and translation errors travel beside it.
            Ok(())
        })?;
        self.knockouts = outer;
        walked
    }

    /// §11.4.6's two stages, for an element whose shape the display list states apart from
    /// its alpha ([`Command::Shaped`]).
    ///
    /// On the transparent initial backdrop a group is built on, the clause's weighted
    /// average is one line per pixel in premultiplied form — with the accumulated result
    /// `P`, the element's shape `f` and its premultiplied colour `S`:
    ///
    /// ```text
    /// P' = (1 − f) × P + S
    /// ```
    ///
    /// which is Porter-Duff Destination-Out with the shape half, then Plus with the object.
    /// [`quorra_scene::Compose::Src`] cannot say it: that operator reads the shape off the
    /// coverage a mark is drawn with, which is the assumption this element exists to
    /// contradict.
    ///
    /// **Both halves are emitted or neither is**, which is this caller's obligation rather
    /// than something the library can check — `Plus` alone drives a premultiplied channel
    /// past its alpha, and one mark cannot tell a library that the other is coming
    /// ([`quorra_scene::Compose::Plus`]). The two halves carry the same clip and the same
    /// geometry (`pdf_model`'s `stated_shape` derives one from the other), so every route
    /// that draws nothing removes both.
    ///
    /// Outside a knockout group the shape is unused and the object would be drawn alone —
    /// but [`Command::Shaped`] guarantees this command occurs nowhere else, so there is no
    /// such branch here.
    fn shaped(
        &mut self,
        builder: &mut SceneBuilder,
        object: &Command,
        shape: &Command,
    ) -> Result<(), QuorraRasterError> {
        self.stage(builder, shape, quorra_scene::Compose::DestOut)?;
        self.stage(builder, object, quorra_scene::Compose::Plus)
    }

    /// One half of [`Self::shaped`], drawn with the operator that stage is.
    ///
    /// A **group** states the operator itself: §11.3.7.2 makes a group's shape "the union
    /// […] of the shapes of the objects it contains", which no single mark can state, and
    /// `quorra_scene::GroupSpec::compose` is where quorra took that ask (its ADR 0033).
    ///
    /// Everything else is drawn inside a group of one element, which is the same
    /// arithmetic: an isolated group at alpha 1 under no mask, clip or blend holds exactly
    /// the element's own premultiplied colour, so compositing it is compositing the
    /// element. That is one buffer per staged mark and it buys uniformity — `stroke` and
    /// `image` carry no compositing operator in this vocabulary at all, and a `fill` whose
    /// paint is a sampled shading is drawn as an image too (see [`Self::sampled_fill`]), so
    /// a per-mark route would be correct for some paints and silently wrong for the rest.
    fn stage(
        &mut self,
        builder: &mut SceneBuilder,
        half: &Command,
        compose: quorra_scene::Compose,
    ) -> Result<(), QuorraRasterError> {
        if let Command::Group {
            commands,
            alpha,
            clip,
            mask,
            blend,
            isolated,
            knockout,
            blending,
        } = half
        {
            refuse_untranslatable_group(*isolated, *knockout, blending.is_some())?;
            return self.group(
                builder,
                GroupParts {
                    commands,
                    alpha: *alpha,
                    blend: *blend,
                    clip: *clip,
                    mask: *mask,
                    isolated: *isolated,
                    knockout: *knockout,
                },
                compose,
            );
        }
        self.group(
            builder,
            GroupParts {
                commands: std::slice::from_ref(half),
                alpha: 1.0,
                blend: BlendMode::Normal,
                // The element carries its own clip and mask, which the walk below applies;
                // stating them here as well would multiply each in twice.
                clip: None,
                mask: None,
                isolated: true,
                knockout: false,
            },
            compose,
        )
    }

    fn fill(
        &mut self,
        builder: &mut SceneBuilder,
        path: &Arc<Path>,
        transform: Transform,
        rule: FillRule,
        paint: &Paint,
        (clip, mask, blend): (Option<ClipId>, Option<SoftMaskId>, BlendMode),
    ) -> Result<(), QuorraRasterError> {
        if path.is_empty() {
            return Ok(());
        }
        let Admitted::Chain(clip) = self.clip_chain(builder, clip)? else {
            return Ok(());
        };
        let mask = self.mask_id(builder, mask)?;
        // ISO 32000-2 §10.7.4: no shape may disappear, and a subpath with no
        // extent along one axis has zero area for every coverage rasteriser. The
        // split — which subpaths enclose area, which become one-device-pixel
        // marks — is `pdf-render`'s, stated once for every backend (the viewer's
        // ADR 0154; QUORRA_FEEDBACK.md section 1 was a page of ruling lines drawn
        // blank). Marks fill under the **non-zero** rule whatever the command's
        // own rule is: a mark is a shape in its own right, and adding it to an
        // even-odd path's winding would punch a hole in what it should draw.
        let split = pdf_render::split_collapsed_fill(path, transform.then(self.target.transform));
        if let Some(split) = split {
            if !split.marks.is_empty() {
                let marks = self.transient_outline(&split.marks)?;
                self.emit_fill(
                    builder,
                    (marks, transform, FillRule::NonZero),
                    paint,
                    (clip, mask, blend),
                    path,
                )?;
            }
            if split.filled.is_empty() {
                return Ok(());
            }
            let filled = self.transient_outline(&split.filled)?;
            return self.emit_fill(
                builder,
                (filled, transform, rule),
                paint,
                (clip, mask, blend),
                path,
            );
        }

        let outline = self.outline(path)?;
        self.emit_fill(
            builder,
            (outline, transform, rule),
            paint,
            (clip, mask, blend),
            path,
        )
    }

    /// One fill, geometry already uploaded: the paint decides the lane.
    ///
    /// `shape` is the fill's **whole** path, before §10.7.4's collapsed-subpath split, and it is a
    /// path rather than the rectangle it used to be for a measured reason. The only thing that
    /// rectangle is wanted for is the device window a radial cone is evaluated over — a question a
    /// solid colour never asks — and taking it eagerly in [`Self::fill`] meant every fill paid
    /// [`Path::bounds`], which on its first call walks the whole path to memoise a hull. On the
    /// project owner's drawing that is 58 003 fills, none of them a shading, and one walk of
    /// 3.0 M path segments for an answer nothing reads.
    ///
    /// **Asking inside the one branch that reads it took 267.3 M instructions off an open plus
    /// two frames — 0.99 % of the whole program — and 20.5 % off the scene phase of a zoom frame**
    /// (callgrind, and minima of ten interleaved rounds on the owner's own adapter; ADR 0387).
    /// The pixels cannot move: the value's only consumer is [`Self::radial_cone`], which now
    /// receives exactly what it received before.
    ///
    /// It stays the *whole* path rather than the split pieces so that the three calls in
    /// [`Self::fill`] still share one conservative bound instead of measuring three.
    fn emit_fill(
        &mut self,
        builder: &mut SceneBuilder,
        (outline, transform, rule): (quorra_scene::OutlineId, Transform, FillRule),
        paint: &Paint,
        (clip, mask, blend): (
            Option<quorra_scene::ClipId>,
            Option<quorra_scene::MaskId>,
            BlendMode,
        ),
        shape: &Path,
    ) -> Result<(), QuorraRasterError> {
        // §8.7.4.5.4's cone, where the clause's "greatest value of s" can be a root the
        // shading's own `/Extend` refuses and the answer is the other one. No two-point
        // conical gradient expresses that, so all three backends leave their gradient and
        // draw `pdf_render::RadialRaster`'s bytes — identical bytes, which is the point.
        // Only a *fill* takes this door, exactly as in the sibling backends: a stroke's
        // outline is not the shape quorra is handed, and no corpus document strokes a cone.
        if let Paint::Shading(shading) = paint
            && let Some(cone) = self.radial_cone(shading, self.device_pixels(shape, transform))?
        {
            return builder
                .fill(
                    outline,
                    self.placed(transform),
                    fill_rule(rule),
                    quorra_scene::Paint::Mesh(cone),
                    clip,
                    blend_mode(blend),
                    quorra_scene::Compose::SrcOver,
                    mask,
                )
                .map_err(Into::into);
        }
        let paint = match paint {
            Paint::Solid(c) => quorra_scene::Paint::Solid(colour(*c)),
            Paint::Shading(shading) => match self.shading_paint(shading)? {
                ShadedPaint::Ready(paint) => paint,
                // A sampled shading is not a quorra paint: it draws as an image
                // clipped to this fill's path instead.
                ShadedPaint::Sampled => {
                    return self.sampled_fill(
                        builder,
                        (outline, transform, rule),
                        shading,
                        (clip, mask, blend),
                    );
                }
                ShadedPaint::Nothing => return Ok(()),
            },
            other => {
                return Err(QuorraRasterError::Unsupported(format!("paint {other:?}")));
            }
        };
        builder.fill(
            outline,
            self.placed(transform),
            fill_rule(rule),
            paint,
            clip,
            blend_mode(blend),
            quorra_scene::Compose::SrcOver,
            mask,
        )?;
        Ok(())
    }

    /// What a shading resolves to: a quorra paint, the sampled kind (drawn as an
    /// image by the caller), or nothing at all.
    pub(crate) fn shading_paint(
        &mut self,
        shading: &Shading,
    ) -> Result<ShadedPaint, QuorraRasterError> {
        let kind = match shading.kind.as_ref() {
            ShadingKind::Axial {
                start, end, extend, ..
            } => quorra_scene::ShadingKind::Axial {
                start: point(*start),
                end: point(*end),
                extend: *extend,
            },
            ShadingKind::Radial {
                start,
                start_radius,
                end,
                end_radius,
                extend,
                ..
            } => quorra_scene::ShadingKind::Radial {
                start: point(*start),
                start_radius: *start_radius,
                end: point(*end),
                end_radius: *end_radius,
                extend: *extend,
            },
            // §8.7.4.5.2's type 1 shading has two statements of one answer, and this is
            // where they are chosen between: the device evaluates the program where it can,
            // and where it cannot the grid below draws exactly what it always drew.
            ShadingKind::Sampled { .. } => {
                return Ok(match self.function_paint(shading)? {
                    Some(paint) => ShadedPaint::Ready(paint),
                    None => ShadedPaint::Sampled,
                });
            }
            ShadingKind::Mesh { triangles, ramp } => {
                return Ok(
                    match self.mesh(triangles, ramp.as_ref(), shading.transform)? {
                        Some(mesh) => ShadedPaint::Ready(quorra_scene::Paint::Mesh(mesh)),
                        None => ShadedPaint::Nothing,
                    },
                );
            }
            other => {
                return Err(QuorraRasterError::Unsupported(format!("shading {other:?}")));
            }
        };
        let ramp = self.ramp(shading)?;
        Ok(ShadedPaint::Ready(quorra_scene::Paint::Shading {
            ramp,
            kind,
            // quorra anchors a shading through its own transform, exactly as the
            // display list states it (§8.7.4.3's shading matrix).
            transform: self.placed(shading.transform),
        }))
    }

    /// §8.7.4.5.2's type 1 shading as a colour the *device* evaluates, where this device
    /// will take it — quorra's `Paint::Function`, ADR 0376.
    ///
    /// `Ok(None)` is not a failure and never becomes one: it means the caller draws the grid,
    /// which is what [`Encoder::sampled_fill`] does and what every backend did before this
    /// existed. Every `None` that is the *device's* answer rather than the display list's
    /// carries its ground into [`FunctionPaints`] on the way out, so a page that stops taking
    /// this path says why.
    ///
    /// # The order the three questions are asked in, and why none of them is asked late
    ///
    /// 1. **Is there a program at all?** `pdf_model::shading` builds one only where the two
    ///    paths compute the same colour; a shading of another function type, or over a colour
    ///    space needing a conversion, arrives with `None` and is not the device's business.
    /// 2. **Will the device execute it?** [`quorra_gpu::function::admit`] — the structural
    ///    check, the shader-generation walk and ADR 0053's agreement classification. It needs
    ///    no adapter, which is why it is asked here rather than at the upload: a refusal then
    ///    costs no resource and no release.
    /// 3. **Does it supply this *shading's* `Range`?** `Analysis::admits`, which is a
    ///    separate question because §7.10.5.3's "shall be an error for the number of remaining
    ///    operands to differ" compares the program against a `Range` the *paint* states, and
    ///    one uploaded program may serve two shadings.
    ///
    /// Questions 2 and 3 are answered on every scene build rather than remembered beside the
    /// cached id. That is deliberate and it is cheap: the walk is linear in a program bounded
    /// at [`quorra_gpu::function::MAX_PROGRAM_LENGTH`] steps — microseconds — against the
    /// milliseconds of grid the fallback would otherwise build, and it keeps the refusal a
    /// decision this frame took rather than one a cache entry remembers.
    fn function_paint(
        &mut self,
        shading: &Shading,
    ) -> Result<Option<quorra_scene::Paint>, QuorraRasterError> {
        let (Some(program), ShadingKind::Sampled { domain, .. }) =
            (shading.device_program(), shading.kind.as_ref())
        else {
            return Ok(None);
        };
        let range = function_range(program.range());
        let steps: Vec<quorra_scene::FnOp> =
            program.steps().iter().copied().map(function_op).collect();

        let analysis = match quorra_gpu::function::admit(&steps) {
            Ok(analysis) => analysis,
            Err(problem) => {
                self.functions.refuse(problem.to_string());
                return Ok(None);
            }
        };
        if let Err(problem) = analysis.admits(range) {
            self.functions.refuse(problem.to_string());
            return Ok(None);
        }

        // Table 78's `/Domain` is `[x_min, x_max, y_min, y_max]`; a scene's rectangle is two
        // corners and is refused unordered rather than repaired, so a file that writes the
        // pair the other way round draws from the grid — which reads it as a signed span and
        // is right either way.
        let [x0, x1, y0, y1] = *domain;
        let region = quorra_scene::Rect::new(
            quorra_scene::Point::new(x0, y0),
            quorra_scene::Point::new(x1, y1),
        );
        let matrix = self.placed(shading.transform);
        if !region.is_finite() || !region.is_ordered() {
            self.functions.refuse(format!(
                "the shading's /Domain {domain:?} is not a rectangle"
            ));
            return Ok(None);
        }
        if matrix.invert().is_none() {
            // §8.7.4.5.2's `Matrix` carries the domain into the page; a fragment has to go
            // the other way to know where in the domain it stands, and a collapsed domain has
            // no such point. The grid draws a degenerate placement as the nothing it is.
            self.functions
                .refuse("the shading's /Matrix has no inverse".to_owned());
            return Ok(None);
        }

        let paint = quorra_scene::Paint::Function {
            program: self.function(program.steps(), &steps)?,
            domain: region,
            matrix,
            range,
            // §8.7.4.5.2's `Background` "shall be applied only when the shading is used as
            // part of a shading pattern, not when painted directly with the `sh` operator",
            // and this tree reads no `/Background` at all — on either path — so points
            // outside the domain rectangle are left unpainted, which is what that entry's
            // absence means and what `sampled_fill` also draws.
            background: None,
        };
        // The budgets the scene boundary states over a rectangle and a transform are quorra's
        // own numbers, so they are asked with quorra's own predicate rather than restated
        // here. Reaching this is a shading placed further from the page than a scene admits.
        if !paint.is_valid() {
            self.functions.refuse(format!(
                "the shading's /Domain {domain:?} under its /Matrix is outside the scene's bounds"
            ));
            return Ok(None);
        }
        self.functions.evaluated = self.functions.evaluated.saturating_add(1);
        Ok(Some(paint))
    }

    /// The uploaded program, from the cache or freshly admitted by this device.
    ///
    /// **Cached rather than transient, and that is the one place a program differs from every
    /// other per-frame resource here.** A device keys its generated shader on the program's
    /// contents and drops it when the last id naming them is released (quorra's ADR 0053), so
    /// a program uploaded and released around each frame would recompile a shader on every
    /// frame of a still page — the cold compile `doc/QUORRA_FUNCTION_PAINT_ANSWER.md` measures
    /// at 6.3 ms, arriving once a frame instead of once. The key is the step list's `Arc`, so
    /// a display list rebuilt from scratch re-uploads it exactly as it re-uploads an outline.
    fn function(
        &mut self,
        key: &Arc<[ProgramStep]>,
        steps: &[quorra_scene::FnOp],
    ) -> Result<quorra_scene::FunctionId, QuorraRasterError> {
        if let Some(id) = self.caches.program(key) {
            return Ok(id);
        }
        let id = self.device.upload_function(steps)?;
        self.caches.store_program(key, id);
        Ok(id)
    }

    /// A sampled shading — the display list's grid stand-in for a function-based
    /// shading — drawn as a linearly-filtered image over the domain rectangle,
    /// clipped to the filled path (integration note 9 in quorra's plan).
    ///
    /// Stated divergence: the CPU backend pads the grid's edge colours beyond the
    /// domain rectangle; this draws the domain exactly and nothing beyond it. The
    /// `render-gpu` backend refuses sampled shadings outright, so the swap still
    /// strictly widens what draws.
    fn sampled_fill(
        &mut self,
        builder: &mut SceneBuilder,
        (outline, transform, rule): (quorra_scene::OutlineId, Transform, FillRule),
        shading: &Shading,
        (clip, mask, blend): (
            Option<quorra_scene::ClipId>,
            Option<quorra_scene::MaskId>,
            BlendMode,
        ),
    ) -> Result<(), QuorraRasterError> {
        if !matches!(shading.kind.as_ref(), ShadingKind::Sampled { .. }) {
            // The caller matched Sampled before dispatching here.
            return Err(QuorraRasterError::Unsupported(format!(
                "shading {:?}",
                shading.kind
            )));
        }

        // The colours are produced here, where the device scale is known — the grid
        // `pdf_render` derives from the domain's own placement, and the block of it this
        // target can sample, so no backend picks either for itself (`Shading::sampled_at`,
        // ADR 0408). Transient for the deferred image's reason: the grid is this placement's
        // and its `Arc` is this frame's, so there is no identity for a cache to be keyed by.
        let grid = shading
            .sampled_at(
                self.target.transform,
                (self.target.width, self.target.height),
            )
            .ok_or_else(|| QuorraRasterError::Unsupported(format!("shading {:?}", shading.kind)))?;
        let [x0, x1, y0, y1] = grid.covers;
        if x1 - x0 == 0.0 || y1 - y0 == 0.0 {
            return Err(QuorraRasterError::Unsupported(
                "a sampled shading with a degenerate domain".into(),
            ));
        }

        // The grid's row 0 sits at the domain's y0; quorra's image convention puts
        // the data's first row at unit y = 1 (ISO 32000-2 §8.9.5), so the rows
        // reverse on the way in.
        let row_len = (grid.width as usize).saturating_mul(4);
        let mut data = Vec::with_capacity(row_len.saturating_mul(grid.height as usize));
        for row in (0..grid.height as usize).rev() {
            let start = row.saturating_mul(grid.width as usize);
            for c in grid
                .pixels
                .get(start..start.saturating_add(grid.width as usize))
                .unwrap_or(&[])
            {
                data.extend_from_slice(&byte_colour(*c));
            }
        }
        let image = self.device.upload_image(&quorra_scene::ImageSpec {
            width: grid.width,
            height: grid.height,
            data: Arc::from(data.as_slice()),
        })?;
        self.transient.push(image.into());

        // Clip to the filled path: quorra clips compose by chaining, so the fill's
        // own geometry becomes one more link under the command's clip.
        let shape_clip = builder.clip(outline, self.placed(transform), fill_rule(rule), clip)?;
        // Unit square → the part of the domain these cells cover → shading space → page.
        let placement = grid.onto_shading().then(shading.transform);
        builder.image(
            image,
            self.placed(placement),
            1.0,
            quorra_scene::ImageFilter::Linear,
            Some(shape_clip),
            blend_mode(blend),
            mask,
        )?;
        Ok(())
    }

    fn image(
        &mut self,
        builder: &mut SceneBuilder,
        source: &ImageSource,
        transform: Transform,
        alpha: f32,
        (clip, mask, blend): (Option<ClipId>, Option<SoftMaskId>, BlendMode),
    ) -> Result<(), QuorraRasterError> {
        let Admitted::Chain(clip) = self.clip_chain(builder, clip)? else {
            return Ok(());
        };
        let mask = self.mask_id(builder, mask)?;
        let placement = transform.then(self.target.transform);

        // Samples the display list deferred are produced here, where the device scale is
        // known — §11.6.5.2's mask on a grid of its own, at the grid `pdf_render` derives from
        // the placement so that no backend picks its own. Those stay *transient*: the samples
        // are made for this placement and their `Arc` is this frame's, so there is no identity
        // for a cache to be keyed by and an entry would be a leak with a lookup on it.
        let resolved = source.at(placement);
        let image: &Image = &resolved;
        let deferred = matches!(source, ImageSource::AtDeviceScale(_));

        // The two decisions RENDER_LIBRARY.md section 4.5 settles on this side of the boundary: area
        // averaging for minification, and the resolved smoothing for the placement.
        let (id, smoothed) = if deferred {
            let reduced = image.area_averaged(placement);
            let uploaded: &Image = reduced.as_ref().unwrap_or(image);
            let id = self.device.upload_image(&spec(uploaded))?;
            self.transient.push(id.into());
            (id, uploaded.is_smoothed(placement))
        } else {
            match image.reduction(placement) {
                Some(reduction) => (
                    self.reduced_image(image, placement, reduction)?,
                    reduction.smoothed,
                ),
                None => (self.cached_image(image)?, image.is_smoothed(placement)),
            }
        };
        let filter = if smoothed {
            quorra_scene::ImageFilter::Linear
        } else {
            quorra_scene::ImageFilter::Nearest
        };
        builder.image(
            id,
            self.placed(transform),
            alpha,
            filter,
            clip,
            blend_mode(blend),
            mask,
        )?;
        Ok(())
    }

    /// The device pixels a path covers, clamped to the target.
    ///
    /// Half a pixel of margin on each side, because a pixel is sampled at its centre and a
    /// shape ending at x = 10.0 still covers the sample at 9.5 — `MeshRaster::build`'s own
    /// margin, and the same bound the sibling backends compute for a radial cone.
    ///
    /// **Not free, which is why [`Self::emit_fill`] asks it only where a cone will read it.**
    /// [`Path::bounds`] walks every control point on its first call for a path, and a display
    /// list of ordinary fills would otherwise pay that walk for a number none of them uses.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "each cast is between a device pixel index and its coordinate, both bounded \
                  by the target's extent"
    )]
    fn device_pixels(&self, path: &Path, transform: Transform) -> (u32, u32, u32, u32) {
        let Some(bounds) = path.bounds(transform.then(self.target.transform)) else {
            return (0, 0, self.target.width, self.target.height);
        };
        (
            (bounds.min.x - 0.5).floor().max(0.0) as u32,
            (bounds.min.y - 0.5).floor().max(0.0) as u32,
            (bounds.max.x + 0.5)
                .ceil()
                .max(0.0)
                .min(self.target.width as f32) as u32,
            (bounds.max.y + 0.5)
                .ceil()
                .max(0.0)
                .min(self.target.height as f32) as u32,
        )
    }

    /// A radial shading exactly evaluated, where §8.7.4.5.4 and a gradient part company.
    ///
    /// `None` for every shading that is not a *cone* — one whose circles are further apart
    /// than their radii differ, which is the exact condition under which a point can lie on
    /// two blend circles and the clause's tie-break can change a pixel. The proof is in
    /// `render_cpu::shading::is_a_cone`; the arithmetic is
    /// [`pdf_render::blend_parameter`]'s.
    ///
    /// The raster is uploaded as a **mesh**, because quorra's mesh paint is precisely "an
    /// RGBA raster already at device resolution, placed at (left, top)" — which is what this
    /// is, and what the other two backends draw for the same geometry.
    fn radial_cone(
        &mut self,
        shading: &Shading,
        within: (u32, u32, u32, u32),
    ) -> Result<Option<quorra_scene::MeshId>, QuorraRasterError> {
        let ShadingKind::Radial {
            start,
            start_radius,
            end,
            end_radius,
            ramp,
            extend,
        } = shading.kind.as_ref()
        else {
            return Ok(None);
        };
        let (dx, dy, dr) = (end.x - start.x, end.y - start.y, end_radius - start_radius);
        if dr.mul_add(-dr, dx.mul_add(dx, dy * dy)) <= 0.0 {
            return Ok(None);
        }
        let Some(raster) = pdf_render::RadialRaster::build(
            pdf_render::Radial {
                start: *start,
                start_radius: *start_radius,
                end: *end,
                end_radius: *end_radius,
                ramp,
                extend: *extend,
            },
            shading.transform.then(self.target.transform),
            within,
        ) else {
            return Ok(None);
        };
        let id = self.device.upload_mesh(&quorra_scene::MeshSpec {
            left: raster.left,
            top: raster.top,
            image: spec(&raster.image),
        })?;
        self.transient.push(id.into());
        Ok(Some(id))
    }

    /// A mesh shading, through the shared rasteriser both existing backends use
    /// (`MeshRaster::build` — one implementation, identical bytes), uploaded for
    /// this frame.
    fn mesh(
        &mut self,
        triangles: &[pdf_render::Triangle],
        ramp: Option<&pdf_render::Ramp>,
        shading_transform: Transform,
    ) -> Result<Option<quorra_scene::MeshId>, QuorraRasterError> {
        let to_device = shading_transform.then(self.target.transform);
        // An empty raster is `None`, and the caller draws nothing — not a
        // refusal: pdf.js's issue #17848 traced such a mesh to a defective
        // document, and both sibling backends already skip it silently.
        let Some(raster) = pdf_render::MeshRaster::build(
            triangles,
            ramp,
            to_device,
            self.target.width,
            self.target.height,
        ) else {
            return Ok(None);
        };
        let id = self.device.upload_mesh(&quorra_scene::MeshSpec {
            left: raster.left,
            top: raster.top,
            image: spec(&raster.image),
        })?;
        self.transient.push(id.into());
        Ok(Some(id))
    }

    /// The uploaded outline for a path, keyed by the `Arc`'s identity
    /// (`RENDER_LIBRARY.md` section 2.2: one glyph outline, thousands of fills, one
    /// upload). Pinning and eviction are [`ResourceCaches`]' business.
    pub(crate) fn outline(
        &mut self,
        path: &Arc<Path>,
    ) -> Result<quorra_scene::OutlineId, QuorraRasterError> {
        if let Some(id) = self.caches.outline(path) {
            return Ok(id);
        }
        let id = self.device.upload_outline(&segments(path))?;
        self.caches.store_outline(path, id);
        Ok(id)
    }

    /// The outline expanded from `path` under `key`, uploading it only if this device does not
    /// already hold one — and calling `expand` only then, which is the larger half of what a hit
    /// saves (see [`crate::cache::StrokeKey`]).
    ///
    /// `expand` is a closure rather than a computed argument for exactly that reason: a caller
    /// that produced the geometry before asking would run `kurbo::stroke` over every stroke on
    /// every frame and save only the upload.
    pub(crate) fn expanded_stroke(
        &mut self,
        path: &Arc<Path>,
        key: crate::cache::StrokeKey,
        expand: impl FnOnce() -> Path,
    ) -> Result<quorra_scene::OutlineId, QuorraRasterError> {
        if let Some(id) = self.caches.stroke(key) {
            return Ok(id);
        }
        let id = self.device.upload_outline(&segments(&expand()))?;
        self.caches.store_stroke(path, key, id);
        Ok(id)
    }

    /// A per-frame outline for geometry this frame computed (dashed or degenerate
    /// strokes), released after the frame.
    pub(crate) fn transient_outline(
        &mut self,
        path: &Path,
    ) -> Result<quorra_scene::OutlineId, QuorraRasterError> {
        let id = self.device.upload_outline(&segments(path))?;
        self.transient.push(id.into());
        Ok(id)
    }

    fn cached_image(&mut self, image: &Image) -> Result<quorra_scene::ImageId, QuorraRasterError> {
        if let Some(id) = self.caches.image(&image.data, WHOLE) {
            return Ok(id);
        }
        let id = self.device.upload_image(&spec(image))?;
        self.caches.store_image(&image.data, WHOLE, id);
        Ok(id)
    }

    /// The reduced grid `reduction` describes, uploaded once and kept under the *source's*
    /// identity together with the factors that produced it.
    ///
    /// **This raster used to be transient, and that was 57% of a scrolled page's frame.**
    /// [`pdf_render::Image::area_averaged`] costs one pass over the *source* samples, so on a
    /// scanned page it is the largest thing in the frame and it does not shrink with the
    /// window: 8.5 to 9.8 ms of a 12.7 to 16.8 ms redraw of one 2700×3450 page, recomputed
    /// identically on every scroll step because the page, the scale and the samples were all
    /// unchanged (ADR 0297, `doc/todo/45`'s witness). Keying it needs no new memory argument:
    /// the entry's own bytes are the device's and are already inside `evict_settled`'s budget,
    /// and its pin is released the frame after the display list holding those samples goes.
    ///
    /// What it costs in readability is this function and one wider key — against
    /// `Image::reduction`, which is the whole of the exactness argument: every byte of the
    /// raster is a function of the source samples and the two factors, both of which are in
    /// the key.
    fn reduced_image(
        &mut self,
        image: &Image,
        placement: Transform,
        reduction: pdf_render::Reduction,
    ) -> Result<quorra_scene::ImageId, QuorraRasterError> {
        if let Some(id) = self.caches.image(&image.data, reduction.factors) {
            return Ok(id);
        }
        // `Image::reduction` answered `Some`, so `area_averaged` does too — they ask one
        // function. Written as "the reduced grid, or the samples themselves" rather than as an
        // unreachable branch, because that is the sentence the fallback would mean anyway and
        // it draws the same picture at a finer grid.
        let reduced = image.area_averaged(placement);
        let uploaded: &Image = reduced.as_ref().unwrap_or(image);
        let id = self.device.upload_image(&spec(uploaded))?;
        self.caches.store_image(&image.data, reduction.factors, id);
        Ok(id)
    }

    fn ramp(&mut self, shading: &Shading) -> Result<quorra_scene::RampId, QuorraRasterError> {
        let (ShadingKind::Axial { ramp, .. } | ShadingKind::Radial { ramp, .. }) =
            shading.kind.as_ref()
        else {
            return Err(QuorraRasterError::Unsupported(format!(
                "a ramp for shading {:?}",
                shading.kind
            )));
        };
        if let Some(id) = self.caches.ramp(&shading.kind) {
            return Ok(id);
        }
        let stops: Vec<quorra_scene::Stop> = ramp
            .stops
            .iter()
            .map(|stop| quorra_scene::Stop {
                offset: stop.at,
                color: colour(stop.colour),
            })
            .collect();
        let id = self.device.upload_ramp(&stops)?;
        self.caches.store_ramp(&shading.kind, id);
        Ok(id)
    }

    /// The quorra clip chain for a display-list clip, memoised per frame — `None`
    /// when some link admits nothing, in which case the command under it draws
    /// nothing at all (an empty clip is a statement, not an accident).
    pub(crate) fn clip_chain(
        &mut self,
        builder: &mut SceneBuilder,
        clip: Option<ClipId>,
    ) -> Result<Admitted, QuorraRasterError> {
        let Some(id) = clip else {
            return Ok(Admitted::Chain(None));
        };
        match self.resolve_clip(builder, id, 0)? {
            ResolvedClip::AdmitsNothing => Ok(Admitted::Nothing),
            ResolvedClip::Chain(chain) => Ok(Admitted::Chain(Some(chain))),
        }
    }

    fn resolve_clip(
        &mut self,
        builder: &mut SceneBuilder,
        id: ClipId,
        depth: usize,
    ) -> Result<ResolvedClip, QuorraRasterError> {
        if let Some(resolved) = self.clips.get(&id.index()) {
            return Ok(*resolved);
        }
        if depth > MAX_CLIP_DEPTH {
            return Err(QuorraRasterError::CyclicClip(id));
        }
        let def = self
            .list
            .clip(id)
            .ok_or(QuorraRasterError::UnknownClip(id))?;
        if def.admits_nothing() {
            self.clips.insert(id.index(), ResolvedClip::AdmitsNothing);
            return Ok(ResolvedClip::AdmitsNothing);
        }
        let parent = match def.parent {
            Some(parent) => match self.resolve_clip(builder, parent, depth.saturating_add(1))? {
                ResolvedClip::AdmitsNothing => {
                    self.clips.insert(id.index(), ResolvedClip::AdmitsNothing);
                    return Ok(ResolvedClip::AdmitsNothing);
                }
                ResolvedClip::Chain(chain) => Some(chain),
            },
            None => None,
        };
        let outline = self.transient_outline(&def.path)?;
        let link = builder.clip(
            outline,
            self.placed(def.transform),
            fill_rule(def.fill_rule),
            parent,
        )?;
        let resolved = ResolvedClip::Chain(link);
        self.clips.insert(id.index(), resolved);
        Ok(resolved)
    }

    /// The quorra soft mask for a display-list one, realised on first use through
    /// the same command walk as the page (a mask that took a different path would
    /// be code the cross-backend tests never exercise).
    pub(crate) fn mask_id(
        &mut self,
        builder: &mut SceneBuilder,
        mask: Option<SoftMaskId>,
    ) -> Result<Option<quorra_scene::MaskId>, QuorraRasterError> {
        let Some(id) = mask else { return Ok(None) };
        if let Some(mapped) = self.masks.get(&id.index()) {
            return Ok(Some(*mapped));
        }
        let def = self
            .list
            .soft_mask(id)
            .ok_or(QuorraRasterError::UnknownSoftMask(id))?;
        let kind = match def.kind {
            SoftMaskKind::Alpha => quorra_scene::MaskKind::Alpha,
            SoftMaskKind::Luminosity { backdrop } => quorra_scene::MaskKind::Luminosity {
                backdrop: colour(backdrop),
            },
        };
        let transfer = def.transfer.as_ref().map(|t| {
            // The display list's table is private behind `apply`; quorra's is the
            // table itself. Reconstructing through `apply` keeps the one source.
            quorra_scene::Transfer(std::array::from_fn(|i| {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "the index enumerates exactly 0..=255"
                )]
                t.apply(i as u8)
            }))
        });
        let commands = def.commands.clone();
        let mut walked = Ok(());
        let mapped = builder.mask(kind, transfer, |body| {
            walked = self.commands(body, &commands);
            Ok(())
        })?;
        walked?;
        self.masks.insert(id.index(), mapped);
        Ok(Some(mapped))
    }

    pub(crate) fn target(&self) -> TargetSpec {
        self.target
    }

    /// A page-space transform placed onto the scene: the target's transform is
    /// baked into every command here, and the quorra viewport stays identity —
    /// which is what lets one scene carry the page, a fallback raster and
    /// window-pixel overlays at their own placements ([`crate::QuorraWindowRenderer`]).
    pub(crate) fn placed(&self, t: Transform) -> quorra_scene::Affine {
        affine(t.then(self.target.transform))
    }
}

/// The display list's transform, as quorra's — same six coefficients, same
/// row-vector convention, same `then` order.
pub(crate) fn affine(t: Transform) -> quorra_scene::Affine {
    quorra_scene::Affine {
        a: t.a,
        b: t.b,
        c: t.c,
        d: t.d,
        e: t.e,
        f: t.f,
    }
}

pub(crate) fn point(p: Point) -> quorra_scene::Point {
    quorra_scene::Point::new(p.x, p.y)
}

/// §7.10.1's `Range` as quorra states it — the bounds, and the component count with them.
///
/// Total both ways: the two vocabularies name the same two shapes, because
/// `pdf_model::shading` builds a [`ProgramRange`] only for the colour spaces whose components
/// are the device's own.
fn function_range(range: ProgramRange) -> quorra_scene::FnRange {
    match range {
        ProgramRange::Gray(pair) => quorra_scene::FnRange::Gray(pair),
        ProgramRange::Rgb(pairs) => quorra_scene::FnRange::Rgb(pairs),
    }
}

/// One step of a compiled §7.10.5 program as quorra's own instruction.
///
/// **Total, with no wildcard arm**, which is the whole reason it is written out: an operator
/// added to either vocabulary stops this compiling rather than reaching the device as
/// something else. The two enums were shaped by the same Table 42 and the map is the identity
/// on every name; what it costs is thirty-eight lines that can be read against the table.
fn function_op(step: ProgramStep) -> quorra_scene::FnOp {
    use quorra_scene::FnOp;
    match step {
        ProgramStep::PushInt(value) => FnOp::PushInt(value),
        ProgramStep::PushReal(value) => FnOp::PushReal(value),
        ProgramStep::PushBool(value) => FnOp::PushBool(value),
        ProgramStep::JumpUnless { target } => FnOp::JumpUnless { target },
        ProgramStep::Jump { target } => FnOp::Jump { target },
        ProgramStep::Operator(operator) => match operator {
            ProgramOperator::Abs => FnOp::Abs,
            ProgramOperator::Add => FnOp::Add,
            ProgramOperator::Atan => FnOp::Atan,
            ProgramOperator::Ceiling => FnOp::Ceiling,
            ProgramOperator::Cos => FnOp::Cos,
            ProgramOperator::Cvi => FnOp::Cvi,
            ProgramOperator::Cvr => FnOp::Cvr,
            ProgramOperator::Div => FnOp::Div,
            ProgramOperator::Exp => FnOp::Exp,
            ProgramOperator::Floor => FnOp::Floor,
            ProgramOperator::Idiv => FnOp::Idiv,
            ProgramOperator::Ln => FnOp::Ln,
            ProgramOperator::Log => FnOp::Log,
            ProgramOperator::Mod => FnOp::Mod,
            ProgramOperator::Mul => FnOp::Mul,
            ProgramOperator::Neg => FnOp::Neg,
            ProgramOperator::Round => FnOp::Round,
            ProgramOperator::Sin => FnOp::Sin,
            ProgramOperator::Sqrt => FnOp::Sqrt,
            ProgramOperator::Sub => FnOp::Sub,
            ProgramOperator::Truncate => FnOp::Truncate,
            ProgramOperator::And => FnOp::And,
            ProgramOperator::Bitshift => FnOp::Bitshift,
            ProgramOperator::Eq => FnOp::Eq,
            ProgramOperator::Ge => FnOp::Ge,
            ProgramOperator::Gt => FnOp::Gt,
            ProgramOperator::Le => FnOp::Le,
            ProgramOperator::Lt => FnOp::Lt,
            ProgramOperator::Ne => FnOp::Ne,
            ProgramOperator::Not => FnOp::Not,
            ProgramOperator::Or => FnOp::Or,
            ProgramOperator::Xor => FnOp::Xor,
            ProgramOperator::Copy => FnOp::Copy,
            ProgramOperator::Dup => FnOp::Dup,
            ProgramOperator::Exch => FnOp::Exch,
            ProgramOperator::Index => FnOp::Index,
            ProgramOperator::Pop => FnOp::Pop,
            ProgramOperator::Roll => FnOp::Roll,
        },
    }
}

pub(crate) fn colour(c: Color) -> quorra_scene::Color {
    quorra_scene::Color::new(c.r, c.g, c.b, c.a)
}

/// A colour quantised to straight-alpha RGBA8 bytes, rounding as every backend's
/// boundary does.
pub(crate) fn byte_colour(c: Color) -> [u8; 4] {
    let byte = |v: f32| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped into 0..=255 before the cast"
        )]
        let quantised = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        quantised
    };
    [byte(c.r), byte(c.g), byte(c.b), byte(c.a)]
}

pub(crate) fn fill_rule(rule: FillRule) -> quorra_scene::FillRule {
    match rule {
        FillRule::NonZero => quorra_scene::FillRule::NonZero,
        FillRule::EvenOdd => quorra_scene::FillRule::EvenOdd,
    }
}

pub(crate) fn blend_mode(blend: BlendMode) -> quorra_scene::BlendMode {
    match blend {
        BlendMode::Normal => quorra_scene::BlendMode::Normal,
        BlendMode::Multiply => quorra_scene::BlendMode::Multiply,
        BlendMode::Screen => quorra_scene::BlendMode::Screen,
        BlendMode::Overlay => quorra_scene::BlendMode::Overlay,
        BlendMode::Darken => quorra_scene::BlendMode::Darken,
        BlendMode::Lighten => quorra_scene::BlendMode::Lighten,
        BlendMode::ColorDodge => quorra_scene::BlendMode::ColorDodge,
        BlendMode::ColorBurn => quorra_scene::BlendMode::ColorBurn,
        BlendMode::HardLight => quorra_scene::BlendMode::HardLight,
        BlendMode::SoftLight => quorra_scene::BlendMode::SoftLight,
        BlendMode::Difference => quorra_scene::BlendMode::Difference,
        BlendMode::Exclusion => quorra_scene::BlendMode::Exclusion,
        BlendMode::Hue => quorra_scene::BlendMode::Hue,
        BlendMode::Saturation => quorra_scene::BlendMode::Saturation,
        BlendMode::Color => quorra_scene::BlendMode::Color,
        BlendMode::Luminosity => quorra_scene::BlendMode::Luminosity,
    }
}

/// A display-list path as quorra segments: both are cubics-only, so the mapping
/// is positional.
pub(crate) fn segments(path: &Path) -> Vec<quorra_scene::Segment> {
    path.commands()
        .iter()
        .map(|command| match *command {
            PathCommand::MoveTo(p) => quorra_scene::Segment::MoveTo(point(p)),
            PathCommand::LineTo(p) => quorra_scene::Segment::LineTo(point(p)),
            PathCommand::CurveTo(c1, c2, to) => quorra_scene::Segment::CubicTo {
                c1: point(c1),
                c2: point(c2),
                to: point(to),
            },
            PathCommand::Close => quorra_scene::Segment::Close,
        })
        .collect()
}

/// A display-list image as quorra's upload spec — the same layout (straight-alpha
/// RGBA8, top row first, no padding), so the `Arc` is shared, not copied.
fn spec(image: &Image) -> quorra_scene::ImageSpec {
    quorra_scene::ImageSpec {
        width: image.width,
        height: image.height,
        data: Arc::clone(&image.data),
    }
}
