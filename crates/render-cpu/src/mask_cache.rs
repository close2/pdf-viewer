//! The masks a strip builds, kept under one budget: a clip chain's coverage, a soft mask's values,
//! and the rows each admits — ISO 32000-2 §8.5.4 and §11.6.5.
//!
//! A strip owns one [`MaskCache`]; a mask is built at most once per strip while it fits the budget,
//! and rebuilt byte for byte where it was evicted. The soft masks' evaluation is
//! `crate::soft_mask`'s.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use pdf_render::{ClipId, DisplayList, Path, SoftMaskId, Transform};

use crate::fill::rectangular_mark;
use crate::soft_mask::MaskReach;
use crate::surface::{Band, Surface, ToDevice};
use crate::{CpuRasterError, area, convert, scan};

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
pub(crate) fn crop_to_mask(
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

/// One clip of a chain, ready to be drawn into a mask.
struct Shape<'a> {
    path: tiny_skia::Path,
    /// The same shape before the rasteriser's library saw it, kept so that §10.7.4's
    /// rectangle question can be asked once the band transform is known.
    ///
    /// The clipping region "consists of the set of pixels that would be included by a fill
    /// operation", so a rectangular clip is scan-converted by the rule a rectangular fill is —
    /// and [`pdf_render::device_rectangle`] is the one place that rule decides what a rectangle
    /// is (ADR 0476). It cannot be asked in the loop that builds this, because the band the mask
    /// covers is not known until every clip in the chain has been measured.
    source: &'a Path,
    /// Whether `path` is §10.7.4's region for `source` rather than `source` itself, which is a
    /// *wider* shape — [`pdf_render::clip_region`]. False for every ordinary clip, which is all
    /// but a handful: the substitution costs an allocation and is made only where the path states
    /// a subpath with no extent. It is what [`rectangular_mark`] reads.
    substituted: bool,
    /// The second fill of §10.7.4's region, where that region is a union of two.
    ///
    /// The marks the clause leaves for the subpaths that enclose no area, scan-converted under
    /// the non-zero rule and composed into the same mask as `path`. `None` wherever one path
    /// under one rule states the whole region, which is every clip in the corpus.
    union_marks: Option<tiny_skia::Path>,
    /// The clip's own transform; the target and band transforms are applied later,
    /// because the band is not known until every clip in the chain has been measured.
    transform: Transform,
    /// The operator's own rule, kept because §10.7.4's region is scan-converted under a rule
    /// [`pdf_render::clip_region`] may change: marks that cross are separate shapes rather than
    /// one winding.
    rule: pdf_render::FillRule,
    fill_rule: tiny_skia::FillRule,
    /// This shape's transform composed with the band's, filled in once the band is known.
    at: Transform,
    /// What [`rectangular_mark`] says this shape is under `at`, computed once: the answer decides
    /// both whether the shape states anything at all and how it is scan-converted if it does.
    mark: scan::Exact,
}

/// A built clip mask, the band it covers, and the device rectangle it can mark within.
pub(crate) struct Built {
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
    /// The mask's value at every pixel outside `band`.
    ///
    /// Zero for a clip and for a clip × soft-mask product: ISO 32000-2 §8.5.4's clipping path
    /// admits nothing outside itself, so a band-sized mask and "nothing elsewhere" are the
    /// same statement. For a soft mask it is [`pdf_render::SoftMask::outside`] — §11.6.5.1's
    /// one value for the area the group's marks never reached — which need not be zero: a
    /// white `/BC` puts it at 255. Carrying the constant beside the band is what lets a soft
    /// mask be *stored* over the rows its group could mark instead of over the whole surface
    /// (`doc/todo/40`); every reader substitutes it for the rows the raster does not hold.
    outside: u8,
    /// For a clip × soft-mask product, the soft mask's own values over `band`; `None` for the
    /// other two kinds of entry, which are not products of anything.
    ///
    /// **Kept because a product is not a set and the two factors are not the same kind of
    /// thing.** ISO 32000-2 §8.5.4 intersects the clipping path with the object's shape and
    /// §11.3.7.2 multiplies the mask shape into the result, so a mark meeting a clip and a
    /// soft mask at once needs both factors and not only their product — [`scan::Clip::Both`]
    /// and ADR 0363. Storing it doubles what such an entry costs, which [`Built::held`]
    /// charges to the same budget the product is charged to.
    value: Option<Vec<u8>>,
}

impl Built {
    /// The bytes this entry holds on a surface `width` pixels across.
    fn held(&self, width: u32) -> usize {
        let mask = self.band.mask_bytes(width);
        match self.value {
            Some(_) => mask.saturating_mul(2),
            None => mask,
        }
    }
}

/// What a command's clip and soft mask together let it mark.
///
/// Three answers to one question, which is why they travel together: which rows of the surface
/// are worth drawing into, what coverage multiplies the command there, and — since the surface
/// is a run of whole rows and the band cannot say it — which columns.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Admitted<'a> {
    /// Rows of the surface the command may mark.
    pub(crate) band: Band,
    /// The coverage it is drawn through, and what that coverage *is*: §10.7.4's set of pixels
    /// or §11.6.5's value, which is what decides whether it composes with a mark by `min` or by
    /// a product. See [`scan::Clip`].
    pub(crate) mask: scan::Clip<'a>,
    /// Device rectangle outside which `mask` is zero, or `None` where nothing bounds it.
    pub(crate) admits: Option<tiny_skia::Rect>,
}

