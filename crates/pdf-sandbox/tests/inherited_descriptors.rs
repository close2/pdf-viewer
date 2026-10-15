//! The three workers' first statement closes the descriptors they inherited, and that statement is
//! the one `unsafe` each of them holds (ADR 1945, `doc/questions/A430`).
//!
//! Two kinds of evidence, for two claims. **Where the permission is** is read off the sources, as
//! `viewer-qt`'s `unsafe_position.rs` reads its own: each worker's root denies `unsafe_code` and
//! lifts the denial once, on the function that closes, whose one `unsafe` block is the one call; and
//! that function is `main`'s first statement. **What it does** is asked of the decoder's worker: a
//! write end it inherits, as `GLib`'s `dbus-launch` pipes reach it (ADR 1907), is let go before it
//! reads a byte, so the reader on the other end sees its end of file.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

/// The three workers, each started with exactly standard input, output and error.
fn workers() -> [PathBuf; 3] {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    [
        crates.join("pdf-sandbox/src/bin/pdf-sandbox-worker.rs"),
        crates.join("viewer-confined/src/bin/pdf-view-worker.rs"),
        crates.join("pdf-script-worker/src/bin/pdf-script-worker.rs"),
    ]
}

/// A line with its comment removed and the lint's own name taken out, so that only a use of the
/// permission is left to find: the attributes that name `unsafe_code` state the rule.
fn code_only(line: &str) -> String {
    let code = line.find("//").map_or(line, |at| &line[..at]);
    code.replace("unsafe_code", "")
}

#[test]
fn each_worker_lifts_the_denial_once_on_the_close_and_calls_it_first() {
    for path in workers() {
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} is a worker: {error}", path.display()));
        let named = path.display();
        assert_eq!(
            source.matches("#![deny(unsafe_code)]").count(),
            1,
            "{named}: the root denies it"
        );
        let lines: Vec<&str> = source.lines().collect();
        let lifts: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(at, line)| {
                line.trim() == "#[expect("
                    && lines
                        .get(at.saturating_add(1))
                        .is_some_and(|next| next.trim() == "unsafe_code,")
            })
            .map(|(at, _)| at)
            .collect();
        let [lift] = lifts.as_slice() else {
            panic!("{named}: the denial is lifted exactly once, not at {lifts:?}");
        };
        let lifted = lines
            .iter()
            .skip(lift.saturating_add(1))
            .map(|line| line.trim())
            .find(|line| line.starts_with("fn "));
        assert_eq!(
            lifted,
            Some("fn keep_only_the_standard_descriptors() -> std::io::Result<()> {"),
            "{named}: and it is lifted on the close"
        );
        let uses: Vec<String> = lines
            .iter()
            .map(|line| code_only(line))
            .filter(|line| line.contains("unsafe"))
            .map(|line| line.trim().to_owned())
            .collect();
        assert_eq!(
            uses,
            vec!["let closed = unsafe { libc::close_range(3, u32::MAX, 0) };".to_owned()],
            "{named}: the one `unsafe` is the one call"
        );
        let first = lines
            .iter()
            .skip_while(|line| !line.starts_with("fn main()"))
            .nth(1)
            .map(|line| line.trim());
        assert_eq!(
            first,
            Some("if let Err(error) = keep_only_the_standard_descriptors() {"),
            "{named}: and it is `main`'s first statement"
        );
    }
}

/// The decoder's worker, handed a write end it was not meant to have, closes it before it reads.
///
/// The pipe's write end is made inheritable — what `GLib`'s `pipe2(…, 0)` makes — before the worker
/// is started, and this side then drops its own copy. A worker that kept the one it inherited
/// would hold the read below open for as long as it lives, which is the hang ADR 1907 measured; one
/// that closed it lets the read end at once. Planted back — the close taken out of `main` — this
/// fails at the wait.
#[cfg(target_os = "linux")]
#[test]
fn the_decoder_lets_go_of_a_write_end_it_inherited() {
    use std::io::Read as _;
    use std::process::{Command, Stdio};

    let (mut reader, writer) = std::io::pipe().expect("a pipe");
    rustix::io::fcntl_setfd(&writer, rustix::io::FdFlags::empty())
        .expect("the write end is made inheritable");
    let mut worker = Command::new(env!("CARGO_BIN_EXE_pdf-sandbox-worker"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the decoder's worker starts");
    drop(writer);

    let (told, ended) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut read = Vec::new();
        let _ = told.send(reader.read_to_end(&mut read).map(|_| read.len()));
    });
    let outcome = ended.recv_timeout(std::time::Duration::from_secs(10));
    let _ = worker.kill();
    let _ = worker.wait();
    match outcome {
        Ok(Ok(0)) => {}
        Ok(other) => panic!("the read ended, but not with an end of file: {other:?}"),
        Err(waited) => panic!("the worker held the write end it inherited: {waited}"),
    }
}
