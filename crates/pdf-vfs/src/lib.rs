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
//! side of the worker protocol here, and says why: "\[t]he faces contain *no* layout knowledge —
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
//! # Where [`Vfs`]'s methods live
//!
//! This file holds the type, its constructors and the structs one generation is made of; its
//! methods are continued by concern: `mount` is the generation being served and the counters,
//! `directory` is [`Vfs::list`] and [`Vfs::stat`], `reading` is [`Vfs::consult`], [`Vfs::open`] and
//! the generators, `writing` is every verb that lands as an update, and `inventory` is the free
//! functions that answer what the document has. `error` holds [`VfsError`] and [`Refused`]; `consent`
//! is the seam every question to a worker crosses (ADR 2368).
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
//!
//! **And it is the face's job to ask for ISO 32000-2 §7.6.4.1's password**, because a face is the
//! only part of this arrangement with somebody to ask. [`Vfs::with_password`] takes one and holds
//! it for the mount's life; a mount built without one opens whatever the clause's default user
//! password opens and refuses the rest by name.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod cache;
pub mod commit;
mod confined;
mod consent;
mod directory;
mod error;
pub mod generation;
mod inventory;
pub mod layout;
mod mount;
pub mod path;
mod reading;
mod serve;
mod wire;
pub mod worker;
mod writing;

use std::sync::{Arc, Mutex};

use pdf_transform::{Budget, Policy};

use crate::cache::Cache;
use crate::commit::Staged;
use crate::consent::{Consenting, Totals};
use crate::generation::{Backing, Generation};
use crate::layout::{Kind, Route, Write, WriteMapping};
use crate::worker::{Answer, WorkerError, Workers};

pub use crate::commit::{Abandoned, Committed, Errno, Pending, Provenance, StagedId};
pub use crate::confined::{Confined, ConfinedWorkers};
pub use crate::consent::Questions;
pub use crate::error::{Refused, VfsError};
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

/// ISO 32000-2 §7.6.4.1's user or owner password, for a face that has one to hand over.
///
/// Re-exported for the same reason as [`Consulted`]: a face should not have to depend on
/// `pdf-transform` to name a type this crate takes in [`Vfs::with_password`].
pub use pdf_transform::Secret;

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
    /// ISO 32000-2 §7.6.4.1's password, where the face that built this mount was given one.
    ///
    /// **Held for the mount's whole life, and lent to every generation's worker.** The clause's
    /// own sentence is what makes that the right lifetime: "Correctly supplying either password (
    /// owner or user password) should enable the user to gain access to the document" — access to
    /// the document, not to one reading of it — while RFC 0003 section 5.4 makes every change to
    /// the file a *new* worker. A password held per worker would have to be supplied again every
    /// time somebody else saved the file, and a mount has nobody to ask.
    ///
    /// Deliberately not in [`Config`]: that struct is `Clone` and has a [`Default`], and
    /// `viewer_core::Secret` is neither.
    password: Option<Secret>,
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
    /// which is exactly what made a test of [`Cache::size_of`] pass without it (ADR 0865, trap
    /// 13). An `AtomicU64` because every operation here is behind `&self`.
    generated: std::sync::atomic::AtomicU64,
    /// What this mount has asked its workers, over every generation it has served.
    ///
    /// [`Totals`] says why the counts are the mount's and the repeat detection is a generation's,
    /// and [`Questions`] says what a floor built on them holds. Shared with each generation's
    /// [`Consenting`], which is the one seam every question in this crate crosses.
    questions: Arc<Totals>,
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
    /// Whether §14.3.2's metadata stream exists, once anything has asked.
    ///
    /// `None` until something asks. It is the *existence* rather than the bytes because the
    /// bytes are the cache's — `metadata_stream` is the one place that reads either — and it is
    /// remembered at all because `meta/xmp.xml` is the one row in the layout whose validation has
    /// to ask the worker a question: without this, every listing of `/meta` and every `stat` and
    /// `open` under it would fetch the whole stream again to decide whether the name exists. That
    /// is trap 33's shape, which this crate's own question counter finds (ADR 0894).
    metadata: Mutex<Option<bool>>,
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
        Self::with(backing, workers, config, None)
    }

    /// The same tree, over a document that wants ISO 32000-2 §7.6.4.1's password.
    ///
    /// **§7.6.4.1 is answered in two halves and this is the second.** The first is the clause's
    /// own default: "the PDF reader shall first try to authenticate the encrypted document using
    /// the padding string defined in 7.6.4.3 … (default user password)", which [`Vfs::new`] gets
    /// for nothing because the reader below tries it on every encrypted document. The second is
    /// what happens when that fails — "the interactive PDF processor should prompt for a password.
    /// Correctly supplying either password ( owner or user password) should enable the user to
    /// gain access to the document" — and a mount is the half of that sentence a *prompt* cannot
    /// serve: the person who could answer is not present at the `readdir` that needs the answer,
    /// and there is no channel to reach them through. So the password is supplied when the mount
    /// is built, by whichever face was in a position to ask for it, and is then the mount's for
    /// its life. Either password works, because the clause names both and authentication is
    /// `pdf-syntax`'s.
    ///
    /// A face that has no password calls [`Vfs::new`] and an encrypted document is refused by
    /// name — `WorkerError::PasswordRequired` with the sentence that says so, which is trap 5's
    /// loudness rather than an `EACCES` a caller has to guess at.
    #[must_use]
    pub fn with_password(
        backing: Box<dyn Backing>,
        workers: Box<dyn Workers>,
        config: Config,
        password: Secret,
    ) -> Self {
        Self::with(backing, workers, config, Some(password))
    }

    /// What both constructors are.
    fn with(
        backing: Box<dyn Backing>,
        workers: Box<dyn Workers>,
        config: Config,
        password: Option<Secret>,
    ) -> Self {
        Self {
            cache: Cache::new(config.cache_bytes),
            backing,
            workers,
            config,
            password,
            state: Mutex::new(Serving::default()),
            generated: std::sync::atomic::AtomicU64::new(0),
            questions: Arc::new(Totals::default()),
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
            detail: "§7.6.4.1's password is supplied when the mount is built and cannot be \
                     supplied later: a document that is encrypted after the mount opened, or \
                     replaced under it by one wanting a different password, is refused rather \
                     than asked about, because a readdir has nobody to prompt",
        });
        out
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
