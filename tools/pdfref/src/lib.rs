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
//! # The two rules above can both apply to one page, and which one wins is stated here
//!
//! Agreement is not transitive, so a page can carry **two** maximal agreeing sets, neither
//! contained in the other — `a` with `b` and `b` with `c` while `a` and `c` part. Then two or
//! more references agree *and* the references disagree among themselves, and the first rule's
//! justification is what settles it: two unrelated implementations arriving at **the** answer.
//! Where they arrive at two, each backed by a coincidence of the same standing, there is no
//! ranking between them — mutual agreement is the only ranking this design has, and neither set
//! is contained in the other.
//!
//! So **a verdict about our render is one every maximal consensus reaches**, and where they
//! reach different ones the page is [`Outcome::Ambiguous`]. That is not a third rule but the
//! second one applied at the granularity the first is stated in: on such a page no renderer in
//! the room, ours or a reference's, is outside every reading the references have. ADR 0617 has
//! the argument, the corpus measurement and the two rules it rejected.
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
//! **Two rasters cannot always tell those apart, and the renderer's own words can.** Where every
//! reference is flat and they disagree with one another, the pixels of a blank page badly drawn
//! and of a drawn page nobody decoded are identical — so the second thing this asks is what each
//! program *said*. [`Testimony`] is that, [`Reference::refusals`] is what counts as a refusal in
//! each program's own vocabulary, and [`consensus_abstentions`] has the argument. ADR 0769.
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
pub use reference::{Reference, Testimony};

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
    /// # And the 278 have been read, which is what closed that half (ADR 0776)
    ///
    /// `oracle.rs`'s `a_raised_formation_bound` runs the raise as a counterfactual through
    /// [`crate::Triangulation::rejudged`] on every gate run, and the composition is the
    /// argument the count could not be:
    ///
    /// - **272 of the 276 are one document**, `freeculture.pdf`, and the other four are one
    ///   page each — so what the raise buys is one dense-text book turned from `ambiguous`
    ///   into contradicted rather than a corpus-wide judgement.
    /// - **Not one conviction is on the differing fraction.** 274 are on structural
    ///   similarity, whose 0.90 is the bound this comment says was chosen to put font
    ///   substitution in `ambiguous`, and 2 on the worst tile.
    /// - **On 263 of the 276 a reference agrees with *us* more closely, on the deciding
    ///   measure, than the convicting set agrees with itself** — a wider formation bound is an
    ///   instruction to accept a less close pair, which is trap 12's subject exactly.
    /// - **And the formation half is the floor half by another route**: with our own floor left
    ///   at the class bound the same raise acquits 27 of the 60 contradicted pages, including
    ///   all six ADR 0771 refused the floor raise for, because [`Self::widened_to`] derives the
    ///   bound from the spread of whatever set formed.
    ///
    /// # The two jobs were separated and priced, and the price is why they stay one number
    ///
    /// The eight-hundred-and-forty-fourth session took the narrower move ADR 0243 left open —
    /// **keep 5% for consensus, floor our own judgement higher** — and the two things ADR 0243
    /// says must be true first were both supplied. The rule is that ADR's own: the 99th
    /// percentile of the reference-against-reference distribution, which is where this class's
    /// other three bounds already sit. The population is the one the `ldd` in
    /// [`Self::widened_to`] had hidden: `ghostscript` against `poppler` or `mupdf` is two
    /// *separately linked* `FreeType` copies and neither member is ours, and on text pages its
    /// differing fraction runs median **2.50%**, p99 **12.04%**, against the sharing pair's
    /// median **0.86%**, p99 11.21% — the median nearly tripling across the boundary while the
    /// other three measures do not move across it at all (0.0%, 0.7%, 0.4% rejected against
    /// 0.0%, 1.5%, 0.4%).
    ///
    /// Floored at that 12.04% for our judgement alone, with consensus formation untouched, the
    /// corpus gate reports **1017 agrees / 24 contradicted / 835 ambiguous** against 980 / 60 /
    /// 836. Thirty-six pages leave `contradicted` and none arrives.
    ///
    /// **Six of the thirty-six are why it was not taken.** Five are
    /// `CONTRADICTED_CALRGB_TO_SCREEN`, where `mupdf` and `ghostscript` build an ICC profile out
    /// of Table 63's dictionary and hand it to Little CMS while `poppler` and this tree evaluate
    /// §8.6.5.3 in their own code, and one is `CONTRADICTED_SUBPIXEL_IMAGE`, whose note measures
    /// our §10.7.4 departure and shows it owning the whole margin. A differing fraction is a
    /// *threshold count*, so the same 5–12% arises from a sub-pixel phase on glyph edges and from
    /// a small colour error over a large area — the two mechanisms are indistinguishable in this
    /// measure, and a floor over the class cannot forgive the first without forgiving the second.
    /// **That is why the two jobs cannot be separated by a number**: not because no number could
    /// be derived, but because the measure the number bounds conflates two mechanisms. ADR 0771.
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
    /// difference is a glyph.** Two of the three references grid-fit through one shared object,
    /// so where those two agree closely about a letter's edges they are one rasteriser agreeing
    /// with itself, while this tree uses `skrifa` and `tiny-skia`. The consequence is one-sided
    /// and worth stating plainly: on a text page their spread *understates* the floor, so a
    /// bound derived from it is too tight rather than too loose. What limits the damage is that
    /// widening only ever loosens — the fixed bounds are a floor and [`Self::TEXT_HEAVY`] was
    /// itself measured against these same three programs — so such a page ends up judged by the
    /// fixed bounds rather than by a tighter derived one. Nothing has been changed on the
    /// strength of this: loosening a gate to make contradictions disappear is the move this
    /// project forbids itself.
    ///
    /// **Which two, and the `ldd` that said three.** This paragraph read "`ldd` on this machine
    /// puts the same `libfreetype.so.6` under `pdftoppm`, `mutool` and `gs`" for four hundred
    /// sessions, and `ldd` reports a transitive closure: `gs` was reaching `FreeType` through
    /// `libfontconfig`. `objdump -p`, which is what a binary *asks for*, says `libpoppler.so`
    /// and `libmupdf.so` both name `libfreetype.so.6` while `libgs.so.10` names none and
    /// defines 194 `FT_*` symbols of a statically linked copy of its own. So the sharing pair is
    /// `poppler` + `mupdf`, and `ghostscript` against either is two separate copies —
    /// `crate::Reference::independence` records it and the eight-hundred-and-forty-fourth
    /// session measured what it is worth (below). It is the same algorithm either way, which is
    /// why that population is a weak independence and not a fourth rasteriser.
    ///
    /// # The measurement this asked for, taken a different way
    ///
    /// This paragraph used to end "what would justify a change is a measurement of how far a
    /// *fourth* independent rasteriser sits from the three, which nobody has", and `pdfium` is
    /// still not packaged. **The question a verdict asks is answerable without one**, because it
    /// is not *how far do two implementations sit* but *how far may the excluded one of three sit
    /// from the pair that agreed* — and [`crate::decide`] does not take an arbitrary pair, it
    /// takes the pairs that agree within the fixed bounds, which on a page carrying one is the
    /// **closest** pair in the room. The bound is therefore derived from a selected minimum, and
    /// the honest control is to run this judgement with a *reference* standing where our render
    /// stands. `oracle.rs`'s `substitutions_of` does exactly that, over the corpus; on text pages
    /// the rate at which a consensus contradicts the voting reference it excludes, beside the
    /// rate at which the same consensus contradicts us on the same pages:
    ///
    /// | consensus | it contradicts the reference it excludes | it contradicts us |
    /// |---|---|---|
    /// | `poppler` + `mupdf` | 9.1% of 758 | 5.1% of 759 |
    /// | `mupdf` + `ghostscript` | 3.4% of 675 | 2.4% of 677 |
    /// | `poppler` + `ghostscript` | 0.6% of 650 | 0.9% of 652 |
    ///
    /// **The bound is not what varies; the consensus is.** The same 5% floor convicts a
    /// known-good independent implementation on 0.6% of text pages under the one consensus whose
    /// members do not share the `FreeType` object, and on 9.1% under the one whose members do — and
    /// under all three this tree is contradicted at a rate between a third less and half again as
    /// much as the reference beside it. ADR 0771; `doc/todo/12` says why that does not move the
    /// number.
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
    ///
    /// Two shapes reach it. No two references agree at all, which is what this variant has
    /// always meant; or they agree in **two** maximal sets that reach different conclusions
    /// about our render, which is the module documentation's third bullet and has been a
    /// verdict of its own since ADR 0617. [`Triangulation::divided`] separates them, and the
    /// second is worth separating: those pages have a reading each renderer is inside, where
    /// the first shape has none.
    Ambiguous,
    /// Fewer than two references produced a picture, so nothing can be triangulated.
    ///
    /// A renderer that was not installed, that refused the file, that returned a raster of one
    /// colour on a page another renderer drew, or that returned one while saying it could not
    /// decode the page, all land here; the last two are [`consensus_abstentions`]'s two routes.
    /// What a program emits when it decoded nothing is not a reading of the page, and the
    /// harness has no second operand to compare against it.
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
///   `jbig2dec` produces on one corpus page — **neither abstains on the pixels**, because neither
///   is a reference that drew. That is what the second route below is for.
///
/// # The second route: what the renderer said
///
/// The rule above is a predicate over rasters, and a predicate over rasters cannot reach the last
/// of those three populations. Two uniform rasters of different colours are two failures agreeing
/// at a spread of zero — but a *genuinely blank* page with one broken renderer produces exactly
/// the same three rasters, and this harness's own suite says so:
/// `a_two_of_three_majority_forms_the_consensus` and
/// `references_disagreeing_among_themselves_is_not_our_failure` are two uniform white rasters
/// against a uniform black one, and both must keep the verdicts they assert. A predicate firing
/// on that shape would forgive a render of ours that painted marks on an empty sheet, which is
/// the defect the first route exists not to suppress (ADR 0768 wrote the rule, tested it and
/// reverted it for exactly this reason).
///
/// What separates the two is not in the pixels at all. It is that a renderer which failed
/// **says so**, and the harness threw those words away: [`Reference::render_within`] writes both
/// of a renderer's output streams to a log beside its image, and [`crate::cache`] did not store
/// it, so on a run answered from the cache no such sentence existed. It does now, and
/// [`Testimony`] is that sentence.
///
/// So a reference also abstains where **its raster is one colour and its own log says it did not
/// draw what the page asked for** — [`Reference::refusals`] for what counts as saying that, and
/// on what evidence. Three things bound it:
///
/// - **our own render still never enters it**, in either route, which is what keeps the whole
///   rule non-circular;
/// - **uniformity is still required.** A renderer that complained and drew marks anyway has
///   produced a picture, and there is no ground to discard one; the question this rule asks only
///   arises for a flat sheet;
/// - **silence concludes nothing.** A renderer that printed nothing has given no testimony, and a
///   log that is missing is treated identically — so a caller that collected no logs gets exactly
///   the pixel rule, unchanged. `testimony` being empty is that caller.
#[must_use]
pub fn consensus_abstentions(
    references: &[(Reference, Raster)],
    testimony: &[Testimony],
    between: &[(Reference, Reference, Comparison)],
    tolerance: &Tolerance,
) -> Vec<Reference> {
    let uniform = |name: Reference| {
        references
            .iter()
            .find(|(reference, _)| *reference == name)
            .is_some_and(|(_, raster)| is_uniform(raster))
    };

    let refused = |name: Reference| {
        testimony
            .iter()
            .filter(|given| given.reference() == name)
            .any(|given| given.refusal().is_some())
    };

    references
        .iter()
        .filter(|(reference, raster)| {
            is_uniform(raster)
                && (refused(*reference)
                    || between.iter().any(|(left, right, comparison)| {
                        let other = match (*left == *reference, *right == *reference) {
                            (true, false) => *right,
                            (false, true) => *left,
                            // A pair that is not this reference's says nothing about it, and a
                            // pair of a reference with itself does not exist.
                            _ => return false,
                        };
                        !uniform(other) && !tolerance.accepts(comparison)
                    }))
        })
        .map(|(reference, _)| *reference)
        .collect()
}

