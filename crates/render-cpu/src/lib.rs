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

use std::collections::{HashMap, HashSet, VecDeque};

use pdf_render::{
    BackendError, ClipId, Color, Command, DisplayList, MAX_EXTENT, MAX_GROUP_DEPTH, Paint, Raster,
    RasterFormat, Rasterizer, TargetSpec, Transform, impose_on_medium,
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
        // Checked here rather than assumed, because [`Band`] converts a row index to
        // `f32` and that is lossless only below 2^24. `TargetSpec::for_page` already
        // enforces this, but the struct's fields are public, so a hand-built spec can
        // violate it and would misplace every banded command if it did.
        for extent in [target.width, target.height] {
            if extent > MAX_EXTENT {
                return Err(BackendError::ExtentTooLarge {
                    extent: u64::from(extent),
                    limit: MAX_EXTENT,
                }
                .into());
            }
        }

        let mut pixmap = tiny_skia::Pixmap::new(target.width, target.height).ok_or(
            CpuRasterError::Allocation {
                width: target.width,
                height: target.height,
            },
        )?;
        // Not filled with the background: §11.4.7 makes the page group *isolated*, so the
        // page's elements composite onto transparency and the medium's colour is applied to
        // the result — see `impose_on_medium`, which both backends end with.

        let mut masks = MaskCache::new(target, self.anti_alias);
        self.encode(&mut pixmap, list, list.commands(), target, &mut masks, 0)?;

        // §11.4.7's page group is isolated, so the medium's colour is composited with the
        // finished page rather than being the backdrop its blend modes saw. Before the
        // conversion below, because `tiny-skia`'s pixels are premultiplied here and that is
        // where the composite is exact.
        impose_on_medium(pixmap.data_mut(), self.background);

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

impl CpuRasterizer {
    /// Draws a sequence of commands onto a target-sized pixmap.
    ///
    /// Split out of [`CpuRasterizer::rasterize`] because a transparency group is the same
    /// loop over a nested list and a fresh surface, and a second copy of the band and clip
    /// handling would be a place for the two to diverge.
    ///
    /// `depth` counts enclosing groups; see [`MAX_GROUP_DEPTH`] for why it is bounded.
    ///
    /// # Errors
    ///
    /// Returns [`CpuRasterError`] for anything [`CpuRasterizer::draw`] rejects, for a clip
    /// chain that is dangling or cyclic, and for groups nested past the bound.
    fn encode(
        &self,
        pixmap: &mut tiny_skia::Pixmap,
        list: &DisplayList,
        commands: &[Command],
        target: TargetSpec,
        masks: &mut MaskCache,
        depth: usize,
    ) -> Result<(), CpuRasterError> {
        for command in commands {
            // A group is the one command that needs the mask cache mutably *while* it
            // draws, so it cannot hold a clip mask borrowed from it across the recursion.
            // It therefore resolves its own clip, twice, either side of its elements.
            if let Command::Group {
                commands,
                alpha,
                blend,
                ..
            } = command
            {
                self.draw_group(
                    pixmap,
                    list,
                    Group {
                        commands,
                        alpha: *alpha,
                        blend: *blend,
                        clip: command.clip(),
                    },
                    target,
                    masks,
                    depth,
                )?;
                continue;
            }

            // Resolved before the match so that every arm shares one code path for
            // clip handling; a per-arm lookup would be a place for them to diverge.
            let (band, clip) = match command.clip() {
                Some(id) => match masks.get(list, id)? {
                    Some((band, mask)) => (band, Some(mask)),
                    // The clip admits no row of the target, so nothing this command
                    // draws can survive it.
                    None => continue,
                },
                None => (Band::whole(target), None),
            };

            // Everything below draws into the band rather than the page, which is what
            // keeps a command's cost proportional to the pixels its clip can admit.
            // The device transform carries the band's offset so that geometry, paints
            // and images all move together; missing one would tear the page apart in a
            // way no metric would notice, so there is exactly one of these.
            let to_device = target.transform.then(band.offset());
            let mut surface = band.rows(pixmap).ok_or(CpuRasterError::Allocation {
                width: target.width,
                height: band.height,
            })?;

            self.draw(&mut surface, command, to_device, clip)?;
        }
        Ok(())
    }

    /// Composites one transparency group (ISO 32000-2 §11.4.1).
    ///
    /// The elements are drawn onto a fully transparent surface of the target's size — the
    /// isolated group's initial backdrop of §11.4.5 — and the result is then painted onto
    /// the page once, under the group's own constant alpha and blend mode. Compositing the
    /// elements one at a time onto the page instead is what §11.6.6's initialisation of the
    /// alpha constants exists to prevent, and is visibly different wherever two elements
    /// overlap.
    ///
    /// The surface is the whole target rather than the group's band because the elements'
    /// clips are resolved against the target, so their bands are target rows; a band-sized
    /// buffer would need every one of them shifted, and one coordinate system that is right
    /// beats two that have to agree.
    fn draw_group(
        &self,
        pixmap: &mut tiny_skia::Pixmap,
        list: &DisplayList,
        group: Group<'_>,
        target: TargetSpec,
        masks: &mut MaskCache,
        depth: usize,
    ) -> Result<(), CpuRasterError> {
        let depth = depth.saturating_add(1);
        if depth > MAX_GROUP_DEPTH {
            return Err(BackendError::GroupsTooDeep {
                depth,
                limit: MAX_GROUP_DEPTH,
            }
            .into());
        }

        // The band is taken first so that a group whose clip admits no row costs nothing at
        // all — not even the buffer.
        let band = match group.clip {
            Some(id) => match masks.get(list, id)? {
                Some((band, _)) => band,
                None => return Ok(()),
            },
            None => Band::whole(target),
        };

        let mut buffer = tiny_skia::Pixmap::new(target.width, target.height).ok_or(
            CpuRasterError::Allocation {
                width: target.width,
                height: target.height,
            },
        )?;
        self.encode(&mut buffer, list, group.commands, target, masks, depth)?;

        // Resolved again rather than held across the recursion: the elements' own clips
        // share this cache and may have evicted the entry, and a rebuilt mask is the mask
        // that was dropped — see `a_rebuilt_mask_is_the_mask_that_was_evicted`.
        let clip = match group.clip {
            Some(id) => match masks.get(list, id)? {
                Some((_, mask)) => Some(mask),
                None => return Ok(()),
            },
            None => None,
        };

        let paint = tiny_skia::PixmapPaint {
            opacity: group.alpha.clamp(0.0, 1.0),
            blend_mode: convert::blend_mode(group.blend),
            // The buffer is drawn at 1:1 with no transform, so no sample is ever
            // interpolated and the quality setting cannot change a pixel.
            quality: tiny_skia::FilterQuality::Nearest,
        };
        let mut surface = band.rows(pixmap).ok_or(CpuRasterError::Allocation {
            width: target.width,
            height: band.height,
        })?;
        // Negative, because the buffer covers the whole target and the surface starts at
        // the band's first row.
        let top = i32::try_from(band.top).map_err(|_| CpuRasterError::Allocation {
            width: target.width,
            height: band.height,
        })?;
        surface.draw_pixmap(
            0,
            top.saturating_neg(),
            buffer.as_ref(),
            &paint,
            tiny_skia::Transform::identity(),
            clip,
        );
        Ok(())
    }

    /// Draws one command onto `surface`, which is the band its clip admits.
    ///
    /// `to_device` maps page space onto that band, so it already carries the band's row
    /// offset; every transform below composes with it and none reaches the page directly.
    ///
    /// # Errors
    ///
    /// Returns [`CpuRasterError`] for a path the rasteriser rejects, for a paint or a
    /// command variant this backend does not implement, or for an inconsistent image.
    fn draw(
        &self,
        surface: &mut tiny_skia::PixmapMut<'_>,
        command: &Command,
        to_device: Transform,
        clip: Option<&tiny_skia::Mask>,
    ) -> Result<(), CpuRasterError> {
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
                        surface,
                        &path,
                        triangles,
                        shading.transform.then(to_device),
                        convert::fill_rule(*fill_rule),
                        convert::transform(transform.then(to_device)),
                        clip,
                        convert::blend_mode(*blend),
                        self.anti_alias,
                    );
                    return Ok(());
                }

                // A sampled shading's pixels are borrowed by its shader, so they need
                // somewhere to live for exactly as long as this call.
                let mut scratch = None;
                surface.fill_path(
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
                surface.stroke_path(
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
                self.draw_image(surface, image, placement, clip)?;
            }
            // `Command` is `#[non_exhaustive]`, so new commands will appear here before
            // this backend implements them. Erroring keeps an unimplemented command
            // visible instead of rendering a page that looks almost right.
            other => {
                return Err(CpuRasterError::UnsupportedCommand(format!("{other:?}")));
            }
        }
        Ok(())
    }
}

