//! What a corpus writes in ISO 32000-2 §9.8.3.3's `/FD`, and which of it could change a page.
//!
//! Table 122 gives the entry as "A dictionary whose keys identify a class of glyphs in a
//! `CIDFont`. Each value shall be a dictionary containing entries that shall override the
//! corresponding values in the main font descriptor dictionary for that class of glyphs", and
//! §9.8.3.3's Table 123 lists the valid keys per character collection. This tree reads a
//! descriptor for one thing a page can see — which installed face stands in for a font the
//! document did not embed — so the population that matters is narrower than the population that
//! states the entry, and this counts both:
//!
//! - every font descriptor stating `/FD` at all, with the class names it uses and the entries
//!   each class descriptor states;
//! - how many of those belong to a `CIDFont` that embeds its program, which never reaches
//!   substitution and whose `/FD` therefore cannot move a glyph here;
//! - the `/CIDSystemInfo` registry and ordering of each, which is what §9.8.3.3 makes the class
//!   names depend on.
//!
//! A descriptor is any dictionary object whose `/Type` is `FontDescriptor`, or which states both
//! `/FontName` and `/Flags` without one — the same rule `font_flags_census` uses. Walks
//! `doc/pdf.js/test/pdfs` and every `.pdf` under `doc/corpora/`, or the directories named on the
//! command line.
//!
//! ```sh
//! cargo run --release -p pdf-model --example glyph_class_census
//! cargo run --release -p pdf-model --example glyph_class_census -- corpus-cache/openpreserve
//! ```
#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a measurement whose output is its purpose: it stops loudly where the corpus is \
              missing, and its counters over the corpus's descriptors are four orders of \
              magnitude below what a usize counts"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

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

/// Table 123's class names, for every collection the clause tabulates.
const TABULATED: &[&str] = &[
    "Alphabetic",
    "AlphaNum",
    "Dingbats",
    "DingbatsRot",
    "Generic",
    "GenericRot",
    "Hangul",
    "Hanja",
    "Hanzi",
    "HKana",
    "HKanaRot",
    "HojoKanji",
    "HRoman",
    "HRomanRot",
    "Kana",
    "Kanji",
    "Proportional",
    "ProportionalRot",
    "Ruby",
];

/// Everything this census keeps.
#[derive(Default)]
struct Census {
    documents: usize,
    descriptors: usize,
    /// Descriptors stating `/FD`, by document.
    stating: BTreeSet<String>,
    /// How many `/FD` dictionaries there are, over every document.
    dictionaries: usize,
    /// Class names used, and how many `/FD` dictionaries use each.
    classes: BTreeMap<String, usize>,
    /// Class names Table 123 does not tabulate.
    untabulated: BTreeSet<String>,
    /// Entries the class descriptors state, and how many state each.
    entries: BTreeMap<String, usize>,
    /// `/FD` on a descriptor whose `CIDFont` embeds its program — never substituted.
    embedded: BTreeSet<String>,
    /// `/FD` on a descriptor whose `CIDFont` embeds nothing — the population a face is chosen for.
    not_embedded: BTreeSet<String>,
    /// The registry and ordering of each `CIDFont` whose descriptor states `/FD`.
    collections: BTreeMap<String, usize>,
    /// Class descriptors restating an entry with the value the main descriptor already has.
    same: BTreeMap<String, usize>,
    /// Class descriptors stating an entry whose value differs from the main descriptor's.
    differing: BTreeMap<String, usize>,
    /// `/FD` dictionaries naming more than one class.
    several: BTreeSet<String>,
}

/// Whether a descriptor carries any of §9.9's three font-program streams.
fn embeds(document: &Document, descriptor: &Dictionary) -> bool {
    ["FontFile", "FontFile2", "FontFile3"]
        .iter()
        .any(|key| document.get_key(descriptor, key).as_stream().is_some())
}

/// The `Registry-Ordering` of a `CIDFont`'s `/CIDSystemInfo`, where it states one.
fn collection(document: &Document, descendant: &Dictionary) -> String {
    let info = document.get_key(descendant, "CIDSystemInfo");
    let Some(info) = info.as_dict() else {
        return "(no /CIDSystemInfo)".to_owned();
    };
    let read = |key: &str| {
        document.get_key(info, key).as_string().map_or_else(
            || "?".to_owned(),
            |bytes| String::from_utf8_lossy(bytes).into_owned(),
        )
    };
    format!("{}-{}", read("Registry"), read("Ordering"))
}

