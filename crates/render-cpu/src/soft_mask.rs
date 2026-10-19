//! Soft masks — ISO 32000-2 §11.6.5: a mask's group drawn and its values derived, over only the
//! rows its consumers can mark.
//!
//! Outside the marks of its group a soft mask is one constant (§11.6.5.1), so a mask is evaluated
//! over a band and the constant stands for every other row; the reach of its consumers decides
//! the band.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use pdf_render::{ClipId, Command, DisplayList, MAX_GROUP_DEPTH, Selection, SoftMaskId, Transform};

use crate::compose::Compose;
use crate::mask_cache::MaskCache;
use crate::surface::{Band, Surface};
use crate::{CpuRasterError, CpuRasterizer, convert};

impl CpuRasterizer {
    /// Evaluates a soft mask into the cache, if it is not there already (§11.5).
    ///
    /// The mask's group is drawn onto a fully transparent buffer — the same isolated backdrop
    /// [`CpuRasterizer::draw_group`] uses, and what both §11.5.2 and §11.5.3 ask for —
    /// covering the rows of this surface the mask's consumers can read
    /// ([`mask_consumer_reach`]; every row of it, wherever some consumer is unclipped) — and
    /// each pixel is then turned into a mask value by [`pdf_render::SoftMask::value`], which
    /// is the function the GPU backend calls on its own readback. That shared derivation is
    /// the whole reason the display list carries a mask's *commands* rather than a raster:
    /// the group is evaluated at device resolution, which only a backend knows, while what
    /// the pixels mean is decided once for both.
    ///
    /// # Errors
    ///
    /// As [`CpuRasterizer::encode`], plus [`CpuRasterError::UnknownSoftMask`] for an
    /// identifier this display list does not hold.
    pub(crate) fn build_soft_mask(
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

        // §11.6.5.1 gives a mask a value everywhere, but a value is only *read* where some
        // consumer reads it — and every consumer reads through [`MaskCache::effective`], over
        // its own clip's band. So the group is evaluated over the rows
        // [`mask_consumer_reach`] says its consumers can reach, not over the whole surface,
        // through the same [`Surface`] machinery a strip uses: the band's first row composed
        // into the translation once and last (ADR 0219). The rows it drops are rows whose
        // values no read can observe; the rows it keeps carry ADR 0219's residue — a command
        // whose clip band clamps to a different first row rounds `y·sy + ty` in another
        // binade — measured across the 974-document corpus at one level on one pixel of one
        // page, and priced in ADR 0783. A mask with an unclipped consumer — including
        // everything [`MaskCache::expand_soft_mask`] serves — reaches this with no
        // restriction and takes the path below unchanged, byte for byte.
        //
        // The benchmark that justifies it, per `CLAUDE.md`'s rule:
        // `MOZILLA-831621-14.pdf` states 3059 soft masks, each one page-sized shading fill
        // read through a clip admitting ~0.1% of the 1280×800 page, and rasterised in 41.5 s
        // — ~3.1 G shaded pixels and as many luminosity derivations for masks whose read
        // rows total ~2% of that (`pdf-model/examples/clip_chain_census`, soft-mask lines).
        // With the restriction the page draws in 1.5 s, byte-identically. ADR 0783.
        let marked = marked_rows(&mask.commands, surface);
        let restricted = masks.soft_mask_rows(id, marked);
        let narrowed = restricted != marked;
        // The drawing surface: the page's own target with the restricted run of rows, so the
        // group's elements draw under the very transforms they draw under on the page.
        let draw = if narrowed {
            Surface {
                page: surface.page,
                rows: restricted,
            }
        } else {
            surface
        };
        // A soft mask's group is evaluated as §11.4.5's ordinary group: `SoftMask` carries
        // no knockout flag, and `pdf-model` reports a mask group that asks for one.
        //
        // A closure because the group is drawn **twice** where its blending colour space has
        // four components (`pdf_render::BlackHalf`): §11.3.4 composites per component and
        // this raster has three channels, so the second list carries the black component and
        // §11.5.3's `Y` is read off the pair. One closure rather than two call sites so that
        // the two halves cannot be drawn onto differently-shaped buffers.
        let paint = |commands: &[Command],
                     masks: &mut MaskCache|
         -> Result<tiny_skia::Pixmap, CpuRasterError> {
            let mut buffer = tiny_skia::Pixmap::new(draw.width(), draw.rows.height).ok_or(
                CpuRasterError::Allocation {
                    width: draw.width(),
                    height: draw.rows.height,
                },
            )?;
            if narrowed {
                // A cache of its own, because a clip mask is banded against the surface it
                // was built for and this one is narrower than `masks.surface`: an entry built
                // here would be wrong for the outer encode and one built there wrong for
                // this. What sharing would have reused is bounded by the very rows this
                // evaluation was narrowed to, so the rebuild costs a fraction of what the
                // narrowing saves.
                let mut own = MaskCache::new(draw, self.anti_alias, masks.budget)
                    .with_reach(Arc::clone(&masks.reach));
                self.encode(
                    &mut buffer.as_mut(),
                    list,
                    commands,
                    draw,
                    &mut own,
                    depth,
                    Compose::Over,
                )?;
            } else {
                self.encode(
                    &mut buffer.as_mut(),
                    list,
                    commands,
                    surface,
                    masks,
                    depth,
                    Compose::Over,
                )?;
            }
            Ok(buffer)
        };
        let buffer = paint(&mask.commands, masks)?;
        let black = match mask.black.as_ref() {
            Some(half) => Some(paint(&half.commands, masks)?),
            None => None,
        };

        // Straight alpha, which is what `SoftMask::value` is defined over and what the GPU
        // backend reads back; `tiny-skia` stores premultiplied, so the conversion happens
        // here at the same boundary as every other one in this backend.
        //
        // **Per pixel rather than through `Pixmap::take_demultiplied`, and only for a pixel
        // the group marked.** The buffer covers the whole target while a mask group covers
        // its own `/BBox`, so most of it is the transparency it was allocated as — and both
        // halves of the old line ran over all of it: `take_demultiplied` divides three
        // channels by the alpha, and `SoftMask::values` derives a luminosity in floating
        // point. For the transparent pixel both answers are constants: the division gives
        // back the zero it started from, and the derivation gives [`SoftMask::outside`].
        //
        // This is exact, not an approximation — the branch's two arms call the same function
        // on the same pixel — and it is worth stating what it buys, because `CLAUDE.md`
        // requires an optimisation to carry its number: the two slowest documents of a
        // 65 944-document sample of the web spend 22.4 s of 25.4 and 51.0 s of 52.6 on this
        // one line, over rasters that are 98.5% and 99.96% transparent. ADR 0271.
        //
        // **And the conversion runs over the rows the group's marks could reach, not the
        // surface** (`doc/todo/40`, ADR 0328): a row outside `marked_rows`' answer is a row
        // of the buffer nobody wrote, its pixels are the transparency the buffer was
        // allocated as, and the value the pass would derive from every one of them is the
        // `outside` constant the entry now carries instead. The drawing buffer itself
        // deliberately stays surface-sized — the group's elements draw under the very
        // transforms they draw under on the page, which is what keeps this byte-exact where
        // a banded *drawing* would move an edge by a supersample (ADR 0219). Its untouched
        // rows are never read, and an untouched page of a zeroed allocation costs no work.
        // `6081357.pdf` — 912 distinct masks on a 4.3-megapixel page, 99.96% of every raster
        // transparent — went from 81.90 G instructions through `open_one` to **17.02 G**
        // when the pass and the storage stopped being surface-sized (ADR 0328's A/B, two
        // binaries built in one sitting).
        let outside = mask.outside();
        let reach = restricted;
        let built = mask_values(
            mask,
            (&buffer, black.as_ref()),
            (draw, reach),
            surface.width(),
        )?;
        masks.admit_soft_mask(id, built, reach, outside);
        Ok(())
    }
}

