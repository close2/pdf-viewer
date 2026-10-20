//! How far an interpretation has got, for a window that is waiting for it.
//!
//! # What this is for
//!
//! The project owner, on a valid page whose one content stream inflates to 278 MB of
//! twenty-nine million operators: allow it, "and possible display a loading bar". A page like
//! that is interpreted for seconds, and for all of them a window had nothing to say but that it
//! was busy. [`Progress`] is the figure a window shows instead — how many operators the page has
//! run, and how many of its `/Contents`' input bytes have been read out of how many there are.
//!
//! # Why a thread's slot and not a parameter
//!
//! The figure is written from one line of the operator loop (`content::run`) and read from
//! another thread, so what has to reach the loop is a shared handle. Threading it through
//! [`crate::content::interpret`] the way the ledger is threaded would add a parameter to every
//! function between a page and its forms, patterns and glyphs — a few dozen signatures, for a
//! value that changes nothing any of them computes. So a caller *installs* a handle on its own
//! thread for the length of one call ([`Progress::watch`]), and the loop publishes to whatever is
//! installed. The interpretation stays a pure function of the bytes: nothing is read back from the
//! handle, and a thread with nothing installed pays one thread-local look per check and no more.
//! ADR 1811 prices both shapes.
//!
//! # What the figure is and is not
//!
//! **Bytes over bytes is a fraction with a known end; operators are not.** No stream states how
//! many operators it holds, and no filter states its output length (`pdf_syntax::Pump::taken`
//! says why), so the bar is the page's input bytes — which reaches exactly its end when the last
//! operator has been read — and the operator count beside it is a count, never a fraction. A form
//! the page invokes does not move the bar while it runs (`ContentReader::progress` says why), so a
//! page whose cost is one large form shows a bar that pauses, which is the truth about the page's
//! own stream.
//!
//! # Across a process boundary
//!
//! A confined worker interprets on the far side of a pipe, where a window's thread cannot read an
//! atomic. So a figure can also be *relayed* ([`Progress::relaying`]): every [`RELAY_EVERY`]
//! operators the interpreting thread hands the reading to a closure, which the worker makes a
//! frame, and the host stores what arrives with [`Progress::store`] into the figure its window
//! reads. Counted in operators rather than timed, because a confined worker reads no clock it was
//! not handed (`viewer-core`'s rule 3), and a count is enough to pace a bar.
//!
//! # The list itself, in parts
//!
//! A figure says how far the read has got; [`Parts`] hands over *what* it has got — the page's
//! display list in parts, as the interpreter builds it — so that a drawer can paint the page
//! while it is still being read, the shape `doc/pdf.js`'s worker gives its operator list when it
//! flushes it in chunks, taken as evidence about an approach (ADR 1855). It is installed the
//! way the figure is, for the figure's reason — nothing is threaded through the interpreter's
//! signatures and nothing is read back — but for one interpretation rather than for a thread:
//! [`Parts::during`] installs it, and the first page interpretation inside *claims* it, so a
//! page that interprets another page inside itself (§8.10.4's reference) hands over its own list
//! and never the other's.
//!
//! **A part is handed over only where the finished list will begin with it**, and where that
//! cannot be known in advance it is checked after the fact rather than assumed. The parts come
//! from the page's own content stream, between two operators, on a page drawn on the device's
//! own components, outside a text object — the places where the interpreter has nothing open
//! that could gather what it already pushed into something else. And a command handed over that
//! the interpreter later takes back ([`pdf_render::DisplayList::taken_back_since_asked`]) spoils
//! the hand-over: the drawer is told, and draws the finished list whole, as it would have
//! without any of this.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError, Weak};
use std::time::Duration;

use pdf_render::{DisplayList, ListPart, Stated};

/// A shared figure of how far one interpretation has got — written on the interpreting thread,
/// read on any.
///
/// Cloning shares the figure; a window keeps one clone and hands the other to the thread it
/// interprets on.
#[derive(Debug, Clone, Default)]
pub struct Progress {
    cells: Arc<Cells>,
}

/// How many operators pass between two relayed readings.
///
/// **A thirty-second of a million**, bounded from both sides on the page that asked for this. From
/// above: the owner's 29 million operators are read in about 2.5 s, so a reading every 32 768 is
/// about 360 a second — several per refresh of a bar that a window repaints thirty times a second.
/// From below: a relayed reading is one write of a 33-byte frame, a few microseconds, so 885 of them
/// on that page cost a few milliseconds of 2.5 s.
pub const RELAY_EVERY: u64 = 1 << 15;

