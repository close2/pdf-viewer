//! The divided read with the inflate pipelined into it (ADR 1888 §4, ADR 1894).
//!
//! [`super::after_the_inflate`] waits for the whole stream before it plans a cut, so the pieces
//! start only after the inflate. Here three kinds of work overlap instead:
//!
//! 1. **The producer** inflates the stream on one of the pool's threads, writing in place, and
//!    hands it on in *segments*. Each segment ends just after a `Q` that a byte scan found near a
//!    target: white space before it, and white space or a delimiter after it (§7.2.3). The scan
//!    is a guess, because a `Q` inside a string or a comment looks the same.
//! 2. **Each segment is lexed** as soon as it is out ([`super::lex_chunk`]). The lex records the
//!    segment's effect on the state stack and confirms the guessed cut at its end: its last token
//!    must be a `Q` operator ending exactly there, with no path or clip open. The effects are
//!    composed in order, and a later piece's state is composed from them, as in ADR 1880.
//! 3. **Each piece is read** once its segment is out and every effect before it is composed, so
//!    a piece starts while the inflate is still producing the segments after it.
//!
//! The page's own interpreter reads the first piece on the calling thread, so that ADR 1855's
//! hand-over still leaves from the thread the launch placed the read on (trap 154). It then joins
//! the later pieces in order as they arrive.
//!
//! **A failed guess moves the cut, never the list.** A cut is joined across only once the lex of
//! the segment before it has confirmed it. A cut that is not confirmed, or anything else the
//! pipeline does not settle, sends the page back whole to [`super::after_the_inflate`]. The page's
//! interpreter is first restored to where it stood before the first piece. That fallback plans
//! the stream exactly, so it divides the page wherever ADR 1880's plan does. A text object ends
//! the cuts, as there (every text object is the last piece's): the last piece then runs from the
//! segment that holds the first `BT` to the end of the stream.

use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};

use super::{
    Chunk, Declined, GraphicsState, Inputs, Interpreter, MAX_STATE_DEPTH, PIECE_FLOOR,
    PIECES_PER_THREAD, PieceRun, Refusal, Stack, lex_chunk, run_piece, spine_of,
};
use crate::content::reader::ContentReader;
use crate::content::run::Composed;

/// How many bytes past a target the producer inflates at a time while it looks for a cut.
///
/// The witness states a `Q` about every 63 bytes, so the first step almost always finds one. The
/// step only bounds how far a stream with no `Q` near the target is inflated ahead of the scan.
const SCAN_STEP: usize = 64 << 10;

/// How the pipelined read ended.
pub(super) enum Outcome {
    /// Read in pieces and joined: the page's list is in the interpreter.
    Divided,
    /// The stream inflated past its bound, which reads the page through its window again.
    PastBound,
    /// A segment holds what a window reads differently from a whole buffer (ADR 1880 §2.4), so
    /// the page goes back to the window.
    Windowed(Declined),
    /// The stream is whole in memory and the pipeline did not settle it, for the reason given; the
    /// page is divided after the inflate instead.
    Inflated(Vec<u8>, Declined),
}

/// A run of the stream's bytes as the producer handed it on.
struct Segment {
    /// Where it begins in the stream.
    from: usize,
    /// Its bytes, shared between the lex, the piece and the fallback.
    bytes: Arc<Vec<u8>>,
}

/// What the calling thread hears from the pool.
enum Event {
    /// The first segment, and the producer's plan: how many pieces at most, and how long it
    /// estimates the stream to be.
    First {
        bytes: Arc<Vec<u8>>,
        planned: usize,
        estimated: usize,
        last: bool,
    },
    /// A segment's effect, composed in order: whether its cut is confirmed and whether it holds
    /// a text object, or why the pipeline cannot use it.
    Folded {
        index: usize,
        folded: Result<Fold, Refusal>,
    },
    /// A later piece's reading.
    Ran {
        index: usize,
        run: Result<Box<PieceRun>, Declined>,
    },
    /// The producer is done: how many segments, or `None` where the stream passed its bound.
    Produced(Option<usize>),
}

