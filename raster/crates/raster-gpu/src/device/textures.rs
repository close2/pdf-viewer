//! The textures a device makes that carry no encode of their own: a frame-internal
//! attachment, the 1×1 white stand-in for an absent source, and the RGBA8 texture a
//! resident paint becomes — premultiplied for an image, straight for a ramp or a mesh.
//!
//! One module because all three are the same decision made three times — a
//! `TextureDescriptor` carrying exactly the usages its consumer needs and no more, so
//! that a texture nothing draws into never asks for `RENDER_ATTACHMENT` and a texture
//! nothing writes never asks for `COPY_DST`. *When* one of them is needed is decided
//! elsewhere: `super::staging` for the frame's own sheets, `super::resident` for the
//! paints, `super::record` for the stand-in a pass binds where there is nothing to
//! bind.
//!
//! The frame's scratch coverage sheet and the glyph atlas are deliberately not here.
//! Both are a texture *and* the bytes a frame just produced for it, and separating the
//! descriptor from the upload it exists for would leave neither readable; they live in
//! `super::staging` beside the counts that size them.

use std::borrow::Cow;

use super::Device;

/// An image's straight-alpha samples, premultiplied for the texture the image lane filters.
///
/// **The hardware sampler interpolates whatever the texels hold**, and a linear filter over
/// straight alpha weights a transparent texel's colour exactly as much as an opaque one's —
/// so the colour an encoder left under an `/SMask`'s zeros, or the black a fully transparent
/// area-averaged cell carries, reaches the page along every edge the mask draws. Filtering
/// premultiplied samples gives a transparent texel no colour to lend. It is the caller's
/// oracle's order as well: `render-cpu` premultiplies each sample into a `tiny-skia` pixmap
/// before its bilinear filter reads it, with the same round-to-nearest, which is what makes
/// the two backends' reduced images agree (this tree's ADR 1287).
///
/// Borrowed where every sample is opaque, which premultiplying leaves unchanged.
pub(super) fn premultiplied(data: &[u8]) -> Cow<'_, [u8]> {
    if data.chunks_exact(4).all(|sample| sample[3] == u8::MAX) {
        return Cow::Borrowed(data);
    }
    let mut out = data.to_vec();
    for sample in out.chunks_exact_mut(4) {
        let alpha = u16::from(sample[3]);
        for component in &mut sample[..3] {
            let scaled = u16::from(*component)
                .saturating_mul(alpha)
                .saturating_add(127);
            *component = u8::try_from(scaled / 255).unwrap_or(u8::MAX);
        }
    }
    Cow::Owned(out)
}

impl Device {
    /// A frame-internal texture: layer, mask, or ping-pong scratch.
    pub(crate) fn create_internal_texture(
        &self,
        label: &str,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> wgpu::Texture {
        self.gpu.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }

    /// One RGBA8 texture, uploaded whole, holding whichever alpha form its caller made.
    pub(super) fn rgba_texture(
        &self,
        label: &str,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = self.gpu.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width.saturating_mul(4)),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    /// The 1×1 stand-in for absent coverage sources and masks: **white**, so an
    /// absent soft mask admits everything.
    pub(super) fn ensure_dummy(&mut self) -> wgpu::TextureView {
        if self.dummy_texture.is_none() {
            let texture = self.gpu.create_texture(&wgpu::TextureDescriptor {
                label: Some("raster dummy white"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &[255],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(1),
                    rows_per_image: None,
                },
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
            self.dummy_texture = Some(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        }
        // Just created above when absent.
        #[expect(clippy::expect_used)]
        self.dummy_texture.clone().expect("created above")
    }
}
