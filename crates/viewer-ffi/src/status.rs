//! What an entry point answers when it is not `PDFV_OK`.
//!
//! **A C caller cannot see a `Result`**, so every refusal in this crate becomes an `int32_t` it
//! can compare and a sentence it can print. The rule the rest of the tree holds — no silent error
//! swallowing, every error typed and handled somewhere deliberate — is kept here by making the
//! type the *return value* rather than a global a caller may forget to read.
//!
//! Nothing here describes a document. A file that will not open is
//! [`viewer_core::Event::OpenFailed`] carrying `pdf-syntax`'s own words, because that is a fact
//! about the file and belongs in the same channel as every other fact about it. What is here is
//! the caller's own mistakes and the two conditions the machine can impose.

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
    /// Distinct from [`Self::OutOfRange`] because it is the one mistake a C caller makes by
    /// accident rather than by arithmetic, and because it is the one this crate can always
    /// detect: a null pointer is a value, and a dangling one is not.
    NullArgument = 1,
    /// An index named nothing: an event, a row or a page the object does not have.
    OutOfRange = 2,
    /// The message at that index is not of the kind this accessor reads.
    ///
    /// A caller that asked for `pdfv_event_opened` on a `PageChanged` gets this rather than
    /// zeroes, because zeroes are a page count.
    WrongKind = 3,
    /// The buffer offered is too small, and `needed` says how many bytes would do.
    ///
    /// Not an error in the sense the others are: it is half of the two-call idiom C uses for a
    /// string of unknown length, and a caller that passes a null buffer to learn the size gets
    /// exactly this.
    BufferTooSmall = 4,
    /// There is no answer: no document is focused, or the question named a page that is not
    /// showing.
    ///
    /// [`viewer_core::Answer::None`], which is not a failure and is deliberately not folded into
    /// one — a host asking for the outline of a document with none has asked a fair question.
    NoAnswer = 5,
    /// A string argument was not UTF-8.
    ///
    /// Every string crossing this boundary is text: §7.9.2.2's text strings arrive decoded, and a
    /// password and a fragment are what the *host* holds. Refusing rather than replacing is the
    /// same rule `pdf-syntax` follows — an invented replacement character in a password is a
    /// password that does not open the file, said quietly.
    NotUtf8 = 6,
    /// The rasteriser refused the request, or the raster it produced does not fit the target.
    ///
    /// The one status that is about drawing. What went wrong is `render-cpu`'s own words, which
    /// `pdfv_status_message` cannot carry — so a caller that wants them hands the failure back
    /// with `pdfv_render_ready_failed` and reads them off the event that comes out.
    RenderRefused = 7,
    /// A number did not fit the type the boundary states for it.
    ///
    /// `usize` is 64 bits here and a page index crosses as one; what this catches is the other
    /// direction, a `u32` viewport dimension or an `i64` page target that does not fit what
    /// `viewer-core` takes.
    OutOfRange64 = 8,
}

impl Status {
    /// One sentence, for `pdfv_status_message`.
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
            Self::WrongKind => "the message at that index is not of the kind this accessor reads\0",
            Self::BufferTooSmall => "the buffer is too small; ask again with the size reported\0",
            Self::NoAnswer => "there is nothing to answer with\0",
            Self::NotUtf8 => "a string argument was not UTF-8\0",
            Self::RenderRefused => "the page could not be rasterised\0",
            Self::OutOfRange64 => "a number does not fit the type this boundary states for it\0",
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

    /// Every message is NUL-terminated exactly once and is not empty.
    ///
    /// The invariant `pdfv_status_message` rests on: it hands back the bytes as a `const char *`,
    /// so a message with no NUL would be a read past the end of a `&'static str` and one with two
    /// would truncate silently. Checked over the list rather than promised beside it.
    #[test]
    fn every_status_message_ends_in_one_nul() {
        for status in [
            Status::Ok,
            Status::NullArgument,
            Status::OutOfRange,
            Status::WrongKind,
            Status::BufferTooSmall,
            Status::NoAnswer,
            Status::NotUtf8,
            Status::RenderRefused,
            Status::OutOfRange64,
        ] {
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
        assert_eq!(Status::Ok.code(), 0);
        assert_eq!(Status::NullArgument.code(), 1);
        assert_eq!(Status::OutOfRange.code(), 2);
        assert_eq!(Status::WrongKind.code(), 3);
        assert_eq!(Status::BufferTooSmall.code(), 4);
        assert_eq!(Status::NoAnswer.code(), 5);
        assert_eq!(Status::NotUtf8.code(), 6);
        assert_eq!(Status::RenderRefused.code(), 7);
        assert_eq!(Status::OutOfRange64.code(), 8);
    }
}
