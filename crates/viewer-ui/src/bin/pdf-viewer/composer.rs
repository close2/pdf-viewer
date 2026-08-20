//! The processor on a thread of its own, and the finished window rasters that cross back.
//!
//! **This module is [`crate::renderer`]'s argument, made a second time for the window that has no
//! graphics device.** That one exists because the device is idle for 99.85% of a frame while a
//! processor walks the display list on the calling thread; here there is no device at all and the
//! walk *is* the frame, so the case is stronger rather than weaker. Until the
//! six-hundred-and-twenty-seventh session a `--cpu` frame ran to completion inside
//! `App::present`: nothing could be presented for the whole of it, a stand-in had to be drawn *in
//! front of* the frame it stood in for, and the pages the other window retains could not exist
//! here at all because the thread that draws them while idle did not exist either.
//!
//! # Which thread owns what
//!
//! | | the event thread | the composing thread |
//! |---|---|---|
//! | holds | [`viewer_ui::software::SoftwareSurface`], the base and the retained pages | the display lists of the frame it is drawing |
//! | does | resamples, composites the chrome, copies to the window | `render-cpu` over Table 29's arrangement |
//! | costs | a resample, measured in tens of milliseconds | a frame, measured in hundreds |
//!
//! **What crosses between them is a [`Job`] one way and a [`Finished`] the other**, and both are
//! owned: the pages by the `Arc`s that are their identity, the pixels by value. The thread never
//! sees a `Document`, a `Viewer` or an `App`, so `interpret` stays the pure function of the bytes
//! and the view state the oracle's whole comparison rests on — the same boundary
//! [`crate::renderer`] keeps, for the same reason.
//!
//! # Rule 4 is still a question here, and the render thread changed what it bounds
//!
//! On the window with a device a stand-in is three textured quads and costs the frame it stands in
//! for nothing, which is why ADR 0391 deleted rule 4 there. A resample on this surface is a window
//! of pixels walked by the processor — tens of milliseconds — and it is drawn by the thread that
//! *presents*. So it is not free, and what it can cost is no longer the whole of itself:
//!
//! - the render begins at the tick that noticed the view change and finishes `frame` later, on the
//!   other thread, whatever this one does;
//! - the resample occupies this thread for `resample`, and the true frame is presented at the first
//!   tick after **both** are done.
//!
//! So standing in costs the real frame `max(0, resample − frame)` rather than `resample`. Rule 4's
//! inequality is unchanged — `resample + period ≤ frame`, ADR 0384's form, no constant in it — and
//! what changed is its derivation: it used to say *the resample is added to the frame, so it must
//! buy the refresh it spends*, and it now says *the resample runs beside the frame, so it must be
//! finished a refresh before the frame lands or it is a picture nobody sees and a frame that is
//! late by the difference*. ADR 0461.
//!
//! # What this thread does when nothing is asked of it
//!
//! The same thing [`crate::renderer`]'s does: one whole page at [`crate::stale::PROXY_EDGE`] per
//! idle turn, so that a page turn, a `GoTo` and a resize have pixels to show. Nothing on the launch
//! path makes one — the thread does not exist until the first job, and it draws that job first.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use pdf_render::{DisplayList, Raster, TargetSpec, Transform};
use viewer_ui::software::{SoftwareError, SoftwareSurface};

/// One window frame the event thread asks the composing thread to draw.
///
/// Everything in it is **owned**, because it crosses a thread: each page by the `Arc` whose
/// address is its identity for as long as something pins it, and its placement travels with it
/// because it is that page's and nothing on the other side can recompute one.
#[derive(Debug)]
struct Job {
    /// Every page of Table 29's arrangement and where each goes, in the window's own pixels.
    pages: Vec<crate::stale::Placed>,
}

/// One finished window frame, on its way back to the thread that will present it.
#[derive(Debug)]
struct Done {
    pages: Vec<crate::stale::Placed>,
    /// The window's own pixels, or what refused to draw them.
    ///
    /// A `Result` rather than an `Option` because a page that will not rasterise is a fact about
    /// the document that the core has to be told: it becomes `Rendered::Failed` on the tick that
    /// adopts this, exactly as it did when the frame was drawn inside `App::present`.
    drawn: Result<Raster, String>,
    /// What drawing it cost on this thread.
    cost: Duration,
}

