//! Which requirements a report spends its time in.
//!
//! `CLAUDE.md` principle 2 asks that an optimisation be justified by a measurement rather than
//! by a belief about where the time goes — twice in this crate's short history that belief has
//! been wrong, so this prints the answer instead.
//!
//! ```sh
//! cargo run --release -p pdf-archive --example cost -- <file.pdf> 4
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose whole product is a measurement"
)]

use std::time::Instant;

use pdf_archive::{Check, Examination, Findings, Target, table};
use pdf_syntax::{Document, FileBytes};

fn main() {
    let mut arguments = std::env::args().skip(1);
    let Some(path) = arguments.next() else {
        println!("usage: cost <file.pdf> <2b|2u|2a|4|4f|4e>");
        return;
    };
    let target = arguments
        .next()
        .and_then(|name| Target::parse(&name))
        .unwrap_or(Target::Four(pdf_archive::Flavour::Plain));
    let Ok(bytes) = FileBytes::on_disk(std::path::Path::new(&path)) else {
        println!("{path}: cannot be read");
        return;
    };
    let opening = Instant::now();
    let Ok(document) = Document::open(bytes) else {
        println!("{path}: does not open");
        return;
    };
    println!("opening the document: {:?}", opening.elapsed());

    let examination = Examination::new(&document, target);
    let population = Instant::now();
    let objects = examination.objects().len();
    println!(
        "the object population ({objects}): {:?}",
        population.elapsed()
    );
    let survey = Instant::now();
    let _ = examination.survey();
    println!("the content survey: {:?}", survey.elapsed());

    let mut costs: Vec<(std::time::Duration, &str)> = Vec::new();
    for requirement in table::binding(target) {
        let Check::Implemented(predicate) = requirement.check else {
            continue;
        };
        let at = Instant::now();
        let mut findings = Findings::default();
        predicate(&examination, &mut findings);
        costs.push((at.elapsed(), requirement.id));
    }
    costs.sort_by_key(|(cost, _)| std::cmp::Reverse(*cost));
    let total: std::time::Duration = costs.iter().map(|(cost, _)| *cost).sum();
    println!("\nthe predicates, {total:?} in all — every one:");
    for (cost, id) in costs.iter() {
        println!("  {cost:>10.2?}  {id}");
    }
}
