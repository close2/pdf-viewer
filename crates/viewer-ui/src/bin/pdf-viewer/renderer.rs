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
//! # Four kinds of layer, in this order, every present
//!
//! 1. **the medium**, one opaque texel scaled over the window. A page moved under a new view
//!    reveals what it does not cover, and what belongs there is the window's background — never
//!    page white, which would assert that the page is blank there (ADR 0378).
//! 2. **the retained pages**, one textured quad each, where a stand-in is being drawn. Each is a
//!    whole page at [`crate::stale::PROXY_EDGE`] pixels along its longer side, under its own
//!    `proxy⁻¹ ∘ asked` — **one placement per page rather than one for the picture**, which is
//!    what lets this layer answer a zoom in a column and a page turn where the one below it
//!    cannot (ADR 0443). Absent on a frame that is not standing in, where they would be entirely
//!    hidden by the layer over them.
//! 3. **the pages**, drawn into one texture and put up under the single placement
//!    [`crate::stale`] computes. The identity where the frame on hand is of the view being asked
//!    for; `settled⁻¹ ∘ asked` where it is not. **One texture and one placement whatever Table
//!    29's arrangement is** — the pages of a column move together, so what carries one onto the
//!    view carries all of them, and [`crate::stale`] refuses outright where that is not true
//!    rather than moving a page to somewhere it is not (ADR 0442). Absent where that refusal
//!    fired and the layer under it is standing in alone.
//! 4. **the chrome**, at the identity, on transparency. It is drawn in window pixels and it does
//!    not move with the page, which is what keeps a sidebar still while a page is being zoomed.
//!
//! **Layer 3 is opaque and that is what makes the pair complementary.** `render-quorra` draws the
//! window's medium under the page into that texture, so wherever the base covers the window it
//! wins outright and the blurrier picture beneath it is invisible; wherever a new view has moved
//! the base off, the retained page shows through. Neither layer has to know about the other.
//!
//! An empty slice would be a legitimate present that cleared the window, which is why this module
//! never issues one: a window with nothing to show presents nothing at all.
//!
//! # What the render thread does when nothing is asked of it
//!
//! **The proxies are produced there, and only there** (`doc/todo/37`, ADR 0443). Nothing on the
//! launch path makes one: the thread does not exist until the first job, it draws that job first,
//! and it looks for a page with no picture only when [`Job`]'s channel is empty. One page per idle
//! turn, so a view change arriving mid-way waits for one low-resolution frame rather than for the
//! whole set.
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

