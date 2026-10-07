//! The textures a device makes that carry no encode of their own: a frame-internal
//! attachment, the 1×1 white stand-in for an absent source, and the RGBA8 texture a
//! resident paint becomes — premultiplied for an image, straight for a ramp or a mesh.
//!
//! **An image's texture is filled where a frame samples it, not whole** (ADR 1493). It is
//! made at the full size of its grid — the image's own or a reduction's — so every texel
//! coordinate the shader computes is the one a whole texture would be asked, and
//! [`FillRecord`] says which squares of it hold their bytes. A zoom step onto a
//! photograph showed a fifth of it and uploaded all 80 MB of it; it now uploads what the
//! window shows and the squares around it.
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

use super::Device;

/// Texels on a side of one square of a [`FillRecord`].
///
/// The margin a scroll finds already filled, and the granularity of the record: at 256 a
/// photograph of 5 280 × 3 792 samples is 315 squares, and the row of squares a window's edge
/// cuts is at most 255 texels of work the frame did not strictly need.
const FILL_SQUARE: u32 = 256;

/// Texels outside the sampled footprint a frame may still read: the linear filter's
/// neighbour (half a texel either side of the point, so one texel), and one more for the
/// shader's `f32` evaluation of the transform against the `f64` one here.
const FILL_MARGIN: f64 = 2.0;

/// A rectangle of a texture's texels, `x0..x1` × `y0..y1`, non-empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TexelRect {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

impl TexelRect {
    /// Texels across.
    pub(crate) fn width(self) -> u32 {
        self.x1.saturating_sub(self.x0)
    }

    /// Texels down.
    pub(crate) fn height(self) -> u32 {
        self.y1.saturating_sub(self.y0)
    }

    /// The whole of a `width` × `height` grid.
    pub(crate) fn whole(width: u32, height: u32) -> Self {
        Self {
            x0: 0,
            y0: 0,
            x1: width.max(1),
            y1: height.max(1),
        }
    }

    /// The texels an image op can read: `dest`'s four corners carried into texel space by
    /// the op's device → texel transform (`image.wgsl`'s `to_texel`, §8.3.3's coefficient
    /// order), bounded, widened by [`FILL_MARGIN`] and clamped into the grid — clamped the way
    /// the shader clamps a coordinate, so a footprint past an edge still names the edge's
    /// texels. A transform that is not finite names the whole grid.
    ///
    /// Every pixel the quad shades has its centre inside `dest`, and the transform is affine,
    /// so the centre's texel coordinate lies inside the corners' bounding box.
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped into the grid first
    pub(crate) fn sampled(texel: &[f32; 6], dest: [f32; 4], width: u32, height: u32) -> Self {
        let to_texel = texel.map(f64::from);
        let mut bounds = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for (x, y) in [
            (dest[0], dest[1]),
            (dest[2], dest[1]),
            (dest[0], dest[3]),
            (dest[2], dest[3]),
        ] {
            let (x, y) = (f64::from(x), f64::from(y));
            let s = to_texel[0] * x + to_texel[2] * y + to_texel[4];
            let t = to_texel[1] * x + to_texel[3] * y + to_texel[5];
            bounds = [
                bounds[0].min(s),
                bounds[1].min(t),
                bounds[2].max(s),
                bounds[3].max(t),
            ];
        }
        if !bounds.iter().all(|edge| edge.is_finite()) {
            return Self::whole(width, height);
        }
        let span = |low: f64, high: f64, size: u32| {
            let size = f64::from(size.max(1));
            let start = (low - FILL_MARGIN).floor().clamp(0.0, size - 1.0);
            let end = (high + FILL_MARGIN)
                .ceil()
                .clamp(1.0, size)
                .max(start + 1.0);
            (start as u32, end as u32)
        };
        let (x0, x1) = span(bounds[0], bounds[2], width);
        let (y0, y1) = span(bounds[1], bounds[3], height);
        Self { x0, y0, x1, y1 }
    }
}

/// An image's texture and the record of which of its squares hold their bytes.
#[derive(Debug)]
pub(crate) struct PaintTexture {
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) record: FillRecord,
}

/// Which [`FILL_SQUARE`] squares of a `width` × `height` grid hold their bytes.
#[derive(Debug)]
pub(crate) struct FillRecord {
    width: u32,
    height: u32,
    /// Squares across one row of the record.
    across: u32,
    /// One flag per square, row-major: whether its texels are written.
    filled: Vec<bool>,
}

impl FillRecord {
    /// A grid none of whose squares is written.
    pub(crate) fn new(width: u32, height: u32) -> Self {
        let across = width.div_ceil(FILL_SQUARE);
        let down = height.div_ceil(FILL_SQUARE);
        Self {
            width,
            height,
            across,
            filled: vec![false; (across as usize).saturating_mul(down as usize)],
        }
    }

