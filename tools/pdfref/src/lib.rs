//! Compares our rendering against independent reference PDF renderers.
//!
//! # The problem this solves
//!
//! "Is our output correct?" has no local answer. The specification is prose, the
//! corpus is the whole web, and the only practical oracle is what other
//! implementations do. But no two renderers agree pixel-for-pixel — `poppler` and
//! `mupdf` differ on any page with a curve on it — so "differs from poppler" is not
//! evidence of a bug.
//!
//! # The triangulation rule
//!
//! Three independent renderers are used, and their *agreement* carries the evidence:
//!
//! - **Two or more agree with each other, and we differ from them** → a real bug. Two
//!   unrelated implementations arriving at the same answer is strong evidence that the
//!   answer is right. This fails the build.
//! - **The references disagree among themselves** → an ambiguous corner of the
//!   specification. Recorded as divergent, but *not* a failure: there is no correct
//!   answer to hold us to, and failing here would train everyone to ignore the suite.
//!
//! That distinction is the whole design. A comparison suite that cries wolf gets
//! switched off, and a switched-off suite catches nothing.
//!
//! # What "agree" means
//!
//! Tolerantly, via [`raster_compare`]. Exact equality is unachievable between correct
//! implementations, so [`Tolerance`] bounds mean error, worst-tile error and the
//! fraction of noticeably differing pixels. The worst-tile bound is the load-bearing
//! one among those: a missing glyph moves the mean less than antialiasing noise does.
//!
//! On text, none of the three is enough — the disagreement between two correct renderers
//! over glyph edges is larger than the disagreement a wrong glyph would cause. So
//! [`Tolerance`] also bounds structural similarity, which measures whether the same shapes
//! are in the same places rather than how far pixels moved, and which is what makes a text
//! page gateable on something better than "not blank".

#![forbid(unsafe_code)]

pub mod cache;
pub mod digest;
pub mod normalise;
pub mod png_io;
pub mod reference;
pub mod report;

use std::path::PathBuf;

use pdf_render::Raster;
use raster_compare::Comparison;

pub use cache::Cache;
pub use normalise::Normalisation;
pub use reference::Reference;

/// Bounds within which two renderings count as agreeing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tolerance {
    /// Maximum mean absolute channel difference, in `0.0..=255.0`.
    pub max_mean: f64,
    /// Maximum per-tile mean absolute difference.
    pub max_worst_tile: f64,
    /// Maximum fraction of channels differing noticeably, in `0.0..=1.0`.
    pub max_differing_fraction: f64,
    /// Minimum mean structural similarity, in `-1.0..=1.0`.
    ///
    /// Independent of the three bounds above, which all measure how far pixels moved.
    /// This one measures whether the same things are in the same places, and it is the
    /// bound that does the work on text.
    pub min_structural_similarity: f64,
}

impl Tolerance {
    /// Bounds for vector content, derived from measurement.
    ///
    /// On the `basic` fixture at 72 dpi the reference renderers differ from *each other*
    /// by a mean of 0.002 to 0.047 and a worst tile of 0.4 to 1.1. These bounds sit an
    /// order of magnitude above that floor, and far below what a genuine defect produces:
    /// a single missing shape pushes worst-tile error past 150.
    ///
    /// The structural bound is measured the same way. Across a 51-document sample of the
    /// pdf.js corpus, the pages where every reference pair already falls inside the three
    /// pixel bounds above have a structural similarity of 0.9971 at worst and 0.9999 at
    /// the median. 0.99 sits just under that floor.
    pub const VECTOR: Self = Self {
        max_mean: 1.0,
        max_worst_tile: 5.0,
        max_differing_fraction: 0.01,
        min_structural_similarity: 0.99,
    };

