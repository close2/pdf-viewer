//! The event thread's half: the surface, the one job in flight, and the frame on hand — what is
//! asked for, what is adopted when it lands, and what each present puts up.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use pdf_render::{DisplayList, Interrupt, Transform};
use raster_gpu::wgpu;
use render_raster::{FrameCost, QuorraWindowRenderer};

use super::lanes::{Costs, Lane, Prediction, Refusals};
use super::sharp::Sharpened;
use super::thread::{Idle, spawn};
use super::{Done, Finished, Job, Landed, Link, Pair, Partial, Shown, same_pages};

/// The frame the render thread is drawing, as the event thread remembers it.
///
/// **The pages and the interrupt are what is new** (ADR 1821), and they are the shape
/// `quorra::composer`'s `InFlight` has had since ADR 0657: the pages so that this window can ask
/// whether the frame being drawn is still worth finishing, and the interrupt so that it can say
/// no — which reaches the processor's half of a refused frame and nothing else, because the
/// device's half is one call raster cannot take back.
#[derive(Debug)]
struct Flight {
    asked: Instant,
    pages: Vec<crate::stale::Placed>,
    interrupt: Interrupt,
    on_the_processor: Arc<AtomicBool>,
    /// [`Landed::predicted`], kept here for the frame's landing rather than sent with the job,
    /// because nothing on the render thread reads it.
    predicted: Option<Prediction>,
}

/// This window's half of the arrangement: the surface, and a handle on the thread that draws.
///
/// Lives on the event thread and never leaves it. What it owns of raster is a
/// [`raster_gpu::Presenter`] — the surface, its swapchain and one pipeline — which is `Send` and
/// which no `&mut Device` stands in front of.
pub struct Window {
    presenter: raster_gpu::Presenter,
    /// One opaque texel of the window's background: the first of [`crate::renderer`]'s four
    /// kinds of layer.
    medium: wgpu::Texture,
    /// The renderer, until the first job moves it to a thread of its own.
    ///
    /// **`CLAUDE.md`'s startup rule as a field.** A thread spawned in `resumed` would put a
    /// scheduler decision in front of `graphics device`, which is a launch milestone; a thread
    /// spawned by the first job costs the launch path nothing and the first frame one spawn.
    idle: Option<QuorraWindowRenderer>,
    thread: Option<Link>,
    /// The newest finished frame, which is what every present draws.
    shown: Option<Shown>,
    /// The chrome the last accepted job was asked to draw over the page.
    ///
    /// **Kept because a frame is the page *and* the chrome, and the tick that decides whether to
    /// ask for a new one used to compare only the pages** (ADR 0526). The find bar, the panel, the
    /// notices card, the selection and §12.5.1's ring are all overlays, so a window whose page had
    /// not changed put up the frame it already had and none of them ever reached the screen. Found
    /// by pressing `?` under `Xvfb` and getting the flag set, the display list built, and nothing
    /// on the window.
    ///
    /// The clone is priced: `Overlays::owned` already builds these fresh every frame — "tens of
    /// commands against a page's tens of thousands" — so keeping a second copy of a window's chrome
    /// costs less than the comparison it enables saves in whole frames.
    chrome: Vec<DisplayList>,
    /// The whole pages this window is holding a low-resolution picture of.
    ///
    /// **A second copy of what the render thread holds, and it is a copy on purpose.** The thread
    /// is the authority — it decides which page to draw next and keeps its own store — and what
    /// crosses is a finished picture, so the two can differ only by whatever is in the channel.
    /// The cost of a difference is a proxy drawn twice or one turn late, and never a wrong
    /// picture; ADR 0385's defect was the opposite arrangement, where the pixels and the record of
    /// what they were of could part company.
    proxies: crate::stale::Proxies<Arc<wgpu::Texture>>,
    /// The pair the last adopted frame displaced, waiting to go back with the next job.
    spare: Option<Pair>,
    /// The settled view at twice the window's resolution, or `None` where none has landed
    /// for the frame on hand. Checked against [`Self::shown`] and [`Self::size`] at every
    /// present rather than invalidated eagerly — a mismatch simply is not drawn.
    sharp: Option<Sharpened>,
    /// What the last adopted sharp pass cost, until the trace takes it.
    sharp_news: Option<Duration>,
    /// What the last scene built while nobody waited cost, until the trace takes it (ADR 1898).
    scene_news: Option<FrameCost>,
    /// `--supersample`'s factor: 1 off, 2 on. Carried to the render thread with the spawn.
    supersample: u32,
    /// `--coverage`'s choice, for the idle work this thread's counterpart draws.
    coverage: crate::lane::CoverageChoice,
    /// What wakes the event loop when idle work lands while it is at rest. Without it a
    /// sharp picture would sit in the channel until the next input.
    waker: Option<crate::Wake>,
    /// When the frame now being drawn was asked for, or `None` when the thread is idle.
    ///
    /// **One job in flight at a time, deliberately.** A queue would fill at the tick rate while a
    /// frame of a heavy page takes thirteen of them, and every job in it would be answering a view
    /// the person had already left. When a frame lands and the view has moved, the next job is
    /// asked for then — with the view as it is by then.
    in_flight: Option<Flight>,
    /// The frames the device refused, which it is not asked for again (ADR 1821).
    refusals: Refusals,
    /// What each lane charged per command for the pages it drew last, which picks the lane of a
    /// frame the device would take seconds over (ADR 1831).
    costs: Costs,
    /// The frame in flight as far as the processor has drawn it, and whether it has arrived since
    /// the window last put it up (ADR 1831).
    partial: Option<Partial>,
    partial_news: bool,
    /// Page one as it was drawn while it was read, for the frame of it to be finished from
    /// (ADR 1855), and the last of its looks this window sent to be uploaded.
    ahead: Option<viewer_host::Ahead>,
    read_looked: u64,
    /// Raised by [`Self::ask`] to take back a whole-page picture the processor is drawing while
    /// the render thread was otherwise idle (ADR 1831): a job always wins.
    idle_interrupt: Arc<std::sync::Mutex<Option<Interrupt>>>,
    /// The arrangement the **person** stopped drawing, which is not asked for again until the
    /// view changes — `quorra::composer`'s field of the same name, for its reason: this window
    /// asks for itself once a tick for as long as the frame on hand is not of the view, so an
    /// abort without it would be answered by the next tick asking for the same frame again.
    declined: Vec<crate::stale::Placed>,
    /// How many jobs this window has asked for.
    serial: u64,
    /// What the adapter is, and what bringing it up cost — read before the device left this
    /// thread, because afterwards there is nothing here to ask.
    description: String,
    startup: raster_gpu::StartupTimings,
    /// What compiling the pipelines cost, once a finished frame has reported it.
    ///
    /// Kept here because the device is not on this thread to ask: [`Self::startup`] is a snapshot
    /// taken before the split and its `pipeline_compilation` is `None` for ever, which is exactly
    /// the launch line ADR 0227 exists to close.
    pipelines: Option<Duration>,
    /// The window's size in device pixels, as the presenter was last told it.
    size: (u32, u32),
    /// What the graphics device has said that no call returned — the other end of the handler
    /// `render-raster` installs, taken from on this thread once per present.
    uncaptured: Arc<render_raster::UncapturedErrors>,
}

