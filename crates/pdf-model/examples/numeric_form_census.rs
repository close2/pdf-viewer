//! Which of ISO 32000-2 §7.3.3's numeric forms real documents actually write, and what they
//! write instead.
//!
//! §7.3.3 states two forms and closes them. An integer:
//!
//! > An integer shall be written as one or more decimal digits optionally preceded by a sign.
//!
//! a real:
//!
//! > A real value shall be written as one or more decimal digits with an optional sign and a
//! > leading, trailing, or embedded PERIOD (2Eh) (decimal point).
//!
//! and the sentence that says what is *not* a number:
//!
//! > A PDF writer shall not use the PostScript language syntax for numbers with non-decimal
//! > radices (such as 16#FFFE) or in exponential format (such as 6.02E23).
//!
//! Errata Collection 3's Issue #327 adds a railroad diagram of both forms above each EXAMPLE,
//! and it closes the grammar the same way: an optional `+` or `-`, then `DecimalDigit`s, with
//! at most one PERIOD for the real form and no other production anywhere in either figure.
//!
//! The two departures this counts are therefore different in kind, and the count is what
//! decides what a reader owes each of them:
//!
//! - **A form the clause forbids a writer to use.** The exponent, the radix, and the
//!   malformed spellings (`--5`, `1.2.3`) that no sentence of §7.3.3 describes at all.
//! - **A form the clause admits whose *value* a double cannot hold.** §7.3.3 says outright
//!   that "[t]he range and precision of numbers may be limited by the internal representations
//!   used in the computer on which the PDF processor is running", so this one is not the
//!   file's defect — it is a conforming number meeting this reader's arithmetic.
//!
//! **Its predicate is its own** (trap 8): the runs are split by §7.2.3's byte classes and
//! classified against §7.3.3's grammar written out here, never by asking
//! `pdf_syntax::Lexer` what it made of them. A census whose predicate is the code under test
//! measures the code.
//!
//! **The population is every stream a lexer is ever pointed at**, identified by its own
//! dictionary: §7.8.2's content streams reached from each page's `/Contents`, §8.10's form
//! `XObject`s, §9.7.5's `CMap`s, §7.10.5's calculator functions, and §7.5.7's object streams —
//! which is where every file written this century keeps the body's own direct objects.
//!
//! It is deliberately **not** every stream, and not the file's raw bytes. An encoded stream's
//! data, a font program's tables and a hexadecimal string's digits are runs of regular
//! characters that no lexer is ever asked to read as numbers, and counting them measures
//! entropy rather than producers: scanning the whole file first put 51 956 "non-decimal radix"
//! runs in front of this round, every one of them a byte of compressed image.
//!
//! Tokens are split by [`pdf_syntax::Lexer`] and classified here. That division is the point of
//! trap 8's rule rather than an exception to it: splitting is §7.2.3 and object syntax, which is
//! not what is under test, and it is what keeps a name's `#`-escape and a hexadecimal string's
//! body out of a census about §7.3.3.
//!
//! ```sh
//! cargo run --release -p pdf-model --example numeric_form_census -- <file.pdf>…
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use pdf_syntax::{Document, Lexer, Object, ObjectId, Stream, Token};

/// How a run of regular characters holding at least one decimal digit reads against §7.3.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    /// Exactly one of the clause's two forms, and a double holds the value.
    Conforming,
    /// Exactly one of the clause's two forms, and the magnitude overflows a double.
    Unrepresentable,
    /// The clause's grammar followed by an exponent — the form the `shall not` names.
    Exponent,
    /// An exponent whose value also overflows a double.
    ExponentUnrepresentable,
    /// A non-decimal radix, the other form the `shall not` names.
    Radix,
    /// Holds a digit and is neither form: `--5`, `1.2.3`, `12pt`.
    Malformed,
}

/// What one document wrote.
#[derive(Default)]
struct Finding {
    /// Runs of each form, in the order [`Form`] states them.
    counts: [usize; 6],
    /// One example of each departure, as written.
    examples: [Option<String>; 6],
}

impl Finding {
    /// Records one run.
    fn record(&mut self, form: Form, raw: &[u8]) {
        let slot = form as usize;
        self.counts[slot] = self.counts[slot].saturating_add(1);
        if self.examples[slot].is_none() {
            self.examples[slot] = Some(String::from_utf8_lossy(raw).into_owned());
        }
    }
}

fn main() {
    let mut opened = 0_usize;
    let mut totals = [0_usize; 6];
    let mut documents = [0_usize; 6];
    let mut lines: Vec<String> = Vec::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        opened = opened.saturating_add(1);
        let finding = examine(&document);
        for (slot, count) in finding.counts.iter().enumerate() {
            totals[slot] = totals[slot].saturating_add(*count);
            if *count > 0 {
                documents[slot] = documents[slot].saturating_add(1);
            }
        }
        let departures: usize = finding.counts.iter().skip(1).sum();
        if departures == 0 {
            continue;
        }
        let detail = NAMES
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(slot, _)| finding.counts[*slot] > 0)
            .map(|(slot, name)| {
                format!(
                    "{} {name} (e.g. {})",
                    finding.counts[slot],
                    finding.examples[slot].as_deref().unwrap_or("?")
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!(
            "  {path}: {detail} — of {} conforming",
            finding.counts[0]
        ));
    }

    println!("{opened} document(s) opened");
    for (slot, name) in NAMES.iter().enumerate() {
        println!(
            "  {:>12} run(s) {name}, in {} document(s)",
            totals[slot], documents[slot]
        );
    }
    for line in &lines {
        println!("{line}");
    }
}

