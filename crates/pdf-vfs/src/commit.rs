//! The transactional half: a write staged, a write committed, and the `errno` each refusal is.
//!
//! # Why a file-system face needs this and a command line does not
//!
//! `pdf-transform new.pdf --insert …` is one call: the arguments arrive together, the operation
//! happens, the status comes back. A POSIX write is four or more — `create`, `write` several
//! times, `flush`, `close` — and the bytes that decide whether the operation is even *possible*
//! arrive in the middle. So the face has to hold a partial write somewhere, decide what the tree
//! looks like while it is held, and choose a moment to make it real. RFC 0003 section 5.4 chooses
//! the moment: a FUSE write buffers, validation and commit happen on `flush`, whose error return
//! reaches the application's `close()` — `release` reaches nobody, which is why it is only
//! cleanup.
//!
//! This module is that sentence as types. [`Staged`] is a write in flight; [`Committed`] is what
//! a `flush` produced; [`Abandoned`] is what a `release` without a `flush` found, which is a
//! write that never happened.
//!
//! # The four answers this module gives, each of them a decision
//!
//! **What an uncommitted write looks like in the tree.** It is *there*: listed in its directory,
//! `stat`-able at the length written so far, readable back byte for byte — and it is in memory,
//! not in the document. That is what a real file system does with a half-copied file, and it is
//! what `cp` needs: every copying tool stats its destination after writing, and a tree that
//! showed nothing there would fail the copy it had just accepted. A second reader sees the same
//! thing: the document as it was, plus a file that is on its way in.
//!
//! **What happens if `flush` never comes.** Nothing. The staged bytes are dropped and the
//! document is not touched — an abandoned write leaves the file byte for byte as it was, and the
//! face logs [`Abandoned`] so that the disappearance of a name a listing showed is explained
//! rather than discovered. A writer that was killed mid-copy is the case this is really about,
//! and it has the second half of its answer below: the kernel calls `flush` on a killed
//! process's descriptors too, so what stops a torn copy from becoming a torn document is
//! **validation**, not the absence of a flush. A truncated PDF copied into `pages/` does not
//! open, so the insertion is refused and the document is unchanged. A truncated file copied into
//! `attachments/` is embedded truncated — exactly as `cp` to a real file system leaves a
//! truncated file, because nothing anywhere knows how long the copy was meant to be.
//!
//! **What the commit is, and why it cannot be observed half-done.** The confined worker computes
//! the whole updated document — the source's bytes and then §7.5.6's update — and the broker
//! checks the clause's own property against the file on disk before writing anything:
//!
//! > When updating a PDF file incrementally, changes shall be appended to the end of the file,
//! > leaving its original contents intact.
//!
//! A byte-for-byte prefix comparison, which is a comparison and not a parse, so it stays on the
//! side of RFC 0003 section 6's line where the broker lives. Only then is the file replaced, and
//! it is replaced by [`Backing::commit`], which POSIX's `rename` requires to be atomic — if the
//! new name exists it is removed and the old is renamed to it, in one step no reader can fall
//! inside.
//!
//! **A bare append would be smaller and is not what this does**, and the reason is worth stating
//! because §7.5.6 makes the append look free: a `write(2)` of the suffix can be interrupted
//! anywhere, and the last few bytes of an update are `startxref`, an offset and `%%EOF` — the
//! three things §7.5.5 makes a reader enter the file by. An append cut short inside them leaves a
//! file whose tail names a cross-reference section that is not there. A rename cannot be cut
//! short: a reader sees the whole old file or the whole new one, and a crash before it leaves the
//! original untouched. What §7.5.6 buys here is not the writing but the *checking* — because the
//! new file is the old file plus a suffix, the broker can prove no byte of the producer's was
//! lost without reading either as a PDF.
//!
//! **What our own commit does to the generation key.** The key changes, because the file
//! changed — and RFC 0003 section 5.4's rule is that a changed key rebuilds the tree. What must
//! not happen is that the rebuild looks like somebody *else*'s edit, since a face reacts to those
//! differently: it invalidates the kernel's caches, tells the file manager, and may want to say
//! so. So a commit records the key it left on disk, and the generation built for that key is
//! [`Provenance::Ours`]; any other new key is [`Provenance::Foreign`]. The whole transition
//! happens under the lock the commit already holds, so no operation can fall between the two
//! generations and see a tree belonging to neither.

