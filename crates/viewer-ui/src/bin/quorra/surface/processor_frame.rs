//! A tick of a window whose pictures are drawn by the processor: the same adopt, ask, place as the
//! device path, over [`crate::composer`] and its retained pages (ADR 0461).

use std::sync::Arc;

use pdf_render::Transform;
use viewer_core::{Answer, Query, Rendered};

use super::in_the_window;
use crate::app::App;
use crate::overlays::Overlays;
use crate::timing::Stages;
use crate::trace::Topic;

impl App {
    /// Everything a tick does with a window the processor draws on: adopt, ask, place.
    ///
    /// **The same three steps in the same order as [`Self::on_the_device`]** (ADR 0461), which is
    /// the whole of what giving this surface a composing thread bought: the two windows had two
    /// policies — one drew its stand-in beside the frame and the other in front of it, one had the
    /// retained pages and the other could not — and they now differ in one number, which is what a
    /// stand-in costs the thread that presents ([`crate::stale::Standing`]).
    ///
    /// `stand_in` is false for §12.4.4's transition, whose frames are pictures of *two* pages
    /// moving: no transform of one is any view of either.
    pub(super) fn on_the_processor(
        &mut self,
        pages: &[crate::stale::Placed],
        stand_in: bool,
        chrome: &Overlays,
        stages: &mut Stages,
    ) -> Option<Rendered> {
        let now = std::time::Instant::now();
        // A page that would not draw is reported before anything is asked for again, so that a
        // document this program cannot rasterise refuses once per redraw rather than spinning.
        if let Some(problem) = self.adopt_composed(stages) {
            return Some(Rendered::Failed(problem));
        }
        // **After the adopt and before the ask**, which is the only placement that says anything
        // true: a frame collected a line above is a frame that is no longer taking a long time,
        // and one asked for below has been out for no time at all. The owner's *warn* half, on
        // the tick this window already runs.
        self.mind_a_long_draw();
        let overlays = chrome.lists();
        let (out_for, superseded) = {
            let composer = self.composer()?;
            // **Read before the ask below**, and that is not an ordering accident — see
            // [`Self::on_the_device`], where the same line has the same reason.
            let was_out = composer.out_for();
            let mut superseded = false;
            // **And not a view the person stopped drawing** (`Composer::declined`). This surface
            // asks for itself once a tick for as long as the pixels on hand are not of this view,
            // so without that second question an abort would be answered by the next tick asking
            // for the same frame again — which is what it did, on camera, before the field
            // existed. A native window needs no such guard: its outstanding request is the
            // viewer's, and a token never answered is never re-issued.
            if !composer.depicts(pages) && !composer.declined(pages) {
                // **ADR 0657's rule 1, and it is asked before the ask rather than instead of
                // it.** A raise does not free the thread this instant — the command in progress
                // finishes first, measured at 1.3 to 2.1 ms against a 2.76 ms command (ADR 0650)
                // — so the job it clears the way for is sent by the tick that collects it, one
                // period later. Asking here is what makes that one period rather than the
                // remainder of a frame.
                superseded = composer.superseded(pages);
                composer.ask(pages.to_vec(), now);
            }
            (was_out, superseded)
        };
        if superseded {
            self.trace.say(
                Topic::Frames,
                format_args!(
                    "the frame being drawn is of a view this one could not stand in with, so the \
                     composing thread was interrupted and will draw this view instead"
                ),
            );
        }
        // The frame in flight goes up as it stands where the window holds no picture of these
        // pages — the first frame, a page turn — which is the device window's rule (ADR 1831).
        if stand_in && self.composer()?.shows_as_built(pages) {
            self.cadence.owed(now);
            self.put_up_composed_as_built(pages, &overlays);
            // A look put up is a present, though no frame line counts it, so it spends the refresh.
            self.cadence.presented(now);
            return None;
        }
        if !stand_in || !self.stale.has_rendering() {
            // The clock stays armed, so page one arrives on the tick after it is drawn rather than
            // waiting for an event that is not coming.
            self.cadence.owed(now);
            return self.put_up_composed(&overlays, stages);
        }
        // Which retained pages have pixels for this view — asked before the plan, because whether
        // a page turn is a refusal or a blurred picture depends on it (ADR 0443).
        let under = self.composer()?.underlay(pages);
        let planned = self.stale.plan(
            pages,
            under.len(),
            self.cadence.period(),
            out_for,
            // A resample of a window of pixels, on the thread that presents: beside the frame it
            // stands in for, as on the other surface, but not free (ADR 0461).
            crate::stale::Standing::Resample,
        );
        let stand = match planned {
            crate::stale::Plan::Render => return self.put_up_composed(&overlays, stages),
            crate::stale::Plan::Approximate(stand) => stand,
            crate::stale::Plan::Refused(why) => {
                let trace = self.trace;
                self.stale.declined(&why, trace);
                // The clock stays armed, so the tick that follows carries the frame this one
                // waited for.
                self.cadence.owed(now);
                return self.put_up_unshown(&overlays, stages);
            }
        };
        let base = if stand.from.has_base() {
            match self.stale.reproject(pages) {
                Ok(carried) => {
                    crate::stale::absorbed(carried, pages.len(), self.trace);
                    Some(carried.placement)
                }
                Err(why) => {
                    let trace = self.trace;
                    self.stale.declined(&why, trace);
                    return self.put_up_unshown(&overlays, stages);
                }
            }
        } else {
            // Rule 3: the refusal of the sharp layer is said and counted even though the window is
            // about to move, because "a low-resolution page was shown" and "nothing was shown" are
            // two answers and a person reading a page turn's trace needs to know which.
            if let Some(why) = &stand.instead_of_the_base {
                let trace = self.trace;
                self.stale.without_base(why, under.len(), trace);
            }
            None
        };
        stages.approximated = Some(stand.from);
        stages.missed = Some(stand.missed);
        self.stand_in_composed(pages, base, &under, &overlays, stages)
    }

