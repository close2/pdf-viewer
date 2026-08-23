//! The corpus column `doc/QUORRA_CLIP_LANE_AND_UPLOAD.md` section 6's third question asks for: how many
//! marks the sampled coverage lane would have to give back to the processor, and on how many
//! pages.
//!
//! quorra's ADR 0076 records a non-conformance rather than a tolerance. Its sampled lane places
//! coverage samples on one lattice of period `p = 1/√coverage_samples` across the whole device —
//! a quarter of a device pixel at the default sixteen — so an axis-aligned mark's ink is a count
//! of lattice rows times `p`, and a pixel the mark's boundary crosses receives **nothing** when no
//! lattice row falls inside it. ISO 32000-2 §10.7.4 forbids that in its own words:
//!
//! > A shape shall be scan-converted by painting any pixel whose half-open square region
//! > intersects the shape, no matter how small the intersection is.
//!
//! They offered to divert marks whose width is not a multiple of the pitch to the processor and
//! asked for our corpus column before doing it, because the lane is ours to pay for. This is that
//! column, and it is a **population** count rather than a simulation of their lattice: what
//! decides the question is how many marks would move, and the rate at which a moved mark was
//! actually being drawn wrong is `p` by their own arithmetic and needs no instrument here.
//!
//! # What is counted, and why the scale is an argument
//!
//! `viewer-ui` takes [`quorra_gpu::Coverage::Gpu`] at ten times magnification and never below it
//! (`GPU_COVERAGE_MAGNIFICATION`), so **the shipped population is a zoomed page** and a census at
//! page scale would count a lane no frame this program draws would have used. The default scale
//! here is therefore ten, and it is the argument as much as the setting.
//!
//! A mark's *width* is the narrow side of its device bounding box, which is what quorra's rule
//! names: for an axis-aligned rule — stroked or filled — that is the thickness, and for anything
//! else the census says so by putting it in the widest band, where the rule's relative error `p/w`
//! is negligible. Marks narrower than `p` are not counted at all: quorra's ADR 0070 already keeps
//! those on the processor, so they are not this lane's population.
//!
//! ```sh
//! cargo run --release -p render-quorra --example sampled_lane_column -- \
//!     doc/pdf.js/test/pdfs/*.pdf
//! cargo run --release -p render-quorra --example sampled_lane_column -- --scale 1 …
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]
#![expect(
    clippy::cast_precision_loss,
    reason = "a count printed as a percentage of another count; both are far below 2^24"
)]

use pdf_render::{Command, Rect, TargetSpec, Transform};
use pdf_syntax::Document;

/// The magnification `viewer-ui` takes the sampled lane at, and the census's default.
///
/// Stated here rather than imported because `viewer-ui` depends on this crate and not the other
/// way about; its `GPU_COVERAGE_MAGNIFICATION` carries the derivation.
const SHIPPED_MAGNIFICATION: f32 = 10.0;

/// The bands a mark's width is reported in, in device pixels, each the band's upper bound.
///
/// They are the relative error `p/w` rather than round numbers: at the default sixteen samples a
/// mark one pixel wide is drawn up to a quarter wrong, one four pixels wide up to a sixteenth, and
/// past sixteen the lattice is finer than a level of an eight-bit raster.
const BANDS: [f32; 4] = [1.0, 4.0, 16.0, f32::INFINITY];

/// How far above or below a multiple of the pitch a width may be and still count as one.
///
/// One part in 4096 of a device pixel: far below what any rasteriser resolves, and far above the
/// rounding of a page transform composed through three matrices.
const ON_THE_LATTICE: f32 = 1.0 / 4096.0;

/// One page's marks, by width band.
#[derive(Default, Clone, Copy)]
struct Tally {
    /// Marks of any kind, including those the lane never sees.
    marks: usize,
    /// Marks at or above the pitch — the sampled lane's own population.
    on_the_lane: usize,
    /// Of them, per band, those whose width is not a multiple of the pitch.
    off_lattice: [usize; BANDS.len()],
    /// Of them, per band, those whose width is a multiple of the pitch.
    on_lattice: [usize; BANDS.len()],
}

impl Tally {
    /// Folds a page's tally into a run's.
    fn add(&mut self, other: &Self) {
        self.marks = self.marks.saturating_add(other.marks);
        self.on_the_lane = self.on_the_lane.saturating_add(other.on_the_lane);
        for (into, from) in self.off_lattice.iter_mut().zip(other.off_lattice) {
            *into = into.saturating_add(from);
        }
        for (into, from) in self.on_lattice.iter_mut().zip(other.on_lattice) {
            *into = into.saturating_add(from);
        }
    }