/// A presenter whose surface has never been configured — and the only way to reach a [`Window`].
///
/// # The abort this type exists to make unrepresentable
///
/// `wgpu` 30.0.0 answers an acquire that finds an unconfigured surface in one of two ways, and
/// which one it chooses is decided by a single field. `CoreSurface::error_sink` is set **only on a
/// configure that succeeded** (`wgpu-30.0.0/src/backend/wgpu_core.rs:3979-3985`), and
/// `get_current_texture` reads it: with a sink it reports `CurrentSurfaceTexture::Validation`,
/// which raster turns into a typed refusal this host already handles; **without one it calls
/// `handle_error_fatal`, which panics** (`ibid.:4023-4037`) — and under `panic = "abort"` that is
/// a core dump. So the fatal branch is reachable on the first configure of a process and never
/// again.
///
/// And the first configure is exactly the one this program used to race. `wgpu::Surface::configure`
/// waits for the device to come idle, and wgpu's own documentation of it says what happens when
/// that wait meets a busy queue: "Submissions that happen _during_ the configure may cause the
/// internal wait-for-idle to fail, raising a validation error"
/// (`wgpu-30.0.0/src/api/surface.rs`). This viewer submits from [`Window::ask`]'s render thread and
/// configures from the event thread's present, microseconds apart, by design (ADR 0391) — so the
/// race is structural rather than unlucky, which is why the project owner met it once and not
/// twice.
///
/// # What the type says
///
/// [`Window::ask`] is what spawns the render thread, and therefore the only way anything on this
/// device can submit. A `Window` cannot be obtained except through [`Ungrounded::ground`], which
/// configures the surface *before* returning one. Nothing left in the program can put a
/// submission alongside the first configure, and no rule has to be kept for that to hold.
pub struct Ungrounded(Window);

impl std::fmt::Debug for Ungrounded {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_tuple("Ungrounded").field(&self.0).finish()
    }
}