    /// Bounds for pages containing substantial text.
    ///
    /// Measured on the specification PDFs in `doc/`: the three references disagree with
    /// *each other* at a worst tile of 26 to 28, with 2.7% of pixels differing. A
    /// difference map shows the disagreement is confined to glyph outlines and
    /// single-pixel shape borders — the interiors of filled areas are identical — so it is
    /// hinting and antialiasing, not a rendering error.
    ///
    /// # The pixel bounds here are weak, and the structural bound is what gates
    ///
    /// A worst tile of 40 would let a genuinely wrong glyph through. Whole-page pixel
    /// comparison cannot police text between independent rasterisers: the noise floor is
    /// above the signal, and no choice of number fixes that.
    ///
    /// `min_structural_similarity` is the bound that can. Measured over a 51-document
    /// sample of the pdf.js corpus, 153 reference-against-reference pairs:
    ///
    /// | structural similarity | what those pairs are |
    /// |---|---|
    /// | 0.94 to 1.00, 84% of pairs | ordinary antialiasing and hinting disagreement |
    /// | 0.80 to 0.94 | `standard_fonts`, `tracemonkey_freetext`, `issue1905` — pages where each renderer substitutes a *different* font |
    /// | 0.62 to 0.80 | the same, more severely |
    /// | 0.00 to 0.04 | one renderer produced nothing at all (JBIG2 symbol dictionaries) |
    ///
    /// The distribution is continuous, not bimodal: there is no empty band to put a
    /// threshold in, and 0.8990, 0.8993, 0.8998 and 0.9009 all occur. So 0.90 is a
    /// deliberate choice about *which* population to exclude rather than a natural
    /// boundary, and what it excludes is font substitution. That is the right call: when
    /// two references pick different substitutes there is no single correct rendering to
    /// hold us to, and [`crate::Outcome::Ambiguous`] is exactly where such a page belongs.
    ///
    /// Text *correctness* still belongs to the extraction metric — comparing our output
    /// against `pdftotext` — which checks encoding and `ToUnicode` independently of how
    /// glyphs are painted. What changed is that this gate is no longer only capable of
    /// catching an inverted or blank page.
    pub const TEXT_HEAVY: Self = Self {
        max_mean: 5.0,
        max_worst_tile: 40.0,
        max_differing_fraction: 0.05,
        min_structural_similarity: 0.90,
    };

    /// The strict bounds, [`Self::VECTOR`].
    ///
    /// Strict by default: loosening a gate should be an explicit, visible choice at the
    /// call site rather than something inherited silently.
    pub const DEFAULT: Self = Self::VECTOR;

    /// Returns `true` if a comparison falls within these bounds.
    #[must_use]
    pub fn accepts(&self, comparison: &Comparison) -> bool {
        comparison.mean_error <= self.max_mean
            && comparison.worst_tile_error <= self.max_worst_tile
            && comparison.differing_fraction <= self.max_differing_fraction
            && comparison.structural_similarity >= self.min_structural_similarity
    }

    /// Widens every bound to `factor` times the disagreement `observed` between two
    /// references, keeping whichever of the two is looser.
    ///
    /// # Why a bound derived from the references beats a fixed one
    ///
    /// A fixed number has to serve two populations at once. A page of flat vector fills
    /// leaves the references agreeing to a worst tile of 0.4, so a worst tile of 5 from us
    /// is ten times their entire spread and unmistakably a defect. A page of small text
    /// leaves them disagreeing at a worst tile of 26 among themselves, so the same 5 says
    /// nothing at all. One threshold cannot separate signal from noise on both, and the
    /// one that passes the second silently forgives the first.
    ///
    /// The references' own spread *is* the noise floor on that page — measured on that
    /// page, by implementations that share no code with ours. Judging our deviation as a
    /// multiple of it asks the question that matters: are we further from the consensus than
    /// the consensus is from itself? Surveyed over a spread sample of the pdf.js corpus, that
    /// distinction is the difference between 15 pages outside [`Self::TEXT_HEAVY`] and the 8
    /// among them that two independent renderers genuinely contradict.
    ///
    /// **This comment used to add "or with each other", and that is false on any page whose
    /// difference is a glyph.** `ldd` on this machine puts the same `libfreetype.so.6` under
    /// `pdftoppm`, `mutool` and `gs` — so where two of them agree closely about a letter's
    /// edges they are one rasteriser agreeing with itself, while this tree uses `skrifa` and
    /// `tiny-skia` and is the only genuinely third opinion in the room. The consequence is
    /// one-sided and worth stating plainly: on a text page their spread *understates* the
    /// floor, so a bound derived from it is too tight rather than too loose. What limits the
    /// damage is that widening only ever loosens — the fixed bounds are a floor and
    /// [`Self::TEXT_HEAVY`] was itself measured against these same three programs — so such a
    /// page ends up judged by the fixed bounds rather than by a tighter derived one. Nothing
    /// has been changed on the strength of this: loosening a gate to make contradictions
    /// disappear is the move this project forbids itself, and what would justify a change is a
    /// measurement of how far a *fourth* independent rasteriser sits from the three, which
    /// nobody has.
    ///
    /// The fixed bounds stay as a floor rather than being replaced, because a spread of
    /// zero — two references producing identical pixels, which happens on simple pages —
    /// would otherwise demand exactness of us that no third implementation can deliver.
    #[must_use]
    pub fn widened_to(&self, observed: &Comparison, factor: f64) -> Self {
        Self {
            max_mean: self.max_mean.max(observed.mean_error * factor),
            max_worst_tile: self.max_worst_tile.max(observed.worst_tile_error * factor),
            max_differing_fraction: self
                .max_differing_fraction
                .max(observed.differing_fraction * factor)
                .min(1.0),
            // Structural similarity runs the other way: 1.0 is identity, so the *distance*
            // from 1.0 is what scales.
            min_structural_similarity: self
                .min_structural_similarity
                .min(1.0 - (1.0 - observed.structural_similarity) * factor),
        }
    }
}

