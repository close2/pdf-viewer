//! The graphics device behind the confined window, and the thread that keeps it off the loop.
//!
//! **This is what the marks crossed the pipe for** (ADRs 0607, 0725): the confined process
//! cannot hold a device — its first `ioctl` under the confinement is a kill — so a page's
//! display list crosses to this side and the device here draws it. Until this module existed
//! the window presented through the processor, and the tier change's whole payload arm ended in
//! a CPU rasteriser; now a page shipped as marks is drawn by `render-quorra` into textures this
//! module owns, and the pixels never cross back to the CPU.
//!
//! # The arrangement, and what it is a smaller copy of
//!
//! The flagship's `renderer` module split one device into a presenter the event thread keeps
//! and a renderer a thread of its own drives (quorra's ADR 0056), because encoding a display
//! list is CPU work proportional to the list and the only path to the screen must not wait on
//! it. That argument is *sharper* here, not weaker: the lists this window draws are a hostile
//! document's, decoded from a pipe. So the same split, without the parts this window has no use
//! for — no retained page proxies, no supersampled sharpening pass, no cadence clock. Each of
//! those exists to serve the flagship's reprojection machinery, and this window reprojects
//! nothing: it shows the newest finished frame and asks for another when the state changes
//! (`doc/todo/37`'s show-what-it-had, with nothing reprojected).
//!
//! One job is in flight at a time and a newer ask replaces the waiting one, for the flagship's
//! reason: a queue would fill with answers to views the reader has already left. While a job is
//! in flight the host polls [`Device::collect`] at [`viewer_host::drawing::POLL`], the same
//! interval the fallback thread is polled at.
//!
//! # What a refusal does
//!
//! The render thread does **not** fall back to the processor itself, and that is a deliberate
//! departure from the flagship's `draw`: the interruptible drawing thread already exists in
//! this window (ADR 0650), and a hostile page must be drawn where an interrupt can reach it —
//! not on this thread, which nothing can take back short of exit. A refused frame comes home as
//! [`Landed::refused`] and the host hands the marks to that thread ([`super::screen`]'s
//! `fall_back`), out loud.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use pdf_render::{DisplayList, TargetSpec, Transform};
use quorra_gpu::wgpu;
use render_quorra::{FrameCost, PresentFrame, QuorraWindowRenderer, WindowTextures};

/// The two window-sized textures one frame is drawn into, travelling as one thing.
///
/// They travel rather than being reallocated (the flagship's pool, same argument): a window's
/// page texture is megabytes, and the event thread hands back whichever pair it has stopped
/// showing with the next job.
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
            page: renderer.layer_texture("the confined page", width, height),
            chrome: renderer.layer_texture("the confined chrome", width, height),
            width,
            height,
        }
    }

    /// Whether this pair can be drawn into for a window of that size.
    ///
    /// `Target::Texture` requires a target sized exactly to the viewport, so a pair from before
    /// a resize is dropped rather than reused — one allocation per resize, none per frame.
    fn fits(&self, width: u32, height: u32) -> bool {
        self.width == width && self.height == height
    }
}

/// One window frame the event thread asks the render thread to draw.
///
/// Everything in it is owned, because it crosses a thread: each page by the `Arc` whose address
/// is its identity for the retained scene (`render_quorra::PresentFrame::pages`), the chrome by
/// value because the host rebuilds it from its own state anyway.
#[derive(Debug)]
struct Job {
    width: u32,
    height: u32,
    /// Every page of the arrangement the device draws, placed into the window —
    /// [`super::screen::Screen::device_pages`]'s answer.
    pages: Vec<(Arc<DisplayList>, TargetSpec)>,
    /// The chrome, in window pixels: §7.6.4.1's card is the one this window has.
    overlays: Vec<DisplayList>,
    /// The pair the event thread has finished with, for this frame to draw into.
    reuse: Option<Pair>,
}

