//! The one place in this tree that starts a process.
//!
//! # Why there is exactly one
//!
//! `doc/questions/A54`, the owner's answer of 2026-09-12: every consumer this project ships — the
//! command-line program, the KIO worker, the FUSE filesystem — runs a remedy's external tool
//! through **one shared executor**, so that *"it just happens"* is the default in each of them and
//! the only code that spawns anything is this file. [`crate::apply`] returns
//! [`ToolRequest`]s and never runs one; a caller loops over [`crate::Report::requested`], calls
//! [`execute`] on each, puts the results in the plan and applies again.
//!
//! The three faces and where each calls it:
//!
//! | face | where the loop goes |
//! |---|---|
//! | the command-line program | `src/bin/quorra-transform.rs`, `carry_out_the_requests` — **built** |
//! | the KIO worker | `crates/pdf-vfs`'s commit path, beside its own `pdf_transform::apply` call |
//! | the FUSE filesystem | the same, through `pdf-vfs`: `pdf-fuse` holds no `apply` of its own |
//!
//! The second and third are RFC 0003's and are not this round's; both reach `apply` through
//! `pdf_vfs`, so both gain the loop in one place when that round comes.
//!
//! # What confinement this offers, which is none, and what it does instead
//!
//! `doc/questions/A56`: **no confinement is offered in the first version**, and the warning lives
//! where an operator declares a tool rather than in a security document nobody opens —
//! [`crate::archive::UNTRUSTED_INPUT_WARNING`], printed by `--remedy-sites` and written into every
//! shipped profile's `[tool.…]` block. A confinement profile written against no particular program
//! is a guess; this is where a per-tool one would go the day a real deployment brings its own
//! program.
//!
//! What *is* done, and it is section 4.5 of `doc/rfc/0007` item by item:
//!
//! - the executor makes the working directory, passes only paths inside it, and removes it after;
//! - nothing from the document reaches `args` — the bytes are written to a file in that directory
//!   and the program is given that file, by a name this executor chose;
//! - the result is size-bounded **before** it is read, by asking the file how long it is;
//! - the program is run directly, never through a shell, so there is no word splitting and no
//!   metacharacter to escape.
//!
//! # Why standard input and output are files
//!
//! A child whose output is a pipe cannot be waited on with a timeout without somebody draining the
//! pipe, and a parent that waits before draining deadlocks the moment the child fills it. Handing
//! the child files for all three descriptors removes the problem rather than managing it: nothing
//! has to be pumped, the wait loop does no I/O, and the bound is applied to a file whose length is
//! known before a byte of it is read.

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use sha2::{Digest as _, Sha256};

use crate::tool::{DECLINED, Delivery, MOST_STDERR, ToolOutcome, ToolRequest, ToolResult};

/// Why an invocation could not be attempted, or could not be cleaned up after.
///
/// **Not the same thing as a tool that failed.** A program that runs and exits non-zero, times out
/// or overruns its limit is a [`ToolOutcome::Failed`] carried in the result, because that is a fact
/// about the tool the report has to state and the site's `on-failure` has to answer. This type is
/// for the cases where there is no result at all: the working directory could not be made, or the
/// input could not be written into it.
#[derive(Debug, thiserror::Error)]
pub enum ExecuteError {
    /// The working directory this invocation owns could not be created.
    #[error("a working directory for the tool {tool:?} could not be created under {at}: {error}")]
    Workspace {
        /// The tool's name.
        tool: String,
        /// Where it was to go.
        at: PathBuf,
        /// What the operating system said.
        error: std::io::Error,
    },
    /// The document-derived bytes could not be written into it.
    #[error("the input for the tool {tool:?} could not be written to {at}: {error}")]
    Input {
        /// The tool's name.
        tool: String,
        /// The file it was to go in.
        at: PathBuf,
        /// What the operating system said.
        error: std::io::Error,
    },
}

/// How often the wait loop asks whether the child has finished.
///
/// Small enough that a fast tool is not held up by a poll interval, large enough that a slow one
/// does not spin a core. Neither number is measured, because neither is on any latency path this
/// project gates: a conversion that runs an external program is already paying that program's
/// startup.
const POLL: Duration = Duration::from_millis(5);

