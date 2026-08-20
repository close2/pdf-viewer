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
//! | 3. it says so | the frame line's outcome word is [`Source::word`] — which since ADR 0443 says *which* layer filled the picture, because a whole page at a fraction of the resolution is a different amount of wrong from the last frame moved — [`Stale::count`] is what the summary prints, split the same three ways, and since ADR 0385 every *refusal* says so too, by name and by kind ([`Refusal`]) |
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
//! # What a revealed edge shows, and what now shows there instead
//!
//! A scroll or a zoom out moves pixels off the region the old rendering covered. The base is a
//! picture of the *window*, so what it does not cover it has nothing to say about, and until the
//! six-hundred-and-eighth session the window put its own background there — never page white,
//! which would assert that the page is blank there, which is the plausible-looking lie principle
//! 1 is about (ADR 0378).
//!
//! **The second layer is a picture of the *page*, and that is what fills it** (ADR 0443). A
//! [`Proxies`] entry is a whole page drawn once into a raster whose longer side is
//! [`PROXY_EDGE`] pixels, produced on the render thread when it has nothing else to do, and
//! placed under the base by the same `settled⁻¹ ∘ asked` composition. The two are complementary
//! rather than competing, which is the whole argument for the construction: a whole-page picture
//! is near the right resolution exactly when the view is zoomed out, which is when the margin
//! appears, and it is badly blurred exactly when the sharp base already covers the view.
//!
//! It also answers the case this file was not looking at: **a page turn and a `GoTo` have no base
//! at all**, because nothing about the outgoing page's pixels is true of the incoming one at any
//! placement. A page whose proxy is retained has pixels there where the base has none, so
//! [`Plan::Approximate`] carries a [`Source`] saying *which* layer filled the picture — rule 3
//! grew that distinction in the same session, because "approximated from the last frame" and
//! "approximated from a low-resolution page" are different amounts of wrong.

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
    /// Stand in for the frame this view is waiting for, from the layers named.
    ///
    /// **It carries no placement**, since ADR 0391, and it carries no pixels: what a stand-in
    /// actually needs is the *placements* that carry the pictures held onto this view, and the
    /// only ways to obtain one are [`Stale::reproject`] for the base and [`Proxies::placements`]
    /// for the pages under it — each composed against the picture it is of. A payload a caller
    /// could pass on instead would be a second route to a placement, and there is deliberately
    /// one apiece.
    Approximate(Stand),
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

/// What a stand-in is made of, and what it is missing.
///
/// **Rule 3 as a value rather than as a sentence somebody remembered to print.** A frame line
/// saying only `approximated` stopped saying what it was showing the moment there were two
/// sources of approximate pixels, so the plan carries the source and the presenter cannot draw
/// one without naming it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Stand {
    /// Which layers have pixels for this view.
    pub(crate) from: Source,
    /// Why the last rendering does not carry onto this view, where it does not.
    ///
    /// `Some` is a picture drawn from the retained pages alone — a page turn, a `GoTo`, a resize,
    /// a zoom in a column — and it is still a refusal of the *sharp* layer, so it is said out loud
    /// and counted ([`Stale::without_base`]). What it is no longer is a blank window.
    pub(crate) instead_of_the_base: Option<Refusal>,
}

/// Which of the two approximate layers filled the picture — rule 3's distinction.
///
/// The order is the amount of wrong: the last frame moved is blurred by the magnification and no
/// more, the pages under it are a whole page at [`PROXY_EDGE`] pixels along its longer side, and
/// a picture made of those alone is the blurrier of the two everywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Source {
    /// The last rendering's own texture, moved — and nothing under it, because no retained page
    /// has pixels for this view.
    LastFrame,
    /// The last rendering, moved, over the retained pages that fill what it does not cover.
    LastFrameOverPages,
    /// The retained pages alone: nothing about the last rendering is true of this view.
    RetainedPages,
}

impl Source {
    /// The frame line's outcome word — rule 3, in the place a person actually reads.
    pub(crate) fn word(self) -> &'static str {
        match self {
            Self::LastFrame => "approximated",
            Self::LastFrameOverPages => "approximated, over a retained page",
            Self::RetainedPages => "approximated from a retained page",
        }
    }

    /// Whether the last rendering's own texture is one of the layers.
    pub(crate) fn has_base(self) -> bool {
        matches!(self, Self::LastFrame | Self::LastFrameOverPages)
    }
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
    /// *Impossible.* Table 29's arrangement is not one affine of itself, so no single placement
    /// carries every page of the picture held onto the view being asked for.
    ///
    /// **What produces it is a zoom in a column** (ADR 0442): `viewer_core::layout`'s gap between
    /// rows is stated in logical pixels and does not scale with the magnification, while the pages
    /// either side of it do. A scroll moves every page by the same distance and so never reaches
    /// this. The two placements are carried because a reader has to be able to see how far apart
    /// they are before deciding it matters.
    Rearranged {
        /// The placement the first page of the arrangement asked for.
        first: Transform,
        /// The placement a later page asked for, which is not the same one.
        then: Transform,
    },
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
            | Self::Rearranged { .. }
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
            Self::Rearranged { first, then } => write!(
                formatter,
                "Table 29's arrangement is not one affine of itself — the first page of the \
                 picture held moves by ({:.3} {:.3} {:.3} {:.3} {:.3} {:.3}) and a later one by \
                 ({:.3} {:.3} {:.3} {:.3} {:.3} {:.3}), and the window puts its pages up as one \
                 texture under one placement",
                first.a,
                first.b,
                first.c,
                first.d,
                first.e,
                first.f,
                then.a,
                then.b,
                then.c,
                then.d,
                then.e,
                then.f,
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
    /// Refusals of the *sharp* layer that the retained pages stood in for anyway.
    ///
    /// **Counted apart from the two above, because it answers a different question** (ADR 0443).
    /// Those two are view changes the window did not move for; this one moved, from the blurrier
    /// source, and a person reading a trace of a page turn needs to be able to tell "nothing was
    /// shown" from "a low-resolution page was shown". It is deliberately not in [`Self::total`].
    pub(crate) proxied: u64,
}

impl Refusals {
    /// Every view change this run showed the real frame for rather than standing in.
    pub(crate) fn total(self) -> u64 {
        self.impossible.saturating_add(self.unwise)
    }
}