/// One maximal set of references that all agree with one another, and what it says about us.
///
/// A page can carry more than one of these, and that fact was invisible until the
/// seven-hundred-and-twenty-seventh session: agreement is not transitive, so with three
/// references `a` agreeing with `b` and `b` with `c` while `a` and `c` differ leaves **two**
/// maximal agreeing pairs — `{a, b}` and `{b, c}` — neither of which contains the other and
/// neither of which is a majority in any sense the other is not. [`Triangulation::consensuses`]
/// holds them all.
///
/// **The verdict is the one they all reach** (ADR 0617). Where they reach different ones the
/// page is [`Outcome::Ambiguous`]; where they concur it is what the first of them concludes,
/// which is what [`decide`] took unconditionally until the seven-hundred-and-twenty-ninth
/// session and which was the enumeration order's choice on the pages where they did not.
///
/// ADR 0616 has the finding and the measurement, ADR 0617 the rule.
#[derive(Debug, Clone, PartialEq)]
pub struct Consensus {
    /// The references, every pair of which is within the class tolerance of the other.
    pub references: Vec<Reference>,
    /// What this set concludes about our render.
    pub outcome: Outcome,
    /// The bounds this set holds us to, widened by its own members' distance from each other.
    pub judged_by: Tolerance,
}

/// The full result of a comparison, including every measurement taken.
#[derive(Debug, Clone, PartialEq)]
pub struct Triangulation {
    /// The conclusion.
    pub outcome: Outcome,
    /// References whose raster was one colour — on a page another reference drew, or beside a
    /// log of their own saying they could not draw it — and which therefore took no part in the
    /// consensus. [`consensus_abstentions`] has both routes.
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
    /// Every maximal agreeing set of references on this page, largest first.
    ///
    /// Empty where no two references agree. Where they all conclude the same thing about us
    /// that conclusion is [`Self::outcome`] and the first set's bounds are [`Self::judged_by`];
    /// where they do not, the page is [`Outcome::Ambiguous`] and [`Self::divided`] names the two
    /// that part — see [`Consensus`].
    pub consensuses: Vec<Consensus>,
}

