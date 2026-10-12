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

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use pdf_render::{DisplayList, Interrupt, TargetSpec, Transform};
use raster_gpu::wgpu;
use render_raster::{
    FrameCost, PresentFrame, QuorraRasterError, QuorraWindowRenderer, WindowTextures,
};

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
    known_refusal: Option<String>,
    /// Where the window predicted the processor draws these pages faster than the device does —
    /// and then the device is not asked at all ([`Costs`], ADR 1831).
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
}

/// What the window predicted a frame would cost on each lane, where it predicted the processor
/// the cheaper (ADR 1831).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Prediction {
    /// How many commands the arrangement's pages hold between them — the figure both lanes'
    /// costs are read per.
    pub(crate) commands: usize,
    /// What the device was predicted to take, and whether from this window's own measurement of
    /// these pages or from [`Costs::DEVICE_SEED`].
    pub(crate) device: Duration,
    pub(crate) device_measured: bool,
    /// What the processor was predicted to take, likewise.
    pub(crate) processor: Duration,
    pub(crate) processor_measured: bool,
}

/// What each lane cost per command the last time it drew a frame of these pages (ADR 1831).
///
/// **Why a cost per command, and why per page.** ADR 1823 measured where the device's frame goes on
/// the project owner's drawing: host encode, which walks every command — 7 884 ms for 4 408 405 of
/// them, nearly two microseconds each, whatever their size. The processor's cost on a page of
/// hairlines is the same walk at a different price per command, divided among the strips the page
/// grants; on a page of large fills it is the pixels instead. So the count of commands is the one
/// figure both lanes scale with that a frame states before it is drawn, and what each lane charges
/// for one of *this page's* commands is read off the last frame of the page that lane drew. Until a
/// lane has drawn the page, its price is a seed measured on this machine (the constants say where).
///
/// **When it is asked at all.** A frame the device is predicted to draw inside
/// `viewer_host::drawing::WARN` stays on the device whatever the processor's prediction: that is a
/// frame nobody waits for, and the device is where `CLAUDE.md` puts a page. Past it, the lane with
/// the smaller prediction draws — a person waiting seconds is owed the faster of the two, and the
/// processor's draw is the one that can be shown as it is built, taken back and stopped.
#[derive(Debug, Default)]
struct Costs {
    /// Newest first, at most [`Self::KEPT`].
    held: Vec<Measured>,
}

/// One arrangement's last measured price per command on each lane, in nanoseconds.
#[derive(Debug)]
struct Measured {
    pictures: Vec<crate::stale::Picture>,
    device: Option<f64>,
    processor: Option<f64>,
    /// Whether the device has been asked these pages at all — it drew them, or refused them at
    /// some magnification — which is what [`Costs::predict`]'s one ask of an unmeasured device
    /// waits for.
    device_asked: bool,
}

/// Which lane drew a frame, for [`Costs::measure`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lane {
    Device,
    Processor,
}

impl Costs {
    /// How many arrangements' prices are kept — [`Refusals::KEPT`]'s count, for its reason.
    const KEPT: usize = 32;

    /// What one command costs the device before it has drawn a page here: the owner's drawing's
    /// fit frame on the Radeon 890M's `Cpu` lane, headless, the quicker of two at 1 113.7 ms over
    /// 4 408 406 commands (`render-raster`'s `zoom_frame`, ADR 1860 section 3).
    const DEVICE_SEED: f64 = 253.0;

    /// What one command costs the processor before it has drawn a page here: the same drawing's
    /// first frame in this window on the owner's display, on the global pool, the quickest of four
    /// at 530.0 ms over 4 408 405 commands (ADR 1860 section 3).
    const PROCESSOR_SEED: f64 = 120.0;

    /// The pictures an arrangement is priced by.
    fn key(pages: &[crate::stale::Placed]) -> Vec<crate::stale::Picture> {
        pages.iter().map(|placed| placed.of).collect()
    }

    /// The prediction for `pages`, where the processor is the lane to take; `None` where the
    /// device is.
    fn processor_for(&self, pages: &[crate::stale::Placed]) -> Option<Prediction> {
        let commands: usize = pages
            .iter()
            .map(|placed| placed.list.commands().len())
            .sum();
        self.predict(&Self::key(pages), commands)
    }

    /// [`Self::processor_for`] over the pictures and the count it reads.
    fn predict(&self, pictures: &[crate::stale::Picture], commands: usize) -> Option<Prediction> {
        let held = self.held.iter().find(|held| held.pictures == *pictures);
        let device = held.and_then(|held| held.device);
        let processor = held.and_then(|held| held.processor);
        let prediction = Prediction {
            commands,
            device: Self::cost(device.unwrap_or(Self::DEVICE_SEED), commands),
            device_measured: device.is_some(),
            processor: Self::cost(processor.unwrap_or(Self::PROCESSOR_SEED), commands),
            processor_measured: processor.is_some(),
        };
        // **The device is asked these pages once before the processor keeps them.** The
        // processor's price is replaced by what it measured the moment it draws a frame, and the
        // device's would be only if it drew one: so a page the seeds gave the processor would keep
        // the device's seed for as long as the window is open, and a device faster than its seed
        // would never be found (ADR 1860). The processor having drawn them is what says the
        // device is the contender here, and the frame it is asked is the one it is measured on.
        //
        // A refusal is an answer too: a device that refused these pages at one magnification is
        // not asked again at the next on the strength of its seed, or every notch of a zoom on a
        // device that refuses would pay the attempt (ADR 1831 section 1 measured 2.2 s of it).
        let device_unasked =
            !held.is_some_and(|held| held.device_asked) && prediction.processor_measured;
        (!device_unasked
            && prediction.device >= viewer_host::drawing::WARN
            && prediction.processor < prediction.device)
            .then_some(prediction)
    }

    /// `commands` at `nanoseconds` each.
    fn cost(nanoseconds: f64, commands: usize) -> Duration {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a count of commands priced for a prediction, far inside f64's exact range"
        )]
        let total = nanoseconds * commands as f64 / 1e9;
        Duration::try_from_secs_f64(total).unwrap_or(Duration::MAX)
    }

    /// Keeps what `lane` took to draw `pages`, newest first.
    fn measure(&mut self, pages: &[crate::stale::Placed], lane: Lane, took: Duration) {
        let commands: usize = pages
            .iter()
            .map(|placed| placed.list.commands().len())
            .sum();
        self.measured(Self::key(pages), commands, lane, took);
    }

    /// [`Self::measure`] over the pictures and the count it reads.
    fn measured(
        &mut self,
        pictures: Vec<crate::stale::Picture>,
        commands: usize,
        lane: Lane,
        took: Duration,
    ) {
        if commands == 0 {
            return;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "a count of commands priced for a prediction, far inside f64's exact range"
        )]
        let each = took.as_secs_f64() * 1e9 / commands as f64;
        let mut measured = self.taken(pictures);
        match lane {
            Lane::Device => {
                measured.device = Some(each);
                measured.device_asked = true;
            }
            Lane::Processor => measured.processor = Some(each),
        }
        self.held.insert(0, measured);
        self.held.truncate(Self::KEPT);
    }

    /// Keeps that the device refused `pages`, which prices nothing but is the device's answer for
    /// [`Self::predict`]'s one ask.
    fn refused(&mut self, pages: &[crate::stale::Placed]) {
        self.refused_pictures(Self::key(pages));
    }

    /// [`Self::refused`] over the pictures it reads.
    fn refused_pictures(&mut self, pictures: Vec<crate::stale::Picture>) {
        let mut measured = self.taken(pictures);
        measured.device_asked = true;
        self.held.insert(0, measured);
        self.held.truncate(Self::KEPT);
    }

    /// The entry for `pictures`, taken out to be put back newest, or a new one where none is held.
    fn taken(&mut self, pictures: Vec<crate::stale::Picture>) -> Measured {
        match self.held.iter().position(|held| held.pictures == pictures) {
            Some(at) => self.held.remove(at),
            None => Measured {
                pictures,
                device: None,
                processor: None,
                device_asked: false,
            },
        }
    }
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
    refused_now: bool,
    /// How long the device was asked before it refused — the part of a refused frame nothing on
    /// this side can interrupt, and the part [`Refusals`] exists to stop paying twice.
    device_tried: Option<Duration>,
    /// How long the processor drew for, where it drew.
    processor_drew: Option<Duration>,
    /// How long the device drew for, where it drew and built its picture rather than replaying it
    /// — the figure [`Costs`] reads the device's price per command from.
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

/// The settled view drawn again at twice the window's resolution, presented box-filtered
/// down so abutting fills stop leaking backdrop at their seams (ADR 0699).
///
/// It lives **beside** the base rather than replacing it: recording it through
/// [`crate::stale::Stale::settled`] would make the next view change refuse to reproject
/// (`Refusal::Resized` — the settled size would be twice the asked one), so the sharp
/// texture is a presentation substitute and the stale machinery never sees it.
#[derive(Debug)]
pub(crate) struct Sharpened {
    /// The pages it depicts, at their **window-resolution** targets — the identity the
    /// presenter matches against [`Shown::pages`] before substituting.
    pages: Vec<crate::stale::Placed>,
    /// The window size it was drawn for, in the window's own pixels (the texture is twice
    /// this in each axis).
    width: u32,
    height: u32,
    texture: wgpu::Texture,
    /// What the pass cost on the render thread, for the trace.
    took: Duration,
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
    /// Whether the frame was page one drawn while it was read, finished (ADR 1855).
    pub(crate) finished_ahead: bool,
    /// Whether the processor's draw was taken back: no pixels landed, and nothing about this frame
    /// may be recorded as the view settled or answered to the core.
    pub(crate) abandoned: bool,
    pub(crate) refused: Option<String>,
    pub(crate) function_refusals: Vec<String>,
}

