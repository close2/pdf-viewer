//! Derives files from a document: pages as images, embedded images, embedded files.
//!
//! ```sh
//! pdf-transform render      in.pdf --dpi 150 -o 'page-%d.png'
//! pdf-transform render      in.pdf --pages 7 --scale-to 1600 -o -
//! pdf-transform images      in.pdf --pages 1-10 --min-pixels 32 -o 'img-%d.png'
//! pdf-transform images      in.pdf --list --report=json
//! pdf-transform images      in.pdf --native -o 'img-%d'
//! pdf-transform attachments in.pdf --list
//! pdf-transform attachments in.pdf --save-all -o dir/
//! pdf-transform attachments in.pdf --save NAME -o file.bin
//! pdf-transform attachments in.pdf --attach report.csv --description 'Q3' -o out.pdf
//! pdf-transform attachments in.pdf --attach data.csv --to-page 3 --icon Graph -o out.pdf
//! pdf-transform attachments in.pdf --remove report.csv -o out.pdf
//! pdf-transform render      in.pdf --page-box media --no-annotations -o 'page-%d.png'
//! pdf-transform images      in.pdf --no-mask -o 'img-%d.png'
//! pdf-transform split       in.pdf -o 'page-%d.pdf'
//! pdf-transform split       in.pdf --every 10 -o 'part-%d.pdf'
//! pdf-transform split       in.pdf --pages 1-3,7-end -o 'sel-%d.pdf'
//! pdf-transform split       in.pdf --at-bookmarks=1 -o '%d-%t.pdf'
//! pdf-transform merge       a.pdf b.pdf -o out.pdf
//! pdf-transform merge       a.pdf:1-5 b.pdf:end-1 -o out.pdf
//! pdf-transform merge       --collate a.pdf b.pdf -o out.pdf
//! pdf-transform pages       in.pdf --delete r1 --rotate +90:1-end -o out.pdf
//! pdf-transform optimize    in.pdf -o out.pdf
//! pdf-transform optimize    in.pdf --object-streams disable --recompress none -o out.pdf
//! pdf-transform archive     in.pdf --to 4 -o out.pdf
//! pdf-transform archive     in.pdf --to 2b --authorise image-smoothing -o out.pdf
//! pdf-transform archive     in.pdf --to 4 --output-intent-profile press.icc -o out.pdf
//! ```

//!
//! **Diagnostics go to stderr, always.** stdout carries bytes (under `-o -`) or the report, never
//! prose — the same discipline as `tools/pdf-retrieve`. The exit status is RFC 0002 section 4.4's:
//! 0 clean, 2 the file defeated us, 3 written with warnings, 4 refused by name, 1 a usage error.
//!
//! **This program is the first consumer of `pdf_transform`'s seam and nothing more**: it turns
//! argv into a [`Plan`], opens paths into [`Source`]s and [`Sinks`], and prints what
//! [`pdf_transform::apply`] reports. Everything a document means is decided below it.
//!
//! **Passwords never appear on the command line.** An argv password is visible in `/proc` and in
//! every shell history, so there is no `--password` flag — deliberately, and this sentence is
//! why its absence is a decision. `--password-fd <n>` reads one line from an open descriptor,
//! which is what a script hands over; an interactive prompt that suppresses echo needs a
//! terminal-mode dependency this tree has not taken, and `doc/todo/57` names it.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line tool whose entire output is a report"
)]

use std::collections::BTreeMap;
use std::io::{BufRead as _, IsTerminal as _, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use pdf_model::icc::Identification;
use pdf_transform::archive::{
    ArchivePlan, Authorisations, Configuration, ExternalData, Loss, sites,
};
use pdf_transform::attachments::{Action, AttachmentsPlan, OnPage, Payload, parse_iso_8601};
use pdf_transform::executor::execute;
use pdf_transform::images::ImagesPlan;
use pdf_transform::merge::{Input, MergePlan};
use pdf_transform::optimize::OptimizePlan;
use pdf_transform::pages::{Angle, Edit, PagesPlan};
use pdf_transform::pattern::Pattern;
use pdf_transform::range::Selection;
use pdf_transform::redact::RedactPlan;
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing, parse_boundary};
use pdf_transform::split::{Pieces, SplitPlan};
use pdf_transform::update::{INFORMATION_KEYS, InfoEntry};

use pdf_syntax::serialize::{ObjectStreams, Streams};
use pdf_transform::{
    Access, Budget, Exit, Level, Listed, Plan, Policy, Protect, Refusal, Report, Secret, Sinks,
    Source, apply_protected,
};

/// What went wrong before or while applying the plan.
#[derive(Debug, thiserror::Error)]
enum Failure {
    /// The caller got the arguments wrong; usage is printed.
    #[error("{0}")]
    Usage(String),
    /// A path could not be read.
    #[error("{0}: cannot be read ({1})")]
    Unreadable(PathBuf, std::io::Error),
    /// The password descriptor could not be read.
    #[error("--password-fd {0}: cannot be read ({1})")]
    Password(u32, std::io::Error),
    /// The terminal the question was put on could not be read from.
    #[error("--restrictions=ask: the answer could not be read ({0})")]
    Answer(String),
    /// The seam refused.
    #[error("{0}")]
    Refused(#[from] Refusal),
}

impl Failure {
    /// The exit status.
    fn exit(&self) -> Exit {
        match self {
            Self::Usage(_) => Exit::Usage,
            Self::Unreadable(..) | Self::Password(..) | Self::Answer(_) => Exit::Error,
            Self::Refused(refusal) => refusal.exit(),
        }
    }
}

/// `CLAUDE.md` principle 3's *ask* level, on a command line — the first of ADR 0874's two round
/// trips, asked here and answered on the terminal.
///
/// **This level used to be a usage error**, on the argument RFC 0002 section 13's fourth open
/// question makes: "a pipe cannot 'ask'". A pipe still cannot, and that half is unchanged — with
/// no terminal on standard input the level is left alone and `apply` answers it with
/// [`pdf_transform::Refusal::Unanswered`], which is the honest degradation and not a silent
/// proceed. What was wrong was the *other* half: this program is not always a pipe, and the
/// suite already draws the distinction for §7.6.4.1's password ("interactive, the default when a
/// document refuses and stdin is a tty"). So where there is a terminal the question is put on it.
///
/// The question is asked before anything is written, of every document the plan reads, and a
/// `yes` lowers the run to `Level::Off` — which is what a person consenting to the operation has
/// chosen for it, and the level `CLAUDE.md` says "shall always be possible". A `no` is
/// [`Exit::Refused`]'s own status with the document's reasons, because a question declined is
/// not the same event as a policy refusing.
///
/// Diagnostics go to stderr, always — the whole file's rule — so the question does too; stdout
/// carries bytes.
fn ask_before_the_operation(
    plan: &Plan,
    sources: &[Source],
    policy: &mut Policy,
    budget: &Budget,
) -> Result<(), Failure> {
    if policy.restrictions != Level::Ask || !std::io::stdin().is_terminal() {
        return Ok(());
    }
    let Some(operation) = plan.operation() else {
        return Ok(());
    };
    let mut asked = false;
    for at in plan.sources() {
        let Some(source) = sources.get(at) else {
            continue;
        };
        let document = source.document(budget.limits)?;
        let consulted = pdf_transform::consult(Level::Ask, &document, operation);
        let Some(question) = consulted.question() else {
            continue;
        };
        eprint!("{question} [y/N] ");
        std::io::stderr().flush().ok();
        let mut answer = String::new();
        std::io::stdin()
            .lock()
            .read_line(&mut answer)
            .map_err(|error| Failure::Answer(error.to_string()))?;
        if !matches!(answer.trim(), "y" | "Y" | "yes" | "Yes") {
            return Err(Failure::Refused(Refusal::Declined {
                operation: consulted.operation(),
                reasons: consulted.reasons().to_owned(),
            }));
        }
        asked = true;
    }
    // Only where somebody actually said yes. A run in which nothing was restricted stays at
    // `Ask`, so that a document reached later — a merge's second input — is still asked about.
    if asked {
        policy.restrictions = Level::Off;
    }
    Ok(())
}

/// The arguments, read once.
struct Arguments {
    /// The verb.
    verb: String,
    /// The positional arguments after it: the input file.
    positional: Vec<String>,
    /// Every `--flag` and `--flag value` / `--flag=value`, in order.
    flags: Vec<(String, Option<String>)>,
}

/// The flags that take a value; every other flag is a switch.
const VALUED: &[&str] = &[
    "-o",
    "--output",
    "--pages",
    "--dpi",
    "--scale-to",
    "--format",
    "--min-pixels",
    "--save",
    "--password-fd",
    "--encrypt-owner-fd",
    "--encrypt-user-fd",
    "--restrictions",
    "--report",
    "--max-pixels",
    "--info",
    "--page-box",
    "--attach",
    "--name",
    "--description",
    "--date",
    "--to-page",
    "--rect",
    "--icon",
    "--remove",
    "--every",
    "--delete",
    "--rotate",
    "--move",
    "--insert",
    "--object-streams",
    "--recompress",
    "--compression-level",
    "--images",
    "--to",
    "--authorise",
    "--output-intent-profile",
    "--config",
    "--font",
];

/// The flags whose value is optional and, when given, is written inline with `=`.
///
/// `--at-bookmarks[=depth]` is the only one and RFC 0002 section 6.1 spells it that way. It
/// cannot be in [`VALUED`]: a flag that consumed the next argument could not tell
/// `--at-bookmarks in.pdf` from a depth.
const OPTIONAL: &[&str] = &["--at-bookmarks"];

/// Every flag this program knows, so an unknown one is a usage error rather than ignored.
const KNOWN: &[&str] = &[
    "-o",
    "--output",
    "--pages",
    "--dpi",
    "--scale-to",
    "--format",
    "--min-pixels",
    "--list",
    "--native",
    "--no-mask",
    "--page-box",
    "--no-annotations",
    "--save-all",
    "--save",
    "--attach",
    "--name",
    "--description",
    "--date",
    "--to-page",
    "--rect",
    "--icon",
    "--remove",
    "--every",
    "--at-bookmarks",
    "--collate",
    "--no-substitute",
    "--font",
    "--delete",
    "--rotate",
    "--move",
    "--insert",
    "--no-prune",
    "--object-streams",
    "--recompress",
    "--compression-level",
    "--linearize",
    "--images",
    "--to",
    "--authorise",
    "--output-intent-profile",
    "--config",
    "--remedy-sites",
    "--claim-conformance",
    "--depart-from-the-standard",
    "--resolve-external-data",
    "--password-fd",
    "--encrypt-owner-fd",
    "--encrypt-user-fd",
    "--restrictions",
    "--report",
    "--max-pixels",
    "--strict",
    "--quiet-warnings",
    "--help",
    "-h",
];

impl Arguments {
    /// Reads argv.
    fn read() -> Result<Self, Failure> {
        let mut arguments = std::env::args().skip(1);
        let verb = arguments.next().unwrap_or_default();
        let mut positional = Vec::new();
        let mut flags = Vec::new();
        while let Some(argument) = arguments.next() {
            if !argument.starts_with('-') || argument == "-" {
                positional.push(argument);
                continue;
            }
            let (name, inline) = match argument.split_once('=') {
                Some((name, value)) => (name.to_owned(), Some(value.to_owned())),
                None => (argument.clone(), None),
            };
            if !KNOWN.contains(&name.as_str()) {
                return Err(Failure::Usage(format!("unknown option {name:?}")));
            }
            let value = if VALUED.contains(&name.as_str()) {
                match inline {
                    Some(value) => Some(value),
                    None => Some(
                        arguments
                            .next()
                            .ok_or_else(|| Failure::Usage(format!("{name} takes a value")))?,
                    ),
                }
            } else if OPTIONAL.contains(&name.as_str()) {
                inline
            } else if inline.is_some() {
                return Err(Failure::Usage(format!("{name} takes no value")));
            } else {
                None
            };
            flags.push((name, value));
        }
        Ok(Self {
            verb,
            positional,
            flags,
        })
    }

