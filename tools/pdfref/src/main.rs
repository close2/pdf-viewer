//! Surveys where the reference renderers disagree with each other.
//!
//! ```text
//! cargo run -p pdfref -- [--dpi N] [--work-dir DIR] <file.pdf>...
//! ```
//!
//! # Why this is useful before we can render a PDF ourselves
//!
//! Our own renderer needs a parser, which does not exist yet, so this cannot compare
//! *us* against anything. What it can do is map the ground where the references
//! themselves disagree — and that map is worth having in advance.
//!
//! Every file the references disagree on is a page where the specification is ambiguous,
//! or where at least one implementation is wrong. Those are exactly the pages that would
//! otherwise produce unexplained failures once we do start rendering, and knowing about
//! them beforehand is the difference between "the suite found something interesting" and
//! "the suite is noisy, ignore it".
//!
//! Run against the specification PDFs in `doc/`, this produces the initial
//! known-divergent list.

#![forbid(unsafe_code)]
// A command-line tool: stdout is the interface.
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "this binary's output is its purpose"
)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use pdf_render::Raster;
use pdfref::{Reference, Tolerance, normalise, reference};

/// What one invocation was asked to do.
#[derive(Debug)]
struct Options {
    dpi: u32,
    work_root: PathBuf,
    files: Vec<PathBuf>,
}

/// How one file turned out.
#[derive(Debug, PartialEq, Eq)]
enum FileOutcome {
    /// Every pair of references agreed within tolerance.
    Consistent,
    /// At least one pair disagreed. Information, not a failure.
    Divergent,
    /// A renderer could not produce an image at all.
    Failed,
}

fn main() -> ExitCode {
    let options = match parse_args() {
        Ok(Some(options)) => options,
        // `--help` was handled and printed.
        Ok(None) => return ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };

    let available = Reference::available();
    if available.len() < 2 {
        eprintln!(
            "only {} reference renderer(s) available; at least two are needed to compare",
            available.len()
        );
        for missing in Reference::ALL.iter().filter(|r| !r.is_available()) {
            eprintln!("  missing {missing}: install {}", missing.package_hint());
        }
        return ExitCode::FAILURE;
    }

    for reference in &available {
        println!(
            "{}: {}",
            reference.name(),
            reference.version().unwrap_or_default()
        );
    }
    println!("comparing at {} dpi\n", options.dpi);

    let mut consistent = 0usize;
    let mut divergent = 0usize;
    let mut failed = 0usize;

    for file in &options.files {
        match survey_file(file, &options, &available) {
            FileOutcome::Consistent => consistent = consistent.saturating_add(1),
            FileOutcome::Divergent => divergent = divergent.saturating_add(1),
            FileOutcome::Failed => failed = failed.saturating_add(1),
        }
    }

    println!(
        "\n{} file(s): {consistent} consistent, {divergent} divergent, {failed} failed to render",
        options.files.len()
    );
    println!("intermediates under {}", options.work_root.display());

    // Divergence between references is information, not our failure — that is the whole
    // point of the triangulation rule. Only a renderer that could not run is an error.
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Parses the command line.
///
/// Returns `Ok(None)` when `--help` was handled and the program should exit successfully.
fn parse_args() -> Result<Option<Options>, String> {
    let mut dpi = 72u32;
    let mut work_root: Option<PathBuf> = None;
    let mut files: Vec<PathBuf> = Vec::new();
    let mut args = std::env::args_os().skip(1);

    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--dpi" => {
                let value = args.next().ok_or("--dpi needs a value")?;
                dpi = value
                    .to_string_lossy()
                    .parse::<u32>()
                    .ok()
                    .filter(|parsed| *parsed > 0)
                    .ok_or_else(|| {
                        format!(
                            "--dpi needs a positive integer, got {}",
                            value.to_string_lossy()
                        )
                    })?;
            }
            "--work-dir" => {
                work_root = Some(PathBuf::from(args.next().ok_or("--work-dir needs a path")?));
            }
            "--help" | "-h" => {
                print_help();
                return Ok(None);
            }
            other if other.starts_with("--") => {
                return Err(format!("unknown option {other}; try --help"));
            }
            _ => files.push(PathBuf::from(arg)),
        }
    }

    if files.is_empty() {
        return Err("no input files; try --help".to_owned());
    }

    Ok(Some(Options {
        dpi,
        work_root: work_root.unwrap_or_else(reference::default_work_dir),
        files,
    }))
}