use pdf_render::{DisplayList, TargetSpec, Transform};
use quorra_gpu::wgpu;
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
/// Everything in it is **owned**, because it crosses a thread: each page by the `Arc` whose address
/// is its identity for as long as something pins it (`render_quorra::PresentFrame::pages`), the
/// chrome by value because this host rebuilds it from its own state every frame anyway.
#[derive(Debug)]
struct Job {
    width: u32,
    height: u32,
    /// Every page of Table 29's arrangement and where each goes, empty for a window with no
    /// document page to show.
    ///
    /// **A list since ADR 0442**, because `OneColumn` puts several pages in one window. Owned by
    /// the `Arc`s for the reason the whole job is owned — it crosses a thread — and each page's
    /// placement travels with it because it is that page's and nothing here can recompute one.
    pages: Vec<(Arc<DisplayList>, TargetSpec)>,
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
    pages: Vec<(Arc<DisplayList>, TargetSpec)>,
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

/// What the render thread sends back: a window frame, or a page it drew while it was idle.
///
/// **One channel rather than two**, so that the order the thread produced them in is the order the
/// event thread reads them in — a proxy that arrived before a frame must not be adopted after it,
/// or the store would hold a page the frame has already replaced.
#[derive(Debug)]
enum Finished {
    /// A window frame, boxed because it is much the larger of the two and a channel's item is as
    /// wide as its widest variant.
    Frame(Box<Done>),
    /// A whole page at [`crate::stale::PROXY_EDGE`], drawn while nothing else was asked for.
    Page(crate::stale::Retained<Arc<wgpu::Texture>>),
}

/// The channels to a running render thread, and the thread itself.
#[derive(Debug)]
struct Link {
    jobs: Sender<Job>,
    done: Receiver<Finished>,
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
    /// The pages it draws and where each was placed — empty for a window showing no page.
    pub(crate) pages: Vec<(Arc<DisplayList>, TargetSpec)>,
}

/// What one adopted frame tells the host about itself.
///
/// Handed back by [`Window::collect`] rather than read off a field, because every one of these is
/// a fact about *that* frame: a caller that read them a tick later would be describing the frame
/// after it.
#[derive(Debug)]
pub(crate) struct Landed {
    /// The pages it drew and where, for [`crate::stale`] to record as the view now settled.
    pub(crate) pages: Vec<(Arc<DisplayList>, TargetSpec)>,
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
    ///
    /// `proxy_pages` is how many whole pages this window retains a low-resolution picture of —
    /// the host's `--proxy-pages`, zero for a run that asked for none.
    pub(crate) fn split(
        mut renderer: QuorraWindowRenderer,
        size: (u32, u32),
        proxy_pages: usize,
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
            proxies: crate::stale::Proxies::new(proxy_pages),
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
        let mut newest = None;
        let mut arrived = Vec::new();
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
                }
            }
        }
        for page in arrived {
            self.proxies.keep(page);
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
                pages: done.pages.clone(),
            })
            .map(|previous| previous.textures);
        Some(Landed {
            pages: done.pages,
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
        pages: Vec<(Arc<DisplayList>, TargetSpec)>,
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
            self.thread = Some(spawn(renderer, self.proxies.extent()));
        }
        let Some(link) = self.thread.as_ref() else {
            return;
        };
        self.serial = self.serial.saturating_add(1);
        let job = Job {
            width,
            height,
            pages,
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

    /// Where each retained page goes so that it depicts this arrangement — [`crate::stale`]'s
    /// answer, asked here because the store lives here.
    pub(crate) fn underlay(
        &self,
        pages: &[(Arc<DisplayList>, TargetSpec)],
    ) -> Vec<(usize, Transform)> {
        self.proxies.placements(pages)
    }

    /// How many whole pages this window is holding a picture of, for the trace.
    pub(crate) fn retained(&self) -> usize {
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
    pub(crate) fn present(
        &mut self,
        placement: Option<Transform>,
        under: &[(usize, Transform)],
    ) -> Result<bool, quorra_gpu::RenderError> {
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
        let mut layers = vec![quorra_gpu::Layer {
            texture: &self.medium,
            placement: affine(Transform::scale(width, height)),
            filter: quorra_scene::ImageFilter::Nearest,
        }];
        // The blurrier layer first, so that the sharp one is drawn over it wherever it has pixels.
        for (index, moved) in under {
            let Some(retained) = self.proxies.get(*index) else {
                continue;
            };
            layers.push(quorra_gpu::Layer {
                texture: &retained.pixels,
                placement: affine(*moved),
                filter: quorra_scene::ImageFilter::Linear,
            });
        }
        if let Some(placement) = placement {
            layers.push(quorra_gpu::Layer {
                texture: &shown.textures.page,
                placement: affine(placement),
                // **Smoothed on purpose, and it is the one place this host chooses how a
                // stand-in looks.** A blur is what an approximation should look like — nobody
                // mistakes it for the page — where squares of four device pixels look like a
                // rendering decision somebody made. At the identity the sampler lands on texel
                // centres and the two filters agree exactly, so this costs a frame of the real
                // page nothing (quorra's `present.wgsl`).
                filter: quorra_scene::ImageFilter::Linear,
            });
        }
        layers.push(quorra_gpu::Layer {
            texture: &shown.textures.chrome,
            // The chrome is drawn in window pixels and stays where it was drawn: a sidebar
            // does not move because a page is being zoomed.
            placement: quorra_scene::Affine::IDENTITY,
            filter: quorra_scene::ImageFilter::Nearest,
        });
        self.presenter.present(&layers)?;
        Ok(true)
    }

    /// What the last present cost, in quorra's own units, or `None` before there has been one.
    pub(crate) fn last_present(&self) -> Option<quorra_gpu::PresentCost> {
        self.presenter.last()
    }
}

/// Starts the render thread around a renderer, and hands back the channels to it.
fn spawn(renderer: QuorraWindowRenderer, proxy_pages: usize) -> Link {
    let (jobs, incoming) = channel::<Job>();
    let (outgoing, done) = channel::<Finished>();
    // A spawn this machine refuses is a machine with no thread to spare, and the window then
    // presents whatever it has and asks for frames nobody answers — which is what
    // `Link::jobs.send` failing already means, so there is one path rather than two.
    let thread = std::thread::Builder::new()
        .name("page renderer".to_owned())
        .spawn(move || draw_until_told_to_stop(renderer, proxy_pages, &incoming, &outgoing))
        .ok();
    Link { jobs, done, thread }
}

/// The render thread's whole life: draw what is asked for, send it back, and draw a whole page at
/// a fraction of the size while nothing else is asked of it.
///
/// Ends when the event thread drops its sender, which is when the window is closing.
///
/// **The order is the whole of `CLAUDE.md`'s startup rule here.** A job always wins: the channel
/// is asked first, a proxy is drawn only when it is empty, and one page is drawn per idle turn so
/// that a view change arriving mid-way waits for one low-resolution frame rather than for a set of
/// them. Nothing about this runs before the first job, because the thread does not exist until
/// there is one.
fn draw_until_told_to_stop(
    mut renderer: QuorraWindowRenderer,
    proxy_pages: usize,
    jobs: &Receiver<Job>,
    done: &Sender<Finished>,
) {
    use std::sync::mpsc::TryRecvError;

    let mut proxies: crate::stale::Proxies<Arc<wgpu::Texture>> =
        crate::stale::Proxies::new(proxy_pages);
    // What the last frame drew, which is the only set of pages this thread has display lists for.
    let mut showing: Vec<(Arc<DisplayList>, TargetSpec)> = Vec::new();
    // The chrome lane needs a target of its own even where there is no chrome, so one scratch
    // texture is kept for whatever size the proxies are and remade when a page of another shape
    // arrives. One allocation per page shape rather than one per proxy.
    let mut scratch: Option<(u32, u32, wgpu::Texture)> = None;
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
            let finished = draw(&mut renderer, job);
            // A send that fails is an event thread that has gone, and there is nobody left to draw
            // for. The loop would end at the next `recv` anyway; leaving now saves a frame nobody
            // would see.
            if done.send(Finished::Frame(Box::new(finished))).is_err() {
                return;
            }
            continue;
        }
        if let Some(page) = proxies.wanted(&showing)
            && let Some(retained) = draw_whole_page(&mut renderer, &page, &mut scratch)
        {
            // Kept here as well as sent, because this thread is the one that decides what to draw
            // next and a store that forgot would draw the same page for ever.
            proxies.keep(crate::stale::Retained {
                page: Arc::clone(&retained.page),
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

/// Draws one whole page into a raster of its own, for the layer under the base.
///
/// `None` where the page states no usable extent, or where the device refused it — **and it is
/// deliberately silent**, because this is a picture nobody asked for: a refusal costs a stand-in
/// that will not be as good and nothing else, and reporting it would be a note about work the
/// person did not request. It cannot spin: the caller falls through to a blocking `recv` when this
/// answers `None`, so a page the device will not draw is retried once per frame at most rather
/// than once per turn round the loop.
fn draw_whole_page(
    renderer: &mut QuorraWindowRenderer,
    page: &Arc<DisplayList>,
    scratch: &mut Option<(u32, u32, wgpu::Texture)>,
) -> Option<crate::stale::Retained<Arc<wgpu::Texture>>> {
    let target = crate::stale::proxy_target(page)?;
    let (width, height) = (target.width, target.height);
    // A whole page at a few hundred pixels is far below the magnification at which quorra's GPU
    // coverage lane is the cheaper one, whatever the window is showing (`crate::surface`), so the
    // lane is stated rather than inherited from whatever the last frame asked for. `draw` sets it
    // per job, so the next window frame is unaffected.
    renderer.set_coverage(quorra_gpu::Coverage::Cpu);
    let chrome = match scratch.take() {
        Some((held_width, held_height, texture))
            if (held_width, held_height) == (width, height) =>
        {
            texture
        }
        // The chrome lane needs a target even where there is no chrome to draw, and
        // `Target::Texture` wants one sized exactly to the frame — so a page of another shape
        // costs one texture and every page of one document costs one between them.
        _ => renderer.layer_texture("a retained page's unused chrome", width, height),
    };
    let texture = renderer.layer_texture("a retained page", width, height);
    let placed = [(page, target)];
    let drawn = renderer.render(
        PresentFrame {
            width,
            height,
            pages: &placed,
            raster: None,
            overlays: &[],
        },
        WindowTextures {
            page: &texture,
            chrome: &chrome,
        },
    );
    *scratch = Some((width, height, chrome));
    drawn.ok()?;
    Some(crate::stale::Retained {
        page: Arc::clone(page),
        target,
        pixels: Arc::new(texture),
    })
}

/// One job, drawn — on the device, or on the processor where the device refused.
fn draw(renderer: &mut QuorraWindowRenderer, job: Job) -> Done {
    let Job {
        width,
        height,
        pages,
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
    let placed: Vec<(&Arc<DisplayList>, TargetSpec)> =
        pages.iter().map(|(list, target)| (list, *target)).collect();
    let into = WindowTextures {
        page: &textures.page,
        chrome: &textures.chrome,
    };
    let frame = PresentFrame {
        width,
        height,
        pages: &placed,
        raster: None,
        overlays: &borrowed,
    };
    let mut fell_back = None;
    let mut refused = None;
    if let Err(problem) = renderer.render(frame, into) {
        // **One of the two jobs `CLAUDE.md` keeps the CPU backend for**: a page the device refuses
        // is a page this program can still show, more slowly, which is a cost a person can see
        // past where a page that never appears is not. The raster goes back through the device as
        // one image, because a window's pixels have one path and this is it — and it is the whole
        // arrangement's raster since ADR 0442, because a column the device refused is still a
        // column.
        fell_back = Some(problem.to_string());
        let borrowed_pages: Vec<(&DisplayList, TargetSpec)> = pages
            .iter()
            .map(|(list, target)| (list.as_ref(), *target))
            .collect();
        let drawn = if borrowed_pages.is_empty() {
            Err("there is no page to draw on the processor".to_owned())
        } else {
            viewer_ui::software::compose_pages(&borrowed_pages)
                .map_err(|problem| problem.to_string())
        };
        match drawn {
            Ok(raster) => {
                let second = renderer.render(
                    PresentFrame {
                        width,
                        height,
                        pages: &[],
                        raster: Some(&raster),
                        overlays: &borrowed,
                    },
                    into,
                );
                if let Err(second) = second {
                    refused = Some(format!("and the processor's page {second}"));
                }
            }
            Err(second) => refused = Some(format!("and {second}")),
        }
    }
    Done {
        textures,
        pages,
        cost: renderer.last_frame(),
        fell_back,
        refused,
        function_refusals: renderer.last_function_paints().refusals().to_vec(),
        pipelines: renderer.startup().pipeline_compilation,
    }
}
