//! One page's stream read on many threads, divided where `Q` restores the state (ADR 1880).
//!
//! A page's `/Contents` is one sequence of operators, and every operator's meaning depends on the
//! graphics state the operators before it left. ISO 32000-2 §8.4.2 bounds that dependence:
//!
//! > The Q operator shall restore the entire graphics state to its former value by popping it from the stack.
//!
//! So at a point just after a `Q` the state is the one in force before the matching `q`, changed
//! only by the operators stated outside every `q`…`Q` pair still closed there — the *spine*. A
//! piece of the stream beginning at such a point is interpreted on its own once that state is
//! composed for it, and the pieces' lists joined in order are the page's list.
//!
//! # The three passes
//!
//! 1. **The plan** (`Plan::of`) lexes the stream in chunks at once. Each chunk records its
//!    effect on the state stack — how many levels it pops below its start, and per level it ends
//!    at the statements whose effect lasts (a `cm`, a clip's path object, the line parameters,
//!    the colours) — and the chunks' effects are composed in order. A cut is a point after a `Q`
//!    where nothing else is open: no path (§8.5.2.1: "the current path is not part of the
//!    graphics state"), no text object (§9.4.1), no marked-content sequence (§8.11.3.2), no
//!    compatibility section (§7.8.2), no inline image.
//! 2. **The pieces** are interpreted at once, each from its composed state: the spine's own
//!    statements are run by a scratch interpreter whose state, and whose clip table, the piece
//!    begins with. The first piece is the page's own interpreter on the calling thread, so that
//!    ADR 1855's hand-over still leaves from the thread a launch placed the read on.
//! 3. **The join** (`Interpreter::join`) takes each later piece's clips into the page's table
//!    — a clip the table already holds keeps its identifier, as `add_clip` gives it — renumbers
//!    the piece's commands, and appends them; the notes, the counts and the list's charge are
//!    the page's.
//!
//! # What declines a division
//!
//! The division is exact or it does not happen: a page whose stream states anything the join
//! does not carry is read as it was before — whole, on one thread — and the reason is a
//! sentence ([`Declined`]) a census can print. **That list is the build's honest limit, not a
//! clause's**: a text object, a marked-content sequence, an inline image, a compatibility
//! section, or a piece whose reading left page-level state the join does not carry (a soft mask,
//! a transfer function, §11.6.4.3's readings, the compositing record, text met inside a form).
//! A `gs` is carried as a statement of the spine, and declines only where it reaches one of those
//! records (ADR 1894). Each of the rest is the next build's to carry, and ADR 1880 says what each
//! needs.

use std::cell::Cell;
use std::ops::Range;

use pdf_render::{ClipId, Command};
use pdf_syntax::{Dictionary, Document, Token};

mod pipeline;

use super::reader::{CEILING, ContentReader, Operator};
use super::run::Composed;
use super::{GraphicsState, Interpreter, MAX_OPERANDS, MAX_STATE_DEPTH};
use crate::colour::Compositing;
use crate::page::Page;

/// The encoded length from which a page's stream is read whole and divided, in bytes.
///
/// Below it a page is read through its window as before: ADR 1873 measured the witness's read at
/// 6.5 µs per kilobyte inflated, so a stream of a mebibyte encoded — three to ten inflated —
/// reads in tens of milliseconds, which is what a division could save at most.
pub const DIVIDE_FROM: usize = 1 << 20;

/// How many times its encoded length a divided stream may inflate to before the division is
/// given up and the page read through its window.
///
/// Reading whole is what a division needs and what ADR 0362's window exists to avoid: a
/// decompression bomb read whole is an allocation nothing takes back. This bounds the allocation
/// by the bytes the file spent on the stream; the witness inflates 7×, ADR 0362's bomb 580×.
pub const INFLATION_BOUND: usize = 128;

/// How many pieces each of the pool's threads is given.
///
/// More than one, because the pieces do not cost alike and the pool's threads do not run alike —
/// this machine's eight slower cores read at half the speed of its fast ones (ADR 1845 §1) — so a
/// thread that finishes early takes another piece rather than waiting for the slowest.
const PIECES_PER_THREAD: usize = 4;

/// The least a piece is, in bytes inflated: below it a thread costs more than it reads.
const PIECE_FLOOR: usize = 1 << 20;

/// How many statements one level of the composed state may hold before the division declines.
///
/// A level keeps every `cm` and clip it states, which a pathological stream can repeat without
/// end; past this the spine costs more to compose than the piece saves.
const MOST_KEPT: usize = 4_096;

/// How a page's stream is divided, where a caller says so for one interpretation.
///
/// The default ([`Division::DEFAULT`]) divides a stream of at least [`DIVIDE_FROM`] encoded
/// bytes into as many pieces as the pool has threads, each at least a mebibyte. A test that
/// sweeps a corpus asks for small pieces of every page, which is what makes the sweep a check of
/// the join rather than of the pages large enough to divide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Division {
    /// The encoded length from which a stream is divided, in bytes.
    pub from: usize,
    /// How many pieces, or `None` for the pool's thread count with `PIECE_FLOOR` each.
    pub pieces: Option<usize>,
    /// Whether the pieces start as the inflate passes them (`pipeline`), or once the whole
    /// stream is inflated and planned — the second is what the first falls back to, and what an
    /// instrument prices it against.
    pub pipelined: bool,
}

impl Division {
    /// The division every interpretation gets unless a caller says otherwise.
    pub const DEFAULT: Self = Self {
        from: DIVIDE_FROM,
        pieces: None,
        pipelined: true,
    };

    /// The division as ADR 1880 built it: the whole stream inflated, then planned, then read.
    pub const AFTER_THE_INFLATE: Self = Self {
        pipelined: false,
        ..Self::DEFAULT
    };

    /// No division: every page read whole, as before ADR 1880.
    pub const NEVER: Self = Self {
        from: usize::MAX,
        pieces: Some(1),
        pipelined: false,
    };

    /// Runs `run` with this division in force for every interpretation it makes on this thread.
    pub fn during<T>(self, run: impl FnOnce() -> T) -> T {
        struct Restore(Option<Division>);
        impl Drop for Restore {
            fn drop(&mut self) {
                let previous = self.0;
                // A thread being torn down has no slot to restore.
                let _ = IN_FORCE.try_with(|slot| slot.set(previous));
            }
        }
        let previous = IN_FORCE.with(|slot| slot.replace(Some(self)));
        let _restore = Restore(previous);
        run()
    }

    /// The division in force on this thread.
    fn in_force() -> Self {
        IN_FORCE
            .try_with(Cell::get)
            .ok()
            .flatten()
            .unwrap_or(Self::DEFAULT)
    }
}

thread_local! {
    static IN_FORCE: Cell<Option<Division>> = const { Cell::new(None) };
    static LAST: Cell<Option<Reading>> = const { Cell::new(None) };
    static LAST_PIPELINE: Cell<Option<Declined>> = const { Cell::new(None) };
}

/// Why the last page interpreted on this thread was not read with its inflate pipelined into
/// the pieces, where it was asked to be and the reading fell back to [`Division::AFTER_THE_INFLATE`]'s
/// — `None` where the pipeline read it, or was not asked. For a census, as [`last_reading`] is.
#[must_use]
pub fn last_pipeline_decline() -> Option<Declined> {
    LAST_PIPELINE.try_with(Cell::get).ok().flatten()
}