/// One finished frame — or one refusal — on its way back to the thread that presents.
#[derive(Debug)]
struct Done {
    textures: Pair,
    cost: FrameCost,
    /// The device's refusal, where nothing was drawn and the textures carry no frame.
    refused: Option<String>,
    /// When the render thread finished, taken there rather than at collection — a finished
    /// frame sits in the channel until the next poll reads it, and measuring to that moment
    /// would charge the wait to the frame (the flagship's `Done::finished`, same argument).
    finished: Instant,
}

/// What one collected frame tells the host about itself.
#[derive(Debug)]
pub(crate) struct Landed {
    /// The device's refusal — `None` for a frame that was drawn and adopted. The host's answer
    /// to `Some` is [`super::screen::Screen::fall_back`], and saying so.
    pub(crate) refused: Option<String>,
    /// What the frame cost on the render thread, in the parts quorra measures it in.
    pub(crate) cost: FrameCost,
    /// From the ask to the render thread finishing it.
    pub(crate) waited: Duration,
}

/// The channels to a running render thread, and the thread itself.
#[derive(Debug)]
struct Link {
    jobs: Sender<Job>,
    done: Receiver<Done>,
    /// Joined on the way out, so the device is shut down rather than abandoned.
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Link {
    /// Ends the thread by taking its work away, then waits for it.
    ///
    /// Dropping the sender is the signal (no stop flag): `recv` on a channel with no senders
    /// returns an error, the loop ends, and the device is dropped on the thread that owns it.
    /// The join waits for at most the frame being drawn — whose encode is bounded by the wire's
    /// own message budget, since everything in it crossed the pipe. A join that panicked is
    /// ignored deliberately: this runs while the program is exiting, and a second panic would
    /// replace whatever the first one said.
    fn drop(&mut self) {
        let (jobs, _) = channel();
        drop(std::mem::replace(&mut self.jobs, jobs));
        if let Some(thread) = self.thread.take() {
            drop(thread.join());
        }
    }
}

/// One ask waiting for the render thread to come free: the pages and the chrome, newest only.
#[derive(Debug)]
struct Pending {
    pages: Vec<(Arc<DisplayList>, TargetSpec)>,
    overlays: Vec<DisplayList>,
}

/// What a present attempt came to, for the event loop to act on.
///
/// The three non-terminal arms are the flagship's `Swapchain` reading of the same states
/// (`viewer-ui`'s `surface` module carries the argument for each): a swapchain state is an
/// event rather than a failure, and which one decides what the next tick does.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Presented {
    /// The frame on hand reached the window.
    Shown,
    /// There is nothing to show yet — before the first frame lands, the grounded medium stays.
    Nothing,
    /// The swapchain has been replaced under the present; ask for a redraw at once and the next
    /// acquire finds a fresh one.
    AskAgain,
    /// Occluded, or every swapchain image still in flight: the next redraw is soon enough.
    Waited,
    /// A refusal trying again does not clear, in the device's words.
    Refused(String),
}

/// This window's half of the arrangement: the surface, and a handle on the thread that draws.
///
/// Lives on the event thread and never leaves it. What it owns of quorra is a
/// [`quorra_gpu::Presenter`] — the surface, its swapchain, one pipeline — which no `&mut Device`
/// stands in front of.
pub(crate) struct Device {
    presenter: quorra_gpu::Presenter,
    /// One opaque texel of the window's surround, scaled over the window under every frame.
    medium: wgpu::Texture,
    /// The renderer, until the first job moves it to a thread of its own.
    ///
    /// `CLAUDE.md`'s startup rule as a field, exactly as the flagship keeps it: a thread spawned
    /// while the window comes up would put a scheduler decision in front of a launch milestone;
    /// one spawned by the first job costs the launch nothing and the first frame one spawn.
    idle: Option<QuorraWindowRenderer>,
    thread: Option<Link>,
    /// The newest finished frame, which is what every present puts up.
    shown: Option<Pair>,
    /// The pair the last adopted frame displaced, going back with the next job.
    spare: Option<Pair>,
    /// When the frame now being drawn was asked for, or `None` while the thread is idle.
    in_flight: Option<Instant>,
    /// The ask that arrived while one was in flight — the newest only, for the reason the
    /// module comment gives: a queue would be a queue of views the reader has left.
    wanted: Option<Pending>,
    /// The window's size in device pixels, as the presenter was last told it.
    size: (u32, u32),
    /// The adapter quorra selected, read before the device left this thread.
    description: String,
    /// What bringing the device up cost — `pipeline_compilation` is `None` for ever here,
    /// because nothing on the launch path waits for warmth.
    startup: quorra_gpu::StartupTimings,
    /// What the device has said that no call returned, taken once per present.
    uncaptured: Arc<render_quorra::UncapturedErrors>,
}