use crate::generation::Generation;
use crate::layout::{Route, Write};

/// A write in flight, addressed by the token [`crate::Vfs::create`] handed back.
///
/// `Copy` and eight bytes: a face keeps one of these per open file handle, and a FUSE face keeps
/// it in the `fh` field the kernel gives it for exactly this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StagedId(pub(crate) u64);

impl StagedId {
    /// The token as a number, for a face whose protocol carries one.
    #[must_use]
    pub fn as_u64(self) -> u64 {
        self.0
    }

    /// The token a number names.
    #[must_use]
    pub fn from_u64(value: u64) -> Self {
        Self(value)
    }
}

/// One write in flight.
#[derive(Debug)]
pub(crate) struct Staged {
    /// Where it is going, canonically.
    pub(crate) path: String,
    /// Which row of the layout that is.
    pub(crate) route: &'static Route,
    /// What the path captured — the position a page goes at, the name a file takes.
    pub(crate) captures: crate::path::Captures,
    /// The generation it was created against, which is the generation it must still be at when
    /// it commits — unless every transition since then was this tree's own, which
    /// [`Staged::foreign_edits`] is what decides.
    pub(crate) generation: Generation,
    /// How many *foreign* generation transitions this tree had seen when the write was staged.
    ///
    /// The discriminator between the two reasons a staged write can find the document moved.
    /// Somebody else's edit is what [`Provenance::Foreign`] names and what `ESTALE` is for:
    /// committing over it could discard it, and RFC 0003 section 5.4 does not let a face do that.
    /// **Our own commit is not that**, and refusing it is what a mount by hand found (round 911):
    /// two files copied into `attachments/` with both descriptors open lost the second, because
    /// the first one's commit moved the key the second was staged against — and `close(2)`'s
    /// error is a thing most programs do not look at, so it lost it *quietly*.
    pub(crate) foreign_edits: u64,
    /// What has been written so far.
    pub(crate) bytes: Vec<u8>,
    /// Whether anything has ever been written to it, or its length ever set.
    ///
    /// **The discriminator between "replace this with nothing" and "never wrote at all"**, and
    /// there is no other one: `fuser` does not pass `O_TRUNC` — the kernel handles it by sending
    /// a separate `SETATTR` of size zero after the `open` — so a handle opened for writing looks
    /// the same either way until somebody acts on it. `touch(1)` opens `O_WRONLY|O_CREAT`, sets
    /// the times and closes without writing a byte, and a mount by hand had that arrive as a
    /// commit of zero bytes and fail "not a PDF: no %PDF- header in the first 0 bytes" — an
    /// input/output error about a document nobody had touched (round 911). A `close(2)` of a
    /// file nothing was written to is not a write.
    pub(crate) touched: bool,
    /// Whether a `flush` has already made it real, so that a second `flush` — which the kernel
    /// issues on every `close` of every descriptor onto one file — does nothing rather than
    /// applying the edit twice.
    pub(crate) committed: bool,
}

/// A write that is staged and not yet in the document, as a listing shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// Its token.
    pub id: StagedId,
    /// Where it is going.
    pub path: String,
    /// How many bytes have been written so far.
    pub size: u64,
    /// What committing it will mean.
    pub meaning: Write,
}

/// What a `flush` did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Committed {
    /// Which path was written.
    pub path: String,
    /// What the write meant.
    pub meaning: Write,
    /// The generation the document had before.
    pub from: Generation,
    /// The generation it has now.
    pub to: Generation,
    /// How many pages it has now, which is what a renumbered `pages/` listing will show.
    pub pages: usize,
    /// What the transform said on the way — including `CLAUDE.md` principle 3's *warn*, which
    /// proceeds and speaks, and §7.5.6's own note that a deleted page's bytes stay in the file.
    pub warnings: Vec<String>,
}