/// What a cached mask was built from.
///
/// Three kinds share one cache because they share one budget and one eviction order, and
/// because what a command asks for is the *effective* mask: a clip, a soft mask, or their
/// product. Keeping the product under its own key is what stops a page of masked text from
/// multiplying the two per glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Key {
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
pub(crate) struct MaskCache {
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
    pub(crate) budget: usize,
    /// The page-space rectangle each chain admits wholly, memoised — `pdf_render::DisplayList::
    /// clip_admits`, which [`MaskCache::cuts_nothing`] asks once per *command* and a page states
    /// one chain per clipping path (ADR 0132).
    admits_wholly: HashMap<usize, Option<pdf_render::Rect>>,
    /// Where a mark's own coverage is built before §10.7.4's intersection composes the two.
    /// Here because it is one buffer per band, which is what this cache already is.
    scratch: scan::Scratch,
    /// The rows each soft mask's consumers can read ([`mask_consumer_reach`]), shared by
    /// every cache of one rasterisation. Empty restricts nothing, which is what a cache
    /// built without a display list — every test's — gets.
    ///
    /// [`mask_consumer_reach`]: crate::soft_mask::mask_consumer_reach
    pub(crate) reach: Arc<HashMap<SoftMaskId, MaskReach>>,
}

/// Largest total size of the masks a [`MaskCache`] holds, in bytes.
///
/// Sized against the corpus rather than guessed. `bug1721218_reduced.pdf` is the heaviest
/// first page in the 974-document pdf.js corpus, and it never evicts: **3554 clip masks built
/// per render of one 612×792 page, peaking at 12.31 MB against this 32 MB** — 10.85 MB of clip
/// masks, 1.45 MB of soft masks, and three clip × soft-mask products alive at the peak.
///
/// The figure is ADR 0236's measurement. It is smaller than the page's mask count suggests
/// because ADR 0132 makes [`DisplayList::add_clip`] return an existing identifier for an
/// identical region, which is a change to how many masks this cache is asked for. **A margin
/// is a claim that decays like any other**, and the reason to re-take it rather than inherit it is
/// [`doc/todo/40`](../../../doc/todo/40-mask-chain-crop.md), whose stated blocker was that this
/// page had no room for its intermediate clips. It has 19.7 MB of room, and they cost 9.4.
///
/// The number is not lowered on that evidence, because nothing has been measured to be paying
/// for it; it is recorded so that a round which finds a document evicting knows what the
/// margin was.
///
/// [`DisplayList::add_clip`]: pdf_render::DisplayList::add_clip
pub(crate) const MASK_BUDGET: usize = 32 << 20;

impl MaskCache {
    /// A cache for one target, bounded by `budget` bytes.
    ///
    /// The budget is a parameter rather than [`MASK_BUDGET`] itself because a page drawn in
    /// strips has one cache per strip and they are alive at once: dividing the constant keeps
    /// a parallel render's mask memory equal to a serial one's.
    pub(crate) fn new(surface: Surface, anti_alias: bool, budget: usize) -> Self {
        Self {
            surface,
            anti_alias,
            built: HashMap::new(),
            order: VecDeque::new(),
            soft_order: VecDeque::new(),
            bytes: 0,
            soft_bytes: 0,
            budget,
            admits_wholly: HashMap::new(),
            scratch: scan::Scratch::for_page(surface.page.width, surface.page.height),
            reach: Arc::new(HashMap::new()),
        }
    }

    /// The same cache, restricting each soft mask to the rows its consumers can read.
    pub(crate) fn with_reach(mut self, reach: Arc<HashMap<SoftMaskId, MaskReach>>) -> Self {
        self.reach = reach;
        self
    }

    /// The rows [`CpuRasterizer::build_soft_mask`] evaluates mask `id` over: `marked` — the
    /// rows the group's own marks can reach on this surface — narrowed to the rows the
    /// mask's consumers can read ([`mask_consumer_reach`](crate::soft_mask::mask_consumer_reach)).
    ///
    /// [`CpuRasterizer::build_soft_mask`]: crate::CpuRasterizer::build_soft_mask
    pub(crate) fn soft_mask_rows(&self, id: SoftMaskId, marked: Band) -> Band {
        let one_row = Band {
            top: self.surface.rows.top,
            height: 1,
        };
        match self.reach.get(&id).copied() {
            // Not walked, or readable everywhere: evaluate over every row the group can mark.
            None | Some(MaskReach::Everywhere) => marked,
            // No consumer can read any row; `marked_rows`' own degenerate answer, whose
            // every value is the `outside` constant an unmarked pixel derives.
            Some(MaskReach::Nothing) => one_row,
            Some(MaskReach::Rows { top, bottom }) => {
                tiny_skia::Rect::from_ltrb(0.0, top, 1.0, bottom)
                    .and_then(|rows| Band::covering(rows, self.surface))
                    .and_then(|band| band.intersection(marked))
                    // Off this surface, or sharing no row with the group's marks: nothing
                    // a consumer reads here differs from the constant.
                    .unwrap_or(one_row)
            }
        }
    }

