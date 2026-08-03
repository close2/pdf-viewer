//! The resolved drawing command buffer.

use std::collections::BTreeMap;
use std::hash::{Hash as _, Hasher as _};
use std::sync::Arc;

use crate::geom::{Path, Point, Rect, Size, Transform};
use crate::paint::{BlendMode, FillRule, Image, Paint, Stroke};
use crate::soft_mask::{SoftMask, SoftMaskId};

/// Identifies a clip region within a [`DisplayList`].
///
/// Clips are stored once and referenced by index because PDF clip state is
/// hierarchical and long-lived: a single clip commonly applies to thousands of
/// consecutive commands. Referencing avoids duplicating the path geometry per
/// command, and lets a backend recognise that a run of commands shares a clip and
/// so needs the clip mask rasterised only once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClipId(u32);

impl ClipId {
    /// Returns the index this identifier refers to.
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A clip region: an intersected path, optionally nested inside another clip.
#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    /// The clipping path, in the coordinate space given by `transform`.
    ///
    /// **An empty path is a region that admits nothing**, not an absent clip — see
    /// [`Clip::admits_nothing`].
    pub path: Path,
    /// Transform mapping `path` into page space.
    pub transform: Transform,
    /// How the interior of `path` is determined.
    pub fill_rule: FillRule,
    /// The enclosing clip, if any. Effective clip is the intersection of the chain.
    pub parent: Option<ClipId>,
}

impl Clip {
    /// Whether this clip admits no pixel at all, ISO 32000-2 §8.5.4 with §8.5.3.3.1.
    ///
    /// §8.5.4 defines a clip by the area a fill would cover — "the same area that would be
    /// filled by the `f` operator" — and §8.5.3.3.1 says a path whose last subpath is a
    /// single-point open one "shall be disregarded and not considered to be part of the
    /// path". A path that is *only* such a subpath therefore encloses nothing, and
    /// intersecting the current clip with nothing leaves nothing: every command inside it
    /// marks no pixel.
    ///
    /// It is stated here rather than in each backend because the two would answer it
    /// differently and neither answer would be visible: `tiny-skia` refuses an empty path
    /// outright, which failed the whole page, and `kurbo` clips to an empty region, which
    /// happens to be right for a reason nobody wrote down. That is trap 2 exactly.
    ///
    /// `issue9017_reduced.pdf` is the corpus document that states one — `568.938 673.022 m
    /// W n`, wrapped around a shading that all three reference renderers leave undrawn.
    #[must_use]
    pub fn admits_nothing(&self) -> bool {
        self.path.is_empty()
    }
}