    /// Marks quorra's proposed rule would divert to the processor.
    fn diverted(&self) -> usize {
        self.off_lattice.iter().fold(0, |a, b| a.saturating_add(*b))
    }
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut scale = SHIPPED_MAGNIFICATION;
    if let Some(at) = args.iter().position(|a| a == "--scale") {
        let Some(value) = args.get(at.saturating_add(1)).and_then(|v| v.parse().ok()) else {
            println!("--scale wants a number");
            return;
        };
        scale = value;
        args.drain(at..=at.saturating_add(1));
    }
    let samples = render_quorra::options().coverage_samples;
    #[expect(
        clippy::cast_precision_loss,
        reason = "a sample count, which quorra bounds far below 2^24"
    )]
    let pitch = 1.0 / (samples as f32).sqrt();
    println!(
        "# scale {scale}, {samples} coverage samples, pitch {pitch:.4} device pixels; \
         width bands {BANDS:?}"
    );

    let mut run = Tally::default();
    let mut pages = 0_usize;
    let mut pages_with_a_diversion = 0_usize;
    for path in args {
        let name = std::path::Path::new(&path)
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let Some(page) = pdf_model::Pages::new(&document).get(0) else {
            continue;
        };
        let list = pdf_model::content::interpret(&document, &page).display_list;
        // The census is about widths in device pixels, so it needs the transform a frame would
        // have been drawn under; a page too large for the budget at this scale is skipped rather
        // than measured at another one.
        let Ok(target) = TargetSpec::for_page(&list, scale, 1 << 31) else {
            continue;
        };
        let mut tally = Tally::default();
        walk(list.commands(), target.transform, pitch, &mut tally);
        pages = pages.saturating_add(1);
        if tally.diverted() > 0 {
            pages_with_a_diversion = pages_with_a_diversion.saturating_add(1);
        }
        println!(
            "{name}\t{} mark(s)\t{} on the lane\t{} diverted ({:.2}% of marks)\toff-lattice by \
             band {:?}\ton-lattice by band {:?}",
            tally.marks,
            tally.on_the_lane,
            tally.diverted(),
            percent(tally.diverted(), tally.marks),
            tally.off_lattice,
            tally.on_lattice,
        );
        run.add(&tally);
    }
    println!(
        "# {pages} page(s), {} mark(s), {} on the sampled lane, {} would divert ({:.2}% of all \
         marks, {:.2}% of the lane's), on {pages_with_a_diversion} page(s)",
        run.marks,
        run.on_the_lane,
        run.diverted(),
        percent(run.diverted(), run.marks),
        percent(run.diverted(), run.on_the_lane),
    );
    println!("# off-lattice by band {:?}", run.off_lattice);
    println!("# on-lattice  by band {:?}", run.on_lattice);
}

/// `part` as a percentage of `whole`, and zero where there is no whole.
fn percent(part: usize, whole: usize) -> f32 {
    if whole == 0 {
        0.0
    } else {
        (part as f32) * 100.0 / (whole as f32)
    }
}

/// Every mark in one level of the display list, and every level below it.
fn walk(commands: &[Command], to_device: Transform, pitch: f32, tally: &mut Tally) {
    for command in commands {
        match command {
            Command::Fill {
                path, transform, ..
            } => {
                tally.marks = tally.marks.saturating_add(1);
                classify(path.bounds(transform.then(to_device)), pitch, tally);
            }
            Command::Stroke {
                path,
                transform,
                stroke,
                ..
            } => {
                tally.marks = tally.marks.saturating_add(1);
                // A stroke's reach is its path grown by half the line width, and the width is
                // stated in the path's own space — which is what `Path::hull` is public for.
                let at = transform.then(to_device);
                let half = stroke.device_width(at) / 2.0;
                classify(
                    path.hull().map(|hull| {
                        Rect::from_corners(
                            pdf_render::Point::new(hull.min.x - half, hull.min.y - half),
                            pdf_render::Point::new(hull.max.x + half, hull.max.y + half),
                        )
                        .mapped(at)
                    }),
                    pitch,
                    tally,
                );
            }
            // An image is drawn by the sampled lane's sibling path and has no width in quorra's
            // sense; it is counted as a mark so that the percentages are of the whole page.
            Command::Image { .. } => tally.marks = tally.marks.saturating_add(1),
            Command::Group { commands, .. } => walk(commands, to_device, pitch, tally),
            Command::Shaped { object, .. } => {
                walk(std::slice::from_ref(object), to_device, pitch, tally);
            }
            _ => {}
        }
    }
}

/// One mark's device bounding box, put in its band.
fn classify(bounds: Option<Rect>, pitch: f32, tally: &mut Tally) {
    let Some(bounds) = bounds else {
        return;
    };
    let width = (bounds.max.x - bounds.min.x).min(bounds.max.y - bounds.min.y);
    if !width.is_finite() || width < pitch {
        // quorra's ADR 0070 keeps anything narrower than the pitch on the processor already, so
        // it is not this lane's population and counting it would overstate the diversion.
        return;
    }
    tally.on_the_lane = tally.on_the_lane.saturating_add(1);
    let band = BANDS
        .iter()
        .position(|bound| width < *bound)
        .unwrap_or(BANDS.len().saturating_sub(1));
    let ratio = width / pitch;
    let on_lattice = (ratio - ratio.round()).abs() * pitch <= ON_THE_LATTICE;
    let counter = if on_lattice {
        &mut tally.on_lattice
    } else {
        &mut tally.off_lattice
    };
    if let Some(slot) = counter.get_mut(band) {
        *slot = slot.saturating_add(1);
    }
}
