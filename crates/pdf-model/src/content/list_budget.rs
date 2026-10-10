//! The bytes one interpretation may keep — its display list and what is read off it — and what
//! each mark costs.
//!
//! # Why this, and not a count, is what bounds a run
//!
//! A content stream is a program, and what it can exhaust is memory: time it can only spend, and
//! spending time is the person's to stop (ADR 0657, ADR 0241). So no count of operators refuses a
//! page (ADR 1809), and this budget is the one that has to see every route by which a run keeps
//! something. An operator's cost in memory is not one thing: a `re f` is one command; a tiling
//! cell is replayed once per site, and each copy carries the cell's clips; a form invoked a million
//! times builds its path a million times; and a Type 3 glyph inside a Type 3 glyph inside a tiling
//! pattern multiplies all of them. The corpus's `ContentStreamCycleType3insideType3.pdf` builds
//! 3.93 million commands and **2.05 million clips**, 1.15 GiB of list for a 2440-byte file (ADR
//! 1507). Principle 3 asks for "[e]xplicit memory and time budgets" against "pathological
//! content", and a bound in the unit of the work is the memory one.
//!
//! # What is charged, and when
//!
//! The loop compares its operator count against [`ListBudget::next_check`], and only when it is
//! reached does the slow path run — every [`CHECK_EVERY`] operators. So an ordinary operator pays
//! nothing for this bound beyond that one comparison, and no mark pays anything as it is drawn
//! (ADR 1507 section 2). The slow path charges, from the list's and the readback's own counts:
//!
//! - **each command added since the last check**, at a command's own size plus the dash array the
//!   graphics state then carries — a stroke clones it. A group's elements were counted as they were
//!   drawn, so a list that shrank because they were wrapped is taken where it now stands;
//! - **each clip added since**, at its own size and its path's, which it owns and which a
//!   tiling's copy clones — walking the table from where the last check stopped, so neither the
//!   places that add one nor the copies a cell's replication makes can miss a route;
//! - **what the readback gained since**: the text, one placement per code shown, and one span per
//!   marked-content section closed with something to record, with the strings a span copies out of
//!   its property list — none of which is a mark, and all of which a run re-reading one form
//!   keeps once per reading (ADR 1810).
//!
//! Two charges are made where the thing is made rather than at the check, because one operator can
//! be the whole budget: **a painted path's segments**, at the painting operator, once however many
//! commands share the path (`Interpreter::charge_painted`); and **a tiling's copies**, every
//! command and every element inside each, before the next copy is made.
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

/// Most bytes one interpretation may keep: its display list and its readback (ADRs 1507, 1810).
///
/// **The value is the confined worker's, and it is half of it.** The worker that interprets a
/// page runs under `pdf_sandbox`'s `INTERPRETER_ADDRESS_SPACE_LIMIT` of 4 GiB, which is the
/// machine's answer to "is this a resource problem": a list that fits under it can be drawn, and
/// one that does not kills the worker. Two gibibytes leaves the other half for what is alive
/// beside the list at its largest — the frame or the encoded list that crosses to the host (ADR
/// 0607), or a second interpretation where a check keeps two — so a page the worker could have
/// shown is never refused here, and a page it could not is refused by name before the kill.
///
/// **What it admits, measured.** The project owner's A1 engineering drawing,
/// `demo_heavy_engineering_A1.pdf` (28 992 250 operators in one stream, 4 408 405 commands),
/// charges 740.1 MiB with its painted paths counted, and stood at 504.7 MiB of the earlier 512 MiB
/// with them uncounted — under it by 1.4%, and only because a path's segments were left to an
/// operator count (ADR 1810). Its interpretation peaks at 1.23 GiB resident.
/// `examples/display_list_census` over the first page of 90 150 documents found the largest list
/// a corpus page builds inside every other bound at 214.3 MiB, uncounted paths again (ADR 1507).
/// The cycle this budget exists for is refused at it, with the same report.
///
/// A charge past the bound stops the run — every enclosing stream stops at its next operator, and a
/// tiling stops before its next copy — and is reported once, as [`Unsupported::ListBytes`] with
/// the charge and the bound.
pub const MAX_LIST_BYTES: usize = 2 << 30;

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
    /// The operator count at which the loop next asks this bound.
    next_check: usize,
    /// How much of each part of the readback has been charged, which only ever grows but for
    /// `truncate_readback` — and a readback that shrank is charged again only past where it was.
    readback_seen: Readback,
    /// Whether the refusal has been reported, so it is reported once.
    reported: bool,
}

