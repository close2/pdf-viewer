//! The threads that make a frame's exact meets while the walk goes on (ADR 1541), and the
//! drain whose fan-out lends them (ADR 1692).
//!
//! **Why beside the walk and not after it.** Made after the walk, divided across eight pinned
//! threads, `bug1721218_reduced.pdf`'s 11 988 exact pixels a render took 11 to 17 ms at the
//! frame's end, the walk's thread waiting for all of it: the eight fastest cores of this
//! machine are four cores and their siblings. Given to helpers as each meet is recorded, they
//! are made on the other cores while the walk's thread goes on with the marks after them, and
//! what is left at the settle is the last few.
//!
//! **Whose threads make them.** A meet is recorded where its tile is committed, and with
//! threads allowed most tiles are committed by a drain of the queue, straight after the
//! drain's fan-out made their coverage (`parallel::rasterise_all`). So a drain whose jobs can
//! meet a residue keeps its fan-out's threads for its commit: each thread, once no job is left
//! to claim, claims the meets the commit gives until the drain lets it go ([`Lent`]). Those
//! threads are a scope's, borrowed from the drain; a stroked Type 3 page whose every meet is
//! recorded in one drain's commit started a second set of threads for them, 46 a frame where
//! 23 do the same work (ADR 1686 section 5, ADR 1692).
//!
//! **And why the frame still has threads of its own.** A meet recorded where no drain is
//! lending — a walk below the fan-out's floor, or the marks after a drain — has nobody to
//! claim it, and `bug1721218_reduced.pdf` records hundreds of them in the 11 ms of walk that
//! follow its one drain. A meet's inputs are owned ([`ExactInputs`]), so a helper borrows
//! nothing and can outlive any one drain; the frame starts them where a meet is given with no
//! thread claiming and the frame's meets have reached the floor, and lets them go before it
//! returns ([`Helpers`]'s `Drop`), which is the shape `parallel`'s module comment states the
//! host asked for: a frame that enters threads inside the render and has left them before the
//! call returns.
//!
//! **Which thread made an area is no part of its bytes**: [`ExactInputs::areas`] is a pure
//! function of what the walk recorded, and the settle writes the areas in the order the walk
//! met the marks, whoever made them.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
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
    /// Set when the frame lets its own helpers go.
    closed: bool,
}

/// The work and the signal that it changed, shared by the walk's thread and the helpers.
#[derive(Default)]
struct Shared {
    work: Mutex<Work>,
    changed: Condvar,
    /// Threads that will claim a meet given now: the frame's own and those a drain has lent.
    /// Counted by whoever starts or lends them, before they run, so that a meet given before a
    /// thread has reached its first claim still finds it counted; outside the lock, because
    /// the walk's thread asks it at every meet it records and the helpers hold the lock to
    /// claim (`Helpers::claimed_by_someone`).
    claiming: AtomicUsize,
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

/// A frame's meets to be made beside the walk, and the helper threads of its own, if it has
/// started any.
pub(in crate::encode) struct Helpers {
    shared: Arc<Shared>,
    /// The helper threads the frame started itself, which it joins as it lets them go.
    own: Vec<JoinHandle<()>>,
}

impl std::fmt::Debug for Helpers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Helpers")
            .field("own", &self.own.len())
            .finish_non_exhaustive()
    }
}

impl Helpers {
    /// The frame's meets with no thread yet to claim them: a drain lends its fan-out's
    /// ([`Helpers::lend`]), and [`Helpers::start`] starts the frame's own.
    pub(in crate::encode) fn idle() -> Self {
        Self {
            shared: Arc::new(Shared::default()),
            own: Vec::new(),
        }
    }

    /// Start `count` helpers of the frame's own; a thread the process cannot start is one
    /// helper fewer, and with none the walk's thread makes every area at the settle.
    pub(super) fn start(&mut self, count: usize) {
        self.shared.claiming.fetch_add(count, Ordering::Relaxed);
        let threads = (0..count)
            .filter_map(|_| {
                let shared = Arc::clone(&self.shared);
                std::thread::Builder::new()
                    .name("raster-exact-meet".into())
                    .spawn(move || {
                        let _leaving = Leaving(&shared);
                        help(&shared, || false, |work| work.closed);
                    })
                    .ok()
            })
            .collect::<Vec<_>>();
        self.shared
            .claiming
            .fetch_sub(count.saturating_sub(threads.len()), Ordering::Relaxed);
        crate::threads::count(threads.len());
        self.own = threads;
    }

    /// Whether meets are given that no thread has claimed.
    pub(super) fn waiting(&self) -> bool {
        !self.shared.lock().given.is_empty()
    }

