use std::sync::Arc;
use std::time::Duration;

use pdf_render::{DisplayList, Size, TargetSpec, Transform};
use viewer_core::DocumentId;

use super::placement::AGREEMENT;
use super::{
    Depicts, Missed, Picture, Placed, Plan, Proxies, Refusal, Retained, Source, Stale, Stand,
    Standing,
};

/// What a stand-in costs the presenting thread, in every test that is not about rule 4: the
/// window with a graphics device, which is the one every case here was written for.
const QUADS: Standing = Standing::Quads;

/// The frame trace of a run that asked for none, which says nothing.
#[derive(Clone, Copy)]
struct Off;

impl crate::FrameLines for Off {
    fn say(self, _: std::fmt::Arguments<'_>) {}
}

/// A page of a document as these tests hold one: its commands, and which page of what it is.
///
/// **The second half is what ADR 0457 added**, and it is why this is a type rather than an
/// `Arc`: the identity a retained picture is kept under is the page and the ink rather than
/// the address of the commands, so a test that means "another page" has to be able to say so
/// in the terms the code now uses.
#[derive(Clone)]
struct Sheet {
    of: Picture,
    list: Arc<DisplayList>,
}

impl Sheet {
    /// This page, placed — one entry of Table 29's arrangement.
    fn placed(&self, target: TargetSpec) -> Placed {
        Placed {
            of: self.of,
            list: Arc::clone(&self.list),
            target,
            depicts: Depicts::Page,
        }
    }
}

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

/// A fresh page of a fresh document's arrangement — A4, with a page number nothing else has.
///
/// The number comes from a counter rather than from an argument so that two pages made here
/// are two *pages*, which is what a page-turn case is about. The ink is zero throughout: no
/// test in this file edits a document, and a case about the ink says so by naming it.
fn page() -> Sheet {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    Sheet {
        of: Picture::new(DocumentId(1), NEXT.fetch_add(1, Ordering::Relaxed), 0),
        list: Arc::new(DisplayList::new(Size::new(595.0, 842.0))),
    }
}

/// Table 29's default arrangement: one page, at a magnification.
///
/// Every test but the column's below asks about this, because `SinglePage` is what the table
/// states as its own default and what every document that says nothing opens in.
fn alone(page: &Sheet, magnification: f32) -> Vec<Placed> {
    vec![page.placed(view(magnification))]
}

/// One page at a placement stated outright, for the two cases a magnification cannot express:
/// a window that changed shape, and a transform that does not invert.
fn at(page: &Sheet, target: TargetSpec) -> Vec<Placed> {
    vec![page.placed(target)]
}

/// A column of `pages`, laid out as `viewer_core::layout` lays one out: each page under the
/// same magnification, stacked down the window with `GAP` logical pixels between them, the
/// whole scrolled by `scroll` device pixels.
///
/// The gap is 8 and it is **not** multiplied by the magnification, which is that module's own
/// documented choice — "a gap is a thing a person sees" — and the whole reason a zoom in a
/// column is not one affine of itself.
fn column(pages: &[Sheet], magnification: f32, scroll: f32) -> Vec<Placed> {
    let mut placed = Vec::new();
    let mut top = -scroll;
    for page in pages {
        placed.push(
            page.placed(TargetSpec {
                width: 800,
                height: 1000,
                transform: Transform::scale(magnification, -magnification)
                    .then(Transform::translate(0.0, 842.0 * magnification))
                    .then(Transform::translate(0.0, top)),
            }),
        );
        top += 842.0 * magnification + 8.0;
    }
    placed
}

/// A column laid out by `viewer_core` over a document committed in `doc/`, as a host holds it.
///
/// **The arrangement comes from the document rather than from this file**, which is the whole
/// point of it: [`column`] above stacks exact translations and cannot produce the residual at
/// all, because nothing in it ever divides. A real placement is a magnification `Zoom::FitPage`
/// chose out of a page's own extent, and the composition [`one_placement`] takes goes through
/// `Transform::invert`'s division by a determinant — which is where the last bits go.
///
/// One entry per page the arrangement shows, in page order, taken from the render requests
/// exactly the way `quorra::dispatch` takes them: a request for a page already held replaces
/// that page's, and the pages the arrangement no longer shows are dropped.
struct Column {
    viewer: viewer_core::Viewer,
    held: Vec<(usize, Picture, Arc<DisplayList>, TargetSpec)>,
}

impl Column {
    /// `doc/PDF20_AN001-BPC.pdf` in `OneColumn`, in an 800×1000 window.
    ///
    /// Five pages of A4 with text on every one, and the document every headless test opens —
    /// not a corpus file, because the corpus is an optional submodule and a test that skipped
    /// itself silently would be worse than no test.
    fn opened() -> Self {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf");
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|error| panic!("{} is committed: {error}", path.display()));
        let mut column = Self {
            viewer: viewer_core::Viewer::new(800, 1000, 1.0),
            held: Vec::new(),
        };
        column.send(viewer_core::Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        });
        column.send(viewer_core::Command::Layout(
            pdf_model::viewer_preferences::PageLayout::OneColumn,
        ));
        column
    }

    /// Sends one command and folds what it asked to be drawn into the arrangement held.
    fn send(&mut self, command: viewer_core::Command) -> Vec<Placed> {
        let events: Vec<_> = self.viewer.handle(command).collect();
        for event in events {
            if let viewer_core::Event::NeedsRender(request) = event {
                // The identity the host builds, out of the request that asked for the
                // render — `quorra::surface::App::arrangement` composes exactly this.
                let of = Picture::new(request.document, request.page, request.ink);
                let entry = (request.page, of, request.list, request.target);
                match self
                    .held
                    .binary_search_by_key(&request.page, |(at, ..)| *at)
                {
                    Ok(at) => self.held[at] = entry,
                    Err(at) => self.held.insert(at, entry),
                }
            }
        }
        // The placement `quorra::surface::Surface::arrangement` builds, and it is built here
        // rather than borrowed because that method needs a window: the core states one raster
        // per page and where in the viewport it goes, and the host composes the two into the
        // window's own extent. A page the arrangement has scrolled off has no geometry, which
        // is what says a request this host still holds has left the screen (ADR 0442).
        let mut placed = Vec::new();
        self.held.retain(|(page, of, list, target)| {
            let viewer_core::Answer::Geometry(geometry) =
                self.viewer.query(viewer_core::Query::PageGeometry(*page))
            else {
                return false;
            };
            placed.push(Placed {
                of: *of,
                list: Arc::clone(list),
                target: TargetSpec {
                    width: 800,
                    height: 1000,
                    transform: target
                        .transform
                        .then(Transform::translate(geometry.origin.0, geometry.origin.1)),
                },
                depicts: Depicts::Page,
            });
            true
        });
        placed
    }
}