/// One transparency group, unpacked from its command.
///
/// Grouped for the same reason as [`ImagePlacement`]: four values that always travel
/// together, and a call site that would otherwise be a row of unlabelled arguments.
#[derive(Debug, Clone, Copy)]
struct Group<'a> {
    commands: &'a [Command],
    alpha: f32,
    blend: pdf_render::BlendMode,
    clip: Option<ClipId>,
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
    /// Whether the samples are filtered is [`pdf_render::Image::is_smoothed`]'s decision,
    /// from §8.9.5.3's `/Interpolate` and how large the image is being drawn: bilinear where
    /// a page is viewed at less than full size, which is the common case and where
    /// nearest-neighbour would alias badly, and nearest where the document has magnified a
    /// few samples across an area and asked for no smoothing.
    ///
    /// Deep reductions are handled before that, by [`pdf_render::Image::area_averaged`],
    /// which leaves this filter under a two-fold shrink where four taps see every sample.
    fn draw_image(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
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

        let placement = transform.then(to_device);
        // Blocks of samples that would share one device pixel are averaged before
        // `tiny-skia` sees them, because its bilinear filter reads four neighbours whatever
        // the reduction and an eleven-fold shrink never looks at most of the source. The
        // decision is `pdf_render`'s so that both backends make it identically; ADR 0025 has
        // why it is a departure from §10.7.4 rather than a reading of it.
        let reduced = image.area_averaged(placement);
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

        let filter = if image.is_smoothed(placement) {
            tiny_skia::FilterQuality::Bilinear
        } else {
            tiny_skia::FilterQuality::Nearest
        };

        let paint = tiny_skia::Paint {
            shader: tiny_skia::Pattern::new(
                samples.as_ref(),
                tiny_skia::SpreadMode::Pad,
                filter,
                alpha.clamp(0.0, 1.0),
                convert::transform(to_unit),
            ),
            blend_mode: convert::blend_mode(blend),
            anti_alias: self.anti_alias,
            ..tiny_skia::Paint::default()
        };

        // The unit square, which the command's transform carries onto the page.
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
            convert::transform(placement),
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

/// A contiguous run of target rows: the only rows a command is allowed to mark.
///
/// A command can only change pixels its clip admits, and a clip usually admits very
/// little of the page. Restricting the drawing surface to those rows is what keeps a
/// command's cost proportional to what it can actually change, and it is not a micro
/// optimisation: on `bug1721218_reduced.pdf` (3576 distinct clips, each covering a
/// mean 1.2% of the page's height) `callgrind` attributed **80% of the whole render**
/// to the raster pipeline's gradient stage, evaluating shadings across the full page
/// only for the clip to discard almost all of it. Bounding the surface removes that
/// work rather than masking it away afterwards.
///
/// Rows and not a rectangle, because a pixmap's rows are contiguous in memory and its
/// columns are not: `tiny_skia::PixmapMut::from_bytes` can borrow a band of a pixmap,
/// and `tiny-skia` exposes no sub-rectangle view. The same constraint applies to the
/// clip mask, which must share the pixmap's row stride, so it is band-tall and
/// page-wide too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Band {
    /// First target row this band covers.
    top: u32,
    /// Number of rows.
    height: u32,
}

