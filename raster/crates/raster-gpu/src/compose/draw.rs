//! The content pass: the marks themselves, drawn onto one attachment.
//!
//! A run of consecutive drawable ops becomes exactly one render pass, which is why the
//! preparation is a phase of its own: every bind group and every pipeline the run needs
//! has to exist *before* the pass borrows the recorder, and none of them can be built
//! while it does.
//!
//! Knockout batches run their erase/add pair strictly per element (ADR 0010):
//! interleaving is what makes overlapping knockout elements compose per ISO 32000-2
//! §11.4.6 rather than approximately. Everything else is instanced, because a single
//! pass over independent marks needs no interleaving at all.

use std::collections::HashMap;
use std::sync::Arc;

use crate::device::shading_params_bytes;
use crate::encode::{Batch, BatchKind, DrawStyle, FunctionOp, ImageOp, Op, PaintSource, ShadedOp};
use crate::error::RenderError;
use crate::pipeline::{Kind, Style};

use super::Executor;

/// What a content pass does with the pixels already in the attachment it draws onto.
///
/// The first pass onto a plan's accumulator clears it: §11.4.5 begins a group over a
/// fully transparent initial backdrop, and brief section 3 hands the caller pixels over one. Every
/// later pass onto the same accumulator keeps what the earlier ones put there, because
/// the painter's order is passes as much as it is instances — so this says which pass
/// this is, and never a preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PassLoad {
    /// The first pass onto this attachment: clear to transparent, then draw.
    Clear,
    /// A later pass: load what is there and draw over it.
    Keep,
}

/// One drawable item of a pass: an instanced lane batch, or a single-quad op
/// (image, shading, mesh, function — ADR 0011's rare cases and ADR 0053's).
pub(crate) enum RunOp {
    Batch(Batch),
    Image(ImageOp),
    Shaded(ShadedOp),
    Function(FunctionOp),
}

/// A prepared item of a pass: everything a draw needs that cannot be made while
/// the pass borrows the recorder.
enum Ready {
    Batch(Batch),
    /// Over is one pipeline; knockout is the erase/add pair, in order.
    ///
    /// The pipelines are resolved here rather than named by [`Kind`] and looked up in the
    /// pass, because ADR 0053's generated ones have no `Kind` to name — their key is a
    /// program's content hash. Resolving both families the same way is what keeps the pass
    /// itself ignorant of which is which.
    Single {
        pipelines: [Option<Arc<wgpu::RenderPipeline>>; 2],
        bind: wgpu::BindGroup,
    },
    /// A shading quad: its pipelines, which of the run's shading bind groups it reads
    /// ([`Shadings::binds`]), and where its numbers sit in that group's buffer (ADR 1555).
    Shaded {
        pipelines: [Option<Arc<wgpu::RenderPipeline>>; 2],
        bind: usize,
        offset: u32,
    },
}

/// The shading quads of one run, gathered before any of their bindings exist: every quad's
/// numbers laid into as few buffers as the device allows, and one bind group per paint, mask
/// and buffer, which every quad reading them shares at its own offset (ADR 1555).
///
/// **Why one buffer and few groups.** A buffer and a bind group made per quad were the
/// largest host cost of a pass on a page of many shadings: `bug1721218_reduced.pdf` draws
/// 3 583 ops a render through five ramps and no mask, and making them took 9 to 10 ms of each
/// of its two renders. The numbers each quad reads are the bytes it read from a buffer of its
/// own; only where they sit has changed, so every draw shades what it shaded.
#[derive(Default)]
struct Shadings {
    /// The closed buffers' bytes, every quad's numbers at a multiple of the device's stride.
    buffers: Vec<Vec<u8>>,
    /// The buffer being filled.
    current: Vec<u8>,
    /// Each bind group's paint, mask and buffer, in the order the run first asked.
    keys: Vec<(PaintSource, Option<u32>, usize)>,
    /// The bind groups, made once every buffer is whole; index for index with `keys`.
    binds: Vec<wgpu::BindGroup>,
}

