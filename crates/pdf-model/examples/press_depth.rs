//! How many *distinct* presses one interpretation names, over a population.
//!
//! `pdf_model::colour::MAX_PRESSES` is a budget on one interpretation (ADR 0417): a page that
//! names more four-component blending spaces than it allows paints the rest in its parent's
//! space and `PagePress::Beyond` says so. Trap 38 asks what such a bound owes the standard, and
//! the half of the answer no clause can give is what documents actually do — which is a
//! question about a *page*, not about a process, so `examples/press_census`'s count of the
//! distinct presses a whole population names cannot answer it.
//!
//! This interprets pages and reads `Interpretation::presses_named`, which is the number the
//! budget is compared against. It prints the deepest page in the population, the distribution,
//! and every document that came within one of the bound.
//!
//! ```sh
//! cargo run --release -p pdf-model --example press_depth -- doc/pdf.js/test/pdfs/*.pdf
//! ```
//!
//! An argument of the form `@paths.txt` names a file holding one path per line, for a corpus
//! too large for one command line. `--pages N` walks the first `N` pages of each document
//! rather than the default.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use pdf_syntax::Document;

/// How many pages of one document are interpreted unless `--pages` says otherwise.
///
/// A press is named by a page group, by a group inside the page or by a soft mask's group, so
/// every page can answer differently and the population's answer is the deepest of them. Ten is
/// what keeps a sixty-five-thousand-document walk inside an afternoon; `--pages` lifts it for a
/// population small enough to walk whole.
const DEFAULT_PAGES: usize = 10;

fn main() {
    let mut paths: Vec<String> = Vec::new();
    let mut pages_per_document = DEFAULT_PAGES;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--pages" {
            if let Some(count) = arguments.next().and_then(|value| value.parse().ok()) {
                pages_per_document = count;
            }
        } else if let Some(list) = argument.strip_prefix('@') {
            if let Ok(text) = std::fs::read_to_string(list) {
                paths.extend(text.lines().map(str::to_owned));
            }
        } else {
            paths.push(argument);
        }
    }

    let mut documents = 0_usize;
    let mut interpreted = 0_usize;
    // How many pages named exactly `n` presses, for every `n` the population reached.
    let mut distribution: BTreeMap<usize, usize> = BTreeMap::new();
    let mut deepest: Vec<(usize, String, usize)> = Vec::new();

    for path in paths {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
        let pages = pdf_model::Pages::new(&document);
        for index in 0..pages_per_document {
            let Some(page) = pages.get(index) else {
                break;
            };
            let interpretation = pdf_model::interpret(&document, &page);
            interpreted = interpreted.saturating_add(1);
            let named = interpretation.presses_named;
            let counter = distribution.entry(named).or_default();
            *counter = counter.saturating_add(1);
            // Within one of the bound is the population worth naming: a page there is one
            // group away from a refusal, and a page at the bound has already had one.
            if named.saturating_add(1) >= pdf_model::colour::MAX_PRESSES {
                deepest.push((named, name.clone(), index.saturating_add(1)));
            }
        }
    }

    println!("press depth over {documents} document(s), {interpreted} page(s) interpreted");
    for (named, pages) in &distribution {
        println!("  {pages} page(s) named {named} press(es)");
    }
    let most = distribution.keys().next_back().copied().unwrap_or(0);
    println!(
        "  deepest page names {most} press(es); the budget is {}",
        pdf_model::colour::MAX_PRESSES
    );
    deepest.sort_unstable();
    for (named, name, page) in deepest.iter().rev() {
        println!("  within one of the bound: {name} page {page} names {named}");
    }
}
