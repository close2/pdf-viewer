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
//! | 4. it does not make the real frame later | **the render is on a thread of its own on both surfaces since ADR 0461**, so a stand-in is drawn *beside* the frame it stands in for rather than in front of it. On the device that settles it: three textured quads take nothing the frame was going to use (ADR 0391). On the processor a resample is tens of milliseconds of the *presenting* thread and can still overrun the render beside it, so [`Stale::affordable`] bounds the overrun at zero — `resample + period ≤ frame`, unmeasured permits, no constant ([`Standing`]) |
//! | 5. it does not fire when it is not needed | [`Stale::missed`] — the frame did not land inside the surface's own refresh, which is the owner's word *miss*, known two ways |
//!
//! # The base is the last rendering's own texture, and that is ADR 0391
//!
//! **There is no readback here, and there is no capture to lose.** Resampling an `Arc<[u8]>` read
//! back off the window costs 2.7 to 6.6 ms of a refresh that is 8.333 on the owner's display, plus
//! an 8 192 000-byte re-upload behind it — and it can fail outright, because reading the window
//! back means replaying an encode that a glyph-atlas repack has destroyed (ADR 0384 section 6,
//! ADR 0385, ADR 0391). What the window shows is a texture the device rendered the page into and
//! never gave up, so:
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
//! structurally zero is not a bound, so rule 4 is enforced by the *arrangement* on that surface.
//!
//! **On the processor's window it is a question again, and ADR 0461 re-derived it rather than
//! restoring it.** ADR 0457 brought the rule back for a window that had no second thread, where a
//! resample genuinely delayed the frame by the whole of what it cost. That window has a composing
//! thread now (`crate::composer`), so a resample runs *beside* the render: the frame is presented
//! at the first tick after both are finished, and standing in costs it `max(0, resample − frame)`
//! rather than `resample`. The inequality did not move — `resample + period ≤ frame` — because
//! "buys the refresh it spends" and "is up a refresh before the true frame lands" are the same
//! arithmetic read from the two ends. What moved is the *size* of being wrong about it, and that
//! is a reason to keep the rule cheap rather than to delete it: it costs one comparison of two
//! measurements and it has no constant in it.
//!
//! What remains is the thing the owner asked for — *"we should still try to render a correct image
//! every frame, but if we miss, we should interpolate"* — with the unit the display's:
//!
//! **Rule 5: a miss is a frame that does not land inside one refresh, and it is now known two
//! ways.** By *prediction*, from what the last frame that had to build a picture cost, which is
//! what answers the very first tick of a view change before anything has been asked for; and by
//! *observation*, because a render that has been out for **more than one period** has missed a
//! refresh whatever anybody predicted. The bound is the period's own, not a constant, and it is
//! load-bearing (ADR 0704): "still being drawn" alone made the tick between the ask and the
//! collect observe a two-millisecond-old render as late, so every view change of a quick page
//! was stood in for — one blurred refresh, every gesture, bought nothing. The observation needs
//! no calibration and cannot be wrong; the prediction is what keeps the first tick of a gesture
//! from being a frozen one. Neither has a constant in it. ADR 0384, extended by ADR 0391.
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
//! Everything that makes an approximate picture is in this module, and this module is part of
//! `quorra`'s binary. A binary crate is not a dependency: `pdf-model`'s corpus and oracle
//! gates, `viewer-core`'s headless harness, `Query::Frame`, `render_at`, `viewer-confined`'s
//! worker and every diagnostic artefact in this tree are compiled without a line of it. **Nothing
//! crosses into a library at all any more**: `render-raster` draws a window's frame into two
//! textures and hands them back, `viewer_ui::software::compose_pages` draws one into a raster and
//! hands that back, and `crate::renderer` and `crate::composer` put them on a window under
//! placements this module computes. None of the three knows what a reprojection is, and
//! `tests::no_library_in_this_tree_knows_what_a_reprojection_is` walks every `.rs` outside
//! `viewer-ui/src/bin` to say so.
//!
//! # What a revealed edge shows, and what now shows there instead
//!
//! A scroll or a zoom out moves pixels off the region the old rendering covered. The base is a
//! picture of the *window*, so what it does not cover it has nothing to say about. Page white there
//! would assert that the page is blank there, which is the plausible-looking lie principle 1 is
//! about (ADR 0378).
//!
//! **The second layer is a picture of the *page*, and that is what fills it** (ADR 0443). A
//! [`Proxies`] entry is a whole page drawn once into a raster whose longer side is
//! [`PROXY_EDGE`] pixels, produced on whichever thread draws this window's pages when it has
//! nothing else to do — `crate::renderer`'s or `crate::composer`'s — and
//! placed under the base by the same `settled⁻¹ ∘ asked` composition. The two are complementary
//! rather than competing, which is the whole argument for the construction: a whole-page picture
//! is near the right resolution exactly when the view is zoomed out, which is when the margin
//! appears, and it is badly blurred exactly when the sharp base already covers the view.
//!
//! It also answers the case this module was not looking at: **a page turn and a `GoTo` have no base
//! at all**, because nothing about the outgoing page's pixels is true of the incoming one at any
//! placement. A page whose proxy is retained has pixels there where the base has none, so
//! [`Plan::Approximate`] carries a [`Source`] saying *which* layer filled the picture — rule 3
//! draws that distinction because "approximated from the last frame" and "approximated from a
//! low-resolution page" are different amounts of wrong.
//!
//! # Where each part is
//!
//! | module | what it is |
//! |---|---|
//! | this one | what a picture is of ([`Picture`], [`Placed`]), and [`Stale`], the view settled |
//! | [`plan`] | what one tick puts up, and every refusal of a stand-in, by name and by kind |
//! | [`placement`] | whether the picture held is of the view asked for, or one placement from it |
//! | [`retained`] | the layer under the base: whole pages kept small, and where each goes |
//! | [`canvas`] | the stand-in resampled on the processor, for the window with no graphics device |

