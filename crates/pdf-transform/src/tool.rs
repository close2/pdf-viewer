//! What a remedy asks an external program for, and what came back — both as data.
//!
//! # The seam, and the owner's answer it is built from
//!
//! `doc/questions/A54` (2026-09-12) settled the question `doc/rfc/0007` section 4.4 put. The owner
//! doubted the recommendation in terms that name the user this design is for — if a configuration
//! says how a PDF should be converted to PDF/A, a normal user would expect it just to happen — and
//! asked whether the caller is our own converter program. It is, in every shape this project ships,
//! and with that said the owner chose the option named **request + shared executor**. The owner's
//! words are in `doc/questions/A54`; this paragraph is the round's reading of them. So:
//!
//! - [`crate::apply`] **never spawns a process**. Where a configured remedy needs a program, it
//!   returns a [`ToolRequest`] — the program, the arguments, the input bytes, the expected media
//!   type and the bounds — in [`crate::Report::requested`], and the requirement stays refused for
//!   that pass with [`crate::archive::Because::AwaitingTool`].
//! - The caller runs the requests through [`crate::executor::execute`], the one module in this tree that
//!   spawns anything, and calls `apply` again with the results in the plan. One command; the
//!   remedies happen.
//! - **A request and its result are data**, which is what keeps RFC 0002 section 5's purity and
//!   its section 9's determinism: given the same [`ToolOutputs`], the second pass is a function of
//!   its inputs and nothing else. A test replays a recorded output and gets the same bytes
//!   (`tests/archive.rs`, `a_replayed_tool_output_converts_to_the_same_bytes`).
//!
//! # Why the input never reaches the arguments
//!
//! Section 4.1 of the RFC: `args` may hold placeholders from a fixed vocabulary and **nothing
//! taken from the document**. A shell is never used — the program and its argument vector are
//! passed to the operating system directly — so there is no word splitting and no metacharacter
//! to escape. The document's bytes travel in [`ToolRequest::input`] and reach the program as a
//! file the executor writes into a directory it made, or on standard input; the program's own name
//! for that file is the executor's, never the document's.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// How a tool hands its result back.
///
/// `doc/rfc/0007` section 4.1: *the result comes back on stdout, or in `{out}`*, and which of the
/// two is a property of the tool rather than of the site. It is read off the declaration instead
/// of being a key of its own: **a tool whose `args` name `{out}` delivers its result there, and
/// one that does not delivers it on standard output.** A key saying the same thing a second time
/// would be a key an operator could contradict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    /// Everything the program wrote to standard output.
    Stdout,
    /// The one file the program left in the directory `{out}` named.
    ///
    /// A **directory** rather than a file path, because the shape that made this key necessary is
    /// `soffice --outdir {out} {in}`, which chooses the output's name itself. The executor makes
    /// the directory, and the result is the single file in it afterwards: none is a tool that
    /// produced nothing, and more than one is a tool whose result this cannot identify — both are
    /// failures named rather than guessed at.
    OutputDirectory,
}

/// The ceilings an external program runs under.
///
/// `doc/rfc/0007` section 4.3, and its sentence is why neither has a default: *the right value is a
/// property of the tool and a wrong default is worse than an absent one*. A `[tool.…]` block
/// stating neither is a configuration error naming the missing key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    /// How long the program may run before it is killed.
    pub timeout: Duration,
    /// The most bytes of result that will be read. A program that produces more is a failure
    /// named, never a truncated result passed off as whole.
    pub output_limit: u64,
}

/// One external program, as a `[tool.…]` block declares it.
///
/// **The warning `doc/questions/A56` asks for lives where this is written** — in the configuration
/// format's documentation and in the shipped profiles, at the `[tool.…]` block itself — and it is
/// [`crate::archive::UNTRUSTED_INPUT_WARNING`]. No confinement is offered in this version: the
/// owner chose *no offer in the first version, warn at the configuration site*, and
/// [`crate::executor::execute`] is where a per-tool confinement would go if a real deployment ever asks for
/// one with its own program in front of us.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    /// The name the configuration declares it under, and a site references it by.
    pub name: String,
    /// The program, as the operator wrote it. Executed directly; never through a shell.
    pub program: PathBuf,
    /// The argument vector, with section 4.1's placeholders still in it.
    pub args: Vec<String>,
    /// The media type the tool promises to produce, checked against what it returns.
    pub expects: String,
    /// The ceilings it runs under.
    pub bounds: Bounds,
}

impl Tool {
    /// Where this tool's result comes back, read off its own arguments.
    ///
    /// [`Delivery`]'s own comment is the argument: a tool that is given `{out}` writes there, and
    /// one that is not writes to standard output. Nothing else has to be stated, and so nothing
    /// else can be stated wrongly.
    #[must_use]
    pub fn delivery(&self) -> Delivery {
        if self.args.iter().any(|argument| argument.contains("{out}")) {
            Delivery::OutputDirectory
        } else {
            Delivery::Stdout
        }
    }
}

