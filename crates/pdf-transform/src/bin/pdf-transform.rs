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

use std::io::{BufRead as _, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use pdf_transform::attachments::{Action, AttachmentsPlan, OnPage, Payload, parse_iso_8601};
use pdf_transform::images::ImagesPlan;
use pdf_transform::merge::{Input, MergePlan};
use pdf_transform::optimize::OptimizePlan;
use pdf_transform::pages::{Angle, Edit, PagesPlan};
use pdf_transform::pattern::Pattern;
use pdf_transform::range::Selection;
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing, parse_boundary};
use pdf_transform::split::{Pieces, SplitPlan};

use pdf_syntax::serialize::{ObjectStreams, Streams};
use pdf_transform::{
    Budget, Exit, Level, Listed, Plan, Policy, Report, Secret, Sinks, Source, apply,
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
    /// The seam refused.
    #[error("{0}")]
    Refused(#[from] pdf_transform::Refusal),
}

impl Failure {
    /// The exit status.
    fn exit(&self) -> Exit {
        match self {
            Self::Usage(_) => Exit::Usage,
            Self::Unreadable(..) | Self::Password(..) => Exit::Error,
            Self::Refused(refusal) => refusal.exit(),
        }
    }
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
    "--restrictions",
    "--report",
    "--max-pixels",
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
    "--password-fd",
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

    let policy = Policy {
        restrictions: match arguments.value(&["--restrictions"]) {
            None => Level::Off,
            // The fourth level is a usage error here rather than a refusal at run time: a
            // command line has nobody to ask, and saying so before the file is opened is what
            // keeps `ask` from looking like a level this program has.
            Some("ask") => {
                return Err(Failure::Usage(
                    "--restrictions=ask: this program cannot ask; use on, warn or off".to_owned(),
                ));
            }
            Some(word) => Level::parse(word).ok_or_else(|| {
                Failure::Usage(format!(
                    "--restrictions takes off, on or warn, not {word:?}"
                ))
            })?,
        },
    };
    let mut budget = Budget::default();
    if let Some(max_pixels) = arguments.parsed::<u64>(&["--max-pixels"])? {
        budget.max_pixels = max_pixels;
    }

    let report = if to_stdout {
        apply(&plan, &sources, &StdoutSinks::default(), &policy, &budget)?
    } else {
        apply(&plan, &sources, &FileSinks, &policy, &budget)?
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
        print_listing(&report);
    }
    Ok(report.exit(
        arguments.switch("--strict"),
        arguments.switch("--quiet-warnings"),
    ))
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
        "attachments" => Ok(Plan::Attachments(AttachmentsPlan {
            source: 0,
            action: attachments_action(arguments, output)?,
        })),
        "" => Err(Failure::Usage("no verb given".to_owned())),
        other => Err(Failure::Usage(format!("no such verb: {other:?}"))),
    }
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
        names,
    })
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
usage: pdf-transform <verb> <file.pdf> [options] -o <name>

verbs:
  render       pages to raster images        -o 'page-%d.png' | -o -
  images       the images the pages embed     -o 'img-%d.png' | --list
  split        one document into many        -o 'page-%d.pdf'
  merge        several documents into one    a.pdf b.pdf [--collate] -o out.pdf
  pages        one document's pages edited   --delete | --rotate | --move | --insert
  optimize     one document rewritten smaller, losslessly   -o out.pdf
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
  --restrictions=off|on|warn
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
