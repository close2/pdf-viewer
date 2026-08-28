//! The rasteriser on a thread of its own, and the finished pages that cross back.
//!
//! # What this is for, and what it is not
//!
//! A tier-1 host is handed [`viewer_core::Event::NeedsRender`] and owes the viewer a
//! [`viewer_core::Command::RenderReady`]. Both native windows used to do the whole of that
//! *inside* the arm that received the event, on the toolkit's main thread — so a page written to
//! be expensive took the window with it. Nothing bounds the number of commands in a display list
//! (ADR 0650): 1567 bytes of PDF amplify to ten thousand page-covering fills and 27.6 s of
//! drawing, and for the whole of it there was no repaint, no key and no way to say stop.
//!
//! [`pdf_render::Interrupt`] is a flag *another* thread raises, so a window with one thread had
//! nothing to raise it from and nothing to raise it about. **The answer is not a watchdog**: a
//! thread whose only job is to raise the flag after a fixed duration is the automatic deadline
//! ADR 0657 measured and refused, and the corpus says no threshold separates the two populations —
//! at twice device scale the median first page of `doc/pdf.js` draws in 2.2 ms, the p99 in
//! 73.9 ms and the slowest of 957 in 252.1 ms, against 27 600 ms for the amplification fixture, so
//! a deadline low enough to catch the fixture refuses one legitimate first page in sixteen.
//!
//! What a window needs is a thread, which `viewer-ui` has had since ADR 0461. This is that
//! arrangement for the hosts that place somebody else's widgets, and it is **one** arrangement in
//! `viewer-host` rather than two copies in two crates — `doc/todo/30`'s "all three hosts stay
//! level", applied to the thing a host does most.
//!
//! # Which thread owns what
//!
//! | | the toolkit's thread | the drawing thread |
//! |---|---|---|
//! | holds | the viewer, the widgets, the textures | one [`viewer_core::RenderRequest`] at a time |
//! | does | asks, collects, answers the viewer, repaints | `render-cpu` over one page's display list |
//! | costs | a channel drain, measured in nanoseconds | a page, measured in milliseconds to seconds |
//!
//! What crosses is a request one way and a [`Finished`] the other, and both are owned: the page by
//! the `Arc<DisplayList>` that is its identity, the pixels by value. The thread never sees a
//! `Document`, a `Viewer` or a widget, which is what keeps `interpret` the pure function of the
//! bytes and the view state the oracle's whole comparison rests on.
//!
//! # When the thread is taken back — the rule, and why it is exact here
//!
//! **A draw is abandoned exactly where the viewer has already stopped wanting its answer**, which
//! for a tier-1 host is a question with a provable answer rather than a judgement:
//! `viewer_core::Viewer` holds one outstanding request per page of Table 29's arrangement and
//! drops a `RenderReady` whose token is not the one outstanding. So a draw whose token the viewer
//! no longer holds cannot change a pixel however long it runs. Two things take a token away and
//! this module acts on both:
//!
//! - **a newer request for the same page** — a resize, a zoom, a re-interpretation. The viewer
//!   overwrites that page's outstanding request as it issues the new one, so the old token is dead
//!   before the host sees the event. [`Drawing::ask`] raises the interrupt for it, and needs
//!   nothing from the host to know.
//! - **the page leaving the arrangement** — a page turn under Table 29's `SinglePage`, where the
//!   whole entry goes and the token with it. That one is [`Drawing::superseded`], because whether
//!   the arrangement still shows a page is `viewer_core::Query::PageGeometry`'s answer and this
//!   crate holds no viewer to ask.
//!
//! **An abandoned draw is answered to nobody**, which is trap 20 and is the half that is easy to
//! get wrong: `viewer_core::Rendered::Failed` records a page as answered for — the scheduler stops
//! asking, deliberately, because a rasteriser that refused this page at this size will refuse it
//! again — and reporting a *failure* for a draw this host merely chose not to finish would freeze
//! the page. So [`Finished::outcome`] is an `Option` and `None` means nothing is owed.
//!
//! # How a host learns that a frame has landed
//!
//! It asks, on a timer that does not exist while nothing is being drawn. **Pulled rather than
//! pushed, because one of the two toolkits cannot be pushed to**: `viewer-qt`'s C++ owns the
//! `Host` for the life of `QApplication::exec` and Rust never calls a Qt object, which is what
//! keeps that crate to one hand-written `unsafe` token (ADRs 0470, 0519, 0526). It is the shape
//! [`crate::Clock`] and `viewer-qt`'s accessibility drain already have, for the same reason: the
//! interval is this crate's decision and the timer is the toolkit's.
//!
//! [`POLL`] is the interval and [`Drawing::interval`] is `None` at rest, so a window with
//! nothing being drawn wakes for this exactly never.
//!
//! # The one place a host does not pull, and why the launch is that place
//!
//! A poll asks the toolkit's loop for a turn, and at launch the toolkit's loop is inside its own
//! first frame and does not give one. Measured (ADR 0678): page one of a five-page document draws
//! in 3.3 ms and `viewer-gtk` waited **57 to 61 ms** for it, because GSK's first frame holds the
//! main loop for that long under `Xvfb`'s software Vulkan — so the whole of the toolkit's most
//! expensive frame landed *in front of* page one instead of beside it, and the launch cost 53 ms
//! against 9.5 where it had rasterised inside the allocation. `viewer-qt` showed none of it, which
//! is what says the fault is the toolkit's first frame rather than this arrangement.
//!
//! [`Drawing::settle`] is the answer and it is deliberately small: a host with **nothing on the
//! screen yet** waits for the page rather than polling for it, for at most [`SETTLE`] over
//! the whole launch. Nothing is interrupted, so a page that outlasts the budget arrives through the
//! poll exactly as before; what the budget buys is that page one is in the toolkit's first frame
//! rather than its second.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use pdf_render::{DisplayList, Interrupt, Rasterizer as _, TargetSpec};
use render_cpu::{CpuRasterError, CpuRasterizer};
use viewer_core::{RenderRequest, Rendered};

