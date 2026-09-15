//! §12.2's print half of Table 147, counted: which documents ask for something on paper.
//!
//! Eight of Table 147's entries state a `shall` whose condition is printing — `/PrintArea`,
//! `/PrintClip`, `/PrintScaling`, `/Duplex`, `/PickTrayByPDFSize`, `/PrintPageRange`,
//! `/NumCopies`, and `/Enforce` where it names the third of them. Every one of them is read by
//! [`pdf_model::viewer_preferences`] and handed to a host; not one reaches paper, because this
//! program has no print operation. What this census answers is how large the population behind
//! that sentence is, and — more usefully — how much of it would even *notice*: an entry stated
//! at Table 147's own default asks for what a processor would have done anyway.
//!
//! So each entry is counted twice, stated and consequential:
//!
//! - **`/PrintArea` and `/PrintClip`** default to `CropBox`, which is what this program already
//!   renders and clips to on screen (§14.11.2.1). A document naming any other boundary is one
//!   whose printed page would differ from its screen page, and those are the only ones a
//!   `Page::print_box` would have anything to do for.
//! - **`/PrintScaling`** is `AppDefault` by default and unrecognised values fall back to it, so
//!   only `/None` — "no page scaling" — states anything. Table 148's `/Enforce` is counted
//!   beside it because it is what makes the entry binding rather than advisory.
//! - **`/Duplex`, `/PickTrayByPDFSize`, `/PrintPageRange` and `/NumCopies`** have no stated
//!   default at all ("implementation dependent"), so stating one *is* the consequence; the
//!   second count for these is the value itself.
//!
//! **Table 147's print half is the only print-conditioned requirement a census can reach.**
//! §12.5.6.22's two remaining bullets — page tiling and n-up — are conditioned on a *selection*
//! ("[w]hen page tiling is selected in a PDF processor", "[w]hen n-up printing is selected")
//! rather than on anything a file states, so no count of documents can rank them and none is
//! attempted here (trap 8).
//!
//! ```sh
//! cargo run --release -p pdf-model --example print_preference_census
//! cargo run --release -p pdf-model --example print_preference_census -- --crawl
//! ```

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "an example whose entire output is a measurement"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "counters over a corpus of a few tens of thousands of files; a measurement rather \
              than a shipped path"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_model::page::Boundary;
use pdf_model::viewer_preferences::{Duplex, PrintScaling, ViewerPreferences};
use pdf_syntax::Document;
use rayon::prelude::*;

/// How many witnessing documents are named per entry before the list is truncated.
const MAX_NAMED: usize = 10;

/// Which population a run is over.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    /// The pdf.js corpus alone — "the 974", which most of this project's claims are about.
    PdfJs,
    /// That, the four `doc/corpora/` submodules, and this project's own fixtures.
    Curated,
    /// The `SafeDocs` `CC-MAIN-2021-31` crawl under `corpus-cache/`, and nothing else.
    Crawl,
}

/// What one document states, in the standard's own terms, about printing.
///
/// One `Option<String>` per counted claim, holding the value that made it true, because a count
/// on its own cannot say whether the population is worth a capability: `/NumCopies 1` and
/// `/NumCopies 500` are the same tally and very different documents.
#[derive(Default)]
struct Stated {
    /// Whether the catalog states a `/ViewerPreferences` dictionary at all.
    dictionary: bool,
    /// `/PrintArea` or `/PrintClip` naming a boundary other than Table 147's `CropBox` default.
    boundary: Option<String>,
    /// `/PrintScaling /None` — the one value that is not `AppDefault`.
    no_scaling: bool,
    /// Table 148's `/Enforce` making that binding.
    enforced: bool,
    /// `/Duplex`, with the name stated.
    duplex: Option<String>,
    /// `/PickTrayByPDFSize`, with the flag stated.
    pick_tray: Option<String>,
    /// `/PrintPageRange`, with the pairs it resolved to.
    page_range: Option<String>,
    /// `/NumCopies`, with the count stated.
    copies: Option<String>,
}

impl Stated {
    /// Whether this document asks for anything on paper that a screen would not give it.
    fn asks_for_paper(&self) -> bool {
        self.boundary.is_some()
            || self.no_scaling
            || self.duplex.is_some()
            || self.pick_tray.is_some()
            || self.page_range.is_some()
            || self.copies.is_some()
    }
}

