//! A tick of a window that has a graphics device: adopt, ask, place.
//!
//! [`App::on_the_device`] is the frame's order (ADR 0391); the rest are the placements it ends in,
//! the present that hands them to the surface, and the frame drawn when there is no page.

use std::sync::Arc;

use pdf_render::Transform;
use viewer_core::Rendered;
use viewer_device::lane::{
    Swapchain, coverage_for, lane_for, linear_bits_of, software_adapter, swapchain,
};

use crate::app::App;
use crate::overlays::Overlays;
use crate::timing::Stages;
use crate::trace::Topic;

impl App {
    /// Everything a tick does with a window that has a graphics device: adopt, ask, place.
    ///
    /// **Three steps in this order, and the order is the design** (ADR 0391).
    ///
    /// 1. **Adopt** whatever the render thread has finished, so that the placement below is
    ///    computed against the newest rendering there is. A frame that landed a microsecond ago
    ///    must not be stood in for as though it had not.
    /// 2. **Ask** for the frame this view needs, where the rendering on hand is not of it and the
    ///    thread is idle. This is where a render *starts*, and nothing waits for it.
    /// 3. **Place** the rendering on hand under whatever transform makes it depict this view —
    ///    the identity where it already does, [`crate::stale`]'s composition where it does not,
    ///    and nothing at all where a refusal says there is nothing true to draw.
    ///
    /// `stand_in` is false for §12.4.4's transition, whose frames are pictures of *two* pages
    /// moving: no transform of one is any view of either, so the newest is put up at the identity
    /// and nothing about it is approximated.
    fn on_the_device(
        &mut self,
        pages: &[crate::stale::Placed],
        stand_in: bool,
        chrome: &Overlays,
        stages: &mut Stages,
    ) -> Option<Rendered> {
        let now = std::time::Instant::now();
        self.adopt(now, stages);
        // After the adopt and before the ask, for `on_the_processor`'s reason: a frame collected a
        // line above is no longer taking a long time. On this window the warning is about the one
        // frame it can stop — a page the device refused, being drawn on the processor (ADR 1821).
        self.mind_a_long_draw();
        let overlays = chrome.owned();
        let choice = self.coverage;
        let last_lane = self.lane;
        let self_seen = self.atlas_saw.clone();
        let drawing = {
            let window = self.device_window()?;
            // The magnification is the arrangement's rather than one page's: every page of a
            // column is placed at the same magnification by `viewer_core::layout`, so the first
            // states it — and the lane follows what the window already shows (ADR 0700).
            let coverage = lane_for(
                pages.first()?.target.transform,
                window
                    .shown()
                    .and_then(|shown| Some(shown.pages.first()?.target.transform)),
                last_lane,
                &self_seen,
                software_adapter(window.description()),
                choice,
            );
            // **Read before the ask below, and that is not an ordering accident.** Rule 5's
            // observation is "a render asked for at an *earlier* tick is out past a refresh",
            // and a render dispatched two lines further down has missed nothing yet. Reading it
            // afterwards would make every view change observe its own dispatch as a miss, and
            // the prediction — the half that decides whether a quick frame is waited for —
            // would never be asked.
            let was_out = window.out_for();
            // A rendering of exactly these lists at exactly these targets needs no successor. Each
            // page is compared by the `Arc` that makes its address mean something, because a page
            // turn at an unchanged magnification is a different picture at the same placement;
            // the targets by value, because a resize is a different frame at the same transform.
            // The *count* is compared with them: a scroll that brings a further row of a column
            // onto the screen leaves every page already up exactly where it was.
            // **And the chrome with them, since ADR 0526.** A frame is the pages *and* the
            // overlays drawn over them, and this test compared only the pages — so a window whose
            // page had not moved put up the frame it already had, and the find bar, the panel, the
            // notices card and §12.5.1's ring never reached the screen at all. Compared by value
            // because that is what a display list is: a few hundred commands against a page's tens
            // of thousands, and the comparison is what decides whether a whole frame is skipped.
            let of_this_view = window.shown().is_some_and(|shown| {
                shown.pages.len() == pages.len()
                    && shown.pages.iter().zip(pages).all(|(drawn, asked)| {
                        Arc::ptr_eq(&drawn.list, &asked.list) && drawn.target == asked.target
                    })
            }) && window.chrome_asked() == overlays;
            // **Asked before the ask, as on the processor's window** (ADR 1821): a processor draw
            // of a magnification the view has left is taken back here, and the job this clears the
            // way for is sent by the tick that collects it. And not a view the person stopped
            // drawing, for `Window::declined`'s reason.
            let mut superseded = false;
            if !of_this_view && !window.declined(pages) {
                superseded = window.superseded(pages);
                window.ask(pages.to_vec(), overlays, coverage, now);
            }
            (was_out, coverage, superseded)
        };
        if drawing.2 {
            self.said_taken_back();
        }
        // What the sticky half of `lane_for` reads next tick: the lane this view was
        // (or already had been) asked in — and, for the revisit rule, which
        // magnifications the atlas has drawn (a short ring; older entries age out as
        // the atlas's own tiles do, by being forgotten).
        self.lane = Some(drawing.1);
        if drawing.1 == raster_gpu::Coverage::Cpu
            && let Some(first) = pages.first()
        {
            let bits = linear_bits_of(first.target.transform);
            if !self.atlas_saw.contains(&bits) {
                if self.atlas_saw.len() >= 8 {
                    self.atlas_saw.remove(0);
                }
                self.atlas_saw.push(bits);
            }
        }
        if self.as_built(pages, stand_in, now) {
            return None;
        }
        // A transition is a picture of two pages moving and no transform of it is any view of
        // either, so the newest is put up at the identity and nothing about it is approximated.
        // A window that has never drawn is the other case with no stand-in to consider: it is not
        // a view change, so the policy is not asked and the tick presents whatever there is —
        // which for the ticks before the first frame lands is nothing at all.
        if !stand_in || !self.stale.has_rendering() {
            // The clock stays armed, so page one arrives on the tick after it is drawn rather
            // than waiting for an event that is not coming.
            self.cadence.owed(now);
            return self.put_up(Some(Transform::IDENTITY), &[], stages);
        }
        // Which retained pages have pixels for this view — asked before the plan, because whether
        // a page turn is a refusal or a blurred picture depends on it (ADR 0443).
        let under = self.device_window()?.underlay(pages);
        // Every rule that makes an approximation defensible is in `crate::stale` rather than
        // here. The period is the one number rule 5 is measured against (ADR 0384), and how
        // long the in-flight render has been out is its second way of knowing that a frame
        // has missed (ADR 0391).
        let planned = self.stale.plan(
            pages,
            under.len(),
            self.cadence.period(),
            drawing.0,
            // Three textured quads, issued while `crate::renderer` draws on a thread of its own:
            // they take nothing from the frame they stand in for (ADR 0391).
            crate::stale::Standing::Quads,
        );
        let stand = match planned {
            crate::stale::Plan::Render => {
                // **A frame asked for on this tick has to be collected on a later one, and this
                // is the only branch where nothing else arms the clock for it** (ADR 0526). A
                // view that has not moved plans no approximation, so `about_to_wait` rests on
                // `Wait`: the frame carrying the chrome that was just asked for would land in the
                // channel and stay there. It cannot spin — the tick that collects it finds the
                // chrome unchanged, asks for nothing, and stops arming.
                if self.device_window().is_some_and(|window| window.drawing()) {
                    self.cadence.owed(now);
                }
                return self.put_up(Some(Transform::IDENTITY), &[], stages);
            }
            crate::stale::Plan::Approximate(stand) => stand,
            crate::stale::Plan::Refused(why) => {
                let trace = self.trace;
                self.stale.declined(&why, trace);
                // Nothing is put up: the window keeps the picture it has, and the clock stays
                // armed below, so the tick that follows carries the rendering this one waited
                // for. Rule 5's whole point is that waiting one refresh beats showing a
                // resampling of a frame that was about to arrive anyway.
                self.cadence.owed(now);
                return None;
            }
        };
        // The sharp layer, where the plan says there is one. Asked through `Stale::reproject` and
        // nowhere else, so that no caller can compose against anything but the last rendering.
        let base = if stand.from.has_base() {
            match self.stale.reproject(pages) {
                Ok(carried) => {
                    // Rule 3 over the bound the placements were accepted under: a column says how
                    // far its pages disagreed, so the number the bound is set against is one a
                    // run prints rather than one this program asserts.
                    crate::stale::absorbed(carried, pages.len(), self.trace);
                    Some(carried.placement)
                }
                Err(why) => {
                    let trace = self.trace;
                    self.stale.declined(&why, trace);
                    return None;
                }
            }
        } else {
            // Rule 3: the refusal of the sharp layer is said and counted even though the window
            // is about to move, because "a low-resolution page was shown" and "nothing was shown"
            // are two answers and a person reading a page turn's trace needs to know which.
            if let Some(why) = &stand.instead_of_the_base {
                let trace = self.trace;
                self.stale.without_base(why, under.len(), trace);
            }
            None
        };
        stages.approximated = Some(stand.from);
        stages.missed = Some(stand.missed);
        self.put_up(base, &under, stages)
    }

