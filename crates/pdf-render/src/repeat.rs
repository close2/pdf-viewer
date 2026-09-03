//! A mark two pattern cells state, drawn once.
//!
//! ISO 32000-2 §8.7.3.1 tiles an area with copies of one cell, and Table 74 clips each copy to
//! the cell's `/BBox`. A producer that wants a *continuous* rule out of a repeating figure draws
//! it on the box's edge and states it twice — once at the bottom of the cell and once at the top
//! — so that each copy contributes the half of the rule that falls inside its own box and the two
//! halves meet. `issue16038.pdf`'s `/pgfpat22` is exactly that figure.
//!
//! The halves meet in *geometry* and not on the raster. A clip mask is anti-aliased, so the pixel
//! the boundary runs through keeps a fraction `a` of one half and a fraction `b` of the other, and
//! two fractions painted one after another composite as `1 − (1−a)(1−b)` rather than adding: the
//! rule comes out 13% lighter than its own width says. §11.6.2 is what makes that wrong rather
//! than merely approximate:
//!
//! > Single graphics objects, as defined in 8.2, "Graphics objects", shall be treated as
//! > elementary objects for transparency compositing purposes … Portions of an object shall not be
//! > composited with one another, even if they are described in a way that would seem to cause
//! > overlaps
//!
//! The tiles are portions of one object: §11.6.7 makes a pattern's evaluation produce the shape
//! the *painted object* is then given — "the colour, shape, and opacity values resulting from the
//! evaluation of the pattern definition shall be used as the object's source colour (𝐶𝑠), object
//! shape (f j ), and object opacity (qi )" — so a tiling is evaluated to one shape before anything
//! composites at all. §11.6.7's NOTE 2 gives the same advice informally, and `pdf_model`'s `tile`
//! already follows it; the loss this module removes is *inside* that group.
//!
//! # What is folded
//!
//! One mark, not one buffer. Where a cell states a mark and states it again a whole number of
//! `/XStep`s and `/YStep`s away, the two are the *same* mark of the tiling: cell *k*'s second copy
//! lands exactly where cell *k+1*'s first does. Keeping one of them and taking the box clip off it
//! draws that mark once, whole, with one anti-aliased outline — the same set of points the clipped
//! pair covers, painted by one command instead of two. Nothing is added and nothing is snapped.
//!
//! [`repeated_subpaths`] states the conditions under which the two are provably the same set, and
//! [`without_subpaths`] carries the answer out.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::display_list::{ClipId, Command, DisplayList, DisplayListError};
use crate::geom::{Path, PathCommand, Point, Rect, Transform};
use crate::outline::stroked_bounds;
use crate::paint::Paint;
use crate::shading::Shading;
use crate::soft_mask::SoftMaskId;

/// Most subpaths a cell's command may have and still be searched for repeats.
///
/// The search is quadratic in the number of subpaths. It runs once per *pattern* rather than once
/// per tile, so the bound is generous rather than tight; it is still a deliberate one, with its
/// cost written down: a cell whose one command draws more than this many subpaths keeps its box
/// clip and the picture it has today, which is 13% light wherever the box halves a mark. The bound
/// is on the *search* and not on the rule.
const MAX_SUBPATHS: usize = 64;

/// Most lattice neighbours a cell's box may be checked against.
///
/// `/BBox` may be many steps across (Table 74's NOTE 2 permits it), and the marks that could
/// supply a folded one then live many cells away. The bound keeps the check finite; a pattern
/// past it is not folded.
const MAX_NEIGHBOURS: i32 = 64;

/// Where a tiling pattern puts its cells, in pattern space (ISO 32000-2 §8.7.3.1, Table 74).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tiles {
    /// `/XStep` and `/YStep`. Neither is zero; a negative one names the same lattice as its
    /// magnitude, so only the magnitudes are used.
    pub step: (f32, f32),
    /// `/BBox`, at the offset of the cell whose commands are being folded.
    pub cell: Rect,
}

/// Which marks of one command another cell also states, and the shape it was decided from.
///
/// Handed back by [`repeated_subpaths`] and applied by [`without_subpaths`]. It carries
/// `subpaths` so that a caller applying one cell's answer to another cell's commands can be
/// refused rather than cut by index into a path that is not the one the answer was about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repeats {
    /// How many subpaths the path had when this was decided.
    pub subpaths: usize,
    /// Which of them are a second statement of a mark, in increasing order. Never empty, and
    /// never all of them.
    pub dropped: Vec<usize>,
}

/// One subpath, with its points already carried into pattern space.
#[derive(Debug)]
struct Mapped {
    /// Where the subpath sits in the path's command list.
    range: core::ops::Range<usize>,
    /// One discriminant per path command, so that two subpaths of different shape are told
    /// apart before their points are compared.
    kinds: Vec<u8>,
    /// Every point the commands name, control points included, in pattern space.
    points: Vec<Point>,
}

