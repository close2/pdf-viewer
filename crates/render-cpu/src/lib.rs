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
mod scan;
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
    /// **This changes how long a page takes and almost nothing about what it looks like**, which
    /// is [`Surface`]'s claim and is the reason this method exists: the property is only
    /// checkable by rendering one page several ways and comparing the bytes, which is what
    /// `strip_parallelism.rs` does — over six scenes here, and over three real pages in
    /// `pdf-model`, which is where the exception the six scenes could not hold was found. What
    /// "almost" covers is `tiny-skia`'s own arithmetic at a shifted origin, worth one supersample
    /// on an edge that lands on one: ADR 0219 measures it and says why it cannot be removed.
    ///
    /// **One** is the value a caller with no filesystem must pass, because asking the machine
    /// reads `/proc` (ADR 0218). A page still gets fewer strips than asked for where its curves
    /// forbid the cuts.
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

        // §11.4.7 puts a colour space under the whole page — "[a]ll page-level compositing
        // shall be done in the default blending colour space of the page, and the entire
        // result shall then ... be converted to the native colour space of the output device
        // before being composited with the context-dependent backdrop". Where that space has
        // four components the interpreter hands over a second page whose colours carry the
        // fourth, drawn on the same geometry under the same shapes and opacities, and the two
        // rasters are put back together here — *before* `impose_on_medium`, which is where
        // the clause puts the conversion. See `pdf_render::blending`.
        if let (Some(space), Some(black)) = (list.blending(), list.black()) {
            let mut ink = tiny_skia::Pixmap::new(target.width, target.height).ok_or(
                CpuRasterError::Allocation {
                    width: target.width,
                    height: target.height,
                },
            )?;
            self.encode_in_strips(&mut ink, black, target)?;
            pdf_render::resolve_blending(pixmap.data_mut(), ink.data(), space);
        }

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
    /// That rule is about *geometry*, and it was not the whole of it: the *arithmetic* had to
    /// be the page's too, which is [`Surface`] and ADR 0219.
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
            let surface = Surface::whole(target);
            let mut masks = MaskCache::new(surface, self.anti_alias, MASK_BUDGET);
            return self.encode(
                &mut pixmap.as_mut(),
                list,
                list.commands(),
                surface,
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
                // The page's own target travels into the strip, not a shifted copy of it:
                // where the strip starts is [`Surface`]'s other field, and it reaches the
                // geometry once, last. ADR 0219 is what a shifted copy cost.
                let surface = Surface {
                    page: target,
                    rows: Band { top, height: rows },
                };
                let mut piece = tiny_skia::PixmapMut::from_bytes(piece, width, rows).ok_or(
                    CpuRasterError::Allocation {
                        width,
                        height: rows,
                    },
                )?;
                let mut masks = MaskCache::new(surface, self.anti_alias, budget);
                self.encode(
                    &mut piece,
                    list,
                    list.commands(),
                    surface,
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
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        path: &Path,
        transform: Transform,
        stroke: &Stroke,
        paint: &Paint,
        blend: tiny_skia::BlendMode,
        to_device: ToDevice,
        clip: Option<&tiny_skia::Mask>,
    ) -> Result<(), CpuRasterError> {
        let at = to_device.of(transform);
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
            let brush = self.paint(paint, blend, page_to_path(transform)?, &mut scratch)?;
            if !draw_sub_pixel_rule(pixmap, (&geometry, &converted), &style, at, &brush, clip) {
                scan::stroke(
                    pixmap,
                    &converted,
                    &brush,
                    &style,
                    convert::transform(at),
                    clip,
                );
            }
        }
        // The marks are *filled*, not stroked: §8.5.3.2 asks for "a filled circle
        // centred at the single point", and the stroking paint is what fills it.
        if !dots.is_empty()
            && let Some(converted) = convert::path(&dots)
        {
            let mut scratch = None;
            scan::fill(
                pixmap,
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
        surface: Surface,
        masks: &mut MaskCache,
        depth: usize,
        compose: Compose,
    ) -> Result<(), CpuRasterError> {
        for command in commands {
            // A command whose extent misses this surface marks nothing, and saying so here is
            // what makes a strip cost what its own rows cost: without it every strip would
            // build every command's path and compile every command's pipeline, which session
            // 154 measured at 19% of a dense page's rasterisation. A row of margin, for the
            // same reason `Band::covering` takes one — the extent comes from control points
            // and the mask from the path.
            if misses_surface(command, surface) {
                continue;
            }

            // A soft mask is evaluated before anything borrows the cache, because building
            // it renders a whole command list of its own and so needs the cache mutably.
            // Idempotent: the second command under the same mask finds it already there.
            if let Some(id) = command.mask() {
                self.build_soft_mask(list, id, surface, masks, depth)?;
            }

            if let Command::Shaped { object, shape } = command {
                self.encode_shaped(
                    pixmap,
                    list,
                    (object, shape),
                    surface,
                    masks,
                    depth,
                    compose,
                )?;
                continue;
            }

            // A group is the one command that needs the mask cache mutably *while* it
            // draws, so it cannot hold a clip mask borrowed from it across the recursion.
            // It therefore resolves its own clip, twice, either side of its elements.
            if let Command::Group {
                commands,
                alpha,
                blend,
                isolated,
                knockout,
                ..
            } = command
            {
                // A group whose shape is needed arrives as one half of a `Command::Shaped`,
                // where the enclosing compose is `Erase` or `Add`. A *bare* group under
                // knockout would have to be composited by its shape and has none stated, so
                // it is refused rather than approximated — `pdf-model` does not build that
                // display list, and erroring is what keeps the assumption from becoming a
                // silence if it ever does.
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
                        isolated: *isolated,
                        compose: if *knockout {
                            Compose::Knockout
                        } else {
                            Compose::Over
                        },
                        into: compose,
                    },
                    surface,
                    masks,
                    depth,
                )?;
                continue;
            }

            // Resolved before the match so that every arm shares one code path for
            // clip handling; a per-arm lookup would be a place for them to diverge.
            // The clip admits no row of the target, so nothing this command draws can
            // survive it.
            let Some(Admitted {
                band,
                mask: clip,
                admits,
            }) = masks.effective(list, command.clip(), command.mask())?
            else {
                continue;
            };

            // Everything below draws into the band rather than the page, which is what
            // keeps a command's cost proportional to the pixels its clip can admit.
            // The device map carries the band's first row so that geometry, paints
            // and images all move together; missing one would tear the page apart in a
            // way no metric would notice, so there is exactly one of these.
            let to_device = surface.to_device(band);

            // ISO 32000-2 §11.3.5.3's four modes are computed by this backend rather than
            // by `tiny-skia`, whose three of them are wrong (ADR 0047), so such a command
            // is drawn onto transparency first and composited by `blend::composite`.
            // Drawing onto transparency loses nothing: with αb = 0 the compositing formula
            // collapses to the source, whatever the blend mode.
            if let Some(mode) = compose.non_separable(command.blend()) {
                let mut layer = tiny_skia::Pixmap::new(surface.width(), band.height).ok_or(
                    CpuRasterError::Allocation {
                        width: surface.width(),
                        height: band.height,
                    },
                )?;
                self.draw(
                    &mut layer.as_mut(),
                    command,
                    to_device,
                    (clip, admits),
                    compose,
                )?;
                let mut rows = band
                    .rows(pixmap, surface)
                    .ok_or(CpuRasterError::Allocation {
                        width: surface.width(),
                        height: band.height,
                    })?;
                blend::composite(&mut rows, &layer, mode);
                continue;
            }

            let mut rows = band
                .rows(pixmap, surface)
                .ok_or(CpuRasterError::Allocation {
                    width: surface.width(),
                    height: band.height,
                })?;

            self.draw(&mut rows, command, to_device, (clip, admits), compose)?;
        }
        Ok(())
    }

    /// Draws §11.4.6's two stages for an element that states its shape (`Command::Shaped`).
    ///
    /// Empty `1 − shape` of what is under it, then add the object: `P' = (1 − f) × P + S`,
    /// which is the clause's weighted average on the transparent initial backdrop a group is
    /// built on. Outside a knockout group the shape is unused — §11.4.4 reaches it only
    /// through shape × opacity — so the object is drawn alone, which is what
    /// `Command::Shaped` guarantees a backend may do.
    ///
    /// # Errors
    ///
    /// As [`CpuRasterizer::encode`].
    #[expect(
        clippy::too_many_arguments,
        reason = "the two halves of one element, and everything `encode` carries; this is \
                  that recursion split at one arm rather than a call with its own state"
    )]
    fn encode_shaped(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        list: &DisplayList,
        (object, shape): (&Command, &Command),
        surface: Surface,
        masks: &mut MaskCache,
        depth: usize,
        compose: Compose,
    ) -> Result<(), CpuRasterError> {
        if compose == Compose::Knockout {
            let shape = std::slice::from_ref(shape);
            self.encode(pixmap, list, shape, surface, masks, depth, Compose::Erase)?;
        }
        let object = std::slice::from_ref(object);
        let after = if compose == Compose::Knockout {
            Compose::Add
        } else {
            compose
        };
        self.encode(pixmap, list, object, surface, masks, depth, after)
    }

    /// Composites one transparency group (ISO 32000-2 §11.4.1).
    ///
    /// The elements are drawn onto a buffer the size of this surface and the result is then
    /// painted onto the page once, under the group's own constant alpha and blend mode.
    /// Compositing the elements one at a time onto the page instead is what §11.6.6's
    /// initialisation of the alpha constants exists to prevent, and is visibly different
    /// wherever two elements overlap.
    ///
    /// The buffer covers the whole surface rather than the group's band because the elements'
    /// clips are resolved against the surface, so their bands are its rows; a band-sized
    /// buffer would need every one of them shifted, and one coordinate system that is right
    /// beats two that have to agree.
    ///
    /// # The two backdrops, and the two ways the buffer comes back
    ///
    /// An **isolated** group starts on transparency — §11.4.5's initial backdrop — and comes
    /// back with source-over. A **non-isolated** one starts as a *copy of the page*, which is
    /// §11.4.4's own model, and comes back as the interpolation `Command::Group`'s `isolated`
    /// derives: `(1 − w) × page + w × buffer`, with `w` the group's constant alpha times its
    /// soft mask at the pixel. That is two draws — Destination-Out by `w` over the band, then
    /// Plus of the buffer at `w` — and the pair is bounded by 1 everywhere, because the two
    /// weights sum to 1 and each operand is a valid premultiplied sample, so `Plus` never
    /// saturates.
    ///
    /// Outside the group's marks the buffer still *is* the page, and the interpolation of a
    /// value with itself is that value, so the copy costs the region nothing.
    ///
    /// # Errors
    ///
    /// Returns [`CpuRasterError::UnsupportedCommand`] for a non-isolated group carrying
    /// anything the collapse above does not hold for; `pdf-model` guarantees it does not
    /// build one, and refusing keeps that from becoming a silence.
    fn draw_group(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        list: &DisplayList,
        group: Group<'_>,
        surface: Surface,
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
        let Some(Admitted { band, .. }) = masks.effective(list, group.clip, group.mask)? else {
            return Ok(());
        };

        let mut buffer = initial_backdrop(pixmap, surface, &group)?;
        self.encode(
            &mut buffer.as_mut(),
            list,
            group.commands,
            surface,
            masks,
            depth,
            group.compose,
        )?;

        // The elements may have evicted the soft mask this group is painted through, since
        // they share the cache; rebuilding it is what makes eviction safe here as it is for
        // a clip.
        if let Some(id) = group.mask {
            self.build_soft_mask(list, id, surface, masks, depth)?;
        }

        // Resolved again rather than held across the recursion: the elements' own clips
        // share this cache and may have evicted the entry, and a rebuilt mask is the mask
        // that was dropped — see `a_rebuilt_mask_is_the_mask_that_was_evicted`.
        let Some(Admitted { mask: clip, .. }) = masks.effective(list, group.clip, group.mask)?
        else {
            return Ok(());
        };

        let paint = tiny_skia::PixmapPaint {
            opacity: group.alpha.clamp(0.0, 1.0),
            // `Compose::mode` reads the group's own blend where the group composites
            // ordinarily, and §11.4.6's operator where it is half of a shaped element —
            // whose blend mode the clause leaves nothing to blend against.
            blend_mode: group.into.mode(group.blend),
            // The buffer is drawn at 1:1 with no transform, so no sample is ever
            // interpolated and the quality setting cannot change a pixel.
            quality: tiny_skia::FilterQuality::Nearest,
        };
        // Negative, because the buffer covers the whole surface and the rows drawn into
        // start at the band's first row. Both are page rows, so the difference is what the
        // buffer has to be shifted by.
        let top = i32::try_from(band.top.saturating_sub(surface.rows.top)).map_err(|_| {
            CpuRasterError::Allocation {
                width: surface.width(),
                height: band.height,
            }
        })?;

        // §11.4.4's own model, for a group whose elements were drawn onto a copy of the
        // page: `(1 − w) × page + w × buffer`, with `w` the group's constant alpha times its
        // soft mask at the pixel. One pass over the band rather than two Porter-Duff draws,
        // for the rounding reason `blend::interpolate` records. The whole band is walked
        // because outside the group's marks the buffer *is* the page and the interpolation
        // has to put it back unchanged — which it does, exactly, and cheaply.
        if !group.isolated {
            let from_row = band.top.saturating_sub(surface.rows.top);
            let mut rows = band
                .rows(pixmap, surface)
                .ok_or(CpuRasterError::Allocation {
                    width: surface.width(),
                    height: band.height,
                })?;
            blend::interpolate(&mut rows, &buffer, from_row, paint.opacity, clip);
            return Ok(());
        }

        // §11.3.5.3's four modes are this backend's own (ADR 0047), and a group reaches
        // them by the same two steps a command does: the group's clip, alpha and offset
        // are applied onto transparency, and the result is composited by `blend`. The
        // extra buffer is the price of the mode, not of every group.
        if let Some(mode) = group.into.non_separable(group.blend) {
            let mut layer = tiny_skia::Pixmap::new(surface.width(), band.height).ok_or(
                CpuRasterError::Allocation {
                    width: surface.width(),
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
            let mut rows = band
                .rows(pixmap, surface)
                .ok_or(CpuRasterError::Allocation {
                    width: surface.width(),
                    height: band.height,
                })?;
            blend::composite(&mut rows, &layer, mode);
            return Ok(());
        }

        let mut rows = band
            .rows(pixmap, surface)
            .ok_or(CpuRasterError::Allocation {
                width: surface.width(),
                height: band.height,
            })?;
        rows.draw_pixmap(
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
    /// The mask's group is drawn onto a fully transparent buffer covering this surface — the same
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
        surface: Surface,
        masks: &mut MaskCache,
        depth: usize,
    ) -> Result<(), CpuRasterError> {
        if masks.holds_soft_mask(id) {
            return Ok(());
        }
        let mask = list
            .soft_mask(id)
            .ok_or(CpuRasterError::UnknownSoftMask(id))?;

        let mut buffer = tiny_skia::Pixmap::new(surface.width(), surface.rows.height).ok_or(
            CpuRasterError::Allocation {
                width: surface.width(),
                height: surface.rows.height,
            },
        )?;
        // A soft mask's group is evaluated as §11.4.5's ordinary group: `SoftMask` carries
        // no knockout flag, and `pdf-model` reports a mask group that asks for one.
        self.encode(
            &mut buffer.as_mut(),
            list,
            &mask.commands,
            surface,
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
            tiny_skia::IntSize::from_wh(surface.width(), surface.rows.height).ok_or(
                CpuRasterError::Allocation {
                    width: surface.width(),
                    height: surface.rows.height,
                },
            )?,
        )
        .ok_or(CpuRasterError::Allocation {
            width: surface.width(),
            height: surface.rows.height,
        })?;
        masks.admit_soft_mask(id, built, surface.rows);
        Ok(())
    }

    /// Draws `path` with a paint no shader can express, and says whether it did.
    ///
    /// Two of them, and each is a *paint* that becomes pixels rather than a gradient:
    ///
    /// - **A mesh** carries a colour per triangle corner, so it is drawn triangle by triangle
    ///   inside the shape rather than as a paint over it.
    /// - **A radial whose two circles do not contain one another** is ISO 32000-2 §8.7.4.5.4's
    ///   cone, where a point can lie on two blend circles and the clause's "greatest value of
    ///   s" decides between them — including the case where the greater root is one `/Extend`
    ///   refuses. No two-point conical gradient expresses that, so the cone is evaluated
    ///   exactly and drawn as a raster; every other radial keeps the native gradient, which
    ///   [`shading::is_a_cone`] proves is enough there.
    ///
    /// Split out of [`CpuRasterizer::draw_fill`] because it is one question — is this paint a
    /// raster? — asked twice, and neither answer shares a line with what follows it there.
    fn filled_as_a_raster(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        (path, fill_rule): (&tiny_skia::Path, pdf_render::FillRule),
        (paint, blend): (&Paint, tiny_skia::BlendMode),
        to_device: ToDevice,
        (transform, clip): (Transform, Option<&tiny_skia::Mask>),
    ) -> bool {
        let Paint::Shading(shading) = paint else {
            return false;
        };
        let at = convert::transform(to_device.of(transform));
        match shading.kind.as_ref() {
            pdf_render::ShadingKind::Mesh { triangles } => {
                shading::fill_mesh(
                    pixmap,
                    path,
                    triangles,
                    to_device.of(shading.transform),
                    convert::fill_rule(fill_rule),
                    at,
                    clip,
                    blend,
                    self.anti_alias,
                );
                true
            }
            pdf_render::ShadingKind::Radial {
                start,
                start_radius,
                end,
                end_radius,
                ramp,
                extend,
            } => {
                shading::is_a_cone(*start, *start_radius, *end, *end_radius)
                    && shading::fill_radial(
                        pixmap,
                        path,
                        pdf_render::Radial {
                            start: *start,
                            start_radius: *start_radius,
                            end: *end,
                            end_radius: *end_radius,
                            ramp,
                            extend: *extend,
                        },
                        to_device.of(shading.transform),
                        convert::fill_rule(fill_rule),
                        at,
                        clip,
                        blend,
                        self.anti_alias,
                    )
            }
            _ => false,
        }
    }

    /// Draws a filled path, including the marks a shape with no area cannot make itself.
    ///
    /// Split out of [`CpuRasterizer::draw`] because ISO 32000-2 §10.7.4 turns one command into
    /// two draws: a subpath with no extent along one axis has zero area, so this rasteriser
    /// computes zero coverage for it at every placement and every scale, and the clause says
    /// no shape may disappear. What it marks instead is `pdf-render`'s geometry rather than
    /// this backend's hairline, so that the two backends cannot answer it differently — and
    /// the marks are filled under the non-zero rule whatever the command's own rule is,
    /// because a mark is a shape in its own right rather than part of the path's winding.
    ///
    /// # Errors
    ///
    /// Returns [`CpuRasterError`] for a path the rasteriser rejects or a paint this backend
    /// does not implement.
    #[expect(
        clippy::too_many_arguments,
        reason = "the fields of one display list command, passed through rather than \
                  regrouped: a struct here would be a second spelling of `Command::Fill`"
    )]
    fn draw_fill(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        (source, fill_rule): (&Path, pdf_render::FillRule),
        transform: Transform,
        paint: &Paint,
        blend: tiny_skia::BlendMode,
        to_device: ToDevice,
        (clip, admits): (Option<&tiny_skia::Mask>, Option<tiny_skia::Rect>),
    ) -> Result<(), CpuRasterError> {
        let at = to_device.of(transform);
        let cropped = crop_to_mask(source, transform, to_device, admits);
        let source = cropped.as_ref().unwrap_or(source);
        let path = convert::path(source).ok_or(CpuRasterError::InvalidPath)?;

        if self.filled_as_a_raster(
            pixmap,
            (&path, fill_rule),
            (paint, blend),
            to_device,
            (transform, clip),
        ) {
            return Ok(());
        }

        // A sampled shading's pixels are borrowed by its shader, so they need somewhere to
        // live for exactly as long as this call.
        let mut scratch = None;
        let brush = self.paint(paint, blend, page_to_path(transform)?, &mut scratch)?;
        let split = pdf_render::split_collapsed_fill(source, at);
        if let Some(split) = &split
            && !split.marks.is_empty()
            && let Some(marks) = convert::path(&split.marks)
        {
            scan::fill(
                pixmap,
                &marks,
                &brush,
                tiny_skia::FillRule::Winding,
                convert::transform(at),
                clip,
            );
        }
        let remaining = match &split {
            // Every subpath collapsed, so the marks are the whole of the drawing.
            Some(split) if split.filled.is_empty() => return Ok(()),
            Some(split) => &split.filled,
            None => source,
        };
        // §10.7.4 again, for a shape this rasteriser's coverage quantum would round to nothing.
        if carries_coverage_as_alpha(&brush)
            && let Some(bands) = pdf_render::sub_pixel_bands(remaining, at, fill_rule)
        {
            for band in &bands {
                let shape = convert::path(&band.shape).ok_or(CpuRasterError::InvalidPath)?;
                let mut faint = brush.clone();
                faint.shader.apply_opacity(band.coverage);
                scan::fill(
                    pixmap,
                    &shape,
                    &faint,
                    tiny_skia::FillRule::Winding,
                    convert::transform(at),
                    clip,
                );
            }
            return Ok(());
        }
        let path = match &split {
            Some(split) => convert::path(&split.filled).ok_or(CpuRasterError::InvalidPath)?,
            None => path,
        };
        scan::fill(
            pixmap,
            &path,
            &brush,
            convert::fill_rule(fill_rule),
            convert::transform(at),
            clip,
        );
        Ok(())
    }

    /// Draws one command onto `pixmap`, which is the band its clip admits.
    ///
    /// `to_device` maps page space onto that band, carrying its first row as a whole number
    /// subtracted at the end of every composition; every transform below goes through it and
    /// none reaches the device directly. See [`ToDevice`] for why that order is the property
    /// rather than a detail.
    ///
    /// # Errors
    ///
    /// Returns [`CpuRasterError`] for a path the rasteriser rejects, for a paint or a
    /// command variant this backend does not implement, or for an inconsistent image.
    fn draw(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        command: &Command,
        to_device: ToDevice,
        (clip, admits): (Option<&tiny_skia::Mask>, Option<tiny_skia::Rect>),
        compose: Compose,
    ) -> Result<(), CpuRasterError> {
        match command {
            Command::Fill {
                path: source,
                transform,
                fill_rule,
                paint,
                blend,
                ..
            } => {
                self.draw_fill(
                    pixmap,
                    (source, *fill_rule),
                    *transform,
                    paint,
                    compose.mode(*blend),
                    to_device,
                    (clip, admits),
                )?;
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
                    pixmap,
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
                self.draw_image(pixmap, image, placement, clip)?;
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
///
/// # The two halves of a [`Command::Shaped`] element
///
/// Where the shape is *not* the coverage the display list states it separately, and the
/// clause's two stages become two draws: [`Self::Erase`] scales the accumulated result by
/// `1 − shape`, then [`Self::Add`] adds the element. In premultiplied form that is
/// `P' = (1 − f) × P + S` exactly — see [`Command::Shaped`] — where the one-step
/// [`Self::Knockout`] form is the same line with `f` read off the coverage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Compose {
    /// §11.4.5: an element paints over the group's accumulated result.
    Over,
    /// §11.4.6: an element replaces it within its own coverage.
    Knockout,
    /// §11.4.6's second stage, weighting the immediate backdrop by `1 − shape`.
    Erase,
    /// §11.4.6's second stage, adding the object the first stage composited.
    ///
    /// Addition rather than source-over, and the difference is the whole point of the pair:
    /// source-over would weight the backdrop by `1 − shape × opacity` a second time, which
    /// is right only where the object is opaque or its shape is 0 or 1. What the two draws
    /// leave behind is disjoint — `1 − f` of the backdrop and `f × opacity` of the object —
    /// so the sum cannot exceed 1 and `Plus`'s saturation never engages.
    Add,
}

impl Compose {
    /// The `tiny-skia` mode an element painted under `blend` is drawn with.
    fn mode(self, blend: pdf_render::BlendMode) -> tiny_skia::BlendMode {
        match self {
            Self::Over => convert::blend_mode(blend),
            Self::Knockout => tiny_skia::BlendMode::Source,
            Self::Erase => tiny_skia::BlendMode::DestinationOut,
            Self::Add => tiny_skia::BlendMode::Plus,
        }
    }

    /// Whether §11.3.5.3's four modes still need this backend's own compositing.
    ///
    /// Never under knockout: the element has no backdrop to blend with.
    fn non_separable(self, blend: pdf_render::BlendMode) -> Option<blend::NonSeparable> {
        match self {
            Self::Over => blend::NonSeparable::of(blend),
            Self::Knockout | Self::Erase | Self::Add => None,
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
    /// What the elements are composited onto — see `Command::Group`'s `isolated`.
    isolated: bool,
    /// How this group's own elements combine with each other (§11.4.6).
    compose: Compose,
    /// How the finished group combines with what it is drawn onto.
    ///
    /// [`Compose::Over`] everywhere except the two halves of a [`Command::Shaped`], where a
    /// group is one element of a knockout group and §11.4.6's two stages are two draws.
    into: Compose,
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
    to_device: ToDevice,
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
        source: &pdf_render::ImageSource,
        placement: ImagePlacement,
        clip: Option<&tiny_skia::Mask>,
    ) -> Result<(), CpuRasterError> {
        let ImagePlacement {
            transform,
            alpha,
            blend,
            to_device,
        } = placement;
        let placement = to_device.of(transform);
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

        scan::fill(
            pixmap,
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

/// Draws a stroke thinner than a device pixel as a shape this rasteriser can measure, ISO 32000-2
/// §10.7.4, and reports whether it did.
///
/// The clause's "no shape ever disappears" applies to strokes as well as fills — "[t]his rule
/// applies both to fill operations and to strokes with non-zero width" — and `tiny-skia` loses a
/// thin one in two ways of its own. Its painter draws a stroke under a pixel wide as a **hairline**
/// with the paint's opacity scaled by the width, which
///
/// - smears the mark symmetrically about the path whatever fraction of a pixel the path lies at,
///   so a rule within half a pixel of the raster's edge loses the half that falls outside — 0.0549
///   of its own 0.1 at the top edge of a 320-unit page, where the graphics device carried 0.0980;
/// - and lays that mark down **one pixel per step along the line's longer device axis**, so a rule
///   at `θ` from the nearer axis carries `cos θ` of its area. Measured by
///   `render-quorra/examples/sub_pixel_marks`: 3.4% short at 15°, 13.4% at 30° and **29.3% at
///   45°**, at every thickness rather than only near the coverage quantum.
///
/// The second reads directly against the sentence three along from the one above — "[t]he area
/// covered by painted pixels shall always be at least as large as the area of the original shape"
/// — so both are answered here rather than left to the library.
///
/// # Two constructions, exact first
///
/// [`draw_rule_as_bands`] is ADR 0226's and is exact: the outline of a straight axis-aligned rule
/// is the rectangle its width and length state, and [`pdf_render::sub_pixel_bands`] draws that at
/// the coverage its own area implies, including the part that is off the raster, which is then
/// clipped away rather than folded back in.
///
/// [`draw_rule_at_one_pixel`] is the general answer and is ADR 0268's: **the same path stroked one
/// device pixel wide, with the width it gave up carried in the paint's alpha.** That conserves the
/// ink exactly at every angle, because widening by a factor and dividing the alpha by it cancel,
/// and it needs no scan converter of our own — which is what the residual `doc/todo/11` carried
/// since ADR 0226 was priced at. It is tried second because it is the blunter of the two: a band
/// one device pixel wide spreads its ink over a pixel where a band of the true width spreads it
/// over `w`, and at the raster's edge it still loses what falls outside.
///
/// Nothing is snapped by either. A 0.1-unit rule draws 0.1 of a row at the fractional position the
/// document put it, which is the sentence `doc/todo/_scan-conversion.md` demands of anything that
/// touches the pixel grid; what §10.7.5 would do under `/SA` is multiply the *ink* by
/// `one_pixel / w` rather than divide the alpha by it.
///
/// # What it still declines
///
/// The whole rule is conditioned on [`carries_coverage_as_alpha`], which is where its warrant
/// lives. Beyond that, only a stroke the stroker itself refuses — and the exact construction
/// declines a great deal more, each case argued in `pdf_render::sub_pixel`'s module comment.
/// A dashed rule is now taken: the dashes are dispensed by `tiny_skia::Path::dash`, which is the
/// same function `stroke_path` would have called, rather than by a second implementation of
/// §8.4.3.6 in this file.
fn draw_sub_pixel_rule(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    geometry: (&Path, &tiny_skia::Path),
    style: &tiny_skia::Stroke,
    at: Transform,
    brush: &tiny_skia::Paint<'_>,
    clip: Option<&tiny_skia::Mask>,
) -> bool {
    let Some(one_pixel) = pdf_render::thinnest_line(at) else {
        return false;
    };
    if style.width >= one_pixel || !carries_coverage_as_alpha(brush) {
        return false;
    }
    // The resolution scale bounds how finely a curve is measured while it is offset, and is what
    // `stroke_path` would have used had the hairline test not fired.
    let scale = tiny_skia::PathStroker::compute_resolution_scale(&convert::transform(at));
    // The cheap questions come first: a path of straight axis-aligned rules is one walk over the
    // commands, and only such a path can reach the exact construction at all.
    if style.dash.is_none()
        && at.preserves_axes()
        && pdf_render::only_flat_subpaths(geometry.0)
        && draw_rule_as_bands(pixmap, geometry.1, style, at, scale, brush, clip)
    {
        return true;
    }
    draw_rule_at_one_pixel(pixmap, geometry.1, style, at, scale, brush, clip)
}

/// ADR 0226's exact construction: the rule's own outline, at the coverage its area implies.
///
/// See [`draw_sub_pixel_rule`], whose first choice this is. Returns `false` — leaving the caller
/// to the construction below — for an outline [`pdf_render::sub_pixel_bands`] cannot measure
/// exactly, which is every case its module comment argues: a round cap's arc, two rules meeting in
/// one pixel row, a subpath that is not a rectangle.
fn draw_rule_as_bands(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    path: &tiny_skia::Path,
    style: &tiny_skia::Stroke,
    at: Transform,
    scale: f32,
    brush: &tiny_skia::Paint<'_>,
    clip: Option<&tiny_skia::Mask>,
) -> bool {
    let Some(outline) = path.stroke(style, scale) else {
        return false;
    };
    let Some(bands) = pdf_render::sub_pixel_bands(
        &convert::from_skia_path(&outline),
        at,
        pdf_render::FillRule::NonZero,
    ) else {
        return false;
    };
    // Converted before anything is drawn, so a shape the rasteriser rejects leaves the whole rule
    // to the construction below rather than drawing half of it twice.
    let Some(shapes) = bands
        .iter()
        .map(|band| Some((convert::path(&band.shape)?, band.coverage)))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    for (shape, coverage) in &shapes {
        let mut faint = brush.clone();
        faint.shader.apply_opacity(*coverage);
        scan::fill(
            pixmap,
            shape,
            &faint,
            tiny_skia::FillRule::Winding,
            convert::transform(at),
            clip,
        );
    }
    true
}

/// ADR 0268's general construction: the rule stroked one device pixel wide, at the alpha its own
/// width implies.
///
/// See [`draw_sub_pixel_rule`] for why this is owed and [`pdf_render::substitute_width`] for why
/// the width is stated from the transform's *smaller* stretch. The two moves are one arithmetic
/// identity: the substitute's device area is `width / style.width` times the rule's, and the alpha
/// is `style.width / width`, so the ink is the rule's own area whatever the transform and whatever
/// the angle.
///
/// The dashes are dispensed first, by the same `tiny_skia::Path::dash` that `stroke_path` would
/// have called, because widening a dashed stroke must not widen its dashes: §8.4.3.6's pattern is
/// measured along the path and has nothing to do with the width.
fn draw_rule_at_one_pixel(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    path: &tiny_skia::Path,
    style: &tiny_skia::Stroke,
    at: Transform,
    scale: f32,
    brush: &tiny_skia::Paint<'_>,
    clip: Option<&tiny_skia::Mask>,
) -> bool {
    let Some(width) = pdf_render::substitute_width(at) else {
        return false;
    };
    // A width above the substitute's is not this rule's business: `draw_sub_pixel_rule` has
    // already established that the stroke is under one device pixel by §8.4.3.2's reading, and
    // under an anisotropic transform that leaves the two readings a factor apart.
    if style.width >= width {
        return false;
    }
    let dashed;
    let path = match style.dash.as_ref() {
        None => path,
        Some(dash) => {
            let Some(cut) = path.dash(dash, scale) else {
                return false;
            };
            dashed = cut;
            &dashed
        }
    };
    let mut widened = style.clone();
    widened.width = width;
    widened.dash = None;
    // The identity above is the *body's*: a cap's area goes as the square of the width, so
    // widening multiplies it by `(width / style.width)²` where the alpha divides it back only
    // once, and the end is then overstated by that factor. On a long rule nobody could see it;
    // on `issue12295.pdf`, whose 65 859 sub-pixel strokes are round-capped and 91.8% of them
    // shorter than one device pixel — median length 0.145 — two round caps at one pixel are
    // `π/4` of a pixel against a body of 0.145, and the page came out 31% heavier than its own
    // 8x limit. The substitute is therefore the swept body and nothing else. What that gives up
    // is the true cap, whose own area is `O(w²)` and which extends the mark by half of a width
    // this rule has already established is under one pixel.
    widened.line_cap = tiny_skia::LineCap::Butt;
    let Some(outline) = path.stroke(&widened, scale) else {
        return false;
    };
    let mut faint = brush.clone();
    // `width` is a reciprocal of a positive stretch, so it is finite and above zero.
    faint.shader.apply_opacity(style.width / width);
    scan::fill(
        pixmap,
        &outline,
        &faint,
        tiny_skia::FillRule::Winding,
        convert::transform(at),
        clip,
    );
    true
}

/// Whether painting a shape at alpha `c` says the same thing as painting it at coverage `c`.
///
/// **The first condition is that this rasteriser is anti-aliasing at all.** The substitution's
/// whole warrant is the departure §10.7.4's row records — the clause's "paint the pixel" replaced
/// by coverage proportional to area — so where [`CpuRasterizer::with_anti_alias`] has turned that
/// departure off, the clause's own answer for a sub-pixel shape is a *whole* covered pixel and a
/// fractional alpha is neither that nor what the rasteriser was asked for. The aliased mode is
/// therefore left exactly as it was. It is one test's knob today and this costs it nothing; what
/// it buys is that the rule below is true as stated rather than true of one configuration.
///
/// [`pdf_render::sub_pixel_bands`] hands back a coverage this rasteriser cannot express as a
/// shape, and the only place left to put it is the paint's alpha. The standard says when the two
/// are one quantity, ISO 32000-2 §11.3.7.1:
///
/// > As stated earlier, the alpha values that control the compositing process shall be defined as
/// > the product of shape and opacity
///
/// and §11.3.7.2's NOTE 1 says that anti-aliased coverage is the first of the two factors:
///
/// > Mathematically, elementary objects have "hard" edges, with a shape value of either 0.0 or
/// > 1.0 at every point. However, when such objects are rasterized to device pixels, the shape
/// > values along the boundaries can be anti-aliased, taking on fractional values representing
/// > fractional coverage of those pixels. When such anti-aliasing is performed, it is important
/// > to treat the fractional coverage as shape rather than opacity.
///
/// A raster that carries one alpha channel per pixel carries exactly that product, so folding a
/// band's coverage into it is the clause's own arithmetic rather than a trick — every blend
/// function reads `αs` and none of them can tell which factor it came from.
///
/// **The NOTE's warning is the exception, and it is why this returns `false` for one mode.**
/// Where shape has to stay distinguishable from opacity the substitution is wrong, and this
/// backend has one such place: §11.4.6's knockout group, where an element replaces its backdrop
/// *within its own shape* and `tiny-skia` states that as Porter-Duff Source. There a scaled alpha
/// leaves a partly transparent pixel where a partly covered one is meant. A sub-pixel shape inside
/// a knockout group therefore keeps whatever the rasteriser already gave it. ADR 0226.
fn carries_coverage_as_alpha(brush: &tiny_skia::Paint<'_>) -> bool {
    brush.anti_alias && brush.blend_mode != tiny_skia::BlendMode::Source
}

/// Multiplies a straight-alpha channel by its alpha.
fn premultiply(value: u8, alpha: u8) -> u8 {
    // Rounded rather than truncated, so a fully opaque pixel round-trips exactly.
    let scaled = u16::from(value)
        .saturating_mul(u16::from(alpha))
        .saturating_add(127);
    u8::try_from(scaled / 255).unwrap_or(u8::MAX)
}

/// A contiguous run of **page** rows: the only rows a command is allowed to mark.
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
///
/// **Page rows, and a [`Surface`]'s own rows are one of these too.** A strip of a page and
/// the band a clip admits are the same idea at two scales, and counting both from the page's
/// first row is what lets the offset from page space reach the geometry once (ADR 0219).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Band {
    /// First page row this band covers.
    top: u32,
    /// Number of rows.
    height: u32,
}

impl Band {
    /// The band covering `bounds`, clipped to the rows `surface` holds.
    ///
    /// Returns `None` when `bounds` covers no row of the surface.
    ///
    /// `bounds` is widened by a row before rounding outward. Clip bounds are computed
    /// from a path's control points and then transformed, while the mask is built by
    /// transforming the path itself; the two agree to within floating-point rounding
    /// rather than exactly, and a band one row short would erase a row of a shading
    /// that the clip admits. A spare row costs a fraction of a percent of the band and
    /// removes that class of defect entirely.
    fn covering(bounds: tiny_skia::Rect, surface: Surface) -> Option<Self> {
        let rows = bounds.outset(0.0, 1.0)?.round_out()?;
        let first = i32::try_from(surface.rows.top).ok()?;
        let limit = first.checked_add(i32::try_from(surface.rows.height).ok()?)?;
        let top = rows.top().clamp(first, limit);
        let bottom = rows.bottom().clamp(first, limit);
        let rows = u32::try_from(bottom.checked_sub(top)?).ok()?;
        (rows > 0).then_some(Self {
            top: u32::try_from(top).ok()?,
            height: rows,
        })
    }

    /// Borrows this band's rows of `pixmap`, which covers `surface`, as a pixmap of its own.
    ///
    /// `None` only if the band does not lie within the surface, which [`Band::covering`]
    /// does not produce.
    fn rows<'a>(
        self,
        pixmap: &'a mut tiny_skia::PixmapMut<'_>,
        surface: Surface,
    ) -> Option<tiny_skia::PixmapMut<'a>> {
        let width = pixmap.width();
        let stride = (width as usize).checked_mul(4)?;
        let start = (self.top.checked_sub(surface.rows.top)? as usize).checked_mul(stride)?;
        let end = start.checked_add((self.height as usize).checked_mul(stride)?)?;
        let rows = pixmap.data_mut().get_mut(start..end)?;
        tiny_skia::PixmapMut::from_bytes(rows, width, self.height)
    }

    /// Bytes a mask covering this band of a `width`-wide target occupies.
    fn mask_bytes(self, width: u32) -> usize {
        (self.height as usize).saturating_mul(width as usize)
    }
}

/// The rows of a page one call draws into, and the page they belong to.
///
/// # Why the page's transform travels and a strip's does not
///
/// A strip is a run of the page's rows, and the obvious way to hand one to a rasteriser is to
/// give it a target whose transform has been shifted up by the rows above it. That is what
/// this backend did from ADR 0139 until session 382, and **it made the picture depend on how
/// the page was divided**: `Transform::then` folds the shift into the page transform's `f`, so
/// a mark's own translation is then added to a number of a different magnitude and the sum
/// rounds elsewhere. One `ulp` of the composed matrix is one supersample of a glyph's edge,
/// which `tiny-skia` quantises to 16 of 255 — the pixel ADR 0219 was written for.
///
/// So the page's own target is what travels, and where the surface starts is a separate whole
/// number of rows, applied once and last by [`ToDevice`]. Every band, clip mask and group
/// buffer below is measured in page rows for the same reason.
#[derive(Debug, Clone, Copy)]
struct Surface {
    /// The whole page's target: `transform` maps page space onto the *page's* device grid,
    /// however few of its rows this surface holds.
    page: TargetSpec,
    /// The page rows it holds.
    rows: Band,
}

impl Surface {
    /// The surface covering a whole page.
    fn whole(page: TargetSpec) -> Self {
        Self {
            page,
            rows: Band {
                top: 0,
                height: page.height,
            },
        }
    }

    /// Pixels across, which is the page's width: a surface is a run of whole rows.
    fn width(self) -> u32 {
        self.page.width
    }

    /// The map onto the device grid of `band`, which must be rows this surface holds.
    fn to_device(self, band: Band) -> ToDevice {
        ToDevice {
            page: self.page.transform,
            top: band.top,
        }
    }
}

/// Page space onto the device grid of a surface starting at page row `top`.
///
/// Two values rather than one matrix, and that is the whole of ADR 0219: the row offset is
/// composed **last**, onto the fully composed matrix, where subtracting a whole number of rows
/// from a device coordinate is exact. Folded in first — as `target.transform.then(offset)` —
/// it changes the magnitude every later composition rounds at, and a page drawn in strips stops
/// being the page drawn whole.
#[derive(Debug, Clone, Copy)]
struct ToDevice {
    /// Page space to the whole page's device grid.
    page: Transform,
    /// First page row of the surface being drawn into.
    top: u32,
}

impl ToDevice {
    /// The matrix geometry stated in `transform`'s space is drawn under.
    fn of(self, transform: Transform) -> Transform {
        #[expect(
            clippy::cast_precision_loss,
            reason = "rasterize rejects a target taller than MAX_EXTENT = 2^24, and every \
                      integer below that is exactly representable as f32"
        )]
        let rows = self.top as f32;
        transform
            .then(self.page)
            .then(Transform::translate(0.0, -rows))
    }
}

/// The rectangle a fill should be drawn as instead, where its mask leaves room for less.
///
/// A rectangle wider than its mask can mark is drawn as the part that can be marked. `sh`
/// states one such fill per shading — ISO 32000-2 §8.7.4.2's Table 76 bounds the operator by
/// the current clipping path and by nothing else, so a display list carries the whole page —
/// and a rasteriser evaluates its shader over the *path's* spans and multiplies the mask in
/// afterwards, so the columns the mask rejects cost full price. The same table says the cost
/// out loud — an unbounded shading paints "across the entire clipping region, which may be
/// time-consuming" — which is the sentence this answers.
///
/// **The benchmark that justifies it**: `bug1721218_reduced.pdf`, the corpus's worst page,
/// states 3490 of those fills, and this is the difference between 10.4 M shaded pixels per
/// render and 85 608. Twenty renders through `examples/callgrind_rasterise` go **38.45 G
/// instructions to 20.03 G**, of which `tiny_skia::pipeline::lowp::gradient` alone is 15.78 G
/// to 0.58 G, and the page's own ink is unchanged to the byte. ADR 0236.
///
/// **What it costs in readability** is this function and one indirection at the call site. What
/// it costs a page it cannot help is one rectangle-containment test per clipped fill, which is
/// [`pdf_render::cropped_rectangle`]'s first line and is why the memoised hull is what it asks
/// for; the specification's own pages measure unchanged.
///
/// The exactness argument, and what a caller owes, are on [`pdf_render::cropped_rectangle`].
/// What is supplied here is the rectangle [`MaskCache::build`] measured, with the pixel of
/// margin [`Band::covering`] takes for the same reason — and `admits` is `None` wherever no
/// rectangle bounds the mask, which is every unclipped command and every bare soft mask.
fn crop_to_mask(
    source: &Path,
    transform: Transform,
    to_device: ToDevice,
    admits: Option<tiny_skia::Rect>,
) -> Option<Path> {
    pdf_render::cropped_rectangle(
        source,
        // The page's own device grid, which is the one `admits` is stated on: the band's row
        // offset is `ToDevice`'s to apply and belongs to neither of them.
        transform.then(to_device.page),
        convert::from_skia_rect(admits?),
    )
}

/// The buffer a transparency group's elements are drawn onto (ISO 32000-2 §11.4.4, §11.4.5).
///
/// Transparent for §11.4.5's isolated group, and a **copy of `pixmap`** for a non-isolated
/// one, whose elements composite onto the group's own backdrop. Copied rather than aliased:
/// the elements read it while they blend, and [`blend::interpolate`] reads the original
/// again to put the two together.
///
/// A non-isolated group carries three guarantees from `pdf-model` — `Command::Group`'s
/// `isolated` states them, and each is what makes §11.4.4's backdrop removal cancel against
/// §11.3.3's re-compositing — and this is where they are checked rather than assumed.
///
/// # Errors
///
/// [`CpuRasterError::Allocation`] if the buffer does not fit, and
/// [`CpuRasterError::UnsupportedCommand`] for a non-isolated group carrying anything the
/// collapse does not hold for.
fn initial_backdrop(
    pixmap: &tiny_skia::PixmapMut<'_>,
    surface: Surface,
    group: &Group<'_>,
) -> Result<tiny_skia::Pixmap, CpuRasterError> {
    if !group.isolated
        && (group.compose != Compose::Over
            || group.into != Compose::Over
            || group.blend != pdf_render::BlendMode::Normal)
    {
        return Err(CpuRasterError::UnsupportedCommand(
            "a non-isolated group whose result is not composited by §11.3.3's Normal blend \
             function needs Table 140's group alpha kept apart from the composite alpha \
             (ISO 32000-2 §11.4.4 NOTE 4)"
                .to_owned(),
        ));
    }
    let mut buffer = tiny_skia::Pixmap::new(surface.width(), surface.rows.height).ok_or(
        CpuRasterError::Allocation {
            width: surface.width(),
            height: surface.rows.height,
        },
    )?;
    if group.isolated {
        return Ok(buffer);
    }
    buffer.as_mut().draw_pixmap(
        0,
        0,
        pixmap.as_ref(),
        &tiny_skia::PixmapPaint {
            opacity: 1.0,
            blend_mode: tiny_skia::BlendMode::Source,
            quality: tiny_skia::FilterQuality::Nearest,
        },
        tiny_skia::Transform::identity(),
        None,
    );
    Ok(buffer)
}

/// Whether a command's own extent lies wholly above or below `surface`'s rows.
///
/// A group answers `false`: its elements carry the extents and each is asked in turn.
///
/// Measured on the *page's* grid, so a strip asks the same question of the same numbers the
/// whole page asks — only over a different run of rows.
fn misses_surface(command: &Command, surface: Surface) -> bool {
    let Some(bounds) = command.device_bounds(surface.page.transform) else {
        return false;
    };
    #[expect(
        clippy::cast_precision_loss,
        reason = "a target height below MAX_EXTENT = 2^24 is exact in f32"
    )]
    let (top, bottom) = (
        surface.rows.top as f32,
        surface.rows.top.saturating_add(surface.rows.height) as f32,
    );
    bounds.max.y < top - 1.0 || bounds.min.y > bottom + 1.0
}