/// What one drawing needs to know about the page it was asked for.
///
/// [`viewer_core::RenderRequest`] is the request every tier-1 host holds, and for as long as
/// there were only tier-1 hosts it was this module's only shape. A host on `viewer-confined`'s
/// boundary holds the same three facts — which page, which marks, which target — and **cannot
/// hold that type**, because a `RenderRequest` carries a `RenderToken` only a `viewer_core::Viewer`
/// can mint and the viewer is on the far side of a pipe. The arrangement — one job in flight, a
/// queue keyed by page, the two rules for taking the thread back — is the same on both sides of
/// that difference, and a second copy of it is where two hosts stop agreeing, which is this
/// crate's own reason to exist.
///
/// So the *request* is the parameter and the arrangement is not: implement this for whatever a
/// host queues, and [`Drawing`] treats it exactly as it treats the viewer's own.
pub trait DrawRequest: Send + 'static {
    /// Which page this drawing is of — the identity [`Drawing::ask`] replaces a queued request by,
    /// and the one it raises the interrupt for.
    fn page(&self) -> usize;
    /// The marks to draw.
    fn list(&self) -> &Arc<DisplayList>;
    /// The pixels to draw them into, and the transform to them.
    fn target(&self) -> TargetSpec;
}

impl DrawRequest for RenderRequest {
    fn page(&self) -> usize {
        self.page
    }

    fn list(&self) -> &Arc<DisplayList> {
        &self.list
    }

    fn target(&self) -> TargetSpec {
        self.target
    }
}

/// One page's rasterisation, on its way to the drawing thread.
#[derive(Debug)]
struct Job<R> {
    /// Everything the drawing needs; a [`DrawRequest`] is self-contained on purpose.
    request: R,
    /// The flag the toolkit's thread raises to take this thread back.
    interrupt: Interrupt,
}

/// One page's rasterisation, on its way back.
#[derive(Debug)]
struct Done<R> {
    request: R,
    /// `None` where the interrupt came back instead of a picture — see [`Finished::outcome`].
    outcome: Option<Rendered>,
    cost: Duration,
}

/// The job the toolkit's thread is waiting on, as it remembers it.
#[derive(Debug)]
struct InFlight {
    /// Which request, so that [`Drawing::ask`] can tell a newer one for the same page.
    page: usize,
    /// When it was handed over, which is what [`Finished::waited`] is measured from.
    asked: Instant,
    /// Raised where the viewer has stopped wanting the answer, and never otherwise.
    interrupt: Interrupt,
}

/// One finished page, on its way to the viewer that asked for it.
#[derive(Debug)]
pub struct Finished<R = RenderRequest> {
    /// The request this answers.
    ///
    /// Handed back whole rather than as a token, because the arrival of a page's pixels means
    /// something to a host beyond the answer it owes: §12.4.4.1's transition takes its two faces
    /// from exactly this list and this target.
    pub request: R,
    /// What to tell the viewer, or **nothing at all**.
    ///
    /// `None` is a draw this host took back, and it is deliberately not
    /// [`viewer_core::Rendered::Failed`]: that one is a statement about the *page*, it sets the
    /// page's own record of what is shown, and the scheduler then stops asking. Saying it about a
    /// draw the host merely chose not to finish freezes the page (trap 20). Nothing is owed for an
    /// abandoned draw, because the token it would carry is one the viewer has already dropped.
    pub outcome: Option<Rendered>,
    /// What drawing it cost on the drawing thread — the whole of it, abandoned or not.
    pub cost: Duration,
    /// How long the toolkit's thread waited for it, from the request being handed over.
    pub waited: Duration,
}

