//! What a graphics device does when the process holding it is confined.
//!
//! `doc/todo/34` §2's second way out gives the confined process a window handle and lets it
//! drive the device itself. That reads as a transport question and is not one: the device is
//! not a buffer the process owns, it is a *conversation with a kernel driver*, and
//! `pdf_sandbox::lockdown` ends every conversation a confined process can have. Whether the
//! conversation survives if it was opened *before* the filter went on is the question this
//! answers, because "create the device, then confine" is the only shape of that option that
//! does not require putting `openat` and `ioctl` on the interpreter's allow-list.
//!
//! Each stage runs in a **child process**, because the seccomp action is `KillProcess`: the
//! parent reports what happened to each child rather than dying with it.
//!
//! ```sh
//! cargo run --release -p render-quorra --example device_under_confinement
//! ```
//!
//! The stages, in the order that isolates the answer:
//!
//! - `warm` — bring the device up, draw a frame, draw a second. The control: no confinement,
//!   so a failure here is the machine rather than the filter.
//! - `confine-then-draw` — bring the device up, confine, then draw. This is the option's own
//!   shape.
//! - `draw-then-confine` — bring the device up, draw once so that every pipeline the frame
//!   needs exists, confine, then draw the same frame again. This is the option's *best* case,
//!   and it is the one `CLAUDE.md` section 2 forbids a launch path from waiting for.
//!
//! A stage that is killed prints nothing and its signal is what the parent reports; signal 31
//! is `SIGSYS`, which is the filter.
#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    reason = "a measurement whose whole output is what it printed; a run that cannot bring a \
              device up should stop loudly rather than report a confinement it never tested"
)]

use pdf_render::{Rasterizer as _, TargetSpec};

/// The stages, in the order the parent runs them.
const STAGES: [&str; 4] = [
    "warm",
    "descriptors",
    "confine-then-draw",
    "draw-then-confine",
];

fn main() {
    match std::env::args().nth(1) {
        None => parent(),
        Some(stage) => child(&stage),
    }
}

/// Runs each stage as a child and reports how it ended.
fn parent() {
    let program = std::env::current_exe().expect("this program's own path");
    println!("# stage\toutcome");
    for stage in STAGES {
        let status = std::process::Command::new(&program)
            .arg(stage)
            .status()
            .expect("the child starts");
        let outcome = match status.code() {
            Some(0) => "drew".to_owned(),
            // The `descriptors` stage reports Landlock's level this way; see the stage.
            Some(40) => "confined, landlock enforced".to_owned(),
            Some(41) => "confined, landlock partial".to_owned(),
            Some(42) => "confined, landlock unavailable".to_owned(),
            _ => describe(status),
        };
        println!("{stage}\t{outcome}");
    }
}

/// How a child ended, naming the signal where there was one.
#[cfg(unix)]
fn describe(status: std::process::ExitStatus) -> String {
    use std::os::unix::process::ExitStatusExt as _;
    status.signal().map_or_else(
        || format!("exited with status {}", status.code().unwrap_or(-1)),
        |signal| {
            let name = if signal == libc_sigsys() {
                " (SIGSYS — the seccomp filter)"
            } else {
                ""
            };
            format!("killed by signal {signal}{name}")
        },
    )
}

#[cfg(not(unix))]
fn describe(status: std::process::ExitStatus) -> String {
    format!("exited with status {}", status.code().unwrap_or(-1))
}

/// `SIGSYS`, stated here so that this example needs no `libc` dependency of its own.
///
/// 31 on x86-64, aarch64 and every other Linux architecture but MIPS and Alpha, neither of
/// which this workspace builds for.
#[cfg(unix)]
fn libc_sigsys() -> i32 {
    31
}

/// One stage, in a process of its own.
fn child(stage: &str) {
    let list = page();
    let target = TargetSpec::for_page(&list, 1.0, 1 << 28).expect("a target");
    let mut backend = render_quorra::QuorraRasterizer::new_headless().expect("an adapter");

    let confine = || {
        let reached = pdf_sandbox::lockdown::apply_for(pdf_sandbox::lockdown::Profile::Interpreter)
            .expect("the confinement installs");
        assert!(reached.is_enforced(), "the filter is not in force");
    };

    match stage {
        "warm" => {
            backend.rasterize(&list, target).expect("a first frame");
            backend.rasterize(&list, target).expect("a second frame");
        }
        // What the confinement's *other* two layers make of a process holding a device.
        //
        // Everything is printed *before* the filter and the answer comes back as an exit code,
        // because a confined process cannot report: `RLIMIT_FSIZE` is zero, so a `println!`
        // whose standard output is a file is `SIGXFSZ` rather than a sentence (trap 18, ADR
        // 0597). The first version of this stage was killed by signal 25 for exactly that.
        "descriptors" => {
            backend.rasterize(&list, target).expect("a first frame");
            let held = std::fs::read_dir("/proc/self/fd").map_or(0, Iterator::count);
            println!("#   {held} descriptors held, against the confinement's ceiling of 8");
            let reached =
                pdf_sandbox::lockdown::apply_for(pdf_sandbox::lockdown::Profile::Interpreter)
                    .expect("the confinement installs");
            std::process::exit(match reached.landlock {
                pdf_sandbox::lockdown::LandlockLevel::Enforced => 40,
                pdf_sandbox::lockdown::LandlockLevel::Partial => 41,
                pdf_sandbox::lockdown::LandlockLevel::Unavailable => 42,
            });
        }
        "confine-then-draw" => {
            confine();
            backend.rasterize(&list, target).expect("a frame");
        }
        "draw-then-confine" => {
            backend.rasterize(&list, target).expect("a first frame");
            confine();
            backend.rasterize(&list, target).expect("a second frame");
        }
        other => panic!("unknown stage {other}"),
    }
}

/// A real page, because a scene of two rectangles asks the driver for less than a page does.
fn page() -> pdf_render::DisplayList {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/ISO_32000-2_sponsored_EC3.pdf");
    let bytes = std::fs::read(&path).expect("the specification is in doc/");
    let document = pdf_syntax::Document::open(bytes).expect("opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(6).expect("that page exists");
    pdf_model::content::interpret(&document, &page).display_list
}
