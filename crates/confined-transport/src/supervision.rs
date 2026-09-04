//! The child, and the two things a host needs of it: the ability to end it, and its last words.

use std::io::{Read as _, Write as _};
use std::path::PathBuf;
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

/// What a host holds so that it can end a confined worker from another thread.
///
/// # Why a cancel is a kill, and not a message
///
/// **The confined process is running a hostile document; a cancel it has to agree to is a cancel
/// the document can decline.** A cooperative cancel — a flag the interpreter polls, a message a
/// second thread inside the confinement reads — bounds only the work that reaches a check. A
/// content stream that expands into a hundred million marks, a form nested to its depth limit and
/// branching at every level, a filter chain that inflates for a minute: each of those reaches the
/// next check when it reaches it, and "when it reaches it" is the number the attacker chooses. So
/// the only cancel worth the name is the one the kernel enforces, and that is `SIGKILL`.
///
/// What follows from that is a cost rather than a caveat: the worker's document and everything it
/// had derived go when it does, and a host that wants to carry on starts another worker.
///
/// # Shape
///
/// Cheap to clone, and every clone cancels the same worker. It may be made *before* the worker is,
/// because starting one blocks reading its greeting, and a blocking call whose canceller does not
/// exist yet is exactly the hole this closes.
#[derive(Debug, Clone)]
pub struct Canceller(Arc<Supervision>);

impl Canceller {
    /// A canceller for a worker that has not been started yet.
    ///
    /// Cancelling one that never gets a worker is not an error: it makes the start fail instead of
    /// spawning anything.
    #[must_use]
    pub fn new() -> Self {
        Self(Arc::new(Supervision {
            cancelled: AtomicBool::new(false),
            worker: Mutex::new(None),
            said: Arc::default(),
            listener: Mutex::new(None),
        }))
    }

    /// Ends the worker, now.
    ///
    /// Idempotent, callable from any thread, and it never blocks on the work being cancelled —
    /// only on the brief moment a host spends reaping a worker that has already gone.
    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::SeqCst);
        self.kill();
    }

    /// Whether [`Self::cancel`] has been called.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::SeqCst)
    }

    /// Signals the worker, if one is still held here.
    pub fn kill(&self) {
        if let Some(child) = self.0.worker().as_mut() {
            // A worker that has already gone is the outcome asked for, so a failure here is not
            // one: `kill` on a child that has exited is `ESRCH` and means the same thing.
            let _ = child.kill();
        }
    }

    /// Waits for the worker and says how it ended, leaving nothing behind to wait for twice.
    ///
    /// Called only where the worker's output has closed or it has been signalled, so the wait is
    /// the moment it takes a dead process to be reaped rather than the length of a render.
    ///
    /// **What the worker said comes after the wait and after the listening thread has ended**, and
    /// both orders matter: a diagnostic written on the way out is still in the pipe while the
    /// process is dying, and reading the tail before the thread has seen the end of it would
    /// report whatever had arrived by then.
    pub fn reap(&self) -> String {
        let ended = {
            let mut worker = self.0.worker();
            match worker.take() {
                Some(mut child) => match child.wait() {
                    Ok(status) => describe_exit(status),
                    Err(error) => format!("and its status could not be read: {error}"),
                },
                None => "and it had already been waited for".to_owned(),
            }
        };
        // The worker's standard error closes when the worker does, so this thread has already
        // ended or is about to; joining it is what makes the tail complete rather than partial.
        let listener = self
            .0
            .listener
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(handle) = listener {
            let _ = handle.join();
        }
        match self.0.said.take() {
            Some(said) => format!("{ended}, saying: {said}"),
            None => ended,
        }
    }

    /// Whether the worker has ended, without waiting for it to.
    #[must_use]
    pub fn ended(&self) -> Option<String> {
        let mut worker = self.0.worker();
        let child = worker.as_mut()?;
        match child.try_wait() {
            Ok(Some(status)) => Some(describe_exit(status)),
            Ok(None) => None,
            Err(error) => Some(format!("and its status could not be read: {error}")),
        }
    }

    /// Starts collecting the child's diagnostics, and takes ownership of the child.
    ///
    /// The listening thread is what makes [`Self::reap`]'s answer a sentence rather than a signal
    /// number; a host that cannot start one still gets a worker, and is told what it has lost.
    pub(crate) fn adopt(&self, program: &str, mut child: Child) {
        if let Some(stderr) = child.stderr.take() {
            let said = Arc::clone(&self.0.said);
            match std::thread::Builder::new()
                .name(format!("{program} diagnostics"))
                .spawn(move || listen(stderr, &said))
            {
                Ok(handle) => {
                    *self
                        .0
                        .listener
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner) = Some(handle);
                }
                // A host that cannot start a thread still gets a worker: what it loses is the
                // worker's own words, not the worker. Said out loud rather than swallowed.
                Err(error) => {
                    eprintln!("{program}: its diagnostics will not be collected: {error}");
                }
            }
        }
        *self.0.worker() = Some(child);
    }
}

