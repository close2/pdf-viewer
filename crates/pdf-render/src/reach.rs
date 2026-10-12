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
//! ([`MarkBounds::reach`]). [`DisplayList::reaching`] is the list of the commands that reach a
//! target, which is what a backend draws a part of a page from (ADR 1841).
//!
//! **A culled list draws the same pixels as the whole one**, and that is the whole contract. A
//! command is left out only where its bound, grown by the margin `render_cpu` already culls rows
//! with, misses the target; such a command marks no pixel of it, so leaving it out changes no
//! pixel the target holds. What does change is what a strip planner is told: a mark that reaches
//! no pixel of the target no longer forbids a cut across rows it never touches.

use std::sync::{Arc, Mutex, PoisonError};

use rayon::prelude::*;

use crate::{DisplayList, Point, Rect, TargetSpec, Transform};

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

    /// The bounds of the commands `kept` names, in the order it names them, under the same
    /// transform: what a list of those commands alone would measure.
    fn kept(&self, kept: &[usize]) -> Self {
        Self {
            at: self.at,
            bounds: kept
                .iter()
                .map(|index| self.bounds.get(*index).copied().flatten())
                .collect(),
        }
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

    /// The commands of this list that can mark `target`, as a list of their own, or `None`
    /// where it would be this list — every command reaches the target — or where this list
    /// carries a plane drawn beside it that a list of fewer commands could not keep in step.
    ///
    /// Byte for byte the same pixels in `target` as this list draws there: a command is left out
    /// only where its bound, grown by [`MARGIN`], misses every pixel of the target (the module
    /// comment, ADR 1841). The list returned keeps this one's clips, soft masks, page boundary
    /// and blending curve, and the bounds of the commands it keeps, so a strip of it asks
    /// [`DisplayList::mark_bounds`] and measures nothing again.
    ///
    /// **Declined where the list carries a plane indexed by its commands**: §11.4.7's second
    /// raster in a four-component blending space, §11.7.5.2's transfer channel and §10.8.3's
    /// spot planes are each drawn beside the page from lists of their own, and a page with one
    /// is drawn whole rather than culled out of step with it.
    #[must_use]
    pub fn reaching(&self, target: TargetSpec) -> Option<Self> {
        if self.black().is_some() || self.transfers().is_some() || self.separation().is_some() {
            return None;
        }
        let bounds = self.mark_bounds(target.transform);
        let kept: Vec<usize> = (0..self.commands().len())
            .into_par_iter()
            .filter(|index| !bounds.misses_target(*index, target))
            .collect();
        if kept.len() == self.commands().len() {
            return None;
        }
        let commands = kept
            .iter()
            .filter_map(|index| self.commands().get(*index).cloned())
            .collect();
        let culled = self.with_commands(commands);
        culled.kept_bounds().keep(Arc::new(bounds.kept(&kept)));
        Some(culled)
    }
}

#[cfg(test)]
mod tests {
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
        assert_eq!(culled.commands(), &list.commands()[3..6]);
    }

    #[test]
    fn a_window_every_mark_reaches_is_the_list_itself() {
        let list = row_of(4);
        assert!(list.reaching(target(40, 10, Transform::IDENTITY)).is_none());
    }

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

    #[test]
    fn a_change_to_the_commands_forgets_the_bounds() {
        let mut list = row_of(2);
        let before = list.mark_bounds(Transform::IDENTITY);
        list.push(square(50.0, 0.0));
        let after = list.mark_bounds(Transform::IDENTITY);
        assert_eq!((before.len(), after.len()), (2, 3));
    }

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
