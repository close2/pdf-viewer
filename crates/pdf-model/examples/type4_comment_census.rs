//! How many §7.10.5 programs carry a comment, and what the old compiler did to each.
//!
//! The population behind ADR 0361. Until the five-hundred-and-twenty-sixth session
//! `compile_postscript` split a type 4 stream on white space *before* looking for a PERCENT
//! SIGN — which destroys the line §7.2.4 ends a comment at — and then skipped exactly one token
//! after the sign. That has two outcomes and only one of them is visible:
//!
//! - **`refused`**: some word left in the comment is not an operator, so the function is refused
//!   and the shading, tint transform or soft mask that named it is reported. Loud, and how the
//!   defect was found.
//! - **`silent`**: every word left in the comment *is* a number or an operator, so they were
//!   compiled into the program. No error, no report, and a function that computes something the
//!   file never said. This is the count that matters, and it is what this program exists to
//!   measure.
//! - **`harmless`**: what was left of the comment changed no value the function returns — most
//!   often a comment of exactly one word, where skipping a token and cutting the line agree.
//!
//! # How each program is classified
//!
//! Both arms go through this crate's *current* compiler, and the old rule is reproduced as a
//! text transform ([`as_the_old_rule_read_it`]) rather than by keeping the old code alive. The
//! two programs are then evaluated over a grid of their own `/Domain`, and a difference at any
//! sample is a difference in what the file draws.
//!
//! Scanning for `/FunctionType` in the raw bytes before opening a document is sound rather than a
//! shortcut: a type 4 function *is* a stream, and §7.5.7 heads its exclusion list "The following
//! objects shall not be stored in an object stream:" with "Stream objects", so a function
//! dictionary is always in the file's own bytes rather than inside a compressed one.
//!
//! ```sh
//! cargo run --release -p pdf-model --example type4_comment_census -- \
//!     doc/pdf.js/test/pdfs/*.pdf doc/corpora/*/**/*.pdf doc/corpora-own/*.pdf
//! ```
//!
//! An argument beginning with `@` names a file of paths, one to a line — the `SafeDocs`
//! population is too large for a command line:
//!
//! ```sh
//! find corpus-cache -name '*.pdf' > /tmp/paths
//! cargo run --release -p pdf-model --example type4_comment_census -- @/tmp/paths
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;

use pdf_model::function::Function;
use pdf_syntax::{Document, Object, ObjectId};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

/// What one document contributes.
#[derive(Default)]
struct Counts {
    /// Documents whose bytes name `/FunctionType` and that opened.
    opened: usize,
    /// Documents carrying at least one type 4 function.
    with_type4: usize,
    /// Type 4 functions found.
    functions: usize,
    /// Type 4 functions whose program contains a PERCENT SIGN.
    commented: usize,
    /// Commented functions by what the old rule did to them.
    verdicts: BTreeMap<&'static str, usize>,
    /// Where each non-`harmless` verdict was found, for reading afterwards.
    witnesses: Vec<String>,
}

impl Counts {
    /// Adds `other`'s totals to this one's.
    fn absorb(&mut self, mut other: Self) {
        self.opened = self.opened.saturating_add(other.opened);
        self.with_type4 = self.with_type4.saturating_add(other.with_type4);
        self.functions = self.functions.saturating_add(other.functions);
        self.commented = self.commented.saturating_add(other.commented);
        for (key, count) in other.verdicts {
            let slot = self.verdicts.entry(key).or_default();
            *slot = slot.saturating_add(count);
        }
        self.witnesses.append(&mut other.witnesses);
    }
}

/// The program as the compiler read it before the fix: `%` and the one token after it, dropped.
///
/// Reproduced here rather than kept alive in the crate, because what is being measured is a
/// state this tree is no longer in. The spacing of the braces and the PERCENT SIGN is the old
/// code's own, so the token stream this returns is the one it compiled.
fn as_the_old_rule_read_it(program: &str) -> String {
    let spaced = program
        .replace('{', " { ")
        .replace('}', " } ")
        .replace('%', " % ");
    let mut out = String::with_capacity(spaced.len());
    let mut tokens = spaced.split_whitespace();
    while let Some(token) = tokens.next() {
        if token == "%" {
            tokens.next();
            continue;
        }
        out.push_str(token);
        out.push(' ');
    }
    out
}

/// A one-object document whose object 1 is a type 4 function with `program` as its stream.
///
/// `domain` and `range` are the real function's, so the arms are evaluated and clipped exactly
/// as the file asks; without them a comparison would be of two functions neither file states.
fn function_document(program: &str, domain: &str, range: &str) -> Option<Function> {
    let mut source = String::from("%PDF-1.7\n");
    let _ = write!(
        source,
        "1 0 obj\n<< /FunctionType 4 /Domain {domain} /Range {range} /Length {} >>\n\
         stream\n{program}\nendstream\nendobj\ntrailer\n<< /Root 1 0 R >>\n",
        program.len().saturating_add(1)
    );
    let document = Document::open(source.into_bytes()).ok()?;
    let object = Object::Reference(ObjectId {
        number: 1,
        generation: 0,
    });
    Function::parse(&document, &object).ok()
}

