//! The C entry points, and the only `unsafe` in this crate.
//!
//! Every function here does the same three things and nothing else: turn raw arguments into safe
//! Rust values, call one safe function in a module beside this one, and write the result through
//! an out-parameter. **No decision is taken in this file** — not about a clause, not about which
//! `errno` a refusal is, not about a sentence's wording — so that the audit a reviewer owes is
//! "does this handle pointers correctly" and never "is this the right answer". `viewer-ffi`'s
//! `abi` module is the precedent and this follows it deliberately rather than inventing a second
//! shape for the same boundary.
//!
//! # The pointer contract, stated once
//!
//! It is the same for every function and is not repeated in each `# Safety` section beyond a
//! pointer back here:
//!
//! - every pointer is either null or valid for the type it names, aligned, and pointing at a live
//!   object this library produced (`pdfvfs_mount_open`, `pdfvfs_list`, `pdfvfs_open`, …);
//! - **null is always checked** and answers [`Status::NullArgument`]. It is the one bad pointer
//!   this side can detect, and detecting it is worth doing precisely because it is the one a C
//!   caller produces by accident rather than by arithmetic;
//! - an owning handle is freed exactly once, with its own `_free`, and is not used afterwards.
//!   There are five: `pdfvfs_mount`, `pdfvfs_listing`, `pdfvfs_file`, `pdfvfs_commit` and
//!   `pdfvfs_refusal`;
//! - a buffer given for output is writable for the number of bytes stated beside it;
//! - a `const char *` argument is NUL-terminated and is UTF-8. A path that is not UTF-8 is
//!   refused rather than repaired: an invented replacement character in a name is a file the tree
//!   does not have, named quietly;
//! - **no handle may be used from two threads at once.** A `pdfvfs_file *` may be *moved* to
//!   another thread and read there; a `pdfvfs_mount *` may not be shared.
//!
//! # A refusal is an object, not a return value and not a slot
//!
//! Every entry point that the *tree* can refuse takes a `pdfvfs_refusal **`, and writes it
//! exactly when it answers `PDFVFS_REFUSED`. [`crate::refusal`] has the argument; what matters
//! here is the discipline: the pointer is written on that status and on no other, so a caller
//! that frees it unconditionally is freeing null, and a caller that reads it after a `PDFVFS_OK`
//! is reading whatever it initialised it to.
//!
//! # Why `unsafe fn` and why `unsafe_op_in_unsafe_fn` is lifted here
//!
//! Every entry point is `pub unsafe extern "C" fn`. C does not see the word — the symbol and the
//! calling convention are identical — and Rust does: a function with preconditions a compiler
//! cannot check says so in its signature. A safe `extern "C" fn` that dereferenced its arguments
//! would be an unsound API for any Rust caller, and this crate is an `rlib` as well as a
//! `cdylib`.
//!
//! `unsafe_op_in_unsafe_fn` is lifted for this module because these bodies are *entirely* the
//! unsafe operation, exactly as in `viewer-ffi`. The position that replaces it is what
//! `tests/unsafe_position.rs` enforces: one `unsafe` token per entry point, in the signature, and
//! none anywhere else in the crate.
//!
//! # Panics
//!
//! None of these panics. An `extern "C"` function that unwound would abort the process, so every
//! fallible step returns a [`Status`] and every index is checked before it is used.

// See the module documentation. The lift is deliberate, narrow to this file, and the position it
// gives up is replaced by a test that reads the sources back.
#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::{c_char, c_int};

use crate::refusal::{self, Refusal};
use crate::status::Status;
use crate::tree::{self, Attributes, Commit, File, Listing, Mount};

/// The revision of everything in this header that a caller compiles against **by value**.
///
/// There is exactly one such shape — [`PdfvfsAttributes`] — and nothing else needs this number: a
/// function added later is a symbol an old caller never looks up, and a status, a kind or an
/// `errno` added later is a number an old caller has a `default:` arm for. A field added to that
/// struct changes a size the caller has already compiled, and no diagnostic anywhere would catch
/// it. This number moves when that happens and at no other time.
pub const PDFVFS_ABI_VERSION: u32 = 1;