/// The rows of `surface` a command list's own marks can reach — a bound, never a coverage.
///
/// The extents are the ones [`misses_surface`] culls by — [`Command::device_bounds`] per
/// leaf, a group answering through its elements — measured on the page's grid and given
/// [`Band::covering`]'s row of margin, so a row outside the answer is a row no command in
/// the list can mark. A leaf whose extent cannot be measured widens the answer to the whole
/// surface, which is the safe direction and the same reading `misses_surface` takes.
///
/// Asked by [`CpuRasterizer::build_soft_mask`], which needs a superset of the rows its
/// buffer was *written* in: outside them the buffer is still the transparency it was
/// allocated as. A list that can mark no row at all answers one row rather than none — that
/// row's pixels are untouched, so every value derived from them is the same
/// [`pdf_render::SoftMask::outside`] constant any other unmarked pixel yields, and a
/// one-row mask keeps [`Built`](crate::mask_cache::Built) free of an empty case no other entry has.
///
/// [`misses_surface`]: crate::surface::misses_surface
fn marked_rows(commands: &[Command], surface: Surface) -> Band {
    let mut extent = None;
    if !vertical_extent(commands, surface.page.transform, &mut extent, 0) {
        return surface.rows;
    }
    let one_row = Band {
        top: surface.rows.top,
        height: 1,
    };
    let Some((low, high)) = extent else {
        return one_row;
    };
    tiny_skia::Rect::from_ltrb(0.0, low, 1.0, high)
        .and_then(|bounds| Band::covering(bounds, surface))
        // Off the surface entirely, so nothing was marked; the degenerate answer above.
        .unwrap_or(one_row)
}