/// **A scroll of a *real* column is carried, and a zoom of one is still refused.**
///
/// This is the case ADR 0442's exact comparison got wrong and [`AGREEMENT`] answers: on a
/// continuous column every page moves by the same distance, and the compositions still differ
/// in the last bits of an `f32` because each goes through an inverse. A measured trace has it —
/// two placements printed identically to three decimals and refused.
///
/// It is also the **measurement**, printed rather than asserted at a number: the two
/// populations either side of the bound, over a range of scrolls and of magnifications, on a
/// document rather than on this file's arithmetic. What is asserted is the property each
/// population has — a scroll agrees, a zoom does not — and never a figure, because a figure
/// asserted here is a threshold fitted to a run (`doc/traps/pixels-and-rasterisers.md` trap
/// 12).
#[test]
fn a_scroll_of_a_real_column_is_carried_and_a_zoom_of_one_is_still_refused() {
    use viewer_core::{Command, Zoom};

    let mut column = Column::opened();
    let mut settled = column.send(Command::Zoom {
        zoom: Zoom::Scale(0.4),
        at: None,
    });
    let mut stale = Stale::default();
    let (mut carried, mut refused) = (Vec::new(), Vec::new());
    for step in 0..34_u32 {
        // Two gestures over one document: a scroll of a distance nothing here rounds, and
        // every fourth step a zoom in, so that the magnifications the scrolls are taken at
        // are the ones a person passes through rather than one this test chose.
        let gesture = if step % 4 == 3 {
            Command::Zoom {
                zoom: Zoom::In,
                at: None,
            }
        } else {
            Command::Scroll {
                dx: 0.0,
                dy: -137.0,
            }
        };
        let zoomed = matches!(gesture, Command::Zoom { .. });
        stale.settled(&settled, Duration::from_millis(700), true);
        let asked = column.send(gesture);
        if asked.len() < 2 {
            settled = asked;
            continue;
        }
        match stale.reproject(&asked) {
            Ok(one) => carried.push((zoomed, asked.len(), one.within)),
            Err(Refusal::Rearranged { by, .. }) => refused.push((zoomed, asked.len(), by)),
            Err(other) => {
                // A page turn is not what either gesture is, and a resize cannot happen here.
                panic!("step {step}: {other:?}");
            }
        }
        settled = asked;
    }
    let worst = |of: &[(bool, usize, f32)], zoomed: bool| {
        of.iter()
            .filter(|(was_zoom, ..)| *was_zoom == zoomed)
            .map(|(_, _, by)| *by)
            .fold(f32::NEG_INFINITY, f32::max)
    };
    let least = |of: &[(bool, usize, f32)], zoomed: bool| {
        of.iter()
            .filter(|(was_zoom, ..)| *was_zoom == zoomed)
            .map(|(_, _, by)| *by)
            .fold(f32::INFINITY, f32::min)
    };
    println!(
        "carried {} scrolls and {} zooms, disagreeing by {:.6} to {:.6} px\n\
         refused {} scrolls and {} zooms, disagreeing by {:.4} to {:.4} px\n\
         the bound is {AGREEMENT} px",
        carried.iter().filter(|(zoom, ..)| !zoom).count(),
        carried.iter().filter(|(zoom, ..)| *zoom).count(),
        least(&carried, false).min(least(&carried, true)),
        worst(&carried, false).max(worst(&carried, true)),
        refused.iter().filter(|(zoom, ..)| !zoom).count(),
        refused.iter().filter(|(zoom, ..)| *zoom).count(),
        least(&refused, false).min(least(&refused, true)),
        worst(&refused, false).max(worst(&refused, true)),
    );
    assert!(
        carried.iter().filter(|(zoom, ..)| !zoom).count() >= 12,
        "the scrolls of a five-page column are the population this is about: {carried:?}"
    );
    assert!(
        !carried.iter().any(|(zoom, ..)| *zoom),
        "a zoom moves the gap and not the pages, so no one placement carries it: {carried:?}"
    );
    assert!(
        !refused.iter().any(|(zoom, ..)| !zoom),
        "a scroll moves every page by the same distance and must not be refused: {refused:?}"
    );
    assert!(
        refused.iter().filter(|(zoom, ..)| *zoom).count() >= 4,
        "every zoom of a column of more than one page rearranges it: {refused:?}"
    );
}

