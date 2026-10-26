//! The frames: pulled from the worker, routed to the device or the processor, and put on the window.
//!
//! Methods of [`crate::Host`], which this file continues (ADR 2352).

use std::time::Instant;

use viewer_confined::{Payload, Reply};
use viewer_core::{Command, Query};
use viewer_host::trace::Topic;

use crate::device::Presented;
use crate::{Host, Presentation, device, pricing};

impl Host {
    /// Asks the confined viewer for the frame and hands it to the screen.
    pub(crate) fn pull_frame(&mut self) {
        if self.stopped.is_some() {
            return;
        }
        let began = self.sent.take().unwrap_or_else(Instant::now);
        let asked = match self.confined.as_mut() {
            Some(confined) => confined.query(Query::Frame),
            None => return,
        };
        match asked {
            Ok(Reply::Frame(frames)) => {
                self.trace.say(
                    Topic::Frames,
                    format_args!(
                        "frame: {} page(s), {} as marks",
                        frames.len(),
                        frames
                            .iter()
                            .filter(|framed| matches!(framed.payload, Payload::List { .. }))
                            .count()
                    ),
                );
                // A window the worker drew, said in the toolkit windows' words for the one they
                // ask for, so a drive judges the three alike (ADR 2016).
                for framed in &frames {
                    if let Payload::Window {
                        target,
                        corner,
                        raster,
                    } = &framed.payload
                    {
                        self.trace.say(
                            Topic::Frames,
                            format_args!(
                                "page {} crossed as the window shows it, {}x{} at {},{} of {}x{}",
                                framed.page.saturating_add(1),
                                raster.width,
                                raster.height,
                                corner.0,
                                corner.1,
                                target.width,
                                target.height
                            ),
                        );
                    }
                }
                // A page the worker drew as pixels whose draw declined a mark: the worker's viewer
                // has said it, and the title names it as it names a page drawn here (ADR 2316).
                let mut named = false;
                for framed in frames.iter().filter(|framed| framed.declined) {
                    named |= self.declined.insert(framed.page);
                }
                if named {
                    self.retitle();
                }
                if self.screen.draws_on_the_device() {
                    let forecasts =
                        self.pricing
                            .frame(&frames, began.elapsed(), self.screen.extent());
                    self.route(&forecasts);
                }
                self.screen.take(frames, &mut self.drawing);
                self.refused_since_take = false;
                // A frame crossed, so where the reader is now is where a resume goes back to —
                // and the restart budget starts again from here, because what it bounds is a
                // recovery that is not working rather than the length of the reading. The view is
                // *asked for* rather than assembled from what this window sent: `Zoom` and
                // `Scroll` are relative and the viewer clamps them, so only the viewer knows.
                if let Some(view) = self.view() {
                    self.resuming.showing(view);
                }
                self.ask_frame();
                self.redraw();
            }
            // No document is focused — before the open, or after a failed one.
            Ok(_) => {}
            Err(problem) => self.died(&problem),
        }
    }

    /// Says each priced page's route, and asks the worker for the one that changed (ADR 2017):
    /// `quorra`'s `device chosen` and `processor chosen` lines, with the worker's window as the
    /// processor (ADR 1886).
    fn route(&mut self, forecasts: &[pricing::Forecast]) {
        for forecast in forecasts {
            let (chosen, past) = match forecast.chosen {
                viewer_confined::Route::Window => ("window", "the device's past the warning"),
                viewer_confined::Route::Marks => {
                    ("device", "the device's inside the warning or the smaller")
                }
            };
            self.trace.say(
                Topic::Frames,
                format_args!(
                    "page {} {chosen} chosen: device {:.1} ms with the crossing (measured), window \
                     {:.1} ms ({}); {past}",
                    forecast.page.saturating_add(1),
                    forecast.device.as_secs_f64() * 1e3,
                    forecast.window.as_secs_f64() * 1e3,
                    if forecast.window_measured {
                        "measured"
                    } else {
                        "seed"
                    },
                ),
            );
            if !forecast.changed {
                continue;
            }
            let Some(confined) = self.confined.as_mut() else {
                return;
            };
            match confined.route(forecast.page, forecast.chosen) {
                Ok(()) => self.pricing.asked(forecast.page, forecast.chosen),
                // The page keeps the route it had, which draws it; the sentence says why the
                // quicker one was not taken.
                Err(problem) => eprintln!(
                    "page {}: the worker did not take the {chosen} route: {problem}",
                    forecast.page.saturating_add(1)
                ),
            }
        }
    }

