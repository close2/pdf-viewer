//! Transparency groups — ISO 32000-2 §11.4: a group's elements drawn onto a buffer of their own
//! and composited onto the page once, isolated or not, knockout or not.
//!
//! An isolated group starts on transparency (§11.4.5) and a non-isolated one on a copy of the page
//! (§11.4.4); [`CpuRasterizer::draw_group`] states how each comes back.

use pdf_render::{BackendError, ClipId, Command, DisplayList, MAX_GROUP_DEPTH, SoftMaskId};

use crate::compose::Compose;
use crate::mask_cache::{Admitted, MaskCache};
use crate::surface::{Band, Surface, band_pixels};
use crate::{CpuRasterError, CpuRasterizer, blend, scan};

impl CpuRasterizer {
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
    /// §11.4.4's own model, and comes back one of two ways.
    ///
    /// Under §11.3.3's **Normal** blend function at the `Do` it comes back as the
    /// interpolation `Command::Group`'s `isolated` derives: `(1 − w) × page + w × buffer`,
    /// with `w` the group's constant alpha times its soft mask at the pixel — §11.4.4's
    /// removal and the re-compositing having cancelled. Outside the group's marks the buffer
    /// still *is* the page, and the interpolation of a value with itself is that value, so the
    /// copy costs the region nothing.
    ///
    /// Under **any other mode** nothing cancels, so §11.4.4's result step is performed for
    /// itself ([`CpuRasterizer::remove_the_backdrop`]) and the group is then painted like any
    /// other single object — the clause's own sentence, that the result "shall then be treated
    /// as if it were a single object, which in turn is composited with the group's backdrop".
    ///
    /// # Errors
    ///
    /// Returns [`CpuRasterError::UnsupportedCommand`] for a non-isolated group carrying
    /// anything the collapse above does not hold for; `pdf-model` guarantees it does not
    /// build one, and refusing keeps that from becoming a silence.
    pub(crate) fn draw_group(
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
        let Some(Admitted { band, .. }) = masks.group_admits(list, group.clip, group.mask)? else {
            return Ok(());
        };

        // §11.6.6, §11.7.2: a group with a blending colour space of its own is composited
        // in it and converted out at the end — two passes over the same geometry, resolved
        // per pixel, exactly the page-level construction in `rasterize` one scope down.
        // §11.4.6 with `isolated: false` is the other construction with a buffer discipline
        // of its own; everything else is one buffer and one pass.
        let mut buffer = self.group_buffer(pixmap, list, &group, (surface, band), masks, depth)?;

        // The elements may have evicted the soft mask this group is painted through, since
        // they share the cache; rebuilding it is what makes eviction safe here as it is for
        // a clip.
        if let Some(id) = group.mask {
            self.build_soft_mask(list, id, surface, masks, depth)?;
        }

        // Resolved again rather than held across the recursion: the elements' own clips
        // share this cache and may have evicted the entry, and a rebuilt mask is the mask
        // that was dropped — see `a_rebuilt_mask_is_the_mask_that_was_evicted`.
        let Some(Admitted { mask: clip, .. }) = masks.group_admits(list, group.clip, group.mask)?
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

        // The mask the blit still owes, which is none where §8.5.4's intersection has already
        // folded it into the buffer — see `group_blit_mask`.
        let blit_mask = group_blit_mask(&mut buffer, &group, (surface, band, top), clip);

        // §11.4.4's own model, for a group whose elements were drawn onto a copy of the
        // page: `(1 − w) × page + w × buffer`, with `w` the group's constant alpha times its
        // soft mask at the pixel. One pass over the band rather than two Porter-Duff draws,
        // for the rounding reason `blend::interpolate` records. The whole band is walked
        // because outside the group's marks the buffer *is* the page and the interpolation
        // has to put it back unchanged — which it does, exactly, and cheaply.
        // Only under §11.3.3's Normal blend function: the interpolation *is* the removal
        // composited back, and a group whose `Do` states another mode had its backdrop
        // removed above and is composited as one object by the blit below.
        if !group.isolated && group.blend == pdf_render::BlendMode::Normal {
            let from_row = band.top.saturating_sub(surface.rows.top);
            let mut rows = band
                .rows(pixmap, surface)
                .ok_or(CpuRasterError::Allocation {
                    width: surface.width(),
                    height: band.height,
                })?;
            // A **non-isolated** group's raster is still composited through its clip as a
            // product, and here that is the buffer's own construction rather than a reading
            // of §10.7.4: §8.5.4 does give a group a shape for the clip to intersect — "the
            // shape of a transparency group … shall be influenced … by the [clipping path] in
            // effect at the time the group's results are painted onto its backdrop" — and this
            // buffer started as a copy of the page, so what its alpha holds is the backdrop's
            // unioned with the group's and not that shape. The interpolation buries the
            // group's own shape inside `E(B)` besides, where no factor can reach it.
            // `doc/todo/11` item 4 carries it; ADR 0492 pays the isolated case above.
            blend::interpolate(&mut rows, &buffer, from_row, paint.opacity, clip.mask());
            return Ok(());
        }

        // §11.3.5.3's four modes are this backend's own (ADR 0047) and §11.7.4.3's is in no
        // rasteriser at all, and a group reaches both by the same two steps a command does:
        // the group's clip, alpha and offset are applied onto transparency, and the result is
        // composited by `blend`. The extra buffer is the price of the mode, not of every group.
        if let Some(mode) = group.into.computed(group.blend) {
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
                blit_mask,
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
            blit_mask,
        );
        Ok(())
    }