/// One drawing operation, with all graphics state resolved.
///
/// Every variant carries its own absolute `transform` and `clip`, so commands are
/// independent of one another and of any ordering-dependent state. That
/// independence is what allows a backend to reorder or parallelise them.
///
/// # Geometry is shared, not copied
///
/// Paths are held behind an `Arc`. A page of text is mostly the same few dozen glyph
/// outlines repeated, and `pdf-font` already hands them out shared, so copying one into
/// every command duplicated the same segments hundreds of times: 3005 fill commands on a
/// dense specification page carried 101 320 path segments between them. Sharing costs an
/// atomic refcount and keeps the list `Send + Sync`, so the property below still holds.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Command {
    /// Fills the interior of a path.
    Fill {
        /// Geometry to fill.
        ///
        /// Shared rather than owned because glyphs dominate a text page and every
        /// occurrence of a letter is the same outline. Copying it per occurrence meant
        /// 101 320 path segments duplicated on one dense specification page; see the
        /// note on [`Command`].
        path: Arc<Path>,
        /// Transform mapping `path` into page space.
        transform: Transform,
        /// How the interior is determined.
        fill_rule: FillRule,
        /// How the interior is painted.
        paint: Paint,
        /// Active clip, or `None` for unclipped.
        clip: Option<ClipId>,
        /// Active soft mask, or `None` for none.
        ///
        /// §11.5.1's NOTE 2 calls a soft mask a *soft clip* — "a hard clip can be
        /// represented as a soft clip having shape values of 1.0 inside and 0.0 outside the
        /// clipping path" — which is why it is referenced beside the clip and resolved the
        /// same way.
        mask: Option<SoftMaskId>,
        /// How the result combines with the backdrop.
        blend: BlendMode,
    },
    /// Draws a decoded image into the unit square.
    ///
    /// PDF images occupy the unit square in user space, with the image's *top* row at
    /// y = 1: the transform carries everything else, including the flip that PDF's y-up
    /// space requires. Keeping that convention here rather than baking a flip into the
    /// samples means the image data is exactly what the file contained.
    Image {
        /// The decoded samples.
        image: Image,
        /// Transform mapping the unit square into page space.
        transform: Transform,
        /// Constant alpha applied on top of the image's own, in `0.0..=1.0`.
        alpha: f32,
        /// Active clip, or `None` for unclipped.
        clip: Option<ClipId>,
        /// Active soft mask, or `None` for none.
        ///
        /// §11.6.4.3 makes an image's own mask supersede this one — "[e]ither form of mask
        /// in the image dictionary shall override, for this image object only, the current
        /// soft mask in the graphics state" — so an image carrying an `/SMask` or a `/Mask`
        /// arrives here with `None` however the graphics state was left.
        mask: Option<SoftMaskId>,
        /// How the result combines with the backdrop.
        blend: BlendMode,
    },
    /// Composites a nested sequence of commands as a single object (§11.4).
    ///
    /// ISO 32000-2 §11.4.1: a transparency group is "a sequence of consecutive objects in
    /// a transparency stack that shall be collected together and composited to produce a
    /// single colour, shape, and opacity at each point", and
    ///
    /// > The result shall then be treated as if it were a single object for subsequent
    /// > compositing operations.
    ///
    /// That is the whole of what this command asks a backend for: draw `commands` onto a
    /// fully transparent backdrop, then paint the result once, under `alpha` and `blend`.
    ///
    /// # Why isolation is not a flag and knockout is
    ///
    /// A backend is told to composite onto a transparent backdrop, which is §11.4.5's
    /// isolated group. Isolation is decided before the command is built: `pdf-model` emits
    /// this command for a non-isolated group only where the computation is provably the
    /// same one (every element blending Normal — see ADR 0026), and reports the cases that
    /// are not. A flag no backend reads would be a placeholder rather than a description.
    ///
    /// Knockout is a flag because the two models differ in the *elements'* compositing and
    /// no rewriting of the element list can express it: see [`Self::knockout`].
    Group {
        /// The group's elements, in painting order.
        ///
        /// [`ClipId`]s inside refer to the enclosing [`DisplayList`], not to a table of
        /// their own: a group's elements are clipped by chains that begin outside it, so
        /// one table is what keeps a chain expressible.
        commands: Vec<Command>,
        /// Constant alpha applied to the composited group, in `0.0..=1.0`.
        ///
        /// §11.6.6 initialises the alpha constants to 1.0 *inside* a group precisely so
        /// that this one is applied once, here, rather than to each element as well.
        alpha: f32,
        /// Active clip, or `None` for unclipped.
        clip: Option<ClipId>,
        /// Soft mask applied to the composited group, or `None` for none.
        ///
        /// The mask in force at the `Do`, not inside the group: §11.6.6 initialises "the
        /// current soft mask to None " for the group's own content, for the same reason it
        /// initialises the alpha constants — the mask belongs to the group as an object.
        mask: Option<SoftMaskId>,
        /// How the composited group combines with its backdrop.
        blend: BlendMode,
        /// Whether the elements knock each other out (§11.4.6).
        ///
        /// > In a knockout group, each individual element shall be composited with the
        /// > group's initial backdrop rather than with the stack of preceding elements in
        /// > the group.
        ///
        /// The backdrop this command composites onto is transparent, so compositing an
        /// element with it yields the element itself — and the group's accumulated result
        /// is then "replaced by only a fraction of the result", the fraction being the
        /// element's *shape*. For a rasteriser that fraction is the coverage the element is
        /// drawn with, so a knockout element is Porter-Duff Source modulated by coverage,
        /// and its own blend mode has a transparent backdrop to blend against and therefore
        /// no effect.
        ///
        /// # What a backend may assume, and who guarantees it
        ///
        /// The clause is explicit that shape and opacity are different quantities — "[t]he
        /// existence of the knockout feature is the main reason for maintaining a separate
        /// shape value" — and a raster of premultiplied
        /// samples carries only the one. So `pdf-model` sets this **only** where every
        /// element's shape is its coverage: no element carries a soft mask, no image or
        /// shading contributes per-sample alpha, and no element is itself a group.
        /// Everything else keeps the report §11.4.6 has had since ADR 0026. A backend
        /// therefore implements one rule — draw each element with Source rather than over —
        /// and needs no shape channel to be right.
        knockout: bool,
    },
    /// Draws the outline of a path.
    Stroke {
        /// Geometry to stroke.
        path: Arc<Path>,
        /// Transform mapping `path` into page space.
        transform: Transform,
        /// Stroke parameters, in `path`'s coordinate space.
        stroke: Stroke,
        /// How the stroke is painted.
        paint: Paint,
        /// Active clip, or `None` for unclipped.
        clip: Option<ClipId>,
        /// Active soft mask, or `None` for none.
        mask: Option<SoftMaskId>,
        /// How the result combines with the backdrop.
        blend: BlendMode,
    },
}

