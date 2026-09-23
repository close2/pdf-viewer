//! The rare-case lanes: an image, a ramp sweep, a mesh, a §7.10.5 program — one
//! uniform-driven quad each.
//!
//! The brief's section 0 premise is that most of a page is a few glyph outlines repeated and
//! axis-aligned rectangles; ADR 0011 encodes what is left to match that premise rather
//! than to match its own complexity. An image (ISO 32000-2 §8.9.5) and a shading
//! (§8.7.4.5) each become a single quad carrying its own parameters, not a third and
//! fourth instance stream whose plumbing no measured page would fill.
//!
//! The two lanes are one module because they answer in the same shape: the fragment
//! shader maps device pixels back through an inverse transform carried in the op, so
//! the quad only has to *cover* the footprint, and the coverage it is weighted by is
//! either analytic — an axis-preserving image placement, a rect-hinted outline — or one
//! tile of the frame's scratch sheet, rasterised by the same CPU rasteriser every other
//! lane's coverage comes from.
//!
//! What the ops mean to the device that draws them is `device.rs`'s half; what a
//! command means to them is here.
//!
//! # The seam between a mark and its paint
//!
//! [`QuadPlacement`] is *where* a quad goes and what weights it; a paint is *what colour*
//! it is. The four lanes above and the function lane of ADR 0053 differ only in the second,
//! so the first is computed once, in [`Encoder::rect_placement`] and
//! [`Encoder::coverage_placement`], and the two ops are built from it. That seam is the
//! reason a device-evaluated colour needed no second copy of the tile arithmetic — the part
//! where "the quad is exactly the tile" is load-bearing for the shader's texel lookup.

use raster_scene::{
    Affine, BlendMode, ClipId, ImageFilter, ImageId, MaskId, Paint, Point, Rect, ShadingKind,
};

use super::clips::ResolvedClip;
use super::device_space::{apply, compose, transform_preserves_axes};
use super::function::FunctionGeometry;
use super::{ChildOp, DrawStyle, Encoder, Op};
use crate::error::RenderError;
use crate::raster::{DeviceTransform, Polyline, Rule};

/// One image draw (ISO 32000-2 §8.9.5), executed as a single uniform-driven quad.
///
/// The fragment shader maps device pixels back through `texel`, so the quad only has
/// to cover the footprint; an axis-preserving placement gets analytic edge coverage
/// from `image_rect`, an oblique one paints where centres land inside the image
/// (ADR 0011 carries both decisions).
#[derive(Debug, Clone, Copy)]
pub(crate) struct ImageOp {
    /// The resident image's raw id.
    pub image: u32,
    /// Device space → the drawn texture's texel space, §8.3.3 coefficient order: `s` to
    /// the right and `t` down from the image's top row, one unit per texel of the texture
    /// this op binds — the reduced grid where [`Self::reduced`] names one. See
    /// [`texel_transform`] for why it is carried instead of the unit square's inverse.
    pub texel: [f32; 6],
    /// The footprint's device bounding rectangle (exact when `axis_aligned`).
    pub image_rect: [f32; 4],
    /// The quad drawn: footprint ∩ clip ∩ target, at pixel bounds.
    pub dest: [f32; 4],
    /// The resolved clip rectangle.
    pub clip: [f32; 4],
    /// Where a rasterised residue clip sits in the frame's scratch, if one applies;
    /// its tile spans exactly `dest`.
    pub residue_origin: Option<[f32; 2]>,
    /// Whether the placement preserves axes (analytic edges).
    pub axis_aligned: bool,
    /// The command's constant alpha (§11.6.4.4).
    pub alpha: f32,
    /// The placement's resolved filter: `true` for linear (brief section 4.5, integration
    /// note 1 — resolved here since ADR 0089 where the command says `Auto`).
    pub linear: bool,
    /// The area-averaged variant this placement asks for, as `(x, y)` factors — the
    /// caller's documented §10.7.4 departure, resolved per placement (ADR 0089).
    /// `None` draws the image's own samples.
    pub reduced: Option<(u32, u32)>,
    pub style: DrawStyle,
    pub mask: Option<u32>,
}

