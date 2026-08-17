//! Re-asks, with this tree's own readers, the "no corpus document does X" claims it repeats.
//!
//! The companion to `witness_census`, and the reason there are two. A name census answers
//! "does the token appear", which over-reports: a document may state `/IDTree` and have the tree
//! resolve to nothing, and a `/Threads` array may be empty. What decides a claim like "no corpus
//! document states an article" is the *structure* the claim is about, read by the code that would
//! act on it — which is the instrument ADR 0403 says must be run beside the grep rather than
//! instead of it.
//!
//! Each block below is one written claim, cited to where this tree states it, and prints the
//! population that would falsify it. **Nothing here asserts**: it is a measurement, and the
//! claims it settles are corrected in prose where they are written rather than pinned here.
//!
//! ```sh
//! cargo run --release -p pdf-model --example absence_audit
//! cargo run --release -p pdf-model --example absence_audit -- --pdfjs
//! ```

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "an example whose entire output is a measurement"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "counters over a corpus of a few thousand files; a measurement rather than a \
              shipped path"
)]

use std::path::{Path, PathBuf};

use pdf_syntax::{Document, Object};
use rayon::prelude::*;

/// Every PDF this project can measure over, or the pdf.js corpus alone under `--pdfjs`.
fn corpus(pdfjs_only: bool) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    let scope: &[&str] = if pdfjs_only {
        &["doc/pdf.js/test/pdfs"]
    } else {
        &["doc/pdf.js/test/pdfs", "doc/corpora", "doc/corpora-own"]
    };
    for relative in scope {
        collect(&root.join(relative), &mut files);
    }
    files.sort();
    files.dedup();
    files
}

/// Every `.pdf` under one directory, recursively.
fn collect(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, into);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
        {
            into.push(path);
        }
    }
}

/// What one document answered to each claim, as a sentence naming what it holds.
#[derive(Default)]
struct Answers {
    /// §14.7.2's `/IDTree`, and how many identifiers it names.
    id_tree: Option<String>,
    /// §12.4.3's threads, and how many beads hang off them.
    articles: Option<String>,
    /// §12.3.5's `/Collection`.
    collection: Option<String>,
    /// §12.9's `/VP` viewports on any page.
    viewports: Option<String>,
    /// §14.10.2's `/SpiderInfo`, and on what.
    spider: Option<String>,
    /// §12.7.5.5's `/Lock` on a signature field.
    field_lock: Option<String>,
    /// §12.8.2.4's `FieldMDP` transform.
    field_mdp: Option<String>,
}

fn main() {
    let pdfjs_only = std::env::args().any(|a| a == "--pdfjs");
    let files = corpus(pdfjs_only);
    eprintln!("{} PDF(s) in the population", files.len());

    let results: Vec<(String, Answers)> = files
        .par_iter()
        .map(|path| {
            let label = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            (label, measure(path))
        })
        .collect();

    report(
        "§14.7.2's /IDTree — the claim was \"none at all\" and is FALSE (ADR 0405)",
        &results,
        |a| a.id_tree.as_deref(),
    );
    report(
        "§12.4.3's articles — the claim was \"none\"; true of pdf.js, FALSE wider (ADR 0405)",
        &results,
        |a| a.articles.as_deref(),
    );
    report(
        "§12.3.5's /Collection — the claim was \"none\"; true of pdf.js, FALSE wider (ADR 0405)",
        &results,
        |a| a.collection.as_deref(),
    );
    report(
        "§12.9's /VP — §12.9.2 has no witness and this SURVIVES: the one /VP is GEO, not RL",
        &results,
        |a| a.viewports.as_deref(),
    );
    report(
        "§14.10.2's /SpiderInfo — the claim was \"none\" and is FALSE (ADR 0405)",
        &results,
        |a| a.spider.as_deref(),
    );
    report(
        "§12.7.5.5's /Lock — the claim is \"none\" and it SURVIVES",
        &results,
        |a| a.field_lock.as_deref(),
    );
    report(
        "§12.8.2.4's FieldMDP — the transform, over the whole population",
        &results,
        |a| a.field_mdp.as_deref(),
    );
}

