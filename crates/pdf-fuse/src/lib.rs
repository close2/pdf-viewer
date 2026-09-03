//! RFC 0003's **FUSE face**: the kernel's file verbs, mapped onto `pdf-vfs`'s.
//!
//! RFC 0003 section 7 asks for a Rust binary — `pdffs <file.pdf> <mountpoint>`, with
//! `--foreground` and `--allow-other` off by default — on `fuser`'s default pure-Rust
//! `/dev/fuse` path, with no C linkage at all. And it says what is in this crate and what is
//! not: the faces contain no layout knowledge, so adding a `fonts/` directory one day is a core
//! change that both faces grow simultaneously.
//!
//! So there is no path pattern, no directory name and no generator here. Every question this
//! crate answers, it answers by asking [`pdf_vfs::Vfs`], and everything it adds is what a *kernel*
//! needs and a core does not: an inode for every name, a file handle for every open, and an
//! `errno` for every refusal.
//!
//! # The three things a FUSE face has to decide
//!
//! **An inode here is a name, not a page**, and that is forced rather than chosen. RFC 0003
//! section 5.2 makes an ordinal a position — "[o]rdinal names are **positions, not identities** —
//! the rule that makes insertion and deletion coherent: after any write, the next listing
//! renumbers" — so there is no page-shaped thing for an inode to be the identity *of*.
//! `pages/0004.pdf` names the fourth page of whatever the document now is. [`Inodes`] therefore
//! maps path to inode, one inode per path for the life of the mount, and the attribute and entry
//! timeouts are **zero**: the kernel is told to ask again every time, because the answer can have
//! changed for a reason no `stat` of ours would show.
//!
//! **An open file keeps the generation it was opened under**, which is RFC 0003 section 5.4's
//! rule and is already a property of [`pdf_vfs::Handle`]'s shape. [`Face::open`] materialises the
//! handle and files it under a file handle number; every `read` is a copy out of those bytes. No
//! reader ever receives a splice of two generations because there is no path by which a later
//! generation could reach a handle already open.
//!
//! **A refusal has no message channel**, which is RFC 0003 section 5.3's own complaint —
//! "FUSE returns `EROFS` for the derived directories and `EPERM` with no message channel — which
//! is FUSE's poverty, and why the mount also logs each refusal's sentence to its own
//! stderr/journal". Every method here that refuses hands the sentence to the [`Face`]'s log sink
//! before it returns the number, which is trap 5's rule applied to a mount.
//!
//! # What is *not* here
//!
//! The invalidation task. RFC 0003 section 5.4 requires the
//! `notify_inval_entry`/`notify_inval_inode` calls to be issued "from a separate task — separate
//! because issuing them synchronously from a request handler can deadlock against the kernel
//! (documented libfuse hazard)", so this crate offers [`Face::changed_since`] and the *binary*
//! runs the thread. A handler here never notifies.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod kernel;

use std::collections::HashMap;
use std::sync::Mutex;

use pdf_vfs::layout::{Kind, Write};
use pdf_vfs::{Committed, Errno, Handle, StagedId, Vfs, VfsError};

pub use crate::kernel::Mount;

/// The inode the kernel calls the root of any FUSE mount.
///
/// Fixed by the protocol rather than chosen: `FUSE_ROOT_ID` is 1, and the first `lookup` the
/// kernel ever sends names it as the parent.
pub const ROOT: u64 = 1;

/// How long the kernel may believe an answer of ours.
///
/// **Zero, and it is a decision with a reason.** A name in this tree is a position (RFC 0003
/// section 5.2), so `pages/0004.pdf` can be a different page after any write — ours or another
/// program's — with nothing about the name changed. A non-zero timeout would hand a file manager
/// a cached size for a page that no longer exists. The cost is a round trip per `stat`, and the
/// core's own cache is what makes that cheap: the bytes are generated once per generation and
/// [`pdf_vfs::Vfs::stat`] answers from the cache afterwards.
pub const TIMEOUT: std::time::Duration = std::time::Duration::ZERO;

