//! What a typed operand stack costs the files that exist, rather than in principle.
//!
//! The population behind ADR 0371. ISO 32000-2 §7.10.5.1 gives a type 4 program three types —
//! "Expressions involving only integers, real numbers, and boolean values" — and this tree's
//! evaluator held one, `f32`, until the five-hundred-and-thirty-sixth session. Annex B's operand
//! columns are what that cost: `eq` and `ne` are typed `any 1 any 2` and so must answer *across*
//! the types, and with a boolean stored as `1.0` this evaluator answered `true 1 eq` with true.
//! `not`, `and`, `or` and `xor` are typed `bool | int` and only one of each pair was reachable.
//!
//! None of that can be priced from the clause. What it costs is a fact about the files that
//! exist, and this program is what counts it. It answers three questions and keeps them apart:
//!
//! - **Containment**, exact and without evaluating anything: how many type 4 functions contain
//!   each operator whose meaning the types decide, how many contain a *source* of a boolean at
//!   all, and how many contain both — the second being the only shape in which the `eq` defect
//!   can reach a colour.
//! - **Literals**, the other half of the typing: how many programs write an integer literal, how
//!   many write a real one, and how many write both. §7.3.2 and §7.3.3 are what makes a token a
//!   literal of one type or the other, and it is the file that decides rather than an inference.
//! - **Consequence**: for *every* type 4 function, whether the value it returns actually moved.
//!   This is the number that says whether a page changed.
//!
//! # How the "before" arm is built, and why it is exact
//!
//! The old semantics are reproduced as a **source rewrite** rather than by keeping the old code
//! alive — the method ADR 0361 established and ADR 0369 used — and here the rewrite is not an
//! approximation of the old evaluator but a *derivation* of it, because the old evaluator is
//! exactly this one with every value forced to a real:
//!
//! - every integer literal is written as a real, which is what `token.parse::<f32>()` made of it;
//! - `true` and `false` become `1.0` and `0.0`, which is what they were;
//! - every operator whose answer is now an integer or a boolean — `idiv`, `mod`, `bitshift`,
//!   `and`, `or`, `xor`, `cvi`, `gt`, `ge`, `lt`, `le` — is followed by `cvr`, which narrows it
//!   to the real the old arm pushed;
//! - `eq` becomes `sub 0 eq cvr` and `ne` becomes `sub 0 ne cvr`, which is numeric equality of
//!   the two operands whatever their types — the old comparison exactly;
//! - `not` becomes `0 sub 0 eq cvr`, which is the logical `not` the old arm computed for every
//!   operand including an integer one.
//!
//! With every value a real, every remaining arm computes what it computed before: `add` of two
//! reals is real addition, an `if` reads a real as false exactly when it is zero, and a pop from
//! an empty stack is a zero either way. Two edges are not covered and are stated rather than
//! hidden: an infinite operand under the rewritten `eq` gives `inf - inf`, which is a `NaN` and
//! therefore unequal where the old `==` said equal, and a value whose type changed but whose
//! number did not is invisible here by construction — that is what makes the arm exact for
//! *values*, which is what a page is made of.
//!
//! Scanning for `/FunctionType` in the raw bytes before opening a document is sound rather than a
//! shortcut: §7.5.7 heads its exclusion list "The following objects shall not be stored in an
//! object stream:" with "Stream objects", and a type 4 function is a stream.
//!
//! ```sh
//! cargo run --release -p pdf-model --example type4_type_census -- \
//!     doc/pdf.js/test/pdfs/*.pdf doc/corpora/*/**/*.pdf doc/corpora-own/*.pdf
//! ```
//!
//! An argument beginning with `@` names a file of paths, one to a line — the `SafeDocs`
//! population is too large for a command line.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use pdf_model::function::Function;
use pdf_syntax::{Document, Object, ObjectId};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

/// The operators whose meaning or whose answer's type the operand stack's types decide.
///
/// Written out rather than derived from `Operator`, for the reason ADR 0369's census gives: the
/// point is to say something about the *clause*, so the list is Annex B's rather than the
/// compiler's.
const TYPED: &[&str] = &[
    "eq", "ne", "not", "and", "or", "xor", "bitshift", "idiv", "mod", "cvi", "cvr", "truncate",
    "gt", "ge", "lt", "le", "if", "ifelse", "true", "false",
];