/// Where a relayed figure goes: a closure the interpreting thread calls with each reading.
pub type Relay = Box<dyn Fn(Reading) + Send + Sync>;

/// The three figures, each written whole.
///
/// **Relaxed, and three cells rather than one lock**: a reader may see the operator count of one
/// check beside the byte count of the next, which moves a bar by a few dozen operators' worth and
/// costs the interpreting thread nothing it would notice. [`Reading::fraction`] clamps, so a torn
/// read never shows more than the whole.
#[derive(Default)]
struct Cells {
    operators: AtomicU64,
    taken: AtomicU64,
    held: AtomicU64,
    /// Where each [`RELAY_EVERY`]th operator's reading goes, for a figure that crosses a process.
    relay: Option<Relay>,
    /// The operator count of the last relayed reading.
    relayed: AtomicU64,
}

/// Written by hand because a [`Relay`] is a closure and has nothing to print.
impl std::fmt::Debug for Cells {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cells")
            .field("operators", &self.operators)
            .field("taken", &self.taken)
            .field("held", &self.held)
            .field("relaying", &self.relay.is_some())
            .finish_non_exhaustive()
    }
}

/// One look at a [`Progress`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Reading {
    /// Operators the page has run, counted where the interpreter counts them.
    pub operators: u64,
    /// Input bytes of the page's `/Contents` read so far.
    pub taken: u64,
    /// Input bytes the page's `/Contents` holds; zero until the page's own stream has begun.
    pub held: u64,
}

impl Reading {
    /// How much of the page's `/Contents` has been read, from 0 to 1, or `None` before any of it
    /// has been — which a window shows as a wait with no bar rather than as an empty one.
    #[must_use]
    pub fn fraction(self) -> Option<f64> {
        (self.held > 0).then(|| {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a share for a bar: a stream past 2^53 bytes loses digits nobody can see"
            )]
            let share = self.taken as f64 / self.held as f64;
            share.clamp(0.0, 1.0)
        })
    }
}

thread_local! {
    /// The figure this thread's interpretations publish to, while a caller has one installed.
    static WATCHING: RefCell<Option<Progress>> = const { RefCell::new(None) };
    /// The hand-over the next page interpretation on this thread claims, while a caller has one
    /// installed and no interpretation has claimed it yet.
    static HANDING: RefCell<Option<Parts>> = const { RefCell::new(None) };
}

/// Takes the hand-over installed on this thread, so that the interpretation claiming it is the
/// only one that hands parts to it.
pub(crate) fn claim() -> Option<Parts> {
    HANDING
        .try_with(|slot| slot.borrow_mut().take())
        .ok()
        .flatten()
}

impl Progress {
    /// A figure nothing has written to yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A figure that also hands a reading to `relay` every [`RELAY_EVERY`] operators — for a
    /// worker whose window is in another process.
    #[must_use]
    pub fn relaying(relay: Relay) -> Self {
        Self {
            cells: Arc::new(Cells {
                relay: Some(relay),
                ..Cells::default()
            }),
        }
    }

    /// Writes a reading that arrived from elsewhere — a relayed one, which the host side of a
    /// process boundary stores for its window to read.
    pub fn store(&self, reading: Reading) {
        self.cells
            .operators
            .store(reading.operators, Ordering::Relaxed);
        self.cells.taken.store(reading.taken, Ordering::Relaxed);
        self.cells.held.store(reading.held, Ordering::Relaxed);
    }

    /// What the figure says now.
    #[must_use]
    pub fn read(&self) -> Reading {
        Reading {
            operators: self.cells.operators.load(Ordering::Relaxed),
            taken: self.cells.taken.load(Ordering::Relaxed),
            held: self.cells.held.load(Ordering::Relaxed),
        }
    }

    /// Runs `run` with this figure installed on the current thread, so that every page
    /// interpreted inside it publishes here.
    ///
    /// What was installed before is put back afterwards, a panic included, so a nested watch is
    /// the inner one's for exactly its own length.
    pub fn watch<T>(&self, run: impl FnOnce() -> T) -> T {
        /// Puts back what was installed, on the way out however the way out is taken.
        struct Restore(Option<Progress>);
        impl Drop for Restore {
            fn drop(&mut self) {
                let previous = self.0.take();
                // A thread being torn down has no slot to restore, and nothing left to publish.
                let _ = WATCHING.try_with(|slot| *slot.borrow_mut() = previous);
            }
        }
        let previous = WATCHING.with(|slot| slot.borrow_mut().replace(self.clone()));
        let _restore = Restore(previous);
        run()
    }