impl Triangulation {
    /// What this page's verdict would be if the two jobs [`Tolerance`] does were two numbers.
    ///
    /// # The question
    ///
    /// `Tolerance::TEXT_HEAVY::max_differing_fraction` decides whether two references **form** a
    /// consensus, and the same number **floors** the bound [`Tolerance::widened_to`] then derives
    /// for us. ADR 0243 measured what raising it does — 457 pages leave `ambiguous` and 278
    /// arrive contradicted — and left it alone, because a change of that size is a programme of
    /// work rather than a number. ADR 0771 took the *floor* half as far as it goes. This is what
    /// the **formation** half needs: the counterfactual verdict, page by page, so that the 278
    /// can be counted and broken down rather than described.
    ///
    /// # Why it is here rather than in the gate
    ///
    /// It runs [`decide`] — the same function that reached the live verdict — over the
    /// comparisons this triangulation already holds. A counterfactual computed by a second
    /// implementation of the consensus rule would answer a question about that implementation;
    /// `oracle.rs` asserts on every page that `rejudged` with the page's own bounds reproduces
    /// [`Self::outcome`] exactly, which is the calibration trap 13 asks for.
    ///
    /// It renders nothing and compares nothing: every number it reads is already in
    /// [`Self::between_references`] and [`Self::ours`].
    ///
    /// `formation` is the bound two references must agree within to form a consensus; `floor` is
    /// the bound our own render is held to before widening. Passing the same value for both is
    /// what the live gate does.
    #[must_use]
    pub fn rejudged(
        &self,
        formation: &Tolerance,
        floor: &Tolerance,
        judgement: Judgement,
    ) -> (Outcome, Tolerance, Vec<Consensus>) {
        let drew: Vec<Reference> = self.ours.iter().map(|(reference, _)| *reference).collect();
        decide(
            &drew,
            &self.abstained,
            &self.between_references,
            &self.ours,
            formation,
            floor,
            judgement,
        )
    }

