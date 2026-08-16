//! The device on a thread of its own, and the finished window frames that cross back.
//!
//! **This module exists because of one number**: over the project owner's own twenty-four-frame
//! run of `tmp/Entwurf.pdf`, `execute` — the graphics device's own timestamps — was 6.7 ms of
//! 4454.9. The device is idle for 99.85% of a frame of that page; what makes a frame long is a
//! processor walking fifty-eight thousand display commands on the calling thread. So while
//! `Device::render` held the only `&mut Device`, and the surface was inside it, **nothing could be
//! presented for the whole of that** — the clock [`crate::cadence`] built could decide when a
//! frame *started* and had no say in how long one lasted, and the owner's median interval between
//! presents was 167.4 ms against a 8.333 ms refresh.
//!
//! quorra's ADR 0056 answered the ask in `doc/QUORRA_NONBLOCKING_RENDER.md`: the surface leaves
//! the device as a `Send` `Presenter`, and `Presenter::present(&[Layer])` puts finished rasters on
//! the window under their own affines. This module is the arrangement that follows from it.
//!
//! # Which thread owns what
//!
//! | | the event thread | the render thread |
//! |---|---|---|
//! | holds | [`quorra_gpu::Presenter`] — the surface, its swapchain, one pipeline | `QuorraWindowRenderer` — the device, the caches, the retained scenes |
//! | does | acquires, draws three textured quads, presents | walks display lists, encodes, draws into textures |
//! | costs | one present, measured in tenths of a millisecond | a frame, measured in hundreds |
//!
//! **What crosses between them is a [`Job`] one way and a [`Done`] the other**, plus the texture
//! pair travelling back and forth so that no frame allocates a window's worth of pixels twice.
//! Nothing else: the render thread never sees a `Document`, a `Viewer` or an `App`, so
//! `interpret` stays exactly the pure function of the bytes and the view state that the oracle's
//! whole comparison rests on.
//!
//! # Three layers, in this order, every present
//!
//! 1. **the medium**, one opaque texel scaled over the window. A page moved under a new view
//!    reveals what it does not cover, and what belongs there is the window's background — never
//!    page white, which would assert that the page is blank there (ADR 0378).
//! 2. **the page**, under the placement [`crate::stale`] computes. The identity where the frame
//!    on hand is of the view being asked for; `settled⁻¹ ∘ asked` where it is not.
//! 3. **the chrome**, at the identity, on transparency. It is drawn in window pixels and it does
//!    not move with the page, which is what keeps a sidebar still while a page is being zoomed.
//!
//! An empty slice would be a legitimate present that cleared the window, which is why this module
//! never issues one: a window with nothing to show presents nothing at all.
//!
//! # What the startup rules bind here
//!
//! `CLAUDE.md` forbids a scheduler decision in front of page one, so **the thread is spawned by
//! the first job and not by `resumed`**. Nothing on the launch path joins it, waits for warmth or
//! blocks on a first frame: `detach_presenter` asks the pipeline store nothing (quorra's own
//! documentation, verified against `Device::detach_presenter`'s four handle clones), and the first
//! present compiles the presenting pass inline if the warm-up thread has not reached it yet —
//! which `PresentCost::compiled` reports, exactly as any first frame of any lane does.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use pdf_render::{DisplayList, Rasterizer as _, TargetSpec, Transform};
use quorra_gpu::wgpu;
use render_cpu::CpuRasterizer;
use render_quorra::{FrameCost, PresentFrame, QuorraWindowRenderer, WindowTextures};

/// The two window-sized textures one frame is drawn into, travelling as one thing.
///
/// **They travel rather than being reallocated**, which is what a pool is here: a window's page
/// texture at 1280×1600 is 8 192 000 bytes and so is its chrome, and creating two of those on
/// every frame would spend a texture allocation per refresh for no reason. The event thread hands
/// back whichever pair it has just stopped showing, with the next job.
#[derive(Debug)]
pub(crate) struct Pair {
    page: wgpu::Texture,
    chrome: wgpu::Texture,
    width: u32,
    height: u32,
}

impl Pair {
    /// A fresh pair for a window of this size.
    fn new(renderer: &QuorraWindowRenderer, width: u32, height: u32) -> Self {
        Self {
            page: renderer.layer_texture("the page", width, height),
            chrome: renderer.layer_texture("the chrome", width, height),
            width,
            height,
        }
    }