/// The drawable prefix of a plan's ops, unboxed for a pass. The caller guarantees
/// the slice holds no `Op::Child`.
pub(crate) fn run_ops(ops: &[Op]) -> Vec<RunOp> {
    ops.iter()
        .map(|op| match op {
            Op::Draw(batch) => RunOp::Batch(*batch),
            Op::Image(image) => RunOp::Image(**image),
            Op::Shaded(shaded) => RunOp::Shaded(**shaded),
            Op::Function(function) => RunOp::Function(**function),
            // The callers collect runs of drawable ops only.
            Op::Child(_) => unreachable!("a draw run contains no child composites"),
        })
        .collect()
}

impl Executor<'_> {
    /// One render pass of lane batches and single-quad ops onto `view`. Public to
    /// the device so the flat fast path draws the root directly onto the frame's
    /// target.
    pub(crate) fn draw_pass(
        &mut self,
        recorder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        load: PassLoad,
        ops: &[RunOp],
    ) -> Result<(), RenderError> {
        let (ready, needed, shadings) = self.prepare_run(ops, format)?;
        // The attachment this pass writes is the current plan's region, and every lane
        // maps device space through it (ADR 0036).
        let globals = self.device.region_globals(self.region);
        let mut pipelines = HashMap::new();
        for kind in needed {
            let (pipeline, compiled) = self.device.pipelines().get(kind, format)?;
            if let Some(duration) = compiled {
                self.phases.push(("pipeline compile (first use)", duration));
            }
            pipelines.insert(kind, pipeline);
        }
        let stamp = self.pass_stamp();
        let mut pass = recorder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("raster content"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Render onto transparency, always (brief section 3; §11.4.7).
                    load: match load {
                        PassLoad::Clear => wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        PassLoad::Keep => wgpu::LoadOp::Load,
                    },
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: stamp,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        self.scissor_pass(&mut pass, self.region, None);
        for item in &ready {
            let batch = match item {
                Ready::Batch(batch) => batch,
                Ready::Single { pipelines, bind } => {
                    for pipeline in pipelines.iter().flatten() {
                        pass.set_pipeline(pipeline);
                        pass.set_bind_group(0, bind, &[]);
                        pass.draw(0..4, 0..1);
                    }
                    continue;
                }
                Ready::Shaded {
                    pipelines,
                    bind,
                    offset,
                } => {
                    let Some(bind) = shadings.binds.get(*bind) else {
                        continue;
                    };
                    for pipeline in pipelines.iter().flatten() {
                        pass.set_pipeline(pipeline);
                        pass.set_bind_group(0, bind, &[*offset]);
                        pass.draw(0..4, 0..1);
                    }
                    continue;
                }
            };
            let buffer = match batch.kind {
                BatchKind::Rect => self.rect_buffer.as_ref(),
                BatchKind::Quad => self.quad_buffer.as_ref(),
            };
            let Some(buffer) = buffer else { continue };
            let bind = &self.lane_binds[&batch.mask];
            let family = batch_family(batch.kind);
            let draw =
                |pass: &mut wgpu::RenderPass<'_>, kind: &Kind, range: std::ops::Range<u32>| {
                    pass.set_pipeline(&pipelines[kind]);
                    pass.set_bind_group(0, &globals, &[]);
                    pass.set_bind_group(1, bind, &[]);
                    pass.set_vertex_buffer(0, buffer.slice(..));
                    pass.draw(0..4, range);
                };
            let whole = batch.first..batch.first.saturating_add(batch.count);
            match batch.style {
                DrawStyle::Over => draw(&mut pass, &family.over, whole),
                // One stage of §11.4.6, asked for by name (ADR 0025): the batch is
                // instanced like any other, because a single pass over independent
                // marks needs no interleaving.
                DrawStyle::DestOut => draw(&mut pass, &family.erase, whole),
                DrawStyle::Plus => draw(&mut pass, &family.add, whole),
                // §11.7.4.3's destination-over is one blend state, so overlapping marks
                // compose in draw order within the instanced call (`doc/adr/1295`).
                DrawStyle::DestOver => draw(&mut pass, &family.dest_over, whole),
                DrawStyle::Knockout => {
                    // §11.4.6 per element: erase by shape, then deposit — strictly
                    // interleaved, or overlapping elements compose wrongly
                    // (ADR 0010 carries the algebra).
                    for i in whole {
                        draw(&mut pass, &family.erase, i..i.saturating_add(1));
                        draw(&mut pass, &family.add, i..i.saturating_add(1));
                    }
                }
            }
        }
        Ok(())
    }

    /// Phase one of a draw pass: build every bind group and resolve every pipeline the
    /// run needs — none of which can happen while the pass borrows the recorder.
    ///
    /// The batches hand back a list of [`Kind`]s for the pass to look up, and the
    /// single-quad ops hand back resolved pipelines. That asymmetry is the point rather
    /// than an oversight: a batch's pipeline is one of a fixed table and is shared by every
    /// batch of its kind in the run, while a function quad's is generated per program and
    /// belongs to that op alone.
    fn prepare_run(
        &mut self,
        ops: &[RunOp],
        format: wgpu::TextureFormat,
    ) -> Result<(Vec<Ready>, Vec<Kind>, Shadings), RenderError> {
        let mut needed: Vec<Kind> = Vec::new();
        let want = |needed: &mut Vec<Kind>, kind: Kind| {
            if !needed.contains(&kind) {
                needed.push(kind);
            }
        };
        let mut ready: Vec<Ready> = Vec::with_capacity(ops.len());
        let mut shadings = Shadings::default();
        let stride = self.device.shading_stride();
        let limit = self.device.shading_buffer_limit();
        for op in ops {
            match op {
                RunOp::Batch(batch) => {
                    for kind in batch_kinds(batch) {
                        want(&mut needed, kind);
                    }
                    self.ensure_lane_bind(batch.mask);
                    ready.push(Ready::Batch(*batch));
                }
                RunOp::Image(image) => {
                    let kinds = style_kinds(
                        image.style,
                        Family {
                            over: Kind::ImageOver,
                            erase: Kind::ImageErase,
                            add: Kind::ImageAdd,
                            dest_over: Kind::ImageDestOver,
                        },
                    );
                    let mask = self.mask_for(image.mask);
                    let scratch = self.scratch_view.as_ref().unwrap_or(&self.dummy_view);
                    let bind = self.device.image_bind(image, self.region, mask, scratch)?;
                    let pipelines = self.lane_pipelines(kinds, format)?;
                    ready.push(Ready::Single { pipelines, bind });
                }
                RunOp::Shaded(shaded) => {
                    let kinds = style_kinds(
                        shaded.style,
                        Family {
                            over: Kind::ShadedOver,
                            erase: Kind::ShadedErase,
                            add: Kind::ShadedAdd,
                            dest_over: Kind::ShadedDestOver,
                        },
                    );
                    let (_, placement) = self.mask_for(shaded.mask);
                    let numbers = shading_params_bytes(shaded, self.region, placement);
                    let (bind, offset) = shadings.place(shaded, &numbers, stride, limit);
                    let pipelines = self.lane_pipelines(kinds, format)?;
                    ready.push(Ready::Shaded {
                        pipelines,
                        bind,
                        offset,
                    });
                }
                RunOp::Function(function) => {
                    let mask = self.mask_for(function.mask());
                    let scratch = self.scratch_view.as_ref().unwrap_or(&self.dummy_view);
                    let bind = self.function_bind(function, scratch, mask);
                    let pipelines = self.function_pipelines(function, format)?;
                    ready.push(Ready::Single { pipelines, bind });
                }
            }
        }
        let buffers: Vec<wgpu::Buffer> = shadings
            .finish()
            .iter()
            .map(|bytes| self.device.shading_params_buffer(bytes))
            .collect();
        let scratch = self.scratch_view.as_ref().unwrap_or(&self.dummy_view);
        for &(paint, mask, buffer) in &shadings.keys {
            let (mask, _) = self.mask_for(mask);
            let Some(params) = buffers.get(buffer) else {
                continue;
            };
            let bind = self.device.shaded_bind(paint, params, scratch, mask)?;
            shadings.binds.push(bind);
        }
        Ok((ready, needed, shadings))
    }

    /// The pipelines one fixed-table single-quad op draws with, compiling on first use.
    fn lane_pipelines(
        &mut self,
        kinds: [Option<Kind>; 2],
        format: wgpu::TextureFormat,
    ) -> Result<[Option<Arc<wgpu::RenderPipeline>>; 2], RenderError> {
        let mut resolved = [None, None];
        for (slot, kind) in resolved.iter_mut().zip(kinds) {
            let Some(kind) = kind else { continue };
            let (pipeline, compiled) = self.device.pipelines().get(kind, format)?;
            if let Some(duration) = compiled {
                self.phases.push(("pipeline compile (first use)", duration));
            }
            *slot = Some(pipeline);
        }
        Ok(resolved)
    }

    /// Build (once per mask) the lane bind group carrying atlas, scratch, and the mask's
    /// view together with where it sits.
    ///
    /// Keyed by the mask, which is what the placement is a property of — a batch changes
    /// mask, the region it draws into does not (that is `Globals`, group 0).
    fn ensure_lane_bind(&mut self, mask: Option<u32>) {
        if self.lane_binds.contains_key(&mask) {
            return;
        }
        let (mask_view, placement) = self.mask_for(mask);
        let atlas = self.atlas_view.as_ref().unwrap_or(&self.dummy_view);
        let scratch = self.scratch_view.as_ref().unwrap_or(&self.dummy_view);
        let bind = self.device.lane_bind(atlas, scratch, mask_view, placement);
        self.lane_binds.insert(mask, bind);
    }
}

