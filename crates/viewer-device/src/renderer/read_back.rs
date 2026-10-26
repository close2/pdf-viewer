//! The device for a window that is handed rasters: each page drawn on the graphics device on a
//! thread of its own and read back, its lane chosen by the rules `quorra`'s frames are chosen by
//! (ADR 2049's G1 and Q1, ADR 2070).
//!
//! # What this is for
//!
//! The toolkit windows place their own widgets and show a page as a `Raster` copied into the
//! toolkit's pixel surface (`gdk::MemoryTexture`, `QImage`), and neither toolkit hands a program a
//! device (ADRs 0244, 0246). So the first device path open to them is the one with no handle in
//! it: the device draws the page into a texture of its own, the texture is read back, and the
//! window is given `Rendered::Raster` exactly as the processor gives it one. No boundary message
//! changes, and no `unsafe` is added anywhere.
//!
//! # Which lane draws a page
//!
//! **The one `quorra` would draw it in, by the same records.** `renderer::lanes`'s `Costs`
//! predicts each lane's price per command for the page and its `Refusals` remembers what the
//! device refused: a page the device is predicted to draw inside `viewer_host::drawing::WARN` is
//! the device's whatever the processor's prediction, and past it the lane with the smaller
//! prediction draws (principle 2, ADR 1831). A page the device refused at a magnification, or
//! whose coverage lane it refused, is not asked of it again. **What the processor draws, it draws
//! on the window's drawing thread** (`viewer_host::Drawing`), where it can be shown as it is built,
//! taken back and stopped — so a request this lane will not draw is handed back rather than drawn
//! here.
//!
//! # A window ahead of its page
//!
//! **Where the view moved, the part of the page the viewport shows is drawn first** (ADR 2322): the
//! window's own target on the page's transform, on the coverage lane `quorra` takes for a moved
//! view (`lane::lane_for`, the compute lane on a device worth the name), read back at the window's
//! size and handed to the window as a picture of the request — no answer to the viewer — and then
//! the page itself as before, on the launch rule's lane. A toolkit's notch was the read back, the
//! copy and the paint of the whole page after an encode the processor's glyph lane redoes at every
//! magnification (ADRs 2314, 2315); the window is a third to a half of those pixels, and the
//! compute lane flattens its outlines on the device, so no glyph is rasterised again on the host at
//! a new magnification. Each window is a fresh scene through `Rasterizer::rasterize`, which keeps
//! no records between draws; keeping them would buy nothing, because the walk is a fraction of a
//! millisecond of the window's device job and the rest is the lane's own passes (ADR 2338). What
//! the window shows at rest is the page drawn as before, so the window's lane differs from it only
//! until the page lands. A launch, a view that did not move and a page the viewport holds whole
//! draw the page alone.
//!
//! # What crosses
//!
//! | | the window's thread | the device's thread |
//! |---|---|---|
//! | holds | the queue, the records of each lane's price and refusals | `QuorraRasterizer`: the device and its caches |
//! | does | chooses the lane, answers the core | brings the device up, draws, reads back |
//!
//! A request goes one way as its list, its target and its coverage lane; a raster or the device's
//! refusal comes back. The thread never sees a `Viewer` or a `Document`.
//!
//! # What the startup rules bind here
//!
//! **The device comes up off the window's thread and nothing waits for warmth**: [`ReadBack::
//! bring_up`] spawns the thread and returns, the thread makes the device — whose pipelines raster
//! compiles in the background — and page one is drawn by it as soon as it exists. No frame is drawn
//! on the processor to cover the bring-up. A machine with no device is told once on the frames
//! trace, and the processor draws every page, saying nothing new (ADR 2049 section 4).

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::{Duration, Instant};

use pdf_render::{DisplayList, Raster, Rasterizer as _, TargetSpec};
use render_raster::{FrameCost, QuorraRasterizer};
use viewer_core::{RenderRequest, Rendered};
use viewer_host::drawing::{Finished, Handed, POLL, PageLane};

use viewer_host::oversize::Window;

use super::lanes::{Costs, Kept, Lane, Prediction, Refusals, remembered};
use crate::stale::{Depicts, Picture, Placed};

/// The graphics device for a window whose pages are handed to it as rasters (ADR 2070).
///
/// Lives on the window's thread and never leaves it; the device is on a thread of its own from
/// [`Self::bring_up`] on.
#[derive(Debug)]
pub struct ReadBack {
    /// What the thread calls once something it sent has landed, where the window's loop can be
    /// woken; `None` for a window that looks on its own timer ([`Self::interval`]).
    wake: Option<crate::Wake>,
    /// The thread and its channels, from [`Self::bring_up`] or the first page asked for.
    link: Option<Link>,
    /// What is known of the device.
    device: Device,
    /// What each lane charged per command for each page it drew.
    costs: Costs,
    /// What the device refused.
    refusals: Refusals,
    /// The page the thread is drawing.
    in_flight: Option<Flight>,
    /// What is asked for and not yet sent — one request per page of a document, the newest.
    queued: VecDeque<Queued>,
    /// The transform each page was last offered at, newest first: a page offered at another is a
    /// view that moved, whose window is drawn ahead of it ([`Self::ahead_of`]).
    views: VecDeque<(viewer_core::DocumentId, usize, pdf_render::Transform)>,
    /// The requests handed back to the drawing thread whose draws are this lane's to price.
    priced: Vec<Placed>,
    /// What [`PageLane::collect`] hands back next.
    handed: Handed,
    /// ADR 1831's sentence about the last page drawn elsewhere than on the device.
    title: Option<String>,
    /// How much of a launch's settling budget has been spent waiting, as `viewer_host::Drawing`
    /// keeps its own.
    spent: Duration,
    /// The next job's number.
    serial: u64,
}

