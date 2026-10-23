//! The walk over a display list: each command clipped to the rows its clip admits, then handed to
//! the kernel that draws its kind.
//!
//! [`CpuRasterizer::encode`] is the loop every strip, every group and every soft mask runs over its
//! own commands; [`CpuRasterizer::draw`] is the dispatch from one command to the fill, stroke or
//! image kernel. What a strip is, and how a target is cut into them, is `crate::strips`'.

use pdf_render::{BackendError, Command, DisplayList, Paint, Stroke, Transform};

use crate::compose::Compose;
use crate::groups::Group;
use crate::image::ImagePlacement;
use crate::mask_cache::{Admitted, MaskCache};
use crate::surface::{Surface, ToDevice, misses_surface};
use crate::{CpuRasterError, CpuRasterizer, blend, scan};

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
    ///
    /// [`CpuRasterizer::rasterize`]: pdf_render::Rasterizer::rasterize
    /// [`MAX_GROUP_DEPTH`]: pdf_render::MAX_GROUP_DEPTH
    #[expect(
        clippy::too_many_arguments,
        reason = "a display list, where it is drawn, and the two things a recursion carries \
                  — how deep it is and how its elements combine. Grouping them would put \
                  the target and the mask cache in a struct that exists once per call"
    )]
    pub(crate) fn encode(
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
            // **The one place a draw already in progress can be interrupted**, and it is per
            // command rather than per strip because a strip is not a unit of time: a page whose
            // curves forbid every cut is one strip, and the amplification fixture's ten thousand
            // page-covering fills are one command each. Every recursion — a group, a shaped pair,
            // a soft mask's own list — comes back through this loop, so one check covers all of
            // them. See [`CpuRasterizer::interruptible`] for what it buys and what it cannot
            // reach.
            if self.interrupted() {
                return Err(BackendError::Interrupted.into());
            }

            // A command whose extent misses this surface marks nothing, and saying so here is
            // what makes a strip cost what its own rows cost: without it every strip would
            // build every command's path and compile every command's pipeline, which ADR 0137
            // measured at 19% of a dense page's rasterisation. A row of margin, for the
            // same reason `Band::covering` takes one — the extent comes from control points
            // and the mask from the path.
            if misses_surface(command, surface) {
                continue;
            }

            self.encode_command(pixmap, list, command, surface, masks, depth, compose)?;
        }
        Ok(())
    }

    /// Draws one command of a list onto `surface` — [`CpuRasterizer::encode`]'s loop body, past
    /// the test of whether the command reaches the surface at all, which its two callers ask in
    /// two ways: a list met in a recursion measures each command's bound as it goes, and the
    /// page's own list asks the bounds measured once for the target (ADR 1841).
    ///
    /// # Errors
    ///
    /// As [`CpuRasterizer::encode`].
    #[expect(
        clippy::too_many_arguments,
        reason = "one step of `encode`, carrying exactly what the recursion carries"
    )]
    pub(crate) fn encode_command(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        list: &DisplayList,
        command: &Command,
        surface: Surface,
        masks: &mut MaskCache,
        depth: usize,
        compose: Compose,
    ) -> Result<(), CpuRasterError> {
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
            return Ok(());
        }

        // §11.4.6's bare draw is Porter-Duff Source modulated by the coverage, and the clip
        // a command carries is part of that coverage (§8.5.4). `tiny-skia` applies a clip
        // mask to Source by *scaling the source* rather than by interpolating, so wherever
        // the mask is below 1.0 inside the path's reach the accumulated result is cleared
        // rather than kept — a glyph-shaped clip on a tile knocked out a ring of the glyph
        // beneath it. Where the clip cuts the mark, the element is drawn as the clause's two
        // stages instead, with an opaque twin of itself as the shape: `(1 − f) × P + S`,
        // which is what the bare draw means and what Destination-Out then Plus compute
        // through a mask exactly (ADR 1320).
        //
        // **A path is drawn in the two stages whether or not a clip cuts it**, because the
        // bare draw's coverage is the library's: Source is the one mode `scan::intersected`
        // cannot deliver a measured coverage under, so a knockout element's edges were the
        // supersampled converter's lattice of sixteenths where every other mark's are the
        // area `crate::area` measures — `f` read a quarter for a 0.13-pixel band. The two
        // stages are Over-family modes, so both reach that measurement (ADR 2286).
        if compose == Compose::Knockout
            && (matches!(command, Command::Fill { .. } | Command::Stroke { .. })
                || command.clip().is_some_and(|clip| {
                    !masks.cuts_nothing(list, clip, marks_reach(command, surface.page.transform))
                }))
            && let Some(shape) = coverage_twin(command)
        {
            self.encode_shaped(
                pixmap,
                list,
                (command, &shape),
                surface,
                masks,
                depth,
                compose,
            )?;
            return Ok(());
        }

        // A group is the one command that needs the mask cache mutably *while* it
        // draws, so it cannot hold a clip mask borrowed from it across the recursion.
        // It therefore resolves its own clip, twice, either side of its elements.
        if matches!(command, Command::Group { .. }) {
            self.encode_group_command(pixmap, list, command, surface, masks, depth, compose)?;
            return Ok(());
        }

        // How far this command's marks reach, in the page's own space: what decides whether
        // §10.7.4's intersection with its clip is the mark itself, and so whether the clip
        // is composed into it at all ([`MaskCache::cuts_nothing`]).
        let reach = marks_reach(command, surface.page.transform);
        // Resolved before the match so that every arm shares one code path for
        // clip handling; a per-arm lookup would be a place for them to diverge.
        // The clip admits no row of the target, so nothing this command draws can
        // survive it.
        let Some(Admitted {
            band,
            mask: clip,
            admits,
        }) = masks.effective(list, command.clip(), command.mask(), reach)?
        else {
            return Ok(());
        };

        // Everything below draws into the band rather than the page, which is what
        // keeps a command's cost proportional to the pixels its clip can admit.
        // The device map carries the band's first row so that geometry, paints
        // and images all move together; missing one would tear the page apart in a
        // way no metric would notice, so there is exactly one of these.
        let to_device = surface.to_device(band);

        // ISO 32000-2 §11.3.5.3's four modes are computed by this backend rather than
        // by `tiny-skia`, whose three of them are wrong (ADR 0047), and §11.7.4.3's
        // special overprinting blend mode is computed here because no rasteriser has
        // it — so such a command is drawn onto transparency first and composited by
        // `blend::composite`. Drawing onto transparency loses nothing: with αb = 0 the
        // compositing formula collapses to the source, whatever the blend mode.
        if let Some(mode) = compose.computed(command.blend()) {
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
            return Ok(());
        }

        let mut rows = band
            .rows(pixmap, surface)
            .ok_or(CpuRasterError::Allocation {
                width: surface.width(),
                height: band.height,
            })?;

        self.draw(&mut rows, command, to_device, (clip, admits), compose)?;
        Ok(())
    }

    /// Unpacks one [`Command::Group`] and hands it to [`CpuRasterizer::draw_group`].
    ///
    /// [`CpuRasterizer::encode`]'s group arm, split out so that the recursion stays
    /// readable at a glance; the argument list is the recursion's own.
    ///
    /// # Errors
    ///
    /// As [`CpuRasterizer::encode`]. A *bare* group under knockout would have to be
    /// composited by its shape and has none stated — a group whose shape is needed arrives
    /// as one half of a [`Command::Shaped`] — so it is refused rather than approximated:
    /// `pdf-model` does not build that display list, and erroring is what keeps the
    /// assumption from becoming a silence if it ever does.
    #[expect(
        clippy::too_many_arguments,
        reason = "one arm of `encode`, carrying exactly what the recursion carries"
    )]
    fn encode_group_command(
        &self,
        pixmap: &mut tiny_skia::PixmapMut<'_>,
        list: &DisplayList,
        command: &Command,
        surface: Surface,
        masks: &mut MaskCache,
        depth: usize,
        compose: Compose,
    ) -> Result<(), CpuRasterError> {
        let Command::Group {
            commands,
            alpha,
            blend,
            isolated,
            knockout,
            alpha_is_shape,
            blending,
            ..
        } = command
        else {
            // The one caller matches `Command::Group` first; anything else arriving here
            // is a programming error, and a loud one beats a skipped command.
            return Err(CpuRasterError::UnsupportedCommand(format!(
                "encode_group_command was handed {command:?}"
            )));
        };
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
                alpha_is_shape: *alpha_is_shape,
                compose: if *knockout {
                    Compose::Knockout
                } else {
                    Compose::Over
                },
                into: compose,
                blending: blending.as_deref(),
            },
            surface,
            masks,
            depth,
        )
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
        (clip, admits): (scan::Clip<'_>, Option<tiny_skia::Rect>),
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

/// The same mark at full opacity, whose drawn alpha is therefore its coverage — the shape a
/// bare knockout element knocks out with (ISO 32000-2 §11.4.6, §11.6.4.2).
///
/// A bare element is one `pdf-model` found opaque apart from a solid colour's constant alpha, so
/// a solid paint goes to white and the rest is the mark itself with its constant lifted. `None`
/// for anything else, which keeps the bare draw.
fn coverage_twin(command: &Command) -> Option<Command> {
    let mut twin = command.clone();
    match &mut twin {
        Command::Fill { paint, blend, .. } | Command::Stroke { paint, blend, .. } => {
            match paint {
                Paint::Solid(_) => *paint = Paint::Solid(pdf_render::Color::WHITE),
                Paint::Shading(shading) if shading.is_opaque() => {}
                _ => return None,
            }
            *blend = pdf_render::BlendMode::Normal;
        }
        Command::Image {
            image,
            alpha,
            blend,
            ..
        } if image.is_opaque() => {
            *alpha = 1.0;
            *blend = pdf_render::BlendMode::Normal;
        }
        _ => return None,
    }
    Some(twin)
}

/// How far a command's marks reach in the page's own space, or `None` where this cannot say.
///
/// The question [`MaskCache::cuts_nothing`] asks its clip about, and **it is asked about the shape
/// the document states, not about the one this backend paints.** ISO 32000-2 §10.7.4 makes the
/// two differ — the clause's substitutions widen a mark too thin to measure, so that "no shape
/// ever disappears" — and the clause itself ranks the two answers where a clip meets the wider
/// one:
///
/// > The area covered by painted pixels shall always be at least as large as the area of the
/// > original shape.
///
/// With the document's shape `S` inside the region `C`, §10.7.4's own intersection leaves `S`
/// whole, so the area owed is `S`'s. Cutting the substitute back to `C` paints *less* than that —
/// a rule half a device pixel tall inside a clip half a device pixel tall keeps three quarters of
/// its ink on `bug1978317.pdf`, 15 004 times over — which is the side of the sentence above that
/// is forbidden. So the substitute is drawn whole, and what it puts outside `C` is the half device
/// pixel ADR 0268 and ADR 0154 already spend on an unclipped mark. ADR 1095.
///
/// - A **fill** marks its own path, so its page-space box is the shape the document states.
/// - A **stroke** is bounded by [`pdf_render::stroked_bounds`] at the width §8.4.3.2 and §10.7.5
///   resolve — [`Stroke::device_width`], asked against the *real* device so that a `0 w` rule and
///   a stroke-adjusted one are measured where they are drawn rather than at page scale.
/// - An **image** fills §8.9.5.2's unit square and [`Command::device_bounds`] maps its corners.
/// - A **group** keeps its clip: §8.5.4 gives its result "the union of the shapes of its
///   constituent objects" and nothing here bounds that union without walking the elements. So
///   does §11.4.6's shaped element, and so does every variant [`Command`] gains after this.
///
/// [`Stroke::device_width`]: pdf_render::Stroke::device_width
fn marks_reach(command: &Command, page: Transform) -> Option<pdf_render::Rect> {
    match command {
        Command::Fill {
            path, transform, ..
        } => path.bounds(*transform),
        Command::Stroke {
            path,
            transform,
            stroke,
            ..
        } => {
            let stated = Stroke {
                // Already resolved against the device, so the copy may not resolve it again:
                // `stroked_bounds` asks `device_width` a second time, in the path's own space.
                adjust: false,
                width: stroke.device_width(transform.then(page)),
                ..stroke.clone()
            };
            pdf_render::stroked_bounds(path, &stated, *transform)
        }
        Command::Image { .. } => command.device_bounds(Transform::IDENTITY),
        _ => None,
    }
}
