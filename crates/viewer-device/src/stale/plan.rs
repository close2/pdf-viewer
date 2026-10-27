//! What one tick puts up — the frame on hand, a stand-in, or nothing new — and every refusal of a
//! stand-in, said by name and counted by kind (rule 3).

use std::time::Duration;

use pdf_render::Transform;

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
///
/// [`Stale::declined`]: super::Stale::declined
#[derive(Debug, Clone, PartialEq)]
pub enum Plan {
    /// Stand in for the frame this view is waiting for, from the layers named.
    ///
    /// **It carries no placement**, since ADR 0391, and it carries no pixels: what a stand-in
    /// actually needs is the *placements* that carry the pictures held onto this view, and the
    /// only ways to obtain one are [`Stale::reproject`] for the base and [`Proxies::placements`]
    /// for the pages under it — each composed against the picture it is of. A payload a caller
    /// could pass on instead would be a second route to a placement, and there is deliberately
    /// one apiece.
    ///
    /// [`Stale::reproject`]: super::Stale::reproject
    /// [`Proxies::placements`]: super::Proxies::placements
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
pub struct Stand {
    /// Which layers have pixels for this view.
    pub from: Source,
    /// Why the last rendering does not carry onto this view, where it does not.
    ///
    /// `Some` is a picture drawn from the retained pages alone — a page turn, a `GoTo`, a resize,
    /// a zoom in a column on the processor's window — and it is still a refusal of the *sharp* layer, so it is said out loud
    /// and counted ([`Stale::without_base`]). What it is no longer is a blank window.
    ///
    /// [`Stale::without_base`]: super::Stale::without_base
    pub instead_of_the_base: Option<Refusal>,
    /// Which way rule 5 knew the frame had missed, for the trace to say truthfully.
    pub missed: Missed,
}

/// How rule 5 knew the frame this stand-in covers for had missed its refresh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missed {
    /// By prediction: the last built frame's cost exceeds the period.
    Predicted {
        /// What the frame this view is waiting for is expected to cost.
        frame: Duration,
    },
    /// By observation: the render asked for at an earlier tick is out past one period.
    Observed {
        /// How long it has been out.
        out: Duration,
    },
}

impl std::fmt::Display for Missed {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Predicted { frame } => write!(
                formatter,
                "this view's frame is expected to cost {:.1} ms",
                frame.as_secs_f64() * 1e3
            ),
            Self::Observed { out } => write!(
                formatter,
                "the render asked for {:.1} ms ago is still out",
                out.as_secs_f64() * 1e3
            ),
        }
    }
}

