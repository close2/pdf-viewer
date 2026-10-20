//! A non-solid paint on the scene: an axial or radial ramp, a type 1 function program, a
//! sampled grid, and a mesh's device-resolution raster (ISO 32000-2 §8.7.4.5).
//!
//! What each construction resolves to is [`super::ShadedPaint`], which the fill and the stroke
//! both consume.

use std::sync::Arc;

use pdf_render::{
    BlendMode, FillRule, ProgramOperator, ProgramRange, ProgramStep, Shading, ShadingKind,
    Transform,
};
use raster_scene::SceneBuilder;

use crate::QuorraRasterError;

use super::{
    Encoder, ShadedPaint, blend_mode, byte_colour, colour, fill_rule, overprinted, point, spec,
};

/// §7.10.1's `Range` as raster states it — the bounds, and the component count with them.
///
/// Total both ways: the two vocabularies name the same two shapes, because
/// `pdf_model::shading` builds a [`ProgramRange`] only for the colour spaces whose components
/// are the device's own.
fn function_range(range: ProgramRange) -> raster_scene::FnRange {
    match range {
        ProgramRange::Gray(pair) => raster_scene::FnRange::Gray(pair),
        ProgramRange::Rgb(pairs) => raster_scene::FnRange::Rgb(pairs),
    }
}