impl std::fmt::Debug for Device {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Device")
            .field("size", &self.size)
            .field("drawing", &self.in_flight.is_some())
            .field("shown", &self.shown.is_some())
            .finish_non_exhaustive()
    }
}

/// A presenter whose surface has never been configured — the only way to reach a [`Device`].
///
/// The type exists for the abort the flagship's `Ungrounded` documents in full: wgpu answers an
/// acquire that finds an unconfigured surface with a **panic** — the fatal branch is reachable
/// on the first configure of a process and never again — and `Surface::configure` can fail
/// exactly when another thread submits beside it. [`Device::ask`] is what spawns the render
/// thread, and a `Device` cannot exist before [`Ungrounded::ground`] has configured the surface
/// with the queue provably empty; nothing is left to race.
pub(crate) struct Ungrounded(Device);

impl Ungrounded {
    /// Puts the window's own surround on the surface, which is what configures it.
    ///
    /// Not a probe frame: nothing waits for warmth and no page is drawn. It is the window's
    /// first honest picture — the surround a page will be placed on — at the one moment the
    /// device's queue is provably empty (the flagship's `ground`, whose comment carries the
    /// whole argument).
    ///
    /// # Errors
    ///
    /// The presenter's refusal, or the device's words where it refused nothing — the sentence a
    /// caller reports before drawing on the processor instead.
    pub(crate) fn ground(mut self) -> Result<Device, String> {
        let (width, height) = self.0.size;
        #[expect(
            clippy::cast_precision_loss,
            reason = "window dimensions are far below f32's exact integer range"
        )]
        let extent = Transform::scale(width as f32, height as f32);
        let layers = [quorra_gpu::Layer {
            texture: &self.0.medium,
            placement: affine(extent),
            filter: quorra_scene::ImageFilter::Nearest,
        }];
        let refused = self.0.presenter.present(&layers).err();
        // Taken whether the present refused or not: `Surface::configure` returns `()`, so a
        // configure that failed under a present that then succeeded would be silent — and this
        // is the one call here that knows a failure is not survivable.
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

impl Device {
    /// Splits a freshly built renderer into a presenter this thread keeps and a renderer a
    /// thread will take, or gives the renderer back where it has no surface to detach.
    ///
    /// What comes back is an [`Ungrounded`], and that type says why. The renderer comes back
    /// boxed on the failing path because a `Result` is as wide as its widest arm.
    pub(crate) fn split(
        mut renderer: QuorraWindowRenderer,
        size: (u32, u32),
    ) -> Result<Ungrounded, Box<QuorraWindowRenderer>> {
        let description = renderer.adapter_description().to_owned();
        let startup = renderer.startup();
        let medium = renderer.medium_texture();
        let uncaptured = renderer.uncaptured();
        let Some(mut presenter) = renderer.detach_presenter() else {
            return Err(Box::new(renderer));
        };
        presenter.resize(size.0, size.1);
        Ok(Ungrounded(Self {
            presenter,
            medium,
            idle: Some(renderer),
            thread: None,
            shown: None,
            spare: None,
            in_flight: None,
            wanted: None,
            size,
            description,
            startup,
            uncaptured,
        }))
    }