/// Which of `command`'s subpaths are marks another cell also states, or `None` where there are
/// none or dropping them would not paint the same tiling.
///
/// `to_pattern` maps page space to the pattern space `tiles` is stated in. A `Some` answer is a
/// statement that the caller may draw the path **without** those subpaths and **without** the
/// cell's `/BBox` clip, and get the same set of points, painted once each, that the whole clipped
/// tiling would have. [`without_subpaths`] carries that out.
///
/// # It is asked of one cell and answered for all of them
///
/// Every cell states the same figure at a translation, so the answer is a property of the pattern.
/// Asking it once and applying the indices is not only cheaper: the comparison below runs on
/// points that a cell far from the pattern's origin holds at a magnitude where an `f32`'s own
/// neighbours are further apart than the tolerance, so re-deriving it per cell can fold some tiles
/// and not others — and half a folded tiling is a worse picture than none. The
/// three-hundred-and-seventy-fourth session watched that happen to **180 of 1296 tiles**, with the
/// page's ink barely moving, which is how it presented.
///
/// # What has to be true, and why each of it
///
/// Write *S* for the marks the cell's command states, *R* for what is left after folding, *B* for
/// the cell's box and *L* for the lattice `/XStep` and `/YStep` generate. The clipped tiling paints
/// `M = ⋃ᵥ ((S ∩ B) + v)`; drawing *R* unclipped at every site paints `P = ⋃ᵥ (R + v)`. Because
/// *B* + *L* covers the plane once, each point `p` has one `v` with `p − v ∈ B`, and `p ∈ M`
/// exactly when `p − v ∈ S`. So `P = M` follows from:
///
/// - **every subpath is a repeat or is one of the kept ones**, which folding gives by
///   construction; and
/// - **for every kept subpath `s` and every `u ∈ L` for which `s + u` reaches into `B`, the
///   command already states `s + u`**. Then any `p ∈ P` lies in `s + w`, its `p − v` lies in
///   `s + (w − v)` and in *B*, so `s + (w − v)` is stated and `p − v ∈ S`.
///
/// Two further conditions are about the raster rather than the set:
///
/// - **`R` fits inside one step in each axis**, so that no two sites' copies of it overlap and
///   "drawn once" is a fact rather than a hope; and
/// - **`R` stays within half a step of `B`**, so that the tile span a caller computed from the
///   *box* still reaches every site whose folded marks touch the painted area.
///
/// # A fill's winding survives it, and that is an argument rather than a check
///
/// §8.5.3.3.2's rules are stated over a whole path, so dropping one subpath of a fill could in
/// principle change where another's interior is. It cannot here, and the reason is the first of
/// the two raster conditions above. Every dropped subpath is `K + v` for a kept `K` and a nonzero
/// lattice `v`; every kept subpath lies inside one box of at most a step in each axis; so a
/// dropped subpath lies inside that box moved by `v`, which meets the box itself in no area at
/// all. A subpath that cannot reach a point cannot be deciding whether that point is inside, and
/// the points that are painted are the kept subpaths' own.
///
/// # What is not folded, deliberately
///
/// An image or a group: neither is a path, and neither is the shape a producer draws a continuous
/// rule out of. A path with a subpath that does not open with its own `m`: Table 58 lets a segment
/// after `h` continue from the last `m`, so removing commands would move a subpath that was left
/// in place.
///
/// # The one tolerance
///
/// Two marks are the same mark when their points agree after one displacement, and that
/// displacement is a lattice vector, both to within a ten-thousandth of a step. The comparison
/// runs in pattern space, which a caller reaches through an inverted matrix, so an exact test
/// would fail on rounding alone. What the tolerance admits is a fold that moves a mark by up to
/// 1e-4 of a tile — 0.0003 of a point on `issue16038.pdf`, which at this viewer's 6400% clamp is
/// two hundredths of a device pixel.
#[must_use]
pub fn repeated_subpaths(
    command: &Command,
    tiles: Tiles,
    to_pattern: Transform,
) -> Option<Repeats> {
    let (path, transform) = match command {
        Command::Fill {
            path, transform, ..
        }
        | Command::Stroke {
            path, transform, ..
        } => (path, *transform),
        // A shaped element states one mark twice, and folding half of it would leave the
        // two describing different regions — see `Command::path_mut`.
        Command::Image { .. } | Command::Group { .. } | Command::Shaped { .. } => return None,
    };
    let step = (tiles.step.0.abs(), tiles.step.1.abs());
    if !step.0.is_finite() || !step.1.is_finite() || step.0 <= 0.0 || step.1 <= 0.0 {
        return None;
    }
    let tolerance = step.0.max(step.1) * 1e-4;
    let to_pattern = transform.then(to_pattern);

    let ranges = subpath_ranges(path)?;
    if ranges.len() < 2 || ranges.len() > MAX_SUBPATHS {
        return None;
    }
    let subpaths: Vec<Mapped> = ranges
        .into_iter()
        .map(|range| mapped(path, range, to_pattern))
        .collect();

    let kept = keep_one_of_each(&subpaths, step, tolerance);
    if kept.len() == subpaths.len() {
        return None;
    }

    let folded = rebuild(path, &subpaths, &kept);
    let bounds = mark_bounds(command, &folded, to_pattern)?;
    // To within the same tolerance: a cell far from the pattern's origin states its rule at
    // coordinates whose f32 spacing is already a millionth of a step, so an exact comparison
    // here folds the tiles near the origin and not the ones further out.
    if bounds.width() > step.0 + tolerance || bounds.height() > step.1 + tolerance {
        return None;
    }
    let reachable = Rect::from_corners(
        Point::new(
            tiles.cell.min.x - step.0 / 2.0,
            tiles.cell.min.y - step.1 / 2.0,
        ),
        Point::new(
            tiles.cell.max.x + step.0 / 2.0,
            tiles.cell.max.y + step.1 / 2.0,
        ),
    );
    if !reachable.contains(bounds) {
        return None;
    }

    for &index in &kept {
        let subpath = subpaths.get(index)?;
        let single = rebuild(path, &subpaths, core::slice::from_ref(&index));
        let reach = mark_bounds(command, &single, to_pattern)?;
        if !neighbours_are_stated(subpath, reach, &subpaths, tiles.cell, step, tolerance)? {
            return None;
        }
    }
    Some(Repeats {
        subpaths: subpaths.len(),
        dropped: (0..subpaths.len())
            .filter(|index| !kept.contains(index))
            .collect(),
    })
}

/// `path` without the subpaths `repeats` names, or `None` where it is not the path that was
/// planned.
///
/// A path with a different number of subpaths is not the figure [`repeated_subpaths`] answered
/// about, and this refuses rather than deleting by index into something else.
#[must_use]
pub fn without_subpaths(path: &Path, repeats: &Repeats) -> Option<Path> {
    let ranges = subpath_ranges(path)?;
    if ranges.len() != repeats.subpaths
        || repeats.dropped.is_empty()
        || repeats.dropped.len() >= ranges.len()
    {
        return None;
    }
    let mut folded = Path::new();
    for (index, range) in ranges.into_iter().enumerate() {
        if repeats.dropped.contains(&index) {
            continue;
        }
        folded.extend(path.commands().get(range)?);
    }
    Some(folded)
}

/// Where a display list stood before a pattern cell was drawn.
///
/// Taken before the cell's content stream runs and handed to [`Cell::drawn`] afterwards, which
/// is what tells a soft mask the cell built — and which therefore travels with it — from one
/// that was already in force and is shared by every copy of the cell.
///
/// **It counted clips too until the six-hundred-and-twenty-fifth session, and a count cannot
/// answer that question about a clip.** [`DisplayList::add_clip`] hands back the identifier of an
/// equal clip already in the table, so a second tiling whose cell states the same box as a first
/// one's gets that first cell's identifier — which is *below* this mark although the cell built
/// it. See [`Displaced::is_the_cells_own`] for what asks instead, and why a soft mask is not
/// affected: [`DisplayList::add_soft_mask`] appends unconditionally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mark {
    /// Commands the list held.
    commands: usize,
    /// Soft masks it held.
    soft_masks: usize,
}