/// The channels to a running drawing thread, and the thread itself.
#[derive(Debug)]
struct Link<R> {
    jobs: Sender<Job<R>>,
    done: Receiver<Done<R>>,
    /// Joined on the way out, so the thread ends with the window rather than being abandoned.
    thread: Option<std::thread::JoinHandle<()>>,
}

impl<R> Drop for Link<R> {
    /// Ends the thread by taking its work away, then waits for it.
    ///
    /// **Dropping the sender is the signal**, which is why there is no stop flag: `recv` on a
    /// channel with no senders returns an error and the loop ends. A join that panicked is ignored
    /// deliberately — this runs while the window is closing, and a second panic there would
    /// replace whatever the first one said.
    fn drop(&mut self) {
        let (jobs, _) = channel();
        drop(std::mem::replace(&mut self.jobs, jobs));
        if let Some(thread) = self.thread.take() {
            drop(thread.join());
        }
    }
}

/// A host's half of the arrangement: what is queued, what is being drawn, and the thread drawing
/// it.
///
/// Lives on the toolkit's thread and never leaves it.
///
/// Generic over what a host queues — see [`DrawRequest`] — and `viewer_core::RenderRequest` is
/// the default because the two tier-1 windows are the population this was built for. A field or
/// a binding written `viewer_host::Drawing` means exactly what it meant before the parameter
/// existed.
#[derive(Debug)]
pub struct Drawing<R: DrawRequest = RenderRequest> {
    /// `None` until the first request, which is `CLAUDE.md` section 2's startup rule: a thread
    /// spawned as the window is built is a cost on the launch path, and a thread spawned by the
    /// first page that needs drawing is not.
    link: Option<Link<R>>,
    /// The job the thread is inside, or `None` while it is idle.
    in_flight: Option<InFlight>,
    /// What has been asked for and not yet handed over.
    ///
    /// **A queue rather than one slot**, which is where this differs from `viewer-ui`'s composer
    /// and is forced by the tier: that host draws Table 29's whole arrangement into one window
    /// raster and a superseded frame is simply not drawn, while this one is asked for each page
    /// separately and *owes an answer for every one of them* — a request dropped on the floor is a
    /// page whose outstanding request is never satisfied and which therefore never draws again.
    queued: VecDeque<R>,
    /// Pages drawn on this thread because there was no other one — see [`Drawing::dispatch`].
    landed: Vec<Finished<R>>,
    /// How much of [`SETTLE`] has been spent waiting — see [`Drawing::settle`].
    ///
    /// **Time actually spent blocked, and nothing else.** A launch's budget is a bound on how long
    /// the window may decline to answer, so what fills it is waiting rather than elapsing: a
    /// thousand-page document whose §7.5 cross-reference table takes fourteen milliseconds to read
    /// has not spent any of this, because during those milliseconds there was nothing in flight to
    /// wait for.
    spent: Duration,
}

/// How long a host waits before asking whether the drawing thread has finished.
///
/// **A millisecond, and the number is bounded from both sides rather than picked.** From
/// below: what a poll costs is one `try_recv` on an empty channel, and the timer does not
/// exist at all while nothing is being drawn ([`Drawing::interval`]), so a window at rest pays
/// nothing. From above: this interval is added to every page's latency, and the median page of
/// `doc/pdf.js`'s first pages draws in about two milliseconds — so a poll at one refresh
/// period, the obvious alternative, would be most of a median page turn again.
///
/// A module constant rather than `Drawing`'s own, since the type became generic: an associated
/// constant on a generic type cannot be named without saying which `Drawing` it is of, and this
/// number is the same for every one of them.
pub const POLL: Duration = Duration::from_millis(1);

/// The whole of what a window with nothing on the screen may spend waiting for page one.
///
/// **One 60 Hz refresh, and it is a budget for the launch rather than a per-page timeout** —
/// see [`Drawing::settle`], which is the only thing that reads it and which a host calls with
/// what is left of it. The two sides of the choice:
///
/// - *why wait at all*: a poll cannot be dispatched while the toolkit's own main loop is
///   inside its first frame, and that frame is the expensive one — GTK's is about 55 ms under
///   `Xvfb`'s software Vulkan, which page one waited through in full (ADR 0678).
/// - *why no longer than this*: the wait is time in which the window answers nothing, and one
///   refresh is the longest that is invisible. It is also where the corpus puts the
///   population: at twice device scale 93.9% of `doc/pdf.js`'s first pages draw inside one
///   60 Hz period (ADR 0657), so this admits page one to the toolkit's first frame for nearly
///   all of them and gives up on the rest rather than growing to fit them — the slowest of the
///   957 takes 252 ms and a document written to be expensive takes 27 600, so no bound reaches
///   those without becoming a freeze.
pub const SETTLE: Duration = Duration::from_nanos(1_000_000_000 / 60);