/// What the window knows of the device.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Device {
    /// Nothing has been asked of it yet.
    NotAsked,
    /// The thread is bringing it up.
    Coming,
    /// It is up, on the adapter named.
    Up(String),
    /// No device could be made, in the words of the refusal; the processor draws every page.
    Absent(String),
}

/// What each step of the device's bring-up cost, as raster and this host timed them on the
/// device's thread — the launch gate's bring-up child prints the same steps (ADR 2282).
///
/// **Said in the window's trace because the window's figure is not the child's.** The child brings
/// a device up in a process that has done nothing else; a window brings it up while the toolkit
/// starts on the main thread, and the difference between the two is a term only this line can
/// split.
#[derive(Debug, Clone, Copy)]
struct Steps {
    /// The instance and its adapter check, where this host made them.
    launch: Option<raster_gpu::LaunchSteps>,
    /// The device's own steps.
    startup: raster_gpu::StartupTimings,
}

impl Steps {
    /// The steps in the order they ran, and what of `took` none of them names: the device's
    /// assembly and the host's caches.
    fn said(&self, took: Duration) -> String {
        let shown = |step: Option<Duration>| {
            step.map_or_else(
                || "-".to_owned(),
                |step| format!("{:.1}", step.as_secs_f64() * 1e3),
            )
        };
        let launch = self.launch;
        let named = [
            launch.map(|steps| steps.render_node_wake),
            launch.map(|steps| steps.instance_creation),
            launch.map(|steps| steps.adapter_check),
            Some(self.startup.adapter_selection),
            Some(self.startup.device_creation),
        ]
        .into_iter()
        .flatten()
        .fold(Duration::ZERO, Duration::saturating_add);
        format!(
            "render-node wake {} + instance {} + adapter check {} + adapter {} + request_device {} \
             + the device's assembly and the host's {} ms",
            shown(launch.map(|steps| steps.render_node_wake)),
            shown(launch.map(|steps| steps.instance_creation)),
            shown(launch.map(|steps| steps.adapter_check)),
            shown(Some(self.startup.adapter_selection)),
            shown(Some(self.startup.device_creation)),
            shown(Some(took.saturating_sub(named))),
        )
    }
}

/// The channels to the device's thread.
#[derive(Debug)]
struct Link {
    jobs: Sender<Job>,
    done: Receiver<Message>,
    /// Joined on the way out, so the device is shut down on the thread that owns it.
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Link {
    /// Ends the thread by taking its work away, then waits for it — `renderer`'s `Link`'s reason.
    fn drop(&mut self) {
        let (jobs, _) = channel();
        drop(std::mem::replace(&mut self.jobs, jobs));
        if let Some(thread) = self.thread.take() {
            // A thread that panicked has nothing left to shut down, and this runs while the window
            // is closing, where a second panic would replace whatever the first said.
            drop(thread.join());
        }
    }
}

/// Most pages whose last view is remembered: the few an arrangement shows at once, as
/// `viewer_host::oversize` holds as many windows.
const VIEWS: usize = 4;

/// A request waiting for the device, and the window to draw of it first, where there is one.
#[derive(Debug)]
struct Queued {
    request: RenderRequest,
    /// The part of the page the viewport showed when it was offered, and the lane to draw it on;
    /// taken when it is sent, so the page follows it.
    ahead: Option<(Window, raster_gpu::Coverage)>,
}

/// One page, or one window of a page, for the device to draw.
#[derive(Debug)]
struct Job {
    serial: u64,
    list: Arc<DisplayList>,
    target: TargetSpec,
    coverage: raster_gpu::Coverage,
}

/// The page the thread is drawing, as the window remembers it.
#[derive(Debug)]
struct Flight {
    serial: u64,
    request: RenderRequest,
    asked: Instant,
    coverage: raster_gpu::Coverage,
    /// The window drawn ahead of the page, or `None` for the page itself.
    window: Option<Window>,
    /// Whether a newer request for the page was offered while this window was drawn, so that it
    /// shows nothing anybody asked for any more.
    superseded: bool,
}

/// What the device's thread sends back.
#[derive(Debug)]
enum Message {
    /// The device is up, on this adapter, after this long, and what each of its steps cost.
    Up(String, Duration, Steps),
    /// No device could be made: what refused.
    Absent(String),
    /// A job answered: the page read back, or the device's refusal and what it is a fact about,
    /// with what raster says each of its stages cost — boxed, being several times the size of
    /// every other variant.
    Drawn {
        serial: u64,
        outcome: Result<Raster, (String, Option<Kept>)>,
        took: Duration,
        phases: Box<FrameCost>,
    },
    /// A job the thread could not draw because it has no device.
    NoDevice(u64),
}

impl ReadBack {
    /// A lane with no thread and no device yet, waking the window with `wake` where it has one.
    #[must_use]
    pub fn new(wake: Option<crate::Wake>) -> Self {
        Self {
            wake,
            link: None,
            device: Device::NotAsked,
            costs: Costs::default(),
            refusals: Refusals::default(),
            in_flight: None,
            queued: VecDeque::new(),
            views: VecDeque::new(),
            priced: Vec::new(),
            handed: Handed::default(),
            title: None,
            spent: Duration::ZERO,
            serial: 0,
        }
    }

