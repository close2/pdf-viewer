//! PDF lexer, object parser and cross-reference resolution.
//!
//! This is the crate that touches untrusted bytes first, and is consequently the most
//! security-sensitive code in the project. It turns a byte slice into an object graph and
//! nothing more: a page tree is a dictionary here, not a page. Meaning belongs to
//! `pdf-model`.
//!
//! Keeping that split sharp is what makes the fuzzing surface a pure
//! bytes-to-object-graph function with no higher-level state to configure.
//!
//! # Malformed files are the normal case
//!
//! Real-world PDFs routinely have broken cross-reference tables, wrong `/Length` values,
//! missing `endobj` keywords and truncated streams. A reader that rejects them is not
//! useful, so this crate recovers where recovery is well-defined — and every recovery is
//! *bounded*, because unbounded recovery is a denial of service.
//!
//! # What protects against what
//!
//! Rust removes memory corruption. It does not remove resource exhaustion, so [`Limits`]
//! bounds nesting depth, container sizes and stream lengths. Exceeding a bound is an
//! error naming the bound — never a panic, and never a silently truncated object, which
//! would render a wrong page while reporting success.

#![forbid(unsafe_code)]

pub mod crypt;
pub mod date;
pub mod document;
pub mod error;
pub mod filter;
pub mod lexer;
pub mod object;
pub mod parser;
pub mod text_string;
pub mod tree;
pub mod version;
pub mod write;
pub mod xref;

pub use crypt::Permissions;
pub use date::Date;
pub use document::{Document, ImageStream};
pub use error::{SyntaxError, SyntaxResult};
pub use lexer::{Lexer, Token};
pub use object::{Dictionary, Name, Object, ObjectId, Stream};
pub use parser::{Limits, Parser};
pub use text_string::text_string;
pub use version::Version;
pub use xref::{Location, XrefTable};
