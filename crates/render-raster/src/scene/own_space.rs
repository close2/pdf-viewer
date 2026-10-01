//! A group, or a soft mask's group, whose result is resolved per pixel *after* it composites —
//! drawn as its own frame, resolved by the shared arithmetic, and placed back in the scene as
//! device pixels (ADR 1471).
//!
//! Two constructions of ISO 32000-2 end in a function of the composited pixel that no paint, blend
//! mode or mask kind of raster's scene states:
//!
//! - **A group compositing in a blending colour space of its own** (§11.6.6, §11.7.2). §11.7.2:
//!   "all blending and compositing computations shall be done in that space", and "[t]he resulting
//!   colours shall then be interpreted in the group's colour space when the group is subsequently
//!   composited with its backdrop". The interpretation is a curve (`CalGray`, a 'GRAY' profile), a
//!   cube (`CalRGB`, an 'RGB ' profile) or, for four components, §11.3.4's per-component pair
//!   resolved through a grid — each a function of the *composited* components.
//! - **A luminosity soft mask whose `Y` is a CIE-based space's own** (§11.5.3), of three components
//!   or of the four a pair of rasters carries between them, which raster's
//!   `MaskKind::Luminosity` cannot weigh: its shader takes §10.4.2.2's fixed coefficients.
//!
//! **What makes a frame of its own exact rather than an approximation is the isolation.** Every
//! such group is isolated — `pdf_render::GroupBlending` guarantees it, and a mask group is §11.6.5.1's
//! "transparency group … rendered on its own" — and §11.4.5 composites an isolated group's
//! elements onto "a fully transparent initial backdrop", so its result is a function of its own
//! elements alone and nothing the scene draws under it can change a pixel of it. Drawing those
//! elements on a separate frame is therefore drawing the same pixels, and the read-back is not a
//! read-back of a scene under composition, which raster's vocabulary rightly has no door for.
//!
//! The resolution is `pdf_render`'s own — [`pdf_render::resolve_blending`],
//! [`pdf_render::resolve_grey`], [`pdf_render::resolve_cube`] and
//! [`pdf_render::SoftMask::values`] — the functions the CPU backend runs over its own buffers, so
//! the two backends cannot answer the clause apart (trap 2). The result reaches the scene through
//! the one vocabulary raster has for pixels already at device resolution, `Paint::Mesh`, which
//! samples "at absolute device pixels": a fill of the whole target with that paint, inside a group
//! that carries the original's alpha, blend mode, clip and mask. The raster is this view's, so the
//! scene that carries it is marked as having read the view (ADR 0702).

use pdf_render::{BlendMode, GroupBlending, Path, PathCommand, Point, SoftMask};
use raster_scene::SceneBuilder;

use super::{Encoder, GroupParts, blend_mode, fill_rule};
use crate::QuorraRasterError;

