//! What the window does with the render thread's answers on the device path: taking a finished
//! picture in, saying which lane drew it, and what a device that reports a fault or refuses a
//! swapchain leaves behind.

use super::predicted;
use crate::app::App;
use crate::timing::Stages;
use crate::trace::Topic;

impl App {
    /// Rule 3 for `Window::superseded`: a frame taken back is a frame the trace says was.
    pub(super) fn said_taken_back(&self) {
        self.trace.say(
            Topic::Frames,
            format_args!(
                "the frame being drawn is of a magnification this view has left, so whatever of \
                 it is still to be drawn on the processor is taken back"
            ),
        );
    }

    /// Says what the render thread did while nobody waited: a sharp pass that landed, and the
    /// device's scene built ahead of the next view (ADRs 0699, 1898).
    fn said_idle_work(
        &self,
        sharpened: Option<std::time::Duration>,
        built_ahead: Option<&render_raster::FrameCost>,
    ) {
        if let Some(took) = sharpened {
            // Rule 3's sentence for ADR 0699: the page on the window just became the 2×
            // rendering shown box-filtered down, and a trace that did not say so would
            // show a colour shift with no cause.
            self.trace.say(
                Topic::Frames,
                format_args!(
                    "sharpened: the settled view at 2x, {:.1} ms on the render thread",
                    took.as_secs_f64() * 1e3
                ),
            );
        }
        if let Some(built) = built_ahead {
            // The next frame of these pages is priced as one that keeps the device's scene, and a
            // trace that did not say why would show a cheaper prediction with no cause (ADR 1898).
            self.trace.say(
                Topic::Frames,
                format_args!(
                    "built ahead: the device's scene of the page on hand, {:.1} ms on the render \
                     thread while nobody waited ({:.1} of it the scene, {} uploads)",
                    built.total.as_secs_f64() * 1e3,
                    built.scene.as_secs_f64() * 1e3,
                    built.uploads
                ),
            );
        }
    }

