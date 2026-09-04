//! A refusal by the tree, owned by the caller: the `errno` it is, and the sentence beside it.
//!
//! # Why this is an object rather than a slot on the mount
//!
//! RFC 0003 section 5.3 requires every refusal to carry a sentence, and section 7 requires the
//! `errno` to be the **core's** rather than a face's — "a KIO worker mapping these onto
//! `KIO::Error` and a FUSE daemon handing them to the kernel must agree about what a refused
//! write *is*, and a face that chose its own numbers would be the second copy of a decision".
//! Both have to survive the call that produced them, and a C caller cannot see a `Result`.
//!
//! The other candidate was a *last-error slot* on the mount, in the shape `errno(3)` itself has,
//! and it is refused for the reason [`crate::status`] gives about the two populations: a slot is
//! a global a caller may forget to read, it is written by calls that did not fail, and two
//! threads sharing one mount would overwrite each other's answer. An owned object is written
//! **only** where the status is `PDFVFS_REFUSED`, is read where the caller likes, and is freed by
//! the caller — which is `viewer-ffi`'s owned-batch discipline applied to one message.
//!
//! # Where the number comes from
//!
//! [`pdf_vfs::Errno`], unchanged and untranslated. This crate names the kinds so that a C caller
//! can print one, and it does not invent a single number: `Errno::code` is what Linux states and
//! `Errno::as_str` is what a log line says beside it.

use pdf_vfs::{Errno, VfsError};

/// Every `errno` kind [`pdf_vfs::Errno`] has, in the order that enumeration declares them.
///
/// The population a C caller may meet. Held as an array rather than derived, because there is no
/// way to iterate a Rust enumeration — and [`tag`] below is what makes the array complete rather
/// than merely long: a kind added to the core fails `tag`'s exhaustive `match` at compile time,
/// and then fails the test that every tag from zero to [`KIND_COUNT`] appears here.
pub const KINDS: [Errno; 13] = [
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
    Errno::Overflow,
    Errno::Stale,
];

/// How many kinds there are, which `pdfvfs_abi_check` compares against the header's.
///
/// **This is what stands in for the Rust rule that a new refusal fails to compile in every
/// consumer.** A C program switching on `errno` numbers cannot be made to fail its build, so it
/// fails its *startup* instead, once, naming the number that moved — `viewer-ffi`'s
/// `PDFV_EVENT_KIND_COUNT` and its argument, one boundary over.
pub const KIND_COUNT: u32 = 13;

/// Each kind's position in [`KINDS`], by an exhaustive `match`.
///
/// The only reason this function exists is that its `match` cannot be written incompletely: a
/// thirteenth kind added to `pdf_vfs::Errno` stops this crate compiling, which is the guarantee
/// the C side has to buy back at runtime.
const fn tag(errno: Errno) -> usize {
    match errno {
        Errno::OperationNotPermitted => 0,
        Errno::NoSuchFile => 1,
        Errno::InputOutput => 2,
        Errno::PermissionDenied => 3,
        Errno::Exists => 4,
        Errno::NotADirectory => 5,
        Errno::IsADirectory => 6,
        Errno::Invalid => 7,
        Errno::TooBig => 8,
        Errno::ReadOnly => 9,
        Errno::NotImplemented => 10,
        Errno::Overflow => 11,
        Errno::Stale => 12,
    }
}

/// Each kind's name, in [`tag`]'s order, with the NUL a `const char *` needs.
///
/// The C side hands these back as a `const char *` that outlives every call and is never freed,
/// which is why they are literals rather than built: a name that allocated would be a name that
/// can fail. `pdf_vfs::Errno::as_str` states the same words, and the test below holds the two
/// together rather than trusting that they were copied correctly.
const NAMES: [&str; 13] = [
    "EPERM\0",
    "ENOENT\0",
    "EIO\0",
    "EACCES\0",
    "EEXIST\0",
    "ENOTDIR\0",
    "EISDIR\0",
    "EINVAL\0",
    "EFBIG\0",
    "EROFS\0",
    "ENOSYS\0",
    "EOVERFLOW\0",
    "ESTALE\0",
];

