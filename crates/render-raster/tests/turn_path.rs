//! `doc/performance.md`'s table of what a page turn and a zoom step cost, held to bands.
//!
//! # Why this exists
//!
//! `CLAUDE.md` principle 2: "Perf gates run in CI: cold open, time-to-first-page, page-turn
//! latency, memory high-water. A regression fails the build." `crates/viewer-ui/tests/launch_path.rs`
//! holds the first two and one page turn inside one already-drawn document — the cheap end of
//! the gesture. The table `examples/frame_budget` prints is the other end, a turn onto a page whose
//! outlines the device has not seen and a notch of zoom on it, across ten page classes, and until
//! this gate nothing held it: three of its rows doubled over two batches and the table was the only
//! place it showed, the next time somebody re-took it by hand (ADR 1513).
//!
//! This holds the `turn` and `step` rows of every page of that table to a band each in
//! [`doc/checks/turn-path.toml`](../../../doc/checks/turn-path.toml), measured by the table's own
//! method — `support/frame_cost.rs` is the one copy of it, which `frame_budget` prints from.
//!
//! # How a figure is taken, and when it is judged
//!
//! The launch gate's discipline, for its reasons, which that file's module comment argues:
//!
//! - **Each figure is the minimum of [`SAMPLES`] fresh processes of [`ROUNDS`] rounds each**,
//!   pinned to the machine's fastest cores (derived from `cpuinfo_max_freq`, never written down)
//!   with rayon given exactly those cores — the table's "minimum of three runs of five rounds,
//!   pinned". Contention adds time and never removes it.
//! - **Every child runs the launch gate's calibration probe after its rounds**: the smallest
//!   committed document opened from memory and its first page interpreted, fifty times, the
//!   quickest. A child whose probe is outside the check file's band did not have the machine it
//!   claims to measure, and its figures are printed and not used.
//! - **A child is started only below a load average of one per physical core**, re-tried up to
//!   [`LOAD_ATTEMPTS`] times, because above it a child shares a core with a neighbour and nothing
//!   in a clock can subtract that (the launch gate's `the_load_ceiling`, ADR 0916).
//! - **Judged only under the profile the bands state**, pinned, with at least one fit child per
//!   row. Anything else prints every figure and says why it judged none of them — the run is then
//!   a measurement, not a verdict, and says so rather than passing or failing on a machine nobody
//!   measured.
//! - **Each row carries a witness that cannot move with the machine**: the page's command count.
//!   A page that now draws a different list is a different measurement, and the run fails on it
//!   whatever the clock says, so that a band is never silently held by a page drawing less.
//!
//! A figure that falls out of the *bottom* of its band fails too: a page twice as fast is either
//! a win, which is re-banded on purpose with its reason written beside it, or an instrument that
//! stopped measuring.
//!
//! # Running it
//!
//! ```text
//! cargo test --release -p render-raster --test turn_path -- --ignored --nocapture
//! ```
//!
//! It draws on the real adapter without a window, as `frame_budget` does, and takes a few
//! minutes: ten pages, three children each, five rounds a child.

#![expect(
    clippy::print_stdout,
    reason = "a gate whose output is the numbers a person reads to see what moved"
)]
#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    reason = "a gate states a broken measurement by stopping, and its window is a few thousand \
              pixels"
)]

#[path = "support/frame_cost.rs"]
mod frame_cost;

use std::path::PathBuf;
use std::process::Command as Child;
use std::time::Instant;

use frame_cost::{Stages, WINDOW, keep, round};

/// Names what a child process is to do; unset in the parent.
const PHASE: &str = "PDFVIEWER_TURN_PHASE";

/// Names the document a child measures, relative to the repository root.
const DOCUMENT: &str = "PDFVIEWER_TURN_DOCUMENT";

/// Names the page of it, one-based.
const PAGE: &str = "PDFVIEWER_TURN_PAGE";

/// Names the document the child's calibration probe opens.
const CALIBRATION: &str = "PDFVIEWER_TURN_CALIBRATION";

/// What a child's one line of figures begins with.
const MARKER: &str = "measured ";

/// Rounds a child takes of its page, keeping the quickest of each row: the table's five.
const ROUNDS: usize = 5;

/// Children a figure is the minimum of: the table's three runs.
const SAMPLES: usize = 3;

/// How many times a child waits for the load to fall under the ceiling before it is started
/// anyway and its figures are printed unjudged.
const LOAD_ATTEMPTS: usize = 3;

/// The calibration probe's passes: the launch gate's fifty.
const CALIBRATION_PASSES: usize = 50;

