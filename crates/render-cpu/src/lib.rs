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

mod blend;
mod convert;
mod shading;

use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};
use rayon::slice::ParallelSliceMut as _;
use std::collections::{HashMap, HashSet, VecDeque};

use pdf_render::{
    BackendError, ClipId, Color, Command, DisplayList, MAX_EXTENT, MAX_GROUP_DEPTH, Paint, Path,
    Raster, RasterFormat, Rasterizer, SoftMaskId, Stroke, TargetSpec, Transform, impose_on_medium,
};

/// Renders display lists on the CPU.
#[derive(Debug, Clone)]
pub struct CpuRasterizer {
    background: Color,
    anti_alias: bool,
    strips: Option<u32>,
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
            strips: None,
        }
    }

    /// Asks for a fixed number of horizontal strips instead of one per available core.
    ///
    /// **This changes how long a page takes and not what it looks like**, which is the whole
    /// claim of [`CpuRasterizer::encode_in_strips`] and is why the knob is public: the property
    /// is only checkable by rendering one page several ways and comparing the bytes, which is
    /// what `strip_parallelism.rs` does. A page still gets fewer strips than asked for where
    /// its curves forbid the cuts.
    #[must_use]
    pub fn with_strips(mut self, strips: u32) -> Self {
        self.strips = Some(strips);
        self
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
        blend: tiny_skia::BlendMode,
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
            blend_mode: blend,
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

        self.encode_in_strips(&mut pixmap, list, target)?;

        // §11.4.7's page group is isolated, so the medium's colour is composited with the
        // finished page rather than being the backdrop its blend modes saw. Before the
        // conversion below, because `tiny-skia`'s pixels are premultiplied here and that is
        // where the composite is exact.
        //
        // Both this and the conversion below are per-pixel and independent, so they are run
        // across the same rows the strips used. **They are not an afterthought**: on a
        // 1192×1684 page they were together a third of what was left after the drawing was
        // divided, and a serial third is a 3× ceiling however many strips the page grants.
        // Splitting a per-pixel pass changes no byte, which is why it needs no rule of its
        // own.
        // Per-pixel and independent, so it is split across the same rows the strips used.
        // Splitting a per-pixel pass changes no byte, which is why it needs no rule of its
        // own; it is here because after the drawing is divided a serial pass over every pixel
        // is what is left to bound the speed-up.
        let stride = (target.width as usize).saturating_mul(4).max(4);
        pixmap
            .data_mut()
            .par_chunks_mut(stride.saturating_mul(PIXEL_PASS_ROWS))
            .for_each(|rows| impose_on_medium(rows, self.background));

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

/// Most horizontal strips one page is divided into, whatever the machine offers.
///
/// A strip carries its own mask cache, its own group buffers and its own soft masks, so the
/// per-strip constants are what bound this rather than the thread count: sixteen strips of a
/// 842-row page are 52 rows apiece, which is shorter than many clip bands. The cores beyond it
/// are not idle in a viewer — pages are rendered on their own threads above this one.
const MAX_STRIPS: u32 = 16;

/// Most work a split may replay, as a multiple of the command list itself.
///
/// See [`pdf_render::replay_ratio`]: a command reaching two strips is built, bounded and
/// pipelined twice, and on a page of a few page-wide commands that duplication is the whole
/// render. A quarter more is what a page of small marks costs and a page of page-wide ones
/// cannot reach.
const MAX_REPLAY: f64 = 1.25;

/// Rows a per-pixel pass over the finished page hands one thread at a time.
///
/// Large enough that a page under this many rows is done on one thread, which is where the
/// scheduling would cost more than the work.
const PIXEL_PASS_ROWS: usize = 64;

/// Fewest rows a strip may have, below which the target is drawn serially.
///
/// A strip pays for a mask cache and a replay of the command list, and a page thumbnail is not
/// worth either.
const MIN_STRIP_ROWS: u32 = 64;

impl CpuRasterizer {
    /// Draws the whole list into `pixmap`, in parallel where the page permits it exactly.
    ///
    /// # Why a strip is not simply a band
    ///
    /// [`Band`] already restricts a command to the rows its clip admits, and cutting the page
    /// into runs of rows and replaying the list into each is the same geometry one level up
    /// (ADR 0137). What makes it a decision rather than a refactor is that **it is only exact
    /// at some rows**: rasterising a path into a surface that does not contain it chops the
    /// path against the surface's edge, and a *curve* chopped at an edge is re-parameterised,
    /// so its coverage differs from the unclipped curve's by up to a quarter of a channel.
    /// ADR 0138 shipped nothing for that reason — four oracle pages stopped agreeing with the
    /// reference consensus — and ADR 0139 is the probe that says exactly where the difference
    /// is: a cut at a row no curve crosses is **bit-identical** to the serial render, and one
    /// at a row a curve crosses is not.
    ///
    /// So [`pdf_render::unsplittable_rows`] names the rows a curve crosses and
    /// [`pdf_render::strip_boundaries_avoiding`] cuts only at the others. A page that grants
    /// none — one wide gradient under one curved clip, `bug1721218_reduced.pdf` — is drawn
    /// serially, which is not a fallback but the only division of it that draws the same page.
    ///
    /// # Errors
    ///
    /// As [`CpuRasterizer::encode`]. A strip that fails takes the render with it: half a page
    /// is not a result.
    fn encode_in_strips(
        &self,
        pixmap: &mut tiny_skia::Pixmap,
        list: &DisplayList,
        target: TargetSpec,
    ) -> Result<(), CpuRasterError> {
        let boundaries = plan_strips(list, target, self.strips);
        let strips = boundaries.len().saturating_sub(1);
        if strips < 2 {
            let mut masks = MaskCache::new(target, self.anti_alias, MASK_BUDGET);
            return self.encode(
                &mut pixmap.as_mut(),
                list,
                list.commands(),
                target,
                &mut masks,
                0,
                Compose::Over,
            );
        }

        // The budget is divided rather than multiplied: the masks of a strip are a strip tall,
        // so the same total memory buys the same coverage of the page it did serially.
        let budget = MASK_BUDGET.checked_div(strips).unwrap_or(MASK_BUDGET);
        let width = pixmap.width();
        let stride = (width as usize).saturating_mul(4);
        let mut rest = pixmap.data_mut();
        let mut pieces = Vec::with_capacity(strips);
        for pair in boundaries.windows(2) {
            let (top, bottom) = (pair.first().copied().unwrap_or(0), pair.get(1).copied());
            let Some(bottom) = bottom else { break };
            let rows = bottom.saturating_sub(top);
            let (piece, tail) =
                rest.split_at_mut((rows as usize).saturating_mul(stride).min(rest.len()));
            rest = tail;
            pieces.push((top, rows, piece));
        }

        pieces
            .into_par_iter()
            .try_for_each(|(top, rows, piece)| -> Result<(), CpuRasterError> {
                let spec = TargetSpec {
                    width,
                    height: rows,
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "rasterize rejects a target taller than MAX_EXTENT = 2^24, \
                                  every integer below which is exact in f32"
                    )]
                    transform: target
                        .transform
                        .then(Transform::translate(0.0, -(top as f32))),
                };
                let mut surface = tiny_skia::PixmapMut::from_bytes(piece, width, rows).ok_or(
                    CpuRasterError::Allocation {
                        width,
                        height: rows,
                    },
                )?;
                let mut masks = MaskCache::new(spec, self.anti_alias, budget);
                self.encode(
                    &mut surface,
                    list,
                    list.commands(),
                    spec,
                    &mut masks,
                    0,
                    Compose::Over,
                )
            })
    }

    /// Turns a dash pattern's zero-length dashes into marks, ISO 32000-2 §8.5.3.2.
    ///
    /// > This rule shall apply only to zero-length subpaths of the path being stroked, and
    /// > not to zero-length dashes in a dash pattern of a non-degenerate subpath. In the
    /// > latter case, the line caps shall always be painted, since their orientation is
    /// > determined by the direction of the underlying path
    ///
    /// `tiny-skia` paints such a cap, so a dotted line is *drawn* here without this — but it
    /// faces a square cap upright, because Skia's dasher loses the direction and its stroker
    /// says so: "since the zero length segment has no direction, set the orientation to
    /// upright as the default orientation". The clause says the direction is the path's, and
    /// on a diagonal dotted line the two answers cover different pixels. Doing the dashing
    /// here keeps the direction (see [`pdf_render::ZERO_DASH`]) and puts the answer in the
    /// crate both backends share.
    ///
    /// Returns `None` when the pattern holds no zero-length dash whose cap would show, which
    /// leaves the ordinary path untouched: `tiny-skia` does its own dashing and this
    /// allocates nothing.
    fn zero_length_dashes(
        geometry: &Path,
        stroke: &Stroke,
        width: f32,
        dots: &mut Path,
    ) -> Option<Path> {
        let pattern = pdf_render::dashes_showing_direction(&stroke.dash_array, stroke.cap)?;
        let source = convert::path(geometry)?;
        let dash = tiny_skia::StrokeDash::new(pattern, stroke.dash_phase)?;
        // The resolution scale bounds how finely a curve is measured for arc length.
        // `tiny-skia` derives it from the transform for stroking; one is what it uses for a
        // path already in device-sized units, and a dash's *position* is what this needs
        // rather than the smoothness of its ends.
        let dashed = source.dash(&dash, 1.0)?;
        let split =
            pdf_render::split_dash_marks(&convert::from_skia_path(&dashed), stroke.cap, width);
        dots.extend(split.dots.commands());
        Some(split.stroked)
    }

    /// Draws a stroked path, including the marks its own geometry has no length to make.
    ///
    /// Split out of [`CpuRasterizer::draw`] because ISO 32000-2 §8.5.3.2 turns one command
    /// into two draws — the subpaths that span a distance are stroked, and the ones that do
    /// not are *filled*, as circles `pdf-render` states rather than as whatever `tiny-skia`'s
    /// caps would have produced.
    ///
    /// # Errors
    ///
    /// Returns [`CpuRasterError`] for a path the rasteriser rejects or a paint this backend
    /// does not implement.
    #[expect(
        clippy::too_many_arguments,
        reason = "the fields of one display list command, passed through rather than \
                  regrouped: a struct here would be a second spelling of `Command::Stroke`"
    )]
    fn draw_stroke(
        &self,
        surface: &mut tiny_skia::PixmapMut<'_>,
        path: &Path,
        transform: Transform,
        stroke: &Stroke,
        paint: &Paint,
        blend: tiny_skia::BlendMode,
        to_device: Transform,
        clip: Option<&tiny_skia::Mask>,
    ) -> Result<(), CpuRasterError> {
        let at = transform.then(to_device);
        let width = stroke.device_width(at);
        // ISO 32000-2 §8.5.3.2's two rules about a stroke with no length. Neither is
        // Skia's answer: it paints a projecting square cap where the clause asks for
        // no output, it refuses a path that is only a `m` rather than drawing
        // nothing, and it faces a zero-length dash's square cap upright rather than
        // along the path. Both are decided in `pdf-render` so that the two backends
        // cannot answer them differently.
        let split = pdf_render::split_degenerate(path, stroke.cap, width);
        let geometry = split.as_ref().map_or(path, |s| &s.stroked);
        let mut dots = split.as_ref().map_or_else(Path::new, |s| s.dots.clone());
        let (geometry, dashed) = match Self::zero_length_dashes(geometry, stroke, width, &mut dots)
        {
            Some(remainder) => (remainder, true),
            None => (geometry.clone(), false),
        };

        let mut scratch = None;
        if !geometry.is_empty() {
            let converted = convert::path(&geometry).ok_or(CpuRasterError::InvalidPath)?;
            let mut style = convert::stroke(stroke, at);
            // The dashes have already been dispensed, so what is left is solid.
            if dashed {
                style.dash = None;
            }
            surface.stroke_path(
                &converted,
                &self.paint(paint, blend, page_to_path(transform)?, &mut scratch)?,
                &style,
                convert::transform(at),
                clip,
            );
        }
        // The marks are *filled*, not stroked: §8.5.3.2 asks for "a filled circle
        // centred at the single point", and the stroking paint is what fills it.
        if !dots.is_empty()
            && let Some(converted) = convert::path(&dots)
        {
            let mut scratch = None;
            surface.fill_path(
                &converted,
                &self.paint(paint, blend, page_to_path(transform)?, &mut scratch)?,
                tiny_skia::FillRule::Winding,
                convert::transform(at),
                clip,
            );
        }
        Ok(())
    }

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
    #[expect(
        clippy::too_many_arguments,
        reason = "a display list, where it is drawn, and the two things a recursion carries \
                  — how deep it is and how its elements combine. Grouping them would put \
                  the target and the mask cache in a struct that exists once per call"
    )]
    fn encode(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        list: &DisplayList,
        commands: &[Command],
        target: TargetSpec,
        masks: &mut MaskCache,
        depth: usize,
        compose: Compose,
    ) -> Result<(), CpuRasterError> {
        for command in commands {
            // A command whose extent misses this target marks nothing, and saying so here is
            // what makes a strip cost what its own rows cost: without it every strip would
            // build every command's path and compile every command's pipeline, which session
            // 154 measured at 19% of a dense page's rasterisation. A row of margin, for the
            // same reason `Band::covering` takes one — the extent comes from control points
            // and the mask from the path.
            if misses_target(command, target) {
                continue;
            }

            // A soft mask is evaluated before anything borrows the cache, because building
            // it renders a whole command list of its own and so needs the cache mutably.
            // Idempotent: the second command under the same mask finds it already there.
            if let Some(id) = command.mask() {
                self.build_soft_mask(list, id, target, masks, depth)?;
            }

            // A group is the one command that needs the mask cache mutably *while* it
            // draws, so it cannot hold a clip mask borrowed from it across the recursion.
            // It therefore resolves its own clip, twice, either side of its elements.
            if let Command::Group {
                commands,
                alpha,
                blend,
                knockout,
                ..
            } = command
            {
                // A group inside a knockout group would have to be composited by its
                // *shape*, which reaches this backend as one alpha channel and so cannot be
                // told from its opacity. `pdf-model` does not build that display list — see
                // `Command::Group`'s `knockout` — and erroring is what keeps the assumption
                // from becoming a silent approximation if it ever does.
                if compose == Compose::Knockout {
                    return Err(CpuRasterError::UnsupportedCommand(
                        "a group inside a knockout group has a shape this backend cannot \
                         separate from its opacity (ISO 32000-2 §11.4.6)"
                            .to_owned(),
                    ));
                }
                self.draw_group(
                    pixmap,
                    list,
                    Group {
                        commands,
                        alpha: *alpha,
                        blend: *blend,
                        clip: command.clip(),
                        mask: command.mask(),
                        compose: if *knockout {
                            Compose::Knockout
                        } else {
                            Compose::Over
                        },
                    },
                    target,
                    masks,
                    depth,
                )?;
                continue;
            }

            // Resolved before the match so that every arm shares one code path for
            // clip handling; a per-arm lookup would be a place for them to diverge.
            // The clip admits no row of the target, so nothing this command draws can
            // survive it.
            let Some((band, clip)) = masks.effective(list, command.clip(), command.mask())? else {
                continue;
            };

            // Everything below draws into the band rather than the page, which is what
            // keeps a command's cost proportional to the pixels its clip can admit.
            // The device transform carries the band's offset so that geometry, paints
            // and images all move together; missing one would tear the page apart in a
            // way no metric would notice, so there is exactly one of these.
            let to_device = target.transform.then(band.offset());

            // ISO 32000-2 §11.3.5.3's four modes are computed by this backend rather than
            // by `tiny-skia`, whose three of them are wrong (ADR 0047), so such a command
            // is drawn onto transparency first and composited by `blend::composite`.
            // Drawing onto transparency loses nothing: with αb = 0 the compositing formula
            // collapses to the source, whatever the blend mode.
            if let Some(mode) = compose.non_separable(command.blend()) {
                let mut layer = tiny_skia::Pixmap::new(target.width, band.height).ok_or(
                    CpuRasterError::Allocation {
                        width: target.width,
                        height: band.height,
                    },
                )?;
                self.draw(&mut layer.as_mut(), command, to_device, clip, compose)?;
                let mut surface = band.rows(pixmap).ok_or(CpuRasterError::Allocation {
                    width: target.width,
                    height: band.height,
                })?;
                blend::composite(&mut surface, &layer, mode);
                continue;
            }

            let mut surface = band.rows(pixmap).ok_or(CpuRasterError::Allocation {
                width: target.width,
                height: band.height,
            })?;

            self.draw(&mut surface, command, to_device, clip, compose)?;
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
        pixmap: &mut tiny_skia::PixmapMut<'_>,
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
        let Some((band, _)) = masks.effective(list, group.clip, group.mask)? else {
            return Ok(());
        };

        let mut buffer = tiny_skia::Pixmap::new(target.width, target.height).ok_or(
            CpuRasterError::Allocation {
                width: target.width,
                height: target.height,
            },
        )?;
        self.encode(
            &mut buffer.as_mut(),
            list,
            group.commands,
            target,
            masks,
            depth,
            group.compose,
        )?;

        // The elements may have evicted the soft mask this group is painted through, since
        // they share the cache; rebuilding it is what makes eviction safe here as it is for
        // a clip.
        if let Some(id) = group.mask {
            self.build_soft_mask(list, id, target, masks, depth)?;
        }

        // Resolved again rather than held across the recursion: the elements' own clips
        // share this cache and may have evicted the entry, and a rebuilt mask is the mask
        // that was dropped — see `a_rebuilt_mask_is_the_mask_that_was_evicted`.
        let Some((_, clip)) = masks.effective(list, group.clip, group.mask)? else {
            return Ok(());
        };

        let paint = tiny_skia::PixmapPaint {
            opacity: group.alpha.clamp(0.0, 1.0),
            blend_mode: convert::blend_mode(group.blend),
            // The buffer is drawn at 1:1 with no transform, so no sample is ever
            // interpolated and the quality setting cannot change a pixel.
            quality: tiny_skia::FilterQuality::Nearest,
        };
        // Negative, because the buffer covers the whole target and the surface starts at
        // the band's first row.
        let top = i32::try_from(band.top).map_err(|_| CpuRasterError::Allocation {
            width: target.width,
            height: band.height,
        })?;

        // §11.3.5.3's four modes are this backend's own (ADR 0047), and a group reaches
        // them by the same two steps a command does: the group's clip, alpha and offset
        // are applied onto transparency, and the result is composited by `blend`. The
        // extra buffer is the price of the mode, not of every group.
        if let Some(mode) = blend::NonSeparable::of(group.blend) {
            let mut layer = tiny_skia::Pixmap::new(target.width, band.height).ok_or(
                CpuRasterError::Allocation {
                    width: target.width,
                    height: band.height,
                },
            )?;
            layer.as_mut().draw_pixmap(
                0,
                top.saturating_neg(),
                buffer.as_ref(),
                &tiny_skia::PixmapPaint {
                    blend_mode: tiny_skia::BlendMode::SourceOver,
                    ..paint
                },
                tiny_skia::Transform::identity(),
                clip,
            );
            let mut surface = band.rows(pixmap).ok_or(CpuRasterError::Allocation {
                width: target.width,
                height: band.height,
            })?;
            blend::composite(&mut surface, &layer, mode);
            return Ok(());
        }

        let mut surface = band.rows(pixmap).ok_or(CpuRasterError::Allocation {
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

    /// Evaluates a soft mask into the cache, if it is not there already (§11.5).
    ///
    /// The mask's group is drawn onto a fully transparent target-sized surface — the same
    /// isolated backdrop [`CpuRasterizer::draw_group`] uses, and what both §11.5.2 and
    /// §11.5.3 ask for — and each pixel is then turned into a mask value by
    /// [`pdf_render::SoftMask::value`], which is the function the GPU backend calls on its
    /// own readback. That shared derivation is the whole reason the display list carries a
    /// mask's *commands* rather than a raster: the group is evaluated at device resolution,
    /// which only a backend knows, while what the pixels mean is decided once for both.
    ///
    /// # Errors
    ///
    /// As [`CpuRasterizer::encode`], plus [`CpuRasterError::UnknownSoftMask`] for an
    /// identifier this display list does not hold.
    fn build_soft_mask(
        &self,
        list: &DisplayList,
        id: SoftMaskId,
        target: TargetSpec,
        masks: &mut MaskCache,
        depth: usize,
    ) -> Result<(), CpuRasterError> {
        if masks.holds_soft_mask(id) {
            return Ok(());
        }
        let mask = list
            .soft_mask(id)
            .ok_or(CpuRasterError::UnknownSoftMask(id))?;

        let mut buffer = tiny_skia::Pixmap::new(target.width, target.height).ok_or(
            CpuRasterError::Allocation {
                width: target.width,
                height: target.height,
            },
        )?;
        // A soft mask's group is evaluated as §11.4.5's ordinary group: `SoftMask` carries
        // no knockout flag, and `pdf-model` reports a mask group that asks for one.
        self.encode(
            &mut buffer.as_mut(),
            list,
            &mask.commands,
            target,
            masks,
            depth,
            Compose::Over,
        )?;

        // Straight alpha, which is what `SoftMask::value` is defined over and what the GPU
        // backend reads back; `tiny-skia` stores premultiplied, so the conversion happens
        // here at the same boundary as every other one in this backend.
        let values = mask.values(&buffer.take_demultiplied());
        let built = tiny_skia::Mask::from_vec(
            values,
            tiny_skia::IntSize::from_wh(target.width, target.height).ok_or(
                CpuRasterError::Allocation {
                    width: target.width,
                    height: target.height,
                },
            )?,
        )
        .ok_or(CpuRasterError::Allocation {
            width: target.width,
            height: target.height,
        })?;
        masks.admit_soft_mask(id, built, target);
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
        compose: Compose,
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
                    && let pdf_render::ShadingKind::Mesh { triangles } = shading.kind.as_ref()
                {
                    shading::fill_mesh(
                        surface,
                        &path,
                        triangles,
                        shading.transform.then(to_device),
                        convert::fill_rule(*fill_rule),
                        convert::transform(transform.then(to_device)),
                        clip,
                        compose.mode(*blend),
                        self.anti_alias,
                    );
                    return Ok(());
                }

                // A sampled shading's pixels are borrowed by its shader, so they need
                // somewhere to live for exactly as long as this call.
                let mut scratch = None;
                surface.fill_path(
                    &path,
                    &self.paint(
                        paint,
                        compose.mode(*blend),
                        page_to_path(*transform)?,
                        &mut scratch,
                    )?,
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
                self.draw_stroke(
                    surface,
                    path,
                    *transform,
                    stroke,
                    paint,
                    compose.mode(*blend),
                    to_device,
                    clip,
                )?;
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
                    blend: compose.mode(*blend),
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

/// How an element of a group combines with the elements drawn before it.
///
/// ISO 32000-2 §11.4.6's two answers. The ordinary one is over, and a knockout group's is
/// that "each individual element shall be composited with the group's initial backdrop
/// rather than with the stack of preceding elements in the group" — which, onto the
/// transparent backdrop [`CpuRasterizer::draw_group`] builds, is the element itself
/// replacing what is under it within its own coverage. That is Porter-Duff Source, which
/// `tiny-skia` applies through the same coverage and clip mask as any other mode, so a
/// knockout element needs no second raster and no shape channel.
///
/// An element's *own* blend mode disappears under knockout, and that is the clause rather
/// than a shortcut: its backdrop is transparent, and §11.3.6's formula with αb = 0 is the
/// source colour whatever the blend function is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Compose {
    /// §11.4.5: an element paints over the group's accumulated result.
    Over,
    /// §11.4.6: an element replaces it within its own coverage.
    Knockout,
}

impl Compose {
    /// The `tiny-skia` mode an element painted under `blend` is drawn with.
    fn mode(self, blend: pdf_render::BlendMode) -> tiny_skia::BlendMode {
        match self {
            Self::Over => convert::blend_mode(blend),
            Self::Knockout => tiny_skia::BlendMode::Source,
        }
    }

    /// Whether §11.3.5.3's four modes still need this backend's own compositing.
    ///
    /// Never under knockout: the element has no backdrop to blend with.
    fn non_separable(self, blend: pdf_render::BlendMode) -> Option<blend::NonSeparable> {
        match self {
            Self::Over => blend::NonSeparable::of(blend),
            Self::Knockout => None,
        }
    }
}

/// One transparency group, unpacked from its command.
///
/// Grouped for the same reason as [`ImagePlacement`]: five values that always travel
/// together, and a call site that would otherwise be a row of unlabelled arguments.
#[derive(Debug, Clone, Copy)]
struct Group<'a> {
    commands: &'a [Command],
    alpha: f32,
    blend: pdf_render::BlendMode,
    clip: Option<ClipId>,
    mask: Option<SoftMaskId>,
    /// How this group's own elements combine with each other (§11.4.6).
    compose: Compose,
}

/// Where and how an image is placed.
///
/// Grouped because these four always travel together, and passing them separately made
/// the call site a row of unlabelled arguments.
#[derive(Debug, Clone, Copy)]
struct ImagePlacement {
    transform: Transform,
    alpha: f32,
    blend: tiny_skia::BlendMode,
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
            blend_mode: blend,
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
    fn rows<'a>(
        self,
        pixmap: &'a mut tiny_skia::PixmapMut<'_>,
    ) -> Option<tiny_skia::PixmapMut<'a>> {
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

/// Whether a command's own extent lies wholly above or below `target`'s rows.
///
/// A group answers `false`: its elements carry the extents and each is asked in turn.
fn misses_target(command: &Command, target: TargetSpec) -> bool {
    let Some(bounds) = command.device_bounds(target.transform) else {
        return false;
    };
    #[expect(
        clippy::cast_precision_loss,
        reason = "a target height below MAX_EXTENT = 2^24 is exact in f32"
    )]
    let height = target.height as f32;
    bounds.max.y < -1.0 || bounds.min.y > height + 1.0
}

/// The rows at which to cut this target, or one strip's worth if it may not be cut.
///
/// The strip count asked for is what this machine offers, bounded by [`MAX_STRIPS`] and by
/// [`MIN_STRIP_ROWS`]; what comes back may be fewer, because a cut is only made at a row no
/// curve crosses. **The picture does not depend on the answer**, which is the property ADR 0139
/// exists to establish and `strip_parallelism.rs` asserts: a machine with four cores and one
/// with thirty-two draw the same bytes.
fn plan_strips(list: &DisplayList, target: TargetSpec, asked: Option<u32>) -> Vec<u32> {
    let cores = std::thread::available_parallelism()
        .map_or(1, std::num::NonZero::get)
        .try_into()
        .unwrap_or(MAX_STRIPS);
    let wanted = asked
        .unwrap_or(cores)
        .min(MAX_STRIPS)
        .min(target.height.checked_div(MIN_STRIP_ROWS).unwrap_or(0));
    if wanted < 2 {
        return vec![0, target.height];
    }
    let extents = pdf_render::command_extents(list, target);
    let costs = pdf_render::row_costs(&extents, target);
    let unsplittable = pdf_render::unsplittable_rows(list, target);

    // Asked for from the most strips downwards, taking the first division whose replay is
    // affordable. A strip pays again for every command that reaches it — see
    // `pdf_render::replay_ratio` — and on a page of a few page-wide commands that is the whole
    // cost: `issue12841_reduced.pdf` is two of them and was 105 ms serially against 166 ms in
    // sixteen strips. The bound is a quarter more work than the list itself, which every text
    // page measured in ADR 0137 sits far inside (1.01 to 1.13 at eight strips) and a page of
    // page-wide commands cannot reach at any count above one.
    let mut count = wanted;
    while count > 1 {
        let boundaries =
            pdf_render::strip_boundaries_avoiding(&costs, &unsplittable, count, MIN_STRIP_ROWS);
        if pdf_render::replay_ratio(&extents, &boundaries) <= MAX_REPLAY {
            return boundaries;
        }
        count = count.checked_div(2).unwrap_or(0);
    }
    vec![0, target.height]
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

/// What a cached mask was built from.
///
/// Three kinds share one cache because they share one budget and one eviction order, and
/// because what a command asks for is the *effective* mask: a clip, a soft mask, or their
/// product. Keeping the product under its own key is what stops a page of masked text from
/// multiplying the two per glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    /// The intersection of a clip chain.
    Clip(ClipId),
    /// A soft mask's values, over the whole target (§11.5).
    Soft(SoftMaskId),
    /// A clip and a soft mask multiplied together.
    ///
    /// §11.5.1's NOTE 2 makes that the right arithmetic rather than a shortcut: a hard clip
    /// "can be represented as a soft clip having shape values of 1.0 inside and 0.0 outside
    /// the clipping path", so intersecting the two is multiplying them.
    Both(ClipId, SoftMaskId),
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
    /// Masks by what they were built from. `None` records a clip that admits no row of the
    /// target, which is worth remembering rather than rediscovering: every command it clips
    /// draws nothing. These entries hold no pixels, and there is one at most per clip in the
    /// display list, so they are bounded by the list itself.
    built: HashMap<Key, Option<Built>>,
    /// Keys holding a mask, in build order, for eviction. Soft masks are not among them —
    /// see [`MaskCache::admit_soft_mask`] for why they are evicted on their own terms.
    order: VecDeque<Key>,
    /// Soft masks holding a raster, in build order, for eviction.
    soft_order: VecDeque<SoftMaskId>,
    /// Bytes held by the soft masks in `built`.
    soft_bytes: usize,
    /// Bytes held by the masks in `built`.
    bytes: usize,
    /// Largest total the masks may reach before the oldest are dropped.
    budget: usize,
}

/// Largest total size of the masks a [`MaskCache`] holds, in bytes.
///
/// Sized against the corpus rather than guessed. `bug1721218_reduced.pdf` is the heaviest
/// first page in the 974-document pdf.js corpus, and it never evicts: **3608 chains built
/// per render of one 612×792 page, peaking at 27.9 MB against this 32 MB** — measured in the
/// hundred-and-thirteenth session by counting admissions, evictions and peak bytes, where an
/// earlier version of this comment said "3576 distinct clips" and "25.5 MB" and called the
/// margin headroom. **It is 13%.** The next document heavier than this one will evict, and
/// will be served correctly at the price of rebuilding a mask it comes back to.
///
/// The number is not raised on that evidence, because nothing has been measured to be paying
/// for it; it is recorded so that a session which finds a document evicting knows the margin
/// was already thin rather than that something regressed.
const MASK_BUDGET: usize = 32 << 20;

impl MaskCache {
    /// A cache for one target, bounded by `budget` bytes.
    ///
    /// The budget is a parameter rather than [`MASK_BUDGET`] itself because a page drawn in
    /// strips has one cache per strip and they are alive at once: dividing the constant keeps
    /// a parallel render's mask memory equal to a serial one's.
    fn new(target: TargetSpec, anti_alias: bool, budget: usize) -> Self {
        Self {
            target,
            anti_alias,
            built: HashMap::new(),
            order: VecDeque::new(),
            soft_order: VecDeque::new(),
            bytes: 0,
            soft_bytes: 0,
            budget,
        }
    }

    /// Returns the mask a command with this clip and this soft mask draws through.
    ///
    /// `None` means nothing this command draws can survive, which is not the same as
    /// drawing unmasked; `Some((band, None))` is the unmasked case, where the band is the
    /// whole target.
    ///
    /// A soft mask must already have been evaluated by
    /// [`CpuRasterizer::build_soft_mask`] — building one means rendering a command list,
    /// which a cache cannot do.
    ///
    /// # Errors
    ///
    /// As [`MaskCache::get`], plus [`CpuRasterError::UnknownSoftMask`] for a soft mask that
    /// was not evaluated first.
    fn effective(
        &mut self,
        list: &DisplayList,
        clip: Option<ClipId>,
        mask: Option<SoftMaskId>,
    ) -> Result<Option<(Band, Option<&tiny_skia::Mask>)>, CpuRasterError> {
        match (clip, mask) {
            (None, None) => Ok(Some((Band::whole(self.target), None))),
            (Some(clip), None) => Ok(self.get(list, clip)?.map(|(band, mask)| (band, Some(mask)))),
            (None, Some(mask)) => {
                let entry = self.soft_mask(mask)?;
                Ok(Some((entry.band, Some(&entry.mask))))
            }
            (Some(clip), Some(mask)) => {
                self.combine(list, clip, mask)?;
                let entry = self
                    .built
                    .get(&Key::Both(clip, mask))
                    .ok_or(CpuRasterError::UnknownClip(clip))?
                    .as_ref();
                Ok(entry.map(|built| (built.band, Some(&built.mask))))
            }
        }
    }

    /// Builds the product of a clip and a soft mask, if it is not cached already.
    fn combine(
        &mut self,
        list: &DisplayList,
        clip: ClipId,
        mask: SoftMaskId,
    ) -> Result<(), CpuRasterError> {
        if self.built.contains_key(&Key::Both(clip, mask)) {
            return Ok(());
        }
        // The clip is built first and the soft mask read after it, which is safe in that
        // order only because building a clip cannot evict a soft mask — see
        // `admit_soft_mask`.
        self.get(list, clip)?;

        let Some(Some(clipped)) = self.built.get(&Key::Clip(clip)) else {
            // The clip admits no row, so their product admits none either.
            self.built.insert(Key::Both(clip, mask), None);
            return Ok(());
        };
        let Some(Some(soft)) = self.built.get(&Key::Soft(mask)) else {
            return Err(CpuRasterError::UnknownSoftMask(mask));
        };

        let band = clipped.band;
        let mut product = clipped.mask.clone();
        let width = self.target.width as usize;
        let start = (band.top as usize).saturating_mul(width);
        let soft_rows = soft
            .mask
            .data()
            .get(start..start.saturating_add(product.data().len()))
            .ok_or(CpuRasterError::UnknownSoftMask(mask))?
            .to_vec();
        for (value, &soft) in product.data_mut().iter_mut().zip(soft_rows.iter()) {
            // Two coverages multiply: 255 x 255 = 65 025 fits a `u16`, and the rounded
            // quotient keeps a fully open pair fully open.
            let scaled = u16::from(*value)
                .saturating_mul(u16::from(soft))
                .saturating_add(127);
            *value = u8::try_from(scaled / 255).unwrap_or(u8::MAX);
        }

        self.admit(
            Key::Both(clip, mask),
            Some(Built {
                mask: product,
                band,
            }),
        );
        Ok(())
    }

    /// Returns an evaluated soft mask's entry.
    fn soft_mask(&self, id: SoftMaskId) -> Result<&Built, CpuRasterError> {
        self.built
            .get(&Key::Soft(id))
            .and_then(Option::as_ref)
            .ok_or(CpuRasterError::UnknownSoftMask(id))
    }

    /// Whether a soft mask's values are already cached.
    fn holds_soft_mask(&self, id: SoftMaskId) -> bool {
        self.built.contains_key(&Key::Soft(id))
    }

    /// Stores an evaluated soft mask, covering the whole target.
    ///
    /// Whole rather than banded because a mask has a value everywhere — §11.6.5.1 gives one
    /// for the area outside its group's bounding box — so there is no row of the target it
    /// says nothing about.
    ///
    /// # Why a soft mask evicts only another soft mask
    ///
    /// Rebuilding an evicted *clip* costs a fill; rebuilding an evicted soft mask means
    /// rendering a whole command list, which only [`CpuRasterizer`] can do — so a cache that
    /// dropped one while a caller was in the middle of combining it with a clip would have
    /// nothing to hand back. Soft masks therefore sit outside the clip eviction order
    /// entirely and are dropped only here, when a *new* one arrives, which is a moment when
    /// no combination is in flight: [`CpuRasterizer::encode`] evaluates the mask a command
    /// needs before it asks for anything else. Their budget is the same one, counted
    /// separately, and the newest is never dropped for the same reason a clip's is not.
    fn admit_soft_mask(&mut self, id: SoftMaskId, mask: tiny_skia::Mask, target: TargetSpec) {
        let band = Band::whole(target);
        self.soft_bytes = self
            .soft_bytes
            .saturating_add(band.mask_bytes(self.target.width));
        self.soft_order.push_back(id);
        self.built.insert(Key::Soft(id), Some(Built { mask, band }));

        while self.soft_bytes > self.budget && self.soft_order.len() > 1 {
            let Some(oldest) = self.soft_order.pop_front() else {
                break;
            };
            if let Some(Some(entry)) = self.built.remove(&Key::Soft(oldest)) {
                self.soft_bytes = self
                    .soft_bytes
                    .saturating_sub(entry.band.mask_bytes(self.target.width));
            }
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
        if !self.built.contains_key(&Key::Clip(id)) {
            let chain = Self::resolve_chain(list, id)?;
            let built = self.build(list, id, &chain)?;
            self.admit(Key::Clip(id), built);
        }

        // Absence here would mean `admit` dropped the entry it had just stored, which
        // it does not do; reporting it beats drawing an unclipped page if it ever did.
        let entry = self
            .built
            .get(&Key::Clip(id))
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
    /// intersected with its parent's. Building the chain costs its depth in band-sized fills,
    /// which is less than one page-sized copy.
    ///
    /// **The reason this comment used to give for that is wrong, and the correction is worth
    /// keeping because it names a real optimisation nobody has taken.** It said "a parent
    /// covers a different band from its child, so a parent's mask cannot be reused as a
    /// starting point" — but the band comes from the running *intersection* of the chain's
    /// bounds, so a child's band is always contained in its parent's, and a mask value at a
    /// given device row does not depend on which band holds that row (`band.offset()` is a
    /// translation). A parent's rows for the child's band are therefore exactly the prefix's
    /// contribution, and a chain could be one crop plus one `intersect_path` instead of a
    /// fill plus depth-minus-one intersects.
    ///
    /// What stops it is memory rather than correctness, measured in the hundred-and-thirteenth
    /// session: on `bug1721218_reduced.pdf` the chains average four deep, so it would be worth
    /// most of `MaskCache::get`'s **24.3%** of that page — but it requires *every intermediate*
    /// clip to be cached, and intermediates have larger bands than the leaves now cached. The
    /// page already peaks at 27.9 MB against [`MASK_BUDGET`]'s 32 MB, so the change as stated
    /// trades a rebuild-free cache for an evicting one. Taking it means sizing the budget from
    /// a measurement of the intermediates first.
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
            // §8.5.4 with §8.5.3.3.1: an empty clipping path encloses nothing, so the
            // chain admits no row. `tiny-skia` would refuse the path instead, which is why
            // `Clip::admits_nothing` states the rule for both backends rather than either
            // rasteriser's treatment of an empty path standing in for it.
            if clip.admits_nothing() {
                return Ok(None);
            }
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
    fn admit(&mut self, id: Key, built: Option<Built>) {
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
        Clip, ClipId, DisplayList, FillRule, Path, PathCommand, Point, Size, SoftMaskId,
        TargetSpec, Transform,
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
        let mut cache = MaskCache::new(target, true, MASK_BUDGET);
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

        let mut generous = MaskCache::new(target, true, MASK_BUDGET);
        let before = generous
            .get(&list, first)
            .expect("builds")
            .map(|(band, mask)| (band, mask.data().to_vec()));

        let mut tight = MaskCache::new(target, true, MASK_BUDGET);
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

        let mut cache = MaskCache::new(target, true, MASK_BUDGET);
        assert!(
            cache.get(&list, id).expect("resolves").is_none(),
            "a clip entirely above the page admits no row"
        );
        assert_eq!(cache.bytes, 0, "an empty clip holds no pixels");
        assert!(
            cache.built.contains_key(&super::Key::Clip(id)),
            "the emptiness is remembered rather than rediscovered per command"
        );
    }

    /// A clip must never evict a soft mask, because only the rasteriser can rebuild one.
    ///
    /// `MaskCache::combine` reads the soft mask *after* building the clip it multiplies into,
    /// and that order is only safe under this rule. A cache that dropped the mask there would
    /// error out on a page that has many clips and one mask — which is `bug1721218_reduced.pdf`
    /// with a `gs` added, not a hypothetical.
    #[test]
    fn a_clip_never_evicts_a_soft_mask() {
        let (list, ids, target) = stacked_clips(40);
        let mut cache = MaskCache::new(target, true, MASK_BUDGET);
        cache.budget = Band { top: 0, height: 4 }.mask_bytes(target.width) * 2;

        let mask = SoftMaskId::new(0);
        cache.admit_soft_mask(
            mask,
            tiny_skia::Mask::new(target.width, target.height).expect("a target-sized mask"),
            target,
        );
        for &id in &ids {
            cache.get(&list, id).expect("a rectangular clip builds");
        }

        assert!(
            cache.built.len() < ids.len(),
            "nothing was evicted, so the rule was never exercised"
        );
        assert!(
            cache.soft_mask(mask).is_ok(),
            "the soft mask was dropped by clips it has nothing to do with"
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
    /// A command referenced a soft mask that is not present, or was never evaluated.
    #[error("soft mask {0:?} is not present in this display list")]
    UnknownSoftMask(SoftMaskId),
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
