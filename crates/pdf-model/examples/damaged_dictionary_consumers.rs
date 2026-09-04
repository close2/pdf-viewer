//! Which of a file's damaged dictionaries something in the document *names*, and under what key.
//!
//! ADR 0784 built a door for §7.3.7's dictionary that stops part-way — `Document::get` still
//! answers §7.3.10's null, and `Document::damaged_dictionary` answers the entries that were whole
//! to a caller that asks for them **by name** — and left one consumer through it: `Pages`'
//! recovery, which reaches an object by its *own* `/Type /Page` declaration because the tree that
//! would have named it is what has failed.
//!
//! **This census asks the other question**, which nothing had asked: how often is such an object
//! named by a *reference* out of a dictionary that parses whole? A reference is the file's own
//! statement of what the object is for (§7.3.10), made in bytes the damage did not reach, so it
//! is the identity a second consumer would come through — and the key it is stated under is what
//! decides whether that consumer's clause can say what the missing entries cost. ADR 0866 takes
//! `/CharProcs` on §9.6.4's step b) and leaves the rest, and this is the population both halves
//! of that sentence are measured over.
//!
//! ```sh
//! cargo build --profile gates -p pdf-model --example damaged_dictionary_consumers
//! tools/bounded.sh --data 12 --tree 12 -- \
//!   <target-dir>/gates/examples/damaged_dictionary_consumers <file-or-directory>…
//! ```
//!
//! Through `tools/bounded.sh` because it is a corpus walk like any other, and a walk's cost is
//! the documents in flight rather than the documents read (ADR 0798).

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rayon::prelude::{IntoParallelRefIterator as _, ParallelIterator as _};

use pdf_syntax::{Document, Object, ObjectId};

/// One reference, out of an object that parses, to an object whose dictionary does not.
struct Naming {
    /// The key it is stated under, which is the consumer's clause in one word.
    key: String,
    /// The damaged object's number.
    number: u32,
    /// How many entries its prefix carries.
    entries: usize,
    /// Whether the referring dictionary states Table 110's `/Subtype /Type3`.
    ///
    /// Only meaningful beside a `/CharProcs` key, and it is what separates ADR 0866's population
    /// from a `/CharProcs` written by something else.
    type3: bool,
}

/// What one document says about the question.
#[derive(Default)]
struct Finding {
    /// The document opened at all.
    opened: bool,
    /// How many damaged dictionaries its bytes hold.
    damaged: usize,
    /// The ones a whole object names, with the key.
    namings: Vec<Naming>,
}

/// Every finding added up.
#[derive(Default)]
struct Totals {
    /// Files read off the disk, and those that opened.
    read: usize,
    /// As above.
    opened: usize,
    /// Documents holding at least one damaged dictionary.
    with_damage: usize,
    /// Damaged dictionaries in all.
    damaged: usize,
    /// Documents where a whole object names one.
    with_naming: usize,
    /// How many such namings there are, by the key they are stated under.
    by_key: BTreeMap<String, usize>,
    /// Of the `/CharProcs` namings, the ones whose referrer is a Type 3 font dictionary.
    char_procs_of_a_type3: usize,
}

impl Totals {
    /// Adds one document's finding.
    fn add(&mut self, finding: &Finding) {
        self.read = self.read.saturating_add(1);
        if !finding.opened {
            return;
        }
        self.opened = self.opened.saturating_add(1);
        if finding.damaged > 0 {
            self.with_damage = self.with_damage.saturating_add(1);
            self.damaged = self.damaged.saturating_add(finding.damaged);
        }
        if finding.namings.is_empty() {
            return;
        }
        self.with_naming = self.with_naming.saturating_add(1);
        for naming in &finding.namings {
            let count = self.by_key.entry(naming.key.clone()).or_default();
            *count = count.saturating_add(1);
            if naming.key == "CharProcs" && naming.type3 {
                self.char_procs_of_a_type3 = self.char_procs_of_a_type3.saturating_add(1);
            }
        }
    }