/// Which of the two approximate layers filled the picture — rule 3's distinction.
///
/// The order is the amount of wrong: the last frame moved is blurred by the magnification and no
/// more, the pages under it are a whole page at [`PROXY_EDGE`] pixels along its longer side, and
/// a picture made of those alone is the blurrier of the two everywhere.
///
/// [`PROXY_EDGE`]: super::PROXY_EDGE
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
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
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::LastFrame => "approximated",
            Self::LastFrameOverPages => "approximated, over a retained page",
            Self::RetainedPages => "approximated from a retained page",
        }
    }

    /// Whether the last rendering's own texture is one of the layers.
    #[must_use]
    pub fn has_base(self) -> bool {
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
pub enum Refusal {
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
    /// either side of it do. **It is the refusal of a window that resamples its pixels**: a window
    /// that draws in quads carries the picture a page at a time instead (ADR 2416). A scroll moves every page by the same distance, so what separates its
    /// placements is arithmetic rather than arrangement and `AGREEMENT` absorbs it. The two
    /// placements are carried because a reader has to be able to see *how* they differ, and
    /// [`Self::Rearranged::by`] because how far apart they are is what the refusal turns on.
    Rearranged {
        /// The placement the first page of the arrangement asked for.
        first: Transform,
        /// The placement a later page asked for, which is not the same one.
        then: Transform,
        /// How far apart the two put a texel of the picture held, in device pixels, at the worst
        /// corner of it — `placement::disagreement`'s answer, which is what exceeded `AGREEMENT`.
        by: f32,
    },
    /// *Impossible.* The presenter declined to put the approximated frame on the window.
    DeviceRefused(String),
    /// *Unwise.* Rule 4: the resample would not be finished a refresh before the frame it stands
    /// in for.
    ///
    /// **Deleted by ADR 0391, back for one surface in ADR 0457, and re-derived in ADR 0461.** The
    /// deletion was right about the window it was written for: a stand-in there is three textured
    /// quads issued by the thread holding the surface while the render is on another, so what it
    /// costs the frame it stands in for is nothing and a bound on nothing is not a bound. ADR 0457
    /// brought it back for the processor's window because that window had no other thread and its
    /// resample delayed the frame by exactly what it cost.
    ///
    /// **That window has a composing thread now, and the rule survives its own premise.** The
    /// resample runs on the presenting thread beside the render rather than in front of it, so
    /// what standing in costs the real frame is `max(0, resample − frame)`: nothing at all while
    /// the resample finishes first, and the overrun where it does not. So the question is no
    /// longer *does it buy the refresh it spends* but *is it up in time to be seen at all* — and
    /// both are the same inequality, which is why the arithmetic did not move:
    ///
    /// `resample + period ≤ frame`, in the display's own unit and with no constant in it. Where it
    /// does not hold, a person would be shown a blurred picture for less than one refresh and then
    /// made to wait longer for the true one than they would have waited without it.
    ///
    /// **Unmeasured permits**, which is the other half of ADR 0384's lesson: the only thing that
    /// can produce the first measurement is drawing one, so a gate that refused until it had a
    /// number would block its own only sample. See `Stale::resampling` — and [`Stale::declined`]
    /// for why a refusal then gives that sample up, which is the same lesson about every sample
    /// after the first.
    ///
    /// [`Stale::declined`]: super::Stale::declined
    TooDear {
        /// What the last resample on this surface cost.
        resample: Duration,
        /// One refresh of this surface, which is what standing in has to buy.
        period: Duration,
        /// What the frame this view is waiting for is expected to cost.
        frame: Duration,
    },
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
            Self::TooDear { .. } | Self::InsideTheRefresh { .. } => "unwise",
        }
    }

    /// Whether this was a judgement rather than an impossibility, for the summary's tally.
    pub(super) fn judged(&self) -> bool {
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
            Self::Rearranged { first, then, by } => write!(
                formatter,
                "Table 29's arrangement is not one affine of itself — the first page of the \
                 picture held moves by ({:.3} {:.3} {:.3} {:.3} {:.3} {:.3}) and a later one by \
                 ({:.3} {:.3} {:.3} {:.3} {:.3} {:.3}), {by:.4} px apart at the worst corner of \
                 the picture, and the window puts its pages up as one texture under one placement",
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
            Self::TooDear {
                resample,
                period,
                frame,
            } => write!(
                formatter,
                "standing in would cost {:.1} ms on this thread against a frame of {:.1} ms being \
                 drawn on another, so it would not be up a whole {:.1} ms refresh before the true \
                 one — the picture would be blurred for less than a refresh and the frame later \
                 by the overrun",
                ms(*resample),
                ms(*frame),
                ms(*period)
            ),
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
pub struct Refusals {
    /// Refusals where there was genuinely nothing true to draw.
    pub impossible: u64,
    /// Refusals that were a judgement between two measurements.
    pub unwise: u64,
    /// Refusals of the *sharp* layer that the retained pages stood in for anyway.
    ///
    /// **Counted apart from the two above, because it answers a different question** (ADR 0443).
    /// Those two are view changes the window did not move for; this one moved, from the blurrier
    /// source, and a person reading a trace of a page turn needs to be able to tell "nothing was
    /// shown" from "a low-resolution page was shown". It is deliberately not in [`Self::total`].
    pub proxied: u64,
}

impl Refusals {
    /// Every view change this run showed the real frame for rather than standing in.
    #[must_use]
    pub fn total(self) -> u64 {
        self.impossible.saturating_add(self.unwise)
    }
}

/// How many stand-ins this run drew, split by what they were made of — rule 3's count.
#[derive(Debug, Default, Clone, Copy)]
pub struct Drawn {
    /// The last rendering's own texture, moved, with nothing under it.
    pub last_frame: u64,
    /// The same, over retained pages filling what it did not cover.
    pub over_pages: u64,
    /// Retained pages alone, where the last rendering said nothing about the view.
    pub pages_only: u64,
}

impl Drawn {
    /// Every stand-in, whatever it was made of.
    #[must_use]
    pub fn total(self) -> u64 {
        self.last_frame
            .saturating_add(self.over_pages)
            .saturating_add(self.pages_only)
    }

    /// Records one of `from`.
    pub(super) fn record(&mut self, from: Source) {
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
    pub(super) fn stands_in(&self) -> bool {
        matches!(self, Self::Approximate(_))
    }
}
