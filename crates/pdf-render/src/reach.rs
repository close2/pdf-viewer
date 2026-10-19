//! Where each of a list's commands can mark the device, measured once per magnification.
//!
//! A frame that shows part of a page owes nothing to a mark outside it, and a strip of a frame
//! owes nothing to a mark above or below it. Both questions are asked of
//! [`Command::device_bounds`](crate::Command::device_bounds), the conservative bound every backend in this tree already culls
//! by, and both were answered by measuring it again wherever it was asked: each strip of a page
//! replayed every command's bound (ADR 0137), which was 9.9 % of the processor's drawing of the
//! owner's witness after ADR 1829, and a window at four times the fit drew all 4.4 M of its
//! marks where 514 527 reach it (ADR 1817 §3).
//!
//! [`MarkBounds`] is the answer kept: one bound per top-level command, measured under one
//! transform and kept with the list ([`DisplayList::mark_bounds`]). It serves every transform of
//! the same magnification — the same linear part — because a scroll moves every bound by the
//! same translation, and the rounding that move leaves is bounded and added to the margin
//! ([`MarkBounds::reach`]). [`DisplayList::reaching`] names the commands that reach a target,
//! which is what a backend draws a part of a page from (ADR 1841), as a selection of the list's
//! own commands rather than a copy of them (`crate::selection`, ADR 1980).
//!
//! **A culled page draws the same pixels as the whole one**, and that is the whole contract. A
//! command is left out only where its bound, grown by the margin `render_cpu` already culls rows
//! with, misses the target; such a command marks no pixel of it, so leaving it out changes no
//! pixel the target holds. What does change is what a strip planner is told: a mark that reaches
//! no pixel of the target no longer forbids a cut across rows it never touches.

use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use rayon::prelude::*;

use crate::{DisplayList, Point, Rect, TargetSpec, Transform, Window};

/// The margin, in device pixels, a bound is grown by before it is said to miss a region.
///
/// `render_cpu`'s strip test takes one pixel for the reason `Band::covering` takes a row: the
/// bound comes from control points and the drawing from the path, and the two agree to within
/// rounding rather than exactly. The cull here asks the same question in two dimensions, so it
/// takes the same margin.
pub const MARGIN: f32 = 1.0;

/// The rounding a bound measured under one translation can carry when it is moved to another,
/// as a fraction of the largest coordinate involved.
///
/// Measuring a bound under `T` composes the mark's matrix with `T` and maps four corners, each a
/// sum of three products: a few units in the last place of the result. Measured under `T₀` and
/// moved by `T`'s translation less `T₀`'s, the bound differs from the one `T` gives by those
/// units twice over and the rounding of the move itself — under ten units in the last place of
/// the largest magnitude involved, where an `f32` holds 2⁻²³ of it. 2⁻¹⁶ is 128 units: room to
/// spare, in the direction a bound has to err in, and still a thousandth of a pixel at a
/// coordinate of sixty-five pixels.
const SHIFT_SLACK: f32 = 1.0 / 65_536.0;

/// One device bound per top-level command of a list, measured under one transform.
///
/// `None` is a command [`Command::device_bounds`](crate::Command::device_bounds) cannot bound — a group, a path with no points
/// — which can mark anywhere and is never said to miss anything.
#[derive(Debug)]
pub struct MarkBounds {
    /// The transform the bounds were measured under.
    at: Transform,
    /// Each top-level command's bound under [`Self::at`], in painting order.
    bounds: Vec<Option<Rect>>,
    /// What [`Self::bounds`] say together, taken on the first cull that asks.
    extremes: OnceLock<Extremes>,
}

/// The innermost edges of a list's bounds, which answer for every command at once whether a
/// target cuts any of them away ([`MarkBounds::every_command_reaches`]).
#[derive(Debug, Clone, Copy)]
enum Extremes {
    /// No command has a bound, so every one can mark anywhere.
    Unbounded,
    /// The bounded commands' leftmost right edge, rightmost left edge, highest bottom edge and
    /// lowest top edge, every one a number and every bound's corners in order.
    Inner {
        /// The smallest `max.x`.
        right: f32,
        /// The largest `min.x`.
        left: f32,
        /// The smallest `max.y`.
        bottom: f32,
        /// The largest `min.y`.
        top: f32,
    },
    /// A bound with a coordinate that is not a number, or with its corners out of order, which
    /// [`MarkBounds::every_command_reaches`]' argument does not cover: each command is asked.
    Unknown,
}