/// One thing the kernel can be told about: a name, its inode, and what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Its inode number, which is stable for this path for the life of the mount.
    pub ino: u64,
    /// Its path in the core's tree.
    pub path: String,
    /// Directory or file.
    pub kind: Kind,
    /// The file's true size, `None` for a directory.
    ///
    /// True rather than estimated, because RFC 0003 section 5.5 says an estimate truncates: "the
    /// kernel clamps reads at the stated size … an under-estimate silently truncates a page".
    pub size: Option<u64>,
    /// Whether a write to this path means something, which is what makes the mode writable.
    pub writable: bool,
}

/// The path-to-inode table, and nothing else.
///
/// One inode per path, never reused, allocated on the first `lookup` or `readdir` that names it.
/// A path that stops existing keeps its number: the kernel may still hold it, and handing the
/// number to a different path later would make a stale `getattr` answer about somebody else.
#[derive(Debug, Default)]
struct Inodes {
    /// What each inode is called.
    by_ino: HashMap<u64, String>,
    /// What each path's number is.
    by_path: HashMap<String, u64>,
    /// The next number to hand out.
    next: u64,
}

impl Inodes {
    /// A fresh table, with the root already in it.
    fn new() -> Self {
        let mut inodes = Self {
            by_ino: HashMap::new(),
            by_path: HashMap::new(),
            next: ROOT.saturating_add(1),
        };
        inodes.by_ino.insert(ROOT, String::from("/"));
        inodes.by_path.insert(String::from("/"), ROOT);
        inodes
    }

    /// This path's number, allocating one if it has none.
    fn number(&mut self, path: &str) -> u64 {
        if let Some(known) = self.by_path.get(path) {
            return *known;
        }
        let ino = self.next;
        self.next = self.next.saturating_add(1);
        self.by_ino.insert(ino, path.to_owned());
        self.by_path.insert(path.to_owned(), ino);
        ino
    }

    /// What this number is called, or `None` for one this mount never handed out.
    fn path(&self, ino: u64) -> Option<&str> {
        self.by_ino.get(&ino).map(String::as_str)
    }
}

/// One file the kernel has open.
#[derive(Debug)]
enum Open {
    /// A read: the bytes, at the generation they were opened under.
    Read(Box<Handle>),
    /// A write in flight, which is in the tree and not in the document until a `flush`.
    ///
    /// The path is not here: [`pdf_vfs::Abandoned`] carries it, and a second copy would be a
    /// second thing that can disagree about where a write was going.
    Staged(StagedId),
}

/// Where a refusal's sentence goes, because FUSE has nowhere to put one.
pub type Sink = Box<dyn Fn(&str) + Send + Sync>;

/// The face: one document as a tree, with the kernel's numbers over it.
///
/// Everything is behind `&self` because `fuser`'s `Filesystem` methods take `&self` since its
/// 0.17 rework and the session may run more than one thread.
pub struct Face {
    /// The core. Every answer comes from here.
    vfs: Vfs,
    /// The kernel's numbers.
    inodes: Mutex<Inodes>,
    /// What the kernel has open, by file handle.
    open: Mutex<HashMap<u64, Open>>,
    /// The next file handle. Never reused, so a stale handle is an error rather than somebody
    /// else's file.
    next_handle: Mutex<u64>,
    /// Where a refusal's sentence goes.
    sink: Sink,
}

impl std::fmt::Debug for Face {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Face")
            .field("vfs", &self.vfs)
            .finish_non_exhaustive()
    }
}

impl Face {
    /// A face over this tree, logging its refusals here.
    #[must_use]
    pub fn new(vfs: Vfs, sink: Sink) -> Self {
        Self {
            vfs,
            inodes: Mutex::new(Inodes::new()),
            open: Mutex::new(HashMap::new()),
            next_handle: Mutex::new(1),
            sink,
        }
    }

    /// The tree underneath, for the binary's notifier thread.
    #[must_use]
    pub fn vfs(&self) -> &Vfs {
        &self.vfs
    }

    /// Says a sentence, and answers the `errno` beside it.
    ///
    /// The one place a [`VfsError`] becomes a number. RFC 0003 section 5.3: FUSE "returns `EPERM`
    /// with no message channel — which is FUSE's poverty, and why the mount also logs each
    /// refusal's sentence to its own stderr/journal".
    fn refuse(&self, verb: &str, error: &VfsError) -> Errno {
        let errno = error.errno();
        (self.sink)(&format!("{verb}: {error} [{}]", errno.as_str()));
        errno
    }