    /// Prints the summary, one claim per line.
    fn print(&self) {
        println!(
            "damaged-dictionary consumers: {} document(s) read, {} opened, {} hold a damaged \
             dictionary ({} in all), {} have one a whole object names by reference",
            self.read, self.opened, self.with_damage, self.damaged, self.with_naming
        );
        let mut keys: Vec<(&String, &usize)> = self.by_key.iter().collect();
        keys.sort_by(|left, right| right.1.cmp(left.1).then(left.0.cmp(right.0)));
        let listed: Vec<String> = keys
            .iter()
            .map(|(key, count)| format!("/{key} {count}"))
            .collect();
        println!(
            "  by the key the reference is stated under: {}",
            if listed.is_empty() {
                "none".to_owned()
            } else {
                listed.join(", ")
            }
        );
        println!(
            "  of which {} are a Type 3 font dictionary's /CharProcs, which is ADR 0866's \
             population",
            self.char_procs_of_a_type3
        );
    }
}

fn main() {
    let files = collect(std::env::args().skip(1).map(PathBuf::from));
    let findings: Vec<(PathBuf, Finding)> = files
        .par_iter()
        .map(|path| (path.clone(), examine(path)))
        .collect();

    let mut totals = Totals::default();
    let mut lines: Vec<String> = Vec::new();
    for (path, finding) in &findings {
        totals.add(finding);
        for naming in &finding.namings {
            lines.push(format!(
                "  {}: /{} names object {}, whose prefix holds {} entr(ies){}",
                path.display(),
                naming.key,
                naming.number,
                naming.entries,
                if naming.type3 {
                    " — the referrer is a Type 3 font dictionary"
                } else {
                    ""
                }
            ));
        }
    }
    lines.sort();
    for line in &lines {
        println!("{line}");
    }
    totals.print();
}

/// Reads one document and answers the census's question about it.
fn examine(path: &Path) -> Finding {
    let Ok(bytes) = std::fs::read(path) else {
        return Finding::default();
    };
    let Ok(document) = Document::open(bytes) else {
        return Finding::default();
    };
    let damaged = document.damaged_dictionaries();
    let mut finding = Finding {
        opened: true,
        damaged: damaged.len(),
        namings: Vec::new(),
    };
    if damaged.is_empty() {
        return finding;
    }
    // Only a reference out of an object that *parses* counts. That is the identity condition: the
    // statement of what the damaged object is for has to come from bytes the damage did not
    // reach, which is what a second consumer would rest on.
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        let Some(dict) = object
            .as_dict()
            .or_else(|| object.as_stream().map(|stream| &stream.dict))
        else {
            continue;
        };
        let type3 = document
            .get_key(dict, "Subtype")
            .as_name()
            .is_some_and(|name| name.as_bytes() == b"Type3");
        for (key, value) in dict.iter() {
            let Some(id) = value.as_reference() else {
                continue;
            };
            let Some(prefix) = damaged.get(&id.number) else {
                continue;
            };
            // The prefix is a second answer to a caller already refused, so the refusal has to
            // have happened: an object number bearing something readable is not this population.
            if !matches!(document.get(id), Object::Null) {
                continue;
            }
            finding.namings.push(Naming {
                key: String::from_utf8_lossy(key.as_bytes()).into_owned(),
                number: id.number,
                entries: prefix.entries.len(),
                type3,
            });
        }
    }
    finding
}

/// Every file named, and every `.pdf` under every directory named.
fn collect(paths: impl Iterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for path in paths {
        if path.is_dir() {
            walk(&path, &mut found);
        } else {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// Adds every `.pdf` under `directory`, depth first.
fn walk(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, found);
        } else if path.extension().is_some_and(|kind| kind == "pdf") {
            found.push(path);
        }
    }
}