impl Census {
    fn count(&mut self, name: &str, document: &Document) {
        self.documents += 1;

        // Which descriptor objects a CIDFont selects, so that the embedded question and the
        // character collection can be asked of the font rather than of the descriptor, which
        // states neither.
        let mut descendants: BTreeMap<u32, Dictionary> = BTreeMap::new();
        for number in document.xref().object_numbers() {
            let object = document.get(ObjectId {
                number,
                generation: 0,
            });
            let Some(dict) = object.as_dict() else {
                continue;
            };
            let subtype = document.get_key(dict, "Subtype");
            let Some(subtype) = subtype.as_name() else {
                continue;
            };
            if !matches!(subtype.as_bytes(), b"CIDFontType0" | b"CIDFontType2") {
                continue;
            }
            if let Some(Object::Reference(id)) = dict.get("FontDescriptor") {
                descendants.insert(id.number, dict.clone());
            }
        }

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
            let shaped = !matches!(document.get_key(dict, "FontName"), Object::Null)
                && !matches!(document.get_key(dict, "Flags"), Object::Null);
            if !typed && !shaped {
                continue;
            }
            self.descriptors += 1;
            let classes = document.get_key(dict, "FD");
            let Some(classes) = classes.as_dict() else {
                continue;
            };
            self.stating.insert(name.to_owned());
            self.dictionaries += 1;
            if classes.iter().count() > 1 {
                self.several.insert(name.to_owned());
            }
            for (key, value) in classes.iter() {
                let class = String::from_utf8_lossy(key.as_bytes()).into_owned();
                if !TABULATED.contains(&class.as_str()) {
                    self.untabulated.insert(format!("{name}: /{class}"));
                }
                *self.classes.entry(class).or_default() += 1;
                let Some(over) = document.resolve(value).as_dict().cloned() else {
                    continue;
                };
                for (entry, value) in over.iter() {
                    let key = String::from_utf8_lossy(entry.as_bytes()).into_owned();
                    *self.entries.entry(key.clone()).or_default() += 1;
                    let main = document.get_key(dict, &key);
                    let overriding = document.resolve(value);
                    if format!("{main:?}") == format!("{overriding:?}") {
                        *self.same.entry(key).or_default() += 1;
                    } else {
                        *self.differing.entry(key).or_default() += 1;
                    }
                }
            }
            match descendants.get(&number) {
                Some(descendant) => {
                    *self
                        .collections
                        .entry(collection(document, descendant))
                        .or_default() += 1;
                    if embeds(document, dict) {
                        self.embedded.insert(name.to_owned());
                    } else {
                        self.not_embedded.insert(name.to_owned());
                    }
                }
                None => {
                    *self
                        .collections
                        .entry("(no CIDFont)".to_owned())
                        .or_default() += 1;
                }
            }
        }
    }
}

fn tally(counts: &BTreeMap<String, usize>) -> String {
    counts
        .iter()
        .map(|(name, count)| format!("{name} x{count}"))
        .collect::<Vec<_>>()
        .join(", ")
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
        "{} documents, {} font descriptors, {} of them stating /FD, in {} documents — {}",
        census.documents,
        census.descriptors,
        census.dictionaries,
        census.stating.len(),
        list(&census.stating)
    );
    println!("  classes named: {}", tally(&census.classes));
    println!(
        "  not tabulated by Table 123: {} — {}",
        census.untabulated.len(),
        list(&census.untabulated)
    );
    println!(
        "  entries the class descriptors state: {}",
        tally(&census.entries)
    );
    println!("  character collections: {}", tally(&census.collections));
    println!(
        "  /FD dictionaries naming more than one class: {} documents — {}",
        census.several.len(),
        list(&census.several)
    );
    println!(
        "  entries restating the main descriptor's own value: {}",
        tally(&census.same)
    );
    println!(
        "  entries stating a value that differs: {}",
        tally(&census.differing)
    );
    println!(
        "  program embedded (never substituted): {} documents — {}",
        census.embedded.len(),
        list(&census.embedded)
    );
    println!(
        "  program not embedded (a face is chosen): {} documents — {}",
        census.not_embedded.len(),
        list(&census.not_embedded)
    );
}