    /// Every inode this mount has handed out, with the directory it is in and the name it has
    /// there.
    ///
    /// What the binary's notifier thread invalidates. It is the *whole* set rather than a
    /// difference, because a generation change can move any name in the tree — a page inserted
    /// renumbers every ordinal after it — and there is nothing cheaper that is also correct.
    #[must_use]
    pub fn known(&self) -> Vec<(u64, u64, String)> {
        let inodes = self
            .inodes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        inodes
            .by_ino
            .iter()
            .filter(|(ino, _)| **ino != ROOT)
            .filter_map(|(ino, path)| {
                let (parent, name) = path.rsplit_once('/')?;
                let parent = if parent.is_empty() { "/" } else { parent };
                let number = inodes.by_path.get(parent)?;
                Some((*number, *ino, name.to_owned()))
            })
            .collect()
    }

    /// Whether the document has changed since this key, and what it is now.
    ///
    /// The notifier thread's whole question. RFC 0003 section 5.4 makes the generation key the
    /// truth about that — "(mtime, size, last `startxref` offset)" — and the core validates it
    /// before every answer anyway, so a face that polls it is asking the same question the next
    /// operation would.
    #[must_use]
    pub fn changed_since(&self, key: Option<pdf_vfs::generation::Generation>) -> Option<Changed> {
        let now = self.vfs.generation().ok()?;
        if key == Some(now) {
            return None;
        }
        Some(Changed { key: now })
    }

    /// The path a number names, or `ENOENT`.
    fn path_of(&self, ino: u64) -> Result<String, Errno> {
        self.inodes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .path(ino)
            .map(str::to_owned)
            .ok_or(Errno::NoSuchFile)
    }

    /// This path's number, allocating one.
    fn number_of(&self, path: &str) -> u64 {
        self.inodes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .number(path)
    }

    /// A path's node, by asking the core what is there.
    fn node(&self, path: &str) -> Result<Node, Errno> {
        let attributes = self
            .vfs
            .stat(path)
            .map_err(|error| self.refuse("stat", &error))?;
        Ok(Node {
            ino: self.number_of(path),
            path: path.to_owned(),
            kind: attributes.kind,
            size: attributes.size,
            writable: self.writable(path),
        })
    }

    /// Whether a write to this path means an operation, which the **core** decides.
    ///
    /// `Vfs::write_meaning` is the layout table answering, so the mode bits a person sees in
    /// `ls -l` are the document's own shape rather than a list this crate keeps. A row the core
    /// does not name at all is read-only, which is the honest answer for a path that is not in
    /// the tree.
    fn writable(&self, path: &str) -> bool {
        self.vfs.write_meaning(path).is_some_and(|mapping| {
            !matches!(mapping.on_write, Write::Refused(_))
                || !matches!(mapping.on_delete, Write::Refused(_))
        })
    }

    /// One name inside a directory.
    ///
    /// # Errors
    ///
    /// `ENOENT` for an inode this mount never handed out or a name the document does not have,
    /// and whatever the core says otherwise.
    pub fn lookup(&self, parent: u64, name: &str) -> Result<Node, Errno> {
        let path = join(&self.path_of(parent)?, name)?;
        self.node(&path)
    }

    /// One inode's attributes.
    ///
    /// # Errors
    ///
    /// As [`Face::lookup`].
    pub fn getattr(&self, ino: u64) -> Result<Node, Errno> {
        let path = self.path_of(ino)?;
        self.node(&path)
    }

    /// A directory's entries, `.` and `..` excluded — those are the adapter's.
    ///
    /// # Errors
    ///
    /// `ENOTDIR` for a file, and whatever the core says about the listing.
    pub fn readdir(&self, ino: u64) -> Result<Vec<Node>, Errno> {
        let path = self.path_of(ino)?;
        let listed = self
            .vfs
            .list(&path)
            .map_err(|error| self.refuse("readdir", &error))?;
        Ok(listed
            .into_iter()
            .map(|entry| {
                let child = join(&path, &entry.name).unwrap_or_else(|_| path.clone());
                Node {
                    ino: self.number_of(&child),
                    kind: entry.kind,
                    // A listing does not generate, and RFC 0003 section 5.5 is why: "[d]irectory
                    // listings are cheap — names and types come from the document's structure —
                    // and file managers stat lazily, so browsing stays fast and the cost lands on
                    // the first touch of each file". The size arrives with the `stat` that
                    // follows.
                    size: None,
                    writable: self.writable(&child),
                    path: child,
                }
            })
            .collect())
    }

