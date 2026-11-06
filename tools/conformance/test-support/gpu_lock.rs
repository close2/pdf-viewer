//! The shared graphics adapter's lock, held around device work and nothing else (ADR 2664).
//!
//! A machine whose hardware adapter is shared between users serialises their device work on one
//! lock file, and a process that holds it for a whole build, gate or suite starves the other user
//! for as long as that takes — an hour and more, where the device work in it is seconds. So the lock
//! is taken by the code doing the work, for the span of the work: [`Hold`] is a guard over one
//! device section, and [`super::gpu_lock_hook`] installs it as `raster_gpu`'s section hook so that
//! every device a test makes holds it from its bring-up to its drop. That span, and not each frame,
//! is the work on the RX 580: another process's allocation in the card's 256 MiB processor-visible
//! window while a job is in flight corrupts the job or hangs the card, and a device allocates and
//! frees memory anywhere in its life (ADR 2675).
//!
//! - **The file** is the place `PDF_VIEWER_GPU_LOCK` names, read through `tools/places.sh` (ADR
//!   2656: one file holds every machine place). Its default there is `/run/lock/gpu.lock` where that
//!   file exists — the server VM's, the one `with-gpu-lock` takes — and empty where it does not, which
//!   means *no lock*: CI and a machine with an adapter of its own wait for nobody. A platform with no
//!   `bash` takes no lock either.
//! - **Hardware only.** A section on a processor's adapter (`wgpu::DeviceType::Cpu`: lavapipe,
//!   llvmpipe) shares nothing and takes nothing. A harness that must lock before any adapter exists
//!   — a clock that starts before bring-up — asks [`Hold::unless_software`], which reads the device
//!   selection the environment makes (`VK_ICD_FILENAMES` naming lavapipe *and* `WGPU_BACKEND=vulkan`) and otherwise counts the
//!   run as hardware, as an unpinned run is (CLAUDE.md's GPU bullet).
//! - **Nested and shared within a process.** The lock is the process's: a section inside a held
//!   section, or on another thread while one is held, counts a holder instead of locking again, which
//!   a second `flock` on a fresh descriptor in the same process would turn into a deadlock.
//! - **Bounded.** A wait longer than `GPU_LOCK_TIMEOUT` seconds — `with-gpu-lock`'s own variable,
//!   600 by default as there — panics naming the file, because a test that waits forever reports
//!   nothing.
//! - **Stopped by a hang, as the kernel records one.** A hardware section that runs past
//!   [`HUNG_AFTER`] — the kernel's ring timeout is ten seconds, so a job the kernel times out holds
//!   its section at least that long — asks the kernel's log ([`hang_line`]) whether amdgpu timed a
//!   ring out, reset the card or reported a fault inside the section's window. Where it did, or
//!   where the log cannot be read, the section leaves a marker beside the lock file
//!   ([`hung_marker`]), and every later hardware section, in any process, refuses while it stands,
//!   until a person has read the kernel's log and removed it: the next process's first frame after
//!   a recovery is where the kernel has reported a privileged register written by the command
//!   stream, so a suite that goes on after a hang feeds a card that is still recovering, and every
//!   reset cancels the other user's work too (ADR 2665). Where the log was read and says nothing,
//!   the section was slow and not hung: it appends a warning line beside the lock file
//!   ([`slow_record`]) and stops nothing (ADR 2740).
//! - **Measured.** Where `PDF_VIEWER_GPU_LOCK_LOG` names a file, every hold appends one line to it —
//!   the process, what the outermost section was, how long it waited and how long it held — so a
//!   run's longest hold and total locked time are an `awk` over the file (`tools/gpu-holds.sh`).
//!
//! Included by `#[path]` beside [`super::gpu_lock_hook`] from the device tests and the clock gates;
//! `std::fs::File::lock`'s non-blocking form is the whole mechanism, so there is no `unsafe` and no
//! dependency.

#![allow(
    dead_code,
    reason = "each including test uses the part it needs, and a #[path] module is compiled per crate"
)]
#![allow(
    clippy::panic,
    reason = "a lock that cannot be taken stops the test that needed it, by name"
)]