    /// Starts the device's thread, which makes the device, and returns at once.
    ///
    /// Called by a window as it is built, so the device comes up beside the document's read rather
    /// than after it; a page asked for before is drawn as soon as the device exists. A machine that
    /// refuses the thread is a machine with no device here, and the processor draws.
    pub fn bring_up(&mut self) {
        if self.link.is_some() || self.device != Device::NotAsked {
            return;
        }
        let (jobs, incoming) = channel::<Job>();
        let (outgoing, done) = channel::<Message>();
        let wake = self.wake.clone();
        let spawned = std::thread::Builder::new()
            .name("page device".to_owned())
            .spawn(move || draw_until_told_to_stop(&incoming, &outgoing, wake.as_ref()));
        match spawned {
            Ok(thread) => {
                self.link = Some(Link {
                    jobs,
                    done,
                    thread: Some(thread),
                });
                self.device = Device::Coming;
            }
            Err(refused) => self.absent(format!("the device's thread was refused: {refused}")),
        }
    }

    /// The adapter the device is on, once it is up.
    #[must_use]
    pub fn adapter(&self) -> Option<&str> {
        match &self.device {
            Device::Up(adapter) => Some(adapter),
            _ => None,
        }
    }

    /// Whether the device has `request`, or the drawing thread is to draw it — and why, where it
    /// is.
    fn choose(&mut self, request: &RenderRequest) -> bool {
        if matches!(self.device, Device::Absent(_)) {
            return false;
        }
        let placed = placed_of(request);
        let coverage =
            crate::lane::coverage_for(request.target.transform, crate::lane::CoverageChoice::Auto);
        let pages = std::slice::from_ref(&placed);
        if let Some((said, _)) = self.refusals.said(pages, coverage) {
            let said = said.to_owned();
            self.hand_to_the_processor(
                placed,
                &format!("the graphics device {}", first_line(&said)),
            );
            return false;
        }
        let (prediction, processor) = self.costs.forecast_for(pages, coverage, None);
        if processor {
            let chose = prediction.said();
            self.handed.traced.push(format!(
                "page {} processor chosen: {chose}",
                request.page.saturating_add(1)
            ));
            self.hand_to_the_processor(placed, &chose);
            return false;
        }
        self.title = None;
        true
    }

    /// Records a page handed to the drawing thread, and what the title says about it.
    fn hand_to_the_processor(&mut self, placed: Placed, why: &str) {
        let title = format!("drawn on the processor: {why}");
        if self.title.as_deref() != Some(title.as_str()) {
            self.handed.said.push(format!("note: the page was {title}"));
            self.title = Some(title);
        }
        self.priced.retain(|priced| !priced.is(&placed));
        self.priced.push(placed);
    }

    /// Sends the next queued page to the thread, where none is being drawn.
    fn dispatch(&mut self) {
        if self.in_flight.is_some() {
            return;
        }
        // A window drawn ahead goes first: it is what the reader is waiting for (ADR 2322).
        let first = self.queued.iter_mut().find_map(|queued| {
            queued
                .ahead
                .take()
                .map(|ahead| (queued.request.clone(), ahead))
        });
        let (request, window, coverage) = if let Some((request, (window, coverage))) = first {
            (request, Some(window), coverage)
        } else {
            let Some(queued) = self.queued.pop_front() else {
                return;
            };
            let coverage = crate::lane::coverage_for(
                queued.request.target.transform,
                crate::lane::CoverageChoice::Auto,
            );
            (queued.request, None, coverage)
        };
        if self.link.is_none() {
            self.bring_up();
        }
        let serial = self.serial;
        self.serial = self.serial.wrapping_add(1);
        let job = Job {
            serial,
            list: Arc::clone(&request.list),
            target: window.map_or(request.target, |window| window.target_of(request.target)),
            coverage,
        };
        let sent = self
            .link
            .as_ref()
            .is_some_and(|link| link.jobs.send(job).is_ok());
        if !sent {
            // A thread that has ended has no device to draw with: the page is the drawing
            // thread's, as every later one is.
            self.link = None;
            self.absent("the device's thread ended".to_owned());
            // A window's page is still queued, and `absent` handed it back.
            if window.is_none() {
                self.handed.back.push(request);
            }
            return;
        }
        self.in_flight = Some(Flight {
            serial,
            request,
            asked: Instant::now(),
            coverage,
            window,
            superseded: false,
        });
    }

