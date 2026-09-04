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
//! four levels — off, on, ask before the operation, warn before the operation. All four are
//! `pdf_model::restriction::Level`, and [`Policy`] carries one; it is *asked once* in [`apply`],
//! through `pdf_model::restriction::decide`, rather than decided at the point of any operation.
//! **A pipe cannot ask** — but a terminal can, and [`consult`] is how a caller asks before it
//! acts (ADR 0874). Where there is nobody to ask, `Level::Ask` is answered here by a refusal that
//! says a question
//! went unanswered — the one degradation a batch tool can make without pretending to be a
//! window, and [`Refusal::Unanswered`] is its own variant so that a caller can tell it from a
//! refusal by policy. The default is `Off` because the program is the reader's. ADRs 0800, 0803.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod attachments;
pub mod images;
pub mod json;
pub mod merge;
pub mod optimize;
pub mod pages;
pub mod pattern;
pub mod range;
pub mod render;
pub mod split;
pub(crate) mod structure;
pub mod update;

use std::io::Write;
use std::sync::{Arc, Mutex};

pub use pdf_model::restriction::{Level, Operation};
use pdf_model::restriction::{Restriction, Verdict};
use pdf_syntax::{Document, Limits, SyntaxError};

pub use viewer_core::Secret;

use crate::json::Value;

/// Bytes this tree may read, with the password that opens them where one is needed.
///
/// Held as [`pdf_syntax::FileBytes`], which is whole in memory or open on disk and read where
/// the file's offsets point — RFC 0002 section 5's seekable form, which arrived with ADR 0809
/// under the syntax crate rather than here. A verb that appends to the file asks for it whole
/// through the same type and is refused by name where the process cannot hold it.
#[derive(Debug)]
pub struct Source {
    /// The file.
    bytes: pdf_syntax::FileBytes,
    /// §7.6.4.1's user or owner password, where the caller has one.
    password: Option<Secret>,
}

impl Source {
    /// A document with no password: §7.6.4.1's default user password, the empty string, is
    /// what an encrypted document is first tried with.
    pub fn new(bytes: impl Into<pdf_syntax::FileBytes>) -> Self {
        Self {
            bytes: bytes.into(),
            password: None,
        }
    }

