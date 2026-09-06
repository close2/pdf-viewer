//! Where in the world a **real** stands at an entry ISO 32000-2 types as an **integer**.
//!
//! §7.3.3 closes the question for the writer and says nothing to the reader:
//!
//! > A real number shall not be present when an integer is expected.
//!
//! ADR 0371 answered that sentence once, for §7.10.5's calculator, and ADR 0904 answered it
//! again for Table 87's `/Width` and `/Height`. Whether the answer travels to every other
//! integer-typed entry is `doc/questions/Q31`, and a policy about the whole tree needs a
//! population rather than a witness. This is the instrument that measures it.
//!
//! # The population is the standard's typing, not a list somebody wrote
//!
//! Trap 25: a hand-written list of "the integer entries" can name a key that does not exist and
//! read as a pass. So the key set is **derived** from the Arlington model (`pdf_spec`), which
//! carries ISO 32000-2's own type column for every key of every object, and the predicate is:
//!
//! - some row naming this key types it `integer` or `bitmask`, and
//! - **no** row naming this key admits `number` anywhere in the model.
//!
//! The second half is what makes a real at such a key a §7.3.3 departure rather than an
//! ordinary value: if any table anywhere would accept a number there, a real is admissible and
//! this census must not count it. Keys typed `integer` in one table and `number` in another are
//! therefore *not judged*, because a dictionary's Arlington object cannot be decided without
//! walking the link graph and guessing it would be worse than not asking — they are counted in
//! a table of their own instead, with witnesses, so that the reading is a person's rather than
//! this file's. **`/Width` and `/Height` are two of them**: Table 87 types both `integer` on an
//! image, and §14.8.5.4's layout attribute of the same name is `number` or `name`, so the
//! entries ADR 0904 is about reach the judged half only through the inline-image list below,
//! which names Table 87's own entries.
//!
//! `bitmask` is kept beside `integer` rather than folded into it: Arlington draws that
//! distinction itself — "[a]n integer used as a set of flags" — and it is the distinction ADR
//! 0912's second family turns on.
//!
//! # Two halves, because an inline image is not an object
//!
//! The only witness this project has — `qpdf-278-0.pdf`, ADR 0904 — writes `/W 1062.00` inside
//! a content stream's `BI` … `ID`, where no cross-reference table can reach it. A census that
//! walked only indirect objects would report zero on the one document it was built for. So:
//!
//! 1. **every object** of the cross-reference table, and every dictionary nested in one; and
//! 2. **every inline image dictionary** in every stream a lexer is ever pointed at, read with
//!    §8.9.7 Table 89's abbreviations beside their long forms.
//!
//! The second half's predicate is this file's own (trap 8): the tokens between `BI` and `ID`
//! are read here, from [`pdf_syntax::Lexer`], and nothing asks `pdf_model::inline_image` what
//! it made of them — a census whose predicate is the code under test measures the code.
//!
//! ```sh
//! cargo run --release -p pdf-model --example integer_entry_census -- <file.pdf>…
//! cargo run --release -p pdf-model --example integer_entry_census -- @<list-of-paths>
//! cargo run --release -p pdf-model --example integer_entry_census -- --list-keys
//! ```
//!
//! `--list-keys` prints the derived population itself, which is the half of a census that is
//! usually invisible: a reader who cannot see which keys were asked about cannot tell a zero
//! that means "nobody writes a real there" from a zero that means "nothing looked".

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::{BTreeMap, BTreeSet};

use pdf_spec::{KeyPattern, PdfType};
use pdf_syntax::{Dictionary, Document, Lexer, Object, ObjectId, Stream, Token};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

/// How deep a nested dictionary or array is followed inside one object.
const MAX_DEPTH: usize = 32;

/// How many name/value pairs are read between `BI` and `ID` before the scan gives up.
///
/// §8.9.7 lists eleven entries and Table 87 the entries they abbreviate; a run of names longer
/// than this is not an image dictionary, it is bytes that happen to lex.
const MAX_INLINE_ENTRIES: usize = 64;