/// One lane family's pipelines, one per [`Style`].
#[derive(Clone, Copy)]
struct Family {
    over: Kind,
    erase: Kind,
    add: Kind,
    dest_over: Kind,
}

/// The family an instanced batch draws in.
fn batch_family(kind: BatchKind) -> Family {
    match kind {
        BatchKind::Rect => Family {
            over: Kind::RectOver,
            erase: Kind::RectErase,
            add: Kind::RectAdd,
            dest_over: Kind::RectDestOver,
        },
        BatchKind::Quad => Family {
            over: Kind::CoverOver,
            erase: Kind::CoverErase,
            add: Kind::CoverAdd,
            dest_over: Kind::CoverDestOver,
        },
    }
}

fn batch_kinds(batch: &Batch) -> Vec<Kind> {
    style_kinds(batch.style, batch_family(batch.kind))
        .into_iter()
        .flatten()
        .collect()
}

/// The pipelines one style needs, in the order they must run — one lane family's [`Kind`]s
/// under [`Style::of`], which is where that rule is stated for all five families.
fn style_kinds(style: DrawStyle, family: Family) -> [Option<Kind>; 2] {
    Style::of(style).map(|wanted| {
        wanted.map(|wanted| match wanted {
            Style::Over => family.over,
            Style::Erase => family.erase,
            Style::Add => family.add,
            Style::DestOver => family.dest_over,
        })
    })
}