    /// The buffer a group's elements are composited into, drawn and ready to be painted onto
    /// the parent.
    ///
    /// Three constructions, and the comment at the call site says which clause each answers:
    /// §11.6.6's own blending colour space, §11.4.6's non-isolated knockout group, and
    /// otherwise one pass onto [`initial_backdrop`]'s backdrop, followed for a non-isolated
    /// group under a mode of its own by §11.4.4's result step
    /// ([`CpuRasterizer::remove_the_backdrop`]).
    ///
    /// # Errors
    ///
    /// As [`CpuRasterizer::encode`], plus [`CpuRasterError::Allocation`] and the refusals
    /// [`initial_backdrop`], [`CpuRasterizer::composite_in_own_space`] and
    /// [`CpuRasterizer::knockout_on_backdrop`] state.
    fn group_buffer(
        &self,
        pixmap: &tiny_skia::PixmapMut<'_>,
        list: &DisplayList,
        group: &Group<'_>,
        (surface, band): (Surface, Band),
        masks: &mut MaskCache,
        depth: usize,
    ) -> Result<tiny_skia::Pixmap, CpuRasterError> {
        if let Some(blending) = group.blending {
            return self.composite_in_own_space(list, blending, group, surface, masks, depth);
        }
        if !group.isolated && group.compose == Compose::Knockout {
            return self.knockout_on_backdrop(pixmap, list, group, surface, band, masks, depth);
        }
        let mut buffer = initial_backdrop(pixmap, surface, band, group)?;
        self.encode(
            &mut buffer.as_mut(),
            list,
            group.commands,
            surface,
            masks,
            depth,
            group.compose,
        )?;
        if !group.isolated && group.blend != pdf_render::BlendMode::Normal {
            self.remove_the_backdrop(
                (&mut buffer, pixmap.as_ref()),
                list,
                group,
                (surface, band),
                masks,
                depth,
            )?;
        }
        Ok(buffer)
    }