use std::fs::{File, OpenOptions, TryLockError};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{Duration, Instant};

/// The place naming the lock file, in `tools/places.sh`.
pub(crate) const PLACE: &str = "PDF_VIEWER_GPU_LOCK";
/// Where each hold is appended, when set.
pub(crate) const LOG: &str = "PDF_VIEWER_GPU_LOCK_LOG";
/// The wait bound in seconds, `with-gpu-lock`'s own name for it.
pub(crate) const TIMEOUT: &str = "GPU_LOCK_TIMEOUT";
const DEFAULT_TIMEOUT: Duration = Duration::from_mins(10);
/// A hardware section held this long is taken for a hung device (ADR 2665): amdgpu's ring timeout
/// is ten seconds, so a job the kernel will time out holds its section at least that long, and the
/// longest hold a working run measured was a planted busy device's 6.1 s.
pub(crate) const HUNG_AFTER: Duration = Duration::from_secs(9);
/// The variable that shortens [`HUNG_AFTER`], in milliseconds, for the marker's own test.
pub(crate) const HUNG_AFTER_MS: &str = "PDF_VIEWER_GPU_HUNG_AFTER_MS";
/// The variable naming a file read in place of the kernel's log, for the marker's own tests. Its
/// lines are `journalctl -k -o short-unix`'s: seconds since the epoch, then the message.
pub(crate) const KERNEL_LOG: &str = "PDF_VIEWER_GPU_KERNEL_LOG";
/// How long a section past the bound waits for the kernel's line: amdgpu prints "soft recovered"
/// once the wave is killed, which is when the job's fence lets the section end, so the line can
/// follow the section's end — on 2026-11-05 the marker was written in second 1793868682 and the
/// line is stamped 1793868682.83. The wait is spent only by a section that has already run nine
/// seconds and found nothing, and five seconds leave the journal room on a machine whose load
/// has reached 677 (ADR 2740).
const KERNEL_GRACE: Duration = Duration::from_secs(5);
/// How far before a section's start a kernel line still counts as its, in seconds: one second of
/// rounding between this process's wall clock and the journal's.
const KERNEL_SLACK: f64 = 1.0;
/// How often a waiting section asks again: short beside a frame, long beside a system call.
const POLL: Duration = Duration::from_millis(5);

/// The process's share of the lock: how many sections hold it, and the descriptor they share.
struct Held {
    holders: usize,
    file: Option<File>,
    since: Option<Instant>,
    waited: Duration,
    label: &'static str,
}

static HELD: Mutex<Held> = Mutex::new(Held {
    holders: 0,
    file: None,
    since: None,
    waited: Duration::ZERO,
    label: "",
});

fn held() -> MutexGuard<'static, Held> {
    // A test that panicked inside a section poisons the mutex; the count it guards is still the
    // count, because the guard's drop ran during the unwind.
    HELD.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The lock file, or `None` where this machine has none (read once per process).
pub(crate) fn lock_file() -> Option<&'static Path> {
    static FILE: OnceLock<Option<PathBuf>> = OnceLock::new();
    FILE.get_or_init(|| {
        let named = match std::env::var_os(PLACE) {
            Some(set) => set.into_string().ok()?,
            None => from_places()?,
        };
        (!named.is_empty()).then(|| PathBuf::from(named))
    })
    .as_deref()
}

/// The file a hung section leaves beside the lock file, or `None` where there is no lock.
pub(crate) fn hung_marker() -> Option<PathBuf> {
    let lock = lock_file()?;
    let name = lock.file_name()?.to_string_lossy().into_owned();
    Some(lock.with_file_name(format!("{name}.pdf-viewer-hung")))
}

/// Refuses a hardware section while a hung section's marker stands (ADR 2665).
fn refuse_after_a_hang() {
    let Some(marker) = hung_marker() else { return };
    if let Ok(said) = std::fs::read_to_string(&marker) {
        panic!(
            "a device section on the shared card hung ({}); no hardware section runs until a \
             person has read `journalctl -k` and removed {} (ADRs 2665, 2740)",
            said.trim(),
            marker.display()
        );
    }
}