impl<R: DrawRequest> Default for Drawing<R> {
    fn default() -> Self {
        Self {
            link: None,
            in_flight: None,
            queued: VecDeque::new(),
            landed: Vec::new(),
            spent: Duration::ZERO,
        }
    }
}

impl<R: DrawRequest> Drawing<R> {
    /// A host that has drawn nothing yet, and has no thread.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes a request the viewer has made, and starts it where the thread is free.
    ///
    /// **This is the first half of the rule at the head of this module**: a request for a page one
    /// is already being drawn for is the viewer having replaced that page's outstanding request,
    /// so the draw in flight is answering a question nobody holds any more and the thread is taken
    /// back for the new one. Nothing about cost is asked, and no clock is read.
    pub fn ask(&mut self, request: R) {
        if let Some(in_flight) = self.in_flight.as_ref()
            && in_flight.page == request.page()
        {
            in_flight.interrupt.raise();
        }
        // A queued request for the same page is dead for the same reason and has not cost anything
        // yet; it is replaced rather than drawn and thrown away.
        self.queued.retain(|queued| queued.page() != request.page());
        self.queued.push_back(request);
        self.dispatch();
    }

    /// Which page the drawing thread is inside, where it is inside one.
    ///
    /// Asked by a host so that it can answer [`Self::superseded`]: whether Table 29's arrangement
    /// still shows a page is `viewer_core::Query::PageGeometry`'s answer, and this crate holds no
    /// viewer to ask.
    #[must_use]
    pub fn inside(&self) -> Option<usize> {
        self.in_flight.as_ref().map(|in_flight| in_flight.page)
    }

    /// The second half of the rule: takes the thread back from a page the arrangement has stopped
    /// showing.
    ///
    /// `shown` is what `viewer_core::Query::PageGeometry` answered for [`Self::inside`] — a page
    /// the arrangement does not show has no place on the screen and answers
    /// `viewer_core::Answer::None`. Ignored where nothing is being drawn, and where the interrupt
    /// is already up.
    ///
    /// **What abandoning cannot cost, and the one thing it can.** The page the viewer stopped
    /// showing lost its outstanding request with its place in the arrangement, so nothing is owed
    /// and nothing freezes: coming back to that page rebuilds the entry, interprets it again and
    /// asks again. The one case where `shown` is false for another reason is a page whose
    /// *re-interpretation* has newly failed, and there the picture already on the screen is left
    /// exactly as it is — this host answers nothing, so `viewer_core` neither records the page as
    /// shown nor drops the pixels it is holding, and a later interpretation that succeeds issues a
    /// fresh request. That is strictly better than the alternative trap 20 names.
    ///
    /// Answers whether it raised an interrupt, for the trace.
    pub fn superseded(&mut self, shown: bool) -> bool {
        if shown {
            return false;
        }
        let Some(in_flight) = self.in_flight.as_ref() else {
            return false;
        };
        if in_flight.interrupt.raised() {
            return false;
        }
        in_flight.interrupt.raise();
        true
    }

    /// Takes whatever has finished, and starts the next thing that is waiting.
    ///
    /// Everything in the answer is owed to the viewer except a [`Finished`] whose
    /// [`outcome`](Finished::outcome) is `None`, which is owed to nobody.
    pub fn collect(&mut self) -> Vec<Finished<R>> {
        let mut finished = std::mem::take(&mut self.landed);
        let mut arrived = Vec::new();
        if let Some(link) = self.link.as_ref() {
            // A thread that has gone is a thread this cannot restart from here; `dispatch` is
            // where that is noticed, so nothing here distinguishes an empty channel from a closed
            // one.
            while let Ok(done) = link.done.try_recv() {
                arrived.push(done);
            }
        }
        for done in arrived {
            let asked = self.in_flight.take().map(|in_flight| in_flight.asked);
            finished.push(landed(done, asked));
        }
        self.dispatch();
        finished
    }