impl Mark {
    /// Where `list` stands now.
    #[must_use]
    pub fn of(list: &DisplayList) -> Self {
        Self {
            commands: list.command_count(),
            soft_masks: list.soft_mask_count(),
        }
    }
}

/// One pattern cell's marks, kept so that the sites after it are copies rather than re-readings.
///
/// # Why a tiling draws its cell once
///
/// ISO 32000-2 §8.7.3.1 defines a tiling pattern by one cell and a lattice: "[t]he pattern cell
/// shall be replicated at fixed horizontal and vertical intervals to fill the area to be
/// painted", and the cell's own appearance is "defined by a content stream containing the
/// painting operators needed to paint **one** instance of the cell". So the cell is one figure
/// and every site shows that same figure moved — which is the whole of why its commands may be
/// copied instead of its content stream re-interpreted.
///
/// Three things make that an equality rather than an approximation, and each is a property of
/// the interpretation rather than a hope about it:
///
/// - **The only input that differs between two sites is the transform**, and it differs by a
///   translation in pattern space. The content stream, the resource dictionary, the optional
///   content state and §8.7.3.3's tint are the pattern's, not the site's.
/// - **The cell's graphics state is initial at every site.** §11.6.7 requires it — "[t]he
///   definition shall not inherit the current values of the graphics state parameters at the
///   time it is evaluated" — so no site can be reached with a state another site left behind.
/// - **Every command carries its own absolute geometry** (see [`Command`]), so moving a
///   command is composing one transform, and a clip or a soft mask the cell built moves with
///   it because it is part of the same figure.
///
/// What a copy is *not* is bit-identical to a re-interpretation: `t.then(by)` adds the site's
/// displacement to a translation the cell's own matrices already accumulated, and the same sum
/// in a different order is the same number to within one `f32` rounding. That is the one
/// documented cost of this construction, and it is smaller than the tolerance
/// [`repeated_subpaths`] already works to.
#[derive(Debug, Clone)]
pub struct Cell {
    /// Where the list stood before the cell was drawn.
    at: Mark,
    /// The clip the cell was drawn *inside*, which bounds the whole tiling rather than the cell.
    ///
    /// It and its ancestors are exactly the clips in force when the cell began, so "is this clip
    /// the cell's own" is "is this clip none of those" — a question about the chain rather than
    /// about when an identifier was minted. `None` where the cell was given no clip at all, in
    /// which case every clip its commands name is one it built.
    base: Option<ClipId>,
    /// What it drew, taken once — a copy displaced is a copy of *this*, never of the list as it
    /// stands after the sites already placed. Getting that wrong doubles the tiling per site,
    /// which is why the template is owned here rather than re-read per repetition.
    commands: Vec<Command>,
}

impl Cell {
    /// The cell a display list has just drawn, since `at`, inside the clip `base`.
    #[must_use]
    pub fn drawn(list: &DisplayList, at: Mark, base: Option<ClipId>) -> Self {
        Self {
            at,
            base,
            commands: list
                .commands()
                .get(at.commands..)
                .unwrap_or_default()
                .to_vec(),
        }
    }

    /// How many commands the cell drew.
    #[must_use]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Whether the cell drew nothing at all, in which case repeating it is free.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Appends a copy of the cell, displaced by `by`.
    ///
    /// `by` is applied *after* each command's own transform, so it is stated in the space the
    /// commands were drawn into — page space — and for a tiling it is the lattice step carried
    /// through the pattern matrix. Returns how many commands were appended, which is what a
    /// caller charges to its own budget: the copies are the work.
    ///
    /// A clip or a soft mask the cell built is copied too, displaced the same way, because it
    /// is part of the figure being moved; one that was already in force is referred to
    /// unchanged, because it bounds the tiling rather than the cell.
    ///
    /// # Errors
    ///
    /// [`DisplayListError`] where the copies would exceed what a [`ClipId`] or a
    /// [`SoftMaskId`] can address. The commands already appended stay: a tiling that runs out
    /// of clips has painted the sites it painted, and the caller reports the limit.
    pub fn repeat(&self, list: &mut DisplayList, by: Transform) -> Result<usize, DisplayListError> {
        let mut copies = self.commands.clone();
        {
            let mut displaced = Displaced {
                list,
                by,
                cell: self.at,
                base: self.base,
                clips: BTreeMap::new(),
                masks: BTreeMap::new(),
            };
            displaced.commands(&mut copies)?;
        }
        let count = copies.len();
        for command in copies {
            list.push(command);
        }
        Ok(count)
    }
}

/// Carries [`Cell::repeat`] out: one displacement, applied to commands and to what they name.
struct Displaced<'a> {
    /// The list the copies are being added to, and the table the cell's clips and masks are in.
    list: &'a mut DisplayList,
    /// Applied after each command's own transform.
    by: Transform,
    /// Which soft masks are the cell's own; see [`Mark`].
    cell: Mark,
    /// The clip the tiling was given, which decides which clips are the cell's own.
    base: Option<ClipId>,
    /// The displaced copy of each of the cell's clips, so a chain is copied once per repetition
    /// however many commands name it.
    clips: BTreeMap<usize, ClipId>,
    /// The same for soft masks.
    masks: BTreeMap<usize, SoftMaskId>,
}

