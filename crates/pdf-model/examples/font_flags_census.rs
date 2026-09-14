//! How a corpus writes a font descriptor's `/Flags`, against what ISO 32000-2 §9.8.2 states of it.
//!
//! > The value of the Flags entry in a font descriptor shall be an unsigned 32-bit integer
//! > containing flags specifying various characteristics of the font.
//!
//! and Table 121, of bits 3 and 6: "This flag and the Nonsymbolic flag shall not both be set or
//! both be clear." `pdf_font::metrics::flag` reads a word outside the unsigned 32-bit range as
//! no entry at all, and `is_symbolic` reads bit 3 alone where the pair contradicts itself, so
//! this counts the populations those two readings decide — and names the documents, since a
//! population of one is a witness and a population of forty is a rule.
//!
//! A descriptor is any dictionary object whose `/Type` is `FontDescriptor`, or which states
//! both `/FontName` and `/Flags` without one. Walks `doc/pdf.js/test/pdfs` and every `.pdf`
//! under `doc/corpora/`, or the directories named on the command line.
//!
//! ```sh
//! cargo run --release -p pdf-model --example font_flags_census
//! cargo run --release -p pdf-model --example font_flags_census -- corpus-cache/openpreserve
//! ```
#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a measurement whose output is its purpose: it stops loudly where the corpus is \
              missing, and its counters over the corpus's descriptors are four orders of \
              magnitude below what a usize counts"
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use pdf_syntax::{Document, Object, ObjectId};

fn corpus() -> Vec<PathBuf> {
    let tree = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let named: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    let roots = if named.is_empty() {
        vec![tree.join("doc/pdf.js/test/pdfs"), tree.join("doc/corpora")]
    } else {
        named
    };
    let mut files = Vec::new();
    for root in roots {
        collect(&root, &mut files);
    }
    files.sort();
    files
}

/// Every `.pdf` under `root`, however deep.
fn collect(root: &Path, into: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(root).expect("the corpus directory is on the disk");
    for path in entries.filter_map(|entry| entry.ok().map(|entry| entry.path())) {
        if path.is_dir() {
            collect(&path, into);
        } else if path.extension().is_some_and(|e| e == "pdf") {
            into.push(path);
        }
    }
}

/// Table 121's defined positions, as a mask; every other bit is "reserved and shall be set to
/// 0 by PDF writers".
const DEFINED: u32 = (1 << 0)
    | (1 << 1)
    | (1 << 2)
    | (1 << 3)
    | (1 << 5)
    | (1 << 6)
    | (1 << 16)
    | (1 << 17)
    | (1 << 18);
const SYMBOLIC: u32 = 1 << 2;
const NONSYMBOLIC: u32 = 1 << 5;

/// Everything this census keeps.
#[derive(Default)]
struct Census {
    documents: usize,
    descriptors: usize,
    /// `/Flags` absent.
    absent: usize,
    /// `/Flags` present and not an integer.
    not_integer: usize,
    /// A negative integer — not an unsigned 32-bit word.
    negative: BTreeSet<String>,
    /// An integer above 2^32 − 1 — not a 32-bit word.
    over: BTreeSet<String>,
    /// In range, with a reserved bit set.
    reserved: BTreeSet<String>,
    /// In range, with Symbolic and Nonsymbolic both set.
    both_set: BTreeSet<String>,
    /// In range, with Symbolic and Nonsymbolic both clear.
    both_clear: BTreeSet<String>,
}

impl Census {
    fn count(&mut self, name: &str, document: &Document) {
        self.documents += 1;
        for number in document.xref().object_numbers() {
            let object = document.get(ObjectId {
                number,
                generation: 0,
            });
            let Some(dict) = object.as_dict() else {
                continue;
            };
            let typed = document
                .get_key(dict, "Type")
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"FontDescriptor");
            let flags = document.get_key(dict, "Flags");
            let shaped = !matches!(document.get_key(dict, "FontName"), Object::Null)
                && !matches!(flags, Object::Null);
            if !typed && !shaped {
                continue;
            }
            self.descriptors += 1;
            let value = match flags {
                Object::Null => {
                    self.absent += 1;
                    continue;
                }
                Object::Integer(value) => value,
                _ => {
                    self.not_integer += 1;
                    continue;
                }
            };
            if value < 0 {
                self.negative.insert(name.to_owned());
                continue;
            }
            let Ok(word) = u32::try_from(value) else {
                self.over.insert(name.to_owned());
                continue;
            };
            if word & !DEFINED != 0 {
                self.reserved.insert(name.to_owned());
            }
            match (word & SYMBOLIC != 0, word & NONSYMBOLIC != 0) {
                (true, true) => {
                    self.both_set.insert(name.to_owned());
                }
                (false, false) => {
                    self.both_clear.insert(name.to_owned());
                }
                _ => {}
            }
        }
    }
}

fn list(set: &BTreeSet<String>) -> String {
    set.iter().cloned().collect::<Vec<_>>().join(", ")
}

fn main() {
    let mut census = Census::default();
    for path in corpus() {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        census.count(&name, &document);
    }
    println!(
        "{} documents, {} font descriptors: {} state no /Flags, {} state one that is not an \
         integer",
        census.documents, census.descriptors, census.absent, census.not_integer
    );
    println!(
        "  negative (not an unsigned 32-bit word): {} documents — {}",
        census.negative.len(),
        list(&census.negative)
    );
    println!(
        "  above 2^32 - 1 (not a 32-bit word): {} documents — {}",
        census.over.len(),
        list(&census.over)
    );
    println!(
        "  a reserved bit set: {} documents — {}",
        census.reserved.len(),
        list(&census.reserved)
    );
    println!(
        "  Symbolic and Nonsymbolic both set: {} documents — {}",
        census.both_set.len(),
        list(&census.both_set)
    );
    println!(
        "  Symbolic and Nonsymbolic both clear: {} documents — {}",
        census.both_clear.len(),
        list(&census.both_clear)
    );
}
