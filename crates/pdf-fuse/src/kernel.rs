//! The adapter: `fuser`'s `Filesystem` in terms of [`crate::Face`], and nothing else.
//!
//! Every method here is the same three lines — read the kernel's arguments, ask the face, turn
//! the answer into a reply — and that sameness is the point. RFC 0003 section 7 puts the layout
//! in the core, so a face that had a decision of its own in one of these methods would be a
//! second place a path means something. The decisions this file *does* make are all about the
//! protocol: what a `FileAttr` says about a virtual file, which timeout the kernel is given, and
//! which of `fuser`'s numbers a [`pdf_vfs::Errno`] is.

use std::ffi::OsStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fuser::{
    BsdFileFlags, Errno as KernelErrno, FileAttr, FileType, FopenFlags, Generation, INodeNo,
    OpenFlags, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEmpty, ReplyEntry,
    ReplyOpen, ReplyStatfs, ReplyWrite, Request, TimeOrNow,
};
use pdf_vfs::Errno;
use pdf_vfs::layout::Kind;

use crate::{Face, Node, TIMEOUT};

/// The FUSE session over one document.
///
/// A thin newtype rather than a bare `Face`, so that `fuser`'s trait is implemented on this
/// crate's own type and the face stays usable — and testable — without a kernel anywhere near it.
/// The face is shared rather than owned because RFC 0003 section 5.4's invalidation task needs
/// the same one the request handlers have.
#[derive(Debug)]
pub struct Mount {
    /// The face every answer comes from.
    face: std::sync::Arc<Face>,
    /// Whose the mount is. See [`owner`].
    owner: (u32, u32),
}

impl Mount {
    /// A mount over this face.
    #[must_use]
    pub fn new(face: std::sync::Arc<Face>) -> Self {
        Self {
            face,
            owner: owner(),
        }
    }

    /// What one virtual thing looks like to `stat(2)` on *this* mount.
    fn attributes(&self, node: &Node) -> FileAttr {
        attributes(node, self.owner, self.face.modified_nanos())
    }

    /// The face underneath, for the notifier thread the binary runs.
    #[must_use]
    pub fn face(&self) -> &Face {
        &self.face
    }
}

/// Whose the mount's files are.
///
/// **Not zero, which is what they used to be.** A mount by hand put `root root` on every line of
/// every `ls -l` of a document owned by uid 1001 (round 911): the kernel does no permission check
/// of its own here — `default_permissions` is not among the mount options and the access-control
/// list is the mounting user alone — so nothing was *permitted* that should not have been, but
/// every program that reads a listing rather than trying the operation was told the wrong thing,
/// and a file manager greys out what it believes belongs to root.
///
/// The real user and group of this process, read from `/proc/self`'s own ownership, which is
/// where they are without a system call this crate could make: `#![forbid(unsafe_code)]` rules
/// out `libc::getuid`, and a dependency for two integers is not worth a line in `doc/stack.md`.
/// A tree without `/proc` answers `(0, 0)`, which is what this was everywhere.
fn owner() -> (u32, u32) {
    use std::os::unix::fs::MetadataExt as _;
    std::fs::metadata("/proc/self").map_or((0, 0), |it| (it.uid(), it.gid()))
}

/// The number `fuser` puts on the wire for one of the core's refusals.
///
/// A total function over [`pdf_vfs::Errno`] rather than a conversion from its `code()`, because
/// `fuser::Errno` holds a `NonZeroI32` and building one from an integer would need a fallible
/// step in a path that cannot fail. The core's own doc comments state which POSIX name each
/// variant is; this is that table, compiled.
fn errno(errno: Errno) -> KernelErrno {
    match errno {
        Errno::OperationNotPermitted => KernelErrno::EPERM,
        Errno::NoSuchFile => KernelErrno::ENOENT,
        Errno::InputOutput => KernelErrno::EIO,
        Errno::PermissionDenied => KernelErrno::EACCES,
        Errno::Exists => KernelErrno::EEXIST,
        Errno::NotADirectory => KernelErrno::ENOTDIR,
        Errno::IsADirectory => KernelErrno::EISDIR,
        Errno::Invalid => KernelErrno::EINVAL,
        Errno::TooBig => KernelErrno::EFBIG,
        Errno::ReadOnly => KernelErrno::EROFS,
        Errno::NotImplemented => KernelErrno::ENOSYS,
        Errno::Stale => KernelErrno::ESTALE,
        Errno::Overflow => KernelErrno::EOVERFLOW,
    }
}