    /// Writes the running page's figure, and relays it where this figure relays.
    fn publish(&self, operators: usize, (taken, held): (usize, usize)) {
        let wide = |count: usize| u64::try_from(count).unwrap_or(u64::MAX);
        let reading = Reading {
            operators: wide(operators),
            taken: wide(taken),
            held: wide(held),
        };
        self.store(reading);
        if let Some(relay) = &self.cells.relay {
            let last = self.cells.relayed.load(Ordering::Relaxed);
            // A count below the last relayed one is the next page's interpretation beginning,
            // which starts its own series rather than waiting for the old one's figure.
            if reading.operators < last || reading.operators.saturating_sub(last) >= RELAY_EVERY {
                self.cells
                    .relayed
                    .store(reading.operators, Ordering::Relaxed);
                relay(reading);
            }
        }
    }
}

/// Fewest commands a part holds.
///
/// **A few milliseconds of the read**, bounded from both sides. From above: the first part is
/// the first picture, and the witness reads about 1.7 million commands a second, so this many is
/// 2.4 ms of its read. From below: a part costs a copy of its commands' headers, a lock and a
/// wake, and a page this small is drawn whole in the time a part would take to cross — no
/// page under it is handed over at all.
pub const FIRST_PART: usize = 4_096;

/// A part grows with the list: after the first, one holds at least this fraction of what was
/// already handed over.
///
/// So a page is handed over in a number of parts that grows with the logarithm of its length —
/// about sixty on the witness's 4.4 million commands — and the part still to draw when the read
/// ends is at most a sixteenth of the list.
pub const PART_GROWTH: usize = 16;

/// A page's display list handed from the thread that reads it to a thread that draws it.
///
/// Cloning shares it: the reading thread installs one for the interpretation it runs
/// ([`Parts::during`]), and the drawer holds another.
#[derive(Debug, Clone)]
pub struct Parts {
    shared: Arc<Shared>,
}

/// The two threads' common state, and the wake a drawer waits on.
#[derive(Debug)]
struct Shared {
    state: Mutex<HandOver>,
    arrived: Condvar,
    /// Fewest commands a part holds: [`FIRST_PART`], or what [`Parts::in_parts_of`] asked.
    first: usize,
}

impl Default for Parts {
    fn default() -> Self {
        Self::new()
    }
}

/// What has been handed over and not yet taken, and how the hand-over stands.
#[derive(Debug, Default)]
struct HandOver {
    waiting: VecDeque<ListPart>,
    /// Where the next part begins.
    handed: Stated,
    /// Why the parts handed over are no longer known to begin the finished list, once they are
    /// not.
    spoiled: Option<&'static str>,
    /// How much the finished list states, once its run has concluded.
    ended: Option<Stated>,
    /// The finished list, once a caller shared it ([`Parts::keep`]).
    list: Option<Weak<DisplayList>>,
}

/// What a drawer is handed next.
#[derive(Debug)]
pub enum Handed {
    /// The next part of the list, in order.
    Part(ListPart),
    /// The read has ended: `Ok` with how much the finished list states, of which every part
    /// handed over is a prefix — or `Err` with why the parts handed over are not known to be.
    Ended(Result<Stated, &'static str>),
}

impl Parts {
    /// A hand-over nothing has been handed to yet.
    #[must_use]
    pub fn new() -> Self {
        Self::in_parts_of(FIRST_PART)
    }

    /// A hand-over whose parts hold at least `first` commands rather than [`FIRST_PART`] — for a
    /// test that wants every page of a population handed over in many parts, which at the
    /// shipped size only a page of thousands of marks is.
    #[must_use]
    pub fn in_parts_of(first: usize) -> Self {
        Self {
            shared: Arc::new(Shared {
                state: Mutex::default(),
                arrived: Condvar::new(),
                first: first.max(1),
            }),
        }
    }