impl Extremes {
    /// One bound's edges.
    fn of(bound: Rect) -> Self {
        let edges = [bound.min.x, bound.min.y, bound.max.x, bound.max.y];
        if edges.iter().any(|edge| edge.is_nan())
            || bound.min.x > bound.max.x
            || bound.min.y > bound.max.y
        {
            return Self::Unknown;
        }
        Self::Inner {
            right: bound.max.x,
            left: bound.min.x,
            bottom: bound.max.y,
            top: bound.min.y,
        }
    }

    /// The edges of two sets of bounds together.
    fn joined(self, other: Self) -> Self {
        match (self, other) {
            (Self::Unknown, _) | (_, Self::Unknown) => Self::Unknown,
            (Self::Unbounded, one) | (one, Self::Unbounded) => one,
            (
                Self::Inner {
                    right,
                    left,
                    bottom,
                    top,
                },
                Self::Inner {
                    right: other_right,
                    left: other_left,
                    bottom: other_bottom,
                    top: other_top,
                },
            ) => Self::Inner {
                right: right.min(other_right),
                left: left.max(other_left),
                bottom: bottom.min(other_bottom),
                top: top.max(other_top),
            },
        }
    }
}

impl MarkBounds {
    /// Measures every top-level command of `list` under `at`.
    ///
    /// On every core, because each bound is a pure function of its command and a list may hold
    /// millions: the owner's witness is 4 408 405 commands, and a frame at its fit pays 18.5 to
    /// 21.9 ms on four threads for the test of every one against the window once the bounds are
    /// kept (`examples/tile_price` section 5, ADR 1841).
    #[must_use]
    pub fn measure(list: &DisplayList, at: Transform) -> Self {
        Self {
            at,
            bounds: list
                .commands()
                .par_iter()
                .map(|command| command.device_bounds(at))
                .collect(),
            extremes: OnceLock::new(),
        }
    }