// `#[path]` for the reason `crate::renderer`'s children give.
#[path = "stale/canvas.rs"]
mod canvas;
#[path = "stale/placement.rs"]
mod placement;
#[path = "stale/plan.rs"]
mod plan;
#[path = "stale/retained.rs"]
mod retained;
#[cfg(test)]
#[path = "stale/tests.rs"]
mod tests;

use std::sync::Arc;
use std::time::Duration;

use pdf_render::{DisplayList, TargetSpec};
use viewer_core::DocumentId;

pub(crate) use canvas::Canvas;
pub(crate) use placement::{Carried, absorbed, could_stand_in};
use placement::{carrying, depicts};
pub(crate) use plan::{Drawn, Missed, Plan, Refusal, Refusals, Source, Stand};
pub(crate) use retained::{PROXY_PAGES, Proxies, Retained, proxy_target};

/// The longer side, in device pixels, of the retained low-resolution picture of a whole page.
///
/// **Chosen from a measurement, and the measurement said the opposite of what was expected.**
/// `doc/todo/37` illustrated the idea with "an eighth of device scale" and said outright that the
/// scale was still open. What decides it is what a proxy *costs*, and on this machine's Radeon
/// 890M — `cargo run --release -p render-raster --example zoom_frame`, sequence
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
/// ladder showed.** A run that drew the same page at seven scales made raster throw its whole
/// glyph atlas away — `repacked` on the last rung — and the real frame after a repack cost 411 ms
/// where the one after a single proxy cost 82. A proxy scale that followed the zoom would put a
/// new glyph size in the atlas at every step; one fixed edge puts in one, once, per page.
pub(crate) const PROXY_EDGE: u32 = 512;

/// What a picture is a picture **of**, which is not the same question as which commands drew it.
///
/// **The identity a page's picture is kept under** (`doc/todo/37`, ADR 0457). Not the
/// `Arc<DisplayList>` it was drawn from: `viewer_core` holds one interpretation per page of the
/// arrangement rather than a cache of them, so a page turn drops the outgoing page's
/// interpretation, coming back produces a new `Arc` over the same commands, and a picture keyed by
/// the old one could never be found again.
///
/// The `Arc` answers *which interpretation*, and that is the right question for the base: the
/// sharp layer is a picture of one arrangement at one placement and [`depicts`] must be exact
/// about it. It is the wrong question for a retained page, which is a picture of a *page*.
///
/// So a page's picture is identified by the page it is of and the state its ink was interpreted
/// against — the document, the page number, and [`viewer_core::RenderRequest::ink`]. Two
/// interpretations that agree on all three drew the same commands, whatever their addresses are.
///
/// **What the ink excludes is the point of it.** A picture whose ink has been superseded is *not*
/// a picture of this page any more, and may not stand in: §8.11's layer switched off would go on
/// being drawn for a frame, sharp and legible, and a person cannot tell that from the switch not
/// working. Blur says *approximation* by itself; stale ink says nothing and asserts something
/// false, which is the plausible-instead-of-true failure `CLAUDE.md`'s first principle is about.
/// A view change over unchanged ink is the opposite case and is exactly what a stand-in is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Picture {
    /// Which document, because a window may be handed another one.
    document: DocumentId,
    /// Which page of it, zero-based, as `viewer_core::RenderRequest::page` numbers them.
    page: usize,
    /// Which state of that document's ink — see [`viewer_core::RenderRequest::ink`].
    ink: u64,
}