/// Turns the mask group's rendered buffer — or §11.4.7's pair of them — into one mask value
/// per pixel of `reach`, as the `tiny_skia::Mask` a consumer reads through.
///
/// Straight alpha, which is what [`pdf_render::SoftMask::value`] is defined over and what the
/// GPU backend reads back; `tiny-skia` stores premultiplied, so the conversion happens here at
/// the same boundary as every other one in this backend.
///
/// **Per pixel rather than through `Pixmap::take_demultiplied`, and only for a pixel the group
/// marked.** The buffer covers the whole target while a mask group covers its own `/BBox`, so
/// most of it is the transparency it was allocated as — and both halves of the old line ran
/// over all of it: `take_demultiplied` divides three channels by the alpha, and
/// `SoftMask::values` derives a luminosity in floating point. For the transparent pixel both
/// answers are constants: the division gives back the zero it started from, and the derivation
/// gives [`pdf_render::SoftMask::outside`].
///
/// This is exact, not an approximation — the branch's two arms call the same function on the
/// same pixel — and it is worth stating what it buys, because `CLAUDE.md` requires an
/// optimisation to carry its number: the two slowest documents of a 65 944-document sample of
/// the web spend 22.4 s of 25.4 and 51.0 s of 52.6 on this one line, over rasters that are
/// 98.5% and 99.96% transparent. ADR 0271.
///
/// **And the conversion runs over the rows the group's marks could reach, not the surface**
/// (`doc/todo/40`, ADR 0328): a row outside `marked_rows`' answer is a row of the buffer nobody
/// wrote, its pixels are the transparency the buffer was allocated as, and the value the pass
/// would derive from every one of them is the `outside` constant the entry carries instead. The
/// drawing buffer itself deliberately stays surface-sized — the group's elements draw under the
/// very transforms they draw under on the page, which is what keeps this byte-exact where a
/// banded *drawing* would move an edge by a supersample (ADR 0219). Its untouched rows are
/// never read, and an untouched page of a zeroed allocation costs no work. `6081357.pdf` — 912
/// distinct masks on a 4.3-megapixel page, 99.96% of every raster transparent — went from
/// 81.90 G instructions through `open_one` to **17.02 G** when the pass and the storage stopped
/// being surface-sized (ADR 0328's A/B, two binaries built in one sitting).
///
/// # Errors
///
/// [`CpuRasterError::Allocation`] where the rows asked for are not in the buffer, or where the
/// values do not make a mask of that size.
fn mask_values(
    mask: &pdf_render::SoftMask,
    (buffer, black): (&tiny_skia::Pixmap, Option<&tiny_skia::Pixmap>),
    (draw, reach): (Surface, Band),
    width: u32,
) -> Result<tiny_skia::Mask, CpuRasterError> {
    let outside = mask.outside();
    let stride = draw.width() as usize;
    let start = (reach.top.saturating_sub(draw.rows.top) as usize).saturating_mul(stride);
    let end = start.saturating_add((reach.height as usize).saturating_mul(stride));
    let short = || CpuRasterError::Allocation {
        width,
        height: reach.height,
    };
    let read = buffer.pixels().get(start..end).ok_or_else(short)?;
    // §11.4.7's second raster, where the group composited in four components: the same rows of
    // the black half, read at the same points. `SoftMask::paired_value` is what puts the four
    // components back together and takes §11.5.3's `Y` of them, so the two backends cannot
    // disagree about it any more than they can about the one-raster case.
    //
    // Borrowed rather than copied, and the two arms are written out rather than folded into
    // one: a mask with no black half takes exactly the arithmetic it took before.
    let black = match black {
        Some(pixmap) => Some(pixmap.pixels().get(start..end).ok_or_else(short)?),
        None => None,
    };
    let transparent = |pixel: &tiny_skia::PremultipliedColorU8| {
        pixel.alpha() == 0 && pixel.red() == 0 && pixel.green() == 0 && pixel.blue() == 0
    };
    let straight = |pixel: &tiny_skia::PremultipliedColorU8| {
        let straight = pixel.demultiply();
        [
            straight.red(),
            straight.green(),
            straight.blue(),
            straight.alpha(),
        ]
    };
    let values: Vec<u8> = match black {
        None => read
            .iter()
            .map(|pixel| {
                if transparent(pixel) {
                    outside
                } else {
                    mask.value(straight(pixel))
                }
            })
            .collect(),
        Some(half) => read
            .iter()
            .zip(half)
            .map(|(pixel, other)| {
                if transparent(pixel) && transparent(other) {
                    outside
                } else {
                    mask.paired_value(straight(pixel), straight(other))
                }
            })
            .collect(),
    };
    tiny_skia::Mask::from_vec(
        values,
        tiny_skia::IntSize::from_wh(width, reach.height).ok_or_else(short)?,
    )
    .ok_or_else(short)
}

