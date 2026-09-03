//! What a confined viewer's address space peaks at, against the ceiling it was given.
//!
//! ```sh
//! cargo run --release -p viewer-confined --example confined_peak -- file.pdf [more.pdf …]
//! CONFINED_PEAK_ROUTE=whole cargo run --release -p viewer-confined --example confined_peak -- file.pdf
//! ```
//!
//! **Each document is opened on disk and crosses as its descriptor** (ADR 0812), which is the
//! route `pdf-viewer-confined` takes; `CONFINED_PEAK_ROUTE=whole` reads it whole in this process
//! and sends the bytes, which is the route every confined host took before that ADR and the
//! column to measure against. Beside the worker's peak this prints **this process's own**
//! (`VmHWM`, the resident high-water mark), because on the whole route the host is where the
//! document's bytes are held and a measurement of the worker alone would miss the larger half.
//!
//! `doc/todo/15`'s instrument, and it is `VmPeak` rather than resident size on purpose: `VmPeak`
//! is the high-water mark of the *address space*, which is the counter `RLIMIT_AS` is compared
//! against. A worker whose resident set never leaves a few megabytes can still be killed for
//! reserving four gibibytes, and a measurement of the wrong counter would say the ceiling was
//! nowhere near.
//!
//! The number is read off `/proc/<worker>/status` after the work rather than sampled during it,
//! because the kernel maintains it as a high-water mark and it survives the memory being freed.
//! The worker is found by its parent process identifier, so a neighbouring round's worker cannot
//! be measured by mistake.
//!
//! **Load-immune.** Unlike a wall clock, a peak does not move when the machine is busy, which is
//! what makes this the figure `doc/todo/15` quotes.

#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    reason = "an example whose whole output is what it printed; a run that cannot do the thing \
              should stop loudly rather than print a number about something else"
)]

use viewer_confined::{Confined, ConfinedError};
use viewer_core::{Command, DocumentId, Event};

/// The confined worker: the child of this process running the worker program.
///
/// Found by walking `/proc` rather than by asking [`Confined`], because a process identifier is
/// not something a host needs and this is an instrument. Parallel rounds run workers of their own,
/// so the parent identifier is what makes the answer this one's.
fn worker_pid() -> Option<u32> {
    let me = std::process::id();
    for entry in std::fs::read_dir("/proc").ok()?.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|text| text.parse::<u32>().ok()) else {
            continue;
        };
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
            continue;
        };
        // The second field is the executable's name in parentheses and may itself contain
        // spaces, so the fields after it are found from the last `)` rather than by splitting
        // the whole line.
        let Some(after) = stat.rsplit_once(')') else {
            continue;
        };
        let parent = after
            .1
            .split_whitespace()
            .nth(1)
            .and_then(|text| text.parse::<u32>().ok());
        if parent == Some(me) {
            return Some(pid);
        }
    }
    None
}

/// One `Vm…` line of `/proc/<pid>/status`, in kilobytes as the kernel writes it.
fn vm_kilobytes(pid: u32, field: &str) -> Option<u64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let line = status.lines().find(|line| line.starts_with(field))?;
    line.split_whitespace()
        .nth(1)
        .and_then(|value| value.parse().ok())
}

/// Prints the worker's address-space peak and what fraction of the ceiling it is, and this
/// process's own resident high-water mark beside it.
fn report(label: &str, pid: u32, ceiling: u64) {
    let peak = vm_kilobytes(pid, "VmPeak:");
    let size = vm_kilobytes(pid, "VmSize:");
    let host = vm_kilobytes(std::process::id(), "VmHWM:").unwrap_or(0);
    let worker_resident = vm_kilobytes(pid, "VmHWM:").unwrap_or(0);
    println!("  {label}: host VmHWM {host} KB, worker VmHWM {worker_resident} KB");
    match (peak, size) {
        (Some(peak), Some(size)) => {
            let ceiling_kb = ceiling / 1024;
            let share = if ceiling_kb == 0 {
                0.0
            } else {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a percentage of a four-gibibyte ceiling, printed to one decimal"
                )]
                {
                    peak as f64 * 100.0 / ceiling_kb as f64
                }
            };
            println!("  {label}: VmPeak {peak} KB ({share:.1}% of the ceiling), VmSize {size} KB");
        }
        _ => println!("  {label}: the worker is gone, so /proc has no peak to read"),
    }
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: confined_peak <file.pdf> [more.pdf …]");
        std::process::exit(2);
    }

    let mut confined = Confined::start().expect("a confined viewer starts");
    let confinement = confined.confinement();
    let ceiling = confinement.address_space_limit;
    println!(
        "ceiling {} MiB, landlock {:?}, system calls {:?}",
        ceiling / (1 << 20),
        confinement.landlock,
        confinement.system_calls
    );
    let pid = worker_pid().expect("the worker is a child of this process");
    report("started", pid, ceiling);

    let whole = std::env::var("CONFINED_PEAK_ROUTE").is_ok_and(|route| route == "whole");
    for (index, path) in paths.iter().enumerate() {
        let bytes = if whole {
            pdf_syntax::FileBytes::from(
                pdf_syntax::read_file(std::path::Path::new(path))
                    .expect("the document is readable"),
            )
        } else {
            pdf_syntax::FileBytes::on_disk(std::path::Path::new(path)).expect("the document opens")
        };
        println!(
            "{path}: {} bytes of file, {}",
            bytes.len(),
            if whole {
                "read whole here and sent as bytes"
            } else {
                "open on disk and sent as a descriptor"
            }
        );
        let id = DocumentId(u64::try_from(index).unwrap_or(0).saturating_add(1));
        match confined.handle(&Command::Open {
            id,
            bytes,
            password: None,
            fragment: None,
        }) {
            Ok(events) => {
                for event in &events {
                    match event {
                        Event::Opened { pages, .. } => println!("  {pages} page(s)"),
                        Event::Reported { notes, .. } => {
                            for note in notes {
                                println!("  reported: {note}");
                            }
                        }
                        Event::OpenFailed { reason, .. } => println!("  open failed: {reason}"),
                        _ => {}
                    }
                }
            }
            // The whole point of the item this instrument serves: what a host is told when the
            // ceiling is reached. Printed rather than unwrapped, because the interesting runs are
            // the ones that do not succeed.
            Err(error) => {
                println!("  the confined viewer answered: {error}");
                if matches!(error, ConfinedError::WorkerDied { .. }) {
                    println!("  (the worker is gone; nothing more can be asked of it)");
                    return;
                }
            }
        }
        report("after this document", pid, ceiling);
    }
}
