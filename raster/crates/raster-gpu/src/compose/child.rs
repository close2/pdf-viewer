//! One finished child, composited onto the plan that accumulates it — ISO 32000-2
//! §11.3.6, and the copy it takes on a device that cannot read what it writes.
//!
//! `composite.wgsl` needs the backdrop and the child at once and writes the backdrop's
//! own attachment, so the pixels it is about to cover are copied out first. That copy
//! is why a plan needs one texture rather than a ping-pong pair (ADR 0038), and its
//! size — `child ∩ parent` rather than the whole plan — is why the pair below takes two
//! regions everywhere.
//!
//! **The copy is a pass; the composite is a draw** (ADR 1618). The copy writes a texture
//! of its own and so ends the accumulator's pass, but the composite writes the accumulator
//! itself, as the run of marks after it does — so its draw is carried to the next pass onto
//! the accumulator as a [`Composite`] and drawn there first, and has a pass of its own only
//! where a child or the plan's end comes next.

use std::sync::Arc;

use crate::encode::ChildOp;
use crate::error::RenderError;
use crate::pipeline::Kind;

use super::{Executor, PassLoad, Region, Rendered, view_of};

/// A composite whose backdrop is copied and whose draw is not yet recorded: everything the
/// draw needs, and the textures it reads, which go back to the pool once it is recorded.
pub(crate) struct Composite {
    pipeline: Arc<wgpu::RenderPipeline>,
    bind: wgpu::BindGroup,
    /// The accumulator's region, which the scissor is stated in.
    region: Region,
    /// `child ∩ parent`: the part of the accumulator the draw may write, and the copy's
    /// rectangle.
    onto: Region,
    /// What the pass carrying the draw does first: `Clear` where nothing has written the
    /// accumulator, whose backdrop is then transparency (§11.4.5).
    load: PassLoad,
    /// The backdrop's copy, the child, and its group alpha where it has one.
    reads: Vec<wgpu::Texture>,
}

impl Composite {
    /// What the pass that carries this draw loads.
    pub(crate) const fn load(&self) -> PassLoad {
        self.load
    }
}

impl Executor<'_> {
    /// Copy out the backdrop one finished child will cover, and prepare the composite that
    /// covers it (§11.3.6); `None` where the child meets its parent nowhere.
    ///
    /// The copy is what lets there be one texture per plan rather than two (ADR 0038): the
    /// composite cannot read the attachment it writes, so the pixels it is about to cover
    /// are copied out — **at the size of `child ∩ parent`**, because that is the whole of
    /// what it writes. Outside the child's own rectangle every branch of `composite.wgsl`
    /// collapses to the backdrop it read, so those pixels are already what the draw would
    /// put there.
    ///
    /// `into.2` says whether anything has written the accumulator. Where nothing has, its
    /// backdrop is §11.4.5's transparency: the copy is cleared rather than read, and the
    /// pass that carries the composite clears the accumulator first — the zeros an empty
    /// pass would have stored and the blit copied, without either pass (ADR 1618).
    ///
    /// A child that meets its parent nowhere composites to nothing: the clip that shrank
    /// the parent's bounds is the same clip whose coverage the pass would multiply by, and
    /// it is zero everywhere the child could have contributed. **By the time a child is
    /// rendered it is too late to save anything by discovering that** — the encoder drops
    /// such a child before it becomes an op at all (ADR 0041), which is where the clip
    /// that emptied it is known.
    ///
    /// `child.1` is the group-alpha accumulator a non-isolated group composited under a
    /// blend of its own is drawn with (`ChildOp::group_alpha`), and `None` for every other.
    pub(super) fn composite_child(
        &mut self,
        recorder: &mut wgpu::CommandEncoder,
        into: (&wgpu::TextureView, Region, bool),
        child: (Rendered, Option<Rendered>),
        op: &ChildOp,
    ) -> Result<Option<Composite>, RenderError> {
        let (accumulator, region, written) = into;
        let (child, group_alpha) = child;
        let Some(onto) = region.meet(child.region()) else {
            self.pool.release(child.texture);
            if let Some(group_alpha) = group_alpha {
                self.pool.release(group_alpha.texture);
            }
            return Ok(None);
        };
        let copy = self.pool.acquire(self.device, onto.width, onto.height);
        let copy_view = view_of(&copy);
        #[expect(clippy::cast_precision_loss)] // extents are exact in f32
        let from = [
            onto.x.saturating_sub(region.x) as f32,
            onto.y.saturating_sub(region.y) as f32,
        ];
        self.copy_pass(
            recorder,
            "raster composite backdrop",
            (written.then_some(accumulator), region),
            (&copy_view, onto),
            from,
        )?;
        let mask = self.mask_for(op.mask);
        let scratch = self.scratch_view.as_ref().unwrap_or(&self.dummy_view);
        let child_view = child.view();
        let group_alpha_view = group_alpha.as_ref().map(Rendered::view);
        // A composite that reads no group alpha binds the frame's dummy texture in its
        // place with an empty region, which `composite.wgsl` never samples.
        let group_alpha_binding = match (&group_alpha_view, &group_alpha) {
            (Some(view), Some(rendered)) => (view, rendered.region()),
            _ => (
                &self.dummy_view,
                Region {
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 0,
                },
            ),
        };
        let bind = self.device.composite_bind(
            op,
            region,
            (&copy_view, onto),
            (&child_view, child.region()),
            group_alpha_binding,
            mask,
            scratch,
        );
        let (pipeline, compiled) = self
            .device
            .pipelines()
            .get(Kind::Composite, wgpu::TextureFormat::Rgba8Unorm)?;
        if let Some(duration) = compiled {
            self.phases.push(("pipeline compile (first use)", duration));
        }
        let mut reads = vec![copy, child.texture];
        reads.extend(group_alpha.map(|rendered| rendered.texture));
        Ok(Some(Composite {
            pipeline,
            bind,
            region,
            onto,
            load: if written {
                PassLoad::Keep
            } else {
                PassLoad::Clear
            },
            reads,
        }))
    }

    /// Draw a prepared composite into the pass recording onto its accumulator:
    /// `accumulator = child over/blended-onto backdrop` per §11.3.6, **scissored to `onto`**
    /// — the part the child can reach (ADR 0038) — and to the damage box where the frame
    /// patches. Everything outside the scissor is already what the draw would have
    /// written there.
    pub(super) fn draw_composite(&self, pass: &mut wgpu::RenderPass<'_>, composite: &Composite) {
        self.scissor_pass(pass, composite.region, Some(composite.onto));
        pass.set_pipeline(&composite.pipeline);
        pass.set_bind_group(0, &composite.bind, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Give back the textures a composite read, once its draw is recorded: a sibling may
    /// have them now (ADR 0020).
    pub(super) fn release_composite(&mut self, composite: Composite) {
        for texture in composite.reads {
            self.pool.release(texture);
        }
    }

    /// A composite in a pass of its own, where no run of marks follows it onto the
    /// accumulator: a child comes next, whose copy must read what this draw wrote, or the
    /// plan ends.
    pub(super) fn composite_pass(
        &mut self,
        recorder: &mut wgpu::CommandEncoder,
        accumulator: &wgpu::TextureView,
        composite: Composite,
    ) {
        let stamp = self.pass_stamp();
        let mut pass = recorder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("raster composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: accumulator,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: composite.load.op(),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: stamp,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        self.draw_composite(&mut pass, &composite);
        drop(pass);
        self.release_composite(composite);
    }
}
