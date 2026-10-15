//! What a document's scripts cost to run, stage by stage, through the view state's own sequence:
//! the open, then each timer tick and the page interpreted again after it (ADR 1950).
//!
//! ```text
//! cargo run --release -p pdf-script-worker --features engine --example script_cost -- \
//!     [--lifted [--minutes M]] [--ticks N] <file.pdf>
//! ```
//!
//! The view state is given an in-process [`Engine`] as its runner: the realm the confined worker
//! keeps, run in this process, so what is timed is the realm, the view state's walk and its edit
//! log, and not the wire every window adds (`viewer_host::policy` hands each a worker). `--lifted` runs every script under a
//! budget no document reaches — ten minutes, or `--minutes`, and no step or loop ceiling — so that a run stopped by
//! [`Budget::FIELD_EVENT`] can be priced as well as named; without it the budget is the one every
//! window runs under. Each tick advances the timers by the period the view state says is owed, so a
//! tick here is one timer run, at whatever pace the engine allows rather than the period's.
//!
//! Prints one line per stage with its wall time in milliseconds, then every sentence the view state
//! reported, so that a script refused by name is seen beside what the rest cost.

#![expect(clippy::print_stdout, reason = "an example prints what it measured")]

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use pdf_model::view::{ScriptEvent, ScriptResult, ScriptRunner, ViewState};
use pdf_script::{Budget, Engine};

/// A budget no document's script reaches, for pricing a run the field-event budget stops.
const LIFTED: Budget = Budget {
    wall: Duration::from_mins(10),
    steps: u64::MAX,
    loop_iterations: u64::MAX,
    recursion: 4096,
    stack: 1 << 20,
    elements: 1 << 28,
    string_units: 1 << 28,
    buffer_bytes: 1 << 28,
    nesting: 1024,
    depth: 12 << 20,
};

/// The engine, with each run's site, script length, wall time and edit count kept for the report.
#[derive(Debug)]
struct Timed {
    /// The runner every event is handed to.
    engine: Engine,
    /// One line per run, in order.
    runs: Mutex<Vec<String>>,
}

impl ScriptRunner for Timed {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        let started = Instant::now();
        let result = self.engine.run(event);
        let line = format!(
            "{:?}: {} bytes of script, {} field(s) told, {:.1} ms, {} edit(s)",
            event.site,
            event.script.len(),
            event.fields.len(),
            millis(started.elapsed()),
            result.edits.len()
        );
        self.runs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(line);
        result
    }
}

/// This process's `VmHWM`, the most memory it has held resident, in mebibytes.
fn high_water() -> f64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    status
        .lines()
        .find(|line| line.starts_with("VmHWM:"))
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<f64>().ok())
        .map_or(0.0, |kilobytes| kilobytes / 1024.0)
}

/// Milliseconds, as a reading.
fn millis(spent: Duration) -> f64 {
    spent.as_secs_f64() * 1000.0
}

/// Each wall time a stage took, summarised on one line.
fn summarise(what: &str, spent: &[f64]) {
    if spent.is_empty() {
        return;
    }
    let (least, most) = spent
        .iter()
        .fold((f64::MAX, 0.0_f64), |(least, most), &each| {
            (least.min(each), most.max(each))
        });
    let mean = spent.iter().sum::<f64>() / f64::from(u32::try_from(spent.len()).unwrap_or(1));
    println!(
        "script_cost: {what}: {} of them, mean {mean:.1} ms, least {least:.1} ms, most {most:.1} ms",
        spent.len()
    );
}

/// Runs `ticks` timer ticks, each the period the view state says is owed, and the page
/// interpreted again after each that changed it; prints the two stages' times.
fn tick(
    document: &pdf_syntax::Document,
    page: &pdf_model::Page,
    view: &mut ViewState,
    ticks: usize,
) {
    let mut spent_ticks = Vec::new();
    let mut spent_draws = Vec::new();
    for _ in 0..ticks {
        let Some(due) = view.timer_due() else {
            println!("script_cost: no timer is set, so no tick runs");
            break;
        };
        let started = Instant::now();
        let ran = view.run_timers(document, due.max(1), 0);
        spent_ticks.push(millis(started.elapsed()));
        if ran.changed {
            let started = Instant::now();
            let drawn = pdf_model::content::interpret_with(document, page, view);
            spent_draws.push(millis(started.elapsed()));
            std::hint::black_box(drawn.display_list.command_count());
        }
    }
    summarise("timer run", &spent_ticks);
    summarise("page interpreted after a tick", &spent_draws);
}

/// The budget, the tick count and the file the command line names.
struct Arguments {
    /// The budget every run is held to.
    budget: Budget,
    /// How many timer ticks to run after the open.
    ticks: usize,
    /// The document.
    path: Option<String>,
}

/// Reads the command line.
fn arguments() -> Arguments {
    let mut lifted = false;
    let mut minutes = 10_u64;
    let mut ticks = 10_usize;
    let mut path = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--lifted" => lifted = true,
            "--minutes" => minutes = args.next().and_then(|n| n.parse().ok()).unwrap_or(minutes),
            "--ticks" => ticks = args.next().and_then(|n| n.parse().ok()).unwrap_or(ticks),
            _ => path = Some(arg),
        }
    }
    let budget = if lifted {
        Budget {
            wall: Duration::from_mins(minutes),
            ..LIFTED
        }
    } else {
        Budget::FIELD_EVENT
    };
    Arguments {
        budget,
        ticks,
        path,
    }
}

fn main() {
    let Arguments {
        budget,
        ticks,
        path,
    } = arguments();
    let Some(path) = path else {
        println!("script_cost: give a PDF file");
        return;
    };
    let Ok(bytes) = std::fs::read(&path) else {
        println!("script_cost: {path} cannot be read");
        return;
    };
    let started = Instant::now();
    let document = match pdf_syntax::Document::open(bytes) {
        Ok(document) => document,
        Err(error) => {
            println!("script_cost: {path} does not open: {error}");
            return;
        }
    };
    println!(
        "script_cost: open the file: {:.1} ms",
        millis(started.elapsed())
    );
    let Some(page) = pdf_model::Pages::new(&document).get(0) else {
        println!("script_cost: {path} has no page one");
        return;
    };
    let mut view = ViewState::of(&document);
    let timed = Arc::new(Timed {
        engine: Engine::new(budget),
        runs: Mutex::new(Vec::new()),
    });
    view.run_scripts_with(Some(Arc::clone(&timed) as Arc<dyn ScriptRunner>));

    let started = Instant::now();
    let first = pdf_model::content::interpret_with(&document, &page, &view);
    println!(
        "script_cost: page one interpreted before any script: {:.1} ms, {} commands",
        millis(started.elapsed()),
        first.display_list.command_count()
    );
    let started = Instant::now();
    let handed = view.run_open_scripts(&document, 0);
    println!(
        "script_cost: the open sequence: {:.1} ms, {handed} script(s) handed to the runner; \
         resident high water {:.1} MiB",
        millis(started.elapsed()),
        high_water()
    );
    tick(&document, &page, &mut view, ticks);

    let runs = timed.runs.lock().unwrap_or_else(PoisonError::into_inner);
    let tail = runs.len().saturating_sub(4);
    for (index, line) in runs.iter().enumerate() {
        // The first runs and the last few: a tick's line repeats.
        if index < 16 || index >= tail {
            println!("script_cost: run {index}: {line}");
        }
    }
    for sentence in view.script_reports() {
        println!(
            "script_cost: reported: {}",
            sentence.chars().take(400).collect::<String>()
        );
    }
}
