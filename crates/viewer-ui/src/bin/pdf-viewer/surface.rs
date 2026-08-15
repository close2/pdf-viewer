//! How this window gets pixels: which surface is behind it, and what one frame does.
//!
//! Two paths and never both — a graphics device or the processor's own surface — and the whole of
//! the difference is in [`Surface`]. Bringing one up is the launch path's last step and is
//! therefore measured; drawing on it is the frame, and the frame is where every other module's
//! work arrives, the page from `viewer-core` and the overlays from this host.

use std::sync::Arc;

use pdf_render::{Rasterizer as _, TargetSpec, Transform};
use render_cpu::CpuRasterizer;
use render_quorra::{PresentFrame, QuorraPresenter};
use viewer_core::{Answer, Command, Query, Rendered};
use viewer_ui::software::SoftwareSurface;
use winit::window::Window;

use crate::app::App;
use crate::arguments::backend_names;
use crate::overlays::Overlays;
use crate::timing::Stages;
use crate::trace::Topic;

/// The magnification past which quorra's GPU coverage lane is the cheaper one.
///
/// **Derived, not tuned.** quorra keeps a glyph's rasterised coverage in an atlas until
/// the glyph exceeds 128 device pixels; past that it rasterises the glyph again on
/// every frame, which is where its cost stops being flat (its ADR 0016). The
/// magnification that happens at is `128 ÷ the height of the text`, so body text of 10
/// to 12 points crosses it between 10.7× and 13×. Ten is the low end of that band,
/// chosen because being early costs a fraction of a millisecond and being late costs
/// ten — measured on this machine at 0.44 ms per frame at 8× against 4.4 ms at 12×
/// (`doc/quorra-gpu-coverage.md`).
///
/// A page whose text is much larger or much smaller than a book's crosses it somewhere
/// else, and the honest way to do better would be to ask the display list what size its
/// text is rather than to move this number.
const GPU_COVERAGE_MAGNIFICATION: f32 = 10.0;

/// Which coverage lane the next frame should be drawn with.
///
/// Per frame, because the crossover is a magnification and a person zooming crosses it;
/// decided *here*, because this is the only crate that knows what magnification the
/// frame is at. The transform's determinant is the magnification squared — the page
/// transform is a scale, a y flip and a translation, and §7.7.3.3's page rotation puts
/// the same factor into `b` and `c` instead of `a` and `d` — so its square root is the
/// number to compare, and it is right for a rotated page as well.
fn coverage_for(transform: Transform) -> quorra_gpu::Coverage {
    let magnification = transform
        .a
        .mul_add(transform.d, -(transform.b * transform.c))
        .abs()
        .sqrt();
    if magnification >= GPU_COVERAGE_MAGNIFICATION {
        quorra_gpu::Coverage::Gpu
    } else {
        quorra_gpu::Coverage::Cpu
    }
}

/// Draws the page with [`CpuRasterizer`] and puts it on the window, whichever surface it has.
///
/// **This is one of the two jobs `CLAUDE.md` keeps the CPU backend for**: the correctness oracle,
/// and the frame the graphics device refuses. (It was three until the two-hundred-and-seventy-third
/// session, where the project owner decided page one goes to the device.) So a page the device
/// refuses is a page this program can still show — more slowly, which is a cost a person can see
/// past, where a page that never appears is not.
///
/// **Two ways back to the window, and the difference is where the overlays are composited.** With
/// a device, the raster is handed over as one image and quorra draws the overlays over it as
/// geometry, because its surface is the only path pixels take. Without one, `SoftwareSurface`
/// composites them on the processor and copies the result. `--cpu` takes the second, and so does
/// a machine whose device would not come up.
///
/// The error is a sentence rather than a type because both of its sources are already strings by
/// the time they reach the caller, which formats them into one report.
fn on_the_processor(
    surface: &mut Surface,
    list: &pdf_render::DisplayList,
    target: TargetSpec,
    overlays: &[&pdf_render::DisplayList],
) -> Result<(), String> {
    let raster = CpuRasterizer::new()
        .rasterize(list, target)
        .map_err(|problem| format!("the processor {problem}"))?;
    match surface {
        Surface::Device(presenter) => presenter
            .present(PresentFrame {
                width: target.width,
                height: target.height,
                page: None,
                raster: Some(&raster),
                overlays,
            })
            .map_err(|problem| format!("presenting the processor's page {problem}")),
        Surface::Processor(surface) => surface
            .present(&raster, overlays)
            .map_err(|problem| format!("presenting the processor's page {problem}")),
    }
}