    /// Whether the frame has started helpers of its own.
    pub(super) fn started(&self) -> bool {
        !self.own.is_empty()
    }

    /// Whether a meet given now has a thread to claim it.
    ///
    /// Read without the lock: every count this reads was added by the walk's own thread
    /// before the threads it counts started, and a lent thread is counted off only after the
    /// drain has let it go, which the drain's scope joins before the walk records another meet.
    pub(super) fn claimed_by_someone(&self) -> bool {
        self.shared.claiming.load(Ordering::Relaxed) > 0
    }

    /// A drain's fan-out, to be lent to the frame's meets: the fan-out counts its threads in
    /// ([`Lent::count_in`]) before it starts them, and each runs [`Lent::help`] once the
    /// drain's jobs are all claimed.
    pub(in crate::encode) fn lend(&self) -> Lent {
        Lent {
            shared: Arc::clone(&self.shared),
            state: AtomicU8::new(LENDING),
        }
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
        for thread in self.own.drain(..) {
            // A helper that panicked has already put its meet back (`Claim`'s `Drop`), and the
            // walk's thread made it; there is nothing of its to report.
            let _ = thread.join();
        }
    }
}

/// A drain's fan-out threads, lent to the frame's meets until the drain lets them go
/// (ADR 1692).
///
/// The threads are the drain's scope's, so the scope cannot end while one of them waits for
/// a meet: the drain lets them go before its scope joins them, and a guard of the drain's lets
/// them go on every path out, an error and a panic among them.
///
/// **How they go depends on what the walk does next.** After the frame's last drain the walk's
/// thread settles the meets and has nothing else to do, so the lent threads make what is
/// still given and the walk's thread makes it beside them ([`Lent::finish`]). After any other
/// drain the walk goes on, and a backlog the lent threads stayed to make would hold it at the
/// scope's join for work the frame's own helpers can make beside it: there the lent threads
/// leave with the meet each has claimed ([`Lent::release`]), and the drain hands what is left
/// to the frame's own (`Encoder::drain`).
pub(in crate::encode) struct Lent {
    shared: Arc<Shared>,
    state: AtomicU8,
}

/// A lent thread claims meets as they are given.
const LENDING: u8 = 0;
/// A lent thread makes what is given and leaves when nothing is.
const FINISHING: u8 = 1;
/// A lent thread leaves once the meet it has claimed is made.
const RELEASED: u8 = 2;

impl Lent {
    /// Count `threads` more as claiming the frame's meets from now until the drain lets them
    /// go, before they are started.
    pub(in crate::encode) fn count_in(&self, threads: usize) {
        self.shared.claiming.fetch_add(threads, Ordering::Relaxed);
    }

    /// Claim the frame's meets until the drain lets this thread go: run by each lent thread
    /// once the drain has no job left for it.
    pub(in crate::encode) fn help(&self) {
        let _leaving = Leaving(&self.shared);
        // Each read is under the lock its state is set under, so that a thread about to wait
        // cannot miss the change that should have woken it.
        help(
            &self.shared,
            || self.state.load(Ordering::Relaxed) == RELEASED,
            |_| self.state.load(Ordering::Relaxed) == FINISHING,
        );
    }

    /// Let every lent thread go once the meet it has claimed is made, leaving what is still
    /// given where it is.
    pub(in crate::encode) fn release(&self) {
        self.set(RELEASED);
    }

    /// Let every lent thread go once nothing is given, and make what is given beside them on
    /// the drain's own thread, which would otherwise wait for them at the scope's join with
    /// nothing to do.
    pub(in crate::encode) fn finish(&self) {
        self.set(FINISHING);
        help(&self.shared, || false, |_| true);
    }

    fn set(&self, state: u8) {
        let work = self.shared.lock();
        // A thread released already stays released.
        self.state.fetch_max(state, Ordering::Relaxed);
        drop(work);
        self.shared.changed.notify_all();
    }
}

/// A thread leaving the meets, counted off the claiming threads however it leaves.
struct Leaving<'s>(&'s Shared);

impl Drop for Leaving<'_> {
    fn drop(&mut self) {
        self.0.claiming.fetch_sub(1, Ordering::Relaxed);
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

/// One helper: claim the next meet given, make its areas, hand them back, until `released`
/// says this thread is let go whatever is given, or `idle_gone` says it is let go once nothing
/// is — each asked under the work's lock.
fn help(shared: &Shared, released: impl Fn() -> bool, idle_gone: impl Fn(&Work) -> bool) {
    let mut work = shared.lock();
    loop {
        if released() {
            return;
        }
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
        } else if idle_gone(&work) {
            return;
        } else {
            work = shared.wait(work);
        }
    }
}