impl Ungrounded {
    /// Puts the window's own background on the surface, which is what configures it.
    ///
    /// **Not a probe frame**, which `CLAUDE.md` forbids on the launch path outright, and the
    /// distinction is not a quibble: nothing here waits for warmth, compiles a pipeline "to be
    /// safe" or draws a page. It is the window's first honest picture — the medium a moved page
    /// reveals at its edge — put up at the one moment in the process when the device's queue is
    /// provably empty, and it is the *same* configure and acquire the first page's present would
    /// have paid a moment later. What it adds to the launch path is one textured quad; what it
    /// buys is that the configure cannot fail for the one reason that is fatal.
    ///
    /// # Errors
    ///
    /// The presenter's own refusal, or the device's words where it refused nothing — either way
    /// the sentence a caller reports. A device that cannot put one opaque texel on its window
    /// cannot show a page on it either, so this is a launch decision and not a lost frame: the
    /// caller draws on the processor instead (`CLAUDE.md`'s second job for the CPU backend).
    pub fn ground(mut self) -> Result<Window, String> {
        let (width, height) = self.0.size;
        #[expect(
            clippy::cast_precision_loss,
            reason = "window dimensions are far below f32's exact integer range"
        )]
        let extent = Transform::scale(width as f32, height as f32);
        let layers = [raster_gpu::Layer {
            texture: &self.0.medium,
            placement: affine(extent),
            filter: raster_scene::ImageFilter::Nearest,
        }];
        let refused = self.0.presenter.present(&layers).err();
        // Taken whether the present refused or not: `Surface::configure` returns `()`, so a
        // configure that failed under a present that then succeeded would be silent — and this is
        // the one call in the program that knows a failure here is not survivable.
        let said = self.0.uncaptured.take();
        match (refused, said) {
            (None, None) => Ok(self.0),
            (Some(problem), Some(said)) => {
                Err(format!("{problem}, and the device said: {}", said.last))
            }
            (Some(problem), None) => Err(problem.to_string()),
            (None, Some(said)) => Err(said.last),
        }
    }
}

impl std::fmt::Debug for Window {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Window")
            .field("presenter", &self.presenter)
            .field("drawing", &self.in_flight.is_some())
            .field("serial", &self.serial)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

/// raster's affine from this tree's transform — §8.3.3's six coefficients, in the same order.
fn affine(transform: Transform) -> raster_scene::Affine {
    raster_scene::Affine {
        a: transform.a,
        b: transform.b,
        c: transform.c,
        d: transform.d,
        e: transform.e,
        f: transform.f,
    }
}

impl Window {
    /// Splits a freshly built renderer into a presenter this thread keeps and a device a thread
    /// will take, or gives the renderer back where it has no surface to detach.
    ///
    /// The renderer is *not* moved to a thread here: see `Self::idle`. **Nor is a `Window`
    /// produced here** — what comes back is an [`Ungrounded`], and that type carries the whole
    /// argument for why.
    ///
    /// The renderer comes back boxed on the failing path rather than by value, because a
    /// `QuorraWindowRenderer` is thousands of bytes and a `Result` is as wide as its widest arm —
    /// so the arm that never happens would otherwise size the one that always does. Which is
    /// raster's own argument for boxing the presenter inside `ForeignPresenter`, one layer down.
    ///
    /// `proxy_pages` is how many whole pages this window retains a low-resolution picture of —
    /// the host's `--proxy-pages`, zero for a run that asked for none.
    ///
    /// # Errors
    ///
    /// The renderer itself, boxed, where it holds no surface to hand over.
    pub fn split(
        mut renderer: QuorraWindowRenderer,
        size: (u32, u32),
        proxy_pages: usize,
        supersample: u32,
        coverage: crate::lane::CoverageChoice,
        waker: Option<crate::Wake>,
    ) -> Result<Ungrounded, Box<QuorraWindowRenderer>> {
        let description = renderer.adapter_description().to_owned();
        let startup = renderer.startup();
        let medium = renderer.medium_texture();
        let uncaptured = renderer.uncaptured();
        let Some(mut presenter) = renderer.detach_presenter() else {
            return Err(Box::new(renderer));
        };
        // Before the first present and on every resize, which is what raster's presenter asks of
        // a host: a resize configures nothing and the swapchain follows at the next present, so
        // it is cheap to say whenever the window system speaks.
        presenter.resize(size.0, size.1);
        Ok(Ungrounded(Self {
            presenter,
            medium,
            idle: Some(renderer),
            thread: None,
            shown: None,
            chrome: Vec::new(),
            proxies: crate::stale::Proxies::new(proxy_pages),
            spare: None,
            sharp: None,
            sharp_news: None,
            scene_news: None,
            supersample,
            coverage,
            waker,
            in_flight: None,
            refusals: Refusals::default(),
            costs: Costs::default(),
            partial: None,
            partial_news: false,
            ahead: None,
            read_looked: 0,
            idle_interrupt: Arc::default(),
            declined: Vec::new(),
            serial: 0,
            description,
            startup,
            pipelines: None,
            size,
            uncaptured,
        }))
    }

    /// What the graphics device has said since this was last asked, or `None` — the ordinary
    /// answer — where it has said nothing.
    ///
    /// **Taken once per present and nowhere else**, so that there is one place a device's
    /// complaint becomes a decision rather than several that each see part of it. See
    /// [`render_raster::UncapturedErrors`] for why the channel exists at all.
    pub fn device_said(&self) -> Option<render_raster::Uncaptured> {
        self.uncaptured.take()
    }

    /// The adapter raster selected, for reports.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// What bringing the device up cost, in the parts raster measures it in.
    ///
    /// **Read before the device left this thread**, so its `pipeline_compilation` is whatever was
    /// true at bring-up — which is `None`, because nothing on the launch path waits for warmth.
    /// [`Self::pipelines`] is where the answer arrives.
    pub fn startup(&self) -> raster_gpu::StartupTimings {
        self.startup
    }