impl Displaced<'_> {
    /// Displaces a sequence of commands in place.
    fn commands(&mut self, commands: &mut [Command]) -> Result<(), DisplayListError> {
        for command in commands {
            self.command(command)?;
        }
        Ok(())
    }

    /// Displaces one command in place, with whatever it names.
    ///
    /// The match is exhaustive on purpose: a command variant added later that carries geometry
    /// is a variant this has to be taught, and the compiler is the only thing that will say so.
    fn command(&mut self, command: &mut Command) -> Result<(), DisplayListError> {
        match command {
            Command::Fill {
                transform,
                paint,
                clip,
                mask,
                ..
            }
            | Command::Stroke {
                transform,
                paint,
                clip,
                mask,
                ..
            } => {
                *transform = transform.then(self.by);
                // A shading carries a transform of its own — §8.7.4.3's pattern matrix rather
                // than the filled path's — so the colours have to move with the geometry or
                // every site would show the first site's gradient.
                if let Paint::Shading(shading) = paint {
                    *shading = Arc::new(Shading {
                        kind: Arc::clone(&shading.kind),
                        transform: shading.transform.then(self.by),
                        // Table 77's wash is a *colour*, so the lattice step moves the
                        // geometry that decides where it applies and never the colour itself.
                        background: shading.background,
                    });
                }
                self.refer(clip, mask)?;
            }
            Command::Image {
                transform,
                clip,
                mask,
                ..
            } => {
                // The samples are on the grid the file states and the unit square is placed by
                // the transform alone, so a displaced image needs no new raster.
                *transform = transform.then(self.by);
                self.refer(clip, mask)?;
            }
            Command::Group {
                commands,
                clip,
                mask,
                blending,
                ..
            } => {
                self.commands(commands)?;
                if let Some(crate::GroupBlending::FourComponents { black, .. }) =
                    blending.as_deref_mut()
                {
                    // §11.7.2's second list is the same elements in the black component; it is
                    // geometry too, and the two halves are composited per pixel.
                    self.commands(black)?;
                }
                self.refer(clip, mask)?;
            }
            Command::Shaped { object, shape } => {
                self.command(object)?;
                self.command(shape)?;
            }
        }
        Ok(())
    }

    /// Points a command's clip and soft mask at the copies that belong to this site.
    fn refer(
        &mut self,
        clip: &mut Option<ClipId>,
        mask: &mut Option<SoftMaskId>,
    ) -> Result<(), DisplayListError> {
        if let Some(id) = *clip {
            *clip = Some(self.clip(id)?);
        }
        if let Some(id) = *mask {
            *mask = Some(self.mask(id)?);
        }
        Ok(())
    }

    /// Whether `id` names a clip the cell built, rather than one that bounds the whole tiling.
    ///
    /// **The question is asked of the clips that were in force, because those are the ones that
    /// can be enumerated.** A tiling is given one clip and the cell's graphics state begins with
    /// it, so what bounds the whole tiling is that clip and its ancestors — a short, closed chain
    /// — and every other clip a cell's commands name is one the cell put there. `None` where the
    /// tiling was given no clip at all, in which case there is nothing in force to share.
    ///
    /// **The alternative was an identifier's position, and a table that interns cannot be asked
    /// that way.** [`DisplayList::add_clip`] hands back the identifier of an equal clip already
    /// in the table, so a cell that states the same box a previous cell stated is handed the
    /// previous cell's identifier — one minted before this cell began — and looked, by position,
    /// like a clip that was already in force. What that cost is a page: `4113230.pdf` of the
    /// `SafeDocs` crawl fills one path with two tiling patterns in turn, each a full-bleed
    /// photograph, and the second one's every site kept the *first* pattern's first-site box —
    /// which is off the page, so the second photograph vanished and the first stayed visible
    /// under it (session 625).
    ///
    /// **Asking it the other way round — "does this clip descend from the one in force" — is not
    /// the same question and is too narrow.** A cell may build a clip whose chain is rooted
    /// somewhere else entirely: a soft mask's group is interpreted in a clip context of its own,
    /// and its commands are displaced with the cell. `issue8565.pdf` is the corpus page that says
    /// so, one radial gradient under a cell the size of the page.
    ///
    /// The walk terminates because a clip's parent is added to the table before the clip itself
    /// and therefore has a strictly smaller identifier.
    fn is_the_cells_own(&self, id: ClipId) -> bool {
        let mut at = self.base;
        while let Some(current) = at {
            if current == id {
                return false;
            }
            at = self.list.clip(current).and_then(|clip| clip.parent);
        }
        true
    }

    /// The displaced copy of one clip chain, built outermost first.
    ///
    /// The walk terminates because a clip's parent is added to the list before the clip itself
    /// and therefore has a smaller identifier; the chain is bounded by the clips the cell built.
    fn clip(&mut self, id: ClipId) -> Result<ClipId, DisplayListError> {
        let mut chain: Vec<ClipId> = Vec::new();
        let mut at = id;
        loop {
            if !self.is_the_cells_own(at) || self.clips.contains_key(&at.index()) {
                break;
            }
            chain.push(at);
            match self.list.clip(at).and_then(|clip| clip.parent) {
                Some(parent) => at = parent,
                None => break,
            }
        }
        for original in chain.into_iter().rev() {
            let Some(mut clip) = self.list.clip(original).cloned() else {
                continue;
            };
            clip.transform = clip.transform.then(self.by);
            clip.parent = clip.parent.map(|parent| self.mapped_clip(parent));
            let copy = self.list.add_clip(clip)?;
            self.clips.insert(original.index(), copy);
        }
        Ok(self.mapped_clip(id))
    }

    /// The copy of a clip this site uses, which is the clip itself where it is not the cell's.
    fn mapped_clip(&self, id: ClipId) -> ClipId {
        self.clips.get(&id.index()).copied().unwrap_or(id)
    }

    /// The displaced copy of one soft mask, whose group is displaced with it.
    ///
    /// A mask's elements may name a mask of their own; those were added to the list first, so
    /// the recursion is as deep as the mask nesting the interpreter allowed and no deeper.
    fn mask(&mut self, id: SoftMaskId) -> Result<SoftMaskId, DisplayListError> {
        if id.index() < self.cell.soft_masks {
            return Ok(id);
        }
        if let Some(copy) = self.masks.get(&id.index()) {
            return Ok(*copy);
        }
        let Some(mut mask) = self.list.soft_mask(id).cloned() else {
            return Ok(id);
        };
        self.commands(&mut mask.commands)?;
        let copy = self.list.add_soft_mask(mask)?;
        self.masks.insert(id.index(), copy);
        Ok(copy)
    }
}

/// Which subpaths survive: the first of each set that are lattice translations of one another.
///
/// The choice of representative changes nothing — every member of a set paints the same points
/// once the tiling is laid down — so it is the first in painting order, which is stable.
fn keep_one_of_each(subpaths: &[Mapped], step: (f32, f32), tolerance: f32) -> Vec<usize> {
    let mut kept: Vec<usize> = Vec::new();
    'next: for (index, subpath) in subpaths.iter().enumerate() {
        for &earlier in &kept {
            let Some(other) = subpaths.get(earlier) else {
                continue;
            };
            if let Some(shift) = displacement(other, subpath, tolerance)
                && is_lattice(shift, step, tolerance)
            {
                continue 'next;
            }
        }
        kept.push(index);
    }
    kept
}

