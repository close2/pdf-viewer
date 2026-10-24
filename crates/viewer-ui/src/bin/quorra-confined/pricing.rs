//! Which of two routes a page past the bound whose marks cross takes to the screen, priced by what
//! this window has measured (ADR 2017).
//!
//! A page past `viewer_core::MAX_PIXELS` whose marks cross is drawn by this window's device from
//! them at the window's size (ADR 1944), and every frame of it carries them across the pipe first;
//! or it is drawn by the worker as the window shows it, and only the window's pixels cross (ADR
//! 1986). On the owner's witness the two differ by more than the device's frame: its frames were
//! all inside `WARN`, and a scroll's next frame still came 166–180 ms after the press, the marks'
//! crossing in front of it, where the worker's window answered in 9–10 ms (ADR 2001 §2).
//!
//! **The crossing is counted in the device's prediction**, which is the project owner's answer to
//! `doc/questions/Q477`, "I agree with both recommendations" (`A477`), the recommendation being
//! "Count the crossing." and a price of both routes per page. So a page's device route is predicted as the bytes
//! that crossed for it times the crossing's measured rate, plus its commands times the device's
//! measured rate per command, and its window route as the largest window's pixels times the
//! window's rate. The window is asked for only where the device's prediction is past
//! `viewer_host::drawing::WARN` and the window's is the smaller; every other page keeps its marks,
//! which is `CLAUDE.md`'s rule that the device draws every frame it is predicted to draw inside
//! that bound (ADR 1831).
//!
//! **Each rate is the last one measured**, as `quorra`'s are (ADR 1886): a crossing from the
//! command sent to its frame decoded, over the bytes of the one page past the bound the frame
//! carried; a device frame over the commands of the marks it drew; and a window that crossed new,
//! from the command sent to its frame decoded, over its pixels. A frame carrying two pages past the
//! bound measures nothing, because what it took cannot be divided between them. Until a window
//! has been measured its rate is a seed, and until both device rates are measured the device is
//! not priced at all and the page keeps its marks.

use std::collections::BTreeMap;
use std::time::Duration;

use pdf_render::TargetSpec;
use viewer_confined::{Framed, Payload, Route};
use viewer_host::drawing::WARN;
use viewer_host::oversize;

/// The worker's window per pixel until one has been measured: 171 ns, the slower end of ADR
/// 1944's processor drawing a window of 1 201 × 1 501 pixels in 139–309 ms. The slower end,
/// because a seed that is too quick would take a page off the device before anything was
/// measured, and one too slow only waits for the first window to be measured.
const WINDOW_SEED: f64 = 309e-3 / (1201.0 * 1501.0);

/// What this window has measured of the two routes, and how each page past the bound was last
/// asked to cross.
#[derive(Debug, Default)]
pub(crate) struct Pricing {
    /// Seconds per byte, from a command sent to its frame decoded.
    crossing: Option<f64>,
    /// Seconds per command of a device frame.
    device: Option<f64>,
    /// Seconds per pixel of a window the worker drew, from a command sent to its frame decoded.
    window: Option<f64>,
    /// Each page past the bound on the screen, by index.
    pages: BTreeMap<usize, Priced>,
}

/// One page past the bound, as this window last saw it cross.
#[derive(Debug, Clone, Copy)]
struct Priced {
    /// The bytes and commands of its marks the last time they crossed, which price the device's
    /// route while the page crosses as windows.
    marks: Option<(usize, usize)>,
    /// The target it is drawn at, which places its largest window.
    target: TargetSpec,
    /// The last window that crossed of it, as its corner and extent: a window that crosses again
    /// unchanged was not drawn again, so it measures nothing.
    window: Option<(u32, u32, u32, u32)>,
    /// How it was last asked to cross.
    route: Route,
}

/// The two predictions for one page, and the route they choose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Forecast {
    /// The page, zero-based.
    pub(crate) page: usize,
    /// The device's route: the crossing and the device's frame.
    pub(crate) device: Duration,
    /// The worker's window.
    pub(crate) window: Duration,
    /// Whether the window's figure is measured rather than the seed.
    pub(crate) window_measured: bool,
    /// What the two choose.
    pub(crate) chosen: Route,
    /// Whether this is a change from how the page was last asked to cross, and is to be sent.
    pub(crate) changed: bool,
}