impl ListBudget {
    /// Bytes charged so far — [`crate::Interpretation::list_bytes`].
    pub(super) const fn charged(self) -> usize {
        self.charged
    }

    /// The operator count at which the loop next asks this bound.
    pub(super) const fn next_check(self) -> usize {
        self.next_check
    }
}

/// How long each part of an interpretation's readback is: its text in bytes, and its placements
/// and spans in entries.
#[derive(Debug, Clone, Copy, Default)]
struct Readback {
    text: usize,
    placed: usize,
    described: usize,
    artifacts: usize,
    marked: usize,
    associated: usize,
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
    /// Charges a path about to be painted: its segments, which the command that keeps it holds.
    ///
    /// Charged at the painting operator rather than at the check, because a command does not say
    /// whether its path is the one a glyph's every occurrence shares — a glyph's outline is built
    /// once per font and never comes through here — or one this run just built, which is every
    /// path that does. `B` and `b` paint one path twice and charge it once. A path under hidden
    /// optional content is charged and then not kept, which errs towards refusing sooner.
    pub(super) fn charge_painted(&mut self, path: &pdf_render::geom::Path) {
        let bytes = elements(path.commands().len(), size_of::<PathCommand>());
        self.list_budget.charged = self.list_budget.charged.saturating_add(bytes);
    }

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
        self.list_budget.next_check = self.operations.saturating_add(CHECK_EVERY);
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
        self.charge_readback();
    }

    /// Charges what the readback gained since the last charge: each part's new entries at their
    /// own size, and a described span's strings, which it copies out of its property list on every
    /// reading of the stream that states it.
    fn charge_readback(&mut self) {
        let seen = self.list_budget.readback_seen;
        let now = Readback {
            text: self.text.len(),
            placed: self.text_layer.len(),
            described: self.described.len(),
            artifacts: self.artifacts.len(),
            marked: self.marked.len(),
            associated: self.associated.len(),
        };
        let strings = self
            .described
            .get(seen.described..)
            .unwrap_or_default()
            .iter()
            .fold(0_usize, |total, described| {
                [&described.alt, &described.expansion, &described.language]
                    .into_iter()
                    .flatten()
                    .fold(total, |total, string| total.saturating_add(string.len()))
            });
        let gained =
            |now: usize, seen: usize, size: usize| elements(now.saturating_sub(seen), size);
        let bytes = [
            gained(now.text, seen.text, 1),
            gained(now.placed, seen.placed, size_of::<super::Placed>()),
            gained(
                now.described,
                seen.described,
                size_of::<crate::accessibility::Described>(),
            ),
            gained(
                now.artifacts,
                seen.artifacts,
                size_of::<super::ArtifactSpan>(),
            ),
            gained(now.marked, seen.marked, size_of::<super::MarkedSpan>()),
            gained(
                now.associated,
                seen.associated,
                size_of::<(std::ops::Range<usize>, crate::attachment::Attachment)>(),
            ),
            strings,
        ]
        .into_iter()
        .fold(0_usize, usize::saturating_add);
        let budget = &mut self.list_budget;
        budget.charged = budget.charged.saturating_add(bytes);
        // A part that shrank is charged again only past its longest, which is what was allocated.
        budget.readback_seen = Readback {
            text: seen.text.max(now.text),
            placed: seen.placed.max(now.placed),
            described: seen.described.max(now.described),
            artifacts: seen.artifacts.max(now.artifacts),
            marked: seen.marked.max(now.marked),
            associated: seen.associated.max(now.associated),
        };
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