    /// The share drawn into the title whatever is on the window, and the frame up as it stands
    /// where the window has no truer picture of these pages — the first frame, a page turn — and
    /// `true` where it is that, not anything else, this tick puts up (ADR 1831). The clock stays
    /// armed, so the next look goes up on the tick after it lands; nothing is answered to the
    /// core, because the frame that answers is the finished one.
    fn as_built(
        &mut self,
        pages: &[crate::stale::Placed],
        stand_in: bool,
        now: std::time::Instant,
    ) -> bool {
        let share = self
            .device_window()
            .and_then(|window| window.drawn_so_far());
        self.mind_the_share(share);
        if !stand_in
            || !self
                .device_window()
                .is_some_and(|window| window.shows_as_built(pages))
        {
            return false;
        }
        self.cadence.owed(now);
        self.put_up_as_built(pages);
        // A look put up is a present, though no frame line counts it, so it spends the refresh.
        self.cadence.presented(now);
        true
    }

    /// Puts the frame the processor is drawing up as it stands, where a newer look has arrived —
    /// and answers nothing to the core, because the frame that answers is the finished one
    /// (ADR 1825's rule: `first frame on the screen` stays the finished page).
    fn put_up_as_built(&mut self, pages: &[crate::stale::Placed]) {
        let Some(page) = pages
            .first()
            .map(|placed| placed.of.page().saturating_add(1))
        else {
            return;
        };
        let Some(window) = self.device_window() else {
            return;
        };
        match window.present_as_built() {
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
            // A swapchain that says "try again" is asked again at the next look; the finished
            // frame's own present is the one that answers for a surface that is gone.
            Err(problem) => self.trace.say(
                Topic::Frames,
                format_args!("page {page} as it stands was not put up: {problem}"),
            ),
        }
    }