/// What `stat(2)` says about one virtual thing.
///
/// The times are the **document's**, and every name in the tree carries the same one: nothing
/// here has a modification time of its own, a page being generated on demand from a document
/// whose own mtime is the backing file's. [`Face::modified_nanos`] says where it comes from and
/// what the epoch it replaced cost.
///
/// The owner is the mounting process's rather than root's; [`owner`] says why that is a fix
/// rather than a preference.
fn attributes(node: &Node, owner: (u32, u32), modified_nanos: Option<i128>) -> FileAttr {
    let modified = modified_nanos.map_or(UNIX_EPOCH, |nanos| {
        let nanos = nanos.max(0);
        // Split before the cast so that a file dated past 2554 saturates rather than wrapping;
        // `Duration` is unsigned seconds and this is the one place a file system's clock reaches
        // this crate.
        let seconds = u64::try_from(nanos / 1_000_000_000).unwrap_or(u64::MAX);
        let rest = u32::try_from(nanos % 1_000_000_000).unwrap_or(0);
        UNIX_EPOCH
            .checked_add(Duration::new(seconds, rest))
            .unwrap_or(UNIX_EPOCH)
    });
    let kind = match node.kind {
        Kind::Directory => FileType::Directory,
        Kind::File => FileType::RegularFile,
    };
    let permissions: u16 = match (node.kind, node.writable) {
        (Kind::Directory, true) => 0o755,
        (Kind::Directory, false) => 0o555,
        (Kind::File, true) => 0o644,
        (Kind::File, false) => 0o444,
    };
    let size = node.size.unwrap_or(0);
    FileAttr {
        ino: INodeNo(node.ino),
        size,
        // 512-byte blocks, rounded up, which is what `du` reads.
        blocks: size.saturating_add(511) / 512,
        atime: modified,
        mtime: modified,
        ctime: modified,
        crtime: modified,
        kind,
        perm: permissions,
        nlink: 1,
        uid: owner.0,
        gid: owner.1,
        rdev: 0,
        blksize: 4096,
        flags: 0,
    }
}

/// The generation number a `lookup` answers with.
///
/// **Always zero, and that is correct rather than lazy.** The kernel uses (inode, generation) to
/// tell one file from another *after an inode number has been reused*, and this face never reuses
/// one: [`crate::Inodes`] hands out a fresh number for every path it sees and keeps it for the
/// life of the mount.
const NEW: Generation = Generation(0);

/// The name a kernel sent, as text, or `None` for bytes that are not UTF-8.
///
/// Every name in this tree is ASCII by construction — RFC 0003 section 4: "[a]ll names are ASCII,
/// generated, and stable within a generation of the document" — so a name that is not is a name
/// the tree cannot hold, and `ENOENT` is the honest answer rather than a lossy conversion that
/// would match the wrong file.
fn text(name: &OsStr) -> Option<&str> {
    name.to_str()
}

impl fuser::Filesystem for Mount {
    fn lookup(&self, _request: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        let Some(name) = text(name) else {
            reply.error(KernelErrno::ENOENT);
            return;
        };
        match self.face.lookup(parent.0, name) {
            Ok(node) => reply.entry(&TIMEOUT, &self.attributes(&node), NEW),
            Err(why) => reply.error(errno(why)),
        }
    }

    fn getattr(
        &self,
        _request: &Request,
        ino: INodeNo,
        _fh: Option<fuser::FileHandle>,
        reply: ReplyAttr,
    ) {
        match self.face.getattr(ino.0) {
            Ok(node) => reply.attr(&TIMEOUT, &self.attributes(&node)),
            Err(why) => reply.error(errno(why)),
        }
    }

