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
//! raster's ADR 0056 answered the ask in `doc/QUORRA_NONBLOCKING_RENDER.md`: the surface leaves
//! the device as a `Send` `Presenter`, and `Presenter::present(&[Layer])` puts finished rasters on
//! the window under their own affines. This module is the arrangement that follows from it.
//!
//! # Which thread owns what
//!
//! | | the event thread | the render thread |
//! |---|---|---|
//! | holds | [`raster_gpu::Presenter`] — the surface, its swapchain, one pipeline | `QuorraWindowRenderer` — the device, the caches, the retained scenes |
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
//! **Layer 3 is opaque and that is what makes the pair complementary.** `render-raster` draws the
//! window's medium under the page into that texture, so wherever the base covers the window it
//! wins outright and the blurrier picture beneath it is invisible; wherever a new view has moved
//! the base off, the retained page shows through. Neither layer has to know about the other.
//!
//! An empty slice would be a legitimate present that cleared the window, which is why this module
//! never issues one: a window with nothing to show presents nothing at all.
//!
//! # What the render thread does when nothing is asked of it
//!
//! **The proxies are produced on a thread that draws, never on the one that presents**
//! (`doc/todo/37`, ADR 0443; [`crate::composer`] does the same for the window with no device since
//! ADR 0461, which is why this sentence no longer says "only here"). Nothing on the
//! launch path makes one: the thread does not exist until the first job, it draws that job first,
//! and it looks for a page with no picture only when [`Job`]'s channel is empty. One page per idle
//! turn, so a view change arriving mid-way waits for one low-resolution frame rather than for the
//! whole set.
//!
//! # What the startup rules bind here
//!
//! `CLAUDE.md` forbids a scheduler decision in front of page one, so **the thread is spawned by
//! the first job and not by `resumed`**. Nothing on the launch path joins it, waits for warmth or
//! blocks on a first frame: `detach_presenter` asks the pipeline store nothing (raster's own
//! documentation, verified against `Device::detach_presenter`'s four handle clones), and the first
//! present compiles the presenting pass inline if the warm-up thread has not reached it yet —
//! which `PresentCost::compiled` reports, exactly as any first frame of any lane does.
//!
//! # Where each part is
//!
//! | module | what it is |
//! |---|---|
//! | this one | what crosses between the threads: a [`Job`], a [`Done`], what is [`Finished`] |
//! | [`window`] | the event thread's half: the job in flight, the frame on hand, each present |
//! | [`lanes`] | which lane draws a frame: each lane's price per command, the device's refusals |
//! | [`thread`] | the render thread's loop, and the order its kinds of work win in |
//! | [`device`] | one frame drawn, the device asked first; and the scene built while nobody waits |
//! | [`processor`] | the processor's half of a frame, taken back or shown as it is built |
//! | [`retained`] | the stand-in's layer: a whole page drawn small while the thread is idle |
//! | [`sharp`] | the settled view at twice the window's resolution, and what it may cost |
//! | [`as_read`] | page one as it is drawn while it is read, uploaded for the window |

// `#[path]` for the reason the binary's root gives: a module file reached through `#[path]` is a
// directory owner, so its children resolve beside it rather than in a directory of its name.
#[path = "renderer/as_read.rs"]
mod as_read;
#[path = "renderer/device.rs"]
mod device;
#[cfg(test)]
#[path = "renderer/gesture.rs"]
mod gesture;
#[path = "renderer/lanes.rs"]
mod lanes;
#[path = "renderer/processor.rs"]
mod processor;
#[path = "renderer/retained.rs"]
mod retained;
#[path = "renderer/sharp.rs"]
mod sharp;
#[path = "renderer/thread.rs"]
mod thread;
#[path = "renderer/window.rs"]
mod window;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use pdf_render::{DisplayList, Interrupt};
use raster_gpu::wgpu;
use render_raster::{FrameCost, QuorraWindowRenderer};

pub(crate) use lanes::Prediction;
use sharp::Sharpened;
pub(crate) use window::Window;

