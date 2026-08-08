//! How many documents state §12.4.4's presentation, and which of Table 164's styles they ask for.
//!
//! The three-hundred-and-ninety-third session's round drew the frames of a transition, and the
//! first question a round that draws something owes is which files it is drawing *for*. ADR 0135
//! recorded "not one page of the corpus's 964 openable documents states a `/Trans` or a `/Dur`",
//! measured in the seventieth session over raw bytes; this asks the page tree instead, so that a
//! `/Trans` inside an object stream — which a byte grep cannot see — is counted where one exists.
//!
//! Three populations, because they are three different claims: a page stating `/Trans` (something
//! to draw), a page stating `/Dur` (something to drive the clock), and a page stating `/PresSteps`
//! (§12.4.4.2's states, which this round does not touch).
//!
//! ```sh
//! cargo run --release -p pdf-model --example presentation_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use pdf_model::navigation::{Style, display_duration, transition};
use pdf_syntax::Document;

/// How many pages of one document are walked.
///
/// A presentation is a slide show and a slide show's first pages are where its transitions are;
/// ISO 32000-2 itself is 1023 pages, and walking every one of a thousand-page document to count
/// an entry no page of it states costs the census minutes for nothing.
const MAX_PAGES: usize = 100;

fn main() {
    let mut documents = 0_usize;
    let mut with_trans = 0_usize;
    let mut with_dur = 0_usize;
    let mut with_steps = 0_usize;
    let mut pages_seen = 0_usize;
    let mut styles: BTreeMap<String, usize> = BTreeMap::new();
    let mut named: Vec<String> = Vec::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
        let pages = pdf_model::Pages::new(&document);
        let (mut trans, mut dur, mut steps) = (false, false, false);
        for index in 0..pages.len().min(MAX_PAGES) {
            let Some(page) = pages.get(index) else {
                continue;
            };
            pages_seen = pages_seen.saturating_add(1);
            if let Some(found) = transition(&document, &page.dict) {
                trans = true;
                let counter = styles.entry(spelling(&found.style)).or_default();
                *counter = counter.saturating_add(1);
            }
            dur |= display_duration(&document, &page.dict).is_some();
            steps |= !pdf_model::navigation::steps(&document, &page.dict).is_empty();
        }
        if trans {
            with_trans = with_trans.saturating_add(1);
            named.push(name);
        }
        if dur {
            with_dur = with_dur.saturating_add(1);
        }
        if steps {
            with_steps = with_steps.saturating_add(1);
        }
    }

    println!("{documents} document(s) opened, {pages_seen} page(s) walked");
    println!("  {with_trans} state a /Trans on a page");
    println!("  {with_dur} state a /Dur on a page");
    println!("  {with_steps} state a /PresSteps on a page");
    println!("  styles: {styles:?}");
    if named.len() <= 40 {
        println!("  stating a /Trans: {}", named.join(" "));
    }
}

/// Table 164's own name for a style, so that the tally reads as the table does.
fn spelling(style: &Style) -> String {
    match style {
        Style::Split => "Split".to_owned(),
        Style::Blinds => "Blinds".to_owned(),
        Style::Box => "Box".to_owned(),
        Style::Wipe => "Wipe".to_owned(),
        Style::Dissolve => "Dissolve".to_owned(),
        Style::Glitter => "Glitter".to_owned(),
        Style::Replace => "R".to_owned(),
        Style::Fly => "Fly".to_owned(),
        Style::Push => "Push".to_owned(),
        Style::Cover => "Cover".to_owned(),
        Style::Uncover => "Uncover".to_owned(),
        Style::Fade => "Fade".to_owned(),
        Style::Unrecognised(name) => {
            format!(
                "(not in Table 164) {}",
                String::from_utf8_lossy(name.as_bytes())
            )
        }
    }
}