/// How our own render is judged, once the references have formed a consensus.
///
/// The consensus itself is always decided by the fixed [`Tolerance`]: deciding whether the
/// references agree from a bound derived from how much they disagree would be circular.
/// This chooses only what happens afterwards.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Judgement {
    /// Judged against the fixed bounds, exactly as the references were.
    ///
    /// Right for a fixture whose content is known and whose bounds were measured on it.
    Absolute,
    /// Judged against bounds widened to `factor` times the disagreement the consensus
    /// references show among themselves — see [`Tolerance::widened_to`].
    ///
    /// Right for a corpus, where one fixed bound has to serve pages of every kind.
    RelativeToReferences {
        /// Multiple of the references' own spread that still counts as agreement.
        factor: f64,
    },
}

impl Judgement {
    /// Twice the references' own spread, the bound the corpus gate uses.
    ///
    /// Two rather than one because the references' spread is the *observed* disagreement
    /// between two implementations, and a third correct implementation is not required to
    /// sit between them: it may differ from both in the same direction, at the same
    /// magnitude. One would fail such a renderer for being correct. Beyond two the bound
    /// starts to forgive real defects on text pages, where the floor is already high.
    pub const CORPUS: Self = Self::RelativeToReferences { factor: 2.0 };
}

impl Default for Tolerance {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// What the triangulation concluded.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Outcome {
    /// A majority of references agree, and our output agrees with them.
    Agrees {
        /// The references that agree with each other and with us.
        with: Vec<Reference>,
    },
    /// A majority of references agree, and our output differs from them. A real bug.
    Regression {
        /// The references that agree with each other but not with us.
        agreeing: Vec<Reference>,
    },
    /// The references disagree among themselves, so there is no answer to hold us to.
    Ambiguous,
    /// Fewer than two references were available, so nothing can be triangulated.
    NotEnoughReferences {
        /// How many were found.
        available: usize,
    },
}

impl Outcome {
    /// Returns `true` if this outcome should fail a build.
    ///
    /// [`Self::Ambiguous`] deliberately does not: see the module documentation.
    /// [`Self::NotEnoughReferences`] does, because a comparison suite silently
    /// degrading to nothing is the failure mode this design exists to avoid.
    #[must_use]
    pub fn is_failure(&self) -> bool {
        matches!(
            self,
            Self::Regression { .. } | Self::NotEnoughReferences { .. }
        )
    }
}

/// The full result of a comparison, including every measurement taken.
#[derive(Debug, Clone, PartialEq)]
pub struct Triangulation {
    /// The conclusion.
    pub outcome: Outcome,
    /// The bounds our own render was actually held to.
    ///
    /// Equal to the tolerance passed in under [`Judgement::Absolute`], and widened by the
    /// consensus references' own disagreement under
    /// [`Judgement::RelativeToReferences`] — in which case a verdict cannot be read
    /// without it.
    pub judged_by: Tolerance,
    /// How our output compared against each reference.
    pub ours: Vec<(Reference, Comparison)>,
    /// How the references compared against each other.
    ///
    /// Reported even on success, because it is the context that makes our own numbers
    /// meaningful: a difference of 0.3 means one thing when the references agree to
    /// 0.002 and quite another when they differ by 0.2 among themselves.
    pub between_references: Vec<(Reference, Reference, Comparison)>,
}