/// What the composing thread sends back: a window frame, or a page it drew while it was idle.
///
/// **One channel rather than two**, for [`crate::renderer`]'s reason: the order the thread
/// produced them in is the order the event thread reads them in, so a retained page that arrived
/// before a frame cannot be adopted after it.
#[derive(Debug)]
enum Finished {
    /// A window frame, boxed because it is much the larger of the two and a channel's item is as
    /// wide as its widest variant.
    Frame(Box<Done>),
    /// A whole page at [`crate::stale::PROXY_EDGE`], drawn while nothing else was asked for.
    Page(crate::stale::Retained<Arc<Raster>>),
}

/// The channels to a running composing thread, and the thread itself.
#[derive(Debug)]
struct Link {
    jobs: Sender<Job>,
    done: Receiver<Finished>,
    /// Joined on the way out, so the thread ends with the program rather than being abandoned.
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Link {
    /// Ends the thread by taking its work away, then waits for it.
    ///
    /// **Dropping the sender is the signal**, which is why there is no stop flag: `recv` on a
    /// channel with no senders returns an error and the loop ends. A join that panicked is ignored
    /// deliberately — this runs while the program is exiting, and a second panic there would
    /// replace whatever the first one said.
    fn drop(&mut self) {
        let (jobs, _) = channel();
        drop(std::mem::replace(&mut self.jobs, jobs));
        if let Some(thread) = self.thread.take() {
            drop(thread.join());
        }
    }
}

/// What one adopted frame tells the host about itself.
///
/// Handed back by [`Composer::collect`] rather than read off a field, for [`crate::renderer`]'s
/// reason: every one of these is a fact about *that* frame, and a caller reading them a tick later
/// would be describing the frame after it.
#[derive(Debug)]
pub(crate) struct Landed {
    /// The pages it drew and where, for [`crate::stale`] to record as the view now settled.
    pub(crate) pages: Vec<crate::stale::Placed>,
    /// What drawing it cost on the composing thread — the frame line's own number.
    pub(crate) cost: Duration,
    /// How long the event thread waited for it, which is what rule 5 predicts the next one by.
    pub(crate) waited: Duration,
    /// What refused, where the page would not draw at all.
    pub(crate) refused: Option<String>,
}

/// This window's half of the arrangement: the surface, the pixels it is showing, and a handle on
/// the thread that draws.
///
/// Lives on the event thread and never leaves it.
pub(crate) struct Composer {
    /// What copies a raster onto a window with no device behind it.
    surface: SoftwareSurface,
    /// The newest finished frame's own pixels — the base a stand-in resamples.
    ///
    /// **It is kept in step with `Stale::settled` by being written in the same breath**, which is
    /// the arrangement ADR 0385 arrived at the hard way: a base and the record of what it is a
    /// picture of, updated by two different events, parted company and the window showed nothing
    /// for want of pixels it was holding. [`Self::collect`] hands back both or neither.
    held: crate::stale::Canvas,
    /// What those pixels are a picture of, which is what says whether this view needs a new frame.
    shown: Vec<crate::stale::Placed>,
    /// The whole pages this window is holding a low-resolution picture of.
    ///
    /// A second copy of what the composing thread holds, and a copy on purpose for
    /// [`crate::renderer`]'s reason: that thread is the authority about which page to draw next,
    /// and what crosses is a finished picture, so the two can differ only by whatever is in the
    /// channel.
    proxies: crate::stale::Proxies<Arc<Raster>>,
    thread: Option<Link>,
    /// When the frame now being drawn was asked for, or `None` when the thread is idle.
    ///
    /// **One job in flight at a time**, exactly as on the other surface: a queue would fill at the
    /// tick rate while a frame takes tens of ticks, and every job in it would be answering a view
    /// the person had already left.
    in_flight: Option<Instant>,
    /// Whether the pixels on hand have reached the window at all, in any form.
    ///
    /// **The window may hold a rendering it has never shown, and that is what this exists to
    /// notice** (ADR 0461). A frame lands at a tick where the view has already moved on: the plan
    /// is then a view change rather than [`crate::stale::Plan::Render`], and where the stand-in is
    /// refused the tick presents nothing — so the window goes on showing a picture that is *older*
    /// than the one in this struct, for as long as the refusal lasts. A rendering that has never
    /// been on the window is strictly truer than the one before it at the same placement, so it
    /// goes up unmoved rather than being held back.
    ///
    /// **Both ways of reaching the window set it**, and the second is what keeps this from being a
    /// jump backwards: a stand-in *is* these pixels, moved, so a tick that stood in has shown them
    /// and putting them up unmoved afterwards would move the picture back to where the view no
    /// longer is.
    presented: bool,
}

impl std::fmt::Debug for Composer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Composer")
            .field("drawing", &self.in_flight.is_some())
            .field("retained", &self.proxies.len())
            .finish_non_exhaustive()
    }
}

