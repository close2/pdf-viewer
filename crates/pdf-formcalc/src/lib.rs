//! `FormCalc`, XFA 3.3 chapter 25's calculation language: lexed ([`lex`]), parsed ([`parse`]) and
//! evaluated ([`eval`]) over a [`Host`] that answers for the form, under a [`Budget`].
//!
//! One responsibility: *turn a script's text and a form's values into the value the script
//! computes, and the assignments it made*. The crate holds no document and reads no file; RFC
//! 0011 section 5's step 6 runs it in the confined script worker beside the ECMAScript engine,
//! because what it evaluates is still a document's code (ADR 2482). XFA 3.3 is cited by page and
//! paraphrased, never quoted.

#![forbid(unsafe_code)]

pub mod eval;
pub mod lex;
pub mod parse;
pub mod value;

pub use eval::{Budget, Host, Occurrence, Segment, Stop, run, run_counted};
pub use value::Value;