    /// How many commands these bounds are of.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bounds.len()
    }

    /// Whether these are the bounds of no command at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bounds.is_empty()
    }

    /// Every top-level command's own [`Command::device_bounds`](crate::Command::device_bounds)
    /// under `transform`, exactly as that function states it, where these were measured under
    /// `transform` itself — all six of its numbers the same bits — and `None` under any other.
    ///
    /// No margin and no move: a strip planner reads these as the bounds it would measure, and
    /// a bound moved from another translation is only within [`SHIFT_SLACK`] of that.
    pub(crate) fn exactly_at(&self, transform: Transform) -> Option<&[Option<Rect>]> {
        let at = self.at;
        let same = [
            (at.a, transform.a),
            (at.b, transform.b),
            (at.c, transform.c),
            (at.d, transform.d),
            (at.e, transform.e),
            (at.f, transform.f),
        ]
        .iter()
        .all(|(kept, asked)| kept.to_bits() == asked.to_bits());
        same.then_some(self.bounds.as_slice())
    }

    /// Whether these bounds answer for `transform`: the same magnification — the same linear
    /// part, compared exactly — under any translation.
    #[must_use]
    pub fn serves(&self, transform: Transform) -> bool {
        let at = self.at;
        at.a.to_bits() == transform.a.to_bits()
            && at.b.to_bits() == transform.b.to_bits()
            && at.c.to_bits() == transform.c.to_bits()
            && at.d.to_bits() == transform.d.to_bits()
    }

    /// The device region command `index` can mark under `transform`, grown by [`MARGIN`] and by
    /// whatever the move from [`Self::at`]'s translation can have rounded away.
    ///
    /// `None` where the command cannot be bounded, where `index` is not one of these bounds'
    /// commands, or where `transform` is of another magnification ([`Self::serves`]) — every one
    /// of them the answer "it can mark anywhere", which is the direction a cull has to err in.
    #[must_use]
    pub fn reach(&self, index: usize, transform: Transform) -> Option<Rect> {
        if !self.serves(transform) {
            return None;
        }
        let bound = (*self.bounds.get(index)?)?;
        let (dx, dy) = (transform.e - self.at.e, transform.f - self.at.f);
        if dx.to_bits() == 0 && dy.to_bits() == 0 {
            return Some(bound.grown(MARGIN));
        }
        let moved = Rect {
            min: Point::new(bound.min.x + dx, bound.min.y + dy),
            max: Point::new(bound.max.x + dx, bound.max.y + dy),
        };
        let largest = [bound.min, bound.max, moved.min, moved.max]
            .iter()
            .flat_map(|point| [point.x.abs(), point.y.abs()])
            .chain([dx.abs(), dy.abs()])
            .fold(0.0f32, f32::max);
        Some(moved.grown(MARGIN + largest * SHIFT_SLACK))
    }

    /// Whether command `index` can mark no row from `top` to `bottom` of a target under
    /// `transform` — the strip test of `render_cpu`, asked of a bound measured once.
    ///
    /// Comparisons that are false on a NaN, so a bound that is not a number misses nothing.
    #[must_use]
    pub fn misses_rows(&self, index: usize, transform: Transform, top: f32, bottom: f32) -> bool {
        self.reach(index, transform)
            .is_some_and(|reach| reach.max.y < top || reach.min.y > bottom)
    }

    /// Whether no command [`Self::misses_target`] — every one either reaches `target` or cannot be
    /// bounded — where the bounds' innermost edges say so for all of them at once, and `false`
    /// where they do not, in which case each command is to be asked.
    ///
    /// **Never `true` where one command misses**, because each of the four tests [`Self::reach`]
    /// is put to is monotone in the edge it reads. Take the right edge: a bound is moved by a sum
    /// and grown by a sum of something not negative, and an `f32` sum rounds monotonically, so a
    /// bound whose right edge is at least the smallest one, moved, is at least that one moved; and
    /// that is not below zero, so neither is its reach. The same holds of the other three edges
    /// against the target's width and height. A NaN or a bound whose corners are out of order is
    /// outside that argument — [`Rect::grown`] puts corners in order, which can swap an edge for
    /// another — and is answered by asking each command (`Extremes::Unknown`).
    ///
    /// A cull of a whole page asked this way costs nothing after the first time, where it walked
    /// every command: 9 to 24 ms of the owner's witness's frame on four threads (ADR 1941).
    #[must_use]
    fn every_command_reaches(&self, target: TargetSpec) -> bool {
        if !self.serves(target.transform) {
            return true;
        }
        let extremes = *self.extremes.get_or_init(|| {
            self.bounds
                .par_iter()
                .flatten()
                .map(|bound| Extremes::of(*bound))
                .reduce(|| Extremes::Unbounded, Extremes::joined)
        });
        #[expect(
            clippy::cast_precision_loss,
            reason = "a target's extent is below MAX_EXTENT = 2^24, which f32 holds exactly"
        )]
        let (width, height) = (target.width as f32, target.height as f32);
        let (dx, dy) = (
            target.transform.e - self.at.e,
            target.transform.f - self.at.f,
        );
        match extremes {
            Extremes::Unbounded => true,
            Extremes::Inner {
                right,
                left,
                bottom,
                top,
            } => {
                right + dx >= 0.0 && left + dx <= width && bottom + dy >= 0.0 && top + dy <= height
            }
            Extremes::Unknown => false,
        }
    }

    /// Whether command `index` can mark no pixel of `target`.
    ///
    /// Comparisons that are false on a NaN, so a bound that is not a number misses nothing.
    #[must_use]
    pub fn misses_target(&self, index: usize, target: TargetSpec) -> bool {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a target's extent is below MAX_EXTENT = 2^24, which f32 holds exactly"
        )]
        let (width, height) = (target.width as f32, target.height as f32);
        self.reach(index, target.transform).is_some_and(|reach| {
            reach.max.x < 0.0 || reach.min.x > width || reach.max.y < 0.0 || reach.min.y > height
        })
    }
}

/// A list's [`MarkBounds`] at the magnification it was last asked at.
///
/// Part of the list for what it says about the list's commands and nothing else, so it is
/// invisible to the list's equality and its `Debug` — `raster_golden` digests that rendering of
/// every page it holds, and a cache that printed would move every digest without a pixel moving.
/// Emptied by every method that changes the commands it was measured from.
#[derive(Default)]
pub(crate) struct KeptBounds(Mutex<Option<Arc<MarkBounds>>>);