/// The frames the graphics device refused, so that it is not asked for them again (ADR 1821).
///
/// **A refused frame costs the device's whole attempt before the processor starts**: raster prices
/// a frame's edges after it has flattened them (`raster-gpu`'s `ComputePersist::price`), so on the
/// project owner's drawing every refusal is the scene built, the edges counted, the frame priced
/// and turned down — and then the processor's draw on top. The answer does not change while the
/// question does not, so it is asked once.
///
/// **The question is the pages and the magnification, not the whole view.** Raster culls what
/// reaches no pixel of the target, so a pan at a magnification well past the page's fit could in
/// principle bring a refused frame under the budget; remembering by magnification means such a
/// pan is drawn by the processor, which is the correctness oracle and draws the same page, more
/// slowly. The other choice — the whole transform — would ask the device again at every pan of a
/// refused page and pay the attempt each time, which is the cost this exists to remove.
///
/// Only a refusal that is a fact about the frame is kept ([`remembered`]); a device lost or a
/// pipeline missing says nothing about the page and is asked again, as it always was.
#[derive(Debug, Default)]
struct Refusals {
    /// Newest first, at most [`Self::KEPT`].
    held: Vec<Refused>,
}

/// One frame the device refused: which pictures, at which magnification, and what it said.
#[derive(Debug)]
struct Refused {
    pictures: Vec<crate::stale::Picture>,
    linear: [u32; 4],
    said: String,
}

impl Refusals {
    /// How many refusals are kept: a zoom gesture's worth of magnifications of one page, and the
    /// pages a column shows beside it. An entry that ages out costs one device attempt, once.
    const KEPT: usize = 32;

    /// The key a frame is remembered by: its pictures, and the linear part of the first page's
    /// placement — the arrangement's magnification, since `viewer_core::layout` places every page
    /// of a column at the same one.
    fn key(pages: &[crate::stale::Placed]) -> Option<(Vec<crate::stale::Picture>, [u32; 4])> {
        let first = pages.first()?;
        Some((
            pages.iter().map(|placed| placed.of).collect(),
            crate::surface::linear_bits_of(first.target.transform),
        ))
    }

    /// What the device said when it refused exactly these pages at exactly this magnification.
    fn said(&self, pages: &[crate::stale::Placed]) -> Option<&str> {
        let (pictures, linear) = Self::key(pages)?;
        self.held
            .iter()
            .find(|held| held.linear == linear && held.pictures == pictures)
            .map(|held| held.said.as_str())
    }

    /// Keeps a refusal, newest first.
    fn remember(&mut self, pages: &[crate::stale::Placed], said: &str) {
        let Some((pictures, linear)) = Self::key(pages) else {
            return;
        };
        self.held
            .retain(|held| !(held.linear == linear && held.pictures == pictures));
        self.held.insert(
            0,
            Refused {
                pictures,
                linear,
                said: said.to_owned(),
            },
        );
        self.held.truncate(Self::KEPT);
    }
}

/// Whether a refusal is a fact about the frame — its pages at its magnification — rather than
/// about the device.
///
/// The budgets and the limits are: raster prices what the frame asks for against what it states
/// it will hold, and the same frame is priced the same way next time. So is what the display list
/// itself carries — a value the scene boundary refuses, a clip or a soft mask it does not define, a
/// paint this backend cannot draw — because the list is the same list. A device lost, a readback
/// failed, a pipeline still missing or an allocation the machine could not make at that moment is
/// none of these, and a page refused for one of those is asked again.
fn remembered(problem: &QuorraRasterError) -> bool {
    use raster_gpu::RenderError;
    match problem {
        QuorraRasterError::Render(refusal) => matches!(
            refusal,
            RenderError::FrameBudgetExceeded { .. }
                | RenderError::ScratchExhausted { .. }
                | RenderError::OutlineConversionBudgetExceeded { .. }
                | RenderError::TargetTooLarge { .. }
                | RenderError::ViewportTransformTooLarge { .. }
        ),
        QuorraRasterError::Scene(_)
        | QuorraRasterError::UnknownClip(_)
        | QuorraRasterError::CyclicClip(_)
        | QuorraRasterError::UnknownSoftMask(_)
        | QuorraRasterError::Unsupported(_)
        | QuorraRasterError::Target(_) => true,
        // The device, an allocation, and whatever `render-raster` adds later: asked again until
        // somebody argues that the new one is a fact about the frame.
        _ => false,
    }
}

/// The frame the render thread is drawing, as the event thread remembers it.
///
/// **The pages and the interrupt are what is new** (ADR 1821), and they are the shape
/// [`crate::composer`]'s `InFlight` has had since ADR 0657: the pages so that this window can ask
/// whether the frame being drawn is still worth finishing, and the interrupt so that it can say
/// no — which reaches the processor's half of a refused frame and nothing else, because the
/// device's half is one call raster cannot take back.
#[derive(Debug)]
struct Flight {
    asked: Instant,
    pages: Vec<crate::stale::Placed>,
    interrupt: Interrupt,
    on_the_processor: Arc<AtomicBool>,
}

