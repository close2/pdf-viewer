//! A worker's memory ceiling, held from outside it where no process can hold one on itself.
//!
//! On Linux a worker installs its own ceiling as `RLIMIT_AS` and an allocation past it fails
//! inside the worker. macOS refuses `setrlimit(RLIMIT_AS)`, and an allocation past the number it
//! asked for still succeeds — measured on macOS 26 — so the bound has to be held by the one
//! process that outlives the worker's mistakes: the one that started it (ADR 2864).
//!
//! [`Watch::start`] is called by whoever spawned the worker, with the ceiling the worker reported
//! in its greeting. A thread reads the worker's physical footprint every [`INTERVAL`], and the
//! first reading past the ceiling kills the worker. What the caller then sees is the worker's
//! death through the path it already has for one — an image reported undecodable, a page refused,
//! a script stopped — and [`Watch::exceeded`] says why, so the sentence can name the ceiling.
//!
//! # What it costs, and what it is not
//!
//! - **A thread per worker** that sleeps between readings; a reading is one `proc_pid_rusage`.
//! - **It is a sampling interval late.** Linux refuses the allocation that crosses the ceiling;
//!   this sees the footprint after it has crossed, so a worker can be past its ceiling by what it
//!   writes in [`INTERVAL`] before it is stopped. That is the difference between a bound held
//!   inside a process and one held from outside it, and ADR 2864 accepts it.
//! - **It is built not to kill a process that is not the worker.** The worker is named by its
//!   identifier, which the system gives to another process once the worker is reaped; so every
//!   reading also compares the start time with the first one, and the watch ends the moment the
//!   process is gone or the start time differs. Its owners end the watch before they reap the
//!   worker. The one window left is between a reading and the kill that follows it, microseconds
//!   against a worker that would have to exit and be reaped by somebody else inside them.
//! - **It fails closed.** A reading refused for any reason but the process being gone — a host
//!   that is itself sandboxed, a worker of another user — is a worker held to nothing: the first
//!   one is an error for the caller, and one later in the run kills the worker and says why.
//!
//! On every platform but macOS [`Watch::start`] answers `None` and starts nothing: Linux's ceiling
//! is the kernel's, and Windows has none yet (`doc/todo/35`).

/// How often a watched worker's footprint is read.
///
/// Twenty milliseconds is about what a refresh of the viewer costs and a small fraction of what a
/// decode that could reach a gigabyte takes, so a bomb is stopped within a few hundred megabytes of
/// its ceiling — memory written at the machine's full rate for that long — while a reading is a few
/// microseconds of one thread fifty times a second.
pub const INTERVAL: std::time::Duration = std::time::Duration::from_millis(20);

/// A worker held to a memory ceiling by this process; dropping it ends the watch.
#[derive(Debug)]
pub struct Watch {
    #[cfg(target_os = "macos")]
    inner: macos::Watching,
}

impl Watch {
    /// Starts holding the process `pid` to `ceiling` bytes of physical footprint.
    ///
    /// `None` where there is nothing to watch: on any platform but macOS, for a ceiling of zero
    /// (none was reported), and for a process already gone.
    ///
    /// # Errors
    ///
    /// The operating system's, where the watching thread could not be started. A caller that gets
    /// one has a worker held to nothing and must not hand it work.
    pub fn start(pid: u32, ceiling: u64) -> std::io::Result<Option<Self>> {
        #[cfg(target_os = "macos")]
        {
            Ok(macos::Watching::start(pid, ceiling)?.map(|inner| Self { inner }))
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (pid, ceiling);
            Ok(None)
        }
    }