/// How a page's stream was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// In pieces at once, joined.
    Divided,
    /// Whole, on one thread, and why.
    Whole(Declined),
}

/// How the last page interpreted on this thread was read, or `None` where no page's stream has
/// been read here — for a census or a test, which is the only reader of a decline.
#[must_use]
pub fn last_reading() -> Option<Reading> {
    LAST.try_with(Cell::get).ok().flatten()
}

/// Why a page's stream was read whole rather than divided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Declined(pub &'static str);

/// What the divided read is handed: the inputs [`Interpreter::for_page`] makes a piece's
/// interpreter from, which are the page's own.
pub(super) struct Inputs<'i> {
    pub(super) document: &'i Document,
    pub(super) page: &'i Page,
    pub(super) state: &'i crate::view::ViewState,
    pub(super) presses: &'i crate::colour::Presses,
    pub(super) fonts: &'i super::FontCache,
    pub(super) references: &'i crate::reference::Supply,
}

impl<'i> Inputs<'i> {
    /// The inputs, one each.
    pub(super) const fn new(
        document: &'i Document,
        page: &'i Page,
        state: &'i crate::view::ViewState,
        presses: &'i crate::colour::Presses,
        fonts: &'i super::FontCache,
        references: &'i crate::reference::Supply,
    ) -> Self {
        Self {
            document,
            page,
            state,
            presses,
            fonts,
            references,
        }
    }
}

/// Reads a page's `/Contents` into `interpreter`: divided where the plan finds cuts, whole
/// otherwise — and through `reader`'s window wherever the whole stream is not read at all.
///
/// The decline is kept for [`last_reading`] and is otherwise silent: it is a statement about this
/// reader's speed, and the page drawn is the same page either way.
pub(super) fn read_page<'r>(
    interpreter: &mut Interpreter<'_>,
    reader: &mut ContentReader<'r>,
    inputs: &Inputs<'r>,
    initial: &GraphicsState,
) {
    let reading =
        read(interpreter, reader, inputs, initial).map_or(Reading::Divided, Reading::Whole);
    let _ = LAST.try_with(|slot| slot.set(Some(reading)));
}

/// See [`read_page`].
fn read<'r>(
    interpreter: &mut Interpreter<'_>,
    reader: &mut ContentReader<'r>,
    inputs: &Inputs<'r>,
    initial: &GraphicsState,
) -> Option<Declined> {
    let division = Division::in_force();
    let encoded = reader.progress().map_or(0, |(_, held)| held);
    if division == Division::NEVER
        || encoded < division.from
        || interpreter.ledger.is_some()
        || !matches!(interpreter.compositing, Compositing::Device)
    {
        interpreter.run_reader(reader, &inputs.page.resources, initial);
        return Some(Declined("the stream is not one this reader divides"));
    }
    let most = encoded
        .saturating_mul(INFLATION_BOUND)
        .min(inputs.document.limits().max_stream_len);
    let _ = LAST_PIPELINE.try_with(|slot| slot.set(None));
    let bytes = if division.pipelined {
        match pipeline::read(interpreter, reader, inputs, initial, division.pieces, most) {
            pipeline::Outcome::Divided => return None,
            pipeline::Outcome::PastBound => None,
            pipeline::Outcome::Windowed(reason) => {
                let _ = LAST_PIPELINE.try_with(|slot| slot.set(Some(reason)));
                windowed_again(interpreter, reader, inputs, initial);
                return Some(reason);
            }
            pipeline::Outcome::Inflated(bytes, reason) => {
                let _ = LAST_PIPELINE.try_with(|slot| slot.set(Some(reason)));
                Some(bytes)
            }
            pipeline::Outcome::Released(reason) => {
                let _ = LAST_PIPELINE.try_with(|slot| slot.set(Some(reason)));
                *reader = ContentReader::for_page(inputs.document, inputs.page);
                // The page's own issues were noted from the first window before the run, and the
                // pump's are met again as this one reads the stream.
                let _ = reader.take_issues();
                reader.read_to_end_within(most)
            }
        }
    } else {
        reader.read_to_end_within(most)
    };
    let Some(bytes) = bytes else {
        windowed_again(interpreter, reader, inputs, initial);
        return Some(Declined("the stream inflates past its bound"));
    };
    after_the_inflate(interpreter, reader, inputs, initial, &bytes, division)
}

/// Divides a stream already inflated whole: planned, then read in pieces (ADR 1880) — what the
/// pipeline falls back to where one of its guessed cuts was not confirmed.
fn after_the_inflate<'r>(
    interpreter: &mut Interpreter<'_>,
    reader: &mut ContentReader<'r>,
    inputs: &Inputs<'r>,
    initial: &GraphicsState,
    bytes: &[u8],
    division: Division,
) -> Option<Declined> {
    let pieces = division.pieces.unwrap_or_else(|| {
        rayon::current_num_threads()
            .saturating_mul(PIECES_PER_THREAD)
            .min(bytes.len() / PIECE_FLOOR)
            .max(1)
    });
    let plan = match Plan::of(bytes, pieces) {
        Ok(plan) => plan,
        Err(Refusal::Lexing(reason)) => {
            windowed_again(interpreter, reader, inputs, initial);
            return Some(reason);
        }
        Err(Refusal::Joining(reason)) => {
            whole(interpreter, bytes, inputs, initial);
            return Some(reason);
        }
    };
    // Zero, one cut and the length are two pieces; anything less is the stream whole.
    if plan.cuts.len() < 3 {
        whole(interpreter, bytes, inputs, initial);
        return Some(Declined(
            "the stream has no point where only the state is open",
        ));
    }
    let seam = interpreter.checkpoint();
    match interpreter.read_divided(bytes, &plan, inputs, initial) {
        Ok(()) => None,
        Err(reason) => {
            interpreter.restore(seam);
            interpreter.forget_memos();
            if let Some(parts) = &interpreter.hand_over {
                parts.spoil_if_handed("the divided read was given up after a part was handed over");
            }
            whole(interpreter, bytes, inputs, initial);
            Some(reason)
        }
    }
}

/// Reads the stream from its whole bytes on this thread, as one piece.
///
/// Only after the plan has lexed every token: a whole buffer and a window read the same tokens
/// but for the two the window refuses — a token past [`CEILING`] and an inline image past its
/// lookahead — and a plan that met either has sent the page back to the window instead.
fn whole(
    interpreter: &mut Interpreter<'_>,
    bytes: &[u8],
    inputs: &Inputs<'_>,
    initial: &GraphicsState,
) {
    let mut reader = ContentReader::piece(bytes, 0, bytes.len());
    interpreter.run_reader(&mut reader, &inputs.page.resources, initial);
}

/// Reads the page through a new window, as before the division: the first one was spent reading
/// the stream whole.
fn windowed_again<'r>(
    interpreter: &mut Interpreter<'_>,
    reader: &mut ContentReader<'r>,
    inputs: &Inputs<'r>,
    initial: &GraphicsState,
) {
    *reader = ContentReader::for_page(inputs.document, inputs.page);
    // The page's own issues were noted from the first window before the run: these are the same.
    let _ = reader.take_issues();
    interpreter.run_reader(reader, &inputs.page.resources, initial);
}

