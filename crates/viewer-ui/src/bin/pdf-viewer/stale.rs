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
//! | 1. never the last word | [`Stale::plan`] refuses to redraw the view already approximated, [`MustFollow`] cannot be dropped without arming the clock that asks for the frame replacing it, and `about_to_wait` will not let the loop come to rest on one |
//! | 2. nothing that judges a picture ever sees one | this module is a **private module of a binary**: no library, no gate, no oracle, no harness can link to it, and nothing below it knows a reprojection exists |
//! | 3. it says so | the frame line's outcome word is `approximated`, [`Stale::count`] is what the summary prints — and since ADR 0385 every *refusal* says so too, by name and by kind ([`Refusal`]) |
//! | 4. it costs the real frame nothing | the pixels come from the encode quorra has **already** retained (a replay, never an encode), they are read back **once** per real frame rather than once per reprojection ([`Base`]), and [`Stale::affordable`] requires that standing in buy a whole refresh of the frame it delays |
//! | 5. it does not fire when it is not needed | [`Stale::missed`] — the frame did not land inside the surface's own refresh, which is the owner's word *miss* and the presenter's own measurement |
//!
//! # A base outlives the frame it was captured from, and that is ADR 0385
//!
//! **The pixels are this host's own `Arc<[u8]>` and there is no reason to throw them away.** They
//! were a field of [`Settled`] until the five-hundred-and-fiftieth session, which made the
//! invariant below easy to state and cost the feature two view changes of every run: a real frame
//! landing dropped the base, the next view change asked the device to read the window back, and
//! quorra had nothing to replay because that frame had repacked its glyph atlas (ADR 0384 section 6). The
//! window then showed **nothing moved at all**, for want of a *capture* — while the previous
//! rendering's pixels, of the same page, at a placement this file still knew, were sitting in
//! memory a line away.
//!
//! So the base is [`Stale`]'s and carries **the page it is of and the placement it was drawn at**.
//! The invariant is unchanged and is now the base's own rather than its owner's: a reprojection is
//! composed against the placement *of the pixels it resamples*, which is why [`Stale::composed`]
//! reads that placement off the base and why no caller can supply one. A base is unusable when
//! there has never been one, when the page changed, or when the window did — and for nothing else.
//!
//! # Both rules are the cadence's now, and neither is a number this project chose
//!
//! **They were, and it cost the project owner two reports of the same sentence.** The shape is
//! worth keeping written down, because it is the same mistake twice at two scales.
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
//! frame is less than what a readback costs, so the second run refused six view changes of
//! fifteen with reprojections of 6 to 16 ms against frames of 58 to 156. **A ratio nobody had
//! measured, in the way of a decision about two things that are both measured.**
//!
//! What replaces both is the thing the owner asked for — *"we should still try to render a correct
//! image every frame, but if we miss, we should interpolate"* — and the unit is the display's:
//!
//! - **rule 5**: a **miss** is a frame that does not land inside the cadence's own period. That
//!   number is [`crate::cadence`]'s, it is the surface's where the surface states one, and it
//!   needs no calibration, no bootstrap and no sample.
//! - **rule 4**: standing in must **buy at least one refresh** — `reprojection + period ≤ frame` —
//!   because a period is the smallest difference this display can show. [`Stale::affordable`].
//!
//! Neither has a constant in it. ADR 0384.
//!
//! # A reprojection may follow a reprojection, and the shape of that is the whole of `doc/todo/36`
//!
//! The project owner allows one explicitly — *"even if the last frame was already incorrect"* —
//! and **how** it does it is what decides whether the picture degrades. Resampling an image that
//! was itself resampled compounds the blur, so nothing here ever does: the pixels a reprojection
//! draws are always [`Base`], the **last real frame's own raster**, and the transform is always
//! composed against the placement *that* frame was drawn at. Ten reprojections in a row are ten
//! single resamples of true pixels rather than a chain of ten, and it is structural rather than
//! careful — a [`Base`] lives inside the [`Settled`] frame it was captured from, so recording a
//! new real frame drops the old base and there is no expression in this file that can resample a
//! resampling.
//!
//! **A late frame re-bases** for the same reason and by the same mechanism: when a delayed frame
//! finally lands, [`Stale::settled`] records it and the first reprojection standing in for it
//! reads it back, so the base becomes that frame's pixels at that frame's placement and the next
//! reprojection composes against it even though the view has moved on. What ADR 0385 changed is
//! only what happens when that readback cannot be had: the *older* base stands, composed against
//! its own placement, rather than nothing being drawn.
//!
//! # Rule 2 is structural, and this is the whole of the argument
//!
//! Everything that makes an approximate picture is in this file, and this file is a module of
//! `pdf-viewer`'s binary. A binary crate is not a dependency: `pdf-model`'s corpus and oracle
//! gates, `viewer-core`'s headless harness, `Query::Frame`, `render_at`, `viewer-confined`'s
//! worker and every diagnostic artefact in this tree are compiled without a line of it. The one
//! thing that crossed into a library is [`render_quorra::QuorraPresenter::capture_presented`],
//! and what that hands back is *the real frame the window is showing* — a readback, with no
//! notion of a reprojection anywhere in it.
//!
//! # What a revealed edge shows
//!
//! A scroll or a zoom out moves pixels off the region the old raster covered, and there is
//! nothing true to put there. The reprojection draws the window's own background — the medium
//! the presenter clears to — and **not** page white: white would be an assertion that the page
//! is blank there, which is the plausible-looking lie principle 1 is about, while the medium is
//! what this window shows wherever it has no page. ADR 0378.

use std::sync::Arc;
use std::time::Duration;

