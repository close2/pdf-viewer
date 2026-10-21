//! A measured price applies only to the view and graphics coverage that produced it.

use pdf_render::TargetSpec;
use raster_gpu::Coverage;
use std::time::Duration;

/// Where a lane prediction's price came from. A seed is not a measurement of this view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstimateBasis {
    /// The last completed sample matches this view and, for the device, coverage lane.
    Measured,
    /// This bucket has no completed sample.
    Seed,
    /// The bucket's last sample drew another placement, output size or command population.
    OtherView,
    /// The device bucket's last sample used another coverage lane.
    OtherCoverage,
}

impl EstimateBasis {
    /// The provenance phrase printed beside the predicted duration.
    #[must_use]
    pub const fn said(self) -> &'static str {
        match self {
            Self::Measured => "last measured for this view",
            Self::Seed => "seed; no retained measurement for this kind of frame",
            Self::OtherView => "seed; the last measurement was for another view",
            Self::OtherCoverage => "seed; the last measurement used another coverage lane",
        }
    }
}

/// Page identity is the arrangement's key in `Costs`; these are its view-dependent inputs.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct View {
    targets: Vec<(TargetSpec, crate::stale::Depicts)>,
    extent: Option<(u32, u32)>,
}

impl View {
    pub(super) fn of(pages: &[crate::stale::Placed], extent: Option<(u32, u32)>) -> Self {
        Self {
            targets: pages
                .iter()
                .map(|page| (page.target, page.depicts))
                .collect(),
            extent,
        }
    }
}

/// One completed frame's measured price, independent of the once-per-arrangement trial state.
#[derive(Debug)]
pub(super) struct Sample {
    view: View,
    coverage: Option<Coverage>,
    commands: usize,
    each: f64,
}

impl Sample {
    pub(super) fn new(
        view: View,
        coverage: Option<Coverage>,
        commands: usize,
        took: Duration,
    ) -> Option<Self> {
        if commands == 0
            || view
                .targets
                .iter()
                .any(|(_, depicts)| *depicts == crate::stale::Depicts::Transition)
        {
            return None;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "a frame's bounded command count fits f64's exact range"
        )]
        let each = took.as_secs_f64() * 1e9 / commands as f64;
        Some(Self {
            view,
            coverage,
            commands,
            each,
        })
    }

    pub(super) fn price(
        sample: Option<&Self>,
        view: &View,
        coverage: Coverage,
        commands: usize,
        seed: f64,
    ) -> (f64, EstimateBasis) {
        let Some(sample) = sample else {
            return (seed, EstimateBasis::Seed);
        };
        if sample.coverage.is_some_and(|used| used != coverage) {
            return (seed, EstimateBasis::OtherCoverage);
        }
        if sample.view != *view || sample.commands != commands {
            return (seed, EstimateBasis::OtherView);
        }
        (sample.each, EstimateBasis::Measured)
    }
}