/// Reads one document's print half, through the reader a print path would use.
///
/// Through [`ViewerPreferences`] rather than by looking for the tokens, for ADR 0403's reason:
/// the entry as the code would act on it is the population, and the two differ — an
/// unrecognised `/PrintScaling` is `AppDefault`, an odd-length `/PrintPageRange` states no
/// range, and `/Enforce` is conditioned on the value it names.
fn measure(path: &Path) -> Stated {
    let Ok(bytes) = std::fs::read(path) else {
        return Stated::default();
    };
    let Ok(document) = Document::open(bytes) else {
        return Stated::default();
    };
    let Ok(catalog) = document.catalog() else {
        return Stated::default();
    };
    let dictionary = document
        .get_key(&catalog, "ViewerPreferences")
        .as_dict()
        .is_some();
    let preferences = ViewerPreferences::in_catalog(&document, &catalog);

    let named = |boundary: Boundary| match boundary {
        Boundary::Media => "MediaBox",
        Boundary::Crop => "CropBox",
        Boundary::Bleed => "BleedBox",
        Boundary::Trim => "TrimBox",
        Boundary::Art => "ArtBox",
    };
    let boundary = (preferences.print_area != Boundary::Crop
        || preferences.print_clip != Boundary::Crop)
        .then(|| {
            format!(
                "/PrintArea /{} /PrintClip /{}",
                named(preferences.print_area),
                named(preferences.print_clip)
            )
        });

    Stated {
        dictionary,
        boundary,
        no_scaling: preferences.print_scaling == PrintScaling::NoScaling,
        enforced: preferences.enforce_print_scaling,
        duplex: preferences.duplex.map(|duplex| {
            match duplex {
                Duplex::Simplex => "Simplex",
                Duplex::FlipShortEdge => "DuplexFlipShortEdge",
                Duplex::FlipLongEdge => "DuplexFlipLongEdge",
            }
            .to_owned()
        }),
        pick_tray: preferences
            .pick_tray_by_pdf_size
            .map(|flag| flag.to_string()),
        page_range: (!preferences.print_page_range.is_empty()).then(|| {
            preferences
                .print_page_range
                .iter()
                .map(|(first, last)| format!("{first}-{last}"))
                .collect::<Vec<_>>()
                .join(",")
        }),
        copies: preferences.num_copies.map(|count| count.to_string()),
    }
}

/// Every PDF this project can measure over, in the scope asked for.
fn corpus(scope: Scope) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    let scope: &[&str] = match scope {
        Scope::PdfJs => &["doc/pdf.js/test/pdfs"],
        Scope::Curated => &["doc/pdf.js/test/pdfs", "doc/corpora", "doc/corpora-own"],
        Scope::Crawl => &["corpus-cache/safedocs/cc-main-2021-31"],
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

/// Prints one line per entry: how many documents state it, and what they state.
fn report(title: &str, results: &[(String, Stated)], of: impl Fn(&Stated) -> Option<&str>) {
    let mut values: BTreeMap<&str, usize> = BTreeMap::new();
    let mut named = Vec::new();
    for (label, stated) in results {
        if let Some(value) = of(stated) {
            *values.entry(value).or_default() += 1;
            if named.len() < MAX_NAMED {
                named.push(label.as_str());
            }
        }
    }
    let total: usize = values.values().sum();
    if total == 0 {
        println!("{title}: none");
        return;
    }
    let spread: Vec<String> = values
        .iter()
        .map(|(value, count)| format!("{value} x{count}"))
        .collect();
    println!("{title}: {total} — {}", spread.join("; "));
    println!("    {}", named.join(" "));
}

fn main() {
    let scope = if std::env::args().any(|a| a == "--crawl") {
        Scope::Crawl
    } else if std::env::args().any(|a| a == "--pdfjs") {
        Scope::PdfJs
    } else {
        Scope::Curated
    };
    let files = corpus(scope);
    eprintln!("{} PDF(s) in the population", files.len());

    let results: Vec<(String, Stated)> = files
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

    let dictionaries = results.iter().filter(|(_, s)| s.dictionary).count();
    let asking = results.iter().filter(|(_, s)| s.asks_for_paper()).count();
    println!(
        "{} document(s) opened with a catalog, {dictionaries} stating a /ViewerPreferences, \
         {asking} stating something in Table 147's print half that is not its default",
        results.len()
    );

    report(
        "/PrintArea or /PrintClip naming a boundary other than CropBox",
        &results,
        |s| s.boundary.as_deref(),
    );
    let no_scaling = results.iter().filter(|(_, s)| s.no_scaling).count();
    let enforced = results.iter().filter(|(_, s)| s.enforced).count();
    println!(
        "/PrintScaling /None: {no_scaling} — of which {enforced} are made binding by Table 148's \
         /Enforce"
    );
    report("/Duplex", &results, |s| s.duplex.as_deref());
    report("/PickTrayByPDFSize", &results, |s| s.pick_tray.as_deref());
    report("/PrintPageRange", &results, |s| s.page_range.as_deref());
    report("/NumCopies", &results, |s| s.copies.as_deref());
}
