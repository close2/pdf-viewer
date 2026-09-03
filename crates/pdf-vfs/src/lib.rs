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
//! # The four pieces, and where each lives
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
//!
//! # Reads this round, writes declared and refused
//!
//! RFC 0003 section 10 orders reads first, and that is what landed: `stat`, `list`, `open` and
//! `read` over the whole tree. Every one of section 5.2's five write verbs is **declared in the
//! layout table** and refused at the point of the call, by the operation's own name; every one of
//! section 5.3's four refusals is refused with the sentence that says why it is not merely
//! unbuilt. [`Vfs::shortfalls`] is the list of both, so a face can print what it cannot do
//! rather than a person discovering it (trap 5).
//!
//! # What a face has to do, and what it must not
//!
//! Call [`Vfs::list`], [`Vfs::stat`], [`Vfs::open`] and [`Handle::read`]; map [`VfsError`] onto
//! its own errors; log [`Refused::sentence`] where its protocol has no channel for one — which
//! is FUSE's poverty and RFC 0003 section 5.3's reason for insisting the sentence exist here.
//! What it must not do is decide anything about the tree: a face that knows that `pages/` holds
//! PDFs has taken layout knowledge out of this table, and the next directory added would have to
//! be added twice.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod cache;
pub mod generation;
pub mod layout;
pub mod path;
pub mod worker;

use std::sync::{Arc, Mutex};

use pdf_transform::{Budget, Policy};

use crate::cache::Cache;
use crate::generation::{Backing, Generation};
use crate::layout::{Generator, Kind, Reason, Route, Write, WriteMapping};
use crate::worker::{Answer, Query, Worker, WorkerError, Workers};

pub use crate::generation::{FileBacking, MemoryBacking};
pub use crate::worker::InProcessWorkers;

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
    /// The most entries one directory may list.
    ///
    /// A listing is built from what the document says, so its length is the document's to choose
    /// and therefore needs a ceiling: a name tree with a million keys is a listing no file
    /// manager survives. Past this the listing is refused by name rather than truncated, because
    /// a truncated directory is a wrong answer that looks like a right one.
    pub max_entries: usize,
}