/// Whether every lattice copy of `subpath` that reaches into the cell's box is itself stated.
///
/// `reach` is the region the subpath's *mark* covers — a stroke's outline rather than its centre
/// line — because that is what has to land inside the box for the argument in [`repeated_subpaths`] to
/// need it. `None` where the box is so many steps across that the neighbour count passes
/// [`MAX_NEIGHBOURS`], which is refused rather than truncated.
fn neighbours_are_stated(
    subpath: &Mapped,
    reach: Rect,
    subpaths: &[Mapped],
    cell: Rect,
    step: (f32, f32),
    tolerance: f32,
) -> Option<bool> {
    let (first_x, last_x) = neighbour_range(
        (reach.min.x, reach.max.x),
        (cell.min.x, cell.max.x),
        step.0,
        tolerance,
    )?;
    let (first_y, last_y) = neighbour_range(
        (reach.min.y, reach.max.y),
        (cell.min.y, cell.max.y),
        step.1,
        tolerance,
    )?;
    for across in first_x..=last_x {
        for down in first_y..=last_y {
            if across == 0 && down == 0 {
                continue;
            }
            let shift = (index_as_f32(across) * step.0, index_as_f32(down) * step.1);
            if !subpaths.iter().any(|other| {
                displacement(subpath, other, tolerance)
                    .is_some_and(|found| close(found, shift, tolerance))
            }) {
                return Some(false);
            }
        }
    }
    Some(true)
}

/// The lattice indices whose copy of `mark` overlaps `cell` by more than `tolerance`.
///
/// Touching counts as no overlap: two marks that share an edge intersect in a set of no area,
/// which no pixel's coverage can see, and counting it would refuse the very figure this module
/// exists for — a rule spanning its cell exactly. The tolerance is what makes "share an edge"
/// survive arithmetic: a tile far from the pattern's origin states that edge at a coordinate
/// whose `f32` neighbours are already a millionth of a step apart, and an exact test would then
/// demand a copy of the mark one cell over. What it gives up is an overlap narrower than a
/// ten-thousandth of a tile, which no device this viewer drives can resolve.
fn neighbour_range(
    mark: (f32, f32),
    cell: (f32, f32),
    step: f32,
    tolerance: f32,
) -> Option<(i32, i32)> {
    let ((low, high), (cell_low, cell_high)) = (mark, cell);
    // Strictly inside, on both sides: the smallest index whose copy starts before `cell_high`
    // and the largest whose copy ends after `cell_low`.
    let first = ((cell_low - high + tolerance) / step).floor() + 1.0;
    let last = ((cell_high - low - tolerance) / step).ceil() - 1.0;
    if !first.is_finite() || !last.is_finite() || last - first > f32::from(u8::MAX) {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "both are finite and within 255 of each other, checked above"
    )]
    let (first, last) = (first as i32, last as i32);
    if first < -MAX_NEIGHBOURS || last > MAX_NEIGHBOURS {
        return None;
    }
    Some((first, last))
}

/// The one displacement carrying `from` onto `to`, or `None` where none does.
///
/// Taken from the first point of each and then checked against every other, so that two subpaths
/// of the same shape at the same start but diverging afterwards are not mistaken for one figure.
fn displacement(from: &Mapped, to: &Mapped, tolerance: f32) -> Option<(f32, f32)> {
    if from.kinds != to.kinds || from.points.len() != to.points.len() {
        return None;
    }
    let start = from.points.first()?;
    let end = to.points.first()?;
    let shift = (end.x - start.x, end.y - start.y);
    for (start, end) in from.points.iter().zip(&to.points) {
        if !close((end.x - start.x, end.y - start.y), shift, tolerance) {
            return None;
        }
    }
    Some(shift)
}

/// Whether two displacements agree to within `tolerance` in both axes.
fn close(found: (f32, f32), wanted: (f32, f32), tolerance: f32) -> bool {
    (found.0 - wanted.0).abs() <= tolerance && (found.1 - wanted.1).abs() <= tolerance
}

/// Whether a displacement is a whole number of steps in each axis, and not the zero one.
fn is_lattice(shift: (f32, f32), step: (f32, f32), tolerance: f32) -> bool {
    let across = (shift.0 / step.0).round();
    let down = (shift.1 / step.1).round();
    if across == 0.0 && down == 0.0 {
        return false;
    }
    close(shift, (across * step.0, down * step.1), tolerance)
}

/// Widens a lattice index for arithmetic in pattern space.
fn index_as_f32(index: i32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "indices are bounded by MAX_NEIGHBOURS, exact in f32"
    )]
    {
        index as f32
    }
}

/// A path holding only the subpaths named by `kept`, in their original order.
fn rebuild(path: &Path, subpaths: &[Mapped], kept: &[usize]) -> Path {
    let mut folded = Path::new();
    for &index in kept {
        if let Some(subpath) = subpaths.get(index)
            && let Some(commands) = path.commands().get(subpath.range.clone())
        {
            folded.extend(commands);
        }
    }
    folded
}

/// The region a command marks when it draws `path`, in the space `to_pattern` maps into.
///
/// A stroke's mark is its outline, which is what has to sit inside a box for the box to be
/// removable; every other command marks the path itself. Only fills and strokes reach here —
/// [`repeated_subpaths`] refuses the rest before asking.
fn mark_bounds(command: &Command, path: &Path, to_pattern: Transform) -> Option<Rect> {
    match command {
        Command::Stroke { stroke, .. } => stroked_bounds(path, stroke, to_pattern),
        Command::Fill { .. }
        | Command::Image { .. }
        | Command::Group { .. }
        | Command::Shaped { .. } => path.bounds(to_pattern),
    }
}

/// One subpath's commands, with every point it names carried into pattern space.
fn mapped(path: &Path, range: core::ops::Range<usize>, to_pattern: Transform) -> Mapped {
    let mut kinds = Vec::new();
    let mut points = Vec::new();
    for command in path.commands().get(range.clone()).unwrap_or_default() {
        match *command {
            PathCommand::MoveTo(p) => {
                kinds.push(0);
                points.push(to_pattern.apply(p));
            }
            PathCommand::LineTo(p) => {
                kinds.push(1);
                points.push(to_pattern.apply(p));
            }
            PathCommand::CurveTo(a, b, at) => {
                kinds.push(2);
                points.push(to_pattern.apply(a));
                points.push(to_pattern.apply(b));
                points.push(to_pattern.apply(at));
            }
            PathCommand::Close => kinds.push(3),
        }
    }
    Mapped {
        range,
        kinds,
        points,
    }
}

