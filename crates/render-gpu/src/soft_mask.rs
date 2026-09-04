//! Evaluating a display list's soft masks on the GPU (ISO 32000-2 §11.5).
//!
//! A soft mask is a transparency group evaluated for its opacity rather than for its colour,
//! and the display list carries it as *commands* precisely because the group is evaluated at
//! device resolution — which only a backend knows. This module renders each of them with
//! Vello, reads the pixels back, and turns them into mask values with
//! [`pdf_render::SoftMask::value`], the same function the CPU backend calls.
//!
//! # Why a readback rather than a layer
//!
//! Vello can express *part* of this natively: a layer composited with `Compose::DestIn`
//! takes the alpha of what is drawn inside it, which is §11.5.2, and
//! `Scene::push_luminance_mask_layer` takes a luminance, which is nearly §11.5.3. Neither
//! covers the clause:
//!
//! - Vello's luminance is the SVG formula, `0.2125 R + 0.7154 G + 0.0721 B`, where §11.5.3
//!   EXAMPLE 2 gives `0.30 R + 0.59 G + 0.11 B` for a device space. On green artwork those
//!   differ by a fifth of the mask's range — and, worse, the *two backends* would then
//!   differ, which is the one thing this project's cross-backend comparison cannot absorb.
//! - §11.6.5.1's `/TR` is an arbitrary function of the mask value, and no blend mode is one.
//!
//! So the group is rendered, read back, and converted by the shared function. The cost is
//! one GPU round trip per mask on a page — the corpus's heaviest first page holds four —
//! and it buys exactly the same mask values the CPU backend computes. The alternative that
//! would remove it is a post-process shader of our own, which means owning a wgpu pipeline
//! beside Vello's; that is a larger change than this feature justified.

use std::collections::HashMap;

use pdf_render::{DisplayList, SoftMaskId, TargetSpec};
use vello::peniko;

use crate::GpuRasterError;

/// The mask rasters a display list's soft masks evaluate to, at one target size.
///
/// Indexed by [`SoftMaskId`], and tied to the target they were rendered for: a mask is
/// device-resolution by construction, so a raster built for one target says nothing about
/// another.
#[derive(Debug, Default)]
pub struct SoftMaskRasters {
    images: HashMap<SoftMaskId, peniko::ImageBrush>,
}

impl SoftMaskRasters {
    /// An empty set, for a list that carries no soft mask.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// Returns the image whose alpha channel holds a mask's values.
    pub(crate) fn get(&self, id: SoftMaskId) -> Result<&peniko::ImageBrush, GpuRasterError> {
        self.images
            .get(&id)
            .ok_or(GpuRasterError::UnknownSoftMask(id))
    }

    /// Records one evaluated mask.
    fn insert(&mut self, id: SoftMaskId, image: peniko::ImageBrush) {
        self.images.insert(id, image);
    }
}

/// Turns one mask value per pixel into an image whose alpha channel is those values.
///
/// The colour components are zero and the image is declared premultiplied, which is the
/// consistent way to write "no colour, this much coverage": the only thing read from this
/// image is its alpha, because it is drawn under `Compose::DestIn`, and a premultiplied
/// black keeps that true whatever a sampler does with the other three channels on the way.
fn image_of(values: &[u8], target: TargetSpec) -> peniko::ImageBrush {
    let mut data = Vec::with_capacity(values.len().saturating_mul(4));
    for &value in values {
        data.extend_from_slice(&[0, 0, 0, value]);
    }
    peniko::ImageBrush::new(peniko::ImageData {
        data: data.into(),
        format: peniko::ImageFormat::Rgba8,
        alpha_type: peniko::ImageAlphaType::AlphaPremultiplied,
        width: target.width,
        height: target.height,
    })
    // Drawn at 1:1 with no transform, so no sample is ever interpolated and the sampler
    // cannot change a mask value. Stated rather than left to the default for that reason.
    .with_quality(peniko::ImageQuality::Low)
}

/// Evaluates every soft mask a display list carries, in the order they were registered.
///
/// The order matters and is guaranteed by construction: a mask's group is *run* before the
/// mask itself is registered, so a mask used inside another mask's group always has the
/// lower identifier and is already available when it is needed.
///
/// `render` renders one scene into a target-sized texture and hands back its straight-alpha
/// RGBA pixels. It is a parameter because the two callers own their GPU device differently —
/// [`crate::GpuRasterizer`] has its own, and the viewer shares one with its window.
///
/// # Errors
///
/// Whatever `render` returns, plus [`GpuRasterError`] for a command or paint the backend
/// cannot encode.
pub(crate) fn evaluate(
    list: &DisplayList,
    target: TargetSpec,
    render: &mut dyn FnMut(&mut vello::Scene) -> Result<Vec<u8>, GpuRasterError>,
) -> Result<SoftMaskRasters, GpuRasterError> {
    let mut rasters = SoftMaskRasters::none();
    for index in 0..list.soft_mask_count() {
        let id = SoftMaskId::new(u32::try_from(index).map_err(|_| GpuRasterError::TargetTooLarge)?);
        let mask = list
            .soft_mask(id)
            .ok_or(GpuRasterError::UnknownSoftMask(id))?;

        // Onto transparency, and with no medium imposed: §11.5.2 factors an arbitrary
        // backdrop out of the group's alpha, and §11.5.3's chosen backdrop is applied by
        // `SoftMask::value` per pixel rather than by painting it underneath, so that a blend
        // mode inside the group cannot see it.
        let mut scene = crate::scene::build_commands(list, &mask.commands, target, &rasters)?;
        let pixels = render(&mut scene)?;
        // §11.4.7's second raster, where the mask group's blending colour space has four
        // components: a second scene of the same size, from the second command list, read
        // back the same way. §11.3.4 composites per component and a texture has three
        // channels, so the four are two rasters and `SoftMask::paired_value` — the CPU
        // oracle's own function — is what reads §11.5.3's `Y` off them.
        let values = match mask.black.as_ref() {
            Some(half) => {
                let mut scene =
                    crate::scene::build_commands(list, &half.commands, target, &rasters)?;
                let black = render(&mut scene)?;
                mask.paired_values(&pixels, &black)
            }
            None => mask.values(&pixels),
        };
        rasters.insert(id, image_of(&values, target));
    }
    Ok(rasters)
}
