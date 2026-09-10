//! Counts what ISO 19005's unreferenced-named-resource exemption would take off this crate.
//!
//! ```sh
//! cargo run --release -p pdf-archive --example unreferenced -- doc/veraPDF-corpus
//! ```
//!
//! # The sentence being measured
//!
//! ISO 19005-2 section 6.2.2 and ISO 19005-4 section 6.2.2 both close with an exemption: a named
//! resource present in a resources dictionary whose name the associated content stream never
//! references is not used for rendering, and is therefore exempt from the part's requirements —
//! wholly in part 2 as published, and in part 4 with sections 6.1.6 to 6.1.9 carved back out.
//! `TechNote 0010` A010 narrows part 2's to a comparable shape. **No row of this crate states
//! it**, and the rows that walk every object a cross-reference section names judge an
//! unreferenced image, font or filter like any other. ADR 0935 argued that implementing it is a
//! whole-file reachability problem rather than a predicate, and deferred it; this is the
//! instrument that says how much is being deferred, so the size is counted rather than recalled.
//! `doc/todo/62`.
//!
//! # What it counts
//!
//! For each document it computes the objects reachable **only** through a named-resource entry
//! the associated content stream does not reference, then asks which of the report's failures
//! land on nothing else. The reachability is the test ADR 0935 says a real implementation owes
//! and that no per-row predicate can make: an object is exempt only when *every* route to it
//! from the trailer passes through such an entry.
//!
//! The association is read as ISO 32000-2 §7.8.3 and `TechNote 0010` A003 draw it — a resources
//! dictionary belongs to the stream that states it, to the page whose `/Contents` it governs, or
//! to the glyph procedures of the Type 3 font that states it — and never to a page that inherits
//! it, which is A003's own sentence. A resources dictionary with no such owner exempts nothing
//! here: an `/AcroForm` `/DR` has no associated content stream, so the clause's premise fails.
//!
//! One approximation, and it runs one way: **every name operand of a content stream counts as a
//! reference**, with no operator table, so `/F1` reaching a `Tf` and `/F1` reaching nothing are
//! the same token. That can only make the exempt set smaller, so a document this prints is a
//! candidate and a document it prints for no row may still hold a resource a stricter reading of
//! *referenced* would free.

#![expect(
    clippy::print_stdout,
    reason = "an example whose whole product is a count a person reads"
)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use pdf_archive::{Flavour, Level, Outcome, Target, check};
use pdf_syntax::{Dictionary, Document, FileBytes, Lexer, Object, ObjectId, Token};

/// The resources dictionary's own subdictionaries, ISO 32000-2 §7.8.3's Table 34.
///
/// `/ProcSet` is absent: its value is an array of names rather than a dictionary of named
/// resources, so it holds no entry this exemption could be about.
const CATEGORIES: [&str; 7] = [
    "ExtGState",
    "ColorSpace",
    "Pattern",
    "Shading",
    "XObject",
    "Font",
    "Properties",
];

/// The names one content stream, or one owner's content streams, state as operands.
type Names = BTreeSet<Vec<u8>>;