impl Picture {
    /// A picture of `page` of `document`, drawn against the ink `ink` names.
    pub(crate) fn new(document: DocumentId, page: usize, ink: u64) -> Self {
        Self {
            document,
            page,
            ink,
        }
    }

    /// Which page of the document this is a picture of, zero-based — for the trace, which counts
    /// from one as a reader does.
    pub(crate) fn page(self) -> usize {
        self.page
    }

    /// Whether the two were drawn against the same state of the same document's ink.
    ///
    /// What [`Proxies::keep`] evicts by: a picture of another ink is not a picture that may be
    /// shown again, so it is dropped rather than left to age out of the store — the same answer
    /// `viewer_core::open`'s own invalidation gives for the same reason, which is that the state
    /// is what *every* held page was interpreted against.
    pub(crate) fn same_ink(self, other: Self) -> bool {
        self.document == other.document && self.ink == other.ink
    }
}

/// One page of Table 29's arrangement as this window is drawing it: what it is of, its commands,
/// and where it goes.
///
/// **Three facts that used to be two.** The pair `(Arc<DisplayList>, TargetSpec)` travelled
/// between the event thread, the render thread and this module until ADR 0457 gave the retained
/// layer an identity that survives a re-interpretation; [`Self::of`] is that identity, and it
/// rides with the list rather than beside it so that the two cannot be paired up wrongly.
#[derive(Debug, Clone)]
pub(crate) struct Placed {
    /// Which page's picture this is, and of which ink.
    pub(crate) of: Picture,
    /// The commands, shared with `viewer-core` and with the render thread.
    pub(crate) list: Arc<DisplayList>,
    /// Where they go in this window, in its own device pixels.
    pub(crate) target: TargetSpec,
    /// Whether the pixels are this page at [`Self::target`], or a frame of the transition to it.
    pub(crate) depicts: Depicts,
}

/// What a [`Placed`] list's pixels are, which the window asks before it carries them anywhere.
///
/// **A transition frame carries the arriving page's [`Picture`] and is not a picture of it.**
/// §12.4.4's frame is two page rasters placed for the whole window, so no transform of it is any
/// view of either page: a reprojection composed from it moves a picture of *two* pages by one
/// page's placement, and a retained copy of it would stand in for the page later. Its identity is
/// still the arriving page's because the window does hold that page — the frame at progress 1 is
/// the page where it is about to be drawn — so a page being built behind it is not put up over it
/// part-way (ADR 1853).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Depicts {
    /// The page's own commands drawn at the target: what every view change is carried from.
    Page,
    /// A frame of §12.4.4's transition to the page, already in the window's own pixels.
    Transition,
}

impl Placed {
    /// Whether these two are the same interpretation of the same page at the same placement.
    ///
    /// **The `Arc`'s address and the target, which is the exact question** — see [`Picture`] for
    /// why the retained layer asks a different one. `render-raster` reuses a scene by the same
    /// address for the same ABA reason (ADR 0351).
    pub(crate) fn is(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.list, &other.list) && self.target == other.target
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
    /// `render-raster` reuses a scene by, for the same ABA reason (ADR 0351). **A list since ADR
    /// 0442**: `OneColumn` puts several pages in one texture, and which pages those are is what
    /// decides whether one placement carries the whole picture.
    ///
    /// **The base still asks the `Arc`, and ADR 0457 did not change that**: [`Picture`] is what
    /// makes a *retained page* findable across a re-interpretation, and this layer is a picture of
    /// one arrangement drawn at one moment, which is an exact question.
    pages: Vec<Placed>,
}