/// What a `stat` answers.
///
/// Passed by pointer and copied out by value, which is why [`PDFVFS_ABI_VERSION`] exists.
///
/// **Named `pdfvfs_attributes` in the header** rather than `pdfvfs_stat`, because C puts a struct
/// tag and a function in one namespace and `pdfvfs_stat` is a function — the mistake
/// `viewer-ffi`'s own C driver found twice, recorded here so it is not found a third time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct PdfvfsAttributes {
    /// `PDFVFS_KIND_DIRECTORY` or `PDFVFS_KIND_FILE`.
    pub kind: u32,
    /// Whether [`Self::size`] means anything: one for a file, zero for a directory.
    pub has_size: u32,
    /// The file's **true** size in bytes, never an estimate — RFC 0003 section 5.5's rule, and
    /// the reason a `stat` generates the file.
    pub size: u64,
}

// ---------------------------------------------------------------------------------------------
// The identity of this ABI.
// ---------------------------------------------------------------------------------------------

/// The version of the by-value shapes this library was built with.
///
/// # Safety
///
/// None: it takes nothing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_abi_version() -> u32 {
    PDFVFS_ABI_VERSION
}

/// How many `errno` kinds the core states.
///
/// **This is what stands in for the Rust rule that a new refusal fails to compile in every
/// consumer.** See [`crate::refusal::KIND_COUNT`].
///
/// # Safety
///
/// None: it takes nothing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_errno_kind_count() -> u32 {
    refusal::KIND_COUNT
}

/// Compares the header a caller compiled against with the library it is running against.
///
/// `PDFVFS_OK` when both agree. Call it in `main`, or in the plugin's own entry point, and refuse
/// to start otherwise: a C caller cannot be made to fail its build when a number moves, so it
/// fails its startup instead, once, saying which number moved.
///
/// # Safety
///
/// None: it takes two numbers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_abi_check(version: u32, errno_kinds: u32) -> c_int {
    if version == PDFVFS_ABI_VERSION && errno_kinds == refusal::KIND_COUNT {
        Status::Ok.code()
    } else {
        Status::NumberOutOfRange.code()
    }
}

/// One sentence for a status, NUL-terminated and never freed.
///
/// A status this build does not define answers a sentence saying so rather than null, which is
/// trap 5 in the form C leaves available.
///
/// # Safety
///
/// None: it takes a number. The pointer it answers is static and outlives every call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_status_message(status: c_int) -> *const c_char {
    let message = match status {
        0 => Status::Ok.message(),
        1 => Status::NullArgument.message(),
        2 => Status::OutOfRange.message(),
        3 => Status::BufferTooSmall.message(),
        4 => Status::NotUtf8.message(),
        5 => Status::Refused.message(),
        6 => Status::NoAnswer.message(),
        7 => Status::NoDocument.message(),
        8 => Status::NumberOutOfRange.message(),
        _ => "a status this build of the library does not name\0",
    };
    message.as_ptr().cast::<c_char>()
}

/// The name of an `errno` — `EPERM`, `ENOENT`, … — for every number, known or not.
///
/// # Safety
///
/// None: it takes a number. The pointer it answers is static and outlives every call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_errno_name(code: c_int) -> *const c_char {
    refusal::name_of(code).as_ptr().cast::<c_char>()
}

/// The name of the confined generator this library spawns, without a platform suffix.
///
/// RFC 0003 section 6: the face holds no parser, and every question that needs one is answered by
/// a separate process under seccomp-BPF and Landlock. A caller that is not installed beside that
/// program says so with this name and [`pdfvfs_worker_variable`], rather than reporting a
/// document that will not open.
///
/// # Safety
///
/// None. The pointer it answers is static and outlives every call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_worker_program() -> *const c_char {
    WORKER_PROGRAM.as_ptr().cast::<c_char>()
}

/// The environment variable that names the confined generator explicitly.
///
/// # Safety
///
/// None. The pointer it answers is static and outlives every call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_worker_variable() -> *const c_char {
    WORKER_VARIABLE.as_ptr().cast::<c_char>()
}

