//! The threads that make a frame's exact meets while the walk goes on (ADR 1541).
//!
//! **Why beside the walk and not after it.** Made after the walk, divided across eight pinned
//! threads, `bug1721218_reduced.pdf`'s 11 988 exact pixels a render took 11 to 17 ms at the
//! frame's end, the walk's thread waiting for all of it: the eight fastest cores of this
//! machine are four cores and their siblings. Given to helpers as each meet is recorded, they
//! are made on the other cores while the walk's thread goes on with the marks after them, and
//! what is left at the settle is the last few.
//!
//! **Why threads of a frame's own and not a scope.** The fan-out's threads live inside one
//! drain of the queue (`parallel::rasterise_all`) because its jobs borrow the scene. A meet's
//! inputs are owned ([`ExactInputs`]), so a helper borrows nothing and can outlive any one
//! drain; it is started by the first frame that records enough exact pixels to pay for it, and
//! the frame lets it go before it returns ([`Helpers`]'s `Drop`), which is the shape
//! `parallel`'s module comment states the host asked for: a frame that enters threads inside
//! the render and has left them before the call returns.
//!
//! **Which thread made an area is no part of its bytes**: [`ExactInputs::areas`] is a pure
//! function of what the walk recorded, and the settle writes the areas in the order the walk
//! met the marks, whoever made them.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;

use super::deferred::ExactInputs;

/// A meet's areas as the helpers hand them back: its place in the frame's recorded meets, and
/// [`ExactInputs::areas`].
type Made = (usize, Option<Vec<u8>>);

/// The meets given and not yet claimed, the areas made and not yet collected, and how many
/// meets a thread has claimed and not yet made.
#[derive(Default)]
struct Work {
    given: VecDeque<(usize, Arc<ExactInputs>)>,
    made: Vec<Made>,
    claimed: usize,
    closed: bool,
}

/// The work and the signal that it changed, shared by the walk's thread and the helpers.
#[derive(Default)]
struct Shared {
    work: Mutex<Work>,
    changed: Condvar,
}

impl Shared {
    /// The work, whatever a thread that panicked while holding it left: every field is a
    /// count or a list that a panic cannot leave half-written.
    fn lock(&self) -> MutexGuard<'_, Work> {
        self.work.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Wait for the work to change.
    fn wait<'g>(&self, guard: MutexGuard<'g, Work>) -> MutexGuard<'g, Work> {
        self.changed
            .wait(guard)
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// A frame's helper threads and the work they share with the walk's thread.
pub(in crate::encode) struct Helpers {
    shared: Arc<Shared>,
    threads: Vec<JoinHandle<()>>,
}

impl std::fmt::Debug for Helpers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Helpers")
            .field("threads", &self.threads.len())
            .finish_non_exhaustive()
    }
}

impl Helpers {
    /// Start `count` helpers; a thread the process cannot start is one helper fewer, and with
    /// none the walk's thread makes every area at the settle.
    pub(super) fn start(count: usize) -> Self {
        let shared = Arc::new(Shared::default());
        let threads = (0..count)
            .filter_map(|_| {
                let shared = Arc::clone(&shared);
                std::thread::Builder::new()
                    .name("raster-exact-meet".into())
                    .spawn(move || help(&shared))
                    .ok()
            })
            .collect();
        Self { shared, threads }
    }

    /// Give the helpers the meet recorded at `index`.
    pub(super) fn give(&self, index: usize, inputs: Arc<ExactInputs>) {
        self.shared.lock().given.push_back((index, inputs));
        self.shared.changed.notify_one();
    }

    /// The areas of the `count` meets given since the last collection, in the order they were
    /// recorded: the walk's thread makes what no helper has claimed, then waits for the rest.
    pub(super) fn collect(&self, count: usize) -> Vec<Option<Vec<u8>>> {
        let mut work = self.shared.lock();
        loop {
            if let Some((index, inputs)) = work.given.pop_front() {
                drop(work);
                let areas = inputs.areas();
                work = self.shared.lock();
                work.made.push((index, areas));
            } else if work.claimed == 0 {
                break;
            } else {
                work = self.shared.wait(work);
            }
        }
        let made = std::mem::take(&mut work.made);
        drop(work);
        let mut areas: Vec<Option<Vec<u8>>> = (0..count).map(|_| None).collect();
        for (index, made) in made {
            if let Some(slot) = areas.get_mut(index) {
                *slot = made;
            }
        }
        areas
    }
}

impl Drop for Helpers {
    /// Close the work and wait for every helper to leave, so that no thread outlives the frame.
    fn drop(&mut self) {
        self.shared.lock().closed = true;
        self.shared.changed.notify_all();
        for thread in self.threads.drain(..) {
            // A helper that panicked has already put its meet back (`Claim`'s `Drop`), and the
            // walk's thread made it; there is nothing of its to report.
            let _ = thread.join();
        }
    }
}

/// A meet a helper has claimed: made, it is handed back; dropped unmade by a panic, it is put
/// back at the front of the work, so that the settle makes it on the walk's thread rather than
/// waiting for an area no thread will make.
struct Claim<'s> {
    shared: &'s Shared,
    meet: Option<(usize, Arc<ExactInputs>)>,
}

impl Drop for Claim<'_> {
    fn drop(&mut self) {
        let mut work = self.shared.lock();
        if let Some(meet) = self.meet.take() {
            work.given.push_front(meet);
        }
        work.claimed = work.claimed.saturating_sub(1);
        drop(work);
        self.shared.changed.notify_all();
    }
}

/// One helper: claim the next meet given, make its areas, hand them back, until the frame
/// closes the work.
fn help(shared: &Shared) {
    let mut work = shared.lock();
    loop {
        if let Some(meet) = work.given.pop_front() {
            work.claimed = work.claimed.saturating_add(1);
            drop(work);
            let mut claim = Claim {
                shared,
                meet: Some(meet),
            };
            let made = claim
                .meet
                .as_ref()
                .map(|(index, inputs)| (*index, inputs.areas()));
            if let Some(made) = made {
                claim.meet = None;
                shared.lock().made.push(made);
            }
            drop(claim);
            work = shared.lock();
        } else if work.closed {
            return;
        } else {
            work = shared.wait(work);
        }
    }
}