    /// [`Self::collect`], having first waited for the page in flight out of a `budget`.
    ///
    /// **For a window that has not put a frame on the screen yet, and for nothing else.** A host
    /// that has presented something owes a person a live window and must not block its toolkit's
    /// loop at all; a host that has presented nothing has no frame to spoil and no input to lose,
    /// and the thing it is waiting for is the only thing it exists to show. ADR 0678 is the
    /// measurement that put this here and [`SETTLE`] is what a host passes.
    ///
    /// **`budget` is the whole launch's rather than this call's**, which is why the accounting is
    /// here and not in the two hosts: Table 29's arrangement asks for every page it shows, so a
    /// column asks two or three times before the first frame and a per-call bound would multiply
    /// by however many pages a document chose to open in. A host calls this with the same
    /// [`SETTLE`] each time and the remainder shrinks.
    ///
    /// **This is not a deadline on the drawing and takes no thread back.** Nothing is interrupted
    /// and nothing is abandoned: a page still unfinished when the budget runs out stays in flight
    /// and arrives through [`Self::interval`]'s poll exactly as it did before, one toolkit frame
    /// later. So the two conditions in this module's head are still the only two that raise an
    /// interrupt, and the automatic deadline ADR 0657 refused is still refused.
    pub fn settle(&mut self, budget: Duration) -> Vec<Finished<R>> {
        let left = budget.saturating_sub(self.spent);
        let arrived = match (self.in_flight.as_ref(), self.link.as_ref()) {
            // Nothing in flight, or no thread because `dispatch` fell back to drawing on this one
            // — in both cases whatever there is to have is already in `landed` or in the channel,
            // and a `recv_timeout` here would be a window frozen for the budget over nothing.
            (Some(_), Some(link)) if !left.is_zero() => {
                let began = Instant::now();
                let arrived = link.done.recv_timeout(left).ok();
                self.spent = self.spent.saturating_add(began.elapsed());
                arrived
            }
            _ => None,
        };
        if let Some(done) = arrived {
            let asked = self.in_flight.take().map(|in_flight| in_flight.asked);
            self.landed.push(landed(done, asked));
        }
        self.collect()
    }

    /// How long the host should wait before asking again, or `None` while there is nothing to ask
    /// about.
    ///
    /// **`None` is the whole of what keeps this off a resting window**: a host arms a one-shot from
    /// this and arms nothing where it answers `None`, so a window showing a drawn page wakes for
    /// this exactly never.
    #[must_use]
    pub fn interval(&self) -> Option<Duration> {
        let waiting =
            self.in_flight.is_some() || !self.queued.is_empty() || !self.landed.is_empty();
        waiting.then_some(POLL)
    }

    /// Hands the next queued request to the thread, spawning one if this is the first.
    ///
    /// **The fallback draws on the calling thread and is not a second policy.** A machine that will
    /// not spawn a thread, or a thread that has ended under this one, would otherwise leave a page
    /// with an outstanding request nothing will ever answer — and a page that never draws again is
    /// worse than a page drawn slowly. What that arm gives up is the ability to be interrupted,
    /// which is what a host without a second thread has to give up anyway; it is the arrangement
    /// this module replaced, reached only where the arrangement is unavailable.
    fn dispatch(&mut self) {
        while self.in_flight.is_none() {
            let Some(request) = self.queued.pop_front() else {
                return;
            };
            if self.link.is_none() {
                self.link = spawn();
            }
            let interrupt = Interrupt::new();
            let asked = Instant::now();
            let page = request.page();
            let job = Job {
                request,
                interrupt: interrupt.clone(),
            };
            let returned = match self.link.as_ref() {
                Some(link) => link.jobs.send(job).err().map(|refused| refused.0),
                None => Some(job),
            };
            let Some(job) = returned else {
                self.in_flight = Some(InFlight {
                    page,
                    asked,
                    interrupt,
                });
                return;
            };
            self.link = None;
            let done = draw(job);
            self.landed.push(landed(done, Some(asked)));
        }
    }
}

/// One finished job, with what the wait cost added to what the drawing cost.
///
/// `asked` is `None` only for a job whose record the host has already taken, which cannot happen
/// while there is one job in flight at a time — the wait is reported as zero rather than guessed.
fn landed<R>(done: Done<R>, asked: Option<Instant>) -> Finished<R> {
    Finished {
        request: done.request,
        outcome: done.outcome,
        cost: done.cost,
        waited: asked.map_or(Duration::ZERO, |asked| asked.elapsed()),
    }
}

/// Starts the drawing thread, and hands back the channels to it.
///
/// `None` where the machine refused the spawn, which [`Drawing::dispatch`] answers by drawing the
/// page itself.
fn spawn<R: DrawRequest>() -> Option<Link<R>> {
    let (jobs, incoming) = channel::<Job<R>>();
    let (outgoing, done) = channel::<Done<R>>();
    let thread = std::thread::Builder::new()
        .name("page rasteriser".to_owned())
        .spawn(move || draw_until_told_to_stop(&incoming, &outgoing))
        .ok()?;
    Some(Link {
        jobs,
        done,
        thread: Some(thread),
    })
}

/// The drawing thread's whole life: draw what is asked for and send it back.
///
/// Ends when the host drops its sender, which is when the window is closing.
fn draw_until_told_to_stop<R: DrawRequest>(jobs: &Receiver<Job<R>>, done: &Sender<Done<R>>) {
    while let Ok(job) = jobs.recv() {
        // A send that fails is a host that has gone, and there is nobody left to draw for.
        if done.send(draw(job)).is_err() {
            return;
        }
    }
}

