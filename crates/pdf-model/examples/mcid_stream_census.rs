//! How many pages number two content streams' marked-content sequences from zero, and collide.
//!
//! The instrument behind ADR 0488, and the measurement session 658 could not make.
//!
//! ISO 32000-2 §14.7.5.2 makes a `/MCID` "an integer marked-content identifier that uniquely
//! identifies the marked-content sequence **within its content stream**", and §14.7.5.2 permits a
//! form `XObject`'s own stream to hold sequences of its own; Errata Collection 3's Issue #308 adds
//! §14.7.5.4 a NOTE saying the consequence outright — identifiers are scoped by content stream and
//! start at zero, so the same one may reappear across pages or `XObject`s. So a page whose
//! `/Contents` and whose form both number from zero has two different sequences called `/MCID 0`,
//! and anything keyed on the identifier alone attributes one to the other.
//!
//! What this counts, per page:
//!
//! - **how many content streams marked at all** — two or more is the *condition* for the defect,
//!   whether or not the numbers happen to meet;
//! - **how many identifiers two streams share** — the collision itself, which is where a rectangle
//!   and a text range go to the wrong element;
//! - and the three things a *file* says about the same question, read without interpreting
//!   anything: a `/StructTreeRoot` at all, a Table 357 `/MCR` stating `/Stm`, and a form `XObject`
//!   with a `/StructParents` of its own (§14.7.5.4 Table 359).
//!
//! The last three are the probe trap 648's rule asks for: a census that only looked at what the
//! interpreter produced would report a clean zero on a population whose files declare the
//! construction in their own dictionaries, and the two counts disagreeing is the tell.
//!
//! ```sh
//! cargo run --release -p pdf-model --example mcid_stream_census -- \
//!   $(find doc/pdf.js/test/pdfs -maxdepth 1 -name '*.pdf') \
//!   $(find -L doc/corpora -name '*.pdf') doc/*.pdf
//! ```
//!
//! **Pages are interpreted only for a document with a structure tree**, which is what bounds this
//! over the 65 944-document crawl: a page with no structure tree has no element for a misattributed
//! identifier to reach, and interpreting every page of every crawled file to say so would cost
//! hours to count nothing. The file-level counts above are taken for *every* document, so the
//! population that was skipped is still named.
#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is the measurement"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "counters over a corpus four orders of magnitude below what a usize counts, and \
              this is a measurement rather than a shipped path"
)]

use std::collections::{BTreeMap, BTreeSet};

use rayon::prelude::*;

use pdf_model::content::ContentStream;
use pdf_model::structure::{Child, Tree};
use pdf_model::{Pages, interpret};
use pdf_syntax::{Document, Object, ObjectId};

/// How deep a form `XObject`'s resources are followed looking for another form.
///
/// A form's `/Resources` may name a form, which may name a form. Real nesting is a level or two;
/// this is the same order of magnitude as the interpreter's own `MAX_FORM_DEPTH` and stops a file
/// whose resources cycle.
const MAX_FORM_DEPTH: usize = 16;

/// What one document turned out to be.
struct Finding {
    /// The file's name, for the list of what matched.
    name: String,
    /// Whether the catalog states a `/StructTreeRoot`, and the pages were therefore interpreted.
    tagged: bool,
    /// Pages on which two or more distinct content streams each closed a sequence with an `/MCID`.
    pages_with_two_streams: usize,
    /// Pages on which two distinct streams used the *same* identifier.
    pages_colliding: usize,
    /// How many identifiers collided, summed over the pages.
    identifiers_colliding: usize,
    /// Structure elements whose `/K` states a Table 357 `/MCR` with a `/Stm`.
    references_naming_a_stream: usize,
    /// Form `XObject`s reachable from a page's resources that state a `/StructParents` of their own.
    forms_with_struct_parents: usize,
}

impl Finding {
    /// Whether this document is worth printing a line for.
    fn notable(&self) -> bool {
        self.pages_with_two_streams > 0
            || self.references_naming_a_stream > 0
            || self.forms_with_struct_parents > 0
    }
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let findings: Vec<Finding> = paths.par_iter().filter_map(|path| examine(path)).collect();

    let opened = findings.len();
    let tagged = findings.iter().filter(|found| found.tagged).count();
    let two_streams = findings
        .iter()
        .filter(|found| found.pages_with_two_streams > 0)
        .count();
    let colliding = findings
        .iter()
        .filter(|found| found.pages_colliding > 0)
        .count();
    let declaring = findings
        .iter()
        .filter(|found| found.references_naming_a_stream > 0)
        .count();
    let form_parents = findings
        .iter()
        .filter(|found| found.forms_with_struct_parents > 0)
        .count();

    // What it matched, before the count that summarises it (trap 11).
    for found in findings.iter().filter(|found| found.notable()) {
        println!(
            "{}: {} page(s) with two marking streams, {} colliding ({} identifier(s)), \
             {} /MCR with /Stm, {} form(s) with /StructParents{}",
            found.name,
            found.pages_with_two_streams,
            found.pages_colliding,
            found.identifiers_colliding,
            found.references_naming_a_stream,
            found.forms_with_struct_parents,
            if found.tagged { "" } else { ", untagged" },
        );
    }