/// Why a plan was not made, and which of the two readings the page then gets.
enum Refusal {
    /// Something the window reads differently from a whole buffer, so the page goes back to it.
    Lexing(Declined),
    /// Something the join does not carry: the whole bytes read on one thread are the page.
    Joining(Declined),
}

/// One kept statement of a level: its kind, and its bytes in the stream.
#[derive(Debug, Clone)]
struct Kept {
    kind: Kind,
    bytes: Range<usize>,
}

/// What a kept statement sets, which decides what a later statement replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A statement whose effect composes with what came before — `cm`, a clip's path object,
    /// `ri` — and which is therefore kept in order and never replaced.
    Ordered,
    /// A line parameter, replaced by the next statement of the same operator.
    Line(u8),
    /// The stroking colour space, which also resets the stroking colour (§8.6.8).
    StrokeSpace,
    /// The stroking colour, in the space in force.
    StrokeColour,
    /// The same two for the other painting operations.
    FillSpace,
    /// See [`Kind::FillSpace`].
    FillColour,
    /// A text state parameter (§9.3.1), replaced by the next statement of the same operator.
    Text(u8),
}

/// The statements one level of the stack keeps — what a `Q` back to it does not undo.
#[derive(Debug, Clone, Default)]
struct Level {
    kept: Vec<Kept>,
}

impl Level {
    /// Keeps `statement`, dropping what it replaces. Kept statements stay in their order, so a
    /// statement that reads another's effect still finds it before itself.
    fn keep(&mut self, kind: Kind, bytes: Range<usize>) -> Result<(), Refusal> {
        let replaces = |held: Kind| match kind {
            Kind::Ordered => false,
            Kind::Line(_) | Kind::Text(_) | Kind::StrokeColour | Kind::FillColour => held == kind,
            Kind::StrokeSpace => matches!(held, Kind::StrokeSpace | Kind::StrokeColour),
            Kind::FillSpace => matches!(held, Kind::FillSpace | Kind::FillColour),
        };
        if kind != Kind::Ordered {
            self.kept.retain(|held| !replaces(held.kind));
        }
        if self.kept.len() >= MOST_KEPT {
            return Err(Refusal::Joining(Declined(
                "a level of the stack keeps more statements than a piece may compose",
            )));
        }
        self.kept.push(Kept { kind, bytes });
        Ok(())
    }
}

/// A chunk's effect on the stack: how many levels below its start it popped, and the levels it
/// ends at, the first of which is the level it popped to.
#[derive(Debug, Clone, Default)]
struct Effect {
    pops: usize,
    levels: Vec<Level>,
    /// How many `EMC`s closed sections opened before the chunk.
    closes: usize,
    /// The `BMC` and `BDC` statements the chunk left open, outermost first.
    opens: Vec<Range<usize>>,
}

/// The composed state at a point of the stream, as statements: the state stack's levels, and the
/// marked-content sections open there (§8.11.3.2), whose statements the next piece's spine
/// states again after the levels — a section nests independently of `q` and `Q`.
#[derive(Debug, Clone)]
struct Stack {
    levels: Vec<Level>,
    sections: Vec<Range<usize>>,
}

impl Stack {
    /// A stream's beginning: one level, keeping nothing, and no section open.
    fn fresh() -> Self {
        Self {
            levels: vec![Level::default()],
            sections: Vec::new(),
        }
    }
}

impl Effect {
    fn new() -> Self {
        Self {
            pops: 0,
            levels: vec![Level::default()],
            closes: 0,
            opens: Vec::new(),
        }
    }

    /// The same effect with every kept statement's bytes `by` further into the stream — a chunk
    /// lexed from a buffer that begins there.
    fn shift(&mut self, by: usize) {
        let moved =
            |bytes: &Range<usize>| bytes.start.saturating_add(by)..bytes.end.saturating_add(by);
        for level in &mut self.levels {
            for kept in &mut level.kept {
                kept.bytes = moved(&kept.bytes);
            }
        }
        for opened in &mut self.opens {
            *opened = moved(opened);
        }
    }

    /// The deepest the chunk went above its start.
    fn depth(&self) -> usize {
        self.levels.len().saturating_sub(1)
    }

    /// The stack `below` with this effect applied on top of it.
    ///
    /// An `EMC` with no section open is ignored by the interpreter (`marked.pop()` finds
    /// nothing), so the sections close down to none and no further.
    fn after(&self, below: &Stack) -> Result<Stack, Refusal> {
        let mut sections: Vec<Range<usize>> = below
            .sections
            .iter()
            .take(below.sections.len().saturating_sub(self.closes))
            .cloned()
            .collect();
        sections.extend(self.opens.iter().cloned());
        if sections.len() > MOST_KEPT {
            return Err(Refusal::Joining(Declined(
                "more marked-content sections open than a piece may compose",
            )));
        }
        Ok(Stack {
            levels: self.levels_after(&below.levels)?,
            sections,
        })
    }

    /// The state stack's levels `below` with this effect applied on top of them.
    fn levels_after(&self, below: &[Level]) -> Result<Vec<Level>, Refusal> {
        let kept = below
            .len()
            .checked_sub(self.pops)
            .filter(|&kept| kept > 0)
            .ok_or(Refusal::Joining(Declined(
                "a Q with no q to restore, which the interpreter ignores and a plan cannot compose",
            )))?;
        let mut stack: Vec<Level> = below.iter().take(kept).cloned().collect();
        let mut levels = self.levels.iter();
        if let (Some(top), Some(first)) = (stack.last_mut(), levels.next()) {
            for statement in &first.kept {
                top.keep(statement.kind, statement.bytes.clone())?;
            }
        }
        stack.extend(levels.cloned());
        Ok(stack)
    }
}

/// Whether a path is being constructed.
///
/// Known at every chunk's start, which is why the chunks begin where they do: the first begins
/// the stream, and every other just after a path-painting operator, which ends a path object
/// (§8.5.3.1) — so a clip's path object is always wholly inside one chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathState {
    /// No path is open.
    Closed,
    /// A path object is open, begun at this byte.
    Open(usize),
}

/// A cut the chunk saw: where, and the chunk's effect up to it.
#[derive(Debug, Clone)]
struct Candidate {
    at: usize,
    effect: Effect,
}

/// What one chunk's lexing found.
struct Chunk {
    /// The effect of the whole chunk.
    effect: Effect,
    /// The deepest the stack went above the chunk's start.
    deepest: usize,
    /// The first cut at or after each target in the chunk, and the chunk's first cut.
    candidates: Vec<Candidate>,
    /// Where the chunk's last token ended, and whether it was an operator outside any array.
    ended: (usize, bool),
    /// Where the chunk's first text object begins, if it holds one.
    first_text: Option<usize>,
    /// Whether the chunk's last token is a `Q` operator that ends exactly at the chunk's end with
    /// no path or clip open — what confirms a cut guessed from the bytes (ADR 1894).
    closes_on_a_q: bool,
}

/// Where the stream is cut, and the state each piece begins in.
pub(super) struct Plan {
    /// Byte offsets: zero, each cut, and the stream's length.
    cuts: Vec<usize>,
    /// For each piece, the spine's statements that compose its state; empty for the first.
    spines: Vec<Vec<u8>>,
}

