//! Failure artefacts and human-readable summaries.
//!
//! A comparison suite is only as useful as its failure output. "Worst tile error 43.2 at
//! (320, 512)" tells you something is wrong; a side-by-side image and a difference
//! heatmap tell you *what*. If diagnosing a failure means re-running things by hand, the
//! suite becomes a nuisance and gets ignored, so the artefacts are written automatically
//! and kept rather than cleaned up.

use std::path::{Path, PathBuf};

use pdf_render::{Raster, RasterFormat};

use crate::{HarnessError, Outcome, Reference, Triangulation, png_io};

/// Difference at or below which a pixel is drawn as unchanged.
///
/// Matches `raster_compare`'s notion of a just-noticeable difference, so the heatmap
/// shows what the metrics counted rather than a differently-tuned view of it.
const NOISE_FLOOR: u8 = 4;

/// Width in pixels of the divider drawn between side-by-side panels.
const DIVIDER: u32 = 2;

/// Builds a heatmap of the per-pixel difference between two rasters.
///
/// The ramp is white for identical, through yellow, to red at maximum difference. It is
/// deliberately not a perceptually uniform colour map: the question this image answers
/// is "where is the difference concentrated", and a ramp that stays pale until something
/// genuinely differs answers that faster than one that colours the whole page.
///
/// # Errors
///
/// [`HarnessError::Compare`] if the rasters differ in size.
pub fn heatmap(left: &Raster, right: &Raster) -> Result<Raster, HarnessError> {
    if left.width != right.width || left.height != right.height {
        return Err(HarnessError::Compare {
            detail: format!(
                "cannot build a heatmap for {}x{} against {}x{}",
                left.width, left.height, right.width, right.height
            ),
        });
    }

    let mut data = Vec::with_capacity(left.data.len());
    for (a, b) in left.data.chunks_exact(4).zip(right.data.chunks_exact(4)) {
        let difference = a
            .iter()
            .zip(b)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0);

        if difference <= NOISE_FLOOR {
            data.extend_from_slice(&[255, 255, 255, 255]);
        } else {
            // Ramp white -> yellow -> red as the difference grows: full red, green
            // falling away with magnitude, no blue.
            let green = u8::MAX.saturating_sub(difference);
            data.extend_from_slice(&[255, green, 0, 255]);
        }
    }

    Ok(Raster {
        width: left.width,
        height: left.height,
        format: RasterFormat::Rgba8,
        data,
    })
}

/// Concatenates rasters horizontally, separated by a thin divider.
///
/// # Errors
///
/// [`HarnessError::Compare`] if the inputs differ in height, or if none were given.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "offsets are bounded by the row length allocated from these same panel \
              widths, and the height equality was checked above"
)]
pub fn side_by_side(panels: &[&Raster]) -> Result<Raster, HarnessError> {
    let first = panels.first().ok_or_else(|| HarnessError::Compare {
        detail: "side-by-side needs at least one panel".to_owned(),
    })?;
    let height = first.height;
    if panels.iter().any(|p| p.height != height) {
        return Err(HarnessError::Compare {
            detail: "side-by-side panels must share a height".to_owned(),
        });
    }

    let dividers = DIVIDER * u32::try_from(panels.len().saturating_sub(1)).unwrap_or(0);
    let total_width = panels.iter().map(|p| p.width).sum::<u32>() + dividers;

    let row_len = total_width as usize * 4;
    let mut data = vec![0u8; row_len * height as usize];

    for y in 0..height as usize {
        let mut x_offset = 0usize;
        for (index, panel) in panels.iter().enumerate() {
            let panel_row = y * panel.width as usize * 4;
            let panel_len = panel.width as usize * 4;
            let target = y * row_len + x_offset * 4;
            data[target..target + panel_len]
                .copy_from_slice(&panel.data[panel_row..panel_row + panel_len]);
            x_offset += panel.width as usize;

            if index + 1 < panels.len() {
                // Mid-grey divider, so it reads as a separator against both white pages
                // and dark content.
                for column in 0..DIVIDER as usize {
                    let at = y * row_len + (x_offset + column) * 4;
                    data[at..at + 4].copy_from_slice(&[128, 128, 128, 255]);
                }
                x_offset += DIVIDER as usize;
            }
        }
    }

    Ok(Raster {
        width: total_width,
        height,
        format: RasterFormat::Rgba8,
        data,
    })
}

