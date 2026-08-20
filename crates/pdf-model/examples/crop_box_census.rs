//! How many documents draw where ISO 32000-2 §14.11.2.1 says nothing shall be shown.
//!
//! The clause is a `shall` and it is about the *page*:
//!
//! > The crop box defines the region to which the contents of the page shall be clipped
//! > (cropped) when displayed or printed. Unlike the other boxes, the crop box has no defined
//! > meaning in terms of physical page geometry or intended use; it merely imposes clipping on
//! > the page contents.
//!
//! `pdf_model::interpret` deliberately keeps the marks a content stream made outside that box —
//! nothing in the display list is dropped — and until the six-hundred-and-twelfth session
//! nothing put the clip back. A **page-sized** raster hid it: the target is the crop box's own
//! extent, so the raster's own edge did the cutting. A **window** is larger than its page, so
//! those marks drew over the ground beside the page and over the neighbouring page of a column.
//!
//! # Two questions, and the second is the one that costs anybody anything
//!
//! `doc/traps/instruments-and-reports.md` trap 11: derive the condition from the clause and
//! print what it matched. The condition the clause states is **content outside the crop box**,
//! which is *not* the same as "a crop box smaller than the media box" — a document may state
//! them equal and still mark beyond both, and one may crop hard and draw nothing out there. So
//! this counts three nested populations:
//!
//! 1. **`/CropBox` smaller than `/MediaBox`** — structural, free, and on its own decides
//!    nothing.
//! 2. **A command whose bounds leave the crop box** — `Command::device_bounds` ignoring the
//!    command's own clip, so this is an over-approximation and a *candidate* set.
//! 3. **Ink actually outside it** — the candidates rasterised on a target three page-widths
//!    across, counting pixels the page marked beyond its own boundary. This is the population
//!    the clip changes what a reader sees for.
//!
//! The third is nested inside the second by construction: a command whose bounds are inside the
//! box cannot mark outside it. It is *not* nested inside the first, and reporting it as though
//! it were is the mistake this example exists to avoid.
//!
//! # What the raster can and cannot see
//!
//! The confirming raster covers the crop box grown by its own width and height on each side —
//! `NEIGHBOURHOOD` — at whatever scale keeps it under [`MAX_PIXELS`]. A mark further away than
//! that is counted separately and by name rather than dropped, because a window scrolled or
//! magnified can be anywhere: what the two counts separate is "beside the page" from "somewhere
//! else entirely".
//!
//! The page geometry comes from [`pdf_model::Page`], which has already applied §14.11.2.1's own
//! intersection rule — "[i]f the bounds of the crop, trim, bleed or art box extends outside of
//! the bounds of the media box, a processor shall treat the box as its intersection with the
//! media box" — and §12.2's `/ViewClip`. That is deliberate and is not
//! `doc/HANDOVER.md` trap 8's mistake: the code under test here is the *clip*, and the boxes it
//! clips to are an input to it rather than the thing being measured.
//!
//! ```sh
//! cargo run --release -p pdf-model --example crop_box_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use rayon::prelude::*;

use pdf_model::{Pages, interpret};
use pdf_render::display_list::Command;
use pdf_render::{DisplayList, Medium, Point, Rasterizer as _, Rect, TargetSpec, Transform};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixels the confirming raster may cost, so that a malformed extent cannot exhaust a worker.
const MAX_PIXELS: u64 = 4 << 20;

/// How far beyond the crop box the confirming raster looks, in multiples of the box's own
/// extent on each side.
///
/// One page-width is the neighbourhood a column shows: the next page of `OneColumn` starts
/// there, and so does the ground a `SinglePage` view shows beside a page that does not fill its
/// window. Beyond it a mark is somewhere else entirely, and that is counted as its own thing.
const NEIGHBOURHOOD: f32 = 1.0;

/// Slack, in page units, before a box is called smaller than another or a mark is called
/// outside one.
///
/// A rectangle stated as `[0 0 612.0 792.0]` in one place and `[0 0 612 792]` in another is one
/// rectangle, and a bound that lands on the boundary is not outside it. A thousandth of a unit
/// is under a thousandth of a pixel at 72 dpi.
const SLACK: f32 = 1e-3;