/// One refresh of a 60 Hz surface, which is what rule 5 compares a frame against.
const REFRESH: Duration = Duration::from_nanos(1_000_000_000 / 60);

/// The render thread is idle: every render this view asked for has landed.
///
/// The state of the *first* tick of a view change, which is the one only the prediction can
/// answer — nothing is out yet, so there is nothing to observe having missed.
const LANDED: Option<Duration> = None;

/// A render asked for at an earlier tick is out past one refresh, so it has observably
/// missed it — whatever the prediction said.
const DRAWING: Option<Duration> = Some(Duration::from_secs(1));

/// A render is in flight but has been out for well under one refresh: nothing has been
/// observed to miss yet, so the prediction alone decides.
const JUST_ASKED: Option<Duration> = Some(Duration::from_millis(2));

/// A frame that missed the refresh by a long way, which is the case the feature is for: the
/// view moved, the page did not, and the machine will be most of a second.
fn slow(stale: &mut Stale, page: &Sheet) {
    stale.settled(&alone(page, 1.0), Duration::from_millis(700), true);
}

/// A scroll's stand-in moves texels without resampling them: the reprojection of
/// a translation-only view change is constructed as the exact translation and
/// snapped to whole device pixels — the owner's "scrolling with the mouse still
/// shows blurry text", fixed at the placement rather than in the sampler. The
/// half-pixel it costs lives one refresh; the blur it removes covered every glyph.
#[expect(
    clippy::float_cmp,
    reason = "snapping to whole device pixels is the behaviour under test, so the expected \
              values are exact in binary and a tolerance would hide a snap that stopped \
              snapping"
)]
#[test]
fn a_scroll_stand_in_lands_on_whole_pixels() {
    let page = page();
    let mut stale = Stale::default();
    let magnification = 1.3_f32;
    stale.settled(
        &alone(&page, magnification),
        Duration::from_millis(700),
        true,
    );
    // The same magnification, scrolled by a fraction of a pixel: only the
    // translation moves, which is what a wheel tick does.
    let mut scrolled = view(magnification);
    scrolled.transform.f += 37.4;
    let carried = stale
        .reproject(&at(&page, scrolled))
        .expect("a scroll carries");
    assert_eq!(
        (carried.placement.a, carried.placement.b),
        (1.0, 0.0),
        "constructed as the exact translation, not an inverse's survivor"
    );
    assert_eq!((carried.placement.c, carried.placement.d), (0.0, 1.0));
    assert_eq!(
        carried.placement.f, 37.0,
        "snapped to the pixel row, so the bilinear tap resamples nothing"
    );
    assert_eq!(carried.placement.e, 0.0);
    // A zoom is not snapped: its stand-in is scaled and resampled either way, and
    // moving it would add error for nothing.
    let zoomed = stale
        .reproject(&alone(&page, 1.45))
        .expect("a zoom carries");
    assert!(
        (zoomed.placement.a - 1.45 / 1.3).abs() < 1e-5,
        "the scale survives untouched: {}",
        zoomed.placement.a
    );
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
            .plan(&alone(&page, 1.2), NONE_HELD, REFRESH, LANDED, QUADS)
            .stands_in(),
        "a slow frame and a new magnification is what this exists for"
    );
    let follow = stale.drawn(Source::LastFrame);
    assert!(stale.showing_approximation());
    assert!(
        stale
            .plan(&alone(&page, 1.2), NONE_HELD, REFRESH, DRAWING, QUADS)
            .stands_in(),
        "the same view again while the render is still out: the same three quads, which is \
         what a picture every refresh means"
    );
    // And the view the *rendering* is of goes up as it is, which is the answer that must not
    // be given to any other view.
    assert_eq!(
        stale.plan(&alone(&page, 1.0), NONE_HELD, REFRESH, DRAWING, QUADS),
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
        stale.plan(&alone(&page, 1.2), NONE_HELD, REFRESH, LANDED, QUADS),
        Plan::Refused(Refusal::InsideTheRefresh {
            frame: REFRESH / 2,
            period: REFRESH,
        }),
        "a view whose frame lands inside the refresh must show that frame, and say so"
    );
    stale.settled(&alone(&page, 1.0), REFRESH * 2, true);
    assert!(
        stale
            .plan(&alone(&page, 1.2), NONE_HELD, REFRESH, LANDED, QUADS)
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
            .plan(&alone(&page, 1.2), NONE_HELD, REFRESH, LANDED, QUADS)
            .stands_in(),
        "the prediction says this one lands in time, so the first tick waits for it"
    );
    assert!(
        !stale
            .plan(&alone(&page, 1.2), NONE_HELD, REFRESH, JUST_ASKED, QUADS)
            .stands_in(),
        "a render out for two milliseconds has missed nothing: the observation is \
         'out past one refresh', not 'in flight' — a boolean here made the tick \
         between the ask and the collect stand in for a frame that landed well \
         inside the refresh (found on the owner's 120 Hz display, ADR 0704)"
    );
    assert!(
        stale
            .plan(&alone(&page, 1.2), NONE_HELD, REFRESH, DRAWING, QUADS)
            .stands_in(),
        "and once it is out past a whole refresh, it has been watched missing"
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
                .plan(&alone(&page, 1.2), NONE_HELD, REFRESH, LANDED, QUADS)
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
            .plan(&alone(&page, 1.2), NONE_HELD, REFRESH, LANDED, QUADS)
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
                .plan(&alone(&page, 1.3), NONE_HELD, period, LANDED, QUADS)
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
/// frame, raster replayed it in 2.1 ms, and the zoom step after it was judged against that
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
            .plan(&alone(&page, 1.2), NONE_HELD, REFRESH, LANDED, QUADS)
            .stands_in(),
        "the zoom after a harmless redraw is the one the owner waited through"
    );
    // A frame that genuinely built a cheap picture *does* move it.
    stale.settled(&alone(&page, 1.0), REFRESH / 2, true);
    assert_eq!(stale.expected(), REFRESH / 2);
    assert!(
        !stale
            .plan(&alone(&page, 1.3), NONE_HELD, REFRESH, LANDED, QUADS)
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
        stale.plan(&alone(&second, 1.0), NONE_HELD, REFRESH, LANDED, QUADS),
        Plan::Refused(Refusal::AnotherPage)
    );
    assert_eq!(
        stale.plan(&alone(&second, 1.3), NONE_HELD, REFRESH, LANDED, QUADS),
        Plan::Refused(Refusal::AnotherPage)
    );
    assert_eq!(
        stale.reproject(&alone(&second, 1.3)).unwrap_err(),
        Refusal::AnotherPage,
        "a rendering of the outgoing page places nothing of the incoming one"
    );
}

