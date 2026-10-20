//! The suite's process split (ADR 2140, `doc/questions/A02`): the worker is confined, and a run
//! through it writes what the in-process route writes, byte for byte, for every verb.
//!
//! **What the equality rests on is that there is one program.** `quorra-transform-worker` runs the
//! file `quorra-transform` runs, over a world whose every answer is the machine's own; so a
//! difference here is the transport losing or reordering a byte, and nothing else can explain one.
//! The rest of this crate's tests run the binary by its default, which is the worker, so they hold
//! each verb's output to its oracle *through* the split; this file holds the split to the route it
//! replaced.

#![cfg(unix)]
#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly, and the \
              confinement reached is printed for the reader of the run"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A committed document under `doc/`.
fn committed(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc")
        .join(name)
}

/// A corpus document, where the corpus is checked out.
fn corpus(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    path.exists().then_some(path)
}

/// A fresh directory for one run's outputs.
fn scratch(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "pdf-transform-confined-{}-{label}-{serial}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("the temporary directory is writable");
    dir
}

/// What one run left: its status, its standard output and error, and every file it wrote.
#[derive(Debug, PartialEq, Eq)]
struct Ran {
    status: i32,
    stdout: Vec<u8>,
    stderr: String,
    files: BTreeMap<String, Vec<u8>>,
}

/// Runs the program in a fresh directory, by the route `extra` selects.
fn run(arguments: &[&str], extra: Option<&str>) -> Ran {
    let dir = scratch(extra.map_or("worker", |_| "here"));
    let mut command = Command::new(env!("CARGO_BIN_EXE_quorra-transform"));
    command.args(arguments).current_dir(&dir);
    if let Some(extra) = extra {
        command.arg(extra);
    }
    let output = command.output().expect("the program runs");
    let mut files = BTreeMap::new();
    let mut pending = vec![dir.clone()];
    while let Some(at) = pending.pop() {
        for entry in std::fs::read_dir(&at).expect("the directory is readable") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let name = path
                .strip_prefix(&dir)
                .expect("under the directory")
                .display()
                .to_string();
            files.insert(name, std::fs::read(&path).expect("the output is readable"));
        }
    }
    std::fs::remove_dir_all(&dir).expect("the directory is removable");
    Ran {
        status: output.status.code().expect("the program exited"),
        stdout: output.stdout,
        stderr: String::from_utf8_lossy(&output.stderr)
            .lines()
            .filter(|line| !line.starts_with("note: --in-process:"))
            .flat_map(|line| [line, "\n"])
            .collect(),
        files,
    }
}

/// The worker is confined before it reads a byte, and says so in its greeting.
#[test]
fn the_worker_greets_its_broker_confined() {
    let worker = confined_transport::program_beside_executable(
        pdf_transform::confined::WORKER_PROGRAM,
        pdf_transform::confined::WORKER_PATH_VARIABLE,
    )
    .expect("cargo builds the worker beside the program for an integration test");
    let host = confined_transport::Host::start(
        &worker,
        pdf_transform::confined::MAGIC,
        &confined_transport::Canceller::new(),
    )
    .expect("the worker starts and greets");
    let confinement = host.confinement();
    println!("transform-worker: confined {confinement:?}");
    if cfg!(target_os = "linux") {
        assert!(
            confinement.is_enforced(),
            "the system-call filter is in force: {confinement:?}"
        );
        assert!(confinement.address_space_limit > 0, "{confinement:?}");
    }
}