    /// Opening a file for reading: the bytes are materialised now, at this generation.
    ///
    /// # Errors
    ///
    /// `EISDIR` for a directory, and whatever generating the file costs.
    pub fn open(&self, ino: u64) -> Result<u64, Errno> {
        let path = self.path_of(ino)?;
        let handle = self
            .vfs
            .open(&path)
            .map_err(|error| self.refuse("open", &error))?;
        Ok(self.file(Open::Read(Box::new(handle))))
    }

    /// `count` bytes from `offset` of an open file.
    ///
    /// # Errors
    ///
    /// `EBADF` for a handle this mount did not hand out, and `EIO` for a read of a write in
    /// flight — which cannot happen through a kernel that opened it, and is refused rather than
    /// guessed at.
    pub fn read(&self, handle: u64, offset: u64, count: u32) -> Result<Vec<u8>, Errno> {
        let open = self
            .open
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match open.get(&handle) {
            Some(Open::Read(bytes)) => Ok(bytes
                .read(offset, usize::try_from(count).unwrap_or(0))
                .to_vec()),
            // A staged write is readable *through the tree* — `Vfs::open` answers it, which is
            // what makes `cp` work — but not through its own write handle, because the kernel
            // never asks that of a descriptor it opened with `O_WRONLY`.
            Some(Open::Staged(_)) => Err(Errno::InputOutput),
            None => Err(Errno::NoSuchFile),
        }
    }

    /// Creating a file: the write the kernel is about to make, staged.
    ///
    /// # Errors
    ///
    /// Whatever the core refuses the write for — `EPERM` for a directory whose shape is the
    /// document's, `EEXIST` for a name §7.7.4's tree already files, `EROFS` for a derived file.
    pub fn create(&self, parent: u64, name: &str) -> Result<(Node, u64), Errno> {
        let path = join(&self.path_of(parent)?, name)?;
        let id = self
            .vfs
            .create(&path)
            .map_err(|error| self.refuse("create", &error))?;
        let handle = self.file(Open::Staged(id));
        Ok((
            Node {
                ino: self.number_of(&path),
                kind: Kind::File,
                size: Some(0),
                writable: true,
                path,
            },
            handle,
        ))
    }

    /// Bytes into a write in flight.
    ///
    /// # Errors
    ///
    /// `EBADF` for a handle this mount did not hand out, `EFBIG` past the staging ceiling, and
    /// `EPERM` for a write to a handle that was opened for reading.
    pub fn write(&self, handle: u64, offset: u64, bytes: &[u8]) -> Result<u32, Errno> {
        let id = self.staged(handle)?;
        let written = self
            .vfs
            .write_at(id, offset, bytes)
            .map_err(|error| self.refuse("write", &error))?;
        Ok(u32::try_from(written).unwrap_or(u32::MAX))
    }

    /// `ftruncate(2)` on a write in flight, which is what a `setattr` with a size is.
    ///
    /// # Errors
    ///
    /// As [`Face::write`].
    pub fn truncate(&self, handle: u64, length: u64) -> Result<(), Errno> {
        let id = self.staged(handle)?;
        self.vfs
            .truncate(id, length)
            .map_err(|error| self.refuse("truncate", &error))
    }

    /// The commit point.
    ///
    /// RFC 0003 section 5.4: "a FUSE write buffers; **validation and commit happen on `flush`**,
    /// whose error return reaches the application's `close()` — `release` reaches nobody, which
    /// is why it is only cleanup." A `flush` of a read handle is nothing, which is what the
    /// kernel expects: it issues one per `close(2)` whatever the descriptor was for.
    ///
    /// # Errors
    ///
    /// Whatever the commit refuses: `ESTALE` where the document changed under the staged write,
    /// `EIO` where what was copied in is not a document, `EACCES` where the host's level said to
    /// obey the document's own restriction.
    pub fn flush(&self, handle: u64) -> Result<Option<Committed>, Errno> {
        let Ok(id) = self.staged(handle) else {
            return Ok(None);
        };
        let committed = self
            .vfs
            .flush(id)
            .map_err(|error| self.refuse("flush", &error))?;
        for warning in &committed.warnings {
            (self.sink)(&format!("{}: {warning}", committed.path));
        }
        Ok(Some(committed))
    }