impl KeptBounds {
    /// What is kept, if it answers for `transform`.
    fn serving(&self, transform: Transform) -> Option<Arc<MarkBounds>> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .filter(|bounds| bounds.serves(transform))
            .map(Arc::clone)
    }

    /// Keeps `bounds` in place of whatever was kept.
    fn keep(&self, bounds: Arc<MarkBounds>) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(bounds);
    }

    /// Forgets what was kept, because the commands it measured have changed.
    pub(crate) fn forget(&mut self) {
        *self.0.get_mut().unwrap_or_else(PoisonError::into_inner) = None;
    }
}

impl Clone for KeptBounds {
    /// A copy of a list holds the same commands, so it may keep the same bounds.
    fn clone(&self) -> Self {
        Self(Mutex::new(
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone(),
        ))
    }
}

impl PartialEq for KeptBounds {
    /// Always equal: what is kept is derived from the commands, which the list compares itself.
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl DisplayList {
    /// Every top-level command's device bound under `transform`'s magnification, measured on
    /// the first ask at it and kept until another magnification is asked for.
    ///
    /// One magnification is kept rather than several because a viewer asks for the one it shows:
    /// a scroll keeps it, and a zoom step leaves it for good. See [`MarkBounds`].
    #[must_use]
    pub fn mark_bounds(&self, transform: Transform) -> Arc<MarkBounds> {
        if let Some(kept) = self.kept_bounds().serving(transform) {
            return kept;
        }
        let measured = Arc::new(MarkBounds::measure(self, transform));
        self.kept_bounds().keep(Arc::clone(&measured));
        measured
    }

    /// The commands of this list that can mark `target`, by their indices into it, or `None`
    /// where that would be this list — every command reaches the target — or where this list
    /// carries a plane drawn beside it that a selection of fewer commands could not keep in step.
    ///
    /// Byte for byte the same pixels in `target` as this list draws there: a command is left out
    /// only where its bound, grown by [`MARGIN`], misses every pixel of the target (the module
    /// comment, ADR 1841). A reader draws the window through [`Window::selection`], which names
    /// this list's own commands, clips and soft masks and copies none of them (ADR 1980), and asks
    /// this list's [`DisplayList::mark_bounds`] by each command's index into it.
    ///
    /// **Declined where the list carries a plane indexed by its commands**: §11.4.7's second
    /// raster in a four-component blending space, §11.7.5.2's transfer channel and §10.8.3's
    /// spot planes are each drawn beside the page from lists of their own, and a page with one
    /// is drawn whole rather than culled out of step with it. And declined for a list of more
    /// commands than a [`Window`]'s 32-bit indices count, which is drawn whole.
    #[must_use]
    pub fn reaching(&self, target: TargetSpec) -> Option<Window<'_>> {
        if self.black().is_some() || self.transfers().is_some() || self.separation().is_some() {
            return None;
        }
        let bounds = self.mark_bounds(target.transform);
        if bounds.every_command_reaches(target) || u32::try_from(self.commands().len()).is_err() {
            return None;
        }
        // The kept indices alone, on every core. A filtered collect joins its threads' pieces on
        // one thread (trap 186), which for four bytes a command is a copy of 17 MB on the witness
        // where the commands themselves were 520 MB and a reference count apiece (ADR 1941).
        let kept: Vec<u32> = (0..self.commands().len())
            .into_par_iter()
            .filter(|index| !bounds.misses_target(*index, target))
            .filter_map(|index| u32::try_from(index).ok())
            .collect();
        if kept.len() == self.commands().len() {
            return None;
        }
        Some(Window::new(self, kept))
    }