/// How many stand-ins this run drew, split by what they were made of — rule 3's count.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Drawn {
    /// The last rendering's own texture, moved, with nothing under it.
    pub(crate) last_frame: u64,
    /// The same, over retained pages filling what it did not cover.
    pub(crate) over_pages: u64,
    /// Retained pages alone, where the last rendering said nothing about the view.
    pub(crate) pages_only: u64,
}

impl Drawn {
    /// Every stand-in, whatever it was made of.
    pub(crate) fn total(self) -> u64 {
        self.last_frame
            .saturating_add(self.over_pages)
            .saturating_add(self.pages_only)
    }

    /// Records one of `from`.
    fn record(&mut self, from: Source) {
        let counter = match from {
            Source::LastFrame => &mut self.last_frame,
            Source::LastFrameOverPages => &mut self.over_pages,
            Source::RetainedPages => &mut self.pages_only,
        };
        *counter = counter.saturating_add(1);
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
        matches!(self, Self::Approximate(_))
    }
}

/// The longer side, in device pixels, of the retained low-resolution picture of a whole page.
///
/// **Chosen from a measurement, and the measurement said the opposite of what was expected.**
/// `doc/todo/37` illustrated the idea with "an eighth of device scale" and said outright that the
/// scale was still open. What decides it is what a proxy *costs*, and on this machine's Radeon
/// 890M — `cargo run --release -p render-quorra --example zoom_frame`, sequence
/// `1.188,0.152,0.304,0.456,0.608,0.912,1.216` — **the cost is flat in the scale**:
///
/// | page | 91 × 128 | 362 × 512 | 724 × 1024 |
/// |---|---|---|---|
/// | `doc/ISO_32000-2_sponsored_EC3.pdf` p60 | 21.8 ms | 24.6 ms | 24.6 ms |
/// | `doc/PDF20_AN001-BPC.pdf` p1 (160 commands) | 4.2 ms | 1.9 ms | 3.1 ms |
///
/// and on the project owner's own witness, 58 029 commands, a sixty-four-fold range of pixels
/// moved the frame from 406.4 ms at 254 × 72 to 408.2 ms at 2027 × 576. **What a frame of a page
/// costs is walking its display list, not covering its pixels**, so buying sharpness here is free
/// in time. The binding constraint is therefore **memory**: 512 gives 362 × 512 × 4 = 741 376
/// bytes for an A4 page, against 8 192 000 for *one* of the two window textures a single frame
/// already holds, and half the linear resolution of a page filling an 800 × 1000 window — soft,
/// unmistakably a stand-in, and unmistakably the page.
///
/// **A fixed edge rather than a fraction of the current magnification, for a second reason the
/// ladder showed.** A run that drew the same page at seven scales made quorra throw its whole
/// glyph atlas away — `repacked` on the last rung — and the real frame after a repack cost 411 ms
/// where the one after a single proxy cost 82. A proxy scale that followed the zoom would put a
/// new glyph size in the atlas at every step; one fixed edge puts in one, once, per page.
pub(crate) const PROXY_EDGE: u32 = 512;

/// How many pages this host retains a low-resolution picture of, when nobody says otherwise.
///
/// **A default is a decision, so this one is chosen from a measurement rather than picked**, and
/// the measurement separates two costs that look like one. Drawing a page's picture is paid
/// **once per page shown**, whatever this number is — the render thread draws it while idle, and
/// the frame after it pays what a rebuilt scene costs instead of what a replay does:
///
/// | page | the proxy | the frame after it | that frame as a replay |
/// |---|---|---|---|
/// | `doc/ISO_32000-2_sponsored_EC3.pdf` p60 | 21.9 ms | 2.1 ms | 1.1 ms |
/// | the owner's witness, 58 029 commands | 411.0 ms | 82.0 ms | 1.9 ms |
///
/// **What *this* number costs is memory alone**, at 741 KB a page, and what it buys is not
/// drawing a page's picture a second time after an eviction — so a larger extent is strictly
/// cheaper in renders and strictly dearer in bytes. Eight is where the whole retained set, 5.9 MB
/// for A4, is still smaller than one of the two window textures a frame holds anyway: a budget
/// stated against something this program already spends rather than a round number. It covers the
/// gestures that actually land on a held page — `Left` and `Right` about where a person is, a
/// continuous column's rows, a `GoTo` back where they came from — and a `GoTo` into a thousand
/// pages is beyond any extent, which is why the curve is here rather than further out.
///
/// The price to know about is neither of the two above: **a view change arriving while a page's
/// picture is being drawn waits for it to finish**, up to 411 ms on that witness. The window
/// still answers at once, from the layers this module is about; what is late is the real frame.
/// One page per idle turn is what bounds it (`crate::renderer`). `--proxy-pages 0` turns the
/// whole of it off.
pub(crate) const PROXY_PAGES: usize = 8;

/// A whole page, drawn once at [`PROXY_EDGE`], and what it is a picture of.
///
/// Generic in its pixels so that one policy serves both threads and the tests: the render thread
/// holds `Arc<wgpu::Texture>` and so does the presenter, and neither of them is what decides which
/// pages are kept.
#[derive(Debug)]
pub(crate) struct Retained<P> {
    /// The page, by the `Arc` whose address is its identity — the same identity
    /// [`one_placement`] matches on, and for the same ABA reason (ADR 0351).
    pub(crate) page: Arc<DisplayList>,
    /// Where the whole page sits in this picture: [`proxy_target`]'s answer, kept beside the
    /// pixels rather than recomputed, so that a change to that function cannot silently re-place
    /// a raster already drawn.
    pub(crate) target: TargetSpec,
    /// Whatever holds the pixels on the thread this lives on.
    pub(crate) pixels: P,
}

/// Where a whole page goes in its own retained picture, or `None` for a page with no extent.
///
/// The same `TargetSpec::for_page` the core builds a render request with (`viewer_core::Viewer`),
/// at a smaller scale — which is what makes the composition below exact rather than approximate:
/// both are affine maps out of the same page's user space, so `proxy⁻¹ ∘ asked` is the map from
/// this raster's texels to the window's pixels and nothing has to agree about anything else.
pub(crate) fn proxy_target(list: &DisplayList) -> Option<TargetSpec> {
    let size = list.page_size;
    let longest = size.width.max(size.height);
    if !longest.is_finite() || longest <= 0.0 {
        return None;
    }
    let scale = f64::from(PROXY_EDGE) / f64::from(longest);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a ratio of two page dimensions, far inside f32's range"
    )]
    let scale = scale as f32;
    // The budget is the square of the edge and one pixel: `TargetSpec::for_page` rounds each
    // dimension **up**, so that the raster contains the page rather than clipping the fraction of
    // a row at its edge (ADR 0064), and a page whose longer side divides exactly still lands one
    // pixel over when the ratio is not exact in `f32`. Stated here so that a page of an absurd
    // aspect ratio is refused by that function rather than allocated for.
    let edge = u64::from(PROXY_EDGE).saturating_add(1);
    TargetSpec::for_page(list, scale, edge.saturating_mul(edge)).ok()
}

