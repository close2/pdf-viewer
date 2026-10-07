//! Pixels moved, not changed: the three passes that run `blit.wgsl`.
//!
//! One shader, a `textureLoad` and a store with no blending, so between two
//! `Rgba8Unorm` textures every one of these is exact. What differs between them is only
//! *where* — a seed copy reads the parent at the parent's origin, the root reaches the
//! target through an origin that is negative because the root is smaller than the frame
//! (ADR 0039), and a damage patch writes one scissored rectangle at a time over
//! contents the target keeps (ADR 0012). A composite's backdrop is copied by a transfer
//! rather than by this shader (ADR 1630), and passes through here only where there is
//! nothing to copy and its copy is cleared.
//!
//! They are together because the offsets are the whole of what a reader has to check,
//! and they are checkable side by side.

use crate::error::RenderError;
use crate::pipeline::Kind;

use super::{Executor, Region};

/// A region's extent, as the shader's floats.
#[expect(clippy::cast_precision_loss)] // extents are exact in f32
fn extent(region: Region) -> [f32; 2] {
    [region.width as f32, region.height as f32]
}

/// The source origin a pass writing the whole target reads the root at: negative, because
/// the root's texel (0, 0) is the *device* pixel `region.x, region.y` (ADR 0039).
#[expect(clippy::cast_precision_loss)] // extents are exact in f32
fn from_root(region: Region) -> [f32; 2] {
    [-(region.x as f32), -(region.y as f32)]
}

impl Executor<'_> {
    /// One rectangle of one texture copied into another, whole and unchanged.
    ///
    /// Two callers. §11.4.4's **seed**: a non-isolated group's buffer begins as a copy of
    /// what is under it (ADR 0019), at the parent's own origin, read inside the source by
    /// construction; `src_region` is what tells the shader so. And the **backdrop** a
    /// composite is about to cover where nothing has written the accumulator yet, a
    /// source of `None` whose every texel is §11.4.5's transparency: the pass clears and
    /// draws nothing, which stores the zeros a copy would have read (ADR 1618). A backdrop
    /// that has a source is a transfer (`child.rs`, ADR 1630).
    ///
    /// The seed stays a blit, scissored by the same rule as every other pass, because no
    /// page this tree times seeds a group, so what a transfer would save there is
    /// unmeasured (ADR 1630 section 4). `blit.wgsl` is a `textureLoad` and a store with no
    /// blending, so between two `Rgba8Unorm` textures it is exact.
    pub(super) fn copy_pass(
        &mut self,
        recorder: &mut wgpu::CommandEncoder,
        label: &str,
        src: (Option<&wgpu::TextureView>, Region),
        into: (&wgpu::TextureView, Region),
        at: [f32; 2],
    ) -> Result<(), RenderError> {
        let (src, src_region) = src;
        let (into, into_region) = into;
        let blit = match src {
            Some(src) => {
                let bind = self.device.blit_bind(src, at, extent(src_region));
                let (pipeline, compiled) = self
                    .device
                    .pipelines()
                    .get(Kind::Blit, wgpu::TextureFormat::Rgba8Unorm)?;
                if let Some(duration) = compiled {
                    self.phases.push(("pipeline compile (first use)", duration));
                }
                Some((pipeline, bind))
            }
            None => None,
        };
        let stamp = self.pass_stamp();
        let mut pass = recorder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: into,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: stamp,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if let Some((pipeline, bind)) = blit {
            self.scissor_pass(&mut pass, into_region, None);
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.draw(0..3, 0..1);
        }
        Ok(())
    }

    /// Blit the finished root onto the frame's target.
    ///
    /// The root is as big as what the page marks (ADR 0039) and the target is the target,
    /// so this is the one copy whose destination is larger than its source: the shader
    /// reads at `p − root.origin` and writes transparency outside the root's rectangle,
    /// which is what a page rendered onto transparency (brief section 3) has there.
    pub(crate) fn blit_to_target(
        &mut self,
        recorder: &mut wgpu::CommandEncoder,
        src: &wgpu::TextureView,
        src_region: Region,
        target: &wgpu::TextureView,
        format: wgpu::TextureFormat,
    ) -> Result<(), RenderError> {
        let bind = self
            .device
            .blit_bind(src, from_root(src_region), extent(src_region));
        let (pipeline, compiled) = self.device.pipelines().get(Kind::Blit, format)?;
        if let Some(duration) = compiled {
            self.phases.push(("pipeline compile (first use)", duration));
        }
        let stamp = self.pass_stamp();
        let mut pass = recorder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("raster blit"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: stamp,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.draw(0..3, 0..1);
        Ok(())
    }

    /// Patch the finished root onto the target: one scissored REPLACE blit per
    /// damage rectangle, over the target's retained contents (`LoadOp::Load`).
    /// Nothing outside the rectangles is written — that is the whole contract.
    ///
    /// Inside a rectangle but outside the root, the blit writes transparency (ADR 0039):
    /// the contract is that a patched rectangle equals a full redraw, and a full redraw
    /// leaves transparency where the page marked nothing.
    pub(crate) fn patch_to_target(
        &mut self,
        recorder: &mut wgpu::CommandEncoder,
        src: &wgpu::TextureView,
        src_region: Region,
        target: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        rects: &[[u32; 4]],
    ) -> Result<(), RenderError> {
        let bind = self
            .device
            .blit_bind(src, from_root(src_region), extent(src_region));
        let (pipeline, compiled) = self.device.pipelines().get(Kind::Blit, format)?;
        if let Some(duration) = compiled {
            self.phases.push(("pipeline compile (first use)", duration));
        }
        let stamp = self.pass_stamp();
        let mut pass = recorder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("raster damage patch"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: stamp,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        for &[x, y, w, h] in rects {
            pass.set_scissor_rect(x, y, w, h);
            pass.draw(0..3, 0..1);
        }
        Ok(())
    }
}
