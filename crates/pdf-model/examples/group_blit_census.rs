//! What each first page's transparency groups would cost to composite —
//! [`pdf_render::group_blit_demand`] over a corpus, which is what sized
//! [`pdf_render::MAX_GROUP_BLIT_PIXELS`].
//!
//! It is deliberately the *same function* the bound is checked with rather than a second
//! reading of the same idea: a census that measures something slightly different from what
//! the code refuses on cannot size it.
//!
//! **It interprets and never rasterises**, which is the whole reason it can be run over a
//! population containing `poppler-978-0.pdf` — the page whose 73 047 page-spanning groups
//! this bound exists for, and which takes some 640 s to draw. Interpretation of it is 2.5 s.
//!
//! ```sh
//! cargo run --release -p pdf-model --example group_blit_census -- doc/pdf.js/test/pdfs/*.pdf
//! find corpus-cache/safedocs -name '*.pdf' -print0 | xargs -0 -n 40 -P 8 <the binary>
//! ```
//!
//! One line per document whose demand reaches [`REPORT_AT`], then a summary: how many first
//! pages state a group at all, the count in each band, how many the bound would refuse, and
//! the heaviest by name. Each line carries the **ratio** to the target's own area beside the
//! pixel count, because that is the scale-free half of the picture even though it is not
//! what the bound holds — `pdf_render::group_cost` says why.
//!
//! Run over a population in several processes, the summaries add: each is a count over the
//! documents that process saw, and the heaviest list is the top of its own batch.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use pdf_model::{Pages, interpret};
use pdf_render::{MAX_GROUP_BLIT_PIXELS, TargetSpec, group_blit_demand};
use pdf_syntax::Document;

/// Demand, in pixels, at or above which a document gets a line of its own.
///
/// A quarter of a gibipixel: low enough that the whole tail of every population this tree
/// has is legible, and high enough that a run over 65 944 documents is not a listing of all
/// of them.
const REPORT_AT: u64 = 1 << 28;

/// The bands the summary counts into, as upper bounds in blitted pixels.
const BANDS: [u64; 8] = [
    0,
    1 << 20,
    1 << 24,
    1 << 28,
    1 << 30,
    1 << 32,
    1 << 34,
    MAX_GROUP_BLIT_PIXELS,
];

/// The scale the census measures at.
///
/// 1:1 on a page-sized target, which is the one scale that is a property of the *document*
/// rather than of somebody's window — and it is the condition every figure this prints has
/// to be quoted with, because the bound is absolute and a window is not a page.
const SCALE: f32 = 1.0;

/// The largest target the census will build a spec for.
const MAX_PIXELS: u64 = 1 << 30;

fn main() {
    let mut documents = 0_usize;
    let mut with_groups = 0_usize;
    let mut counts = [0_usize; BANDS.len() + 1];
    let mut over = 0_usize;
    let mut worst: Vec<(u64, f64, String)> = Vec::new();

    for path in std::env::args().skip(1) {
        let name = std::path::Path::new(&path)
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let Some(page) = Pages::new(&document).get(0) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let list = interpret(&document, &page).display_list;
        let Ok(target) = TargetSpec::for_page(&list, SCALE, MAX_PIXELS) else {
            continue;
        };
        let area = u64::from(target.width).saturating_mul(u64::from(target.height));
        if area == 0 {
            continue;
        }
        let demanded = group_blit_demand(&list, target);
        if demanded == 0 {
            counts[0] = counts[0].saturating_add(1);
            continue;
        }
        with_groups = with_groups.saturating_add(1);
        #[expect(
            clippy::cast_precision_loss,
            reason = "a ratio printed to one decimal; neither operand reaches 2^53"
        )]
        let ratio = demanded as f64 / area as f64;
        let band = BANDS
            .iter()
            .position(|bound| demanded <= *bound)
            .unwrap_or(BANDS.len());
        counts[band] = counts[band].saturating_add(1);
        if demanded > MAX_GROUP_BLIT_PIXELS {
            over = over.saturating_add(1);
        }
        if demanded >= REPORT_AT {
            println!(
                "{name}\t{demanded} pixels\t{ratio:.1} x the target\t{}x{}",
                target.width, target.height
            );
        }
        worst.push((demanded, ratio, name));
    }

    worst.sort_by_key(|row| std::cmp::Reverse(row.0));
    worst.truncate(20);

    println!("---");
    println!("{documents} document(s) with a first page, {with_groups} of them stating a group");
    for (index, count) in counts.iter().enumerate() {
        match (index, BANDS.get(index)) {
            (0, _) => println!("  no group at all:\t{count}"),
            (_, Some(high)) => println!("  d <= {high:>14}:\t{count}"),
            (_, None) => println!("  over the bound:\t{count}"),
        }
    }
    println!("over MAX_GROUP_BLIT_PIXELS ({MAX_GROUP_BLIT_PIXELS}): {over}");
    println!("the heaviest, in order:");
    for (demanded, ratio, name) in &worst {
        println!("  {demanded:>16}\t{ratio:>10.1}\t{name}");
    }
}