    /// Whether a switch was given.
    fn switch(&self, name: &str) -> bool {
        self.flags.iter().any(|(flag, _)| flag == name)
    }

    /// The last value a flag was given, under either of its spellings.
    fn value(&self, names: &[&str]) -> Option<&str> {
        self.flags
            .iter()
            .rev()
            .find(|(flag, _)| names.contains(&flag.as_str()))
            .and_then(|(_, value)| value.as_deref())
    }

    /// Every value a repeatable flag was given, in the order they were written.
    ///
    /// `--info` is the one flag a caller may state more than once and mean all of them:
    /// §14.3.3's Table 349 has nine keys and a merge may state any of them, so the last-wins
    /// rule [`Self::value`] applies to every other flag would silently drop eight of nine.
    fn every(&self, name: &str) -> Vec<&str> {
        self.flags
            .iter()
            .filter(|(flag, _)| flag == name)
            .filter_map(|(_, value)| value.as_deref())
            .collect()
    }

    /// A parsed value, with the flag named in the error.
    fn parsed<T: std::str::FromStr>(&self, names: &[&str]) -> Result<Option<T>, Failure>
    where
        T::Err: std::fmt::Display,
    {
        self.value(names)
            .map(|text| {
                text.parse()
                    .map_err(|error| Failure::Usage(format!("{} {text:?}: {error}", names[0])))
            })
            .transpose()
    }
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(exit) => std::process::ExitCode::from(exit.code()),
        Err(failure) => {
            // Usage above the message wherever the status is 1 — the seam's own usage refusals
            // (a pattern that cannot name the outputs) included.
            if failure.exit() == Exit::Usage {
                eprint!("{USAGE}");
            }
            eprintln!("error: {failure}");
            std::process::ExitCode::from(failure.exit().code())
        }
    }
}

/// Reads the arguments, builds the plan, applies it, prints the report.
fn run() -> Result<Exit, Failure> {
    let arguments = Arguments::read()?;
    if arguments.switch("--help")
        || arguments.switch("-h")
        || matches!(arguments.verb.as_str(), "help" | "--help" | "-h")
    {
        print!("{USAGE}");
        return Ok(Exit::Success);
    }
    if arguments.switch("--remedy-sites") {
        print_remedy_sites(&arguments)?;
        return Ok(Exit::Success);
    }
    let output = arguments.value(&["-o", "--output"]);
    let to_stdout = output == Some("-");
    let json = match arguments.value(&["--report"]) {
        None => false,
        Some("json") => true,
        Some(other) => {
            return Err(Failure::Usage(format!(
                "--report takes json, not {other:?}"
            )));
        }
    };
    if json && to_stdout {
        return Err(Failure::Usage(
            "--report=json and -o - both want stdout".to_owned(),
        ));
    }
    let plan = plan(&arguments, output)?;
    let sources = open_inputs(&arguments, &plan)?;

    let mut policy = Policy {
        restrictions: match arguments.value(&["--restrictions"]) {
            None => Level::Off,
            Some(word) => Level::parse(word).ok_or_else(|| {
                Failure::Usage(format!(
                    "--restrictions takes off, on, ask or warn, not {word:?}"
                ))
            })?,
        },
    };
    let mut budget = Budget::default();
    if let Some(max_pixels) = arguments.parsed::<u64>(&["--max-pixels"])? {
        budget.max_pixels = max_pixels;
    }
    ask_before_the_operation(&plan, &sources, &mut policy, &budget)?;

    let protect = protection(&arguments, &plan)?;
    let borrowed: Vec<&Source> = sources.iter().collect();

    let mut plan = plan;
    let mut passes = 0_usize;
    let report = loop {
        let sinks: &dyn Sinks = if to_stdout {
            &StdoutSinks::default()
        } else {
            &FileSinks
        };
        let report = apply_protected(&plan, &borrowed, sinks, &policy, &budget, protect.as_ref())?;
        // **`doc/questions/A54`, in one command.** The owner's doubt about the recommendation was
        // that "a normal user would expect it just to happen", and this loop is what makes that
        // true while `apply` itself starts nothing: a pass that needs an external program returns
        // the invocations as data, this program runs them through the one shared executor, puts
        // the results in the plan and applies again. The caller *is* our own converter program.
        if report.requested.is_empty() {
            // **The same two-pass shape, for the one other thing `apply` will not do itself.**
            // A conversion whose source keeps a stream's data outside the file needs those bytes
            // handed to it, and `apply` opens no path (`doc/questions/A54`, RFC 0002 section 9).
            // So a pass names the streams, this program resolves what its own rule permits, and
            // the next pass is a pure function of what came back (`doc/adr/1199`).
            if resolve_the_external_data(&arguments, &report, &mut plan) {
                passes = passes.saturating_add(1);
                if passes > MOST_TOOL_PASSES {
                    break report;
                }
                continue;
            }
            break report;
        }
        passes = passes.saturating_add(1);
        if passes > MOST_TOOL_PASSES {
            return Err(Failure::Usage(format!(
                "this configuration still asks for a tool after {MOST_TOOL_PASSES} passes, which \
                 means a request is being asked for and never answered. That is a defect rather \
                 than a configuration mistake; the bound is here so it stops rather than runs \
                 forever (CLAUDE.md principle 3)"
            )));
        }
        if !carry_out_the_requests(&report, &mut plan)? {
            break report;
        }
    };

    for warning in &report.warnings {
        match warning.page {
            Some(page) => eprintln!("warning: page {page}: {}", warning.detail),
            None => eprintln!("warning: {}", warning.detail),
        }
    }
    for declined in &report.refused {
        eprintln!("refused: {}: {}", declined.subject, declined.detail);
    }
    if json {
        print!("{}", report.to_json().render());
    } else if !to_stdout {
        // The conversion's report is an output rather than a listing (`doc/adr/0927`), and it
        // goes to stderr with the other diagnostics: stdout carries bytes or the JSON report.
        if let Some(conversion) = &report.archive {
            eprint!("{}", conversion.render());
        }
        print_listing(&report);
    }
    Ok(report.exit(
        arguments.switch("--strict"),
        arguments.switch("--quiet-warnings"),
    ))
}

/// How many times a plan may come back asking for a tool before this program refuses to try again.
///
/// A request's identifier is a function of the document, so the set a conversion asks for is stable
/// and one extra pass is all a correct implementation ever needs. The bound is a ceiling on a
/// defect, not on a configuration: `CLAUDE.md` principle 3's rule that a loop over untrusted input
/// has an explicit budget, applied to a loop whose input is our own.
const MOST_TOOL_PASSES: usize = 4;

/// Runs every invocation the last pass asked for, and puts the results in the plan.
///
/// **The one loop that starts a process, in the one program that ships this verb today.**
/// `doc/questions/A54` puts the executor in every consumer this project ships — this command-line
/// program, the KIO worker and the FUSE filesystem — and all three reach `pdf_transform::apply`
/// the same way, so this loop is what the other two gain when RFC 0003's round comes:
/// `crates/pdf-vfs`'s commit path is where theirs goes, because `pdf-fuse` and `pdf-vfs-ffi` both
/// hold their converter through it rather than calling `apply` themselves.
///
/// `Ok(false)` where nothing new was learned — every request already had a result — which stops the
/// loop rather than repeating a pass that would ask for the same thing again.
fn carry_out_the_requests(report: &Report, plan: &mut Plan) -> Result<bool, Failure> {
    let Plan::Archive(archive) = plan else {
        // No other verb asks for one, and a verb that did would need its own field: a request is
        // returned by the plan that wants it, so there is nothing here to guess at.
        return Ok(false);
    };
    let mut new = false;
    for request in &report.requested {
        if archive.tool_outputs.get(&request.id).is_some() {
            continue;
        }
        eprintln!(
            "running the tool {:?} ({}) over {} — doc/rfc/0007 section 4.5: {}",
            request.tool,
            request.program.display(),
            request.subject,
            pdf_transform::archive::UNTRUSTED_INPUT_WARNING
        );
        let result = execute(request).map_err(|error| Failure::Usage(error.to_string()))?;
        archive.tool_outputs.insert(result);
        new = true;
    }
    Ok(new)
}

/// Reads the files a source's streams name, where the operator asked for it.
///
/// **The second thing `apply` will not do for itself, in the same shape as the first.**
/// ISO 19005-2 section 6.1.7.1 and ISO 19005-4 section 6.1.6.1 forbid a stream the keys that put
/// its data outside the file; nothing in the document supplies those bytes, and `apply` opens no
/// path (`doc/questions/A54`, RFC 0002 section 9). So a pass names the streams and this program —
/// the caller — reads what its own rule permits.
///
/// **The rule is `doc/adr/1155`'s, and it is narrow on purpose**: a name a document wrote is
/// resolved as a single path component against the directory the document itself is in, and
/// nowhere else. `../`, an absolute path and a drive-relative one are all refused by the same
/// check, and \u{a7}7.11.5's URL is refused because it is not a file name at all — fetching one
/// would be a network operation this program does not have and `CLAUDE.md` principle 3 will not
/// acquire. Every refusal is said out loud rather than leaving a user with a conversion that
/// quietly did nothing.
///
/// `false` where nothing new was resolved, which stops the loop rather than repeating a pass that
/// would ask for the same thing again. **No error of its own**: every refusal here is one name a
/// document wrote, said out loud and passed over, because a document that points at a file this
/// machine will not hand over is a document to refuse by requirement rather than a command line to
/// reject.
fn resolve_the_external_data(arguments: &Arguments, report: &Report, plan: &mut Plan) -> bool {
    if !arguments.switch("--resolve-external-data") {
        return false;
    }
    let Plan::Archive(archive) = plan else {
        // No other verb reads a stream's file specification, and a verb that did would name its
        // own field: what a plan needs is returned by the plan that needs it.
        return false;
    };
    let Some(conversion) = &report.archive else {
        return false;
    };
    let source = arguments.positional.first().map(|spec| input_spec(spec).0);
    let directory = source.as_deref().and_then(Path::parent);
    let mut resolved = false;
    for stream in &conversion.external_data {
        if archive.external_data.contains_key(&stream.at) {
            continue;
        }
        let named = match &stream.data {
            ExternalData::Named {
                shown,
                components,
                absolute,
            } => (shown, components, *absolute),
            ExternalData::AtUrl(url) => {
                eprintln!(
                    "--resolve-external-data: object {} names {url}, which is a URL rather than \
                     a file beside the document; this program performs no network request",
                    stream.at.number
                );
                continue;
            }
            // Nothing to read: the stream states the filter keys and no file, so the conversion
            // answers it out of the file's own bytes.
            ExternalData::NoneNamed | ExternalData::Unreadable => continue,
        };
        let path = match beside_the_document(directory, named.1, named.2) {
            Ok(path) => path,
            Err(refusal) => {
                eprintln!(
                    "--resolve-external-data: object {} names {}: {refusal}",
                    stream.at.number, named.0
                );
                continue;
            }
        };
        match std::fs::read(&path) {
            Ok(bytes) => {
                eprintln!(
                    "--resolve-external-data: object {} takes the {} byte(s) of {}",
                    stream.at.number,
                    bytes.len(),
                    path.display()
                );
                archive.external_data.insert(stream.at, Arc::from(bytes));
                resolved = true;
            }
            Err(error) => eprintln!(
                "--resolve-external-data: object {}: cannot read {}: {error}",
                stream.at.number,
                path.display()
            ),
        }
    }
    resolved
}

