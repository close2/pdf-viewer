//! The line lane's arithmetic, made on the encode's threads ahead of the walk (ADR 1884).
//!
//! What the closed form makes of a line before its clip is known — the instance's head and the
//! box its ink lies in, or the stroke road ([`Unclipped`]) — is a pure function of the command and
//! the view (ADR 1878 §3). Nothing in it is decided by the frame's order: not the clip chain's
//! resolution, the budget, the queue or the instance stream. So it is the same answer whichever
//! thread asks and whenever, and on a page of millions of lines it is asked by other threads a
//! block of the walk ahead of the walk's own thread, which takes what is ordered — the replay
//! record, the soft mask, the chain, the cull, the queue and the instance — line by line in
//! encounter order, exactly as [`Encoder::encode_line`] takes it.
//!
//! # The pipeline
//!
//! The walk's positions — a scene's commands, or a replay's records — are cut into blocks of
//! [`BLOCK`]. A few threads claim blocks in order and make the forms of the lines in each; the
//! walk takes the blocks in order, and within a block each line command that was made takes its
//! form and every other position is walked as it always was. The threads run at most a ring of
//! blocks ahead of the walk, so what is made and not yet taken is bounded by the encoder's
//! `in_flight_limit`, the share of the host's own frame budget the fan-out's queue is already
//! bounded by (`parallel.rs`). Where the walk reaches a block no thread has claimed, it makes the
//! block itself; where a thread stopped inside one, the walk walks that block as before.
//!
//! So every byte, count and refusal is the one-threaded walk's, and `tests/encode_threads.rs`
//! holds that to byte equality at every thread count rather than arguing it.
//!
//! **The threads live for one walk**, inside a scope that has joined them before the encode
//! returns, for the reasons `parallel.rs` gives for its own: the host names how many threads a
//! frame may use, and nothing of this library is alive between frames.

use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;

use raster_scene::{ClipId, Command, MaskId, Rect};

use super::Encoder;
use super::device_space::compose;
use super::line::{LineMark, Unclipped};
use crate::error::RenderError;

/// How many positions of the walk one block spans (ADR 1884 §2).
///
/// A line's form is about 210 instructions on the witness's page, and its place in the walk
/// about 280 (callgrind, ADR 1884 §1), so a block of this many is a few hundred microseconds on
/// either side — long beside the few microseconds a hand-over between threads costs, and short
/// enough that the walk's first block is not a long wait.
const BLOCK: usize = 4_096;

/// The fewest line commands a walk needs before threads are started for it: below this the
/// threads would cost more to start than the forms they make (ADR 1884 §2).
const FLOOR: usize = 4 * BLOCK;

/// The most threads that make forms beside the walk (ADR 1884 §2).
///
/// A line's form costs less than its place in the walk, so two threads keep ahead of the walk
/// on an idle machine and the third is the margin for a core a neighbour shares; beyond that the
/// walk is what the frame waits for, and another thread would wait with it (ADR 1884 §2).
const MAKERS: usize = 3;

/// One line made ahead: where it is in the walk, what its draw asks of the walk, and its closed
/// form.
#[derive(Debug, Clone, Copy)]
struct Made {
    /// Its position in the walk.
    at: usize,
    clip: Option<ClipId>,
    mask: Option<MaskId>,
    form: Unclipped,
}

/// One block's place in the ring.
enum Slot {
    /// Nobody holds it; its buffer waits for the next block.
    Free(Vec<Made>),
    /// A thread is making its forms, or the walk is taking them.
    Held,
    /// Its forms are made and the walk has not taken them.
    Made(Vec<Made>),
    /// The thread making it stopped, so the walk walks it as before.
    Lost,
}

/// What the threads and the walk agree about, under one lock.
struct State {
    /// The next block no thread has claimed.
    claimed: usize,
    /// The blocks the walk has finished with.
    taken: usize,
    slots: Vec<Slot>,
    /// Set when the walk leaves, however it leaves.
    stop: bool,
}

/// The ring the threads fill and the walk empties.
struct Pipe {
    state: Mutex<State>,
    changed: Condvar,
    blocks: usize,
}

