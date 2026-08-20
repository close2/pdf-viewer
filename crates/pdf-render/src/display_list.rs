//! The resolved drawing command buffer.

use std::collections::BTreeMap;
use std::hash::{Hash as _, Hasher as _};
use std::sync::Arc;

use crate::geom::{Path, Point, Rect, Size, Transform};
use crate::paint::{BlendMode, FillRule, ImageSource, Paint, Stroke};
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

/// The four-component blending colour space one group's elements composite in
/// (ISO 32000-2 §11.6.6, §11.7.2).
///
/// §11.7.2 states what a group's own colour space means for the marks inside it:
///
/// > If the colour space of a graphics object within the group is not equivalent to the
/// > group's blending colour space, then it shall be converted to the group's colour space ,
/// > and all blending and compositing computations shall be done in that space (see 11.3.4,
/// > "Blending colour space"). The resulting colours shall then be interpreted in the
/// > group's colour space when the group is subsequently composited with its backdrop.
///
/// Where that space has four components and the parent composites on the device's three,
/// the same construction that draws §11.4.7's page group applies one scope down
/// ([`crate::blending`]): the group's elements are interpreted twice, once carrying the
/// additive complements of cyan, magenta and yellow — [`Command::Group`]'s own `commands` —
/// and once carrying the complement of black, which is [`GroupBlending::black`]. A backend
/// composites each list onto §11.4.5's transparent backdrop, resolves the pair through
/// [`GroupBlending::space`] with [`crate::blending::resolve`], and only then paints the
/// result onto the parent — which is exactly where §11.7.2's second sentence puts the
/// interpretation of the group's accumulated colour.
///
/// The two lists are two interpretations of one content stream and differ only in what a
/// colour resolved to; their geometry, clips, blend modes and nesting are identical by
/// construction, and `pdf-model` verifies that before pairing them.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupBlending {
    /// The conversion out of the group's four components, applied to the composited pair
    /// before the group is painted onto its parent.
    pub space: crate::blending::BlendingSpace,
    /// The same elements, drawn in the black component of the space.
    pub black: Vec<Command>,
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
        /// Where the samples come from.
        ///
        /// Usually [`ImageSource::Decoded`], which is the raster on the grid the file states.
        /// An image whose grid is not settled until the device scale is — §11.6.5.2's
        /// soft-mask image on a grid of its own is the case this exists for — arrives as
        /// [`ImageSource::AtDeviceScale`] and is produced by the backend at the resolution it
        /// is about to draw at.
        image: ImageSource,
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
    /// That is the whole of what this command asks a backend for: draw `commands` onto the
    /// backdrop [`Self::isolated`] names, then paint the result once, under `alpha` and
    /// `blend`.
    ///
    /// # Why knockout is a flag
    ///
    /// The two models differ in the *elements'* compositing and no rewriting of the element
    /// list can express it: see [`Self::knockout`].
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
        /// What the elements are composited *onto* (§11.4.5, §11.4.4).
        ///
        /// `true` is §11.4.5's isolated group, which is what a layer in any rasterising
        /// library is:
        ///
        /// > An isolated group is one whose elements shall be composited onto a fully
        /// > transparent initial backdrop rather than onto the group's backdrop.
        ///
        /// `false` is §11.4.4's own model, where the elements composite onto the backdrop
        /// the group is being painted over and the backdrop's contribution is then taken
        /// out again (its NOTE 3: "Essentially, these formulas remove the contribution of
        /// the group backdrop from the computed results."). It matters only where an
        /// element *blends*, which is what §11.4.4's NOTE 2 gives as the whole reason the
        /// two kinds of group exist — with every element painting Normal the backdrop is
        /// composited in and removed again exactly, and the two models are the same page.
        ///
        /// # What a backend must do for `false`, and why it needs no second alpha
        ///
        /// §11.4.4's removal divides by Table 140's *group alpha* — the elements' own
        /// accumulated alpha, "excluding the initial backdrop" — which is not the alpha a
        /// raster of premultiplied samples holds. NOTE 4 is the clause's own advice:
        ///
        /// > For shape and alpha, backdrop removal can be accomplished by maintaining two
        /// > sets of variables to hold the accumulated values.
        ///
        /// A rasteriser has one set. **It does not need the second**, because the quantity
        /// the removal divides out is multiplied straight back in when the group's result is
        /// composited with that same backdrop under §11.3.3. Writing `B` for the backdrop
        /// and `E(B)` for the elements composited onto it, both premultiplied, and `w` for
        /// [`Self::alpha`] times [`Self::mask`] at the pixel, the two steps together are
        ///
        /// ```text
        /// result = (1 − w) × B + w × E(B)
        /// ```
        ///
        /// — an ordinary interpolation, exact for every backdrop alpha and every blend mode
        /// *inside* the group. ADR 0237 has the derivation; `w = 1` reduces it to `E(B)`,
        /// which is NOTE 5's flattening, and that is the case `pdf-model` never builds a
        /// group for at all.
        ///
        /// # What is guaranteed, because the collapse has one condition
        ///
        /// The step that cancels is §11.3.3 with the **Normal** blend function. Under any
        /// other, the group's own colour is needed and with it the group alpha, and the
        /// identity is false — 0.60 of full scale apart at its worst over random inputs. So
        /// `pdf-model` emits `false` only where [`Self::blend`] is [`BlendMode::Normal`]
        /// and no enclosing group is a knockout group, and reports the groups it therefore
        /// cannot draw. A backend may rely on both and should refuse rather than
        /// approximate if handed anything else.
        ///
        /// `false` **with [`Self::knockout`] set** is §11.4.6's non-isolated knockout
        /// group, whose initial backdrop is the group's own — see [`Self::knockout`] for
        /// what that asks of a backend and what `pdf-model` guarantees about its elements.
        isolated: bool,
        /// Whether the elements knock each other out (§11.4.6).
        ///
        /// > In a knockout group, each individual element shall be composited with the
        /// > group's initial backdrop rather than with the stack of preceding elements in
        /// > the group.
        ///
        /// The backdrop this command composites onto is transparent, so compositing an
        /// element with it yields the element itself — and the group's accumulated result
        /// is then "replaced by only a fraction of the result", the fraction being the
        /// element's *shape*. For a rasteriser that fraction is usually the coverage the
        /// element is drawn with, so such an element is Porter-Duff Source modulated by
        /// coverage, and its own blend mode has a transparent backdrop to blend against and
        /// therefore no effect.
        ///
        /// # What a backend may assume, and who guarantees it
        ///
        /// The clause is explicit that shape and opacity are different quantities:
        ///
        /// > The existence of the knockout feature is the main reason for maintaining a
        /// > separate shape value rather than only a single alpha that combines shape and
        /// > opacity.
        ///
        /// A raster of premultiplied samples carries only the one, so an element whose
        /// shape is *not* its coverage arrives as [`Command::Shaped`], which states the two
        /// separately. Every other element of a knockout group has a shape a backend can
        /// read off the coverage it draws with, and `pdf-model` guarantees that: it emits
        /// this flag only where every element is one or the other, and reports the groups
        /// where a shape cannot be stated at all.
        ///
        /// # A non-isolated knockout group, where the two backdrops differ
        ///
        /// With [`Self::isolated`] `false` beside this flag, the initial backdrop every
        /// element composites with is **the group's own backdrop** rather than
        /// transparency — "[a] nonisolated knockout group composites its topmost enclosing
        /// element with the group's backdrop" — and the staged reading above no longer
        /// holds, because an element's blend mode now has that backdrop to blend against.
        /// §11.4.6's two stages per element `i`, in premultiplied form with `B` the
        /// initial backdrop, `P` the accumulation (which starts at `B`), `fᵢ` the
        /// element's shape and `Eᵢ(B)` the element composited onto `B`, are
        ///
        /// ```text
        /// Pᵢ = (1 − fᵢ) × Pᵢ₋₁ + fᵢ × Eᵢ(B)
        /// ```
        ///
        /// so a backend has to retain `B` beside the accumulation and composite each
        /// element against it in a scratch of its own. `pdf-model` emits the combination
        /// only where **every** element is a [`Command::Shaped`] (the per-pixel `fᵢ` has
        /// to come from somewhere), [`Self::blend`] is [`BlendMode::Normal`] (the
        /// non-isolated collapse's own condition, above), and no enclosing group is a
        /// knockout group. A backend that cannot retain the backdrop refuses the
        /// combination rather than substituting either backdrop for the other.
        knockout: bool,
        /// The four-component blending colour space this group's elements composite in,
        /// or `None` for a group composited in the space its parent already composites in
        /// (§11.6.6, §11.7.2). See [`GroupBlending`].
        ///
        /// `pdf-model` emits `Some` only for an isolated group, because §11.6.6 gives a
        /// `/CS` effect for isolated groups alone — "[f]or non-isolated groups, or if no
        /// group colour space is specified, the group colour space shall be inherited from
        /// the parent group or page". A backend that cannot composite the pair and resolve
        /// it must refuse the command: the colours the two lists hold are ink complements,
        /// and drawing either list alone paints the page in them.
        blending: Option<Box<GroupBlending>>,
    },
    /// An object of a knockout group whose shape is not the coverage it is drawn with
    /// (ISO 32000-2 §11.4.6, §11.6.4.2).
    ///
    /// # Why the display list says this twice
    ///
    /// §11.6.4.2 gives an object's *shape* from its geometry alone — for a path "the shape
    /// shall always be 1.0 inside and 0.0 outside the path" — while a soft mask and the
    /// constant alpha are *opacity*, §11.6.4.3 and §11.6.4.4. §11.6.4.3:
    ///
    /// > The mask may serve as a source of either shape … or opacity … values, depending on
    /// > the setting of the alpha source parameter in the graphics state (see 8.4,
    /// > "Graphics state").
    ///
    /// Everywhere except §11.4.6 the two quantities are only ever multiplied together, and
    /// one alpha channel per pixel carries the product exactly. A knockout group is where
    /// they are used apart: §11.4.6's first stage composites the object with the group's
    /// initial backdrop
    ///
    /// > disregarding the object's shape and using a source shape value of 1.0 everywhere
    ///
    /// and its second stage takes a
    ///
    /// > weighted average of this result with the object's immediate backdrop, using the
    /// > source shape as the weighting factor
    ///
    /// — so the shape weights the *backdrop* while shape × opacity weights the object.
    ///
    /// # The two steps a backend draws, and why they are exactly the clause
    ///
    /// Onto the transparent initial backdrop a group is built on, §11.4.6's two stages come
    /// to one line per pixel in premultiplied form: with the accumulated result `P`, the
    /// object's shape `f` and its premultiplied colour `S` (which already carries
    /// `f × opacity`),
    ///
    /// ```text
    /// P' = (1 − f) × P + S
    /// ```
    ///
    /// which a backend draws as **Porter-Duff Destination-Out with [`Self::shape`], then
    /// Plus with [`Self::object`]** — the first factor and the second term, in that order.
    /// Ordinary source-over would multiply the backdrop term by `(1 − f × opacity)` as
    /// well, which is right only where the object is opaque or its shape is 0 or 1.
    ///
    /// # What is guaranteed
    ///
    /// This command appears only as a direct element of a [`Self::Group`] whose `knockout`
    /// is set. Outside one the shape is unused — §11.4.4's non-knockout formulas reach it
    /// only through `shape × opacity` — so a backend may draw `object` alone there.
    Shaped {
        /// The object, drawn exactly as it would be anywhere else.
        object: Box<Command>,
        /// The same object with every source of *opacity* removed, so that the alpha a
        /// backend draws it with is §11.6.4.2's shape.
        ///
        /// A backend reads the coverage and alpha this marks and ignores its colour.
        shape: Box<Command>,
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
            // A shaped element's two halves carry the same clip — a clip constrains a
            // shape as much as it constrains a mark — so the object answers for both.
            Self::Shaped { object, .. } => object.clip(),
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
            // Both halves, or the shape would knock out an area the object no longer marks.
            Self::Shaped { object, shape } => {
                object.set_clip(clip);
                shape.set_clip(clip);
            }
        }
    }

    /// The geometry this command draws, or `None` where it draws none.
    ///
    /// A group's is its elements', which a caller walking them reads one at a time; an image's is
    /// the unit square its transform places, which is not a path.
    #[must_use]
    pub fn path(&self) -> Option<&Arc<Path>> {
        match self {
            Self::Fill { path, .. } | Self::Stroke { path, .. } => Some(path),
            Self::Image { .. } | Self::Group { .. } => None,
            Self::Shaped { object, .. } => object.path(),
        }
    }

    /// The geometry this command draws, for a caller replacing it, or `None` where it draws
    /// none.
    ///
    /// Exists for one caller and states its reason here rather than there, as
    /// [`Self::set_clip`] does: a tiling pattern's cell may state a mark that a neighbouring
    /// cell states again a whole lattice step away, and folding the pair to the one mark of the
    /// tiling they both describe is an edit to the path of a command already emitted
    /// (§8.7.3.1, §11.6.2 — see [`crate::repeat`]). Doing it by editing the command is what
    /// saves running the cell's content stream a second time to find out.
    #[expect(
        clippy::match_same_arms,
        reason = "the two `None`s are different answers: an image and a group have no path \
                  to hand out, and a shaped element has one it withholds"
    )]
    pub fn path_mut(&mut self) -> Option<&mut Arc<Path>> {
        match self {
            Self::Fill { path, .. } | Self::Stroke { path, .. } => Some(path),
            Self::Image { .. } | Self::Group { .. } => None,
            // **Not** the object's, although [`Self::path`] reads it: this hands out a
            // geometry to *replace*, and replacing the object's without its shape's would
            // leave the pair describing two different marks. A shaped element is left whole.
            Self::Shaped { .. } => None,
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
            // The shape is the object with its opacity taken off, so the two mark the same
            // region and the object's bound holds for both.
            Self::Shaped { object, .. } => object.device_bounds(to_device),
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
            // §11.4.6 leaves a knockout element's own mode nothing to blend against, but
            // the object states it and this reports what the object says.
            Self::Shaped { object, .. } => object.blend(),
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
            // The object's. The shape carries none by construction: §11.6.4.3's mask is
            // opacity, which is exactly what a shape has had removed.
            Self::Shaped { object, .. } => object.mask(),
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
    /// §11.4.7's blending colour space, where it is one whose components this list carries in
    /// two rasters rather than one. See [`DisplayList::set_blending`].
    blending: Option<crate::blending::BlendingSpace>,
    /// The same page, drawn in the black component of that space.
    black: Option<Box<DisplayList>>,
    /// §14.11.2.1's boundary, in this list's own space, or `None` for a list that is not a
    /// page. See [`DisplayList::content_clip`].
    content_clip: Option<Rect>,
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
            blending: None,
            black: None,
            content_clip: None,
        }
    }

    /// States where §14.11.2.1 stops this page's contents, in this list's own space.
    ///
    /// ISO 32000-2 §14.11.2.1, of the five page boundaries, and it is a `shall`:
    ///
    /// > The crop box defines the region to which the contents of the page shall be clipped
    /// > (cropped) when displayed or printed.
    ///
    /// §12.2's `/ViewClip` may name a different one of the five — Table 147 states it as "the
    /// name of the page boundary to which the contents of a page shall be clipped when viewing
    /// the document on the screen", defaulting to `CropBox` — so what a caller sets here is
    /// *that* boundary rather than the crop box by name, and the two are the same rectangle for
    /// every document that states no preference.
    ///
    /// # Why the region is here rather than in a clipping path
    ///
    /// A [`Clip`] would say the same thing and would cost every page a page-sized coverage mask
    /// and a masked composite per command, on the ninety-seven percent of documents that never
    /// mark outside their own boundary. This is one rectangle, in the list's own space, that a
    /// backend maps into its target and applies once — see [`crate::crop_area`].
    ///
    /// # Why a list can decline to have one
    ///
    /// `None` means *this list is not a page*: a host's chrome — a sidebar, a find bar, a modal
    /// card — is a display list too, drawn into a window-sized target, and §14.11.2.1 says
    /// nothing about it. [`Self::new`] therefore starts at `None` and `pdf_model::interpret` is
    /// what sets it.
    pub fn set_content_clip(&mut self, region: Rect) {
        self.content_clip = Some(region);
    }

    /// §14.11.2.1's boundary in this list's own space, or `None` for a list that is not a page.
    ///
    /// See [`Self::set_content_clip`]; [`crate::crop_area`] is this in a target's pixels.
    #[must_use]
    pub fn content_clip(&self) -> Option<Rect> {
        self.content_clip
    }

    /// States that this page composites in a four-component blending colour space.
    ///
    /// ISO 32000-2 §11.4.7: "[a]ll page-level compositing shall be done in the default
    /// blending colour space of the page". `black` is the same page drawn in the fourth
    /// component of that space, with identical geometry, shapes and opacities — see
    /// [`crate::blending`] for why two rasters answer four components, and
    /// [`DisplayList::geometry_digest`] for what a caller must check before calling this.
    ///
    /// A backend that cannot draw a page twice and put the halves together must refuse a list
    /// whose [`DisplayList::blending`] is `Some`, because the colours it holds are ink and not
    /// light: drawing the chromatic list alone would paint the page in the complements of
    /// cyan, magenta and yellow with no black at all.
    pub fn set_blending(&mut self, space: crate::blending::BlendingSpace, black: DisplayList) {
        self.blending = Some(space);
        self.black = Some(Box::new(black));
    }

    /// The blending colour space this page composites in, where it is not the device's.
    #[must_use]
    pub fn blending(&self) -> Option<&crate::blending::BlendingSpace> {
        self.blending.as_ref()
    }

    /// The companion list carrying the blending space's fourth component.
    #[must_use]
    pub fn black(&self) -> Option<&DisplayList> {
        self.black.as_deref()
    }

    /// A digest of everything about this list that is *not* a colour.
    ///
    /// The two lists [`DisplayList::set_blending`] pairs are two interpretations of one page
    /// that differ only in which components of the blending space each colour was resolved
    /// into, so their geometry, clips, masks, blend modes and nesting are identical by
    /// construction. This is what checks that they are: the halves are put back together per
    /// pixel, so a command present in one and absent from the other would be composited
    /// against a shape that never drew it, and no gate in this tree would see the result as
    /// anything but a wrong colour.
    ///
    /// Deliberately cheap and structural — variants, counts, transforms, identifiers and path
    /// lengths — rather than a second `PartialEq` with colours masked out: this runs on every
    /// page that states such a space, and what it is guarding against is a *structural*
    /// divergence rather than a numerical one.
    #[must_use]
    pub fn geometry_digest(&self) -> u64 {
        let mut hasher = std::hash::DefaultHasher::new();
        self.clips.len().hash(&mut hasher);
        self.soft_masks.len().hash(&mut hasher);
        Self::hash_commands(&self.commands, &mut hasher);
        hasher.finish()
    }

    /// [`DisplayList::geometry_digest`] over one level of commands, recursing into groups.
    fn hash_commands(commands: &[Command], hasher: &mut std::hash::DefaultHasher) {
        commands.len().hash(hasher);
        for command in commands {
            std::mem::discriminant(command).hash(hasher);
            command.clip().map(ClipId::index).hash(hasher);
            command.mask().map(SoftMaskId::index).hash(hasher);
            std::mem::discriminant(&command.blend()).hash(hasher);
            if let Some(path) = command.path() {
                path.commands().len().hash(hasher);
            }
            match command {
                Command::Group {
                    commands, blending, ..
                } => {
                    Self::hash_commands(commands, hasher);
                    // The pair's second list is part of the geometry too: a group whose
                    // halves diverged structurally would be resolved against a shape that
                    // never drew it, exactly the failure this digest exists to catch.
                    blending.is_some().hash(hasher);
                    if let Some(pair) = blending {
                        Self::hash_commands(&pair.black, hasher);
                    }
                }
                Command::Shaped { object, shape, .. } => {
                    Self::hash_commands(std::slice::from_ref(object.as_ref()), hasher);
                    Self::hash_commands(std::slice::from_ref(shape.as_ref()), hasher);
                }
                _ => {}
            }
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

    /// Returns how many clips the list holds.
    ///
    /// Exists for [`crate::Cell`], which repeats a passage of commands and has to tell a clip
    /// the passage built — and which therefore travels with it — from one that was already in
    /// force and is shared by every copy.
    #[must_use]
    pub fn clip_count(&self) -> usize {
        self.clips.len()
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