    /// Whether this pair can be drawn into for a window of that size.
    ///
    /// `Target::Texture` requires a target sized exactly to the viewport, so a pair from before a
    /// resize is dropped rather than reused — which is one allocation per resize and none per
    /// frame.
    fn fits(&self, width: u32, height: u32) -> bool {
        self.width == width && self.height == height
    }
}

/// One window frame the event thread asks the render thread to draw.
///
/// Everything in it is **owned**, because it crosses a thread: the page by the `Arc` whose address
/// is its identity for as long as something pins it (`render_quorra::PresentFrame::page`), the
/// chrome by value because this host rebuilds it from its own state every frame anyway.
#[derive(Debug)]
struct Job {
    width: u32,
    height: u32,
    /// The page and where it goes, or `None` for a window with no document page to show.
    page: Option<(Arc<DisplayList>, TargetSpec)>,
    /// The chrome, in window pixels.
    overlays: Vec<DisplayList>,
    /// Which coverage lane this frame's magnification asks for (`crate::surface::coverage_for`).
    coverage: quorra_gpu::Coverage,
    /// The pair the event thread has finished with, for this frame to draw into.
    reuse: Option<Pair>,
}

/// One finished window frame, on its way back to the thread that will present it.
#[derive(Debug)]
struct Done {
    textures: Pair,
    page: Option<(Arc<DisplayList>, TargetSpec)>,
    cost: FrameCost,
    /// What the device refused, where the processor then drew the page instead. A note the host
    /// prints once per occurrence, as it always has.
    fell_back: Option<String>,
    /// What refused when neither the device nor the processor could draw the page.
    refused: Option<String>,
    /// §8.7.4.5.2 programs the device declined, in quorra's own words (ADR 0376).
    function_refusals: Vec<String>,
    /// What compiling the pipelines cost, once quorra's own background thread has finished.
    pipelines: Option<Duration>,
}