/// One page, drawn.
///
/// **A rasteriser per job rather than one held across them**, which costs nothing — `CpuRasterizer`
/// is a handful of settings and no state — and buys the one thing this needs:
/// [`CpuRasterizer::interruptible`] takes the flag by value, and a flag that outlived the draw it
/// was raised for would abandon the *next* page before it had drawn a command.
///
/// **`render-cpu` by name rather than a `Rasterizer` behind a parameter**, and that is the same
/// decision ADR 0650 took one layer down: it is the only backend in this tree that can be
/// interrupted at all, because the loop is ours and a hostile document arrives in it as data. A
/// device backend was deliberately given no such method rather than one it would ignore, so a
/// parameter here would be a choice between one implementation and one that silently could not
/// stop.
fn draw<R: DrawRequest>(job: Job<R>) -> Done<R> {
    let Job { request, interrupt } = job;
    let began = Instant::now();
    let outcome = match CpuRasterizer::new()
        .interruptible(interrupt)
        .rasterize(request.list(), request.target())
    {
        Ok(raster) => Some(Rendered::Raster(raster)),
        // **Asked of the error rather than of the flag**, which is not fastidiousness: the flag can
        // go up in the moment between the last command and the return, and a page that was finished
        // before it did is a page this window can show.
        Err(problem) if was_interrupted(&problem) => None,
        // Trap 5: a host that quietly kept the previous page would be telling a person something
        // false about this one.
        Err(problem) => Some(Rendered::Failed(problem.to_string())),
    };
    Done {
        request,
        outcome,
        cost: began.elapsed(),
    }
}

