//! What one survey of a document costs, and what a full report costs on top of it.
//!
//! ```sh
//! cargo run --release -p pdf-archive --example survey_cost -- FILE.pdf
//! ```
//!
//! `CLAUDE.md` principle 2: an optimisation is justified by a benchmark, and so is the decision
//! not to make one. This is the benchmark the note in `crate::survey` cites.

use std::time::Instant;

use pdf_archive::survey::Survey;
use pdf_archive::{Flavour, Target, check};
use pdf_model::Pages;
use pdf_model::content::reader::ContentReader;

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: survey_cost FILE.pdf");
        return;
    };
    let Ok(bytes) = pdf_syntax::FileBytes::on_disk(std::path::Path::new(&path)) else {
        eprintln!("could not open {path}");
        return;
    };
    let Ok(document) = pdf_syntax::Document::open(bytes) else {
        eprintln!("could not parse {path}");
        return;
    };

    let start = Instant::now();
    let pages = Pages::new(&document);
    let mut tokens = 0_u64;
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let mut reader = ContentReader::for_page(&document, &page);
        while reader.with_token(|token| token.is_some()) {
            tokens = tokens.saturating_add(1);
        }
    }
    println!(
        "reading {tokens} tokens of page content alone: {:?}",
        start.elapsed()
    );

    let start = Instant::now();
    let survey = Survey::of(&document);
    println!(
        "one survey: {:?} over {} pages — {} colours, {} groups, {} missing resources",
        start.elapsed(),
        survey.pages(),
        survey.device_colours().len(),
        survey.group_spaces().len(),
        survey.missing_resources().len(),
    );

    let start = Instant::now();
    let report = check(&document, Target::Four(Flavour::Plain));
    println!(
        "a whole PDF/A-4 report: {:?} over {} requirements",
        start.elapsed(),
        report.judgements.len(),
    );
}
