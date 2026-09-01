//! What a display list's transparency groups cost to composite, and the bound on it.
//!
//! ISO 32000-2 §11.4.1 says what a group is — "a sequence of consecutive objects in a
//! transparency stack that shall be collected together and composited to produce a single
//! colour, shape, and opacity at each point" — and every backend in this tree carries that
//! out the same way: the elements are drawn onto a buffer and the buffer is then painted
//! onto the parent, once, over the rows the group's clip admits. That last step is the
//! **blit**, and its cost is the number of destination pixels it covers.
//!
//! # Why a bound is owed here at all
//!
//! Nothing else in this program can see this cost. [`crate::MAX_GROUP_DEPTH`] bounds how
//! deeply groups *nest*; `pdf-model`'s five interpretation budgets count tokens, operations
//! and bytes; and [`crate::TargetSpec::for_page`]'s `max_pixels` bounds the target once.
//! A page that states many groups side by side passes all of them:
//! `poppler-978-0.pdf`, filed against two readers by two people, states **73 047 unclipped
//! soft-masked groups** on a page of 298 379 commands, opens in 1.6 ms, interprets in 2.5 s
//! with nothing reported, and then asks a backend for some three hundred **billion** blitted
//! pixels — about 640 s on this machine, measured at 115 groups a second. There is no
//! interpretation defect to fix: the file says what it says, and what is owed is a bound.
//!
//! # What the standard says about a bound like this
//!
//! Nothing normative, and it says so at length rather than by silence. Annex C is
//! *informative* — its own title line says so — and ISO 32000-2 §C.1 is the clause that
//! describes exactly this situation:
//!
//! > In general, this PDF standard does not restrict the size or quantity of things
//! > described in the PDF file format, such as numbers, arrays, images, and so on. However,
//! > a particular PDF processor running on a particular device and in a particular operating
//! > environment will always have practical limits. When a PDF processor encounters a PDF
//! > construct that exceeds one of these internal limits or performs a computation whose
//! > intermediate results exceeds a limit, an error occurs.
//!
//! That is what this is: a computation whose intermediate results exceed a limit, and an
//! error. §C.3 puts memory limits in the same class and declines to characterise them
//! ("[m]emory limits cannot be characterised as precisely as architectural limits"), and
//! §C.2's NOTE adds that "[m]emory limits are often exceeded before architectural limits are
//! reached" — so the standard neither states a number here nor expects one to be stated. The
//! number is therefore this project's, sized by measurement and documented as a choice, which
//! is `CLAUDE.md` principle 5's rule for a place the specification defines nothing.
//!
//! # The measure is pixels, and the bound is absolute
//!
//! A group *count* is the wrong measure: 73 047 groups a few rows tall is cheap, and this
//! page's groups span the sheet. So the measure is cumulative blitted pixels.
//!
//! It is bounded **absolutely** rather than as a multiple of the target's own area, and that
//! choice was made from measurement rather than from taste. A ratio — "a page may repaint
//! the target N times" — is the scale-free statement and reads better, and the census below
//! prints it; what it is not is a statement about the resource. Timed at 1:1 with
//! `examples/render_at`, `6942273.pdf` asks for **660 repaints and draws in 0.2 s** because
//! its page is small, while `poppler-57-0.pdf` asks for **301 and takes 11.2 s** because its
//! page is not. Wall clock tracks the *product*, which is what this bound holds, and a
//! ratio bound tight enough to refuse the second would refuse the first — trap 11's own
//! shape, a refusal firing on a condition that is not the resource's.
//!
//! The consequence to know is about tiles: a target is a page, a window or one tile of one,
//! so this bounds the work of *one draw* rather than of a whole page at every zoom. That is
//! the right unit for an interactive viewer — a frame is what a reader waits for — and it
//! is stated here rather than left to be discovered.
//!
//! # It is a demand rather than a spend, and it errs upward
//!
//! [`group_blit_demand`] reads the list; it draws nothing. A group's rows come from
//! [`DisplayList::clip_bounds`], which is "a bound and never an underestimate", and a soft
//! mask may narrow the band further at draw time — so the demand is an **upper bound** on
//! what a backend would actually spend. That is the safe direction for a refusal to be
//! sized from as long as the *bound* is sized by the same instrument, which is what
//! [`MAX_GROUP_BLIT_PIXELS`] records: the constant is the census's own maximum with headroom
//! over it, so a page the census measured as affordable cannot be refused here.
//!
//! Reading the list rather than watching the spend is also what makes the refusal cheap.
//! A budget counted down during the draw returns the thread only after it has been spent;
//! this one is a walk over the commands, so the witness page above is refused in
//! milliseconds instead of in minutes.

