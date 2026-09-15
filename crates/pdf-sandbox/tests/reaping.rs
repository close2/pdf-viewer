//! A worker that is refused at its handshake is reaped, not left behind.
//!
//! A child nobody waits on stays a zombie once it exits, and a zombie is a task the cgroup
//! still counts: on 2026-09-15 three crawl censuses leaked ten thousand of them and the shared
//! scope's `pids.max` refused every fork on the machine. Two handshake refusals part here. A
//! worker that exits without greeting is collected by `died`, which waits on it to say how it
//! ended. A worker that is *alive* and greets with something else — or greets with nothing
//! until the deadline — used to be dropped alive: it left when its pipe closed, and became a
//! zombie of a parent that never waited. `Connection`'s `Drop` is the reap; the second test is
//! what fails without it (planted: one child left behind).
//!
//! The worker path is an environment variable, so each check runs in a child copy of this test
//! binary with the variable set for that process alone, and the parent reads the count it
//! prints. Linux only: the count is read from `/proc`.
#![cfg(target_os = "linux")]
#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot set itself up has failed, and the message is its report"
)]

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

use pdf_sandbox::{Sandbox, WORKER_PATH_VARIABLE};

/// Tells the child copy of this binary it is the inner half, and which worker to expect.
const INNER: &str = "PDF_SANDBOX_REAPING_INNER";

/// How many direct children this process has, in any state, per `/proc/<pid>/stat`.
///
/// A reaped worker is no child at all; a leaked one is a live process until it exits and a
/// zombie after, and either is a child this count sees.
fn children() -> usize {
    let me = std::process::id().to_string();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return 0;
    };
    entries
        .flatten()
        .filter_map(|entry| std::fs::read_to_string(entry.path().join("stat")).ok())
        .filter(|stat| {
            // `pid (comm) state ppid …` — the command name may hold spaces and parentheses,
            // so the fields after it start at the last `)`.
            let Some(after) = stat.rsplit_once(')').map(|(_, rest)| rest) else {
                return false;
            };
            after.split_whitespace().nth(1) == Some(me.as_str())
        })
        .count()
}

/// A worker that greets with something that is not this protocol's and then stays alive.
fn stranger() -> PathBuf {
    let path = std::env::temp_dir().join(format!("pdf-sandbox-stranger-{}.sh", std::process::id()));
    std::fs::write(
        &path,
        "#!/bin/sh\nprintf 'this is not the greeting the parent expects, and it is long enough to be read whole'\nexec sleep 60\n",
    )
    .expect("the temporary directory takes a script");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("the script can be made executable");
    path
}

/// Runs the refusal in a child copy of this binary and returns how many children it had left.
fn children_left_by(test: &str, worker: &std::path::Path) -> String {
    let output = Command::new(std::env::current_exe().expect("this test binary has a path"))
        .env(WORKER_PATH_VARIABLE, worker)
        .env(INNER, "1")
        .args(["--exact", test, "--nocapture"])
        .output()
        .expect("this test binary runs its own inner half");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "the inner run failed:\n{stdout}");
    stdout
        .lines()
        .find_map(|line| line.strip_prefix("children="))
        .unwrap_or_else(|| panic!("the inner run printed no count:\n{stdout}"))
        .to_owned()
}

/// The inner half: the refusal, a settle, and the count.
fn inner() {
    assert!(
        Sandbox::shared().confinement().is_err(),
        "the worker must be refused"
    );
    // Long enough for a worker that was still running at the refusal to have exited.
    std::thread::sleep(std::time::Duration::from_millis(300));
    println!("children={}", children());
}

#[test]
fn a_worker_that_exits_before_greeting_is_reaped() {
    if std::env::var_os(INNER).is_some() {
        return inner();
    }
    // `true` exits at once with nothing on its stdout.
    let left = children_left_by(
        "a_worker_that_exits_before_greeting_is_reaped",
        "/bin/true".as_ref(),
    );
    assert_eq!(
        left, "0",
        "a worker that exited unheard was left as a zombie"
    );
}

#[test]
fn a_worker_alive_with_a_wrong_greeting_is_reaped() {
    if std::env::var_os(INNER).is_some() {
        return inner();
    }
    let worker = stranger();
    let left = children_left_by("a_worker_alive_with_a_wrong_greeting_is_reaped", &worker);
    let _ = std::fs::remove_file(&worker);
    assert_eq!(
        left, "0",
        "a refused worker was left alive, a zombie once it exits"
    );
}
