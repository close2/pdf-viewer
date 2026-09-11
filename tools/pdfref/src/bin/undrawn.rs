//! Sweeps the oracle's ambiguous bucket for a page where we draw less than every reference.
//!
//! ```sh
//! cargo test --profile gates -p pdf-model --test oracle -- --ignored --nocapture > oracle.log
//! cargo run --release -p pdfref --bin undrawn -- oracle.log
//! ```
//!
//! `doc/todo/00` step 7, which had been a paragraph since the two-hundred-and-sixty-fifth session
//! and was rebuilt by hand at least fifteen times. [`pdfref::undrawn`] says what the sweep
//! asks, what its three corrections are, why the greyscale recipe is part of the measurement, and
//! what the rebuild in the nine-hundred-and-seventy-fourth session found: a page from a labelled
//! corpus is *printed* with its label in the name and *written* with the label as a directory, so
//! a loop written from the recipe skipped 63 of 838 pages and said nothing.
//!
//! It renders nothing. Every panel it reads is already on disk beside the report, which is why
//! the whole bucket costs a couple of minutes — the work is `magick`'s, one invocation per panel.
//!
//! It exits non-zero **only** where it could not measure a page the report listed, or could not
//! run at all. A row past the alarm is a reading list and not a build failure: the groups that
//! hold a page live in the gate, and whether a gap is a defect or a reference's excess is a
//! person's reading of a clause.

#![expect(
    clippy::print_stdout,
    reason = "the report is the whole output of the program"
)]

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use pdfref::undrawn::{self, INK_ARGUMENTS, Measured, Measurement, Page, REFERENCES};

/// How many rows past the alarm are printed either side of it.
const SHOWN: usize = 12;

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("undrawn: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Why the sweep could not be run at all.
#[derive(Debug, thiserror::Error)]
enum Error {
    /// The gate's report could not be read.
    #[error("the gate's report: {0}")]
    Report(#[from] std::io::Error),
    /// The report held no `ambiguous` line, so there is no population.
    #[error(
        "no ambiguous page in the report — it is the output of `cargo test --profile gates -p \
         pdf-model --test oracle -- --ignored --nocapture` that is wanted, with `--nocapture`"
    )]
    Empty,
    /// The report did not say where the run wrote its artefacts.
    #[error(
        "the report does not end with an `artefacts under …` line, so the panels cannot be \
         found; pass the artefact root as a second argument"
    )]
    Rootless,
    /// `magick` is not on this machine, so nothing can be measured.
    #[error(
        "`magick` could not be run ({0}) — the ink is ImageMagick's, and a sweep that quietly \
         measured nothing would read exactly like a clean one"
    )]
    Magick(std::io::Error),
}

fn run() -> Result<bool, Error> {
    let mut arguments = std::env::args().skip(1);
    let report = read_report(arguments.next())?;
    let pages = undrawn::pages(&report);
    if pages.is_empty() {
        return Err(Error::Empty);
    }
    let root = match arguments.next() {
        Some(given) => PathBuf::from(given),
        None => PathBuf::from(undrawn::artefact_root(&report).ok_or(Error::Rootless)?),
    };

    let mut measurement = Measurement::default();
    for page in pages {
        match measure(&root, &page) {
            Ok(Some(measured)) => measurement.measured.push(measured),
            Ok(None) => measurement
                .missing
                .push((page, "every reference drew a blank sheet".to_owned())),
            Err(reason) => measurement.missing.push((page, reason)),
        }
    }
    measurement.rank();
    Ok(print(&measurement, &root))
}

/// The report, from the named file or from standard input.
fn read_report(named: Option<String>) -> Result<String, std::io::Error> {
    if let Some(path) = named {
        return std::fs::read_to_string(path);
    }
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text)?;
    Ok(text)
}

/// One page's row, or why it has none.
fn measure(root: &Path, page: &Page) -> Result<Option<Measured>, String> {
    let ours = undrawn::under(root, &page.ours());
    if !ours.exists() {
        return Err(format!("no raster of ours at {}", ours.display()));
    }
    let our_ink = ink(&ours)?;

    let mut references = Vec::new();
    for renderer in REFERENCES {
        if !page.drew(renderer) {
            continue;
        }
        let panel = undrawn::under(root, &page.reference(renderer));
        match std::fs::metadata(&panel) {
            // Correction 4: a refusal can leave an empty file behind, and the gate's own line is
            // the first thing consulted. An empty one it did *not* name is still not a render.
            Ok(metadata) if metadata.len() == 0 => {}
            Ok(_) => references.push(((*renderer).to_owned(), ink(&panel)?)),
            Err(_) => {}
        }
    }
    if references.is_empty() {
        return Err("no reference panel on disk".to_owned());
    }
    Ok(
        undrawn::lightest(&references).map(|(renderer, reference)| Measured {
            page: page.clone(),
            ours: our_ink,
            renderer: renderer.clone(),
            reference: *reference,
        }),
    )
}

/// A panel's ink in levels of 255, by the recipe [`INK_ARGUMENTS`] states.
fn ink(panel: &Path) -> Result<f64, String> {
    let output = Command::new("magick")
        .arg(panel)
        .args(INK_ARGUMENTS)
        .output()
        .map_err(|error| Error::Magick(error).to_string())?;
    if !output.status.success() {
        return Err(format!(
            "magick refused {}: {}",
            panel.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.trim()
        .parse()
        .map_err(|_| format!("magick answered {:?} for {}", text.trim(), panel.display()))
}

/// The report. Returns whether every listed page was measured.
fn print(measurement: &Measurement, root: &Path) -> bool {
    let (listed, measured) = measurement.denominator();
    println!("artefacts under {}", root.display());
    println!("{listed} ambiguous page(s) listed, {measured} measured");

    if !measurement.missing.is_empty() {
        println!(
            "\n{} page(s) the report listed and this sweep could not measure — a silent skip \
             here reads exactly like a clean run:",
            measurement.missing.len()
        );
        for (page, reason) in &measurement.missing {
            println!("    {} — {reason}", page.name);
        }
    }

    let alarming = measurement
        .measured
        .iter()
        .filter(|row| row.alarming())
        .count();
    let reported = measurement
        .measured
        .iter()
        .filter(|row| row.alarming() && row.page.reported)
        .count();
    println!(
        "\n{alarming} page(s) at or past the alarm of {:.2}, {reported} of them documents this \
         tree reports on:",
        undrawn::ALARM
    );
    for row in measurement
        .measured
        .iter()
        .take(alarming.saturating_add(SHOWN))
    {
        println!("    {}", line(row));
    }

    println!(
        "\nand the positive head — ink nobody else is drawing, which is most often a reference \
         that failed:"
    );
    for row in measurement.measured.iter().rev().take(SHOWN) {
        println!("    {}", line(row));
    }

    println!(
        "\nA gap is a difference between two programs, so a reference re-rendered by a newer \
         binary moves a row with nothing of ours having changed. Read the row with the \
         side-by-side beside it, never alone."
    );
    measurement.missing.is_empty()
}

/// One row, as `doc/todo/00` step 7's runs have always written them.
fn line(row: &Measured) -> String {
    format!(
        "{:9.3}  {}{}  ours {:.4}, lightest {} {:.4}",
        row.gap(),
        row.page.name,
        if row.page.reported {
            " [incomplete]"
        } else {
            ""
        },
        row.ours,
        row.renderer,
        row.reference,
    )
}