/// What composing one segment's effect found.
#[derive(Debug, Clone, Copy)]
struct Fold {
    /// Whether the lex confirmed the cut at the segment's end.
    confirmed: bool,
    /// Whether the segment holds a text object.
    text: bool,
}

/// The order-dependent half of the pipeline, shared between the producer and the lexes: the
/// segments out so far, the effects composed so far, and which pieces have been started.
struct Composer {
    segments: Vec<Segment>,
    /// The segment the producer said was the last, once it has.
    last: Option<usize>,
    /// Lexed segments waiting for the ones before them.
    waiting: BTreeMap<usize, Result<Chunk, Refusal>>,
    /// The state stack after every segment composed so far.
    stack: Stack,
    /// How many segments have been composed.
    folded: usize,
    /// The spine each piece begins from, for every piece up to the first text object.
    spines: Vec<Arc<Vec<u8>>>,
    /// The first segment holding a text object, once composed.
    text_at: Option<usize>,
    /// The next piece to start.
    started: usize,
    /// Set once a composition fails, after which nothing more is composed or started.
    broken: bool,
}

/// A piece ready to start.
struct Ready {
    index: usize,
    from: usize,
    bytes: Arc<Vec<u8>>,
    spine: Arc<Vec<u8>>,
    last: bool,
}

impl Composer {
    fn new() -> Self {
        Self {
            segments: Vec::new(),
            last: None,
            waiting: BTreeMap::new(),
            stack: Stack::fresh(),
            folded: 0,
            spines: vec![Arc::new(Vec::new())],
            text_at: None,
            started: 1,
            broken: false,
        }
    }

    /// The bytes of `range`, from the segment that holds them.
    ///
    /// A kept statement never spans two segments: it ends at an operator, and every segment
    /// whose effect is composed ends on a confirmed `Q`.
    fn statement(&self, range: Range<usize>) -> &[u8] {
        let index = self
            .segments
            .partition_point(|segment| segment.from <= range.start)
            .saturating_sub(1);
        self.segments
            .get(index)
            .and_then(|segment| {
                let start = range.start.checked_sub(segment.from)?;
                let end = range.end.checked_sub(segment.from)?;
                segment.bytes.get(start..end)
            })
            .unwrap_or_default()
    }

    /// Composes every lexed segment whose predecessors are composed, telling `events` what each
    /// found — from under the lock, so that they arrive in the stream's order — and answers the
    /// pieces now ready to start.
    fn advance(&mut self, events: &mpsc::Sender<Event>) -> Vec<Ready> {
        while !self.broken
            && let Some(lexed) = self.waiting.remove(&self.folded)
        {
            let index = self.folded;
            let folded = self.fold(index, lexed);
            if !matches!(
                folded,
                Ok(Fold {
                    confirmed: true,
                    ..
                })
            ) {
                self.broken = true;
            }
            // A receiver gone is a read already given up, which wants no answer.
            let _ = events.send(Event::Folded { index, folded });
        }
        self.ready()
    }

    /// Whether a text object stops piece `index` from starting on its own segment: one in an
    /// earlier segment, or in this one where it is not the last (ADR 1880 §2.2).
    fn text_stops(&self, index: usize) -> bool {
        self.text_at
            .is_some_and(|text| text < index || (text == index && self.last != Some(index)))
    }

    /// Composes segment `index`'s effect onto the stack.
    fn fold(&mut self, index: usize, lexed: Result<Chunk, Refusal>) -> Result<Fold, Refusal> {
        let chunk = lexed?;
        let last = self.last == Some(index);
        let fold = Fold {
            confirmed: chunk.closes_on_a_q || last,
            text: chunk.first_text.is_some(),
        };
        if !fold.confirmed {
            return Ok(fold);
        }
        if self
            .stack
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
        self.stack = chunk.effect.after(&self.stack)?;
        self.folded = self.folded.saturating_add(1);
        if fold.text && self.text_at.is_none() {
            self.text_at = Some(index);
        }
        if self.text_at.is_none() && !last {
            let spine = spine_of(&self.stack, |range| self.statement(range));
            self.spines.push(Arc::new(spine));
        }
        Ok(fold)
    }

