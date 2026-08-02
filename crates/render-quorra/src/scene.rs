//! Display list → quorra scene: the translation the two vocabularies were shaped
//! to make small.
//!
//! One walk over the commands, mirroring the recursive structure (groups and soft
//! masks carry command lists of the same display list). Resources upload through
//! the [`Encoder`]'s caches — outlines and images by their `Arc` identity, exactly
//! the keying `doc/RENDER_LIBRARY.md` §2.2 asks for — or transiently for the
//! per-frame forms (clips, dashed strokes, meshes, sampled grids, area-averaged
//! images), which the caller releases after the frame.

use std::collections::HashMap;
use std::sync::Arc;

use pdf_render::{
    BlendMode, ClipId, Color, Command, DisplayList, FillRule, Image, Paint, Path, PathCommand,
    Point, Shading, ShadingKind, SoftMaskId, SoftMaskKind, TargetSpec, Transform,
};
use quorra_scene::{ResourceId, SceneBuilder};

use crate::QuorraRasterError;

/// The walk's state: the device and caches it uploads through, and the per-list
/// clip and mask tables it resolves against.
pub(crate) struct Encoder<'a> {
    device: &'a mut quorra_gpu::Device,
    list: &'a DisplayList,
    target: TargetSpec,
    outlines: &'a mut HashMap<usize, (Arc<Path>, quorra_scene::OutlineId)>,
    images: &'a mut HashMap<usize, (Arc<[u8]>, quorra_scene::ImageId)>,
    ramps: &'a mut HashMap<usize, (Arc<ShadingKind>, quorra_scene::RampId)>,
    transient: &'a mut Vec<ResourceId>,
    clips: HashMap<usize, ResolvedClip>,
    masks: HashMap<usize, quorra_scene::MaskId>,
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