/// How this window's pixels reach it: with a graphics device, or without one.
///
/// **Never both, and that is the point.** A process holding [`Surface::Processor`] has created no
/// `wgpu::Instance`, selected no adapter and made no device, so a driver that faults while it
/// loads cannot reach it. Before the three-hundred-and-eighty-fourth session there was one
/// variant and `--cpu` chose only which rasteriser drew into it (ADR 0221).
pub(crate) enum Surface {
    /// quorra's device holds the surface; one call draws and presents a frame,
    /// and a refused frame is a typed error naming what refused — the banding
    /// machinery, the owned intermediate texture and the blitter of the Vello
    /// host all fell away with the backend that needed them.
    Device(Box<QuorraPresenter>),
    /// The processor's raster copied onto the window, with the overlays composited into it
    /// first. `--cpu`, and a device that would not come up.
    Processor(SoftwareSurface),
}

impl std::fmt::Debug for Surface {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Device(presenter) => formatter.debug_tuple("Device").field(presenter).finish(),
            Self::Processor(surface) => formatter.debug_tuple("Processor").field(surface).finish(),
        }
    }
}

/// The window, and whatever puts pixels on it.
pub(crate) struct State {
    pub(crate) window: Arc<Window>,
    /// The graphics device's surface, or the software one. See [`Surface`].
    pub(crate) surface: Surface,
    /// The surface size in device pixels, updated on `WindowEvent::Resized`.
    pub(crate) size: (u32, u32),
}