    /// The two maximal consensuses that reach different conclusions about our render, if any.
    ///
    /// `None` on every page whose agreeing sets concur — which is every page carrying one, and
    /// most of those carrying two. Where it is `Some`, the page is [`Outcome::Ambiguous`] for
    /// that reason rather than for the usual one, and the pair is what a reader has to see: which
    /// references form each set is the whole of what such a page is about.
    ///
    /// The first element is the set at the head of [`Self::consensuses`], so a caller printing
    /// them prints the one every earlier session's verdict rested on first. ADR 0617.
    #[must_use]
    pub fn divided(&self) -> Option<(&Consensus, &Consensus)> {
        let first = self.consensuses.first()?;
        let rival = self
            .consensuses
            .iter()
            .skip(1)
            .find(|consensus| consensus.agrees_with_us() != first.agrees_with_us())?;
        Some((first, rival))
    }
}

impl Consensus {
    /// Whether this set of references accepts our render.
    #[must_use]
    pub fn agrees_with_us(&self) -> bool {
        matches!(self.outcome, Outcome::Agrees { .. })
    }
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
/// No testimony is collected, so [`consensus_abstentions`]'s second route cannot fire — see
/// [`triangulate_with`] for what that costs and why the pixel rule is the whole of it here.
pub fn triangulate(
    ours: &Raster,
    references: &[(Reference, Raster)],
    tolerance: &Tolerance,
) -> Result<Triangulation, HarnessError> {
    triangulate_with(ours, references, &[], tolerance, Judgement::Absolute)
}

/// Applies the triangulation rule to one page, choosing how we ourselves are judged.
///
/// Every raster must already share a size. Callers reconcile renderer rounding first
/// with [`normalise::to_common_size`]; that is kept separate so the reconciliation is
/// reported rather than buried inside the comparison.
///
/// `testimony` is what each reference said while it drew — [`Reference::testimony`] reads it out
/// of the same work directory the rasters came from — and it can decide a verdict, through
/// [`consensus_abstentions`]'s second route. An empty slice is a caller that collected none, and
/// is treated exactly as three silent renderers would be: nothing is concluded from it.
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
    testimony: &[Testimony],
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

    let abstained = consensus_abstentions(references, testimony, &between_references, tolerance);
    let drew: Vec<Reference> = references.iter().map(|(reference, _)| *reference).collect();
    // The same number in both places, which is the whole of what `doc/todo/12` is about: one
    // bound decides whether the references agree *and* floors what we are held to.
    let (outcome, judged_by, consensuses) = decide(
        &drew,
        &abstained,
        &between_references,
        &ours_vs,
        tolerance,
        tolerance,
        judgement,
    );