    /// Puts the frame on hand on the window under `placement`, and says what it was.
    ///
    /// The one place a swapchain state is answered, and it is answered exactly as it was when the
    /// device owned the surface: raster's presenter reconfigures itself on a timeout or an
    /// outdated surface, so these are events to try again on rather than failures to report.
    ///
    /// `None` where nothing was put up — there is no frame yet, or the swapchain said to try
    /// again — and `None` for a reprojection as well, deliberately: the core is told what became
    /// of its request by the frame that *answers* it, and a stand-in answers nothing.
    fn put_up(
        &mut self,
        placement: Option<Transform>,
        under: &[(usize, Transform)],
        stages: &mut Stages,
    ) -> Option<Rendered> {
        let began = std::time::Instant::now();
        let expected = self.stale.expected();
        let period = self.cadence.period();
        let (outcome, cost, retained, said) = {
            let window = self.device_window()?;
            (
                window.present(placement, under),
                window.last_present(),
                window.retained(),
                // **The one place a device's uncaptured complaint is taken**, and it is here
                // rather than once a frame because this is the call that can provoke one:
                // `Surface::configure` returns `()` and says what it thought only through the
                // handler `render-raster` installs. Taken whether the present refused or not, so
                // that a failure under a present that then succeeded is not left in the record to
                // be attributed to some later frame.
                window.device_said(),
            )
        };
        if let Some(said) = &said {
            self.device_reported(said, outcome.is_err());
        }
        match outcome {
            Ok(true) => {}
            // Nothing has been drawn yet, which is every tick between the window appearing and
            // the first frame landing. Not a refusal and not a failure.
            Ok(false) => return None,
            // Swapchain states are events, not failures: nothing was presented, nothing is stale,
            // and the processor cannot help a window that is not presentable.
            Err(raster_gpu::RenderError::SurfaceUnavailable { reason }) => {
                return match swapchain(reason, said.as_ref()) {
                    Swapchain::AskAgain => {
                        self.redraw();
                        None
                    }
                    Swapchain::Wait => None,
                    Swapchain::Refused(why) => {
                        self.swapchain_refused(&why);
                        Some(Rendered::Failed(why))
                    }
                };
            }
            Err(problem) => {
                let why = crate::stale::Refusal::DeviceRefused(problem.to_string());
                let trace = self.trace;
                self.stale.declined(&why, trace);
                return None;
            }
        }
        if let Some(cost) = cost {
            stages.present = cost
                .acquire_wall
                .saturating_add(cost.record_wall)
                .saturating_add(cost.present_wall);
        }
        let Some(from) = stages.approximated else {
            // A rendering reached the window. **Whether it landed at *this* tick is deliberately
            // not asked**: a tick that put the same rendering up again has still presented one,
            // which is what the core's acknowledgement and the launch timeline both read, and
            // what the summary counts as a correct frame.
            return Some(Rendered::Presented);
        };
        // Rule 3 twice over: the frame line carries [`crate::stale::Source::word`], and this says
        // what the picture is an approximation *of* — which frame it stands in for, which layers
        // filled it, and what putting it up cost. There is no readback in it and no upload behind
        // it any more, which is why the third number this line used to carry is gone with them
        // (ADR 0391).
        self.trace.say(
            Topic::Frames,
            format_args!(
                "{}: {} against a {:.1} ms refresh, so it misses, and {} of {retained} \
                 retained low-resolution page(s) stand in under composed placements \
                 (present {:.2} ms); the real frame is being drawn",
                from.word(),
                stages
                    .missed
                    .unwrap_or(crate::stale::Missed::Predicted { frame: expected }),
                period.as_secs_f64() * 1e3,
                under.len(),
                began.elapsed().as_secs_f64() * 1e3,
            ),
        );
        // Rule 1, as a value that cannot be dropped: the frame that replaces this one is asked
        // for here, in the same expression that records it — of the clock rather than of the
        // window, so that a view still moving is answered again on the next tick.
        self.stale.drawn(from).follow(&mut self.cadence, began);
        None
    }

