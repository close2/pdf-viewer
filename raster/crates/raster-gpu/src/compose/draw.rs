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

use crate::device::{SHADING_PARAMS_BYTES, SHADING_QUADS_PER_WINDOW, shading_params_bytes};
use crate::encode::{Batch, BatchKind, DrawStyle, FunctionOp, ImageOp, Op, PaintSource, ShadedOp};
use crate::error::RenderError;
use crate::pipeline::{Kind, Style};

use super::{Composite, Executor};

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

impl PassLoad {
    /// The attachment's load operation: transparency, or what an earlier pass stored.
    pub(crate) const fn op(self) -> wgpu::LoadOp<wgpu::Color> {
        match self {
            PassLoad::Clear => wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            PassLoad::Keep => wgpu::LoadOp::Load,
        }
    }
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
    /// A run of shading quads drawn as one instanced draw: their pipelines, which of the run's
    /// shading bind groups they read ([`Shadings::binds`]), the window of that group's buffer
    /// their numbers sit in (ADR 1555), and their indices in it (ADR 1594).
    Shaded {
        pipelines: [Option<Arc<wgpu::RenderPipeline>>; 2],
        bind: usize,
        offset: u32,
        instances: std::ops::Range<u32>,
    },
}

impl Ready {
    /// Take the next shading quad into this item where one draw draws both exactly as two
    /// would: the same pipeline, bind group and window, the quad the next index of it, and a
    /// style of one stage.
    ///
    /// **Why one draw is two draws' pixels.** A draw's primitives are rasterised and blended in
    /// their order — instance by instance, as consecutive draws are — so a run of quads drawn
    /// as instances deposits what the same quads drawn one by one deposit, each fragment reading
    /// the same 176 bytes through its instance index that it read at its own offset. A style of
    /// two stages (§11.4.6's knockout, erase then deposit per element) is never taken in: its
    /// stages interleave element by element (ADR 0010), which one draw per stage would not.
    fn absorb(
        &mut self,
        next_pipelines: &[Option<Arc<wgpu::RenderPipeline>>; 2],
        next_bind: usize,
        next_offset: u32,
        index: u32,
    ) -> bool {
        let Ready::Shaded {
            pipelines,
            bind,
            offset,
            instances,
        } = self
        else {
            return false;
        };
        let one_stage =
            |stages: &[Option<Arc<wgpu::RenderPipeline>>; 2]| matches!(stages, [Some(_), None]);
        let same_pipeline = match (&pipelines[0], &next_pipelines[0]) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        };
        if !(one_stage(pipelines)
            && one_stage(next_pipelines)
            && same_pipeline
            && *bind == next_bind
            && *offset == next_offset
            && instances.end == index)
        {
            return false;
        }
        instances.end = index.saturating_add(1);
        true
    }
}

