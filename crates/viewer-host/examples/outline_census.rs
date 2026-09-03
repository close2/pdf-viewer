//! How many rows §12.3.3's outline becomes, and how many the document asks to be open.
//!
//! Not a gate: a count a round quoted, kept runnable so the next round can check it rather than
//! believe it. `cargo run -p viewer-host --example outline_census -- <file.pdf>`.

use viewer_core::{Answer, Command, DocumentId, Query, Viewer};
use viewer_host::{PanelRow, outline_rows};

/// Every row, depth first.
fn flatten<'a>(rows: &'a [PanelRow], into: &mut Vec<&'a PanelRow>) {
    for row in rows {
        into.push(row);
        flatten(&row.children, into);
    }
}

/// How many rows a tree shows with the document's own `/Count` signs obeyed.
fn visible(rows: &[PanelRow]) -> usize {
    rows.iter().fold(0, |total, row| {
        let under = if row.expanded {
            visible(&row.children)
        } else {
            0
        };
        total.saturating_add(1).saturating_add(under)
    })
}

fn main() -> std::process::ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: outline_census <file.pdf>");
        return std::process::ExitCode::FAILURE;
    };
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("cannot read {path}");
        return std::process::ExitCode::FAILURE;
    };
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    // Five, and the median reported, for the reason ADR 0222 gives about every wall clock in this
    // tree: one run of anything on this machine moves by more than the quantity being measured.
    let mut asking: Vec<u128> = Vec::with_capacity(5);
    for _ in 0..5u8 {
        let began = std::time::Instant::now();
        let answer = viewer.query(Query::Outline);
        let taken = began.elapsed().as_nanos();
        drop(answer);
        asking.push(taken);
    }
    asking.sort_unstable();
    let Answer::Outline(outline) = viewer.query(Query::Outline) else {
        println!("{path}: no §12.3.3 outline");
        return std::process::ExitCode::SUCCESS;
    };
    let rows = outline_rows(&outline);
    let mut flat = Vec::new();
    flatten(&rows, &mut flat);
    println!(
        "{path}: {} top-level, {} rows in all, {} shown with the document's own /Count signs",
        rows.len(),
        flat.len(),
        visible(&rows)
    );
    // What ADR 0247's second amendment costs, on this document, in this build: `Query::Outline`
    // clones the tree rather than lending it, so the whole of the answer is here.
    println!(
        "{path}: Query::Outline answers in {} ns, median of five ({asking:?})",
        asking[2]
    );
    std::process::ExitCode::SUCCESS
}