/// How many witnesses are kept per key, unless `--witnesses N` asks for more.
///
/// Six is enough to read a shape and short enough to print. A round measuring the *reach* of a
/// change wants the whole list instead — the population that can move is the set of documents a
/// key's real actually appears in — which is what the switch is for.
const MAX_WITNESSES: usize = 6;

/// The witness ceiling this run is under.
fn witness_ceiling() -> usize {
    let mut arguments = std::env::args();
    while let Some(argument) = arguments.next() {
        if argument == "--witnesses" {
            return arguments
                .next()
                .and_then(|n| n.parse().ok())
                .unwrap_or(MAX_WITNESSES);
        }
    }
    MAX_WITNESSES
}

/// What the model says about one key name, summed over every row that names it.
#[derive(Debug, Default, Clone, Copy)]
struct Typing {
    /// Rows typing it `integer`.
    integer: usize,
    /// Rows typing it `bitmask`.
    bitmask: usize,
    /// Rows admitting `number`, which is what disqualifies a key from this census.
    number: usize,
}

/// The key names at which the Arlington model admits no real anywhere.
///
/// Returns the admissible set and, beside it, the names the model types as an integer
/// *somewhere* and as a number elsewhere — the ones this instrument declines to judge, counted
/// in their own table so that a reader can see what was left out.
fn real_forbidden_keys() -> (
    BTreeMap<&'static str, Typing>,
    BTreeMap<&'static str, Typing>,
) {
    let mut typing: BTreeMap<&'static str, Typing> = BTreeMap::new();
    for object in pdf_spec::OBJECTS {
        for spec in object.keys {
            let KeyPattern::Name(name) = spec.pattern else {
                continue;
            };
            let slot = typing.entry(name).or_default();
            for alternative in spec.types {
                match alternative.kind {
                    PdfType::Integer => slot.integer = slot.integer.saturating_add(1),
                    PdfType::Bitmask => slot.bitmask = slot.bitmask.saturating_add(1),
                    PdfType::Number => slot.number = slot.number.saturating_add(1),
                    _ => {}
                }
            }
        }
    }
    let mut unjudged: BTreeMap<&'static str, Typing> = BTreeMap::new();
    for (name, facts) in &typing {
        if facts.number > 0 && (facts.integer > 0 || facts.bitmask > 0) {
            unjudged.insert(name, *facts);
        }
    }
    typing.retain(|_, facts| facts.number == 0 && (facts.integer > 0 || facts.bitmask > 0));
    (typing, unjudged)
}

/// §8.9.7 Table 89's abbreviations for the integer-typed entries of Table 87, beside the long
/// forms the same clause permits ("[t]he abbreviated names … shall be used"; the full names are
/// what a producer writes when it ignores that).
///
/// `/L` is Errata Collection 3's addition to the table and is `/Length`.
const INLINE_INTEGER_KEYS: [(&[u8], &str); 8] = [
    (b"W", "Width"),
    (b"Width", "Width"),
    (b"H", "Height"),
    (b"Height", "Height"),
    (b"BPC", "BitsPerComponent"),
    (b"BitsPerComponent", "BitsPerComponent"),
    (b"L", "Length"),
    (b"Length", "Length"),
];

/// Which of the three tables a real is recorded in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Table {
    /// An entry the model types integer or bitmask and never number.
    Object,
    /// An inline image entry §8.9.7 and Table 87 type as an integer.
    Inline,
    /// A key the model types integer in one table and number in another.
    Unjudged,
}