/// Writes artefacts for one comparison and returns the paths written.
///
/// Always writes our own render and a side-by-side; writes a heatmap per reference only
/// when the outcome is a failure, since a heatmap of a passing comparison is a blank
/// page and thousands of blank pages obscure the ones that matter.
///
/// # Errors
///
/// [`HarnessError::Png`] if any artefact cannot be written, and
/// [`HarnessError::Compare`] if a heatmap cannot be built.
pub fn write_artefacts(
    dir: &Path,
    case: &str,
    ours: &Raster,
    references: &[(Reference, Raster)],
    triangulation: &Triangulation,
) -> Result<Vec<PathBuf>, HarnessError> {
    let mut written = Vec::new();

    let ours_path = dir.join(format!("{case}-ours.png"));
    png_io::write(&ours_path, ours)?;
    written.push(ours_path);

    let mut panels: Vec<&Raster> = vec![ours];
    for (_, raster) in references {
        panels.push(raster);
    }
    if let Ok(combined) = side_by_side(&panels) {
        let path = dir.join(format!("{case}-side-by-side.png"));
        png_io::write(&path, &combined)?;
        written.push(path);
    }

    if triangulation.outcome.is_failure() {
        for (reference, raster) in references {
            let map = heatmap(ours, raster)?;
            let path = dir.join(format!("{case}-diff-{reference}.png"));
            png_io::write(&path, &map)?;
            written.push(path);
        }
    }

    Ok(written)
}

/// Formats a triangulation as a human-readable report.
#[must_use]
pub fn summarise(case: &str, triangulation: &Triangulation) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();

    let verdict = match &triangulation.outcome {
        Outcome::Agrees { with } => {
            let names: Vec<&str> = with.iter().map(|r| r.name()).collect();
            format!("PASS — agrees with {}", names.join(", "))
        }
        Outcome::Regression { agreeing } => {
            let names: Vec<&str> = agreeing.iter().map(|r| r.name()).collect();
            format!(
                "FAIL — {} agree with each other, we differ",
                names.join(" and ")
            )
        }
        Outcome::Ambiguous => {
            "DIVERGENT — the references disagree among themselves; not a failure".to_owned()
        }
        Outcome::NotEnoughReferences { available } => {
            format!("FAIL — only {available} reference(s) available; cannot triangulate")
        }
    };
    let _ = writeln!(out, "{case}: {verdict}");

    // The bounds are part of the verdict, not context for it: under a judgement relative
    // to the references they are derived from this page and differ from every other page's.
    let bounds = &triangulation.judged_by;
    let _ = writeln!(
        out,
        "  judged against: mean {:.4}, worst tile {:.4}, differing {:.4}%, ssim {:.4}",
        bounds.max_mean,
        bounds.max_worst_tile,
        bounds.max_differing_fraction * 100.0,
        bounds.min_structural_similarity
    );

    let _ = writeln!(out, "  ours vs references:");
    for (reference, comparison) in &triangulation.ours {
        let _ = writeln!(
            out,
            "    {:<12} mean {:>7.4}  worst tile {:>8.4} at {:?}  differing {:>7.4}%  \
             ssim {:>6.4}  worst ssim {:>6.4} at {:?}",
            reference.name(),
            comparison.mean_error,
            comparison.worst_tile_error,
            comparison.worst_tile_at,
            comparison.differing_fraction * 100.0,
            comparison.structural_similarity,
            comparison.worst_tile_similarity,
            comparison.worst_tile_similarity_at
        );
    }

    // Printed even on success: our own numbers are only interpretable next to how much
    // the references differ from each other.
    //
    // All **four** of `Tolerance::accepts`' measures, since the four-hundred-and-seventh
    // session. This line printed three and left out the differing fraction, which is the same
    // omission ADR 0242 found in `oracle.rs`'s per-page line and has the same consequence one
    // step earlier: these are the numbers a fixed bound is *derived* from, and the one bound
    // whose derivation nothing could check was the one nothing printed.
    let _ = writeln!(out, "  references vs each other:");
    for (left, right, comparison) in &triangulation.between_references {
        let _ = writeln!(
            out,
            "    {:<12} vs {:<12} mean {:>7.4}  worst tile {:>8.4}  differing {:>7.4}%  \
             ssim {:>6.4}  worst ssim {:>6.4}",
            left.name(),
            right.name(),
            comparison.mean_error,
            comparison.worst_tile_error,
            comparison.differing_fraction * 100.0,
            comparison.structural_similarity,
            comparison.worst_tile_similarity
        );
    }

    out
}