impl Composer {
    /// A window drawn on by the processor, retaining `proxy_pages` whole pages.
    pub(crate) fn new(surface: SoftwareSurface, proxy_pages: usize) -> Self {
        Self {
            surface,
            held: crate::stale::Canvas::default(),
            shown: Vec::new(),
            proxies: crate::stale::Proxies::new(proxy_pages),
            thread: None,
            in_flight: None,
            presented: false,
        }
    }

    /// Whether the window is holding a rendering it has never put up.
    ///
    /// See [`Self::presented`]: the answer to a refused stand-in, and the reason a moving view on
    /// this surface still advances rather than freezing until it comes to rest.
    pub(crate) fn unshown(&self) -> bool {
        !self.presented && self.held.frame().is_some()
    }

    /// Whether a frame is being drawn right now — rule 5's second way of knowing that one missed.
    pub(crate) fn drawing(&self) -> bool {
        self.in_flight.is_some()
    }

    /// Whether the pixels on hand are of exactly this arrangement, at exactly these placements.
    ///
    /// The same comparison [`crate::surface`] makes on the other surface and for the same reasons:
    /// each page by the `Arc` that makes its address mean something, because a page turn at an
    /// unchanged magnification is a different picture at the same placement; the targets by value,
    /// because a resize is a different frame at the same transform; and the count with them,
    /// because a scroll that brings a further row of a column on leaves every page already up
    /// exactly where it was.
    pub(crate) fn depicts(&self, pages: &[crate::stale::Placed]) -> bool {
        self.shown.len() == pages.len()
            && self.shown.iter().zip(pages).all(|(drawn, asked)| {
                Arc::ptr_eq(&drawn.list, &asked.list) && drawn.target == asked.target
            })
    }

    /// Takes whatever the composing thread has finished, and keeps the newest of it.
    ///
    /// **Newest rather than each in turn**: a frame that has been overtaken is a picture nobody
    /// will ever see. In practice there is at most one, because [`Self::ask`] keeps one job in
    /// flight.
    pub(crate) fn collect(&mut self) -> Option<Landed> {
        let mut newest = None;
        let mut arrived = Vec::new();
        {
            let link = self.thread.as_ref()?;
            // A thread that has gone is a thread nothing here can restart, so nothing here
            // distinguishes an empty channel from a closed one.
            while let Ok(finished) = link.done.try_recv() {
                match finished {
                    Finished::Frame(done) => newest = Some(done),
                    // Every one of these is kept, unlike the frames: a retained page is a picture
                    // of a page rather than of a moment, so an older one is not superseded by a
                    // newer one of a *different* page.
                    Finished::Page(page) => arrived.push(page),
                }
            }
        }
        for page in arrived {
            self.proxies.keep(page);
        }
        let done = newest?;
        let waited = self
            .in_flight
            .take()
            .map_or(Duration::ZERO, |began| began.elapsed());
        let mut refused = None;
        match done.drawn {
            Ok(raster) => {
                self.held.keep(raster);
                self.shown.clone_from(&done.pages);
                self.presented = false;
            }
            // The pixels on hand are left alone: a page that would not draw says nothing about the
            // picture already on the window, and replacing it with a blank one would be this
            // program throwing away the last true thing it had.
            Err(problem) => refused = Some(problem),
        }
        Some(Landed {
            pages: done.pages,
            cost: done.cost,
            waited,
            refused,
        })
    }