/// A write that was dropped without a `flush`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Abandoned {
    /// Where it was going.
    pub path: String,
    /// How many bytes had been written.
    pub size: u64,
    /// What it would have meant.
    pub meaning: Write,
}

impl Abandoned {
    /// The sentence a face logs, because the name it listed is about to stop being there.
    #[must_use]
    pub fn sentence(&self) -> String {
        format!(
            "{}: {} bytes were written and never flushed, so nothing was committed and the \
             document is unchanged",
            self.path, self.size
        )
    }
}

/// Whose edit the generation being served is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// The first generation this tree ever served, which nobody's edit produced.
    Opened,
    /// This tree's own commit left this key on the file.
    Ours,
    /// The key changed and no commit of ours left it: another program wrote the document.
    Foreign,
}

/// The `errno` a face returns for one refusal.
///
/// **Named here rather than in either face**, for RFC 0003 section 7's reason: a face that chose
/// its own numbers would be two faces disagreeing about what a refused write is, and the KIO one
/// maps these onto `KIO::Error` while the FUSE one hands them to the kernel. The numbers are
/// Linux's `<asm-generic/errno-base.h>` and `<asm-generic/errno.h>`, which is the platform's
/// convention rather than anything the specification states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Errno {
    /// `EPERM` — this program will not do it: a write into a directory whose shape is the
    /// document's, a rename that would be an ambiguous reorder, a page's text edited through a
    /// byte stream.
    OperationNotPermitted,
    /// `ENOENT` — the layout does not name this path, or this document does not have it.
    NoSuchFile,
    /// `EIO` — the document, or the file being written into it, could not be read as one.
    InputOutput,
    /// `EACCES` — the document asserts something over its reader and the host's level said to
    /// obey it, or said to ask and a file system has nobody to ask.
    PermissionDenied,
    /// `EEXIST` — §7.7.4's tree already files an embedded file under that name.
    Exists,
    /// `ENOTDIR` — a file was listed.
    NotADirectory,
    /// `EISDIR` — a directory was read or written.
    IsADirectory,
    /// `EINVAL` — a name this directory cannot hold, or a file whose content this write cannot
    /// make sense of.
    Invalid,
    /// `EFBIG` — more bytes than a staged write may hold.
    TooBig,
    /// `EROFS` — a derived file: written by asking what it was derived from.
    ReadOnly,
    /// `ENOSYS` — the layout declares what a write here means and it is not built.
    NotImplemented,
    /// `ESTALE` — the document changed under a write that was already in flight.
    Stale,
    /// `EOVERFLOW` — the document would fill this directory past the ceiling.
    Overflow,
}

impl Errno {
    /// The number, as Linux states it.
    #[must_use]
    pub fn code(self) -> i32 {
        match self {
            Self::OperationNotPermitted => 1,
            Self::NoSuchFile => 2,
            Self::InputOutput => 5,
            Self::PermissionDenied => 13,
            Self::Exists => 17,
            Self::NotADirectory => 20,
            Self::IsADirectory => 21,
            Self::Invalid => 22,
            Self::TooBig => 27,
            Self::ReadOnly => 30,
            Self::NotImplemented => 38,
            Self::Overflow => 75,
            Self::Stale => 116,
        }
    }

    /// The name, for a log line that a person reads beside the number.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OperationNotPermitted => "EPERM",
            Self::NoSuchFile => "ENOENT",
            Self::InputOutput => "EIO",
            Self::PermissionDenied => "EACCES",
            Self::Exists => "EEXIST",
            Self::NotADirectory => "ENOTDIR",
            Self::IsADirectory => "EISDIR",
            Self::Invalid => "EINVAL",
            Self::TooBig => "EFBIG",
            Self::ReadOnly => "EROFS",
            Self::NotImplemented => "ENOSYS",
            Self::Overflow => "EOVERFLOW",
            Self::Stale => "ESTALE",
        }
    }
}