/// What the kernel's log says of the window a slow section ran in.
enum Kernel {
    /// amdgpu timed a ring out, reset the card or reported a fault in the window: its line.
    Hang(String),
    /// The log was read and says nothing of the card in the window.
    Quiet,
    /// The log could not be read, and why.
    Unread(String),
}

/// Leaves the marker when a hardware section held past the hang bound ran while the kernel
/// recorded the card hanging, or when the kernel's log cannot say (ADR 2740).
///
/// The bound is the kernel's own figure read backwards — a job amdgpu times out holds its section
/// at least ten seconds — so a section past it *may* have hung, and the kernel's log is what says
/// whether it did. A section of many jobs, or one starved of processors, passes the bound with no
/// job anywhere near the ring timeout; it leaves a warning and stops nothing. A log this process
/// cannot read leaves the marker on the bound alone, because an unmarked hang lets the next suite
/// reset the card under the other user (ADR 2665).
fn mark_if_hung(label: &str, holding: Duration) {
    let bound = std::env::var(HUNG_AFTER_MS)
        .ok()
        .and_then(|millis| millis.parse().ok())
        .map_or(HUNG_AFTER, Duration::from_millis);
    if holding < bound {
        return;
    }
    let Some(marker) = hung_marker() else { return };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let began = now.as_secs_f64() - holding.as_secs_f64();
    let said = format!(
        "pid={} section={label} held_ms={:.0} unix_time={}",
        std::process::id(),
        holding.as_secs_f64() * 1e3,
        now.as_secs()
    );
    let kernel = match kernel_since(began - KERNEL_SLACK) {
        Kernel::Quiet => {
            let note = format!("{said} kernel=quiet");
            eprintln!(
                "a device section on the shared card was held past the hang bound, and the \
                 kernel recorded no ring timeout, reset or fault: slow, not hung ({note})"
            );
            if let Some(record) = slow_record()
                && let Ok(mut file) = OpenOptions::new().create(true).append(true).open(record)
            {
                // A warning's line: a failed append loses the line and stops nothing.
                drop(file.write_all(format!("{note}\n").as_bytes()));
            }
            return;
        }
        Kernel::Hang(line) => format!("kernel=\"{line}\""),
        Kernel::Unread(why) => format!("kernel=unread ({why})"),
    };
    // The marker is the guard: a failed write is said, because without it the suite goes on.
    if let Err(error) = std::fs::write(&marker, format!("{said} {kernel}\n")) {
        eprintln!(
            "the hung-section marker {} was not written: {error}",
            marker.display()
        );
    }
}

/// The file a slow section's warning is appended to, beside the lock file.
pub(crate) fn slow_record() -> Option<PathBuf> {
    let lock = lock_file()?;
    let name = lock.file_name()?.to_string_lossy().into_owned();
    Some(lock.with_file_name(format!("{name}.pdf-viewer-slow")))
}