impl Command {
    /// Returns the clip in effect for this command, if any.
    #[must_use]
    pub fn clip(&self) -> Option<ClipId> {
        match self {
            Self::Fill { clip, .. }
            | Self::Stroke { clip, .. }
            | Self::Image { clip, .. }
            | Self::Group { clip, .. } => *clip,
        }
    }

    /// Replaces the clip in effect for this command.
    ///
    /// Exists for one caller and states its reason here rather than there: a tiling pattern's
    /// cell is drawn under its `/BBox` clip and the clip is taken back off where it removed no
    /// geometry (§8.7.3.1, Table 74). Doing that by editing the command is what saves running
    /// the cell's content stream a second time to find out.
    pub fn set_clip(&mut self, clip: Option<ClipId>) {
        match self {
            Self::Fill { clip: at, .. }
            | Self::Stroke { clip: at, .. }
            | Self::Image { clip: at, .. }
            | Self::Group { clip: at, .. } => *at = clip,
        }
    }

    /// The region of the target this command can mark, ignoring its clip, or `None` where
    /// this cannot say.
    ///
    /// `to_device` maps page space to target pixels. The answer is a **bound**: a curve
    /// inside its control polygon and a mitre at its limit are both counted, so a caller may
    /// use it to rule a command out of a region but never to decide what it covers.
    ///
    /// `None` for a group, whose extent is its elements' and which a caller walking them
    /// bounds one at a time. A caller that cannot walk them must treat `None` as "anywhere",
    /// which is the safe reading and the only one that keeps this usable for skipping work.
    #[must_use]
    pub fn device_bounds(&self, to_device: Transform) -> Option<Rect> {
        match self {
            Self::Fill {
                path, transform, ..
            } => path.bounds(transform.then(to_device)),
            Self::Stroke {
                path,
                transform,
                stroke,
                ..
            } => {
                let placed = transform.then(to_device);
                // A mitre reaches `miter_limit × width / 2` from the join, which is the
                // furthest any stroke decoration goes: §8.4.3.5's caps reach half a width,
                // and a dash's caps are on the same line. One whole width times the limit is
                // that with room to spare, which is the direction a bound has to err in.
                //
                // **The reach is grown into the path's hull before it is mapped**, because a
                // line width is stated in the path's own space and this rectangle is in the
                // device's. Growing the mapped box by an unmapped width under-bounds every
                // stroke drawn at a scale above the slack above — a page at 3× with a mitre
                // near its limit — and `misses_target` would then skip a strip the mitre
                // reaches into. Found by `outline::stroked_bounds`, which answers the same
                // question the slow exact way and refused to fit inside this one.
                let reach = stroke.device_width(placed) * stroke.miter_limit.max(1.0);
                Some(path.hull()?.grown(reach).mapped(placed))
            }
            // §8.9.5.2 puts an image in the unit square and the transform carries the rest.
            Self::Image { transform, .. } => {
                let placed = transform.then(to_device);
                let corners = [
                    placed.apply(Point::new(0.0, 0.0)),
                    placed.apply(Point::new(1.0, 0.0)),
                    placed.apply(Point::new(1.0, 1.0)),
                    placed.apply(Point::new(0.0, 1.0)),
                ];
                let mut bounds = Rect::from_corners(corners[0], corners[0]);
                for corner in corners {
                    bounds = bounds.union(Rect::from_corners(corner, corner));
                }
                Some(bounds)
            }
            Self::Group { .. } => None,
        }
    }

    /// Returns the blend mode this command is composited under (§11.3.5).
    ///
    /// Every command carries one, including a group, for which §11.6.6 makes it the mode
    /// the group's *result* is painted with rather than one its elements see.
    #[must_use]
    pub fn blend(&self) -> BlendMode {
        match self {
            Self::Fill { blend, .. }
            | Self::Stroke { blend, .. }
            | Self::Image { blend, .. }
            | Self::Group { blend, .. } => *blend,
        }
    }