/// The rows at which to cut this target, or one strip's worth if it may not be cut.
///
/// The strip count asked for is what this machine offers, bounded by [`MAX_STRIPS`] and by
/// [`MIN_STRIP_ROWS`]; what comes back may be fewer, because a cut is only made at a row no
/// curve crosses. **The picture is very nearly independent of the answer**, which is the property
/// ADR 0139 exists to establish and `strip_parallelism.rs` asserts: a machine with four cores and
/// one with thirty-two draw the same bytes but for a handful in a million.
///
/// **"Very nearly" is a correction sessions 381 and 382 made together**, and both halves are worth
/// carrying. The 381st found the claim false — `doc/PDF20_AN001-BPC.pdf` page 1 differed by one
/// pixel between one strip and every division above one — by drawing a page in one strip on
/// purpose, which a confined process must because it may not ask how many cores it has (ADR 0218).
/// The 382nd found the cause not to be the one written down: a strip's offset was folded into the
/// page transform *before* a mark's own transform was composed with it, so the sum rounded at
/// another magnitude. It is applied last now, and that page is exact — see [`ToDevice`].
///
/// What survives is a dependency's. `tiny-skia` maps a point as `y·sy + ty`, and shifting `ty` by
/// a whole number of rows moves the sum into another binade; ADR 0219 measures what is left —
/// fewer than one pixel in ten thousand, none by more than one supersample — and says why no
/// arrangement of this crate's arithmetic closes it.
fn plan_strips(list: &DisplayList, target: TargetSpec, asked: Option<u32>) -> Vec<u32> {
    // Asked of the machine only where the caller did not say, and that is not tidiness:
    // `available_parallelism` reads `/proc/self/cgroup` on Linux, and a caller drawing inside a
    // confinement with no filesystem is *killed* for it rather than told no. Such a caller states
    // the number with `with_strips` (ADR 0218).
    let wanted = asked
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map_or(1, std::num::NonZero::get)
                .try_into()
                .unwrap_or(MAX_STRIPS)
        })
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