/// One invocation `apply` asks its caller to perform.
///
/// Every field is data: nothing here is a handle, a path this crate opened, or a closure. That is
/// what lets a recorded request and its recorded result stand in for a program that is not on this
/// machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRequest {
    /// What a result is filed under in [`ToolOutputs`].
    ///
    /// Stable across passes and derived from the document rather than from the order of a walk:
    /// the site's requirement identifier, the object number of the stream the input came from, and
    /// its generation. So a caller that executes the first pass's requests and applies again finds
    /// each result where the second pass looks for it, and a recorded output checked into a test
    /// stays valid as long as the fixture does.
    pub id: String,
    /// The refusal site this answers — a `pdf_archive` requirement identifier.
    pub site: String,
    /// The `[tool.…]` block's name.
    pub tool: String,
    /// The program to run.
    pub program: PathBuf,
    /// The argument vector. `{site}` and `{target}` are already substituted; `{in}` and `{out}`
    /// are the executor's to fill, because only the executor has a directory.
    pub args: Vec<String>,
    /// The document-derived bytes the program is to read.
    pub input: Arc<[u8]>,
    /// What the program promises to produce.
    pub expects: String,
    /// Its ceilings.
    pub bounds: Bounds,
    /// Where its result comes back.
    pub delivery: Delivery,
    /// What the input is, for a report a person reads — an embedded file's own name, say.
    pub subject: String,
}

/// What an invocation did.
///
/// `doc/rfc/0007` section 4.1's three exit classes, as a type: *`0` means it produced a result; a
/// documented code means I decline, take the fallback; anything else is a failure.*
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolOutcome {
    /// Exit status 0, and what came back is the result.
    Produced,
    /// [`DECLINED`], which is the tool saying *not this one* rather than failing.
    Declined,
    /// Anything else: a non-zero status, a timeout, a result past the limit, a program that could
    /// not be run at all. The sentence says which, for the report.
    Failed(String),
}

/// The exit status a tool declines with.
///
/// `doc/rfc/0007` section 4.1 asks for "a documented code" and does not pick one; this is the pick,
/// and it is a choice rather than a derivation. **69** is outside the range a shell reserves for
/// signals (128 and above) and outside `sysexits.h`'s meanings that a tool might produce by
/// accident from its own libraries — it is `EX_UNAVAILABLE`, whose sense is nearest to *I cannot do
/// this one*. The value is documented where an operator declares a tool, because a tool author has
/// to know it to use it.
pub const DECLINED: i32 = 69;

/// What one invocation returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResult {
    /// The [`ToolRequest::id`] this answers.
    pub id: String,
    /// What it did.
    pub outcome: ToolOutcome,
    /// The bytes it produced. Empty unless the outcome is [`ToolOutcome::Produced`].
    pub output: Arc<[u8]>,
    /// What it wrote to standard error, truncated to [`MOST_STDERR`] with the truncation stated.
    ///
    /// Section 4.1: *a tool's own explanation of why it declined is the most useful thing a report
    /// can carry*.
    pub stderr: String,
    /// The program that was actually run, as the operating system resolved it.
    ///
    /// Section 4.4 asks a tool-invoking conversion to record "each tool's name, its resolved
    /// program path, and a digest of what it returned — so that a re-run can be *checked* even
    /// though it cannot be *guaranteed*". This is the second of the three.
    pub program: PathBuf,
    /// The SHA-256 of [`Self::output`], lower-case hexadecimal. The third.
    pub digest: String,
}

/// How many bytes of a tool's standard error the report carries.
///
/// A bound rather than a reading (`CLAUDE.md` principle 3): a program's diagnostics are unbounded
/// and a report is prose. Where the cut is made it is stated in the string itself, so nobody reads
/// a truncated message as a whole one.
pub const MOST_STDERR: usize = 4096;

/// Results the caller recorded, for a pass that is not to spawn anything.
///
/// **The replay of RFC 0002 section 9.** A conversion handed a full set of these runs to
/// completion without a process being created, which is what makes a tool-invoking conversion
/// testable at all: the gate ships the tool's recorded output beside the fixture and asserts the
/// bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolOutputs {
    /// One result per [`ToolRequest::id`].
    results: BTreeMap<String, ToolResult>,
}

impl ToolOutputs {
    /// Nothing recorded, which is what every conversion that names no tool uses.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one result, replacing any held under the same identifier.
    pub fn insert(&mut self, result: ToolResult) {
        self.results.insert(result.id.clone(), result);
    }

    /// The result recorded for a request, where one is.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&ToolResult> {
        self.results.get(id)
    }

    /// Whether nothing at all has been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// Every result, in identifier order.
    pub fn iter(&self) -> impl Iterator<Item = &ToolResult> {
        self.results.values()
    }
}

/// The placeholders section 4.1 admits in an argument.
///
/// `{in}` and `{out}` are the executor's, because only it has a directory; `{site}` and `{target}`
/// are substituted where the request is built, because both are the *configuration's* own words
/// rather than the document's.
///
/// **`{media-type}` is not here, and that is a decision rather than an omission.** The RFC lists
/// it, and at the one site this version builds a request for — an embedded file that is not itself
/// PDF/A — the only media type available is the attachment's own `/Subtype`, which comes from the
/// document; section 4.1's other rule is that nothing document-derived reaches `args`. So a
/// configuration naming it is a configuration error saying exactly that, rather than a placeholder
/// quietly filled from a place the RFC forbids. `doc/adr/1019` has the argument.
pub const PLACEHOLDERS: [&str; 4] = ["{in}", "{out}", "{site}", "{target}"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recorded_result_is_found_under_its_requests_identifier() {
        let mut outputs = ToolOutputs::new();
        assert!(outputs.is_empty());
        outputs.insert(ToolResult {
            id: "a-site/12/0".to_owned(),
            outcome: ToolOutcome::Produced,
            output: Arc::from(&b"%PDF-2.0"[..]),
            stderr: String::new(),
            program: PathBuf::from("/usr/bin/true"),
            digest: "0".repeat(64),
        });
        assert!(!outputs.is_empty());
        assert_eq!(
            outputs.get("a-site/12/0").map(|held| held.id.as_str()),
            Some("a-site/12/0")
        );
        assert!(outputs.get("another").is_none());
    }
}