    /// What compiling the pipelines cost, once a finished frame has reported it.
    pub fn pipelines(&self) -> Option<Duration> {
        self.pipelines
    }

    /// Tells the presenter how big the window is now.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.size = (width, height);
        self.presenter.resize(width, height);
        // A sharp frame of another size would fail the present-time match anyway; dropping
        // it here just returns four windows' worth of texture sooner.
        if self
            .sharp
            .as_ref()
            .is_some_and(|sharp| (sharp.width, sharp.height) != (width, height))
        {
            self.sharp = None;
        }
    }

    /// Whether a frame is being drawn right now.
    pub fn drawing(&self) -> bool {
        self.in_flight.is_some()
    }

    /// Whether the frame in flight is being drawn on the processor and has been out for longer
    /// than `viewer_host::drawing::WARN` — `quorra::composer::Composer::overlong`'s question,
    /// asked of the one frame on this window that can be stopped.
    ///
    /// **A frame on the device is never overlong here, and that is not an oversight**: raster
    /// draws it in one call nothing can take back (ADR 0725), so a warning offering a key would
    /// offer one that does nothing. A frame the device refused is drawn by the same processor the
    /// `--cpu` window draws with, and that draw takes the interrupt (ADR 1821).
    pub fn overlong(&self) -> bool {
        self.in_flight.as_ref().is_some_and(|flight| {
            flight.on_the_processor.load(Ordering::Acquire)
                && !flight.interrupt.raised()
                && flight.asked.elapsed() >= viewer_host::drawing::WARN
        })
    }

    /// Takes the processor's draw back because the **person** asked —
    /// `quorra::composer::Composer::abandon`, on this window.
    ///
    /// Answers whether it took anything back: `false` where nothing is being drawn on the
    /// processor, which is also the race between the key and the frame's last command.
    pub fn abandon(&mut self) -> bool {
        let Some(flight) = self.in_flight.as_ref() else {
            return false;
        };
        if flight.interrupt.raised() || !flight.on_the_processor.load(Ordering::Acquire) {
            return false;
        }
        flight.interrupt.raise();
        self.declined.clone_from(&flight.pages);
        true
    }

    /// Whether this is the arrangement the person stopped drawing — see [`Self::declined`].
    pub fn declined(&self, pages: &[crate::stale::Placed]) -> bool {
        !self.declined.is_empty() && same_pages(&self.declined, pages)
    }

    /// Takes the processor's draw back where finishing it would buy a picture of a magnification
    /// the view has already left (ADR 1821).
    ///
    /// `quorra::composer::Composer::superseded` interrupts where the finished frame could not
    /// stand in for the view at all, and lets a zoom of one page run on, because there the frame
    /// in flight is an ordinary page's and what it costs is hundreds of milliseconds. Here it is
    /// the frame the device *refused*, and what a refusal states is that the page is beyond what a
    /// frame was budgeted to be: finishing it at a magnification the gesture has moved past puts
    /// the remainder of a processor draw in front of the frame the person will stop at. So it is
    /// interrupted at the first notch that leaves its magnification, and the window stands in
    /// with what it has until the magnification settles. A pan is not a reason: the frame in
    /// flight is of the magnification the view is at, and carries onto it.
    ///
    /// The raise is harmless to a frame the device accepts — the device's half takes no
    /// interrupt — and where the device refuses, the processor is not started at all. Answers
    /// whether it raised one, for the trace.
    pub fn superseded(&mut self, wanted: &[crate::stale::Placed]) -> bool {
        let Some(flight) = self.in_flight.as_ref() else {
            return false;
        };
        if flight.interrupt.raised() {
            return false;
        }
        let magnification = |pages: &[crate::stale::Placed]| {
            pages
                .first()
                .map(|placed| crate::lane::linear_bits_of(placed.target.transform))
        };
        let left = magnification(&flight.pages) != magnification(wanted)
            || !crate::stale::could_stand_in(&flight.pages, wanted);
        if left {
            flight.interrupt.raise();
        }
        left
    }

    /// How long the render asked for at an earlier tick has been out, or `None` where
    /// none is. Rule 5's observation is measured from this, against the period: a
    /// render out for two milliseconds has missed nothing yet, whatever is in flight.
    pub fn out_for(&self) -> Option<Duration> {
        self.in_flight.as_ref().map(|flight| flight.asked.elapsed())
    }

    /// What the window is currently able to draw, and what it is a picture of.
    pub fn shown(&self) -> Option<&Shown> {
        self.shown.as_ref()
    }

