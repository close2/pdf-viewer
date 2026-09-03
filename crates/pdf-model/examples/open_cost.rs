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

/// Every outline item at every level, not `Outline::items.len()`.
///
/// The top level of a book's table of contents is its chapters, and the cost is per *item*:
/// reporting the first as the second is how this example's own first run said "6.7 ms for 38
/// items" about 988 of them.
fn outline_items(level: &[pdf_model::outline::Item]) -> usize {
    level.iter().fold(level.len(), |total, item| {
        total.saturating_add(outline_items(&item.children))
    })
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: open_cost <document.pdf>");
    // On disk, as every host opens a file since ADR 0809; `OPEN_COST_ROUTE=whole` reads it whole
    // first, which is the route the confined viewer's worker still receives it by.
    let whole = std::env::var("OPEN_COST_ROUTE").is_ok_and(|route| route == "whole");
    let started = Instant::now();
    let bytes = if whole {
        pdf_syntax::FileBytes::from(
            pdf_syntax::read_file(std::path::Path::new(&path)).expect("readable"),
        )
    } else {
        pdf_syntax::FileBytes::on_disk(std::path::Path::new(&path)).expect("opens")
    };
    println!(
        "{:8.3} ms  {}  ({} bytes)",
        ms(started),
        if whole {
            "read_file       "
        } else {
            "FileBytes::on_disk"
        },
        bytes.len()
    );

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
        "{:8.3} ms  Outline::read   (§12.3.3, {} items over {} at the top)",
        ms(started),
        outline_items(&outline.items),
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
    println!(
        "{:>8} kB  peak resident   (VmHWM, the whole process)",
        peak_resident_kb()
    );
}

/// The process's resident high-water mark in kilobytes, off `/proc/self/status`.
///
/// A high-water mark rather than a sample, because the kernel keeps it across frees and it does
/// not move with the machine's load — `confined_peak` quotes `VmPeak` for the same reason.
fn peak_resident_kb() -> String {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find_map(|line| line.strip_prefix("VmHWM:"))
                .map(|rest| rest.trim().trim_end_matches(" kB").to_owned())
        })
        .unwrap_or_else(|| "?".to_owned())
}