/// Distinguishes the working directories of two invocations in one process.
///
/// The process identifier alone is not enough — a conversion makes one request per attachment —
/// and a clock is not used because two invocations can fall in the same nanosecond on a coarse
/// one. A counter cannot repeat within a process, and the process identifier separates processes.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// Runs one request and returns what came back.
///
/// The working directory is created before the program starts and removed before this returns,
/// whatever the program did. The result's bytes are whatever the delivery says — everything on
/// standard output, or the one file left in the output directory — bounded by
/// [`crate::tool::Bounds::output_limit`] and digested so a report can record what was returned.
///
/// **`expects` is not checked here.** What a tool promised and what it produced is a question about
/// the *conversion* — `doc/rfc/0007` section 4.2's "what comes back is not trusted" — and the answer
/// belongs where the bytes are used, so that a caller replaying a recorded result is held to the
/// same check as one that ran the program. `crate::archive` is where it is asked.
///
/// # Errors
///
/// [`ExecuteError`], for the two ways an invocation cannot be attempted at all. A program that runs
/// and fails is `Ok` with [`ToolOutcome::Failed`].
pub fn execute(request: &ToolRequest) -> Result<ToolResult, ExecuteError> {
    let workspace = Workspace::make(&request.tool)?;
    let input = workspace.path.join("in");
    std::fs::write(&input, &*request.input).map_err(|error| ExecuteError::Input {
        tool: request.tool.clone(),
        at: input.clone(),
        error,
    })?;
    let out = workspace.path.join("out");
    let stdout = workspace.path.join("stdout");
    let stderr = workspace.path.join("stderr");
    if request.delivery == Delivery::OutputDirectory
        && let Err(error) = std::fs::create_dir(&out)
    {
        return Err(ExecuteError::Workspace {
            tool: request.tool.clone(),
            at: out,
            error,
        });
    }
    let outcome = run(request, &input, &out, &stdout, &stderr);
    let (outcome, output) = match outcome {
        Err(failure) => (ToolOutcome::Failed(failure), Vec::new()),
        Ok(()) => collect(request, &out, &stdout),
    };
    let mut digest = Sha256::new();
    digest.update(&output);
    Ok(ToolResult {
        id: request.id.clone(),
        outcome,
        output: output.into(),
        stderr: diagnostics(&stderr),
        program: request.program.clone(),
        digest: hexadecimal(&digest.finalize()),
    })
}

/// Starts the program, waits for it under the timeout, and says what the status meant.
///
/// `Err` is the sentence a [`ToolOutcome::Failed`] carries; `Ok(())` means the program exited 0 or
/// declined, and [`collect`] then decides which.
fn run(
    request: &ToolRequest,
    input: &Path,
    out: &Path,
    stdout: &Path,
    stderr: &Path,
) -> Result<(), String> {
    let Ok(from) = std::fs::File::open(input) else {
        return Err("the input file this executor wrote could not be reopened".to_owned());
    };
    let (Ok(to), Ok(errors)) = (std::fs::File::create(stdout), std::fs::File::create(stderr))
    else {
        return Err(
            "the files this executor collects the program's output in could not be \
                    created"
                .to_owned(),
        );
    };
    // `{in}` and `{out}` are filled here and nowhere else: the executor is the only thing that has
    // a directory, which is `doc/rfc/0007` section 4.5's first bullet. Every other placeholder was
    // substituted where the request was built.
    let args: Vec<String> = request
        .args
        .iter()
        .map(|argument| {
            argument
                .replace("{in}", &input.to_string_lossy())
                .replace("{out}", &out.to_string_lossy())
        })
        .collect();
    let mut child = match Command::new(&request.program)
        .args(&args)
        .stdin(Stdio::from(from))
        .stdout(Stdio::from(to))
        .stderr(Stdio::from(errors))
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return Err(format!(
                "the program {} could not be started: {error}",
                request.program.display()
            ));
        }
    };
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Err(error) => return Err(format!("the program could not be waited for: {error}")),
            Ok(Some(status)) => {
                if status.success() || status.code() == Some(DECLINED) {
                    return Ok(());
                }
                return Err(match status.code() {
                    Some(code) => format!("the program exited with status {code}"),
                    None => "the program was killed by a signal".to_owned(),
                });
            }
            Ok(None) => {}
        }
        if started.elapsed() >= request.bounds.timeout {
            // Best effort in both calls: a child that has exited between the poll above and here
            // makes `kill` fail with no consequence, and the sentence below is true either way.
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "the program did not finish within {} second(s), the timeout this tool declares",
                request.bounds.timeout.as_secs()
            ));
        }
        std::thread::sleep(POLL);
    }
}