    /// Returns the soft mask in effect for this command, if any.
    ///
    /// Per command rather than per run of commands because §11.6.4.3's NOTE 2 makes the
    /// mask apply to one object at a time: "[i]f a soft mask is applied when painting two or
    /// more overlapping objects, the effect of the mask multiplies with itself in the area
    /// of overlap". Applying it once to a run of objects would be a different picture, and
    /// the one the clause warns is *not* what a mask does.
    #[must_use]
    pub fn mask(&self) -> Option<SoftMaskId> {
        match self {
            Self::Fill { mask, .. }
            | Self::Stroke { mask, .. }
            | Self::Image { mask, .. }
            | Self::Group { mask, .. } => *mask,
        }
    }
}

/// Everything needed to rasterise one page.
///
/// A display list is self-contained and immutable once built: it borrows nothing
/// from the document it came from. That property is what lets the parser run in a
/// sandboxed process and hand only this across the process boundary, and what lets
/// rasterisation happen on any thread without synchronisation.
///
/// Deliberately not `Default`: a display list without a page size is not a
/// meaningful value, and a zero-sized default would silently produce empty renders
/// rather than a compile error at the call site that forgot to supply the size.
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayList {
    /// Page dimensions in PDF user-space units (1/72 inch).
    pub page_size: Size,
    commands: Vec<Command>,
    clips: Vec<Clip>,
    soft_masks: Vec<SoftMask>,
    /// Which [`ClipId`]s describe which region, so that [`DisplayList::add_clip`] can hand
    /// back one that is already there.
    ///
    /// Keyed by a hash of the clip's content with the identifiers that hashed to it, because
    /// the alternative — a map keyed by the clip itself — would store a second copy of every
    /// clipping path. A collision costs one `PartialEq`, which is what decides the answer.
    clip_index: BTreeMap<u64, Vec<ClipId>>,
}

impl DisplayList {
    /// Creates an empty display list for a page of the given size.
    #[must_use]
    pub fn new(page_size: Size) -> Self {
        Self {
            page_size,
            commands: Vec::new(),
            clips: Vec::new(),
            soft_masks: Vec::new(),
            clip_index: BTreeMap::new(),
        }
    }

    /// Appends a drawing command.
    pub fn push(&mut self, command: Command) {
        self.commands.push(command);
    }

    /// Returns how many commands have been pushed.
    ///
    /// Exists for [`DisplayList::split_off_commands`]: a caller that is about to run
    /// content which may turn out to be a transparency group records the mark here and
    /// takes what was drawn after it.
    #[must_use]
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    /// Removes and returns every command pushed after `at`, leaving the clips behind.
    ///
    /// A transparency group is discovered from the outside — the elements are drawn, and
    /// only then are they collected into one object (§11.4.1) — so a builder needs to take
    /// back what it has just pushed. The clip table is deliberately not split: a group's
    /// elements are clipped by chains that begin outside the group, and renumbering them
    /// would break every parent link.
    ///
    /// A mark past the end returns nothing rather than panicking, since it can only mean
    /// the commands were already taken.
    pub fn split_off_commands(&mut self, at: usize) -> Vec<Command> {
        if at >= self.commands.len() {
            return Vec::new();
        }
        self.commands.split_off(at)
    }

    /// Registers a clip region and returns its identifier.
    ///
    /// **A region already in the table gets its existing identifier back**, and that is not
    /// tidiness. A backend caches the *effective mask* of a chain, keyed by the leaf's
    /// identifier — which is a name — so two identifiers describing one region are two
    /// page-sized masks built and two page-sized buffers zeroed. Page 6 of ISO 32000-2 states
    /// **one** clipping region and `q`/`W n`/`Q` around **303** text runs, and before this the
    /// CPU backend built 303 identical masks: 18.1% of that page's rasterisation was `calloc`
    /// under `tiny_skia::Mask::new`, all of it. ADR 0132.
    ///
    /// The comparison is exact — the same path, transform, fill rule and parent — so nothing
    /// here decides that two regions are "close enough". A producer that states two clips one
    /// ten-thousandth of a point apart gets two identifiers, and gets what it asked for.
    ///
    /// # Errors
    ///
    /// Returns [`DisplayListError::TooManyClips`] if the list already holds
    /// `u32::MAX` clips. The bound exists because [`ClipId`] is a `u32`, and because
    /// a document that produces four billion clip regions is hostile rather than
    /// merely complex — refusing it is a resource-exhaustion defence.
    pub fn add_clip(&mut self, clip: Clip) -> Result<ClipId, DisplayListError> {
        let digest = Self::clip_digest(&clip);
        if let Some(candidates) = self.clip_index.get(&digest)
            && let Some(existing) = candidates
                .iter()
                .find(|id| self.clips.get(id.index()) == Some(&clip))
        {
            return Ok(*existing);
        }

        let index = u32::try_from(self.clips.len()).map_err(|_| DisplayListError::TooManyClips)?;
        self.clips.push(clip);
        let id = ClipId(index);
        self.clip_index.entry(digest).or_default().push(id);
        Ok(id)
    }

