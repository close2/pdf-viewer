//! A PDF document as a directory tree — RFC 0003's shared core, and nothing of either face.
//!
//! # What this is
//!
//! The owner's idea, in RFC 0003 section 1's words: "once a document is a directory, every tool
//! the user already has becomes a PDF tool. `cp doc.pdf/pages/0007.pdf ~/` extracts a page. `ls
//! doc.pdf/images/` inventories the artwork. `grep -r` searches the text." Two faces are planned
//! over it — a KIO worker so Dolphin browses into a PDF as it browses into a tar, and a FUSE
//! filesystem so every program on the machine sees the same tree — and **neither is in this
//! crate**. RFC 0003 section 7 puts the layout, the cache, the generation rules and the broker
//! side of the worker protocol here, and says why: "[t]he faces contain *no* layout knowledge —
//! adding `fonts/` one day is a core change that both faces grow simultaneously".
//!
//! # The five pieces, and where each lives
//!
//! - [`layout`] is the tree, as one declarative table: path pattern → generator → write mapping.
//!   It is the whole design, and it is data rather than control flow so that a reviewer can read
//!   the tree without reading the code that walks it.
//! - [`worker`] is RFC 0003 section 6's confined side: every question that requires looking at a
//!   PDF is a [`worker::Query`], and this crate holds no `Document` and calls no reader.
//! - [`generation`] is the key, and section 5.4's consistency rule, which is a **correctness**
//!   requirement — a generation served after the document changed is a wrong answer given
//!   silently.
//! - [`cache`] is what makes section 5.5's "no virtual file is stat'd before it is generated"
//!   affordable, since every `cp` is a `stat` and then a `read`.
//! - [`commit`] is the transaction: a POSIX write is `create`, `write` several times, `flush` and
//!   `close`, and section 5.4 makes the third of those the moment the document changes.
//!
//! # Reads and writes, and what a write is not
//!
//! Every read of section 5.1 and every write of section 5.2 are here; every refusal of section
//! 5.3 is refused with the sentence that says why it will still be refused. [`Vfs::shortfalls`]
//! is what the layout declares and this does not do, so a face can print it rather than a person
//! discovering it (trap 5).
//!
//! **A write is never a rewrite.** `CLAUDE.md` permits exactly one thing to be done to a
//! document somebody already has open — §7.5.6's incremental update, "appended to the end of the
//! file, leaving its original contents intact" — so `pdf_transform`'s in-place `update` verb is
//! what every one of the five goes through, and the broker checks the clause's own property
//! against the file before it writes a byte. What that buys is stated in [`commit`], along with
//! what a torn write looks like, what an abandoned one leaves behind, and why our own commit does
//! not look to the tree like somebody else editing the file underneath it.
//!
//! # What a face has to do, and what it must not
//!
//! Call [`Vfs::list`], [`Vfs::stat`], [`Vfs::open`] and [`Handle::read`]; for a write,
//! [`Vfs::create`], [`Vfs::write_at`], [`Vfs::flush`] and [`Vfs::release`], which are `open`,
//! `write`, `flush` and `release` with the names the kernel gives them; map [`VfsError`] onto its
//! own errors with [`VfsError::errno`], and log the sentence beside it where its protocol has no
//! channel for one — which is FUSE's poverty and RFC 0003 section 5.3's reason for insisting the
//! sentence exist here. What it must not do is decide anything about the tree: a face that knows
//! that `pages/` holds PDFs has taken layout knowledge out of this table, and the next directory
//! added would have to be added twice; a face that chose its own `errno` for a refusal is the
//! same mistake one layer down.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod cache;
pub mod commit;
mod confined;
pub mod generation;
pub mod layout;
pub mod path;
mod serve;
mod wire;
pub mod worker;

use std::sync::{Arc, Mutex};

use pdf_transform::{Budget, Operation, Policy};

use crate::cache::Cache;
use crate::commit::Staged;
use crate::generation::{Backing, Generation};
use crate::layout::{Generator, Kind, Reason, Route, Write, WriteMapping};
use crate::worker::{Answer, Query, Worker, WorkerError, Workers};

pub use crate::commit::{Abandoned, Committed, Errno, Pending, Provenance, StagedId};
pub use crate::confined::{Confined, ConfinedWorkers};
pub use crate::generation::{FileBacking, MemoryBacking};
pub use crate::serve::{
    WORKER_PATH_VARIABLE, WORKER_PROGRAM, WorkerLimits, confine, message_budget, serve,
};
pub use crate::worker::InProcessWorkers;
pub use pdf_font::provider::MachineFaces;

/// What [`Vfs::consult`] answers: `CLAUDE.md` principle 3's verdict for one operation, with the
/// document's own reasons worded.
///
/// Re-exported rather than redefined, because it is `pdf_transform::apply`'s own answer to the
/// same question — which is what makes asking first and acting afterwards one reading.
pub use pdf_transform::Consulted;

/// The resolutions `renders/` offers.
///
/// RFC 0003 section 4 makes this the core's decision rather than a mount option, and says why:
/// "a KIO URL has no mount options and two faces must show one tree". 150 dpi is poppler's
/// `pdftoppm` default and the modern-screen answer; 300 dpi is the de-facto print-grade one.
pub const RESOLUTIONS: &[u32] = &[150, 300];

/// The ceilings and the policy a face supplies.
#[derive(Debug, Clone)]
pub struct Config {
    /// How many bytes of generated content the cache may hold. Explicit, in principle-3 style:
    /// nothing here grows without a number beside it.
    pub cache_bytes: usize,
    /// The transform seam's own ceilings — `pdf_syntax::Limits` and the pixels one rendered page
    /// may have.
    pub budget: Budget,
    /// What the host decides about the document's assertions over its reader.
    ///
    /// `CLAUDE.md` principle 3: a document's restrictions are the reader's to set, they have four
    /// levels, and the policy is asked **once, in a place a host can supply**. This is that
    /// place for a mount, and the default is `Level::Off` because the program is the reader's —
    /// the same default and the same argument as `pdf_transform::Policy`.
    pub policy: Policy,
    /// The resolutions `renders/` offers. [`RESOLUTIONS`] unless a host states otherwise.
    pub resolutions: Vec<u32>,
    /// How many bytes one write in flight may hold before it is refused.
    ///
    /// A `cp` of a forty-gigabyte file into `attachments/` is a `create` and then writes, and the
    /// bytes have nowhere to go but memory until the `flush` that validates them. So the ceiling
    /// is explicit, in principle-3 style, and a write past it is refused by name at the write
    /// that crosses it rather than when the machine runs out.
    pub max_staged_bytes: usize,
    /// The most entries one directory may list.
    ///
    /// A listing is built from what the document says, so its length is the document's to choose
    /// and therefore needs a ceiling: a name tree with a million keys is a listing no file
    /// manager survives. Past this the listing is refused by name rather than truncated, because
    /// a truncated directory is a wrong answer that looks like a right one.
    pub max_entries: usize,
}

impl Default for Config {
    /// 64 MiB of cache, the transform seam's own budget, restrictions off, both resolutions,
    /// 512 MiB of one write in flight, and 65 536 entries a directory.
    ///
    /// The cache figure is a stated choice rather than a measurement: it is a few pages of a
    /// large document at 300 dpi, which is the working set a `cp -r` of one directory has. A
    /// face that knows better states its own.
    fn default() -> Self {
        Self {
            cache_bytes: 64 << 20,
            budget: Budget::default(),
            policy: Policy::default(),
            resolutions: RESOLUTIONS.to_vec(),
            max_staged_bytes: 512 << 20,
            max_entries: 1 << 16,
        }
    }
}

/// Why an operation on the tree could not be answered.
#[derive(Debug, thiserror::Error)]
pub enum VfsError {
    /// Nothing in the layout, or nothing in this document, is at that path.
    #[error("{0}: no such file or directory in this document")]
    NoSuchPath(String),
    /// A file was listed.
    #[error("{0}: not a directory")]
    NotADirectory(String),
    /// A directory was read.
    #[error("{0}: is a directory")]
    IsADirectory(String),
    /// The write is refused, and [`Refused`] says whether by design or as unbuilt.
    #[error("{0}")]
    Refused(#[from] Refused),
    /// The backing file could not be stat'd or opened.
    #[error("{document}: {error}")]
    Backing {
        /// What the face calls it.
        document: String,
        /// The file system's own error.
        error: std::io::Error,
    },
    /// The worker could not answer.
    #[error("{0}")]
    Worker(#[from] WorkerError),
    /// The document changed while a write was staged against it.
    ///
    /// RFC 0003 section 5.4's key, applied to a write rather than a read: the update the worker
    /// computes is a function of the document it was computed from, so committing it over a file
    /// somebody else has edited would throw their edit away. Refused instead, by name.
    #[error(
        "{path}: the document changed while this write was in flight, so committing it would \
         discard whatever changed it; nothing was written"
    )]
    Changed {
        /// Which path was being written.
        path: String,
    },
    /// More bytes than [`Config::max_staged_bytes`] allows one write in flight.
    #[error("{path}: a write in flight may hold {ceiling} bytes, and this one reached {reached}")]
    TooLarge {
        /// Which path.
        path: String,
        /// How far it got.
        reached: usize,
        /// The ceiling.
        ceiling: usize,
    },
    /// A token no write in flight answers to.
    #[error("no write in flight has the token {0}")]
    NoSuchWrite(u64),
    /// A name this directory cannot file.
    #[error("{path}: {detail}")]
    Unnameable {
        /// Which path.
        path: String,
        /// Why the name cannot be used.
        detail: String,
    },
    /// §7.7.4's tree already files an embedded file under this name.
    #[error(
        "{path}: §7.7.4's /EmbeddedFiles tree already files a file under this name; remove it \
         before writing another"
    )]
    AlreadyFiled {
        /// Which path.
        path: String,
    },
    /// A directory the document would fill past [`Config::max_entries`].
    #[error("{path}: this document would list {count} entries here, and the ceiling is {ceiling}")]
    TooManyEntries {
        /// Which directory.
        path: String,
        /// How many it would have.
        count: usize,
        /// The ceiling.
        ceiling: usize,
    },
}