impl Plan {
    /// Plans `bytes` into at most `pieces` pieces of about equal length.
    fn of(bytes: &[u8], pieces: usize) -> Result<Self, Refusal> {
        let length = bytes.len();
        let targets: Vec<usize> = (1..pieces)
            .map(|piece| {
                length
                    .saturating_mul(piece)
                    .checked_div(pieces)
                    .unwrap_or(0)
            })
            .collect();
        // The chunks are the pieces' lengths again: each begins just after the first operator a
        // lexer meets from a line's start, and the chunk before must end exactly there.
        let starts: Vec<usize> = {
            use rayon::prelude::*;
            let mut starts: Vec<usize> = targets
                .par_iter()
                .map(|&target| sync_point(bytes, target))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect();
            starts.insert(0, 0);
            starts.dedup();
            starts
        };
        let chunks: Vec<Chunk> = {
            use rayon::prelude::*;
            starts
                .par_iter()
                .enumerate()
                .map(|(index, &start)| {
                    let stop = starts
                        .get(index.saturating_add(1))
                        .copied()
                        .unwrap_or(length);
                    lex_chunk(bytes, start, stop, &targets)
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        let mut stack = Stack::fresh();
        let mut found: Vec<(usize, Stack)> = Vec::new();
        for (index, chunk) in chunks.iter().enumerate() {
            let next = starts.get(index.saturating_add(1)).copied();
            if let Some(next) = next
                && chunk.ended != (next, true)
            {
                return Err(Refusal::Lexing(Declined(
                    "a chunk's lexing did not end where the next began",
                )));
            }
            if stack
                .levels
                .len()
                .saturating_sub(chunk.effect.pops)
                .saturating_add(chunk.deepest)
                >= MAX_STATE_DEPTH
            {
                return Err(Refusal::Joining(Declined(
                    "the stack reaches MAX_STATE_DEPTH, where a q is refused",
                )));
            }
            for candidate in &chunk.candidates {
                found.push((candidate.at, candidate.effect.after(&stack)?));
            }
            stack = chunk.effect.after(&stack)?;
        }

        // Every text object is the last piece's, so that the readback the page builds is built in
        // one piece and the seam between pieces never falls between two show operations.
        let text_from = chunks
            .iter()
            .find_map(|chunk| chunk.first_text)
            .unwrap_or(length);
        let mut cuts = vec![0];
        let mut spines = vec![Vec::new()];
        for &target in &targets {
            let floor = cuts.last().copied().unwrap_or(0).max(target);
            if let Some((at, composed)) = found.iter().find(|(at, _)| *at >= floor && *at > 0)
                && *at < length
                && *at <= text_from
                && cuts.last() != Some(at)
            {
                cuts.push(*at);
                spines.push(spine(bytes, composed));
            }
        }
        cuts.push(length);
        Ok(Self { cuts, spines })
    }

    /// The pieces: each one's bytes, and the spine that composes its state.
    fn pieces(&self) -> impl Iterator<Item = (Range<usize>, &[u8])> {
        self.cuts
            .windows(2)
            .filter_map(|cut| Some(cut.first().copied()?..cut.get(1).copied()?))
            .zip(self.spines.iter().map(Vec::as_slice))
    }
}

/// The spine's statements as one stream: each level's kept statements, with a `q` between one
/// level and the next — the producer's own bytes, in the producer's order.
fn spine(bytes: &[u8], composed: &Stack) -> Vec<u8> {
    spine_of(composed, |range| bytes.get(range).unwrap_or_default())
}

/// [`spine`], each statement's bytes found by `statement` — in whichever piece of the stream
/// holds them, where the stream is held in pieces.
fn spine_of<'b>(composed: &Stack, statement: impl Fn(Range<usize>) -> &'b [u8]) -> Vec<u8> {
    let mut spine = Vec::new();
    for (index, level) in composed.levels.iter().enumerate() {
        if index > 0 {
            spine.extend_from_slice(b"q\n");
        }
        for kept in &level.kept {
            spine.extend_from_slice(statement(kept.bytes.clone()));
            spine.push(b'\n');
        }
    }
    // The sections open at the cut, opened again in order: whether each hides what follows is
    // decided by the view's optional content configuration (§8.11.4), not by the graphics state
    // the levels above composed, so they may follow all of it.
    for section in &composed.sections {
        spine.extend_from_slice(statement(section.clone()));
        spine.push(b'\n');
    }
    spine
}

/// The point after the first path-painting operator a lexer meets from the first line start at
/// or after `target` — where a chunk begins — or `None` where the stream ends first.
///
/// A painting operator ends the path object and any clip it set, so the chunk begins with no path
/// open; the chunk before must end exactly there, on the same operator outside any array, which
/// is what [`Plan::of`] checks before believing it.
fn sync_point(bytes: &[u8], target: usize) -> Result<Option<usize>, Refusal> {
    let Some(line) = bytes
        .get(target..)
        .and_then(|rest| rest.iter().position(|&byte| byte == b'\n'))
        .map(|offset| target.saturating_add(offset).saturating_add(1))
    else {
        return Ok(None);
    };
    let mut reader = ContentReader::over(bytes);
    reader.skip(line);
    loop {
        let operator = reader.with_token(|token| match token {
            None => None,
            Some(Token::Keyword(word)) => Some(Operator::of(word).is_some_and(Operator::paints)),
            Some(_) => Some(false),
        });
        match operator {
            None => return Ok(None),
            Some(true) => return Ok(reader.position()),
            Some(false) => {}
        }
    }
}

/// What a token is, for the plan.
enum Lexed {
    End,
    Operand,
    ArrayOpen,
    ArrayClose,
    Dictionary,
    Keyword(Option<Operator>),
}

/// Lexes the tokens beginning in `start..stop` and records the chunk's effect on the stack.
#[expect(
    clippy::too_many_lines,
    reason = "one pass over the operators the plan reads, as one table; split, the path's and the \
              stack's bookkeeping would be passed back and forth"
)]
fn lex_chunk(bytes: &[u8], start: usize, stop: usize, targets: &[usize]) -> Result<Chunk, Refusal> {
    let mut reader = ContentReader::over(bytes);
    reader.skip(start);
    let mut effect = Effect::new();
    let mut deepest = 0usize;
    let mut path = PathState::Closed;
    let mut clip_pending = false;
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut statement: Option<usize> = None;
    let mut operands = 0usize;
    let mut arrays = 0usize;
    let mut ended = (start, true);
    let mut target = targets
        .iter()
        .copied()
        .filter(|&target| target >= start)
        .peekable();
    let mut first_candidate = true;
    let mut first_text: Option<usize> = None;
    let mut closed_on_a_q = false;
    while let Some(begins) = reader.next_token_start() {
        if begins >= stop {
            break;
        }
        let lexed = reader.with_token(|token| match token {
            None => Lexed::End,
            Some(Token::ArrayOpen) => Lexed::ArrayOpen,
            Some(Token::ArrayClose) => Lexed::ArrayClose,
            Some(Token::DictOpen) => Lexed::Dictionary,
            Some(Token::Keyword(word)) => Lexed::Keyword(Operator::of(word)),
            Some(_) => Lexed::Operand,
        });
        let ends = reader.position().unwrap_or(begins);
        if ends.saturating_sub(begins) > CEILING {
            return Err(Refusal::Lexing(Declined(
                "a token longer than the window's ceiling",
            )));
        }
        let operator = match lexed {
            Lexed::End => break,
            Lexed::Dictionary if arrays == 0 => {
                // A marked-content operator's property list (§14.6.2). The plan steps over it as
                // `inline_dictionary` reads it, and declines one whose shape that reader would
                // read some other way — then the two could disagree about where it ends.
                let after = step_over_dictionary(&mut reader, 0)?;
                statement.get_or_insert(begins);
                closed_on_a_q = false;
                operands = operands.saturating_add(1);
                ended = (after, false);
                continue;
            }
            Lexed::Dictionary => {
                return Err(Refusal::Joining(Declined(
                    "a dictionary inside an array outside a property list",
                )));
            }
            Lexed::Operand | Lexed::ArrayOpen | Lexed::ArrayClose => {
                match lexed {
                    Lexed::ArrayOpen => arrays = arrays.saturating_add(1),
                    Lexed::ArrayClose => arrays = arrays.saturating_sub(1),
                    _ => {}
                }
                statement.get_or_insert(begins);
                closed_on_a_q = false;
                operands = operands.saturating_add(1);
                if operands >= MAX_OPERANDS {
                    return Err(Refusal::Joining(Declined(
                        "more operands before one operator than MAX_OPERANDS",
                    )));
                }
                ended = (ends, false);
                continue;
            }
            Lexed::Keyword(_) if arrays > 0 => {
                // §7.3.6: a keyword inside an array is neither an element nor an operator; the
                // interpreter reports it and keeps the array.
                closed_on_a_q = false;
                ended = (ends, false);
                continue;
            }
            Lexed::Keyword(operator) => operator,
        };
        ended = (ends, true);
        closed_on_a_q = matches!(operator, Some(Operator::Restore))
            && path == PathState::Closed
            && !clip_pending;
        let range = statement.take().unwrap_or(begins)..ends;
        operands = 0;
        let Some(operator) = operator else {
            // A keyword Table 50 does not list: the interpreter reports it and clears the
            // operands, which is all the plan needs to know of it.
            continue;
        };
        let top = effect
            .levels
            .last_mut()
            .ok_or(Refusal::Joining(Declined("the plan's stack is empty")))?;
        match operator {
            Operator::Save => {
                effect.levels.push(Level::default());
                deepest = deepest.max(effect.depth());
            }
            Operator::Restore => {
                if effect.levels.len() > 1 {
                    effect.levels.pop();
                } else {
                    effect.pops = effect.pops.saturating_add(1);
                    effect.levels = vec![Level::default()];
                }
                if path == PathState::Closed {
                    let due = target.peek().is_some_and(|&next| ends >= next);
                    if due || first_candidate {
                        candidates.push(Candidate {
                            at: ends,
                            effect: effect.clone(),
                        });
                        first_candidate = false;
                        while target.peek().is_some_and(|&next| ends >= next) {
                            target.next();
                        }
                    }
                }
            }
            // §8.4.5's `gs` sets graphics state parameters, and §8.4.2's `Q` restores them as it
            // restores the rest — so its statement is kept in order, like a `cm`, and the spine
            // states it again. A parameter it sets that reaches a page-level record — a soft
            // mask, a transfer function, §11.6.4.3's alpha source, the compositing record — is
            // met by the piece that states it, which `outside_the_division` then declines.
            Operator::Concatenate | Operator::RenderingIntent | Operator::ExtGState => {
                top.keep(Kind::Ordered, range)?;
            }
            Operator::LineWidth => top.keep(Kind::Line(0), range)?,
            Operator::LineCap => top.keep(Kind::Line(1), range)?,
            Operator::LineJoin => top.keep(Kind::Line(2), range)?,
            Operator::MiterLimit => top.keep(Kind::Line(3), range)?,
            Operator::Dash => top.keep(Kind::Line(4), range)?,
            Operator::Flatness => top.keep(Kind::Line(5), range)?,
            Operator::StrokeColourSpace
            | Operator::StrokeGray
            | Operator::StrokeRgb
            | Operator::StrokeCmyk => top.keep(Kind::StrokeSpace, range)?,
            Operator::StrokeColour | Operator::StrokeColourN => {
                top.keep(Kind::StrokeColour, range)?;
            }
            Operator::FillColourSpace
            | Operator::FillGray
            | Operator::FillRgb
            | Operator::FillCmyk => {
                top.keep(Kind::FillSpace, range)?;
            }
            Operator::FillColour | Operator::FillColourN => top.keep(Kind::FillColour, range)?,
            Operator::MoveTo
            | Operator::LineTo
            | Operator::CurveTo
            | Operator::CurveInitialReplicated
            | Operator::CurveFinalReplicated
            | Operator::ClosePath
            | Operator::Rectangle => {
                if path == PathState::Closed {
                    path = PathState::Open(range.start);
                }
            }
            Operator::Clip | Operator::ClipEvenOdd => {
                clip_pending = true;
                if path == PathState::Closed {
                    path = PathState::Open(range.start);
                }
            }
            Operator::Stroke
            | Operator::CloseStroke
            | Operator::Fill
            | Operator::FillObsolete
            | Operator::FillEvenOdd
            | Operator::FillStroke
            | Operator::FillStrokeEvenOdd
            | Operator::CloseFillStroke
            | Operator::CloseFillStrokeEvenOdd
            | Operator::EndPath => {
                // §8.5.4: the clip takes effect at the painting operator that ends its path object,
                // so the whole object is one statement of the level it was stated at.
                if clip_pending {
                    let from = match path {
                        PathState::Open(from) => from,
                        PathState::Closed => range.start,
                    };
                    top.keep(Kind::Ordered, from..ends)?;
                }
                clip_pending = false;
                path = PathState::Closed;
            }
            // §9.3.1's text state parameters, which a text object leaves in force after it.
            Operator::CharSpacing => top.keep(Kind::Text(0), range)?,
            Operator::WordSpacing => top.keep(Kind::Text(1), range)?,
            Operator::HorizontalScaling => top.keep(Kind::Text(2), range)?,
            Operator::Leading => top.keep(Kind::Text(3), range)?,
            Operator::Font => top.keep(Kind::Text(4), range)?,
            Operator::RenderMode => top.keep(Kind::Text(5), range)?,
            Operator::Rise => top.keep(Kind::Text(6), range)?,
            // A text object is the last piece's (ADR 1880 §2): no cut follows the first one, so
            // what the object's own operators leave behind never reaches a spine.
            Operator::BeginText => {
                first_text.get_or_insert(range.start);
            }
            // A shading, an XObject and a text object's own operators leave no state behind: §8.10.1's
            // `Do` restores what it saved. A marked-content point opens no section.
            Operator::MarkPoint
            | Operator::MarkPointProperties
            | Operator::Shade
            | Operator::XObject
            | Operator::EndText
            | Operator::MoveText
            | Operator::MoveTextSetLeading
            | Operator::TextMatrix
            | Operator::NextLine
            | Operator::ShowText
            | Operator::ShowTextAdjusted
            | Operator::NextLineShowText => {}
            // An inline image's data is not tokens, and the window reads it through a lookahead a
            // whole buffer does not have (`LOOKAHEAD`), so the page goes back to the window.
            Operator::BeginImage | Operator::ImageData | Operator::EndImage => {
                return Err(Refusal::Lexing(Declined(beyond_the_join(operator))));
            }
            // §14.6's marked-content sections. A cut may fall inside one: the next piece's spine
            // opens it again, and a piece declines where one it leaves open records anything but
            // whether it hides (`Leftover::only_plain_sections_open`).
            Operator::BeginMarked | Operator::BeginMarkedProperties => {
                effect.opens.push(range);
            }
            Operator::EndMarked => {
                if effect.opens.pop().is_none() {
                    effect.closes = effect.closes.saturating_add(1);
                }
            }
            other => return Err(Refusal::Joining(Declined(beyond_the_join(other)))),
        }
    }
    Ok(Chunk {
        effect,
        deepest,
        candidates,
        ended,
        first_text,
        closes_on_a_q: closed_on_a_q && ended == (stop, true),
    })
}