    /// The share of this list's top-level commands a drawing of `target` keeps — the share
    /// [`DisplayList::reaching`] would keep, `1.0` where it would keep them all — estimated from
    /// at most 4 096 of them taken at an even stride.
    ///
    /// **What a host asks before a draw, on the thread that asks** (`viewer_host::oversize`, ADR
    /// 1994): whether a window of a page is worth drawing in place of the page is a question of
    /// how many of its commands the window keeps, because a draw of a page this size costs per
    /// command (ADRs 1941 §1, 1980 §4). The exact answer is [`MarkBounds::measure`]'s, on every
    /// core: 18 to 22 ms of the owner's witness on four threads, and a host thread asking it while
    /// a draw holds the pool waits for that draw. A stride of the list asked on one thread is a
    /// few hundred microseconds and within a percent or two of the exact share, and a decision
    /// near the bound costs about the same either way, which is what a bound is.
    ///
    /// Each sampled command is asked as [`MarkBounds::misses_target`] asks a kept one, its bound
    /// grown by [`MARGIN`]. A list that carries a plane drawn beside it is drawn whole at every
    /// target, so it keeps every command.
    #[must_use]
    pub fn share_reaching(&self, target: TargetSpec) -> f64 {
        let commands = self.commands();
        if commands.is_empty()
            || self.black().is_some()
            || self.transfers().is_some()
            || self.separation().is_some()
        {
            return 1.0;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "a target's extent is below MAX_EXTENT = 2^24, which f32 holds exactly"
        )]
        let (width, height) = (target.width as f32, target.height as f32);
        let stride = commands.len().div_ceil(SAMPLED).max(1);
        let (mut asked, mut reaching) = (0_u32, 0_u32);
        for command in commands.iter().step_by(stride) {
            asked = asked.saturating_add(1);
            let misses = command
                .device_bounds(target.transform)
                .map(|bound| bound.grown(MARGIN))
                .is_some_and(|reach| {
                    reach.max.x < 0.0
                        || reach.min.x > width
                        || reach.max.y < 0.0
                        || reach.min.y > height
                });
            if !misses {
                reaching = reaching.saturating_add(1);
            }
        }
        f64::from(reaching) / f64::from(asked.max(1))
    }
}

/// Most commands [`DisplayList::share_reaching`] asks: a binomial standard error of under eight
/// tenths of a per cent on any share.
const SAMPLED: usize = 4096;

#[cfg(test)]
mod tests {
    //! Every test here declines under Miri, and for one reason: each one measures a list, and
    //! [`MarkBounds::measure`] and [`DisplayList::reaching`] run on rayon's pool whatever the
    //! list's length, so each starts the pool and meets `crossbeam-epoch`'s retag that `paint.rs`'s
    //! test of the divided reduction describes (ADR 0450). Nothing here parses untrusted input.
    use super::*;
    use crate::{Color, Command, FillRule, Paint, Path, PathCommand, Size};

    /// A unit square filled at `(x, y)` in page space.
    fn square(x: f32, y: f32) -> Command {
        let mut path = Path::new();
        path.push(PathCommand::MoveTo(Point::new(x, y)));
        path.push(PathCommand::LineTo(Point::new(x + 1.0, y)));
        path.push(PathCommand::LineTo(Point::new(x + 1.0, y + 1.0)));
        path.push(PathCommand::LineTo(Point::new(x, y + 1.0)));
        path.push(PathCommand::Close);
        Command::Fill {
            path: Arc::new(path),
            transform: Transform::IDENTITY,
            fill_rule: FillRule::NonZero,
            paint: Paint::Solid(Color::BLACK),
            clip: None,
            mask: None,
            blend: crate::BlendMode::Normal,
        }
    }

    /// A page of `count` squares in a row ten units apart.
    fn row_of(count: u16) -> DisplayList {
        let mut list = DisplayList::new(Size::new(f32::from(count) * 10.0, 10.0));
        for at in 0..count {
            list.push(square(f32::from(at) * 10.0, 4.0));
        }
        list
    }

    fn target(width: u32, height: u32, transform: Transform) -> TargetSpec {
        TargetSpec {
            width,
            height,
            transform,
        }
    }

