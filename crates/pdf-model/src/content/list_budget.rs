//! The bytes one interpretation may build into its display list, and what each mark costs.
//!
//! # Why the operator budget does not bound this
//!
//! `MAX_OPERATIONS` counts operators, and an operator's cost in memory is not one thing. A `re f`
//! is one command; a tiling cell is replayed once per site and every site's copy is charged to
//! the operator budget as one operator a command, but the copy also carries the cell's clips, and
//! a Type 3 glyph inside a Type 3 glyph inside a tiling pattern multiplies both. The corpus's
//! `ContentStreamCycleType3insideType3.pdf` spends its four million on 3.93 million commands and
//! **2.05 million clips**, 1.15 GiB of list and a 1.8 GiB peak for a 2440-byte file, and under
//! the fuzz target's purity check, which keeps two interpretations alive at once, 3.16 GiB
//! (ADR 1507). Principle 3 asks for "[e]xplicit memory and time budgets" against "pathological
//! content", and a bound in the unit of the work is the memory one.
//!
//! # What is charged, and when
//!
//! The charge is asked where the operator loop already asks `MAX_OPERATIONS`, and folded into
//! that one comparison: the loop compares its count against [`ListBudget::next_check`], which is
//! never past `MAX_OPERATIONS`, and only when it is reached does the slow path run — every
//! [`CHECK_EVERY`] operators. So an ordinary operator pays nothing for this bound beyond the
//! comparison it already made, and no mark pays anything as it is drawn (ADR 1507 section 2).
//! The slow path charges, from the list's own counts:
//!
//! - **each command added since the last check**, at a command's own size plus the dash array the
//!   graphics state then carries — a stroke clones it, and it is the one part of a mark a content
//!   stream can make large without spending operators. A path is not charged: its segments each
//!   cost an operator, and a copy shares it. A group's elements were counted as they were drawn,
//!   so a list that shrank because they were wrapped is taken where it now stands;
//! - **each clip added since**, at its own size and its path's, which it owns and which a
//!   tiling's copy clones — walking the table from where the last check stopped, so neither the
//!   places that add one nor the copies a cell's replication makes can miss a route;
//! - **a tiling's copies**, charged as they are made rather than at the check, every command and
//!   every element inside it, because one operator's replication can be the whole budget.
//!
//! A run may therefore pass the bound by what [`CHECK_EVERY`] operators add before it is asked —
//! one tiling copy at most, since replication asks before every copy.
//!
//! Image samples are not charged here: they are shared through an `Arc` by every copy, and
//! `MAX_SAMPLES` already bounds what one image may decode. What is charged is an estimate of the
//! allocation, not the allocator's own count, and it is a function of the document alone, which
//! is what lets the refusal be the same on every machine — `interpret` stays a pure function of
//! the bytes (`CLAUDE.md`, "`pdf_syntax::Document` stays immutable").

use std::mem::size_of;

use pdf_render::display_list::Clip;
use pdf_render::geom::PathCommand;
use pdf_render::{ClipId, Command, Paint};

use super::Interpreter;
use super::report::Unsupported;

/// Most bytes one interpretation may charge to its display list (ADR 1507).
///
/// **The value is a census's, and a stated multiple of it.** `examples/display_list_census`
/// over the first page of 90 150 documents — `doc/pdf.js`, `doc/corpora` and the three corpus
/// caches — finds the largest list a page builds inside every other bound at 214.3 MiB
/// (`poppler-12206-0.pdf` of the Tika tracker: 957 898 commands, 435 681 clips, nothing
/// reported); 512 MiB is 2.4 times that. The one other page past 256 MiB,
/// `GHOSTSCRIPT-697013-0.pdf`, is stopped by `MAX_OPERATIONS` at 441.6 MiB, so this bound moves
/// no refusal a page already had. The cycle this exists for built 1.15 GiB of
/// list before `MAX_OPERATIONS` stopped it, and is refused at under half of that (ADR 1507).
///
/// A charge past the bound stops the run as `MAX_OPERATIONS` does — every enclosing stream stops
/// at its next operator, and a tiling stops before its next copy — and is reported once, as
/// [`Unsupported::ListBytes`] with the charge and the bound.
pub const MAX_LIST_BYTES: usize = 512 << 20;

/// A command's own size, before anything it carries.
pub(super) const COMMAND_BYTES: usize = size_of::<Command>();

/// How many operators run between two charges of the list.
///
/// Small enough that the overshoot is a few kilobytes on an honest page, and large enough that the
/// slow path runs for one operator in sixty-four (ADR 1507 section 2).
const CHECK_EVERY: usize = 64;

/// What one interpretation has charged to its display list.
///
/// `Copy` so that a checkpoint (`Interpreter::checkpoint`) carries it as it carries the operator
/// count: a run rolled back to the end of its content stream has charged what it had charged
/// there.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct ListBudget {
    /// Bytes charged so far.
    charged: usize,
    /// How many top-level commands the list held when it was last charged.
    commands_seen: usize,
    /// How many entries of the list's clip table have been charged — the table only grows.
    clips_charged: usize,
    /// The operator count at which the loop next asks either bound.
    next_check: usize,
    /// Whether the refusal has been reported, so it is reported once.
    reported: bool,
}

impl ListBudget {
    /// Bytes charged so far — [`crate::Interpretation::list_bytes`].
    pub(super) const fn charged(self) -> usize {
        self.charged
    }

