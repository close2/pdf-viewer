//! Table 382's `/ContinuedList` and `/ContinuedFrom`: how many lists say they continue another.
//!
//! ISO 32000-2 §14.8.5.5 puts three attributes in Table 382 and addresses two of them to whoever
//! *interprets* an `L` element rather than to whoever lays one out:
//!
//! > The ContinuedList and the ContinuedFrom attributes described in "Table 382 -Standard list
//! > attributes" control the interpretation of the L element as it relates to other L elements
//! > that are not its immediate parent.
//!
//! `/ContinuedList` is "[a] flag specifying whether the list is a continuation of a previous list
//! in the structure tree", and `/ContinuedFrom` is "[t]he ID … of the list for which this list is
//! a continuation". Where the flag is set and no identifier is stated, the clause names the
//! predecessor itself: "the continuation is from the preceding list at the same level in the
//! structure hierarchy".
//!
//! This counts, over whatever documents it is given:
//!
//! - `L` elements, so the two counts below have a denominator;
//! - `L` elements stating `/ContinuedList`, and how many of those state it **true**;
//! - `L` elements stating `/ContinuedFrom`, and how many of those identifiers name an element the
//!   same walk reached — the resolution a reader can actually make;
//! - continuing lists with no `/ContinuedFrom` for which the clause's own fallback finds a
//!   preceding list at the same depth;
//! - and the probe that keeps a clean zero honest: elements stating either entry that are **not**
//!   `L` elements, which is a document saying something §14.8.5.5 does not define.
//!
//! The last one is trap 11's rule applied to a census rather than to a report. A run that counted
//! only well-placed attributes and printed zero would look identical whether the population is
//! empty or the reader is looking in the wrong place, and the two need telling apart.
//!
//! ```sh
//! cargo run --release -p pdf-model --example list_continuation_census -- \
//!   $(find doc/pdf.js/test/pdfs -maxdepth 1 -name '*.pdf') \
//!   $(find -L doc/corpora corpus-cache -name '*.pdf') doc/*.pdf
//! ```
#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is the measurement"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "counters over a corpus four orders of magnitude below what a usize counts, and \
              this is a measurement rather than a shipped path"
)]

use rayon::prelude::*;

use pdf_model::structure::{
    Child, ListContinuation, ListEntry, StandardType, Tree, list_predecessors,
};
use pdf_syntax::Document;

/// What one document turned out to be.
struct Finding {
    /// The file's name, for the list of what matched.
    name: String,
    /// Whether the catalog states a `/StructTreeRoot`.
    tagged: bool,
    /// `L` elements the walk reached.
    lists: usize,
    /// `L` elements stating Table 382's `/ContinuedList`, whatever its value.
    flagged: usize,
    /// `L` elements this reader takes to continue another, which is `/ContinuedList true` or —
    /// by the choice `Tree::list_continuation` documents — a `/ContinuedFrom` with no flag.
    continuing: usize,
    /// Continuing lists that state `/ContinuedFrom` and **no** `/ContinuedList`, which is the
    /// population that choice decides and the reason it is counted apart.
    named_without_a_flag: usize,
    /// Continuing lists stating Table 382's `/ContinuedFrom`.
    named: usize,
    /// Continuing lists whose predecessor resolved, by either of the clause's two routes.
    resolved: usize,
    /// Those resolved by the clause's fallback rather than by a stated identifier.
    derived: usize,
    /// Elements stating either entry whose mapped type is not `L`.
    misplaced: usize,
}

impl Finding {
    /// Whether this document is worth printing a line for.
    fn notable(&self) -> bool {
        self.flagged > 0 || self.named > 0 || self.misplaced > 0
    }
}

/// Walks one document's structure tree and counts what §14.8.5.5 is about.
fn census(name: String, document: &Document) -> Finding {
    let mut found = Finding {
        name,
        tagged: false,
        lists: 0,
        flagged: 0,
        continuing: 0,
        named_without_a_flag: 0,
        named: 0,
        resolved: 0,
        derived: 0,
        misplaced: 0,
    };
    let Some(tree) = Tree::of(document) else {
        return found;
    };
    found.tagged = true;
    let mut lists: Vec<ListEntry> = Vec::new();
    for (position, (depth, child)) in tree.walk(document).items.into_iter().enumerate() {
        let Child::Element(dict) = child else {
            continue;
        };
        let flag = tree.attribute(document, &dict, "ContinuedList");
        let from = tree.attribute(document, &dict, "ContinuedFrom");
        if tree.standard_role(document, &dict) != Some(StandardType::List) {
            if flag.is_some() || from.is_some() {
                found.misplaced += 1;
            }
            continue;
        }
        found.lists += 1;
        if flag.is_some() {
            found.flagged += 1;
        }
        let continuation = tree.list_continuation(document, &dict);
        if continuation.is_some() {
            found.continuing += 1;
            if flag.is_none() {
                found.named_without_a_flag += 1;
            }
        }
        if matches!(continuation, Some(ListContinuation::From(_))) {
            found.named += 1;
        }
        lists.push(ListEntry {
            position,
            depth,
            id: document
                .get_key(&dict, "ID")
                .as_string()
                .map(<[u8]>::to_vec)
                .filter(|id| !id.is_empty()),
            continuation,
        });
    }
    let predecessors = list_predecessors(&lists);
    found.resolved = predecessors.len();
    found.derived = lists
        .iter()
        .filter(|list| {
            list.continuation == Some(ListContinuation::Preceding)
                && predecessors.contains_key(&list.position)
        })
        .count();
    found
}

/// Opens one document and counts it, or answers `None` for a file that will not open.
fn examine(path: &str) -> Option<Finding> {
    let bytes = std::fs::read(path).ok()?;
    let document = Document::open(bytes).ok()?;
    Some(census(path.to_owned(), &document))
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let findings: Vec<Finding> = paths
        .par_iter()
        .filter_map(|path| examine(path))
        .collect::<Vec<_>>();

    // What matched, before the count that summarises it (trap 11): the day one of these appears
    // is the day somebody wants to open the file it is in.
    for found in findings.iter().filter(|found| found.notable()) {
        println!(
            "{}: {} list(s), {} state /ContinuedList, {} continue ({} of them named by \
             /ContinuedFrom alone), {} name one with /ContinuedFrom, {} resolved ({} by the \
             clause's fallback), {} on a non-list element",
            found.name,
            found.lists,
            found.flagged,
            found.continuing,
            found.named_without_a_flag,
            found.named,
            found.resolved,
            found.derived,
            found.misplaced,
        );
    }

    let sum = |pick: fn(&Finding) -> usize| findings.iter().map(pick).sum::<usize>();
    println!(
        "\n{} path(s) given, {} opened, {} with a /StructTreeRoot",
        paths.len(),
        findings.len(),
        findings.iter().filter(|found| found.tagged).count(),
    );
    println!("{} L elements", sum(|found| found.lists));
    println!(
        "{} state Table 382's /ContinuedList; {} continue, {} of them on a /ContinuedFrom alone",
        sum(|found| found.flagged),
        sum(|found| found.continuing),
        sum(|found| found.named_without_a_flag),
    );
    println!(
        "{} name their predecessor with /ContinuedFrom",
        sum(|found| found.named),
    );
    println!(
        "{} continuing lists resolve to one, {} of them by the clause's fallback",
        sum(|found| found.resolved),
        sum(|found| found.derived),
    );
    println!(
        "{} elements state either entry and are not L elements",
        sum(|found| found.misplaced),
    );
}