    Ok(Triangulation {
        outcome,
        abstained,
        judged_by,
        ours: ours_vs,
        between_references,
        consensuses,
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
///
/// The third return is **every** maximal agreeing set rather than one. Agreement is not
/// transitive, so a page can carry two maximal sets neither of which contains the other; before
/// the seven-hundred-and-twenty-seventh session the second was discarded without being counted,
/// and on a page where the two reach different conclusions about us the verdict was the
/// enumeration's rather than the page's (ADR 0616).
///
/// **A verdict is now the one every maximal consensus reaches**, and a page whose sets divide is
/// [`Outcome::Ambiguous`] — the module documentation has the argument and ADR 0617 the
/// measurement. Where they concur, which is every page carrying one set and most of those
/// carrying two, the outcome and the bounds are the first set's exactly as before.
///
/// # Two tolerances, which the live gate passes one number for
///
/// `formation` is what two references must agree within to form a consensus at all; `floor` is
/// what our own render is held to before [`Tolerance::widened_to`] widens it. They are one
/// number on every path this harness renders through — `doc/todo/12`'s "one bound doing two
/// jobs" — and they are two parameters so that [`Triangulation::rejudged`] can ask what a
/// different formation bound would decide **without a second copy of this function**.
fn decide(
    drew: &[Reference],
    abstained: &[Reference],
    between: &[(Reference, Reference, Comparison)],
    ours: &[(Reference, Comparison)],
    formation: &Tolerance,
    floor: &Tolerance,
    judgement: Judgement,
) -> (Outcome, Tolerance, Vec<Consensus>) {
    let names: Vec<Reference> = drew
        .iter()
        .copied()
        .filter(|r| !abstained.contains(r))
        .collect();

    if names.len() < 2 {
        return (
            Outcome::NotEnoughReferences {
                available: names.len(),
            },
            *floor,
            Vec::new(),
        );
    }

    let consensuses: Vec<Consensus> = maximal_agreements(&names, between, formation)
        .into_iter()
        .map(|group| conclude(&group, between, ours, floor, judgement))
        .collect();

    match consensuses.first() {
        None => (Outcome::Ambiguous, *floor, consensuses),
        Some(first)
            if consensuses
                .iter()
                .any(|other| other.agrees_with_us() != first.agrees_with_us()) =>
        {
            // The class bounds rather than any set's widened ones, which is what the arm above
            // returns and for the same reason: no set judged this page, so quoting one set's
            // bound beside the verdict would name a judgement that was not made.
            (Outcome::Ambiguous, *floor, consensuses)
        }
        Some(first) => (first.outcome.clone(), first.judged_by, consensuses),
    }
}

/// Every maximal set of references that all agree with one another, largest first.
///
/// "Maximal" means no larger agreeing set contains it, which is not the same as "largest":
/// with `a` agreeing with `b` and `b` with `c` while `a` and `c` differ, both `{a, b}` and
/// `{b, c}` are maximal and neither is a majority the other is not.
///
/// Exhaustive over subsets, which three references make cheap, and doing so avoids the subtle
/// bug in "count pairwise agreements": A agreeing with B and B with C does not make A agree
/// with C, and treating it as if it did would let a chain of near-misses masquerade as
/// consensus. The order is by descending size and then by the subset bitmask, which is the
/// order the single `best` this function replaced was found in — so its first element is what
/// that loop returned, unchanged.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the subset bitmask is bounded by the reference count, at most three"
)]
fn maximal_agreements(
    names: &[Reference],
    between: &[(Reference, Reference, Comparison)],
    tolerance: &Tolerance,
) -> Vec<Vec<Reference>> {
    let agrees = |a: Reference, b: Reference| {
        between
            .iter()
            .find(|(l, r, _)| (*l == a && *r == b) || (*l == b && *r == a))
            .is_some_and(|(_, _, c)| tolerance.accepts(c))
    };

    let mut mutual: Vec<Vec<Reference>> = Vec::new();
    // Every non-empty subset, as a bitmask. Three references means seven subsets.
    for mask in 1u32..(1 << names.len()) {
        let subset: Vec<Reference> = names
            .iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, r)| *r)
            .collect();
        if subset.len() < 2 {
            continue;
        }
        if subset
            .iter()
            .enumerate()
            .all(|(i, a)| subset.iter().skip(i + 1).all(|b| agrees(*a, *b)))
        {
            mutual.push(subset);
        }
    }

    let contained_in_a_larger = |subset: &[Reference]| {
        mutual
            .iter()
            .any(|other| other.len() > subset.len() && subset.iter().all(|r| other.contains(r)))
    };
    let mut maximal: Vec<Vec<Reference>> = mutual
        .iter()
        .filter(|subset| !contained_in_a_larger(subset))
        .cloned()
        .collect();
    maximal.sort_by_key(|subset| std::cmp::Reverse(subset.len()));
    maximal
}