/// Steps over an inline dictionary after its `<<`, as `reader::inline_dictionary` reads it, and
/// answers where it ended — or declines one that reader would read otherwise: a key that is not a
/// name, a value missing, a nesting past its bound, an array past its count.
fn step_over_dictionary(reader: &mut ContentReader<'_>, depth: usize) -> Result<usize, Refusal> {
    /// The same for an array after its `[`, inside a property list.
    fn step_over_array(
        reader: &mut ContentReader<'_>,
        depth: usize,
        step: &mut impl FnMut(&mut ContentReader<'_>) -> Result<Step, Refusal>,
    ) -> Result<(), Refusal> {
        let malformed = || {
            Refusal::Joining(Declined(
                "an inline array whose shape the property-list reader reads otherwise",
            ))
        };
        if depth > MAX_DEPTH {
            return Err(malformed());
        }
        for _ in 0..MAX_ELEMENTS {
            match step(reader)? {
                Step::ArrayClose => return Ok(()),
                Step::ArrayOpen => step_over_array(reader, depth.saturating_add(1), step)?,
                Step::Open => {
                    step_over_dictionary(reader, depth.saturating_add(1))?;
                }
                Step::Name | Step::Other => {}
                Step::Close | Step::End => return Err(malformed()),
            }
        }
        Err(malformed())
    }

    /// `inline_dictionary`'s own bounds: past them it reads a shape this walk does not follow.
    const MAX_DEPTH: usize = 8;
    const MAX_ELEMENTS: usize = 65_536;
    /// One token, for the walk.
    enum Step {
        Open,
        Close,
        ArrayOpen,
        ArrayClose,
        Name,
        Other,
        End,
    }
    let malformed = || {
        Refusal::Joining(Declined(
            "an inline dictionary whose shape the property-list reader reads otherwise",
        ))
    };
    let mut step = |reader: &mut ContentReader<'_>| -> Result<Step, Refusal> {
        let begins = reader.next_token_start().ok_or_else(malformed)?;
        let step = reader.with_token(|token| match token {
            None => Step::End,
            Some(Token::DictOpen) => Step::Open,
            Some(Token::DictClose) => Step::Close,
            Some(Token::ArrayOpen) => Step::ArrayOpen,
            Some(Token::ArrayClose) => Step::ArrayClose,
            Some(Token::Name(_)) => Step::Name,
            Some(_) => Step::Other,
        });
        let ends = reader.position().unwrap_or(begins);
        if ends.saturating_sub(begins) > CEILING {
            return Err(Refusal::Lexing(Declined(
                "a token longer than the window's ceiling",
            )));
        }
        Ok(step)
    };
    if depth > MAX_DEPTH {
        return Err(malformed());
    }
    loop {
        match step(reader)? {
            Step::Close => return reader.position().ok_or_else(malformed),
            Step::Name => {}
            _ => return Err(malformed()),
        }
        match step(reader)? {
            Step::Open => {
                step_over_dictionary(reader, depth.saturating_add(1))?;
            }
            Step::ArrayOpen => step_over_array(reader, depth.saturating_add(1), &mut step)?,
            Step::Name | Step::Other => {}
            Step::Close | Step::ArrayClose | Step::End => return Err(malformed()),
        }
    }
}