impl Pricing {
    /// Takes in one frame that crossed `took` after the command that caused it was sent, at a
    /// window `extent` device pixels across, and answers the predictions for every page past the
    /// bound it carries that can be priced.
    pub(crate) fn frame(
        &mut self,
        frames: &[Framed],
        took: Duration,
        extent: (u32, u32),
    ) -> Vec<Forecast> {
        let past: Vec<&Framed> = frames
            .iter()
            .filter(|framed| match &framed.payload {
                Payload::List { target, .. } | Payload::Window { target, .. } => {
                    past_the_bound(*target)
                }
                Payload::Raster(_) => false,
            })
            .collect();
        self.pages
            .retain(|page, _| past.iter().any(|framed| framed.page == *page));
        let alone = past.len() == 1;
        for framed in &past {
            self.saw(framed, alone.then_some(took));
        }
        past.iter()
            .filter_map(|framed| self.predict(framed.page, extent))
            .collect()
    }

    /// Takes in a device frame that drew `commands` commands of marks in `took`.
    pub(crate) fn device_drew(&mut self, took: Duration, commands: usize) {
        if let Some(rate) = per(took, commands) {
            self.device = Some(rate);
        }
    }

    /// Records that `page` was asked to cross as `route`.
    pub(crate) fn asked(&mut self, page: usize, route: Route) {
        if let Some(priced) = self.pages.get_mut(&page) {
            priced.route = route;
        }
    }

    /// One page past the bound as it crossed, and what it measured where it crossed alone.
    fn saw(&mut self, framed: &Framed, took: Option<Duration>) {
        // A page first seen is taken to cross as it did: a window is the worker's, asked for by
        // this window before it last priced the page, and a change from it is to be sent.
        let route = match framed.payload {
            Payload::Window { .. } => Route::Window,
            _ => Route::Marks,
        };
        let entry = |target| Priced {
            marks: None,
            target,
            window: None,
            route,
        };
        match &framed.payload {
            Payload::List { list, target } => {
                let priced = self
                    .pages
                    .entry(framed.page)
                    .or_insert_with(|| entry(*target));
                priced.marks = Some((framed.bytes, list.commands().len()));
                priced.target = *target;
                priced.window = None;
                if let Some(rate) = took.and_then(|took| per(took, framed.bytes)) {
                    self.crossing = Some(rate);
                }
            }
            Payload::Window {
                target,
                corner,
                raster,
            } => {
                let priced = self
                    .pages
                    .entry(framed.page)
                    .or_insert_with(|| entry(*target));
                let window = (corner.0, corner.1, raster.width, raster.height);
                let new = priced.window != Some(window);
                priced.target = *target;
                priced.window = Some(window);
                let pixels = usize::try_from(
                    u64::from(raster.width).saturating_mul(u64::from(raster.height)),
                );
                if new
                    && let Some(rate) = took
                        .zip(pixels.ok())
                        .and_then(|(took, pixels)| per(took, pixels))
                {
                    self.window = Some(rate);
                }
            }
            Payload::Raster(_) => {}
        }
    }

    /// The two predictions for `page` at a window `extent` across, where both device rates and
    /// the page's marks are known.
    fn predict(&self, page: usize, extent: (u32, u32)) -> Option<Forecast> {
        let priced = self.pages.get(&page)?;
        let (bytes, commands) = priced.marks?;
        let (crossing, device) = (self.crossing?, self.device?);
        #[expect(
            clippy::cast_precision_loss,
            reason = "a byte or command count is far below 2^52, where f64 stops counting by one"
        )]
        let device = Duration::from_secs_f64(crossing * bytes as f64 + device * commands as f64);
        let largest = oversize::largest_centred(priced.target, extent);
        #[expect(
            clippy::cast_precision_loss,
            reason = "a window's pixels are a few million, far below 2^52"
        )]
        let pixels = u64::from(largest.width).saturating_mul(u64::from(largest.height)) as f64;
        let rate = self.window.unwrap_or(WINDOW_SEED);
        let window = Duration::from_secs_f64(rate * pixels);
        let chosen = if device > WARN && window < device {
            Route::Window
        } else {
            Route::Marks
        };
        Some(Forecast {
            page,
            device,
            window,
            window_measured: self.window.is_some(),
            chosen,
            changed: chosen != priced.route,
        })
    }
}

/// Seconds per unit of `count`, or nothing for a count of zero.
fn per(took: Duration, count: usize) -> Option<f64> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a byte, command or pixel count is far below 2^52"
    )]
    (count > 0).then(|| took.as_secs_f64() / count as f64)
}