    /// Takes whatever the render thread has finished, and keeps the newest of it.
    ///
    /// **Newest rather than each in turn**: a frame that has been overtaken is a picture nobody
    /// will ever see, and presenting it before the one behind it would be a stutter of exactly one
    /// refresh. In practice there is at most one, because [`Self::ask`] keeps one job in flight.
    pub fn collect(&mut self) -> Option<Landed> {
        let mut newest = None;
        let mut arrived = Vec::new();
        let mut sharp = None;
        let mut scene = None;
        {
            let link = self.thread.as_ref()?;
            // A thread that has gone is a device that has gone, and the window keeps showing
            // whatever it last had. Nothing here can restart it, so nothing here distinguishes an
            // empty channel from a closed one.
            while let Ok(finished) = link.done.try_recv() {
                match finished {
                    Finished::Frame(done) => newest = Some(done),
                    // Every one of these is kept, unlike the frames: a proxy is a picture of a
                    // page rather than of a moment, so an older one is not superseded by a newer
                    // one of a *different* page.
                    Finished::Page(page) => arrived.push(page),
                    Finished::Sharp(sharpened) => sharp = Some(sharpened),
                    Finished::Partial(partial) => {
                        // Only the frame in flight's, and only while it is still wanted: a raise
                        // means the view has left it, so its pixels are of a view nobody asks for.
                        let current = self.in_flight.as_ref().is_some_and(|flight| {
                            !flight.interrupt.raised() && same_pages(&flight.pages, &partial.pages)
                        });
                        if current {
                            self.partial = Some(*partial);
                            self.partial_news = true;
                        }
                    }
                    // Only while no frame is in flight: the first frame's own looks, and the frame,
                    // are truer pictures than the page as it was read.
                    Finished::Read(read) => {
                        if self.in_flight.is_none() {
                            self.partial = Some(*read);
                            self.partial_news = true;
                        }
                    }
                    Finished::Scene(prepared) => scene = Some(prepared),
                    // Kept at once, before the frame behind it is priced or the next one asked:
                    // the render thread sent it before either (ADR 2000).
                    Finished::Lane { lane, said } => {
                        self.refusals.remember(&[], &said, super::Kept::Lane(lane));
                    }
                }
            }
        }
        for page in arrived {
            self.proxies.keep(page);
        }
        if let Some(sharpened) = sharp {
            self.sharp_news = Some(sharpened.took);
            self.sharp = Some(*sharpened);
        }
        // Read before the frame behind it, in the order the thread sent them: a frame drawn after
        // the build says what the device holds by then.
        if let Some(prepared) = scene {
            self.costs.scene_of(Some(&prepared.pages));
            self.scene_news = Some(prepared.cost);
        }
        let done = newest?;
        self.pipelines = self.pipelines.or(done.pipelines);
        let predicted = self.in_flight.take().and_then(|flight| flight.predicted);
        self.partial = None;
        self.partial_news = false;
        let drew = done.finished.saturating_duration_since(done.started);
        self.price(&done);
        if done.abandoned {
            // **Nothing of it reaches the window**: the processor was taken back before it drew,
            // so the pair holds whatever the device left in it, and it goes back to be drawn into
            // next. The frame on hand stays the newest true picture there is.
            self.spare = Some(done.textures);
            return Some(Landed {
                pages: done.pages,
                cost: done.cost,
                drew,
                fell_back: done.fell_back,
                device_tried: done.device_tried,
                known: done.known,
                processor_drew: done.processor_drew,
                chose: done.chose,
                predicted,
                finished_ahead: done.finished_ahead,
                abandoned: true,
                refused: done.refused,
                function_refusals: done.function_refusals,
            });
        }
        // A frame of other pages makes the sharp picture a bystander; the present-time
        // match would decline it, but holding four windows of texels for a picture nobody
        // will draw again is not worth the allocation it saves.
        if let Some(sharpened) = self.sharp.as_ref()
            && !same_pages(&sharpened.pages, &done.pages)
        {
            self.sharp = None;
        }
        // The pair being displaced goes back to the render thread with the next job; the one
        // displaced before it, if the host never asked again, is simply dropped.
        self.spare = self
            .shown
            .replace(Shown {
                textures: done.textures,
                pages: done.pages.clone(),
            })
            .map(|previous| previous.textures);
        Some(Landed {
            pages: done.pages,
            cost: done.cost,
            drew,
            fell_back: done.fell_back,
            device_tried: done.device_tried,
            known: done.known,
            processor_drew: done.processor_drew,
            chose: done.chose,
            predicted,
            finished_ahead: done.finished_ahead,
            abandoned: false,
            refused: done.refused,
            function_refusals: done.function_refusals,
        })
    }