    /// The only attribute this tree can be asked to change is a staged write's length, which is
    /// `ftruncate(2)`. Everything else — mode, owner, times — is refused rather than accepted and
    /// ignored, because a `chmod` that returns success and changes nothing is a lie a file
    /// manager will act on.
    ///
    /// **The times were the half of that sentence the code did not keep**, and a mount by hand
    /// found it the way the paragraph above predicts: `touch mnt/pages/0001.pdf` exited 0 and
    /// changed nothing, so `make`, `rsync -t` and every incremental build would believe a
    /// timestamp this tree cannot hold (round 911, trap 28 — the comment above the guard was a
    /// different claim from the guard).
    #[expect(
        clippy::similar_names,
        reason = "`ctime` and `crtime` are the FUSE protocol's own field names, and both are in \
                  the guard below"
    )]
    fn setattr(
        &self,
        _request: &Request,
        ino: INodeNo,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        atime: Option<TimeOrNow>,
        mtime: Option<TimeOrNow>,
        ctime: Option<SystemTime>,
        fh: Option<fuser::FileHandle>,
        crtime: Option<SystemTime>,
        chgtime: Option<SystemTime>,
        bkuptime: Option<SystemTime>,
        flags: Option<BsdFileFlags>,
        reply: ReplyAttr,
    ) {
        if mode.is_some()
            || uid.is_some()
            || gid.is_some()
            || atime.is_some()
            || mtime.is_some()
            || ctime.is_some()
            || crtime.is_some()
            || chgtime.is_some()
            || bkuptime.is_some()
            || flags.is_some()
        {
            reply.error(KernelErrno::EPERM);
            return;
        }
        // The two shapes a truncation arrives in: `ftruncate(2)` on a descriptor, and the
        // kernel's own `O_TRUNC` handling between an `open` and its first `write`, which carries
        // no handle. [`Face::truncate_at`] is the second, and why it exists.
        if let Some(size) = size {
            let outcome = match fh {
                Some(handle) => self.face.truncate(handle.0, size),
                None => self.face.truncate_at(ino.0, size),
            };
            if let Err(why) = outcome {
                reply.error(errno(why));
                return;
            }
        }
        match self.face.getattr(ino.0) {
            Ok(node) => reply.attr(&TIMEOUT, &self.attributes(&node)),
            Err(why) => reply.error(errno(why)),
        }
    }

    /// `O_RDONLY` reads the generated bytes; anything else stages a write.
    ///
    /// [`Face::open`] has the argument, and it is the defect a real kernel found: `cp` onto a
    /// name that already exists issues this rather than `create`, so ignoring the access mode
    /// made RFC 0003 section 5.2's first write verb unreachable from a shell.
    fn open(&self, _request: &Request, ino: INodeNo, flags: OpenFlags, reply: ReplyOpen) {
        let writing = !matches!(flags.acc_mode(), fuser::OpenAccMode::O_RDONLY);
        match self.face.open(ino.0, writing) {
            Ok(handle) => reply.opened(fuser::FileHandle(handle), FopenFlags::empty()),
            Err(why) => reply.error(errno(why)),
        }
    }

    fn read(
        &self,
        _request: &Request,
        _ino: INodeNo,
        fh: fuser::FileHandle,
        offset: u64,
        size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<fuser::LockOwner>,
        reply: ReplyData,
    ) {
        match self.face.read(fh.0, offset, size) {
            Ok(bytes) => reply.data(&bytes),
            Err(why) => reply.error(errno(why)),
        }
    }

    fn opendir(&self, _request: &Request, _ino: INodeNo, _flags: OpenFlags, reply: ReplyOpen) {
        // A directory listing is built per `readdir` from the document as it is then, so there is
        // nothing to hold open. Handle zero, which no `create` or `open` of this face hands out.
        reply.opened(fuser::FileHandle(0), FopenFlags::empty());
    }

    fn readdir(
        &self,
        _request: &Request,
        ino: INodeNo,
        _fh: fuser::FileHandle,
        offset: u64,
        mut reply: ReplyDirectory,
    ) {
        let listed = match self.face.readdir(ino.0) {
            Ok(listed) => listed,
            Err(why) => {
                reply.error(errno(why));
                return;
            }
        };
        // `.` and `..` are the protocol's rather than the tree's, so they are added here and
        // nowhere else. `..` of the root is the root, which is what a file system without a
        // parent says.
        let mut entries: Vec<(u64, FileType, String)> = vec![
            (ino.0, FileType::Directory, String::from(".")),
            (ino.0, FileType::Directory, String::from("..")),
        ];
        entries.extend(listed.into_iter().map(|node| {
            let kind = match node.kind {
                Kind::Directory => FileType::Directory,
                Kind::File => FileType::RegularFile,
            };
            let name = node
                .path
                .rsplit_once('/')
                .map_or(node.path.clone(), |(_, name)| name.to_owned());
            (node.ino, kind, name)
        }));
        for (index, (child, kind, name)) in entries
            .into_iter()
            .enumerate()
            .skip(usize::try_from(offset).unwrap_or(usize::MAX))
        {
            // The offset a kernel sends back is the one *after* this entry, which is why it is
            // the index plus one; an off-by-one here repeats or skips a name on a large listing.
            let next = u64::try_from(index).unwrap_or(u64::MAX).saturating_add(1);
            if reply.add(INodeNo(child), next, kind, name) {
                break;
            }
        }
        reply.ok();
    }

    fn create(
        &self,
        _request: &Request,
        parent: INodeNo,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        let Some(name) = text(name) else {
            reply.error(KernelErrno::EINVAL);
            return;
        };
        match self.face.create(parent.0, name) {
            Ok((node, handle)) => reply.created(
                &TIMEOUT,
                &self.attributes(&node),
                NEW,
                fuser::FileHandle(handle),
                FopenFlags::empty(),
            ),
            Err(why) => reply.error(errno(why)),
        }
    }

    fn write(
        &self,
        _request: &Request,
        _ino: INodeNo,
        fh: fuser::FileHandle,
        offset: u64,
        data: &[u8],
        _write_flags: fuser::WriteFlags,
        _flags: OpenFlags,
        _lock_owner: Option<fuser::LockOwner>,
        reply: ReplyWrite,
    ) {
        match self.face.write(fh.0, offset, data) {
            Ok(written) => reply.written(written),
            Err(why) => reply.error(errno(why)),
        }
    }

    /// The commit point, and the one reply whose error a program actually sees.
    fn flush(
        &self,
        _request: &Request,
        _ino: INodeNo,
        fh: fuser::FileHandle,
        _lock_owner: fuser::LockOwner,
        reply: ReplyEmpty,
    ) {
        match self.face.flush(fh.0) {
            Ok(_) => reply.ok(),
            Err(why) => reply.error(errno(why)),
        }
    }

    fn release(
        &self,
        _request: &Request,
        _ino: INodeNo,
        fh: fuser::FileHandle,
        _flags: OpenFlags,
        _lock_owner: Option<fuser::LockOwner>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        self.face.release(fh.0);
        reply.ok();
    }

    fn releasedir(
        &self,
        _request: &Request,
        _ino: INodeNo,
        _fh: fuser::FileHandle,
        _flags: OpenFlags,
        reply: ReplyEmpty,
    ) {
        reply.ok();
    }

    fn unlink(&self, _request: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        let Some(name) = text(name) else {
            reply.error(KernelErrno::ENOENT);
            return;
        };
        match self.face.unlink(parent.0, name) {
            Ok(_) => reply.ok(),
            Err(why) => reply.error(errno(why)),
        }
    }

    fn rmdir(&self, _request: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        let Some(name) = text(name) else {
            reply.error(KernelErrno::ENOENT);
            return;
        };
        // Removing a directory is removing part of the document's shape, which is the same
        // refusal `mkdir` gets and for the same reason.
        let _ = name;
        let _ = parent;
        reply.error(KernelErrno::EPERM);
    }

    fn mkdir(
        &self,
        _request: &Request,
        parent: INodeNo,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        let Some(name) = text(name) else {
            reply.error(KernelErrno::EINVAL);
            return;
        };
        reply.error(errno(self.face.mkdir(parent.0, name)));
    }

    fn rename(
        &self,
        _request: &Request,
        parent: INodeNo,
        name: &OsStr,
        newparent: INodeNo,
        newname: &OsStr,
        _flags: fuser::RenameFlags,
        reply: ReplyEmpty,
    ) {
        let (Some(name), Some(newname)) = (text(name), text(newname)) else {
            reply.error(KernelErrno::EINVAL);
            return;
        };
        reply.error(errno(self.face.rename(
            parent.0,
            name,
            newparent.0,
            newname,
        )));
    }

    /// What `df` says about a mount whose size is a document's.
    ///
    /// Zeros, which is what a virtual file system without a block device honestly has, and a
    /// name length of 255 because that is what the directory entries this tree generates are
    /// bounded by on every file system a mount point can be on.
    fn statfs(&self, _request: &Request, _ino: INodeNo, reply: ReplyStatfs) {
        reply.statfs(0, 0, 0, 0, 0, 512, 255, 512);
    }
}