/// [`pdf_vfs::WORKER_PROGRAM`] with the NUL a `const char *` needs.
///
/// Written as a literal beside a test that compares it with the core's, rather than built at run
/// time: a `String` here would be an allocation on a path that cannot fail.
const WORKER_PROGRAM: &str = "pdf-vfs-worker\0";

/// [`pdf_vfs::WORKER_PATH_VARIABLE`], the same way.
const WORKER_VARIABLE: &str = "PDF_VFS_WORKER\0";

// ---------------------------------------------------------------------------------------------
// Where the document ends and the tree begins.
// ---------------------------------------------------------------------------------------------

/// The length of the prefix of `url_path` that names the document.
///
/// The tree inside it is the rest of the string; an empty rest is the root. `PDFVFS_NO_DOCUMENT`
/// where no prefix is a file. See [`crate::tree::split`] for why the file system decides this.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_split(
    url_path: *const c_char,
    document_length: *mut usize,
) -> c_int {
    let path = match borrowed_text(url_path) {
        Ok(Some(path)) => path,
        Ok(None) => return Status::NullArgument.code(),
        Err(()) => return Status::NotUtf8.code(),
    };
    let Some(length) = tree::split(&path) else {
        return Status::NoDocument.code();
    };
    let Some(out) = document_length.as_mut() else {
        return Status::NullArgument.code();
    };
    *out = length;
    Status::Ok.code()
}

// ---------------------------------------------------------------------------------------------
// A refusal, owned by the caller.
// ---------------------------------------------------------------------------------------------

/// The `errno` this refusal is, as Linux numbers it.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_refusal_errno(why: *const Refusal, out: *mut c_int) -> c_int {
    let (Some(why), Some(out)) = (why.as_ref(), out.as_mut()) else {
        return Status::NullArgument.code();
    };
    *out = why.code();
    Status::Ok.code()
}

/// The sentence this refusal carries, in the two-call idiom.
///
/// RFC 0003 section 5.3 requires it to exist; a KIO worker hands it to `KIO::WorkerResult::fail`
/// and it is what Dolphin shows a person.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_refusal_message(
    why: *const Refusal,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(why) = why.as_ref() else {
        return Status::NullArgument.code();
    };
    copy_out(why.sentence(), out, cap, needed)
}

/// Releases a refusal. Freeing null is nothing.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_refusal_free(why: *mut Refusal) {
    if !why.is_null() {
        drop(Box::from_raw(why));
    }
}

// ---------------------------------------------------------------------------------------------
// The mount.
// ---------------------------------------------------------------------------------------------

/// Opens a document as a tree, at one of the four restriction levels.
///
/// `restrictions` is `PDFVFS_RESTRICT_OFF`, `_ON`, `_ASK` or `_WARN`; a number that is none of
/// them is refused rather than rounded. `CLAUDE.md` principle 3 asks the policy **once, in a
/// place a host can supply**, and this argument is that place for this face.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_mount_open(
    document: *const c_char,
    restrictions: u32,
    out: *mut *mut Mount,
    why: *mut *mut Refusal,
) -> c_int {
    let path = match borrowed_text(document) {
        Ok(Some(path)) => path,
        Ok(None) => return Status::NullArgument.code(),
        Err(()) => return Status::NotUtf8.code(),
    };
    let Some(out) = out.as_mut() else {
        return Status::NullArgument.code();
    };
    match Mount::open(&path, restrictions) {
        Ok(mount) => {
            *out = Box::into_raw(Box::new(mount));
            Status::Ok.code()
        }
        Err(refusal) => refused(refusal, why),
    }
}

/// Releases a mount and everything it holds, including its confined generator. Freeing null is
/// nothing.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_mount_free(mount: *mut Mount) {
    if !mount.is_null() {
        drop(Box::from_raw(mount));
    }
}

/// How many pages ISO 32000-2 §7.7.3.2's tree holds.
///
/// **The first call that reads the document**, which is where a password, a confined generator
/// that will not start, or a file that is not a PDF is first reported.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_mount_pages(
    mount: *mut Mount,
    out: *mut u64,
    why: *mut *mut Refusal,
) -> c_int {
    let (Some(mount), Some(out)) = (mount.as_ref(), out.as_mut()) else {
        return Status::NullArgument.code();
    };
    match mount.pages() {
        Ok(pages) => {
            *out = pages;
            Status::Ok.code()
        }
        Err(refusal) => refused(refusal, why),
    }
}