    /// What a landed frame says about each lane's price for its pages, and about what the device
    /// holds of them (ADRs 1831, 1860, 1886) — everything [`Costs`] and [`Refusals`] learn from it.
    fn price(&mut self, done: &Done) {
        // What each lane charged for these pages, which the next frame of them is priced by. A
        // draw taken back is a sample of nothing (`adopt_composed`'s reason, ADR 0657).
        if !done.abandoned {
            if let Some(took) = done.device_drew {
                let scene_kept = done.cost.scene_kept;
                self.costs
                    .measure(&done.pages, Lane::Device { scene_kept }, took);
            }
            if let Some(took) = done.processor_drew
                && done.refused.is_none()
            {
                self.costs.measure(&done.pages, Lane::Processor, took);
            }
        }
        // A frame the processor drew that priced nothing is still the processor's frame of these
        // pages, which is what the device's one ask waits for (ADR 1886).
        let processor_drew = done.chose.is_some() || done.finished_ahead;
        if processor_drew && (done.abandoned || done.finished_ahead) {
            self.costs.drawn_by_the_processor(&done.pages);
        }
        // The device asked and not refused drew these pages and holds their scene; a refusal
        // leaves it none; a frame the processor drew instead leaves it what it had.
        if !processor_drew && !done.pages.is_empty() {
            match (&done.fell_back, done.device_tried) {
                (None, _) => self.costs.scene_of(Some(&done.pages)),
                (Some(_), Some(_)) => self.costs.scene_of(None),
                // Not asked: a refusal remembered, and the device holds what it held.
                (Some(_), None) => {}
            }
        }
        if let Some(kept) = done.refused_now
            && let Some(said) = done.fell_back.as_deref()
        {
            self.refusals.remember(&done.pages, said, kept);
            self.costs.refused(&done.pages);
        }
    }

    /// Asks for a frame, spawning the render thread if this is the first.
    ///
    /// Does nothing while one is already being drawn — see `Self::in_flight` for why a queue
    /// would be a queue of answers to questions nobody is still asking.
    pub fn ask(
        &mut self,
        pages: Vec<crate::stale::Placed>,
        overlays: Vec<DisplayList>,
        coverage: raster_gpu::Coverage,
        now: Instant,
    ) {
        if self.in_flight.is_some() {
            return;
        }
        let (width, height) = self.size;
        if width == 0 || height == 0 {
            return; // minimised: nothing to draw into
        }
        if self.started().is_none() {
            return;
        }
        let Some(link) = self.thread.as_ref() else {
            return;
        };
        self.serial = self.serial.saturating_add(1);
        // Recorded before the move, and only here: every path above this line declines the job, so
        // a window that remembered chrome it never asked for would stop asking for it.
        self.chrome.clone_from(&overlays);
        // A view the person did not stop is a view change, which is what lets the one they did
        // stop be drawn again when they come back to it.
        if !self.declined.is_empty() && !same_pages(&self.declined, &pages) {
            self.declined.clear();
        }
        let known_refusal = self
            .refusals
            .said(&pages, coverage)
            .map(|(said, of)| super::Known {
                said: said.to_owned(),
                of,
            });
        let (predicted, cheaper) = if known_refusal.is_none() && !pages.is_empty() {
            let (prediction, processor) = self.costs.forecast_for(&pages);
            (Some(prediction), processor.then_some(prediction))
        } else {
            (None, None)
        };
        // Page one drawn while it was read, where this is its frame at the target it was drawn at
        // (ADR 1855). The lane is the processor's for it: all of the page but its last part is
        // already drawn there, so the processor is the faster lane by every prediction, which is
        // the condition A393 puts on page one drawn there.
        let ahead = match (self.ahead.as_ref(), pages.as_slice()) {
            (Some(ahead), [placed]) => ahead.take_for(&placed.list, placed.target),
            _ => None,
        };
        // The device's scene of these pages, built while the processor's frame is on the window and
        // nobody waits, where the device would draw the next frame of them from it (ADR 1898).
        let build_ahead = if known_refusal.is_none() && (cheaper.is_some() || ahead.is_some()) {
            self.costs.build_ahead_for(&pages)
        } else {
            None
        };
        let interrupt = Interrupt::new();
        let on_the_processor = Arc::new(AtomicBool::new(
            known_refusal.is_some() || cheaper.is_some() || ahead.is_some(),
        ));
        let flight = Flight {
            asked: now,
            pages: pages.clone(),
            interrupt: interrupt.clone(),
            on_the_processor: Arc::clone(&on_the_processor),
            predicted,
        };
        let job = Job {
            width,
            height,
            pages,
            overlays,
            coverage,
            reuse: self.spare.take(),
            known_refusal,
            cheaper,
            interrupt,
            on_the_processor,
            ahead,
            look: None,
            build_ahead,
        };
        // A send that fails is a render thread that has ended, which is a device that has gone;
        // the frame is not in flight, so the window keeps presenting what it has and asks again.
        if link.jobs.send(job).is_ok() {
            self.in_flight = Some(flight);
            self.partial = None;
            self.partial_news = false;
        }
        // **A job always wins over a whole page the processor is drawing for the retained layer**,
        // and the raise comes after the send on purpose: the render thread leaves its flag under
        // the same lock and then looks for a job, so either the flag is here to be raised or the
        // job is there to be found (`retain`).
        if let Ok(mut idle) = self.idle_interrupt.lock()
            && let Some(idle) = idle.take()
        {
            idle.raise();
        }
    }

    /// Page one as it is drawn while it is read, and the frame of it to be finished from that
    /// drawing (ADR 1855) — handed over by the window when it takes its document in.
    pub fn draw_ahead(&mut self, ahead: viewer_host::Ahead) {
        self.ahead = Some(ahead);
    }