/// This window's half of the arrangement: the surface, and a handle on the thread that draws.
///
/// Lives on the event thread and never leaves it. What it owns of raster is a
/// [`raster_gpu::Presenter`] — the surface, its swapchain and one pipeline — which is `Send` and
/// which no `&mut Device` stands in front of.
pub(crate) struct Window {
    presenter: raster_gpu::Presenter,
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
    /// `--supersample`'s factor: 1 off, 2 on. Carried to the render thread with the spawn.
    supersample: u32,
    /// `--coverage`'s choice, for the idle work this thread's counterpart draws.
    coverage: crate::arguments::CoverageChoice,
    /// What wakes the event loop when idle work lands while it is at rest. Without it a
    /// sharp picture would sit in the channel until the next input.
    waker: Option<winit::event_loop::EventLoopProxy<()>>,
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
    /// view changes — [`crate::composer`]'s field of the same name, for its reason: this window
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
/// (`wgpu-30.0.0/src/api/surface.rs`). This viewer submits from [`Self::ask`]'s render thread and
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
pub(crate) struct Ungrounded(Window);

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
    pub(crate) fn ground(mut self) -> Result<Window, String> {
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
    /// The renderer is *not* moved to a thread here: see [`Self::idle`]. **Nor is a `Window`
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
    pub(crate) fn split(
        mut renderer: QuorraWindowRenderer,
        size: (u32, u32),
        proxy_pages: usize,
        supersample: u32,
        coverage: crate::arguments::CoverageChoice,
        waker: Option<winit::event_loop::EventLoopProxy<()>>,
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
    pub(crate) fn device_said(&self) -> Option<render_raster::Uncaptured> {
        self.uncaptured.take()
    }

    /// The adapter raster selected, for reports.
    pub(crate) fn description(&self) -> &str {
        &self.description
    }

    /// What bringing the device up cost, in the parts raster measures it in.
    ///
    /// **Read before the device left this thread**, so its `pipeline_compilation` is whatever was
    /// true at bring-up — which is `None`, because nothing on the launch path waits for warmth.
    /// [`Self::pipelines`] is where the answer arrives.
    pub(crate) fn startup(&self) -> raster_gpu::StartupTimings {
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
    pub(crate) fn drawing(&self) -> bool {
        self.in_flight.is_some()
    }

    /// Whether the frame in flight is being drawn on the processor and has been out for longer
    /// than `viewer_host::drawing::WARN` — [`crate::composer::Composer::overlong`]'s question,
    /// asked of the one frame on this window that can be stopped.
    ///
    /// **A frame on the device is never overlong here, and that is not an oversight**: raster
    /// draws it in one call nothing can take back (ADR 0725), so a warning offering a key would
    /// offer one that does nothing. A frame the device refused is drawn by the same processor the
    /// `--cpu` window draws with, and that draw takes the interrupt (ADR 1821).
    pub(crate) fn overlong(&self) -> bool {
        self.in_flight.as_ref().is_some_and(|flight| {
            flight.on_the_processor.load(Ordering::Acquire)
                && !flight.interrupt.raised()
                && flight.asked.elapsed() >= viewer_host::drawing::WARN
        })
    }

    /// Takes the processor's draw back because the **person** asked —
    /// [`crate::composer::Composer::abandon`], on this window.
    ///
    /// Answers whether it took anything back: `false` where nothing is being drawn on the
    /// processor, which is also the race between the key and the frame's last command.
    pub(crate) fn abandon(&mut self) -> bool {
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
    pub(crate) fn declined(&self, pages: &[crate::stale::Placed]) -> bool {
        !self.declined.is_empty() && same_pages(&self.declined, pages)
    }

    /// Takes the processor's draw back where finishing it would buy a picture of a magnification
    /// the view has already left (ADR 1821).
    ///
    /// [`crate::composer::Composer::superseded`] interrupts where the finished frame could not
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
    pub(crate) fn superseded(&mut self, wanted: &[crate::stale::Placed]) -> bool {
        let Some(flight) = self.in_flight.as_ref() else {
            return false;
        };
        if flight.interrupt.raised() {
            return false;
        }
        let magnification = |pages: &[crate::stale::Placed]| {
            pages
                .first()
                .map(|placed| crate::surface::linear_bits_of(placed.target.transform))
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
    pub(crate) fn out_for(&self) -> Option<Duration> {
        self.in_flight.as_ref().map(|flight| flight.asked.elapsed())
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
        let mut sharp = None;
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
        let done = newest?;
        self.pipelines = self.pipelines.or(done.pipelines);
        self.in_flight = None;
        self.partial = None;
        self.partial_news = false;
        let drew = done.finished.saturating_duration_since(done.started);
        // What each lane charged for these pages, which the next frame of them is priced by. A
        // draw taken back is a sample of nothing (`adopt_composed`'s reason, ADR 0657).
        if !done.abandoned {
            if let Some(took) = done.device_drew {
                self.costs.measure(&done.pages, Lane::Device, took);
            }
            if let Some(took) = done.processor_drew
                && done.refused.is_none()
            {
                self.costs.measure(&done.pages, Lane::Processor, took);
            }
        }
        if done.refused_now
            && let Some(said) = done.fell_back.as_deref()
        {
            self.refusals.remember(&done.pages, said);
            self.costs.refused(&done.pages);
        }
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
                processor_drew: done.processor_drew,
                chose: done.chose,
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
            processor_drew: done.processor_drew,
            chose: done.chose,
            finished_ahead: done.finished_ahead,
            abandoned: false,
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
        let known_refusal = self.refusals.said(&pages).map(str::to_owned);
        let cheaper = if known_refusal.is_none() {
            self.costs.processor_for(&pages)
        } else {
            None
        };
        // Page one drawn while it was read, where this is its frame at the target it was drawn at
        // (ADR 1855). The lane is the processor's for it: all of the page but its last part is
        // already drawn there, so the processor is the faster lane by every prediction, which is
        // the condition A393 puts on page one drawn there.
        let ahead = match (self.ahead.as_ref(), pages.as_slice()) {
            (Some(ahead), [placed]) => ahead.take_for(&placed.list, placed.target),
            _ => None,
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
    pub(crate) fn draw_ahead(&mut self, ahead: viewer_host::Ahead) {
        self.ahead = Some(ahead);
    }

    /// Sends the newest look at page one being read to be uploaded, where it is newer than the
    /// last one sent — on the window's tick while the document is still opening. The upload
    /// comes back through [`Self::collect`] as the picture [`Self::present_as_built`] puts up.
    pub(crate) fn show_read(&mut self, ahead: &viewer_host::Ahead) {
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
    pub(crate) fn underlay(&self, pages: &[crate::stale::Placed]) -> Vec<(usize, Transform)> {
        self.proxies.placements(pages)
    }

    /// The chrome the frame on hand was drawn with, or was asked to be drawn with.
    ///
    /// What [`crate::surface`] compares against this frame's overlays, for the reason
    /// [`Self::chrome`] states: a rendering of the same pages with *different* chrome is a
    /// different picture, and skipping it left the find bar and the notices card unreachable.
    pub(crate) fn chrome_asked(&self) -> &[DisplayList] {
        &self.chrome
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
    pub(crate) fn drawn_so_far(&self) -> Option<f64> {
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
    pub(crate) fn shows_as_built(&self, pages: &[crate::stale::Placed]) -> bool {
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
    pub(crate) fn present_as_built(&mut self) -> Result<Option<f64>, raster_gpu::RenderError> {
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
    pub(crate) fn last_present(&self) -> Option<raster_gpu::PresentCost> {
        self.presenter.last()
    }

    /// What the sharp pass that just landed cost, once — for the trace, which is rule 3's
    /// half of ADR 0699: a picture that quietly got better is still a picture that changed.
    pub(crate) fn sharpened(&mut self) -> Option<Duration> {
        self.sharp_news.take()
    }
}

/// What the render thread does between jobs, as the window configured it.
#[derive(Debug)]
struct Idle {
    /// How many whole pages it retains a low-resolution picture of.
    proxy_pages: usize,
    /// `--supersample`'s factor.
    supersample: u32,
    /// `--coverage`'s choice.
    coverage: crate::arguments::CoverageChoice,
    /// Where it leaves the flag that takes a whole page drawn on the processor back
    /// ([`Window::idle_interrupt`]).
    interrupt: Arc<std::sync::Mutex<Option<Interrupt>>>,
}

/// Starts the render thread around a renderer, and hands back the channels to it.
fn spawn(
    renderer: QuorraWindowRenderer,
    idle: Idle,
    waker: Option<winit::event_loop::EventLoopProxy<()>>,
) -> Link {
    let (jobs, incoming) = channel::<Job>();
    let (outgoing, done) = channel::<Finished>();
    // A spawn this machine refuses is a machine with no thread to spare, and the window then
    // presents whatever it has and asks for frames nobody answers — which is what
    // `Link::jobs.send` failing already means, so there is one path rather than two.
    let thread = std::thread::Builder::new()
        .name("page renderer".to_owned())
        .spawn(move || {
            draw_until_told_to_stop(renderer, &idle, waker.as_ref(), &incoming, &outgoing);
        })
        .ok();
    Link { jobs, done, thread }
}

impl Done {
    /// Whether the processor drew this frame: the device refused it, the processor was predicted
    /// the faster lane, or it was page one drawn while it was read (ADRs 1821, 1831, 1855).
    fn on_the_processor(&self) -> bool {
        self.fell_back.is_some() || self.chose.is_some() || self.finished_ahead
    }
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
#[expect(
    clippy::too_many_lines,
    reason = "the render thread's one loop, its kinds of work in the order they win: a look, a \
              frame, the sharp pass, a retained page. Each kind's work is a function already; \
              what is left here is the order, which reads best in one place"
)]
fn draw_until_told_to_stop(
    mut renderer: QuorraWindowRenderer,
    idle: &Idle,
    waker: Option<&winit::event_loop::EventLoopProxy<()>>,
    jobs: &Receiver<Job>,
    done: &Sender<Finished>,
) {
    use std::sync::mpsc::TryRecvError;

    let (supersample, coverage) = (idle.supersample, idle.coverage);
    let mut proxies: crate::stale::Proxies<Arc<wgpu::Texture>> =
        crate::stale::Proxies::new(idle.proxy_pages);
    // What the last frame drew, which is the only set of pages this thread has display lists for.
    let mut showing: Vec<crate::stale::Placed> = Vec::new();
    // The window size the last frame was drawn for — what the sharp pass doubles.
    let mut shown_size = (0_u32, 0_u32);
    // Whether the view on hand has had its sharp pass. True after one attempt either way:
    // a refusal (a device limit, most likely) must not spin, exactly as `draw_whole_page`'s
    // silence must not — the next view change resets it.
    let mut sharpened = false;
    // What the last frame this thread had to *build* cost — [`sharp_pass_affordable`]'s
    // prediction. A replay says what a replay costs and nothing about what a 4× render
    // will, which is the same reading the event thread makes for rule 5's prediction
    // (`crate::surface`'s `adopt`, ADR 0384).
    let mut last_built: Option<Duration> = None;
    // The chrome lane needs a target of its own even where there is no chrome, so one scratch
    // texture is kept for whatever size the proxies are and remade when a page of another shape
    // arrives. One allocation per page shape rather than one per proxy.
    let mut scratch: Option<(u32, u32, wgpu::Texture)> = None;
    // The pages whose whole-page picture the device would not draw, which are not asked for again
    // (ADR 1821). `draw_whole_page`'s silence stays: a proxy is a picture nobody asked for. What
    // changes is that its refusal stops being paid once per frame — on a page the device refuses
    // that was the scene built and priced again after every frame, in front of the next one.
    let mut no_proxy: Vec<crate::stale::Picture> = Vec::new();
    // The pages whose last frame the processor drew — refused by the device, or predicted cheaper
    // there (ADR 1831). Their whole-page picture is the processor's too: the device would refuse
    // it or take as long over it as over the frame, and a device draw cannot be taken back when
    // the next job arrives, where the processor's can.
    let mut on_the_processor: Vec<crate::stale::Picture> = Vec::new();
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
        if let Some(mut job) = job {
            if let Some(look) = job.look.take() {
                if !sent_read(&mut renderer, (job.width, job.height), &look, (done, waker)) {
                    return;
                }
                continue;
            }
            // A job of the same pages at the same size — a chrome-only change, a selection,
            // the find bar — leaves the sharp picture true, so it is not paid for again.
            if !(same_pages(&showing, &job.pages) && shown_size == (job.width, job.height)) {
                sharpened = false;
            }
            showing.clone_from(&job.pages);
            shown_size = (job.width, job.height);
            let finished = draw(&mut renderer, job, done, waker);
            // Where the frame was sent, not whether it began: a frame taken back before the
            // processor started was still the processor's, and its page's picture is too.
            let drawn_there = finished.on_the_processor();
            for placed in &finished.pages {
                on_the_processor.retain(|picture| *picture != placed.of);
                if drawn_there {
                    on_the_processor.push(placed.of);
                }
            }
            // **A view the device refused is not sharpened by it** (ADR 1821): the pass is the
            // same view at four times the pixels, which raster prices higher rather than lower,
            // so asking would build and price a scene to hear the answer this frame already gave.
            // Nor one the processor drew because it was the faster lane (ADR 1831): the device
            // was predicted to take seconds over the view at one times the pixels.
            let processor_frame = finished.on_the_processor();
            if processor_frame {
                sharpened = true;
            }
            // A processor's frame is an upload to the device, whose cost says nothing about what
            // the device would take to build the page (trap 143), so it predicts no pass.
            if !processor_frame
                && !matches!(
                    finished.cost.encode_source,
                    Some(raster_gpu::EncodeSource::Replayed)
                )
            {
                last_built = Some(finished.cost.total);
            }
            // A send that fails is an event thread that has gone, and there is nobody left to draw
            // for. The loop would end at the next `recv` anyway; leaving now saves a frame nobody
            // would see.
            if done.send(Finished::Frame(Box::new(finished))).is_err() {
                return;
            }
            continue;
        }
        // The sharp pass goes before the proxies: it is what the person is looking at now,
        // where a proxy is for a view change that has not happened yet. Declined without a
        // word where the prediction says the machine cannot afford it — `draw_sharp`'s own
        // silence rule: this is a picture nobody asked for, and what a decline costs is a
        // seam drawn at full width rather than half (ADR 0699), not a wrong pixel. The flag
        // is set either way, so a declined view is not re-asked until the view changes.
        if supersample >= 2 && !sharpened && !showing.is_empty() {
            // **The view has to have been still for as long as the pass will take before the
            // pass may begin** (ADR 1289). It cannot be taken back once begun, so a view change
            // arriving during it waits for all of it, and on the owner's 890M the worst page's
            // pass costs what a zoom step does (57–63 ms against 55–62): begun the moment the
            // thread is idle, it puts a whole step in front of every step of a gesture, and the
            // window shows a stand-in from the first notch to the last. Waiting out the pass's
            // own predicted length first is the rent-or-buy bound: a view change is delayed by
            // the pass only after the person has already been still for at least as long as the
            // delay, and never by a pass begun in a pause shorter than itself.
            if let Some(stillness) =
                sharp_pass_cost(last_built).filter(|pass| *pass <= SHARP_STALL_BUDGET)
            {
                match jobs.recv_timeout(stillness) {
                    Ok(job) => {
                        waiting = Some(job);
                        continue;
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
            sharpened = true;
            if sharp_pass_affordable(last_built)
                && let Some(sharp) = draw_sharp(&mut renderer, &showing, shown_size, coverage)
            {
                if done.send(Finished::Sharp(Box::new(sharp))).is_err() {
                    return;
                }
                // The loop this picture is for may be at rest — a settled view is exactly
                // when it is — so it is woken. A send to a loop that has exited is nothing.
                if let Some(waker) = waker {
                    let _ = waker.send_event(());
                }
                continue;
            }
        }
        match retain_one(
            &mut renderer,
            (&showing, &on_the_processor),
            (&mut proxies, &mut no_proxy, &mut scratch),
            idle,
            jobs,
            done,
        ) {
            Turn::Drew => continue,
            Turn::Job(job) => {
                waiting = Some(*job);
                continue;
            }
            Turn::Stop => return,
            Turn::Nothing => {}
        }
        match jobs.recv() {
            Ok(job) => waiting = Some(job),
            Err(_) => return,
        }
    }
}

/// The render thread's own record of the whole pages it holds pictures of.
type Store = crate::stale::Proxies<Arc<wgpu::Texture>>;

/// The chrome lane's target for a whole page drawn on the device, kept per page shape.
type Scratch = Option<(u32, u32, wgpu::Texture)>;

/// What one idle turn spent on the retained layer came to.
enum Turn {
    /// A page was drawn, or given up on, or taken back: the loop looks for work again.
    Drew,
    /// A job arrived first.
    Job(Box<Job>),
    /// The event thread has gone.
    Stop,
    /// No page wants a picture.
    Nothing,
}

/// One idle turn's page for the retained layer, where one wants a picture (ADR 0443, ADR 1831).
fn retain_one(
    renderer: &mut QuorraWindowRenderer,
    (showing, on_the_processor): (&[crate::stale::Placed], &[crate::stale::Picture]),
    (proxies, no_proxy, scratch): (&mut Store, &mut Vec<crate::stale::Picture>, &mut Scratch),
    idle: &Idle,
    jobs: &Receiver<Job>,
    done: &Sender<Finished>,
) -> Turn {
    let drawable: Vec<crate::stale::Placed> = showing
        .iter()
        .filter(|placed| !no_proxy.contains(&placed.of))
        .cloned()
        .collect();
    let Some(page) = proxies.wanted(&drawable) else {
        return Turn::Nothing;
    };
    let on_the_device = !on_the_processor.contains(&page.of);
    let retained = match retain(renderer, &page, on_the_device, scratch, idle, jobs) {
        Drawn::Picture(retained) => retained,
        Drawn::Behind(job) => return Turn::Job(job),
        Drawn::Taken => return Turn::Drew,
        Drawn::Refused => {
            // The next page that wants one is asked for at the next idle turn, and this one never
            // again for this ink; the store stays a document's pages long, as `Proxies` is.
            no_proxy.retain(|picture| picture.same_ink(page.of));
            no_proxy.push(page.of);
            return Turn::Drew;
        }
    };
    // Kept here as well as sent, because this thread is the one that decides what to draw next
    // and a store that forgot would draw the same page for ever.
    proxies.keep(crate::stale::Retained {
        of: retained.of,
        target: retained.target,
        pixels: Arc::clone(&retained.pixels),
    });
    if done.send(Finished::Page(retained)).is_err() {
        return Turn::Stop;
    }
    Turn::Drew
}

/// Whether two arrangements are the same pictures at the same placements — [`crate::stale::
/// Placed::is`]'s question over a whole list, which is what decides both whether a sharp
/// picture is still true and whether a job invalidates it.
fn same_pages(a: &[crate::stale::Placed], b: &[crate::stale::Placed]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.is(b))
}

/// The most the settled view's sharp pass may be *predicted* to cost before it is declined.
///
/// The prediction is four times the last frame this thread had to build — the pass draws
/// the same commands at four times the pixels — and the pass is uninterruptible: raster
/// draws it in one call and one submission, so for its whole length a newly arrived view
/// change waits behind it, and on DirectX 12 so does every present the event thread
/// issues, because a present is a queue operation and executes after whatever was
/// submitted before it. This budget bounds that stall; it is the cost ADR 0699 already
/// priced and accepted (~350 ms of idle-thread time on the worst page), written down as a
/// bound rather than left open-ended.
///
/// **The number is a choice between two measured machines, not a derivation** (ADR 0761).
/// On the owner's Radeon 890M a settled zoom frame of the worst page builds in 53–66 ms
/// (`doc/todo/46`), predicting the pass at ~270 ms, inside the budget — it runs. On the
/// owner's Windows Intel UHD (DX12, the 2026-08-31 trace) the first frame of the same
/// page built in 431.6 ms, and the unbounded pass measured **8 867.6 ms**, during which
/// the window's presents blocked for up to 5.25 s and a zoom the person had already made
/// waited eight seconds for its first real frame. Any bound between those two populations
/// decides both correctly.
const SHARP_STALL_BUDGET: Duration = Duration::from_millis(400);

/// Whether the sharp pass is predicted to land inside [`SHARP_STALL_BUDGET`].
///
/// `last_built` is the cost of the last frame this thread built rather than replayed.
/// `None` — no frame built yet — declines, because a pass nobody asked for is not worth
/// starting with no evidence it is affordable; in practice the pass is only reachable
/// after a first job, which always builds.
fn sharp_pass_affordable(last_built: Option<Duration>) -> bool {
    sharp_pass_cost(last_built).is_some_and(|pass| pass <= SHARP_STALL_BUDGET)
}

/// What the sharp pass is predicted to cost: four times the last frame this thread built,
/// because it draws the same commands at four times the pixels (ADR 0761). `None` before any
/// frame has been built.
///
/// It is both the number [`SHARP_STALL_BUDGET`] bounds and how long the view must have been
/// still before the pass begins (ADR 1289), and the prediction is an over-estimate on the
/// compute lane — the worst page's pass measured 57–63 ms against frames of 55–62 — which
/// errs towards waiting longer, the side that costs a picture nobody asked for a little later
/// rather than a picture somebody did.
fn sharp_pass_cost(last_built: Option<Duration>) -> Option<Duration> {
    last_built.map(|frame| frame.saturating_mul(4))
}

/// The settled view again, at twice the window's resolution, for the presenter to show
/// box-filtered down (ADR 0699).
///
/// `None` where the device refused — most plausibly `2 × window` past the adapter's
/// texture limit — and silent for `draw_whole_page`'s reason: this is a picture nobody
/// asked for, and the caller's `sharpened` flag keeps a refusal from spinning.
fn draw_sharp(
    renderer: &mut QuorraWindowRenderer,
    showing: &[crate::stale::Placed],
    size: (u32, u32),
    coverage: crate::arguments::CoverageChoice,
) -> Option<Sharpened> {
    let began = Instant::now();
    let (width, height) = (size.0.saturating_mul(2), size.1.saturating_mul(2));
    if width == 0 || height == 0 {
        return None;
    }
    let doubled: Vec<(&Arc<DisplayList>, TargetSpec)> = showing
        .iter()
        .map(|placed| {
            (
                &placed.list,
                TargetSpec {
                    width,
                    height,
                    transform: placed.target.transform.then(Transform::scale(2.0, 2.0)),
                },
            )
        })
        .collect();
    // The sharp pass is a moved view by construction — twice the shown transform — so
    // `auto` takes the compute lane, which also keeps the 2× tiles out of the glyph
    // atlas (the thrash ADR 0699 priced) because that lane has no atlas at all. On a
    // software adapter the dispatch loses (`lane_for`'s reason) and the pass keeps the
    // processor's lanes.
    renderer.set_coverage(match coverage {
        crate::arguments::CoverageChoice::Fixed(lane) => lane,
        crate::arguments::CoverageChoice::Auto => {
            if crate::surface::software_adapter(renderer.adapter_description()) {
                raster_gpu::Coverage::Cpu
            } else {
                raster_gpu::Coverage::Compute
            }
        }
    });
    // A fresh texture per pass rather than a pooled one: the one it replaces is still on
    // the window until the event thread adopts this, and a sharp pass happens once per
    // settled view rather than once per frame — the pool that keeps `Pair` cheap would
    // here be a second four-window allocation held for ever.
    let texture = renderer.layer_texture("the sharpened page", width, height);
    renderer
        .render_page_only(
            PresentFrame {
                width,
                height,
                pages: &doubled,
                raster: None,
                overlays: &[],
            },
            &texture,
        )
        .ok()?;
    Some(Sharpened {
        pages: showing.to_vec(),
        width: size.0,
        height: size.1,
        texture,
        took: began.elapsed(),
    })
}

/// Draws one whole page into a raster of its own, for the layer under the base.
///
/// `None` where the page states no usable extent, or where the device refused it — **and it is
/// deliberately silent**, because this is a picture nobody asked for: a refusal costs a stand-in
/// that will not be as good and nothing else, and reporting it would be a note about work the
/// person did not request. It cannot spin, and it is not paid twice: the caller keeps the page out
/// of every later idle turn for as long as its ink stands (ADR 1821).
fn draw_whole_page(
    renderer: &mut QuorraWindowRenderer,
    page: &crate::stale::Placed,
    scratch: &mut Option<(u32, u32, wgpu::Texture)>,
) -> Option<crate::stale::Retained<Arc<wgpu::Texture>>> {
    let target = crate::stale::proxy_target(&page.list)?;
    let (width, height) = (target.width, target.height);
    // A whole page at a few hundred pixels is far below the magnification at which raster's GPU
    // coverage lane is the cheaper one, whatever the window is showing (`crate::surface`), so the
    // lane is stated rather than inherited from whatever the last frame asked for. `draw` sets it
    // per job, so the next window frame is unaffected.
    renderer.set_coverage(raster_gpu::Coverage::Cpu);
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
    let placed = [(&page.list, target)];
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
        of: page.of,
        target,
        pixels: Arc::new(texture),
    })
}

/// What a whole page drawn for the retained layer came to.
enum Drawn {
    /// The picture, uploaded.
    Picture(crate::stale::Retained<Arc<wgpu::Texture>>),
    /// A job had arrived before the processor began, and goes first.
    Behind(Box<Job>),
    /// A job arrived and took the thread back; the page is asked for again at the next idle turn.
    Taken,
    /// Neither lane would draw it.
    Refused,
}

/// One whole page for the retained layer: on the device where its frame is drawn there, and on the
/// processor where the frame is or the device refused this picture (ADR 1831).
fn retain(
    renderer: &mut QuorraWindowRenderer,
    page: &crate::stale::Placed,
    on_the_device: bool,
    scratch: &mut Option<(u32, u32, wgpu::Texture)>,
    idle: &Idle,
    jobs: &Receiver<Job>,
) -> Drawn {
    if on_the_device && let Some(retained) = draw_whole_page(renderer, page, scratch) {
        return Drawn::Picture(retained);
    }
    let interrupt = Interrupt::new();
    if let Ok(mut held) = idle.interrupt.lock() {
        *held = Some(interrupt.clone());
    }
    // A job that arrived before the flag was left is a job this draw would stand in front of:
    // asked first, and the page is drawn at the next idle turn.
    if let Ok(job) = jobs.try_recv() {
        return Drawn::Behind(Box::new(job));
    }
    draw_whole_page_on_the_processor(renderer, page, &interrupt)
}

/// [`draw_whole_page`] on the processor, for a page whose frame the processor draws (ADR 1831).
///
/// Silent for that function's reason — a picture nobody asked for — and interruptible because the
/// processor's draw is: [`Window::ask`] raises `interrupt` the moment a job is sent, so a view
/// change waits for the command in progress rather than for a whole page drawn small.
fn draw_whole_page_on_the_processor(
    renderer: &mut QuorraWindowRenderer,
    page: &crate::stale::Placed,
    interrupt: &Interrupt,
) -> Drawn {
    let Some(target) = crate::stale::proxy_target(&page.list) else {
        return Drawn::Refused;
    };
    let drawn = viewer_ui::software::compose_pages_interruptibly(
        &[(page.list.as_ref(), target)],
        Some(interrupt),
    );
    let raster = match drawn {
        Ok(raster) => raster,
        Err(problem) if viewer_ui::software::was_interrupted(&problem) => return Drawn::Taken,
        Err(_) => return Drawn::Refused,
    };
    let texture = renderer.layer_texture("a retained page", target.width, target.height);
    let uploaded = renderer.render_page_only(
        PresentFrame {
            width: target.width,
            height: target.height,
            pages: &[],
            raster: Some(&raster),
            overlays: &[],
        },
        &texture,
    );
    if uploaded.is_err() {
        return Drawn::Refused;
    }
    Drawn::Picture(crate::stale::Retained {
        of: page.of,
        target,
        pixels: Arc::new(texture),
    })
}

/// The processor's half of a frame the device refused: `None` where the interrupt took it back,
/// and how long the processor drew, where it was started.
///
/// **The one half of a refused frame that can be taken back** (ADR 1821): the event thread raises
/// the interrupt where the view has left this magnification or the person has pressed the key, and
/// a raise that came while the device was being asked is honoured here, before anything starts.
///
/// **And the half that can be shown as it is built** (ADR 1831): the draw runs under
/// [`viewer_ui::software::compose_pages_watched`], and every look is uploaded into a pair of its
/// own by `shown` and sent to the window, which puts it up where it has no truer picture.
fn draw_on_the_processor(
    pages: &[crate::stale::Placed],
    interrupt: &Interrupt,
    on_the_processor: &AtomicBool,
    mut shown: impl FnMut(&viewer_ui::software::Assembly),
) -> (Option<Result<pdf_render::Raster, String>>, Option<Duration>) {
    if interrupt.raised() {
        return (None, None);
    }
    if pages.is_empty() {
        return (
            Some(Err("there is no page to draw on the processor".to_owned())),
            None,
        );
    }
    let borrowed: Vec<(&DisplayList, TargetSpec)> = pages
        .iter()
        .map(|placed| (placed.list.as_ref(), placed.target))
        .collect();
    on_the_processor.store(true, Ordering::Release);
    let began = Instant::now();
    let drawn = viewer_ui::software::compose_pages_watched(
        &borrowed,
        Some(interrupt),
        viewer_ui::software::Looks::SHIPPED,
        |assembly| {
            if !interrupt.raised() {
                shown(assembly);
            }
        },
    );
    let drew = Some(began.elapsed());
    match drawn {
        // Asked of the error rather than of the flag, for `crate::composer`'s reason: a frame
        // finished before the raise is a frame this window can present.
        Err(problem) if viewer_ui::software::was_interrupted(&problem) => (None, drew),
        drawn => (Some(drawn.map_err(|problem| problem.to_string())), drew),
    }
}

/// One job, drawn — on the device, or on the processor where the device refused.
#[expect(
    clippy::too_many_lines,
    reason = "one frame's two lanes in the order they are tried, each with what it reports; the \
              sequence is the function, and splitting it would hand each half the other's locals"
)]
fn draw(
    renderer: &mut QuorraWindowRenderer,
    job: Job,
    done: &Sender<Finished>,
    waker: Option<&winit::event_loop::EventLoopProxy<()>>,
) -> Done {
    let started = Instant::now();
    let Job {
        width,
        height,
        pages,
        overlays,
        coverage,
        reuse,
        known_refusal,
        cheaper,
        interrupt,
        on_the_processor,
        ahead,
        look: _,
    } = job;
    let drawn_ahead = ahead.is_some();
    let textures = reuse
        .filter(|pair| pair.fits(width, height))
        .unwrap_or_else(|| Pair::new(renderer, width, height));
    // Which lane draws this frame's coverage, decided from this frame's magnification by the
    // thread that knows it (`crate::surface::coverage_for`) and carried here.
    renderer.set_coverage(coverage);
    let borrowed: Vec<&DisplayList> = overlays.iter().collect();
    let placed: Vec<(&Arc<DisplayList>, TargetSpec)> = pages
        .iter()
        .map(|placed| (&placed.list, placed.target))
        .collect();
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
    let mut refused = None;
    let mut device_tried = None;
    let mut device_drew = None;
    let mut processor_drew = None;
    let mut finished_ahead = false;
    let mut abandoned = false;
    // **A refusal already known is not asked again** (ADR 1821): the device's attempt is the
    // whole scene built and priced before it says no, and it would say the same thing. **Nor is a
    // frame the processor was predicted to draw faster** (ADR 1831).
    let (fell_back, refused_now) = if let Some(said) = known_refusal {
        (Some(said), false)
    } else if cheaper.is_some() || drawn_ahead {
        (None, false)
    } else {
        let asked = Instant::now();
        match renderer.render(frame, into) {
            Ok(()) => {
                if !matches!(
                    renderer.last_frame().encode_source,
                    Some(raster_gpu::EncodeSource::Replayed)
                ) {
                    device_drew = Some(asked.elapsed());
                }
                (None, false)
            }
            Err(problem) => {
                device_tried = Some(asked.elapsed());
                (Some(problem.to_string()), remembered(&problem))
            }
        }
    };
    if fell_back.is_some() || cheaper.is_some() || drawn_ahead {
        let drawn = processor_half(
            renderer,
            (width, height),
            (&pages, ahead),
            &borrowed,
            (&interrupt, &on_the_processor),
            (done, waker),
            into,
        );
        // A frame finished from the page drawn while it was read measures nothing about what the
        // processor charges for the page: its cost is the last part's (ADR 0657's rule).
        finished_ahead = drawn.finished_ahead;
        processor_drew = drawn.drew.filter(|_| !finished_ahead);
        abandoned = drawn.abandoned;
        refused = drawn.refused;
    }
    Done {
        textures,
        pages,
        // An abandoned frame drew nothing, and the device's last frame is somebody else's.
        cost: if abandoned {
            FrameCost::default()
        } else {
            renderer.last_frame()
        },
        fell_back,
        refused_now,
        device_tried,
        processor_drew,
        device_drew,
        chose: cheaper,
        finished_ahead,
        abandoned,
        refused,
        function_refusals: renderer.last_function_paints().refusals().to_vec(),
        pipelines: renderer.startup().pipeline_compilation,
        started,
        finished: Instant::now(),
    }
}

/// What the processor's half of a frame came to.
struct ProcessorHalf {
    drew: Option<Duration>,
    /// Whether the frame was finished from the page drawn while it was read.
    finished_ahead: bool,
    abandoned: bool,
    refused: Option<String>,
}

/// The processor's half of a frame: drawn under its looks, each uploaded and sent to the window,
/// and the finished raster uploaded into the job's pair (ADR 1821, ADR 1831).
fn processor_half(
    renderer: &mut QuorraWindowRenderer,
    (width, height): (u32, u32),
    (pages, ahead): (&[crate::stale::Placed], Option<render_cpu::Underway>),
    borrowed: &[&DisplayList],
    (interrupt, on_the_processor): (&Interrupt, &AtomicBool),
    (done, waker): (
        &Sender<Finished>,
        Option<&winit::event_loop::EventLoopProxy<()>>,
    ),
    into: WindowTextures<'_>,
) -> ProcessorHalf {
    // **One of the two jobs `CLAUDE.md` keeps the CPU backend for**: a page the device refuses is
    // a page this program can still show, more slowly, which is a cost a person can see past where
    // a page that never appears is not. The raster goes back through the device as one image,
    // because a window's pixels have one path and this is it — and it is the whole arrangement's
    // raster since ADR 0442, because a column the device refused is still a column.
    let looked_at = |assembly: &viewer_ui::software::Assembly| {
        let pair = Pair::new(renderer, width, height);
        let uploaded = renderer.render(
            PresentFrame {
                width,
                height,
                pages: &[],
                raster: Some(assembly.picture()),
                overlays: borrowed,
            },
            WindowTextures {
                page: &pair.page,
                chrome: &pair.chrome,
            },
        );
        // A look the device would not take is a look the window goes without: the frame itself
        // is still being drawn, and its own upload says what the device refused.
        if uploaded.is_ok() {
            let partial = Partial {
                pages: pages.to_vec(),
                textures: pair,
                fraction: assembly.fraction(),
            };
            if done.send(Finished::Partial(Box::new(partial))).is_ok()
                && let Some(waker) = waker
            {
                let _ = waker.send_event(());
            }
        }
    };
    // Page one drawn while it was read is finished here from the list it was read into, and a page
    // whose drawing ahead cannot be its picture is drawn whole as every other is (ADR 1855).
    let began = Instant::now();
    let finished = match (ahead, pages) {
        (Some(ahead), [placed]) => ahead.finish(&placed.list).ok(),
        _ => None,
    };
    let finished_ahead = finished.is_some();
    let (drawn, drew) = match finished {
        Some(raster) => {
            on_the_processor.store(true, Ordering::Release);
            (Some(Ok(raster)), Some(began.elapsed()))
        }
        None => draw_on_the_processor(pages, interrupt, on_the_processor, looked_at),
    };
    let (abandoned, refused) = match drawn {
        None => (true, None),
        Some(Ok(raster)) => {
            let second = renderer.render(
                PresentFrame {
                    width,
                    height,
                    pages: &[],
                    raster: Some(&raster),
                    overlays: borrowed,
                },
                into,
            );
            (
                false,
                second
                    .err()
                    .map(|second| format!("and the processor's page {second}")),
            )
        }
        Some(Err(second)) => (false, Some(format!("and {second}"))),
    };
    ProcessorHalf {
        drew,
        finished_ahead,
        abandoned,
        refused,
    }
}

/// A look at page one being read, uploaded and sent to the window, which is woken for it — and
/// `false` only where the window has gone. A look the device would not take is one the window
/// goes without. An upload and nothing else: no frame, no pages, nothing for the render thread's
/// stores to learn (ADR 1855).
fn sent_read(
    renderer: &mut QuorraWindowRenderer,
    size: (u32, u32),
    look: &pdf_render::Raster,
    (done, waker): (
        &Sender<Finished>,
        Option<&winit::event_loop::EventLoopProxy<()>>,
    ),
) -> bool {
    let Some(read) = upload_read(renderer, size, look) else {
        return true;
    };
    if done.send(Finished::Read(Box::new(read))).is_err() {
        return false;
    }
    if let Some(waker) = waker {
        let _ = waker.send_event(());
    }
    true
}

/// A look at page one being read, uploaded into a pair of its own for the window to put up
/// (ADR 1855) — `None` where the device would not take it, and the window goes without.
fn upload_read(
    renderer: &mut QuorraWindowRenderer,
    (width, height): (u32, u32),
    look: &pdf_render::Raster,
) -> Option<Partial> {
    let pair = Pair::new(renderer, width, height);
    renderer
        .render(
            PresentFrame {
                width,
                height,
                pages: &[],
                raster: Some(look),
                overlays: &[],
            },
            WindowTextures {
                page: &pair.page,
                chrome: &pair.chrome,
            },
        )
        .ok()?;
    Some(Partial {
        pages: Vec::new(),
        textures: pair,
        fraction: 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two machines [`SHARP_STALL_BUDGET`] was chosen between (ADR 0761). The 890M's
    /// settled zoom frame of the worst page builds in 53–66 ms, so its pass is predicted
    /// inside the budget and runs.
    #[test]
    fn the_sharp_pass_runs_where_the_frame_it_doubles_was_cheap() {
        assert!(sharp_pass_affordable(Some(Duration::from_millis(66))));
    }

    /// The Windows Intel UHD's first frame of the same page built in 431.6 ms, and its
    /// unbounded sharp pass measured 8 867.6 ms — the pass is declined there.
    #[test]
    fn the_sharp_pass_is_declined_where_the_frame_itself_took_hundreds_of_milliseconds() {
        assert!(!sharp_pass_affordable(Some(Duration::from_micros(431_600))));
    }

    /// No built frame means no prediction, and a pass nobody asked for is not started on
    /// no evidence.
    #[test]
    fn the_sharp_pass_waits_for_a_built_frame_to_predict_from() {
        assert!(!sharp_pass_affordable(None));
    }

    /// A page of `page`'s number in document 1, placed at `magnification` and shifted right by
    /// `pan` device pixels — the two things a view change does to one page.
    fn placed(page: usize, magnification: f32, pan: f32) -> Vec<crate::stale::Placed> {
        vec![crate::stale::Placed {
            of: crate::stale::Picture::new(viewer_core::DocumentId(1), page, 0),
            list: Arc::new(DisplayList::new(pdf_render::Size::new(595.0, 842.0))),
            target: TargetSpec {
                width: 800,
                height: 1000,
                transform: Transform::scale(magnification, -magnification)
                    .then(Transform::translate(pan, 842.0 * magnification)),
            },
            depicts: crate::stale::Depicts::Page,
        }]
    }

    /// **A refusal is remembered by its pages and its magnification, and not by where the page
    /// sits** (ADR 1821): a pan of a refused page is the same question, a notch of zoom or another
    /// page is a different one.
    #[test]
    fn a_refusal_is_remembered_by_its_pages_and_magnification() {
        let mut refusals = Refusals::default();
        refusals.remember(&placed(0, 1.0, 0.0), "frame refused: too much");
        assert_eq!(
            refusals.said(&placed(0, 1.0, 120.0)),
            Some("frame refused: too much"),
            "a pan of the refused page is not asked again"
        );
        assert_eq!(
            refusals.said(&placed(0, 1.25, 0.0)),
            None,
            "a notch of zoom is asked"
        );
        assert_eq!(
            refusals.said(&placed(1, 1.0, 0.0)),
            None,
            "another page is asked"
        );
        assert_eq!(refusals.said(&[]), None, "no page is never refused");
    }

    /// The memory is bounded, newest first, and the same refusal learned twice is one entry.
    #[test]
    fn the_memory_of_refusals_keeps_the_newest() {
        let mut refusals = Refusals::default();
        for step in 0..=Refusals::KEPT {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a step count far below f32's exact integer range"
            )]
            let magnification = 1.0 + step as f32 / 8.0;
            refusals.remember(&placed(0, magnification, 0.0), "refused");
            refusals.remember(&placed(0, magnification, 0.0), "refused");
        }
        assert_eq!(refusals.held.len(), Refusals::KEPT);
        assert_eq!(
            refusals.said(&placed(0, 1.0, 0.0)),
            None,
            "the oldest aged out"
        );
        assert!(refusals.said(&placed(0, 1.125, 0.0)).is_some());
    }

    /// The owner's drawing, before anything has been measured: the seeds put it on the processor,
    /// at the figures they were measured as (ADRs 1831, 1860).
    #[test]
    fn a_page_of_millions_of_commands_goes_to_the_processor_on_the_seeds() {
        let pictures = [crate::stale::Picture::new(viewer_core::DocumentId(1), 0, 0)];
        let chose = Costs::default()
            .predict(&pictures, 4_408_405)
            .expect("1.1 s on the device against 0.5 s on the processor");
        assert!(!chose.device_measured && !chose.processor_measured);
        assert_eq!(chose.device.as_millis(), 1_115);
        assert_eq!(chose.processor.as_millis(), 529);
    }

    /// A frame the device is predicted to draw inside `WARN` stays there, whatever the processor
    /// is predicted to take: nobody waits for it, and the device is where page one goes.
    #[test]
    fn a_frame_nobody_waits_for_stays_on_the_device() {
        let pictures = [crate::stale::Picture::new(viewer_core::DocumentId(1), 0, 0)];
        assert_eq!(Costs::default().predict(&pictures, 3_900_000), None);
        assert!(Costs::default().predict(&pictures, 4_000_000).is_some());
    }

    /// What a lane measured on these pages replaces its seed, and only on these pages.
    #[test]
    fn a_measured_price_replaces_the_seed_for_its_own_pages() {
        let page = |page| {
            vec![crate::stale::Picture::new(
                viewer_core::DocumentId(1),
                page,
                0,
            )]
        };
        let mut costs = Costs::default();
        // Large fills: the processor took twenty seconds over the page, so the device draws it.
        costs.measured(page(0), 4_408_405, Lane::Processor, Duration::from_secs(20));
        assert_eq!(costs.predict(&page(0), 4_408_405), None);
        assert!(
            costs.predict(&page(1), 4_408_405).is_some(),
            "another page is still priced by the seeds"
        );
        // A device that drew it in half a second keeps it, measured.
        costs.measured(page(2), 4_408_405, Lane::Device, Duration::from_millis(500));
        assert_eq!(costs.predict(&page(2), 4_408_405), None);
        // The processor measured faster than the device measured: the processor, and it says so.
        costs.measured(page(2), 4_408_405, Lane::Device, Duration::from_secs(8));
        costs.measured(page(2), 4_408_405, Lane::Processor, Duration::from_secs(2));
        let chose = costs.predict(&page(2), 4_408_405).expect("2 s against 8 s");
        assert!(chose.device_measured && chose.processor_measured);
        assert_eq!(costs.held.len(), 2, "one entry an arrangement");
    }

    /// A page the seeds gave the processor is asked of the device once, after the processor has
    /// drawn it, so a device faster than its seed is found; and a device that refused the page has
    /// answered, so the seeds decide again rather than asking at every magnification (ADR 1860).
    #[test]
    fn the_device_is_asked_once_a_page_the_seeds_gave_the_processor() {
        let pictures = vec![crate::stale::Picture::new(viewer_core::DocumentId(1), 0, 0)];
        let mut costs = Costs::default();
        assert!(
            costs.predict(&pictures, 4_408_405).is_some(),
            "the seeds give the processor the first frame"
        );
        costs.measured(
            pictures.clone(),
            4_408_405,
            Lane::Processor,
            Duration::from_millis(900),
        );
        assert_eq!(
            costs.predict(&pictures, 4_408_405),
            None,
            "the next frame is the device's, its one ask"
        );
        costs.measured(
            pictures.clone(),
            4_408_405,
            Lane::Device,
            Duration::from_millis(1_500),
        );
        let chose = costs
            .predict(&pictures, 4_408_405)
            .expect("measured, 0.9 s against 1.5 s");
        assert!(chose.device_measured && chose.processor_measured);

        let refusing = vec![crate::stale::Picture::new(viewer_core::DocumentId(1), 1, 0)];
        costs.measured(
            refusing.clone(),
            4_408_405,
            Lane::Processor,
            Duration::from_millis(900),
        );
        costs.refused_pictures(refusing.clone());
        let chose = costs
            .predict(&refusing, 4_408_405)
            .expect("a refusal is the device's answer; the seed decides again");
        assert!(!chose.device_measured && chose.processor_measured);
    }

    /// Only a refusal that is a fact about the frame is kept: the budget is, a lost device is not.
    #[test]
    fn only_a_refusal_about_the_frame_is_remembered() {
        assert!(remembered(&QuorraRasterError::Render(
            raster_gpu::RenderError::FrameBudgetExceeded {
                needed: 423_206_976,
                budget: 268_435_456,
            }
        )));
        assert!(!remembered(&QuorraRasterError::Render(
            raster_gpu::RenderError::DeviceLost {
                detail: "gone".to_owned(),
            }
        )));
        assert!(!remembered(&QuorraRasterError::Allocation {
            width: 1,
            height: 1,
        }));
    }
}

/// What a zoom gesture's frames are, measured on the real render thread and the real adapter.
///
/// **A measurement rather than a gate**, and ignored for the reason every measurement on a real
/// device is: it needs one, and it prints durations rather than asserting them. It drives the
/// render thread this window uses — [`spawn`], the sharp pass and the retained pages included —
/// from a clock that ticks at the owner's 120 Hz, asks [`crate::stale::Stale::plan`] the question
/// `crate::surface` asks at every tick, and counts what each tick would have put up: a rendering
/// of the view asked for, a stand-in, or the picture already there held for one more refresh.
///
/// ```sh
/// cargo test --release -p viewer-ui --bin quorra -- --ignored --nocapture a_zoom_gesture
/// GESTURE_PDF=tmp/Entwurf.pdf GESTURE_EVERY=12 cargo test … a_zoom_gesture
/// ```
///
/// `GESTURE_EVERY` is how many refreshes lie between two wheel notches (4 by default, which is a
/// hand turning a wheel briskly), `GESTURE_SUPERSAMPLE` is `--supersample`'s factor.
#[cfg(test)]
mod gesture {
    use super::*;
    use crate::stale::{Placed, Plan, Stale, Standing};

    /// What one tick put up, in the words the frame line uses.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Put {
        Fresh,
        StoodIn,
        Held,
        Refused,
    }

    fn variable(name: &str) -> Option<String> {
        std::env::var(name).ok().filter(|value| !value.is_empty())
    }

    /// The arrangement `crate::surface`'s `arrangement` builds, for a window with no panel.
    fn arrangement(
        viewer: &viewer_core::Viewer,
        requests: &mut Vec<viewer_core::RenderRequest>,
        (width, height): (u32, u32),
    ) -> Vec<Placed> {
        let mut placed = Vec::new();
        for request in requests.iter() {
            let viewer_core::Answer::Geometry(geometry) =
                viewer.query(viewer_core::Query::PageGeometry(request.page))
            else {
                continue;
            };
            placed.push(Placed {
                of: crate::stale::Picture::new(request.document, request.page, request.ink),
                list: Arc::clone(&request.list),
                target: TargetSpec {
                    width,
                    height,
                    transform: request
                        .target
                        .transform
                        .then(Transform::translate(geometry.origin.0, geometry.origin.1)),
                },
                depicts: crate::stale::Depicts::Page,
            });
        }
        requests.retain(|request| placed.iter().any(|one| one.of.page() == request.page));
        placed
    }

    fn fold(
        events: impl Iterator<Item = viewer_core::Event>,
        into: &mut Vec<viewer_core::RenderRequest>,
    ) {
        for event in events {
            if let viewer_core::Event::NeedsRender(request) = event {
                into.retain(|held| held.page != request.page);
                into.push(request);
            }
        }
    }

    /// The committed document every headless test opens, laid out in a window of `size`.
    fn opened(size: (u32, u32)) -> (viewer_core::Viewer, Vec<viewer_core::RenderRequest>) {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf");
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|error| panic!("{} is committed: {error}", path.display()));
        let mut viewer = viewer_core::Viewer::new(size.0, size.1, 1.0);
        let mut requests = Vec::new();
        fold(
            viewer.handle(viewer_core::Command::Open {
                id: viewer_core::DocumentId(1),
                bytes: bytes.into(),
                password: None,
                fragment: None,
            }),
            &mut requests,
        );
        (viewer, requests)
    }

    /// What the render thread does between jobs, for a test: no flag left anywhere it is read.
    fn idle(proxy_pages: usize, supersample: u32) -> Idle {
        Idle {
            proxy_pages,
            supersample,
            coverage: crate::arguments::CoverageChoice::Auto,
            interrupt: Arc::default(),
        }
    }

    fn job(pages: &[Placed], size: (u32, u32)) -> Job {
        Job {
            width: size.0,
            height: size.1,
            pages: pages.to_vec(),
            overlays: Vec::new(),
            coverage: raster_gpu::Coverage::Compute,
            reuse: None,
            known_refusal: None,
            cheaper: None,
            interrupt: Interrupt::new(),
            on_the_processor: Arc::new(AtomicBool::new(false)),
            ahead: None,
            look: None,
        }
    }

    /// **A zoom step is drawn before the sharp pass of the view it replaced, and the sharp pass
    /// still comes once the view is still** (ADR 1289).
    ///
    /// The owner's report was a zoom gesture that showed a stand-in throughout on a device quick
    /// enough to draw every step: each step's render waited behind the previous view's sharp
    /// pass, which the render thread began the moment it was idle and which nothing can take
    /// back. The shape asserted is the order of what comes back, never a duration: a view change
    /// sent the instant the first frame lands must be answered by its own frame, and the
    /// sharpened picture that follows must be of the *new* view.
    #[test]
    fn a_zoom_step_is_drawn_before_the_sharp_pass_of_the_view_it_replaced() {
        let Ok(renderer) = QuorraWindowRenderer::new_headless() else {
            println!("skipped: no adapter on this machine");
            return;
        };
        let size = (800_u32, 1000_u32);
        let (mut viewer, mut requests) = opened(size);
        let first = arrangement(&viewer, &mut requests, size);
        fold(
            viewer.handle(viewer_core::Command::Zoom {
                zoom: viewer_core::Zoom::Out,
                at: None,
            }),
            &mut requests,
        );
        let second = arrangement(&viewer, &mut requests, size);
        assert!(
            !first.is_empty() && !second.is_empty(),
            "the document shows a page"
        );

        let link = spawn(renderer, idle(0, 2), None);
        let patience = Duration::from_mins(2);
        let next = || {
            link.done
                .recv_timeout(patience)
                .expect("the render thread answers")
        };
        link.jobs
            .send(job(&first, size))
            .expect("the thread is running");
        let Finished::Frame(landed) = next() else {
            panic!("the first answer is the frame asked for");
        };
        assert!(landed.refused.is_none(), "{:?}", landed.refused);
        // The step, sent the moment the first frame is in hand — the tightest a gesture can be.
        link.jobs
            .send(job(&second, size))
            .expect("the thread is running");
        match next() {
            Finished::Frame(done) => assert!(
                same_pages(&done.pages, &second),
                "the frame that answered is of the view asked for"
            ),
            Finished::Sharp(_) => panic!(
                "the previous view's sharp pass was drawn in front of the zoom step that \
                 replaced it"
            ),
            Finished::Page(_) => panic!("no retained page was asked for"),
            Finished::Partial(_) | Finished::Read(_) => {
                panic!("a device frame is not drawn on the processor")
            }
        }
        let Finished::Sharp(sharp) = next() else {
            panic!("a still view is sharpened");
        };
        assert!(
            same_pages(&sharp.pages, &second),
            "the sharp pass is of the view that stayed"
        );
    }

    /// **A frame the device refused before is drawn on the processor without asking it, and is not
    /// sharpened by it** (ADR 1821).
    ///
    /// The job carries the refusal the window remembered; what comes back says the device was not
    /// asked, the processor drew, the pixels landed — and nothing follows, because the 2× pass is
    /// the same view at four times the pixels and would only hear the same answer.
    #[test]
    fn a_frame_the_device_refused_before_is_drawn_on_the_processor_without_asking_it() {
        let Ok(renderer) = QuorraWindowRenderer::new_headless() else {
            println!("skipped: no adapter on this machine");
            return;
        };
        let size = (400_u32, 500_u32);
        let (viewer, mut requests) = opened(size);
        let pages = arrangement(&viewer, &mut requests, size);
        assert!(!pages.is_empty(), "the document shows a page");
        let link = spawn(renderer, idle(0, 2), None);
        let mut asked = job(&pages, size);
        asked.known_refusal = Some("frame refused: remembered".to_owned());
        link.jobs.send(asked).expect("the thread is running");
        let Ok(Finished::Frame(done)) = link.done.recv_timeout(Duration::from_mins(2)) else {
            panic!("the first answer is the frame asked for");
        };
        assert_eq!(done.fell_back.as_deref(), Some("frame refused: remembered"));
        assert!(done.device_tried.is_none(), "the device was not asked");
        assert!(done.processor_drew.is_some(), "the processor drew it");
        assert!(
            !done.abandoned && done.refused.is_none(),
            "{:?}",
            done.refused
        );
        assert!(!done.refused_now, "nothing new was learned");
        match link.done.recv_timeout(Duration::from_secs(3)) {
            Err(_) => {}
            Ok(Finished::Sharp(_)) => panic!("a view the device refused was sharpened by it"),
            Ok(_) => panic!("nothing else was asked for"),
        }
    }

    /// **An interrupt raised before the processor starts abandons the frame**, and nothing is drawn
    /// for it (ADR 1821) — the shape a notch of zoom takes while the device is still being asked.
    #[test]
    fn an_interrupt_raised_before_the_processor_starts_abandons_the_frame() {
        let Ok(renderer) = QuorraWindowRenderer::new_headless() else {
            println!("skipped: no adapter on this machine");
            return;
        };
        let size = (400_u32, 500_u32);
        let (viewer, mut requests) = opened(size);
        let pages = arrangement(&viewer, &mut requests, size);
        let link = spawn(renderer, idle(0, 1), None);
        let mut asked = job(&pages, size);
        asked.known_refusal = Some("frame refused: remembered".to_owned());
        asked.interrupt.raise();
        link.jobs.send(asked).expect("the thread is running");
        let Ok(Finished::Frame(done)) = link.done.recv_timeout(Duration::from_mins(2)) else {
            panic!("the first answer is the frame asked for");
        };
        assert!(done.abandoned, "the frame was taken back");
        assert!(
            done.processor_drew.is_none(),
            "the processor was not started"
        );
    }

    /// **A frame that waited behind other work is charged only its own drawing** (ADR 1289).
    ///
    /// Rule 5 predicts the next render from this one, and time spent queued behind a pass
    /// nobody asked for is not something the next render will cost. Two jobs sent together are
    /// drawn one after the other, so the second waits for the whole of the first — and the span
    /// [`Landed::drew`] is measured over has to begin after the first has finished.
    #[test]
    fn a_frame_that_waited_behind_another_is_charged_only_its_own_drawing() {
        let Ok(renderer) = QuorraWindowRenderer::new_headless() else {
            println!("skipped: no adapter on this machine");
            return;
        };
        let size = (800_u32, 1000_u32);
        let (mut viewer, mut requests) = opened(size);
        let first = arrangement(&viewer, &mut requests, size);
        fold(
            viewer.handle(viewer_core::Command::Zoom {
                zoom: viewer_core::Zoom::Out,
                at: None,
            }),
            &mut requests,
        );
        let second = arrangement(&viewer, &mut requests, size);
        let link = spawn(renderer, idle(0, 1), None);
        let asked = Instant::now();
        link.jobs
            .send(job(&first, size))
            .expect("the thread is running");
        link.jobs
            .send(job(&second, size))
            .expect("the thread is running");
        let mut frames = Vec::new();
        while frames.len() < 2 {
            if let Finished::Frame(done) = link
                .done
                .recv_timeout(Duration::from_mins(2))
                .expect("the render thread answers")
            {
                frames.push(done);
            }
        }
        assert!(
            frames[1].started >= frames[0].finished,
            "the second began after the first"
        );
        assert!(
            frames[1]
                .finished
                .saturating_duration_since(frames[1].started)
                < frames[1].finished.saturating_duration_since(asked),
            "what the second is charged excludes the first it waited behind"
        );
    }

    #[test]
    #[ignore = "a measurement on the real adapter; prints what a gesture's refreshes put up"]
    #[expect(
        clippy::too_many_lines,
        reason = "one simulated event loop, read top to bottom as the tick it models"
    )]
    fn a_zoom_gesture_out_counts_what_each_refresh_put_up() {
        let Ok(renderer) = QuorraWindowRenderer::new_headless() else {
            eprintln!("no adapter: nothing to measure");
            return;
        };
        let adapter = renderer.adapter_description().to_owned();
        let path = variable("GESTURE_PDF").map_or_else(
            || {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../doc/PDF20_AN001-BPC.pdf")
            },
            |named| {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .join(named)
            },
        );
        let every: u32 = variable("GESTURE_EVERY")
            .and_then(|v| v.parse().ok())
            .unwrap_or(4);
        let notches: u32 = variable("GESTURE_NOTCHES")
            .and_then(|v| v.parse().ok())
            .unwrap_or(6);
        let supersample: u32 = variable("GESTURE_SUPERSAMPLE")
            .and_then(|v| v.parse().ok())
            .unwrap_or(2);
        let size = (1200_u32, 1500_u32);
        let period = Duration::from_micros(8_333);

        let bytes = std::fs::read(&path).expect("the document named is readable");
        let mut viewer = viewer_core::Viewer::new(size.0, size.1, 1.0);
        let mut requests = Vec::new();
        fold(
            viewer.handle(viewer_core::Command::Open {
                id: viewer_core::DocumentId(1),
                bytes: bytes.into(),
                password: None,
                fragment: None,
            }),
            &mut requests,
        );

        let link = spawn(renderer, idle(2, supersample), None);
        let mut stale = Stale::default();
        let mut proxies: crate::stale::Proxies<Arc<wgpu::Texture>> = crate::stale::Proxies::new(2);
        let mut shown: Option<Vec<Placed>> = None;
        let mut spare: Option<Pair> = None;
        let mut in_flight: Option<Instant> = None;
        let mut lane: Option<raster_gpu::Coverage> = None;
        let mut puts = Vec::new();
        let mut first_notch_at: Option<u32> = None;
        let settle_ticks = 90;
        let ticks = every * notches + settle_ticks;
        let began = Instant::now();
        for tick in 0..ticks {
            let due = began + period * tick;
            if let Some(wait) = due.checked_duration_since(Instant::now()) {
                std::thread::sleep(wait);
            }
            let now = Instant::now();
            let mut landed = String::new();
            while let Ok(finished) = link.done.try_recv() {
                match finished {
                    Finished::Frame(done) => {
                        // Exactly what `Window::collect` hands `crate::surface`'s `adopt`.
                        let waited = in_flight.take().map_or(Duration::ZERO, |asked| {
                            done.finished.saturating_duration_since(asked)
                        });
                        let drew = done.finished.saturating_duration_since(done.started);
                        let built = !matches!(
                            done.cost.encode_source,
                            Some(raster_gpu::EncodeSource::Replayed)
                        );
                        stale.settled(&done.pages, drew, built);
                        landed = format!(
                            "landed: waited {:.1} ms, drawn {:.1} ms",
                            waited.as_secs_f64() * 1e3,
                            done.cost.total.as_secs_f64() * 1e3
                        );
                        spare = Some(done.textures);
                        shown = Some(done.pages);
                    }
                    Finished::Page(page) => proxies.keep(page),
                    Finished::Sharp(sharp) => {
                        landed = format!("{landed} sharp {:.1} ms", sharp.took.as_secs_f64() * 1e3);
                    }
                    // A look at a frame the processor is drawing changes nothing this test
                    // counts: what it reads is when each true frame landed.
                    Finished::Partial(_) | Finished::Read(_) => {}
                }
            }
            // The gesture: a notch every `every` refreshes, after a still start that lets the
            // launch frame, its sharp pass and the retained pages land.
            let warm = 60;
            let notch =
                tick >= warm && (tick - warm) % every == 0 && (tick - warm) / every < notches;
            if notch {
                first_notch_at.get_or_insert(tick);
                fold(
                    viewer.handle(viewer_core::Command::Zoom {
                        zoom: viewer_core::Zoom::Out,
                        at: None,
                    }),
                    &mut requests,
                );
            }
            let pages = arrangement(&viewer, &mut requests, size);
            let was_out = in_flight.map(|asked| asked.elapsed());
            let of_this_view = shown.as_ref().is_some_and(|drawn| {
                drawn.len() == pages.len()
                    && drawn.iter().zip(&pages).all(|(drawn, asked)| {
                        Arc::ptr_eq(&drawn.list, &asked.list) && drawn.target == asked.target
                    })
            });
            if !of_this_view && in_flight.is_none() && !pages.is_empty() {
                let coverage = crate::surface::lane_for(
                    pages[0].target.transform,
                    shown
                        .as_ref()
                        .and_then(|drawn| Some(drawn.first()?.target.transform)),
                    lane,
                    &[],
                    false,
                    crate::arguments::CoverageChoice::Auto,
                );
                lane = Some(coverage);
                let job = Job {
                    width: size.0,
                    height: size.1,
                    pages: pages.clone(),
                    overlays: Vec::new(),
                    coverage,
                    reuse: spare.take(),
                    known_refusal: None,
                    cheaper: None,
                    interrupt: Interrupt::new(),
                    on_the_processor: Arc::new(AtomicBool::new(false)),
                    ahead: None,
                    look: None,
                };
                if link.jobs.send(job).is_ok() {
                    in_flight = Some(now);
                }
            }
            if !stale.has_rendering() || first_notch_at.is_none() {
                continue;
            }
            let under = proxies.placements(&pages).len();
            let put = match stale.plan(&pages, under, period, was_out, Standing::Quads) {
                Plan::Render => Put::Fresh,
                Plan::Approximate(_) => Put::StoodIn,
                Plan::Refused(crate::stale::Refusal::InsideTheRefresh { .. }) => Put::Held,
                Plan::Refused(_) => Put::Refused,
            };
            puts.push((tick, notch, put));
            if notch || put != Put::Fresh || !landed.is_empty() {
                println!(
                    "tick {tick:>3}{} {put:?} {landed}",
                    if notch { " notch" } else { "      " }
                );
            }
        }
        let count = |what: Put| puts.iter().filter(|(.., put)| *put == what).count();
        println!(
            "{adapter}; {notches} notches every {every} refresh(es) at 120 Hz, supersample \
             {supersample}"
        );
        println!(
            "refreshes from the first notch on: {} a rendering of the view asked for, {} stood in, \
             {} held the picture already up, {} refused",
            count(Put::Fresh),
            count(Put::StoodIn),
            count(Put::Held),
            count(Put::Refused)
        );
        // How long after each notch the first rendering of *its* view was put up.
        let mut after = Vec::new();
        let notch_ticks: Vec<u32> = puts
            .iter()
            .filter(|(_, notch, _)| *notch)
            .map(|(t, ..)| *t)
            .collect();
        for (at, next) in notch_ticks
            .iter()
            .zip(notch_ticks.iter().skip(1).map(Some).chain([None]))
        {
            let fresh = puts
                .iter()
                .find(|(t, _, put)| t >= at && next.is_none_or(|n| t < n) && *put == Put::Fresh);
            after.push(fresh.map(|(t, ..)| t - at));
        }
        println!(
            "refreshes from each notch to a rendering of its view (None: never before the next notch): {after:?}"
        );
    }
}