    /// The adapter quorra selected, for reports.
    pub(crate) fn description(&self) -> &str {
        &self.description
    }

    /// What bringing the device up cost, in the parts quorra measures it in.
    pub(crate) fn startup(&self) -> quorra_gpu::StartupTimings {
        self.startup
    }

    /// Tells the presenter how big the window is now.
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.size = (width, height);
        self.presenter.resize(width, height);
    }

    /// Whether a frame is being drawn or waiting to be — what decides the event loop's poll.
    pub(crate) fn busy(&self) -> bool {
        self.in_flight.is_some() || self.wanted.is_some()
    }

    /// Asks for a frame of these pages under this chrome, newest ask winning.
    ///
    /// While a job is in flight the ask waits — replaced by any newer one — and goes out when
    /// the frame lands ([`Self::collect`]); the caller polls at
    /// [`viewer_host::drawing::POLL`] meanwhile, which [`Self::busy`] says to do.
    pub(crate) fn ask(
        &mut self,
        pages: Vec<(Arc<DisplayList>, TargetSpec)>,
        overlays: Vec<DisplayList>,
    ) {
        self.wanted = Some(Pending { pages, overlays });
        self.pump();
    }

    /// Sends the waiting ask if the thread is free, spawning the thread for the first.
    fn pump(&mut self) {
        if self.in_flight.is_some() {
            return;
        }
        let (width, height) = self.size;
        if width == 0 || height == 0 {
            return; // minimised: nothing to draw into
        }
        let Some(Pending { pages, overlays }) = self.wanted.take() else {
            return;
        };
        if self.thread.is_none() {
            // The first job is what starts the thread — `CLAUDE.md`'s startup rule.
            let Some(renderer) = self.idle.take() else {
                return; // the thread ended and its device went with it; nothing can restart it
            };
            self.thread = Some(spawn(renderer));
        }
        let Some(link) = self.thread.as_ref() else {
            return;
        };
        let job = Job {
            width,
            height,
            pages,
            overlays,
            reuse: self.spare.take(),
        };
        // A send that fails is a render thread that has ended, which is a device that has gone;
        // the window keeps presenting what it has.
        if link.jobs.send(job).is_ok() {
            self.in_flight = Some(Instant::now());
        }
    }

    /// Takes the finished frame if one has landed, adopting it for the next present.
    ///
    /// A refused frame is not adopted — its textures carry no picture — and comes back with the
    /// device's words for the caller to fall back on. Either way the waiting ask, if any, goes
    /// out now.
    pub(crate) fn collect(&mut self) -> Option<Landed> {
        let done = {
            let link = self.thread.as_ref()?;
            let mut newest = None;
            // At most one, because one job is in flight at a time; drained defensively anyway.
            while let Ok(done) = link.done.try_recv() {
                newest = Some(done);
            }
            newest?
        };
        Some(self.adopt(done))
    }

    /// Waits for the first frame, out of the launch's one-refresh budget (ADR 0678).
    ///
    /// The same rule and the same number as the fallback thread's settle: a window with nothing
    /// on the screen yet waits for page one rather than presenting a surround and replacing it
    /// a refresh later. Nothing here waits for pipeline warmth — what is waited for is a frame
    /// already asked for, and only while the budget lasts.
    pub(crate) fn settle(&mut self, budget: Duration) -> Option<Landed> {
        self.in_flight?;
        let done = {
            let link = self.thread.as_ref()?;
            link.done.recv_timeout(budget).ok()?
        };
        Some(self.adopt(done))
    }

    /// One landed frame, adopted or declined, and the waiting ask sent after it.
    fn adopt(&mut self, done: Done) -> Landed {
        let waited = self.in_flight.take().map_or(Duration::ZERO, |began| {
            done.finished.saturating_duration_since(began)
        });
        let Done {
            textures,
            cost,
            refused,
            ..
        } = done;
        if refused.is_some() {
            // Nothing was drawn: the textures go back into the pool, the shown frame stays.
            self.spare = Some(textures);
        } else {
            self.spare = self.shown.replace(textures);
        }
        self.pump();
        Landed {
            refused,
            cost,
            waited,
        }
    }

    /// Puts the frame on hand on the window: the surround, the pages, the chrome.
    ///
    /// Three layers, each at the placement it was drawn for: the medium scaled over the window
    /// (what a resize reveals before the next frame lands), the page texture at the identity —
    /// it is opaque and window-sized, so wherever it covers the window it wins — and the chrome
    /// at the identity on transparency. Nearest filtering throughout: at the identity the
    /// sampler lands on texel centres and nothing is resampled.
    pub(crate) fn present(&mut self) -> Presented {
        let outcome = {
            let Some(shown) = self.shown.as_ref() else {
                return Presented::Nothing;
            };
            #[expect(
                clippy::cast_precision_loss,
                reason = "window dimensions are far below f32's exact integer range"
            )]
            let extent = Transform::scale(self.size.0 as f32, self.size.1 as f32);
            let layers = [
                quorra_gpu::Layer {
                    texture: &self.medium,
                    placement: affine(extent),
                    filter: quorra_scene::ImageFilter::Nearest,
                },
                quorra_gpu::Layer {
                    texture: &shown.page,
                    placement: quorra_scene::Affine::IDENTITY,
                    filter: quorra_scene::ImageFilter::Nearest,
                },
                quorra_gpu::Layer {
                    texture: &shown.chrome,
                    placement: quorra_scene::Affine::IDENTITY,
                    filter: quorra_scene::ImageFilter::Nearest,
                },
            ];
            self.presenter.present(&layers)
        };
        // The one place the device's uncaptured complaint is taken, present or refuse: a
        // `Surface::configure` failure reports only here (the flagship's `put_up`, same rule).
        let said = self.uncaptured.take();
        if let Some(said) = &said {
            eprintln!("the graphics device said: {}", said.last.trim());
        }
        match outcome {
            Ok(()) => Presented::Shown,
            Err(quorra_gpu::RenderError::SurfaceUnavailable { reason }) => {
                swapchain(reason, said.as_ref())
            }
            Err(problem) => Presented::Refused(problem.to_string()),
        }
    }
}

