//! Where a frame's 8.333 ms goes on the shipped path: interpretation, scene, encode, transfer,
//! the device's own passes, and what is left over.
//!
//! `doc/todo/36` asks for a picture every refresh — 60 Hz as the floor, 120 Hz as the target — and
//! every instrument this tree had answered part of that question about part of a frame.
//! `examples/zoom_frame` splits a *magnification* into raster's phases and starts from a display
//! list somebody else built; `crates/viewer-ui/tests/launch_path.rs` times a page turn end to end
//! and reports one number for it. Neither says what share of a refresh each stage of a page turn
//! takes, and the two cannot be added: they are measured on different coverage lanes.
//!
//! **The lane is the thing to get right, and it is why this example exists beside `zoom_frame`.**
//! `viewer-ui`'s `lane_for` gives a *moved view* the compute lane and everything else the lane
//! `coverage_for` picks from the magnification, which below 10× is `Coverage::Cpu` (ADR 0700).
//! So a page turn and a zoom step of the same page are drawn by two different rasterisers, and a
//! budget that measures one on the other's lane is a budget for a configuration nobody runs —
//! `doc/todo/47`'s own correction, in the other direction.
//!
//! ```sh
//! cargo run --release -p render-raster --example frame_budget
//! cargo run --release -p render-raster --example frame_budget -- doc/….pdf 101 text
//! FRAME_BUDGET_ROUNDS=5 FRAME_BUDGET_WINDOW=1600x1000 cargo run … --example frame_budget
//! ```
//!
//! # What each row is
//!
//! | row | what produced it |
//! |---|---|
//! | **turn** | the page interpreted and drawn once on a device that has already drawn another document's page — what `Command::GoTo(Next)` costs, on the page-turn lane |
//! | **warm** | the same frame asked for again, nothing changed — what a chrome-only repaint costs |
//! | **step** | the same page placed at twice the magnification on the moved-view lane, against caches the first placement filled — what one notch of a zoom gesture costs |
//!
//! Every row is the **minimum of `FRAME_BUDGET_ROUNDS` rounds, each on a device of its own**, for
//! the reason `zoom_frame` states: this machine is shared, contention adds time and never removes
//! it, and a device that has drawn the pair already answers the first frame out of its atlas. The
//! one-minute load average is printed before and after, because a figure taken beside a neighbour
//! is worth less than it looks (`doc/habits/measuring.md`).
//!
//! **The device is warmed and the page is not.** Each round draws [`WARM_UP`]'s first page before
//! anything is timed, so that the pipeline compile and the first-use allocations a cold device
//! pays — about 12 ms of it, `examples/first_frame` — are not charged to the page turn. A person
//! turning a page has a device that has drawn something; page one's own cold device is
//! `launch_path`'s `first_page_ms`, which is a different figure with its own gate.
//!
//! **So the `turn` row is the expensive end of a page turn, and it is worth saying which end.**
//! The warm-up is another document, so every outline this page states is uploaded during the
//! timed frame — which is what a turn onto a page whose glyphs the device has not seen costs.
//! `launch_path`'s `turn_ms` is the other end: five arrow keys inside *one* document whose page
//! one is already drawn, where a glyph outline is an `Arc` the resource cache already holds. A
//! real session moves between the two, and neither figure is the other's.
//!
//! **`readback` is printed and left out of the budget.** This example draws offscreen, so it pays
//! a copy of the finished frame back off the device that a window never pays: a window frame is
//! drawn into a texture and presented from there (`FrameCost::readback`'s own note). It is in the
//! table so that the subtraction is visible rather than implied.
//!
//! **`present` is not here at all**, and cannot be: the presenter owns the surface on the event
//! thread (ADR 0391) and there is no surface without a display server. `quorra --trace`'s cadence
//! summary is the instrument for that half.
//!
//! **`interp` is `pdf_model::content::interpret_with_fonts` against the font cache a page turn
//! has**, which is the work `viewer-core` commissions for a page it has not read: the viewer keeps
//! one `FontCache` per open document, so the page before this one is interpreted first and only
//! the second interpretation is timed ([`read`] has the argument, and what a one-page document
//! does instead). The viewer's own wrapper adds §12.5.3's replacement snapshot and the wording of
//! the reports, and neither draws anything.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    missing_docs
)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use pdf_render::{DisplayList, TargetSpec, Transform};
use pdf_syntax::Document;
use render_raster::{FrameCost, PresentFrame, QuorraRasterizer};