/// Why an operator the plan meets declines the division: the state it sets, or the span it opens,
/// is one the join does not carry.
const fn beyond_the_join(operator: Operator) -> &'static str {
    match operator {
        Operator::BeginImage | Operator::ImageData | Operator::EndImage => {
            "an inline image, whose data the plan does not step over"
        }
        Operator::BeginCompatibility | Operator::EndCompatibility => "a compatibility section",
        Operator::GlyphWidth | Operator::GlyphWidthAndBox => "a glyph description's operator",
        _ => "a text operator outside a text object, whose effect the plan does not compose",
    }
}

/// What the last piece read back of the page's text, moved whole into the page's interpreter —
/// whose own is empty, since no piece before the last holds a text object.
struct Readback {
    text: String,
    text_layer: Vec<super::Placed>,
    word_gaps: super::text::WordGaps,
    glyph_coverage: std::collections::BTreeMap<String, super::Coverage>,
    glyphs: usize,
    codes_without_a_glyph: usize,
    codes_reaching_a_blank_glyph: usize,
    codes_without_a_vertical_form: usize,
    codes_without_a_character: super::report::UnnamedCodes,
    inferred_separators: usize,
    reversed_chars: usize,
    text_cursor: Option<(f32, f32)>,
    text_operations: usize,
}

impl Readback {
    /// The readback `piece` built, taken out of it.
    fn taken_from(piece: &mut Interpreter<'_>) -> Self {
        Self {
            text: std::mem::take(&mut piece.text),
            text_layer: std::mem::take(&mut piece.text_layer),
            word_gaps: std::mem::take(&mut piece.word_gaps),
            glyph_coverage: std::mem::take(&mut piece.glyph_coverage),
            glyphs: piece.glyphs,
            codes_without_a_glyph: piece.codes_without_a_glyph,
            codes_reaching_a_blank_glyph: piece.codes_reaching_a_blank_glyph,
            codes_without_a_vertical_form: piece.codes_without_a_vertical_form,
            codes_without_a_character: std::mem::take(&mut piece.codes_without_a_character),
            inferred_separators: piece.inferred_separators,
            reversed_chars: piece.reversed_chars,
            text_cursor: piece.text_cursor,
            text_operations: piece.text_operations,
        }
    }
}

/// One piece's reading, handed back from its thread.
struct PieceRun {
    /// The last piece's readback, which is the page's: every text object is the last piece's.
    text: Option<Readback>,
    /// §11.6.4.3's record as the piece left it.
    readings: super::Readings,
    list: pdf_render::DisplayList,
    /// The spans of the sections the piece closed, in the order it closed them (ADR 1894).
    described: Vec<crate::accessibility::Described>,
    artifacts: Vec<super::ArtifactSpan>,
    marked: Vec<super::MarkedSpan>,
    associated: Vec<(Range<usize>, crate::attachment::Attachment)>,
    /// The extents of sections with an `/MCID` still open, which only the last piece may leave.
    marking: Vec<super::marked::OpenSequence>,
    /// How many hiding sections were open at the piece's end.
    hidden: usize,
    seeded: usize,
    notes: super::report::Notes,
    notes_raised: usize,
    operations: usize,
    segments_without_a_current_point: usize,
    reach_scanned: usize,
    budget: super::list_budget::ListBudget,
    view_dependent: bool,
    magnified_tiling: bool,
}