/// Where each subpath sits in a path's command list, or `None` where one of them cannot be
/// removed on its own.
///
/// A subpath runs from its `m` to the next one or to the `h` that ends it, which is
/// [`crate::collapsed`]'s convention and Table 58's. A subpath opened by a segment rather than by
/// an `m` continues from the last `m` there was, so its meaning depends on commands outside its
/// own range: this refuses the whole path rather than moving such a subpath by deleting another.
fn subpath_ranges(path: &Path) -> Option<Vec<core::ops::Range<usize>>> {
    let commands = path.commands();
    let mut ranges = Vec::new();
    let mut index = 0usize;
    while index < commands.len() {
        if !matches!(commands.get(index), Some(PathCommand::MoveTo(_))) {
            return None;
        }
        let first = index;
        index = index.saturating_add(1);
        while let Some(command) = commands.get(index) {
            match command {
                PathCommand::MoveTo(_) => break,
                PathCommand::Close => {
                    index = index.saturating_add(1);
                    break;
                }
                PathCommand::LineTo(_) | PathCommand::CurveTo(..) => {
                    index = index.saturating_add(1);
                }
            }
        }
        ranges.push(first..index);
    }
    Some(ranges)
}

#[cfg(test)]
mod tests {
    use super::{Repeats, Tiles, repeated_subpaths, without_subpaths};
    use crate::display_list::Command;
    use crate::geom::{Path, PathCommand, Point, Rect, Transform};
    use crate::paint::{BlendMode, Color, FillRule, LineCap, LineJoin, Paint, Stroke};
    use std::sync::Arc;

    fn rule(from: Point, to: Point) -> [PathCommand; 2] {
        [PathCommand::MoveTo(from), PathCommand::LineTo(to)]
    }

    fn square(min: Point, max: Point) -> [PathCommand; 5] {
        [
            PathCommand::MoveTo(min),
            PathCommand::LineTo(Point::new(max.x, min.y)),
            PathCommand::LineTo(max),
            PathCommand::LineTo(Point::new(min.x, max.y)),
            PathCommand::Close,
        ]
    }

    fn stroke_of(path: Path) -> Command {
        Command::Stroke {
            path: Arc::new(path),
            transform: Transform::IDENTITY,
            stroke: Stroke {
                width: 0.4,
                adjust: false,
                cap: LineCap::Butt,
                join: LineJoin::Miter,
                miter_limit: 10.0,
                dash_array: Vec::new(),
                dash_phase: 0.0,
            },
            paint: Paint::Solid(Color::BLACK),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        }
    }

    fn unit_cell() -> Tiles {
        Tiles {
            step: (3.0, 3.0),
            cell: Rect::from_corners(Point::new(0.0, 0.0), Point::new(3.0, 3.0)),
        }
    }

    /// `issue16038.pdf`'s `/pgfpat22`, reduced to its two rules.
    fn cell_with_a_rule_on_each_edge() -> Path {
        let mut path = Path::new();
        path.extend(&rule(Point::new(0.0, 0.0), Point::new(3.0, 0.0)));
        path.extend(&rule(Point::new(0.0, 3.0), Point::new(3.0, 3.0)));
        path
    }

    #[test]
    fn a_rule_stated_at_both_edges_of_the_cell_is_one_mark() {
        let command = stroke_of(cell_with_a_rule_on_each_edge());
        let repeats = repeated_subpaths(&command, unit_cell(), Transform::IDENTITY)
            .expect("the top rule is the bottom one a step away");
        assert_eq!(repeats.dropped, vec![1], "the second statement of the rule");
        let folded = without_subpaths(&cell_with_a_rule_on_each_edge(), &repeats)
            .expect("one of the two subpaths survives");
        assert_eq!(
            folded.commands(),
            &rule(Point::new(0.0, 0.0), Point::new(3.0, 0.0)),
            "the first of the two survives whole"
        );
    }

    /// A path that is not the one the answer was about is refused rather than cut by index.
    #[test]
    fn a_different_path_does_not_take_another_ones_answer() {
        let two = Repeats {
            subpaths: 2,
            dropped: vec![1],
        };
        let mut one_rule = Path::new();
        one_rule.extend(&rule(Point::new(0.0, 0.0), Point::new(3.0, 0.0)));
        assert!(
            without_subpaths(&one_rule, &two).is_none(),
            "one subpath is not the two the answer counted"
        );
        let all = Repeats {
            subpaths: 2,
            dropped: vec![0, 1],
        };
        assert!(
            without_subpaths(&cell_with_a_rule_on_each_edge(), &all).is_none(),
            "dropping every subpath is not a fold"
        );
    }

    #[test]
    fn a_cell_whose_marks_are_unrelated_is_left_alone() {
        let mut path = Path::new();
        path.extend(&rule(Point::new(0.0, 0.0), Point::new(3.0, 0.0)));
        path.extend(&rule(Point::new(0.0, 1.7), Point::new(3.0, 1.7)));
        assert!(
            repeated_subpaths(&stroke_of(path), unit_cell(), Transform::IDENTITY).is_none(),
            "1.7 is not a whole number of steps"
        );
    }

    /// The condition the argument turns on: the copy that lands in the box has to be stated.
    ///
    /// Here the cell states a rule on its bottom edge and a *third* one two steps up, so folding
    /// the pair would leave the box's own top edge with nothing to supply the half a neighbour
    /// cell's clip removes.
    #[test]
    fn a_repeat_two_steps_away_with_nothing_between_is_refused() {
        let mut path = Path::new();
        path.extend(&rule(Point::new(0.0, 0.0), Point::new(3.0, 0.0)));
        path.extend(&rule(Point::new(0.0, 6.0), Point::new(3.0, 6.0)));
        assert!(
            repeated_subpaths(&stroke_of(path), unit_cell(), Transform::IDENTITY).is_none(),
            "the copy at one step is not stated, so the fold would lose it"
        );
    }