/// Whether what the rasteriser refused with is the interrupt coming back.
fn was_interrupted(problem: &CpuRasterError) -> bool {
    matches!(
        problem,
        CpuRasterError::Target(pdf_render::BackendError::Interrupted)
    )
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use pdf_render::{
        BlendMode, Color, Command as Mark, DisplayList, FillRule, Paint, Path as MarkPath,
        PathCommand, Point, Transform,
    };
    use viewer_core::{Command, DocumentId, Event, RenderRequest, Rendered, Viewer};

    use super::{Drawing, Finished, POLL, SETTLE};

    /// How long a test waits for a thread before calling it stuck.
    ///
    /// Generous on purpose: these run beside three other rounds on a shared machine, and a
    /// timeout tight enough to be a measurement would be a measurement of the load (749).
    const GIVE_UP: Duration = Duration::from_mins(2);

    /// A document committed in `doc/`, which every checkout has — the same one
    /// `viewer-core/tests/headless.rs` opens, and for the same reason: the corpus is an optional
    /// submodule and a test that skipped itself silently would be worse than no test.
    fn a_real_request() -> RenderRequest {
        let path: PathBuf =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf");
        let bytes = std::fs::read(&path).expect("the document is committed beside this crate");
        let mut viewer = Viewer::new(600, 800, 1.0);
        let events: Vec<Event> = viewer
            .handle(Command::Open {
                id: DocumentId(1),
                bytes,
                password: None,
                fragment: None,
            })
            .collect();
        events
            .into_iter()
            .find_map(|event| match event {
                Event::NeedsRender(request) => Some(request),
                _ => None,
            })
            .expect("opening a five-page document asks for its first page")
    }

    /// The same request, aimed at another page and over another display list.
    ///
    /// **The token stays the one the viewer minted**, because [`RenderToken`] is opaque and there
    /// is no way to invent one — which is the type doing its job. Nothing here compares two
    /// tokens: what [`Drawing`] keys on is the page, and what a test asserts is the page.
    fn like(request: &RenderRequest, page: usize, list: &Arc<DisplayList>) -> RenderRequest {
        RenderRequest {
            page,
            list: Arc::clone(list),
            ..request.clone()
        }
    }

    /// A display list holding `fills` page-covering fills, at the request's own size.
    ///
    /// The construction `crates/viewer-confined/tests/support/amplification.rs` writes as a file,
    /// built here directly: nothing bounds the number of commands in a list, so what a page costs
    /// to draw is chosen by choosing how many of them to state (ADR 0650).
    fn amplified(request: &RenderRequest, fills: usize) -> Arc<DisplayList> {
        let size = request.list.page_size;
        let mut list = DisplayList::new(size);
        let mut path = MarkPath::new();
        path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
        path.push(PathCommand::LineTo(Point::new(size.width, 0.0)));
        path.push(PathCommand::LineTo(Point::new(size.width, size.height)));
        path.push(PathCommand::LineTo(Point::new(0.0, size.height)));
        path.push(PathCommand::Close);
        let path = Arc::new(path);
        for _ in 0..fills {
            list.push(Mark::Fill {
                path: Arc::clone(&path),
                transform: Transform::IDENTITY,
                paint: Paint::Solid(Color::rgb(0.1, 0.25, 0.5)),
                fill_rule: FillRule::NonZero,
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            });
        }
        Arc::new(list)
    }

    /// Waits until something has been drawn, so that a test measures a drawing and not a
    /// scheduler.
    fn wait(drawing: &mut Drawing) -> Vec<Finished> {
        let began = Instant::now();
        loop {
            let finished = drawing.collect();
            if !finished.is_empty() {
                return finished;
            }
            assert!(
                began.elapsed() < GIVE_UP,
                "the drawing thread never answered"
            );
            std::thread::sleep(POLL);
        }
    }

    /// The point of the module: a page asked for comes back drawn, from somewhere else.
    #[test]
    fn a_page_asked_for_comes_back_as_a_raster() {
        let request = a_real_request();
        let mut drawing = Drawing::new();
        drawing.ask(request.clone());
        let finished = wait(&mut drawing);
        assert_eq!(finished.len(), 1);
        assert_eq!(finished[0].request.token, request.token);
        assert!(matches!(finished[0].outcome, Some(Rendered::Raster(_))));
    }

    /// `CLAUDE.md` section 2: nothing on the launch path that page one does not need.
    ///
    /// A host that has been built and asked for nothing has no thread and wants no wakeup.
    #[test]
    fn no_thread_exists_until_a_page_is_asked_for() {
        // Annotated because nothing else here constrains the request type: the default parameter
        // applies in a type position, which an expression is not.
        let drawing: Drawing = Drawing::new();
        assert!(drawing.link.is_none());
        assert_eq!(drawing.interval(), None);
    }

    /// ADR 0678: a window with nothing on the screen waits for page one rather than polling for
    /// it, and one call is enough for an ordinary page.
    ///
    /// The assertion is about the *answer* rather than about a clock, which is 749's rule: on a
    /// machine that gave the drawing thread no core the wait would run out and the page would
    /// arrive through the poll instead, so timing this would measure the machine.
    #[test]
    fn a_launch_waits_for_page_one_instead_of_polling_for_it() {
        let request = a_real_request();
        let mut drawing = Drawing::new();
        drawing.ask(request.clone());
        let finished = drawing.settle(SETTLE);
        assert_eq!(finished.len(), 1, "one settle answered page one");
        assert_eq!(finished[0].request.token, request.token);
        assert!(matches!(finished[0].outcome, Some(Rendered::Raster(_))));
    }

    /// And the budget is a budget rather than a deadline: a page that outlasts it is still being
    /// drawn, and the host's poll is still what brings it back.
    #[test]
    fn a_page_that_outlasts_the_budget_is_not_taken_back() {
        let request = a_real_request();
        let mut drawing = Drawing::new();
        drawing.ask(like(&request, 0, &amplified(&request, 20_000)));
        assert!(
            drawing.settle(Duration::ZERO).is_empty(),
            "nothing had finished"
        );
        assert_eq!(drawing.inside(), Some(0), "and it is still being drawn");
        assert_eq!(drawing.interval(), Some(POLL));
        let finished = wait(&mut drawing);
        assert!(
            matches!(finished[0].outcome, Some(Rendered::Raster(_))),
            "no interrupt was raised, so the page came back whole"
        );
    }

    /// And the budget is the launch's rather than the call's.
    ///
    /// Table 29's `OneColumn` asks for every page it shows, so a window asks two or three times
    /// before its first frame; a bound that started again on each call would multiply by however
    /// many pages a document chose to open in. Asserted on what was *spent* rather than on a
    /// clock, which is 749's rule.
    #[test]
    fn the_budget_is_spent_once_over_a_whole_launch() {
        let request = a_real_request();
        let mut drawing = Drawing::new();
        drawing.ask(like(&request, 0, &amplified(&request, 20_000)));
        assert!(
            drawing.settle(SETTLE).is_empty(),
            "the page outlasted the budget"
        );
        assert!(drawing.spent >= SETTLE, "and spent the whole of it");
        let spent = drawing.spent;
        assert!(drawing.settle(SETTLE).is_empty());
        assert_eq!(drawing.spent, spent, "so a second call waited for nothing");
    }

    /// A settle on a host that has asked for nothing is a settle that returns at once.
    ///
    /// The one that would hurt is a `recv_timeout` on a channel nobody will send to, which is a
    /// window frozen for the budget at every launch that has yet to ask for a page.
    #[test]
    fn a_settle_before_anything_is_asked_for_waits_for_nothing() {
        // Annotated for `no_thread_exists_until_a_page_is_asked_for`'s reason.
        let mut drawing: Drawing = Drawing::new();
        let began = Instant::now();
        assert!(drawing.settle(Duration::from_secs(30)).is_empty());
        assert!(began.elapsed() < GIVE_UP, "it did not wait for the budget");
    }

    /// And a window showing a drawn page goes back to wanting none.
    #[test]
    fn a_window_at_rest_asks_for_no_wakeups() {
        let mut drawing = Drawing::new();
        drawing.ask(a_real_request());
        assert_eq!(drawing.interval(), Some(POLL));
        let _ = wait(&mut drawing);
        assert_eq!(drawing.interval(), None);
    }

    /// The rule's first half: a newer request for the same page takes the thread back, and the
    /// draw it took back is answered to **nobody**.
    ///
    /// **Trap 20 is the assertion that matters.** A `Rendered::Failed` here would set that page's
    /// record of what is shown and stop the scheduler ever asking again, which is a freeze rather
    /// than a report.
    #[test]
    fn a_newer_request_for_a_page_takes_the_thread_back_and_owes_nothing() {
        let request = a_real_request();
        let expensive = amplified(&request, 20_000);
        let mut drawing = Drawing::new();
        drawing.ask(like(&request, 0, &expensive));
        drawing.ask(like(&request, 0, &request.list));
        let (mut abandoned, mut drawn) = (0_usize, 0_usize);
        let began = Instant::now();
        while drawn == 0 {
            for finished in drawing.collect() {
                match finished.outcome {
                    None => abandoned += 1,
                    Some(Rendered::Raster(_)) => drawn += 1,
                    Some(other) => panic!("the page refused to draw: {other:?}"),
                }
            }
            assert!(began.elapsed() < GIVE_UP, "nothing was ever drawn");
            std::thread::sleep(POLL);
        }
        assert_eq!(abandoned, 1, "the superseded draw was not taken back");
    }

    /// The rule's second half: a page Table 29's arrangement has stopped showing.
    #[test]
    fn a_page_that_left_the_arrangement_takes_the_thread_back() {
        let request = a_real_request();
        let expensive = amplified(&request, 20_000);
        let mut drawing = Drawing::new();
        drawing.ask(like(&request, 0, &expensive));
        assert_eq!(drawing.inside(), Some(0));
        assert!(drawing.superseded(false), "the interrupt was not raised");
        assert!(
            !drawing.superseded(false),
            "one draw was interrupted twice, so the trace would count two"
        );
        let finished = wait(&mut drawing);
        assert_eq!(finished.len(), 1);
        assert!(
            finished[0].outcome.is_none(),
            "an abandoned draw owes the viewer nothing"
        );
    }

    /// A page the arrangement still shows is left alone, whatever it costs.
    #[test]
    fn a_page_the_arrangement_still_shows_is_left_alone() {
        let request = a_real_request();
        let mut drawing = Drawing::new();
        drawing.ask(request);
        assert!(!drawing.superseded(true));
        let finished = wait(&mut drawing);
        assert!(matches!(finished[0].outcome, Some(Rendered::Raster(_))));
    }

    /// Table 29's column asks for several pages at once and **every one of them is owed an
    /// answer**: a request dropped for want of a free thread is a page whose outstanding request
    /// is never satisfied, and which therefore never draws again.
    #[test]
    fn every_page_of_an_arrangement_is_answered() {
        let request = a_real_request();
        let mut drawing = Drawing::new();
        for page in 0..4 {
            drawing.ask(like(&request, page, &request.list));
        }
        let mut answered = Vec::new();
        let began = Instant::now();
        while answered.len() < 4 {
            for finished in drawing.collect() {
                assert!(finished.outcome.is_some(), "a page was left unanswered");
                answered.push(finished.request.page);
            }
            assert!(began.elapsed() < GIVE_UP, "not every page was answered");
            std::thread::sleep(POLL);
        }
        answered.sort_unstable();
        assert_eq!(answered, vec![0, 1, 2, 3]);
    }

    /// A page asked for twice while the thread is busy is drawn once, not twice.
    #[test]
    fn a_queued_request_is_replaced_rather_than_drawn_and_thrown_away() {
        let request = a_real_request();
        let slow = amplified(&request, 20_000);
        let mut drawing = Drawing::new();
        // Page 0 occupies the thread; pages 1 and 1 again queue behind it.
        drawing.ask(like(&request, 0, &slow));
        drawing.ask(like(&request, 1, &request.list));
        drawing.ask(like(&request, 1, &request.list));
        assert!(drawing.superseded(false));
        let mut pages = Vec::new();
        let began = Instant::now();
        while pages.len() < 2 {
            for finished in drawing.collect() {
                pages.push(finished.request.page);
            }
            assert!(began.elapsed() < GIVE_UP, "the queue never drained");
            std::thread::sleep(POLL);
        }
        assert_eq!(pages, vec![0, 1], "page 1 was drawn twice");
    }
}
