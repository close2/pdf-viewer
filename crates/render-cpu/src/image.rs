//! The image kernel — ISO 32000-2 §8.9.5: an image's samples mapped onto the unit square and drawn
//! through a pattern shader.
//!
//! The reduction of a large image before it is filtered is `crate::reduced_images`'s, shared by
//! every strip and every redraw.

use pdf_render::Transform;

use crate::compose::premultiply;
use crate::surface::ToDevice;
use crate::{CpuRasterError, CpuRasterizer, HIGH_PRECISION_PIPELINE, convert, scan};

mod centres;

/// Where and how an image is placed.
///
/// Grouped because these four always travel together, and passing them separately made
/// the call site a row of unlabelled arguments.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ImagePlacement {
    pub(crate) transform: Transform,
    pub(crate) alpha: f32,
    pub(crate) blend: tiny_skia::BlendMode,
    pub(crate) to_device: ToDevice,
}

impl CpuRasterizer {
    /// Draws an image mapped onto the unit square.
    ///
    /// `tiny-skia` has no image primitive, so this fills the unit square with a pattern
    /// shader whose own transform maps image pixels onto it. The composed transform is:
    /// scale by `1/width` and `1/height` to reach the unit square, flip vertically because
    /// PDF's y-up space puts the image's *first* row at the top, then the command's
    /// transform, then the device transform.
    ///
    /// Whether the samples are filtered is [`pdf_render::Image::is_smoothed`]'s decision,
    /// from §8.9.5.3's `/Interpolate` and how large the image is being drawn: bilinear where
    /// a page is viewed at less than full size, which is the common case and where
    /// nearest-neighbour would alias badly, and nearest where the document has magnified a
    /// few samples across an area and asked for no smoothing.
    ///
    /// Deep reductions are handled before that, by [`pdf_render::Image::area_averaged`],
    /// which leaves this filter under a two-fold shrink where four taps see every sample.
    pub(crate) fn draw_image(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        source: &pdf_render::ImageSource,
        placement: ImagePlacement,
        clip: scan::Clip<'_>,
    ) -> Result<(), CpuRasterError> {
        let ImagePlacement {
            transform,
            alpha,
            blend,
            to_device,
        } = placement;
        // ISO 32000-2 §8.3.4 NOTE 3, the same refusal a fill and a stroke take: a matrix with no
        // inverse carries the unit square this image is drawn on onto a line or a point, so the
        // mark is refused and the page is drawn (ADR 0482). Before the samples are resolved,
        // because their grid is derived from a placement that states no scale.
        if pdf_render::paint_space(transform).is_none() {
            return Ok(());
        }
        let placement = to_device.of(transform);
        let placement_of_image = centres::Placement {
            mark: transform,
            to_device,
        };
        // Where a command's samples do not exist until the device scale does — §11.6.5.2's
        // soft mask on a grid of its own is the case — this is where they are produced, and
        // `pdf_render` decides the grid so that the three backends cannot ask for different
        // ones. An ordinary image borrows and costs nothing.
        let resolved = source.at(placement);
        let image: &pdf_render::Image = &resolved;
        if !image.is_consistent() {
            return Err(CpuRasterError::InvalidImage {
                width: image.width,
                height: image.height,
                bytes: image.data.len(),
            });
        }

        // Blocks of samples that would share one device pixel are averaged before
        // `tiny-skia` sees them, because its bilinear filter reads four neighbours whatever
        // the reduction and an eleven-fold shrink never looks at most of the source. The
        // decision is `pdf_render`'s so that every backend makes it identically; ADR 0025 has
        // why it is a departure from §10.7.4 rather than a reading of it.
        //
        // Through the memo rather than straight to `area_averaged`, because the work is per
        // *source* sample and this function is called once per strip and once per redraw:
        // `reduced_images` has the measurement and the key.
        let reduced = self.reduced_images.reduced(image, placement);
        let image = reduced.as_ref().unwrap_or(image);

        // `tiny-skia` pixmaps are premultiplied; `Image` is documented as straight alpha,
        // so the conversion happens here at the boundary.
        let mut samples = tiny_skia::Pixmap::new(image.width, image.height).ok_or(
            CpuRasterError::Allocation {
                width: image.width,
                height: image.height,
            },
        )?;
        for (target, source) in samples
            .pixels_mut()
            .iter_mut()
            .zip(image.data.chunks_exact(4))
        {
            let a = source[3];
            // `from_rgba` rejects a channel exceeding its alpha, which `premultiply`
            // cannot produce; a fully transparent pixel is the safe fallback if it ever
            // does, since it changes nothing on the page.
            *target = tiny_skia::PremultipliedColorU8::from_rgba(
                premultiply(source[0], a),
                premultiply(source[1], a),
                premultiply(source[2], a),
                a,
            )
            .unwrap_or(tiny_skia::PremultipliedColorU8::TRANSPARENT);
        }

        let width = f32::from(u16::try_from(image.width).unwrap_or(u16::MAX));
        let height = f32::from(u16::try_from(image.height).unwrap_or(u16::MAX));
        // Image space (pixels, y down) to the unit square (y up) — and no further. The
        // pattern is a paint, so `tiny-skia` reads its transform in the space of the path
        // being filled and applies the drawing transform itself; see
        // [`shading::shader`]. That path *is* the unit square, so this is the whole of it,
        // and composing the device transform in here as well would apply it twice.
        let to_unit = Transform::new(1.0 / width, 0.0, 0.0, -1.0 / height, 0.0, 1.0);

        // `tiny-skia` reserves the right to overrule this: `Pattern::push_stages` substitutes
        // `Nearest` for *any* pure-translation pattern transform, which is what an image drawn
        // at one device pixel per sample composes to. That used to be a silent override of a
        // filter this tree had asked for; it is not one now, because
        // `pdf_render::Image::is_smoothed` answers `false` at exactly that placement and for
        // §10.7.4's own reason. The two readings agree, so nothing here is overruled — and a
        // round that widens the rule again owes this sentence a second look.
        let filter = if image.is_smoothed(placement) {
            tiny_skia::FilterQuality::Bilinear
        } else {
            tiny_skia::FilterQuality::Nearest
        };

        // §10.7.4 colours an unsmoothed image's pixel by the sample its centre maps back to, and the
        // centre is the page's, so `centres` chooses the samples on the page's device grid (ADR
        // 2100).
        if filter == tiny_skia::FilterQuality::Nearest {
            let drawn = centres::Drawn {
                alpha,
                blend,
                anti_alias: self.anti_alias,
            };
            return centres::draw(pixmap, &samples, placement_of_image, drawn, clip);
        }

        let paint = tiny_skia::Paint {
            shader: tiny_skia::Pattern::new(
                samples.as_ref(),
                tiny_skia::SpreadMode::Pad,
                filter,
                alpha.clamp(0.0, 1.0),
                convert::transform(to_unit),
            ),
            blend_mode: blend,
            anti_alias: self.anti_alias,
            force_hq_pipeline: HIGH_PRECISION_PIPELINE,
            ..tiny_skia::Paint::default()
        };

        scan::fill(
            pixmap,
            &unit_square()?,
            &paint,
            tiny_skia::FillRule::Winding,
            convert::transform(placement),
            clip,
        );
        Ok(())
    }
}

/// The unit square, which a command's transform carries onto the page.
fn unit_square() -> Result<tiny_skia::Path, CpuRasterError> {
    let mut builder = tiny_skia::PathBuilder::new();
    builder.push_rect(tiny_skia::Rect::from_xywh(0.0, 0.0, 1.0, 1.0).ok_or(
        CpuRasterError::InvalidImage {
            width: 1,
            height: 1,
            bytes: 0,
        },
    )?);
    builder.finish().ok_or(CpuRasterError::InvalidPath)
}