    /// Asks for a frame, spawning the composing thread if this is the first.
    ///
    /// Does nothing while one is already being drawn — see [`Self::in_flight`].
    pub(crate) fn ask(&mut self, pages: Vec<crate::stale::Placed>, now: Instant) {
        if self.in_flight.is_some() || pages.is_empty() {
            return;
        }
        if self.thread.is_none() {
            // The first job is what starts the thread, which is `CLAUDE.md`'s startup rule: a
            // spawn in `resumed` would put a scheduler decision in front of a launch milestone.
            self.thread = Some(spawn(self.proxies.extent()));
        }
        let Some(link) = self.thread.as_ref() else {
            return;
        };
        // A send that fails is a thread that has ended; the frame is not in flight, so the window
        // keeps presenting what it has and asks again.
        if link.jobs.send(Job { pages }).is_ok() {
            self.in_flight = Some(now);
        }
    }

    /// Where each retained page goes so that it depicts this arrangement — [`crate::stale`]'s
    /// answer, asked here because the store lives here.
    pub(crate) fn underlay(&self, pages: &[crate::stale::Placed]) -> Vec<(usize, Transform)> {
        self.proxies.placements(pages)
    }

    /// How many whole pages this window is holding a picture of, for the trace.
    pub(crate) fn retained(&self) -> usize {
        self.proxies.len()
    }

    /// Puts the pixels on hand on the window as they are, under `overlays`.
    ///
    /// **The frame that is of the view being asked for pays no resample at all**, which is why
    /// this is its own method rather than [`Self::stand_in`] at the identity: on the other surface
    /// the identity is a textured quad the device draws either way, and here it would be a walk of
    /// every pixel in the window for a picture that is already correct.
    ///
    /// `Ok(false)` where nothing has been drawn yet, which is every tick between the window
    /// appearing and the first frame landing.
    ///
    /// # Errors
    ///
    /// Whatever the software surface refused.
    pub(crate) fn put_up(&mut self, overlays: &[&DisplayList]) -> Result<bool, SoftwareError> {
        let Some(frame) = self.held.frame() else {
            return Ok(false);
        };
        self.surface.present(frame, overlays)?;
        self.presented = true;
        Ok(true)
    }

    /// Draws the layers a stand-in is made of into one window raster and puts that on the window.
    ///
    /// `base` is [`crate::stale::Stale::reproject`]'s answer and `None` where the sharp layer was
    /// refused; `under` is [`Self::underlay`]'s. What comes back is what the two halves cost —
    /// **the resample and the copy apart**, because they are two different questions: what a
    /// resample of a window of pixels costs is this program's to improve, and what a copy onto the
    /// window costs is the frame's own price, paid again by the true frame a moment later. Rule 4
    /// is judged on the sum, which is what the person waits.
    ///
    /// `Ok(None)` where there was nothing to draw from either layer.
    ///
    /// # Errors
    ///
    /// Whatever the software surface refused.
    pub(crate) fn stand_in(
        &mut self,
        extent: (u32, u32),
        base: Option<Transform>,
        under: &[(usize, Transform)],
        overlays: &[&DisplayList],
    ) -> Result<Option<(Duration, Duration)>, SoftwareError> {
        let began = Instant::now();
        let picture = {
            let layers: Vec<(&Raster, Transform)> = under
                .iter()
                .filter_map(|(index, moved)| {
                    self.proxies
                        .get(*index)
                        .map(|retained| (retained.pixels.as_ref(), *moved))
                })
                .collect();
            self.held.stand_in(extent, base, &layers)
        };
        let Some(picture) = picture else {
            return Ok(None);
        };
        let resample = began.elapsed();
        self.surface.present(&picture, overlays)?;
        // These pixels have been on the window, moved — see [`Self::presented`] for why putting
        // them up unmoved afterwards would be a step backwards rather than forwards.
        self.presented = true;
        Ok(Some((resample, began.elapsed())))
    }
}