/// `Auto` resolved where the placement is known (ADR 0089): the filter by the
/// mirrored §8.9.5.3 rule, and the caller's documented area-averaging reduction where
/// a device pixel gathers two samples — as `(linear, Some(factors))` naming a
/// resident variant the device realises once per `(image, factors)`.
fn resolve_filter(
    filter: ImageFilter,
    spec: &raster_scene::ImageSpec,
    to_device: &DeviceTransform,
) -> (bool, Option<(u32, u32)>) {
    let placement = [
        to_device.a,
        to_device.b,
        to_device.c,
        to_device.d,
        to_device.e,
        to_device.f,
    ];
    match filter {
        ImageFilter::Nearest => (false, None),
        ImageFilter::Linear => (true, None),
        ImageFilter::Auto { interpolate } => {
            match crate::raster::reduce::reduction(spec, interpolate, &placement) {
                Some(reduction) => (reduction.smoothed, Some(reduction.factors)),
                None => (
                    crate::raster::reduce::smoothed(
                        spec.width,
                        spec.height,
                        interpolate,
                        &placement,
                    ),
                    None,
                ),
            }
        }
    }
}

/// Which texture paints a [`ShadedOp`].
#[derive(Debug, Clone, Copy)]
pub(crate) enum PaintSource {
    /// A 256×1 pre-sampled colour ramp (raw ramp id).
    Ramp(u32),
    /// A pre-rasterised mesh (raw mesh id), sampled at absolute device pixels.
    Mesh(u32),
}

/// One shading or mesh draw (ISO 32000-2 §8.7.4.5), a single uniform-driven quad
/// over a coverage source: a scratch tile for a rasterised shape, or the analytic
/// rectangle for the rect-hinted case.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ShadedOp {
    pub paint: PaintSource,
    /// Inverse of the shading-space → device transform (identity for meshes, which
    /// are already device-space).
    pub inv: [f32; 6],
    /// 0 axial, 1 radial, 2 mesh — the shader's kind word.
    pub kind_word: f32,
    /// Bit 0: extend beyond the start; bit 1: beyond the end (§8.7.4.5.2/.3).
    pub extend_bits: u32,
    /// Axial/radial: start.xy, end.xy in shading space. Mesh: left, top in device
    /// pixels.
    pub geo0: [f32; 4],
    /// Radial: start radius, end radius.
    pub geo1: [f32; 4],
    /// The quad drawn; when coverage comes from scratch, exactly the tile's bounds.
    pub dest: [f32; 4],
    /// The coverage tile's origin in scratch, or `None` for the analytic rectangle.
    pub coverage_origin: Option<[f32; 2]>,
    /// The analytic coverage rectangle (the shape itself), used when
    /// `coverage_origin` is `None`.
    pub coverage_rect: [f32; 4],
    pub clip: [f32; 4],
    pub style: DrawStyle,
    pub mask: Option<u32>,
}

/// The shading-space geometry of a non-solid paint, resolved once per command.
#[derive(Debug, Clone, Copy)]
pub(super) struct ShadedGeometry {
    paint: PaintSource,
    kind_word: f32,
    extend_bits: u32,
    geo0: [f32; 4],
    geo1: [f32; 4],
    inv: [f32; 6],
}

/// Where a rare-case quad goes, what weights it, and under which of ADR 0010's styles.
///
/// The half of a quad op that does not depend on the paint. Both lanes that draw one build
/// it through the same two functions, so "the quad is exactly the tile" — which the
/// shaders' texel arithmetic (`coverage.xy + p − dest.xy`) depends on — is one statement
/// rather than one per lane.
#[derive(Debug, Clone, Copy)]
pub(crate) struct QuadPlacement {
    /// The quad drawn; when coverage comes from scratch, exactly the tile's bounds.
    pub dest: [f32; 4],
    /// The coverage tile's origin in scratch, or `None` for the analytic rectangle.
    pub coverage_origin: Option<[f32; 2]>,
    /// The analytic coverage rectangle (the shape itself), used when `coverage_origin`
    /// is `None`.
    pub coverage_rect: [f32; 4],
    pub clip: [f32; 4],
    pub style: DrawStyle,
    pub mask: Option<u32>,
}

