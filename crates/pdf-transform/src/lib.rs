//! Files derived from documents this tree can read — the transform seam of RFC 0002 section 5, and
//! the verbs that have landed on it.
//!
//! # What this is
//!
//! `tools/pdf-retrieve` is "a program asks a document questions" over the readers this tree
//! already had (ADR 0257). This crate is the same move one step further: **a program asks for a
//! new file derived from documents this tree can already read** — a page as a PNG, the images a
//! page embeds, the files a document carries. The command-line program in `src/bin/` is the first
//! consumer; a KIO worker, a FUSE filesystem and a viewer menu are the others RFC 0002 section 2 names,
//! and each of them constructs the same [`Plan`] and supplies its own [`Sinks`].
//!
//! # The seam's four rules, each inherited from a boundary this tree already proved
//!
//! 1. **A transform is a pure plan applied to sources through caller-supplied sinks.** A
//!    [`Plan`] is data — which verb, which pages, which options — with no path inside. A
//!    [`Source`] is bytes and, where the document needs one, §7.6.4.1's password. [`Sinks`] hand
//!    out one writer per output the plan names, on demand, keyed by the pattern-expanded name;
//!    the library never opens a path.
//! 2. **No filesystem, no clock, no environment.** [`apply`]'s output is a function of
//!    (sources, plan, policy, budget) and nothing else, which is what makes RFC 0002 section 9's
//!    determinism claim a test rather than a demo: the same inputs produce the same bytes, with
//!    no flag needed.
//! 3. **Streaming is in the types.** Sinks are writers, because a consumer that fills a kernel
//!    buffer cannot hold an output in memory any more than the CLI should.
//! 4. **The seam is transport-free.** Nothing in this API names a process arrangement; running
//!    [`apply`] in a confined child on the `pdf-view-worker` pattern is a transport change RFC
//!    0002 section 8 names as the follow-up, not a redesign.
//!
//! # What it is not
//!
//! It is not `viewer-core`'s boundary and does not leak into it. `viewer-core`'s vocabulary is a
//! person at a window's; a transform is a batch job over documents no window has open. The one
//! type taken from there is [`Secret`], because a second password type would be a second buffer
//! to clear.
//!
//! # The policy a document asserts, and the four levels
//!
//! `CLAUDE.md` principle 3: a document's restrictions are the reader's to set, and they have
//! four levels — off, on, ask before the operation, warn before the operation. A pipe cannot
//! ask, so [`Restrictions`] carries the three a batch tool can honour, defaults to `Off` because
//! the program is the reader's, and is *asked once* in [`apply`] rather than decided at the point
//! of any operation — the shape that lets the fourth level be added by a host that can ask,
//! without revisiting this decision. ADR 0800.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod attachments;
pub mod images;
pub mod json;
pub mod pattern;
pub mod range;
pub mod render;

use std::io::Write;
use std::sync::{Arc, Mutex};

use pdf_syntax::{Document, Limits, SyntaxError};

pub use viewer_core::Secret;

use crate::json::Value;

/// Bytes this tree may read, with the password that opens them where one is needed.
///
/// Whole bytes rather than a seekable reader, because `pdf_syntax::Document` opens over an
/// `Arc<[u8]>` today; RFC 0002 section 5's seekable form is the serializer round's need, and the type
/// is where it will be added.
#[derive(Debug)]
pub struct Source {
    /// The file.
    bytes: Arc<[u8]>,
    /// §7.6.4.1's user or owner password, where the caller has one.
    password: Option<Secret>,
}

impl Source {
    /// A document with no password: §7.6.4.1's default user password, the empty string, is
    /// what an encrypted document is first tried with.
    pub fn new(bytes: impl Into<Arc<[u8]>>) -> Self {
        Self {
            bytes: bytes.into(),
            password: None,
        }
    }

    /// A document with a password the caller already holds.
    pub fn with_password(bytes: impl Into<Arc<[u8]>>, password: Secret) -> Self {
        Self {
            bytes: bytes.into(),
            password: Some(password),
        }
    }

    /// How many bytes the source is.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Whether the source is empty, which no PDF is.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Opens the document, which is the one place in this crate a password is read.
    fn open(&self, at: usize, limits: Limits) -> Result<Document, Refusal> {
        // `Secret::reveal` is named so that this moment is visible at the call site; §7.6.4.1's
        // default user password is the empty string, so a source without one asks with it.
        let password = self.password.as_ref().map_or("", Secret::reveal);
        Document::open_with_password(Arc::clone(&self.bytes), limits, password).map_err(|error| {
            match error {
                SyntaxError::PasswordRequired => Refusal::PasswordRequired { at },
                other => Refusal::Unopenable { at, error: other },
            }
        })
    }
}

