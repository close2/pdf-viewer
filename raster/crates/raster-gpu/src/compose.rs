//! The frame executor: layers, composites, mask reductions, and the passes that
//! carry them, recorded into one submission.
//!
//! ISO 32000-2 clause 11, run: each [`LayerPlan`] renders bottom-up into its own
//! premultiplied texture; a [`ChildOp`] closes the current pass, renders the child,
//! and composites it onto the accumulated parent through `composite.wgsl` — a
//! ping-pong between the parent's two textures, because a pass cannot read its own
//! attachment. Soft masks realise first, in id order (they may only reference
//! earlier masks), each reduced to R8 by `reduce.wgsl`. A frame whose root has no
//! children and no masks draws straight into the target — the flat fast path M1
//! measured stays exactly as cheap as it was.
//!
//! **Count then allocate** (brief section 5): every layer texture and mask texture is priced
//! against the frame budget before anything is created; the refusal names both
//! numbers. Knockout batches run their erase/add pair strictly per element
//! (ADR 0010): interleaving is what makes overlapping knockout elements compose per
//! §11.4.6 rather than approximately.
//!
//! # This file, and the five modules under it
//!
//! What is left here is the frame's **state and its walk**: the [`Executor`] every pass
//! borrows, [`Executor::render_plan`]'s recursion over the plan tree, the scissor rule
//! every pass obeys, and the two timestamps that make `execute` mean first pass to last.
//! Each kind of pass is its own module, because a pass is only reviewable beside the
//! reason it loads, scissors and reads the way it does:
//!
//! - `region` — where a plan's pixels are: the rectangle arithmetic, and no device.
//! - `draw` — the content pass, and the run preparation that must precede it.
//! - `child` — §11.3.6: a finished child composited onto its parent.
//! - `blit` — pixels moved and not changed: the seed copy, the root, the damage patch.
//! - `masks` — §11.5's soft masks, realised first and bound by everything after.
//! - `function` — ADR 0053's quad: the one paint whose pipeline is generated per program
//!   rather than taken from a fixed table.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::device::{Device, PassQuery};
use crate::encode::{Encoded, Op};
use crate::error::RenderError;
use crate::layers::LayerPool;
use crate::mask::Realised;

mod blit;
mod child;
mod draw;
mod function;
mod masks;
mod region;

// `RunOp` is deliberately not re-exported: the device names `run_ops` and `draw_pass`
// and lets the item type follow, so a name nothing writes would be a name to keep in
// step for nothing.
use child::Composite;
pub(crate) use draw::{PassLoad, run_ops};
pub(crate) use region::Region;
use region::overlap;

/// Everything the executor holds for one frame.
pub(crate) struct Executor<'a> {
    pub device: &'a Device,
    pub encoded: &'a Encoded,
    pub width: u32,
    pub height: u32,
    /// The frame's layer textures, acquired per plan and given back when the parent's
    /// composite has read them (ADR 0020).
    pub pool: LayerPool,
    /// Realised masks by mask index: the R8 texture and where it sits (ADR 0037).
    pub masks: Vec<Option<Realised>>,
    /// Lane instance buffers.
    pub rect_buffer: Option<wgpu::Buffer>,
    pub quad_buffer: Option<wgpu::Buffer>,
    /// Lane bind groups per mask index (`None` key = no mask), built lazily.
    pub lane_binds: HashMap<Option<u32>, wgpu::BindGroup>,
    pub scratch_view: Option<wgpu::TextureView>,
    pub dummy_view: wgpu::TextureView,
    pub atlas_view: Option<wgpu::TextureView>,
    pub first_pass_stamped: bool,
    /// Whether the pass recorded next is the frame's last, onto its target, which carries
    /// the end timestamp (ADR 1618).
    pub closing: bool,
    pub query: Option<&'a PassQuery>,
    pub phases: Vec<(&'static str, Duration)>,
    /// When a frame patches a damage list (ADR 0012), every internal pass is
    /// scissored to the damage bounding box — every pass in this pipeline is
    /// pixel-local (a fragment reads attachments only at its own coordinate), so
    /// pixels outside the scissor are never needed and never paid for.
    pub scissor: Option<[u32; 4]>,
    /// The region of the frame the pass being recorded renders into (ADR 0036): the
    /// whole target while the root is drawing, a plan's own rectangle inside a group.
    pub region: Region,
}

/// One plan's finished pixels — the texture itself rather than a view, because the
/// caller is what gives it back to the pool (ADR 0020).
pub(crate) struct Rendered {
    texture: wgpu::Texture,
    /// Where in device space this plan's texture sits, which the composite that reads it
    /// must subtract (ADR 0036).
    region: Region,
}

impl Rendered {
    /// The texture holding this plan's result.
    pub(crate) fn view(&self) -> wgpu::TextureView {
        view_of(&self.texture)
    }