impl App {
    /// `stages` is filled in as the frame goes: see [`Stages`] for why one number was not enough.
    #[expect(
        clippy::too_many_lines,
        reason = "one frame in one sequence — the request, the placement, the reprojection \
                  decision, the transition, the chrome, the device and what to do when it \
                  refuses. Each is a handful of lines and their *order* is the content; split \
                  into parts, a reader would have to reconstruct it from six signatures"
    )]
    fn present(&mut self, stages: &mut Stages) -> Option<Rendered> {
        let began = std::time::Instant::now();
        // §12.3.4's list is built here and nowhere else: this is the one place that holds
        // `&mut self` and runs before the panel is drawn.
        self.ensure_pages();
        let request = self.request.clone()?;
        stages.page = request.page.saturating_add(1);
        stages.commands = request.list.commands().len();
        // Where the page sits in the window: the core centres it and scrolls it, and the host
        // draws it there by composing that offset into the target's own transform.
        let origin = match self.viewer.query(Query::PageGeometry(request.page)) {
            Answer::Geometry(geometry) => geometry.origin,
            _ => (0.0, 0.0),
        };
        let (width, height) = {
            let state = self.state.as_ref()?;
            state.size
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a panel width in pixels, which is hundreds"
        )]
        let edge = self.inset() as f32;
        let target = TargetSpec {
            width,
            height,
            transform: request
                .target
                .transform
                .then(Transform::translate(origin.0 + edge, origin.1)),
        };

        // Asked before the transition below, because it is a question about *this page's*
        // placement: whether the pixels on the screen are this page under another view, and
        // whether the frame that put them there was slow enough for the wait to be worth
        // answering. See [`crate::stale`] — every rule that makes an approximation defensible
        // is enforced in there rather than here.
        let planned = self.stale.plan(&request.list, target);
        let placement = target;

        // §12.4.4: a transition in flight substitutes its own picture of *two* pages for the one,
        // and it is a display list, so everything below this line is unchanged by it — which is
        // the point of shaping a frame in `viewer_core::transition` rather than compositing here.
        let (playing, target) = self.frame_to_draw(&request, target, width, height);
        let list = playing.as_ref().unwrap_or(&request.list);

        let chrome = Overlays::of(self, edge, width, height);
        let overlays = chrome.lists();
        stages.host = began.elapsed();

        // A transition is already a picture of two pages moving, drawn from rasters this host
        // took for it: there is no stall to cover and the pixels on the screen are not a page.
        if playing.is_some() {
            self.stale.forget();
        } else if let Some(moved) = planned
            && self.approximate(moved, &overlays, stages)
        {
            // Deliberately not a `Rendered`: the core is told what became of its request by the
            // frame that *answers* it, and a reprojection answers nothing. Nothing is
            // acknowledged, the launch timeline stays open, and the accessibility tree is not
            // published from a picture that is not the page.
            return None;
        }

        let state = self.state.as_mut()?;
        let drawn = match &mut state.surface {
            // No device to ask. Not a refusal either: the software surface below is this run's
            // only path, and calling it a failure would print a note on every frame.
            Surface::Processor(_) => Err(String::new()),
            Surface::Device(presenter) => {
                // Which lane draws this frame's coverage, decided from this frame's
                // magnification: see `coverage_for`. Set every frame rather than when it
                // changes, because it is a field write and tracking the change would be
                // more state than the thing it saved.
                presenter.set_coverage(coverage_for(target.transform));
                let handed = std::time::Instant::now();
                let outcome = presenter.present(PresentFrame {
                    width,
                    height,
                    page: Some((list, target)),
                    raster: None,
                    overlays: &overlays,
                });
                // Read back whatever the frame cost before anything is decided about it: a
                // refusal has an accounting too, and it is the one a person most wants.
                stages.gpu = presenter.last_frame();
                // ADR 0376: a §8.7.4.5.2 program the device declined draws from the grid
                // instead — the right picture, four orders of magnitude slower — so the ground
                // is said out loud rather than left to a timing to imply. Said on every frame
                // that carries one, because a replayed frame is still drawing it.
                for ground in presenter.last_function_paints().refusals() {
                    self.trace.say(
                        Topic::Frames,
                        format_args!("function shading drawn on the processor's grid: {ground}"),
                    );
                }
                // The first frame's scene translation is a launch milestone — the other
                // half, with interpretation, of what used to sit unnamed between `document
                // joined` and `first present` (ADR 0332). The method keeps only the first
                // and computes the mark from quorra's own `scene` measurement, because the
                // boundary is inside the call above.
                self.launch.scene_built(handed, stages.gpu.scene);
                match outcome {
                    Ok(()) => Ok(()),
                    // Swapchain states are events, not failures: nothing was presented,
                    // nothing is stale, and the processor cannot help a window that is
                    // not presentable — so these return rather than fall back.
                    Err(render_quorra::QuorraRasterError::Render(
                        quorra_gpu::RenderError::SurfaceUnavailable { reason },
                    )) => {
                        return match reason {
                            quorra_gpu::SurfaceProblem::Outdated
                            | quorra_gpu::SurfaceProblem::Lost => {
                                state.window.request_redraw();
                                None
                            }
                            quorra_gpu::SurfaceProblem::Timeout
                            | quorra_gpu::SurfaceProblem::Occluded => None,
                            quorra_gpu::SurfaceProblem::Validation => {
                                Some(Rendered::Failed("swapchain validation failed".to_owned()))
                            }
                        };
                    }
                    Err(error) => Err(error.to_string()),
                }
            }
        };
        if let Err(problem) = drawn {
            let fell_back = std::time::Instant::now();
            let second = on_the_processor(&mut state.surface, list, target, &overlays);
            stages.fallback = fell_back.elapsed();
            if let Err(second) = second {
                return Some(Rendered::Failed(if problem.is_empty() {
                    second
                } else {
                    format!("the graphics device {problem}, and {second}")
                }));
            }
            // Reported when there *was* a device that refused. A page drawn by the slower of two
            // backends is a fact about this build worth saying out loud, and saying it is what
            // would have made the hundred-and-forty-second session's report a sentence rather
            // than a mystery — but under `--cpu` there is nothing to report, which is why the
            // empty `problem` above is a sentinel rather than a message.
            if !problem.is_empty() {
                println!(
                    "note: page {}: the graphics device {problem}, so it was drawn on the \
                     processor instead",
                    request.page.saturating_add(1)
                );
            }
        }
        // What the window is showing, and what showing it cost. The next view change reads both:
        // the placement to carry the pixels *from*, and the cost to decide whether carrying them
        // is worth anything at all (`doc/todo/37` rule 5, ADR 0378).
        if playing.is_none() {
            self.stale
                .settled(&request.list, placement, began.elapsed());
        }
        Some(Rendered::Presented)
    }

    /// Puts the pixels the window is already showing where this view puts them, and asks for the
    /// frame that replaces them.
    ///
    /// `true` when the window answered the input; `false` when it did not, and the caller then
    /// draws the real frame exactly as it always did. **Every one of those refusals is an
    /// ordinary state rather than a failure** — no device, no retained encode, a readback the
    /// device declined, a raster this cannot read — and the two that say something about *this
    /// machine* rather than about this frame stop the attempt from being made again.
    ///
    /// The chrome is the current frame's, drawn over the reprojection as geometry: a selection
    /// and a sidebar are this host's own state and are true at the moment they are drawn, so
    /// only the page underneath them is an approximation.
    fn approximate(
        &mut self,
        moved: Transform,
        overlays: &[&pdf_render::DisplayList],
        stages: &mut Stages,
    ) -> bool {
        let began = std::time::Instant::now();
        let trace = self.trace;
        let covering = self.stale.covering();
        let Some(state) = self.state.as_mut() else {
            return false;
        };
        let (width, height) = state.size;
        let Surface::Device(presenter) = &mut state.surface else {
            // Nothing to read back: the processor's window has no retained encode, so its
            // pixels would have to be produced by rasterising the page again — which is the
            // cost this exists to hide rather than one it can hide. Asked once, then never.
            self.stale.refuse();
            return false;
        };
        let captured = match presenter.capture_presented() {
            Ok(Some(captured)) => captured,
            Ok(None) => return false,
            Err(problem) => {
                trace.say(
                    Topic::Frames,
                    format_args!("the frame on the window could not be read back: {problem}"),
                );
                self.stale.refuse();
                return false;
            }
        };
        // Rule 4, and the one condition that cannot be checked in advance: a capture that
        // re-encoded has just paid the whole of the cost the reprojection exists to cover, so
        // it is said out loud and never asked for again.
        if !captured.replayed {
            trace.say(
                Topic::Frames,
                format_args!(
                    "reading the frame back re-encoded it in {:.1} ms instead of replaying it, \
                     which costs the real frame more than the wait is worth — no frame will be \
                     approximated in this run",
                    captured.cost.as_secs_f64() * 1e3
                ),
            );
            self.stale.refuse();
            return false;
        }
        let Some(list) = crate::stale::reprojection(&captured.raster, moved) else {
            return false;
        };
        let list = Arc::new(list);
        // The pixels are the window's own, so they are placed in the window's own space — the
        // identity target every overlay list already uses.
        let placement = TargetSpec {
            width,
            height,
            transform: Transform::IDENTITY,
        };
        if let Err(problem) = presenter.present(PresentFrame {
            width,
            height,
            page: Some((&list, placement)),
            raster: None,
            overlays,
        }) {
            trace.say(
                Topic::Frames,
                format_args!("the device refused an approximated frame: {problem}"),
            );
            return false;
        }
        stages.gpu = presenter.last_frame();
        stages.commands = list.commands().len();
        stages.approximated = true;
        let cost = began.elapsed();
        // Rule 3 twice over: the frame line says `approximated`, and this says what it is an
        // approximation *of* — which frame it stands in for, and what the picture on the window
        // cost against it.
        trace.say(
            Topic::Frames,
            format_args!(
                "approximated: the {:.1} ms frame this view replaces is stood in for by its own \
                 pixels moved (read back in {:.1} ms, whole reprojection {:.1} ms); the real \
                 frame has been asked for",
                covering.as_secs_f64() * 1e3,
                captured.cost.as_secs_f64() * 1e3,
                cost.as_secs_f64() * 1e3,
            ),
        );
        // Rule 1, as a value that cannot be dropped: the window is asked for the frame that
        // replaces this one, here, in the same expression that records it.
        self.stale.drawn(cost).follow(&state.window);
        true
    }

    /// Brings up whatever will put pixels on this window, or says why nothing can.
    ///
    /// **Two paths, and `--cpu` chooses between them.** With the flag, no `wgpu::Instance` is
    /// created, no adapter is enumerated and no device is made — the driver is never loaded, which
    /// is what makes the flag an answer to a driver that faults while loading. Without it, the
    /// device is brought up as it always was, and the processor's path is what a machine falls to
    /// when the device will not come at all.
    ///
    /// `None` means neither worked, which is the one launch failure this program cannot show a
    /// page past.
    pub(crate) fn bring_up(&mut self, window: &Arc<Window>) -> Option<Surface> {
        if self.processor {
            let surface = Self::software(window);
            self.launch.mark("software surface");
            return surface;
        }

        // Shaders compile on a background thread and nothing here waits for them —
        // `CLAUDE.md`'s rule, since page one goes to the graphics device: what bringing the
        // device up costs is part of time-to-first-page, so it is measured rather than
        // assumed. The presenter reports uncaptured device errors itself, for the same
        // silent-window reason the Vello host did.
        let mut instance = self
            .instancing
            .take()
            .map(|thread| thread.join().expect("the thread creating the instance"));
        self.launch.mark("graphics instance");
        let began = std::time::Instant::now();
        let mut attempt = match instance.as_ref() {
            Some(instance) => QuorraPresenter::with_instance(instance, window.clone()),
            None => QuorraPresenter::new(window.clone()),
        };

        // **A default gives way; a flag does not.** This arm is only reachable where this build
        // restricted the backends by itself — today that is Windows and DX12 — and the machine
        // turned out to have no adapter behind it. Refusing there would be this project's guess
        // deciding that somebody's machine cannot start, so it is a note and a second attempt
        // with everything. `QuorraPresenter::new` makes its own all-backends instance, which is
        // why the old one is dropped rather than reused.
        if attempt.is_err()
            && !self.backend_asked_for
            && let Some(named) = self.backend.take()
        {
            println!(
                "note: no graphics adapter behind the {} backend, which is the one this build \
                 asks for first on this platform, so every backend was offered instead",
                named.name()
            );
            instance = None;
            attempt = QuorraPresenter::new(window.clone());
        }

        let presenter = match attempt {
            Ok(presenter) => presenter,
            Err(problem) => return self.no_device(window, instance.as_ref(), &problem),
        };
        let brought_up = began.elapsed();
        self.launch.mark("graphics device");
        if self.trace.on(Topic::Launch) {
            let startup = presenter.startup();
            // Two lines about one choice, and they answer different questions. The first is what
            // was *asked for* — which is a fact about this command line — and the second ends in
            // the backend that was actually chosen, because quorra's adapter description carries
            // it: `llvmpipe (LLVM 22.1.8, 256 bits) (Cpu, Vulkan)`. A person diagnosing a driver
            // crash needs both, and before the three-hundred-and-eighty-fourth session there was
            // no way to ask for the first at all.
            self.trace.say(
                Topic::Launch,
                format_args!("backend asked for: {}", self.backend_description()),
            );
            self.trace.say(
                Topic::Launch,
                format_args!("rendering with {}", presenter.adapter_description()),
            );
            self.trace.say(
                Topic::Launch,
                format_args!(
                    "device up in {brought_up:?} — instance {:?}, surface {:?}, adapter {:?}, \
                     device {:?}, pipelines {}",
                    startup.instance_creation,
                    startup.surface_creation,
                    startup.adapter_selection,
                    startup.device_creation,
                    startup
                        .pipeline_compilation
                        .map_or_else(|| "still compiling".to_owned(), |d| format!("{d:?}"))
                ),
            );
        }
        Some(Surface::Device(Box::new(presenter)))
    }

    /// What this run asked the graphics stack for, in words, for `--trace` and for a refusal.
    fn backend_description(&self) -> String {
        match (self.backend, self.backend_asked_for) {
            (Some(named), true) => format!("{} (--backend)", named.name()),
            (Some(named), false) => format!("{} (this platform's default)", named.name()),
            (None, _) => "every backend this build has".to_owned(),
        }
    }

    /// The window written to by the processor, or a sentence saying why there is not one.
    fn software(window: &Arc<Window>) -> Option<Surface> {
        match SoftwareSurface::new(Arc::clone(window)) {
            Ok(surface) => Some(Surface::Processor(surface)),
            Err(problem) => {
                eprintln!(
                    "this window cannot be drawn on without a graphics device: {problem}\n\
                     The software path is compiled for X11 and Wayland here; a session that is \
                     neither has no way to show a page without a device."
                );
                None
            }
        }
    }

    /// What to say — and what to do — when the graphics device will not come up.
    ///
    /// **This was `.expect("presenter creation")` until the three-hundred-and-eighty-fourth
    /// session**, which is `CLAUDE.md` principle 1's rule about panics in the one place a person
    /// most needs a sentence: a device that will not come up is a fact about the machine, not a
    /// defect in this program, and the shape to use is `Confinement::shortfall`'s — name the
    /// stage, name what was seen, name what to try.
    ///
    /// The two outcomes are deliberately different. A backend a **person** named is honoured or
    /// refused, because "this machine has no DX12 adapter" is an answer to the question they
    /// asked and starting on the stack they were avoiding is not. A device that failed with no
    /// backend named is a broken machine rather than a mistaken command line, so the page is
    /// drawn on the processor and the note says so.
    fn no_device(
        &self,
        window: &Arc<Window>,
        instance: Option<&quorra_gpu::wgpu::Instance>,
        problem: &render_quorra::QuorraRasterError,
    ) -> Option<Surface> {
        eprintln!("the graphics device could not be brought up: {problem}");
        eprintln!("  asked for: {}", self.backend_description());
        if let Some(instance) = instance {
            // What *this* instance could see, which is the number that distinguishes a backend
            // this machine does not have (none) from a driver that failed later (some).
            let visible = QuorraPresenter::adapters_on(instance);
            eprintln!(
                "  adapters behind it: {}",
                if visible.is_empty() {
                    "none — this machine has no adapter for that backend".to_owned()
                } else {
                    visible.join(", ")
                }
            );
        }
        // And what the machine has by every route, which is the list a person picks their next
        // `--backend` out of. Its own all-backends instance, so it costs a driver load — which is
        // acceptable here and nowhere else on this path: this run has already failed.
        let every = quorra_gpu::Device::adapter_names();
        eprintln!(
            "  adapters on this machine: {}",
            if every.is_empty() {
                "none".to_owned()
            } else {
                every.join(", ")
            }
        );
        if self.backend_asked_for {
            eprintln!(
                "Refused rather than started on another backend: --backend named one, and a flag \
                 that silently did something else would be worse than no flag. Try --backend with \
                 one of {}, or --cpu, which opens no graphics driver at all.",
                backend_names()
            );
            return None;
        }
        eprintln!(
            "Drawing on the processor instead, which opens no graphics driver. Pages will be slower."
        );
        Self::software(window)
    }

    /// Closes the one line the launch timeline never had: when the pipelines finished compiling.
    ///
    /// **ADR 0227.** Bring-up prints `pipelines still compiling` because
    /// `CLAUDE.md` forbids waiting for warmth on the launch path, and nothing ever said when the
    /// wait nobody did would have ended — so a first frame that absorbed a shader compilation
    /// and one that did not read identically. Polled once a frame, which behind the topic check
    /// is one `Option` read; *noticed* rather than *finished*, because the compilation ends on
    /// quorra's own thread and this is only the first frame to look.
    fn pipelines_compiled(&mut self) {
        if self.frames.pipelines || !self.trace.on(Topic::Launch) {
            return;
        }
        let Some(State {
            surface: Surface::Device(presenter),
            ..
        }) = self.state.as_ref()
        else {
            return;
        };
        let Some(compiling) = presenter.startup().pipeline_compilation else {
            return;
        };
        self.frames.pipelines = true;
        self.trace.say(
            Topic::Launch,
            format_args!(
                "pipelines compiled in {compiling:?}, noticed at this frame — nothing on the \
                 launch path waited for them, so every frame before this one drew with whatever \
                 was ready"
            ),
        );
    }

    /// Draws the frame the window asked for, and tells the core what became of it.
    ///
    /// **The frame's number is the frame's, since the three-hundred-and-ninetieth session.**
    /// This used to start a timer, present, close the launch timeline, publish the accessibility
    /// tree, and *then* read the timer — so `present -> presented in T` was the frame plus the
    /// bridge plus the timeline's own printing. ADR 0227 called that a measurement
    /// defect rather than a design choice, which is exactly what it was: on a page turn the tree
    /// is rebuilt and published, and that work was being attributed to the graphics device.
    pub(crate) fn redraw_requested(&mut self) {
        // Before the frame rather than after it: a tick can advance the page, and a page that
        // advanced after its frame was drawn would be one frame late for the whole slide show.
        self.drive_the_clock();
        let started = std::time::Instant::now();
        let mut stages = Stages::default();
        let outcome = self.present(&mut stages);
        stages.total = started.elapsed();
        if matches!(outcome, Some(Rendered::Presented | Rendered::Raster(_))) {
            self.launch.arrived(self.trace);
            // **After the timeline is closed, never before it.** Everything the accessibility
            // bridge does is off the launch path by construction, and this line is where that is
            // enforced rather than merely intended.
            let attending = std::time::Instant::now();
            self.attend();
            stages.attend = attending.elapsed();
        }
        // Rule 1's other half: *every* frame that is not a reprojection clears the flag, a frame
        // that drew nothing included. A flag that could outlive the redraw answering it would
        // leave `about_to_wait` asking for a frame that never comes, which is a spinning loop.
        if !stages.approximated {
            self.stale.real();
        }
        let outcome_said = match &outcome {
            None if stages.approximated => "approximated".to_owned(),
            None => "nothing to show".to_owned(),
            Some(Rendered::Presented) => "presented".to_owned(),
            Some(Rendered::Failed(why)) => format!("failed: {why}"),
            Some(Rendered::Raster(_)) => "a raster".to_owned(),
        };
        let trace = self.trace;
        self.frames.frame(trace, &stages, &outcome_said);
        self.pipelines_compiled();
        let Some(rendered) = outcome else {
            return;
        };
        if !self.acknowledged
            && let Some(token) = self.request.as_ref().map(|request| request.token)
        {
            self.acknowledged = true;
            self.dispatch(Command::RenderReady { token, rendered });
        }
    }
}

