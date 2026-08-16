//! The frame that says it is stale: what this window shows while the real one is being built.
//!
//! When a view changes — a zoom step, a scroll — the frame for the new view takes as long as it
//! takes, and on the project owner's own drawing that is most of a second (ADR 0368). Until this
//! module existed the window showed the *previous* view's pixels, unmoved, for the whole of it,
//! and nothing said why. A **reprojection** takes the raster the window is already showing and
//! transforms it to where the new view puts it — the same pixels, moved and scaled — so the
//! window answers the input at once, and the real frame replaces it the moment it exists.
//!
//! # It is a wrong picture, drawn deliberately, and that is why the rules are here
//!
//! `CLAUDE.md`'s first principle forbids drawing something plausible instead of something true,
//! and a reprojected frame is exactly that: a raster scaled up is blurred, a scroll reveals an
//! edge the old raster has no pixels for, and anything the new view would draw that the old one
//! did not is simply absent. `doc/todo/37` states the five conditions that make it defensible,
//! and each is enforced by something here rather than promised in a comment:
//!
//! | rule | what enforces it |
//! |---|---|
//! | 1. never the last word | [`MustFollow`] cannot be dropped without arming the clock that asks for the frame replacing it, and `about_to_wait` will not let the loop come to rest on one. **`Stale::plan`'s half of this is gone and is not missed**: it used to refuse to approximate the view already approximated, so that the event thread drew the real frame instead, and since ADR 0391 there is no instead — the render is on another thread whatever this tick decides |
//! | 2. nothing that judges a picture ever sees one | this module is a **private module of a binary**: no library, no gate, no oracle, no harness can link to it, and nothing below it knows a reprojection exists |
//! | 3. it says so | the frame line's outcome word is `approximated`, [`Stale::count`] is what the summary prints — and since ADR 0385 every *refusal* says so too, by name and by kind ([`Refusal`]) |
//! | 4. it costs the real frame nothing | **structural since ADR 0391**: the render is on a thread of its own, so a reprojection is three textured quads on the *presenting* thread and takes nothing from the frame it stands in for |
//! | 5. it does not fire when it is not needed | [`Stale::missed`] — the frame did not land inside the surface's own refresh, which is the owner's word *miss*, known two ways |
//!
//! # The base is the last rendering's own texture, and that is ADR 0391
//!
//! **There is no readback here any more, and there is no capture to lose.** Until the
//! five-hundred-and-fifty-sixth session a reprojection resampled an `Arc<[u8]>` this host had read
//! back off the window — 2.7 to 6.6 ms of a refresh that is 8.333 on the owner's display, plus an
//! 8 192 000-byte re-upload behind it — and it could fail outright, because reading the window
//! back meant replaying an encode that a glyph-atlas repack had destroyed (ADR 0384 section 6, ADR
//! 0385). What the window now shows is a texture the device rendered the page into and never gave
//! up, so:
//!
//! - **the base cannot be lost.** A repack throws tile placements away; it does not touch pixels
//!   already drawn. The two refusals the owner saw twice in twenty-four presents are gone with the
//!   route that produced them, rather than handled;
//! - **the base is what [`Settled`] is.** It used to be a separate thing that outlived its frame
//!   because its frame could not always produce one. A rendering and its pixels now arrive
//!   together and are replaced together, which is one fact instead of two;
//! - **composing rather than chaining is unchanged and still structural.** [`Stale::reproject`]
//!   hands back a *placement* and no pixels at all, composed against the placement of the last
//!   **rendering** whatever is on the screen — so ten reprojections in a row are ten single
//!   resamples of true pixels, and no caller can supply either half of the composition.
//!
//! # Rule 5 is the cadence's, and rule 4 stopped being a question
//!
//! **Both were numbers this project chose, and it cost the project owner two reports of the same
//! sentence.** The shape is worth keeping written down, because it is the same mistake twice at
//! two scales.
//!
//! Rule 5 used to be `SHARE` × a *measured* reprojection cost — the strictest possible reading of
//! rule 4, until you ask where the first measurement comes from. It comes from drawing a
//! reprojection; a reprojection was drawn only above the bar; and before any measurement the bar
//! was ten times an assumed 51 ms, which is **510 ms**. On any machine quicker than the software
//! adapter the assumption was taken on it could never come down, because **its own gate blocked
//! its only sample**. The owner ran it on a real graphics device: fifteen presents, frames of 80
//! to 438 ms, not one reprojection.
//!
//! Rule 4 then became the binding constraint, still at a tenth — and a tenth of a real device's
//! frame is less than what a readback cost, so the second run refused six view changes of fifteen
//! with reprojections of 6 to 16 ms against frames of 58 to 156. It was re-grounded on the
//! display's own unit in ADR 0384: standing in had to *buy* a whole refresh, `reprojection +
//! period ≤ frame`, because a reprojection delayed the real frame by what it cost.
//!
//! **And that premise is what ADR 0391 removed.** A reprojection no longer delays anything: it
//! happens on the thread holding the surface while the render runs on another, so what it costs
//! the real frame is nothing at all rather than a small fraction. A bound on a cost that is
//! structurally zero is not a bound, so rule 4 is enforced by the *arrangement* and this file
//! carries no test of it. Deleting it is the same discipline that put it here — a number nobody
//! can measure a purpose for is a number to remove.
//!
//! What remains is the thing the owner asked for — *"we should still try to render a correct image
//! every frame, but if we miss, we should interpolate"* — with the unit the display's:
//!
//! **Rule 5: a miss is a frame that does not land inside one refresh, and it is now known two
//! ways.** By *prediction*, from what the last frame that had to build a picture cost, which is
//! what answers the very first tick of a view change before anything has been asked for; and by
//! *observation*, because a render that is still being drawn when the next tick comes round has
//! missed that refresh whatever anybody predicted. The observation needs no calibration and cannot
//! be wrong; the prediction is what keeps the first tick of a gesture from being a frozen one.
//! Neither has a constant in it. ADR 0384, extended by ADR 0391.
//!
//! # A reprojection may follow a reprojection, and the shape of that is the whole of `doc/todo/36`
//!
//! The project owner allows one explicitly — *"even if the last frame was already incorrect"* —
//! and **how** it does it is what decides whether the picture degrades. Resampling an image that
//! was itself resampled compounds the blur, so nothing here ever does: what a reprojection places
//! is always the last **rendering's** own texture, and the transform is always composed against
//! the placement *that* rendering was drawn at. Ten reprojections in a row are ten single
//! resamples of true pixels rather than a chain of ten, and it is structural rather than careful —
//! [`Stale::reproject`] hands back nothing but a placement, read off [`Settled`], so there is no
//! expression anywhere in this program that can compose against anything else.
//!
//! **A late frame re-bases** by the same mechanism: when a delayed frame lands,
//! [`Stale::settled`] records the view it drew, and the reprojection after it composes against
//! *that* placement even though the view has moved on. One field, replaced with the texture it
//! describes, which is why there is nothing to keep in step.
//!
//! # Rule 2 is structural, and this is the whole of the argument
//!
//! Everything that makes an approximate picture is in this file, and this file is a module of
//! `pdf-viewer`'s binary. A binary crate is not a dependency: `pdf-model`'s corpus and oracle
//! gates, `viewer-core`'s headless harness, `Query::Frame`, `render_at`, `viewer-confined`'s
//! worker and every diagnostic artefact in this tree are compiled without a line of it. **Nothing
//! crosses into a library at all any more**: `render-quorra` draws a window's frame into two
//! textures and hands them back, and `crate::renderer` puts them on a window under placements this
//! file computes. Neither knows what a reprojection is, and the test at the foot of this file
//! walks every `.rs` outside `viewer-ui/src/bin` to say so.
//!
//! # What a revealed edge shows
//!
//! A scroll or a zoom out moves pixels off the region the old rendering covered, and there is
//! nothing true to put there. The window shows its own background — the medium layer
//! [`crate::renderer`] puts under the page — and **not** page white: white would be an assertion
//! that the page is blank there, which is the plausible-looking lie principle 1 is about, while
//! the medium is what this window shows wherever it has no page. ADR 0378.