impl<'a> Encoder<'a> {
    pub(crate) fn new(
        device: &'a mut quorra_gpu::Device,
        list: &'a DisplayList,
        target: TargetSpec,
        outlines: &'a mut HashMap<usize, (Arc<Path>, quorra_scene::OutlineId)>,
        images: &'a mut HashMap<usize, (Arc<[u8]>, quorra_scene::ImageId)>,
        ramps: &'a mut HashMap<usize, (Arc<ShadingKind>, quorra_scene::RampId)>,
        transient: &'a mut Vec<ResourceId>,
    ) -> Self {
        Self {
            device,
            list,
            target,
            outlines,
            images,
            ramps,
            transient,
            clips: HashMap::new(),
            masks: HashMap::new(),
        }
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
                    knockout,
                } => {
                    let Admitted::Chain(clip) = self.clip_chain(builder, *clip)? else {
                        continue; // the clip admits nothing: the group draws nothing
                    };
                    let mask = self.mask_id(builder, *mask)?;
                    let spec = quorra_scene::GroupSpec {
                        alpha: *alpha,
                        blend: blend_mode(*blend),
                        clip,
                        knockout: *knockout,
                        mask,
                    };
                    let mut walked = Ok(());
                    builder.group(spec, |body| {
                        walked = self.commands(body, commands);
                        // The builder's own error channel carries scene refusals;
                        // upload and translation errors travel beside it.
                        Ok(())
                    })?;
                    walked?;
                }
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
        let outline = self.outline(path)?;
        let paint = match paint {
            Paint::Solid(c) => quorra_scene::Paint::Solid(colour(*c)),
            Paint::Shading(shading) => match self.shading_paint(shading)? {
                Some(paint) => paint,
                // A sampled shading is not a quorra paint: it draws as an image
                // clipped to this fill's path instead.
                None => {
                    return self.sampled_fill(
                        builder,
                        (outline, transform, rule),
                        shading,
                        (clip, mask, blend),
                    );
                }
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

    /// The quorra paint for a shading, or `None` for the sampled kind (drawn as an
    /// image by the caller).
    pub(crate) fn shading_paint(
        &mut self,
        shading: &Shading,
    ) -> Result<Option<quorra_scene::Paint>, QuorraRasterError> {
        let kind = match shading.kind.as_ref() {
            ShadingKind::Axial { start, end, extend, .. } => quorra_scene::ShadingKind::Axial {
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
            ShadingKind::Sampled { .. } => return Ok(None),
            ShadingKind::Mesh { triangles } => {
                return Ok(Some(quorra_scene::Paint::Mesh(
                    self.mesh(triangles, shading.transform)?,
                )));
            }
            other => {
                return Err(QuorraRasterError::Unsupported(format!("shading {other:?}")));
            }
        };
        let ramp = self.ramp(shading)?;
        Ok(Some(quorra_scene::Paint::Shading {
            ramp,
            kind,
            // quorra anchors a shading through its own transform, exactly as the
            // display list states it (§8.7.4.3's shading matrix).
            transform: self.placed(shading.transform),
        }))
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
        (clip, mask, blend): (Option<quorra_scene::ClipId>, Option<quorra_scene::MaskId>, BlendMode),
    ) -> Result<(), QuorraRasterError> {
        let ShadingKind::Sampled {
            domain,
            width,
            height,
            pixels,
        } = shading.kind.as_ref()
        else {
            // The caller matched Sampled before dispatching here.
            return Err(QuorraRasterError::Unsupported(format!(
                "shading {:?}",
                shading.kind
            )));
        };
        let [x0, x1, y0, y1] = *domain;
        let (span_x, span_y) = (x1 - x0, y1 - y0);
        if span_x == 0.0 || span_y == 0.0 || *width == 0 || *height == 0 {
            return Err(QuorraRasterError::Unsupported(
                "a sampled shading with a degenerate domain".into(),
            ));
        }

        // The grid's row 0 sits at the domain's y0; quorra's image convention puts
        // the data's first row at unit y = 1 (ISO 32000-2 §8.9.5), so the rows
        // reverse on the way in.
        let row_len = (*width as usize).saturating_mul(4);
        let mut data = Vec::with_capacity(row_len.saturating_mul(*height as usize));
        for row in (0..*height as usize).rev() {
            let start = row.saturating_mul(*width as usize);
            for c in pixels
                .get(start..start.saturating_add(*width as usize))
                .unwrap_or(&[])
            {
                data.extend_from_slice(&byte_colour(*c));
            }
        }
        let image = self.device.upload_image(&quorra_scene::ImageSpec {
            width: *width,
            height: *height,
            data: Arc::from(data.as_slice()),
        })?;
        self.transient.push(image.into());

        // Clip to the filled path: quorra clips compose by chaining, so the fill's
        // own geometry becomes one more link under the command's clip.
        let shape_clip = builder.clip(outline, self.placed(transform), fill_rule(rule), clip)?;
        // Unit square → domain rectangle → shading space → page.
        let to_domain = Transform::new(span_x, 0.0, 0.0, span_y, x0, y0);
        let placement = to_domain.then(shading.transform);
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
        image: &Image,
        transform: Transform,
        alpha: f32,
        (clip, mask, blend): (Option<ClipId>, Option<SoftMaskId>, BlendMode),
    ) -> Result<(), QuorraRasterError> {
        let Admitted::Chain(clip) = self.clip_chain(builder, clip)? else {
            return Ok(());
        };
        let mask = self.mask_id(builder, mask)?;
        let placement = transform.then(self.target.transform);

        // The two decisions §4.5 settles on this side of the boundary: area
        // averaging for minification, and the resolved smoothing for the placement.
        let (id, smoothed) = match image.area_averaged(placement) {
            Some(reduced) => {
                let smoothed = reduced.is_smoothed(placement);
                let id = self.device.upload_image(&spec(&reduced))?;
                self.transient.push(id.into());
                (id, smoothed)
            }
            None => (self.cached_image(image)?, image.is_smoothed(placement)),
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

    /// A mesh shading, through the shared rasteriser both existing backends use
    /// (`MeshRaster::build` — one implementation, identical bytes), uploaded for
    /// this frame.
    fn mesh(
        &mut self,
        triangles: &[pdf_render::Triangle],
        shading_transform: Transform,
    ) -> Result<quorra_scene::MeshId, QuorraRasterError> {
        let to_device = shading_transform.then(self.target.transform);
        let raster = pdf_render::MeshRaster::build(
            triangles,
            to_device,
            self.target.width,
            self.target.height,
        )
        .ok_or_else(|| {
            QuorraRasterError::Unsupported("a mesh shading with no visible raster".into())
        })?;
        let id = self.device.upload_mesh(&quorra_scene::MeshSpec {
            left: raster.left,
            top: raster.top,
            image: spec(&raster.image),
        })?;
        self.transient.push(id.into());
        Ok(id)
    }

    /// The uploaded outline for a path, keyed by the `Arc`'s identity (§2.2: one
    /// glyph outline, thousands of fills, one upload). The entry pins the `Arc`
    /// so the address cannot be recycled while the key lives — see the cache's
    /// own docs.
    pub(crate) fn outline(
        &mut self,
        path: &Arc<Path>,
    ) -> Result<quorra_scene::OutlineId, QuorraRasterError> {
        let key = Arc::as_ptr(path).cast::<u8>() as usize;
        if let Some((_, id)) = self.outlines.get(&key) {
            return Ok(*id);
        }
        let id = self.device.upload_outline(&segments(path))?;
        self.outlines.insert(key, (Arc::clone(path), id));
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
        let key = image.data.as_ptr() as usize;
        if let Some((_, id)) = self.images.get(&key) {
            return Ok(*id);
        }
        let id = self.device.upload_image(&spec(image))?;
        self.images.insert(key, (Arc::clone(&image.data), id));
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
        let key = Arc::as_ptr(&shading.kind).cast::<u8>() as usize;
        if let Some((_, id)) = self.ramps.get(&key) {
            return Ok(*id);
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
        self.ramps.insert(key, (Arc::clone(&shading.kind), id));
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
    /// window-pixel overlays at their own placements ([`crate::QuorraPresenter`]).
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

pub(crate) fn colour(c: Color) -> quorra_scene::Color {
    quorra_scene::Color::new(c.r, c.g, c.b, c.a)
}

/// A colour quantised to straight-alpha RGBA8 bytes, rounding as every backend's
/// boundary does.
fn byte_colour(c: Color) -> [u8; 4] {
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