/// The two window-sized textures one frame is drawn into, travelling as one thing.
///
/// **They travel rather than being reallocated**, which is what a pool is here: a window's page
/// texture at 1280×1600 is 8 192 000 bytes and so is its chrome, and creating two of those on
/// every frame would spend a texture allocation per refresh for no reason. The event thread hands
/// back whichever pair it has just stopped showing, with the next job.
#[derive(Debug)]
struct Pair {
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
/// is its identity for as long as something pins it (`render_raster::PresentFrame::pages`), the
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
    pages: Vec<crate::stale::Placed>,
    /// The chrome, in window pixels.
    overlays: Vec<DisplayList>,
    /// Which coverage lane this frame's magnification asks for (`crate::surface::coverage_for`).
    coverage: raster_gpu::Coverage,
    /// The pair the event thread has finished with, for this frame to draw into.
    reuse: Option<Pair>,
    /// What the device said when it last refused these pages at this magnification, where it has
    /// — and then the device is not asked again, and the processor draws at once ([`Refusals`]).
    ///
    /// [`Refusals`]: lanes::Refusals
    known_refusal: Option<String>,
    /// Where the window predicted the processor draws these pages faster than the device does —
    /// and then the device is not asked at all ([`Costs`], ADR 1831).
    ///
    /// [`Costs`]: lanes::Costs
    cheaper: Option<Prediction>,
    /// The flag the event thread raises to take the processor's half of this frame back.
    ///
    /// The device's half cannot be taken back — raster draws a frame in one call (ADR 0725) — so
    /// a raise that arrives while the device is still being asked is honoured where the device
    /// refuses: the processor is then not started at all.
    interrupt: Interrupt,
    /// Set by the render thread the moment the processor starts on this frame, and by the event
    /// thread at the ask where [`Self::known_refusal`] or [`Self::cheaper`] says it will — which
    /// is what lets the window warn about a long draw it can stop, and only about one it can.
    on_the_processor: Arc<AtomicBool>,
    /// Page one drawn on the processor while it was read, where this frame is of that page at the
    /// target it was drawn at (ADR 1855): the frame is finished from it rather than drawn.
    ahead: Option<render_cpu::Underway>,
    /// A look at page one as it is drawn while it is read, to upload and send back as the window's
    /// picture of the page being read — a job with no frame in it, asked for by
    /// [`Window::show_read`].
    look: Option<Arc<pdf_render::Raster>>,
    /// What building these pages' scene on the device is predicted to cost, where the processor
    /// draws this frame and the device would draw the next one of these pages from the scene it
    /// then holds ([`Costs::build_ahead`]): the render thread builds it once the view has been
    /// still that long (ADR 1898).
    ///
    /// [`Costs::build_ahead`]: lanes::Costs::build_ahead
    build_ahead: Option<Duration>,
}

/// One finished window frame, on its way back to the thread that will present it.
#[derive(Debug)]
struct Done {
    textures: Pair,
    pages: Vec<crate::stale::Placed>,
    cost: FrameCost,
    /// What the device refused, where the processor then drew the page instead — or was asked to
    /// and did not, where [`Self::abandoned`] says so.
    fell_back: Option<String>,
    /// Whether the refusal in [`Self::fell_back`] was the device's answer to *this* frame and is a
    /// fact about these pages at this magnification, which is what [`Refusals`] keeps. `false` for
    /// a refusal the job already carried and for one that says nothing about the page (a device
    /// lost, a pipeline missing), which is asked again next frame as it always was.
    ///
    /// [`Refusals`]: lanes::Refusals
    refused_now: bool,
    /// How long the device was asked before it refused — the part of a refused frame nothing on
    /// this side can interrupt, and the part [`Refusals`] exists to stop paying twice.
    ///
    /// [`Refusals`]: lanes::Refusals
    device_tried: Option<Duration>,
    /// How long the processor drew for, where it drew.
    processor_drew: Option<Duration>,
    /// How long the device drew for, where it drew and built its picture rather than replaying it
    /// — the figure [`Costs`] reads the device's price per command from.
    ///
    /// [`Costs`]: lanes::Costs
    device_drew: Option<Duration>,
    /// [`Job::cheaper`], carried back so that the window can say why the processor drew.
    chose: Option<Prediction>,
    /// Whether the frame was the page drawn while it was read, finished ([`Job::ahead`]) — the
    /// processor's, and a measurement of nothing: its cost is a part's, not the page's.
    finished_ahead: bool,
    /// Whether the event thread raised [`Job::interrupt`] and this thread honoured it: no pixels
    /// were drawn and [`Self::textures`] holds nothing worth showing.
    abandoned: bool,
    /// What refused when neither the device nor the processor could draw the page.
    refused: Option<String>,
    /// §8.7.4.5.2 programs the device declined, in raster's own words (ADR 0376).
    function_refusals: Vec<String>,
    /// What compiling the pipelines cost, once raster's own background thread has finished.
    pipelines: Option<Duration>,
    /// When the render thread took this frame up — the start of [`Landed::drew`]'s span.
    started: Instant,
    /// When the render thread finished this frame — the end of [`Landed::drew`]'s span.
    ///
    /// Taken on the thread that drew, not at collection: a finished frame sits in the
    /// channel until the next tick reads it, and measuring to *that* moment quantised
    /// every wait up to roughly one refresh — so rule 5's prediction sat just over the
    /// period on frames that took half of one, and every view change of a quick page
    /// was stood in for instead of waited for (found on the owner's 120 Hz display).
    finished: Instant,
}

