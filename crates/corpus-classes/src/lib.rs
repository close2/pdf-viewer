//! Every corpus on this disk, sampled at a fixed stride and classified.
//!
//! # Why a crate rather than a helper in one test
//!
//! Two instruments sweep a *confined* worker over documents that are awkward in ways the
//! `doc/pdf.js` corpus under-populates — `pdf-vfs`'s read walk, which holds every file of RFC
//! 0003 section 4's layout against the generator the layout names, and `viewer-confined`'s, which
//! opens and draws through `pdf-view-worker`. They ask different questions of different workers
//! and they need the *same* population: the vocabulary of classes, which corpora this machine
//! has, and how many documents of each are enough. Two copies of that would be two populations
//! that drift, and a difference between the two sweeps would then be a difference between their
//! samples rather than between their workers. So it is one crate, the way `test-scenes` is one
//! crate for exactly the same reason (ADR 0879).
//!
//! # What a class is, and what it is not
//!
//! A class is a property a document has, not a diagnosis and not a slot: a document is in as many
//! as it satisfies, so an encrypted, damaged, thousand-page scan is three rows of the matrix. The
//! control class is in the list because a sweep that meets only awkward documents cannot say
//! whether what it found is the class or the sweep — session 917 removed one guard and watched the
//! *control* class die more often than the encrypted one, which is what ADR 0876's
//! misattribution looks like at corpus scale.
//!
//! # What it does not do
//!
//! It opens documents and it answers a list. It draws nothing, confines nothing, mounts nothing
//! and asserts nothing: what fails a sweep is the sweep's own question, and the classes are only
//! how its report is grouped and how its population is balanced.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use pdf_syntax::{Document, FileBytes, Limits, Object, ObjectId, SyntaxError};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

/// The root every gate of `doc/todo/02` §2 walks, under the name [`roots`] reports it by.
pub const PDFJS: &str = "pdf.js";

/// A class of document that is awkward in a way worth sweeping on its own.
///
/// The vocabulary is `safedocs::survey::Outcome`'s, which already names five of these for the
/// corpus survey, plus the three session 917 added: a document too large to be swept cheaply, and
/// the two image codecs that are decoded by a separate program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Class {
    /// §7.6 encryption that opens under §7.6.4.1's default user password, or under the one the
    /// caller supplied for it.
    Encrypted,
    /// §7.6 encryption that refuses it: a person would be asked for a password.
    Locked,
    /// §7.6 encryption this reader does not implement.
    EncryptionUnread,
    /// Opens, and reaches no page.
    Pageless,
    /// Opens only because §7.5.7's cross-reference table was rebuilt by scanning.
    Damaged,
    /// Does not open at all.
    Unopenable,
    /// A hundred pages or more, or eight mebibytes or more of file.
    Huge,
    /// States a §7.4.7 `/JBIG2Decode` image, which is decoded by another program still.
    Jbig2,
    /// States a §7.4.9 `/JPXDecode` image, likewise.
    Jpeg2000,
    /// None of the above: the control.
    Plain,
}

impl Class {
    /// Every class, in the order a report prints them.
    pub const ALL: &'static [Self] = &[
        Self::Encrypted,
        Self::Locked,
        Self::EncryptionUnread,
        Self::Pageless,
        Self::Damaged,
        Self::Unopenable,
        Self::Huge,
        Self::Jbig2,
        Self::Jpeg2000,
        Self::Plain,
    ];

    /// What it is called in a report.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Encrypted => "encrypted",
            Self::Locked => "locked",
            Self::EncryptionUnread => "encryption unimplemented",
            Self::Pageless => "pageless",
            Self::Damaged => "damaged",
            Self::Unopenable => "unopenable",
            Self::Huge => "huge",
            Self::Jbig2 => "jbig2",
            Self::Jpeg2000 => "jpeg 2000",
            Self::Plain => "plain (control)",
        }
    }
}