use std::sync::Arc;
use std::time::Duration;

use pdf_render::{DisplayList, TargetSpec, Transform};

/// What the presenter should do about a view change, and — where a person would ask — why.
///
/// **Rule 3 reaches the refusals, and ADR 0385 finishes what ADR 0384 started.** A reprojection
/// that does not happen looks exactly like a feature that does not work; the project owner said so
/// twice, of two different causes, and the second time the cause was a decision this program was
/// making in silence. So a view change that is not stood in for carries a [`Refusal`] which says
/// what was refused and of which kind, and [`Stale::declined`] both prints it and counts it.
///
/// [`Self::Render`] is deliberately *not* a refusal and says nothing: it is the answer for a frame
/// that is not a view change at all — the picture on the window already depicts what is being
/// asked for — which is every frame of a document nobody is touching.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Plan {
    /// Stand in for the frame this view is waiting for, with the rendering already held.
    ///
    /// **It carries nothing**, since ADR 0391. It used to carry the view being asked for, which
    /// the caller already has; what a stand-in actually needs is the *placement* that carries the
    /// rendering onto that view, and the only way to obtain one is [`Stale::reproject`], which
    /// composes it against the rendering itself. A payload a caller could pass on instead would
    /// be a second route to a placement, and there is deliberately one.
    Reproject,
    /// Put the rendering up as it is: it already depicts the view being asked for, so this is not
    /// a view change and there is nothing to say about it.
    Render,
    /// Put nothing up this tick, and say which of [`Refusal`]'s two kinds the reason was.
    ///
    /// **This used to mean "draw the real frame here"**, which is what the event thread then did.
    /// Since ADR 0391 the real frame is already being drawn on another thread, so what a refusal
    /// asks for is the one thing left: keep the picture the window has, keep the clock armed, and
    /// let the tick that follows carry the rendering.
    Refused(Refusal),
}

/// Why a view change was not stood in for.
///
/// **Every refusal in this program is one of two kinds, and saying which is the whole point of
/// this type.** The project owner named three when this was written — *impossible*, *unwise*, and
/// *unnecessary*, a refusal for want of something the design does not need — and there is
/// deliberately **no word here for the third**. An unnecessary refusal is a defect rather than a
/// state: the one this tree had is what ADR 0385 removed, and the next one is to be deleted the
/// same way rather than labelled. So a kind this trace can print is a refusal a reader may trust.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Refusal {
    /// *Impossible.* No frame has been rendered, so there are no pixels of this page at all.
    NothingRendered,
    /// *Impossible.* Another page: nothing about the outgoing page's pixels is true of the
    /// incoming one, at any placement.
    AnotherPage,
    /// *Impossible.* The window changed shape, and what is held is the old window's own picture —
    /// **its chrome included**, which is why this is not the revealed-edge case in disguise. A
    /// page transform moves the page; the sidebar and the scrollbar in those pixels would arrive
    /// at the old window's edges with the new window's chrome drawn over them.
    Resized,
    /// *Impossible.* The placement the held rendering was drawn at does not invert, or the
    /// composition is not a finite affine — so nothing carries it onto this view.
    NoPlacement,
    /// *Impossible.* The presenter declined to put the approximated frame on the window.
    DeviceRefused(String),
    /// *Unwise.* Rule 5: the frame this view is waiting for is expected to land inside one
    /// refresh and has not yet missed one, so it *is* the frame every refresh the owner asked
    /// for and there is nothing to stand in for.
    InsideTheRefresh {
        /// What the frame this view is waiting for is expected to cost. Zero before any frame
        /// has *built* a picture, which is the state that has measured nothing rather than a
        /// prediction of nothing.
        frame: Duration,
        /// One refresh of this surface.
        period: Duration,
    },
}

impl Refusal {
    /// Which kind of answer this is, in the word the trace prints.
    ///
    /// *Impossible* is "there is genuinely nothing true to draw"; *unwise* is a judgement between
    /// two measurements, and every one of those carries the numbers it judged.
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::NothingRendered
            | Self::AnotherPage
            | Self::Resized
            | Self::NoPlacement
            | Self::DeviceRefused(_) => "impossible",
            Self::InsideTheRefresh { .. } => "unwise",
        }
    }

    /// Whether this was a judgement rather than an impossibility, for the summary's tally.
    fn judged(&self) -> bool {
        self.kind() == "unwise"
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        /// A duration in the milliseconds the rest of the frame lines are printed in.
        fn ms(duration: Duration) -> f64 {
            duration.as_secs_f64() * 1e3
        }
        match self {
            Self::NothingRendered => formatter.write_str(
                "no frame has been rendered yet, so there are no pixels of this page to move",
            ),
            Self::AnotherPage => formatter.write_str(
                "another page — nothing about the outgoing page's pixels is true of the incoming \
                 one, at any placement",
            ),
            Self::Resized => formatter.write_str(
                "the window changed shape, and the pixels held are the old window's own picture, \
                 its chrome included — which no page transform moves",
            ),
            Self::NoPlacement => formatter.write_str(
                "the placement the rendering held was drawn at does not invert onto this view",
            ),
            Self::DeviceRefused(problem) => {
                write!(
                    formatter,
                    "the presenter refused the approximated frame: {problem}"
                )
            }
            Self::InsideTheRefresh { frame, period } => write!(
                formatter,
                "this frame is expected to take {:.1} ms against a {:.1} ms refresh and has not \
                 missed one yet, so it lands inside one and is itself the frame every refresh \
                 that was asked for",
                ms(*frame),
                ms(*period)
            ),
        }
    }
}