    /// Takes whatever the render thread has finished, and records what it is.
    ///
    /// Everything a landed frame says about itself is said here, once: the launch timeline's
    /// scene mark, the note a device refusal earns, the §8.7.4.5.2 programs it drew from the
    /// grid, and the view it settled — which is what the next reprojection composes against.
    pub(super) fn adopt(&mut self, now: std::time::Instant, stages: &mut Stages) {
        let Some(window) = self.device_window() else {
            return;
        };
        let landed = window.collect();
        let sharpened = window.sharpened();
        let built_ahead = window.built_ahead();
        self.said_idle_work(sharpened, built_ahead.as_ref());
        let Some(landed) = landed else {
            return;
        };
        // What a refused frame cost, in its two halves: the device's attempt is what the memory of
        // refusals saves, the processor's draw is what the interrupt can shorten (ADR 1821).
        if let Some(problem) = &landed.fell_back {
            let tried = landed.device_tried.map_or_else(
                || landed.not_asked().to_owned(),
                |tried| format!("the device refused in {:.1} ms", tried.as_secs_f64() * 1e3),
            );
            let drew = match (landed.abandoned, landed.processor_drew) {
                (true, Some(drew)) => format!(
                    "the processor was taken back after {:.1} ms",
                    drew.as_secs_f64() * 1e3
                ),
                (true, None) => "the processor was not started".to_owned(),
                (false, Some(drew)) => {
                    format!(
                        "the processor drew it in {:.1} ms",
                        drew.as_secs_f64() * 1e3
                    )
                }
                (false, None) => "the processor did not draw it".to_owned(),
            };
            self.trace.say(
                Topic::Frames,
                format_args!("refused frame: {tried}; {drew} — {problem}"),
            );
        }
        self.say_the_lane(&landed);
        // **An abandoned frame is answered to nobody and settles nothing** — `adopt_composed`'s
        // rule, for its reason (ADR 0657, trap 20): no pixels of it exist, so recording it as the
        // base would reproject a picture that was never drawn. The refusal it learned is kept by
        // the window all the same.
        if landed.abandoned {
            return;
        }
        stages.gpu = landed.cost;
        // The first frame's scene translation is a launch milestone — the other half, with
        // interpretation, of what used to sit unnamed between `document joined` and `first
        // present` (ADR 0332). Marked when the frame *lands* rather than when it was asked for,
        // because the render is somebody else's thread now and this is the moment it is known: the
        // frame began `total` before now, and the scene was the first `scene` of that.
        //
        // A clock that will not go back that far is a machine whose monotonic clock started inside
        // this frame, which is not a state — but a subtraction that could saturate silently is,
        // so `now` stands in and the mark reads as an instant rather than as a wrong one.
        self.launch.scene_built(
            now.checked_sub(landed.cost.total).unwrap_or(now),
            landed.cost.scene,
        );
        // ADR 0376: a §8.7.4.5.2 program the device declined draws from the grid instead — the
        // right picture, four orders of magnitude slower — so the ground is said out loud rather
        // than left to a timing to imply.
        for ground in &landed.function_refusals {
            self.trace.say(
                Topic::Frames,
                format_args!("function shading drawn on the processor's grid: {ground}"),
            );
        }
        // A page drawn by the slower of two backends is a fact about this build worth saying out
        // loud: said, it makes a report a sentence rather than a mystery (ADR 0125). Said on the
        // terminal and in the title when the answer changes and not at every frame (ADR 1466,
        // ADR 1821): a zoom gesture over a refused page draws a frame per magnification it stops
        // at, and the person needs the sentence once. Every refusal is still in the trace.
        // **A title is one line**, and a refusal in the adapter's words is often several: `wgpu`
        // writes a refused pipeline as a chain of causes, one per line (ADR 1962). The title takes
        // the first, which names what was refused; the terminal and the trace keep all of it.
        let on_the_processor = match (&landed.fell_back, &landed.chose) {
            (Some(problem), _) => Some(format!(
                "drawn on the processor: the graphics device {}",
                problem.lines().next().unwrap_or(problem)
            )),
            (None, Some(chose)) => Some(format!("drawn on the processor: {}", predicted(chose))),
            (None, None) if landed.finished_ahead => {
                Some("drawn on the processor while it was read".to_owned())
            }
            (None, None) => None,
        };
        if on_the_processor != self.on_the_processor {
            if let Some(problem) = &landed.fell_back {
                // The adapter's words last, so that a refusal of several lines ends the sentence
                // rather than splitting it (ADR 1962).
                println!(
                    "note: the page was drawn on the processor instead, because the graphics \
                     device {problem}"
                );
            } else if let Some(chose) = &landed.chose {
                println!(
                    "note: the page was drawn on the processor, the faster of the two here: {}",
                    predicted(chose)
                );
            } else if landed.finished_ahead {
                println!("note: the page was drawn on the processor while it was read");
            }
            self.on_the_processor = on_the_processor;
            self.retitle();
            self.retitle_incomplete();
        }
        if let Some(problem) = &landed.refused {
            eprintln!("note: this page could not be drawn: {problem}");
        }
        // What the window is now able to draw, and what producing it cost. The next view change
        // reads both: the placement to carry the pixels *from*, and the cost to predict whether
        // the frame after it will miss its refresh (`doc/todo/37` rule 5, ADR 0384).
        //
        // **Whether the frame *built* its picture is part of that**, and it is raster's own
        // observable rather than an inference from a small duration: a frame that replayed a
        // retained encode (ADR 0351) says what a replay costs and nothing about what the next
        // render will, and a view change never replays.
        if !landed.pages.is_empty() {
            // A frame finished from the page drawn while it was read cost its last part, which
            // says no more about the next render than a replay does (trap 143, ADR 1855).
            let built = !landed.finished_ahead
                && !matches!(
                    landed.cost.encode_source,
                    Some(raster_gpu::EncodeSource::Replayed)
                );
            self.stale.settled(&landed.pages, landed.drew, built);
        }
    }