/// What one agreeing set of references concludes about our render.
///
/// The bounds are widened by how far the *consensus* references sit from one another — pairs
/// involving a reference outside this set are excluded deliberately, since an outlier's
/// distance measures its own error and would otherwise buy us licence to be wrong by the same
/// amount.
fn conclude(
    group: &[Reference],
    between: &[(Reference, Reference, Comparison)],
    ours: &[(Reference, Comparison)],
    tolerance: &Tolerance,
    judgement: Judgement,
) -> Consensus {
    let applied = match judgement {
        Judgement::Absolute => *tolerance,
        Judgement::RelativeToReferences { factor } => between
            .iter()
            .filter(|(l, r, _)| group.contains(l) && group.contains(r))
            .fold(*tolerance, |widened, (_, _, comparison)| {
                widened.widened_to(comparison, factor)
            }),
    };

    let we_match_all = group.iter().all(|reference| {
        ours.iter()
            .find(|(r, _)| r == reference)
            .is_some_and(|(_, c)| applied.accepts(c))
    });

    let references = group.to_vec();
    let outcome = if we_match_all {
        Outcome::Agrees {
            with: references.clone(),
        }
    } else {
        Outcome::Regression {
            agreeing: references.clone(),
        }
    };
    Consensus {
        references,
        outcome,
        judged_by: applied,
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
    use super::{
        Judgement, Outcome, Reference, Testimony, Tolerance, triangulate, triangulate_with,
    };
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
            triangulate_with(&ours, &refs, &[], &tolerance, Judgement::CORPUS).expect("comparable");
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
        let result = triangulate_with(
            &solid(BLACK),
            &refs,
            &[],
            &Tolerance::VECTOR,
            Judgement::CORPUS,
        )
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
        let result = triangulate_with(
            &solid(GREY),
            &refs,
            &[],
            &Tolerance::VECTOR,
            Judgement::CORPUS,
        )
        .expect("comparable");
        assert_eq!(
            result.judged_by,
            Tolerance::VECTOR,
            "only pairs within the consensus may widen the bounds"
        );
        assert!(matches!(result.outcome, Outcome::Regression { .. }));
    }

    /// Agreement is not transitive, so a page can carry two maximal consensuses.
    ///
    /// Three panels a step apart: the outer two are two steps from their neighbour and four
    /// from each other, so each agrees with the middle one and they do not agree with one
    /// another. Both `{poppler, mupdf}` and `{mupdf, ghostscript}` are maximal — neither is
    /// contained in the other and neither is a majority the other is not — and here they reach
    /// **opposite** verdicts about the same render. Which one the gate took used to be the order
    /// the subsets are enumerated in (ADR 0616); since ADR 0617 neither is taken and the page is
    /// [`Outcome::Ambiguous`], which is what this asserts alongside both sets still being
    /// computed and both conclusions still being readable.
    #[test]
    fn two_maximal_consensuses_can_disagree_about_us() {
        let poppler = banded(1, GREY);
        let mupdf = banded(3, GREY);
        let ghostscript = banded(5, GREY);
        let ours = banded(4, GREY);

        // A fifth wider than one neighbouring step, so a step is inside and two are not.
        let step = raster_compare::compare(&poppler, &mupdf).expect("same size");
        let tolerance = Tolerance {
            max_mean: step.mean_error * 1.2,
            max_worst_tile: step.worst_tile_error * 1.2,
            max_differing_fraction: step.differing_fraction * 1.2,
            min_structural_similarity: -1.0,
        };
        let refs = vec![
            (Reference::Poppler, poppler),
            (Reference::MuPdf, mupdf),
            (Reference::Ghostscript, ghostscript),
        ];

        let result = triangulate(&ours, &refs, &tolerance).expect("comparable");
        assert_eq!(
            result.consensuses.len(),
            2,
            "two maximal agreeing pairs, neither containing the other: {:?}",
            result.consensuses
        );
        assert_eq!(
            result.consensuses[0].references,
            vec![Reference::Poppler, Reference::MuPdf]
        );
        assert_eq!(
            result.consensuses[1].references,
            vec![Reference::MuPdf, Reference::Ghostscript]
        );
        assert!(
            matches!(result.consensuses[0].outcome, Outcome::Regression { .. }),
            "the first calls us wrong"
        );
        assert!(
            matches!(result.consensuses[1].outcome, Outcome::Agrees { .. }),
            "the second calls us right"
        );
        assert_eq!(
            result.outcome,
            Outcome::Ambiguous,
            "two readings of the page, one accepting us and one not, is no reading to hold us to"
        );
        assert_eq!(
            result.judged_by, tolerance,
            "no set judged the page, so the bounds reported are the class's rather than a set's"
        );
        let (taken, rival) = result.divided().expect("the two sets that part");
        assert_eq!(taken.references, vec![Reference::Poppler, Reference::MuPdf]);
        assert_eq!(
            rival.references,
            vec![Reference::MuPdf, Reference::Ghostscript]
        );
    }

    /// The rule fires on the sets *disagreeing*, not on there being two of them.
    ///
    /// The same three panels a step apart, with our render four steps past the far end of them:
    /// both maximal pairs are still there and neither contains the other, and both reject us. A
    /// page like that is contradicted, because every reading the references have puts us outside
    /// it — which is what stops [`Outcome::Ambiguous`] from swallowing the population rather than
    /// the pages ADR 0617 is about. `issue19633.pdf` page 1 is the corpus's instance.
    #[test]
    fn two_maximal_consensuses_that_concur_still_reach_a_verdict() {
        let poppler = banded(1, GREY);
        let mupdf = banded(3, GREY);
        let ghostscript = banded(5, GREY);
        let ours = banded(64, GREY);

        let step = raster_compare::compare(&poppler, &mupdf).expect("same size");
        let tolerance = Tolerance {
            max_mean: step.mean_error * 1.2,
            max_worst_tile: step.worst_tile_error * 1.2,
            max_differing_fraction: step.differing_fraction * 1.2,
            min_structural_similarity: -1.0,
        };
        let refs = vec![
            (Reference::Poppler, poppler),
            (Reference::MuPdf, mupdf),
            (Reference::Ghostscript, ghostscript),
        ];

        let result = triangulate(&ours, &refs, &tolerance).expect("comparable");
        assert_eq!(result.consensuses.len(), 2, "still two maximal pairs");
        assert!(
            result.consensuses.iter().all(|c| !c.agrees_with_us()),
            "and both of them reject us: {:?}",
            result.consensuses
        );
        assert!(result.divided().is_none(), "concurring sets do not divide");
        assert_eq!(
            result.outcome, result.consensuses[0].outcome,
            "so the verdict is theirs, and it is the first set's bounds it was reached under"
        );
        assert_eq!(result.judged_by, result.consensuses[0].judged_by);
    }

    /// Where all three agree, the pairs inside that set are not separate consensuses.
    #[test]
    fn a_unanimous_agreement_is_one_consensus_and_not_four() {
        let refs = vec![
            (Reference::Poppler, solid(WHITE)),
            (Reference::MuPdf, solid(WHITE)),
            (Reference::Ghostscript, solid(WHITE)),
        ];
        let result = triangulate(&solid(WHITE), &refs, &Tolerance::DEFAULT).expect("comparable");
        assert_eq!(result.consensuses.len(), 1);
        assert_eq!(result.consensuses[0].references.len(), 3);
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
    /// that drew — so on the **pixels** neither abstains and the page keeps its verdict. That is
    /// the honest limit of the first route, and it must stay: a genuinely blank page with one
    /// broken renderer has these same three rasters.
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

    /// The same three rasters, with the two renderers' own logs beside them: `mupdf` and
    /// `ghostscript` both say they could not decode the image, so neither flat sheet is a
    /// reading of the page and one reference is left.
    ///
    /// **The logs are verbatim** — `bitmap-symbol-context-reuse.pdf` page 1, asked with §2's own
    /// reference command lines — because a condition on another project's prose is a claim about
    /// a vocabulary this tree does not own, and a paraphrased fixture would pass while the rule
    /// stopped working (trap 13: run the sweep against the defect). `poppler`'s line is here too
    /// and is deliberately **not** a refusal: nothing in its wording separates it from the tens
    /// of thousands of `Syntax Error` lines it writes about defects it recovers from.
    #[test]
    fn a_flat_sheet_whose_renderer_says_it_could_not_decode_is_not_a_reading() {
        let refs = vec![
            (Reference::Poppler, solid(WHITE)),
            (Reference::MuPdf, solid(BLACK)),
            (Reference::Ghostscript, solid(WHITE)),
        ];
        let testimony = vec![
            Testimony::of(
                Reference::Poppler,
                "Syntax Error (681): Too many symbols in JBIG2 symbol dictionary\n",
            ),
            Testimony::of(
                Reference::MuPdf,
                "page bitmap-symbol-context-reuse.pdf 1warning: jbig2dec warning: segment marks \
                 bitmap coding context as retained (NYI) (segment 1)\n\
                 warning: jbig2dec warning: segment marks bitmap coding context as used (NYI) \
                 (segment 2)\n\
                 warning: jbig2dec warning: failed to decode; treating as end of file (segment \
                 2)\n\
                 library error: cannot decode jbig2 image\n\
                 warning: read error; treating as end of file\n\
                 warning: padding truncated image\n",
            ),
            Testimony::of(
                Reference::Ghostscript,
                "jbig2dec WARNING segment marks bitmap coding context as retained (NYI) (segment \
                 0x01)\n\
                 jbig2dec WARNING segment marks bitmap coding context as used (NYI) (segment \
                 0x02)\n\
                 jbig2dec WARNING failed to decode; treating as end of file (segment 0x02)\n",
            ),
        ];

        let result = triangulate_with(
            &banded(8, GREY),
            &refs,
            &testimony,
            &Tolerance::DEFAULT,
            Judgement::Absolute,
        )
        .expect("comparable");
        assert_eq!(
            result.abstained,
            vec![Reference::MuPdf, Reference::Ghostscript],
            "each said it could not decode, and each returned one colour"
        );
        assert_eq!(
            result.outcome,
            Outcome::NotEnoughReferences { available: 1 },
            "one reading is left, and one cannot triangulate"
        );
    }

    /// And the population the rule must not reach: the same rasters, with logs that narrate a
    /// defect each program recovered from. Nobody abstains and the verdict is the pixels'.
    ///
    /// Both sentences are the commonest of their kind in the oracle's corpus — `mupdf` repairs a
    /// broken cross-reference table on 14 flat sheets and draws them, `poppler` writes 28 901
    /// `Type mismatch in PostScript function` lines on pages nobody disputes — which is why
    /// reading a program's *severity* rather than what it says it produced would be trap 11.
    #[test]
    fn a_flat_sheet_whose_renderer_recovered_keeps_its_vote() {
        let refs = vec![
            (Reference::Poppler, solid(WHITE)),
            (Reference::MuPdf, solid(BLACK)),
            (Reference::Ghostscript, solid(WHITE)),
        ];
        let testimony = vec![
            Testimony::of(
                Reference::Poppler,
                "Syntax Error: Type mismatch in PostScript function\n",
            ),
            Testimony::of(
                Reference::MuPdf,
                "warning: trying to repair broken xref\nwarning: repairing PDF document\n",
            ),
            Testimony::silent(Reference::Ghostscript),
        ];

        let result = triangulate_with(
            &banded(8, GREY),
            &refs,
            &testimony,
            &Tolerance::DEFAULT,
            Judgement::Absolute,
        )
        .expect("comparable");
        assert!(
            result.abstained.is_empty(),
            "a recovered defect is not a refusal: {:?}",
            result.abstained
        );
        match result.outcome {
            Outcome::Regression { ref agreeing } => assert_eq!(agreeing.len(), 2),
            other => panic!("expected a regression, got {other:?}"),
        }
    }

    /// Testimony reaches a raster of one colour and nothing else.
    ///
    /// A renderer that complained and drew marks anyway has produced a picture, and there is no
    /// ground to discard one. Here every panel carries the same mark and `mupdf`'s log is the
    /// refusal from the test above.
    #[test]
    fn a_renderer_that_refused_and_drew_anyway_keeps_its_vote() {
        let refs = vec![
            (Reference::Poppler, banded(8, GREY)),
            (Reference::MuPdf, banded(8, GREY)),
            (Reference::Ghostscript, banded(8, GREY)),
        ];
        let testimony = vec![Testimony::of(
            Reference::MuPdf,
            "library error: cannot decode jbig2 image\n",
        )];

        let result = triangulate_with(
            &banded(8, GREY),
            &refs,
            &testimony,
            &Tolerance::DEFAULT,
            Judgement::Absolute,
        )
        .expect("comparable");
        assert!(result.abstained.is_empty());
        match result.outcome {
            Outcome::Agrees { ref with } => assert_eq!(with.len(), 3),
            other => panic!("expected agreement, got {other:?}"),
        }
    }

    /// A silent renderer gives no testimony, so the pixel rule is the whole of the decision.
    ///
    /// This is what a caller that collected no logs gets, and it is why passing an empty slice to
    /// [`triangulate_with`] is safe rather than merely convenient.
    #[test]
    fn silence_concludes_nothing_either_way() {
        let refs = vec![
            (Reference::Poppler, solid(WHITE)),
            (Reference::MuPdf, solid(BLACK)),
            (Reference::Ghostscript, solid(WHITE)),
        ];
        let silent: Vec<Testimony> = Reference::ALL
            .iter()
            .map(|r| Testimony::silent(*r))
            .collect();

        let with_silence = triangulate_with(
            &banded(8, GREY),
            &refs,
            &silent,
            &Tolerance::DEFAULT,
            Judgement::Absolute,
        )
        .expect("comparable");
        let without =
            triangulate(&banded(8, GREY), &refs, &Tolerance::DEFAULT).expect("comparable");
        assert_eq!(with_silence.abstained, without.abstained);
        assert_eq!(with_silence.outcome, without.outcome);
        assert!(with_silence.abstained.is_empty());
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

    /// The calibration `Triangulation::rejudged` is only useful with: handed the bounds the page
    /// was actually judged by, it must reproduce the verdict the page actually got.
    ///
    /// A counterfactual that cannot reproduce the fact is measuring its own arithmetic, which is
    /// trap 13's rule applied to a re-judgement rather than to a grep. `oracle.rs` asserts the
    /// same identity over every corpus page on every run; this pins it on the three shapes a
    /// verdict has, where a fixture can state each of them outright.
    #[test]
    fn rejudging_with_the_same_bounds_reproduces_the_verdict() {
        /// One fixture: what the verdict should be called, our render, and the references.
        struct Case(&'static str, Raster, Vec<(Reference, Raster)>);

        let cases = vec![
            Case(
                "agrees",
                solid(WHITE),
                vec![
                    (Reference::Poppler, solid(WHITE)),
                    (Reference::MuPdf, solid(WHITE)),
                ],
            ),
            Case(
                "contradicted",
                solid(BLACK),
                vec![
                    (Reference::Poppler, solid(WHITE)),
                    (Reference::MuPdf, solid(WHITE)),
                ],
            ),
            Case(
                "ambiguous",
                solid(WHITE),
                vec![
                    (Reference::Poppler, solid(WHITE)),
                    (Reference::MuPdf, solid(BLACK)),
                    (Reference::Ghostscript, solid(GREY)),
                ],
            ),
        ];
        for Case(label, ours, refs) in cases {
            let live = triangulate(&ours, &refs, &Tolerance::DEFAULT).expect("comparable");
            let (outcome, judged_by, consensuses) = live.rejudged(
                &Tolerance::DEFAULT,
                &Tolerance::DEFAULT,
                Judgement::Absolute,
            );
            assert_eq!(outcome, live.outcome, "{label}: outcome");
            assert_eq!(judged_by, live.judged_by, "{label}: bounds");
            assert_eq!(consensuses, live.consensuses, "{label}: sets");
        }
    }

    /// And the counterfactual has to be able to *move*, or the assertion above is vacuous.
    ///
    /// Two references 8 columns apart form no consensus under the class bound and do form one
    /// under a bound wide enough to admit them — which is the whole mechanism `doc/todo/12`
    /// item 1 is about: raising the formation bound manufactures consensuses, and each one then
    /// reaches a verdict about us that nobody has looked at.
    #[test]
    fn a_wider_formation_bound_forms_a_consensus_that_then_convicts() {
        let refs = vec![
            (Reference::Poppler, banded(8, GREY)),
            (Reference::MuPdf, banded(4, GREY)),
        ];
        let live = triangulate(&solid(WHITE), &refs, &Tolerance::DEFAULT).expect("comparable");
        assert_eq!(
            live.outcome,
            Outcome::Ambiguous,
            "the fixture must start with no consensus, or this proves nothing"
        );

        let wide = Tolerance {
            max_mean: 255.0,
            max_worst_tile: 255.0,
            max_differing_fraction: 1.0,
            min_structural_similarity: -1.0,
        };
        let (outcome, _, consensuses) =
            live.rejudged(&wide, &Tolerance::DEFAULT, Judgement::Absolute);
        assert_eq!(consensuses.len(), 1, "the two references now agree");
        match outcome {
            Outcome::Regression { ref agreeing } => assert_eq!(agreeing.len(), 2),
            other => panic!("expected the new consensus to contradict us, got {other:?}"),
        }
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