/// How many view changes this run refused, and of which kind — rule 3 over the refusals.
///
/// **The count reaches the summary and that is ADR 0385's half of rule 3.** ADR 0384 made one
/// refusal speak in a frame line; a person reading a trace of a session that felt frozen needs the
/// *total* as well, because a line they have to find is a line they can miss.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Refusals {
    /// Refusals where there was genuinely nothing true to draw.
    pub(crate) impossible: u64,
    /// Refusals that were a judgement between two measurements.
    pub(crate) unwise: u64,
}

impl Refusals {
    /// Every view change this run showed the real frame for rather than standing in.
    pub(crate) fn total(self) -> u64 {
        self.impossible.saturating_add(self.unwise)
    }
}

/// The one question the tests ask of a [`Plan`] that the presenter does not.
///
/// `#[cfg(test)]` because the presenter matches on the variants themselves: it has to say the
/// reason out loud, so an accessor that threw the reason away would be the wrong shape for it and
/// dead weight in the binary. A test that cares *which* refusal compares the whole value, so that
/// a refusal arriving in place of another cannot pass for it.
#[cfg(test)]
impl Plan {
    /// Whether the window is to stand in for the frame rather than wait for it.
    fn stands_in(&self) -> bool {
        matches!(self, Self::Reproject)
    }
}

/// Proof that the frame which replaces a reprojection was asked for.
///
/// **Rule 1 as a type.** A reprojection that is still on the screen when the machine goes idle is
/// a defect rather than a degradation, so the call that records one hands back a value that
/// cannot be ignored and whose only use is [`MustFollow::follow`] — arming the clock that asks
/// for the frame replacing it. Dropping it is a lint failure, which in this project is a build
/// failure.
#[must_use = "a reprojection that is not followed by a redraw is the last word, which doc/todo/37 \
              rule 1 forbids"]
pub(crate) struct MustFollow(());

impl MustFollow {
    /// Arms the clock for the real frame, which is the only thing this value is for.
    ///
    /// **The window is no longer asked directly, and that is `doc/todo/36`'s change to rule 1.**
    /// This used to call `request_redraw` here, which answered the obligation *at once* — so the
    /// real frame took the event thread for as long as it took and a view that kept moving could
    /// not be answered a second time. The obligation is unchanged and still cannot be dropped;
    /// what changed is that [`crate::cadence::Cadence`] discharges it on the surface's own tick,
    /// and `about_to_wait` refuses to let the loop rest while a reprojection is on the screen.
    pub(crate) fn follow(self, cadence: &mut crate::cadence::Cadence, now: std::time::Instant) {
        // Destructured rather than ignored: this value *is* the obligation, and consuming it is
        // the whole of what the method does.
        let Self(()) = self;
        cadence.owed(now);
    }
}

/// The rendering the window is able to draw: which page's display list, and placed where.
///
/// **This is the base**, since ADR 0391, and it is a description of a texture `crate::renderer`
/// is holding rather than a copy of its pixels. The two are replaced together, by the one call
/// that adopts a finished frame, so the placement here is always the placement of the pixels a
/// reprojection will resample and there is nothing to keep in step. Before the render moved to a
/// thread of its own this was two types — the frame record and a base of read-back samples that
/// had to outlive it, because the readback that produced one could fail (ADR 0385).
///
/// **What it cost is deliberately not here**, and it used to be. A cost belongs to the *machine*
/// and not to a placement — the question rule 5 asks is what the next render will take, and the
/// answer outlives any one frame's pixels. Keeping it here made every re-base overwrite the
/// prediction, including a re-base by a frame that had only replayed an encode. See
/// [`Stale::building`] and ADR 0384.
#[derive(Debug)]
struct Settled {
    /// The page, by the `Arc` that makes its address mean something — the identity
    /// `render-quorra` reuses a scene by, for the same ABA reason (ADR 0351).
    page: Arc<DisplayList>,
    /// Where it was placed, in this window's own device pixels.
    target: TargetSpec,
}

/// Whether the window is showing a reprojection, and what it would take to draw the next one.
#[derive(Debug, Default)]
pub(crate) struct Stale {
    /// The last rendering the window can draw, and where it put the page.
    settled: Option<Settled>,
    /// Whether what the window is showing is a reprojection rather than a rendering.
    ///
    /// **A flag again, since ADR 0391, and it was a transform for one round.** `doc/todo/36` made
    /// it a transform because rule 1 then asked *which* view was being approximated: a second
    /// reprojection of the view already on the screen was refused, so that the event thread drew
    /// the real frame instead. There is no "instead" now — the real frame is being drawn on
    /// another thread whatever this one does — and re-issuing the same three quads is what a
    /// picture every refresh *means*, so the question is only ever whether one is showing at all.
    ///
    /// Rule 1 is unchanged and is enforced by the two things that always enforced it:
    /// [`MustFollow`] cannot be dropped without arming the clock, and `about_to_wait` will not let
    /// the loop come to rest while this is true.
    showing: bool,
    /// What the last frame that had to *build* a picture cost, which is rule 5's prediction.
    ///
    /// **Deliberately not [`Settled::cost`], and the difference is a frame the owner waited
    /// through.** A frame whose page, placement, size and chrome are the ones already on the
    /// screen costs a replay of an encode that exists (ADR 0351) — two milliseconds where the
    /// render was seven hundred — and a view change never replays, because its placement is part
    /// of the key. So the previous *present* is a bad predictor of the next *render* exactly when
    /// something harmless was redrawn in between: in the owner's own trace a window focus event
    /// redrew the launch frame in 2.1 ms, and the zoom step after it was judged against that
    /// rather than against the 778.6 ms rendering it replayed. `None` until a frame has built one.
    building: Option<Duration>,
    /// How many have been drawn — rule 3's count, which the frame summary prints.
    count: u64,
    /// Rule 3 over the refusals: how many view changes showed the real frame, and of which kind.
    refusals: Refusals,
}

impl Stale {
    /// What the next frame is expected to cost, from the last one that had to build a picture.
    ///
    /// Zero before there has been one, which [`Self::plan`] reads as "nothing missed yet".
    pub(crate) fn expected(&self) -> Duration {
        self.building.unwrap_or(Duration::ZERO)
    }