/// A built clip mask, the band it covers, and the device rectangle it can mark within.
struct Built {
    mask: tiny_skia::Mask,
    band: Band,
    /// Device rectangle outside which this mask is zero, or `None` where nothing bounds it.
    ///
    /// The band already says which *rows* the mask can mark, and a surface is a run of whole
    /// rows, so the columns had nowhere to be recorded until a caller wanted them. One does:
    /// [`pdf_render::cropped_rectangle`] shrinks a rectangular fill to the part of it that can
    /// survive, and this is what tells it where that part is. Outset and rounded outward
    /// exactly as [`Band::covering`] treats the rows, and for the same reason — a clip's bounds
    /// are composed one way and its mask drawn through another.
    admits: Option<tiny_skia::Rect>,
}

/// What a command's clip and soft mask together let it mark.
///
/// Three answers to one question, which is why they travel together: which rows of the surface
/// are worth drawing into, what coverage multiplies the command there, and — since the surface
/// is a run of whole rows and the band cannot say it — which columns.
#[derive(Debug, Clone, Copy)]
struct Admitted<'a> {
    /// Rows of the surface the command may mark.
    band: Band,
    /// The coverage it is drawn through, or `None` where nothing masks it.
    mask: Option<&'a tiny_skia::Mask>,
    /// Device rectangle outside which `mask` is zero, or `None` where nothing bounds it.
    admits: Option<tiny_skia::Rect>,
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
/// distinct clips as it likes — the corpus's worst holds 3554 on one page — so keeping
/// every mask is a memory-exhaustion vector: before this bound and before [`Band`],
/// that page held 1.7 GB of page-sized masks. Dropping an entry costs a rebuild and
/// nothing else, so eviction can be crude, and it is: entries go in build order,
/// oldest first, and the most recently built is never dropped. Clips are used in runs,
/// so build order tracks use order closely enough that an active clip is rebuilt at
/// most once per run.
struct MaskCache {
    surface: Surface,
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
/// first page in the 974-document pdf.js corpus, and it never evicts: **3554 clip masks built
/// per render of one 612×792 page, peaking at 12.31 MB against this 32 MB** — 10.85 MB of clip
/// masks, 1.45 MB of soft masks, and three clip × soft-mask products alive at the peak.
///
/// **That figure said 27.9 MB and a margin of 13% until the three-hundred-and-ninety-ninth
/// session, which measured it again and found 12.31.** The 27.9 was taken in the
/// hundred-and-thirteenth and was true then; ADR 0132 arrived in the hundred-and-forty-seventh
/// and made [`DisplayList::add_clip`] return an existing identifier for an identical region,
/// which is a change to how many masks this cache is asked for. **A margin recorded as thin is
/// a claim that decays like any other**, and the reason to re-take it rather than inherit it is
/// [`doc/todo/40`](../../../doc/todo/40-mask-chain-crop.md), whose stated blocker was that this
/// page had no room for its intermediate clips. It has 19.7 MB of room, and they cost 9.4.
///
/// The number is not lowered on that evidence, because nothing has been measured to be paying
/// for it; it is recorded so that a session which finds a document evicting knows what the
/// margin was.
///
/// [`DisplayList::add_clip`]: pdf_render::DisplayList::add_clip
const MASK_BUDGET: usize = 32 << 20;

impl MaskCache {
    /// A cache for one target, bounded by `budget` bytes.
    ///
    /// The budget is a parameter rather than [`MASK_BUDGET`] itself because a page drawn in
    /// strips has one cache per strip and they are alive at once: dividing the constant keeps
    /// a parallel render's mask memory equal to a serial one's.
    fn new(surface: Surface, anti_alias: bool, budget: usize) -> Self {
        Self {
            surface,
            anti_alias,
            built: HashMap::new(),
            order: VecDeque::new(),
            soft_order: VecDeque::new(),
            bytes: 0,
            soft_bytes: 0,
            budget,
        }
    }