/// One document's findings, and the totals they sum into.
#[derive(Debug)]
struct Counts {
    /// Documents opened.
    opened: usize,
    /// Documents in which at least one real stands at an integer-typed entry.
    documents_with_a_real: usize,
    /// Integer-typed entries seen holding an integer, which is the control denominator.
    integers: u64,
    /// Integer-typed entries seen holding a real.
    reals: u64,
    /// Of those, the ones whose value has no fractional part — `1062.00` rather than `1062.5`.
    reals_integral: u64,
    /// Per key: the entries seen holding a real, and how many documents carry one.
    by_key: BTreeMap<String, (u64, usize)>,
    /// Inline image entries holding a real, by long-form key name.
    inline_by_key: BTreeMap<String, (u64, usize)>,
    /// Inline image entries holding an integer.
    inline_integers: u64,
    /// Reals at a key the model types integer in one table and number in another.
    unjudged_reals: u64,
    /// Per key, the same, so that the reading of each is a person's.
    unjudged_by_key: BTreeMap<String, (u64, usize)>,
    /// Witnesses, at most [`MAX_WITNESSES`] a key.
    witnesses: BTreeMap<String, Vec<String>>,
    /// Streams whose bytes could not be decoded, so their inline images were not seen.
    undecodable_streams: u64,
    /// How many witnesses this run keeps per key.
    ceiling: usize,
}

impl Default for Counts {
    fn default() -> Self {
        Self {
            opened: 0,
            documents_with_a_real: 0,
            integers: 0,
            reals: 0,
            reals_integral: 0,
            by_key: BTreeMap::new(),
            inline_by_key: BTreeMap::new(),
            inline_integers: 0,
            unjudged_reals: 0,
            unjudged_by_key: BTreeMap::new(),
            witnesses: BTreeMap::new(),
            undecodable_streams: 0,
            ceiling: witness_ceiling(),
        }
    }
}

impl Counts {
    /// Folds `other` in.
    fn absorb(&mut self, other: Self) {
        self.opened = self.opened.saturating_add(other.opened);
        self.documents_with_a_real = self
            .documents_with_a_real
            .saturating_add(other.documents_with_a_real);
        self.integers = self.integers.saturating_add(other.integers);
        self.reals = self.reals.saturating_add(other.reals);
        self.reals_integral = self.reals_integral.saturating_add(other.reals_integral);
        self.inline_integers = self.inline_integers.saturating_add(other.inline_integers);
        self.unjudged_reals = self.unjudged_reals.saturating_add(other.unjudged_reals);
        for (key, (entries, documents)) in other.unjudged_by_key {
            let slot = self.unjudged_by_key.entry(key).or_default();
            slot.0 = slot.0.saturating_add(entries);
            slot.1 = slot.1.saturating_add(documents);
        }
        self.undecodable_streams = self
            .undecodable_streams
            .saturating_add(other.undecodable_streams);
        for (key, (entries, documents)) in other.by_key {
            let slot = self.by_key.entry(key).or_default();
            slot.0 = slot.0.saturating_add(entries);
            slot.1 = slot.1.saturating_add(documents);
        }
        for (key, (entries, documents)) in other.inline_by_key {
            let slot = self.inline_by_key.entry(key).or_default();
            slot.0 = slot.0.saturating_add(entries);
            slot.1 = slot.1.saturating_add(documents);
        }
        for (key, witnesses) in other.witnesses {
            let slot = self.witnesses.entry(key).or_default();
            for witness in witnesses {
                if slot.len() < self.ceiling {
                    slot.push(witness);
                }
            }
        }
    }