/// Applies the triangulation rule to one page, judging us against the fixed bounds.
///
/// Every raster must already share a size. Callers reconcile renderer rounding first
/// with [`normalise::to_common_size`]; that is kept separate so the reconciliation is
/// reported rather than buried inside the comparison.
///
/// # Errors
///
/// As [`triangulate_with`].
pub fn triangulate(
    ours: &Raster,
    references: &[(Reference, Raster)],
    tolerance: &Tolerance,
) -> Result<Triangulation, HarnessError> {
    triangulate_with(ours, references, tolerance, Judgement::Absolute)
}

/// Applies the triangulation rule to one page, choosing how we ourselves are judged.
///
/// Every raster must already share a size. Callers reconcile renderer rounding first
/// with [`normalise::to_common_size`]; that is kept separate so the reconciliation is
/// reported rather than buried inside the comparison.
///
/// # Errors
///
/// [`HarnessError::Compare`] if any pair cannot be compared — in practice a dimension
/// mismatch, which means a renderer disagreed about the page *size*. That is reported
/// as an error rather than a large difference because it is a different and more
/// serious class of bug than disagreeing about pixels.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "loop indices are bounded by the reference slice, which holds at most the \
              three known renderers"
)]
pub fn triangulate_with(
    ours: &Raster,
    references: &[(Reference, Raster)],
    tolerance: &Tolerance,
    judgement: Judgement,
) -> Result<Triangulation, HarnessError> {
    let mut between_references = Vec::new();
    for (index, (left_ref, left)) in references.iter().enumerate() {
        for (right_ref, right) in references.iter().skip(index + 1) {
            let comparison =
                raster_compare::compare(left, right).map_err(|e| HarnessError::Compare {
                    detail: e.to_string(),
                })?;
            between_references.push((*left_ref, *right_ref, comparison));
        }
    }

    let mut ours_vs = Vec::new();
    for (reference, raster) in references {
        let comparison =
            raster_compare::compare(ours, raster).map_err(|e| HarnessError::Compare {
                detail: e.to_string(),
            })?;
        ours_vs.push((*reference, comparison));
    }

    let (outcome, judged_by) = decide(
        references,
        &between_references,
        &ours_vs,
        tolerance,
        judgement,
    );

    Ok(Triangulation {
        outcome,
        judged_by,
        ours: ours_vs,
        between_references,
    })
}

/// Finds the largest mutually-agreeing group of references and judges us against it.
///
/// Returns the bounds we were actually held to alongside the conclusion, since under
/// [`Judgement::RelativeToReferences`] they are derived from the page and a reader cannot
/// otherwise tell what a verdict meant.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the subset bitmask is bounded by the reference count, at most three"
)]
fn decide(
    references: &[(Reference, Raster)],
    between: &[(Reference, Reference, Comparison)],
    ours: &[(Reference, Comparison)],
    tolerance: &Tolerance,
    judgement: Judgement,
) -> (Outcome, Tolerance) {
    if references.len() < 2 {
        return (
            Outcome::NotEnoughReferences {
                available: references.len(),
            },
            *tolerance,
        );
    }

    // The largest set of references that all agree with one another. With three
    // references this is small enough to check exhaustively, and doing so avoids the
    // subtle bug in "count pairwise agreements": A agreeing with B and B with C does
    // not make A agree with C, and treating it as if it did would let a chain of
    // near-misses masquerade as consensus.
    let names: Vec<Reference> = references.iter().map(|(r, _)| *r).collect();
    let agrees = |a: Reference, b: Reference| {
        between
            .iter()
            .find(|(l, r, _)| (*l == a && *r == b) || (*l == b && *r == a))
            .is_some_and(|(_, _, c)| tolerance.accepts(c))
    };

    let mut best: Vec<Reference> = Vec::new();
    // Every non-empty subset, as a bitmask. Three references means seven subsets.
    for mask in 1u32..(1 << names.len()) {
        let subset: Vec<Reference> = names
            .iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, r)| *r)
            .collect();
        if subset.len() < 2 || subset.len() <= best.len() {
            continue;
        }
        let mutual = subset
            .iter()
            .enumerate()
            .all(|(i, a)| subset.iter().skip(i + 1).all(|b| agrees(*a, *b)));
        if mutual {
            best = subset;
        }
    }

    if best.is_empty() {
        return (Outcome::Ambiguous, *tolerance);
    }

    // The bounds we are held to. Under `RelativeToReferences` they are widened by how far
    // the *consensus* references sit from one another — pairs involving an outlier are
    // excluded deliberately, since an outlier's distance measures its own error and would
    // otherwise buy us licence to be wrong by the same amount.
    let applied = match judgement {
        Judgement::Absolute => *tolerance,
        Judgement::RelativeToReferences { factor } => between
            .iter()
            .filter(|(l, r, _)| best.contains(l) && best.contains(r))
            .fold(*tolerance, |widened, (_, _, comparison)| {
                widened.widened_to(comparison, factor)
            }),
    };

    let we_match_all = best.iter().all(|reference| {
        ours.iter()
            .find(|(r, _)| r == reference)
            .is_some_and(|(_, c)| applied.accepts(c))
    });

    if we_match_all {
        (Outcome::Agrees { with: best }, applied)
    } else {
        (Outcome::Regression { agreeing: best }, applied)
    }
}