    /// Takes whatever the composing thread has finished, and records what it is.
    ///
    /// Answers the sentence a page that would not draw produced, and `None` for every other tick —
    /// including the ones where nothing has finished, which is most of them.
    pub(super) fn adopt_composed(&mut self, stages: &mut Stages) -> Option<String> {
        let landed = self
            .composer()
            .and_then(crate::composer::Composer::collect)?;
        // **ADR 0657's rule 3, and trap 20: an abandoned draw is answered to nobody.** It is not a
        // refusal — `Rendered::Failed` would set the core's `shown` for this page and stop the
        // scheduler asking again, which is right for a page that will not rasterise and would
        // freeze one this host merely chose not to finish. It is not a settled view either: no
        // pixels of it exist, so recording it as the base would have `Stale` reproject from a
        // picture that was never drawn and feed rule 5's prediction a frame that did not land.
        //
        // **And `stages.composed` stays unset**, which is a measurement decision rather than a
        // tidy-up: it is a row of the summary's percentiles (`crate::timing::SUMMARY_ROWS`) and
        // what those describe is what a *frame* costs the composing thread. A draw stopped part
        // way through is a sample of nothing, and letting it into the distribution would pull that
        // number down by however far through the page the interrupt landed. What it cost is said
        // in a line of its own instead.
        if landed.abandoned {
            self.trace.say(
                Topic::Frames,
                format_args!(
                    "the interrupted frame came back after {:.1} ms, drawn and dropped; nothing \
                     is reported and the view it was of is not recorded as settled",
                    landed.cost.as_secs_f64() * 1e3
                ),
            );
            return None;
        }
        stages.composed = landed.cost;
        if landed.refused.is_some() {
            return landed.refused;
        }
        // What the draw of these pages left to the library's converter, told to the viewer, which
        // says each condition once and keeps it with the page's notes (ADR 2290).
        for (picture, declined) in &landed.declined {
            self.dispatch(viewer_core::Command::Declined {
                document: picture.document(),
                page: picture.page(),
                declined: declined.clone(),
            });
        }
        if landed.finished_ahead {
            self.trace.say(
                Topic::Frames,
                format_args!(
                    "page drawn while it was read, finished on the processor in {:.1} ms",
                    landed.cost.as_secs_f64() * 1e3
                ),
            );
        }
        // What the window is now able to draw, and what waiting for it cost. The next view change
        // reads both: the placement to carry the pixels *from*, and the cost to predict whether the
        // frame after it will miss its refresh (`doc/todo/37` rule 5, ADR 0384). Every frame on
        // this surface builds its picture — there is no encode to replay — so the prediction is
        // updated by all of them.
        //
        // A frame finished from the page drawn while it was read cost its last part, which says
        // no more about the next frame than a replay does (trap 143, ADR 1855).
        if !landed.pages.is_empty() {
            self.stale
                .settled(&landed.pages, landed.waited, !landed.finished_ahead);
        }
        None
    }