    /// `stages` is filled in as the frame goes: see [`Stages`] for why one number was not enough.
    pub(super) fn present(&mut self, stages: &mut Stages) -> Option<Rendered> {
        let began = std::time::Instant::now();
        // §12.3.4's list is built here and nowhere else: this is the one place that holds
        // `&mut self` and runs before the panel is drawn.
        self.fill_visible_pages();
        self.fill_collection_previews();
        let (width, height) = {
            let state = self.state.as_ref()?;
            state.size
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a panel width in pixels, which is hundreds"
        )]
        let edge = self.inset() as f32;
        let pages = self.arrangement(edge, width, height);
        let Some(first) = pages.first() else {
            // **A window with no page is a window that can still have something on it**, so this
            // does not end the frame. §7.6.4.1's card is drawn over a document that has not
            // authenticated: there is no page behind it because the document is not open, which is
            // the whole reason the card is there (ADR 0545). A piece of chrome added is a piece of
            // chrome to check on *both* surfaces (ADR 0526), and this is the third path it has to
            // reach.
            let chrome = Overlays::of(self, edge, width, height);
            stages.host = began.elapsed();
            return self.without_a_page(&chrome, (width, height), stages);
        };
        stages.page = first.of.page().saturating_add(1);
        stages.pages = pages.len();
        stages.commands = pages
            .iter()
            .map(|placed| placed.list.commands().len())
            .sum();

        // §12.4.4: a transition in flight substitutes its own picture of *two* pages for the
        // arrangement, and it is a display list, so everything below this line is unchanged by it
        // — which is the point of shaping a frame in `viewer_core::transition` rather than
        // compositing here. A transition runs in §12.4.4's presentation mode, which shows one
        // page, so it stands in for the whole arrangement rather than for one entry of it.
        let (playing, drawn) = self.frame_to_draw(&first.list, first.target, width, height);
        let transitioning = playing.is_some();
        let identity = first.of;
        let for_the_frame = match playing {
            // The transition's own picture, under the identity of the page it is a transition
            // *to* and marked as no view of it: when it lands it settles nothing, so the frame
            // after the transition is the page or the transition's end and never a reprojection
            // of two pages by one page's placement, and the retained layer never keeps it
            // (`crate::stale::Depicts`, ADR 1853).
            Some(frame) => vec![crate::stale::Placed {
                of: identity,
                list: frame,
                target: drawn,
                depicts: crate::stale::Depicts::Transition,
            }],
            None => pages,
        };