impl Interpreter<'_> {
    /// Reads `bytes` in the plan's pieces at once and joins them into this interpreter, or says
    /// why not — in which case this interpreter is the caller's to restore.
    fn read_divided(
        &mut self,
        bytes: &[u8],
        plan: &Plan,
        inputs: &Inputs<'_>,
        initial: &GraphicsState,
    ) -> Result<(), Declined> {
        let count = plan.cuts.len().saturating_sub(1);
        let mut pieces = plan.pieces();
        let (first, _) = pieces.next().ok_or(Declined("a plan without a piece"))?;
        let rest: Vec<(usize, Range<usize>, &[u8])> = pieces
            .enumerate()
            .map(|(index, (range, spine))| (index.saturating_add(1), range, spine))
            .collect();
        let readings = self.readings;
        self.list_budget = super::list_budget::ListBudget::for_piece(count, 0);
        let (send, arrived) = std::sync::mpsc::channel();
        rayon::in_place_scope(|scope| {
            for (number, range, spine) in &rest {
                let (send, range, number) = (send.clone(), range.clone(), *number);
                let last = number.saturating_add(1) == count;
                scope.spawn(move |_| {
                    let piece = bytes.get(range.clone()).unwrap_or_default();
                    let run =
                        run_piece(inputs, piece, range.start, bytes.len(), spine, count, last);
                    // A receiver gone is a division already given up, which wants no answer.
                    let _ = send.send((number, run));
                });
            }
            drop(send);
            let mut reader = ContentReader::piece(
                bytes.get(first.clone()).unwrap_or_default(),
                first.start,
                bytes.len(),
            );
            let left = self
                .run_from(
                    &mut reader,
                    &inputs.page.resources,
                    Composed::fresh(initial),
                )
                .ok_or(Declined("the list's bound stopped the first piece"))?;
            if !left.only_plain_sections_open() {
                return Err(Declined(
                    "the first piece ended with more open than the state and plain sections",
                ));
            }
            if let Some(reason) = self.outside_the_division(readings, false) {
                return Err(Declined(reason));
            }
            self.settle_list_charge();
            // The pieces are joined in order as they arrive, so that copying one into the page's
            // list runs beside the pieces still being read rather than after the last of them.
            let mut waiting = std::collections::BTreeMap::new();
            let mut next = 1;
            for (number, run) in arrived {
                waiting.insert(number, run);
                while let Some(run) = waiting.remove(&next) {
                    self.join(run?)?;
                    next = next.saturating_add(1);
                }
            }
            if next == count {
                Ok(())
            } else {
                Err(Declined("a piece did not run"))
            }
        })?;
        if self.reach_scanned >= Self::MAX_REACH_SCAN {
            return Err(Declined(
                "the pieces together scanned as far as the page's reach bound",
            ));
        }
        self.note_alpha_source(initial.alpha_is_shape);
        Ok(())
    }

    /// Empties the memos a reading given up left behind, so that the page read again reports
    /// what its own reading meets: a font whose load failed is remembered as failed, and the
    /// remembered failure says nothing the second time (`font.rs`'s `cached_font`), so a report the
    /// given-up reading made and [`Interpreter::restore`] took back would otherwise be lost.
    fn forget_memos(&mut self) {
        drop(std::mem::take(&mut self.fonts));
        drop(std::mem::take(&mut self.shadings));
        drop(std::mem::take(&mut self.image_masks));
        drop(std::mem::take(&mut self.image_rasters));
        drop(std::mem::take(&mut self.icc_spaces));
        drop(std::mem::take(&mut self.resource_tables));
        drop(std::mem::take(&mut self.stream_structures));
        self.clip_extents.clear();
    }

    /// Takes the next piece's reading into this one, which read every piece before it (ADR 1880
    /// §2).
    fn join(&mut self, mut run: PieceRun) -> Result<(), Declined> {
        let (map, returned) = self.take_clips(&run)?;
        let mut commands = run.list.split_off_commands(0);
        if map
            .iter()
            .enumerate()
            .any(|(index, id)| id.index() != index)
        {
            for command in &mut commands {
                renumber(command, &map);
            }
        }
        self.list.append(vec![commands]);
        if run.list.overprints() {
            self.list.note_overprinting();
        }
        self.unsupported.absorb(run.notes);
        self.notes_raised = self.notes_raised.saturating_add(run.notes_raised);
        self.operations = self.operations.saturating_add(run.operations);
        self.segments_without_a_current_point = self
            .segments_without_a_current_point
            .saturating_add(run.segments_without_a_current_point);
        self.reach_scanned = self.reach_scanned.saturating_add(run.reach_scanned);
        self.view_dependent |= run.view_dependent;
        self.magnified_tiling |= run.magnified_tiling;
        self.readings.absorb_piece(run.readings);
        self.described.append(&mut run.described);
        self.artifacts.append(&mut run.artifacts);
        self.marked.append(&mut run.marked);
        self.associated.append(&mut run.associated);
        self.marking.append(&mut run.marking);
        self.hidden = run.hidden;
        if let Some(readback) = run.text {
            self.text = readback.text;
            self.text_layer = readback.text_layer;
            self.word_gaps = readback.word_gaps;
            self.glyph_coverage = readback.glyph_coverage;
            self.glyphs = readback.glyphs;
            self.codes_without_a_glyph = readback.codes_without_a_glyph;
            self.codes_reaching_a_blank_glyph = readback.codes_reaching_a_blank_glyph;
            self.codes_without_a_vertical_form = readback.codes_without_a_vertical_form;
            self.codes_without_a_character = readback.codes_without_a_character;
            self.inferred_separators = readback.inferred_separators;
            self.reversed_chars = readback.reversed_chars;
            self.text_cursor = readback.text_cursor;
            self.text_operations = readback.text_operations;
        }
        self.list_budget.absorb(
            run.budget,
            returned,
            self.list.command_count(),
            self.list.clip_count(),
        );
        Ok(())
    }

    /// Takes a later piece's clips into the page's table, so that each lands where the page's
    /// own reading would have put it: a region the table holds already keeps its identifier.
    /// Answers the piece's map from its own identifiers to the page's, and the bytes of its own
    /// clips the table already held, which its charge gives back.
    fn take_clips(&mut self, run: &PieceRun) -> Result<(Vec<ClipId>, usize), Declined> {
        let mut map: Vec<ClipId> = Vec::with_capacity(run.list.clip_count());
        let mut returned = 0usize;
        for index in 0..run.list.clip_count() {
            let id = u32::try_from(index)
                .ok()
                .map(ClipId::new)
                .and_then(|id| run.list.clip(id))
                .ok_or(Declined("a clip outside its piece's table"))?;
            let mut clip = id.clone();
            clip.parent = match clip.parent {
                Some(parent) => Some(
                    *map.get(parent.index())
                        .ok_or(Declined("a clip whose parent follows it"))?,
                ),
                None => None,
            };
            let before = self.list.clip_count();
            let joined = self
                .list
                .add_clip(clip.clone())
                .map_err(|_| Declined("the page's clip table is full"))?;
            if index >= run.seeded && joined.index() < before {
                returned = returned.saturating_add(super::list_budget::clip_bytes(&clip));
            }
            map.push(joined);
        }
        Ok((map, returned))
    }

    /// Why this piece's reading cannot be joined, where it cannot: page-level state its reading
    /// set that the join does not carry (ADR 1880 §3).
    fn outside_the_division(
        &self,
        readings: super::Readings,
        text_allowed: bool,
    ) -> Option<&'static str> {
        if !text_allowed
            && (!self.text.is_empty()
                || !self.text_layer.is_empty()
                || self.text_cursor.is_some()
                || self.glyphs > 0
                || !self.glyph_coverage.is_empty()
                || self.inferred_separators > 0
                || self.reversed_chars > 0
                || self.text_operations > 0)
        {
            return Some("a piece before the last showed text, whose readback is the last piece's");
        }
        // A closed section's spans are offsets into the readback, which is empty before the
        // last piece in the page's reading as in the piece's, so they join in order; one that
        // records an extent and is still open does not (ADR 1894). `clip_extents` is a memo of
        // the piece's own clip table, and is left behind.
        if !self.structural_annotations.is_empty() || (!text_allowed && !self.marking.is_empty()) {
            return Some(
                "a piece left a sequence with an /MCID open, whose extent the join does not carry",
            );
        }
        if self.list.soft_mask_count() > 0 {
            return Some("a piece set a soft mask, whose table the join does not carry");
        }
        if self.transfers.is_live() || !self.transfer_maps.is_empty() {
            return Some("a piece met a transfer function, whose channel the join does not carry");
        }
        if !self.readings.carried_by_join(readings) {
            return Some("a piece changed §11.6.4.3's record of the page's alpha readings");
        }
        if self.blending_changed || self.black_generation_painted || self.nested_space_departed {
            return Some("a piece changed the page's compositing record");
        }
        match self.presses.named() {
            Err(_) => return Some("a piece's press table was poisoned; its count is unknown"),
            Ok(0) => {}
            Ok(_) => return Some("a piece named a press, whose table is filled in reading order"),
        }
        if self.list_budget.reached() {
            return Some("a piece reached its share of the list's bound");
        }
        None
    }
}