/// `doc/adr/1155`'s rule: one path component, against the directory the document is in.
///
/// Restated here rather than taken from `viewer_host::policy`, and the reason is a layer rather
/// than a preference: this crate is a batch job over documents no window has open, and it names
/// no part of the viewer's vocabulary. What the two share is the *rule*, which is two sentences
/// and is stated in both places with the ADR that decided it.
///
/// **The components are \u{a7}7.11.2.1's rather than this platform's**, because the clause makes
/// SOLIDUS "a generic component separator that shall be mapped to the appropriate
/// platform-specific separator": a name the standard reads as three components is three
/// components on a system whose own separator is something else, and splitting it here would ask
/// the wrong question. `pdf_model::file_spec::FileSpec` did the splitting; what is left is to
/// insist there was exactly one, that it is neither the directory itself nor its parent, and that
/// it is a name this system can spell.
fn beside_the_document(
    directory: Option<&Path>,
    components: &[Vec<u8>],
    absolute: bool,
) -> Result<PathBuf, String> {
    let directory = directory.ok_or_else(|| {
        "the document is not in a known directory, so there is nothing to resolve against"
            .to_owned()
    })?;
    if absolute {
        return Err(
            "this is an absolute file specification, and only a plain file name beside \
                    the document is read"
                .to_owned(),
        );
    }
    let [single] = components else {
        return Err(format!(
            "this file specification has {} component(s), and only a plain file name beside the \
             document is read",
            components.len()
        ));
    };
    let single = std::str::from_utf8(single)
        .map_err(|_| "this file name is not text this system can spell".to_owned())?;
    if single.is_empty() || single == "." || single == ".." {
        return Err(format!(
            "{single:?} is not a plain file name beside the document"
        ));
    }
    Ok(directory.join(single))
}

/// The plan the verb and its flags describe.
fn plan(arguments: &Arguments, output: Option<&str>) -> Result<Plan, Failure> {
    let pages = arguments
        .parsed::<Selection>(&["--pages"])?
        .unwrap_or_else(Selection::all);
    let names = |what: &str| -> Result<Pattern, Failure> {
        output
            .ok_or_else(|| Failure::Usage(format!("{what} needs -o <name>")))?
            .parse()
            .map_err(|error| Failure::Usage(format!("-o: {error}")))
    };
    match arguments.verb.as_str() {
        "render" => {
            let dpi = arguments.parsed::<f32>(&["--dpi"])?;
            let scale_to = arguments.value(&["--scale-to"]);
            let size = match (dpi, scale_to) {
                (Some(_), Some(_)) => {
                    return Err(Failure::Usage(
                        "--dpi and --scale-to are two answers to one question".to_owned(),
                    ));
                }
                (Some(dpi), None) if dpi > 0.0 && dpi.is_finite() => Sizing::Dpi(dpi),
                (Some(dpi), None) => {
                    return Err(Failure::Usage(format!(
                        "--dpi {dpi}: not a positive number"
                    )));
                }
                (None, Some(fit)) => sizing_from(fit)?,
                (None, None) => Sizing::Dpi(150.0),
            };
            let format = image_format(arguments)?;
            let page_box = match arguments.value(&["--page-box"]) {
                None => None,
                Some(word) => Some(parse_boundary(word).ok_or_else(|| {
                    Failure::Usage(format!(
                        "--page-box takes media, crop, bleed, trim or art, not {word:?}"
                    ))
                })?),
            };
            Ok(Plan::Render(RenderPlan {
                source: 0,
                pages,
                size,
                format,
                page_box,
                annotations: !arguments.switch("--no-annotations"),
                names: names("render")?,
                strips: None,
            }))
        }
        "images" => {
            let list_only = arguments.switch("--list");
            Ok(Plan::Images(ImagesPlan {
                source: 0,
                pages,
                min_pixels: arguments.parsed::<u64>(&["--min-pixels"])?.unwrap_or(0),
                list_only,
                native: arguments.switch("--native"),
                no_mask: arguments.switch("--no-mask"),
                format: image_format(arguments)?,
                names: if list_only {
                    "%d".parse()
                        .map_err(|error| Failure::Usage(format!("{error}")))?
                } else {
                    names("images")?
                },
            }))
        }
        "split" => split_plan(arguments, pages, names("split")?),
        "merge" => Ok(Plan::Merge(merge_plan(arguments, names("merge")?)?)),
        "pages" => Ok(Plan::Pages(PagesPlan {
            source: 0,
            edits: page_edits(arguments)?,
            names: names("pages")?,
        })),
        "optimize" => Ok(Plan::Optimize(optimize_plan(
            arguments,
            names("optimize")?,
        )?)),
        "redact" => Ok(Plan::Redact(RedactPlan {
            source: 0,
            names: names("redact")?,
        })),
        "archive" => Ok(Plan::Archive(archive_plan(arguments, names("archive")?)?)),
        "attachments" => Ok(Plan::Attachments(AttachmentsPlan {
            source: 0,
            action: attachments_action(arguments, output)?,
        })),
        "" => Err(Failure::Usage("no verb given".to_owned())),
        other => Err(Failure::Usage(format!("no such verb: {other:?}"))),
    }
}