/// What a swapchain state means for the tick after it — the flagship's reading of the same
/// four states, told apart here so each is testable without a graphics device.
fn swapchain(
    reason: quorra_gpu::SurfaceProblem,
    said: Option<&render_quorra::Uncaptured>,
) -> Presented {
    match reason {
        // The window system moved under the swapchain — a resize, a compositor restart. quorra
        // has marked the surface for reconfiguration; the next acquire finds a fresh one.
        quorra_gpu::SurfaceProblem::Outdated | quorra_gpu::SurfaceProblem::Lost => {
            Presented::AskAgain
        }
        // Ordinary, and neither is this program's to fix: an occluded window is one nobody can
        // see, a timeout is a swapchain whose images are all still in flight.
        quorra_gpu::SurfaceProblem::Timeout | quorra_gpu::SurfaceProblem::Occluded => {
            Presented::Waited
        }
        // A validation failure that reaches a configured surface is very nearly always a
        // reconfigure that failed, and the only account of it is what the device told the
        // handler — carried where it exists.
        quorra_gpu::SurfaceProblem::Validation => Presented::Refused(match said {
            Some(said) => format!(
                "the window's swapchain could not be rebuilt, and the graphics device said: {}",
                said.last.trim()
            ),
            None => "the window's swapchain could not be rebuilt, and the graphics device left \
                     no account of why"
                .to_owned(),
        }),
    }
}