/// Accumulates the least and greatest device row the leaves of `commands` can mark.
///
/// `true` when every leaf answered; `false` the moment one cannot — an extent
/// [`Command::device_bounds`] does not state, one that overflowed past `f32`, or nesting
/// past [`MAX_GROUP_DEPTH`] — at which point the caller must read the answer as
/// "anywhere". The depth bound mirrors [`CpuRasterizer::encode`]'s rather than trusting
/// it: a group whose clip admits nothing is skipped there without its elements ever being
/// walked, so this walk can meet nesting the draw never did.
fn vertical_extent(
    commands: &[Command],
    to_device: Transform,
    extent: &mut Option<(f32, f32)>,
    depth: usize,
) -> bool {
    if depth > MAX_GROUP_DEPTH {
        return false;
    }
    for command in commands {
        // A group's own extent is its elements' — the documented `None` of
        // `device_bounds` — so the elements are what get asked.
        if let Command::Group { commands, .. } = command {
            if !vertical_extent(commands, to_device, extent, depth.saturating_add(1)) {
                return false;
            }
            continue;
        }
        let Some(bounds) = command.device_bounds(to_device) else {
            return false;
        };
        if !bounds.min.y.is_finite() || !bounds.max.y.is_finite() {
            return false;
        }
        *extent = Some(match *extent {
            None => (bounds.min.y, bounds.max.y),
            Some((low, high)) => (low.min(bounds.min.y), high.max(bounds.max.y)),
        });
    }
    true
}

/// The device rows a set of commands can *read* of a soft mask, before any is built.
///
/// A y-extent on the page's own device grid, so it is strip-independent; each strip clamps
/// it with [`Band::covering`]. `Everywhere` is the safe answer and the default.
#[derive(Debug, Clone, Copy)]
pub(crate) enum MaskReach {
    /// Some consumer can read every row — it has no clip, or a clip nobody could measure.
    Everywhere,
    /// Every consumer reads through a clip, and their chains' bounds reach only these rows.
    Rows { top: f32, bottom: f32 },
    /// Every consumer's clip admits nothing, so no row of the mask is ever read.
    Nothing,
}

impl MaskReach {
    /// The reach of a consumer whose clip chain bounds are `extent`, joined with `self`.
    fn union(self, other: Self) -> Self {
        match (self, other) {
            (Self::Everywhere, _) | (_, Self::Everywhere) => Self::Everywhere,
            (Self::Nothing, reach) | (reach, Self::Nothing) => reach,
            (Self::Rows { top, bottom }, Self::Rows { top: t, bottom: b }) => Self::Rows {
                top: top.min(t),
                bottom: bottom.max(b),
            },
        }
    }
}