/// Where in a document's object graph one value sits, for the purpose of the exemption.
#[derive(Debug, Clone, Copy)]
enum Spot<'a> {
    /// Anywhere else — every edge from here is an ordinary reference.
    Elsewhere,
    /// A `/Resources` dictionary, with what its associated content streams reference.
    Resources(&'a Names),
    /// One category of such a dictionary, so its keys are the resource names.
    Category(&'a Names),
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let root = arguments
        .next()
        .map_or_else(|| PathBuf::from("doc/veraPDF-corpus"), PathBuf::from);
    if !root.is_dir() {
        println!("{}: not a directory", root.display());
        return;
    }
    for (folder, target) in [
        ("PDF_A-4", Target::Four(Flavour::Plain)),
        ("PDF_A-4f", Target::Four(Flavour::F)),
        ("PDF_A-4e", Target::Four(Flavour::E)),
        ("PDF_A-2b", Target::Two(Level::B)),
        ("PDF_A-2u", Target::Two(Level::U)),
        ("PDF_A-2a", Target::Two(Level::A)),
    ] {
        let corner = root.join(folder);
        if corner.is_dir() {
            sweep(folder, &corner, target);
        }
    }
}

/// One document's answer.
#[derive(Debug, Default)]
struct Measurement {
    /// Named resource entries whose name the associated content stream does not reference.
    entries: usize,
    /// Objects reachable only through such an entry.
    exempt: BTreeSet<ObjectId>,
    /// The rows that failed on nothing but exempt objects: citation and row identifier.
    only_exempt: BTreeMap<String, &'static str>,
    /// Whether some other row failed too, so that withdrawing these would not clear the verdict.
    other_failures: bool,
}

/// Runs one target's corner of the corpus and prints what the exemption would reach.
fn sweep(folder: &str, corner: &Path, target: Target) {
    let mut documents = 0_usize;
    let mut with_entries = 0_usize;
    let mut with_exempt = 0_usize;
    let mut would_pass = 0_usize;
    let mut candidates: Vec<(String, Vec<String>)> = Vec::new();
    for path in files(corner) {
        let Ok(bytes) = FileBytes::on_disk(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let measured = measure(&document, target);
        if measured.entries > 0 {
            with_entries = with_entries.saturating_add(1);
        }
        if !measured.exempt.is_empty() {
            with_exempt = with_exempt.saturating_add(1);
        }
        if measured.only_exempt.is_empty() {
            continue;
        }
        if !measured.other_failures {
            would_pass = would_pass.saturating_add(1);
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("?")
            .to_owned();
        let rows = measured
            .only_exempt
            .iter()
            .map(|(citation, id)| format!("{citation}  {id}"))
            .collect();
        candidates.push((name, rows));
    }
    println!("\n== {folder} ==");
    println!("  {documents:>5} documents read");
    println!("  {with_entries:>5} state a named resource their content stream does not reference");
    println!("  {with_exempt:>5} hold an object reachable only through such an entry");
    println!(
        "  {:>5} fail a row whose every finding is on such an object",
        candidates.len()
    );
    println!("  {would_pass:>5} of those fail nothing else, so the verdict itself turns on one");
    for (name, rows) in &candidates {
        println!("    {name}");
        for row in rows {
            println!("      {row}");
        }
    }
}

/// Every PDF under a directory, in a stable order.
fn files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|kind| kind == "pdf") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// One document's exempt objects, and which of its failures rest on them alone.
fn measure(document: &Document, target: Target) -> Measurement {
    let owners = owners(document);
    let (resources, categories) = indirect_dictionaries(document, &owners);
    let mut measurement = Measurement::default();
    let mut plain: BTreeMap<ObjectId, Vec<ObjectId>> = BTreeMap::new();
    let mut all: BTreeMap<ObjectId, Vec<ObjectId>> = BTreeMap::new();
    for number in document.xref().object_numbers() {
        let id = ObjectId::new(number, 0);
        let mut here = Edges::default();
        let spot = spot_of(id, &resources, &categories);
        edges(&document.get(id), spot, owners.get(&id), &mut here);
        measurement.entries = measurement.entries.saturating_add(here.exempt.len());
        let both: Vec<ObjectId> = here.plain.iter().chain(&here.exempt).copied().collect();
        all.insert(id, both);
        plain.insert(id, here.plain);
    }
    let mut roots = Edges::default();
    edges(
        &Object::Dictionary(document.trailer().clone()),
        Spot::Elsewhere,
        None,
        &mut roots,
    );
    let kept = reachable(&roots.plain, &plain);
    let everything = reachable(&roots.plain, &all);
    measurement.exempt = everything.difference(&kept).copied().collect();
    if measurement.exempt.is_empty() {
        return measurement;
    }
    for judgement in &check(document, target).judgements {
        let Outcome::Failed { places, total } = &judgement.outcome else {
            continue;
        };
        // A truncated list is a prefix, so "every finding is exempt" cannot be read off it, and
        // such a row counts as a failure the exemption leaves standing.
        if *total == places.len()
            && !places.is_empty()
            && places.iter().all(|place| {
                place
                    .place
                    .object
                    .is_some_and(|id| measurement.exempt.contains(&id))
            })
        {
            measurement
                .only_exempt
                .insert(judgement.citation.clone(), judgement.id);
        } else {
            measurement.other_failures = true;
        }
    }
    measurement
}

/// Every object that states a `/Resources` entry, with what its content streams reference.
///
/// The three owners A003 recognises, and no fourth: a stream — a form `XObject`, a tiling
/// pattern, an annotation appearance — is associated with its own data; a page with the streams
/// its `/Contents` names; a Type 3 font with its glyph procedures. Anything else that happens to
/// carry a `/Resources` key has no associated content stream, so it is absent here and exempts
/// nothing.
fn owners(document: &Document) -> BTreeMap<ObjectId, Names> {
    let mut found: BTreeMap<ObjectId, Names> = BTreeMap::new();
    for number in document.xref().object_numbers() {
        let id = ObjectId::new(number, 0);
        let object = document.get(id);
        let dict = match &object {
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => &stream.dict,
            _ => continue,
        };
        if matches!(document.get_key(dict, "Resources"), Object::Null) {
            continue;
        }
        let mut names = Names::new();
        if let Object::Stream(stream) = &object {
            read_names(document, &Object::Stream(stream.clone()), &mut names);
        }
        read_names(document, &document.get_key(dict, "Contents"), &mut names);
        if let Object::Dictionary(procedures) = document.get_key(dict, "CharProcs") {
            for (_, value) in procedures.iter() {
                read_names(document, &document.resolve(value), &mut names);
            }
        }
        found.insert(id, names);
    }
    found
}

/// Adds every name a content stream, or an array of them, states as an operand.
fn read_names(document: &Document, object: &Object, into: &mut Names) {
    match object {
        Object::Array(items) => {
            for item in items {
                read_names(document, &document.resolve(item), into);
            }
        }
        Object::Stream(stream) => {
            let Some(data) = document.decoded_stream_data(stream) else {
                return;
            };
            let mut lexer = Lexer::new(&data);
            while let Some(token) = lexer.next_token() {
                if let Token::Name(name) = token {
                    into.insert(name);
                }
            }
        }
        _ => {}
    }
}

/// The resources dictionaries and categories that are indirect objects of their own.
type Indirect = (BTreeMap<ObjectId, Names>, BTreeMap<ObjectId, Names>);

/// Finds them, with the names of every owner that reaches each.
///
/// Repeated until the maps stop growing, because a resources dictionary may be indirect and its
/// categories indirect again, and each level is discoverable only once the level above it is.
/// A dictionary two owners share carries the union of their names, which can only shrink the
/// exempt set — the direction the module comment states.
fn indirect_dictionaries(document: &Document, owners: &BTreeMap<ObjectId, Names>) -> Indirect {
    let mut resources: BTreeMap<ObjectId, Names> = BTreeMap::new();
    let mut categories: BTreeMap<ObjectId, Names> = BTreeMap::new();
    loop {
        let before = total(&resources).saturating_add(total(&categories));
        let mut next_resources = resources.clone();
        let mut next_categories = categories.clone();
        for number in document.xref().object_numbers() {
            let id = ObjectId::new(number, 0);
            let spot = spot_of(id, &resources, &categories);
            note(
                &document.get(id),
                spot,
                owners.get(&id),
                &mut next_resources,
                &mut next_categories,
            );
        }
        resources = next_resources;
        categories = next_categories;
        if total(&resources).saturating_add(total(&categories)) == before {
            return (resources, categories);
        }
    }
}

/// How much a map holds, counting its names, so that a fixpoint can see either half grow.
fn total(map: &BTreeMap<ObjectId, Names>) -> usize {
    map.iter()
        .fold(map.len(), |sum, (_, names)| sum.saturating_add(names.len()))
}

/// Where an object sits, given what is known so far.
fn spot_of<'a>(
    id: ObjectId,
    resources: &'a BTreeMap<ObjectId, Names>,
    categories: &'a BTreeMap<ObjectId, Names>,
) -> Spot<'a> {
    if let Some(names) = categories.get(&id) {
        Spot::Category(names)
    } else if let Some(names) = resources.get(&id) {
        Spot::Resources(names)
    } else {
        Spot::Elsewhere
    }
}

