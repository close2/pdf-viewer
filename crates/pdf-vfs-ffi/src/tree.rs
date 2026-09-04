//! The safe half: `pdf_vfs::Vfs` with every value in a shape a C header can declare.
//!
//! Nothing here is `unsafe` and nothing here decides anything about the tree. Every path,
//! `errno`, sentence and listing comes out of [`pdf_vfs`]; what this module adds is the three
//! things a C boundary needs and a Rust API does not — an owned answer where the core lends one,
//! a small integer where the core has an enumeration, and a document opened from a path where
//! the core takes a [`pdf_vfs::generation::Backing`] somebody else made.

use pdf_vfs::layout::{Kind, Write};
use pdf_vfs::{Config, ConfinedWorkers, Errno, FileBacking, Handle, Vfs};

use crate::refusal::Refusal;
use crate::status::Status;

/// `CLAUDE.md` principle 3's four levels, as the numbers the header declares.
///
/// **The policy is asked once, where a host can supply it** — which is the principle's own words
/// and the reason this is an argument to [`Mount::open`] rather than a switch anywhere later.
/// `Off` is the default and stays the default: the program is the reader's.
///
/// What `Ask` means *for this face* is the honest part. The question is decided inside the
/// confined worker, a process RFC 0003 section 6 gives no channel to a person at all, so today
/// the level reaches a caller as `EACCES` with a sentence saying a question went unanswered.
/// A KIO worker has `WorkerBase::messageBox` and could put it; ADR 0869 says what the wire owes
/// before it can.
pub const LEVEL_OFF: u32 = 0;
/// The document's assertions are obeyed and the operation is refused. See [`LEVEL_OFF`].
pub const LEVEL_ON: u32 = 1;
/// The person is asked. See [`LEVEL_OFF`] for what this face can and cannot do with it.
pub const LEVEL_ASK: u32 = 2;
/// The operation proceeds and the reasons are said afterwards, as a commit's warnings.
pub const LEVEL_WARN: u32 = 3;

/// What a write to a path means, as the numbers the header declares.
///
/// [`pdf_vfs::layout::Write`], narrowed to a number and nothing else: the *reason* a refused row
/// gives is a sentence, and a sentence reaches a caller through [`Refusal`] rather than through
/// an enumeration it would have to translate.
pub const MEANS_NOTHING: u32 = 0;
/// RFC 0003 section 5.2's first verb: the copied document's pages, inserted at this position.
pub const MEANS_INSERT_PAGES: u32 = 1;
/// Deleting this page.
pub const MEANS_DELETE_PAGE: u32 = 2;
/// Embedding the copied file as a §7.11.4 embedded file.
pub const MEANS_EMBED_FILE: u32 = 3;
/// Removing this embedded file from §7.7.4's name tree.
pub const MEANS_REMOVE_ATTACHMENT: u32 = 4;
/// Setting the §14.3.3 entries the written file states.
pub const MEANS_SET_INFORMATION: u32 = 5;

/// A directory, as [`Kind`] says.
pub const KIND_DIRECTORY: u32 = 0;
/// A file.
pub const KIND_FILE: u32 = 1;

/// The number a [`Write`] is.
#[must_use]
pub fn meaning_of(write: Write) -> u32 {
    match write {
        Write::InsertPages => MEANS_INSERT_PAGES,
        Write::DeletePage => MEANS_DELETE_PAGE,
        Write::EmbedFile => MEANS_EMBED_FILE,
        Write::RemoveAttachment => MEANS_REMOVE_ATTACHMENT,
        Write::SetInformation => MEANS_SET_INFORMATION,
        Write::Refused(_) => MEANS_NOTHING,
    }
}

/// The number a [`Kind`] is.
#[must_use]
pub fn kind_of(kind: Kind) -> u32 {
    match kind {
        Kind::Directory => KIND_DIRECTORY,
        Kind::File => KIND_FILE,
    }
}