#[cfg(test)]
mod tests {
    use super::{attributes, errno};
    use crate::Node;
    use pdf_vfs::Errno;
    use pdf_vfs::layout::Kind;
    use std::time::{Duration, UNIX_EPOCH};

    /// Every one of the core's refusals reaches the kernel as the number the core states.
    ///
    /// The list is exhaustive by construction — a variant added to [`pdf_vfs::Errno`] fails
    /// [`errno`]'s own `match` to compile — and the *value* is checked against
    /// `pdf_vfs::Errno::code`, which is the core's own statement of what each POSIX name is. So
    /// this holds the one thing the compiler cannot: that `EACCES` is not wired to `EPERM`.
    #[test]
    fn every_refusal_reaches_the_kernel_as_the_number_the_core_states() {
        let every = [
            Errno::OperationNotPermitted,
            Errno::NoSuchFile,
            Errno::InputOutput,
            Errno::PermissionDenied,
            Errno::Exists,
            Errno::NotADirectory,
            Errno::IsADirectory,
            Errno::Invalid,
            Errno::TooBig,
            Errno::ReadOnly,
            Errno::NotImplemented,
            Errno::Stale,
            Errno::Overflow,
        ];
        for one in every {
            assert_eq!(
                i32::from(errno(one)),
                one.code(),
                "{} is not the number the core states",
                one.as_str()
            );
        }
        // A mapping that had collapsed two variants onto one number would pass the loop above
        // only if the core stated the same number twice, which it does not.
        let mut numbers: Vec<i32> = every.iter().map(|one| i32::from(errno(*one))).collect();
        numbers.sort_unstable();
        let before = numbers.len();
        numbers.dedup();
        assert_eq!(numbers.len(), before, "two refusals share one number");
    }