    /// The pieces whose segment is out and whose spine is composed, up to the first text object.
    fn ready(&mut self) -> Vec<Ready> {
        let mut ready = Vec::new();
        while !self.broken
            && !self.text_stops(self.started)
            && let (Some(segment), Some(spine)) = (
                self.segments.get(self.started),
                self.spines.get(self.started),
            )
        {
            ready.push(Ready {
                index: self.started,
                from: segment.from,
                bytes: Arc::clone(&segment.bytes),
                spine: Arc::clone(spine),
                last: self.last == Some(self.started),
            });
            self.started = self.started.saturating_add(1);
        }
        ready
    }
}

/// Everything a task on the pool needs, borrowed for the scope.
struct Shared<'s, 'i> {
    inputs: &'s Inputs<'i>,
    composer: Mutex<Composer>,
    events: mpsc::Sender<Event>,
    /// Set once the calling thread has given the pipeline up, so a piece not yet begun is not.
    abandoned: AtomicBool,
    /// The pieces the producer may make at most, which sizes each one's share of the list's
    /// bound; set before the first piece starts.
    planned: std::sync::OnceLock<usize>,
}

impl Shared<'_, '_> {
    fn composer(&self) -> std::sync::MutexGuard<'_, Composer> {
        // A lex or a piece that panicked has already lost the read; its panic is the scope's.
        self.composer.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn send(&self, event: Event) {
        // A receiver gone is a read already given up, which wants no answer.
        let _ = self.events.send(event);
    }

    /// Starts the pieces composing made ready.
    fn start<'scope>(&'scope self, scope: &rayon::Scope<'scope>, ready: Vec<Ready>) {
        for piece in ready {
            scope.spawn(move |_| self.run(&piece));
        }
    }

    /// Lexes a segment just out, and composes whatever it completes.
    fn lex<'scope>(&'scope self, scope: &rayon::Scope<'scope>, index: usize, segment: &Segment) {
        if self.abandoned.load(Ordering::Relaxed) {
            return;
        }
        let lexed = lex_chunk(&segment.bytes, 0, segment.bytes.len(), &[]).map(|mut chunk| {
            chunk.effect.shift(segment.from);
            chunk
        });
        let ready = {
            let mut composer = self.composer();
            composer.waiting.insert(index, lexed);
            composer.advance(&self.events)
        };
        self.start(scope, ready);
    }

    /// Reads one later piece.
    fn run(&self, piece: &Ready) {
        if self.abandoned.load(Ordering::Relaxed) {
            return;
        }
        let planned = self.planned.get().copied().unwrap_or(1);
        let of = piece.from.saturating_add(piece.bytes.len());
        let run = run_piece(
            self.inputs,
            &piece.bytes,
            piece.from,
            of,
            &piece.spine,
            planned,
            piece.last,
        );
        self.send(Event::Ran {
            index: piece.index,
            run: run.map(Box::new),
        });
    }

    /// Takes a segment the producer has just handed on.
    fn arrive<'scope>(
        &'scope self,
        scope: &rayon::Scope<'scope>,
        index: usize,
        segment: Segment,
        last: bool,
    ) {
        let lexed = Segment {
            from: segment.from,
            bytes: Arc::clone(&segment.bytes),
        };
        let ready = {
            let mut composer = self.composer();
            composer.segments.push(segment);
            if last {
                composer.last = Some(index);
            }
            composer.advance(&self.events)
        };
        self.start(scope, ready);
        scope.spawn(move |scope| self.lex(scope, index, &lexed));
    }
}

/// What the producer was asked for and found while it inflated.
struct Producer<'p, 'r> {
    reader: &'p mut ContentReader<'r>,
    /// The bound on the whole stream, in bytes.
    most: usize,
    /// How many pieces a caller asked for, or `None` for four a pool thread.
    asked: Option<usize>,
}