impl Shadings {
    /// Lay `numbers` into the run's current buffer at the next `stride`, closing it and
    /// starting another where it would pass `limit` bytes, and name the bind group the quad
    /// reads them through: the index of its paint, mask and buffer, and its offset there.
    fn place(
        &mut self,
        shaded: &ShadedOp,
        numbers: &[u8],
        stride: u64,
        limit: u64,
    ) -> (usize, u32) {
        let stride = usize::try_from(stride).unwrap_or(usize::MAX);
        let limit = usize::try_from(limit).unwrap_or(usize::MAX);
        if self.current.len().saturating_add(stride) > limit {
            self.buffers.push(std::mem::take(&mut self.current));
        }
        let buffer = self.buffers.len();
        let at = self.current.len();
        self.current.extend_from_slice(numbers);
        self.current
            .resize(at.saturating_add(stride.max(numbers.len())), 0);
        // In range: the limit is at most `u32::MAX + 1` (`Device::shading_buffer_limit`)
        // and `at` is below it by at least a stride.
        let offset = u32::try_from(at).unwrap_or(u32::MAX);
        let key = (shaded.paint, shaded.mask, buffer);
        let same = |(paint, mask, at): &(PaintSource, Option<u32>, usize)| {
            same_paint(*paint, key.0) && *mask == key.1 && *at == key.2
        };
        let bind = self.keys.iter().position(same).unwrap_or_else(|| {
            self.keys.push(key);
            self.keys.len().saturating_sub(1)
        });
        (bind, offset)
    }