/// Whether a target's whole raster would be past `viewer_core::MAX_PIXELS`.
fn past_the_bound(target: TargetSpec) -> bool {
    u64::from(target.width).saturating_mul(u64::from(target.height)) > viewer_core::MAX_PIXELS
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use pdf_render::{DisplayList, Raster, RasterFormat, Size, TargetSpec, Transform};
    use viewer_confined::{Framed, Payload, Route};

    use super::{Pricing, WARN};

    /// A target past the bound: 20 000 pixels on a side, 4 × 10⁸ in all.
    const PAST: TargetSpec = TargetSpec {
        width: 20_000,
        height: 20_000,
        transform: Transform::IDENTITY,
    };

    fn listed(page: usize, bytes: usize) -> Framed {
        Framed {
            page,
            payload: Payload::List {
                list: Arc::new(DisplayList::new(Size::new(612.0, 792.0))),
                target: PAST,
            },
            bytes,
            origin: (0.0, 0.0),
            declined: false,
        }
    }

    fn windowed(page: usize, corner: (u32, u32)) -> Framed {
        Framed {
            page,
            payload: Payload::Window {
                target: PAST,
                corner,
                raster: Raster {
                    width: 1000,
                    height: 1000,
                    format: RasterFormat::Rgba8,
                    data: vec![0; 4_000_000],
                },
            },
            bytes: 4_000_000,
            origin: (0.0, 0.0),
            declined: false,
        }
    }

    /// **The crossing is counted** (`doc/questions/A477`): a page whose device frame is quick but
    /// whose marks take past `WARN` to cross is predicted past it, and its window is asked for; a
    /// page whose crossing is quick keeps its marks.
    #[test]
    fn a_page_whose_crossing_is_past_the_warning_is_asked_for_as_its_window() {
        let viewport = (800, 600);
        let mut pricing = Pricing::default();
        assert!(
            pricing
                .frame(&[listed(0, 400_000_000)], Duration::from_secs(2), viewport)
                .is_empty(),
            "nothing is priced before the device has drawn"
        );
        pricing.device_drew(Duration::from_millis(100), 1_000);
        let [forecast] = pricing
            .frame(&[listed(0, 400_000_000)], Duration::from_secs(2), viewport)
            .try_into()
            .expect("one page priced");
        assert!(forecast.device > WARN, "{forecast:?}");
        assert!(!forecast.window_measured);
        assert_eq!((forecast.chosen, forecast.changed), (Route::Window, true));
        pricing.asked(0, Route::Window);
        // Its window crosses, new, in 50 ms: measured, and still the smaller.
        let [forecast] = pricing
            .frame(&[windowed(0, (0, 0))], Duration::from_millis(50), viewport)
            .try_into()
            .expect("the page is priced by the marks it last crossed with");
        assert!(forecast.window_measured);
        assert_eq!((forecast.chosen, forecast.changed), (Route::Window, false));
        // A quick crossing keeps the marks, and the change is to be sent.
        let [forecast] = pricing
            .frame(&[listed(0, 1_000)], Duration::from_millis(20), viewport)
            .try_into()
            .expect("one page priced");
        assert!(forecast.device < WARN, "{forecast:?}");
        assert_eq!((forecast.chosen, forecast.changed), (Route::Marks, true));
    }

    /// A frame of two pages past the bound measures nothing, because what it took cannot be
    /// divided between them; a window that crosses again unchanged was not drawn again, and
    /// measures nothing either.
    #[test]
    fn only_a_page_that_crossed_alone_and_new_measures_its_route() {
        let viewport = (800, 600);
        let mut pricing = Pricing::default();
        pricing.device_drew(Duration::from_millis(100), 1_000);
        assert!(
            pricing
                .frame(
                    &[listed(0, 1_000), listed(1, 1_000)],
                    Duration::from_secs(5),
                    viewport
                )
                .is_empty(),
            "two pages together measure no crossing"
        );
        pricing.frame(&[listed(0, 1_000)], Duration::from_millis(10), viewport);
        pricing.frame(&[windowed(0, (0, 0))], Duration::from_millis(30), viewport);
        let measured = pricing.window;
        pricing.frame(&[windowed(0, (0, 0))], Duration::from_millis(1), viewport);
        assert_eq!(pricing.window, measured, "the same window measured again");
        pricing.frame(&[windowed(0, (5, 0))], Duration::from_millis(1), viewport);
        assert_ne!(pricing.window, measured, "a new window is measured");
    }
}