/// `archive`: the target, and the losses the caller has authorised.
///
/// **`--to` has no default, deliberately.** Section 9 of `doc/pdf-a-conversion-limits.md` is
/// the user
/// whose deposit rule names one part and one level, for whom a converter guessing would be
/// producing a file against a target nobody asked for; and section 1.1 is the user with a free
/// choice, for whom the interesting sentence is "this cannot be PDF/A-2 and can be PDF/A-4f".
/// Neither is served by a default.
fn archive_plan(arguments: &Arguments, names: Pattern) -> Result<ArchivePlan, Failure> {
    let Some(word) = arguments.value(&["--to"]) else {
        return Err(Failure::Usage(
            "archive needs --to <target>: 2b, 2u, 2a, 4, 4f or 4e. There is no default, because \
             which part and level a document has to reach is the one thing this program cannot \
             work out for you"
                .to_owned(),
        ));
    };
    let target = pdf_archive::Target::parse(word).ok_or_else(|| {
        Failure::Usage(format!(
            "--to {word:?}: the targets are 2b, 2u, 2a, 4, 4f and 4e. Parts 1 and 3 are not \
             targets and will not become ones — doc/questions/A17"
        ))
    })?;
    let mut authorised = Authorisations::default();
    for (flag, value) in &arguments.flags {
        if flag != "--authorise" {
            continue;
        }
        let word = value.as_deref().unwrap_or_default();
        let loss = Loss::parse(word).ok_or_else(|| {
            Failure::Usage(format!(
                "--authorise {word:?}: the losses this converter can be authorised are {}",
                Loss::ALL
                    .iter()
                    .map(|loss| loss.word())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })?;
        authorised.authorise(loss);
    }
    let supplied_fonts = supplied_fonts(arguments)?;
    // `doc/rfc/0007`: the configuration is read by the *caller* and handed in as data, so `apply`
    // stays the pure function RFC 0002 section 5 tests. What it contributes is the losses its
    // `discard` remedies stand for — folded into the same `Authorisations` the flags build — and
    // the departures it names.
    let read = read_config(arguments, target, &mut authorised)?;
    Ok(ArchivePlan {
        source: 0,
        names,
        target,
        authorised,
        profile: output_intent_profile(arguments)?,
        substitute_fonts: !arguments.switch("--no-substitute"),
        supplied_fonts,
        departures: read.departures,
        claim_conformance: read.claim_conformance,
        derivations: read.derivations,
        supplies: read.supplies,
        preservations: read.preservations,
        resolutions: read.resolutions,
        tool_outputs: pdf_transform::tool::ToolOutputs::new(),
        // Empty until a pass says which streams keep their data outside the file; `run` resolves
        // what `--resolve-external-data` allows it to and applies again (`doc/adr/1199`).
        external_data: BTreeMap::new(),
    })
}

/// What `--config` contributed to the plan, beyond the authorisations it folded in.
struct FromConfig {
    /// The departures it names.
    departures: Vec<pdf_transform::archive::Departure>,
    /// Whether the output still claims the target (`--claim-conformance`, `A59`).
    claim_conformance: bool,
    /// The `derive` remedies it names, each with the tool it declares.
    derivations: Vec<pdf_transform::archive::Derivation>,
    /// The `supply` remedies it names.
    supplies: Vec<pdf_transform::archive::Supply>,
    /// The `preserve` remedies it names that append pages.
    preservations: Vec<pdf_transform::archive::Preservation>,
    /// The `preserve` remedies it names that fetch what a stream keeps outside the file.
    resolutions: Vec<pdf_transform::archive::Resolution>,
}

/// Reads every `--font <base-font>=<path>`, one program per `/BaseFont` the operator names.
///
/// **The file is read here, in the binary — the caller** — so `apply` opens no path (RFC 0002
/// section 5, `doc/questions/A54`). What reaches the conversion is bytes.
///
/// **Naming the file is the operator's statement about a licence, and that is the whole reason
/// this is a flag rather than a search.** ISO 32000-2 §9.9.1:
///
/// > One of the conditions may be that the font program cannot be embedded, in which case it
/// > should not be incorporated into a PDF file.
///
/// ISO 19005-2 section 6.2.11.4.1 admits only a program that may lawfully be embedded for
/// unlimited universal rendering. Nothing in the document says whether a given program may be,
/// and neither does the program on somebody's disk; the operator does. So the run records whose
/// authority it was, in the report and in the output's own `xmpMM:History`, and this program
/// never goes looking for a face by itself (`doc/adr/1209`, `doc/adr/1200` section 4).
fn supplied_fonts(arguments: &Arguments) -> Result<BTreeMap<String, Arc<[u8]>>, Failure> {
    let mut out: BTreeMap<String, Arc<[u8]>> = BTreeMap::new();
    for stated in arguments.every("--font") {
        let Some((base_font, path)) = stated.split_once('=') else {
            return Err(Failure::Usage(format!(
                "--font {stated:?}: the form is --font <base-font>=<path>, naming the /BaseFont \
                 the document states and a file holding the program to embed for it"
            )));
        };
        if base_font.is_empty() || path.is_empty() {
            return Err(Failure::Usage(format!(
                "--font {stated:?}: both halves are needed — the /BaseFont on the left of the \
                 equals sign and the font program's path on the right"
            )));
        }
        let path = PathBuf::from(path);
        let bytes =
            std::fs::read(&path).map_err(|error| Failure::Unreadable(path.clone(), error))?;
        eprintln!(
            "note: --font states that {} may lawfully be embedded for unlimited, universal \
             rendering (ISO 19005-2 section 6.2.11.4.1); that is your statement, not this \
             program's, and the report and the output's xmpMM:History record it as yours",
            path.display()
        );
        out.insert(base_font.to_owned(), Arc::from(bytes));
    }
    Ok(out)
}

/// Reads `--config <file>`, folds its built `discard` remedies into `authorised`, and returns its
/// departures and whether the output claims conformance.
///
/// **The file is read here, in the binary — the caller** — so `apply` opens no path (RFC 0002
/// section 5). A configuration that names a departure is refused unless `--depart-from-the-standard`
/// is also on the command line: `doc/rfc/0007` section 4.7.3 puts the operator's intent to go
/// against the standard at the call site rather than only in a file that can be inherited or copied
/// between teams. Sites whose remedy this version cannot yet carry out are named on stderr, so an
/// operator sees their intent was read rather than ignored.
fn read_config(
    arguments: &Arguments,
    target: pdf_archive::Target,
    authorised: &mut Authorisations,
) -> Result<FromConfig, Failure> {
    let claim = arguments.switch("--claim-conformance");
    let Some(path) = arguments.value(&["--config"]) else {
        return Ok(FromConfig {
            departures: Vec::new(),
            claim_conformance: claim,
            derivations: Vec::new(),
            supplies: Vec::new(),
            preservations: Vec::new(),
            resolutions: Vec::new(),
        });
    };
    let path = PathBuf::from(path);
    let text =
        std::fs::read_to_string(&path).map_err(|error| Failure::Unreadable(path.clone(), error))?;
    let config = Configuration::read(&text, target)
        .map_err(|error| Failure::Usage(format!("--config {}: {error}", path.display())))?;
    let departures = config.built_departures(target);
    if !departures.is_empty() && !arguments.switch("--depart-from-the-standard") {
        return Err(Failure::Usage(format!(
            "--config {} names a departure from ISO 19005, which produces a file that does not \
             conform on purpose. Add --depart-from-the-standard to say so at the call site — \
             doc/rfc/0007 section 4.7.3",
            path.display()
        )));
    }
    let folded = config.authorisations(target);
    for loss in Loss::ALL {
        if folded.grants(loss) {
            authorised.authorise(loss);
        }
    }
    for unbuilt in config.unbuilt(target) {
        eprintln!(
            "note: the configuration answers {:?} with `{}`, which this version does not carry out \
             yet; that site stays refused with the sentence it names",
            unbuilt.site,
            unbuilt.remedy.word()
        );
    }
    let derivations = config.derivations(target);
    let supplies = config.supplies(target);
    let preservations = config.preservations(target);
    let resolutions = config.resolutions(target);
    // **`doc/questions/A56`'s warning, where the operator meets it.** The answer put it at the
    // configuration site rather than in a security document nobody opens, and the two places an
    // operator meets a tool are the `[tool.…]` block they wrote and this line: a run that is about
    // to start somebody else's program over an untrusted document says so before it does.
    for tool in config.tools() {
        eprintln!(
            "note: {} declares the tool {:?} as {} — {}",
            path.display(),
            tool.name,
            tool.program.display(),
            pdf_transform::archive::UNTRUSTED_INPUT_WARNING
        );
    }
    Ok(FromConfig {
        departures,
        claim_conformance: claim,
        derivations,
        supplies,
        preservations,
        resolutions,
    })
}

/// `--remedy-sites --to <target>`: every refusal site the target binds, from the decision table.
///
/// `doc/rfc/0007` section 3.1: the list is generated from the same table the converter decides
/// from, so a site cannot exist undocumented and a configuration naming one that does not exist is
/// an error rather than a silently ignored line.
fn print_remedy_sites(arguments: &Arguments) -> Result<(), Failure> {
    let Some(word) = arguments.value(&["--to"]) else {
        return Err(Failure::Usage(
            "--remedy-sites needs --to <target>: a site's remedies depend on the target, so the \
             list is the target's — doc/rfc/0007 section 4.6"
                .to_owned(),
        ));
    };
    let target = pdf_archive::Target::parse(word).ok_or_else(|| {
        Failure::Usage(format!(
            "--to {word:?}: the targets are 2b, 2u, 2a, 4, 4f, 4e"
        ))
    })?;
    let sites = sites(target);
    println!(
        "{} refusal site(s) a configuration may answer for {target}. Every site's default is \
         `stop`; a remedy a target does not admit is an error naming both.",
        sites.len()
    );
    let mut takes_a_tool = 0_usize;
    let mut not_built = 0_usize;
    for site in &sites {
        // The gap is a property of the site rather than of the sentence printed for it: a site
        // whose catalogued remedy has no code behind it is one with no built `discard`, no tool to
        // derive from and no fact to supply. Counting the predicate rather than the words means the
        // trailer cannot disagree with the listing when a sentence is reworded (`doc/todo/66`).
        let built = site.built_discard.is_some()
            || site.takes_a_tool
            || site.takes_a_supplied_fact
            || site.takes_a_fetched_file
            || site.conditional.is_some();
        if !built {
            not_built = not_built.saturating_add(1);
        }
        let mut remedy = match site.built_discard {
            Some(loss) => format!("discard (authorises --authorise {})", loss.word()),
            None if site.takes_a_tool => {
                "derive, with `tool = \"<name>\"` and a [tool.<name>] block — never a default, and \
                 refused unless the configuration names the site AND the tool (doc/questions/A55); \
                 departable (doc/rfc/0007 section 4.7)"
                    .to_owned()
            }
            // One word, two facts: what the operator supplies differs per site, so the line
            // names the key that site actually reads rather than one key for both.
            None if site.takes_a_supplied_fact => {
                if site.requirement == "graphics/separations-of-one-name-agree" {
                    "supply, with `winner = \"first\"` or `winner = \"most-used\"` — the operator \
                     states which of the file's own definitions of an ink the archive means, \
                     reported and recorded as theirs (doc/adr/1188)"
                        .to_owned()
                } else {
                    "supply, with `media-types = { \".ext\" = \"type/subtype\" }` — the operator \
                     states what their own attachments are, reported and recorded as theirs"
                        .to_owned()
                }
            }
            // **`doc/adr/1209`.** A stream whose data the file keeps outside itself is brought
            // inside by bytes somebody fetched, and §7.11.5's URL is what the operator's own
            // program reaches: `--resolve-external-data` is this program's own rule and reads
            // only a plain file name beside the document (`doc/adr/1155`).
            None if site.takes_a_fetched_file => {
                "preserve, with `tool = \"<name>\"` and a [tool.<name>] block — the program is \
                 handed the file specification the document wrote, on standard input, and what \
                 it returns is written into the stream; every fetch is named in the report and \
                 recorded in the file's own xmpMM:History. --resolve-external-data reads a plain \
                 file name beside the document without any tool (doc/adr/1155)"
                    .to_owned()
            }
            None if site.departable => {
                "stop; discard/preserve/derive not built yet; departable (doc/rfc/0007 section 4.7)"
                    .to_owned()
            }
            None => "stop; the catalogued remedy is not built yet".to_owned(),
        };
        // **`doc/adr/1209`, in the listing an operator reads.** A site whose built answer the
        // decision table does not settle by itself still refuses documents, so the line says what
        // that answer waits on rather than leaving the site looking finished.
        if let Some(waits_on) = site.conditional {
            remedy.push_str("\n      ");
            remedy.push_str(waits_on.describe());
        }
        // **`doc/adr/1014`'s amendment, in the listing an operator reads.** A site may take more
        // than one remedy, and the page is the one that keeps what a `discard` at the same site
        // would lose — so it is named beside it rather than instead of it.
        if site.takes_a_page {
            remedy.push_str(
                "\n      preserve, with `placement = \"append\"` — the content is kept on a page \
                 appended to the document, which every target admits (doc/rfc/0007 section 4.6.1) \
                 and doc/adr/1014 permits; every page is named in the report and recorded in the \
                 file's own xmpMM:History",
            );
        }
        println!("  {} ({})\n      {remedy}", site.requirement, site.citation);
        if site.takes_a_tool || site.takes_a_fetched_file {
            // **`doc/questions/A56`.** The warning lives where an operator configures a tool rather
            // than in a security document nobody opens, and this listing is one of the two places
            // they meet one — the `[tool.…]` block they write is the other.
            println!(
                "      warning: {} — doc/rfc/0007 section 4.5",
                pdf_transform::archive::UNTRUSTED_INPUT_WARNING
            );
            takes_a_tool = takes_a_tool.saturating_add(1);
        }
    }
    if takes_a_tool > 0 {
        println!(
            "\n{takes_a_tool} of these sites can be answered with an external program. \
             {}: no confinement is offered for it, because a profile written against no \
             particular program is a guess (doc/questions/A56).",
            pdf_transform::archive::UNTRUSTED_INPUT_WARNING
        );
    }
    // **The program prints its own total**, which is what `doc/todo/66` asks for: a count taken
    // beside a program's output goes stale the moment the program's words change, and this one is
    // computed from the same walk it summarises.
    println!("\n{not_built} of {} sites not built yet.", sites.len());
    if let Some(path) = arguments.value(&["--config"]) {
        print_unbuilt_answers(Path::new(path), target, sites.len())?;
    }
    Ok(())
}

/// The second half of `doc/todo/66`'s done condition: what a profile answers and this version does
/// not carry out.
///
/// **It needs no document, and that is the finding rather than a shortcut.**
/// `Configuration::unbuilt` is a function of the profile and the target alone — it reads the
/// answers the file gives and asks which of them have code behind them — so a conversion prints
/// the same notes for every document it is handed. Counting them over a corpus would count the
/// corpus. `doc/todo/66`.
fn print_unbuilt_answers(
    path: &Path,
    target: pdf_archive::Target,
    bound: usize,
) -> Result<(), Failure> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| Failure::Unreadable(path.to_path_buf(), error))?;
    let config = Configuration::read(&text, target)
        .map_err(|error| Failure::Usage(format!("--config {}: {error}", path.display())))?;
    let named = config.name.clone().unwrap_or_else(|| {
        path.file_name().map_or_else(
            || path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        )
    });
    let unbuilt = config.unbuilt(target);
    println!();
    for answer in &unbuilt {
        println!(
            "  {:?} is answered with `{}`, which this version does not carry out yet",
            answer.site,
            answer.remedy.word()
        );
    }
    println!(
        "\n{} of {bound} sites answered with a remedy not carried out yet, for {named} at {target}.",
        unbuilt.len()
    );
    Ok(())
}