/// One step of a compiled §7.10.5 program as raster's own instruction.
///
/// **Total, with no wildcard arm**, which is the whole reason it is written out: an operator
/// added to either vocabulary stops this compiling rather than reaching the device as
/// something else. The two enums were shaped by the same Table 42 and the map is the identity
/// on every name; what it costs is thirty-eight lines that can be read against the table.
fn function_op(step: ProgramStep) -> raster_scene::FnOp {
    use raster_scene::FnOp;
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

impl Encoder<'_> {
    /// What a shading resolves to: a raster paint, the sampled kind (drawn as an
    /// image by the caller), or nothing at all.
    pub(crate) fn shading_paint(
        &mut self,
        shading: &Shading,
    ) -> Result<ShadedPaint, QuorraRasterError> {
        // A fill carrying Table 77's `/Background` took [`Encoder::background_raster`]'s door
        // before reaching here, so what is left is a *stroke* — whose outline is not the shape
        // this encoder is handed. Refused rather than drawn without its wash, which keeps the
        // shortfall the report `pdf_model` already raises instead of a quiet difference.
        if shading.background.is_some() {
            return Err(QuorraRasterError::Unsupported(
                "a stroking shading pattern's /Background (§8.7.4.3)".to_owned(),
            ));
        }
        let kind = match shading.kind.as_ref() {
            ShadingKind::Axial {
                start, end, extend, ..
            } => raster_scene::ShadingKind::Axial {
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
            } => raster_scene::ShadingKind::Radial {
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
            ShadingKind::Mesh {
                triangles,
                patches,
                ramp,
            } => {
                return Ok(
                    match self.mesh(
                        triangles,
                        patches.as_ref(),
                        ramp.as_ref(),
                        shading.transform,
                    )? {
                        Some(mesh) => ShadedPaint::Ready(raster_scene::Paint::Mesh(mesh)),
                        None => ShadedPaint::Nothing,
                    },
                );
            }
            other => {
                return Err(QuorraRasterError::Unsupported(format!("shading {other:?}")));
            }
        };
        let ramp = self.ramp(shading)?;
        Ok(ShadedPaint::Ready(raster_scene::Paint::Shading {
            ramp,
            kind,
            // raster anchors a shading through its own transform, exactly as the
            // display list states it (§8.7.4.3's shading matrix).
            transform: self.placed(shading.transform),
        }))
    }

    /// §8.7.4.5.2's type 1 shading as a colour the *device* evaluates, where this device
    /// will take it — raster's `Paint::Function`, ADR 0376.
    ///
    /// `Ok(None)` is not a failure and never becomes one: it means the caller draws the grid,
    /// which is what [`Encoder::sampled_fill`] does and what every backend did before this
    /// existed. Every `None` that is the *device's* answer rather than the display list's
    /// carries its ground into [`super::FunctionPaints`] on the way out, so a page that stops taking
    /// this path says why.
    ///
    /// # The order the three questions are asked in, and why none of them is asked late
    ///
    /// 1. **Is there a program at all?** `pdf_model::shading` builds one only where the two
    ///    paths compute the same colour; a shading of another function type, or over a colour
    ///    space needing a conversion, arrives with `None` and is not the device's business.
    /// 2. **Will the device execute it?** [`raster_gpu::function::admit`] — the structural
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
    /// at [`raster_gpu::function::MAX_PROGRAM_LENGTH`] steps — microseconds — against the
    /// milliseconds of grid the fallback would otherwise build, and it keeps the refusal a
    /// decision this frame took rather than one a cache entry remembers.
    fn function_paint(
        &mut self,
        shading: &Shading,
    ) -> Result<Option<raster_scene::Paint>, QuorraRasterError> {
        let (Some(program), ShadingKind::Sampled { domain, .. }) =
            (shading.device_program(), shading.kind.as_ref())
        else {
            return Ok(None);
        };
        let range = function_range(program.range());
        let steps: Vec<raster_scene::FnOp> =
            program.steps().iter().copied().map(function_op).collect();

        let analysis = match raster_gpu::function::admit(&steps) {
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
        let region = raster_scene::Rect::new(
            raster_scene::Point::new(x0, y0),
            raster_scene::Point::new(x1, y1),
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

        let paint = raster_scene::Paint::Function {
            program: self.function(program.steps(), &steps)?,
            domain: region,
            matrix,
            range,
            // §8.7.4.3 Table 77's `/Background`, which reaches this device as `None` by
            // construction rather than by omission: a shading carrying one took
            // [`Encoder::background_raster`]'s door in `emit_fill` and never arrives here, so
            // what this paint draws is always the entry's *absence* — points outside the domain
            // rectangle "left unpainted", §8.7.4.5.2's own other branch, which is what
            // `sampled_fill` draws too. Sending the colour down here instead would put the wash
            // on the device for one shading kind and in `pdf_render` for the other three,
            // which is the split trap 2 is about.
            background: None,
        };
        // The budgets the scene boundary states over a rectangle and a transform are raster's
        // own numbers, so they are asked with raster's own predicate rather than restated
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
    /// contents and drops it when the last id naming them is released (raster's ADR 0053), so
    /// a program uploaded and released around each frame would recompile a shader on every
    /// frame of a still page — the cold compile `doc/QUORRA_FUNCTION_PAINT_ANSWER.md` measures
    /// at 6.3 ms, arriving once a frame instead of once. The key is the step list's `Arc`, so
    /// a display list rebuilt from scratch re-uploads it exactly as it re-uploads an outline.
    fn function(
        &mut self,
        key: &Arc<[ProgramStep]>,
        steps: &[raster_scene::FnOp],
    ) -> Result<raster_scene::FunctionId, QuorraRasterError> {
        if let Some(id) = self.caches.program(key) {
            return Ok(id);
        }
        let id = self.handing_over(|device| device.upload_function(steps))?;
        self.caches.store_program(key, id);
        Ok(id)
    }

    /// A sampled shading — the display list's grid stand-in for a function-based
    /// shading — drawn as a linearly-filtered image over the domain rectangle,
    /// clipped to the filled path (integration note 9 in raster's plan).
    ///
    /// Stated divergence: the CPU backend pads the grid's edge colours beyond the
    /// domain rectangle; this draws the domain exactly and nothing beyond it. The
    /// `render-gpu` backend refuses sampled shadings outright, so the swap still
    /// strictly widens what draws.
    pub(super) fn sampled_fill(
        &mut self,
        builder: &mut SceneBuilder,
        (outline, transform, rule): (raster_scene::OutlineId, Transform, FillRule),
        shading: &Shading,
        (clip, mask, blend): (
            Option<raster_scene::ClipId>,
            Option<raster_scene::MaskId>,
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
        self.consume_view(); // the grid is this placement's (ADR 0702)
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

        // The grid's row 0 sits at the domain's y0; raster's image convention puts
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
        let grid_spec = raster_scene::ImageSpec {
            width: grid.width,
            height: grid.height,
            data: Arc::from(data.as_slice()),
        };
        let image = self.handing_over(|device| device.upload_image(&grid_spec))?;
        self.transient.push(image.into());

        // Clip to the filled path: raster clips compose by chaining, so the fill's
        // own geometry becomes one more link under the command's clip.
        let shape_clip = builder.clip(outline, self.placed(transform), fill_rule(rule), clip)?;
        // Unit square → the part of the domain these cells cover → shading space → page.
        let placement = grid.onto_shading().then(shading.transform);
        let at = self.placed(placement);
        overprinted(builder, blend, self.knockout_element(), |builder| {
            builder.image(
                image,
                at,
                1.0,
                raster_scene::ImageFilter::Linear,
                Some(shape_clip),
                blend_mode(blend),
                mask,
            )
        })?;
        Ok(())
    }

    /// A radial shading exactly evaluated, where §8.7.4.5.4 and a gradient part company.
    ///
    /// `None` for every shading that is not a *cone* — one whose circles are further apart
    /// than their radii differ, which is the exact condition under which a point can lie on
    /// two blend circles and the clause's tie-break can change a pixel. The proof is in
    /// `render_cpu::shading::is_a_cone`; the arithmetic is
    /// [`pdf_render::blend_parameter`]'s.
    ///
    /// The raster is uploaded as a **mesh**, because raster's mesh paint is precisely "an
    /// RGBA raster already at device resolution, placed at (left, top)" — which is what this
    /// is, and what the other two backends draw for the same geometry.
    pub(super) fn radial_cone(
        &mut self,
        shading: &Shading,
        within: (u32, u32, u32, u32),
    ) -> Result<Option<raster_scene::MeshId>, QuorraRasterError> {
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
        self.consume_view(); // the raster is this view's pixels (ADR 0702)
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
        let mesh = raster_scene::MeshSpec {
            left: raster.left,
            top: raster.top,
            image: spec(&raster.image),
        };
        let id = self.handing_over(|device| device.upload_mesh(&mesh))?;
        self.transient.push(id.into());
        Ok(Some(id))
    }

    /// A shading washed with ISO 32000-2 §8.7.4.3 Table 77's `/Background`, through the shared
    /// rasteriser all three backends use (`ShadingRaster::build` — one implementation,
    /// identical bytes), uploaded for this frame as a mesh.
    ///
    /// `Ok(None)` is a shape covering no device pixel, and the caller draws nothing — the same
    /// answer [`Encoder::mesh`] gives an empty mesh, and not a refusal.
    pub(super) fn background_raster(
        &mut self,
        shading: &Shading,
        within: (u32, u32, u32, u32),
    ) -> Result<Option<raster_scene::MeshId>, QuorraRasterError> {
        self.consume_view(); // the raster is this view's pixels (ADR 0702)
        let Some(raster) = pdf_render::ShadingRaster::build(
            shading,
            self.target.transform,
            within,
            (self.target.width, self.target.height),
        ) else {
            return Ok(None);
        };
        let mesh = raster_scene::MeshSpec {
            left: raster.left,
            top: raster.top,
            image: spec(&raster.image),
        };
        let id = self.handing_over(|device| device.upload_mesh(&mesh))?;
        self.transient.push(id.into());
        Ok(Some(id))
    }

    /// A mesh shading, through the shared rasteriser both existing backends use
    /// (`MeshRaster::build` — one implementation, identical bytes), uploaded for
    /// this frame.
    fn mesh(
        &mut self,
        triangles: &[pdf_render::Triangle],
        patches: Option<&pdf_render::PatchMesh>,
        ramp: Option<&pdf_render::Ramp>,
        shading_transform: Transform,
    ) -> Result<Option<raster_scene::MeshId>, QuorraRasterError> {
        let to_device = shading_transform.then(self.target.transform);
        self.consume_view(); // the mesh raster is this view's pixels (ADR 0702)
        // An empty raster is `None`, and the caller draws nothing — not a
        // refusal: pdf.js's issue #17848 traced such a mesh to a defective
        // document, and both sibling backends already skip it silently.
        let Some(raster) = pdf_render::MeshRaster::build(
            triangles,
            patches,
            ramp,
            to_device,
            self.target.width,
            self.target.height,
        ) else {
            return Ok(None);
        };
        let mesh = raster_scene::MeshSpec {
            left: raster.left,
            top: raster.top,
            image: spec(&raster.image),
        };
        let id = self.handing_over(|device| device.upload_mesh(&mesh))?;
        self.transient.push(id.into());
        Ok(Some(id))
    }

    fn ramp(&mut self, shading: &Shading) -> Result<raster_scene::RampId, QuorraRasterError> {
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
        let stops: Vec<raster_scene::Stop> = ramp
            .stops
            .iter()
            .map(|stop| raster_scene::Stop {
                offset: stop.at,
                color: colour(stop.colour),
            })
            .collect();
        let id = self.handing_over(|device| device.upload_ramp(&stops))?;
        self.caches.store_ramp(&shading.kind, id);
        Ok(id)
    }
}