/// What one document's first page says about the question.
struct Finding {
    /// The file, as it was named on the command line.
    path: String,
    /// The crop box is smaller than the media box on at least one side.
    cropped: bool,
    /// Commands whose bounds leave the crop box, ignoring their own clips.
    commands_outside: usize,
    /// Pixels of ink the raster found outside the crop box.
    ink_outside: u64,
    /// A command's bounds leave the neighbourhood the raster covers.
    beyond_neighbourhood: bool,
    /// The raster could not be built or refused, so question 3 was not answered here.
    unrasterised: Option<String>,
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let findings: Vec<Finding> = paths.par_iter().filter_map(|path| examine(path)).collect();

    let opened = findings.len();
    let cropped = findings.iter().filter(|f| f.cropped).count();
    let candidates = findings.iter().filter(|f| f.commands_outside > 0).count();
    let inked = findings.iter().filter(|f| f.ink_outside > 0).count();
    let both = findings
        .iter()
        .filter(|f| f.cropped && f.ink_outside > 0)
        .count();
    let cropped_only = findings
        .iter()
        .filter(|f| f.cropped && f.commands_outside == 0)
        .count();
    let uncropped_ink = findings
        .iter()
        .filter(|f| !f.cropped && f.ink_outside > 0)
        .count();
    let beyond = findings.iter().filter(|f| f.beyond_neighbourhood).count();
    let unrasterised = findings
        .iter()
        .filter(|f| f.unrasterised.is_some() && f.commands_outside > 0)
        .count();

    println!(
        "{} path(s) given, {opened} first page(s) interpreted",
        paths.len()
    );
    println!("  {cropped} document(s) state a /CropBox smaller than their /MediaBox");
    println!(
        "  {cropped_only} of those place every command inside it — the clip costs them nothing"
    );
    println!("  {candidates} document(s) have a command whose bounds leave the crop box");
    println!("  {inked} document(s) actually mark outside it — the population the clip changes");
    println!("    {both} of those also crop smaller than the medium");
    println!("    {uncropped_ink} of those state no smaller crop box and mark beyond it anyway");
    println!("  {beyond} document(s) have such a command beyond one page-width of the box");
    println!("  {unrasterised} candidate(s) could not be rasterised, so are unconfirmed");
    for finding in findings.iter().filter(|f| f.ink_outside > 0) {
        println!(
            "  ink outside: {} — {} px, {} command(s){}{}",
            finding.path,
            finding.ink_outside,
            finding.commands_outside,
            if finding.cropped {
                ", crop < media"
            } else {
                ""
            },
            if finding.beyond_neighbourhood {
                ", and one beyond the neighbourhood"
            } else {
                ""
            }
        );
    }
    for finding in findings
        .iter()
        .filter(|f| f.commands_outside > 0 && f.ink_outside == 0)
    {
        if let Some(reason) = &finding.unrasterised {
            println!("  unconfirmed: {} — {reason}", finding.path);
        }
    }
}

/// Interprets one document's first page and answers all three questions about it.
fn examine(path: &str) -> Option<Finding> {
    let bytes = std::fs::read(path).ok()?;
    let document = Document::open(bytes).ok()?;
    let page = Pages::new(&document).get(0)?;
    let cropped = page.crop_box[0] > page.media_box[0] + SLACK
        || page.crop_box[1] > page.media_box[1] + SLACK
        || page.crop_box[2] < page.media_box[2] - SLACK
        || page.crop_box[3] < page.media_box[3] - SLACK;
    let list = interpret(&document, &page).display_list;

    // The display list's own space, where the box the page is displayed in sits at the origin
    // — so the page's boundary is `page_bounds` and a command's bounds are comparable to it
    // without a transform at all.
    let bounds = list.page_bounds();
    let mut outside = Outside::default();
    walk(list.commands(), bounds, &mut outside);

    let mut finding = Finding {
        path: path.to_owned(),
        cropped,
        commands_outside: outside.commands,
        ink_outside: 0,
        beyond_neighbourhood: false,
        unrasterised: None,
    };
    if outside.commands == 0 {
        return Some(finding);
    }
    let neighbourhood = Rect::from_corners(
        Point::new(
            bounds.min.x - bounds.width() * NEIGHBOURHOOD,
            bounds.min.y - bounds.height() * NEIGHBOURHOOD,
        ),
        Point::new(
            bounds.max.x + bounds.width() * NEIGHBOURHOOD,
            bounds.max.y + bounds.height() * NEIGHBOURHOOD,
        ),
    );
    finding.beyond_neighbourhood = outside
        .reach
        .is_some_and(|reach| !neighbourhood.contains(reach));
    match confirm(&list, neighbourhood) {
        Ok(pixels) => finding.ink_outside = pixels,
        Err(reason) => finding.unrasterised = Some(reason),
    }
    Some(finding)
}