/// What drawing a stand-in costs the thread that presents — rule 4's whole question.
///
/// **The two windows of this program differ in exactly this and in nothing else** (`doc/todo/37`,
/// ADR 0461). They used to differ in the *arrangement* as well — one drew its stand-in beside the
/// frame it stood in for and the other in front of it — and since the processor's window has a
/// composing thread of its own both draw beside. What is left is the price, and the price is four
/// orders of magnitude apart, which is why there is still an answer to give here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Standing {
    /// **Three textured quads**, issued by the thread holding the surface while the device draws
    /// on another. Tenths of a millisecond, and nothing the frame it stands in for was going to
    /// use — a bound on that is not a bound (ADR 0391). Every run with a graphics device.
    Quads,
    /// **A window of pixels, resampled by the processor** on the thread that presents. Tens of
    /// milliseconds: the render is on another thread, so what standing in can cost the real frame
    /// is `max(0, resample − frame)` rather than the whole of the resample — bounded rather than
    /// zero, which is why rule 4 stays a question here. `--cpu`, and a machine whose device would
    /// not come up.
    Resample,
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
    /// **Deliberately not the last present's cost, and the difference is a frame the owner waited
    /// through.** A frame whose page, placement, size and chrome are the ones already on the
    /// screen costs a replay of an encode that exists (ADR 0351) — two milliseconds where the
    /// render was seven hundred — and a view change never replays, because its placement is part
    /// of the key. So the previous *present* is a bad predictor of the next *render* exactly when
    /// something harmless was redrawn in between: in the owner's own trace a window focus event
    /// redrew the launch frame in 2.1 ms, and the zoom step after it was judged against that
    /// rather than against the 778.6 ms rendering it replayed. `None` until a frame has built one.
    building: Option<Duration>,
    /// What a resample of one window of pixels costs on the processor, once one has been drawn.
    ///
    /// **Rule 4's measurement, and `None` permits** ([`Refusal::TooDear`]). It is a property of the
    /// machine rather than of a frame, so it is kept here beside [`Self::building`] and for the
    /// same reason: the question is what the *next* one will cost. Never set on a run with a
    /// graphics device, where nothing resamples on the presenting thread and rule 4 is enforced by
    /// the arrangement instead.
    resampling: Option<Duration>,
    /// Whether rule 4 has refused against [`Self::resampling`] since the last frame landed — the
    /// sample is then spent when the view it refused lands, not at the next tick ([`Self::declined`]).
    spent: bool,
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
    /// **Two ways of knowing, and the second is the one that cannot be wrong.** `out_for`
    /// says how long the render asked for at an earlier tick has been out — and only one
    /// out for *more than a period* has missed a refresh. The bare fact of being in
    /// flight is not a miss: on the owner's 120 Hz display a view change's render is
    /// typically collected one tick after it was asked for, so a boolean here made the
    /// tick in between observe a two-millisecond-old render as late and stand in for a
    /// frame that landed well inside the refresh. The prediction from [`Self::expected`]
    /// is what answers the first tick of a view change, before anything has been asked
    /// for and while there is nothing yet to observe; without it every gesture would
    /// begin with one frozen refresh.
    fn missed(&self, period: Duration, out_for: Option<Duration>) -> bool {
        out_for.is_some_and(|out| out > period) || self.expected() > period
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
    fn carries(settled: &Settled, pages: &[Placed]) -> Result<(), Refusal> {
        carrying(&settled.pages, pages).map(|_| ())
    }

    /// **Rule 4.** Whether standing in buys at least the refresh it costs, on this surface.
    ///
    /// `Ok(())` for [`Standing::Quads`] without looking at anything, because there is nothing to
    /// look at: a cost that is structurally zero is not a cost to bound. `Ok(())` for an
    /// unmeasured resample, because drawing one is the only thing that can produce the number.
    fn affordable(&self, standing: Standing, period: Duration) -> Result<(), Refusal> {
        let (Standing::Resample, Some(resample)) = (standing, self.resampling) else {
            return Ok(());
        };
        let frame = self.expected();
        if resample.saturating_add(period) > frame {
            return Err(Refusal::TooDear {
                resample,
                period,
                frame,
            });
        }
        Ok(())
    }

    pub(crate) fn plan(
        &self,
        pages: &[Placed],
        covered: usize,
        period: Duration,
        out_for: Option<Duration>,
        standing: Standing,
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
        // the owner asked for, so there is nothing to stand in for.
        //
        // It gates the retained pages exactly as it gates the base, and for the same reason: a
        // blurred page shown for one refresh and replaced is worse than the refresh spent waiting.
        if !self.missed(period, out_for) {
            return Plan::Refused(Refusal::InsideTheRefresh {
                frame: self.expected(),
                period,
            });
        }
        // Which way the miss was known, so the trace can say the truth: "expected to
        // cost X, so it misses" on a tick where the prediction was under the period and
        // the *observation* fired was a sentence about the wrong number.
        let missed = if self.expected() > period {
            Missed::Predicted {
                frame: self.expected(),
            }
        } else {
            Missed::Observed {
                out: out_for.unwrap_or_default(),
            }
        };
        // Rule 4, and it is a question again on exactly one surface (ADR 0457). On the window with
        // a device this reads nothing and refuses nothing — the render is on another thread and
        // what standing in costs it is zero, which ADR 0391 is right that no bound improves. On
        // the processor's window the resample is in front of the frame rather than beside it, so
        // it has to buy the refresh it spends. The two are one call because the *rule* is one
        // rule; what differs is the arrangement it is asked about.
        if let Err(why) = self.affordable(standing, period) {
            return Plan::Refused(why);
        }
        Plan::Approximate(match base {
            Ok(()) if covered > 0 => Stand {
                from: Source::LastFrameOverPages,
                instead_of_the_base: None,
                missed,
            },
            Ok(()) => Stand {
                from: Source::LastFrame,
                instead_of_the_base: None,
                missed,
            },
            Err(why) => Stand {
                from: Source::RetainedPages,
                instead_of_the_base: Some(why),
                missed,
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
        // **Rule 4's sample is spent by the refusal it makes** (ADR 0461). Holding the *last*
        // resample for the rest of the run would let one sample taken while the machine was busy
        // refuse every view change after it — and refusing is exactly what stops another sample
        // being taken. ADR 0461's measured run shows it: a stand-in of 348.7 ms under a load
        // average of 56, and every page turn for the remaining five seconds refused against that
        // number while frames of 45 ms went by.
        //
        // "Unmeasured permits" answered the *first* sample and nothing after it. So a refusal
        // gives up its own reason: the next view change permits, measures, and decides on a number
        // that is about the machine as it is now. The cost of being wrong is what rule 4 bounds —
        // `max(0, resample − frame)` on one view change — and the cost of being locked shut is the
        // whole feature, which is the trade ADR 0384 already made once.
        //
        // **Spent when the refused view's frame lands, not at the next tick** (ADR 1832). Cleared
        // here, the very next tick of the same view change permitted, resampled and blocked this
        // thread for the stand-in it had just called unwise: at a 2500 × 1600 window a 180 ms
        // resample against a 34 ms frame, so every notch of a zoom put up the true frame a
        // resample late. What ADR 0461 asks for is the *next view change* to measure afresh.
        if matches!(why, Refusal::TooDear { .. }) {
            self.spent = true;
        }
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
    ///
    /// **A transition frame that has landed settles nothing**, and the window then holds no view
    /// to carry: the frame after it is the page itself or nothing, never a picture of two pages
    /// moved by one page's placement — which is what went up for one refresh at the end of every
    /// §12.4.4 transition when this recorded it (ADR 1853). Nor is its cost a page's, so it does
    /// not feed the prediction either. [`Self::forget`] cannot see to it, because a frame lands
    /// on a later tick than the one that asked for it.
    pub(crate) fn settled(&mut self, pages: &[Placed], cost: Duration, built: bool) {
        if pages
            .iter()
            .any(|placed| placed.depicts == Depicts::Transition)
        {
            self.settled = None;
            return;
        }
        if std::mem::take(&mut self.spent) {
            self.resampling = None;
        }
        if built {
            self.building = Some(cost);
        }
        self.settled = Some(Settled {
            pages: pages.to_vec(),
        });
    }

    /// Records what a resample of one window of pixels cost on the processor — rule 4's sample.
    ///
    /// Taken from the stand-in that was actually drawn, on the surface it was drawn on, rather
    /// than from anything this file assumed: ADR 0384's defect was a bar built out of an assumed
    /// cost, and the fix is not a better assumption but a measurement of the thing itself.
    pub(crate) fn resampled(&mut self, cost: Duration) {
        self.resampling = Some(cost);
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
    /// The answer is in the page texture's own texel space, which is the window's: raster's
    /// `Layer::placement` maps a layer's texels to the surface's pixels, and the identity puts
    /// texel (0, 0) at the window's corner. So a frame of the view being asked for composes to the
    /// identity, exactly as it should.
    ///
    /// The three ways there is no picture are the three ways a rendering is unusable: there has
    /// never been one, the page changed, or the window did.
    pub(crate) fn reproject(&self, pages: &[Placed]) -> Result<Carried, Refusal> {
        let Some(settled) = self.settled.as_ref() else {
            return Err(Refusal::NothingRendered);
        };
        carrying(&settled.pages, pages)
    }
}