/// The channels to a running render thread, and the thread itself.
#[derive(Debug)]
struct Link {
    jobs: Sender<Job>,
    done: Receiver<Done>,
    /// Joined on the way out, so that a device is shut down rather than abandoned.
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Link {
    /// Ends the thread by taking its work away, then waits for it.
    ///
    /// **Dropping the sender is the signal**, which is why there is no stop flag: `recv` on a
    /// channel with no senders returns an error, the loop ends, and the device is dropped on the
    /// thread that owns it. A join that panicked is ignored deliberately — this runs while the
    /// program is exiting, and a second panic there would replace whatever the first one said.
    fn drop(&mut self) {
        let (jobs, _) = channel();
        drop(std::mem::replace(&mut self.jobs, jobs));
        if let Some(thread) = self.thread.take() {
            drop(thread.join());
        }
    }
}

/// The newest finished frame, and what it is a picture of.
#[derive(Debug)]
pub(crate) struct Shown {
    textures: Pair,
    /// The page it draws and where that page was placed, or `None` for a window showing no page.
    pub(crate) page: Option<(Arc<DisplayList>, TargetSpec)>,
}

/// What one adopted frame tells the host about itself.
///
/// Handed back by [`Window::collect`] rather than read off a field, because every one of these is
/// a fact about *that* frame: a caller that read them a tick later would be describing the frame
/// after it.
#[derive(Debug)]
pub(crate) struct Landed {
    /// The page it drew and where, for [`crate::stale`] to record as the view now settled.
    pub(crate) page: Option<(Arc<DisplayList>, TargetSpec)>,
    /// What the whole frame cost on the render thread, in the parts quorra measures it in.
    pub(crate) cost: FrameCost,
    /// How long the event thread waited for it — which is what rule 5 predicts the next one by.
    pub(crate) waited: Duration,
    pub(crate) fell_back: Option<String>,
    pub(crate) refused: Option<String>,
    pub(crate) function_refusals: Vec<String>,
}

/// This window's half of the arrangement: the surface, and a handle on the thread that draws.
///
/// Lives on the event thread and never leaves it. What it owns of quorra is a
/// [`quorra_gpu::Presenter`] — the surface, its swapchain and one pipeline — which is `Send` and
/// which no `&mut Device` stands in front of.
pub(crate) struct Window {
    presenter: quorra_gpu::Presenter,
    /// One opaque texel of the window's background. See this module's third layer.
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
    /// The pair the last adopted frame displaced, waiting to go back with the next job.
    spare: Option<Pair>,
    /// When the frame now being drawn was asked for, or `None` when the thread is idle.
    ///
    /// **One job in flight at a time, deliberately.** A queue would fill at the tick rate while a
    /// frame of a heavy page takes thirteen of them, and every job in it would be answering a view
    /// the person had already left. When a frame lands and the view has moved, the next job is
    /// asked for then — with the view as it is by then.
    in_flight: Option<Instant>,
    /// How many jobs this window has asked for.
    serial: u64,
    /// What the adapter is, and what bringing it up cost — read before the device left this
    /// thread, because afterwards there is nothing here to ask.
    description: String,
    startup: quorra_gpu::StartupTimings,
    /// What compiling the pipelines cost, once a finished frame has reported it.
    ///
    /// Kept here because the device is not on this thread to ask: [`Self::startup`] is a snapshot
    /// taken before the split and its `pipeline_compilation` is `None` for ever, which is exactly
    /// the launch line ADR 0227 exists to close.
    pipelines: Option<Duration>,
    /// The window's size in device pixels, as the presenter was last told it.
    size: (u32, u32),
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

/// quorra's affine from this tree's transform — §8.3.3's six coefficients, in the same order.
fn affine(transform: Transform) -> quorra_scene::Affine {
    quorra_scene::Affine {
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
    /// The renderer is *not* moved to a thread here: see [`Self::idle`].
    ///
    /// The renderer comes back boxed on the failing path rather than by value, because a
    /// `QuorraWindowRenderer` is thousands of bytes and a `Result` is as wide as its widest arm —
    /// so the arm that never happens would otherwise size the one that always does. Which is
    /// quorra's own argument for boxing the presenter inside `ForeignPresenter`, one layer down.
    pub(crate) fn split(
        mut renderer: QuorraWindowRenderer,
        size: (u32, u32),
    ) -> Result<Self, Box<QuorraWindowRenderer>> {
        let description = renderer.adapter_description().to_owned();
        let startup = renderer.startup();
        let medium = renderer.medium_texture();
        let Some(mut presenter) = renderer.detach_presenter() else {
            return Err(Box::new(renderer));
        };
        // Before the first present and on every resize, which is what quorra's presenter asks of
        // a host: a resize configures nothing and the swapchain follows at the next present, so
        // it is cheap to say whenever the window system speaks.
        presenter.resize(size.0, size.1);
        Ok(Self {
            presenter,
            medium,
            idle: Some(renderer),
            thread: None,
            shown: None,
            spare: None,
            in_flight: None,
            serial: 0,
            description,
            startup,
            pipelines: None,
            size,
        })
    }

    /// The adapter quorra selected, for reports.
    pub(crate) fn description(&self) -> &str {
        &self.description
    }

    /// What bringing the device up cost, in the parts quorra measures it in.
    ///
    /// **Read before the device left this thread**, so its `pipeline_compilation` is whatever was
    /// true at bring-up — which is `None`, because nothing on the launch path waits for warmth.
    /// [`Self::pipelines`] is where the answer arrives.
    pub(crate) fn startup(&self) -> quorra_gpu::StartupTimings {
        self.startup
    }

    /// What compiling the pipelines cost, once a finished frame has reported it.
    pub(crate) fn pipelines(&self) -> Option<Duration> {
        self.pipelines
    }

    /// Tells the presenter how big the window is now.
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.size = (width, height);
        self.presenter.resize(width, height);
    }

    /// Whether a frame is being drawn right now.
    pub(crate) fn drawing(&self) -> bool {
        self.in_flight.is_some()
    }

    /// What the window is currently able to draw, and what it is a picture of.
    pub(crate) fn shown(&self) -> Option<&Shown> {
        self.shown.as_ref()
    }

    /// Takes whatever the render thread has finished, and keeps the newest of it.
    ///
    /// **Newest rather than each in turn**: a frame that has been overtaken is a picture nobody
    /// will ever see, and presenting it before the one behind it would be a stutter of exactly one
    /// refresh. In practice there is at most one, because [`Self::ask`] keeps one job in flight.
    pub(crate) fn collect(&mut self) -> Option<Landed> {
        let link = self.thread.as_ref()?;
        let mut newest = None;
        // A thread that has gone is a device that has gone, and the window keeps showing whatever
        // it last had. Nothing here can restart it, so nothing here distinguishes an empty channel
        // from a closed one.
        while let Ok(done) = link.done.try_recv() {
            newest = Some(done);
        }
        let done = newest?;
        self.pipelines = self.pipelines.or(done.pipelines);
        let waited = self
            .in_flight
            .take()
            .map_or(Duration::ZERO, |began| began.elapsed());
        // The pair being displaced goes back to the render thread with the next job; the one
        // displaced before it, if the host never asked again, is simply dropped.
        self.spare = self
            .shown
            .replace(Shown {
                textures: done.textures,
                page: done.page.clone(),
            })
            .map(|previous| previous.textures);
        Some(Landed {
            page: done.page,
            cost: done.cost,
            waited,
            fell_back: done.fell_back,
            refused: done.refused,
            function_refusals: done.function_refusals,
        })
    }

    /// Asks for a frame, spawning the render thread if this is the first.
    ///
    /// Does nothing while one is already being drawn — see [`Self::in_flight`] for why a queue
    /// would be a queue of answers to questions nobody is still asking.
    pub(crate) fn ask(
        &mut self,
        page: Option<(Arc<DisplayList>, TargetSpec)>,
        overlays: Vec<DisplayList>,
        coverage: quorra_gpu::Coverage,
        now: Instant,
    ) {
        if self.in_flight.is_some() {
            return;
        }
        let (width, height) = self.size;
        if width == 0 || height == 0 {
            return; // minimised: nothing to draw into
        }
        if self.thread.is_none() {
            // The first job is what starts the thread, which is `CLAUDE.md`'s startup rule: a
            // spawn in `resumed` would put a scheduler decision in front of a launch milestone.
            let Some(renderer) = self.idle.take() else {
                return; // the thread ended and its device went with it; nothing can restart it
            };
            self.thread = Some(spawn(renderer));
        }
        let Some(link) = self.thread.as_ref() else {
            return;
        };
        self.serial = self.serial.saturating_add(1);
        let job = Job {
            width,
            height,
            page,
            overlays,
            coverage,
            reuse: self.spare.take(),
        };
        // A send that fails is a render thread that has ended, which is a device that has gone;
        // the frame is not in flight, so the window keeps presenting what it has and asks again.
        if link.jobs.send(job).is_ok() {
            self.in_flight = Some(now);
        }
    }

    /// Puts the frame on hand on the window, with the page under `placement`.
    ///
    /// `placement` maps the page texture's own texels to the window's pixels — the identity where
    /// the frame is of the view being asked for, and `settled⁻¹ ∘ asked` where it is not.
    ///
    /// `Ok(false)` where there is nothing to show yet, which is every tick before the first frame
    /// has landed. Nothing is presented then, deliberately: a present with no layers would clear
    /// the window, and a window that has not been drawn to yet is not the same thing as a window
    /// somebody emptied.
    ///
    /// # Errors
    ///
    /// Whatever the presenter refused. `SurfaceUnavailable` is the swapchain saying "try again"
    /// and is handled by the caller exactly as it was when the device owned the surface.
    pub(crate) fn present(
        &mut self,
        placement: Transform,
    ) -> Result<bool, quorra_gpu::RenderError> {
        let Some(shown) = self.shown.as_ref() else {
            return Ok(false);
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "window dimensions are far below f32's exact integer range"
        )]
        let (width, height) = (self.size.0 as f32, self.size.1 as f32);
        let layers = [
            // One texel over the whole window: what a moved page reveals at its edge.
            quorra_gpu::Layer {
                texture: &self.medium,
                placement: affine(Transform::scale(width, height)),
                filter: quorra_scene::ImageFilter::Nearest,
            },
            quorra_gpu::Layer {
                texture: &shown.textures.page,
                placement: affine(placement),
                // **Smoothed on purpose, and it is the one place this host chooses how a
                // stand-in looks.** A blur is what an approximation should look like — nobody
                // mistakes it for the page — where squares of four device pixels look like a
                // rendering decision somebody made. At the identity the sampler lands on texel
                // centres and the two filters agree exactly, so this costs a frame of the real
                // page nothing (quorra's `present.wgsl`).
                filter: quorra_scene::ImageFilter::Linear,
            },
            quorra_gpu::Layer {
                texture: &shown.textures.chrome,
                // The chrome is drawn in window pixels and stays where it was drawn: a sidebar
                // does not move because a page is being zoomed.
                placement: quorra_scene::Affine::IDENTITY,
                filter: quorra_scene::ImageFilter::Nearest,
            },
        ];
        self.presenter.present(&layers)?;
        Ok(true)
    }