/// The shading quads of one run, gathered before any of their bindings exist: every quad's
/// numbers laid into as few buffers as the device allows, in windows of
/// [`SHADING_QUADS_PER_WINDOW`] quads, and one bind group per paint, mask and buffer, which
/// every quad reading them shares at its window's offset and its own index there (ADRs 1555,
/// 1594).
///
/// **Why one buffer and few groups.** A buffer and a bind group made per quad were the
/// largest host cost of a pass on a page of many shadings: `bug1721218_reduced.pdf` draws
/// 3 583 ops a render through five ramps and no mask, and making them took 9 to 10 ms of each
/// of its two renders. The numbers each quad reads are the bytes it read from a buffer of its
/// own; only where they sit has changed, so every draw shades what it shaded.
#[derive(Default)]
struct Shadings {
    /// The closed buffers' bytes: windows at a multiple of the device's window stride, and
    /// every quad's numbers at its index in its window.
    buffers: Vec<Vec<u8>>,
    /// The buffer being filled.
    current: Vec<u8>,
    /// Where the window being filled starts in `current`, and how many quads it holds; `None`
    /// before the buffer's first window opens.
    window: Option<(usize, u64)>,
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
    ///
    /// `composites` are children's composites onto the same attachment, drawn first in this
    /// pass, in their children's order, rather than in a pass of their own (ADR 1618, 1631). **The pixels are the two passes'**: a
    /// pass's draws are rasterised and blended in the order they are recorded, as the
    /// passes are, every draw here reads only textures other than the attachment, and an
    /// `Rgba8Unorm` attachment stored by one pass and loaded by the next carries its bytes
    /// unchanged — so each pixel meets the same writes, in the same order, from the same
    /// value. What changes is the scissor between them, set for each draw as each pass
    /// set it.
    pub(crate) fn draw_pass(
        &mut self,
        recorder: &mut wgpu::CommandEncoder,
        target: (&wgpu::TextureView, wgpu::TextureFormat),
        load: PassLoad,
        ops: &[RunOp],
        composites: Vec<Composite>,
    ) -> Result<(), RenderError> {
        let (view, format) = target;
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
                    load: load.op(),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: stamp,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if composites.is_empty() {
            self.scissor_pass(&mut pass, self.region, None);
        } else {
            for composite in &composites {
                self.draw_composite(&mut pass, composite);
            }
            // A composite's scissor is its own; the marks take the pass's, which is the
            // whole attachment where the frame does not patch.
            let whole = [0, 0, self.region.width, self.region.height];
            let rect = self.scissor_rect(self.region, None).unwrap_or(whole);
            pass.set_scissor_rect(rect[0], rect[1], rect[2], rect[3]);
        }
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
                    instances,
                } => {
                    let Some(bind) = shadings.binds.get(*bind) else {
                        continue;
                    };
                    for pipeline in pipelines.iter().flatten() {
                        pass.set_pipeline(pipeline);
                        pass.set_bind_group(0, bind, &[*offset]);
                        pass.draw(0..4, instances.clone());
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
        drop(pass);
        for composite in composites {
            self.release_composite(composite);
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
        let stride = self.device.shading_window_stride();
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
                    let (bind, offset, index) = shadings.place(shaded, &numbers, stride, limit);
                    let pipelines = self.lane_pipelines(kinds, format)?;
                    let absorbed = ready
                        .last_mut()
                        .is_some_and(|last| last.absorb(&pipelines, bind, offset, index));
                    if !absorbed {
                        ready.push(Ready::Shaded {
                            pipelines,
                            bind,
                            offset,
                            instances: index..index.saturating_add(1),
                        });
                    }
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
    /// Lay `numbers` into the run's current window at its next index, opening the next window
    /// `stride` bytes on when this one is full and starting another buffer where that window
    /// would pass `limit` bytes, and name where the quad reads them: the index of its paint,
    /// mask and buffer's bind group, its window's offset there, and its index in the window.
    fn place(
        &mut self,
        shaded: &ShadedOp,
        numbers: &[u8],
        stride: u64,
        limit: u64,
    ) -> (usize, u32, u32) {
        let stride = usize::try_from(stride).unwrap_or(usize::MAX);
        let limit = usize::try_from(limit).unwrap_or(usize::MAX);
        let (start, count) = match self.window {
            Some((start, count)) if count < SHADING_QUADS_PER_WINDOW => (start, count),
            full_or_none => {
                let mut start = full_or_none.map_or(0, |(start, _)| start.saturating_add(stride));
                if start > 0 && start.saturating_add(stride) > limit {
                    self.buffers.push(std::mem::take(&mut self.current));
                    start = 0;
                }
                // The whole window is allocated when it opens, so the binding's window is
                // inside the buffer however few quads it ends up holding.
                self.current.resize(start.saturating_add(stride), 0);
                (start, 0)
            }
        };
        // In range: `count` is below `SHADING_QUADS_PER_WINDOW`, and a window is at least that
        // many `Params` long (`Device::shading_window_stride`).
        let params = usize::try_from(SHADING_PARAMS_BYTES).unwrap_or(usize::MAX);
        let at = start.saturating_add(
            usize::try_from(count)
                .unwrap_or(usize::MAX)
                .saturating_mul(params),
        );
        let end = at.saturating_add(numbers.len());
        if let Some(slot) = self.current.get_mut(at..end) {
            slot.copy_from_slice(numbers);
        }
        self.window = Some((start, count.saturating_add(1)));
        let buffer = self.buffers.len();
        // In range: the limit is at most `u32::MAX + 1` (`Device::shading_buffer_limit`)
        // and `start` is below it by at least a stride; `count` is below 93.
        let offset = u32::try_from(start).unwrap_or(u32::MAX);
        let index = u32::try_from(count).unwrap_or(u32::MAX);
        let key = (shaded.paint, shaded.mask, buffer);
        let same = |(paint, mask, at): &(PaintSource, Option<u32>, usize)| {
            same_paint(*paint, key.0) && *mask == key.1 && *at == key.2
        };
        let bind = self.keys.iter().position(same).unwrap_or_else(|| {
            self.keys.push(key);
            self.keys.len().saturating_sub(1)
        });
        (bind, offset, index)
    }

    /// Every buffer's bytes, the current one last, once the run has placed every quad; each
    /// buffer ends with its last window whole.
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
    use super::{PaintSource, SHADING_QUADS_PER_WINDOW, Shadings};
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

    /// **Every quad's numbers land at its index in its window, a full window opens the next
    /// one a stride on, and a window that would pass the limit starts the next buffer at offset
    /// 0** — so no offset is past what the binding can address and every instance reads
    /// exactly the bytes it was given (ADR 1594).
    #[test]
    fn numbers_are_laid_by_index_in_windows_and_roll_over_at_the_limit() {
        let mut shadings = Shadings::default();
        let per = usize::try_from(SHADING_QUADS_PER_WINDOW).unwrap();
        let stride = 16_384_u64;
        let numbers = |n: usize| [u8::try_from(n % 251).unwrap(); 176];
        let ramp = PaintSource::Ramp(7);
        let placed: Vec<(usize, u32, u32)> = (0..=2 * per)
            .map(|n| shadings.place(&quad(ramp, None), &numbers(n), stride, 2 * stride))
            .collect();
        for (n, &(bind, offset, index)) in placed.iter().enumerate() {
            let window = n / per;
            let expected = match window {
                0 | 1 => (0, u32::try_from(window).unwrap() * 16_384, n % per),
                _ => (1, 0, 0),
            };
            assert_eq!(
                (bind, offset, index),
                (expected.0, expected.1, u32::try_from(expected.2).unwrap()),
                "quad {n}"
            );
        }
        let buffers = shadings.finish();
        assert_eq!(
            buffers.iter().map(Vec::len).collect::<Vec<_>>(),
            [32_768, 16_384],
            "two windows fill the first buffer; the third opens a buffer of its own, whole"
        );
        for (n, (bind, offset, index)) in placed.iter().enumerate() {
            let at = *offset as usize + *index as usize * 176;
            let bytes = &buffers[*bind][at..at + 176];
            assert!(
                bytes.iter().all(|&b| b == numbers(n)[0]),
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
        .map(|(paint, mask)| {
            shadings
                .place(&quad(paint, mask), &numbers, 16_384, 1 << 20)
                .0
        })
        .collect();
        assert_eq!(groups, [0, 1, 2, 3, 0]);
        assert_eq!(shadings.keys.len(), 4);
    }
}
