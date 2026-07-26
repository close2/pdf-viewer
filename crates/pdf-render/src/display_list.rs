//! The resolved drawing command buffer.

use std::sync::Arc;

use crate::geom::{Path, Rect, Size, Transform};
use crate::paint::{BlendMode, FillRule, Image, Paint, Stroke};

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
    pub path: Path,
    /// Transform mapping `path` into page space.
    pub transform: Transform,
    /// How the interior of `path` is determined.
    pub fill_rule: FillRule,
    /// The enclosing clip, if any. Effective clip is the intersection of the chain.
    pub parent: Option<ClipId>,
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
        /// How the result combines with the backdrop.
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
        /// How the result combines with the backdrop.
        blend: BlendMode,
    },
}

impl Command {
    /// Returns the clip in effect for this command, if any.
    #[must_use]
    pub fn clip(&self) -> Option<ClipId> {
        match self {
            Self::Fill { clip, .. } | Self::Stroke { clip, .. } | Self::Image { clip, .. } => *clip,
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
}

impl DisplayList {
    /// Creates an empty display list for a page of the given size.
    #[must_use]
    pub fn new(page_size: Size) -> Self {
        Self {
            page_size,
            commands: Vec::new(),
            clips: Vec::new(),
        }
    }

    /// Appends a drawing command.
    pub fn push(&mut self, command: Command) {
        self.commands.push(command);
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
}