    /// Runs `run` with this hand-over installed on the current thread, for the first page
    /// interpretation inside it to claim; what was installed before is put back afterwards.
    ///
    /// **The hand-over ends with the call, however it ends**: a call in which no interpretation
    /// claimed it, or one whose interpretation never finished its list, leaves the drawer told
    /// so rather than waiting for a part that will not come.
    pub fn during<T>(&self, run: impl FnOnce() -> T) -> T {
        /// Puts back what was installed and closes this hand-over, on the way out however the
        /// way out is taken.
        struct Restore(Option<Parts>, Parts);
        impl Drop for Restore {
            fn drop(&mut self) {
                let previous = self.0.take();
                let _ = HANDING.try_with(|slot| *slot.borrow_mut() = previous);
                self.1.close();
            }
        }
        let previous = HANDING.with(|slot| slot.borrow_mut().replace(self.clone()));
        let _restore = Restore(previous, self.clone());
        run()
    }

    /// Ends a hand-over whose run did not conclude it.
    fn close(&self) {
        let mut state = self.lock();
        if state.ended.is_none() {
            state.spoiled = state.spoiled.or(Some(
                "no page interpretation finished a list for this hand-over",
            ));
            state.ended = Some(state.handed);
        }
        drop(state);
        self.shared.arrived.notify_all();
    }