use pdf_render::{
    BlendMode, Command, DisplayList, Image, ImageSource, Raster, RasterFormat, Size, TargetSpec,
    Transform,
};

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
    /// Stand in for the frame this view is waiting for, with the pixels already held.
    ///
    /// **The transform is the view being asked for and not a composition**, since ADR 0385. What
    /// the pixels have to be carried *from* is the placement of whichever base is drawn, and which
    /// base that is depends on whether the readback below this decision succeeds — so the
    /// composition is [`Stale::composed`]'s, taken off the base itself at the moment it is drawn.
    Reproject(Transform),
    /// Draw the real frame, and say nothing: this was not a view change.
    Render,
    /// Draw the real frame, and say which of [`Refusal`]'s two kinds this was.
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
    /// *Impossible.* No pixels of this page are held and none can be read back here.
    NoPixels,
    /// *Impossible.* The placement the held pixels were drawn at does not invert, or the
    /// composition is not a finite affine — so nothing carries them onto this view.
    NoPlacement,
    /// *Impossible.* This window has no graphics device, so there is no presented frame to read
    /// back: `doc/todo/37`'s remaining surface, and the run-level refusal it produces today.
    NoDevice,
    /// *Impossible.* The device declined the approximated frame itself.
    ///
    /// A readback the device declined is deliberately **not** here, and the distinction is ADR
    /// 0385's: that is a reason this frame's own pixels could not be *had*, which the trace says
    /// where it happens, and it refuses nothing on its own — the base already held may still
    /// stand. What refuses is [`Stale::reproject`] finding no usable base, and that is
    /// [`Self::NoPixels`].
    DeviceRefused(String),
    /// *Unwise.* Rule 5: the frame this view is waiting for lands inside one refresh, so it *is*
    /// the frame every refresh the owner asked for and there is nothing to stand in for.
    InsideTheRefresh {
        /// What the frame this view is waiting for is expected to cost. Zero before any frame
        /// has *built* a picture, which is the state that has measured nothing rather than a
        /// prediction of nothing.
        frame: Duration,
        /// One refresh of this surface.
        period: Duration,
    },
    /// *Unwise.* Rule 4: standing in would not buy a whole refresh of the frame it delays.
    TooDear {
        /// What a reprojection has cost on this machine, at its worst.
        reprojection: Duration,
        /// What the frame this view is waiting for is expected to cost.
        frame: Duration,
        /// One refresh of this surface, which is what standing in has to gain.
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
            | Self::NoPixels
            | Self::NoPlacement
            | Self::NoDevice
            | Self::DeviceRefused(_) => "impossible",
            Self::InsideTheRefresh { .. } | Self::TooDear { .. } => "unwise",
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
            Self::NoPixels => formatter.write_str(
                "no pixels of this page are held and none can be read back from this window",
            ),
            Self::NoPlacement => formatter.write_str(
                "the placement the pixels held were drawn at does not invert onto this view",
            ),
            Self::NoDevice => formatter.write_str(
                "this window has no graphics device, so its pixels could only be had by drawing \
                 the page again — which is the cost this exists to hide (doc/todo/37)",
            ),
            Self::DeviceRefused(problem) => {
                write!(
                    formatter,
                    "the device refused the approximated frame: {problem}"
                )
            }
            Self::InsideTheRefresh { frame, period } => write!(
                formatter,
                "this frame is expected to take {:.1} ms against a {:.1} ms refresh, so it lands \
                 inside one and is itself the frame every refresh that was asked for",
                ms(*frame),
                ms(*period)
            ),
            Self::TooDear {
                reprojection,
                frame,
                period,
            } => write!(
                formatter,
                "one costs {:.1} ms here and this frame is expected to take {:.1}, so standing in \
                 would not gain the {:.1} ms refresh it delays the real frame by",
                ms(*reprojection),
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
        matches!(self, Self::Reproject(_))
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

/// The pixels one real frame put on the window, kept so that every reprojection resamples them.
///
/// **This is what makes a reprojection compose rather than chain**, and since ADR 0385 it says so
/// itself: the page these pixels are of and the placement they were drawn at travel *with* them,
/// so a base is a complete statement — "this window, showing this page, placed here" — rather than
/// a field whose meaning came from the frame record it hung off.
///
/// That is what lets it outlive the frame that produced it. It used to be a field of [`Settled`],
/// which made the invariant easy to state and threw the pixels away the moment a new real frame
/// landed; when the readback for that new frame then failed — the atlas repack of ADR 0384 section 6,
/// twice in every one of the owner's runs — there was nothing left to draw and the window did not
/// move. Now the older base stands, under a transform composed against *its* placement.
///
/// It is read back **once** per real frame — `doc/todo/36`'s first unsettled question, answered
/// by amortisation rather than by a cheaper readback: ADR 0378 measured 19.2 to 35.9 ms for the
/// replay-and-map, which is more than a tick, so the second and every later reprojection of the
/// same base pays none of it. The samples are held as the `Arc` `render_quorra`'s resource cache
/// keys an upload by, so they cross the bus once as well.
#[derive(Debug)]
struct Base {
    /// The window's own device pixels, straight-alpha RGBA8, top row first.
    pixels: Arc<[u8]>,
    width: u32,
    height: u32,
    /// The page these pixels are of, by the `Arc` that makes its address mean something.
    page: Arc<DisplayList>,
    /// Where that page was placed when they were drawn, in that window's own device pixels.
    placement: Transform,
}

impl Base {
    /// The base a captured frame makes, or `None` for a raster this cannot read.
    ///
    /// [`RasterFormat`] is `#[non_exhaustive]`, so a second layout can arrive without this file
    /// changing, and drawing bytes under the wrong interpretation would put a plausible-looking
    /// wrong picture on the screen — precisely what this module is not allowed to do, even in its
    /// own approximate register. Checked here, once, rather than on every reprojection.
    fn of(raster: &Raster, frame: &Settled) -> Option<Self> {
        if raster.format != RasterFormat::Rgba8 || raster.width == 0 || raster.height == 0 {
            return None;
        }
        Some(Self {
            pixels: raster.data.as_slice().into(),
            width: raster.width,
            height: raster.height,
            page: Arc::clone(&frame.page),
            placement: frame.target.transform,
        })
    }
}

/// The view one frame drew: which page's display list, and placed where.
///
/// **What it cost is deliberately not here**, and it used to be. A cost belongs to the *machine*
/// and not to a placement — the question rule 5 asks is what the next render will take, and the
/// answer outlives any one frame's pixels. Keeping it here made every re-base overwrite the
/// prediction, including a re-base by a frame that had only replayed an encode. See
/// [`Stale::building`] and ADR 0384.
///
/// **Its pixels are not here either, since ADR 0385** — see [`Base`] for why they outlive it.
#[derive(Debug)]
struct Settled {
    /// The page, by the `Arc` that makes its address mean something — the identity
    /// `render-quorra` reuses a scene by, for the same ABA reason (ADR 0351).
    page: Arc<DisplayList>,
    /// Where it was placed, in this window's own device pixels.
    target: TargetSpec,
    /// Whether this frame's own pixels have been read back into [`Stale::base`].
    ///
    /// `false` until the first reprojection stands in for it — nothing is captured for a frame no
    /// view change ever stands in for, which is every frame of a window nobody is touching.
    captured: bool,
}

/// Whether the window is showing a reprojection, and what it would take to draw the next one.
#[derive(Debug, Default)]
pub(crate) struct Stale {
    /// The last frame that was the real thing.
    settled: Option<Settled>,
    /// The view the reprojection on the screen depicts, or `None` when the screen is a rendering.
    ///
    /// **A transform rather than a flag, since `doc/todo/36`.** The owner allows a reprojection to
    /// follow a reprojection, so "one is showing" is no longer a reason to refuse; what is a
    /// reason is that the one showing already depicts the view being asked for, which is a
    /// question about *which* view rather than about whether there is one.
    showing: Option<Transform>,
    /// The most expensive reprojection this run has drawn, which is what rule 4 is checked
    /// against. The worst rather than the last: rule 4 is a bound, and a bound reads the worst
    /// case. `None` until one has been drawn, and **that state permits rather than refuses** —
    /// see [`Stale::affordable`].
    measured: Option<Duration>,
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
    /// The pixels every reprojection resamples, and the page and placement they are of.
    ///
    /// Outlives the [`Settled`] it was captured from, which is ADR 0385. `None` until the first
    /// reprojection of the run has paid for one.
    base: Option<Base>,
    /// Whether the window may be read back again on this machine.
    ///
    /// **A refusal to *capture*, and it was read as a refusal to *reproject* for two sessions.**
    /// A capture that re-encodes has cost a whole frame of exactly the work the reprojection
    /// exists to hide, so rule 4 says it must not happen twice — but that says nothing about
    /// pixels this host is already holding, and [`Self::plan`] no longer treats it as though it
    /// did. ADR 0385.
    captures_refused: bool,
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

    /// **Rule 5.** Whether the frame this view is waiting for is expected to miss the cadence.
    ///
    /// The owner's own word: *"we should still try to render a correct image every frame, but if
    /// we miss, we should interpolate"*. A miss is a frame that does not land inside one refresh
    /// of the surface, so the comparison is against [`crate::cadence::Cadence::period`] and
    /// against nothing this module invented. It needs no calibration and no first sample, which
    /// is exactly what the bar it replaced could not do without.
    fn missed(&self, period: Duration) -> bool {
        self.expected() > period
    }

    /// **Rule 4.** Whether standing in gains enough to be worth delaying the truth for.
    ///
    /// A reprojection answers the input at what it costs and pushes the real frame back by the
    /// same amount, so what it *buys* is the difference between the two. **It has to buy at least
    /// one whole refresh**, because that is the smallest difference this display can show: a
    /// stand-in that arrives less than a period before the frame it stands in for has put a wrong
    /// picture on the screen in exchange for nothing anybody can see.
    ///
    /// ```text
    /// what a reprojection costs here  +  one refresh  ≤  what this frame will cost
    /// ```
    ///
    /// **There is no ratio in it, and there used to be.** ADR 0378 read `doc/todo/37`'s "within a
    /// small fraction of the frame it replaces" as a tenth, which is a number this project chose
    /// rather than measured — and the owner's second trace is what it cost: reprojections of 6 to
    /// 16 ms were refused against frames of 58 to 156, because a tenth of those frames is less
    /// than what a readback costs on any real device. Six view changes of fifteen showed nothing.
    /// The rule above says *which* fraction and why, and the unit is the surface's rather than
    /// ours; on that machine it admits every one of the six and still refuses churn.
    ///
    /// **Unmeasured permits.** Rule 5 has already established that the frame will miss its
    /// refresh, the first reprojection is the only way this machine's number can ever be learned,
    /// and it is one frame — measured, reported by name, and binding on every one after it. A
    /// bound that refuses until it has a measurement it can only obtain by not refusing is not a
    /// bound; it is an off switch. That is the defect ADR 0384 exists for and it is not to be
    /// reintroduced in another shape.
    fn affordable(&self, frame: Duration, period: Duration) -> bool {
        self.measured
            .is_none_or(|worst| worst.saturating_add(period) <= frame)
    }

    /// What to do about this view change, and why.
    ///
    /// **The order is the audit ADR 0385 records, and one line of it is not a refusal at all.**
    /// The first question is whether this frame is a *view change*: a window whose picture already
    /// depicts what is being asked for gets [`Plan::Render`] and no line, because that is every
    /// frame of a document nobody is touching and a trace of them would say nothing. Everything
    /// after it is a genuine refusal on a genuine view change, and every one of them speaks.
    ///
    /// `period` is the surface's own refresh, which is what rules 5 and 4 are both measured
    /// against.
    pub(crate) fn plan(
        &self,
        page: &Arc<DisplayList>,
        target: TargetSpec,
        period: Duration,
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
        // Not a view change. Two ways to be one and the same answer to both: the window is showing
        // a rendering of this view — a redraw quorra replays for the price of a replay (ADR 0351)
        // — or it is showing the approximation of it that rule 1 forbids drawing twice. Either
        // way the picture asked for is the picture up, and there is nothing to stand in for.
        if settled.target.transform == target.transform || self.showing == Some(target.transform) {
            return Plan::Render;
        }
        // A resize changes what the window is as well as where the page is in it, and what is
        // held is that window's whole picture — see [`Refusal::Resized`] for why the chrome in it
        // makes this an impossibility rather than the revealed edge under another name.
        if settled.target.width != target.width || settled.target.height != target.height {
            return Plan::Refused(Refusal::Resized);
        }
        // Rule 5: a frame the machine delivers inside one refresh *is* the frame every refresh
        // the owner asked for, so there is nothing to stand in for.
        if !self.missed(period) {
            return Plan::Refused(Refusal::InsideTheRefresh {
                frame: self.expected(),
                period,
            });
        }
        // Rule 4, and it is a separate question from rule 5 on purpose. Rule 5 says the frame will
        // be late; this says whether standing in for it buys a refresh anybody could see.
        if !self.affordable(self.expected(), period) {
            return Plan::Refused(Refusal::TooDear {
                reprojection: self.measured.unwrap_or_default(),
                frame: self.expected(),
                period,
            });
        }
        // **The one refusal about pixels, and it asks the question ADR 0385 corrected.** What was
        // asked here for two sessions was whether a *capture* was still permitted, which refused a
        // run whose base was in memory the whole time. What is asked now is whether there is
        // anything to draw: a base already held, or the possibility of reading one back. The rest
        // — is it this page, does it invert — belongs to the base and is asked in
        // [`Self::reproject`], because until the readback below has been tried this cannot know
        // which base it will draw.
        if self.base.is_none() && self.captures_refused {
            return Plan::Refused(Refusal::NoPixels);
        }
        Plan::Reproject(target.transform)
    }

    /// The transform that carries the base's own pixels onto `view`.
    ///
    /// **This one expression is the whole of `doc/todo/36`'s "compose, do not chain"**, and since
    /// ADR 0385 it reads the placement off the [`Base`] rather than off the last frame record.
    /// Whatever the window is showing, and whichever rendering the pixels held came from, the
    /// transform carries *those* pixels onto the view being asked for — so a run of reprojections
    /// is a run of single resamples of true pixels, and a base that outlived its frame is composed
    /// against the frame that produced it rather than against the one that replaced it.
    ///
    /// `None` where the placement does not invert, or where the composition is not finite: a
    /// coordinate that is not a finite number is not a placement, and drawing one would hand the
    /// scene boundary a value it would refuse mid-frame.
    fn composed(&self, view: Transform) -> Option<Transform> {
        let moved = self.base.as_ref()?.placement.invert()?.then(view);
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

    /// Whether the next reprojection should have the frame on the window read back.
    ///
    /// True at most once per real frame: the first reprojection standing in for it captures the
    /// base, and every later one resamples what that capture holds. **The condition is also what
    /// makes a chain impossible**, and not merely unlikely — the only moment a capture is asked
    /// for is the moment the window is showing a rendering, because a base exists for every real
    /// frame from the first reprojection of it onward.
    ///
    /// False once captures have been refused for this run, which is the difference ADR 0385 drew:
    /// that flag stops the *asking* and no longer stops the drawing.
    pub(crate) fn wants_base(&self) -> bool {
        !self.captures_refused && self.settled.as_ref().is_some_and(|frame| !frame.captured)
    }

    /// Whether this host is holding pixels of a real frame at all.
    pub(crate) fn has_base(&self) -> bool {
        self.base.is_some()
    }

    /// Keeps the pixels the last real frame put on the window, for every reprojection of it.
    ///
    /// `false` for a raster this cannot read, which is a refusal to draw rather than a failure —
    /// **and the base already held is left standing**, because a layout this host cannot read says
    /// nothing about pixels it read earlier.
    pub(crate) fn rebase(&mut self, raster: &Raster) -> bool {
        let Some(settled) = self.settled.as_mut() else {
            return false;
        };
        let Some(base) = Base::of(raster, settled) else {
            return false;
        };
        settled.captured = true;
        self.base = Some(base);
        true
    }

    /// Records that a reprojection was drawn, which view it depicts, and what the whole of it
    /// cost.
    ///
    /// The cost is the pixels *and* the frame that put them up, because that is what
    /// [`Self::affordable`] has to be a tenth of. **This is the only writer of
    /// [`Self::measured`]**, which is why it must never be what rule 5 reads: a gate whose only
    /// sample comes from passing it cannot be passed. ADR 0384.
    pub(crate) fn drawn(&mut self, view: Transform, cost: Duration) -> MustFollow {
        self.showing = Some(view);
        self.count = self.count.saturating_add(1);
        self.measured = Some(self.measured.map_or(cost, |worst| worst.max(cost)));
        MustFollow(())
    }

    /// Records that the window will not be read back again on this machine.
    ///
    /// **A refusal to capture and not a refusal to reproject** — see [`Self::captures_refused`].
    /// Whatever base is already held goes on standing in; what stops is the asking.
    pub(crate) fn refuse_captures(&mut self) {
        self.captures_refused = true;
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
    /// so it replaces the whole [`Settled`] — the base with it — and the reprojection after it
    /// composes against *this* placement.
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
            // Captured on demand, and only where a view change asks for one: a window nobody is
            // touching reads nothing back. **The base itself is not cleared here**, which is ADR
            // 0385: until this frame's own pixels can be had, the previous rendering's are still
            // true pixels of this page at a placement this file knows.
            captured: false,
        });
    }

    /// Records that this frame was not a reprojection, whatever else it was.
    ///
    /// Called for every frame that is not one — including a frame that drew nothing at all —
    /// which is what stops [`Self::showing`] from outliving the redraw that answers it, and what
    /// stops `about_to_wait`'s guard from asking for a frame that will never come.
    pub(crate) fn real(&mut self) {
        self.showing = None;
    }

    /// Whether what the window is showing is a reprojection. Rule 1's runtime witness.
    pub(crate) fn showing_approximation(&self) -> bool {
        self.showing.is_some()
    }

    /// How many reprojections this run has drawn — rule 3's count.
    pub(crate) fn count(&self) -> u64 {
        self.count
    }

    /// How many view changes it refused, and of which kind — rule 3's other count.
    pub(crate) fn refusals(&self) -> Refusals {
        self.refusals
    }

    /// The one-image frame that puts the pixels this host holds where `target` puts them.
    ///
    /// **The only way out of this module for a picture**, and the reason it is a method rather
    /// than a free function taking pixels: a caller cannot pass in a raster of its own and cannot
    /// pass in a transform of its own, so no caller can resample anything but the base and none
    /// can resample it under a placement that is not the base's. That is what makes "compose, do
    /// not chain" a property of the type instead of a rule somebody has to follow.
    ///
    /// The three ways there is no picture are the three ways a base is unusable, and they are
    /// exactly the ones ADR 0385 names: there has never been one, the page changed, or the window
    /// did. A *lost capture* is not among them, which is the whole of what that round repaired.
    pub(crate) fn reproject(
        &self,
        page: &Arc<DisplayList>,
        target: TargetSpec,
    ) -> Result<DisplayList, Refusal> {
        let Some(base) = self.base.as_ref() else {
            return Err(Refusal::NoPixels);
        };
        if !Arc::ptr_eq(&base.page, page) {
            return Err(Refusal::AnotherPage);
        }
        if base.width != target.width || base.height != target.height {
            return Err(Refusal::Resized);
        }
        let moved = self
            .composed(target.transform)
            .ok_or(Refusal::NoPlacement)?;
        Ok(reprojection(base, moved))
    }
}

/// The one-image display list that draws the last real frame's pixels where `moved` puts them.
///
/// The base holds the window's own device pixels as that frame presented them, so the list is
/// drawn under an identity target transform, exactly as this host's chrome is: the placement is
/// in window pixels and nothing about the page's space is involved.
///
/// **The samples are handed over by `Arc` rather than copied**, which is what makes the second
/// and every later reprojection of one base cost no transfer at all: `render_quorra`'s resource
/// cache is keyed by the address of exactly this allocation, so the first reprojection uploads
/// the window and the rest of them find it.
///
fn reprojection(base: &Base, moved: Transform) -> DisplayList {
    #[expect(
        clippy::cast_precision_loss,
        reason = "window dimensions are far below f32's exact integer range"
    )]
    let (width, height) = (base.width as f32, base.height as f32);
    let mut list = DisplayList::new(Size::new(width, height));
    list.push(Command::Image {
        image: ImageSource::Decoded(Image {
            width: base.width,
            height: base.height,
            data: Arc::clone(&base.pixels),
            // **Smoothed on purpose, and it is the one place this module chooses how it looks.**
            // §8.9.5.3's `/Interpolate` is about a file's own image and does not reach these
            // pixels at all; what it decides here is whether a magnified reprojection is a grid
            // of squares or a blur. A blur is what an approximation should look like — nobody
            // mistakes it for the page — and squares of four device pixels look like a rendering
            // decision somebody made.
            interpolate: true,
        }),
        // The unit square onto the window the pixels came from, then wherever the new view puts
        // it. The y scale is negative because a `Command::Image` puts the image's *top* row at
        // unit y = 1 (§8.9.5) while these are a device raster whose first row is the top one —
        // the same flip `viewer_core::transition::Frame::draw` makes, for the same reason.
        transform: Transform::scale(width, -height)
            .then(Transform::translate(0.0, height))
            .then(moved),
        alpha: 1.0,
        clip: None,
        mask: None,
        blend: BlendMode::Normal,
    });
    list
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use pdf_render::{
        Command, DisplayList, ImageSource, Raster, RasterFormat, Size, TargetSpec, Transform,
    };

    use super::{Plan, Refusal, Settled, Stale, reprojection};

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

    /// A frame that missed the refresh by a long way, which is the case the feature is for: the
    /// view moved, the page did not, and the machine will be most of a second.
    fn slow(stale: &mut Stale, page: &Arc<DisplayList>) {
        stale.settled(page, view(1.0), Duration::from_millis(700), true);
    }

    /// A raster of the size these views are drawn at, for the base a capture would give.
    fn captured() -> Raster {
        Raster {
            width: 800,
            height: 1000,
            format: RasterFormat::Rgba8,
            data: vec![0; 800 * 1000 * 4],
        }
    }

    /// A slow frame whose pixels this host is holding, which is the state every reprojection but
    /// the first of one rendering is drawn from.
    fn slow_and_captured(stale: &mut Stale, page: &Arc<DisplayList>) {
        slow(stale, page);
        assert!(stale.rebase(&captured()), "an RGBA8 raster of the window");
    }

    /// Rule 1, as `doc/todo/36` leaves it. A reprojection is never the state the window settles
    /// in: the view it already depicts is drawn rather than approximated a second time.
    #[test]
    fn a_reprojection_is_never_the_last_word() {
        let page = page();
        let mut stale = Stale::default();
        slow(&mut stale, &page);
        assert!(
            stale.plan(&page, view(1.2), REFRESH).stands_in(),
            "a slow frame and a new magnification is what this exists for"
        );
        let follow = stale.drawn(view(1.2).transform, Duration::from_millis(20));
        assert!(stale.showing_approximation());
        assert_eq!(
            stale.plan(&page, view(1.2), REFRESH),
            Plan::Render,
            "the view on the screen has been answered; drawing it again would be a window that \
             had stopped drawing the document — and it is not a refusal, so it says nothing"
        );
        drop(follow);
        // Every frame that is not one clears it, including a frame that drew nothing: the guard
        // in `about_to_wait` asks for a redraw while this holds, and a flag that could outlive
        // the redraw answering it would spin the loop.
        stale.real();
        assert!(!stale.showing_approximation());
    }

    /// Rule 5. A frame that lands inside one refresh *is* the frame every refresh the owner asked
    /// for, and a frame that does not is a miss.
    #[test]
    fn a_frame_inside_the_refresh_is_shown_and_one_outside_it_is_stood_in_for() {
        let page = page();
        let mut stale = Stale::default();
        stale.settled(&page, view(1.0), REFRESH / 2, true);
        assert_eq!(
            stale.plan(&page, view(1.2), REFRESH),
            Plan::Refused(Refusal::InsideTheRefresh {
                frame: REFRESH / 2,
                period: REFRESH,
            }),
            "a view whose frame lands inside the refresh must show that frame, and say so"
        );
        stale.settled(&page, view(1.0), REFRESH * 2, true);
        assert!(stale.plan(&page, view(1.2), REFRESH).stands_in());
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
                stale.plan(&page, view(1.2), REFRESH).stands_in(),
                "{frame} ms is {} refreshes and must be stood in for",
                cost.as_secs_f64() / REFRESH.as_secs_f64()
            );
        }
        // And the property behind it, stated directly: nothing that *gates* a reprojection may
        // depend on a measurement only a reprojection can produce.
        stale.settled(&page, view(1.0), REFRESH * 3, true);
        assert!(
            stale.plan(&page, view(1.2), REFRESH).stands_in(),
            "a run that has drawn none must still be able to draw its first"
        );
    }

    /// **The same defect at the second scale, and the owner's second trace is the case.** With
    /// rule 5 re-grounded, rule 4 became the binding constraint and was still a tenth — so
    /// reprojections costing 6 to 16 ms were refused against frames of 58 to 156, and six view
    /// changes of fifteen showed nothing at all. A tenth of a real device's frame is less than
    /// what a readback costs on it.
    ///
    /// The rule is now that standing in must buy a whole refresh, which is the smallest difference
    /// the display can show. The costs below are the owner's own, off `tmp/trace3.entwurf.txt`, at
    /// the 120 Hz cadence that run reached.
    #[test]
    fn standing_in_is_worth_it_when_it_buys_a_refresh() {
        let page = page();
        let period = Duration::from_nanos(1_000_000_000 / 120);
        let mut stale = Stale::default();
        // The worst reprojection of that run, readback included.
        let reprojection = Duration::from_micros(16_300);
        drop(stale.drawn(view(1.2).transform, reprojection));
        // Every frame a tenth refused, all of which gain far more than a refresh.
        for frame in [57.7_f64, 71.0, 90.2, 104.5, 155.5, 156.3] {
            let cost = Duration::from_secs_f64(frame / 1e3);
            stale.settled(&page, view(1.0), cost, true);
            assert!(
                stale.plan(&page, view(1.3), period).stands_in(),
                "{frame} ms: a 16.3 ms picture now instead of the truth in {frame} is what the \
                 owner asked for"
            );
        }
        // And churn is still refused: a frame that misses by less than the reprojection costs
        // gains nothing anybody can see, and it says so rather than falling silent.
        stale.settled(&page, view(1.0), reprojection, true);
        assert_eq!(
            stale.plan(&page, view(1.3), period),
            Plan::Refused(Refusal::TooDear {
                reprojection,
                frame: reprojection,
                period,
            }),
            "a refusal that is a judgement carries the numbers it judged"
        );
    }

    /// Rule 4 reads the worst reprojection rather than the last, because a bound reads the worst
    /// case — and an approximation that turned out expensive must raise the bar it has to clear
    /// rather than be repeated at the real frame's expense.
    #[test]
    fn what_a_reprojection_cost_bounds_the_next_one() {
        let page = page();
        let mut stale = Stale::default();
        let expensive = Duration::from_millis(30);
        drop(stale.drawn(view(1.2).transform, expensive));
        // A frame that misses the refresh but does not gain a whole one is shown rather than
        // stood in for.
        stale.settled(&page, view(1.0), expensive, true);
        assert!(!stale.plan(&page, view(1.3), REFRESH).stands_in());
        stale.settled(&page, view(1.0), expensive.saturating_add(REFRESH), true);
        assert!(stale.plan(&page, view(1.3), REFRESH).stands_in());
        // The worst, not the last: a cheap one after an expensive one does not lower the bound.
        drop(stale.drawn(view(1.4).transform, Duration::from_millis(3)));
        stale.settled(&page, view(1.0), expensive, true);
        assert!(!stale.plan(&page, view(1.3), REFRESH).stands_in());
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
            stale.plan(&page, view(1.2), REFRESH).stands_in(),
            "the zoom after a harmless redraw is the one the owner waited through"
        );
        // A frame that genuinely built a cheap picture *does* move it.
        stale.settled(&page, view(1.0), REFRESH / 2, true);
        assert_eq!(stale.expected(), REFRESH / 2);
        assert!(!stale.plan(&page, view(1.3), REFRESH).stands_in());
    }

    /// A different page is not this page moved, and no placement makes it one.
    ///
    /// Checked at **both** gates, which is what ADR 0385 made worth stating twice: the plan
    /// refuses on the frame record, and the base — which now outlives the frame it came from —
    /// refuses on the page it is a picture of.
    #[test]
    fn a_page_turn_is_never_reprojected() {
        let first = page();
        let second = page();
        let mut stale = Stale::default();
        slow_and_captured(&mut stale, &first);
        assert_eq!(
            stale.plan(&second, view(1.0), REFRESH),
            Plan::Refused(Refusal::AnotherPage)
        );
        assert_eq!(
            stale.plan(&second, view(1.3), REFRESH),
            Plan::Refused(Refusal::AnotherPage)
        );
        assert_eq!(
            stale.reproject(&second, view(1.3)).unwrap_err(),
            Refusal::AnotherPage,
            "a base held from the outgoing page draws nothing of the incoming one"
        );
    }

    /// A resize changes the window the pixels were captured from, and a view that did not move
    /// has nothing to stand in for — and only one of those two is a refusal.
    #[test]
    fn neither_a_resize_nor_a_still_view_is_reprojected() {
        let page = page();
        let mut stale = Stale::default();
        slow_and_captured(&mut stale, &page);
        assert_eq!(
            stale.plan(&page, view(1.0), REFRESH),
            Plan::Render,
            "the view already on the screen is drawn, not approximated — and not refused either"
        );
        let resized = TargetSpec {
            width: 900,
            ..view(1.2)
        };
        assert_eq!(
            stale.plan(&page, resized, REFRESH),
            Plan::Refused(Refusal::Resized)
        );
        assert_eq!(
            stale.reproject(&page, resized).unwrap_err(),
            Refusal::Resized,
            "the pixels held are the old window's whole picture, chrome included"
        );
    }

    /// The transform is the one that carries the old view's device pixels onto the new view's,
    /// which is what makes the picture *move* rather than merely appear.
    #[test]
    fn the_transform_carries_old_device_pixels_onto_new_ones() {
        let page = page();
        let mut stale = Stale::default();
        slow_and_captured(&mut stale, &page);
        assert!(stale.plan(&page, view(2.0), REFRESH).stands_in());
        let moved = stale
            .composed(view(2.0).transform)
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

    /// The list is one image, placed where the reprojection says, with its top row at the top.
    #[test]
    fn the_list_is_one_image_the_right_way_up() {
        let pixels = Raster {
            width: 4,
            height: 2,
            format: RasterFormat::Rgba8,
            data: vec![0; 4 * 2 * 4],
        };
        let frame = Settled {
            page: page(),
            target: view(1.0),
            captured: false,
        };
        let base = super::Base::of(&pixels, &frame).expect("an RGBA8 raster");
        let list = reprojection(&base, Transform::IDENTITY);
        let [
            Command::Image {
                image, transform, ..
            },
        ] = list.commands()
        else {
            panic!("a reprojection is one image and nothing else");
        };
        assert!(matches!(image, ImageSource::Decoded(_)));
        // The unit square's top-left corner (0, 1) is the raster's first row, and it belongs at
        // the window's own top-left corner.
        let top_left = transform.apply(pdf_render::Point::new(0.0, 1.0));
        assert!(
            top_left.x.abs() < 1e-6 && top_left.y.abs() < 1e-6,
            "{top_left:?}"
        );
        let bottom_right = transform.apply(pdf_render::Point::new(1.0, 0.0));
        assert!((bottom_right.x - 4.0).abs() < 1e-6 && (bottom_right.y - 2.0).abs() < 1e-6);
    }

    /// A raster this cannot read is refused rather than drawn under a guessed layout — and the
    /// refusal is at the *capture*, so no base exists for a later reprojection to draw from.
    #[test]
    fn an_unreadable_raster_draws_nothing() {
        let empty = Raster {
            width: 0,
            height: 0,
            format: RasterFormat::Rgba8,
            data: Vec::new(),
        };
        let page = page();
        let mut stale = Stale::default();
        slow(&mut stale, &page);
        assert!(
            !stale.rebase(&empty),
            "a raster this cannot read is no base"
        );
        assert!(!stale.has_base());
        assert_eq!(
            stale.reproject(&page, view(1.2)).unwrap_err(),
            Refusal::NoPixels
        );
    }

    /// `doc/todo/36`'s second point, which is the one that decides how the picture degrades: a
    /// reprojection of a reprojection resamples the **base** and not the picture on the screen,
    /// so two in a row are two single resamples rather than a chain of two.
    ///
    /// Read off the transforms rather than off the pixels, because that is where the property
    /// lives: each composition carries a page point through the *rendering's* placement onto the
    /// view being asked for, and if the second composed against the first it would not.
    #[test]
    fn a_reprojection_of_a_reprojection_composes_against_the_base() {
        let page = page();
        let mut stale = Stale::default();
        slow_and_captured(&mut stale, &page);
        let corner = pdf_render::Point::new(100.0, 700.0);
        let base = view(1.0).transform;
        for magnification in [1.2_f32, 1.44, 1.728, 2.0736] {
            let asked = view(magnification);
            assert!(
                stale.plan(&page, asked, REFRESH).stands_in(),
                "the view keeps moving and the frame stays slow"
            );
            let moved = stale.composed(asked.transform).expect("a base is held");
            // Through the pixels of the last *rendering*, which is the only thing composed
            // against however many reprojections have been drawn since.
            let through = moved.apply(base.apply(corner));
            let directly = asked.transform.apply(corner);
            assert!(
                (through.x - directly.x).abs() < 1e-3 && (through.y - directly.y).abs() < 1e-3,
                "{magnification}: {through:?} {directly:?}"
            );
            drop(stale.drawn(asked.transform, Duration::from_millis(2)));
        }
    }

    /// The base is read back once per real frame and not once per reprojection, which is what
    /// makes a cadence affordable at all: ADR 0378's readback is more than a tick.
    #[test]
    fn the_base_is_captured_once_per_real_frame() {
        let page = page();
        let mut stale = Stale::default();
        slow(&mut stale, &page);
        assert!(stale.wants_base(), "the first reprojection pays for it");
        assert!(stale.rebase(&captured()));
        assert!(stale.reproject(&page, view(1.2)).is_ok());
        drop(stale.drawn(view(1.2).transform, Duration::from_millis(2)));
        assert!(
            !stale.wants_base(),
            "every later reprojection of one rendering resamples what the first captured"
        );
        assert!(stale.reproject(&page, view(1.4)).is_ok());
    }

    /// `doc/todo/36`'s third point. A delayed frame becomes the base the moment it lands, even
    /// though the view has moved on — its pixels are truer than the ones being reprojected, and
    /// the composed transform simply changes.
    #[test]
    fn a_late_frame_becomes_the_base() {
        let page = page();
        let mut stale = Stale::default();
        slow_and_captured(&mut stale, &page);
        drop(stale.drawn(view(1.2).transform, Duration::from_millis(2)));
        assert!(!stale.wants_base());
        // The frame for the 1.2× view finally lands, while the person has already asked for 2×.
        stale.settled(&page, view(1.2), Duration::from_millis(700), true);
        stale.real();
        assert!(
            stale.wants_base(),
            "the pixels of the frame that has just landed are the ones to resample now"
        );
        assert!(stale.rebase(&captured()));
        assert!(stale.plan(&page, view(2.0), REFRESH).stands_in());
        let moved = stale
            .composed(view(2.0).transform)
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

    /// **ADR 0385, and it is the project owner's own trace twice over.** A real frame landed, it
    /// had repacked the glyph atlas, and the readback for the next view change found no retained
    /// encode to replay. The window then showed nothing at all — for want of a *capture*, while
    /// the previous rendering's own pixels, of this page, at a placement this file knew, were held
    /// a line away.
    ///
    /// So a lost capture is not a refusal. The base already held stands, and the transform is
    /// composed against **the frame that produced it** rather than against the frame that
    /// replaced it — which is the same "compose, do not chain" property read at one more remove.
    #[test]
    fn a_lost_capture_reprojects_from_the_base_already_held() {
        let page = page();
        let mut stale = Stale::default();
        // The rendering at 1.0×, and the first reprojection of it captures its pixels.
        slow_and_captured(&mut stale, &page);
        drop(stale.drawn(view(1.2).transform, Duration::from_millis(12)));
        // The real frame for 1.2× lands and repacks the atlas, so nothing can be read back for it.
        stale.settled(&page, view(1.2), Duration::from_millis(156), true);
        stale.real();
        assert!(
            stale.wants_base(),
            "the newest frame's pixels are the ones worth having, and asking is nearly free"
        );
        // The capture fails — `capture_presented` answers `Ok(None)` — so `rebase` is never
        // called, and the question is what the next view change does about it.
        assert!(
            stale.plan(&page, view(1.44), REFRESH).stands_in(),
            "a base is held; nothing about a lost encode says otherwise"
        );
        let list = stale
            .reproject(&page, view(1.44))
            .expect("the pixels of the 1.0x rendering are still this page's");
        assert_eq!(list.commands().len(), 1);
        // And it is composed against the placement those pixels were drawn at — 1.0×, not the
        // 1.2× frame that landed in between and could not be read back.
        let corner = pdf_render::Point::new(100.0, 700.0);
        let moved = stale
            .composed(view(1.44).transform)
            .expect("a base is held");
        let through = moved.apply(view(1.0).transform.apply(corner));
        let directly = view(1.44).transform.apply(corner);
        assert!(
            (through.x - directly.x).abs() < 1e-3 && (through.y - directly.y).abs() < 1e-3,
            "{through:?} {directly:?}"
        );
    }

    /// The run-level refusal is a refusal to **capture** and not a refusal to draw, which is the
    /// second half of the same correction.
    ///
    /// A device that will not read its window back, or one whose readback re-encoded, says
    /// nothing about pixels this host already has. Before ADR 0385 either switched the whole
    /// feature off for the run — including for the base it was already holding.
    #[test]
    fn refusing_to_capture_does_not_refuse_the_base_already_held() {
        let page = page();
        let mut stale = Stale::default();
        slow_and_captured(&mut stale, &page);
        stale.refuse_captures();
        assert!(
            !stale.wants_base(),
            "nothing is read back again in this run"
        );
        assert!(
            stale.plan(&page, view(1.3), REFRESH).stands_in(),
            "the pixels are in memory; the device is not being asked for anything"
        );
        assert!(stale.reproject(&page, view(1.3)).is_ok());
        // With no base at all it is a refusal, and it says which kind.
        let mut empty = Stale::default();
        slow(&mut empty, &page);
        empty.refuse_captures();
        assert_eq!(
            empty.plan(&page, view(1.3), REFRESH),
            Plan::Refused(Refusal::NoPixels)
        );
    }

    /// Rule 3 over the refusals, which is what reaches the summary: every one of them is either an
    /// impossibility or a judgement, every one prints its kind, and the tally separates them.
    ///
    /// **There is deliberately no third word.** The owner named three kinds — the third being a
    /// refusal for want of something the design does not need — and one of those is what this
    /// round removed. A vocabulary that could describe it would invite the next one to be labelled
    /// rather than deleted.
    #[test]
    fn every_refusal_says_which_kind_it_is_and_is_counted_as_that_kind() {
        let judged = [
            Refusal::InsideTheRefresh {
                frame: REFRESH / 2,
                period: REFRESH,
            },
            Refusal::TooDear {
                reprojection: Duration::from_millis(16),
                frame: Duration::from_millis(20),
                period: REFRESH,
            },
        ];
        let impossible = [
            Refusal::NothingRendered,
            Refusal::AnotherPage,
            Refusal::Resized,
            Refusal::NoPixels,
            Refusal::NoPlacement,
            Refusal::NoDevice,
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
        slow_and_captured(&mut stale, &page);
        for _ in 0..100 {
            assert_eq!(
                stale.plan(&page, view(1.0), REFRESH),
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