    /// A document with a password the caller already holds.
    pub fn with_password(bytes: impl Into<pdf_syntax::FileBytes>, password: Secret) -> Self {
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

    /// Opens the document, for a consumer of the seam that wants a *reader* rather than a verb.
    ///
    /// RFC 0003's `pdf-vfs` is why this is public: three of its generators — a page's text,
    /// §14.3.3's information dictionary and §14.3.2's metadata stream — are `pdf-model` readers
    /// that no verb of this suite covers, and a consumer that had to open the document itself
    /// would need its own copy of §7.6.4.1's password. [`Secret`] is deliberately not `Clone`
    /// (`viewer_core::secret`), so a second buffer is exactly what this avoids: one `Source`
    /// holds the password, and both the verbs and the readers reach the document through it.
    ///
    /// # Errors
    ///
    /// [`Refusal::PasswordRequired`] where §7.6.4.1's password does not open it, and
    /// [`Refusal::Unopenable`] for bytes this tree does not open at all. The source's index is
    /// reported as 0, because a caller holding one source has one.
    pub fn document(&self, limits: Limits) -> Result<Document, Refusal> {
        self.open(0, limits)
    }

    /// Opens the document, which is the one place in this crate a password is read.
    fn open(&self, at: usize, limits: Limits) -> Result<Document, Refusal> {
        // `Secret::reveal` is named so that this moment is visible at the call site; §7.6.4.1's
        // default user password is the empty string, so a source without one asks with it.
        let password = self.password.as_ref().map_or("", Secret::reveal);
        Document::open_with_password(self.bytes.clone(), limits, password).map_err(|error| {
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

/// What the host decides about a document's assertions over its reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    /// How much of what the document asserts — Table 22's bits, §12.8.2.2's certification —
    /// this run obeys. `Level::Ask` is honoured by [`Refusal::Unanswered`], because nothing
    /// here can put a question to anybody.
    pub restrictions: Level,
}

impl Default for Policy {
    /// `Level::Off`: the program is the reader's, and `pdf_model` deliberately supplies no
    /// default of its own, so the choice is stated here where a batch tool makes it.
    fn default() -> Self {
        Self {
            restrictions: Level::Off,
        }
    }
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

/// What to do, as data: which verb, over which source, with which options.
#[derive(Debug, Clone, PartialEq)]
pub enum Plan {
    /// Pages to raster images — RFC 0002 section 6.4.
    Render(render::RenderPlan),
    /// The images a document embeds, as files — RFC 0002 section 6.3.
    Images(images::ImagesPlan),
    /// §7.11.4's embedded files, listed or extracted — RFC 0002 section 6.6.
    Attachments(attachments::AttachmentsPlan),
    /// One document into many — RFC 0002 section 6.1.
    Split(split::SplitPlan),
    /// Several documents into one — RFC 0002 section 6.2.
    Merge(merge::MergePlan),
    /// One document's pages deleted, inserted, moved and rotated — RFC 0002 section 6.2.
    Pages(pages::PagesPlan),
    /// One document rewritten smaller — RFC 0002 section 6.5.
    Optimize(optimize::OptimizePlan),
    /// One document edited **in place**, by §7.5.6's incremental update: a page taken out, a
    /// document's pages carried in, or §14.3.3's entries set.
    ///
    /// The verb RFC 0003's file-system faces write through, and the reason it is not a flag on
    /// [`Plan::Pages`] is `CLAUDE.md`'s: what a person does to a document they already have open
    /// "is written back by §7.5.6's incremental update: the new objects and a new
    /// cross-reference section appended, never a rewrite of what was there". `pages` and `merge`
    /// write a new file; this appends to the one it read. [`update`] has the whole argument.
    Update(update::UpdatePlan),
}

impl Plan {
    /// Which source the plan reads first.
    ///
    /// Every verb but [`Plan::Merge`] reads exactly one, and a merge reads its first input's —
    /// [`Plan::sources`] is what a caller wanting all of them asks.
    #[must_use]
    pub fn source(&self) -> usize {
        match self {
            Self::Render(plan) => plan.source,
            Self::Images(plan) => plan.source,
            Self::Attachments(plan) => plan.source,
            Self::Split(plan) => plan.source,
            Self::Pages(plan) => plan.source,
            Self::Optimize(plan) => plan.source,
            Self::Update(plan) => plan.source,
            Self::Merge(plan) => plan.inputs.first().map_or(0, |input| input.source),
        }
    }

    /// Every source the plan reads, ascending and without repeats.
    ///
    /// The order [`apply`] opens them in, and therefore the order they take in the
    /// serializer's `Assembly`; the plan's own input order decides which pages come first and
    /// is separate from this.
    #[must_use]
    pub fn sources(&self) -> Vec<usize> {
        match self {
            Self::Merge(plan) => {
                let mut sources: Vec<usize> =
                    plan.inputs.iter().map(|input| input.source).collect();
                sources.sort_unstable();
                sources.dedup();
                sources
            }
            // An insertion reads the document it edits *and* the one whose pages it carries,
            // so both are opened and the policy below is asked of both — the same shape a merge
            // has, for the same reason: Table 22's flags are one document's assertion over its
            // reader, and taking pages out of the second is as much an assembly as putting them
            // into the first.
            Self::Update(plan) => match plan.edit {
                update::Edit::InsertPages { from, .. } => {
                    let mut sources = vec![plan.source, from];
                    sources.sort_unstable();
                    sources.dedup();
                    sources
                }
                update::Edit::DeletePage { .. } | update::Edit::SetInformation { .. } => {
                    vec![plan.source]
                }
            },
            other => vec![other.source()],
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
                attachments::Action::Attach { .. } | attachments::Action::Remove { .. } => {
                    Some(Operation::Modify)
                }
            },
            // Table 22's bit 11 names this operation in as many words — "[a]ssemble the
            // document (insert, rotate, or delete pages …)" — and a split is a file made of
            // pages the source stated. `pdf_model::restriction` has the reading.
            //
            // **A merge is the same operation and is asked of every source it reads.** The
            // bit's sentence names inserting pages first of all, which is what a merge does to
            // each input's pages; and §12.8.2.2's certification permits it for the reason
            // `restriction::certification_permits` records — Table 257 is about "changes to the
            // document", and a merge leaves every source's bytes where they were and writes a
            // different file beside them.
            //
            // **`pages` is the operation the bit's sentence describes word for word** — "insert,
            // rotate, or delete pages" — so it is the same answer and no separate reading.
            //
            // **And `optimize` is the same operation for the same sentence.** Table 22 bit 11
            // is "[a]ssemble the document (insert, rotate, or delete pages …)", and a
            // rewritten file is the document's own pages assembled into a new one: the same
            // pages, in the same order, stated more compactly. Nothing here is a Table 22
            // operation of its own — there is no bit for "make this smaller" — and answering
            // `None` would make the one verb whose whole output is a derived file the one verb
            // no policy is asked about.
            Self::Split(_) | Self::Merge(_) | Self::Pages(_) | Self::Optimize(_) => {
                Some(Operation::Assemble)
            }
            // The same reading, applied in place: Table 22 bit 11 is "[a]ssemble the document
            // (insert, rotate, or delete pages …)", and two of this verb's three edits are the
            // first and the third of those words. The third edit writes §14.3.3's entries, which
            // is none of bits 6, 9 or 11 and is therefore bit 4's residual — "[m]odify the
            // contents of the document by operations other than those controlled by bits 6, 9,
            // and 11" — the same bit an embedded file falls under, for the same sentence
            // (ADR 0802).
            Self::Update(plan) => Some(match plan.edit {
                update::Edit::InsertPages { .. } | update::Edit::DeletePage { .. } => {
                    Operation::Assemble
                }
                update::Edit::SetInformation { .. } => Operation::Modify,
            }),
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
    /// `--at-bookmarks` on a document whose §12.3.3 outline names no page at the stated depth.
    ///
    /// The same shape as [`Refusal::Selection`] and the same status: the request is well formed
    /// and the document is readable, but it does not hold what the plan asked for. §12.3.3 makes
    /// an outline optional — "[a] PDF document **may** contain a document outline" — so most
    /// documents land here, and a verb that answered by writing one piece would have cut nowhere
    /// while saying it cut at the bookmarks.
    #[error(
        "source {at}: no outline item at depth {depth} or shallower resolves to a page of this \
         document, so §12.3.3's outline says nowhere to cut"
    )]
    NoBookmarks {
        /// Which source.
        at: usize,
        /// The depth the plan asked for, counting §12.3.3's top-level items as 1.
        depth: usize,
    },
    /// The output-name pattern cannot name what the plan produces — a usage error.
    #[error("{0}")]
    Pattern(String),
    /// The output could not be assembled at all: a ceiling one file cannot state, a page that
    /// is not an indirect object, a serializer refusal.
    #[error("{0}")]
    Assembly(String),
    /// The source states its structure only through §C.4's recovery, so this program declines
    /// to write a file restating it.
    ///
    /// Told apart from [`Refusal::Assembly`] because the two mean opposite things to a caller,
    /// and RFC 0002 section 4.4 draws the line: 2 is "the *file* defeated us" and 4 is "*we*
    /// declined … an unsupported construct hit on the path". This tree **opens** such a
    /// document, finds its pages and draws them — `pdf_model::Pages` looks for what Table 31
    /// describes, which is right for a reader — so nothing about it defeated anybody. What
    /// happens is that a *writer* declines: Table 15's `/Root` and Table 29's `/Pages` are each
    /// "( Required; shall be an indirect reference )", so a rewrite of a reconstruction would
    /// state a structure no producer wrote. That is exit 4's own sentence, and the message names
    /// the clause.
    #[error("{0}")]
    Reconstructed(String),
    /// A merge names one page twice, and Table 31 gives a page one `/Parent`.
    #[error(
        "source {at}: page {page} would be in the merged document twice, and Table 31 makes a \
         page's /Parent \"the page tree node that is the immediate parent of this page object\""
    )]
    PageTwice {
        /// Which source.
        at: usize,
        /// The page, counted from 1.
        page: usize,
    },
    /// §12.7.4.2: two sources state one fully qualified field name with different contents.
    ///
    /// > In addition, actual field dictionaries with the same fully qualified field name shall
    /// > have the same field type ( FT ), value ( V ), and default value ( DV ).
    ///
    /// A merged document holding both would break that `shall`, and renaming one would change
    /// what §12.7.6.2's submit-form action exports — a change to the document's meaning that is
    /// invisible on the page. So the merge is refused by name instead.
    #[error(
        "§12.7.4.2: these fully qualified field names are stated by two sources with a different \
         /FT, /V or /DV, and one document may not hold both: {fields}"
    )]
    FieldCollision {
        /// Every colliding name, with the sources that state it, joined by `; `.
        fields: String,
    },
    /// Two sources state one §14.7 key the derived document cannot hold both values of.
    ///
    /// Table 354's `/RoleMap` and `/ClassMap` are maps over a namespace two documents can both
    /// use, and Table 355 makes a structure element's `/ID` "unique among all elements in the
    /// document's structure hierarchy". Where the sources agree the entry is carried once;
    /// where they disagree the derived document would have to state two values for one key,
    /// which is §12.7.4.2's shape and gets §12.7.4.2's answer (ADR 0821 section 2, ADR 0834).
    #[error("{clause}, and these keys are stated with different values by two sources: {keys}")]
    StructureConflict {
        /// Which clause forbids the document that would have to be written.
        clause: &'static str,
        /// Every colliding key, with the sources that state it, joined by `; `.
        keys: String,
    },
    /// Under `Level::On`, the document withholds the operation.
    #[error("this document restricts {operation}: {reasons}, and --restrictions is on")]
    Restricted {
        /// What was declined.
        operation: &'static str,
        /// Every reason the document gave, worded, joined by `; `.
        reasons: String,
    },
    /// Under `Level::Ask`, the document withholds the operation and the caller had nobody to ask.
    ///
    /// A pipe has nobody to put the question to, and going ahead on an unanswered question would
    /// be `Level::Off` under another name; not going ahead is what a closed dialogue means
    /// everywhere else. Its own variant rather than [`Refusal::Restricted`] so that a caller can
    /// tell "refused by policy" from "a question went unanswered" without parsing the sentence.
    ///
    /// **A caller that *has* somebody to ask asks first** — [`consult`], the round trip ADR 0874
    /// added — and reaches this only where there is no terminal, no dialogue and no message box.
    #[error(
        "this document restricts {operation}: {reasons}; the restriction level is ask and there \
         was nobody to ask, so it was not done"
    )]
    Unanswered {
        /// What was declined.
        operation: &'static str,
        /// Every reason the document gave, worded, joined by `; `.
        reasons: String,
    },
    /// Under `Level::Ask`, somebody *was* asked and said no.
    ///
    /// Its own variant rather than [`Refusal::Restricted`], and the distinction is the person:
    /// `Restricted` is this program obeying the document at `Level::On`, and this is a reader
    /// deciding. A sentence saying "--restrictions is on" for an answered question would be a
    /// statement about a level nobody chose (ADR 0874).
    #[error("this document restricts {operation}: {reasons}, and the question was answered no")]
    Declined {
        /// What was declined.
        operation: &'static str,
        /// Every reason the document gave, worded, joined by `; `.
        reasons: String,
    },
    /// §7.7.3.3: `--rotate` was given an angle that is not a multiple of 90.
    ///
    /// > The value shall be a multiple of 90.
    ///
    /// A usage error rather than a rounding: the caller asked for a page state the clause has
    /// no way to write, and guessing which quarter turn was meant would be this program
    /// deciding what a document says.
    #[error("§7.7.3.3 makes /Rotate \"a multiple of 90\", and {degrees} is not one")]
    Rotation {
        /// The angle asked for.
        degrees: i64,
    },
    /// A `pages` edit names a position the running page list does not have.
    #[error(
        "source {at}: no position {position}; the page list has {count} page(s), so the \
             positions are 1 to {count} and one past the end appends"
    )]
    Position {
        /// Which source.
        at: usize,
        /// The position asked for, counted from 1.
        position: usize,
        /// How long the list was.
        count: usize,
    },
    /// §12.7.4.2: `--insert` would put a page carrying a widget annotation into the document
    /// twice.
    ///
    /// > In addition, actual field dictionaries with the same fully qualified field name shall
    /// > have the same field type ( FT ), value ( V ), and default value ( DV ).
    ///
    /// A widget is a field's representation on a page. Duplicating one is either a second field
    /// under a name the clause governs — a field this program would have invented — or a second
    /// representation of the same field, which needs an entry in that field's own `/Kids`. Both
    /// are a form edited rather than a page duplicated, so the operation is declined by name.
    #[error(
        "source {at}: page {page} carries a widget annotation, and §12.7.4.2 makes a field's \
         fully qualified name its identity; duplicating the page would either invent a second \
         field under that name or need an entry written into the field's own /Kids, and this \
         verb does neither"
    )]
    DuplicateWidget {
        /// Which source.
        at: usize,
        /// The page, counted from 1.
        page: usize,
    },
    /// `--to-page` names a page the document does not have.
    #[error("source {at}: no page {page}; the document has {count}")]
    NoSuchPage {
        /// Which source.
        at: usize,
        /// The page asked for, counted from 1.
        page: usize,
        /// How many the document has.
        count: usize,
    },
    /// No embedded file has the name asked for.
    #[error("source {at}: no embedded file is named {name:?}")]
    NoSuchAttachment {
        /// Which source.
        at: usize,
        /// The name asked for.
        name: String,
    },
    /// The document already files an embedded file under the name to be attached.
    ///
    /// §7.9.6's keys "shall not overlap", so a second entry under one key would be a tree the
    /// clause forbids; and quietly replacing the file the document had would be a deletion
    /// nobody asked for.
    #[error("source {at}: an embedded file is already named {name:?}")]
    AttachmentExists {
        /// Which source.
        at: usize,
        /// The name.
        name: String,
    },
    /// §7.5.6's update cannot be appended to this document, for a reason the writer names.
    #[error("source {at}: {error}")]
    Update {
        /// Which source.
        at: usize,
        /// The writer's own sentence.
        error: pdf_syntax::write::UpdateError,
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
            // A pattern that cannot name the outputs, an angle §7.7.3.3 cannot state, a
            // position the page list does not have: three ways of writing an argument wrong.
            Self::Pattern(_) | Self::Rotation { .. } | Self::Position { .. } => Exit::Usage,
            // Both are this program declining a well-formed request by name: a policy, or a
            // document whose cross-reference table it will not chain an update to.
            //
            // §12.7.4.2's field collision is here too, and it is the clearest case of the
            // status's meaning: the request is well formed, the sources are readable, and this
            // program declines to write a document the clause forbids.
            Self::Restricted { .. }
            | Self::Unanswered { .. }
            | Self::Declined { .. }
            | Self::Update { .. }
            | Self::FieldCollision { .. }
            | Self::StructureConflict { .. }
            | Self::DuplicateWidget { .. }
            | Self::Reconstructed(_) => Exit::Refused,
            Self::NoSuchSource { .. }
            | Self::Unopenable { .. }
            | Self::PasswordRequired { .. }
            | Self::Selection { .. }
            | Self::NoBookmarks { .. }
            | Self::NoSuchPage { .. }
            | Self::NoSuchAttachment { .. }
            | Self::AttachmentExists { .. }
            | Self::Assembly(_)
            | Self::PageTwice { .. }
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
    /// An image: an `XObject`, or §8.9.7's inline image.
    Image {
        /// Which source.
        source: usize,
        /// The first selected page it is placed on, counted from 1.
        page: usize,
        /// The object it is, where it is an indirect object.
        object: Option<String>,
        /// Whether it was written into the content stream itself (§8.9.7).
        inline: bool,
        /// Which file form it was written in — decoded PNG, or the codec's own stream.
        file: images::ImageFile,
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
    /// A new document assembled out of a source's pages — `split`'s output.
    Piece {
        /// Which source.
        source: usize,
        /// The first source page it holds, counted from 1.
        first_page: usize,
        /// How many pages it holds.
        pages: usize,
        /// The first page's §12.4.2 label, where the document states one.
        label: Option<String>,
        /// How many indirect objects the piece was written with.
        objects: u32,
    },
    /// One document assembled out of several sources' pages — `merge`'s output.
    Merged {
        /// The sources it drew pages from, in input order.
        sources: Vec<usize>,
        /// How many pages it holds.
        pages: usize,
        /// How many indirect objects it was written with.
        objects: u32,
    },
    /// One document's own pages, edited — `pages`'s output.
    Edited {
        /// Which source.
        source: usize,
        /// How many pages the output holds.
        pages: usize,
        /// How many indirect objects it was written with.
        objects: u32,
    },
    /// One document rewritten smaller — `optimize`'s output.
    Optimized {
        /// Which source.
        source: usize,
        /// How many pages it holds, which is how many the source held.
        pages: usize,
        /// What each pass changed.
        savings: optimize::Savings,
    },
    /// The source document with §7.5.6's update appended, its page list or its §14.3.3 entries
    /// edited in place — `update`'s output.
    Amended {
        /// Which source.
        source: usize,
        /// How many pages the document holds after the edit.
        pages: usize,
        /// What the edit was, in a sentence.
        edit: String,
    },
    /// The source document with §7.5.6's incremental update appended: its own bytes, byte for
    /// byte, and then what was added.
    Updated {
        /// Which source.
        source: usize,
        /// The name the new embedded file is filed under.
        attached: String,
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
        Value::Object(vec![
            ("name".to_owned(), Value::text(self.name.clone())),
            ("bytes".to_owned(), Value::bytes(self.bytes)),
            ("sanitised".to_owned(), Value::Bool(self.sanitised)),
            ("origin".to_owned(), Value::Object(self.origin.to_json())),
        ])
    }
}

impl Origin {
    /// One origin's fields, in the order RFC 0002 section 4.5's report states them.
    fn to_json(&self) -> Vec<(String, Value)> {
        match self {
            Self::Page {
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
            Self::Image {
                source,
                page,
                object,
                inline,
                file,
                width,
                height,
            } => vec![
                ("kind".to_owned(), Value::text("image")),
                ("source".to_owned(), Value::count(*source)),
                ("page".to_owned(), Value::count(*page)),
                ("object".to_owned(), Value::optional(object.clone())),
                ("inline".to_owned(), Value::Bool(*inline)),
                ("file".to_owned(), Value::text(file.extension())),
                ("width".to_owned(), Value::Integer(i64::from(*width))),
                ("height".to_owned(), Value::Integer(i64::from(*height))),
            ],
            Self::Piece {
                source,
                first_page,
                pages,
                label,
                objects,
            } => vec![
                ("kind".to_owned(), Value::text("piece")),
                ("source".to_owned(), Value::count(*source)),
                ("first_page".to_owned(), Value::count(*first_page)),
                ("pages".to_owned(), Value::count(*pages)),
                ("label".to_owned(), Value::optional(label.clone())),
                ("objects".to_owned(), Value::Integer(i64::from(*objects))),
            ],
            Self::Merged {
                sources,
                pages,
                objects,
            } => vec![
                ("kind".to_owned(), Value::text("merged")),
                (
                    "sources".to_owned(),
                    Value::Array(sources.iter().map(|source| Value::count(*source)).collect()),
                ),
                ("pages".to_owned(), Value::count(*pages)),
                ("objects".to_owned(), Value::Integer(i64::from(*objects))),
            ],
            Self::Edited {
                source,
                pages,
                objects,
            } => vec![
                ("kind".to_owned(), Value::text("edited")),
                ("source".to_owned(), Value::count(*source)),
                ("pages".to_owned(), Value::count(*pages)),
                ("objects".to_owned(), Value::Integer(i64::from(*objects))),
            ],
            Self::Optimized {
                source,
                pages,
                savings,
            } => {
                let mut fields = vec![
                    ("kind".to_owned(), Value::text("optimized")),
                    ("source".to_owned(), Value::count(*source)),
                    ("pages".to_owned(), Value::count(*pages)),
                ];
                fields.extend(savings.to_json());
                fields
            }
            other => other.written_json(),
        }
    }

    /// The origins whose output is a *file taken out of* or *made from* a document, rather than
    /// a page or an image drawn from one. Split out because one `match` over every origin is
    /// past this tree's line-count lint, and the split is where the report's own two halves are.
    fn written_json(&self) -> Vec<(String, Value)> {
        match self {
            Self::Attachment { source, name } => vec![
                ("kind".to_owned(), Value::text("attachment")),
                ("source".to_owned(), Value::count(*source)),
                ("name".to_owned(), Value::text(name.clone())),
            ],
            Self::Updated { source, attached } => vec![
                ("kind".to_owned(), Value::text("updated")),
                ("source".to_owned(), Value::count(*source)),
                ("attached".to_owned(), Value::text(attached.clone())),
            ],
            Self::Amended {
                source,
                pages,
                edit,
            } => vec![
                ("kind".to_owned(), Value::text("amended")),
                ("source".to_owned(), Value::count(*source)),
                ("pages".to_owned(), Value::count(*pages)),
                ("edit".to_owned(), Value::text(edit.clone())),
            ],
            // Every other origin is answered by `to_json`, which is the only caller.
            Self::Page { .. }
            | Self::Image { .. }
            | Self::Piece { .. }
            | Self::Merged { .. }
            | Self::Edited { .. }
            | Self::Optimized { .. } => Vec::new(),
        }
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
/// The one entry point. Opens **every** source the plan names, asks the policy once per opened
/// document, resolves the plan's selection against each, and runs the verb — which writes its
/// outputs through `sinks`, in parallel where the verb is embarrassingly so, and accounts for
/// every one of them in the returned [`Report`].
///
/// **The policy is asked per document and not per plan**, which is what a merge made necessary:
/// Table 22's flags and §12.8.2.2's certification are each *one document's* assertion over its
/// reader, so a merge of a document that permits assembly with one that does not is refused on
/// the second's word, by name. A verb reading one source asks once, as before.
///
/// # Errors
///
/// A [`Refusal`] where nothing usable could be produced: an unopenable source, a password
/// missing, a selection or pattern the document cannot satisfy, a restriction under
/// `Level::On` or `Level::Ask`, a sink that failed. Anything less — a page refused, an image the codec
/// worker declined, a mark the interpreter could not draw — is in the report beside what was
/// written.
pub fn apply(
    plan: &Plan,
    sources: &[Source],
    sinks: &dyn Sinks,
    policy: &Policy,
    budget: &Budget,
) -> Result<Report, Refusal> {
    let borrowed: Vec<&Source> = sources.iter().collect();
    apply_borrowed(plan, &borrowed, sinks, policy, budget)
}

/// [`apply`], over sources the caller holds one at a time.
///
/// The same function; the borrow is the whole difference. A caller that keeps one [`Source`] for
/// a document's whole life and is handed a second one per operation cannot build a `&[Source]`
/// out of the two without copying the first — and a [`Source`] carries §7.6.4.1's password,
/// which `viewer_core::Secret` deliberately makes not `Clone` ("[a] copy is a second buffer to
/// clear and a second lifetime to reason about"). RFC 0003's core is exactly that caller: its
/// worker holds the mounted document and is handed the document being inserted.
///
/// # Errors
///
/// [`apply`]'s.
pub fn apply_borrowed(
    plan: &Plan,
    sources: &[&Source],
    sinks: &dyn Sinks,
    policy: &Policy,
    budget: &Budget,
) -> Result<Report, Refusal> {
    let wanted = plan.sources();
    let mut opened = Vec::with_capacity(wanted.len());
    for at in &wanted {
        let source = sources.get(*at).ok_or(Refusal::NoSuchSource {
            at: *at,
            count: sources.len(),
        })?;
        opened.push(source.open(*at, budget.limits)?);
    }
    let mut report = Report::default();

    // The policy is asked here, once per document, and nowhere else: this is the place a host
    // supplies its answer, and every verdict has an arm — a pipe's answer to *ask* included.
    // [`consult`] is the call, so that a host which asked the same question a round trip earlier
    // and is now acting on the answer cannot be told something different (ADR 0874).
    if let Some(operation) = plan.operation() {
        for (position, at) in wanted.iter().enumerate() {
            let Some(document) = opened.get(position) else {
                continue;
            };
            match consult(policy.restrictions, document, operation) {
                Consulted::Proceed => {}
                Consulted::Refuse { operation, reasons } => {
                    return Err(Refusal::Restricted { operation, reasons });
                }
                Consulted::Ask { operation, reasons } => {
                    return Err(Refusal::Unanswered { operation, reasons });
                }
                Consulted::Warn { operation, reasons } => report.warnings.push(Warning {
                    source: *at,
                    page: None,
                    detail: format!("this document restricts {operation}: {reasons}"),
                }),
            }
        }
    }

    // Every verb but `merge` reads one document, and `Plan::sources` gave it first.
    let first = opened.first().ok_or(Refusal::NoSuchSource {
        at: plan.source(),
        count: sources.len(),
    })?;
    match plan {
        Plan::Render(plan) => render::run(plan, first, sinks, budget, &mut report)?,
        Plan::Images(plan) => images::run(plan, first, sinks, &mut report)?,
        Plan::Attachments(plan) => attachments::run(plan, first, sinks, &mut report)?,
        Plan::Split(plan) => split::run(plan, first, sinks, &mut report)?,
        Plan::Merge(plan) => merge::run(plan, &wanted, &opened, sinks, &mut report)?,
        Plan::Pages(plan) => pages::run(plan, 0, &opened, sinks, &mut report)?,
        Plan::Optimize(plan) => optimize::run(plan, 0, &opened, sinks, &mut report)?,
        Plan::Update(plan) => update::run(plan, &wanted, &opened, sinks, &mut report)?,
    }
    Ok(report)
}

/// What [`apply`] would decide about one operation on one document, asked *before* committing to
/// the operation.
///
/// **The first half of the two round trips ADR 0874 chose**, and the reason it exists: the
/// decision is taken where the document is — inside `pdf-vfs`'s confined worker, inside this
/// crate's own `apply` — and a confined process has no channel to a person by construction (RFC
/// 0003 section 6). So `Level::Ask` degraded to a refusal in every caller. Asking first turns the
/// question into data: a host that has somewhere to put it — a message box, a terminal, a
/// window — gets the level, the operation and the sentence, puts it to the person, and then
/// issues the operation with the answer in hand.
///
/// It carries no restriction values, only their wording, because the *policy* is
/// `pdf_model::restriction::decide`'s and stays there: what crosses a boundary is a question and
/// an answer, never a second copy of the rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Consulted {
    /// Nothing to say: the document asserts nothing against this operation, or the level is
    /// `Level::Off`. [`apply`] would do it in silence.
    Proceed,
    /// [`apply`] would do it and say these afterwards — `CLAUDE.md`'s *warn*.
    Warn {
        /// The operation, as `Operation::as_str` words it.
        operation: &'static str,
        /// Every reason the document gave, worded, joined by `; `.
        reasons: String,
    },
    /// The person is to be asked — `CLAUDE.md`'s *ask*, and the one verdict this type exists for.
    ///
    /// A caller that gets this and has somebody to ask puts [`Consulted::question`] to them; one
    /// that has nobody says so and does not proceed, which is what [`Refusal::Unanswered`] is.
    Ask {
        /// The operation, as `Operation::as_str` words it.
        operation: &'static str,
        /// Every reason the document gave, worded, joined by `; `.
        reasons: String,
    },
    /// [`apply`] would refuse it — `CLAUDE.md`'s *on*. Asking here would be theatre: the level
    /// says the document's word stands, so there is nothing for a person to decide.
    Refuse {
        /// The operation, as `Operation::as_str` words it.
        operation: &'static str,
        /// Every reason the document gave, worded, joined by `; `.
        reasons: String,
    },
}

impl Consulted {
    /// The question to put to a person, worded once here so that four hosts cannot word it four
    /// ways.
    ///
    /// `None` for every verdict but [`Consulted::Ask`], because the other three are statements
    /// rather than questions and a host that showed them as a dialogue would be asking somebody
    /// to decide something already decided.
    #[must_use]
    pub fn question(&self) -> Option<String> {
        match self {
            Self::Ask { operation, reasons } => Some(format!(
                "This document restricts {operation}: {reasons}. Do it anyway?"
            )),
            Self::Proceed | Self::Warn { .. } | Self::Refuse { .. } => None,
        }
    }

    /// Every reason the document gave, worded; empty for [`Consulted::Proceed`].
    #[must_use]
    pub fn reasons(&self) -> &str {
        match self {
            Self::Proceed => "",
            Self::Warn { reasons, .. }
            | Self::Ask { reasons, .. }
            | Self::Refuse { reasons, .. } => reasons,
        }
    }

    /// The operation asked about, as `Operation::as_str` words it; empty for
    /// [`Consulted::Proceed`], which is the one verdict that names nothing.
    #[must_use]
    pub fn operation(&self) -> &'static str {
        match self {
            Self::Proceed => "",
            Self::Warn { operation, .. }
            | Self::Ask { operation, .. }
            | Self::Refuse { operation, .. } => operation,
        }
    }
}

/// What [`apply`] would decide about `operation` on this document, without doing it.
///
/// **The one place this crate asks the policy**, and [`apply_borrowed`] is its other caller:
/// `pdf_model::restriction::decide` is called here and nowhere else in this crate, so a host that
/// asks first and then acts gets the same answer twice rather than two readings that could
/// disagree. `CLAUDE.md` principle 3: "the *policy* is asked, once, in a place a host can
/// supply".
#[must_use]
pub fn consult(level: Level, document: &Document, operation: Operation) -> Consulted {
    let reasons = |restrictions: &[Restriction]| {
        restrictions
            .iter()
            .map(|restriction| describe_restriction(operation, *restriction))
            .collect::<Vec<_>>()
            .join("; ")
    };
    let operation_word = operation.as_str();
    match pdf_model::restriction::decide(level, document, operation, None, None) {
        Verdict::Proceed => Consulted::Proceed,
        Verdict::Warn(restrictions) => Consulted::Warn {
            operation: operation_word,
            reasons: reasons(&restrictions),
        },
        Verdict::Ask(restrictions) => Consulted::Ask {
            operation: operation_word,
            reasons: reasons(&restrictions),
        },
        Verdict::Refuse(restrictions) => Consulted::Refuse {
            operation: operation_word,
            reasons: reasons(&restrictions),
        },
    }
}

/// One restriction, worded for a batch tool's stderr.
///
/// `pdf_model::restriction` answers with clauses and levels and words nothing, for the reason
/// its module comment gives; `viewer_core::notes` words the same list for a window, and this is
/// the same list worded for a pipe — shorter, because a line on stderr is read beside a command
/// rather than in a dialogue.
fn describe_restriction(operation: Operation, restriction: Restriction) -> String {
    use pdf_model::signature::Modification;
    match restriction {
        // §7.6.4.2's Table 22, the bit `Operation::bit` chose for this document's revision.
        Restriction::AccessDenied { bit } => {
            format!("Table 22 bit {} is clear", bit.position())
        }
        // §12.8.2.2's Table 257, and §12.8.6's sentence that makes it binding: "PDF processors
        // shall enforce the permissions specified by the P entry".
        Restriction::Certified { level } => match level {
            Modification::None => "its author certified it as final (§12.8.2.2's /P 1)".to_owned(),
            Modification::FormFilling => format!(
                "its author's certification permits only form filling and signing (§12.8.2.2's \
                 /P 2), not {}",
                operation.as_str()
            ),
            Modification::FormFillingAndAnnotation => format!(
                "its author's certification permits form filling, signing and annotation \
                 (§12.8.2.2's /P 3), not {}",
                operation.as_str()
            ),
            Modification::Unknown(value) => {
                format!(
                    "its author's certification states /P {value}, which Table 257 does not define"
                )
            }
        },
        // Neither names a field or an annotation this crate's verbs touch; `decide` is asked
        // with no field and no annotation, so neither can arrive. Worded all the same, because
        // a variant a match cannot word is a sentence waiting to be missing.
        Restriction::FieldLocked => "a signature locks the field (§12.7.5.5)".to_owned(),
        Restriction::FieldCovered => {
            "a signature's FieldMDP transform covers the field (§12.8.2.4)".to_owned()
        }
        Restriction::AnnotationLocked => {
            "the annotation's LockedContents flag is set (Table 167 bit 10)".to_owned()
        }
    }
}

/// The words for an interpreter report, which `pdf-model` deliberately does not word for a
/// person — `viewer-core` does that for a window, in its own register. The `Debug` form is what
/// `tools/pdf-retrieve` prints for the same list, so the two programs say one thing.
pub(crate) fn describe(unsupported: &pdf_model::content::Unsupported) -> String {
    format!("{unsupported:?}")
}
