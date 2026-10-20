//! A filled path: the paint it resolves to, the overprint wrapper and the emission of the fill
//! itself (ISO 32000-2 §8.5.3, §8.7).

use std::sync::Arc;

use pdf_render::{BlendMode, ClipId, FillRule, Paint, Path, SoftMaskId, Transform};
use raster_scene::SceneBuilder;

use crate::QuorraRasterError;

use super::{Admitted, Encoder, ShadedPaint, blend_mode, colour, compose, fill_rule};

impl Encoder<'_> {
    pub(super) fn fill(
        &mut self,
        builder: &mut SceneBuilder,
        path: &Arc<Path>,
        transform: Transform,
        rule: FillRule,
        paint: &Paint,
        (clip, mask, blend): (Option<ClipId>, Option<SoftMaskId>, BlendMode),
    ) -> Result<(), QuorraRasterError> {
        if path.is_empty() {
            return Ok(());
        }
        // ISO 32000-2 §8.3.4 NOTE 3, the same refusal `crate::stroke::encode` takes and for the
        // same reason: a matrix with no inverse carries this path onto a line or a point, so the
        // mark is refused and the page is drawn. Stated in `pdf-render` so that all three
        // backends refuse the same marks (viewer trap 2, ADR 0482).
        if pdf_render::paint_space(transform).is_none() {
            return Ok(());
        }
        // A fill marks exactly its own path, so its page-space box is a bound on the pixels it
        // reaches — **unless** a subpath of it collapsed, in which case §10.7.4 gives that
        // subpath a mark one device pixel thick, which is wider than the path and a width this
        // view decides. [`Encoder::cuts_nothing`] takes `None` for that case and keeps the clip.
        let marks = (!path.collapses())
            .then(|| path.bounds(transform))
            .flatten();
        let Admitted::Chain(clip) = self.clip_chain(builder, clip, marks)? else {
            return Ok(());
        };
        let mask = self.mask_id(builder, mask)?;
        // ISO 32000-2 §10.7.4: no shape may disappear, and a subpath with no extent
        // along one axis has zero area for every coverage rasteriser. Until ADR 0703
        // this crate split such subpaths out here through
        // `pdf_render::split_collapsed_fill` — marks sized and floored on *this*
        // view's pixel grid, so a scene holding them was true at exactly one
        // placement, and one hairline ruling cost a page-space scene its survival
        // (ADR 0702's measured finding). Since raster's ADR 0086 the collapse table
        // is resident on the outline and raster's encode places the marks per
        // viewport, mirrored statement for statement from `pdf_render::collapsed` —
        // which remains the oracle's own path, so the cross-backend gates compare
        // the two implementations continuously. The whole original path crosses the
        // boundary, which is also what keeps its `Arc` cache identity.
        let outline = self.outline(path)?;
        self.emit_fill(
            builder,
            (outline, transform, rule),
            paint,
            (clip, mask, blend),
            path,
        )
    }

    /// One fill, geometry already uploaded: the paint decides the lane.
    ///
    /// `shape` is the fill's **whole** path, before §10.7.4's collapsed-subpath split, and it is a
    /// path rather than the rectangle it used to be for a measured reason. The only thing that
    /// rectangle is wanted for is the device window a radial cone is evaluated over — a question a
    /// solid colour never asks — and taking it eagerly in [`Self::fill`] meant every fill paid
    /// [`Path::bounds`], which on its first call walks the whole path to memoise a hull. On the
    /// project owner's drawing that is 58 003 fills, none of them a shading, and one walk of
    /// 3.0 M path segments for an answer nothing reads.
    ///
    /// **Asking inside the one branch that reads it took 267.3 M instructions off an open plus
    /// two frames — 0.99 % of the whole program — and 20.5 % off the scene phase of a zoom frame**
    /// (callgrind, and minima of ten interleaved rounds on the owner's own adapter; ADR 0387).
    /// The pixels cannot move: the value's only consumer is [`Self::radial_cone`], which now
    /// receives exactly what it received before.
    ///
    /// It stays the *whole* path rather than the split pieces so that the three calls in
    /// [`Self::fill`] still share one conservative bound instead of measuring three.
    fn emit_fill(
        &mut self,
        builder: &mut SceneBuilder,
        (outline, transform, rule): (raster_scene::OutlineId, Transform, FillRule),
        paint: &Paint,
        (clip, mask, blend): (
            Option<raster_scene::ClipId>,
            Option<raster_scene::MaskId>,
            BlendMode,
        ),
        shape: &Path,
    ) -> Result<(), QuorraRasterError> {
        // §8.7.4.5.4's cone, where the clause's "greatest value of s" can be a root the
        // shading's own `/Extend` refuses and the answer is the other one. No two-point
        // conical gradient expresses that, so all three backends leave their gradient and
        // draw `pdf_render::RadialRaster`'s bytes — identical bytes, which is the point.
        // Only a *fill* takes this door, exactly as in the sibling backends: a stroke's
        // outline is not the shape raster is handed, and no corpus document strokes a cone.
        // ISO 32000-2 §8.7.4.3 Table 77's `/Background`: a colour the shading answers outside
        // its own bounds, which no raster shading kind expresses. §11.6.7 makes the wash and
        // the shading one painting operation — the pattern's implicit group is "filled with the
        // specified background colour before the sh operator is invoked" — so both are
        // evaluated into one device-resolution raster by `pdf_render::ShadingRaster` and drawn
        // through raster's mesh paint, which is precisely "an RGBA raster already at device
        // resolution, placed at (left, top)". Identical bytes to the sibling backends, and the
        // door is asked before the cone's because it holds for every shading kind.
        if let Paint::Shading(shading) = paint
            && shading.background.is_some()
        {
            let within = self.device_pixels(shape, transform);
            let Some(id) = self.background_raster(shading, within)? else {
                // A shape covering no device pixel. Answered all the same: falling through
                // would ask `shading_paint` for a paint that cannot carry the wash.
                return Ok(());
            };
            return builder
                .fill(
                    outline,
                    self.placed(transform),
                    fill_rule(rule),
                    raster_scene::Paint::Mesh(id),
                    clip,
                    blend_mode(blend),
                    compose(blend),
                    mask,
                )
                .map_err(Into::into);
        }
        if let Paint::Shading(shading) = paint
            && let Some(cone) = self.radial_cone(shading, self.device_pixels(shape, transform))?
        {
            return builder
                .fill(
                    outline,
                    self.placed(transform),
                    fill_rule(rule),
                    raster_scene::Paint::Mesh(cone),
                    clip,
                    blend_mode(blend),
                    compose(blend),
                    mask,
                )
                .map_err(Into::into);
        }
        let paint = match paint {
            Paint::Solid(c) => raster_scene::Paint::Solid(colour(*c)),
            Paint::Shading(shading) => match self.shading_paint(shading)? {
                ShadedPaint::Ready(paint) => paint,
                // A sampled shading is not a raster paint: it draws as an image
                // clipped to this fill's path instead.
                ShadedPaint::Sampled => {
                    return self.sampled_fill(
                        builder,
                        (outline, transform, rule),
                        shading,
                        (clip, mask, blend),
                    );
                }
                ShadedPaint::Nothing => return Ok(()),
            },
            other => {
                return Err(QuorraRasterError::Unsupported(format!("paint {other:?}")));
            }
        };
        builder.fill(
            outline,
            self.placed(transform),
            fill_rule(rule),
            paint,
            clip,
            blend_mode(blend),
            compose(blend),
            mask,
        )?;
        Ok(())
    }
}
