//! `pdf-vfs`'s tree, spelled in C — the Rust half of RFC 0003's **KIO face**.
//!
//! # Why a C ABI at all, when the other face is a Rust binary
//!
//! RFC 0003 section 7 states the constraint rather than choosing it: "KF6 admits no Rust worker —
//! no binding exists (the one experiment, cxx-kde-frameworks, was archived in 2024 at 19
//! commits), and `WorkerBase` is a C++ class". So the face that Dolphin loads is a C++ `MODULE`
//! plugin, and what carries every one of its operations into this tree is "a **C ABI** — the
//! `viewer-ffi` precedent, which is exactly the boundary this tree already knows how to freeze
//! and test". This crate is that boundary. `kio/` beside the workspace is the plugin; it holds no
//! PDF logic, no layout knowledge and no `errno` of its own.
//!
//! The division RFC 0003 section 7 draws is kept literally: **the shim owns the Qt types**
//! (`QUrl`, `UDSEntry`, `KIO::WorkerResult`) **and the core never sees them.** Nothing in this
//! crate names Qt, links Qt, or knows that KIO exists; a second C++ face — or a C program, or
//! Python through `ctypes` — reaches the same thirty-five functions.
//!
//! # The four questions a C boundary has to answer, and this one's answers
//!
//! ## 1. A verb is a function, not a tagged union
//!
//! `pdfvfs_list`, `pdfvfs_stat`, `pdfvfs_open`, `pdfvfs_write`: one entry point per operation,
//! taking that operation's own arguments. `viewer-ffi`'s argument applies unchanged and is worth
//! restating because it is entirely about C — **a union's size is part of the ABI**, so a verb
//! added later would change the size of a type every caller has already compiled, and an old
//! caller passing an old-sized struct to a new library is undefined behaviour no diagnostic
//! catches. A function added later is a symbol an old caller never looks up.
//!
//! ## 2. An answer arrives owned, in a batch the caller frees
//!
//! [`pdf_vfs::Vfs::list`] answers a `Vec`, [`pdf_vfs::Vfs::open`] a [`pdf_vfs::Handle`] holding
//! an `Arc<[u8]>`, and neither survives a C boundary as a borrow: a caller holding one while it
//! calls back into the mount is the aliasing hazard nothing on this side would notice. So a
//! listing, an open file, a commit and a refusal are each an **owned handle** released with its
//! own `_free`, and bytes reach the caller by **copy into a buffer the caller owns**. No pointer
//! into this library's memory is ever handed out.
//!
//! ## 3. A refusal is a number *and* a sentence, and both are the core's
//!
//! This is the requirement RFC 0003 section 5.3 puts on a face, and the one KIO is *good* at.
//! Every refusal carries [`pdf_vfs::Errno`] — "a KIO worker mapping these onto `KIO::Error` and a
//! FUSE daemon handing them to the kernel must agree about what a refused write *is*" — and the
//! sentence that says why. FUSE has nowhere to put the second of those and logs it; KIO hands it
//! to `KIO::WorkerResult::fail` and Dolphin shows it to a person. That asymmetry is the whole
//! reason this face is worth building, and it is why a refusal here is an object rather than an
//! `errno` alone ([`refusal`]).
//!
//! ## 4. A kind added later, and what it costs a compiled C caller
//!
//! The question the FUSE face never had to answer. An `errno` kind is a number a caller switches
//! on and a kind it does not know cannot be made to fail its build — so every number answers
//! [`abi::pdfvfs_errno_name`], *including* ones this build has never heard of, and the count is
//! checkable at startup: the header states `PDFVFS_ERRNO_KIND_COUNT` as it was when the caller
//! was compiled, the library answers `pdfvfs_errno_kind_count()`, and `pdfvfs_abi_check` compares
//! them with the version. That converts "fails to compile in every consumer" into "**fails to
//! start, once, saying which number moved**". It is weaker, and it is the strongest thing
//! available.
//!
//! What it costs, plainly: a new `errno` kind costs a compiled caller *nothing* until it meets
//! one, and then costs it a `default:` arm. A new verb costs it nothing. A field added to
//! [`abi::PdfvfsAttributes`] — the one struct passed by value — costs it a recompilation it has
//! no way of knowing it needs, which is why that struct is small, is output-only, and is what
//! [`abi::PDFVFS_ABI_VERSION`] is about.
//!
//! # What this boundary deliberately does *not* state
//!
//! **The staged four.** `create`, `write_at`, `flush` and `release` are what a *kernel* hands a
//! file system one call at a time, and RFC 0003 section 5.4 makes `flush` the commit point for
//! that reason. KIO's `put` is not that shape — the section says so in the same breath: "a KIO
//! `put` commits when the worker's `put` completes (KIO's verb is already transactional)" — so
//! this boundary states [`abi::pdfvfs_write`], one call with the whole file, and the staged four
//! stay in Rust where the FUSE face reaches them. Four entry points nobody would call would be
//! four more shapes to keep, and a C caller that used them would be inventing a transaction the
//! protocol above it does not have.
//!
//! # Where the `unsafe` is
//!
//! One module, [`abi`], and `tests/unsafe_position.rs` counts its tokens and asserts that
//! everything else in the crate is safe Rust — the position `viewer-qt` established and
//! `viewer-ffi` follows. **There is no PDF parsing behind an `unsafe` and none in this process
//! at all**: RFC 0003 section 6 puts every byte of it in a confined generator, and this crate
//! holds paths, verbs and sentences.

#![deny(unsafe_code)]
#![warn(missing_docs)]

// The only place in this crate the permission is used, and `tests/unsafe_position.rs` is what
// keeps that true. Everything it calls is safe Rust in the modules beside it.
#[allow(unsafe_code)]
pub mod abi;

pub mod refusal;
pub mod status;
pub mod tree;

pub use refusal::Refusal;
pub use status::Status;
pub use tree::{Attributes, Commit, File, Listing, Mount};