use crate::backend::{BackendError, TargetSpec};
use crate::display_list::{Command, DisplayList};

/// Most pixels one draw's transparency groups may blit.
///
/// # Where the number comes from
///
/// Sized the way this tree's other resource bounds are — measure the population, then leave
/// headroom over its maximum — and `cargo run --release -p pdf-model --example
/// group_blit_census` is that measurement. It computes [`group_blit_demand`] for the first
/// page of every document in a corpus at 1:1 on a page-sized target, which is the condition
/// every figure below is quoted under. Over all three populations this tree has — 958 of the
/// curated pdf.js documents with a first page, the `SafeDocs` crawl's 65 659 and the
/// issue-tracker corpus's 8215, **74 832 first pages, of which 4764 state a group at all** —
/// the two ends of what is asked for are four orders of magnitude apart:
///
/// | first page | blitted pixels | drawn in |
/// |---|---|---|
/// | `poppler-978-0.pdf` (= `PDFBOX-3688-0.pdf`), the witness | 299.3 G | ~640 s |
/// | `1530064.pdf`, the heaviest that is not it | 23.08 G | 46.5 s |
/// | `poppler-57-0.pdf` | 2.42 G | 11.2 s |
/// | `poppler-LINK-250-0.pdf` | 2.33 G | 5.1 s |
/// | `7311598.pdf` | 0.82 G | 4.4 s |
/// | the other 4759 | under 0.5 G | under 3 s |
///
/// `2^35` — 34.36 G — is the heaviest **that is not the witness** with a factor of 1.49 over
/// it, which is the same modest headroom `MASK_BUDGET` was given over the 25.5 MB of banded
/// masks that motivated it (ADR 0010). The witness sits a factor of 8.7 *above* the bound,
/// so nothing here is finely balanced between the two.
///
/// **Refusal rate, with its conditions named**: one document — the witness, under both its
/// names — of the 74 832 first pages, at scale 1.0 on a page-sized target. It is the only
/// page in any of the three populations this bound takes away.
///
/// # What this bound is for, and what it is not for
///
/// It is `CLAUDE.md` principle 3's explicit resource bound and nothing more: it stops a page
/// that **cannot finish** from being started. It is deliberately *not* a bound on how long a
/// reader waits — a page right at it still costs about seventy seconds of drawing, and
/// `render-quorra/tests/group_cost.rs` measured four minutes for one under the test profile.
/// Interactivity is [`Interrupt`](crate::Interrupt)'s job (ADR 0650): a host abandons a draw
/// it no longer wants, per command, and gets its thread back in milliseconds. Setting this
/// constant low enough to make a draw feel fast would refuse `1530064.pdf` and four other
/// real documents that do finish — a page nobody can read, to save a page somebody can
/// already cancel.
///
/// A power of two carries no arithmetic meaning and nothing relies on it; it is written that
/// way so that a reader can see at a glance that no measurement was fitted to.
pub const MAX_GROUP_BLIT_PIXELS: u64 = 1 << 35;

/// The pixels `list`'s transparency groups would blit onto `target`.
///
/// Every group in the list is counted, wherever it is: nested inside another group, inside
/// the black half of §11.7.2's blending pair — which a backend composites as a second list
/// over the same geometry — and inside a soft mask's own commands, which are drawn into a
/// buffer of their own before the mask is applied. A soft mask is counted once however many
/// commands take it, because a backend builds each one once and keeps it.
///
/// The result saturates rather than wrapping. It cannot overflow in practice — a list holds
/// at most `u32::MAX` commands and a target at most `2^48` pixels — but a saturating sum is
/// what makes that a fact about the code rather than about the arithmetic.
#[must_use]
pub fn group_blit_demand(list: &DisplayList, target: TargetSpec) -> u64 {
    let mut pixels = walk(list, list.commands(), target);
    if let Some(black) = list.black() {
        // The page's own §11.7.2 pair: a second whole list, drawn onto a second target.
        pixels = pixels.saturating_add(walk(black, black.commands(), target));
    }
    for index in 0..list.soft_mask_count() {
        let Some(mask) = u32::try_from(index)
            .ok()
            .map(crate::soft_mask::SoftMaskId::new)
            .and_then(|id| list.soft_mask(id))
        else {
            continue;
        };
        pixels = pixels.saturating_add(walk(list, &mask.commands, target));
    }
    pixels
}