/// The level a number names, or `None` for a number this build does not define.
///
/// A number outside the four is **refused** rather than rounded to the nearest level: a host
/// asking for a level this build has never heard of has asked for something, and quietly giving
/// it `Off` would be the program deciding to ignore a document's assertions on the host's behalf.
fn level_of(number: u32) -> Option<pdf_model::restriction::Level> {
    match number {
        LEVEL_OFF => Some(pdf_model::restriction::Level::Off),
        LEVEL_ON => Some(pdf_model::restriction::Level::On),
        LEVEL_ASK => Some(pdf_model::restriction::Level::Ask),
        LEVEL_WARN => Some(pdf_model::restriction::Level::Warn),
        _ => None,
    }
}

/// Where in a URL's path the document ends and the tree inside it begins.
///
/// `pdf:/home/u/doc.pdf/pages/0007.pdf` has to become a file to open and a path to serve, and
/// **nothing but the file system can say where the boundary is**: `doc.pdf` is a file and
/// `pages` is not, and no rule about names could know that. So the longest prefix that names a
/// regular file wins, tried from the right — the same construction `kio_archive` uses to decide
/// where an archive stops and its contents start (RFC 0003 section 2).
///
/// Answers the *length* of that prefix rather than two strings, so nothing is allocated and no
/// buffer crosses: the caller already holds the path, and the tree inside is the rest of it.
///
/// A trailing solidus is not part of the document's name — `stat(2)` refuses a regular file
/// named with one, which would make `pdf:/home/u/doc.pdf/` a path with no document in it.
#[must_use]
pub fn split(url_path: &str) -> Option<usize> {
    let trimmed = url_path.trim_end_matches('/');
    let mut end = trimmed.len();
    loop {
        let candidate = trimmed.get(..end)?;
        if !candidate.is_empty() && std::fs::metadata(candidate).is_ok_and(|it| it.is_file()) {
            return Some(end);
        }
        match candidate.rfind('/') {
            None | Some(0) => return None,
            Some(at) => end = at,
        }
    }
}

/// One document, as a tree.
#[derive(Debug)]
pub struct Mount {
    /// The core. Every answer comes from here.
    vfs: Vfs,
}

/// What a `stat` answers, in the two numbers a C struct holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attributes {
    /// [`KIND_DIRECTORY`] or [`KIND_FILE`].
    pub kind: u32,
    /// The file's true size — never estimated, which is RFC 0003 section 5.5's rule — or `None`
    /// for a directory.
    pub size: Option<u64>,
}

impl Mount {
    /// Opens a document, at the restriction level the host states.
    ///
    /// **Nothing is parsed here.** The file is checked to be a regular file so that the mistake a
    /// caller is likeliest to make — a directory, or a path that has gone — is one sentence
    /// rather than a worker that starts and fails; the document itself is read by the confined
    /// generator on the first question, which is RFC 0003 section 5.1's rule that listing the
    /// root reads nothing but the page count.
    ///
    /// The workers are [`ConfinedWorkers`] and there is no switch beside them, for RFC 0003
    /// section 6's reason: a face is fed hostile bytes by anything that opens a folder.
    ///
    /// # Errors
    ///
    /// A [`Refusal`] carrying `ENOENT` for a path that is not there, `EISDIR` for a directory,
    /// `EINVAL` for a restriction level this build does not define.
    pub fn open(document: &str, restrictions: u32) -> Result<Self, Refusal> {
        let Some(level) = level_of(restrictions) else {
            return Err(Refusal::stated(
                Errno::Invalid,
                format!(
                    "{restrictions} is not one of this boundary's four restriction levels (off, \
                     on, ask, warn)"
                ),
            ));
        };
        match std::fs::metadata(document) {
            Ok(found) if found.is_file() => {}
            Ok(_) => {
                return Err(Refusal::stated(
                    Errno::IsADirectory,
                    format!("{document}: not a file, so there is no document to serve"),
                ));
            }
            Err(error) => {
                return Err(Refusal::stated(
                    match error.kind() {
                        std::io::ErrorKind::PermissionDenied => Errno::PermissionDenied,
                        std::io::ErrorKind::NotFound => Errno::NoSuchFile,
                        _ => Errno::InputOutput,
                    },
                    format!("{document}: {error}"),
                ));
            }
        }
        let mut config = Config::default();
        config.policy.restrictions = level;
        Ok(Self {
            vfs: Vfs::new(
                Box::new(FileBacking::new(std::path::PathBuf::from(document))),
                Box::new(ConfinedWorkers),
                config,
            ),
        })
    }