/// The tokens that can put a boolean on the operand stack.
///
/// `true` and `false` state one; the six relational operators answer one by §B.3's result column;
/// and `and`, `or`, `xor` and `not` answer one when they were *given* ones, so they are in this
/// list as carriers rather than as sources. A program containing none of these can hold no
/// boolean at all, and the `eq` defect cannot reach it whatever else it does.
const BOOLEAN_SOURCES: &[&str] = &[
    "true", "false", "eq", "ne", "gt", "ge", "lt", "le", "and", "or", "xor", "not",
];

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
    /// Functions containing each operator of [`TYPED`].
    functions_with: BTreeMap<&'static str, usize>,
    /// Documents containing each operator of [`TYPED`].
    documents_with: BTreeMap<&'static str, usize>,
    /// Functions matching each of the shapes the typing can be observed through.
    shapes: BTreeMap<&'static str, usize>,
    /// Functions whose value the typing moved, and functions that could not be compared.
    moved: usize,
    /// Functions where no `/Domain`, no `/Range` or no compiling arm made a comparison possible.
    uncomparable: usize,
    /// Functions compared arm against arm.
    compared: usize,
    /// Where each function of an interesting shape was found, for reading afterwards.
    witnesses: Vec<String>,
}

impl Counts {
    /// Adds `other`'s totals to this one's.
    fn absorb(&mut self, mut other: Self) {
        self.opened = self.opened.saturating_add(other.opened);
        self.with_type4 = self.with_type4.saturating_add(other.with_type4);
        self.functions = self.functions.saturating_add(other.functions);
        self.unreadable = self.unreadable.saturating_add(other.unreadable);
        self.moved = self.moved.saturating_add(other.moved);
        self.uncomparable = self.uncomparable.saturating_add(other.uncomparable);
        self.compared = self.compared.saturating_add(other.compared);
        for (target, source) in [
            (&mut self.functions_with, other.functions_with),
            (&mut self.documents_with, other.documents_with),
            (&mut self.shapes, other.shapes),
        ] {
            for (key, count) in source {
                let slot = target.entry(key).or_default();
                *slot = slot.saturating_add(count);
            }
        }
        self.witnesses.append(&mut other.witnesses);
    }