/// Every page this window is holding a low-resolution picture of, newest first.
///
/// **The extent is the host's setting and could not be anything else** (`doc/todo/37`, ADR 0443).
/// A stand-in is a wrong picture, so rule 2 keeps everything that makes one inside a private
/// module of a binary: the count may not become a `Command`, a field of a boundary type or
/// anything a gate, an oracle or a harness could link to. It is a flag beside `--cpu` and
/// `--backend`, and `PROXY_PAGES` is what it defaults to.
///
/// **Newest first, oldest dropped, and that is a choice with a reason.** A proxy is produced only
/// for a page of the most recent frame, so the pages on the screen are always the newest entries
/// and the one evicted is never one the window is showing — provided the extent is at least the
/// number of pages Table 29's arrangement puts up, which is what `PROXY_PAGES` is sized for.
#[derive(Debug)]
pub(crate) struct Proxies<P> {
    held: Vec<Retained<P>>,
    extent: usize,
}

impl<P> Proxies<P> {
    /// A store that will hold `extent` pages. Zero is the feature turned off.
    pub(crate) fn new(extent: usize) -> Self {
        Self {
            held: Vec::new(),
            extent,
        }
    }

    /// Keeps a page's picture, replacing any picture of that page and dropping the oldest.
    pub(crate) fn keep(&mut self, one: Retained<P>) {
        self.held.retain(|held| !Arc::ptr_eq(&held.page, &one.page));
        self.held.insert(0, one);
        self.held.truncate(self.extent);
    }

    /// The first page of `pages` with no picture yet, which is what the render thread draws next.
    ///
    /// `None` where every one of them is held, and `None` for an extent of zero — a store that
    /// keeps nothing must not ask for anything, or the render thread would draw a proxy per idle
    /// turn and throw each one away.
    pub(crate) fn wanted(
        &self,
        pages: &[(Arc<DisplayList>, TargetSpec)],
    ) -> Option<Arc<DisplayList>> {
        if self.extent == 0 {
            return None;
        }
        pages
            .iter()
            .map(|(page, _)| page)
            .find(|page| !self.held.iter().any(|held| Arc::ptr_eq(&held.page, page)))
            .map(Arc::clone)
    }

    /// Where each retained page goes so that it depicts the view being asked for.
    ///
    /// One placement per page that is in both this store and the arrangement, as an index into
    /// [`Self::get`] — **not one affine for the whole picture**, which is the difference between
    /// this layer and the base. The base is one texture of the window and so needs one placement
    /// true of every page in it ([`Refusal::Rearranged`]); these are one texture *per page*, so a
    /// zoom in a column places each of them correctly and the arrangement's gap never comes into
    /// it.
    ///
    /// A page whose composition is not a finite affine is left out rather than refused: the rest
    /// of the picture is still true, and there is no single answer here to spoil.
    pub(crate) fn placements(
        &self,
        asked: &[(Arc<DisplayList>, TargetSpec)],
    ) -> Vec<(usize, Transform)> {
        let mut placed = Vec::new();
        for (index, held) in self.held.iter().enumerate() {
            let Some((_, target)) = asked.iter().find(|(page, _)| Arc::ptr_eq(page, &held.page))
            else {
                continue;
            };
            let Some(moved) = held
                .target
                .transform
                .invert()
                .map(|back| back.then(target.transform))
            else {
                continue;
            };
            if [moved.a, moved.b, moved.c, moved.d, moved.e, moved.f]
                .iter()
                .all(|coefficient| coefficient.is_finite())
            {
                placed.push((index, moved));
            }
        }
        placed
    }

    /// The page at an index [`Self::placements`] handed back.
    pub(crate) fn get(&self, index: usize) -> Option<&Retained<P>> {
        self.held.get(index)
    }

    /// How many pages are held, for the trace.
    pub(crate) fn len(&self) -> usize {
        self.held.len()
    }