    /// The footprint, in bytes, at which the worker was killed for passing its ceiling — or `None`
    /// where the watch has killed nothing.
    #[must_use]
    pub fn exceeded(&self) -> Option<u64> {
        #[cfg(target_os = "macos")]
        {
            self.inner.exceeded()
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    /// The sentence a worker killed by this watch is reported with, or `None` where it was not.
    #[must_use]
    pub fn sentence(&self) -> Option<String> {
        if let Some(footprint) = self.exceeded() {
            return Some(format!(
                "it was stopped by the process that started it, at a footprint of {footprint} \
                 bytes, past its ceiling"
            ));
        }
        #[cfg(target_os = "macos")]
        if self.inner.unread() {
            return Some(
                "it was stopped by the process that started it, which could no longer read its \
                 footprint and so could not hold it to its ceiling"
                    .to_owned(),
            );
        }
        None
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::mpsc::{self, RecvTimeoutError};
    use std::thread::JoinHandle;

    use super::INTERVAL;

    /// The thread, the line that stops it, and what it found.
    #[derive(Debug)]
    pub(super) struct Watching {
        /// Dropped to end the watch: the thread's wait on it returns at once.
        stop: Option<mpsc::Sender<()>>,
        /// The footprint the worker was killed at, or 0 while it has not been.
        exceeded: Arc<AtomicU64>,
        /// Whether the worker was killed because a reading of it was refused.
        unread: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    /// Whether a refused reading means only that the process is gone, which ends a watch quietly.
    fn gone(error: &std::io::Error) -> bool {
        error.raw_os_error() == Some(rustix::io::Errno::SRCH.raw_os_error())
    }

    impl Watching {
        pub(super) fn start(pid: u32, ceiling: u64) -> std::io::Result<Option<Self>> {
            if ceiling == 0 {
                return Ok(None);
            }
            // The first reading names the process the watch is about. A worker already gone has
            // nothing to be held to; one that cannot be read cannot be held, and that is the
            // caller's to refuse.
            let first = match darwin_sandbox::usage(pid) {
                Ok(first) => first,
                Err(error) if gone(&error) => return Ok(None),
                Err(error) => return Err(error),
            };
            let Some(target) = i32::try_from(pid)
                .ok()
                .and_then(rustix::process::Pid::from_raw)
            else {
                return Ok(None);
            };
            let exceeded = Arc::new(AtomicU64::new(0));
            let found = Arc::clone(&exceeded);
            let unread = Arc::new(AtomicBool::new(false));
            let refused = Arc::clone(&unread);
            let (stop, stopped) = mpsc::channel::<()>();
            let thread = std::thread::Builder::new()
                .name("pdf-sandbox-watch".to_owned())
                .spawn(move || {
                    loop {
                        match stopped.recv_timeout(INTERVAL) {
                            Err(RecvTimeoutError::Timeout) => {}
                            Ok(()) | Err(RecvTimeoutError::Disconnected) => return,
                        }
                        let now = match darwin_sandbox::usage(pid) {
                            Ok(now) => now,
                            Err(error) if gone(&error) => return,
                            Err(_) => {
                                // Fails closed: a worker that can no longer be read is no longer
                                // held, and the start time cannot be checked either — but the
                                // process could still be read a tick ago, so it is the worker.
                                refused.store(true, Ordering::Release);
                                let _ = rustix::process::kill_process(
                                    target,
                                    rustix::process::Signal::KILL,
                                );
                                return;
                            }
                        };
                        if now.started != first.started {
                            return;
                        }
                        if now.footprint > ceiling {
                            // Recorded before the kill, so that whoever reads the worker's death
                            // can already read why.
                            found.store(now.footprint, Ordering::Release);
                            let _ = rustix::process::kill_process(
                                target,
                                rustix::process::Signal::KILL,
                            );
                            return;
                        }
                    }
                })?;
            Ok(Some(Self {
                stop: Some(stop),
                exceeded,
                unread,
                thread: Some(thread),
            }))
        }

        pub(super) fn unread(&self) -> bool {
            self.unread.load(Ordering::Acquire)
        }

        pub(super) fn exceeded(&self) -> Option<u64> {
            match self.exceeded.load(Ordering::Acquire) {
                0 => None,
                footprint => Some(footprint),
            }
        }
    }

    impl Drop for Watching {
        fn drop(&mut self) {
            drop(self.stop.take());
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }
}