    /// What `ls -l` shows: a page is read-only, an attachment is not, and a size is the file's.
    #[test]
    fn the_mode_says_what_the_core_says_a_write_would_mean() {
        let node = |kind, size, writable| Node {
            ino: 7,
            path: String::from("/x"),
            kind,
            size,
            writable,
        };
        let seen =
            |kind, size, writable| attributes(&node(kind, size, writable), (1001, 1002), None);
        assert_eq!(seen(Kind::File, Some(4097), false).perm, 0o444);
        assert_eq!(seen(Kind::File, Some(0), true).perm, 0o644);
        assert_eq!(seen(Kind::Directory, None, true).perm, 0o755);
        assert_eq!(seen(Kind::Directory, None, false).perm, 0o555);
        // The size a `stat` states is the file's own, and the block count rounds up — which is
        // what `du` reads and the only place this crate does arithmetic on a length.
        let file = seen(Kind::File, Some(4097), false);
        assert_eq!(file.size, 4097);
        assert_eq!(file.blocks, 9);
        assert_eq!(seen(Kind::Directory, None, true).size, 0);
        // The mount's own user and group, never root's. A mount by hand put `root root` on every
        // line of every listing of a document owned by uid 1001 (round 911, `owner`).
        assert_eq!((file.uid, file.gid), (1001, 1002));
    }

    /// The times a `stat` states are the document's, and the epoch only where it has none.
    ///
    /// The arithmetic is the one place a file system's clock reaches this crate, and it is the
    /// shape that has to hold: a nanosecond count split into whole seconds and a remainder, with
    /// nothing before the epoch and nothing past what `Duration` can hold.
    #[test]
    fn the_times_are_the_documents_own() {
        let at = |nanos| {
            attributes(
                &Node {
                    ino: 7,
                    path: String::from("/x"),
                    kind: Kind::File,
                    size: Some(1),
                    writable: false,
                },
                (0, 0),
                nanos,
            )
            .mtime
        };
        assert_eq!(
            at(None),
            UNIX_EPOCH,
            "a backing with no clock states the epoch"
        );
        assert_eq!(at(Some(0)), UNIX_EPOCH);
        assert_eq!(
            at(Some(1_500_000_000)),
            UNIX_EPOCH + Duration::new(1, 500_000_000),
            "the seconds and the remainder are split, not truncated"
        );
        assert_eq!(
            at(Some(-1)),
            UNIX_EPOCH,
            "a time before the epoch is the epoch rather than a wrap"
        );
        assert_eq!(
            at(Some(i128::MAX)),
            UNIX_EPOCH,
            "and one past what a `SystemTime` can hold is too"
        );
    }
}