    /// How many this store will hold — the host's setting, carried to the render thread.
    pub(crate) fn extent(&self) -> usize {
        self.extent
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
    /// The pages of Table 29's arrangement and where each was placed, in this window's own
    /// device pixels.
    ///
    /// Each page by the `Arc` that makes its address mean something — the identity
    /// `render-quorra` reuses a scene by, for the same ABA reason (ADR 0351). **A list since ADR
    /// 0442**: `OneColumn` puts several pages in one texture, and which pages those are is what
    /// decides whether one placement carries the whole picture.
    pages: Vec<(Arc<DisplayList>, TargetSpec)>,
}

impl Settled {
    /// The extent of the window this picture is of, or `None` where it is of no page at all.
    ///
    /// Read off the first placement rather than kept beside them: every page of one frame was
    /// drawn into the same window-sized target, so they all state it and one is the answer.
    fn extent(&self) -> Option<(u32, u32)> {
        self.pages
            .first()
            .map(|(_, target)| (target.width, target.height))
    }
}

/// Whether the picture held already depicts the arrangement being asked for.
///
/// The same pages, in the same order, at the same placements — all three, because Table 29's
/// arrangement is a *set* of placed pages and a picture that is right about two of three is a
/// window with a hole in it. Under `SinglePage` this is the one comparison it has always been.
fn depicts(
    settled: &[(Arc<DisplayList>, TargetSpec)],
    asked: &[(Arc<DisplayList>, TargetSpec)],
) -> bool {
    settled.len() == asked.len()
        && settled
            .iter()
            .zip(asked)
            .all(|((was, placed), (now, asked))| Arc::ptr_eq(was, now) && placed == asked)
}

/// The one affine that carries every page of the settled picture onto the view being asked for.
///
/// **This is what replaced a single page's `settled⁻¹ ∘ asked` when a column arrived**, and it is
/// an exact question rather than a tolerant one. The window presents its pages as *one* texture
/// under *one* placement, so a reprojection is defensible only where one affine is true of every
/// page in the picture. For each page in both the settled arrangement and the one being asked for
/// — matched by the `Arc`'s address, which is what identifies a page here — this composes that
/// page's own `settled⁻¹ ∘ asked`, and answers only where they all agree.
///
/// The three ways it answers `None` are three different facts, and the caller tells them apart:
///
/// - **no page is in both**, which is a page turn — nothing about the outgoing pages' pixels is
///   true of the incoming ones, at any placement;
/// - **a placement does not invert**, or the composition is not finite;
/// - **the pages disagree**, which is the case a column introduced. A *scroll* moves every page by
///   the same distance, so they agree exactly and a column reprojects as a single page always did.
///   A *zoom* does not: `viewer_core::layout`'s gap between rows is stated in logical pixels and so
///   does not scale with the magnification, while the pages either side of it do — so no one affine
///   carries both, and the honest answer is to wait for the real frame rather than to move a page
///   to a place it is not.
fn one_placement(
    settled: &[(Arc<DisplayList>, TargetSpec)],
    asked: &[(Arc<DisplayList>, TargetSpec)],
) -> Result<Transform, Refusal> {
    let mut agreed: Option<Transform> = None;
    for (page, target) in asked {
        let Some((_, was)) = settled
            .iter()
            .find(|(settled_page, _)| Arc::ptr_eq(settled_page, page))
        else {
            continue;
        };
        let Some(moved) = was
            .transform
            .invert()
            .map(|back| back.then(target.transform))
        else {
            return Err(Refusal::NoPlacement);
        };
        if ![moved.a, moved.b, moved.c, moved.d, moved.e, moved.f]
            .iter()
            .all(|coefficient| coefficient.is_finite())
        {
            return Err(Refusal::NoPlacement);
        }
        match agreed {
            None => agreed = Some(moved),
            Some(first) if first == moved => {}
            Some(first) => return Err(Refusal::Rearranged { first, then: moved }),
        }
    }
    agreed.ok_or(Refusal::AnotherPage)
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
    /// How many have been drawn and of what — rule 3's count, which the frame summary prints.
    count: Drawn,
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
    /// Whether the last rendering carries onto this view, and why not where it does not.
    ///
    /// Split out of [`Self::plan`] when the retained pages arrived (ADR 0443), because the answer
    /// stopped deciding the whole frame: a base that does not carry used to mean a window that
    /// showed nothing, and now means a window drawn from the blurrier layer alone.
    fn carries(settled: &Settled, pages: &[(Arc<DisplayList>, TargetSpec)]) -> Result<(), Refusal> {
        // A resize changes what the window is as well as where the pages are in it, and what is
        // held is that window's whole picture — see [`Refusal::Resized`] for why the chrome in it
        // makes this an impossibility rather than the revealed edge under another name. It is not
        // an impossibility for the retained pages, which are pictures of a *page* and know nothing
        // about the window: a resize is one of the cases they now answer.
        let extent = pages
            .first()
            .map(|(_, target)| (target.width, target.height));
        if extent.is_some() && settled.extent() != extent {
            return Err(Refusal::Resized);
        }
        one_placement(&settled.pages, pages).map(|_| ())
    }

    pub(crate) fn plan(
        &self,
        pages: &[(Arc<DisplayList>, TargetSpec)],
        covered: usize,
        period: Duration,
        drawing: bool,
    ) -> Plan {
        let Some(settled) = self.settled.as_ref() else {
            return Plan::Refused(Refusal::NothingRendered);
        };
        // Not a view change: the rendering held *is* of the view being asked for, so it goes up
        // as it is and nothing is being stood in for. This is every tick of a document nobody is
        // touching, and it says nothing. **The arrangement has to be the same arrangement** —
        // same pages, same order, same placements — because a further page arriving at the bottom
        // of a column is a picture the one held does not contain.
        //
        // **The second half of this test is gone, and it was load-bearing before ADR 0391.** It
        // read `|| self.showing == Some(target.transform)` — rule 1 refusing to approximate the
        // same view twice, so that the event thread would draw the real frame instead. There is no
        // instead now: the render is on another thread whatever this one decides, and answering
        // `Render` for a view the rendering is *not* of would put the old view up unmoved, which
        // is a jump backwards on the screen. So a view that keeps being asked for keeps being
        // stood in for, at three quads a refresh, until the rendering lands — which is what a
        // picture every refresh means.
        if depicts(&settled.pages, pages) {
            return Plan::Render;
        }
        // Whether one placement is true of every page in the picture — **asked before rule 5**,
        // because a page turn and a rearrangement are impossibilities and rule 5 is a judgement
        // about two measurements. Nothing about the outgoing pages' pixels says anything true
        // about the incoming ones, at any placement.
        let base = Self::carries(settled, pages);
        // Nothing true to draw from either layer is the refusal this has always been. **The
        // condition gained its second half in ADR 0443**: a page turn with a retained picture of
        // the incoming page is no longer a window that shows nothing, so an impossibility for the
        // base alone is not a refusal of the frame.
        if let Err(why) = &base
            && covered == 0
        {
            return Plan::Refused(why.clone());
        }
        // Rule 5: a frame the machine delivers inside one refresh *is* the frame every refresh
        // the owner asked for, so there is nothing to stand in for. **Rule 4 used to be the next
        // question and there is no next question**: a reprojection is issued by the thread holding
        // the surface while the render runs on another, so what standing in costs the real frame
        // is nothing rather than a fraction, and a bound on nothing is not a bound (ADR 0391).
        //
        // It gates the retained pages exactly as it gates the base, and for the same reason: a
        // blurred page shown for one refresh and replaced is worse than the refresh spent waiting.
        if !self.missed(period, drawing) {
            return Plan::Refused(Refusal::InsideTheRefresh {
                frame: self.expected(),
                period,
            });
        }
        Plan::Approximate(match base {
            Ok(()) if covered > 0 => Stand {
                from: Source::LastFrameOverPages,
                instead_of_the_base: None,
            },
            Ok(()) => Stand {
                from: Source::LastFrame,
                instead_of_the_base: None,
            },
            Err(why) => Stand {
                from: Source::RetainedPages,
                instead_of_the_base: Some(why),
            },
        })
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
    pub(crate) fn drawn(&mut self, from: Source) -> MustFollow {
        self.showing = true;
        self.count.record(from);
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

    /// Says that the sharp layer was refused and the retained pages stood in, and counts it.
    ///
    /// **Beside [`Self::declined`] rather than inside it, because the two are different facts.**
    /// That one is a view change the window did not move for; this one moved, from the blurrier
    /// source, and the summary keeps them apart so that a person reading a trace of a page turn
    /// can tell "nothing was shown" from "a low-resolution page was shown".
    pub(crate) fn without_base(&mut self, why: &Refusal, held: usize, trace: crate::trace::Trace) {
        self.refusals.proxied = self.refusals.proxied.saturating_add(1);
        trace.say(
            crate::trace::Topic::Frames,
            format_args!(
                "no reprojection ({}): {why} — {held} retained low-resolution page(s) stand in \
                 instead",
                why.kind()
            ),
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
        pages: &[(Arc<DisplayList>, TargetSpec)],
        cost: Duration,
        built: bool,
    ) {
        if built {
            self.building = Some(cost);
        }
        self.settled = Some(Settled {
            pages: pages.to_vec(),
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

    /// How many stand-ins this run has drawn and of what — rule 3's count.
    pub(crate) fn count(&self) -> Drawn {
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
        pages: &[(Arc<DisplayList>, TargetSpec)],
    ) -> Result<Transform, Refusal> {
        let Some(settled) = self.settled.as_ref() else {
            return Err(Refusal::NothingRendered);
        };
        let extent = pages
            .first()
            .map(|(_, target)| (target.width, target.height));
        if extent.is_some() && settled.extent() != extent {
            return Err(Refusal::Resized);
        }
        one_placement(&settled.pages, pages)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use pdf_render::{DisplayList, Size, TargetSpec, Transform};

    use super::{Plan, Proxies, Refusal, Retained, Source, Stale, Stand};

    /// No retained page has pixels for the view being asked for, which is what every case written
    /// before ADR 0443 assumed and what a run with `--proxy-pages 0` is.
    const NONE_HELD: usize = 0;

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

    /// Table 29's default arrangement: one page, at a magnification.
    ///
    /// Every test but the column's below asks about this, because `SinglePage` is what the table
    /// states as its own default and what every document that says nothing opens in.
    fn alone(page: &Arc<DisplayList>, magnification: f32) -> Vec<(Arc<DisplayList>, TargetSpec)> {
        vec![(Arc::clone(page), view(magnification))]
    }

    /// One page at a placement stated outright, for the two cases a magnification cannot express:
    /// a window that changed shape, and a transform that does not invert.
    fn at(page: &Arc<DisplayList>, target: TargetSpec) -> Vec<(Arc<DisplayList>, TargetSpec)> {
        vec![(Arc::clone(page), target)]
    }

    /// A column of `pages`, laid out as `viewer_core::layout` lays one out: each page under the
    /// same magnification, stacked down the window with `GAP` logical pixels between them, the
    /// whole scrolled by `scroll` device pixels.
    ///
    /// The gap is 8 and it is **not** multiplied by the magnification, which is that module's own
    /// documented choice — "a gap is a thing a person sees" — and the whole reason a zoom in a
    /// column is not one affine of itself.
    fn column(
        pages: &[Arc<DisplayList>],
        magnification: f32,
        scroll: f32,
    ) -> Vec<(Arc<DisplayList>, TargetSpec)> {
        let mut placed = Vec::new();
        let mut top = -scroll;
        for page in pages {
            placed.push((
                Arc::clone(page),
                TargetSpec {
                    width: 800,
                    height: 1000,
                    transform: Transform::scale(magnification, -magnification)
                        .then(Transform::translate(0.0, 842.0 * magnification))
                        .then(Transform::translate(0.0, top)),
                },
            ));
            top += 842.0 * magnification + 8.0;
        }
        placed
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
        stale.settled(&alone(page, 1.0), Duration::from_millis(700), true);
    }

    /// Rule 1. A reprojection is never the state the window settles in, and what enforces that is
    /// the obligation rather than the plan.
    ///
    /// **`Plan` used to answer `Render` for a view already approximated**, so that the event
    /// thread would draw the real frame there and then. Since ADR 0391 it answers `Approximate`
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
            stale
                .plan(&alone(&page, 1.2), NONE_HELD, REFRESH, LANDED)
                .stands_in(),
            "a slow frame and a new magnification is what this exists for"
        );
        let follow = stale.drawn(Source::LastFrame);
        assert!(stale.showing_approximation());
        assert!(
            stale
                .plan(&alone(&page, 1.2), NONE_HELD, REFRESH, DRAWING)
                .stands_in(),
            "the same view again while the render is still out: the same three quads, which is \
             what a picture every refresh means"
        );
        // And the view the *rendering* is of goes up as it is, which is the answer that must not
        // be given to any other view.
        assert_eq!(
            stale.plan(&alone(&page, 1.0), NONE_HELD, REFRESH, DRAWING),
            Plan::Render
        );
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
        stale.settled(&alone(&page, 1.0), REFRESH / 2, true);
        assert_eq!(
            stale.plan(&alone(&page, 1.2), NONE_HELD, REFRESH, LANDED),
            Plan::Refused(Refusal::InsideTheRefresh {
                frame: REFRESH / 2,
                period: REFRESH,
            }),
            "a view whose frame lands inside the refresh must show that frame, and say so"
        );
        stale.settled(&alone(&page, 1.0), REFRESH * 2, true);
        assert!(
            stale
                .plan(&alone(&page, 1.2), NONE_HELD, REFRESH, LANDED)
                .stands_in()
        );
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
        stale.settled(&alone(&page, 1.0), REFRESH / 4, true);
        assert!(
            !stale
                .plan(&alone(&page, 1.2), NONE_HELD, REFRESH, LANDED)
                .stands_in(),
            "the prediction says this one lands in time, so the first tick waits for it"
        );
        assert!(
            stale
                .plan(&alone(&page, 1.2), NONE_HELD, REFRESH, DRAWING)
                .stands_in(),
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
            stale.settled(&alone(&page, 1.0), cost, true);
            assert!(
                stale
                    .plan(&alone(&page, 1.2), NONE_HELD, REFRESH, LANDED)
                    .stands_in(),
                "{frame} ms is {} refreshes and must be stood in for",
                cost.as_secs_f64() / REFRESH.as_secs_f64()
            );
        }
        // And the property behind it, stated directly: nothing that *gates* a reprojection may
        // depend on a measurement only a reprojection can produce.
        stale.settled(&alone(&page, 1.0), REFRESH * 3, true);
        assert!(
            stale
                .plan(&alone(&page, 1.2), NONE_HELD, REFRESH, LANDED)
                .stands_in(),
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
            stale.settled(&alone(&page, 1.0), cost, true);
            assert!(
                stale
                    .plan(&alone(&page, 1.3), NONE_HELD, period, LANDED)
                    .stands_in(),
                "{frame} ms misses a 8.333 ms refresh, and standing in now costs that frame \
                 nothing at all"
            );
            drop(stale.drawn(Source::LastFrame));
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
        stale.settled(&alone(&page, 1.0), render, true);
        assert_eq!(stale.expected(), render);
        // The same view, redrawn for a reason that has nothing to do with the page.
        stale.settled(&alone(&page, 1.0), Duration::from_millis(2), false);
        assert_eq!(
            stale.expected(),
            render,
            "a replay says what a replay costs and nothing about the next render"
        );
        assert!(
            stale
                .plan(&alone(&page, 1.2), NONE_HELD, REFRESH, LANDED)
                .stands_in(),
            "the zoom after a harmless redraw is the one the owner waited through"
        );
        // A frame that genuinely built a cheap picture *does* move it.
        stale.settled(&alone(&page, 1.0), REFRESH / 2, true);
        assert_eq!(stale.expected(), REFRESH / 2);
        assert!(
            !stale
                .plan(&alone(&page, 1.3), NONE_HELD, REFRESH, LANDED)
                .stands_in()
        );
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
            stale.plan(&alone(&second, 1.0), NONE_HELD, REFRESH, LANDED),
            Plan::Refused(Refusal::AnotherPage)
        );
        assert_eq!(
            stale.plan(&alone(&second, 1.3), NONE_HELD, REFRESH, LANDED),
            Plan::Refused(Refusal::AnotherPage)
        );
        assert_eq!(
            stale.reproject(&alone(&second, 1.3)).unwrap_err(),
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
            stale.plan(&alone(&page, 1.0), NONE_HELD, REFRESH, LANDED),
            Plan::Render,
            "the view already on the screen is drawn, not approximated — and not refused either"
        );
        let resized = TargetSpec {
            width: 900,
            ..view(1.2)
        };
        assert_eq!(
            stale.plan(&at(&page, resized), NONE_HELD, REFRESH, LANDED),
            Plan::Refused(Refusal::Resized)
        );
        assert_eq!(
            stale.reproject(&at(&page, resized)).unwrap_err(),
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
        assert!(
            stale
                .plan(&alone(&page, 2.0), NONE_HELD, REFRESH, LANDED)
                .stands_in()
        );
        let moved = stale
            .reproject(&alone(&page, 2.0))
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
            .reproject(&alone(&page, 1.0))
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
        stale.settled(&at(&page, flat), Duration::from_millis(700), true);
        assert_eq!(
            stale.reproject(&alone(&page, 1.2)).unwrap_err(),
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
                stale
                    .plan(&at(&page, asked), NONE_HELD, REFRESH, DRAWING)
                    .stands_in(),
                "the view keeps moving and the frame stays slow"
            );
            let moved = stale
                .reproject(&at(&page, asked))
                .expect("a rendering is held");
            // Through the pixels of the last *rendering*, which is the only thing composed
            // against however many reprojections have been drawn since.
            let through = moved.apply(base.apply(corner));
            let directly = asked.transform.apply(corner);
            assert!(
                (through.x - directly.x).abs() < 1e-3 && (through.y - directly.y).abs() < 1e-3,
                "{magnification}: {through:?} {directly:?}"
            );
            drop(stale.drawn(Source::LastFrame));
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
        drop(stale.drawn(Source::LastFrame));
        // The frame for the 1.2× view finally lands, while the person has already asked for 2×.
        stale.settled(&alone(&page, 1.2), Duration::from_millis(700), true);
        stale.real();
        assert!(
            stale
                .plan(&alone(&page, 2.0), NONE_HELD, REFRESH, LANDED)
                .stands_in()
        );
        let moved = stale
            .reproject(&alone(&page, 2.0))
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

    /// **A scroll of Table 29's column reprojects exactly**, which is the case ADR 0442 had to
    /// keep working: every page of the arrangement moves by the same distance, so one placement
    /// is true of the whole texture and the window answers the wheel at once.
    ///
    /// Checked as a *property of the pages* rather than of the answer: a point of each page is
    /// carried through that page's own settled placement and then through the one placement, and
    /// it must land where that page's new placement puts it. A composition read off the first
    /// page alone would pass this for page one and be wrong for page two.
    #[test]
    fn a_scroll_of_a_column_moves_every_page_by_the_same_placement() {
        let pages = [page(), page(), page()];
        let mut stale = Stale::default();
        let settled = column(&pages, 1.0, 0.0);
        stale.settled(&settled, Duration::from_millis(700), true);
        let asked = column(&pages, 1.0, 240.0);
        assert!(stale.plan(&asked, NONE_HELD, REFRESH, LANDED).stands_in());
        let moved = stale.reproject(&asked).expect("a column that scrolled");
        let corner = pdf_render::Point::new(100.0, 700.0);
        for (index, ((_, was), (_, now))) in settled.iter().zip(&asked).enumerate() {
            let through = moved.apply(was.transform.apply(corner));
            let directly = now.transform.apply(corner);
            assert!(
                (through.x - directly.x).abs() < 1e-3 && (through.y - directly.y).abs() < 1e-3,
                "page {index}: {through:?} {directly:?}"
            );
        }
    }

    /// **A zoom of a column is refused, and the refusal is exact rather than tolerant.**
    ///
    /// The window presents its pages as one texture under one placement, and
    /// `viewer_core::layout`'s gap between rows is stated in logical pixels — it does not scale
    /// with the magnification while the pages either side of it do. So no single affine carries
    /// both pages, and moving one of them to a place it is not would be the plausible-looking lie
    /// `CLAUDE.md`'s first principle is about. The window waits for the real frame and says why.
    ///
    /// One page under the same zoom is *not* refused, which is the half that says this is about
    /// the arrangement and not about zooming.
    #[test]
    fn a_zoom_of_a_column_is_refused_because_no_one_placement_carries_it() {
        let pages = [page(), page()];
        let mut stale = Stale::default();
        stale.settled(&column(&pages, 1.0, 0.0), Duration::from_millis(700), true);
        let refused = stale.reproject(&column(&pages, 1.25, 0.0)).unwrap_err();
        assert!(
            matches!(refused, Refusal::Rearranged { .. }),
            "a zoom leaves the gap where it was and moves the pages: {refused:?}"
        );
        assert_eq!(refused.kind(), "impossible");
        assert_eq!(
            stale.plan(&column(&pages, 1.25, 0.0), NONE_HELD, REFRESH, LANDED),
            Plan::Refused(refused)
        );
        // And the single page it is not about.
        let mut alone_stale = Stale::default();
        alone_stale.settled(
            &column(&pages[..1], 1.0, 0.0),
            Duration::from_millis(700),
            true,
        );
        assert!(
            alone_stale
                .plan(&column(&pages[..1], 1.25, 0.0), NONE_HELD, REFRESH, LANDED)
                .stands_in(),
            "one page has one placement and always did"
        );
    }

    /// A further row of the column arriving is a picture the one held does not contain, so it is
    /// not the view already on the screen — and it is still a scroll, so it is stood in for.
    ///
    /// The revealed edge shows the window's medium, which is `doc/todo/37`'s standing gap and not
    /// something this refuses over.
    #[test]
    fn a_row_arriving_is_not_the_picture_already_up() {
        let pages = [page(), page(), page()];
        let mut stale = Stale::default();
        stale.settled(
            &column(&pages[..2], 1.0, 0.0),
            Duration::from_millis(700),
            true,
        );
        let asked = column(&pages, 1.0, 100.0);
        assert_ne!(
            stale.plan(&asked, NONE_HELD, REFRESH, LANDED),
            Plan::Render,
            "a third page on the screen is not the frame that showed two"
        );
        assert!(stale.plan(&asked, NONE_HELD, REFRESH, LANDED).stands_in());
        assert!(
            stale.reproject(&asked).is_ok(),
            "the two pages in both pictures agree on one placement"
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
                    .plan(&alone(&page, magnification), NONE_HELD, REFRESH, DRAWING)
                    .stands_in(),
                "{magnification}: nothing about a frame's atlas reaches a texture already drawn"
            );
            assert!(stale.reproject(&alone(&page, magnification)).is_ok());
            drop(stale.drawn(Source::LastFrame));
        }
        assert_eq!(stale.refusals().total(), 0);
        assert_eq!(stale.count().total(), 5);
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
                stale.plan(&alone(&page, 1.0), NONE_HELD, REFRESH, LANDED),
                Plan::Render,
                "the same view, redrawn: quorra replays it and nothing is being stood in for"
            );
        }
        assert_eq!(stale.refusals().total(), 0);
    }

    /// A store of pictures whose "pixels" are the page's index, so that a placement can be
    /// checked against the page it belongs to without a graphics device.
    fn retaining(pages: &[Arc<DisplayList>]) -> Proxies<usize> {
        let mut proxies = Proxies::new(pages.len());
        for (index, page) in pages.iter().enumerate() {
            proxies.keep(Retained {
                page: Arc::clone(page),
                target: super::proxy_target(page).expect("a page with an extent"),
                pixels: index,
            });
        }
        proxies
    }

    /// **The case `doc/todo/37` said this construction pays for, and it is the strongest one.** A
    /// page turn and a `GoTo` have no base at all — nothing about the outgoing page's pixels is
    /// true of the incoming one at any placement — so before ADR 0443 the window showed the page
    /// being left, frozen, for the whole of the render. A retained picture of the page arriving
    /// is the only thing that can stand in there.
    ///
    /// Both halves are asserted, because the difference between them is the feature: with no
    /// retained page the refusal is exactly what it always was.
    #[test]
    fn a_page_turn_shows_a_retained_page_where_one_is_held() {
        let first = page();
        let second = page();
        let mut stale = Stale::default();
        slow(&mut stale, &first);
        let held = retaining(&[Arc::clone(&second)]);
        let asked = alone(&second, 1.0);
        let under = held.placements(&asked);
        assert_eq!(under.len(), 1, "the incoming page's own picture is held");
        assert_eq!(
            stale.plan(&asked, under.len(), REFRESH, LANDED),
            Plan::Approximate(Stand {
                from: Source::RetainedPages,
                instead_of_the_base: Some(Refusal::AnotherPage),
            }),
            "the sharp layer is still impossible and says so; the window still moves"
        );
        assert_eq!(
            stale.plan(&asked, NONE_HELD, REFRESH, LANDED),
            Plan::Refused(Refusal::AnotherPage),
            "and with nothing retained it is the refusal it always was"
        );
    }

    /// A retained page is placed where the *view* puts that page, which is what makes the layer
    /// under the base line up with it rather than merely appear.
    ///
    /// Read as a property of a point on the page — carried through the proxy's own target and
    /// then through the placement — because that is where the correctness lives: the two targets
    /// are built by the same `TargetSpec::for_page` out of the same page, so their composition is
    /// exact and a test that compared transforms would be re-deriving the arithmetic.
    #[test]
    fn a_retained_page_lands_where_the_view_puts_that_page() {
        let page = page();
        let held = retaining(&[Arc::clone(&page)]);
        let corner = pdf_render::Point::new(100.0, 700.0);
        for magnification in [0.3_f32, 1.0, 2.5] {
            let asked = alone(&page, magnification);
            let placed = held.placements(&asked);
            let (index, moved) = placed.first().copied().expect("one page, one placement");
            let proxy = held.get(index).expect("the index it handed back").target;
            let through = moved.apply(proxy.transform.apply(corner));
            let directly = asked[0].1.transform.apply(corner);
            assert!(
                (through.x - directly.x).abs() < 1e-2 && (through.y - directly.y).abs() < 1e-2,
                "{magnification}: {through:?} {directly:?}"
            );
        }
    }

    /// **The zoom in a column, which the base refuses and this layer answers** (ADR 0442's
    /// `Refusal::Rearranged`). The window puts its pages up as one texture under one placement, so
    /// no single affine carries a column whose gap does not scale; a retained page is one texture
    /// *per page*, so each is placed by its own composition and the gap never enters into it.
    #[test]
    fn a_zoom_of_a_column_places_every_retained_page_by_itself() {
        let pages = [page(), page()];
        let held = retaining(&pages);
        let mut stale = Stale::default();
        stale.settled(&column(&pages, 1.0, 0.0), Duration::from_millis(700), true);
        let asked = column(&pages, 1.25, 0.0);
        let placed = held.placements(&asked);
        assert_eq!(placed.len(), 2, "both pages of the column are held");
        let corner = pdf_render::Point::new(100.0, 700.0);
        for (index, moved) in placed {
            let retained = held.get(index).expect("the index it handed back");
            let (_, target) = asked
                .iter()
                .find(|(page, _)| Arc::ptr_eq(page, &retained.page))
                .expect("a page of the arrangement");
            let through = moved.apply(retained.target.transform.apply(corner));
            let directly = target.transform.apply(corner);
            assert!(
                (through.x - directly.x).abs() < 1e-2 && (through.y - directly.y).abs() < 1e-2,
                "page {index}: {through:?} {directly:?}"
            );
        }
        assert_eq!(
            stale.plan(&asked, 2, REFRESH, LANDED),
            Plan::Approximate(Stand {
                from: Source::RetainedPages,
                instead_of_the_base: Some(Refusal::Rearranged {
                    first: match stale.reproject(&asked) {
                        Err(Refusal::Rearranged { first, .. }) => first,
                        other => panic!("a zoom of a column rearranges: {other:?}"),
                    },
                    then: match stale.reproject(&asked) {
                        Err(Refusal::Rearranged { then, .. }) => then,
                        other => panic!("a zoom of a column rearranges: {other:?}"),
                    },
                }),
            }),
        );
    }

    /// **Rule 5 gates this layer exactly as it gates the base**, and for the same reason: a
    /// blurred page shown for one refresh and replaced is worse than the refresh spent waiting.
    #[test]
    fn a_frame_inside_the_refresh_is_not_stood_in_for_by_a_retained_page_either() {
        let first = page();
        let second = page();
        let mut stale = Stale::default();
        stale.settled(&alone(&first, 1.0), REFRESH / 2, true);
        assert_eq!(
            stale.plan(&alone(&second, 1.0), 1, REFRESH, LANDED),
            Plan::Refused(Refusal::InsideTheRefresh {
                frame: REFRESH / 2,
                period: REFRESH,
            }),
            "a page turn whose frame lands in time shows that frame, retained page or not"
        );
    }

    /// Rule 3's new distinction: the three sources are three different amounts of wrong and the
    /// frame line says which, so no two of them may print the same word — and the tally keeps
    /// them apart.
    #[test]
    fn every_source_says_which_layers_filled_the_picture() {
        let every = [
            Source::LastFrame,
            Source::LastFrameOverPages,
            Source::RetainedPages,
        ];
        let mut said: Vec<&str> = every.iter().map(|from| from.word()).collect();
        said.sort_unstable();
        said.dedup();
        assert_eq!(said.len(), every.len(), "two sources printed the same word");
        assert!(Source::LastFrame.has_base() && Source::LastFrameOverPages.has_base());
        assert!(!Source::RetainedPages.has_base());
        let mut stale = Stale::default();
        for from in every {
            drop(stale.drawn(from));
        }
        let drawn = stale.count();
        assert_eq!(
            (drawn.last_frame, drawn.over_pages, drawn.pages_only),
            (1, 1, 1)
        );
        assert_eq!(drawn.total(), 3);
    }

    /// The store keeps the newest and drops the oldest, and asks for the first page of the
    /// arrangement it has no picture of.
    ///
    /// **The eviction order is what keeps a page on the screen from being thrown away**: a
    /// picture is produced only for a page of the most recent frame, so the pages showing are
    /// always the newest entries — provided the extent is at least as large as the arrangement,
    /// which is what `PROXY_PAGES` is sized for.
    #[test]
    fn the_store_keeps_the_newest_pages_and_asks_for_what_it_has_not_got() {
        let pages = [page(), page(), page()];
        let mut proxies: Proxies<usize> = Proxies::new(2);
        let arrangement: Vec<(Arc<DisplayList>, TargetSpec)> = pages
            .iter()
            .map(|page| (Arc::clone(page), view(1.0)))
            .collect();
        for (index, page) in pages.iter().enumerate() {
            assert!(
                proxies
                    .wanted(&arrangement)
                    .is_some_and(|asked| Arc::ptr_eq(&asked, page)),
                "the first page with no picture, in the arrangement's own order"
            );
            proxies.keep(Retained {
                page: Arc::clone(page),
                target: super::proxy_target(page).expect("a page with an extent"),
                pixels: index,
            });
        }
        assert_eq!(proxies.len(), 2, "the extent is a bound and it binds");
        assert_eq!(
            proxies.placements(&arrangement).len(),
            2,
            "the two newest, and the oldest is gone"
        );
        // And a store that keeps nothing asks for nothing, or the render thread would draw a
        // picture per idle turn and throw every one of them away.
        let empty: Proxies<usize> = Proxies::new(0);
        assert!(empty.wanted(&arrangement).is_none());
    }

    /// The retained picture is the *whole page*, bounded by [`super::PROXY_EDGE`] on its longer
    /// side — which is what makes it complementary to the base rather than a second copy of it.
    ///
    /// A crop of a real frame would not do, and this is the property that says so: at a high
    /// magnification a window frame is a crop of the page, while this is the page whatever the
    /// view is doing.
    #[test]
    fn a_retained_picture_is_the_whole_page_within_the_stated_edge() {
        for (width, height) in [(595.0_f32, 842.0_f32), (842.0, 595.0), (200.0, 200.0)] {
            let page = Arc::new(DisplayList::new(Size::new(width, height)));
            let target = super::proxy_target(&page).expect("a page with an extent");
            // Within a pixel either way: `TargetSpec::for_page` rounds **up** so that the raster
            // contains the page (ADR 0064), and the ratio of two page dimensions is not exact in
            // `f32` — so a longer side that divides exactly still lands one row over. The edge is
            // a budget to spend rather than a ceiling to stay under, which is why the floor is
            // asserted as well as the ceiling.
            assert!(
                (super::PROXY_EDGE - 1..=super::PROXY_EDGE + 1)
                    .contains(&target.width.max(target.height)),
                "{width}x{height} became {}x{}",
                target.width,
                target.height
            );
            // The whole page: its two far corners land inside the raster, which is what a crop
            // would fail.
            let corners = [
                pdf_render::Point::new(0.0, 0.0),
                pdf_render::Point::new(width, height),
            ];
            for corner in corners {
                let in_raster = target.transform.apply(corner);
                assert!(
                    in_raster.x >= -0.5
                        && in_raster.y >= -0.5
                        && in_raster.x
                            <= f32::from(u16::try_from(target.width).expect("small")) + 0.5
                        && in_raster.y
                            <= f32::from(u16::try_from(target.height).expect("small")) + 0.5,
                    "{corner:?} left the raster at {in_raster:?}"
                );
            }
        }
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
