//! What share of a function-based shading's grid a magnified window actually shows.
//!
//! ISO 32000-2 §8.7.4.5.2's type 1 shading has no resolution of its own, so the display list
//! carries a producer and a backend asks it for one cell per device pixel of the *domain*
//! (`pdf_render::Shading::sampled_at`). A viewer past the magnification at which a page fits
//! does not rasterise the page: it rasterises the **window**, at a transform that scales the
//! page and translates the region of interest into view — which is what
//! `render-quorra/examples/zoom_ladder.rs` models and what `viewer-ui`'s own surface builds.
//! So the grid grows with the magnification while the window does not, and the share of it a
//! person can see falls as the square of the zoom.
//!
//! This is the A/B behind ADR 0408, and it is taken **in one sitting** the way
//! `doc/habits.md` asks: for every page carrying a type 1 shading it resolves the whole grid
//! (`Patch::whole`, which is what every backend asked for before that decision) and then the
//! block the window can sample (`Shading::sampled_at`, which is what they ask for now), at
//! each rung of a zoom ladder, and prints the cells and the milliseconds of both.
//!
//! **Both come from the shipping path** — the same producer, through the same vocabulary —
//! rather than from a second implementation of the conditions. `doc/habits.md`'s rule about a
//! measurement taken with the instrument under test is why neither side is recomputed here.
//!
//! ```sh
//! cargo run --release -p pdf-model --example shading_grid_census -- \
//!     doc/pdf.js/test/pdfs/*.pdf doc/corpora/*/**/*.pdf doc/corpora-own/*.pdf
//! ```
//!
//! An argument beginning with `@` names a file of paths, one to a line. With no arguments at
//! all it walks `doc/pdf.js/test/pdfs`.
#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    reason = "a measurement whose output is its purpose: it stops loudly where the corpus is \
              missing, and its counters over the corpus's shadings are orders of magnitude \
              below what a usize counts"
)]

use std::path::{Path, PathBuf};

use pdf_render::{
    Command, DisplayList, Grid, Paint, Patch, Shading, ShadingKind, TargetSpec, Transform,
};
use pdf_syntax::Document;

/// The window this pretends to be, which is `zoom_ladder`'s and the owner's own screen's.
const WINDOW: (u32, u32) = (900, 1100);

/// The rungs, each a magnification the page is drawn at inside that window.
const RUNGS: [f32; 4] = [1.0, 2.0, 4.0, 8.0];

fn paths() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for argument in std::env::args().skip(1) {
        if let Some(list) = argument.strip_prefix('@') {
            let text = std::fs::read_to_string(list).expect("the list of paths is readable");
            files.extend(
                text.lines()
                    .filter(|line| !line.is_empty())
                    .map(PathBuf::from),
            );
        } else {
            files.push(PathBuf::from(argument));
        }
    }
    if files.is_empty() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
        let mut found: Vec<PathBuf> = std::fs::read_dir(&root)
            .expect("the submodule is checked out")
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|e| e == "pdf"))
            .collect();
        found.sort();
        files = found;
    }
    files
}

/// Every type 1 shading a display list paints with, groups included.
fn sampled_shadings(list: &[Command], out: &mut Vec<Shading>) {
    for command in list {
        match command {
            Command::Fill {
                paint: Paint::Shading(shading),
                ..
            } if matches!(shading.kind.as_ref(), ShadingKind::Sampled { .. }) => {
                out.push(shading.as_ref().clone());
            }
            Command::Group { commands, .. } => sampled_shadings(commands, out),
            _ => {}
        }
    }
}

/// The transform carrying the domain's unit square onto the device.
///
/// The same composition `Shading::sampled_at` derives its grid from — stated here because the
/// unclipped arm has to ask the producer for the lattice that composition names.
fn placement(shading: &Shading, page_to_device: Transform) -> Option<Transform> {
    let ShadingKind::Sampled { domain, .. } = shading.kind.as_ref() else {
        return None;
    };
    let [x0, x1, y0, y1] = *domain;
    let onto_domain = Transform::new(x1 - x0, 0.0, 0.0, y1 - y0, x0, y0);
    Some(onto_domain.then(shading.transform).then(page_to_device))
}