    /// What the last present cost, in quorra's own units, or `None` before there has been one.
    pub(crate) fn last_present(&self) -> Option<quorra_gpu::PresentCost> {
        self.presenter.last()
    }
}

/// Starts the render thread around a renderer, and hands back the channels to it.
fn spawn(renderer: QuorraWindowRenderer) -> Link {
    let (jobs, incoming) = channel::<Job>();
    let (outgoing, done) = channel::<Done>();
    // A spawn this machine refuses is a machine with no thread to spare, and the window then
    // presents whatever it has and asks for frames nobody answers — which is what
    // `Link::jobs.send` failing already means, so there is one path rather than two.
    let thread = std::thread::Builder::new()
        .name("page renderer".to_owned())
        .spawn(move || draw_until_told_to_stop(renderer, &incoming, &outgoing))
        .ok();
    Link { jobs, done, thread }
}

/// The render thread's whole life: draw what is asked for, send it back, wait for the next.
///
/// Ends when the event thread drops its sender, which is when the window is closing.
fn draw_until_told_to_stop(
    mut renderer: QuorraWindowRenderer,
    jobs: &Receiver<Job>,
    done: &Sender<Done>,
) {
    while let Ok(job) = jobs.recv() {
        let finished = draw(&mut renderer, job);
        // A send that fails is an event thread that has gone, and there is nobody left to draw
        // for. The loop would end at the next `recv` anyway; leaving now saves a frame nobody
        // would see.
        if done.send(finished).is_err() {
            return;
        }
    }
}

/// One job, drawn — on the device, or on the processor where the device refused.
fn draw(renderer: &mut QuorraWindowRenderer, job: Job) -> Done {
    let Job {
        width,
        height,
        page,
        overlays,
        coverage,
        reuse,
    } = job;
    let textures = reuse
        .filter(|pair| pair.fits(width, height))
        .unwrap_or_else(|| Pair::new(renderer, width, height));
    // Which lane draws this frame's coverage, decided from this frame's magnification by the
    // thread that knows it (`crate::surface::coverage_for`) and carried here.
    renderer.set_coverage(coverage);
    let borrowed: Vec<&DisplayList> = overlays.iter().collect();
    let into = WindowTextures {
        page: &textures.page,
        chrome: &textures.chrome,
    };
    let frame = PresentFrame {
        width,
        height,
        page: page.as_ref().map(|(list, target)| (list, *target)),
        raster: None,
        overlays: &borrowed,
    };
    let mut fell_back = None;
    let mut refused = None;
    if let Err(problem) = renderer.render(frame, into) {
        // **One of the two jobs `CLAUDE.md` keeps the CPU backend for**: a page the device refuses
        // is a page this program can still show, more slowly, which is a cost a person can see
        // past where a page that never appears is not. The raster goes back through the device as
        // one image, because a window's pixels have one path and this is it.
        fell_back = Some(problem.to_string());
        match page
            .as_ref()
            .map(|(list, target)| CpuRasterizer::new().rasterize(list, *target))
        {
            Some(Ok(raster)) => {
                let second = renderer.render(
                    PresentFrame {
                        width,
                        height,
                        page: None,
                        raster: Some(&raster),
                        overlays: &borrowed,
                    },
                    into,
                );
                if let Err(second) = second {
                    refused = Some(format!("and the processor's page {second}"));
                }
            }
            Some(Err(second)) => refused = Some(format!("and the processor {second}")),
            None => refused = Some("and there is no page to draw on the processor".to_owned()),
        }
    }
    Done {
        textures,
        page,
        cost: renderer.last_frame(),
        fell_back,
        refused,
        function_refusals: renderer.last_function_paints().refusals().to_vec(),
        pipelines: renderer.startup().pipeline_compilation,
    }
}
