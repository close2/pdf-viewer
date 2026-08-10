//! A lazy fetcher for the DARPA `SafeDocs` PDF corpora, and a survey that runs this tree over
//! what it fetched.
//!
//! # Why this is not a submodule
//!
//! `doc/pdf.js`, `doc/corpora/pdf20examples`, `doc/corpora/pdf-differences` and
//! `doc/corpora/pdfbox` are git repositories and are checked out as submodules. `SafeDocs` is
//! not one and cannot be: the `CC-MAIN-2021-31-PDF-UNTRUNCATED` corpus is **7 933 ZIP
//! archives of about a gigabyte each**, nearly eight million PDF files and close to 8 TB
//! uncompressed, distributed as plain objects through AWS Open Data and the Digital Corpora
//! project. Git degrades on a thousandth of that.
//!
//! # What this crate will and will not do
//!
//! **The constraint is that a large download must be impossible *by accident*, and it is not that
//! a large download is impossible.** The project owner stated it from a mobile connection and
//! then, in the same session, that a fibre connection with no limit is the usual case and a big
//! download is then perfectly fine. So this crate is built to make the size *known and asked
//! for*, never to decide for the person:
//!
//! - **The archive is addressed a member at a time, never as an object.** [`plan()`] resolves a
//!   chunk through the archive's *central directory*, which is tens of kilobytes at the end of
//!   the object, and [`fetch_range`] then asks for one contiguous byte range covering the members
//!   that chunk names. `--all` widens that range to every member, which on fibre is exactly the
//!   whole-archive download and is still one deliberate argument.
//! - **Nothing is transferred until an explicit argument says so.** The default is a plan
//!   printed to stdout: every member, its compressed and uncompressed size, and the total.
//! - **A plan larger than the byte budget is refused**, named in bytes and in the `--budget-mb`
//!   that would admit it, whether or not the download argument is present. The budget defaults to
//!   [`Budget::DEFAULT`] and has no ceiling.
//! - **What arrives is verified against what the archive says**, by uncompressed length and
//!   by the CRC-32 the central directory records, before it reaches the cache.
//!
//! # Where it caches
//!
//! `corpus-cache/safedocs/`, which `.gitignore` covers. A file promoted out of it into this
//! tree's own test set is a deliberate act with a budget; see `doc/adr/0258`.
//!
//! # The transport
//!
//! `curl`, run as a subprocess with an explicit `Range` header. This crate takes no HTTP
//! client and no TLS stack — see the manifest for the argument.

#![forbid(unsafe_code)]

pub mod cache;
pub mod plan;
pub mod source;
pub mod survey;
pub mod transport;
pub mod zip;

pub use cache::{Cache, Fetched};
pub use plan::{Budget, Chunk, Plan, plan};
pub use source::{Corpus, KNOWN};
pub use transport::fetch_range;

/// Everything that can go wrong between naming a chunk and having its files on disk.
///
/// One enum for the crate: a caller of a command-line tool wants one sentence, and the
/// variants are what that sentence can be.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// No corpus is registered under that name.
    #[error("no such corpus: {0:?} (try `safedocs corpora`)")]
    NoSuchCorpus(String),
    /// The archive name does not fit the corpus's own naming scheme.
    #[error("{0}")]
    NoSuchArchive(String),
    /// `curl` could not be started at all.
    #[error("the transport is unavailable: `curl` could not be run ({0})")]
    NoTransport(std::io::Error),
    /// `curl` ran and failed.
    #[error("the transport failed for {url} bytes {first}-{last}: {reason}")]
    Transport {
        /// What was being read.
        url: String,
        /// First byte of the requested range, inclusive.
        first: u64,
        /// Last byte of the requested range, inclusive.
        last: u64,
        /// What `curl` said, or the exit status when it said nothing.
        reason: String,
    },
    /// The server answered with a different number of bytes than were asked for.
    #[error("{url}: asked for {want} bytes and received {got}")]
    ShortRange {
        /// What was being read.
        url: String,
        /// Bytes requested.
        want: u64,
        /// Bytes received.
        got: usize,
    },
    /// The archive's own structure could not be read.
    #[error("{0}")]
    Archive(#[from] zip::ArchiveError),
    /// The plan is larger than the budget allows.
    ///
    /// This is the owner's constraint made structural, and it fires before any member's bytes
    /// are requested.
    #[error(
        "this chunk would transfer {wanted} bytes and the budget is {budget}. \
         Narrow it with --count, or ask for it deliberately: --budget-mb {needed}"
    )]
    OverBudget {
        /// What the plan would transfer.
        wanted: u64,
        /// What the budget permits.
        budget: u64,
        /// The `--budget-mb` that would admit it, rounded up.
        ///
        /// Named in the sentence on purpose. A refusal that does not say what would work is a
        /// wall, and this is a bound on *accident* rather than on the person: `--budget-mb` has
        /// no ceiling, and on a connection with no limit the right answer to this message is
        /// often to repeat the command with the number it prints.
        needed: u64,
    },
    /// A member arrived whose bytes do not match what the archive recorded.
    #[error("{member}: {what}")]
    Corrupt {
        /// The member's name inside the archive.
        member: String,
        /// Which check failed and by how much.
        what: String,
    },
    /// The cache could not be read or written.
    #[error("{path}: {source}")]
    Cache {
        /// The path that failed.
        path: String,
        /// The underlying failure.
        source: std::io::Error,
    },
}