/// Records every indirect resources dictionary and category this value names.
fn note(
    object: &Object,
    spot: Spot<'_>,
    owner: Option<&Names>,
    resources: &mut BTreeMap<ObjectId, Names>,
    categories: &mut BTreeMap<ObjectId, Names>,
) {
    match object {
        Object::Array(items) => {
            for item in items {
                note(item, Spot::Elsewhere, owner, resources, categories);
            }
        }
        Object::Dictionary(dict) => note_keys(dict, spot, owner, resources, categories),
        Object::Stream(stream) => note_keys(&stream.dict, spot, owner, resources, categories),
        _ => {}
    }
}

/// The same, for one dictionary's entries.
fn note_keys(
    dict: &Dictionary,
    spot: Spot<'_>,
    owner: Option<&Names>,
    resources: &mut BTreeMap<ObjectId, Names>,
    categories: &mut BTreeMap<ObjectId, Names>,
) {
    for (key, value) in dict.iter() {
        let inner = inner_spot(spot, owner, key.0.as_ref());
        match (inner, value) {
            (Spot::Resources(names), Object::Reference(id)) => {
                resources
                    .entry(*id)
                    .or_default()
                    .extend(names.iter().cloned());
            }
            (Spot::Category(names), Object::Reference(id)) => {
                categories
                    .entry(*id)
                    .or_default()
                    .extend(names.iter().cloned());
            }
            _ => note(value, inner, owner, resources, categories),
        }
    }
}