        let chrome = Overlays::of(self, edge, width, height);
        stages.host = began.elapsed();

        // A transition is already a picture of two pages moving, drawn from rasters this host
        // took for it: there is no stall to cover and the pixels on the screen are not a page.
        if transitioning {
            self.stale.forget();
        }

        if self.device_window().is_some() {
            return self.on_the_device(&for_the_frame, !transitioning, &chrome, stages);
        }
        self.on_the_processor(&for_the_frame, !transitioning, &chrome, stages)
    }

    /// The chrome alone, on a window with no page under it.
    ///
    /// One method for both surfaces because there is nothing to decide between them here: no page
    /// means no magnification to pick a coverage lane for, no retained pixels to stand in with and
    /// no view change to approximate, so all the machinery `on_the_device` and `on_the_processor`
    /// exist for has no subject. What is left is *draw these lists over the surround and present*.
    ///
    /// `None` where there is nothing to draw either, which is a window whose document has not
    /// arrived yet — every tick between the window appearing and the first frame landing, exactly
    /// as before.
    fn without_a_page(
        &mut self,
        chrome: &Overlays,
        extent: (u32, u32),
        stages: &mut Stages,
    ) -> Option<Rendered> {
        // **Nothing to draw and nothing ever drawn is the launch path**, and it stays exactly what
        // it was: the ticks between the window appearing and page one landing present nothing, so
        // no blank frame goes up in front of the first page. Once this method *has* put chrome up,
        // an empty list stops meaning "nothing to show" and starts meaning "take it away" — which
        // is what a card whose attempts have run out needs, and what the screen said otherwise: the
        // prompt stayed on the window after the program had said it was done asking.
        if chrome.lists().is_empty() && !self.drawn_without_a_page {
            return None;
        }
        self.drawn_without_a_page = true;
        let began = std::time::Instant::now();
        if self.device_window().is_some() {
            let owned = chrome.owned();
            let now = std::time::Instant::now();
            // **The frame drawn for the *previous* tick is taken here, and leaving this line out
            // is what a first run of this method looked like**: the job went to the render thread,
            // the thread drew the card, and nothing ever collected it — so every tick asked again
            // and reported *nothing to show*, for ever, at the tick rate. `on_the_device` opens
            // with the same call for the same reason.
            self.adopt(now, stages);
            let asked = {
                let chrome_coverage = coverage_for(Transform::IDENTITY, self.coverage);
                let window = self.device_window()?;
                // The same test `on_the_device` makes, with the page half of it answered by there
                // being no page: a frame of exactly these overlays over nothing needs no successor.
                let of_this_view = window.shown().is_some_and(|shown| shown.pages.is_empty())
                    && window.chrome_asked() == owned;
                if of_this_view {
                    false
                } else {
                    window.ask(Vec::new(), owned, chrome_coverage, now);
                    true
                }
            };
            // **Armed only where a frame was asked for**, which is `doc/todo/36`'s fourth rule and
            // not a detail: a frame is collected on a later tick than the one that asked for it, so
            // the clock has to stay on until it lands — and arming it unconditionally makes a
            // window with an unchanging card present at the tick rate for ever. Measured doing
            // exactly that before this line distinguished the two cases.
            if asked {
                self.cadence.owed(now);
            }
            return self.put_up(Some(Transform::IDENTITY), &[], stages);
        }
        // The composing thread's counterpart, and the same reason: a refusal it reported has to be
        // taken before anything is asked for again, or a window that cannot draw spins.
        if let Some(problem) = self.adopt_composed(stages) {
            return Some(Rendered::Failed(problem));
        }
        let overlays = chrome.lists();
        let outcome = self.composer()?.put_up_without_a_page(extent, &overlays);
        stages.present = began.elapsed();
        match outcome {
            Ok(()) => Some(Rendered::Presented),
            // Trap 5: a card a person is meant to type into that did not reach the window is
            // exactly the failure this whole item is about, so it is said rather than counted as a
            // tick with nothing on it.
            Err(problem) => Some(Rendered::Failed(format!(
                "presenting the chrome over an empty window: {problem}"
            ))),
        }
    }
}