/// Starts the composing thread, and hands back the channels to it.
fn spawn(proxy_pages: usize) -> Link {
    let (jobs, incoming) = channel::<Job>();
    let (outgoing, done) = channel::<Finished>();
    // A spawn this machine refuses is a machine with no thread to spare, and the window then
    // presents whatever it has and asks for frames nobody answers — which is what `Link::jobs.send`
    // failing already means, so there is one path rather than two.
    let thread = std::thread::Builder::new()
        .name("page composer".to_owned())
        .spawn(move || compose_until_told_to_stop(proxy_pages, &incoming, &outgoing))
        .ok();
    Link { jobs, done, thread }
}

/// The composing thread's whole life: draw what is asked for, send it back, and draw a whole page
/// at a fraction of the size while nothing else is asked of it.
///
/// Ends when the event thread drops its sender, which is when the window is closing.
///
/// **The order is `CLAUDE.md`'s startup rule here, and it is [`crate::renderer`]'s order.** A job
/// always wins: the channel is asked first, a retained page is drawn only when it is empty, and one
/// page is drawn per idle turn so that a view change arriving mid-way waits for one low-resolution
/// frame rather than for a set of them.
fn compose_until_told_to_stop(proxy_pages: usize, jobs: &Receiver<Job>, done: &Sender<Finished>) {
    use std::sync::mpsc::TryRecvError;

    let mut proxies: crate::stale::Proxies<Arc<Raster>> = crate::stale::Proxies::new(proxy_pages);
    // What the last frame drew, which is the only set of pages this thread has display lists for.
    let mut showing: Vec<crate::stale::Placed> = Vec::new();
    let mut waiting: Option<Job> = None;
    loop {
        let job = match waiting.take() {
            Some(job) => Some(job),
            None => match jobs.try_recv() {
                Ok(job) => Some(job),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => return,
            },
        };
        if let Some(job) = job {
            showing.clone_from(&job.pages);
            let finished = compose(job);
            // A send that fails is an event thread that has gone, and there is nobody left to draw
            // for.
            if done.send(Finished::Frame(Box::new(finished))).is_err() {
                return;
            }
            continue;
        }
        if let Some(page) = proxies.wanted(&showing)
            && let Some(retained) = draw_whole_page(&page)
        {
            // Kept here as well as sent, because this thread is the one that decides what to draw
            // next and a store that forgot would draw the same page for ever.
            proxies.keep(crate::stale::Retained {
                of: retained.of,
                target: retained.target,
                pixels: Arc::clone(&retained.pixels),
            });
            if done.send(Finished::Page(retained)).is_err() {
                return;
            }
            continue;
        }
        match jobs.recv() {
            Ok(job) => waiting = Some(job),
            Err(_) => return,
        }
    }
}

/// One job, drawn: Table 29's arrangement into one window-sized raster.
///
/// The chrome is deliberately not here. It is drawn in window pixels at the identity, it does not
/// move with the page, and it is composited by the thread that presents — which is what lets a
/// stand-in put fresh chrome over an old page, and what makes the raster this hands back the
/// window's own picture *without* the chrome, which is exactly what the other surface's base is.
fn compose(job: Job) -> Done {
    let Job { pages } = job;
    let began = Instant::now();
    let drawn = {
        let borrowed: Vec<(&DisplayList, TargetSpec)> = pages
            .iter()
            .map(|placed| (placed.list.as_ref(), placed.target))
            .collect();
        viewer_ui::software::compose_pages(&borrowed).map_err(|problem| problem.to_string())
    };
    Done {
        cost: began.elapsed(),
        pages,
        drawn,
    }
}

/// Draws one whole page into a raster of its own, for the layer under the base.
///
/// `None` where the page states no usable extent or would not draw — **and it is deliberately
/// silent**, exactly as [`crate::renderer`]'s is, because this is a picture nobody asked for: a
/// refusal costs a stand-in that will not be as good and nothing else, and reporting it would be a
/// note about work the person did not request. It cannot spin: the caller falls through to a
/// blocking `recv` when this answers `None`.
fn draw_whole_page(page: &crate::stale::Placed) -> Option<crate::stale::Retained<Arc<Raster>>> {
    let target = crate::stale::proxy_target(&page.list)?;
    let raster = viewer_ui::software::compose_pages(&[(page.list.as_ref(), target)]).ok()?;
    Some(crate::stale::Retained {
        of: page.of,
        target,
        pixels: Arc::new(raster),
    })
}