/// One document of a population: where it is, what it is called in a report, and what it is.
#[derive(Debug, Clone)]
pub struct Chosen {
    /// Which root it came from, under the name [`roots`] reports that root by.
    ///
    /// A sweep whose depth or whose figures differ per root asks this rather than parsing
    /// [`Self::display`], which is a *label*.
    pub root: String,
    /// `<root>/<file name>`, so that two roots holding one name are two rows of a report.
    pub display: String,
    /// The file name alone, which is what a caller's password table is keyed by.
    pub name: String,
    /// Where it is on this disk.
    pub path: PathBuf,
    /// Every class it falls into, in [`Class::ALL`]'s order.
    pub classes: Vec<Class>,
}

/// What one root contributed to a population.
#[derive(Debug, Clone)]
pub struct Contribution {
    /// The root's name in a report.
    pub root: String,
    /// How many of its documents were opened and classified.
    pub classified: usize,
    /// How many of them the population took.
    pub chosen: usize,
}

/// How a population is drawn: which roots are taken whole, and how much of the rest.
#[derive(Debug, Clone)]
pub struct Choice {
    /// Roots taken whole rather than sampled, by the name [`roots`] reports them under.
    ///
    /// A sweep whose figures are compared across sessions names the root those figures were over
    /// here, so that widening the population does not silently move them.
    pub whole: Vec<String>,
    /// How many documents are classified from each root that is *not* taken whole.
    ///
    /// A stride over the sorted names rather than the first N, because a corpus directory's first
    /// hundred names are one contributor's and one generator's.
    pub sample_per_root: usize,
    /// How many documents of each class are taken from each root that is sampled.
    pub per_class: usize,
}

/// The corpus roots on this disk, each with the name it is reported under.
///
/// [`PDFJS`] first and by name, because it is the root every gate of `doc/todo/02` §2 walks; the
/// rest are whatever `doc/corpora/` and `corpus-cache/` hold, which is machine-local by design
/// (`doc/environment.md`) and is why a report names each root and its count rather than saying
/// "the corpus" (`doc/todo/02` §4's `undenominated` sweep).
#[must_use]
pub fn roots() -> Vec<(String, PathBuf)> {
    let tree = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut found = Vec::new();
    let mut consider = |name: &str, path: PathBuf| {
        if path.is_dir() {
            found.push((name.to_owned(), path));
        }
    };
    consider(PDFJS, tree.join("doc/pdf.js/test/pdfs"));
    for corpora in [tree.join("doc/corpora"), tree.join("corpus-cache")] {
        let Ok(entries) = std::fs::read_dir(&corpora) else {
            continue;
        };
        let mut names: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        names.sort();
        for path in names {
            if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                let name = name.to_owned();
                consider(&name, path);
            }
        }
    }
    found
}

