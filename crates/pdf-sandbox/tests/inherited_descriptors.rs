//! The decoder's worker closes the descriptors it inherited before it reads a byte (ADR 1945,
//! `doc/questions/A430`).
//!
//! **What the close does** is asked of a running worker here: a write end it inherits, as `GLib`'s
//! `dbus-launch` pipes reach it (ADR 1907), is let go before it reads a byte, so the reader on the
//! other end sees its end of file. **Where the close is** — one `unsafe` call in
//! `inherited-descriptors`, called as `main`'s first statement by each helper — is read off the
//! sources by that crate's `tests/callers.rs` (ADR 3361).

#![forbid(unsafe_code)]

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