/// Starts the render thread around a renderer, and hands back the channels to it.
fn spawn(renderer: QuorraWindowRenderer) -> Link {
    let (jobs, incoming) = channel::<Job>();
    let (outgoing, done) = channel::<Done>();
    // A spawn this machine refuses is a machine with no thread to spare; the window then
    // presents whatever it has and asks for frames nobody answers — the same path a failed
    // `Link::jobs.send` already means, so there is one path rather than two.
    let thread = std::thread::Builder::new()
        .name("confined page renderer".to_owned())
        .spawn(move || {
            let mut renderer = renderer;
            while let Ok(job) = incoming.recv() {
                let finished = draw(&mut renderer, job);
                // A send that fails is an event thread that has gone; nobody is left to draw
                // for.
                if outgoing.send(finished).is_err() {
                    return;
                }
            }
        })
        .ok();
    Link { jobs, done, thread }
}

/// One job, drawn on the device — and *only* on the device.
///
/// A refusal comes home as [`Done::refused`] rather than being drawn on the processor here,
/// which is where this deliberately departs from the flagship's render thread: this window
/// already owns an interruptible CPU path (ADR 0650), and a hostile page belongs on the thread
/// an interrupt can reach.
fn draw(renderer: &mut QuorraWindowRenderer, job: Job) -> Done {
    let Job {
        width,
        height,
        pages,
        overlays,
        reuse,
    } = job;
    let textures = reuse
        .filter(|pair| pair.fits(width, height))
        .unwrap_or_else(|| Pair::new(renderer, width, height));
    let borrowed: Vec<&DisplayList> = overlays.iter().collect();
    let placed: Vec<(&Arc<DisplayList>, TargetSpec)> =
        pages.iter().map(|(list, target)| (list, *target)).collect();
    let refused = renderer
        .render(
            PresentFrame {
                width,
                height,
                pages: &placed,
                raster: None,
                overlays: &borrowed,
            },
            WindowTextures {
                page: &textures.page,
                chrome: &textures.chrome,
            },
        )
        .err()
        .map(|problem| problem.to_string());
    Done {
        textures,
        cost: renderer.last_frame(),
        refused,
        finished: Instant::now(),
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

#[cfg(test)]
mod tests {
    use super::{Presented, swapchain};

    /// The two states that mean "ask again now" — the swapchain has been replaced.
    #[test]
    fn a_replaced_swapchain_asks_again() {
        assert_eq!(
            swapchain(quorra_gpu::SurfaceProblem::Outdated, None),
            Presented::AskAgain
        );
        assert_eq!(
            swapchain(quorra_gpu::SurfaceProblem::Lost, None),
            Presented::AskAgain
        );
    }

    /// The two states that owe nothing — the next redraw is soon enough.
    #[test]
    fn an_occluded_or_saturated_swapchain_waits() {
        assert_eq!(
            swapchain(quorra_gpu::SurfaceProblem::Timeout, None),
            Presented::Waited
        );
        assert_eq!(
            swapchain(quorra_gpu::SurfaceProblem::Occluded, None),
            Presented::Waited
        );
    }

    /// A validation failure is a refusal, and it carries the device's own words where any were
    /// left — the account `Surface::configure` gives nowhere else.
    #[test]
    fn a_validation_failure_refuses_with_the_devices_words() {
        let said = render_quorra::Uncaptured {
            since: 1,
            last: "  the device's own sentence  ".to_owned(),
        };
        let Presented::Refused(why) =
            swapchain(quorra_gpu::SurfaceProblem::Validation, Some(&said))
        else {
            panic!("a validation failure is a refusal");
        };
        assert!(why.contains("the device's own sentence"), "was {why:?}");
        let Presented::Refused(why) = swapchain(quorra_gpu::SurfaceProblem::Validation, None)
        else {
            panic!("a validation failure is a refusal");
        };
        assert!(why.contains("no account"), "was {why:?}");
    }
}