/// A paint that draws as one quad: the ramp sweeps and meshes of the shading lane, or a
/// §7.10.5 program the device evaluates (ADR 0053).
///
/// One enum rather than two paths through the fill and stroke walks, so that `encode.rs`
/// resolves a paint and places a quad without knowing which of the two it has — the
/// difference is entirely in what is bound at draw time.
#[derive(Debug, Clone, Copy)]
pub(super) enum RarePaint {
    Shaded(ShadedGeometry),
    Function(FunctionGeometry),
}

impl Encoder<'_> {
    /// The image arm (ISO 32000-2 §8.9.5): one uniform-driven quad per placement,
    /// with a non-Normal blend through an implicit child, as fills take it.
    #[expect(clippy::too_many_arguments)] // one command's fields, destructured once
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    #[expect(clippy::arithmetic_side_effects, clippy::cast_precision_loss)]
    pub(super) fn encode_image(
        &mut self,
        image: ImageId,
        transform: Affine,
        alpha: f32,
        filter: ImageFilter,
        clip: Option<ClipId>,
        blend: BlendMode,
        mask: Option<MaskId>,
    ) -> Result<(), RenderError> {
        let mask = self.use_mask(mask)?;
        if blend != BlendMode::Normal && self.style == DrawStyle::Over {
            // §11.3.5 for a single element: an implicit one-element group (the same
            // degeneracy argument as in `encode_fill` skips it under knockout).
            let child = self.plan_child(|encoder| {
                encoder.encode_image(
                    image,
                    transform,
                    alpha,
                    filter,
                    clip,
                    BlendMode::Normal,
                    None,
                )
            })?;
            self.push_op(Op::Child(ChildOp::implicit_blend_group(child, blend, mask)))?;
            return Ok(());
        }
        let Some(stored) = self.resources.image(image) else {
            return Err(RenderError::UnknownImage { image });
        };
        let resolved = self.resolve_clip(clip)?;
        let to_device = compose(transform, self.viewport);
        let (linear, reduced) = resolve_filter(filter, &stored.spec, &to_device);
        match reduced {
            Some(factors) => self.used_reductions.insert((image.0, factors.0, factors.1)),
            None => self.used_images.insert(image.0),
        };
        let (width, height) = match reduced {
            Some((fx, fy)) => (
                stored.spec.width.div_ceil(fx.max(1)),
                stored.spec.height.div_ceil(fy.max(1)),
            ),
            None => (stored.spec.width, stored.spec.height),
        };
        let Some(texel) = texel_transform(&to_device, width, height) else {
            // A singular placement collapses the unit square to a zero-area set:
            // nothing to paint, and no way to map pixels back into it.
            return Ok(());
        };
        let corners = [
            apply(&to_device, Point::new(0.0, 0.0)),
            apply(&to_device, Point::new(1.0, 0.0)),
            apply(&to_device, Point::new(0.0, 1.0)),
            apply(&to_device, Point::new(1.0, 1.0)),
        ];
        let bx0 = corners.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let by0 = corners.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let bx1 = corners
            .iter()
            .map(|p| p.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let by1 = corners
            .iter()
            .map(|p| p.y)
            .fold(f32::NEG_INFINITY, f32::max);
        // The quad drawn: footprint ∩ clip ∩ target, expanded to pixel bounds so
        // partially covered edge pixels get their fragments. `mark_bounds` and not
        // `rect`, so that a residue clip bounds the residue tile this quad samples the
        // way it bounds every other rasterised tile (ADR 0057) — outside the chain's own
        // box that tile is zero, and a zero-weighted image texel is an image texel not
        // drawn.
        let clip_bounds = resolved.mark_bounds();
        let vx0 = bx0.max(clip_bounds.min.x).max(0.0);
        let vy0 = by0.max(clip_bounds.min.y).max(0.0);
        let vx1 = bx1.min(clip_bounds.max.x).min(self.viewport.width as f32);
        let vy1 = by1.min(clip_bounds.max.y).min(self.viewport.height as f32);
        if vx0 >= vx1 || vy0 >= vy1 {
            // Clipped to nothing or off the target: draws nothing, legitimately.
            // Exact, like the analytic rectangle lane — this *is* the region drawn.
            self.note_culled();
            return Ok(());
        }
        let left = vx0.floor() as i32;
        let top = vy0.floor() as i32;
        let width = (vx1.ceil() as i32 - left).max(1) as u32;
        let height = (vy1.ceil() as i32 - top).max(1) as u32;
        let residue_origin = if resolved.residues.is_some() {
            self.charge_tile(width, height)?;
            match self.residue_intersection(&resolved, left, top, width, height)? {
                Some(product) => {
                    let (sx, sy) = self.pack_scratch(&product)?;
                    Some([sx as f32, sy as f32])
                }
                None => None,
            }
        } else {
            None
        };
        self.push_op(Op::Image(Box::new(ImageOp {
            image: image.0,
            texel,
            image_rect: [bx0, by0, bx1, by1],
            dest: [left as f32, top as f32, vx1.ceil(), vy1.ceil()],
            clip: [
                resolved.rect.min.x,
                resolved.rect.min.y,
                resolved.rect.max.x,
                resolved.rect.max.y,
            ],
            residue_origin,
            axis_aligned: transform_preserves_axes(&to_device),
            alpha,
            linear,
            reduced,
            style: self.style,
            mask,
        })))
    }

    /// The shading-space geometry of a non-solid paint. `None` means a singular
    /// shading transform made the sweep unmappable — a degenerate shading matrix
    /// paints nothing rather than something arbitrary (brief section 4.7).
    ///
    /// Callers guarantee `paint` is not `Solid`. The shaded *command's* transform is
    /// deliberately absent here: a shading anchors to the scene through its own
    /// transform (§8.7.4.3), not to the path it fills.
    #[expect(clippy::cast_precision_loss)] // mesh anchors are device pixel indices
    pub(super) fn rare_paint(&mut self, paint: Paint) -> Result<Option<RarePaint>, RenderError> {
        match paint {
            // The two callers matched Solid off before calling.
            Paint::Solid(_) => unreachable!("rare_paint is called for non-solid paints only"),
            Paint::Function { .. } => Ok(self.function_geometry(paint)?.map(RarePaint::Function)),
            Paint::Shading {
                ramp,
                kind,
                transform,
            } => {
                if self.resources.ramp(ramp).is_none() {
                    return Err(RenderError::UnknownRamp { ramp });
                }
                self.used_ramps.insert(ramp.0);
                let Some(inverse) = transform.then(self.viewport.transform).invert() else {
                    return Ok(None);
                };
                let (kind_word, extend, geo0, geo1) = match kind {
                    ShadingKind::Axial { start, end, extend } => {
                        (0.0, extend, [start.x, start.y, end.x, end.y], [0.0; 4])
                    }
                    ShadingKind::Radial {
                        start,
                        start_radius,
                        end,
                        end_radius,
                        extend,
                    } => (
                        1.0,
                        extend,
                        [start.x, start.y, end.x, end.y],
                        [start_radius, end_radius, 0.0, 0.0],
                    ),
                };
                Ok(Some(RarePaint::Shaded(ShadedGeometry {
                    paint: PaintSource::Ramp(ramp.0),
                    kind_word,
                    extend_bits: u32::from(extend.0) | (u32::from(extend.1) << 1),
                    geo0,
                    geo1,
                    inv: [
                        inverse.a, inverse.b, inverse.c, inverse.d, inverse.e, inverse.f,
                    ],
                })))
            }
            Paint::Mesh(mesh) => {
                let Some(stored) = self.resources.mesh(mesh) else {
                    return Err(RenderError::UnknownMesh { mesh });
                };
                self.used_meshes.insert(mesh.0);
                // Meshes sample at absolute device pixels (integration note 5): no
                // inverse needed, the anchor is the whole mapping.
                Ok(Some(RarePaint::Shaded(ShadedGeometry {
                    paint: PaintSource::Mesh(mesh.0),
                    kind_word: 2.0,
                    extend_bits: 0,
                    geo0: [stored.spec.left as f32, stored.spec.top as f32, 0.0, 0.0],
                    geo1: [0.0; 4],
                    inv: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                })))
            }
        }
    }

    /// A rare-case fill of a rect-hinted outline under an axis-preserving transform:
    /// analytic coverage, no scratch tile (the shading twin of ADR 0007's fast
    /// path).
    pub(super) fn push_rare_rect(
        &mut self,
        paint: RarePaint,
        rect: Rect,
        to_device: &DeviceTransform,
        resolved: &ResolvedClip,
        style: DrawStyle,
        mask: Option<u32>,
    ) -> Result<(), RenderError> {
        let Some(placement) = self.rect_placement(rect, to_device, resolved, style, mask) else {
            return Ok(());
        };
        self.push_op(paint.at(placement))
    }

    /// A rare-case fill or stroke through a rasterised coverage tile in scratch.
    pub(super) fn push_rare_coverage(
        &mut self,
        paint: RarePaint,
        polylines: &[Polyline],
        rule: Rule,
        resolved: &ResolvedClip,
        style: DrawStyle,
        mask: Option<u32>,
    ) -> Result<(), RenderError> {
        let Some(placement) = self.coverage_placement(polylines, rule, resolved, style, mask)?
        else {
            return Ok(());
        };
        self.push_op(paint.at(placement))
    }

    /// Where the quad goes for a rect-hinted shape: the shape's device rectangle, cut to
    /// the clip and the target and expanded to pixel bounds, with the shape itself as the
    /// analytic coverage. `None` is a mark that reaches no pixel.
    #[expect(clippy::cast_precision_loss)] // target sizes are far below 2^24
    fn rect_placement(
        &mut self,
        rect: Rect,
        to_device: &DeviceTransform,
        resolved: &ResolvedClip,
        style: DrawStyle,
        mask: Option<u32>,
    ) -> Option<QuadPlacement> {
        let p0 = apply(to_device, rect.min);
        let p1 = apply(to_device, rect.max);
        let device_rect = Rect::new(
            Point::new(p0.x.min(p1.x), p0.y.min(p1.y)),
            Point::new(p0.x.max(p1.x), p0.y.max(p1.y)),
        );
        let vx0 = device_rect.min.x.max(resolved.rect.min.x).max(0.0);
        let vy0 = device_rect.min.y.max(resolved.rect.min.y).max(0.0);
        let vx1 = device_rect
            .max
            .x
            .min(resolved.rect.max.x)
            .min(self.viewport.width as f32);
        let vy1 = device_rect
            .max
            .y
            .min(resolved.rect.max.y)
            .min(self.viewport.height as f32);
        if vx0 >= vx1 || vy0 >= vy1 {
            return None;
        }
        Some(QuadPlacement {
            dest: [vx0.floor(), vy0.floor(), vx1.ceil(), vy1.ceil()],
            coverage_origin: None,
            coverage_rect: [
                device_rect.min.x,
                device_rect.min.y,
                device_rect.max.x,
                device_rect.max.y,
            ],
            clip: [
                resolved.rect.min.x,
                resolved.rect.min.y,
                resolved.rect.max.x,
                resolved.rect.max.y,
            ],
            style,
            mask,
        })
    }

    /// Where the quad goes for a rasterised shape: exactly the coverage tile, which is
    /// what both shaders' texel arithmetic (`coverage.xy + p − dest.xy`) depends on.
    #[expect(clippy::cast_precision_loss, clippy::arithmetic_side_effects)]
    fn coverage_placement(
        &mut self,
        polylines: &[Polyline],
        rule: Rule,
        resolved: &ResolvedClip,
        style: DrawStyle,
        mask: Option<u32>,
    ) -> Result<Option<QuadPlacement>, RenderError> {
        let Some(tile) = self.coverage_tile(polylines, rule, resolved)? else {
            return Ok(None);
        };
        let (sx, sy) = self.pack_scratch(&tile)?;
        Ok(Some(QuadPlacement {
            dest: [
                tile.left as f32,
                tile.top as f32,
                (tile.left + tile.width.cast_signed()) as f32,
                (tile.top + tile.height.cast_signed()) as f32,
            ],
            coverage_origin: Some([sx as f32, sy as f32]),
            coverage_rect: [0.0; 4],
            clip: [
                resolved.rect.min.x,
                resolved.rect.min.y,
                resolved.rect.max.x,
                resolved.rect.max.y,
            ],
            style,
            mask,
        }))
    }
}

