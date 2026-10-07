//! One finished child, composited onto the plan that accumulates it — ISO 32000-2
//! §11.3.6, and the copy it takes on a device that cannot read what it writes.
//!
//! `composite.wgsl` needs the backdrop and the child at once and writes the backdrop's
//! own attachment, so the pixels it is about to cover are copied out first. That copy
//! is why a plan needs one texture rather than a ping-pong pair (ADR 0038), and its
//! size — `child ∩ parent` rather than the whole plan — is why the pair below takes two
//! regions everywhere.
//!
//! **The copy is a transfer; the composite is a draw.** The copy writes a texture of its
//! own and so ends the accumulator's pass, and is recorded as `copy_texture_to_texture`
//! between the two passes rather than as a pass of its own (ADR 1630). The composite
//! writes the accumulator itself, as the run of marks after it does — so its draw is
//! carried to the next pass onto the accumulator as a [`Composite`] and drawn there first
//! (ADR 1618). Where a child comes next, it is carried past that child too when the two
//! cover rectangles apart and the frame's priced peak holds both (ADR 1631); otherwise, and
//! at the plan's end, it has a pass of its own.

use std::sync::Arc;

use crate::encode::{ChildOp, LayerPlan};
use crate::error::RenderError;
use crate::layers::bytes_of;
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
    /// The bytes of the child's texture and the copy, which this composite holds alive
    /// until its draw is recorded; `None` where it also holds a group alpha, which the
    /// frame's price does not count, and so may not wait beside another child (ADR 1631).
    waits: Option<u64>,
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
    /// **The copy is a transfer** (ADR 1630): `copy_texture_to_texture` of `onto` out of the
    /// accumulator into a texture exactly `onto`'s size, which copies every byte unchanged
    /// by definition, needs no pipeline, bind group or uniform, and is recorded into the
    /// command buffer the passes around it share. It sees what the pass before it stored:
    /// that pass ends with `StoreOp::Store`, and `wgpu` moves the accumulator from colour
    /// attachment to copy source, and the copy from copy destination to sampled texture,
    /// with a barrier between each pass and the transfer beside it. Under a damage scissor
    /// it copies the whole of `onto` where only `onto ∩ damage` is read, because the draw
    /// that reads the copy is scissored to that and reads at its own fragment; the texels
    /// outside it are the accumulator's own, cleared by its first pass. And a transfer that
    /// wrote less than the whole copy would have `wgpu` clear the copy first, since it
    /// tracks a texture's initialisation per subresource rather than per rectangle.
    ///
    /// `into.2` says whether anything has written the accumulator. Where nothing has, its
    /// backdrop is §11.4.5's transparency: the copy is cleared rather than read, and the
    /// pass that carries the composite clears the accumulator first — the zeros an empty
    /// pass would have stored and a copy would have read, without either (ADR 1618).
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
        into: (&wgpu::Texture, Region, bool),
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
        if written {
            transfer_backdrop(recorder, accumulator, region, (&copy, onto));
        } else {
            self.copy_pass(
                recorder,
                "raster composite backdrop",
                (None, region),
                (&copy_view, onto),
                [0.0, 0.0],
            )?;
        }
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
        let waits = group_alpha
            .is_none()
            .then(|| bytes_of(child.region()).saturating_add(bytes_of(onto)));
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
            waits,
        }))
    }

    /// What the frame was priced at for `plan`'s heaviest child, which composites waiting
    /// past a child may not take it beyond (ADR 1631) — and nothing for a seeded plan, whose
    /// region is its parent's rather than the one it was priced at, so nothing waits there.
    pub(super) fn wait_ceiling(&self, plan: &LayerPlan, region: Region, seeded: bool) -> u64 {
        if seeded {
            return 0;
        }
        self.prices.heaviest_child(self.encoded, plan, region)
    }

    /// Whether the composites waiting for a pass may wait past the child `next` too, and be
    /// drawn with its composite in one pass (ADR 1631).
    ///
    /// **The bytes**: `next`'s backdrop is copied before the waiting composites are drawn,
    /// so the accumulator under `next` must be what their draws leave it — which it is
    /// where `next`'s rectangle meets none of theirs, since each draw writes only its own
    /// `onto`. `next` must be isolated, because a seeded child copies the whole accumulator
    /// and is not drawn at its own rectangle; and the accumulator must already be written
    /// by a recorded pass, because a copy is a read of it.
    ///
    /// **The budget**: while `next` renders, every waiting composite still holds its
    /// child's texture and its copy. The frame was priced at `worst`, the plan's heaviest
    /// child (`Prices::heaviest_child`), so waiting is allowed only where those bytes and
    /// `next`'s own price fit under it — the peak the frame was refused against does not
    /// move.
    pub(super) fn may_wait(
        &self,
        waiting: &[Composite],
        next: &ChildOp,
        region: Region,
        worst: u64,
    ) -> bool {
        let Some(first) = waiting.first() else {
            return false;
        };
        if !next.isolated || next.group_alpha.is_some() || first.load == PassLoad::Clear {
            return false;
        }
        let Some(plan) = self.encoded.layers.get(next.layer) else {
            return false;
        };
        let Some(onto) = region.meet(self.prices.region(plan)) else {
            return false;
        };
        if waiting
            .iter()
            .any(|composite| composite.onto.meet(onto).is_some())
        {
            return false;
        }
        let held = waiting.iter().try_fold(0_u64, |held, composite| {
            composite.waits.map(|bytes| held.saturating_add(bytes))
        });
        held.is_some_and(|held| {
            held.saturating_add(self.prices.child(self.encoded, next.layer, region)) <= worst
        })
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

    /// Composites in a pass of their own, where no run of marks follows them onto the
    /// accumulator: a child comes next, whose copy must read what these draws wrote, or the
    /// plan ends. They are drawn in the order their children came, each under its own
    /// scissor; the first says what the pass loads, since each later one waited on an
    /// accumulator a recorded pass had written (ADR 1631).
    pub(super) fn composite_pass(
        &mut self,
        recorder: &mut wgpu::CommandEncoder,
        accumulator: &wgpu::TextureView,
        composites: Vec<Composite>,
    ) {
        let Some(load) = composites.first().map(Composite::load) else {
            return;
        };
        let stamp = self.pass_stamp();
        let mut pass = recorder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("raster composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: accumulator,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: load.op(),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: stamp,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        for composite in &composites {
            self.draw_composite(&mut pass, composite);
        }
        drop(pass);
        for composite in composites {
            self.release_composite(composite);
        }
    }
}

/// The backdrop's transfer: `onto`, read from the accumulator at its offset inside `region`,
/// written at the copy's origin (ADR 1630).
///
/// `onto` is `region ∩ child`, so it lies inside the accumulator; the copy was acquired at
/// `onto`'s size; both are layer textures of one format with `LAYER_USAGES`; and they are two
/// textures. So every precondition `copy_texture_to_texture` validates holds by
/// construction, and the call cannot raise the validation error that would be a panic.
fn transfer_backdrop(
    recorder: &mut wgpu::CommandEncoder,
    accumulator: &wgpu::Texture,
    region: Region,
    copy: (&wgpu::Texture, Region),
) {
    let (copy, onto) = copy;
    recorder.copy_texture_to_texture(
        wgpu::TexelCopyTextureInfo {
            texture: accumulator,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: onto.x.saturating_sub(region.x),
                y: onto.y.saturating_sub(region.y),
                z: 0,
            },
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyTextureInfo {
            texture: copy,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::Extent3d {
            width: onto.width,
            height: onto.height,
            depth_or_array_layers: 1,
        },
    );
}