/// The window target for a page at this magnification, its middle held in the window's.
fn window_target(list: &DisplayList, zoom: f32) -> Option<TargetSpec> {
    let size = list.page_size;
    let page = TargetSpec::for_page(list, zoom, u64::MAX).ok()?;
    let (w, h) = (WINDOW.0 as f32, WINDOW.1 as f32);
    let centre = Transform::translate(
        w.mul_add(0.5, -(size.width * zoom * 0.5)),
        h.mul_add(0.5, -(size.height * zoom * 0.5)),
    );
    Some(TargetSpec {
        width: WINDOW.0,
        height: WINDOW.1,
        transform: page.transform.then(centre),
    })
}

/// One arm of the A/B: cells produced and the milliseconds producing them took.
#[derive(Default, Clone, Copy)]
struct Arm {
    cells: u64,
    spent: f64,
}

impl Arm {
    fn add(&mut self, other: Self) {
        self.cells += other.cells;
        self.spent += other.spent;
    }
}

/// The whole grid, which is what every backend asked for before ADR 0408.
fn unclipped(shading: &Shading, page_to_device: Transform) -> Option<Arm> {
    let ShadingKind::Sampled { source, .. } = shading.kind.as_ref() else {
        return None;
    };
    let place = placement(shading, page_to_device)?;
    let began = std::time::Instant::now();
    let grid = source.colours(Patch::whole(Grid::for_placement(place)));
    Some(Arm {
        cells: u64::from(grid.width) * u64::from(grid.height),
        spent: began.elapsed().as_secs_f64() * 1e3,
    })
}

/// The block the target can sample, which is what they ask for now.
fn clipped(shading: &Shading, target: TargetSpec) -> Option<Arm> {
    let began = std::time::Instant::now();
    let grid = shading.sampled_at(target.transform, (target.width, target.height))?;
    Some(Arm {
        cells: u64::from(grid.width) * u64::from(grid.height),
        spent: began.elapsed().as_secs_f64() * 1e3,
    })
}

fn main() {
    let mut pages = 0usize;
    let mut shadings = 0usize;
    let mut documents: Vec<String> = Vec::new();
    let mut totals = [(Arm::default(), Arm::default()); RUNGS.len()];
    let mut rows: Vec<String> = Vec::new();

    for path in paths() {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let all = pdf_model::Pages::new(&document);
        let mut mine = 0usize;
        for index in 0..all.len() {
            let Some(page) = all.get(index) else {
                continue;
            };
            let list = pdf_model::content::interpret(&document, &page).display_list;
            let mut found = Vec::new();
            sampled_shadings(list.commands(), &mut found);
            if found.is_empty() {
                continue;
            }
            pages += 1;
            shadings += found.len();
            mine += found.len();
            for (rung, zoom) in RUNGS.iter().enumerate() {
                let Some(target) = window_target(&list, *zoom) else {
                    continue;
                };
                let (mut whole, mut block) = (Arm::default(), Arm::default());
                for shading in &found {
                    if let Some(arm) = unclipped(shading, target.transform) {
                        whole.add(arm);
                    }
                    if let Some(arm) = clipped(shading, target) {
                        block.add(arm);
                    }
                }
                totals[rung].0.add(whole);
                totals[rung].1.add(block);
                rows.push(format!(
                    "{name} p{} {:>4.0}%  {:>10} → {:>10} cells  {:>6.1}%  \
                     {:>8.1} → {:>8.1} ms",
                    index + 1,
                    zoom * 100.0,
                    whole.cells,
                    block.cells,
                    share(block.cells, whole.cells),
                    whole.spent,
                    block.spent,
                ));
            }
        }
        if mine > 0 {
            documents.push(format!("{name} ({mine})"));
        }
    }

    println!("window {} × {}", WINDOW.0, WINDOW.1);
    println!(
        "{pages} page(s) carrying {shadings} type 1 shading(s), in {} document(s)",
        documents.len()
    );
    for document in &documents {
        println!("  {document}");
    }
    println!();
    for row in &rows {
        println!("{row}");
    }
    println!();
    println!(
        "{:>6}  {:>14}  {:>14}  {:>8}  {:>12}  {:>12}",
        "zoom", "whole grid", "block drawn", "share", "whole", "block"
    );
    for (rung, zoom) in RUNGS.iter().enumerate() {
        let (whole, block) = totals[rung];
        println!(
            "{:>5.0}%  {:>14}  {:>14}  {:>7.1}%  {:>9.1} ms  {:>9.1} ms",
            zoom * 100.0,
            whole.cells,
            block.cells,
            share(block.cells, whole.cells),
            whole.spent,
            block.spent,
        );
    }
}

/// `part` as a percentage of `whole`, and zero where there is no whole.
fn share(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 * 100.0 / whole as f64
    }
}