/// The document every round's device is warmed with, and the page of it.
///
/// Committed to this repository, small, and none of the three witnesses — so it fills the
/// device's pipelines and its first-use allocations without filling any cache the measured page
/// would hit. `doc/todo/36`'s own A/B document, for the same reason: it is the one that is here.
///
/// **A page of *this* document named on the command line is the one input the example cannot
/// answer for**: its outlines are in the device's caches before the timed frame, so its `turn`
/// row is a warm row wearing a cold row's name.
const WARM_UP: (&str, usize) = ("doc/PDF20_AN001-BPC.pdf", 1);

/// The refresh `doc/todo/36` names as the target; the floor it names is twice this.
const REFRESH_MS: f64 = 1000.0 / 120.0;

/// The window every figure is measured in, in device pixels.
///
/// The same viewport `launch_path.rs` states, and for its reason: a frame's cost is a function of
/// how many pixels are drawn, so a figure taken at the machine's own screen size would be a
/// different question on a different machine.
const WINDOW: (u32, u32) = (1600, 1000);

/// The three page classes, and the page of each.
///
/// **Named here rather than derived, and the population is the claim.** A text page, a page whose
/// marks are vector artwork, and a page that is one large photograph are the three shapes whose
/// frames are made of different things — and the numbers below say so: they put their time in
/// three different stages. Each is committed to this repository.
const POPULATION: [(&str, usize, &str); 3] = [
    ("doc/ISO_32000-2_sponsored_EC3.pdf", 101, "text"),
    ("doc/pdf.js/test/pdfs/personwithdog.pdf", 1, "vector"),
    ("doc/pdf.js/test/pdfs/issue12841_reduced.pdf", 1, "image"),
];

/// Milliseconds, the unit every frame line in this project is in.
fn ms(span: Duration) -> f64 {
    span.as_secs_f64() * 1e3
}

/// The one-minute load average, or `None` where this system publishes none.
fn load_average() -> Option<f64> {
    let text = std::fs::read_to_string("/proc/loadavg").ok()?;
    text.split_whitespace().next()?.parse().ok()
}