/// Where outputs go: one writer per output the plan names, opened on demand.
///
/// The CLI's sinks open files; a FUSE consumer's fill a kernel buffer; [`MemorySinks`] fill
/// memory for a test. `Sync` because a verb writes its outputs from several threads at once
/// (RFC 0002 section 12: a transform is throughput-first), and the name is what keys them.
pub trait Sinks: Sync {
    /// Opens the output called `name`.
    ///
    /// # Errors
    ///
    /// Whatever opening it costs — a directory that does not exist, a second request for a sink
    /// that can only be opened once.
    fn open(&self, name: &str) -> std::io::Result<Box<dyn Write + Send + '_>>;
}

/// One in-memory output's bytes, shared between the sink that hands it out and the writer.
type Buffer = Arc<Mutex<Vec<u8>>>;

/// Sinks that keep every output in memory, in the order they were opened.
#[derive(Debug, Default)]
pub struct MemorySinks {
    /// Each output's name and its bytes so far.
    outputs: Mutex<Vec<(String, Buffer)>>,
}

impl MemorySinks {
    /// Empty.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Every output written, by name, in the order the outputs were opened.
    #[must_use]
    pub fn into_outputs(self) -> Vec<(String, Vec<u8>)> {
        self.outputs
            .into_inner()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .into_iter()
            .map(|(name, bytes)| {
                let bytes = bytes
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone();
                (name, bytes)
            })
            .collect()
    }
}

/// A writer into one of [`MemorySinks`]'s buffers.
#[derive(Debug)]
struct MemoryWriter {
    /// The buffer.
    bytes: Buffer,
}

impl Write for MemoryWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.bytes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Sinks for MemorySinks {
    fn open(&self, name: &str) -> std::io::Result<Box<dyn Write + Send + '_>> {
        let bytes = Arc::new(Mutex::new(Vec::new()));
        self.outputs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((name.to_owned(), Arc::clone(&bytes)));
        Ok(Box::new(MemoryWriter { bytes }))
    }
}

/// How much of a document's own restrictions this run honours — three of `CLAUDE.md`'s four
/// levels, the fourth (*ask*) needing a host that can ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Restrictions {
    /// The document's `/P` bits are not consulted. The default: the program is the reader's.
    #[default]
    Off,
    /// A restricted operation is refused by name, exit 4.
    On,
    /// A restricted operation is performed and the restriction is reported as a warning.
    Warn,
}

/// What the host decides about a document's assertions over its reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Policy {
    /// Table 22's bits, at one of the levels a batch tool can honour.
    pub restrictions: Restrictions,
}

/// Explicit ceilings, the same family as the interpreter's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    /// `pdf-syntax`'s parse limits.
    pub limits: Limits,
    /// The most pixels one rendered page may have. Page dimensions come from the document, so
    /// an unbounded scale multiplication is a memory-exhaustion vector; a page past this is
    /// refused by name rather than attempted.
    pub max_pixels: u64,
}

impl Default for Budget {
    /// [`Limits::DEFAULT`], and 2^28 pixels a page — a gibibyte of RGBA, enough for A4 at
    /// 1200 dpi (≈ 1.4 × 10^8 pixels) and a stated choice a caller overrides.
    fn default() -> Self {
        Self {
            limits: Limits::DEFAULT,
            max_pixels: 1 << 28,
        }
    }
}

/// One thing a reader does to a document that Table 22 can restrict, as this crate's verbs
/// do it.
///
/// `pdf_model::restriction::Operation` names the two operations the viewer performs — filling a
/// field, adding an annotation — and this names the two a transform performs. They belong in one
/// enum, in `pdf-model`, and moving them there is a first-row change (`doc/todo/02` §2) deferred
/// to a round that runs the whole sequence; ADR 0800 records the debt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// Rasterising a page to a file — Table 22 bit 3, "Print the document".
    ///
    /// A choice: a page raster is what a print driver produces, and it is the nearest of the
    /// bits. It is written down as a choice because the clause does not mention rasterisation.
    Print,
    /// Taking images or embedded files out — Table 22 bit 5, "[c]opy or otherwise extract text
    /// and graphics from the document".
    Extract,
}

impl Operation {
    /// Table 22's bit number.
    #[must_use]
    pub fn bit(self) -> u8 {
        match self {
            Self::Print => 3,
            Self::Extract => 5,
        }
    }