    /// Records one function of a named shape.
    fn shape(&mut self, name: &'static str) {
        let slot = self.shapes.entry(name).or_default();
        *slot = slot.saturating_add(1);
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

/// Whether a token is a literal of each of §7.3.2's and §7.3.3's two numeric types.
///
/// The same test the compiler makes, in the same order: an `i64` first, so that a token written
/// without a PERIOD is an integer, and an `f32` after it.
fn literal(token: &str) -> Option<bool> {
    if token.parse::<i64>().is_ok() {
        return Some(true);
    }
    token.parse::<f32>().ok().map(|_| false)
}

/// The program with every value forced to a real, which is what this evaluator computed before
/// its operand stack had types.
///
/// One pass, emitting each replacement as final text rather than rescanning it, so a `cvr` this
/// function writes is never read as an operator to rewrite. This file's own documentation has the
/// derivation and the two edges it does not cover.
fn as_it_evaluated_before(tokens: &[String]) -> String {
    let mut out = String::with_capacity(tokens.len().saturating_mul(8));
    for token in tokens {
        match token.as_str() {
            "true" => out.push_str("1.0"),
            "false" => out.push_str("0.0"),
            // Numeric equality of the two operands whatever their types, which is what an `f32`
            // stack could express and all it could express.
            "eq" => out.push_str("sub 0 eq cvr"),
            "ne" => out.push_str("sub 0 ne cvr"),
            // The logical `not`, which is the one of §B.3's two that an untyped stack implemented.
            "not" => out.push_str("0 sub 0 eq cvr"),
            // Every operator whose answer is now an integer or a boolean, narrowed to the real
            // the old arm pushed.
            "idiv" | "mod" | "bitshift" | "and" | "or" | "xor" | "cvi" | "gt" | "ge" | "lt"
            | "le" => {
                out.push_str(token);
                out.push_str(" cvr");
            }
            _ => match literal(token) {
                // An integer literal is what the old compiler read with `parse::<f32>()`.
                Some(true) => {
                    out.push_str(token);
                    out.push_str(".0");
                }
                _ => out.push_str(token),
            },
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
/// rather than its body.
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
/// Nine because what is being looked for is an exact equality and a branch, which a coarse grid
/// walks straight past; an odd count puts a sample on the midpoint and on both bounds. Bounded,
/// because a grid over a six-input tint transform is half a million evaluations and the point is
/// to find *a* difference rather than to map it.
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

/// Whether the typing moved what one function returns.
fn compare_arms(
    path: &str,
    number: u32,
    stream_dict: &pdf_syntax::Dictionary,
    document: &Document,
    tokens: &[String],
    counts: &mut Counts,
) {
    let (domain, range) = (
        numbers(document, &document.get_key(stream_dict, "Domain")),
        numbers(document, &document.get_key(stream_dict, "Range")),
    );
    let (Some(domain), Some(range)) = (domain, range) else {
        counts.uncomparable = counts.uncomparable.saturating_add(1);
        return;
    };
    let (domain_source, range_source) = (as_written(&domain), as_written(&range));
    let now = function_document(&tokens.join(" "), &domain_source, &range_source);
    let before = function_document(
        &as_it_evaluated_before(tokens),
        &domain_source,
        &range_source,
    );
    // A program the tree admitted before ADR 0412 and refuses now is one whose `ge`, `gt`, `lt`
    // or `le` provably reads a boolean: the rewritten arm has no boolean in it at all — `true`
    // is `1.0` there and every comparison is followed by `cvr` — so it cannot be refused for
    // that reason, and the pair is the exact population rather than the containment one. This is
    // the count the containment shapes above only bound; ADR 0412 has why the two differ.
    if now.is_none() && before.is_some() {
        counts.shape("refused: an ordering operator provably reads a boolean");
        counts
            .witnesses
            .push(format!("  REFUSED: {path} object {number}"));
    }
    let (Some(now), Some(before)) = (now, before) else {
        counts.uncomparable = counts.uncomparable.saturating_add(1);
        return;
    };

    counts.compared = counts.compared.saturating_add(1);
    let samples = grid(&domain);
    let differs = samples
        .iter()
        .any(|inputs| now.eval(inputs) != before.eval(inputs));
    if differs {
        counts.moved = counts.moved.saturating_add(1);
        counts
            .witnesses
            .push(format!("  MOVED: {path} object {number}"));
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
        let has = |name: &str| tokens.iter().any(|token| token == name);

        for name in TYPED.iter().filter(|name| has(name)) {
            let slot = counts.functions_with.entry(*name).or_default();
            *slot = slot.saturating_add(1);
            seen_in_document.insert(name);
        }

        // The literal shapes, which are where a type comes from at all.
        let integers = tokens.iter().any(|token| literal(token) == Some(true));
        let reals = tokens.iter().any(|token| literal(token) == Some(false));
        match (integers, reals) {
            (true, true) => counts.shape("both kinds of literal"),
            (true, false) => counts.shape("integer literals only"),
            (false, true) => counts.shape("real literals only"),
            (false, false) => counts.shape("no numeric literal"),
        }

        // The shapes the typing is observable through, in the order they matter.
        let boolean_source = BOOLEAN_SOURCES.iter().any(|name| has(name));
        if boolean_source {
            counts.shape("a boolean can exist at all");
        }
        if boolean_source && (has("eq") || has("ne")) {
            counts.shape("a boolean and an eq or ne — the defect's shape");
            counts.witnesses.push(format!(
                "  eq/ne with a boolean in reach: {path} object {number}"
            ));
        }
        // An ordering operator is its own boolean source — `x 0.5 gt` *makes* one rather than
        // eating one — so the containment that matters for contract answer (b) is a boolean from
        // somewhere else. The looser count is kept beside it because the difference between the
        // two is the whole reason to state it.
        let ordering = ["gt", "ge", "lt", "le"].iter().any(|name| has(name));
        if boolean_source && ordering {
            counts.shape("an ordering operator, and a boolean from anywhere");
        }
        if ordering
            && ["true", "false", "eq", "ne", "and", "or", "xor", "not"]
                .iter()
                .any(|name| has(name))
        {
            counts.shape("an ordering operator, and a boolean from elsewhere — answer (b)");
            counts.witnesses.push(format!(
                "  an ordering operator with a boolean in reach: {path} object {number}"
            ));
        }
        if has("not") {
            counts.shape("not — two operators wearing one name");
            counts
                .witnesses
                .push(format!("  not: {path} object {number}"));
        }
        if ["and", "or", "xor"].iter().any(|name| has(name)) {
            counts.shape("and, or or xor — answers in the type it was given");
        }
        if ["idiv", "mod", "bitshift"].iter().any(|name| has(name)) {
            counts.shape("an integer operator");
        }
        if has("cvi") || has("cvr") || has("truncate") {
            counts.shape("a conversion — cvi, cvr or truncate");
        }

        compare_arms(path, number, &stream.dict, document, &tokens, &mut counts);
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

    println!("\nThe operators a type decides, by how many programs contain each:");
    println!("{:<10} {:>10} {:>10}", "operator", "functions", "documents");
    for name in TYPED {
        let functions = counts.functions_with.get(name).copied().unwrap_or(0);
        let documents = counts.documents_with.get(name).copied().unwrap_or(0);
        println!("{name:<10} {functions:>10} {documents:>10}");
    }

    println!("\nThe shapes the typing can be observed through:");
    for (shape, count) in &counts.shapes {
        println!("{count:>10}  {shape}");
    }

    println!(
        "\n{} of {} functions compared arm against arm ({} uncomparable), and {} moved.",
        counts.compared, counts.functions, counts.uncomparable, counts.moved,
    );

    if !counts.witnesses.is_empty() {
        println!("\nWitnesses:");
        for witness in &counts.witnesses {
            println!("{witness}");
        }
    }
}