impl VfsError {
    /// The `errno` a face returns for this, and the one place either face may learn it.
    ///
    /// RFC 0003 section 7 keeps every decision about the tree here rather than in a face, and
    /// this is one of them: a KIO worker mapping these onto `KIO::Error` and a FUSE daemon
    /// handing them to the kernel must agree about what a refused write *is*, and a face that
    /// chose its own numbers would be the second copy of a decision.
    ///
    /// FUSE carries no message beside the number — RFC 0003 section 5.3: it "returns `EROFS` for
    /// the derived directories and `EPERM` with no message channel — which is FUSE's poverty,
    /// and why the mount also logs each refusal's sentence to its own stderr/journal". So a face
    /// logs `self.to_string()` beside every one of these.
    #[must_use]
    pub fn errno(&self) -> Errno {
        match self {
            // A path nothing names, and a token no write in flight answers to: both are a name
            // this tree does not have.
            Self::NoSuchPath(_) | Self::NoSuchWrite(_) => Errno::NoSuchFile,
            Self::NotADirectory(_) => Errno::NotADirectory,
            Self::IsADirectory(_) => Errno::IsADirectory,
            Self::Refused(refused) => refused.errno(),
            // The document could not be stat'd, opened or replaced. The file system's own kind
            // is the honest answer where there is one, because a mount whose backing file has
            // been deleted should say `ENOENT` rather than `EIO`.
            Self::Backing { error, .. } => match error.kind() {
                std::io::ErrorKind::NotFound => Errno::NoSuchFile,
                std::io::ErrorKind::PermissionDenied => Errno::PermissionDenied,
                std::io::ErrorKind::IsADirectory => Errno::IsADirectory,
                _ => Errno::InputOutput,
            },
            Self::Worker(error) => match error {
                // `CLAUDE.md` principle 3's two levels that do not proceed. Both are `EACCES`
                // and the *sentence* is what tells them apart, which is FUSE's poverty rather
                // than a choice: there is no number for "somebody would have been asked".
                WorkerError::Restricted(_) | WorkerError::Unanswerable(_) => {
                    Errno::PermissionDenied
                }
                // §7.6.4.1's password, which a mount has no way to ask for yet.
                WorkerError::PasswordRequired(_) => Errno::PermissionDenied,
                WorkerError::NotPresent(_) => Errno::NoSuchFile,
                // The document, or the file being written into it, could not be read as one.
                // `EIO` and not `EINVAL`, because the caller's *request* was well formed and it
                // is the bytes that were not — which is what a `cp` of a truncated PDF into
                // `pages/` is, and the one a `close(2)` should report.
                WorkerError::Refused(_)
                | WorkerError::Declined { .. }
                | WorkerError::Mismatched { .. }
                | WorkerError::Transport(_) => Errno::InputOutput,
            },
            Self::TooManyEntries { .. } => Errno::Overflow,
            Self::Changed { .. } => Errno::Stale,
            Self::TooLarge { .. } => Errno::TooBig,
            Self::Unnameable { .. } => Errno::Invalid,
            Self::AlreadyFiled { .. } => Errno::Exists,
        }
    }
}

/// A write this program will not do, and the two different things that can mean.
///
/// The distinction is the point, and RFC 0003 section 5.3 draws it: a *refusal by design* is a
/// file verb whose meaning would have to be invented, and it will still be refused when the write
/// side lands; a *declared and unbuilt* mapping is one this layout has decided the meaning of and
/// has not implemented. A face that could not tell them apart would report "read-only file
/// system" for both, and the design would be invisible from outside.
#[derive(Debug, thiserror::Error)]
pub enum Refused {
    /// Refused by design, for one of [`Reason`]'s six.
    #[error("{path}: {}", reason.sentence())]
    ByDesign {
        /// Which path.
        path: String,
        /// Why.
        reason: Reason,
    },
    /// The layout declares what a verb means on this row and the *other* verb was asked.
    ///
    /// Unreachable through this crate's own entry points, which check the row before they accept
    /// a byte — and written rather than asserted, for the reason
    /// [`worker::WorkerError::Mismatched`] is: a layout table that grew a row this code did not
    /// grow an arm for should be a sentence rather than a panic.
    #[error("{path}: {} is what this verb means here, and that is not what was asked",
            mapping.name())]
    WrongVerb {
        /// Which path.
        path: String,
        /// What the layout says the verb means.
        mapping: Write,
    },
}

impl Refused {
    /// The `errno` this refusal is.
    ///
    /// RFC 0003 section 5.3 states two of these outright — FUSE "returns `EROFS` for the derived
    /// directories and `EPERM` with no message channel" — and the rest follow the same reading:
    /// a *derived* file is a read-only view of something else, and everything else this program
    /// declines is a thing it will not do, which is what `EPERM` says.
    #[must_use]
    pub fn errno(&self) -> Errno {
        match self {
            Self::ByDesign { reason, .. } => match reason {
                Reason::Derived => Errno::ReadOnly,
                Reason::LayoutIsNotWritable
                | Reason::TextIsNotAByteStream
                | Reason::ImageReplacementNotDesigned
                | Reason::ReorderIsAmbiguous
                | Reason::NotOneOfTheFiveVerbs => Errno::OperationNotPermitted,
            },
            Self::WrongVerb { .. } => Errno::NotImplemented,
        }
    }

    /// The sentence a face shows a person, or logs where its protocol carries no message.
    ///
    /// RFC 0003 section 5.3: FUSE "returns `EROFS` for the derived directories and `EPERM` with
    /// no message channel — which is FUSE's poverty, and why the mount also logs each refusal's
    /// sentence to its own stderr/journal".
    #[must_use]
    pub fn sentence(&self) -> String {
        self.to_string()
    }
}

impl Write {
    /// What this mapping is called in a refusal.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::InsertPages => "inserting the copied document's pages at this position",
            Self::DeletePage => "deleting this page",
            Self::EmbedFile => "embedding the copied file as a §7.11.4 embedded file",
            Self::RemoveAttachment => "removing this embedded file from §7.7.4's name tree",
            Self::SetInformation => "setting the §14.3.3 entries this file states",
            Self::Refused(_) => "nothing",
        }
    }
}

/// One entry of a directory listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    /// The name, with no solidus in it.
    pub name: String,
    /// Whether it is itself a directory.
    pub kind: Kind,
}

/// What a `stat` answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attributes {
    /// Directory or file.
    pub kind: Kind,
    /// The file's true size in bytes, `None` for a directory.
    ///
    /// **True, never estimated**, which is RFC 0003 section 5.5's rule and the reason a `stat`
    /// generates: "FUSE `stat` must state sizes for files whose bytes do not exist yet, and the
    /// kernel clamps reads at the stated size … an under-estimate silently truncates a page".
    pub size: Option<u64>,
    /// Which generation of the document answered.
    pub generation: Generation,
}

/// One virtual file, materialised at the generation it was opened under.
///
/// RFC 0003 section 5.4: "an open virtual file keeps the generation it was opened under (its
/// bytes are already materialised in the cache); the *next* open sees the new generation. No
/// reader ever receives a splice of two generations." That property is held by this type's
/// *shape*: the bytes are here, whole, and a read is a copy out of them — there is no path by
/// which a later generation could reach a handle already open.
#[derive(Debug, Clone)]
pub struct Handle {
    /// Which path it is.
    path: String,
    /// Which generation produced it.
    generation: Generation,
    /// The bytes.
    bytes: Arc<[u8]>,
}

impl Handle {
    /// Which path this is.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Which generation of the document these bytes are from.
    #[must_use]
    pub fn generation(&self) -> Generation {
        self.generation
    }

    /// How many bytes there are.
    #[must_use]
    pub fn len(&self) -> u64 {
        u64::try_from(self.bytes.len()).unwrap_or(u64::MAX)
    }

    /// Whether the file is empty, which a page's text may well be.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// `count` bytes from `offset`, or fewer at the end.
    ///
    /// A short answer at the end and an empty answer past it, which is what `read(2)` does; a
    /// face passes the kernel's own offset and length through.
    #[must_use]
    pub fn read(&self, offset: u64, count: usize) -> &[u8] {
        let Ok(from) = usize::try_from(offset) else {
            return &[];
        };
        let Some(tail) = self.bytes.get(from..) else {
            return &[];
        };
        tail.get(..count).unwrap_or(tail)
    }

    /// All of it.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// One thing the layout declares and this round does not do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortfall {
    /// Which row of the layout it is about, by its pattern.
    pub pattern: &'static str,
    /// What is missing, in a sentence a face can print.
    pub detail: &'static str,
}