    /// One device frame came home: adopted, or refused and handed to the fallback thread.
    pub(crate) fn frame_landed(&mut self, landed: device::Landed) {
        if let Some(why) = landed.refused {
            let asked = self.screen.fall_back(&mut self.drawing);
            eprintln!(
                "the graphics device refused the frame ({why}); {asked} page(s) fall back to \
                 the processor"
            );
            self.refused_since_take = true;
            // **A title is one line**, and `wgpu` writes a refused pipeline as a chain of causes,
            // one per line: the title takes the first, which names what was refused, and the
            // terminal above keeps all of it (ADR 1962).
            let said = format!(
                "drawn on the processor: the graphics device refused ({})",
                why.lines().next().unwrap_or(&why)
            );
            if self.on_the_processor.as_ref() != Some(&said) {
                self.on_the_processor = Some(said);
                self.retitle();
            }
            if asked > 0 {
                // The card and the pixel pages still want a frame, now without the refused
                // marks; the fallback pages join it as each lands off the thread.
                self.ask_frame();
            }
            return;
        }
        if !self.refused_since_take && self.on_the_processor.take().is_some() {
            self.retitle();
        }
        self.pricing
            .device_drew(landed.cost.total, self.screen.device_commands());
        self.trace.say(
            Topic::Frames,
            format_args!(
                "device frame in {:.1} ms, asked-to-finished {:.1} ms",
                landed.cost.total.as_secs_f64() * 1e3,
                landed.waited.as_secs_f64() * 1e3
            ),
        );
        self.redraw();
    }

    /// The cards as display lists — §7.6.4.1's and the *ask* level's — over whatever the screen
    /// has, which for an encrypted document is the surround, exactly the flagship's arrangement of
    /// the same cards. The device and the processor both draw what this answers (ADR 2040).
    fn overlays(&self) -> Vec<pdf_render::DisplayList> {
        let (Some(window), Some(chrome)) = (self.window.as_ref(), self.chrome.as_ref()) else {
            return Vec::new();
        };
        let extent = window.inner_size();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a display's scale factor is a small ratio"
        )]
        let scale = window.scale_factor() as f32;
        self.permissions
            .draw(chrome, extent.width, extent.height, scale)
            .into_iter()
            .chain(
                self.password
                    .draw(chrome, extent.width, extent.height, scale),
            )
            .chain(
                self.question
                    .draw(chrome, extent.width, extent.height, scale),
            )
            .collect()
    }

    /// Asks the device for a frame of what the screen now holds, where a device is what draws.
    ///
    /// Called wherever the picture changed — a frame pulled, the card typed into, a resize, a
    /// fallback draw landing. Under `--cpu` there is no device and presentation composes from
    /// the screen directly, so this is nothing.
    pub(crate) fn ask_frame(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let extent = window.inner_size();
        self.screen.resize(extent.width, extent.height);
        let overlays = self.overlays();
        if let Some(Presentation::Device(device)) = self.presentation.as_mut() {
            device.ask(
                self.screen.device_pages(extent.width, extent.height),
                overlays,
            );
        }
    }

    /// Puts what the screen has onto the window, under the presentation gate.
    pub(crate) fn present(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let extent = window.inner_size();
        self.screen.resize(extent.width, extent.height);
        match self.presentation.as_mut() {
            Some(Presentation::Device(device)) => match device.present() {
                Presented::Shown => self.shown(extent),
                // Before the first frame lands the grounded surround stays up, which is the
                // window's honest picture of having nothing yet.
                Presented::Nothing | Presented::Waited => {}
                Presented::AskAgain => self.redraw(),
                Presented::Refused(why) => {
                    eprintln!("the frame could not be presented: {why}");
                }
            },
            Some(Presentation::Software(_)) => self.present_software(),
            None => return,
        }
        for (page, words) in self.screen.refusals() {
            eprintln!(
                "page {} could not be drawn: {words}",
                page.saturating_add(1)
            );
        }
        // The reader has their page, so the document may now be asked what it says about itself
        // — §12.8's signatures above all, whose answer reads and digests the signed part of the
        // file behind the filter. Once per opened document; `viewer_host::report::Due` is the
        // rule all four hosts follow. ADR 1044.
        if self.presented.is_some() && self.report_due.after_a_frame() {
            self.dispatch(Command::Report);
            // And its open sequence, which runs in the worker beside the scripts' view, after the
            // first present and never in front of it (ADR 1602).
            self.dispatch(Command::Presented);
        }
    }

    /// The processor's present: the screen composed, the card drawn over it.
    fn present_software(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let extent = window.inner_size();
        let resized = self.presented != Some((extent.width, extent.height));
        if !(self.screen.settled() || self.presented.is_none() || resized) {
            // Something newer is still on the drawing thread and the window already shows a
            // whole frame of the right size: keep it (doc/todo/37's rule) rather than blink
            // pages in and out.
            return;
        }
        let Some(composed) = self.screen.compose() else {
            return;
        };
        let cards = self.overlays();
        let Some(Presentation::Software(surface)) = self.presentation.as_mut() else {
            return;
        };
        let overlays: Vec<&pdf_render::DisplayList> = cards.iter().collect();
        match surface.present(&composed, &overlays) {
            Ok(()) => self.shown(extent),
            Err(problem) => eprintln!("the frame could not be presented: {problem}"),
        }
    }
}