/// The name of the `errno` a number is, **for every number including ones this build does not
/// know**.
///
/// Trap 5 in the form C leaves available, and the same answer `viewer-ffi` gives an event kind it
/// has never heard of: a caller handed a number it has no arm for can still log something rather
/// than dropping it in silence. A number no kind here states is not an error — it is a build of
/// this library newer than the caller.
#[must_use]
pub fn name_of(code: i32) -> &'static str {
    let Some(errno) = KINDS.iter().copied().find(|kind| kind.code() == code) else {
        return "an errno this build of the library does not name\0";
    };
    NAMES
        .get(tag(errno))
        .copied()
        // Unreachable: `tag` answers an index into an array of the same length, and the test
        // below walks every kind through both. Written as a sentence rather than an index so
        // that nothing here can panic across a C boundary.
        .unwrap_or("an errno this build of the library does not name\0")
}

/// One refusal, owned by whoever asked for the operation that produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// Which `errno` the core says this is.
    errno: Errno,
    /// The sentence RFC 0003 section 5.3 requires, which a KIO worker hands to `KIO::WorkerResult`
    /// and which a FUSE mount can only log.
    sentence: String,
}

impl Refusal {
    /// The refusal a [`pdf_vfs::VfsError`] is, with both halves taken from the core.
    #[must_use]
    pub fn of(error: &VfsError) -> Self {
        Self {
            errno: error.errno(),
            sentence: error.to_string(),
        }
    }

    /// One this crate states itself, for a condition that never reached the core.
    ///
    /// There is exactly one — a document that cannot be opened at all — and it is here rather
    /// than in the core because a face opens the file and the core is handed what it opened.
    #[must_use]
    pub fn stated(errno: Errno, sentence: String) -> Self {
        Self { errno, sentence }
    }

    /// The number, as Linux states it.
    #[must_use]
    pub fn code(&self) -> i32 {
        self.errno.code()
    }

    /// The sentence a person reads.
    #[must_use]
    pub fn sentence(&self) -> &str {
        &self.sentence
    }
}

#[cfg(test)]
mod tests {
    use super::{KIND_COUNT, KINDS, Refusal, name_of, tag};
    use pdf_vfs::Errno;

    /// Every kind the core has appears in [`KINDS`] exactly once.
    ///
    /// [`tag`] is what makes this a check rather than a restatement: its `match` is exhaustive, so
    /// a kind added to `pdf_vfs::Errno` breaks the build, and the tag it is then given has to
    /// appear below or this fails.
    #[test]
    fn the_population_is_every_kind_the_core_has() {
        let mut seen = vec![false; KINDS.len()];
        for kind in KINDS {
            let at = tag(kind);
            assert!(!seen[at], "{kind:?} appears twice");
            seen[at] = true;
        }
        assert!(
            seen.iter().all(|found| *found),
            "a kind the core has is missing from KINDS"
        );
        assert_eq!(KIND_COUNT as usize, KINDS.len());
    }

    /// Every kind has a distinct number and a name that ends in one NUL.
    #[test]
    fn every_kind_has_its_own_number_and_a_name() {
        let mut codes = Vec::new();
        for kind in KINDS {
            let name = name_of(kind.code());
            assert_eq!(name.matches('\0').count(), 1, "{kind:?}: {name:?}");
            assert!(name.len() > 1, "{kind:?} has no name");
            assert_eq!(
                name.trim_end_matches('\0'),
                kind.as_str(),
                "the name here and the core's have come apart"
            );
            codes.push(kind.code());
        }
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), KINDS.len(), "two kinds share a number");
    }

    /// A number no kind states is named rather than dropped, which is the whole of trap 5 here.
    #[test]
    fn a_number_this_build_does_not_know_is_still_named() {
        let unknown = name_of(9999);
        assert!(unknown.ends_with('\0'));
        assert!(unknown.contains("does not name"));
    }

    /// A refusal keeps the core's number and the core's words.
    #[test]
    fn a_refusal_carries_the_cores_number_and_the_cores_sentence() {
        let refusal = Refusal::stated(Errno::Stale, String::from("the document changed"));
        assert_eq!(refusal.code(), 116);
        assert_eq!(refusal.sentence(), "the document changed");
    }
}