    /// Puts the frame the composing thread is drawing up as it stands, where a newer look has
    /// arrived — and answers nothing to the core, because the frame that answers is the finished
    /// one (ADR 1825's rule: `first frame on the screen` stays the finished page).
    fn put_up_composed_as_built(
        &mut self,
        pages: &[crate::stale::Placed],
        overlays: &[&pdf_render::DisplayList],
    ) {
        let Some(page) = pages
            .first()
            .map(|placed| placed.of.page().saturating_add(1))
        else {
            return;
        };
        let Some(composer) = self.composer() else {
            return;
        };
        match composer.put_up_as_built(overlays) {
            Ok(Some(fraction)) => {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a share in [0, 1], times a hundred"
                )]
                let percent = (fraction.clamp(0.0, 1.0) * 100.0).floor() as u32;
                self.trace.say(
                    Topic::Frames,
                    format_args!("page {page} shown as it stands, {percent} % drawn"),
                );
            }
            Ok(None) => {}
            Err(problem) => self.trace.say(
                Topic::Frames,
                format_args!("page {page} as it stands was not put up: {problem}"),
            ),
        }
    }

    /// Puts up a rendering the window is holding and has never shown, where a stand-in was refused.
    ///
    /// **What a refusal costs is not the same on the two surfaces, and this is the difference**
    /// (ADR 0461). On the device the only *judged* refusal is [`crate::stale::Refusal::InsideTheRefresh`],
    /// which says the true frame is expected within one refresh — so waiting costs one refresh and
    /// bounds itself. Here rule 4 refuses precisely when the frame is **slow**, and a frame that
    /// landed while the view was still moving is then never put up at all: the window would go on
    /// showing a picture older than the one it is holding, for as long as the gesture lasts. It
    /// was measured doing exactly that — three renderings finished and discarded inside one scroll.
    ///
    /// So the refusal keeps its meaning — *do not stand in* — and stops meaning *show nothing*. A
    /// rendering nobody has seen is the truest picture this window has, at the placement it was
    /// drawn for, which is what every viewer showed before `doc/todo/37` existed. It is a
    /// rendering rather than an approximation and is counted as one: the pixels are right, and
    /// what is old is the view they are of.
    fn put_up_unshown(
        &mut self,
        overlays: &[&pdf_render::DisplayList],
        stages: &mut Stages,
    ) -> Option<Rendered> {
        if !self.composer()?.unshown() {
            return None;
        }
        self.trace.say(
            Topic::Frames,
            format_args!(
                "the rendering on hand is of the view before this one and has not been on the \
                 window at all, so it goes up unmoved rather than being held back"
            ),
        );
        self.put_up_composed(overlays, stages)
    }

    /// Puts the frame on hand on the window as it is, under the chrome.
    ///
    /// `None` where nothing has been drawn yet, which is every tick between the window appearing
    /// and the first frame landing, and `None` where the surface refused — which is reported by
    /// name rather than counted as a frame.
    fn put_up_composed(
        &mut self,
        overlays: &[&pdf_render::DisplayList],
        stages: &mut Stages,
    ) -> Option<Rendered> {
        let began = std::time::Instant::now();
        let outcome = self.composer()?.put_up(overlays);
        stages.present = began.elapsed();
        match outcome {
            Ok(true) => Some(Rendered::Presented),
            Ok(false) => None,
            Err(problem) => Some(Rendered::Failed(format!(
                "presenting the processor's page {problem}"
            ))),
        }
    }

    /// Resamples the last frame's own pixels onto this view, over whatever retained pages have
    /// pixels for it, and puts that on the window.
    ///
    /// **Every rule that makes this defensible is `crate::stale`'s and none of them is new.** Rule
    /// 5 decided that the frame being drawn will miss the refresh; rule 4 decided that the resample
    /// finishes a refresh before it, which is a question on this surface and only on this one; rule
    /// 3 says what was shown, in the trace and in the summary's count; and rule 1 is
    /// [`crate::stale::MustFollow`], discharged of the clock exactly as the other surface
    /// discharges it.
    fn stand_in_composed(
        &mut self,
        pages: &[crate::stale::Placed],
        base: Option<Transform>,
        under: &[(usize, Transform)],
        overlays: &[&pdf_render::DisplayList],
        stages: &mut Stages,
    ) -> Option<Rendered> {
        let extent = pages
            .first()
            .map(|placed| (placed.target.width, placed.target.height))?;
        let began = std::time::Instant::now();
        let (retained, outcome) = {
            let composer = self.composer()?;
            (
                composer.retained(),
                composer.stand_in(extent, base, under, overlays),
            )
        };
        let costs = match outcome {
            Ok(costs) => costs,
            Err(problem) => {
                let why = crate::stale::Refusal::DeviceRefused(problem.to_string());
                let trace = self.trace;
                self.stale.declined(&why, trace);
                stages.approximated = None;
                return None;
            }
        };
        // `None` is a picture made of no layers at all, which the plan has already ruled out: it
        // answers `Approximate` only where the base carries or a retained page has pixels.
        let Some((resample, cost)) = costs else {
            stages.approximated = None;
            return None;
        };
        // Rule 4's only possible sample, taken from the thing itself, on the machine it ran on.
        self.stale.resampled(cost);
        stages.present = cost.saturating_sub(resample);
        let from = stages.approximated?;
        self.trace.say(
            Topic::Frames,
            format_args!(
                "{}: {} against a {:.1} ms refresh, so it misses, and {} of {retained} \
                 retained low-resolution page(s) stand in under composed placements \
                 (resample {:.2} ms, present {:.2} ms — rule 4 judges their sum); the real \
                 frame is being drawn",
                from.word(),
                stages.missed.unwrap_or(crate::stale::Missed::Predicted {
                    frame: self.stale.expected(),
                }),
                self.cadence.period().as_secs_f64() * 1e3,
                under.len(),
                resample.as_secs_f64() * 1e3,
                cost.saturating_sub(resample).as_secs_f64() * 1e3,
            ),
        );
        // Rule 1, as a value that cannot be dropped: the frame that replaces this one is asked for
        // here, of the clock rather than of the window, so that a view still moving is answered
        // again on the next tick.
        self.stale.drawn(from).follow(&mut self.cadence, began);
        None
    }

    /// Table 29's arrangement as this window is about to draw it: every page, placed.
    ///
    /// **The whole of what a tier-2 host needs in order to obey `/PageLayout`, and it needed no
    /// message.** `viewer-core` hands one [`viewer_core::RenderRequest`] per page on the screen
    /// and answers `Query::PageGeometry` for each of them; a page the arrangement no longer shows
    /// has no geometry — that question's own documentation says so — which is what says a request
    /// this host is still holding has scrolled off. So the requests kept are exactly the pages
    /// placed, and the list comes back in page order because that is the order they arrived in and
    /// the order `viewer_core::layout` sorts its placements into.
    ///
    /// Each target is the *window's* extent with the page's placement composed into its transform,
    /// which is the one thing this host adds to what the core said: the core centres and scrolls
    /// the arrangement, and the panel's edge is the host's own.
    pub(super) fn arrangement(
        &mut self,
        edge: f32,
        width: u32,
        height: u32,
    ) -> Vec<crate::stale::Placed> {
        let mut placed = Vec::with_capacity(self.requests.len());
        for request in &self.requests {
            let Answer::Geometry(geometry) = self.viewer.query(Query::PageGeometry(request.page))
            else {
                continue;
            };
            placed.push(crate::stale::Placed {
                // What a picture of this page is a picture *of*, carried from the request that
                // asked for it: the document, the page, and the state of the ink it was
                // interpreted against (ADR 0457).
                of: crate::stale::Picture::new(request.document, request.page, request.ink),
                list: Arc::clone(&request.list),
                target: in_the_window(request.target, geometry.origin, edge, (width, height)),
                depicts: crate::stale::Depicts::Page,
            });
        }
        // What the arrangement dropped is dropped here, so that a thousand-page document scrolled
        // from end to end holds one request per page on the screen rather than one per page read.
        self.requests
            .retain(|request| placed.iter().any(|placed| placed.of.page() == request.page));
        placed
    }
}
