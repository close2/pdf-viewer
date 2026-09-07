//! How many of the table's requirements each target binds, and where the flavours differ.
//!
//! The levels and flavours are not a presentation detail: a requirement's applicability column
//! decides which of the six targets it reaches, so `PDF/A-4` and `PDF/A-4f` are held to
//! genuinely different sets of rules. This example prints both halves of that — the size of each
//! target's set, and every requirement that binds some PDF/A-4 flavours and not others.
//!
//! ```sh
//! cargo run -p pdf-archive --example targets
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose whole product is a table"
)]

use pdf_archive::{Check, Flavour, Target, table};

fn main() {
    println!(
        "{:<10} {:>6} {:>8} {:>10}",
        "target", "binds", "checked", "unchecked"
    );
    for target in Target::ALL {
        let bound: Vec<_> = table::binding(target).collect();
        let checked = bound
            .iter()
            .filter(|requirement| matches!(requirement.check, Check::Implemented(_)))
            .count();
        println!(
            "{:<10} {:>6} {:>8} {:>10}",
            target.to_string(),
            bound.len(),
            checked,
            bound.len().saturating_sub(checked)
        );
    }
    println!("\nrequirements that bind some PDF/A-4 flavours and not others:");
    for requirement in table::requirements() {
        let bound = [Flavour::Plain, Flavour::F, Flavour::E].map(|flavour| {
            table::binding(Target::Four(flavour)).any(|bound| bound.id == requirement.id)
        });
        if bound.iter().any(|one| *one != bound[0]) {
            let mark = |at: usize| if bound[at] { "yes" } else { " — " };
            println!(
                "  4:{} 4f:{} 4e:{}  {}",
                mark(0),
                mark(1),
                mark(2),
                requirement.id
            );
        }
    }
}