/// §12.5.3's re-interpretation: a zoom of a `NoZoom` page makes a new `Arc` over the same
/// ink, and that is this page moved — not another page. The pairing is by [`Picture`], so
/// the address does not decide it in either direction: a fresh list of the same picture
/// carries, and the old list of a superseded ink refuses, which is what the address test
/// was actually guarding.
#[test]
fn a_reinterpretation_of_the_same_picture_is_still_this_page() {
    let page = page();
    let mut stale = Stale::default();
    slow(&mut stale, &page);
    // The re-interpretation: same picture, another list over its commands.
    let reinterpreted = Sheet {
        of: page.of,
        list: Arc::new(DisplayList::new(Size::new(595.0, 842.0))),
    };
    assert!(
        stale
            .plan(
                &alone(&reinterpreted, 1.3),
                NONE_HELD,
                REFRESH,
                LANDED,
                QUADS
            )
            .stands_in(),
        "a new list over the same ink is this page zoomed, and it stands in"
    );
    assert!(
        stale.reproject(&alone(&reinterpreted, 1.3)).is_ok(),
        "and the placement comes out of the same gate"
    );
    // The ink moved: a picture of the old ink may not stand in, whatever its address.
    let other_ink = Sheet {
        of: Picture::new(DocumentId(1), page.of.page(), 1),
        list: Arc::clone(&page.list),
    };
    assert_eq!(
        stale.plan(&alone(&other_ink, 1.3), NONE_HELD, REFRESH, LANDED, QUADS),
        Plan::Refused(Refusal::AnotherPage),
        "the same address over superseded ink is not a picture of this page any more"
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
        stale.plan(&alone(&page, 1.0), NONE_HELD, REFRESH, LANDED, QUADS),
        Plan::Render,
        "the view already on the screen is drawn, not approximated — and not refused either"
    );
    let resized = TargetSpec {
        width: 900,
        ..view(1.2)
    };
    assert_eq!(
        stale.plan(&at(&page, resized), NONE_HELD, REFRESH, LANDED, QUADS),
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
            .plan(&alone(&page, 2.0), NONE_HELD, REFRESH, LANDED, QUADS)
            .stands_in()
    );
    let moved = stale
        .reproject(&alone(&page, 2.0))
        .expect("a doubled magnification")
        .placement;
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
/// raster's `Layer::placement` maps the layer's own texels to the surface's pixels, so the
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
        .expect("the view the rendering is of")
        .placement;
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
/// earn raster's own `LayerProblem::Placement` mid-frame instead of a sentence this program
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
                .plan(&at(&page, asked), NONE_HELD, REFRESH, DRAWING, QUADS)
                .stands_in(),
            "the view keeps moving and the frame stays slow"
        );
        let moved = stale
            .reproject(&at(&page, asked))
            .expect("a rendering is held")
            .placement;
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
            .plan(&alone(&page, 2.0), NONE_HELD, REFRESH, LANDED, QUADS)
            .stands_in()
    );
    let moved = stale
        .reproject(&alone(&page, 2.0))
        .expect("the view moved on")
        .placement;
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
    assert!(
        stale
            .plan(&asked, NONE_HELD, REFRESH, LANDED, QUADS)
            .stands_in()
    );
    let moved = stale
        .reproject(&asked)
        .expect("a column that scrolled")
        .placement;
    let corner = pdf_render::Point::new(100.0, 700.0);
    for (index, (was, now)) in settled.iter().zip(&asked).enumerate() {
        let through = moved.apply(was.target.transform.apply(corner));
        let directly = now.target.transform.apply(corner);
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
        stale.plan(
            &column(&pages, 1.25, 0.0),
            NONE_HELD,
            REFRESH,
            LANDED,
            Standing::Resample
        ),
        Plan::Refused(refused),
        "a window that resamples its pixels has one placement or none"
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
            .plan(
                &column(&pages[..1], 1.25, 0.0),
                NONE_HELD,
                REFRESH,
                LANDED,
                QUADS
            )
            .stands_in(),
        "one page has one placement and always did"
    );
}

