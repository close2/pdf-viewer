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

use pdf_transform::attachments::{Action, AttachmentsPlan};
use pdf_transform::images::ImagesPlan;
use pdf_transform::pattern::Pattern;
use pdf_transform::range::Selection;
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::{
    Budget, Exit, Listed, Plan, Policy, Report, Restrictions, Secret, Sinks, Source, apply,
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
];

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
    "--save-all",
    "--save",
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

    let [path] = arguments.positional.as_slice() else {
        return Err(Failure::Usage(format!(
            "exactly one input file, and {} were given",
            arguments.positional.len()
        )));
    };
    let path = PathBuf::from(path);
    let bytes = std::fs::read(&path).map_err(|error| Failure::Unreadable(path.clone(), error))?;
    let source = match arguments.parsed::<u32>(&["--password-fd"])? {
        Some(fd) => Source::with_password(bytes, password_from(fd)?),
        None => Source::new(bytes),
    };

    let policy = Policy {
        restrictions: match arguments.value(&["--restrictions"]) {
            None | Some("off") => Restrictions::Off,
            Some("on") => Restrictions::On,
            Some("warn") => Restrictions::Warn,
            Some(other) => {
                return Err(Failure::Usage(format!(
                    "--restrictions takes off, on or warn, not {other:?}"
                )));
            }
        },
    };
    let mut budget = Budget::default();
    if let Some(max_pixels) = arguments.parsed::<u64>(&["--max-pixels"])? {
        budget.max_pixels = max_pixels;
    }

    let report = if to_stdout {
        apply(&plan, &[source], &StdoutSinks::default(), &policy, &budget)?
    } else {
        apply(&plan, &[source], &FileSinks, &policy, &budget)?
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
            let format = match arguments.value(&["--format"]) {
                None => ImageFormat::Png,
                Some(word) => ImageFormat::parse(word).ok_or_else(|| {
                    Failure::Usage(format!("--format takes png or ppm, not {word:?}"))
                })?,
            };
            Ok(Plan::Render(RenderPlan {
                source: 0,
                pages,
                size,
                format,
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
                names: if list_only {
                    "%d".parse()
                        .map_err(|error| Failure::Usage(format!("{error}")))?
                } else {
                    names("images")?
                },
            }))
        }
        "attachments" => {
            let action = match (
                arguments.switch("--list"),
                arguments.switch("--save-all"),
                arguments.value(&["--save"]),
            ) {
                (true, false, None) => Action::List,
                (false, true, None) => Action::SaveAll {
                    names: directory_or_pattern(output, "--save-all")?,
                },
                (false, false, Some(name)) => Action::Save {
                    name: name.to_owned(),
                    names: directory_or_pattern(output, "--save")?,
                },
                _ => {
                    return Err(Failure::Usage(
                        "attachments takes exactly one of --list, --save-all, --save <name>"
                            .to_owned(),
                    ));
                }
            };
            Ok(Plan::Attachments(AttachmentsPlan { source: 0, action }))
        }
        "" => Err(Failure::Usage("no verb given".to_owned())),
        other => Err(Failure::Usage(format!("no such verb: {other:?}"))),
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
  attachments  embedded files (ISO 32000-2 §7.11.4), from the name tree, the catalog's
               /AF and every page's file attachment annotations
               --list | --save-all -o dir/ | --save <name> -o <file>

render:
  --pages <selection>   which pages (default: all)
  --dpi <n>             dots per inch, 72 units to the inch (default 150)
  --scale-to <N|WxH>    fit the longer side to N pixels, or the page inside WxH
  --format png|ppm      PNG (default) or binary PPM; JPEG is absent until an encoder is
                        decided (RFC 0002 section 6.5); PGM is absent until the grey conversion is
  --max-pixels <n>      refuse a page larger than this (default 2^28)
images:
  --pages <selection>   which pages to look on (default: all)
  --min-pixels <n>      leave out images with fewer samples
  --list                inventory only; nothing decoded, nothing written
  --native              the embedded stream as it is where it is a file on its own: DCT as
                        .jpg, JPX as .jp2, the rest decoded to PNG (JBIG2 and CCITT say so);
                        the extension is appended to the name, so -o 'img-%d'. A native
                        JPEG is the JPEG: its /SMask and /Decode are not in it
  every image is decoded to PNG with its soft mask in the alpha; an XObject once, an inline
  image (BI … ID … EI) at every placement

options for every verb:
  --report=json         the report on stdout (not with -o -)
  --strict              exit 2 rather than 3 on a warning
  --quiet-warnings      exit 0 rather than 3 on a warning
  --password-fd <n>     read the password, one line, from descriptor n;
                        there is no --password, because argv is public
  --restrictions=off|on|warn
                        whether the document's own /P bits are honoured (default off: the
                        program is the reader's); `on` refuses with exit 4, `warn` reports

page selection (RFC 0002 section 4.2): 5  3-7  7-3  1-end  r1  r3-r1  a,b,c  x3-4  3-7:odd  @iv
  @{A-3}  @iv-@ix — parity is the page number's; a label is §12.4.2's, first match where the
  document repeats one
output names: %d ordinal (zero-padded to the count; %03d for a width), %p first source page,
  %l its label, %t a title (an embedded file's name); more than one output needs %d. A label
  or title in a name has /, \\, control bytes and <>:\"|?* replaced by _, and the report says so.
exit: 0 clean, 2 error, 3 written with warnings, 4 refused by name, 1 usage
";