    #[test]
    fn a_figure_wider_than_a_step_is_refused() {
        let mut path = Path::new();
        path.extend(&rule(Point::new(-2.0, 0.0), Point::new(5.0, 0.0)));
        path.extend(&rule(Point::new(-2.0, 3.0), Point::new(5.0, 3.0)));
        assert!(
            repeated_subpaths(&stroke_of(path), unit_cell(), Transform::IDENTITY).is_none(),
            "seven units of rule on a three-unit step would overlap its own copies"
        );
    }

    /// A fill folds too, and an even-odd hole travels with the subpath that owns it.
    ///
    /// The cell states a square with a hole in it at the bottom edge and the same figure a step
    /// up. Folding keeps the lower pair whole — square *and* hole — because the hole is not a
    /// lattice repeat of the square, so the winding that makes it a hole is untouched.
    #[test]
    fn a_fill_folds_and_keeps_the_subpath_that_makes_its_hole() {
        let mut path = Path::new();
        for at in [0.0_f32, 3.0] {
            path.extend(&square(
                Point::new(0.0, at - 0.5),
                Point::new(2.0, at + 0.5),
            ));
            path.extend(&square(
                Point::new(0.5, at - 0.2),
                Point::new(1.5, at + 0.2),
            ));
        }
        let command = Command::Fill {
            path: Arc::new(path),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::EvenOdd,
            paint: Paint::Solid(Color::BLACK),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        };
        let repeats = repeated_subpaths(&command, unit_cell(), Transform::IDENTITY)
            .expect("both squares repeat a step up");
        assert_eq!(
            repeats.dropped,
            vec![2, 3],
            "the upper square and the hole that belongs to it"
        );
    }

    /// The same figure through the matrix a real pattern arrives with, rather than the identity.
    #[test]
    fn the_fold_survives_the_matrix_the_cell_is_placed_by() {
        let to_page = Transform::new(2.0, 0.0, 0.0, 2.0, 17.0, -4.0);
        let to_pattern = to_page.invert().expect("a scale and a translation invert");
        let mut path = Path::new();
        path.extend(&rule(Point::new(0.0, 0.0), Point::new(3.0, 0.0)));
        path.extend(&rule(Point::new(0.0, 3.0), Point::new(3.0, 3.0)));
        let mut command = stroke_of(path);
        if let Command::Stroke { transform, .. } = &mut command {
            *transform = to_page;
        }
        let repeats = repeated_subpaths(&command, unit_cell(), to_pattern)
            .expect("the lattice is the pattern's, whatever the cell is placed by");
        assert_eq!(repeats.dropped, vec![1], "the lattice is the pattern's");
    }
}

#[cfg(test)]
mod repetition {
    use super::{Cell, Mark};
    use crate::display_list::{Clip, Command, DisplayList};
    use crate::geom::{Path, PathCommand, Point, Size, Transform};
    use crate::paint::{BlendMode, Color, FillRule, Paint};
    use crate::shading::{Ramp, Shading, ShadingKind, Stop};
    use crate::soft_mask::{SoftMask, SoftMaskKind};
    use std::sync::Arc;

    /// A closed rectangle, which is every figure these fixtures need.
    fn square(min: Point, max: Point) -> Path {
        let mut path = Path::new();
        path.extend(&[
            PathCommand::MoveTo(min),
            PathCommand::LineTo(Point::new(max.x, min.y)),
            PathCommand::LineTo(max),
            PathCommand::LineTo(Point::new(min.x, max.y)),
            PathCommand::Close,
        ]);
        path
    }

    /// A black fill of `path`, unclipped and unmasked.
    fn fill_of(path: Path) -> Command {
        Command::Fill {
            path: Arc::new(path),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(Color::BLACK),
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        }
    }

    /// A list with a clip already in force, which is the tiling's own and not the cell's.
    fn list_with_an_outer_clip() -> (DisplayList, crate::display_list::ClipId) {
        let mut list = DisplayList::new(Size {
            width: 100.0,
            height: 100.0,
        });
        let clip = list
            .add_clip(Clip {
                path: square(Point::new(0.0, 0.0), Point::new(100.0, 100.0)),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                parent: None,
            })
            .expect("the first clip");
        (list, clip)
    }

    /// A gradient, whose own transform has to move with the marks it paints.
    fn gradient() -> Shading {
        Shading {
            background: None,
            kind: Arc::new(ShadingKind::Axial {
                start: Point::new(0.0, 0.0),
                end: Point::new(1.0, 0.0),
                extend: (false, false),
                ramp: Ramp {
                    stops: Arc::from(vec![
                        Stop {
                            at: 0.0,
                            colour: Color::BLACK,
                        },
                        Stop {
                            at: 1.0,
                            colour: Color::WHITE,
                        },
                    ]),
                },
            }),
            transform: Transform::IDENTITY,
        }
    }

    /// The whole of what a copy is: the marks, the cell's own clip and its gradient, all moved,
    /// and the clip that was already in force shared rather than duplicated.
    #[test]
    fn a_copy_moves_the_cells_marks_its_clip_and_its_gradient_and_shares_what_was_in_force() {
        let (mut list, outer) = list_with_an_outer_clip();
        let at = Mark::of(&list);

        let cell_clip = list
            .add_clip(Clip {
                path: square(Point::new(0.0, 0.0), Point::new(3.0, 3.0)),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                parent: Some(outer),
            })
            .expect("the cell's own clip");
        list.push(Command::Fill {
            path: Arc::new(square(Point::new(0.0, 0.0), Point::new(2.0, 2.0))),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Shading(Arc::new(gradient())),
            clip: Some(cell_clip),
            mask: None,
            blend: BlendMode::Normal,
        });

        let cell = Cell::drawn(&list, at, Some(outer));
        assert_eq!(cell.len(), 1, "the cell drew one command");
        let copied = cell
            .repeat(&mut list, Transform::translate(3.0, 0.0))
            .expect("a copy fits");
        assert_eq!(copied, 1, "and the copy is one command too");

        let commands = list.commands();
        let Some(Command::Fill {
            transform,
            paint,
            clip,
            ..
        }) = commands.get(1)
        else {
            panic!("the copy is a fill");
        };
        assert!(
            (transform.e - 3.0).abs() < 1e-6 && transform.f.abs() < 1e-6,
            "the copy's marks moved by the displacement: {transform:?}"
        );
        let Paint::Shading(shading) = paint else {
            panic!("the copy keeps its gradient");
        };
        assert!(
            (shading.transform.e - 3.0).abs() < 1e-6,
            "and the gradient moved with them: {:?}",
            shading.transform
        );
        let copy_clip = clip.expect("the copy is clipped");
        assert_ne!(copy_clip, cell_clip, "by a clip of its own");
        let copied_clip = list.clip(copy_clip).expect("it is in the table");
        assert!(
            (copied_clip.transform.e - 3.0).abs() < 1e-6,
            "which is the cell's box moved: {:?}",
            copied_clip.transform
        );
        assert_eq!(
            copied_clip.parent,
            Some(outer),
            "and whose parent is the clip that was already in force, shared rather than copied"
        );
    }

