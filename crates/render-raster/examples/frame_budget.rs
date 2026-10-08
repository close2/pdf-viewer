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
//! | **seventh** | the turn again, drawn as the seventh frame of a device that has drawn six of another document's — what a window's long-lived device pays for it (ADR 1607) |
//! | **step** | the same page placed at twice the magnification on the moved-view lane, against caches the first placement filled — what one notch of a zoom gesture costs |
//!
//! Every row is the **minimum of `FRAME_BUDGET_ROUNDS` rounds, each on a device of its own**, for
//! the reason `zoom_frame` states: this machine is shared, contention adds time and never removes
//! it, and a device that has drawn the pair already answers the first frame out of its atlas. The
//! one-minute load average is printed before and after, because a figure taken beside a neighbour
//! is worth less than it looks (`doc/habits/measuring.md`).
//!
//! **The device is warmed and the page is not.** Each round draws [`frame_cost::WARM_UP`]'s first page before
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
//! the second interpretation is timed ([`frame_cost::read`] has the argument, and what a one-page document
//! does instead). The viewer's own wrapper adds §12.5.3's replacement snapshot and the wording of
//! the reports, and neither draws anything.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    missing_docs
)]

#[path = "../tests/support/frame_cost.rs"]
mod frame_cost;

use frame_cost::{Stages, WINDOW, keep, round};

/// The refresh `doc/todo/36` names as the target; the floor it names is twice this.
const REFRESH_MS: f64 = 1000.0 / 120.0;

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

/// The one-minute load average, or `None` where this system publishes none.
fn load_average() -> Option<f64> {
    let text = std::fs::read_to_string("/proc/loadavg").ok()?;
    text.split_whitespace().next()?.parse().ok()
}

fn variable(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// One page of the population, with the quickest sample of each of its four rows.
struct Row {
    path: String,
    index: usize,
    class: String,
    commands: usize,
    /// Whether this page's interpretation was timed against the font cache a *predecessor*
    /// filled, which is what a page turn has — false for a one-page document, where a viewer
    /// arriving at the page has an empty cache too.
    preceded: bool,
    stages: [Option<Stages>; 4],
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
            stages: [None, None, None, None],
        })
        .collect();
    for _ in 0..rounds {
        for row in &mut table {
            // `frame_cost::round` is the method, shared with `tests/turn_path.rs`, which holds
            // the turn and step rows this prints to bands (ADR 1513).
            let sample = round(&row.path, row.index, window);
            row.commands = sample.commands;
            row.preceded = sample.preceded;
            sample.adapter.clone_into(&mut adapter);
            keep(&mut row.stages[0], sample.turn);
            keep(&mut row.stages[1], sample.warm);
            keep(&mut row.stages[2], sample.seventh);
            keep(&mut row.stages[3], sample.step);
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
        "{:<26}{:>9}{:>9}{:>8}{:>9}{:>8}{:>10}{:>9}{:>9}{:>12}{:>9}",
        "page / row",
        "budget",
        "interp",
        "scene",
        "encode",
        "transf",
        "elsewhere",
        "execute",
        "readback",
        "bytes",
        "threads"
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
        for (name, slot) in ["turn", "warm", "seventh", "step"].iter().zip(&row.stages) {
            let Some(stage) = slot else {
                continue;
            };
            println!(
                "  {:<24}{:>9.2}{:>9.2}{:>8.2}{:>9.2}{:>8.2}{:>10.2}{:>9.2}{:>9.2}{:>12}{:>9}",
                name,
                stage.budget(),
                stage.interpret,
                stage.scene,
                stage.encode,
                stage.transfer,
                stage.elsewhere,
                stage.execute,
                stage.readback,
                stage.bytes,
                stage.threads
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