    /// The window of `request`'s page to draw ahead of it, and its lane: the part the viewport
    /// shows, `shown`, where the view of the page moved since it was last offered and the device
    /// is up and has not refused that lane for the page (ADR 2322). Remembers the view either way.
    fn ahead_of(
        &mut self,
        request: &RenderRequest,
        shown: Option<Window>,
    ) -> Option<(Window, raster_gpu::Coverage)> {
        let same =
            |&(document, page, _): &(viewer_core::DocumentId, usize, pdf_render::Transform)| {
                (document, page) == (request.document, request.page)
            };
        let before = self.views.iter().position(same);
        let last = before
            .and_then(|at| self.views.remove(at))
            .map(|(_, _, transform)| transform);
        self.views
            .push_front((request.document, request.page, request.target.transform));
        self.views.truncate(VIEWS);
        let shown = shown?;
        let last =
            last.filter(|&last| !crate::lane::same_transform(last, request.target.transform))?;
        let Device::Up(adapter) = &self.device else {
            return None;
        };
        let coverage = crate::lane::lane_for(
            request.target.transform,
            Some(last),
            None,
            &[],
            crate::lane::software_adapter(adapter),
            crate::lane::CoverageChoice::Auto,
        );
        let placed = placed_of(request);
        self.refusals
            .said(std::slice::from_ref(&placed), coverage)
            .is_none()
            .then_some((shown, coverage))
    }

    /// No device: everything this lane holds goes to the drawing thread, and nothing is asked of
    /// it again.
    fn absent(&mut self, said: String) {
        self.handed.traced.push(format!(
            "no graphics device ({said}): the processor draws every page"
        ));
        self.device = Device::Absent(said);
        // A window's page is queued still, and goes back with the queue.
        if let Some(flight) = self
            .in_flight
            .take()
            .filter(|flight| flight.window.is_none())
        {
            self.handed.back.push(flight.request);
        }
        self.handed
            .back
            .extend(self.queued.drain(..).map(|queued| queued.request));
    }

    /// Takes in one thing the thread sent.
    fn land(&mut self, message: Message) {
        match message {
            Message::Up(adapter, took, steps) => {
                self.handed.traced.push(format!(
                    "graphics device up in {took:?}: {adapter}; {}",
                    steps.said(took)
                ));
                self.device = Device::Up(adapter);
            }
            Message::Absent(said) => self.absent(said),
            Message::NoDevice(serial) => {
                if let Some(flight) = self.in_flight.take_if(|flight| flight.serial == serial)
                    && flight.window.is_none()
                {
                    self.handed.back.push(flight.request);
                }
            }
            Message::Drawn {
                serial,
                outcome,
                took,
                phases,
            } => {
                let Some(flight) = self.in_flight.take_if(|flight| flight.serial == serial) else {
                    return;
                };
                if let Some(window) = flight.window {
                    self.window_drawn(flight, window, outcome, took, &phases);
                    return;
                }
                let placed = placed_of(&flight.request);
                let pages = std::slice::from_ref(&placed);
                match outcome {
                    Ok(raster) => {
                        self.costs.measure(
                            pages,
                            Lane::Device { scene_kept: false },
                            took,
                            flight.coverage,
                            None,
                        );
                        self.handed.traced.push(phases_of(
                            &format!("page {}'s", flight.request.page.saturating_add(1)),
                            &phases,
                        ));
                        self.handed.finished.push(Finished {
                            request: flight.request,
                            outcome: Some(Rendered::Raster(raster)),
                            // The union's exact measure is the processor's (ADR 2280); what the
                            // device will not draw is a refusal, said below, not a decline.
                            declined: Vec::new(),
                            cost: took,
                            waited: flight.asked.elapsed(),
                        });
                    }
                    Err((said, kept)) => {
                        if let Some(kept) = kept {
                            self.refusals.remember(pages, &said, kept);
                        }
                        self.costs.refused(pages);
                        // The adapter's words in full on the trace; the title takes the first line,
                        // which names what was refused (ADR 1962).
                        self.handed.traced.push(format!(
                            "page {} refused by the graphics device under {:?}, drawn on the \
                             processor: {said}",
                            flight.request.page.saturating_add(1),
                            flight.coverage
                        ));
                        self.hand_to_the_processor(
                            placed,
                            &format!("the graphics device {}", first_line(&said)),
                        );
                        self.handed.back.push(flight.request);
                    }
                }
            }
        }
    }