fn print_help() {
    println!("usage: pdfref [--dpi N] [--work-dir DIR] <file.pdf>...");
    println!();
    println!("Renders each file with every available reference renderer and reports");
    println!("where they disagree with one another.");
    println!();
    println!("Intermediate renders are kept rather than cleaned up: on a disagreement");
    println!("they are the evidence. Defaults to ./target/pdfref.");
}

/// Renders one file with every available reference and reports their agreement.
fn survey_file(file: &Path, options: &Options, available: &[Reference]) -> FileOutcome {
    let stem = file
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let work_dir = options.work_root.join(&stem);

    let mut rasters = Vec::new();
    for reference in available {
        match reference.render(file, options.dpi, &work_dir) {
            Ok(raster) => rasters.push((*reference, raster)),
            Err(e) => {
                println!("{stem}: ERROR — {e}");
                return FileOutcome::Failed;
            }
        }
    }

    // Reconcile the one-pixel rounding of fractional page sizes before comparing. A
    // genuine geometry disagreement still surfaces as an error here.
    let normalisation = {
        let mut views: Vec<&mut Raster> = rasters.iter_mut().map(|(_, r)| r).collect();
        match normalise::to_common_size(&mut views) {
            Ok(normalisation) => normalisation,
            Err(e) => {
                println!("{stem}: GEOMETRY — {e}");
                return FileOutcome::Divergent;
            }
        }
    };

    let (disagreements, worst) = compare_pairs(&stem, &rasters, &Tolerance::DEFAULT);

    let summary = worst.map_or_else(
        || "no comparable pairs".to_owned(),
        |(left, right, value)| format!("worst pair {left} vs {right} at {value:.4}"),
    );
    let note = if normalisation.cropped() {
        format!("; {normalisation}")
    } else {
        String::new()
    };

    if disagreements > 0 {
        println!("{stem}: DIVERGENT — {disagreements} pair(s) outside tolerance; {summary}{note}");
        FileOutcome::Divergent
    } else {
        println!("{stem}: consistent — {summary}{note}");
        FileOutcome::Consistent
    }
}

/// Compares every pair of references, returning how many disagreed and the worst pair.
fn compare_pairs(
    stem: &str,
    rasters: &[(Reference, Raster)],
    tolerance: &Tolerance,
) -> (usize, Option<(Reference, Reference, f64)>) {
    let mut disagreements = 0usize;
    let mut worst: Option<(Reference, Reference, f64)> = None;

    for (index, (left_ref, left)) in rasters.iter().enumerate() {
        for (right_ref, right) in rasters.iter().skip(index.saturating_add(1)) {
            match raster_compare::compare(left, right) {
                Ok(comparison) => {
                    if !tolerance.accepts(&comparison) {
                        disagreements = disagreements.saturating_add(1);
                    }
                    if worst.is_none_or(|(_, _, value)| comparison.worst_tile_error > value) {
                        worst = Some((*left_ref, *right_ref, comparison.worst_tile_error));
                    }
                    // Printed per pair rather than summarised, because this survey's
                    // purpose is to establish where the references' own noise floor sits
                    // and a single worst number cannot show a distribution.
                    println!(
                        "  {stem}: {left_ref} vs {right_ref}  mean {:.4}  worst tile {:.4}  \
                         ssim {:.4}  worst ssim {:.4} at {:?}  worst tile at {:?}",
                        comparison.mean_error,
                        comparison.worst_tile_error,
                        comparison.structural_similarity,
                        comparison.worst_tile_similarity,
                        comparison.worst_tile_similarity_at,
                        comparison.worst_tile_at
                    );
                }
                Err(e) => {
                    // A size mismatch here means the renderers read the page geometry
                    // differently, which is more interesting than any pixel difference.
                    println!("{stem}: GEOMETRY — {left_ref} vs {right_ref}: {e}");
                    disagreements = disagreements.saturating_add(1);
                }
            }
        }
    }

    (disagreements, worst)
}