impl Producer<'_, '_> {
    /// Inflates into `segment` until it holds `upto` bytes, or the stream ends; `Err` where the
    /// stream, `from` bytes in before the segment, has passed its bound.
    fn fill(&mut self, segment: &mut Vec<u8>, upto: usize, from: usize) -> Result<bool, ()> {
        let room = self.most.saturating_sub(from).saturating_add(1);
        let upto = upto.min(room);
        if upto > segment.capacity() {
            segment.reserve(upto.saturating_sub(segment.len()));
        }
        let more = self.reader.extend_within(segment, upto);
        if from.saturating_add(segment.len()) > self.most {
            return Err(());
        }
        Ok(more)
    }

    /// The stream's length as the inflate so far predicts it: what came out, scaled by how much
    /// of the encoded bytes went in.
    fn estimate(&self, produced: usize, more: bool) -> usize {
        match self.reader.progress() {
            Some((taken, held)) if more && taken > 0 => produced
                .saturating_mul(held)
                .checked_div(taken)
                .unwrap_or(produced)
                .max(produced),
            _ => produced,
        }
    }

    /// Inflates the stream and hands it on in segments; answers how many, or `None` where it
    /// passed its bound.
    fn produce<'scope>(
        &mut self,
        shared: &'scope Shared<'scope, '_>,
        scope: &rayon::Scope<'scope>,
    ) -> Option<usize> {
        let mut segment = Vec::new();
        let mut more = self.fill(&mut segment, PIECE_FLOOR, 0).ok()?;
        let mut estimated = self.estimate(segment.len(), more);
        let (planned, floor) = match self.asked {
            Some(pieces) => (pieces.max(1), 1),
            None => (
                rayon::current_num_threads()
                    .saturating_mul(PIECES_PER_THREAD)
                    .min(estimated.checked_div(PIECE_FLOOR).unwrap_or(0))
                    .max(1),
                PIECE_FLOOR,
            ),
        };
        let _ = shared.planned.set(planned);
        let mut from = 0_usize;
        let mut index = 0_usize;
        while index.saturating_add(1) < planned {
            let left = planned.saturating_sub(index);
            let target = estimated
                .saturating_sub(from)
                .checked_div(left)
                .unwrap_or(0)
                .max(floor);
            // Room for the scan past the target as well, so finding the cut moves no bytes.
            segment.reserve(
                target
                    .saturating_add(SCAN_STEP)
                    .saturating_sub(segment.len()),
            );
            if more && segment.len() < target {
                more = self.fill(&mut segment, target, from).ok()?;
            }
            let mut scanned = target.min(segment.len());
            let cut = loop {
                if let Some(cut) = cut_after(&segment, scanned) {
                    break Some(cut);
                }
                if !more {
                    break None;
                }
                scanned = segment.len().saturating_sub(1);
                let upto = segment.len().saturating_add(SCAN_STEP);
                more = self.fill(&mut segment, upto, from).ok()?;
            };
            let Some(cut) = cut else {
                break;
            };
            let rest = segment.split_off(cut);
            let length = segment.len();
            hand_on(
                shared, scope, index, from, segment, false, planned, estimated,
            );
            from = from.saturating_add(length);
            index = index.saturating_add(1);
            segment = rest;
            if more {
                estimated = self
                    .estimate(from.saturating_add(segment.len()), more)
                    .max(estimated);
            }
        }
        while more {
            let upto = segment.len().saturating_mul(2).max(PIECE_FLOOR);
            more = self.fill(&mut segment, upto, from).ok()?;
        }
        hand_on(
            shared, scope, index, from, segment, true, planned, estimated,
        );
        Some(index.saturating_add(1))
    }
}

