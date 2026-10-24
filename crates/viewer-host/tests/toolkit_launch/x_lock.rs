//! An X display number, as an instrument that starts its own `Xvfb` takes it and gives it back
//! (ADR 2317).
//!
//! `Xvfb` writes `/tmp/.X<n>-lock`, holding its pid, and listens on `/tmp/.X11-unix/X<n>`; it
//! removes both when it is stopped by a signal it handles, `TERM`, and leaves both behind when it
//! is killed. A lock left so names a server that is gone, and an instrument that took every lock
//! for a display in use walked past the numbers such locks held until it ran out of numbers.
//!
//! So the rule has two halves. **A lock is free where this user wrote it and its pid is not
//! running**: the lock and the socket are removed and the number is taken, while a lock of
//! another user, or of a live process, is passed over as before (trap 181 still decides whether a
//! number is *ours*: a server's lock must name its own pid). **And a server is stopped with
//! `TERM`**, given two seconds, and killed only after them; whatever it still left that names its
//! pid is removed. Shared by path between `viewer-host`'s launch harness and `viewer-gtk`'s
//! stand-in tests, which is why it names no crate.

use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// How long a server is given to go on `TERM` before it is killed.
const TERM_GRACE: Duration = Duration::from_secs(2);

/// Display `number`'s lock file.
pub(crate) fn lock(number: u32) -> PathBuf {
    PathBuf::from(format!("/tmp/.X{number}-lock"))
}

/// Display `number`'s socket.
fn socket(number: u32) -> PathBuf {
    PathBuf::from(format!("/tmp/.X11-unix/X{number}"))
}

/// Whether a server may be started on display `number`: its lock is absent, or was this user's
/// and named a process that is gone, in which case it and the socket are removed first.
pub(crate) fn free(number: u32) -> bool {
    free_at(&lock(number), &socket(number))
}

/// [`free`], for a lock and a socket at these paths.
fn free_at(lock: &Path, socket: &Path) -> bool {
    match std::fs::symlink_metadata(lock) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        Err(_) => false,
        Ok(metadata) => {
            if !stale(lock, &metadata) {
                return false;
            }
            remove_own(socket);
            std::fs::remove_file(lock).is_ok()
        }
    }
}

/// Whether a lock is this user's and names a process that is not running. A lock that cannot be
/// read, or holds no number — a server between creating it and writing it — is not stale.
fn stale(lock: &Path, metadata: &std::fs::Metadata) -> bool {
    if own_uid() != Some(metadata.uid()) {
        return false;
    }
    std::fs::read_to_string(lock)
        .ok()
        .and_then(|text| text.trim().parse::<u32>().ok())
        .is_some_and(|pid| !Path::new(&format!("/proc/{pid}")).exists())
}

/// The user this process runs as, read off its own `/proc` entry.
fn own_uid() -> Option<u32> {
    std::fs::metadata("/proc/self").ok().map(|own| own.uid())
}

/// Removes `path` where it exists and is this user's; another user's is left alone.
fn remove_own(path: &Path) {
    let ours = std::fs::symlink_metadata(path)
        .ok()
        .is_some_and(|metadata| Some(metadata.uid()) == own_uid());
    if ours {
        // A file that went between the look and the removal is the outcome wanted.
        drop(std::fs::remove_file(path));
    }
}

/// Stops a server this instrument started on display `number`: `TERM`, [`TERM_GRACE`] to go, a
/// kill after it, and the lock and socket removed where they still name its pid.
pub(crate) fn stop(server: &mut Child, number: u32) {
    let pid = server.id();
    if matches!(server.try_wait(), Ok(None)) {
        let asked = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        let began = Instant::now();
        while asked && began.elapsed() < TERM_GRACE && matches!(server.try_wait(), Ok(None)) {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    // A server already gone answers both with an error, which is the state wanted.
    drop(server.kill());
    drop(server.wait());
    let lock = lock(number);
    let named = std::fs::read_to_string(&lock)
        .ok()
        .and_then(|text| text.trim().parse::<u32>().ok());
    if named == Some(pid) {
        remove_own(&socket(number));
        remove_own(&lock);
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    /// A directory of this test's own, holding a lock and a socket stand-in.
    fn scratch(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "x-lock-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        drop(std::fs::remove_dir_all(&directory));
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    /// The pid of a process that has been started and reaped, which nothing runs under now.
    fn a_gone_pid() -> u32 {
        let mut child = std::process::Command::new("true").spawn().unwrap();
        let pid = child.id();
        child.wait().unwrap();
        pid
    }

    fn files(directory: &Path, pid: u32) -> (PathBuf, PathBuf) {
        let (lock, socket) = (directory.join("lock"), directory.join("socket"));
        std::fs::write(&lock, format!("{pid:>10}\n")).unwrap();
        std::fs::write(&socket, b"").unwrap();
        (lock, socket)
    }

    /// A lock this user left naming a gone process is taken, socket and all; one naming a live
    /// process — this one — is passed over and left as it was, and so is a lock not yet written.
    #[test]
    fn a_lock_of_a_gone_server_is_free_and_a_live_one_is_not() {
        let directory = scratch("lock");
        let (lock, socket) = files(&directory, a_gone_pid());
        assert!(
            super::free_at(&lock, &socket),
            "a gone server's lock is free"
        );
        assert!(!lock.exists() && !socket.exists(), "and both files went");
        let (lock, socket) = files(&directory, std::process::id());
        assert!(
            !super::free_at(&lock, &socket),
            "a live server's lock is taken"
        );
        assert!(lock.exists() && socket.exists(), "and nothing was removed");
        std::fs::write(&lock, b"").unwrap();
        assert!(
            !super::free_at(&lock, &socket),
            "a lock not yet written is taken"
        );
        std::fs::remove_file(&lock).unwrap();
        assert!(super::free_at(&lock, &socket), "an absent lock is free");
        std::fs::remove_dir_all(&directory).unwrap();
    }
}
