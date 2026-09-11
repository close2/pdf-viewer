//! What the structural audit covers, at both of its granularities, and where it stops.
//!
//! Two questions, and the second is the one no other instrument in this crate can answer:
//!
//! - **Subclause-level**: of every subclause of ISO 19005-2 and ISO 19005-4, how many does a row
//!   of the table bind, and how many are recorded as stating no requirement, as scoping, as
//!   restating another, or as a container heading?
//! - **Sentence-level**: of the subclauses read sentence by sentence, how many normative
//!   sentences are there, and what carries each? And — the point of printing it — **which
//!   subclauses have no sentence-level reading at all**, which is the frontier.
//!
//! The frontier is computed from the data rather than written down, so a round that reads
//! further shortens this list by adding a [`pdf_archive::Reading`] and nothing else.
//!
//! ```sh
//! cargo run -p pdf-archive --example frontier
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose whole product is a report a person reads"
)]

use pdf_archive::{Binding, Carried, Part, frontier, readings, subclauses};

fn main() {
    for part in [Part::Two, Part::Four] {
        let listed: Vec<_> = subclauses().filter(|s| s.part == part).collect();
        let count = |wanted: fn(&Binding) -> bool| {
            listed
                .iter()
                .filter(|subclause| wanted(&subclause.binding))
                .count()
        };
        println!(
            "{part:?}: {} subclauses — {} bound, {} containers, {} state no requirement, \
             {} scoping, {} restated",
            listed.len(),
            count(|binding| matches!(binding, Binding::Bound)),
            count(|binding| matches!(binding, Binding::Container)),
            count(|binding| matches!(binding, Binding::StatesNoRequirement(_))),
            count(|binding| matches!(binding, Binding::Scoping(_))),
            count(|binding| matches!(binding, Binding::Restated(_))),
        );
    }

    println!("\nread sentence by sentence:");
    println!(
        "{:<8} {:<10} {:>10} {:>8} {:>10} {:>9} {:>10}",
        "part", "clause", "sentences", "by rows", "restated", "scoping", "no rule"
    );
    for reading in readings() {
        let tally = |wanted: fn(&Carried) -> bool| {
            reading
                .sentences
                .iter()
                .filter(|sentence| wanted(&sentence.carried))
                .count()
        };
        println!(
            "{:<8} {:<10} {:>10} {:>8} {:>10} {:>9} {:>10}",
            format!("{:?}", reading.part),
            reading.clause,
            reading.sentences.len(),
            tally(|carried| matches!(carried, Carried::By(_))),
            tally(|carried| matches!(carried, Carried::Restated(_))),
            tally(|carried| matches!(carried, Carried::Scoping(_))),
            tally(|carried| matches!(carried, Carried::StatesNoRequirement(_))),
        );
    }
    let sentences: usize = readings().map(|reading| reading.sentences.len()).sum();
    let total = |wanted: fn(&Carried) -> bool| {
        readings()
            .flat_map(|reading| reading.sentences)
            .filter(|sentence| wanted(&sentence.carried))
            .count()
    };
    println!(
        "\n{} subclauses read, {sentences} normative sentences — {} carried by a row, \
         {} restated elsewhere, {} scoping, {} stating no requirement",
        readings().count(),
        total(|carried| matches!(carried, Carried::By(_))),
        total(|carried| matches!(carried, Carried::Restated(_))),
        total(|carried| matches!(carried, Carried::Scoping(_))),
        total(|carried| matches!(carried, Carried::StatesNoRequirement(_))),
    );

    // The frontier, and it is deliberately printed in full rather than counted: a list a person
    // can pick the next round's work off is worth more than a number, and a number here would be
    // the kind of derived fact `CLAUDE.md` says belongs in a command's output rather than in a
    // document.
    println!("\nno sentence-level reading yet:");
    for subclause in frontier() {
        println!(
            "  {:<6} {:<10} {}",
            format!("{:?}", subclause.part),
            subclause.clause,
            subclause.subject
        );
    }
}