/// How many things the layout declares that this build does not do.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_mount_shortfall_count(mount: *mut Mount, out: *mut usize) -> c_int {
    let (Some(mount), Some(out)) = (mount.as_ref(), out.as_mut()) else {
        return Status::NullArgument.code();
    };
    *out = mount.shortfalls().len();
    Status::Ok.code()
}

/// One shortfall's sentence, in the two-call idiom.
///
/// Trap 5 across a boundary: a face prints these at start-up rather than a person discovering
/// them.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_mount_shortfall(
    mount: *mut Mount,
    index: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(mount) = mount.as_ref() else {
        return Status::NullArgument.code();
    };
    let shortfalls = mount.shortfalls();
    let Some(sentence) = shortfalls.get(index) else {
        return Status::OutOfRange.code();
    };
    copy_out(sentence, out, cap, needed)
}

// ---------------------------------------------------------------------------------------------
// Reads: RFC 0003 section 5.1.
// ---------------------------------------------------------------------------------------------

/// A directory's entries, as an owned batch the caller frees.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_list(
    mount: *mut Mount,
    path: *const c_char,
    out: *mut *mut Listing,
    why: *mut *mut Refusal,
) -> c_int {
    let inside = match borrowed_text(path) {
        Ok(Some(inside)) => inside,
        Ok(None) => return Status::NullArgument.code(),
        Err(()) => return Status::NotUtf8.code(),
    };
    let (Some(mount), Some(out)) = (mount.as_ref(), out.as_mut()) else {
        return Status::NullArgument.code();
    };
    match mount.list(&inside) {
        Ok(listing) => {
            *out = Box::into_raw(Box::new(listing));
            Status::Ok.code()
        }
        Err(refusal) => refused(refusal, why),
    }
}

/// How many entries a listing holds.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_listing_len(listing: *const Listing, out: *mut usize) -> c_int {
    let (Some(listing), Some(out)) = (listing.as_ref(), out.as_mut()) else {
        return Status::NullArgument.code();
    };
    *out = listing.len();
    Status::Ok.code()
}

/// One entry's name, in the two-call idiom. Never holds a solidus.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_listing_name(
    listing: *const Listing,
    index: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(listing) = listing.as_ref() else {
        return Status::NullArgument.code();
    };
    match listing.name(index) {
        Ok(name) => copy_out(name, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// One entry's kind: `PDFVFS_KIND_DIRECTORY` or `PDFVFS_KIND_FILE`.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_listing_kind(
    listing: *const Listing,
    index: usize,
    out: *mut u32,
) -> c_int {
    let (Some(listing), Some(out)) = (listing.as_ref(), out.as_mut()) else {
        return Status::NullArgument.code();
    };
    match listing.kind(index) {
        Ok(kind) => {
            *out = kind;
            Status::Ok.code()
        }
        Err(status) => status.code(),
    }
}

/// Releases a listing. Freeing null is nothing.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_listing_free(listing: *mut Listing) {
    if !listing.is_null() {
        drop(Box::from_raw(listing));
    }
}

/// One path's kind and true size.
///
/// **This generates the file**, which is RFC 0003 section 5.5's rule and not an implementation
/// detail: "an under-estimate silently truncates a page", so no virtual file is stat'd before it
/// exists. A listing is cheap; a `stat` is where the work lands.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_stat(
    mount: *mut Mount,
    path: *const c_char,
    out: *mut PdfvfsAttributes,
    why: *mut *mut Refusal,
) -> c_int {
    let inside = match borrowed_text(path) {
        Ok(Some(inside)) => inside,
        Ok(None) => return Status::NullArgument.code(),
        Err(()) => return Status::NotUtf8.code(),
    };
    let (Some(mount), Some(out)) = (mount.as_ref(), out.as_mut()) else {
        return Status::NullArgument.code();
    };
    match mount.stat(&inside) {
        Ok(Attributes { kind, size }) => {
            *out = PdfvfsAttributes {
                kind,
                has_size: u32::from(size.is_some()),
                size: size.unwrap_or(0),
            };
            Status::Ok.code()
        }
        Err(refusal) => refused(refusal, why),
    }
}