/// The document, as a tree.
#[derive(Debug)]
pub struct Vfs {
    /// The file behind it.
    backing: Box<dyn Backing>,
    /// What makes a worker for one generation.
    workers: Box<dyn Workers>,
    /// The ceilings and the policy.
    config: Config,
    /// Generated content, keyed by generation and path.
    cache: Cache,
    /// The generation being served, the writes in flight, and the key this tree's own last
    /// commit left behind.
    ///
    /// **One lock over all three**, and that is the design rather than an economy: a commit has
    /// to read the generation, compute an update from it, replace the file and build the next
    /// generation without any other operation falling between the two — which is exactly what
    /// holding this across the whole of [`Vfs::flush`] gives. Every read blocks for the length of
    /// one commit, which is the cost of a transaction and is what a caller of a file system
    /// expects `close(2)` to be doing.
    state: Mutex<Serving>,
    /// How many times a virtual file's bytes have actually been produced.
    ///
    /// The instrument the cache's own numbers do not give: bytes held say what is remembered,
    /// this says what was *done*. A gate that means "and it did not generate it again" has no
    /// other way to say so — a size is the same number whether it was remembered or recomputed,
    /// which is exactly what made a test of [`Cache::size_of`] pass without it (round 911, trap
    /// 13). An `AtomicU64` because every operation here is behind `&self`.
    generated: std::sync::atomic::AtomicU64,
}

/// Everything one document's mount holds that a commit changes.
#[derive(Debug, Default)]
struct Serving {
    /// The generation being served, where anything has been read yet.
    current: Option<Arc<Current>>,
    /// The key this tree's own last commit left on the file, until the generation for it is
    /// built. [`Provenance`] is what it is for.
    ours: Option<Generation>,
    /// The writes in flight, by their tokens.
    staged: std::collections::BTreeMap<u64, Staged>,
    /// The next token, which is never reused inside one mount.
    next_token: u64,
    /// How many generation transitions have been [`Provenance::Foreign`] since this tree opened.
    ///
    /// Monotonic, and only ever compared for equality: [`Staged::foreign_edits`] says what it is
    /// for. A counter rather than the served generation's own [`Provenance`], because that flag
    /// describes the *last* transition and a write can be staged across several.
    foreign_edits: u64,
}

/// A worker with `CLAUDE.md` principle 3's *ask* level held beside it.
///
/// **Where the two round trips of ADR 0874 meet.** [`Vfs::consult`] puts the question to the
/// worker and records that it was asked; a face puts it to a person; [`Vfs::answer`] records the
/// yes; and the very next query that performs the operation spends it, once, by going through
/// [`Worker::ask_consented`] instead of [`Worker::ask`].
///
/// **Here rather than on [`Vfs`], and that is the design rather than an economy.** A consent is
/// about one document as it stood when the question was put, so it belongs to the generation:
/// a `Current` thrown away because somebody else edited the file underneath the mount takes the
/// answer with it, and nothing has to remember to. And every question this crate asks goes
/// through `current.worker`, so putting the spend here is what makes it impossible for a call
/// site to forget — there are fifteen of them and no list of which ones matter.
#[derive(Debug)]
struct Consenting {
    /// What actually answers.
    inner: Box<dyn Worker>,
    /// The question outstanding and the answer standing.
    asked: Mutex<Asked>,
}

/// One question put to a person, and one answer held until it is spent.
///
/// One question outstanding at a time and a second replaces it — `viewer_core`'s own rule for
/// `Event::Asking`, so that the two boundaries in this tree behave the same way.
#[derive(Debug, Default)]
struct Asked {
    /// The operation [`Vfs::consult`] last answered `Consulted::Ask` about, until it is
    /// answered.
    outstanding: Option<Operation>,
    /// The operation a person said yes to, until the query that performs it is asked.
    consented: Option<Operation>,
}

impl Consenting {
    /// A worker with nothing asked and nothing answered.
    fn new(inner: Box<dyn Worker>) -> Self {
        Self {
            inner,
            asked: Mutex::new(Asked::default()),
        }
    }

    /// The lock, poison and all: a panic in a caller must not make this tree unusable, and the
    /// worst a poisoned answer can be is one consent spent or not spent.
    fn held(&self) -> std::sync::MutexGuard<'_, Asked> {
        self.asked
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Records that a person is being asked about `operation`.
    fn putting(&self, operation: Operation) {
        let mut held = self.held();
        held.outstanding = Some(operation);
    }

    /// Records that nothing is outstanding — a consultation that came back anything but *ask*.
    fn nothing_outstanding(&self) {
        let mut held = self.held();
        held.outstanding = None;
    }

    /// The person's answer, and whether there was a question for it to be an answer to.
    ///
    /// A `no` forgets the question and says nothing further, which is what a declined dialogue
    /// means everywhere else in this tree: the operation is simply not done.
    fn answered(&self, proceed: bool) -> bool {
        let mut held = self.held();
        let Some(operation) = held.outstanding.take() else {
            return false;
        };
        held.consented = proceed.then_some(operation);
        true
    }
}

impl Worker for Consenting {
    fn ask(&self, query: &Query) -> Result<Answer, WorkerError> {
        // Spent, or not, before the question is asked — and spent *once*: a yes to deleting one
        // page is not a yes to deleting every page after it.
        let spend = {
            let mut held = self.held();
            match (held.consented, query.operation()) {
                (Some(given), Some(wanted)) if given == wanted => {
                    held.consented = None;
                    true
                }
                _ => false,
            }
        };
        if spend {
            self.inner.ask_consented(query)
        } else {
            self.inner.ask(query)
        }
    }

    fn ask_consented(&self, query: &Query) -> Result<Answer, WorkerError> {
        self.inner.ask_consented(query)
    }

    fn is_alive(&self) -> bool {
        self.inner.is_alive()
    }
}

/// One generation of the document, and what has been read of it so far.
#[derive(Debug)]
struct Current {
    /// The key this was built for.
    generation: Generation,
    /// The confined side, over this generation's bytes, with `CLAUDE.md` principle 3's
    /// outstanding question and standing answer beside it.
    worker: Consenting,
    /// How many pages ISO 32000-2 §7.7.3.2's tree holds — the one thing read eagerly, because
    /// RFC 0003 section 5.1 says listing the root "reads nothing but the page count".
    pages: usize,
    /// §7.11.4's embedded files, once something has asked for them.
    attachments: Mutex<Option<Arc<Vec<Embedded>>>>,
    /// Whose edit produced this generation.
    provenance: Provenance,
}

/// One embedded file, under the name it takes in the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Embedded {
    /// The name `attachments/` lists it under: the document's own, made safe for a directory.
    file_name: String,
    /// The name the document files it under, which is what the worker is asked for.
    document_name: String,
}