    /// Every buffer's bytes, the current one last, once the run has placed every quad.
    fn finish(&mut self) -> Vec<Vec<u8>> {
        let mut buffers = std::mem::take(&mut self.buffers);
        if !self.current.is_empty() {
            buffers.push(std::mem::take(&mut self.current));
        }
        buffers
    }
}

/// Whether two paints name one texture.
fn same_paint(a: PaintSource, b: PaintSource) -> bool {
    match (a, b) {
        (PaintSource::Ramp(a), PaintSource::Ramp(b))
        | (PaintSource::Mesh(a), PaintSource::Mesh(b)) => a == b,
        _ => false,
    }
}

/// Where a run's shading quads put their numbers, and which bind group each reads them by.
#[cfg(test)]
mod tests {
    use super::{PaintSource, Shadings};
    use crate::encode::{DrawStyle, ShadedOp};

    /// A shading quad of `paint` under `mask`; nothing else about it is placed.
    fn quad(paint: PaintSource, mask: Option<u32>) -> ShadedOp {
        ShadedOp {
            paint,
            inv: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            kind_word: 0.0,
            extend_bits: 0,
            geo0: [0.0; 4],
            geo1: [0.0; 4],
            dest: [0.0; 4],
            coverage_origin: None,
            coverage_rect: [0.0; 4],
            clip: [0.0; 4],
            style: DrawStyle::Over,
            mask,
        }
    }

    /// **Every quad's numbers land at their own stride, and a quad that would pass the limit
    /// starts the next buffer at offset 0** — so no offset is past what the binding can
    /// address and every quad reads exactly the bytes it was given.
    #[test]
    fn numbers_are_laid_at_the_stride_and_roll_over_at_the_limit() {
        let mut shadings = Shadings::default();
        let numbers = |n: u8| [n; 176];
        let ramp = PaintSource::Ramp(7);
        let placed: Vec<(usize, u32)> = (0..5)
            .map(|n| shadings.place(&quad(ramp, None), &numbers(n), 256, 768))
            .collect();
        assert_eq!(
            placed,
            vec![(0, 0), (0, 256), (0, 512), (1, 0), (1, 256)],
            "three quads fill 768 bytes; the fourth opens a buffer and a group of its own"
        );
        let buffers = shadings.finish();
        assert_eq!(buffers.iter().map(Vec::len).collect::<Vec<_>>(), [768, 512]);
        for (n, (buffer, offset)) in placed.iter().enumerate() {
            let at = *offset as usize;
            let bytes = &buffers[*buffer][at..at + 176];
            assert!(
                bytes.iter().all(|&b| usize::from(b) == n),
                "quad {n}'s own numbers"
            );
        }
    }

    /// **One bind group per paint, mask and buffer**: a quad of a paint and mask the run has
    /// already bound reads that group, whatever was placed between.
    #[test]
    fn a_paint_and_mask_seen_before_share_their_group() {
        let mut shadings = Shadings::default();
        let numbers = [0_u8; 176];
        let groups: Vec<usize> = [
            (PaintSource::Ramp(1), None),
            (PaintSource::Ramp(2), None),
            (PaintSource::Ramp(1), Some(3)),
            (PaintSource::Mesh(1), None),
            (PaintSource::Ramp(1), None),
        ]
        .into_iter()
        .map(|(paint, mask)| shadings.place(&quad(paint, mask), &numbers, 256, 1 << 20).0)
        .collect();
        assert_eq!(groups, [0, 1, 2, 3, 0]);
        assert_eq!(shadings.keys.len(), 4);
    }
}