/// What writing to and deleting this path would each mean, as `PDFVFS_MEANS_*`.
///
/// `PDFVFS_NO_ANSWER` where the layout names no row at all, which is a fair question about a path
/// this tree does not have. **The core decides this**, so the mode bits a file manager shows are
/// the document's own shape rather than a list a face keeps.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_write_meaning(
    mount: *mut Mount,
    path: *const c_char,
    on_write: *mut u32,
    on_delete: *mut u32,
) -> c_int {
    let inside = match borrowed_text(path) {
        Ok(Some(inside)) => inside,
        Ok(None) => return Status::NullArgument.code(),
        Err(()) => return Status::NotUtf8.code(),
    };
    let (Some(mount), Some(write), Some(delete)) =
        (mount.as_ref(), on_write.as_mut(), on_delete.as_mut())
    else {
        return Status::NullArgument.code();
    };
    let Some((writing, deleting)) = mount.write_meaning(&inside) else {
        return Status::NoAnswer.code();
    };
    *write = writing;
    *delete = deleting;
    Status::Ok.code()
}

/// Opens a file, materialising its bytes at the generation it was opened under.
///
/// RFC 0003 section 5.4: an open file keeps that generation, so no reader ever receives a splice
/// of two. The handle may be moved to another thread and read there.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_open(
    mount: *mut Mount,
    path: *const c_char,
    out: *mut *mut File,
    why: *mut *mut Refusal,
) -> c_int {
    let inside = match borrowed_text(path) {
        Ok(Some(inside)) => inside,
        Ok(None) => return Status::NullArgument.code(),
        Err(()) => return Status::NotUtf8.code(),
    };
    let (Some(mount), Some(out)) = (mount.as_ref(), out.as_mut()) else {
        return Status::NullArgument.code();
    };
    match mount.open_file(&inside) {
        Ok(file) => {
            *out = Box::into_raw(Box::new(file));
            Status::Ok.code()
        }
        Err(refusal) => refused(refusal, why),
    }
}

/// How many bytes an open file holds.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_file_size(file: *const File, out: *mut u64) -> c_int {
    let (Some(file), Some(out)) = (file.as_ref(), out.as_mut()) else {
        return Status::NullArgument.code();
    };
    *out = file.len();
    Status::Ok.code()
}

/// Copies up to `capacity` bytes from `offset` into the caller's buffer.
///
/// Short at the end and empty past it, which is what `read(2)` does. **Pixels and bytes reach the
/// caller by copy into a buffer it owns**: no pointer into this library's memory is ever handed
/// out, so there is no lifetime for a C program to get wrong.
///
/// # Safety
///
/// See the module documentation. `buffer` is writable for `capacity` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_file_read(
    file: *const File,
    offset: u64,
    buffer: *mut u8,
    capacity: usize,
    filled: *mut usize,
) -> c_int {
    let (Some(file), Some(filled)) = (file.as_ref(), filled.as_mut()) else {
        return Status::NullArgument.code();
    };
    if buffer.is_null() && capacity != 0 {
        return Status::NullArgument.code();
    }
    let bytes = file.read(offset, capacity);
    *filled = bytes.len();
    if bytes.is_empty() {
        return Status::Ok.code();
    }
    let room = core::slice::from_raw_parts_mut(buffer, bytes.len());
    room.copy_from_slice(bytes);
    Status::Ok.code()
}

/// Releases an open file. Freeing null is nothing.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_file_free(file: *mut File) {
    if !file.is_null() {
        drop(Box::from_raw(file));
    }
}

// ---------------------------------------------------------------------------------------------
// Writes: RFC 0003 section 5.2, and the refusals of section 5.3.
// ---------------------------------------------------------------------------------------------