impl Pipe {
    fn new(blocks: usize, ring: usize) -> Self {
        Self {
            state: Mutex::new(State {
                claimed: 0,
                taken: 0,
                slots: (0..ring).map(|_| Slot::Free(Vec::new())).collect(),
                stop: false,
            }),
            changed: Condvar::new(),
            blocks,
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn wait<'g>(&self, state: MutexGuard<'g, State>) -> MutexGuard<'g, State> {
        self.changed
            .wait(state)
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// The slot `block` is kept in.
    fn slot(state: &mut State, block: usize) -> Option<&mut Slot> {
        let ring = state.slots.len();
        state.slots.get_mut(block.checked_rem(ring)?)
    }

    /// Claim the next block whose slot is free and take its buffer, or `None` once there is
    /// nothing left to claim or the walk has left.
    fn claim(&self) -> Option<(usize, Vec<Made>)> {
        let mut state = self.lock();
        loop {
            if state.stop || state.claimed >= self.blocks {
                return None;
            }
            if state.claimed < state.taken.saturating_add(state.slots.len()) {
                break;
            }
            state = self.wait(state);
        }
        let block = state.claimed;
        state.claimed = block.saturating_add(1);
        let held = Self::slot(&mut state, block).map(|slot| std::mem::replace(slot, Slot::Held));
        let buffer = match held {
            Some(Slot::Free(buffer)) => buffer,
            // A block is claimed only once the walk has given its slot back, so the slot is
            // free; a fresh buffer is the answer that stays right if it were not.
            _ => Vec::new(),
        };
        Some((block, buffer))
    }

    /// Put `block`'s made forms, or the news that they were lost, where the walk takes them.
    fn hand_over(&self, block: usize, made: Option<Vec<Made>>) {
        let mut state = self.lock();
        if let Some(slot) = Self::slot(&mut state, block) {
            *slot = made.map_or(Slot::Lost, Slot::Made);
        }
        drop(state);
        self.changed.notify_all();
    }

    /// One thread's share: claim blocks and make them until there is none left.
    fn make_blocks(&self, form_block: &(impl Fn(usize, &mut Vec<Made>) + Sync)) {
        while let Some((block, buffer)) = self.claim() {
            let mut making = Making {
                pipe: self,
                block,
                buffer: Some(buffer),
            };
            if let Some(buffer) = making.buffer.as_mut() {
                form_block(block, buffer);
            }
            let made = making.buffer.take();
            self.hand_over(block, made);
        }
    }

    /// The walk's turn at `block`: its made forms, made here where no thread has claimed it,
    /// or `None` where the thread making it stopped.
    fn take(&self, block: usize, form_block: &impl Fn(usize, &mut Vec<Made>)) -> Option<Vec<Made>> {
        let mut state = self.lock();
        loop {
            let unclaimed = state.claimed == block;
            let slot = Self::slot(&mut state, block)?;
            match std::mem::replace(slot, Slot::Held) {
                Slot::Made(made) => return Some(made),
                Slot::Lost => return None,
                Slot::Free(mut buffer) if unclaimed => {
                    state.claimed = block.saturating_add(1);
                    drop(state);
                    form_block(block, &mut buffer);
                    return Some(buffer);
                }
                other => *slot = other,
            }
            state = self.wait(state);
        }
    }

    /// The walk is done with `block`: its buffer serves a later block.
    fn give_back(&self, block: usize, mut buffer: Vec<Made>) {
        buffer.clear();
        let mut state = self.lock();
        if let Some(slot) = Self::slot(&mut state, block) {
            *slot = Slot::Free(buffer);
        }
        state.taken = block.saturating_add(1);
        drop(state);
        self.changed.notify_all();
    }
}

/// A block a thread is making: handed over as lost if the thread stops inside it, so that the
/// walk never waits for a block nobody will finish.
struct Making<'p> {
    pipe: &'p Pipe,
    block: usize,
    buffer: Option<Vec<Made>>,
}

impl Drop for Making<'_> {
    fn drop(&mut self) {
        if self.buffer.is_some() {
            self.pipe.hand_over(self.block, None);
        }
    }
}

/// Stops the threads however the walk leaves the scope — finished, refused or panicking — so
/// that the scope's join never waits on a thread waiting for room in the ring.
struct Stop<'p>(&'p Pipe);

impl Drop for Stop<'_> {
    fn drop(&mut self) {
        self.0.lock().stop = true;
        self.0.changed.notify_all();
    }
}