impl Encoder<'_> {
    /// A group compositing in a blending colour space of its own (§11.6.6, §11.7.2), drawn as
    /// its own frame and placed back as device pixels.
    ///
    /// `clip` and `mask` are the group's own, already resolved onto `builder` by
    /// [`Encoder::group`], and `compose` the operator the finished group composites with,
    /// already resolved against the group's own blend mode.
    pub(super) fn group_in_own_space(
        &mut self,
        builder: &mut SceneBuilder,
        (parts, clip, mask): (
            GroupParts<'_>,
            Option<raster_scene::ClipId>,
            Option<raster_scene::MaskId>,
        ),
        compose: raster_scene::Compose,
        blending: &GroupBlending,
    ) -> Result<(), QuorraRasterError> {
        // The elements as §11.4.5 composites them: onto transparency, under the group's own
        // knockout flag, with none of what the group's *result* carries — alpha, blend mode,
        // clip and mask apply below, once, to the resolved pixels.
        let elements = GroupParts {
            commands: parts.commands,
            alpha: 1.0,
            blend: BlendMode::Normal,
            clip: None,
            mask: None,
            isolated: true,
            knockout: parts.knockout,
            blending: None,
        };
        let mut composited = self.frame_of(elements)?;
        crate::premultiply(&mut composited);
        match blending {
            GroupBlending::FourComponents { space, black } => {
                let mut ink = self.frame_of(GroupParts {
                    commands: black,
                    ..elements
                })?;
                crate::premultiply(&mut ink);
                pdf_render::resolve_blending(&mut composited, &ink, space);
            }
            GroupBlending::OneComponent { curve } => {
                pdf_render::resolve_grey(&mut composited, curve);
            }
            GroupBlending::ThreeComponents { cube } => {
                pdf_render::resolve_cube(&mut composited, cube);
            }
        }
        crate::demultiply(&mut composited);
        let spec = raster_scene::GroupSpec {
            alpha: parts.alpha,
            blend: blend_mode(parts.blend),
            clip,
            // The knockout flag governs how the elements composite with one another, which
            // happened on the frame above; what is left is one opaque-to-itself raster.
            knockout: false,
            mask,
            compose,
            isolated: true,
        };
        self.place_pixels(builder, composited, |body, fill| body.group(spec, fill))
    }

    /// A luminosity soft mask whose `Y` is the group's own space's (§11.5.3), drawn as its own
    /// frame — two where the space has four components — reduced to mask values by
    /// [`SoftMask::values`] or [`SoftMask::paired_values`], and handed to the scene as an alpha
    /// mask whose body is those values.
    ///
    /// §11.5.2's alpha rule over a raster whose alpha *is* the mask value is that value, and the
    /// transfer table has been applied by the shared reduction, so the scene's mask states none.
    pub(super) fn mask_in_own_space(
        &mut self,
        builder: &mut SceneBuilder,
        def: &SoftMask,
    ) -> Result<raster_scene::MaskId, QuorraRasterError> {
        let elements = |commands| GroupParts {
            commands,
            alpha: 1.0,
            blend: BlendMode::Normal,
            clip: None,
            mask: None,
            isolated: true,
            knockout: false,
            blending: None,
        };
        let chromatic = self.frame_of(elements(&def.commands))?;
        let values = match def.black.as_ref() {
            Some(half) => {
                let black = self.frame_of(elements(&half.commands))?;
                def.paired_values(&chromatic, &black)
            }
            None => def.values(&chromatic),
        };
        let pixels: Vec<u8> = values.iter().flat_map(|&value| [0, 0, 0, value]).collect();
        let mut mapped = None;
        self.place_pixels(builder, pixels, |body, fill| {
            mapped = Some(body.mask(raster_scene::MaskKind::Alpha, None, fill)?);
            Ok(())
        })?;
        // `place_pixels` calls its closure exactly once, so the id is always set here.
        mapped.ok_or_else(|| {
            QuorraRasterError::Unsupported(
                "a soft mask whose values the scene never received".to_owned(),
            )
        })
    }

    /// The elements `parts` names, drawn onto transparency as a frame of their own at this
    /// encoder's target, read back straight-alpha.
    ///
    /// A second encoder over the same device, list and caches, so every path a page's own
    /// commands take is the path these take; its transient resources join this frame's and are
    /// released with them.
    fn frame_of(&mut self, parts: GroupParts<'_>) -> Result<Vec<u8>, QuorraRasterError> {
        let mut builder = SceneBuilder::new();
        let target = self.target;
        let mut inner = Encoder::new(
            &mut *self.device,
            self.list,
            target,
            &mut *self.caches,
            &mut *self.transient,
            &mut *self.functions,
        );
        inner.group(&mut builder, parts, raster_scene::Compose::SrcOver)?;
        let scene = builder.finish();
        let viewport =
            raster_gpu::Viewport::full(target.width, target.height, raster_scene::Affine::IDENTITY);
        let frame = self
            .device
            .render(&scene, &viewport, raster_gpu::Target::Readback)?;
        Ok(frame.into_raster()?.into_pixels())
    }

    /// Uploads straight-alpha `pixels` covering this encoder's whole target as a device-pixel
    /// raster, and hands `wrap` a body that fills the target with it, for the caller to place
    /// inside a group or a mask.
    fn place_pixels(
        &mut self,
        builder: &mut SceneBuilder,
        pixels: Vec<u8>,
        wrap: impl FnOnce(
            &mut SceneBuilder,
            &mut dyn FnMut(&mut SceneBuilder) -> Result<(), raster_scene::SceneError>,
        ) -> Result<(), raster_scene::SceneError>,
    ) -> Result<(), QuorraRasterError> {
        self.consume_view(); // the raster is this view's pixels (ADR 0702)
        let target = self.target;
        let mesh = raster_scene::MeshSpec {
            left: 0,
            top: 0,
            image: raster_scene::ImageSpec {
                width: target.width,
                height: target.height,
                data: pixels.into(),
            },
        };
        let id = self.handing_over(|device| device.upload_mesh(&mesh))?;
        self.transient.push(id.into());
        // The target's own rectangle in device pixels, carried back into the space the scene is
        // built in: the placement then lands it on exactly the pixels the raster holds.
        let Some(to_page) = target.transform.invert() else {
            return Ok(()); // a target with no inverse shows nothing of the page
        };
        let (width, height) = (target_extent(target.width), target_extent(target.height));
        let mut rectangle = Path::new();
        rectangle.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
        rectangle.push(PathCommand::LineTo(Point::new(width, 0.0)));
        rectangle.push(PathCommand::LineTo(Point::new(width, height)));
        rectangle.push(PathCommand::LineTo(Point::new(0.0, height)));
        rectangle.push(PathCommand::Close);
        let outline = self.transient_outline(&rectangle)?;
        let at = self.placed(to_page);
        let rule = fill_rule(pdf_render::FillRule::NonZero);
        let mut fill = |body: &mut SceneBuilder| {
            body.fill(
                outline,
                at,
                rule,
                raster_scene::Paint::Mesh(id),
                None,
                raster_scene::BlendMode::Normal,
                raster_scene::Compose::SrcOver,
                None,
            )
        };
        wrap(builder, &mut fill)?;
        Ok(())
    }
}

/// A target extent as a device coordinate.
#[expect(
    clippy::cast_precision_loss,
    reason = "a target side is bounded by the adapter's texture limit, far inside f32's exact \
              integers"
)]
fn target_extent(pixels: u32) -> f32 {
    pixels as f32
}