    /// Sends the newest look at page one being read to be uploaded, where it is newer than the
    /// last one sent — on the window's tick while the document is still opening. The upload
    /// comes back through [`Self::collect`] as the picture [`Self::present_as_built`] puts up.
    pub fn show_read(&mut self, ahead: &viewer_host::Ahead) {
        let Some(look) = ahead.look().filter(|look| look.serial != self.read_looked) else {
            return;
        };
        if (look.target.width, look.target.height) != self.size || self.in_flight.is_some() {
            return;
        }
        let (width, height) = self.size;
        let Some(link) = self.started() else {
            return;
        };
        let job = Job {
            width,
            height,
            pages: Vec::new(),
            overlays: Vec::new(),
            coverage: raster_gpu::Coverage::default(),
            reuse: None,
            known_refusal: None,
            cheaper: None,
            interrupt: Interrupt::new(),
            on_the_processor: Arc::new(AtomicBool::new(false)),
            ahead: None,
            look: Some(look.raster),
            build_ahead: None,
        };
        if link.jobs.send(job).is_ok() {
            self.read_looked = look.serial;
        }
    }

    /// The render thread, started by the first job that needs it — `None` where it ended and
    /// its device went with it, which nothing can restart.
    fn started(&mut self) -> Option<&Link> {
        if self.thread.is_none() {
            // The first job is what starts the thread, which is `CLAUDE.md`'s startup rule: a
            // spawn in `resumed` would put a scheduler decision in front of a launch milestone.
            let Some(renderer) = self.idle.take() else {
                return None; // the thread ended and its device went with it; nothing can restart it
            };
            self.thread = Some(spawn(
                renderer,
                Idle {
                    proxy_pages: self.proxies.extent(),
                    supersample: self.supersample,
                    coverage: self.coverage,
                    interrupt: Arc::clone(&self.idle_interrupt),
                },
                self.waker.clone(),
            ));
        }
        self.thread.as_ref()
    }

    /// Where each retained page goes so that it depicts this arrangement — [`crate::stale`]'s
    /// answer, asked here because the store lives here.
    pub fn underlay(&self, pages: &[crate::stale::Placed]) -> Vec<(usize, Transform)> {
        self.proxies.placements(pages)
    }

    /// The chrome the frame on hand was drawn with, or was asked to be drawn with.
    ///
    /// What `quorra::surface` compares against this frame's overlays, for the reason
    /// `Self::chrome` states: a rendering of the same pages with *different* chrome is a
    /// different picture, and skipping it left the find bar and the notices card unreachable.
    pub fn chrome_asked(&self) -> &[DisplayList] {
        &self.chrome
    }

    /// How many whole pages this window is holding a picture of, for the trace.
    pub fn retained(&self) -> usize {
        self.proxies.len()
    }

