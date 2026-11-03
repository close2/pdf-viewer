//! This machine's clock bands, put into a band file before the file is parsed (ADR 2662).
//!
//! A clock band is a claim about a machine and a counted figure is a claim about this program, so
//! the three clock gates' band files — `doc/checks/launch-path.toml`, `turn-path.toml` and
//! `toolkit-launch.toml` — hold the counted figures and the structure of each row, and every row
//! that is judged by a clock carries one line,
//!
//! ```toml
//! clock_bands = machine
//! ```
//!
//! where its clock bands go. The bands themselves live in `doc/checks/machines/<name>/`, one
//! directory per machine, each with a `fingerprint` file and one file per gate stating, row by row,
//! only the clock bands. Which directory is this machine's is decided by its [`fingerprint`] — the
//! processor's model, how many processors this process may run on, and the adapter the gate draws
//! on as its own bring-up names it — compared byte for byte with each directory's `fingerprint`.
//! A machine with no directory is not judged against another machine's numbers: the gate fails and
//! names `tools/bands.sh retake`, which takes the bands by ADR 1969's rule and writes a directory of
//! the machine's own, beside every other machine's (ADR 2662 says why a factor of a baseline probe
//! does not do this).
//!
//! **Why the merge is textual.** Each gate's parser already refuses a key it does not know and a row
//! that lacks one, so the machine's lines are put where the sentinel stands and the gate's own
//! parser judges the result; the merge itself checks only that every row of the machine file has a
//! row to go to and every sentinel has lines to receive.
//!
//! Included by `#[path]` from the three harnesses: `crates/viewer-ui/tests/launch_path.rs`,
//! `crates/render-raster/tests/turn_path.rs` and `crates/viewer-host/tests/toolkit_launch/mod.rs`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The line in a band file whose place the machine's clock bands take.
pub(crate) const SENTINEL: &str = "clock_bands = machine";

/// Set to `1` by `tools/bands.sh retake`: the clock bands are unbounded, so that every clock figure
/// is printed and none is judged, and the gate's output is what the retake reads.
pub(crate) const RETAKE: &str = "PDFVIEWER_BANDS_RETAKE";

/// The upper edge every band is given while bands are re-taken: far past any figure a launch, a
/// turn or a window takes, in milliseconds.
const UNBOUNDED_MS: f64 = 1_000_000.0;

/// A band file with a machine's clock bands put in, and the sentence that says whose they are.
pub(crate) struct MachineBands {
    /// The band file's text with every sentinel replaced, ready for the gate's own parser.
    pub(crate) text: String,
    /// Whose bands these are, for the gate's first lines.
    pub(crate) said: String,
}

/// This machine's fingerprint: the processor's model, how many processors this process may run
/// on, and `adapter` — the description of the adapter the gate draws on, as the gate's own
/// bring-up names it (name, type and backend).
///
/// **The adapter is the one wgpu chose, never the one the environment was meant to choose.** On the
/// server VM, `VK_ICD_FILENAMES` naming a file that is not there leaves the Vulkan loader with no
/// driver, and wgpu then draws through OpenGL on the same card; naming lavapipe's file alone still
/// draws through OpenGL on the card, because wgpu prefers it to a processor's Vulkan device. A
/// fingerprint read from `vulkaninfo` named a device the gate never drew on in both (ADR 2662).
pub(crate) fn fingerprint(adapter: &str) -> String {
    let processor = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|info| {
            info.lines()
                .find_map(|line| line.strip_prefix("model name"))
                .and_then(|rest| rest.split_once(':'))
                .map(|(_, model)| model.trim().to_owned())
        })
        .unwrap_or_else(|| std::env::consts::ARCH.to_owned());
    let processors = std::thread::available_parallelism().map_or(0, std::num::NonZero::get);
    format!("processor={processor}; processors={processors}; device={adapter}")
}

/// Every machine directory under `doc/checks/machines`, in name order.
pub(crate) fn machine_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root.join("doc/checks/machines"))
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.join("fingerprint").is_file())
                .collect()
        })
        .unwrap_or_default();
    dirs.sort();
    dirs
}

/// The directory whose `fingerprint` is `print`, if one is.
fn dir_of(root: &Path, print: &str) -> Option<PathBuf> {
    machine_dirs(root).into_iter().find(|dir| {
        std::fs::read_to_string(dir.join("fingerprint")).is_ok_and(|stated| stated.trim() == print)
    })
}