    /// Marks every square `rect` meets as filled, and hands back the texels the caller now
    /// owes for the squares that were not: one rectangle per run of unfilled squares along a
    /// row of the record, a run continued down the rows below it where they ask the same
    /// columns — so a first frame owes one rectangle, and a scroll owes the strip it uncovered.
    pub(crate) fn claim(&mut self, rect: TexelRect) -> Vec<TexelRect> {
        let to_texels = |first: u32, past: u32, size: u32| {
            (
                first.saturating_mul(FILL_SQUARE),
                past.saturating_mul(FILL_SQUARE).min(size),
            )
        };
        let (left, right) = (rect.x0 / FILL_SQUARE, rect.x1.div_ceil(FILL_SQUARE));
        let (top, bottom) = (rect.y0 / FILL_SQUARE, rect.y1.div_ceil(FILL_SQUARE));
        let mut owed: Vec<TexelRect> = Vec::new();
        let across = self.across as usize;
        for row in top..bottom {
            let mut column = left;
            while column < right {
                let start = column;
                // Each unfilled square is marked as the run passes it; the run ends at the
                // first square already filled, or at the rectangle's edge.
                while column < right {
                    let index = (row as usize)
                        .saturating_mul(across)
                        .saturating_add(column as usize);
                    match self.filled.get_mut(index) {
                        Some(flag) if !*flag => *flag = true,
                        _ => break,
                    }
                    column = column.saturating_add(1);
                }
                if column == start {
                    column = column.saturating_add(1);
                    continue;
                }
                let (x0, x1) = to_texels(start, column, self.width);
                let (y0, y1) = to_texels(row, row.saturating_add(1), self.height);
                match owed
                    .iter_mut()
                    .find(|run| run.x0 == x0 && run.x1 == x1 && run.y1 == y0)
                {
                    Some(run) => run.y1 = y1,
                    None => owed.push(TexelRect { x0, y0, x1, y1 }),
                }
            }
        }
        owed
    }
}

/// An image's texture at the full size of its grid, with nothing written yet: what
/// [`FillRecord::claim`] hands out is written by [`write_texels`].
pub(super) fn paint_texture(
    gpu: &wgpu::Device,
    label: &str,
    width: u32,
    height: u32,
) -> PaintTexture {
    let texture = gpu.create_texture(&wgpu::TextureDescriptor {
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
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    PaintTexture {
        texture,
        view,
        record: FillRecord::new(width, height),
    }
}

/// Writes `rect` of `texture` from `data`, whose rows are `stride` bytes apart and whose
/// first byte is the rectangle's top-left texel — so a region of a resident image is
/// written straight out of the image's own bytes, with no copy on this side.
pub(super) fn write_texels(
    queue: &wgpu::Queue,
    texture: &PaintTexture,
    rect: TexelRect,
    data: &[u8],
    stride: u32,
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture.texture,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: rect.x0,
                y: rect.y0,
                z: 0,
            },
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(stride),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: rect.width(),
            height: rect.height(),
            depth_or_array_layers: 1,
        },
    );
}

/// Writes `rect` of an image's own samples into its texture, premultiplied, and returns the
/// bytes written. Where every sample of the rectangle is opaque, which premultiplying leaves
/// unchanged, the rows are written out of the image's bytes where they lie; otherwise the
/// rectangle is copied out and premultiplied in the copy.
#[expect(clippy::arithmetic_side_effects)] // `rect` lies inside the image's grid (its claim was
// clamped into it), so every offset is below the length of bytes the image already holds
pub(super) fn write_samples(
    queue: &wgpu::Queue,
    texture: &PaintTexture,
    image: &raster_scene::ImageSpec,
    rect: TexelRect,
) -> u64 {
    let line = (image.width as usize).saturating_mul(4);
    let (left, right) = ((rect.x0 as usize) * 4, (rect.x1 as usize) * 4);
    let rows = || {
        (rect.y0 as usize..rect.y1 as usize)
            .filter_map(|y| image.data.get(y * line + left..y * line + right))
    };
    let written = (right - left).saturating_mul(rect.height() as usize) as u64;
    if rows().all(opaque) {
        let first = (rect.y0 as usize) * line + left;
        if let Some(data) = image.data.get(first..) {
            write_texels(queue, texture, rect, data, image.width.saturating_mul(4));
            return written;
        }
    }
    let mut copy: Vec<u8> = rows().flatten().copied().collect();
    premultiply_in_place(&mut copy);
    write_texels(queue, texture, rect, &copy, rect.width().saturating_mul(4));
    written
}