    /// ISO 32000-2 §11.4.4's result step, for the non-isolated group whose `Do` composites
    /// under anything but §11.3.3's Normal blend function.
    ///
    /// Under Normal the removal cancels against the re-compositing and the two steps collapse
    /// to [`blend::interpolate`]; under any other mode they do not, and the group's own colour
    /// and Table 140's group alpha are both needed. §11.4.4's NOTE 4 says where the second
    /// comes from:
    ///
    /// > For shape and alpha, backdrop removal can be accomplished by maintaining two sets of
    /// > variables to hold the accumulated values.
    ///
    /// This is that second set — the same elements run again onto transparency, whose
    /// accumulated alpha *is* `αgn`, because §11.4.8's recurrence for shape and alpha reads no
    /// colour and so answers the same whatever backdrop the first run was given.
    /// [`blend::remove_backdrop`] then rewrites `buffer` into Table 139's `C` and `α`, which
    /// the caller composites once as one object.
    ///
    /// **It costs a second run of the elements**, and it is paid only here: `pdf-model` emits
    /// this combination for a group that is non-isolated, non-knockout, outside every knockout
    /// group, holds an element that blends, and is composited under a mode of its own.
    ///
    /// **Priced on the first corpus page that reaches it** —
    /// `doc/pdf.js/test/pdfs/issue12798_page1_reduced.pdf`, whose two groups are both
    /// §11.7.4's implicit construction under `/Multiply` (ADR 1206). `callgrind_rasterise`,
    /// 20 draws, one sitting, two arms from one tree with this call planted away in the second:
    /// 6 225 323 395 → 6 668 515 460, **+443 192 065 instructions, +7.12%**, or 22.2 M a draw.
    /// The elements re-run are the group's own, so the figure is the page's marks rather than a
    /// constant; what bounds it is that no other page of the curated corpora reaches this branch
    /// at all, and that a page stating no group under a mode of its own pays the one comparison
    /// at the call site.
    ///
    /// # Errors
    ///
    /// As [`CpuRasterizer::encode`], plus [`CpuRasterError::Allocation`] for the second buffer.
    fn remove_the_backdrop(
        &self,
        (buffer, backdrop): (&mut tiny_skia::Pixmap, tiny_skia::PixmapRef<'_>),
        list: &DisplayList,
        group: &Group<'_>,
        (surface, band): (Surface, Band),
        masks: &mut MaskCache,
        depth: usize,
    ) -> Result<(), CpuRasterError> {
        let mut own = tiny_skia::Pixmap::new(surface.width(), surface.rows.height).ok_or(
            CpuRasterError::Allocation {
                width: surface.width(),
                height: surface.rows.height,
            },
        )?;
        self.encode(
            &mut own.as_mut(),
            list,
            group.commands,
            surface,
            masks,
            depth,
            group.compose,
        )?;
        blend::remove_backdrop(buffer, backdrop, &own, band_pixels(surface, band));
        Ok(())
    }

    /// Composites a group's elements in the blending colour space the group states, and
    /// converts the result out (ISO 32000-2 §11.6.6, §11.7.2).
    ///
    /// §11.7.2: "all blending and compositing computations shall be done in that space",
    /// and "[t]he resulting colours shall then be interpreted in the group's colour space
    /// when the group is subsequently composited with its backdrop". For four components the
    /// first sentence is two `encode` passes — §11.3.4 composites per component, so four
    /// components are two passes of three, the same construction `rasterize` applies to
    /// §11.4.7's page — and the second is `pdf_render::blending::resolve`; for one component
    /// through a curve it is one pass with every colour painted as its component, and
    /// `pdf_render::blending::resolve_grey`; for three CIE-based components it is one pass
    /// with every colour painted as its components, and `pdf_render::blending::resolve_cube`.
    /// Each runs before the caller paints the buffer onto the parent. The group is isolated
    /// by `pdf_render::GroupBlending`'s own guarantee, so every pass starts on transparency.
    ///
    /// # Errors
    ///
    /// As [`CpuRasterizer::encode`], plus [`CpuRasterError::UnsupportedCommand`] for a
    /// non-isolated group carrying a space of its own, which `pdf-model` never builds.
    fn composite_in_own_space(
        &self,
        list: &DisplayList,
        blending: &pdf_render::GroupBlending,
        group: &Group<'_>,
        surface: Surface,
        masks: &mut MaskCache,
        depth: usize,
    ) -> Result<tiny_skia::Pixmap, CpuRasterError> {
        if !group.isolated {
            return Err(CpuRasterError::UnsupportedCommand(
                "a non-isolated group cannot composite in a blending colour space of its \
                 own: §11.6.6 inherits its space from the parent (ISO 32000-2 §11.6.6)"
                    .to_owned(),
            ));
        }
        let allocation = || CpuRasterError::Allocation {
            width: surface.width(),
            height: surface.rows.height,
        };
        let mut composited =
            tiny_skia::Pixmap::new(surface.width(), surface.rows.height).ok_or_else(allocation)?;
        self.encode(
            &mut composited.as_mut(),
            list,
            group.commands,
            surface,
            masks,
            depth,
            group.compose,
        )?;
        match blending {
            pdf_render::GroupBlending::FourComponents { space, black } => {
                let mut ink = tiny_skia::Pixmap::new(surface.width(), surface.rows.height)
                    .ok_or_else(allocation)?;
                self.encode(
                    &mut ink.as_mut(),
                    list,
                    black,
                    surface,
                    masks,
                    depth,
                    group.compose,
                )?;
                pdf_render::resolve_blending(composited.data_mut(), ink.data(), space);
            }
            pdf_render::GroupBlending::OneComponent { curve } => {
                pdf_render::resolve_grey(composited.data_mut(), curve);
            }
            pdf_render::GroupBlending::ThreeComponents { cube } => {
                pdf_render::resolve_cube(composited.data_mut(), cube);
            }
        }
        Ok(composited)
    }

    /// Accumulates §11.4.6's two stages for a non-isolated knockout group, whose initial
    /// backdrop is the group's own.
    ///
    /// > In a knockout group, each individual element shall be composited with the group's
    /// > initial backdrop rather than with the stack of preceding elements in the group.
    ///
    /// The clause's two stages, per element: a) "[c]omposite the source object with the
    /// group's initial backdrop, disregarding the object's shape and using a source shape
    /// value of 1.0 everywhere", then b) "[c]ompute a weighted average of this result with
    /// the object's immediate backdrop, using the source shape as the weighting factor".
    /// Stage a) is the element drawn onto a scratch copy of the backdrop `B` — its blend
    /// mode, opacity and soft mask all applied, against `B` — and stage b) is
    /// [`blend::knockout_average`], with the element's shape read per pixel off its
    /// [`Command::Shaped`] shape half, which `pdf-model` guarantees every element carries.
    ///
    /// Drawing the object bakes its shape into the scratch where stage a) asks for a shape
    /// of 1.0 everywhere, and the identity `f × E = scratch − (1 − f) × B` (see
    /// [`blend::knockout_average`]) is what takes it back out, so the accumulation is the
    /// clause's arithmetic and not an approximation of it. The buffer starts as `B` — the
    /// accumulation's own initial value — and the caller's non-isolated completion then
    /// interpolates it with the page by the group's alpha, which is the collapse of
    /// §11.4.4's backdrop removal against §11.3.3's recompositing under the Normal blend
    /// function (ADR 0307's formula; ADR 0237's cancellation, unchanged by knockout).
    ///
    /// # Under a blend mode of the group's own
    ///
    /// Where the `Do` composites under another mode that collapse is not available, and
    /// §11.4.4 says what the group's result is composited with and under which function:
    ///
    /// > In those formulas, the colour, shape, and alpha ( C, 𝑓 , and α ) calculated by the
    /// > group compositing function shall be used, respectively, as the source colour 𝐶𝑠 ,
    /// > the object shape 𝑓 j , and the object alpha α j .
    ///
    /// So the accumulation is taken through §11.4.4's result step before it leaves — the
    /// step §11.4.8 states once for every kind of group — and the caller composites the
    /// result as one object under the group's own mode, exactly as it does for a
    /// non-knockout group ([`CpuRasterizer::remove_the_backdrop`]). The step needs Table
    /// 140's group alpha beside the accumulation, and §11.4.8 gives it for a knockout group
    /// as `αgᵢ = (1 − fsᵢ) × αgᵢ₋₁ + αsᵢ` — the same stage b) with a transparent initial
    /// backdrop, since the group alpha excludes it. That is a second accumulation of the
    /// same elements onto transparency, weighted by the same stated shapes, and it is paid
    /// only here (ADR 1305).
    ///
    /// # Errors
    ///
    /// As [`CpuRasterizer::encode`], plus [`CpuRasterError::UnsupportedCommand`] where a
    /// guarantee `pdf-model` states for the combination does not hold: every element a
    /// [`Command::Shaped`], the group composited by Over rather than as a stage of an
    /// enclosing knockout group.
    #[expect(
        clippy::too_many_arguments,
        reason = "one arm of `draw_group`'s buffer construction, carrying what the \
                  recursion carries plus the band the backdrop is copied to"
    )]
    fn knockout_on_backdrop(
        &self,
        pixmap: &tiny_skia::PixmapMut<'_>,
        list: &DisplayList,
        group: &Group<'_>,
        surface: Surface,
        band: Band,
        masks: &mut MaskCache,
        depth: usize,
    ) -> Result<tiny_skia::Pixmap, CpuRasterError> {
        if group.into != Compose::Over {
            return Err(CpuRasterError::UnsupportedCommand(
                "a non-isolated knockout group drawn as a stage of an enclosing knockout \
                 group has no page for its initial backdrop to be copied from \
                 (ISO 32000-2 §11.4.6 NOTE 6)"
                    .to_owned(),
            ));
        }
        let allocation = || CpuRasterError::Allocation {
            width: surface.width(),
            height: surface.rows.height,
        };
        // The group's initial backdrop, retained beside the accumulation: every element
        // composites against *it*, not against what earlier elements left. Copied to the
        // group's band by ADR 0328's own argument, which carries over unchanged: the only
        // reader of this construction's result is the caller's band-rows interpolation, the
        // per-element scratches below are clones of this buffer and so hold the same rows,
        // and no operation moves a value between rows — so a row outside the band, which
        // here is transparency rather than the page, can never reach the page.
        let mut backdrop =
            tiny_skia::Pixmap::new(surface.width(), surface.rows.height).ok_or_else(allocation)?;
        let stride = (surface.width() as usize).saturating_mul(4);
        let start = (band.top.saturating_sub(surface.rows.top) as usize).saturating_mul(stride);
        let end = start.saturating_add((band.height as usize).saturating_mul(stride));
        let source = pixmap
            .as_ref()
            .data()
            .get(start..end)
            .ok_or_else(allocation)?;
        backdrop
            .data_mut()
            .get_mut(start..end)
            .ok_or_else(allocation)?
            .copy_from_slice(source);
        let mut accumulated = backdrop.clone();
        // Table 140's group alpha, kept only where §11.4.4's result step will read it: the
        // Normal composite at the `Do` is the collapse above and needs no second set.
        let transparent =
            tiny_skia::Pixmap::new(surface.width(), surface.rows.height).ok_or_else(allocation)?;
        let mut group_alpha =
            (group.blend != pdf_render::BlendMode::Normal).then(|| transparent.clone());
        for element in group.commands {
            let Command::Shaped { object, shape } = element else {
                return Err(CpuRasterError::UnsupportedCommand(
                    "an element of a non-isolated knockout group must state its shape \
                     apart from its alpha (ISO 32000-2 §11.4.6)"
                        .to_owned(),
                ));
            };
            // Stage a): the element against the initial backdrop, in a scratch of its own.
            let mut composed = backdrop.clone();
            self.encode(
                &mut composed.as_mut(),
                list,
                std::slice::from_ref(object),
                surface,
                masks,
                depth,
                Compose::Over,
            )?;
            // §11.6.4.2's shape, drawn onto transparency so its alpha *is* the shape.
            let mut stated = tiny_skia::Pixmap::new(surface.width(), surface.rows.height)
                .ok_or_else(allocation)?;
            self.encode(
                &mut stated.as_mut(),
                list,
                std::slice::from_ref(shape),
                surface,
                masks,
                depth,
                Compose::Over,
            )?;
            blend::knockout_average(&mut accumulated, &backdrop, &composed, &stated);
            // The same stage b) with a transparent initial backdrop, whose alpha is `αgᵢ`.
            if let Some(group_alpha) = group_alpha.as_mut() {
                let mut alone = transparent.clone();
                self.encode(
                    &mut alone.as_mut(),
                    list,
                    std::slice::from_ref(object),
                    surface,
                    masks,
                    depth,
                    Compose::Over,
                )?;
                blend::knockout_average(group_alpha, &transparent, &alone, &stated);
            }
        }
        if let Some(group_alpha) = group_alpha {
            blend::remove_backdrop(
                &mut accumulated,
                pixmap.as_ref(),
                &group_alpha,
                band_pixels(surface, band),
            );
        }
        Ok(accumulated)
    }
}