/// `doc/checks/<gate>.toml` with the clock bands of this machine — [`fingerprint`] of `adapter` —
/// put in, or, under [`RETAKE`], with every clock band unbounded.
///
/// # Errors
///
/// Where this machine has no directory and no retake is asked for (the sentence names the
/// fingerprint and the command), or where a file does not read or does not merge.
pub(crate) fn for_this_machine(
    root: &Path,
    gate: &str,
    adapter: &str,
) -> Result<MachineBands, String> {
    let print = fingerprint(adapter);
    let base = read(&root.join(format!("doc/checks/{gate}.toml")))?;
    if std::env::var(RETAKE).is_ok_and(|value| value == "1") {
        let Some(template) = machine_dirs(root).into_iter().next() else {
            return Err(
                "no machine directory under doc/checks/machines to take the rows' \
                        clock keys from"
                    .to_owned(),
            );
        };
        let machine = unbounded(&read(&template.join(format!("{gate}.toml")))?, &print);
        return Ok(MachineBands {
            text: merge(&base, &machine)?,
            said: format!(
                "no band is judged: the bands are being re-taken (`{RETAKE}=1`, tools/bands.sh \
                 retake)\nbands: fingerprint {print}"
            ),
        });
    }
    let Some(dir) = dir_of(root, &print) else {
        return Err(format!(
            "no clock bands for this machine — {print} — under doc/checks/machines; \
             `tools/bands.sh retake` takes them by ADR 1969's rule (ADR 2662)"
        ));
    };
    for_dir(root, gate, &dir).map(|text| MachineBands {
        text,
        said: format!("{}\nbands: fingerprint {print}", dir.display()),
    })
}

/// `doc/checks/<gate>.toml` with the clock bands of the machine directory `dir` put in.
///
/// # Errors
///
/// Where a file does not read or the two do not merge.
pub(crate) fn for_dir(root: &Path, gate: &str, dir: &Path) -> Result<String, String> {
    let base = read(&root.join(format!("doc/checks/{gate}.toml")))?;
    let machine = read(&dir.join(format!("{gate}.toml")))?;
    merge(&base, &machine).map_err(|complaint| format!("{}: {complaint}", dir.display()))
}

fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
}

/// The keys whose values say which row a line belongs to.
const IDENTITY: [&str; 3] = ["path", "page", "name"];

/// A line's key, where it is a `key = value` line.
fn key_of(line: &str) -> Option<&str> {
    let line = line.trim();
    if line.starts_with('#') {
        return None;
    }
    line.split_once('=').map(|(key, _)| key.trim())
}

/// The machine file's band lines, by the row they belong to: the row's header and its identity
/// lines joined, the empty string for the lines above every header. The top section's `fingerprint`
/// is the directory's and not a band, so it is left out.
fn sections(machine: &str) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut sections: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut identity = String::new();
    for (index, line) in machine.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with("[[") {
            trimmed.clone_into(&mut identity);
            continue;
        }
        let Some(key) = key_of(trimmed) else {
            return Err(format!(
                "line {} is neither a key nor a header",
                index.saturating_add(1)
            ));
        };
        if IDENTITY.contains(&key) {
            identity = format!("{identity} {trimmed}");
            continue;
        }
        if identity.is_empty() && key == "fingerprint" {
            continue;
        }
        sections
            .entry(identity.clone())
            .or_default()
            .push(trimmed.to_owned());
    }
    Ok(sections)
}

/// `base` with every sentinel replaced by the lines `machine` states for the same row.
///
/// # Errors
///
/// Where a sentinel's row has no lines in `machine`, or `machine` states lines for a row `base`
/// has no sentinel in.
pub(crate) fn merge(base: &str, machine: &str) -> Result<String, String> {
    let mut sections = sections(machine)?;
    let mut merged = String::with_capacity(base.len().saturating_add(machine.len()));
    let mut identity = String::new();
    for (index, line) in base.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("[[") {
            trimmed.clone_into(&mut identity);
        } else if key_of(trimmed).is_some_and(|key| IDENTITY.contains(&key)) {
            identity = format!("{identity} {trimmed}");
        }
        if trimmed == SENTINEL {
            let Some(lines) = sections.remove(&identity) else {
                return Err(format!(
                    "line {} of the band file asks for the clock bands of `{identity}`, and the \
                     machine's file states none",
                    index.saturating_add(1)
                ));
            };
            for band in lines {
                merged.push_str(&band);
                merged.push('\n');
            }
            continue;
        }
        merged.push_str(line);
        merged.push('\n');
    }
    if let Some(row) = sections.keys().next() {
        return Err(format!(
            "the machine's file states clock bands for `{row}`, which no row of the band file \
             asks for"
        ));
    }
    Ok(merged)
}

/// A machine file with every band unbounded, and the split by the device's clock level (ADR 2402)
/// left out, since a retake has no band at either level yet: what a retake judges against.
fn unbounded(machine: &str, print: &str) -> String {
    let mut out = String::new();
    for line in machine.lines() {
        let trimmed = line.trim();
        let Some(key) = key_of(trimmed) else {
            out.push_str(line);
            out.push('\n');
            continue;
        };
        let value = trimmed
            .split_once('=')
            .map_or("", |(_, value)| value.trim());
        if key.ends_with("_lowest_ms") || key.ends_with("_raised_mhz") {
            continue;
        }
        if key == "machine" {
            // Writing to a `String` cannot fail.
            let _ = writeln!(out, "machine = \"re-taken: {print}\"");
        } else if value.contains("..") {
            let _ = writeln!(out, "{key} = 0 .. {UNBOUNDED_MS}");
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}