    /// Where in device space that texture sits.
    pub(crate) const fn region(&self) -> Region {
        self.region
    }
}

/// The default view of a whole texture, which is the only kind this crate makes.
fn view_of(texture: &wgpu::Texture) -> wgpu::TextureView {
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

impl Executor<'_> {
    /// Whether the frame can skip layers entirely and draw the root straight into
    /// the target.
    pub(crate) fn is_flat(encoded: &Encoded) -> bool {
        encoded.layers.is_empty() && encoded.mask_plans.iter().all(Option::is_none)
    }

    /// Render one plan into a pair borrowed from the pool; returns the pair and which
    /// of its two textures holds the result, for the caller to read and then release
    /// (ADR 0020).
    ///
    /// `plan_index` 0 is the root, `layers[i]` is `i + 1`. Recursion depth is the plan
    /// tree's depth, which the scene builder bounds.
    ///
    /// `seed` is §11.4.4's initial backdrop for a non-isolated group: the parent's
    /// accumulated view, blitted in before anything draws (ADR 0019). `None` is
    /// §11.4.5's transparency, and then the first pass clears instead.
    pub(crate) fn render_plan(
        &mut self,
        recorder: &mut wgpu::CommandEncoder,
        plan_index: usize,
        seed: Option<&wgpu::TextureView>,
    ) -> Result<Rendered, RenderError> {
        let plan = if plan_index == 0 {
            &self.encoded.root
        } else {
            &self.encoded.layers[plan_index.saturating_sub(1)]
        };
        // **As big as the plan, not as big as the target** (ADR 0036) — the root
        // included, since ADR 0039 measured what roots actually mark. One exception is
        // left: a plan that is seeded takes its parent's region, because §11.4.4's initial
        // backdrop is copied in texel for texel and the interpolation that later takes it
        // back out (ADR 0019) is stated over the whole of the group's own buffer.
        //
        // A plan that marks nothing still needs somewhere for the composite to read, and
        // one texel is enough for a rectangle nobody samples.
        let region = if seed.is_some() {
            self.region
        } else {
            Region::of(plan.bounds, self.width, self.height)
        };
        // One texture, not a ping-pong pair (ADR 0038): a child's composite writes into
        // this same accumulator, because the pixels it needs to read are copied out of it
        // first — at the child's size, which is all the composite writes.
        let accumulator = self.pool.acquire(self.device, region.width, region.height);
        let view = view_of(&accumulator);
        let outer = std::mem::replace(&mut self.region, region);
        // Whether a recorded pass has written the whole accumulator, or the pending
        // composite's pass will: that pass clears it when nothing has (ADR 1618).
        let mut cleared = false;
        if let Some(backdrop) = seed {
            // The parent's region and the child's are the same rectangle for a seeded
            // plan, so this copy is texel for texel and reads nothing outside.
            self.copy_pass(
                recorder,
                "raster seed non-isolated group",
                (Some(backdrop), region),
                (&view, region),
                [0.0, 0.0],
            )?;
            cleared = true;
        }
        // A child's composite whose copy is recorded and whose draw is not. **It is drawn
        // first in the pass of the run after it**, because both write this accumulator and
        // nothing between them reads it: two passes onto one attachment, each loading what
        // the last stored, deposit what one pass deposits drawing the same draws in the
        // same order (ADR 1618). A child, or the plan's end, records it in a pass of its own,
        // since the next child's copy has to read what it wrote.
        let mut pending: Option<Composite> = None;
        let mut op_index = 0;
        while op_index < plan.ops.len() {
            match &plan.ops[op_index] {
                Op::Draw(_) | Op::Image(_) | Op::Shaded(_) | Op::Function(_) => {
                    // A run of consecutive drawable ops becomes one pass.
                    let run_start = op_index;
                    while op_index < plan.ops.len() && !matches!(plan.ops[op_index], Op::Child(_)) {
                        op_index = op_index.saturating_add(1);
                    }
                    let run = run_ops(&plan.ops[run_start..op_index]);
                    let load = match &pending {
                        Some(composite) => composite.load(),
                        None if cleared => PassLoad::Keep,
                        None => PassLoad::Clear,
                    };
                    self.draw_pass(
                        recorder,
                        (&view, wgpu::TextureFormat::Rgba8Unorm),
                        load,
                        &run,
                        pending.take(),
                    )?;
                    cleared = true;
                }
                Op::Child(child) => {
                    let child_op = *child;
                    op_index = op_index.saturating_add(1);
                    if let Some(composite) = pending.take() {
                        self.composite_pass(recorder, &view, composite);
                    }
                    // §11.4.4: a non-isolated group's elements composite onto the
                    // group's backdrop, so its buffer begins as a copy of what is under
                    // it. The composite that follows takes that contribution back out
                    // (ADR 0019), which is why this seeding is only half a change. A
                    // non-isolated group's backdrop has to be in the accumulator to be
                    // copied, so an accumulator nothing has written is cleared first.
                    if !child_op.isolated && !cleared {
                        self.draw_pass(
                            recorder,
                            (&view, wgpu::TextureFormat::Rgba8Unorm),
                            PassLoad::Clear,
                            &[],
                            None,
                        )?;
                        cleared = true;
                    }
                    let seed = (!child_op.isolated).then_some(&view);
                    let child =
                        self.render_plan(recorder, child_op.layer.saturating_add(1), seed)?;
                    // NOTE 4's second accumulator, where the group's own blend needs
                    // Table 140's group alpha: the same elements onto transparency.
                    let group_alpha = match child_op.group_alpha {
                        Some(layer) => {
                            Some(self.render_plan(recorder, layer.saturating_add(1), None)?)
                        }
                        None => None,
                    };
                    pending = self.composite_child(
                        recorder,
                        (&view, region, cleared),
                        (child, group_alpha),
                        &child_op,
                    )?;
                    cleared = cleared || pending.is_some();
                }
            }
        }
        if let Some(composite) = pending.take() {
            self.composite_pass(recorder, &view, composite);
        }
        if !cleared {
            self.draw_pass(
                recorder,
                (&view, wgpu::TextureFormat::Rgba8Unorm),
                PassLoad::Clear,
                &[],
                None,
            )?;
        }
        self.region = outer;
        Ok(Rendered {
            texture: accumulator,
            region,
        })
    }

    /// Scissor an internal pass: to the damage bounding box when this frame patches, and
    /// to `limit` when the pass writes only part of its attachment.
    ///
    /// `into` is the region of the frame the pass's attachment holds, because that is the
    /// space a scissor is stated in while both of the others are stated in device space
    /// (ADR 0036 made the two differ). A pass rendering into a plan smaller than the
    /// damage box is a wgpu validation error otherwise, and that is a panic inside a
    /// library rather than a refusal.
    ///
    /// `limit` is the composite's `child ∩ parent` (ADR 0038), and is the reason this
    /// takes two rectangles rather than one: a patched frame compositing a small group
    /// must honour both, and only their overlap does.
    fn scissor_pass(&self, pass: &mut wgpu::RenderPass<'_>, into: Region, limit: Option<Region>) {
        if let Some(rect) = self.scissor_rect(into, limit) {
            pass.set_scissor_rect(rect[0], rect[1], rect[2], rect[3]);
        }
    }

    /// The rectangle [`Self::scissor_pass`] sets, in `into`'s space; `None` where the pass
    /// writes its whole attachment.
    fn scissor_rect(&self, into: Region, limit: Option<Region>) -> Option<[u32; 4]> {
        if self.scissor.is_none() && limit.is_none() {
            return None;
        }
        let mut rect = [0, 0, into.width, into.height];
        if let Some(damage) = self.scissor {
            rect = overlap(rect, into.scissor_in(damage));
        }
        if let Some(limit) = limit {
            rect = overlap(rect, into.scissor_in(limit.rect()));
        }
        Some(rect)
    }

    /// The end-of-frame timestamp of a frame that records no pass: an empty pass whose only
    /// job is the second timestamp. Every route that draws stamps its end on its last pass
    /// instead ([`Self::closing`]), because a pass of its own onto the same target costs a
    /// pass and changes no pixel (ADR 1618).
    pub(crate) fn end_stamp(
        &mut self,
        recorder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let Some(query) = self.query else { return };
        let mut _pass = recorder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("raster end stamp"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: Some(wgpu::RenderPassTimestampWrites {
                query_set: &query.set,
                beginning_of_pass_write_index: None,
                end_of_pass_write_index: Some(1),
            }),
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }

    /// The first pass stamps the frame's beginning and the closing pass its end, so
    /// `execute` spans first pass to last whatever the pass count; every other pass stamps
    /// nothing.
    fn pass_stamp(&mut self) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        let beginning = (!self.first_pass_stamped).then_some(0);
        let end = self.closing.then_some(1);
        self.first_pass_stamped = true;
        if beginning.is_none() && end.is_none() {
            return None;
        }
        self.query.map(|q| wgpu::RenderPassTimestampWrites {
            query_set: &q.set,
            beginning_of_pass_write_index: beginning,
            end_of_pass_write_index: end,
        })
    }
}

/// Finish and submit the executor's recording, without waiting for the device: what the host
/// spent on it, for the fallback `execute`, and the submission a collect waits on (ADR 1595).
pub(crate) fn submit(
    device: &Device,
    recorder: wgpu::CommandEncoder,
) -> (Duration, wgpu::SubmissionIndex) {
    let started = Instant::now();
    let (_, queue) = device.wgpu();
    let submission = queue.submit([recorder.finish()]);
    (started.elapsed(), submission)
}
