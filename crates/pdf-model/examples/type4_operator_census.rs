//! Which of Table 42's operators the corpora's §7.10.5 programs actually reach, and what the
//! two defects of the five-hundred-and-thirty-fourth session cost where they were reached.
//!
//! The population behind ADR 0369. Two arms of `apply_operator` were wrong against the
//! semantics ISO 32000-2 §7.10.5.2 defers to the PostScript Language Reference — `round` took a
//! tie away from zero rather than to the greater integer, and `eq`/`ne` compared within
//! `f32::EPSILON` where the operator is a relation — and a third question, `bitshift`'s width on
//! a right shift of a negative value, is a documented choice rather than a reading. None of the
//! three can be priced from the clause: what they cost is a fact about the files that exist, and
//! this program is what counts it.
//!
//! It answers two questions and keeps them apart, because they have different denominators
//! (`CLAUDE.md`, "Two questions, two denominators"):
//!
//! - **Containment.** For every operator in Table 42, how many type 4 functions and how many
//!   documents contain it. Exact, no evaluation involved, and it prices the whole audit rather
//!   than only the two defects — an operator no file reaches is a defect that costs nothing
//!   today and everything on the day a file reaches it.
//! - **Consequence.** For each function reaching `round`, `eq` or `ne`, whether the value the
//!   function returns actually moved. This is the number that says whether a page changed.
//!
//! # How the "before" arm is built, and why it is exact
//!
//! The old semantics are reproduced as a **source rewrite** rather than by keeping the old code
//! alive — the method `type4_comment_census` established for ADR 0361 — and here the rewrite can
//! be exact rather than approximate, because each old operator is expressible in Table 42 using
//! operators this round did not touch:
//!
//! - `eq` was `(a - b).abs() < f32::EPSILON`, which is `sub abs <eps> lt` to the bit. `ne` was
//!   its `>=`. The epsilon is written out of `f32::EPSILON` itself, so no constant is retyped.
//! - `round` differs from the fixed one at exactly one place: a value that is an exact tie *and*
//!   negative, where the old answer is one lower. So old `round` is the new one minus that
//!   condition, and the condition — `is a tie` and `is negative` — is `floor`, `sub`, `eq`, `lt`
//!   and `and`, none of which moved. The `eq` inside that test is deliberately the *fixed* one:
//!   it is asking whether a fraction is exactly a half, which is what the fixed operator answers.
//!
//! Both arms then go through this crate's current compiler and are evaluated over a grid of the
//! function's own `/Domain`, so a difference at any sample is a difference in what the file draws.
//!
//! Scanning for `/FunctionType` in the raw bytes before opening a document is sound rather than a
//! shortcut, for the reason `type4_comment_census` gives: §7.5.7 heads its exclusion list "The
//! following objects shall not be stored in an object stream:" with "Stream objects", and a type
//! 4 function is a stream.
//!
//! ```sh
//! cargo run --release -p pdf-model --example type4_operator_census -- \
//!     doc/pdf.js/test/pdfs/*.pdf doc/corpora/*/**/*.pdf doc/corpora-own/*.pdf
//! ```
//!
//! An argument beginning with `@` names a file of paths, one to a line — the `SafeDocs`
//! population is too large for a command line:
//!
//! ```sh
//! find corpus-cache -name '*.pdf' > /tmp/paths
//! cargo run --release -p pdf-model --example type4_operator_census -- @/tmp/paths
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use pdf_model::function::Function;
use pdf_syntax::{Document, Object, ObjectId};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

/// Table 42's operators, in the clause's own order, plus the two conditional ones.
///
/// Written out rather than derived from `Operator`, because the point of the census is to say
/// something about the *clause*: a name this list carries that the compiler does not know would
/// be a finding, and a name the compiler knows that the clause does not is one too.
const TABLE_42: &[&str] = &[
    "abs", "add", "atan", "ceiling", "cos", "cvi", "cvr", "div", "exp", "floor", "idiv", "ln",
    "log", "mod", "mul", "neg", "round", "sin", "sqrt", "sub", "truncate", "and", "bitshift", "eq",
    "false", "ge", "gt", "le", "lt", "ne", "not", "or", "true", "xor", "if", "ifelse", "copy",
    "dup", "exch", "index", "pop", "roll",
];

/// The three operators whose meaning this round changed or wrote down as a choice.
const AUDITED: &[&str] = &["round", "eq", "ne", "bitshift", "not"];

/// What one document contributes.
#[derive(Default)]
struct Counts {
    /// Documents whose bytes name `/FunctionType` and that opened.
    opened: usize,
    /// Documents carrying at least one type 4 function.
    with_type4: usize,
    /// Type 4 functions found.
    functions: usize,
    /// Type 4 functions whose program could not be read back as a function at all.
    unreadable: usize,
    /// Functions containing each operator.
    functions_with: BTreeMap<&'static str, usize>,
    /// Documents containing each operator.
    documents_with: BTreeMap<&'static str, usize>,
    /// For `round`, `eq` and `ne`: functions whose value the fix moved, and functions it did not.
    moved: BTreeMap<&'static str, usize>,
    /// Functions reaching an audited operator that could not be compared, and why.
    uncomparable: BTreeMap<&'static str, usize>,
    /// Where each function reaching an audited operator was found, for reading afterwards.
    witnesses: Vec<String>,
}

