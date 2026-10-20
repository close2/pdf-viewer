//! The toolkit windows' launch, every figure held to a band: what `viewer-ui`'s
//! `tests/launch_path.rs` is for `quorra`, for `quorra-gtk` and `quorra-qt`.
//!
//! # Why this exists
//!
//! `CLAUDE.md` principle 2 makes time-to-first-page a gate and cold graphics bring-up "its own
//! gate, separate from time-to-first-page, so that a regression in the driver, the adapter
//! selection or the shader set is legible as itself rather than as a slower page". The toolkit
//! windows draw page one on the device as `quorra` does (ADRs 2070, 2071), and until this harness
//! their launch was a table one round took by hand: ADR 2071 calls its own figures "a round's
//! measurement, not a band". This is the band (ADR 2125).
//!
//! **It is not a module of `viewer-host`'s library, and not a test of its own.** It is one file
//! that each toolkit crate's `tests/launch_path.rs` includes by path, because the binary it
//! launches is that crate's (`CARGO_BIN_EXE_<name>` exists only inside the package that owns the
//! binary), and because a harness compiled into the library would ship in every window for the
//! sake of a test. `tests/` holds it in a directory so that Cargo does not take it for a target.
//!
//! # What each figure is
//!
//! Every figure is read off the window's own `--trace=launch,frames`, in milliseconds since the
//! window's `main` began, for each of the two lanes: the device, which is the default, and
//! `--cpu`, which brings no device up.
//!
//! | figure | the trace line it is | what moves it |
//! |---|---|---|
//! | `window_ms` | the toolkit's first size for the page area (`first resize`, `first allocation`) | the toolkit's own start-up, and — on the device lane — whatever the device's thread costs it beside it |
//! | `up_ms` | `graphics device up in` | the cold bring-up: the Vulkan loader, the driver, the adapter (device lane only) |
//! | `page_one_ms` | `page 1 rasterised … in` | page one's draw on its lane, the device's read back included |
//! | `first_frame_ms` | `first frame on the screen at` | all of the above, and the toolkit's paint |
//!
//! # How a figure is taken
//!
//! - **On a display of its own**: an `Xvfb` this harness starts on a number whose lock file was
//!   absent, and only once that lock names its own server's pid (trap 181). No `Xvfb` is a failure
//!   with a sentence, never a skip.
//! - **The minimum of [`SAMPLES`] fresh processes per lane, the lanes interleaved**, each killed at
//!   its first frame: contention adds time and never removes it, and interleaving puts the
//!   machine's drift into both lanes alike rather than into their difference.
//! - **Pinned to the machine's fastest cores** where it has two classes of them, with rayon given
//!   that many threads — `launch_path`'s derivation and its reason (`doc/environment.md`).
//! - **Judged only on a machine at or under one runnable task per physical core**, measured before
//!   and after the run; above it every figure is printed and none judged, as `launch_path` does.
//!
//! # Running it
//!
//! ```text
//! ulimit -u 8192; tools/bounded.sh --lock --clock --round <session> --tree 12 -- \
//!     cargo test --release -p viewer-qt --test launch_path -- --ignored --nocapture
//! ```
//!
//! and the same with `viewer-gtk`. `TOOLKIT_LAUNCH_BINARY=<path>` measures another build of the
//! same window against the same bands — HEAD's export beside a change, in one hold (habit 101) —
//! and the run prints which binary it measured.

use std::io::{BufRead as _, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::time::{Duration, Instant};

/// Fresh processes per lane, and the figure is their minimum — `launch_path`'s count.
const SAMPLES: usize = 9;

/// How long one launch may take to put its first frame on the screen before it is a failure.
const PATIENCE: Duration = Duration::from_secs(20);

/// The variable naming another build of the window to measure in place of this crate's own.
const BINARY: &str = "TOOLKIT_LAUNCH_BINARY";

/// The document every launch opens: the drive's small page, the one ADR 2071 measured on.
const DOCUMENT: &str = "doc/PDF20_AN001-BPC.pdf";

/// One toolkit window, as its crate's test names it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Window {
    /// The program's name, which is its row's `name` in the band file.
    pub(crate) name: &'static str,
    /// The program Cargo built for this test (`env!("CARGO_BIN_EXE_<name>")`).
    pub(crate) binary: &'static str,
    /// The trace's words for the toolkit's first size of the page area.
    pub(crate) sized: &'static str,
}