/// Every verb writes the same files, the same report and the same exit status through the worker
/// as in this process.
#[test]
fn every_verb_writes_what_the_in_process_route_writes() {
    let bpc = committed("PDF20_AN001-BPC.pdf");
    let bpc = bpc.to_str().expect("a path that is text");
    let af = committed("PDF20_AN002-AF.pdf");
    let af = af.to_str().expect("a path that is text");
    let payload = committed("PDF20_AN002-AF.pdf");
    let payload = payload.to_str().expect("a path that is text");
    let mut runs: Vec<Vec<&str>> = vec![
        vec!["render", bpc, "--dpi", "36", "-o", "page-%d.png"],
        vec![
            "render", bpc, "--dpi", "36", "--format", "pgm", "--pages", "1", "-o", "-",
        ],
        vec!["images", bpc, "--list"],
        vec!["images", bpc, "-o", "img-%d.png", "--report=json"],
        vec!["attachments", af, "--list"],
        vec!["attachments", af, "--save-all", "-o", "./"],
        vec![
            "attachments",
            bpc,
            "--attach",
            payload,
            "--name",
            "af.pdf",
            "-o",
            "out.pdf",
        ],
        vec!["split", bpc, "-o", "piece-%d.pdf"],
        vec!["merge", bpc, af, bpc, af, bpc, af, "-o", "merged.pdf"],
        vec!["pages", bpc, "--rotate", "+90:1-end", "-o", "turned.pdf"],
        vec!["optimize", bpc, "-o", "small.pdf"],
        vec!["redact", bpc, "-o", "redacted.pdf"],
        vec![
            "archive",
            bpc,
            "--to",
            "2b",
            "-o",
            "archived.pdf",
            "--report=json",
        ],
        vec!["render", bpc, "--pages", "99", "-o", "page-%d.png"],
        vec!["render", "absent.pdf", "-o", "page-%d.png"],
        vec!["no-such-verb"],
    ];
    let issue = corpus("issue2177.pdf");
    if let Some(issue) = issue.as_deref().and_then(Path::to_str) {
        runs.push(vec!["render", issue, "--dpi", "24", "-o", "page-%d.png"]);
    }
    let attachment = corpus("attachment.pdf");
    if let Some(attachment) = attachment.as_deref().and_then(Path::to_str) {
        runs.push(vec!["attachments", attachment, "--list"]);
        runs.push(vec![
            "attachments",
            attachment,
            "--save-all",
            "-o",
            "saved/",
        ]);
        runs.push(vec!["attachments", attachment, "--save-all", "-o", "./"]);
    }
    let masked = corpus("issue21570.pdf");
    if let Some(masked) = masked.as_deref().and_then(Path::to_str) {
        runs.push(vec!["images", masked, "-o", "img-%d.png"]);
        runs.push(vec!["images", masked, "--native", "-o", "img-%d"]);
    }
    let grey = corpus("images_1bit_grayscale.pdf");
    if let Some(grey) = grey.as_deref().and_then(Path::to_str) {
        runs.push(vec!["images", grey, "--list"]);
        runs.push(vec!["images", grey, "--native", "-o", "img-%d"]);
        runs.push(vec![
            "render",
            grey,
            "--restrictions",
            "ask",
            "-o",
            "page-%d.png",
        ]);
    }
    for arguments in &runs {
        let confined = run(arguments, None);
        let here = run(arguments, Some("--in-process"));
        assert_eq!(confined, here, "{arguments:?}");
        println!(
            "transform-worker: {arguments:?}: exit {}, {} file(s), {} stdout byte(s) — the same",
            confined.status,
            confined.files.len(),
            confined.stdout.len()
        );
    }
}

/// A merge of more documents than the confinement has descriptors for still reads each one: the
/// fifth on arrives as its bytes (`confined::DOCUMENT_DESCRIPTORS`).
#[test]
fn a_merge_past_the_descriptor_ceiling_reads_every_input() {
    let bpc = committed("PDF20_AN001-BPC.pdf");
    let bpc = bpc.to_str().expect("a path that is text");
    let mut arguments = vec!["merge"];
    arguments.extend(std::iter::repeat_n(bpc, 9));
    arguments.extend(["-o", "merged.pdf"]);
    let confined = run(&arguments, None);
    assert!(
        confined.files.contains_key("merged.pdf"),
        "{}",
        confined.stderr
    );
    assert_eq!(confined, run(&arguments, Some("--in-process")));
}