/// The repository root, where every path in the check file is relative to.
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The one test a child runs: nothing in the parent's run, one page's rounds in a child's.
///
/// Not `#[ignore]`d, so a workspace test run executes it — where it returns at once — and the
/// gate below re-executes this binary selecting it, the launch gate's own idiom.
#[test]
fn turn_probe() {
    let Ok(phase) = std::env::var(PHASE) else {
        return;
    };
    assert_eq!(phase, "measure", "the only phase a child runs");
    let document = std::env::var(DOCUMENT).expect("a document");
    let page: usize = std::env::var(PAGE)
        .expect("a page")
        .parse()
        .expect("a page number");
    let (mut turn, mut warm, mut step) = (None, None, None);
    let (mut commands, mut preceded) = (0, false);
    for _ in 0..ROUNDS {
        let sample = round(&document, page, WINDOW);
        (commands, preceded) = (sample.commands, sample.preceded);
        println!("adapter {}", sample.adapter);
        keep(&mut turn, sample.turn);
        keep(&mut warm, sample.warm);
        keep(&mut step, sample.step);
    }
    let rounds = "five rounds";
    let (turn, warm, step) = (
        turn.expect(rounds),
        warm.expect(rounds),
        step.expect(rounds),
    );
    let calibration = calibration_ms();
    println!(
        "{MARKER}turn_ms={:.3} turn_interp={:.3} turn_encode={:.3} turn_transfer={:.3} \
         turn_readback={:.3} warm_ms={:.3} step_ms={:.3} step_encode={:.3} \
         step_transfer={:.3} step_bytes={} step_uploads={} commands={commands} \
         preceded={} calibration_ms={calibration:.3}",
        turn.budget(),
        turn.interpret,
        turn.encode,
        turn.transfer,
        turn.readback,
        warm.budget(),
        step.budget(),
        step.encode,
        step.transfer,
        step.bytes,
        step.uploads,
        u8::from(preceded),
    );
}

/// The launch gate's probe: the calibration document opened from bytes in memory and its first
/// page interpreted, [`CALIBRATION_PASSES`] times, the quickest, in milliseconds.
fn calibration_ms() -> f64 {
    let path = std::env::var(CALIBRATION).expect("a calibration document");
    let bytes = std::fs::read(&path).expect("the calibration document reads");
    let mut quickest = f64::INFINITY;
    for _ in 0..CALIBRATION_PASSES {
        let began = Instant::now();
        let opened = pdf_syntax::Document::open(bytes.clone()).expect("it opens");
        let pages = pdf_model::Pages::new(&opened);
        let page = pages.get(0).expect("a first page");
        let interpreted = pdf_model::interpret(&opened, &page);
        assert!(
            interpreted.display_list.command_count() > 0,
            "the calibration page draws"
        );
        quickest = quickest.min(frame_cost::ms(began.elapsed()));
    }
    quickest
}

/// A band, `low .. high`.
#[derive(Debug, Clone, Copy)]
struct Band {
    low: f64,
    high: f64,
}

impl Band {
    fn holds(self, value: f64) -> bool {
        value >= self.low && value <= self.high
    }
}

/// One `[[page]]` of the check file.
#[derive(Debug, Default)]
struct Row {
    path: String,
    page: usize,
    commands: usize,
    turn_ms: Option<Band>,
    step_ms: Option<Band>,
}

/// The check file.
#[derive(Debug, Default)]
struct Check {
    profile: String,
    calibration_document: String,
    calibration_ms: Option<Band>,
    pages: Vec<Row>,
}

/// Reads `low .. high`.
fn band(value: &str, at: usize) -> Band {
    let (low, high) = value
        .split_once("..")
        .unwrap_or_else(|| panic!("line {at} is not `low .. high`"));
    let number = |part: &str| -> f64 {
        part.trim()
            .parse()
            .unwrap_or_else(|_| panic!("line {at}'s bound is not a number"))
    };
    Band {
        low: number(low),
        high: number(high),
    }
}