    /// Registers a soft mask and returns its identifier.
    ///
    /// Stored in a table for the same reason clips are: a mask set by `gs` applies to every
    /// object painted until it is replaced, and duplicating its group per command would
    /// copy the whole group thousands of times over a page of masked text.
    ///
    /// # Errors
    ///
    /// Returns [`DisplayListError::TooManySoftMasks`] if the list already holds `u32::MAX`
    /// of them — the same bound and the same reasoning as [`DisplayList::add_clip`]'s.
    pub fn add_soft_mask(&mut self, mask: SoftMask) -> Result<SoftMaskId, DisplayListError> {
        let index =
            u32::try_from(self.soft_masks.len()).map_err(|_| DisplayListError::TooManySoftMasks)?;
        self.soft_masks.push(mask);
        Ok(SoftMaskId::new(index))
    }

    /// Returns the soft mask with the given identifier.
    ///
    /// Returns `None` only if the identifier came from a different display list, which is a
    /// programming error rather than a document defect.
    #[must_use]
    pub fn soft_mask(&self, id: SoftMaskId) -> Option<&SoftMask> {
        self.soft_masks.get(id.index())
    }

    /// Returns how many soft masks the list holds.
    ///
    /// A backend evaluates each one at target resolution, so this is how many extra
    /// rasters a page costs.
    #[must_use]
    pub fn soft_mask_count(&self) -> usize {
        self.soft_masks.len()
    }

    /// Returns the commands in painting order.
    #[must_use]
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// A hash of everything [`Clip`]'s `PartialEq` compares.
    ///
    /// Floating-point values are hashed by their bits, which is stricter than `==` in exactly
    /// one place — `-0.0` and `0.0` hash apart and compare equal — and stricter is the safe
    /// direction here: a missed match costs one extra clip, a false match would clip a page by
    /// the wrong region. `f32::NAN` is unreachable in a clip that any backend can draw.
    fn clip_digest(clip: &Clip) -> u64 {
        let mut hasher = std::hash::DefaultHasher::new();
        for command in clip.path.commands() {
            std::mem::discriminant(command).hash(&mut hasher);
            for value in Self::command_values(command) {
                value.to_bits().hash(&mut hasher);
            }
        }
        for value in [
            clip.transform.a,
            clip.transform.b,
            clip.transform.c,
            clip.transform.d,
            clip.transform.e,
            clip.transform.f,
        ] {
            value.to_bits().hash(&mut hasher);
        }
        std::mem::discriminant(&clip.fill_rule).hash(&mut hasher);
        clip.parent.map(ClipId::index).hash(&mut hasher);
        hasher.finish()
    }

    /// The coordinates one path command names, in order.
    fn command_values(command: &crate::geom::PathCommand) -> Vec<f32> {
        match *command {
            crate::geom::PathCommand::MoveTo(p) | crate::geom::PathCommand::LineTo(p) => {
                vec![p.x, p.y]
            }
            crate::geom::PathCommand::CurveTo(a, b, c) => vec![a.x, a.y, b.x, b.y, c.x, c.y],
            crate::geom::PathCommand::Close => Vec::new(),
        }
    }

    /// Returns the clip with the given identifier.
    ///
    /// Returns `None` only if the identifier came from a different display list,
    /// which is a programming error rather than a document defect.
    #[must_use]
    pub fn clip(&self, id: ClipId) -> Option<&Clip> {
        self.clips.get(id.index())
    }

    /// Returns the page bounds in user space, with the origin at the bottom left.
    #[must_use]
    pub fn page_bounds(&self) -> Rect {
        Rect::from_corners(
            Point::new(0.0, 0.0),
            Point::new(self.page_size.width, self.page_size.height),
        )
    }
}

/// Failures that can arise while building a display list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum DisplayListError {
    /// More clip regions were added than a [`ClipId`] can address.
    #[error("display list exceeded the maximum of {} clip regions", u32::MAX)]
    TooManyClips,
    /// More soft masks were added than a [`SoftMaskId`] can address.
    #[error("display list exceeded the maximum of {} soft masks", u32::MAX)]
    TooManySoftMasks,
}