/// Refuses a list whose groups would blit more than [`MAX_GROUP_BLIT_PIXELS`].
///
/// Every backend that composites a group calls this before it draws anything, so that the
/// refusal costs a walk over the commands rather than the work it is refusing.
///
/// # Errors
///
/// [`BackendError::GroupsTooCostly`], carrying both figures, so that a host reports the
/// bound by name and by number rather than showing an empty page.
pub fn check_group_blit(list: &DisplayList, target: TargetSpec) -> Result<(), BackendError> {
    let demanded = group_blit_demand(list, target);
    if demanded > MAX_GROUP_BLIT_PIXELS {
        return Err(BackendError::GroupsTooCostly {
            demanded,
            limit: MAX_GROUP_BLIT_PIXELS,
        });
    }
    Ok(())
}

/// Sums the blit of every group in `commands`, resolving clips against `owner`.
///
/// `owner` is the list whose clip table the commands' [`crate::ClipId`]s name — the same
/// list for a group's elements and for a soft mask's, and the black half's own table for
/// the commands inside it.
fn walk(owner: &DisplayList, commands: &[Command], target: TargetSpec) -> u64 {
    let mut pixels = 0_u64;
    for command in commands {
        match command {
            Command::Group {
                commands,
                clip,
                blending,
                ..
            } => {
                let rows = admitted_rows(owner, *clip, target);
                pixels =
                    pixels.saturating_add(u64::from(target.width).saturating_mul(u64::from(rows)));
                pixels = pixels.saturating_add(walk(owner, commands, target));
                if let Some(black) = blending.as_deref().and_then(crate::GroupBlending::black) {
                    pixels = pixels.saturating_add(walk(owner, black, target));
                }
            }
            // §11.4.6's shaped pair is two commands drawn one after the other, and either
            // may be a group — a walk over the top level alone would miss both.
            Command::Shaped { object, shape, .. } => {
                pixels = pixels.saturating_add(walk(owner, std::slice::from_ref(object), target));
                pixels = pixels.saturating_add(walk(owner, std::slice::from_ref(shape), target));
            }
            _ => {}
        }
    }
    pixels
}

/// How many rows of `target` a command under `clip` may mark.
///
/// The whole target where nothing clips, and nothing at all where the chain admits no
/// pixel. In between it is the chain's bounding rectangle in device space, rounded outward
/// and widened by a row — which is exactly what `render_cpu`'s `Band::covering` does, and
/// for its reason: the bound is computed from control points while the mask is built from
/// the path, so the two agree to within rounding and a row short would lose a row of ink.
fn admitted_rows(list: &DisplayList, clip: Option<crate::ClipId>, target: TargetSpec) -> u32 {
    let Some(clip) = clip else {
        return target.height;
    };
    let Some(bounds) = list.clip_bounds(clip) else {
        return 0;
    };
    let device = bounds.mapped(target.transform);
    if !device.min.y.is_finite() || !device.max.y.is_finite() {
        return target.height;
    }
    let height = f64::from(target.height);
    let top = (f64::from(device.min.y) - 1.0).floor().clamp(0.0, height);
    let bottom = (f64::from(device.max.y) + 1.0).ceil().clamp(0.0, height);
    if bottom <= top {
        return 0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "both ends are whole numbers clamped to 0..=target.height, so the \
                  difference is a whole number a u32 holds exactly"
    )]
    let rows = (bottom - top) as u32;
    rows
}

#[cfg(test)]
mod tests {
    use super::{MAX_GROUP_BLIT_PIXELS, check_group_blit, group_blit_demand};
    use crate::backend::{BackendError, TargetSpec};
    use crate::display_list::{Clip, Command, DisplayList};
    use crate::geom::{Path, PathCommand, Point, Size};
    use crate::paint::{BlendMode, FillRule};

    /// A4 at 72 dpi, which is one pixel per PDF unit.
    fn a4() -> (DisplayList, TargetSpec) {
        let list = DisplayList::new(Size::new(595.0, 842.0));
        let target =
            TargetSpec::for_page(&list, 1.0, 1 << 30).expect("A4 at 1:1 is a valid target");
        (list, target)
    }

