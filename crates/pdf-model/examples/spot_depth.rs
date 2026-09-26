//! How many spot colourants one page names, over a population.
//!
//! ISO 32000-2 §10.8.3's step a) gives the simulated device a plane per spot colourant, and
//! `pdf_model::colourants::spot_colourants` is the enumeration that sizes it before the first
//! mark. No clause bounds how many colourants a page may name — §8.6.6.4 allows names "subject to
//! implementation limits" and §8.6.6.5 a `DeviceN` of "an arbitrary number of colour components"
//! — so trap 38's question has the answer *the standard states none*, and the bound on planes is
//! `examples/press_depth`'s shape (ADR 1254): what documents actually do, measured per page.
//!
//! This reads `SpotColourants::len` for each page without interpreting it, because the count is
//! a property of the page's resources rather than of its content stream. It prints the
//! distribution, the largest page, and every page at or above the bound the tree carries.
//!
//! ```sh
//! cargo run --release -p pdf-model --example spot_depth -- @paths.txt
//! ```
//!
//! An argument of the form `@paths.txt` names a file holding one path per line, for a corpus
//! too large for one command line. `--pages N` walks the first `N` pages of each document
//! rather than the default.
//!
//! `--separate` also interprets every page naming a spot colourant under the reader's request for
//! the simulation and counts the pages §10.8.3's separation is made for against the pages it is
//! given up on, naming each of the second with the reason its report gives
//! (`Unsupported::SeparationGivenUp`, ADR 1329) — the population a page falls back from the press
//! to one painting operation's simulation on (ADR 1317). A page given up with no such report is
//! printed as a defect, because the report is what makes the fallback loud.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use pdf_syntax::Document;

/// How many pages of one document are read unless `--pages` says otherwise.
///
/// Ten, `examples/press_depth`'s number, for the same reason: it keeps a walk over the whole
/// crawl inside an afternoon, and `--pages` lifts it for a population small enough to walk whole.
const DEFAULT_PAGES: usize = 10;

fn main() {
    let mut paths: Vec<String> = Vec::new();
    let mut pages_per_document = DEFAULT_PAGES;
    let mut separate = false;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--separate" {
            separate = true;
        } else if argument == "--pages" {
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

    let bound = pdf_model::colourants::MAX_SPOT_COLOURANTS;
    let mut documents = 0_usize;
    let mut read = 0_usize;
    // How many pages named exactly `n` spot colourants, for every `n` the population reached.
    let mut distribution: BTreeMap<usize, usize> = BTreeMap::new();
    // The documents with a page naming at least one, for the population's second number.
    let mut documents_with_spots = 0_usize;
    let mut largest: Vec<(usize, String, usize)> = Vec::new();
    let mut separated = 0_usize;
    let mut given_up: Vec<(String, usize, String)> = Vec::new();
    // How many pages each reason gave up, for the split the record prints.
    let mut reasons: BTreeMap<String, usize> = BTreeMap::new();

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
        let mut any = false;
        for index in 0..pages_per_document {
            let Some(page) = pages.get(index) else {
                break;
            };
            let spots = pdf_model::colourants::spot_colourants(&document, &page).len();
            read = read.saturating_add(1);
            let counter = distribution.entry(spots).or_default();
            *counter = counter.saturating_add(1);
            any |= spots > 0;
            if spots.saturating_add(2) >= bound {
                largest.push((spots, name.clone(), index.saturating_add(1)));
            }
            if separate && spots > 0 {
                let mut state = pdf_model::view::ViewState::of(&document);
                state.set_separation_simulation(true);
                let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
                if interpretation.separation.is_some() {
                    separated = separated.saturating_add(1);
                } else {
                    let reason = given_up_reason(&interpretation);
                    let kind = kind_of(&reason);
                    let counter = reasons.entry(kind).or_default();
                    *counter = counter.saturating_add(1);
                    given_up.push((name.clone(), index.saturating_add(1), reason));
                }
            }
        }
        if any {
            documents_with_spots = documents_with_spots.saturating_add(1);
        }
    }

    println!("spot depth over {documents} document(s), {read} page(s) read");
    println!("  {documents_with_spots} document(s) have a page naming a spot colourant");
    for (spots, pages) in &distribution {
        println!("  {pages} page(s) named {spots} spot colourant(s)");
    }
    let most = distribution.keys().next_back().copied().unwrap_or(0);
    println!("  largest page names {most}; the bound is {bound}");
    largest.sort_unstable();
    for (spots, name, page) in largest.iter().rev() {
        println!("  within two of the bound or past it: {name} page {page} names {spots}");
    }
    if separate {
        println!(
            "  separated under the simulation: {separated}; given up: {}",
            given_up.len()
        );
        for (reason, pages) in &reasons {
            println!("  {pages} page(s): {reason}");
        }
        for (name, page, reason) in &given_up {
            println!("  given up: page {page} of {name}: {reason}");
        }
    }
}

/// The sentence the page's report gives for the separation it gave up, or a defect where it gives
/// none, because the report is what makes the fallback loud (ADR 1329).
fn given_up_reason(interpretation: &pdf_model::Interpretation) -> String {
    interpretation
        .unsupported
        .iter()
        .find_map(|report| match report {
            pdf_model::Unsupported::SeparationGivenUp { reason } => Some(reason.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "DEFECT: given up with no report".to_owned())
}

/// A reason with the plane's number taken out, so that the split counts kinds rather than planes.
fn kind_of(reason: &str) -> String {
    reason
        .split(' ')
        .map(|word| {
            if word.chars().all(|c| c.is_ascii_digit()) {
                "N"
            } else {
                word
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