    /// The operation as a sentence names it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Print => "rendering a page",
            Self::Extract => "extracting from the document",
        }
    }

    /// Whether the document, opened as it was, withholds this operation.
    ///
    /// §7.6.4.2 Table 22's bits bind the *user*; a document opened with its owner password
    /// withholds nothing, which is `Permissions::owner`.
    fn withheld_by(self, document: &Document) -> bool {
        document.permissions().is_some_and(|permissions| {
            !permissions.owner
                && match self {
                    Self::Print => !permissions.print,
                    Self::Extract => !permissions.copy,
                }
        })
    }
}

/// What to do, as data: which verb, over which source, with which options.
#[derive(Debug, Clone, PartialEq)]
pub enum Plan {
    /// Pages to raster images — RFC 0002 section 6.4.
    Render(render::RenderPlan),
    /// The images a document embeds, as files — RFC 0002 section 6.3.
    Images(images::ImagesPlan),
    /// §7.11.4's embedded files, listed or extracted — RFC 0002 section 6.6.
    Attachments(attachments::AttachmentsPlan),
}

impl Plan {
    /// Which source the plan reads.
    #[must_use]
    pub fn source(&self) -> usize {
        match self {
            Self::Render(plan) => plan.source,
            Self::Images(plan) => plan.source,
            Self::Attachments(plan) => plan.source,
        }
    }

    /// Which of Table 22's operations the plan performs, if any.
    #[must_use]
    pub fn operation(&self) -> Option<Operation> {
        match self {
            Self::Render(_) => Some(Operation::Print),
            Self::Images(plan) if plan.list_only => None,
            Self::Images(_) => Some(Operation::Extract),
            Self::Attachments(plan) => match plan.action {
                attachments::Action::List => None,
                attachments::Action::SaveAll { .. } | attachments::Action::Save { .. } => {
                    Some(Operation::Extract)
                }
            },
        }
    }
}

/// Why the whole operation produced nothing usable.
///
/// The source's index is `at` rather than `source` because `thiserror` reads a field of that
/// name as the error's cause.
///
/// Per-item refusals — one page the rasteriser would not draw, one image whose codec is
/// unavailable — are not this: they are [`Report::refused`], beside the outputs that were
/// written, so that a caller can tell "page 7 of 300 was refused" from "nothing happened".
#[derive(Debug, thiserror::Error)]
pub enum Refusal {
    /// The plan names a source the caller did not supply.
    #[error("the plan reads source {at}, and {count} were given")]
    NoSuchSource {
        /// The index asked for.
        at: usize,
        /// How many there are.
        count: usize,
    },
    /// The bytes are not a PDF this tree opens.
    #[error("source {at}: does not open as a PDF ({error})")]
    Unopenable {
        /// Which source.
        at: usize,
        /// `pdf-syntax`'s own sentence.
        error: SyntaxError,
    },
    /// §7.6.4.1: the document is encrypted and neither the empty password nor the one supplied
    /// opens it.
    #[error("source {at}: a password is required")]
    PasswordRequired {
        /// Which source.
        at: usize,
    },
    /// The page selection does not resolve against this document.
    #[error("source {at}: {error}")]
    Selection {
        /// Which source.
        at: usize,
        /// What the grammar could not find.
        error: range::ResolveError,
    },
    /// The output-name pattern cannot name what the plan produces — a usage error.
    #[error("{0}")]
    Pattern(String),
    /// Under [`Restrictions::On`], the document withholds the operation.
    #[error(
        "this document restricts {operation}: Table 22 bit {bit} is clear, and --restrictions is \
         on"
    )]
    Restricted {
        /// What was declined.
        operation: &'static str,
        /// Which bit.
        bit: u8,
    },
    /// No embedded file has the name asked for.
    #[error("source {at}: no embedded file is named {name:?}")]
    NoSuchAttachment {
        /// Which source.
        at: usize,
        /// The name asked for.
        name: String,
    },
    /// A sink could not be opened or written.
    #[error("{name}: {error}")]
    Sink {
        /// The output being written.
        name: String,
        /// The sink's own error.
        error: std::io::Error,
    },
}

impl Refusal {
    /// RFC 0002 section 4.4's exit status for this refusal.
    #[must_use]
    pub fn exit(&self) -> Exit {
        match self {
            Self::Pattern(_) => Exit::Usage,
            Self::Restricted { .. } => Exit::Refused,
            Self::NoSuchSource { .. }
            | Self::Unopenable { .. }
            | Self::PasswordRequired { .. }
            | Self::Selection { .. }
            | Self::NoSuchAttachment { .. }
            | Self::Sink { .. } => Exit::Error,
        }
    }
}