/// Which lane draws page one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lane {
    /// The graphics device, read back — the default (ADR 2071).
    Device,
    /// `--cpu`: the processor, with no device brought up.
    Processor,
}

impl Lane {
    /// Both, in the order a round interleaves them.
    const BOTH: [Self; 2] = [Self::Device, Self::Processor];

    /// The band file's prefix for this lane's figures.
    fn name(self) -> &'static str {
        match self {
            Self::Device => "device",
            Self::Processor => "processor",
        }
    }

    /// The words the window is given for this lane.
    fn words(self) -> &'static [&'static str] {
        match self {
            Self::Device => &[],
            Self::Processor => &["--cpu"],
        }
    }
}

/// One launch's figures, in milliseconds; `None` for a line the trace did not print.
#[derive(Debug, Clone, Copy, Default)]
struct Figures {
    window: Option<f64>,
    up: Option<f64>,
    page_one: Option<f64>,
    first_frame: Option<f64>,
}

impl Figures {
    /// The figures by their band-file names, in the order a table prints them.
    fn named(&self) -> [(&'static str, Option<f64>); 4] {
        [
            ("window_ms", self.window),
            ("up_ms", self.up),
            ("page_one_ms", self.page_one),
            ("first_frame_ms", self.first_frame),
        ]
    }

    /// Takes in one line of the window's trace.
    fn read(&mut self, window: &Window, line: &str) {
        let Some((at, said)) = line
            .strip_prefix("trace:")
            .and_then(|rest| rest.trim_start().split_once(' '))
        else {
            return;
        };
        let Ok(at) = at.parse::<f64>() else {
            return;
        };
        if self.window.is_none() && said.starts_with(window.sized) {
            self.window = Some(at * 1000.0);
        } else if let Some(took) = said.strip_prefix("graphics device up in ") {
            self.up = self.up.or_else(|| milliseconds(took.split(':').next()?));
        } else if let Some(drawn) = said.strip_prefix("page 1 rasterised ") {
            let took = drawn.split_once(" in ").map(|(_, took)| took);
            self.page_one = self
                .page_one
                .or_else(|| milliseconds(took?.split(',').next()?));
        } else if let Some(at) = said.strip_prefix("first frame on the screen at ") {
            self.first_frame = self.first_frame.or_else(|| milliseconds(at));
        }
    }
}

/// A `Duration`'s `Debug` text — `10.689375ms`, `906.504µs`, `1.2s` — in milliseconds.
fn milliseconds(text: &str) -> Option<f64> {
    let text = text.trim();
    let (number, scale) = if let Some(n) = text.strip_suffix("ms") {
        (n, 1.0)
    } else if let Some(n) = text.strip_suffix("µs") {
        (n, 1e-3)
    } else if let Some(n) = text.strip_suffix("ns") {
        (n, 1e-6)
    } else {
        (text.strip_suffix('s')?, 1e3)
    };
    number.parse::<f64>().ok().map(|n| n * scale)
}

/// An `Xvfb` this harness started, on a display nobody else holds, stopped when dropped.
#[derive(Debug)]
struct Display {
    server: Child,
    name: String,
}

impl Display {
    /// Starts a server on the first display number from 160 whose lock file is absent, and keeps
    /// it only once the lock names this server's own pid (trap 181).
    fn start() -> Result<Self, String> {
        for number in 160..260_u32 {
            let lock = PathBuf::from(format!("/tmp/.X{number}-lock"));
            if lock.exists() {
                continue;
            }
            let name = format!(":{number}");
            let server = Command::new("Xvfb")
                .args([
                    name.as_str(),
                    "-screen",
                    "0",
                    "1400x1100x24",
                    "-nolisten",
                    "tcp",
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|error| {
                    format!("Xvfb would not start ({error}); this gate needs it installed")
                })?;
            let mut display = Self { server, name };
            let began = Instant::now();
            while began.elapsed() < Duration::from_secs(5) {
                if let Ok(held) = std::fs::read_to_string(&lock) {
                    if held.trim().parse::<u32>().ok() == Some(display.server.id()) {
                        return Ok(display);
                    }
                    // Another server took the number between the look and the start.
                    if !held.trim().is_empty() {
                        break;
                    }
                }
                if display.server.try_wait().ok().flatten().is_some() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            display.stop();
        }
        Err("no display from :160 to :259 could be taken".to_owned())
    }

    /// Stops the server; a server that has already gone has nothing left to stop.
    fn stop(&mut self) {
        drop(self.server.kill());
        drop(self.server.wait());
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The CPUs this machine runs fastest on, as `taskset -c` spells them, where it has two classes
/// of them and `taskset` is installed — `launch_path`'s `the_performance_cores`, for its reason.
fn the_performance_cores() -> Option<String> {
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
    Command::new("taskset")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let cores: Vec<String> = speeds
        .iter()
        .filter(|&&(_, speed)| speed == fastest)
        .map(|&(cpu, _)| cpu.to_string())
        .collect();
    Some(cores.join(","))
}

/// One runnable task per physical core — `launch_path`'s load ceiling, for its reason.
fn the_load_ceiling() -> f64 {
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

/// The one-minute load average, or infinity where the kernel does not say — which declines.
fn load() -> f64 {
    std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|text| text.split_whitespace().next()?.parse().ok())
        .unwrap_or(f64::INFINITY)
}

/// One fresh launch of `binary` on `lane`, killed at its first frame.
fn launch(
    window: &Window,
    binary: &Path,
    lane: Lane,
    display: &Display,
    cores: Option<&str>,
    document: &Path,
) -> Result<Figures, String> {
    let mut command = if let Some(cores) = cores {
        let mut taskset = Command::new("taskset");
        taskset.arg("-c").arg(cores).arg(binary);
        taskset.env("RAYON_NUM_THREADS", cores.split(',').count().to_string());
        taskset
    } else {
        let mut command = Command::new(binary);
        command.env_remove("RAYON_NUM_THREADS");
        command
    };
    let mut child = command
        .args(lane.words())
        .arg("--trace=launch,frames")
        .arg(document)
        .env("DISPLAY", &display.name)
        .env_remove("WAYLAND_DISPLAY")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("{} would not start: {error}", binary.display()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "the window's output was not piped".to_owned())?;
    // A thread reads the pipe so that a window that never draws is a timeout here rather than a
    // read that blocks for ever.
    let (sender, arrived) = channel();
    let reader = std::thread::spawn(move || {
        for said in BufReader::new(stdout).lines().map_while(Result::ok) {
            if sender.send(said).is_err() {
                return;
            }
        }
    });
    let began = Instant::now();
    let mut figures = Figures::default();
    let outcome = loop {
        let left = PATIENCE.saturating_sub(began.elapsed());
        match arrived.recv_timeout(left) {
            Ok(said) => {
                figures.read(window, &said);
                if figures.first_frame.is_some() {
                    break Ok(figures);
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                break Err(format!(
                    "{} {:?}: no first frame in {PATIENCE:?}",
                    window.name, lane
                ));
            }
            Err(RecvTimeoutError::Disconnected) => {
                break Err(format!(
                    "{} {:?}: the window ended before its first frame",
                    window.name, lane
                ));
            }
        }
    };
    drop(child.kill());
    drop(child.wait());
    drop(arrived);
    drop(reader.join());
    outcome
}

/// A figure's name in the band file, and its band's two edges in milliseconds.
type Band = (String, f64, f64);

/// One `[[window]]` row of `doc/checks/toolkit-launch.toml`: each figure's band by its name.
fn bands(file: &str, name: &str) -> Result<Vec<Band>, String> {
    let mut rows: Vec<(Option<String>, Vec<Band>)> = Vec::new();
    for (at, line) in file.lines().enumerate() {
        let line = line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        if line == "[[window]]" {
            rows.push((None, Vec::new()));
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!(
                "line {}: `{line}` is not `key = value`",
                at.saturating_add(1)
            ));
        };
        let Some(row) = rows.last_mut() else {
            return Err(format!(
                "line {}: a value before any [[window]]",
                at.saturating_add(1)
            ));
        };
        let (key, value) = (key.trim(), value.trim());
        if key == "name" {
            row.0 = Some(value.trim_matches('"').to_owned());
            continue;
        }
        let band = value.split_once("..").and_then(|(low, high)| {
            Some((
                low.trim().parse::<f64>().ok()?,
                high.trim().parse::<f64>().ok()?,
            ))
        });
        let Some((low, high)) = band.filter(|(low, high)| low <= high) else {
            return Err(format!(
                "line {}: `{value}` is not `low .. high`",
                at.saturating_add(1)
            ));
        };
        row.1.push((key.to_owned(), low, high));
    }
    rows.into_iter()
        .find(|(named, _)| named.as_deref() == Some(name))
        .map(|(_, bands)| bands)
        .ok_or_else(|| format!("doc/checks/toolkit-launch.toml has no [[window]] named {name}"))
}

/// Launches `window` on both lanes and prints every sample and each minimum beside its band.
///
/// # Errors
///
/// Where a judged minimum is outside its band, where a figure and a band do not match one to one,
/// or where the display, the band file or a launch could not be had — each in a sentence.
/// Fails the gate if this build cannot reach the sandboxed image decoder.
///
/// The windows this harness launches decode `CCITTFaxDecode`, `JBIG2Decode` and `JPXDecode` images
/// through a separate program that Cargo does not build when it tests this crate (trap 10). A launch
/// that cannot decode an image is a *faster* launch, so without this the figures would improve
/// silently on a build that draws less (trap 16, ADR 0557), as `viewer-ui`'s launch gate says.
pub(crate) fn require_the_sandbox() -> Result<(), String> {
    pdf_model::image::sandboxed_decoder().map_err(|error| {
        format!(
            "the sandboxed image decoder is not available, so the figures would be wrong: {error}"
        )
    })
}

pub(crate) fn hold(window: &Window) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary =
        std::env::var_os(BINARY).map_or_else(|| PathBuf::from(window.binary), PathBuf::from);
    let document = root.join(DOCUMENT);
    let file = std::fs::read_to_string(root.join("doc/checks/toolkit-launch.toml"))
        .map_err(|error| format!("doc/checks/toolkit-launch.toml: {error}"))?;
    let bands = bands(&file, window.name)?;
    let display = Display::start()?;
    let cores = the_performance_cores();
    let ceiling = the_load_ceiling();
    println!(
        "toolkit launch: {} ({}), {} on {}, pinned to {}, load ceiling {ceiling}",
        window.name,
        binary.display(),
        DOCUMENT,
        display.name,
        cores
            .as_deref()
            .unwrap_or("nothing — one class of core, or no taskset"),
    );
    let before = load();
    let mut samples: Vec<(Lane, Figures)> = Vec::new();
    for _ in 0..SAMPLES {
        for lane in Lane::BOTH {
            let figures = launch(window, &binary, lane, &display, cores.as_deref(), &document)?;
            samples.push((lane, figures));
        }
    }
    let after = load();
    let judged = before <= ceiling && after <= ceiling;
    println!(
        "toolkit launch: load {before:.2} before and {after:.2} after, {}",
        if judged {
            "judged"
        } else {
            "above the ceiling, so printed and NOT JUDGED"
        }
    );
    let mut failures: Vec<String> = Vec::new();
    let mut measured: Vec<String> = Vec::new();
    for lane in Lane::BOTH {
        let mine: Vec<Figures> = samples
            .iter()
            .filter(|(of, _)| *of == lane)
            .map(|(_, figures)| *figures)
            .collect();
        for (at, (figure, _)) in Figures::default().named().into_iter().enumerate() {
            let values: Vec<f64> = mine.iter().filter_map(|f| f.named()[at].1).collect();
            let key = format!("{}_{figure}", lane.name());
            if values.is_empty() {
                // `up_ms` has no meaning on the processor's lane; anywhere else a figure the
                // trace never printed is a window that has stopped saying it.
                if lane == Lane::Processor && figure == "up_ms" {
                    continue;
                }
                failures.push(format!("{key}: not one launch printed it"));
                continue;
            }
            let least = values.iter().copied().fold(f64::INFINITY, f64::min);
            let spelled: Vec<String> = values.iter().map(|v| format!("{v:.1}")).collect();
            println!("toolkit launch: {key} {least:.1}, of {}", spelled.join(" "));
            measured.push(key.clone());
            let Some(&(_, low, high)) = bands.iter().find(|(named, _, _)| *named == key) else {
                failures.push(format!("{key}: no band in the window's row"));
                continue;
            };
            gate_ratchet::band(&format!("{} {key}", window.name), least, low, high);
            if judged && !(low..=high).contains(&least) {
                failures.push(format!("{key}: {least:.1} outside {low} .. {high}"));
            }
        }
    }
    for (named, _, _) in &bands {
        if !measured.contains(named) {
            failures.push(format!("{named}: a band with no figure"));
        }
    }
    drop(display);
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("{}'s launch: {}", window.name, failures.join("; ")))
    }
}

/// The band file as it is committed: both windows' rows, every band a range.
#[test]
fn both_windows_rows_are_read_whole() {
    let file = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/checks/toolkit-launch.toml"),
    )
    .expect("the band file is committed");
    for name in ["quorra-gtk", "quorra-qt"] {
        let row = bands(&file, name).expect("the window has a row");
        let keys: Vec<&str> = row.iter().map(|(key, _, _)| key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "device_window_ms",
                "device_up_ms",
                "device_page_one_ms",
                "device_first_frame_ms",
                "processor_window_ms",
                "processor_page_one_ms",
                "processor_first_frame_ms"
            ],
            "{name}: one band for every figure the harness measures, and no other"
        );
    }
    assert!(
        bands(&file, "quorra").is_err(),
        "a window with no row is said"
    );
}