/// One transparency group, unpacked from its command.
///
/// Grouped for the same reason as [`ImagePlacement`]: five values that always travel
/// together, and a call site that would otherwise be a row of unlabelled arguments.
///
/// [`ImagePlacement`]: crate::image::ImagePlacement
#[derive(Debug, Clone, Copy)]
pub(crate) struct Group<'a> {
    pub(crate) commands: &'a [Command],
    pub(crate) alpha: f32,
    pub(crate) blend: pdf_render::BlendMode,
    pub(crate) clip: Option<ClipId>,
    pub(crate) mask: Option<SoftMaskId>,
    /// What the elements are composited onto — see `Command::Group`'s `isolated`.
    pub(crate) isolated: bool,
    /// Whether the raster the elements accumulate carries Table 139's shape as well as its
    /// alpha — see `Command::Group`'s `alpha_is_shape`, which is where the argument is.
    pub(crate) alpha_is_shape: bool,
    /// How this group's own elements combine with each other (§11.4.6).
    pub(crate) compose: Compose,
    /// How the finished group combines with what it is drawn onto.
    ///
    /// [`Compose::Over`] everywhere except the two halves of a [`Command::Shaped`], where a
    /// group is one element of a knockout group and §11.4.6's two stages are two draws.
    pub(crate) into: Compose,
    /// The blending colour space of its own the elements composite in, as the conversion out
    /// of it — see `pdf_render::GroupBlending`.
    pub(crate) blending: Option<&'a pdf_render::GroupBlending>,
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
/// # Only `band`'s rows are copied, and that is exact rather than approximate
///
/// The buffer still covers the whole surface, so the elements draw under the very transforms
/// they draw under on the page — a buffer starting at another row would shift `tiny-skia`'s
/// `y·sy + ty` into another binade, which is the departure ADR 0219 measured and this backend,
/// being the oracle, does not take. But only the rows of the group's band — the rows its clip
/// and mask admit — are copied into it, because no other row of the buffer can reach the page:
/// every way [`CpuRasterizer::draw_group`] brings the buffer back reads exactly the band's
/// rows, and every operation an element performs on a buffer pixel writes that pixel and no
/// other, so a value outside the band cannot travel into it. A nested group repeats the same
/// argument one level down, over its own band.
///
/// **The benchmark that justifies the crop**, per `CLAUDE.md`'s rule: `0423548.pdf`, the
/// second-slowest document of a 65 944-document crawl of the web, states 132 non-isolated
/// groups on one 1843 × 5103 page and paid **4.3 GB** of whole-surface copies for **82 MB**
/// of band — 2.85 s of the 6.6 that remained after ADR 0271. With this crop, and the
/// soft-mask banding that shipped beside it, `open_one` on that document — open, interpret,
/// rasterise — went from **149.48 G instructions to 76.48 G**, an A/B of two binaries built
/// in one sitting (ADR 0328). What it costs in readability is this paragraph and one block
/// of slice arithmetic below.
///
/// The mask a group's blit still owes, after §8.5.4's intersection has been taken over
/// `buffer`'s band where it could be.
///
/// `None` says the intersection is in the buffer already and the blit carries no mask; the clip's
/// own mask says it was not taken and the blit owes it, which is what this backend did for every
/// group before ADR 0492.
///
/// The arithmetic and its exactness are [`scan::intersect_group`]'s; this is the two conditions
/// that decide whether the buffer is one it may be asked of.
///
/// - **`alpha_is_shape`**, which is `pdf_render::Command::Group`'s own field and the whole
///   argument: Table 139 returns a group's shape beside its alpha and this buffer holds one
///   number per pixel, so the composition is expressible only for a group whose opacity is 1.0
///   everywhere. `pdf-model` answers it, because the display list holds a translucent colour
///   and not the reason it is translucent.
/// - **Isolated only.** A non-isolated group's buffer starts as a copy of the page
///   ([`initial_backdrop`]), so the alpha in it is the backdrop's unioned with the group's and
///   is not that shape at all — the sentence `pdf-model`'s `element_alpha_is_shape` writes one
///   layer up, and the reason the caller's interpolation is left alone.
///
/// A group composited in a blending colour space of its own is *not* excluded: `resolve_blending`
/// rewrites three channels of every pixel and leaves the fourth, so the alpha that reaches here
/// is the one the elements accumulated.
fn group_blit_mask<'a>(
    buffer: &mut tiny_skia::Pixmap,
    group: &Group<'_>,
    (surface, band, top): (Surface, Band, i32),
    clip: scan::Clip<'a>,
) -> Option<&'a tiny_skia::Mask> {
    if !group.isolated || !group.alpha_is_shape {
        return clip.mask();
    }
    let stride = (surface.width() as usize).saturating_mul(4);
    let start = (top.unsigned_abs() as usize).saturating_mul(stride);
    let end = start.saturating_add((band.height as usize).saturating_mul(stride));
    let composed = buffer
        .data_mut()
        .get_mut(start..end)
        .is_some_and(|rows| scan::intersect_group(rows, clip));
    if composed { None } else { clip.mask() }
}