/// Where one entry's value sits, given where its dictionary sits and what the key is.
fn inner_spot<'a>(spot: Spot<'a>, owner: Option<&'a Names>, key: &[u8]) -> Spot<'a> {
    match spot {
        Spot::Elsewhere if key == b"Resources" => owner.map_or(Spot::Elsewhere, Spot::Resources),
        Spot::Resources(names) if CATEGORIES.iter().any(|name| name.as_bytes() == key) => {
            Spot::Category(names)
        }
        // A resources dictionary's other keys, and everything below a category's entries, are
        // ordinary objects again.
        _ => Spot::Elsewhere,
    }
}

/// One value's outgoing references, split by whether the exemption would cut them.
#[derive(Debug, Default)]
struct Edges {
    /// References by a route the exemption leaves alone.
    plain: Vec<ObjectId>,
    /// References reached only through a named resource entry nothing names.
    exempt: Vec<ObjectId>,
}

/// Collects one value's outgoing references, classifying each.
fn edges(object: &Object, spot: Spot<'_>, owner: Option<&Names>, into: &mut Edges) {
    match object {
        Object::Reference(id) => into.plain.push(*id),
        Object::Array(items) => {
            for item in items {
                edges(item, Spot::Elsewhere, owner, into);
            }
        }
        Object::Dictionary(dict) => entry_edges(dict, spot, owner, into),
        Object::Stream(stream) => entry_edges(&stream.dict, spot, owner, into),
        _ => {}
    }
}

/// The same, for one dictionary's entries.
fn entry_edges(dict: &Dictionary, spot: Spot<'_>, owner: Option<&Names>, into: &mut Edges) {
    for (key, value) in dict.iter() {
        if let Spot::Category(names) = spot
            && !names.contains(key.0.as_ref())
        {
            // A direct value is no object of its own, so nothing is exempted by it — but
            // whatever it references is reached only through this entry.
            let mut below = Edges::default();
            edges(value, Spot::Elsewhere, owner, &mut below);
            into.exempt.extend(below.plain);
            into.exempt.extend(below.exempt);
            continue;
        }
        edges(value, inner_spot(spot, owner, key.0.as_ref()), owner, into);
    }
}

/// Every object reachable from the roots along the given edges.
fn reachable(roots: &[ObjectId], graph: &BTreeMap<ObjectId, Vec<ObjectId>>) -> BTreeSet<ObjectId> {
    let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
    let mut queue: VecDeque<ObjectId> = roots.iter().copied().collect();
    while let Some(id) = queue.pop_front() {
        if !seen.insert(id) {
            continue;
        }
        for next in graph.get(&id).into_iter().flatten() {
            if !seen.contains(next) {
                queue.push_back(*next);
            }
        }
    }
    seen
}