/// **A zoom of a real column stands in from its last frame, each page carried to where the view
/// puts it** — the owner's "zooming out, always shows the blurry frame" in a column (ADR 2408 §3,
/// ADR 2416).
///
/// One placement for the whole picture puts the second page of a column `GAP × (1 − k)` device
/// pixels from where a zoom by `k` puts it, past [`AGREEMENT`], because `viewer_core::layout`'s
/// gap does not scale; so the last frame used to be refused and the window stood in from the
/// 512-pixel retained pages alone. On a surface that draws in quads each page of the last frame
/// now goes up under its own placement, showing its own rectangle only. What is asserted is
/// arithmetic on the arrangement the core states, never a figure fitted to a run: the plan stands
/// in from the last frame; every page the view and the picture share gets a part; each part's
/// corners land where the view puts that page's corners, to within [`AGREEMENT`]; and no two parts
/// share a texel, so no page's pixels are drawn beside another page.
#[test]
fn a_zoom_of_a_real_column_stands_in_from_its_last_frame_page_by_page() {
    use viewer_core::{Command, Zoom};

    let mut column = Column::opened();
    let mut settled = column.send(Command::Zoom {
        zoom: Zoom::Scale(0.4),
        at: None,
    });
    let mut checked = 0;
    for zoom in [Zoom::In, Zoom::In, Zoom::Out, Zoom::Out, Zoom::Out] {
        let mut stale = Stale::default();
        stale.settled(&settled, Duration::from_millis(700), true);
        let asked = column.send(Command::Zoom { zoom, at: None });
        let shared = asked
            .iter()
            .filter(|page| settled.iter().any(|held| held.of == page.of))
            .count();
        assert_eq!(
            stale.plan(&asked, NONE_HELD, REFRESH, LANDED, QUADS),
            Plan::Approximate(Stand {
                from: Source::LastFrame,
                instead_of_the_base: None,
                missed: Missed::Predicted {
                    frame: stale.expected(),
                },
            }),
            "{zoom:?}: the last frame stands in, rather than nothing or the retained pages"
        );
        let carry = stale
            .reproject_on_quads(&asked)
            .unwrap_or_else(|why| panic!("{zoom:?}: {why}"));
        let super::Carry::ByPage { parts, apart } = &carry else {
            panic!("{zoom:?}: a zoom of a column has no one placement: {carry:?}");
        };
        assert!(*apart > AGREEMENT, "{zoom:?}: {apart}");
        assert_eq!(parts.len(), shared, "{zoom:?}: one part a page held");
        for (part, page) in parts.iter().zip(
            asked
                .iter()
                .filter(|page| settled.iter().any(|held| held.of == page.of)),
        ) {
            let held = settled
                .iter()
                .find(|held| held.of == page.of)
                .expect("a page the picture holds");
            let size = page.list.page_size;
            for (x, y) in [
                (0.0, 0.0),
                (size.width, 0.0),
                (0.0, size.height),
                (size.width, size.height),
            ] {
                let corner = pdf_render::Point::new(x, y);
                let through = part.placement.apply(held.target.transform.apply(corner));
                let wanted = page.target.transform.apply(corner);
                let off = (through.x - wanted.x).hypot(through.y - wanted.y);
                assert!(
                    off <= AGREEMENT,
                    "{zoom:?}: page {} lands {off} px from where the view puts it",
                    page.of.page() + 1
                );
            }
        }
        for (index, one) in parts.iter().enumerate() {
            for other in &parts[index + 1..] {
                let ([a_left, a_top, a_right, a_bottom], [b_left, b_top, b_right, b_bottom]) = (
                    one.texels.expect("a page's part"),
                    other.texels.expect("a page's part"),
                );
                let overlap =
                    a_left < b_right && b_left < a_right && a_top < b_bottom && b_top < a_bottom;
                assert!(!overlap, "{zoom:?}: {one:?} and {other:?} share texels");
            }
        }
        checked += parts.len();
        settled = asked;
    }
    assert!(
        checked >= 10,
        "five zooms of a column of two pages or more: {checked} parts"
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
        stale.plan(&asked, NONE_HELD, REFRESH, LANDED, QUADS),
        Plan::Render,
        "a third page on the screen is not the frame that showed two"
    );
    assert!(
        stale
            .plan(&asked, NONE_HELD, REFRESH, LANDED, QUADS)
            .stands_in()
    );
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
                .plan(
                    &alone(&page, magnification),
                    NONE_HELD,
                    REFRESH,
                    DRAWING,
                    QUADS
                )
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
    let trace = Off;
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
            stale.plan(&alone(&page, 1.0), NONE_HELD, REFRESH, LANDED, QUADS),
            Plan::Render,
            "the same view, redrawn: raster replays it and nothing is being stood in for"
        );
    }
    assert_eq!(stale.refusals().total(), 0);
}

