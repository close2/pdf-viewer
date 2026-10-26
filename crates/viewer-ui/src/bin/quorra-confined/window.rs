//! The window's own state: bringing up what puts pixels on it, its size, its title and its redraw.
//!
//! Methods of [`crate::Host`], which this file continues (ADR 2352).

use std::sync::Arc;
use std::time::Instant;

use render_raster::QuorraWindowRenderer;
use viewer_host::trace::Topic;
use viewer_ui::software::SoftwareSurface;
use winit::window::Window;

use crate::arguments::file_name;
use crate::device::Device;
use crate::{Host, Presentation};

impl Host {
    /// Brings up whatever will put pixels on this window (ADR 0725).
    ///
    /// The device path first, unless `--cpu` kept the driver unloaded: the instance thread is
    /// joined, the renderer built on it, the surface detached and configured (`Ungrounded`
    /// carries that argument). Every refusal on the way falls to the processor's path, out
    /// loud — the second of the two jobs `CLAUDE.md` keeps the CPU backend for.
    pub(crate) fn bring_up(&mut self, window: &Arc<Window>) -> Presentation {
        if let Some(instancing) = self.instancing.take() {
            let Ok(instance) = instancing.join() else {
                eprintln!(
                    "the thread creating the graphics instance panicked; drawing on the \
                     processor instead"
                );
                return self.software(window);
            };
            self.trace
                .say(Topic::Launch, format_args!("graphics instance"));
            let began = Instant::now();
            match QuorraWindowRenderer::with_instance(&instance, Arc::clone(window)) {
                Ok(renderer) => {
                    let size = window.inner_size();
                    match Device::split(renderer, (size.width.max(1), size.height.max(1))) {
                        Ok(ungrounded) => match ungrounded.ground() {
                            Ok(device) => {
                                let device = Box::new(device);
                                self.trace.say(
                                    Topic::Launch,
                                    format_args!(
                                        "graphics device in {:.1} ms, surface configured; \
                                         rendering with {}",
                                        began.elapsed().as_secs_f64() * 1e3,
                                        device.description()
                                    ),
                                );
                                self.trace.say(
                                    Topic::Launch,
                                    format_args!("bring-up parts: {:?}", device.startup()),
                                );
                                return Presentation::Device(device);
                            }
                            Err(why) => eprintln!(
                                "the graphics device could not put anything on this window \
                                 ({why}); drawing on the processor instead"
                            ),
                        },
                        Err(renderer) => eprintln!(
                            "the graphics device came up with no surface to present through \
                             ({}); drawing on the processor instead",
                            renderer.adapter_description()
                        ),
                    }
                }
                Err(problem) => eprintln!(
                    "no graphics device for this window ({problem}); drawing on the processor \
                     instead"
                ),
            }
        }
        self.software(window)
    }

    /// The processor's surface, which is the one launch failure this window cannot show past.
    fn software(&self, window: &Arc<Window>) -> Presentation {
        match SoftwareSurface::new(Arc::clone(window)) {
            Ok(surface) => {
                self.trace
                    .say(Topic::Launch, format_args!("software surface"));
                Presentation::Software(surface)
            }
            Err(problem) => {
                // Nothing can put pixels on this window; a blank window for ever is the worse
                // answer.
                eprintln!("no software surface for this window: {problem}");
                std::process::exit(1);
            }
        }
    }

    /// A frame reached the window. It is the *first* frame only once a page is in it: the window
    /// presents the bare surround while its worker draws (ADR 1827), and the launch stamp, the
    /// presentation gate and the document's report all mean the reader's page.
    pub(crate) fn shown(&mut self, extent: winit::dpi::PhysicalSize<u32>) {
        if !self.screen.shows_a_page() {
            return;
        }
        if self.presented.is_none() {
            self.trace
                .say(Topic::Launch, format_args!("first frame presented"));
        }
        self.presented = Some((extent.width, extent.height));
    }

    pub(crate) fn redraw(&self) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    pub(crate) fn retitle(&self) {
        if let Some(window) = self.window.as_ref() {
            let mut drawn = self
                .on_the_processor
                .as_ref()
                .map_or_else(String::new, |said| format!(" — {said}"));
            if let Some(declined) = viewer_host::status::declined_pages(&self.declined) {
                drawn.push_str(" — ");
                drawn.push_str(&declined);
            }
            let loading = self
                .loading
                .as_deref()
                .or_else(|| self.waiting_said())
                .map_or_else(String::new, |said| format!(" — {said}"));
            let title = match &self.stopped {
                Some(said) => format!("{} — {said}{drawn} — confined", self.heading),
                None => format!("{}{loading}{drawn} — confined", self.heading),
            };
            window.set_title(&title);
        }
    }

    /// The file's own name, for the title.
    pub(crate) fn name(&self) -> String {
        file_name(&self.path)
    }

    /// The card changed: the picture is the card over the screen, so a device frame carries it
    /// (`--cpu` composes at present time and needs only the redraw).
    pub(crate) fn card_changed(&mut self) {
        self.ask_frame();
        self.redraw();
    }
}