/// Every `.pdf` under a root, sorted, however deep.
#[must_use]
pub fn documents_under(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// A fixed-stride sample of a population, so that a root's first names are not the whole answer.
#[must_use]
pub fn sampled(documents: Vec<PathBuf>, most: usize) -> Vec<PathBuf> {
    if documents.len() <= most {
        return documents;
    }
    let stride = documents.len().checked_div(most).unwrap_or(1).max(1);
    documents.into_iter().step_by(stride).take(most).collect()
}

/// Which classes this document falls into, by opening it with this password and looking.
///
/// The password is the sweep's own: a document a sweep can open because it knows the word for it
/// is *encrypted* there and would be *locked* to a reader that does not, and a class that does not
/// say what the sweep meets is a class the sweep's report cannot be read by.
#[must_use]
pub fn classify(path: &Path, password: &str) -> Vec<Class> {
    let mut classes = Vec::new();
    let Ok(metadata) = std::fs::metadata(path) else {
        return classes;
    };
    let Ok(bytes) = FileBytes::on_disk(path) else {
        return classes;
    };
    let document = match Document::open_with_password(bytes, Limits::DEFAULT, password) {
        Ok(document) => document,
        Err(SyntaxError::PasswordRequired) => {
            classes.push(Class::Locked);
            return classes;
        }
        Err(error) => {
            classes.push(if error.to_string().contains("encrypt") {
                Class::EncryptionUnread
            } else {
                Class::Unopenable
            });
            return classes;
        }
    };

    if document.is_encrypted() {
        classes.push(Class::Encrypted);
    }
    if document.was_recovered() {
        classes.push(Class::Damaged);
    }
    let pages = pdf_model::Pages::new(&document).len();
    if pages == 0 {
        classes.push(Class::Pageless);
    }
    if pages >= 100 || metadata.len() >= 8 << 20 {
        classes.push(Class::Huge);
    }
    for (filter, class) in [
        ("JBIG2Decode", Class::Jbig2),
        ("JPXDecode", Class::Jpeg2000),
    ] {
        if states_filter(&document, filter) {
            classes.push(class);
        }
    }
    if classes.is_empty() {
        classes.push(Class::Plain);
    }
    classes
}

/// Whether any object of this document is a stream filtered by this name.
///
/// The objects are walked rather than the bytes scanned, because a §7.5.7 compressed object
/// stream hides every name a `grep` would look for, and those are exactly the documents a modern
/// producer writes.
fn states_filter(document: &Document, filter: &str) -> bool {
    document.xref().object_numbers().any(|number| {
        let object = document.get(ObjectId::new(number, 0));
        let Object::Stream(stream) = &object else {
            return false;
        };
        match &document.get_key(&stream.dict, "Filter") {
            Object::Name(name) => name.as_str() == Some(filter),
            Object::Array(names) => names.iter().any(|entry| {
                matches!(document.resolve(entry), Object::Name(name) if name.as_str() == Some(filter))
            }),
            _ => false,
        }
    })
}

/// The population a sweep walks: every document of the roots taken whole, and a class-balanced
/// sample of every other root.
///
/// Balanced *per root* rather than over the whole population, so that one collection of sixty-five
/// thousand documents cannot fill every class by itself. A document is taken when **any** of its
/// classes is still under [`Choice::per_class`] for that root, and then counts against all of
/// them: a class is a property rather than a slot, and one whose quota fills because its documents
/// arrived on another class's coat-tails is a class the sample holds *more* of, not less.
///
/// `password_for` answers the word a sweep knows for a file name, or the empty string; it is
/// called from several threads, which is why it is `Sync`.
#[must_use]
pub fn population(
    roots: &[(String, PathBuf)],
    choice: &Choice,
    password_for: &(dyn Fn(&str) -> String + Sync),
) -> (Vec<Chosen>, Vec<Contribution>) {
    let mut chosen: Vec<Chosen> = Vec::new();
    let mut contributions = Vec::new();
    for (root, directory) in roots {
        let whole = choice.whole.iter().any(|taken| taken == root);
        let documents = documents_under(directory);
        let sample = if whole {
            documents
        } else {
            sampled(documents, choice.sample_per_root)
        };
        let classified = sample.len();
        let verdicts = Mutex::new(Vec::new());
        sample.par_iter().for_each(|path| {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let classes = classify(path, &password_for(&name));
            if let Ok(mut verdicts) = verdicts.lock() {
                verdicts.push((path.clone(), name, classes));
            }
        });
        let mut verdicts = verdicts.into_inner().unwrap_or_default();
        verdicts.sort_by(|left, right| left.0.cmp(&right.0));
        let mut taken: BTreeMap<Class, usize> = BTreeMap::new();
        let before = chosen.len();
        for (path, name, classes) in verdicts {
            let wanted = whole
                || classes
                    .iter()
                    .any(|class| taken.get(class).copied().unwrap_or_default() < choice.per_class);
            if !wanted {
                continue;
            }
            for class in &classes {
                let count = taken.entry(*class).or_default();
                *count = count.saturating_add(1);
            }
            chosen.push(Chosen {
                root: root.clone(),
                display: format!("{root}/{name}"),
                name,
                path,
                classes,
            });
        }
        contributions.push(Contribution {
            root: root.clone(),
            classified,
            chosen: chosen.len().saturating_sub(before),
        });
    }
    (chosen, contributions)
}

/// Whether a sentence a worker's refusal came back with is a worker that *died*.
///
/// `confined_transport::supervision` words a signal death as `killed by signal N`, and that
/// sentence is what reaches a sweep — through whichever question was being asked when the worker
/// went, which is why the predicate is on the sentence rather than on one error variant. A death
/// is what a face's user sees as a folder that stops answering, and it is what these sweeps exist
/// to find; a refusal is a sentence a face can *show*, and is not a failure (trap 11).
#[must_use]
pub fn is_a_death(sentence: &str) -> bool {
    sentence.contains("killed by signal")
}