/// The rows each soft mask's *consumers* can read, over the whole display list.
///
/// Every reader of a stored soft mask goes through [`MaskCache::effective`], and each of its
/// arms reads the mask over the band of the consuming command's clip — or over the whole
/// surface where the command has none (§11.6.5.1 gives a mask a value everywhere, so an
/// unclipped consumer reads all of it). So the union, per mask, of its consumers' clip-chain
/// bounds is a bound on the rows whose values can ever matter, and it is computable before
/// anything is built: the same per-clip path bounds [`MaskCache::build`] measures, intersected
/// along each chain, united across consumers. A clip that cannot be measured widens the answer
/// — the direction `build` errs in too — and a walk that cannot finish (nesting past
/// [`MAX_GROUP_DEPTH`], which `encode` refuses anyway) answers the empty map, which restricts
/// nothing.
///
/// Consumers inside *other masks'* groups are walked as well, with their own clip chains: the
/// rows such a consumer actually reads are those drawn during the enclosing mask's build,
/// which are a subset of its clip's band, so the answer stays a superset of every read.
///
/// Of a [`Selection`] — a window's commands — the consumers at the top level are the selection's,
/// and those inside every soft mask the list's, as a list of the kept commands alone would walk
/// them (ADR 1980).
pub(crate) fn mask_consumer_reach(
    drawn: Selection<'_>,
    page: Transform,
) -> HashMap<SoftMaskId, MaskReach> {
    let list = drawn.list();
    let mut reach = HashMap::new();
    let mut known = HashMap::new();
    let mut complete = note_mask_consumers(list, drawn.commands(), page, &mut known, &mut reach, 0);
    for number in 0..list.soft_mask_count() {
        let mask = u32::try_from(number)
            .ok()
            .and_then(|index| list.soft_mask(SoftMaskId::new(index)));
        let Some(mask) = mask else {
            complete = false;
            break;
        };
        complete =
            complete && note_mask_consumers(list, &mask.commands, page, &mut known, &mut reach, 0);
        if let Some(half) = mask.black.as_ref() {
            // §11.4.7's second raster is a list of its own, and a mask a consumer inside it
            // names is a *separate* registration — the second interpretation registered its
            // own copies — so a walk that stopped at the first list would restrict one of
            // those masks to rows no consumer it could see reads.
            complete = complete
                && note_mask_consumers(list, &half.commands, page, &mut known, &mut reach, 0);
        }
    }
    if complete {
        reach
    } else {
        // A walk that could not see every consumer must not restrict any mask.
        HashMap::new()
    }
}

/// The y-extent of the chain ending at `id`, from the same bounds [`MaskCache::build`]
/// measures, memoised per node. [`mask_consumer_reach`]'s half that reads geometry.
fn chain_reach(
    list: &DisplayList,
    id: ClipId,
    page: Transform,
    known: &mut HashMap<ClipId, MaskReach>,
) -> MaskReach {
    // The chain, leaf to root, stopping at the first memoised ancestor. Iterative and
    // bounded by a seen-set for `MaskCache::resolve_chain`'s reason: a cyclic chain is
    // reachable from a malformed document, and `build` will report it — here it only
    // needs a safe answer.
    let mut walk = Vec::new();
    let mut seen = HashSet::new();
    let mut current = Some(id);
    let mut reach = MaskReach::Everywhere;
    while let Some(this) = current {
        if let Some(&known) = known.get(&this) {
            reach = known;
            break;
        }
        if !seen.insert(this) {
            return MaskReach::Everywhere;
        }
        walk.push(this);
        current = match list.clip(this) {
            Some(clip) => clip.parent,
            // Dangling: `build` errors the page; any answer here is moot, so the safe one.
            None => return MaskReach::Everywhere,
        };
    }
    // Root-first, so each memoised entry is the extent of the chain ending there.
    for &this in walk.iter().rev() {
        let Some(clip) = list.clip(this) else {
            return MaskReach::Everywhere;
        };
        if clip.admits_nothing() {
            reach = MaskReach::Nothing;
        } else if let Some(own) = convert::path(&clip.path).and_then(|path| {
            path.bounds()
                .transform(convert::transform(clip.transform.then(page)))
        }) {
            reach = match reach {
                MaskReach::Nothing => MaskReach::Nothing,
                MaskReach::Everywhere => MaskReach::Rows {
                    top: own.top(),
                    bottom: own.bottom(),
                },
                MaskReach::Rows { top, bottom } => {
                    let (top, bottom) = (top.max(own.top()), bottom.min(own.bottom()));
                    // `<=`, not `<`: `tiny_skia::Rect::intersect` admits a touching,
                    // zero-area overlap, and `Band::covering`'s row of outset makes such
                    // a chain read real rows — so `build` will, and this may not say no.
                    if top <= bottom {
                        MaskReach::Rows { top, bottom }
                    } else {
                        MaskReach::Nothing
                    }
                }
            };
        }
        // A clip whose path cannot be converted or measured widens nothing and narrows
        // nothing, exactly as `MaskCache::build` treats an unmeasurable bound.
        known.insert(this, reach);
    }
    reach
}

