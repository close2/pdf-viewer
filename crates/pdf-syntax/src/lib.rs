//! PDF lexer, object parser and cross-reference resolution.
//!
//! This is the crate that touches untrusted bytes first, and it is consequently the
//! most security-sensitive code in the project. It parses tokens, objects, streams
//! and the cross-reference table or stream, and it resolves indirect references.
//!
//! It deliberately does *not* interpret meaning: a page tree is a dictionary here,
//! not a page tree. Semantics belong to `pdf-model`. Keeping the split sharp means
//! the fuzzing surface is a pure byte-slice-to-object-graph function with no
//! higher-level state to configure.
//!
//! Every entry point must terminate on arbitrary input. Malformed files are the
//! normal case, not the exception: real-world PDFs routinely have broken xref
//! tables that must be recovered by scanning, and a viewer that refuses them is
//! useless. Recovery must be bounded, because unbounded recovery is a denial of
//! service.

#![forbid(unsafe_code)]