impl RarePaint {
    /// The op this paint becomes once the mark's placement is known.
    fn at(self, placement: QuadPlacement) -> Op {
        match self {
            Self::Shaded(geometry) => Op::Shaded(Box::new(ShadedOp {
                paint: geometry.paint,
                inv: geometry.inv,
                kind_word: geometry.kind_word,
                extend_bits: geometry.extend_bits,
                geo0: geometry.geo0,
                geo1: geometry.geo1,
                dest: placement.dest,
                coverage_origin: placement.coverage_origin,
                coverage_rect: placement.coverage_rect,
                clip: placement.clip,
                style: placement.style,
                mask: placement.mask,
            })),
            Self::Function(geometry) => Op::Function(Box::new(geometry.at(placement))),
        }
    }
}

/// The map from a device point to the texel space of the `width` × `height` texture an image
/// op binds, or `None` for a singular placement.
///
/// ISO 32000-2 §10.7.4 decides a sampled image's pixel by one point:
///
/// > The position of the centre of such a pixel -in other words, the point whose coordinate
/// > values have fractional parts of one-half -shall be mapped back into source space to
/// > determine how to colour the pixel.
///
/// and §8.9.4 gives source space one unit per sample, "[t]he upper-left corner of the first
/// sample is at coordinates (0, 0)". So the texel a nearest lookup reads is the one whose
/// square holds that point — the sample whose upper-left corner is the floor of it, which is
/// §10.7.4's own convention for pixels ("[a] pixel is a square region identified by the
/// location of its corner with minimum horizontal and vertical coordinates"), applied to the
/// samples, where §8.9.4 states the squares and not which of two neighbours owns the edge
/// they share. That edge is where a pixel centre lands at one device pixel per sample with the
/// image's origin on a half pixel, which is an ordinary placement rather than a corner case.
///
/// **Composed here in `f64` and rounded once, rather than composed per fragment**, because the
/// shader would otherwise multiply the unit square's inverse — whose `1 ⁄ w` no float holds
/// exactly — back up by the texture's dimensions, landing a hair below the whole number on
/// some rows and columns and reading the sample above or to the left: a whole image shifted by
/// one sample wherever it happened, which `render-raster`'s `masked_image_edge.rs` measured at
/// up to 239 levels against the oracle (the caller's ADR 1302). Rounded once, a native
/// placement's coefficients are exactly ±1 and its translation a half pixel, so every centre
/// maps to a whole number exactly and the floor is the clause's.
fn texel_transform(to_device: &DeviceTransform, width: u32, height: u32) -> Option<[f32; 6]> {
    let m = [
        to_device.a,
        to_device.b,
        to_device.c,
        to_device.d,
        to_device.e,
        to_device.f,
    ]
    .map(f64::from);
    let det = m[0] * m[3] - m[1] * m[2];
    if det == 0.0 || !det.is_finite() {
        return None;
    }
    // The unit square's inverse, §8.3.3 order: u = inv[0]·x + inv[2]·y + inv[4], and v the
    // same with the odd coefficients.
    let inv = [
        m[3] / det,
        -m[1] / det,
        -m[2] / det,
        m[0] / det,
        (m[2] * m[5] - m[3] * m[4]) / det,
        (m[1] * m[4] - m[0] * m[5]) / det,
    ];
    // §8.9.4's image space: s = width·u, and t = height·(1 − v) because the top row sits at
    // v = 1.
    let (across, down) = (f64::from(width), f64::from(height));
    let texel = [
        inv[0] * across,
        -inv[1] * down,
        inv[2] * across,
        -inv[3] * down,
        inv[4] * across,
        (1.0 - inv[5]) * down,
    ];
    #[expect(clippy::cast_possible_truncation)] // rounding to f32 once is the point
    Some(texel.map(|coefficient| coefficient as f32))
}