/// Joins each mask-consuming command's clip reach into `reach`, recursively.
///
/// [`mask_consumer_reach`]'s walking half. `false` the moment the walk cannot see a subtree
/// — nesting past [`MAX_GROUP_DEPTH`], which [`CpuRasterizer::encode`] refuses anyway — at
/// which point the caller must restrict nothing, because an unseen consumer could read
/// anywhere.
fn note_mask_consumers<'c>(
    list: &DisplayList,
    commands: impl IntoIterator<Item = &'c Command>,
    page: Transform,
    known: &mut HashMap<ClipId, MaskReach>,
    reach: &mut HashMap<SoftMaskId, MaskReach>,
    depth: usize,
) -> bool {
    if depth > MAX_GROUP_DEPTH {
        return false;
    }
    for command in commands {
        if let Some(id) = command.mask() {
            let this = match command.clip() {
                None => MaskReach::Everywhere,
                Some(clip) => chain_reach(list, clip, page, known),
            };
            let joined = reach
                .get(&id)
                .copied()
                .unwrap_or(MaskReach::Nothing)
                .union(this);
            reach.insert(id, joined);
        }
        let walked = match command {
            Command::Group { commands, .. } => {
                note_mask_consumers(list, commands, page, known, reach, depth.saturating_add(1))
            }
            Command::Shaped { object, shape } => {
                note_mask_consumers(list, std::iter::once(&**object), page, known, reach, depth)
                    && note_mask_consumers(
                        list,
                        std::iter::once(&**shape),
                        page,
                        known,
                        reach,
                        depth,
                    )
            }
            _ => true,
        };
        if !walked {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {

    use super::{marked_rows, mask_consumer_reach};
    use crate::CpuRasterizer;
    use crate::compose::Compose;
    use crate::mask_cache::{MASK_BUDGET, MaskCache};
    use crate::surface::{Band, Surface};

    use pdf_render::{
        Clip, DisplayList, FillRule, Path, PathCommand, Point, Size, TargetSpec, Transform,
    };

    /// A soft mask evaluated over the rows its consumers can read draws the page the
    /// unrestricted evaluation draws — and the restriction must actually be in force, or the
    /// equality is vacuous (ADR 0783).
    ///
    /// The geometry is whole integers under an integer transform so that the band's row
    /// offset costs no rounding anywhere — the two arms are then byte-comparable rather than
    /// near — and the mask's group paints *different* grey bars above, inside and below the
    /// consumer's clip, so a restriction that read the wrong rows, or substituted
    /// [`pdf_render::SoftMask::outside`] where a consumer reads, changes the picture.
    /// Calibrated by planting `soft_mask_rows`'s `Rows` arm as `one_row`: the equality fails.
    #[test]
    fn a_mask_narrowed_to_its_consumers_rows_draws_the_unrestricted_page() {
        use pdf_render::{BlendMode, Color, Paint, SoftMask, SoftMaskKind};

        let rectangle = |ltrb: (f32, f32, f32, f32)| {
            let mut path = Path::new();
            path.push(PathCommand::MoveTo(Point::new(ltrb.0, ltrb.1)));
            path.push(PathCommand::LineTo(Point::new(ltrb.2, ltrb.1)));
            path.push(PathCommand::LineTo(Point::new(ltrb.2, ltrb.3)));
            path.push(PathCommand::LineTo(Point::new(ltrb.0, ltrb.3)));
            path.push(PathCommand::Close);
            path
        };
        let fill = |ltrb, paint, clip, mask| pdf_render::Command::Fill {
            path: std::sync::Arc::new(rectangle(ltrb)),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint,
            clip,
            mask,
            blend: BlendMode::Normal,
        };

        let mut list = DisplayList::new(Size::new(100.0, 100.0));
        // Three grey bars, so the luminosity varies with the row: rows a wrong restriction
        // would misread carry a different value from the rows the consumer reads.
        let grey = |level| Paint::Solid(Color::rgb(level, level, level));
        let mask = list
            .add_soft_mask(SoftMask {
                commands: vec![
                    fill((0.0, 0.0, 100.0, 34.0), grey(0.2), None, None),
                    fill((0.0, 34.0, 100.0, 66.0), grey(0.6), None, None),
                    fill((0.0, 66.0, 100.0, 100.0), grey(0.9), None, None),
                ],
                kind: SoftMaskKind::Luminosity {
                    backdrop: Color::rgb(0.0, 0.0, 0.0),
                },
                transfer: None,
                luminance: None,
                black: None,
            })
            .expect("under the mask limit");
        let clip = list
            .add_clip(Clip {
                path: rectangle((10.0, 40.0, 90.0, 60.0)),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                parent: None,
            })
            .expect("under the clip limit");
        list.push(fill(
            (0.0, 0.0, 100.0, 100.0),
            Paint::Solid(Color::rgb(0.8, 0.1, 0.1)),
            Some(clip),
            Some(mask),
        ));

        let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("valid target");
        let surface = Surface::whole(target);
        let reach = mask_consumer_reach(pdf_render::Selection::whole(&list), target.transform);

        // The restriction must narrow, or this test would stay green with the feature gone.
        let marked = marked_rows(
            &list.soft_mask(mask).expect("registered above").commands,
            surface,
        );
        let narrowing = MaskCache::new(surface, true, MASK_BUDGET)
            .with_reach(std::sync::Arc::new(reach.clone()));
        assert_ne!(
            narrowing.soft_mask_rows(mask, marked),
            marked,
            "the consumer's clip must narrow the evaluation for this page to discriminate"
        );

        let rasterizer = CpuRasterizer::new();
        let draw = |masks: &mut MaskCache| {
            let mut pixmap =
                tiny_skia::Pixmap::new(target.width, target.height).expect("a small target");
            rasterizer
                .encode(
                    &mut pixmap.as_mut(),
                    &list,
                    list.commands(),
                    surface,
                    masks,
                    0,
                    Compose::Over,
                )
                .expect("every command is supported");
            pixmap
        };
        let mut restricted =
            MaskCache::new(surface, true, MASK_BUDGET).with_reach(std::sync::Arc::new(reach));
        let mut unrestricted = MaskCache::new(surface, true, MASK_BUDGET);
        let narrowed = draw(&mut restricted);
        let whole = draw(&mut unrestricted);
        assert!(
            whole.data().iter().any(|&byte| byte != 0),
            "the page must draw something for the comparison to mean anything"
        );
        assert_eq!(
            narrowed.data(),
            whole.data(),
            "a narrowed evaluation changed a pixel its consumers can read"
        );
    }

    /// `marked_rows` bounds every leaf, recurses into groups, and widens where it cannot say.
    #[test]
    fn marked_rows_bounds_every_leaf_and_widens_where_it_cannot_say() {
        use pdf_render::{BlendMode, Paint};

        let list = DisplayList::new(Size::new(100.0, 100.0));
        let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("valid target");
        let surface = Surface::whole(target);

        let bar = |top: f32, bottom: f32| {
            let mut path = Path::new();
            path.push(PathCommand::MoveTo(Point::new(10.0, top)));
            path.push(PathCommand::LineTo(Point::new(90.0, top)));
            path.push(PathCommand::LineTo(Point::new(90.0, bottom)));
            path.push(PathCommand::LineTo(Point::new(10.0, bottom)));
            path.push(PathCommand::Close);
            pdf_render::Command::Fill {
                path: std::sync::Arc::new(path),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                paint: Paint::Solid(pdf_render::Color::BLACK),
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            }
        };

        // The page transform flips y, so page y 30..40 is device rows 60..70 of 100.
        let alone = marked_rows(&[bar(30.0, 40.0)], surface);
        assert!(
            alone.top <= 60 && alone.top >= 58 && alone.height >= 10 && alone.height <= 14,
            "one bar at device rows 60..70 answered {alone:?}"
        );

        // A group's elements carry the extents, so nesting must not widen the answer.
        let grouped = marked_rows(
            &[pdf_render::Command::Group {
                commands: vec![bar(30.0, 40.0)],
                alpha: 1.0,
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
                isolated: true,
                knockout: false,
                alpha_is_shape: true,
                blending: None,
            }],
            surface,
        );
        assert_eq!(alone, grouped, "a group widened its elements' own extent");

        // An empty list marks nothing: one degenerate row, not the surface.
        let nothing = marked_rows(&[], surface);
        assert_eq!(nothing, Band { top: 0, height: 1 });

        // A leaf whose extent cannot be measured — an empty path has no bounds — widens
        // the answer to the whole surface, which is `device_bounds`'s documented reading.
        let unbounded = marked_rows(
            &[pdf_render::Command::Fill {
                path: std::sync::Arc::new(Path::new()),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                paint: Paint::Solid(pdf_render::Color::BLACK),
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            }],
            surface,
        );
        assert_eq!(unbounded, surface.rows, "an unmeasurable leaf must widen");
    }

    /// `build_soft_mask`'s two arms give the same answer for every pixel a buffer can hold.
    ///
    /// The function reads a premultiplied buffer and takes [`pdf_render::SoftMask::outside`]
    /// for the wholly transparent pixel instead of demultiplying it and deriving a value. That
    /// is only sound if `tiny-skia`'s own `demultiply` sends `[0, 0, 0, 0]` to `[0, 0, 0, 0]`
    /// — it divides by the alpha, so the answer is a `NaN` that Rust's saturating cast lands on
    /// zero, which is a fact about the dependency and about the cast rather than about this
    /// tree. Asserted rather than reasoned about, over every alpha and both mask kinds, because
    /// what rests on it is 51 s of a 52.6 s page. ADR 0271.
    #[test]
    fn the_transparent_pixels_shortcut_is_the_derivation() {
        use pdf_render::{Color, SoftMask, SoftMaskKind};

        let transparent = tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, 0)
            .expect("all-zero is a valid premultiplied pixel");
        let straight = transparent.demultiply();
        assert_eq!(
            [
                straight.red(),
                straight.green(),
                straight.blue(),
                straight.alpha()
            ],
            [0, 0, 0, 0],
            "demultiplying transparency must give transparency, or the shortcut changes pixels"
        );

        for kind in [
            SoftMaskKind::Alpha,
            SoftMaskKind::Luminosity {
                backdrop: Color::rgb(0.0, 0.0, 0.0),
            },
            SoftMaskKind::Luminosity {
                backdrop: Color::rgb(1.0, 0.5, 0.0),
            },
        ] {
            let mask = SoftMask {
                commands: Vec::new(),
                kind,
                transfer: None,
                luminance: None,
                black: None,
            };
            assert_eq!(
                mask.outside(),
                mask.value([
                    straight.red(),
                    straight.green(),
                    straight.blue(),
                    straight.alpha()
                ])
            );
        }
    }

    /// Every premultiplied pixel a buffer can hold reaches the same value either way.
    ///
    /// The test above pins the transparent pixel, which is the one the shortcut *replaces*;
    /// this one walks the other arm over a spread of alphas and colours to say that nothing
    /// else moved when `Pixmap::take_demultiplied` stopped being the route. ADR 0271.
    #[test]
    fn the_per_pixel_route_agrees_with_the_whole_buffers() {
        use pdf_render::{Color, SoftMask, SoftMaskKind};

        let mut buffer = tiny_skia::Pixmap::new(16, 16).expect("a 16x16 pixmap");
        for (index, pixel) in buffer.pixels_mut().iter_mut().enumerate() {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "index is bounded by 16*16 = 256, so every cast below is exact"
            )]
            let alpha = index as u8;
            // Premultiplied, so no channel may exceed the alpha; `from_rgba` refuses one that
            // does, which is what keeps this fixture a buffer the backend could have produced.
            *pixel =
                tiny_skia::PremultipliedColorU8::from_rgba(alpha / 2, alpha / 3, alpha / 4, alpha)
                    .expect("no channel exceeds the alpha");
        }

        for kind in [
            SoftMaskKind::Alpha,
            SoftMaskKind::Luminosity {
                backdrop: Color::rgb(0.2, 0.7, 0.9),
            },
        ] {
            let mask = SoftMask {
                commands: Vec::new(),
                kind,
                transfer: None,
                luminance: None,
                black: None,
            };
            let outside = mask.outside();
            let per_pixel: Vec<u8> = buffer
                .pixels()
                .iter()
                .map(|pixel| {
                    if pixel.alpha() == 0
                        && pixel.red() == 0
                        && pixel.green() == 0
                        && pixel.blue() == 0
                    {
                        outside
                    } else {
                        let straight = pixel.demultiply();
                        mask.value([
                            straight.red(),
                            straight.green(),
                            straight.blue(),
                            straight.alpha(),
                        ])
                    }
                })
                .collect();
            let whole_buffer = mask.values(&buffer.clone().take_demultiplied());
            assert_eq!(
                per_pixel, whole_buffer,
                "the per-pixel conversion and `take_demultiplied` must agree pixel for pixel"
            );
        }
    }
}