    /// The operator count at which the loop next asks `MAX_OPERATIONS` and this bound.
    pub(super) const fn next_check(self) -> usize {
        self.next_check
    }
}

/// An `Arc`'s two reference counts, which sit in front of what it shares.
const ARC_COUNTS: usize = elements(2, size_of::<usize>());

/// `count` elements of `size` bytes each.
const fn elements(count: usize, size: usize) -> usize {
    count.saturating_mul(size)
}

/// A stroke's dash array, which every copy of the command clones.
fn dash_bytes(stroke: &pdf_render::Stroke) -> usize {
    elements(stroke.dash_array.len(), size_of::<f32>())
}

/// A command a tiling's replication has copied: its own size and every element it carries.
///
/// The copy shares its paths with the cell; it clones a group's elements, and it makes a new
/// shading wherever the paint is one, displaced with the geometry.
fn copied_bytes(command: &Command) -> usize {
    match command {
        Command::Fill { paint, .. } => COMMAND_BYTES.saturating_add(shading_bytes(paint)),
        Command::Stroke { paint, stroke, .. } => COMMAND_BYTES
            .saturating_add(shading_bytes(paint))
            .saturating_add(dash_bytes(stroke)),
        Command::Group { commands, .. } => commands.iter().fold(COMMAND_BYTES, |total, element| {
            total.saturating_add(copied_bytes(element))
        }),
        Command::Shaped { object, shape } => COMMAND_BYTES
            .saturating_add(copied_bytes(object))
            .saturating_add(copied_bytes(shape)),
        // An image's samples are shared by every copy, and a kind added later costs itself.
        _ => COMMAND_BYTES,
    }
}

/// The new shading a copy's paint carries, where it is one.
fn shading_bytes(paint: &Paint) -> usize {
    match paint {
        Paint::Shading(_) => ARC_COUNTS.saturating_add(size_of::<pdf_render::shading::Shading>()),
        _ => 0,
    }
}

/// A clip and the path it owns.
fn clip_bytes(clip: &Clip) -> usize {
    size_of::<Clip>().saturating_add(elements(
        clip.path.commands().len(),
        size_of::<PathCommand>(),
    ))
}

impl Interpreter<'_> {
    /// Charges the last `copied` commands of the list, which a tiling's replication just made.
    pub(super) fn charge_copied(&mut self, copied: usize) {
        let commands = self.list.commands();
        let from = commands.len().saturating_sub(copied);
        let bytes = commands
            .get(from..)
            .unwrap_or_default()
            .iter()
            .fold(0_usize, |total, command| {
                total.saturating_add(copied_bytes(command))
            });
        let budget = &mut self.list_budget;
        budget.charged = budget.charged.saturating_add(bytes);
        // Charged here in full, so the next check does not charge them again as drawn.
        budget.commands_seen = budget.commands_seen.saturating_add(copied);
    }

    /// The loop's slow path: charges what the list gained since the last check and says whether
    /// it has spent [`MAX_LIST_BYTES`], reporting the refusal the first time it has. `dash` is
    /// the dash array's length in the graphics state the commands were drawn under.
    pub(super) fn list_spent(&mut self, dash: usize) -> bool {
        self.charge_gained(dash);
        self.list_budget.next_check = self
            .operations
            .saturating_add(CHECK_EVERY)
            .min(super::MAX_OPERATIONS.saturating_add(1));
        self.list_past(0)
    }

    /// Charges what the list gained after the run's last check, so that
    /// [`crate::Interpretation::list_bytes`] is the whole list's charge. The run is over and cut
    /// nothing, so there is no refusal to report here.
    pub(super) fn settle_list_charge(&mut self) {
        self.charge_gained(0);
    }

    /// Charges the commands and clips the list gained since the last charge.
    fn charge_gained(&mut self, dash: usize) {
        let count = self.list.command_count();
        let budget = &mut self.list_budget;
        let added = count.saturating_sub(budget.commands_seen);
        let each = COMMAND_BYTES.saturating_add(elements(dash, size_of::<f32>()));
        budget.charged = budget.charged.saturating_add(elements(added, each));
        budget.commands_seen = count;
        self.charge_clips();
    }

    /// Whether `more` bytes on top of what is charged would pass [`MAX_LIST_BYTES`] — asked
    /// before a tiling's next copy, so the copy that would cross the bound is not made.
    pub(super) fn list_past(&mut self, more: usize) -> bool {
        let budget = self.list_budget;
        if budget.charged.saturating_add(more) <= MAX_LIST_BYTES {
            return false;
        }
        // Spent stays spent: every operator after this one, in this stream and in every stream
        // enclosing it, takes the slow path and stops at once.
        self.list_budget.next_check = 0;
        if !budget.reported {
            self.list_budget.reported = true;
            self.note(Unsupported::ListBytes {
                charged: budget.charged.saturating_add(more),
                bound: MAX_LIST_BYTES,
            });
        }
        true
    }

    /// Charges every clip the table has gained since the last charge.
    fn charge_clips(&mut self) {
        let count = self.list.clip_count();
        while self.list_budget.clips_charged < count {
            let index = self.list_budget.clips_charged;
            let bytes = u32::try_from(index)
                .ok()
                .and_then(|index| self.list.clip(ClipId::new(index)))
                .map_or(0, clip_bytes);
            let budget = &mut self.list_budget;
            budget.charged = budget.charged.saturating_add(bytes);
            budget.clips_charged = index.saturating_add(1);
        }
    }
}
