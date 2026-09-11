//! What this converter has decided about every requirement each PDF/A target binds.
//!
//! ```sh
//! cargo run --release -p pdf-transform --example archive_census
//! ```
//!
//! # What it answers
//!
//! `CLAUDE.md`'s coverage question, whose denominator is the specification rather than a corpus.
//! `tests/archive_corpus.rs` sweeps the veraPDF corpus and says what share of the files that
//! exist convert; that is robustness, and no number of corpus documents can rank a requirement
//! nothing in the corpus exercises. This prints the other denominator: every requirement
//! `pdf_archive` holds a target to, and which of the converter's seven standings it has.
//!
//! Three of the seven are reasons the converter is never asked — a requirement whose subject is
//! a program, one a published clarification puts outside validation, one the validator does not
//! yet check. Three are answers: a `REMEDIES` row, a rule the serializer satisfies by
//! construction, or a refusal with an argument of its own. The seventh is the product, and it is
//! printed in full at the end: the requirements in none of the tables, which a document failing
//! one of them is refused over with a sentence saying only that the gap is this program's.
//!
//! `tests/archive.rs` holds that last list to `tests/archive_unconsidered.txt` in both
//! directions, so this example is the reading and the test is the ratchet.

#![expect(
    clippy::print_stdout,
    reason = "an example whose whole product is a table a person reads"
)]

use std::collections::BTreeMap;

use pdf_archive::{Part, Target, table};
use pdf_transform::archive::{Standing, census, unconsidered};

fn main() {
    println!(
        "{:<7} {:>6} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>13}",
        "target",
        "binds",
        "remedy",
        "writer",
        "refused",
        "proc",
        "clarif",
        "unchkd",
        "unconsidered"
    );
    for target in Target::ALL {
        let rows: Vec<Standing> = census(target).map(|(_, standing)| standing).collect();
        let count = |wanted: fn(Standing) -> bool| rows.iter().filter(|it| wanted(**it)).count();
        println!(
            "{:<7} {:>6} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>13}",
            target.to_string(),
            rows.len(),
            count(|it| matches!(it, Standing::Remedy(_))),
            count(|it| matches!(it, Standing::WriterEmits)),
            count(|it| matches!(it, Standing::Refused(_))),
            count(|it| matches!(it, Standing::Processor)),
            count(|it| matches!(it, Standing::OutsideValidation)),
            count(|it| matches!(it, Standing::Unchecked)),
            count(|it| matches!(it, Standing::Unconsidered)),
        );
    }

    // The refusals, by the kind of *no* each is. `Because`'s four kinds are four different facts
    // about the world — the fence, no conforming file for this target, a caller's own choice,
    // and a gap named as one — and a reader ranking the work needs them apart.
    println!("\nrefusals by kind, over every requirement any target binds:");
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for target in Target::ALL {
        for (_, standing) in census(target) {
            if let Standing::Refused(because) = standing {
                let seen = kinds.entry(because.word()).or_default();
                *seen = seen.saturating_add(1);
            }
        }
    }
    for (word, count) in kinds {
        println!("  {word:<16} {count:>4} target-requirement pairs");
    }

    // `binds` and `table::binding` are not the same question — the second also drops what the
    // committee has withdrawn from part 4 — so the targets printed here are the binding
    // function's, which is the one every verdict uses.
    let bound = |requirement: &pdf_archive::Requirement, target: Target| {
        table::binding(target).any(|it| it.id == requirement.id)
    };
    let unconsidered = unconsidered();
    println!(
        "\nno considered answer at all — {} requirements, each refused with the catch-all:",
        unconsidered.len()
    );
    for requirement in unconsidered {
        let clause = |part: Part| {
            Target::ALL
                .iter()
                .find(|target| target.part() == part && bound(requirement, **target))
                .and_then(|target| requirement.clauses.citation(*target))
                .unwrap_or_else(|| "—".to_owned())
        };
        let targets: Vec<String> = Target::ALL
            .iter()
            .filter(|target| bound(requirement, **target))
            .map(ToString::to_string)
            .collect();
        println!("  {}", requirement.id);
        println!(
            "      {} | {} | binds {}",
            clause(Part::Two),
            clause(Part::Four),
            targets.join(" ")
        );
        println!("      {}", requirement.asks);
    }
}