/// Writes a whole file into the tree, as one transaction.
///
/// This is RFC 0003 section 5.2's five verbs, whichever of them the path names: a PDF written to
/// `pages/NNNN.pdf` inserts its pages there, a file written to `attachments/NAME` embeds it,
/// `meta/info.json` sets §14.3.3's entries. The bytes arrive whole because **KIO's own verb is
/// transactional** — section 5.4 says so — and the staged four a kernel needs are the FUSE face's.
///
/// # Safety
///
/// See the module documentation. `bytes` is readable for `length` bytes, or null when `length` is
/// zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_write(
    mount: *mut Mount,
    path: *const c_char,
    bytes: *const u8,
    length: usize,
    out: *mut *mut Commit,
    why: *mut *mut Refusal,
) -> c_int {
    let inside = match borrowed_text(path) {
        Ok(Some(inside)) => inside,
        Ok(None) => return Status::NullArgument.code(),
        Err(()) => return Status::NotUtf8.code(),
    };
    let (Some(mount), Some(out)) = (mount.as_ref(), out.as_mut()) else {
        return Status::NullArgument.code();
    };
    if bytes.is_null() && length != 0 {
        return Status::NullArgument.code();
    }
    let written: &[u8] = if length == 0 {
        &[]
    } else {
        core::slice::from_raw_parts(bytes, length)
    };
    match mount.write(&inside, written) {
        Ok(commit) => {
            *out = Box::into_raw(Box::new(commit));
            Status::Ok.code()
        }
        Err(refusal) => refused(refusal, why),
    }
}

/// Removes a name: a page out of §7.7.3.2's tree, or an embedded file out of §7.7.4's.
///
/// §7.5.6 does not destroy bytes, and the commit's warnings say so where it applies.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_remove(
    mount: *mut Mount,
    path: *const c_char,
    out: *mut *mut Commit,
    why: *mut *mut Refusal,
) -> c_int {
    let inside = match borrowed_text(path) {
        Ok(Some(inside)) => inside,
        Ok(None) => return Status::NullArgument.code(),
        Err(()) => return Status::NotUtf8.code(),
    };
    let (Some(mount), Some(out)) = (mount.as_ref(), out.as_mut()) else {
        return Status::NullArgument.code();
    };
    match mount.remove(&inside) {
        Ok(commit) => {
            *out = Box::into_raw(Box::new(commit));
            Status::Ok.code()
        }
        Err(refusal) => refused(refusal, why),
    }
}

/// How many pages the document has after this commit, which is what a renumbered listing shows.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_commit_pages(commit: *const Commit, out: *mut u64) -> c_int {
    let (Some(commit), Some(out)) = (commit.as_ref(), out.as_mut()) else {
        return Status::NullArgument.code();
    };
    *out = commit.pages();
    Status::Ok.code()
}

/// How many warnings this commit produced.
///
/// **`CLAUDE.md` principle 3's *warn* level arrives here**: at that level the operation proceeds
/// and the document's reasons are said afterwards, and this is where a face collects them to show
/// a person. §7.5.6's own note — a deletion does not destroy bytes — arrives the same way.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_commit_warning_count(
    commit: *const Commit,
    out: *mut usize,
) -> c_int {
    let (Some(commit), Some(out)) = (commit.as_ref(), out.as_mut()) else {
        return Status::NullArgument.code();
    };
    *out = commit.warnings();
    Status::Ok.code()
}

/// One warning's sentence, in the two-call idiom.
///
/// # Safety
///
/// See the module documentation. `out` is writable for `cap` bytes, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_commit_warning(
    commit: *const Commit,
    index: usize,
    out: *mut c_char,
    cap: usize,
    needed: *mut usize,
) -> c_int {
    let Some(commit) = commit.as_ref() else {
        return Status::NullArgument.code();
    };
    match commit.warning(index) {
        Ok(warning) => copy_out(warning, out, cap, needed),
        Err(status) => status.code(),
    }
}

/// Releases a commit. Freeing null is nothing.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_commit_free(commit: *mut Commit) {
    if !commit.is_null() {
        drop(Box::from_raw(commit));
    }
}