/// `--output-intent-profile <file>`: the destination profile an added output intent names.
///
/// **Read and checked here rather than per document**, because a profile that is not one is the
/// caller's mistake and belongs in usage rather than in a conversion report. What it is checked
/// against is what ISO 19005-2 section 6.2.3 and ISO 19005-4 section 6.2.3 require of a
/// destination profile: an output or a monitor profile, over grey, RGB or CMYK.
///
/// Its `cprt` tag is printed at the same time, and that is not decoration:
/// `doc/pdf-a-conversion-limits.md` section 10.1 records the ICC's own guidance that a profile's
/// terms of use live in its header's creator field and that tag, so a user embedding somebody
/// else's press profile is told whose it is before the run rather than after.
fn output_intent_profile(arguments: &Arguments) -> Result<Option<Arc<[u8]>>, Failure> {
    let Some(path) = arguments.value(&["--output-intent-profile"]) else {
        return Ok(None);
    };
    let path = PathBuf::from(path);
    let bytes = std::fs::read(&path).map_err(|error| Failure::Unreadable(path.clone(), error))?;
    let stated = Identification::read(&bytes).ok_or_else(|| {
        Failure::Usage(format!(
            "--output-intent-profile {}: this file is not an ICC profile",
            path.display()
        ))
    })?;
    let class = stated.class_name();
    let space = stated.space_name();
    if !matches!(class.as_str(), "prtr" | "mntr")
        || !matches!(space.as_str(), "GRAY" | "RGB" | "CMYK")
    {
        return Err(Failure::Usage(format!(
            "--output-intent-profile {}: ISO 19005 admits an output or a monitor profile over \
             grey, RGB or CMYK as a destination profile, and this one is a {class} profile over \
             {space}",
            path.display()
        )));
    }
    Ok(Some(bytes.into()))
}

/// `optimize`: the two knobs RFC 0002 section 6.5 names, and the one it defers.
///
/// The lossless default is every pass on. `--linearize` is refused rather than ignored, because
/// an ignored flag is a promise this program did not keep.
fn optimize_plan(arguments: &Arguments, names: Pattern) -> Result<OptimizePlan, Failure> {
    if arguments.switch("--linearize") {
        return Err(Failure::Usage(
            "--linearize: Annex F is excluded — CLAUDE.md's amended exclusion says \"Annex F \
             stays excluded until linearisation is separately ratified\", and RFC 0002 section \
             6.5 puts it in a phase of its own that may be declined permanently"
                .to_owned(),
        ));
    }
    if let Some(word) = arguments.value(&["--images"]) {
        return Err(Failure::Usage(format!(
            "--images {word:?}: lossy image optimisation needs a DCT encoder, which this tree \
             does not have (RFC 0002 section 13's second question, doc/stack.md); no flag here \
             re-encodes an image, deliberately"
        )));
    }
    let object_streams = match arguments.value(&["--object-streams"]) {
        None | Some("generate") => ObjectStreams::DEFAULT,
        Some("disable") => ObjectStreams::Disable,
        Some(other) => {
            return Err(Failure::Usage(format!(
                "--object-streams takes generate or disable, not {other:?}"
            )));
        }
    };
    let streams = match arguments.value(&["--recompress"]) {
        None | Some("all") => Streams::DEFAULT,
        Some("none") => Streams::Carry,
        Some(other) => {
            return Err(Failure::Usage(format!(
                "--recompress takes all or none, not {other:?}"
            )));
        }
    };
    let streams = match (streams, arguments.parsed::<u32>(&["--compression-level"])?) {
        (Streams::Carry, _) => Streams::Carry,
        (Streams::Recompress { .. }, Some(level)) if level > 9 => {
            return Err(Failure::Usage(
                "--compression-level takes zlib's 0 to 9".to_owned(),
            ));
        }
        (Streams::Recompress { .. }, Some(level)) => Streams::Recompress { level },
        (kept, None) => kept,
    };
    Ok(OptimizePlan {
        source: 0,
        names,
        prune: !arguments.switch("--no-prune"),
        object_streams,
        streams,
    })
}

/// `pages`: the edit flags in the order they were written on the command line.
///
/// RFC 0002 section 6.2's composition rule is left to right over the current page list, so the
/// *order* of the flags is data and `Arguments::value` — which takes the last of a repeated
/// flag — is the wrong accessor for all four. They are read off `flags` instead, which is argv
/// order.
fn page_edits(arguments: &Arguments) -> Result<Vec<Edit>, Failure> {
    let mut edits = Vec::new();
    for (flag, value) in &arguments.flags {
        let Some(value) = value.as_deref() else {
            continue;
        };
        edits.push(match flag.as_str() {
            "--delete" => Edit::Delete(selection(value, flag)?),
            "--rotate" => rotation(value)?,
            "--move" => {
                let (pages, to) = at_position(value, ':', "--move")?;
                Edit::Move {
                    pages: selection(pages, flag)?,
                    to,
                }
            }
            "--insert" => {
                // One input, and the boundary between this verb and `merge` is the count of
                // files rather than the kind of edit (RFC 0002 sections 4.1 and 6.2). A
                // path here is the other verb's request, said by name.
                if value.contains(".pdf") || value.contains('/') {
                    return Err(Failure::Usage(format!(
                        "--insert {value:?}: pages takes one input, so --insert takes a range \
                         of this document; another file's pages are what merge is for"
                    )));
                }
                let (pages, at) = at_position(value, '@', "--insert")?;
                Edit::Insert {
                    pages: selection(pages, flag)?,
                    at,
                }
            }
            _ => continue,
        });
    }
    if edits.is_empty() {
        return Err(Failure::Usage(
            "pages needs at least one of --delete, --rotate, --move or --insert".to_owned(),
        ));
    }
    Ok(edits)
}

/// One range, with the flag named in the error.
fn selection(text: &str, flag: &str) -> Result<Selection, Failure> {
    text.parse()
        .map_err(|error| Failure::Usage(format!("{flag} {text:?}: {error}")))
}

/// RFC 0002 section 6.1's `split`: where the cuts are, and how the pieces are named.
fn split_plan(arguments: &Arguments, pages: Selection, names: Pattern) -> Result<Plan, Failure> {
    let every = arguments.parsed::<usize>(&["--every"])?;
    if every == Some(0) {
        return Err(Failure::Usage("--every counts from 1".to_owned()));
    }
    let at_bookmarks = arguments.switch("--at-bookmarks");
    // §12.3.3's levels count from 1, so the default is its top-level items.
    let depth = arguments.parsed::<usize>(&["--at-bookmarks"])?.unwrap_or(1);
    if at_bookmarks && depth == 0 {
        return Err(Failure::Usage(
            "--at-bookmarks counts §12.3.3's outline levels from 1".to_owned(),
        ));
    }
    // Four ways of saying where the cuts are, and the default is the one every
    // toolbox has: one file per page (pdftk's `burst`, poppler's `pdfseparate`).
    // `--pages` without `--every` cuts at the selection's own commas, which is RFC
    // 0002 section 6.1's `--pages 1-3,7-end` writing two files.
    let pieces = match (every, at_bookmarks, arguments.value(&["--pages"])) {
        (Some(_), true, _) => {
            return Err(Failure::Usage(
                "--every and --at-bookmarks are two different places to cut".to_owned(),
            ));
        }
        (_, true, _) => Pieces::AtBookmarks(depth),
        (Some(every), false, _) => Pieces::Every(every),
        (None, false, Some(_)) => Pieces::Groups,
        (None, false, None) => Pieces::EachPage,
    };
    Ok(Plan::Split(SplitPlan {
        source: 0,
        pages,
        pieces,
        names,
    }))
}

/// `range<sep>position`, split at the **last** separator so a range may contain one.
fn at_position<'a>(
    value: &'a str,
    separator: char,
    flag: &str,
) -> Result<(&'a str, usize), Failure> {
    let (pages, position) = value.rsplit_once(separator).ok_or_else(|| {
        Failure::Usage(format!(
            "{flag} {value:?}: takes a range and a position, as 5{separator}1"
        ))
    })?;
    let position = position
        .parse::<usize>()
        .map_err(|error| Failure::Usage(format!("{flag} {value:?}: {position:?}: {error}")))?;
    Ok((pages, position))
}

/// `[+|-]angle:range` — qpdf's spelling, and RFC 0002 section 6.2's.
///
/// A sign makes the angle relative to the page's effective §7.7.3.3 rotation; no sign makes it
/// absolute. The split is at the **first** colon, because the range after it may hold colons of
/// its own (`:odd`).
fn rotation(value: &str) -> Result<Edit, Failure> {
    let (angle, range) = value.split_once(':').ok_or_else(|| {
        Failure::Usage(format!(
            "--rotate {value:?}: takes an angle and a range, as +90:1-end"
        ))
    })?;
    let relative = angle.starts_with('+') || angle.starts_with('-');
    let degrees = angle
        .parse::<i64>()
        .map_err(|error| Failure::Usage(format!("--rotate {value:?}: {angle:?}: {error}")))?;
    Ok(Edit::Rotate {
        angle: if relative {
            Angle::Relative(degrees)
        } else {
            Angle::Absolute(degrees)
        },
        pages: selection(range, "--rotate")?,
    })
}

/// `merge`: one input per positional argument, in the order their pages appear.
fn merge_plan(arguments: &Arguments, names: Pattern) -> Result<MergePlan, Failure> {
    if arguments.value(&["--pages"]).is_some() {
        return Err(Failure::Usage(
            "merge takes a range per input, as file.pdf:1-5, rather than one --pages for all of \
             them"
                .to_owned(),
        ));
    }
    if arguments.positional.is_empty() {
        return Err(Failure::Usage(
            "merge needs at least one input file".to_owned(),
        ));
    }
    let mut inputs = Vec::new();
    for (source, spec) in arguments.positional.iter().enumerate() {
        let (_, selection) = input_spec(spec);
        inputs.push(Input {
            source,
            pages: selection.unwrap_or_else(Selection::all),
        });
    }
    Ok(MergePlan {
        inputs,
        collate: arguments.switch("--collate"),
        information: information_entries(arguments)?,
        names,
    })
}

/// `--info Key=value`, repeated: §14.3.3's entries the merged document states.
///
/// **Never derived from the inputs**, which is the merge's own rule (ADR 0821 section 9) and
/// `doc/questions/A55`'s: the entries a merged document states are the operator's statement
/// about a document no producer wrote, and stating none leaves it with no `/Info` at all. A key
/// outside Table 349, or a value the table's type refuses, is an error naming both rather than a
/// quietly dropped flag. `--info Key=` with nothing after the `=` states no entry and removes
/// none, because a merged document begins with none to remove.
fn information_entries(arguments: &Arguments) -> Result<Vec<InfoEntry>, Failure> {
    let mut out = Vec::new();
    for text in arguments.every("--info") {
        let Some((key, value)) = text.split_once('=') else {
            return Err(Failure::Usage(format!(
                "--info takes Key=value, one of §14.3.3's Table 349 keys ({}), and {text:?} has                  no =",
                INFORMATION_KEYS.join(", ")
            )));
        };
        if value.is_empty() {
            continue;
        }
        out.push(InfoEntry {
            key: key.to_owned(),
            value: Some(value.to_owned()),
        });
    }
    Ok(out)
}

