//! The shared adapter's lock is taken by the code around a device section, waits for another
//! process's hold, and is never asked for by a section on a processor's adapter (ADR 2664).
//!
//! Each test re-executes this binary as two children on a lock file of its own: a *planted holder*
//! that holds the lock for [`PLANTED`], and a *section* that asks for it once the holder has it.
//! The section reports how long it waited, through the lock's own log line, and the parent judges
//! that figure: about [`PLANTED`] where the section is the hardware's, nothing where it is not.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a test states a broken expectation by stopping"
)]

#[path = "../../../tools/conformance/test-support/gpu_lock.rs"]
mod gpu_lock;
#[path = "../../../tools/conformance/test-support/gpu_lock_hook.rs"]
mod gpu_lock_hook;

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Which child this process is, when it is one.
const ROLE: &str = "GPU_LOCK_TEST_ROLE";
/// How long the planted holder holds the lock.
const PLANTED: Duration = Duration::from_millis(1500);
/// The holder's line once it holds the lock, so that the section is started after it.
const HOLDING: &str = "planted holder: holding";

/// The children: not `#[ignore]`d, so a plain run executes it, and it returns at once there.
#[test]
fn gpu_lock_child() {
    let Ok(role) = std::env::var(ROLE) else {
        return;
    };
    match role.as_str() {
        "holder" => {
            let _held = gpu_lock::Hold::when(true, "planted-holder");
            println!("{HOLDING}");
            std::thread::sleep(PLANTED);
        }
        // A section the harness declares hardware without any adapter: the lock's own mechanism.
        "declared" => drop(gpu_lock::Hold::when(true, "declared-section")),
        // A bring-up and one frame through `raster_gpu`'s seam, on whichever adapter the
        // environment selects; the hook decides from the adapter's own type.
        "device" => {
            gpu_lock_hook::install();
            let mut raster = render_raster::QuorraRasterizer::new_headless()
                .expect("an adapter for the section");
            println!("adapter {}", raster.adapter_description());
            let list = pdf_render::DisplayList::new(test_scenes::A4);
            let target =
                pdf_render::TargetSpec::for_page(&list, 0.1, 1 << 24).expect("a small target");
            pdf_render::Rasterizer::rasterize(&mut raster, &list, target).expect("one frame");
        }
        other => panic!("no child role {other}"),
    }
}

fn child(role: &str, lock: &Path, log: &Path, environment: &[(&str, &str)]) -> Command {
    let mut command = Command::new(std::env::current_exe().expect("this test binary"));
    command
        .args([
            "--exact",
            "gpu_lock_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(ROLE, role)
        .env(gpu_lock::PLACE, lock)
        .env(gpu_lock::LOG, log)
        .envs(environment.iter().copied());
    command
}

/// The planted holder, with its output, which is read to its end so that the holder never writes
/// into a closed pipe.
struct Planted {
    child: Child,
    rest: std::io::Lines<std::io::BufReader<std::process::ChildStdout>>,
}

impl Planted {
    /// Lets the holder finish its hold and checks it ended well.
    fn finish(mut self) {
        self.rest.by_ref().for_each(drop);
        let status = self.child.wait().expect("the holder ends");
        assert!(status.success(), "the planted holder failed");
    }
}

/// Starts the planted holder and returns once it holds the lock.
fn plant(lock: &Path, log: &Path) -> Planted {
    let mut child = child("holder", lock, log, &[])
        .stdout(Stdio::piped())
        .spawn()
        .expect("the planted holder starts");
    let out = child.stdout.take().expect("its output");
    let mut rest = std::io::BufRead::lines(std::io::BufReader::new(out));
    assert!(
        rest.by_ref()
            .any(|line| line.is_ok_and(|line| line.contains(HOLDING))),
        "the planted holder never held the lock"
    );
    Planted { child, rest }
}

/// The `waited_ms` of the log's line for `section`, if the section held the lock at all.
fn waited(log: &Path, section: &str) -> Option<f64> {
    let text = std::fs::read_to_string(log).unwrap_or_default();
    text.lines()
        .filter(|line| line.contains(&format!("section={section} ")))
        .find_map(|line| {
            line.split_whitespace()
                .find_map(|word| word.strip_prefix("waited_ms="))
                .and_then(|figure| figure.parse().ok())
        })
}

fn scratch(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("gpu-lock-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let lock = dir.join("gpu.lock");
    std::fs::write(&lock, b"").expect("a lock file");
    let log = dir.join("holds.log");
    drop(std::fs::remove_file(&log));
    (lock, log)
}

/// The mechanism: a section waits for a hold another process took, for about as long as the
/// holder still had to go, and then holds the lock itself.
#[test]
fn a_section_waits_for_a_planted_holder() {
    let (lock, log) = scratch("declared");
    let holder = plant(&lock, &log);
    let asked = Instant::now();
    let status = child("declared", &lock, &log, &[])
        .status()
        .expect("the section runs");
    let elapsed = asked.elapsed();
    holder.finish();
    assert!(status.success(), "the section child failed");
    let waited = waited(&log, "declared-section").expect("the section held the lock");
    // The holder had all of PLANTED left when the section asked, less the child's start-up.
    assert!(
        waited > 0.5 * PLANTED.as_secs_f64() * 1e3,
        "the section waited {waited:.1} ms for a {PLANTED:?} hold ({elapsed:?} in all)"
    );
}

/// The seam on a processor's adapter: a lavapipe bring-up and frame take no lock, so they finish
/// while the planted holder still holds it, and leave no hold in the log.
#[test]
fn a_processor_s_adapter_never_waits() {
    if !Path::new(LAVAPIPE).exists() {
        println!("no lavapipe on this machine ({LAVAPIPE}); nothing to judge");
        return;
    }
    let (lock, log) = scratch("lavapipe");
    let holder = plant(&lock, &log);
    let asked = Instant::now();
    let output = child(
        "device",
        &lock,
        &log,
        &[("VK_ICD_FILENAMES", LAVAPIPE), ("WGPU_BACKEND", "vulkan")],
    )
    .output()
    .expect("the section runs");
    let elapsed = asked.elapsed();
    holder.finish();
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "the lavapipe child failed:\n{said}"
    );
    assert!(
        said.contains("Cpu"),
        "the child drew on a hardware adapter:\n{said}"
    );
    assert_eq!(
        waited(&log, "bring-up"),
        None,
        "a processor's bring-up took the lock"
    );
    assert!(
        elapsed < PLANTED,
        "the lavapipe section took {elapsed:?}, as long as the planted hold"
    );
}

/// The seam on the hardware adapter: the bring-up waits for the planted holder. `#[ignore]`d
/// because it brings the real adapter up, for a moment and under the lock it tests.
#[test]
#[ignore = "brings the hardware adapter up; run by name on a machine with one"]
fn a_hardware_bring_up_waits_for_a_planted_holder() {
    let (lock, log) = scratch("hardware");
    let holder = plant(&lock, &log);
    let output = child("device", &lock, &log, &[])
        .output()
        .expect("the section runs");
    holder.finish();
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "the hardware child failed:\n{said}"
    );
    assert!(!said.contains("(Cpu,"), "no hardware adapter here:\n{said}");
    let waited = waited(&log, "bring-up").expect("the bring-up held the lock");
    assert!(
        waited > 0.5 * PLANTED.as_secs_f64() * 1e3,
        "the bring-up waited {waited:.1} ms for a {PLANTED:?} hold"
    );
}

/// The ICD file that selects lavapipe on a Debian machine.
const LAVAPIPE: &str = "/usr/share/vulkan/icd.d/lvp_icd.json";