/// The result the delivery names, bounded, and what the exit status made of it.
///
/// The bound is applied to the file's stated length before a byte is read, which is the whole of
/// what keeps a program that writes without stopping from being this process's memory problem.
fn collect(request: &ToolRequest, out: &Path, stdout: &Path) -> (ToolOutcome, Vec<u8>) {
    let at = match request.delivery {
        Delivery::Stdout => stdout.to_path_buf(),
        Delivery::OutputDirectory => match one_file_in(out) {
            Ok(path) => path,
            Err(sentence) => return (ToolOutcome::Failed(sentence), Vec::new()),
        },
    };
    match bounded(&at, request.bounds.output_limit) {
        Ok(bytes) if bytes.is_empty() => (
            ToolOutcome::Declined,
            // A tool that exits 0 and produces nothing has not produced a result, and saying so
            // as a decline rather than as a success is what lets the site's `on-failure` answer
            // it. The empty output is carried all the same, so the digest is of what came back.
            Vec::new(),
        ),
        Ok(bytes) => (ToolOutcome::Produced, bytes),
        Err(sentence) => (ToolOutcome::Failed(sentence), Vec::new()),
    }
}

/// The single file in the output directory, or the sentence saying why there is not one.
fn one_file_in(out: &Path) -> Result<PathBuf, String> {
    let Ok(entries) = std::fs::read_dir(out) else {
        return Err("the output directory this executor made could not be read".to_owned());
    };
    let mut found: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        if entry.path().is_file() {
            found.push(entry.path());
        }
    }
    found.sort();
    match found.len() {
        0 => Err("the program left no file in the output directory it was given".to_owned()),
        1 => found.pop().ok_or_else(|| {
            // Unreachable: the match arm is on the length being one. Stated rather than
            // `expect`ed, which `CLAUDE.md` principle 1 asks of every panicking path.
            "the output directory held one file and then did not".to_owned()
        }),
        many => Err(format!(
            "the program left {many} files in the output directory, so which of them is the \
             result is not something this executor can decide"
        )),
    }
}

/// A file's bytes, refused rather than truncated where it is longer than the limit.
fn bounded(at: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let Ok(metadata) = std::fs::metadata(at) else {
        return Err("the program's result could not be measured".to_owned());
    };
    if metadata.len() > limit {
        return Err(format!(
            "the program produced {} bytes, past the {limit} this tool declares as its \
             output-limit",
            metadata.len()
        ));
    }
    std::fs::read(at).map_err(|error| format!("the program's result could not be read: {error}"))
}