/// What a walk of the commands found outside the page's boundary.
#[derive(Default)]
struct Outside {
    /// Commands whose bounds are not contained in it.
    commands: usize,
    /// The union of those bounds, or `None` where none was found.
    reach: Option<Rect>,
}

/// Counts the commands whose bounds leave `bounds`, descending into groups.
///
/// A group answers `None` to `device_bounds` and its elements carry the extent, which is why
/// this recurses rather than asking the group.
fn walk(commands: &[Command], bounds: Rect, outside: &mut Outside) {
    for command in commands {
        if let Command::Group { commands, .. } = command {
            walk(commands, bounds, outside);
            continue;
        }
        let Some(reach) = command.device_bounds(Transform::IDENTITY) else {
            continue;
        };
        if bounds.grown(SLACK).contains(reach) {
            continue;
        }
        outside.commands = outside.commands.saturating_add(1);
        outside.reach = Some(outside.reach.map_or(reach, |seen| seen.union(reach)));
    }
}

/// Rasterises the page over `region` and counts the ink pixels wholly outside the page.
///
/// [`Medium::NONE`] because what is wanted is the page's own alpha: a medium composited under
/// it would make every pixel opaque and the count meaningless.
fn confirm(list: &DisplayList, region: Rect) -> Result<u64, String> {
    let (width, height) = (region.width(), region.height());
    if !(width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0) {
        return Err("the neighbourhood has no extent".to_owned());
    }
    // Whatever scale keeps the neighbourhood under the pixel budget, and never above 1:
    // a mark is being *detected* here rather than drawn, and anti-aliasing keeps a hairline
    // visible as a low alpha rather than dropping it.
    #[expect(
        clippy::cast_precision_loss,
        reason = "a constant pixel budget of four million, exact in f64"
    )]
    let budget = MAX_PIXELS as f64;
    let scale = (budget / (f64::from(width) * f64::from(height)))
        .sqrt()
        .min(1.0);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a ratio of two finite positive quantities, clamped to at most 1.0"
    )]
    let scale = scale as f32;
    let target = TargetSpec {
        width: extent(width * scale)?,
        height: extent(height * scale)?,
        // The same shape `TargetSpec::for_page` builds — scale, flip y — with the region's own
        // top-left corner at the origin instead of the page's.
        transform: Transform::scale(scale, -scale).then(Transform::translate(
            -region.min.x * scale,
            region.max.y * scale,
        )),
    };
    let area = pdf_render::page_area(list, target);
    let raster = CpuRasterizer::new()
        .with_medium(Medium::NONE)
        .rasterize(list, target)
        .map_err(|problem| problem.to_string())?;
    let mut ink = 0_u64;
    let stride = (raster.width as usize).saturating_mul(4).max(4);
    for (row, pixels) in raster.data.chunks_exact(stride).enumerate() {
        // A pixel counts only where the *whole* of it lies beyond the boundary, so that the
        // anti-aliased edge of a mark that stops at the crop box is not read as a mark past it.
        let outside_row = !overlaps(row, area.min.y, area.max.y);
        for (column, pixel) in pixels.chunks_exact(4).enumerate() {
            if pixel[3] == 0 {
                continue;
            }
            if outside_row || !overlaps(column, area.min.x, area.max.x) {
                ink = ink.saturating_add(1);
            }
        }
    }
    Ok(ink)
}

/// Whether the unit-wide pixel at `index` overlaps `lo..hi` at all.
fn overlaps(index: usize, lo: f32, hi: f32) -> bool {
    #[expect(
        clippy::cast_precision_loss,
        reason = "an index into a raster bounded by MAX_PIXELS, far inside f32's exact integers"
    )]
    let edge = index as f32;
    edge + 1.0 > lo && edge < hi
}

/// One axis of the target, in whole pixels, or a reason it cannot be one.
fn extent(exact: f32) -> Result<u32, String> {
    if !exact.is_finite() || exact < 1.0 {
        return Err(format!("an axis of {exact} pixels"));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "finite and at least 1.0, and the pixel budget bounds it from above"
    )]
    let rounded = exact.ceil() as u32;
    Ok(rounded)
}
