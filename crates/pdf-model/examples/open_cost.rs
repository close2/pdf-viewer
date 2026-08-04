//! Where the time between `open` and page one goes, step by step.
//!
//! `CLAUDE.md`'s startup rules make two claims this measures: **nothing eager** — "no full
//! page-tree walk … anything not needed to show page one is deferred until first use" — and
//! **incremental parsing**, "a 500-page document must open no slower than a 5-page one". Both
//! are claims about *this* path, which is everything `viewer_core::Open::around` and
//! `viewer_core::notes::about` do before a window exists.
//!
//! ```sh
//! cargo run --release -p pdf-model --example open_cost -- doc/ISO_32000-2_sponsored_EC3.pdf
//! ```
//!
//! Each line is one step of that path, measured on its own. They are *not* additive with the
//! viewer's own launch timeline — this reads the file first and each step reuses what the
//! previous one warmed — which is the point: a step's cost here is what it would still cost if
//! everything else were free.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::time::Instant;

use pdf_syntax::Document;

fn ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1e3
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: open_cost <document.pdf>");
    let bytes = std::fs::read(&path).expect("readable");

    let started = Instant::now();
    let document = Document::open(bytes).expect("opens");
    println!(
        "{:8.3} ms  Document::open  (§7.5's trailer and xref)",
        ms(started)
    );

    let started = Instant::now();
    let pages = pdf_model::Pages::new(&document);
    println!(
        "{:8.3} ms  Pages::new      (§7.7.3's page tree, {} pages)",
        ms(started),
        pages.len()
    );

    let started = Instant::now();
    let labels = pdf_model::page_label::PageLabels::read(&document);
    println!(
        "{:8.3} ms  PageLabels      (§12.4.2, {})",
        ms(started),
        if labels.is_empty() {
            "absent"
        } else {
            "present"
        }
    );

    let started = Instant::now();
    let outline = pdf_model::outline::Outline::read(&document, &pages);
    println!(
        "{:8.3} ms  Outline::read   (§12.3.3, {} items)",
        ms(started),
        outline.items.len()
    );

    let started = Instant::now();
    let action = pdf_model::destination::Destination::open_action(&document);
    println!(
        "{:8.3} ms  OpenAction      (§12.3.2.1, {})",
        ms(started),
        if action.is_some() {
            "present"
        } else {
            "absent"
        }
    );

    let started = Instant::now();
    let view = pdf_model::view::ViewState::of(&document);
    drop(view);
    println!(
        "{:8.3} ms  ViewState::of   (§8.11's configuration)",
        ms(started)
    );

    let started = Instant::now();
    let unmet = pdf_model::requirements::unmet(&document).len();
    println!(
        "{:8.3} ms  requirements    (§12.11, {unmet} unmet)",
        ms(started)
    );

    let started = Instant::now();
    let files = pdf_model::attachment::attachments(&document).len();
    println!("{:8.3} ms  attachments     (§7.11.4, {files})", ms(started));

    let started = Instant::now();
    let signatures = pdf_model::signature::signatures(&document).len();
    println!(
        "{:8.3} ms  signatures      (§12.8, {signatures})",
        ms(started)
    );

    let started = Instant::now();
    let page = pages.get(0).expect("a first page");
    let interpreted = pdf_model::content::interpret(&document, &page);
    println!(
        "{:8.3} ms  page one        (§7.8.2's content, {} commands)",
        ms(started),
        interpreted.display_list.command_count()
    );
}