    /// Rule 3 for the lane a frame took (ADRs 1831, 1886): what was predicted on each, and what
    /// the lane taken then cost.
    fn say_the_lane(&self, landed: &crate::renderer::Landed) {
        if landed.finished_ahead {
            self.trace.say(
                Topic::Frames,
                format_args!(
                    "page drawn while it was read, finished on the processor in {:.1} ms",
                    landed.drew.as_secs_f64() * 1e3
                ),
            );
        }
        // A frame the device kept says so too, so that a zoom's notches each name their lane.
        if landed.chose.is_none()
            && landed.fell_back.is_none()
            && !landed.finished_ahead
            && let Some(forecast) = &landed.predicted
        {
            let scene = if landed.cost.scene_kept {
                "drawing the scene it held"
            } else {
                "building its scene"
            };
            self.trace.say(
                Topic::Frames,
                format_args!(
                    "device chosen: {}; it drew in {:.1} ms, {scene}",
                    predicted(forecast),
                    landed.drew.as_secs_f64() * 1e3
                ),
            );
        }
        let Some(chose) = &landed.chose else {
            return;
        };
        let drew = match (landed.abandoned, landed.processor_drew) {
            (true, _) => "it was taken back".to_owned(),
            (false, None) if landed.finished_ahead => {
                "it was finished from the page drawn while it was read".to_owned()
            }
            (false, Some(drew)) => format!("it drew in {:.1} ms", drew.as_secs_f64() * 1e3),
            (false, None) => "it did not draw".to_owned(),
        };
        self.trace.say(
            Topic::Frames,
            format_args!("processor chosen: {}; {drew}", predicted(chose)),
        );
    }

    /// Says what the program did about something the device reported that no call returned.
    ///
    /// **The half that was missing when the project owner's viewer aborted.** The device's own
    /// sentence was printed by the handler, and then nothing said what became of it — so a person
    /// reading the output could not tell whether the program had noticed. Two sentences, two
    /// authors: `render-raster`'s handler says what the device said, and this says what was done.
    ///
    /// `refused` is whether the present that provoked it went on to refuse, because those are two
    /// different reports: an error under a present that succeeded is a fact about the device that
    /// cost this frame nothing, and one under a present that refused is this frame's cause.
    pub(super) fn device_reported(&mut self, said: &render_raster::Uncaptured, refused: bool) {
        if refused {
            // The refusal itself is reported by the arm that classifies it, which has the state
            // raster named and can therefore say something this cannot; a second sentence here
            // would be the same event twice.
            self.trace.say(
                Topic::Frames,
                format_args!(
                    "{} uncaptured device error(s) arrived with this frame's refusal",
                    said.since
                ),
            );
            return;
        }
        // Loud rather than traced: nothing refused, so no other line will mention this at all,
        // and an error the device raised on its own is exactly the thing that goes unnoticed
        // until a window stops updating.
        println!(
            "note: the graphics device reported {} error(s) that no call returned; the frame was \
             presented anyway, so this is being carried on past deliberately",
            said.since
        );
    }

    /// Says, once, that this window's swapchain is not coming back — and what a person can do.
    ///
    /// **A sentence rather than a crash, and a sentence rather than a silent fallback.** The
    /// processor path exists and could draw here, but taking it without being asked would make
    /// this a different program from the one `CLAUDE.md` describes: page one goes to the graphics
    /// device by the project owner's decision, and a run that quietly stopped using it would hide
    /// exactly the fault a person needs to see. So the two things that *are* offered are named.
    pub(super) fn swapchain_refused(&mut self, why: &str) {
        if self.frames.refused_swapchain {
            return;
        }
        self.frames.refused_swapchain = true;
        eprintln!("{why}");
        eprintln!(
            "This window cannot present again until it is restarted. --cpu opens no graphics \
             driver at all and draws every page on the processor."
        );
    }
}
