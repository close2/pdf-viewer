//! CPU rasteriser backend; the correctness oracle for the GPU backend.
//!
//! Implements [`pdf_render::Rasterizer`] on the CPU using `tiny-skia`. This backend
//! has three jobs, and the second and third are why it exists first:
//!
//! 1. It renders when no usable GPU is present.
//! 2. It is the **reference** against which `render-gpu` is validated. Both backends
//!    consume the same [`pdf_render::DisplayList`], so any difference between their
//!    outputs is a backend defect rather than a difference in how the document was
//!    interpreted — a far tighter test than comparing against another PDF viewer,
//!    where antialiasing and colour handling differ for legitimate reasons.
//! 3. It is the **startup path**. Creating a GPU device and compiling pipelines costs
//!    tens to hundreds of milliseconds, so page one renders here while the GPU
//!    initialises on another thread.
//!
//! Correctness therefore outranks speed in this crate. Where the two conflict, this
//! backend takes the clearer construction and leaves optimisation to `render-gpu`.
//! See `doc/adr/0002-cpu-rasteriser-first.md`.

#![forbid(unsafe_code)]

mod convert;
mod shading;

use std::collections::{HashMap, HashSet};

use pdf_render::display_list::Clip;
use pdf_render::{
    BackendError, ClipId, Color, Command, DisplayList, Paint, Raster, RasterFormat, Rasterizer,
    TargetSpec, Transform,
};

/// Renders display lists on the CPU.
#[derive(Debug, Clone)]
pub struct CpuRasterizer {
    background: Color,
    anti_alias: bool,
}

impl CpuRasterizer {
    /// Creates a rasteriser that paints onto an opaque white background.
    ///
    /// White rather than transparent because a PDF page is conceptually opaque white
    /// unless the document paints otherwise, and because the reference renderers used
    /// by the comparison harness (`pdftoppm`, `mutool draw`) do the same. Matching
    /// them removes an entire class of spurious differences.
    #[must_use]
    pub fn new() -> Self {
        Self {
            background: Color::WHITE,
            anti_alias: true,
        }
    }

    /// Sets the background colour painted before any command.
    ///
    /// [`Color::TRANSPARENT`] is the right choice when compositing a page over
    /// something else, such as a page-edge shadow in the viewer.
    #[must_use]
    pub fn with_background(mut self, background: Color) -> Self {
        self.background = background;
        self
    }

    /// Enables or disables antialiasing.
    ///
    /// Disabling it is useful when diffing against a reference renderer that was
    /// itself run without antialiasing, since it makes exact pixel comparison
    /// meaningful rather than merely approximate.
    #[must_use]
    pub fn with_anti_alias(mut self, anti_alias: bool) -> Self {
        self.anti_alias = anti_alias;
        self
    }

    /// Builds the `tiny-skia` paint for a resolved paint and blend mode.
    ///
    /// `page_to_path` maps page space into the space the path is stated in — the
    /// inverse of the command's own transform. See [`shading::shader`] for why a paint
    /// is positioned in the path's space rather than the device's; getting this wrong
    /// draws a gradient in the right shape and the wrong place, which no metric
    /// short of looking at the page detects.
    ///
    /// # Errors
    ///
    /// Returns [`CpuRasterError::UnsupportedPaint`] for a paint variant this backend
    /// does not implement yet. [`Paint`] is `#[non_exhaustive]`, so shadings and
    /// patterns will be added to it before this backend handles them; failing loudly
    /// in the interim is deliberate, because a silent fallback colour would give the
    /// comparison harness a plausible-looking wrong image.
    fn paint<'a>(
        &self,
        paint: &Paint,
        blend: pdf_render::BlendMode,
        page_to_path: Transform,
        scratch: &'a mut Option<tiny_skia::Pixmap>,
    ) -> Result<tiny_skia::Paint<'a>, CpuRasterError> {
        let shader = match paint {
            Paint::Solid(colour) => tiny_skia::Shader::SolidColor(convert::color(*colour)),
            Paint::Shading(shading) => shading::shader(shading, page_to_path, scratch)
                .ok_or_else(|| CpuRasterError::UnsupportedPaint(format!("{shading:?}")))?,
            other => return Err(CpuRasterError::UnsupportedPaint(format!("{other:?}"))),
        };
        Ok(tiny_skia::Paint {
            shader,
            blend_mode: convert::blend_mode(blend),
            anti_alias: self.anti_alias,
            ..tiny_skia::Paint::default()
        })
    }
}