/// Reads one piece on a thread of its own: its state composed from the spine, then its bytes.
fn run_piece(
    inputs: &Inputs<'_>,
    piece_bytes: &[u8],
    from: usize,
    of: usize,
    spine: &[u8],
    count: usize,
    last: bool,
) -> Result<PieceRun, Declined> {
    let fresh = || {
        Interpreter::for_page(
            inputs.document,
            inputs.page,
            inputs.state,
            Compositing::Device,
            inputs.presses,
            inputs.fonts,
            inputs.references,
            None,
        )
    };
    let resources: &Dictionary = &inputs.page.resources;
    // The page's own initial state, made here rather than handed over: a graphics state holds a
    // pattern by `Rc` and stays on the thread that made it.
    let initial = GraphicsState::initial(super::base_transform(inputs.page));
    let initial = &initial;
    let mut scratch = fresh();
    let composed = if spine.is_empty() {
        Composed::fresh(initial)
    } else {
        let mut reader = ContentReader::over(spine);
        let left = scratch
            .run_from(&mut reader, resources, Composed::fresh(initial))
            .ok_or(Declined("the list's bound stopped a spine"))?;
        if !left.only_plain_sections_open() {
            return Err(Declined(
                "a spine ended with more open than the state and plain sections",
            ));
        }
        left.into_composed()
    };
    if scratch.list.soft_mask_count() > 0 {
        return Err(Declined("a spine set a soft mask"));
    }
    let mut piece = fresh();
    // What the spine's statements reported is the page's to say: they are the producer's own, run
    // in the page's order, and a font they loaded first reports its defect here or nowhere, since
    // the cache across pieces loads it once. The notes are a set, so a report an earlier piece made
    // too is said once; `notes_raised` stays the piece's, which only brackets its own sections.
    piece
        .unsupported
        .absorb(std::mem::take(&mut scratch.unsupported));
    let readings = piece.readings;
    let seeded = scratch.list.clip_count();
    for index in 0..seeded {
        let clip = u32::try_from(index)
            .ok()
            .map(ClipId::new)
            .and_then(|id| scratch.list.clip(id))
            .ok_or(Declined("a spine's clip outside its table"))?
            .clone();
        let id = piece
            .list
            .add_clip(clip)
            .map_err(|_| Declined("a piece's clip table is full"))?;
        if id.index() != index {
            return Err(Declined(
                "a spine's clips did not seed the piece's table in place",
            ));
        }
    }
    piece.list_budget = super::list_budget::ListBudget::for_piece(count, seeded);
    piece.hidden = composed.hiding();
    let mut reader = ContentReader::piece(piece_bytes, from, of);
    let leftover = piece
        .run_from(&mut reader, resources, composed)
        .ok_or(Declined("the list's bound stopped a piece"))?;
    if last {
        // The stream's end is the last piece's, and what it says is said here; the `/AIS`
        // reading the page began under is put back on the joined list, which is the page's.
        piece.report_leftover(leftover);
    } else if !leftover.only_plain_sections_open() {
        return Err(Declined(
            "a piece ended with more open than the state and plain sections",
        ));
    }
    if let Some(reason) = piece.outside_the_division(readings, last) {
        return Err(Declined(reason));
    }
    // What the piece's list gained after its last check is its charge too: the page's own reading
    // would have charged it at the check after, which falls in the next piece's operators.
    piece.settle_list_charge();
    let text = last.then(|| Readback::taken_from(&mut piece));
    Ok(PieceRun {
        list: piece.list,
        described: piece.described,
        artifacts: piece.artifacts,
        marked: piece.marked,
        associated: piece.associated,
        marking: piece.marking,
        hidden: piece.hidden,
        seeded,
        notes: piece.unsupported,
        notes_raised: piece.notes_raised,
        operations: piece.operations,
        segments_without_a_current_point: piece.segments_without_a_current_point,
        reach_scanned: piece.reach_scanned,
        budget: piece.list_budget,
        view_dependent: piece.view_dependent,
        magnified_tiling: piece.magnified_tiling,
        readings: piece.readings,
        text,
    })
}

/// Names every clip a command holds by its identifier in the joined table.
fn renumber(command: &mut Command, map: &[ClipId]) {
    let joined = |clip: &mut Option<ClipId>| {
        if let Some(id) = *clip
            && let Some(&to) = map.get(id.index())
        {
            *clip = Some(to);
        }
    };
    match command {
        Command::Group { commands, clip, .. } => {
            joined(clip);
            for inner in commands {
                renumber(inner, map);
            }
        }
        // Both halves state the same clip, and each is renumbered once.
        Command::Shaped { object, shape } => {
            renumber(object, map);
            renumber(shape, map);
        }
        Command::Fill { clip, .. } | Command::Stroke { clip, .. } | Command::Image { clip, .. } => {
            joined(clip);
        }
        // A kind added later answers for its clip through the command's own accessors.
        other => {
            let mut clip = other.clip();
            joined(&mut clip);
            other.set_clip(clip);
        }
    }
}