/// Prints one claim's witnesses, or says it has none.
fn report(claim: &str, results: &[(String, Answers)], pick: impl Fn(&Answers) -> Option<&str>) {
    let found: Vec<String> = results
        .iter()
        .filter_map(|(label, answers)| pick(answers).map(|what| format!("{label}: {what}")))
        .collect();
    println!();
    println!("{claim}");
    if found.is_empty() {
        println!("  no witness in this population");
        return;
    }
    println!("  {} witness(es)", found.len());
    for entry in &found {
        println!("    {entry}");
    }
}

/// Asks one document every claim.
///
/// One block per written claim, each cited to the clause it is about: splitting them into
/// helpers would separate a claim from the reader that settles it, which is the whole point of
/// this example existing beside `witness_census`.
fn measure(path: &Path) -> Answers {
    let mut answers = Answers::default();
    let Ok(bytes) = std::fs::read(path) else {
        return answers;
    };
    let Ok(document) = Document::open(bytes) else {
        return answers;
    };

    // §14.7.2's Table 354 `/IDTree`, resolved as §7.9.6's name tree rather than counted as a key:
    // an entry present and empty would not falsify the claim.
    if let Ok(catalog) = document.catalog() {
        let root = document.get_key(&catalog, "StructTreeRoot");
        if let Some(root) = root.as_dict() {
            let entry = document.get_key(root, "IDTree");
            if let Some(dict) = entry.as_dict() {
                let pairs = pdf_syntax::tree::name_pairs(dict, &|object| document.resolve(object));
                answers.id_tree = Some(format!("{} identifier(s)", pairs.len()));
            }
        }
    }

    // §12.4.3's threads and beads, through the reader §12.4.3's panel uses.
    let articles = pdf_model::article::Articles::read(&document);
    if !articles.is_empty() {
        let beads: usize = articles.threads.iter().map(|t| t.beads.len()).sum();
        answers.articles = Some(format!(
            "{} thread(s), {beads} bead(s)",
            articles.threads.len()
        ));
    }

    // §12.3.5's `/Collection`, through the reader the sidebar's folder tree uses.
    if let Some(collection) = pdf_model::collection::Collection::read(&document) {
        answers.collection = Some(format!(
            "{} field(s), {} folder(s)",
            collection.schema.len(),
            collection.all_folders().len()
        ));
    }

    // §12.9's `/VP`, asked of every page rather than of page one: a viewport is a region of a
    // page and a document that states one need not state it first.
    let pages = pdf_model::Pages::new(&document);
    let mut viewports = 0usize;
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        viewports += pdf_model::measurement::Viewports::read(&document, &page.dict)
            .viewports
            .len();
    }
    if viewports > 0 {
        answers.viewports = Some(format!("{viewports} viewport(s)"));
    }

    // §14.10.2's `/SpiderInfo`, which Table 28 puts on the catalog and Table 358 on a structure
    // element's `/K`; asked of every object, because the claim is about the document.
    let mut spider = Vec::new();
    for number in document.xref().object_numbers() {
        let object = document.get(pdf_syntax::ObjectId {
            number,
            generation: 0,
        });
        let dict = match &object {
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => &stream.dict,
            _ => continue,
        };
        if !document.get_key(dict, "SpiderInfo").is_null() {
            let owner = document.get_key(dict, "Type");
            spider.push(match owner.as_name() {
                Some(name) => String::from_utf8_lossy(name.as_bytes()).into_owned(),
                None => "untyped".to_owned(),
            });
        }
    }
    if !spider.is_empty() {
        spider.sort();
        spider.dedup();
        answers.spider = Some(format!("on /Type {}", spider.join(", ")));
    }

    // §12.7.5.5 and §12.8.2.4, the pair ADR 0403 corrected, re-asked over the wider population.
    let locks = pdf_model::signature::field_locks(&document);
    if !locks.is_empty() {
        answers.field_lock = Some(format!("{locks:?}"));
    }
    let covered = pdf_model::signature::field_mdp(&document);
    if !covered.is_empty() {
        answers.field_mdp = Some(format!("{covered:?}"));
    }

    answers
}
