//! What an entry point answers when it is not `PDFVFS_OK`.
//!
//! **Two populations, kept apart, and that separation is the whole of this module.** A caller of
//! this ABI can be wrong in two unrelated ways: it can pass a null pointer or an index nothing
//! names, which is a *mistake in the C program*; or the tree can refuse what it asked, which is a
//! *fact about the document* and carries [`pdf_vfs::Errno`] and a sentence. Folding the two into
//! one number would make a KIO worker report "operation not permitted" for its own null pointer,
//! and would make an `EPERM` from the layout indistinguishable from a bug in the shim.
//!
//! So a refusal by the tree is exactly one status — [`Status::Refused`] — and everything a person
//! could be shown about it arrives in the [`crate::Refusal`] the same call wrote. The rest of
//! this enumeration is the caller's own mistakes and the two conditions the machine can impose,
//! which is `viewer-ffi`'s division and is followed here rather than invented again.

/// The result of an entry point.
///
/// `#[repr(i32)]` because that is what the header declares and what C compares against. The
/// values are part of the ABI: a variant added later takes the next number and never reuses one,
/// which is why they are written out rather than left to the compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum Status {
    /// It worked, and every out-parameter has been written.
    Ok = 0,
    /// A pointer argument was null where the function requires one.
    ///
    /// The one mistake a C caller makes by accident rather than by arithmetic, and the one this
    /// crate can always detect: a null pointer is a value, and a dangling one is not.
    NullArgument = 1,
    /// An index named nothing: an entry, a warning or a shortfall the object does not have.
    OutOfRange = 2,
    /// The buffer offered is too small, and `needed` says how many bytes would do.
    ///
    /// Not an error in the sense the others are: it is half of the two-call idiom C uses for a
    /// string of unknown length, and a caller that passes a null buffer to learn the size gets
    /// exactly this.
    BufferTooSmall = 3,
    /// A string argument was not UTF-8.
    ///
    /// Every path crossing this boundary is text. Refusing rather than replacing is the rule
    /// `pdf-syntax` follows and the one a file name deserves most: an invented replacement
    /// character in a name is a file the tree does not have, named quietly.
    NotUtf8 = 4,
    /// The tree refused, and the [`crate::Refusal`] beside it says which `errno` and why.
    ///
    /// **The only status that is about the document rather than about the caller.** Every
    /// entry point that can answer it takes a `pdfvfs_refusal **`, and writes it only here.
    Refused = 5,
    /// There is no answer, and the question was fair.
    ///
    /// The layout names no row for that path, so there is no meaning for a write to it — not a
    /// refusal, because a refusal is a row that says no. `viewer-ffi`'s `PDFV_NO_ANSWER` for the
    /// same reason: a host asking about a path the tree does not have has asked a fair question.
    NoAnswer = 6,
    /// No component of the path names a file on disk, so there is no document to serve.
    ///
    /// What `pdfvfs_split` answers for `pdf:/home/u/nothing-here/pages`. Its own status rather
    /// than a refusal because nothing has been opened yet: there is no tree to refuse.
    NoDocument = 7,
    /// A number did not fit the type this boundary states for it.
    ///
    /// `size_t` is 64 bits here and so is every count; what this catches is a length or an offset
    /// that does not fit what `pdf-vfs` takes.
    NumberOutOfRange = 8,
}

impl Status {
    /// One sentence, for `pdfvfs_status_message`.
    ///
    /// `&'static str` with a NUL already in it, because the C side hands back a `const char *`
    /// that outlives every call and is never freed. Written as literals rather than built,
    /// because a message that allocated would be a message that can fail.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::Ok => "ok\0",
            Self::NullArgument => "a required pointer argument was null\0",
            Self::OutOfRange => "the index names nothing\0",
            Self::BufferTooSmall => "the buffer is too small; ask again with the size reported\0",
            Self::NotUtf8 => "a string argument was not UTF-8\0",
            Self::Refused => "the tree refused; read the refusal beside this status\0",
            Self::NoAnswer => "there is nothing to answer with\0",
            Self::NoDocument => "no part of this path is a file, so there is no document here\0",
            Self::NumberOutOfRange => {
                "a number does not fit the type this boundary states for it\0"
            }
        }
    }

    /// The number the header declares, as C sees it.
    #[must_use]
    pub const fn code(self) -> i32 {
        self as i32
    }
}

#[cfg(test)]
mod tests {
    use super::Status;

    /// Every status this ABI can answer with, so the two tests below walk the population rather
    /// than a subset somebody remembered.
    const EVERY: [Status; 9] = [
        Status::Ok,
        Status::NullArgument,
        Status::OutOfRange,
        Status::BufferTooSmall,
        Status::NotUtf8,
        Status::Refused,
        Status::NoAnswer,
        Status::NoDocument,
        Status::NumberOutOfRange,
    ];

    /// Every message is NUL-terminated exactly once and is not empty.
    ///
    /// The invariant `pdfvfs_status_message` rests on: it hands the bytes back as a
    /// `const char *`, so a message with no NUL would be a read past the end of a `&'static str`
    /// and one with two would truncate silently.
    #[test]
    fn every_status_message_ends_in_one_nul() {
        for status in EVERY {
            let message = status.message();
            assert_eq!(
                message.matches('\0').count(),
                1,
                "{status:?}: {message:?} has the wrong number of terminators"
            );
            assert!(message.ends_with('\0'), "{status:?}: {message:?}");
            assert!(
                message.len() > 1,
                "{status:?}: an empty message says nothing"
            );
        }
    }

    /// The numbers are the ABI, so they are asserted rather than derived.
    #[test]
    fn the_status_numbers_are_the_ones_the_header_declares() {
        for (expected, status) in EVERY.iter().enumerate() {
            let expected = i32::try_from(expected).unwrap_or(-1);
            assert_eq!(status.code(), expected, "{status:?} has moved");
        }
    }
}
