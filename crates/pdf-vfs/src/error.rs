//! What an operation on the tree can be refused with, and the words each refusal is given.
//!
//! [`VfsError`] is the one error every [`crate::Vfs`] operation returns; a face maps it onto its
//! own protocol with [`VfsError::errno`] and prints [`VfsError`]'s sentence beside it (RFC 0003
//! section 5.3 insists the sentence exist). [`Refused`] is the part of it that is a decision of
//! the layout rather than a failure, and [`Write`]'s `name` words the mapping a refusal names.

use crate::commit::Errno;
use crate::layout::{Reason, Write};
use crate::worker::WorkerError;

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
    /// More bytes than [`crate::Config::max_staged_bytes`] allows one write in flight.
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
    /// A directory the document would fill past [`crate::Config::max_entries`].
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
                // §7.6.4.1's password. `EACCES` and the sentence beside it, which is the same
                // poverty as the two above: there is no errno for "and here is how to supply
                // one", so the face that built the mount is where the password goes in
                // ([`Vfs::with_password`]) and the sentence is what says so.
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
    /// [`crate::worker::WorkerError::Mismatched`] is: a layout table that grew a row this code did not
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