    /// Returns what a command with this clip and this soft mask may mark.
    ///
    /// `None` means nothing this command draws can survive, which is not the same as
    /// drawing unmasked; an [`Admitted`] whose `mask` is `None` is the unmasked case, where
    /// the band is the whole target.
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
    ) -> Result<Option<Admitted<'_>>, CpuRasterError> {
        match (clip, mask) {
            (None, None) => Ok(Some(Admitted {
                band: self.surface.rows,
                mask: None,
                admits: None,
            })),
            (Some(clip), None) => Ok(self.get(list, clip)?.map(|built| Admitted {
                band: built.band,
                mask: Some(&built.mask),
                admits: built.admits,
            })),
            (None, Some(mask)) => {
                let entry = self.soft_mask(mask)?;
                Ok(Some(Admitted {
                    band: entry.band,
                    mask: Some(&entry.mask),
                    admits: entry.admits,
                }))
            }
            (Some(clip), Some(mask)) => {
                self.combine(list, clip, mask)?;
                let entry = self
                    .built
                    .get(&Key::Both(clip, mask))
                    .ok_or(CpuRasterError::UnknownClip(clip))?
                    .as_ref();
                Ok(entry.map(|built| Admitted {
                    band: built.band,
                    mask: Some(&built.mask),
                    admits: built.admits,
                }))
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
        // The product is zero wherever the clip is, so the clip's rectangle bounds it too; a
        // soft mask has a value everywhere (§11.6.5.1) and bounds nothing on its own.
        let admits = clipped.admits;
        let mut product = clipped.mask.clone();
        let width = self.surface.width() as usize;
        // The soft mask covers the whole surface and the clip a band of it, both counted in
        // page rows, so the difference is where the clip's rows start within the mask.
        let start = (band.top.saturating_sub(self.surface.rows.top) as usize).saturating_mul(width);
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
                admits,
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
    fn admit_soft_mask(&mut self, id: SoftMaskId, mask: tiny_skia::Mask, band: Band) {
        self.soft_bytes = self
            .soft_bytes
            .saturating_add(band.mask_bytes(self.surface.width()));
        self.soft_order.push_back(id);
        self.built.insert(
            Key::Soft(id),
            Some(Built {
                mask,
                band,
                // A mask has a value everywhere — §11.6.5.1 gives one outside its group's
                // bounding box — so no rectangle bounds where it is non-zero.
                admits: None,
            }),
        );

        while self.soft_bytes > self.budget && self.soft_order.len() > 1 {
            let Some(oldest) = self.soft_order.pop_front() else {
                break;
            };
            if let Some(Some(entry)) = self.built.remove(&Key::Soft(oldest)) {
                self.soft_bytes = self
                    .soft_bytes
                    .saturating_sub(entry.band.mask_bytes(self.surface.width()));
            }
        }
    }

    /// Returns the mask for `id`, the band it covers and the rectangle it marks within,
    /// building it if needed.
    ///
    /// `None` means the clip admits no row of the target: the caller must draw
    /// nothing at all, which is not the same as drawing unclipped.
    fn get(&mut self, list: &DisplayList, id: ClipId) -> Result<Option<&Built>, CpuRasterError> {
        if !self.built.contains_key(&Key::Clip(id)) {
            let chain = Self::resolve_chain(list, id)?;
            let built = self.build(list, id, &chain)?;
            self.admit(Key::Clip(id), built);
        }

        // Absence here would mean `admit` dropped the entry it had just stored, which
        // it does not do; reporting it beats drawing an unclipped page if it ever did.
        Ok(self
            .built
            .get(&Key::Clip(id))
            .ok_or(CpuRasterError::UnknownClip(id))?
            .as_ref())
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
    /// bounds, so a child's band is always contained in its parent's. A parent's rows for the
    /// child's band would then be the prefix's contribution, and a chain could be one crop plus
    /// one `intersect_path` instead of a fill plus depth-minus-one intersects.
    ///
    /// **Three things about that were re-derived in the three-hundred-and-ninety-ninth session
    /// and all three moved**, which is why the item is still open and why it is
    /// [`doc/todo/40`](../../../doc/todo/40-mask-chain-crop.md) rather than a line here:
    ///
    /// - **It is worth about 17% of the page and not "most of `MaskCache::get`'s 24.3%".**
    ///   Intermediates are barely shared: 3551 leaf clips on `bug1721218_reduced.pdf` reach
    ///   through **7066 distinct nodes**, so building each node once replaces 3551 fills and
    ///   10 702 intersects with 7065 intersects and as many crops — 42% of this function.
    /// - **It is not blocked on memory.** That was the stated blocker, on a peak of 27.9 MB
    ///   against [`MASK_BUDGET`]; the peak is **12.31 MB** and the intermediates cost 9.4.
    /// - **It is not obviously pixel-exact, and the sentence above is where that hides.** A
    ///   mask value at a given device row *does* depend on which band holds it, because
    ///   [`ToDevice`] composes the band's first row into the translation and ADR 0219 measures
    ///   what shifting a whole number of rows does to `y·sy + ty` — fewer than one pixel in ten
    ///   thousand, none by more than one supersample, but not nothing. A parent's mask rows are
    ///   therefore *nearly* the prefix's contribution for the child's band, and this backend is
    ///   the oracle. Taking the item means either building intermediates in the child's band or
    ///   proving the difference away.
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
                clip.transform.then(self.surface.page.transform),
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
            Some(bounds) => match Band::covering(bounds, self.surface) {
                Some(band) => band,
                // The clip admits no row of this surface at all.
                None => return Ok(None),
            },
            // Nothing in the chain could be measured, so nothing bounds the band.
            None => self.surface.rows,
        };
        // The columns, kept for the same reason the rows are and treated the same way: a whole
        // pixel of margin before rounding outward, because `bounds` composes the clip's
        // transform with the page's while the mask below composes the band's in as well, and
        // the two agree to within rounding rather than exactly. See [`Band::covering`].
        //
        // A rectangle covering the whole surface is recorded as `None`, which is not tidiness:
        // it is what keeps [`crop_to_mask`] off a page that has nothing to gain. The commonest
        // clip in the corpus is a page-wide rectangle — the specification's own page 6 wraps
        // 303 text runs in one, which `DisplayList::add_clip` folds into a single identifier
        // (ADR 0132) — and answering here costs one containment test per chain *built* where
        // answering in the fill would cost one per fill drawn. Measured: page 6's rasterisation
        // is 4 004.7 M without the crop and 4 021.7 M with it asked per fill, which is the
        // whole of the +0.42%.
        let admits = bounds
            .and_then(|bounds| bounds.outset(1.0, 1.0))
            .and_then(|bounds| bounds.round_out())
            .map(|rounded| rounded.to_rect())
            .filter(|admits| !self.covers_the_surface(*admits));

        let to_band = self.surface.to_device(band);
        let mut mask = tiny_skia::Mask::new(self.surface.width(), band.height).ok_or(
            CpuRasterError::Allocation {
                width: self.surface.width(),
                height: band.height,
            },
        )?;
        // A fresh mask blocks everything, so filling the root path is what opens it.
        scan::mask_fill(
            &mut mask,
            &root.path,
            root.fill_rule,
            self.anti_alias,
            convert::transform(to_band.of(root.transform)),
        );
        for shape in nested {
            scan::mask_intersect(
                &mut mask,
                &shape.path,
                shape.fill_rule,
                self.anti_alias,
                convert::transform(to_band.of(shape.transform)),
            );
        }

        Ok(Some(Built { mask, band, admits }))
    }

    /// Whether a device rectangle reaches every pixel of this surface.
    ///
    /// Measured on the *page's* grid, which is where a clip's bounds are computed and where
    /// this surface's own rows are counted, so a strip asks the question of the same numbers
    /// the whole page asks.
    fn covers_the_surface(&self, rect: tiny_skia::Rect) -> bool {
        #[expect(
            clippy::cast_precision_loss,
            reason = "rasterize rejects a target larger than MAX_EXTENT = 2^24, and every \
                      integer below that is exact in f32"
        )]
        let (top, bottom) = (
            self.surface.rows.top as f32,
            self.surface
                .rows
                .top
                .saturating_add(self.surface.rows.height) as f32,
        );
        #[expect(
            clippy::cast_precision_loss,
            reason = "as above, for the width this surface spans"
        )]
        let right = self.surface.width() as f32;
        rect.left() <= 0.0 && rect.top() <= top && rect.right() >= right && rect.bottom() >= bottom
    }

    /// Stores an entry and evicts oldest-first until the budget is met.
    fn admit(&mut self, id: Key, built: Option<Built>) {
        if let Some(entry) = &built {
            self.bytes = self
                .bytes
                .saturating_add(entry.band.mask_bytes(self.surface.width()));
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
                    .saturating_sub(entry.band.mask_bytes(self.surface.width()));
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

    use super::{Band, MASK_BUDGET, MaskCache, Surface};

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
        let mut cache = MaskCache::new(Surface::whole(target), true, MASK_BUDGET);
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

        let mut generous = MaskCache::new(Surface::whole(target), true, MASK_BUDGET);
        let before = generous
            .get(&list, first)
            .expect("builds")
            .map(|built| (built.band, built.admits, built.mask.data().to_vec()));

        let mut tight = MaskCache::new(Surface::whole(target), true, MASK_BUDGET);
        tight.budget = 1;
        for &id in &ids {
            tight.get(&list, id).expect("builds");
        }
        let after = tight
            .get(&list, first)
            .expect("rebuilds")
            .map(|built| (built.band, built.admits, built.mask.data().to_vec()));

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

        let mut cache = MaskCache::new(Surface::whole(target), true, MASK_BUDGET);
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
        let surface = Surface::whole(target);
        let mut cache = MaskCache::new(surface, true, MASK_BUDGET);
        cache.budget = Band { top: 0, height: 4 }.mask_bytes(target.width) * 2;

        let mask = SoftMaskId::new(0);
        cache.admit_soft_mask(
            mask,
            tiny_skia::Mask::new(target.width, target.height).expect("a target-sized mask"),
            surface.rows,
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

    /// A strip's matrix is the page's matrix with a whole number of rows taken off, exactly.
    ///
    /// This is the whole of ADR 0219 in one assertion, and it is stated in `f64` so that it is
    /// about the arithmetic rather than about itself: recomputing the subtraction in `f32`
    /// would round the same way twice and prove nothing. Every component but `f` must be the
    /// page's own bits, and `f` must be the page's `f` minus the offset with no rounding at all
    /// — which is what makes composing the offset *last* different from folding it into the
    /// page transform first, where the mark's own translation is then added at another
    /// magnitude and rounds elsewhere.
    #[test]
    fn the_offset_is_composed_last_and_costs_nothing() {
        // A page transform of the shape `TargetSpec::for_page` builds, and mark transforms of
        // the shapes a content stream states: a glyph at 8 and at 20 points, and a form's.
        let pages = [
            Transform::new(1.0, 0.0, 0.0, -1.0, 0.0, 841.89),
            Transform::new(0.838_926_2, 0.0, 0.0, -0.838_926_2, 0.0, 706.283_57),
            Transform::new(2.019_7, 0.0, 0.0, -2.019_7, 0.0, 1_683.783_4),
        ];
        let marks = [
            Transform::IDENTITY,
            Transform::new(8.0, 0.0, 0.0, 8.0, 488.387_54, 51.023_6),
            Transform::new(20.0, 0.0, 0.0, 20.0, 51.023_6, 51.023_6),
            Transform::new(0.5, 0.25, -0.25, 0.5, 133.777, 642.101),
        ];

        for page in pages {
            for mark in marks {
                let whole = super::ToDevice { page, top: 0 }.of(mark);
                for top in [1_u32, 8, 64, 236, 512, 887, 1024] {
                    let strip = super::ToDevice { page, top }.of(mark);
                    assert_eq!(
                        (
                            strip.a.to_bits(),
                            strip.b.to_bits(),
                            strip.c.to_bits(),
                            strip.d.to_bits(),
                            strip.e.to_bits()
                        ),
                        (
                            whole.a.to_bits(),
                            whole.b.to_bits(),
                            whole.c.to_bits(),
                            whole.d.to_bits(),
                            whole.e.to_bits()
                        ),
                        "a row offset moved something other than the vertical translation"
                    );
                    #[expect(
                        clippy::float_cmp,
                        reason = "exactly what is asserted: the subtraction of a whole number \
                                  of rows from a device coordinate rounds not at all, so an \
                                  epsilon here would assert nothing"
                    )]
                    let exact = (f64::from(whole.f) - f64::from(top)) == f64::from(strip.f);
                    assert!(
                        exact,
                        "{top} rows off {whole:?} gave {strip:?}, which is not the page's \
                         translation minus {top}: the subtraction rounded"
                    );
                }
            }
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