    /// Cleanup, and the log line an abandoned write leaves.
    ///
    /// `release` reaches no application, so its only job is to stop holding what the kernel has
    /// let go — and to say, where a write was never flushed, that nothing was committed.
    pub fn release(&self, handle: u64) {
        let open = self
            .open
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&handle);
        // `None` back from the core means the `flush` before this committed it, which is the
        // ordinary case and has nothing to say.
        if let Some(Open::Staged(id)) = open
            && let Some(abandoned) = self.vfs.release(id)
        {
            (self.sink)(&abandoned.sentence());
        }
    }

    /// Removing a name.
    ///
    /// # Errors
    ///
    /// Whatever the core refuses it for; the two it means are a page deleted from §7.7.3.2's
    /// tree and an embedded file removed from §7.7.4's.
    pub fn unlink(&self, parent: u64, name: &str) -> Result<Committed, Errno> {
        let path = join(&self.path_of(parent)?, name)?;
        let committed = self
            .vfs
            .remove(&path)
            .map_err(|error| self.refuse("unlink", &error))?;
        for warning in &committed.warnings {
            (self.sink)(&format!("{path}: {warning}"));
        }
        Ok(committed)
    }

    /// Renaming, which RFC 0003 section 5.3 refuses in v1 whatever it names.
    ///
    /// # Errors
    ///
    /// Always, and the sentence says why: "[r]ename semantics under position-names are ambiguous
    /// … Reorder belongs to the transform CLI".
    pub fn rename(&self, parent: u64, name: &str, new_parent: u64, new_name: &str) -> Errno {
        let (Ok(from), Ok(to)) = (self.path_of(parent), self.path_of(new_parent)) else {
            return Errno::NoSuchFile;
        };
        let (Ok(from), Ok(to)) = (join(&from, name), join(&to, new_name)) else {
            return Errno::Invalid;
        };
        match self.vfs.rename(&from, &to) {
            Ok(()) => Errno::OperationNotPermitted,
            Err(error) => self.refuse("rename", &error),
        }
    }

    /// Creating a directory, which the core refuses: every directory here is the document's own
    /// shape.
    ///
    /// # Errors
    ///
    /// Always.
    pub fn mkdir(&self, parent: u64, name: &str) -> Errno {
        let Ok(parent) = self.path_of(parent) else {
            return Errno::NoSuchFile;
        };
        let Ok(path) = join(&parent, name) else {
            return Errno::Invalid;
        };
        match self.vfs.create_directory(&path) {
            Ok(()) => Errno::OperationNotPermitted,
            Err(error) => self.refuse("mkdir", &error),
        }
    }

    /// Files an open thing under a fresh number.
    fn file(&self, open: Open) -> u64 {
        let mut next = self
            .next_handle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let handle = *next;
        *next = next.saturating_add(1);
        drop(next);
        self.open
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(handle, open);
        handle
    }

    /// The staged write behind a handle, or why it is not one.
    fn staged(&self, handle: u64) -> Result<StagedId, Errno> {
        match self
            .open
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&handle)
        {
            Some(Open::Staged(id)) => Ok(*id),
            Some(Open::Read(_)) => Err(Errno::OperationNotPermitted),
            None => Err(Errno::NoSuchFile),
        }
    }
}

/// The document is at a new generation, and every name the kernel holds may mean something else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Changed {
    /// The key it is at now, to be polled against next time.
    pub key: pdf_vfs::generation::Generation,
}

/// A parent path and a name, joined into a path the core reads.
///
/// A name with a solidus in it is `EINVAL` rather than a path with two components: the kernel
/// cannot send one, and accepting it here would let a caller reach a directory it did not name.
fn join(parent: &str, name: &str) -> Result<String, Errno> {
    if name.is_empty() || name.contains('/') {
        return Err(Errno::Invalid);
    }
    if parent == "/" {
        Ok(format!("/{name}"))
    } else {
        Ok(format!("{parent}/{name}"))
    }
}