    println!(
        "{} path(s) given, {opened} opened, {tagged} with a /StructTreeRoot (pages interpreted)",
        paths.len(),
    );
    println!(
        "documents with a page whose sequences came from two or more content streams: {two_streams}"
    );
    println!("documents where two streams on one page share an identifier: {colliding}");
    println!(
        "documents whose structure tree states a Table 357 /Stm: {declaring}; \
         documents with a form /XObject stating its own /StructParents: {form_parents}"
    );
    println!(
        "pages with two marking streams: {}; pages colliding: {}; identifiers colliding: {}",
        total(&findings, |found| found.pages_with_two_streams),
        total(&findings, |found| found.pages_colliding),
        total(&findings, |found| found.identifiers_colliding),
    );
}

/// Adds one field over every finding.
fn total(findings: &[Finding], of: impl Fn(&Finding) -> usize) -> usize {
    findings.iter().map(of).sum()
}

/// Reads one document, interpreting its pages only where it has a structure tree.
fn examine(path: &str) -> Option<Finding> {
    let name = std::path::Path::new(path).file_name().map_or_else(
        || path.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let bytes = std::fs::read(path).ok()?;
    let document = Document::open(bytes).ok()?;

    let tree = Tree::of(&document);
    let mut found = Finding {
        name,
        tagged: tree.is_some(),
        pages_with_two_streams: 0,
        pages_colliding: 0,
        identifiers_colliding: 0,
        references_naming_a_stream: 0,
        forms_with_struct_parents: 0,
    };

    let pages = Pages::new(&document);
    // Cheap and unconditional: what the file's own dictionaries declare, whether or not anything
    // is interpreted. Table 359 puts `/StructParents` on "the stream dictionary of a form or image
    // XObject" exactly when that stream holds structure content items.
    let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        found.forms_with_struct_parents +=
            forms_with_struct_parents(&document, &page.resources, &mut seen, 0);
    }
    if let Some(tree) = &tree {
        found.references_naming_a_stream = references_naming_a_stream(&document, tree);
    }

    if tree.is_none() {
        return Some(found);
    }

    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        // Which identifiers each content stream of this page used.
        let mut per_stream: BTreeMap<ContentStream, BTreeSet<i64>> = BTreeMap::new();
        for span in &interpret(&document, &page).marked {
            per_stream.entry(span.stream).or_default().insert(span.mcid);
        }
        if per_stream.len() < 2 {
            continue;
        }
        found.pages_with_two_streams += 1;
        let mut shared: BTreeSet<i64> = BTreeSet::new();
        for (at, (_, one)) in per_stream.iter().enumerate() {
            for (_, other) in per_stream.iter().skip(at + 1) {
                shared.extend(one.intersection(other));
            }
        }
        if !shared.is_empty() {
            found.pages_colliding += 1;
            found.identifiers_colliding += shared.len();
        }
    }
    Some(found)
}

/// How many `/K` entries anywhere in the tree are a Table 357 reference naming a `/Stm`.
fn references_naming_a_stream(document: &Document, tree: &Tree) -> usize {
    tree.walk(document)
        .items
        .iter()
        .filter(|(_, child)| {
            matches!(
                child,
                Child::MarkedContent {
                    stream: Some(_),
                    ..
                }
            )
        })
        .count()
}

/// How many form `XObject`s this resource dictionary reaches state a `/StructParents` of their own.
///
/// Deduplicated by object, because a form drawn on twenty pages is one stream and one entry in
/// §14.7.5.4's parent tree.
fn forms_with_struct_parents(
    document: &Document,
    resources: &pdf_syntax::Dictionary,
    seen: &mut BTreeSet<ObjectId>,
    depth: usize,
) -> usize {
    if depth >= MAX_FORM_DEPTH {
        return 0;
    }
    let xobjects = document.get_key(resources, "XObject");
    let Some(table) = xobjects.as_dict() else {
        return 0;
    };
    let mut count = 0;
    for (_, entry) in table.iter() {
        let Some(object) = entry.as_reference() else {
            continue;
        };
        if !seen.insert(object) {
            continue;
        }
        let resolved = document.resolve(entry);
        let Some(stream) = resolved.as_stream() else {
            continue;
        };
        if document
            .get_key(&stream.dict, "Subtype")
            .as_name()
            .map(pdf_syntax::Name::as_bytes)
            != Some(b"Form")
        {
            continue;
        }
        if document
            .get_key(&stream.dict, "StructParents")
            .as_integer()
            .is_some()
        {
            count += 1;
        }
        if let Some(inner) = document.get_key(&stream.dict, "Resources").as_dict() {
            count += forms_with_struct_parents(document, inner, seen, depth + 1);
        }
    }
    count
}

/// Silences the unused-import warning for [`Object`], which the reference reads need.
const _: fn(&Object) -> Option<ObjectId> = Object::as_reference;