impl Counts {
    /// Adds `other`'s totals to this one's.
    fn absorb(&mut self, mut other: Self) {
        self.opened = self.opened.saturating_add(other.opened);
        self.with_type4 = self.with_type4.saturating_add(other.with_type4);
        self.functions = self.functions.saturating_add(other.functions);
        self.unreadable = self.unreadable.saturating_add(other.unreadable);
        for (target, source) in [
            (&mut self.functions_with, other.functions_with),
            (&mut self.documents_with, other.documents_with),
            (&mut self.moved, other.moved),
            (&mut self.uncomparable, other.uncomparable),
        ] {
            for (key, count) in source {
                let slot = target.entry(key).or_default();
                *slot = slot.saturating_add(count);
            }
        }
        self.witnesses.append(&mut other.witnesses);
    }
}

/// The program's tokens, with §7.2.4's comments cut out and §7.10.5.2's braces spaced apart.
///
/// The same two steps the compiler takes, in the same order, so that a token this returns is a
/// token the compiler saw. Cutting each line at its first PERCENT SIGN is §7.2.4's rule, and
/// §7.10.5.1's list of what the subset contains is what makes the cut safe here: with no string
/// literals in the language a PERCENT SIGN can only begin a comment. ADR 0361 has the argument.
fn tokens(program: &str) -> Vec<String> {
    let mut uncommented = String::with_capacity(program.len());
    for line in program.split(['\r', '\n']) {
        uncommented.push_str(line.split('%').next().unwrap_or_default());
        uncommented.push('\n');
    }
    uncommented
        .replace('{', " { ")
        .replace('}', " } ")
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// The program with one operator replaced by what this crate computed for it before ADR 0369.
///
/// One pass, emitting the replacement as final text rather than rescanning it, so an `eq` this
/// function *writes* is the fixed operator and an `eq` it *reads* is the one being replaced.
fn as_it_evaluated_before(tokens: &[String], operator: &str) -> String {
    // `f32::EPSILON` written out of itself: the shortest decimal that round-trips, so the
    // constant the rewritten program parses is the constant the old comparison used.
    let epsilon = format!("{:e}", f32::EPSILON);
    let replacement = match operator {
        "eq" => format!("sub abs {epsilon} lt"),
        "ne" => format!("sub abs {epsilon} ge"),
        // The fixed `round` less one where the operand is an exact tie *and* negative, which is
        // the only place the two answers part. See this file's own documentation.
        "round" => {
            "dup dup floor sub 0.5 eq exch dup 0 lt 3 1 roll round 3 1 roll and { 1 sub } if"
                .to_owned()
        }
        _ => return tokens.join(" "),
    };

    let mut out = String::with_capacity(tokens.len().saturating_mul(8));
    for token in tokens {
        if token == operator {
            out.push_str(&replacement);
        } else {
            out.push_str(token);
        }
        out.push(' ');
    }
    out
}

/// A one-object document whose object 1 is a type 4 function with `program` as its stream.
///
/// `domain` and `range` are the real function's, so both arms are evaluated and clipped exactly
/// as the file asks; without them the comparison would be of two functions neither file states.
/// `program` carries §7.10.5.2's own enclosing braces, because it is the file's token stream
/// rather than its body — wrapping it in another pair would make the whole program a procedure
/// with no `if` after it, which the compiler refuses.
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

/// The inputs the two arms are compared at: each dimension at nine points across its domain.
///
/// Nine rather than the comment census's five because what is being looked for here is a *tie*
/// and an exact equality, which a coarse grid walks straight past; an odd count puts a sample on
/// the midpoint and on both bounds. Bounded, because a grid over a six-input tint transform is
/// half a million evaluations and the point is to find *a* difference rather than to map it.
fn grid(domain: &[f32]) -> Vec<Vec<f32>> {
    /// Most sample points one comparison may spend.
    const MOST: usize = 6561;
    /// Samples across each dimension.
    const ACROSS: u8 = 9;

    let mut rows: Vec<Vec<f32>> = vec![Vec::new()];
    for pair in domain.chunks_exact(2) {
        let (low, high) = (pair.first().copied(), pair.get(1).copied());
        let (Some(low), Some(high)) = (low, high) else {
            continue;
        };
        let mut next = Vec::new();
        for row in &rows {
            for step in 0..ACROSS {
                if next.len() >= MOST {
                    return next;
                }
                let mut row = row.clone();
                row.push(low + (high - low) * f32::from(step) / f32::from(ACROSS - 1));
                next.push(row);
            }
        }
        rows = next;
    }
    rows
}

/// An array of numbers written back as PDF source.
fn as_written(values: &[f32]) -> String {
    let mut out = String::from("[");
    for value in values {
        let _ = write!(out, "{value} ");
    }
    out.push(']');
    out
}

/// Whether the fix moved what one function returns, one audited operator at a time.
///
/// Everything it learns goes into `counts`: a `moved` for a function whose value changed, and an
/// `uncomparable` for one where the file states no usable `/Domain` and `/Range`, or where an arm
/// does not compile. An uncomparable function is not a function that agrees — the two are kept
/// apart so that a number nobody could measure cannot be read as a number that came out zero.
fn compare_arms(
    path: &str,
    number: u32,
    stream_dict: &pdf_syntax::Dictionary,
    document: &Document,
    tokens: &[String],
    comparable: &[&'static str],
    counts: &mut Counts,
) {
    let give_up = |counts: &mut Counts| {
        for name in comparable {
            let slot = counts.uncomparable.entry(name).or_default();
            *slot = slot.saturating_add(1);
        }
    };

    let (domain, range) = (
        numbers(document, &document.get_key(stream_dict, "Domain")),
        numbers(document, &document.get_key(stream_dict, "Range")),
    );
    let (Some(domain), Some(range)) = (domain, range) else {
        give_up(counts);
        return;
    };
    let (domain_source, range_source) = (as_written(&domain), as_written(&range));
    let Some(now) = function_document(&tokens.join(" "), &domain_source, &range_source) else {
        give_up(counts);
        return;
    };
    let samples = grid(&domain);

    for name in comparable {
        let before = function_document(
            &as_it_evaluated_before(tokens, name),
            &domain_source,
            &range_source,
        );
        let Some(before) = before else {
            let slot = counts.uncomparable.entry(name).or_default();
            *slot = slot.saturating_add(1);
            continue;
        };
        let differs = samples
            .iter()
            .any(|inputs| now.eval(inputs) != before.eval(inputs));
        if differs {
            let slot = counts.moved.entry(name).or_default();
            *slot = slot.saturating_add(1);
            counts
                .witnesses
                .push(format!("  MOVED by {name}: {path} object {number}"));
        }
    }
}

/// What one document's type 4 functions say.
fn census(path: &str, document: &Document) -> Counts {
    let mut counts = Counts {
        opened: 1,
        ..Counts::default()
    };
    let mut seen_in_document: BTreeSet<&'static str> = BTreeSet::new();

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
            counts.unreadable = counts.unreadable.saturating_add(1);
            continue;
        };
        let program = String::from_utf8_lossy(&data).into_owned();
        let tokens = tokens(&program);

        let present: BTreeSet<&'static str> = TABLE_42
            .iter()
            .filter(|name| tokens.iter().any(|token| token == *name))
            .copied()
            .collect();
        for name in &present {
            let slot = counts.functions_with.entry(name).or_default();
            *slot = slot.saturating_add(1);
            seen_in_document.insert(name);
        }

        let audited: Vec<&'static str> = AUDITED
            .iter()
            .copied()
            .filter(|name| present.contains(name))
            .collect();
        if audited.is_empty() {
            continue;
        }
        counts
            .witnesses
            .push(format!("{path} object {number}: {}", audited.join(" ")));

        // Only the three whose semantics moved can be compared; `bitshift` and `not` are
        // unchanged code and are counted for containment alone.
        let comparable: Vec<&'static str> = audited
            .into_iter()
            .filter(|name| matches!(*name, "round" | "eq" | "ne"))
            .collect();
        if comparable.is_empty() {
            continue;
        }
        compare_arms(
            path,
            number,
            &stream.dict,
            document,
            &tokens,
            &comparable,
            &mut counts,
        );
    }

    for name in seen_in_document {
        let slot = counts.documents_with.entry(name).or_default();
        *slot = slot.saturating_add(1);
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
        "{} type 4 functions, {} of them with a stream this tree cannot decode",
        counts.functions, counts.unreadable,
    );

    println!("\nTable 42, by how many programs reach each operator:");
    println!("{:<10} {:>10} {:>10}", "operator", "functions", "documents");
    for name in TABLE_42 {
        let functions = counts.functions_with.get(name).copied().unwrap_or(0);
        let documents = counts.documents_with.get(name).copied().unwrap_or(0);
        println!("{name:<10} {functions:>10} {documents:>10}");
    }

    println!("\nWhat the fix moved, among the functions that reach each operator:");
    for name in ["round", "eq", "ne"] {
        let reached = counts.functions_with.get(name).copied().unwrap_or(0);
        let moved = counts.moved.get(name).copied().unwrap_or(0);
        let uncomparable = counts.uncomparable.get(name).copied().unwrap_or(0);
        println!(
            "  {name}: {reached} functions reach it, {moved} return a different value, \
             {uncomparable} could not be compared"
        );
    }

    for witness in &counts.witnesses {
        println!("{witness}");
    }
}