/// The program's standard error, bounded and with the cut stated.
fn diagnostics(at: &Path) -> String {
    let Ok(mut file) = std::fs::File::open(at) else {
        return String::new();
    };
    let mut bytes = Vec::new();
    // One more than the bound, so that a message exactly at it is not reported as truncated.
    let take = u64::try_from(MOST_STDERR.saturating_add(1)).unwrap_or(u64::MAX);
    let read = file.by_ref().take(take).read_to_end(&mut bytes);
    if read.is_err() {
        return String::new();
    }
    if bytes.len() > MOST_STDERR {
        bytes.truncate(MOST_STDERR);
        let mut text = String::from_utf8_lossy(&bytes).into_owned();
        text.push_str("… (the program wrote more than this executor carries)");
        return text;
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// A digest as lower-case hexadecimal.
fn hexadecimal(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        // Writing to a `String` cannot fail; the result is discarded rather than unwrapped.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// A directory this invocation owns, removed when it goes out of scope.
///
/// A guard rather than a call at the end of [`execute`], because every early return is a path that
/// would otherwise leave a directory of somebody's document behind.
struct Workspace {
    /// Where it is.
    path: PathBuf,
}

impl Workspace {
    /// Makes one, under the system's temporary directory.
    fn make(tool: &str) -> Result<Self, ExecuteError> {
        let root = std::env::temp_dir();
        let at = root.join(format!(
            "quorra-tool-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        // `create_dir` rather than `create_dir_all`: it fails where the directory exists, which is
        // the property that keeps two runs from sharing one, and this executor wants that failure
        // rather than somebody else's directory.
        std::fs::create_dir(&at).map_err(|error| ExecuteError::Workspace {
            tool: tool.to_owned(),
            at: at.clone(),
            error,
        })?;
        Ok(Self { path: at })
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        // Best effort: a directory that cannot be removed is a fact about the filesystem, and
        // there is nothing this program can do about it that would not be worse than saying
        // nothing — every path inside it is one this executor wrote.
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::Bounds;
    use std::sync::Arc;

    /// A request over `/bin/cat`, which is the smallest real program that transforms bytes.
    fn cat(input: &[u8]) -> ToolRequest {
        ToolRequest {
            id: "test/1/0".to_owned(),
            site: "a-site".to_owned(),
            tool: "cat".to_owned(),
            program: PathBuf::from("/bin/cat"),
            args: Vec::new(),
            input: Arc::from(input),
            expects: "application/octet-stream".to_owned(),
            bounds: Bounds {
                timeout: Duration::from_secs(10),
                output_limit: 1 << 20,
            },
            delivery: Delivery::Stdout,
            subject: "the test's bytes".to_owned(),
        }
    }

    #[test]
    fn a_program_reading_standard_input_returns_what_it_wrote() {
        let request = cat(b"the document's bytes");
        let Ok(result) = execute(&request) else {
            panic!("cat could not be run");
        };
        assert_eq!(result.outcome, ToolOutcome::Produced);
        assert_eq!(&*result.output, b"the document's bytes");
        // The digest is of what came back, which is what `doc/rfc/0007` section 4.4 asks a
        // tool-invoking conversion to record so a re-run can be checked.
        assert_eq!(result.digest.len(), 64);
    }

    #[test]
    fn a_result_past_the_limit_is_a_failure_rather_than_a_truncation() {
        let mut request = cat(&vec![b'x'; 4096]);
        request.bounds.output_limit = 16;
        let Ok(result) = execute(&request) else {
            panic!("cat could not be run");
        };
        let ToolOutcome::Failed(sentence) = &result.outcome else {
            panic!("a result past the limit is a failure: {:?}", result.outcome);
        };
        assert!(sentence.contains("output-limit"), "{sentence}");
        assert!(result.output.is_empty(), "and nothing is carried");
    }

    #[test]
    fn a_program_that_is_not_there_is_a_failure_naming_it() {
        let mut request = cat(b"x");
        request.program = PathBuf::from("/nonexistent/quorra-no-such-program");
        let Ok(result) = execute(&request) else {
            panic!("a missing program is a result, not an error");
        };
        let ToolOutcome::Failed(sentence) = &result.outcome else {
            panic!("a missing program fails");
        };
        assert!(sentence.contains("quorra-no-such-program"), "{sentence}");
    }

    #[test]
    fn the_declining_status_is_not_a_failure() {
        let mut request = cat(b"x");
        request.program = PathBuf::from("/bin/sh");
        request.args = vec!["-c".to_owned(), format!("exit {DECLINED}")];
        let Ok(result) = execute(&request) else {
            panic!("sh could not be run");
        };
        // Exit 69 with nothing produced is the tool saying *not this one*, which is what the
        // site's `on-failure` answers.
        assert_eq!(result.outcome, ToolOutcome::Declined);
    }

    #[test]
    fn the_working_directory_does_not_outlive_the_invocation() {
        // The guard is what makes this true on every early return, so the test asks the
        // filesystem — and it asks about **this** invocation's directory rather than about the
        // temporary directory as a whole, which other tests of this file are using at the same
        // time. The program is told the input's path in `{in}` and echoes it back, so the test
        // learns the name of the directory it is to find gone.
        let mut request = cat(b"x");
        request.program = PathBuf::from("/bin/sh");
        request.args = vec!["-c".to_owned(), "printf %s \"{in}\"".to_owned()];
        let Ok(result) = execute(&request) else {
            panic!("sh could not be run");
        };
        let told = String::from_utf8_lossy(&result.output).into_owned();
        let input = Path::new(&told);
        assert!(
            input.file_name().is_some_and(|name| name == "in"),
            "the program was given a path inside the directory this executor made: {told}"
        );
        let workspace = input.parent().expect("the working directory");
        assert!(
            !workspace.exists(),
            "the directory is gone with the invocation: {}",
            workspace.display()
        );
    }

    #[test]
    fn a_program_that_overruns_its_timeout_is_killed_and_named() {
        let mut request = cat(b"x");
        request.program = PathBuf::from("/bin/sh");
        request.args = vec!["-c".to_owned(), "sleep 30".to_owned()];
        request.bounds.timeout = Duration::from_millis(200);
        let Ok(result) = execute(&request) else {
            panic!("sh could not be run");
        };
        let ToolOutcome::Failed(sentence) = &result.outcome else {
            panic!("an overrun is a failure");
        };
        assert!(sentence.contains("timeout"), "{sentence}");
    }
}