impl Vfs {
    /// A tree over `backing`, with workers from `workers`.
    ///
    /// Nothing is read here: the first operation builds the generation. That is `CLAUDE.md`'s
    /// launch rule applied to a mount — a face that mounts a thousand documents has not opened
    /// one of them.
    #[must_use]
    pub fn new(backing: Box<dyn Backing>, workers: Box<dyn Workers>, config: Config) -> Self {
        Self {
            cache: Cache::new(config.cache_bytes),
            backing,
            workers,
            config,
            state: Mutex::new(Serving::default()),
            generated: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// The layout table, whole, for a face that wants to describe the tree.
    #[must_use]
    pub fn layout(&self) -> &'static [Route] {
        layout::LAYOUT
    }

    /// What writing to `path` and deleting it would each mean, per the layout table.
    ///
    /// `None` for a path the layout does not name at all. A face can call this before it accepts
    /// a write, which is how a KIO worker decides whether to advertise the operation.
    #[must_use]
    pub fn write_meaning(&self, path: &str) -> Option<WriteMapping> {
        path::resolve(path).map(|(route, _)| route.write)
    }

    /// Everything the layout declares that this round does not do, by name.
    #[must_use]
    pub fn shortfalls(&self) -> Vec<Shortfall> {
        let mut out: Vec<Shortfall> = vec![Shortfall {
            pattern: "/attachments/NAME",
            detail: "a write to a name §7.7.4's tree already files is refused rather than \
                     replacing it, because an in-place replacement is two updates and this \
                     verb writes one; remove it and write it again",
        }];
        out.push(Shortfall {
            pattern: "/pages/NNNN.pdf",
            detail: "an insertion carries the pages and not what the incoming document says \
                     about them at the document level — its form, its optional content, its \
                     outline, its name trees and its structure tree are each named in a warning \
                     rather than carried",
        });
        out.push(Shortfall {
            pattern: "/attachments",
            detail: "a document with a §12.3.5 /Collection is listed flat rather than under the \
                     folder schema its collection states, which RFC 0003 section 4 asks for",
        });
        out.push(Shortfall {
            pattern: "/text/document.txt",
            detail: "the concatenation is built whole rather than streamed page by page, so its \
                     first byte costs the whole document",
        });
        out.push(Shortfall {
            pattern: "/",
            detail: "the cache has a memory bound and no disk bound, which RFC 0003 section 5.5 \
                     offers as optional",
        });
        out.push(Shortfall {
            pattern: "/",
            detail: "an encrypted document opens only under §7.6.4.1's default user password: a \
                     worker is created per generation and `viewer_core::Secret` is deliberately \
                     not Clone, so a mount that survived a change of the file would need the \
                     password re-supplied, and nothing here asks for one yet",
        });
        out
    }

    /// Which generation is being served, building it where nothing has been read yet.
    ///
    /// **This is RFC 0003 section 5.4's rule and every public operation begins with it**: the
    /// backing is asked for its key, and a key that differs from the one being served throws the
    /// generation away — the worker, the inventories and every cached output. A stale answer here
    /// is not a slow answer, it is a wrong one.
    ///
    /// # Errors
    ///
    /// [`VfsError::Backing`] where the file cannot be stat'd or opened, and [`VfsError::Worker`]
    /// where a worker cannot be started or the page count cannot be read.
    fn current(&self) -> Result<Arc<Current>, VfsError> {
        let mut held = self.held();
        self.current_in(&mut held)
    }

    /// The lock over everything a commit changes.
    fn held(&self) -> std::sync::MutexGuard<'_, Serving> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// [`Vfs::current`], with the lock already held — which is what a commit needs, because the
    /// old generation, the write and the new generation are one transaction.
    fn current_in(&self, held: &mut Serving) -> Result<Arc<Current>, VfsError> {
        let generation = self
            .backing
            .generation()
            .map_err(|error| VfsError::Backing {
                document: self.backing.describe(),
                error,
            })?;
        // **Two questions, and both have to be `yes`.** The key is RFC 0003 section 5.4's rule; the
        // second is RFC 0003 section 6's — a confined worker is killable by design, and one the
        // kernel has ended answers `false` for ever after. Asking it again would produce a second,
        // stranger error about a closed descriptor, so the generation is thrown away and the next
        // operation starts a fresh worker over the same file. `InProcess` is always alive, so this
        // costs the unconfined path an atomic load.
        if let Some(current) = held.current.as_ref()
            && current.generation == generation
            && current.worker.is_alive()
        {
            return Ok(Arc::clone(current));
        }
        let bytes = self.backing.bytes().map_err(|error| VfsError::Backing {
            document: self.backing.describe(),
            error,
        })?;
        let worker = Consenting::new(self.workers.spawn(
            bytes,
            None,
            self.config.policy,
            self.config.budget,
        )?);
        let pages = match worker.ask(&Query::PageCount)? {
            Answer::Count(pages) => pages,
            other => return Err(VfsError::Worker(mismatch(&other, "count"))),
        };
        // Before anything of the new generation is answered, everything of the old one is
        // forgotten. In this order, so that no window exists in which a caller could be handed
        // an entry keyed by a generation the document no longer has.
        self.cache.retain(generation);
        // Whose edit this is. `ours` is set by [`Vfs::flush`] under this same lock, so the only
        // way a generation is `Ours` is that this tree wrote the key that produced it; a key
        // that arrived any other way is somebody else's, whether or not a write of ours ever
        // happened.
        let provenance = if held.current.is_none() && held.ours.is_none() {
            Provenance::Opened
        } else if held.ours.take() == Some(generation) {
            Provenance::Ours
        } else {
            held.ours = None;
            held.foreign_edits = held.foreign_edits.saturating_add(1);
            Provenance::Foreign
        };
        let current = Arc::new(Current {
            generation,
            worker,
            pages,
            attachments: Mutex::new(None),
            provenance,
        });
        held.current = Some(Arc::clone(&current));
        Ok(current)
    }

    /// How many virtual files this tree has produced the bytes of, since it opened.
    ///
    /// Monotonic, and the counterpart to what the cache holds: see [`Vfs::generated`]'s field.
    #[must_use]
    pub fn generated(&self) -> u64 {
        self.generated.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Whose edit produced the generation being served.
    ///
    /// # Errors
    ///
    /// As [`Vfs::list`].
    pub fn provenance(&self) -> Result<Provenance, VfsError> {
        Ok(self.current()?.provenance)
    }

    /// The generation being served right now, for a face that wants to report it.
    ///
    /// # Errors
    ///
    /// As [`Vfs::list`].
    pub fn generation(&self) -> Result<Generation, VfsError> {
        Ok(self.current()?.generation)
    }

    /// How many pages the document has.
    ///
    /// # Errors
    ///
    /// As [`Vfs::list`].
    pub fn pages(&self) -> Result<usize, VfsError> {
        Ok(self.current()?.pages)
    }

    /// What is in a directory.
    ///
    /// # Errors
    ///
    /// [`VfsError::NoSuchPath`] for a path the layout does not name or the document does not
    /// have, [`VfsError::NotADirectory`] for a file, and the backing's or the worker's own.
    pub fn list(&self, path: &str) -> Result<Vec<DirEntry>, VfsError> {
        let current = self.current()?;
        let (route, captures) = locate_in(&current, path, &self.config.resolutions)?;
        let directory = canonical_path(path)?;
        if route.kind != Kind::Directory {
            return Err(VfsError::NotADirectory(path.to_owned()));
        }
        let mut entries = self.entries_of(&current, path, route, &captures)?;
        // A write in flight is *in* the tree: `cp` stats what it has just written, a file
        // manager shows the copy arriving, and a directory that named nothing there would fail
        // the copy it had accepted. `crate::commit` has the argument.
        for staged in self.pending() {
            let Some(name) = staged
                .path
                .strip_prefix(&directory)
                .and_then(|rest| rest.strip_prefix('/'))
                .filter(|rest| !rest.contains('/'))
            else {
                continue;
            };
            if !entries.iter().any(|entry| entry.name == name) {
                entries.push(DirEntry {
                    name: name.to_owned(),
                    kind: Kind::File,
                });
            }
        }
        if entries.len() > self.config.max_entries {
            return Err(VfsError::TooManyEntries {
                path: path.to_owned(),
                count: entries.len(),
                ceiling: self.config.max_entries,
            });
        }
        Ok(entries)
    }

    /// What one directory's own generator names, before the writes in flight join it.
    fn entries_of(
        &self,
        current: &Current,
        path: &str,
        route: &'static Route,
        captures: &path::Captures,
    ) -> Result<Vec<DirEntry>, VfsError> {
        let entries = match route.generator {
            Generator::Root => layout::children("/")
                .into_iter()
                .map(|child| DirEntry {
                    name: last_component(child.pattern),
                    kind: child.kind,
                })
                .collect(),
            Generator::PageOrdinals => page_names(current, "pdf"),
            Generator::Resolutions => self
                .config
                .resolutions
                .iter()
                .map(|dpi| DirEntry {
                    name: path::resolution_name(*dpi),
                    kind: Kind::Directory,
                })
                .collect(),
            Generator::RenderOrdinals => page_names(current, "png"),
            Generator::ImagePageOrdinals => (1..=current.pages)
                .map(|page| DirEntry {
                    name: path::page_name_stem(page, width(current)),
                    kind: Kind::Directory,
                })
                .collect(),
            Generator::ImageInventory => {
                let page = captures.page.ok_or(VfsError::NoSuchPath(path.to_owned()))?;
                let mut names: Vec<String> = images(current, page)?.keys().cloned().collect();
                names.sort();
                names
                    .into_iter()
                    .map(|name| DirEntry {
                        name,
                        kind: Kind::File,
                    })
                    .collect()
            }
            Generator::TextOrdinals => {
                let mut entries = page_names(current, "txt");
                entries.push(DirEntry {
                    name: String::from("document.txt"),
                    kind: Kind::File,
                });
                entries
            }
            Generator::AttachmentInventory => attachments(current)?
                .iter()
                .map(|embedded| DirEntry {
                    name: embedded.file_name.clone(),
                    kind: Kind::File,
                })
                .collect(),
            Generator::MetaNames => {
                let mut entries = vec![
                    DirEntry {
                        name: String::from("info.json"),
                        kind: Kind::File,
                    },
                    DirEntry {
                        name: String::from("outline.json"),
                        kind: Kind::File,
                    },
                ];
                // `xmp.xml` is listed only where §14.3.2's stream exists, because its content is
                // the stream's own bytes and there are none to invent. `info.json` and
                // `outline.json` are always listed: both are answers this program composes, and
                // a document that states neither an `/Info` nor an `/Outlines` has an empty
                // answer rather than no answer.
                if matches!(
                    current.worker.ask(&Query::MetadataStream)?,
                    Answer::Bytes(_)
                ) {
                    entries.push(DirEntry {
                        name: String::from("xmp.xml"),
                        kind: Kind::File,
                    });
                }
                entries
            }
            Generator::ExtractedPage
            | Generator::RenderedPage
            | Generator::ExtractedImage
            | Generator::PageText
            | Generator::DocumentText
            | Generator::ExtractedAttachment
            | Generator::Information
            | Generator::MetadataStream
            | Generator::Outline => return Err(VfsError::NotADirectory(path.to_owned())),
        };
        Ok(entries)
    }

    /// What a path is, and — for a file — how big.
    ///
    /// **A file is generated here.** RFC 0003 section 5.5: "[r]ule: no virtual file is stat'd
    /// before it is generated. `stat` on `pages/0007.pdf` generates (or finds cached) the bytes
    /// and reports the true size." The cache is what stops a `cp` paying for it twice.
    ///
    /// # Errors
    ///
    /// As [`Vfs::open`], plus [`VfsError::NoSuchPath`].
    pub fn stat(&self, path: &str) -> Result<Attributes, VfsError> {
        let current = self.current()?;
        if let Some(staged) = self.staged_at(path) {
            return Ok(Attributes {
                kind: Kind::File,
                size: Some(staged.len()),
                generation: staged.generation,
            });
        }
        let (route, _) = locate_in(&current, path, &self.config.resolutions)?;
        if route.kind == Kind::Directory {
            return Ok(Attributes {
                kind: Kind::Directory,
                size: None,
                generation: current.generation,
            });
        }
        // A size this generation has already produced, answered without producing it again.
        // RFC 0003 section 5.5's rule is that a `stat` may not *estimate* — "an under-estimate
        // silently truncates a page" — and a length taken off the bytes themselves is not an
        // estimate whether or not those bytes are still in the cache. `Cache::sizes` has what a
        // mount by hand measured this to be worth.
        let canonical = canonical_path(path)?;
        if let Some(size) = self.cache.size_of(current.generation, &canonical) {
            return Ok(Attributes {
                kind: Kind::File,
                size: Some(size),
                generation: current.generation,
            });
        }
        let handle = self.open(path)?;
        Ok(Attributes {
            kind: Kind::File,
            size: Some(handle.len()),
            generation: handle.generation,
        })
    }

    /// **Would this operation be restricted, and why** — the question a host puts to a person
    /// before committing to the operation.
    ///
    /// `CLAUDE.md` principle 3's four levels are `off`, `on`, *ask before the operation* and
    /// *warn before the operation*, and "a refusal that cannot become an 'ask' is the thing to
    /// avoid". RFC 0003 section 6 makes the *ask* level unaskable from inside: the decision is
    /// taken where the document is, which is a process with no channel to a person by
    /// construction. So the question crosses instead — this call — and the answer comes back
    /// through [`Vfs::answer`]; the operation is then issued exactly as it always was, and the
    /// yes is spent by the query that performs it (ADR 0874).
    ///
    /// A face with somewhere to put the question — KIO's `messageBox`, a window's dialogue, a
    /// terminal — puts it. A face with nowhere does not call this at all and gets
    /// [`worker::WorkerError::Unanswerable`] from the operation, which is what a mount gets and
    /// why the sentence names the poverty rather than hiding it.
    ///
    /// The level is the mount's own ([`Config::policy`]), so a tree at `off` answers
    /// `Consulted::Proceed` for everything and a face that always asks first costs one round
    /// trip and no dialogue.
    ///
    /// # Errors
    ///
    /// [`VfsError::NoSuchPath`] for a path the layout does not name, and the worker's own where
    /// the document cannot be opened at all.
    pub fn consult(&self, path: &str, verb: Verb) -> Result<Consulted, VfsError> {
        let current = self.current()?;
        // A write is asked about *before* the file exists — `cp new.pdf pages/0004.pdf` names a
        // position rather than an entry — so the write side is located the way `Vfs::create`
        // locates it and the other two the way a read does. Getting this wrong would make every
        // insertion's consultation a `NoSuchPath`, which is the shape of a face that cannot ask.
        let (route, _) = match verb {
            Verb::Write => Self::locate_for_write(&current, &canonical_path(path)?)?,
            Verb::Read | Verb::Delete => locate_in(&current, path, &self.config.resolutions)?,
        };
        // The layout table is what says which operation a path's verb performs, because it is
        // the one place a path's meaning is stated; `tests/a_write.rs` holds it to
        // `pdf_transform::Plan::operation`'s own answer rather than to this call.
        let operation = match verb {
            Verb::Read => route.generator.operation(),
            Verb::Write => route.write.on_write.operation(),
            Verb::Delete => route.write.on_delete.operation(),
        };
        let Some(operation) = operation else {
            // A verb this tree will not perform, or a listing that reads nothing the standard
            // restricts: there is no operation, so there is nothing to consult about. The
            // refusal such a path earns is the layout's own and is unrelated to policy.
            return Ok(Consulted::Proceed);
        };
        let consulted = match current.worker.ask(&Query::Consult { operation })? {
            Answer::Consulted(consulted) => consulted,
            other => return Err(VfsError::Worker(mismatch(&other, "a consultation"))),
        };
        if matches!(consulted, Consulted::Ask { .. }) {
            current.worker.putting(operation);
        } else {
            current.worker.nothing_outstanding();
        }
        Ok(consulted)
    }

    /// The person's answer to the question [`Vfs::consult`] last put — the second of ADR 0874's
    /// two round trips.
    ///
    /// `true` means *do it*: the next operation that performs the operation asked about runs at
    /// `pdf_model::restriction::Level::Off`, which is the level `CLAUDE.md` says "shall always
    /// be possible" and is what a person consenting to one operation has chosen for it. The
    /// consent is spent by that one query and by no other, and it goes when the generation goes.
    ///
    /// `false` forgets the question and does nothing else, because a question declined is
    /// neither the document doing something nor this program refusing — `viewer_core`'s own rule
    /// for `Command::Answer`.
    ///
    /// Answers whether there was a question outstanding to answer. `false` there is a face's
    /// defect rather than a person's: nothing asked, or the document moved underneath the mount
    /// while the dialogue was up and the generation that was asked about is gone.
    ///
    /// # Errors
    ///
    /// The backing's, where the generation cannot be read.
    pub fn answer(&self, proceed: bool) -> Result<bool, VfsError> {
        Ok(self.current()?.worker.answered(proceed))
    }

    /// Materialises a virtual file and hands back a handle onto its bytes.
    ///
    /// # Errors
    ///
    /// [`VfsError::NoSuchPath`], [`VfsError::IsADirectory`], and the worker's own refusal where
    /// the content cannot be produced — a codec the confined worker does not have, a page the
    /// rasteriser declined. Loud in every case (trap 5).
    pub fn open(&self, path: &str) -> Result<Handle, VfsError> {
        let current = self.current()?;
        if let Some(staged) = self.staged_at(path) {
            return Ok(staged);
        }
        let (route, captures) = locate_in(&current, path, &self.config.resolutions)?;
        if route.kind == Kind::Directory {
            return Err(VfsError::IsADirectory(path.to_owned()));
        }
        let canonical = canonical_path(path)?;
        if let Some(bytes) = self.cache.get(current.generation, &canonical) {
            return Ok(Handle {
                path: canonical,
                generation: current.generation,
                bytes,
            });
        }
        self.generated
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let bytes = self.generate(&current, route, &captures, &canonical)?;
        Ok(Handle {
            path: canonical.clone(),
            generation: current.generation,
            bytes: self.cache.put(current.generation, &canonical, bytes),
        })
    }

    /// Copying a file into the tree, whole, in one call.
    ///
    /// The transaction of [`Vfs::create`], [`Vfs::write_at`] and [`Vfs::flush`] with nothing
    /// between them — what a KIO `put` is, because "KIO's verb is already transactional"
    /// (RFC 0003 section 5.4), and what a test writes.
    ///
    /// # Errors
    ///
    /// As [`Vfs::create`] and [`Vfs::flush`].
    pub fn write(&self, path: &str, bytes: &[u8]) -> Result<Committed, VfsError> {
        let id = self.create(path)?;
        let staged = self.write_at(id, 0, bytes);
        let committed = staged.and_then(|_| self.flush(id));
        self.release(id);
        committed
    }

    /// Begins a write to `path`, and hands back the token the rest of it is addressed by.
    ///
    /// **Everything that can be decided before the bytes arrive is decided here**, because a
    /// `create` is where a file manager's error message comes from: the layout's refusals, the
    /// names this directory can file, and the position a page may be inserted at. What is left
    /// for [`Vfs::flush`] is what needs the bytes — whether they are a document — and the
    /// document's own assertions over its reader, which are asked once at the transform seam.
    ///
    /// # Errors
    ///
    /// [`VfsError::Refused`] where the layout refuses the destination by design or declares a
    /// meaning nothing implements, [`VfsError::NoSuchPath`] for a path the layout does not name,
    /// [`VfsError::Unnameable`] for a name this directory cannot file, [`VfsError::AlreadyFiled`]
    /// for an embedded file whose name §7.7.4's tree already holds, and the backing's or the
    /// worker's own.
    pub fn create(&self, path: &str) -> Result<StagedId, VfsError> {
        let mut held = self.held();
        let current = self.current_in(&mut held)?;
        let canonical = canonical_path(path)?;
        let (route, captures) = Self::locate_for_write(&current, &canonical)?;
        let token = held.next_token;
        held.next_token = held.next_token.saturating_add(1);
        let foreign_edits = held.foreign_edits;
        held.staged.insert(
            token,
            Staged {
                path: canonical,
                route,
                captures,
                generation: current.generation,
                foreign_edits,
                bytes: Vec::new(),
                touched: false,
                committed: false,
            },
        );
        Ok(StagedId(token))
    }

    /// Writes into a staged file at an offset, as `pwrite(2)` does.
    ///
    /// A gap is filled with zero bytes, which is what a sparse write to a real file produces; a
    /// caller that seeks past the end and writes has made a file that long.
    ///
    /// # Errors
    ///
    /// [`VfsError::NoSuchWrite`] for a token nothing answers to, and [`VfsError::TooLarge`] past
    /// [`Config::max_staged_bytes`].
    pub fn write_at(&self, id: StagedId, offset: u64, bytes: &[u8]) -> Result<usize, VfsError> {
        let mut held = self.held();
        let ceiling = self.config.max_staged_bytes;
        let staged = held
            .staged
            .get_mut(&id.0)
            .ok_or(VfsError::NoSuchWrite(id.0))?;
        let from = usize::try_from(offset).map_err(|_| VfsError::TooLarge {
            path: staged.path.clone(),
            reached: usize::MAX,
            ceiling,
        })?;
        let end = from.saturating_add(bytes.len());
        if end > ceiling {
            return Err(VfsError::TooLarge {
                path: staged.path.clone(),
                reached: end,
                ceiling,
            });
        }
        staged.touched = true;
        if staged.bytes.len() < end {
            staged.bytes.resize(end, 0);
        }
        staged
            .bytes
            .get_mut(from..end)
            .ok_or(VfsError::NoSuchWrite(id.0))?
            .copy_from_slice(bytes);
        staged.committed = false;
        Ok(bytes.len())
    }

    /// Cuts a staged file to `length`, as `ftruncate(2)` does.
    ///
    /// # Errors
    ///
    /// [`VfsError::NoSuchWrite`], and [`VfsError::TooLarge`] for a length past the ceiling.
    pub fn truncate(&self, id: StagedId, length: u64) -> Result<(), VfsError> {
        let mut held = self.held();
        let ceiling = self.config.max_staged_bytes;
        let staged = held
            .staged
            .get_mut(&id.0)
            .ok_or(VfsError::NoSuchWrite(id.0))?;
        let to = usize::try_from(length)
            .ok()
            .filter(|to| *to <= ceiling)
            .ok_or_else(|| VfsError::TooLarge {
                path: staged.path.clone(),
                reached: usize::try_from(length).unwrap_or(usize::MAX),
                ceiling,
            })?;
        staged.bytes.resize(to, 0);
        staged.touched = true;
        staged.committed = false;
        Ok(())
    }

    /// Commits a staged write, which is the moment RFC 0003 section 5.4 names: a FUSE write
    /// buffers, and validation and commit happen on `flush`, whose error return reaches the
    /// application's `close()` — `release` reaches nobody, which is why it is only cleanup.
    ///
    /// A second `flush` with nothing written since the first does nothing and says so with the
    /// same answer, because the kernel issues one per `close(2)` and a file opened twice is
    /// closed twice.
    ///
    /// # Errors
    ///
    /// [`VfsError::NoSuchWrite`], [`VfsError::Changed`] where the document moved under the write,
    /// and the worker's own — including [`worker::WorkerError::Restricted`] and
    /// [`worker::WorkerError::Unanswerable`], which are `CLAUDE.md` principle 3's two levels that
    /// do not proceed.
    pub fn flush(&self, id: StagedId) -> Result<Committed, VfsError> {
        let mut held = self.held();
        let (path, mut route, mut captures, generation, foreign_edits, bytes, touched, committed) = {
            let staged = held.staged.get(&id.0).ok_or(VfsError::NoSuchWrite(id.0))?;
            (
                staged.path.clone(),
                staged.route,
                staged.captures.clone(),
                staged.generation,
                staged.foreign_edits,
                staged.bytes.clone(),
                staged.touched,
                staged.committed,
            )
        };
        let current = self.current_in(&mut held)?;
        // Nothing to do, twice over. `committed` is the second `close(2)` of a file the kernel
        // opened twice; `!touched` is a handle opened for writing that nobody wrote to or
        // truncated, which [`Staged::touched`] says is not a write at all. Both mark the write
        // done so that [`Vfs::release`] does not then announce an abandonment.
        if committed || !touched {
            if let Some(staged) = held.staged.get_mut(&id.0) {
                staged.committed = true;
            }
            return Ok(Committed {
                path,
                meaning: route.write.on_write,
                from: current.generation,
                to: current.generation,
                pages: current.pages,
                warnings: Vec::new(),
            });
        }
        if current.generation != generation {
            // **Whose edit moved it decides, and so does what the name means.**
            //
            // A *foreign* edit is `ESTALE` without exception: RFC 0003 section 5.4 makes the
            // backing file the single source of truth, and committing over somebody else's
            // update could discard it. [`Staged::foreign_edits`] is how that question is asked
            // exactly, across any number of transitions.
            //
            // Our own commit is a different thing, and it used to be refused as though it were
            // the same one — two writes in flight at once in a single mount, the first
            // committing, the second `ESTALE` (round 911). What decides there is whether the
            // *name* still means what it meant. An embedded file's name and the information
            // dictionary are identities, so they still do. A page's ordinal is a **position** —
            // RFC 0003 section 5.2: "[o]rdinal names are positions, not identities … after any
            // write, the next listing renumbers" — so an insertion staged before a commit that
            // renumbered would land somewhere nobody asked for, and it stays `ESTALE`.
            let rebasable = foreign_edits == held.foreign_edits
                && matches!(
                    route.write.on_write,
                    Write::EmbedFile | Write::SetInformation
                );
            if !rebasable {
                return Err(VfsError::Changed { path });
            }
            // Re-asked rather than reused, so that every check `create` made — the name §7.7.4's
            // tree already holds above all — is made again against the generation this is about
            // to be committed to.
            let (fresh_route, fresh_captures) = Self::locate_for_write(&current, &path)?;
            route = fresh_route;
            captures = fresh_captures;
            if let Some(staged) = held.staged.get_mut(&id.0) {
                staged.route = route;
                staged.captures = captures.clone();
                staged.generation = current.generation;
            }
        }
        let query = match route.write.on_write {
            Write::InsertPages => Query::InsertPages {
                at: captures
                    .page
                    .ok_or_else(|| VfsError::NoSuchPath(path.clone()))?,
                document: bytes,
            },
            Write::EmbedFile => Query::Attach {
                name: captures
                    .name
                    .clone()
                    .ok_or_else(|| VfsError::NoSuchPath(path.clone()))?,
                bytes,
            },
            Write::SetInformation => Query::SetInformation { json: bytes },
            // `create` refused every other mapping before a byte was accepted, so this is a row
            // that changed under a staged write rather than a caller's mistake.
            Write::DeletePage | Write::RemoveAttachment | Write::Refused(_) => {
                return Err(refusal_for(&path, Verb::Write)?.into());
            }
        };
        let outcome = self.apply_update(&mut held, &current, &path, &query)?;
        if let Some(staged) = held.staged.get_mut(&id.0) {
            staged.committed = true;
        }
        Ok(Committed {
            meaning: route.write.on_write,
            ..outcome
        })
    }

    /// Drops a staged write.
    ///
    /// `Some` where it was never committed, which is a write that did not happen: the document
    /// is byte for byte what it was, and [`Abandoned::sentence`] is what a face logs so that the
    /// name its listing showed is explained rather than discovered.
    pub fn release(&self, id: StagedId) -> Option<Abandoned> {
        let mut held = self.held();
        let staged = held.staged.remove(&id.0)?;
        (!staged.committed).then(|| Abandoned {
            path: staged.path,
            size: u64::try_from(staged.bytes.len()).unwrap_or(u64::MAX),
            meaning: staged.route.write.on_write,
        })
    }

    /// A handle onto a write in flight at this path, where there is one.
    ///
    /// The generation is the one the write was created against, which is what makes the property
    /// RFC 0003 section 5.4 states hold for a staged file too: what a reader is handed is one
    /// generation's worth of bytes and never a splice.
    fn staged_at(&self, path: &str) -> Option<Handle> {
        let canonical = canonical_path(path).ok()?;
        let held = self.held();
        let staged = held
            .staged
            .values()
            .find(|staged| !staged.committed && staged.path == canonical)?;
        Some(Handle {
            path: canonical,
            generation: staged.generation,
            bytes: Arc::from(staged.bytes.as_slice()),
        })
    }

    /// The writes in flight, as a listing shows them.
    #[must_use]
    pub fn pending(&self) -> Vec<Pending> {
        self.held()
            .staged
            .iter()
            .filter(|(_, staged)| !staged.committed)
            .map(|(token, staged)| Pending {
                id: StagedId(*token),
                path: staged.path.clone(),
                size: u64::try_from(staged.bytes.len()).unwrap_or(u64::MAX),
                meaning: staged.route.write.on_write,
            })
            .collect()
    }

    /// Deleting a path.
    ///
    /// **Not staged, because a deletion has no bytes**: `unlink(2)` is one call and this is one
    /// transaction, so it validates and commits together and its error is the caller's own.
    ///
    /// # Errors
    ///
    /// [`VfsError::Refused`] where the layout refuses the deletion, [`VfsError::NoSuchPath`],
    /// and the worker's own.
    pub fn remove(&self, path: &str) -> Result<Committed, VfsError> {
        let mut held = self.held();
        let current = self.current_in(&mut held)?;
        let canonical = canonical_path(path)?;
        let (route, captures) = locate_in(&current, &canonical, &self.config.resolutions)?;
        let query = match route.write.on_delete {
            Write::DeletePage => Query::DeletePage {
                page: captures
                    .page
                    .ok_or_else(|| VfsError::NoSuchPath(canonical.clone()))?,
            },
            // The *document's* own name for it, which is what the worker files it under; the
            // tree lists it under a name a directory can hold, and `attachments` is the map
            // between the two.
            Write::RemoveAttachment => {
                let listed = captures
                    .name
                    .clone()
                    .ok_or_else(|| VfsError::NoSuchPath(canonical.clone()))?;
                Query::Detach {
                    name: attachments(&current)?
                        .iter()
                        .find(|embedded| embedded.file_name == listed)
                        .map(|embedded| embedded.document_name.clone())
                        .ok_or_else(|| VfsError::NoSuchPath(canonical.clone()))?,
                }
            }
            Write::InsertPages | Write::EmbedFile | Write::SetInformation | Write::Refused(_) => {
                return Err(refusal_for(&canonical, Verb::Delete)?.into());
            }
        };
        let outcome = self.apply_update(&mut held, &current, &canonical, &query)?;
        Ok(Committed {
            meaning: route.write.on_delete,
            ..outcome
        })
    }

    /// The whole commit: ask the worker, check §7.5.6's property, replace the file, and step the
    /// generation on — all under the lock the caller already holds.
    fn apply_update(
        &self,
        held: &mut Serving,
        current: &Current,
        path: &str,
        query: &Query,
    ) -> Result<Committed, VfsError> {
        let Answer::Written { bytes, warnings } = current.worker.ask(query)? else {
            return Err(VfsError::Worker(mismatch(
                &Answer::Absent,
                "an updated document",
            )));
        };
        self.check_appended(&bytes)?;
        self.backing
            .commit(&bytes)
            .map_err(|error| VfsError::Backing {
                document: self.backing.describe(),
                error,
            })?;
        let from = current.generation;
        // The old generation goes now and the new one is built before the lock is let go, so no
        // operation can see a tree that belongs to neither — and `ours` is what makes the next
        // one `Provenance::Ours` rather than looking like somebody else's edit.
        held.current = None;
        held.ours = self.backing.generation().ok();
        let next = self.current_in(held)?;
        Ok(Committed {
            path: path.to_owned(),
            meaning: Write::Refused(Reason::LayoutIsNotWritable),
            from,
            to: next.generation,
            pages: next.pages,
            warnings,
        })
    }

    /// ISO 32000-2 §7.5.6, checked against the file rather than trusted:
    ///
    /// > When updating a PDF file incrementally, changes shall be appended to the end of the
    /// > file, leaving its original contents intact.
    ///
    /// So the document the worker computed begins with the document on disk, byte for byte, and
    /// the broker can say so **without reading either as a PDF** — which is what keeps this on
    /// its side of RFC 0003 section 6's line. A worker that answered with anything else has
    /// either been compromised or has a defect, and either way its answer is not written.
    fn check_appended(&self, bytes: &[u8]) -> Result<(), VfsError> {
        /// How much is compared at a time, so that a large document is not held twice.
        const WINDOW: usize = 1 << 16;

        let original = self.backing.bytes().map_err(|error| VfsError::Backing {
            document: self.backing.describe(),
            error,
        })?;
        let length = original.len();
        let refuse = |detail: &str| {
            VfsError::Worker(WorkerError::Refused(format!(
                "the update this worker computed does not have the document as its prefix ({detail}), \
             and §7.5.6 makes an update \"appended to the end of the file, leaving its original \
             contents intact\"; nothing was written"
            )))
        };
        if bytes.len() < length {
            return Err(refuse("it is shorter than the file"));
        }
        let mut at = 0;
        while at < length {
            let end = at.saturating_add(WINDOW).min(length);
            let window = original.read(at..end);
            if bytes.get(at..end) != Some(window.as_ref()) {
                return Err(refuse("they differ inside the file's own bytes"));
            }
            at = end;
        }
        Ok(())
    }

    /// The row a path names, for a **write** — which admits two paths a read does not.
    ///
    /// A `pages/NNNN.pdf` one past the last page, because RFC 0003 section 5.2 makes an ordinal a
    /// position and appending is inserting at the position after the end; and an
    /// `attachments/NAME` the document does not file, because that is what embedding a file *is*.
    /// Everything else is the read's own answer.
    fn locate_for_write(
        current: &Current,
        path: &str,
    ) -> Result<(&'static Route, path::Captures), VfsError> {
        let missing = || VfsError::NoSuchPath(path.to_owned());
        let (route, captures) = path::resolve(path).ok_or_else(missing)?;
        match route.write.on_write {
            Write::InsertPages => {
                let page = captures.page.ok_or_else(missing)?;
                // One past the end appends; nothing further out is a position, and a listing
                // never showed it.
                if page == 0 || page > current.pages.saturating_add(1) {
                    return Err(missing());
                }
                if !spells(current, path, page) {
                    return Err(missing());
                }
            }
            Write::EmbedFile => {
                let name = captures.name.clone().ok_or_else(missing)?;
                // The tree lists an embedded file under a name a directory can hold, and a read
                // is looked up by that name; a file filed under a name the listing would show
                // differently is a file `ls` could not name. So the two have to agree, and a
                // name that would be changed is refused rather than quietly renamed.
                if safe_name(&name) != name {
                    return Err(VfsError::Unnameable {
                        path: path.to_owned(),
                        detail: format!(
                            "§7.7.4's tree would file this as {:?}, which is not the name asked \
                             for; a directory cannot hold every byte a name tree can",
                            safe_name(&name)
                        ),
                    });
                }
                if attachments(current)?
                    .iter()
                    .any(|embedded| embedded.file_name == name)
                {
                    return Err(VfsError::AlreadyFiled {
                        path: path.to_owned(),
                    });
                }
            }
            Write::SetInformation => {}
            Write::DeletePage | Write::RemoveAttachment | Write::Refused(_) => {
                return Err(refusal_for(path, Verb::Write)?.into());
            }
        }
        Ok((route, captures))
    }

    /// Creating a directory.
    ///
    /// # Errors
    ///
    /// As [`Vfs::write`]; every directory in this tree is the document's own shape.
    pub fn create_directory(&self, path: &str) -> Result<(), VfsError> {
        let _ = canonical_path(path)?;
        Err(Refused::ByDesign {
            path: path.to_owned(),
            reason: Reason::LayoutIsNotWritable,
        }
        .into())
    }

    /// Renaming within the tree.
    ///
    /// **Refused outright and not merely unbuilt**, which is RFC 0003 section 5.3's first
    /// refusal: "[r]ename semantics under position-names are ambiguous (is `mv 0007 0002` 'before
    /// old 0002' or 'become new 0002'?), and a file manager's drag-reorder emits rename storms
    /// this tree cannot make atomic."
    ///
    /// # Errors
    ///
    /// Always [`Refused::ByDesign`] with [`Reason::ReorderIsAmbiguous`].
    pub fn rename(&self, from: &str, to: &str) -> Result<(), VfsError> {
        Err(Refused::ByDesign {
            path: format!("{from} -> {to}"),
            reason: Reason::ReorderIsAmbiguous,
        }
        .into())
    }

    /// One virtual file's bytes.
    fn generate(
        &self,
        current: &Current,
        route: &'static Route,
        captures: &path::Captures,
        canonical: &str,
    ) -> Result<Vec<u8>, VfsError> {
        let missing = || VfsError::NoSuchPath(canonical.to_owned());
        let page = || captures.page.ok_or_else(missing);
        let answer = match route.generator {
            Generator::ExtractedPage => {
                current.worker.ask(&Query::ExtractPage { page: page()? })?
            }
            Generator::RenderedPage => current.worker.ask(&Query::RenderPage {
                page: page()?,
                dpi: captures.dpi.ok_or_else(missing)?,
            })?,
            Generator::ExtractedImage => {
                // One extraction, every output kept. A page's images come out of one
                // `pdf_transform::images` run, so reading them one at a time would re-run it once
                // per file — and a `cp -r` of an `images/NNNN/` directory reads them all. The
                // whole run goes into the cache under each output's own path, and the caller's is
                // returned from there; the cache's byte budget decides how much of it survives,
                // which is the one place a decision about memory belongs.
                let page = page()?;
                let name = captures.name.clone().ok_or_else(missing)?;
                let width = width(current);
                let stem = path::page_name_stem(page, width);
                let mut wanted = None;
                for (output, bytes) in images(current, page)? {
                    let at = format!("/images/{stem}/{output}");
                    let kept = self.cache.put(current.generation, &at, bytes);
                    if output == name {
                        wanted = Some(kept.to_vec());
                    }
                }
                return wanted.ok_or_else(missing);
            }
            Generator::PageText => current.worker.ask(&Query::PageText { page: page()? })?,
            Generator::DocumentText => return self.document_text(current),
            Generator::ExtractedAttachment => {
                let name = captures.name.clone().ok_or_else(missing)?;
                let document_name = attachments(current)?
                    .iter()
                    .find(|embedded| embedded.file_name == name)
                    .map(|embedded| embedded.document_name.clone())
                    .ok_or_else(missing)?;
                current.worker.ask(&Query::ExtractAttachment {
                    name: document_name,
                })?
            }
            Generator::Information => current.worker.ask(&Query::Information)?,
            Generator::MetadataStream => current.worker.ask(&Query::MetadataStream)?,
            Generator::Outline => current.worker.ask(&Query::Outline)?,
            Generator::Root
            | Generator::PageOrdinals
            | Generator::Resolutions
            | Generator::RenderOrdinals
            | Generator::ImagePageOrdinals
            | Generator::ImageInventory
            | Generator::TextOrdinals
            | Generator::AttachmentInventory
            | Generator::MetaNames => {
                return Err(VfsError::IsADirectory(canonical.to_owned()));
            }
        };
        match answer {
            Answer::Bytes(bytes) => Ok(bytes),
            Answer::Absent => Err(missing()),
            other => Err(VfsError::Worker(mismatch(&other, "bytes"))),
        }
    }

    /// `text/document.txt`: every page's readback in page order.
    ///
    /// Separated by U+000C FORM FEED, which is a stated choice rather than a clause: the standard
    /// says nothing about how a concatenation of pages' text is delimited, and the form feed is
    /// what every text extractor a script would otherwise be piped through emits between pages.
    /// `CLAUDE.md`'s rule for a silence applies — a documented choice, said to be a choice.
    ///
    /// Each page goes through the per-page cache on the way, so a caller that greps
    /// `document.txt` and then reads a page pays for that page once.
    fn document_text(&self, current: &Current) -> Result<Vec<u8>, VfsError> {
        let width = width(current);
        let mut out: Vec<u8> = Vec::new();
        for page in 1..=current.pages {
            let path = format!("/text/{}", path::page_name(page, width, "txt"));
            let bytes = if let Some(bytes) = self.cache.get(current.generation, &path) {
                bytes
            } else {
                let Answer::Bytes(bytes) = current.worker.ask(&Query::PageText { page })? else {
                    return Err(VfsError::Worker(mismatch(&Answer::Absent, "bytes")));
                };
                self.cache.put(current.generation, &path, bytes)
            };
            if page > 1 {
                out.push(0x0c);
            }
            out.extend_from_slice(&bytes);
        }
        Ok(out)
    }
}

/// Which of the two write verbs a refusal is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    /// Reading a file out of the tree, or listing a directory that generates to list.
    ///
    /// Not one of RFC 0003 section 5.2's write verbs and never a refusal a write mapping words —
    /// it is here because [`Vfs::consult`] answers about a read too: taking a page out of the
    /// mount is Table 22 bit 11's assembly, a render is bit 3's printing and an image is bit 5's
    /// extraction, and a face is owed the question before it starts the copy.
    Read,
    /// Creating or overwriting a path.
    Write,
    /// Deleting one.
    Delete,
}

/// The refusal `verb` on `path` earns.
fn refusal_for(path: &str, verb: Verb) -> Result<Refused, VfsError> {
    let (route, _) = path::resolve(path).ok_or(VfsError::NoSuchPath(path.to_owned()))?;
    let meaning = match verb {
        Verb::Write => route.write.on_write,
        Verb::Delete => route.write.on_delete,
        // This function words what a *change* earns. A read is refused by `Vfs::open`'s own
        // errors — a directory, a path the layout does not name — and never by a write mapping,
        // so a caller here has asked the wrong question and is told so rather than answered
        // with a sentence about writing (trap 5).
        Verb::Read => {
            return Err(VfsError::Worker(WorkerError::Mismatched {
                got: "a read",
                wanted: "a verb that changes the document",
            }));
        }
    };
    Ok(match meaning {
        Write::Refused(reason) => Refused::ByDesign {
            path: path.to_owned(),
            reason,
        },
        mapping => Refused::WrongVerb {
            path: path.to_owned(),
            mapping,
        },
    })
}

/// The canonical spelling of a path the layout names.
fn canonical_path(path: &str) -> Result<String, VfsError> {
    let parts = path::components(path).ok_or(VfsError::NoSuchPath(path.to_owned()))?;
    Ok(path::canonical(&parts))
}

/// The row a path names, checked against what this document actually has.
///
/// [`path::resolve`] answers structurally; this is where a page number past the end, an
/// unoffered resolution and an attachment the document does not file become
/// [`VfsError::NoSuchPath`]. Two answers rather than one on purpose: a face passes arbitrary
/// paths, and "not in the layout" and "not in this document" are the same error to it and
/// different errors to a reader of this code.
fn locate_in(
    current: &Current,
    path: &str,
    resolutions: &[u32],
) -> Result<(&'static Route, path::Captures), VfsError> {
    let missing = || VfsError::NoSuchPath(path.to_owned());
    let (route, captures) = path::resolve(path).ok_or_else(missing)?;
    if let Some(page) = captures.page
        && (page > current.pages || page == 0)
    {
        return Err(missing());
    }
    if let Some(dpi) = captures.dpi
        && !resolutions.contains(&dpi)
    {
        return Err(missing());
    }
    match route.generator {
        // A page ordinal has exactly one spelling, and `path::resolve` already held it to the
        // minimum width; this is where the *document's* width decides, so a 12 000-page
        // document's pages are `00001.pdf` and nothing answers to `0001.pdf`.
        Generator::ExtractedPage | Generator::RenderedPage | Generator::PageText => {
            let page = captures.page.ok_or_else(missing)?;
            if !spells(current, path, page) {
                return Err(missing());
            }
        }
        Generator::ImageInventory | Generator::ImagePageOrdinals => {
            if let Some(page) = captures.page
                && !spells(current, path, page)
            {
                return Err(missing());
            }
        }
        Generator::ExtractedImage => {
            let page = captures.page.ok_or_else(missing)?;
            let name = captures.name.clone().ok_or_else(missing)?;
            if !spells(current, path, page) || !images(current, page)?.contains_key(&name) {
                return Err(missing());
            }
        }
        Generator::ExtractedAttachment => {
            let name = captures.name.clone().ok_or_else(missing)?;
            if !attachments(current)?
                .iter()
                .any(|embedded| embedded.file_name == name)
            {
                return Err(missing());
            }
        }
        _ => {}
    }
    // §14.3.2's stream is the one file in this tree whose *existence* the document states, so it
    // is the one row whose validation has to ask the worker a question. Outside the match rather
    // than an arm of it, because a `?` cannot live in a match guard and an arm that only asks a
    // question reads as though it were doing something else.
    if route.generator == Generator::MetadataStream
        && !matches!(
            current.worker.ask(&Query::MetadataStream)?,
            Answer::Bytes(_)
        )
    {
        return Err(missing());
    }
    Ok((route, captures))
}

/// Whether the path spells its page ordinal at this document's own width.
///
/// [`path::resolve`] has already held the ordinal to one spelling at the *minimum* width, so
/// what is left is the document's own: a twelve-thousand-page document's pages are `00001.pdf`
/// and nothing answers to `0001.pdf`. It is asked of every component rather than of the one the
/// route puts the ordinal in, which is loose in principle and exact for this table — the only
/// component that can equal a page's stem is the one the ordinal came from, and every row whose
/// path holds a second variable component (an image's name, an attachment's) checks that
/// component against the document's own inventory in the same breath.
fn spells(current: &Current, path: &str, page: usize) -> bool {
    let width = width(current);
    let expected = path::page_name_stem(page, width);
    path::components(path).is_some_and(|parts| {
        parts.iter().any(|part| {
            part == &expected.as_str()
                || part
                    .split_once('.')
                    .is_some_and(|(stem, _)| stem == expected.as_str())
        })
    })
}

/// How many digits a page ordinal takes in this document.
///
/// RFC 0003 section 4's "zero-padded ordinal; width from page count", with four as the floor
/// so that a five-page document reads like the RFC's own example.
fn width(current: &Current) -> usize {
    current.pages.to_string().len().max(path::MIN_ORDINAL_WIDTH)
}

/// One name per page, at the document's width, with this extension.
fn page_names(current: &Current, extension: &str) -> Vec<DirEntry> {
    let width = width(current);
    (1..=current.pages)
        .map(|page| DirEntry {
            name: path::page_name(page, width, extension),
            kind: Kind::File,
        })
        .collect()
}

/// §7.11.4's embedded files, read once per generation, under the names the tree lists.
///
/// ISO 32000-2 §7.11.4.1:
///
/// > This makes the PDF file a self-contained unit that can be stored or transmitted as a
/// > single entity.
///
/// Which is why the mount can show them at all, and why the names are the document's rather
/// than this program's: the file specification says what the file is called. What this does
/// to them is the minimum a directory requires — `pdf_transform::pattern::sanitise`'s
/// replacement of the bytes a file name may not hold, then `.` and `..` and the empty name,
/// then a deterministic suffix where two of the document's names collide after that. Every
/// step is recorded in the map, so a read is looked up by the *document's* name and never by
/// the sanitised one.
fn attachments(current: &Current) -> Result<Arc<Vec<Embedded>>, VfsError> {
    let mut held = current
        .attachments
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(known) = held.as_ref() {
        return Ok(Arc::clone(known));
    }
    let Answer::Attachments(entries) = current.worker.ask(&Query::AttachmentInventory)? else {
        return Err(VfsError::Worker(mismatch(&Answer::Absent, "attachments")));
    };
    let mut taken: Vec<String> = Vec::new();
    let mut embedded = Vec::with_capacity(entries.len());
    for entry in entries {
        let safe = safe_name(&entry.name);
        let mut file_name = safe.clone();
        let mut suffix = 1_u32;
        while taken.contains(&file_name) {
            suffix = suffix.saturating_add(1);
            file_name = format!("{safe}~{suffix}");
        }
        taken.push(file_name.clone());
        embedded.push(Embedded {
            file_name,
            document_name: entry.name,
        });
    }
    let shared = Arc::new(embedded);
    *held = Some(Arc::clone(&shared));
    Ok(shared)
}

/// One page's images, by the name each output took.
///
/// Not held per generation the way the attachments are: an image is bytes rather than a name, so
/// what would be kept is the content, and [`cache`] is what keeps content — under the same
/// generation key and the same byte budget. What that costs is stated rather than hidden: a
/// *listing* of `images/NNNN/` re-runs the extraction every time, because a listing is exactly
/// this call's keys, and only a **read** puts the bytes in the cache. `doc/todo/58` §5 carries it,
/// beside the measurement nobody has taken of it.
fn images(
    current: &Current,
    page: usize,
) -> Result<std::collections::BTreeMap<String, Vec<u8>>, VfsError> {
    match current.worker.ask(&Query::ExtractImages { page })? {
        Answer::Files(files) => Ok(files),
        other => Err(VfsError::Worker(mismatch(&other, "files"))),
    }
}

/// A name a directory can hold, from a name a document wrote.
///
/// Three steps, and the order is the point: `pdf_transform::pattern::sanitise` first, because it
/// is the tree's one statement of which bytes a file name may not hold; then the two names that
/// are not names, `.` and `..`, and the empty one, each replaced rather than rejected — a
/// document may file an attachment under any string, and refusing to show it would be a silent
/// loss.
fn safe_name(name: &str) -> String {
    let clean = pdf_transform::pattern::sanitise(name);
    match clean.as_str() {
        "" | "." => String::from("_"),
        ".." => String::from("__"),
        _ => clean,
    }
}

/// The last component of a layout pattern, which is the name it takes in its parent's listing.
fn last_component(pattern: &str) -> String {
    pattern.rsplit('/').next().unwrap_or(pattern).to_owned()
}

/// The error for a worker that answered the wrong shape.
fn mismatch(got: &Answer, wanted: &'static str) -> WorkerError {
    WorkerError::Mismatched {
        got: match got {
            Answer::Count(_) => "count",
            Answer::Bytes(_) => "bytes",
            Answer::Files(_) => "files",
            Answer::Attachments(_) => "attachments",
            Answer::Absent => "nothing",
            Answer::Written { .. } => "an updated document",
            Answer::Consulted(_) => "a consultation",
        },
        wanted,
    }
}
