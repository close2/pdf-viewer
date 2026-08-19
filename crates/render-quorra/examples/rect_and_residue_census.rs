//! Two facts about this corpus in one walk: how many of its fills are axis-aligned
//! rectangles, and what its clip chains cost the device.
//!
//! Both were asked for by the quorra team and are recorded as ours in
//! `doc/QUORRA_FEEDBACK.md` sections 25.3 and 27.4, and both are questions about a *corpus*
//! rather than about either implementation:
//!
//! - **The rectangular fills.** quorra's ADR 0047 gives a rectangle its own lane, and section 19
//!   of that document records that not one corpus page emits a `Command::Rect` from this side —
//!   this tree states every fill as a path. So what is left to know is how much of the corpus
//!   is in that shape at all. The predicate is [`pdf_render::crop::whole_rectangle`], the one
//!   the shipping crop path uses, and **not** a copy of it: a census carrying its own
//!   rectangle test would be measuring a second implementation. A rectangle in the path's own
//!   space reaches the device as one only where the transform preserves the axes, so both are
//!   asked and both are printed — the second is the share a device could act on.
//!
//! - **The residue pair.** `(clip_residue_regions, clip_residue_tiles)`: a chain whose links
//!   are not all rectangles becomes **one** coverage region cut into a window per mark, unless
//!   the region would cost more than the tiles it would replace, in which case every mark pays
//!   its own rasterisation (quorra's ADR 0049). Their `artwork` archetype gains 1.2× from a
//!   divided encode where their drawing gains 6.6×, and residue-clipped marks are what
//!   separates the two — so this is the number that says how much of this corpus is which.
//!
//! ```sh
//! cargo run --release -p render-quorra --example rect_and_residue_census
//! cargo run --release -p render-quorra --example rect_and_residue_census -- <file.pdf>…
//! ```
//!
//! With no argument it walks the pdf.js corpus's first pages at the scale
//! `render-quorra/tests/corpus.rs` renders them at, which is the population every other
//! statement in this project about "this corpus" is made over.
//!
//! **What it matched is printed before any share is taken** (trap 11): the fills it walked,
//! the pages it drew, and the pages it could not. A share whose denominator is not printed is
//! not a measurement.
//!
//! **It is deterministic and the run says so.** One device draws every page, so the caches and
//! the atlas carry across documents exactly as they do in the gate; what the residue rule reads
//! is the scene's own uses and the *frame's* budget, neither of which depends on what was drawn
//! before. Session 581 found a census whose answer moved between runs because a `static` table
//! was a process budget, and the rule that came out of it is that an instrument which cannot
//! answer the same thing twice establishes nothing — so this one is run twice and the two
//! outputs are compared.
#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    reason = "a measurement example: its output is the point, an absent adapter should stop it \
              loudly rather than report a corpus with no clips in it, a share of a count is a \
              float, and one walk answering both questions is clearer than two"
)]

use std::path::{Path, PathBuf};

use pdf_render::{Command, Rasterizer, TargetSpec, Transform};
use pdf_syntax::Document;
use render_quorra::QuorraRasterizer;

/// The scale `tests/corpus.rs` renders the corpus at, so that this walk's population is that
/// gate's population.
const SCALE: f32 = 1.0;

/// The pixel budget `tests/corpus.rs` gives a page.
const PIXEL_BUDGET: u64 = 1 << 30;

/// What the fills of one corpus said.
#[derive(Default)]
struct Fills {
    /// Every `Command::Fill` reached, groups walked into.
    total: u64,
    /// Of those, the ones whose path is one axis-aligned rectangle and nothing else.
    rectangles: u64,
    /// Of the rectangles, the ones the device would also see as one — the transform in force
    /// maps the axes onto the axes.
    on_axis: u64,
}

impl Fills {
    /// Walks a command tree, carrying the transform each command is placed by.
    ///
    /// A group's children are commands too, and a page whose top level is one group would
    /// otherwise read as a page with one fill on it.
    fn walk(&mut self, commands: &[Command], to_device: Transform) {
        for command in commands {
            match command {
                Command::Fill {
                    path, transform, ..
                } => {
                    self.total = self.total.saturating_add(1);
                    if pdf_render::crop::whole_rectangle(path).is_some() {
                        self.rectangles = self.rectangles.saturating_add(1);
                        if transform.then(to_device).preserves_axes() {
                            self.on_axis = self.on_axis.saturating_add(1);
                        }
                    }
                }
                Command::Group { commands, .. } => self.walk(commands, to_device),
                _ => {}
            }
        }
    }
}

