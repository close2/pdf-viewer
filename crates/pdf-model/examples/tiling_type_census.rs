//! Which of Table 74's three `/TilingType` codes the tiling patterns in a corpus ask for.
//!
//! ISO 32000-2 §8.7.3.1 makes the entry "( Required )" of every Type 1 pattern dictionary and
//! calls it "[a] code that controls adjustments to the spacing of tiles relative to the device
//! pixel grid". All three codes are about that grid, and a display list has none in it — the sites
//! a tiling paints are multiples of `/XStep` and `/YStep` in pattern space, rasterised by each
//! backend at its own resolution — so this tree draws value 2, "[t]he pattern cell shall not be
//! distorted, but the spacing between pattern cells may vary by as much as 1 device pixel, both
//! horizontally and vertically, when the pattern is painted", whatever a file asks for.
//!
//! **This census exists because that departure is not reported per page** (ADR 1031), and a
//! departure nobody can count is a departure nobody can rank. The report was measured before it
//! was declined: it put seven of `doc/pdf.js`'s documents on `corpus.rs`'s incomplete list, every
//! one of them a tiling document, and an incomplete page leaves the oracle's judged population —
//! which would have taken two diagnosed entries out of its ambiguous buckets for a difference the
//! clause itself bounds at one device pixel. So the population is printed by a command instead
//! (`CLAUDE.md`, *Where knowledge lives*).
//!
//! What is counted, and why each number is here:
//!
//! - **Type 1 pattern dictionaries**, found by `/PatternType 1` over every object of the file.
//!   That is the population the entry is required of.
//! - **How many state each of Table 74's three codes**, so that 1 and 3 — the two that ask for an
//!   adjustment this tree does not make — can be told from 2, which asks for what it does.
//! - **How many state the entry as something else, or not at all**, kept apart from the three
//!   because it is a different claim: a required entry absent or outside its enumeration is a
//!   malformed file, and this tree treats it exactly as it treats value 2.
//! - **The documents, by name, with the codes each asks for**, because a population of one is a
//!   witness and a population of forty is a rule, and only the list says which.
//!
//! What it does **not** say is whether a pattern is ever painted: a dictionary is counted where
//! the file states one, and a pattern no content stream names as a colour never reaches
//! `Interpreter::tiling` at all. That is deliberate — this is a census of what producers ask for,
//! and the narrower question is what the corpus gate's own output answers when a report is raised.
//!
//! ```sh
//! cargo run --release -p pdf-model --example tiling_type_census -- \
//!     doc/pdf.js/test/pdfs/*.pdf
//! ```
//!
//! One process per directory is the surveys' own method, for the surveys' own reason: this parses
//! hostile input, and an abort would take every other verdict in the process with it.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "counters over a corpus four orders of magnitude below what a usize counts, and \
              this is a measurement rather than a shipped path"
)]

use pdf_syntax::{Document, Object, ObjectId};

/// Everything this census keeps.
#[derive(Default)]
struct Census {
    /// Documents opened.
    documents: usize,
    /// Type 1 pattern dictionaries seen.
    patterns: usize,
    /// Of those, how many state each of Table 74's codes: index 0 is code 1.
    stated: [usize; 3],
    /// Of those, how many state an integer outside 1 to 3.
    outside: usize,
    /// Of those, how many state no `/TilingType` at all, or a value that is not an integer.
    unstated: usize,
    /// Every document holding a pattern that asks for 1 or 3, as `document: codes`.
    witnesses: Vec<String>,
}

impl Census {
    /// Counts one document's Type 1 pattern dictionaries.
    fn count(&mut self, name: &str, document: &Document) {
        self.documents += 1;
        let mut asked: Vec<i64> = Vec::new();
        for number in document.xref().object_numbers() {
            let object = document.get(ObjectId {
                number,
                generation: 0,
            });
            // A tiling pattern is a *stream*, so its dictionary is reached through the stream —
            // Issue #294 against this subclause inserts "stream" into Table 74's own caption. A
            // plain dictionary is read as well, because a malformed file may write one.
            let dict = match &object {
                Object::Stream(stream) => Some(&stream.dict),
                other => other.as_dict(),
            };
            let Some(dict) = dict else {
                continue;
            };
            if document.get_key(dict, "PatternType").as_integer() != Some(1) {
                continue;
            }
            self.patterns += 1;
            match document.get_key(dict, "TilingType").as_integer() {
                Some(code @ 1..=3) => {
                    #[expect(
                        clippy::indexing_slicing,
                        reason = "the arm's own pattern bounds the code to 1..=3, so the index is \
                                  0..=2 and the array is three long"
                    )]
                    {
                        self.stated[usize::try_from(code - 1).unwrap_or(0)] += 1;
                    }
                    if code != 2 {
                        asked.push(code);
                    }
                }
                Some(_) => self.outside += 1,
                None => self.unstated += 1,
            }
        }
        if !asked.is_empty() {
            asked.sort_unstable();
            asked.dedup();
            let codes: Vec<String> = asked.iter().map(ToString::to_string).collect();
            self.witnesses
                .push(format!("{name}: /TilingType {}", codes.join(" and ")));
        }
    }

    /// Prints the measurement.
    fn report(&self) {
        println!("documents opened: {}", self.documents);
        println!("type 1 pattern dictionaries: {}", self.patterns);
        for (slot, count) in self.stated.iter().enumerate() {
            println!("  /TilingType {}: {count}", slot + 1);
        }
        println!("  an integer outside 1 to 3: {}", self.outside);
        println!("  stated as nothing readable: {}", self.unstated);
        println!(
            "documents asking for an adjustment this tree does not make: {}",
            self.witnesses.len()
        );
        for witness in &self.witnesses {
            println!("  {witness}");
        }
    }
}

fn main() {
    let mut census = Census::default();
    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
        census.count(&name, &document);
    }
    census.report();
}