/// What each slot of [`Finding::counts`] is called in the output.
///
/// The six are a partition of every run holding a decimal digit and they are the reading rather
/// than a tidying-up, so they carry the decision that drew them: ADR 0733.
const NAMES: [&str; 6] = [
    "in one of §7.3.3's two forms",
    "in a form the clause states and a double cannot hold",
    "in the exponential format the clause forbids a writer",
    "in the exponential format, and beyond a double",
    "with a non-decimal radix",
    "holding a digit and matching neither form",
];

/// Scans every stream of one document that a lexer is ever pointed at.
fn examine(document: &Document) -> Finding {
    let mut finding = Finding::default();
    let contents = content_streams(document);
    for number in document.xref().object_numbers() {
        let id = ObjectId::new(number, 0);
        let object = document.get(id);
        let Some(stream) = object.as_stream() else {
            continue;
        };
        if !is_lexed(document, stream, id, &contents) {
            continue;
        }
        if let Some(data) = document.decoded_stream_data(stream) {
            scan(&data, &mut finding);
        }
    }
    finding
}

/// Whether a lexer is ever pointed at this stream's decoded bytes, on its dictionary's word.
fn is_lexed(document: &Document, stream: &Stream, id: ObjectId, contents: &[ObjectId]) -> bool {
    let name_is = |key: &str, want: &str| {
        document
            .get_key(&stream.dict, key)
            .as_name()
            .is_some_and(|name| name.as_str() == Some(want))
    };
    name_is("Type", "ObjStm")
        || name_is("Type", "CMap")
        || (name_is("Type", "XObject") && name_is("Subtype", "Form"))
        || !document.get_key(&stream.dict, "FunctionType").is_null()
        || contents.contains(&id)
}

/// Every object a page names in `/Contents`, which §7.7.3.3 lets be a stream or an array of them.
fn content_streams(document: &Document) -> Vec<ObjectId> {
    let mut out = Vec::new();
    let Ok(catalog) = document.catalog() else {
        return out;
    };
    let Some(root) = document.get_key(&catalog, "Pages").as_dict().cloned() else {
        return out;
    };
    let mut queue = vec![(root, 0_usize)];
    let mut visited = 0_usize;
    while let Some((node, depth)) = queue.pop() {
        visited = visited.saturating_add(1);
        if depth > 64 || visited > 100_000 {
            break;
        }
        // The *unresolved* value, because what is wanted is the identifier and `get_key`
        // resolves one away.
        match node.get("Contents") {
            Some(Object::Reference(id)) => out.push(*id),
            Some(Object::Array(items)) => out.extend(items.iter().filter_map(|item| match item {
                Object::Reference(id) => Some(*id),
                _ => None,
            })),
            _ => {}
        }
        if let Object::Array(kids) = document.get_key(&node, "Kids") {
            for kid in &kids {
                if let Some(dict) = document.resolve(kid).as_dict() {
                    queue.push((dict.clone(), depth.saturating_add(1)));
                }
            }
        }
    }
    out
}

/// Tokenises `bytes` and classifies the span of every token that could have been a number.
///
/// A number token always spans a whole regular run — the fast path in `Lexer::read_number`
/// only stands when the byte that stopped it also ends the token — so the span between the
/// cursor before and after is exactly what §7.3.3 was asked to read.
fn scan(bytes: &[u8], finding: &mut Finding) {
    let mut lexer = Lexer::new(bytes);
    loop {
        lexer.skip_whitespace();
        let start = lexer.position();
        let Some(token) = lexer.next_token() else {
            break;
        };
        // A keyword is in the population because a run this reader refuses to read as a number
        // is still a run a producer may have meant as one; a name, a string and a delimiter are
        // not, whatever digits they hold.
        if !matches!(
            token,
            Token::Integer(_) | Token::Real(_) | Token::Keyword(_)
        ) {
            continue;
        }
        let run = bytes.get(start..lexer.position()).unwrap_or_default();
        if let Some(form) = classify(run) {
            finding.record(form, run);
        }
    }
}

/// Reads a run against §7.3.3's grammar, written out here rather than asked of the lexer.
fn classify(run: &[u8]) -> Option<Form> {
    if !run.iter().any(u8::is_ascii_digit) {
        return None;
    }
    let (sign, body) = match run.split_first() {
        Some((b'+' | b'-', rest)) => (1_usize, rest),
        _ => (0, run),
    };
    // The clause's two forms together: decimal digits with at most one PERIOD anywhere in them.
    let mut digits = 0_usize;
    let mut points = 0_usize;
    let mut read = 0_usize;
    while let Some(&byte) = body.get(read) {
        match byte {
            b'0'..=b'9' => digits = digits.saturating_add(1),
            b'.' if points == 0 => points = 1,
            _ => break,
        }
        read = read.saturating_add(1);
    }
    if digits == 0 {
        return None;
    }
    let value = std::str::from_utf8(run.get(..read.saturating_add(sign))?)
        .ok()?
        .parse::<f64>()
        .ok()?;
    if read == body.len() {
        return Some(if value.is_finite() {
            Form::Conforming
        } else {
            Form::Unrepresentable
        });
    }
    if run.contains(&b'#') {
        return Some(Form::Radix);
    }
    // An exponent is the grammar above followed by `e` or `E`, an optional sign, and one or
    // more digits, with nothing after them.
    let tail = body.get(read..).unwrap_or_default();
    let Some((b'e' | b'E', after)) = tail.split_first() else {
        return Some(Form::Malformed);
    };
    let after = match after.split_first() {
        Some((b'+' | b'-', rest)) => rest,
        _ => after,
    };
    if after.is_empty() || !after.iter().all(u8::is_ascii_digit) {
        return Some(Form::Malformed);
    }
    let whole = std::str::from_utf8(run).ok()?.parse::<f64>().ok()?;
    Some(if whole.is_finite() {
        Form::Exponent
    } else {
        Form::ExponentUnrepresentable
    })
}