/// A device-lane trace of `quorra-qt`, as it printed on 2026-10-20, read into its four figures.
#[test]
fn a_trace_is_read_into_its_figures() {
    let window = Window {
        name: "quorra-qt",
        binary: "",
        sized: "first resize",
    };
    let mut figures = Figures::default();
    for line in [
        "trace:     0.006 host ready, handing Qt the loop",
        "trace:     0.106 first resize 619x997 device px, scale 1",
        "trace:     0.118 graphics device up in 65.53477ms: AMD Radeon 890M Graphics (RADV \
         STRIX1) (IntegratedGpu, Vulkan)",
        "trace:     0.118 page 1 rasterised 619x876 in 10.689375ms, waited 10.716836ms",
        "trace:     0.123 page 2 rasterised 619x876 in 4.939726ms, waited 4.952821ms",
        "trace:     0.126 4337952 bytes into a QImage in 906.504µs",
        "trace:     0.126 first frame on the screen at 125.967298ms",
    ] {
        figures.read(&window, line);
    }
    let close = |found: Option<f64>, wanted: f64| found.is_some_and(|f| (f - wanted).abs() < 1e-6);
    assert!(close(figures.window, 106.0), "{figures:?}");
    assert!(close(figures.up, 65.534_77), "{figures:?}");
    assert!(close(figures.page_one, 10.689_375), "{figures:?}");
    assert!(close(figures.first_frame, 125.967_298), "{figures:?}");
    assert!(close(milliseconds("906.504µs"), 0.906_504));
    assert!(close(milliseconds("1.5s"), 1500.0));
    assert_eq!(milliseconds("soon"), None);
}