    /// **Rule 5.** Whether the frame this view is waiting for has missed the cadence.
    ///
    /// The owner's own word: *"we should still try to render a correct image every frame, but if
    /// we miss, we should interpolate"*. A miss is a frame that does not land inside one refresh
    /// of the surface, so the comparison is against [`crate::cadence::Cadence::period`] and
    /// against nothing this module invented.
    ///
    /// **Two ways of knowing, and the second is the one that cannot be wrong.** `drawing` says
    /// that a render asked for at an earlier tick is *still* being drawn — so it has already
    /// missed this refresh, whatever anybody predicted, and nothing needs calibrating to see it.
    /// The prediction from [`Self::expected`] is what answers the first tick of a view change,
    /// before anything has been asked for and while there is nothing yet to observe; without it
    /// every gesture would begin with one frozen refresh.
    fn missed(&self, period: Duration, drawing: bool) -> bool {
        drawing || self.expected() > period
    }

    /// What to do about this view change, and why.
    ///
    /// **The order is the audit ADR 0385 records, and one line of it is not a refusal at all.**
    /// The first question is whether this frame is a *view change*: a window whose picture already
    /// depicts what is being asked for gets [`Plan::Render`] and no line, because that is every
    /// frame of a document nobody is touching and a trace of them would say nothing. Everything
    /// after it is a genuine refusal on a genuine view change, and every one of them speaks.
    ///
    /// `period` is the surface's own refresh, which is what rule 5 is measured against; `drawing`
    /// is whether a render asked for at an earlier tick is still out.
    pub(crate) fn plan(
        &self,
        page: &Arc<DisplayList>,
        target: TargetSpec,
        period: Duration,
        drawing: bool,
    ) -> Plan {
        let Some(settled) = self.settled.as_ref() else {
            return Plan::Refused(Refusal::NothingRendered);
        };
        // A different page is not this page moved. Nothing about the outgoing page's pixels
        // says anything true about the incoming one, at any placement — **asked before the
        // question below**, because "the picture up already depicts this view" is a claim about
        // the placement alone, and a page turn at an unchanged magnification satisfies it while
        // being the one thing this may never approximate.
        if !Arc::ptr_eq(&settled.page, page) {
            return Plan::Refused(Refusal::AnotherPage);
        }
        // Not a view change: the rendering held *is* of the view being asked for, so it goes up
        // as it is and nothing is being stood in for. This is every tick of a document nobody is
        // touching, and it says nothing.
        //
        // **The second half of this test is gone, and it was load-bearing before ADR 0391.** It
        // read `|| self.showing == Some(target.transform)` — rule 1 refusing to approximate the
        // same view twice, so that the event thread would draw the real frame instead. There is no
        // instead now: the render is on another thread whatever this one decides, and answering
        // `Render` for a view the rendering is *not* of would put the old view up unmoved, which
        // is a jump backwards on the screen. So a view that keeps being asked for keeps being
        // stood in for, at three quads a refresh, until the rendering lands — which is what a
        // picture every refresh means.
        if settled.target.transform == target.transform {
            return Plan::Render;
        }
        // A resize changes what the window is as well as where the page is in it, and what is
        // held is that window's whole picture — see [`Refusal::Resized`] for why the chrome in it
        // makes this an impossibility rather than the revealed edge under another name.
        if settled.target.width != target.width || settled.target.height != target.height {
            return Plan::Refused(Refusal::Resized);
        }
        // Rule 5: a frame the machine delivers inside one refresh *is* the frame every refresh
        // the owner asked for, so there is nothing to stand in for. **Rule 4 used to be the next
        // question and there is no next question**: a reprojection is issued by the thread holding
        // the surface while the render runs on another, so what standing in costs the real frame
        // is nothing rather than a fraction, and a bound on nothing is not a bound (ADR 0391).
        if !self.missed(period, drawing) {
            return Plan::Refused(Refusal::InsideTheRefresh {
                frame: self.expected(),
                period,
            });
        }
        Plan::Reproject
    }

    /// The transform that carries the last rendering's own pixels onto `view`.
    ///
    /// **This one expression is the whole of `doc/todo/36`'s "compose, do not chain".** Whatever
    /// the window is showing, the transform carries the pixels of the last *rendering* onto the
    /// view being asked for — so a run of reprojections is a run of single resamples of true
    /// pixels rather than a chain, and a late frame that re-based simply changes which placement
    /// is inverted here.
    ///
    /// `None` where the placement does not invert, or where the composition is not finite: a
    /// coordinate that is not a finite number is not a placement, and handing one to the presenter
    /// would earn `LayerProblem::Placement` mid-frame instead of a refusal this file can name.
    fn composed(&self, view: Transform) -> Option<Transform> {
        let moved = self.settled.as_ref()?.target.transform.invert()?.then(view);
        [moved.a, moved.b, moved.c, moved.d, moved.e, moved.f]
            .iter()
            .all(|coefficient| coefficient.is_finite())
            .then_some(moved)
    }

    /// Forgets what the window is showing, so that nothing is reprojected from it.
    ///
    /// For the frames whose pixels are not a page at a placement: §12.4.4's transition is a
    /// picture of *two* pages moving, and no transform of it is any view of either.
    pub(crate) fn forget(&mut self) {
        self.settled = None;
    }

    /// Whether any rendering of any view has landed yet.
    ///
    /// **A window that has never drawn is not a view change**, and asking [`Self::plan`] about one
    /// would earn [`Refusal::NothingRendered`] on every tick between the window appearing and the
    /// first frame landing — a refusal a second, counted in the summary, for a window that is
    /// simply still opening. So the caller asks this first and the refusal stays what it is: an
    /// answer about a *view change* that cannot be stood in for.
    pub(crate) fn has_rendering(&self) -> bool {
        self.settled.is_some()
    }

    /// Records that a reprojection was drawn and which view it depicts.
    ///
    /// **What it cost is no longer kept, and that is ADR 0391.** It was the sample rule 4 bounded
    /// the next reprojection against, back when a reprojection took the event thread away from the
    /// render it stood in for. It takes nothing from the render now, so there is nothing for a
    /// measurement of it to decide, and a number nobody can name a purpose for is a number to
    /// remove. What it costs is still *reported*, on the frame line, where a person reads it.
    pub(crate) fn drawn(&mut self) -> MustFollow {
        self.showing = true;
        self.count = self.count.saturating_add(1);
        MustFollow(())
    }