    /// How many pages §7.7.3.2's tree holds.
    ///
    /// # Errors
    ///
    /// Whatever opening the document costs — a confined generator that will not start, a
    /// document that will not parse, §7.6.4.1's password.
    pub fn pages(&self) -> Result<u64, Refusal> {
        self.vfs
            .pages()
            .map(|count| u64::try_from(count).unwrap_or(u64::MAX))
            .map_err(|error| Refusal::of(&error))
    }

    /// A directory's entries.
    ///
    /// # Errors
    ///
    /// `ENOTDIR` for a file, `ENOENT` for a path the layout does not name, and the core's own.
    pub fn list(&self, path: &str) -> Result<Listing, Refusal> {
        self.vfs
            .list(path)
            .map(|entries| Listing { entries })
            .map_err(|error| Refusal::of(&error))
    }

    /// One path's kind and true size.
    ///
    /// # Errors
    ///
    /// As [`Mount::list`], plus whatever generating the file costs: RFC 0003 section 5.5 makes a
    /// `stat` generate, because a stated size that is not the true one truncates the file.
    pub fn stat(&self, path: &str) -> Result<Attributes, Refusal> {
        self.vfs
            .stat(path)
            .map(|attributes| Attributes {
                kind: kind_of(attributes.kind),
                size: attributes.size,
            })
            .map_err(|error| Refusal::of(&error))
    }

    /// Opens a file, materialising its bytes at the generation it was opened under.
    ///
    /// # Errors
    ///
    /// `EISDIR` for a directory, and whatever generating the file costs.
    pub fn open_file(&self, path: &str) -> Result<File, Refusal> {
        self.vfs
            .open(path)
            .map(|handle| File { handle })
            .map_err(|error| Refusal::of(&error))
    }

    /// What writing to and deleting this path would each mean, or `None` where the layout names
    /// no row at all.
    #[must_use]
    pub fn write_meaning(&self, path: &str) -> Option<(u32, u32)> {
        self.vfs
            .write_meaning(path)
            .map(|mapping| (meaning_of(mapping.on_write), meaning_of(mapping.on_delete)))
    }

    /// A whole write, as one transaction.
    ///
    /// RFC 0003 section 5.4: "a KIO `put` commits when the worker's `put` completes (KIO's verb is
    /// already transactional)". That is why this boundary states the transactional verb and not
    /// the staged four — `create`, `write_at`, `flush`, `release` are what a *kernel* hands a
    /// file system one call at a time, and the FUSE face reaches them in Rust.
    ///
    /// # Errors
    ///
    /// Every refusal the core states for a write: `EPERM` for a path whose shape is the
    /// document's, `EROFS` for a derived file, `EEXIST` for a name §7.7.4's tree already holds,
    /// `EIO` for bytes that are not a document, `EACCES` where the level said to obey the
    /// document's own restriction, `ESTALE` where the document changed underneath.
    pub fn write(&self, path: &str, bytes: &[u8]) -> Result<Commit, Refusal> {
        self.vfs
            .write(path, bytes)
            .map(Commit::of)
            .map_err(|error| Refusal::of(&error))
    }

    /// Removing a name: a page deleted from §7.7.3.2's tree, or an embedded file from §7.7.4's.
    ///
    /// # Errors
    ///
    /// As [`Mount::write`].
    pub fn remove(&self, path: &str) -> Result<Commit, Refusal> {
        self.vfs
            .remove(path)
            .map(Commit::of)
            .map_err(|error| Refusal::of(&error))
    }