    /// Puts the frame on hand on the window, with the page under `placement`.
    ///
    /// `placement` maps the page texture's own texels to the window's pixels — the identity where
    /// the frame is of the view being asked for, and `settled⁻¹ ∘ asked` where it is not. `None`
    /// leaves the sharp layer out altogether, which is the picture drawn from the retained pages
    /// alone: a page turn, a `GoTo`, a resize, a zoom in a column.
    ///
    /// `under` is [`Self::underlay`]'s answer, empty on every frame that is not standing in.
    ///
    /// `Ok(false)` where there is nothing to show yet, which is every tick before the first frame
    /// has landed — and where the caller asked for neither layer, which is a present that would
    /// clear the window. Nothing is presented then, deliberately: a window that has not been drawn
    /// to yet is not the same thing as a window somebody emptied.
    ///
    /// # Errors
    ///
    /// Whatever the presenter refused. `SurfaceUnavailable` is the swapchain saying "try again"
    /// and is handled by the caller exactly as it was when the device owned the surface.
    pub fn present(
        &mut self,
        placement: Option<Transform>,
        under: &[(usize, Transform)],
    ) -> Result<bool, raster_gpu::RenderError> {
        let Some(shown) = self.shown.as_ref() else {
            return Ok(false);
        };
        if placement.is_none() && under.is_empty() {
            return Ok(false);
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "window dimensions are far below f32's exact integer range"
        )]
        let (width, height) = (self.size.0 as f32, self.size.1 as f32);
        // One texel over the whole window: what a moved page reveals at its edge and no retained
        // page covers.
        let mut layers = vec![raster_gpu::Layer {
            texture: &self.medium,
            placement: affine(Transform::scale(width, height)),
            filter: raster_scene::ImageFilter::Nearest,
        }];
        // The blurrier layer first, so that the sharp one is drawn over it wherever it has pixels.
        for (index, moved) in under {
            let Some(retained) = self.proxies.get(*index) else {
                continue;
            };
            layers.push(raster_gpu::Layer {
                texture: &retained.pixels,
                placement: affine(*moved),
                filter: raster_scene::ImageFilter::Linear,
            });
        }
        if let Some(placement) = placement {
            // The sharp picture substitutes for the base wherever it is still true of it:
            // twice the window's texels drawn at half scale, which through the presenter's
            // one bilinear tap is an exact 2×2 box average (ADR 0699) — the same pixels a
            // high-resolution rendering scaled down would show. Under a reprojection it
            // rides the same placement the base would have, composed after the halving.
            let sharp = self.sharp.as_ref().filter(|sharp| {
                (sharp.width, sharp.height) == self.size && same_pages(&sharp.pages, &shown.pages)
            });
            match sharp {
                Some(sharp) => layers.push(raster_gpu::Layer {
                    texture: &sharp.texture,
                    placement: affine(Transform::scale(0.5, 0.5).then(placement)),
                    filter: raster_scene::ImageFilter::Linear,
                }),
                None => layers.push(raster_gpu::Layer {
                    texture: &shown.textures.page,
                    placement: affine(placement),
                    // **Smoothed on purpose, and it is the one place this host chooses how a
                    // stand-in looks.** A blur is what an approximation should look like — nobody
                    // mistakes it for the page — where squares of four device pixels look like a
                    // rendering decision somebody made. At the identity the sampler lands on texel
                    // centres and the two filters agree exactly, so this costs a frame of the real
                    // page nothing (raster's `present.wgsl`).
                    filter: raster_scene::ImageFilter::Linear,
                }),
            }
        }
        layers.push(raster_gpu::Layer {
            texture: &shown.textures.chrome,
            // The chrome is drawn in window pixels and stays where it was drawn: a sidebar
            // does not move because a page is being zoomed.
            placement: raster_scene::Affine::IDENTITY,
            filter: raster_scene::ImageFilter::Nearest,
        });
        self.presenter.present(&layers)?;
        Ok(true)
    }

    /// How much of the frame in flight the processor has drawn, where it has been looked at —
    /// what the title's bar carries (ADR 1831).
    pub fn drawn_so_far(&self) -> Option<f64> {
        self.partial.as_ref().map(|partial| partial.fraction)
    }

    /// Whether the frame the processor is drawing should go up as it stands for `pages`: it is of
    /// exactly these pages, and the window holds no picture of any of them (ADR 1831).
    ///
    /// **Where the window has an older picture of the same page, that picture stands in instead**,
    /// reprojected as on every view change (`crate::stale`): a zoom of a page this window has
    /// drawn shows the page it had, moved, rather than a part of the page at its new size. That is
    /// the choice `doc/pdf.js`'s viewer makes (`web/base_pdf_page_view.js` shows a canvas as it is
    /// drawn only where no earlier one of the page exists), taken as evidence about the approach
    /// and not as a source; what it decides is which of two true pictures a person sees first.
    pub fn shows_as_built(&self, pages: &[crate::stale::Placed]) -> bool {
        let Some(partial) = self.partial.as_ref() else {
            return false;
        };
        same_pages(&partial.pages, pages)
            && self.shown.as_ref().is_none_or(|shown| {
                !shown
                    .pages
                    .iter()
                    .any(|held| pages.iter().any(|asked| asked.of == held.of))
            })
    }

    /// Puts the frame in flight up as the processor has drawn it so far, where a newer look has
    /// arrived since it last went up: `Ok(None)` where none has, and `Ok(Some(fraction))` with the
    /// share it holds where it went up (ADR 1831).
    ///
    /// # Errors
    ///
    /// Whatever the presenter refused, as [`Self::present`].
    pub fn present_as_built(&mut self) -> Result<Option<f64>, raster_gpu::RenderError> {
        let Some(partial) = self.partial.as_ref().filter(|_| self.partial_news) else {
            return Ok(None);
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "window dimensions are far below f32's exact integer range"
        )]
        let (width, height) = (self.size.0 as f32, self.size.1 as f32);
        if (partial.textures.width, partial.textures.height) != self.size {
            return Ok(None);
        }
        let layers = [
            raster_gpu::Layer {
                texture: &self.medium,
                placement: affine(Transform::scale(width, height)),
                filter: raster_scene::ImageFilter::Nearest,
            },
            raster_gpu::Layer {
                texture: &partial.textures.page,
                placement: raster_scene::Affine::IDENTITY,
                filter: raster_scene::ImageFilter::Nearest,
            },
            raster_gpu::Layer {
                texture: &partial.textures.chrome,
                placement: raster_scene::Affine::IDENTITY,
                filter: raster_scene::ImageFilter::Nearest,
            },
        ];
        let fraction = partial.fraction;
        self.presenter.present(&layers)?;
        self.partial_news = false;
        Ok(Some(fraction))
    }

    /// What the last present cost, in raster's own units, or `None` before there has been one.
    pub fn last_present(&self) -> Option<raster_gpu::PresentCost> {
        self.presenter.last()
    }

    /// What the sharp pass that just landed cost, once — for the trace, which is rule 3's
    /// half of ADR 0699: a picture that quietly got better is still a picture that changed.
    pub fn sharpened(&mut self) -> Option<Duration> {
        self.sharp_news.take()
    }

    /// What the scene the device built while nobody waited cost, once — for the trace: the next
    /// frame of these pages is priced as kept on the strength of it (ADR 1898).
    pub fn built_ahead(&mut self) -> Option<FrameCost> {
        self.scene_news.take()
    }
}