impl Band {
    /// The band covering the whole target — what an unclipped command draws into.
    fn whole(target: TargetSpec) -> Self {
        Self {
            top: 0,
            height: target.height,
        }
    }

    /// The band covering `bounds`, clipped to a target `height` rows tall.
    ///
    /// Returns `None` when `bounds` covers no row of the target.
    ///
    /// `bounds` is widened by a row before rounding outward. Clip bounds are computed
    /// from a path's control points and then transformed, while the mask is built by
    /// transforming the path itself; the two agree to within floating-point rounding
    /// rather than exactly, and a band one row short would erase a row of a shading
    /// that the clip admits. A spare row costs a fraction of a percent of the band and
    /// removes that class of defect entirely.
    fn covering(bounds: tiny_skia::Rect, height: u32) -> Option<Self> {
        let rows = bounds.outset(0.0, 1.0)?.round_out()?;
        let limit = i32::try_from(height).ok()?;
        let top = rows.top().clamp(0, limit);
        let bottom = rows.bottom().clamp(0, limit);
        let rows = u32::try_from(bottom.checked_sub(top)?).ok()?;
        (rows > 0).then_some(Self {
            top: u32::try_from(top).ok()?,
            height: rows,
        })
    }

    /// Maps target coordinates into this band's coordinates.
    fn offset(self) -> Transform {
        #[expect(
            clippy::cast_precision_loss,
            reason = "rasterize rejects a target taller than MAX_EXTENT = 2^24, and every \
                      integer below that is exactly representable as f32"
        )]
        Transform::translate(0.0, -(self.top as f32))
    }

    /// Borrows this band's rows of `pixmap` as a pixmap in its own right.
    ///
    /// `None` only if the band does not lie within the pixmap, which
    /// [`Band::covering`] does not produce.
    fn rows(self, pixmap: &mut tiny_skia::Pixmap) -> Option<tiny_skia::PixmapMut<'_>> {
        let width = pixmap.width();
        let stride = (width as usize).checked_mul(4)?;
        let start = (self.top as usize).checked_mul(stride)?;
        let end = start.checked_add((self.height as usize).checked_mul(stride)?)?;
        let rows = pixmap.data_mut().get_mut(start..end)?;
        tiny_skia::PixmapMut::from_bytes(rows, width, self.height)
    }

    /// Bytes a mask covering this band of a `width`-wide target occupies.
    fn mask_bytes(self, width: u32) -> usize {
        (self.height as usize).saturating_mul(width as usize)
    }
}

