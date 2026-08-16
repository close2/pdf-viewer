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
//! | 3. it says so | the frame line's outcome word is `approximated`, and [`Stale::count`] is what the summary prints |
//! | 4. it costs the real frame nothing | the pixels come from the encode quorra has **already** retained (a replay, never an encode), they are read back **once** per real frame rather than once per reprojection ([`Base`]), and the threshold below is measured against what a reprojection actually cost |
//! | 5. it does not fire when it is not needed | [`Stale::threshold`] — [`SHARE`] times a *measured* cost, not a taste |
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
//! finally lands, [`Stale::settled`] replaces the whole [`Settled`], the base with it, and the
//! next reprojection composes against the new placement even though the view has moved on.
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

/// What share of the frame it stands in for a reprojection may cost: one part in this many.
///
/// `doc/todo/37`'s fourth rule — "if the reprojection cannot be produced within a small fraction
/// of the frame it replaces, it is not produced at all" — as arithmetic. A tenth is the reading
/// of "a small fraction" this project takes, and the number it is a tenth *of* is measured
/// ([`Stale::threshold`]), so the rule that decides whether to approximate is a measurement with
/// one ratio on top of it rather than a duration somebody liked.
const SHARE: u32 = 10;

/// What a reprojection is taken to cost before this run has measured one.
///
/// **Measured, on the witness `doc/todo/37` names**, rather than chosen: seven reprojections over
/// four scripted sessions of that drawing under `Xvfb` at 900×1100 on llvmpipe, each reading the
/// presented 800×1000 frame back off the device and presenting it again as one image, cost
/// **23.1 to 50.6 ms** — of which the readback is 19.2 to 35.9 — against frames of 492 to 1036 ms
/// (ADR 0378's table). The bootstrap is the **top** of that band, because rule 4 is a bound and a
/// bound reads the worst case, and it is replaced by this run's own worst measurement as soon as
/// there is one. So a machine slower than this software adapter raises its own bar rather than
/// showing approximations it cannot afford, and a machine with a real graphics device lowers it
/// after the first step.
const ASSUMED: Duration = Duration::from_millis(51);

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
/// **This is what makes a reprojection compose rather than chain**, and it is why it is a field of
/// [`Settled`] rather than of [`Stale`]: the pixels and the placement they were drawn at are one
/// fact, and recording a new real frame replaces both together. Nothing in this file can hold a
/// base whose placement is not the placement it was captured at.
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
}

impl Base {
    /// The base a captured frame makes, or `None` for a raster this cannot read.
    ///
    /// [`RasterFormat`] is `#[non_exhaustive]`, so a second layout can arrive without this file
    /// changing, and drawing bytes under the wrong interpretation would put a plausible-looking
    /// wrong picture on the screen — precisely what this module is not allowed to do, even in its
    /// own approximate register. Checked here, once, rather than on every reprojection.
    fn of(raster: &Raster) -> Option<Self> {
        if raster.format != RasterFormat::Rgba8 || raster.width == 0 || raster.height == 0 {
            return None;
        }
        Some(Self {
            pixels: raster.data.as_slice().into(),
            width: raster.width,
            height: raster.height,
        })
    }
}

/// The view one frame drew: which page's display list, placed where, what it cost, and its pixels.
#[derive(Debug)]
struct Settled {
    /// The page, by the `Arc` that makes its address mean something — the identity
    /// `render-quorra` reuses a scene by, for the same ABA reason (ADR 0351).
    page: Arc<DisplayList>,
    /// Where it was placed, in this window's own device pixels.
    target: TargetSpec,
    /// What the frame that drew it cost, which is what rule 5's threshold is compared against.
    cost: Duration,
    /// This frame's own pixels, read back the first time a reprojection needed them.
    ///
    /// `None` until then — nothing is captured for a frame no view change ever stands in for,
    /// which is every frame of a window nobody is touching.
    base: Option<Base>,
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
    /// The most expensive reprojection this run has drawn, which is what the threshold is built
    /// from. The worst rather than the last: rule 4 is a bound, and a bound reads the worst case.
    measured: Option<Duration>,
    /// How many have been drawn — rule 3's count, which the frame summary prints.
    count: u64,
    /// Whether the pixels could not be had for a replay, after which none are asked for again.
    ///
    /// A capture that re-encodes has cost a whole frame of exactly the work the reprojection
    /// exists to hide, so rule 4 says it must not happen twice.
    refused: bool,
}