    /// Renaming, which RFC 0003 section 5.3 refuses in v1 whatever it names.
    ///
    /// Answers the refusal rather than a `Result`, because there is no other answer: the core
    /// states one, with the sentence saying why reorder belongs to the transform command line.
    #[must_use]
    pub fn rename(&self, from: &str, to: &str) -> Refusal {
        match self.vfs.rename(from, to) {
            Err(error) => Refusal::of(&error),
            // Unreachable through the core as it stands, and written rather than asserted for the
            // reason `pdf_vfs::Refused::WrongVerb` is written: a core that grew an answer this
            // code did not grow an arm for should be a sentence rather than a panic.
            Ok(()) => Refusal::stated(
                Errno::OperationNotPermitted,
                format!("{from} -> {to}: renaming within this tree is refused"),
            ),
        }
    }

    /// Creating a directory, which the core refuses: every directory here is the document's own
    /// shape.
    #[must_use]
    pub fn create_directory(&self, path: &str) -> Refusal {
        match self.vfs.create_directory(path) {
            Err(error) => Refusal::of(&error),
            Ok(()) => Refusal::stated(
                Errno::OperationNotPermitted,
                format!("{path}: this tree's directories are the document's own shape"),
            ),
        }
    }

    /// What the layout declares and the core does not do, one sentence each.
    ///
    /// Trap 5 across a boundary: a face prints these rather than a person discovering them.
    #[must_use]
    pub fn shortfalls(&self) -> Vec<String> {
        self.vfs
            .shortfalls()
            .into_iter()
            .map(|shortfall| format!("{}: {}", shortfall.pattern, shortfall.detail))
            .collect()
    }
}

/// One directory listing, owned.
#[derive(Debug)]
pub struct Listing {
    /// The entries, in the order the core answered with.
    entries: Vec<pdf_vfs::DirEntry>,
}

impl Listing {
    /// How many entries there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there are none, which an empty `attachments/` answers.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// One entry's name.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such entry.
    pub fn name(&self, index: usize) -> Result<&str, Status> {
        self.entries
            .get(index)
            .map(|entry| entry.name.as_str())
            .ok_or(Status::OutOfRange)
    }

    /// One entry's kind, [`KIND_DIRECTORY`] or [`KIND_FILE`].
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such entry.
    pub fn kind(&self, index: usize) -> Result<u32, Status> {
        self.entries
            .get(index)
            .map(|entry| kind_of(entry.kind))
            .ok_or(Status::OutOfRange)
    }
}

/// One open virtual file, at the generation it was opened under.
#[derive(Debug)]
pub struct File {
    /// The core's handle, which holds the bytes.
    handle: Handle,
}

impl File {
    /// How many bytes there are.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.handle.len()
    }

    /// Whether it is empty, which a page with no text is.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.handle.is_empty()
    }

    /// Up to `count` bytes from `offset`, as `read(2)` answers: short at the end, empty past it.
    #[must_use]
    pub fn read(&self, offset: u64, count: usize) -> &[u8] {
        self.handle.read(offset, count)
    }
}

/// What a write did, owned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    /// How many pages the document has now, which is what a renumbered listing will show.
    pages: u64,
    /// What the transform said on the way, including principle 3's *warn* level.
    warnings: Vec<String>,
}

impl Commit {
    /// What the core answered, narrowed to what crosses.
    fn of(committed: pdf_vfs::Committed) -> Self {
        Self {
            pages: u64::try_from(committed.pages).unwrap_or(u64::MAX),
            warnings: committed.warnings,
        }
    }

    /// How many pages the document has now.
    #[must_use]
    pub fn pages(&self) -> u64 {
        self.pages
    }

    /// How many warnings there are.
    #[must_use]
    pub fn warnings(&self) -> usize {
        self.warnings.len()
    }

    /// One warning's sentence.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such warning.
    pub fn warning(&self, index: usize) -> Result<&str, Status> {
        self.warnings
            .get(index)
            .map(String::as_str)
            .ok_or(Status::OutOfRange)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        KIND_DIRECTORY, KIND_FILE, LEVEL_ASK, LEVEL_OFF, LEVEL_ON, LEVEL_WARN, MEANS_DELETE_PAGE,
        MEANS_NOTHING, Mount, kind_of, level_of, meaning_of, split,
    };
    use pdf_vfs::layout::{Kind, Reason, Write};

