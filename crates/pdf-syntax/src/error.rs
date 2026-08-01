//! Parsing failures.
//!
//! Every variant carries the byte offset. A PDF error without an offset is nearly
//! useless: the file is binary, often generated, and "unexpected token" alone gives a
//! reader nowhere to look.

/// A parsing failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SyntaxError {
    /// Input ended while something was still expected.
    #[error("unexpected end of input at byte {at}, expected {expected}")]
    UnexpectedEnd {
        /// Byte offset at which input ran out.
        at: usize,
        /// What was being looked for.
        expected: &'static str,
    },
    /// A token appeared where it cannot.
    #[error("unexpected {found} at byte {at}, expected {expected}")]
    Unexpected {
        /// Byte offset of the offending token.
        at: usize,
        /// What was found.
        found: String,
        /// What was being looked for.
        expected: &'static str,
    },
    /// A resource bound was reached.
    ///
    /// Distinct from a malformed-input error because it is a statement about *this
    /// reader's* configuration rather than about the file: the document may be perfectly
    /// valid and merely larger than the configured bound.
    #[error("resource limit {limit} exceeded at byte {at}")]
    LimitExceeded {
        /// Byte offset at which the bound was reached.
        at: usize,
        /// Which bound, named as its field in `Limits`.
        limit: &'static str,
    },
    /// The file begins with neither a PDF header (§7.5.2) nor an FDF one (§12.7.8.2.2).
    #[error("not a PDF: no %PDF- or %FDF- header in the first {searched} bytes")]
    NoHeader {
        /// How far the search went.
        searched: usize,
    },
    /// No cross-reference information could be found or reconstructed.
    #[error("no usable cross-reference table: {detail}")]
    NoCrossReferences {
        /// What was tried.
        detail: String,
    },
    /// The trailer is missing a key the document cannot be read without.
    #[error("trailer has no {key}")]
    TrailerMissing {
        /// The absent key.
        key: &'static str,
    },
    /// The document is encrypted and the password supplied does not open it.
    ///
    /// ISO 32000-2 §7.6.4.1 has a reader try the default user password first and prompt
    /// only if that fails, so this is the signal to ask — not a statement that the file is
    /// broken.
    #[error("the document is encrypted and needs a password (ISO 32000-2 §7.6.4.1)")]
    PasswordRequired,
    /// The document is encrypted by something this reader does not implement.
    ///
    /// Distinct from [`Self::PasswordRequired`] because no password will help: the file
    /// names a security handler, revision or crypt filter method that is not here.
    #[error("unsupported encryption: {detail}")]
    UnsupportedEncryption {
        /// What was named, and the clause that defines it.
        detail: String,
    },
}

/// Shorthand for a parsing result.
pub type SyntaxResult<T> = Result<T, SyntaxError>;