impl Default for Canceller {
    fn default() -> Self {
        Self::new()
    }
}

/// The worker handle and the cancelled flag, shared by a host and its cancellers.
///
/// The child lives here rather than in the host for one reason: `Child::kill` needs `&mut`, and
/// the thread that would call it is not the thread that owns the host — it is the one that is
/// *not* blocked in a read. A mutex is what lets both reach it without this crate reaching for a
/// raw process identifier and a signal, which would cost the `unsafe` it forbids.
#[derive(Debug)]
struct Supervision {
    /// Set once by [`Canceller::cancel`] and never cleared.
    cancelled: AtomicBool,
    /// The worker, from the moment it is spawned until it has been waited for.
    worker: Mutex<Option<Child>>,
    /// What the worker wrote to its standard error, and the thread collecting it.
    said: Arc<LastWords>,
    /// That thread, until it has been joined.
    listener: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl Supervision {
    /// Takes the lock, recovering it from a panic rather than propagating one.
    ///
    /// Nothing under this lock can panic — it holds a `Child` and calls `kill`, `wait` and
    /// `try_wait` on it — so a poisoned lock would mean a panic somewhere that cannot poison it.
    /// Recovering the guard is therefore the honest reading, and it is spelled out rather than
    /// hidden behind an `unwrap` this workspace forbids.
    fn worker(&self) -> std::sync::MutexGuard<'_, Option<Child>> {
        self.worker.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The last thing the worker said before it stopped.
///
/// # Why the host keeps this at all
///
/// **A worker that is killed cannot report why**, and the two ways it is killed are the two that
/// most have to be explained: the seccomp filter, which is a signal number the host can name, and
/// the address-space ceiling, which is not. `RLIMIT_AS` makes an allocation fail; the standard
/// library prints one line and aborts; the host sees `SIGABRT` and has no way to tell that from
/// any other abort. The line is the diagnosis, and it was going to the operator's terminal rather
/// than to the program that has to say something to a person.
///
/// # Why the pipe replaced an inherited descriptor, which is a finding rather than a preference
///
/// The worker's standard error used to be inherited, on the reasoning that a worker that dies
/// should say so where an operator can see it. **`RLIMIT_FSIZE` is 0 in the confinement, and that
/// makes the reasoning false exactly where an operator would be looking**: where the host's own
/// standard error is a *file* — every logged deployment — the worker's first write to it raises
/// `SIGXFSZ` and kills it before a character is printed. Measured (ADR 0597): the same document,
/// the same worker, stderr a pipe gives `killed by signal 6` and the allocation message; stderr a
/// file gives `killed by signal 25` and total silence, which names the wrong cause and explains
/// nothing.
///
/// Everything read here is still written on to this process's own standard error, so an operator
/// watching a terminal sees exactly what they saw before. What is new is that the host keeps a
/// copy.
#[derive(Debug, Default)]
struct LastWords {
    /// The tail of what was written, bounded so that a chatty worker cannot cost the host memory.
    tail: Mutex<String>,
}

impl LastWords {
    /// How much of the worker's diagnostics to keep, in bytes.
    ///
    /// The interesting message is one line and libstd's allocation failure is two. Four kilobytes
    /// is far past both and is a bound rather than a budget: what a host prints to a person is a
    /// sentence, not a log.
    const KEPT: usize = 4096;

    /// Adds what the worker just said, keeping the end of it.
    fn push(&self, said: &str) {
        let mut tail = self.tail.lock().unwrap_or_else(PoisonError::into_inner);
        tail.push_str(said);
        if tail.len() > Self::KEPT {
            // On a character boundary, because the tail is a `String` and half a code point is
            // not one. `drain` on a range that splits one would panic.
            let wanted = tail.len().saturating_sub(Self::KEPT);
            let cut = tail
                .char_indices()
                .find(|(index, _)| *index >= wanted)
                .map_or(tail.len(), |(index, _)| index);
            drop(tail.drain(..cut));
        }
    }

    /// What was said, as one line, or `None` where the worker said nothing.
    ///
    /// Newlines become `; ` because this is appended to a sentence a host prints, and a diagnosis
    /// that arrives as three lines in the middle of one is harder to read than the same words in
    /// a row.
    fn take(&self) -> Option<String> {
        let mut tail = self.tail.lock().unwrap_or_else(PoisonError::into_inner);
        let said = std::mem::take(&mut *tail);
        let joined = said
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("; ");
        (!joined.is_empty()).then_some(joined)
    }
}

/// Reads the worker's standard error to its end, echoing it and keeping the tail.
///
/// Its own thread because the alternative is a deadlock: a host that read the worker's diagnostics
/// only when it had already stopped would leave them in a pipe, and a worker blocked writing to a
/// full pipe is a worker that never answers the frame the host is blocked reading.
fn listen(mut stderr: std::process::ChildStderr, said: &Arc<LastWords>) {
    let mut buffer = [0u8; 4096];
    loop {
        match stderr.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                let Some(chunk) = buffer.get(..read) else {
                    break;
                };
                // Still the operator's. Losing the terminal copy would trade one silence for
                // another.
                let _ = std::io::stderr().write_all(chunk);
                said.push(&String::from_utf8_lossy(chunk));
            }
        }
    }
}