/// RFC 0002 section 4.4's exit statuses.
///
/// qpdf's are the prior art (0 clean, 2 error, 3 warnings, never 1 because wrappers use it),
/// and 4 is this project's own: **refused by name** — the operation is well-formed but this
/// program declines it, either by policy or because an unsupported construct was hit on the
/// path (trap 5: unsupported input stays loud). 2 means the *file* defeated us, 4 means *we*
/// declined, and a caller can tell them apart without parsing stderr.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    /// Success, no warnings.
    Success,
    /// The shell's and argument parsing's, never a transform's.
    Usage,
    /// The operation did not produce usable output; the message says why.
    Error,
    /// Output written, recoverable malformations reported.
    Warnings,
    /// Refused by name.
    Refused,
}

impl Exit {
    /// The process exit code.
    #[must_use]
    pub fn code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::Usage => 1,
            Self::Error => 2,
            Self::Warnings => 3,
            Self::Refused => 4,
        }
    }
}

/// What was done: every output, every inventory entry, every warning, every refusal.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    /// The files written, in output order.
    pub outputs: Vec<Output>,
    /// The inventory, for a plan that lists rather than writes.
    pub listed: Vec<Listed>,
    /// Recoverable trouble: output written, but this was met on the way.
    pub warnings: Vec<Warning>,
    /// Items this program declined, by name. Non-empty means [`Exit::Refused`].
    pub refused: Vec<Declined>,
}

/// One file written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// The name the sink was opened with.
    pub name: String,
    /// How many bytes were written.
    pub bytes: u64,
    /// Whether a `%l` or `%t` in the name had a byte replaced by sanitisation.
    pub sanitised: bool,
    /// What it was derived from.
    pub origin: Origin,
}

/// Where an output came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// A rendered page.
    Page {
        /// Which source.
        source: usize,
        /// The page, counted from 1.
        page: usize,
        /// Its §12.4.2 label, where the document states one.
        label: Option<String>,
        /// The raster's width in pixels.
        width: u32,
        /// Its height.
        height: u32,
    },
    /// An image `XObject`.
    Image {
        /// Which source.
        source: usize,
        /// The first selected page it is placed on, counted from 1.
        page: usize,
        /// The object it is, where it is an indirect object.
        object: Option<String>,
        /// Its width in samples.
        width: u32,
        /// Its height.
        height: u32,
    },
    /// A §7.11.4 embedded file.
    Attachment {
        /// Which source.
        source: usize,
        /// The name the document files it under.
        name: String,
    },
}

/// One inventory entry, for a listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Listed {
    /// An image `XObject` a selected page places.
    Image(images::ImageEntry),
    /// A §7.11.4 embedded file.
    Attachment(attachments::AttachmentEntry),
}

/// Something met on the way that did not stop the output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// Which source.
    pub source: usize,
    /// The page it concerns, counted from 1, where it concerns one.
    pub page: Option<usize>,
    /// What it was, in the reporting layer's own words.
    pub detail: String,
}

/// One item this program declined, by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declined {
    /// Which source.
    pub source: usize,
    /// The page it concerns, counted from 1, where it concerns one.
    pub page: Option<usize>,
    /// What was declined.
    pub subject: String,
    /// Why.
    pub detail: String,
}

impl Report {
    /// RFC 0002 section 4.4's exit status: 4 if anything was refused, else 3 if anything was warned
    /// about — which `strict` turns into 2 and `quiet_warnings` into 0 — else 0.
    #[must_use]
    pub fn exit(&self, strict: bool, quiet_warnings: bool) -> Exit {
        if !self.refused.is_empty() {
            Exit::Refused
        } else if self.warnings.is_empty() || quiet_warnings {
            Exit::Success
        } else if strict {
            Exit::Error
        } else {
            Exit::Warnings
        }
    }

