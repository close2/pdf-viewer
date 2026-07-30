//! The resolved drawing command buffer.

use std::sync::Arc;

use crate::geom::{Path, Rect, Size, Transform};
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
    /// # Why no isolation or knockout flag
    ///
    /// A backend is told to composite onto a transparent backdrop, which is §11.4.5's
    /// isolated group. The two attributes that would ask for anything else are decided
    /// before the command is built: `pdf-model` emits this command for a non-isolated
    /// group only where the computation is provably the same one (every element blending
    /// Normal — see ADR 0026), and reports the cases that are not. A flag no backend reads
    /// would be a placeholder rather than a description.
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
    /// # Errors
    ///
    /// Returns [`DisplayListError::TooManyClips`] if the list already holds
    /// `u32::MAX` clips. The bound exists because [`ClipId`] is a `u32`, and because
    /// a document that produces four billion clip regions is hostile rather than
    /// merely complex — refusing it is a resource-exhaustion defence.
    pub fn add_clip(&mut self, clip: Clip) -> Result<ClipId, DisplayListError> {
        let index = u32::try_from(self.clips.len()).map_err(|_| DisplayListError::TooManyClips)?;
        self.clips.push(clip);
        Ok(ClipId(index))
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
            crate::geom::Point::new(0.0, 0.0),
            crate::geom::Point::new(self.page_size.width, self.page_size.height),
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