    /// Says why a view change was not stood in for, and counts it. Rule 3 over the refusals.
    ///
    /// One method rather than a `say` at each site, because the count and the sentence are the
    /// same fact seen twice: a refusal the summary counts but does not name is a number nobody can
    /// act on, and a refusal named in one frame line of six hundred is a line nobody finds.
    pub(crate) fn declined(&mut self, why: &Refusal, trace: crate::trace::Trace) {
        if why.judged() {
            self.refusals.unwise = self.refusals.unwise.saturating_add(1);
        } else {
            self.refusals.impossible = self.refusals.impossible.saturating_add(1);
        }
        trace.say(
            crate::trace::Topic::Frames,
            format_args!("no reprojection ({}): {why}", why.kind()),
        );
    }

    /// Records the view a real frame drew, and what that frame cost.
    ///
    /// **This is where a late frame re-bases** (`doc/todo/36`'s third point). Whatever the view
    /// has moved on to, the frame that has just landed is the truest picture this window holds,
    /// so it replaces the whole [`Settled`] and the reprojection after it composes against *this*
    /// placement. Since ADR 0391 the pixels are replaced by the same event — `crate::renderer`
    /// adopts the texture and this records what it is of — so a base cannot go missing.
    ///
    /// `built` says whether this frame made its picture or replayed one the device already had.
    /// **Only a frame that built one updates the prediction** ([`Self::building`]): a replay
    /// measures the replay, and rule 5 is a question about what the *next* render will cost.
    pub(crate) fn settled(
        &mut self,
        page: &Arc<DisplayList>,
        target: TargetSpec,
        cost: Duration,
        built: bool,
    ) {
        if built {
            self.building = Some(cost);
        }
        self.settled = Some(Settled {
            page: Arc::clone(page),
            target,
        });
    }

    /// Records that this frame was not a reprojection, whatever else it was.
    ///
    /// Called for every frame that is not one — including a frame that drew nothing at all —
    /// which is what stops [`Self::showing`] from outliving the redraw that answers it, and what
    /// stops `about_to_wait`'s guard from asking for a frame that will never come.
    pub(crate) fn real(&mut self) {
        self.showing = false;
    }

    /// Whether what the window is showing is a reprojection. Rule 1's runtime witness.
    pub(crate) fn showing_approximation(&self) -> bool {
        self.showing
    }

    /// How many reprojections this run has drawn — rule 3's count.
    pub(crate) fn count(&self) -> u64 {
        self.count
    }

    /// How many view changes it refused, and of which kind — rule 3's other count.
    pub(crate) fn refusals(&self) -> Refusals {
        self.refusals
    }