/// Failures the harness can report.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum HarnessError {
    /// A reference renderer is not installed.
    #[error("{reference} is not installed (provided by {package})")]
    RendererMissing {
        /// Which reference.
        reference: Reference,
        /// Package that provides it.
        package: &'static str,
    },
    /// A reference renderer ran but did not produce a usable image.
    #[error("{reference} failed: {detail}")]
    RendererFailed {
        /// Which reference.
        reference: Reference,
        /// What went wrong, including the renderer's own stderr where available.
        detail: String,
    },
    /// A reference renderer outlived its budget and was killed.
    ///
    /// Separate from [`Self::RendererFailed`] because it is the only failure here that is
    /// not a property of the document: the same file may render in time on an idle machine
    /// and not on a loaded one. [`crate::cache`] refuses to remember this outcome for that
    /// reason.
    #[error("{reference} exceeded {budget:?} and was killed")]
    RendererTimedOut {
        /// Which reference.
        reference: Reference,
        /// The budget it outlived.
        budget: std::time::Duration,
    },
    /// A PNG could not be read or written.
    #[error("PNG error at {}: {message}", path.display())]
    Png {
        /// File involved.
        path: PathBuf,
        /// Underlying message.
        message: String,
    },
    /// A PNG used a colour type or bit depth the harness does not handle.
    #[error("unsupported PNG at {}: {detail}", path.display())]
    UnsupportedPng {
        /// File involved.
        path: PathBuf,
        /// Which feature was unsupported.
        detail: String,
    },
    /// Two rasters could not be compared.
    #[error("comparison failed: {detail}")]
    Compare {
        /// Underlying message, typically a dimension mismatch.
        detail: String,
    },
}

#[cfg(test)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "test fixtures are built from small literals whose bounds are visible here"
)]
mod tests {
    use super::{Judgement, Outcome, Reference, Tolerance, triangulate, triangulate_with};
    use pdf_render::{Raster, RasterFormat};
    use raster_compare::Comparison;

    fn solid(rgba: [u8; 4]) -> Raster {
        Raster {
            width: 64,
            height: 64,
            format: RasterFormat::Rgba8,
            data: rgba.iter().copied().cycle().take(64 * 64 * 4).collect(),
        }
    }

    /// A raster whose left `columns` columns are `fill` and whose remainder is white.
    ///
    /// Two of these differ by a controllable amount, which is what a bound derived from
    /// one difference and applied to another needs.
    fn banded(columns: usize, fill: [u8; 4]) -> Raster {
        let mut raster = solid(WHITE);
        for y in 0..64usize {
            for x in 0..columns.min(64) {
                let start = (y * 64 + x) * 4;
                raster.data[start..start + 4].copy_from_slice(&fill);
            }
        }
        raster
    }

    const WHITE: [u8; 4] = [255, 255, 255, 255];
    const BLACK: [u8; 4] = [0, 0, 0, 255];
    const GREY: [u8; 4] = [128, 128, 128, 255];

    #[test]
    fn agreement_with_a_consensus_passes() {
        let refs = vec![
            (Reference::Poppler, solid(WHITE)),
            (Reference::MuPdf, solid(WHITE)),
        ];
        let result = triangulate(&solid(WHITE), &refs, &Tolerance::DEFAULT).expect("comparable");
        assert!(matches!(result.outcome, Outcome::Agrees { .. }));
        assert!(!result.outcome.is_failure());
    }