fn variable(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// One frame's stages, in milliseconds, with the host's walk separated from the library's.
#[derive(Clone, Copy, Default)]
struct Stages {
    interpret: f64,
    scene: f64,
    encode: f64,
    transfer: f64,
    execute: f64,
    elsewhere: f64,
    readback: f64,
    bytes: u64,
    uploads: u32,
}

impl Stages {
    fn of(interpret: f64, cost: FrameCost) -> Self {
        // `device` less the phases raster names, which is host time inside its own `render` —
        // the same arithmetic `zoom_frame` does, and a bound rather than a duration because
        // `execute` is usually the adapter's clock and the rest are this thread's.
        let elsewhere = cost
            .device
            .saturating_sub(cost.encode)
            .saturating_sub(cost.upload)
            .saturating_sub(cost.execute)
            .saturating_sub(cost.readback);
        Self {
            interpret,
            scene: ms(cost.scene),
            encode: ms(cost.encode),
            transfer: ms(cost.upload),
            execute: ms(cost.execute),
            elsewhere: ms(elsewhere),
            readback: ms(cost.readback),
            bytes: cost.bytes_uploaded,
            uploads: cost.uploads,
        }
    }

    /// What a window would wait for: everything but the readback this example alone pays.
    fn budget(self) -> f64 {
        self.interpret + self.scene + self.encode + self.transfer + self.execute + self.elsewhere
    }
}

/// One page of the population, with the quickest sample of each of its three rows.
struct Row {
    path: String,
    index: usize,
    class: String,
    commands: usize,
    /// Whether this page's interpretation was timed against the font cache a *predecessor*
    /// filled, which is what a page turn has — false for a one-page document, where a viewer
    /// arriving at the page has an empty cache too.
    preceded: bool,
    stages: [Option<Stages>; 3],
}

/// The page placed in the window at this magnification, centred — what the keyboard zoom does.
fn placed(list: &DisplayList, zoom: f32, window: (u32, u32)) -> TargetSpec {
    let page = TargetSpec::for_page(list, zoom, 1 << 30).expect("a target");
    let size = list.page_size;
    let centre = Transform::translate(
        (window.0 as f32).mul_add(0.5, -(size.width * zoom * 0.5)),
        (window.1 as f32).mul_add(0.5, -(size.height * zoom * 0.5)),
    );
    TargetSpec {
        width: window.0,
        height: window.1,
        transform: page.transform.then(centre),
    }
}

/// Draws one placed list into the window-sized target and hands back what it cost.
fn draw(backend: &mut QuorraRasterizer, list: &Arc<DisplayList>, target: TargetSpec) -> FrameCost {
    let frame = PresentFrame {
        width: target.width,
        height: target.height,
        pages: &[(list, target)],
        raster: None,
        overlays: &[],
    };
    backend
        .rasterize_frame(&frame)
        .unwrap_or_else(|error| panic!("refused: {error}"));
    backend.last_frame()
}

/// Opens a document and interprets one of its pages, reporting what the interpretation cost.
///
/// **The font cache is the one a page turn has, and getting that wrong halves the figure.**
/// `viewer-core` keeps a [`FontCache`](pdf_model::content::FontCache) for as long as a document
/// is open and hands it to every interpretation, so §9.6's font programs are loaded once per
/// document rather than once per page — a page arrived at by `Command::GoTo(Next)` pays for the
/// faces its predecessor did not use and for no others. So the page *before* this one is
/// interpreted first, against the same cache, and only the second interpretation is timed.
///
/// Where the document has one page there is no predecessor, and the cache is empty exactly as it
/// is when the viewer arrives at such a page. The caller is told which of the two happened.
fn read(path: &str, index: usize) -> (Arc<DisplayList>, f64, usize, bool) {
    let document = Document::open(std::fs::read(path).unwrap_or_else(|error| {
        panic!("{path} is readable: {error}");
    }))
    .expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    let state = pdf_model::view::ViewState::of(&document);
    let fonts = pdf_model::content::FontCache::new();
    let preceded = index > 1;
    if preceded && let Some(before) = pages.get(index.saturating_sub(2)) {
        drop(pdf_model::content::interpret_with_fonts(
            &document, &before, &state, &fonts,
        ));
    }
    let page = pages
        .get(index.saturating_sub(1))
        .unwrap_or_else(|| panic!("{path} has a page {index}"));
    let began = Instant::now();
    let interpretation = pdf_model::content::interpret_with_fonts(&document, &page, &state, &fonts);
    let spent = ms(began.elapsed());
    let list = interpretation.display_list;
    let commands = list.command_count();
    (Arc::new(list), spent, commands, preceded)
}

/// A device on the lane a page turn is drawn on, warmed by [`WARM_UP`] and by nothing else.
fn warmed_device(lane: raster_gpu::Coverage, window: (u32, u32)) -> (QuorraRasterizer, String) {
    let mut backend =
        QuorraRasterizer::with_options(&render_raster::options()).expect("an adapter");
    backend.set_coverage(lane);
    let description = backend.adapter_description().to_owned();
    let (path, page) = WARM_UP;
    let (list, _, _, _) = read(path, page);
    let _warmed = draw(&mut backend, &list, placed(&list, 1.0, window));
    (backend, description)
}

/// Keeps whichever of two samples of the same row is the quicker.
fn keep(slot: &mut Option<Stages>, sample: Stages) {
    if slot.is_none_or(|kept| sample.budget() < kept.budget()) {
        *slot = Some(sample);
    }
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let population: Vec<(String, usize, String)> = match arguments.next() {
        Some(path) => {
            let index: usize = arguments
                .next()
                .map_or(1, |n| n.parse().expect("a page number"));
            let class = arguments.next().unwrap_or_else(|| "given".to_owned());
            vec![(path, index, class)]
        }
        None => POPULATION
            .iter()
            .map(|(path, index, class)| ((*path).to_owned(), *index, (*class).to_owned()))
            .collect(),
    };
    let rounds: usize = variable("FRAME_BUDGET_ROUNDS")
        .and_then(|n| n.parse().ok())
        .unwrap_or(3);
    let window = variable("FRAME_BUDGET_WINDOW").map_or(WINDOW, |spec| {
        let (w, h) = spec.split_once('x').expect("a window as WIDTHxHEIGHT");
        (
            w.parse().expect("a window width"),
            h.parse().expect("a window height"),
        )
    });

    let before = load_average();
    let mut adapter = String::new();
    let mut table: Vec<Row> = population
        .iter()
        .map(|(path, index, class)| Row {
            path: path.clone(),
            index: *index,
            class: class.clone(),
            commands: 0,
            preceded: false,
            stages: [None, None, None],
        })
        .collect();
    for _ in 0..rounds {
        for row in &mut table {
            let (list, interpret, commands, preceded) = read(&row.path, row.index);
            row.commands = commands;
            row.preceded = preceded;
            // The page turn and the repaint after it, on the lane `coverage_for` gives every
            // magnification below 10× — which is the lane the window turns a page on.
            let (mut backend, description) = warmed_device(raster_gpu::Coverage::Cpu, window);
            description.clone_into(&mut adapter);
            let target = placed(&list, 1.0, window);
            let turn = Stages::of(interpret, draw(&mut backend, &list, target));
            keep(&mut row.stages[0], turn);
            let warm = Stages::of(0.0, draw(&mut backend, &list, target));
            keep(&mut row.stages[1], warm);
            drop(backend);
            // The zoom step, on the moved-view lane, against the caches the first placement
            // filled — one notch of a gesture rather than a first sight of the page.
            let (mut backend, _) = warmed_device(raster_gpu::Coverage::Compute, window);
            let _first = draw(&mut backend, &list, target);
            let stepped = placed(&list, 2.0, window);
            let step = Stages::of(0.0, draw(&mut backend, &list, stepped));
            keep(&mut row.stages[2], step);
        }
    }

    report(&table, &adapter, window, rounds, before);
}

/// The table, with each stage's share of one refresh under it.
fn report(table: &[Row], adapter: &str, window: (u32, u32), rounds: usize, before: Option<f64>) {
    println!("frame budget, {adapter}");
    println!(
        "{}×{} window, minima of {rounds} round(s), load average {} → {}",
        window.0,
        window.1,
        before.map_or_else(|| "?".to_owned(), |load| format!("{load:.2}")),
        load_average().map_or_else(|| "?".to_owned(), |load| format!("{load:.2}"))
    );
    println!(
        "the budget is {REFRESH_MS:.3} ms (120 Hz, `doc/todo/36`'s target); the floor it names is {:.3} (60 Hz)",
        REFRESH_MS * 2.0
    );
    println!(
        "{:<26}{:>9}{:>9}{:>8}{:>9}{:>8}{:>10}{:>9}{:>9}{:>12}",
        "page / row",
        "budget",
        "interp",
        "scene",
        "encode",
        "transf",
        "elsewhere",
        "execute",
        "readback",
        "bytes"
    );
    for row in table {
        let stem = std::path::Path::new(&row.path).file_name().map_or_else(
            || row.path.clone(),
            |name| name.to_string_lossy().into_owned(),
        );
        println!(
            "{stem} page {} — {}, {} command(s), fonts {}",
            row.index,
            row.class,
            row.commands,
            if row.preceded {
                "as its predecessor left them"
            } else {
                "empty, as a one-page document leaves them"
            }
        );
        for (name, slot) in ["turn", "warm", "step"].iter().zip(&row.stages) {
            let Some(stage) = slot else {
                continue;
            };
            println!(
                "  {:<24}{:>9.2}{:>9.2}{:>8.2}{:>9.2}{:>8.2}{:>10.2}{:>9.2}{:>9.2}{:>12}",
                name,
                stage.budget(),
                stage.interpret,
                stage.scene,
                stage.encode,
                stage.transfer,
                stage.elsewhere,
                stage.execute,
                stage.readback,
                stage.bytes
            );
            println!(
                "  {:<24}{:>8.0}%{:>8.0}%{:>7.0}%{:>8.0}%{:>7.0}%{:>9.0}%{:>8.0}%{:>9}{:>12}",
                "  of one refresh",
                stage.budget() / REFRESH_MS * 100.0,
                stage.interpret / REFRESH_MS * 100.0,
                stage.scene / REFRESH_MS * 100.0,
                stage.encode / REFRESH_MS * 100.0,
                stage.transfer / REFRESH_MS * 100.0,
                stage.elsewhere / REFRESH_MS * 100.0,
                stage.execute / REFRESH_MS * 100.0,
                "—",
                format!("{} upload(s)", stage.uploads)
            );
        }
    }
    println!(
        "readback is this example's alone — a window draws into a texture and presents from it; \
         present is the presenter thread's and needs a surface (ADR 0391)"
    );
}