/// What the render thread sends back: a window frame, or work it did while it was idle.
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
    /// The settled view at twice the window's resolution, drawn while nothing else was
    /// asked for.
    Sharp(Box<Sharpened>),
    /// The frame in flight as the processor has drawn it so far (ADR 1831).
    Partial(Box<Partial>),
    /// Page one as it stands while it is read, uploaded (ADR 1855).
    Read(Box<Partial>),
    /// The device's scene of the pages on hand, built while nothing was asked for (ADR 1898).
    Scene(Box<Prepared>),
}

/// The device's scene of an arrangement, built while nobody waited for it (ADR 1898).
#[derive(Debug)]
struct Prepared {
    /// The pages the device now holds the scene of.
    pages: Vec<crate::stale::Placed>,
    /// What the build cost, in `render-raster`'s own accounting.
    cost: FrameCost,
}

/// The frame the processor is drawing, as it stood at the last look — on its way to the window.
///
/// Uploaded on the render thread, which owns the device, into a pair of its own: the pair the
/// frame will land in is still being drawn for, and the pair on the window is the last true
/// picture, so neither may be written while this is shown.
#[derive(Debug)]
pub(crate) struct Partial {
    /// The frame's pages — the job it is a picture of.
    pages: Vec<crate::stale::Placed>,
    textures: Pair,
    /// How much of the arrangement's drawing the picture holds (`viewer_ui::software::Assembly`).
    pub(crate) fraction: f64,
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
    pub(crate) pages: Vec<crate::stale::Placed>,
}

/// What one adopted frame tells the host about itself.
///
/// Handed back by [`Window::collect`] rather than read off a field, because every one of these is
/// a fact about *that* frame: a caller that read them a tick later would be describing the frame
/// after it.
#[derive(Debug)]
pub(crate) struct Landed {
    /// The pages it drew and where, for [`crate::stale`] to record as the view now settled.
    pub(crate) pages: Vec<crate::stale::Placed>,
    /// What the whole frame cost on the render thread, in the parts raster measures it in.
    pub(crate) cost: FrameCost,
    /// How long the render thread spent drawing it, from taking the job up to finishing it —
    /// which is what rule 5 predicts the next render by.
    ///
    /// **Neither "until collected" nor "since the ask"** (ADR 1289). A finished frame sits in the
    /// channel until the next tick reads it ([`Done::finished`]), and a job asked for while the
    /// thread is drawing the sharp pass or a retained page waits behind that pass before it is
    /// begun — so a span from the ask charges a view change's render with a picture nobody asked
    /// for, and on the owner's 890M a page drawn in 5 ms predicts 9 ms against an 8.3 ms refresh
    /// and is stood in for. A render that is late because the thread is busy is still known to
    /// be late: rule 5's *observation* reads how long the ask has been out.
    pub(crate) drew: Duration,
    pub(crate) fell_back: Option<String>,
    /// How long the device was asked before it refused, where it was asked and refused — `None`
    /// where it accepted, and where it was not asked because it had refused this frame before.
    pub(crate) device_tried: Option<Duration>,
    /// How long the processor drew for, where it drew.
    pub(crate) processor_drew: Option<Duration>,
    /// Where the processor drew because it was predicted the faster lane, the prediction.
    pub(crate) chose: Option<Prediction>,
    /// What the window predicted the frame would cost on each lane, whichever it took — so a frame
    /// the device keeps says its lane and its figures as one the processor takes does (ADR 1886).
    pub(crate) predicted: Option<Prediction>,
    /// Whether the frame was page one drawn while it was read, finished (ADR 1855).
    pub(crate) finished_ahead: bool,
    /// Whether the processor's draw was taken back: no pixels landed, and nothing about this frame
    /// may be recorded as the view settled or answered to the core.
    pub(crate) abandoned: bool,
    pub(crate) refused: Option<String>,
    pub(crate) function_refusals: Vec<String>,
}

impl Done {
    /// Whether the processor drew this frame: the device refused it, the processor was predicted
    /// the faster lane, or it was page one drawn while it was read (ADRs 1821, 1831, 1855).
    fn on_the_processor(&self) -> bool {
        self.fell_back.is_some() || self.chose.is_some() || self.finished_ahead
    }
}

/// Whether two arrangements are the same pictures at the same placements — [`crate::stale::
/// Placed::is`]'s question over a whole list, which is what decides both whether a sharp
/// picture is still true and whether a job invalidates it.
fn same_pages(a: &[crate::stale::Placed], b: &[crate::stale::Placed]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.is(b))
}