    /// Records one real at `key`, from `path`.
    fn real(&mut self, key: &str, value: f64, path: &str, where_: &str, table: Table) {
        if table == Table::Unjudged {
            self.unjudged_reals = self.unjudged_reals.saturating_add(1);
        } else {
            self.reals = self.reals.saturating_add(1);
            if value.is_finite() && value.fract() == 0.0 {
                self.reals_integral = self.reals_integral.saturating_add(1);
            }
        }
        let counter = match table {
            Table::Object => &mut self.by_key,
            Table::Inline => &mut self.inline_by_key,
            Table::Unjudged => &mut self.unjudged_by_key,
        };
        let slot = counter.entry(key.to_owned()).or_default();
        slot.0 = slot.0.saturating_add(1);
        // Keyed by table *and* name, not by name alone: `/Width` reaches this function from
        // two tables at once — Table 87's inline dimension and §14.8.5.4's layout attribute —
        // and a single map let the second crowd the first out of a six-deep witness list, which
        // is how the run before this one lost sight of one of its two inline witnesses.
        let witnesses = self
            .witnesses
            .entry(format!("{table:?}/{key}"))
            .or_default();
        if witnesses.len() < self.ceiling {
            witnesses.push(format!("  /{key} {value} — {path} ({where_})"));
        }
    }
}

/// The two key sets one walk asks about.
struct Population {
    /// Keys the model types integer or bitmask and never number.
    judged: BTreeMap<&'static str, Typing>,
    /// Keys the model types integer somewhere and number elsewhere.
    unjudged: BTreeMap<&'static str, Typing>,
}

/// Walks one object's dictionaries, counting what stands at each integer-typed key.
fn walk(
    document: &Document,
    keys: &Population,
    object: &Object,
    depth: usize,
    path: &str,
    where_: &str,
    counts: &mut Counts,
) {
    if depth > MAX_DEPTH {
        return;
    }
    match object {
        Object::Dictionary(dict) => {
            walk_dictionary(document, keys, dict, depth, path, where_, counts);
        }
        Object::Stream(stream) => {
            walk_dictionary(document, keys, &stream.dict, depth, path, where_, counts);
        }
        Object::Array(items) => {
            for item in items {
                walk(
                    document,
                    keys,
                    item,
                    depth.saturating_add(1),
                    path,
                    where_,
                    counts,
                );
            }
        }
        _ => {}
    }
}

/// The dictionary half of [`walk`], split out because a stream reaches it by another route.
fn walk_dictionary(
    document: &Document,
    keys: &Population,
    dict: &Dictionary,
    depth: usize,
    path: &str,
    where_: &str,
    counts: &mut Counts,
) {
    for (name, value) in dict.iter() {
        if let Some(key) = name.as_str() {
            let table = if keys.judged.contains_key(key) {
                Some(Table::Object)
            } else if keys.unjudged.contains_key(key) {
                Some(Table::Unjudged)
            } else {
                None
            };
            if let Some(table) = table {
                // Resolved only at a key the model types as an integer, which keeps the walk
                // from paying for a resolution at every entry of every dictionary in the file.
                let resolved = match value {
                    Object::Reference(_) => document.resolve(value),
                    other => other.clone(),
                };
                match resolved {
                    Object::Integer(_) if table == Table::Object => {
                        counts.integers = counts.integers.saturating_add(1);
                    }
                    Object::Real(real) => counts.real(key, real, path, where_, table),
                    _ => {}
                }
            }
        }
        walk(
            document,
            keys,
            value,
            depth.saturating_add(1),
            path,
            where_,
            counts,
        );
    }
}

/// Reads the inline image dictionaries of one decoded content stream.
///
/// The scan is this file's own: from a `BI` keyword it reads name/value token pairs until `ID`,
/// then steps over the image data by looking for a delimited `EI` — which is the only way a
/// token scanner can tell a sample byte from an operator, and is stated rather than assumed.
fn scan_inline_images(bytes: &[u8], path: &str, counts: &mut Counts, seen: &mut BTreeSet<String>) {
    let mut lexer = Lexer::new(bytes);
    while let Some(token) = lexer.next_token() {
        if !matches!(token, Token::Keyword(b"BI")) {
            continue;
        }
        for _ in 0..MAX_INLINE_ENTRIES {
            let Some(Token::Name(name)) = lexer.next_token() else {
                break;
            };
            let Some(value) = lexer.next_token() else {
                break;
            };
            let long = INLINE_INTEGER_KEYS
                .iter()
                .find(|(abbreviation, _)| *abbreviation == name.as_slice())
                .map(|(_, long)| *long);
            let Some(long) = long else {
                continue;
            };
            match value {
                Token::Integer(_) => {
                    counts.inline_integers = counts.inline_integers.saturating_add(1);
                }
                Token::Real(real) => {
                    counts.real(long, real, path, "inline image", Table::Inline);
                    seen.insert(long.to_owned());
                }
                _ => {}
            }
        }
        // Past `ID`, every byte is a sample until a delimited `EI`, so the lexer is moved
        // rather than run. Where no such marker stands, this stream has nothing more to say.
        match skip_inline_data(bytes, lexer.position()) {
            Some(resume) => lexer.seek(resume),
            None => return,
        }
    }
}

