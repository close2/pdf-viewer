//! Table 384's `/Headers`, and §14.8.4.8.3's search for the cells that state none.
//!
//! Table 384 gives a table cell its header cells twice over: an array of element identifiers the
//! producer wrote, and — "[i]f the `Headers` attribute … is not specified" — an algorithm
//! §14.8.4.8.3 states in four bullets. This counts both, because implementing only the array
//! would be a clause with two routes to one answer and one of them silent, which is the shape
//! `doc/HANDOVER.md`'s trap 5 is about.
//!
//! What it was written to settle is whether the *search* is worth its code: a screen reader
//! announces a cell's headers before the cell, and if every cell that has headers stated them
//! outright the algorithm would be decoration.
//!
//! ```sh
//! cargo run --release -p pdf-model --example cell_header_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use pdf_model::structure::{Child, StandardType, TableStack, Tree};
use pdf_syntax::Document;

/// What one document contributes.
#[derive(Default)]
struct Counts {
    /// Documents with a structure tree at all.
    tagged: usize,
    /// Documents stating Table 354's `/IDTree`.
    with_an_id_tree: usize,
    /// `TH` and `TD` elements.
    cells: usize,
    /// Cells this reader placed in a grid, which is one inside a `TR`.
    placed: usize,
    /// Cells stating Table 384's `/Headers`.
    stated: usize,
    /// Cells stating a `/Headers` array with nothing in it.
    stated_empty: usize,
    /// Identifiers those arrays name.
    named: usize,
    /// Header associations a stated array produced, expansion included.
    named_found: usize,
    /// Cells that end with at least one header.
    with_headers: usize,
    /// Cells that end with at least one header **and stated none**, which is the search's yield.
    searched: usize,
    /// Header cells across every cell's answer, counted with repetition.
    total_headers: usize,
    /// The longest answer any one cell got.
    longest: usize,
    /// `TH` elements stating Table 384's `/Short`.
    shorts: usize,
    /// `Table` elements stating Table 384's `/Summary`.
    summaries: usize,
    /// Documents whose tables outgrew the grid this reader keeps.
    truncated: usize,
}

impl Counts {
    /// Adds `other`'s totals to this one's.
    fn absorb(&mut self, other: &Self) {
        self.tagged = self.tagged.saturating_add(other.tagged);
        self.with_an_id_tree = self.with_an_id_tree.saturating_add(other.with_an_id_tree);
        self.cells = self.cells.saturating_add(other.cells);
        self.placed = self.placed.saturating_add(other.placed);
        self.stated = self.stated.saturating_add(other.stated);
        self.stated_empty = self.stated_empty.saturating_add(other.stated_empty);
        self.named = self.named.saturating_add(other.named);
        self.named_found = self.named_found.saturating_add(other.named_found);
        self.with_headers = self.with_headers.saturating_add(other.with_headers);
        self.searched = self.searched.saturating_add(other.searched);
        self.total_headers = self.total_headers.saturating_add(other.total_headers);
        self.longest = self.longest.max(other.longest);
        self.shorts = self.shorts.saturating_add(other.shorts);
        self.summaries = self.summaries.saturating_add(other.summaries);
        self.truncated = self.truncated.saturating_add(other.truncated);
    }
}

/// Whether the document states Table 354's `/IDTree`, which is what `/Headers` names cells through.
fn states_an_id_tree(document: &Document) -> bool {
    document
        .catalog()
        .ok()
        .map(|catalog| document.get_key(&catalog, "StructTreeRoot"))
        .and_then(|root| root.as_dict().cloned())
        .is_some_and(|root| document.get_key(&root, "IDTree").as_dict().is_some())
}

/// Walks one document's structure tree, driving the grid the way a reader does.
fn census(document: &Document) -> Counts {
    let mut counts = Counts::default();
    let Some(tree) = Tree::of(document) else {
        return counts;
    };
    counts.tagged = 1;
    counts.with_an_id_tree = usize::from(states_an_id_tree(document));
    let mut stack = TableStack::new();
    // The tokens of the cells that stated `/Headers`, in walk order, so that the search's own
    // yield can be told from the array's.
    let mut stated: Vec<usize> = Vec::new();
    for (token, (depth, child)) in tree.walk(document).items.into_iter().enumerate() {
        let Child::Element(dict) = child else {
            continue;
        };
        let kind = tree.standard_role(document, &dict);
        let placement = stack.enter(depth, kind.as_ref(), token, || {
            tree.cell_facts(document, &dict)
        });
        if kind == Some(StandardType::Table) && tree.attribute(document, &dict, "Summary").is_some()
        {
            counts.summaries = counts.summaries.saturating_add(1);
        }
        if !matches!(
            kind,
            Some(StandardType::TableHeader | StandardType::TableData)
        ) {
            continue;
        }
        counts.cells = counts.cells.saturating_add(1);
        if placement.is_some() {
            counts.placed = counts.placed.saturating_add(1);
        }
        if kind == Some(StandardType::TableHeader)
            && tree.attribute(document, &dict, "Short").is_some()
        {
            counts.shorts = counts.shorts.saturating_add(1);
        }
        if let Some(ids) = tree.cell_headers(document, &dict) {
            counts.stated = counts.stated.saturating_add(1);
            if ids.is_empty() {
                counts.stated_empty = counts.stated_empty.saturating_add(1);
            }
            counts.named = counts.named.saturating_add(ids.len());
            stated.push(token);
        }
    }
    counts.truncated = usize::from(stack.truncated());
    for (token, headers) in stack.headers() {
        counts.with_headers = counts.with_headers.saturating_add(1);
        counts.total_headers = counts.total_headers.saturating_add(headers.len());
        counts.longest = counts.longest.max(headers.len());
        if stated.binary_search(&token).is_ok() {
            counts.named_found = counts.named_found.saturating_add(headers.len());
        } else {
            counts.searched = counts.searched.saturating_add(1);
        }
    }
    counts
}

fn main() {
    let mut total = Counts::default();
    let mut documents = 0usize;
    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let counts = census(&document);
        if counts.with_headers > 0 {
            println!(
                "{path}: {} cells, {} with headers ({} of them from the search), \
                 {} headers in all, longest {}",
                counts.cells,
                counts.with_headers,
                counts.searched,
                counts.total_headers,
                counts.longest,
            );
        }
        // A witness for the two rare entries is named, not merely counted: the day one appears
        // is the day somebody wants to open it.
        if counts.summaries > 0 || counts.shorts > 0 {
            println!(
                "{path}: {} /Summary, {} /Short",
                counts.summaries, counts.shorts
            );
        }
        total.absorb(&counts);
    }
    println!("\n{documents} opened, {} tagged", total.tagged);
    println!("{} state Table 354's /IDTree", total.with_an_id_tree);
    println!(
        "{} cells (TH and TD), {} of them placed in a grid",
        total.cells, total.placed
    );
    println!(
        "{} cells state Table 384's /Headers ({} of them an empty array), naming {} identifiers",
        total.stated, total.stated_empty, total.named
    );
    println!(
        "{} cells end with at least one header: {} of them from §14.8.4.8.3's search",
        total.with_headers, total.searched
    );
    println!(
        "{} header associations in all, {} of them from a stated array, longest answer {}",
        total.total_headers, total.named_found, total.longest
    );
    println!(
        "{} TH state Table 384's /Short, {} Table its /Summary; {} documents outgrew the grid",
        total.shorts, total.summaries, total.truncated
    );
}