impl<'s> Encoder<'s> {
    /// Walk `positions` positions in order, `walk` taking each one as the walk always has —
    /// except the line commands among them, whose closed forms are made by other threads ahead
    /// of the walk where the host allowed threads and the walk holds at least [`FLOOR`] of its
    /// `lines` line commands.
    ///
    /// `line_at` names the command at a position and its index in the scene, or `None` for a
    /// position that is no command of the scene's (a replay's record of its own).
    pub(super) fn walk_lines_ahead(
        &mut self,
        positions: usize,
        lines: usize,
        line_at: impl Fn(usize) -> Option<(usize, &'s Command)> + Sync,
        mut walk: impl FnMut(&mut Self, usize) -> Result<(), RenderError>,
    ) -> Result<(), RenderError> {
        let makers = self.threads.saturating_sub(1).min(MAKERS);
        if makers == 0 || lines < FLOOR {
            for at in 0..positions {
                walk(self, at)?;
            }
            return Ok(());
        }
        let viewport = self.viewport;
        let form_block = |block: usize, made: &mut Vec<Made>| {
            made.clear();
            let start = block.saturating_mul(BLOCK);
            for at in start..start.saturating_add(BLOCK).min(positions) {
                let Some(line) = line_at(at).and_then(|(_, command)| LineMark::of(command)) else {
                    continue;
                };
                if let Some(form) = Unclipped::of(&line, &compose(line.transform, viewport)) {
                    made.push(Made {
                        at,
                        clip: line.clip,
                        mask: line.mask,
                        form,
                    });
                }
            }
        };
        let blocks = positions.div_ceil(BLOCK);
        let per_block = (size_of::<Made>() as u64).saturating_mul(BLOCK as u64);
        let ring = usize::try_from(self.in_flight_limit.checked_div(per_block).unwrap_or(0))
            .unwrap_or(usize::MAX)
            .clamp(2, blocks.max(2));
        let pipe = Pipe::new(blocks, ring);
        crate::threads::count(makers);
        thread::scope(|scope| {
            for _ in 0..makers {
                scope.spawn(|| pipe.make_blocks(&form_block));
            }
            let _stop = Stop(&pipe);
            let mut admitted = None;
            for block in 0..blocks {
                let start = block.saturating_mul(BLOCK);
                let end = start.saturating_add(BLOCK).min(positions);
                let Some(made) = pipe.take(block, &form_block) else {
                    for at in start..end {
                        walk(self, at)?;
                    }
                    pipe.give_back(block, Vec::new());
                    continue;
                };
                let mut next = made.iter().peekable();
                for at in start..end {
                    match next.next_if(|line| line.at == at) {
                        Some(line) => {
                            let index = line_at(at).map_or(at, |(index, _)| index);
                            self.place_made(index, line, &mut admitted, &line_at)?;
                        }
                        None => walk(self, at)?,
                    }
                }
                pipe.give_back(block, made);
            }
            Ok(())
        })
    }

    /// One line made ahead, taken in its place in the walk: every step
    /// [`Encoder::encode_line`] takes in the frame's order, in that order, and none of the
    /// arithmetic it would repeat.
    fn place_made(
        &mut self,
        index: usize,
        line: &Made,
        admitted: &mut Option<(Option<ClipId>, Option<Rect>)>,
        line_at: &impl Fn(usize) -> Option<(usize, &'s Command)>,
    ) -> Result<(), RenderError> {
        self.begin_command_records(index);
        self.record_slow();
        // Most lines have no soft mask, and asking `use_mask` for none is a call of about twenty
        // instructions a line on the witness's page, a fourteenth of the walk's share
        // (callgrind, ADR 1884 §1).
        let mask_index = match line.mask {
            None => None,
            mask => self.use_mask(mask)?,
        };
        if let Some(admit) = self.admitted(line.clip, admitted)? {
            return self.place_line(&line.form, admit, mask_index);
        }
        // A chain with a residue link, which the closed form does not meet: the stroke road, as
        // the walk takes it.
        let Some(mark) = line_at(line.at).and_then(|(_, command)| LineMark::of(command)) else {
            // Cannot be reached: a line was made only where its position names a line.
            return Ok(());
        };
        let resolved = self.resolve_clip(mark.clip)?;
        let to_device = compose(mark.transform, self.viewport);
        self.encode_line_as_stroke(&mark, &to_device, &resolved, mask_index)
    }

    /// The rectangle a line under `clip` is admitted to — the chain's met with the target's —
    /// or `None` where the chain has a residue link; `last` keeps the answer for the chain
    /// asked last, since the lines of a page come in long runs under one chain.
    ///
    /// The chain is resolved the first time it is asked in encounter order, as the walk
    /// resolves it; asked again it is the resolver's memo of the same function (`clips.rs`),
    /// which `last` stands in for.
    fn admitted(
        &mut self,
        clip: Option<ClipId>,
        last: &mut Option<(Option<ClipId>, Option<Rect>)>,
    ) -> Result<Option<Rect>, RenderError> {
        if let Some((asked, admit)) = *last
            && asked == clip
        {
            return Ok(admit);
        }
        let resolved = self.resolve_clip(clip)?;
        let admit = resolved
            .residues
            .is_none()
            .then(|| resolved.rect.intersection(self.visible));
        *last = Some((clip, admit));
        Ok(admit)
    }
}