/// One positional argument split into a path and, where it has one, a page selection.
///
/// `merge` takes its ranges per input — `a.pdf:1-5` — because one `--pages` cannot say
/// different things about different files. The split is at the **last** colon and only where
/// what follows it parses as §4.2's range grammar, so a file whose name contains a colon and no
/// range is still opened by its own name.
fn input_spec(spec: &str) -> (PathBuf, Option<Selection>) {
    if let Some((path, range)) = spec.rsplit_once(':')
        && let Ok(selection) = range.parse::<Selection>()
        && !path.is_empty()
    {
        return (PathBuf::from(path), Some(selection));
    }
    (PathBuf::from(spec), None)
}

/// The files the plan reads, opened into [`Source`]s.
///
/// **One `--password-fd` opens one document**, which is `viewer_core::Secret`'s own rule: it is
/// deliberately not `Clone`, because "[a] copy is a second buffer to clear and a second lifetime
/// to reason about". So a merge of more than one input with a password is a usage error rather
/// than a password quietly used for a file it was not typed for; a per-input fd is what would
/// lift it and nobody has asked for one.
fn open_inputs(arguments: &Arguments, plan: &Plan) -> Result<Vec<Source>, Failure> {
    let wanted = if matches!(plan, Plan::Merge(_)) {
        arguments.positional.len()
    } else {
        1
    };
    if arguments.positional.len() != wanted {
        return Err(Failure::Usage(format!(
            "exactly {wanted} input file(s), and {} were given",
            arguments.positional.len()
        )));
    }
    let mut password = arguments.parsed::<u32>(&["--password-fd"])?;
    if password.is_some() && wanted > 1 {
        return Err(Failure::Usage(
            "--password-fd opens one document, and this merge reads several; a password is one \
             file's"
                .to_owned(),
        ));
    }
    let mut sources = Vec::with_capacity(wanted);
    for spec in &arguments.positional {
        let (path, _) = input_spec(spec);
        // On disk rather than read whole: ADR 0809's window, so that a merge of six-gigabyte
        // inputs costs each one's trailer, table and selected pages rather than its bytes.
        let bytes = pdf_syntax::FileBytes::on_disk(&path)
            .map_err(|error| Failure::Unreadable(path.clone(), error))?;
        sources.push(match password.take() {
            Some(fd) => Source::with_password(bytes, password_from(fd)?),
            None => Source::new(bytes),
        });
    }
    Ok(sources)
}

/// `attachments`: exactly one of its five actions, from the flags.
fn attachments_action(arguments: &Arguments, output: Option<&str>) -> Result<Action, Failure> {
    Ok(
        match (
            arguments.switch("--list"),
            arguments.switch("--save-all"),
            arguments.value(&["--save"]),
            arguments.value(&["--attach"]),
            arguments.value(&["--remove"]),
        ) {
            (true, false, None, None, None) => Action::List,
            (false, true, None, None, None) => Action::SaveAll {
                names: directory_or_pattern(output, "--save-all")?,
            },
            (false, false, Some(name), None, None) => Action::Save {
                name: name.to_owned(),
                names: directory_or_pattern(output, "--save")?,
            },
            (false, false, None, Some(file), None) => attach_action(arguments, output, file)?,
            (false, false, None, None, Some(name)) => Action::Remove {
                name: name.to_owned(),
                names: output
                    .ok_or_else(|| Failure::Usage("--remove needs -o <name>".to_owned()))?
                    .parse()
                    .map_err(|error| Failure::Usage(format!("-o: {error}")))?,
            },
            _ => {
                return Err(Failure::Usage(
                    "attachments takes exactly one of --list, --save-all, --save <name>, \
                 --attach <file>, --remove <name>"
                        .to_owned(),
                ));
            }
        },
    )
}

/// `--attach <file>`: the file read, its filing name decided, the date read where given.
fn attach_action(
    arguments: &Arguments,
    output: Option<&str>,
    file: &str,
) -> Result<Action, Failure> {
    let names = |what: &str| -> Result<Pattern, Failure> {
        output
            .ok_or_else(|| Failure::Usage(format!("{what} needs -o <name>")))?
            .parse()
            .map_err(|error| Failure::Usage(format!("-o: {error}")))
    };
    let path = PathBuf::from(file);
    // The payload is *attached*, so every byte of it is written into the document: read whole,
    // with the room asked for first, rather than opened on disk.
    let bytes =
        pdf_syntax::read_file(&path).map_err(|error| Failure::Unreadable(path.clone(), error))?;
    // The filing name is the file's own unless `--name` says otherwise, and
    // it has to be a name: a path with no final component names nothing.
    let name = match arguments.value(&["--name"]) {
        Some(name) if !name.is_empty() => name.to_owned(),
        Some(_) => return Err(Failure::Usage("--name is empty".to_owned())),
        None => path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .ok_or_else(|| {
                Failure::Usage(format!(
                    "--attach {file:?} has no file name to file it under; \
                     give one with --name"
                ))
            })?,
    };
    let date = arguments
        .value(&["--date"])
        .map(|text| {
            parse_iso_8601(text).ok_or_else(|| {
                Failure::Usage(format!(
                    "--date takes YYYY-MM-DDTHH:MM:SS with an optional Z or \
                     ±HH:MM, not {text:?}"
                ))
            })
        })
        .transpose()?;
    let on_page = match arguments.parsed::<usize>(&["--to-page"])? {
        None => {
            for flag in ["--rect", "--icon"] {
                if arguments.value(&[flag]).is_some() {
                    return Err(Failure::Usage(format!(
                        "{flag} places an annotation, which needs --to-page <n>"
                    )));
                }
            }
            None
        }
        Some(0) => return Err(Failure::Usage("--to-page counts from 1".to_owned())),
        Some(page) => Some(OnPage {
            page,
            rect: arguments.value(&["--rect"]).map(parse_rect).transpose()?,
            icon: match arguments.value(&["--icon"]) {
                None => None,
                Some(icon) if OnPage::ICONS.contains(&icon) => Some(icon.to_owned()),
                Some(other) => {
                    return Err(Failure::Usage(format!(
                        "--icon takes Graph, PushPin, Paperclip or Tag, not {other:?}"
                    )));
                }
            },
        }),
    };
    Ok(Action::Attach {
        payload: Payload::new(bytes),
        name,
        description: arguments.value(&["--description"]).map(str::to_owned),
        date,
        names: names("--attach")?,
        on_page,
    })
}

/// `--rect 'x y w h'`: the annotation's lower-left corner and its size, in user-space units,
/// separated by spaces or commas — Table 166's `/Rect` is `[x0 y0 x1 y1]`, and a person states
/// a box by where it is and how big.
fn parse_rect(text: &str) -> Result<[f32; 4], Failure> {
    let bad = || {
        Failure::Usage(format!(
            "--rect takes 'x y w h' in page units, not {text:?}"
        ))
    };
    let fields: Vec<f32> = text
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|field| !field.is_empty())
        .map(|field| field.parse::<f32>().map_err(|_error| bad()))
        .collect::<Result<_, _>>()?;
    let [x, y, w, h] = fields[..] else {
        return Err(bad());
    };
    if !(x.is_finite() && y.is_finite() && w > 0.0 && h > 0.0 && w.is_finite() && h.is_finite()) {
        return Err(bad());
    }
    Ok([x, y, x + w, y + h])
}

/// `--format png|ppm|pgm`, PNG where nothing is said.
fn image_format(arguments: &Arguments) -> Result<ImageFormat, Failure> {
    match arguments.value(&["--format"]) {
        None => Ok(ImageFormat::Png),
        Some(word) => ImageFormat::parse(word)
            .ok_or_else(|| Failure::Usage(format!("--format takes png, ppm or pgm, not {word:?}"))),
    }
}

/// `--scale-to WxH` or `--scale-to N`.
fn sizing_from(text: &str) -> Result<Sizing, Failure> {
    let bad = || Failure::Usage(format!("--scale-to takes N or WxH, not {text:?}"));
    if let Some((width, height)) = text.split_once('x') {
        let width: u32 = width.parse().map_err(|_error| bad())?;
        let height: u32 = height.parse().map_err(|_error| bad())?;
        if width == 0 || height == 0 {
            return Err(bad());
        }
        return Ok(Sizing::Within { width, height });
    }
    let longest: u32 = text.parse().map_err(|_error| bad())?;
    if longest == 0 {
        return Err(bad());
    }
    Ok(Sizing::Longest(longest))
}

/// `-o dir/` becomes `dir/%t`, and anything else is the pattern it says.
fn directory_or_pattern(output: Option<&str>, what: &str) -> Result<Pattern, Failure> {
    let output = output.ok_or_else(|| Failure::Usage(format!("{what} needs -o <name>")))?;
    let pattern = if output.ends_with('/') {
        format!("{output}%t")
    } else {
        output.to_owned()
    };
    pattern
        .parse()
        .map_err(|error| Failure::Usage(format!("-o: {error}")))
}

/// §7.6.4's protection over whatever whole file the verb writes, from the two descriptor flags.
///
/// **There is no `--encrypt-owner`, for the reason there is no `--password`**: argv is public,
/// and a password on a command line is in every process listing and every shell history. The
/// flags name a descriptor and this reads one line from it, exactly as `--password-fd` does.
///
/// **Every permission is granted, and that is a decision rather than a gap.** `CLAUDE.md`
/// principle 3 makes a document's restrictions the reader's to set and ranks them low; a program
/// encrypting a file on somebody's behalf has no business withholding from its next reader what
/// the person running it did not ask to withhold. Table 22's seven bits are
/// `pdf_transform::Access`, so a caller of the library can state them; a flag that spells them
/// on a command line is a separate argument nobody has made.
fn protection(arguments: &Arguments, plan: &Plan) -> Result<Option<Protect>, Failure> {
    let owner = arguments.parsed::<u32>(&["--encrypt-owner-fd"])?;
    let user = arguments.parsed::<u32>(&["--encrypt-user-fd"])?;
    if owner.is_none() && user.is_none() {
        return Ok(None);
    }
    // The verbs that write a whole file are the ones the serializer writes for; every other verb
    // produces a PNG, a listing or an appended update, none of which an encryption dictionary
    // belongs in. A flag that was silently ignored would be worse than one refused.
    if !matches!(
        plan,
        Plan::Split(_) | Plan::Merge(_) | Plan::Pages(_) | Plan::Optimize(_) | Plan::Redact(_)
    ) {
        return Err(Failure::Usage(
            "--encrypt-owner-fd and --encrypt-user-fd apply to split, merge, pages, optimize and \
             redact, which are the verbs that write a whole file"
                .to_owned(),
        ));
    }
    Ok(Some(Protect {
        // §7.6.4.1: the empty string is the default user password, which every reader tries
        // first, so a file given only an owner password opens without a prompt.
        user_password: match user {
            Some(fd) => password_from(fd)?,
            None => Secret::new(),
        },
        owner_password: match owner {
            Some(fd) => password_from(fd)?,
            None => Secret::new(),
        },
        access: Access::ALL,
        encrypt_metadata: true,
    }))
}