/// One clip of a chain, ready to be drawn into a mask.
struct Shape {
    path: tiny_skia::Path,
    /// The clip's own transform; the target and band transforms are applied later,
    /// because the band is not known until every clip in the chain has been measured.
    transform: Transform,
    fill_rule: tiny_skia::FillRule,
}

/// A built clip mask and the band it covers.
struct Built {
    mask: tiny_skia::Mask,
    band: Band,
}

/// Builds and memoises clip masks, within a memory budget.
///
/// A clip commonly applies to thousands of consecutive commands, so rasterising its
/// mask once per command would dominate the render; masks are therefore built on first
/// use and kept.
///
/// The cache is bounded, which the memoisation alone is not. A document names as many
/// distinct clips as it likes — the corpus's worst holds 3576 on one page — so keeping
/// every mask is a memory-exhaustion vector: before this bound and before [`Band`],
/// that page held 1.7 GB of page-sized masks. Dropping an entry costs a rebuild and
/// nothing else, so eviction can be crude, and it is: entries go in build order,
/// oldest first, and the most recently built is never dropped. Clips are used in runs,
/// so build order tracks use order closely enough that an active clip is rebuilt at
/// most once per run.
struct MaskCache {
    target: TargetSpec,
    anti_alias: bool,
    /// Masks by clip. `None` records a clip that admits no row of the target, which is
    /// worth remembering rather than rediscovering: every command it clips draws
    /// nothing. These entries hold no pixels, and there is one at most per clip in the
    /// display list, so they are bounded by the list itself.
    built: HashMap<ClipId, Option<Built>>,
    /// Clips holding a mask, in build order, for eviction.
    order: VecDeque<ClipId>,
    /// Bytes held by the masks in `built`.
    bytes: usize,
    /// Largest total the masks may reach before the oldest are dropped.
    budget: usize,
}