/// # Errors
///
/// [`CpuRasterError::Allocation`] if the buffer does not fit or `band` lies outside it, and
/// [`CpuRasterError::UnsupportedCommand`] for a non-isolated group composited by §11.4.6's
/// staged pair, whose transparent start is not this group's backdrop.
fn initial_backdrop(
    pixmap: &tiny_skia::PixmapMut<'_>,
    surface: Surface,
    band: Band,
    group: &Group<'_>,
) -> Result<tiny_skia::Pixmap, CpuRasterError> {
    if !group.isolated && (group.compose != Compose::Over || group.into != Compose::Over) {
        return Err(CpuRasterError::UnsupportedCommand(
            "a non-isolated group composited by §11.4.6's staged pair: its elements start on \
             the group's own backdrop, which the pair's transparent start is not \
             (ISO 32000-2 §11.4.4, §11.4.6)"
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
    // A byte copy rather than a `Source` draw: both move the band's bytes unchanged, and the
    // slice arithmetic is `Band::rows`'s, over the same rows of both buffers.
    let stride = (surface.width() as usize).saturating_mul(4);
    let start = (band.top.saturating_sub(surface.rows.top) as usize).saturating_mul(stride);
    let end = start.saturating_add((band.height as usize).saturating_mul(stride));
    let source = pixmap
        .as_ref()
        .data()
        .get(start..end)
        .ok_or(CpuRasterError::Allocation {
            width: surface.width(),
            height: band.height,
        })?;
    buffer
        .data_mut()
        .get_mut(start..end)
        .ok_or(CpuRasterError::Allocation {
            width: surface.width(),
            height: band.height,
        })?
        .copy_from_slice(source);
    Ok(buffer)
}

#[cfg(test)]
mod tests {

    use super::{Group, initial_backdrop};

    use crate::compose::Compose;
    use crate::surface::{Band, Surface};

    use pdf_render::{DisplayList, Size, TargetSpec};

    /// The backdrop buffer holds the band's rows of the page and transparency elsewhere.
    ///
    /// [`initial_backdrop`]'s crop rests on one claim — no buffer row outside the group's
    /// band can reach the page — and this pins what the crop must still guarantee: inside
    /// the band the buffer *is* the page, byte for byte, because [`blend::interpolate`]
    /// reads those rows as the group's backdrop and any difference there would be a
    /// different picture, not a faster one.
    #[test]
    fn the_backdrop_copy_is_the_bands_rows() {
        let list = DisplayList::new(Size::new(8.0, 8.0));
        let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("valid target");
        let surface = Surface::whole(target);

        let mut page = tiny_skia::Pixmap::new(target.width, target.height).expect("a pixmap");
        for (index, pixel) in page.pixels_mut().iter_mut().enumerate() {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "an 8x8 fixture has 64 pixels, all of which fit u8"
            )]
            let level = index as u8;
            *pixel = tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, level)
                .expect("black under any alpha is premultiplied-valid");
        }

        let band = Band { top: 2, height: 3 };
        let group = Group {
            commands: &[],
            alpha: 1.0,
            blend: pdf_render::BlendMode::Normal,
            clip: None,
            mask: None,
            isolated: false,
            alpha_is_shape: false,
            compose: Compose::Over,
            into: Compose::Over,
            blending: None,
        };
        let buffer = initial_backdrop(&page.as_mut(), surface, band, &group)
            .expect("a small buffer allocates");

        let stride = target.width as usize * 4;
        let (start, end) = (band.top as usize * stride, 5 * stride);
        assert_eq!(
            &buffer.data()[start..end],
            &page.data()[start..end],
            "inside the band the buffer must be the page"
        );
        assert!(
            buffer.data()[..start].iter().all(|&byte| byte == 0)
                && buffer.data()[end..].iter().all(|&byte| byte == 0),
            "outside the band the buffer holds rows nothing can read, and copying them \
             was 2.85 s of a 6.6 s page (ADRs 0271, 0328)"
        );
    }
}