/// The offset past the next delimited `EI` at or after `from`, if there is one.
fn skip_inline_data(bytes: &[u8], from: usize) -> Option<usize> {
    let mut at = from;
    while let Some(found) = bytes.get(at..)?.windows(2).position(|pair| pair == b"EI") {
        let start = at.saturating_add(found);
        let before = start.checked_sub(1).and_then(|index| bytes.get(index));
        let after = bytes.get(start.saturating_add(2));
        let delimited = before.is_none_or(|byte| pdf_syntax::lexer::is_whitespace(*byte))
            && after.is_none_or(|byte| !pdf_syntax::lexer::is_regular(*byte));
        if delimited {
            return Some(start.saturating_add(2));
        }
        at = start.saturating_add(2);
    }
    None
}

/// Every object a page names in `/Contents`, which §7.7.3.3 lets be a stream or an array.
fn content_streams(document: &Document) -> BTreeSet<ObjectId> {
    let mut out = BTreeSet::new();
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
        match node.get("Contents") {
            Some(Object::Reference(id)) => {
                out.insert(*id);
            }
            Some(Object::Array(items)) => {
                out.extend(items.iter().filter_map(|item| match item {
                    Object::Reference(id) => Some(*id),
                    _ => None,
                }));
            }
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

/// Whether a lexer is ever pointed at this stream's decoded bytes, on its dictionary's word.
fn is_lexed(
    document: &Document,
    stream: &Stream,
    id: ObjectId,
    contents: &BTreeSet<ObjectId>,
) -> bool {
    let name_is = |key: &str, want: &str| {
        document
            .get_key(&stream.dict, key)
            .as_name()
            .is_some_and(|name| name.as_str() == Some(want))
    };
    (name_is("Type", "XObject") && name_is("Subtype", "Form")) || contents.contains(&id)
}

/// What one document says.
fn census(path: &str, document: &Document, keys: &Population) -> Counts {
    let mut counts = Counts {
        opened: 1,
        ..Counts::default()
    };
    let mut inline_seen: BTreeSet<String> = BTreeSet::new();
    let mut object_seen: BTreeSet<String> = BTreeSet::new();

    let contents = content_streams(document);
    for number in document.xref().object_numbers() {
        let id = ObjectId::new(number, 0);
        let object = document.get(id);
        let before: Vec<String> = counts.by_key.keys().cloned().collect();
        walk(
            document,
            keys,
            &object,
            0,
            path,
            &format!("object {number}"),
            &mut counts,
        );
        object_seen.extend(
            counts
                .by_key
                .keys()
                .filter(|key| !before.contains(key))
                .cloned(),
        );

        if let Some(stream) = object.as_stream()
            && is_lexed(document, stream, id, &contents)
        {
            match document.decoded_stream_data(stream) {
                Some(data) => scan_inline_images(&data, path, &mut counts, &mut inline_seen),
                None => {
                    counts.undecodable_streams = counts.undecodable_streams.saturating_add(1);
                }
            }
        }
    }

    for key in counts.by_key.keys().cloned().collect::<Vec<_>>() {
        counts.by_key.entry(key).and_modify(|slot| slot.1 = 1);
    }
    for key in counts.inline_by_key.keys().cloned().collect::<Vec<_>>() {
        counts
            .inline_by_key
            .entry(key)
            .and_modify(|slot| slot.1 = 1);
    }
    for key in counts.unjudged_by_key.keys().cloned().collect::<Vec<_>>() {
        counts
            .unjudged_by_key
            .entry(key)
            .and_modify(|slot| slot.1 = 1);
    }
    if counts.reals > 0 {
        counts.documents_with_a_real = 1;
    }
    counts
}

/// The paths to walk: arguments, and the lines of any argument beginning with `@`.
fn paths() -> Vec<String> {
    let mut out = Vec::new();
    let mut skip_next = false;
    for argument in std::env::args().skip(1) {
        if std::mem::replace(&mut skip_next, false) {
            continue;
        }
        if argument == "--list-keys" {
            continue;
        }
        if argument == "--witnesses" {
            skip_next = true;
            continue;
        }
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
    let (judged, unjudged) = real_forbidden_keys();
    let paths = paths();
    println!(
        "{} key names the Arlington model types integer or bitmask and never number; \
         {} more are integer in one table and number in another, counted but not judged.",
        judged.len(),
        unjudged.len(),
    );
    if std::env::args().any(|argument| argument == "--list-keys") {
        for (key, facts) in &judged {
            println!(
                "  judged   /{key:<24} {} integer row(s), {} bitmask row(s)",
                facts.integer, facts.bitmask
            );
        }
        for (key, facts) in &unjudged {
            println!(
                "  unjudged /{key:<24} {} integer, {} bitmask, {} number row(s)",
                facts.integer, facts.bitmask, facts.number
            );
        }
    }
    let keys = Population { judged, unjudged };

    let counts = paths
        .par_iter()
        .map(|path| {
            let mut counts = Counts::default();
            let Ok(bytes) = std::fs::read(path) else {
                return counts;
            };
            let Ok(document) = Document::open(bytes) else {
                return counts;
            };
            counts.absorb(census(path, &document, &keys));
            counts
        })
        .reduce(Counts::default, |mut total, counts| {
            total.absorb(counts);
            total
        });

    println!(
        "\n{} path(s), {} opened, {} carrying a real where the model admits none.",
        paths.len(),
        counts.opened,
        counts.documents_with_a_real,
    );
    println!(
        "{} integer-typed entries held an integer, {} held a real ({} of them with no \
         fractional part).",
        counts.integers, counts.reals, counts.reals_integral,
    );
    println!(
        "{} inline image entries held an integer; {} stream(s) could not be decoded, so their \
         inline images were not read.",
        counts.inline_integers, counts.undecodable_streams,
    );

    println!("\nBy key, in indirect and direct objects:");
    for (key, (entries, documents)) in &counts.by_key {
        println!("  {key:<24} {entries:>8} entries in {documents:>6} document(s)");
    }
    if counts.by_key.is_empty() {
        println!("  (none)");
    }

    println!(
        "\n{} real(s) at a key the model types integer in one table and number in another — \
         counted, not judged:",
        counts.unjudged_reals,
    );
    for (key, (entries, documents)) in &counts.unjudged_by_key {
        println!("  {key:<24} {entries:>8} entries in {documents:>6} document(s)");
    }
    if counts.unjudged_by_key.is_empty() {
        println!("  (none)");
    }

    println!("\nBy key, in inline image dictionaries:");
    for (key, (entries, documents)) in &counts.inline_by_key {
        println!("  {key:<24} {entries:>8} entries in {documents:>6} document(s)");
    }
    if counts.inline_by_key.is_empty() {
        println!("  (none)");
    }

    if !counts.witnesses.is_empty() {
        println!("\nWitnesses:");
        for witnesses in counts.witnesses.values() {
            for witness in witnesses {
                println!("{witness}");
            }
        }
    }
}