impl Default for CpuRasterizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Rasterizer for CpuRasterizer {
    type Error = CpuRasterError;

    fn name(&self) -> &'static str {
        "cpu"
    }

    fn rasterize(&mut self, list: &DisplayList, target: TargetSpec) -> Result<Raster, Self::Error> {
        let mut pixmap = tiny_skia::Pixmap::new(target.width, target.height).ok_or(
            CpuRasterError::Allocation {
                width: target.width,
                height: target.height,
            },
        )?;
        pixmap.fill(convert::color(self.background));

        let to_device = target.transform;
        let mut masks = MaskCache::new(target, self.anti_alias);

        for command in list.commands() {
            // Resolved before the match so that both arms share one code path for
            // clip handling; a per-arm lookup would be a place for them to diverge.
            let clip = match command.clip() {
                Some(id) => Some(masks.get(list, id)?),
                None => None,
            };

            match command {
                Command::Fill {
                    path,
                    transform,
                    fill_rule,
                    paint,
                    blend,
                    ..
                } => {
                    let path = convert::path(path).ok_or(CpuRasterError::InvalidPath)?;

                    // A mesh carries a colour per triangle corner, which no shader can
                    // express, so it is drawn triangle by triangle inside the shape rather
                    // than as a paint over it.
                    if let Paint::Shading(shading) = paint
                        && let pdf_render::ShadingKind::Mesh { triangles } = &shading.kind
                    {
                        shading::fill_mesh(
                            &mut pixmap,
                            &path,
                            triangles,
                            shading.transform.then(to_device),
                            convert::fill_rule(*fill_rule),
                            convert::transform(transform.then(to_device)),
                            clip,
                            convert::blend_mode(*blend),
                            self.anti_alias,
                        );
                        continue;
                    }

                    // A sampled shading's pixels are borrowed by its shader, so they need
                    // somewhere to live for exactly as long as this call.
                    let mut scratch = None;
                    pixmap.fill_path(
                        &path,
                        &self.paint(paint, *blend, page_to_path(*transform)?, &mut scratch)?,
                        convert::fill_rule(*fill_rule),
                        convert::transform(transform.then(to_device)),
                        clip,
                    );
                }
                Command::Stroke {
                    path,
                    transform,
                    stroke,
                    paint,
                    blend,
                    ..
                } => {
                    let path = convert::path(path).ok_or(CpuRasterError::InvalidPath)?;
                    let mut scratch = None;
                    pixmap.stroke_path(
                        &path,
                        &self.paint(paint, *blend, page_to_path(*transform)?, &mut scratch)?,
                        &convert::stroke(stroke),
                        convert::transform(transform.then(to_device)),
                        clip,
                    );
                }
                Command::Image {
                    image,
                    transform,
                    alpha,
                    blend,
                    ..
                } => {
                    let placement = ImagePlacement {
                        transform: *transform,
                        alpha: *alpha,
                        blend: *blend,
                        to_device,
                    };
                    self.draw_image(&mut pixmap, image, placement, clip)?;
                }
                // `Command` is `#[non_exhaustive]`, so new commands will appear here before
                // this backend implements them. Erroring keeps an unimplemented command
                // visible instead of rendering a page that looks almost right.
                other => {
                    return Err(CpuRasterError::UnsupportedCommand(format!("{other:?}")));
                }
            }
        }

        Ok(Raster {
            width: target.width,
            height: target.height,
            format: RasterFormat::Rgba8,
            // `tiny-skia` stores premultiplied alpha internally; `Raster` is documented
            // as straight alpha, so the conversion happens here at the backend boundary
            // rather than being left to every consumer to remember.
            data: pixmap.take_demultiplied(),
        })
    }
}

