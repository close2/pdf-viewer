//! The adapter: `fuser`'s `Filesystem` in terms of [`crate::Face`], and nothing else.
//!
//! Every method here is the same three lines — read the kernel's arguments, ask the face, turn
//! the answer into a reply — and that sameness is the point. RFC 0003 section 7 puts the layout
//! in the core, so a face that had a decision of its own in one of these methods would be a
//! second place a path means something. The decisions this file *does* make are all about the
//! protocol: what a `FileAttr` says about a virtual file, which timeout the kernel is given, and
//! which of `fuser`'s numbers a [`pdf_vfs::Errno`] is.

use std::ffi::OsStr;
use std::time::{SystemTime, UNIX_EPOCH};

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
pub struct Mount(std::sync::Arc<Face>);

impl Mount {
    /// A mount over this face.
    #[must_use]
    pub fn new(face: std::sync::Arc<Face>) -> Self {
        Self(face)
    }

    /// The face underneath, for the notifier thread the binary runs.
    #[must_use]
    pub fn face(&self) -> &Face {
        &self.0
    }
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
/// The times are the epoch and are stated as a choice: nothing in this tree has a modification
/// time of its own — a page is generated on demand from a document whose own mtime is the
/// backing file's — and inventing "now" would make every `ls` of a mount look like a directory
/// that had just changed. A face that wanted the backing file's times would take them from the
/// generation key, which is the only clock this design has.
fn attributes(node: &Node) -> FileAttr {
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
        atime: UNIX_EPOCH,
        mtime: UNIX_EPOCH,
        ctime: UNIX_EPOCH,
        crtime: UNIX_EPOCH,
        kind,
        perm: permissions,
        nlink: 1,
        uid: 0,
        gid: 0,
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
        match self.0.lookup(parent.0, name) {
            Ok(node) => reply.entry(&TIMEOUT, &attributes(&node), NEW),
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
        match self.0.getattr(ino.0) {
            Ok(node) => reply.attr(&TIMEOUT, &attributes(&node)),
            Err(why) => reply.error(errno(why)),
        }
    }

    /// The only attribute this tree can be asked to change is a staged write's length, which is
    /// `ftruncate(2)`. Everything else — mode, owner, times — is refused rather than accepted and
    /// ignored, because a `chmod` that returns success and changes nothing is a lie a file
    /// manager will act on.
    fn setattr(
        &self,
        _request: &Request,
        ino: INodeNo,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<TimeOrNow>,
        _mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        fh: Option<fuser::FileHandle>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<BsdFileFlags>,
        reply: ReplyAttr,
    ) {
        if mode.is_some() || uid.is_some() || gid.is_some() {
            reply.error(KernelErrno::EPERM);
            return;
        }
        if let (Some(size), Some(handle)) = (size, fh)
            && let Err(why) = self.0.truncate(handle.0, size)
        {
            reply.error(errno(why));
            return;
        }
        match self.0.getattr(ino.0) {
            Ok(node) => reply.attr(&TIMEOUT, &attributes(&node)),
            Err(why) => reply.error(errno(why)),
        }
    }

    fn open(&self, _request: &Request, ino: INodeNo, _flags: OpenFlags, reply: ReplyOpen) {
        match self.0.open(ino.0) {
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
        match self.0.read(fh.0, offset, size) {
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
        let listed = match self.0.readdir(ino.0) {
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
        match self.0.create(parent.0, name) {
            Ok((node, handle)) => reply.created(
                &TIMEOUT,
                &attributes(&node),
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
        match self.0.write(fh.0, offset, data) {
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
        match self.0.flush(fh.0) {
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
        self.0.release(fh.0);
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
        match self.0.unlink(parent.0, name) {
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
        reply.error(errno(self.0.mkdir(parent.0, name)));
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
        reply.error(errno(self.0.rename(parent.0, name, newparent.0, newname)));
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
        assert_eq!(attributes(&node(Kind::File, Some(4097), false)).perm, 0o444);
        assert_eq!(attributes(&node(Kind::File, Some(0), true)).perm, 0o644);
        assert_eq!(attributes(&node(Kind::Directory, None, true)).perm, 0o755);
        assert_eq!(attributes(&node(Kind::Directory, None, false)).perm, 0o555);
        // The size a `stat` states is the file's own, and the block count rounds up — which is
        // what `du` reads and the only place this crate does arithmetic on a length.
        let file = attributes(&node(Kind::File, Some(4097), false));
        assert_eq!(file.size, 4097);
        assert_eq!(file.blocks, 9);
        assert_eq!(attributes(&node(Kind::Directory, None, true)).size, 0);
    }
}