    /// The report as RFC 0002 section 4.5's JSON.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Value::Object(vec![
            (
                "outputs".to_owned(),
                Value::Array(self.outputs.iter().map(Output::to_json).collect()),
            ),
            (
                "listed".to_owned(),
                Value::Array(self.listed.iter().map(Listed::to_json).collect()),
            ),
            (
                "warnings".to_owned(),
                Value::Array(
                    self.warnings
                        .iter()
                        .map(|warning| {
                            Value::Object(vec![
                                ("source".to_owned(), Value::count(warning.source)),
                                ("page".to_owned(), Value::optional_count(warning.page)),
                                ("detail".to_owned(), Value::text(warning.detail.clone())),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "refused".to_owned(),
                Value::Array(
                    self.refused
                        .iter()
                        .map(|declined| {
                            Value::Object(vec![
                                ("source".to_owned(), Value::count(declined.source)),
                                ("page".to_owned(), Value::optional_count(declined.page)),
                                ("subject".to_owned(), Value::text(declined.subject.clone())),
                                ("detail".to_owned(), Value::text(declined.detail.clone())),
                            ])
                        })
                        .collect(),
                ),
            ),
        ])
    }
}

impl Output {
    /// One output as JSON.
    fn to_json(&self) -> Value {
        let origin = match &self.origin {
            Origin::Page {
                source,
                page,
                label,
                width,
                height,
            } => vec![
                ("kind".to_owned(), Value::text("page")),
                ("source".to_owned(), Value::count(*source)),
                ("page".to_owned(), Value::count(*page)),
                ("label".to_owned(), Value::optional(label.clone())),
                ("width".to_owned(), Value::Integer(i64::from(*width))),
                ("height".to_owned(), Value::Integer(i64::from(*height))),
            ],
            Origin::Image {
                source,
                page,
                object,
                width,
                height,
            } => vec![
                ("kind".to_owned(), Value::text("image")),
                ("source".to_owned(), Value::count(*source)),
                ("page".to_owned(), Value::count(*page)),
                ("object".to_owned(), Value::optional(object.clone())),
                ("width".to_owned(), Value::Integer(i64::from(*width))),
                ("height".to_owned(), Value::Integer(i64::from(*height))),
            ],
            Origin::Attachment { source, name } => vec![
                ("kind".to_owned(), Value::text("attachment")),
                ("source".to_owned(), Value::count(*source)),
                ("name".to_owned(), Value::text(name.clone())),
            ],
        };
        Value::Object(vec![
            ("name".to_owned(), Value::text(self.name.clone())),
            ("bytes".to_owned(), Value::bytes(self.bytes)),
            ("sanitised".to_owned(), Value::Bool(self.sanitised)),
            ("origin".to_owned(), Value::Object(origin)),
        ])
    }
}

impl Listed {
    /// One inventory entry as JSON.
    fn to_json(&self) -> Value {
        match self {
            Self::Image(entry) => entry.to_json(),
            Self::Attachment(entry) => entry.to_json(),
        }
    }
}

/// Applies a plan to its sources through the sinks, under the policy and the budget.
///
/// The one entry point. Opens the source the plan names, asks the policy once, resolves the
/// plan's selection against the document, and runs the verb — which writes its outputs through
/// `sinks`, in parallel where the verb is embarrassingly so, and accounts for every one of them
/// in the returned [`Report`].
///
/// # Errors
///
/// A [`Refusal`] where nothing usable could be produced: an unopenable source, a password
/// missing, a selection or pattern the document cannot satisfy, a restriction under
/// [`Restrictions::On`], a sink that failed. Anything less — a page refused, an image the codec
/// worker declined, a mark the interpreter could not draw — is in the report beside what was
/// written.
pub fn apply(
    plan: &Plan,
    sources: &[Source],
    sinks: &dyn Sinks,
    policy: &Policy,
    budget: &Budget,
) -> Result<Report, Refusal> {
    let at = plan.source();
    let source = sources.get(at).ok_or(Refusal::NoSuchSource {
        at,
        count: sources.len(),
    })?;
    let document = source.open(at, budget.limits)?;
    let mut report = Report::default();

    // The policy is asked here, once, and nowhere else: this is the place a host supplies its
    // answer, and it is the shape that lets *ask* be added later without touching a verb.
    if let Some(operation) = plan.operation()
        && operation.withheld_by(&document)
    {
        match policy.restrictions {
            Restrictions::Off => {}
            Restrictions::On => {
                return Err(Refusal::Restricted {
                    operation: operation.as_str(),
                    bit: operation.bit(),
                });
            }
            Restrictions::Warn => report.warnings.push(Warning {
                source: at,
                page: None,
                detail: format!(
                    "this document restricts {}: Table 22 bit {} is clear",
                    operation.as_str(),
                    operation.bit()
                ),
            }),
        }
    }

    match plan {
        Plan::Render(plan) => render::run(plan, &document, sinks, budget, &mut report)?,
        Plan::Images(plan) => images::run(plan, &document, sinks, &mut report)?,
        Plan::Attachments(plan) => attachments::run(plan, &document, sinks, &mut report)?,
    }
    Ok(report)
}

/// The words for an interpreter report, which `pdf-model` deliberately does not word for a
/// person — `viewer-core` does that for a window, in its own register. The `Debug` form is what
/// `tools/pdf-retrieve` prints for the same list, so the two programs say one thing.
pub(crate) fn describe(unsupported: &pdf_model::content::Unsupported) -> String {
    format!("{unsupported:?}")
}