/// A store of pictures whose "pixels" are the page's index, so that a placement can be
/// checked against the page it belongs to without a graphics device.
fn retaining(pages: &[Sheet]) -> Proxies<usize> {
    let mut proxies = Proxies::new(pages.len());
    for (index, page) in pages.iter().enumerate() {
        proxies.keep(Retained {
            of: page.of,
            target: super::proxy_target(&page.list).expect("a page with an extent"),
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
    let held = retaining(std::slice::from_ref(&second));
    let asked = alone(&second, 1.0);
    let under = held.placements(&asked);
    assert_eq!(under.len(), 1, "the incoming page's own picture is held");
    assert_eq!(
        stale.plan(&asked, under.len(), REFRESH, LANDED, QUADS),
        Plan::Approximate(Stand {
            from: Source::RetainedPages,
            instead_of_the_base: Some(Refusal::AnotherPage),
            missed: Missed::Predicted {
                frame: stale.expected(),
            },
        }),
        "the sharp layer is still impossible and says so; the window still moves"
    );
    assert_eq!(
        stale.plan(&asked, NONE_HELD, REFRESH, LANDED, QUADS),
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
    let held = retaining(std::slice::from_ref(&page));
    let corner = pdf_render::Point::new(100.0, 700.0);
    for magnification in [0.3_f32, 1.0, 2.5] {
        let asked = alone(&page, magnification);
        let placed = held.placements(&asked);
        let (index, moved) = placed.first().copied().expect("one page, one placement");
        let proxy = held.get(index).expect("the index it handed back").target;
        let through = moved.apply(proxy.transform.apply(corner));
        let directly = asked[0].target.transform.apply(corner);
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
        let target = asked
            .iter()
            .find(|placed| placed.of == retained.of)
            .expect("a page of the arrangement")
            .target;
        let through = moved.apply(retained.target.transform.apply(corner));
        let directly = target.transform.apply(corner);
        assert!(
            (through.x - directly.x).abs() < 1e-2 && (through.y - directly.y).abs() < 1e-2,
            "page {index}: {through:?} {directly:?}"
        );
    }
    // The last frame goes over them a page at a time (ADR 2416); a window that resamples its
    // pixels shows them alone.
    assert_eq!(
        stale.plan(&asked, 2, REFRESH, LANDED, QUADS),
        Plan::Approximate(Stand {
            from: Source::LastFrameOverPages,
            instead_of_the_base: None,
            missed: Missed::Predicted {
                frame: stale.expected(),
            },
        }),
    );
    assert_eq!(
        stale.plan(&asked, 2, REFRESH, LANDED, Standing::Resample),
        Plan::Approximate(Stand {
            from: Source::RetainedPages,
            instead_of_the_base: Some(match stale.reproject(&asked) {
                Err(rearranged @ Refusal::Rearranged { .. }) => rearranged,
                other => panic!("a zoom of a column rearranges: {other:?}"),
            }),
            missed: Missed::Predicted {
                frame: stale.expected(),
            },
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
        stale.plan(&alone(&second, 1.0), 1, REFRESH, LANDED, QUADS),
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
    let arrangement: Vec<Placed> = pages.iter().map(|page| page.placed(view(1.0))).collect();
    for (index, page) in pages.iter().enumerate() {
        assert!(
            proxies
                .wanted(&arrangement)
                .is_some_and(|asked| asked.of == page.of),
            "the first page with no picture, in the arrangement's own order"
        );
        proxies.keep(Retained {
            of: page.of,
            target: super::proxy_target(&page.list).expect("a page with an extent"),
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

/// §12.4.4's transition frame lands as a picture of no view, so nothing is carried from it.
///
/// The owner's report (2026-10-12): the last frame of a transition showed "something
/// incorrect" for a fraction of a second. That frame was this window reprojecting the last
/// transition frame — two pages already in window pixels, at the identity — by the arriving
/// page's own placement, because the frame had been recorded as that page settled at the
/// identity. A frame that has landed settles nothing, and the retained layer never keeps one
/// (ADR 1853).
#[test]
fn a_transition_frame_settles_no_view_and_is_never_retained() {
    let arriving = page();
    let frame = Placed {
        of: arriving.of,
        list: Arc::new(DisplayList::new(Size::new(800.0, 1000.0))),
        target: TargetSpec {
            width: 800,
            height: 1000,
            transform: Transform::IDENTITY,
        },
        depicts: Depicts::Transition,
    };
    let mut stale = Stale::default();
    slow(&mut stale, &page());
    assert!(
        stale.has_rendering(),
        "the page before the transition is a view"
    );
    stale.settled(std::slice::from_ref(&frame), Duration::from_millis(2), true);
    assert!(
        !stale.has_rendering(),
        "a transition frame is no view of the page it moves to, so none is held to carry"
    );
    let proxies: Proxies<usize> = Proxies::new(2);
    assert!(
        proxies.wanted(std::slice::from_ref(&frame)).is_none(),
        "a retained copy of a transition frame would stand in for its page later"
    );
    assert!(
        proxies
            .wanted(&alone(&arriving, 1.0))
            .is_some_and(|asked| asked.of == arriving.of),
        "the page itself is still wanted"
    );
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
                    && in_raster.x <= f32::from(u16::try_from(target.width).expect("small")) + 0.5
                    && in_raster.y <= f32::from(u16::try_from(target.height).expect("small")) + 0.5,
                "{corner:?} left the raster at {in_raster:?}"
            );
        }
    }
}

impl Sheet {
    /// The same page of the same document, interpreted again — a **new** `Arc` over the same
    /// commands, which is what `viewer_core` hands back when a page leaves the arrangement
    /// and comes back to it.
    fn interpreted_again(&self) -> Self {
        Self {
            of: self.of,
            list: Arc::new(DisplayList::new(Size::new(595.0, 842.0))),
        }
    }

    /// The same page after something changed its ink: an edit, an undo, a layer switched.
    fn after_an_edit(&self) -> Self {
        Self {
            of: Picture {
                ink: self.of.ink.saturating_add(1),
                ..self.of
            },
            list: Arc::new(DisplayList::new(Size::new(595.0, 842.0))),
        }
    }
}

/// **The `SinglePage` page turn, which is what ADR 0457 is for.** A page left and returned to
/// is interpreted again and arrives as a different `Arc` over the same commands, so a picture
/// found by that address would show nothing at all and print `another page — nothing to show`.
///
/// Both halves are asserted here because the second is what makes the first safe: the same
/// page under changed **ink** is a different picture and must not be found.
#[test]
fn a_page_returned_to_finds_its_picture_and_a_page_edited_does_not() {
    let page = page();
    let held = retaining(std::slice::from_ref(&page));
    let again = page.interpreted_again();
    assert!(
        !Arc::ptr_eq(&page.list, &again.list),
        "a page turn produces a new interpretation, which is the whole premise"
    );
    assert_eq!(
        held.placements(&alone(&again, 1.0)).len(),
        1,
        "the same page of the same document under the same ink is the same picture"
    );
    assert!(
        held.placements(&alone(&page.after_an_edit(), 1.0))
            .is_empty(),
        "changed ink is a different picture, and a stand-in may not assert the old one"
    );
}

/// And a picture of superseded ink is *dropped* rather than left to age out, because a slot it
/// occupies is a slot a showable page wants.
#[test]
fn a_picture_of_superseded_ink_leaves_the_store_when_the_next_one_arrives() {
    let (first, second) = (page(), page());
    let mut proxies = retaining(&[first.clone(), second.clone()]);
    assert_eq!(proxies.len(), 2);
    let edited = second.after_an_edit();
    proxies.keep(Retained {
        of: edited.of,
        target: super::proxy_target(&edited.list).expect("a page with an extent"),
        pixels: 9,
    });
    assert_eq!(
        proxies.len(),
        1,
        "both pictures of the ink that has gone are dropped, not one of them"
    );
    assert!(proxies.placements(&alone(&first, 1.0)).is_empty());
    assert_eq!(proxies.placements(&alone(&edited, 1.0)).len(), 1);
}

/// **Rule 4, on one surface and on one surface only** (ADR 0391 deleted it, ADR 0457 restored
/// it for the processor's window, ADR 0461 re-derived it once that window had a composing
/// thread). A resample there runs on the presenting thread beside the frame it stands in for,
/// so it has to be finished a refresh before that frame lands or it is a picture nobody sees.
///
/// The three states are the whole rule: unmeasured permits, because drawing one is the only
/// thing that can produce the number; a resample finished in time is drawn; one that is not is
/// refused, and the refusal carries all three durations it judged.
#[test]
fn a_resample_beside_the_frame_has_to_be_up_before_it() {
    let page = page();
    let mut stale = Stale::default();
    // A frame of four refreshes: rule 5 says it misses, so rule 4 is the next question.
    stale.settled(&alone(&page, 1.0), REFRESH * 4, true);
    let asked = alone(&page, 1.2);
    assert!(
        stale
            .plan(&asked, NONE_HELD, REFRESH, LANDED, Standing::Resample)
            .stands_in(),
        "unmeasured permits, or the gate would block its own only sample"
    );
    stale.resampled(REFRESH * 2);
    assert!(
        stale
            .plan(&asked, NONE_HELD, REFRESH, LANDED, Standing::Resample)
            .stands_in(),
        "two refreshes of resample plus one of refresh, against a frame of four: it is up a \
         whole refresh before the true one"
    );
    stale.resampled(REFRESH * 4);
    assert_eq!(
        stale.plan(&asked, NONE_HELD, REFRESH, LANDED, Standing::Resample),
        Plan::Refused(Refusal::TooDear {
            resample: REFRESH * 4,
            period: REFRESH,
            frame: REFRESH * 4,
        }),
        "a resample as long as the frame lands with it, so nobody sees it and the frame is \
         late by the overrun"
    );
    assert_eq!(
        stale.refusals().unwise,
        0,
        "counted by the caller, not here"
    );
    // **And the refusal gives up the sample it refused on** (ADR 0461) — when the view it
    // refused has landed, and not at the next tick of the same view change (ADR 1832). A
    // measurement taken while the machine was busy would otherwise refuse every view change
    // after it, and refusing is what stops another one being taken — ADR 0384's defect a
    // layer along. The caller is what says the refusal happened, so the caller is where it is
    // given up.
    stale.declined(
        &Refusal::TooDear {
            resample: REFRESH * 4,
            period: REFRESH,
            frame: REFRESH * 4,
        },
        Off,
    );
    assert!(
        !stale
            .plan(&asked, NONE_HELD, REFRESH, LANDED, Standing::Resample)
            .stands_in(),
        "the same view change stays refused: a stand-in it called unwise a tick ago would hold \
         this thread for the resample and put the true frame up late by it"
    );
    stale.settled(&asked, REFRESH * 4, true);
    assert!(
        stale
            .plan(
                &alone(&page, 1.44),
                NONE_HELD,
                REFRESH,
                LANDED,
                Standing::Resample
            )
            .stands_in(),
        "the next view change measures again rather than deciding on a spent number"
    );
    // And the window with a device is untouched by any of it: what a stand-in costs the frame
    // it stands in for there is structurally nothing, and a bound on nothing is not a bound.
    assert!(
        stale
            .plan(&alone(&page, 1.44), NONE_HELD, REFRESH, LANDED, QUADS)
            .stands_in(),
        "rule 4 must not reach the surface ADR 0391 deleted it for"
    );
}

/// Rule 2, as far as a test can reach it: nothing outside the windows' own code names the
/// reprojection, so no gate, oracle, harness or diagnostic artefact can photograph one.
///
/// The structural half of the argument is that only a window depends on this library, which
/// [`nothing_but_a_window_depends_on_the_library_that_holds_the_stand_ins`] holds (ADR 2052). This
/// checks the other half: that no library a judge is built from grew its own copy of the idea
/// while nobody was looking.
#[test]
fn nothing_a_judge_is_built_from_knows_what_a_reprojection_is() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/");
    // Everything under `src/bin` is a binary target and can be a dependency of nothing, and this
    // crate is a dependency of windows alone: that is the boundary this test is about.
    let windows = [root.join("viewer-ui/src/bin"), root.join("viewer-device")];
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
                && !windows.iter().any(|window| path.starts_with(window))
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

/// Rule 2's structural half: **no manifest in the tree names this crate but a window's**, so the
/// corpus and oracle gates, `Query::Frame`, the confined worker and every harness are compiled
/// without a line of the stand-ins (ADRs 0378, 2052).
///
/// A binary could not be linked to in principle; a library can, so the principle is this test.
/// The population is every `Cargo.toml` under the repository, found by walking it, and a window
/// that comes to draw through this crate is added to `WINDOWS` by the round that argues it.
#[test]
fn nothing_but_a_window_depends_on_the_library_that_holds_the_stand_ins() {
    /// The manifests that may name this crate: the workspace's table of versions, `quorra`'s, and
    /// the two toolkit windows', which draw their pages on the device and read them back (ADR 2070).
    const WINDOWS: [&str; 4] = [
        "Cargo.toml",
        "crates/viewer-ui/Cargo.toml",
        "crates/viewer-gtk/Cargo.toml",
        "crates/viewer-qt/Cargo.toml",
    ];
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository");
    let mut naming: Vec<String> = Vec::new();
    let mut manifests = 0_usize;
    let mut stack = vec![repository.clone()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            // A link leaves the tree (`doc/`'s documents are links into the owner's checkout), and
            // build output, history and scratch are not manifests of this tree.
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                if name != "target" && name != ".git" && name != "scratchpad" {
                    stack.push(path);
                }
            } else if kind.is_file() && name == "Cargo.toml" {
                manifests = manifests.saturating_add(1);
                let names_this = std::fs::read_to_string(&path).is_ok_and(|manifest| {
                    manifest
                        .lines()
                        .any(|line| line.trim_start().starts_with("viewer-device"))
                });
                let relative = path.strip_prefix(&repository).map_or_else(
                    |_| path.display().to_string(),
                    |at| at.display().to_string(),
                );
                if names_this && !WINDOWS.contains(&relative.as_str()) {
                    naming.push(relative);
                }
            }
        }
    }
    assert!(
        manifests > 40,
        "the walk found {manifests} manifests, which is not the tree"
    );
    assert!(
        naming.is_empty(),
        "the stand-ins are a window's and nothing that judges a picture may link them, but these \
         manifests name `viewer-device`: {naming:?}"
    );
}

/// **ADR 0657's rule 1, in the four cases that decide it.** The condition
/// `quorra::composer::Composer::superseded` raises an interrupt on is the negation of this,
/// so what it must not answer `false` to is a view change whose frame is still a base — and
/// interrupting one of those would throw away pixels the next stand-in was going to use.
#[test]
fn a_frame_is_worth_finishing_exactly_where_its_pixels_could_still_stand_in() {
    let sheet = page();
    let next = page();
    let drawn = alone(&sheet, 1.0);
    assert!(
        super::could_stand_in(&drawn, &drawn),
        "the view has not moved at all, which is every frame of a document nobody is touching"
    );
    assert!(
        super::could_stand_in(&drawn, &alone(&sheet, 1.5)),
        "a zoom of a single page is one placement of itself, so the frame being drawn is a \
         base for the view being asked for and finishing it is not waste"
    );
    assert!(
        !super::could_stand_in(&drawn, &alone(&next, 1.0)),
        "a page turn is not: nothing about the outgoing page's pixels is true of the \
         incoming one, at any placement"
    );
    assert!(
        !super::could_stand_in(
            &drawn,
            &at(
                &sheet,
                TargetSpec {
                    width: 640,
                    height: 480,
                    transform: Transform::IDENTITY,
                },
            ),
        ),
        "and a resize is not, because what is drawn is the whole window's picture"
    );
}

/// The case that separates rule 1 from *interrupt whenever the view moved*, which is the
/// simpler policy it was chosen over.
///
/// A scroll of a column is the commonest view change there is, and its frame carries onto the
/// view that superseded it to within [`AGREEMENT`] — so the pixels are a base and the draw is
/// worth finishing. A zoom of a column is the case `Refusal::Rearranged` exists for: no one
/// affine carries it, the frame will be neither presented nor reprojected from, and the
/// thread is better spent on the view being asked for.
#[test]
fn a_scroll_of_a_column_is_worth_finishing_and_a_zoom_of_one_is_not() {
    let pages = [page(), page(), page()];
    let drawn = column(&pages, 1.0, 0.0);
    assert!(
        super::could_stand_in(&drawn, &column(&pages, 1.0, 137.0)),
        "every page moves by the same distance, so one placement carries the picture"
    );
    assert!(
        !super::could_stand_in(&drawn, &column(&pages, 1.1, 0.0)),
        "the gap between rows does not scale with the magnification, so no one affine does"
    );
}