/// One line from an open descriptor, without its line ending — what a script hands over.
fn password_from(fd: u32) -> Result<Secret, Failure> {
    let file = std::fs::File::open(format!("/dev/fd/{fd}"))
        .map_err(|error| Failure::Password(fd, error))?;
    let mut line = String::new();
    std::io::BufReader::new(file)
        .read_line(&mut line)
        .map_err(|error| Failure::Password(fd, error))?;
    while line.ends_with(['\n', '\r']) {
        line.pop();
    }
    Ok(Secret::from(line))
}

/// A listing on stdout, one line per entry, for a person; `--report=json` is for a program.
fn print_listing(report: &Report) {
    for listed in &report.listed {
        match listed {
            Listed::Image(image) => println!(
                "page {}\t{}x{}\t{} bpc\t{}\t{}{}{}",
                image.page,
                image.width,
                image.height,
                image
                    .bits_per_component
                    .map_or_else(|| "?".to_owned(), |bits| bits.to_string()),
                image.colour_space.as_deref().unwrap_or("-"),
                if image.filters.is_empty() {
                    "raw".to_owned()
                } else {
                    image.filters.join("+")
                },
                if image.stencil { "\tstencil" } else { "" },
                if image.masked { "\tmasked" } else { "" },
            ),
            Listed::Attachment(file) => println!(
                "{}\t{}\t{}\t{}",
                file.name,
                file.file_name.as_deref().unwrap_or("-"),
                file.size
                    .map_or_else(|| "?".to_owned(), |size| size.to_string()),
                file.media_type.as_deref().unwrap_or("-"),
            ),
        }
    }
}

/// Sinks that open files at the names the plan expands.
///
/// The name is a path as given: relative to the working directory, its parent directory
/// existing. Nothing is created but the file.
#[derive(Debug)]
struct FileSinks;

impl Sinks for FileSinks {
    fn open(&self, name: &str) -> std::io::Result<Box<dyn Write + Send + '_>> {
        Ok(Box::new(std::io::BufWriter::new(std::fs::File::create(
            name,
        )?)))
    }
}

/// Sinks for `-o -`: the one output goes to stdout, and a second is an error.
#[derive(Debug, Default)]
struct StdoutSinks {
    /// Whether stdout has been handed out already.
    taken: Mutex<bool>,
}

impl Sinks for StdoutSinks {
    fn open(&self, _name: &str) -> std::io::Result<Box<dyn Write + Send + '_>> {
        let mut taken = self
            .taken
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if *taken {
            return Err(std::io::Error::other(
                "only one output can be written to stdout",
            ));
        }
        *taken = true;
        // The handle rather than its lock: `StdoutLock` is not `Send`, and the one writer a
        // verb opens here may be written from a worker thread.
        Ok(Box::new(std::io::stdout()))
    }
}

/// What the tool takes, printed by `--help` on stdout and above a usage error on stderr — one
/// text, so the two say the same words.
const USAGE: &str = "\
usage: quorra-transform <verb> <file.pdf> [options] -o <name>

verbs:
  render       pages to raster images        -o 'page-%d.png' | -o -
  images       the images the pages embed     -o 'img-%d.png' | --list
  split        one document into many        -o 'page-%d.pdf'
  merge        several documents into one    a.pdf b.pdf [--collate] -o out.pdf
  pages        one document's pages edited   --delete | --rotate | --move | --insert
  optimize     one document rewritten smaller, losslessly   -o out.pdf
  redact       one document's /Redact annotations applied (ISO 32000-2 §12.5.6.23):
               the marked content removed from the content stream and the annotations
               taken away, written as a new file   -o out.pdf
  archive      one document converted to ISO 19005 (PDF/A)   --to <target> -o out.pdf
  attachments  embedded files (ISO 32000-2 §7.11.4), from the name tree, the catalog's
               /AF and every page's file attachment annotations
               --list | --save-all -o dir/ | --save <name> -o <file>
               --attach <file> -o out.pdf   the file added to the document's name tree by
                                            §7.5.6's incremental update: the input's bytes,
                                            byte for byte, and the new objects after them
               --remove <name> -o out.pdf   the file taken out of the name tree by the same
                                            update; its objects are marked free, never erased


render:
  --pages <selection>   which pages (default: all)
  --dpi <n>             dots per inch, 72 units to the inch (default 150)
  --scale-to <N|WxH>    fit the longer side to N pixels, or the page inside WxH
  --format png|ppm|pgm  PNG (default), binary PPM (the RGB, no alpha), or binary PGM: the
                        grey of the RGB by ISO 32000-2 §10.4.2.2's rule, 0.3 R + 0.59 G +
                        0.11 B; JPEG is absent until an encoder is decided (RFC 0002
                        section 6.5)
  --max-pixels <n>      refuse a page larger than this (default 2^28)
  --page-box <box>      media, crop, bleed, trim or art (§7.7.3.3): the box is the raster's
                        extent and its clip; default is the viewer's own display boundary,
                        the crop box unless §12.2's /ViewArea names another
  --no-annotations      the page contents alone, without §12.5.3's annotation pass
images:

  --pages <selection>   which pages to look on (default: all)
  --min-pixels <n>      leave out images with fewer samples
  --list                inventory only; nothing decoded, nothing written
  --native              the embedded stream as it is where it is a file on its own: DCT as
                        .jpg, JPX as .jp2, the rest decoded to PNG (JBIG2 and CCITT say so);
                        the extension is appended to the name, so -o 'img-%d'. A native
                        JPEG is the JPEG: its /Decode is not in it, and its mask is
                        written beside it as <name>.mask.png
  --no-mask             the image with no mask applied, and its mask beside it as
                        <name>.mask.png — an 8-bit grey PNG on the mask's own grid whose
                        value is the opacity it gives the image
  --format png|ppm|pgm  the file form of every image that is decoded: PNG (default), PPM,
                        or PGM by §10.4.2.2's rule over the decoded RGB, whatever the
                        image's own colour space; a native stream is never converted. A
                        netpbm file has no alpha, so the mask goes beside it as
                        <name>.mask.pgm
  every image is decoded to PNG with its mask in the alpha; an XObject once, an inline
  image (BI … ID … EI) at every placement
split:
  --pages <selection>   which pages (default: all), and without --every the selection's own
                        commas are where the cuts are: --pages 1-3,7-end writes two files
  --every <n>           pieces of n pages; --every 1 is one file per page, the default
  --at-bookmarks[=n]    a piece begins at every page a §12.3.3 outline item at level n or
                        shallower lands on (default 1, the top-level items), and runs to the
                        next such page; the pages before the first one are a piece with no
                        title. %t in the output name is that item's /Title
  each piece is a new document: the source's page objects, their whole object closure and
  their content streams carried byte for byte, under a new one-level page tree and a new
  catalog. §7.7.3.4's inherited /Resources, /MediaBox, /CropBox and /Rotate are written onto
  each page, because the ancestors that carried them are not coming along. A reference to a
  page outside the piece becomes §7.3.10's null and is reported (exit 3). Carried and cut to
  the piece: §14.7's structure tree, §12.3.3's outline, §12.4.2's page labels (recomputed —
  a label is a position) and §12.3.2.4's named destinations that resolve inside it. /Metadata
  and the rest are **not** carried and every one the document states is named in a warning.
  A piece of an encrypted document is not encrypted, and says so.