    /// The rule that makes the suite worth running: two independent implementations
    /// agreeing, and us differing, is a bug.
    #[test]
    fn differing_from_a_consensus_is_a_regression() {
        let refs = vec![
            (Reference::Poppler, solid(WHITE)),
            (Reference::MuPdf, solid(WHITE)),
        ];
        let result = triangulate(&solid(BLACK), &refs, &Tolerance::DEFAULT).expect("comparable");
        match result.outcome {
            Outcome::Regression { ref agreeing } => assert_eq!(agreeing.len(), 2),
            other => panic!("expected a regression, got {other:?}"),
        }
        assert!(result.outcome.is_failure());
    }

    /// The rule that keeps the suite trustworthy: where the references cannot agree,
    /// there is no answer to hold us to, so this must not fail.
    #[test]
    fn references_disagreeing_among_themselves_is_not_our_failure() {
        let refs = vec![
            (Reference::Poppler, solid(WHITE)),
            (Reference::MuPdf, solid(BLACK)),
            (Reference::Ghostscript, solid(GREY)),
        ];
        let result = triangulate(&solid(WHITE), &refs, &Tolerance::DEFAULT).expect("comparable");
        assert_eq!(result.outcome, Outcome::Ambiguous);
        assert!(
            !result.outcome.is_failure(),
            "ambiguity must not fail the build"
        );
    }

    /// A majority of two against one outlier still constitutes consensus.
    #[test]
    fn a_two_of_three_majority_forms_the_consensus() {
        let refs = vec![
            (Reference::Poppler, solid(WHITE)),
            (Reference::MuPdf, solid(WHITE)),
            (Reference::Ghostscript, solid(BLACK)),
        ];
        let result = triangulate(&solid(WHITE), &refs, &Tolerance::DEFAULT).expect("comparable");
        match result.outcome {
            Outcome::Agrees { ref with } => {
                assert_eq!(
                    with.len(),
                    2,
                    "the two agreeing references form the consensus"
                );
                assert!(!with.contains(&Reference::Ghostscript));
            }
            other => panic!("expected agreement with the majority, got {other:?}"),
        }
    }

    /// Silently passing with nothing to compare against is the failure mode this whole
    /// design exists to prevent.
    #[test]
    fn a_single_reference_cannot_triangulate_and_fails() {
        let refs = vec![(Reference::Poppler, solid(WHITE))];
        let result = triangulate(&solid(WHITE), &refs, &Tolerance::DEFAULT).expect("comparable");
        assert_eq!(
            result.outcome,
            Outcome::NotEnoughReferences { available: 1 }
        );
        assert!(result.outcome.is_failure());
    }

    /// The arithmetic of [`Tolerance::widened_to`], stated independently of any raster.
    ///
    /// Each error bound becomes the looser of the fixed one and `factor` times what was
    /// observed; the structural bound, which runs the other way, scales its distance from
    /// identity instead.
    #[test]
    fn widening_takes_the_looser_of_the_fixed_bound_and_the_observed_spread() {
        let observed = Comparison {
            mean_error: 4.0,
            max_error: 255,
            worst_tile_error: 30.0,
            worst_tile_at: (0, 0),
            differing_fraction: 0.02,
            structural_similarity: 0.95,
            worst_tile_similarity: 0.5,
            worst_tile_similarity_at: (0, 0),
        };
        let widened = Tolerance::VECTOR.widened_to(&observed, 2.0);

        // Each bound is written as the arithmetic that produces it rather than as its
        // value, so the rule is visible in the assertion.
        let close = |actual: f64, expected: f64| (actual - expected).abs() < 1e-12;
        assert!(
            close(widened.max_mean, 2.0 * 4.0),
            "twice the observed mean"
        );
        assert!(
            close(widened.max_worst_tile, 2.0 * 30.0),
            "twice the observed worst tile"
        );
        assert!(close(widened.max_differing_fraction, 2.0 * 0.02));
        assert!(close(
            widened.min_structural_similarity,
            1.0 - 2.0 * (1.0 - 0.95)
        ));
    }

