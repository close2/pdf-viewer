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
//! # The one picture that carries no evidence
//!
//! The rule above rests on two unrelated implementations arriving at the same answer being
//! improbable unless the answer is right. That improbability is absent for exactly one
//! picture: the empty one. Every way of failing produces the page background, so two
//! renderers that each decoded nothing agree exactly, and their agreement says nothing about
//! the file. [`consensus_abstentions`] takes such a reference out of the vote — but only where
//! a reference that drew marks *disagrees* with it, because a page whose correct rendering is
//! a flat sheet is not the same thing, and a rule that could not tell those apart would
//! forgive a reader that painted marks on an empty page.
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
pub mod extract;
pub mod normalise;
pub mod png_io;
pub mod reference;
pub mod report;

use std::path::PathBuf;

use pdf_render::Raster;
use raster_compare::Comparison;

pub use cache::Cache;
pub use extract::{ExtractionCache, ExtractionError, Extractor};
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
    ///
    /// # Both halves re-run in the four-hundred-and-seventh session
    ///
    /// `pdfref`'s own `end_to_end` test prints the first sentence's numbers on every run, and
    /// today's renderers give **0.0016 to 0.0352** and **0.4062 to 1.0625** — the worst tile
    /// exactly as written, the mean's upper end 0.035 where the sentence says 0.047, which is
    /// four hundred sessions of `poppler` and `ghostscript` releases and not a correction.
    ///
    /// The 51-document sample is gone and cannot be re-run, but the whole corpus can be, and
    /// `oracle.rs`'s `the_fixed_bounds_against_the_references_own_spread` does it. Over the
    /// **1121** reference pairs on vector pages, each measure taken over the pairs the other
    /// three admit, the share of pairs each of these four bounds rejects is **0.0%, 9.7%, 2.8%
    /// and 3.3%** in the order the fields are declared. So the sample's "0.9971 at worst" does
    /// not hold corpus-wide — the worst is 0.9300 — and what does hold is that all four bounds
    /// sit in the same place relative to their own references: at the top of the distribution,
    /// rejecting a few percent of it. **`max_differing_fraction` here has no written
    /// derivation of its own and needs none**, because it lands where its three siblings land.
    /// [`Self::TEXT_HEAVY`]'s does not, and that is where the finding is.
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
    /// # `max_differing_fraction` is the one bound here set *below* the spread it was
    /// measured on, and the sentence above is why
    ///
    /// Re-run on its own population in the four-hundred-and-seventh session — the 14
    /// specification PDFs' first pages, **42 reference pairs** — the worst tile reproduces to
    /// the digit: median 18.42, p90 **26.72**, max **28.17**, against a bound of 40. The
    /// differing fraction on those same pairs is median **3.11%**, p90 **4.99%**, max
    /// **5.14%**, and **11.9% of them are already outside the 5.00% written here**. The
    /// number the sentence attributes to it, 2.7, is `mean_error`'s maximum on that
    /// population — 2.7355 — so the one bound of the four that names no derivation of its own
    /// is the one that was given another measure's.
    ///
    /// Over the whole corpus the gap is wider and one-sided. Across **2638** pairs of the
    /// three references on text pages, each measure taken over the pairs the other three
    /// admit, the share of pairs each bound rejects is **0.0%** (mean), **1.2%** (worst tile),
    /// **29.4%** (differing fraction) and **0.5%** (structural similarity). Three of these
    /// four sit at or above the 99th percentile of what two independent implementations do to
    /// each other on a page of this class; the fourth sits between the median (1.69%) and the
    /// 90th percentile (10.38%).
    ///
    /// **It is left where it is, and that is a decision rather than an omission — ADR 0243.**
    /// The bound does two jobs: it decides whether two references form a consensus at all,
    /// and it floors the per-page bound [`Self::widened_to`] derives. Raising it to the 99th
    /// percentile of the reference spread, 12.02%, was run over the corpus and forms **457**
    /// new consensuses, of which **278** then contradict us — so the derived value cannot be
    /// adopted without arguing 278 pages, and adopting it for our own side alone would loosen
    /// the gate in the one direction that flatters us. `doc/todo/12` is the work.
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
    /// # The four-hundred-and-seventh session took that measurement and it is still not that
    /// renderer
    ///
    /// `hayro` is independent of all three C references and does not grid-fit, so a
    /// `hayro`-against-`poppler` pair is one hinting renderer against one that is not, with
    /// neither of the two being us. Over the corpus's text pages, on the pairs every other
    /// bound admits, that population's differing fraction runs median **3.42%**, p90 11.24%,
    /// p99 16.00% against the three C references' own median **1.69%**, p90 10.38%, p99
    /// 12.02% — the *median* doubling as soon as one member of the pair stops hinting. So the
    /// sentence above is quantified: their spread understates the floor, and by how much is
    /// now a number.
    ///
    /// It still justifies no change, and the reason is the one [`crate::Reference`] states.
    /// `hayro` shares `skrifa` with this tree, so widening our own bound on the strength of
    /// how far *it* sits from `poppler` would forgive whatever the two of us get wrong
    /// together — which is the circularity `Reference::independence` exists to prevent, and
    /// the same circularity whether it reaches the verdict through a vote or through a bound.
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
    /// Fewer than two references produced a picture, so nothing can be triangulated.
    ///
    /// A renderer that was not installed, that refused the file, or that returned a raster of
    /// one colour on a page another renderer drew all land here, and the last of the three is
    /// [`consensus_abstentions`]'s: what a program emits when it decoded nothing is not a
    /// reading of the page, and the harness has no second operand to compare against it.
    NotEnoughReferences {
        /// How many produced a picture of the page.
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

/// Whether every pixel of a raster is one colour.
///
/// A page is a picture of something. A raster of one colour is a picture of nothing, and
/// [`consensus_abstentions`] is what this exists for; the argument is there rather than here,
/// because on its own this function asks a question about pixels and answers it exactly.
///
/// Exactly rather than within a noise floor, deliberately. A renderer that drew anything at
/// all antialiases its edges, so the population this separates is not near-uniform rasters
/// but rasters that carry a single value — which is what a program that decoded nothing
/// emits, and which no threshold has to be invented to recognise.
#[must_use]
pub fn is_uniform(raster: &Raster) -> bool {
    let mut pixels = raster.data.chunks_exact(4);
    let Some(first) = pixels.next() else {
        // No pixels at all is not a picture either, and the callers below treat it as one
        // colour rather than as a special case.
        return true;
    };
    pixels.all(|pixel| pixel == first)
}

/// Which references took no part in the consensus, because their raster is one colour and a
/// reference that drew marks does not agree with it.
///
/// # Why a uniform raster is usually not a vote
///
/// The whole instrument rests on ADR 0005: two implementations sharing no code arriving at the
/// same picture is evidence that the picture is right, because two *wrong* implementations
/// arriving at the same wrong picture is improbable. That improbability is the inference, and
/// it fails completely for one picture — the empty one. A renderer that refused an image, gave
/// up on a filter, lost a font or threw an exception emits the page background, and so does
/// every other renderer that failed for a wholly different reason. Two failures agree with each
/// other exactly, at a spread of zero, which under [`Tolerance::widened_to`] is also the
/// *tightest* bound the harness can hold anything to.
///
/// Worse, the comparison then has no second operand: against a constant raster every measure
/// [`raster_compare`] produces is a statistic of our own render, the mean being exactly
/// `255 × (1 − our own mean channel value)`. ADR 0499 measured that identity on six corpus
/// pages, digit for digit on all four bounds at once.
///
/// # Two things a uniform raster can be, and only one of them is a failure
///
/// A raster of one colour is uninformative only about a page that has marks on it, and two
/// quite different pages produce one:
///
/// - a page whose correct rendering is a flat sheet — an empty page, or one covered by a
///   single fill. There the flat sheet is the right answer and a renderer that produced it
///   *read* the file;
/// - a page with marks on it, which this renderer did not draw.
///
/// Our own render cannot tell those apart without circularity: a reader that painted nothing
/// would otherwise excuse every reference that painted nothing, and a reader that painted
/// something would disqualify every reference that disagreed with it.
///
/// The non-circular answer is the **other references**, and the question to ask them is not
/// "did anybody draw" but "does anybody who drew *disagree*". A reference whose marks are
/// inside [`Tolerance`] of a flat sheet has drawn a page that is, at this bound, a flat sheet;
/// the two rasters are the same picture as far as this instrument can measure, and nothing is
/// gained by refusing one of them a vote. It is the reference that drew marks *and* falls
/// outside the bound which establishes that there was a picture to draw — and against that,
/// one flat colour is a renderer that did not draw it.
///
/// So: **a uniform raster abstains exactly where a reference that drew marks fails to agree
/// with it**, judged by the same [`Tolerance::accepts`] that decides every other agreement
/// here. Three consequences worth stating, because each is a population:
///
/// - where every reference is uniform, none abstains. Every independent reading of the file
///   says the page is a flat sheet, which is a reading; a render of ours that puts marks on
///   such a page stays contradicted, and that is the defect this rule must not suppress;
/// - where a reference draws marks a flat sheet is inside the bound of, none abstains either,
///   and the page keeps whatever verdict it had;
/// - where two references disagree and both are uniform — one white, one black, which is what
///   `jbig2dec` produces on one corpus page — neither abstains, because neither is a reference
///   that drew. That page is outside what this rule can reach, and it is left visible rather
///   than reached for by loosening the predicate.
#[must_use]
pub fn consensus_abstentions(
    references: &[(Reference, Raster)],
    between: &[(Reference, Reference, Comparison)],
    tolerance: &Tolerance,
) -> Vec<Reference> {
    let uniform = |name: Reference| {
        references
            .iter()
            .find(|(reference, _)| *reference == name)
            .is_some_and(|(_, raster)| is_uniform(raster))
    };

    references
        .iter()
        .filter(|(reference, raster)| {
            is_uniform(raster)
                && between.iter().any(|(left, right, comparison)| {
                    let other = match (*left == *reference, *right == *reference) {
                        (true, false) => *right,
                        (false, true) => *left,
                        // A pair that is not this reference's says nothing about it, and a
                        // pair of a reference with itself does not exist.
                        _ => return false,
                    };
                    !uniform(other) && !tolerance.accepts(comparison)
                })
        })
        .map(|(reference, _)| *reference)
        .collect()
}

/// The full result of a comparison, including every measurement taken.
#[derive(Debug, Clone, PartialEq)]
pub struct Triangulation {
    /// The conclusion.
    pub outcome: Outcome,
    /// References whose raster was one colour on a page another reference drew, and which
    /// therefore took no part in the consensus — [`consensus_abstentions`].
    ///
    /// They are still measured and still reported: what they produced is a fact about the
    /// page worth reading, and it is the evidence for the abstention itself.
    pub abstained: Vec<Reference>,
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

    let abstained = consensus_abstentions(references, &between_references, tolerance);
    let (outcome, judged_by) = decide(
        references,
        &abstained,
        &between_references,
        &ours_vs,
        tolerance,
        judgement,
    );

    Ok(Triangulation {
        outcome,
        abstained,
        judged_by,
        ours: ours_vs,
        between_references,
    })
}

/// Finds the largest mutually-agreeing group of references and judges us against it.
///
/// `abstained` names the references that produced no picture of the page — see
/// [`consensus_abstentions`], which has the argument. They are excluded from the consensus
/// search and from the widening, and a page left with fewer than two references that did draw
/// is [`Outcome::NotEnoughReferences`]: not because a renderer was missing, but because what
/// it returned cannot be compared with anything.
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
    abstained: &[Reference],
    between: &[(Reference, Reference, Comparison)],
    ours: &[(Reference, Comparison)],
    tolerance: &Tolerance,
    judgement: Judgement,
) -> (Outcome, Tolerance) {
    // The largest set of references that all agree with one another. With three
    // references this is small enough to check exhaustively, and doing so avoids the
    // subtle bug in "count pairwise agreements": A agreeing with B and B with C does
    // not make A agree with C, and treating it as if it did would let a chain of
    // near-misses masquerade as consensus.
    let names: Vec<Reference> = references
        .iter()
        .map(|(r, _)| *r)
        .filter(|r| !abstained.contains(r))
        .collect();

    if names.len() < 2 {
        return (
            Outcome::NotEnoughReferences {
                available: names.len(),
            },
            *tolerance,
        );
    }
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
        // Every panel carries a mark. A reference of one flat colour would abstain under
        // `consensus_abstentions` and leave nothing to widen by, which is a different test
        // and is the one below.
        let poppler = banded(1, GREY);
        let mupdf = banded(2, GREY);
        let ours = banded(3, GREY);

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

    /// Two renderers that each decoded nothing agree exactly, and that agreement is
    /// manufactured by the shape of failure rather than by a reading of the file.
    #[test]
    fn two_references_that_drew_nothing_do_not_form_a_consensus() {
        let refs = vec![
            (Reference::Poppler, banded(8, BLACK)),
            (Reference::MuPdf, solid(WHITE)),
            (Reference::Ghostscript, solid(WHITE)),
        ];
        let result =
            triangulate(&banded(8, BLACK), &refs, &Tolerance::DEFAULT).expect("comparable");
        assert_eq!(
            result.outcome,
            Outcome::NotEnoughReferences { available: 1 },
            "with the two blank rasters abstaining, one reading is left and one cannot triangulate"
        );
        assert_eq!(
            result.abstained,
            vec![Reference::MuPdf, Reference::Ghostscript]
        );
        assert_eq!(
            result.ours.len(),
            3,
            "an abstaining reference is still measured and still reported"
        );
    }

    /// The over-reach this rule must not commit: where *no* reference drew, a blank page is
    /// what every independent reading of the file says, and a reader that painted marks on it
    /// is contradicted exactly as before.
    #[test]
    fn a_page_no_reference_draws_still_contradicts_a_reader_that_draws() {
        let refs = vec![
            (Reference::Poppler, solid(WHITE)),
            (Reference::MuPdf, solid(WHITE)),
            (Reference::Ghostscript, solid(WHITE)),
        ];
        let result =
            triangulate(&banded(8, BLACK), &refs, &Tolerance::DEFAULT).expect("comparable");
        match result.outcome {
            Outcome::Regression { ref agreeing } => assert_eq!(agreeing.len(), 3),
            other => panic!("expected a regression, got {other:?}"),
        }
        assert!(
            result.abstained.is_empty(),
            "nobody abstains where nobody drew"
        );
    }

    /// And the same page drawn blank by us agrees, which is the population the rule above
    /// would suppress if it fired on uniformity alone.
    #[test]
    fn a_blank_page_every_renderer_agrees_about_is_still_an_agreement() {
        let refs = vec![
            (Reference::Poppler, solid(WHITE)),
            (Reference::MuPdf, solid(WHITE)),
            (Reference::Ghostscript, solid(WHITE)),
        ];
        let result = triangulate(&solid(WHITE), &refs, &Tolerance::DEFAULT).expect("comparable");
        match result.outcome {
            Outcome::Agrees { ref with } => assert_eq!(with.len(), 3),
            other => panic!("expected agreement, got {other:?}"),
        }
    }

    /// A renderer that emitted a flat sheet of the wrong colour is failing just as plainly as
    /// one that emitted white, and `mupdf` does exactly this on nineteen corpus pages.
    #[test]
    fn a_uniform_raster_abstains_whatever_its_colour() {
        let refs = vec![
            (Reference::Poppler, banded(8, GREY)),
            (Reference::MuPdf, solid(BLACK)),
            (Reference::Ghostscript, solid(WHITE)),
        ];
        let result = triangulate(&banded(8, GREY), &refs, &Tolerance::DEFAULT).expect("comparable");
        assert_eq!(
            result.abstained,
            vec![Reference::MuPdf, Reference::Ghostscript]
        );
        assert_eq!(
            result.outcome,
            Outcome::NotEnoughReferences { available: 1 }
        );
    }

    /// The refinement that keeps this rule off pages that are *legitimately* flat: a mark
    /// small enough to sit inside the bound means the page is a flat sheet at this bound, so
    /// the flat renders are reading it rather than failing at it.
    ///
    /// Nine corpus pages turn on this distinction, and without it every one of them lost an
    /// agreement it had earned.
    #[test]
    fn a_uniform_raster_that_no_drawn_page_disagrees_with_keeps_its_vote() {
        // One column of a colour a single channel apart from white: `raster_compare` counts
        // it, and every bound admits it.
        let faint = banded(1, [255, 254, 255, 255]);
        let refs = vec![
            (Reference::Poppler, faint.clone()),
            (Reference::MuPdf, solid(WHITE)),
            (Reference::Ghostscript, solid(WHITE)),
        ];
        let result = triangulate(&faint, &refs, &Tolerance::DEFAULT).expect("comparable");
        assert!(
            result.abstained.is_empty(),
            "a flat sheet inside the bound of the drawn page is a reading of it: {:?}",
            result.abstained
        );
        match result.outcome {
            Outcome::Agrees { ref with } => assert_eq!(with.len(), 3),
            other => panic!("expected agreement, got {other:?}"),
        }
    }

    /// Two uniform rasters of different colours are two failures, and neither is a reference
    /// that drew — so neither abstains and the page keeps its verdict. `jbig2dec` produces
    /// exactly this on `bitmap-symbol-context-reuse.pdf`, and it is the honest limit of the
    /// rule rather than a case to reach by loosening the predicate.
    #[test]
    fn two_uniform_rasters_disagreeing_with_each_other_reach_no_abstention() {
        let refs = vec![
            (Reference::Poppler, solid(WHITE)),
            (Reference::MuPdf, solid(BLACK)),
            (Reference::Ghostscript, solid(WHITE)),
        ];
        let result = triangulate(&banded(8, GREY), &refs, &Tolerance::DEFAULT).expect("comparable");
        assert!(result.abstained.is_empty());
        match result.outcome {
            Outcome::Regression { ref agreeing } => assert_eq!(agreeing.len(), 2),
            other => panic!("expected a regression, got {other:?}"),
        }
    }

    /// One abstention out of three still leaves a consensus, and it is the two that drew.
    #[test]
    fn one_abstention_leaves_the_two_that_drew_to_decide() {
        let refs = vec![
            (Reference::Poppler, banded(8, GREY)),
            (Reference::MuPdf, banded(8, GREY)),
            (Reference::Ghostscript, solid(WHITE)),
        ];
        let result = triangulate(&solid(WHITE), &refs, &Tolerance::DEFAULT).expect("comparable");
        match result.outcome {
            Outcome::Regression { ref agreeing } => assert_eq!(
                agreeing,
                &[Reference::Poppler, Reference::MuPdf],
                "the abstaining reference must not join the consensus it agrees with"
            ),
            other => panic!("expected a regression, got {other:?}"),
        }
    }

    #[test]
    fn uniformity_is_exact_rather_than_within_a_noise_floor() {
        assert!(super::is_uniform(&solid(WHITE)));
        assert!(super::is_uniform(&solid(BLACK)));
        assert!(!super::is_uniform(&banded(1, [254, 255, 255, 255])));
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
