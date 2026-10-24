//! The first page of every tracked corpus document drawn ahead in strips on the pool while it is
//! read, as a window's first frame is (`viewer_host::Ahead`, ADR 1896), against the page drawn whole
//! at the same cuts.
//!
//! `render_cpu::Underway::in_strips` defines its page as the page drawn whole at its cuts, and a
//! claim about strips made from fixtures is a claim about the fixtures (trap 66): this asks it of
//! the corpus, at four strips stated so that the answer does not depend on the machine (trap 134),
//! in parts of a few commands so that nearly every page is handed over in many.

#![expect(
    clippy::panic,
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a corpus gate: it stops loudly where the sandbox is missing and prints its tally"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use pdf_model::Pages;
use pdf_model::content::{displayed_size, interpret};
use pdf_model::progress::{Handed, Parts};
use pdf_render::{DisplayList, Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::{CpuRasterizer, Underway};

/// The pixel budget of one target, as `pdf-model`'s `drawn_while_read` states it.
const PIXEL_BUDGET: u64 = 64 << 20;

/// Fewest commands a part holds: few enough that a page of a handful is handed over in several.
const SMALL_PARTS: usize = 8;

/// The strips asked for, stated rather than asked of the machine.
const STRIPS: u32 = 4;

/// Every tracked corpus document, sorted; `None` when the submodule is not checked out.
fn corpus() -> Option<Vec<PathBuf>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&root)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
        .collect();
    files.sort();
    (!files.is_empty()).then_some(files)
}

/// What one page drawn ahead came to.
enum Outcome {
    /// Finished from its parts in this many strips, and whether that is the page drawn whole at
    /// its cuts.
    Finished { strips: usize, same: bool },
    /// Drawn whole instead, for this reason.
    Whole(&'static str),
}

/// Page one of `document` read on this thread with a hand-over installed while a second thread
/// draws each batch of parts it finds waiting into strips, then finished from the list.
fn drawn_ahead(document: &Document) -> Option<Outcome> {
    let page = Pages::new(document).get(0)?;
    let target =
        TargetSpec::for_page(&DisplayList::new(displayed_size(&page)), 1.0, PIXEL_BUDGET).ok()?;
    let rasterizer = CpuRasterizer::new().with_strips(STRIPS);
    let parts = Parts::in_parts_of(SMALL_PARTS);
    let (interpretation, (underway, ended)) = std::thread::scope(|scope| {
        let drawer = {
            let (parts, rasterizer) = (parts.clone(), rasterizer.clone());
            scope.spawn(move || {
                let mut underway = Underway::in_strips(&rasterizer, target);
                loop {
                    let mut waiting = Vec::new();
                    let mut wait = Duration::from_mins(1);
                    let mut ended = None;
                    while let Some(handed) = parts.take(wait) {
                        match handed {
                            Handed::Part(part) => waiting.push(part),
                            Handed::Ended(result) => {
                                ended = Some(result);
                                break;
                            }
                        }
                        wait = Duration::ZERO;
                    }
                    if let Some(underway) = underway.as_mut()
                        && !waiting.is_empty()
                    {
                        underway.take_all(waiting);
                    }
                    if let Some(ended) = ended {
                        return (underway, ended);
                    }
                }
            })
        };
        let interpretation = parts.during(|| interpret(document, &page));
        (
            interpretation,
            drawer.join().expect("the drawer does not panic"),
        )
    });
    let list = interpretation.display_list;
    let underway = underway?;
    if let Err(reason) = ended {
        return Some(Outcome::Whole(reason));
    }
    let cuts = underway.cuts().to_vec();
    Some(match underway.finish(&list) {
        Ok(render_cpu::Finished { raster, .. }) => {
            let whole = rasterizer.with_cuts(&cuts).rasterize(&list, target).ok()?;
            Outcome::Finished {
                strips: cuts.len().saturating_sub(1),
                same: raster.data == whole.data,
            }
        }
        Err(reason) => Outcome::Whole(reason),
    })
}

/// Refuses to measure without the sandboxed decoder — trap 10, ADR 0557 — for the reason every
/// corpus gate does: a page whose JBIG2, JPX or CCITT image the worker did not decode holds no
/// command for it on either side of this comparison, so a pass taken without the worker is a pass
/// of the build and not of the tree.
fn require_the_sandbox() {
    if let Err(error) = pdf_model::image::sandboxed_decoder() {
        panic!(
            "the sandboxed image decoder is not available, so the comparison below would be of the \
             build rather than of the tree: {error}"
        );
    }
}

/// Every tracked first page drawn ahead in strips is the page drawn whole at its cuts, or says
/// why it was drawn whole instead.
#[test]
#[ignore = "the whole tracked corpus interpreted once and drawn twice; run explicitly, under the \
            gates profile"]
fn every_tracked_first_page_drawn_ahead_in_strips_is_the_page_drawn_whole_at_its_cuts() {
    require_the_sandbox();
    if let Err(error) = pdf_model::image::sandboxed_decoder() {
        panic!(
            "the sandboxed image decoder is not available, so the comparison below would be of \
             the build rather than of the tree: {error}"
        );
    }
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let mut in_strips = 0_usize;
    let mut one_surface = 0_usize;
    let mut whole_instead: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut different = Vec::new();
    for path in &files {
        let Some(document) = std::fs::read(path)
            .ok()
            .and_then(|bytes| Document::open(bytes).ok())
        else {
            continue;
        };
        match drawn_ahead(&document) {
            Some(Outcome::Finished { strips, same: true }) if strips > 1 => in_strips += 1,
            Some(Outcome::Finished { same: true, .. }) => one_surface += 1,
            Some(Outcome::Finished { same: false, .. }) => {
                different.push(path.display().to_string());
            }
            Some(Outcome::Whole(reason)) => *whole_instead.entry(reason).or_default() += 1,
            None => {}
        }
    }
    println!(
        "{} documents: {in_strips} first pages finished in {STRIPS} strips and {one_surface} as \
         one surface, each byte for byte the page drawn whole at its cuts; drawn whole instead: \
         {whole_instead:?}",
        files.len()
    );
    assert!(
        different.is_empty(),
        "pages drawn ahead in strips that are not the page drawn whole at their cuts: {different:?}"
    );
    assert!(
        in_strips > files.len() / 2,
        "most first pages are tall enough for {STRIPS} strips, and {in_strips} were finished in them"
    );
}
