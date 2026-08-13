//! Table 202's `/SD`, counted: how many go-to actions state a structure destination, and how
//! many of them would go somewhere else if it were honoured.
//!
//! §12.6.4.2 gives a go-to action two destinations and an order between them:
//!
//! > (Optional; PDF 2.0) The structure destination to jump to (see 12.3.2.3, "Structure
//! > destinations"). If present, the structure destination should take precedence over
//! > destination in the D entry.
//!
//! The population that *ranks* that sentence is not "actions stating an `/SD`" but the narrower
//! one where obeying it changes an answer: an action whose `/SD` resolves to a different page
//! from its `/D`, or whose `/D` resolves to nothing. Anything else jumps to the same page either
//! way, which is trap 8's shape and the reason this counts four things rather than one.
//!
//! Every object the cross-reference table lists is walked rather than every page's `/Annots`,
//! because a go-to action hangs off a link annotation, an outline item, a catalog `/OpenAction`,
//! a widget's `/AA` and another action's `/Next` — and a census that visited only one of those
//! would measure the walk rather than the corpus.
//!
//! `/S /GoToR` is counted beside it and is **not** the same question: `CLAUDE.md` excludes a
//! remote go-to, and Table 203's `/SD` names a structure element in a file this reader will not
//! open.
//!
//! ```sh
//! cargo run --release -p pdf-model --example structure_destination_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use pdf_model::Pages;
use pdf_model::destination::Destination;
use pdf_syntax::{Document, ObjectId};

/// What one document contributes to the census.
#[derive(Default)]
struct Counts {
    /// Action dictionaries stating `/S /GoTo`.
    go_to: usize,
    /// Of those, the ones stating Table 202's `/SD`.
    with_structure: usize,
    /// Of those, the ones whose `/SD` this reader can read as a destination at all.
    structure_readable: usize,
    /// Of those, the ones whose `/SD` names a different page from their `/D`.
    ///
    /// The count that ranks the rule: every one of these is a jump this program takes to the
    /// wrong page.
    disagreeing: usize,
    /// Of those, the ones whose `/SD` states a different *view* of the same page.
    ///
    /// Counted apart because §12.3.2.3 makes a structure destination behave "identically to a
    /// destination" once its page is identified, so Table 149's form and parameters are the
    /// second half of what the entry states and can differ where the page does not.
    disagreeing_view: usize,
    /// Action dictionaries stating `/S /GoToR` with an `/SD`, which is a different clause.
    remote_with_structure: usize,
}

impl Counts {
    /// Adds `other`'s totals to this one's.
    fn absorb(&mut self, other: &Self) {
        self.go_to = self.go_to.saturating_add(other.go_to);
        self.with_structure = self.with_structure.saturating_add(other.with_structure);
        self.structure_readable = self
            .structure_readable
            .saturating_add(other.structure_readable);
        self.disagreeing = self.disagreeing.saturating_add(other.disagreeing);
        self.disagreeing_view = self.disagreeing_view.saturating_add(other.disagreeing_view);
        self.remote_with_structure = self
            .remote_with_structure
            .saturating_add(other.remote_with_structure);
    }
}

fn main() {
    let mut total = Counts::default();
    let mut opened = 0_usize;
    let mut lines: Vec<String> = Vec::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        opened = opened.saturating_add(1);
        let counts = document_counts(&document);
        if counts.with_structure > 0 || counts.remote_with_structure > 0 {
            let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
            lines.push(format!(
                "  {name}: {} go-to action(s), {} stating /SD ({} readable, {} naming a different \
                 page from /D, {} a different view of the same page), {} remote",
                counts.go_to,
                counts.with_structure,
                counts.structure_readable,
                counts.disagreeing,
                counts.disagreeing_view,
                counts.remote_with_structure
            ));
        }
        total.absorb(&counts);
    }

    println!("{opened} document(s) opened");
    println!("  {} /S /GoTo action dictionar(ies)", total.go_to);
    println!(
        "  {} state Table 202's /SD, {} of which read as a destination",
        total.with_structure, total.structure_readable
    );
    println!(
        "  {} name a page their /D does not and {} a different view of the same page, which \
         together are the population the rule ranks",
        total.disagreeing, total.disagreeing_view
    );
    println!(
        "  {} /S /GoToR action(s) state an /SD, which is Table 203's and excluded",
        total.remote_with_structure
    );
    for line in &lines {
        println!("{line}");
    }
}

/// Walks every object the cross-reference table lists in one document.
fn document_counts(document: &Document) -> Counts {
    let mut counts = Counts::default();
    let pages = Pages::new(document);
    let numbers: Vec<u32> = document.xref().object_numbers().collect();
    for number in numbers {
        let object = document.get(ObjectId::new(number, 0));
        let Some(dict) = object.as_dict() else {
            continue;
        };
        let kind = document.get_key(dict, "S");
        let Some(kind) = kind.as_name() else {
            continue;
        };
        match kind.as_bytes() {
            b"GoTo" => counts.go_to = counts.go_to.saturating_add(1),
            b"GoToR" => {
                if dict.get("SD").is_some() {
                    counts.remote_with_structure = counts.remote_with_structure.saturating_add(1);
                }
                continue;
            }
            _ => continue,
        }
        let Some(entry) = dict.get("SD") else {
            continue;
        };
        counts.with_structure = counts.with_structure.saturating_add(1);
        let Some(structure) = Destination::read(document, entry) else {
            continue;
        };
        counts.structure_readable = counts.structure_readable.saturating_add(1);
        let stated = dict
            .get("D")
            .and_then(|entry| Destination::read(document, entry));
        let stated_page = stated.and_then(|destination| destination.page_index(document, &pages));
        if structure.page_index(document, &pages) != stated_page {
            counts.disagreeing = counts.disagreeing.saturating_add(1);
        } else if stated.is_none_or(|destination| destination.view != structure.view) {
            counts.disagreeing_view = counts.disagreeing_view.saturating_add(1);
        }
    }
    counts
}