    /// A soft mask the cell built is a mask of its own at every site, moved with it.
    ///
    /// §11.6.5.1 positions a mask's group by the transform in force when `gs` set it, so a mask
    /// left where the first site drew it would show that site's shape at every other one.
    #[test]
    fn a_copy_moves_a_soft_mask_the_cell_built() {
        let (mut list, _) = list_with_an_outer_clip();
        let at = Mark::of(&list);

        let mask = list
            .add_soft_mask(SoftMask {
                commands: vec![fill_of(square(Point::new(0.0, 0.0), Point::new(2.0, 2.0)))],
                kind: SoftMaskKind::Alpha,
                transfer: None,
                luminance: None,
            })
            .expect("the cell's own mask");
        list.push(Command::Fill {
            path: Arc::new(square(Point::new(0.0, 0.0), Point::new(2.0, 2.0))),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(Color::BLACK),
            clip: None,
            mask: Some(mask),
            blend: BlendMode::Normal,
        });

        let cell = Cell::drawn(&list, at, None);
        cell.repeat(&mut list, Transform::translate(0.0, 5.0))
            .expect("a copy fits");

        let copy = list.commands().get(1).expect("the copy");
        let copied = copy.mask().expect("it keeps a mask");
        assert_ne!(copied, mask, "a mask of its own");
        let group = list.soft_mask(copied).expect("it is in the table");
        let placed = group
            .commands
            .first()
            .and_then(|command| match command {
                Command::Fill { transform, .. } => Some(*transform),
                _ => None,
            })
            .expect("the mask's group draws");
        assert!(
            (placed.f - 5.0).abs() < 1e-6,
            "whose group moved with the marks it masks: {placed:?}"
        );
    }

    /// An empty cell repeats to nothing, which is what a bounded loop over it costs.
    #[test]
    fn an_empty_cell_copies_nothing() {
        let (mut list, _) = list_with_an_outer_clip();
        let at = Mark::of(&list);
        let cell = Cell::drawn(&list, at, None);
        assert!(cell.is_empty(), "the cell drew nothing");
        assert_eq!(
            cell.repeat(&mut list, Transform::translate(1.0, 1.0))
                .expect("nothing cannot overflow"),
            0
        );
        assert_eq!(list.commands().len(), 0, "and nothing was appended");
    }

    /// A second tiling whose cell states the same box as the first still gets a clip per site.
    ///
    /// [`DisplayList::add_clip`] hands back the identifier of an equal clip already in the table,
    /// so the second cell's box arrives with the *first* cell's identifier — one minted before
    /// this cell's [`Mark`] was taken. Deciding provenance by position therefore called it a clip
    /// that was already in force, left it where the first cell's first site had put it, and gave
    /// every site of the second tiling that one box.
    ///
    /// `4113230.pdf` of the `SafeDocs` crawl is what that draws (session 625): a title page filling
    /// one path with two full-bleed photographs in turn, whose second photograph disappeared
    /// because the box it was clipped to belonged to a site off the top of the page.
    #[test]
    fn a_second_cell_stating_the_first_cells_box_still_moves_it() {
        let (mut list, outer) = list_with_an_outer_clip();
        let box_of_the_cell = || Clip {
            path: square(Point::new(0.0, 0.0), Point::new(3.0, 3.0)),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            parent: Some(outer),
        };

        let first_at = Mark::of(&list);
        let first_box = list
            .add_clip(box_of_the_cell())
            .expect("the first cell's box");
        list.push(clipped_fill(first_box));
        Cell::drawn(&list, first_at, Some(outer))
            .repeat(&mut list, Transform::translate(3.0, 0.0))
            .expect("a copy fits");

        let second_at = Mark::of(&list);
        let second_box = list
            .add_clip(box_of_the_cell())
            .expect("the second cell's box");
        assert_eq!(
            second_box, first_box,
            "the fixture is only a fixture if the table interned it"
        );
        list.push(clipped_fill(second_box));
        Cell::drawn(&list, second_at, Some(outer))
            .repeat(&mut list, Transform::translate(3.0, 0.0))
            .expect("a copy fits");

        let copy = list.commands().last().expect("the second tiling's copy");
        let copied = copy.clip().expect("the copy is clipped");
        assert_ne!(
            copied, second_box,
            "the second cell's box is the cell's own and moves with it"
        );
        let moved = list.clip(copied).expect("it is in the table");
        assert!(
            (moved.transform.e - 3.0).abs() < 1e-6,
            "by the site's displacement: {:?}",
            moved.transform
        );
    }

    /// A black fill of a small square under `clip`.
    fn clipped_fill(clip: crate::display_list::ClipId) -> Command {
        Command::Fill {
            path: Arc::new(square(Point::new(0.0, 0.0), Point::new(2.0, 2.0))),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(Color::BLACK),
            clip: Some(clip),
            mask: None,
            blend: BlendMode::Normal,
        }
    }

    /// Two copies of one cell are two copies of the *cell*, not of the list as it stands.
    ///
    /// The failure this pins is the one the construction invites: read the template off the
    /// list at each repetition and the tiling doubles per site, which the budget then refuses
    /// as a page too large to draw.
    #[test]
    fn a_second_copy_is_a_copy_of_the_cell_rather_than_of_what_came_before() {
        let (mut list, _) = list_with_an_outer_clip();
        let at = Mark::of(&list);
        list.push(fill_of(square(Point::new(0.0, 0.0), Point::new(1.0, 1.0))));
        let cell = Cell::drawn(&list, at, None);

        for site in [1.0, 2.0, 3.0, 4.0] {
            let copied = cell
                .repeat(&mut list, Transform::translate(site, 0.0))
                .expect("a copy fits");
            assert_eq!(copied, 1, "each site copies the one command the cell drew");
        }
        assert_eq!(list.commands().len(), 5, "one cell and four copies");
    }
}