impl Stale {
    /// How slow the last real frame has to have been before an approximation is worth drawing.
    ///
    /// [`SHARE`] times what a reprojection costs on *this* machine, with [`ASSUMED`] standing in
    /// until one has been drawn. That is rule 5's "a measurement rather than a taste": a machine
    /// whose reprojection costs 2 ms approximates a 20 ms frame, and one whose reprojection costs
    /// 51 ms — this software adapter's worst — leaves everything under 510 ms alone.
    pub(crate) fn threshold(&self) -> Duration {
        self.measured.unwrap_or(ASSUMED).saturating_mul(SHARE)
    }

    /// The transform that puts the pixels now on the screen where this view wants them, or
    /// `None` where no reprojection is to be drawn.
    ///
    /// Every refusal is one of the five rules, in the order that rejects soonest.
    pub(crate) fn plan(&self, page: &Arc<DisplayList>, target: TargetSpec) -> Option<Transform> {
        // The pixels could not be had cheaply on this machine, so none are asked for again.
        if self.refused {
            return None;
        }
        // Rule 1, in the form `doc/todo/36` leaves it. A second reprojection is *allowed* — the
        // owner asked for one explicitly — but only where the view has moved again: one that
        // depicts the view being asked for has already answered it, and drawing it a second time
        // would be a window that had stopped drawing the document.
        if self.showing == Some(target.transform) {
            return None;
        }
        let settled = self.settled.as_ref()?;
        // A different page is not this page moved. Nothing about the outgoing page's pixels
        // says anything true about the incoming one, at any placement.
        if !Arc::ptr_eq(&settled.page, page) {
            return None;
        }
        // A resize changes what the window is as well as where the page is in it, and the
        // captured raster is the old window's own pixels.
        if settled.target.width != target.width || settled.target.height != target.height {
            return None;
        }
        // Nothing moved: this is a redraw of the view already on the screen, and quorra replays
        // it for the price of a replay (ADR 0351). There is nothing to stand in for.
        if settled.target.transform == target.transform {
            return None;
        }
        // Rule 5: a frame the machine delivers quickly must show that frame and never an
        // approximation of the one before it.
        if settled.cost < self.threshold() {
            return None;
        }
        // **Composed against the last real frame and never against the picture on the screen.**
        // This one expression is the whole of `doc/todo/36`'s "compose, do not chain": whatever
        // the window is showing, the transform carries the pixels of the *rendering* onto the
        // view being asked for, so a run of reprojections is a run of single resamples.
        let moved = settled.target.transform.invert()?.then(target.transform);
        // A placement with a coordinate that is not a finite number is not a placement. It
        // cannot arise from two invertible page transforms, and drawing one would hand the
        // scene boundary a value it would refuse mid-frame.
        [moved.a, moved.b, moved.c, moved.d, moved.e, moved.f]
            .iter()
            .all(|coefficient| coefficient.is_finite())
            .then_some(moved)
    }

    /// What the frame a reprojection stands in for cost: the last real frame's own duration.
    ///
    /// Zero before there has been one, which is a state [`Self::plan`] has already refused in.
    pub(crate) fn covering(&self) -> Duration {
        self.settled
            .as_ref()
            .map_or(Duration::ZERO, |settled| settled.cost)
    }

    /// Forgets what the window is showing, so that nothing is reprojected from it.
    ///
    /// For the frames whose pixels are not a page at a placement: §12.4.4's transition is a
    /// picture of *two* pages moving, and no transform of it is any view of either.
    pub(crate) fn forget(&mut self) {
        self.settled = None;
    }

    /// Whether the next reprojection needs the frame on the window read back.
    ///
    /// True exactly once per real frame: the first reprojection standing in for it captures the
    /// base, and every later one resamples what that capture holds. **The condition is also what
    /// makes a chain impossible**, and not merely unlikely — the only moment a capture is asked
    /// for is the moment the window is showing a rendering, because a base exists for every real
    /// frame from the first reprojection of it onward.
    pub(crate) fn wants_base(&self) -> bool {
        self.settled
            .as_ref()
            .is_some_and(|settled| settled.base.is_none())
    }

    /// Keeps the pixels the last real frame put on the window, for every reprojection of it.
    ///
    /// `false` for a raster this cannot read, which is a refusal to draw rather than a failure.
    pub(crate) fn rebase(&mut self, raster: &Raster) -> bool {
        let Some(settled) = self.settled.as_mut() else {
            return false;
        };
        settled.base = Base::of(raster);
        settled.base.is_some()
    }