impl Default for Config {
    /// 64 MiB of cache, the transform seam's own budget, restrictions off, both resolutions, and
    /// 65 536 entries a directory.
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

/// A write this program will not do, and the two different things that can mean.
///
/// The distinction is the point, and RFC 0003 section 5.3 draws it: a *refusal by design* is a
/// file verb whose meaning would have to be invented, and it will still be refused when the write
/// side lands; a *declared and unbuilt* mapping is one this layout has decided the meaning of and
/// has not implemented. A face that could not tell them apart would report "read-only file
/// system" for both, and the design would be invisible from outside.
#[derive(Debug, thiserror::Error)]
pub enum Refused {
    /// Refused by design, for one of [`Reason`]'s five.
    #[error("{path}: {}", reason.sentence())]
    ByDesign {
        /// Which path.
        path: String,
        /// Why.
        reason: Reason,
    },
    /// The layout declares what this write means and RFC 0003's write side has not landed.
    #[error("{path}: {} is what a write here means, and the write side of this mount is not \
             built yet — use the pdf-transform command line", mapping.name())]
    NotYetImplemented {
        /// Which path.
        path: String,
        /// What the layout says it would mean.
        mapping: Write,
    },
}

impl Refused {
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
    /// The generation being served, and everything derived from it.
    current: Mutex<Option<Arc<Current>>>,
}

/// One generation of the document, and what has been read of it so far.
#[derive(Debug)]
struct Current {
    /// The key this was built for.
    generation: Generation,
    /// The confined side, over this generation's bytes.
    worker: Box<dyn Worker>,
    /// How many pages ISO 32000-2 §7.7.3.2's tree holds — the one thing read eagerly, because
    /// RFC 0003 section 5.1 says listing the root "reads nothing but the page count".
    pages: usize,
    /// §7.11.4's embedded files, once something has asked for them.
    attachments: Mutex<Option<Arc<Vec<Embedded>>>>,
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
            current: Mutex::new(None),
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
        let mut out: Vec<Shortfall> = layout::LAYOUT
            .iter()
            .filter(|route| route.write.declares_an_operation())
            .map(|route| Shortfall {
                pattern: route.pattern,
                detail: "the layout states what a write here means and the write side of this \
                         mount is not built: RFC 0003 section 5.2's verbs are the next round's",
            })
            .collect();
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
        let generation = self
            .backing
            .generation()
            .map_err(|error| VfsError::Backing {
                document: self.backing.describe(),
                error,
            })?;
        let mut held = self
            .current
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(current) = held.as_ref()
            && current.generation == generation
        {
            return Ok(Arc::clone(current));
        }
        let bytes = self.backing.bytes().map_err(|error| VfsError::Backing {
            document: self.backing.describe(),
            error,
        })?;
        let worker = self
            .workers
            .spawn(bytes, None, self.config.policy, self.config.budget)?;
        let pages = match worker.ask(&Query::PageCount)? {
            Answer::Count(pages) => pages,
            other => return Err(VfsError::Worker(mismatch(&other, "count"))),
        };
        // Before anything of the new generation is answered, everything of the old one is
        // forgotten. In this order, so that no window exists in which a caller could be handed
        // an entry keyed by a generation the document no longer has.
        self.cache.retain(generation);
        let current = Arc::new(Current {
            generation,
            worker,
            pages,
            attachments: Mutex::new(None),
        });
        *held = Some(Arc::clone(&current));
        Ok(current)
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
        if route.kind != Kind::Directory {
            return Err(VfsError::NotADirectory(path.to_owned()));
        }
        let entries = match route.generator {
            Generator::Root => layout::children("/")
                .into_iter()
                .map(|child| DirEntry {
                    name: last_component(child.pattern),
                    kind: child.kind,
                })
                .collect(),
            Generator::PageOrdinals => page_names(&current, "pdf"),
            Generator::Resolutions => self
                .config
                .resolutions
                .iter()
                .map(|dpi| DirEntry {
                    name: path::resolution_name(*dpi),
                    kind: Kind::Directory,
                })
                .collect(),
            Generator::RenderOrdinals => page_names(&current, "png"),
            Generator::ImagePageOrdinals => (1..=current.pages)
                .map(|page| DirEntry {
                    name: path::page_name_stem(page, width(&current)),
                    kind: Kind::Directory,
                })
                .collect(),
            Generator::ImageInventory => {
                let page = captures.page.ok_or(VfsError::NoSuchPath(path.to_owned()))?;
                let mut names: Vec<String> = images(&current, page)?.keys().cloned().collect();
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
                let mut entries = page_names(&current, "txt");
                entries.push(DirEntry {
                    name: String::from("document.txt"),
                    kind: Kind::File,
                });
                entries
            }
            Generator::AttachmentInventory => attachments(&current)?
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
        if entries.len() > self.config.max_entries {
            return Err(VfsError::TooManyEntries {
                path: path.to_owned(),
                count: entries.len(),
                ceiling: self.config.max_entries,
            });
        }
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
        let (route, _) = locate_in(&current, path, &self.config.resolutions)?;
        if route.kind == Kind::Directory {
            return Ok(Attributes {
                kind: Kind::Directory,
                size: None,
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

    /// Materialises a virtual file and hands back a handle onto its bytes.
    ///
    /// # Errors
    ///
    /// [`VfsError::NoSuchPath`], [`VfsError::IsADirectory`], and the worker's own refusal where
    /// the content cannot be produced — a codec the confined worker does not have, a page the
    /// rasteriser declined. Loud in every case (trap 5).
    pub fn open(&self, path: &str) -> Result<Handle, VfsError> {
        let current = self.current()?;
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
        let bytes = self.generate(&current, route, &captures, &canonical)?;
        Ok(Handle {
            path: canonical.clone(),
            generation: current.generation,
            bytes: self.cache.put(current.generation, &canonical, bytes),
        })
    }

    /// Copying a file into the tree.
    ///
    /// Nothing is written this round: the layout says what each destination would mean and this
    /// refuses by that name. RFC 0003 section 5.2's five verbs are the write side's.
    ///
    /// # Errors
    ///
    /// Always: [`Refused::NotYetImplemented`] where the layout declares a meaning,
    /// [`Refused::ByDesign`] where it refuses one, [`VfsError::NoSuchPath`] where it names
    /// neither.
    pub fn write(&self, path: &str, _bytes: &[u8]) -> Result<(), VfsError> {
        Err(refusal_for(path, Verb::Write)?.into())
    }

    /// Deleting a path.
    ///
    /// # Errors
    ///
    /// As [`Vfs::write`].
    pub fn remove(&self, path: &str) -> Result<(), VfsError> {
        Err(refusal_for(path, Verb::Delete)?.into())
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
    };
    Ok(match meaning {
        Write::Refused(reason) => Refused::ByDesign {
            path: path.to_owned(),
            reason,
        },
        mapping => Refused::NotYetImplemented {
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
        },
        wanted,
    }
}