/// Where and how an image is placed.
///
/// Grouped because these four always travel together, and passing them separately made
/// the call site a row of unlabelled arguments.
#[derive(Debug, Clone, Copy)]
struct ImagePlacement {
    transform: Transform,
    alpha: f32,
    blend: pdf_render::BlendMode,
    to_device: Transform,
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
    /// Nearest-neighbour would alias badly when a page is viewed at less than full size,
    /// which is the common case, so bilinear filtering is used.
    fn draw_image(
        &self,
        pixmap: &mut tiny_skia::Pixmap,
        image: &pdf_render::Image,
        placement: ImagePlacement,
        clip: Option<&tiny_skia::Mask>,
    ) -> Result<(), CpuRasterError> {
        let ImagePlacement {
            transform,
            alpha,
            blend,
            to_device,
        } = placement;
        if !image.is_consistent() {
            return Err(CpuRasterError::InvalidImage {
                width: image.width,
                height: image.height,
                bytes: image.data.len(),
            });
        }

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
        // Image space (pixels, y down) to the unit square (y up).
        let to_unit = Transform::new(1.0 / width, 0.0, 0.0, -1.0 / height, 0.0, 1.0);
        let pattern_transform = convert::transform(to_unit.then(transform).then(to_device));

        let paint = tiny_skia::Paint {
            shader: tiny_skia::Pattern::new(
                samples.as_ref(),
                tiny_skia::SpreadMode::Pad,
                tiny_skia::FilterQuality::Bilinear,
                alpha.clamp(0.0, 1.0),
                pattern_transform,
            ),
            blend_mode: convert::blend_mode(blend),
            anti_alias: self.anti_alias,
            ..tiny_skia::Paint::default()
        };

        // The unit square, transformed into device space by the paint's own matrix above.
        let mut builder = tiny_skia::PathBuilder::new();
        builder.push_rect(tiny_skia::Rect::from_xywh(0.0, 0.0, 1.0, 1.0).ok_or(
            CpuRasterError::InvalidImage {
                width: 1,
                height: 1,
                bytes: 0,
            },
        )?);
        let square = builder.finish().ok_or(CpuRasterError::InvalidPath)?;

        pixmap.fill_path(
            &square,
            &paint,
            tiny_skia::FillRule::Winding,
            convert::transform(transform.then(to_device)),
            clip,
        );
        Ok(())
    }
}

/// Maps page space into the space a path drawn under `transform` is stated in.
///
/// This is what a shading paint has to be expressed in — see [`shading::shader`] — and
/// it is the inverse of the command's own transform.
///
/// # Errors
///
/// Returns [`CpuRasterError::UnsupportedPaint`] when `transform` is singular. A path
/// under a singular transform has collapsed to a line or a point, so there is no space
/// left to position a paint in. Reporting it is deliberate: the alternative is a
/// gradient placed somewhere arbitrary, which looks like a rendering rather than a
/// failure.
fn page_to_path(transform: Transform) -> Result<Transform, CpuRasterError> {
    transform.invert().ok_or_else(|| {
        CpuRasterError::UnsupportedPaint(format!("singular transform {transform:?}"))
    })
}

/// Multiplies a straight-alpha channel by its alpha.
fn premultiply(value: u8, alpha: u8) -> u8 {
    // Rounded rather than truncated, so a fully opaque pixel round-trips exactly.
    let scaled = u16::from(value)
        .saturating_mul(u16::from(alpha))
        .saturating_add(127);
    u8::try_from(scaled / 255).unwrap_or(u8::MAX)
}

/// Builds and memoises clip masks.
///
/// A clip commonly applies to thousands of consecutive commands, so rasterising its
/// mask once per command would dominate the render. Masks are built on first use and
/// reused for the remainder of the page.
struct MaskCache {
    target: TargetSpec,
    anti_alias: bool,
    built: HashMap<ClipId, tiny_skia::Mask>,
}

impl MaskCache {
    fn new(target: TargetSpec, anti_alias: bool) -> Self {
        Self {
            target,
            anti_alias,
            built: HashMap::new(),
        }
    }

    /// Returns the mask for `id`, building it and any missing ancestors.
    fn get(&mut self, list: &DisplayList, id: ClipId) -> Result<&tiny_skia::Mask, CpuRasterError> {
        if !self.built.contains_key(&id) {
            let chain = Self::resolve_chain(list, id)?;
            self.build_chain(list, &chain)?;
        }
        self.built.get(&id).ok_or(CpuRasterError::UnknownClip(id))
    }