/// Reads the subset of TOML the check file is written in: comments, `key = value` lines and
/// `[[page]]` headers. A key it does not know is an error rather than a silence, so a band
/// misspelt is a failure rather than a figure nobody judges.
fn parse(text: &str) -> Check {
    let mut check = Check::default();
    for (index, line) in text.lines().enumerate() {
        let at = index.saturating_add(1);
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[page]]" {
            check.pages.push(Row::default());
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .unwrap_or_else(|| panic!("line {at} is not `key = value`"));
        let (key, value) = (key.trim(), value.trim());
        let text = || value.trim_matches('"').to_owned();
        let count = || -> usize {
            value
                .parse()
                .unwrap_or_else(|_| panic!("line {at} is not a count"))
        };
        match (check.pages.last_mut(), key) {
            (None, "machine") | (Some(_), "why") => {}
            (None, "profile") => check.profile = text(),
            (None, "calibration_document") => check.calibration_document = text(),
            (None, "calibration_ms") => check.calibration_ms = Some(band(value, at)),
            (Some(row), "path") => row.path = text(),
            (Some(row), "page") => row.page = count(),
            (Some(row), "commands") => row.commands = count(),
            (Some(row), "turn_ms") => row.turn_ms = Some(band(value, at)),
            (Some(row), "step_ms") => row.step_ms = Some(band(value, at)),
            _ => panic!("line {at}: `{key}` is not a key of the check file"),
        }
    }
    check
}

/// The CPUs of the machine's fastest class, as `taskset` takes them, or `None` where every CPU
/// is of one class — the launch gate's `the_performance_cores`.
fn performance_cores() -> Option<(String, usize)> {
    let mut speeds: Vec<(usize, u64)> = Vec::new();
    for cpu in 0..1024_usize {
        let path = format!("/sys/devices/system/cpu/cpu{cpu}/cpufreq/cpuinfo_max_freq");
        let Ok(text) = std::fs::read_to_string(&path) else {
            break;
        };
        speeds.push((cpu, text.trim().parse().ok()?));
    }
    let fastest = speeds.iter().map(|&(_, speed)| speed).max()?;
    if speeds.iter().all(|&(_, speed)| speed == fastest) {
        return None;
    }
    let list: Vec<String> = speeds
        .iter()
        .filter(|&&(_, speed)| speed == fastest)
        .map(|&(cpu, _)| cpu.to_string())
        .collect();
    let count = list.len();
    Some((list.join(","), count))
}

/// The machine's physical core count, which is the load average a child may start under — the
/// launch gate's `the_load_ceiling` and its argument.
fn load_ceiling() -> f64 {
    let mut seen = std::collections::BTreeSet::new();
    for cpu in 0..1024_usize {
        let topology = format!("/sys/devices/system/cpu/cpu{cpu}/topology");
        let (Ok(core), Ok(package)) = (
            std::fs::read_to_string(format!("{topology}/core_id")),
            std::fs::read_to_string(format!("{topology}/physical_package_id")),
        ) else {
            break;
        };
        seen.insert((package.trim().to_owned(), core.trim().to_owned()));
    }
    u32::try_from(seen.len())
        .ok()
        .filter(|&cores| cores > 0)
        .map_or(f64::INFINITY, f64::from)
}

/// The one-minute load average, or infinity where there is no `/proc`.
fn load_average() -> f64 {
    std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|text| text.split_whitespace().next()?.parse().ok())
        .unwrap_or(f64::INFINITY)
}