/// Describes how a worker ended, naming the signal where the platform has one.
///
/// `SIGSYS` is the interesting one and it is the seccomp filter firing: the confined process
/// attempted something no page and no generated file needs. A platform with no filter cannot
/// produce that diagnosis and does not pretend to.
///
/// **A number is not a diagnosis, which is why [`LastWords`] exists beside this.** The two ways a
/// confinement ends a worker that are not the filter both arrive here as numbers that name the
/// mechanism rather than the cause: `SIGABRT` for an allocation `RLIMIT_AS` refused, and —
/// before the worker's standard error became a pipe — `SIGXFSZ` for the worker's attempt to *say*
/// so.
#[must_use]
pub fn describe_exit(status: std::process::ExitStatus) -> String {
    #[cfg(unix)]
    if let Some(signal) = {
        use std::os::unix::process::ExitStatusExt as _;
        status.signal()
    } {
        // 31 is `SIGSYS` on every Linux architecture this builds for, and naming it costs no
        // dependency; a platform whose numbering differs simply reports the number.
        let name = if cfg!(target_os = "linux") && signal == 31 {
            " (SIGSYS: a system call the confinement forbids)"
        } else {
            ""
        };
        return format!("killed by signal {signal}{name}");
    }
    match status.code() {
        Some(code) => format!("exited with status {code}"),
        None => "stopped for an unknown reason".to_owned(),
    }
}

/// Finds a worker program by name.
///
/// Searched next to the running executable, then one directory up, because Cargo puts test
/// binaries in `target/<profile>/deps/` while it puts programs in `target/<profile>/`. The
/// environment variable named by `variable` overrides both.
///
/// # Errors
///
/// The executable whose directory was searched, where the program is beside neither it nor its
/// parent — so that a caller can name its own build command in the sentence it prints.
pub fn program_beside_executable(program: &str, variable: &str) -> Result<PathBuf, ProgramMissing> {
    if let Some(named) = std::env::var_os(variable) {
        return Ok(PathBuf::from(named));
    }

    let executable = std::env::current_exe().map_err(ProgramMissing::Unlocatable)?;
    let directory = executable.parent().unwrap_or(&executable);
    let name = format!("{program}{}", std::env::consts::EXE_SUFFIX);
    let beside = directory.join(&name);
    if beside.is_file() {
        return Ok(beside);
    }
    if let Some(parent) = directory.parent() {
        let above = parent.join(&name);
        if above.is_file() {
            return Ok(above);
        }
    }
    Err(ProgramMissing::NotBeside { executable })
}

/// Why a worker program could not be located.
#[derive(Debug, thiserror::Error)]
pub enum ProgramMissing {
    /// It is not beside the running executable nor one directory above it.
    #[error("not found next to {}", executable.display())]
    NotBeside {
        /// The executable whose directory was searched.
        executable: PathBuf,
    },
    /// This process cannot say where it is, so there is nowhere to search.
    #[error("this process cannot say where it is: {0}")]
    Unlocatable(#[source] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::LastWords;

    #[test]
    fn a_tail_is_kept_and_joined_into_one_sentence() {
        let words = LastWords::default();
        words.push("first line\n");
        words.push("second line\n");
        assert_eq!(words.take().as_deref(), Some("first line; second line"));
        assert_eq!(words.take(), None);
    }

    /// A chatty worker cannot cost the host memory, and the cut lands on a character boundary.
    #[test]
    fn a_chatty_worker_costs_a_bounded_tail() {
        let words = LastWords::default();
        for _ in 0..100 {
            words.push(&"é".repeat(100));
        }
        let kept = words.take().expect("something was said");
        assert!(kept.len() <= LastWords::KEPT + 200, "{}", kept.len());
        assert!(kept.chars().all(|character| character == 'é'));
    }
}