/// Hands one segment on: to the calling thread where it is the first, and to the composer.
#[expect(
    clippy::too_many_arguments,
    reason = "the segment and the plan it was cut under, passed once from the one loop that \
              makes them"
)]
fn hand_on<'scope>(
    shared: &'scope Shared<'scope, '_>,
    scope: &rayon::Scope<'scope>,
    index: usize,
    from: usize,
    bytes: Vec<u8>,
    last: bool,
    planned: usize,
    estimated: usize,
) {
    let bytes = Arc::new(bytes);
    if index == 0 {
        shared.send(Event::First {
            bytes: Arc::clone(&bytes),
            planned,
            estimated,
            last,
        });
    }
    shared.arrive(scope, index, Segment { from, bytes }, last);
}

/// The offset just after the first `Q` at or after `from` that stands between delimiters —
/// white space before it, and white space or a delimiter after it (§7.2.3) — or `None`.
fn cut_after(bytes: &[u8], from: usize) -> Option<usize> {
    let white = |byte: u8| matches!(byte, b'\0' | b'\t' | b'\n' | b'\x0C' | b'\r' | b' ');
    let delimiter = |byte: u8| {
        white(byte)
            || matches!(
                byte,
                b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
            )
    };
    let mut at = from.max(1);
    loop {
        let found = bytes.get(at..)?.iter().position(|&byte| byte == b'Q')?;
        at = at.saturating_add(found);
        let before = bytes.get(at.checked_sub(1)?).copied()?;
        let after = bytes.get(at.saturating_add(1)).copied()?;
        if white(before) && delimiter(after) {
            return Some(at.saturating_add(1));
        }
        at = at.saturating_add(1);
    }
}