/// The cargo profile this binary was built under: the directory above `deps`.
fn profile_of_this_binary() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|exe| {
            exe.parent()?
                .parent()?
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

/// One child's figures, by key.
type Fields = std::collections::BTreeMap<String, f64>;

/// Runs one child on `row`, pinned where `pinning` names cores, and reads its line.
fn child(row: &Row, check: &Check, pinning: Option<&(String, usize)>) -> Fields {
    let exe = std::env::current_exe().expect("this binary");
    let mut command = if let Some((cores, count)) = pinning {
        let mut wrapper = Child::new("taskset");
        wrapper.arg("-c").arg(cores).arg(exe);
        // Rayon is given the cores the child is pinned to, which is what it finds by itself
        // under `taskset` and what the table was taken with — and not the four a batch's gate
        // runner sets for its corpus walks.
        wrapper.env("RAYON_NUM_THREADS", count.to_string());
        wrapper
    } else {
        let mut bare = Child::new(exe);
        bare.env_remove("RAYON_NUM_THREADS");
        bare
    };
    let output = command
        .args(["--exact", "turn_probe", "--nocapture", "--test-threads=1"])
        .current_dir(root())
        // Headless, as `frame_budget` draws: a display the agent user has no cookie for costs
        // a failed handshake per device.
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env(PHASE, "measure")
        .env(DOCUMENT, &row.path)
        .env(PAGE, row.page.to_string())
        .env(CALIBRATION, &check.calibration_document)
        .output()
        .expect("the child runs");
    let said = String::from_utf8_lossy(&output.stdout);
    let Some(line) = said
        .lines()
        .find_map(|line| line.split_once(MARKER).map(|(_, figures)| figures))
    else {
        let complaint = String::from_utf8_lossy(&output.stderr);
        let last: Vec<&str> = complaint.lines().rev().take(4).collect();
        panic!(
            "the child for {} page {} measured nothing ({}): {}",
            row.path,
            row.page,
            output.status,
            last.join(" / ")
        );
    };
    line.split_whitespace()
        .filter_map(|piece| piece.split_once('='))
        .filter_map(|(key, value)| Some((key.to_owned(), value.parse().ok()?)))
        .collect()
}

/// What one row's children came to: the quickest fit child's figures, or why there is none.
struct Taken {
    /// The quickest turn and step over fit children, and that turn's child's other fields.
    best: Option<(f64, f64, Fields)>,
    /// The quickest over every child, fit or not, printed where nothing fit.
    any: Option<(f64, f64)>,
    commands: Option<f64>,
    unfit: Vec<String>,
}

/// Takes [`SAMPLES`] children of `row`, keeping only those the machine was fit for.
fn take(row: &Row, check: &Check, pinning: Option<&(String, usize)>, ceiling: f64) -> Taken {
    let mut taken = Taken {
        best: None,
        any: None,
        commands: None,
        unfit: Vec::new(),
    };
    for _ in 0..SAMPLES {
        let mut load = load_average();
        for _ in 1..LOAD_ATTEMPTS {
            if load <= ceiling {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(10));
            load = load_average();
        }
        let fields = child(row, check, pinning);
        let figure = |key: &str| fields.get(key).copied().expect("the child printed it");
        let (turn, step, calibration) = (
            figure("turn_ms"),
            figure("step_ms"),
            figure("calibration_ms"),
        );
        taken.commands = Some(figure("commands"));
        taken.any = Some(
            taken
                .any
                .map_or((turn, step), |(t, s)| (t.min(turn), s.min(step))),
        );
        println!(
            "    child: turn {turn:.2} step {step:.2} ms, calibration {calibration:.3} ms, load \
             {load:.2}"
        );
        let fit_load = load <= ceiling;
        let fit_probe = check
            .calibration_ms
            .is_some_and(|band| band.holds(calibration));
        if fit_load && fit_probe {
            match &mut taken.best {
                Some((best_turn, best_step, kept)) => {
                    *best_step = best_step.min(step);
                    if turn < *best_turn {
                        *best_turn = turn;
                        *kept = fields;
                    }
                }
                None => taken.best = Some((turn, step, fields)),
            }
        } else {
            taken.unfit.push(format!(
                "load {load:.2} against {ceiling}, calibration {calibration:.3} ms"
            ));
        }
    }
    taken
}

/// Every turn and step row of `doc/performance.md`'s table inside its band.
///
/// `#[ignore]`d for the launch gate's reason: it spawns thirty processes that each bring a
/// graphics device up ten times, and takes minutes. `tools/batch.sh gates` runs it behind the
/// lock, and `doc/verify.md` has the line.
#[test]
#[ignore = "spawns a pinned process per sample on the real adapter and takes minutes; run it from \
            tools/batch.sh gates or doc/verify.md"]
fn the_turn_path_stays_inside_its_bands() {
    let file = root().join("doc/checks/turn-path.toml");
    let check = parse(&std::fs::read_to_string(&file).expect("doc/checks/turn-path.toml reads"));
    assert!(!check.pages.is_empty(), "the check file names no page");
    let profile = profile_of_this_binary();
    let pinning = performance_cores();
    let ceiling = load_ceiling();
    let mut declined: Vec<String> = Vec::new();
    if profile != check.profile {
        declined.push(format!(
            "built under `{profile}`, and the bands are a claim about `{}`",
            check.profile
        ));
    }
    if pinning.is_none() {
        declined.push("no faster class of core to pin to".to_owned());
    }
    println!(
        "turn path: {} page(s), {SAMPLES} children of {ROUNDS} rounds each, pinned to {}, load \
         ceiling {ceiling}, profile {profile}",
        check.pages.len(),
        pinning
            .as_ref()
            .map_or("nothing", |(cores, _)| cores.as_str()),
    );
    let mut tally = Tally::default();
    for row in &check.pages {
        if !root().join(&row.path).is_file() {
            tally.missing = tally.missing.saturating_add(1);
            println!("  {} — absent, skipped", row.path);
            continue;
        }
        let taken = take(row, &check, pinning.as_ref(), ceiling);
        tally.row(row, &taken, &declined);
    }
    println!(
        "{} figures banded, {} judged, {} outside; {} page(s) absent",
        tally.banded,
        tally.judged,
        tally.failures.len(),
        tally.missing
    );
    assert!(
        tally.missing < check.pages.len(),
        "none of the check file's pages is here, so nothing was measured"
    );
    assert!(tally.failures.is_empty(), "{}", tally.failures.join("\n"));
}

/// What a run has judged so far.
#[derive(Default)]
struct Tally {
    banded: usize,
    judged: usize,
    missing: usize,
    failures: Vec<String>,
}

impl Tally {
    /// Prints and judges one row's two figures and its witness; `declined` is why the run
    /// judges nothing, where it judges nothing.
    fn row(&mut self, row: &Row, taken: &Taken, declined: &[String]) {
        let commands = taken.commands.unwrap_or(0.0);
        #[expect(clippy::float_cmp)] // a count printed as an integer, compared exactly
        if commands != row.commands as f64 {
            self.failures.push(format!(
                "{} page {} drew {commands} commands where its bands were taken on {}: a \
                 different list is a different measurement",
                row.path, row.page, row.commands
            ));
        }
        let name = std::path::Path::new(&row.path).file_name().map_or_else(
            || row.path.clone(),
            |name| name.to_string_lossy().into_owned(),
        );
        let why = if declined.is_empty() {
            taken.unfit.join("; ")
        } else {
            declined.join("; ")
        };
        let best = taken.best.as_ref();
        self.figure(
            (&name, row.page, "turn"),
            row.turn_ms,
            best.map(|(turn, _, _)| *turn)
                .filter(|_| declined.is_empty()),
            (taken.any.map(|(turn, _)| turn), &why),
        );
        self.figure(
            (&name, row.page, "step"),
            row.step_ms,
            best.map(|(_, step, _)| *step)
                .filter(|_| declined.is_empty()),
            (taken.any.map(|(_, step)| step), &why),
        );
        if let Some((_, _, fields)) = best {
            let stage = |key: &str| fields.get(key).map_or(0.0, |value| *value);
            println!(
                "    turn interp {:.2} encode {:.2} transfer {:.2}; step encode {:.2} transfer \
                 {:.2}; calibration {:.3} ms; {commands} commands",
                stage("turn_interp"),
                stage("turn_encode"),
                stage("turn_transfer"),
                stage("step_encode"),
                stage("step_transfer"),
                stage("calibration_ms"),
            );
        }
    }

    /// Prints one figure and judges it where `fit` is the figure of a fit child; otherwise
    /// prints the quickest of any child and why it was not judged.
    fn figure(
        &mut self,
        (name, page, figure): (&str, usize, &str),
        band: Option<Band>,
        fit: Option<f64>,
        (any, why): (Option<f64>, &str),
    ) {
        let Some(band) = band else {
            println!("  {name} p{page} {figure}: no band stated");
            return;
        };
        self.banded = self.banded.saturating_add(1);
        let Some(value) = fit else {
            println!(
                "  {name} p{page} {figure} {} ms, band {:.2} .. {:.2}: not judged ({why})",
                any.map_or_else(|| "?".to_owned(), |value| format!("{value:.2}")),
                band.low,
                band.high,
            );
            return;
        };
        self.judged = self.judged.saturating_add(1);
        let verdict = if band.holds(value) {
            "inside"
        } else {
            self.failures.push(format!(
                "{name} p{page} {figure} {value:.2} ms outside {:.2} .. {:.2}",
                band.low, band.high
            ));
            "OUTSIDE"
        };
        println!(
            "  {name} p{page} {figure} {value:.2} ms, band {:.2} .. {:.2}: {verdict}",
            band.low, band.high
        );
    }
}

/// The check file parses, names no key the gate does not read, and states both bands of every
/// row with a floor below its ceiling — read without measuring anything, so it runs in every
/// workspace test run.
#[test]
fn the_check_file_states_a_band_for_every_row() {
    let check = parse(
        &std::fs::read_to_string(root().join("doc/checks/turn-path.toml"))
            .expect("doc/checks/turn-path.toml reads"),
    );
    assert!(check.calibration_ms.is_some(), "a calibration band");
    assert!(!check.pages.is_empty(), "a page");
    for row in &check.pages {
        assert!(
            row.page > 0 && row.commands > 0,
            "{} names its page",
            row.path
        );
        for band in [row.turn_ms, row.step_ms] {
            let band = band.unwrap_or_else(|| panic!("{} states both bands", row.path));
            assert!(
                band.low < band.high,
                "{}'s floor is below its ceiling",
                row.path
            );
        }
    }
}

/// [`Stages`] is the table's quantity: the budget a window waits for, the readback left out.
#[test]
fn a_budget_leaves_the_readback_out() {
    let stages = Stages {
        interpret: 1.0,
        encode: 2.0,
        readback: 100.0,
        ..Stages::default()
    };
    assert!((stages.budget() - 3.0).abs() < 1e-12);
}