/// Largest total size of the masks a [`MaskCache`] holds, in bytes.
///
/// Sized against the corpus rather than guessed. `bug1721218_reduced.pdf` is the heaviest
/// first page in the 974-document pdf.js corpus — 3576 distinct clips on one 612×792
/// page — and its banded masks come to 25.5 MB, so the heaviest real document known fits
/// with headroom and never evicts. A document that exceeds this is either far past
/// anything in the corpus or trying to exhaust memory, and either way is served correctly
/// at the price of rebuilding a mask it comes back to.
const MASK_BUDGET: usize = 32 << 20;

impl MaskCache {
    fn new(target: TargetSpec, anti_alias: bool) -> Self {
        Self {
            target,
            anti_alias,
            built: HashMap::new(),
            order: VecDeque::new(),
            bytes: 0,
            budget: MASK_BUDGET,
        }
    }

    /// Returns the mask for `id` and the band it covers, building it if needed.
    ///
    /// `None` means the clip admits no row of the target: the caller must draw
    /// nothing at all, which is not the same as drawing unclipped.
    fn get(
        &mut self,
        list: &DisplayList,
        id: ClipId,
    ) -> Result<Option<(Band, &tiny_skia::Mask)>, CpuRasterError> {
        if !self.built.contains_key(&id) {
            let chain = Self::resolve_chain(list, id)?;
            let built = self.build(list, id, &chain)?;
            self.admit(id, built);
        }

        // Absence here would mean `admit` dropped the entry it had just stored, which
        // it does not do; reporting it beats drawing an unclipped page if it ever did.
        let entry = self
            .built
            .get(&id)
            .ok_or(CpuRasterError::UnknownClip(id))?
            .as_ref();
        Ok(entry.map(|built| (built.band, &built.mask)))
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

    /// Builds the effective mask for a root-first chain, in the band it covers.
    ///
    /// The whole chain is drawn into one mask rather than each clip being cached and
    /// intersected with its parent's: a parent covers a different band from its child,
    /// so a parent's mask cannot be reused as a starting point. Building the chain
    /// costs its depth in band-sized fills, which is less than one page-sized copy.
    fn build(
        &self,
        list: &DisplayList,
        id: ClipId,
        chain: &[ClipId],
    ) -> Result<Option<Built>, CpuRasterError> {
        let mut shapes = Vec::with_capacity(chain.len());
        // The effective clip is the intersection of the chain, so it lies within the
        // intersection of the chain's bounds. Path bounds are control-point bounds,
        // which overstate a curve — the safe direction, since too large a band only
        // costs work while too small a one would erase pixels the clip admits.
        let mut bounds: Option<tiny_skia::Rect> = None;

        for &id in chain {
            let clip = list.clip(id).ok_or(CpuRasterError::UnknownClip(id))?;
            let path = convert::path(&clip.path).ok_or(CpuRasterError::InvalidPath)?;
            // A bound that overflows to infinity is left out of the measurement rather
            // than reported. The band is an optimisation over which rows may be marked,
            // so leaving a clip out of it only widens the band; what is drawn is decided
            // by the mask, which is built from the paths themselves either way.
            if let Some(device) = path.bounds().transform(convert::transform(
                clip.transform.then(self.target.transform),
            )) {
                bounds = match bounds {
                    None => Some(device),
                    // An empty intersection is an empty clip, not a failure.
                    Some(outer) => match outer.intersect(&device) {
                        Some(both) => Some(both),
                        None => return Ok(None),
                    },
                };
            }
            shapes.push(Shape {
                path,
                transform: clip.transform,
                fill_rule: convert::fill_rule(clip.fill_rule),
            });
        }

        // A chain always holds at least the clip it was resolved from, so this is
        // unreachable; an empty clip chain would mean "clipped by nothing", which the
        // caller expresses by having no clip at all, and silently drawing unclipped here
        // would be exactly the plausible-looking wrong page this backend refuses to
        // produce.
        let Some((root, nested)) = shapes.split_first() else {
            return Err(CpuRasterError::UnknownClip(id));
        };
        let band = match bounds {
            Some(bounds) => match Band::covering(bounds, self.target.height) {
                Some(band) => band,
                // The clip admits no row of the target at all.
                None => return Ok(None),
            },
            // Nothing in the chain could be measured, so nothing bounds the band.
            None => Band::whole(self.target),
        };

        let to_band = self.target.transform.then(band.offset());
        let mut mask = tiny_skia::Mask::new(self.target.width, band.height).ok_or(
            CpuRasterError::Allocation {
                width: self.target.width,
                height: band.height,
            },
        )?;
        // A fresh mask blocks everything, so filling the root path is what opens it.
        mask.fill_path(
            &root.path,
            root.fill_rule,
            self.anti_alias,
            convert::transform(root.transform.then(to_band)),
        );
        for shape in nested {
            mask.intersect_path(
                &shape.path,
                shape.fill_rule,
                self.anti_alias,
                convert::transform(shape.transform.then(to_band)),
            );
        }

        Ok(Some(Built { mask, band }))
    }

    /// Stores an entry and evicts oldest-first until the budget is met.
    fn admit(&mut self, id: ClipId, built: Option<Built>) {
        if let Some(entry) = &built {
            self.bytes = self
                .bytes
                .saturating_add(entry.band.mask_bytes(self.target.width));
            self.order.push_back(id);
        }
        self.built.insert(id, built);

        // The entry just built is the last in `order` and the caller is about to draw
        // with it, so eviction stops before reaching it even if one mask alone exceeds
        // the budget. A budget that cannot hold the mask in hand is not a reason to
        // fail the page.
        while self.bytes > self.budget && self.order.len() > 1 {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(Some(entry)) = self.built.remove(&oldest) {
                self.bytes = self
                    .bytes
                    .saturating_sub(entry.band.mask_bytes(self.target.width));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use pdf_render::{
        Clip, ClipId, DisplayList, FillRule, Path, PathCommand, Point, Size, TargetSpec, Transform,
    };

    use super::{Band, MASK_BUDGET, MaskCache};

    /// A page carrying `count` clips, each a thin horizontal bar at its own height.
    ///
    /// Thin so that one mask is small and a budget of a few of them is easy to state.
    fn stacked_clips(count: u16) -> (DisplayList, Vec<ClipId>, TargetSpec) {
        let mut list = DisplayList::new(Size::new(200.0, 200.0));
        let mut ids = Vec::with_capacity(usize::from(count));
        for index in 0..count {
            let y = f32::from(index) * 2.0;
            let mut path = Path::new();
            path.push(PathCommand::MoveTo(Point::new(0.0, y)));
            path.push(PathCommand::LineTo(Point::new(200.0, y)));
            path.push(PathCommand::LineTo(Point::new(200.0, y + 1.0)));
            path.push(PathCommand::LineTo(Point::new(0.0, y + 1.0)));
            path.push(PathCommand::Close);
            ids.push(
                list.add_clip(Clip {
                    path,
                    transform: Transform::IDENTITY,
                    fill_rule: FillRule::NonZero,
                    parent: None,
                })
                .expect("under the clip limit"),
            );
        }
        let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("valid target");
        (list, ids, target)
    }

    /// The bound is the whole point of the cache, so it is checked directly rather than
    /// inferred from a render: a page whose masks happen to fit says nothing about one
    /// whose do not, and the documents that do not fit are the hostile ones.
    #[test]
    fn the_mask_cache_stays_inside_its_budget() {
        let (list, ids, target) = stacked_clips(40);
        let mut cache = MaskCache::new(target, true);
        // Room for two of these masks and no more.
        let one = Band { top: 0, height: 4 }.mask_bytes(target.width);
        cache.budget = one * 2;

        for (drawn, &id) in ids.iter().enumerate() {
            cache.get(&list, id).expect("a rectangular clip builds");
            assert!(
                cache.bytes <= cache.budget.max(one),
                "after {} clips the cache holds {} bytes, over a budget of {}",
                drawn + 1,
                cache.bytes,
                cache.budget
            );
        }
        assert!(
            cache.built.len() < ids.len(),
            "nothing was evicted, so the budget was never exercised"
        );
    }

    /// Rebuilding an evicted mask must produce the mask that was dropped.
    ///
    /// Eviction is only safe because it costs a rebuild and nothing else. If a rebuilt
    /// mask differed, how much memory happened to be available would decide what the
    /// page looks like.
    #[test]
    fn a_rebuilt_mask_is_the_mask_that_was_evicted() {
        let (list, ids, target) = stacked_clips(8);
        let first = *ids.first().expect("eight clips");

        let mut generous = MaskCache::new(target, true);
        let before = generous
            .get(&list, first)
            .expect("builds")
            .map(|(band, mask)| (band, mask.data().to_vec()));

        let mut tight = MaskCache::new(target, true);
        tight.budget = 1;
        for &id in &ids {
            tight.get(&list, id).expect("builds");
        }
        let after = tight
            .get(&list, first)
            .expect("rebuilds")
            .map(|(band, mask)| (band, mask.data().to_vec()));

        assert!(before.is_some(), "the clip covers rows of the target");
        assert_eq!(before, after, "a rebuilt mask differs from the original");
    }

    /// A clip that admits no row is remembered as such, not rebuilt on every command.
    #[test]
    fn a_clip_off_the_target_is_cached_as_admitting_nothing() {
        let mut list = DisplayList::new(Size::new(200.0, 200.0));
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(0.0, 400.0)));
        path.push(PathCommand::LineTo(Point::new(200.0, 400.0)));
        path.push(PathCommand::LineTo(Point::new(200.0, 500.0)));
        path.push(PathCommand::LineTo(Point::new(0.0, 500.0)));
        path.push(PathCommand::Close);
        let id = list
            .add_clip(Clip {
                path,
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                parent: None,
            })
            .expect("first clip");
        let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("valid target");

        let mut cache = MaskCache::new(target, true);
        assert!(
            cache.get(&list, id).expect("resolves").is_none(),
            "a clip entirely above the page admits no row"
        );
        assert_eq!(cache.bytes, 0, "an empty clip holds no pixels");
        assert!(
            cache.built.contains_key(&id),
            "the emptiness is remembered rather than rediscovered per command"
        );
    }

    /// The shipped budget is stated in the type, so a careless edit to it fails here.
    #[test]
    fn the_shipped_budget_is_thirty_two_mebibytes() {
        assert_eq!(MASK_BUDGET, 32 * 1024 * 1024);
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