/// Renaming, which RFC 0003 section 5.3 refuses in v1 whatever it names.
///
/// **Always `PDFVFS_REFUSED`**, and the sentence is the core's: rename semantics under
/// position-names are ambiguous, and a file manager's drag-reorder emits rename storms this tree
/// cannot make atomic. A face that answered this itself would be a second copy of the decision.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_rename(
    mount: *mut Mount,
    from: *const c_char,
    to: *const c_char,
    why: *mut *mut Refusal,
) -> c_int {
    let (Ok(Some(from)), Ok(Some(to))) = (borrowed_text(from), borrowed_text(to)) else {
        return Status::NullArgument.code();
    };
    let Some(mount) = mount.as_ref() else {
        return Status::NullArgument.code();
    };
    refused(mount.rename(&from, &to), why)
}

/// Creating a directory, which the core refuses: every directory here is the document's own
/// shape.
///
/// **Always `PDFVFS_REFUSED`**, with the core's sentence.
///
/// # Safety
///
/// See the module documentation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfvfs_create_directory(
    mount: *mut Mount,
    path: *const c_char,
    why: *mut *mut Refusal,
) -> c_int {
    let Ok(Some(inside)) = borrowed_text(path) else {
        return Status::NullArgument.code();
    };
    let Some(mount) = mount.as_ref() else {
        return Status::NullArgument.code();
    };
    refused(mount.create_directory(&inside), why)
}

// ---------------------------------------------------------------------------------------------
// The three helpers every entry point above shares. None is exported.
// ---------------------------------------------------------------------------------------------

/// A NUL-terminated argument as an owned `String`, or `None` for a null pointer.
///
/// Owned rather than borrowed for the same reason `viewer-ffi` owns one: what it becomes is a
/// path the core canonicalises and keeps, and a borrow would tie its lifetime to a buffer the
/// caller may reuse the moment the call returns.
///
/// # Errors
///
/// The bytes were not UTF-8, which is refused rather than repaired.
///
/// # Safety
///
/// `text` is null or points at a NUL-terminated sequence of bytes.
unsafe fn borrowed_text(text: *const c_char) -> Result<Option<String>, ()> {
    if text.is_null() {
        return Ok(None);
    }
    core::ffi::CStr::from_ptr(text)
        .to_str()
        .map(|text| Some(text.to_owned()))
        .map_err(|_| ())
}

/// Writes a string and its terminating NUL into a caller's buffer.
///
/// The second half of C's two-call idiom, in one place so that every string-valued entry point
/// spells it the same way. `needed` counts the NUL, so a caller that allocates exactly that many
/// bytes succeeds on the second call. **Nothing is written unless the whole string fits**, which
/// is what keeps a truncated sentence from looking like a short one.
///
/// # Safety
///
/// `out` is null or writable for `cap` bytes; `needed` is null or writable.
unsafe fn copy_out(text: &str, out: *mut c_char, cap: usize, needed: *mut usize) -> c_int {
    let wanted = text.len().saturating_add(1);
    if let Some(needed) = needed.as_mut() {
        *needed = wanted;
    }
    if out.is_null() || cap < wanted {
        return Status::BufferTooSmall.code();
    }
    let room = core::slice::from_raw_parts_mut(out.cast::<u8>(), wanted);
    let Some(body) = room.get_mut(..text.len()) else {
        // Unreachable: `wanted` is one more than `text.len()`. Written as a refusal rather than
        // an index so that the slice above is the only place a length is trusted.
        return Status::BufferTooSmall.code();
    };
    body.copy_from_slice(text.as_bytes());
    if let Some(last) = room.last_mut() {
        *last = 0;
    }
    Status::Ok.code()
}

/// Hands a refusal to the caller and answers `PDFVFS_REFUSED`.
///
/// **The one place the out-parameter is written**, so that the discipline the module
/// documentation states — written on this status and on no other — is a property of the code
/// rather than of thirteen separate arms. A caller that passed null for it gets the status and
/// loses the sentence, which is its own choice to make.
///
/// # Safety
///
/// `why` is null or writable.
unsafe fn refused(refusal: Refusal, why: *mut *mut Refusal) -> c_int {
    if let Some(slot) = why.as_mut() {
        *slot = Box::into_raw(Box::new(refusal));
    }
    Status::Refused.code()
}
