//! What a script run costs the address space, measured where the script worker's ceiling is set
//! from (ADR 1609).
//!
//! ```text
//! cargo run --release -p pdf-script-worker --features engine --example script_peak
//! ```
//!
//! Prints this process's `VmSize` before any engine exists, then for each script, each in a fresh
//! process, the address space the run reached (`VmPeak`) beside the size it began at, how long it
//! took and how it ended. Unconfined, because a confined process has no `/proc` to read: the
//! figures are the worker's own, since it links the same engine and runs the same function.

#![expect(clippy::print_stdout, reason = "an example prints what it measured")]

use pdf_model::aform::Trigger;
use pdf_model::view::{ScriptEvent, ScriptSite};
use pdf_script::{Budget, Request};

/// This process's `VmSize` and `VmPeak`, in mebibytes.
fn address_space() -> (f64, f64) {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let read = |key: &str| {
        status
            .lines()
            .find(|line| line.starts_with(key))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|value| value.parse::<f64>().ok())
            .map_or(0.0, |kilobytes| kilobytes / 1024.0)
    };
    (read("VmSize:"), read("VmPeak:"))
}

/// The scripts measured, each with what it is: the first is the cheapest a run can be, the next
/// six are one call at a per-call ceiling of `pdf_script::Budget::FIELD_EVENT`, and the last two
/// are what no per-call budget sees.
const SCRIPTS: &[(&str, &str)] = &[
    ("one statement", "event.value = '<' + event.value + '>';"),
    (
        "repeat at the string budget",
        "var s = 'x'.repeat(16777216); event.value = String(s.length);",
    ),
    (
        "repeat at the loop budget",
        "var s = 'xxxxxxxxxxxxxxxx'.repeat(100000); event.value = String(s.length);",
    ),
    (
        "padStart at the string budget",
        "var s = 'x'.padStart(16777216, 'é'); event.value = String(s.length);",
    ),
    (
        "an array at the element budget",
        "var a = new Array(1048576).fill(1.5); event.value = String(a.length);",
    ),
    (
        "join at the element budget",
        "var a = new Array(1048576).fill(1); event.value = String(a.join(',').length);",
    ),
    (
        "a buffer at the byte budget",
        "var b = new ArrayBuffer(16777216); event.value = String(b.byteLength);",
    ),
    (
        "a typed array's join at the byte budget",
        "var t = new Int8Array(16777216); event.value = String(t.join(',').length);",
    ),
    (
        "a string grown by an operator",
        "var s = 'x'; for (var i = 0; i < 26; i++) { s += s; } event.value = String(s.length);",
    ),
];

/// Runs the script at `index` and prints what it cost.
fn measure(index: usize) {
    let Some((what, script)) = SCRIPTS.get(index) else {
        return;
    };
    let (before, _) = address_space();
    let event = ScriptEvent {
        site: ScriptSite::Field(Trigger::Format),
        field: "Total",
        label: "",
        script,
        value: "12",
        change: "",
        selection: (0, 0),
        will_commit: false,
        source: "",
        fields: &[],
        page: 0,
        pages: 1,
        commit_key: None,
        field_full: false,
        change_ex: "",
        dirty: false,
        document: None,
    };
    let request = Request::of(&event, 0, 0);
    let started = std::time::Instant::now();
    let outcome = pdf_script::run(&request, &Budget::FIELD_EVENT);
    let spent = started.elapsed();
    let (_, peak) = address_space();
    println!(
        "script_peak: {what}: VmSize before {before:.1} MiB, VmPeak {peak:.1} MiB (+{:.1}), \
         {:.1} ms, ended {:?}, value {:?}",
        peak - before,
        spent.as_secs_f64() * 1000.0,
        outcome.ending,
        outcome.value
    );
}

fn main() {
    // Each script in a process of its own, because `VmPeak` only rises: a figure read after a
    // second script in the same process would be the first's.
    if let Some(index) = std::env::args().nth(1).and_then(|index| index.parse().ok()) {
        measure(index);
        return;
    }
    let (size, _) = address_space();
    println!(
        "script_peak: a process that links the engine, before any is constructed: VmSize {size:.1} MiB"
    );
    let Ok(me) = std::env::current_exe() else {
        return;
    };
    for index in 0..SCRIPTS.len() {
        let Ok(output) = std::process::Command::new(&me)
            .arg(index.to_string())
            .output()
        else {
            return;
        };
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
}