    /// Walks parent links from `id` to the root, returning the chain root-first.
    ///
    /// Iterative rather than recursive, and bounded by a seen-set: a deep or cyclic
    /// chain is reachable from a malformed document, and recursion there would exhaust
    /// the stack. Both a cycle and a dangling reference are reported as errors.
    fn resolve_chain(list: &DisplayList, id: ClipId) -> Result<Vec<ClipId>, CpuRasterError> {
        let mut chain = Vec::new();
        let mut seen = HashSet::new();
        let mut current = Some(id);

        while let Some(this) = current {
            if !seen.insert(this) {
                return Err(CpuRasterError::CyclicClip(this));
            }
            let clip = list.clip(this).ok_or(CpuRasterError::UnknownClip(this))?;
            chain.push(this);
            current = clip.parent;
        }

        chain.reverse();
        Ok(chain)
    }

    /// Builds every mask in a root-first chain, reusing any already built.
    fn build_chain(&mut self, list: &DisplayList, chain: &[ClipId]) -> Result<(), CpuRasterError> {
        for &id in chain {
            if self.built.contains_key(&id) {
                continue;
            }
            let clip = list.clip(id).ok_or(CpuRasterError::UnknownClip(id))?;
            let mask = self.build_one(clip)?;
            self.built.insert(id, mask);
        }
        Ok(())
    }

    /// Builds a single mask. Requires that any parent has already been built.
    fn build_one(&self, clip: &Clip) -> Result<tiny_skia::Mask, CpuRasterError> {
        let path = convert::path(&clip.path).ok_or(CpuRasterError::InvalidPath)?;
        let transform = convert::transform(clip.transform.then(self.target.transform));
        let fill_rule = convert::fill_rule(clip.fill_rule);

        if let Some(parent) = clip.parent {
            // Nested: start from the parent's coverage and intersect, so the effective
            // clip is the intersection of the whole chain.
            let mut mask = self
                .built
                .get(&parent)
                .ok_or(CpuRasterError::UnknownClip(parent))?
                .clone();
            mask.intersect_path(&path, fill_rule, self.anti_alias, transform);
            Ok(mask)
        } else {
            // Root: a fresh mask is fully opaque-clipped, so filling the path opens it.
            let mut mask = tiny_skia::Mask::new(self.target.width, self.target.height).ok_or(
                CpuRasterError::Allocation {
                    width: self.target.width,
                    height: self.target.height,
                },
            )?;
            mask.fill_path(&path, fill_rule, self.anti_alias, transform);
            Ok(mask)
        }
    }
}

/// Failures specific to the CPU backend.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CpuRasterError {
    /// A pixmap or mask of the requested size could not be allocated.
    #[error("could not allocate a {width}x{height} buffer")]
    Allocation {
        /// Requested width in pixels.
        width: u32,
        /// Requested height in pixels.
        height: u32,
    },
    /// A path was empty or contained non-finite coordinates.
    ///
    /// Reported rather than skipped: silently drawing nothing would hand the
    /// comparison harness a plausible-looking wrong image instead of a failure.
    #[error("path is empty or contains non-finite coordinates")]
    InvalidPath,
    /// A command referenced a clip that is not present in the display list.
    #[error("clip {0:?} is not present in this display list")]
    UnknownClip(ClipId),
    /// A clip's parent chain forms a cycle.
    #[error("clip {0:?} is part of a cyclic parent chain")]
    CyclicClip(ClipId),
    /// A command variant this backend does not implement yet.
    #[error("command not supported by the CPU backend: {0}")]
    UnsupportedCommand(String),
    /// A paint variant this backend does not implement yet.
    #[error("paint not supported by the CPU backend: {0}")]
    UnsupportedPaint(String),
    /// An image's dimensions and buffer length disagree.
    #[error("image is {width}x{height} but holds {bytes} bytes")]
    InvalidImage {
        /// Declared width.
        width: u32,
        /// Declared height.
        height: u32,
        /// Actual buffer length.
        bytes: usize,
    },
    /// A failure originating in the shared backend layer.
    #[error(transparent)]
    Target(#[from] BackendError),
}