/// An array of numbers as PDF source, or `None` where the file does not state a usable one.
fn numbers(document: &Document, object: &Object) -> Option<Vec<f32>> {
    let resolved = document.resolve(object);
    let array = resolved.as_array()?;
    let mut out = Vec::with_capacity(array.len());
    for entry in array {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a bound outside f32's range is already useless as one"
        )]
        out.push(document.resolve(entry).as_number()? as f32);
    }
    (!out.is_empty() && out.len() % 2 == 0).then_some(out)
}

/// The inputs a function is compared at: each dimension at five points across its domain.
///
/// Bounded, because a grid over a six-input tint transform is 15 625 evaluations and the point
/// is to find *a* difference rather than to characterise it.
fn grid(domain: &[f32]) -> Vec<Vec<f32>> {
    /// Most sample points one comparison may spend.
    const MOST: usize = 4096;

    let mut rows: Vec<Vec<f32>> = vec![Vec::new()];
    for pair in domain.chunks_exact(2) {
        let (low, high) = (pair.first().copied(), pair.get(1).copied());
        let (Some(low), Some(high)) = (low, high) else {
            continue;
        };
        let mut next = Vec::new();
        for row in &rows {
            for step in 0..5_u8 {
                if next.len() >= MOST {
                    return next;
                }
                let mut row = row.clone();
                row.push(low + (high - low) * f32::from(step) / 4.0);
                next.push(row);
            }
        }
        rows = next;
    }
    rows
}

/// What one document's type 4 functions say.
fn census(path: &str, document: &Document) -> Counts {
    let mut counts = Counts {
        opened: 1,
        ..Counts::default()
    };
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        let Some(stream) = object.as_stream() else {
            continue;
        };
        if document.get_key(&stream.dict, "FunctionType").as_integer() != Some(4) {
            continue;
        }
        counts.functions = counts.functions.saturating_add(1);
        let Some(data) = document.decoded_stream_data(stream) else {
            continue;
        };
        if !data.contains(&b'%') {
            continue;
        }
        counts.commented = counts.commented.saturating_add(1);

        let program = String::from_utf8_lossy(&data).into_owned();
        let Some(domain) = numbers(document, &document.get_key(&stream.dict, "Domain")) else {
            continue;
        };
        let Some(range) = numbers(document, &document.get_key(&stream.dict, "Range")) else {
            continue;
        };
        let as_written = |values: &[f32]| {
            let mut out = String::from("[");
            for value in values {
                let _ = write!(out, "{value} ");
            }
            out.push(']');
            out
        };
        let (domain_source, range_source) = (as_written(&domain), as_written(&range));

        let now = function_document(&program, &domain_source, &range_source);
        let before = function_document(
            &as_the_old_rule_read_it(&program),
            &domain_source,
            &range_source,
        );
        let verdict = match (now, before) {
            (None, _) => "unreadable even now",
            (Some(_), None) => "refused",
            (Some(now), Some(before)) => {
                let differs = grid(&domain)
                    .into_iter()
                    .any(|inputs| now.eval(&inputs) != before.eval(&inputs));
                if differs { "silent" } else { "harmless" }
            }
        };
        let slot = counts.verdicts.entry(verdict).or_default();
        *slot = slot.saturating_add(1);
        // Every commented function is named, `harmless` included: the population is small
        // enough to read, and a verdict nobody can go and look at is a number to be believed.
        counts
            .witnesses
            .push(format!("{verdict}: {path} object {number}"));
    }
    if counts.functions > 0 {
        counts.with_type4 = 1;
    }
    counts
}

/// The paths to walk: arguments, and the lines of any argument beginning with `@`.
fn paths() -> Vec<String> {
    let mut out = Vec::new();
    for argument in std::env::args().skip(1) {
        match argument.strip_prefix('@') {
            Some(list) => match std::fs::read_to_string(list) {
                Ok(text) => out.extend(text.lines().map(str::to_owned)),
                Err(error) => println!("{list}: {error}"),
            },
            None => out.push(argument),
        }
    }
    out
}

fn main() {
    let paths = paths();
    let counts = paths
        .par_iter()
        .map(|path| {
            let mut counts = Counts::default();
            let Ok(bytes) = std::fs::read(path) else {
                return counts;
            };
            // §7.5.7 keeps every stream out of an object stream, so a document with no
            // `/FunctionType` in its own bytes has no type 4 function to find.
            if !bytes
                .windows(b"/FunctionType".len())
                .any(|window| window == b"/FunctionType")
            {
                return counts;
            }
            let Ok(document) = Document::open(bytes) else {
                return counts;
            };
            counts.absorb(census(path, &document));
            counts
        })
        .reduce(Counts::default, |mut total, counts| {
            total.absorb(counts);
            total
        });

    println!(
        "{} paths, {} opened with a /FunctionType in their bytes, {} carry a type 4 function",
        paths.len(),
        counts.opened,
        counts.with_type4,
    );
    println!(
        "{} type 4 functions, {} of them containing a PERCENT SIGN",
        counts.functions, counts.commented,
    );
    for (verdict, count) in &counts.verdicts {
        println!("  {verdict}: {count}");
    }
    for witness in &counts.witnesses {
        println!("{witness}");
    }
}