    /// [`MaskCache::effective`] for a group, whose marks this cannot bound.
    ///
    /// ISO 32000-2 §8.5.4 gives a group's result a shape of its own, "defined as the union of the
    /// shapes of its constituent objects", and nothing bounds that union without walking the
    /// elements — so a group keeps its clip whatever it contains ([`MaskCache::cuts_nothing`]).
    pub(crate) fn group_admits(
        &mut self,
        list: &DisplayList,
        clip: Option<ClipId>,
        mask: Option<SoftMaskId>,
    ) -> Result<Option<Admitted<'_>>, CpuRasterError> {
        self.effective(list, clip, mask, None)
    }

    /// Whether this chain takes nothing from a mark that reaches no further than `marks`, so
    /// that the mark may be drawn with no clip composed into it at all.
    ///
    /// # The clause states a set, and a set that contains a mark takes nothing from it
    ///
    /// ISO 32000-2 §10.7.4:
    ///
    /// > For clipping, the clipping region consists of the set of pixels that would be included
    /// > by a fill operation. Subsequent painting operations shall affect a region that is the
    /// > intersection of the set of pixels defined by the clipping region with the set of pixels
    /// > for the region to be painted.
    ///
    /// and §8.5.4 says the same of the shape — "\[t]he effective shape is the intersection of the
    /// object's intrinsic shape with the clipping path". `S ∩ C = S` where `S` lies inside `C`,
    /// at every pixel, the mark's own anti-aliased boundary included. ADR 1095.
    ///
    /// # Why the oracle needs it when it already composes by `min`
    ///
    /// ADR 0355 made this backend meet a clip by `min` rather than by a product, which is exact
    /// wherever the two boundaries coincide or nest **and both are measured the same way**. They
    /// are not always: a clip stating a rectangle is measured by §10.7.4's own closed form (ADR
    /// 0476) and a mark `crate::area` declines falls back to the library's converter, whose
    /// boundary pixel is rounded to a quarter (ADR 1082). A quarter that rounded *up* is then cut
    /// back to the clip's exact value and the quarter that rounded *down* is not restored, so a
    /// clip coinciding with a mark's own outline costs the page ink — 9.0% of `bug1844576.pdf`.
    /// Not stating the clip removes the arithmetic rather than making the two converters agree,
    /// and it is what `render_raster::scene::Encoder::cuts_nothing` does on the other backend
    /// (ADR 1088), so the two agree here by one rule rather than by luck.
    ///
    /// # Why the question is asked in page space
    ///
    /// Both rectangles are the document's, before this target's transform, so the answer is a
    /// property of the file rather than of this magnification: every affine carries a containment
    /// to a containment, and §10.7.4's pixel region — "painting any pixel whose half-open square
    /// region intersects the shape" — is a superset of the geometric region it is built from, so a
    /// containment that holds geometrically holds against the pixels the device clips with. It is
    /// also what keeps ADR 0219's row offset out of the comparison: a band's transform and the
    /// page's agree to within rounding rather than exactly, and a containment decided within that
    /// rounding would make a strip of a page a different picture from the page.
    ///
    /// `marks` is `None` for a command whose reach `encode::marks_reach` cannot bound, which keeps
    /// the clip.
    pub(crate) fn cuts_nothing(
        &mut self,
        list: &DisplayList,
        clip: ClipId,
        marks: Option<pdf_render::Rect>,
    ) -> bool {
        let Some(marks) = marks else { return false };
        self.admits_wholly
            .entry(clip.index())
            .or_insert_with(|| list.clip_admits(clip))
            .is_some_and(|admits| admits.contains(marks))
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
    ///
    /// [`CpuRasterizer::build_soft_mask`]: crate::CpuRasterizer::build_soft_mask
    pub(crate) fn effective(
        &mut self,
        list: &DisplayList,
        clip: Option<ClipId>,
        mask: Option<SoftMaskId>,
        marks: Option<pdf_render::Rect>,
    ) -> Result<Option<Admitted<'_>>, CpuRasterError> {
        // ISO 32000-2 §10.7.4's intersection with a region that contains the mark is the mark, so
        // the clip is left off it entirely: not composed, not built, and not banding the draw
        // either. A band is a surface that starts some rows down the page, and the mark's matrix
        // reaches it with those rows taken off its translation (ADR 0219) — the same real number,
        // rounded in another binade, which `tiny-skia`'s converter turns into one of its sixteen
        // samples and `crate::area` into a level: at 10x the paperclip of veraPDF's
        // `6-3-1-t01-fail-f` covers its pixel `(798, 567)` 255 on the page's rows and 240 on its
        // own `/BBox`'s. So the mark is drawn on the rows an unclipped mark is drawn on (ADR 2106).
        let contained = match clip {
            Some(clip) => self.cuts_nothing(list, clip, marks),
            None => false,
        };
        match (clip.filter(|_| !contained), mask) {
            // Unmasked, and still carrying the scratch buffer: a mark whose own portions share a
            // device pixel is composed in one whether or not anything clips it (§11.6.2, ADR 0590).
            (None, None) => Ok(Some(Admitted {
                band: self.surface.rows,
                mask: scan::Clip::Unclipped {
                    scratch: &self.scratch,
                },
                admits: None,
            })),
            // A clip on its own is §10.7.4's set of pixels, which is the one case a mark's
            // own coverage may be composed with by `min`. Built first and looked up after, the
            // way the `(Some, Some)` arm below does, because the composition needs the cache's
            // scratch buffer beside the mask and the two are different fields.
            (Some(clip), None) => {
                if self.get(list, clip)?.is_none() {
                    return Ok(None);
                }
                let Self { built, scratch, .. } = self;
                Ok(built
                    .get(&Key::Clip(clip))
                    .and_then(Option::as_ref)
                    .map(|built| Admitted {
                        band: built.band,
                        mask: scan::Clip::Region {
                            mask: &built.mask,
                            scratch,
                        },
                        admits: built.admits,
                    }))
            }
            (None, Some(mask)) => {
                // Handed the whole surface rather than the stored band: with no clip to
                // band the draw, the command draws over every row — §11.6.5.1 gives a soft
                // mask a value everywhere — and `tiny-skia` applies a mask to a pixmap of
                // exactly its own size. Returning the band instead would also move the
                // rows the command is drawn from, which is the arithmetic ADR 0219 pins.
                self.expand_soft_mask(mask)?;
                let entry = self.soft_mask(mask)?;
                Ok(Some(Admitted {
                    band: entry.band,
                    mask: scan::Clip::Value {
                        value: &entry.mask,
                        scratch: &self.scratch,
                    },
                    admits: entry.admits,
                }))
            }
            (Some(clip), Some(mask)) => {
                self.combine(list, clip, mask)?;
                let Self { built, scratch, .. } = self;
                let entry = built
                    .get(&Key::Both(clip, mask))
                    .ok_or(CpuRasterError::UnknownClip(clip))?
                    .as_ref();
                // The product is what every draw but a fill takes. A fill takes it beside the
                // soft mask it was made from, because §8.5.4 intersects the clip with the
                // object's own shape *before* §11.3.7.2 multiplies the mask shape in — so the
                // two may not be one buffer at the moment the mark's coverage arrives.
                Ok(entry.map(|built| Admitted {
                    band: built.band,
                    mask: match built.value.as_deref() {
                        Some(value) => scan::Clip::Both {
                            product: &built.mask,
                            value,
                            scratch,
                        },
                        // Only `combine` builds this key and it always stores the soft mask's
                        // rows, so this is unreachable; taking the product alone is what this
                        // backend did before ADR 0363 and is coarser rather than wrong.
                        None => scan::Clip::Value {
                            value: &built.mask,
                            scratch,
                        },
                    },
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
        // The soft mask is stored over the rows its group could mark and is `outside`
        // everywhere else (§11.6.5.1), so the clip's rows are assembled here: the constant
        // first, then the stored rows laid over it where the two bands overlap. Both bands
        // count page rows, so the three offsets below are row differences and nothing else.
        let mut soft_rows = vec![soft.outside; product.data().len()];
        let from = band.top.max(soft.band.top);
        let until = band
            .top
            .saturating_add(band.height)
            .min(soft.band.top.saturating_add(soft.band.height));
        if from < until {
            let length = (until.saturating_sub(from) as usize).saturating_mul(width);
            let into = (from.saturating_sub(band.top) as usize).saturating_mul(width);
            let out_of = (from.saturating_sub(soft.band.top) as usize).saturating_mul(width);
            let source = soft
                .mask
                .data()
                .get(out_of..out_of.saturating_add(length))
                .ok_or(CpuRasterError::UnknownSoftMask(mask))?;
            soft_rows
                .get_mut(into..into.saturating_add(length))
                .ok_or(CpuRasterError::UnknownSoftMask(mask))?
                .copy_from_slice(source);
        }
        for (value, &soft) in product.data_mut().iter_mut().zip(soft_rows.iter()) {
            // Two coverages multiply, through the one rounding `scan::intersected` also scales
            // the mark by: the minimum it then takes is only exact while both sides round the
            // same way. ADR 0363.
            *value = scan::scaled(*value, soft);
        }

        self.admit(
            Key::Both(clip, mask),
            Some(Built {
                mask: product,
                band,
                admits,
                // The clip's own zero: outside its band their product admits nothing.
                outside: 0,
                // The soft mask's rows are kept beside the product rather than only inside
                // it: §8.5.4's intersection happens before §11.3.7.2's multiplication, so a
                // fill's own coverage needs the two factors apart. ADR 0363.
                value: Some(soft_rows),
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
    pub(crate) fn holds_soft_mask(&self, id: SoftMaskId) -> bool {
        self.built.contains_key(&Key::Soft(id))
    }

    /// Stores an evaluated soft mask, covering the rows its group's marks could reach.
    ///
    /// `band` is those rows — `soft_mask::marked_rows`' answer — and `outside` is the mask's value
    /// everywhere else: §11.6.5.1 gives a soft mask one value for the whole area its group's
    /// marks never reached, so a band loses nothing as long as [`Built::outside`] carries
    /// the constant, which is exactly what `doc/todo/40` said an entry would have to do.
    /// [`MaskCache::combine`] substitutes it row by row; the one reader that needs the whole
    /// surface in one raster goes through [`MaskCache::expand_soft_mask`].
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
    ///
    /// [`CpuRasterizer`]: crate::CpuRasterizer
    /// [`CpuRasterizer::encode`]: crate::CpuRasterizer::encode
    pub(crate) fn admit_soft_mask(
        &mut self,
        id: SoftMaskId,
        mask: tiny_skia::Mask,
        band: Band,
        outside: u8,
    ) {
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
                outside,
                // A soft mask is not a product of anything, so there is nothing beside it.
                value: None,
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

    /// Rebuilds a soft mask's entry to cover the whole surface, if it does not already.
    ///
    /// The reader this serves is a command masked by a soft mask *alone*: with no clip to
    /// band the draw it draws over every row of the surface, the mask has a value on all of
    /// them (§11.6.5.1), and `tiny-skia` applies a mask to a pixmap of exactly its own size
    /// — so the banded entry cannot serve it the way it serves [`MaskCache::combine`]. The
    /// expansion writes `outside` everywhere and lays the stored band over it, which is byte
    /// for byte what the whole-surface conversion produced before the entries were banded:
    /// a row outside the band is a row the mask's group never marked, and the value derived
    /// from an unmarked pixel *is* the constant ([`pdf_render::SoftMask::outside`] is
    /// `value([0, 0, 0, 0])`).
    ///
    /// Memoised by replacement — the entry becomes the expanded one — so a mask read this
    /// way twice is expanded once, and a document whose masks are all read this way pays
    /// what storing every mask whole used to cost and no more. The growth is charged to the
    /// same budget, evicting other soft masks but never this one, for [`MaskCache::admit`]'s
    /// reason: the caller is about to draw with it.
    ///
    /// # Errors
    ///
    /// [`CpuRasterError::UnknownSoftMask`] for a mask never evaluated, and
    /// [`CpuRasterError::Allocation`] where the expanded raster cannot be built.
    fn expand_soft_mask(&mut self, id: SoftMaskId) -> Result<(), CpuRasterError> {
        let rows = self.surface.rows;
        let width = self.surface.width();
        let entry = self
            .built
            .get(&Key::Soft(id))
            .and_then(Option::as_ref)
            .ok_or(CpuRasterError::UnknownSoftMask(id))?;
        if entry.band == rows {
            return Ok(());
        }
        let mut values = vec![entry.outside; rows.mask_bytes(width)];
        let start =
            (entry.band.top.saturating_sub(rows.top) as usize).saturating_mul(width as usize);
        let end = start.saturating_add(entry.band.mask_bytes(width));
        values
            .get_mut(start..end)
            .ok_or(CpuRasterError::UnknownSoftMask(id))?
            .copy_from_slice(entry.mask.data());
        let held = entry.band.mask_bytes(width);
        let outside = entry.outside;
        let mask = tiny_skia::Mask::from_vec(
            values,
            tiny_skia::IntSize::from_wh(width, rows.height).ok_or(CpuRasterError::Allocation {
                width,
                height: rows.height,
            })?,
        )
        .ok_or(CpuRasterError::Allocation {
            width,
            height: rows.height,
        })?;
        self.soft_bytes = self
            .soft_bytes
            .saturating_sub(held)
            .saturating_add(rows.mask_bytes(width));
        // Replacement, not admission: the entry keeps its place in `soft_order`.
        self.built.insert(
            Key::Soft(id),
            Some(Built {
                mask,
                band: rows,
                admits: None,
                outside,
                value: None,
            }),
        );

        while self.soft_bytes > self.budget && self.soft_order.len() > 1 {
            let Some(oldest) = self.soft_order.pop_front() else {
                break;
            };
            if oldest == id {
                // Never the entry in hand. Parking it at the back keeps the loop finite:
                // everything in front of it is popped before it comes round again, and a
                // queue of one ends the loop.
                self.soft_order.push_back(oldest);
                continue;
            }
            if let Some(Some(entry)) = self.built.remove(&Key::Soft(oldest)) {
                self.soft_bytes = self
                    .soft_bytes
                    .saturating_sub(entry.band.mask_bytes(self.surface.width()));
            }
        }
        Ok(())
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
    /// **Three things about that were re-derived (ADR 0236) and all three moved**, which is why
    /// the item is still open and why it is
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
    ///   what shifting a whole number of rows does to `y·sy + ty` — a level or a few on the pixels
    ///   an edge puts on a sample row (ADR 1758), but not nothing. A parent's mask rows are
    ///   therefore *nearly* the prefix's contribution for the child's band, and this backend is
    ///   the oracle. Taking the item means either building intermediates in the child's band or
    ///   proving the difference away.
    ///
    /// **A fourth thing is measured, and it is why the `retain_mut` below is here rather than any
    /// of the three above** (ADR 0656). The exactness
    /// question has a *price*, and it is nearly the whole item: restricted to the prefixes a
    /// parent shares a band with — the ones reusable byte for byte, which is half the corpus's
    /// worst page's nodes — the proposal saves **5.6%** of that page's scanned mask rows where
    /// taking the departure saves **51.1%** (`pdf-model/examples/clip_chain_census`, which prints
    /// both arms). So there was never a cheap exact version of *reuse*.
    ///
    /// What the same census found instead is that reuse was the wrong question on that page:
    /// **three chain steps in four state a rectangle that admits every pixel of the band they are
    /// converted into**, and a step that admits everything can be *declined* rather than reused.
    /// That costs no departure of any kind — nothing is shifted between bands, so ADR 0219's
    /// arithmetic never enters — and it is what [`scan::admits_every_pixel`] answers.
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
                let device = room_for_marks(&clip.path, device);
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
                source: &clip.path,
                substituted: false,
                union_marks: None,
                transform: clip.transform,
                rule: clip.fill_rule,
                fill_rule: convert::fill_rule(clip.fill_rule),
                at: Transform::IDENTITY,
                mark: scan::Exact::default(),
            });
        }

        // A chain always holds at least the clip it was resolved from, so this is
        // unreachable; an empty clip chain would mean "clipped by nothing", which the
        // caller expresses by having no clip at all, and silently drawing unclipped here
        // would be exactly the plausible-looking wrong page this backend refuses to
        // produce.
        if shapes.is_empty() {
            return Err(CpuRasterError::UnknownClip(id));
        }
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
        let extent = (self.surface.width(), band.height);
        let shapes = state_regions(shapes, to_band, extent)?;
        let mut mask = tiny_skia::Mask::new(self.surface.width(), band.height).ok_or(
            CpuRasterError::Allocation {
                width: self.surface.width(),
                height: band.height,
            },
        )?;
        // A fresh mask blocks everything, so filling the root path is what opens it.
        let Some((root, nested)) = shapes.split_first() else {
            // Every step admitted the whole band, so the chain's region is the whole band: what
            // the fills above would have written, pixel for pixel, is the level a whole pixel's
            // coverage takes.
            mask.data_mut().fill(u8::MAX);
            return Ok(Some(Built {
                mask,
                band,
                admits,
                outside: 0,
                value: None,
            }));
        };
        self.compose(&mut mask, (root, nested), band)?;

        Ok(Some(Built {
            mask,
            band,
            admits,
            // §8.5.4: outside the clipping path nothing is admitted, so outside its band
            // there is nothing to state.
            outside: 0,
            // A clip on its own is already the set §10.7.4 states; nothing multiplies it.
            value: None,
        }))
    }

    /// Fills a resolved chain's shapes into `mask`: the root opens it, each nested step
    /// intersects — ISO 32000-2 §8.5.4, §10.7.4.
    ///
    /// §10.7.4:
    ///
    /// > Subsequent painting operations shall affect a region that is the intersection of the
    /// > set of pixels defined by the clipping region with the set of pixels for the region to
    /// > be painted.
    ///
    /// A step whose region is §10.7.4's union of two fills contributes both, composed before the
    /// intersection rather than after it, because the clipping region *is* the union (ADR 1231).
    fn compose(
        &self,
        mask: &mut tiny_skia::Mask,
        (root, nested): (&Shape<'_>, &[Shape<'_>]),
        band: Band,
    ) -> Result<(), CpuRasterError> {
        // One accumulator for the whole chain, on [`scan::Scratch`]'s own terms: `crate::area`
        // clears it per mark and grows it to the largest the chain holds.
        let mut cells = area::Buffers::default();
        // One scratch mask for the whole chain, allocated from the same width and height as
        // the mask above so that the two are the same size by construction. `tiny-skia`
        // allocates one per `intersect_path` call; the chain needs only one, and the
        // difference is 3554 allocations rather than 7108 on the corpus's worst page. A chain
        // of one step needs it only where that step states §10.7.4's union of two fills.
        let mut scratch = if nested.is_empty() && root.union_marks.is_none() {
            None
        } else {
            Some(self.band_mask(band)?)
        };
        // A second one only where a *nested* step states that union, because there the first is
        // already holding the step's own fill while the union is composed.
        let mut second = if nested.iter().any(|shape| shape.union_marks.is_some()) {
            Some(self.band_mask(band)?)
        } else {
            None
        };
        scan::mask_fill(
            mask,
            &mut cells,
            &root.path,
            root.fill_rule,
            self.anti_alias,
            (convert::transform(root.at), &root.mark),
        );
        if let (Some(marks), Some(scratch)) = (root.union_marks.as_ref(), scratch.as_mut()) {
            scan::mask_union(
                mask,
                (scratch, &mut cells),
                marks,
                self.anti_alias,
                convert::transform(root.at),
            );
        }
        let Some(scratch) = scratch.as_mut() else {
            return Ok(());
        };
        for shape in nested {
            let union = shape
                .union_marks
                .as_ref()
                .zip(second.as_mut())
                .map(|(marks, second)| (marks, &mut *second));
            scan::mask_intersect(
                mask,
                (scratch, &mut cells),
                &shape.path,
                shape.fill_rule,
                self.anti_alias,
                (convert::transform(shape.at), &shape.mark),
                union,
            );
        }
        Ok(())
    }

    /// A clear mask the width of this surface and the height of `band`.
    fn band_mask(&self, band: Band) -> Result<tiny_skia::Mask, CpuRasterError> {
        tiny_skia::Mask::new(self.surface.width(), band.height).ok_or(CpuRasterError::Allocation {
            width: self.surface.width(),
            height: band.height,
        })
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
        // The columns are the page's too: a window's first is its corner's (ADR 2132).
        #[expect(
            clippy::cast_precision_loss,
            reason = "as above, for the columns this surface spans"
        )]
        let (left, right) = (
            self.surface.shown.left as f32,
            self.surface.shown.left.saturating_add(self.surface.width()) as f32,
        );
        rect.left() <= left && rect.top() <= top && rect.right() >= right && rect.bottom() >= bottom
    }

    /// Stores an entry and evicts oldest-first until the budget is met.
    fn admit(&mut self, id: Key, built: Option<Built>) {
        if let Some(entry) = &built {
            self.bytes = self.bytes.saturating_add(entry.held(self.surface.width()));
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
                self.bytes = self.bytes.saturating_sub(entry.held(self.surface.width()));
            }
        }
    }
}

/// A clip's device bound, widened where §10.7.4's marks will reach past its path.
///
/// The mark a subpath with no extent contributes is the whole device pixel its collapsed axis
/// passes through, which lies up to one pixel outside the subpath's own bound; the band the mask
/// covers is computed from these bounds, so it has to hold what the region will admit. Widening it
/// only costs rows — the mask decides what is drawn either way, which is the same argument the
/// caller's own comment makes for leaving an unmeasurable bound out.
fn room_for_marks(path: &Path, device: tiny_skia::Rect) -> tiny_skia::Rect {
    if path.collapses() {
        device.outset(1.0, 1.0).unwrap_or(device)
    } else {
        device
    }
}

/// Gives every chain step the region ISO 32000-2 §10.7.4 says it states, dropping those that
/// state nothing.
///
/// §10.7.4 defines a clipping region as "the set of pixels that would be included by a fill
/// operation", and that one sentence decides all four things this does.
///
/// **A region is measured by the rule a mark is** — the same [`rectangular_mark`], for the same
/// reason `clip_intersection.rs` exists: a mark painted at its exact area under a region measured
/// to a quarter breaks `S ∩ C = S` (ADR 0476).
///
/// **A step whose fill would include every pixel of this band states nothing**, so it is dropped
/// here rather than scan-converted and then composed with `min`. That is the one saving in the
/// caller that costs no departure at all — see [`scan::admits_every_pixel`] — and on
/// the corpus's worst page it is three chain steps in four
/// (`pdf-model/examples/clip_chain_census`). ADR 0656.
///
/// **A step whose path rules a line states that line**, because the fill of a subpath with no
/// extent paints one: "A zero-width or zero-height rectangle paints a line 1 pixel wide".
/// [`pdf_render::clip_region`] builds that region for every backend — a clip is the one
/// construction where a region wider than its own path is the right answer, and a backend deciding
/// it alone is a decision none of them has made (ADR 1064).
///
/// **A substituted region is scan-converted rather than measured in closed form**, and that is the
/// one place §10.7.4's two halves can still part company. A mark is a run of *whole* device pixels,
/// so the closed form and the converter agree on it arithmetically — but the mark reaches
/// here through the inverse of the placement and back, and at a scale whose reciprocal is not exact
/// in binary that round trip lands the row a ten-thousandth of a pixel off. `mask_rectangle` writes
/// that sliver into the neighbouring row; `tiny-skia`'s four sample rows per pixel cannot see it,
/// and the fill's own marks go through exactly that converter. Measured at scale 3.5: the closed
/// form admits 212 pixels where the fill paints 106. The clause defines the region *by* the fill,
/// so the region takes the fill's converter.
fn state_regions(
    shapes: Vec<Shape<'_>>,
    to_band: ToDevice,
    extent: (u32, u32),
) -> Result<Vec<Shape<'_>>, CpuRasterError> {
    let mut stated = Vec::with_capacity(shapes.len());
    for mut shape in shapes {
        shape.at = to_band.of(shape.transform);
        match pdf_render::clip_region(shape.source, shape.rule, shape.at) {
            Some(pdf_render::ClipRegion::One(region, rule)) => {
                shape.path = convert::path(&region).ok_or(CpuRasterError::InvalidPath)?;
                shape.fill_rule = convert::fill_rule(rule);
                shape.substituted = true;
            }
            Some(pdf_render::ClipRegion::Union {
                filled: (filled, rule),
                marks,
            }) => {
                shape.path = convert::path(&filled).ok_or(CpuRasterError::InvalidPath)?;
                shape.fill_rule = convert::fill_rule(rule);
                shape.union_marks = Some(convert::path(&marks).ok_or(CpuRasterError::InvalidPath)?);
                shape.substituted = true;
            }
            None => {}
        }
        shape.mark = if shape.substituted {
            scan::Exact::default()
        } else {
            rectangular_mark(shape.source, shape.at)
        };
        if !scan::admits_every_pixel(&shape.mark, extent) {
            stated.push(shape);
        }
    }
    Ok(stated)
}

#[cfg(test)]
mod tests {

    use super::{Key, MASK_BUDGET, MaskCache};

    use crate::surface::{Band, Surface};

    use pdf_render::{
        Clip, ClipId, DisplayList, FillRule, Path, PathCommand, Point, Size, SoftMaskId,
        TargetSpec, Transform,
    };

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

    /// A clip chain whose steps admit the whole band builds the mask the remaining steps alone
    /// build — bytes, band and rectangle alike.
    ///
    /// The saving `MaskCache::build`'s `retain_mut` takes is byte-identical by arithmetic, so no
    /// page can discriminate it and this is what stands in for one: a bar wrapped in two
    /// page-covering rectangles against the same bar wrapped in nothing. What it protects against
    /// is not the arithmetic — `scan`'s own tests pin that — but a future condition that drops a
    /// step which does *not* admit everything, which would erase pixels the clip rejects with
    /// every gate in this tree still green on the pages that have no such chain. ADR 0656.
    #[test]
    fn a_chain_step_admitting_the_whole_band_changes_no_byte_of_the_mask() {
        let mut list = DisplayList::new(Size::new(200.0, 200.0));
        let rectangle = |ltrb: (f32, f32, f32, f32)| {
            let mut path = Path::new();
            path.push(PathCommand::MoveTo(Point::new(ltrb.0, ltrb.1)));
            path.push(PathCommand::LineTo(Point::new(ltrb.2, ltrb.1)));
            path.push(PathCommand::LineTo(Point::new(ltrb.2, ltrb.3)));
            path.push(PathCommand::LineTo(Point::new(ltrb.0, ltrb.3)));
            path.push(PathCommand::Close);
            path
        };
        let mut add = |path: Path, parent: Option<ClipId>| {
            list.add_clip(Clip {
                path,
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                parent,
            })
            .expect("under the clip limit")
        };
        // The bar's edge is fractional on purpose: a whole-pixel one would be 0 or 255 everywhere
        // and could not tell a mask that was composed from one that was not.
        let bar = rectangle((0.0, 40.25, 200.0, 44.75));
        let alone = add(bar.clone(), None);
        let outer = add(rectangle((-5.0, -5.0, 205.0, 205.0)), None);
        let inner = add(rectangle((-1.0, -1.0, 201.0, 201.0)), Some(outer));
        let wrapped = add(bar, Some(inner));

        let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("valid target");
        let mut cache = MaskCache::new(Surface::whole(target), true, MASK_BUDGET);
        let read = |cache: &mut MaskCache, id| {
            cache
                .get(&list, id)
                .expect("a rectangular chain builds")
                .map(|built| (built.band, built.admits, built.mask.data().to_vec()))
                .expect("the chain admits rows")
        };
        let (band, admits, bytes) = read(&mut cache, alone);
        assert!(
            bytes.iter().any(|&level| level > 0 && level < u8::MAX),
            "the bar's edge must be partly covered for this to discriminate"
        );
        assert_eq!(read(&mut cache, wrapped), (band, admits, bytes));
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
            cache.built.contains_key(&Key::Clip(id)),
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
            0,
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

    /// A soft mask stored over its band combines with a clip exactly as one stored whole.
    ///
    /// This is the exactness claim `doc/todo/40` said had to be settled before a soft mask
    /// could be banded: outside its group's marks the mask is one constant (§11.6.5.1), so
    /// an entry that carries the constant beside the band is the same *function* of a page
    /// row as the whole-surface raster was — and [`MaskCache::combine`] is the reader that
    /// has to substitute it. The clip here spans rows above, inside and below the soft
    /// band, so both substitution edges are exercised.
    #[test]
    fn a_banded_soft_mask_combines_as_a_whole_one_does() {
        let (list, ids, target) = stacked_clips(40);
        let clip = *ids.get(20).expect("forty clips"); // rows around y = 40..41
        let surface = Surface::whole(target);
        let width = target.width as usize;

        // Clip 20 sits at page y 40..41, which the flipped page transform puts at device
        // rows 159..160, so its band is a few rows around them. A two-row soft band inside
        // it leaves clip rows above and below to take the substituted constant — and the
        // constant is non-zero, which is the white-`/BC` case a zero fill would get wrong.
        let band = Band {
            top: 159,
            height: 2,
        };
        let outside = 200u8;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "row and column indices in a 200-pixel test target fit u8 modulo 256"
        )]
        let pattern: Vec<u8> = (0..band.height as usize * width)
            .map(|index| (index % 251) as u8)
            .collect();

        let mask = SoftMaskId::new(0);
        let banded_mask = tiny_skia::Mask::from_vec(
            pattern.clone(),
            tiny_skia::IntSize::from_wh(target.width, band.height).expect("a non-zero size"),
        )
        .expect("data matches the size");
        let mut banded = MaskCache::new(surface, true, MASK_BUDGET);
        banded.admit_soft_mask(mask, banded_mask, band, outside);
        banded.combine(&list, clip, mask).expect("combines");

        // The same mask stored the way every entry used to be: the constant everywhere,
        // the band's rows laid in.
        let mut whole_values = vec![outside; surface.rows.mask_bytes(target.width)];
        let start = band.top as usize * width;
        whole_values[start..start + pattern.len()].copy_from_slice(&pattern);
        let whole_mask = tiny_skia::Mask::from_vec(
            whole_values,
            tiny_skia::IntSize::from_wh(target.width, target.height).expect("a non-zero size"),
        )
        .expect("data matches the size");
        let mut whole = MaskCache::new(surface, true, MASK_BUDGET);
        whole.admit_soft_mask(mask, whole_mask, surface.rows, outside);
        whole.combine(&list, clip, mask).expect("combines");

        let read = |cache: &MaskCache| {
            cache
                .built
                .get(&Key::Both(clip, mask))
                .and_then(Option::as_ref)
                .map(|built| (built.band, built.mask.data().to_vec()))
                .expect("the product was built")
        };
        assert_eq!(
            read(&banded),
            read(&whole),
            "the banded entry and the whole one produced different products"
        );
    }

    /// Expansion writes the outside constant everywhere the band was not, and nothing else.
    ///
    /// This is the other reader of a banded entry: a command masked by a soft mask alone
    /// needs one raster covering the surface, and what it must receive is byte for byte
    /// what the whole-surface conversion used to produce — the band's rows unchanged, the
    /// constant on every other row (§11.6.5.1).
    #[test]
    fn an_expanded_soft_mask_is_the_constant_with_the_band_laid_in() {
        let (_, _, target) = stacked_clips(4);
        let surface = Surface::whole(target);
        let width = target.width as usize;

        let band = Band { top: 3, height: 2 };
        let outside = 55u8;
        let pattern: Vec<u8> = vec![7; band.height as usize * width];
        let mask = SoftMaskId::new(9);
        let banded_mask = tiny_skia::Mask::from_vec(
            pattern.clone(),
            tiny_skia::IntSize::from_wh(target.width, band.height).expect("a non-zero size"),
        )
        .expect("data matches the size");

        let mut cache = MaskCache::new(surface, true, MASK_BUDGET);
        cache.admit_soft_mask(mask, banded_mask, band, outside);
        cache.expand_soft_mask(mask).expect("expands");

        let entry = cache.soft_mask(mask).expect("still cached");
        assert_eq!(entry.band, surface.rows, "the entry now covers the surface");
        let mut expected = vec![outside; surface.rows.mask_bytes(target.width)];
        let start = band.top as usize * width;
        expected[start..start + pattern.len()].copy_from_slice(&pattern);
        assert_eq!(
            entry.mask.data(),
            expected.as_slice(),
            "expansion changed a value the conversion produced"
        );
        assert_eq!(
            cache.soft_bytes,
            surface.rows.mask_bytes(target.width),
            "the growth was not charged to the budget"
        );
        cache.expand_soft_mask(mask).expect("idempotent");
        assert_eq!(
            cache.soft_bytes,
            surface.rows.mask_bytes(target.width),
            "a second expansion charged the budget again"
        );
    }
}