    /// A spread of zero must not demand exactness of us: the fixed bounds are a floor.
    #[test]
    fn widening_by_an_identical_pair_leaves_the_fixed_bounds_alone() {
        let identical = Comparison {
            mean_error: 0.0,
            max_error: 0,
            worst_tile_error: 0.0,
            worst_tile_at: (0, 0),
            differing_fraction: 0.0,
            structural_similarity: 1.0,
            worst_tile_similarity: 1.0,
            worst_tile_similarity_at: (0, 0),
        };
        assert_eq!(
            Tolerance::VECTOR.widened_to(&identical, 2.0),
            Tolerance::VECTOR
        );
    }

    /// The corpus rule: a deviation no larger than twice what the references allow
    /// *themselves* is not evidence of a defect, though a fixed bound calls it one.
    ///
    /// The structural bound is neutralised here so the test is about the pixel bounds
    /// alone; the arithmetic that widens it is pinned by the two tests above.
    #[test]
    fn a_deviation_within_twice_the_references_own_spread_is_not_a_regression() {
        let poppler = solid(WHITE);
        let mupdf = banded(1, GREY);
        let ours = banded(2, GREY);

        // Bounds a fifth wider than the references' measured disagreement: they agree
        // with each other, and we sit at roughly twice their spread, so we do not.
        let spread = raster_compare::compare(&poppler, &mupdf).expect("same size");
        let tolerance = Tolerance {
            max_mean: spread.mean_error * 1.2,
            max_worst_tile: spread.worst_tile_error * 1.2,
            max_differing_fraction: spread.differing_fraction * 1.2,
            min_structural_similarity: -1.0,
        };
        let refs = vec![(Reference::Poppler, poppler), (Reference::MuPdf, mupdf)];

        let absolute = triangulate(&ours, &refs, &tolerance).expect("comparable");
        assert!(
            matches!(absolute.outcome, Outcome::Regression { .. }),
            "a fixed bound calls this a defect: {absolute:?}"
        );

        let relative =
            triangulate_with(&ours, &refs, &tolerance, Judgement::CORPUS).expect("comparable");
        assert!(
            matches!(relative.outcome, Outcome::Agrees { .. }),
            "twice the references' own spread must forgive it: {relative:?}"
        );
        assert!(
            relative.judged_by.max_mean > tolerance.max_mean,
            "the bounds actually applied must be reported, and must be the widened ones"
        );
    }

    /// Widening must not become a licence to be wrong: where the references agree
    /// exactly, twice nothing is still nothing.
    #[test]
    fn a_relative_judgement_still_fails_where_the_references_agree_exactly() {
        let refs = vec![
            (Reference::Poppler, solid(WHITE)),
            (Reference::MuPdf, solid(WHITE)),
        ];
        let result = triangulate_with(&solid(BLACK), &refs, &Tolerance::VECTOR, Judgement::CORPUS)
            .expect("comparable");
        assert!(matches!(result.outcome, Outcome::Regression { .. }));
        assert_eq!(
            result.judged_by,
            Tolerance::VECTOR,
            "with no spread to widen by, the fixed bounds are what applied"
        );
    }

    /// An outlier's distance from the consensus measures the outlier's error, not the
    /// page's difficulty, so it must not widen anything.
    #[test]
    fn an_outlier_reference_does_not_widen_the_bounds() {
        let refs = vec![
            (Reference::Poppler, solid(WHITE)),
            (Reference::MuPdf, solid(WHITE)),
            (Reference::Ghostscript, solid(BLACK)),
        ];
        let result = triangulate_with(&solid(GREY), &refs, &Tolerance::VECTOR, Judgement::CORPUS)
            .expect("comparable");
        assert_eq!(
            result.judged_by,
            Tolerance::VECTOR,
            "only pairs within the consensus may widen the bounds"
        );
        assert!(matches!(result.outcome, Outcome::Regression { .. }));
    }

    #[test]
    fn a_dimension_mismatch_is_an_error_not_a_large_difference() {
        let mut odd = solid(WHITE);
        odd.height = 32;
        odd.data.truncate(64 * 32 * 4);
        let refs = vec![(Reference::Poppler, solid(WHITE)), (Reference::MuPdf, odd)];
        let err = triangulate(&solid(WHITE), &refs, &Tolerance::DEFAULT).unwrap_err();
        assert!(matches!(err, super::HarnessError::Compare { .. }));
    }
}