    /// A window drawn ahead of its page came back: handed to the window where it is still of the
    /// newest request for the page, and a refusal remembered, its page drawn as before (ADR 2322).
    /// Neither lane's price is measured by it: a window is not the page the forecast is for.
    fn window_drawn(
        &mut self,
        flight: Flight,
        window: Window,
        outcome: Result<Raster, (String, Option<Kept>)>,
        took: Duration,
        phases: &FrameCost,
    ) {
        let page = flight.request.page.saturating_add(1);
        let subject = format!(
            "page {page}'s window {}x{} at {},{}, its",
            window.width, window.height, window.x, window.y
        );
        match outcome {
            Ok(raster) => {
                self.handed.traced.push(phases_of(&subject, phases));
                if flight.superseded {
                    self.handed.traced.push(format!(
                        "page {page}'s window was asked for again while it was drawn, and is let go"
                    ));
                    return;
                }
                self.handed.windows.push((
                    Finished {
                        request: flight.request,
                        outcome: Some(Rendered::Raster(raster)),
                        declined: Vec::new(),
                        cost: took,
                        waited: flight.asked.elapsed(),
                    },
                    window,
                ));
            }
            Err((said, kept)) => {
                if let Some(kept) = kept {
                    let placed = placed_of(&flight.request);
                    self.refusals
                        .remember(std::slice::from_ref(&placed), &said, kept);
                }
                self.handed.traced.push(format!(
                    "page {page}'s window refused by the graphics device under {:?}, and the page \
                     is drawn as before: {said}",
                    flight.coverage
                ));
            }
        }
    }

    /// Takes in everything the thread has sent, without waiting.
    fn drain(&mut self) {
        let mut arrived = Vec::new();
        if let Some(link) = self.link.as_ref() {
            while let Ok(message) = link.done.try_recv() {
                arrived.push(message);
            }
        }
        for message in arrived {
            self.land(message);
        }
    }

    /// Waits for the page in flight for at most what is left of `budget` — the launch's, which
    /// fills with time actually spent waiting (`viewer_host::Drawing::settle`'s ledger).
    fn settle(&mut self, budget: Duration) {
        while self.in_flight.is_some() {
            let left = budget.saturating_sub(self.spent);
            let Some(link) = self.link.as_ref().filter(|_| !left.is_zero()) else {
                return;
            };
            let began = Instant::now();
            let arrived = link.done.recv_timeout(left);
            self.spent = self.spent.saturating_add(began.elapsed());
            match arrived {
                Ok(message) => self.land(message),
                Err(RecvTimeoutError::Timeout) => return,
                Err(RecvTimeoutError::Disconnected) => {
                    self.link = None;
                    self.absent("the device's thread ended".to_owned());
                    return;
                }
            }
        }
    }
}

impl PageLane for ReadBack {
    fn offer(&mut self, request: RenderRequest, shown: Option<Window>) -> Option<RenderRequest> {
        let ahead = self.ahead_of(&request, shown);
        if !self.choose(&request) {
            return Some(request);
        }
        let same = |queued: &RenderRequest| {
            queued.document == request.document && queued.page == request.page
        };
        // A newer request for a page replaces the one queued for it, which the core has stopped
        // holding the token of — `viewer_host::Drawing::ask`'s rule; the one in flight cannot be
        // taken back, because raster draws a page in one call (ADR 0725), and a window of it in
        // flight is let go when it lands.
        self.queued.retain(|queued| !same(&queued.request));
        if let Some(flight) = self
            .in_flight
            .as_mut()
            .filter(|flight| flight.window.is_some() && same(&flight.request))
        {
            flight.superseded = true;
        }
        self.queued.push_back(Queued { request, ahead });
        self.dispatch();
        None
    }

    fn collect(&mut self, settle: Option<Duration>) -> Handed {
        self.drain();
        if let Some(budget) = settle {
            self.settle(budget);
        }
        self.dispatch();
        std::mem::take(&mut self.handed)
    }

    fn processor_drew(&mut self, request: &RenderRequest, cost: Duration) {
        let placed = placed_of(request);
        let Some(at) = self.priced.iter().position(|priced| priced.is(&placed)) else {
            return;
        };
        self.priced.remove(at);
        self.costs.measure(
            std::slice::from_ref(&placed),
            Lane::Processor,
            cost,
            raster_gpu::Coverage::Cpu,
            None,
        );
    }

    fn interval(&self) -> Option<Duration> {
        let waiting = self.in_flight.is_some()
            || !self.queued.is_empty()
            || !self.handed.finished.is_empty()
            || !self.handed.windows.is_empty()
            || !self.handed.back.is_empty();
        waiting.then_some(POLL)
    }

    fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
}

impl Prediction {
    /// ADR 1831's sentence about a page given to the processor: each lane's prediction, and
    /// whether this window measured it.
    #[must_use]
    pub fn said(&self) -> String {
        let scene = if self.scene_kept {
            "drawing the scene it holds"
        } else {
            "building its scene"
        };
        format!(
            "{} commands predicted at {:.2} s on the processor ({}) against {:.2} s on the \
             graphics device ({scene}, {})",
            self.commands,
            self.processor.as_secs_f64(),
            self.processor_basis.said(),
            self.device.as_secs_f64(),
            self.device_basis.said(),
        )
    }
}

/// A request as the lane records price and refusals by: the page, its ink, and its target.
fn placed_of(request: &RenderRequest) -> Placed {
    Placed {
        of: Picture::new(request.document, request.page, request.ink),
        list: Arc::clone(&request.list),
        target: request.target,
        depicts: Depicts::Page,
    }
}

/// The first line of what the adapter said: `wgpu` writes a refused pipeline as a chain of
/// causes, one per line, and a title is one line (ADR 1962).
/// A page the device drew and read back, stage by stage, as `quorra`'s first frame is said.
///
/// **Why a page this small is slower on the device than on the processor is a question about
/// these stages**, and the lane's one figure could not answer it: a launch's first page pays a
/// cold device's first use, and every page pays the copy back off the device, neither of which a
/// processor's draw has (ADR 2124). `elsewhere` is the device call's time outside the four stages
/// raster names — the submit and the wait for the queue among it — and `outside both` is the
/// page's time outside the scene and the device call, which held the processor's passes over the
/// read-back page until the device put the medium under it in its own pass (ADR 2150).
fn phases_of(subject: &str, cost: &FrameCost) -> String {
    let ms = |duration: Duration| duration.as_secs_f64() * 1000.0;
    let elsewhere = cost.device.saturating_sub(
        cost.encode
            .saturating_add(cost.upload)
            .saturating_add(cost.execute)
            .saturating_add(cost.readback),
    );
    let outside = cost
        .total
        .saturating_sub(cost.scene.saturating_add(cost.device));
    format!(
        "{subject} phases on the graphics device, ms: scene {:.1}, device {:.1} = encode {:.1} + \
         transfer {:.1} + execute {:.1} + read back {:.1} + elsewhere {:.1}; outside both {:.1}, \
         of {:.1} in all",
        ms(cost.scene),
        ms(cost.device),
        ms(cost.encode),
        ms(cost.upload),
        ms(cost.execute),
        ms(cost.readback),
        ms(elsewhere),
        ms(outside),
        ms(cost.total),
    )
}

fn first_line(said: &str) -> &str {
    said.lines().next().unwrap_or(said)
}

/// The device's thread: make the device, then draw what is asked until the window goes.
fn draw_until_told_to_stop(
    jobs: &Receiver<Job>,
    done: &Sender<Message>,
    wake: Option<&crate::Wake>,
) {
    let send = |message: Message| {
        let sent = done.send(message).is_ok();
        if let Some(wake) = wake {
            wake.wake();
        }
        sent
    };
    let began = Instant::now();
    let mut device = match QuorraRasterizer::with_options(&render_raster::options()) {
        Ok(device) => device,
        Err(refused) => {
            if !send(Message::Absent(refused.to_string())) {
                return;
            }
            // Every page already asked is answered, so that none waits on a device there is not.
            while let Ok(job) = jobs.recv() {
                if !send(Message::NoDevice(job.serial)) {
                    return;
                }
            }
            return;
        }
    };
    let (launch, startup) = device.startup();
    if !send(Message::Up(
        device.adapter_description().to_owned(),
        began.elapsed(),
        Steps { launch, startup },
    )) {
        return;
    }
    while let Ok(job) = jobs.recv() {
        device.set_coverage(job.coverage);
        let began = Instant::now();
        let outcome = device
            .rasterize(&job.list, job.target)
            .map_err(|problem| (problem.to_string(), remembered(&problem, job.coverage)));
        let took = began.elapsed();
        // A send that fails is a window that has gone, and there is nobody left to draw for.
        if !send(Message::Drawn {
            serial: job.serial,
            outcome,
            took,
            phases: Box::new(device.last_frame()),
        }) {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use viewer_core::{Command, DocumentId, Event, RenderRequest, Viewer};
    use viewer_host::drawing::PageLane as _;

    use std::sync::mpsc::{Receiver, Sender, channel};

    use render_raster::FrameCost;

    use super::{Device, Job, Kept, Link, Message, ReadBack, Steps, Window};

    /// A request the core minted for page one of a document every checkout has, as
    /// `viewer_host::drawing`'s tests take theirs.
    fn a_real_request() -> RenderRequest {
        let path: PathBuf =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf");
        let bytes = std::fs::read(&path).expect("the document is committed beside this crate");
        let mut viewer = Viewer::new(600, 800, 1.0);
        let events: Vec<Event> = viewer
            .handle(Command::Open {
                id: DocumentId(1),
                bytes: bytes.into(),
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

    /// A lane whose device is said to be up, over channels the test holds the far ends of in place
    /// of a thread: what it is asked is sent to nobody, and what lands is handed to
    /// [`ReadBack::land`] by the test.
    fn up() -> (ReadBack, Receiver<Job>, Sender<Message>) {
        let (jobs, asked) = channel();
        let (answer, done) = channel();
        let mut lane = ReadBack::new(None);
        lane.device = Device::Up("a test's".to_owned());
        lane.link = Some(Link {
            jobs,
            done,
            thread: None,
        });
        (lane, asked, answer)
    }

    /// Takes the job the lane would have sent, as the thread would.
    fn in_flight(lane: &ReadBack) -> u64 {
        lane.in_flight.as_ref().expect("a page in flight").serial
    }

    /// The window's bring-up line names each step the launch gate's child names, in its order, and
    /// what of the whole none of them is — and `-` for the steps a host that did not make the
    /// instance has no figure for (ADR 2282).
    #[test]
    fn the_bring_up_line_names_each_step_and_the_remainder() {
        let millis = Duration::from_millis;
        let startup = raster_gpu::StartupTimings {
            instance_creation: None,
            surface_creation: Duration::ZERO,
            adapter_selection: millis(1),
            device_creation: millis(10),
            pipeline_compilation: None,
        };
        let launch = raster_gpu::LaunchSteps {
            render_node_wake: millis(2),
            render_nodes_opened: 1,
            instance_creation: millis(15),
            adapter_check: millis(5),
            every_backend: false,
        };
        let made = Steps {
            launch: Some(launch),
            startup,
        };
        assert_eq!(
            made.said(millis(40)),
            "render-node wake 2.0 + instance 15.0 + adapter check 5.0 + adapter 1.0 + \
             request_device 10.0 + the device's assembly and the host's 7.0 ms"
        );
        let borrowed = Steps {
            launch: None,
            startup,
        };
        assert_eq!(
            borrowed.said(millis(12)),
            "render-node wake - + instance - + adapter check - + adapter 1.0 + request_device \
             10.0 + the device's assembly and the host's 1.0 ms"
        );
    }

    #[test]
    fn with_no_device_every_page_is_handed_back_and_nothing_is_said() {
        let (mut lane, _asked, _answer) = up();
        let request = a_real_request();
        // A page asked for while the device is coming is the device's.
        lane.device = Device::Coming;
        assert!(lane.offer(request.clone(), None).is_none());
        lane.land(Message::Absent("no adapter".to_owned()));
        let handed = lane.collect(None);
        assert_eq!(
            handed.back.len(),
            1,
            "the page in flight goes to the drawing thread"
        );
        assert!(
            handed.said.is_empty(),
            "a machine with no device is told nothing new"
        );
        assert!(
            lane.offer(request, None).is_some(),
            "and every later page is the processor's"
        );
        assert_eq!(lane.title(), None);
        assert!(lane.interval().is_none());
    }

    #[test]
    fn a_refusal_sends_the_page_back_and_its_next_ask_goes_to_the_processor_at_once() {
        let (mut lane, _asked, _answer) = up();
        let request = a_real_request();
        assert!(lane.offer(request.clone(), None).is_none());
        let serial = in_flight(&lane);
        lane.land(Message::Drawn {
            serial,
            outcome: Err(("frame refused: too large".to_owned(), Some(Kept::Frame))),
            took: Duration::from_millis(3),
            phases: Box::default(),
        });
        let handed = lane.collect(None);
        assert_eq!(handed.back.len(), 1);
        assert_eq!(handed.said.len(), 1, "the refusal is said once");
        let title = lane.title().expect("the title says the processor drew");
        assert!(title.starts_with("drawn on the processor: the graphics device frame refused"));
        // The same page at the same magnification is not asked of the device again.
        assert!(lane.offer(request, None).is_some());
        assert!(
            lane.collect(None).said.is_empty(),
            "and the sentence is not said twice"
        );
    }

    #[test]
    fn a_page_the_device_drew_is_finished_and_priced() {
        let (mut lane, _asked, _answer) = up();
        let request = a_real_request();
        assert!(lane.offer(request.clone(), None).is_none());
        assert_eq!(lane.interval(), Some(viewer_host::drawing::POLL));
        let serial = in_flight(&lane);
        let raster = pdf_render::Raster {
            width: request.target.width,
            height: request.target.height,
            format: pdf_render::RasterFormat::Rgba8,
            data: vec![255; 4],
        };
        let phases = FrameCost {
            total: Duration::from_millis(2),
            device: Duration::from_millis(1),
            readback: Duration::from_micros(400),
            ..FrameCost::default()
        };
        lane.land(Message::Drawn {
            serial,
            outcome: Ok(raster),
            took: Duration::from_millis(2),
            phases: Box::new(phases),
        });
        let handed = lane.collect(None);
        assert_eq!(handed.finished.len(), 1);
        assert!(
            handed
                .traced
                .iter()
                .any(|line| line.starts_with("page 1's phases")
                    && line.contains(
                        "read back 0.4 + elsewhere 0.6; outside both 1.0, of 2.0 in all"
                    )),
            "the page's stages are on the frames trace: {:?}",
            handed.traced
        );
        assert!(handed.back.is_empty());
        assert!(
            lane.interval().is_none(),
            "a lane with nothing out wakes nobody"
        );
    }

    #[test]
    fn a_newer_request_for_a_queued_page_replaces_it() {
        let (mut lane, _asked, _answer) = up();
        let request = a_real_request();
        assert!(lane.offer(request.clone(), None).is_none());
        let other = RenderRequest {
            page: 1,
            ..request.clone()
        };
        assert!(lane.offer(other.clone(), None).is_none());
        assert!(lane.offer(other, None).is_none());
        assert_eq!(lane.queued.len(), 1, "page two once, the newest");
    }

    /// `request` at twice its magnification, as a notch asks for the page again.
    fn zoomed(request: &RenderRequest, by: f32) -> RenderRequest {
        let mut zoomed = request.clone();
        zoomed.target = pdf_render::TargetSpec {
            width: request.target.width.saturating_mul(2),
            height: request.target.height.saturating_mul(2),
            transform: request
                .target
                .transform
                .then(pdf_render::Transform::scale(by, by)),
        };
        zoomed
    }

    /// What lands for the job in flight: a raster of the job's own size.
    fn drawn(lane: &mut ReadBack, job: &Job) {
        lane.land(Message::Drawn {
            serial: job.serial,
            outcome: Ok(pdf_render::Raster {
                width: job.target.width,
                height: job.target.height,
                format: pdf_render::RasterFormat::Rgba8,
                data: vec![255; 4],
            }),
            took: Duration::from_millis(1),
            phases: Box::default(),
        });
    }

    /// ADR 2322: a page whose view moved has the part the viewport shows drawn first, on the lane
    /// `quorra` takes for a moved view, handed back as a window and no answer; the page follows on
    /// the launch rule's lane. A launch's page, with nothing drawn of it before, is drawn alone.
    #[test]
    fn a_moved_view_draws_the_shown_window_first_and_the_page_after() {
        let (mut lane, asked, _answer) = up();
        let request = a_real_request();
        let window = Window {
            x: 10,
            y: 20,
            width: 100,
            height: 50,
        };
        assert!(lane.offer(request.clone(), Some(window)).is_none());
        let page = asked.try_recv().expect("the page is sent");
        assert_eq!(page.target, request.target, "a launch draws its page alone");
        drawn(&mut lane, &page);
        assert_eq!(lane.collect(None).finished.len(), 1);

        let notch = zoomed(&request, 2.0);
        assert!(lane.offer(notch.clone(), Some(window)).is_none());
        let first = asked.try_recv().expect("the window is sent");
        assert_eq!((first.target.width, first.target.height), (100, 50));
        assert_eq!(first.target, window.target_of(notch.target));
        assert_eq!(first.coverage, raster_gpu::Coverage::Compute);
        drawn(&mut lane, &first);
        let handed = lane.collect(None);
        assert!(handed.finished.is_empty(), "a window answers nothing");
        let [(finished, landed)] = handed.windows.as_slice() else {
            panic!("one window: {:?}", handed.windows);
        };
        assert_eq!(*landed, window);
        assert_eq!(finished.request.token, notch.token);
        assert!(
            handed
                .traced
                .iter()
                .any(|line| line.starts_with("page 1's window 100x50 at 10,20, its phases")),
            "{:?}",
            handed.traced
        );
        let after = asked.try_recv().expect("the page follows its window");
        assert_eq!(after.target, notch.target);
        assert_eq!(after.coverage, raster_gpu::Coverage::Cpu);
        drawn(&mut lane, &after);
        assert_eq!(lane.collect(None).finished.len(), 1);
        assert!(lane.interval().is_none());
    }

    /// A window drawn while a newer request for its page was offered is let go when it lands, and
    /// the newer request's window is drawn next; a window refused hands nothing back, and its page
    /// is drawn as before.
    #[test]
    fn a_window_asked_for_again_is_let_go_and_a_refused_one_leaves_its_page() {
        let (mut lane, asked, _answer) = up();
        let request = a_real_request();
        let window = Window {
            x: 0,
            y: 0,
            width: 64,
            height: 32,
        };
        assert!(lane.offer(request.clone(), Some(window)).is_none());
        drawn(&mut lane, &asked.try_recv().expect("the page"));
        lane.collect(None);
        let notch = zoomed(&request, 2.0);
        assert!(lane.offer(notch.clone(), Some(window)).is_none());
        let stale = asked.try_recv().expect("the first notch's window");
        let again = zoomed(&request, 3.0);
        assert!(lane.offer(again.clone(), Some(window)).is_none());
        drawn(&mut lane, &stale);
        let handed = lane.collect(None);
        assert!(handed.windows.is_empty(), "{:?}", handed.windows);
        assert!(
            handed.traced.iter().any(|line| line.contains("let go")),
            "{:?}",
            handed.traced
        );
        let next = asked.try_recv().expect("the newer request's window");
        assert_eq!(next.target, window.target_of(again.target));
        lane.land(Message::Drawn {
            serial: next.serial,
            outcome: Err(("the adapter said no".to_owned(), None)),
            took: Duration::from_millis(1),
            phases: Box::default(),
        });
        let handed = lane.collect(None);
        assert!(handed.windows.is_empty() && handed.back.is_empty());
        assert!(
            handed.traced.iter().any(|line| line.contains("refused")),
            "{:?}",
            handed.traced
        );
        let page = asked.try_recv().expect("the page is drawn as before");
        assert_eq!(page.target, again.target);
    }
}