/// The pdf.js corpus, or whatever the arguments name.
fn population() -> Vec<PathBuf> {
    let named: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    if !named.is_empty() {
        return named;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("the doc/pdf.js submodule is checked out")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
        .collect();
    // Sorted, because a census whose order is the file system's is a census with a second
    // input nobody stated.
    files.sort();
    files
}

fn main() {
    let files = population();
    let mut quorra = QuorraRasterizer::new_headless().expect("an adapter");
    println!("adapter: {}", quorra.adapter_description());
    println!("{} documents, first page each, at {SCALE}×", files.len());

    let mut census = Fills::default();
    let mut drawn = 0u32;
    let mut unreadable = 0u32;
    let mut refused = 0u32;
    // Every page's pair, kept whole: a distribution is what was asked for, and a mean over a
    // corpus whose pages state no clip at all would say nothing about the pages that do.
    let mut pairs: Vec<(u32, u32, String)> = Vec::new();

    for path in &files {
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let Ok(bytes) = std::fs::read(path) else {
            unreadable = unreadable.saturating_add(1);
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            unreadable = unreadable.saturating_add(1);
            continue;
        };
        let Some(page) = pdf_model::Pages::new(&document).get(0) else {
            unreadable = unreadable.saturating_add(1);
            continue;
        };
        let list = pdf_model::content::interpret(&document, &page).display_list;
        let Ok(target) = TargetSpec::for_page(&list, SCALE, PIXEL_BUDGET) else {
            unreadable = unreadable.saturating_add(1);
            continue;
        };
        census.walk(list.commands(), target.transform);
        if quorra.rasterize(&list, target).is_err() {
            // A page the device refuses states its clips all the same, but no frame counted
            // them; counting it as a page with no residue would be a page invented.
            refused = refused.saturating_add(1);
            continue;
        }
        drawn = drawn.saturating_add(1);
        let (regions, tiles) = quorra.last_clip_residue();
        pairs.push((regions, tiles, name));
    }

    println!(
        "\npages: {drawn} drawn, {refused} refused by the device, {unreadable} not readable as \
         a first page"
    );

    println!(
        "\nfills, over every page walked (refused pages included — the fill is stated \
              whether or not it was drawn):"
    );
    println!("  {:>12}  fills reached", census.total);
    println!(
        "  {:>12}  are one axis-aligned rectangle and nothing else ({:.2}%)",
        census.rectangles,
        share(census.rectangles, census.total)
    );
    println!(
        "  {:>12}  of those reach the device as one ({:.2}% of all fills)",
        census.on_axis,
        share(census.on_axis, census.total)
    );

    let with_residue: Vec<&(u32, u32, String)> = pairs
        .iter()
        .filter(|(regions, tiles, _)| *regions > 0 || *tiles > 0)
        .collect();
    let regions: u64 = pairs.iter().map(|(r, _, _)| u64::from(*r)).sum();
    let tiles: u64 = pairs.iter().map(|(_, t, _)| u64::from(*t)).sum();
    println!("\nresidue, over the {drawn} pages a frame was counted for:");
    println!(
        "  {:>12}  pages report (0, 0) — no chain that is not all rectangles",
        u64::from(drawn).saturating_sub(with_residue.len() as u64)
    );
    println!(
        "  {:>12}  pages report a region or a tile ({:.2}%)",
        with_residue.len(),
        share(with_residue.len() as u64, u64::from(drawn))
    );
    println!("  {regions:>12}  regions and {tiles} tiles in total");
    let region_only = with_residue
        .iter()
        .filter(|(_, tiles, _)| *tiles == 0)
        .count();
    let tile_only = with_residue
        .iter()
        .filter(|(regions, _, _)| *regions == 0)
        .count();
    println!(
        "  {region_only:>12}  of them pay regions only, {tile_only} pay tiles only, {} pay both",
        with_residue
            .len()
            .saturating_sub(region_only)
            .saturating_sub(tile_only)
    );

    // The pages themselves, because a distribution is a list and a summary of it is not one.
    let mut listed: Vec<&&(u32, u32, String)> = with_residue.iter().collect();
    listed.sort_by_key(|(regions, tiles, name)| {
        (
            std::cmp::Reverse(u64::from(*tiles).saturating_add(u64::from(*regions))),
            name.clone(),
        )
    });
    println!("\n  {:>8}  {:>8}  page", "regions", "tiles");
    for (regions, tiles, name) in listed {
        println!("  {regions:>8}  {tiles:>8}  {name}");
    }
}

/// A percentage, and zero where there is nothing to take a share of.
fn share(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    part as f64 * 100.0 / whole as f64
}