    /// The four levels are the four numbers, and a fifth is refused rather than rounded.
    #[test]
    fn the_restriction_levels_are_the_four_the_principle_states() {
        use pdf_model::restriction::Level;
        assert_eq!(level_of(LEVEL_OFF), Some(Level::Off));
        assert_eq!(level_of(LEVEL_ON), Some(Level::On));
        assert_eq!(level_of(LEVEL_ASK), Some(Level::Ask));
        assert_eq!(level_of(LEVEL_WARN), Some(Level::Warn));
        assert_eq!(level_of(4), None, "a fifth level is not silently `off`");
    }

    /// Every meaning is its own number, and a refused row is zero whatever its reason.
    #[test]
    fn every_write_meaning_has_its_own_number() {
        let mut numbers = vec![
            meaning_of(Write::InsertPages),
            meaning_of(Write::DeletePage),
            meaning_of(Write::EmbedFile),
            meaning_of(Write::RemoveAttachment),
            meaning_of(Write::SetInformation),
        ];
        numbers.sort_unstable();
        numbers.dedup();
        assert_eq!(numbers.len(), 5, "two verbs share a number");
        assert_eq!(meaning_of(Write::DeletePage), MEANS_DELETE_PAGE);
        for reason in [
            Reason::LayoutIsNotWritable,
            Reason::TextIsNotAByteStream,
            Reason::ImageReplacementNotDesigned,
            Reason::Derived,
            Reason::ReorderIsAmbiguous,
            Reason::NotOneOfTheFiveVerbs,
        ] {
            assert_eq!(
                meaning_of(Write::Refused(reason)),
                MEANS_NOTHING,
                "a refused row's number says nothing about which reason it is; the sentence does"
            );
        }
        assert_eq!(kind_of(Kind::Directory), KIND_DIRECTORY);
        assert_eq!(kind_of(Kind::File), KIND_FILE);
    }

    /// A URL's path splits at the longest prefix that is a file, whatever follows it.
    ///
    /// Driven against this crate's own manifest rather than a corpus document, because what is
    /// under test is `stat(2)` and a path, and a file that is always there is what makes the test
    /// always run (the same argument `viewer-ffi`'s C driver makes for its hand-written form).
    #[test]
    fn a_url_splits_at_the_longest_prefix_that_is_a_file() {
        let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
        assert_eq!(split(manifest), Some(manifest.len()));
        let inside = format!("{manifest}/pages/0001.pdf");
        assert_eq!(
            split(&inside),
            Some(manifest.len()),
            "the document is the prefix, and everything after it is the tree"
        );
        assert_eq!(
            split(&format!("{manifest}/")),
            Some(manifest.len()),
            "a trailing solidus is not part of the document's name"
        );
        assert_eq!(
            split(env!("CARGO_MANIFEST_DIR")),
            None,
            "a directory is not a document, and neither is anything above it"
        );
        assert_eq!(split("/"), None);
        assert_eq!(split(""), None);
    }

    /// A path that is not a file is refused with a sentence, before any worker is started.
    #[test]
    fn opening_something_that_is_not_a_document_is_one_sentence() {
        let refused = Mount::open(env!("CARGO_MANIFEST_DIR"), LEVEL_OFF)
            .err()
            .map(|refusal| (refusal.code(), refusal.sentence().to_owned()));
        assert_eq!(refused.as_ref().map(|(code, _)| *code), Some(21), "EISDIR");
        let missing = Mount::open("/nowhere/at/all.pdf", LEVEL_OFF)
            .err()
            .map(|refusal| refusal.code());
        assert_eq!(missing, Some(2), "ENOENT");
        let level = Mount::open(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"), 9)
            .err()
            .map(|refusal| refusal.code());
        assert_eq!(level, Some(22), "EINVAL for a level this build has not got");
    }
}