    fn group(clip: Option<crate::ClipId>, commands: Vec<Command>) -> Command {
        Command::Group {
            commands,
            alpha: 1.0,
            clip,
            mask: None,
            blend: BlendMode::Normal,
            isolated: true,
            knockout: false,
            alpha_is_shape: false,
            blending: None,
        }
    }

    #[test]
    fn a_list_with_no_group_demands_nothing() {
        let (list, target) = a4();
        assert_eq!(group_blit_demand(&list, target), 0);
    }

    /// An unclipped group repaints the whole target, which is the unit the bound counts in.
    #[test]
    fn an_unclipped_group_demands_one_whole_target() {
        let (mut list, target) = a4();
        list.push(group(None, Vec::new()));
        let whole = u64::from(target.width) * u64::from(target.height);
        assert_eq!(group_blit_demand(&list, target), whole);
    }

    /// The measure is pixels and not a count: the same number of groups costs what their
    /// clips admit, which is the whole reason a group count would be the wrong bound.
    #[test]
    fn a_clip_that_admits_few_rows_costs_few_rows() {
        let (mut list, target) = a4();
        let mut band = Path::default();
        band.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
        band.push(PathCommand::LineTo(Point::new(595.0, 0.0)));
        band.push(PathCommand::LineTo(Point::new(595.0, 10.0)));
        band.push(PathCommand::LineTo(Point::new(0.0, 10.0)));
        band.push(PathCommand::Close);
        let clip = list
            .add_clip(Clip {
                path: band,
                transform: crate::geom::Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                parent: None,
            })
            .expect("one clip is under the table's bound");
        for _ in 0..100 {
            list.push(group(Some(clip), Vec::new()));
        }
        // The clip is the bottom ten units of the page, which the y-flip puts on device
        // rows 832..842. Widened by a row at each end and rounded out that is 831..843, and
        // clamped to the target's own last row it is eleven rows a group.
        let per_group = u64::from(target.width) * 11;
        assert_eq!(group_blit_demand(&list, target), per_group * 100);
        check_group_blit(&list, target)
            .expect("a hundred twelve-row blits is far inside the bound");
    }

    /// The pathology `poppler-978-0.pdf` states, in miniature: unclipped groups side by
    /// side, each blitting the whole sheet.
    #[test]
    fn groups_spanning_the_page_are_refused_past_the_bound() {
        let (mut list, target) = a4();
        let whole = u64::from(target.width) * u64::from(target.height);
        // One group past the bound, whatever the page's own area is.
        let count = MAX_GROUP_BLIT_PIXELS / whole + 1;
        for _ in 0..count {
            list.push(group(None, Vec::new()));
        }
        match check_group_blit(&list, target) {
            Err(BackendError::GroupsTooCostly { demanded, limit }) => {
                assert_eq!(demanded, whole * count);
                assert_eq!(limit, MAX_GROUP_BLIT_PIXELS);
                assert!(demanded > limit);
            }
            other => panic!("expected a group-cost refusal, got {other:?}"),
        }
    }

    /// Exactly the bound is admitted: the refusal is `>` and not `>=`.
    #[test]
    fn the_bound_itself_is_admitted() {
        let (mut list, target) = a4();
        let whole = u64::from(target.width) * u64::from(target.height);
        for _ in 0..MAX_GROUP_BLIT_PIXELS / whole {
            list.push(group(None, Vec::new()));
        }
        check_group_blit(&list, target).expect("the bound itself is inside the bound");
    }

    /// A group inside a group is a second blit, which is what makes nesting cost as well as
    /// width — and what a walk over the top level alone would miss.
    #[test]
    fn a_nested_group_is_counted_too() {
        let (mut list, target) = a4();
        list.push(group(None, vec![group(None, Vec::new())]));
        let whole = u64::from(target.width) * u64::from(target.height);
        assert_eq!(group_blit_demand(&list, target), whole * 2);
    }

    /// A group inside a soft mask's own list blits into the mask's buffer, so it is counted:
    /// a bound a mask could hide a page behind is not a bound.
    #[test]
    fn a_group_inside_a_soft_mask_is_counted() {
        let (mut list, target) = a4();
        list.add_soft_mask(crate::soft_mask::SoftMask {
            kind: crate::soft_mask::SoftMaskKind::Alpha,
            commands: vec![group(None, Vec::new())],
            transfer: None,
        })
        .expect("one soft mask is under the table's bound");
        let whole = u64::from(target.width) * u64::from(target.height);
        assert_eq!(group_blit_demand(&list, target), whole);
    }
}
