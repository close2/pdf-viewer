//! Bringing the window's surface up: the graphics device where there is one, the processor's own
//! surface where there is not, and the sentence that says which and why. The launch path's last step.

use std::sync::Arc;

use render_raster::QuorraWindowRenderer;
use viewer_ui::software::SoftwareSurface;
use winit::window::Window;

use super::{Surface, wake_on};
use crate::app::App;
use crate::arguments::backend_names;
use crate::trace::Topic;

impl App {
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
            let surface = self.software(window);
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
            Some(instance) => QuorraWindowRenderer::with_instance(instance, window.clone()),
            None => QuorraWindowRenderer::new(window.clone()),
        };

        // **A default gives way; a flag does not.** This arm is only reachable where this build
        // restricted the backends by itself — today that is Windows and DX12 — and the machine
        // turned out to have no adapter behind it. Refusing there would be this project's guess
        // deciding that somebody's machine cannot start, so it is a note and a second attempt
        // with everything. `QuorraWindowRenderer::new` makes its own all-backends instance, which is
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
            attempt = QuorraWindowRenderer::new(window.clone());
        }

        let renderer = match attempt {
            Ok(renderer) => renderer,
            Err(problem) => return self.no_device(window, instance.as_ref(), &problem),
        };
        let brought_up = began.elapsed();
        self.launch.mark("graphics device");
        // **The surface leaves the device here, on the launch path, and that is deliberate.**
        // raster's `detach_presenter` clones four handles and moves the surface state; it asks the
        // pipeline store nothing, so it cannot compile, cannot wait for warmth and cannot block
        // (raster's ADR 0056). The *thread* is not started here — that is the first job's, which
        // is `CLAUDE.md`'s rule about scheduler decisions in front of a launch milestone.
        let size = window.inner_size();
        let ungrounded = match crate::renderer::Window::split(
            renderer,
            (size.width.max(1), size.height.max(1)),
            self.proxy_pages,
            self.supersample,
            self.coverage,
            self.waker.clone().map(wake_on),
        ) {
            Ok(ungrounded) => ungrounded,
            Err(renderer) => {
                // A device built with `for_surface` always has one to hand over, so this is a
                // proof about raster's constructors rather than a state anybody has seen — and it
                // is still a sentence rather than a panic, because the alternative to a window
                // that cannot present is a window nobody can read.
                eprintln!(
                    "the graphics device came up with no surface to present through ({}), so the \
                     page is drawn on the processor instead",
                    renderer.adapter_description()
                );
                return self.software(window);
            }
        };
        // **The surface is configured here and it is a launch milestone.** `Ungrounded` carries
        // the whole argument; what it comes to is that the *first* configure of a process is the
        // one wgpu answers with a panic rather than a status, and this is the only moment where
        // no other thread of this program can be submitting while it happens. A device that
        // cannot put its own background on its window cannot show a page on it either, so the
        // refusal is a launch decision — the processor draws instead, out loud, exactly as it
        // does for a device that would not come up at all.
        let grounded = std::time::Instant::now();
        let presenter = match ungrounded.ground() {
            Ok(presenter) => presenter,
            Err(why) => {
                eprintln!("the graphics device could not put anything on this window: {why}");
                eprintln!("  asked for: {}", self.backend_description());
                eprintln!(
                    "Drawing on the processor instead, which opens no graphics driver. Pages \
                     will be slower."
                );
                return self.software(window);
            }
        };
        let grounding = grounded.elapsed();
        self.launch.mark("surface configured");
        if self.trace.on(Topic::Launch) {
            let startup = presenter.startup();
            // Two lines about one choice, and they answer different questions. The first is what
            // was *asked for* — which is a fact about this command line — and the second ends in
            // the backend that was actually chosen, because raster's adapter description carries
            // it: `llvmpipe (LLVM 22.1.8, 256 bits) (Cpu, Vulkan)`. A person diagnosing a driver
            // crash needs both (ADR 0221).
            self.trace.say(
                Topic::Launch,
                format_args!("backend asked for: {}", self.backend_description()),
            );
            self.trace.say(
                Topic::Launch,
                format_args!("rendering with {}", presenter.description()),
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
            // What grounding cost, said on its own line because it is a *number to keep small*
            // rather than a step to take on trust: it is the swapchain's creation and the
            // window's first acquire, moved off page one's present onto a moment where the
            // configure cannot race a submission (`crate::renderer::window::Ungrounded`).
            self.trace.say(
                Topic::Launch,
                format_args!(
                    "surface configured and the window's ground put up in {grounding:?} — the \
                     first configure of this process, with no render thread to submit beside it"
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
    ///
    /// **A method rather than an associated function since ADR 0461**, because this surface has a
    /// setting of its own now: `--proxy-pages` reaches the composing thread exactly as it reaches
    /// the render thread, and the host is where it lives (`doc/todo/37` rule 2).
    fn software(&self, window: &Arc<Window>) -> Option<Surface> {
        match SoftwareSurface::new(Arc::clone(window)) {
            Ok(surface) => Some(Surface::Processor(Box::new(
                crate::composer::Composer::new(surface, self.proxy_pages, self.waker.clone()),
            ))),
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
    /// **Not an `.expect`**, by `CLAUDE.md` principle 1's rule about panics, in the one place a
    /// person most needs a sentence (ADR 0221): a device that will not come up is a fact about the
    /// machine, not a defect in this program, and the shape to use is `Confinement::shortfall`'s —
    /// name the stage, name what was seen, name what to try.
    ///
    /// The two outcomes are deliberately different. A backend a **person** named is honoured or
    /// refused, because "this machine has no DX12 adapter" is an answer to the question they
    /// asked and starting on the stack they were avoiding is not. A device that failed with no
    /// backend named is a broken machine rather than a mistaken command line, so the page is
    /// drawn on the processor and the note says so.
    fn no_device(
        &self,
        window: &Arc<Window>,
        instance: Option<&raster_gpu::wgpu::Instance>,
        problem: &render_raster::QuorraRasterError,
    ) -> Option<Surface> {
        eprintln!("the graphics device could not be brought up: {problem}");
        eprintln!("  asked for: {}", self.backend_description());
        if let Some(instance) = instance {
            // What *this* instance could see, which is the number that distinguishes a backend
            // this machine does not have (none) from a driver that failed later (some).
            let visible = QuorraWindowRenderer::adapters_on(instance);
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
        let every = raster_gpu::Device::adapter_names();
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
        self.software(window)
    }
}