    /// Where the rendering this host holds goes, so that it depicts `target`.
    ///
    /// **The only way out of this module for a placement**, and the reason it is a method rather
    /// than a free function: a caller cannot pass in a placement to compose against, so no caller
    /// can carry any pixels but the last rendering's and none can carry them from anywhere but
    /// where that rendering put them. That is what makes "compose, do not chain" a property of the
    /// type instead of a rule somebody has to follow — and since ADR 0391 it is stronger still,
    /// because what comes back is a transform and the pixels never leave `crate::renderer`.
    ///
    /// The answer is in the page texture's own texel space, which is the window's: quorra's
    /// `Layer::placement` maps a layer's texels to the surface's pixels, and the identity puts
    /// texel (0, 0) at the window's corner. So a frame of the view being asked for composes to the
    /// identity, exactly as it should.
    ///
    /// The three ways there is no picture are the three ways a rendering is unusable: there has
    /// never been one, the page changed, or the window did.
    pub(crate) fn reproject(
        &self,
        page: &Arc<DisplayList>,
        target: TargetSpec,
    ) -> Result<Transform, Refusal> {
        let Some(settled) = self.settled.as_ref() else {
            return Err(Refusal::NothingRendered);
        };
        if !Arc::ptr_eq(&settled.page, page) {
            return Err(Refusal::AnotherPage);
        }
        if settled.target.width != target.width || settled.target.height != target.height {
            return Err(Refusal::Resized);
        }
        self.composed(target.transform).ok_or(Refusal::NoPlacement)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use pdf_render::{DisplayList, Size, TargetSpec, Transform};

    use super::{Plan, Refusal, Stale};

    /// A page's placement at a magnification, as `App::present` composes one: scale, y flip,
    /// translation of the page's origin in the window.
    fn view(magnification: f32) -> TargetSpec {
        TargetSpec {
            width: 800,
            height: 1000,
            transform: Transform::scale(magnification, -magnification)
                .then(Transform::translate(0.0, 842.0 * magnification)),
        }
    }

    fn page() -> Arc<DisplayList> {
        Arc::new(DisplayList::new(Size::new(595.0, 842.0)))
    }

    /// One refresh of a 60 Hz surface, which is what rule 5 compares a frame against.
    const REFRESH: Duration = Duration::from_nanos(1_000_000_000 / 60);

    /// The render thread is idle: every render this view asked for has landed.
    ///
    /// The state of the *first* tick of a view change, which is the one only the prediction can
    /// answer — nothing is out yet, so there is nothing to observe having missed.
    const LANDED: bool = false;

    /// A render asked for at an earlier tick is still being drawn, so it has missed that refresh.
    const DRAWING: bool = true;

    /// A frame that missed the refresh by a long way, which is the case the feature is for: the
    /// view moved, the page did not, and the machine will be most of a second.
    fn slow(stale: &mut Stale, page: &Arc<DisplayList>) {
        stale.settled(page, view(1.0), Duration::from_millis(700), true);
    }

    /// Rule 1. A reprojection is never the state the window settles in, and what enforces that is
    /// the obligation rather than the plan.
    ///
    /// **`Plan` used to answer `Render` for a view already approximated**, so that the event
    /// thread would draw the real frame there and then. Since ADR 0391 it answers `Reproject`
    /// again — the render is on another thread whatever this tick decides, and answering `Render`
    /// for a view the *rendering* is not of would put the old view up unmoved, which is a jump
    /// backwards on the screen. So what stops the loop resting on a stand-in is the value that
    /// cannot be dropped, and `about_to_wait`'s guard on the flag below.
    #[test]
    fn a_reprojection_is_never_the_last_word() {
        let page = page();
        let mut stale = Stale::default();
        slow(&mut stale, &page);
        assert!(
            stale.plan(&page, view(1.2), REFRESH, LANDED).stands_in(),
            "a slow frame and a new magnification is what this exists for"
        );
        let follow = stale.drawn();
        assert!(stale.showing_approximation());
        assert!(
            stale.plan(&page, view(1.2), REFRESH, DRAWING).stands_in(),
            "the same view again while the render is still out: the same three quads, which is \
             what a picture every refresh means"
        );
        // And the view the *rendering* is of goes up as it is, which is the answer that must not
        // be given to any other view.
        assert_eq!(stale.plan(&page, view(1.0), REFRESH, DRAWING), Plan::Render);
        drop(follow);
        // Every frame that is not one clears it, including a frame that drew nothing: the guard
        // in `about_to_wait` asks for a redraw while this holds, and a flag that could outlive
        // the redraw answering it would spin the loop.
        stale.real();
        assert!(!stale.showing_approximation());
    }

    /// Rule 5's prediction. A frame that lands inside one refresh *is* the frame every refresh
    /// the owner asked for, and a frame that does not is a miss.
    #[test]
    fn a_frame_inside_the_refresh_is_shown_and_one_outside_it_is_stood_in_for() {
        let page = page();
        let mut stale = Stale::default();
        stale.settled(&page, view(1.0), REFRESH / 2, true);
        assert_eq!(
            stale.plan(&page, view(1.2), REFRESH, LANDED),
            Plan::Refused(Refusal::InsideTheRefresh {
                frame: REFRESH / 2,
                period: REFRESH,
            }),
            "a view whose frame lands inside the refresh must show that frame, and say so"
        );
        stale.settled(&page, view(1.0), REFRESH * 2, true);
        assert!(stale.plan(&page, view(1.2), REFRESH, LANDED).stands_in());
    }

    /// **Rule 5's second way of knowing, and it is ADR 0391's.** A prediction can be wrong; a
    /// render that is *still being drawn* when the next tick comes round has missed that refresh
    /// and no arithmetic is involved.
    ///
    /// The case that needs it: a page whose last frame was quick, so the prediction says the next
    /// one will land in time, and then it does not. Before the render had a thread of its own
    /// there was nothing to observe — the frame ran to completion inside the tick that started it
    /// — so this state could not exist and the prediction was the only instrument there was.
    #[test]
    fn a_render_still_out_at_the_next_tick_has_missed_however_quick_the_last_one_was() {
        let page = page();
        let mut stale = Stale::default();
        stale.settled(&page, view(1.0), REFRESH / 4, true);
        assert!(
            !stale.plan(&page, view(1.2), REFRESH, LANDED).stands_in(),
            "the prediction says this one lands in time, so the first tick waits for it"
        );
        assert!(
            stale.plan(&page, view(1.2), REFRESH, DRAWING).stands_in(),
            "and the tick after it, with the render still out, has watched it miss"
        );
    }

    /// **The defect this whole scheme had, as a test.** The project owner ran the feature on a
    /// real graphics device and reported *"I don't have the impression that reprojection works"*:
    /// fifteen presents, frames of 80 to 438 ms, not one reprojection. Rule 5's bar was ten times
    /// a measured reprojection cost, the only way to measure one was to draw one, and one was only
    /// drawn above the bar — which before any measurement was ten times an assumed 51 ms.
    ///
    /// So the case that catches it is a frame that **reliably misses the refresh while staying far
    /// below half a second**, which is every frame in that trace and no frame the old bar would
    /// admit. It needs no display, no document and no graphics device: the defect was in the
    /// arithmetic, and the harness that hid it was a virtual display slow enough that its frames
    /// cleared 510 ms by accident.
    #[test]
    fn a_frame_that_misses_the_refresh_is_reprojected_however_quick_the_machine() {
        let page = page();
        let mut stale = Stale::default();
        // The owner's own distribution, in milliseconds, off `tmp/trace2.entwurf.txt`.
        for frame in [80.8_f64, 93.5, 176.2, 182.7, 191.3, 237.7, 272.0, 437.9] {
            let mut stale = Stale::default();
            let cost = Duration::from_secs_f64(frame / 1e3);
            assert!(
                cost < Duration::from_millis(510),
                "the point of the case is that it never reaches the bar that used to be there"
            );
            stale.settled(&page, view(1.0), cost, true);
            assert!(
                stale.plan(&page, view(1.2), REFRESH, LANDED).stands_in(),
                "{frame} ms is {} refreshes and must be stood in for",
                cost.as_secs_f64() / REFRESH.as_secs_f64()
            );
        }
        // And the property behind it, stated directly: nothing that *gates* a reprojection may
        // depend on a measurement only a reprojection can produce.
        stale.settled(&page, view(1.0), REFRESH * 3, true);
        assert!(
            stale.plan(&page, view(1.2), REFRESH, LANDED).stands_in(),
            "a run that has drawn none must still be able to draw its first"
        );
    }

    /// **What used to be rule 4, and what makes its absence safe.** ADR 0384 required that
    /// standing in *buy* a refresh — `reprojection + period ≤ frame` — because a reprojection ran
    /// on the event thread and pushed the real frame back by what it cost. The render is on a
    /// thread of its own now, so the premise is false and the bound is gone (ADR 0391).
    ///
    /// What the arrangement must therefore hold instead is that a reprojection is never *refused*
    /// for what it costs, at any cost the owner's own traces recorded. Those costs are the
    /// frames their reprojections stood in for, and the widest of them is a frame barely past a
    /// refresh: with rule 4 in place that one was refused and the window did not move.
    #[test]
    fn a_frame_that_misses_by_a_hair_is_still_stood_in_for() {
        let page = page();
        let period = Duration::from_nanos(1_000_000_000 / 120);
        let mut stale = Stale::default();
        // The costs the owner's second trace refused under a tenth, off `tmp/trace3.entwurf.txt`,
        // plus a frame that overruns the refresh by a fraction of a millisecond.
        for frame in [8.4_f64, 16.3, 57.7, 71.0, 90.2, 104.5, 155.5, 156.3] {
            let cost = Duration::from_secs_f64(frame / 1e3);
            stale.settled(&page, view(1.0), cost, true);
            assert!(
                stale.plan(&page, view(1.3), period, LANDED).stands_in(),
                "{frame} ms misses a 8.333 ms refresh, and standing in now costs that frame \
                 nothing at all"
            );
            drop(stale.drawn());
            stale.real();
        }
    }

    /// The prediction comes from the last frame that **built** a picture, never from one that
    /// replayed an encode the device already had (ADR 0351).
    ///
    /// **This is the owner's trace, frame for frame.** A `Focused(true)` event redrew the launch
    /// frame, quorra replayed it in 2.1 ms, and the zoom step after it was judged against that
    /// rather than against the 778.6 ms rendering the replay was a replay *of*. A replay measures
    /// the replay; rule 5 asks what the next *render* will cost, and a view change never replays.
    #[test]
    fn a_replayed_frame_does_not_speak_for_what_a_render_will_cost() {
        let page = page();
        let mut stale = Stale::default();
        let render = Duration::from_millis(778);
        stale.settled(&page, view(1.0), render, true);
        assert_eq!(stale.expected(), render);
        // The same view, redrawn for a reason that has nothing to do with the page.
        stale.settled(&page, view(1.0), Duration::from_millis(2), false);
        assert_eq!(
            stale.expected(),
            render,
            "a replay says what a replay costs and nothing about the next render"
        );
        assert!(
            stale.plan(&page, view(1.2), REFRESH, LANDED).stands_in(),
            "the zoom after a harmless redraw is the one the owner waited through"
        );
        // A frame that genuinely built a cheap picture *does* move it.
        stale.settled(&page, view(1.0), REFRESH / 2, true);
        assert_eq!(stale.expected(), REFRESH / 2);
        assert!(!stale.plan(&page, view(1.3), REFRESH, LANDED).stands_in());
    }

    /// A different page is not this page moved, and no placement makes it one.
    ///
    /// Checked at **both** gates: the plan refuses on the rendering held, and [`Stale::reproject`]
    /// refuses again on the page that rendering is of — because the two are asked at different
    /// moments and a caller that reached the second directly must not be able to get a placement.
    #[test]
    fn a_page_turn_is_never_reprojected() {
        let first = page();
        let second = page();
        let mut stale = Stale::default();
        slow(&mut stale, &first);
        assert_eq!(
            stale.plan(&second, view(1.0), REFRESH, LANDED),
            Plan::Refused(Refusal::AnotherPage)
        );
        assert_eq!(
            stale.plan(&second, view(1.3), REFRESH, LANDED),
            Plan::Refused(Refusal::AnotherPage)
        );
        assert_eq!(
            stale.reproject(&second, view(1.3)).unwrap_err(),
            Refusal::AnotherPage,
            "a rendering of the outgoing page places nothing of the incoming one"
        );
    }

    /// A resize changes the window the rendering was drawn for, and a view that did not move has
    /// nothing to stand in for — and only one of those two is a refusal.
    #[test]
    fn neither_a_resize_nor_a_still_view_is_reprojected() {
        let page = page();
        let mut stale = Stale::default();
        slow(&mut stale, &page);
        assert_eq!(
            stale.plan(&page, view(1.0), REFRESH, LANDED),
            Plan::Render,
            "the view already on the screen is drawn, not approximated — and not refused either"
        );
        let resized = TargetSpec {
            width: 900,
            ..view(1.2)
        };
        assert_eq!(
            stale.plan(&page, resized, REFRESH, LANDED),
            Plan::Refused(Refusal::Resized)
        );
        assert_eq!(
            stale.reproject(&page, resized).unwrap_err(),
            Refusal::Resized,
            "the texture held is the old window's whole picture, chrome beside it"
        );
    }

    /// The placement is the one that carries the old view's device pixels onto the new view's,
    /// which is what makes the picture *move* rather than merely appear.
    #[test]
    fn the_transform_carries_old_device_pixels_onto_new_ones() {
        let page = page();
        let mut stale = Stale::default();
        slow(&mut stale, &page);
        assert!(stale.plan(&page, view(2.0), REFRESH, LANDED).stands_in());
        let moved = stale
            .reproject(&page, view(2.0))
            .expect("a doubled magnification");
        // A point of the page, mapped both ways: through the old placement and then the
        // reprojection, and through the new placement directly.
        let corner = pdf_render::Point::new(100.0, 700.0);
        let through = moved.apply(view(1.0).transform.apply(corner));
        let directly = view(2.0).transform.apply(corner);
        assert!(
            (through.x - directly.x).abs() < 1e-3,
            "{through:?} {directly:?}"
        );
        assert!(
            (through.y - directly.y).abs() < 1e-3,
            "{through:?} {directly:?}"
        );
    }

    /// **A frame of the view being asked for composes to the identity**, which is what makes one
    /// method serve both a rendering and a stand-in.
    ///
    /// quorra's `Layer::placement` maps the layer's own texels to the surface's pixels, so the
    /// identity puts texel (0, 0) at the window's corner and one texel on one pixel — the page
    /// exactly where it was drawn. A composition that produced anything else for an unmoved view
    /// would resample a frame that needs no resampling.
    #[test]
    fn a_rendering_of_the_view_asked_for_places_at_the_identity() {
        let page = page();
        let mut stale = Stale::default();
        slow(&mut stale, &page);
        let placed = stale
            .reproject(&page, view(1.0))
            .expect("the view the rendering is of");
        for (was, expected) in [
            (placed.a, 1.0),
            (placed.b, 0.0),
            (placed.c, 0.0),
            (placed.d, 1.0),
            (placed.e, 0.0),
            (placed.f, 0.0),
        ] {
            assert!((was - expected).abs() < 1e-5, "{placed:?}");
        }
    }

    /// A placement that does not invert has no arithmetic to carry the pixels through, and it is
    /// refused rather than substituted — a degenerate composition handed to the presenter would
    /// earn quorra's own `LayerProblem::Placement` mid-frame instead of a sentence this program
    /// can print.
    #[test]
    fn a_placement_that_does_not_invert_is_refused() {
        let page = page();
        let mut stale = Stale::default();
        let flat = TargetSpec {
            transform: Transform::scale(0.0, 1.0),
            ..view(1.0)
        };
        stale.settled(&page, flat, Duration::from_millis(700), true);
        assert_eq!(
            stale.reproject(&page, view(1.2)).unwrap_err(),
            Refusal::NoPlacement
        );
    }

    /// `doc/todo/36`'s second point, which is the one that decides how the picture degrades: a
    /// reprojection of a reprojection composes against the **rendering** and not against the
    /// picture on the screen, so two in a row are two single resamples rather than a chain of two.
    ///
    /// Read off the placements rather than off the pixels, because that is where the property
    /// lives: each composition carries a page point through the *rendering's* placement onto the
    /// view being asked for, and if the second composed against the first it would not.
    #[test]
    fn a_reprojection_of_a_reprojection_composes_against_the_base() {
        let page = page();
        let mut stale = Stale::default();
        slow(&mut stale, &page);
        let corner = pdf_render::Point::new(100.0, 700.0);
        let base = view(1.0).transform;
        for magnification in [1.2_f32, 1.44, 1.728, 2.0736] {
            let asked = view(magnification);
            assert!(
                stale.plan(&page, asked, REFRESH, DRAWING).stands_in(),
                "the view keeps moving and the frame stays slow"
            );
            let moved = stale.reproject(&page, asked).expect("a rendering is held");
            // Through the pixels of the last *rendering*, which is the only thing composed
            // against however many reprojections have been drawn since.
            let through = moved.apply(base.apply(corner));
            let directly = asked.transform.apply(corner);
            assert!(
                (through.x - directly.x).abs() < 1e-3 && (through.y - directly.y).abs() < 1e-3,
                "{magnification}: {through:?} {directly:?}"
            );
            drop(stale.drawn());
        }
    }

    /// `doc/todo/36`'s third point. A delayed frame becomes the base the moment it lands, even
    /// though the view has moved on — its pixels are truer than the ones being reprojected, and
    /// the composed placement simply changes.
    ///
    /// **And there is nothing left that could stop it**, which is ADR 0391's half: the pixels and
    /// the record of what they are of are replaced by one call, so a re-base cannot half happen.
    #[test]
    fn a_late_frame_becomes_the_base() {
        let page = page();
        let mut stale = Stale::default();
        slow(&mut stale, &page);
        drop(stale.drawn());
        // The frame for the 1.2× view finally lands, while the person has already asked for 2×.
        stale.settled(&page, view(1.2), Duration::from_millis(700), true);
        stale.real();
        assert!(stale.plan(&page, view(2.0), REFRESH, LANDED).stands_in());
        let moved = stale
            .reproject(&page, view(2.0))
            .expect("the view moved on");
        let corner = pdf_render::Point::new(100.0, 700.0);
        let through = moved.apply(view(1.2).transform.apply(corner));
        let directly = view(2.0).transform.apply(corner);
        assert!(
            (through.x - directly.x).abs() < 1e-3 && (through.y - directly.y).abs() < 1e-3,
            "composed against the frame that landed, not against the one before it: \
             {through:?} {directly:?}"
        );
    }

    /// **The two refusals ADR 0385 fought are gone with the route that produced them**, and this
    /// is the property that says so rather than a comment claiming it.
    ///
    /// The owner's own trace printed `no reprojection: the device has no retained encode to
    /// replay` twice in twenty-four presents: a real frame had repacked the glyph atlas, the
    /// readback that a reprojection needed had nothing to replay, and the window did not move.
    /// There is no readback now — the pixels are the texture the device rendered into — so a run
    /// of view changes over one rendering refuses **nothing at all**, whatever the device does
    /// with its atlas in between.
    #[test]
    fn a_repacked_atlas_can_no_longer_refuse_a_view_change() {
        let page = page();
        let mut stale = Stale::default();
        slow(&mut stale, &page);
        for magnification in [1.2_f32, 1.44, 1.728, 2.0736, 2.488] {
            assert!(
                stale
                    .plan(&page, view(magnification), REFRESH, DRAWING)
                    .stands_in(),
                "{magnification}: nothing about a frame's atlas reaches a texture already drawn"
            );
            assert!(stale.reproject(&page, view(magnification)).is_ok());
            drop(stale.drawn());
        }
        assert_eq!(stale.refusals().total(), 0);
        assert_eq!(stale.count(), 5);
    }

    /// Rule 3 over the refusals, which is what reaches the summary: every one of them is either an
    /// impossibility or a judgement, every one prints its kind, and the tally separates them.
    ///
    /// **There is deliberately no third word.** The owner named three kinds — the third being a
    /// refusal for want of something the design does not need — and one of those is what ADR 0385
    /// removed. A vocabulary that could describe it would invite the next one to be labelled
    /// rather than deleted.
    #[test]
    fn every_refusal_says_which_kind_it_is_and_is_counted_as_that_kind() {
        let judged = [Refusal::InsideTheRefresh {
            frame: REFRESH / 2,
            period: REFRESH,
        }];
        let impossible = [
            Refusal::NothingRendered,
            Refusal::AnotherPage,
            Refusal::Resized,
            Refusal::NoPlacement,
            Refusal::DeviceRefused("the surface is not presentable".to_owned()),
        ];
        for why in &judged {
            assert_eq!(why.kind(), "unwise", "{why}");
            assert!(why.judged());
        }
        for why in &impossible {
            assert_eq!(why.kind(), "impossible", "{why}");
            assert!(!why.judged());
        }
        // Every one of them says something a person can act on, rather than a variant name.
        for why in judged.iter().chain(impossible.iter()) {
            let said = why.to_string();
            assert!(said.len() > 30, "{why:?} says too little: {said:?}");
            assert!(!said.contains("Refusal"), "{said:?}");
        }
        // And the tally is by kind, which is what the summary prints.
        let mut stale = Stale::default();
        let trace = crate::trace::Trace::off(std::time::Instant::now());
        for why in judged.iter().chain(impossible.iter()) {
            stale.declined(why, trace);
        }
        let refusals = stale.refusals();
        assert_eq!(refusals.unwise, judged.len() as u64);
        assert_eq!(refusals.impossible, impossible.len() as u64);
        assert_eq!(
            refusals.total(),
            (judged.len() + impossible.len()) as u64,
            "the total is what a person reads first"
        );
    }

    /// A frame that is not a view change is not a refusal, and this is the property that keeps the
    /// count above worth reading: an idle window would otherwise refuse once a frame for ever.
    #[test]
    fn a_window_nobody_is_touching_refuses_nothing() {
        let page = page();
        let mut stale = Stale::default();
        slow(&mut stale, &page);
        for _ in 0..100 {
            assert_eq!(
                stale.plan(&page, view(1.0), REFRESH, LANDED),
                Plan::Render,
                "the same view, redrawn: quorra replays it and nothing is being stood in for"
            );
        }
        assert_eq!(stale.refusals().total(), 0);
    }

    /// Rule 2, as far as a test can reach it: nothing outside this window's own binary names the
    /// reprojection, so no gate, oracle, harness or diagnostic artefact can photograph one.
    ///
    /// The structural half of the argument is that a binary crate is not a dependency — nothing
    /// in this tree *can* link to this module. This checks the other half: that no library grew
    /// its own copy of the idea while nobody was looking.
    #[test]
    fn no_library_in_this_tree_knows_what_a_reprojection_is() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crates/");
        // Everything under `src/bin` is a binary target and can be a dependency of nothing:
        // that is the boundary this test is about, rather than one directory of it.
        let mine = root.join("viewer-ui/src/bin");
        let mut found: Vec<String> = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(directory) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&directory) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|kind| kind == "rs")
                    && !path.starts_with(&mine)
                    && std::fs::read_to_string(&path)
                        .is_ok_and(|source| source.contains("reprojection"))
                {
                    found.push(path.display().to_string());
                }
            }
        }
        assert!(
            found.is_empty(),
            "a reprojection belongs to the presenter and to nothing else, but these name one: \
             {found:?}"
        );
    }
}