/// Whether every sample of a run of RGBA bytes is opaque. Asked of 64 samples at a time —
/// their alphas folded with `&` and the block tested once — because a test per sample stays a
/// scalar loop over a photograph's every pixel, where the fold is vector instructions and the
/// same answer (this tree's ADR 1433).
pub(super) fn opaque(data: &[u8]) -> bool {
    let block = |samples: &[u8]| {
        samples
            .chunks_exact(4)
            .fold(u8::MAX, |alphas, sample| alphas & sample[3])
            == u8::MAX
    };
    let mut blocks = data.chunks_exact(256);
    blocks.by_ref().all(block) && block(blocks.remainder())
}

/// An image's straight-alpha samples premultiplied, where they lie, for the texture the image
/// lane filters.
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
/// Its callers skip it where [`opaque`] says every sample is, which premultiplying leaves
/// unchanged.
pub(super) fn premultiply_in_place(data: &mut [u8]) {
    for sample in data.chunks_exact_mut(4) {
        let alpha = u16::from(sample[3]);
        for component in &mut sample[..3] {
            let scaled = u16::from(*component)
                .saturating_mul(alpha)
                .saturating_add(127);
            *component = u8::try_from(scaled / 255).unwrap_or(u8::MAX);
        }
    }
}

impl Device {
    /// A frame-internal texture: a layer, or a soft mask's reduction, with the usages its
    /// caller names — both are drawn into and sampled, and only a layer is a transfer's
    /// source or destination (`layers::LAYER_USAGES`, ADR 1630).
    pub(crate) fn create_internal_texture(
        &self,
        label: &str,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
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
            usage,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A footprint is carried into texel space, widened by the margin and clamped into the
    /// grid; a footprint wholly past an edge names that edge's texels, as the shader's clamp
    /// reads them; a transform that is not finite names the whole grid.
    #[test]
    fn a_footprint_names_the_texels_it_can_read() {
        // A 100 × 80 image one texel a pixel, its top-left at device (10, 10).
        let texel = [1.0, 0.0, 0.0, 1.0, -10.0, -10.0];
        assert_eq!(
            TexelRect::sampled(&texel, [30.0, 20.0, 60.0, 40.0], 100, 80),
            TexelRect {
                x0: 18,
                y0: 8,
                x1: 52,
                y1: 32
            }
        );
        assert_eq!(
            TexelRect::sampled(&texel, [0.0, 0.0, 300.0, 300.0], 100, 80),
            TexelRect::whole(100, 80)
        );
        assert_eq!(
            TexelRect::sampled(&texel, [200.0, 20.0, 260.0, 40.0], 100, 80),
            TexelRect {
                x0: 99,
                y0: 8,
                x1: 100,
                y1: 32
            }
        );
        assert_eq!(
            TexelRect::sampled(&[f32::NAN; 6], [0.0, 0.0, 1.0, 1.0], 100, 80),
            TexelRect::whole(100, 80)
        );
    }

    /// A claim owes the unfilled squares it meets, a run along a row continued down the rows
    /// that ask the same columns, and owes nothing for a square already claimed.
    #[test]
    fn a_claim_owes_only_what_no_earlier_claim_filled() {
        let mut texture = FillRecord::new(1000, 600);
        let first = TexelRect {
            x0: 0,
            y0: 0,
            x1: 300,
            y1: 300,
        };
        assert_eq!(
            texture.claim(first),
            vec![TexelRect {
                x0: 0,
                y0: 0,
                x1: 512,
                y1: 512
            }]
        );
        assert_eq!(texture.claim(first), Vec::new());
        assert_eq!(
            texture.claim(TexelRect::whole(1000, 600)),
            vec![
                TexelRect {
                    x0: 512,
                    y0: 0,
                    x1: 1000,
                    y1: 512
                },
                TexelRect {
                    x0: 0,
                    y0: 512,
                    x1: 1000,
                    y1: 600
                },
            ]
        );
        assert_eq!(texture.claim(TexelRect::whole(1000, 600)), Vec::new());
    }

    /// The premultiplication is the caller's round-to-nearest at every component and alpha:
    /// all 65 536 pairs against `⌊(c·a + 127) ÷ 255⌋` computed the slow way.
    #[test]
    fn premultiplying_is_round_to_nearest_at_every_pair() {
        for alpha in 0..=255u8 {
            let mut data: Vec<u8> = (0..=255u8).flat_map(|c| [c, c, c, alpha]).collect();
            premultiply_in_place(&mut data);
            for (c, sample) in (0..=255u16).zip(data.chunks_exact(4)) {
                let expected = u8::try_from((c * u16::from(alpha) + 127) / 255).unwrap_or(0);
                assert_eq!(
                    sample,
                    [expected, expected, expected, alpha],
                    "{c} at {alpha}"
                );
            }
        }
    }
}