    #[cfg_attr(
        miri,
        ignore = "crossbeam-epoch's retag under rayon, not this tree's — see `paint.rs`'s test of \
                  the divided reduction"
    )]
    #[test]
    fn a_window_keeps_the_marks_it_shows_and_the_margin_around_it() {
        let list = row_of(10);
        // Two squares' worth of the row, at twice the size, its left edge on page x = 30.
        let window = target(
            40,
            20,
            Transform::scale(2.0, 2.0).then(Transform::translate(-60.0, 0.0)),
        );
        let culled = list
            .reaching(window)
            .expect("most of the row is outside the window");
        // Squares at 30 and 40 are inside; the one at 20 ends at device x = -18 and the one at
        // 50 begins at device x = 40, the window's edge, which a bound grown by a pixel meets.
        let kept: Vec<usize> = culled
            .selection()
            .indexed()
            .map(|(index, _)| index)
            .collect();
        assert_eq!(kept, [3, 4, 5]);
        assert_eq!(culled.copied().commands(), &list.commands()[3..6]);
    }

    #[cfg_attr(
        miri,
        ignore = "crossbeam-epoch's retag under rayon, not this tree's — see `paint.rs`'s test of \
                  the divided reduction"
    )]
    #[test]
    fn the_share_a_window_keeps_of_a_short_list_is_its_selection_counted() {
        let list = row_of(10);
        // The row at twice its size, its left edge on page x = 29.75: the square at 50 begins at
        // device x = 40.5, half a pixel past the window's edge, which only the margin reaches.
        let window = target(
            40,
            20,
            Transform::scale(2.0, 2.0).then(Transform::translate(-59.5, 0.0)),
        );
        let kept = list
            .reaching(window)
            .expect("most of the row is outside")
            .len();
        assert_eq!(kept, 3);
        // Every command of a list this short is asked, so the share is exactly the selection's.
        assert!((list.share_reaching(window) - 0.3).abs() < 1e-12);
        assert!((list.share_reaching(target(100, 10, Transform::IDENTITY)) - 1.0).abs() < 1e-12);
        let empty = DisplayList::new(Size::new(10.0, 10.0));
        assert!((empty.share_reaching(window) - 1.0).abs() < 1e-12);
    }

    #[cfg_attr(
        miri,
        ignore = "crossbeam-epoch's retag under rayon, not this tree's — see `paint.rs`'s test of \
                  the divided reduction"
    )]
    #[test]
    fn the_share_a_window_keeps_of_a_long_list_is_estimated_within_a_percent() {
        // 50 000 squares, so a stride of thirteen asks 3 847 of them; the window keeps a run of
        // 7 000 and the two its margin meets.
        let list = row_of(50_000);
        let window = target(70_000, 10, Transform::translate(-100_000.0, 0.0));
        let kept = list
            .reaching(window)
            .expect("most of the row is outside the window")
            .len();
        #[expect(
            clippy::cast_precision_loss,
            reason = "counts of a few tens of thousands"
        )]
        let exact = kept as f64 / 50_000.0;
        let estimated = list.share_reaching(window);
        assert!(
            (estimated - exact).abs() < 0.01,
            "{estimated} against {exact}"
        );
    }

    #[cfg_attr(
        miri,
        ignore = "crossbeam-epoch's retag under rayon, not this tree's — see `paint.rs`'s test of \
                  the divided reduction"
    )]
    #[test]
    fn a_window_every_mark_reaches_is_the_list_itself() {
        let list = row_of(4);
        assert!(list.reaching(target(40, 10, Transform::IDENTITY)).is_none());
    }

    #[cfg_attr(
        miri,
        ignore = "crossbeam-epoch's retag under rayon, not this tree's — see `paint.rs`'s test of \
                  the divided reduction"
    )]
    #[test]
    fn a_scroll_is_served_by_the_bounds_of_the_magnification_and_a_zoom_is_not() {
        let list = row_of(3);
        let at = Transform::scale(3.0, 3.0);
        let first = list.mark_bounds(at);
        let scrolled = list.mark_bounds(at.then(Transform::translate(-7.5, 2.25)));
        assert!(
            Arc::ptr_eq(&first, &scrolled),
            "a scroll measures nothing again"
        );
        let zoomed = list.mark_bounds(Transform::scale(4.0, 4.0));
        assert!(
            !Arc::ptr_eq(&first, &zoomed),
            "a zoom is another magnification"
        );
    }

    #[cfg_attr(
        miri,
        ignore = "crossbeam-epoch's retag under rayon, not this tree's — see `paint.rs`'s test of \
                  the divided reduction"
    )]
    #[test]
    fn a_moved_bound_contains_the_bound_measured_where_it_was_moved_to() {
        let list = row_of(5);
        let at = Transform::scale(1.7, 1.3);
        let bounds = MarkBounds::measure(&list, at);
        for (dx, dy) in [(-1234.567, 89.01), (0.5, -0.25), (65_000.0, -40_000.0)] {
            let moved = at.then(Transform::translate(dx, dy));
            for (index, command) in list.commands().iter().enumerate() {
                let exact = command.device_bounds(moved).expect("a square has a bound");
                let reach = bounds.reach(index, moved).expect("the same magnification");
                assert!(
                    reach.grown(-MARGIN).contains(exact),
                    "{reach:?} against {exact:?}"
                );
            }
        }
    }

    #[cfg_attr(
        miri,
        ignore = "crossbeam-epoch's retag under rayon, not this tree's — see `paint.rs`'s test of \
                  the divided reduction"
    )]
    #[test]
    fn a_bound_of_another_magnification_or_none_misses_nothing() {
        let list = row_of(2);
        let bounds = MarkBounds::measure(&list, Transform::IDENTITY);
        let elsewhere = target(
            1,
            1,
            Transform::scale(2.0, 2.0).then(Transform::translate(-500.0, 0.0)),
        );
        assert!(!bounds.misses_target(0, elsewhere));
        assert!(!bounds.misses_target(7, target(1, 1, Transform::IDENTITY)));
    }

    /// Whether any command of `bounds` misses `target`, asked of each.
    fn any_misses(bounds: &MarkBounds, target: TargetSpec) -> bool {
        (0..bounds.len()).any(|index| bounds.misses_target(index, target))
    }

    #[cfg_attr(
        miri,
        ignore = "crossbeam-epoch's retag under rayon, not this tree's — see `paint.rs`'s test of \
                  the divided reduction"
    )]
    #[test]
    fn the_edges_say_every_command_reaches_only_where_none_misses() {
        let list = row_of(10);
        let at = Transform::scale(2.0, 2.0);
        let bounds = MarkBounds::measure(&list, at);
        let mut answered = 0;
        // Windows sliding across the row and the page by fractions of a pixel, through the
        // translations where the outermost square is first cut away.
        for step in -40..=40 {
            #[expect(clippy::cast_precision_loss, reason = "a small step count")]
            let shift = step as f32 * 0.37;
            for (width, height) in [(200, 20), (199, 20), (40, 20), (200, 4)] {
                let window = target(width, height, at.then(Transform::translate(shift, shift)));
                let reaches = bounds.every_command_reaches(window);
                assert!(
                    !(reaches && any_misses(&bounds, window)),
                    "{width} x {height} at {shift}"
                );
                answered += usize::from(reaches);
            }
        }
        assert!(answered > 0, "the edges answer for some window");
        // The page itself, under the transform the bounds were measured at and under a scroll.
        assert!(bounds.every_command_reaches(target(200, 20, at)));
        // A bound that is not a number is answered by asking each command.
        let mut odd = row_of(2);
        odd.push(square(f32::NAN, 4.0));
        let odd = MarkBounds::measure(&odd, at);
        assert!(!odd.every_command_reaches(target(40, 20, at)));
    }

    #[cfg_attr(
        miri,
        ignore = "crossbeam-epoch's retag under rayon, not this tree's — see `paint.rs`'s test of \
                  the divided reduction"
    )]
    #[test]
    fn a_change_to_the_commands_forgets_the_bounds() {
        let mut list = row_of(2);
        let before = list.mark_bounds(Transform::IDENTITY);
        list.push(square(50.0, 0.0));
        let after = list.mark_bounds(Transform::IDENTITY);
        assert_eq!((before.len(), after.len()), (2, 3));
    }

    #[cfg_attr(
        miri,
        ignore = "crossbeam-epoch's retag under rayon, not this tree's — see `paint.rs`'s test of \
                  the divided reduction"
    )]
    #[test]
    fn the_kept_bounds_are_invisible_to_equality_and_to_debug() {
        let list = row_of(2);
        let fresh = list.clone();
        let _ = list.mark_bounds(Transform::IDENTITY);
        assert_eq!(fresh, list);
        // Another list of the same commands, asked at another magnification: each keeps other
        // bounds, and each path has memoised its hull (which `Path`'s own `Debug` prints), so
        // the two renderings differ only if the kept bounds print.
        let other = row_of(2);
        let _ = other.mark_bounds(Transform::scale(2.0, 2.0));
        assert_eq!(list, other);
        assert_eq!(format!("{list:?}"), format!("{other:?}"));
    }
}