    /// The next thing handed over, waiting for it at most `wait` — `None` where nothing came.
    ///
    /// Parts come in the order they were stated; [`Handed::Ended`] comes after the last of them
    /// and then again on every call.
    #[must_use]
    pub fn take(&self, wait: Duration) -> Option<Handed> {
        let mut state = self.lock();
        if state.waiting.is_empty() && state.ended.is_none() && state.spoiled.is_none() {
            state = self
                .shared
                .arrived
                .wait_timeout(state, wait)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        if let Some(reason) = state.spoiled {
            state.waiting.clear();
            return Some(Handed::Ended(Err(reason)));
        }
        if let Some(part) = state.waiting.pop_front() {
            return Some(Handed::Part(part));
        }
        state.ended.map(|ended| Handed::Ended(Ok(ended)))
    }

    /// Whether `list` is the list the run that handed these parts over finished with.
    ///
    /// By the allocation rather than by the commands: two interpretations of one page are two
    /// lists, and only the one whose parts were handed over is the one they are a prefix of. The
    /// weak reference keeps the allocation from being reused while it is held, so an equal
    /// address is the same list.
    #[must_use]
    pub fn is(&self, list: &Arc<DisplayList>) -> bool {
        self.lock()
            .list
            .as_ref()
            .is_some_and(|kept| Weak::ptr_eq(kept, &Arc::downgrade(list)))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HandOver> {
        self.shared
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// The reading thread's side: hands over what `list` has stated since the last part, where
    /// a part is due and the interpreter says it may — `may_hand_over` is its word that nothing
    /// it has open could gather the commands pushed so far into anything else.
    pub(crate) fn offer(&self, list: &mut DisplayList, may_hand_over: bool) {
        let taken_back = list.taken_back_since_asked();
        let mut state = self.lock();
        if state.ended.is_some() || state.spoiled.is_some() {
            return;
        }
        if taken_back.is_some_and(|at| at < state.handed.commands) {
            state.spoiled = Some("a command already handed over was taken back by the reader");
            drop(state);
            self.shared.arrived.notify_all();
            return;
        }
        let due = self.shared.first.max(state.handed.commands / PART_GROWTH);
        if !may_hand_over || list.command_count().saturating_sub(state.handed.commands) < due {
            return;
        }
        let Some(part) = list.part_since(state.handed) else {
            state.spoiled = Some("the list no longer holds what was handed over");
            drop(state);
            self.shared.arrived.notify_all();
            return;
        };
        state.handed = part.until();
        state.waiting.push_back(part);
        drop(state);
        self.shared.arrived.notify_all();
    }

    /// Spoils the hand-over for `reason`: the parts handed are no longer known to be the list's,
    /// and the drawer draws the finished list whole (ADR 1855 §1).
    ///
    /// For a reader that handed parts of a list it then gave up and read again (ADR 1880): the
    /// list read again states the same commands, but a part already handed names a list that no
    /// longer exists.
    pub(crate) fn spoil(&self, reason: &'static str) {
        let mut state = self.lock();
        if state.ended.is_none() && state.spoiled.is_none() {
            state.spoiled = Some(reason);
        }
        drop(state);
        self.shared.arrived.notify_all();
    }

    /// [`Self::spoil`], where any part has been handed over: a reader that gave a reading up
    /// before handing anything leaves the drawer nothing to distrust, and the list it reads
    /// again is handed over from its first command (ADR 1894).
    pub(crate) fn spoil_if_handed(&self, reason: &'static str) {
        if self.lock().handed.commands > 0 {
            self.spoil(reason);
        }
    }

    /// The reading thread's side, once the run's list is finished: the last chance to find a
    /// command handed over and then taken back.
    pub(crate) fn conclude(&self, list: &mut DisplayList) {
        let taken_back = list.taken_back_since_asked();
        let mut state = self.lock();
        if state.ended.is_some() {
            return;
        }
        if state.spoiled.is_none() && taken_back.is_some_and(|at| at < state.handed.commands) {
            state.spoiled = Some("a command already handed over was taken back by the reader");
        }
        state.ended = Some(list.stated());
        drop(state);
        self.shared.arrived.notify_all();
    }

    /// Names the list the run that claimed this hand-over finished with, once a caller has
    /// shared it — so that a drawer handed that list later can tell it from any other.
    ///
    /// Taken only once the run has concluded, and only once: the list a caller names first is
    /// the one [`Self::is`] answers for.
    pub fn keep(&self, list: &Arc<DisplayList>) {
        let mut state = self.lock();
        if state.ended.is_some() && state.list.is_none() {
            state.list = Some(Arc::downgrade(list));
        }
    }
}

/// Publishes the running page's figure to whatever this thread has installed, or does nothing.
///
/// Called from the operator loop's periodic check, so it is paid every few dozen operators and
/// never per operator; `bytes` is a closure so that a thread with nothing installed does not even
/// ask the reader. A figure is published only where `bytes` answers — a page's own `/Contents` —
/// because a form's interpreter counts from zero and would set the count back.
pub(crate) fn publish(operators: usize, bytes: impl FnOnce() -> Option<(usize, usize)>) {
    // `try_with`: a thread being torn down is interpreting nothing anybody is waiting for.
    let _ = WATCHING.try_with(|slot| {
        if let Some(progress) = slot.borrow().as_ref()
            && let Some(bytes) = bytes()
        {
            progress.publish(operators, bytes);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_published_where_nothing_is_watching() {
        let progress = Progress::new();
        publish(7, || Some((1, 2)));
        assert_eq!(progress.read(), Reading::default());
    }

    #[test]
    fn a_watch_receives_and_ends_with_its_call() {
        let progress = Progress::new();
        progress.watch(|| publish(40, || Some((5, 10))));
        let read = progress.read();
        assert_eq!((read.operators, read.taken, read.held), (40, 5, 10));
        assert_eq!(read.fraction(), Some(0.5));
        publish(99, || Some((10, 10)));
        assert_eq!(
            progress.read().operators,
            40,
            "the watch ended with the call"
        );
    }

    #[test]
    fn a_form_s_count_does_not_set_the_page_s_back() {
        let progress = Progress::new();
        progress.watch(|| {
            publish(1_000, || Some((50, 100)));
            publish(3, || None);
        });
        assert_eq!(progress.read().operators, 1_000);
    }

    #[test]
    fn a_nested_watch_is_the_inner_one_s_for_its_length() {
        let (outer, inner) = (Progress::new(), Progress::new());
        outer.watch(|| {
            inner.watch(|| publish(1, || Some((1, 4))));
            publish(2, || Some((2, 4)));
        });
        assert_eq!(inner.read().operators, 1);
        assert_eq!(outer.read().operators, 2);
    }

    #[test]
    fn a_relayed_figure_is_handed_on_every_relay_interval_and_at_a_new_page() {
        let relayed = Arc::new(Mutex::new(Vec::new()));
        let progress = Progress::relaying(Box::new({
            let relayed = Arc::clone(&relayed);
            move |reading| relayed.lock().expect("unpoisoned").push(reading.operators)
        }));
        progress.watch(|| {
            for operators in (64..3 * RELAY_EVERY).step_by(64) {
                publish(usize::try_from(operators).expect("small"), || Some((1, 2)));
            }
            publish(64, || Some((1, 2)));
        });
        let relayed = relayed.lock().expect("unpoisoned").clone();
        assert_eq!(relayed, vec![RELAY_EVERY, 2 * RELAY_EVERY, 64]);
        let elsewhere = Progress::new();
        elsewhere.store(progress.read());
        assert_eq!(elsewhere.read(), progress.read());
    }

    #[test]
    fn no_bar_before_the_page_s_stream_and_never_past_its_end() {
        assert_eq!(Reading::default().fraction(), None);
        let torn = Reading {
            operators: 1,
            taken: 12,
            held: 10,
        };
        assert_eq!(torn.fraction(), Some(1.0));
    }
}