/// Asks the kernel's log about the card from `since` (seconds since the epoch), waiting
/// [`KERNEL_GRACE`] for a line the journal has not yet stored.
fn kernel_since(since: f64) -> Kernel {
    let asked = Instant::now();
    loop {
        match kernel_lines(since) {
            Err(why) => return Kernel::Unread(why),
            Ok(text) => {
                if let Some(line) = hang_line(&text, since) {
                    return Kernel::Hang(line);
                }
            }
        }
        if asked.elapsed() >= KERNEL_GRACE {
            return Kernel::Quiet;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// The kernel's log from `since` in `journalctl -k -o short-unix`'s shape, or why it is unread.
///
/// `journalctl` answers a user outside the `systemd-journal` group with that user's own journal,
/// which holds no kernel line, and exits well; so an empty window is believed only where the log
/// holds some kernel line at all. [`KERNEL_LOG`] names a file read instead, whole.
fn kernel_lines(since: f64) -> Result<String, String> {
    if let Some(stub) = std::env::var_os(KERNEL_LOG) {
        let text = std::fs::read_to_string(&stub)
            .map_err(|error| format!("{}: {error}", Path::new(&stub).display()))?;
        return if text.trim().is_empty() {
            Err(format!("{} holds no line", Path::new(&stub).display()))
        } else {
            Ok(text)
        };
    }
    let journal = |extra: &[String]| -> Result<String, String> {
        let said = std::process::Command::new("journalctl")
            .args(["-k", "-q", "--no-pager", "-o", "short-unix"])
            .args(extra)
            .output()
            .map_err(|error| format!("journalctl: {error}"))?;
        if said.status.success() {
            Ok(String::from_utf8_lossy(&said.stdout).into_owned())
        } else {
            Err(format!(
                "journalctl -k: {}",
                String::from_utf8_lossy(&said.stderr).trim()
            ))
        }
    };
    // Whole seconds, floored: the window's edge is filtered again by `hang_line`.
    let window = journal(&[format!("--since=@{}", since.max(0.0).floor())])?;
    if window.trim().is_empty()
        && journal(&["-n".to_owned(), "1".to_owned()])?
            .trim()
            .is_empty()
    {
        return Err("journalctl -k returns no kernel line to this user".to_owned());
    }
    Ok(window)
}

/// The first line of `log` at or after `since` in which amdgpu records a hang: a ring timeout, a
/// reset, a fault, or a command stream's illegal register access (ADR 2665's table).
///
/// Any process's line counts, not only this one's: a recovery the other user's job caused leaves
/// the same card recovering under the next suite, and amdgpu's "soft recovered" line names no
/// process at all.
pub(crate) fn hang_line(log: &str, since: f64) -> Option<String> {
    log.lines()
        .filter(|line| {
            line.split_whitespace()
                .next()
                .and_then(|time| time.parse::<f64>().ok())
                .is_some_and(|time| time >= since)
        })
        .find(|line| {
            let line = line.to_ascii_lowercase();
            line.contains("amdgpu")
                && ["timeout", "gpu reset", "fault", "illegal register"]
                    .iter()
                    .any(|word| line.contains(word))
        })
        .map(|line| line.trim().replace('"', "'"))
}

/// The place as `tools/places.sh` answers it, from the workspace that built this test.
fn from_places() -> Option<String> {
    let mut dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = loop {
        if dir.join("tools/places.sh").is_file() {
            break dir;
        }
        dir = dir.parent()?;
    };
    let said = std::process::Command::new("bash")
        .arg("-c")
        .arg(format!(". tools/places.sh; printf %s \"${PLACE}\""))
        .current_dir(root)
        .output()
        .ok()?;
    said.status
        .success()
        .then(|| String::from_utf8_lossy(&said.stdout).into_owned())
}

/// Whether the environment pins a processor's adapter (lavapipe through `VK_ICD_FILENAMES`).
pub(crate) fn environment_pins_software() -> bool {
    // Both halves, because an ICD file alone leaves wgpu free to fall back to OpenGL on the
    // hardware card (ADR 2663): such a run is the hardware's, and is locked as one.
    std::env::var("VK_ICD_FILENAMES").is_ok_and(|icd| icd.contains("lvp"))
        && std::env::var("WGPU_BACKEND").is_ok_and(|backend| backend.eq_ignore_ascii_case("vulkan"))
}

/// One device section's hold on the lock; released when dropped.
#[must_use = "the lock is released when the hold drops"]
pub(crate) struct Hold {
    taken: bool,
    label: &'static str,
    began: Instant,
}

impl Hold {
    /// Holds the lock for a section on an adapter that is hardware (`hardware`), or does nothing.
    pub(crate) fn when(hardware: bool, label: &'static str) -> Self {
        if !hardware || lock_file().is_none() {
            return Self {
                taken: false,
                label,
                began: Instant::now(),
            };
        }
        enter(label);
        Self {
            taken: true,
            label,
            began: Instant::now(),
        }
    }

    /// Holds the lock before any adapter exists, unless the environment pins a processor's.
    pub(crate) fn unless_software(label: &'static str) -> Self {
        Self::when(!environment_pins_software(), label)
    }
}

impl Drop for Hold {
    fn drop(&mut self) {
        if self.taken {
            leave();
            ended(self.label, self.began);
        }
    }
}

/// Leaves the hang marker where a hardware section that began at `began` ran past the bound.
///
/// Each section is timed on its own rather than the process's outermost hold, because the
/// outermost is a device's whole life where the section hook holds the lock across it (ADR 2675),
/// and a device may live far longer than any one piece of its work.
pub(crate) fn ended(label: &str, began: Instant) {
    if lock_file().is_some() {
        mark_if_hung(label, began.elapsed());
    }
}

/// Counts one more holder, taking the file's lock if this is the process's first.
pub(crate) fn enter(label: &'static str) {
    let Some(path) = lock_file() else { return };
    refuse_after_a_hang();
    let mut held = held();
    if held.holders > 0 {
        held.holders = held.holders.saturating_add(1);
        return;
    }
    let asked = Instant::now();
    let file = OpenOptions::new()
        .read(true)
        .open(path)
        .unwrap_or_else(|error| panic!("the GPU lock {} does not open: {error}", path.display()));
    let bound = std::env::var(TIMEOUT)
        .ok()
        .and_then(|seconds| seconds.parse().ok())
        .map_or(DEFAULT_TIMEOUT, Duration::from_secs);
    loop {
        match file.try_lock() {
            Ok(()) => break,
            Err(TryLockError::WouldBlock) if asked.elapsed() < bound => std::thread::sleep(POLL),
            Err(TryLockError::WouldBlock) => panic!(
                "the GPU lock {} was held by another process for more than {} s ({TIMEOUT})",
                path.display(),
                bound.as_secs()
            ),
            Err(TryLockError::Error(error)) => {
                panic!("the GPU lock {} cannot be taken: {error}", path.display())
            }
        }
    }
    held.holders = 1;
    held.waited = asked.elapsed();
    held.since = Some(Instant::now());
    held.label = label;
    held.file = Some(file);
}

/// Counts one holder fewer, releasing the file's lock with the last, and logs the hold.
pub(crate) fn leave() {
    if lock_file().is_none() {
        return;
    }
    let mut held = held();
    held.holders = held.holders.saturating_sub(1);
    if held.holders > 0 {
        return;
    }
    // Closing the descriptor releases the lock; the figure is taken first, so it is the hold.
    let holding = held
        .since
        .take()
        .map_or(Duration::ZERO, |since| since.elapsed());
    drop(held.file.take());
    if let Some(log) = std::env::var_os(LOG) {
        let line = format!(
            "gpu-lock pid={} section={} waited_ms={:.3} held_ms={:.3}\n",
            std::process::id(),
            // One word, so that the line splits on whitespace into its fields.
            held.label.replace(' ', "-"),
            held.waited.as_secs_f64() * 1e3,
            holding.as_secs_f64() * 1e3,
        );
        // The log is a measurement's, never a test's verdict: a failed append loses one line.
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log) {
            drop(file.write_all(line.as_bytes()));
        }
    }
}

/// Records the kind of adapter a device was brought up on, where `PDF_VIEWER_GPU_LOCK_LOG` names a
/// file, whether or not the section locks: a run meant for lavapipe is shown to have had it by
/// these lines rather than by the absence of holds, and a hardware bring-up in such a run is the
/// leak ADR 2663 found (ADR 2665).
pub(crate) fn note_adapter(device_type: &str) {
    if let Some(log) = std::env::var_os(LOG) {
        // The test binary, so that a bring-up on the wrong adapter names the file that made it.
        let exe = std::env::current_exe()
            .ok()
            .and_then(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .unwrap_or_default();
        let line = format!(
            "gpu-adapter pid={} type={device_type} exe={exe}\n",
            std::process::id()
        );
        // A measurement's line, as a hold's is: a failed append loses one line.
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log) {
            drop(file.write_all(line.as_bytes()));
        }
    }
}

/// Whether this process holds the lock now (for the lock's own tests).
pub(crate) fn held_now() -> bool {
    held().holders > 0
}