#[cfg(test)]
mod tests {
    use pdf_render::Transform;

    use super::{GPU_COVERAGE_MAGNIFICATION, coverage_for};

    /// The lane follows the magnification, and the page transform a frame is drawn
    /// with is what states it: scale, y flip, translation.
    #[test]
    fn the_lane_follows_the_magnification() {
        let page = |magnification: f32| {
            Transform::scale(magnification, -magnification)
                .then(Transform::translate(0.0, 842.0 * magnification))
        };
        assert_eq!(
            coverage_for(page(8.0)),
            quorra_gpu::Coverage::Cpu,
            "below the atlas cliff the cached lane is cheaper"
        );
        assert_eq!(
            coverage_for(page(12.0)),
            quorra_gpu::Coverage::Gpu,
            "above it the CPU lane rasterises every glyph on every frame"
        );
        assert_eq!(
            coverage_for(page(GPU_COVERAGE_MAGNIFICATION)),
            quorra_gpu::Coverage::Gpu,
            "the threshold itself belongs to the lane it names"
        );
    }

    /// §7.7.3.3's page rotation puts the magnification in `b` and `c` rather than `a`
    /// and `d`, so a rotated page must land on the same lane as an upright one at the
    /// same zoom. This is the case a `transform.a` test would get wrong — and get wrong
    /// silently, by choosing the slow lane on a quarter of the corpus.
    #[test]
    fn a_rotated_page_reads_the_same_magnification() {
        let upright = Transform::scale(12.0, -12.0);
        // A quarter turn: the scale moves off the diagonal entirely.
        let turned = Transform {
            a: 0.0,
            b: 12.0,
            c: -12.0,
            d: 0.0,
            e: 0.0,
            f: 0.0,
        };
        assert_eq!(coverage_for(upright), coverage_for(turned));
        assert_eq!(coverage_for(turned), quorra_gpu::Coverage::Gpu);
    }
}