merge:
  a.pdf b.pdf …         the inputs, in the order their pages appear; a range per input, as
                        a.pdf:1-5 or b.pdf:end-1, using the same grammar as --pages
  --collate             interleave the inputs a page at a time (pdftk's shuffle) instead of
                        concatenating them
  --info Key=value      one of §14.3.3's Table 349 entries the merged document states, repeat
                        for more: Title, Author, Subject, Keywords, Creator, Producer,
                        CreationDate, ModDate, Trapped. Nothing is taken from the inputs — the
                        merged document was made by no input's producer at no input's creation
                        time — so with no --info it states no /Info at all, which is what it did
                        before this flag existed. Where CreationDate, ModDate or Creator is
                        stated, §14.3.2's metadata stream is written beside the dictionary with
                        the same values, because §14.3.4 requires the two fully equivalent where
                        both are written
  the merged document is a new file: every page's object closure and content streams carried
  byte for byte under one page tree, with §7.7.3.4's inherited /Resources, /MediaBox, /CropBox
  and /Rotate written onto each page. What is reconciled, and where each choice comes from:
  §8.11's optional content groups and their initial states are unioned; §7.9.6's name trees
  are merged and a colliding key is renamed, with every /Dest and /GoTo naming a renamed
  destination rewritten to match; §12.3.3's outlines are spliced into one chain; §12.4.2's
  labels are written one entry per page; and §14.11.5's output intent goes onto each source's
  own pages where the sources disagree, which is the home the clause gives it. §12.7.4.2's
  fully qualified field names must not collide with a different /FT, /V or /DV — a merge that
  would write two such fields is refused by name (exit 4). A signature crosses without its /V
  (§12.8.1), the outline destinations that leave the merge become §7.3.10's null, and
  /Info and /Metadata are not carried and are named in a warning. §14.7's structure tree is
  carried: the elements the merged pages reach, under one root with the output's own
  §14.7.5.4 parent-tree keys, and a cross-source /ID collision is refused by name (exit 4).

pages:
  one input, one output, and every flag may repeat; the edits compose left to right over the
  running page list, so each range is read against the list as the edits before it left it
  --delete <selection>       take these pages out
  --rotate [+|-]angle:range  §7.7.3.3's /Rotate, a multiple of 90, clockwise when displayed:
                             90:1 sets page 1 to 90, +90:1-end turns every page a quarter
                             further than it is displayed now — the sign is what makes it
                             relative, and a relative angle composes with the rotation
                             §7.7.3.4 gives the page rather than with what it states itself
  --move <range>:<position>  move these pages so the first lands at that position,
                             counted from 1; one past the end appends
  --insert <range>@<position>  a second copy of these pages before that position. This
                             verb reads one file, so the range is this document's; another
                             file's pages are merge's. A page that appears twice is two page
                             objects (Table 31 gives a page one /Parent) with its content and
                             resources shared, and its annotations copied with it — a page
                             carrying a §12.7 widget is refused by name (exit 4), because a
                             field's fully qualified name is its identity (§12.7.4.2)
  the output is a new document on the same construction as merge, so the same reconciliations
  apply when a page leaves: a destination to a deleted page becomes §7.3.10's null (exit 3),
  §12.4.2's labels are written one entry per surviving page, and §12.3.3's outline, §7.9.6's
  name trees, §8.11's groups and §12.7's fields cross as they do there. §14.7's structure tree
  is carried by this verb as by the other two, pruned to the pages the output holds, with the
  parent-tree keys and each page's /StructParents restated as the output's own.

optimize:
  one input, one output, and nothing on the page changes. Four lossless passes, all on by
  default, each reported by --report=json with what it saved:
  --no-prune                 keep every object the file holds. By default an object no path
                             from §7.5.5's /Root reaches is not written, and neither is one
                             whose value is §7.3.10's null nor the object a stream stated its
                             /Length in — the writer re-derives /Length as a direct integer
  --object-streams <mode>    generate (default) or disable: §7.5.7's object streams, every
                             object the clause permits packed into a FlateDecode carrier.
                             Generating them makes the cross-reference section a §7.5.8
                             stream, because Table 18's type 2 entry is the only way to say
                             where a compressed object is, and raises the header to 1.5
  --recompress <mode>        all (default) or none: every stream decoded through the filters
                             this tree reads and re-encoded as one FlateDecode, kept only
                             where it is smaller. The decoded bytes are identical, so no mark
                             changes; an image codec stops the walk and its bytes are carried
                             inside the new outer filter
  --compression-level <n>    zlib's 0 to 9 (default 9)
  --linearize                refused by name: CLAUDE.md excludes Annex F until linearisation
                             is separately ratified
  lossy image optimisation is deliberately absent: it needs a DCT encoder this tree does not
  have (RFC 0002 section 13's second question), and a downsampler without one would keep every
  image under qpdf's fails-to-shrink rule and do nothing while claiming to. Optimising an
  encrypted document produces an unencrypted one, and says so.

archive:
  --to <target>            2b, 2u, 2a (ISO 19005-2), 4, 4f, 4e (ISO 19005-4). Required: which
                           part and level a document has to reach is the one thing this program
                           cannot work out for you. Parts 1 and 3 are not targets and will not
                           become ones, because their text is not held and a requirement may not
                           be implemented from somebody else's reading of it
  --authorise <what>       may repeat. What the conversion may throw away: image-smoothing turns
                           /Interpolate off, so a low-resolution image looks blockier;
                           metadata-property removes an XMP property whose own predefined schema
                           does not define the value it holds, naming each one and what it held;
                           annotation-printing gives an annotation that stated no flags the Print
                           bit ISO 19005 requires, so one whose appearance never printed now
                           prints; jpeg2000-colour-fallback keeps only the colour space
                           specification a JPEG 2000 image uses, so a processor that cannot use
                           that one falls back to a device space rather than to the producer's
                           next specification, no image sample being touched;
                           forbidden-annotation takes an annotation of a subtype ISO 19005 does
                           not admit off the page it was on, and with it the sound, movie,
                           rendition or 3D artwork it named and its own Contents description,
                           the report naming each and whether it drew anything — answer the site
                           with a preserve remedy in a --config file and its normal appearance
                           is kept on a page appended to the document instead;
                           encryption writes the document out with no encryption, which both
                           parts forbid outright: nothing in the document changes, every string
                           and stream having been decrypted when the source was opened, but the
                           archived copy is readable by anybody holding it and ISO 32000-2
                           §7.6.4.2's Table 22 permission flags stop being asserted — the report
                           and the output's own xmpMM:History name every flag the source stated;
                           signature-assertion lets a signed document be rewritten at all — a
                           signature covers the bytes of one file and a conversion moves every
                           one of them, so each signature field loses its value and keeps its
                           widget and appearance, and the report names each signature, its
                           signer, its time and what verifying it over the source found.
                           Anything not authorised stops the conversion instead of happening
                           quietly
  --output-intent-profile <file>
                           the ICC profile a PDF/A output intent added by this conversion names
                           as its destination profile. The default is one of the two profiles
                           this program ships, picked by the colour the document actually draws
                           in: the GRACoL 2006 CMYK profile where its unlicensed device colour
                           is CMYK and none of it is RGB, the sRGB profile otherwise. Supply
                           your own for a document produced for a particular press — which
                           press a document was made for is the one thing nobody but its owner
                           knows. The profile's own copyright tag is printed either way,
                           because embedding somebody's profile means shipping their terms
                           with it
  --no-substitute          a font the file renders and does not embed is refused by name instead
                           of being given one of the faces this program ships. The default is to
                           substitute, because a PDF whose font is not embedded has no appearance
                           of its own — every reader picks a face at display time and they pick
                           different ones — so embedding one removes that indeterminacy, which is
                           what the format is for. The report names, per font, what was asked
                           for, what was embedded, and whether the face's own advances were used
                           or restated to the widths the file states; no glyph moves either way.
                           Batch archiving wants the default; a curator checking one document
                           may want the flag
  --font <base>=<path>     embed the font program in <path> for the /BaseFont <base>, instead of
                           one of the shipped faces. Repeatable, one per font; §9.9.2's six-letter
                           subset tag is passed over, so /ABCDEF+Garamond is named as Garamond.
                           This is the answer where no shipped face covers a document's
                           characters, and it is a statement you are making rather than a setting:
                           ISO 19005-2 section 6.2.11.4.1 admits only a program that may lawfully
                           be embedded for unlimited, universal rendering, and ISO 32000-2 §9.9.1
                           makes that a fact about a licence which nothing in a document or on a
                           disk states. So the report and the output's own xmpMM:History record
                           the face as the operator's. The program's advances are restated to the
                           widths the file already states, so no glyph moves; a face without a
                           glyph for every code the document shows is refused by name rather than
                           embedded with holes in it
  --resolve-external-data  a stream whose dictionary states F keeps its data in another file, and
                           both parts of ISO 19005 forbid that outright. With this flag the bytes
                           are read and written into the stream, its Filter and DecodeParms taken
                           from the FFilter and FDecodeParms that described them, so the archive
                           holds what the producer pointed at. Only a plain file name beside the
                           document itself is read — never ../, never an absolute path, and never
                           a URL, which would be a network request this program does not make.
                           Each name refused is printed. A stream stating the filter keys and no
                           F names no file at all: those keys describe filters for a file that is
                           not there, no reader consults them, and they are removed with no flag
  --remedy-sites           print every refusal site this target binds, with the remedy each one
                           admits, and stop. The list is generated from the same table the
                           converter decides from, so a site cannot exist undocumented and a
                           configuration naming one that does not exist is an error rather than
                           an ignored line. It ends with its own total, 'N of M sites not built
                           yet'; add --config <file> and it also names every answer that profile
                           gives which this version does not carry out, with a second total. That
                           second question needs no document: what a profile asks for and what
                           has code behind it are both properties of the profile and the target
  --config <file>          a remedy configuration: a refusal answered in advance, per site, so a
                           queue does not stop for a person (doc/rfc/0007). A site absent from
                           the file behaves exactly as without it, so installing one changes no
                           pipeline until it names a site. The remedy words are stop (the default
                           everywhere), discard, preserve, derive and supply; --remedy-sites says
                           which of them each site admits, and a site whose remedy this version
                           cannot carry out is named on stderr and stays refused.
                           A `derive` site names a program in a [tool.<name>] block and is
                           refused unless it names the site AND the tool. THE PROGRAM IS YOUR
                           CHOICE AND THE DOCUMENT IS NOT: this runs a program you chose, on a
                           document you did not write. No confinement is offered for it. This
                           program never starts it from inside the conversion — the conversion
                           returns the invocation as data and this program runs it between two
                           passes, so one command still does the whole thing. What a tool made is
                           reported per document in the words 'this is derived, not original',
                           with the tool, the program and a SHA-256 of what came back, and the
                           same is written into the file's own xmpMM:History.
                           A `supply` site states a fact the document does not — an attachment's
                           media type — which is recorded as the operator's in both places too.
                           doc/profiles/ ships six configurations; derive-attachments.toml is the
                           worked example of a tool
  --depart-from-the-standard
                           required before a configuration's departure is carried out: a departure
                           produces a file that does NOT conform, on purpose, and the intent to go
                           against the standard belongs at the call site rather than only in a
                           file that can be inherited or copied between teams
  --claim-conformance      a departed file keeps the PDF/A identification anyway. By default it is
                           left off, so the file does not claim what it has not earned; a
                           downstream validator fails it either way, and the only difference is
                           whether the file lied before it failed
  the document is validated against the target, one decision is taken per requirement it fails,
  and the rewrites those decisions call for are applied — then the output is validated again and
  is **not written** if it fails a requirement the source met. The report says, per document,
  what already conformed, what was changed and under which clause, what was refused and why, and
  which requirements the verdict does not cover; --report=json carries all of it. Adding an
  output intent is reported as what it is: it states an interpretation, so every device colour
  in the file afterwards means what that profile says it means to a conforming reader. A font
  the file does not embed is given one of the faces this program ships unless --no-substitute
  says otherwise or --font names a program of your own, and an embedded program whose stated
  advances disagree with its own font
  dictionary has the program's numbers restated, never the dictionary's — /Widths is what
  positions the glyphs. The structure tree, encryption, attachments, a composite font nothing
  embedded, and a page that draws a glyph its own program has not got are refused **by name**,
  with the clause they could not meet, and no file is written.
  doc/pdf-a-conversion-limits.md is the whole list.

attachments --attach:
  --name <name>         the name the file is filed under (default: the file's own name)
  --description <text>  Table 43's /Desc
  --date <iso-8601>     YYYY-MM-DDTHH:MM:SS[Z|±HH:MM] written as the file's creation and
                        modification date; none is written otherwise, so the same
                        attachment is the same bytes on every run
  --to-page <n>         filed by a §12.5.6.15 file attachment annotation on page n instead
                        of by the name tree; the icon is drawn by this tree's own artwork
  --rect 'x y w h'      the annotation's box in page units (default: a 20-unit square
                        20 units in from the crop box's upper-left corner)
  --icon <name>         Graph, PushPin (default), Paperclip or Tag — §12.5.6.15's four


options for every verb:
  --report=json         the report on stdout (not with -o -)
  --strict              exit 2 rather than 3 on a warning
  --quiet-warnings      exit 0 rather than 3 on a warning
  --password-fd <n>     read the password, one line, from descriptor n;
                        there is no --password, because argv is public
  --encrypt-owner-fd <n>  encrypt the output (ISO 32000-2 §7.6.4, /V 5 /R 6) with the
  --encrypt-user-fd <n>   owner and user passwords read from those descriptors;
                        split, merge, pages, optimize and redact only
  --restrictions=off|on|ask|warn
                        whether what the document asserts over its reader — Table 22's /P bits,
                        §12.8.2.2's certification — is honoured (default off: the program is
                        the reader's); `on` refuses with exit 4, `warn` reports; `ask` is the
                        fourth level and a command line has nobody to ask

page selection (RFC 0002 section 4.2): 5  3-7  7-3  1-end  r1  r3-r1  a,b,c  x3-4  3-7:odd  @iv
  @{A-3}  @iv-@ix — parity is the page number's; a label is §12.4.2's, first match where the
  document repeats one
output names: %d ordinal (zero-padded to the count; %03d for a width), %p first source page,
  %l its label, %t a title (an embedded file's name); more than one output needs %d. A label
  or title in a name has /, \\, control bytes and <>:\"|?* replaced by _, and the report says so.
exit: 0 clean, 2 error, 3 written with warnings, 4 refused by name, 1 usage
";
