//! §14.8.5.7's table attributes, counted over a corpus.
//!
//! Table 384 states six entries and four of them are what makes a table *readable* rather than
//! merely drawable: `/RowSpan`, `/ColSpan`, `/Headers` and `/Scope`. This counts what the
//! documents actually state, and — for the `TH` cells that state no `/Scope`, which the clause
//! answers for with an assumption — what that assumption comes to.
//!
//! The question it was written to settle is whether reading `/Scope` changes anything a person
//! would hear: a `TH` becomes `accesskit::Role::ColumnHeader` or `RowHeader` by its axis, and if
//! every header cell in every corpus document were a column's, the two roles would be one.
//!
//! ```sh
//! cargo run --release -p pdf-model --example table_header_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use pdf_model::structure::{Child, HeaderScope, StandardType, TableStack, Tree};
use pdf_syntax::Document;

/// What one document contributes.
#[derive(Default)]
struct Counts {
    /// Documents with a structure tree at all.
    tagged: usize,
    /// Documents stating at least one `Table` element.
    with_a_table: usize,
    /// `Table` elements.
    tables: usize,
    /// `TH` elements.
    headers: usize,
    /// `TD` elements.
    data: usize,
    /// `TH` elements stating Table 384's `/Scope`.
    stated_scope: usize,
    /// The scope each `TH` ends with — stated or, failing that, §14.8.5.7's assumption.
    scopes: [usize; 3],
    /// `TH` elements this reader could place in no grid, which is one outside a `TR`.
    unplaced: usize,
    /// Cells stating a `/RowSpan` or `/ColSpan` above 1.
    spanning: usize,
    /// Cells stating Table 384's `/Headers`.
    with_headers: usize,
}

impl Counts {
    /// Adds `other`'s totals to this one's.
    fn absorb(&mut self, other: &Self) {
        self.tagged = self.tagged.saturating_add(other.tagged);
        self.with_a_table = self.with_a_table.saturating_add(other.with_a_table);
        self.tables = self.tables.saturating_add(other.tables);
        self.headers = self.headers.saturating_add(other.headers);
        self.data = self.data.saturating_add(other.data);
        self.stated_scope = self.stated_scope.saturating_add(other.stated_scope);
        for (total, one) in self.scopes.iter_mut().zip(other.scopes) {
            *total = total.saturating_add(one);
        }
        self.unplaced = self.unplaced.saturating_add(other.unplaced);
        self.spanning = self.spanning.saturating_add(other.spanning);
        self.with_headers = self.with_headers.saturating_add(other.with_headers);
    }

    /// Whether this document is worth naming in the output.
    fn is_a_witness(&self) -> bool {
        self.headers > 0
    }
}

/// Which slot of [`Counts::scopes`] a scope lands in.
fn slot(scope: HeaderScope) -> usize {
    match scope {
        HeaderScope::Row => 0,
        HeaderScope::Column => 1,
        HeaderScope::Both => 2,
    }
}

/// Walks one document's structure tree, driving the grid the way a reader does.
fn census(document: &Document) -> Counts {
    let mut counts = Counts::default();
    let Some(tree) = Tree::of(document) else {
        return counts;
    };
    counts.tagged = 1;
    let mut stack = TableStack::new();
    for (depth, child) in tree.walk(document).items {
        let Child::Element(dict) = child else {
            continue;
        };
        let kind = tree.standard_role(document, &dict);
        let placement = stack.enter(depth, kind.as_ref(), || tree.cell_span(document, &dict));
        match kind {
            Some(StandardType::Table) => counts.tables = counts.tables.saturating_add(1),
            Some(StandardType::TableHeader) => {
                counts.headers = counts.headers.saturating_add(1);
                let stated = tree.header_scope(document, &dict);
                if stated.is_some() {
                    counts.stated_scope = counts.stated_scope.saturating_add(1);
                }
                match stated
                    .or_else(|| placement.map(|cell| HeaderScope::assumed(cell.row, cell.column)))
                {
                    Some(scope) => {
                        if let Some(count) = counts.scopes.get_mut(slot(scope)) {
                            *count = count.saturating_add(1);
                        }
                    }
                    None => counts.unplaced = counts.unplaced.saturating_add(1),
                }
            }
            Some(StandardType::TableData) => counts.data = counts.data.saturating_add(1),
            _ => {}
        }
        if matches!(
            kind,
            Some(StandardType::TableHeader | StandardType::TableData)
        ) {
            if tree.cell_span(document, &dict) != (1, 1) {
                counts.spanning = counts.spanning.saturating_add(1);
            }
            if tree.attribute(document, &dict, "Headers").is_some() {
                counts.with_headers = counts.with_headers.saturating_add(1);
            }
        }
    }
    counts.with_a_table = usize::from(counts.tables > 0);
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
        if counts.is_a_witness() {
            println!(
                "{path}: {} table(s), {} TH, {} TD, {} stated /Scope, \
                 assumed Row/Column/Both {}/{}/{}, {} spanning, {} with /Headers",
                counts.tables,
                counts.headers,
                counts.data,
                counts.stated_scope,
                counts.scopes[0],
                counts.scopes[1],
                counts.scopes[2],
                counts.spanning,
                counts.with_headers,
            );
        }
        total.absorb(&counts);
    }
    println!(
        "\n{documents} documents, {} with a structure tree",
        total.tagged
    );
    println!(
        "{} with at least one Table, {} tables",
        total.with_a_table, total.tables
    );
    println!("{} TH, {} TD", total.headers, total.data);
    println!(
        "{} TH state Table 384's /Scope; the rest take §14.8.5.7's assumption",
        total.stated_scope
    );
    println!(
        "scopes: Row {}, Column {}, Both {}, and {} TH this reader could place nowhere",
        total.scopes[0], total.scopes[1], total.scopes[2], total.unplaced
    );
    println!(
        "{} cells span more than one row or column, {} state /Headers",
        total.spanning, total.with_headers
    );
}