/// The next event, waited for.
///
/// A calling thread that is one of the pool's own runs the pool's work while it waits rather than
/// blocking: the producer and the pieces may be queued on this very thread, and a pool whose every
/// thread waits for the work queued on it never finishes (a corpus sweep reading pages on the pool
/// is that caller). Any other thread blocks.
fn hear(heard: &mpsc::Receiver<Event>) -> Result<Event, mpsc::RecvError> {
    if rayon::current_thread_index().is_none() {
        return heard.recv();
    }
    loop {
        match heard.try_recv() {
            Ok(event) => return Ok(event),
            Err(mpsc::TryRecvError::Disconnected) => return Err(mpsc::RecvError),
            Err(mpsc::TryRecvError::Empty) => {}
        }
        if !matches!(rayon::yield_now(), Some(rayon::Yield::Executed)) {
            match heard.recv_timeout(std::time::Duration::from_millis(1)) {
                Ok(event) => return Ok(event),
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err(mpsc::RecvError),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    }
}

/// What the calling thread learned, in order.
#[derive(Default)]
struct Joining {
    folds: Vec<Fold>,
    runs: BTreeMap<usize, Result<Box<PieceRun>, Declined>>,
    /// Whether the producer is done, and how many segments it made where it did not pass the
    /// stream's bound.
    ended: bool,
    count: Option<usize>,
    failed: Option<Refusal>,
}

impl Joining {
    fn take(&mut self, event: Event) {
        match event {
            Event::First { .. } => {}
            Event::Folded { index, folded } => match folded {
                Ok(fold) if fold.confirmed && index == self.folds.len() => self.folds.push(fold),
                Ok(_) => {
                    self.failed.get_or_insert(Refusal::Joining(Declined(
                        "a cut guessed from the bytes was not confirmed by the lex before it",
                    )));
                }
                Err(refusal) => {
                    self.failed.get_or_insert(refusal);
                }
            },
            Event::Ran { index, run } => {
                self.runs.insert(index, run);
            }
            Event::Produced(count) => {
                self.ended = true;
                self.count = count;
            }
        }
    }

    /// Gives the pipeline up: waits for the producer, since the fallback reads the whole stream,
    /// and answers why.
    fn give(
        &mut self,
        shared: &Shared<'_, '_>,
        heard: &mpsc::Receiver<Event>,
        reason: Declined,
    ) -> Settled {
        shared.abandoned.store(true, Ordering::Relaxed);
        while !self.ended {
            match hear(heard) {
                Ok(event) => self.take(event),
                Err(_) => break,
            }
        }
        if self.ended && self.count.is_none() {
            return Settled::PastBound;
        }
        Settled::Gave(self.failed.take().unwrap_or(Refusal::Joining(reason)))
    }

    /// The first segment holding a text object, among those composed.
    fn text_at(&self) -> Option<usize> {
        self.folds.iter().position(|fold| fold.text)
    }
}

/// Reads a page's stream with its inflate pipelined into the pieces.
pub(super) fn read<'r>(
    interpreter: &mut Interpreter<'_>,
    reader: &mut ContentReader<'r>,
    inputs: &Inputs<'r>,
    initial: &GraphicsState,
    asked: Option<usize>,
    most: usize,
) -> Outcome {
    let seam = interpreter.checkpoint();
    let (events, heard) = mpsc::channel();
    let shared = Shared {
        inputs,
        composer: Mutex::new(Composer::new()),
        events,
        abandoned: AtomicBool::new(false),
        planned: std::sync::OnceLock::new(),
    };
    let readings = interpreter.readings;
    let settled = rayon::in_place_scope(|scope| {
        let shared = &shared;
        let mut inflating = Producer {
            reader,
            most,
            asked,
        };
        scope.spawn(move |scope| {
            let produced = inflating.produce(shared, scope);
            shared.send(Event::Produced(produced));
        });
        let settled = interpreter.take_pieces(shared, &heard, initial, readings);
        shared.abandoned.store(true, Ordering::Relaxed);
        settled
    });
    let segments = std::mem::take(&mut shared.composer().segments);
    match settled {
        Settled::Joined => Outcome::Divided,
        Settled::PastBound => {
            interpreter.give_up(seam);
            Outcome::PastBound
        }
        Settled::Gave(refusal) => {
            interpreter.give_up(seam);
            match refusal {
                Refusal::Lexing(reason) => Outcome::Windowed(reason),
                Refusal::Joining(reason) => {
                    let length = segments.iter().map(|segment| segment.bytes.len()).sum();
                    let mut bytes = Vec::with_capacity(length);
                    for segment in &segments {
                        bytes.extend_from_slice(&segment.bytes);
                    }
                    Outcome::Inflated(bytes, reason)
                }
            }
        }
    }
}

/// How the calling thread's half of the pipeline ended.
enum Settled {
    Joined,
    PastBound,
    Gave(Refusal),
}

impl Interpreter<'_> {
    /// Puts this interpreter back where it stood before the first piece, and tells a drawer
    /// that took parts of it.
    fn give_up(&mut self, seam: crate::content::Checkpoint) {
        self.restore(seam);
        self.forget_memos();
        if let Some(parts) = &self.hand_over {
            parts.spoil_if_handed("the divided read was given up after a part was handed over");
        }
    }

    /// The calling thread's half: the first piece, then the later pieces joined in order.
    fn take_pieces(
        &mut self,
        shared: &Shared<'_, '_>,
        heard: &mpsc::Receiver<Event>,
        initial: &GraphicsState,
        readings: crate::content::Readings,
    ) -> Settled {
        let mut joining = Joining::default();
        // The first segment, and the producer's plan.
        let (first, planned, estimated, only) = loop {
            match hear(heard) {
                Ok(Event::First {
                    bytes,
                    planned,
                    estimated,
                    last,
                }) => break (bytes, planned, estimated, last),
                Ok(Event::Produced(None)) => return Settled::PastBound,
                Ok(event) => joining.take(event),
                Err(_) => return Settled::Gave(Refusal::Joining(Declined("a piece did not run"))),
            }
        };
        if only {
            return joining.give(
                shared,
                heard,
                Declined("the stream is one segment, which the plan after the inflate cuts finer"),
            );
        }
        if let Err(reason) = self.first_piece(shared, &first, planned, estimated, initial, readings)
        {
            return joining.give(shared, heard, reason);
        }

        let mut next = 1;
        loop {
            if joining.failed.is_some() {
                return joining.give(shared, heard, Declined("a segment was refused"));
            }
            // A text object ends the cuts at its segment, and in the first one there is no cut
            // before it: the stream is one piece, which the plan after the inflate may cut finer.
            if joining.text_at() == Some(0) {
                return joining.give(
                    shared,
                    heard,
                    Declined("the first segment holds a text object"),
                );
            }
            let count = joining.count;
            if count == Some(next) {
                break;
            }
            let text_here = joining.text_at() == Some(next);
            let last_here = count.map(|count| next.saturating_add(1) == count);
            // The tail is read once every segment is lexed, composed and confirmed, so nothing in
            // it is refused.
            if text_here
                && last_here == Some(false)
                && joining.folds.len() == count.unwrap_or(usize::MAX)
            {
                return match run_tail(shared, next, planned).and_then(|run| self.join(run)) {
                    Ok(()) => self.settle_joined(initial),
                    Err(reason) => joining.give(shared, heard, reason),
                };
            }
            let joinable = joining.folds.get(next).is_some()
                && (!text_here || last_here == Some(true))
                && joining.runs.contains_key(&next);
            if joinable && let Some(run) = joining.runs.remove(&next) {
                match run.and_then(|run| self.join(*run)) {
                    Ok(()) => next = next.saturating_add(1),
                    Err(reason) => return joining.give(shared, heard, reason),
                }
                continue;
            }
            match hear(heard) {
                Ok(Event::Produced(None)) => return Settled::PastBound,
                Ok(event) => joining.take(event),
                Err(_) => {
                    return joining.give(shared, heard, Declined("a piece did not run"));
                }
            }
        }
        // The last piece is joined only after its own segment's fold, and the folds come in order,
        // so every segment is composed and confirmed and nothing was refused in the stream.
        self.settle_joined(initial)
    }

    /// Reads the first piece into this interpreter, the page's own, on the calling thread.
    fn first_piece(
        &mut self,
        shared: &Shared<'_, '_>,
        first: &[u8],
        planned: usize,
        estimated: usize,
        initial: &GraphicsState,
        readings: crate::content::Readings,
    ) -> Result<(), Declined> {
        self.list_budget = crate::content::list_budget::ListBudget::for_piece(planned, 0);
        let mut reader = ContentReader::piece(first, 0, estimated.max(first.len()));
        let resources = &shared.inputs.page.resources;
        let left = self
            .run_from(&mut reader, resources, Composed::fresh(initial))
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
        Ok(())
    }

    /// What every joined reading ends with, as [`Interpreter::read_divided`]'s does.
    fn settle_joined(&mut self, initial: &GraphicsState) -> Settled {
        if self.reach_scanned >= Self::MAX_REACH_SCAN {
            return Settled::Gave(Refusal::Joining(Declined(
                "the pieces together scanned as far as the page's reach bound",
            )));
        }
        self.note_alpha_source(initial.alpha_is_shape);
        Settled::Joined
    }
}

/// Reads the last piece where a text object ended the cuts: from segment `text` to the end
/// of the stream, as one piece, on this thread.
fn run_tail(shared: &Shared<'_, '_>, text: usize, planned: usize) -> Result<PieceRun, Declined> {
    let (from, bytes, spine) = {
        let composer = shared.composer();
        let from = composer
            .segments
            .get(text)
            .map(|segment| segment.from)
            .ok_or(Declined("a text object in a segment that never arrived"))?;
        let tail = composer.segments.get(text..).unwrap_or_default();
        let mut bytes = Vec::with_capacity(tail.iter().map(|s| s.bytes.len()).sum());
        for segment in tail {
            bytes.extend_from_slice(&segment.bytes);
        }
        let spine = composer
            .spines
            .get(text)
            .cloned()
            .ok_or(Declined("no spine composed for the piece holding the text"))?;
        (from, bytes, spine)
    };
    let of = from.saturating_add(bytes.len());
    run_piece(shared.inputs, &bytes, from, of, &spine, planned, true)
}
