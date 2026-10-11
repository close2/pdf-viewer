//! Interprets one page repeatedly, for deterministic instruction counting.
//!
//! Run under callgrind, which counts instructions rather than measuring time and is
//! therefore unaffected by CPU frequency, background load or thermal state:
//!
//! ```sh
//! valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.out \
//!     cargo run --release -p pdf-model --example callgrind_interpret
//! ```
//!
//! With no arguments it interprets page 101 of ISO 32000-2 itself, which is committed in
//! `doc/` and is the page `callgrind_rasterise.rs` draws, so the two numbers are about one
//! page. That page is text and vector graphics; a change to how *images* are decoded costs
//! nothing measurable on it, because image decode happens here rather than in the backend
//! and that page carries no image. So a path and a 1-based page number may be given
//! instead:
//!
//! ```sh
//! cargo run --release -p pdf-model --example callgrind_interpret -- \
//!     doc/pdf.js/test/pdfs/issue19971.pdf 1
//! ```
//!
//! `--digest` first prints, instead of the command count, a SHA-256 of the interpretation's
//! display list and reports in their `Debug` renderings — the rendering `tests/raster_golden.rs`
//! holds — so that a change to the interpreter's speed is shown to hand back the same list, byte
//! for byte, by running HEAD's build and the change's on one page. The hash streams through the
//! formatter rather than building the text, which on a list of millions of commands would be
//! gigabytes; and it is a separate mode so that a counted run has no hashing in it.
//!
//! ```sh
//! cargo run --release -p pdf-model --example callgrind_interpret -- --digest <file.pdf> 1
//! ```
//!
//! `--stage=<stage>` runs a prefix of the interpretation instead, so that the instructions each
//! stage adds are the difference between two counted runs rather than a reading of a profile in
//! which most of the token loop is inlined into one function: `read` pumps the page's
//! `/Contents` through the window and keeps nothing (§7.4's filters, the window's copies);
//! `lex` reads every token as the interpreter's loop does and keeps nothing; `operands` makes
//! each operand the [`pdf_syntax::Object`] the interpreter holds it as, in a list an operator
//! clears, which is what §7.8.2's operand stack costs before any operator runs. A whole run less
//! `operands` is what the operators themselves do — the graphics state, the paths, the colours
//! and the display list.
//!
//! ```sh
//! valgrind --tool=callgrind cargo run --release -p pdf-model --example callgrind_interpret -- \
//!     --stage=lex <file.pdf> 1
//! ```
use std::fmt::Write as _;

use sha2::{Digest as _, Sha256};

/// A `fmt::Write` that feeds a hash, so a `Debug` rendering is digested without being held.
struct Hashing(Sha256);

impl std::fmt::Write for Hashing {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0.update(text.as_bytes());
        Ok(())
    }
}

#[expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a measurement tool should stop loudly if the corpus file is missing, and \
              its command counter is bounded by fifty pages"
)]
fn main() {
    let mut args = std::env::args().skip(1).peekable();
    let digesting = args.next_if(|first| first == "--digest").is_some();
    let stage = args
        .next_if(|first| first.starts_with("--stage="))
        .map(|flag| flag.trim_start_matches("--stage=").to_owned());
    let named = args.next();
    let index = args
        .next()
        .map_or(101, |n| n.parse::<usize>().expect("a page number"));
    // A photograph is decoded once per interpretation and dwarfs everything else on its
    // page, so fifty repetitions of one would be fifty times a measurement one already
    // makes. The specification page is the opposite: small, and worth repeating to lift the
    // signal above process start-up.
    let repetitions = if named.is_some() { 1 } else { 50 };
    let path = named.map_or_else(
        || {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../doc/ISO_32000-2_sponsored_EC3.pdf")
        },
        std::path::PathBuf::from,
    );

    let bytes = std::fs::read(&path).expect("corpus file is readable");
    let document = pdf_syntax::Document::open(bytes).expect("valid PDF");
    let page = pdf_model::Pages::new(&document)
        .get(index - 1)
        .expect("page exists");

    if let Some(stage) = stage {
        println!("{}", run_stage(&stage, &document, &page));
        return;
    }

    if digesting {
        let interpretation = pdf_model::interpret(&document, &page);
        let mut hashing = Hashing(Sha256::new());
        write!(
            hashing,
            "{:?}\n{:?}\n{}",
            interpretation.display_list, interpretation.unsupported, interpretation.list_bytes
        )
        .expect("hashing cannot fail");
        let mut hex = String::with_capacity(64);
        for byte in hashing.0.finalize() {
            write!(hex, "{byte:02x}").expect("writing to a string cannot fail");
        }
        println!(
            "{hex}\t{} commands\t{} bytes charged",
            interpretation.display_list.command_count(),
            interpretation.list_bytes
        );
        return;
    }

    let mut total = 0usize;
    for _ in 0..repetitions {
        total += pdf_model::interpret(&document, &page)
            .display_list
            .commands()
            .len();
    }
    println!("{total}");
}

/// Runs one prefix of the interpretation over `page`'s `/Contents`, and returns a count that
/// depends on every token so that nothing is optimised away.
#[expect(
    clippy::panic,
    reason = "an unknown stage name is the measurement's caller's mistake"
)]
fn run_stage(stage: &str, document: &pdf_syntax::Document, page: &pdf_model::Page) -> usize {
    use pdf_model::content::reader::ContentReader;
    use pdf_syntax::{Object, Token};

    let mut reader = ContentReader::for_page(document, page);
    match stage {
        "read" => reader.read_to_end().len(),
        "lex" => {
            let mut tokens = 0usize;
            while reader.with_token(|token| token.is_some()) {
                tokens = tokens.wrapping_add(1);
            }
            tokens
        }
        "operands" => {
            // The interpreter's own conversion is private to its module; this is the same
            // match, so that the stage costs what the interpreter's operand list costs.
            let mut pending: Vec<Object> = Vec::new();
            let mut operators = 0usize;
            loop {
                let more = reader.with_token(|token| match token {
                    None => false,
                    Some(Token::Keyword(_)) => {
                        operators = operators.wrapping_add(pending.len());
                        pending.clear();
                        true
                    }
                    Some(other) => {
                        pending.push(match other {
                            Token::Integer(value) => Object::Integer(value),
                            Token::Real(value) => Object::Real(value),
                            Token::Name(bytes) => Object::Name(pdf_syntax::Name::new(bytes)),
                            Token::String(bytes) => Object::String(bytes.into()),
                            _ => Object::Null,
                        });
                        true
                    }
                });
                if !more {
                    break;
                }
            }
            operators
        }
        other => panic!("no stage {other}: read, lex or operands"),
    }
}
