//! An image on the scene: its upload (cached by the `Arc`'s identity), the filter its placement
//! asks for, and the device pixels it covers (ISO 32000-2 §8.9.5).

use pdf_render::{BlendMode, ClipId, Image, ImageSource, Path, Point, Rect, SoftMaskId, Transform};
use raster_scene::SceneBuilder;

use crate::QuorraRasterError;

use super::{Admitted, Encoder, blend_mode, overprinted, spec};

/// The factor pair that stands for an image uploaded whole, as its own key in the
/// resource cache: one source sample per output sample, in both axes.
const WHOLE: (u32, u32) = (1, 1);

/// The page-space box of the unit square an image is drawn on, under `transform`.
///
/// ISO 32000-2 §8.9.5.2 states the placement that way — the samples fill the unit square in
/// the space `transform` maps from — so the four mapped corners bound the mark whatever the
/// matrix does to them.
fn unit_square(transform: Transform) -> Rect {
    let corners = [
        transform.apply(Point::new(0.0, 0.0)),
        transform.apply(Point::new(1.0, 0.0)),
        transform.apply(Point::new(1.0, 1.0)),
        transform.apply(Point::new(0.0, 1.0)),
    ];
    let mut bounds = Rect::from_corners(corners[0], corners[0]);
    for corner in corners {
        bounds = bounds.union(Rect::from_corners(corner, corner));
    }
    bounds
}

impl Encoder<'_> {
    pub(super) fn image(
        &mut self,
        builder: &mut SceneBuilder,
        source: &ImageSource,
        transform: Transform,
        alpha: f32,
        (clip, mask, blend): (Option<ClipId>, Option<SoftMaskId>, BlendMode),
    ) -> Result<(), QuorraRasterError> {
        // ISO 32000-2 §8.3.4 NOTE 3, the same refusal a fill and a stroke take: a matrix with no
        // inverse carries the unit square this image is drawn on onto a line or a point, so the
        // mark is refused and the page is drawn (ADR 0482).
        if pdf_render::paint_space(transform).is_none() {
            return Ok(());
        }
        // §8.9.5.2 puts an image in the unit square and the transform carries the rest, so the
        // four mapped corners bound every sample it can reach.
        let marks = Some(unit_square(transform));
        let Admitted::Chain(clip) = self.clip_chain(builder, clip, marks)? else {
            return Ok(());
        };
        let mask = self.mask_id(builder, mask)?;
        let placement = transform.then(self.target.transform);
        let deferred = matches!(source, ImageSource::AtDeviceScale(_));
        // Since raster's ADR 0089 (this tree's ADR 0706), an ordinary image crosses
        // the boundary as its own samples plus §8.9.5.3's flag, and raster's encode
        // resolves the filter and the area-averaging reduction per placement —
        // mirrored statement for statement from `pdf_render`, whose originals the
        // oracle keeps. The command therefore reads nothing of the view, and a page
        // with pictures on it keeps its scene across a zoom (ADR 0702).
        if !deferred {
            let resolved = source.at(placement);
            let image: &Image = &resolved;
            let id = self.cached_image(image)?;
            let at = self.placed(transform);
            let filter = raster_scene::ImageFilter::Auto {
                interpolate: image.interpolate,
            };
            overprinted(builder, blend, self.knockout_element(), |builder| {
                builder.image(id, at, alpha, filter, clip, blend_mode(blend), mask)
            })?;
            return Ok(());
        }
        // Samples the display list deferred are produced here, where the device scale
        // is known — §11.6.5.2's mask on a grid of its own, at the grid `pdf_render`
        // derives from the placement so that no backend picks its own. Those stay
        // *transient*: the samples are made for this placement and their `Arc` is this
        // frame's, so there is no identity for a cache to be keyed by. The grid is
        // this view's, so the scene carries its answer (ADR 0702).
        self.consume_view();
        let resolved = source.at(placement);
        let image: &Image = &resolved;
        let reduced = image.area_averaged(placement);
        let uploaded: &Image = reduced.as_ref().unwrap_or(image);
        let uploaded_spec = spec(uploaded);
        let id = self.handing_over(|device| device.upload_image(&uploaded_spec))?;
        self.transient.push(id.into());
        let filter = if uploaded.is_smoothed(placement) {
            raster_scene::ImageFilter::Linear
        } else {
            raster_scene::ImageFilter::Nearest
        };
        let at = self.placed(transform);
        overprinted(builder, blend, self.knockout_element(), |builder| {
            builder.image(id, at, alpha, filter, clip, blend_mode(blend), mask)
        })?;
        Ok(())
    }

    /// The device pixels a path covers, clamped to the target.
    ///
    /// Half a pixel of margin on each side, because a pixel is sampled at its centre and a
    /// shape ending at x = 10.0 still covers the sample at 9.5 — `MeshRaster::build`'s own
    /// margin, and the same bound the sibling backends compute for a radial cone.
    ///
    /// **Not free, which is why [`Self::emit_fill`] asks it only where a cone will read it.**
    /// [`Path::bounds`] walks every control point on its first call for a path, and a display
    /// list of ordinary fills would otherwise pay that walk for a number none of them uses.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "each cast is between a device pixel index and its coordinate, both bounded \
                  by the target's extent"
    )]
    pub(super) fn device_pixels(&self, path: &Path, transform: Transform) -> (u32, u32, u32, u32) {
        let Some(bounds) = path.bounds(transform.then(self.target.transform)) else {
            return (0, 0, self.target.width, self.target.height);
        };
        (
            (bounds.min.x - 0.5).floor().max(0.0) as u32,
            (bounds.min.y - 0.5).floor().max(0.0) as u32,
            (bounds.max.x + 0.5)
                .ceil()
                .max(0.0)
                .min(self.target.width as f32) as u32,
            (bounds.max.y + 0.5)
                .ceil()
                .max(0.0)
                .min(self.target.height as f32) as u32,
        )
    }

    fn cached_image(&mut self, image: &Image) -> Result<raster_scene::ImageId, QuorraRasterError> {
        if let Some(id) = self.caches.image(&image.data, WHOLE) {
            return Ok(id);
        }
        let whole = spec(image);
        let id = self.handing_over(|device| device.upload_image(&whole))?;
        self.caches.store_image(&image.data, WHOLE, id);
        Ok(id)
    }
}
