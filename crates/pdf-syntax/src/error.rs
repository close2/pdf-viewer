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
    /// The file does not begin with a PDF header.
    #[error("not a PDF: no %PDF- header in the first {searched} bytes")]
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
}

/// Shorthand for a parsing result.
pub type SyntaxResult<T> = Result<T, SyntaxError>;