    /// Records that a reprojection was drawn, which view it depicts, and what the whole of it
    /// cost.
    ///
    /// The cost is the pixels *and* the frame that put them up, because that is what the next
    /// [`Self::threshold`] has to be a tenth of.
    pub(crate) fn drawn(&mut self, view: Transform, cost: Duration) -> MustFollow {
        self.showing = Some(view);
        self.count = self.count.saturating_add(1);
        self.measured = Some(self.measured.map_or(cost, |worst| worst.max(cost)));
        MustFollow(())
    }

    /// Records that the pixels could not be had cheaply, so none will be asked for again.
    pub(crate) fn refuse(&mut self) {
        self.refused = true;
    }

    /// Records the view a real frame drew, and what that frame cost.
    ///
    /// **This is where a late frame re-bases** (`doc/todo/36`'s third point). Whatever the view
    /// has moved on to, the frame that has just landed is the truest picture this window holds,
    /// so it replaces the whole [`Settled`] — the base with it — and the reprojection after it
    /// composes against *this* placement.
    pub(crate) fn settled(&mut self, page: &Arc<DisplayList>, target: TargetSpec, cost: Duration) {
        self.settled = Some(Settled {
            page: Arc::clone(page),
            target,
            cost,
            // Captured on demand, and only where a view change asks for one: a window nobody is
            // touching reads nothing back.
            base: None,
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

    /// The one-image frame that puts the last real frame's pixels where `moved` says.
    ///
    /// **The only way out of this module for a picture**, and the reason it is a method rather
    /// than a free function taking pixels: a caller cannot pass in a raster of its own, so no
    /// caller can resample anything but the base — which is what makes "compose, do not chain" a
    /// property of the type instead of a rule somebody has to follow.
    ///
    /// `None` where no base has been captured, which [`Self::wants_base`] is asked first to
    /// avoid.
    pub(crate) fn reproject(&self, moved: Transform) -> Option<DisplayList> {
        let base = self.settled.as_ref()?.base.as_ref()?;
        Some(reprojection(base, moved))
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

    use super::{SHARE, Stale, reprojection};

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

    /// A frame that took ten times what a reprojection costs, which is the case the feature is
    /// for: the view moved, the page did not, and the machine will be most of a second.
    fn slow(stale: &mut Stale, page: &Arc<DisplayList>) {
        let cost = stale.threshold();
        stale.settled(page, view(1.0), cost);
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

    /// Rule 1, as `doc/todo/36` leaves it. A reprojection is never the state the window settles
    /// in: the view it already depicts is drawn rather than approximated a second time.
    #[test]
    fn a_reprojection_is_never_the_last_word() {
        let page = page();
        let mut stale = Stale::default();
        slow(&mut stale, &page);
        assert!(
            stale.plan(&page, view(1.2)).is_some(),
            "a slow frame and a new magnification is what this exists for"
        );
        let follow = stale.drawn(view(1.2).transform, Duration::from_millis(20));
        assert!(stale.showing_approximation());
        assert!(
            stale.plan(&page, view(1.2)).is_none(),
            "the view on the screen has been answered; drawing it again would be a window that \
             had stopped drawing the document"
        );
        drop(follow);
        // Every frame that is not one clears it, including a frame that drew nothing: the guard
        // in `about_to_wait` asks for a redraw while this holds, and a flag that could outlive
        // the redraw answering it would spin the loop.
        stale.real();
        assert!(!stale.showing_approximation());
    }

    /// Rule 5. The threshold is [`SHARE`] times a measured cost, and a frame under it is shown
    /// rather than approximated.
    #[test]
    fn nothing_is_approximated_below_the_measured_threshold() {
        let page = page();
        let mut stale = Stale::default();
        let quick = stale.threshold() / 2;
        stale.settled(&page, view(1.0), quick);
        assert!(
            stale.plan(&page, view(1.2)).is_none(),
            "a view whose frame is quick must show that frame"
        );
        stale.settled(&page, view(1.0), stale.threshold());
        assert!(stale.plan(&page, view(1.2)).is_some());
    }

    /// Rule 4, from the other side: what the threshold is a multiple *of* is this run's own
    /// worst reprojection, so an approximation that turned out expensive raises the bar it must
    /// clear next time rather than being repeated at the real frame's expense.
    #[test]
    fn the_threshold_follows_what_a_reprojection_actually_cost() {
        let mut stale = Stale::default();
        let assumed = stale.threshold();
        let expensive = Duration::from_millis(120);
        drop(stale.drawn(view(1.2).transform, expensive));
        assert_eq!(stale.threshold(), expensive.saturating_mul(SHARE));
        assert!(stale.threshold() > assumed);
        // The worst, not the last: a cheap one after an expensive one does not lower the bound.
        drop(stale.drawn(view(1.3).transform, Duration::from_millis(3)));
        assert_eq!(stale.threshold(), expensive.saturating_mul(SHARE));
    }

    /// A different page is not this page moved, and no placement makes it one.
    #[test]
    fn a_page_turn_is_never_reprojected() {
        let first = page();
        let second = page();
        let mut stale = Stale::default();
        slow(&mut stale, &first);
        assert!(stale.plan(&second, view(1.0)).is_none());
        assert!(stale.plan(&second, view(1.3)).is_none());
    }

    /// A resize changes the window the pixels were captured from, and a view that did not move
    /// has nothing to stand in for.
    #[test]
    fn neither_a_resize_nor_a_still_view_is_reprojected() {
        let page = page();
        let mut stale = Stale::default();
        slow(&mut stale, &page);
        assert!(
            stale.plan(&page, view(1.0)).is_none(),
            "the view already on the screen is drawn, not approximated"
        );
        let resized = TargetSpec {
            width: 900,
            ..view(1.2)
        };
        assert!(stale.plan(&page, resized).is_none());
    }

    /// The transform is the one that carries the old view's device pixels onto the new view's,
    /// which is what makes the picture *move* rather than merely appear.
    #[test]
    fn the_transform_carries_old_device_pixels_onto_new_ones() {
        let page = page();
        let mut stale = Stale::default();
        slow(&mut stale, &page);
        let moved = stale
            .plan(&page, view(2.0))
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
        let base = super::Base::of(&pixels).expect("an RGBA8 raster");
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
        assert!(super::Base::of(&empty).is_none());
        let page = page();
        let mut stale = Stale::default();
        slow(&mut stale, &page);
        assert!(
            !stale.rebase(&empty),
            "a raster this cannot read is no base"
        );
        assert!(stale.reproject(Transform::IDENTITY).is_none());
    }

    /// `doc/todo/36`'s second point, which is the one that decides how the picture degrades: a
    /// reprojection of a reprojection resamples the **base** and not the picture on the screen,
    /// so two in a row are two single resamples rather than a chain of two.
    ///
    /// Read off the transforms rather than off the pixels, because that is where the property
    /// lives: each `moved` carries a page point through the *rendering's* placement onto the
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
            let moved = stale
                .plan(&page, asked)
                .expect("the view keeps moving and the frame stays slow");
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
        assert!(stale.reproject(Transform::IDENTITY).is_some());
        drop(stale.drawn(view(1.2).transform, Duration::from_millis(2)));
        assert!(
            !stale.wants_base(),
            "every later reprojection of one rendering resamples what the first captured"
        );
        assert!(stale.reproject(Transform::IDENTITY).is_some());
    }

    /// `doc/todo/36`'s third point. A delayed frame becomes the base the moment it lands, even
    /// though the view has moved on — its pixels are truer than the ones being reprojected, and
    /// the composed transform simply changes.
    #[test]
    fn a_late_frame_becomes_the_base() {
        let page = page();
        let mut stale = Stale::default();
        slow(&mut stale, &page);
        assert!(stale.rebase(&captured()));
        drop(stale.drawn(view(1.2).transform, Duration::from_millis(2)));
        assert!(!stale.wants_base());
        // The frame for the 1.2× view finally lands, while the person has already asked for 2×.
        stale.settled(&page, view(1.2), stale.threshold());
        stale.real();
        assert!(
            stale.wants_base(),
            "the pixels of the frame that has just landed are the ones to resample now"
        );
        let moved = stale.plan(&page, view(2.0)).expect("the view moved on");
        let corner = pdf_render::Point::new(100.0, 700.0);
        let through = moved.apply(view(1.2).transform.apply(corner));
        let directly = view(2.0).transform.apply(corner);
        assert!(
            (through.x - directly.x).abs() < 1e-3 && (through.y - directly.y).abs() < 1e-3,
            "composed against the frame that landed, not against the one before it: \
             {through:?} {directly:?}"
        );
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
