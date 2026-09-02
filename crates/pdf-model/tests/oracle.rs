//! Every page of the corpus, drawn by us and by three independent renderers.
//!
//! # What this gate is for, and how it differs from `corpus.rs`
//!
//! `corpus.rs` asks whether we *reported* everything we could not draw. This asks whether
//! what we did draw is *right*, which no amount of self-reporting can establish. The two
//! defects found in the fourth working session — every gradient mirrored about the page's
//! centre line, every image sampled through a doubled transform — sat on pages that
//! reported nothing wrong, on both backends at once, and would have been caught the first
//! time this ran.
//!
//! The oracle is [`pdfref`]'s triangulation rule: where two independent implementations
//! agree with each other and we differ from both, we are wrong. Where they disagree among
//! themselves there is no answer to hold us to, and the page is recorded rather than
//! failed. Principle 5 governs what happens next: a contradiction is a question to take
//! back to the specification, never a target to move towards. The value of this gate is
//! that it *finds the page*; it never says what the answer is.
//!
//! # Why the bound is relative to the references' own disagreement
//!
//! A fixed tolerance has to serve a page of flat fills, where the references agree to a
//! worst tile of 0.4, and a page of small text, where they disagree at 26 among
//! themselves. No single number separates signal from noise on both. So the references'
//! own spread on *that page* sets the bound — see [`pdfref::Judgement::CORPUS`] — and the
//! question asked is whether we are further from the consensus than the consensus is from
//! itself. On a spread sample of this corpus that distinction was the difference between
//! 15 pages outside the fixed bounds and the 8 among them that are genuinely contradicted.
//!
//! # What is compared
//!
//! Every page of all 974 pdf.js corpus documents, and page one of the 14 specification PDFs
//! in `doc/` — 1794 pages. The corpus files are there because each one broke a reader once
//! and the interesting page is not always the first; the specification PDFs are 1382 pages
//! of consistent typesetting from 14 files, where page 500 exercises what page 499 did. See
//! [`work_items`].
//!
//! # What is gated, and what is only recorded
//!
//! Only pages we claim to draw *completely*. A page whose interpretation reports an
//! unsupported font or an undecodable image is expected to differ from a renderer that
//! implements it, and listing all of them would drown the signal. Those pages are counted
//! and printed — the count is a rough measure of how much the missing features cost
//! visually — but they cannot fail this gate. `corpus.rs` owns them.
//!
//! # Running it
//!
//! ```text
//! cargo test --release -p pdf-model --test oracle -- --ignored --nocapture
//! ```
//!
//! Artefacts for every page that is not agreement — our render, each reference's, and a
//! difference heatmap — are kept under `<target>/tmp/oracle/<stem>/p<n>/` and named in the
//! output. Pages that agree have theirs deleted: five thousand PNGs of pages nobody will
//! look at is a gigabyte of evidence for nothing.

// `diagnosed_ambiguous` chains one iterator per `AMBIGUOUS_*` group, and each `.chain` nests
// the type one level deeper. The sixty-third group, added in the four-hundredth session,
// overflowed the compiler's default query depth of 128 while computing that type's layout —
// which is a limit on the *type* rather than on anything the test does at run time. Raised
// here rather than folding the groups into a slice of slices, because the chain is what makes
// a group's name appear beside its diagnosis at the one place both are read.
#![recursion_limit = "256"]
#![expect(
    clippy::panic,
    clippy::print_stdout,
    reason = "test code: the survey output is the point of the run, on a failure it is the \
              evidence, and an explanatory panic is the intended failure"
)]
#![expect(
    clippy::doc_markdown,
    reason = "the group comments quote the standard, and a quotation with backticks added \
              to please a lint is no longer a quotation"
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use pdf_render::{Raster, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use pdfref::{Cache, Judgement, Outcome, Reference, Tolerance, normalise, report};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use render_cpu::CpuRasterizer;

/// Comparison resolution. 72 dpi means one pixel per PDF unit, so a difference is a
/// difference rather than a resampling artefact.
const DPI: u32 = 72;

/// The same resolution as our scale factor, in pixels per PDF unit.
const SCALE: f32 = 1.0;

/// Pixel budget per page, the same one `corpus.rs` uses.
const PIXEL_BUDGET: u64 = 64 << 20;

/// Pages we claim to draw completely, and which two independent reference renderers
/// contradict: pages whose raster is one pixel smaller than the references'.
///
/// **Empty as of the four-hundred-and-forty-third session, and the emptying is a correction
/// rather than a fix: this tree's raster was never the smaller one.** `TargetSpec::for_page`
/// rounds a fractional page *up* so that the raster contains it — the ledger's §10.7.4 row has
/// said so since the sixty-first session — so on both of the two pages left here our own render
/// is the *larger* size, the same size `poppler` and `mupdf` produce, and `ghostscript` is the
/// renderer that truncates. Rendered straight through `examples/render_at` at scale 1:
/// `colorkeymask.pdf` is **596 × 842** where this comment said 595, and `issue21346.pdf` is
/// **179 × 179** where it said 178.
///
/// # What was misread was an artefact, and the artefact is doing what it is documented to do
///
/// `<stem>-p<n>-ours.png` under `<target>/tmp/oracle/` is our raster **after
/// `normalise::to_common_size` has cropped it to the smallest of the three voting references**,
/// which on both of these pages is `ghostscript`'s. The reference PNGs beside it are the
/// *cache's* renders and are not cropped, so a directory listing shows ours at 595 or 178 next
/// to a `poppler` at 596 or 179 and reads exactly like this tree rounding down. Both files were
/// re-derived: our own render cropped to the reference's size is byte-identical to the artefact
/// (`magick compare -metric AE` = 0 on both).
///
/// **So the rule is the one trap 1 states one directory over, arriving in the instrument rather
/// than in a count: `-ours.png` is our raster reconciled with somebody else's page size, and the
/// only place our page size can be read is a render of our own.** `report::write_artefacts`
/// carries the same sentence where the file is written.
///
/// Both pages were then diagnosed by what they actually differ by, one clause apart:
/// `issue21346.pdf` went to [`CONTRADICTED_COINCIDENT_CLIP_EDGES`] — which it has since left
/// altogether, agreeing since ADR 0476 measured a rectangular clip's edge at its own coverage —
/// and `colorkeymask.pdf` to [`CONTRADICTED_IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE`]. That is the tenth
/// and eleventh time a group's name in this file has named a hypothesis rather than a diagnosis,
/// and the first time the hypothesis was contradicted by the harness's own output.
///
/// The paragraphs below are what the group said while it had members, kept because the history
/// is what makes the correction legible.
///
/// 4 pages. Each has a page box whose size is fractional, and at 72 dpi we and `ghostscript`
/// produce a raster of one size while `poppler` and `mupdf` produce one a pixel wider, taller,
/// or both. Nothing in ISO 32000-2 says how a fractional page becomes an integer number of
/// pixels; it is a rasterisation choice and all four are defensible.
///
/// It only reaches this list because the pages are small — 72 rows, 62 rows — so a one-row
/// shift moves everything on them and the structural-similarity bound sees a page-wide
/// change. On a 792-row page the same difference disappears into the noise.
///
/// # Half of this group was our defect, and the group's own name is what hid it
///
/// It held 8 pages until the sixty-first session, and 4 of them left when one line changed:
/// the y flip translated by the *raster's* height rather than by the page's own, so on a page
/// whose height is not a whole number of pixels every mark sat a fraction of a row too low
/// (ADR 0064). `bug1065245.pdf`, `bug1922766.pdf`, `bug1934157.pdf` and `issue12963.pdf`
/// page 6 all agree now, and none of them was ever about which way anybody rounded.
///
/// The tell was there to be read and nobody read it: on `issue3694_reduced.pdf` — which was
/// in `CONTRADICTED_UNEXPLAINED`, not here — `ghostscript`'s raster is *the same size as
/// ours*, 273x56, and its content sat one row above ours. A rounding difference cannot do
/// that. **A group whose name explains the page size will keep explaining it after the page
/// size has stopped being the difference**, which is the ninth time a group's name has failed
/// to diagnose one of its members, and the first where the name was true of every member and
/// causal for only half.
///
/// # How these lists work
///
/// Named rather than counted, and checked for *equality* rather than as an upper bound: a
/// page that starts disagreeing fails the gate even if another was fixed the same day, and
/// a page that is fixed must be deleted from its list rather than left to rot. That is what
/// "a fixed one can never come back" requires.
///
/// A page, not a document — the same file can be right on one page and wrong on the next,
/// and a page-one-only comparison would never have looked at several of the entries below.
///
/// The grouping is by what the page *carries*, which is a hypothesis about the cause and
/// not a diagnosis. A page here may differ for some quite other reason, and only the
/// artefacts settle it. What every entry does establish is that two implementations sharing
/// no code agree about this page and we do not.
///
/// **And two more left in the two-hundred-and-fifth session, for the same reason one page over.**
/// `bug1669097.pdf` and `issue19505.pdf` were filed here on the page-size split and both agree
/// now that a redundant `/BBox` clip is taken back off an annotation's appearance — they are
/// widget borders, and the clip was eating a fifth of every stroke's coverage (ADR 0165). **Six
/// of this group's eight members have now left it for a reason other than its name**, which is
/// the strongest statement this file makes about what a group name is worth: it says what the
/// pages have in common, and twice running that has not been what they differ by.
///
/// The two that remain are the ones whose story is still only about rounding.
/// `colorkeymask.pdf` became comparable when §8.9.6.4's colour key masking landed and is the
/// 595-against-596 split, ours and `ghostscript`'s against `poppler`'s and `mupdf`'s; its
/// difference is three vertical lines one pixel wide at the three band edges of a page whose
/// only content is two coloured bands. `issue21346.pdf` is 178x178 where `poppler`'s and
/// `mupdf`'s is 179, its colour is identical to `poppler`'s, `ghostscript`'s and `hayro`'s at
/// every point sampled — mean 0.70, worst tile 0.96, both inside their bounds — and what fails
/// is structural similarity, 0.9830 against 0.9900, which is what a one-pixel edge does to a
/// page that is one flat square.
///
/// (Neither sentence survives the paragraph at the top: both rasters are ours at the *larger*
/// size, the sampled colours were never in dispute on either page, and the numbers quoted for
/// `issue21346.pdf` were `ghostscript`'s, which session 406 corrected to mean 0.25 and
/// similarity 0.9734 against the pair that decides it.)
///
/// **`french_diacritics.pdf` left this group in the sixteenth session and not by being rounded
/// differently.** It agrees because area averaging replaced the four-tap filter that was
/// drawing its reduced inline images (ADR 0025) — worst tile 12.60 against a bound of 5.89
/// before, inside the bound after. Its raster really is 595x842 against `poppler`'s and
/// `mupdf`'s 596, which is what put it here; that was true and was not what the references
/// were disagreeing about.
const CONTRADICTED_PAGE_ROUNDING: [&str; 0] = [];

/// Contradicted, where every clip boundary the page states falls on the same device edge.
///
/// 1 page, moved out of [`CONTRADICTED_PAGE_ROUNDING`] in the four-hundred-and-forty-third
/// session. `issue21346.pdf` is 178.34645 points square and holds one mark: a 150 × 150 square
/// of `(0.227, 0.498, 0.690)` painted at a mask value of 0.25 over white. The interior is not in
/// dispute — the closed form is `0.25 c + 0.75` per channel, which is **(206, 223, 235)**, and
/// ours, `poppler`'s, `ghostscript`'s and `hayro`'s centre pixel is that byte for byte while
/// `mupdf` is one level up on each. What differs is the square's one-pixel border.
///
/// # The boundaries, counted
///
/// Every construction on the page states the same device rectangle, device `[14.173, 164.173]`
/// on both axes:
///
/// | | what states it |
/// |---|---|
/// | the page's `W n` | `14.173228 164.17322 m …` under the page's own matrix |
/// | form `15`'s `/BBox [0 0 200 200]` | §8.10.1 step c), under `0.75 0 0 -0.75 14.173228 164.17322 cm` |
/// | form `13`'s `/BBox [0 0 200 200]` | the same clause, under the same matrix |
/// | form `13`'s fill | `0 0 m 200 0 l 200 200 l 0 200 l h f` |
/// | the mask group `14`'s `/BBox` | §11.6.5.2's group, under the same matrix |
/// | the mask group's own fill | the same four lines |
///
/// `examples/clip_chain_census` says the first three outright — *clip references 3, distinct
/// leaves 2, distinct clip nodes 3, chain depth histogram {2: 1, 3: 1}*.
///
/// # What multiplying them cost, as a ladder this project wrote
///
/// A synthetic A/B: the same 178.34645-point page, the whole page filled, under **n** `W n` clips
/// of that rectangle and nothing else. Rendered at 8× through `examples/render_at`, the coverage
/// of the boundary column (device 113, where the clip's left edge lands at 113.386):
///
/// ```text
///   coincident boundaries      1       2       3       4       5       6
///   before session 444     0.5020  0.2510  0.1255  0.0627  0.0314  0.0157
///   since                  0.5020  0.5020  0.5020  0.5020  0.5020  0.5020
/// ```
///
/// Each rung used to be the one above it halved, because the coverages were **multiplied**. Three
/// of this page's six statements are a clip *chain*, and `render-cpu` composes a chain with `min`
/// since the four-hundred-and-forty-fourth session (ADR 0280) — a set intersected with itself is
/// that set — so the ladder is flat and the page's edge went **0.041 → 0.163** of the mark, level
/// 253 to level 247 against an interior of 206.
///
/// # The clause, which this file had never cited
///
/// §10.7.4 gives clipping a paragraph of its own, and it is about sets rather than about
/// coverage:
///
/// > For clipping, the clipping region consists of the set of pixels that would be included by a
/// > fill operation. Subsequent painting operations shall affect a region that is the
/// > intersection of the set of pixels defined by the clipping region with the set of pixels for
/// > the region to be painted.
///
/// A pixel the clip's fill would include is in the region, whole, however little of it the path
/// covers — so the clause paints this edge at **1.000**, which is exactly what `poppler` and
/// `ghostscript` do (both give 206, the interior colour, at device column 14). `mupdf` gives
/// 0.755 and `hayro` 0.327, both of them anti-aliasing the clip and conflating it fewer times
/// than we do. This tree's documented departure (1) from that subclause — an anti-aliasing
/// rasteriser paints a partly covered pixel partly — would give **0.827**.
///
/// # Why the page is still here after the composition changed
///
/// `min` is what a set intersection asks for, and the four-hundred-and-forty-fourth session took
/// it for the one composition that is this tree's: a clip *chain*, in `MaskCache::build`. Three of
/// this page's six statements are the chain the mark draws under — the `W n` and the two `/BBox`
/// clips — so composing them removes **two** factors and the edge went **0.041 → 0.163** of the
/// mark, exactly the fourfold this ladder's `0.5020` predicts. The structural-similarity bound this
/// page fails went 0.9734 → **0.9781** against a bound of 0.9900, so the verdict is unmoved, which
/// is what ADR 0279 predicted before the change and ADR 0280 measured after it.
///
/// What is left is three factors and only one of them is a product the standard states. Two are the
/// *same* sentence as the chain — a mark's own coverage meets the clip mask inside `tiny-skia`'s
/// `fill_path`, which multiplies, once for form 13's fill under the chain and once for the mask
/// group's fill under its own `/BBox` — and reaching them means this backend rasterising coverage
/// into a buffer of its own instead of handing the mask to the library. The third, the mask's value
/// multiplying the mark, is §11.6.5's alpha and the standard says multiply.
/// [`doc/todo/11`](../../../doc/todo/11-shapes-that-still-disappear.md) carries what is owed.
///
/// # The page left, and what moved was how finely each factor was measured
///
/// Every one of the seven statements tabulated above is the **same axis-aligned device
/// rectangle**, and this backend measured each of them with `tiny-skia`'s supersampled *path*
/// converter — whose answer at an axis-aligned edge is a quarter of a pixel, because all four of
/// its sub-rows see the same run. This page's boundary falls at device 14.173, so the edge's own
/// coverage is **0.827** and the quantum gave **0.75**: every surviving factor short by one ratio.
///
/// ADR 0476 gives a rectangular fill *and* a rectangular clip region the coverage §10.7.4's own
/// definition of a pixel implies — the region "consists of the set of pixels that would be
/// included by a fill operation", so the two are one rule — and the mark's edge went
/// **0.306 → 0.469** of the mark on both axes, level 240 to level 232 against an interior of 206.
/// Both numbers are `examples/render_at` at scale 1, either side of the change in one sitting.
/// That crossed the bound and the page agrees.
///
/// **It does not mean item 4 is paid**, and the arithmetic says so rather than a hope: departure
/// (1)'s answer for a single anti-aliased boundary is 0.827 and the clause's is 1.000, and the two
/// edges stand in the ratio `(0.75/0.827)^4.4` — so between four and five of the seven statements
/// are still *multiplied* where §10.7.4 intersects sets. What this changed is what each surviving
/// factor is worth, not how many there are.
const CONTRADICTED_COINCIDENT_CLIP_EDGES: [&str; 0] = [];

/// Contradicted, and **we are the ones who are right**: an image sample at the pixel's centre.
///
/// **Empty since the seven-hundred-and-twenty-ninth session, and the page is still `colorkeymask.pdf`'s.**
/// It is `ambiguous` now, in [`AMBIGUOUS_DIVIDED_CONSENSUS`], because the references reached two
/// readings of it and only one of them contradicts us (ADR 0617). Everything below is the reading
/// of the page and none of it is a reading of the verdict, so it stays here rather than moving:
/// what §10.7.4 says about a one-to-one image placement does not depend on which pair the gate
/// took, and the last section is what that pair turned out to be.
///
/// 1 page until then, moved out of [`CONTRADICTED_PAGE_ROUNDING`] in the four-hundred-and-forty-third
/// session, where it had been filed since the sixth on a raster size that is not ours.
///
/// `colorkeymask.pdf` is nine commands: a §8.7.3 tiling pattern whose cell draws one
/// 200 × 267 `/DeviceRGB` image with `/Mask [255 255 0 255 0 255]` (§8.9.6.4's colour key
/// masking), under `200 0 0 267 0 0 cm` with the pattern's `/Matrix [1 0 0 1 18 557]`. So the
/// image covers device x `[18, 218)` and y `[17.9998, 284.9998)` — **one device pixel per
/// source sample, exactly**, with no reduction and no `/Interpolate`.
///
/// # The measurement
///
/// Ours and `ghostscript` are **byte-identical over the whole 595 × 842 raster**
/// (`magick compare -metric AE` = 0), and `poppler` — which votes with `mupdf` — differs from
/// both on **942 pixels of 500 990**, 0.19%. They are not scattered: 268 apiece in device
/// columns 78, 138 and 218, and 141 in device row 17.
///
/// # §10.7.4's image paragraph decides all four of them, and it decides for us
///
/// > However, only those pixels whose centres lie within the region shall be painted. The
/// > position of the centre of such a pixel -in other words, the point whose coordinate values
/// > have fractional parts of one-half -shall be mapped back into source space to determine how
/// > to colour the pixel. There shall not be averaging over the pixel area.
///
/// - **Device row 17** has its centre at y 17.5, which is outside `[17.9998, 284.9998)`. The
///   clause paints nothing there and we paint nothing there; `poppler` paints it. Rows 18 to 284
///   are 267 rows for 267 samples, which is the arithmetic saying the row we decline is the one
///   the clause declines.
/// - **Device column 78** has its centre at x 78.5, which maps back to source x 60.5 — sample
///   **60**. The image's row 110 is `(255, 0, 0)` at samples 58 and 59 and `(0, 255, 0)` at 60
///   and 61, read out of the file's own uncompressed bytes. Ours paints `(0, 255, 0)`, the
///   sample whose region holds the centre. `poppler` paints `(130, 201, 77)`, which is neither
///   sample. Columns 138 and 218 are the image's other two colour boundaries and behave the
///   same way.
///
/// So the difference is the sentence "[t]here shall not be averaging over the pixel area",
/// against a consensus that averages at a one-to-one placement. This is *not* ADR 0025's
/// departure seen from the other side: that departure averages the samples that share a device
/// pixel when an image is **reduced**, and one sample per pixel is the case where it has nothing
/// to average. It is the clause carried out.
///
/// The page fails one bound and it is the worst tile, 5.03 against 5.00 — 942 pixels at up to
/// 255 levels, gathered into three one-pixel columns, is what a tile maximum is for. The ink
/// agrees to **0.03 of 255** across all five renderers (ours 12.4099, `ghostscript` 12.4099,
/// `poppler` 12.4021, `mupdf` 12.4355, `hayro` 12.4195), which is why nothing else notices.
///
/// Listed rather than chased, and this is the group's whole point: a contradicted page is a
/// question for the specification, and here the specification answers against the two renderers
/// that agree. [`CONTRADICTED_VISIBILITY_EXPRESSION`] is the other entry of this shape and
/// `AMBIGUOUS_IMAGE_REDUCTION` is where the same paragraph goes the other way.
///
/// # And the two renderers that agree are not the only two that agree
///
/// The sentence above says "`poppler` — which votes with `mupdf`" and that is the whole of what it
/// says about the pair. **`ghostscript` and `mupdf` also agree on this page**, within every class
/// bound, and neither of those two pairs contains the other: `poppler` and `ghostscript` are what
/// differ. So the page carries two maximal consensuses, `pdfref::decide` took the one whose
/// subset bitmask is smaller, and this page's verdict was that choice — the rival pair contains a
/// renderer our raster is **byte-identical to**, so it accepts us (the seven-hundred-and-twenty-seventh
/// session, ADR 0616).
///
/// **It changes no line of the reading above and it changes what the verdict is worth.** Every
/// number in this note was measured against the file and the clause rather than against the pair,
/// which is why none of them moves; what moves is that "the specification answers against the two
/// renderers that agree" now has to say *which* two, and the other two answer with the
/// specification and with us.
///
/// # And the rule that replaced the enumeration order took the page off this list
///
/// A verdict is one **every** maximal consensus reaches (ADR 0617), and the two here do not
/// concur, so the page is `ambiguous` and named in [`AMBIGUOUS_DIVIDED_CONSENSUS`]. The control
/// that settles it is one line of arithmetic already above: our raster *is* `ghostscript`'s, so
/// the worst tile of 5.03 that contradicts us is `ghostscript`'s distance from `poppler` — the
/// set that decided this page contradicts a voting reference, in the same numbers, while the
/// other set accepts them both. Nothing in the clause reading changes, including the part this
/// project cares about most: the specification answers this page, and it answers for us.
const CONTRADICTED_IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE: [&str; 0] = [];

/// Contradicted, where the difference was said to be a *spectrum of edge softness*.
///
/// **Empty since the six-hundred-and-forty-third session, and the emptying is a correction.** The
/// group held `colors.pdf` pages 1 and 2 — grids of flat colour swatches whose interiors every
/// renderer agrees about to the byte — on this sentence: "the five sit on a spectrum of edge
/// softness with `poppler` at one end, and the pair the gate votes with is the pair nearest that
/// end … it is the departure being visible". Two claims, and the measurement disproves the first
/// and re-attributes the second. The pages are [`CONTRADICTED_TIGHT_CONSENSUS`]'s mechanism and
/// have moved there. **Twelfth for twelve on a group's name naming a hypothesis rather than a
/// diagnosis.**
///
/// # The instrument is the page's own arithmetic, and no renderer is in it
///
/// Each page is sixteen axis-aligned rectangles under `0.001968504 0 0 0.001968504 0 0 cm`, so
/// every boundary is at a *known* fraction of a device pixel: the column boundaries land at
/// device x 198.4252 and 396.8504, which is `100800/508` and `201600/508`. A rectangle's coverage
/// of a pixel is therefore a product of two one-dimensional overlaps, and compositing the sixteen
/// in the order the content stream states them gives the page, pixel by pixel, out of the file
/// alone. Two forms were written out and rendered to PNG: one using the **exact** overlap, one
/// using the overlap `tiny-skia` measures — four samples per axis at 0.125, 0.375, 0.625 and
/// 0.875, so a quarter of a pixel is its quantum. Against the oracle's own artefacts, by
/// `raster_compare`:
///
/// | | vs the exact form | vs the quarter-quantised form |
/// |---|---|---|
/// | ours | mean 0.0406, max **33** | mean 0.0023, max **1**, ssim 1.00000 |
/// | `hayro` | mean 0.0015, max **2**, ssim 1.00000 | mean 0.0545, max 33 |
/// | `mupdf` | mean 0.0173, max 13 | mean 0.0526, max 25 |
/// | `ghostscript` | mean 0.1368, max 54 | mean 0.1791, max 64 |
/// | `poppler` | mean 0.2823, max 124 | mean 0.3313, max 130 |
///
/// **Our raster is the quantised closed form to one level of 255 over 595 × 841 pixels**, and
/// `hayro`'s is the exact one to two. So the five are not a spectrum of softness with us at the
/// soft end. Ranked by distance from the geometry at the worst pixel — `hayro` 2, `mupdf` 13, ours
/// **33**, `ghostscript` 54, `poppler` 124 — two of them paint the area the shape covers, `poppler`
/// paints whole pixels because it does not anti-alias an axis-aligned edge at all, `ghostscript`
/// supersamples and filters, and ours is the geometry with each edge's coverage **rounded to a
/// quarter**: up in most places, down in some, and to nothing wherever an edge covers less than an
/// eighth of its pixel. **Third of five, not the outlier.** Page 2 reproduces it: ours to the
/// quantised form mean 0.0022 and max 1, `hayro` to the exact form mean 0.0017 and max 2.
///
/// `render-quorra/examples/edge_coverage_ladder` is the same finding without a document, and it
/// says which backend owns it: at a rectangle edge placed every twentieth of a pixel, `render-cpu`
/// answered 0, 0.2510, 0.5020, 0.7529 and 1.0000 on both axes while the graphics device tracks the
/// fraction to a level of 255. ADR 0474 has the price and `doc/todo/11` item 7 what a cure costs.
///
/// **Every past tense above is the six-hundred-and-forty-sixth session's** (ADR 0476): an
/// axis-aligned rectangle's coverage is the product of its two overlaps, which §10.7.4's own
/// definition of a pixel gives, and `render-cpu` draws a rectangular fill and a rectangular clip
/// region at it. Our raster of these two pages **is** the exact closed form now, and both backends
/// read the ladder to a level of 255.
///
/// # Why the pages still moved to a *bound* group rather than to a defect
///
/// Because the exact form is contradicted here too, which is the one thing this note could not
/// have assumed. The bound the gate applies is twice the consensus pair's own distance, and the
/// pair is `poppler` and `ghostscript`: page 1's bound is ssim 0.98862 and page 2's 0.98402. A
/// rasteriser painting precisely the area each rectangle covers is contradicted on both, so the
/// verdict is trap 12's rather than a report about our marks.
///
/// **The paragraph above said until the six-hundred-and-sixty-fifth session that it was
/// "unaffected — which it predicted", and it was affected in the one place the prediction was
/// checkable.** It went on to give ours as ssim 0.98591 and 0.97906 against an exact form at
/// 0.98772 and 0.98001, and those are the *quarter-quantised* raster's numbers: ADR 0476 made ours
/// the exact form, so the two rows stopped being two things and neither figure survived the change
/// the sentence above it announces. The gate prints, on this tree, `ours at worst … ssim 0.9879`
/// against `bound … ssim 0.9886` for page 1 and `0.9802` against `0.9840` for page 2 — which is
/// the exact form's place in the ranking and not the quantised one's, and ADR 0489 re-derived both
/// closed forms independently and put ours 0.0000% of either page from the exact one. **The 33
/// levels this paragraph used to call "ours" are paid.**
///
/// The correction is `--bin overtaken`'s first finding (ADR 0491), and the reason the note could hold
/// it for
/// nineteen sessions is the sweep's whole subject: ADR 0476 rewrote the paragraph above and left
/// the paragraph below, and no gate reads a note against the decisions taken after it.
const CONTRADICTED_ANTIALIASED_EDGES: [&str; 0] = [];

/// Contradicted, where the difference was said to be a `CalRGB` space converted rather than
/// assumed.
///
/// **Empty since the five-hundred-and-fourteenth session, and the emptying is a correction.** The
/// group held `issue9940.pdf` page 1 on this sentence: "we and `poppler` convert a `CalRGB` through
/// CIE XYZ as §8.6.5.3 defines it, while `mupdf` and `ghostscript` take its components for
/// `DeviceRGB`". ADR 0296 flagged it as the one unmeasured sentence in the neighbourhood, because
/// the same claim had just been disproved on `calrgb.pdf`. Measured, it is disproved here too:
/// **nobody takes the components for `DeviceRGB`.** The page is
/// [`CONTRADICTED_CALRGB_TO_SCREEN`]'s mechanism and has moved there. **Tenth for ten on a group's
/// name naming a hypothesis rather than a diagnosis.**
///
/// # The instrument, which needed no page of the corpus
///
/// The page draws its cover art through
/// `[/Indexed [/DeviceN [/IBM /None /None /None] [/CalRGB …] tint] 255 table]`, so the claim is
/// about one link of a four-link chain and cannot be read off the page. It can be asked directly:
/// a 100 × 100 fixture filling itself with `0.5 0.25 0.75 sc` in *this file's own* `/CalRGB`
/// dictionary — `/WhitePoint [0.9505 1 1.089]`, `/Gamma [2.20003 …]`, `/Matrix [0.9505 0.00002 0
/// −0.00002 1 0 0 0.00002 1.08899]` — put to all four renderers. The two readings are arithmetic
/// rather than anybody's output: §8.6.5.3's decoding gives `X Y Z = (0.20686, 0.04737, 0.57830)`,
/// and IEC 61966-2-1's XYZ → sRGB transform on it gives **(151, 0, 205)**, while taking the
/// components for `DeviceRGB` gives **(128, 64, 191)**.
///
/// | | centre pixel |
/// |---|---|
/// | **the closed form, §8.6.5.3 + XYZ → sRGB** | **(151, 0, 205)** |
/// | ours | (151, 0, 205) |
/// | `poppler` | (151, 0, 205) |
/// | `mupdf` | (166, 0, 205) |
/// | `ghostscript` | (166, 0, 207) |
/// | **the closed form, components as `DeviceRGB`** | **(128, 64, 191)** |
///
/// Ours and `poppler`'s are the closed form exactly. `mupdf` and `ghostscript` are 15 levels away
/// **in red alone** — and the `DeviceRGB` reading would have moved all three channels, by −23, +64
/// and −14. So the sentence named a mechanism whose signature is not the one the page has.
///
/// The page then says the same thing, over 484 704 pixels rather than one: the per-channel means of
/// the five panels are `R − G` of −2.05 for ours, `poppler` and `hayro` and **+2.02 and +1.72** for
/// `mupdf` and `ghostscript`, with the green and blue means agreeing across all five to 0.6 of 255.
/// One channel moves. That is the swatch's shape at page scale, and it is why the page looks pinker
/// in two of the five panels.
///
/// **What the specification determines** is [`CONTRADICTED_CALRGB_TO_SCREEN`]'s reading and is not
/// repeated here: §8.6.5.3 defines components-to-XYZ exactly and every renderer agrees there, and
/// §10.3.1 puts the rest — "[t]he specific method by which the CIE-based destination colour space
/// is established is beyond the scope of this document" — outside the standard.
///
/// The `None` colourants are not the cause, though they look like one. §8.6.6.5 is explicit:
/// "when the DeviceN colour space reverts to its alternate colour space, those components
/// shall be passed to the tint transformation function", which is what happens here — the
/// space never reaches a device colourant, so it always reverts.
const CONTRADICTED_CALIBRATED_COLOUR: [&str; 0] = [];

/// Contradicted, where the two references that agree are one house's colour pipeline.
///
/// Four pages of `calrgb.pdf`, which are **one page four times**, and one of `issue9940.pdf`, which
/// arrived by the same route one group over. The four spent four hundred and fifty-five sessions
/// in [`CONTRADICTED_SUBSTITUTED_FONT`] on that group's membership rule — the page names a font
/// nobody embedded — under a two-sentence note calling them "a residue of colour management rather
/// than of fonts" with no number behind it. The four-hundred-and-sixty-first session measured them.
/// **Ninth for nine on a group's name naming a hypothesis rather than a diagnosis** — and the fifth
/// member made it ten for ten, which [`CONTRADICTED_CALIBRATED_COLOUR`] records because that is the
/// group whose name was wrong.
///
/// # This entry was chosen for what it never asked, and the title above is the answer
///
/// A contradicted verdict makes two claims. One is that the *standard* rather than the consensus
/// decides the page, which the six-hundred-and-sixty-second session audited by counting each
/// group's clause citations. The other is ADR 0005's premise — that **the agreement which outvotes
/// us is evidence** — and nothing had audited that at all. Asked of every non-empty
/// `CONTRADICTED_*` list, *does the note name a mechanism for the two voting references agreeing,
/// and is the mechanism verified rather than asserted?*, the fourteen split ten, three and one:
/// ten name one and check it against a binary, a source file, a data file, a log or a ladder;
/// three name one and infer it from the picture ([`CONTRADICTED_IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE`],
/// [`CONTRADICTED_SUBPIXEL_IMAGE`], [`CONTRADICTED_REFERENCE_GLYPH_WIDTHS`]); and **this one named
/// none**, writing instead that the two "happen to agree to 4.41%". A verdict resting on an
/// agreement nobody has explained is trap 9's whole subject stated as a coincidence.
///
/// It is explained now, and the explanation is a file that exists on no disk until a renderer
/// makes it.
///
/// # The four pages differ from each other in one entry, and four renderers ignore it
///
/// `calrgb.pdf` is 850 × 1100 points, so the oracle's raster is one pixel per point, and each page
/// states its own space in a header above a grid of eighty swatches labelled with the `A, B, C`
/// that produced them. Pages 1, 5, 11 and 12 state the *same* space in three of Table 63's four
/// entries — `/WhitePoint [1 1 1]`, `/Gamma [1 1 1]`, the identity `/Matrix` — and differ only in
/// the fourth:
///
/// | page | `/BlackPoint` |
/// |---|---|
/// | 1 | `[0 0 0]`, which is the table's default |
/// | 5 | `[1 1 1]`, the white point itself |
/// | 11 | `[8 8 8]` |
/// | 12 | `[50 50 50]` |
///
/// **Below the header, each of our raster, `poppler`'s, `mupdf`'s and `ghostscript`'s is
/// byte-identical across all four pages** — device rows 150 to 1090, `md5` of the raw RGB. Four
/// renderers read the entry and none of them lets it move a colour. `hayro` is the only one that
/// does, and it does not vote, which is why it is the only panel that changes: **four different
/// digests over the same four pages**, mean 0.90 of 255 from us on page 1 and 12.47 on page 12.
/// (This entry recorded 16.54 for page 12; the gate's own arithmetic through
/// `examples/compare_rasters` now prints 12.47. A number a note carries that the gate also
/// computes is trap 1's cheapest tell, and it is worth re-running rather than copying.)
///
/// So the gate is printing one measurement four times, and it says so itself — mean 1.38, worst
/// tile 14.16, differing 11.23%, similarity 0.9908 on pages 1, 5 and 11 alike, with page 12's
/// worst tile the only figure that moves, to 13.86. **That one figure is the header, not the
/// page**: the worst 32-pixel tile against `ghostscript` sits at (192, 0) on all four, which is the
/// line printing the `/BlackPoint` values, and page 12 prints `[50.00000 …]` where page 1 prints
/// `[0.00000 …]`. So on this page the worst tile is measuring the label font and the differing
/// fraction is measuring the swatches — and only the second decides the verdict.
///
/// **And these four are the pages on which §8.6.5.3 has nothing left to decide.** Of the
/// seventeen spaces this document states, the four listed here are the only ones whose `/Gamma`,
/// `/Matrix` *and* `/WhitePoint` are all the identity or `[1 1 1]`, so the subclause's decode is
/// the identity and the space **is** XYZ. Every one of the pages where `/Gamma` or `/Matrix` does
/// real work is `ambiguous` or agrees. Whatever separates the renderers here is downstream of
/// everything §8.6.5.3 defines, which is a fact about the file rather than an inference.
///
/// # It is not the font, and the instrument is the swatch interiors
///
/// 76.6% of the page is flat in all five renderers — a pixel whose 7 × 7 neighbourhood is one
/// colour in every one of them — and that region contains no glyph at all. Splitting the
/// difference across it (the four-hundred-and-sixty-first session's measurement, and consistent
/// with this session's swatch sampling, where `poppler` is at most one level from us on all
/// eighty):
///
/// | | mean over the flat region | share of the page's total difference falling inside it |
/// |---|---|---|
/// | `poppler` | **0.004** of 255 | 0.5% |
/// | `mupdf` | 1.677 | 67.0% |
/// | `ghostscript` | 1.362 | 56.6% |
///
/// **Against `poppler`, not one channel of the swatch interiors moves by more than four levels**,
/// and `poppler` substitutes a *different* serif face from ours — this sheet's labels are
/// `/Times-Roman` with no `/FontFile`, ours is `FoxitSerif` (ADR 0133) and the C references resolve
/// `NimbusRoman` through fontconfig. Two renderers with different faces agreeing to 0.004 of 255
/// over three quarters of the page is the substitution costing nothing measurable, which is ADR
/// 0267's finding for the serif family arriving on a fifth document. Against the pair that decides
/// the verdict, two thirds of the difference is inside swatches where no glyph is drawn.
///
/// # Ranked against the closed form, ours and `poppler` are on it and the pair that votes is not
///
/// Page 1's space is XYZ itself, so each of the eighty swatches has an arithmetic answer that owes
/// nothing to any renderer: Bradford-adapt the stated `/WhitePoint` onto sRGB's own white, apply
/// IEC 61966-2-1's matrix and its transfer function. Written out in a script holding the published
/// constants and none of this crate's code, and compared with each panel at every swatch centre,
/// in levels of 255:
///
/// ```text
///                  mean over the 80 swatches   worst swatch
///   poppler                  0.013                   1
///   ours                     0.025                   1
///   hayro                    2.150                   8
///   ghostscript              4.300                  15
///   mupdf                    4.838                  31
/// ```
///
/// Ours and `poppler` are that arithmetic to **one level of 255 on every swatch of the page**, by
/// two implementations sharing no line of code, and the two that vote are the two furthest from it.
/// That is [`CONTRADICTED_TIGHT_CONSENSUS`]'s shape on a colour page, and it is the ranking rather
/// than the verdict: §10.3.1 leaves the choice of destination open, so being on this form is not by
/// itself conformance. What it does establish is that the camps are a fact about implementations
/// and not a coin toss.
///
/// # And nobody here assumes `DeviceRGB` — which is now checkable, because `ghostscript` will
///
/// A processor taking the components for `DeviceRGB` would paint `0.75 0.00 0.00` as `(191, 0, 0)`
/// and `0.50 0.50 0.50` as `(128, 128, 128)`. Read off the five rasters at those swatch centres:
///
/// | | ours | `poppler` | `mupdf` | `ghostscript` | `hayro` |
/// |---|---|---|---|---|---|
/// | `0.75 0.00 0.00` | 255, 0, 62 | 255, 0, 62 | 255, 0, 65 | 255, 0, 66 | 255, 0, 60 |
/// | `0.01 0.00 0.00` | 50, 0, 2 | 50, 0, 2 | 19, 0, 2 | 35, 0, 2 | 49, 0, 2 |
/// | `0.50 0.50 0.50` | 188, 188, 187 | 188, 188, 188 | 193, 187, 188 | 196, 187, 188 | 194, 184, 188 |
///
/// All five convert; none assumes. **The reading is producible rather than hypothetical**:
/// `gs -dUseFastColor=true` turns ghostscript's colour management off, and it paints exactly that
/// — 3 for `0.01`, 127 for `0.50 0.50 0.50`, the component times 255 with no transfer function at
/// all — which is a mean of **75.51 of 255 and a worst pixel of 173** from `ghostscript`'s own
/// default rendering of the same file. One program, asked both questions, answering them 173 levels
/// apart.
///
/// # Why the two that vote agree: each turns Table 63 into an ICC profile, and they are one house
///
/// `objdump -p`, `nm -D` and `strings -a` on this machine's three references — what a binary asks
/// for, what it exports and what it carries, rather than a family resemblance:
///
/// - **`ghostscript`** carries `gsicc_create_from_cal` and `./base/gsicc_create.c` among its own
///   internal names, alongside the parameters `CalRGBProfile` and `CalGrayProfile`. (Strings, not
///   symbols: `libgs.so.10` exports 242 dynamic symbols and no `gsicc_*` is among them.) It lists
///   `liblcms2.so.2` as `NEEDED` and leaves 22 `cms*` undefined.
/// - **`mupdf`** *exports* `fz_new_cal_rgb_colorspace` and `fz_new_icc_data_from_cal`, and carries
///   the message *CalRGB profile creation failed; bad values*. It asks for no colour library at all
///   and **defines 437 `lcms2mt_*` symbols**, which is Artifex's fork of Little CMS compiled in.
/// - **`poppler`** has its own `GfxCalRGBColorSpace` — the typeinfo name is in the binary — and
///   exports `make_GfxLCMSProfilePtr`, which is the entry to `liblcms2` for an `ICCBased` stream
///   and not for this space.
///
/// So the split on this page is two renderers evaluating §8.6.5.3 in their own code against two
/// that **synthesise an ICC profile from the dictionary first**, and the second two are one house's
/// two programs running two builds of one CMM. Trap 9's first shape, on a colour clause where
/// nobody had looked for it.
///
/// # The profile, obtained, and this tree pointed at it
///
/// `gs -sDEVICE=pdfwrite` writes the page back out with the `/CalRGB` array replaced by an
/// `ICCBased` stream, so `gsicc_create_from_cal`'s output is **585 bytes on disk** and can be read:
/// a version 4.2 `scnr` profile, RGB to XYZ, `wtpt` the D50 connection white, `rTRC`/`gTRC`/`bTRC`
/// each a `curv` of gamma 1.0, and colorants
///
/// ```text
///   rXYZ = [1.006409, 0.000000, 0.000000]
///   gXYZ = [0.000000, 1.019592, 0.000000]
///   bXYZ = [0.000000, 0.000000, 0.827927]
/// ```
///
/// A Bradford adaptation of the identity `/Matrix` from `/WhitePoint [1 1 1]` onto the D50 connection
/// space is *not* diagonal — it is `[0.997781, −0.009757, −0.007429]`, `[−0.004152, 1.018325,
/// 0.013463]`, `[−0.029429, −0.008567, 0.818866]`, whose columns sum to D50 exactly, which is what
/// "the white point maps to the white point" means. The synthesised profile keeps three numbers and
/// drops the cross terms, and its three colorants sum to `(1.0064, 1.0196, 0.8279)` against its own
/// `wtpt` of `(0.9642, 1.0, 0.8249)` — **4.4% adrift in X**.
///
/// **The instrument is ADR 0048's, and the answer is one file.** Rendering that rewritten page — the
/// same eighty swatches, now stating ghostscript's own profile — and comparing each renderer with
/// its `/CalRGB` answer, in levels of 255 over the 72 swatches outside the deepest shadow:
///
/// ```text
///                                                        mean    worst
///   ours, /CalRGB              vs ghostscript, /CalRGB    4.15      11
///   ours, ghostscript's profile vs ghostscript, /CalRGB   0.07       1
/// ```
///
/// **This tree, handed the profile `ghostscript` builds out of Table 63, reproduces
/// `ghostscript`'s rendering of Table 63 to one level of 255.** And asked whether the profile
/// changes each renderer's mind at all — its own `/CalRGB` answer against its own answer to the
/// rewritten page:
///
/// ```text
///   ghostscript   0.03   (max 1)     it was already using this file
///   mupdf         0.83   (max 3)     its own synthesised profile is effectively this one
///   ours          4.17   (max 11)    it moves us onto their answer
///   poppler       4.24   (max 11)    and poppler with us
/// ```
///
/// The two that outvote us do not move when handed the other's profile; the two that agree with us
/// both do. **The whole verdict is that 585-byte file** — and it is a file no dependency graph
/// shows, no digest comparison finds and nothing on this disk contains, because each of the two
/// manufactures it from the document. That is trap 9's eighth mechanism.
///
/// # And where the page differs most, the pair that votes agrees least
///
/// The gate's differing fraction counts channels moving by more than four levels of 255. Over the
/// eighty swatch centres, taken as the maximum channel difference at each:
///
/// ```text
///                                  max   mean   swatches over four
///   ours        <-> poppler          1   0.01     0 of 80
///   mupdf       <-> ghostscript     16   2.35     8 of 80
///   ours        <-> ghostscript     15   4.31    36 of 80
///   ours        <-> mupdf           31   4.85    34 of 80
/// ```
///
/// On the 41 swatches where the camps differ at all, the pair that votes is a mean 3.78 and a
/// **maximum of 16** apart from each other. At `A B C = 0.01 0.00 0.00`, which carries the largest
/// difference on the page, ours is 50, `poppler` 50, `ghostscript` 35 and `mupdf` 19 — so **we are
/// nearer `ghostscript` there (15) than `mupdf` is (16)**, on the swatch the verdict is made of.
/// Handed one profile the three still spread 19 to 54 at that swatch. Their 4.41% over the whole
/// page is an average taken across a sheet three quarters of which no camp disputes, which is trap
/// 12 with the population named: a bound derived from an aggregate is not a bound on the pixels the
/// aggregate is made of.
///
/// # What the specification determines, and this note stopped one sentence short of it
///
/// The entry above read *the difference is the half of the journey §10.3.1 puts beyond itself*, and
/// quoted the sentence that puts it there — the specific method by which the destination space is
/// established being beyond the document's scope, output intents being one way. **The next sentence
/// of the same subclause is a `shall`**, and it says the conversion is not open at all: a CIE-based
/// source colour to a CIE-based destination colour is to be converted based on the appropriate ICC
/// specification. (Prose rather than a quotation because Errata Collection 3's Issue #181,
/// `Review`/`Completed`, strikes that sentence's dated *ISO 15076-1:2010 (ICC.1:2010)* and points at
/// Table 66 instead; `spec-errata emit` files it under §10.4.1's heading and §10.3.1's ledger row
/// carries it.) So the destination is a choice — Artifex's *Artifex Software sRGB ICC Profile* for
/// `ghostscript`, IEC 61966-2-1 for us — and the *route* to it is the referenced standard's, whose
/// media-relative colorimetric intent adapts the source white onto D50 by the transform ICC's own
/// `chad` tag carries. `colour::BRADFORD` cites that and this page is its corpus witness.
///
/// The other half is §8.6.5.3's, and this note had it under the wrong one of its two subjects. The
/// sentence quoted below sits under `/BlackPoint` here and names two entries:
///
/// > The WhitePoint and BlackPoint entries in the colour space dictionary shall control the overall
/// > effect of the CIE-based gamut mapping function described in subclause 10.3, "CIE-Based colour
/// > to device colour".
///
/// On these four pages `/BlackPoint` moves nothing in four renderers and `/WhitePoint` is the whole
/// question, because it is what the adaptation is *from*. Both halves of the `shall` are §10.3's to
/// carry out; only one of them was being read.
///
/// # Two camps of two, and the reference that agrees with us is further out than we are
///
/// Every pair on page 1, in the gate's own units, reproduced through `examples/compare_rasters`:
///
/// ```text
///   ours        <-> poppler        1.62%      <- the closest pair on the page
///   mupdf       <-> ghostscript    4.41%      <- the consensus, so the bound is twice it: 8.82%
///   ours        <-> mupdf         11.12%
///   ours        <-> ghostscript   11.23%      <- the figure the gate prints for us
///   poppler     <-> mupdf         11.21%
///   poppler     <-> ghostscript   11.65%
/// ```
///
/// **`poppler` is further from the consensus pair than we are, on both of its members**, so the
/// verdict "`mupdf` and `ghostscript` agree, we differ" would read identically with `poppler` in
/// our place and by a larger margin. The gate votes with whichever camp's members are nearer each
/// other, and the camp that wins is the one whose two members build the same file.
///
/// `issue9940.pdf` page 1 is the same split without a swatch on it. Its space is
/// `/WhitePoint [0.9505 1 1.089]`, `/Gamma [2.20003 …]` and a near-identity `/Matrix`, reached
/// through an `/Indexed` `/DeviceN` alternate — ADR 0296's parting question, answered *not a
/// different path*. Panel means over the whole page: ours `(240.07, 242.12, 246.34)`, `poppler`
/// `(239.86, 241.91, 246.18)`, `hayro` `(240.11, 242.16, 246.40)` against `mupdf`
/// `(243.56, 241.54, 246.04)` and `ghostscript` `(243.45, 241.74, 246.48)` — three panels one way
/// and the Artifex pair 3.4 levels of red the other, which is the mauve cast the side-by-side shows
/// in one look. Ours to `hayro` is 2.16% differing, closer than either reference pair.
///
/// # And `/BlackPoint` is a decision rather than a gap, which this page is the corpus witness for
///
/// §8.6.5.9 says who decides whether the entry does anything: "[i]f the value is not given or set
/// to `Default`, then the behaviour is left to the PDF processor to determine", which is every
/// document in the corpus. `colour::cie_to_srgb` reads the entry and applies none of it, argued
/// there and in ADR 0012. **What these four pages add is the cost, measured**: a `/BlackPoint` moved
/// from `[0 0 0]` to `[50 50 50]` changes nothing in this tree's raster, in `poppler`'s, in
/// `mupdf`'s or in `ghostscript`'s, so the choice is the one four independent readers of the clause
/// make. `cargo run --release -p pdf-model --example black_point_census` counts how many corpus
/// spaces state one at all.
const CONTRADICTED_CALRGB_TO_SCREEN: [&str; 5] = [
    "calrgb.pdf page 1",
    "calrgb.pdf page 5",
    "calrgb.pdf page 11",
    "calrgb.pdf page 12",
    "issue9940.pdf page 1",
];

/// Contradicted, where the references space the glyphs by no width the document states.
///
/// 1 page. `issue9915_reduced.pdf` shows `ISSUE 9915` in an `OCRB` `CIDFontType0` whose
/// `/Encoding` is an embedded `CMap` with eight `cidrange`s, and whose `/W` is
/// `[32 [719] 0 180 719 181 [878] 182 65534 719]` — every CID 719 except 181.
///
/// # The measurement, which is what settles it
///
/// The `CMap` sends the shown codes to CIDs 1, 38, 42, 52, 54, 18, 22 and 26, every one of
/// them inside `0 180 719`. So one width applies to the whole line, and **our glyph origins
/// are 14.38 pt apart at 20 pt, which is exactly 719/1000** — measured off the display list,
/// not inferred. `ghostscript`'s ink columns are ours to the pixel: 27, 42, 56, 70, 85 against
/// our 28, 42, 56, 70, 85.
///
/// `poppler` and `mupdf` put the five letters at 28, 47, 67, 87, 107 — **20 pt apart, which is
/// the `/DW` default of 1000 that this font does not state** — and then the four digits at 141,
/// 155, 171, 185, about 15 apart, which is 719 again. **Their spacing is not consistent with
/// any single reading of this `/W`**, and ours is exactly one reading of it. §9.7.4.3 makes the
/// array the source of a CID's width, so this is not a tie.
///
/// This entry used to say "our letters sit about 1.39× closer together … which is 1000/719"
/// and left the question open with "[s]omebody is not reading `/W`; the clause says which of us
/// should be." The ratio is 1.20 over the ink span rather than 1.39, the inconsistency inside
/// the references' own line is the fact that decides it, and both took ten minutes with the
/// rasters and the display list. **Measure an entry before believing its label** — including
/// one this project wrote.
///
/// # Column positions are not a bound either, and restating the width settles it
///
/// The measurement above is in ink columns, and the page fails on **mean, differing fraction and
/// structural similarity**, similarity by the widest margin at three times its bound. The
/// conversion is not arithmetic here — a glyph moved four pixels is not an ink figure — so it is
/// done by taking the mechanism out. A §7.5.6 incremental update replaces object 7's
/// `/W [32 [719] 0 180 719 181 [878] 182 65534 719]` with `/DW 719`, which is the width that
/// array assigns to every CID this line uses; §9.7.4.3's default then says the same thing the
/// array said, in the one form nobody can misread. Ours at 72 dpi:
///
/// | | as the file ships | `/DW 719` |
/// |---|---|---|
/// | `poppler` | 13.64 / 31.29 / 8.79% / 0.7020 | 0.7669 / 2.17 / 3.3450% / 0.99677 |
/// | `mupdf` | 13.64 / 31.04 / 8.77% / 0.7032 | 0.6233 / 1.80 / 3.1950% / 0.99772 |
/// | `ghostscript` | **1.7271 / 5.59 / 3.2925% / 0.98274** | **1.7271 / 5.59 / 3.2925% / 0.98274** |
///
/// **The two references that outvote us move onto us; the one that already agreed does not move
/// at all** — `ghostscript`'s four figures are byte for byte what they were, which is the control
/// this experiment needs and gets. A restatement that agrees with a renderer's reading cannot
/// change that renderer's picture, and it changes the other two completely. So the mechanism owns
/// every bound the page fails, and it owns it for the reason the clause gives rather than by
/// coincidence: `poppler` and `mupdf` are not reading a *different* width, they are failing to
/// find this one. ADR 0499.
const CONTRADICTED_REFERENCE_GLYPH_WIDTHS: [&str; 1] = ["issue9915_reduced.pdf page 1"];

/// Contradicted, where the document asks for a line width the clause forbids — **and we are
/// the ones carrying the clause out**.
///
/// 1 page. `issue19633.pdf` is 312 652 bytes of iTextSharp form, not the four-object file this
/// entry used to describe, but its crop box `[131.5 439.89 383.0 600.89]` admits exactly one
/// mark: `/Fm0`, drawn under `0.85409 0 0 0.85409 43.38 44.22 cm`, whose whole content is
/// `-0.1 w 1 j 1 J`, `0 0 m -185.44 77.07 l`, `S`. Everything else the content stream draws —
/// `/Fm1` to `/Fm6`, all of them `/Tx BMC EMC` with a `[32768 32768 -32768 -32768]` bounding
/// box — is placed at page y 771 and above, outside the crop box. So the raster is 252 × 161
/// and the page really is one stroked diagonal, 171.51 points long at 22.56° from horizontal,
/// asked for at a device width of 0.0854.
///
/// # This entry was written from the picture, and every sentence it made about a reference was
/// wrong
///
/// It said "[t]hree readings are available and each renderer takes a different one", that
/// `poppler` and `mupdf` draw "a very faint one, consistent with the magnitude, 0.1 of a
/// pixel's coverage", that `ghostscript` "draws something between the two", and that "the
/// clause does not decide it". Ink over the page's own raster (`-alpha off`, R channel; the
/// mark's length is known, so ink ÷ length is the width the renderer actually painted):
///
/// | | ink, in whole pixels | ÷ 171.51 |
/// |---|---|---|
/// | ours | 172.54 | **1.006** |
/// | `hayro` | 170.87 | 0.996 |
/// | `ghostscript` | 96.81 | 0.564 |
/// | `poppler` | 42.44 | 0.247 |
/// | `mupdf` | 37.49 | 0.219 |
///
/// The document asked for 0.0854. `poppler` paints 2.9 times that, `mupdf` 2.6, `ghostscript`
/// 6.6 — so "consistent with the magnitude" was true of nobody, and no two of the five agree
/// about anything.
///
/// # The instrument, which is ADR 0419's ladder continued through zero
///
/// The five-hundred-and-eighty-fourth session built a ladder of one rule at seventeen positive
/// widths to price each renderer's *floor*; it never asked the sign question, and the sign is
/// what this page turns on. Same geometry, same metric — a 160-unit rule on a 200 × 200 page at
/// 72 dpi, mean ink in levels of 255, so the geometry's own answer is `1.02 × w` — with the
/// ladder run down through zero into the negatives:
///
/// ```text
///    width   geometry      ours   poppler     mupdf        gs     hayro
///      1.0     1.0200    1.0200    1.0200    1.0241    1.3000    1.0241
///      0.5     0.5100    0.5120    1.0200    0.4761    0.8201    1.0241
///      0.2     0.2040    0.1999    1.0200    0.2040    0.2721    1.0241
///     0.05     0.0510    0.0479    1.0200    0.2040    0.2721    1.0241
///      0.0     0.0000    1.0200    1.0264    0.2040    0.2721    1.0241
///    -0.05     0.0000    1.0200    1.0200    0.0000    0.2721    1.0241
///     -0.2     0.0000    1.0200    1.0200    0.0000    0.2721    1.0241
///     -0.5     0.0000    1.0200    1.0200    0.0000    0.8201    1.0241
///     -1.0     0.0000    1.0200    1.0200    0.0000    1.3000    1.0241
/// ```
///
/// The positive half reproduces ADR 0419's 72 dpi table to the digit, which is how the
/// instrument is checked before its new half is believed. The new half says three things.
///
/// - **`poppler` and `ghostscript` stroke the *magnitude*.** Every negative rung equals the
///   positive rung of the same size, at every width and — swept at 0°, 1°, 5°, 10°, 20°,
///   29.4°, 45°, 60° and 90° — at every angle, to four figures.
/// - **`mupdf` does not**, and does not take one reading either: within 5° of an axis a
///   negative width paints **nothing at all**, and beyond 10° it paints exactly what that
///   renderer paints for a width of *zero*, which is its own 0.2-device-pixel floor. One
///   renderer, two answers, chosen by the angle of the line.
/// - **Ours and `hayro`'s are one device pixel at every negative rung**, which is the same
///   answer they give for zero.
///
/// # So the consensus that outvotes us is two mechanisms meeting by accident
///
/// On this page `poppler` paints 0.247 of a pixel because it is stroking |−0.1| × 0.85409 plus
/// its own anti-aliasing spread, and `mupdf` paints 0.219 because that is its floor and it
/// would paint the same for any width it will not go below. They are inside the fixed tolerance
/// of each other and they are answering different questions: at `-1 w` they are 1.02 and 0.00
/// apart, the widest disagreement on the ladder. Trap 9's second shape sitting on its first —
/// and it is decided by the *angle*: had this file drawn the same rule horizontally, `mupdf`
/// would have drawn nothing and there would have been no pair to vote.
///
/// # And the clause does decide it, one subclause above the one this entry was reading
///
/// §8.4.3.2 gives the parameter its range and stops there — "[i]t shall be a nonnegative number
/// expressed in user space units" — which is why this entry concluded that nothing decides a
/// value outside it. §8.4.1 decides it, and names this parameter while doing so:
///
/// > Parameters that are numeric values, such as the current colour, line width, and miter
/// > limit, shall be clipped into valid range, if necessary. However, they shall not be
/// > adjusted to reflect capabilities of the raster output device, such as resolution or number
/// > of distinguishable colours. Painting operators perform such adjustments, but the adjusted
/// > values shall not be stored back into the graphics state.
///
/// Three sentences and this tree obeys all three: `content.rs`'s `w` clips `-0.1` to 0,
/// `Stroke::device_width` substitutes one device pixel at painting time because §8.4.3.2
/// requires that of zero, and the substituted value is never stored back. The magnitude reading
/// is the one the first sentence forbids, because |−0.1| is not a clip of −0.1 into `[0, ∞)`.
///
/// §10.7.5's floor is not an alternative route to the references' answer either, for ADR 0419's
/// reason on a second document: it applies where "stroke adjustment is enabled", Table 52
/// initialises that parameter to `false`, and this file contains no `/SA` and no `/ExtGState`
/// at all.
///
/// **The sentence was already quoted in this crate** — `content.rs`'s `miter_limit`, for the
/// parameter the same list names one later, since the twenty-fourth session — and the §8.4.1
/// ledger row states the line-width half of it outright while §8.4.3.2's row called the same
/// clamp a documented choice. `CLAUDE.md`'s rule about reading the titles around a subject
/// before recording a silence had a shorter distance to travel here than it has ever had.
///
/// So this group is [`CONTRADICTED_IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE`]'s shape and
/// [`CONTRADICTED_VISIBILITY_EXPRESSION`]'s: the specification answers, and it answers against
/// the two renderers that agree. The page stays listed because nothing about our rendering
/// should change and the gate should keep watching it.
///
/// # The ink ladder converts into the gate's arithmetic, and it lands on the bounds this page
/// *passes*
///
/// The table above is a real measurement of a real mechanism and it is priced in ink ÷ length,
/// which is not one of the four numbers the verdict is made of. It converts, exactly. The raster
/// is 252 × 161 and `raster_compare` divides by width × height × **four** channels, so a mark
/// ours paints and a reference does not costs `Δink × 255 × 3 ÷ (252 × 161 × 4)`. Against
/// `mupdf`, `Δink` is 172.54 − 37.49 = 135.05 whole pixels, and that arithmetic is 0.6366 — which
/// is the mean the gate prints for this page, to every digit it prints. Carried into the failing
/// tile: the rule crosses a 32-pixel tile over 32 ÷ cos 22.56° = 34.7 points of its length at
/// 1.006 − 0.219 = 0.787 of a pixel of width apiece, which is `34.7 × 0.787 × 765 ÷ 4096` = 5.10
/// against a printed worst tile of 5.09.
///
/// **And the mean is a bound this page meets.** What it fails is the worst tile — the gate prints
/// 5.09 against a bound of 5.00 — and structural similarity, which is the binding one at 2.3 times
/// its bound where the tile is at 1.02. So the ladder prices the mechanism correctly and prices it
/// into the wrong column.
///
/// # Taking the sign out of the file moves `poppler` and does not move `mupdf` at all
///
/// §8.4.1's clip of −0.1 into `[0, ∞)` is 0, so a conforming reader renders `-0.1 w` exactly as
/// it renders `0 w`. Two §7.5.6 incremental updates restate object 36's stream as `0 w` and as
/// `0.1 w`, and the difference between the first and the original is *the whole of what the sign
/// is worth to each renderer*. Ours at 72 dpi, the four numbers in the gate's order:
///
/// | against | `-0.1 w`, as the file ships | `0 w` | `0.1 w` |
/// |---|---|---|---|
/// | `poppler` | 0.6133 / 4.90 / 0.6341% / 0.97959 | 0.4538 / 3.59 / 0.6267% / 0.99145 | 0.1468 / 1.20 / 0.3623% / 0.98449 |
/// | `mupdf` | 0.6366 / 5.09 / 0.6341% / 0.97700 | **0.6366 / 5.09 / 0.6341% / 0.97700** | 0.1221 / 0.99 / 0.4141% / 0.98812 |
/// | `ghostscript` | 0.3570 / 2.87 / 0.6322% / 0.99469 | **0.3570 / 2.87 / 0.6322% / 0.99469** | 0.3939 / 3.15 / 0.4880% / 0.97214 |
///
/// The gate's "ours at worst" on this page is `mupdf`'s column, and **restating the width as the
/// number the `shall` produces leaves that column byte for byte where it was**. The ladder above
/// says why before the experiment does: `mupdf` answers 0.2040 for a width of zero and 0.2040 for
/// a negative one at this angle, and `ghostscript` answers 0.2721 for both. `poppler` is the only
/// renderer the sign is worth anything against, and it is worth a third of the disagreement
/// there.
///
/// (`hayro` is absent from that table and from this page's artefact directory for a reason that
/// is not a refusal: it rasterises this crop box **251 × 161** where the other four produce
/// 252 × 161, so `raster_compare` reports a dimension mismatch and no diff panel is written. The
/// ink figure above is a mean over its own raster and is unaffected by that.)
///
/// **So the mechanism this group is named for owns none of the failing measurement.** What owns
/// it is the other half of the same derivation — §8.4.3.2's "1 device pixel wide", which ours
/// obeys and `mupdf` does not, painting 0.2 of one. The page is still ours-right and still
/// contradicted for a clause; it is a different sentence of a different subclause than the name
/// says, and the third column is the check: give every renderer a width it will draw and the
/// whole disagreement collapses to about a fifth.
///
/// **The group's *name* survived the measurement and its *verdict* did not.** ADR 0499. The page
/// is about the negative line width, and what the gate is failing us on is the width that clip
/// produces. Sixth criterion, and the second group in a row where the deciding clause was not the
/// one the group cites.
///
/// # The 2.3 above is one of the page's two readings, and it is the harsher one
///
/// Since ADR 0617 a contradicted verdict is one **every** maximal consensus reaches, and this page
/// is the only one in the pool that carries more than one set and is still contradicted — the
/// census line above counts the population it is the remainder of. Both of its sets reject us, so
/// the verdict stands; what nobody had computed until [`rank_the_contradicted_by_the_bound`] is
/// that they price the rejection at two very different amounts (ADR 0636):
///
/// Every figure below is the structural similarity, which is the measure that decides this page;
/// the pair's own column is a comparison between two references and is not a number this gate ever
/// prints for a page.
///
/// ```text
///                          the pair agree to   which bounds us at   our worst member   outside by
///   {poppler, mupdf}            0.99896          0.9900 (floor)     mupdf   0.97700      2.30x
///   {poppler, ghostscript}      0.99088          0.98176            poppler 0.97959      1.12x
/// ```
///
/// The first row is the page's own printed line — ours at ssim 0.9770 against a bound of 0.9900 —
/// and the second row is the reading beside it that nothing printed.
///
/// `mupdf` with `ghostscript` reaches 0.98828, under the class floor, so those two form no set and
/// the two above are both maximal. The taken one is `poppler` with `mupdf`, whose agreement is so
/// close that [`Tolerance::widened_to`] leaves the bound at the class floor — **trap 12's arithmetic
/// exactly: the tighter the pair, the harsher the bound derived from it** — while the rival pair's
/// own wider agreement doubles its distance into a bound admitting nearly all of the same
/// difference.
///
/// So this page's standing exemption is worth **1.12x, not 2.3x**, because that is what the
/// references' most forgiving reading of it comes to. Nothing about the verdict, the mechanism or
/// the two clauses above changes: the rival set rejects us too, and the third table's `poppler`
/// column is the one it rejects us on — the one renderer of the three the sign is worth anything
/// against.
///
/// # This page is **not** one of the three where the excluded reference meets the bound, and ADR
/// 0771 said it was
///
/// The eight-hundred-and-forty-fourth session ran trap 12's control over the whole contradicted
/// pool and named the three pages that survive it in prose. This page was one of the three names
/// and does not belong: the taken consensus is `poppler` and `mupdf`, the excluded voting
/// reference is `ghostscript`, and `ghostscript` against `mupdf` is structural similarity
/// **0.98828** where the vector class floor this page is held to is 0.9900. The consensus
/// contradicts it too, so the page is in the pool's 52 rather than its 3, and the ink ladder above
/// says why in a unit the verdict does not use: `ghostscript` paints 0.564 of a device pixel
/// between `mupdf`'s 0.219 and our 1.006, which is *between* rather than *inside*.
///
/// **The correction is the gate's rather than this note's**, since the
/// eight-hundred-and-forty-fifth session:
/// [`name_the_pages_the_excluded_reference_survives`] prints the population every run, so nothing
/// here has to be believed. ADR 0772.
const CONTRADICTED_NEGATIVE_LINE_WIDTH: [&str; 1] = ["issue19633.pdf page 1"];

/// Contradicted, where the difference is how `DeviceCMYK` becomes a pixel.
///
/// Five pages in four documents, and the group with the most evidence behind it of any here —
/// none of which is anybody's rendering. (This line said "4 pages in 3 documents" from the
/// hundred-and-sixtieth session, which is the session whose own heading below records
/// `transparent.pdf` joining as the fourth document; the array underneath it has said five all
/// along, and the five-hundred-and-fourteenth read the array.)
///
/// # What the pages are
///
/// **All four documents reach `DeviceCMYK`, and this heading said "all three" while the list
/// below it named four** — the same arithmetic the paragraph above corrects, one heading down.
/// `type4psfunc.pdf` and `postscript_type4_many_outputs.pdf` arrive through a `/DeviceN` whose
/// alternate it is, `function_based_shading_cmyk.pdf` directly and through a `/Separation`,
/// `transparent.pdf` through a `k` operator.
/// `postscript_type4_many_outputs.pdf` is the one that settles the group, because it is a
/// controlled experiment somebody else wrote: a 200-pixel page holding one axial shading
/// whose function is `{ dup dup dup dup dup dup dup dup }` and whose tint transform is
/// `{ pop pop pop pop pop pop pop pop 0 0 0 }`, so the colour is exactly `(t, 0, 0, 0)` for
/// `t` running linearly from 0 at the left edge to 1 at the right. One CMYK axis, sampled
/// two hundred times.
///
/// # What every renderer agrees about, and where they part
///
/// All five agree at both ends — white at `c` = 0, and (0, 174, 239) within a level at
/// `c` = 1. They differ *only in the interior*, which is the signature of an interpolation
/// rather than of a formula. Ours is exactly the multilinear interpolation of ADR 0009's
/// sixteen corners: at `c` = 0.5 the red channel is 127, which is 255 × (1 − c), and green
/// is 214, which is 255 − c × (255 − 173). `poppler` agrees with us to **one level over the
/// whole page**; `mupdf`, `ghostscript` and `hayro` agree with each other to under one and
/// sit 9 to 10 levels away from us on average and 80 at the worst pixel.
///
/// # Their agreement is one profile seen twice, and this tree's own evaluator proves it
///
/// Trap 9 says two references can agree because they share code or share a gap. This is a
/// third way: **they share data**, and the six-hundred-and-fifty-sixth session turned the
/// inference into byte identity. `/usr/share/ghostscript/iccprofiles/default_cmyk.icc` is
/// 187 484 bytes whose `desc` tag reads *Artifex CMYK SWOP Profile*, `md5` **fd199526f0a7e0bc
/// eb294a777cd84252**; scanning `libmupdf.so` for ICC headers finds five embedded profiles and
/// one of them is **the same 187 484 bytes at the same digest**, at offset 3 360 896.
/// `libgs.so` embeds none, so `ghostscript` reads that file off the disk and `mupdf` carries a
/// verbatim copy compiled in. Neither reads the other's, and they are the same bytes.
/// Evaluated by `pdf_model::icc` — our own A2B evaluator, written for `ICCBased` streams and
/// pointed at a file on this machine — it produces 255/219/186/150/112/59/0/0/0 in red for the
/// nine eighths of `c`. `mupdf` renders 255/220/186/150/110/54/0/0/0 and `ghostscript`
/// 254/218/184/148/108/50/0/0/0. That is not two implementations agreeing; it is one CMYK
/// profile, run twice, and the closeness of *their* agreement is also what tightens the
/// relative bound until ours is outside it (trap 12).
///
/// # Why the pages stay listed rather than being fixed
///
/// Because principle 5 forbids the fix. ISO 32000-2 states no destination for `DeviceCMYK`:
/// §8.6.4.4 says only "concentrations of process colourants", §10.4.2.1 ranks §10.3's ICC
/// route above §10.4.2's "crude approximations", and §10.3.2 licenses a processor to supply
/// a profile for a device space — which is what `default_cmyk.icc` is, somebody else's
/// choice of press. Adopting it because it would move four pages into agreement is
/// curve-fitting with a licence attached. ADRs 0009 and 0042 argue the sixteen corners, and
/// the measurement that supports them is corpus-wide rather than four pages: §10.4.2.5's
/// formula, tried, moved the gate from 802 agreeing to 800.
///
/// What would change this is a *document* asking for a profile — a `/DefaultCMYK`
/// (§8.6.5.6) or an output intent's `/DestOutputProfile` (§14.11.5), both of which outrank
/// the table and both of which are already honoured. None of these three files has one.
///
/// # A fourth document joined them in the hundred-and-sixtieth session, from the unexplained list
///
/// `transparent.pdf` sat in `CONTRADICTED_UNEXPLAINED` and is the cleanest member of this
/// group, because it has nothing else in it. One page, one shape — a wine bottle — one
/// operator: `0.82 0.7 0.54 0.67 k`, under a `/GS0` stating `/ca 1.0` and `/BM /Normal`, so
/// **no compositing stands between the conversion and the pixel**. Sampled inside the shape:
///
/// | | R | G | B |
/// |---|---|---|---|
/// | ours | 28 | 32 | 40 |
/// | `poppler` | **28** | **32** | **40** |
/// | `mupdf` | 25 | 34 | 45 |
/// | `ghostscript` | 25 | 35 | 46 |
/// | `hayro` | 26 | 33 | 45 |
///
/// Byte-identical to `poppler` and five levels from the pair the gate votes with, which is
/// this group's shape exactly. And §10.4.2.5's crude approximation is what it is *not*:
/// `1 − min(1, 0.82 + 0.67)` is 0 in every channel, so the formula draws the bottle black and
/// **not one of the five renderers does** — every one of them is on §10.3's ICC route, and
/// what they disagree about is which profile.
///
/// The heatmap is the whole silhouette rather than its edges, which is what said "colour, not
/// geometry" before any pixel was sampled. **Open the side-by-side, then the heatmap, then
/// sample one pixel** — three minutes, against a page that had been on the unexplained list
/// since the list existed.
///
/// # Re-sampled in the five-hundred-and-forty-sixth session, on the member the ratio ranks highest
///
/// `postscript_type4_many_outputs.pdf` at device (100, 25), which is `c` = 0.5 on the ramp
/// above:
///
/// ```text
///   ours (127, 214, 247)   poppler (128, 214, 247)
///   mupdf (109, 207, 246)  ghostscript (108, 207, 246)  hayro (109, 206, 246)
/// ```
///
/// 127 is 255 × (1 − c) and 214 is 255 − c × (255 − 173) to the level, so the paragraph above
/// reproduces exactly nine sessions and a hundred commits later, and the three-against-two
/// split is a profile rather than a formula. Nothing here moved; what is worth recording is
/// that it was checked rather than assumed, because this group's argument is the one most
/// often mistaken for a page to fix.
///
/// # The other three pages had never been opened, and the six-hundred-and-fifty-sixth session
/// wrote their closed forms out
///
/// Two of the five members carried the whole argument. `function_based_shading_cmyk.pdf`'s two
/// pages and `type4psfunc.pdf`'s one were admitted on the heading above — a sentence about what
/// their *dictionaries* say — which is the hypothesis trap 9's fourth shape warns is not a
/// diagnosis. All three admit a closed form, so nobody's opinion is needed for the colour:
///
/// - `function_based_shading_cmyk.pdf` page 1 is three §8.7.4.5.2 type 1 shadings on a
///   290 × 290 page. `/Sh20` and `/Sh21` share object 10, a §7.10.2 sampled function with
///   `/Size [2 2]`, `/BitsPerSample 8` and the sixteen bytes `00 00 00 00 | FF 00 00 00 |
///   00 FF 00 00 | 00 00 FF 40`, so with `/Order` defaulting to 1 the colour is bilinear in the
///   domain: `C = u(1−v)`, `M = (1−u)v`, `Y = uv`, `K = 64uv/255`. `/Sh22` is the same
///   construction one space out — a `/Separation /Spot /DeviceCMYK` whose tint is bilinear over
///   0, 128/255, 192/255, 1 and whose §7.10.3 transform is `t · (0.1, 0.9, 0.8, 0.05)`.
/// - Page 2 is **the same 600 × 600 square six times**: `/Sh30` to `/Sh35` are object 10 again
///   under `/Matrix [600 0 0 600 …]` at six integer offsets on an 1880 × 1260 page.
/// - `type4psfunc.pdf` is one §8.7.4.5.3 axial shading through
///   `[/DeviceN [/Magenta /Yellow] /DeviceCMYK …]`. Its §7.10.5 tint transform is 292 bytes of
///   `roll`/`index`/`sub` that hand-evaluate to `(0, m, y, 0)` — the identity into two of the
///   four channels — and its `/Function` is a §7.10.4 stitch over one exponential from
///   `[.2 .8]` to `[0 0]`, so the colour is `(0, 0.2(1−t), 0.8(1−t), 0)` along a 44.42-point
///   axis.
///
/// **The first thing the forms buy is a sentence with no renderer in it at all.** Multilinear
/// interpolation of ADR 0009's sixteen ink corners, applied to the closed-form CMYK and compared
/// with our own raster, is **within one level of 255 at all 125 sample points**. Ours is that
/// arithmetic, and whatever the rest of this note says about anybody else, nothing here is a
/// question about our shading.
///
/// Sampled against those forms at **125 points over the five shadings** — 25 on the axial band
/// and 25 on each square — the worst channel difference in levels of 255:
///
/// ```text
///                                   ours  poppler  mupdf  ghostscript  hayro
///   Artifex SWOP profile              48       51      8            8      8
///   CGATS001Compat micro profile      48       51      5            4      4
///   ours                               —        4     48           48     48
///   poppler                            4        —     51           51     51
///   mupdf                             48       51      —            6      4
///   ghostscript                       48       51      6            —      5
///   hayro                             48       51      4            5      —
/// ```
///
/// **Two camps, and neither of them is a shading.** Ours and `poppler` are within four levels of
/// each other at every point of every page; `mupdf`, `ghostscript` and `hayro` within six; across
/// the divide 48 and 51. Both profiles, run through this tree's own evaluator, land inside the
/// second camp and 48 outside the first. So there is nothing here for §8.7.4.5.2's interpolation,
/// §7.10.2's `/Order`, §7.10.5's operators or §8.6.6.4's and §8.6.6.5's tint transforms to be
/// wrong about, and the group's name is right about all five of its members.
///
/// # And `hayro` is a third reading of the same press, sharing neither code nor data
///
/// The heading above says the agreement is one profile seen twice, and it is — for two of the
/// three. `hayro` is the third, and `objdump -p` on `pdfref-hayro` lists `libgcc_s.so.1`,
/// `libm.so.6` and `libc.so.6` and nothing else: no `liblcms2`, no C colour library at all. What
/// it carries is `hayro-interpret`'s own `assets/CGATS001Compat-v2-micro.icc` — **8 464 bytes,
/// `desc` `uCMY`, `cprt` `CC0`, one `A2B0` tag**, against Artifex's 187 484 bytes and three
/// `A2B` tables.
/// Different size, different author, different licence, and our evaluator on either one predicts
/// all three renderers.
///
/// **That is a mechanism trap 9 did not list, and it is the trap's sixth.** Not shared code, not
/// shared data, not a shared default argument, not two wrong answers meeting at one angle: two
/// independently authored files describing **the same printing condition**. Artifex's `desc` says
/// *SWOP*; CGATS TR 001 is the characterisation data SWOP publishes. Three implementations that
/// share nothing agree because each went and got a copy of the same press.
///
/// And that is what §10.3.2's NOTE describes, which is the sentence this group turns on:
///
/// > Establishing a CIE-based source colour space can happen based on a user-driven
/// > configuration, by assumptions made by the PDF processor software, by analysis of the colour
/// > values and other properties, or by other mechanisms.
///
/// Four processors, four assumptions, one licence. `CMYK_CORNERS` is ours and is the same kind
/// of thing as theirs — which is why the 48 levels are not a defect on either side, and why
/// principle 5 forbids closing them by adopting somebody's press. §10.3.1's NOTE says the same
/// of the *destination*, and `colour.rs` cited that one for a *source* assumption until this
/// session; the two NOTEs differ by one word and the ledger's §10.3.1 and §10.3.2 rows now say
/// which is which.
///
/// # The six-square page asks each renderer a question about itself, and one answers twice
///
/// Page 2's six shadings are one picture six times, so the document states an invariant about
/// its own raster and every renderer owes six identical 600 × 600 squares — trap 9's
/// corpus-invariant instrument, with no renderer treated as truth. Ours, `mupdf`, `ghostscript`
/// and `hayro` return six squares differing in **zero** channels. `poppler` returns two answers:
/// the square's top row is painted on the three squares at `y` 640 and left white on the three
/// at `y` 20 — **600 pixels at 255 levels, one row**, on clip rectangles and shading matrices
/// that differ only by an integer translation. Recorded rather than chased, and unreported
/// upstream.
///
/// # Which bound the gate actually fails each page on, and what the press owns of it
///
/// Everything above is a measurement of *colour*, in levels of 255, and not one line of it was
/// ever in the units the verdict is made of — ADR 0497's sixth criterion, *a mechanism explained
/// is not a number accounted for*. The six-hundred-and-eightieth session put it there (ADR 0510):
///
/// ```text
///   function_based_shading_cmyk.pdf p1   mean, worst tile, differing   bound 1.00 / 5.00 / 1.00%
///   function_based_shading_cmyk.pdf p2   mean, worst tile, differing   bound 1.00 / 5.00 / 1.00%
///   postscript_type4_many_outputs.pdf    mean, worst tile, differing   bound 1.00 / 5.00 / 1.56%
///   transparent.pdf p1                   the differing fraction alone  bound 1.00 / 5.00 / 1.38%
///   type4psfunc.pdf p1                   worst tile, differing         bound 1.00 / 5.00 / 1.00%
/// ```
///
/// **`transparent.pdf` converts exactly, because it is one flat ink.** The table above samples it
/// at ours (28, 32, 40) against `ghostscript` (25, 35, 46) — 3, 3 and 6 levels — and
/// `raster_compare`'s `JUST_NOTICEABLE` is **4**, so exactly one channel of four crosses it and
/// the differing fraction is the bottle's own area divided by four:
///
/// ```text
///   ours' ink                                      11.4175% of the page
///   blue alone, as a share of all four channels     2.8750%   = 11.50% of pixels ÷ 4
///   red and green, at the silhouette's edge only    0.4413%
///   printed by the gate                             3.3163%   against a bound of 1.38%
/// ```
///
/// **The whole failing measurement is two levels of blue.** At four levels rather than six in
/// every channel this page would report about 0.44% and agree, which is what §10.3.2's licence
/// costs a verdict when the ink is dark enough to cross a noise threshold and nothing else is.
///
/// # And the press owns all of it on all five, priced by naming a press in the document
///
/// §8.6.5.6 lets a *document* say what its `DeviceCMYK` is, and this tree honours it — so the
/// counterfactual "if our source assumption had been theirs" is a §7.5.6 incremental update adding
/// a `/DefaultCMYK`, with no line of this tree's code changed. Our ablated render against **the
/// references' renders of the original file**, worst-ratio member of the consensus pair (`mupdf`
/// and `ghostscript` on all five):
///
/// ```text
///                                          ours                       with hayro's CGATS press
///   function_based_shading_cmyk p1   2.70 / 10.68 / 17.25% / .9959    0.48 / 1.74 / 0.19% / .9988
///   function_based_shading_cmyk p2   5.15 / 19.47 / 29.19% / .9956    0.41 / 3.15 / 0.46% / .9988
///   postscript_type4_many_outputs    7.30 / 18.04 / 37.25% / .9942    0.39 / 0.74 / 0.85% / .9976
///   transparent p1                   0.65 /  3.18 /  3.32% / .9952    0.41 / 1.77 / 0.66% / .9953
///   type4psfunc p1                   0.31 /  6.70 /  1.29% / .9998    0.12 / 1.72 / 0.07% / .9954
/// ```
///
/// **Every one of the five is then inside every bound**, the largest ratio in that column being
/// 0.63 of what its page allows. So the group's named mechanism owns **100% of every failing
/// measurement on all five pages** — and the paragraph above stands unchanged, because pricing a
/// mechanism is not a licence to adopt it. §10.4.2.1 ranks §10.3 above §10.4.2.5's formula and
/// §10.3.2's NOTE licenses a source assumption; moving to somebody's press because it takes five
/// pages green is the curve-fitting principle 5 forbids, and the table is what the refusal costs.
///
/// # The two profiles are not interchangeable through *our* evaluator, and the shadow end is why
///
/// Run with Artifex's SWOP profile instead, the three shading pages land in the same place — but
/// `transparent.pdf` goes to 1.07 / 5.97 / 8.64% / 0.9924, *further* than our own conversion. The
/// same 187 484 bytes `mupdf` and `ghostscript` are both reading put the bottle at **(36, 44, 53)**
/// through `pdf_model::icc` where all three renderers are within a level of (25, 34, 45), while
/// `hayro`'s 8 464-byte CGATS profile through the same evaluator gives **(25, 34, 44)**.
///
/// It is not the rendering intent — Artifex's `A2B0` and `A2B1` point at the same 41 478 bytes —
/// and `icc.rs` had already predicted it in prose, under `detect_black`: a colorimetric
/// black point walked off the device range and Little CMS's perceptual one round-tripped through
/// `B2A` "agree everywhere except in the darkest few percent". `0.82 0.7 0.54 0.67 k` is the
/// darkest few percent, and the CGATS profile carries no `B2A` for the two constructions to
/// disagree over. **So trap 9's sixth bullet is right about the region it was measured in and not
/// about this one**: our evaluator on *either* press predicts all three renderers to eight levels
/// on the sampled ramp, and on this one deep ink one of the two is eleven levels out. ADR 0510.
///
/// # `function_based_shading_cmyk.pdf` page 2 left in the six-hundred-and-eighty-eighth session,
/// and the mechanism is untouched
///
/// **The section after this one supersedes this one's *cause* and keeps its evidence.** Both
/// halves of what that round measured are still true — our raster is byte-identical across its
/// change, and the mechanism this group names was untouched — and the third sentence, that the
/// consensus dissolved, was a reading of a run rather than of the page.
///
/// It left because **the consensus dissolved**, not because we moved. That round drew §8.7.4.3
/// Table 77's `/Background` (ADR 0529), and a page moving on the round that touched shadings is
/// exactly the coincidence worth measuring rather than assuming: our raster for this page is
/// **byte-identical** across the change — rendered both ways in one sitting, `cmp` on the two PNGs
/// — and the file states no `/Background` at all. What the gate now prints is `ambiguous` with
/// **29.06 between the closest two references** against a page bound of 1.00, so no pair of them
/// agrees and there is nothing left to contradict us. Page 1 of the same file is still here, on
/// the same two references and the same mechanism, which is what says this is a reference's
/// movement on one page rather than anything about the group.
///
/// The page was in [`AMBIGUOUS_DEVICE_CMYK_CONVERSION`] for five rounds and is back here, and
/// **the consensus never moved — a reading was missing from the run that said it had.**
///
/// # What the six-hundred-and-ninety-fourth session found, and how
///
/// The figure recorded above, *29.06 between the closest two references*, is `poppler` against
/// `mupdf` on this page to the hundredth. Over the artefacts the gate writes, at 72 dpi, the
/// three differing fractions among the references are:
///
/// ```text
///   poppler   vs mupdf         29.063%      the number the removal was written on
///   poppler   vs ghostscript   29.435%
///   mupdf     vs ghostscript    0.192%      inside the page's 1.00% bound, so a consensus
/// ```
///
/// `mupdf` and `ghostscript` are 0.192% and 0.214 of 255 apart — a consensus by any bound this
/// gate holds — so **a run that reported no consensus on this page was a run with no
/// `ghostscript` reading in it**, and the only pair left was the one at 29.06. Nothing in the
/// gate's output said so, which is the instrument defect ADR 0542 fixes: `render_references`
/// tolerated a missing reference silently as long as two remained.
///
/// Everything else about the page is unchanged and was checked rather than assumed:
///
/// - **The references have not moved.** All three cached panels for this page carry an mtime of
///   2026-07-29 in the shared reference cache, and re-running today's `pdftoppm`, `mutool` and
///   `gs` with the gate's own arguments reproduces each of them — ink 80.6357, 77.9291 and
///   77.7736 of 255, to the fourth decimal.
/// - **Our raster has not moved.** The gate prints 5.15 / 19.47 / 29.19% / .9956 for it, which is
///   this note's own table above, measured in the six-hundred-and-eightieth session.
/// - **The camps are the group's subject.** Ours against `poppler` is 0.170 of 255 and 0.130% of
///   channels — the two that assume standard process inks — against `mupdf` and `ghostscript`
///   reading Artifex's SWOP profile. That is trap 9's second shape, and it is why page 1 and
///   page 2 belong to one mechanism.
const CONTRADICTED_DEVICE_CMYK_CONVERSION: [&str; 5] = [
    "function_based_shading_cmyk.pdf page 1",
    // Back from [`AMBIGUOUS_DEVICE_CMYK_CONVERSION`] in the six-hundred-and-ninety-fourth
    // session, on the reading above: it never stopped being contradicted, and the run that said
    // otherwise was judging it on two references instead of three.
    "function_based_shading_cmyk.pdf page 2",
    "postscript_type4_many_outputs.pdf page 1",
    "transparent.pdf page 1",
    "type4psfunc.pdf page 1",
];

/// Contradicted, where an image is thinner than a device pixel.
///
/// 1 page. `issue4436r.pdf` is the whole test: a 1x1 image mask under
/// `180 0 0 -0.48 10 25 cm`, so it covers 180 pixels by *0.48* of one, and the page says in
/// words that "a thin line should be visible above this text". pdf.js issue 4436 is the
/// bibliography and it is about a *document* rather than a clause — "fraction lines and top
/// bars of square root signs are occasionally missing" in a LaTeX paper at low zoom.
///
/// # This entry said "[n]othing in ISO 32000-2 decides this", and that was wrong
///
/// It reasoned from §8.4.3.2, which gives a *stroke* the rule for a zero width and says
/// nothing about an image — true, and the wrong clause. §10.7.4 has a paragraph for exactly
/// this case, and the four-hundred-and-fifth session read it:
///
/// > However, only those pixels whose centres lie within the region shall be painted.
///
/// That paragraph opens by saying a sampled image's region is determined similarly to a
/// filled shape's *though not identically*, and the sentence above is the difference. An
/// image mask is an image XObject (§8.9.6.2), so it is the paragraph that applies here — and
/// note what it does not carry over: the guarantee §10.7.4 states two paragraphs earlier,
///
/// > This ensures that no shape ever disappears as a result of unfavourable placement
/// > relative to the device pixel grid, as might happen with other possible scan conversion
/// > rules.
///
/// is stated for a *shape*. An image is therefore allowed to vanish, and this one does.
///
/// # The arithmetic, and it names four answers where the ratio had named two
///
/// The page is `[0 0 200 50]` with no crop box, rendered 200 x 50, so device y is 50 − user
/// y exactly. The `cm` maps the unit square to user y `[24.52, 25]`, which is device y
/// `[25, 25.48)`. Row 25's centre is at device y **25.5**, outside it — and no other row is
/// nearer. So the clause paints **nothing**:
///
/// | | coverage of row 25 |
/// |---|---|
/// | §10.7.4's sampled-image rule | 0.000 |
/// | the geometry (0.48 of a row) | 0.480 |
/// | ours | **0.502** — row 25 is `0x7F` across all 180 columns |
/// | `poppler`, `mupdf`, `ghostscript`, `hayro` | 1.000 |
///
/// **All five renderers depart from the clause, in two directions**, which is the ambiguous
/// bucket's shape 2 arriving on the contradicted list. Ours is §10.7.4's ledger row's
/// departure (1) — an anti-aliasing rasteriser paints a partly covered pixel partly — and the
/// four references' whole row is the *shape* rule applied to an image, which is the paragraph
/// before the one that governs. Snapping to a full row would be neither the clause nor our
/// departure, so the page stays listed.
///
/// The 0.502 against a geometry of 0.480 is a second, smaller thing and it is `tiny-skia`'s:
/// the scan converter samples four sub-rows at 0.125, 0.375, 0.625 and 0.875, and a band from
/// 25.000 to 25.480 crosses two of them, so 0.48 is quantised to 0.50. ADR 0226 removed that
/// quantum for an axis-aligned *rectangle* — `pdf_render::sub_pixel_bands` — and an image does
/// not take that path. 4.6% on one row of one corpus page, recorded rather than chased.
///
/// # One row of coverage, converted into the one bound this page fails
///
/// The table above is a coverage, which is not one of the four numbers the verdict is made of;
/// the page fails on the **differing fraction** alone, and by 1.16 points. The conversion is
/// arithmetic. `raster_compare` counts channels rather than pixels and divides by
/// width × height × 4, so 180 columns of one row, differing in three of four channels because
/// both rasters are opaque, is `180 × 3 ÷ (200 × 50 × 4)` = **1.35 percentage points** — which is
/// three quarters of the 1.80 the pixel count alone would suggest and is what decides whether the
/// page clears its bound.
///
/// Measured rather than asserted, by the instrument [`CONTRADICTED_VISIBILITY_EXPRESSION`] uses:
/// a §7.5.6 incremental update restates object 5 without the `BI … EI`, and nothing else about
/// the file changes. Against `poppler`, the differing fraction falls from the 8.49% the gate
/// prints to 7.13%, so the mask owns **1.3575** of it — the closed form to the digit. The bound is
/// twice the consensus pair's own figure and the pair barely moves, so it stays at 7.32: **take the
/// image out and this page agrees.** The control is the references against each other, where
/// `mupdf` against `ghostscript` is byte for byte what it was.
///
/// So this note *does* account for the number the gate fails us on, once its own row of coverage
/// is put in the gate's units — it belongs with [`CONTRADICTED_IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE`]
/// and not with the entries that price a mechanism in a statistic the verdict never reads.
/// ADR 0499.
///
/// And the *bound* the verdict rests on belongs to another population: the page is convicted by
/// `poppler` and `mupdf` alone, the voting pair that shares a glyph rasteriser, and it is one of
/// the 32 pages on which `ghostscript` fails the same differing-fraction bound against both
/// members of that pair — [`CONTRADICTED_GLYPH_EDGES`]'s last section has the table. The mask
/// paragraph above owns the margin; ADR 0717 measured what the bound under it is made of.
const CONTRADICTED_SUBPIXEL_IMAGE: [&str; 1] = ["issue4436r.pdf page 1"];

/// Contradicted, where the two references that agree are the same decoder.
///
/// 7 pages, and the most interesting entry in this file, because the *gate* is what is
/// wrong about them rather than the rendering.
///
/// `mupdf` and `ghostscript` both link `jbig2dec`, Artifex's JBIG2 library. On a page whose
/// image is JBIG2 they are not two implementations, they are one, and the triangulation rule
/// this gate is built on — two independent renderers agreeing is evidence — does not hold.
/// The rule's premise is independence, and nothing in the harness could have known these two
/// lack it.
///
/// What `jbig2dec` does on these seven, measured page by page in the
/// five-hundred-and-forty-sixth session rather than summarised (ink of 255, `-alpha off`):
///
/// ```text
///                                              ours  poppler   mupdf      gs   hayro
///   bitmap-halftone-composite                 17.495   19.253  22.594  22.594  17.495
///   bitmap-refine-page-subrect                17.495   17.589  21.052  21.052  17.495
///   bitmap-symbol-context-reuse               17.495    0.000 255.000   0.000  17.495
///   bitmap-symbol-symhuffrefineone            17.495   17.589  19.422  19.422  17.495
///   bitmap-symbol-texthuffrefinecustom        17.495   17.589   0.000   0.000  17.495
///   bitmap-symbol-texthuffrefinecustomposdims 17.495   17.589   0.000   0.000  17.495
///   issue20439                                17.495   17.589   0.000   0.000  17.495
/// ```
///
/// **This paragraph used to say "on four of them it decodes nothing and renders a blank page,
/// on two it produces the drawing strewn with noise blocks"; the table is what the pages are.**
/// Three are blank, three carry the drawing with extra ink, and on `bitmap-symbol-context-reuse.pdf`
/// `mupdf` renders the page *entirely black* while `ghostscript` renders it white — the one
/// page of the seven where the two are not byte-identical (`magick compare -metric AE` is 0 on
/// the other six and 159 600, every pixel, on that one). That last row is also why that page's
/// verdict line names `poppler` and `ghostscript` rather than the usual pair.
///
/// **And six of the seven are worse than a failure: `jbig2dec` says nothing at all.** Asked
/// with §2's own reference command lines, only `bitmap-symbol-context-reuse.pdf` produces a
/// warning from either program. On the other six both are silent and both return a different
/// picture — so a note that generalised the `NYI` log from one page to seven was describing
/// one page.
///
/// # The evidence that *we* are right, which is the corpus's invariant pointed at them
///
/// It is not `poppler`'s agreement, which would only be evidence that we read ISO/IEC 14492
/// the same way, and it is not `hayro`'s, whose raster is byte-identical to ours on all seven
/// because it shares `hayro-jbig2` with this tree. It is `tests/jbig2.rs`: the corpus encodes
/// one image through every coding mode the standard defines, and all of them decode to
/// byte-identical pixels here.
///
/// **The same invariant asked of each reference is what settles the group**, and it treats no
/// renderer as truth — each is compared only with *itself*:
///
/// | | distinct images over the family | self-consistent on |
/// |---|---|---|
/// | ours | **1** | all of them |
/// | `poppler` | 8 | 79 |
/// | `mupdf` | 6 | 71 |
/// | `ghostscript` | 6 | 71 |
///
/// So `jbig2dec` answers differently depending on how the image was coded, on a quarter of the
/// family. And the image it produces on the 71 it is consistent about is **byte-identical to
/// ours** — `magick compare -metric AE` between our render of `bitmap-halftone-composite.pdf`,
/// which it gets wrong, and its own render of `bitmap-halftone.pdf`, which it gets right, is
/// **0**. A decoder wrong about refinement, or about Huffman symbol dictionaries, or about
/// retained coding contexts could not produce that. (`poppler` cannot be compared byte-wise
/// with anyone: it smooths the image on the way to the page, 198 grey levels against our two
/// at one device pixel per sample, which is what its 17.589 against our 17.495 is.)
///
/// These stay listed rather than being excused, because the gate should keep watching them:
/// if `jbig2dec` is fixed they will leave this list, and if our decode changes they will
/// change too.
///
/// **Re-checked in the five-hundred-and-fourteenth session, on the page that heads the ranking**
/// — `bitmap-symbol-context-reuse.pdf` sits 28.91 from its *nearest* reference and 178.13
/// from its furthest, which is the whole contradicted list's head by either number. (Those are
/// **bounds**, not levels of 255, and this sentence said levels until ADR 0499: `Distance::of`
/// reduces each comparison to the largest of its three ratios *against the bounds this page was
/// held to*, which is what makes the figures comparable across pages at all. 28.91 is this page's
/// worst tile of 144.56 over its bound of 5.00.) Asked again,
/// the three references say what this note says, in words rather than in pixels:
///
/// ```text
/// poppler  Syntax Error (681): Too many symbols in JBIG2 symbol dictionary
/// mupdf    jbig2dec warning: segment marks bitmap coding context as retained (NYI) (segment 1)
///          library error: cannot decode jbig2 image
/// gs       jbig2dec WARNING segment marks bitmap coding context as retained (NYI) (segment 0x01)
/// ```
///
/// `mupdf` and `ghostscript` print the *same library's* sentence — one string, two programs — and
/// `poppler`'s own decoder fails for an unrelated reason. So the pair the verdict rests on is
/// `poppler` and `ghostscript` agreeing on a page **neither of them decoded**, which is trap 9's
/// second shape sitting on top of its third. There is no consensus here to be contradicted by, and
/// the reason it takes a log rather than a raster to see that is the reason this note exists.
///
/// **`issue20439.pdf` is a member of the family whose name does not say so**, which is why it sat
/// in this group while being outside the one test that can judge it. Our render of it and our
/// render of `bitmap-halftone-composite.pdf` differ in **zero** pixels, and `mupdf`'s render of it
/// and of `bitmap-symbol-texthuffrefinecustom.pdf` likewise; it is 1 300 bytes, one
/// `/JBIG2Decode` image XObject on a `[0 0 399 400]` page. `tests/jbig2.rs` admits it by name
/// since the five-hundred-and-forty-sixth session, and the admission is self-checking: a
/// different picture would make that test report two images instead of one.
///
/// # On four of the seven the verdict line was a statistic of *our* raster and of nothing else
///
/// The ink table above prices the mechanism in ink, which is not one of the four numbers the
/// verdict is made of, and on four of these pages the conversion was not an approximation — it
/// was an identity. Where both voting references return a page that is 255 in every channel,
/// `raster_compare` is comparing our render with a constant, so every figure it produces is a
/// property of our render alone. All seven of our rasters are byte-identical, and ours against a
/// synthetic 399 × 400 white sheet is **mean 13.12, worst tile 144.56, differing 5.15%, ssim
/// 0.8990** — which was, digit for digit, the line the gate printed for
/// `bitmap-symbol-context-reuse.pdf`, `bitmap-symbol-texthuffrefinecustom.pdf`,
/// `bitmap-symbol-texthuffrefinecustomposdims.pdf` and `issue20439.pdf`. The mean falls out of
/// the ink table by hand: 17.495 × ¾, because three of the four channels differ and the fourth is
/// opaque on both sides.
///
/// **No renderer that draws this image could meet any of those bounds**, which is the sharpest
/// form of what this note has been saying in words: there was nothing there to be contradicted
/// by. It is also why the *worst tile* was the bound those pages failed hardest — a 32-pixel tile
/// inside the drawing is 75.6% covered, and against white that is 144.56 of a bound of 5.00.
///
/// # Three of those four left this list in the six-hundred-and-eighty-first session
///
/// ADR 0499 recorded the identity and deliberately did not act on it; ADR 0513 is the rule that
/// did. `pdfref::consensus_abstentions` takes a reference out of the consensus where its raster
/// is one colour *and* a reference that drew marks disagrees with it — so on
/// `bitmap-symbol-texthuffrefinecustom.pdf`, `bitmap-symbol-texthuffrefinecustomposdims.pdf` and
/// `issue20439.pdf`, where `mupdf` and `ghostscript` both return white and `poppler` draws
/// 17.589, the pair that used to contradict us abstains and one reading is left. All three are
/// `NOT_COMPARABLE_A_SHARED_JBIG2_DECODER_RETURNED_ONE_COLOUR` now.
///
/// **`bitmap-symbol-context-reuse.pdf` stayed for two more rounds, and it left in the
/// eight-hundred-and-forty-second.** It was the rule's own limit rather than an oversight: on that
/// page *all three* references are one colour — `poppler` white after *Too many symbols in JBIG2
/// symbol dictionary*, `ghostscript` white, `mupdf` entirely black — so not one of them drew marks
/// and nothing in the **pixels** establishes that there was a picture to draw except our own
/// render, which reading would be the circularity ADR 0513 exists to avoid. ADR 0768 wrote the
/// narrow raster rule that would have moved it, tested it, and reverted it: `pdfref`'s own
/// `a_two_of_three_majority_forms_the_consensus` and
/// `references_disagreeing_among_themselves_is_not_our_failure` are two uniform white rasters
/// against a uniform black one, which is this page's shape exactly, because a genuinely blank page
/// with one broken renderer and a page nobody decoded with one broken renderer have the same three
/// rasters.
///
/// What separated them is not in the rasters at all: `mupdf` says *library error: cannot decode
/// jbig2 image* and `ghostscript` says *jbig2dec WARNING failed to decode; treating as end of
/// file*, and each returned a flat sheet after saying it. Since ADR 0769 `pdfref::cache` stores a
/// renderer's log beside its raster, so those sentences survive a cache hit and
/// `pdfref::consensus_abstentions` can read them; both references abstain, `poppler`'s white sheet
/// is the one reading left, and the page is
/// [`NOT_COMPARABLE_THE_RENDERERS_SAID_THEY_DREW_NOTHING`]. **`poppler` is not read here** — its
/// refusals are worded like the tens of thousands of `Syntax Error` lines it writes about defects
/// it recovers from — so the move rests on the two programs whose wording says what they produced.
///
/// **`hayro`'s raster is byte-identical to ours, and that is not a fourth reading**: `pdf-sandbox`
/// decodes §7.4.7 through `hayro-jbig2`, so the two rasters are one decoder run twice. Trap 9's
/// standing rule — agreement with `hayro` is never evidence about us — has a sharper form here than
/// it does on a font page, and what *does* establish our decode is ADR 0381's: the `bitmap-*` family
/// is one drawing encoded through nearly every path ISO/IEC 14492 defines, so each program is
/// compared with itself and this tree returns one image where `jbig2dec` returns six.
///
/// On the remaining three of the original seven `jbig2dec` returns ink rather than silence, so
/// no reference abstains and the comparison is a real one; there the ink table stops being an
/// identity and starts being a floor. The gate's mean exceeds ¾ of the ink *difference* by 1.4×
/// on `bitmap-symbol-symhuffrefineone.pdf`, 1.7× on `bitmap-refine-page-subrect.pdf` and 4.9× on
/// `bitmap-halftone-composite.pdf`, because on those the ink is displaced rather than added and
/// an absolute difference counts a moved mark twice. **An ink table cannot account for a page
/// where the disagreement is about placement**, and naming which of them those are is the honest
/// limit of this entry. ADR 0499.
///
/// # And these three are three quarters of the convictions in the tree that rest on one raster
///
/// `what_the_consensus_was_made_of` counts, every run, how many verdicts are decided by a set
/// whose members drew the *same bytes* — `raster_compare`'s `max_error` at zero, which is trap 9's
/// own tell — and names the contradicted ones. Of the whole pool, four are, and three of them are
/// these; the fourth is [`CONTRADICTED_ON_A_PAGE_WE_REPORT`]. So the population where a conviction
/// rests on `jbig2dec` twice is exactly the group already named for `jbig2dec` twice, which is why
/// the census moved no page: it found no conviction this file was not already holding by name for
/// the same mechanism. ADR 0774.
const CONTRADICTED_SHARED_JBIG2_DECODER: [&str; 3] = [
    "bitmap-halftone-composite.pdf page 1",
    "bitmap-refine-page-subrect.pdf page 1",
    "bitmap-symbol-symhuffrefineone.pdf page 1",
];

/// Contradicted, where a large image is drawn small.
///
/// **Empty as of the sixteenth session**, and kept rather than deleted because what it held
/// is the clearest example in this file of a group whose two members looked like the same
/// cosmetic difference and were not. Both agree now: ADR 0025 replaced the four-tap filter
/// with an average over the samples that share a device pixel, which is a documented
/// departure from §10.7.4's "there shall not be averaging over the pixel area" rather than a
/// reading of it. The two paragraphs below are what the group said when it had members.
///
/// `firefox_logo.pdf` was a hair over the bound: worst tile 9.97 against a bound of 9.95, mean
/// 0.09, structural similarity 0.9974. It draws a 512x543 image into about a hundred pixels
/// square, and the three references all soften the eight-fold reduction more than
/// `tiny-skia`'s bilinear filter does — bilinear samples four neighbours whatever the
/// reduction, so shrinking by eight discards most of the source and leaves a stair-step on a
/// curved edge. On that page it is a cosmetic difference, and the item sat in the handover as
/// "Small, 1 document" for four sessions on that evidence.
///
/// **`bug1001080.pdf` is the same defect, and on it the cost is legibility.** Four renderers
/// draw `pint test` and `Untitled` where we draw `pinL LesL` and `UnLiLLec` — the crossbar of
/// every `t` is gone. Worst tile 6.28 against a bound of 6.18, structural similarity 0.9986,
/// and a page a person cannot read: which is the measure's own limitation as much as the
/// renderer's.
///
/// The page has no image on it, and finding that out is the useful part. Its text is set in a
/// **Type 3 font whose every glyph description is an inline image mask** (§9.6.4 with §8.9.7),
/// each one `/F /CCF` — Group 4 fax, `/K -1` — so `t` is a 39x53 bitmap drawn through
/// `0.01 0 0 0.01 0 0 cm` inside a `/FontMatrix` of 1/83 at 9.94 pt, which puts 53 source rows
/// into about five device ones. The crossbar is one of those rows. `tiny-skia`'s bilinear
/// filter samples four neighbours whatever the reduction, and at eleven-to-one it never looks
/// at that row.
///
/// The page therefore became comparable in the twelfth session because `CCITTFaxDecode`
/// landed (§7.4.6), and the thing it exposed is not in §7.4.6 at all. The fix was the one
/// `firefox_logo.pdf` had already asked for, and it took four sessions to be scheduled
/// because the only page arguing for it was a logo 0.02 outside a bound. **A cosmetic-looking
/// entry and an unreadable one can be the same defect**, and only the second gets built.
const CONTRADICTED_IMAGE_RESAMPLING: [&str; 0] = [];

/// Contradicted, where the whole difference is that a mask value is eight bits.
///
/// 1 page. `smask_luminosity_oob_transfer.pdf` paints one red rectangle over the whole page
/// through a luminosity mask whose `/BC` is white and whose `/TR` is `0.25 + 0.5 x`, so
/// every pixel outside the mask group's bounding box — which is almost the whole page — is
/// the same interpolation between the red and the grey beneath it, at a mask value of
/// `0.75`. The closed form is `(223, 99, 80)`; we produce `(223, 100, 81)`, `mupdf`
/// `(222, 98, 79)` and `ghostscript` `(223, 99, 79)`. Everybody is within a level of the
/// arithmetic and of each other, and because `mupdf` and `ghostscript` are within one level
/// of *each other*, the bound derived from them is a mean of 1.11 — which our mean of 1.25
/// exceeds.
///
/// The level comes from the mask being quantised: `tiny_skia::Mask` holds one byte per pixel
/// and a GPU texture holds no more, so `0.75` is stored as 191 of 255 and the interpolation
/// that follows is done at that resolution. `render-cpu/tests/soft_mask.rs` pins the same
/// arithmetic against the clause and allows exactly one level for it.
///
/// It is listed rather than chased because the alternative is a mask raster of floats, which
/// costs four times the memory of every mask on every page to move a page-wide difference of
/// one level. Worth revisiting only if a page is ever contradicted by *more* than that.
///
/// # Both of those paragraphs were wrong, and the page is fixed
///
/// **Empty since the five-hundred-and-eighty-third session, and the emptying is a defect
/// found.** The name and the diagnosis under it were a hypothesis nobody had put a number
/// behind: the mask value *is* eight bits, the byte *is* 191, and eight bits is enough. Drive
/// 191 through §11.3.6's weighted average by hand — `0.85 · 191/255 + 0.949020 · 64/255` and
/// its two siblings, with `0.949020` the byte the grey fill actually wrote — and the answer is
/// `(223, 99, 80)`, the closed form the paragraph above already states. An eight-bit mask
/// predicts the closed form; it does not predict our `(223, 100, 81)`.
///
/// What did was `tiny-skia`'s **low-precision raster pipeline**. It compiles two, picks the
/// lowp one whenever every stage of the pipeline has a lowp implementation — a solid colour
/// drawn through a mask always does — and lowp's division by 255 is
/// `div255(v) = (v + 255) >> 8`, an *upper* bound on `v ÷ 255` rather than its rounding. This
/// path spends two of them per pixel, one scaling the source by the mask and one scaling the
/// destination by `1 − α`, and both biases point the same way. Reproduced by hand out of the
/// library's own source, in bytes: source `(217, 51, 26)`, destination `242`, mask `191` gives
/// `div255(217·191) + div255(242·64) = 162 + 61 = 223`, `39 + 61 = 100`, `20 + 61 = 81` —
/// which is this page, arrived at from the arithmetic rather than from the raster.
///
/// **Swept over all 256 mask values, the high-precision pipeline reproduces the closed form at
/// every one of them and the low-precision one departs by up to two levels**, always towards
/// the backdrop. `render-cpu`'s `HIGH_PRECISION_PIPELINE` asks for the first since that
/// session, `render-cpu/tests/soft_mask.rs` sweeps all 256 with no slack at all, and the page
/// agrees. It cost nothing: ISO 32000-2 page 101 is 2.1% *fewer* instructions, `alphatrans.pdf`
/// 1.4% fewer, `firefox_logo.pdf` 0.6% more. ADR 0418.
///
/// **Eleventh for eleven on a group's name naming a hypothesis rather than a diagnosis**, and
/// this one is the shape `doc/HANDOVER.md`'s trap 1 calls the cheapest tell: the note's own
/// paragraph said the verdict comes from "`mupdf` and `ghostscript` … within one level of
/// *each other*", which is [`CONTRADICTED_TIGHT_CONSENSUS`]'s mechanism, while the name
/// asserted a cause of its own that no line under it measured. Two claims in one note, one of
/// them another group's and one of them unmeasured, is what a round should read first.
///
/// The name is kept rather than corrected, on [`CONTRADICTED_CALIBRATED_COLOUR`]'s precedent:
/// this is the group whose name was wrong, and renaming it would lose the record of that.
///
/// # The gate printed 27.02 here and the entry had to state its own numbers
///
/// Until the four-hundred-and-sixth session this page's line read `ours at worst mean 27.02
/// worst tile 27.84 differing 72.71% ssim 0.9239` beside a bound of 1.11 — a page that looks
/// ruined, on an entry arguing that everybody is within a level. **The 27.02 is `poppler`,
/// which is not in the consensus and is the outlier here**: measured over the artefacts, ours
/// sits 1.66 of 255 from `mupdf`, 1.06 from `ghostscript` and 0.50 from `hayro`, while
/// `poppler` sits 34.0 to 36.0 from all four of us. Four renderers within 1.7 of each other and
/// one 35 away is not a page anybody should read a 27 off, and nothing said so.
///
/// The line now reports the comparison the verdict rests on: **mean 1.25 against a bound of
/// 1.11**, with worst tile 1.47 of 5.04, differing 0.20% of 1.00% and structural similarity
/// 0.9998 of 0.9900. One bound of four, missed by 0.14 of a level — which is what a mask
/// quantised to a byte costs, stated by the gate instead of by this paragraph. (The 2.02 this
/// entry used to quote was measured by hand in the sessions before the gate could say it.)
///
/// That correction was right about whose number the 27.02 was and wrong about what the 1.25
/// was: "what a mask quantised to a byte costs" is the claim the section above disproves. The
/// gate's own line was never at fault — it said `mean 1.25 against a bound of 1.11`, which is
/// exactly the two levels the pipeline was adding, and no reading of it could have named the
/// cause. **A number stated correctly is not a mechanism explained**, which is the second
/// lesson this entry has taught about itself.
const CONTRADICTED_MASK_QUANTISATION: [&str; 0] = [];

/// Contradicted, where a visibility expression decides the picture and a **press** decides
/// the verdict.
///
/// 1 page. This entry used to hold three, and the other two left when optional content
/// landed (§8.11) — `issue12007_reduced.pdf` was drawing a whole hidden screenshot over a
/// page the references leave nearly blank. What is left is a page where the reference
/// consensus is wrong about `/VE`, which is rare enough to be worth the paragraph — and where
/// the bound it fails is mostly about something else, which is why the title changed.
///
/// `visibility_expressions.pdf` draws five lines twice: once pale at `0 0 0 0.150 k`, and once
/// dark at `0 0 0 0.890 k` inside five `BDC /OC` sections whose membership dictionaries each
/// carry a `/VE` visibility expression and *no* `/OCGs` or `/P`. With group C off and A and B
/// on, `[/Not 9 0 R]` and `[/Not [/Or 9 0 R 10 0 R]]` are false, so two of the five dark lines
/// are hidden. We draw them hidden; `poppler` draws them hidden; `mupdf`, `ghostscript` **and
/// `hayro`** draw all five dark.
///
/// # This note never measured the page, and the heatmap had been saying so all along
///
/// Until the six-hundred-and-seventy-second session the whole entry below was four source
/// citations and a clause. It counted the page's objects — five lines, five sections, two of
/// them hidden — and measured not one pixel of it, which made it the only non-empty list in
/// this file with no measurement of its own raster anywhere in it.
///
/// `<stem>/p1/…-diff-mupdf.png` lights **all five** lines, the two disputed ones orange and
/// the three undisputed ones yellow, and the three nobody disputes carry most of one of the
/// two bounds this page fails. [`CONTRADICTED_MASK_QUANTISATION`] closes with *a number
/// stated correctly is not a mechanism explained*; this entry is the mirror of it —
/// **a mechanism explained is not a number accounted for** (ADR 0497).
///
/// # Which bounds fail, and what each is made of
///
/// The gate's line is `mean 3.89 worst tile 50.01 differing 6.38% ssim 0.9521` against
/// `mean 5.00 worst tile 40.00 differing 5.00% ssim 0.9000`. Those four *are*
/// `Tolerance::TEXT_HEAVY` to the digit, so the bound is the floor and twice the voting pair's
/// own spread is under it on every measure — which is what `mupdf` and `ghostscript` sitting
/// 0.55 of 255 and 1.58% apart says directly, measured by hand on the 340 × 340 both of them
/// cover. **Two of the four fail: the worst tile and the differing fraction.**
///
/// Three renders of the same page, each taking one mechanism out, compared with
/// `examples/compare_rasters` at the gate's own 72 dpi. Both variants are §7.5.6 incremental
/// updates on the file itself, so nothing but the named entries moves:
///
/// ```text
///   ours vs mupdf                                 mean  worst tile  differing     ssim
///   the document                                3.8696       50.01    6.3456%  0.95234
///   /VE replaced by /OCGs 8 0 R /P /AnyOn       0.9012        4.59    6.3086%  0.99553
///   ... and 0 0 0 k restated as the same rg     0.5133        2.75    1.9378%  0.99589
/// ```
///
/// **Take the `/VE` disagreement away completely and the differing fraction moves by 0.037 of
/// the 1.35 percentage points it is over by.** The page stays contradicted on that bound with
/// the group's whole subject removed. Split three ways, the 6.3456 points are `/VE` **0.037**,
/// the `DeviceCMYK` press **4.371** and glyph edges **1.938** — and the worst tile is the
/// other way round, `/VE` owning 45.42 of its 50.01.
///
/// # The gap, verified on the running binaries rather than in a source tree
///
/// The first variant is a control in both directions, and it is stronger than the citations
/// this entry used to rest on because it measures the programs that were installed:
///
/// - `mupdf`, `ghostscript` and `hayro` render it **byte for byte** as they render the
///   document (mean 0.0000, max 0) — they were drawing all five dark either way.
/// - ours moves 3.4943 of 255 and `poppler`'s 4.6608, maximum 165 apiece.
/// - The other direction, so that *they ignore optional content* is excluded: a variant
///   stating `/OCGs 10 0 R /P /AnyOn` — group C, the one `/OFF` names — is hidden by all of
///   them. They read §8.11 and they do not read `/VE`.
///
/// The source evidence is kept where it was re-checked, and two of the four items are worth
/// separating from the other two:
///
/// - `poppler` 26.07.0 exports `OCGs::evalOCVisibilityExpr(Object const*, int) const` and
///   carries `Loop detected in optional content visibility expression` — re-checked with `nm`
///   and `strings` on the installed library.
/// - `pdf.js` reads `/VE` first and falls back to `/OCGs` and `/P`, in
///   `src/core/evaluator_utils.js` — in this repository, under `doc/pdf.js`, so it is the one
///   citation here with a checkout behind it. Its issue #12097 is closed by PR #13243, and this
///   very file is the test that came with it.
/// - `mupdf`'s `source/pdf/pdf-layer.c` was cited for
///   `/* FIXME: Calculate visibility from array */` in the `OCMD` branch of
///   `pdf_is_ocg_hidden_imp`. **This machine has the package and not the sources**, so the
///   sentence is inherited rather than re-checked; what is checked is the behaviour, above.
/// - **`ghostscript`'s no longer reproduces at all.** This entry quoted `WARNING: OCMD contains
///   VE, which is not supported (ignoring)`; `strings` on `libgs.so.10` at 10.07.1 finds neither
///   that sentence nor `not supported (ignoring)`, and the run without `-q` prints nothing
///   about optional content. A citation of another project's source is a claim with no gate on
///   it, and this one outlived the string. The control above replaces it.
///
/// **`hayro` is a fourth program with the same gap, and it is the one that shares `skrifa`
/// with this tree.** So the count is not three implementations reading the clause our way
/// against two that have not: it is `poppler`, `pdf.js` and us against `mupdf`, `ghostscript`
/// and `hayro`. Agreement is evidence only where implementations can fail independently, and
/// a missing feature is not an independent failure — trap 9's second shape, now measured on
/// four binaries instead of read in two source trees.
///
/// # The other failing bound is a different group's, and a different clause's
///
/// A four-object page holding nothing but the document's own two colours (trap 9's probe)
/// says where the rest of the differing fraction comes from:
///
/// ```text
///                 0 0 0 0.150 k    0 0 0 0.890 k
///   ours          (222, 221, 222)  ( 59,  56,  57)
///   poppler       (222, 221, 222)  ( 59,  56,  57)
///   mupdf         (219, 220, 222)  ( 66,  66,  68)
///   ghostscript   (220, 221, 222)  ( 67,  67,  68)
///   hayro         (220, 221, 223)  ( 66,  64,  66)
/// ```
///
/// Ours is `colour::CMYK_CORNERS` written out — `(1 − k)·(255, 255, 255) + k·(35, 31, 32)` is
/// (222.0, 221.4, 221.6) and (59.2, 55.6, 56.5) — and `poppler` is on it byte for byte, which
/// is [`CONTRADICTED_DEVICE_CMYK_CONVERSION`]'s two camps arriving on a page whose only
/// colours are `k`. **Why it lands on this bound and on no other is
/// `raster_compare::JUST_NOTICEABLE`, which is 4**: the pale difference is 2 to 3 levels and is
/// counted as noise, the dark one is 7 to 11 and is not, so every dark glyph pixel on a page
/// that is nothing but dark glyphs enters the differing fraction. What those same pixels are
/// worth to the *other* metrics is the ladder's second row: 0.90 of 255 against a mean bounded
/// at 5.00, a worst tile of 4.59 against 40.00, and 0.996 against 0.900. A metric that counts
/// pixels and a metric that averages them do not see the same mechanism.
///
/// # The clause, and it is in two rows rather than one
///
/// §8.11.2.2 is not ambiguous — "If the VE key is present it shall be used in preference to
/// the OCGs and P keys" — so the clause settles the two hidden lines, and this page is the
/// corpus's only witness for that `shall`. What it does *not* settle is the bound the page
/// fails hardest: that one is §8.6.4.4 with §10.3.2's NOTE, where four processors make four
/// source assumptions and no clause chooses among them (ADR 0484). **Fifth round running in
/// which the deciding clause sat in a different row than the group cites** — and the first in
/// which it sits in a different *group*.
///
/// The page stays listed and nothing about our rendering should change: we carry out
/// §8.11.2.2 where three renderers do not, and the colour is the documented choice ADRs 0009
/// and 0042 argue. What is no longer claimed is that `/VE` is what the gate measures.
const CONTRADICTED_VISIBILITY_EXPRESSION: [&str; 1] = ["visibility_expressions.pdf page 1"];

/// Contradicted, where the references that agree are two that did not draw the page.
///
/// 2 pages. Trap 9's second shape at its plainest: "[a]n unimplemented feature almost always
/// falls through to a *default*", and the default here is to draw nothing at all — so two
/// renderers that both gave up produce identical white and the gate reads it as consensus.
/// The fixed tolerance decides whether the references agree, and two blank pages agree
/// perfectly.
///
/// **`issue11549_reduced.pdf`** writes `/FontName /AASGAA+Arial,Unicode MS`. A SPACE is a
/// delimiter (§7.2.3), so that lexes as the name `AASGAA+Arial,Unicode` followed by the
/// keyword `MS` — sitting where §7.3.7 requires a key, since "[t]he key shall be a name".
/// The clause states no recovery. `mupdf` discards the whole object ("ignoring broken object
/// (70 0 R)") and `ghostscript` does the same; both then render a page that is 255 in every
/// channel. `poppler` inserts a placeholder key and draws; this reader skips the stray token
/// and draws — which is `parser.rs`'s documented choice, made so that one bad token costs one
/// entry rather than the dictionary. §7.3.5 says what the file should have written:
/// "[w]hitespace used as part of a name shall always be coded using the 2-digit hexadecimal
/// notation".
///
/// **`issue11740_reduced.pdf`** is the fortieth session's fix and its consensus is two
/// failures: `ghostscript` renders it blank and `poppler` renders one blob glyph, and the two
/// are within the fixed tolerance of each other. We, `mupdf` and `hayro` draw *Оглавление*.
/// The page was drawing mojibake until §9.7.4.2's rule for a non-CID-keyed CFF was read across
/// to the bare Type 1 program its descriptor embeds — see ADR 0049.
///
/// Neither entry is a page to chase. What both are is an argument for reading a reference's
/// *log* as well as its raster: `mupdf.log` and `ghostscript.log` both say, in words, that
/// they threw the object away.
///
/// **Re-asked in the five-hundred-and-forty-sixth session, and `ghostscript`'s half of that
/// sentence needs a flag to be true.** The oracle passes `-q`, under which `gs` prints nothing
/// on either page — so a round reading the log the gate's own invocation produces would find
/// silence and conclude the note was wrong. Without `-q` it names them:
///
/// ```text
/// issue11549_reduced   Loading font F1 (or substitute) from …/NimbusSans-Regular
///                      object lacks an endobj / xref table was repaired
/// issue11740_reduced   Loading font F1 (or substitute) from …/NimbusSans-Regular
///                      error reading a stream
/// ```
///
/// Two things worth keeping out of that. `gs` reports the *stream* it could not read and then
/// loads a substitute face, so on both pages it is drawing with a font the file did not supply
/// and drawing nothing with it; and `mupdf` still prints `ignoring broken object (70 0 R)` on
/// the first, which is the sentence this note was written from. **A reference's silence is a
/// fact about the invocation before it is a fact about the renderer** — trap 3 one level down.
///
/// # The log is the diagnosis; the arithmetic is one line, and this note never wrote it
///
/// "Two blank pages agree perfectly" explains the *verdict*. What it leaves unsaid is that the
/// comparison against a blank reference has no second operand: where a reference's raster is
/// constant, `raster_compare`'s mean is `255 × (1 − our own mean channel value)` exactly, and the
/// other three numbers are likewise statistics of our render alone. Checked with `magick
/// identify` on the gate's own artefacts, both files at 72 dpi:
///
/// | | the blank reference | our ink of 255 | the mean the gate prints |
/// |---|---|---|---|
/// | `issue11549_reduced.pdf` | `mupdf` and `ghostscript`, both min = max = 1 | 12.718 | 12.72 |
/// | `issue11740_reduced.pdf` | `ghostscript`, min = max = 1 | 13.672 | 13.67 |
///
/// So the failing mean **is** our ink, to the digit, on both pages, and the same holds of the
/// bound each is held to: nothing about either figure is a measurement of a disagreement. That is
/// worth more than the two pages, because it is the general shape — a voting reference that drew
/// nothing turns all four of the gate's numbers into a description of us, and the only instrument
/// that can see it is the one this note already argues for. `poppler`'s half of the second page is
/// the exception that shows the difference: it is not blank — 0.982 of full white against
/// `ghostscript`'s 1.000 — which is the "one blob glyph" above. ADR 0499.
///
/// # Empty since the six-hundred-and-eighty-first session, and kept for what it argued
///
/// ADR 0499 wrote the identity down and left the gate alone; ADR 0513 acted on it, and this
/// group is the population that motivated the rule, so both of its pages left it by the route
/// the paragraph above describes. `pdfref::consensus_abstentions` refuses a vote to a reference
/// whose raster is one colour where a reference that drew marks disagrees with it, and:
///
/// - `issue11549_reduced.pdf` page 1 — `mupdf` and `ghostscript` both return white and
///   `poppler` draws, so both abstain, one reading is left and the page is
///   [`NOT_COMPARABLE_THE_OBJECT_TWO_REFERENCES_THREW_AWAY`];
/// - `issue11740_reduced.pdf` page 1 — only `ghostscript` returns white. It abstains, `poppler`
///   and `mupdf` are left, and those two do not agree with each other, so the page is
///   **ambiguous** and is [`AMBIGUOUS_REFERENCE_DREW_NOTHING`]'s seventh entry.
///
/// The group is kept empty rather than deleted because the sentence it was written to prove —
/// that the failing mean *is* our own ink, to the digit, on a page nothing else drew — is the
/// argument the rule rests on, and these two pages are where it was measured.
const CONTRADICTED_REFERENCES_DREW_NOTHING: [&str; 0] = [];

/// Contradicted on a page this tree *reports*, which is where the ranking's head has been sitting
/// with no group at all.
///
/// # Why this list is not one of the ratcheted ones, and why it exists anyway
///
/// The gate holds only pages we claim to draw completely, for the reason the module comment gives:
/// a page whose interpretation reports an unsupported font or an undecodable image is *expected* to
/// differ from a renderer that implements it, and `corpus.rs` owns those. That argument is right and
/// is not being reopened. What it also does, silently, is keep the contradicted list's **largest**
/// disagreements out of every group in this file — ranked by our worst measurement over the bound it
/// is held to, the two pages below were the first and the twelfth of sixty-eight, one of them by a
/// factor of ten over anything gated. So they are diagnosed here and held by a staleness check
/// rather than by a ratchet: a name that stops being contradicted fails the build, and a page that
/// stops *reporting* while still contradicted joins the gated list one assertion above and fails
/// there. Neither direction is unwatched; only the arrival of a *new* reported page is, and that is
/// exactly the population `corpus.rs` counts.
///
/// # `xobject-image.pdf` page 1 — three renderers, three pictures, no two for the same reason
///
/// 1462 bytes, hand-written, and every one of its five panels is a flat rectangle: ours and
/// `hayro`'s red, `mupdf`'s black, `poppler`'s and `ghostscript`'s white. The gate reads the last
/// two as a consensus and prints mean 127.75 of 1.00, worst tile 127.75 of 5.00, differing 50.00%
/// of 1.00% and similarity 0.4054 of 0.9900 — every bound failed, by the widest margin on the list.
/// Asked in words rather than in pixels, the three references say three different things:
///
/// ```text
/// poppler  Syntax Error (1274): Missing 'endstream' or incorrect stream length
///          Syntax Error: Unknown operator 'endstream'
/// mupdf    warning: PDF stream Length incorrect
///          warning: padding truncated image
/// gs       Incorrect /Length for stream object ... recoverable image error ... bad DecodeParms
/// ```
///
/// The file gives them two independent things to fail at, and they fail at different ones. Its
/// content stream is `500 0 0 400 0 0 cm\n/SomeImage Do\n`, 33 bytes, under a `/Length 14` that
/// stops in the middle of the `cm` — so **`poppler` never reaches the image at all**: it reads
/// fourteen bytes, meets `endstream` where an operator belongs, and stops. Its blank page is about
/// Table 5's `/Length`, which "shall be the number of bytes", and about nothing else on this page.
/// `ghostscript` repairs the length, reaches the image, and refuses *it*. So the two renderers
/// whose agreement contradicts us agree on white for two unrelated reasons — trap 9's fourth shape
/// under its second — and `mupdf`, the one reference that both repairs the stream and draws the
/// image, produces a third picture that neither of them produces.
///
/// # What the image is, and the one thing the standard does not say
///
/// The XObject states `/Width 200 /Height 100 /ColorSpace /DeviceRGB` and its `DCTDecode` data is a
/// **1 × 1** JPEG of one red sample — the file says so in a comment, `convert -size 1x1 xc:red`. The
/// `cm` maps the unit square to 500 × 400 on a 200 × 100 page, so the visible part of the image is
/// its bottom-left corner.
///
/// Two clauses point in two directions and neither addresses a file that contradicts itself.
/// §8.9.3: "The image dictionary shall specify the width, height, and number of bits per component
/// explicitly", and Table 87 makes `/Width` and `/Height` required. §7.4.8, of the encoder's
/// parameters: "The values of these parameters, which include the dimensions of the image and the
/// number of components per sample, are entirely under the control of the encoder and shall be
/// stored in the encoded data. DCTDecode may obtain the parameter values it requires directly from
/// the encoded data." In a conforming file the two agree; where they do not, the standard states no
/// recovery, so every renderer here is choosing.
///
/// The three choices are visible in the three pictures, and each is coherent:
///
/// | | reads the image as | visible result |
/// |---|---|---|
/// | ours, `hayro` | the codestream's 1 × 1 grid | the red sample over the whole region |
/// | `mupdf` | the dictionary's 200 × 100 grid, padded | black, because the visible corner is padding |
/// | `ghostscript` | neither — it refuses the image | white |
///
/// `mupdf`'s own log names its choice ("padding truncated image") and the arithmetic confirms it:
/// the visible corner is source rows 75 to 99, so the one real sample — row 0, column 0 — falls
/// outside it and every visible sample is pad. This tree's choice is `image::contradicted_frame`'s,
/// which draws the codestream's samples **and reports the contradiction beside them**, so the page
/// is on `corpus.rs`'s list saying in words what the picture cannot: that a 200 × 100 picture was
/// described and one red sample was supplied.
const CONTRADICTED_ON_A_PAGE_WE_REPORT: [&str; 1] = ["xobject-image.pdf page 1"];

/// Contradicted for drawing a link's border, where the two references that agree agree for two
/// unrelated reasons — and neither of them is a reading of the clause.
///
/// 3 pages, all gated for the first time in the twenty-first session, when appearances began
/// being constructed (ADR 0030). Each carries `Link` annotations with a colour and a non-zero
/// border width, and on each of them we and `poppler` draw the rectangles while `mupdf` and
/// `ghostscript` draw nothing.
///
/// All three arrived by becoming *comparable*. A fourth, `issue18823.pdf`, was here because
/// its links were only part of a larger difference — mean 8.10 against a bound of 5.00, spread
/// across every glyph and both ellipses on the page. It left in the thirtieth session and now
/// agrees, because those glyphs came from a bare Type 1 program this tree could not read and
/// was substituting for. **Which says the entry was right to record that the borders were 0.7
/// of it and wrong to keep the page under this label**, and is the eighth time a group's name
/// has named a hypothesis rather than a diagnosis.
///
/// # What the clause says
///
/// Table 166 names a link as one of the three subtypes a writer need not supply an appearance
/// dictionary for, gives `/C` as "a colour used for ... The border of a link annotation", and
/// says of `/Border` that it "shall be drawn as a rounded rectangle ... if the border width is
/// 0, no border is drawn". §12.5.4 adds where it goes: "completely inside the annotation
/// rectangle". `file_url_link.pdf` writes `/C [0 1 0]` and `/Border [0 0 1]`, which is a green
/// one-unit border and nothing about it is ambiguous.
///
/// # Why the agreement is not evidence
///
/// The two silent renderers are silent for different reasons, which is trap 9's second shape
/// with a twist: the premise is not "two implementations read the clause the same way" but "two
/// implementations happened to produce the same picture".
///
/// - **`mupdf` constructs no appearance for a link at all.** `source/pdf/pdf-appearance.c`
///   switches over eighteen subtypes in `pdf_write_appearance` — widget, ink, polygon, polyline,
///   line, square, circle, caret, text, file attachment, sound, the four text markups, redact,
///   stamp, free text — and its `default` arm throws `cannot create appearance stream for %s
///   annotations`. `Link` is not in the list.
/// - **`ghostscript` does implement it, and is being asked to print.** `pdf/pdf_annot.c` has
///   `pdfi_annot_draw_Link` call `pdfi_annot_setcolor` and then `pdfi_annot_draw_border`, so its
///   blank page is not a missing feature. It is Table 167's Print flag: this annotation has no
///   `/F` at all, so bit 3 is clear, and "If clear, never print the annotation, regardless of
///   whether it is rendered on the screen." Adding `/F 4` to the same file makes `gs` draw the
///   green border — checked, not assumed. Table 167's next sentence is also worth recording,
///   because it makes even the printing case ours: "If the annotation does not contain any
///   appearance streams this flag shall be ignored."
///
/// So one reference has a gap, one is answering a different question, and a viewer is a screen —
/// where bit 6, `NoView`, is the flag with a say, and it is clear on all three pages. Listed
/// rather than chased.
///
/// # The Print-flag half was checked on one page of the three, and now on all three
///
/// The argument above rests on Table 167's bit 3 being clear, and the `/F 4` experiment that
/// established it was run on `file_url_link.pdf` alone. The four-hundred-and-sixth session read
/// the other two files, which are 905 and 1 014 bytes: **all three annotation dictionaries
/// carry `/Border [0 0 1]`, a `/C`, and no `/F` entry at all**, so Table 167's flags are the
/// default of 0 on every one of them and bit 3 is clear on every one of them. `issue7115.pdf`
/// writes its `/Rect` as four *indirect references* — `[7 0 R 8 0 R 9 0 R 10 0 R]`, which is
/// what pdf.js issue 7115 is about — and the border lands where `poppler` puts it, so §7.3.10's
/// resolution reaches inside the array here.
///
/// # And the ranking says this group is the *other* shape, which is worth naming
///
/// `rank_the_contradicted` prints our distance from the nearest reference beside our distance
/// from the furthest, **in bounds rather than in levels** — `Distance::of` reduces each
/// comparison to the largest of its three ratios against the bounds the page was held to. This
/// paragraph said "in levels of 255" and quoted **5.39, 5.81 and 5.91**, with "the four
/// references spread over 8.46, 9.55 and 9.60 among themselves"; the unit was wrong and the
/// figures are not what the gate prints (ADR 0499). What survives, and is the paragraph's point,
/// is the *shape*: these pages sit nearer their nearest reference than their references sit to
/// each other, which is what a page looks like where the consensus is two renderers *declining*
/// to draw a mark, and is the opposite of the shape that accuses us.
///
/// It is also the size the mark predicts, and that arithmetic was wrong too. `file_url_link.pdf`'s
/// border is a 175 × 30 rectangle on a 200 × 50 page, whose perimeter at one unit is about 410
/// pixels going from white to `/C [0 1 0]`. This note divided by 10 000 pixels and averaged 255,
/// 0 and 255 over *three* channels to get 6.97. `raster_compare` divides by width × height × **4**
/// and sums the absolute difference over all four, so the closed form is `410 × 510 ÷ 40 000` =
/// **5.23** — and `poppler`, which draws the border where we draw it, is 5.2275 away from its own
/// borderless render, which is 410 pixels exactly. Ours is 5.1765, which is 406.
///
/// # Three things the five-hundred-and-forty-sixth session added by opening the pictures again
///
/// - **`hayro` draws no border either**, which this note had never said and which matters
///   because it is the fourth renderer and the one that shares no C library with the other
///   three. It is not a fourth reading of Table 166: `hayro` constructs an appearance for no
///   annotation subtype at all, so it is `mupdf`'s gap arriving by a different route. Step 7's
///   ink gap over the contradicted list is the measurement — `issue14802.pdf` **+10.001**,
///   `file_url_link.pdf` +3.051, `issue7115.pdf` +2.836, all three against `hayro` or `mupdf`
///   as the lightest live reference, and `issue14802.pdf` is the whole list's largest positive
///   gap. A large positive gap is content *nobody else* is drawing, and here that is correct.
/// - **The colour is the annotation's and not a default.** On `issue14802.pdf` ours and
///   `poppler` each paint 546 and 550 pixels of exactly `#0000FF` around a page of red text —
///   two renderers reaching the same `/C` through Table 166 rather than two defaults
///   coinciding.
/// - **All three files render without a word from any of the three references.** Table 167's
///   Print flag is a decision `ghostscript` takes silently, so there is no log to read here
///   and the raster is all there is; that is the opposite of
///   [`CONTRADICTED_REFERENCES_DREW_NOTHING`], where the log is the evidence. **A group where
///   the references are silent needs the clause, and this one has it.**
///
/// # And the mark owns every bound the gate fails these pages on, which took taking it out
///
/// A perimeter is not one of the four numbers the verdict is made of, and all three of these
/// pages fail on **mean and structural similarity** — the second by the wider margin, three times
/// its bound where the mean is one and a half. Table 166 states the ablation for us: "if the
/// border width is 0, no border is drawn", so a §7.5.6 incremental update restating each
/// annotation's `/Border [0 0 1]` as `/Border [0 0 0]` takes the mechanism out of the file and
/// changes nothing else. Ours at 72 dpi against `ghostscript`, which is the reference the gate's
/// "ours at worst" is taken from on all three:
///
/// | | as the file ships | `/Border [0 0 0]` | our own border's cost |
/// |---|---|---|---|
/// | `file_url_link.pdf` | 7.4518 / 0.69785 | 2.2753 / 0.97056 | 5.1765 / 0.72630 |
/// | `issue14802.pdf` | 7.0675 / 0.57766 | 1.7125 / 0.97619 | 5.3550 / 0.60121 |
/// | `issue7115.pdf` | 6.2751 / 0.71924 | 1.3536 / 0.98586 | 4.9215 / 0.73319 |
///
/// Mean and similarity, in that order; the third column is our shipped render against our own
/// borderless one, which is the mark measured with no renderer in it at all. **Every page clears
/// both bounds with the border gone** — 5.00 and 0.9000 — so the mechanism owns the whole verdict
/// and not merely the part of it a perimeter can price. And the closed form holds on the second
/// page as well as the first: `issue14802.pdf`'s `/Rect [5 10 250 40]` on a 260 × 50 sheet is a
/// perimeter of about 550 going from white to `/C [0 0 1]`, or `550 × 510 ÷ 52 000` = 5.39,
/// against `poppler`'s measured 5.3942.
///
/// The control is what makes the table a measurement rather than a rerun: `mupdf` against
/// `ghostscript`, `mupdf` against `hayro` and `ghostscript` against `hayro` are **byte for byte
/// identical** between the two variants on all three documents, because none of those three draws
/// a link border and an entry they ignore cannot move them. ADR 0499.
const CONTRADICTED_LINK_BORDER: [&str; 3] = [
    "file_url_link.pdf page 1",
    "issue14802.pdf page 1",
    "issue7115.pdf page 1",
];

/// Contradicted, drawing glyphs this gate is measuring with the *vector* tolerance.
///
/// **Empty, and the emptying is the point.** The tolerance class came from `has_text`, which
/// asked whether we read any *text* back from the page — so a page drawing glyphs from a
/// subset with no `/ToUnicode` and no names the Adobe Glyph List knows was nothing but text
/// and was judged by the bound measured on flat fills. The thirtieth session made `has_text`
/// ask what it means to ask, `Interpretation::glyphs`, which counts glyphs that *marked the
/// page*.
///
/// It cost something and bought more. 25 pages left `ambiguous` — a looser bound is one two
/// references can agree inside — of which 19 agree and 6 do not, so the gate now judges pages
/// it used to shrug at. Both of this group's members left: `issue5070.pdf` agrees, and
/// `issue7901.pdf` is still contradicted but under the bound that fits it, so it moved to
/// `CONTRADICTED_UNEXPLAINED` where the reason for it is the honest label. Its numbers are
/// worth keeping here as the archetype: 200 by 40 pixels holding nothing but the words "The
/// Free Software Definition" at about eight pixels, every absolute bound met with room to
/// spare, and 10.12% of pixels differing against a bound of 7.98% — which on a page that is
/// entirely glyph edges is the anti-aliasing of every letter, against two references that share
/// `FreeType`. (The differing fraction read 9.89% when this paragraph was written and the gate
/// prints 10.12% now; the bound is quoted beside it from the same line, so the next round that
/// finds one of them moved can see which — ADR 0495.)
///
/// The reverse direction is real too and arrived with it: a page of *invisible* text over a
/// scanned image reads back plenty of text and marks the page with no glyph at all, and it is
/// an image page. `issue11150_reduced.pdf` was one, and it was drawing nothing where four
/// references draw three thetas — a bare Type 1 program this tree could not read until the
/// same session. The instrument found it by tightening, not by loosening.
const CONTRADICTED_GLYPHS_JUDGED_AS_VECTOR: [&str; 0] = [];

/// Contradicted, missing one glyph of a symbolic simple font whose `/Differences` misname it.
///
/// 1 page, arrived in the twentieth session by becoming comparable, and the cause is not that
/// session's work.
///
/// `issue20232.pdf` is an engineering drawing that four renderers draw identically down to the
/// frame and the title block — except that its dimension label reads `⌀56` in three of them and
/// `56` in ours. The glyph comes from `/F1`, a simple `TrueType` font named `HPDFAA+Symbol_A`
/// whose `/Encoding` names code 71 `/Ccedilla` in a `/Differences` array, whose `/Widths` gives
/// that code 448 units, and whose embedded subset holds the *diameter* sign there — 448 is also
/// what the document's own `CIDFont` gives CID 8709, which is U+2205. So the name in
/// `/Differences` describes nothing in the program, and the code reaches no glyph at all.
///
/// # The story this entry carried for nine sessions was wrong, and reading §9.8.2 settled it
///
/// It said the descriptor's contradictory `/Flags` — 36, which sets the Symbolic bit (4) *and*
/// the Nonsymbolic bit (32), a combination Table 121 forbids in the same sentence that defines
/// them — left §9.6.5.4's symbolic route "unreachable here". That is not what happens, and
/// §9.8.2 says why it cannot be:
///
/// > The use of the two flags to represent a single binary choice is a historical accident. A
/// > PDF processor should always check the Symbolic flag to determine whether the state is
/// > Symbolic or NonSymbolic.
///
/// Which is exactly what `is_symbolic` does. So the font *is* symbolic, `/Encoding` *is*
/// ignored as §9.6.5.4 requires, and the code goes to the (3, 0) subtable — where it finds
/// glyph 34, which this subset embeds with an **empty outline**. Of the font's 160 glyphs only
/// two have any contour at all: 0, and 90, which the `post` table names `Ccedilla`. So the
/// document's `/Differences` is the only statement in the file that reaches the diameter sign,
/// and §9.6.5.4 is the clause that says not to read it.
///
/// Three things are malformed about this font, and each is a "shall" the file breaks: the two
/// flags, the (3, 0) subtable's codes (which the clause confines to one of four ranges and
/// which here run to 0x2219), and `/BaseEncoding /StandardEncoding`, which Table 112 does not
/// list among the three names it permits. §9.6.5.4's NOTE 1 is about exactly this file —
/// "implementations … have evolved heuristics for dealing with such problems; those heuristics
/// are not described here" — and the clause's escape hatch does not open, because it is for a
/// character that "cannot be mapped", and this one is mapped, to a blank. Listed, with the
/// reading that produces the blank written down, rather than fixed by copying a heuristic.
///
/// # The list is empty and the glyph is still missing
///
/// `issue20232.pdf` left in the sixty-first session, and **not because anything above was
/// fixed**: its crop box is fractional, so it was one of the eleven pages the y-flip defect
/// moved a row (ADR 0064), and with the row back where it belongs the one absent glyph is
/// inside the bound. The drawing still reads `56` where three references read `⌀56`.
///
/// So this entry is kept, empty, as the record of a difference the gate can no longer see.
/// **A page leaving a contradicted list is not the same as a page being right** — the gate
/// answers "within the bound the references set for each other", and a single missing glyph on
/// a 595x842 engineering drawing was always going to be near it. The reading is unchanged and
/// so is the file.
const CONTRADICTED_SYMBOLIC_FONT_FLAGS: [&str; 0] = [];

/// Contradicted, with a font on the page that carries no embedded program.
///
/// 8 pages, and it held 17 until the four-hundred-and-thirty-first session measured the five
/// that had never been opened and found them to be `CONTRADICTED_GLYPH_EDGES` (below), and 12
/// until the four-hundred-and-sixty-first measured `calrgb.pdf`'s four and found them to be
/// [`CONTRADICTED_CALRGB_TO_SCREEN`]. The header
/// said 19 while the list held 18 before that, which is what a count written beside a list rather
/// than counted off it does. The weakest entries here, because the difference
/// need not be anyone's defect:
/// every renderer substitutes, and where two references happen to choose the same system
/// font and we choose another, the consensus is about their font rather than about the page.
///
/// **Since the hundred-and-forty-eighth session it is the references that are machine-dependent
/// here and not us.** §9.6.2.2's fourteen font programs are compiled into this binary
/// (`pdf_font::standard`, ADR 0133), so a non-embedded standard-14 font draws the same on every
/// machine; `poppler`, `mupdf` and `ghostscript` still resolve one through fontconfig. This group
/// would therefore *grow* on a machine whose installed faces differ from this one's — not because
/// our answer changed, but because theirs did.
///
/// Listed rather than excluded, because a page in this group can *also* be wrong for a real
/// reason, and dropping it would hide that. This is not hypothetical: `calgray.pdf` and
/// `calrgb.pdf` were filed here because they label their swatches with a non-embedded font,
/// while what actually differed was the swatches — every value markedly too dark, because
/// `CalGray` and `CalRGB` were being treated as their device equivalents rather than
/// converted through XYZ as §8.6.5.2 and §8.6.5.3 define them. Eight of their twelve pages
/// left this list when that was fixed.
///
/// **The four of `calrgb.pdf` that remained are gone too, and the sentence that kept them here
/// was right about the mechanism and wrong to leave them under this name.** It read "a residue of
/// colour management rather than of fonts", which is exactly what the four-hundred-and-sixty-first
/// session measured them to be: against `poppler` — a renderer that substitutes a *different*
/// serif face from ours — not one channel of the swatch interiors moves by more than four levels,
/// while two thirds of the difference against the pair that decides the verdict lies inside
/// swatches that hold no glyph at all. [`CONTRADICTED_CALRGB_TO_SCREEN`] has the whole measurement.
/// **A group's own note naming another group's mechanism is a page in the wrong group**, and this
/// one said so from the sixth session — the same one that converted these spaces through XYZ and
/// took eight of their twelve pages off this list — without the other four moving.
///
/// Seven entries left this list without being fixed, and the distinction matters: they are
/// **Type 3** fonts, which have no embedded program because §9.6.4 gives them no program at
/// all — each glyph is a content stream. They were reaching the substitution path, which is
/// how they came to be filed here, and they now report instead. They left this list by
/// leaving the comparison, not by getting better.
///
/// # Six more left in the twelfth session, and none of them was ever about a font
///
/// `hello_world_rotated.pdf` pages 1 to 5 and `issue6019.pdf` page 1 all carry `/Rotate 90`,
/// and every one of them was drawn **turned by 180°** from what four other renderers produce:
/// §7.7.3.3's clockwise rotation was built anticlockwise, so 90 and 270 were exchanged. They
/// were filed here because they also happen to name a font nobody embedded, which is the
/// caution at the top of this comment arriving in the largest quantity it has yet — a
/// hypothesis about a group is not a diagnosis of its members, and six of these twenty-five
/// were one line in `content.rs`. The picture said so in ten seconds; six sessions of counting
/// had not.
///
/// # And one more in the fifteenth, for the third distinct non-font reason
///
/// `alphatrans.pdf` was contradicted by all three references and filed here because its
/// labels are set in a font nobody embedded. What differed was the *gradient* it announces on
/// itself as `Gradient: .5`: a shading replaces the current colour rather than tinting it, so
/// §11.6.4.4's constant alpha was dropped along with the colour it did not use, and we painted
/// an opaque gradient over three objects the references show through it. `Shading::with_alpha`
/// is the fix. The page agrees now — and it is no longer in this comparison either, because
/// the same session began reporting §11.6.2's fill-and-stroke and that page has one.
///
/// # Two left in the seventeenth, and neither was about its font either
///
/// `knockout_groups_test.pdf` pages 2 and 3 were filed here for the usual reason and are
/// knockout groups (§11.4.6), which the same session began reporting. They are still
/// contradicted; they are no longer *judged*, which is the trade a report makes. Four for
/// five now on this list's name failing to diagnose a member.
///
/// The seventy-first session implemented §11.4.6 for the elements whose shape a rasteriser
/// can draw, and page 2 came from mean 5.07 to 3.08 without leaving the contradicted list —
/// it still held what that condition refuses. Page 1 of the same document became a page we
/// draw completely and its mean fell 4.19 to 3.20.
///
/// **Both pages left it in the three-hundred-and-ninety-seventh**, when the display list
/// started stating a knockout element's shape apart from its alpha (ADR 0234): page 2 at mean
/// 3.20 and page 3 at 4.02 stopped being contradicted and now agree, and so did
/// `knockout_nested.pdf` at 8.19, `knockout_nested_group_alpha.pdf` at 6.42 and
/// `knockout_smask.pdf` at 8.06. Five for six on this list's name failing to diagnose a
/// member, and the diagnosis was in the clause both times.
/// # Six arrived in the hundred-and-forty-eighth session, and the reason is the whole point
///
/// The standard 14 font programs are compiled into the binary now (ADR 0133), so `/Helvetica`,
/// `/Times-Roman`, `/Symbol` and `/ZapfDingbats` are drawn from Liberation Sans and PDFium's
/// Foxit faces on every machine rather than from whatever this one has installed. **That is what
/// put six pages here, and it is trap 9's second shape read from the inside.** `poppler`, `mupdf`
/// and `ghostscript` all resolve a non-embedded standard-14 font through this machine's
/// fontconfig, so until that session we agreed with them partly because we were reading *their
/// data*: the same URW faces, off the same disk. We no longer are, and the oracle noticed
/// immediately.
///
/// Every one of the six was opened and every one draws the same text in a different face. There
/// is no defect among them: `issue6069.pdf` is one line of sans-serif, `issue9243.pdf` one word
/// under a gradient, `bug847420.pdf` one italic line, `issue11403_reduced.pdf` one line in which
/// *ghostscript* draws a stray acute accent nobody else does, `bug850854.pdf` likewise, and
/// `issue15716.pdf` is a grid of card suits where ours are Foxit's ZapfDingbats and theirs are
/// the machine's clone of it.
///
/// **`issue5238.pdf` page 1 went the other way and left this list**, which is worth as much as
/// the six: the compiled-in face agrees with the consensus where the machine's did not.
///
/// **This is the trade written down**: five contradicted pages net, against a page that renders
/// the same on every machine — which is what §6.3.2.2 asks of a rendering processor by way of
/// Table 109's permission, and what `CLAUDE.md`'s principle 5 means by not treating agreement
/// with other renderers as the definition of right.
///
/// **This sentence cited §9.6.2.2's "These fonts, or their font metrics and suitable
/// substitution fonts, shall be available to the PDF processor" until the
/// four-hundred-and-nineteenth session**, and Errata Collection 3 strikes that outright while
/// turning the paragraph above it into an informative NOTE (Issue #47 and #48, `/State`
/// `Review` `Completed`). The four-hundred-and-eighteenth corrected the same quotation in three
/// files and named two it had missed; this is one of them, found when the sweep learned to
/// split a quotation at its own ellipsis. `pdf_font::standard` carries the warrant that
/// survives, and it is the stronger one — a requirement on what is drawn rather than a sentence
/// about what a processor happens to have.
///
/// # Two more in the hundred-and-fifty-sixth, and the sentence written about them was false
///
/// `noembed-eucjp.pdf` and `noembed-sjis.pdf` are one line of あいうえお in a non-embedded
/// Japanese font, and they were added here with a comment saying that "the side-by-side has the
/// same five kana in the same places in all four panels … which is what two different Japanese
/// faces look like". **Our panel is blank.** The interpretation produces *no commands at all*
/// and the raster's mean is exactly 1.0; `poppler`, `mupdf` and `ghostscript` draw the kana and
/// `hayro`, the other Rust renderer, is blank beside us. The pages were contradicted because we
/// drew nothing, not because we drew a different face.
///
/// Nothing announced that, because a substituted composite font whose face has no glyph for the
/// characters §9.10.2 gives it simply drew nothing and reported nothing — which is what the
/// hundred-and-eighty-second session fixed (ADR 0152). **They left this list twice in two
/// sessions and only the second time was a fix.** The 182nd made them *report*, so they stopped
/// being judged; the 183rd chose their substitute by coverage rather than by family name (ADR
/// 0153) and they now draw あいうえお and **agree** with all three references. So the sentence
/// this comment used to hold — five kana in the same places in every panel — is true at last,
/// and was written twenty-seven sessions before it was.
///
/// The lesson is the older one about this file: a group's comment is a claim about a picture,
/// and a picture is one `Read` away.
///
/// # `issue8125.pdf` page 1 left in the three-hundred-and-eighty-ninth, and not for a font
///
/// It agrees with `poppler` and `mupdf` since §10.7.4's sub-pixel rule reached this backend (ADR
/// 0226), and the mechanism is worth the paragraph because it is the *opposite* of the one that
/// rule was written for. The page states one rectangle twice whose device extent is **0.882 of a
/// pixel** along y; `tiny-skia`'s scan converter samples four sub-rows at their centres, so a
/// shape that crosses all four is rounded **up** to a whole row, and the mark was 13% larger than
/// the document asked for. It is now 0.598 of one row plus 0.284 of the next, which is its own
/// area. Nothing here disappeared and nothing was promoted — a promotion was *withdrawn*, which
/// is why this page is proof that the rule does not fight the anti-aliasing departure on
/// ordinary thin shapes.
///
/// The font is still substituted and the page's text is still drawn in a different face; that
/// simply was not what put it over the bound.
///
/// # `issue4304.pdf` page 1 left in the four-hundred-and-fifth, and it was six missing spaces
///
/// **Six for seven on this list's name failing to diagnose a member**, and this one is the
/// worst of the seven because the page drew the exact defect the file was collected to prove.
/// pdf.js issue 4304 is titled *PDF Without Spaces*, and the four-panel strip says it in one
/// look: four renderers draw *Words that should have spaces between them.* and this tree drew
/// **Wordsthatshouldhavespacesbetweenthem.** The font is a non-embedded `/Times-Roman`, so the
/// page landed here; the face was never what differed.
///
/// The file is 895 bytes and is one experiment: `/Differences [ 32 /.notdef 39 /quotesingle
/// … ]` over a standard-14 font dictionary with no `/Widths`, `/FirstChar`, `/LastChar` or
/// `/FontDescriptor`. So code 32 selects the glyph named `.notdef` — §9.6.5.1's Table 112
/// makes the first name after a code "the name corresponding to that code", and §9.6.5.2 says
/// every Type 1 program contains an actual glyph of that name whose effect "is at the
/// discretion of the font designer". Drawing nothing for it is right. **The advance is what
/// was wrong**: §9.2.4 makes a glyph's width "the distance the current text position shall
/// move … when the glyph is painted", and this tree moved it by 0.
///
/// The measurement is the column profile, and it is exact rather than close. Our ink runs
/// device x 11 to 176 and all four references' 11 to **191**: 15 pixels, which is six spaces
/// of 250/1000 em at 10 pt, to the pixel. Ours was short by the whole width six times.
///
/// # Where the 0 came from, and it is a reader this tree has and did not use
///
/// §9.6.2.1's closing paragraph used to put the obligation on the processor in so many words —
/// "For compatibility reasons PDF processors shall provide glyph widths and font descriptor
/// data for those standard fonts for use in processing PDF files when the entries are absent" —
/// and Errata Collection 3 strikes the whole sentence, leaving a cross-reference to §9.6.2.2 in
/// its place (Issue #47 and #48, `/State` `Review` `Completed`; corrected here in the
/// four-hundred-and-nineteenth session). What is left standing is Table 109's permission to
/// omit the four entries and §6.3.2.2's requirement to render the page, which between them make
/// the provision the processor's whatever the struck sentence said; and
/// `pdf_font::standard_metrics` is it. Adobe's published metrics
/// name only the standard character set and no `.notdef`, which is consistent with §9.6.5.2
/// leaving that glyph to the designer, so `simple_widths` fell through to its third source:
/// the advance the substitute program itself states, which Table 109 requires to agree with
/// the width anyway ("These widths shall be consistent with the actual widths given in the
/// font program").
///
/// That third source read the program through `skrifa`'s `FontRef`, **which parses an sfnt
/// container and refuses a bare CFF** — and ten of the fourteen compiled-in standard faces
/// (ADR 0133) are bare CFF programs from `PDFium`'s Foxit set. So on every serif, fixed-pitch
/// and symbolic standard-14 substitution the third source answered nothing at all, and the
/// code fell to Table 120's `/MissingWidth` default of 0. `pdf_font::cff::advances` is the
/// reader that was missing; `FoxitSerif.pfb` states 250 for `.notdef`, which is what the page
/// draws now.
///
/// **The 250 is corroboration and not the target.** It is read from the program this tree
/// draws, and the same number turns up in every metric clone of the face for the same reason —
/// `NimbusRoman` 250, `NimbusSans` 278, `NimbusMonoPS` 600, `LiberationMono` 600, each of them
/// its own `space` — which is why four renderers agree. The clause is what decides; that they
/// agree is how we know the clause was read the same way.
///
/// # The clause this group has been arguing from without naming, since the sixth session
///
/// Every paragraph above turns on substitution being the standard's to leave open, and no
/// entry here ever cited the sentence that leaves it. §9.5 NOTE 5:
///
/// > However, some details of font naming, font substitution, and glyph selection are
/// > implementation-dependent and can vary among different PDF processors and operating system
/// > environments.
///
/// That is `doc/todo/00`'s shape 3 — the clause puts the answer beyond itself and says so —
/// and it is why a page here is *listed* rather than chased. Its sentence before says which
/// files it is talking about, and it is this list exactly:
///
/// > If a PDF file refers to font programs that are not embedded, the results depend on the
/// > availability of fonts in the PDF processor's environment.
///
/// What neither sentence leaves open is the *advances*: §9.6.2.1's Table 109 states where those
/// come from whatever face is chosen, which is the measurement below and the half of such a
/// page this tree can still be held to.
///
/// # `bug847420.pdf` page 1, measured in the four-hundred-and-sixth, and it is the group's name
///
/// It was written up as the head of the contradicted list ranked in *levels* — the hand-built
/// second ranking [`rank_the_contradicted`]'s own comment describes — at "8.65 of 255 from the
/// nearest of four renderers that agree among themselves to 4.64, twice as far as any page on the
/// list that is not a link border". **The unit is real and all three figures are wrong about their
/// operand**, on rasters that have not moved since: 8.65 is our distance from **`hayro`**
/// (8.6520), the *furthest* of the four and the one that does not vote, where the nearest is
/// `poppler` at **7.44**; the four references' six pairwise means run **1.38 to 3.48** and 4.64 is
/// none of them; and `issue15716.pdf` sits **13.96** from its nearest in the same unit, so nothing
/// here is twice anything. Our render has not changed — the ink ladder below still reproduces to
/// three decimals — so these were the wrong end of the range when they were written rather than
/// figures that decayed (ADR 0510). What is true and was the point is that nothing had ranked that
/// list at all until `rank_the_contradicted`.
/// 200 × 50 points, one line: `/BaseFont /Arial,Italic`, a `/TrueType`
/// dictionary with no `/FontFile`, `/Flags 96` (Nonsymbolic and Italic), `/ItalicAngle -120`
/// and a full `/Widths`. Mozilla bug 847420 is *Text that should be italicized*, and it is:
/// five renderers draw it sloped, so the `,Italic` half is nobody's difference.
///
/// **The advances are the document's and are exact.** At 8× — ours through `render_at`,
/// `poppler` through `pdftoppm -cropbox -r 576`, `mutool draw -r 576` — the line's ink spans
/// **1420** device columns for us and **1419** for both references, over a 1600-column raster.
/// One part in 1420 across thirty glyphs is `/Widths` honoured, which is Table 109's
/// requirement and the half of the page the clause does decide.
///
/// **What differs is the face, and it differs by weight rather than by placement.** The ink
/// ladder is ours 12.955 → 12.824 from 1× to 8× against `poppler` 13.224 → **13.306** and
/// `mupdf` 13.324 → **13.311**: two references converging to 0.005 of each other and ours 0.48
/// of 255 below them, 3.6% less ink at every scale. A scan-conversion difference shrinks with
/// the pixels and this one does not. The 8× crops show it directly — ours is a narrower oblique
/// with a tailed `a`, theirs a wider one — because `Request::derive` folds `/BaseFont` to
/// `arialitalic`, `names_a_standard_font` accepts `arial` as a metric clone of Helvetica, and
/// the compiled-in face answers where `fontconfig` answers for them. **This is ADR 0133's
/// stated trade being visible on one page**, and §9.5 NOTE 5 is why it is not a defect.
///
/// It is also *not* `CONTRADICTED_GLYPH_EDGES`, and the bounds now say which: that group's
/// pages fail on the differing fraction and nothing else, while this one fails **three of the
/// four** — mean 8.45 of 5.00, differing 9.16% of 8.09%, structural similarity 0.8581 of
/// 0.9000, with only the worst tile inside. A different face and a different sub-pixel phase
/// are different measurements, and one page of each is what shows it.
///
/// # Seventeen pages were two populations, and the four-hundred-and-thirty-first measured which
///
/// The paragraphs above examined six pages one at a time over four hundred sessions. **Five had
/// never been opened at all** — `bad-PageLabels.pdf`, `franz_2.pdf` and `issue8088.pdf`'s three —
/// and they were admitted on the group's own membership rule, which is that the page names a font
/// nobody embedded. That rule is a hypothesis about what the page carries and this comment has
/// said so since the sixth session; measuring all seventeen the same three ways splits the list
/// along the substituted *family*, and it splits it cleanly.
///
/// **Where the file names a Times face, the substitution costs nothing that can be measured.**
/// Ink at the page's own scale, in levels of 255, and the ink's bounding box at 8×:
///
/// | page | ours | `poppler` | `mupdf` | box at 8×, ours | `poppler` |
/// |---|---|---|---|---|---|
/// | `bad-PageLabels.pdf` p1 | 9.440 | 9.449 | 9.495 | 1442 × 86 at (83, 176) | 1441 × 86 at (83, 176) |
/// | `franz_2.pdf` p1 | 115.278 | 115.285 | 116.251 | 1437 × 102 at (83, 164) | 1436 × 102 at (83, 164) |
/// | `issue8088.pdf` p1 | 12.966 | 12.870 | 12.919 | 1233 × 143 at (84, 133) | 1233 × 143 at (84, 133) |
/// | `issue8088.pdf` p2 | 13.247 | 13.095 | 13.140 | | |
/// | `issue8088.pdf` p3 | 13.188 | 13.010 | 13.055 | | |
///
/// Three renderers putting the page's ink in the *same box to the pixel* over 1600 columns is
/// §9.2.4's advances and the same cap height; `issue8088.pdf`'s three boxes are identical in all
/// of ours, `poppler`'s and `mupdf`'s, and the other two differ by one column in 1440. Each of
/// the five fails exactly one of the four bounds and it is the differing fraction — 5.03% to
/// 5.59% against 5.00% — with mean at most 1.62 of 5.00 and structural similarity at worst 0.9909
/// of 0.9000. **That is `CONTRADICTED_GLYPH_EDGES`' diagnosis and not this one's**, so the five
/// have moved there. (`franz_2.pdf`'s wide spread is `mupdf` alone, and it is not a glyph: the
/// page is a `0.5 0.5 0.5 sc` background that four renderers give as 146 and `mupdf` as 145, on a
/// page that is nearly all background.)
///
/// **Where the file names a Helvetica or Arial face, the substitution costs one number.** The
/// compiled-in sans is Liberation Sans (ADR 0133) and the references resolve one through this
/// machine's fontconfig, which is `NimbusSans`. Drawn by `magick` at 144 px/em straight from the
/// two files, the capital `I` is **99 rows** for Liberation Sans and **105** for `NimbusSans` —
/// 0.6875 em against 0.729167 em, and the same 0.6875 against 0.729167 for the bold pair at
/// 288 px/em. That is 5.7% shorter capitals, and it is what the rasters show:
///
/// | page | `/BaseFont` | ours | lightest reference | cap rows at 8×, ours | theirs |
/// |---|---|---|---|---|---|
/// | `issue6108.pdf` p1 | `/Arial`, 12 pt | 11.023 | 11.356 | 66 | 70 |
/// | `issue7580.pdf` p1 | `/ArialMT`, 18 pt | 14.954 | 15.435 | 99 | 105 |
/// | `issue9243.pdf` p1 | `/Helvetica-Bold` | 18.630 | 20.179 | 50 | 54 |
/// | `bug850854.pdf` p1 | `/Helvetica` | 13.217 | 13.586 | 110 (`B`) | 117 |
/// | `bug847420.pdf` p1 | `/Arial,Italic` | 12.955 | 13.224 | 77 (`T`) | 82 |
/// | `issue6069.pdf` p1 | `/Arial` | 12.650 | 12.869 | 77 (`M`) | 82 |
/// | `issue11403_reduced.pdf` p1 | `/Helvetica` | 14.710 | 14.841 | 99 (`E`) | 105 |
///
/// 66/96 and 99/144 are 0.6875 exactly, 70/96 and 105/144 are 0.729167 exactly — the two font
/// files' own numbers, at two point sizes, arrived at without comparing anything to anybody. **So
/// the whole of this group's remaining sans-face difference is one metric**, and the ink deficit
/// it produces runs 1.0% on `issue11403_reduced.pdf` to 7.7% on `issue9243.pdf`, which is a page
/// of nothing but capitals and therefore the pure case.
///
/// **The table's bottom four cells were empty until the five-hundred-and-fourteenth session**, which
/// is to say four of the seven sans pages carried this diagnosis on the strength of their `/BaseFont`
/// and their ink alone — the membership rule this group's own history keeps catching. They are filled
/// now, one capital per page at 8× through `render_at` against `pdftoppm -cropbox -r 576` and
/// `mutool draw -r 576`, and every one of them lands on 0.942857 of the reference's to within half a
/// row: 82 × 0.942857 = 77.3, 117 × 0.942857 = 110.3, 105 × 0.942857 = 99.0. Each capital sits on
/// the *same baseline* as the references' — 240, 240, 240 and 264 device rows — and is short only at
/// the top, which is what a cap height is and what a smaller point size would not be.
///
/// **Two of the four had to be measured twice, and the reason is `doc/todo/00`'s own warning that a
/// band of rows is a hypothesis about what is in it.** The whole line's ink box on `issue6069.pdf` is
/// 106 rows in ours and 107 in both references — no difference at all — because the line's tallest
/// ink is the ascender of `h`, `l` and `d` and the dot of `i`, which the two faces place alike; the
/// difference is confined to the capital, and a crop of 55 columns is what shows it. On
/// `issue11403_reduced.pdf` the leading `2.` reads 101 against 104, a ratio of 0.971 that fits nothing
/// — a digit's height is not a cap height in either face — and the `E` of *Eat* two glyphs later is
/// the exact 99 against 105. **A page-level box cannot test a per-glyph metric**, which is ADR 0174's
/// lesson arriving in a different instrument.
///
/// (One thing seen while cropping and worth a line, because it is evidence about a reference rather
/// than about us: on `issue11403_reduced.pdf` `mupdf` draws a stray acute accent 32 device columns to
/// the left of the line — its ink box starts at column 125 where `poppler`'s starts at 157 and ours at
/// 159 — so the pair the gate calls agreement here differs by a mark one of them invented.)
///
/// **And that parenthesis had a second half nobody could see until the
/// seven-hundred-and-twenty-seventh session.** `poppler` and `ghostscript` agree on that page too,
/// inside every class bound — 4.815% of channels against the 5.00% its class allows, which is
/// `examples/compare_rasters` between two references rather than a figure the gate prints — and
/// neither that pair nor the one the gate took contains the other, since `ghostscript` and `mupdf`
/// are the two that part, at 5.16%. Under `{poppler, ghostscript}` the bound widens to twice
/// 4.815% and our 6.24% is inside it. Which pair decided it was the order the subsets are
/// enumerated in (ADR 0616). The face measurements above are unaffected,
/// because none of them was taken against the pair; what is affected is the sentence a verdict
/// makes, and here the pair the note already called suspect is the pair that was kept.
///
/// **The page left this list in the seven-hundred-and-twenty-ninth session and is the one member of
/// [`AMBIGUOUS_DIVIDED_CONSENSUS`] the new rule does not flatter** (ADR 0617). A verdict is one every
/// maximal consensus reaches and these two do not concur, so it is `ambiguous` — but the division
/// here is of *width* rather than of camps: `poppler` is in both sets, and we sit 6.24%, 6.14% and
/// 5.20% of channels from `poppler`, `mupdf` and `ghostscript`, **further from every reference than
/// any two of them are from each other**. What takes the page out of `contradicted` is that
/// `{poppler, ghostscript}`'s own 4.815% spread doubles to a bound admitting us, and nothing about
/// this render improved. The cap-height measurement above is the diagnosis and it stands unchanged;
/// this group is where the page is read, whatever bucket the verdict puts it in.
///
/// The advances are untouched by it, which is the half Table 109 does state and which was checked
/// on the same rasters: `issue7580.pdf`'s ink spans 1463 device columns against 1461 and 1462,
/// `issue6108.pdf`'s 2931 against 2928 and 2929, `issue9243.pdf`'s 210 against 211.
///
/// **It stays listed and the face is not changed**, and ADR 0267 has the argument rather than this
/// comment: §9.5 NOTE 5 puts the choice beyond the standard, §9.8.1 says a descriptor's metrics
/// exist so that a processor may "synthesise a substitute font or select a similar font when the
/// font program is unavailable" and states no `shall` about doing it, and moving 0.6875 to 0.729167
/// because that is where `NimbusSans` sits is curve-fitting with the arithmetic written out.
/// `/CapHeight` is on §9.8.1's ledger row's list of Table 120 entries this tree does not read, and
/// what this measurement adds is the row's missing number.
///
/// # `issue15716.pdf` page 1 is the group's third mechanism, and it has a closed form
///
/// Measured in the four-hundred-and-forty-third session, which took it off the ranking's head:
/// **3.10 from the nearest reference against 3.92 from the furthest**, the tightest ratio on the
/// contradicted list that is not a link border, which step 1 of `doc/todo/00` reads as *we are
/// alone*. It had carried one sentence — "a grid of card suits where ours are Foxit's
/// ZapfDingbats and theirs are the machine's clone of it" — since the hundred-and-forty-eighth,
/// with no number behind it.
///
/// The page is 200 x 200 points holding a §8.7.3 tiling pattern of a 100 x 100 cell, so its
/// sixteen marks are four glyphs drawn four times: `/BaseFont /ZapfDingbats` with
/// `/Differences [1 /a109 /a110 /a111 /a112]`, at `64 0 0 64 … Tm` with `/F1 1 Tf`, two of them
/// black under one `/OC` layer and two red under another. Nothing is embedded, so
/// `pdf_font::standard` answers from `FoxitDingbats.pfb` (ADR 0133) while the three C references
/// resolve `D050000L` through this machine's fontconfig.
///
/// **The ink is arithmetic rather than an agreement**, because the placement is the document's —
/// each glyph is positioned by its own `TD` rather than by an advance — and the only unknown is
/// the outlines. Each glyph's area comes from the two font programs themselves, in font units,
/// scaled by `(64/1000)^2` and taken four times; the painted areas come from the rasters, black
/// from `(1 - mean R) x 200^2` (a red glyph leaves R at 255) and the total from the same over G:
///
/// | | black px² | red px² | total |
/// |---|---|---|---|
/// | **`FoxitDingbats`, from its own charstrings** | **6147.1** | **6373.6** | **12 520.7** |
/// | ours | 6129.2 | 6382.7 | 12 511.9 |
/// | `hayro` | 6102.3 | 6406.7 | 12 509.0 |
/// | **`D050000L`, from its own charstrings** | **8200.5** | **7081.5** | **15 282.0** |
/// | `poppler` | 8212.8 | 7081.8 | 15 294.5 |
/// | `mupdf` | 8189.5 | 7079.4 | 15 269.0 |
/// | `ghostscript` | 8188.5 | 7078.4 | 15 266.9 |
///
/// **Every renderer paints the area its own font program states, to a fifth of a percent** — ours
/// 0.07% under Foxit's total, `poppler` 0.08% over URW's — and the whole 18.1% difference is the
/// two programs' outlines. No reference is trusted anywhere in that table: both closed forms are
/// read out of the two files.
///
/// **What the two faces share is exactly what the standard states.** The advances are
/// **626, 694, 595 and 776** in *both* programs — Adobe's published ZapfDingbats metrics, which is
/// §9.6.2.1's Table 109 half honoured on both sides — and `a110`, the heart, has the same outline
/// in both to 0.2% of its area. What differs is the other three: `a109` is 8.4% narrower in
/// Foxit's face and 20.0% smaller in area, `a111` 11.4% and 24.5%, `a112` 14.2% and 28.7%. That is
/// why the red pair, which contains the shared glyph, is 10.0% apart while the black pair is 25.0%.
///
/// So this group's three mechanisms are now measured and they are three different sizes: a
/// substituted **serif** costs nothing that can be measured, a substituted **sans** costs one
/// number (`/CapHeight`, 5.7%), and a substituted **symbolic** face costs a quarter of its ink
/// while costing nothing at all in placement. All three are §9.5 NOTE 5's sentence — "some details
/// of font naming, font substitution, and glyph selection are implementation-dependent" — and the
/// third is the plainest instance of it in the file, because a dingbat *is* its outline.
///
/// # Which bound the gate fails each page on, and five of the eight are this note's own exit rule
///
/// Every figure above is ink, cap rows, charstring area or a bounding box, and none of it is in
/// the units the verdict is made of — ADR 0497's sixth criterion. Written out (ADR 0510), the
/// eight are not one shape:
///
/// ```text
///   bug847420.pdf p1           poppler+mupdf   mean 8.45/5.00, differing 9.16%/8.09%, ssim .8581/.9000
///   issue15716.pdf p1          mupdf+gs        mean 14.03/5.00, differing 9.22%/6.06%, ssim .6886/.9000
///   issue9243.pdf p1           all three       structural similarity alone, .8907/.9000
///   bug850854.pdf p1           poppler+mupdf   the differing fraction alone, 5.38%/5.00%
///   issue11403_reduced.pdf p1  poppler+mupdf   differing alone, 6.24%/5.00%
///   issue6069.pdf p1           poppler+mupdf   differing alone, 6.5550%/6.5475%
///   issue6108.pdf p1           poppler+mupdf   differing alone, 6.55%/5.75%
///   issue7580.pdf p1           poppler+mupdf   differing alone, 6.92%/5.00%
/// ```
///
/// **Five of the eight are the shape this note moved five pages *out* for.** The paragraph above
/// about `bad-PageLabels.pdf` and its four says a page failing on the differing fraction alone is
/// `CONTRADICTED_GLYPH_EDGES`' diagnosis — so the discriminator, applied as written, would empty
/// most of what is left. Whether the cap height owns those five differing fractions or the glyph
/// edges do is not answerable by any arithmetic in this comment.
///
/// # So the face was taken out of the documents, and it owns the bound on seven of the eight
///
/// `gs -sDEVICE=pdfwrite -dEmbedAllFonts=true -dSubsetFonts=false` with `<</NeverEmbed[]>>`
/// writes each page back with `ghostscript`'s own fontconfig resolution embedded as a
/// `/FontFile3`, after which every renderer draws one program and §9.5 NOTE 5's mechanism cannot
/// act. **The control is as clean as this instrument gets**: on seven of the eight files
/// `poppler`, `mupdf` and `ghostscript` render the rewritten document byte-identically to the
/// original, so the rewrite changed the font program and nothing else — and the page's bound,
/// derived from the references' distance from each other, is unchanged with it. The exceptions
/// are `mupdf` at 0.09 on `bug847420.pdf` and 0.08 on `issue11403_reduced.pdf`, and `poppler` at
/// 0.72 on `issue15716.pdf`, where `poppler` is not in the consensus and both members of it are
/// byte-identical.
///
/// Worst-ratio member of the named consensus, before and after:
///
/// ```text
///                                 ours                            with the face embedded
///   bug847420.pdf p1          8.45 / 16.01 / 9.16% / .8581     1.62 /  3.77 / 6.51% / .9928
///   bug850854.pdf p1          2.76 / 10.39 / 5.38% / .9758     1.01 /  4.26 / 4.22% / .9964
///   issue11403_reduced.pdf    2.55 /  5.32 / 6.25% / .9795     0.80 /  1.69 / 4.78% / .9975
///   issue15716.pdf p1        14.03 / 31.31 / 9.22% / .6886     3.28 / 10.55 / 4.94% / .9461
///   issue6069.pdf p1          2.41 /  5.33 / 6.62% / .9836     1.46 /  4.19 / 5.97% / .9936
///   issue6108.pdf p1          2.35 /  4.94 / 6.55% / .9791     1.48 /  3.58 / 5.89% / .9926
///   issue7580.pdf p1          2.92 /  6.93 / 6.92% / .9753     0.80 /  1.66 / 4.99% / .9978
///   issue9243.pdf p1          3.13 / 16.29 / 3.05% / .8907     0.92 /  4.49 / 1.85% / .9793
/// ```
///
/// **Seven of eight go inside every bound they were failing**, including all three of
/// `bug847420.pdf`'s and all three of `issue15716.pdf`'s, and including `issue9243.pdf`, the one
/// page in this file whose only failing measure is structural similarity. So the answer to the
/// five-page question above is the group's name and not the edges'.
///
/// **`issue6108.pdf` is the eighth and it carries two mechanisms.** The face owns 0.66 of the 0.80
/// points by which it misses its differing bound — 82% — and the 5.89% left against 5.75% is a
/// sub-pixel glyph-edge population that would keep the page contradicted with the substitution
/// entirely removed. That is trap 9's *a page can carry two of the eight*, in a group that is not
/// `visibility_expressions.pdf`; it is recorded rather than moved, because a mechanism owning 82%
/// of a bound is this group's and the second one is named beside it.
///
/// `issue7580.pdf` clears its bound by **0.0125 of a percentage point**, stated at the precision
/// it was measured. Five of these pages sit within a point of a bound in one direction or the
/// other, which is what a 200 × 50 line of text does to a metric that counts channels.
///
/// **And `issue6069.pdf` is now the tightest verdict in the whole contradicted pool: six channels
/// of eighty thousand.** The seven-hundred-and-twenty-second session found it by asking every
/// contradicted page which of the four bounds it fails on and getting *none* for this one — the
/// gate prints `differing 6.55%` against `bound … differing 6.55%`, identical at the two decimals
/// it writes, so its own line can no longer say what the verdict rests on (`--bin unpriced`,
/// ADR 0606). Taken from this run's artefacts at the precision `examples/compare_rasters` prints:
/// `poppler` against `mupdf` is **3.2738%** of channels, so the bound is **6.5475%**, and ours
/// against `poppler` is **6.5550%**. The raster is 400 × 50, which is 80 000 channels, so that is
/// **5244 differing against an allowance of 5238** — the six channels being 0.11% of the bound.
/// The note's own row above read 6.62% until this session, and ours moved to 6.55% somewhere in
/// the forty rounds after ADR 0510 measured it; what did not move is the ablation's answer, since
/// the embedded face takes the page to 5.97% against the same 6.55% and inside it. **A page held
/// contradicted by six channels is still contradicted** — the arithmetic is the arithmetic, and
/// trap 12 is about reading such a margin as a statement about the page rather than about the
/// pair. It is recorded here so that the next round to open this row is not looking for a figure
/// the printed line no longer distinguishes.
///
/// # Four more from a second corpus, and the cap-height constant predicted the line
///
/// `pdfbox/PDFBOX-2984-rotations.pdf` pages 1 to 4 are the first pages this group has taken from
/// outside the pdf.js corpus (ADR 0541). The document is six pages of one line of 50 pt
/// `/Helvetica` with `/Encoding /WinAnsiEncoding`, nothing embedded and nothing else on the
/// sheet, drawn at each of `/Rotate` 90, 180 and 270 once through a text matrix and once through
/// a `cm` — so it is the sans case with the page reduced to it.
///
/// **The instrument is the table above's and the number it predicts arrived.** The capital `A` at
/// 8× on page 1, `render_at` against `pdftoppm -cropbox -r 576` and `mutool draw -r 576`:
///
/// | | box | cap rows | ratio to the references' |
/// |---|---|---|---|
/// | ours | 359 × **358** | 358 | 358 / 379 = **0.9446** |
/// | `poppler` | 353 × **379** | 379 | — |
/// | `mupdf` | 353 × **379** | 379 | — |
///
/// 0.6875 / 0.729167 is **0.942857**, so the prediction is 357.3 rows and the measurement is 358
/// — within 0.7 of a row, on a page nobody had rendered when the constant was derived. Ink at the
/// page's own scale runs 8.2% under, which is `issue9243.pdf`'s pure case again and for the same
/// reason: at 50 pt this page is nearly all capitals.
///
/// **The 8.2% is not evenly spread, and where it sits is the section after next.** Ours is 4515.78
/// px² of ink on page 1 against `poppler`'s 4917.91; **175.5 of the 402.1 px² missing is the one
/// `registered` sign**, which is 44% of the deficit from one glyph in fifteen, and the other 217.7
/// is the cap height over the rest of the line. That second half fails no bound at all.
///
/// **And the advances are not what differs**, which is worth stating because the four pages'
/// *centroid* moves 5.5 device pixels and reads like a shifted line. The ink's bounding box at
/// the page's own scale is **420 × 86 at (100, 64)** in ours and **420 × 87 at (101, 63)** in
/// `poppler` — the same width to the pixel over a 420-column line, so §9.2.4's widths agree and
/// what moves the centroid is where the missing weight sits along the line rather than where the
/// glyphs are.
///
/// **Only four of the six pages are here, and the other two carry the same deficit.** Pages 5 and
/// 6 measure 1 150 087 and 1 128 745 against `poppler`'s 1 256 804 and 1 234 906, the same 8.5%,
/// and the gate calls them `agrees`. ~~because their consensus pair happens to sit further apart
/// and the bound derived from it is wider~~ — that clause was measured in the
/// eight-hundred-and-twenty-sixth session and is false in both halves; the section below has what
/// is true instead. What survives it is the reason it was written: **this group's membership is a
/// measurement and never a verdict.**
///
/// # The cap height is not the bound these four fail, and one glyph is
///
/// Everything above is ink and cap rows, and none of it is in the units the verdict is made of —
/// ADR 0497's sixth criterion, which ADR 0688 sharpens into *a mechanism is only priced when it is
/// priced in the measure the row is ranked on*. These four pages fail on the **worst tile and
/// nothing else**: 62.57 of 56.81 on pages 1 and 2, 62.01 of 56.57 on pages 3 and 4, with the mean
/// at 0.63 of 5.00, the differing fraction at 0.57% of 5.00% and the structural similarity at
/// 0.9905 of 0.9000. A cap height spread along a line of fifteen glyphs is not a *localised*
/// quantity, and the worst tile is the only localised measure the gate has.
///
/// **The worst tile is one glyph, and `raster_compare` says which by saying where.** On all four
/// pages, ours against every reference *and* between every pair of references, it is the same
/// 32-pixel tile: `(480, 64)`. What is there is the line's last glyph — the `registered` sign,
/// code `AE` under `/WinAnsiEncoding`, the shown string being
/// `<41706163686520504446426F7820AE>`. Ours against `ghostscript` on page 1, by tile, summed in
/// level-pixels and then over the tile's own 1024 pixels:
///
/// ```text
///   (448, 64)      5 514       5.38
///   (480, 64)     64 072      62.57      <- the registered sign
///   (512, 64)      8 740       8.53
/// ```
///
/// The page's whole difference is 317 158 level-pixels and 64 072 of it is in that one tile, whose
/// 62.57 is therefore the `worst tile 62.57` the gate's own line prints for this page.
///
/// **And the glyph is each font program's own area, which is a closed form rather than an
/// agreement.** The net outline area of `registered` read out of the two files themselves, with no
/// renderer in it: `LiberationSans-Regular.ttf`, which `pdf_font::standard` answers `/Helvetica`
/// with, states **664 570.5 units² over a 2048 em** — 0.158 445 em²; `NimbusSans-Regular.otf`,
/// which the three C references resolve through this machine's fontconfig, states **228 762.3 over
/// a 1000 em** — 0.228 762 em². At 50 pt those are 396.11 px² and 571.91 px², against the ink each
/// renderer paints over the glyph's own box (`-alpha off -channel R`, at the page's own scale and
/// at 576 dpi):
///
/// | | its own program says | 72 dpi | 576 dpi |
/// |---|---|---|---|
/// | ours, `LiberationSans` | **396.11** | 393.13 | **395.73** |
/// | `poppler`, `NimbusSans` | **571.91** | 568.60 | **572.08** |
/// | `mupdf`, `NimbusSans` | **571.91** | 569.83 | **572.06** |
/// | `ghostscript`, `NimbusSans` | **571.91** | 574.71 | — |
///
/// **Every renderer paints the area its own font program states, to a tenth of a percent at eight
/// times the resolution** — `issue15716.pdf`'s ZapfDingbats result reproduced on a second face and
/// a single glyph. The advance is not what differs and that is the half the standard does state:
/// Adobe's published Helvetica width for `registered` is **737**, `standard_metrics.rs` answers it,
/// and both faces' own advances are that width to a thousandth of an em (1509/2048 = 0.7368 against
/// 737/1000 = 0.7370). What the two programs disagree about is the drawing, which is §9.5 NOTE 5's
/// "some details of font naming, font substitution, and glyph selection are
/// implementation-dependent" with a number under it. The `registered` sign is 30.7% of ink short
/// where the cap height is 5.7%, so the constant above predicts the *line* and does not predict
/// this glyph.
///
/// **The mechanism owns the bound, by taking the glyph out of the document.** A §7.5.6 incremental
/// update rewriting each content stream's `20AE` as `2020` — the registered sign replaced by a
/// space, nothing else touched — and all four renderers re-run at 72 dpi with the invocations
/// `pdfref::Reference::build_command` states. The bound is twice the widest worst tile inside the
/// consensus or the text class's floor of 40.00, whichever is larger:
///
/// ```text
///                  ours at worst   widest inside the consensus   the bound
///   page 1  ships      62.57                 28.40                 56.81   contradicted
///           ablated    38.27                 22.66                 45.32   inside every bound
///   page 3  ships      62.01                 28.28                 56.57   contradicted
///           ablated    39.93                 17.11                 40.00   inside every bound
/// ```
///
/// Pages 2 and 4 are their neighbours to the hundredth. Note that the ablation *tightens* the bound
/// on pages 3 and 4, because the references agree more closely once the glyph is gone and the floor
/// takes over — and the page is inside it anyway, by 0.07 of a level, stated at the precision it was
/// measured rather than rounded into comfort.
///
/// # Why pages 5 and 6 agree is the tile grid, and this note said it was the bound
///
/// The sentence struck through above blamed the references: *their consensus pair happens to sit
/// further apart and the bound derived from it is wider.* Measured with the same invocations, the
/// widest pair inside the consensus on pages 5 and 6 is **25.33** against page 1's **28.40**, so
/// they sit **closer** and their bound is **narrower** — 50.66 against 56.81. Both halves of the
/// sentence are wrong. What differs is our own number: **35.32 against 62.57**, on a page carrying
/// the same face and the same glyph.
///
/// **All of that is where the glyph falls on a grid fixed to the raster's origin.** The registered
/// sign occupies device columns 484 to 519 on page 1 and 526 to 561 on page 5, so on page 1 it is
/// 28 of its 36 columns inside the tile at `x = 480`, and on page 5 it is split 18 and 18 across
/// the tiles at 512 and 544. Ours against `ghostscript`, in level-pixels:
///
/// ```text
///                        over the glyph's own columns   worst tile
///   page 1  (480, 64)              75 004                 64 072   -> 62.57
///   page 5  (512, 64)              78 212                 36 170   -> 35.32
///           (544, 64)                                     28 670
/// ```
///
/// **The same glyph, the same difference to four percent, and the measure a factor of 1.77 apart**
/// — one page contradicted and one agreeing on where a 32-pixel grid happened to fall. That is a
/// property of the instrument rather than of either page; it is
/// `doc/traps/oracle-and-references.md` trap 26, `raster_compare::DEFAULT_TILE`'s own doc comment
/// carries it beside the constant, and a unit test pins the halving. ADR 0755.
///
/// # Four of these verdicts rest on the pair that shares a glyph rasteriser
///
/// `issue6069.pdf`, `issue6108.pdf`, `issue7580.pdf` and `bug850854.pdf` are convicted on the
/// differing fraction by `poppler` and `mupdf` alone, and they are members of the population
/// [`CONTRADICTED_GLYPH_EDGES`]'s last section measures whole: on all four, `ghostscript` —
/// the voting reference with its own statically linked `FreeType` — fails the same
/// differing-fraction bound against both members of the convicting pair, and on `issue7580.pdf`
/// that pair agrees to an exact 0.00% on the count while `ghostscript` sits at 5.50%
/// (`examples/compare_rasters` over the gate's artefacts, not the gate's line). The
/// substitution is this group's mechanism; the *bound* those four verdicts rest on is that
/// section's, and ADR 0717 is the measurement.
///
/// # `bug847420.pdf` page 1 is one of three pages in the pool where a reference outside the
/// consensus meets the bound, and on it the three references are **one face**
///
/// The gate names that population now (`name_the_pages_the_excluded_reference_survives`, ADR
/// 0772), and this page is its head: the consensus is `poppler` and `mupdf`, and `ghostscript` —
/// the voting reference it excludes — is inside every one of the four bounds our own render is
/// held to (mean 3.48 of 5.00, worst tile 7.21 of 40.00, differing 7.53% of 8.09%, ssim 0.9681 of
/// 0.9000 at its worst against either member, by `examples/compare_rasters` over the gate's
/// artefacts because the gate prints no line for a reference) where we fail three. On the face of it that
/// is the sharpest accusation this pool can produce, because `ghostscript`'s `FreeType` is its own
/// statically linked copy and trap 9's usual answer — *the pair shares a glyph rasteriser* — does
/// not reach it.
///
/// **It does not survive asking what the three references are reading.** Every one of them draws
/// the *same substituted design*, which is measurable without opening a font at all: at 8× through
/// `render_at` against `pdftoppm -cropbox -r 576`,
/// `mutool draw -r 576` and `gs -dUseCropBox -r576`, the line's ink bounding box is device columns
/// **98 to 1515 in all three of them** and the capital `T` is **82 device rows in all three**,
/// where ours is columns 90 to 1509 and 77 rows. Three programs agreeing to the pixel over 1420
/// columns and to the row on a cap height is one face, not three readings; ours is Liberation
/// Sans's 0.687500 em against `NimbusSans`'s, which is §9.8.1's row and ADR 0267.
///
/// **The face is named, and by the programs rather than by inference where that was available.**
/// `ghostscript` without `-q` says it outright — *Loading font Arial,Italic (or substitute) from
/// /usr/share/ghostscript/Resource/Font/NimbusSans-Italic*, a 120 927-byte Type 1 program in its
/// own `Resource` tree, reached because `Fontmap.GS` maps `/Arial,Italic` to `/Arial-ItalicMT`
/// and nothing maps that. `fc-match Helvetica:italic` on this machine answers
/// `/usr/share/fonts/gsfonts/NimbusSans-Italic.otf`, 95 244 bytes, which is where `poppler` goes
/// once it has taken `Arial` for the base-14 Helvetica. `mupdf`'s route was not asked and does not
/// need to be: whatever it reads, the raster says it is this design.
/// **The two files that were named are not the same file** — different formats, different lengths,
/// different digests — so no dependency graph, digest comparison or `desc` tag finds this, which
/// is trap 9's *sixth* bullet exactly: implementations agree because each independently went and
/// got a copy of the same published design. The tell is a rendering metric, and the tell is what
/// the paragraph above measures.
///
/// It is worth saying which bullet it is **not**. Trap 9's font entries are about
/// `libfreetype.so.6`, the rasteriser *object*, and [`CONTRADICTED_GLYPH_EDGES`]' note says in as
/// many words that sharing a rasteriser is not what makes two references agree. This is one level
/// further out: the three do not share the object here — `ghostscript`'s `FreeType` is its own
/// statically linked copy — they share what they hand it.
///
/// **And the specification puts the choice beyond itself twice over.** §9.5 NOTE 5 is quoted at
/// the head of this note — the results "depend on the availability of fonts in the PDF processor's
/// environment" — and §9.8.1's route is blocked by this file's own descriptor: it states
/// `/CapHeight 500` beside `/Ascent 728` and a `/FontBBox` reaching 998, so a processor that did
/// scale a substitute to the stated cap height would draw capitals **27% shorter than ours and
/// 31% shorter than the references'**. Nobody honours it, ours included. **`/CapHeight` unread is
/// what ADR 0267 decided and this page is its third witness**, after `issue7580.pdf`'s zeroes and
/// `bug1671312_ArialNarrow.pdf`'s 922: the entry is stated and unusable, on the very page whose
/// cap-height deficit §9.8's row prices.
const CONTRADICTED_SUBSTITUTED_FONT: [&str; 11] = [
    "bug847420.pdf page 1",
    "bug850854.pdf page 1",
    "issue15716.pdf page 1",
    "issue6069.pdf page 1",
    "issue6108.pdf page 1",
    "issue7580.pdf page 1",
    "issue9243.pdf page 1",
    "pdfbox/PDFBOX-2984-rotations.pdf page 1",
    "pdfbox/PDFBOX-2984-rotations.pdf page 2",
    "pdfbox/PDFBOX-2984-rotations.pdf page 3",
    "pdfbox/PDFBOX-2984-rotations.pdf page 4",
];

/// Pages that are almost entirely glyph edges, where our *ink* matches the consensus.
///
/// Eight pages, measured in the seventy-fifth session, and they are one population rather than
/// eight questions. Structural similarity is the bound that "does the work on text"
/// (`pdfref::Tolerance`), and it says the same shapes are in the same places.
///
/// # This entry named the wrong bound for three hundred and thirty sessions
///
/// It opened "[e]ach fails **only** on mean absolute difference — 5.4 to 6.4 against a bound of
/// 5.00 — while every other measure passes with room: worst tile 13.7 to 22.1 against 40, and
/// structural similarity 0.904 to 0.946 against 0.900." **Every number in that sentence was
/// `ghostscript`'s, and `ghostscript` is in the consensus on none of the 21 pages it then held**
/// — all of them read "poppler and mupdf agree". The gate's own line was measuring us against whichever
/// reference had the largest tile, which need not be one the verdict rests on, and printing it
/// beside a bound derived from the pair that does; `measurements` is where that was corrected
/// in the four-hundred-and-sixth session, and this group is the largest thing it moved.
///
/// Against the pair that decides them, all 21 looked like this:
///
/// | | across the 21 | its bound |
/// |---|---|---|
/// | mean | 1.01 to 2.57 | 5.00 |
/// | worst tile | 2.67 to 9.53 | 40.00 |
/// | structural similarity | 0.9655 to 0.9981 | 0.9000 |
/// | **differing fraction** | **1.00× to 1.56× its own page bound** | 5.00% to 8.75% |
///
/// **Every one of them fails on exactly one bound and it is the differing fraction**, which
/// `Tolerance::accepts` has always checked and which nothing printed until that session. The
/// mean — the measure this entry was built on — is a fifth to a half of its bound on every page.
///
/// **The diagnosis survives and gets sharper, which is why the correction is worth more than
/// the error.** `differing_fraction` counts channels that moved by more than four levels of
/// 255 — not channels that moved *at all*, which is what this sentence said until the
/// four-hundred-and-seventh session, and not pixels either; `mean_error` weighs how far. A
/// glyph drawn at a different sub-pixel phase moves every pixel of every outline by a little,
/// which is a large count and a small average — so this population failing the count and
/// nothing else is the arithmetic form of "the ink is right and its placement inside the pixel
/// is not". The ink table below was already saying it; the bound was saying it too and was not
/// being read.
///
/// # And the bound they fail is the one of the four that sits below the references' own spread
///
/// Measured in the four-hundred-and-seventh session by
/// [`the_fixed_bounds_against_the_references_own_spread`], which is the derivation
/// `Tolerance::TEXT_HEAVY` claims and nothing had re-run. Over **2638** pairs of the three
/// independent references on text pages — each measure taken over the pairs the *other* three
/// bounds admit, so that a bound is not measured over the population it already defines — the
/// share of reference pairs each bound rejects is:
///
/// | bound | its value | reference pairs it rejects |
/// |---|---|---|
/// | mean | 5.00 | 0.0% |
/// | worst tile | 40.00 | 1.2% |
/// | structural similarity | 0.9000 | 0.5% |
/// | **differing fraction** | **5.00%** | **29.4%** |
///
/// Our 5.07% to 10.58% on these 21 pages sits between that population's median (1.69%) and its
/// 99th percentile (12.02%). **So the verdict on them is a statement about the consensus pair
/// being unusually close**, which is trap 12, and not about the marks — exactly what the ink
/// table and the twice-drawn glyph already said by two other routes. The bound is not moved:
/// ADR 0243 has the derivation, what moving it would cost, and why the measurement that would
/// justify moving it for us alone comes from a renderer sharing `skrifa` with us.
///
/// `franz_2.pdf` was the same shape one group over and is this file's tightest instance of it —
/// **5.01% against 5.00%**, one part in five hundred — and the four-hundred-and-thirty-first
/// session moved it into this group along with four more (below).
///
/// The measurement that turns that into a diagnosis is the *ink*: the mean luminance of the whole
/// page, which counts how much was painted without caring where. Over the oracle's own artefacts:
///
/// | page | ours | `poppler` | `mupdf` | `ghostscript` |
/// |---|---|---|---|---|
/// | `bug1175962.pdf` | 229.07 | 229.30 | 229.25 | 231.26 |
/// | `issue6889.pdf` | 236.63 | 236.59 | 236.58 | 233.90 |
/// | `bug894572.pdf` | 241.41 | 241.36 | 241.35 | 243.20 |
/// | `issue8570.pdf` | 220.50 | 220.52 | 220.51 | 221.08 |
/// | `bug1200096.pdf` | 240.72 | 240.70 | 240.67 | 238.40 |
/// | `bug1108301.pdf` | 233.27 | 233.68 | 233.60 | 235.86 |
/// | `issue2017r.pdf` | 236.85 | 236.84 | 236.82 | 236.88 |
/// | `openoffice.pdf` | 228.19 | 228.09 | 228.08 | 227.92 |
///
/// **We are within half a level of the two references the gate votes with on every one of them**,
/// and on five of the eight `ghostscript` — which links the same `libfreetype` as the other two —
/// is further from them than we are. So the amount of ink is right and its *sub-pixel placement*
/// is what differs, which is precisely what a mean absolute difference measures and what neither
/// tile nor structure can see.
///
/// Trap 9 is the other half of the explanation and it is why the bound stays at 5.00: on a page
/// whose difference is a letter's edges, `poppler`, `mupdf` and `ghostscript` are one glyph
/// rasteriser, so their mutual distance is small and widening buys nothing. `Reference::
/// independence` records that sharing and `Tolerance::widened_to` has carried the argument since
/// the forty-first session, acted on nowhere — this group is the first thing to act on it, and it
/// acts by *naming* rather than by loosening a number. A page whose ink stopped matching would
/// leave this group by failing the ratchet.
///
/// `bug1175962.pdf` was measured on its own in the sixty-eighth session and reached the same
/// conclusion by a different route; the other seven were sitting in `CONTRADICTED_UNEXPLAINED`
/// claiming to be unexplained. Reproducing the table: the oracle leaves every renderer's PNG
/// under `<target>/tmp/oracle/<stem>/p1/`, and the mean of its RGB channels is the number above.
/// # The ninth page, and the instrument the page supplied itself
///
/// `issue7696.pdf` joined this group in the hundred-and-fortieth session, and what put it here
/// is stronger than the ink table above. The page is 200x50 and draws the same four glyphs
/// **twice**, 80 pixels apart. So it answers a question about a renderer that no comparison
/// between two renderers can: are the two copies the same picture?
///
/// | | the two halves differ by | worst pixel |
/// |---|---|---|
/// | `poppler` | 0 | 0 |
/// | `mupdf` | 0 | 0 |
/// | `ghostscript` | 0 | 0 |
/// | ours | 2 893 | 64 |
/// | `hayro` | 3 541 | 12 |
///
/// **The three C renderers draw the same glyph identically at both positions; the two Rust ones
/// do not.** That is grid-fitting: `poppler`, `mupdf` and `ghostscript` share `FreeType` *and its
/// hinting*, which snaps each glyph to the pixel grid, so the second copy lands on the same
/// samples as the first. This tree places a glyph where §9.4.4's text rendering matrix puts it,
/// so the second copy is at a different sub-pixel phase and rasterises differently — and its ink
/// is conserved to 0.67%, which is what a phase shift does and what a wrong glyph does not.
/// `hayro`, the other renderer here that does not hint, differs from itself by more than we do.
///
/// This is the same conclusion the ink table reaches, arrived at without comparing anything to
/// anybody: **a page that draws one glyph twice at different sub-pixel phases measures whether a
/// renderer grid-fits, and needs no reference at all.**
///
/// # Eleven more in the hundred-and-sixty-first session, and the same two instruments
///
/// `CONTRADICTED_UNEXPLAINED` had thirteen members and eleven of them are this. The method is
/// session 75's, applied to a list rather than to a page: **look at the heatmap's shape, then
/// measure the ink.** Every one of the eleven heatmaps is glyph outlines and nothing else — no
/// fill, no image, no edge of any shape that is not a letter — and the ink is conserved:
///
/// | page | ours | `poppler` | `mupdf` | `ghostscript` | `hayro` |
/// |---|---|---|---|---|---|
/// | `bug1151216.pdf` | 15.89 | 15.87 | 15.89 | 15.99 | 15.68 |
/// | `bug1252420.pdf` | 10.37 | 10.20 | 10.23 | 10.53 | 10.20 |
/// | `issue3207r.pdf` | 21.82 | 21.82 | 21.88 | 21.63 | 22.12 |
/// | `issue3405r.pdf` | 21.97 | 22.26 | 22.27 | 21.97 | 22.29 |
/// | `issue3694_reduced.pdf` | 13.11 | 12.85 | 13.01 | 13.27 | 13.13 |
/// | `issue4061.pdf` | 14.92 | 14.65 | 14.66 | 15.28 | 14.64 |
/// | `issue4650.pdf` | 14.94 | 14.88 | 14.93 | 15.18 | 14.89 |
/// | `issue5010.pdf` | 13.10 | 13.11 | 13.13 | 13.49 | 12.89 |
/// | `issue7492.pdf` | 20.28 | 20.06 | 20.19 | 20.58 | 20.02 |
/// | `issue7901.pdf` | 26.21 | 26.14 | 26.25 | 26.51 | 25.72 |
/// | `issue8097_reduced.pdf` | 18.69 | 18.56 | 18.62 | 18.69 | 18.61 |
///
/// **We are within 0.3 of every reference on every page, and the three references span as much
/// among themselves.** Ink that is conserved with the difference confined to glyph outlines is
/// this group's whole diagnosis, and these eleven meet it as squarely as the original eight.
///
/// What is different is only the bound that catches them: the eight fail on mean absolute
/// difference against the class bound of 5.00, and these fail on the *tightened* one two closely
/// agreeing references produce (trap 12) — mean 1.09 to 4.19, all inside 5.00.
///
/// **And the instrument had to be checked before it could be believed.** The first measurement
/// put our ink at exactly half the three C renderers' on ten pages running, a ratio of 2.00 to
/// three significant figures — which is not what hinting does and is what a broken instrument
/// does. Our renders and `hayro`'s carry an alpha channel and the three C ones do not, and
/// `magick -colorspace Gray` was averaging alpha in as a fourth channel. `-alpha off -channel R`
/// is the measurement above. A suspiciously clean number is a reason to check the instrument,
/// and the tell here was that the two renderers agreeing with us were the two whose output
/// format matched ours.
///
/// # A twenty-first, from `CONTRADICTED_UNEXPLAINED`, and the closed form is what moved it
///
/// **`freeculture.pdf` page 313** is one page of the book whose other three hundred are
/// `ambiguous` under `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE`. It sat unexplained because nothing had
/// been measured on it; the two-hundred-and-forty-second session measured it, and the two
/// ladders answer it outright:
///
/// ```text
///          the 242nd session      re-run in the 845th
///            1x       8x            1x       8x
/// ours     5.9139   6.0729        5.854    5.993
/// poppler  6.0271   6.0658        5.943    5.983
/// mupdf    6.0725   6.0819        6.013    6.019
/// ```
///
/// (Both columns are ink in levels of 255, `-alpha off -channel R`, `render_at` against
/// `pdftoppm` and `mutool draw` at 72 and 576 dpi. The absolute figures moved — this tree's own
/// scan conversion changed under them, ADR 0476 among the reasons — and the *reading* did not:
/// ours at 8× is between the two references that vote in both columns.)
///
/// **Ours at eight times the resolution is inside the references' own spread**, which is this
/// group's diagnosis exactly: the marks are the right marks and the difference is glyph coverage
/// at the page's own scale.
///
/// **The three figures this paragraph then quoted from the gate went stale, and the sentence
/// after them was backwards.** It said "[e]very printed metric is *inside* the class bound — mean
/// 2.56 against 5.00, worst tile 12.54 against 40.00, ssim 0.9445 against 0.9000 — and the page is
/// contradicted only because `poppler` and `mupdf` agree so closely that twice their spread is a
/// tighter bound than the floor". The gate prints mean **1.88**, worst tile **9.54**, ssim
/// **0.9685** and differing **6.05%** against a bound of **6.01%** — so the failing measure is one
/// the sentence did not mention, and the widening on this page runs the other way: twice the
/// pair's 3.00% is 6.01%, which is *looser* than the 5.00% class floor, and the page is
/// contradicted with a bound the consensus **widened**. The ladder was re-run in the
/// eight-hundred-and-forty-fifth session and its conclusion is unchanged while its numbers moved
/// with the rasteriser — ours 5.854 → **5.993** against `poppler` 5.943 → **5.983** and `mupdf`
/// 6.013 → **6.019**, which puts us between the two that vote at 8×. ADR 0772, and the last
/// section of this note is where the page's verdict is accounted for.
///
/// # Five more in the four-hundred-and-thirty-first, and every one had a *substituted* face
///
/// `bad-PageLabels.pdf` page 1, `franz_2.pdf` page 1 and `issue8088.pdf` pages 1 to 3 sat in
/// `CONTRADICTED_SUBSTITUTED_FONT`, admitted together on that group's membership rule — the page
/// names a font nobody embedded — and never opened. All five name `/Times-Roman`, so the rule is
/// true of them; it is not what they differ by, which is the seventh time this file has caught a
/// group's name naming a hypothesis rather than a diagnosis.
///
/// The two instruments are this group's own. **The ink is conserved**, in levels of 255 at the
/// page's own scale:
///
/// | page | ours | `poppler` | `mupdf` | `ghostscript` | `hayro` |
/// |---|---|---|---|---|---|
/// | `bad-PageLabels.pdf` | 9.440 | 9.449 | 9.495 | 9.654 | 9.344 |
/// | `franz_2.pdf` | 115.278 | 115.285 | 116.251 | 115.380 | 115.222 |
/// | `issue8088.pdf` p1 | 12.966 | 12.870 | 12.919 | 12.997 | 12.951 |
/// | `issue8088.pdf` p2 | 13.247 | 13.095 | 13.140 | 13.237 | 13.164 |
/// | `issue8088.pdf` p3 | 13.188 | 13.010 | 13.055 | 13.157 | 13.100 |
///
/// **And the marks are in the same box.** At 8× the ink's bounding box is 1233 × 143 at (84, 133)
/// in ours, `poppler`'s and `mupdf`'s alike on `issue8088.pdf` — identical, over a 1600-column
/// raster — and 1442 against 1441 columns on `bad-PageLabels.pdf` and 1437 against 1436 on
/// `franz_2.pdf`, with the same height and the same origin in every one. Equal *height* is the
/// part worth naming: it says the compiled-in `FoxitSerif` and the machine's `NimbusRoman` have
/// the same cap height to the device pixel, which is exactly what the sans half of the group next
/// door does *not* have. So the substitution is real and is not the difference.
///
/// Each of the five fails one bound and it is the differing fraction — `franz_2.pdf` 5.01%,
/// `bad-PageLabels.pdf` 5.33%, `issue8088.pdf` 5.50%, 5.57% and 5.68%, against 5.00% — with mean
/// at most 1.64 of 5.00, worst tile at most 5.05 of 40.00 and structural similarity at worst
/// 0.9906 of 0.9000. The heatmaps are hollow letters and nothing else. That is this group's
/// signature in all three of its instruments at once.
/// # A twenty-seventh, from `pdfbox`, and the ladder is the whole of the argument
///
/// `pdfbox/unencrypted.pdf page 2` (ADR 0541). Both its fonts are **embedded** — a subset
/// `ArialMT` and a subset `CourierNewPS-ItalicMT`, both `Identity-H` — so the group next door's
/// membership rule cannot reach it and nothing about a substituted face is in play. The two
/// ladders answer it, in levels of 255 over the red channel (`-alpha off -channel R`, for the
/// reason the instrument note above gives):
///
/// ```text
///            1x       8x
/// ours     6.0086   6.2690
/// poppler  6.2305   6.2613
/// mupdf    6.0988   6.2542
/// ```
///
/// **At eight times the resolution the three agree to 0.015 of 255 and ours is inside their
/// span**, 0.008 above `poppler` and 0.015 above `mupdf`; at the page's own scale we are 0.22
/// under `poppler` and 0.09 under `mupdf`. That is this group's diagnosis stated in its own
/// instrument: the marks are the right marks and what the gate is measuring is glyph coverage at
/// the page's own scale. `ghostscript` is at **10.93** on the same page and draws the italic
/// Courier visibly bolder, which is why it is not in the consensus and why the pair's own spread
/// tightens the bound (trap 12).
///
/// **The page carries a second mechanism and it is named rather than folded in.** Its heatmap is
/// hollow letters *and* one-pixel edges around the yellow table cells, which is
/// `CONTRADICTED_ANTIALIASED_EDGES`' subject — empty since ADR 0476 made a rectangle's coverage
/// the exact closed form. What is established here is the ladder above; that the rectangle edges
/// contribute nothing further is not, and trap 9's *a page can carry two of the eight* is the
/// reason to write that down instead of assuming it.
///
/// # The convicting pair shares a glyph rasteriser, and the third voting reference fails the bound
///
/// Every page of this group is convicted on the differing fraction by `poppler` and `mupdf`
/// alone, and those two are the one voting pair that hints its glyphs through a single
/// rasteriser: `pdftoppm` and `mutool` each load the machine's `libfreetype.so.6` where `gs`
/// links no `FreeType` and carries its own statically linked copy — `objdump -p` on this
/// machine's binaries, re-checked in the seven-hundred-and-eightieth session rather than
/// inherited from trap 9's bullet. The gate's ranking prints the count of such convictions
/// every run since that session.
///
/// The measurement is `examples/compare_rasters` over the gate's own artefacts — one named
/// pair per row, cropped top-left to the common size where the panels differ, so these are
/// that instrument's figures and not the gate's line. Over the 32 pages the gate convicts on
/// the differing fraction with that pair (these 27, four of `CONTRADICTED_SUBSTITUTED_FONT`'s,
/// and `CONTRADICTED_SUBPIXEL_IMAGE`'s one), the differing fraction of:
///
/// - **the convicting pair runs 0.00% to 4.37%, median 2.33%** — on `issue4061.pdf`,
///   `issue7580.pdf` and `issue7696.pdf` it prints an exact 0.00%, two programs agreeing to
///   the count's own zero;
/// - **every pair containing `ghostscript` runs 5.32% to 13.37%, median 6.8%** — the two
///   distributions do not overlap, so on every one of the 32 pages the only two renderers
///   inside the class floor of each other are the two hinting through one `FreeType`;
/// - **ours, best against a pair member, is median 5.70% against `ghostscript`'s 6.75%** —
///   the third voting reference fails the bound these verdicts rest on against *both* members
///   of the pair on **31 of 32** pages, and sits further outside it than we do on 27 of the 32.
///
/// That is the control trap 12 asks for, taken over the population instead of a page: put
/// `ghostscript` where our render stands and the same consensus contradicts it on nearly every
/// page of this group. It is **not** evidence that our phases are right — agreement and its
/// absence run in one direction only — and no bound moves on it (`doc/todo/12` says what moving
/// one costs). What it establishes is that these verdicts rest on a bound derived from the one
/// pair whose agreement trap 9's tenth mechanism manufactures, and that a voting reference
/// with its own rasteriser cannot meet that bound either. ADR 0717.
///
/// **The two figures above read "32 of 32" and "on every page" until the
/// eight-hundred-and-forty-fifth session, and the gate counts them now** rather than quoting the
/// ADR, which is how the exception was found: the printed line under
/// [`rank_the_contradicted_by_the_bound`] takes the count off `ExcludedReading` every run. ADR
/// 0772.
///
/// # `freeculture.pdf` page 313 is the exception, and it is where the bound falls inside one
/// continuous spread
///
/// It is the one page of the 32 on which `ghostscript` is **inside** the bound — 5.32% against
/// `poppler` and 5.35% against `mupdf`, where the bound is 6.01% — while ours is 6.05% against
/// `poppler` and misses by 0.04 of a percentage point. Every other measure is inside with room:
/// mean 1.88 of 5.00, worst tile 9.54 of 40.00, ssim 0.9685 of 0.9000, so the differing fraction
/// is the whole verdict. It is therefore also one of the three pages in the whole pool where the
/// voting reference the consensus excludes meets the bound we do not, which the gate names.
///
/// **What the page is** is one leaf of a book whose other three hundred are `ambiguous`, and its
/// diagnosis is this group's: `examples/render_at` at 1× and 8× against `pdftoppm` and
/// `mutool draw` at 72 and 576 dpi, ink in levels of 255 with `-alpha off -channel R`, gives ours
/// 5.854 → **5.993** against `poppler` 5.943 → **5.983** and `mupdf` 6.013 → **6.019**, so at
/// eight times the resolution we are *between* the two references that vote and the difference at
/// the page's own scale is glyph coverage. The marks are the right marks.
///
/// **And what the verdict is** is where a threshold fell in a spread with no gap in it. The five
/// cross-pair differing fractions that do *not* define the bound run **5.32%, 5.35%, 5.88%, 6.05%
/// and 6.15%** — `ghostscript` against each pair member, ours against each pair member, and ours
/// against `ghostscript` — and the bound derived from the sixth, the pair's own 3.00%, lands at
/// 6.01%, inside that range and 0.13 points from the top of it. Nobody is on one side of a
/// boundary the page states; `ghostscript` is 0.69 points inside a cut and we are 0.04 outside it.
/// That is trap 12's mechanism at its purest — a bound that is a *selected minimum* rather than a
/// spread — and it is why this page's membership of the excluded-reference population is not an
/// accusation. ADR 0772.
///
/// # And *32 of 32* is the pool's base rate, which is what the population could not say
///
/// The eight-hundred-and-forty-fourth session ran the same control over the **whole**
/// contradicted pool — [`the_excluded_reference_under_the_same_bound`], counted by
/// [`rank_the_contradicted_by_the_bound`] every run — and it holds on **52 of the 60** pages,
/// across the JBIG2 pages, the `CalRGB` pages, the CMYK shading pages and the link border alike.
/// So it is not this group's signature, it discriminates nothing, and no verdict rule can rest
/// on it. **This group's diagnosis is untouched by that**, and the reason is worth being clear
/// about: it never rested on the control. It rests on three instruments that read no bound at
/// all — the ink is conserved to a third of a level where the three references span as much
/// among themselves, the marks are in the same box at eight times the resolution, and
/// `issue7696.pdf` draws its glyphs twice so that the page measures grid-fitting with no
/// reference in the room.
///
/// Seven members whose numbers appear in none of the tables above were measured pair by pair in
/// that session, chosen off the verdict list rather than off this note — `bug894572.pdf`,
/// `openoffice.pdf`, `issue6889.pdf`, `bug1200096.pdf`, `issue2017r.pdf`, `issue8570.pdf`,
/// `bug1108301.pdf`. The convicting pair runs **1.28% to 3.79%** differing, every pair containing
/// `ghostscript` **6.72% to 10.11%**, and ours against the nearer pair member **5.06% to 7.02%**
/// — nearer than `ghostscript` is to that same member on all seven — with our ink within 0.43 of
/// 255 of both pair members while `ghostscript` sits up to 2.7 away. The group reproduces on the
/// pages it never listed.
///
/// **And the bound that convicts them was derived and priced in that session, and left where it
/// is.** A floor of 12.04%, the 99th percentile of the pairs whose `FreeType` copies are separate,
/// takes every page of this group off the list — and takes `CONTRADICTED_CALRGB_TO_SCREEN`'s five
/// and `CONTRADICTED_SUBPIXEL_IMAGE`'s one with them, which are a §8.6.5.3 colour reading and a
/// §10.7.4 departure. A differing fraction is a threshold count, so a glyph phase and a small
/// colour error over a large area reach the same 5–12% and no bound can separate them. ADR 0771.
const CONTRADICTED_GLYPH_EDGES: [&str; 27] = [
    "pdfbox/unencrypted.pdf page 2",
    "bad-PageLabels.pdf page 1",
    "franz_2.pdf page 1",
    "issue8088.pdf page 1",
    "issue8088.pdf page 2",
    "issue8088.pdf page 3",
    "freeculture.pdf page 313",
    "bug1108301.pdf page 1",
    "bug1151216.pdf page 1",
    "bug1175962.pdf page 1",
    "bug1200096.pdf page 1",
    "bug1252420.pdf page 1",
    "bug894572.pdf page 1",
    "issue2017r.pdf page 1",
    "issue3207r.pdf page 1",
    "issue3405r.pdf page 1",
    "issue3694_reduced.pdf page 1",
    "issue4061.pdf page 1",
    "issue4650.pdf page 1",
    "issue5010.pdf page 1",
    "issue6889.pdf page 1",
    "issue7492.pdf page 1",
    "issue7696.pdf page 1",
    "issue7901.pdf page 1",
    "issue8097_reduced.pdf page 1",
    "issue8570.pdf page 1",
    "openoffice.pdf page 1",
];

/// Contradicted with nothing on the page to explain it. **This is the interesting list.**
///
/// 42 pages carrying no undrawn annotation, no hidden optional content and no substituted
/// font — so the difference is in something we believe we implement. Three causes are
/// identified; the rest are unexamined, and working through them is the highest-value use of
/// this gate. **Four pages left in the thirty-ninth session and none of them was a defect**:
/// `type4psfunc.pdf`, `postscript_type4_many_outputs.pdf` and both pages of
/// `function_based_shading_cmyk.pdf` are `DeviceCMYK` conversion, and the way that was
/// settled is worth as much as the pages —
/// `CONTRADICTED_DEVICE_CMYK_CONVERSION` has it. The short version: the two references that
/// agreed were running one ICC profile between them, and this tree's own evaluator, pointed
/// at that profile on this machine, reproduced both of their renders.
///
/// **The subdivision lattice is gone and it took three pages with it** (forty-third session,
/// ADR 0051). `mesh_shading_empty.pdf` was the one identified live cause here: the
/// twenty-eighth session measured it rather than repeating the guess this comment used to
/// carry — it said the mesh was "displaced horizontally", and it was not; every reference put
/// the coloured region in the same pixel columns, and what failed was *structural* similarity,
/// 0.972 against 0.990, measuring the faint lattice left by filling a Gouraud triangle as many
/// flat sub-triangles (§8.7.4.5.5). `issue2948.pdf` was the same thing an order of magnitude
/// louder, a moiré grid across a whole rainbow page. `issue18816.pdf` came with them. All
/// three now agree, and the entry that said closing it "needs a Gouraud rasteriser in **both**
/// backends" was right about the requirement and wrong about the difficulty: one raster,
/// shared.
///
/// **Five pages left in the seventeenth session and only one of them was fixed.** Four
/// `knockout_*.pdf` are knockout transparency groups (§11.4.6), which that session began to
/// *report* rather than to implement, so they left this list by leaving the comparison — the
/// same trade §9.3.8 and §11.6.2 made before them. **One of the four came back in the
/// seventy-first session as an agreement**: `knockout_isolated_overlap.pdf` is drawn under
/// the clause now and agrees with the reference consensus, which is the report being paid
/// back rather than traded again. The fifth, `issue11279.pdf`, is a fix and
/// had nothing to do with groups: it draws a form XObject that paints beyond its own `/BBox`,
/// which §8.10.1 step c) says shall be clipped, and this tree clipped only an annotation's
/// appearance. That was found by reading §8.10 because §11.6.6 sent a reader there, which is
/// the family review paying for itself again.
///
/// **15 pages left this list at once**, which is the largest single fall it has had, and
/// they were all one cause: ISO 32000-2 §9.6.5.4, the algorithm that turns a character code
/// into an index into a `TrueType` font's `cmap`, was not implemented. The code was handed
/// straight to `skrifa`'s `Charmap`, which selects the best *Unicode* subtable — right for
/// laying out text, and wrong here, because the subclause's whole subject is that a code is
/// not a character. A font whose only subtable is a (1, 0) Macintosh one, which is exactly
/// what §9.6.5.4's own guidelines tell a producer to emit, mapped nothing at all and fell
/// through to a guess that the code was the glyph index. `issue20504.pdf` was the entry that
/// named the cause: six scripts in six embedded subsets, five of them drawing nothing and
/// one drawing `!"#$`, all of it reported as complete.
///
/// Two further causes have been found through this list and fixed: `CalGray` and `CalRGB`
/// converted as their device equivalents, and now this. That is what the list is for.
///
/// # Seventeen left in the thirtieth session and seven arrived, and both halves are one change
///
/// Reading §9.9 and implementing `/FontFile` — a bare Type 1 font program — took 19 corpus
/// documents off the incomplete row, and with them **17 pages of this list**, every one of
/// which now agrees with the reference consensus. `issue5751.pdf` is the one that stayed:
/// it is a CIDFont whose descendant descriptor embeds a `/FontFile`, which §9.9's Table 124
/// does not allow there, so it draws in a substitute face as it always did. They had been drawing in a substitute
/// typeface, silently, because an unreadable embedded program falls through to substitution
/// and substitution only speaks when it can address nothing. So the largest single fall this
/// list has had came from a feature whose corpus count was recorded as zero.
///
/// The seven that arrived came from the *instrument*, and they are pages this gate used to
/// call `ambiguous`. `has_text` decided a page's tolerance class by whether we read any text
/// back; it now asks `Interpretation::glyphs`, which counts glyphs that marked the page.
/// 25 pages left `ambiguous` because a bound wide enough for glyph hinting is one two
/// references can agree inside — 19 of them agree with us and 7 do not. **A page that was
/// unjudgeable and is now contradicted is not a regression; it is a page that was already
/// wrong and could not be said to be.** `issue9915_reduced.pdf` was the one with a visible
/// story, and the forty-fifth session measured it and moved it to
/// `CONTRADICTED_REFERENCE_GLYPH_WIDTHS` — see there for what the measurement says, and for
/// the arithmetic this comment used to carry, which was wrong.
///
/// # One left in the forty-fourth session, and the file told us the answer
///
/// `issue2537r.pdf` drew `.notdef` boxes where three references draw `LINE UP`, and it was
/// the only page left where we differed from *every* reference — 10.3 levels from each,
/// against their own closest agreement of 1.03. Its embedded `TrueType` states
/// `indexToLocFormat` as **0x0100**, which is 1 written in the wrong byte order and is neither
/// of the two values ISO/IEC 14496-22 defines, so `skrifa` read `loca` in the wrong width and
/// reached no outlines. What settled the repair was the font's own table directory rather than
/// any other reader: `loca`'s last entry equals `glyf`'s length under exactly one of the two
/// readings, and `loca`'s own length is `4 × (n + 1)` rather than `2 × (n + 1)`. ADR 0052.
///
/// # What these files are, looked up in the fifty-ninth session
///
/// Every one of them is a pdf.js test file named after an issue or a Mozilla bug, and reading
/// those says what the file was collected *for*. Eleven were checked and the answer is nearly
/// uniform: **this is a list of hard fonts.**
///
/// | file | what its issue is about |
/// |---|---|
/// | `issue4061.pdf` | Chinese characters rendered wrongly — `font-conversion` |
/// | `issue8570.pdf` | hiragana, katakana and punctuation missing — CJK `cmap` |
/// | `issue5010.pdf` | "buildin cmap parameters are not provided" — CMap regression |
/// | `issue7696.pdf` | `Adobe-Japan1-UCS2` version 8.001 fails where 5.001 works |
/// | `issue6889.pdf` | `/Differences` names of the form `uniXXXX` from Scribus |
/// | `issue8097_reduced.pdf` | a Type 1 font, `AdvTT7b515deb`, converted wrongly |
/// | `issue3694_reduced.pdf` | missing glyphs — `font-conversion` |
/// | `bug1252420.pdf` | an embedded Garamond CFF rejected by the OpenType Sanitiser |
/// | `bug1175962.pdf` | a Unicode chart — `pdfjs-d-font-conversion` |
/// | `bug1650302_reduced.pdf` | Czech diacritics — duplicate of the long-standing bug 844092 |
///
/// That is corroboration from outside this project for what its own instruments had only
/// hinted: the residue of this list is **glyph rasterisation**, on files chosen because their
/// fonts are hard, judged against a consensus of three renderers that share `libfreetype` while
/// this tree uses `skrifa`. A pixel comparison cannot separate "the wrong glyph" from "the same
/// glyph, antialiased differently", and on an eight-point Chinese character almost every pixel
/// is an edge. **The instrument to reach for is not a tighter tolerance but a different
/// question** — which glyphs were drawn, and where — and `has_text` already asks half of it.
///
/// # The one that is not a font, and it comes with a clause
///
/// `issue7891_bc1.pdf` is a **soft mask's backdrop colour**. Its issue says the two files
/// `_bc0` and `_bc1` are identical but for `/BC`, that the mask is deliberately smaller than
/// the group it applies to, and that Adobe draws the transparency group's boundary as a red
/// line where pdf.js drew a black one. Our own numbers fit a thin line: mean 0.15 of a level
/// against a bound of 1.00, one tile at 6.73 against 6.04, 0.54% of pixels differing, with
/// `mupdf` and `ghostscript` the pair that agree. `soft_mask.rs` does read Table 142's `/BC`,
/// so the question is not whether the entry is read but what fills the area the mask's group
/// does not cover.
///
/// **That question was answered in the six-hundred-and-sixty-second session and this paragraph
/// went on asking it** (ADR 0489, and the numbers above are the six-hundred-and-sixty-fifth's off
/// the gate, where they read 0.22, 10.76 and 0.52% until it ran). The page is a closed form:
/// away from the stroke every pixel is `255 × (1 − L)`, with `L` the image's own sample inside the
/// mask group's `/BBox` and `/BC`'s white outside it — so what fills the uncovered area is settled
/// and it is not the difference. Two thirds of the difference is seven lines of pixels, and the
/// largest of them is the `/BBox` edge itself, where §10.7.4 makes a clipping region a **set**:
///
/// > For clipping, the clipping region consists of the set of pixels that would be included by a
/// > fill operation. Subsequent painting operations shall affect a region that is the intersection
/// > of the set of pixels defined by the clipping region with the set of pixels for the region to
/// > be painted.
///
/// [`CONTRADICTED_TIGHT_CONSENSUS`] is where the page lives and where the measurement is; this
/// paragraph is its history, corrected off the gate in the six-hundred-and-sixty-fifth (ADR 0491).
///
/// # One left in the fifty-fourth session, and it is a step drawn as a gradient
///
/// `issue10572.pdf` is twenty-four hard stripes of green and blue, and we drew each boundary as
/// a seven-pixel gradient. The file states the steps precisely: a type 3 stitching function
/// whose sub-functions are themselves type 3 with `/Bounds [0.5 0.5]` — a subdomain of zero
/// width, which is how a producer writes a discontinuity. §8.7.4.5.3 makes the colour at a
/// point whatever the function says, and 256 evenly spaced samples over an 1800-unit axis put
/// a sample every seven pixels, so every step landed inside one interval and was averaged
/// across it.
///
/// A `Ramp` now carries a *position* per stop, and `Function::breakpoints` reports where the
/// clause allows a jump — `/Bounds`, recursively, mapped back through `/Encode`. Each break
/// gets two stops at one position, which is how a gradient expresses a step. ADR 0059.
///
/// # One left in the fifty-third session, and two documents left the comparison with it
///
/// `pattern_text_embedded_font.pdf` draws two lines of `AbCdEf`: one filled with a shading and
/// one with a *tiling* pattern. Three references draw both; this tree drew the shading line and
/// left the other blank, because a glyph took `fill_paint()`, which leaves a tiling pattern
/// alone by design — a tiling pattern is not a paint, it is a cell replayed across an area, and
/// nothing replayed it across a glyph. §8.7.2 settles it in five words: "All patterns shall be
/// treated as colours."
///
/// The same change made two documents *report*, which is trap 5's exchange and is written up on
/// `MAX_INCOMPLETE`: `scorecard_reduced.pdf` strokes with a tiling pattern, which needs the
/// stroked outline the backends compute for themselves, and
/// `ContentStreamCycleType3insideType3.pdf` puts the Type 3 font being drawn inside its own
/// pattern's `/Resources`, so tiling its glyph enters the cycle the file is named for and stops
/// at `MAX_FORM_DEPTH`. 820 agreeing, 77 contradicted, over two fewer pages.
///
/// # Three left in the fifty-second session, on one clause and one page's evidence
///
/// `issue6231_1.pdf` is a TeX plot: three references draw a coloured surface over its axes and
/// **we drew the axes alone, reporting nothing**. The display list held the mesh all along —
/// 79 commands, one of them a `Fill` with a type 5 shading — positioned about 180 points below
/// and 140 to the left of where it belonged, so every triangle fell outside the clip and
/// nothing was drawn.
///
/// §8.7.2 is what decides it, and the sentence is about *forms* rather than about shadings:
/// "if a pattern is used within a form XObject …, the pattern matrix maps pattern space to the
/// form's default user space (that is, the form coordinate space at the time the form is
/// painted with the Do operator)". This tree mapped every pattern to the *page's* default
/// space, which is the rule for a pattern used on a page and the sentence immediately before
/// it. `issue6961.pdf`'s two pages left on the same change. ADR 0057.
///
/// # One left in the fifty-first session, and it is a rule nobody had read
///
/// `tiling-pattern-large-steps.pdf` is 4000 points wide and holds one tiling pattern whose
/// cell paints a rectangle to x = 4000 inside a `/BBox` that ends at 3950. Table 74 states
/// what to do with that in one sentence — "These boundaries shall be used to clip the pattern
/// cell" — and this tree carried no `/BBox` on a tiling pattern at all. poppler, ghostscript
/// and `hayro` stop the paint at 3950; we and `mupdf` ran on to the end of the page, which is
/// why the page's mean difference was only 1.60 while one tile differed by **128 levels of
/// 255**. Ranking the unexplained list by *worst tile over its own bound* put it 25.7× ahead
/// of anything else on the list, and nothing else about the page is unusual.
///
/// The rule is per *cell*, not per fill: a cell whose content runs past its own box would
/// otherwise spill into the neighbouring cell's, and where `/XStep` exceeds the box — which is
/// how a pattern tiles with gaps — into the gap between them. 817 agreeing, 81 contradicted.
///
/// # One left in the forty-second session, and it is a fix
///
/// `issue215.pdf` drew `openmagazin` in lower case where all four references draw small
/// capitals, and it is the only page in this list where we differed from *every* reference
/// while they agreed among themselves — which is what the pairwise table is for. Its
/// `/Differences` name eleven small-capital variants, `o.sc` through `n.sc`; its `post` table
/// is version 3.0 and so holds no names at all; its `CFF ` charset names all eleven; and its
/// `/ToUnicode` maps the codes to U+F76F and neighbours, the private-use block Adobe assigns
/// to small capitals. Two readings of §9.6.5.4 had to change and ADR 0050 has both: the Adobe
/// Glyph List is a *list*, which no `o.sc` is in, and the clause's fallback to "the font
/// program's `post` table" means the program's names wherever a CFF-based OpenType keeps
/// them. 812 agreeing, 86 contradicted.
///
/// # Two more left in the thirtieth session, and only one of them is a fix
///
/// `issue7439.pdf` **is** one, and it is a width rather than a picture: its single line of
/// text shows character code 2 six times, its `/FirstChar` is 3, and its font descriptor
/// states no `/MissingWidth` — so §9.6.2's Table 109 sends those six codes to Table 120's
/// default, which is 0 and which this tree had as half an em. Six half-ems of invented
/// space opened between `Issue` and `7439`.
///
/// `issue3566.pdf` is **not** a fix and is worth more than the page. Its raster is
/// byte-identical before and after — checked, not assumed — and what changed is which
/// *bound* it was judged by. The page's font is a symbolic bare CFF with no `/ToUnicode`,
/// so nothing could name what it drew, so `has_text` was false and a page that is nothing
/// but the word `different` was held to the tolerance measured on flat fills. It passed
/// every absolute bound in that class — mean 0.92 against 1.00, worst tile 3.56 against
/// 5.00, SSIM 0.9940 against 0.9900 — and failed only the relative test. Reading a font
/// program's built-in encoding for its glyph *names* (§9.6.5.1) made the readback work,
/// and the page moved to the text tolerance without a pixel moving.
///
/// That is the same measurement artefact `CONTRADICTED_GLYPHS_JUDGED_AS_VECTOR` names, and
/// this is its second witness. It also says which way to fix it: `has_text` asks whether
/// we could *name* what we drew, and what it means to ask is whether we drew glyphs at all.
///
/// `issue6387.pdf` left the list in the same session and **is not a fix**: its text is set
/// vertically in an `Identity-V` font, which this tree accepted while drawing it
/// horizontally, and which now reports. The page stopped being compared rather than starting
/// to agree, and saying so is the difference between a count that means something and one
/// that can be improved by reporting more.
///
/// `close-path-bug.pdf` left in the tenth session and is worth more than one page: **no
/// dashed line in any document was ever dashed**. The `d` operator was parsed, its arguments
/// were dropped — the content lexer hands an array to an operator flattened, and the code
/// read only the empty case — and both backends had implemented dashing all along. It is
/// this file's own warning about a gap *inside* an implemented feature, found by a page that
/// draws the standard's own simple graphics example (Annex H.4) with `[4 6] 0 d` in it.
///
/// # Seven left in the sixty-first and sixty-eighth sessions, and the top of the list is why
///
/// `issue3694_reduced.pdf` ranked first at 1.81, and opening its artefact found a defect in
/// the *device transform* rather than anything on the page: a crop box 272.595 x 56.122 tall,
/// a raster of 57 rows, and the y flip anchored to the raster's last row instead of the page's
/// own top edge, so every mark sat 0.878 of a row too low (ADR 0064). `bug1650302_reduced.pdf`,
/// `freeculture.pdf` pages 67, 76 and 339, and `issue1002.pdf` went with it, along with six
/// pages from three other groups — 11 in all, 76 contradicted pages down to 65.
///
/// What settled it was not the heatmap but *`ghostscript`'s raster being the same size as
/// ours* and its content sitting one row higher. A one-pixel offset reads as a rounding
/// difference until you notice that the renderer which rounded the same way disagrees anyway.
///
/// `issue3694_reduced.pdf` stays on this list at 0.60 instead of 1.81 — mean 2.51 against a
/// bound of 5.00, 9.39% of pixels differing against a bound of 6.26% — which is a page of
/// hairline-outlined display type at seventeen pixels against two references that share
/// `FreeType`.
///
/// `bug1175962.pdf` at 1.61 was measured in the sixty-eighth session and is the same shape as
/// `colors.pdf` one step along: a 220x180 page of runic display type at about five pixels, where
/// the whole page is glyph edges. Total ink is ours 1 026 676, `poppler` 1 017 828, `mupdf`
/// 1 019 826 and `ghostscript` 939 960 — so the voting pair are within 0.2% of each other and
/// `ghostscript`, which links the *same* `libfreetype`, is 8% lighter than either. **Sharing a
/// font rasteriser is not what makes two references agree**; sharing its settings is, and trap 9
/// is the more careful statement of that than "they share code".
///
/// `issue7891_bc1.pdf` is now the top of the list at 1.78, and it was measured in the same
/// session without being fixed. Its difference is one word inside a luminosity soft mask whose
/// group draws a 676x436 greyscale image reduced 2.8-fold; the column centroids of the five
/// renderers span 0.76 of a pixel, the pair the gate votes with are 0.25 apart, and switching
/// our own reduction between area averaging and point sampling moves the raster without moving
/// any printed metric. That is trap 12's shape rather than a defect: the bound is 6.04 because
/// two references agree very closely, and no reading of a clause chooses between five
/// resamplings of one image.
const CONTRADICTED_UNEXPLAINED: [&str; 0] = [];

/// Contradicted because two references agree with each other more closely than anybody is right.
///
/// # One subclause decides all three pages, and this note cited none of it for nineteen sessions
///
/// The six-hundred-and-sixty-second session (ADR 0489; the citation is the six-hundred-and-sixty-fifth's,
/// ADR 0491, which is where the rule that a note names the decision that wrote it comes from) chose this
/// group by counting, for every
/// non-empty `CONTRADICTED_*` list, **how many clauses of ISO 32000-2 its note cites**. Thirteen of the
/// fourteen cite at least one — `CONTRADICTED_DEVICE_CMYK_CONVERSION` seventeen,
/// `CONTRADICTED_SUBSTITUTED_FONT` eighteen — and this one cited **none**. A contradicted verdict
/// is the claim that the standard rather than the consensus decides the page (principle 5), so a
/// group with no clause under it is a verdict with no warrant, whatever its arithmetic says.
///
/// Opened, all three pages turned out to be **§10.7.4**, in three different paragraphs of it: the
/// shape paragraph for `colors.pdf`, the image paragraph for the reduced greyscale inside
/// `issue7891_bc1.pdf`'s soft mask, and — the one that actually decides the verdict — the
/// **clipping** paragraph, which is about sets where every renderer here is composing coverages.
///
/// # `issue7891_bc1.pdf` page 1: the note blamed the word, and the word is where we are right
///
/// **It was the last page of `CONTRADICTED_UNEXPLAINED` and left in the two-hundred-and-forty-third
/// session.** What it said until the six-hundred-and-sixty-second was that "the difference is one
/// word inside a luminosity soft mask whose group draws a 676 × 436 greyscale image reduced
/// 2.8-fold", on an ink ladder agreeing to 0.0014 of 255. The first half is a description of the
/// file and the second is a real measurement of a metric that **passes**; between them sat no
/// account of the metric that fails, which is the worst tile at 6.73 against a bound of 6.04.
///
/// The page admits a closed form, and a tighter one than most. Object 12 strokes
/// `211.76 421.544 243.36 156.960 re` in red at the default width, sets `/GS1` — Table 145's
/// `/SMask` with `/S /Luminosity`, `/BC [1 1 1]` and `/G` the form that draws the image — and fills
/// the same rectangle black through it. So away from the stroke every pixel is `255 × (1 − L)`,
/// with `L` the mask group's luminosity: the image's own sample inside the group's
/// `/BBox [198.8 434.504 362.16 501.464]`, and `/BC`'s white, worth 1.0, outside it. Two forms were
/// written out pixel by pixel and compared with `raster_compare` through
/// `examples/compare_rasters`: **point**, which is §10.7.4's image paragraph carried out —
///
/// > However, only those pixels whose centres lie within the region shall be painted. The
/// > position of the centre of such a pixel -in other words, the point whose coordinate values
/// > have fractional parts of one-half -shall be mapped back into source space to determine how
/// > to colour the pixel. There shall not be averaging over the pixel area.
///
/// — and **area**, the exact box average over each device pixel's source footprint, which is what
/// ADR 0025's departure approximates. On the tile the gate fails at, device x 224–255 by y 320–351,
/// which is the middle of the word:
///
/// ```text
///                 vs the area form   vs the point form
///   ours             0.166  max  1      0.947  max  9
///   hayro            2.814  max 18      3.012  max 22
///   poppler          4.255  max 27      4.362  max 34
///   ghostscript      4.596  max 30      4.677  max 29
///   mupdf            6.723  max 40      6.802  max 43
/// ```
///
/// **On the tile that decides the verdict our raster is the page's own arithmetic to one level of
/// 255, and the two renderers that vote are 4.60 and 6.72 levels from it.** Their distance from
/// each other there is 3.018, twice which is the bound of 6.04 — and our 6.725 against `mupdf` is
/// `mupdf`'s own 6.723 from the form, because ours *is* the form. That is trap 12 stated from the
/// document instead of from a ranking. Two things fall out of the same table: every one of the five
/// is nearer the average than the point sample, so **§10.7.4's "[t]here shall not be averaging over
/// the pixel area" is departed from by all of them**, ours in writing (ADR 0025) and theirs not;
/// and the reduction the old note pointed at is settled rather than in dispute.
///
/// # What the page does differ by is seven lines of pixels, and they are all edges
///
/// Splitting our distance from each voting reference by device row and column — the fill
/// rectangle's own two fractional edges and four columns, the mask group's `/BBox` rows 290 and
/// 357 and its column 362 — leaves the whole of the rest of the page in the last row:
///
/// ```text
///                              ours vs mupdf   ours vs ghostscript   mupdf vs ghostscript
///   fill rectangle's rows           0.0197            0.0259                 0.0153
///   fill rectangle's columns        0.0128            0.0266                 0.0175
///   mask /BBox rows 290, 357        0.0625            0.0625                 0.0003
///   mask /BBox column 362           0.0227            0.0228                 0.0001
///   everything else                 0.0560            0.0565                 0.0474
///   total                           0.1721            0.1926                 0.0799
/// ```
///
/// **Seven lines carry 68.3% and 71.5% of it**, and the two references that vote agree with each
/// other about the mask's `/BBox` to 0.0003 — which is what a pair agreeing *because they take a
/// clip as a set* looks like. §10.7.4's clipping paragraph is why:
///
/// > For clipping, the clipping region consists of the set of pixels that would be included by a
/// > fill operation. Subsequent painting operations shall affect a region that is the intersection
/// > of the set of pixels defined by the clipping region with the set of pixels for the region to
/// > be painted.
///
/// Device row 290 is `[290, 291)`, the group's `/BBox` reaches to device 290.536, and a fill would
/// include any pixel it intersects — so the clause admits the whole row, which is what all three C
/// references draw (255, the mask's own value) and we draw 118, this tree's departure (1) painting
/// a partly covered pixel partly. **The pair is not uniformly the clause either**: at column 362,
/// where the `/BBox` reaches device 362.16 and the clause admits the column entire, `poppler` does
/// and `mupdf` and `ghostscript` drop it to black. The two that vote depart from the sentence they
/// are agreeing under, one column away from where they agree under it.
///
/// # And one line of the seven is a defect of ours with a number to four digits
///
/// Object 16 is a form XObject whose `/BBox` is exactly the rectangle its content fills, and it
/// carries `/Group`. Device row 213 is covered 0.504 by that rectangle and row 370 by 0.456, and
/// ours paints them at **0.2549** and **0.2079** — `0.504²` and `0.456²` to four digits. The
/// coincident boundary is being **multiplied** where §10.7.4 intersects sets, which is
/// `doc/todo/11` item 4, and `examples/coincident_edge_probe` isolates which composition does it:
/// of the eight ways one rectangle can be stated twice, seven give the edge its own coverage and
/// the eighth — a transparency group *and* a soft mask — gives its square. §11.4.4's NOTE 5
/// flattens a group away unless a soft mask is in force, so the group blit `draw_group` performs
/// is only reached when there is a mask beside it, which is why the residual had no small witness.
/// This page is one now, and a cleaner one than `issue21346.pdf` was: two factors and a closed
/// form rather than seven statements and a ladder.
///
/// **It stays here rather than being fixed** because fixing it moves the page toward the bound and
/// not past it — the whole of those two rows is 0.0197 of a distance of 0.1721 — and because the
/// composition it needs is the one `doc/todo/11` item 4 prices: a group's buffer carries alpha,
/// which is shape times opacity, and only a shape channel beside it makes `min` the right answer.
///
/// # `colors.pdf` pages 1 and 2, and a note that went stale three sessions after it was written
///
/// They arrived in the six-hundred-and-forty-third session from
/// [`CONTRADICTED_ANTIALIASED_EDGES`], whose name said the difference was our anti-aliasing being
/// softer than anybody's. Each page is sixteen axis-aligned rectangles at known sub-pixel
/// boundaries, so the page *is* a closed form: every rectangle's coverage of every pixel is a
/// product of two overlaps, composited in the order the content stream states.
///
/// **This note said until the six-hundred-and-sixty-second session that ours is that form with
/// every coverage rounded to `tiny-skia`'s quarter, and that `hayro`'s is the exact one. Ours has
/// been the exact one since ADR 0476, three sessions after the sentence was written.** The
/// correction reached `doc/traps/pixels-and-rasterisers.md` and §10.7.4's ledger row and not the
/// group it corrects, which is a *third* place a group's note can be wrong: not its name, not its
/// reading, but a sentence that was true when written and that nothing pointed at when the tree
/// moved under it. Both forms re-derived from the content stream in a fresh script and compared by
/// the oracle's own arithmetic:
///
/// ```text
///                  page 1: vs exact   vs quarter    page 2: vs exact   vs quarter
///   ours             0.0026  max  1   0.0428 max 34   0.0026  max  2   0.0463 max 36
///   hayro            0.0015  max  2   0.0409 max 33   0.0017  max  2   0.0462 max 35
///   mupdf            0.0173  max 13   0.0395 max 37   0.0201  max 14   0.0383 max 44
///   ghostscript      0.1026  max 54   0.1342 max 75   0.0890  max 63   0.1046 max 64
///   poppler          0.2117  max 124  0.2483 max 127  0.1883  max 112  0.2078 max 128
/// ```
///
/// Ours differs from the exact form on **0.0000% of either page** — one level at the worst pixel of
/// page 1 and two of page 2 — so the ranking §10.7.4's shape paragraph asks for now reads ours 1,
/// `hayro` 2, `mupdf` 13, `ghostscript` 54, `poppler` 124, and the 33 levels ADR 0474 priced are
/// paid. What has not changed is the verdict, and the reason is the whole of this group's name:
///
/// ```text
///                        page 1     page 2
///   bound (twice the     0.98862    0.98402
///   consensus spread)
///   ours                 0.98786    0.98024
/// ```
///
/// **A rasteriser painting precisely the area each rectangle covers is contradicted on both
/// pages.** `poppler` does not anti-alias an axis-aligned edge at all and `ghostscript`
/// supersamples and filters, so the two that vote are the two furthest from the geometry, and
/// twice their mutual distance is a bound no analytic-coverage renderer meets on a page that is
/// nothing but edges. That is this group's sentence, arrived at from the document rather than
/// from a ranking.
///
/// # Those eight numbers are **structural similarity**, which this note did not say for sixty
/// sessions
///
/// The table above is the only account either `colors.pdf` page has of the bound it fails, and it
/// carries no unit: four figures at four decimals under the words *bound* and *ours*. The
/// seven-hundred-and-twenty-second session found it by asking the whole contradicted pool a
/// question nothing could ask before — *which of the gate's four measures does this page fail on,
/// and does the note holding it name that measure* — and this note names exactly one measure in
/// a hundred and sixty lines, the **worst tile at 6.73 against 6.04**, which is
/// `issue7891_bc1.pdf`'s and not either `colors.pdf` page's. `doc/todo/01`'s twenty-first sweep
/// is the question, `--bin unpriced` is the command, and ADR 0606 has the argument.
///
/// So, in the gate's own words: **both `colors.pdf` pages fail on structural similarity and on
/// nothing else.** Page 1 prints mean 0.21 of 1.00, worst tile 2.63 of 5.00 and differing 0.50%
/// of 1.00%; page 2 prints 0.19, 3.17 and 0.54%. Three of four bounds are met with room to spare
/// on both, and the fourth is the whole verdict.
///
/// # And the sentence above is now measured in that metric, with two renderers that are not us
///
/// *"A bound no analytic-coverage renderer meets"* was an argument about mean distances from a
/// closed form; it is a claim about the failing metric, so it is checked in the failing metric.
/// [`Tolerance::widened_to`] scales the *distance from 1.0*, so the bound is
/// `1 − 2 × (1 − ssim(poppler, ghostscript))` — 0.99431 and 0.99201 between the pair give 0.98862
/// and 0.98402 exactly, which is where those two figures come from. Every pair on the page, from
/// this run's own artefacts through `examples/compare_rasters`:
///
/// ```text
///                              page 1     page 2      against the bound
///   poppler <-> ghostscript    0.99431    0.99201     the pair: it sets it
///   ours                       0.98786    0.98024     fails by 0.00076 / 0.00378
///   hayro                      0.98772    0.98011     fails, and by more than ours
///   mupdf                      0.98739    0.97943     fails, and by more than either
/// ```
///
/// The lower three are each that renderer's worst against the two that vote, which is `poppler`
/// on all three and on both pages. **`mupdf` and `hayro` are outside this page's bound as well,
/// and both are further outside it than we are** — one an independent C interpreter, the other a
/// separate Rust one, neither of them us and neither of them a party to how we round an edge. The
/// ranking of the four is the ranking of how much anti-aliasing each does: `ours ↔ hayro` is ssim
/// 0.99999 with a worst pixel of two levels and `ours ↔ mupdf` 0.99989, so three renderers
/// converge on the geometry and are held to twice the distance between the two that do not.
///
/// That is ADR 0497's sixth criterion answered for these two pages in the units the gate uses:
/// the mechanism does not merely explain the picture, it accounts for the failing bound — and
/// the control is that taking *us* out of the room does not rescue the bound, because two other
/// renderers fail it too.
///
/// # And the pair the bound comes from is not the tightest pair on the page
///
/// The paragraph above measured every pair and read four of the ten. The sixth row of that same
/// table is the one nothing had asked about, and it says that **`ghostscript` and `mupdf` agree
/// with each other more closely than `poppler` and `ghostscript` do** — 0.99625 against 0.99431
/// on page 1 and 0.99278 against 0.99201 on page 2, and on mean and worst tile as well on page 1
/// and on mean and the differing fraction on page 2. They agree inside every class bound, so
/// `{ghostscript, mupdf}` is a consensus of exactly the same standing as the one the verdict
/// rests on, and neither pair contains the other because `poppler` and `mupdf` are what differ.
///
/// **Agreement is not transitive, and this page is what that costs.** `pdfref::decide` takes the
/// largest mutually agreeing set and, where two tie, the one the subset enumeration reaches first
/// — which is the order [`Reference`]'s variants are declared in. Under `{ghostscript, mupdf}`
/// the widened structural bound is tighter than the class floor, so the floor of 0.9900 applies,
/// and our worst structural similarity against that pair is `ghostscript`'s **0.99627** on page 1
/// and **0.99336** on page 2 — inside it, with the other three measures inside as well. The
/// seven-hundred-and-twenty-seventh session made the gate count and name this population, and ADR
/// 0616 has the three candidate rules and what each would cost.
///
/// **Nothing above is withdrawn by it and one thing is sharpened.** The closed forms, the ranking
/// against the geometry and the structural-similarity table are measurements against the page's
/// own arithmetic with no renderer in them, and they stand. What this group is *named* for —
/// a bound derived from two agreeing references being tighter than the arithmetic — turns out on
/// two of its three pages to be a bound derived from two agreeing references **where a third pair
/// agreed more closely and was discarded**, which is trap 12 with a second half nobody had
/// written down.
///
/// # Both `colors.pdf` pages left this list in the seven-hundred-and-twenty-ninth session
///
/// A verdict is one every maximal consensus reaches (ADR 0617), and the two here do not concur, so
/// both pages are `ambiguous` and named in [`AMBIGUOUS_DIVIDED_CONSENSUS`]. **What moves them is
/// the control clause four paragraphs up, applied to the references instead of to us**: "taking us
/// out of the room does not rescue the bound, because two other renderers fail it too" is a true
/// sentence about `{poppler, ghostscript}`'s bound, and `mupdf` — one of the two renderers that
/// fail it — is a member of a maximal consensus of its own, which fails `poppler` right back. Each
/// of the two is contradicted by the other's set on both pages. There is no ranking between two
/// coincidences of the same standing, and the group keeps the pages' whole reading either way.
///
/// `issue7891_bc1.pdf` is untouched by any of this and is why the group still exists: one pair, one
/// reading, a bound tighter than eight-bit arithmetic, and no rival set at all.
///
/// # `poppler` meets this page's bound by 0.06 of a level while sitting 26 times further from the
/// page's own arithmetic than we do
///
/// This is one of three pages in the pool where the voting reference the consensus excludes is
/// inside the bound our own render is outside — the population
/// [`name_the_pages_the_excluded_reference_survives`] prints, and the sharpest accusation the
/// oracle can construct. Taken at face value it says an independent implementation managed what
/// we did not.
///
/// The two numbers are three lines apart in this note and had never been put beside each other.
/// The consensus is `mupdf` and `ghostscript`, agreeing on the worst tile at **3.02**, so the
/// bound is **6.04**; `poppler` against each of them is **5.98**, inside by 0.06; ours against
/// `mupdf` is **6.73**, outside by 0.69. (Every figure but the bound and our own is
/// `examples/compare_rasters` over the gate's artefacts — the gate prints no line for a
/// reference.) And the closed-form table above, on that same tile,
/// measures each renderer against the page's own arithmetic: ours **0.166**, `poppler` **4.255**,
/// `ghostscript` 4.596, `mupdf` 6.723. **Being inside the bound and being right are different
/// facts**, and here they point at different renderers: the reference that meets it is 25.6 times
/// further from what the file says than the render that does not.
///
/// It is the same sentence this group is named for, arriving through the control instead of
/// through the verdict — a bound derived from two agreeing references is tighter than the
/// arithmetic, and *which* third implementation happens to fall inside it is a fact about where it
/// sits on the spread rather than about the clause. ADR 0772.
const CONTRADICTED_TIGHT_CONSENSUS: [&str; 1] = ["issue7891_bc1.pdf page 1"];

/// Documents where our page geometry differs from the references' by more than the one
/// pixel a fractional page size can round to.
///
/// Separate from the four lists above because it is a different and more serious class of
/// defect: a page box, `/Rotate` or `/UserUnit` read differently, not pixels drawn
/// differently. The comparison cannot even proceed.
///
/// **Empty since the twenty-ninth session, and all three entries had one cause.**
///
/// The list held `bug1947248_forms.pdf` and `bug1947248_text.pdf`, which carry `/UserUnit 3`
/// and came out 612x792 where `mutool` and `gs` produced 1836x2376; and `issue19176.pdf`,
/// recorded here as "the reverse case … has not been looked into", where we and `poppler`
/// took a 9x11 page and the other two fell back to 612x792. It is not the reverse case. Its
/// `/MediaBox` is `[0 0 8.5 11]` with `/UserUnit 72`: a page stated in **inches**, which is
/// US Letter once §7.7.3.3's entry is applied. One clause, three documents, and the entry
/// that explained the two obvious ones was sitting in the third with a comment saying it had
/// not been examined.
///
/// The list stays because the class is real and more serious than a pixel difference: a page
/// box, `/Rotate` or `/UserUnit` read differently means the comparison cannot even proceed.
const GEOMETRY: [&str; 0] = [];

// ---------------------------------------------------------------------------------------
// `no render`: the verdict this gate reaches without asking the references
// ---------------------------------------------------------------------------------------
//
// [`examine`] returns as soon as [`render_ours`] fails, so on a `no render` page the three
// reference renderers are never invoked at all. Every other verdict in this file is a
// statement about a comparison; this one is a statement about us alone, and until the
// five-hundred-and-seventy-fifth session **nothing held it in either direction** — a change
// that stopped a document opening would have printed one more line in a report of 888 and
// failed nothing. It is also the bucket where a defect is worst: a page here is not a page
// drawn differently, it is a page a person is shown nothing of.
//
// So the bucket was sized, and then every one of its pages was put to `pdftoppm`, `mutool`
// and `gs` by hand — the same invocations `tools/pdfref/src/reference.rs` builds, explicit
// about the page box, because trap 3 binds a measurement taken outside the harness exactly
// as it binds one inside it. The groups below are what that produced, and the recipe is in
// `doc/oracle-and-corpus.md` §3d so that the next round need not rebuild it.
//
// The lists are held to equality in both directions by [`check_the_ratchets`], over **all**
// pages rather than the complete ones: a page that renders nothing is never complete, so
// filtering on `complete` would hold nothing at all.

/// Pages refused because §7.6.4.1 asks for a password nobody has supplied.
///
/// > If a user attempts to open an encrypted document that has a user password, the PDF reader
/// > shall first try to authenticate the encrypted document using the padding string defined in
/// > 7.6.4.3, "File encryption key algorithm" (default user password):
///
/// and where that fails, "the interactive PDF processor should prompt for a password". This gate
/// is not interactive and supplies none, so the refusal is the clause working rather than a gap.
///
/// What makes that more than an assertion is the **first** sentence: the empty user password is
/// tried on every one of these eight and rejected, and all three references reject it too, each in
/// its own words — `gs` prints *This file requires a password for access*, `mutool` *cannot
/// authenticate password*, `pdftoppm` *Incorrect password*. Four independent derivations of
/// §7.6.4.3's key agreeing that the default password is not this document's is evidence about our
/// reading of that clause, in principle 5's one permitted direction, and it is the only thing this
/// list needs from them.
///
/// `bug1782186.pdf` is the one worth a sentence, because a reference does produce a raster
/// there: `poppler` prints *Unsupported version/revision (4/4) of Standard security handler*
/// and then emits an 842x596 sheet of **zero ink**, so what it drew is not the document. `gs`
/// and `mutool` refuse it as they refuse the rest.
const NO_RENDER_NEEDS_A_PASSWORD: [&str; 8] = [
    "bug1782186.pdf page 1",
    "issue15893_reduced.pdf page 1",
    "issue3371.pdf page 1",
    "issue6010_1.pdf page 1",
    "issue6010_2.pdf page 1",
    "pr6531_1.pdf page 1",
    "print_protection.pdf page 1",
    "saslprep-r6.pdf page 1",
];

/// Pages refused because the file's encryption is not something §7.6 states an algorithm for.
///
/// `issue21579.pdf` is Table 21's `/R` 5, the deprecated proprietary extension ISO 32000-2
/// describes and does not specify; `doc/HANDOVER.md` records the decision to leave it, and the
/// crawl puts it at 0.03% of the web. All three references refuse it too.
///
/// `PDFBOX-4352-0.pdf` is a fuzzed cross-reference table — one entry reads
/// `0007777777770000130 00000 n`, nineteen digits where §7.5.4 states twenty bytes — so object 6
/// is unreachable and the `/Encrypt` the trailer names resolves to nothing. §7.6.1 makes that
/// dictionary the statement of which security handler applies, so a reader that cannot read it
/// cannot decrypt one stream of the file, and opening it would mean reading ciphertext as
/// content. `poppler` and `mutool` rebuild the table and produce a 200x50 sheet of **zero ink**;
/// `gs` produces nothing. A blank page is not a recovery, and nothing here is owed to it.
const NO_RENDER_ENCRYPTION_THE_STANDARD_DOES_NOT_STATE: [&str; 2] =
    ["PDFBOX-4352-0.pdf page 1", "issue21579.pdf page 1"];

/// Pages the page tree does not yield, which `tests/corpus.rs` documents one file at a time.
///
/// Five documents and one second page, and the references say the same thing about all but one
/// of them: `REDHAT-1531897-0.pdf`, `bug1020226.pdf` and `poppler-85140-0.pdf` are drawn by
/// nobody; `Pages-tree-refs.pdf` page 2 is a `/Kids` cycle that `poppler` answers with a 1x1
/// sheet — *Syntax Error: Loop in Pages tree* — while `mutool` says *cycle in page tree* and
/// refuses, as we do; `poppler-937-0-fuzzed.pdf` is the same 1x1 from the same reader and
/// nothing from the other two.
///
/// **`poppler-742-0-fuzzed.pdf` left this list in the eight-hundred-and-sixtieth session**, and
/// it is this corpus's one witness of ADR 0784: its page object's `/TrimBox` array is mutated
/// mid-value and runs into the stream after it, so the whole object refused to parse and the
/// page tree yielded nothing. §7.3.7 states no extent for a dictionary beyond its closing `>>`
/// and states that the written order is not information, so the seven entries whole before the
/// damage are a *subset* of what the producer wrote rather than the dictionary — taken by the
/// recovery on the `/Type /Page` inside them, drawn on the producer's own `/MediaBox`, and
/// reported, because `/Contents` is among the entries the damage took.
///
/// **`Brotli-Prototype-FileA.pdf` is the one where two references draw a page and we draw
/// none**, and it is the one page of this whole bucket where that is true and the reason is not
/// ours: `mutool` and `gs` produce 1224x792 at ink 17.12 and 22.28, `poppler` refuses with
/// *Unknown filter 'BrotliDecode'*, and the file is a prototype of a filter ISO 32000-2 does not
/// define — its object streams are compressed with it, so the page tree is inside a stream this
/// reader cannot inflate. Two renderers implementing an unpublished extension is not evidence
/// about a clause. `tests/corpus.rs` carries the same reading and nothing is owed until the
/// filter is published.
const NO_RENDER_NO_PAGE_IN_THE_TREE: [&str; 6] = [
    "Brotli-Prototype-FileA.pdf page 1",
    "Pages-tree-refs.pdf page 2",
    "REDHAT-1531897-0.pdf page 1",
    "bug1020226.pdf page 1",
    "poppler-85140-0.pdf page 1",
    "poppler-937-0-fuzzed.pdf page 1",
];

/// The page this *gate* will not raster, which the program draws perfectly well.
///
/// `issue19517.pdf` is 12608x16806 at one device pixel per point — 211 890 048 pixels, past
/// [`PIXEL_BUDGET`]'s 67 108 864 — so [`render_ours`] never reaches the rasteriser and the
/// verdict reads exactly like a document this reader cannot handle. It is not one.
/// `examples/render_at` draws the same page at the same scale with the interpreter's own bound
/// and gets **12608x16806 at ink 172.597**, against `pdftoppm` 172.602, `mutool` 172.599 and
/// `gs` 172.599: agreement with all three to **0.005 of 255**, on a page this gate has never
/// judged.
///
/// It is listed rather than exempted, and the budget is not moved. Three rasters of 212
/// megapixels are 2.5 GiB of reference renders to hold and to cache for one page, which is what
/// the constant is there to refuse; what was wrong was not the refusal but that the bucket it
/// lands in said nothing about whose refusal it is. **A verdict that names the program when the
/// instrument is what declined is the shape to watch for**, and this is the one instance of it
/// in 1794 pages.
const NO_RENDER_LARGER_THAN_THIS_GATES_BUDGET: [&str; 1] = ["issue19517.pdf page 1"];

/// Every page the gate produced no render of, in one list.
fn no_render_expected() -> Vec<&'static str> {
    NO_RENDER_NEEDS_A_PASSWORD
        .iter()
        .chain(&NO_RENDER_ENCRYPTION_THE_STANDARD_DOES_NOT_STATE)
        .chain(&NO_RENDER_NO_PAGE_IN_THE_TREE)
        .chain(&NO_RENDER_LARGER_THAN_THIS_GATES_BUDGET)
        .copied()
        .collect()
}

// ---------------------------------------------------------------------------------------
// `not comparable` and `reference geometry`: the other two verdicts nothing watched
// ---------------------------------------------------------------------------------------
//
// ADR 0410 held the `no render` bucket by name and left these two printed and ungated,
// with the reason for leaving them written down: neither is an accusation against this
// tree *by construction* — one is fewer than two references producing an image, the other
// is the references disagreeing about the page's size — but "by construction" is a claim,
// and the same claim had been true of `no render` for four hundred rounds.
//
// So the fifteen pages were put to `pdftoppm`, `mutool` and `gs` by hand on
// `doc/oracle-and-corpus.md` §3d's recipe, and the groups below are what that produced.
// The claim survives, with two things to say for it that nobody could say before:
//
// - **On four of the fifteen the one reference that did draw agrees with us**, at 0.06 to
//   3.15 of 255 mean absolute difference. That is not a vote and it is not evidence in
//   principle 5's sense — one renderer is one renderer — but it is the opposite of what a
//   bucket nobody watches is feared to contain.
// - **The `reference geometry` label is wrong about both of its own members**, and the
//   mechanism is one line of trap 3: `pdftoppm` writes a **1x1 raster and exits 0** when it
//   fails to create a page, so a refusal enters [`reconcile`] as an opinion about the
//   page's extent and outvotes the one renderer that drew. Neither page has three
//   references disagreeing about a size; both have one reference and two refusals.
//
// Held over **all** pages rather than the complete ones, for `no render`'s reason: what
// these verdicts are about is the references, so filtering on our own completeness would
// hold seven of the thirteen and watch the rest not at all.

/// The two pages where `pdftoppm`'s refusal is counted as an opinion about the page's size.
///
/// [`reconcile`] takes the largest set of references agreeing about the extent, and with two
/// rasters of different sizes there is no such set — so the verdict is
/// "no two references agree about the page size". On both of these pages that sentence is
/// true of the rasters and false about the page: one of the two rasters is **1x1 and of zero
/// ink**, which is what `pdftoppm` leaves behind when it fails to create a page while still
/// exiting 0, and `gs` produced nothing at all on either.
///
/// `bug1978317.pdf` page 1 is a browser's print-to-PDF whose annotation array `poppler`
/// refuses — *Page annotations object (page 1) is likely malformed. Too big: (32768)*,
/// *Failed to create page* — and `gs` fails silently under the `-q` this gate passes. So the
/// evidence about that page is `mutool`'s 612x792 alone, and ours is 612x792 too: **1.69 of
/// 255 mean absolute difference over the sheet**, the same text in the same places. A page
/// the gate cannot judge, on which the only reading available agrees with ours.
///
/// `boundingBox_invalid.pdf` page 3 is the third construction of the file ADR 0410 took the
/// first of, captioned by its own producer *Empty /CropBox and /MediaBox intersection*:
/// `/MediaBox [0 0 600 800]` with `/CropBox [600 800 1000 1000]`, two rectangles meeting at
/// one corner. §14.11.2.1 states the rule and this tree applies it —
///
/// > If the bounds of the crop, trim, bleed or art box extends outside of the bounds of the
/// > media box, a processor shall treat the box as its intersection with the media box.
///
/// — and the intersection encloses no area, which the clause states no recovery for. Table
/// 31 does: `/CropBox`'s "default value is the page's media box", so an unusable crop box
/// falls back to a rectangle the *file* states rather than to one this program invented,
/// which is why it is not reported the way ADR 0389's substituted media box is. We draw
/// 600x800 at ink 1.502. **No reference draws the page at all**: `mutool` produces 612x792
/// of ink 0, `pdftoppm` its 1x1, and `gs` exits *Unrecoverable error*. A blank sheet is not
/// a page (ADR 0410), so there is nothing here that could contradict anybody.
const REFERENCE_GEOMETRY_A_REFUSAL_WEARING_A_RASTER: [&str; 2] =
    ["boundingBox_invalid.pdf page 3", "bug1978317.pdf page 1"];

/// Every page whose references could not be reconciled into a size, in one list.
fn reference_geometry_expected() -> Vec<&'static str> {
    REFERENCE_GEOMETRY_A_REFUSAL_WEARING_A_RASTER.to_vec()
}

/// §7.6's encryption, where `poppler` and this tree open the file and the other two decline.
///
/// `auth-event-ef-open.pdf` and `encrypted-attachment.pdf` are both opened here and by
/// `pdftoppm`, to **612x792 at ink 0.264989 against 0.269507** — 0.06 of 255 mean absolute
/// difference, the same page. `mutool` answers *cannot authenticate password* on each and
/// `gs` *This file requires a password for access*.
///
/// That is the mirror of [`NO_RENDER_NEEDS_A_PASSWORD`] and it is worth the distinction. There
/// four derivations of §7.6.4.3's key agree that the empty user password is **not** the
/// document's; here two say it is and two say it is not, which `doc/HANDOVER.md`'s trap 9
/// already records for a different pair of files — two against two is not a tie but a question,
/// and §7.6.6 puts a refusal on the stream whose key is missing rather than on the document.
/// Nothing is owed unless the page we draw is wrong, and the reference that agrees with it is
/// the one that got past the same clause.
const NOT_COMPARABLE_ENCRYPTION_TWO_REFERENCES_DECLINE: [&str; 2] = [
    "auth-event-ef-open.pdf page 1",
    "encrypted-attachment.pdf page 1",
];

/// A cross-reference table one reference rebuilds, and its answer is ours.
///
/// Three pages, each with exactly one reference reaching a page and agreeing with us:
///
/// - `issue9418.pdf` page 1 — `gs` 3024x2304 at ink 20.376 against ours 20.698, **3.15 of 255
///   mean absolute difference** on a text page whose class bound is 5.00. `pdftoppm` rebuilds
///   the table and then finds no `/Pages`; `mutool` repairs the document and asks for page -1.
/// - `poppler-67295-0.pdf` page 1 — `gs` 612x792 at ink 0.426603 against ours 0.409477,
///   **0.14 of 255**. `pdftoppm` refuses a `/Count` of 1 410 065 407 against eight objects and
///   `mutool` *Invalid number of pages*; §7.7.3.2 makes `/Count` "the number of leaf nodes",
///   which a page tree with eight objects in it cannot have, and neither of us needs the entry
///   to walk the tree.
/// - `bug1980958.pdf` page 1 — `mutool` repairs the file to **10x10 of ink 0** and we produce
///   10x10 of ink 0. The one page in this bucket where the agreement is about a blank sheet,
///   which is worth naming rather than counting: ADR 0410's rule is that a blank is not a page,
///   so what agrees here is the *geometry* and nothing else.
const NOT_COMPARABLE_ONE_REFERENCE_REBUILT_THE_FILE: [&str; 3] = [
    "bug1980958.pdf page 1",
    "issue9418.pdf page 1",
    "poppler-67295-0.pdf page 1",
];

/// Pages no reference reaches at all, so there is nothing to compare in either direction.
///
/// All three refuse each of these eight, and where `pdftoppm` leaves a raster behind it is the
/// 1x1 of [`REFERENCE_GEOMETRY_A_REFUSAL_WEARING_A_RASTER`]. Six of the eight are documents
/// this tree reports; `poppler-91414-0-53.pdf` and `poppler-91414-0-54.pdf` are not, and they
/// are two names for one page — 795x842 at ink 0.185644 in both, the word *foobar*.
///
/// The reason to look rather than to shrug is that a page nobody else draws is a page nobody
/// else can check, so what this tree draws on one is worth a glance on its own: they are one
/// signature appearance (`poppler-395-0-fuzzed.pdf`, ink 1.077), one word
/// (`poppler-91414-0-53.pdf`), and six blank sheets, five of which are pages this tree reports
/// and one of which — `issue19484_1.pdf` — reports a corrupt object stream that its twin
/// `issue19484_2.pdf` reports too. None is a plausible-looking page built out of nothing, which
/// is the failure trap 5 exists for.
///
/// **`poppler-742-0-fuzzed.pdf` joined this group in the eight-hundred-and-sixtieth session**,
/// out of [`NO_RENDER_NO_PAGE_IN_THE_TREE`], and it is ADR 0784's witness in this corpus: its
/// page object's `/TrimBox` is mutated mid-value and runs into the stream after it, so §7.3.7's
/// dictionary never closes and the whole object used to refuse. The seven entries whole before
/// the damage are the producer's own and one of them is `/MediaBox`, so the sheet is 596x842
/// and the page is blank because `/Contents` is among the entries the damage took — said out
/// loud, which is the whole of what keeps it out of the failure the paragraph above names. The
/// three references were asked again this session rather than quoted: `pdftoppm` prints
/// *Dictionary key must be a name object* twice and *Illegal character '&gt;'* and writes no
/// file, `mutool` repairs the cross-reference table and writes **zero bytes**, and `gs` says
/// *Catalog dictionary not located in file* and reports the document as having no pages. That
/// is agreement about the *object* rather than about the page — the seven entries are in the
/// file and every one of the four readers can see them.
const NOT_COMPARABLE_NO_REFERENCE_REACHES_A_PAGE: [&str; 8] = [
    "issue15590.pdf page 1",
    "issue19484_1.pdf page 1",
    "issue19484_2.pdf page 1",
    "issue9105_other.pdf page 1",
    "poppler-395-0-fuzzed.pdf page 1",
    "poppler-742-0-fuzzed.pdf page 1",
    "poppler-91414-0-53.pdf page 1",
    "poppler-91414-0-54.pdf page 1",
];

/// The decompression bomb two references are killed on before they answer.
///
/// `bomb_giant.pdf` page 1: `pdftoppm` and `gs` are each given 30 seconds by
/// `Reference::render_within` and neither returns, which is what the limit is for. Asked by
/// hand with 40 seconds `pdftoppm` still does not finish and `mutool` draws **612x792 at ink
/// 1.45335** against ours **1.33797** — 0.32 of 255 mean absolute difference on a page this
/// tree reports, having stopped at the interpreter's own budget.
///
/// This is the one member of the bucket whose verdict is about the *instrument* rather than
/// about the file, which is [`NO_RENDER_LARGER_THAN_THIS_GATES_BUDGET`]'s shape in the other
/// bucket: the timeout stays where it is, and the group says whose refusal it is.
const NOT_COMPARABLE_TWO_REFERENCES_RAN_OUT_OF_TIME: [&str; 1] = ["bomb_giant.pdf page 1"];

// The four groups below arrived together in the six-hundred-and-eighty-first session, and
// they are one rule rather than four findings: `pdfref::consensus_abstentions` refuses a vote
// to a reference whose raster is **one colour** where a reference that **drew marks**
// disagrees with it, so a page whose remaining readings number fewer than two lands here
// instead of being judged by two flat sheets. ADR 0513 has the argument and the measurement;
// what belongs beside the lists is what each page turned out to be.
//
// Read them with the bucket's own caution in mind: this verdict is the one where a page of
// ours is least likely to be checked by anybody, and 29 pages arrived at once.

/// The JBIG2 family where `jbig2dec` returns a flat sheet and `poppler`'s own decoder draws.
///
/// 21 pages, and 20 of their names contain `refine` — `issue20439.pdf` is the member of the
/// family whose name does not say so, which [`CONTRADICTED_SHARED_JBIG2_DECODER`] records.
/// `mupdf` and `ghostscript` are one implementation here because both link `jbig2dec`, and on a
/// refinement region or a Huffman-coded symbol dictionary it gives up: on eighteen of these
/// `mupdf` returns a page that is entirely **black** and `ghostscript` one that is entirely
/// **white**, and on the other three both return white. Either way not one pixel of either
/// raster is a mark, while `poppler` draws the picture at 17.589 of 255 against our 17.495.
///
/// **They came from two different verdicts, and that is the point of the move.** Eighteen were
/// `ambiguous` — [`AMBIGUOUS_SHARED_JBIG2_DECODER`], where a black sheet and a white sheet
/// disagreeing with each other read as the references failing to settle a question — and three
/// were `contradicted`, where two white sheets agreed with each other and outvoted the renderer
/// that drew. Neither verdict was false about the rasters and both were misleading about the
/// page: there is one reading of these files here, `poppler`'s, and it is within a level of
/// ours.
///
/// What says we are right is not `poppler` and is unchanged by any of this: `tests/jbig2.rs`
/// decodes ninety-six documents that encode one image through every coding mode ISO/IEC 14492
/// defines, and all ninety-six decode to byte-identical pixels here.
const NOT_COMPARABLE_A_SHARED_JBIG2_DECODER_RETURNED_ONE_COLOUR: [&str; 21] = [
    "bitmap-composite-and-xnor-refine.pdf page 1",
    "bitmap-composite-or-xor-replace-refine.pdf page 1",
    "bitmap-refine-customat-tpgron.pdf page 1",
    "bitmap-refine-customat.pdf page 1",
    "bitmap-refine-lossless.pdf page 1",
    "bitmap-refine-refine.pdf page 1",
    "bitmap-refine-template1-tpgron.pdf page 1",
    "bitmap-refine-template1.pdf page 1",
    "bitmap-refine-tpgron.pdf page 1",
    "bitmap-refine.pdf page 1",
    "bitmap-symbol-symhuffrefine-textrefine.pdf page 1",
    "bitmap-symbol-symhuffrefineseveral.pdf page 1",
    "bitmap-symbol-texthuffrefine.pdf page 1",
    "bitmap-symbol-texthuffrefineB15.pdf page 1",
    "bitmap-symbol-texthuffrefinecustom.pdf page 1",
    "bitmap-symbol-texthuffrefinecustomdims.pdf page 1",
    "bitmap-symbol-texthuffrefinecustompos.pdf page 1",
    "bitmap-symbol-texthuffrefinecustomposdims.pdf page 1",
    "bitmap-symbol-texthuffrefinecustomsize.pdf page 1",
    "bitmap-trailing-7fff-stripped-harder-refine.pdf page 1",
    "issue20439.pdf page 1",
];

/// Every reference returned a flat sheet, and two of the three said they could not draw it.
///
/// The second route of `pdfref::consensus_abstentions`, and the only pages this gate's own
/// population — the pdf.js corpus, `doc/corpora/` and the specification PDFs in `doc/` — has for it.
/// ADR 0769 has the argument; what belongs here is what each page turned out to be and what the
/// move cost.
///
/// A raster of one colour is a picture of nothing, and the first route — ADR 0513's — separates a
/// page that *is* a flat sheet from a program that failed at one by asking whether any reference
/// that drew marks disagrees. Where **every** reference is flat there is no such reference, and
/// the rasters of a genuinely blank page with one broken renderer are identical to those of a page
/// nobody decoded. What is not identical is what the programs said, and on all four of these
/// `mupdf` and `ghostscript` say in their own words that they produced nothing:
///
/// ```text
///   bitmap-symbol-context-reuse.pdf 1   mupdf  library error: cannot decode jbig2 image
///                                       gs     jbig2dec WARNING failed to decode; treating as
///                                              end of file (segment 0x02)
///   jbig2_file_header.pdf 1             mupdf  library error: cannot complete jbig2 image
///                                       gs     jbig2dec FATAL ERROR decoding image: page has no
///                                              image, cannot be completed
///   poppler-90-0-fuzzed.pdf 12          mupdf  library error: zlib error: invalid bit length
///                                              repeat
///                                       gs     **** Error: Page drawing error occurred.
///   poppler-90-0-fuzzed.pdf 16          mupdf  library error: zlib error: invalid code --
///                                              missing end-of-block
///                                       gs     **** Error: Page drawing error occurred.
/// ```
///
/// `poppler` complains on all four as well — *Unknown segment type in JBIG2 stream*, *Bad dynamic
/// code table in flate stream* — and is deliberately **not** read: nothing in its wording separates
/// a refusal from the tens of thousands of `Syntax Error` lines it writes about defects it recovers
/// from, and a list that took one of its sentences and not the others would be fitted to the pages
/// this round wanted to move (trap 11). So on each of these its flat sheet is the one reading left,
/// and one reading cannot triangulate.
///
/// # The move takes away a manufactured contradiction and three manufactured agreements
///
/// The four divide by what *we* drew, and the rule is right about both halves for one reason.
///
/// `bitmap-symbol-context-reuse.pdf` is the page this rule was built for: we draw the image — 10 950
/// black pixels of 159 600, byte-identical to our render of every other encoding of the same drawing
/// (`tests/jbig2.rs`) — and two flat sheets outvoted us at a worst tile of 144.56 against a bound of
/// 5.00. It was the head of `rank_the_contradicted` and it was [`CONTRADICTED_SHARED_JBIG2_DECODER`]
/// for eight hundred sessions. **No renderer that drew this image could have met that bound**: ADR
/// 0499 reproduced all four of its numbers by comparing our render with a synthetic white sheet of
/// the same size, so the verdict was a statistic of our own raster.
///
/// On the other three **we drew nothing either**, and the gate says so — all three are pages this
/// tree reports as incomplete — so `agrees` there was four programs failing at one file and matching
/// each other exactly, which is the empty picture ADR 0005's inference does not reach. Losing it
/// costs nothing the gate was measuring: over that same population this rule moved **three**
/// agreeing pages, **all three incomplete**, and the count of agreements on pages we call complete is
/// unchanged. That number is the one to check before widening `Reference::refusals`, and the oracle
/// prints the population it would be widened over.
const NOT_COMPARABLE_THE_RENDERERS_SAID_THEY_DREW_NOTHING: [&str; 4] = [
    "bitmap-symbol-context-reuse.pdf page 1",
    "jbig2_file_header.pdf page 1",
    "poppler-90-0-fuzzed.pdf page 12",
    "poppler-90-0-fuzzed.pdf page 16",
];

/// The object two references threw away, and said so in their own logs.
///
/// `issue11549_reduced.pdf` page 1 is 200 × 50. `mupdf` prints *ignoring broken object
/// (70 0 R)* after repairing the cross-reference table and `ghostscript` — without the `-q`
/// this gate passes — *object lacks an endobj*, and both then return white paper; `poppler`
/// draws at 0.9338 of full white against our 0.9501. It was [`CONTRADICTED_REFERENCES_DREW_NOTHING`]
/// until the six-hundred-and-eighty-first session, where the contradiction was two blank sheets
/// agreeing perfectly and the failing mean the gate then printed was our own ink of 12.718 with
/// a factor of ¾ on it (ADR 0499).
///
/// The reading that is left is `poppler`'s, and the line the gate prints now is against it:
/// **mean 1.37** of a bound of 5.00, the two of us drawing the same words. Nothing here is owed
/// unless that changes.
const NOT_COMPARABLE_THE_OBJECT_TWO_REFERENCES_THREW_AWAY: [&str; 1] =
    ["issue11549_reduced.pdf page 1"];

/// Six pages where the flat sheets that used to carry the verdict were hiding a mark **we do
/// not draw**, and this is the half of ADR 0513 that cost an agreement.
///
/// All six were `agrees` before the rule, and each agreed with two references that returned one
/// colour while a third drew. That is the exact shape the rule is about, arriving with the sign
/// reversed: where the JBIG2 family had the flat sheets outvoting a renderer that was right,
/// here they outvote a renderer that may be, and our own raster is one of the flat ones. **Six
/// agreements are the price of seeing them, and they are worth it**: the gate said "PASS —
/// agrees" on every one of these while a fourth renderer drew something we did not.
///
/// **All three documents are read now** (ADR 0520), and on each one the clause that decides it
/// is a different clause. The panel figures below are taken with `-alpha off`, which is
/// `doc/todo/00` step 5's rule and is why `hayro`'s two differ from the ones ADR 0513 recorded
/// by exactly the factor of ¾ an averaged alpha channel costs.
///
/// # `issue17333.pdf` page 1 — §9.6.5.4 runs out, and its last sentence says so
///
/// 100 × 100, one `Tj`, one **character code 0** through `/TT3`: an embedded `SymbolMT` subset
/// of **two** glyphs, `/Encoding /MacRomanEncoding`, and a descriptor whose `/Flags 32` sets
/// Table 121's Nonsymbolic bit. So §9.6.5.4's named-encoding branch applies and the clause's
/// own algorithm is walked to its end: MacRomanEncoding assigns code 0 no glyph name and
/// neither does the `StandardEncoding` fill, so the (3, 1) and (1, 0) rules — both of which
/// begin "A character code shall be first mapped to a glyph name using the table described
/// above" — have nothing to carry, and the `post` fallback has no name to look up. The font's
/// only `cmap` subtable is a (1, 0) format 6 covering **one** code, 165, which is `bullet` and
/// which draws. What is left is the subclause's closing sentence:
///
/// > If a character cannot be mapped in any of the ways described previously, a PDF processor
/// > may supply a mapping of its choosing.
///
/// **A permission, not a requirement**, and this is `doc/todo/00`'s third shape: the clause puts
/// the answer beyond itself and says which sentence does it. `mupdf` and `hayro` supply
/// `.notdef` — glyph 0, whose 100 bytes of `glyf` are the two contours of the classic hollow box
/// — and they agree to **0.004 of 255**, 0.346 against 0.3498. `poppler`, `ghostscript` and this
/// tree supply nothing. `truetype_code_table` states the two mappings this tree does supply and
/// why each is narrower than the code it replaced (ADR 0015); neither reaches code 0 here,
/// because the font *has* a readable `cmap` and that `cmap` does not cover it.
///
/// **What was wrong on this page is that we were silent about it**, and that was a defect of
/// this tree rather than a departure. The page drew zero commands and reported nothing:
/// `Interpretation::codes_without_a_glyph` excluded a code §9.10.2 could not *name*, which is a
/// question about the reader and not about whether the program answered, so the one condition
/// that would have fired — a font that drew nothing of what it was asked to show — could not.
/// The document is on `corpus.rs`'s incomplete list since ADR 0520 and says so by name.
///
/// # `issue18042.pdf` pages 1 to 4 — a `DCTDecode` stream that is four bytes of ASCII
///
/// 1247 bytes, four pages sharing one content stream — `100 0 0 100 146 152 cm /Im1 Do` — and
/// one image XObject declaring `/Width 7300 /Height 7600 /BitsPerComponent 8 /ColorSpace
/// /DeviceRGB /Filter /DCTDecode` over a stream of `1234`. §7.4.1 says "[a] PDF reader shall
/// invoke the corresponding decoding filter or filters to convert the information back to its
/// original form" and §7.4.8 requires that form to be "encoded in the JPEG baseline format in
/// accordance with ISO/IEC 10918 (all parts)"; four bytes of digits are not, there is no
/// original form to convert back to, and **no clause of ISO 32000-2 states any artwork to put
/// in an undecodable image's place**. This tree reports the filter's own refusal by name and
/// draws nothing.
///
/// `mupdf` paints the image's rectangle solid black, which the closed form confirms exactly
/// rather than approximately: the `cm` puts the unit square of §8.9.5.1 on 100 × 100 of a
/// 400 × 400 page, so a black fill is `255 ÷ 16` = **15.9375 of 255**, and 15.9375 is what the
/// panel measures. `poppler`, `ghostscript`, `hayro` and this tree return white.
///
/// # `text_field_own_canvas_calc.pdf` page 3 — §12.7.4.3's regeneration is a splice
///
/// 612 × 792, one read-only `/Tx` widget named *Mirror* with **no `/V`**, under an `/AcroForm`
/// stating `/NeedAppearances true`. Table 224 makes that flag "[a] flag specifying whether to
/// construct appearance streams and appearance dictionaries for all widget annotations in the
/// document (see 12.7.4.3, "Variable text")", and §12.7.4.3's closing paragraph says what
/// constructing one over an existing stream *is*:
///
/// > The interactive PDF processor shall then replace the existing contents of the appearance
/// > stream from / Tx BMC to the matching EMC with the corresponding new contents
///
/// The whole of this widget's stored appearance is `/Tx BMC q 0.85 0.85 0.85 rg 0 0 200 20 re f
/// Q EMC` — every mark of it *inside* the pair — so the splice replaces the lot with what a
/// field holding no value shows, which is nothing. ADR 0032 is where that reading was made and
/// `tests/variable_text.rs` pins both sides of it, including the same clause's opposite answer
/// for a stream whose marks sit outside the pair.
///
/// **The closed form says the other two draw the pre-regeneration artwork and nothing else.**
/// `0.85` at eight bits is 217, the rectangle is 200 × 20 of 612 × 792, and
/// `(255 − 217) × 4000 ÷ 484 704` is **0.313591 of 255**; `ghostscript` and `hayro` both measure
/// **0.313593** with a minimum channel of exactly 217. `poppler`, `mupdf` and this tree return
/// white. So the disagreement is about whether the regeneration runs, and the clause states that
/// it does — a documented departure rather than a page anybody is owed.
const NOT_COMPARABLE_A_MARK_ONE_REFERENCE_DRAWS: [&str; 6] = [
    "issue17333.pdf page 1",
    "issue18042.pdf page 1",
    "issue18042.pdf page 2",
    "issue18042.pdf page 3",
    "issue18042.pdf page 4",
    "text_field_own_canvas_calc.pdf page 3",
];

/// The page where a flat sheet **is** the right answer, which is the limit of ADR 0513's rule
/// stated as a page rather than as a caveat.
///
/// `recursiveCompositGlyf.pdf` page 1 shows *hello world* in §9.3.6's text rendering mode 7 and
/// then paints the page red, expecting to see it through the letters. Its font is a deliberately
/// malformed TrueType whose composite glyph refers to itself, so no outline is produced and
/// §9.3.6's "if the only glyphs shown have no outlines … no clipping shall occur" applies: the
/// page comes out **solidly red**, which is what this tree, `poppler` and `hayro` all draw.
/// `mupdf` refuses the font and returns white; `ghostscript`, with its own TrueType interpreter,
/// recovers the glyphs and draws them. `corpus.rs` carries that reading and this tree reports the
/// page.
///
/// So of the two rasters that abstain here, one is a failure (`mupdf`'s white) and **the other
/// is the page** (`poppler`'s red) — and the rule cannot tell them apart, because the only
/// renderer that drew *marks* is the one whose reading of §9.3.6 the clause does not support.
/// The predicate was not loosened to rescue it, and the reason is that every candidate for doing
/// so reads our own render: a flat sheet is excused where it agrees with us, and the three pages
/// of [`NOT_COMPARABLE_A_MARK_ONE_REFERENCE_DRAWS`] — where our render is the flat one — are
/// exactly what such a rule would hide. **A limit that is one page and named is better than a
/// circularity that is invisible.**
///
/// The page was `ambiguous` before the rule and is not judged now either, so nothing about what
/// the gate can conclude has changed; what changed is which sentence it prints, and the sentence
/// is why this group exists.
const NOT_COMPARABLE_A_FLAT_SHEET_IS_THE_PAGE: [&str; 1] = ["recursiveCompositGlyf.pdf page 1"];

/// The pages judged on **two** readings rather than three, with why the third is missing.
///
/// ADR 0542 made the absence visible — `[judged without: …]` on the page's own line, and a count
/// in the summary — and left the next question, which ADR 0575 asks and answers: *is a consensus
/// of two the same evidence as a consensus of three?* Two references stay enough, and the short of
/// the argument is that ADR 0005's inference is about a **pair** — two implementations sharing no
/// code arriving at one picture — so a third multiplies the improbability rather than creating it.
/// None of these six is `contradicted`.
///
/// **What the six turn out to be about is not the count.** ADR 0541's precondition is that a vote
/// is evidence only where there is a clause the references are both reading, and `CLAUDE.md` says
/// the standard "describes *valid* files and says nothing about the rest". Five of the six lost
/// their third reading because the *document* is outside what ISO 32000-2 describes, so what the
/// two that drew agree about is partly how to **repair** it — which no clause states, and which is
/// the first thing to establish if such a page ever contradicts us.
///
/// Each was reproduced by hand with `tools/pdfref/src/reference.rs`'s own invocation, trap 3
/// binding a measurement taken outside the harness exactly as it binds one inside it:
///
/// - **`GHOSTSCRIPT-698804-1-fuzzed.pdf`** — its §7.5.4 cross-reference subsection header reads
///   `00000004294967296 3`, an object number of 2³². `mutool` repairs the file, reports
///   `non-page object in page tree` and finds **0 pages**; `poppler` and `gs` recover it, and its
///   one content stream is `/Length 0`, so the page they agree about is blank.
/// - **`bug1606566.pdf`** — the file begins `%\xe2\xe3\xcf\xd3`, the binary comment line, with no
///   `%PDF–n.m` header at all (§7.5.2). `gs` stops at file position 14 with `Error: /undefined in
///   obj`; `mutool` and `poppler` repair it, as does this tree.
/// - **`bug_jpx.pdf`** — a JPX stream whose first box is not §7.4.9's JP2 signature box.
///   `pdftoppm` falls back to raw J2K and **dies on a signal**, `opj_int_ceildiv: Assertion 'b'
///   failed` inside OpenJPEG 2.5.4. A refusal is a reading and a crash is not, which is why this
///   one is the reference's failure as much as the file's. The page is `ambiguous` because the two
///   that survive do not agree with each other.
/// - **`issue18986.pdf`** — `cannot find page tree`, **0 pages**, on a file whose `1 0 obj` is a
///   `/Pages` node nothing reaches; its content stream is `/Length 0` too.
/// - **`issue21436.pdf`** — `too many kids in page tree`, **0 pages**.
/// - **`pr6531_2.pdf`** — **the reference is wrong here, and the standard settles it.** `/V 5 /R 6
///   /CFM AESV3`, so §7.6.4.4.11's Algorithm 12 over §7.6.4.3.4's Algorithm 2.B decides
///   authentication; run on this file's own `/O`, `/U` and the empty password it authenticates as
///   the **owner** and not as the user, and §7.6.4.1 says that "should allow full (owner) access".
///   `poppler`, `gs` and this tree open it — `pdf-syntax`'s
///   `encryption.rs::an_empty_password_may_be_the_owner_password` has asserted exactly that since
///   it was written — and `mutool` accepts only the user password `asdfasdf`.
///
/// Printed against the population and **not asserted**, by `doc/todo/05`'s standing rule: a figure
/// enters a gate once it has held across rounds, and membership here depends on the machine's
/// installed renderers as well as on the corpus.
const JUDGED_WITHOUT_A_THIRD_READING: [&str; 6] = [
    "GHOSTSCRIPT-698804-1-fuzzed.pdf page 1",
    "bug1606566.pdf page 1",
    "bug_jpx.pdf page 1",
    "issue18986.pdf page 1",
    "issue21436.pdf page 1",
    "pr6531_2.pdf page 1",
];

/// Ambiguous because the references reached **two** readings of the page, one accepting our
/// render and one not.
///
/// 4 pages, and the group is a verdict rule rather than a mechanism of this tree's. **Agreement
/// is not transitive**, so a page can carry two maximal agreeing sets, neither contained in the
/// other — `a` with `b` and `b` with `c` while `a` and `c` part. The seven-hundred-and-twenty-seventh
/// session found that `pdfref::decide` counted one where a page had two and that the survivor was
/// the order [`Reference`]'s variants happen to be declared in (ADR 0616); the
/// seven-hundred-and-twenty-ninth replaced that order with a rule (ADR 0617): **a verdict about our
/// render is one every maximal consensus reaches**, and where they reach different ones there is no
/// answer to hold us to, which is what `ambiguous` means.
///
/// It is `pdfref`'s own second rule applied at the granularity the first is stated in. Two
/// unrelated implementations arriving at *the* answer is the evidence a contradiction rests on;
/// where they arrive at two, each backed by a coincidence of the same standing and neither set
/// contained in the other, mutual agreement — the only ranking this design has — ranks neither.
///
/// **The control is what makes that a measurement rather than a preference**, and it is ADR
/// 0497's sixth criterion pointed at the room instead of at us: put each *reference* where our
/// render stands and ask what the maximal consensuses it is not a member of conclude about it.
/// On all four pages the set that used to decide the verdict contradicts a voting reference that
/// is itself a member of a consensus — `ghostscript` on two of them, and on both `colors.pdf`
/// pages `poppler` and `mupdf` each contradicted by the other's set. So **on a divided page no
/// renderer in the room, ours or a reference's, is outside every reading the references have**,
/// and a rule holding us to each set in turn would condemn the implementations whose agreement is
/// the evidence it runs on.
///
/// The four, each with the note that holds its reading and the bound it used to fail:
///
/// - **`colorkeymask.pdf page 1`** — [`CONTRADICTED_IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE`], which
///   failed on the **worst tile** and nothing else, 5.03 against 5.00. The sharpest of the four
///   and the one needing no tolerance arithmetic: our render is **byte-identical to
///   `ghostscript`'s** over the whole 595 × 842 page, so `{mupdf, ghostscript}` accepts us by
///   identity while `{poppler, mupdf}` rejects us — and rejects `ghostscript` in the same
///   numbers, since they are the same numbers. That note reads §10.7.4 against the file and
///   concludes the specification answers *for* us; nothing in it moves.
/// - **`colors.pdf` pages 1 and 2** — [`CONTRADICTED_TIGHT_CONSENSUS`], both failing on
///   **structural similarity** and nothing else. `{poppler, ghostscript}` set the bound at
///   0.98862 and 0.98402; `{mupdf, ghostscript}` agree more closely still — 0.99625 and 0.99278
///   against 0.99431 and 0.99201 — and accept us at 0.99627 and 0.99336. Here the division is a
///   division of *camps*: our render and `mupdf`'s compare at 0.99989 and 0.99974 of structural
///   identity — `examples/compare_rasters` between two rasters rather than a figure the gate
///   prints, which is why it is written finer than a per-page line is — while `ghostscript`
///   straddles, and `poppler` is what parts from both. Under each set the other's outer member is
///   contradicted, which is that note's own control clause arriving on the reference it was
///   measured against.
/// - **`issue11403_reduced.pdf page 1`** — [`CONTRADICTED_SUBSTITUTED_FONT`], which failed on the
///   **differing fraction**, 6.24% against a class bound of 5.00%. **This one is not flattered by
///   the rule and the group would misread it if the entry did not say so.** The division here is
///   of *width* rather than of camps: `poppler` is in both sets, and we sit 6.24%, 6.14% and 5.20%
///   of channels from `poppler`, `mupdf` and `ghostscript` — further from every reference than any
///   two of them are from each other. The page leaves `contradicted` because `{poppler,
///   ghostscript}` part from `mupdf` at 5.16% and their own 4.815% spread doubles to a bound that
///   admits us, not because anything about our render improved; and `mupdf`'s membership of the
///   set that used to decide it is a stray acute accent 32 device columns to the left of the line,
///   which its note recorded before any of this. The cap-height measurement is the page's
///   diagnosis and it is unchanged: Liberation Sans at 0.6875 em against `NimbusSans` at 0.729167.
///
/// **`issue19633.pdf page 1` is the page that shows the rule discriminating** and is deliberately
/// not here: it carries two maximal consensuses as well, and both of them reject us, so it stays
/// contradicted. The condition is the sets *disagreeing*, never their number —
/// `pdfref::tests::two_maximal_consensuses_that_concur_still_reach_a_verdict` is that property
/// held against a fixture.
///
/// **And its two sets convict it at very different prices**, which ADR 0636 measured when it built
/// the pool's ratio ranking: the taken pair holds the page 2.30 times outside its bound and the
/// rival 1.12 times, because the tighter pair's agreement leaves the bound at the class floor while
/// the wider pair's widens it. [`CONTRADICTED_NEGATIVE_LINE_WIDTH`] carries the table. It is the
/// same rule as this list's — a verdict is what every set reaches — read for *how far outside*
/// rather than for *which way*, and it is why that ranking takes the mildest set's number.
const AMBIGUOUS_DIVIDED_CONSENSUS: [&str; 4] = [
    "colorkeymask.pdf page 1",
    "colors.pdf page 1",
    "colors.pdf page 2",
    "issue11403_reduced.pdf page 1",
];
/// Every page fewer than two references produced a comparable picture of, in one list.
fn not_comparable_expected() -> Vec<&'static str> {
    NOT_COMPARABLE_ENCRYPTION_TWO_REFERENCES_DECLINE
        .iter()
        .chain(&NOT_COMPARABLE_ONE_REFERENCE_REBUILT_THE_FILE)
        .chain(&NOT_COMPARABLE_NO_REFERENCE_REACHES_A_PAGE)
        .chain(&NOT_COMPARABLE_TWO_REFERENCES_RAN_OUT_OF_TIME)
        .chain(&NOT_COMPARABLE_A_SHARED_JBIG2_DECODER_RETURNED_ONE_COLOUR)
        .chain(&NOT_COMPARABLE_THE_RENDERERS_SAID_THEY_DREW_NOTHING)
        .chain(&NOT_COMPARABLE_THE_OBJECT_TWO_REFERENCES_THREW_AWAY)
        .chain(&NOT_COMPARABLE_A_MARK_ONE_REFERENCE_DRAWS)
        .chain(&NOT_COMPARABLE_A_FLAT_SHEET_IS_THE_PAGE)
        .copied()
        .collect()
}

/// Ambiguous because the two references that share a JBIG2 decoder both fail, and fail
/// *differently*.
///
/// 19 pages, and every one of their names contains `refine`. This is
/// [`CONTRADICTED_SHARED_JBIG2_DECODER`]'s finding one verdict over: `mupdf` and
/// `ghostscript` are one implementation on a JBIG2 page because both link `jbig2dec`, and
/// on a refinement region it gives up — `unhandled segment type 'intermediate generic
/// region' (NYI)` on eleven of them, `failed to find reference bitmap` on the halftone one,
/// and `encountered unpopulated huffman table entry` or `ran off the end of the entries
/// table!` on the seven Huffman-coded ones. Where those seven pages have the two of them
/// failing the *same* way, so that they agree with each other and contradict us, these
/// nineteen have them failing in two different ways: **`mupdf` renders the page black on
/// every one of the nineteen and `ghostscript` renders it white on eighteen** — on
/// `bitmap-halftone-refine.pdf` it keeps what it decoded before the refinement segment, at
/// a mean of 0.9296 against our 0.9314. So no two references agree and there is nothing to
/// hold us to.
///
/// That is the whole reason these sit here rather than in `agrees`, and it is a property of
/// the gate rather than of the rendering. `poppler`, which has its own decoder, is within
/// 715 pixels of 159 600 of our render on sixteen of the nineteen, and within 1 534 on a
/// seventeenth.
///
/// The two it is 12 500 pixels away on are `bitmap-refine-tpgron.pdf` and
/// `bitmap-refine-template1-tpgron.pdf` — typical prediction for a generic refinement
/// region — and there its render is *visibly* broken: a block of noise above the drawing
/// and the glyph shapes torn. Ours is the same picture as the other twenty-two encodings of
/// it, to the byte. **That is the corpus stating an invariant about
/// itself**, which is what `tests/jbig2.rs` gates: ninety-six documents encode one image
/// through every coding mode ISO/IEC 14492 defines and all ninety-six decode to
/// byte-identical pixels here. A decoder wrong about refinement could not produce that, and
/// no amount of agreement with `poppler` would say as much.
///
/// They stay listed rather than excused: if `jbig2dec` learns refinement they will leave
/// this group, and if our decode changes they will change with it.
///
/// # Eighteen of the nineteen left in the six-hundred-and-eighty-first session, and the
/// mechanism is unchanged
///
/// Everything above is still true of these pages; what changed is what the gate does with a
/// raster of one colour. A page black in `mupdf` and white in `ghostscript` is two rasters
/// neither of which has a mark on it, so under ADR 0513 both abstain — the abstention's
/// precondition being that `poppler`, which drew, disagrees with each of them — one reading is
/// left, and the verdict becomes `not comparable`. That is
/// [`NOT_COMPARABLE_A_SHARED_JBIG2_DECODER_RETURNED_ONE_COLOUR`], and it is worth the move
/// rather than being merely equivalent: trap 9's fifth shape is *shared code manufacturing the
/// absence of a consensus*, and inside `ambiguous` — a bucket whose own definition is that the
/// specification may be the reason — it was indistinguishable from a genuine disagreement.
///
/// **`bitmap-halftone-refine.pdf` is the one that stays**, for the reason the paragraph above
/// already gives: `ghostscript` keeps what it decoded before the refinement segment, so its
/// raster carries marks, nobody abstains, and the three references genuinely disagree.
const AMBIGUOUS_SHARED_JBIG2_DECODER: [&str; 1] = ["bitmap-halftone-refine.pdf page 1"];

/// Ambiguous, and the specification settles what it can: it forbids what every renderer here
/// does, including us.
///
/// `issue7229.pdf` is a 1654x2338 photograph placed on a 596x842 page, so every renderer
/// reduces it by 2.8 and no two reductions are alike. §10.7.4 is not silent about that —
/// "there shall not be averaging over the pixel area" — and this tree departs from it
/// deliberately for a *reduction*, with the argument and the cost written down in ADR 0025:
/// the clause describes scan conversion of a shape, point-sampling a photograph at a third of
/// its resolution throws away five sixths of the samples, and the departure is recorded as a
/// departure rather than dressed up as a reading. None of the four renderers point-samples
/// either, and each averages differently.
///
/// So the clause determines that all five of us are departing, and determines nothing about
/// *which* departure — there is no conforming answer for our number to be compared with. The
/// six pairwise measurements are what is left to say, and they say we are inside the others'
/// spread rather than outside it. Mean absolute error over the page:
///
/// | | page 1 | page 2 |
/// |---|---|---|
/// | ours vs `mupdf` | **0.0346** | **0.0168** |
/// | ours vs `ghostscript` | 0.0589 | 0.0228 |
/// | ours vs `poppler` | 0.0776 | 0.0261 |
/// | `mupdf` vs `ghostscript` | 0.0745 | 0.0305 |
/// | `poppler` vs `mupdf` | 0.0750 | 0.0320 |
/// | `poppler` vs `ghostscript` | 0.1035 | 0.0342 |
///
/// On page 2 **every one of our three distances is smaller than every distance between two
/// references**, and on page 1 our closest is half the references' closest. That is what
/// `ambiguous` means when it is telling the truth — and it is corroboration, not the finding.
/// The finding is the paragraph above, and it came from the clause.
///
/// **`issue13372.pdf` page 1 is the same clause over a one-bit raster**, added in the
/// hundred-and-eighty-first session: a 646x761 CCITT *stencil* of a halftone screen, drawn
/// through an axial shading pattern into about 300x390 device pixels. Reducing a halftone by
/// two is where resampling differences are loudest — the screen and the pixel grid beat
/// against each other — and the four renderers produce four moirés. Ours sits closest of all
/// six pairs: 0.0093 from `mupdf`, 0.0233 from `ghostscript` and 0.0461 from `poppler`,
/// against a *best* reference pair of 0.0319. The page drew nothing at all until that session
/// (ADR 0151), which is why it is here rather than in the undiagnosed list.
///
/// **It left the group in the six-hundred-and-sixteenth session by leaving the comparison** — the
/// same trade §9.3.8, §11.6.2 and the four `knockout_*.pdf` made — because its shading states
/// §8.7.4.3 Table 77's `/Background [0 1 1]` and that was reported rather than painted (ADR 0452).
/// **It is back, in the six-hundred-and-eighty-eighth session, and the wash it came back for marks
/// nothing at all** (ADR 0529). That is worth the sentences, because this page was the entry's
/// headline witness in three documents at once and all three said the same wrong thing — that the
/// page's corners project outside `[0, 1]` on that axis, so every marked cell of the stencil beyond
/// the band is cyan in the document and unpainted here.
///
/// **The area to be painted is not the page.** Table 77 fills "those portions of *the area to be
/// painted*" outside the shading's bounds, and this page's area to be painted is one stencil, placed
/// by `0.24 0 0 0.24 0 0 cm` then `1800 0 0 -2400 375 2850 cm` — which is the rectangle
/// (90, 108) to (522, 684), and `/Coords [90 108 522 684]` is that rectangle's own **diagonal**. The
/// axial parameter over it is `t = (432(x − 90) + 576(y − 108)) / 518400`, which is 0 at one corner,
/// 1 at the opposite one and 0.36 and 0.64 at the other two: every point of the stencil is inside
/// the band, so `/Extend` withholds nothing and the wash has zero area. Painting it moves **0 pixels
/// by more than 8 levels** on this page, against 26 690 that move by exactly one — and those are the
/// axial leaving `tiny-skia`'s gradient for `pdf_render::ShadingRaster`'s own evaluation at each
/// pixel centre, which is what a background-carrying shading now takes on all three backends.
///
/// So the page is back on the halftone diagnosis above and on nothing else, and the lesson is trap
/// 1's inverted: **a claim about what a document draws is a measurement, and "the page's corners"
/// is not the clause's phrase.**
///
/// **`freeculture.pdf` page 1 is the third instance and the cheapest diagnosis in the bucket**,
/// added in the hundred-and-ninety-second session. It is the head of the 320-page book that is
/// 42% of the ambiguous bucket on its own, it sat fourth on the printed ranking at 11.84 bounds
/// from the nearest reference, and the interpreter draws it with **one command**: `pdfimages`
/// says why — a single 1366×2048 JPEG with an `/SMask`, reduced into 490×734 device pixels.
/// The cover artwork is a field of horizontal stripes, so the reduction beats the stripe
/// frequency against the pixel grid and four renderers produce four moirés, which is
/// `issue13372.pdf`'s halftone one format over. Pairwise mean absolute difference: **ours to
/// `hayro` 0.0342**, `poppler` to `mupdf` 0.0334, ours to `mupdf` 0.0493, ours to `poppler`
/// 0.0706, ours to `ghostscript` 0.1101 — inside a reference spread that runs from 0.0334 to
/// **0.1710**. The interior pages of the same book are in the bucket for a different reason
/// (nobody embedded its fonts) and are not covered by this group.
///
/// **`issue5747.pdf` page 1 is the fourth and the cleanest measurement of the four**, added in
/// the hundred-and-ninety-fourth session. One command again, and `pdfimages` again: a single
/// **one-bit CCITT scan, 2480×1748, reduced into 420×595 device pixels**. A bilevel scan reduced
/// by six is the loudest case resampling has — every output pixel averages thirty-odd black or
/// white samples, and each renderer's filter weights them differently. Pairwise mean absolute
/// difference:
///
/// ```text
/// ours vs hayro        0.0434     poppler vs mupdf        0.0583
/// ours vs mupdf        0.0458     poppler vs ghostscript  0.0595
/// ours vs poppler      0.0502     mupdf   vs ghostscript  0.0790
/// ours vs ghostscript  0.0618
/// ```
///
/// **Three of our four distances are below every distance between two references**, and the
/// fourth is below the widest of them. There is no consensus here to be wrong about: the three
/// C renderers are further from each other than we are from any of them.
///
/// **Both of the first two pages are here because of the session that put them here.** Until the
/// hundred-and-seventy-seventh this document drew the *second* page as the first and had no
/// second page at all — its first cross-reference section files every entry one object number
/// too high (§7.5.4 with §7.3.10, ADR 0148) — and page 1 sat in the undiagnosed list at 77
/// bounds from the nearest reference, the largest in the corpus. The repair moved it to under
/// 10 and made page 2 render at all, which is why the undiagnosed list gained a name in the
/// same session that lost one.
/// **`jp2k-resetprob.pdf` page 1 is the fifth, and it is here because a *cause was ruled out*.**
/// Added in the two-hundredth session, from the top of the printed ranking at 5.03 bounds. One
/// command again: a 40×27 JPEG 2000 photograph of a sunset drawn into 30×21 device pixels.
/// `opj_dump` says its code-blocks state `cblksty=0x2` — ISO/IEC 15444-1 Table A.19's RESET,
/// which is the coding option the file's name announces and exactly the sort of thing a decoder
/// gets subtly wrong — so the obvious hypothesis was the codec. It is not: `tests/jpeg2000.rs`
/// decodes this codestream **byte-identically** to the reference software's, which leaves the
/// reduction. Pairwise mean absolute difference:
///
/// ```text
/// ours vs hayro        0.0161     mupdf   vs poppler        0.0267
/// ours vs mupdf        0.0215     ghostscript vs hayro      0.0469
/// ours vs ghostscript  0.0353     ghostscript vs mupdf      0.0537
/// ours vs poppler      0.0399     ghostscript vs poppler    0.0682
/// ```
///
/// **Two** of our four distances are below every distance between two references, and a third is
/// below all but one — this paragraph said three, which its own table refutes, and the
/// five-hundred-and-eighteenth session corrected it by re-measuring: `ours vs ghostscript` at
/// 0.0353 is above `mupdf vs poppler`'s 0.0267. A diagnosis that removes a candidate is worth
/// what one that finds a defect is, and unlike reading the picture it is checkable. ADR 0161.
///
/// **And it is the head of [`rank_the_manufactured_ambiguity`]**, at 35.12 bounds between the
/// closest two references — the largest consensus failure in the bucket. That is the same reading
/// from the other side: no pair of them is anywhere near agreeing, so the verdict carries no
/// information about us and the pairwise table above is the whole of what can be said.
///
/// **This paragraph used to set that 35.12 against "our own 5.03 from the nearest", and those two
/// numbers are not comparable** — which is the defect ADR 0643 found by measuring the pool, on
/// this page as its plainest witness. The pair's figure is [`outside_by`], all four bounds; the
/// 5.03 is [`Distance::nearest`], which is three of them and does not include the differing
/// fraction. Over all four, ours from the nearest reference is **32.42** — eight percent from the
/// pair rather than a seventh of it. Both are printed now. The note's conclusion is untouched and
/// the reason is worth the line: it never rested on the contrast, but on the pairwise table above,
/// which is one instrument — mean absolute difference — applied to our four distances and the
/// references' own.
/// **`issue7200.pdf` page 1 is the eighth, and it is the family's cleanest result.** Added in the
/// two-hundred-and-third session at 3.81 bounds, from the top of the ranking. One command:
/// `pdfimages` says a **501×583 four-bit indexed image at 80 ppi** — a whole page of Lorem ipsum
/// rasterised into a picture — drawn into about 451×525 device pixels, a reduction of 0.9. Text
/// resampled just under one to one is where filters differ most and where an eye sees it least.
///
/// Step 6's closed form: `poppler` at 72, 288 and 576 dpi gives 11.37, 11.54, **11.46**, so the
/// geometry is 11.5. Ours is **11.46**, `mupdf` 11.52, `ghostscript` 11.59, `hayro` 11.72,
/// `poppler` 11.37 — everyone within 2% of the truth and of each other, which says the difference
/// is *where* the ink is rather than how much.
///
/// And pairwise it is as clean as this bucket gets: **every one of our four distances (0.0142 to
/// 0.0291) is at or below every distance between two references (0.0267 to 0.0358)**. There is no
/// consensus here to be outside of.
/// **`issue1985.pdf` page 1 is the seventh and the extreme of the family.** Added in the
/// two-hundred-and-third session at 4.10 bounds. Three commands, and `pdfimages` names the
/// subject: an **861×537 one-bit CCITT stencil at 418 ppi drawn into 20×21 device pixels** — a
/// reduction of forty-three, so every output pixel averages about eleven hundred samples and the
/// whole page is 420 pixels.
///
/// The closed form of step 6: `poppler` at 72, 288, 1152 and 3200 dpi gives 2.86, 2.98, 3.05,
/// **3.05**, so the geometry is 3.05. At the page's own scale, ours 3.33, `hayro` 3.23, `mupdf`
/// 3.10, `poppler` 2.86, `ghostscript` 2.35 — a spread of 42% about a limit every one of them is
/// several percent from, in either direction, on a page where a whole device pixel is 0.24% of
/// the ink. `ghostscript` draws a hard black blob where the other four draw a grey smudge, which
/// is a threshold rather than an average. Pairwise, ours sits 0.0058 from `poppler` against a
/// tightest reference pair of 0.0053.
/// **`bug1799927.pdf` page 1 is the sixth, and it is where this group finally got a closed
/// form.** Added in the two-hundred-and-second session from the ranking's top, at 4.57 bounds. It
/// is an A4 CAD drawing whose text is not text: **2 156 of its 2 331 commands are inline
/// one-bit stencils, 2 153 of them 7×10 samples**, drawn at 116 ppi onto a 72 dpi page — so every
/// glyph is about 4.3 × 6.2 device pixels and every one of its samples is well under a pixel.
///
/// # The closed form, and no reference is trusted for it
///
/// Ink is a *geometric* quantity: it is what the page's marks cover, and a renderer's departure
/// from it shrinks as the pixels shrink. So the same renderer at rising resolution converges on
/// the answer, and the limit is the measurement — `poppler` at 72, 288 and 576 dpi gives 12.64,
/// 11.39, **10.82**, and `mupdf` 13.40 → **11.40**. Nobody's *verdict* is being borrowed; two
/// programs are being asked the same question at two scales and only the limit is used.
///
/// Against that, the five renderers at the page's own scale:
///
/// ```text
/// ours 10.94 │ ghostscript 11.70 │ poppler 12.64 │ mupdf 13.40 │ hayro 5.94
///            └ the 576 dpi limit is 10.8
/// ```
///
/// **We are the only one of the five already at the geometry**, within 1% of the limit, while the
/// three C references deposit 8% to 24% more at 72 dpi than *they themselves* deposit at 576.
/// That is not an accusation: §10.7.4 says to paint "any pixel whose half-open square region
/// intersects the shape, no matter how small the intersection is", which is exactly what puts
/// ink where the geometry has none. Ours is ADR 0025's documented departure from that sentence,
/// and on this page the departure is what lands on the truth. `hayro` at 5.94 is 45% *under* and
/// its panel is visibly missing most of the stencil text.
///
/// Pairwise, ours is nearer every reference (0.018 to 0.037) than any two references are to each
/// other (0.029 to 0.041), which is the corroboration rather than the finding.
/// **`stamps.pdf` page 1 is the ninth, and it is the first member this group gained from the
/// *other* end of the instrument.** Added in the two-hundred-and-twelfth session at 1.91 bounds.
/// Two photographs with soft masks — a 512×543 and a 480×400 JPEG at 212 and 227 ppi — drawn onto
/// a 612×792 page, so each is reduced by about three.
///
/// **Every renderer's ink agrees**: ours 10.06, `poppler` 10.19, `mupdf` 10.24, `ghostscript`
/// 10.25, `hayro` 10.15, against a 576 dpi limit of 10.15. A spread of 2% on a page where the
/// pictures are a fifth of the area says the *quantity* is not in question at all, and step 5's
/// closed form has nothing further to say — what differs is where each filter puts an edge.
///
/// That is worth recording as a shape rather than as a page: **the ink instrument answers "how
/// much" and is silent on "where"**, and a page whose ink everybody agrees on is one to open the
/// heatmap for rather than the ink table.
/// **`bug1703683_page2_reduced.pdf` page 1 is the twelfth**, taken off §3a's ranking in the
/// two-hundred-and-sixtieth session at 0.91 from the nearest reference and 2.02 from the
/// furthest. A product diagram: one 322 × 333 indexed image with an 8-bit JPEG soft mask at
/// 300 ppi on a 612 × 792 page, so it is reduced by about four, beside 328 commands of vector
/// artwork.
///
/// The two ladders put ours on the geometry and name the outlier:
///
/// ```text
///                72 dpi   288 dpi   576 dpi
/// poppler        5.4291   5.3723    5.3695
/// mupdf          5.2215   5.2234    5.2258
/// ours (1x/4x/8x) 5.3636  5.3652    5.3638
/// ```
///
/// `poppler` descends onto **5.3695** and ours is flat at 5.364 — **0.006 of 255 apart**, which
/// is the tightest agreement any member of this group has produced. `mupdf` is flat 0.14 *below*
/// both, so it is drawing different marks rather than the same marks differently, and it is the
/// reference the page is about.
///
/// A four-by-four grid of per-tile differences against `poppler` spreads the remaining 0.065
/// over both content regions, largest 0.469 where the photograph and its red labels are — which
/// is `stamps.pdf`'s lesson one page along: the quantity is settled and what is left is where
/// each filter puts an edge.
///
/// **It left this group in the three-hundred-and-eightieth session without being fixed or being
/// wrong**, and the diagnosis above still holds of the page: it started *reporting*, so the
/// oracle stopped judging it. Its `/Luminosity` soft mask blends in `/DeviceGray` and the group
/// draws a `/DeviceN` shading, whose colours rest on `DeviceCMYK` and so reach the mask as the
/// grey of an RGB rather than as §10.4.2.3's grey (ADR 0217). That is a real departure this tree
/// had never named, and the page is the corpus's only witness for it.
/// **`two_pages.pdf` page 1 is the fourteenth**, off §3a's ranking in the two-hundred-and-eighty-sixth
/// session at 0.68 from the nearest reference and 1.38 from the furthest. It is the group's
/// simplest instance and the reason it is worth adding: **one command**, which `open_one` prints
/// and which `doc/todo/00`'s step 4 says has meant one image every time — a 512 × 543 JPEG with a
/// 512 × 543 JPEG soft mask, at 96 ppi on a 612 × 792 page, so it is reduced by about a third.
///
/// ```text
///                 72 dpi   288 dpi   576 dpi
/// poppler        33.5714   33.6059   33.4836
/// mupdf          33.5528   33.4645   33.4944
/// ours (1x/4x/8x) 33.4937  33.4917   33.4917
/// ```
///
/// **Ours is flat to four decimal places** and the two ladders land 0.011 of 255 apart around it,
/// so the quantity is not in question — 0.03% of the page's own ink separates three renderers at
/// eight times the resolution. What the oracle sees at the page's own scale is a worst tile of
/// 6.88 with a mean of 0.55, which is `stamps.pdf`'s sentence again: the ink instrument answers
/// *how much* and is silent on *where*, and where is what a resampling filter decides.
/// **`blendmode.pdf` page 1 is the fifteenth, and it is here because the page's own *name* is a
/// hypothesis this ruled out.** Off §3a's ranking in the three-hundred-and-seventeenth session at
/// **0.46 from the nearest reference and 0.59 from the furthest** — the tightest ratio the tail
/// had left, which step 1 reads as *we are alone*. The corpus's manifest calls it "[e]very blend
/// mode that PDF supports": sixteen labelled swatches, each a 100 × 100 RGB `DCTDecode`
/// photograph with an 8-bit soft mask at **90 ppi**, so every one of the thirty-two images is
/// reduced by 0.8, and 173 commands in all.
///
/// The two ladders converge and ours is already at its own limit:
///
/// ```text
///                  72 dpi   288 dpi   576 dpi
/// poppler         30.7164   30.4748   30.1638
/// mupdf           30.7680   30.2355   30.1531
/// ours (1x/4x/8x) 30.1818   30.0651   30.0703
/// ```
///
/// The references agree with each other at 8× to **0.011 of 255**; ours is flat across three
/// scales and lands 0.09 under them. At the page's own scale — which is what the oracle judges —
/// ours is **0.11 from its own limit** while `poppler` is 0.55 and `mupdf` 0.61 *above* theirs,
/// which is this group's sentence: the reduction is where they part from the geometry and ours is
/// ADR 0025's documented departure landing on it.
///
/// # And no blend mode is the difference
///
/// The residual is worth two more numbers, because a page called `blendmode.pdf` invites the
/// obvious guess. At 8×, mean absolute difference per pixel: **ours against `mupdf` 0.53, the two
/// references against each other 0.67** — we are inside their own spread. And the *signed* ink
/// difference over the same page is 0.09, an eighth of that, so the difference cancels: it is
/// where each renderer puts an edge and not what any of them drew.
///
/// A four-by-eight grid of tile means says the same thing about *place*. The ratio of
/// `|ours − mupdf|` to the tile's own ink is **0.009 to 0.055 over every tile that has ink**,
/// largest on the one tile that is a heading rather than a photograph — glyph edges — and not one
/// of the sixteen swatches is an outlier. Sixteen blend modes, and the difference is spread over
/// all of them in proportion to how much they draw.
/// # A sixteenth, and the tightest limit this group has produced
///
/// `issue269_2.pdf` page 1 came off §3a's ranking in the three-hundred-and-twenty-fourth session
/// at 0.43 from the nearest reference and 1.23 from the furthest. It is a checkerboard: one
/// 200 × 200 `DCTDecode` photograph at **144 ppi** repeated across the page inside `/OC` sections,
/// with crop marks and no text anywhere — so every mark on it is an image reduced by two.
///
/// ```text
///                72 dpi    576 dpi
/// poppler       29.0344   28.48040
/// mupdf         28.7474   28.48050
/// ours (1x/8x)  28.4280   28.46650
/// ```
///
/// **The two ladders converge to 0.0001 of 255 of each other**, which is this group's tightest
/// and the bucket's; ours is 0.014 under it at 8× and 0.052 under at the page's own scale, where
/// `poppler` is 0.554 *over* its own limit and `mupdf` 0.267 over. Same sentence as every other
/// member: the reduction is where they part from the geometry and ADR 0025's departure is what
/// lands ours on it.
///
/// **Its sibling `issue269_1.pdf` is `AMBIGUOUS_DEVICE_CMYK_CONVERSION`**, four rounds earlier and
/// for a reason nothing about the name would suggest: a common stem is a reason to look, not an
/// answer. `issue840.pdf`'s two pages are the same caution inside one file.
/// # A seventeenth, and the largest reduction the group has had, in the three-hundred-and-sixty-third
///
/// `issue12841_reduced.pdf` page 1 stood at the **head** of `doc/todo/00`'s ranking, 0.55 from the
/// nearest reference and 9.36 from the furthest. It is one photograph of a mirror and nothing
/// else: a **5280 × 3792** RGB `DCTDecode` at 144 ppi placed on a 612 × 792 page, so the image is
/// reduced by about nine — three times the largest reduction any other member of this group
/// carries.
///
/// ```text
///                72 dpi    576 dpi
/// poppler       78.7713   78.2404
/// mupdf         78.1782   78.2116
/// ours (1x/8x)  78.1065   78.2101
/// ```
///
/// **Three ladders inside 0.03 of 255**, and ours at 8× is **0.0015 from `mupdf`'s limit**. At the
/// page's own scale — which is what the oracle judges — `poppler` is 0.53 *above* its own limit
/// while ours is 0.10 under and `mupdf` 0.03 under, which is this group's sentence for the
/// seventeenth time. The pairwise distances agree that the odd one out is not us: ours to `hayro`
/// 0.0013 and to `mupdf` 0.0019, while `poppler` is 0.0098 to `mupdf`, 0.0101 to `hayro` and
/// 0.0107 to ours — one renderer 0.01 from everybody, on a page where all five are within 0.66 of
/// 255 in ink.
/// # An eighteenth, and the first whose clause can be evaluated rather than argued
///
/// `issue4379.pdf` page 1 came off `doc/todo/00`'s ranking in the three-hundred-and-seventy-second
/// session at 0.19 from the nearest reference and 3.67 from the furthest. It is **one command** —
/// `q 500 0 0 400 36 406 cm /img1 Do Q` — where `/img1` is a 1000 × 800 one-bit indexed image
/// carrying §8.9.6.3's explicit `/Mask`, itself a 1000 × 800 `CCITTFaxDecode` stencil spelling
/// *Image Mask Example*.
///
/// Every other member of this group is settled by two ladders agreeing on a limit. This one does
/// not need them, because the placement is a **two-to-one reduction onto integer device
/// coordinates**: the image rectangle is device x `[36, 536)` and y `[36, 436)` on a 595 × 842
/// page, so §10.7.4 names one raster sample by sample and leaves nothing to interpret.
///
/// > However, only those pixels whose centres lie within the region shall be painted. The position
/// > of the centre of such a pixel -in other words, the point whose coordinate values have
/// > fractional parts of one-half -shall be mapped back into source space to determine how to
/// > colour the pixel. There shall not be averaging over the pixel area.
///
/// Device column `i` has centre `i + 0.5`, so its source column is `floor(2(i − 36) + 1)`, the
/// **odd** samples, and the same in y.
///
/// **The source samples are settled without trusting anybody.** Rendered at 2× the image covers
/// 1000 × 800 device pixels from 1000 × 800 samples, one to one, and ours, `mupdf` and
/// `ghostscript` are **byte-identical over the whole 1190 × 1684 raster** — `magick compare
/// -metric AE` exactly 0, twice — while `poppler` differs on 3 702 pixels. Subsampling that raster
/// at the odd rows and columns *is* the clause's answer. Against it, at the page's own scale, over
/// 500 990 pixels:
///
/// ```text
///                pixels differing   worst channel   MAE of 255
/// ghostscript                   0               0      0.00000
/// ours                      3 927              94      0.44667
/// poppler                   8 610             126      1.67094
/// mupdf                    11 570              98      0.58306
/// hayro                    35 326             101      0.62554
/// ```
///
/// **`ghostscript` reproduces §10.7.4 exactly and this tree departs on 0.78% of the page**, which
/// is ADR 0025's stated cost measured on a real page for the first time rather than argued from
/// the clause. The ADR's own sentence — "a producer who relied on a particular sample surviving
/// the reduction gets a softened version of it instead of that sample" — is 0.447 of 255 here, and
/// the softening is what the ADR bought `bug1001080.pdf`'s legibility with.
///
/// **The ink cannot see any of it**, which is why the page is in this group rather than fixed: all
/// five renderers land within **0.023 of 255** of each other (ours 7.7404, `ghostscript` 7.7410,
/// `hayro` 7.7438, `mupdf` 7.7555, `poppler` 7.7637), and three ladders converge on **7.74035** to
/// five decimal places at 8× with ours already 0.0001 from that limit at 72 dpi. Step 5's closed
/// form answers "how much" and is silent on "where"; this is the page where only the pixel count
/// speaks. The pairwise distances say the same and say the outlier is not us: ours is 0.143 of 255
/// from `hayro` and 0.152 from `mupdf`, while every pair involving `poppler` exceeds 1.26.
/// # The book's second cartoon, and the page a neighbouring group said it could not explain
///
/// `freeculture.pdf` page 255 arrived here in the seven-hundred-and-sixty-first session off
/// [`rank_the_pages_we_are_alone_on`], where it sits **below** that list's widened-bound mark —
/// the row shape ADR 0684 says is a question about the divisor rather than about us. It had been
/// `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE`'s since the book was diagnosed as a population, and that
/// group's own note had already refused it in as many words: *it is in this list and it has never
/// been opened; whatever it is, it is not the diagnosis these paragraphs make*. It is this
/// group's, for the same reason page 171 is, and page 171 is the sentence one paragraph long that
/// this one measures (ADR 0685).
///
/// **The page is one image.** `pdfimages -list` reports a single `CCITTFaxDecode` stencil,
/// **9258 × 12259** at 2182 ppi, drawn onto 397 × 595 device pixels — a reduction of about 23 in
/// each axis, so every device pixel is the average of some five hundred source samples. The only
/// text on it is a running foot and the margin's line numbers; `pdftotext` returns *BALANCES 247*
/// and the digits 1 to 33. Nothing about it is dense text at book size.
///
/// **Four ink ladders, and they converge on one number:**
///
/// ```text
///                    72 dpi    288 dpi   576 dpi
/// ours (1x/4x/8x)   36.0541    36.1299   36.1291
/// poppler           36.0913    36.0787   36.1436
/// mupdf             36.0617    36.1426   36.1485
/// ghostscript       36.2774          -   36.0832
/// ```
///
/// Four limits inside **0.066 of 255**, ours between `ghostscript`'s and `poppler`'s, and at the
/// page's own scale all five are inside 0.74 (`hayro` 36.7909 is the widest). *How much* ink the
/// page states is agreed by four independent programs to a fifteenth of a level. **The two-hundred
/// -and-thirty-third session had already measured that** and wrote it into the group this page is
/// leaving — `36.144 / 36.149` for the two reference limits against ours, `hayro`'s, `poppler`'s,
/// `mupdf`'s and `ghostscript`'s panels — and every figure of it reproduces here to the
/// thousandth, five hundred rounds later. What the gate
/// measures is *where* it lands, and on a 23-to-1 reduction of a bilevel scan that is the choice
/// of reduction rule and nothing else — this group's subject exactly.
///
/// **And the renderer that is alone is not us.** `examples/compare_rasters` over the gate's own
/// panels, all ten pairs, mean of 255 and structural similarity. **These are that example's numbers
/// and not this gate's** — the gate's line is our render against the *worst* member of a consensus
/// and every row below is one named pair, which is why `--bin quoted` reports the whole table and
/// is right to (ADR 0663):
///
/// ```text
/// ours vs poppler          7.7509  0.89105        poppler vs mupdf        5.0690  0.95142
/// ours vs mupdf            6.8331  0.90454        poppler vs hayro        9.5472  0.84007
/// ours vs hayro            6.4309  0.92111        mupdf vs hayro          9.9300  0.83218
/// ours vs ghostscript     16.6313  0.75319        poppler vs ghostscript 16.2858  0.73813
///                                                 mupdf vs ghostscript   15.1531  0.78301
///                                                 ghostscript vs hayro   16.0906  0.75754
/// ```
///
/// Averaged over the other three, **ours is the most central of the four smooth renderers** — 7.00
/// against `mupdf`'s 7.28, `poppler`'s 7.46 and `hayro`'s 8.64 — while `ghostscript` is 15.2 to
/// 16.6 from every one of them. `magick identify -format '%k'` says why in one number: the other
/// four panels carry 255 or 256 distinct levels and `ghostscript`'s carries **20**, so it is
/// quantising the reduction where they average it. The page is `ambiguous` because the four that
/// average do not agree with each other either, and the closest two of the voting three —
/// `poppler` and `mupdf` at 1.01 floors — are what the ranking divides by.
/// # The book's cover, which is on the same ranking's head and has never been priced
///
/// `freeculture.pdf` page 1 has been in this group without a measurement of its own, and the
/// seven-hundred-and-sixty-first session took it because it is **marked `[widened: outside]`** on
/// [`rank_the_pages_we_are_alone_on`] — 11.69 ours over 6.39 between `poppler` and `mupdf`, which
/// is 1.83× and below that list's readable cut while the per-measure test fires (ADR 0684).
///
/// It is one 1366 × 2048 `DCTDecode` photograph at 201 ppi under an `/SMask` that is white
/// everywhere, drawn onto 490 × 735 — a reduction of 2.79 in each axis. The cover art is a field
/// of near-horizontal rules whose period is 38 to 41 source rows over four sampled columns, so
/// about **14 device rows**, and the lettering is inside the JPEG.
///
/// **Two things about the numbers follow from that last clause and are worth having in this order.**
/// The page carries no glyphs, so `Interpretation::glyphs` earns it the **vector** tolerance — mean
/// 1.00, worst tile 5.00, similarity 0.99 — and a page that is nothing but high-contrast image
/// edges is then held to the bounds measured on flat fills. Every pair on the page is outside them,
/// which is why it also heads [`rank_the_manufactured_ambiguity`] at 18.33. And the measure our own
/// number is taken on is the **similarity**, against `mupdf`: 0.88307, where the pair the ratio
/// divides by is at 0.94337 and misses on its *mean* instead. Numerator and denominator are two
/// different measures of one reduction. (Both figures are `examples/compare_rasters`' over the
/// gate's own panels, one named pair each, and not the gate's line — ADR 0663.)
///
/// ```text
///                     72 dpi    288 dpi   576 dpi
/// ours (1x/4x/8x)    166.119    166.364   166.607
/// poppler            166.499    166.770   166.774
/// mupdf              164.939    166.844   166.620
/// ghostscript        166.589          -   166.096
/// ```
///
/// Four limits inside **0.68 of 255**, ours between `ghostscript`'s and `mupdf`'s. At the page's
/// own scale ours is 0.49 under its own limit where `mupdf` is 1.68 under and `ghostscript` 0.49
/// over, so of the four this tree is the *second* nearest the geometry it converges on. Taking each
/// renderer's mean distance of 255 from the other three, ours and `mupdf` are joint most central at
/// **14.55**, `poppler` is 17.49 and `ghostscript` 27.19 — on the page a ratio calls ours.
///
/// **And the picture says the same and says where.** Differencing the panels pairwise leaves ink on
/// the rules' edges and on the letterforms' edges and nowhere else, and ours-against-`mupdf` and
/// `poppler`-against-`mupdf` leave the *same* pattern. This group's sentence, on a page where every
/// pixel is an edge.
const AMBIGUOUS_IMAGE_REDUCTION: [&str; 18] = [
    "issue4379.pdf page 1",
    "issue12841_reduced.pdf page 1",
    "issue269_2.pdf page 1",
    "blendmode.pdf page 1",
    "two_pages.pdf page 1",
    // Taken out of the book beside it in the two-hundred-and-sixty-second session, by the band
    // rather than by the ranking: its worst tile is 81.57 where nothing else in `freeculture.pdf`
    // exceeds 29.09. The cartoon is a one-bit stencil, `ghostscript` thresholds it to a black
    // blob where the other four draw a grey halftone — this group's own sentence, one book over —
    // and the ladders put ours between the two references' limits: ours 15.5059 at 8x against
    // `poppler` 15.4710 and `mupdf` 15.5141 at 576 dpi.
    "freeculture.pdf page 171",
    // Taken out of the same book in the seven-hundred-and-sixty-first, and by the group note that
    // had it rather than by a band: it said in as many words that whatever page 255 is, it is not
    // dense text at book size. It is the book's other full-page cartoon — one 9258x12259 CCITT
    // stencil reduced 23 to 1 — and the section above has its four ladders (ADR 0685).
    "freeculture.pdf page 255",
    "bug1799927.pdf page 1",
    // Arrived in the judged set in the two-hundred-and-eighteenth session, when the stencil it
    // is made of stopped being refused: a 421x320 one-bit CCITT scan drawn into 252x191 points
    // through a tiling pattern, which is this group's subject with two clauses in front of it.
    // Two ladders agree on the geometry — `poppler` 1.228 at 576 dpi and 1.227 at 2304, `mupdf`
    // 1.235 at 576 — and at the page's own scale ours is 1.236, `hayro` 1.274, `poppler` 1.218,
    // `mupdf` 1.205, `ghostscript` 1.091. Ours is 0.009 from the limit and nearest of the five.
    "issue13561_reduced.pdf page 1",
    // Back in the six-hundred-and-eighty-eighth session, on the halftone reduction it always had:
    // the `/Background` it left over is painted now and marks nothing, for the reason above.
    "issue13372.pdf page 1",
    // Priced in the seven-hundred-and-sixty-first session, having been in this group unmeasured:
    // the cover, one 1366x2048 JPEG of horizontal rules reduced 2.79 to 1, held to the vector
    // tolerance because its lettering is inside the image. The section above has the ladders.
    "freeculture.pdf page 1",
    "issue5747.pdf page 1",
    "issue7229.pdf page 1",
    "issue7229.pdf page 2",
    "issue1985.pdf page 1",
    "issue7200.pdf page 1",
    "jp2k-resetprob.pdf page 1",
    "stamps.pdf page 1",
];

/// Ambiguous, and the specification says how far it can be settled — which is not all the way.
///
/// **What the standard determines about this page, it determines completely.** The samples are
/// ordinary CMYK: §8.9.5.2's `/Decode` is where a PDF states a sample's polarity, Table 88's
/// default for `DeviceCMYK` is the identity, and this file states no array. That half was
/// wrong until the hundred-and-seventy-eighth session and is not a matter of opinion —
/// ADR 0149.
///
/// **What is left is a place the standard puts beyond itself, in as many words.** §10.4.2.1
/// ranks two answers for turning four ink concentrations into a pixel: §10.3's colour
/// management for an ICC-enabled processor, which this is, and §10.4.2.2 to §10.4.2.5's
/// "crude approximations" for anything less. On the higher branch §10.3.1 says the choice of
/// destination profile is "beyond the scope of this document", and its NOTE names
/// "assumptions made by the PDF processor software" as one way to make it. So there is no
/// number a renderer can be *checked against* here, and four renderers assuming four presses
/// is the specification working as written rather than anybody being wrong. `colour.rs`'s
/// `CMYK_CORNERS` is this tree's assumption, argued and measured there; §10.4.2.5's formula
/// was tried against the whole oracle and is worse, which is what §10.4.2.1 says to expect.
///
/// That is the whole answer, and the pairwise numbers below only corroborate it. Mean
/// absolute error over `cmykjpeg.pdf` page 1, all six pairs:
///
/// | | |
/// |---|---|
/// | ours vs `ghostscript` | **0.00128** |
/// | ours vs `mupdf` | 0.00160 |
/// | ours vs `poppler` | 0.00211 |
/// | `mupdf` vs `ghostscript` | 0.00097 |
/// | `poppler` vs `mupdf` | 0.00210 |
/// | `poppler` vs `ghostscript` | 0.00253 |
///
/// The tightest pair is the two that share a profile, which is trap 9's second shape and ADR
/// 0048's finding; we sit closer to both of them than `poppler` does to either. That is
/// evidence that assuming standard process inks is the conventional reading of §10.3.2's
/// licence. It is not evidence that our numbers are right, because the clause states nothing
/// for them to be right against.
///
/// **This page was where the ambiguous ranking earned its keep for the second time.** Until
/// the hundred-and-seventy-eighth session the photograph was drawn *black*, because
/// `zune-jpeg` was left to convert the four-component codestream and applied the
/// standalone-JPEG inversion; the page sat at 30 bounds from the nearest reference inside a
/// verdict nothing watched. ADR 0149.
/// # A second page, and it is the cleanest statement of the same thing
///
/// `issue269_1.pdf` page 1 came off §3a's ranking in the three-hundred-and-twentieth session at
/// 0.47 from the nearest reference and 1.36 from the furthest. It is 100 × 100 points of
/// Illustrator vector art — **three** commands, three `/OC` sections, one of them switched off by
/// the default configuration — and every mark in it is a `k` operator:
///
/// ```text
/// /OC /MC0 BDC  0 0 0 1 k          EMC   the black 1
/// /OC /MC1 BDC  1 1 0.149 0.188 k  EMC   the blue 2
/// /OC /MC2 BDC  0.761 0 1 0 k      EMC   a green layer the /OFF array hides
/// ```
///
/// So the page's ink is two colours and their edges, and the two can be separated. At 2304 dpi
/// the whole raster is three colours, and the histogram is the measurement:
///
/// ```text
///            the 1              the 2
/// ours       (35, 31, 32)       (38, 40, 108)
/// poppler    (35, 31, 32)       (38, 40, 108)
/// mupdf      (34, 31, 31)       (40, 37, 111)
/// ```
///
/// **Ours and `poppler` are byte-identical on both** and `mupdf` is two to three levels away on
/// each channel, which is this group's subject: §10.4.2.1 ranks §10.3's ICC route above
/// §10.4.2.5's formula, §10.3.1 puts the destination profile "beyond the scope of this document",
/// and a renderer's answer is its own assumption about a press.
///
/// The ink ladder says the same thing and separates the colour from the edges:
///
/// ```text
///              72 dpi   288 dpi   576 dpi   1152 dpi   2304 dpi
/// poppler     28.7324   28.4165   28.3613    28.3280    28.3097
/// mupdf       28.4492   28.4290   28.4263    28.4243    28.4233
/// ours        28.2459   28.2788   28.2808    28.2878    28.2880
/// ```
///
/// **Ours is flat from 4× onwards** — an area-exact rasteriser has nothing left to converge — and
/// `poppler` descends steadily towards it, its excess halving with each doubling exactly as
/// §10.7.4's "paint any pixel the shape intersects" predicts. `mupdf` is flat too, 0.135 *above*,
/// and a flat offset is not a scan-conversion difference at all: the difference image against it
/// is the *interiors* of both glyphs at 2 to 6 levels, not their outlines. Two renderers agreeing
/// on the geometry to the byte while a third is uniformly darker is a colour result wearing an
/// ink measurement's clothes.
///
const AMBIGUOUS_DEVICE_CMYK_CONVERSION: [&str; 2] = [
    "cmykjpeg.pdf page 1",
    "issue269_1.pdf page 1",
    // `function_based_shading_cmyk.pdf` page 2 was here from the six-hundred-and-eighty-eighth
    // session to the six-hundred-and-ninety-fourth and is back in
    // [`CONTRADICTED_DEVICE_CMYK_CONVERSION`], which is where it had always belonged: the
    // consensus it was said to have lost is `mupdf` and `ghostscript` 0.192% of channels apart,
    // and the run that could not see it had rendered no `ghostscript` for this page. ADR 0542.
];

/// Ambiguous, and settled outright by §10.7.5's last sentence.
///
/// **This is the group that shows what an ambiguous verdict is worth.** The references split
/// two against two and there is no consensus to hold anybody to — and the specification
/// decides the page anyway, in one sentence, with the document's own dictionary supplying the
/// condition:
///
/// > If stroke adjustment is enabled and the requested line width, transformed into device
/// > space, is less than half a pixel, the stroke shall be rendered as a single-pixel line.
///
/// `bug1743245.pdf` is a page of squared paper with handwriting on it, and its object 4 is
/// `<< /AIS false /CA 1 /SA true /SM 0.02 /SMask /None /Type /ExtGState /ca 1 >>`, applied by
/// the content stream's first `/FXE1 gs`. So stroke adjustment **is** enabled. The grid lines
/// transform to well under half a device pixel — `mupdf` covers one of them at 45 of 255 and
/// `ghostscript` at 68, which is 0.18 and 0.27 of a pixel — so the sentence applies and each
/// line shall be one pixel wide.
///
/// Measured on row 400 of the 445x594 raster, and the two camps are unmistakable. Mean grey
/// over the whole page: **ours 0.8630, `poppler` 0.8602, `hayro` 0.8625** against **`mupdf`
/// 0.9537 and `ghostscript` 0.9415**. The two that agree most closely with each other are the
/// two that ignore the sentence (0.0162 apart), which is why the gate can conclude nothing;
/// we are 0.0035 from `hayro` and both of us are doing what the clause says.
///
/// **The residual difference from `poppler` is the half of §10.7.5 this tree does not
/// implement, and it is a departure recorded as one.** The clause also says the *coordinates*
/// shall be adjusted; `poppler` grid-fits, so both of its lines are one solid black column,
/// while ours are one solid column and one pair of half-covered ones — the same ink in the
/// same place, differently distributed. `doc/todo/_scan-conversion.md` and §10.7.4's ledger
/// row carry the argument: the non-uniformity grid-fitting removes is an artefact of the
/// binary scan conversion §10.7.4 states, and this tree already departs from that. **This
/// sentence said "the handover's list of departures" and that list has not been in
/// `doc/HANDOVER.md` for a long time**; a pointer written as prose rather than as a path is
/// one `conformance --bin pointers` cannot resolve, which is why it outlived the move.
///
/// # Which measure ranks this page, and it is not the one every paragraph above prices
///
/// The page is **third on [`rank_the_pages_we_are_alone_on`] at 4.13×, marked `[widened:
/// outside]`**, and the gate now names the measure each half of that ratio is taken on. The
/// numerator is 31.43 bounds and the divisor 7.62, and **both are the structural similarity** —
/// ours against `poppler`, theirs between `mupdf` and `ghostscript`. Everything above this line is
/// priced in whole-page mean grey,
/// which is not a measure the gate holds anything to and is not the one the ranking reads. ADR
/// 0688. `examples/compare_rasters` over the gate's own panels — that example's figures, one named
/// pair per row, where the gate's line is our render against a consensus's worst member (ADR 0663)
/// — against this page's *vector* bounds of mean 1.00, worst tile 5.00, similarity 0.9900:
///
/// ```text
///                           mean      ssim     bounds
/// ours vs poppler        17.6275   0.68566    31.43  ← the numerator, on the similarity
/// ours vs mupdf          17.5866   0.55528    44.47
/// ours vs ghostscript    16.4961   0.63480    36.52
/// ours vs hayro           2.8429   0.98976     2.84  ← 1.02 on the similarity, so this is a mean
/// poppler vs mupdf       21.3666   0.37853    62.15
/// poppler vs ghostscript 20.4497   0.44878    55.12
/// mupdf vs ghostscript    3.0929   0.92383     7.62  ← the divisor, on the similarity
/// ```
///
/// **The divisor is the mechanism this note names and the numerator is not.** `mupdf` and
/// `ghostscript` are the closest pair because both leave the grid lines thin, which is the shared
/// gap the camps above are about. But our own nearest reference is `poppler`, which is in *our*
/// camp on the mean and 31 bounds away on the similarity — so the sentence "two camps, and we are
/// in the right one" cannot account for the number that ranks the page. What can is the paragraph
/// above it: **the half of §10.7.5 this tree does not implement.** Same ink, different
/// distribution, is a difference a mean cannot see and a similarity is made of.
///
/// # Priced by taking the mechanism out of the document, and the control is the finding
///
/// `doc/todo/00` step 1's instrument (ADR 0663): make the mechanism unable to act and re-measure
/// **both** halves. `/SA true` was renamed to `/S1 true` in place — eight bytes for eight, so the
/// cross-reference table still resolves, and Table 58's initial value for `SA` is `false`, so the
/// page is the same page with stroke adjustment disabled — and all five renderers were re-run at
/// 72 dpi. The control first: on the *unedited* file, freshly rendered, all four references and our
/// own raster are **byte-identical to the gate's cached panels**, so nothing below is a measurement
/// of a stale artefact.
///
/// ```text
///                        with /SA   without it
/// ours vs poppler          31.43       64.34
/// ours vs mupdf            44.47        2.62   ← our nearest, and the page's whole rank
/// ours vs ghostscript      36.52       10.70
/// ours vs hayro             2.84       50.37
/// mupdf vs ghostscript      7.62        7.62   ← byte-identical
/// ```
///
/// **Not one reference moves by a single bit.** `poppler`, `mupdf`, `ghostscript` and `hayro` all
/// render the `/SA`-free file identically to the original, so on this page the entry decides a
/// pixel for **this tree and for nobody else** — which is the sentence below, measured rather than
/// asserted, and it makes both camps something other than what the note called them: `mupdf` and
/// `ghostscript` never read the entry, and `poppler` and `hayro` widen a sub-pixel stroke whether
/// the document asks for it or not. Our agreement with them was two answers to two different
/// questions (trap 9), in our own camp and with the sign reversed.
///
/// And with the entry gone our nearest falls from 31.43 to **2.62**, against a divisor that did not
/// move at all, so the ratio is 0.34 and **the page leaves that list entirely**. The whole of its
/// rank is one clause we implement and no reference here conditions on. That is not a defect and
/// not a reason to stop: it is `AMBIGUOUS_OVERSIZED_BORDER`'s cost stated once more — obeying a
/// `shall` moves a page *away* from renderers that do not.
///
/// `Stroke::device_width` conditions the rule on `/SA` rather than applying it always, which
/// is what makes this a derivation rather than a coincidence: this is the one page in the
/// corpus where the entry decides a pixel. **How many documents state it is a command rather
/// than a number here** — `pdf-model/examples/absence_audit [--pdfjs|--crawl]`, whose
/// §10.7.5 block asks for the *value* `true` rather than for the name, because the clause's
/// rule fires "[w]hen stroke adjustment is enabled" and a `/SA false` states the entry too.
/// This note said **30** and §10.7.5's ledger row said **49**, neither naming a population
/// and neither naming a command, and the census agrees with neither over any population this
/// tree can measure. ADR 0610.
///
/// **A degenerate fill's mark *does* snap to the grid since the three-hundred-and-sixty-eighth
/// session, and that is not this rule quietly arriving.** The two are separated by the clauses
/// rather than by preference: a stroke has a width the document stated and §10.7.5 makes the
/// adjustment of its coordinates conditional, while a fill with no extent has no width at all
/// and §10.7.4 states which pixels its mark covers with no condition attached — and exempts a
/// zero-width *stroke* from that same rule in the next sentence, "[z]ero-width strokes may be
/// done in an implementation-defined manner that may include fewer pixels than the rule
/// implies". So this page is still measuring what it says it measures. ADR 0208.
const AMBIGUOUS_STROKE_ADJUSTMENT: [&str; 1] = ["bug1743245.pdf page 1"];

/// Ambiguous because two references reduce a discontinuous function to something smooth.
///
/// `function_based_shading.pdf` draws nine type 1 swatches, and the one at the bottom left
/// is object 11: `/Matrix [170 0 0 170 30 182]` over object 20, a §7.10.5 PostScript
/// calculator function whose whole program is
///
/// ```text
/// { 4 mul floor exch 4 mul floor add 2 mod }
/// ```
///
/// **That can be evaluated by hand, and the clause defines every operator in it.** With
/// inputs (x, y) in `[0 1]`: `4 mul floor` is `floor(4y)`, `exch 4 mul floor` is
/// `floor(4x)`, and `add 2 mod` is `(floor(4x) + floor(4y)) mod 2` — a four-by-four
/// checkerboard of 0 and 1. There is nothing here for renderers to have opinions about.
///
/// We draw the checkerboard. So do `mupdf` and `hayro`. **`poppler` draws the swatch solid
/// black and `ghostscript` draws it flat mid-grey**, which is the average of a checkerboard
/// and the signature of sampling a discontinuous function onto a smooth mesh. Two failures,
/// two different pictures, so no consensus and no verdict — trap 9's shape again, and the
/// reason this page can be *settled* while remaining `ambiguous` for ever.
///
/// It is here rather than in `agrees` because of that one swatch: worst tile 191 of 255
/// against a bound of 5, which is exactly a black square against a white one. The rest of the
/// page agrees with everybody.
///
/// **The page's other half was ours and is fixed** (ADR 0150): object 10 states
/// `/Matrix [85 85 -85 85 515 382]`, so its domain is a *diamond*, and §8.7.4.5.2 says points
/// outside the transformed domain "shall be left unpainted". We painted a square. That took
/// the page from 27.57 bounds from the nearest reference to under 6.
const AMBIGUOUS_FUNCTION_SAMPLED_BY_A_REFERENCE: [&str; 1] = ["function_based_shading.pdf page 1"];

/// Ambiguous, and the clause said we were the ones who were wrong — **fixed in the
/// hundred-and-eighty-sixth session**, and the page stays here because what is left of the
/// difference is a departure this project has already argued.
///
/// `issue4260_reduced.pdf` draws a grid of ruling lines as **zero-height rectangles**:
///
/// ```text
/// 848 1085 10159 0 re f
/// 848 1281 10159 0 re f
/// ```
///
/// Three references drew the grid; we drew the surrounding box and nothing inside it. §10.7.4
/// is where the answer is:
///
/// > A shape shall be scan-converted by painting any pixel whose half-open square region
/// > intersects the shape, no matter how small the intersection is. This ensures that no shape
/// > ever disappears as a result of unfavourable placement relative to the device pixel grid, as
/// > might happen with other possible scan conversion rules. The area covered by painted pixels
/// > shall always be at least as large as the area of the original shape. This rule applies both
/// > to fill operations and to strokes with non-zero width.
///
/// **This is not §8.5.3.3.1's degenerate subpath**, which this tree records as a documented
/// departure: that clause defines degenerate as "consists entirely of one or more points at the
/// same coordinates", and these four points differ in x. It is an ordinary zero-area fill, and
/// an antialiasing rasteriser computes its coverage as zero and paints nothing.
///
/// `pdf_render::collapsed` is the rule that now draws it, and its module comment is where the
/// reading is argued — the half-open convention two paragraphs above the sentence quoted here
/// annihilates a shape whose floor and ceiling are one line, and what breaks that tie is
/// §8.5.3.3.1 stating that a *point* encloses a device pixel. A shape cannot mark less of the
/// page than a shape it contains.
///
/// # What is left, measured
///
/// The grid draws, and the page is still `ambiguous` because the difference is now about the
/// lines' **weight** rather than their existence. Ink over the page, as `255 − mean` on the
/// artefacts beside this file: ours **19.74**, `hayro` **19.83**, `ghostscript` 6.29, `poppler`
/// 3.51, `mupdf` 2.16. §10.7.4 asks for the pixel to be *painted* — "no matter how small the
/// intersection is" — which is a full mark and is what the two Rust renderers put down; the
/// three C ones shade it by something under a fifth. That is
/// `CONTRADICTED_ANTIALIASED_EDGES`' departure seen from the other side, and this project's
/// answer is the clause's rather than the consensus's.
///
/// # And *where* the mark goes was the other half, paid in the three-hundred-and-sixty-eighth
///
/// The ink was right and its placement was not. Until that session each mark was a band of one
/// device pixel centred on the shape's own fractional position, so an anti-aliasing rasteriser
/// split it across two rows at every placement but one and the page came out as a mixture of
/// crisp lines and fuzzy grey double ones — the project owner saw it beside Okular on a
/// high-DPI screen. §10.7.4 states the answer twice: a filling region "is considered to
/// intersect every pixel through which its boundary passes, even if the interior of the filling
/// region is empty", and its EXAMPLE says a zero-height rectangle "paints a line 1 pixel wide".
/// `render-quorra/examples/mark_width` measures it: 1–2 rows per line before and **1 row at
/// 1.00 ink on both backends at 1×, 2× and 4×** after. The page's own numbers moved with it —
/// mean 13.31 → 13.09, differing 10.02% → 6.87%, similarity 0.5619 → 0.5835 — and the verdict
/// did not, because the references still disagree with each other about the weight. It is also
/// the only page in the 1794 whose numbers moved at all. ADR 0208. (That pair is what ADR 0208
/// measured and it stays; **the gate prints mean 13.06, differing 7.03% and similarity 0.5842
/// today**, which is later work on the same page rather than a correction to the movement —
/// ADR 0495.)
///
/// The verdict was `ambiguous` before the fix as well, because the references disagree about the
/// weight among themselves, so nobody's pair ever agreed closely enough to contradict us for
/// drawing **none** of it. A page can be plainly wrong inside this verdict, which is §3a's whole
/// argument, and this is the group that demonstrates it.
///
/// # Which measure ranks it, and the ink above turns out to be the right instrument for it
///
/// The page is ninth on [`rank_the_pages_we_are_alone_on`] at 2.90×, marked `[widened: outside]`,
/// and the gate names both halves since ADR 0688: ours is the **structural similarity against
/// `ghostscript`**, 32.84, and the divisor is the **similarity between `poppler` and `mupdf`**,
/// 11.33. Like for like, which is not true of most of that list's head — and the ink table above
/// reaches it, because on a page whose only marks are single-pixel rules *how much* and *where*
/// are the same question. `examples/compare_rasters` over the gate's own panels, that example's
/// figures and one named pair per row (ADR 0663), against vector bounds of mean 1.00, worst tile
/// 5.00, similarity 0.9900:
///
/// ```text
///                           mean      ssim     bounds
/// ours vs ghostscript    11.8958   0.67165    32.84  ← the numerator, on the similarity
/// ours vs poppler        13.0571   0.58418    41.58
/// ours vs mupdf          13.2446   0.54941    45.06
/// poppler vs mupdf        1.4381   0.88670    11.33  ← the divisor, on the similarity
/// poppler vs ghostscript  3.5061   0.75348    24.65
/// mupdf vs ghostscript    3.5340   0.73197    26.80
/// ```
///
/// **The similarity orders the three references exactly as the ink does**: `ghostscript` shades
/// the rules hardest of the three and is our nearest, `mupdf` shades them least and is our
/// furthest. So the mechanism this note names accounts for the measure that ranks the page, which
/// is what ADR 0497's sixth criterion asks and what most of that list's head cannot say. The
/// divisor is the same clause from the other side — `poppler` and `mupdf` are the closest pair
/// because they are the two that shade a full mark down furthest — and 11.33 is a pair that agrees
/// about nothing except how far to depart, which is why this page's 2.90× is small beside its
/// absolute numbers.
const AMBIGUOUS_ZERO_AREA_FILL: [&str; 1] = ["issue4260_reduced.pdf page 1"];

/// Ambiguous, and the page is its own conformance test twice over.
///
/// `issue16038.pdf` is 72.7 by 38.7 points and draws two squares, each `B` — filled with an
/// **uncoloured tiling pattern** in blue and stroked in black at width 0.3985. It headed the
/// undiagnosed ranking at 20.54 bounds from the nearest reference, and this file used to say
/// of it that "the `B` operator fills *and* strokes; we appear to do one of them". **That was
/// wrong and one `open_one` at eight times the scale showed it**: the black border is drawn,
/// the blue rules are drawn, and every difference is about their *weight*.
///
/// # What the document states, as an area
///
/// The two patterns are the same figure at two phases, which is what the file was reduced to
/// demonstrate. `/pgfpat21` has `/BBox [-1.49442 -1.49442 1.49442 1.49442]` and strokes one
/// horizontal line across the middle of the cell; `/pgfpat22` has `/BBox [0 0 2.98883
/// 2.98883]` and strokes one along the *bottom* and one along the top. Both steps are
/// 2.98883, so both put a rule at every multiple of 2.98883 in pattern space — the same rows
/// of the page — and the second states each of them twice, once from the cell below and once
/// from the cell above. Table 74 is what makes the two equal rather than the second twice as
/// heavy: "These boundaries shall be used to clip the pattern cell", and the stroke is 0.3985
/// wide, so each cell contributes the half of the rule that falls inside its own box.
///
/// So the ink the document asks for is computable, and there is no rounding in it: ten rules
/// per square, each 28.3468 long and 0.3985 wide, plus a border of the same width around a
/// 28.3468 square — **316.29 square points**. Measured as `(1 − red) × area` on the oracle's
/// own artefacts, red because the fill is pure blue and the stroke is black, so the red
/// channel is coverage:
///
/// ```text
/// ours, 24× the page's own scale     311.13   98.4% of what the geometry states
/// ours, 8×                           307.81   97.3%
/// ours, 1×                           269.71   85.3%
/// hayro                              438.53  138.6%
/// mupdf                              364.83  115.3%
/// poppler                            495.93  156.8%
/// ghostscript                        945.78  299.0%
/// ```
///
/// **Every reference is above the area and we are the only one below it**, which is the
/// direction §10.7.4 names: "The area covered by painted pixels shall always be at least as
/// large as the area of the original shape." Their excess is that clause working as written —
/// a rule 0.4 of a pixel wide paints a whole pixel — and this tree's anti-aliasing is the
/// documented departure that draws it at 0.4. What is *not* a departure is coming out under
/// the area, and that is what the 1× row is.
///
/// **Our three rows are from before ADR 0155 and ADR 0213**, which is why they are the only
/// ones that move. Measured the same way with `examples/render_at` in the
/// three-hundred-and-seventy-fourth session: **299.89** at 1× (94.8%), **315.18** at 8× (99.6%)
/// and 313.84 at 24×.
///
/// # The two squares are the second instrument, and they need no reference at all
///
/// Interior coverage of each square, ours against the geometry's 0.1333:
///
/// ```text
///              left    right   right/left
/// ours        0.1114  0.1159      1.04
/// hayro       0.1353  0.1367      1.01
/// mupdf       0.1329  0.2170      1.63
/// ghostscript 0.3077  0.6538      2.13
/// poppler     0.2705  0.0821      0.30
/// ```
///
/// The two squares state the same rules, so a renderer whose two squares differ has misread
/// one of the two `/BBox` conventions — and the three C references each differ in a different
/// direction. `mupdf`'s 1.63 is Table 74's clip not applied: the doubled rule painted twice at
/// full width instead of twice at half. Only the two Rust renderers draw the squares alike,
/// and `mupdf`'s **left** square is 0.1329 against the geometry's 0.1333, which is the
/// independent confirmation that the closed form above is right.
///
/// # What the shortfall is, measured by removing the suspect
///
/// The per-cell clip. Deleting it and re-rendering (a probe, not a change): the left square's
/// coverage goes **0.1114 → 0.1323**, within 0.8% of the 0.1333 the geometry states, while the
/// right square doubles to 0.2348 — which is what says the clip is load-bearing there and
/// redundant here. `/pgfpat21`'s rule spans exactly its own box, so clipping removes no
/// geometry at all; what it removes is *coverage*, because the clip mask is anti-aliased and
/// the two halves of a boundary pixel composite as `1 − (1−a)(1−b)` rather than adding.
///
/// **So this was ours, it was not the anti-aliasing departure, and the
/// hundred-and-eighty-eighth session fixed it**: a cell's box is not applied as a clip where it
/// removes no geometry (ADR 0155). The left square is **0.1323** against the geometry's 0.1333,
/// and the page's worst-reference distance went 61.72 bounds to 41.12. What it needed was a
/// bound on a stroke's *outline* rather than on its path — `Command::device_bounds` reaches
/// `width × miter_limit` in every direction, 3.99 units here against a box 2.99 across, so it
/// cannot show containment for the shape that most needs it — and `pdf_render::outline` is that
/// bound.
///
/// # And the right square, taken in the three-hundred-and-seventy-fourth session
///
/// There the clip is load-bearing: the rule sits on the cell's edge and is *meant* to be halved,
/// so each half was drawn by a different cell and the two composited. Removing the clip would
/// draw the rule twice at full width, which is what `mupdf` does; **what the two halves are is
/// one mark**, stated twice by a cell whose figure repeats at a whole `/YStep`, and §11.6.2
/// forbids compositing them — "[p]ortions of an object shall not be composited with one another"
/// — because §11.6.7 makes the tiling one object's paint. So one of the two statements is kept,
/// the box comes off it and the rule is drawn whole (ADR 0213).
///
/// Interior coverage of each square as a fraction of the geometry's own answer, measured over
/// eight whole periods with `examples/render_at` at four scales:
///
/// ```text
///                1×      2×      4×      8×
/// left        0.983   0.985   0.967   1.002
/// right       0.858   0.771   0.901   0.951     before
/// right       0.989   0.971   0.972   1.003     after
/// ```
///
/// The two squares state the same rules and now weigh the same at every scale, which is the
/// instrument that needs no reference. Total ink over the page against the 316.29 square points
/// the geometry states: **287.16 → 299.89** at 1× and **309.14 → 315.18** at 8×.
///
/// The page keeps its `ambiguous` verdict, and for the honest reason: the three C references
/// disagree with each other about the weight by more than anybody disagrees with us. What is
/// left in both squares is 1.5% to 3% at 2× and 4×, and it is the rules' *ends* meeting column by
/// column — the same seam one axis over, one pixel column per three rather than the whole length
/// of every rule.
///
/// # The three-hundred-and-eighty-ninth moved where the ink is and hardly moved how much
///
/// That session made a sub-pixel stroke the fill of its own outline on the processor (ADR 0226),
/// and this page's rule is 0.53 of a device pixel at 1×, so it is one of the marks that changed.
/// Measured over the whole raster with `examples/render_at`, ink in square points, before and
/// after: **284.74 → 283.44 at 1×**, a movement of 0.46%, and **299.02 → 299.02 at 8×** to the
/// digit — the second is the check rather than a spare number, because a 0.53-unit stroke is 4.2
/// device pixels wide at 8× and the rule may not touch it there. (Those two are the *whole*
/// raster; the four-scale table above measures interior coverage over eight periods, and its 1×
/// column predates ADR 0226. It was not re-measured, and it is the only row here that would
/// move.)
///
/// **What moved is the placement, and the instrument that says so needs no reference at all.**
/// `render-quorra/tests/corpus.rs` compares the two backends on this page's own display list, and
/// its mean went **6.5359 → 1.8563** with structural similarity 0.90046 → 0.97723 — the largest
/// movement that gate has recorded. The device was already drawing the rule as its area; the
/// processor was smearing it as a hairline. Against the *references* the same change reads the
/// other way — worst mean 40.55 → 42.78, similarity 0.3935 → 0.3228 — and that is expected here
/// rather than a contradiction: the worst reference on this page is `ghostscript` at 2.13× the
/// geometry, so approaching the geometry is receding from it. (The gate prints worst mean 42.17
/// and similarity 0.3400 today; the pair above is the movement that session measured, and the
/// two figures preceding it are `render-quorra`'s gate rather than this one's — ADR 0495.)
///
/// # The eight-hundred-and-second cannot reach this page, and the file says so in one line
///
/// ADR 0735 gave a stroke whose colour is a tiling pattern the region §11.5.2 states as a
/// group's alpha, so `Interpreter::tile` now takes a `Tiled::Fill` or a `Tiled::Stroke`. **This
/// page takes the first**, and it is not an inference from the picture: its two squares are `B`,
/// the pattern is installed with `scn` and the file contains no `SCN` at all, and the stroking
/// colour in force is the `0 G` set above them. Every arm the eight-hundred-and-second session
/// added is the stroking one — the span from `stroked_bounds`, the shape mask, `CA` for `ca`,
/// and the fifth term in §11.7.5.2's `inside` test — so the fill route is unchanged in every
/// particular and this note's mechanism is what it was. The oracle's line and `doc/todo/00`
/// step 7's gap both reproduce to the digit. ADR 0738.
///
/// # But the closed form above is 1.0% high, and correcting it is what moves this diagnosis
///
/// 316.29 is the twenty rules' area *plus* the two borders', and the two overlap. Each rule is
/// clipped to its square's fill path, so it runs to x = 0 and x = 28.3468 — and the border is a
/// stroke of that same path, straddling it by half a width on each side. The region a rule
/// shares with the border it ends under is `w²`, twice `w/2 × w`, which is 0.15881 apiece; over
/// twenty rules that is 3.18 square points counted twice. **The document asks for 313.12**,
/// confirmed on a 1/1024-point grid over both squares' geometry with no renderer in it — and it
/// is a test rather than a sentence now, which is why it stood wrong for four hundred sessions:
/// `pdf-model`'s `tiling.rs::the_page_that_is_a_closed_form_weighs_what_the_closed_form_says`
/// computes the three terms and holds this page to their sum.
///
/// Re-measured with `examples/render_at` in the eight-hundred-and-sixth session, ink over the
/// raster's own area — which is not the page's, because `pixel_extent` rounds up and a 73rd
/// column of white is inside the mean:
///
/// ```text
/// ours, 24× the page's own scale     313.02  100.0% of the 313.12 the geometry states
/// ours, 8×                           312.75   99.9%
/// ours, 4×                           311.49   99.5%
/// ours, 2×                           308.21   98.4%
/// ours, 1×                           299.86   95.8%
/// mupdf, 1×                          367.62  117.4%
/// hayro, 1×                          429.85  137.3%
/// poppler, 1×                        499.72  159.6%
/// ghostscript, 1×                    953.00  304.4%
/// ```
///
/// **So the sentence "we are the only one below the area" survives and its size does not.** In
/// the limit this tree is *at* the geometry — 313.02 against 313.117, a thirtieth of a percent —
/// and what is left at the page's own scale is 4.2% of anti-aliasing on a rule half a device
/// pixel wide, against a nearest reference 17% over it. The −5.642 of `doc/todo/00` step 7 is
/// therefore the references' excess and not our shortfall, which is what that file's own
/// paragraph on this page predicted and had never measured against a limit of ours.
///
/// # And ADR 0226's owed column, taken
///
/// The four-scale table above was left with a 1× column older than that decision, marked as such
/// rather than guessed at. It is re-measured here by a construction that needs no whole number of
/// pixels: a band from 2.5 to 10.5 periods has its edges in the middle of a white gap and holds
/// exactly the eight rules 3 to 10, so snapping it to whole rows changes its *area* and not its
/// ink, and the quantity is that ink over the `8 × 0.3985 × width` the geometry puts in it.
///
/// ```text
///                1×      2×      4×      8×     24×
/// left        0.971   0.986   0.990   0.994   0.998
/// right       0.980   0.970   0.993   0.994   0.998
/// ```
///
/// The two squares still weigh the same — 1.6% apart at 2× and within 0.9% at the other four —
/// and both climb to the geometry from below without crossing it, which
/// is the ADR 0213 result holding under everything since. The figures are not the old table's
/// row continued: that one was taken over a differently placed band and read above 1.0 at 8×,
/// and two instruments of the same shape are not one instrument.
const AMBIGUOUS_TILING_CELL_CLIP: [&str; 1] = ["issue16038.pdf page 1"];

/// Ambiguous, and it is a page made almost entirely of sub-pixel line work.
///
/// `22060_A1_01_Plans.pdf` is an A1 architectural drawing rendered onto 842×1191 pixels: four
/// floor plans, their hatching, their dimension lines and their annotations. At 13.32 bounds from
/// the nearest reference it was second on the undiagnosed ranking, and the whole of the difference
/// is how heavy a line thinner than a pixel comes out.
///
/// **This paragraph said "nearly all of it strokes narrower than a device pixel" for
/// forty-three sessions and the four-hundred-and-thirty-second counted the page instead.** Its
/// display list is 136 commands: **72 sampled images**, whose combined device footprint is
/// 1 524 354 px² on a 250 916-pixel raster, 24 fills and 40 strokes. Twenty-six of the strokes are
/// sub-pixel and **98% of their length lies within 5° of a device axis**, where a hairline's
/// deficit is 0.3%. So the departure that moves this picture is §10.7.4's *image* paragraph and
/// ADR 0025's area averaging, not its shape rule — which is why ADR 0226 left it unmoved to four
/// decimals and ADR 0268 moves it by 0.06%, and why `doc/todo/11` named it as the witness for a
/// residual it was never a witness to. The clause reading below is unaffected: the file's line
/// weights are what they are, and every renderer here is drawing the same images.
///
/// # What the clause determines, and it is not in our favour
///
/// §10.7.4's stated scan conversion paints "any pixel whose half-open square region intersects
/// the shape, no matter how small the intersection is", so a stroke 0.4 of a pixel wide is a
/// **solid** line — Figure 70 draws exactly that. This tree anti-aliases instead, which is that
/// subclause's first documented departure, licensed by §10.7.1's NOTE that the algorithm "is
/// not defined by PDF" and argued in the ledger and in `CONTRADICTED_ANTIALIASED_EDGES`. On a
/// page of ordinary text and rectangles that departure moves edges; on a page that is *all*
/// edges it moves the whole picture.
///
/// **§10.7.5 does not apply here and that was checked rather than assumed.** The clause that
/// would make a sub-half-pixel stroke one pixel wide is conditioned on stroke adjustment, and
/// this document does not enable it: `/SA` occurs **zero** times in its 6.3 MB, raw and inside
/// every stream that inflates. `bug1743245.pdf` is the page where the same clause *does* apply
/// and decides it (`AMBIGUOUS_STROKE_ADJUSTMENT`); the two together are what make ours a
/// derivation rather than a preference.
///
/// # The measurement, and it splits the renderers into two camps
///
/// Ink over the page, `(1 − mean) × 255` on the greyscale of each artefact:
///
/// ```text
/// ours 10.00   hayro 10.27   ghostscript 10.59  │  poppler 13.49   mupdf 13.75
/// ```
///
/// And mean absolute difference from our render, over the 841×1190 both rasters share:
///
/// ```text
/// hayro 571   mupdf 1478   poppler 1749   ghostscript 2091      (poppler vs mupdf: 1081)
/// ```
///
/// **We agree with `hayro` almost twice as closely as the closest pair of references agree
/// with each other**, and `hayro` is the other renderer here that anti-aliases at a shape's
/// own coverage. The pair that agrees most closely with *each other* — `poppler` and `mupdf`,
/// 2% apart in ink — is the pair that draws a thin line heavier than its coverage, which is
/// §10.7.4 as written.
///
/// **One thing measured and not explained**: `ghostscript` is the *furthest* from us of the
/// four while carrying almost the same total ink, and no whole-pixel shift improves it (all
/// nine offsets are worse than none). So its difference is about *where* rather than *how
/// much* — a plan drawing is exactly the kind of document that carries optional content, and
/// what `gs` does with a configuration is a separate question from this one.
/// **`issue21068.pdf` page 1 is the second, and it is the same clause with a defect in front of
/// it.** Four rows of comb text fields — 164×162, eight commands — whose separators are thin
/// vertical rules. It sat at 2.82 bounds until the two-hundred-and-seventh session, and 8% of
/// that was ours: each separator is a two-point subpath *closed on itself*, so both of its joins
/// double back, and `pdf_render::hull` bounded a join over the miter limit **by the limit**
/// instead of by the bevel §8.4.3.5 converts it to. Every separator was therefore bounded 4.5
/// units outside the `/BBox` that contains it, ADR 0165's redundant-clip rule could not fire,
/// and the anti-aliased clip ate the difference.
///
/// Ink 18.54 → **20.35** against a high-resolution limit of 20.12, and the distance 2.82 → 1.46.
/// `issue21068.pdf` also left `render-quorra`'s differing list, because both backends draw the
/// same display list and there was nothing left to differ about.
///
/// What remains is the group's own subject: ours 20.35, `poppler` 19.72, `mupdf` 19.99,
/// `hayro` 18.57, `ghostscript` 25.56 — a page of rules a fraction of a pixel wide, where
/// §10.7.4 as written paints each of them solid.
///
/// # `vertical.pdf` pages 2 and 3, and they are the clause reduced to two operators
///
/// The whole content stream of page 2, decoded:
///
/// ```text
/// q 1 0 0 1 72 249.02 cm
///   q .1 w -72 71.95 m 177.45 71.95 l S Q
///   q .1 w -72 -248.97 m 177.45 -248.97 l S Q
/// Q
/// ```
///
/// Two rules **a tenth of a user unit wide** across a 249.45 × 321.02 page, one 0.05 below its
/// top edge and one 0.05 above its bottom, and nothing else on the page at all. `/SA` is never
/// set, so §10.7.5's "rendered as a single-pixel line" does not apply and the width stands at
/// a tenth of a device pixel; the closed form is 2 × 249.45 × 0.1 over the page's area, which
/// is **0.159** of 255, and `poppler` at 576 and 2304 dpi gives 0.199 and 0.174, converging on
/// it from above.
///
/// At the page's own scale:
///
/// ```text
/// ours 0.140   mupdf 0.263   ghostscript 0.424   hayro 0.857   poppler 1.578
///                                                            └ the limit is 0.174
/// ```
///
/// **`poppler` paints these rules nine times their area and `mupdf` one and a half times**,
/// which is §10.7.4 as written applied to a shape a tenth of a pixel thick, and it is the same
/// disagreement the two pages above hold. Ours is the only one under the limit, and the reason
/// is measured rather than guessed: see below.
///
/// ## Why ours is under it, and the half of that which was a defect until the 389th
///
/// A synthetic page with the same box and five identical rules — at the top edge, at y 300,
/// 160, 20 and at the bottom edge — says where the ink goes. **Ours read 0.121 until the
/// three-hundred-and-eighty-ninth session** because four of the five carried 0.098 of an
/// expected 0.1 while the one whose edge lay on the page's top carried **0.055**:
/// `tiny-skia` drew a stroke under a pixel wide as a hairline smeared symmetrically about the
/// path — the ladder showed it, since each interior rule split 0.047/0.051 across two rows
/// whatever its sub-pixel position — and for a rule 0.05 above the top edge half of that smear
/// fell above row zero and was lost with the raster.
///
/// A sub-pixel stroke on a straight axis-aligned rule is now the fill of its own outline (ADR
/// 0226), so all five carry **0.098 of 0.1** and this page reads **0.140**. Its own raster at
/// scale 1 is 250 × 322 for a 249.45 × 321.02 page, and the per-row profile is exactly the two
/// rules and nothing else: row 0 at 25 of 255, rows 320 and 321 at 20 and 5, which is the lower
/// rule's 0.1 divided between the two rows its 0.08/0.02 straddle — 0.155 of 255 over that
/// raster where the geometry over the same raster is 0.158.
///
/// **What is left below the limit is the departure and not a loss.** 0.140 against 0.174 is
/// anti-aliased coverage against a clause that paints whole pixels, and the two rules are 0.1
/// of a pixel thick: no arrangement of an eight-bit raster puts more there.
///
/// # `issue11473.pdf`, and it is the same width inside a §8.7.3 tiling cell
///
/// Four swatches of hatching — *crosshatch*, *north east lines*, *north west lines*, *grid* —
/// each painted with a `/PatternType 1 /PaintType 2` tiling pattern whose whole cell is one or
/// two strokes:
///
/// ```text
/// q 0.3985 w 3.08846 0.0 m 0.0 3.08846 l 0.0 0.0 m 3.08846 3.08846 l S Q
/// ```
///
/// **0.3985 user units**, which at the page's own scale is 0.4 of a device pixel — the same
/// sub-pixel width as the rules above, repeated by `/XStep 2.98883` across four small squares.
/// Off §3a's ranking in the two-hundred-and-seventy-ninth session at 0.68 from the nearest
/// reference and 1.35 from the furthest.
///
/// ```text
///                72 dpi   288 dpi   576 dpi
/// poppler        1.1007   0.7769    0.7604
/// mupdf          0.7674   0.7519    0.7543
/// ours (1x/4x/8x) 0.6780  0.7507    0.7516
/// hayro          0.7130      —         —
/// ghostscript    1.2027      —         —
/// ```
///
/// **Two ladders and ours agree on 0.752 to 0.760**, one descending and two ascending, so the
/// geometry is about 0.752 of 255 and every renderer is measuring the same marks. At the page's
/// own scale `ghostscript` paints **60% more** than that and `poppler` **46% more**, which is
/// §10.7.4 as written on a 0.4-pixel stroke; ours is 10% under and `hayro` 5% under, which is
/// the hairline smear this group's `vertical.pdf` paragraph measures. The spread is 0.53 of 255
/// on a page whose whole ink is 0.75.
///
/// The first rung read 0.6753 until the three-hundred-and-eighty-ninth and moved by **0.003**
/// when a sub-pixel stroke became the fill of its own outline (ADR 0226) — the *grid* swatch's
/// two axis-aligned rules and nothing else, because the other three swatches are diagonals,
/// which `pdf_render::sub_pixel` declines. The 4× and 8× rungs cannot have moved and were not
/// re-measured: at those scales a 0.3985-unit stroke is 1.6 and 3.2 device pixels wide, and the
/// rule applies only under one.
///
/// **And this is the page that pays for the four-hundred-and-thirty-second session**, whose
/// subject is exactly those three diagonal swatches. `tiny-skia`'s hairline lays one pixel down
/// per step along the line's *longer* device axis, so a rule at 45° carried `cos 45°` of its own
/// area; ADR 0268 draws such a rule one device pixel wide with the width it gave up in the paint's
/// alpha. Measured with `examples/render_at` at 1×, the page's ink goes **0.6768 → 0.7566** —
/// against our own 8× rung of 0.7516 and the two references' 0.7543 and 0.7604, so ten per cent
/// under the geometry to a twentieth of a level over it. The *grid* swatch does not move, because
/// it is the one ADR 0226 already took.
///
/// Not `AMBIGUOUS_TILED_STROKES`, though it is one document over: that group is about
/// `poppler`'s ladder *drifting* on a tiling pattern rather than converging, and here its ladder
/// converges perfectly well — it is only its first rung that is high.
/// # A whole tax form of it, in the three-hundred-and-third, and the two clusters are the finding
///
/// `prefilled_f1040.pdf` pages 1 and 2 split the five renderers into two groups **2.4 of 255
/// apart** at the page's own scale, which is enormous for a page whose ink is rules and
/// eight-point type:
///
/// ```text
///           ours     mupdf    poppler  ghostscript  hayro
/// page 1   16.9357  16.9049   19.3762    19.3823   19.2577
/// page 2   16.7671  16.6750   18.7574    19.1025   19.2713
/// ```
///
/// Two clusters is the shape that looks like a defect and is not, and only the ladder says which:
///
/// ```text
///            72 dpi    288      576        ours 1x   ours 8x
/// page 1 pop 19.3762  16.6787  16.5338     16.9357   16.6059
///        mu  16.9049  16.6454  16.6316
/// page 2 pop      —        —   16.2016     16.7671   16.2726
///        mu       —        —   16.2843
/// ```
///
/// **`poppler` descends 2.84 of 255 onto its own limit** and `ghostscript` and `hayro` start where
/// it does, while ours and `mupdf` are within 0.33 of the geometry at the page's own scale
/// already and land between the two limits at eight times. So the two clusters are not two
/// readings of the file: they are three renderers painting a hairline at a whole pixel of full
/// ink and two painting it at a pixel of partial ink, which is `Stroke::device_width` and ADR
/// 0028 on this side.
///
/// The page is `ambiguous` rather than agreeing for trap 12's reason with three references on the
/// far side rather than one.
/// # And the group's extreme, in the three-hundred-and-seventh
///
/// `issue7454.pdf` page 1 is a 384 × 111 table of insurance cover — hairline rules and
/// six-point type — and it is where `poppler`'s first rung is highest of anywhere in this bucket:
///
/// ```text
///                 72 dpi    576 dpi
/// poppler        22.3507   13.3415
/// mupdf          13.9130   13.6931
/// ours (1x, 8x)  13.7607   13.5387
/// ```
///
/// **Nine of 255**, which is two thirds of the page's own ink, painted and then given back as the
/// pixels shrink. `hayro` starts at 23.09 and `ghostscript` at 20.02, so three of the four
/// references are up there with it while ours and `mupdf` start within 0.4 of the geometry — the
/// same two clusters as `prefilled_f1040.pdf` and four times as far apart. Ours at eight times
/// lands between the two limits.
///
/// This page is why the group's own caution is worth repeating: at the page's own scale the five
/// renderers span **9.3 of 255** and none of them is wrong about anything.
/// # A ninth, and the reference is 34% over its own limit
///
/// `tiling-pattern-box.pdf` page 1 came off §3a's ranking in the three-hundred-and-thirtieth
/// session at 0.45 from the nearest reference and 2.99 from the furthest. It is a line drawing of
/// a cube on a §8.7.3 tiling-pattern grid — 567 commands, and **0.67 of 255 of ink on the whole
/// page**, which is what a page of hairlines looks like to an ink table.
///
/// ```text
///                 72 dpi    576 dpi
/// mupdf          0.68275   0.669391
/// poppler        0.89469   0.666952
/// ours (1x/8x)   0.66091   0.668751
/// ```
///
/// The two ladders converge to **0.0024 of 255** of each other and ours at 8× lands *between*
/// them. At the page's own scale `poppler` is **34% over** its own limit — the grid is where its
/// "paint any pixel the shape intersects" costs most — and ours is 0.008 under. The four-panel
/// strip says the same thing without a number: the same cube on four grids, and `poppler`'s grid
/// is visibly the darkest.
/// # A tenth, and the per-row profile says *phase* rather than width
///
/// `issue1350.pdf` page 2 came off §3a's ranking in the three-hundred-and-forty-fifth session at
/// 0.39 from the nearest reference and 0.71 from the furthest — the closest pair on the list, which
/// `doc/todo/00`'s step 1 says is the shape that accuses us. It is a 361 × 310 table of rules and
/// small type, and at the page's own scale the five renderers span **0.134 of 255**: ours 11.0347,
/// `poppler` 11.0597, `ghostscript` 11.0760, `hayro` 11.1547, `mupdf` 11.1691.
///
/// The per-row ink profile says what an ink table cannot — the difference is *which row* a rule
/// lands in:
///
/// ```text
/// row       ours   poppler   mupdf   ghostscript   hayro
///  44          2       160      38             2      38
///  46        157         2     129           156     130
///  48          2       155      24             3      25
///  49        153         3     132           150     136
/// ```
///
/// Three answers to one question. Ours and `ghostscript` put each rule wholly in one row;
/// `poppler` puts it wholly in the *adjacent* row; `mupdf` and `hayro` split it across both. No
/// clause decides which — §10.7.4's half-open rule is the departure this whole group records — and
/// a rule one row up is a rule, which is why the page's total moves by a tenth of a level while
/// four of its rows move by 150.
///
/// Step 6, and here the two ladders do not converge on each other:
///
/// ```text
///                72 dpi    576 dpi
/// poppler       11.0597   11.1367
/// mupdf         11.1691   11.3833
/// ours (1x/8x)  11.0347   11.1168
/// ```
///
/// **Ours and `poppler` end 0.020 of 255 apart** and `mupdf` ends 0.267 above both, so the limit
/// worth trusting is the one two ladders agree on and ours is on it. Unlike the rest of this
/// group, nobody's first rung is wildly high: the page's marks are a *whole* pixel wide, so what
/// is being decided is where the pixel goes rather than how many of them there are.
const AMBIGUOUS_SUB_PIXEL_LINE_WORK: [&str; 10] = [
    "issue1350.pdf page 2",
    "tiling-pattern-box.pdf page 1",
    "issue7454.pdf page 1",
    "prefilled_f1040.pdf page 1",
    "prefilled_f1040.pdf page 2",
    "22060_A1_01_Plans.pdf page 1",
    "issue11473.pdf page 1",
    "issue21068.pdf page 1",
    "vertical.pdf page 2",
    "vertical.pdf page 3",
];

/// Ambiguous, and every renderer here is guessing at a face nobody shipped.
///
/// `issue8697.pdf` is 250×50 points and shows one string, *What Operating Systems Do*, in
/// `/SegoeUISymbol` at 18 pt with no `/FontFile2`. It sat sixth on §3a's ranking at 3.52 bounds
/// from the nearest reference and **3.55 from the furthest** — the everybody-against-us shape —
/// because we drew a single `∝`. The name ends in the word "Symbol" and `substitute::family_of`
/// matched the substring before reading the `/Encoding /WinAnsiEncoding` and Table 121's
/// Nonsymbolic flag the file also states, so §9.6.5.4's Latin code-to-glyph-name table was
/// replaced by the standard-14 `Symbol`'s. ADR 0158; the sentence is on the page now and the
/// distance is **0.21 from the nearest, 0.99 from the furthest**.
///
/// # What the clause determines, and where it stops
///
/// It determines the mapping and says so twice. §9.6.5.4: "If the font has a named Encoding
/// entry of either MacRomanEncoding or WinAnsiEncoding , or if the font descriptor's
/// Nonsymbolic flag … is set, the PDF processor shall create a table that maps from character
/// codes to glyph names". §9.6.5.2 says the same of a Type 1 program's built-in encoding. Both
/// are `shall`, both are now implemented, and both are about *which glyph* — not about which
/// face draws it.
///
/// **Which face is left to the processor, in as many words.** §9.8.2 on the Nonsymbolic flag:
/// "This influences the font's default base encoding and *may* affect a PDF processor's font
/// substitution strategies." A *may*, and the clause states no strategy. So the page stays
/// ambiguous by construction: nobody here has `SegoeUISymbol`, and five renderers pick five
/// faces.
///
/// # The measurement, and it is the references that disagree
///
/// Mean absolute difference over the 250×50 raster all five share:
///
/// ```text
/// ours vs mupdf  359      mupdf vs poppler       1768
/// ours vs hayro  618      ghostscript vs poppler 1483
/// ours vs gs     989      ghostscript vs mupdf   1181
/// ours vs poppler 1704
/// ```
///
/// **We are three to five times closer to `mupdf` than any two references are to each other**,
/// which is what makes the verdict ambiguous rather than a contradiction, and it is the bound
/// doing its job (trap 12 in reverse).
///
/// Ink, `-alpha off -channel R` on each greyscale artefact:
///
/// ```text
/// ours 18.43   hayro 18.16   mupdf 18.40   ghostscript 18.62   poppler 18.75
/// ```
///
/// **All five within 0.6 of each other**, which is ink conserved and the difference confined to
/// where the glyphs are — five substitutes for a face nobody shipped, drawing the same sentence
/// in the same places.
///
/// **This entry first read "ours 9.22, hayro 9.08 │ mupdf 18.40, ghostscript 18.62, poppler
/// 18.75" and concluded that three `libfreetype` references were darkening stems.** They were
/// not: our artefacts and `hayro`'s carry an alpha channel and `-colorspace Gray` was averaging
/// it in, halving both. Session 161 found exactly this and recorded it in
/// `CONTRADICTED_GLYPH_EDGES`; the recipe in `doc/todo/00-ambiguous-bucket.md` was not corrected
/// and the two-hundred-and-second session followed the recipe. ADR 0163.
/// # Two more of the same shape, and they add what the clause *does* determine
///
/// `non-embedded-NuptialScript.pdf` (350×50) draws one sentence in `/NuptialScript` at 20 pt
/// with no font program; `bug1671312_ArialNarrow.pdf` (200×50) draws two words in a
/// non-embedded Arial Narrow. Both sat on §3a's ranking at 2.32 and 1.60 from the nearest
/// reference.
///
/// **The positioning is the document's and every renderer here honours it, which is checkable
/// and checked.** Table 109 makes `/Widths` "the glyph width for the character code that equals
/// FirstChar plus the array index", in thousandths of text space. `NuptialScript`'s array gives
/// `N` 778, so at 20 pt the second glyph starts 15.56 points along — and `mutool`'s own
/// structured text puts it at 25.560 against an origin of 10.000. Every one of the five
/// renders places its last mark within two pixels of the others over a 330-pixel line, on both
/// documents. Nobody is inventing metrics.
///
/// **What is left is the face, and §9.8.1 puts that beyond the clause in as many words**:
///
/// > These font metrics provide information that enables a PDF processor to synthesise a
/// > substitute font or select a similar font when the font program is unavailable.
///
/// Two routes, neither required, and this tree takes the second: `substitute.rs` ranks the
/// name, §9.8.3.2's PANOSE and Table 121's flags (ADR 0086), against whatever faces the machine
/// has — deliberately, because ADR 0133 compiled in §9.6.2.2's fourteen and drew the boundary
/// there. **We do not take the first route at all**, and these two pages are what that costs.
///
/// # And this note had the direction backwards on `bug1671312_ArialNarrow.pdf`
///
/// It said "we are the only renderer that finds a *narrow* face at all, and the four that do
/// not draw a better-fitting line", which cannot both be true of one page and is not: **ours is
/// the wide one.** Corrected in the five-hundred-and-eighteenth session, by four measurements
/// that agree and by the picture, which says it in one look — our letters *collide* where the
/// other four have clean gaps between them.
///
/// The file is 1913 bytes and states a whole Table 120 descriptor for a non-embedded
/// `/ArialNarrow`: `/StemV 66`, `/StemH 66`, `/AvgWidth 362`, `/MaxWidth 833`,
/// `/FontBBox [-250 -210 1000 1054]`, `/Flags 32`, and 224 `/Widths`.
///
/// - **The advances and the extent are ours already.** The ink's bounding box is x[10, 149]
///   y[15, 34] in ours against x[10, 147] y[15, 34] in `poppler`'s and `mupdf`'s — 1.4% wider
///   over a 138-pixel line and the same rows — so §9.2.4's advances and the cap height are
///   honoured and are not what differs.
/// - **Inside that same box we mark 983 pixels against `poppler`'s 844, `mupdf`'s 825,
///   `hayro`'s 812 and `ghostscript`'s 702.** Ink over the page: ours 18.45, `poppler` 15.52,
///   `mupdf` 15.32, `hayro` 14.97, `ghostscript` 12.71 — ours 19% to 45% heavier than four
///   renderers that sit within 2.9 of each other.
/// - **At 576 dpi the modal dark run across the x-height band is 14 device pixels in ours and
///   12 in `poppler`'s**, against the `/StemV 66` the file states, which at 20 pt and eight
///   times is **10.56**. Both substitutes are heavier than the descriptor asks for and ours is
///   three times as far over it.
/// - **`hayro` is with the other three**, 10.41 from us, which is the whole argument: it shares
///   `skrifa` with this tree and nothing else, so the choice of face is not a rasteriser
///   question. Four renderers find a condensed face and we do not.
///
/// So a substitute's glyphs can be *wider* than the widths the document states as easily as
/// narrower, and then the difference appears as collision rather than as letter spacing. §9.8.1
/// still states no `shall` and the page is still `ambiguous` for that reason — but the document
/// is the witness `doc/todo/21` item 4 says would open the question, and it is now recorded
/// there.
///
/// The measurement says the references are no closer to each other than to us. Mean absolute
/// difference on `non-embedded-NuptialScript.pdf`:
///
/// ```text
/// ours vs hayro  2924    mupdf vs hayro          422
/// ours vs mupdf  3972    ghostscript vs mupdf   2924
/// ours vs gs     4062    hayro vs poppler       4044
/// ours vs poppler 5833   ghostscript vs poppler 4096
/// ```
///
/// and the 422 is its own small finding: `mupdf` and `hayro` are within a rounding of each
/// other while every other pair is seven to ten times further apart, because those two answer
/// a missing face from a *built-in* rather than from this machine. Trap 9's second shape, seen
/// from outside — ask what data a renderer reads from the machine before crediting its
/// agreement, and ask the same before crediting its disagreement.
/// # Two more, and the first of them states its own answer
///
/// `issue9291.pdf` is 500 × 50 and its one line of text reads **"Non-embedded LucidaSans-Demi,
/// should be bold."** That is a corpus document acting as a conformance test, which outranks
/// every renderer here, and §9.8.1's Table 120 says where the answer comes from: the descriptor
/// gives `/FontWeight 700` and `/StemV 144`, with `/Flags 32` — nonsymbolic, and **no**
/// ForceBold bit, so the weight is the whole of the evidence and it is unambiguous.
///
/// ```text
/// ours 23.47   hayro 24.30   poppler 24.14   mupdf 17.76   ghostscript 15.67
/// ```
///
/// Ours, `poppler` and `hayro` draw it bold; `mupdf` and `ghostscript` draw it regular, and
/// their ink is a quarter lower for exactly that reason. **Two references failing a test the
/// document writes out in words is not a spread to sit inside**, which is why this page is here
/// with a verdict rather than on the undiagnosed list.
///
/// `issue5244.pdf` is 200 × 50 of Polish diacritics in a *non-embedded* `TimesNewRoman,Bold`
/// under `Identity-H`, so §9.7.4.2 leaves the codes reachable only through `/ToUnicode` — which
/// this file has. Ink: ours 15.50, `poppler` 15.60, `ghostscript` 13.17, `mupdf` 19.43 and
/// `hayro` **2.80**, which is `hayro` drawing two of the eleven characters. Every pair in the
/// matrix is 0.021 to 0.069 apart and the smallest of the ten is ours against `poppler`.
///
/// Both pages are this group's standing subject with the numbers to say so: the clause hands the
/// face to the processor, and five processors reach five faces.
/// # Two more, in the three-hundred-and-thirtieth, and the picture is the whole argument
///
/// `issue13343.pdf`'s two pages sat on §3a's undiagnosed list at 0.42 and 0.42 from the nearest
/// reference. Each is **eight commands** — a line reading `( 57)【要約】` — in
/// `Ryumin-Light-90ms-RKSJ-H`, a **non-embedded** Adobe-Japan1 font reached through §9.7.5.2's
/// `90ms-RKSJ-H` `CMap`. The descriptor states `/Flags 6` and `/StemV 69` and **no
/// `/FontWeight`**, so §9.8.1 leaves the weight to the face a processor finds.
///
/// ```text
///           ours    poppler   mupdf
/// page 1   9.2865   6.1379   7.0004
/// page 2  15.1896  11.4200  11.5458
/// ```
///
/// **The ink table is not the finding here and the four-panel strip is.** `poppler` draws
/// `【要約】` and *not* `( 57)`; `hayro` draws `( 57)` and not the ideographs; `mupdf`,
/// `ghostscript` and ours draw the line. So the spread is three renderers drawing different
/// *sets of characters* and, among those that draw all of them, three faces of different weight
/// — Ryumin is a light-weight Mincho and what each of us substitutes for it is whatever the
/// machine has. Ours is the heaviest, which is what 30% more ink over the same glyphs means.
///
/// This group's standing sentence, with a page that states it twice: §9.10.2 says how to learn
/// what a code means and nothing about which face draws it, so five processors reach five faces.
/// The `-UCS2` tables (ADR 0140) are what make the *characters* right; the weight is not theirs
/// to fix.
/// # An eighth, in the three-hundred-and-thirty-third, where one reference draws a third less
///
/// `XiaoBiaoSong.pdf` page 1 — 0.52 from the nearest reference and 2.05 from the furthest — is
/// 693 commands of Chinese in two **non-embedded** `TrueType` fonts whose `/BaseFont` names are
/// themselves mojibake (the bytes of a GB2312 name read as Latin-1). At the page's own scale:
///
/// ```text
/// poppler 10.8844   ghostscript 10.8804   ours 10.7511   hayro 9.7978   mupdf 7.1015
/// ```
///
/// Ours, `poppler` and `ghostscript` agree to 0.13 of 255; `mupdf` is **3.6 below all three**,
/// which is a third of the page's ink and is a face that draws far less of it. Two ladders say
/// the same thing at 576 dpi — `poppler` 10.8095 against `mupdf` 7.1384 — so there is no limit
/// to climb onto here and the finding is the *spread*: §9.10.2 says how to learn what a code
/// means and nothing about which face draws it, and five processors reached five answers, one of
/// them nearly empty.
/// # A ninth, in the three-hundred-and-seventy-ninth, where the clause names the freedom itself
///
/// `issue9084.pdf` page 1 came off `doc/todo/00`'s ranking at **0.16 from the nearest reference
/// and 1.09 from the furthest**, which was the bottom of that list. It is 200 × 50, twelve
/// commands, and shows twelve two-byte codes at 20 points through `/Encoding /Identity-H`. The
/// descendant is a `CIDFontType2` with `/Ordering (Identity)`, a `/W` array, **no `/CIDToGIDMap`**
/// and **no font program**: `/BaseFont /ArialMT` and a descriptor with no `/FontFile2`. A
/// `/ToUnicode` carries every code to a character, and the line reads *SS-2541-03-M*.
///
/// # What §9.7.4.2 determines, and what it hands over
///
/// The clause splits Type 2 CIDFonts by one condition — "TrueType font programs are integrated
/// with the CID-keyed font architecture in one of two ways, depending on whether the font program
/// is embedded in the PDF file" — and this file is the second way. Its bullet:
///
/// > … CIDs shall not participate in glyph selection, and only predefined CMaps may be used with
/// > this CIDFont … The means by which this is accomplished are implementation-dependent.
///
/// `Identity-H` **is** one of §9.7.5.2's predefined CMaps, so the file is conforming and the
/// standard's own last word on the outcome is *implementation-dependent* — `doc/todo/00`'s third
/// shape, stated by the clause rather than inferred from a spread. §9.5's NOTE 5 says the same
/// thing two subclauses up: "some details of font naming, font substitution, and glyph selection
/// are implementation-dependent and can vary among different PDF processors and operating system
/// environments".
///
/// # What the clause does *not* leave open, and the measurement that checks it
///
/// §9.7.4.2 again, two sentences later:
///
/// > Even though the CIDs are not used to select glyphs in a Type 2 CIDFont, they shall always be
/// > used to determine the glyph metrics
///
/// So the *placement* is determined even where the outlines are not, and that half is checkable.
/// The bounding box of the ink, at eight times the page's own scale:
///
/// ```text
/// ours    1022 x 123 at (87, 121)      mupdf       1022 x 123 at (87, 121)
/// poppler 1029 x 118 at (88, 124)      ghostscript 1023 x 119 at (88, 123)
/// ```
///
/// Ours and `mupdf` are identical to the raster pixel over a 1 029-pixel line, and all four span
/// the same width to 0.7%. Nobody is inventing metrics; `/W` is doing what the sentence says, and
/// the letter spacing `poppler` shows is a substitute whose glyphs are narrower than the widths
/// the document states — this group's standing subject on `bug1671312_ArialNarrow.pdf`.
///
/// # And the ladders are flat, which is what makes it this group rather than the next one
///
/// ```text
///                72 dpi     288       576      1152
/// poppler       12.86500  12.88830  12.88730  12.89100
/// ghostscript   12.80070  12.69520  12.67880  12.69980
/// mupdf         14.27660  14.33990  14.35620  14.35980
/// ours          14.43400  14.35070  14.33050  14.35120   (1x, 4x, 8x, 16x)
/// ```
///
/// **Four ladders, four limits, and they are two camps 1.46 of 255 apart** — ours and `mupdf`
/// agree to **0.009**, `poppler` and `ghostscript` to 0.19. A difference that does not shrink with
/// the pixels is not scan conversion, and the pairwise matrix at the page's own scale says it
/// again: ours against `mupdf` is the smallest of the ten pairs by a factor of four, and no other
/// pair is closer than that one. `hayro` is the fifth answer and the picture names it — its ink
/// box is four rows taller and four rows higher than everybody's, because it draws an accented
/// glyph where the other four draw the hyphen.
const AMBIGUOUS_SUBSTITUTED_FACE: [&str; 9] = [
    "issue9084.pdf page 1",
    "XiaoBiaoSong.pdf page 1",
    "issue13343.pdf page 1",
    "issue13343.pdf page 2",
    "issue8697.pdf page 1",
    "non-embedded-NuptialScript.pdf page 1",
    "bug1671312_ArialNarrow.pdf page 1",
    "issue5244.pdf page 1",
    "issue9291.pdf page 1",
];

/// Ambiguous, and the reason is a JPEG 2000 decoder that is measurably wrong.
///
/// `S2.pdf` page 1 is a set of six colour photographs and four greyscale plates; `issue5475.pdf`
/// page 1 is one 512×512 greyscale image. Both sat undiagnosed on §3a's ranking — S2 at 3.07
/// worst mean over 16.06% of the page, `issue5475.pdf` at 6.77 over 31.63% — and both are made
/// almost entirely of `JPXDecode` images.
///
/// # What the standard determines, and it determines all of it
///
/// §7.4.9 says only that the data "shall be" a JPEG 2000 codestream and hands the decoding
/// entirely to ISO/IEC 15444-1, which defines it exactly. There is no latitude here at all: two
/// decoders of one codestream produce the same samples or one of them is wrong. That makes this
/// §3a's *first* shape — the clause determines it and we can be checked against it — and the
/// check is `tests/jpeg2000.rs`, against the reference software ISO/IEC 15444-5 publishes.
///
/// **We are the ones who are wrong.** Every one of `S2.pdf`'s nine quantised codestreams and
/// `issue5475.pdf`'s single one decodes to samples OpenJPEG does not produce, by up to 87 levels
/// of 255 on three quarters of the samples. The discriminator across all thirty corpus
/// codestreams is `qntsty`: the reversible 5/3 path is byte-identical and the irreversible 9/7
/// one is not, with one 316-byte crossing where the difference rounds away. Our samples move
/// toward the image's own mean two to one, which is the signature of an inverse quantisation
/// that reconstructs at the edge of the interval rather than at its middle.
///
/// The defect was `hayro-jpeg2000`'s, is written up for its author in
/// `doc/JPEG2000_FEEDBACK.md`, and **is mostly fixed**: 0.4.0 implemented none of ISO/IEC
/// 15444-1 E.1.1.2's reconstruction bias, upstream `9cce046b` does, and this tree pins that
/// revision. The worst sample error over the corpus fell from 87 levels to 3 and **these two
/// pages stayed ambiguous**, which is worth knowing rather than assuming: the residual is 1 to 5
/// levels over a large share of each plate's samples, and whether it is a second defect or the
/// last place of two `f32` pipelines is not established. `jpeg2000.rs`'s own list is where that
/// question is written down, and it fails first if either end of it moves. ADR 0161, ADR 0190.
///
/// # And `issue5475.pdf` is second on `rank_the_manufactured_ambiguity`, which adds a reading
///
/// **31.63 bounds between the closest two references, and 0.00 between us and the nearest** —
/// the largest consensus failure in the bucket on a page where our own distance is zero. Mean
/// absolute difference over the 512 × 512 raster says what that is made of:
///
/// ```text
/// ours vs mupdf   0.0002      poppler vs mupdf        9.0293
/// ours vs hayro   0.3506      ghostscript vs mupdf   14.5610
/// ours vs poppler 9.0294      ghostscript vs poppler 19.0794
/// ```
///
/// Ours and `mupdf` are **two ten-thousandths of a level apart over 262 144 pixels** and the
/// three voting references span 9 to 19 among themselves.
///
/// **All three of them link the same `libopenjp2.so.7`** — `objdump -p`, the
/// five-hundred-and-eighteenth session, `Reference::independence` — so this is not three
/// decoders disagreeing. It is one decoder and three *callers* of it, differing in what they ask
/// for and what they do with the samples afterwards, and the result is that no consensus can
/// form at all. Trap 9's fifth shape without the shared code even having failed: shared code
/// manufactures the absence of a consensus here, and the thing that settles the page is
/// `tests/jpeg2000.rs`, which asks ISO/IEC 15444-5's own software and no renderer.
const AMBIGUOUS_IRREVERSIBLE_JPEG_2000: [&str; 2] = ["S2.pdf page 1", "issue5475.pdf page 1"];

/// Ambiguous, and the file has broken the one rule Table 73 states about `scn`.
///
/// `issue18894.pdf` is 612×792 with two commands, and the second is a 50×50 square. Five
/// renderers paint it five colours:
///
/// ```text
/// ours (75, 5, 50)   poppler (0, 0, 0)   mupdf (74, 75, 74)   ghostscript (50, 50, 50)
/// hayro (75, 75, 75)
/// ```
///
/// # What the clause determines
///
/// The content stream sets no colour space at all before painting. §8.6.8 makes the initial
/// non-stroking space `DeviceGray`, and Table 73 says of `SC` — which `sc` and `scn` inherit —
///
/// > The number of operands required and their interpretation depends on the current stroking
/// > colour space: For DeviceGray , CalGray , and Indexed colour spaces, one operand shall be
/// > required (n = 1).
///
/// The stream writes **three**: `0.294118 0.019608 0.196078 scn`. So the clause determines the
/// operand count exactly, the file has broken it, and the standard states nothing whatever about
/// what a processor should then paint. That is §3a's third shape, one level down from a clause
/// leaving something open: here the clause is closed and the *file* is outside it.
///
/// # Four recoveries, and each is legible in the pixels
///
/// | | reads | |
/// |---|---|---|
/// | ours | the three operands as an RGB triple | `(75, 5, 50)` |
/// | `mupdf`, `hayro` | the **first** operand as the grey | `0.294 × 255 = 75` |
/// | `ghostscript` | the **last** operand as the grey | `0.196 × 255 = 50` |
/// | `poppler` | nothing; the initial `DeviceGray` colour stands | black |
///
/// **Ours is the one derived from the file rather than from the operator.** Three numbers in the
/// order `0.294, 0.020, 0.196` is a dark maroon written by a producer that forgot its
/// `/DeviceRGB cs`, and `set_colour`'s rule — "where the operand count disagrees with the
/// declared space, the operands win" — recovers what the producer specified, which is what
/// `CLAUDE.md` asks of a renderer. It is a **documented choice** and not a derivation: the
/// clause states no recovery, and a session that wants a different one should argue with the
/// paragraph above rather than with a reference's pixels.
const AMBIGUOUS_COLOUR_OPERANDS: [&str; 1] = ["issue18894.pdf page 1"];

/// Ambiguous by a *third of one level of one channel*, and the closed form says which of five
/// renderers is on it.
///
/// `chrome-text-selection-markedContent.pdf` is a 960 × 540 financial page of dense small text
/// with a lavender panel down its right third. It sat at the head of §3a's undiagnosed ranking —
/// 0.98 from the nearest reference, 2.15 from the furthest — and the two-ladder measurement
/// (`doc/todo/00` step 6) shows the difference is not scan conversion at all:
///
/// ```text
///              72 dpi   288 dpi   576 dpi
/// poppler      27.1499  27.1878   27.2107
/// mupdf        27.0757  27.1903   27.2193
/// ours (1x, 4x, 8x)  26.9460  26.9286  26.9767
/// ```
///
/// Both reference ladders climb onto **27.21**; ours is *flat* at 26.95, which is what a
/// renderer whose coverage is proportional to area looks like — and 0.25 of 255 below them.
/// A flat ladder that does not move toward the others is the shape that says the difference is
/// in the *marks* rather than in the pixels.
///
/// # Where it is, and it is not spread over the page
///
/// A three-by-six grid of per-tile ink differences against `mupdf` at 8× puts the whole of it in
/// the two rightmost columns — −0.67 to −0.81 in all three rows, against ±0.04 everywhere else —
/// and a per-channel mean over that third names the channel: red differs by 0.06, blue by 0.03,
/// **green by 0.985**. One level of green, over a third of the page, is 0.23 of 255 of the page's
/// ink, which is the whole difference.
///
/// # The closed form, and it is §11.3.6's
///
/// The panel is one fill, and the file states every number in it: `0.851 0.847 1 rg` under an
/// `/ExtGState` whose `/ca` is `0.49804`, over white. §11.3.6 gives the result for a group whose
/// backdrop is opaque, and with the backdrop white it is one line per channel:
///
/// ```text
/// red    0.49804 × 0.851 + 0.50196 × 1 = 0.925790 → × 255 = 236.076 → 236
/// green  0.49804 × 0.847 + 0.50196 × 1 = 0.923800 → × 255 = 235.569 → 236
/// ```
///
/// | | panel | the page's other colour, `0.247 0.243 1 rg` at `ca 1` |
/// |---|---|---|
/// | exact | (236.08, 235.57, 255) | (62.99, 61.97, 255) |
/// | ours | **(236, 236, 255)** | **(63, 62, 255)** |
/// | `poppler` | (236, 235, 255) | (63, 62, 255) |
/// | `mupdf` | (236, 235, 255) | (62, 61, 255) |
///
/// **Ours is the arithmetic, on both colours and every channel.** The references lose the 0.57
/// of a level that a composite in eight-bit premultiplied storage cannot carry, and `mupdf` loses
/// another by truncating rather than rounding a plain fill. Nothing here is a defect of ours, and
/// the reason to say so at this length is that the *sign* invites the opposite conclusion: we
/// draw less ink than everybody, which is the shape `doc/todo/00`'s step 7 exists to catch, and
/// on this page it is one channel rounding the right way.
///
/// This is §3a's first shape — the clause determines it and we can be checked against it — and
/// the check needs no reference at all: the file's own numbers give 235.569, and 235.569 is 236.
/// # A second page, and it is one document giving two different answers
///
/// `issue840.pdf` page 1 came off §3a's ranking in the three-hundred-and-twenty-third session at
/// 0.44 from the nearest reference and 1.35 from the furthest. It is the *Download Festival 2009*
/// timetable: 4 328 commands of flat coloured blocks with small white text on them, and 177 of
/// 255 of the page is ink.
///
/// **Three ladders, all parallel and none converging**, which is what says this is not an edge:
///
/// ```text
///              72 dpi   288 dpi   576 dpi   1152 dpi
/// mupdf       176.783   177.526   177.636    177.716
/// poppler     176.174   176.989   177.056    177.117
/// ours        176.230   176.738   176.845    176.901
/// ```
///
/// `mupdf` is 0.60 of 255 above `poppler` at every rung and ours 0.22 below it at every rung. A
/// difference that does not shrink with the pixels is a colour, and `doc/todo/00`'s step for it
/// is the histogram of a page whose colours are few. At 1152 dpi the three dominant fills are:
///
/// ```text
///                   ours          poppler       mupdf
/// the page ground   (32,32,32)    (32,32,32)    (31,31,31)
/// the grid          (60,60,60)    (60,60,60)    (59,59,59)
/// a block under /ca (117,51,53)   (116,50,52)   (116,50,52)
/// ```
///
/// **And the file states the first two outright**, so this is §3a's first shape and needs no
/// reference at all: the content stream fills `0.125 0.125 0.125 rg` and `0.235 0.235 0.235 rg`,
/// which are **31.875** and **59.925** of 255. Ours and `poppler` round them to 32 and 60;
/// `mupdf` truncates to 31 and 59, one level darker on nearly every pixel of a page that is
/// nearly all flat fill — which is the whole of its 0.60. The third row is the group's own
/// subject one step along: 127 of the page's blocks are drawn under an `/ExtGState` with
/// `/ca 0.95` and twelve more under `/ca 0.35`, and the level ours keeps is the one an eight-bit
/// premultiplied composite cannot.
///
/// **Its page 2 is *not* this group**, and that is the finding worth carrying: `doc/todo/00` says
/// to check what else on the list is the same file, and here the same file gives two different
/// answers. Page 2 is a light page of text whose two ladders converge to **0.0002 of 255** of
/// each other — the tightest limit this bucket has measured — with ours 0.005 under it, which is
/// `AMBIGUOUS_GLYPH_SCAN_CONVERSION`. Check what else is the same file; do not assume it is the
/// same answer.
const AMBIGUOUS_EIGHT_BIT_COMPOSITING: [&str; 2] = [
    "chrome-text-selection-markedContent.pdf page 1",
    "issue840.pdf page 1",
];

/// Ambiguous, and it is a page of nothing but widget borders at their own geometry.
///
/// `bug1863910.pdf` is 353×59 with two commands: two text fields whose appearance streams each
/// state `0.5 0.5 149 21 re s` inside a `/BBox [0 0 150 22]` — a one-point stroke whose outer
/// edge lies **exactly** on the box that clips it.
///
/// It sat at 3.03 bounds from the nearest reference and 11.41 from the furthest, and the finding
/// is ADR 0165: the `/BBox` clip was multiplying its anti-aliased coverage into every stroke that
/// touched it, so the page carried **22% less ink than its geometry states**. Taking a clip that
/// cuts nothing back off — ADR 0155's rule, one path over — closes it.
///
/// # What is left, and the closed form says it is not us
///
/// Step 6's limit: `poppler` at 72, 576 and 2304 dpi gives 8.16, 8.29, **8.30**, and 8.30 is also
/// the arithmetic — two rectangles' perimeters at one point wide over a 353×59 page. Ours is
/// **8.299**. `poppler` 8.16, `mupdf` 8.20, `hayro` 6.46, `ghostscript` 10.60.
///
/// So we are on the geometry to three figures, `poppler` and `mupdf` are 1–2% under it,
/// `ghostscript` 28% over — §10.7.4's "any pixel whose half-open square region intersects the
/// shape" applied to a stroke a pixel wide — and `hayro` is 22% under, which is where we were.
/// The verdict is `ambiguous` at 0.79 because four renderers draw four one-pixel borders in four
/// slightly different places on a page that is 59 rows tall.
/// # `textfields.pdf` is the same page at letter size, and the fingerprint is the same five numbers
///
/// Off §3a's ranking in the two-hundred-and-eighty-sixth session at 0.69 from the nearest
/// reference and 2.97 from the furthest. Six empty fields on a 612 × 792 page — a one-point
/// border apiece and two comb rows of eleven cells — and nothing else at all, so like the page
/// above its ink *is* its line work.
///
/// ```text
///                 72 dpi   288 dpi   576 dpi
/// poppler        2.80579   2.85672   2.86581
/// mupdf          2.87486   2.86953   2.86834
/// ours (1x/4x/8x) 2.6789   2.85402   2.85492
/// ```
///
/// The two ladders agree to **0.0025 of 255** and ours climbs onto 2.8549, 0.4% under them —
/// so all three are drawing the same rectangles. At the page's own scale the five renderers
/// spread by 1.33 of 255: `ghostscript` **27% over** the geometry and `hayro` **19% under**,
/// which are the same two outliers in the same two directions as `bug1863910.pdf`'s 28% and 22%.
/// Ours is 6.5% under at 72 dpi and on the geometry by 288, which is a one-pixel stroke's
/// coverage arriving as the pixels shrink.
/// # A field with nothing in it at all, in the three-hundred-and-sixth
///
/// `multiline.pdf` page 1 is **one empty multiline text field's border and nothing else** — a
/// rectangle on a blank page, which makes its mean its border's coverage exactly.
///
/// ```text
///                 72 dpi    576 dpi
/// poppler        2.48928   2.49989
/// mupdf          2.50244   2.50086
/// ours (1x, 8x)  2.22286   2.46552
/// ```
///
/// The two ladders agree to **0.001 of 255** and ours climbs onto the limit from below, ending
/// 0.035 under. `ghostscript` is 3.17 at the page's own scale — 27% over the geometry — and
/// `hayro` 2.15, 14% under: **the same two outliers in the same two directions** as
/// `bug1863910.pdf`'s 28% and 22% and `textfields.pdf`'s, which is why this is that page's group
/// rather than a new one.
/// # A fourth, and its rows say which mark the difference is
///
/// `issue19083.pdf` page 1 came off §3a's ranking in the three-hundred-and-twenty-fifth session at
/// 0.43 from the nearest reference and 1.67 from the furthest. It is **149 × 68 device pixels**:
/// one §12.7.5.4 choice field with `/DA (/Helv 0 Tf 0 g)` — auto-sized — reading *Hello World*
/// inside a one-unit border, twelve commands in all.
///
/// ```text
///          72 dpi   144   216   288   432   576 dpi
/// poppler  11.3319  11.3687  11.4359  11.4715  11.4743  11.4912
/// ours     9.8814   10.3898  10.8367  11.3052  —        11.3176   (1x, 2x, 3x, 4x, 8x)
/// ```
///
/// `poppler` is at its own limit from 72 dpi; ours climbs 1.44 of 255 to reach the same place,
/// and the two limits are 0.17 apart. **The auto-size is not the difference**: thresholded at 8×,
/// the ink's bounding box is 126 × 21 at (15, 26) in *both*, to the pixel — so §12.7.4.3's
/// computed size, the face's metrics and the placement all agree, and what is left is scan
/// conversion.
///
/// **And the page is small enough to read row by row, which says which mark it is.** The mean per
/// raster row, ours against `poppler`, over the rows that carry anything:
///
/// ```text
///  y     ours   poppler          y     ours   poppler
/// 26   120.03    213.93         34    38.23     47.12
/// 27    56.05      3.42         …
/// 45   161.09    213.93         46    13.43      0.00
/// ```
///
/// The field's horizontal borders are rows 26 and 45. `poppler` puts **213.93** into one row of
/// each; ours spreads **176.08** and **174.52** across two, which is a one-unit line at a
/// fractional device position drawn as its own area. Those two marks are **77.3 of the 99.2** of
/// row-mean separating the whole page — 78% of it — and the rest is the vertical borders and the
/// glyph edges, where the text rows differ by ±5 in both directions.
///
/// So this is §10.7.4 twice over: "paint any pixel whose half-open square region intersects the
/// shape, no matter how small the intersection is" is what puts 214 where the geometry has 176,
/// and ADR 0025's departure is what puts ours on the area. `poppler` is 1.5% over its *own* limit
/// at 576 dpi, which is the same sentence at the other end of the ladder.
/// # A fifth, and the tightest pair of ladders this group has seen
///
/// `160F-2019.pdf` page 1 — the Italian tax form the window test presses Tab on — came off §3a's
/// ranking in the three-hundred-and-thirty-first session at 0.45 from the nearest reference and
/// 2.06 from the furthest. 2 474 commands: Arial subsets, one 102 × 72 image, and several hundred
/// field borders.
///
/// ```text
///                 72 dpi     576 dpi
/// mupdf          14.8713   14.87940
/// poppler        17.5573   14.87860
/// ours (1x/8x)   14.8597   14.86320
/// ```
///
/// **The two ladders end 0.0008 of 255 apart** — the tightest pair this group has measured — and
/// ours is 0.016 under them. What the first column shows is this group's whole subject at a scale
/// nothing else in the corpus reaches: `poppler` at the page's own scale is **2.70 of 255 over
/// its own limit**, 18% of the page's ink, on a page whose marks are mostly one-unit rules. Ours
/// is 0.003 from its own limit at 72 dpi.
/// # A sixth, whose geometry is arithmetic rather than a ladder's limit
///
/// `bug1889122.pdf` page 1 came off `doc/todo/00`'s ranking in the three-hundred-and-seventy-second
/// session at 0.13 from the nearest reference and 2.94 from the furthest. It is **one command** on
/// a 231 × 85 crop box: one text field, `/Rect [66.7639 663.309 216.764 685.309]`, whose stored
/// `/AP /N` has `/BBox [0.0 0.0 150.0 22.0]` and an identity `/Matrix`, so §12.5.5's map onto the
/// rectangle is the identity and the whole of what is drawn is the stream's four operators:
/// `q 0 G 0.5 0.5 149 21 re s Q`.
///
/// **That is a page whose ink can be written down.** A default-width stroke on that rectangle
/// covers the region between (0, 0)–(150, 22) and (1, 1)–(149, 21) — `150 × 22 − 148 × 20 = 340`
/// square points — in black on white paper, over a raster of `231 × 85 = 19 635` pixels. The ink
/// the geometry states is therefore `255 × 340 / 19 635 = 4.4156 of 255`, exactly, with no
/// reference and no limit involved.
///
/// ```text
///                     72 dpi     288      576     1152
/// geometry            4.4156   4.4286   4.4286   4.4328
/// poppler             4.3159   4.4286   4.4286   4.4331
/// mupdf               4.4329   4.4324   4.4308   4.4342
/// ours (1x/4x/8x/16x) 4.4177   4.4286   4.4286   4.4320
/// ```
///
/// (The geometry moves along the row because a raster of 231 × 85 holds a 230.94 × 84.63 crop box
/// with a fraction of a row spare; the last column is against `poppler`'s and `mupdf`'s
/// 3 695 × 1 355, ours against 3 696 × 1 355, whose own figure is 4.4323.)
///
/// **At the page's own scale ours is 0.05% over the closed form** and `mupdf` 0.39% over, while
/// `poppler` is 2.3% under and `ghostscript` — 5.5965 — is **26.7% over**. `hayro` rasterises this
/// crop box as 230 × 84, and against *that* raster's geometry of 4.4876 its 3.7258 is **17%
/// under**.
///
/// So: **the same two outliers in the same two directions** as `bug1863910.pdf`'s 28% over and 22%
/// under, `textfields.pdf`'s 27% and 19% and `issue19083.pdf`'s one raster row against our two —
/// the sixth time, and the first time measured against a number the file states rather than
/// against a limit two references had to agree on first.
const AMBIGUOUS_WIDGET_BORDER: [&str; 6] = [
    "bug1889122.pdf page 1",
    "160F-2019.pdf page 1",
    "issue19083.pdf page 1",
    "multiline.pdf page 1",
    "bug1863910.pdf page 1",
    "textfields.pdf page 1",
];

/// Ambiguous, and it is the one construction in this file that no gradient implementation has.
///
/// `radial_gradients.pdf` pages 4 and 5 are a test sheet: twenty-four §8.7.4.5.4 shadings in a
/// grid, four `/Extend` combinations across each of six geometries. They sit at 2.70 and 2.74
/// bounds from the nearest reference and **within 0.01 of the furthest**, which is the
/// everybody-against-us shape §3a says to prefer, and the pictures say it plainly: on the cone
/// cells the four references draw a filled disc with a cone on it and we draw only the crescent
/// between the two.
///
/// # What §8.7.4.5.4 determines, exactly
///
/// > Therefore, if a point lies on more than one blend circle, its final colour shall be that of
/// > the last of the enclosing circles to be painted, corresponding to the greatest value of s .
///
/// For `/Coords [511 489 25 431 489 60] /Extend [true false]`, `c(s) = (511 − 80s, 489)` and
/// `r(s) = 25 + 35s`. At the ending circle's own centre `(431, 489)` the blend circles through it
/// are `s = 0.478` and `s = 2.333`; the greater is outside `/Extend`, so the clause paints the
/// point at **t = 0.478** and we paint nothing.
///
/// # Fixed in the two-hundred-and-thirty-second session, and the group stays
///
/// `render-cpu` handed the shading to `tiny_skia::RadialGradient` — the SVG two-point conical
/// gradient — which solves for one root and clamps it with its spread mode, where the clause
/// says take the greatest *admissible* root. The other two backends built their own gradients
/// from the same `ShadingKind::Radial`, so all three inherited the same decision.
/// `pdf_render::blend_parameter` solves the quadratic and `pdf_render::RadialRaster` evaluates
/// it per device pixel; all three backends now draw those same bytes through the shape, exactly
/// as they already did for §8.7.4.5.5's mesh (ADR 0171).
///
/// ```text
///                    ink        worst tile   worst mean
/// page 4  before    66.05         108.49         8.62
/// page 4  after     70.62          15.78         4.42
/// page 5  before    67.19         109.58         8.67
/// page 5  after     71.42          15.78         4.52
/// ```
///
/// against 70.55 to 73.94 for the four references, so the 7% shortfall was the missing discs
/// and it is gone. **The group stays because the pages are still `ambiguous`**, and now for the
/// reason the bucket is *for*: twenty-four shadings on one page put twenty-four antialiased
/// boundaries and twenty-four ramps in front of five rasterisers, and no two of them agree
/// closely enough for anybody to be called wrong. What is left is §10.7.4's own subject and not
/// this clause's — including one departure this tree makes on purpose, because the raster is
/// point-sampled, so the *shading*'s boundary is hard where the *shape*'s is antialiased.
/// `MeshRaster` made that trade one shading type over and for the same reason.
///
/// **This group said "we are wrong" for twenty-six sessions**, which §3a allows and
/// `AMBIGUOUS_ZERO_AREA_FILL` did for two before its own fix. Both of them being right about
/// that is the argument for letting a group say it.
const AMBIGUOUS_RADIAL_CONE: [&str; 2] =
    ["radial_gradients.pdf page 4", "radial_gradients.pdf page 5"];

/// Ambiguous, and it is `CONTRADICTED_LINK_BORDER`'s subject with `poppler` on our side.
///
/// `bug766086.pdf` is 200×50 with one link over one line of text. Its annotation is four
/// entries — `/Border [0 0 1]`, `/C [1 0 0]`, `/Rect [5 10 190 40]` and a `/GoToR` — so Table 166
/// makes it a red one-unit border and §12.5.4 puts it "completely inside the annotation
/// rectangle". Ours and `poppler`'s draw it; `mupdf`, `ghostscript` and `hayro` draw the text
/// alone.
///
/// Ink: ours **20.61**, `poppler` **20.73**, against `mupdf` 12.16, `ghostscript` 12.34 and
/// `hayro` 11.99 — the border is 40% of the page's ink, which is why a 200×50 page with one line
/// of text reaches 2.58 bounds.
///
/// # "Ours and `poppler`'s draw it" agreed about the ink and hid a pixel, which is trap 12's rule
///
/// Those two sentences were written from an ink table, and the ink table is right: the two borders
/// weigh the same to 0.12 of 255. **They are not in the same place**, and the measure this page's
/// 2.58 is taken on is the structural similarity, not the mean — so the note priced a metric the
/// page passes (`doc/traps/oracle-and-references.md` trap 12's second paragraph, one group over).
///
/// The page is 200 × 50 at 72 dpi, so a user-space unit is a device pixel and `/Rect [5 10 190 40]`
/// is device columns 5 to 190 and device rows 10 to 40. A one-unit border completely inside it
/// occupies columns 5 and 189 and rows 10 and 39. Reading the red pixels off the two rasters —
/// device row 25, then device column 100:
///
/// ```text
///           left  right      top  bottom
/// ours         5    189       10      39
/// poppler      5    190       10      40
/// ```
///
/// **Ours is the clause and `poppler` is one pixel outside it on two of the four sides**: its
/// column 190 covers x ∈ [190, 191] and its row 40 covers user-space y ∈ [9, 10], both beyond the
/// rectangle the border is required to be inside. It is `AMBIGUOUS_OVERSIZED_BORDER`'s finding at a
/// width of 1 instead of 112 — the same renderer centring the stroke on the rectangle's edge — and
/// at that width the ink cannot see it while the similarity can.
///
/// # And it is a reference's departure rather than a page's, which is a population's answer
///
/// Re-measured in the seven-hundred-and-fifty-sixth session, which reproduces every column and row
/// above and then asks the question a single page cannot answer. §12.5.4 states no width-1 case:
/// the only sentence naming 1 is Table 168's default `/W`, which says how wide a border is and
/// nothing about where it goes. A stroke straddles its path, so a border whose ink is entirely
/// inside `/Rect` has its path inset by half its width, at every width.
///
/// The mechanism is unreadable at one pixel and plain at ten. A link with `/Border [0 0 10]` on
/// `/Rect [20 20 120 80]`, one device pixel per unit: **ours covers device 20…119 by 20…79, and
/// `poppler` covers 15…124 by 15…84** — five units beyond `/Rect` on all four sides, which is half
/// the width exactly. At width 1 `poppler` snaps a thin line to the pixel grid, so which sides show
/// it depends on where the rectangle's edges fall: two of four here, and **none** on
/// `issue12750.pdf`, whose `/Rect` is `[178.019 654.247 265.051 668.194]` and whose border lands on
/// the same columns as ours. That is rounding on top of the placement, not a second placement.
///
/// `crates/pdf-model/examples/border_overhang_census.rs` is what says so over a population rather
/// than a witness — both renders of every page whose annotation states a border this tree strokes
/// and no `/AP`, asking how far outside `/Rect` ink of the border's *stated* colour reaches. Its
/// counts are its own to print (ADR 0281); the shape of them is that on the comparisons whose
/// border is in a colour of the page's own, `poppler` reaches further outside than this tree on
/// three quarters and this tree reaches further on **none**. ADR 0675.
///
/// # And the ratio that ranks this page measures the same annotation twice
///
/// It is the head of `rank_the_pages_we_are_alone_on` at **5.68×**, ours 2.58 over 0.45 between
/// `mupdf` and `ghostscript`. Both halves are this annotation, which was measured by removing it:
/// `/Annots [4 0 R]` replaced by `/Annots []` in place, same byte length so the cross-reference
/// table still resolves, and all four renderers re-run.
///
/// **These are `examples/compare_rasters`' numbers and not this gate's**, which is why they carry
/// four decimals the gate does not print: the gate's line is our render against the *worst* member
/// of a consensus, and every row below is one named pair. `--bin quoted` reports the whole table
/// for that reason and is right to.
///
/// ```text
///                        with the annotation    without it
/// ours vs poppler        mean 7.5845 ssim 0.74205    mean 2.1275 ssim 0.98237
/// ours vs mupdf          mean 6.7478 ssim 0.60519    mean 1.3163 ssim 0.99105
/// mupdf vs ghostscript   mean 2.2695 ssim 0.98268    mean 2.2695 ssim 0.98268
/// ```
///
/// Our own number falls from 2.58 bounds to **0.43** — inside every one of them — while the pair
/// the ratio divides by is **byte-identical**, because neither of those two draws the annotation at
/// all. So the numerator is a clause we implement and the denominator is the same clause two
/// renderers do not, and the ranking reads the difference as ours. That is trap 9's tenth
/// mechanism with a shared *gap* in the divisor rather than a shared library (ADR 0663).
///
/// **The divisor is the *mean*, where the numerator is the similarity, and 5.68× therefore divides
/// one measure by another** (ADR 0688). `mupdf` against `ghostscript` is mean 2.2695 of a bound of
/// 5.00 and similarity 0.98268 of a bound of 0.9000, so 0.45 is the first of those and 0.17 the
/// second. Read like for like on the similarity the page is 2.58 over 0.17, which is **14.9×** and
/// the head of that list by a wide margin — so naming the measure makes this row *sharper* and
/// changes nothing about the reading, because the removal above already answered it: both halves
/// are one annotation, in either measure.
///
/// # Why three renderers drawing nothing is not three votes
///
/// `CONTRADICTED_LINK_BORDER` has the reading and every word of it applies here: `mupdf`'s
/// `pdf_write_appearance` switches over eighteen subtypes and `Link` is not among them, so it
/// constructs no appearance at all; `ghostscript` implements it and is being asked to *print*,
/// and Table 167's Print flag is clear because this annotation states no `/F` — "If clear, never
/// print the annotation, regardless of whether it is rendered on the screen." A viewer is a
/// screen, where bit 6 `NoView` is the flag with a say and it is clear.
///
/// What is different here is only which side the fourth renderer fell on: `poppler` draws the
/// border too, so there is no two-reference consensus and the verdict is `ambiguous` rather than
/// contradicted. **The page is the same page and the reading is the same reading** — which is
/// worth recording, because a group that exists only where the vote happened to go 2–2 would be
/// a group about the vote rather than about the clause.
/// **`issue18030.pdf` page 1 is the second, and it adds the sub-pixel half of the question.**
/// Four links over four words on a 48×71 page, each `/C [0 0 .8]` with a `/Border` and `/F 4`.
/// Ours, `poppler` and `ghostscript` draw the blue boxes; `mupdf` and `hayro` draw none, for
/// exactly the reasons above.
///
/// The closed form separates the two questions. `mupdf` and `hayro`, with no border at all, sit
/// at **21.6**. Ours is **31.66** at the page's own scale and **32.82** at eight times it — the
/// same picture, so our border is a real width that scales. `poppler` is 39.28 at 72 dpi and
/// **31.39** at 2304, which is a quarter of its ink at the page's own scale coming from painting
/// a line a fraction of a pixel wide solid (§10.7.4 as written).
///
/// So the page carries both of this file's standing arguments at once: whether a link has a
/// border, and how heavy a sub-pixel one is. Neither is ours to be wrong about.
/// **`bug886717.pdf` page 1 is the third, and it asks a different question with the same picture.**
/// A thesis's table of contents, 595 × 842, with 45 links over its entries. Two renderers draw a
/// red box round every one:
///
/// ```text
/// ours 6.6024 │ mupdf 6.5896 │ hayro 6.0302 │ poppler 10.0075 │ ghostscript 10.9182
/// ```
///
/// Ours and `mupdf` agree to **0.013 of 255** and the two that draw boxes are 3.4 and 4.3 above.
/// But the file is not the other two's file: every one of these links states
///
/// ```text
/// /Border [0 0 1]   /C [1 0 0]   /F 4   /AP << /N 9 0 R >>
/// ```
///
/// and **object 9 is a form XObject with a `/BBox` and an empty content stream**. `Length 10` of
/// `FlateDecode` that decodes to `""`. So the producer did not omit an appearance: it wrote one,
/// and what it says is *draw nothing*.
///
/// That makes this page's question narrower than the other two's. It is not whether a link has a
/// border — §12.5.5 settles it: "[a]n annotation may define as many as three separate
/// appearances", the normal one "shall be used when the annotation is not interacting with the
/// user", and §12.5.2's obligation is to draw it. A stated appearance outranks the entries a
/// border would be *built* from, and Table 166's `/Border` and `/C` are exactly
/// those entries. Ours, `mupdf` and `hayro` draw the empty stream; `poppler` and `ghostscript`
/// synthesise a border anyway.
///
/// **The panels are not all the same size**, and it is worth saying so where somebody will
/// measure them: 595 × 842 for ours and `ghostscript`, 596 × 842 for `poppler` and `mupdf`,
/// 595 × 841 for `hayro`. `doc/todo/00`'s step 6 rule about `magick identify` applies to the
/// oracle's own artefacts too — the numbers above are means, which survive a column of difference,
/// and a pairwise metric would not.
const AMBIGUOUS_LINK_BORDER: [&str; 3] = [
    "bug766086.pdf page 1",
    "issue18030.pdf page 1",
    "bug886717.pdf page 1",
];

/// Ambiguous, and §12.5.6.10 states the region and not one number about the marks in it.
///
/// `bug1538111.pdf` has **no content stream at all**: a 595×842 page whose whole ink is four
/// text markup annotations — a `Highlight`, an `Underline`, and the other two of Table 182's four
/// — each with a `/QuadPoints` array of six quadrilaterals and a `/C`.
///
/// Ink: `hayro` **0**, ours **1.48**, `mupdf` 1.69, `ghostscript` 2.00, `poppler` 2.40. Nobody
/// agrees with anybody, and one of the five draws nothing at all.
///
/// # What the clause determines, and where it stops
///
/// §12.5.6.10 is two sentences and a table. It says these annotations "shall appear as
/// highlights, underlines, strikeouts (all PDF 1.3), or jagged (\"squiggly\") underlines ( PDF
/// 1.4 ) in the text of a document", and Table 182 gives `/Subtype` and `/QuadPoints`, whose
/// quadrilaterals "encompass a word or group of contiguous words". **That is the whole of it.**
/// No line width, no position of an underline within its quadrilateral, no squiggle amplitude,
/// no rule about what a strikeout crosses.
///
/// So the region is determined exactly and the artwork is not determined at all — the same shape
/// as §12.5.6.4's icons, where "the clause requires one and draws none" (ADR 0109). Ours is a
/// **documented choice** in `appearance.rs`: a bar one sixteenth of the quadrilateral's height,
/// on the bottom edge for an underline and at the middle for a strikeout, a squiggle of
/// amplitude one twelfth, and a highlight filled under `Multiply` — which is not decoration
/// either, since §11.3.5.2 defines exactly one blend mode whose "result colour is always at
/// least as dark as either of the two constituent colours".
///
/// A renderer drawing a heavier underline is not reading the clause differently; there is
/// nothing there to read differently. Five numbers between 0 and 2.40 is what a clause that
/// states no artwork produces, and the honest thing is to say so rather than move toward the
/// middle of them.
const AMBIGUOUS_MARKUP_ARTWORK: [&str; 1] = ["bug1538111.pdf page 1"];

/// Ambiguous, and §12.5.6.4 requires an icon and draws none — the sentence ADR 0109 is about.
///
/// `rc_annotation.pdf` is a 100 × 100 page whose whole content is one text annotation:
/// `/Subtype /Text /Name /Note /Rect [50 50 50 50]`. A degenerate rectangle, and the clause makes
/// that not a refusal:
///
/// > A text annotation represents a "sticky note" attached to a point in the PDF document. When
/// > closed, the annotation shall appear as an icon
///
/// **Attached to a point**, and a `shall` — so the size comes from the next sentence, which makes
/// a text annotation behave "as if the NoZoom and NoRotate annotation flags … were always set",
/// and §12.5.3's `NoZoom` is a fixed size on the screen. `annotation::anchored_icon` gives it
/// twenty units hanging from §12.5.3's own upper-left corner.
///
/// # This page's number got worse and the page got right, which is trap 1 in reverse
///
/// This tree drew **nothing** here — zero commands, `is_complete`, and 0.73 from the nearest
/// reference, because a nearly blank page resembles a nearly blank page. `doc/todo/00`'s step 7
/// sweep is what saw it: ours minus the lightest reference's ink, **−1.783 of 255**, past the
/// −1 the file names as the alarm. `ghostscript` and `hayro` draw nothing either; `poppler` draws
/// 1.78 and `mupdf` 2.35.
///
/// With the icon drawn, ours is 2.38 — beside `mupdf` — and the page moved *up* the ranking to
/// 4.93, because three renderers now draw three different pictures in the same place. That is
/// §12.5.6.4's silence and not a defect: the clause requires "predefined icon appearances for at
/// least the following standard names" and states not one line of their artwork, which is why
/// `icon.rs` is the one module in the tree that is pure invention and says so. Ours is a page
/// glyph with a folded corner, `poppler`'s a pinned note, `mupdf`'s a lined box.
const AMBIGUOUS_ICON_ARTWORK: [&str; 1] = ["rc_annotation.pdf page 1"];

/// Ambiguous, and the page has no fonts at all — the words are paths.
///
/// `issue12213.pdf` is 449×72 and reads *blue [shield] of california*. `pdffonts` lists **no
/// font**, because the producer converted the type to outlines, which is what makes this page
/// worth its own entry: `Interpretation::glyphs` sees no glyphs, so the oracle gives it the
/// *vector* tolerance rather than the text one, and a page of letter-shaped curves is then held
/// to 0.99 structural similarity. It sat at 1.60 from the nearest reference for that reason
/// alone.
///
/// Step 6's closed form settles it outright. `poppler` at 72, 576 and 2304 dpi gives 9.567,
/// 8.921 and **8.862**, so the marks cover 8.86 of 255, and at the page's own scale:
///
/// ```text
/// ours 8.898   ghostscript 8.945   hayro 8.736   poppler 9.567   mupdf 7.384
///              └ the limit is 8.862
/// ```
///
/// **Ours is 0.04 from the geometry and nearest of the five.** `poppler` is 0.71 over and
/// converges down to the limit as the pixels shrink, which is scan conversion of curves at 72
/// dpi; `mupdf` is 1.48 under. §10.7.4 states the rule these five are spread around — "there
/// shall not be averaging over the pixel area" — and this tree's departure from it is ADR 0025's,
/// argued and measured. What no clause states is how a curve's edge lands on a pixel grid.
const AMBIGUOUS_OUTLINED_TEXT: [&str; 1] = ["issue12213.pdf page 1"];

/// Ambiguous, and the instrument the method file recommends is the thing that fails here.
///
/// `issue2177.pdf` is 225×225: a yellow rectangle and three circles filled with a §8.7.3 tiling
/// pattern of stroked ellipses. The pattern's content stream states **no `w` at all**, so the
/// width is Table 57's initial 1.0 in pattern space, and `/Matrix [1.4 1 -.5 .7 0 0]` carries it
/// to the page. That matrix's own arithmetic gives the width exactly: `MᵀM` is diagonal —
/// `1.4² + 1² = 2.96` and `0.5² + 0.7² = 0.74`, the off-diagonal `1.4×(−0.5) + 1×0.7` being zero
/// — so the singular values are **1.72 and 0.86 device pixels**, and every ellipse is stroked
/// between those two widths depending on its direction. Half of that is under a pixel, which is
/// why the page is here at all: at its own scale the five renderers span
///
/// ```text
/// poppler 34.15   ours 36.75   hayro 37.12   mupdf 37.84   ghostscript 47.48
/// ```
///
/// # Step 6 says take the reference to a high resolution, and on this page that is wrong
///
/// `poppler` at 576 and 2304 dpi gives **18.03 and 16.32** — less than half its own answer at
/// 72 — which would say every renderer here paints two to three times the geometry. It is
/// `poppler` that is moving: opened side by side at 8×, its ellipse outlines are a fraction of
/// the width the matrix above states, while ours are the stated width.
///
/// The ladder that settles it is two renderers' rather than one, which is what this page adds to
/// the method:
///
/// ```text
/// ours at 1×, 2×, 4×, 8×:   36.75   37.37   37.25   37.20
/// mupdf at 8×:                                      37.20
/// ghostscript at 8×:                                40.86
/// poppler at 8×:                                    18.03
/// ```
///
/// **Ours and `mupdf` agree to four significant figures at 8×**, having differed by 1.09 at the
/// page's own scale, and our own ink is flat across four scales — a renderer measuring area.
/// `ghostscript` is 10% over and `poppler` is at half. So the *geometry* is 37.2, the page's
/// ambiguity at 1× is §10.7.4 applied to strokes 0.86 of a pixel wide, and step 6's assumption —
/// that a reference converges on the geometry as the pixels shrink — does not hold for a tiling
/// pattern. `doc/todo/00` carries the caveat.
const AMBIGUOUS_TILED_STROKES: [&str; 1] = ["issue2177.pdf page 1"];

/// Ambiguous, and the reason this file's pages are here is not the one that was written down.
///
/// `freeculture.pdf` is one of the two long books that make up half §3a's bucket, and this
/// project's handover explained both of them the same way for many sessions: "two long books set
/// in fonts nobody embedded, so each renderer substitutes differently". **`pdffonts` says
/// otherwise** — `P22Typewriter` twice, `ZapfDingbats` and `Monospace821BT-Roman`, and every one
/// of the four is *embedded*. There is no substitution on these pages at all.
///
/// What they are is dense text at book size, where `Interpretation::glyphs` earns the page the
/// **text** tolerance — 0.90 structural similarity, chosen over 153 reference-against-reference
/// pairs because the references disagree with each other at worst-tile 26 to 28 on text. A page
/// of nothing but small glyphs is a page where five rasterisers cannot agree closely enough for
/// any of them to be called wrong, and the bound is doing exactly what it was measured to do.
///
/// The one on the ranking, with the closed form:
///
/// ```text
///                  limit             ours     hayro    poppler   mupdf    ghostscript
/// page 333        12.515 / 12.549   12.435   12.141   12.434   12.437   12.529
/// ```
///
/// Ours is 0.10 under the geometry and the five renderers are inside 0.4 of 255 of one another.
/// **Nobody here is drawing anything anybody else is not**, which is the finding: these pages are
/// ambiguous by the tolerance's design and not by any defect, and the tail of the ranking below
/// 1.4 is mostly this book.
///
/// **That table had a second row, and it is the reason this note contradicted itself for four
/// hundred sessions.** `page 255  36.144 / 36.149  36.206  36.791  36.091  36.062  36.252` stood
/// here from the two-hundred-and-thirty-third session, and the paragraph about the band added
/// later said of the same page that *it has never been opened*. Both were in this comment at once.
/// The measurement is sound and reproduces — the seven-hundred-and-sixty-first session re-took it
/// and every figure lands to the thousandth — and it answers the wrong question: **the ink says
/// how much and is silent about where**, and page 255 is a 23-to-1 reduction of a bilevel stencil
/// whose whole disagreement is placement. It is `AMBIGUOUS_IMAGE_REDUCTION`'s now (ADR 0685). A
/// closed form that clears a page for the measure it can see is not a diagnosis of a page failing
/// the measures it cannot.
///
/// # Four more, and the whole head of the ranking with them
///
/// The two-hundred-and-thirty-third session took the next four names — the entire undiagnosed
/// ranking above 1.0 — and they are the same page four times over:
///
/// ```text
///                ours 1x   ours 8x   poppler 1x   poppler 8x   mupdf 8x
/// page 329        12.066    12.183     12.070       12.165      12.197
/// page 323        12.212    12.323     12.217       12.305      12.338
/// page 315        11.859    11.976     11.870       11.959      11.991
/// page 322        11.029    11.155     11.063       11.141      11.171
/// ```
///
/// The two ladders agree to **0.032 of 255** on the first and no worse than 0.03 on any of
/// them, so there is a limit; ours sits *between* the two references' limits on all four, and
/// at the page's own scale ours and `poppler` are **0.004** apart. A page cannot be much more
/// agreed about than that, and it is still `ambiguous`, because a fifth renderer somewhere in
/// the set differs by more than a bound measured over dense small glyphs. That is the
/// tolerance's design and not a defect, one more time and with four decimal places.
///
/// # Those same four pages are the book's whole head on a ranking built five hundred rounds later
///
/// [`rank_the_pages_we_are_alone_on`] orders the pages we sit further from every reference on than
/// the closest two references sit from each other, and pages **315, 322, 323, 329 and 333** — the
/// four above plus the one in the first table — are five of its top eight (ADR 0647). Two things
/// came out of asking why, and neither is a defect:
///
/// - **The ladder reproduces.** Re-taken on page 315 in the seven-hundred-and-forty-fourth
///   session: ours 11.8908 → 11.9540 → 11.9855 at 1×, 4× and 8×, `poppler` 11.8704 → 11.9478 →
///   11.9592, `mupdf` 11.9611 → 11.9979 → 11.9914. **Both references reproduce that table's three
///   decimals exactly**, 511 rounds later, and ours is 0.032 heavier at 1× and 0.010 at 8× — the
///   direction ADR 0418's round recorded for this population, where most of its moves were upward,
///   which is a candidate rather than an attribution. Ours still lies between the two limits at
///   every rung, and at 8× the three are inside 0.032 of 255.
/// - **What lifts them is the ranking's *denominator*.** Over the book's 321 compared pages,
///   `poppler` and `mupdf` — the two voting references that share `libfreetype.so.6`, where
///   `ghostscript` links its own copy — are the closest pair on **9 of the 11 pages that reach
///   that list** and on **7 of the other 310**, and their own median MAE is **724** over those 11
///   against **1760** over the rest. Trap
///   9 is a list of ways shared code manufactures an agreement between references; in a ratio that
///   agreement is the divisor, so here it accuses us instead of excusing somebody.
///
/// **And the two halves of that ratio are two different measures, which is the third thing and was
/// invisible until the gate printed it** (ADR 0688). On all five of these pages our own number is
/// the **structural similarity against `mupdf`** and the divisor is the **mean between `poppler`
/// and `mupdf`** — so 3.39× is not a ratio of like for like, and both bullets above are priced in
/// ink, which is neither of those two. `examples/compare_rasters` over the gate's own panels, that
/// example's figures and one named pair per row (ADR 0663), against text bounds of mean 5.00,
/// worst tile 40.00, similarity 0.9000:
///
/// ```text
/// page 315                  mean      ssim     bounds
/// ours vs poppler         5.4095   0.87334      1.27
/// ours vs mupdf           5.1514   0.88154      1.18  ← the numerator, on the similarity
/// ours vs ghostscript     6.5841   0.82608      1.74
/// poppler vs mupdf        1.7452   0.97776      0.35  ← the divisor, on the mean
/// poppler vs ghostscript  3.6882   0.93547      0.74
/// mupdf vs ghostscript    3.6854   0.93595      0.74
///
/// page 322
/// ours vs poppler         5.0380   0.88086      1.19
/// ours vs mupdf           4.7575   0.88979      1.10  ← the numerator, on the similarity
/// ours vs ghostscript     6.1033   0.83924      1.61
/// poppler vs mupdf        1.6720   0.97848      0.33  ← the divisor, on the mean
/// poppler vs ghostscript  3.6082   0.93487      0.72
/// mupdf vs ghostscript    3.4711   0.94018      0.69
/// ```
///
/// **Read like for like the page is further out, not nearer.** Taken on the similarity alone the
/// sharing pair is at 0.22 of the bound and we are at 1.18, which is 5.3× rather than the printed
/// 3.39×; taken on the mean alone we are at 1.03 over their 0.35, which is 3.0×. So naming the
/// measure does not excuse
/// these pages and was never going to: what it does is say **what is left to explain**, and it is
/// not the ink. The ladders above answer *how much* to four decimal places and the similarity is a
/// statement about *where* — which is `freeculture.pdf` page 255's lesson arriving on the five
/// pages that stayed. The clause that licenses the difference is §10.7.4's closing sentence, "[s]can
/// conversion of character glyphs may be performed by a different algorithm from the preceding
/// one", and [`AMBIGUOUS_GLYPH_COVERAGE`] is where the same pair of facts is measured on a page of
/// fifteen rows instead of six hundred.
///
/// # And then the whole of both books, on a sample and a band
///
/// The two-hundred-and-sixty-second session took the population rather than the page. Six pages
/// had been measured one at a time over three sessions and every one came out the same way, so
/// the question stopped being "what is wrong with page 329" and became "is this book one finding
/// or three hundred". **Twelve more pages, spread evenly through both books and none of them
/// previously measured**, with the two ladders and ours beside them:
///
/// ```text
/// freeculture      ours 1x   ours 8x   poppler 72   poppler 576   mupdf 576
/// page 20          11.9160   12.0676     11.9577      12.0616      12.0913
/// page 60          12.7387   12.8816     12.7416      12.8719      12.9034
/// page 100         13.1436   13.2864     13.1494      13.2767      13.3093
/// page 140         12.7389   12.8947     12.7452      12.8869      12.9183
/// page 180         11.3470   11.4491     11.3292      11.4457      11.4734
/// page 220          6.8226    6.9061      6.8338       6.9014       6.9179
/// page 260         12.8318   12.9916     12.8541      12.9802      13.0119
/// page 300         11.7759   11.6815     11.5614      11.6772      11.7062
///
/// pdkids
/// page 5           12.9361   13.0508     12.9894      13.0685      13.0601
/// page 20          13.0262   13.1003     13.0485      13.1252      13.1174
/// page 35          12.1869   12.2588     12.2124      12.2776      12.2701
/// page 50          12.2426   12.3033     12.2547      12.3237      12.3162
/// ```
///
/// **Ours at 8× is within 0.012 of `poppler`'s own limit on all twelve**, and lies between the
/// two references' limits on every `freeculture` page. The two ladders themselves are 0.03 apart.
/// Twelve pages measured this way, twelve times the same answer, on two books that between them
/// are three quarters of §3a's bucket.
///
/// # The band, over all 364, and the one page it caught
///
/// A sample is a sample, so the *whole* population's printed metrics were read as well. Re-derived
/// from the gate's own per-page lines over the list as it stands (ADR 0495, ADR 0685):
/// `freeculture`'s 317 sit at mean 2.43 to 8.99, worst tile 11.60 to 29.05, differing 5.61% to
/// 15.37% and similarity 0.7210 to 0.9648, and `pdkids`'s 52 at mean 5.80 to 11.67, worst tile
/// 28.27 to 39.45, similarity 0.8075 to 0.9033. One band, no gaps, and the text tolerance — 0.90
/// similarity, measured over 153 reference-against-reference pairs — running through the middle
/// of it.
///
/// **Two pages stood outside that band and neither was ours, and both were cartoons.**
/// `freeculture.pdf` page 171 has a worst tile of 81.57 where nothing else in the book exceeds
/// 29.09; its cartoon is a one-bit stencil, `ghostscript` thresholds it to a black blob where the
/// other four draw a grey halftone, and the page is `AMBIGUOUS_IMAGE_REDUCTION`'s subject rather
/// than this group's. That is the argument for reading the band before claiming the population:
/// **a diagnosis by sampling would have buried it**, and what found it was one number over three
/// hundred pages and then the picture.
///
/// **Page 255 is the second, and this note carried it for four hundred sessions while saying it
/// could not explain it.** It was the extreme member on three of the four measures at once — mean
/// 16.63 against a next-highest 8.99, worst tile 51.93 against 29.05, 19.98% differing against
/// 15.37% — and the sentence that stood here said *whatever it is, it is not the diagnosis these
/// paragraphs make*. It is the book's other full-page cartoon: one 9258 × 12259 `CCITTFaxDecode`
/// stencil reduced 23 to 1, with a running foot for text. It is in `AMBIGUOUS_IMAGE_REDUCTION`
/// now, beside page 171, with the four ladders that settle it (ADR 0685). **A disclaimer inside a
/// group note is a page nobody is holding**, and the honest form of it is a page in the group that
/// describes it.
const AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE: [&str; 369] = [
    "freeculture.pdf page 2",
    "freeculture.pdf page 7",
    "freeculture.pdf page 10",
    "freeculture.pdf page 12",
    "freeculture.pdf page 13",
    "freeculture.pdf page 14",
    "freeculture.pdf page 17",
    "freeculture.pdf page 18",
    "freeculture.pdf page 19",
    "freeculture.pdf page 20",
    "freeculture.pdf page 21",
    "freeculture.pdf page 22",
    "freeculture.pdf page 23",
    "freeculture.pdf page 24",
    "freeculture.pdf page 25",
    "freeculture.pdf page 26",
    "freeculture.pdf page 27",
    "freeculture.pdf page 28",
    "freeculture.pdf page 29",
    "freeculture.pdf page 32",
    "freeculture.pdf page 33",
    "freeculture.pdf page 36",
    "freeculture.pdf page 37",
    "freeculture.pdf page 38",
    "freeculture.pdf page 39",
    "freeculture.pdf page 40",
    "freeculture.pdf page 41",
    "freeculture.pdf page 42",
    "freeculture.pdf page 43",
    "freeculture.pdf page 45",
    "freeculture.pdf page 46",
    "freeculture.pdf page 47",
    "freeculture.pdf page 48",
    "freeculture.pdf page 49",
    "freeculture.pdf page 50",
    "freeculture.pdf page 51",
    "freeculture.pdf page 52",
    "freeculture.pdf page 53",
    "freeculture.pdf page 54",
    "freeculture.pdf page 55",
    "freeculture.pdf page 56",
    "freeculture.pdf page 57",
    "freeculture.pdf page 58",
    "freeculture.pdf page 59",
    "freeculture.pdf page 60",
    "freeculture.pdf page 61",
    "freeculture.pdf page 62",
    "freeculture.pdf page 63",
    "freeculture.pdf page 64",
    "freeculture.pdf page 65",
    "freeculture.pdf page 66",
    "freeculture.pdf page 68",
    "freeculture.pdf page 69",
    "freeculture.pdf page 70",
    "freeculture.pdf page 71",
    "freeculture.pdf page 72",
    "freeculture.pdf page 73",
    "freeculture.pdf page 74",
    "freeculture.pdf page 75",
    "freeculture.pdf page 77",
    "freeculture.pdf page 78",
    "freeculture.pdf page 79",
    "freeculture.pdf page 80",
    "freeculture.pdf page 81",
    "freeculture.pdf page 82",
    "freeculture.pdf page 83",
    "freeculture.pdf page 84",
    "freeculture.pdf page 85",
    "freeculture.pdf page 86",
    "freeculture.pdf page 87",
    "freeculture.pdf page 88",
    "freeculture.pdf page 89",
    "freeculture.pdf page 90",
    "freeculture.pdf page 91",
    "freeculture.pdf page 92",
    "freeculture.pdf page 93",
    "freeculture.pdf page 95",
    "freeculture.pdf page 96",
    "freeculture.pdf page 97",
    "freeculture.pdf page 98",
    "freeculture.pdf page 99",
    "freeculture.pdf page 100",
    "freeculture.pdf page 101",
    "freeculture.pdf page 102",
    "freeculture.pdf page 103",
    "freeculture.pdf page 104",
    "freeculture.pdf page 105",
    "freeculture.pdf page 106",
    "freeculture.pdf page 107",
    "freeculture.pdf page 108",
    "freeculture.pdf page 109",
    "freeculture.pdf page 110",
    "freeculture.pdf page 112",
    "freeculture.pdf page 113",
    "freeculture.pdf page 114",
    "freeculture.pdf page 115",
    "freeculture.pdf page 116",
    "freeculture.pdf page 117",
    "freeculture.pdf page 118",
    "freeculture.pdf page 119",
    "freeculture.pdf page 120",
    "freeculture.pdf page 121",
    "freeculture.pdf page 122",
    "freeculture.pdf page 123",
    "freeculture.pdf page 124",
    "freeculture.pdf page 125",
    "freeculture.pdf page 126",
    "freeculture.pdf page 128",
    "freeculture.pdf page 129",
    "freeculture.pdf page 130",
    "freeculture.pdf page 131",
    "freeculture.pdf page 132",
    "freeculture.pdf page 133",
    "freeculture.pdf page 134",
    "freeculture.pdf page 135",
    "freeculture.pdf page 136",
    "freeculture.pdf page 137",
    "freeculture.pdf page 138",
    "freeculture.pdf page 139",
    "freeculture.pdf page 140",
    "freeculture.pdf page 141",
    "freeculture.pdf page 142",
    "freeculture.pdf page 143",
    "freeculture.pdf page 145",
    "freeculture.pdf page 146",
    "freeculture.pdf page 147",
    "freeculture.pdf page 148",
    "freeculture.pdf page 149",
    "freeculture.pdf page 150",
    "freeculture.pdf page 151",
    "freeculture.pdf page 152",
    "freeculture.pdf page 153",
    "freeculture.pdf page 154",
    "freeculture.pdf page 155",
    "freeculture.pdf page 156",
    "freeculture.pdf page 157",
    "freeculture.pdf page 158",
    "freeculture.pdf page 159",
    "freeculture.pdf page 160",
    "freeculture.pdf page 161",
    "freeculture.pdf page 162",
    "freeculture.pdf page 163",
    "freeculture.pdf page 164",
    "freeculture.pdf page 165",
    "freeculture.pdf page 166",
    "freeculture.pdf page 167",
    "freeculture.pdf page 168",
    "freeculture.pdf page 169",
    "freeculture.pdf page 170",
    "freeculture.pdf page 172",
    "freeculture.pdf page 173",
    "freeculture.pdf page 174",
    "freeculture.pdf page 175",
    "freeculture.pdf page 176",
    "freeculture.pdf page 177",
    "freeculture.pdf page 178",
    "freeculture.pdf page 179",
    "freeculture.pdf page 180",
    "freeculture.pdf page 181",
    "freeculture.pdf page 182",
    "freeculture.pdf page 183",
    "freeculture.pdf page 184",
    "freeculture.pdf page 185",
    "freeculture.pdf page 187",
    "freeculture.pdf page 188",
    "freeculture.pdf page 189",
    "freeculture.pdf page 190",
    "freeculture.pdf page 191",
    "freeculture.pdf page 193",
    "freeculture.pdf page 194",
    "freeculture.pdf page 195",
    "freeculture.pdf page 196",
    "freeculture.pdf page 197",
    "freeculture.pdf page 198",
    "freeculture.pdf page 199",
    "freeculture.pdf page 200",
    "freeculture.pdf page 201",
    "freeculture.pdf page 202",
    "freeculture.pdf page 203",
    "freeculture.pdf page 204",
    "freeculture.pdf page 205",
    "freeculture.pdf page 206",
    "freeculture.pdf page 207",
    "freeculture.pdf page 208",
    "freeculture.pdf page 209",
    "freeculture.pdf page 210",
    "freeculture.pdf page 211",
    "freeculture.pdf page 212",
    "freeculture.pdf page 213",
    "freeculture.pdf page 214",
    "freeculture.pdf page 215",
    "freeculture.pdf page 216",
    "freeculture.pdf page 217",
    "freeculture.pdf page 220",
    "freeculture.pdf page 221",
    "freeculture.pdf page 222",
    "freeculture.pdf page 223",
    "freeculture.pdf page 224",
    "freeculture.pdf page 225",
    "freeculture.pdf page 226",
    "freeculture.pdf page 227",
    "freeculture.pdf page 228",
    "freeculture.pdf page 229",
    "freeculture.pdf page 230",
    "freeculture.pdf page 231",
    "freeculture.pdf page 232",
    "freeculture.pdf page 233",
    "freeculture.pdf page 234",
    "freeculture.pdf page 235",
    "freeculture.pdf page 236",
    "freeculture.pdf page 237",
    "freeculture.pdf page 238",
    "freeculture.pdf page 239",
    "freeculture.pdf page 240",
    "freeculture.pdf page 241",
    "freeculture.pdf page 242",
    "freeculture.pdf page 243",
    "freeculture.pdf page 244",
    "freeculture.pdf page 245",
    "freeculture.pdf page 246",
    "freeculture.pdf page 247",
    "freeculture.pdf page 248",
    "freeculture.pdf page 249",
    "freeculture.pdf page 250",
    "freeculture.pdf page 251",
    "freeculture.pdf page 252",
    "freeculture.pdf page 253",
    "freeculture.pdf page 254",
    "freeculture.pdf page 256",
    "freeculture.pdf page 257",
    "freeculture.pdf page 258",
    "freeculture.pdf page 259",
    "freeculture.pdf page 260",
    "freeculture.pdf page 261",
    "freeculture.pdf page 262",
    "freeculture.pdf page 263",
    "freeculture.pdf page 264",
    "freeculture.pdf page 265",
    "freeculture.pdf page 266",
    "freeculture.pdf page 267",
    "freeculture.pdf page 268",
    "freeculture.pdf page 269",
    "freeculture.pdf page 270",
    "freeculture.pdf page 271",
    "freeculture.pdf page 272",
    "freeculture.pdf page 273",
    "freeculture.pdf page 274",
    "freeculture.pdf page 275",
    "freeculture.pdf page 276",
    "freeculture.pdf page 277",
    "freeculture.pdf page 278",
    "freeculture.pdf page 281",
    "freeculture.pdf page 282",
    "freeculture.pdf page 283",
    "freeculture.pdf page 284",
    "freeculture.pdf page 285",
    "freeculture.pdf page 286",
    "freeculture.pdf page 287",
    "freeculture.pdf page 288",
    "freeculture.pdf page 289",
    "freeculture.pdf page 290",
    "freeculture.pdf page 291",
    "freeculture.pdf page 294",
    "freeculture.pdf page 295",
    "freeculture.pdf page 296",
    "freeculture.pdf page 297",
    "freeculture.pdf page 298",
    "freeculture.pdf page 299",
    "freeculture.pdf page 300",
    "freeculture.pdf page 301",
    "freeculture.pdf page 302",
    "freeculture.pdf page 303",
    "freeculture.pdf page 304",
    "freeculture.pdf page 305",
    "freeculture.pdf page 306",
    "freeculture.pdf page 307",
    "freeculture.pdf page 308",
    "freeculture.pdf page 309",
    "freeculture.pdf page 310",
    "freeculture.pdf page 311",
    "freeculture.pdf page 314",
    "freeculture.pdf page 315",
    "freeculture.pdf page 316",
    "freeculture.pdf page 317",
    "freeculture.pdf page 318",
    "freeculture.pdf page 319",
    "freeculture.pdf page 320",
    "freeculture.pdf page 321",
    "freeculture.pdf page 322",
    "freeculture.pdf page 323",
    "freeculture.pdf page 324",
    "freeculture.pdf page 325",
    "freeculture.pdf page 326",
    "freeculture.pdf page 327",
    "freeculture.pdf page 328",
    "freeculture.pdf page 329",
    "freeculture.pdf page 330",
    "freeculture.pdf page 331",
    "freeculture.pdf page 332",
    "freeculture.pdf page 333",
    "freeculture.pdf page 334",
    "freeculture.pdf page 335",
    "freeculture.pdf page 336",
    "freeculture.pdf page 337",
    "freeculture.pdf page 338",
    "freeculture.pdf page 340",
    "freeculture.pdf page 341",
    "freeculture.pdf page 342",
    "freeculture.pdf page 343",
    "freeculture.pdf page 344",
    "freeculture.pdf page 345",
    "freeculture.pdf page 346",
    "freeculture.pdf page 347",
    "freeculture.pdf page 348",
    "freeculture.pdf page 349",
    "freeculture.pdf page 350",
    "freeculture.pdf page 351",
    "pdkids.pdf page 1",
    "pdkids.pdf page 2",
    "pdkids.pdf page 3",
    "pdkids.pdf page 4",
    "pdkids.pdf page 5",
    "pdkids.pdf page 6",
    "pdkids.pdf page 7",
    "pdkids.pdf page 8",
    "pdkids.pdf page 9",
    "pdkids.pdf page 10",
    "pdkids.pdf page 11",
    "pdkids.pdf page 12",
    "pdkids.pdf page 14",
    "pdkids.pdf page 15",
    "pdkids.pdf page 16",
    "pdkids.pdf page 17",
    "pdkids.pdf page 18",
    "pdkids.pdf page 19",
    "pdkids.pdf page 20",
    "pdkids.pdf page 21",
    "pdkids.pdf page 22",
    "pdkids.pdf page 24",
    "pdkids.pdf page 25",
    "pdkids.pdf page 26",
    "pdkids.pdf page 27",
    "pdkids.pdf page 28",
    "pdkids.pdf page 29",
    "pdkids.pdf page 30",
    "pdkids.pdf page 31",
    "pdkids.pdf page 32",
    "pdkids.pdf page 33",
    "pdkids.pdf page 34",
    "pdkids.pdf page 35",
    "pdkids.pdf page 36",
    "pdkids.pdf page 37",
    "pdkids.pdf page 38",
    "pdkids.pdf page 39",
    "pdkids.pdf page 40",
    "pdkids.pdf page 41",
    "pdkids.pdf page 42",
    "pdkids.pdf page 43",
    "pdkids.pdf page 44",
    "pdkids.pdf page 45",
    "pdkids.pdf page 46",
    "pdkids.pdf page 47",
    "pdkids.pdf page 48",
    "pdkids.pdf page 49",
    "pdkids.pdf page 50",
    "pdkids.pdf page 51",
    "pdkids.pdf page 52",
    "pdkids.pdf page 53",
    "pdkids.pdf page 54",
];

/// Ambiguous because one reference is alone by eighteen levels, and it is not this one.
///
/// `issue17065.pdf` is one axial shading — `/ShadingType 2`, `/Extend [true true]` — painted
/// through a pattern into an arrow-shaped clip, and its colour space is the interesting part:
///
/// ```text
/// /ColorSpace [/DeviceN [/L /A /B] [/CalRGB << /WhitePoint [0.9505 1 1.0888] /Gamma [1 1 1]
///              /Matrix [0.4124 0.2126 0.0193 …] >>] 8 0 R << /Subtype /DeviceN >>]
/// ```
///
/// §8.6.6.5's `DeviceN` with three colourants *named* `L`, `A` and `B`, a tint transform into a
/// `CalRGB` alternate whose white point is D65 and whose matrix is the sRGB primaries. The names
/// are a producer's labels and §8.6.6.5 is explicit that they are: a `DeviceN` space's components
/// mean whatever its tint transform says they mean, and the alternate is where the colour is.
///
/// # The ranking sent this page here, and the ratio is what did it
///
/// 0.73 from the nearest reference and **14.86 from the furthest** — a ratio of twenty, which
/// `doc/todo/00`'s step 1 reads as a page about the references rather than about us. The ink
/// says which reference:
///
/// ```text
/// ours 43.5058 │ hayro 43.469 │ poppler 43.7916 │ ghostscript 44.836 │ mupdf 62.0301
/// ```
///
/// and the ladder settles it: `poppler` descends from 43.7916 at 72 dpi onto **43.4975** at 576,
/// and ours is flat at 43.4945 — **0.003 of 255 apart**, which is the closest this bucket has
/// come to two renderers producing the same number. `mupdf` is 18 of 255 above everybody and its
/// panel is a dark teal where the other four draw a green-to-magenta ramp: it is not drawing the
/// same colours at all.
///
/// Nothing here is a defect of ours, and the page is `ambiguous` rather than agreeing for the
/// arithmetic reason trap 12 describes — one reference far enough out drags the consensus apart.
const AMBIGUOUS_DEVICE_N_ALTERNATE: [&str; 1] = ["issue17065.pdf page 1"];

/// Ambiguous, and the clause puts the answer beyond itself and says where.
///
/// `calrgb.pdf` is a test sheet: seventeen pages of `CalRGB` patches, each page stating its own
/// `/WhitePoint`, `/BlackPoint`, `/Gamma` and `/Matrix` in a header and then a grid of swatches
/// labelled with the `A, B, C` that produced them. Five renderers produce five sheets, and eight
/// of the pages reach §3a's bucket with a shape nothing else in it has: similarity 0.9322 to
/// 0.9932 — the same shapes in the same places — with worst tiles up to **76.5**, which is a
/// large difference in *colour* over a small part of the page.
///
/// # What the clause determines, and it is all of the first half
///
/// §8.6.5.3 defines the conversion from a `CalRGB`'s components to CIE XYZ exactly: the three
/// components are raised to `/Gamma` and multiplied by `/Matrix`. The first page's space is
/// `/WhitePoint [1 1 1]`, `/BlackPoint [0 0 0]`, `/Gamma [1 1 1]` and an identity `/Matrix`, so
/// on that page the arithmetic is the identity and the file is stating XYZ values directly — and
/// among them are triples like `(1.00, 0.00, 0.00)`, which is not a colour any display can show
/// and, with the later pages' `/WhitePoint [2.0000 1.0000 1.7000]`, not a plausible illuminant
/// either. **The sheet is asking every processor what it does outside the gamut.**
///
/// # What it does not determine, and §10.3.1 says so in **one** of its sentences
///
/// > The specific method by which the CIE-based destination colour space is established is
/// > beyond the scope of this document, but may include the use of Output Intents
///
/// So the *destination* — which pixel space an XYZ is being carried into — is each processor's,
/// and this sheet is built to make that visible. Ours is one route and it is written down rather
/// than tuned: Bradford adaptation to D50 and then the sRGB matrix and transfer, in
/// `colour::xyz_d50_to_srgb`, which is the *only* place in the tree where an XYZ becomes a pixel
/// (ADR 0012) — `Lab`, `CalGray`, `CalRGB` and every ICC profile arrive there, so the four cannot
/// drift apart.
///
/// **The journey to it is not open, and this note said it was for as long as it has existed.**
/// The next sentence of the same subclause is a `shall`: a CIE-based source colour is to be
/// converted to a CIE-based destination colour based on the appropriate ICC specification. (Prose
/// rather than a quotation because Errata Collection 3's Issue #181, `Review`/`Completed`, strikes
/// that sentence's dated *ISO 15076-1:2010 (ICC.1:2010)* and points at Table 66 instead;
/// `spec-errata emit` files the strike under §10.4.1's heading and §10.3.1's ledger row carries
/// it.) So what is a processor's here is the *choice* of destination, and the transform onto it is
/// the referenced standard's — which is why [`CONTRADICTED_CALRGB_TO_SCREEN`], four pages of this
/// same document, turned out to be a difference of **route** rather than of taste once somebody
/// read the sentence after the one quoted above (ADR 0494). This note is the third home of the
/// half-read and the twentieth sweep is what pointed at it (ADR 0495).
///
/// This is still §3a's third shape, and the sharpest instance of it the bucket holds — but the
/// open part is one sentence narrower than this note claimed: the clause is closed about the
/// conversion, closed about §8.6.5.3's arithmetic, and open only about which space the pixels are.
///
/// **Four more of this document's pages carry the same reading one verdict over.**
/// [`CONTRADICTED_CALRGB_TO_SCREEN`] holds pages 1, 5, 11 and 12, which differ from each other only
/// in `/BlackPoint` and are contradicted rather than ambiguous for one reason: on them the two
/// references that share a camp agree to 4.41%, so the bound derived from them is tight enough to
/// exclude us. Same document, same clause, same open half — a different bound.
const AMBIGUOUS_CALRGB_TO_SCREEN: [&str; 8] = [
    "calrgb.pdf page 3",
    "calrgb.pdf page 6",
    "calrgb.pdf page 8",
    "calrgb.pdf page 9",
    "calrgb.pdf page 10",
    "calrgb.pdf page 15",
    "calrgb.pdf page 16",
    "calrgb.pdf page 17",
];

/// Ambiguous, and it is **one paper under fifteen names** — 155 of this list's 179.
///
/// **These paragraphs spent an unknown number of sessions above a different group's list.**
/// They are this list's opening — the section below begins *and a second document* and the
/// first was never above it — and they sat instead above `AMBIGUOUS_DEVICE_N_ALTERNATE`, whose
/// own note therefore opened with forty lines about a paper it holds no page of. Nothing said
/// so: a doc comment is attached to whatever declaration follows it, and both declarations are
/// page lists. The twentieth sweep found it by arithmetic rather than by reading — it reported
/// a one-page `DeviceN` group quoting a band `mean 3.51 to 9.93` that none of its one page
/// carries — which is a fourth way for a note to be wrong, after its name, its reading and its
/// figures: **a note attached to the wrong list.** ADR 0495.
///
/// `tracemonkey.pdf` is pdf.js's canonical fixture: *Trace-based Just-in-Time Type
/// Specialization for Dynamic Languages*, fourteen pages of two-column academic text at about
/// nine points. Eleven other corpus documents are the same fourteen pages with something added
/// — highlights, comments, free text, an editable annotation, an accessibility tree — and
/// `pdftotext` on page 9 gives the *same md5* for all of them. So this group is one document
/// under many names and **one** finding, and saying otherwise would make the count a vanity
/// number.
///
/// # What the closed form says, on five pages of it
///
/// ```text
///          ours 1x   ours 8x   poppler 1x   poppler 8x   mupdf 8x
/// page 1   14.8523   14.8523    14.7315      14.8540     14.8678
/// page 4   19.9253   19.9064    19.7787      19.9137     19.9328
/// page 9   20.0751   20.0599    19.9217      20.0695     20.0890
/// page 11  14.8750   14.9197    15.7177      14.9441     14.9504
/// page 14   7.1371    7.1170     7.0054       7.1184      7.1290
/// ```
///
/// The two ladders agree to **0.014 to 0.023 of 255** on every page, so there is a limit each
/// time; ours at 8× lands between them or a hundredth under, and **ours at the page's own scale
/// is already there** — 0.02 to 0.05 from our own limit, where `poppler` at 72 dpi is 0.12 to
/// 0.15 *below* its. On page 9 the five renderers' ink is ours 20.075, `ghostscript` 20.249,
/// `mupdf` 19.942, `poppler` 19.922, `hayro` 19.723: a spread of 0.53, and ours is the nearest
/// of the five to the geometry.
///
/// So nobody is drawing anything anybody else is not. What makes all of them `ambiguous` is the
/// **text** tolerance — 0.90 structural similarity, measured over 153 reference-against-
/// reference pairs because five rasterisers cannot agree more closely than that about small
/// glyphs — applied to a page that is nothing but small glyphs. Across the paper's 155 pages the
/// metrics form one band: mean 3.51 to 9.91, worst tile 20.94 to 42.96, ssim 0.7977 to 0.9195,
/// against bounds of 5.00, 40.00 and 0.9000, with 7.07% to 19.15% of pixels differing against a
/// bound of 5.00%. (The band read `9.93`, `48.31` and `0.9194` until ADR 0495 re-derived it from
/// the gate's own per-page lines; the two ends that moved are pages the tree has drawn
/// differently since, not a different population.) §10.7.4's own last sentence licenses the
/// spread, and `AMBIGUOUS_GLYPH_SCAN_CONVERSION` quotes it.
///
/// **`poppler` on page 11 is the one number worth keeping.** It is 15.7177 at 72 dpi against its
/// own 14.9441 at 576 — 5% over its geometry where every other page has it 1% under. That is
/// §10.7.4 as written on marks a fraction of a pixel wide, one page over from where
/// `AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY` found it, and it is not ours.
///
/// # And a second document, on the same instrument, in the two-hundred-and-sixty-third
///
/// `TAMReview.pdf` is 23 pages of a technical review at paper size — dense text, tables, the same
/// nine-point band — and 22 of them are in this bucket. Read as a population the way both books
/// were: mean 4.05 to 9.96, worst tile 20.15 to 33.65, similarity 0.7723 to 0.9214, one band and
/// no page outside it. Four pages spread through it, with two ladders each:
///
/// ```text
///           ours 1x   ours 8x   poppler 72   poppler 576   mupdf 576
/// page 1    10.0004   10.0181     10.0398      10.0396      10.0184
/// page 8    11.0257   10.9948     10.8604      11.0049      11.0154
/// page 16    7.4774    7.5113      7.7494       7.5645       7.5272
/// page 23   11.6485   11.7169     11.8844      11.9601      11.6462
/// ```
///
/// Ours at 8× is within 0.05 of `poppler`'s limit on three of the four and between the two
/// references' limits on the fourth, where the references are 0.31 apart — which is this group's
/// own finding restated: five glyph rasterisers cannot agree more closely than the tolerance
/// allows, and the tolerance is what makes the page ambiguous rather than anything on it.
/// # Three more of the same paper, in the three-hundred-and-first
///
/// `doc/todo/00`'s step 2 again, and it produced three of the ranking's top six.
/// `issue15012.pdf` and `bug1885505.pdf` give the **same md5** for page 1's readback as
/// `tracemonkey.pdf`, so they are the paper's first page under two more names; `issue7014.pdf`
/// is it a third time with §12.5.6.10's markup over the abstract — a highlight, a squiggly
/// underline and a strike-out — which ours and `poppler` draw indistinguishably, same colour,
/// same extent, and which costs 0.1 of 255.
///
/// ```text
/// issue7014 p1     ours 15.2915  poppler 15.2197  mupdf 15.1241  hayro 15.0175  gs 15.4563
/// issue15012 p1    ours 14.9054  poppler 14.7852  mupdf 14.6921  hayro 14.6292  gs 14.9939
/// bug1885505 p1    ours 15.8159  poppler 15.8634  mupdf 15.6451  hayro 15.6507  gs 16.1317
/// ```
///
/// A band 0.44 to 0.50 of 255 wide with ours inside it on all three, which is this group's own.
///
/// **And the md5 method has a false positive worth naming.** `multiline.pdf` and
/// `bug852992_reduced.pdf` also matched each other — on the md5 of *no text at all*. A page whose
/// readback is empty matches every other page whose readback is empty, so the check is evidence
/// only when the readback is non-empty, and `doc/todo/00`'s recipe now says so.
/// # And one that is not the paper at all, in the three-hundred-and-eighth
///
/// `issue6127.pdf` page 3 is a French social-security form — a whole A4 of five- and six-point
/// type set in a red-orange, and nothing else. It is in this group because the group's *premise*
/// is what it demonstrates rather than because it is the same document:
///
/// ```text
///                 72 dpi    576 dpi
/// poppler        13.0523   12.7763
/// mupdf          12.9968   12.0742
/// ours (1x)      12.7522
/// hayro          11.7778   ghostscript 17.0525
/// ```
///
/// **The two ladders end 0.70 of 255 apart**, which every other page in this bucket has settled
/// to within a hundredth — so on type this small there is no limit to be near, which is exactly
/// what the 0.90 similarity tolerance was measured over 153 reference-against-reference pairs to
/// express. Ours lands 0.024 from `poppler`'s end and 0.68 from `mupdf`'s, and `ghostscript` is
/// 4.3 above every one of the four.
/// # And one this gate had never judged, in the three-hundred-and-eighty-third
///
/// `issue14297.pdf` page 1 is an A3 financial statement — two consolidated tables of five-point
/// type and nothing else — and it reached this bucket by being *drawn*: it reported a soft mask
/// group compositing more than one unit of ink until ADR 0220, so the oracle had never judged it
/// at all. Its verdict is the text tolerance's (mean 4.33 against a bound of 5.00, similarity
/// 0.9135 against 0.9000), and its ink at the page's own scale is 1.14 of 255 *below* the
/// lightest reference, which is the shape that looks like missing marks.
///
/// It is not. Two ladders and ours beside them:
///
/// ```text
///              72 dpi   288 dpi   576 dpi
/// poppler     10.1206   8.73678   8.75421
/// mupdf        9.83978  8.99105   8.87480
/// ours         8.70517  8.75412   8.79450
/// ```
///
/// The references' 72-dpi ink is what their scan conversion adds to type this small, and it falls
/// away as the pixels do: `poppler` loses 1.37 of 255 between 72 and 576 dpi and `mupdf` 0.96,
/// while ours *rises* 0.09 and is within 0.05 of `poppler`'s limit at the page's own scale.
/// At eight times ours is 8.794, between `poppler`'s 8.754 and `mupdf`'s 8.875 — this group's
/// premise exactly, one axis over: on five-point type five glyph rasterisers cannot agree more
/// closely than the tolerance allows, and here they cannot even agree with *themselves* until the
/// resolution is eight times the page's.
///
/// **Our column was re-measured in the seven-hundred-and-fifteenth and the paragraph's argument
/// survived it unchanged** (ADR 0590). This page states 45 fills whose axis-aligned rectangles
/// share a device pixel, so it is in that decision's population and its ink moved — but the mean
/// moved by 0.01 and the ladder in its third decimal, and every relation the paragraph rests on
/// still holds. The stale digits were mostly *not* that round's: `cargo run --release -p
/// conformance --bin quoted` caught the mean this paragraph used to quote, and the ladder had
/// drifted 0.01 to 0.03 across the
/// rounds since the three-hundred-and-eighty-third, unpointed-at. Trap 1's third shape, found by
/// the instrument built for it. (The superseded figures are deliberately not spelled here: a note
/// narrating its own correction in the gate's vocabulary is that sweep's own documented noise, and
/// `git log -p` is where a retired number belongs.)
///
/// # And two whose entries left by being *reported*, and came back when the report was wrong
///
/// `comments.pdf` page 1 and `highlights.pdf` page 1 are the same PLDI paper as the rest of their
/// runs, and the diagnosis above has described them the whole time. Their entries were taken out
/// when ADR 0359 made a damaged form `XObject` loud: both pages draw an ink annotation whose
/// appearance invokes a form whose flate stream carries no RFC 1951 final block, so both became
/// pages this reader calls **incomplete** — and the undiagnosed check is over `complete &&
/// Ambiguous`, so an incomplete page owes no diagnosis and an entry for one fails the ratchet's
/// other direction. `issue3885.pdf` page 1 went the same way and was never in a group.
///
/// **That stream is not damaged.** Its producer flushed and never called `deflateEnd`, so every
/// byte it was given is present and what was absent is the declaration that there is no more; ADR
/// 0744 decides which of the two it is by handing the decoder RFC 1951's final empty block and
/// requiring `StreamEnd` with no further output, rather than by reading the tail bytes. The pages
/// stop reporting, become complete, and owe a diagnosis again.
///
/// **What the round trip is worth keeping is what it did *not* move.** Their verdict is
/// `ambiguous` on both sides — before it the gate printed `ambiguous (incomplete)` — so what a
/// wrong report cost was never the verdict but the *diagnosis*: a page nobody has to explain is a
/// page nobody opens, and while it stood no instrument could notice, because a page exempted from
/// explanation cannot be found to want one. Nothing about the pages themselves moved in either
/// direction — `examples/display_list_digest` was byte-identical over all 974 documents across
/// the change that removed the entries, and the change that brings them back alters no mark.
const AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE: [&str; 181] = [
    "comments.pdf page 1",
    "highlights.pdf page 1",
    "issue14297.pdf page 1",
    "issue6127.pdf page 3",
    "issue7014.pdf page 1",
    "issue15012.pdf page 1",
    "bug1885505.pdf page 1",
    "bug1992868.pdf page 1",
    "bug1992868.pdf page 2",
    "bug1992868.pdf page 3",
    "bug1992868.pdf page 4",
    "bug1992868.pdf page 5",
    "bug1992868.pdf page 6",
    "bug1992868.pdf page 7",
    "bug1992868.pdf page 8",
    "bug1992868.pdf page 9",
    "bug1992868.pdf page 10",
    "bug1992868.pdf page 11",
    "bug1992868.pdf page 12",
    "bug1992868.pdf page 13",
    "bug1992868.pdf page 14",
    "comments.pdf page 2",
    "comments.pdf page 3",
    "comments.pdf page 4",
    "comments.pdf page 5",
    "comments.pdf page 6",
    "comments.pdf page 7",
    "comments.pdf page 8",
    "comments.pdf page 9",
    "comments.pdf page 10",
    "comments.pdf page 11",
    "comments.pdf page 12",
    "comments.pdf page 13",
    "comments.pdf page 14",
    "highlights.pdf page 2",
    "highlights.pdf page 3",
    "highlights.pdf page 4",
    "highlights.pdf page 5",
    "highlights.pdf page 6",
    "highlights.pdf page 7",
    "highlights.pdf page 8",
    "highlights.pdf page 9",
    "highlights.pdf page 10",
    "highlights.pdf page 11",
    "highlights.pdf page 12",
    "highlights.pdf page 13",
    "highlights.pdf page 14",
    "issue12337.pdf page 2",
    "issue12337.pdf page 3",
    "issue12337.pdf page 4",
    "issue12337.pdf page 5",
    "issue12337.pdf page 6",
    "issue12337.pdf page 7",
    "issue12337.pdf page 8",
    "issue12337.pdf page 9",
    "issue12337.pdf page 10",
    "issue12337.pdf page 11",
    "issue12337.pdf page 12",
    "issue12337.pdf page 13",
    "issue12337.pdf page 14",
    "issue18911.pdf page 1",
    "issue18911.pdf page 2",
    "issue18911.pdf page 3",
    "issue18911.pdf page 4",
    "issue18911.pdf page 5",
    "issue18911.pdf page 6",
    "issue18911.pdf page 7",
    "issue18911.pdf page 8",
    "issue18911.pdf page 9",
    "issue18911.pdf page 10",
    "issue18911.pdf page 11",
    "issue18911.pdf page 12",
    "issue18911.pdf page 13",
    "issue18911.pdf page 14",
    "issue19239.pdf page 1",
    "issue19239.pdf page 2",
    "issue19239.pdf page 3",
    "issue19239.pdf page 4",
    "issue19239.pdf page 5",
    "issue19239.pdf page 6",
    "issue19239.pdf page 7",
    "issue19239.pdf page 8",
    "issue19239.pdf page 9",
    "issue19239.pdf page 10",
    "issue19239.pdf page 11",
    "issue19239.pdf page 12",
    "issue19239.pdf page 13",
    "issue19239.pdf page 14",
    "tracemonkey.pdf page 1",
    "tracemonkey.pdf page 2",
    "tracemonkey.pdf page 3",
    "tracemonkey.pdf page 4",
    "tracemonkey.pdf page 5",
    "tracemonkey.pdf page 6",
    "tracemonkey.pdf page 7",
    "tracemonkey.pdf page 8",
    "tracemonkey.pdf page 9",
    "tracemonkey.pdf page 10",
    "tracemonkey.pdf page 11",
    "tracemonkey.pdf page 12",
    "tracemonkey.pdf page 13",
    "tracemonkey.pdf page 14",
    "tracemonkey_a11y.pdf page 1",
    "tracemonkey_annotation_on_page_8.pdf page 1",
    "tracemonkey_annotation_on_page_8.pdf page 2",
    "tracemonkey_annotation_on_page_8.pdf page 3",
    "tracemonkey_annotation_on_page_8.pdf page 4",
    "tracemonkey_annotation_on_page_8.pdf page 5",
    "tracemonkey_annotation_on_page_8.pdf page 6",
    "tracemonkey_annotation_on_page_8.pdf page 7",
    "tracemonkey_annotation_on_page_8.pdf page 8",
    "tracemonkey_annotation_on_page_8.pdf page 9",
    "tracemonkey_annotation_on_page_8.pdf page 10",
    "tracemonkey_annotation_on_page_8.pdf page 11",
    "tracemonkey_annotation_on_page_8.pdf page 12",
    "tracemonkey_annotation_on_page_8.pdf page 13",
    "tracemonkey_annotation_on_page_8.pdf page 14",
    "tracemonkey_freetext.pdf page 1",
    "tracemonkey_freetext.pdf page 2",
    "tracemonkey_freetext.pdf page 3",
    "tracemonkey_freetext.pdf page 4",
    "tracemonkey_freetext.pdf page 5",
    "tracemonkey_freetext.pdf page 6",
    "tracemonkey_freetext.pdf page 7",
    "tracemonkey_freetext.pdf page 8",
    "tracemonkey_freetext.pdf page 9",
    "tracemonkey_freetext.pdf page 10",
    "tracemonkey_freetext.pdf page 11",
    "tracemonkey_freetext.pdf page 12",
    "tracemonkey_freetext.pdf page 13",
    "tracemonkey_freetext.pdf page 14",
    "tracemonkey_with_annotations.pdf page 1",
    "tracemonkey_with_annotations.pdf page 2",
    "tracemonkey_with_annotations.pdf page 3",
    "tracemonkey_with_annotations.pdf page 4",
    "tracemonkey_with_annotations.pdf page 5",
    "tracemonkey_with_annotations.pdf page 6",
    "tracemonkey_with_annotations.pdf page 7",
    "tracemonkey_with_annotations.pdf page 8",
    "tracemonkey_with_annotations.pdf page 9",
    "tracemonkey_with_annotations.pdf page 10",
    "tracemonkey_with_annotations.pdf page 11",
    "tracemonkey_with_annotations.pdf page 12",
    "tracemonkey_with_annotations.pdf page 13",
    "tracemonkey_with_annotations.pdf page 14",
    "tracemonkey_with_editable_annotations.pdf page 1",
    "tracemonkey_with_editable_annotations.pdf page 2",
    "tracemonkey_with_editable_annotations.pdf page 3",
    "tracemonkey_with_editable_annotations.pdf page 4",
    "tracemonkey_with_editable_annotations.pdf page 5",
    "tracemonkey_with_editable_annotations.pdf page 6",
    "tracemonkey_with_editable_annotations.pdf page 7",
    "tracemonkey_with_editable_annotations.pdf page 8",
    "tracemonkey_with_editable_annotations.pdf page 9",
    "tracemonkey_with_editable_annotations.pdf page 10",
    "tracemonkey_with_editable_annotations.pdf page 11",
    "tracemonkey_with_editable_annotations.pdf page 12",
    "tracemonkey_with_editable_annotations.pdf page 13",
    "tracemonkey_with_editable_annotations.pdf page 14",
    "TAMReview.pdf page 1",
    "TAMReview.pdf page 2",
    "TAMReview.pdf page 3",
    "TAMReview.pdf page 4",
    "TAMReview.pdf page 5",
    "TAMReview.pdf page 6",
    "TAMReview.pdf page 7",
    "TAMReview.pdf page 8",
    "TAMReview.pdf page 9",
    "TAMReview.pdf page 10",
    "TAMReview.pdf page 11",
    "TAMReview.pdf page 12",
    "TAMReview.pdf page 13",
    "TAMReview.pdf page 14",
    "TAMReview.pdf page 15",
    "TAMReview.pdf page 16",
    "TAMReview.pdf page 17",
    "TAMReview.pdf page 18",
    "TAMReview.pdf page 19",
    "TAMReview.pdf page 20",
    "TAMReview.pdf page 22",
    "TAMReview.pdf page 23",
];

/// Ambiguous, and the reference is the one departing — from a rule PDF states for *encoders*.
///
/// `issue11931.pdf` is 1280 × 720 and two `Do`s: a full-page `FlateDecode` `DeviceRGB` image and
/// a 256 × 16 `DCTDecode` one stretched across a 1146 × 72 band. Ink: ours **1.357**,
/// `hayro` 1.371, `mupdf` 1.382, `poppler` 1.395 — and `ghostscript` **8.831**, which is six and
/// a half times the page. The band is a magenta bar with a green arc in it where the other four
/// draw nothing anybody would notice.
///
/// # What the codestream says, read rather than guessed
///
/// The JPEG's `SOF0` declares three components with identifiers `0x52 0x47 0x42` — the ASCII
/// letters **R, G, B**. There is no `JFIF` `APP0` and no Adobe `APP14`, and the image dictionary
/// states no `/DecodeParms`.
///
/// # What §7.4.8's Table 13 determines, and it is the opposite
///
/// > If the Adobe-defined marker code (APP14) in the encoded data indicating the ColorTransform
/// > value is not present and this dictionary entry is not present in the filter dictionary then
/// > the default value of ColorTransform shall be 1 if the image has three components and 0
/// > otherwise.
///
/// `ColorTransform` 1 means "from YCbCr to RGB after decoding". So **`ghostscript` is the one
/// obeying the clause**, and its magenta band is what this file says on a literal reading. The
/// other four — this tree among them — treat the samples as RGB already, because the component
/// identifiers say so; `zune-jpeg` does it in ten lines whose own comment reads "I am not sure
/// if this is even specified in any standard".
///
/// It is not. The convention is `libjpeg`'s, and no clause of ISO 32000-2 or ISO/IEC 10918-1
/// gives a component identifier any meaning at all.
///
/// # Why that is still the right answer here, and it is `CLAUDE.md`'s two denominators
///
/// §7.4.8 closes with a `shall` that is addressed to the *producer*:
///
/// > The exact rules for producing and consuming DCT encoded data within PostScript language are
/// > provided in Adobe Technical Note #5116 and PDF DCT encoding shall exactly follow all those
/// > rules established by Adobe for the PostScript language.
///
/// A conforming file's three-component codestream carries the `APP14` marker or is YCbCr. This
/// one is neither, so it is outside what the clause describes, and the question stops being
/// *coverage* — which of the standard's requirements are implemented — and becomes
/// *robustness*: what a reader does with a file the standard does not describe. Table 13's
/// default is the answer for files that follow the rules; the codestream's own declaration is
/// the only evidence there is for a file that does not.
///
/// **What was wrong until the two-hundred-and-thirty-fourth session is that nothing said so.**
/// §7.4.8's ledger row recorded one half of this departure — `issue12841_reduced.pdf`, where the
/// dictionary says `/ColorTransform 0` and we transform anyway — and was silent about the other,
/// which is the same decision seen from the other side: **on a `DCTDecode` image this tree lets
/// the codestream decide, not the dictionary, in both directions.** One policy, stated once now.
const AMBIGUOUS_JPEG_COMPONENT_IDS: [&str; 1] = ["issue11931.pdf page 1"];

/// Ambiguous, and the page exists at all because §7.7.3.2's tree was rebuilt from Table 31.
///
/// `issue21436.pdf` is 450 bytes and its whole content stream is `10 10 180 180 re S`. The trap
/// is one reference: the catalogue's `/Pages` names object **3**, which is `/Type /Page` — a
/// leaf where the tree's root belongs — while object 2 is the `/Type /Pages` node nothing points
/// at. **`mupdf` refuses the document outright** ("invalid page number: -1"), so only three
/// renderers and `hayro` are comparable at all.
///
/// `Pages::new` recovers it the way `xref::rebuild` recovers a cross-reference table: Table 31
/// makes `/Type` required of a page object and says it "shall be Page", so a document whose tree
/// walks to nothing is *asked* instead — every object declaring itself a page is one. That is a
/// recovery from the file's own declarations and from no other reader's behaviour, and it runs
/// only where the tree produced nothing, so 963 of the 974 corpus documents never reach it.
///
/// What is left is one stroke, and the closed form settles it in one line: **ours is 4.5836 at
/// the page's own scale and 4.5900 at eight times it, and `poppler` is 4.5900 at both.** The
/// file states no `w`, so §8.4.3.2's default width of 1.0 applies and the geometry is exactly
/// 4.59. `ghostscript` is **5.85**, 27% over, which is §10.7.4 as written — "any pixel whose
/// half-open square region intersects the shape" — on a one-unit stroke centred on an integer
/// coordinate, so it covers two whole columns where the shape covers one.
///
/// Three renderers, three answers, and the two that are not ours are a refusal and a documented
/// departure. `AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY` has the same subject with the sign the
/// other way.
const AMBIGUOUS_RECOVERED_PAGE_TREE: [&str; 1] = ["issue21436.pdf page 1"];

/// Ambiguous, and it is §11.4's group compositing with one renderer alone in it.
///
/// `transparency_group.pdf` page 1 came off §3a's ranking in the three-hundred-and-thirty-third
/// session at 0.50 from the nearest reference and 3.76 from the furthest. **Three commands**: two
/// overlapping ellipses filled with axial shadings inside a §11.4.7 transparency group, which is
/// what the corpus put the file there to exercise.
///
/// ```text
///                 72 dpi    576 dpi
/// mupdf          30.2985   30.2768
/// poppler        29.6131   29.5573
/// ours (1x/8x)   29.5207   29.5512
/// ```
///
/// Both ladders are flat from the start — a page of large smooth fills has no edges to converge
/// — and they end **0.72 of 255 apart**, so there is no consensus to sit inside or outside.
/// What decides the grouping is who is with whom: at the page's own scale ours is 29.5207,
/// `hayro` 29.5058 and `poppler` 29.6131, while `mupdf` is 30.2985 and `ghostscript` 30.4637.
/// **Three renderers within 0.09 and two others 0.9 above them**, and the four-panel strip shows
/// where it lives: the region where the two ellipses overlap is visibly darker in `mupdf`'s.
///
/// So this is §11.4.7's page group and §11.6.6's blending space in one picture, and the clause is
/// what says nobody can be checked against anybody: an isolated group's backdrop is "a
/// transparent backdrop" and what a processor composites it onto afterwards is the *device's*
/// space, which §10.3.1 puts beyond the standard. `doc/todo/23` holds the departures this tree
/// reports by name; this page reports none, and the difference is a compositing space rather than
/// a missing mark.
const AMBIGUOUS_TRANSPARENCY_GROUP: [&str; 1] = ["transparency_group.pdf page 1"];

/// Ambiguous, and it is a **blur test** where five renderers give five answers to one clause.
///
/// `issue19634.pdf` is Skia's own `blurSmallRadii`: a 100 × 100 page drawing *guest* five times,
/// each a red blurred copy under a green sharp one, the radius growing down the page. The red
/// is a Type 3 font whose glyph procedure is `d1`, a `gs` naming an `/SMask << /S /Luminosity >>`
/// and one `re f` — so the blur is a *mask* and the glyph is a rectangle poured through it.
///
/// ```text
/// ours 8.03    hayro 8.11    mupdf 7.63    poppler 16.64    ghostscript 47.98
/// ```
///
/// **Ours was 2.87 until the two-hundred-and-thirty-seventh session**, drawing no red at all:
/// §8.6.8's uncoloured restriction was still in force inside the mask's own group, so the
/// group's image was skipped, the mask came out zero and every glyph the font drew was masked
/// away — with all 51 of the page's commands present and nothing reported. ADR 0173 has the
/// reading; we now sit between the two references that draw the same picture.
///
/// # What the clause determines about the two that do not
///
/// `ghostscript` paints solid red blocks — the mask ignored entirely — which is §8.6.8's
/// `/ExtGState` list read as though `/SMask` were on it. It is not: the list is `TR`, `TR2`,
/// `HT`, `BG`, `BG2`, `UCR`, `UCR2` and `UseBlackPtComp`, and every one of them is a colour
/// decision §8.6.8 reserves for whoever uses an uncoloured figure rather than a shape it may
/// state. `poppler` at 16.64 is between the two, which is a mask applied at some other
/// strength.
///
/// What is left between ours, `mupdf` and `hayro` is half a level of 255 on a blur, and it is
/// §11.6.5.2's own arithmetic on a `DCTDecode` greyscale image — the three of us differ by less
/// than the eight-bit quantisation of the mask at its brightest, which this image reaches at
/// 110 of 255. The page stays `ambiguous` because two renderers are 2× and 6× away, which is a
/// statement about them.
const AMBIGUOUS_MASKED_BLUR: [&str; 1] = ["issue19634.pdf page 1"];

/// Ambiguous, and ours is on `poppler`'s limit while `mupdf` is 0.14 of 255 below both.
///
/// `bug1703683_page2_reduced.pdf` page 1 is a photograph of a power adapter whose LED highlight is
/// drawn through a `/Luminosity` soft mask: a `/DeviceGray` group containing a `/DeviceN` shading
/// whose alternate rests on `DeviceCMYK`. It reached this bucket by being *drawn* — the round
/// before this one found that shading and reported it, and ADR 0220 made the ramp reach the mask
/// in the components §11.5.3 composites, so the page stopped reporting and started being judged.
///
/// Its verdict is the ordinary tolerance's on everything but one tile: mean 0.40 against a bound
/// of 1.00, similarity 0.9903 against 0.9900, and a worst tile of 10.01 against 5.00 — which is
/// the highlight and nothing else, 192 pixels of 1.9 M.
///
/// ```text
///              72 dpi   288 dpi   576 dpi
/// poppler     5.42909   5.37225   5.36945
/// mupdf       5.22151   5.22336   5.22575
/// ours        5.36249   5.36410   5.36273
/// ```
///
/// Ours is flat to four decimals across three scales — a page of large smooth fills has no edges
/// to converge — and it ends **0.007 of 255** from `poppler`'s limit, which is two renderers
/// agreeing rather than two renderers being close. `mupdf` is flat too, 0.14 below both, and
/// `ghostscript` 0.11 above: five answers spanning a quarter of a level on a photograph, which is
/// what `AMBIGUOUS_DEVICE_CMYK_CONVERSION` says about any page whose colour rests on a press.
/// **The page left this bucket again in the four-hundred-and-fifteenth session, and the reason
/// is one entry nothing had read.** Its page dictionary states
/// `/Group << /S /Transparency /CS /DeviceCMYK >>`, which §11.4.7 makes "the default blending
/// colour space for each page" — so every mark on it composites in ink and this tree composites
/// in the device's three components. The page reports that by name now and is no longer judged.
/// The measurement above stands as what was found while it was, and it is *consistent* with the
/// report rather than superseded by it: a photograph whose five renderers span a quarter of a
/// level is exactly the page where a colour-space departure is small and real.
const AMBIGUOUS_SUBTRACTIVE_MASK_GROUP: [&str; 0] = [];

/// Ambiguous, and Table 87 defines the premultiplication and nothing else.
///
/// `jpx_smaskindata.pdf` is 140 × 40 of grey with four 2 × 1 `JPXDecode` images blown up to
/// 30 × 30. Three state `/SMaskInData 2` and one states 1; two of the three also state a
/// `/Matte` — `[0 1 0]` and `[0 0 1]` — in the **image** dictionary.
///
/// §8.9.5.1 Table 87, of code 2:
///
/// > The image's data stream includes colour channels that have been premultiplied with an
/// > opacity channel; the image data also includes the opacity channel.
///
/// Premultiplied with an *opacity channel*, and with nothing else: the inverse is one division
/// per component, which `jpx_samples_to_rgba` does. **Table 87 has no `/Matte`.** That entry is
/// Table 144's, in a *soft-mask image's* dictionary, "specifying the matte colour with which
/// the image data in the parent image shall have been pre-blended" — and where the mask travels
/// inside the codestream there is no soft-mask image dictionary to put it in. So this file
/// states a pre-blend in a place the standard defines no meaning for, and the four renderers
/// answer four ways:
///
/// ```text
/// ours 127.38   ghostscript 128.22   mupdf 135.60   hayro 139.21   poppler 165.18
/// ```
///
/// **Ours and `ghostscript` are the same picture** — 0.0033 of a channel apart over the whole
/// page, which is two renderers agreeing rather than two renderers being close — while
/// `ghostscript` against `poppler` is 0.248, a quarter of every channel. The reading behind that
/// agreement is Table 87's sentence taken as the whole of what code 2 means.
///
/// The page stays `ambiguous` because three references disagree with us and with each other,
/// which is what it looks like when a file states something the standard does not define.
const AMBIGUOUS_MATTE_WITHOUT_A_SOFT_MASK_IMAGE: [&str; 1] = ["jpx_smaskindata.pdf page 1"];

/// Ambiguous, and ours ends **0.0030 of 255** from `poppler`'s limit on three flat ladders.
///
/// `issue12798_page1_reduced.pdf` page 1 is a public-health poster: white space over a magenta
/// band carrying two lines of small type. Four commands. It reached this bucket by being
/// *drawn* — until the four-hundredth session its artwork sat inside a **non-isolated group
/// under a soft mask** whose elements blend `Multiply` and `Screen`, which §11.4.4's NOTE 5
/// cannot flatten, and the page reported it by name (ADR 0237).
///
/// Its verdict is the ordinary tolerance's on everything but one tile: mean 0.26 against a
/// bound of 1.00, similarity 0.9963 against 0.9900, 0.48% of pixels differing — and a worst
/// tile of 8.79 against 5.00, which is the small white type on the band and nothing else.
///
/// ```text
///              72 dpi   288 dpi   576 dpi
/// poppler     23.7199   23.8327   23.8408
/// mupdf       23.8604   23.9477   23.9610
/// ours        23.8002   23.8373   23.8438
/// ```
///
/// Three ladders climbing in parallel, each converged by 288 dpi, and ours ends **0.0030**
/// from `poppler`'s limit while the two references' own limits are **0.120** apart. At the
/// page's own scale the five renderers span 23.72 to 24.01 — a tenth of one level of 255 over
/// the whole poster — and ours is second of the five. The page stays `ambiguous` because a
/// worst-tile bound measured over glyph edges is tighter than five renderers' scan conversion
/// of 6-point type, which is `AMBIGUOUS_GLYPH_SCAN_CONVERSION`'s subject wearing this file's
/// name.
/// **And it left this bucket in the four-hundred-and-fifteenth session for the same reason
/// `AMBIGUOUS_SUBTRACTIVE_MASK_GROUP` did**: its page dictionary states a
/// `/Group << /S /Transparency /CS /DeviceCMYK >>`, §11.4.7 makes that the page's default
/// blending colour space, and this tree composites in the device's three components. Three
/// ladders 0.0030 apart were never the whole story about a poster printed in ink.
///
/// **It came back in the four-hundred-and-twenty-seventh, and the ladder above is the
/// measurement rather than the history.** The page is composited in ink now — §11.4.7's four
/// components in two rasters (ADR 0262) with §11.7.2's conversion *into* the space as a right
/// inverse of the one out (ADR 0263) — and re-measured at 72, 288 and 576 dpi it prints
/// **23.8002, 23.8373 and 23.8438**, which is the row above to the fourth decimal. The band's
/// own colour is `#E60575` against `poppler`'s `#E60576`, `mupdf`'s `#E60376` and
/// `ghostscript`'s `#E50275`. That is the round trip's claim shown on a real page: a colour
/// the assumed inks can make is separated, composited in four components and converted back
/// to the colour the file states.
///
/// What moved is the *tile*, 8.79 → 9.14, and the difference image says where — the outlines
/// of the two lines of type and the band's top edge, with the band's interior black. Small
/// white type on a saturated ground is where a half-covered pixel's colour is decided by the
/// order the conversion and the coverage are applied in, and this tree now applies them in the
/// order §11.4.7 states.
const AMBIGUOUS_NON_ISOLATED_POSTER: [&str; 1] = ["issue12798_page1_reduced.pdf page 1"];

/// Ambiguous, and the five renderers span **3.65 of 255** on a 209 x 90 illustration.
///
/// `issue13520.pdf` page 1 is one glossy blob — an Illustrator drawing of five overlapping
/// lozenges, ten commands, nested transparency groups with `/Luminosity` soft masks, `Screen`
/// blend modes and constant alphas of 0.20, 0.79 and 0.81. It reached this bucket the same way
/// `AMBIGUOUS_NON_ISOLATED_POSTER` did: its outer group is non-isolated under a soft mask with
/// elements that blend, so until the four-hundredth session the page reported §11.4.4 by name
/// and was never judged (ADR 0237).
///
/// ```text
///              72 dpi   288 dpi   576 dpi
/// poppler     17.6287   17.4891   17.4397
/// mupdf       16.5747   16.7086   16.6970
/// ours        16.9987   17.1636   17.1585
/// ```
///
/// **The two references' limits are 0.743 of 255 apart and they approach from opposite sides**,
/// so there is no consensus to sit inside or outside; ours converges by 288 dpi and ends
/// between them, 0.281 below `poppler` and 0.462 above `mupdf`. The other two are further
/// still: at the page's own scale `ghostscript` is 20.03 and `hayro` 20.23 against `mupdf`'s
/// 16.57, which is **3.65 of 255** across five renderers on an illustration 209 pixels wide.
///
/// The four-panel strip says where it lives and the numbers do not: the same lozenge carries a
/// white highlight in ours and `poppler`, a grey one in `mupdf`, an outlined one in
/// `ghostscript` and a dark blot at its right-hand end in `hayro`. That is five readings of a
/// stack of `Screen` blends under luminosity masks, which is §11.6.6's blending space —
/// `doc/todo/23`'s standing item — rather than a mark anybody is missing. This tree reports
/// nothing on the page and `open_one` finds all ten commands.
/// **And that last sentence turned out to name the entry rather than the mechanism, which the
/// four-hundred-and-fifteenth session found by reading it.** The page's blending space is not
/// §11.6.6's at all: its *page* dictionary states `/Group << /S /Transparency /CS /DeviceCMYK >>`
/// and §11.4.7 makes that the default for everything on the page, groups and top-level marks
/// alike. Every group inside it is non-isolated and inherits it. So the five readings of a stack
/// of `Screen` blends are five readings of `Screen` **in ink**, the page reports it by name now,
/// and it is no longer judged here. The hypothesis in the paragraph above was right about the
/// clause family and wrong about which clause, which is trap 1's shape one directory over.
const AMBIGUOUS_STACKED_SCREEN_UNDER_MASKS: [&str; 0] = [];

/// Ambiguous, and three pages where every renderer paints more than the geometry.
///
/// ```text
///                    limit             ours     hayro    poppler   mupdf    ghostscript
/// issue12963 p8     5.6177 / 5.6180   5.601    5.841    5.921    5.630    5.488
/// two_pages p2      1.0448 / 1.0457   1.032    1.029    1.070    1.073    1.063
/// issue12295 p1     7.4106 / 6.9985   7.681   12.744   11.036   10.504   14.763
/// ```
///
/// On the first two the ladders agree to four figures and ours is nearest the geometry — 0.017
/// and 0.013 under, where the pair that is furthest is 0.3 and 0.03 over.
///
/// **`issue12295.pdf` is the extreme of this group's standing subject and is worth its numbers.**
/// The two ladders themselves are 0.41 apart, so there is no exact limit, but both are near 7
/// and **all five renderers are above them at the page's own scale** — `mupdf` by 3.1, `poppler`
/// by 3.6, `hayro` by 5.3, `ghostscript` by 7.4. A page whose marks are thin enough that every
/// renderer paints three to seven levels more than their area is §10.7.4 as written on five
/// implementations at once, and ADR 0025's departure is why ours is the smallest of the five
/// overshoots rather than the largest.
///
/// **Ours read 8.792 until the four-hundred-and-thirty-second session and 7.547 after it**, 1.4
/// levels over its own limit to 0.55, which is over half of what separated us from the geometry.
/// The page states **65 859 strokes thinner than a device pixel**, every one of them round-capped
/// and 91.8% of them shorter than one device pixel — median length 0.145, and every one of them
/// 0.1366 of a device pixel wide (`pdf-model/examples/sub_pixel_width_census`) — and ADR 0268 draws
/// such a rule as its swept body one device pixel wide with the width it gave up in the paint's
/// alpha. **It reads 7.681 since the four-hundred-and-fifty-fifth**, which drew the caps ADR 0268
/// butt-capped away: a cap's area is a *square* of the width, so it is stated at the substitute's
/// width with `(w / W)²` of the alpha (ADR 0290). Those caps are 1.14 levels of the page's own
/// geometry and 0.133 of a level landed, because at that width each cap's ink is about half a level
/// of 255 on each of a few pixels and half a level is what an eight-bit raster rounds away.
/// Our own ladder at 8× is **6.934 before and after both changes**, which is the check rather than
/// a spare number: at that scale the strokes are no longer sub-pixel and neither rule may touch
/// them. The page therefore sits 0.75 of a level above its own geometry where it sat 0.61, and
/// every reference sits 3 to 7 levels above it, which is this group's whole subject.
///
/// `doc/todo/00`'s step-7 sweep reads it the other way and its own note already says why: this page
/// went −1.712 → **−2.956** of our ink minus the lightest reference's, because moving toward a
/// geometry every reference overpaints is moving away from all of them. ADR 0290 moves it back to
/// −2.823 for the same reason in reverse.
///
/// ## What the references are doing on that page, settled in the five-hundred-and-eighty-fourth
///
/// The picture is what makes this page worth a session rather than a row: its two ECG panels are a
/// **ghost** in our render and dark in all four references, and the side-by-side says so at a
/// glance. ADR 0419 asked which reading of the standard that is, and the answer came off a ladder
/// rather than off any of them — one 160-unit rule per page at seventeen widths, ink against the
/// geometry's own `1.02 × w`, at 72 dpi and again at 576:
///
/// ```text
///    width   geometry      ours   poppler     mupdf        gs     hayro
///   0.1366     0.1393     0.136      1.02     0.204     0.272     1.024
///     0.05     0.0510     0.048      1.02     0.204     0.272     1.024
///    0.001     0.0010         0      0.128     0.204     0.272     1.024
/// ```
///
/// Each reference floors a sub-pixel rule at a **device-pixel** width — the same numbers at 8× —
/// and **no two of the four floors agree**: `poppler` and `hayro` at 1.0 device pixels, `mupdf` at
/// 0.2, `ghostscript` at 0.27. Ours is the straight line through the origin at both resolutions.
/// Ask the three C references for the clause's *own* algorithm instead — `pdftoppm -aa no`,
/// `gs -dGraphicsAlphaBits=1`, `mutool draw -A 0` — and the disagreement vanishes: **1.02 apiece at
/// every sub-pixel width**, one whole device pixel, which is §10.7.4 read literally. They agree
/// about the clause and part the moment anti-aliasing is switched on, which is exactly the region
/// §10.7.1's NOTE hands to the implementation.
///
/// §10.7.5 states the floor they draw — "[i]f stroke adjustment is enabled and the requested line
/// width, transformed into device space, is less than half a pixel, the stroke shall be rendered as
/// a single-pixel line" — and Table 52 initialises that parameter to `false`. **`issue12295.pdf`
/// states no `/SA`, no `/ExtGState` and no `/GS` at all**, so the clause that would darken its
/// traces is not asked for. `hayro`'s source is the one on this disk and it says the quiet part:
/// "Best-effort attempt to ensure a line width of at least 1.0, as required by the PDF
/// specification" — with a different threshold for text and the rule disabled inside patterns and
/// Type 3 glyphs, three exceptions no clause states.
///
/// So the page stays here, with the citation beside it rather than a target: the group's subject is
/// every renderer painting more than the geometry, and this one is the extreme of it because its
/// marks are the thinnest.
/// # A fourth, in the two-hundred-and-sixty-eighth, and the tightest limit the bucket has
///
/// `issue12963.pdf` page 7 is page 8's neighbour and says the same thing more exactly: `poppler`
/// descends onto **9.5574** and `mupdf` onto **9.5584** — one thousandth of a level apart, which
/// is two independent programs agreeing about a geometry rather than two programs being close —
/// and ours at 8× is 9.5344, 0.023 under. At the page's own scale ours is 9.510 and `poppler`
/// 9.888, a third of a level *over* its own limit. Same document, same finding, four figures.
/// # And the other four pages of that document, in the three-hundredth, which finish it
///
/// `doc/todo/00`'s own instruction is to check what else on the list is the same file before
/// taking a name off it, and here it was worth four: pages 2, 3, 4 and 5 are the same Russian tax
/// form as 7 and 8, and they say the same thing to the same number of figures.
///
/// ```text
///          poppler 72   poppler 576   mupdf 576    apart     ours 1x   ours - limit
/// page 2     10.9983      10.7058      10.7062    0.0004     10.647      -0.059
/// page 3     11.9148      11.5641      11.5645    0.0004     11.5037     -0.061
/// page 4      9.6651       9.3382       9.3375    0.0007      9.2901     -0.048
/// page 5     10.9246      10.6455      10.6461    0.0006     10.6236     -0.022
/// ```
///
/// **Two independent programs agreeing about a geometry to 0.0004 of 255 is the tightest limit
/// this bucket has measured**, four figures better than page 7's thousandth, and it is measured
/// four times over on one document. At the page's own scale `poppler` is 0.28 to 0.35 *over* its
/// own limit and ours is 0.02 to 0.06 under it — a form of five-point type and comb cells is
/// thin marks all the way down, which is this group's standing subject.
///
/// The verdict is `ambiguous` rather than agreeing because `ghostscript` draws the same form a
/// fifth of a level lighter still and its comb cells come out dotted; the picture is five
/// renderings of one page that a person would call identical.
///
/// # What ADR 0735 has to do with `issue12295.pdf`, which is nothing
///
/// The eight-hundred-and-second session drew strokes coloured by a tiling pattern for the first
/// time, and the nineteenth sweep reads it as a decision taken about a page this note argues —
/// correctly, because ADR 0735 names `issue12295.pdf` once, in the `doc/todo/00` step-7 line
/// recording that the page did not move. **It could not have moved.** Expanded with `qpdf --qdf
/// --object-streams=disable`, the document states `/Pattern` zero times and `SCN` zero times, so
/// neither arm of `Interpreter::tile` is reached on it at all; what it states is the 65 859
/// sub-pixel strokes above, in a flat colour. The gap reproduces at −2.362 and the verdict is
/// unchanged. ADR 0738.
const AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY: [&str; 8] = [
    "issue12963.pdf page 2",
    "issue12963.pdf page 3",
    "issue12963.pdf page 4",
    "issue12963.pdf page 5",
    "issue12963.pdf page 7",
    "issue12963.pdf page 8",
    "two_pages.pdf page 2",
    "issue12295.pdf page 1",
];

/// Ambiguous, three more, and the third is where only one ladder can be climbed.
///
/// ```text
///                             limit             ours     hayro    poppler   mupdf    gs
/// tiling_patterns_variations  23.953 / 23.977  23.897   25.196   25.241   23.968   25.076
/// issue16224                  12.662 / 12.676  12.656   12.483   12.467   12.517   12.659
/// issue12337                       — / 16.225  16.030   14.578   16.114   16.010   16.251
/// ```
///
/// `tiling_patterns_variations.pdf` is 600×800 of §8.7.3 cells and ours is 0.06 under the
/// geometry with `mupdf` 0.01 over it, while `poppler`, `hayro` and `ghostscript` are 1.1 to 1.3
/// above — the same shape `issue2177.pdf` has one entry over, and on a page whose whole subject
/// is tiling.
///
/// `issue16224.pdf` is 183×33 and ours is 0.01 from the limit and nearest of the five, with the
/// rest 0.01 to 0.21 under it.
///
/// **And it is the one page of `rank_the_pages_we_are_alone_on`'s head where the divisor is trap
/// 9's tenth mechanism outright.** The page is one line of type in an embedded Type 1C subset of
/// `MyriadPro-Regular`, and the three voting references do not divide three ways over it —
/// `examples/compare_rasters` per pair, which is a different instrument from the gate's own line
/// and prints two more decimals than it, so `--bin quoted` names every row of this table:
///
/// ```text
/// poppler vs mupdf        mean  2.0394   ssim 0.98174     0.41 bounds
/// poppler vs ghostscript  mean 10.0894   ssim 0.68934     3.11 bounds
/// mupdf   vs ghostscript  mean  9.9475   ssim 0.69494     3.05 bounds
/// ours    vs mupdf        mean  5.3409   ssim 0.88671     1.13 bounds
/// ```
///
/// The two that share `libfreetype.so.6` are **seven and a half times** closer to each other than
/// either is to the `ghostscript` that links its own copy, and we sit less than half as far from
/// `mupdf` as `ghostscript` does. The ratio that ranks this page at 2.78× is 1.13 over that 0.41:
/// *we* are the second-closest thing on the page to the reference pair, and the page rises because
/// the pair is one glyph rasteriser (ADR 0663).
///
/// **Those two numbers are two different measures, and the table above is where it can be seen**
/// (ADR 0688). Against this page's text bounds — mean 5.00, worst tile 40.00, similarity 0.9000 —
/// our 1.13 is the **structural similarity** against `mupdf`, 0.88671, while the pair's 0.41 is
/// their **mean**, 2.0394, their own similarity being 0.98174 and therefore 0.18 of the bound. So
/// the printed 2.78× divides a similarity by a mean. Read like for like the trap-9 reading holds
/// either way and is *stronger* on the measure our number is: 1.13 over 0.18 is **6.2×** on the
/// similarity, and 1.07 over 0.41 is 2.6× on the mean. The sharing pair is the closest pair on
/// both, so nothing here turns on which measure is asked — which is worth saying plainly, because
/// one group over it does: on `freeculture.pdf` page 322 our own **mean** is 0.95 of its bound,
/// *inside* it, and only the similarity puts that page on this list at all.
///
/// **`issue12337.pdf` is the case where step 6 has one ladder and not two**: `pdftoppm` at 576
/// dpi produces no image for it at all, so only `mupdf`'s 16.225 is available, and a single
/// ladder cannot tell convergence from drift. What can be said without it is that four of the
/// five renderers are inside 0.24 of each other at the page's own scale and `hayro` is 1.4
/// below all of them, which makes `hayro` the one to explain rather than us. Listed on that
/// basis, which is weaker than the other two and is said so.
///
/// # And `issue12337.pdf` had a second answer the ladder could not reach, which is the ink table's
///
/// The ink above is a *page* number and this page's whole disagreement is a *place*. It carries one
/// markup annotation with no `/AP` —
///
/// ```text
/// /Subtype /Highlight  /Rect [48.75 300 297 443.25]  /C [1 1 0]  /CA 1  /F 4
/// /QuadPoints [48.75 443.25 297 443.25 48.75 300 297 300]
/// ```
///
/// — one quadrilateral, identical to the rectangle. §12.5.6.10 states the region and nothing about
/// the marks in it, which is [`AMBIGUOUS_MARKUP_ARTWORK`]'s subject, and the standard states the
/// region here **twice**: Table 166 makes `/Rect` the annotation rectangle, "defining the location
/// of the annotation on the page", and §12.5.6.10 makes `/QuadPoints` the quadrilateral
/// the marks encompass, and on this file the two are the same rectangle. What the standard states
/// about marks outside it is nothing at all — the nearest sentence is §12.5.5's, of the appearance
/// a file supplies rather than one a processor constructs:
///
/// > Each appearance stream is a form XObject (see 8.10, "Form XObjects"): a self-contained
/// > content stream that shall be rendered inside the annotation rectangle.
///
/// So the honest statement is that a supplied appearance could not put ink where these three put
/// it, and that where the file supplies none this tree constructs the appearance the two stated
/// regions describe and no larger — a documented choice, on the side the one relevant `shall`
/// points.
///
/// The rectangle is device columns 48.75 to 297.0 at 72 dpi. Yellow pixels, counted off each
/// renderer's own panel:
///
/// ```text
/// ours          x  49 .. 296     inside, flush with both edges
/// poppler       x  22 .. 323     27 columns left of it, 26 right
/// mupdf         x  23 .. 321     26 left, 24 right
/// ghostscript   x  31 .. 314     18 left, 17 right
/// hayro         no yellow at all
/// ```
///
/// All four that draw it agree about the rows — 349 to 491, inside the rectangle — and bulge only
/// sideways, so this is three renderers rounding the *ends* of a highlight outwards by three
/// different amounts and one drawing none. **Ours is the only one of the five inside the
/// rectangle**, and the page's worst tile, at (288, 416), is exactly where our yellow stops and
/// theirs continues.
///
/// The ranking is why this was looked at, and removing the annotation is what priced it —
/// `/Annots 998 0 R` replaced by `/Annots []` in place, same byte length, all four re-run:
///
/// ```text
///                       with the highlight   without it
/// ours, nearest reference      1.117 (gs)      0.613 (mupdf)
/// closest pair                 0.881           0.891
/// ```
///
/// Our number is the annotation and the denominator is not: **without it the page is not on that
/// list at all**, because 0.613 is inside 0.891. So `rank_the_pages_we_are_alone_on` is right that
/// we are alone here and the reason is that we are the only renderer obeying the sentence above
/// (ADR 0663).
const AMBIGUOUS_ONE_LADDER: [&str; 3] = [
    "tiling_patterns_variations.pdf page 1",
    "issue16224.pdf page 1",
    "issue12337.pdf page 1",
];

/// Ambiguous, and four renderers give four answers about one text field.
///
/// `bug1844583.pdf` is 178×54 and holds one widget containing *Hello World*. The four other
/// renderers disagree about two separate things, neither of which is the text:
///
/// ```text
/// ours 11.21   hayro 11.51   mupdf 13.52   ghostscript 15.76   poppler 4.44
/// ```
///
/// `poppler` draws **no border at all**, which is the whole of its 4.44; `mupdf` draws a heavier
/// one than ours; and `ghostscript` draws the string **backwards** — `DlrowOlleh`. Step 6 cannot
/// arbitrate: `poppler` converges on 4.475 and `mupdf` on 13.523, a factor of three apart, which
/// is two renderers drawing different pictures rather than one grid resolving.
///
/// So the page is here rather than diagnosed against a limit, and what is worth recording is
/// that the *spread* is about §12.5.4's border and §9.7's writing direction and not about
/// anything this tree does with the value. `AMBIGUOUS_WIDGET_BORDER` and
/// `AMBIGUOUS_CONSTRUCTED_WIDGET` are the two neighbours; this one is listed beside them because
/// no single clause settles it and a fifth renderer would probably give a fifth answer.
///
/// `tagged_stamp.pdf` is here for the opposite reason: 612×792 and almost blank, ink 0.639
/// against a two-ladder limit of 0.6406 and 0.6390 — **all five renderers inside 0.02 of 255 of
/// each other**, and ours a thousandth from the geometry. A page with almost no ink is a page
/// where any bound is tight in relative terms, which is the other way to reach this bucket.
///
/// **And `issue6081.pdf` page 1 is that at twenty times the extreme**, off the ranking in the
/// three-hundred-and-fiftieth session. 612 × 792, 109 commands, and the *whole page's ink is
/// 0.03 of 255* — all of it in rows 205 to 220. The five span 0.014:
///
/// ```text
/// ours 0.029635 │ mupdf 0.030897 │ poppler 0.039181 │ hayro 0.042880 │ ghostscript 0.043455
/// ```
///
/// and the worst single row differs by **4 of 255**. Step 6's two ladders end at 0.029228 and
/// 0.028583 — **0.00065 of 255 apart** — with ours at 0.027989, 0.0006 under the lower. In
/// relative terms ours is 4% light and in absolute terms it is six ten-thousandths of a level,
/// which is the whole reason this page reaches a bucket at all: `Interpretation::glyphs` earns it
/// no tolerance and a similarity measure over a page that is 99.99% white has almost nothing to
/// be similar about.
const AMBIGUOUS_FOUR_ANSWERS: [&str; 3] = [
    "bug1844583.pdf page 1",
    "tagged_stamp.pdf page 1",
    "issue6081.pdf page 1",
];

/// Ambiguous, and the whole disagreement is the row or column at a shape's boundary.
///
/// Two pages the three-hundred-and-forty-third session took off §3a's ranking, and the per-row
/// ink profile named the same thing on both: a shape whose edge falls between pixels, and five
/// renderers deciding differently whether the pixels it half covers are painted.
///
/// # `bug_jpx.pdf`, where the arithmetic is exact
///
/// 612 × 792, and everything on it is one 128 × 64 grey JPEG 2000 image drawn at 1:1 — the whole
/// page is blank but rows 200 to 264. The interior is uniform, and the profile says the four
/// renderers agree about it to two thousandths of a level: over the image's own rows the mean is
/// **201.665 for ours and 201.667 for `ghostscript`**, 201.250 for `mupdf` and `hayro`. The name
/// is a red herring: nothing here is about the codestream.
///
/// What differs is the boundary. Per column, at the image's left and right edges:
///
/// ```text
/// column         268     269     270 …   396     397     398
/// ours            0      64     255      255     192      0
/// mupdf           0     255     255      255     255      0
/// hayro           0     255     255      255     255      0
/// ghostscript     0       0     255      255     255      0
/// ```
///
/// **64/255 + 127 + 192/255 = 128.004**, which is the image's width to four decimal places, and
/// the rows do the same thing with 64. So ours covers exactly the rectangle the page states, with
/// two partly covered columns at its edges; `mupdf` and `hayro` round outwards to 129 × 65 and
/// `ghostscript` to 128 × 64 at another phase. The ink follows arithmetically: 129 × 65 ÷
/// (128 × 64) = 1.0236, and 4.3099 × 1.0236 = **4.4116** against the 4.4113 both of them print.
/// `ghostscript` is 4.30976 against our 4.30990.
///
/// # `tensor-allflags-withfunction.pdf`, where five renderers split three ways
///
/// A §8.7.4.5.8 tensor-product mesh on a 612 × 792 page. Two rows of the profile are singular and
/// nothing else on the page is:
///
/// ```text
/// row 203    ours   0   poppler 100   mupdf   0   ghostscript 100   hayro   0
/// row 492    ours   0   poppler   0   mupdf   0   ghostscript  86   hayro  86
/// ```
///
/// The mesh's top edge is drawn by `poppler` and `ghostscript` and not by the other three; its
/// bottom edge by `ghostscript` and `hayro` and not by the other three. Every other row of 792 is
/// within one level of 255 across all five. Removing those two rows takes the page's spread from
/// **0.343 of 255 to 0.135** — they are 61% of the whole disagreement.
///
/// And step 6 says ours is the one measuring the area:
///
/// ```text
///                72 dpi    576 dpi
/// poppler       26.5570   26.3894
/// mupdf         26.4176   26.4130
/// ours (1x/8x)  26.3708   26.3707
/// ```
///
/// **Two ladders 0.024 of 255 apart at the limit**, ours flat to a ten-thousandth across an
/// eightfold change and 0.018 under `poppler`'s limit. A renderer whose ladder does not move is
/// one already measuring the geometry (`doc/todo/00` step 6), and a page where the references
/// disagree about a boundary row by more than any of them disagrees with us about everything else
/// is `ambiguous` by the verdict's own definition.
///
/// **Why these are not `AMBIGUOUS_IMAGE_REDUCTION`'s**, though the sentence rhymes: that group is
/// about a reduction, where §10.7.4's area averaging is the departure ADR 0025 records. Neither of
/// these is reduced — the image is at 1:1 and the mesh is a shading — so what is being decided is
/// only the edge, and no clause decides it. §10.7.4 says a shape covering part of a pixel is the
/// device's business and §8.5.3.3.1 calls the degenerate case "device-dependent and not generally
/// useful"; three answers among five renderers is what a clause that decides nothing produces.
/// # And its sibling, which is the same two rows
///
/// `coons-allflags-withfunction.pdf` page 1 is §8.7.4.5.7's Coons patch mesh where the other is
/// §8.7.4.5.8's tensor-product one, and its profile is the same page's:
///
/// ```text
/// row 203    ours   0   poppler 100   mupdf   0   ghostscript 100   hayro   0
/// row 492    ours   0   poppler   0   mupdf   0   ghostscript  86   hayro  86
/// ```
///
/// The same two rows, the same three-way split, the same values. `doc/todo/00`'s "check what else
/// on the list is the same file" paying its seventh time — with the caution that goes with it,
/// since `issue840.pdf`'s two pages were a colour and an edge respectively. Here the profile is
/// what says they are the same rather than the name.
///
/// **The tensor page's pixels moved in the eight-hundred-and-fifty-first session and these two
/// rows did not** (ADR 0778). §8.7.4.5.7's fold-over precedence was ranking `u` above `v` where the
/// clause ranks `v` above `u`, and over `doc/pdf.js/test/pdfs` — `examples/raster_digest`, 975
/// documents — correcting it moves the pixels of exactly one page, this file's. Rows 203 and 492
/// were re-measured off the corrected render and are still 0 ink and 0 non-white pixels on both
/// files, which is what this note is about: the split is over the mesh's top and bottom *edge*
/// rows, and a precedence between two preimages of one interior point cannot reach an edge.
const AMBIGUOUS_BOUNDARY_PIXELS: [&str; 3] = [
    "bug_jpx.pdf page 1",
    "tensor-allflags-withfunction.pdf page 1",
    "coons-allflags-withfunction.pdf page 1",
];
/// Ambiguous, and the five renderers agree to a fortieth of a level.
///
/// Two pages the three-hundred-and-fifty-third session took off §3a's ranking, both for the same
/// reason: the verdict is `ambiguous` because no *pair* of references is close enough to form a
/// consensus, and the reason no pair is close enough is that **all five are already within a
/// rounding error of each other**. A bound that is a fraction of a level is a bound nobody meets.
///
/// # `issue4246.pdf`, where our eight-times render *is* a reference's limit
///
/// 595 × 842 and **one command**, which `doc/todo/00`'s step 4 says means one image — a 50 × 40
/// indexed image at 7 ppi with a 1000 × 800 stencil mask over it, magnified across most of the
/// page.
///
/// ```text
/// ours 5.86948 │ ghostscript 5.86801 │ hayro 5.86707 │ mupdf 5.87978 │ poppler 5.89171
/// ```
///
/// **The five span 0.0246 of 255**, and step 6 closes it further:
///
/// ```text
///                72 dpi     576 dpi
/// poppler       5.89171    5.86779
/// mupdf         5.87978    5.86170
/// ours (1x/8x)  5.86948    5.86779
/// ```
///
/// **Ours at eight times equals `poppler`'s limit to seven significant figures** — 5.86779 both —
/// and `mupdf`'s is 0.006 below. There is nothing on this page for anybody to be wrong about, and
/// the entry exists because a page can sit on §3a's ranking for that reason alone.
///
/// # `bug1872721.pdf`, which is 88 × 31
///
/// Ten commands on a page the size of a postage stamp, and the five span 0.42 of 255 with ours in
/// the middle. Three ladders all *climb* and end within **0.028 of 255** of each other — `poppler`
/// 12.2616, `mupdf` 12.2763, ours 12.2897 — which on a page of 2728 pixels is a handful of
/// partly-covered ones.
///
/// **And the renderers do not agree about the raster's own size**: 87 × 31 for ours and
/// `ghostscript`, 88 × 31 for `poppler` and `mupdf`, 87 × 30 for `hayro`. On a page this small a
/// column is more than 1% of it, which is `doc/todo/00`'s `magick identify` rule at the scale
/// where it bites hardest — and the reason the numbers above are means rather than a pairwise
/// metric.
/// # And `issue15150.pdf`, which is **10 × 10**
///
/// A hundred pixels, and the five span 1.06 of 255 — which on a page this size is less than half
/// of one pixel's worth of ink, since a single fully covered pixel is 2.55 of the mean.
///
/// ```text
///                72 dpi     576 dpi
/// poppler       1.30699    0.517679
/// mupdf         0.440913   0.501932
/// ours (1x/8x)  0.684989   0.501932
/// ```
///
/// **Ours at eight times equals `mupdf`'s limit exactly** — 0.501932 both — and `poppler`'s is
/// 0.0158 above it. There is no bound a page of a hundred pixels can meet and no disagreement
/// underneath this one.
const AMBIGUOUS_INSIDE_A_ROUNDING_ERROR: [&str; 3] = [
    "issue4246.pdf page 1",
    "bug1872721.pdf page 1",
    "issue15150.pdf page 1",
];

/// Ambiguous, and it took a scope decision rather than a fix — §10.5's transfer function.
///
/// `issue6931_reduced.pdf` page 1 says, in words, *The color should be red*, and it is the entry
/// that proves this bucket's whole premise: a page can be plainly wrong inside an `ambiguous`
/// verdict with nothing announcing it. It is not on the ranking — 0.35 from the nearest reference
/// — and it came off `doc/todo/00`'s **step 7** at **+17.26**, our ink minus the lightest live
/// reference's.
///
/// ```text
/// ours 20.3861 │ mupdf 20.6726 │ poppler 3.12869 │ ghostscript 3.70614 │ hayro 3.67272
/// ```
///
/// The image's samples are near black — ours reads `[2, 2, 2]` and `pdfimages`, *poppler's own*
/// extractor, writes a PNG of `srgb(2,2,2)` — and the `/ExtGState` in force when it is drawn sets
/// `/TR` to three type-0 sampled functions that map **2/255 to 0.992**. Three renderers apply the
/// function and show a red heart on white; ours and `mupdf` do not and show a black square.
///
/// **It was drawn wrong until the three-hundred-and-fifty-eighth session**, and what took the
/// round was the argument rather than the code: implementing §10.5 crossed `CLAUDE.md`'s own scope
/// sentence, which called transfer functions inapplicable. The standard settles it — §10.1's list
/// of rendering steps makes halftoning conditional on the device and the transfer function not,
/// and §10.6.1 keeps the transfer for a device that needs no halftone — so the project owner split
/// the scope line and the clause was implemented. Ours is **3.61878** now:
///
/// ```text
/// ours 3.61878 │ poppler 3.12869 │ ghostscript 3.70614 │ hayro 3.67272 │ mupdf 20.6726
/// ```
///
/// **`mupdf` is now the only renderer of the five that does not apply it**, which is what turns
/// this from a page we were wrong about into a page that stays `ambiguous` because one reference
/// draws something else. The remaining spread among the four is 0.58 of 255 on a page whose ink is
/// 3.6, which is the ordinary difference between five CMYK-to-RGB conversions of one photograph.
const AMBIGUOUS_TRANSFER_FUNCTION_UNAPPLIED: [&str; 1] = ["issue6931_reduced.pdf page 1"];

/// Ambiguous, and step 6's two ladders bracket ours inside a fifth of a level.
///
/// Three pages the three-hundred-and-fifty-sixth session took off §3a's ranking. Each has two
/// clusters at the page's own scale — which is the shape that looks like a defect — and on each
/// the ladders say the clusters are one renderer's first rung rather than two readings of the
/// file.
///
/// ```text
///                                72 dpi     576 dpi
/// images_1bit_grayscale  poppler 13.5551   13.3466
///                        mupdf   13.5348   13.3913
///                        ours    13.3326   13.3466
///
/// decodeACSuccessive     poppler  6.58517   6.42541
///                        mupdf    6.46243   6.40702
///                        ours     6.39351   6.40981
///
/// ccitt_EndOfBlock_false poppler 86.3513   85.7700
///                        mupdf   85.4370   85.5363
///                        ours    85.5844   85.6742
/// ```
///
/// - **`images_1bit_grayscale.pdf`**: ours at eight times **equals `poppler`'s limit to six
///   figures** — 13.3466 both — and at the page's own scale ours is byte-for-byte
///   `ghostscript`'s (13.3550 each).
/// - **`decodeACSuccessive.pdf`** is a progressive JPEG's AC successive approximation, and the
///   three ladders end within **0.018 of 255** with ours between the other two. At 72 dpi ours and
///   `hayro` are equal to five decimals.
/// - **`ccitt_EndOfBlock_false.pdf`** is §7.4.6's `/EndOfBlock false`, and ours lands **between**
///   the two limits — 0.096 under `poppler`'s and 0.138 over `mupdf`'s, which are 0.234 apart. At
///   72 dpi ours is byte-for-byte `ghostscript`'s again (85.7282 each).
///
/// The panels are not all one size on two of the three — 595 × 842, 596 × 842 and 595 × 841 — so
/// the numbers above are means, for `doc/todo/00`'s reason.
const AMBIGUOUS_OURS_ON_THE_LIMIT: [&str; 3] = [
    "images_1bit_grayscale.pdf page 1",
    "decodeACSuccessive.pdf page 1",
    "ccitt_EndOfBlock_false.pdf page 1",
];

/// Ambiguous, and §11.4.6's knockout groups are the whole of it.
///
/// `knockout_groups_test.pdf` page 1 is four labelled tests in a 2 × 2 grid — non-isolated
/// knockout, isolated knockout, non-isolated normal, isolated normal — which makes the page its
/// own instrument: measure each quadrant and the disagreement names itself.
///
/// ```text
///                       ours   poppler   mupdf   ghostscript   hayro
/// non-isolated knockout 72.36    74.40   71.89       71.21      78.56
/// isolated knockout     60.16    65.93   59.60       59.00      66.01
/// non-isolated normal   76.87    77.68   77.46       77.99      77.89
/// isolated normal       64.54    65.16   64.91       65.32      65.30
/// ```
///
/// **The two `normal` quadrants agree across all five within 1.1 and 0.8 of 255.** The two
/// `knockout` quadrants split them into two groups that are **5.8 to 6.4 apart**: ours with
/// `mupdf` and `ghostscript` (within 0.47 and 1.15 of us), against `poppler` and `hayro`. Three
/// renderers give one answer to §11.4.6 and two give another, which is a clause with two readings
/// rather than a page with a defect.
///
/// **And this page reports nothing**, which used to be worth saying because five of the corpus's
/// documents reported exactly this clause. `doc/todo/23`'s first population was "a knockout element
/// whose shape is not its coverage" and it is **closed** as of ADR 0234, so of the pages this
/// paragraph named, none still reports: `knockout_blend_multiply.pdf` and
/// `knockout_inner_backdrop.pdf` kept theirs for §11.4.4's non-isolated residue rather than for a
/// shape until the four-hundred-and-seventy-second, which read §11.4.6 for *which* backdrop it
/// hands each element and found neither page needed a construction — one is §11.4.4's group
/// wearing `/K`, the other is §11.4.5's under NOTE 6 (ADR 0307). Page 2 of this file
/// agrees with the consensus now. Page 1's knockout groups were always ones this tree composites
/// without reaching that condition, so the page is judged rather than excused, and **the numbers
/// above are unmoved to a hundredth**: it is the group the five-renderer split is about, and no
/// element of it states a shape.
const AMBIGUOUS_KNOCKOUT_GROUP: [&str; 1] = ["knockout_groups_test.pdf page 1"];

/// Ambiguous, and the whole of the difference is four horizontal rules.
///
/// `issue9972-1.pdf`, `-2` and `-3` are **one page under three names**: the same catalog, the
/// same readback md5 (`0444a129…`), and the same ink in all five renderers to the fourth
/// decimal. A *Personal Sleep Study* form — A4 of six- to nine-point Helvetica with two ruled
/// tables at the foot — and the three files differ in what their pdf.js issue was about rather
/// than in what a page draws.
///
/// At the page's own scale:
///
/// ```text
/// ours 16.7900   poppler 16.5226   mupdf 16.7102   hayro 17.0341   ghostscript 18.2760
/// ```
///
/// Ours is 0.08 from `mupdf` and 0.24 from `hayro`, inside a band the four span and 1.49 under
/// `ghostscript`. What makes this diagnosable rather than another dense-text page is that the
/// difference is **localised**, and one instrument says where.
///
/// # Where: a per-row ink profile at eight times
///
/// Ours against `mupdf`, row by row over the 5880-row raster, worst twelve rows:
///
/// ```text
/// 1x row 560.9   ours  91   mupdf  53   poppler   7
/// 1x row 561.0   ours  72   mupdf  34   poppler   5
/// 1x row 589.8   ours 134   mupdf  97   poppler  30
/// 1x row 589.9   ours 112   mupdf  68   poppler  17
/// 1x row 669.9   ours  85   mupdf  50   poppler   7
/// 1x row 697.9   ours  93   mupdf  56   poppler  15
/// ```
///
/// **Every one of the twelve is within a fifth of a pixel of one of the page's four horizontal
/// table rules** — 1× rows 561, 590, 670 and 698 — and the ordering at each is ours > `mupdf` >
/// `poppler`. Nothing else on the page reaches the list. The prose half of the page settles it
/// from the other side: over the top 55% the three ladders agree to **0.02 of 255** —
///
/// ```text
///              1x / 72 dpi    8x / 576 dpi
/// ours            11.8885        11.8832
/// poppler         11.8324        11.9073
/// mupdf           11.7970        11.9023
/// ```
///
/// — so the text is not in question at all, and four rules are.
///
/// # Why that is `ambiguous` and not a defect
///
/// The same row profiles, as distances over the whole page:
///
/// ```text
/// ours    vs mupdf     0.6862
/// mupdf   vs poppler   1.2978
/// ours    vs poppler   1.5850
/// ```
///
/// **We are nearer `mupdf` than the two references are to each other**, by a factor of two. Three
/// renderers put a hairline rule's edge in three places and no clause states which: §8.4.3.2's
/// "thinnest line" and §10.7.5's stroke adjustment are permissions, and `doc/todo/_scan-conversion`
/// holds what this tree departs from and why.
///
/// **And it explains the whole-page ladders**, which is what the ink table alone could not.
/// Ours is flat — 16.79 at 1×, 16.84 at 4×, 16.83 at 8× — while `poppler` descends 16.52 → 16.28
/// and `mupdf` 16.71 → 16.43. A flat ladder is a renderer already measuring the geometry; the
/// references' descent is their rule edges narrowing as the pixels shrink, which is the same
/// quantity the row profile above measures directly.
///
/// **A caution this page earned the hard way.** A band of rows chosen by eye to be "rules only"
/// caught the bottom of a caption as well, and produced a 14% over-paint that is not there.
/// `doc/todo/00`'s step 5's rule about `-alpha off` has a sibling: **a band is a hypothesis about
/// what is in it**, and the row profile is what checks it — 13 rows at 1× is 6 of text, 1 of rule
/// and 6 of white, and only the profile says so.
const AMBIGUOUS_TABLE_RULE_EDGES: [&str; 3] = [
    "issue9972-1.pdf page 1",
    "issue9972-2.pdf page 1",
    "issue9972-3.pdf page 1",
];

/// Ambiguous, and on all three ours is the render nearest the geometry.
///
/// Three pages of small marks — `issue7339_reduced.pdf` at 115×220, `issue21570.pdf` at 842×595
/// and `personwithdog.pdf` at 612×792 — where the five renderers spread by more than a bound and
/// none of them is drawing anything the others are not. Step 6's closed form, taken with two
/// ladders at 576 dpi:
///
/// ```text
///                        limit           ours     hayro    poppler   mupdf    ghostscript
/// issue7339_reduced   11.636 / 11.561   11.554   13.181   13.082   12.024   12.904
/// issue21570          12.562 / 12.547   12.554   12.565   12.683   12.547   12.852
/// personwithdog       21.991 / 21.101   21.620   20.901   22.114   20.911   21.532
/// ```
///
/// **On the first two ours is inside a hundredth of the limit and every other renderer is above
/// it**, by up to 1.6 of 255 — §10.7.4 as written applied to marks a fraction of a pixel wide,
/// which is this group's standing subject and ADR 0025's documented departure from the other
/// side.
///
/// **The third is a result about the instrument rather than about the page, and it is the first
/// of its kind.** `poppler` and `mupdf` at 576 dpi are 0.89 apart, where on the other two they
/// agree to a hundredth — so there is no limit to take, and the two-ladder rule the
/// two-hundred-and-sixteenth session added is doing exactly what it was added for: one ladder
/// cannot tell convergence from drift, and two say when neither has converged. Ours sits in the
/// middle of the five at 21.62 and nothing here can rank them. That is a page for a heatmap and
/// a later session, and it is listed rather than explained.
///
/// **And "a later session" came in the four-hundred-and-fifteenth, which answered it without a
/// heatmap.** `personwithdog.pdf`'s page dictionary states
/// `/Group << /S /Transparency /CS /DeviceCMYK >>`, and §11.4.7 makes that the default blending
/// colour space for everything on the page. This tree composited in the device's three
/// components, so the page reported it by name and left this group; the row above stands as what
/// the ladders said while it was here, and the two references being 0.89 apart where they agree
/// to a hundredth elsewhere reads differently once the page is known to be printed in ink.
///
/// **The four-hundred-and-twenty-sixth drew it in ink and the page is back**, with the same
/// verdict and a ladder that moved by exactly the amount ADR 0262 predicts: ours is **21.620 →
/// 21.720** at the page's own scale and 21.722 → 21.822 at 576 dpi, +0.100 of 255 at both, and
/// the direction is the one ADR 0251 derived — half a covering of ink over ink is darker than
/// the average of the two colours it converts to. Still inside the 21.991 / 21.101 bracket and
/// still in the middle of the five, so nothing about the ranking changed; what changed is that
/// the page is no longer reported, which is what puts it back in front of this gate.
/// # A fourth in the three-hundred-and-fifth, where ours lands *between* the two limits
///
/// `pdfjs_wikipedia.pdf` page 1 — a Wikipedia article printed to PDF, all text and rules:
///
/// ```text
///                 72 dpi    576 dpi
/// poppler        20.8373   20.7291
/// mupdf          20.6976   20.7084
/// ours (1x, 8x)  20.6243   20.7117
/// ```
///
/// The two ladders bracket the geometry within **0.021 of 255** and ours at eight times sits
/// between them, 0.017 from one and 0.003 from the other — the tightest a renderer can be to a
/// limit, which is inside it. `ghostscript` is **20.62 against 23.93**, 3.2 of 255 above all four
/// of the others at the page's own scale, and it is the whole reason the verdict is `ambiguous`:
/// trap 12's arithmetic with one reference far enough out to drag the consensus apart.
const AMBIGUOUS_NEAREST_THE_GEOMETRY: [&str; 4] = [
    "pdfjs_wikipedia.pdf page 1",
    "issue7339_reduced.pdf page 1",
    "issue21570.pdf page 1",
    "personwithdog.pdf page 1",
];

/// Ambiguous, and the word spaces **were** drawn as marks — fixed in the two-hundred-and-
/// thirty-ninth session.
///
/// `issue7074_reduced.pdf` reads *Our 2015 Graduates* in an embedded `CIDFontType2` subset of
/// Arial Bold under `Identity-H`. Three references and `hayro` drew it with spaces between the
/// words; we drew `Our|2015|Graduates` — a narrow mark where each space belongs.
///
/// # What it was, and it is one clause past ADR 0170
///
/// This font is one of the six in the corpus whose `loca` offsets do not ascend, and the
/// two-hundred-and-twenty-third session's repair rebuilds such a `glyf` in glyph order by
/// reading each entry's length *from its own bytes*. Its table begins
///
/// ```text
/// 0  108  0  108  108  282  0  282  962 …
/// ```
///
/// and glyph 3 — the space — has start 108 with a successor of 108. The glyph table's own
/// standard writes a glyph with **no outline** by repeating the offset, and that statement is
/// self-consistent whatever the rest of the table does; the repair read the entry at 108
/// instead, which is glyph 4. So the space was given a real glyph, by the very code that exists
/// to give a glyph back its own bytes.
///
/// **The `loca` repair was checked against this document in the session that landed it and
/// cleared**: `AMBIGUOUS_SPACE_DRAWN_AS_A_MARK` recorded that switching the repair off left the
/// page's ink at 19.576 either way. That measurement was right and the conclusion drawn from it
/// was not — the page's ink is dominated by three words of bold text, and five narrow bars are
/// under a tenth of a level of it. **A page-level number cannot clear a mechanism of a defect
/// that is five glyphs wide.**
///
/// ```text
///              ours before   ours after   hayro   mupdf   poppler   ghostscript   limit
/// at 72 dpi       20.83        19.59      19.52   19.15    19.09       18.06     19.751/19.755
/// ```
///
/// Ours at 8× is **19.749**, which is the two-ladder limit to three figures; before the fix we
/// were the only renderer above it. The page stays `ambiguous` because five rasterisers still
/// disagree about bold glyph coverage at nine points, which is
/// `AMBIGUOUS_GLYPH_SCAN_CONVERSION`'s subject and §10.7.4's last sentence.
const AMBIGUOUS_SPACE_DRAWN_AS_A_MARK: [&str; 1] = ["issue7074_reduced.pdf page 1"];

/// Ambiguous, and **we were the ones who were wrong**: half the sentence was missing.
///
/// `issue11131_reduced.pdf` is 207×41 and draws *Operating Account Consolidated Statement* in an
/// embedded `CIDFontType2` subset under `Identity-H`. We draw about half of it —
/// `p  r ti g   ou t o soli t  St t     t` — and say nothing, because `FontError` is per font
/// and this font loaded and produced glyphs.
///
/// ```text
/// ink   ours 3.29   hayro 3.25   mupdf 7.92   poppler 7.91   ghostscript 9.51
/// ```
///
/// **The three renderers that draw the whole sentence are the three that share `libfreetype`**,
/// and the two that do not are the two that read the font with `skrifa` — trap 9's third shape
/// with us in the minority, and the clause is what settles it rather than the vote. The font's
/// `loca` states 72 long offsets for 71 glyphs, which is the shape ISO/IEC 14496-22 requires,
/// and its contents begin `16776 16776 16776 16776 10674 2188 2590 1886`: the offsets are
/// **not ascending**, so 36 of the 71 glyphs have a negative stated length. `read-fonts` refuses
/// those and `FreeType` derives each entry's extent from the entry itself.
///
/// **Fixed in the two-hundred-and-twenty-third session** (ADR 0170): `repaired_loca_order`
/// rebuilds `glyf` in glyph order with a new monotonic `loca` beside it, which is exact because
/// a `glyf` entry is self-describing. The whole sentence draws. Six of the corpus's 623 embedded
/// TrueType programs have such a table.
///
/// The page stays here because the verdict is still `ambiguous` — `ghostscript` is 1.6 of 255
/// away from `poppler` and `mupdf` on a page this small, so there is no consensus to contradict
/// either way — and the entry stays because **a page can be plainly wrong inside this bucket and
/// nothing announces either the defect or the fix**. That is the whole argument for §3a, and
/// this page is the eleventh time it has paid.
const AMBIGUOUS_LOCA_OUT_OF_ORDER: [&str; 1] = ["issue11131_reduced.pdf page 1"];

/// Ambiguous, because one reference drew a blank page and the other three agree with us.
///
/// `issue6006.pdf` is 100×100 and is one §8.7.4.5.4 radial shading — red at the rim, white, blue
/// at the centre. It sat at 1.52 from the nearest reference and **78.43 from the furthest**,
/// which is the whole diagnosis in two numbers: `ghostscript` renders it as white paper, ink
/// **0.00**, and a renderer that drew nothing is 78 bounds away from everybody.
///
/// The other four:
///
/// ```text
/// ink        ours 126.13   hayro 126.11   poppler 126.18   mupdf 126.58   ghostscript 0.00
/// vs ours (MAE)           hayro 3.2      poppler 651      mupdf 522      ghostscript 26876
/// ```
///
/// and the geometry, from two ladders at 576 dpi, is `poppler` 126.134 and `mupdf` 126.386 —
/// **ours is 0.13 from it**. §8.7.4.5.4 states the construction exactly and four of the five
/// renderers here execute it; the fifth is trap 9's "an unimplemented feature has a default"
/// with the default being *nothing*, which is the shape that makes a page ambiguous rather than
/// contradicted because there is no consensus left to contradict.
///
/// **Not to be confused with `AMBIGUOUS_RADIAL_CONE`**, which was a radial defect this tree did have until ADR 0171: that
/// is §8.7.4.5.4's greatest *admissible* root on a cone whose circles do not contain one
/// another, and this page's circles are concentric, where every gradient implementation agrees.
/// # And one where the reference drew *something*, which is harder to see
///
/// `bug920426.pdf` page 1 is 200 × 50 and shows two words — *Checkliste Service* — in a bold sans
/// face. Four of the five renderers draw them:
///
/// ```text
/// ours 25.4879 │ hayro 25.4815 │ mupdf 25.7350 │ ghostscript 26.5349 │ poppler 4.4150
/// ```
///
/// **Ours and `hayro` agree to 0.006 of 255** and the four span 1.05. `poppler` draws fourteen
/// **`.notdef` boxes** — the four-panel strip says it without a number — so its 4.415 is the ink
/// of an outline per character instead of a glyph per character, and the 21 of 255 between it and
/// everyone else is a reference failing to map a code rather than anything about this page.
///
/// It came off `doc/todo/00`'s **step 7** rather than off the ranking, in the
/// three-hundred-and-forty-eighth session, and that is the entry worth keeping: the step's
/// *positive* side is "content nobody else is drawing", and taking the **minimum** over live
/// references makes one outlier the whole comparison. Here that outlier is the finding. The
/// ranking had it at 0.35 from the nearest and 2.62 from the furthest, which accuses nobody.
/// # And three where the reference that drew nothing is `mupdf`, on a function it does not evaluate
///
/// `colorspace_sin.pdf`, `colorspace_cos.pdf` and `colorspace_atan.pdf` are 276 × 276 and each
/// paints one `/DeviceN` whose tint transform is a §7.10.5 **type 4 PostScript calculator
/// function** — `{ pop exch 360 mul sin 2 add 4 div sub abs .1 exch sub … }`.
///
/// ```text
///                    ours      hayro   ghostscript    poppler    mupdf
/// sin and cos     29.1376    29.1376       29.1382    29.5397     0.00
/// atan            28.0044    28.0044       28.0044    28.3870     0.00
/// ```
///
/// **Ours and `hayro` are equal to four decimal places on all three**, `ghostscript` is within
/// 0.0006, `poppler` is 0.39 above, and `mupdf` renders white paper. Four renderers agreeing to a
/// ten-thousandth of a level while the fifth draws nothing is trap 9's "an unimplemented feature
/// has a default" with the default being *nothing*, exactly as `issue6006.pdf` above.
///
/// Step 6 puts ours on the geometry: `poppler` descends 29.5397 → 29.1529 and **ours is flat at
/// 29.1376** across an eightfold change, 0.015 under its limit.
///
/// **`colorspace_sin` and `colorspace_cos` render byte-identically** — the same md5 for our own
/// panel — so they are one page under two names, which is `doc/todo/00`'s own check and the reason
/// their numbers agree to the digit rather than merely closely.
/// # And a sixth, where the blank one is `hayro` and a fifth renderer is halfway
///
/// `issue2840.pdf` page 1 is 200 × 50. `hayro` draws **nothing**; `ghostscript` draws 13.68 where
/// ours is 21.32; `poppler` is 19.91; `mupdf` is 21.30. Ours and `mupdf` agree to **0.03 of 255**
/// at the page's own scale and to **0.0025** at the limit — 21.4514 against 21.4489 — while
/// `poppler`'s ladder is flat at 19.90, a level and a half below both.
///
/// So this page carries two of this file's shapes at once: a reference that drew nothing, and a
/// second that draws something else. Neither is a consensus, which is why the verdict is
/// `ambiguous` rather than contradicted, and the pair that agrees to two thousandths of a level
/// includes us.
/// # And a seventh, which arrived from the contradicted list rather than from a ranking
///
/// `issue11740_reduced.pdf` page 1 is 200 x 50 and shows *Оглавление*. It was
/// [`CONTRADICTED_REFERENCES_DREW_NOTHING`] from the sixth session until the
/// six-hundred-and-eighty-first, on a consensus of `poppler` and `ghostscript` — and
/// `ghostscript` returned **white paper**, 1.000 of full white, having reported *error reading
/// a stream* under a `-q` this gate passes and then loaded a substitute face it drew nothing
/// with. Under ADR 0513 that raster abstains, and what is left is `poppler` at 0.982 — the "one
/// blob glyph" the paragraph above names — against `mupdf` at 0.928. They do not agree with
/// each other, so the page is ambiguous rather than contradicted, which is what a page with one
/// failure and two different pictures on it always was.
///
/// We, `mupdf` and `hayro` draw the word; ADR 0049 is where that came from, and the group's own
/// shape is the reason it is filed here: a reference that drew nothing, and a second that drew
/// something else.
const AMBIGUOUS_REFERENCE_DREW_NOTHING: [&str; 7] = [
    "issue6006.pdf page 1",
    "bug920426.pdf page 1",
    "colorspace_sin.pdf page 1",
    "colorspace_cos.pdf page 1",
    "colorspace_atan.pdf page 1",
    "issue2840.pdf page 1",
    "issue11740_reduced.pdf page 1",
];

/// Ambiguous, and the file declares a glyph bounding box that encloses nothing.
///
/// `issue14953.pdf` page 1 came off `doc/todo/00`'s ranking in the three-hundred-and-seventy-second
/// session at 0.28 from the nearest reference and 3.64 from the furthest. It is 200 × 50 device
/// pixels: fifteen codes shown at 20 points from an embedded Type 3 font — ConTeXt's
/// `ContextRuntimeFont-1`, `/FontMatrix [0.04 0 0 0.04 0 0]` — whose glyph descriptions draw a
/// handwriting face out of `1 w` and `2 w` strokes, 0.8 and 1.6 device units at the page's own
/// scale.
///
/// **Every one of the fifteen descriptions begins `wx 0 0 0 0 0 d1`**, and the font dictionary's
/// `/FontBBox` is `[0 0 0 0]` beside it. So the file declares, sixteen times over, a box that
/// encloses none of the marks it then makes.
///
/// # What the clause determines: that nothing here is determined
///
/// §9.6.4 Table 111, on `d1`:
///
/// > The declared bounding box shall be correct - in other words, sufficiently large to enclose
/// > the entire glyph. If any marks fall outside this bounding box, the result is
/// > implementation-dependent.
///
/// and Table 110 on the font's own box, which this file also zeroes:
///
/// > If all four elements of the rectangle are zero, a PDF processor shall make no assumptions
/// > about glyph sizes based on the font bounding box.
///
/// This is `doc/todo/00`'s third shape — the clause puts the answer beyond itself and says so —
/// and it is the sharpest instance of it in the bucket, because the standard names the very
/// situation the file is in and hands the outcome to the implementation in the same sentence. Five
/// renderers may therefore draw five things and none of them can be called wrong. **What this tree
/// chooses is to ignore the declared box**, which `content.rs`'s `d0 | d1` arm has said in a
/// comment since the tenth session: clipping to a box the clause requires to be correct can only
/// remove marks a correct file does not have, and on an incorrect one it hides the defect instead
/// of drawing what the producer drew.
///
/// # What the five actually do, and one ladder that is not a limit
///
/// ```text
///               72 dpi      288       576      1152
/// poppler      13.6783    3.9541    1.4760    0.2577
/// mupdf        11.9546   12.1528   12.2232   12.2611
/// ours         11.6675   12.2806   12.2958   12.2939     (1x, 4x, 8x, 16x)
/// ghostscript   0.2962
/// hayro        11.9743
/// ```
///
/// Ours is flat from 8× and `mupdf` climbs onto it, the two **0.033 of 255 apart at the top rung
/// and still closing**; `poppler` descends towards *nothing*, which is drift rather than
/// convergence and is the two-hundred-and-sixteenth session's lesson met a second time. Its 576
/// dpi panel says what the number cannot: it draws the first few hundredths of an inch of each
/// stroke and stops.
///
/// # The mechanism, isolated by a file this project wrote
///
/// Two synthetic 100 × 40 pages, one Type 3 glyph apiece, one `1 w` stroke from (2, 0) to (14, 20)
/// in glyph space, **identical in every byte but the four bounding-box operands of `d1`** —
/// `1 -1 15 21` against `0 0 0 0`, degenerate before the slash:
///
/// ```text
///                72 dpi           288             576            1152
/// ours       1.6935/1.6935   1.9758/1.9758   1.9637/1.9637   1.9694/1.9694
/// mupdf      2.0338/2.0338   1.9681/1.9681   1.9617/1.9617   1.9674/1.9674
/// poppler    2.3290/2.3290   1.0216/2.0470   0.3900/2.0057   0.0528/1.9920
/// ghostscr.  0.1275/2.0400   0.0000/2.8767   0.0000/2.3847   0.0000/2.1934
/// ```
///
/// **Ours and `mupdf` are byte-identical across the pair at all four rungs** — `magick compare
/// -metric AE` exactly 0, eight times — so neither reads the box at all. `ghostscript` loses the
/// glyph at every rung and draws *exactly nothing* above 72 dpi. `poppler` is byte-identical at 72
/// dpi and then diverges as the pixels shrink (AE 271, 1 648, 7 820), which is precisely the shape
/// of its corpus ladder above. Both are clipping the marks to the declared box; one does it in
/// device space with enough slack that a small page hides it.
///
/// That leaves what separates us from the *nearest* reference, and it is this file's other
/// subject: a 0.8-device-unit stroke. Ours and `mupdf` agree to **0.002 of 255** at the synthetic
/// page's limit (1.9694 against 1.9674) and to 0.033 on the corpus page's, which is
/// `AMBIGUOUS_SUB_PIXEL_LINE_WORK`'s sentence and not a second finding.
const AMBIGUOUS_DEGENERATE_GLYPH_BOX: [&str; 1] = ["issue14953.pdf page 1"];

/// Ambiguous, and every glyph procedure on the page paints itself white before the clause
/// takes the colour away from it.
///
/// `issue12705.pdf` page 1 came off `doc/todo/00`'s ranking in the three-hundred-and-seventy-ninth
/// session at **0.18 from the nearest reference and 1.00 from the furthest**. It is 250 × 50,
/// twenty-four commands, and shows one line of Hebrew and digits at 14 points from an embedded
/// Type 3 font with `/FontMatrix [0.001 0 0 0.001 0 0]`, a nondegenerate `/FontBBox
/// [-43 -206 926 817]` and 114 `/CharProcs`.
///
/// # What the clause determines, outright, and the page is the witness
///
/// **All 114 descriptions begin with `d1`, and 111 of them — every one that paints — then state
/// `0 0 0 RG`, `1 1 1 rg`, `1 w`, `[] 0 d` and fill a path with `f`.** The page's own content
/// stream sets no colour at all, so the current colour is §8.6.4.2's initial `DeviceGray` black.
/// §9.6.4 Table 111, on `d1`:
///
/// > A glyph description that begins with the d1 operator should not execute any operators that
/// > set the colour (or other colour-related parameters including transparency) in the graphics
/// > state; any use of such operators shall be ignored and the glyph stream continues to be
/// > processed without error
///
/// > The glyph description is executed solely to determine the glyph's shape. Its colour shall be
/// > determined by the graphics state in effect each time this glyph is painted by a text-showing
/// > operator.
///
/// This is `doc/todo/00`'s **first** shape — the clause determines it and we can be checked
/// against it — and the check is the strongest kind, because the two readings are not near each
/// other: a processor that honoured `1 1 1 rg` would paint white on white and **the page would be
/// blank**. All five renderers draw the line, so all five apply the rule, and this tree applies it
/// in `content.rs`'s `d0 | d1` arm, which raises the same `uncoloured` flag §8.6.8 names for an
/// uncoloured tiling pattern.
///
/// # The three descriptions that mark nothing declare a box that could enclose nothing
///
/// `/iSQP000A`, `/iSQP000D` and `/iSQP0020` — LF, CR and space — are `260 0 1000 1000 0 0 d1` and
/// `313 0 1000 1000 0 0 d1`: `ll` at (1000, 1000) and `ur` at (0, 0), a rectangle **inverted on
/// both axes**. `pdftoppm` says so out loud, *eleven* times, which is exactly the number of spaces
/// in the string the page shows. It costs nothing here because those three descriptions make no
/// marks, so Table 111's "[i]f any marks fall outside this bounding box, the result is
/// implementation-dependent" has no marks to be about — but it is `AMBIGUOUS_DEGENERATE_GLYPH_BOX`
/// one file over, and the reason this tree never clips to the declared box is the same reason
/// there.
///
/// # What separates the five, measured
///
/// ```text
///                72 dpi      288       576      1152
/// poppler       11.60030  11.30750  11.12960  11.07050
/// mupdf         10.98710  11.01290  11.01570  11.00830
/// ghostscript   13.54060  11.67690  11.78160  11.64680
/// ours          11.05520  10.98010  10.99680  10.99130   (1x, 4x, 8x, 16x)
/// ```
///
/// `poppler` descends and `mupdf` climbs, **bracketing the geometry from opposite sides and
/// ending 0.062 of 255 apart**; ours is flat from 4× and lands 0.017 under the lower of the two.
/// At the page's own scale ours is 0.064 from its own limit where `poppler` is 0.53 above its own.
///
/// **`ghostscript` is the one ladder that does not converge on the geometry**, and that is a
/// different finding from the 1× excess the other rungs explain away. It sits 0.66 to 0.78 of 255
/// above ours at 4×, 8× *and* 16× alike — 6.0% to 7.1% of the page's ink at every rung — where a
/// difference in scan conversion falls with the pixels, as its own 2.49 at 72 dpi does. Divide the
/// excess by the ink a one-pixel erosion of our own raster removes, which is an outward offset
/// measured in device pixels:
///
/// ```text
///          excess   erosion   offset (px)   offset (pt)
/// 4x       0.6968   4.33173      0.161         0.0402
/// 8x       0.7848   2.21220      0.355         0.0443
/// 16x      0.6555   1.11707      0.587         0.0367
/// ```
///
/// The offset **triples in device pixels and holds at 0.040 ± 0.004 points**, so what
/// `ghostscript` fills is the glyph procedure's path outset by a constant amount in *user* space —
/// about three glyph units at this font's 14-point size — rather than the path §9.6.4 says the
/// description describes. Nothing in Table 111 or in §10.7.4 licenses that, and the other four
/// renderers do not do it, so the finding is `ghostscript`'s and not this page's. (The erosion is
/// a coarse perimeter: it removes a thin feature entirely, which can only *over*-state the
/// perimeter and so *under*-state the offset, and the rung most exposed to that is the 4× one that
/// already agrees.)
///
/// The pairwise matrix at the page's own scale puts ours against `hayro` smallest of the ten pairs
/// and ours against `mupdf` third; the two largest all involve `poppler`, which is the reference
/// with the most to descend.
const AMBIGUOUS_UNCOLOURED_GLYPH_PROCEDURE: [&str; 1] = ["issue12705.pdf page 1"];

/// Ambiguous, because one reference refused an embedded font program and substituted a face.
///
/// `bug1308536.pdf` page 1 came off `doc/todo/00`'s ranking in the three-hundred-and-seventy-ninth
/// session at **0.35 from the nearest reference and 3.79 from the furthest**, where it had been
/// the head of that list. It is 240 × 50, forty-five commands, and sets one line of French at
/// 20 points from `BAFOKE+UltraCondensedSansTwo` — a `/Subtype /Type1C` program in `/FontFile3`,
/// with `/Widths` for every code and `/StemV 45`, so its stems are 0.9 of a device pixel at the
/// page's own scale.
///
/// # The reference says what it did, and the file says why
///
/// `gs` without `-q` prints two lines: *An embedded font is invalid*, and *Loading font
/// BAFOKE+UltraCondensedSansTwo (or substitute) from …/NimbusSans-Regular*. The four-panel strip
/// agrees — four renderers draw an ultra-condensed small-capital face and `ghostscript` draws a
/// normal-width sans in mixed case, at the same advances.
///
/// **What is invalid is the Private DICT and nothing else.** Decompressed, the program's 77-byte
/// Private DICT opens with six real-number operands the format cannot mean — `-4E-0349998`, three
/// empty ones whose first nibble is the terminator, `56E-483978` — separated by an operator 3 that
/// no Private DICT defines. Its last 26 bytes then parse cleanly: `BlueValues`, `BlueScale`,
/// `StdHW 21`, `StdVW 45`, `StemSnapH`, `StemSnapV`, `defaultWidthX 251`, `nominalWidthX 223`.
///
/// # What the specification determines
///
/// §9.9's Table 124, of a `/FontFile3` whose `/Subtype` is `Type1C`:
///
/// > The font program provided as the value of this key shall conform to Adobe Technical Note
/// > #5176.
///
/// So the *file* is in breach, and the standard states no consequence: what a processor does with
/// a program that does not conform is nowhere in clause 9. §9.8.1 names the route `ghostscript`
/// took, as a capability rather than an obligation — "[t]hese font metrics provide information
/// that enables a PDF processor to synthesise a substitute font or select a similar font when the
/// font program is unavailable" — and a program that is present but malformed is a judgement call
/// about the word *unavailable* that no clause makes for anybody.
///
/// **This tree reads it, and the reason is checkable rather than lucky**: the corrupt operands are
/// the *hinting* parameters, the outlines are in the CharStrings INDEX and parse without
/// complaint, and §9.2.4's advances come from the font dictionary's `/Widths` rather than from the
/// Private DICT's two width defaults. So nothing that decides a mark is in the broken bytes, which
/// is what four renderers agreeing to 0.015 of 255 at the limit then demonstrates:
///
/// ```text
///                72 dpi      288       576      1152
/// poppler       16.09120  16.13680  16.16340  16.16310
/// mupdf         16.23580  16.19410  16.18850  16.17800
/// ghostscript   11.47960  11.44280  11.40380  11.42440
/// ours          16.65250  16.14460  16.12970  16.17510   (1x, 4x, 8x, 16x)
/// ```
///
/// **Three ladders end within 0.015 of 255** — 16.1631, 16.1780, 16.1751 — and ours is between the
/// other two. `ghostscript`'s is flat 4.75 *below* all of them at every rung, which is what a
/// different face looks like and not what scan conversion looks like: there is no limit for it to
/// climb onto because it is not drawing the same shapes. The pairwise matrix says it again — every
/// pair involving `ghostscript` lands between 5 189 and 5 355 where the largest of the six that do
/// not is 1 073 and the smallest is 451.
///
/// Ours at the page's own scale is 0.48 over its own limit where `poppler` is 0.07 under and
/// `mupdf` 0.06 over, and that residual is this file's *other* subject rather than this group's:
/// `/StemV 45` at 20 points is a stem 0.9 of a device pixel wide, which is
/// `AMBIGUOUS_SUB_PIXEL_LINE_WORK`'s sentence about a mark thinner than the raster's quantum.
///
/// **Not `AMBIGUOUS_REFERENCE_DREW_NOTHING`**, which is the same shape with the substitution
/// missing: there the fifth renderer's ink is 0.00 and here it is a whole page drawn in the wrong
/// face. And **not `AMBIGUOUS_SUBSTITUTED_FACE`**, which is §9.8.1 reached the way the clause
/// intends — a font nobody embedded, where *every* processor must choose. Here four processors
/// had the producer's own outlines and used them.
const AMBIGUOUS_REFUSED_EMBEDDED_FONT: [&str; 1] = ["bug1308536.pdf page 1"];

/// Ambiguous, and both pages are gradients on a page too small for a bound to be loose.
///
/// `issue4706.pdf` is an A4 page of vector artwork and `issue18529.pdf` is 65×50 with a red
/// gradient frame. Neither is a defect this gate can name, and the two of them together say what
/// the *bound* is doing, which is why they share an entry.
///
/// Step 6's closed form, taken with two ladders because one cannot tell convergence from drift
/// (the two-hundred-and-sixteenth session's lesson):
///
/// ```text
///                    limit    ours     hayro    poppler   mupdf    ghostscript
/// issue4706.pdf      8.66     8.664    8.664    8.746     8.635    8.624
/// issue18529.pdf    23.21    21.874   21.933   23.166    23.391   24.226
/// ```
///
/// **On `issue4706.pdf` ours is 0.004 from the geometry and byte-identical to `hayro`** — MAE
/// exactly 0 over 595×842, where `ghostscript` is 23 away, `mupdf` 140 and `poppler` 273. Two
/// renderers sharing no rasteriser (`hayro` draws through `vello_cpu`, this tree through
/// `tiny-skia`) producing the same bytes is what an axis-aligned page with no anti-aliased edge
/// looks like from the inside, and the page is ambiguous only because a vector page's bound is
/// 0.99 similarity and the three C renderers spread 0.12 of ink between them.
///
/// **On `issue18529.pdf` ours and `hayro` are both 5.8% under the limit and the three C
/// renderers are on it**, which is a real difference and a small one: 1.3 of 255 on a page that
/// is one gradient. It is named rather than diagnosed — what separates two implementations of
/// §8.7.4.5.3's ramp on a 65×50 raster is a question worth a session of its own, and the two
/// renderers on one side of it are the two that do not share `libfreetype`, `lcms` or anything
/// else.
const AMBIGUOUS_GRADIENT_ON_A_TIGHT_BOUND: [&str; 2] =
    ["issue4706.pdf page 1", "issue18529.pdf page 1"];

/// Ambiguous, and eighty-two per cent of the page is within *one level* of `poppler`.
///
/// `issue7821.pdf` is a 166×55 "APPROVED" stamp whose whole area is one §8.7.4.5.3 axial
/// shading between two very pale greens — `/C0 [.812 .878 .776]`, `/C1 [.949 .969 .922]` — under
/// a type 3 stitching function. It sat at the *top* of §3a's ranking for four sessions at 5.44
/// and the two-hundred-and-fifth session fixed what was there: §8.7.2 places a pattern in the
/// space of the content stream that names it, and an annotation's appearance is a form, so the
/// axis had been landing off the page and `/Extend` painting one flat colour (ADR 0160).
///
/// **What is left is arithmetic no clause reaches.** Per-pixel, against our render:
///
/// ```text
/// identical or one level of 255:   poppler 81.8%   ghostscript 80.6%
///                                  mupdf   31.8%   hayro       30.6%
/// ```
///
/// and the split is not about the shading at all — `mupdf` rasterises this page 167×55 and
/// `hayro` 166×54, so the two that disagree are the two whose grid is offset from ours, which
/// on a ramp that changes by a level every few pixels is the whole difference. Ink: ours 45.69,
/// `poppler` 46.69, `mupdf` 47.08, `hayro` 48.33, `ghostscript` 50.00, against `poppler`'s own
/// high-resolution limit of 46.10 — ours and `poppler` bracket it within 0.6.
///
/// Every pairwise heatmap is diagonal stripes at right angles to `/Coords`, which is the
/// signature: §8.7.4.5.3 defines the colour at every point of the axis exactly and says nothing
/// about what an eight-bit target does with it, so where two renderers round the same ramp in
/// opposite directions a band one level wide appears. That is §3a's third shape — the clause
/// determines the value and puts the device's quantisation beyond itself — and the floor is one
/// level, which is what these two numbers are.
///
/// What is *not* quantisation is the remaining 18%: the glyph edges of the word and the rounded
/// border, differing by up to 86 against `poppler` and 179 against `ghostscript`, which is
/// `AMBIGUOUS_GLYPH_COVERAGE`'s subject on a page that also has a gradient.
/// # And a page that is nothing but two ramps, in the three-hundred-and-sixth
///
/// `bug852992_reduced.pdf` page 1 is a green-to-white gradient with an orange-to-red one over it
/// inside a blue border, and the five renderings are indistinguishable to a person:
///
/// ```text
/// ours 75.5062 │ ghostscript 75.2559 │ hayro 75.7016 │ poppler 75.8339 │ mupdf 75.9012
/// ```
///
/// **Our ladder is flat** — 75.5062 at the page's own scale against 75.4903 at eight times — so
/// whatever separates the five is not scan conversion; and the two reference ladders are 0.25
/// apart from each other (`poppler` flat on 75.8307, `mupdf` climbing to 76.085), so there is no
/// exact limit to be near. A page whose whole ink is two ramps, where nobody converges and the
/// spread is 0.64 of 255, is the eight-bit ramp this group is about.
const AMBIGUOUS_GRADIENT_QUANTISATION: [&str; 2] =
    ["bug852992_reduced.pdf page 1", "issue7821.pdf page 1"];

/// Ambiguous, and it is the glyph-rasterisation floor with the geometry to settle it.
///
/// `copy_paste_ligatures.pdf` is 143×15 device pixels: one line of text with ligatures, and
/// nothing else. It sat at 2.81 bounds from the nearest reference on a page where a single glyph
/// is 2% of the ink.
///
/// Step 6's closed form says who is measuring the outlines. `poppler` at 72, 576 and 2304 dpi
/// gives 40.81, 43.17, **43.26**, so the glyphs cover 43.3 of 255. At the page's own scale:
///
/// ```text
/// ours 43.32   hayro 43.55   poppler 40.81   mupdf 40.88   ghostscript 61.38
///              └ the limit is 43.26
/// ```
///
/// **Ours is on the geometry to three figures.** `poppler` and `mupdf` are 5.7% under it at 72
/// dpi and converge on it by 576 — which is hinting, a thing that exists to make small text
/// legible and by construction moves ink; `ghostscript` is 42% over, which is §10.7.4 as written
/// applied to stems a fraction of a pixel wide. The verdict is `ambiguous` because five
/// renderers disagree by more than any bound can call, on a page of fifteen rows.
/// # Two more, both of them one word on a page the size of a postage stamp
///
/// `endchar.pdf` is 40×50 device pixels and draws a single `É` from an embedded `/FontFile3`
/// whose name is its own hypothesis — the CFF `endchar` operator's four-argument form, which
/// composes an accented character out of two glyphs. **All five renderers compose it**, so the
/// hypothesis is answered and what is left is one outline's edges. `poppler` at 2304 dpi gives
/// 60.98 and at 72 gives 59.06; ours 59.39, `ghostscript` 59.66, `mupdf` 58.16, `hayro` 62.84 —
/// five renderers spanning 4.7 levels about a limit of 61.0, on a page where one glyph is all
/// the ink there is.
///
/// **That limit stood on one ladder for four hundred sessions, and it has four now** — which is
/// `doc/todo/02` §7's rule, because one ladder cannot tell convergence from drift and this group's
/// own `issue2177.pdf` paragraph is where that was learned. Ink as `(1 − mean) × 255` after
/// `-alpha off -channel R -colorspace Gray`, each renderer on its own uncropped raster
/// (`examples/render_at` for ours, `mutool draw -r`, `pdftoppm -cropbox -r`, `gs -dUseCropBox
/// -r`), at one, four, eight and thirty-two times the page's own scale:
///
/// ```text
///                   72       288       576      2304
/// ours          59.4874   59.8367   61.1486   60.9729
/// mupdf         58.1554   59.9419   60.6850   60.9314
/// poppler       59.0589   60.3054   60.8458   60.9757
/// ghostscript   59.6630   61.4630   60.9818   61.0843
/// ```
///
/// **Four independent ladders land within 0.153 of 255 of each other**, ours between `mupdf`'s and
/// `poppler`'s, so the geometry is 60.93 to 61.08 and every one of the four converges on it —
/// including this tree's. What is left at 72 dpi is a spread of 1.51 of 255 over a raster of
/// **15 × 34 device pixels**, which is this group's sentence rather than a defect: the coverage is
/// agreed and what differs is where the covered pixels are.
///
/// **And on this page the ladder is the right instrument, which is a thing to check rather than to
/// assume** (ADR 0688). `endchar.pdf` is eleventh on [`rank_the_pages_we_are_alone_on`] at 2.36×,
/// marked `[widened: outside]`, and the gate names both halves: ours is the **mean against
/// `mupdf`**, 1.97, and the divisor is the **mean between `poppler` and `ghostscript`**, 0.83. Like
/// for like, so this page's number may be read as the ratio it looks like — one of the three rows
/// of that list's head whose numerator is a mean at all, the others being
/// `copy_paste_ligatures.pdf` below and `AMBIGUOUS_OVERSIZED_BORDER`'s page.
/// `examples/compare_rasters` over the gate's own panels, that
/// example's figures and one named pair
/// per row (ADR 0663), against text bounds of mean 5.00, worst tile 40.00, similarity 0.9000:
///
/// ```text
///                           mean      ssim     bounds
/// ours vs mupdf           9.8343   0.92555     1.97  ← the numerator, on the mean
/// ours vs poppler        11.6544   0.90148     2.33
/// ours vs ghostscript    15.0172   0.85351     3.00
/// poppler vs ghostscript  4.1686   0.98289     0.83  ← the divisor, on the mean
/// poppler vs mupdf        4.9809   0.98269     1.00
/// mupdf vs ghostscript    9.0015   0.94456     1.80
/// ```
///
/// **Our similarity against `mupdf` is 0.92555, which is 0.74 of the bound and therefore inside
/// it**: the accented glyph is in the same place in both rasters and differently covered, which is
/// the group's own sentence read off the measure that is failing rather than off the one that is
/// not.
///
/// **One caution the table earns and the ladder cannot state.** A ladder's ink is a *signed page
/// total* and the gate's mean is a *per-pixel absolute difference*, and the two are not the same
/// statistic even though both are means: at 72 dpi the ink puts `ghostscript` 0.18 from us and
/// `mupdf` 1.33, while the gate's mean puts `mupdf` nearest at 9.83 and `ghostscript` furthest at
/// 15.02. On a raster of 15 × 34 two renderers can cover the same total with different pixels, and
/// which is nearer then depends on which question is asked. Both answers are above, and neither is
/// a correction of the other.
///
/// **Three of the four 72-dpi figures above this table reproduce and the fourth is ours**:
/// `poppler` 59.0589 against a recorded 59.06, `ghostscript` 59.6630 against 59.66, `mupdf`
/// 58.1554 against 58.16, and *ours* **59.4874 against a recorded 59.39**. So the number that moved
/// in the rounds between is this tree's own, by 0.10 of 255 on a fifteen-column raster, while every
/// reference is where it was — which is the only direction of that comparison worth anything, and
/// is why a note's figures are re-measured rather than cited (ADR 0663).
///
/// `issue16316.pdf` is 60×10 device pixels: the word *Experimentation* in an embedded
/// `NimbusRomNo9L`. Its crop box is **59.813 × 9.375 points**, which has no whole-pixel answer
/// at all, and the five renderers give it three different rasters — 60×10 (ours, `poppler`,
/// `mupdf`), 60×9 (`ghostscript`) and 59×9 (`hayro`). `CLAUDE.md` names this among the places
/// the standard defines nothing: "how a fractional page becomes a whole number of pixels" is a
/// documented choice, and ours is `TargetSpec::for_page`'s rounding *up* so that the raster
/// contains the page (ADR 0064).
///
/// Among the three that agree about the raster, step 6's closed form settles the rest:
/// `poppler` at 576 and 2304 dpi gives 43.81 and 43.79, so the outlines cover **43.8**, and at
/// the page's own scale ours is 42.02 against `mupdf`'s 41.09 and `poppler`'s 41.03. Ours is
/// nearest the geometry and the two `libfreetype` references are 6% under it, which is hinting
/// — the same result `copy_paste_ligatures.pdf` gives above, on a page of ten rows.
///
/// # Everything above this line spent sessions attached to a different group
///
/// The paragraphs above are this group's diagnosis and they were **not above this group**: an
/// edit that inserted a new `const` between a doc comment and the const it documented welded
/// two notes into one and left this array bare. A reader of `AMBIGUOUS_OUTLINED_TEXT` got four
/// pages' worth of argument for a group of one, and `AMBIGUOUS_GLYPH_COVERAGE` — three pages —
/// said nothing at all. `doc/todo/00`'s "what a group must say" forbids a group that names no
/// clause, and a group that says nothing whatever is the same failure with nothing left to
/// correct.
///
/// **It had happened three times**, and the other two were `AMBIGUOUS_MASKED_BLUR` and
/// `AMBIGUOUS_OURS_ON_THE_LIMIT`. All three are put back where they belong, and
/// [`every_group_of_pages_carries_a_diagnosis_naming_one_of_them`] is what makes the next one
/// fail the build instead of being found by eye.
///
/// # Re-measured, because a note nothing checks is a claim rather than a measurement
///
/// The three pages are the smallest rasters in the bucket: **143 × 14**, **15 × 34** and
/// **60 × 9** device pixels. On a page nine rows tall a glyph's edge is not a detail of the
/// page, it *is* the page — so a half-pixel of phase between two rasterisers moves a fifth of
/// every pixel, and every whole-page measure reads it as a large difference.
///
/// Ink as `(1 − mean) × 255` over each renderer's own uncropped raster, at the page's own
/// scale and at four and eight times it (`doc/todo/00` steps 5b and 6):
///
/// ```text
/// copy_paste_ligatures.pdf   72 dpi   288 dpi   576 dpi
///   ours (1x/4x/8x)          40.431    43.496    43.126
///   poppler                  40.807    43.164    43.174
///   mupdf                    40.879    43.190    43.191
///   ghostscript              61.383    44.995    43.221
///
/// issue16316.pdf
///   ours (1x/4x/8x)          42.023    43.467    43.532
///   poppler                  41.030    43.138    43.813
///   mupdf                    41.085    43.252    43.836
///   ghostscript              45.882    43.467    43.970
/// ```
///
/// **Two reference ladders converge to 0.017 of 255 of each other on the first page** and to
/// 0.023 on the second, and ours ends 0.05 and 0.28 under them. On `issue16316.pdf` ours at
/// 72 dpi is **1.51 under its own limit where `poppler` is 2.78 and `mupdf` 2.75 under
/// theirs**, which is the nearest of the four.
///
/// **Our 40.43 here and the 43.32 four paragraphs up are the same render**, and the difference
/// is `doc/todo/00` step 3's: that figure came off `<stem>-p1-ours.png`, which is our raster
/// after `normalise::to_common_size` cropped its last row away, and dropping a blank row from a
/// fifteen-row page raises the mean by a fifteenth. The ladder above is `examples/render_at`'s
/// output, uncropped, which is the only place our own page size can be read.
///
/// So the *coverage* is agreed — the five renderers' inks sit within 0.6 of 255 on
/// `copy_paste_ligatures.pdf` once `ghostscript` is set aside, and within 1.9 on `endchar.pdf`
/// — and the group's name is a claim the measurement does not support. What differs is
/// **where** the covered pixels are, which is §10.7.4's closing sentence:
///
/// > Scan conversion of character glyphs may be performed by a different algorithm from the
/// > preceding one.
///
/// That is `AMBIGUOUS_GLYPH_SCAN_CONVERSION`'s clause. The group is kept separate rather than
/// folded into it because its distinguishing property is a fact about the *raster* rather than
/// about the glyphs: these are the pages where that clause's licence is worth the whole page,
/// and they are the ones a ranking by our own distance keeps offering.
///
/// # And `ghostscript` on `copy_paste_ligatures.pdf` is a renderer rather than a page
///
/// Its 72 dpi ink is **61.38 against four renderers within 0.56 of each other** and its own
/// 576 dpi value is 43.22, so it descends 18 levels onto the same geometry as everybody else:
/// its excess at the page's own scale is scan conversion of an embedded `TimesNewRomanPSMT`
/// subset and not a substituted face, which the four-panel strip invites and the ladder
/// refuses. Nothing is reported and no warning is printed.
///
/// # Which bound `copy_paste_ligatures.pdf` is alone on, which the ink above cannot say
///
/// It is **marked `[widened: outside]`** on [`rank_the_pages_we_are_alone_on`] — 2.81 ours over
/// 1.71 between `poppler` and `mupdf`, 1.65× and below that list's readable cut while the
/// per-measure test fires (ADR 0684) — and the paragraphs above price the page in ink, which is
/// not the measure any of that is taken on. `examples/compare_rasters` over the gate's own panels,
/// whose figures are **that example's and not this gate's**, one named pair per row where the
/// gate's line is our render against a consensus's worst member (ADR 0663):
///
/// ```text
/// at the gate's own 143 x 14
/// ours vs mupdf          mean 14.0709  ssim 0.85082      poppler vs mupdf        mean  8.5400  ssim 0.94612
/// ours vs poppler        mean 15.8362  ssim 0.83136      poppler vs ghostscript  mean 29.6598  ssim 0.62435
/// ours vs ghostscript    mean 39.6009  ssim 0.39894      mupdf vs ghostscript    mean 33.5215  ssim 0.56207
/// ```
///
/// Our own number is the **mean** against `mupdf`, 14.0709 of a text bound of 5.00, which is the
/// 2.81 the list prints; the pair's is the mean too, 8.5400, which is its 1.71. Widen that bound
/// by `Judgement::CORPUS`'s factor and the mean goes *inside* — 14.0709 against 17.08 — while the
/// **similarity** does not: 0.85082 against a widened 0.89224. So the measure that puts this page
/// above the mark is the one the ink cannot reach, on a raster of 143 × 14 where §10.7.4's licence
/// for a different glyph algorithm is worth the whole page.
///
/// **And the camps are the sharpest in the bucket.** `hayro`'s raster is 142 wide, so it is not in
/// the comparison above and both were cropped to 142 × 14 for this one: ours against it is mean
/// **4.3906** and similarity **0.98537**, closer than any two voting references are to each other,
/// where the closest voting pair is 8.5400 and 0.94612. That is `doc/todo/00`'s two-camp reading at
/// its extreme, and it is evidence about the verdict rather than about us
/// (`Reference::independence`).
const AMBIGUOUS_GLYPH_COVERAGE: [&str; 3] = [
    "copy_paste_ligatures.pdf page 1",
    "endchar.pdf page 1",
    "issue16316.pdf page 1",
];

/// Ambiguous, and §12.5.4's one sentence settles it against `poppler`.
///
/// `bug1552113.pdf` is 250×50 with one link, and the link is a trap: `/Border [0 0 112]` on a
/// `/Rect [5 25 155 45]`. Table 166 makes the third number the border's width, so the file asks
/// for a **112-unit border on a 150 × 20 rectangle**.
///
/// §12.5.4, of any annotation's border:
///
/// > If present, the border shall be drawn completely inside the annotation rectangle.
///
/// A `shall`, and it decides the picture entirely: a border that wide, drawn inside that
/// rectangle, *is* the rectangle. **`poppler` strokes 112 units centred on the
/// rectangle's edge**, so its blue covers most of the page — ink **201.32**, and the document's
/// own text says "this text should be visible". `mupdf` 17.06 and `hayro` draw no link border at
/// all, for `CONTRADICTED_LINK_BORDER`'s reasons; `ghostscript` 18.50 draws none either and is
/// being asked to print.
///
/// So four renderers disagree three ways and the clause names one of them. This is the shape
/// step 1 calls everybody-against-us read the other way round: the page reached its distance
/// because *one* reference is very far off, and the printed distance from the nearest is the
/// number that accuses us.
///
/// # "Ours fills it" was this note's own sentence and the raster refused it
///
/// It read that way for a hundred and sixty sessions, and so did the comment on `Border::inset`
/// one crate over: "the inset stops at the centre line, which fills the rectangle solid". What
/// this tree actually drew for `/Border [0 0 112]` on a 150 × 20 rectangle was a **38 × 20 block
/// in the middle of it** — ink **29.65** — because a stroke of the inset path cannot state that
/// region once the path degenerates in one axis, and past *both* dimensions it drew nothing at
/// all. Since the seven-hundred-and-fifty-sixth session a border whose width reaches either
/// dimension is *filled*, which is what the region is; ours is **67.21** and the sentence above is
/// now a description. ADR 0674, and `tests/annotations.rs`'s
/// `a_border_as_wide_as_its_rectangle_covers_it_and_no_more` is what holds it.
///
/// These are `doc/todo/00` step 5's inks, `magick`'s and not this gate's, which is why they carry
/// two decimals the gate does not print.
///
/// # And obeying the clause moved us **away** from the references, which is the cost stated
///
/// Our nearest was **1.90** bounds before that fix and is **5.98** after it, which puts this page
/// second in [`rank_the_pages_we_are_alone_on`] at 5.60× where it was on no printed list. Nothing
/// about that is a regression and the verdict does not move: the two references nearest us,
/// `mupdf` and `ghostscript`, are nearest because they draw **no link border at all**, so every
/// unit of the border this clause requires is a unit further from them. It is
/// `AMBIGUOUS_LINK_BORDER`'s divisor argument on the numerator — trap 9's shared *gap*, which
/// flatters a reader that shares it and accuses one that does not (ADR 0663) — and a ranking that
/// promoted this page for becoming more correct is the instrument working as designed rather than
/// a reason to stop.
///
/// **And the measure is the *mean* on both halves of that ratio, which is the one thing the
/// paragraph above did not say** (ADR 0688). The gate names it now: the numerator is 5.98 bounds
/// and the divisor 1.07, and **both are the mean** — ours against `mupdf`, theirs between `mupdf`
/// and `ghostscript`. The two halves are like for like, so this page's number may be read as the
/// ratio it looks like. `examples/compare_rasters`' figures, one named pair per row and not the
/// gate's line (ADR 0663), against text bounds of mean 5.00, worst tile 40.00, similarity
/// 0.9000:
///
/// ```text
///                           mean      ssim     bounds
/// ours vs poppler        75.0211   0.30279    15.00
/// ours vs mupdf          29.9125   0.64561     5.98  ← the numerator, on the mean
/// ours vs ghostscript    31.8506   0.62032     6.37
/// mupdf vs ghostscript    5.3459   0.94233     1.07  ← the divisor, on the mean
/// ```
///
/// A border is an area of ink, so a mean is exactly the measure a border mechanism should move,
/// and the shared gap is on the same measure: the two references nearest us are 1.07 apart on the
/// mean because neither draws the annotation. Numerator and denominator are one clause counted
/// twice with the sign reversed, in the unit the clause acts in — `AMBIGUOUS_LINK_BORDER`'s
/// removal on a page where the mean is what moves.
///
/// **And `poppler`'s side of this page is a population rather than a page**, which is the other
/// half of the same session: it puts an annotation border's path *on* `/Rect`'s boundary at every
/// width, so half the stroke is outside — five units at a width of 10, 56 here — and
/// `crates/pdf-model/examples/border_overhang_census.rs` measures that over every corpus page
/// stating a border and no `/AP`. `issue12750.pdf` is the honest limit of the instrument at a width
/// of 1: a `/Rect` on fractional coordinates puts `poppler`'s snapped thin line on the same columns
/// as ours, so the overhang is there in the geometry and not in any whole pixel. ADR 0675.
const AMBIGUOUS_OVERSIZED_BORDER: [&str; 1] = ["bug1552113.pdf page 1"];

/// Ambiguous, and it is what four renderers construct for a widget the file left to them.
///
/// `bug1844576.pdf` is a form: a text field holding "Hello World" and a push button reading
/// "Click", with `/NeedAppearances true` on the interactive form. The text field states
/// `/MK << /BC [0 0 0] >>` and no `/Border` or `/BS`, so Table 191 gives it a black border and
/// §12.5.4 gives that border a width — "[i]f neither the Border nor the BS entry is present, the
/// border shall be drawn as a solid line with a width of 1 point".
///
/// Ours, `mupdf`, `ghostscript` and `hayro` draw the border; **`poppler` draws none**. Ink: ours
/// 22.04, `hayro` 22.57, `mupdf` 25.21, `ghostscript` 27.23, `poppler` 15.70 — and `poppler` at
/// 576 dpi is 15.88, so its omission is a decision rather than a resolution effect.
/// `ghostscript`'s extra is Table 168's `/S /B` bevel on the button, which the file does not ask
/// for.
///
/// Four out of five and the clause agree, which is as settled as this bucket gets without a
/// closed form. What keeps it `ambiguous` is that the four draw four slightly different borders.
const AMBIGUOUS_CONSTRUCTED_WIDGET: [&str; 1] = ["bug1844576.pdf page 1"];

/// Ambiguous, and the difference **does not shrink with the pixels**, which is what makes it a
/// group of its own rather than `AMBIGUOUS_IMAGE_REDUCTION`'s page.
///
/// `issue19971.pdf` page 5 is a letter page of list items, a heading, two paragraphs, and one
/// photograph: a 2500 × 1750 `DCTDecode` image in an `ICCBased` space, drawn at 504 ppi. It came
/// off §3a's ranking in the two-hundred-and-ninety-fifth session at **0.69 from the nearest
/// reference and 1.23 from the furthest** — the tightest ratio the tail had left, which
/// `doc/todo/00`'s step 1 reads as *we are alone*.
///
/// # Step 6's two ladders, and they produce the tightest limit this bucket has measured
///
/// ```text
///                 72 dpi    576 dpi
/// poppler        32.0810    32.0907
/// mupdf          32.0583    32.0899
/// ours (1x, 8x)  31.8793    31.9350
/// ```
///
/// The two references agree at eight times the resolution to **0.0008 of 255**, so the geometry
/// is 32.090 and there is no argument about where it is. Ours climbs towards it and stops
/// **0.155 short**.
///
/// # Localising it, because a page-level number says nothing about where
///
/// A four-by-four grid of tile means at 8×, ours against `poppler`'s, is a **constant fraction**
/// rather than a constant offset or a missing tile: every one of the fourteen tiles that carry
/// ink is between 0.4% and 0.9% light, from the 3.18 tile to the 161.26 one. Nothing is absent.
/// The two tiles holding the photograph are 161.256 / 161.977 and 157.043 / 157.743, which is
/// **57% of the whole deficit** on 12% of the page.
///
/// # And then the assumption behind step 6 fails, in the direction that is a finding
///
/// Step 6 works because a renderer's departure from the geometry shrinks as the pixels shrink.
/// At **16×**, where the image is *enlarged* rather than reduced, the photograph's per-channel
/// difference is identical to 8× to three decimal places:
///
/// ```text
///            R        G        B
/// 8×    +0.882   +0.625   +0.936    (ours minus poppler, over the photograph's band)
/// 16×   +0.882   +0.625   +0.936
/// ```
///
/// A difference that does not move with resolution is not scan conversion and not the reduction.
/// It is in the samples, and there are only two places it can be.
///
/// **It is not the JPEG decoder.** The codestream extracted with `pdfimages -j` and decoded twice
/// gives `libjpeg` R 98.2522 G 97.7088 B 98.1845 against `zune-jpeg`'s R 98.4042 G 97.5980
/// B 98.3734 — under 0.2 of 255 apiece and **mixed in sign**, which cannot produce a uniform lift
/// six times larger.
///
/// **It is the colour management.** The space is `[/ICCBased 27 0 R]` with `/N 3`, and the
/// profile is a 296-byte `mntrRGB` matrix-shaper — `rTRC`, `gTRC`, `bTRC` and an XYZ matrix,
/// described as *Google/Skia/7C5FA21513974 74A0486BBCC83733D59*. `pdf_model::icc` evaluates that
/// form itself (ADR 0009); `poppler` evaluates it through `lcms`. Two implementations of the same
/// transform, differing by about 1.2 of 255 on a photograph.
///
/// # What the specification determines, and this note over-read the silence twice
///
/// §8.6.5.5 defines the *source*: the profile is the space, and both renderers read the same one.
///
/// **What follows was wrong in two ways until ADR 0495, and the second is the one that matters.**
/// It read: *the destination is where the standard stops, and it says so in one sentence — §10.3.1
/// puts "[t]he characteristics of the output device" beyond the scope of this document … there is
/// no clause left to depart from.*
///
/// The first fault is the quotation. **§10.3.1 contains no such phrase**; what it puts beyond the
/// document's scope is
///
/// > The specific method by which the CIE-based destination colour space is established
///
/// and the words in quotation marks came from §10.4.2.4, which is about black generation and
/// undercolour removal on the way into `DeviceCMYK`. Principle 5's rule is that quotation marks
/// mean verbatim, and a sentence attributed to the wrong subclause is how a silence gets asserted
/// about a clause that never said it. (The NOTE *is* quoted correctly, and it belongs to §10.3.1.)
///
/// The second fault is the reading, and it is the one [`CONTRADICTED_CALRGB_TO_SCREEN`] was
/// corrected for one round earlier (ADR 0494), one colour space over. **§10.3.1's next sentence is
/// a `shall`**: a CIE-based source colour is to be converted to a CIE-based destination colour
/// based on the appropriate ICC specification. (Prose rather than a quotation because Errata
/// Collection 3's Issue #181, `Review`/`Completed`, strikes that sentence's dated *ISO
/// 15076-1:2010 (ICC.1:2010)* and points at Table 66 instead; `spec-errata emit` files the strike
/// under §10.4.1's heading and §10.3.1's ledger row carries it.) So there *is* a clause left to
/// depart from, and it is the one this group is about: two evaluations of one matrix-shaper
/// profile, differing by about 1.2 of 255, where the referenced standard says how a profile is
/// evaluated. What each processor may still choose is which space the pixels are — sRGB here, by
/// ADR 0012 — and that choice cannot produce a 1.2 offset between two renderers reading the same
/// profile onto the same screen.
///
/// **The verdict stays `ambiguous` and the work it names changes.** Nothing here is evidence about
/// which of the two evaluations ICC.1 licenses; establishing that needs the profile's own tags
/// worked through by hand, the way ADR 0494 worked `ghostscript`'s synthesised `scnr` profile
/// through. Until somebody does, this is a page where the standard names an authority this note
/// had recorded as silence — which is `CLAUDE.md`'s own warning about a recorded silence, for the
/// second time in two rounds.
///
/// It is still `AMBIGUOUS_DEVICE_CMYK_CONVERSION`'s argument one colour space over in *shape*, and
/// the group is separate because the *evidence* is different: there the spread is between four
/// conversions of a device space, here it is between two evaluations of one embedded profile,
/// and only the resolution ladder could tell the second from a scan-conversion difference.
///
/// The remaining 43% of the page's deficit is its text, which is
/// `AMBIGUOUS_GLYPH_SCAN_CONVERSION`'s subject and is measured alone on page **6** of the same
/// document — 456 commands, no image at all, two ladders agreeing to 0.0055 of 255 and ours
/// climbing onto the limit to within 0.025.
/// # A second in the four-hundred-and-fifteenth, and it arrived by a *report* being withdrawn
///
/// `issue14200.pdf` page 1 is one command: a 918 x 427 `FlateDecode` photograph of a kitchen
/// display in `[/ICCBased 13 0 R]` with `/N 3` and `/Intent /RelativeColorimetric`, placed at
/// 220.3 x 102.5 points on a 425 x 239 page, with an `/SMask` image beside it. It was never
/// judged before this round because the form it sits in carries
/// `/Group << /S /Transparency /CS /DeviceCMYK >>` and this tree read that as the blending space
/// — but the group states no `/I`, and §11.6.6 gives a non-isolated group no colour space of its
/// own. The page states no `/Group` either, so nothing on it composites anywhere but the device's
/// components and the report was false.
///
/// ```text
///                 72 dpi   288 dpi   576 dpi
/// poppler        12.5109   12.3841   12.5137
/// mupdf          12.5293   12.4036   12.5115
/// ours (1x/4x/8x)12.2911   12.3540   12.3575
/// ```
///
/// The dip at 288 dpi is both references passing through the resolution where the image is drawn
/// **1:1** — 918 samples onto 881 device pixels — so the ladder is measuring resampling rather
/// than edges, and only the outer rungs are worth reading. At 576 dpi the two references agree to
/// **0.0022 of 255** and ours is flat 0.154 below them, having moved 0.0035 between 4x and 8x.
///
/// **A flat offset is a colour**, and the per-channel means say it is a uniform one:
///
/// ```text
///             R          G          B
/// ours    244.992    242.153    240.570
/// poppler 244.844    241.995    240.407
/// mupdf   244.842    241.998    240.414       (all at 576 dpi, -alpha off)
/// ```
///
/// +0.148, +0.157, +0.160 on the three channels against `poppler` and the same against `mupdf`,
/// where the two references are 0.007 apart from each other. The profile is a 3144-byte `mntrRGB`
/// with `rXYZ`/`gXYZ`/`bXYZ` and one shared 1024-point `curv` — the same matrix-shaper form as
/// above, evaluated by `pdf_model::icc` here and by `lcms` in both references, which is trap 9's
/// shared data seen from the other side.
///
/// At the page's own scale ours is a further 0.066 below its own limit, and that part is the
/// image reduction — 918 samples onto 220 device pixels, `AMBIGUOUS_IMAGE_REDUCTION`'s subject
/// and ADR 0025's stated departure. Two mechanisms, both already named, and neither of them the
/// transparency group the page spent this project's history being reported for.
const AMBIGUOUS_ICC_MATRIX_PROFILE: [&str; 2] = ["issue19971.pdf page 5", "issue14200.pdf page 1"];

/// Ambiguous, and **the picture is the whole finding**: one reference decoded the image wrongly.
///
/// `issue19326.pdf` page 1 is a 132 × 81 crop holding one `JPXDecode` image — 551 × 337, sixteen
/// bits per component, with a `JPXDecode` soft mask beside it — that reads *JPX* in a sans-serif
/// face. It came off §3a's ranking at **0.65 from the nearest reference and 11.06 from the
/// furthest**, a ratio of seventeen, which `doc/todo/00`'s step 1 reads as a page about the
/// references rather than about us.
///
/// ```text
/// ours 46.2472 │ poppler 46.4277 │ hayro 46.4416 │ mupdf 46.4549 │ ghostscript 47.6433
/// ```
///
/// **The ink is the least informative number on this page**, and that is worth saying out loud:
/// `ghostscript` is 1.2 of 255 above the other four, which on a page of black letterforms looks
/// like an edge difference — and its panel is not letterforms at all. Ours, `poppler`, `mupdf`
/// and `hayro` draw *JPX*; `ghostscript` draws a band of scrambled blocks with about the same
/// coverage. A picture is one `Read` away and no metric on this page would have said so.
///
/// # What the specification determines
///
/// ISO/IEC 15444-1 defines the decoding exactly, which makes this `doc/todo/00`'s first shape —
/// the clause determines it and we can be checked against it. `tests/jpeg2000.rs` is that check,
/// and on *this* codestream it declines to run: `opj_decompress` writes a Netpbm whose maximum is
/// the codestream's own precision, so a sixteen-bit image comes back at 65535 against this tree's
/// eight-bit pipeline. So the evidence here is the weaker kind — four independent decoders
/// producing the same legible glyphs — and it is recorded as the weaker kind rather than dressed
/// up. What would make it the strong kind is the question that test's own comment already names:
/// how a decoder scales sixteen bits to eight, which §7.4.9 leaves to it.
///
/// The verdict is `ambiguous` for trap 12's arithmetic reason: one reference far enough out drags
/// the consensus apart, and the four that agree do so to **0.21 of 255**.
const AMBIGUOUS_A_REFERENCE_DECODED_THE_IMAGE_WRONG: [&str; 1] = ["issue19326.pdf page 1"];

/// Ambiguous, and §10.7.4's own last sentence is the answer.
///
/// `issue4402_reduced.pdf` is a 215 × 28 crop box — `/CropBox [19.7223 787.097 234.535 815.348]`
/// out of an A4 `/MediaBox` — holding one line of eight-point text, *KEY INVESTOR INFORMATION*,
/// in an embedded `CIDFontType2` subset, and a hairline rule under it. Forty-five commands, no
/// image, no shading, nothing else on the page. So the page's mean **is** its glyph coverage:
/// there is nothing else for the five renderers to differ about, and the tolerance is measured
/// against a mean over 6 020 pixels rather than over a page.
///
/// §10.7.4, after eight paragraphs of rules that bind fills, strokes, images and clips:
///
/// > Scan conversion of character glyphs may be performed by a different algorithm from the
/// > preceding one.
///
/// > NOTE 2 Font rendering algorithms use hints in the glyph descriptions and techniques that are
/// > specialised to glyph rasterization.
///
/// A *may*, addressed to exactly this page. The clause spends its normative force on the
/// geometry — the outlines and where they land — and then hands glyph coverage to the
/// implementation in one sentence. This group is todo `00`'s third shape: the clause puts the
/// answer beyond itself, and says so.
///
/// What is left to check is the half the clause does bind, and the two-ladder closed form checks
/// it. Ink over the 215 × 28 frame the oracle compares on:
///
/// ```text
///              72 dpi    limit (poppler 1152 / mupdf 1152)
/// ours          55.41    56.976 / 56.988
/// hayro         55.37
/// ghostscript   56.11
/// mupdf         57.19
/// poppler       58.38
/// ```
///
/// The two ladders agree to 0.012 of 255, so there is a limit. **Ours climbs onto it** — 55.41,
/// 56.71, 56.78, 56.91 at 1×, 4×, 8× and 16× — ending 0.07 under, which is our marks covering
/// the geometry. At the page's own resolution the five spread by **3.0 of 255**, ours 1.57 under
/// the geometry and `poppler` 1.40 over, with `ghostscript` and `mupdf` between them: a
/// fourteen-percent spread in text ink on one line of eight-point glyphs.
///
/// And we are not the outlier. Every pair of renders, mean absolute error over that frame:
///
/// ```text
///              poppler   mupdf   ghostscript   hayro
/// ours          0.0450  0.0369      0.0704    0.0219
/// poppler          —    0.0392      0.0630    0.0338
/// mupdf                    —        0.0683    0.0288
/// ghostscript                          —      0.0605
/// ```
///
/// **Ours against `hayro` at 0.0219 is the smallest pair in the whole matrix**, and every pair
/// involving `ghostscript` is larger than our worst. The page reaches this bucket because five
/// glyph rasterisers disagree with each other, which is the sentence above written out.
///
/// # The same finding at page scale
///
/// `pr12564.pdf` is a school newsletter — 612 × 1008, **3940 commands**, nine faces, and
/// essentially no geometry that is not a glyph. The pictures are indistinguishable side by side
/// and the numbers say why they cannot be identical:
///
/// ```text
///                ours     hayro    mupdf    poppler   ghostscript   limit (pop/mu at 8x)
/// at 72 dpi     25.050    24.859   25.207    26.268      27.189       25.329 / 25.314
/// ```
///
/// The two ladders agree to **0.015 of 255**, ours at 8× is 25.246 — 0.075 under — and at the
/// page's own scale ours is the **second nearest of the five** to that limit, behind `mupdf` by
/// 0.16 and ahead of `poppler` by two thirds of a level and `ghostscript` by one and a half.
/// Everybody draws the same newsletter; the spread is 2.3 of 255 of glyph coverage, which is the
/// permission quoted above spread over four thousand marks instead of twenty-four.
/// # And the same finding on glyphs nobody would call small
///
/// `issue2884_reduced.pdf` is a 169 × 19 crop box holding one line of Japanese — sixteen
/// commands, an embedded `Identity-H` `CIDFontType2` subset, and nothing else — so like
/// `issue4402_reduced.pdf` its mean *is* its glyph coverage. Taken off §3a's ranking in the
/// two-hundred-and-sixtieth session at 0.90 from the nearest reference and 4.53 from the
/// furthest.
///
/// ```text
///                72 dpi   288 dpi   576 dpi
/// poppler        33.2516  33.7927   33.8111
/// mupdf          33.3049      —     33.7929
/// ours (1x, 8x)  33.6799      —     33.7511
/// ```
///
/// The two ladders agree to **0.018 of 255**, so there is a limit at 33.80, and ours climbs onto
/// it from 33.68 to 33.75 — ending 0.05 under, which is 0.15% of the page's ink. **Both
/// references start half a level *below* their own limit at the page's own scale and climb; ours
/// starts a third of a level above and climbs a little further.** The glyphs are the same glyphs
/// in the same places — magnified eight times the two panels are indistinguishable — and the
/// verdict is the text tolerance applied to a page that is nothing but ideographs.
/// # Three more off the ranking's new head, in the two-hundred-and-sixty-fourth
///
/// With three populations diagnosed the bucket is its tail, and the tail's head is this:
///
/// ```text
///                        ours 1x   ours 8x   poppler 72   poppler 576   mupdf 576
/// issue11913.pdf p1      16.5021   16.6020     16.5224      16.6238      16.6199
/// issue1350.pdf  p1      17.9281   16.8382     17.7382      16.8468      16.8635
/// issue1350.pdf  p3      20.1250   20.2458     20.1743      20.3821      20.5221
/// ```
///
/// The first is three weights of embedded Verdana and no images at all, and **the two ladders
/// and ours agree to 0.024 of 255** — the tightest three-way agreement any page in this bucket
/// has produced. The second is a voucher of five-point text: the two references' limits are 0.017
/// apart and ours is between them, while at the page's own scale every renderer is a level high,
/// which is what small glyphs cost before the pixels shrink. The third is the same document's
/// third page, where the references are 0.14 apart from each other and ours is 0.14 from the
/// nearer — a page about the references' spread rather than about ours.
///
/// # And the ranking's head in the two-hundred-and-seventy-ninth, which is this shape again
///
/// `issue7769.pdf` is 153 × 63 and its whole content is **24 commands** setting one sentence —
/// *Scan here to make a donation!* — over two lines. No image (`pdfimages -list` names none),
/// no rules, nothing that is not a glyph, so like `issue4402_reduced.pdf` and
/// `issue2884_reduced.pdf` its mean *is* its glyph coverage. It reached the head of the ranking
/// at **0.67 from the nearest reference and 0.97 from the furthest** — a ratio of 1.45, which is
/// `doc/todo/00`'s step 1 shape for *we are alone* and the tightest such ratio the tail had left.
///
/// ```text
///                72 dpi   144 dpi   288 dpi   576 dpi   1152 dpi
/// ours          16.5160   16.9669   16.9304   17.0152       —
/// poppler       16.7065   16.8624   16.9656   17.0238       —
/// mupdf         16.7431   16.8830   16.9722   17.0270   17.0452
/// hayro         17.0665       —         —         —         —
/// ghostscript   17.3083       —         —         —         —
/// ```
///
/// **The two ladders agree to 0.003 of 255 at 576 dpi** — the tightest pair this group has
/// measured — so there is a limit at about 17.03, and `mupdf`'s own 1152 says it is still
/// creeping up towards 17.05. **Ours climbs onto it**: 16.516 at the page's own scale to 17.015
/// at eight times, ending 0.01 under two references that are 0.01 and 0.00 under themselves.
/// So the marks are the geometry's and the difference is where the coverage lands on a
/// 153-pixel-wide raster.
///
/// At 72 dpi the five renderers spread by **0.79 of 255** with ours lowest and `ghostscript`
/// highest, which on a page this small is a sixth of a level per glyph. §10.7.4's last sentence
/// is the permission and it is quoted at the top of this group: scan conversion of character
/// glyphs may be performed by a different algorithm.
/// # And the tightest *ratio* the tail has produced, in the three-hundred-and-tenth
///
/// `issue14999_reduced.pdf` page 1 is a 370 × 60 crop of **55 commands** — a line of text and
/// nothing else — and it came off §3a's ranking at **0.55 from the nearest reference and 0.76
/// from the furthest**, a ratio of 1.38 where step 1 says a ratio near one means *we are alone*.
///
/// ```text
///                 72 dpi    576 dpi
/// poppler        16.8574   16.9759
/// mupdf          16.6496   16.8506
/// ours (1x, 8x)  16.8074   16.8609
/// ```
///
/// Alone, and inside them: all three climb, the two references end **0.125 of 255 apart** and
/// ours lands between them. `ghostscript` is 17.2736 at the page's own scale, 0.4 above all four.
/// So the shape step 1 reads as *we are alone* is here the shape of a page where nobody has a
/// limit tight enough to be alone from — which is what §10.7.4's last sentence permits and this
/// group quotes.
/// # Four pages of one document, in the three-hundred-and-thirteenth
///
/// `file_pdfjs_test.pdf` had **four** of the seventy-six undiagnosed names, which is
/// `doc/todo/00`'s "check what else on the list is the same file" paying for the fourth time. It
/// is Mozilla's own test-suite documentation: four US Letter pages of headings, paragraphs and
/// bulleted lists in six embedded subsets — Times New Roman in three weights, Georgia Bold,
/// Symbol and a `CIDFontType2` SimSun — with no image anywhere, so each page's mean *is* its
/// glyph coverage.
///
/// ```text
///            poppler 576   mupdf 576   they agree to   ours 8x   short by
/// page 1        11.0742     11.0708        0.0034      11.0665     0.006
/// page 2        11.2625     11.2629        0.0004      11.2539     0.009
/// page 3         9.6327      9.63485       0.0022       9.61846    0.015
/// page 4        11.0166     11.0174        0.0008      11.0090     0.008
/// ```
///
/// Four independent limits, each measured from two ladders that converge on it from opposite
/// sides, and ours climbing onto every one of them from below by 0.05 to 0.10 of 255 between 1x
/// and 8x. **The residual is 0.006 to 0.015 of 255**, which is this group's own subject; the
/// nearest reference at the page's own scale is 0.46 to 0.55 away, which is what a page of
/// eight-point text costs before the pixels shrink.
///
/// # A page of TeX, in the three-hundred-and-eighteenth, where ours lands *between* the limits
///
/// `issue6132.pdf` page 1 came off §3a's ranking at **0.50 from the nearest reference and 0.93
/// from the furthest**. It is a US Letter page of 2 328 commands, no image anywhere, set in nine
/// embedded Computer Modern `Type1C` subsets — so its mean, like every other member here, *is* its
/// glyph coverage.
///
/// ```text
///                  72 dpi   288 dpi   576 dpi
/// poppler         10.3520   10.4153   10.4255
/// mupdf           10.4040   10.4455   10.4430
/// ours (1x/4x/8x) 10.4147   10.4315   10.4361
/// ```
///
/// Both references climb and end **0.0175 of 255 apart**; ours climbs too and ends **between
/// them** — 0.011 above `poppler`'s limit and 0.007 below `mupdf`'s. And this page is the group's
/// clearest instance of the other half of the argument: at the page's own scale ours is already
/// 0.02 from its own limit, where `poppler` is 0.07 and `mupdf` 0.04 below theirs, so of the five
/// renderers at 72 dpi — ours 10.4147, `mupdf` 10.4040, `poppler` 10.3520, `ghostscript` 10.7446,
/// `hayro` 10.3149, a spread of 0.43 — ours is the nearest to a limit neither of us is trusted
/// for.
///
/// # And a page with a highlight over it, in the three-hundred-and-nineteenth
///
/// `issue13242.pdf` page 1 — 0.51 from the nearest reference and 1.21 from the furthest — is
/// 2 449 commands of Lorem ipsum in one embedded `CIDFontType2` Calibri subset with §12.5.6.10's
/// yellow wash over eight lines of it, and no image at all.
///
/// ```text
///                  72 dpi   288 dpi   576 dpi
/// poppler         13.1328   13.3008   13.3168
/// mupdf           13.1223   13.2659   13.3052
/// ours (1x/4x/8x) 13.3152   13.2913   13.3119
/// ```
///
/// The two ladders converge to **0.0116 of 255** of each other and ours ends between them again.
/// What is unusual is the first column: **ours at the page's own scale is already 0.003 from its
/// own 8× value**, where both references are 0.18 *below* theirs — so on this page the whole of
/// the oracle's disagreement is the two references' own 72 dpi shortfall.
///
/// The other two are worth naming because they are why the verdict is `ambiguous` rather than a
/// contradiction: `ghostscript` is 14.4157 and `hayro` 10.8733 at 72 dpi, 1.1 over and 2.4 under a
/// limit the other three agree about, on a page whose only non-glyph mark is a flat wash under
/// `Multiply`. The spread is 3.5 of 255 and the diagnosis rests on the two ladders that converge.
///
/// # And the tightest limit this bucket has measured, in the three-hundred-and-twenty-third
///
/// `issue840.pdf` page 2 — 0.48 from the nearest reference and 2.28 from the furthest — is the
/// second page of a festival timetable: text and rules on a light ground, 14 of 255 of ink.
///
/// ```text
///                72 dpi   288 dpi    576 dpi
/// poppler       14.4058   14.2878   14.27200
/// mupdf         14.0665   14.2421   14.27220
/// ours (1x/8x)  14.1965             14.26690
/// ```
///
/// **The two ladders converge from opposite sides to 0.0002 of 255 of each other** — `poppler`
/// descending, `mupdf` climbing — which is the tightest agreement about a geometry this bucket
/// has produced, better than `issue12963.pdf`'s 0.0004. Ours ends 0.005 under it, and at the
/// page's own scale ours is 0.052 under where `poppler` is 0.134 over, `mupdf` 0.206 under and
/// `ghostscript` **5.05 over**, which is why nobody can be called wrong.
///
/// Its page 1 is `AMBIGUOUS_EIGHT_BIT_COMPOSITING` and the reason is written there: one document,
/// two pages, two different answers.
/// # A page of a web site, in the three-hundred-and-twenty-ninth
///
/// `bug1868759.pdf` page 1 came off §3a's ranking at **0.42 from the nearest reference and 0.84
/// from the furthest** — a ratio of two, which is as alone as this tail gets. It is a printed
/// Mozilla support page: 1 573 commands of French text, links and bullets, with one 480 × 501
/// image at **1440 ppi** that occupies a few dozen device pixels.
///
/// ```text
///                 72 dpi    576 dpi   1152 dpi
/// mupdf          10.2470   10.26790   10.27120
/// poppler        10.2621   10.25220   10.25160
/// ours (1x/8x/16x) 10.0780  10.24210   10.24050
/// ```
///
/// **The two ladders close from opposite sides** — `mupdf` climbing, `poppler` descending — and
/// end 0.0196 of 255 apart; ours is flat from 8× and sits 0.011 under the lower of them. At the
/// page's own scale ours is 0.163 under its own limit, which is this group's standing subject: a
/// page that is nothing but nine-point text is a page of glyph edges, and §10.7.4's last sentence
/// is the permission both ends of the ladder are exercising.
/// # Three more in the three-hundred-and-thirty-first, two of them one document
///
/// ```text
///                        ours 1x   ours 8x   poppler 576   mupdf 576   apart
/// issue13201.pdf p1      10.3559   10.3506     10.36500     10.35930   0.0057
/// canvas.pdf p1          23.6980   23.7920     23.86670     23.90290   0.0362
/// canvas.pdf p2          21.3130   21.4305     21.43890     21.51130   0.0724
/// ```
///
/// All three are pages of embedded `TrueType` subsets and nothing else, so each mean *is* its
/// glyph coverage, and all three have ours climbing onto a limit two ladders bracket. What is
/// worth keeping is `issue13201.pdf`'s first column: **ours at the page's own scale is 0.005 from
/// its own 8× value**, where `poppler` is 0.07 under its and `mupdf` 0.14 under — the same shape
/// `issue13242.pdf` has, and the reason this group's argument is about *where* the ink lands
/// rather than about how much of it there is.
///
/// `canvas.pdf`'s two pages are `doc/todo/00`'s "check what else on the list is the same file"
/// paying its sixth time, and the two are the same finding to two figures.
/// # And one where a single row is a whole reference's excess, in the three-hundred-and-forty-sixth
///
/// `issue7406.pdf` page 1 is 200 × 50 and shows one line — *placed after the SOF[n] markers.* — in
/// nine-point type. `doc/todo/00`'s note about it is that it once drew a JPEG cyan on black inside
/// an `ambiguous` verdict and is correct now; what nobody had measured is why it is still
/// `ambiguous`.
///
/// ```text
/// ours 21.9458 │ ghostscript 21.9519 │ hayro 21.8320 │ mupdf 22.1948 │ poppler 22.5321
/// ```
///
/// **Ours and `ghostscript` agree to 0.006 of 255**, which on a page this small is 50 rows of
/// glyph edges landing in the same places. `poppler` is 0.59 above, and the per-row profile puts
/// all of it in **one row**:
///
/// ```text
/// row 40   ours 3   poppler 35   mupdf 4   ghostscript 4   hayro 4
/// ```
///
/// 32 of 255 on one row of fifty is 0.64 of the page, against `poppler`'s 0.59 excess — so its
/// whole disagreement with the other four is the bottom row of the letters, and every other row is
/// within a few levels across all five. Step 6 agrees that nobody is wrong: `poppler` descends
/// 22.5321 → 22.1655, `mupdf` 22.1948 → 22.1089 and ours climbs 21.9458 → 22.2116, **three ladders
/// ending within 0.103 of 255 of each other**.
/// # And its neighbour, one page over on the same ranking
///
/// `issue5896.pdf` page 1 is 200 × 50 too — one line of text and 25 commands — and it came off the
/// ranking in the three-hundred-and-fiftieth session at 0.30 from the nearest and 1.07 from the
/// furthest. Every row that differs is a row the letters are on (21 to 30 of 50), and the largest
/// is 19 of 255 at row 24 where `poppler` is 63 against our 44 and `mupdf`'s 43.
///
/// ```text
///                72 dpi    576 dpi
/// poppler       8.0013    7.13641
/// mupdf         7.0505    7.24492
/// ours (1x/8x)  7.2990    7.23938
/// ```
///
/// **The two ladders close from opposite sides and end 0.109 of 255 apart**, and ours at 8× is
/// **0.005 from `mupdf`'s limit** and inside both. This group's standing sentence, one more time:
/// a page that is nothing but nine-point text is a page of glyph edges.
/// # Two more specimens, in the three-hundred-and-sixty-second
///
/// `issue13193.pdf` is 300 × 50 and sets one line of italic mathematics —
/// `∘∠⊥ABCDEFGHMORSabcm−△` — and `issue3584.pdf` is 200 × 50 and sets the single word *waves* in
/// a blue serif face. Both came off the ranking at 0.19 from the nearest reference, which is the
/// bottom of it, and both answer to step 6 without a picture being needed:
///
/// ```text
///                    1x        4x        8x
/// issue13193  ours   15.5932   15.6568   15.6619
///             poppler 15.6538  15.6570   15.6505
/// issue3584   ours    4.39376   4.46974   4.46574
///             poppler 4.83001   4.54540   4.52264
/// ```
///
/// **On `issue13193` the two limits are 0.011 of 255 apart** — 15.6619 against 15.6505 — so every
/// bit of the 1× difference is where the edges land, and there is nothing else to find. On
/// `issue3584` they end 0.057 apart, and the five renderers split into two camps at the page's own
/// scale: ours 4.394, `hayro` 4.382 and `mupdf` 4.441 against `ghostscript` 4.732 and `poppler`
/// 4.830. A 0.35 spread on a page whose whole ink is 4.5 is stem darkening on a 12-point serif,
/// and the two camps are the two answers to it.
/// # Two more in the three-hundred-and-seventy-ninth, and the first is four ladders on one limit
///
/// Both came off `doc/todo/00`'s ranking with a *nearest* under 0.30, which was the bottom of that
/// list, and both are one embedded font on a page with nothing else on it.
///
/// `issue4665.pdf` page 1 (0.17 / 0.93) is 275 × 50 and sets one kerned line — *If you're reading
/// this, chances are that you care* — at 12 points from an embedded `FXXLHJ+Cambria` `TrueType`
/// subset under `/WinAnsiEncoding`.
///
/// ```text
///                 72 dpi      288       576      1152
/// poppler        8.78691   8.80700   8.80619   8.80634
/// mupdf          8.81804   8.81219   8.80961   8.80744
/// ghostscript   12.13910   8.84644   8.77455   8.84269
/// ours           8.64698   8.83051   8.80322   8.79873   (1x, 4x, 8x, 16x)
/// ```
///
/// **All four ladders end within 0.044 of 255 of each other, and the three without `ghostscript`
/// within 0.009** — which makes this the first page in this group where every reference present
/// converges on one number rather than two bracketing it. `ghostscript`'s 3.34 of 255 at the
/// page's own scale is therefore scan conversion *by its own later rungs*, which is a stronger
/// statement than a spread: the renderer that is 38% heavy at 72 dpi agrees with everybody at
/// 1 152. Ours is 0.15 under the limit at the page's own scale and on it from 4×.
///
/// `bug911034.pdf` page 1 (0.29 / 2.44) is 200 × 200 and is 72 `Tj` operators over eight lines,
/// each showing one two-byte code at 20 points through `/Encoding /Identity-H` from an **embedded**
/// `PATKIN+ArialUnicodeMS` subset — glyph indices 0x2000 to 0x207f, which is a specimen sheet of
/// that subset's Kannada.
///
/// ```text
///                72 dpi      288       576      1152
/// poppler       18.44380  18.55370  18.59410  18.59170
/// mupdf         18.48410  18.56620  18.60150  18.61480
/// ghostscript   27.66400  18.23080  18.91160  18.72720
/// ours          18.18930  18.49010  18.63810  18.58340   (1x, 4x, 8x, 16x)
/// ```
///
/// **Three ladders end within 0.031 of 255** — 18.5917, 18.6148, 18.5834 — and ours is inside
/// them. `ghostscript` at the page's own scale is **9.07 of 255 over the limit those three reach**
/// — its own ladder is too unsteady to be one — which is half again
/// the ink of the page, and collapses onto the other three by 4×; it is the same renderer doing
/// the same thing as on `issue4665.pdf` above, on glyphs a quarter of the size. The pairwise
/// matrix at 72 dpi puts ours against `mupdf` smallest of the ten pairs, with every pair involving
/// `ghostscript` six times larger than our worst.
///
/// Neither page is a defect and both are §10.7.4's last sentence: a page that is nothing but text
/// is a page of glyph edges, and where a renderer puts an edge is what the clause declines to say.
const AMBIGUOUS_GLYPH_SCAN_CONVERSION: [&str; 26] = [
    "issue4665.pdf page 1",
    "bug911034.pdf page 1",
    "issue13193.pdf page 1",
    "issue3584.pdf page 1",
    "issue5896.pdf page 1",
    "issue7406.pdf page 1",
    "issue13201.pdf page 1",
    "canvas.pdf page 1",
    "canvas.pdf page 2",
    "bug1868759.pdf page 1",
    "issue840.pdf page 2",
    "issue13242.pdf page 1",
    "issue6132.pdf page 1",
    "file_pdfjs_test.pdf page 1",
    "file_pdfjs_test.pdf page 2",
    "file_pdfjs_test.pdf page 3",
    "file_pdfjs_test.pdf page 4",
    "issue14999_reduced.pdf page 1",
    "issue19971.pdf page 6",
    "issue11913.pdf page 1",
    "issue7769.pdf page 1",
    "issue1350.pdf page 1",
    "issue1350.pdf page 3",
    "issue2884_reduced.pdf page 1",
    "issue4402_reduced.pdf page 1",
    "pr12564.pdf page 1",
];

/// Ambiguous because one reference smooths an image the file did not ask it to.
///
/// `issue10339_reduced.pdf` page 1 draws two **39 × 40** indexed images at 23 ppi on a 280 × 150
/// page — an enlargement of about three and a half, and each image is a small grid of flat
/// coloured cells. It sat at the *bottom* of `doc/todo/00`'s ranking at **0.00 from the nearest
/// reference and 6.84 from the furthest**, which is the ranking saying as loudly as it can that
/// the page is not ours to answer for.
///
/// ```text
/// ours 57.3291 │ ghostscript 57.3291 │ hayro 57.3872 │ mupdf 57.3987 │ poppler 58.3126
/// ```
///
/// **Our raster and `ghostscript`'s are identical**: mean absolute difference **0**, not rounded —
/// every one of the 42 000 pixels the same byte. `hayro` is 0.00034 away and `mupdf` 0.0041.
/// `poppler` is alone, and the picture says what it is doing: its cells have soft edges where the
/// other four have hard ones.
///
/// Neither image states `/Interpolate`, so Table 87's default applies — "[a] flag indicating
/// whether image interpolation should be performed by a PDF processor … Default value: false" —
/// and §8.9.5.3 is careful about what the entry is even when it *is* stated:
///
/// > However, this is only a hint, and a PDF processor may ignore it.
///
/// A hint that may be ignored when present cannot oblige anybody when absent, so smoothing here is
/// a choice rather than an error, and the clause is why this is `ambiguous` rather than
/// `CONTRADICTED`: four renderers take the default and one does not, on a page where the default
/// is the only thing stated.
const AMBIGUOUS_UNASKED_INTERPOLATION: [&str; 1] = ["issue10339_reduced.pdf page 1"];

/// Ambiguous because a gradient band is one shading and five anti-aliasings of its edges.
///
/// `issue6769_no_matrix.pdf` page 1 is 100 × 100 and draws one diagonal band filled with an
/// axial shading running blue to teal. Its name is the corpus telling you what it is for: the
/// pattern states no `/Matrix`, so Table 75's default — the identity — is the whole test.
///
/// ```text
/// mupdf 16.7318 │ ours 16.7509 │ hayro 16.8162 │ poppler 17.0354 │ ghostscript 18.401
/// ```
///
/// **Ours is 0.019 of 255 from `mupdf` and 0.065 from `hayro`**, and the mean absolute difference
/// per pixel says the same more sharply: 0.00029 against `mupdf` and 0.00028 against `hayro`,
/// which on a 100 × 100 page is the same band in the same place. So the default matrix is not in
/// question; what is left is the edges of a diagonal band, which is the group's usual subject.
///
/// Step 6 separates the two renderers above us, and they are not the same case:
///
/// ```text
///                72 dpi    576 dpi
/// ours (1x/8x)  16.7509   16.7852
/// poppler       17.0354   16.8227
/// ghostscript   18.4010   17.2192
/// ```
///
/// **Ours and `poppler` end 0.0375 of 255 apart** — converging from opposite sides, which is this
/// bucket's signature for a scan-conversion difference. **`ghostscript` does not converge**: it
/// sheds 1.18 of its 1.65 excess and keeps 0.43 at eight times the resolution, so it is drawing a
/// larger band rather than a differently-sampled one. That residue is a fact about `ghostscript`
/// and about this page, and it is the reason the page is `ambiguous` rather than agreeing: one
/// renderer out of five is far enough away to widen the bound past what the other four need.
const AMBIGUOUS_GRADIENT_BAND_EDGES: [&str; 1] = ["issue6769_no_matrix.pdf page 1"];

/// Ambiguous because two references clip a soft mask with an axis-aligned box.
///
/// `issue16742.pdf` page 1 is **one command**: a 200 × 200 green square, rotated ten degrees,
/// filled under an `/ExtGState` whose `/SMask` is a `/Luminosity` mask. It came off
/// `doc/todo/00`'s ranking at 0.04 from the nearest reference and **19.88 from the furthest**,
/// which is the widest spread in the bucket, and the five split cleanly in two:
///
/// ```text
/// mupdf 49.3966 │ hayro 49.4025 │ ours 49.4034 │ ghostscript 64.638 │ poppler 65.7891
/// ```
///
/// **Three inside 0.007 of 255 and two painting a third more ink.** No ladder is needed, because
/// the disagreement is where the green *stops*, and one column profile says which of the two
/// answers the standard states.
///
/// The mask group's bounding box is `/BBox [-20 -20 80 220.00002]` — a strip a hundred units wide
/// — while its content fills white over the whole 0…200 square. So the mask is white inside that
/// strip and, §11.6.5.1:
///
/// > Outside the transparency group's bounding box, the mask value shall be derived by
/// > transforming the BC colour to luminosity and applying the transfer function to the result.
///
/// with Table 142's `/BC` absent and its default "the colour space's initial value, representing
/// black" — luminosity 0, so nothing is painted there. And §8.10.2 puts the box in the *form's*
/// coordinate space, which here is the rotated one: the `gs` that sets the mask is executed after
/// `0.9848077 0.17364818 -0.17364818 0.9848077 0 0 cm`.
///
/// The right-hand edge of the green, by row:
///
/// ```text
///                row 50   row 100   row 150   row 199   max column
/// ours              72       63        54        45         78
/// mupdf             72       63        54        46         78
/// hayro             72       63        54        46         78
/// ghostscript       82       82        82        82         82
/// poppler           83       83        83        83         83
/// ```
///
/// **Three renderers draw a sloping edge and two draw a vertical one.** The slope is
/// 27 columns over 149 rows, which is 0.181 — the tangent of ten degrees, the rotation the file
/// states. And 0.9848 × 80 = **78.8**, which is where a boundary at x = 80 in the rotated space
/// lands: our maximum column exactly. The two vertical edges sit at 82 and 83, and the device-space
/// *bounding box* of the rotated `/BBox` reaches 0.9848 × 80 + 0.17365 × 20 = **82.2** — so those
/// two renderers clip the mask by the axis-aligned bound of the box rather than by the box.
///
/// This is `CLAUDE.md` principle 5 running in the direction it is written in: the answer was
/// derived from §11.6.5.1 and §8.10.2, and the agreement of `mupdf` and `hayro` is evidence that
/// the reading is right rather than the reason for it.
const AMBIGUOUS_ROTATED_MASK_BOUNDING_BOX: [&str; 1] = ["issue16742.pdf page 1"];

/// Ambiguous because a bilevel image enlarged four times has no right answer at its edges.
///
/// `jbig2_symbol_offset.pdf` page 1 draws one thing: a **132 × 14** one-bit `JBIG2Decode` image at
/// 19 ppi, stretched across an A4 page — an enlargement of about 3.8, with `/Interpolate` false,
/// so §8.9.5.3 asks for no smoothing and every renderer still has to decide what a source sample
/// straddling four device pixels looks like. It sat third on `doc/todo/00`'s ranking at 0.46 from
/// the nearest reference.
///
/// ```text
/// poppler 2.54071 │ ghostscript 2.56736 │ mupdf 2.60065 │ ours 2.60406 │ hayro 2.72227
/// ```
///
/// **Five renderers inside 0.18 of 255** on a page whose whole ink is 2.6, and ours between
/// `mupdf` and `hayro`. Step 6 says the shape is agreed:
///
/// ```text
///                72 dpi    576 dpi
/// poppler       2.54071   2.51204
/// ours (1x/8x)  2.59969   2.49495
/// ```
///
/// **The two ladders end 0.017 of 255 apart.** Ours descends 0.105 as the pixels shrink where
/// `poppler` descends 0.029, which is the measurement of what this group is about: at the page's
/// own scale we put slightly more ink into the enlarged sample edges, and by 8× there is nothing
/// left to put it in.
///
/// One reference is missing from the strip entirely — four panels, not five — which is a fact
/// about that renderer's `JBIG2Decode` and not about this page.
const AMBIGUOUS_ENLARGED_BILEVEL: [&str; 1] = ["jbig2_symbol_offset.pdf page 1"];

/// Ambiguous because four hairline borders on a 198 × 204 page are four hairlines.
///
/// `issue18072.pdf` page 1 is **4 commands** — four widget appearance streams, each a rectangle
/// with a one-unit border — on a page 198 × 204 device pixels at its own scale. It came off
/// `doc/todo/00`'s ranking at 0.36 from the nearest reference and 4.04 from the furthest, and the
/// two outliers are at opposite ends:
///
/// ```text
/// hayro 7.30672 │ poppler 8.44043 │ ours 8.5938 │ mupdf 8.61215 │ ghostscript 10.9632
/// ```
///
/// **A spread of 3.66 of 255 with ours in the middle of it**, 0.02 from `mupdf`. `ghostscript`
/// draws half again as much ink as `hayro` on the same four rectangles, which is the range of
/// answers a one-unit border admits when a unit is one pixel.
///
/// Step 6, three ladders:
///
/// ```text
///                72 dpi    576 dpi
/// poppler       8.44043   8.60487
/// ours (1x/8x)  8.55167   8.62365
/// mupdf         8.61215   8.62643
/// ```
///
/// **All three limits inside 0.022 of 255, and ours 0.003 from `mupdf`'s.** The rectangles are the
/// right rectangles; what differs at the page's own scale is a stroke a pixel wide landing between
/// pixel centres — `doc/todo/_scan-conversion.md`'s subject, on the smallest page in this file.
///
/// A second fact worth recording because it makes a mean incomparable: the five rasters are 197,
/// 198 wide and 203 or 204 tall for the same page. **A page whose device height falls between two
/// integers is rounded differently by different renderers**, so an ink *mean* is taken over a
/// different number of rows — 0.5% of the denominator here. That is trap 12's shape one step
/// earlier than usual, and the reason step 6's ladders are quoted from rasters of identical size.
const AMBIGUOUS_HAIRLINE_BORDERS: [&str; 1] = ["issue18072.pdf page 1"];

/// Ambiguous, and both pages reach this gate's judgement for the first time.
///
/// Two pages that state §11.4.7's `/Group << /S /Transparency /CS /DeviceCMYK >>` and were
/// *reported* for it until the four-hundred-and-fortieth session, which found the report was a
/// soft mask's group being counted as a change to the page's blending space (ADR 0276). A page
/// this tree reports is excused this gate's diagnosis, so both arrived here with no reading
/// behind them — and the first thing to establish about each is that the blending space is
/// **not** what makes it ambiguous.
///
/// That is measurable rather than arguable, because the same page can be drawn both ways: this
/// tree's ink route against the device-components route it replaces, RMSE over the same raster,
/// beside what two references differ from each other by on the same page.
///
/// ```text
///                                    ours in ink vs ours on the device   poppler vs ghostscript
/// issue13520                                            0.0144                   0.0736
/// bug1703683_page2_reduced                              0.0018                   0.0229
/// ```
///
/// **A fifth and a thirteenth of the references' own disagreement.** So each page needs its own
/// reading, and each has one.
///
/// # `bug1703683_page2_reduced.pdf`, where two ladders agree and the third does not
///
/// 612 × 792, four product photographs and 5-point captions. Step 6's ladders:
///
/// ```text
///                72 dpi    576 dpi
/// poppler       5.42909   5.36945
/// mupdf         5.22151   5.22575
/// ours (1x/8x)  5.36769   5.35913
/// ```
///
/// Ours moves by **0.009 of 255 across an eightfold change** — a ladder that does not move is one
/// already measuring the geometry — and its limit is **0.010** from `poppler`'s. `mupdf`'s is
/// **0.144** below both, and it is the whole reason no consensus forms. The difference image says
/// where that 0.144 is: subtract our render from either reference and every lit pixel is on the
/// outline of a photograph or a glyph, with the interiors black. So this is the boundary of every
/// mark, at 1.60% of the page's pixels, and §10.7.4 hands a partly covered pixel to the device.
///
/// # `issue13520.pdf`, where no two of the five draw the same picture
///
/// 208 × 89, one glossy button drawn as `Screen`-blended groups over a `DeviceN` shading.
/// **`poppler` draws none of the white highlights at all** and `hayro` paints a dark disc over
/// the right-hand bulge where the others paint a highlight; `ghostscript` outlines the whole
/// shape in a rough dark line. Ours and `mupdf` are the two that draw the artwork, and step 6
/// cannot arbitrate the rest:
///
/// ```text
///                72 dpi    576 dpi
/// poppler      17.6287   17.4397
/// mupdf        16.5747   16.6970
/// ours (1x/8x) 16.8109   16.9811
/// ```
///
/// The two ladders end **0.74 of 255 apart and move in opposite directions**, which is
/// `doc/todo/00` step 6's own definition of neither having converged. Ours ends between them,
/// 0.28 from one and 0.46 from the other. A page where one reference omits a whole class of mark
/// is `ambiguous` by the verdict's definition and is listed rather than ranked.
///
/// # `bug1721218_reduced.pdf`, the same shape one scope down (session 492)
///
/// The page states no group at all; what it draws is one isolated `/CS /DeviceCMYK` **form**
/// group holding the whole artwork — a product photograph built from hundreds of nested
/// groups, shadings and gray soft masks — which ADR 0327 composites in ink as a pair of its
/// own instead of reporting (§11.6.6). So this page too reaches the gate's judgement for the
/// first time, and the first thing to establish is again that the group's space is not what
/// makes it ambiguous. Mean |difference| over the whole page, all five renders pairwise, in
/// levels of 255:
///
/// ```text
/// ours vs poppler   0.116      poppler vs mupdf         0.303
/// ours vs hayro     0.147      poppler vs ghostscript   0.324
/// ours vs mupdf     0.241      mupdf   vs ghostscript   0.328
/// ours vs ghostscript 0.367    mupdf   vs hayro         0.130
/// ```
///
/// **Ours sits nearer to `poppler` than any two references sit to each other**, and every
/// pixel that differs from the nearest reference by more than 8 levels lies inside the
/// artwork's own rectangle (x 242–461, y 374–522 of 612×792) — where `ghostscript` draws the
/// chassis' perforation texture as distinct dots and saturated ports while the other four
/// soften both, which is a sub-pixel-artwork reduction question and not a colour-space one.
/// Four renderers, four flattenings of the same transparency stack, no consensus to hold
/// anybody to.
const AMBIGUOUS_PAGE_DRAWN_IN_INK: [&str; 3] = [
    "bug1703683_page2_reduced.pdf page 1",
    "bug1721218_reduced.pdf page 1",
    "issue13520.pdf page 1",
];

/// Ambiguous because a barcode is a page of bars narrower than a pixel.
///
/// `issue8187.pdf` page 1 is 200 × 50 and holds one Code-39 barcode and nothing else. It sat
/// second on `doc/todo/00`'s ranking at 0.54 from the nearest reference, and the ink says at once
/// that the difference is not how much is drawn:
///
/// ```text
/// ours 34.3200 │ poppler 34.3095 │ hayro 34.2825 │ mupdf 34.2105 │ ghostscript 26.7750
/// ```
///
/// **Four renderers inside 0.11 of 255** — ours and `poppler` inside 0.011 — and one 7.5 below
/// them all. So the area is agreed and the *distribution* is not, which is what a mean-per-pixel
/// metric measures and an ink measurement cannot see.
///
/// The per-column profile is the instrument for a page of vertical bars, and it is the row
/// profile of `doc/todo/00`'s step 6 turned ninety degrees:
///
/// ```text
/// mean |ours - poppler| per column   5.76
/// mean |mupdf - poppler| per column  5.25
/// mean |gs - poppler| per column    29.46
/// ```
///
/// **Our column-wise disagreement with `poppler` is the same size as another C renderer's**, and
/// the columns say what the disagreement is: ours reads a flat 211 across the bar field where
/// `poppler` alternates 181 and 237. That is the difference between spreading a sub-pixel bar
/// across the pixels it covers and snapping it to one — §10.7.4's subject, and
/// `doc/todo/_scan-conversion.md` is where this tree's choice is written down.
///
/// Step 6 closes it, and it closes `ghostscript` too:
///
/// ```text
///                 72 dpi     576 dpi
/// ours (1x/8x)    34.3200    34.3277
/// poppler         34.3095    34.3022
/// ghostscript     26.7750    33.4687
/// ```
///
/// **The two ladders end 0.026 of 255 apart.** `ghostscript` climbs 6.7 of its 7.5 deficit as the
/// pixels shrink, which is the proof that its outlier at the page's own scale is a scan-conversion
/// choice — thin bars given a minimum width and less ink than they cover — rather than a different
/// barcode. Its rasters were taken without a crop flag and are the same 1600 × 400 as the other
/// two, which is the check `doc/todo/00`'s step 6 asks for before any number is believed.
const AMBIGUOUS_SUB_PIXEL_BARS: [&str; 1] = ["issue8187.pdf page 1"];

/// Ambiguous because five renderers set two thousand rows of five-point text.
///
/// `issue1905.pdf` is a one-page poster — *TDF NUMBERS*, seven charts, three 3-D pie charts and a
/// paragraph of licence text at four points — 1247 × 1984 at the page's own scale and 2731
/// commands. It sat at the head of `doc/todo/00`'s ranking at **0.42 from the nearest reference
/// and 1.24 from the furthest**, which is the shape step 1 calls *we are alone*.
///
/// It is not. Every one of the five draws it, and the ink at the page's own scale is:
///
/// ```text
/// ours 51.4731 │ hayro 51.6848 │ mupdf 52.1001 │ poppler 52.2025 │ ghostscript 52.4447
/// ```
///
/// **A spread of 0.97 of 255 across the references alone**, with ours 0.21 below the lowest of
/// them. The per-row profile says the same from the other end: the mean row-by-row difference over
/// all 1984 rows is **1.87 of 255 between ours and `poppler`, and 2.16 between `ghostscript` and
/// `poppler`** — so this tree is closer to `poppler` than another C renderer is, and the rows that
/// differ most are the licence paragraph (1861, 1885) and the masthead (56), which are the rows
/// with the smallest type on the page.
///
/// Step 6 closes it:
///
/// ```text
///             1x/72dpi   4x/288dpi   8x/576dpi
/// ours        51.4731    51.5939     51.6171
/// poppler     52.2025    51.8299     51.8933
/// ```
///
/// **Both ladders converge and their limits are 0.25 of 255 apart**, against a reference-to-
/// reference spread four times that. The marks are the right marks; what is left is where the
/// edges of five-point glyphs land, on a page that is almost entirely five-point glyphs. Same
/// argument as `AMBIGUOUS_GLYPH_SCAN_CONVERSION`'s and `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE`'s, and
/// its own group because it is neither a specimen nor a book: it is one sheet carrying both, and
/// the number that identifies it is the *reference* spread rather than ours.
const AMBIGUOUS_DENSE_CHART_POSTER: [&str; 1] = ["issue1905.pdf page 1"];

/// Ambiguous because Table 179 names ten shapes and gives not one dimension.
///
/// `issue13447.pdf` is a page of Acrobat markup — sticky notes, squares, a circle, two lines and
/// two polylines — and two of its lines state `/LE [/None /ClosedArrow]` over a six-unit and a
/// three-unit border. **The clause states the shape and not the size** (ADR 0192), so every
/// renderer chooses one, and the page is the measurement of what four of them chose. The
/// arrowhead on the six-unit line, in device pixels of its bounding box at the page's own scale:
///
/// ```text
/// ours          33 wide    720 blue pixels     4 x the line width
/// poppler       49 wide   1304                ~7.2 x
/// ghostscript   61 wide   1344                ~9 x
/// mupdf         63 wide   2015                ~9.5 x
/// hayro         — nothing at all —
/// ```
///
/// **Four answers spanning a factor of 2.4, and a fifth renderer that draws no ending**, on a
/// shape whose only stated property is "[t]wo short lines meeting in an acute angle … connected
/// by a third line". That is `doc/todo/00`'s shape 3 in its purest form: the specification
/// determines the shape, determines nothing about its extent, and this tree's four-widths is a
/// choice recorded as one beside the artwork — the same construction §12.5.6.10's marks use,
/// where the quadrilateral's own height is the only length the annotation gives.
///
/// The page arrived in this bucket **because it was fixed**: it was on the corpus's incomplete
/// list until the three-hundred-and-fourteenth session, so the oracle did not judge it, and
/// drawing the endings is what let it be judged. Ink 24.13 → 24.81 of 255 against the three C
/// renderers' 25.5 to 25.8 and `hayro`'s 23.93 — we moved 45% of the way to them by drawing the
/// arrowheads, and the rest is their size.
///
/// **And the same page found a defect a size could not explain.** Both of its arrowed lines
/// state a `/Rect` that does not contain their own `/L` — `[598.31 146.63 537 316.13]` against a
/// line at x ≈ 177 — and a constructed appearance was clipped to `/Rect`, so this tree drew
/// *neither* the line nor its ending and said nothing. ADR 0193.
const AMBIGUOUS_LINE_ENDING_SIZE: [&str; 1] = ["issue13447.pdf page 1"];

/// Ambiguous, and §9.6.2.2 states fourteen *names* and not one outline.
///
/// `standard_fonts.pdf` is the sheet: fourteen pages setting specimen text in Times, Helvetica,
/// Courier, Symbol and ZapfDingbats and every weight and slope of them, with **not one font
/// program embedded**. So the page is what each renderer's copy of the standard 14 looks like,
/// and nothing else.
///
/// §9.6.2.2 names them and then puts the artwork beyond itself in one sentence:
///
/// > PDF processors supporting PDF 1.0 to PDF 1.7 files shall have these fonts, or their font
/// > metrics and suitable substitution fonts, available.
///
/// Two routes, either acceptable, and neither of them a set of outlines the standard states.
/// This tree takes the first: `data/standard-fonts/` holds §9.6.2.2's fourteen programs, 804 KB
/// of PDFium's Foxit faces compiled in, so these pages reproduce on a machine with no fonts
/// installed (ADR 0133). The three C references read URW's from this machine's disk and `hayro`
/// answers from its own built-in. **Five renderers, four sets of outlines, one clause that
/// requires none of them.**
///
/// The ink says how much that is worth on page 7 — `poppler` 15.506, `mupdf` 16.402, ours
/// 17.487, `ghostscript` 18.374, `hayro` 20.518, a spread of **5.0 of 255**, a third of the
/// page — and the pairwise matrix says who is alone, which is nobody:
///
/// ```text
///              poppler   mupdf   ghostscript   hayro
/// ours          0.0360  0.0243      0.0265    0.0190
/// poppler          —    0.0241      0.0274    0.0315
/// mupdf                    —        0.0147    0.0222
/// ghostscript                          —      0.0182
/// ```
///
/// Ours against `hayro` is the second-smallest pair of the ten and `poppler` is the furthest
/// from everybody, including from the two references that share `FreeType` with it. All fourteen
/// pages sit in one band — mean 4.72 to 7.09, worst tile 16.73 to 23.71, ssim 0.8592 to 0.9052 —
/// which is the tolerance's design applied to a page whose whole subject is a typeface.
///
/// **Not `AMBIGUOUS_SUBSTITUTED_FACE`**, which is §9.8.1's *other* route: a face nobody embedded
/// and nobody standardised, ranked out of whatever this machine holds. Here the clause names the
/// font and we have it; what differs is whose drawing of it.
/// # A fifteenth page, from a different document, in the two-hundred-and-sixty-fourth
///
/// `ZapfDingbats.pdf` sets eight fonts and **every one of them is a standard 14 with no program
/// embedded** — Helvetica twice, Times three times, Courier-Bold and ZapfDingbats. So it is this
/// sheet's subject arriving from a document that is not a specimen sheet, and the ladders say the
/// same thing: `poppler` climbs onto 16.0883 and `mupdf` onto 16.0992 — 0.011 apart, so there is
/// a limit — while ours is flat at 15.35 to 15.49, **0.60 of 255 below it**. That is 3.7% of the
/// page's ink, and it is the difference between PDFium's Foxit outlines compiled into this binary
/// and URW's read off this machine's disk. Neither is the clause's, because the clause states
/// none.
/// # A sixteenth, and it is eleven characters on a page the size of a postage stamp
///
/// `issue16473.pdf` is 176 × 40 device pixels: one widget, a `0.5 0.5 149 21 re s` border and
/// `(Hello ) Tj (World) Tj` at `/Helv 12 Tf`, with Helvetica **not embedded**. So the page's mean
/// is eleven glyphs and a rectangle, and whose Helvetica draws them is most of it.
///
/// ```text
///                1x       2x       4x       8x       16x
/// ours         14.9153  18.1384  18.2702  18.3156  18.2232
/// poppler      18.0950                    18.5724 (576 dpi)
/// mupdf                                   18.6207 (576 dpi)
/// ```
///
/// The whole of our climb is between 1× and 2× — eleven glyphs on a forty-pixel page is where
/// scan conversion costs most — and from there ours is flat at 18.2 to 18.3 against a limit the
/// two references put at 18.57 and 18.62. **0.3 of 255 under, at every resolution**, which is a
/// systematic difference in the outlines rather than in the pixels: this is the sheet's subject
/// arriving through a widget's appearance stream. `ghostscript` draws 22.05 and `hayro` 15.72 at
/// the page's own scale, a spread of 6.3 on eleven characters.
const AMBIGUOUS_STANDARD_FOURTEEN_FACE: [&str; 16] = [
    "ZapfDingbats.pdf page 1",
    "issue16473.pdf page 1",
    "standard_fonts.pdf page 1",
    "standard_fonts.pdf page 10",
    "standard_fonts.pdf page 11",
    "standard_fonts.pdf page 12",
    "standard_fonts.pdf page 13",
    "standard_fonts.pdf page 14",
    "standard_fonts.pdf page 2",
    "standard_fonts.pdf page 3",
    "standard_fonts.pdf page 4",
    "standard_fonts.pdf page 5",
    "standard_fonts.pdf page 6",
    "standard_fonts.pdf page 7",
    "standard_fonts.pdf page 8",
    "standard_fonts.pdf page 9",
];

/// A markup annotation with an appearance stream, and three renderers answering three
/// questions.
///
/// One page, and it is the first this file has taken from `doc/corpora/pdf20examples` —
/// the PDF Association's own seven-file demonstration of ISO 32000-2, which the oracle
/// reached for the first time in the six-hundred-and-ninety-second session (ADR 0541).
///
/// `pdf20examples/PDF 2.0 UTF-8 string and annotation.pdf` is a blank sheet carrying one
/// `/Highlight` annotation. It has an `/AP` whose `/N` draws
/// `1.0 1.0 0.0 rg 100 200 300 36 re h f` — a yellow rectangle, no border — and a
/// `/QuadPoints [100 200 400 200 400 236 100 236]`.
/// **The file explains itself in its own comments**, which is why it is in this corpus — a
/// `%` comment in the document rather than anything the standard says, so it is set here as
/// what it is:
///
/// ```text
/// % The QuadPoints array here conforms to 32000-2 and therefore acts strange
/// % in readers that do not conform to the standard.
/// % Use the XXAcroOrderQuadPoints array for QuadPoints if you want
/// % implementation compatibility rather than specification conformance.
/// ```
///
/// Ink at the page's own scale, in levels of 255: ours **0.820296**, `mupdf` **0.820296**,
/// `hayro` **0.820296**, `poppler` 4.81698, `ghostscript` 0. Three renderings to four
/// significant figures identical and two others nowhere near, so no two of the three voting
/// references agree and the verdict is `ambiguous` — which is the instrument saying it has
/// nothing to hold us to, and is exactly right here. What the page is worth is the reason
/// each of the other two differs, because each has one and they are different clauses:
///
/// - **`poppler` draws a bow-tie.** It synthesises the mark from `/QuadPoints` instead of
///   drawing the `/AP`, and reads the four vertices in Acrobat's historical order rather
///   than Table 182's, which specifies "the quadrilateral's four vertices in
///   counterclockwise order". The file ships the other order commented out under the name
///   `XXAcroOrderQuadPoints` and says what it is for. Under the conforming order read as
///   Acrobat's, the polygon crosses itself — which is the shape on the raster.
/// - **`ghostscript` draws nothing, and is answering a different question.** The annotation
///   states no `/F`, so Table 167's Print flag is clear, and `gs` renders for a printer.
///   Trap 3.
///
/// **We are on the side §6.3.2.2 names**, and it is the sentence `CLAUDE.md` quotes as one of
/// a rendering processor's three obligations:
///
/// > A PDF processor shall also render the appropriate appearance stream for all annotations
/// > (12.5.5, "Appearance streams") which have appearance streams designated for this purpose
/// > as indicated by the annotation flags (see 12.5.3, "Annotation flags"), unless otherwise
/// > instructed.
///
/// That is a statement about the clause and not about the vote: agreeing with `mupdf` here is
/// evidence our reading is right, and it would remain the reading if `mupdf` changed its mind.
const AMBIGUOUS_HIGHLIGHT_APPEARANCE_STREAM: [&str; 1] =
    ["pdf20examples/PDF 2.0 UTF-8 string and annotation.pdf page 1"];

/// Ambiguous, and the bound every one of them fails is one no pair of the references meets here
/// either.
///
/// **The whole of `doc/corpora/pdfbox`'s undiagnosed queue but four pages**, opened in the
/// six-hundred-and-ninety-fourth session (ADR 0543). The six-hundred-and-ninety-second put this
/// corpus through the oracle for the first time and 63 pages came back `ambiguous` with nothing
/// said about them — the first entries in `tests/ambiguous_undiagnosed.txt` since the
/// three-hundred-and-seventy-ninth session, and not one of them a regression.
///
/// Nineteen documents: `pdfbox/cweb.pdf` (22 pages, pdfTeX, embedded Computer Modern Type 1
/// subsets),
/// PDFBox's `input/merge/` inputs (26 pages of Acrobat Distiller Word output at US letter),
/// `eu-001.pdf`, `unencrypted.pdf` page 1, three single pages and the four
/// `PDFBox.GlobalResourceMergeTest` names below. Every one is judged by
/// [`Tolerance::TEXT_HEAVY`] — mean 5.00, worst tile 40.00, differing 5.00%, similarity 0.9000.
///
/// # Four names, one page, and the check that says so
///
/// `doc/todo/00`'s rule is to ask what else in the list is the same file before taking a name off
/// it. `PDFBox.GlobalResourceMergeTest.Doc01.pdf`, `…Doc02.pdf` and the `.decoded` variant of each
/// print **the same four metrics to the digit**, and they are one page: our own render is
/// `md5 e0a91939…` on all four and `poppler`'s is `3d197096…` on all four. `Doc01` and `Doc02` are
/// both 9 511 bytes and both `.decoded` files 14 336, so the pairs differ in stream filters and the
/// two documents in nothing this gate can see. Their `pdftotext` readback is one digest and is not
/// empty — *Lorem ipsum dolor sit amet* — so the match is evidence rather than
/// `doc/todo/00`'s false positive of two pages with no text. **One measurement, four names.**
///
/// # Which bound fails, because the hypothesis it was filed under was wrong about two of the four
///
/// The queue was written down with a shape attached: *62 of the 63 fail the differing fraction and
/// the structural similarity while sitting well inside the mean and the worst tile.* Counted off
/// the gate's own lines, the first half holds and the second does not:
///
/// ```text
///   differing fraction   63 of 63 fail
///   structural similarity 47 of 63
///   mean                  47 of 63   — the hypothesis said none
///   worst tile             4 of 63   — the four in [`AMBIGUOUS_PAGE_PLACED_A_ROW_APART`]
/// ```
///
/// The differing fraction is also the *worst* ratio on all 59 of these, at 2.3 to 3.2 times its
/// bound where the mean never reaches 2.6 and the worst tile never reaches 0.8. So the page that
/// has to be accounted for is the differing fraction, and the rest follows it.
///
/// # And that bound is one the references miss against each other on every page here
///
/// Measured over the artefacts the gate has already written, with `raster_compare`'s own
/// definition — channels differing by more than four levels, over width × height × four:
///
/// ```text
///   the smallest of the three reference-to-reference differing fractions
///     exceeds the 5.00% bound on   63 of 63 pages      range 5.11% to 14.27%
///   our worst differing fraction is at or below the largest of them on 51 of 63
///   median: ours 11.22%   reference-to-reference 11.48%
/// ```
///
/// **No two of the three voting references agree to this bound anywhere in this population**, so a
/// page failing it is not an accusation. That is `doc/oracle-and-corpus.md` §3c's result — the
/// six-hundred-and-seventh session re-derived all eight fixed bounds and found this one rejecting
/// **29.4%** of reference pairs on text pages, alone among the four — reproduced on a corpus
/// written by different producers and never used to set it. `doc/todo/12` owns the bound; ADR 0243
/// has the argument for leaving it where it is, and this group is not a request to move it.
///
/// The mean converts the same way: **on 61 of 63 our worst mean is at or below the largest
/// reference-to-reference mean**, median ours 6.37 against 7.90. The two that are not are
/// `PDFBOX-5840-410609.pdf` pages 1 and 2 (4.64 against 4.42, 6.63 against 5.81), which is the
/// document whose four faces are §9.6.2.2's standard fourteen with no program embedded — the
/// subject of [`AMBIGUOUS_STANDARD_FOURTEEN_FACE`], arriving inside this band rather than beside
/// it.
///
/// # Two camps, measured over this population instead of assumed from the other one
///
/// All ten renderer pairs, over all 63 pages, in levels of 255:
///
/// ```text
///   the closest of the ten pairs is ours + hayro on                63 of 63
///   median ours-to-hayro                                            1.538
///   median closest *voting* pair                                    4.226
///   we sit nearer a voting reference than the closest two vote-
///     ing references sit to each other on                          62 of 63
/// ```
///
/// That is trap 9's third shape and `doc/todo/00`'s table — on the pdf.js bucket the same two
/// medians are 1.94 and 5.39 — holding to within a tenth on a population it was not measured on.
/// **It is not evidence that we are right**, and `Reference::independence` says why: `hayro`
/// shares `skrifa` with this tree and may not vote. What it establishes is what the verdict is
/// made of, which is a voting camp 2.75 times wider than the gap between us and the renderer that
/// abstains.
///
/// # Whose line the gate prints, and the placement underneath it
///
/// On an ambiguous page the printed comparison is against the reference we look *least* like — the
/// largest worst tile — and here that is **`poppler` on 53 of the 63 and `ghostscript` on 10**.
/// Never `mupdf`, and the reason is one measurement: the best whole-pixel offset between our
/// raster and `mupdf`'s is **(0, 0) on all 63 pages**, while the best offset between ours and
/// `poppler`'s is **one device row down on 50 of them**. `poppler` places this text a row from
/// where we and `mupdf` place it; taking that row out drops ours against `poppler` from 9.23 to
/// 7.56 on `cweb.pdf` page 4 and from 14.28 to 3.95 on the pages in the group below.
///
/// # The closed form, on two documents and with two ladders each
///
/// `doc/todo/00` step 6, with the gate's own arguments to each renderer — `-dTextAlphaBits=4` is
/// not optional, and without it `ghostscript`'s 72 dpi ink on `poems-beads` reads 16.57 instead of
/// the 18.25 the gate compares against, which is trap 3 inside a ladder.
///
/// ```text
///   cweb.pdf page 4                72 dpi / 1x     576 dpi / 8x
///     ours                          14.1503         14.2305
///     poppler                       14.1894         14.2245
///     mupdf                         14.1936         14.2461
///     ghostscript                   14.9504         14.2615
/// ```
///
/// **Two independent limits 0.022 of 255 apart, and ours at 8× lies between them.** What is
/// 0.73 out at the page's own scale is `ghostscript`, against its own high-resolution value —
/// scan conversion of nine-point Computer Modern, not a disagreement about the marks.
///
/// ```text
///   PDFBOX-5840-410609.pdf page 3  72 dpi / 1x     576 dpi / 8x
///     ours                          22.9489         22.8568
///     poppler                       22.6507         22.7155
///     mupdf                         22.7579         22.7390
/// ```
///
/// Here the two references' limits agree to 0.024 and **ours is 0.13 above them at every scale**,
/// which is a difference in outlines rather than in pixels: §9.6.2.2's *these fonts, or their font
/// metrics and suitable substitution fonts* names Times and states no artwork, this tree draws
/// PDFium's Foxit faces compiled in (ADR 0133) and the three C references read URW's off this
/// machine. Five renderers, four sets of outlines, one clause that requires none of them.
///
/// # What the specification determines
///
/// `doc/todo/00`'s second and third shapes, on two halves of the population. **Where the fonts are
/// embedded** — `cweb.pdf`'s Computer Modern subsets, `PDFBOX-3110`'s Helvetica subsets,
/// `PDFBOX-5811-362972`'s Century, `unencrypted.pdf`'s CID TrueType — §10.7.4 determines the marks
/// and every renderer here is departing from it in its own direction at 72 dpi, ours least at the
/// limit and between the two references that converge. **Where they are not** — the `merge/`
/// documents' `TimesNewRomanPSMT` and `ArialMT`, `PDFBOX-5840-410609`'s standard fourteen — §9.5
/// NOTE 5 puts the answer beyond the standard in as many words:
///
/// > some details of font naming, font substitution, and glyph selection are
/// > implementation-dependent and can vary among different PDF processors and operating system
/// > environments
///
/// so there is no artwork to be right about, and the ladders say what the choice costs rather than
/// who is wrong. Neither half is a defect and neither is a licence: what holds these pages is this
/// list, and a page that stops being ambiguous fails the build.
const AMBIGUOUS_TEXT_AT_DOCUMENT_SIZE: [&str; 59] = [
    "pdfbox/cweb.pdf page 10",
    "pdfbox/cweb.pdf page 11",
    "pdfbox/cweb.pdf page 12",
    "pdfbox/cweb.pdf page 16",
    "pdfbox/cweb.pdf page 18",
    "pdfbox/cweb.pdf page 19",
    "pdfbox/cweb.pdf page 2",
    "pdfbox/cweb.pdf page 20",
    "pdfbox/cweb.pdf page 21",
    "pdfbox/cweb.pdf page 22",
    "pdfbox/cweb.pdf page 23",
    "pdfbox/cweb.pdf page 24",
    "pdfbox/cweb.pdf page 26",
    "pdfbox/cweb.pdf page 27",
    "pdfbox/cweb.pdf page 28",
    "pdfbox/cweb.pdf page 3",
    "pdfbox/cweb.pdf page 4",
    "pdfbox/cweb.pdf page 5",
    "pdfbox/cweb.pdf page 6",
    "pdfbox/cweb.pdf page 7",
    "pdfbox/cweb.pdf page 8",
    "pdfbox/cweb.pdf page 9",
    "pdfbox/eu-001.pdf page 1",
    "pdfbox/eu-001.pdf page 2",
    "pdfbox/eu-001.pdf page 3",
    "pdfbox/PDFBOX-3042-003177-p2.pdf page 1",
    "pdfbox/PDFBOX-3044-010197-p5-ligatures.pdf page 1",
    "pdfbox/PDFBOX-3062-002207-p1.pdf page 1",
    "pdfbox/PDFBOX-4417-001031.pdf page 1",
    "pdfbox/PDFBOX-4417-001031.pdf page 2",
    "pdfbox/PDFBOX-4417-001031.pdf page 3",
    "pdfbox/PDFBOX-4417-054080.pdf page 1",
    "pdfbox/PDFBOX-5762-722238.pdf page 2",
    "pdfbox/PDFBOX-5762-722238.pdf page 3",
    "pdfbox/PDFBOX-5762-722238.pdf page 4",
    "pdfbox/PDFBOX-5762-722238.pdf page 5",
    "pdfbox/PDFBOX-5762-722238.pdf page 6",
    "pdfbox/PDFBOX-5792-240045.pdf page 1",
    "pdfbox/PDFBOX-5792-240045.pdf page 2",
    "pdfbox/PDFBOX-5792-240045.pdf page 3",
    "pdfbox/PDFBOX-5792-240045.pdf page 4",
    "pdfbox/PDFBOX-5792-240045.pdf page 5",
    "pdfbox/PDFBOX-5792-240045.pdf page 6",
    "pdfbox/PDFBOX-5809-509329.pdf page 1",
    "pdfbox/PDFBOX-5809-509329.pdf page 2",
    "pdfbox/PDFBOX-5811-362972.pdf page 1",
    "pdfbox/PDFBOX-5811-362972.pdf page 2",
    "pdfbox/PDFBOX-5811-362972.pdf page 3",
    "pdfbox/PDFBOX-5811-362972.pdf page 4",
    "pdfbox/PDFBOX-5840-410609.pdf page 1",
    "pdfbox/PDFBOX-5840-410609.pdf page 2",
    "pdfbox/PDFBOX-5840-410609.pdf page 3",
    "pdfbox/PDFBOX-5840-410609.pdf page 4",
    "pdfbox/PDFBOX-5840-410609.pdf page 5",
    "pdfbox/PDFBox.GlobalResourceMergeTest.Doc01.decoded.pdf page 1",
    "pdfbox/PDFBox.GlobalResourceMergeTest.Doc01.pdf page 1",
    "pdfbox/PDFBox.GlobalResourceMergeTest.Doc02.decoded.pdf page 1",
    "pdfbox/PDFBox.GlobalResourceMergeTest.Doc02.pdf page 1",
    "pdfbox/unencrypted.pdf page 1",
];

/// Ambiguous, and the only four pages in `doc/corpora/pdfbox` that fail the *worst tile* —
/// because one reference draws the page a device row from where the other two draw it.
///
/// `pdfbox/PDFBOX-3110-poems-beads.pdf` and `…-cropbox.pdf` are the same two pages of Quartz output
/// twice, the second with a `/CropBox` inset ten points on each side. Their fonts are **embedded**
/// TrueType subsets of Helvetica and Helvetica-Oblique, so nobody here is substituting anything.
/// They sit apart from [`AMBIGUOUS_TEXT_AT_DOCUMENT_SIZE`] because they are the four of that
/// round's 63 whose worst tile fails — 48.33, 49.97, 40.90 and 42.39 against a bound of 40.00 —
/// and because the reason is legible and is not a bound's design.
///
/// # The offset, and it is a whole row
///
/// Best whole-pixel offset between our raster and each reference's, by mean absolute difference
/// over the common area, at the page's own scale (`poems-beads` page 1):
///
/// ```text
///   ours vs mupdf         2.87  best offset (0, 0)          no improvement available
///   ours vs hayro         2.04  best offset (0, 0)
///   ours vs poppler      13.33  best offset (0, +1) → 3.77  a 72% reduction
///   poppler vs mupdf     13.80  best offset (0, −1) → 2.78
/// ```
///
/// The ink bounding boxes say it a second way: `poppler` 62..714, ours 63..714, `ghostscript`
/// 63..714, `mupdf` 63..715 — one row up, on a page whose `/MediaBox` height is **841.89** and
/// where every renderer rasterises 842 rows. Where the 0.11 of a row left over goes is the
/// question `CLAUDE.md` names as one the standard answers nowhere: *how a fractional page becomes
/// a whole number of pixels*. On a sheet of nine-point type in 5-pixel glyph bodies, one row is
/// every baseline, which is why the worst tile fails here and on no other page of this corpus.
///
/// # And the fourth renderer is not the offset, it is its own scale
///
/// The gate's printed line for all four of these is against `ghostscript`, whose worst tile is
/// larger still. With the gate's own arguments, two ladders and ours:
///
/// ```text
///   poems-beads page 1     72 dpi / 1x   288 / 4x   576 dpi / 8x
///     ours                   16.1173     16.1288     16.1555
///     poppler                16.0831     16.1666     16.1853
///     mupdf                  16.0772     16.1657     16.1820
///     ghostscript            18.2534     16.3101     16.7208
/// ```
///
/// **`poppler` and `mupdf` converge to within 0.0033 of 255 of each other, so this page has a
/// limit**, and ours at 8× is 0.028 under it. `ghostscript` at the page's own scale is **2.07
/// above its own 8× value** — 12.8% of the page's ink — which is `-dTextAlphaBits=4` on small
/// type and not a claim about the marks. So of the four failing numbers the gate prints, the
/// mean and the worst tile are `ghostscript`'s scale-dependent excess and the offset above; §10.7.4
/// determines the marks and the two ladders that converge say what they are.
const AMBIGUOUS_PAGE_PLACED_A_ROW_APART: [&str; 4] = [
    "pdfbox/PDFBOX-3110-poems-beads-cropbox.pdf page 1",
    "pdfbox/PDFBOX-3110-poems-beads-cropbox.pdf page 2",
    "pdfbox/PDFBOX-3110-poems-beads.pdf page 1",
    "pdfbox/PDFBOX-3110-poems-beads.pdf page 2",
];

/// The ambiguous pages that carry a written diagnosis, as one list.
///
/// Held exactly like the contradicted groups, and for the same reason: which group a page
/// belongs to is a hypothesis about it, so the groups ratchet together and only the total
/// fails the build.
fn diagnosed_ambiguous() -> Vec<&'static str> {
    AMBIGUOUS_SHARED_JBIG2_DECODER
        .iter()
        .chain(&AMBIGUOUS_IMAGE_REDUCTION)
        .chain(&AMBIGUOUS_DEVICE_CMYK_CONVERSION)
        .chain(&AMBIGUOUS_STROKE_ADJUSTMENT)
        .chain(&AMBIGUOUS_FUNCTION_SAMPLED_BY_A_REFERENCE)
        .chain(&AMBIGUOUS_ZERO_AREA_FILL)
        .chain(&AMBIGUOUS_TILING_CELL_CLIP)
        .chain(&AMBIGUOUS_SUB_PIXEL_LINE_WORK)
        .chain(&AMBIGUOUS_SUBSTITUTED_FACE)
        .chain(&AMBIGUOUS_IRREVERSIBLE_JPEG_2000)
        .chain(&AMBIGUOUS_COLOUR_OPERANDS)
        .chain(&AMBIGUOUS_CALRGB_TO_SCREEN)
        .chain(&AMBIGUOUS_DEVICE_N_ALTERNATE)
        .chain(&AMBIGUOUS_LINE_ENDING_SIZE)
        .chain(&AMBIGUOUS_EIGHT_BIT_COMPOSITING)
        .chain(&AMBIGUOUS_WIDGET_BORDER)
        .chain(&AMBIGUOUS_RADIAL_CONE)
        .chain(&AMBIGUOUS_LINK_BORDER)
        .chain(&AMBIGUOUS_MARKUP_ARTWORK)
        .chain(&AMBIGUOUS_ICON_ARTWORK)
        .chain(&AMBIGUOUS_GLYPH_COVERAGE)
        .chain(&AMBIGUOUS_GRADIENT_QUANTISATION)
        .chain(&AMBIGUOUS_OUTLINED_TEXT)
        .chain(&AMBIGUOUS_TILED_STROKES)
        .chain(&AMBIGUOUS_REFERENCE_DREW_NOTHING)
        .chain(&AMBIGUOUS_DEGENERATE_GLYPH_BOX)
        .chain(&AMBIGUOUS_UNCOLOURED_GLYPH_PROCEDURE)
        .chain(&AMBIGUOUS_REFUSED_EMBEDDED_FONT)
        .chain(&AMBIGUOUS_LOCA_OUT_OF_ORDER)
        .chain(&AMBIGUOUS_SPACE_DRAWN_AS_A_MARK)
        .chain(&AMBIGUOUS_NEAREST_THE_GEOMETRY)
        .chain(&AMBIGUOUS_FOUR_ANSWERS)
        .chain(&AMBIGUOUS_BOUNDARY_PIXELS)
        .chain(&AMBIGUOUS_INSIDE_A_ROUNDING_ERROR)
        .chain(&AMBIGUOUS_OURS_ON_THE_LIMIT)
        .chain(&AMBIGUOUS_TRANSFER_FUNCTION_UNAPPLIED)
        .chain(&AMBIGUOUS_KNOCKOUT_GROUP)
        .chain(&AMBIGUOUS_TABLE_RULE_EDGES)
        .chain(&AMBIGUOUS_ONE_LADDER)
        .chain(&AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY)
        .chain(&AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE)
        .chain(&AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE)
        .chain(&AMBIGUOUS_JPEG_COMPONENT_IDS)
        .chain(&AMBIGUOUS_RECOVERED_PAGE_TREE)
        .chain(&AMBIGUOUS_STANDARD_FOURTEEN_FACE)
        .chain(&AMBIGUOUS_HIGHLIGHT_APPEARANCE_STREAM)
        .chain(&AMBIGUOUS_TEXT_AT_DOCUMENT_SIZE)
        .chain(&AMBIGUOUS_PAGE_PLACED_A_ROW_APART)
        .chain(&AMBIGUOUS_TRANSPARENCY_GROUP)
        .chain(&AMBIGUOUS_MASKED_BLUR)
        .chain(&AMBIGUOUS_SUBTRACTIVE_MASK_GROUP)
        .chain(&AMBIGUOUS_MATTE_WITHOUT_A_SOFT_MASK_IMAGE)
        .chain(&AMBIGUOUS_NON_ISOLATED_POSTER)
        .chain(&AMBIGUOUS_STACKED_SCREEN_UNDER_MASKS)
        .chain(&AMBIGUOUS_GRADIENT_ON_A_TIGHT_BOUND)
        .chain(&AMBIGUOUS_OVERSIZED_BORDER)
        .chain(&AMBIGUOUS_CONSTRUCTED_WIDGET)
        .chain(&AMBIGUOUS_GLYPH_SCAN_CONVERSION)
        .chain(&AMBIGUOUS_DENSE_CHART_POSTER)
        .chain(&AMBIGUOUS_SUB_PIXEL_BARS)
        .chain(&AMBIGUOUS_ENLARGED_BILEVEL)
        .chain(&AMBIGUOUS_ROTATED_MASK_BOUNDING_BOX)
        .chain(&AMBIGUOUS_GRADIENT_BAND_EDGES)
        .chain(&AMBIGUOUS_UNASKED_INTERPOLATION)
        .chain(&AMBIGUOUS_HAIRLINE_BORDERS)
        .chain(&AMBIGUOUS_PAGE_DRAWN_IN_INK)
        .chain(&AMBIGUOUS_ICC_MATRIX_PROFILE)
        .chain(&AMBIGUOUS_A_REFERENCE_DECODED_THE_IMAGE_WRONG)
        .chain(&AMBIGUOUS_DIVIDED_CONSENSUS)
        .copied()
        .collect()
}

/// Every group of pages in this file carries a diagnosis, and it is a diagnosis of *those*
/// pages.
///
/// # The defect it exists for, which happened three times before anybody noticed
///
/// A group is an array of page names with the argument about them in the doc comment above it.
/// Rust attaches a doc comment to whatever item follows it — so an edit that inserts a new
/// `const` between an existing comment and the const it documented silently welds two notes
/// together and leaves an array with none. It is invisible to `rustc`, to `clippy` and to every
/// gate in this tree, because nothing here is wrong: the comment is well formed and the array
/// compiles.
///
/// It had happened to [`AMBIGUOUS_GLYPH_COVERAGE`], [`AMBIGUOUS_MASKED_BLUR`] and
/// [`AMBIGUOUS_OURS_ON_THE_LIMIT`] — seven pages between them, each with its argument written
/// down and filed above a group it does not describe, and each group left silent. That is
/// `doc/todo/00`'s "a diagnosis that outlives what it diagnosed is this project's oldest
/// failure" in its other direction, and the handover's trap 1 one file over: **no gate can
/// check a comment**, so the one check that can be mechanised is worth having.
///
/// # What it checks, and why that rule and not a stricter one
///
/// For every non-empty `AMBIGUOUS_*`, `CONTRADICTED_*`, `NO_RENDER_*`, `NOT_COMPARABLE_*` and
/// `REFERENCE_GEOMETRY_*` array: the doc comment
/// above it names
/// at least one of the documents in it. That is deliberately weak. A group of 370 pages cannot
/// name them all, several notes cite a *neighbouring* group's page on purpose to say how the
/// two differ, and a rule that forbade either would be a rule this file has to fight. What the
/// weak rule catches is the whole of the failure above: a welded comment names none of the
/// array under it, because it was written about the array above it.
///
/// An empty array is exempt and keeps its note — `AMBIGUOUS_SUBTRACTIVE_MASK_GROUP` is empty
/// because its page left the bucket, and the argument for why is the reason to keep the entry.
///
/// # Why it reads the source text
///
/// A doc comment is not visible to the program it documents, so there is nothing else to read.
/// `include_str!` of this file's own path is exact and costs nothing at run time; the parse is
/// deliberately literal — a `const NAME: [&str; N] = [` line, the string literals under it, and
/// the run of `///` lines immediately above.
#[test]
fn every_group_of_pages_carries_a_diagnosis_naming_one_of_them() {
    let source = include_str!("oracle.rs");
    let lines: Vec<&str> = source.lines().collect();
    let mut silent: Vec<(&str, usize)> = Vec::new();
    let mut groups = 0usize;

    for (index, line) in lines.iter().enumerate() {
        let Some(name) = group_name(line) else {
            continue;
        };
        let members = group_members(&lines, index);
        if members.is_empty() {
            continue;
        }
        groups += 1;
        let comment: String = lines[..index]
            .iter()
            .rev()
            .take_while(|above| above.trim_start().starts_with("///"))
            .copied()
            .collect::<Vec<&str>>()
            .join("\n");
        if !members.iter().any(|document| comment.contains(document)) {
            silent.push((name, index + 1));
        }
    }

    assert!(
        groups > 50,
        "the parse found only {groups} groups, which is fewer than this file holds — \
         the check is reading the source wrongly rather than the file being small"
    );
    assert!(
        silent.is_empty(),
        "these groups' doc comments name none of their own pages, which is what a comment \
         welded onto the group above looks like — move the diagnosis back over the array it \
         is about:\n{}",
        silent
            .iter()
            .map(|(name, line)| format!("  {name} at line {line}"))
            .collect::<Vec<String>>()
            .join("\n")
    );
}

/// The name of the group a line declares, or `None` where the line declares no group.
fn group_name(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("const ")?;
    let name = rest.split(':').next()?;
    (name.starts_with("AMBIGUOUS_")
        || name.starts_with("CONTRADICTED_")
        || name.starts_with("NO_RENDER_")
        || name.starts_with("NOT_COMPARABLE_")
        || name.starts_with("REFERENCE_GEOMETRY_"))
    .then_some(name)
}

/// The documents named in the array declared at `index`, without their page numbers.
///
/// Reads forward to the line ending the array rather than counting brackets, because every
/// declaration in this file is either one line or a list of one string literal per line.
fn group_members<'a>(lines: &[&'a str], index: usize) -> Vec<&'a str> {
    let mut members = Vec::new();
    for line in &lines[index..] {
        for piece in line.split('"').skip(1).step_by(2) {
            if let Some(document) = piece.split(" page ").next()
                && Path::new(document)
                    .extension()
                    .is_some_and(|ext| ext == "pdf")
            {
                members.push(document);
            }
        }
        if line.trim_end().ends_with("];") {
            break;
        }
    }
    members
}

/// The ambiguous pages nobody has diagnosed, one name per line.
///
/// # Why this list exists at all
///
/// `ambiguous` means "no two references agree closely enough for anybody to be called
/// wrong". It is the right verdict for the *ratchet* to reach and it is not the same as
/// "right": 754 of the 1683 pages this gate judges land here, and until the
/// hundred-and-seventy-sixth session **no gate watched one of them in either direction**.
/// `issue7406.pdf` drew a JPEG cyan-on-black inside an ambiguous verdict for as long as
/// anybody looked, and it is correct now, and nothing announced either event.
///
/// So the bucket gets the instrument the contradicted pages have had since the sixth
/// session. A page leaves this file by becoming `agrees` or by joining an `AMBIGUOUS_*`
/// group with a diagnosis beside it; a page arriving in it is a page that used to agree,
/// which is a regression nobody would otherwise see. Both directions fail the build.
///
/// The list is data rather than a `const` because it was 754 names and the argument for each
/// one is that there is *no* argument yet — the diagnoses live in the groups above, where
/// they can be read. **It is empty as of the three-hundred-and-seventy-ninth session**, which
/// retires none of this: the gate still holds it to equality, so a page that stops agreeing
/// arrives here and fails the build on the arrival. Emptiness is the state, not the end of the
/// instrument.
fn undiagnosed_ambiguous() -> Vec<&'static str> {
    include_str!("ambiguous_undiagnosed.txt")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// How far our render sits from the references, as a multiple of the bound applied.
///
/// One number per end of the range, because the two answer different questions and §3a's
/// ranking needs both. `furthest` is the measurement the report prints and the one that
/// ranks a *contradiction*; on an ambiguous page it is dominated by whichever reference
/// drew the least like the page — a renderer that failed and produced a black rectangle
/// sits at 178 and says nothing about us. `nearest` is the one that accuses us: a page
/// where even the closest reference is far away is a page where we differ from everybody.
#[derive(Debug, Clone, Copy)]
struct Distance {
    nearest: f64,
    furthest: f64,
}

impl Distance {
    /// Over every voting reference, or `None` where nothing was measured.
    ///
    /// Each comparison is reduced to the largest of its three ratios against the bounds
    /// this page was held to, so that a page failing on structural similarity alone ranks
    /// beside one failing on mean error. The bounds are the page's own — derived from how
    /// far the references sit from each other — which is what makes the numbers comparable
    /// across pages at all.
    ///
    /// **Three and not four**, and since ADR 0643 that is an answered question rather than a
    /// standing debt: the differing fraction is left out because a hundred notes quote these
    /// figures, *and* because a run that put it in was measured and would have made the
    /// ordering worse. [`nearest_on_every_measure`] is the fourth measure printed beside this
    /// one, and carries both counts.
    fn of(triangulation: &pdfref::Triangulation) -> Option<Self> {
        let bounds = &triangulation.judged_by;
        let mut ratios = triangulation
            .ours
            .iter()
            .map(|(_, c)| outside_by_in_three_measures(c, bounds));
        let first = ratios.next()?;
        Some(ratios.fold(
            Self {
                nearest: first,
                furthest: first,
            },
            |d, r| Self {
                nearest: d.nearest.min(r),
                furthest: d.furthest.max(r),
            },
        ))
    }
}

/// What the oracle concluded about one document.
#[derive(Debug)]
enum Verdict {
    /// We agree with a consensus of at least two references.
    Agrees,
    /// The references agree with each other and contradict us. A defect.
    Contradicted(String),
    /// The references disagree among themselves. No answer to hold us to.
    Ambiguous(String),
    /// Our page size differs from the references' by more than rounding. A defect.
    OurGeometry(String),
    /// The references disagree about the page size. Not ours to answer.
    ReferenceGeometry(String),
    /// Fewer than two references produced an image, so nothing can be triangulated.
    NotComparable(String),
    /// We could not produce a page to compare. `corpus.rs` owns this outcome.
    NoRender(String),
}

impl Verdict {
    /// Short label for the summary line.
    fn label(&self) -> &'static str {
        match self {
            Self::Agrees => "agrees",
            Self::Contradicted(_) => "CONTRADICTED",
            Self::Ambiguous(_) => "ambiguous",
            Self::OurGeometry(_) => "GEOMETRY",
            Self::ReferenceGeometry(_) => "reference geometry",
            Self::NotComparable(_) => "not comparable",
            Self::NoRender(_) => "no render",
        }
    }

    /// What was measured, for the printed survey.
    fn detail(&self) -> &str {
        match self {
            Self::Agrees => "",
            Self::Contradicted(detail)
            | Self::Ambiguous(detail)
            | Self::OurGeometry(detail)
            | Self::ReferenceGeometry(detail)
            | Self::NotComparable(detail)
            | Self::NoRender(detail) => detail,
        }
    }
}

/// One page's result.
#[derive(Debug)]
struct Examined {
    name: String,
    verdict: Verdict,
    /// Whether we reported the page as fully drawn. Only complete pages are gated.
    complete: bool,
    /// How far we sit from the nearest and furthest reference, in bounds. `None` where the
    /// comparison never happened.
    distance: Option<Distance>,
    /// The same distance from the nearest reference over **all four** measures, which is the
    /// unit [`Self::consensus_missed_by`] is already in.
    ///
    /// It exists so that the two numbers on one printed line can be read against each other,
    /// and for nothing else — see [`nearest_on_every_measure`], which has the measurement that
    /// says why the mixed reading was not a ratio. [`Distance`] is deliberately *not* extended
    /// to four: its figures are quoted across this file and `doc/todo/00`, and a page recorded
    /// at "0.16 from the nearest reference" has to stay the number that was recorded.
    ///
    /// `None` on exactly the pages [`Self::distance`] is `None` on.
    nearest_on_every_measure: Option<f64>,
    /// How far the *closest pair of references* sits outside the bound, in multiples of it.
    ///
    /// `ambiguous` is the verdict for a page on which no two voting references agreed, and
    /// until the five-hundred-and-eighteenth session nothing said **by how much** they
    /// missed. That number is the one trap 9's fifth shape moves: two references sharing a
    /// decoder can manufacture the *absence* of a consensus, and where they do, this is
    /// large. Below 1 on an ambiguous page is impossible by construction; a little above 1
    /// is trap 12's arithmetic, and 20 is a renderer that failed.
    ///
    /// `None` where fewer than two references were compared with each other.
    consensus_missed_by: Option<f64>,
    /// The same, in [`Distance`]'s three measures rather than in four.
    ///
    /// It is what [`Self::distance`]'s `nearest` can be read against, and therefore what orders
    /// the queue `doc/todo/00` step 1 names: the pages we sit further from every reference on
    /// than the closest two references sit from each other. See
    /// [`consensus_missed_in_three_measures`] for why the pool needs both readings.
    ///
    /// `None` on exactly the pages [`Self::consensus_missed_by`] is `None` on.
    consensus_missed_in_three_measures: Option<f64>,
    /// Whether our nearest reference is outside the bound the *closest pair* would have set.
    ///
    /// The two fields above are both measured against the class floor, which is what
    /// `pdfref::decide` returns where no consensus formed. This is the same comparison against
    /// the bound a consensus at that pair's own spread would have applied — the criterion
    /// [`rank_the_pages_we_are_alone_on`] marks its head with, and the one a round diagnosing
    /// that list stops at. See [`outside_what_the_closest_pair_would_allow`].
    ///
    /// `None` on exactly the pages [`Self::consensus_missed_in_three_measures`] is `None` on.
    outside_what_the_closest_pair_would_allow: Option<bool>,
    /// Which measure, and against which renderers, each half of that ratio is taken on.
    ///
    /// `None` on exactly the pages [`Self::consensus_missed_in_three_measures`] is `None` on.
    /// See [`AloneOn`] for why a ratio whose two halves are anonymous cannot be diagnosed.
    alone_on: Option<AloneOn>,
    /// How far outside its own bound a contradicted page sits, in multiples of that bound.
    ///
    /// `None` on every verdict but `CONTRADICTED`, which is trap 11 rather than tidiness: on an
    /// `ambiguous` page no two references agreed, so the bound beside them decided nothing and a
    /// ratio against it would rank a quantity no verdict rests on. `--bin unpriced` draws its
    /// population the same way and for the same reason (ADR 0606).
    ///
    /// See [`outside_the_bound`] for what the number is made of and why it is a minimum, and
    /// [`worst_ratio`] for why the measure's name travels with it.
    outside_the_bound: Option<(f64, &'static str)>,
    /// How the voting reference the consensus excludes fares under that same consensus and bound.
    ///
    /// `None` where every voting reference is in the consensus, or where the excluded one
    /// abstained or never drew. See [`ExcludedReading`], which is trap 12's own control turned
    /// into something the gate prints — a count on one side and, since the
    /// eight-hundred-and-forty-fifth session, a **named population** on the other.
    excluded_reference: Option<ExcludedReading>,
    /// How far apart the rasters of the consensus that decided this page sit, and whether the
    /// bound it applied ever left the class floor.
    ///
    /// `None` on exactly the pages that carry no consensus at all. See [`ConsensusIdentity`] for
    /// what the question is and ADR 0774 for what the answer turned out to be.
    consensus_identity: Option<ConsensusIdentity>,
    /// What this page's verdict would be if the bound that forms a consensus were raised.
    ///
    /// `None` on every page the gate reached no comparison on. See [`RaisedFormation`] for the
    /// question — `doc/todo/12` item 1, the half ADR 0243 measured once and left — and
    /// [`a_raised_formation_bound`] for the census it feeds.
    raised_formation: Option<RaisedFormation>,
    /// How many references produced a raster of one colour on a page another one drew, and
    /// therefore took no part in the consensus — `pdfref::consensus_abstentions`.
    ///
    /// Counted for the census rather than for a ratchet: the pages it moves are held by name
    /// in the `not comparable` groups below, and what this number answers is the question a
    /// ratchet cannot, which is how large the population is that the rule can reach at all.
    abstentions: usize,
    /// Which available references produced no raster at all on this page, with their reasons.
    ///
    /// Empty on all but a handful of pages, and different from [`Self::abstentions`] in kind:
    /// an abstention is a reference that *drew* and was refused a vote, this is a reference
    /// that never drew. A page still judged on the two that remain is a comparison against a
    /// smaller population than the page beside it, and until ADR 0542 the gate printed the two
    /// identically — which is how a page can change verdict with every input unchanged and
    /// nothing in the output to say why.
    absent: Vec<String>,
    /// Every reference whose raster was one colour here, and what its own log said.
    ///
    /// The population `pdfref::consensus_abstentions` decides over, printed rather than
    /// ratcheted, and printed in **both** directions: the refusals the rule matched, and the
    /// distinct sentences of every flat sheet it did **not** match. That second list is what
    /// trap 11 asks for and what nothing here had — "list everything in this population that
    /// satisfies the condition and does not satisfy the question", and its converse. A round
    /// widening or narrowing `Reference::refusals` reads it off this gate's own output rather
    /// than off a grep over log files some earlier run left behind.
    flat_sheets: Vec<FlatSheet>,
    /// How many maximal agreeing sets of references the page carried — `pdfref::Consensus`.
    ///
    /// One on almost every page, and 0 where no two references agreed at all. More than one
    /// means agreement was not transitive here: `a` with `b` and `b` with `c` while `a` and
    /// `c` differ leaves two sets, neither contained in the other and neither a majority the
    /// other is not.
    consensuses: usize,
    /// Where those sets reach *different* verdicts about our render, what the set the gate did
    /// not take concluded.
    ///
    /// `None` on every page with one consensus, and on a page whose several consensuses say
    /// the same thing about us — which is the common case and the harmless one. Where it is
    /// `Some`, the page's verdict was settled by which set the enumeration reached first
    /// rather than by the page. ADR 0616.
    divided_by: Option<String>,
    /// Processor time spent in our own pipeline, and in the three external renderers.
    ///
    /// Summed across the run and reported, because "where does this gate's time go" is
    /// otherwise answered by intuition — and the intuitive answer, that three subprocesses
    /// must dominate a Rust render, is wrong here by a factor this measures.
    spent: Spent,
}

impl Examined {
    /// A page the gate reached no comparison on, so there is nothing to measure.
    ///
    /// The four early exits from [`examine`] differ only in their verdict, and writing the
    /// two `None`s out four times is how a field added later gets missed at one of them.
    fn unjudged(name: String, verdict: Verdict, complete: bool, spent: Spent) -> Self {
        Self {
            name,
            verdict,
            complete,
            distance: None,
            nearest_on_every_measure: None,
            consensus_missed_by: None,
            consensus_missed_in_three_measures: None,
            outside_what_the_closest_pair_would_allow: None,
            alone_on: None,
            outside_the_bound: None,
            excluded_reference: None,
            consensus_identity: None,
            raised_formation: None,
            abstentions: 0,
            absent: Vec::new(),
            flat_sheets: Vec::new(),
            consensuses: 0,
            divided_by: None,
            spent,
        }
    }
}

/// One reference whose raster on one page carried a single colour, and what it said about it.
///
/// A raster of one colour is a picture of nothing, and the question that decides whether it is
/// a *reading* of the page is which of two things it is: a page whose correct rendering is a
/// flat sheet, or a page this program did not draw. `pdfref::Testimony` is the evidence that
/// separates them where there is any, and this is how much of it the corpus supplies.
#[derive(Debug, Clone)]
struct FlatSheet {
    reference: Reference,
    /// The sentence in which the renderer says it did not draw what the page asked for, if it
    /// says one — `pdfref::Reference::refusals`.
    refusal: Option<String>,
    /// Otherwise, the distinct non-empty lines it did write, which is what a round auditing the
    /// condition has to read. Empty where the renderer said nothing at all.
    said: Vec<String>,
}

/// Wall-clock spent on one page, split by who spent it.
#[derive(Debug, Default, Clone, Copy)]
struct Spent {
    ours: std::time::Duration,
    references: std::time::Duration,
}

impl Spent {
    /// Both halves, which is what decides whether this page is the run's longest pole.
    fn total(self) -> std::time::Duration {
        self.ours.saturating_add(self.references)
    }
}

/// One page of one document: the unit this gate compares and ratchets.
#[derive(Debug, Clone)]
struct Work {
    path: PathBuf,
    /// One-based, as the specification and all three reference renderers number pages.
    page: u32,
    /// Which population it came from, or `None` for the pdf.js corpus and the
    /// specification PDFs — see [`Work::name`] for why those two carry no label.
    corpus: Option<&'static Corpus>,
}

impl Work {
    /// How this page is named in the report and in the ratchet lists.
    ///
    /// A page out of a submodule corpus carries its corpus's label; a page out of the pdf.js
    /// corpus or out of `doc/`'s specification PDFs carries none. That asymmetry is
    /// deliberate and is the reason this gate could take a second population at all: every
    /// `CONTRADICTED_*` and `AMBIGUOUS_*` list in this file names its pages by this string,
    /// and prefixing the ones already there would have been a thousand-line rename with no
    /// argument behind it.
    ///
    /// The label is not decoration either. Three of the 275 documents under `doc/corpora/`
    /// share a *file name* with one of the 974 — `attachment.pdf`, `rotation.pdf` and
    /// `IndexedCS_negative_and_high.pdf` — and only two of those three share their bytes.
    /// `pdfbox`'s `attachment.pdf` is a different document from pdf.js's, so an unlabelled
    /// name would have put two different pages in one ratchet entry.
    fn name(&self) -> String {
        let file = self.path.file_name().map_or_else(
            || self.path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        match self.corpus {
            Some(corpus) => format!("{}/{file} page {}", corpus.label, self.page),
            None => format!("{file} page {}", self.page),
        }
    }

    /// The directory this page's artefacts are written under, relative to the work root.
    ///
    /// Carries the label for the same reason [`Work::name`] does: two documents of one name
    /// from two corpora would otherwise overwrite each other's evidence, which is the failure
    /// the per-page directory already exists to prevent one level down.
    fn artefact_directory(&self) -> PathBuf {
        let stem = self.path.file_stem().unwrap_or_default().to_string_lossy();
        match self.corpus {
            Some(corpus) => PathBuf::from(corpus.label).join(stem.as_ref()),
            None => PathBuf::from(stem.as_ref()),
        }
    }

    /// What this page's own artefacts are named inside that directory.
    ///
    /// One name per page, so a document's pages cannot overwrite one another's evidence — every
    /// renderer writes to a fixed file name inside the per-page directory.
    fn case(&self) -> String {
        let stem = self.path.file_stem().unwrap_or_default().to_string_lossy();
        format!("{stem}-p{}", self.page)
    }
}

/// Where the references' answers are remembered between runs.
///
/// The references' answers do not change between runs, and asking them again is 95% of this
/// gate's cost — 1020 seconds of processor time against 46 of ours. See `pdfref::cache` for
/// the key, which is derived from the invocation itself so that a changed flag cannot be
/// answered from a render made under the old one, and for why a timeout is the one outcome it
/// refuses to remember.
///
/// `PDFREF_CACHE` names a directory, or `off` to ask the renderers again — which is how the
/// claim that the cache changes no verdict is checked over the whole corpus rather than only
/// on the fixture `pdfref`'s own tests use.
fn reference_cache() -> Cache {
    let default = Path::new(env!("CARGO_TARGET_TMPDIR")).join("pdfref-cache");
    match std::env::var("PDFREF_CACHE") {
        Ok(value) if value.eq_ignore_ascii_case("off") => Cache::disabled(),
        Ok(value) if !value.trim().is_empty() => Cache::at(value),
        _ => Cache::at(default),
    }
}

/// Which pages this run looks at.
///
/// # Why a filter exists on a gate whose whole point is the whole corpus
///
/// Because the loop between writing a feature and seeing which pages it moved is what
/// decides how much gets built, and the two halves of that loop want different things. The
/// gate wants every page, every time, and gets it — `PDFVIEWER_ORACLE_ONLY` is unset in CI
/// and a filtered run refuses to check the ratchets at all, because a list held to *equality*
/// over a subset would report every page the filter excluded as newly fixed.
///
/// What a filtered run is for is the other half: having implemented something that affects a
/// dozen documents, looking at those dozen without waiting for the other nine hundred. With
/// `pdfref::cache` in place the whole run is cheap enough that this is a convenience rather
/// than a necessity, which is the right order — the fast path is the complete one.
#[derive(Debug)]
struct Selection {
    /// A substring of the file name, or `None` for everything.
    pattern: Option<String>,
}

impl Selection {
    /// Reads `PDFVIEWER_ORACLE_ONLY`, whose value is a comma-separated list of substrings.
    fn from_environment() -> Self {
        Self {
            pattern: std::env::var("PDFVIEWER_ORACLE_ONLY")
                .ok()
                .filter(|value| !value.trim().is_empty()),
        }
    }

    /// The pattern, when the run is filtered at all.
    fn pattern(&self) -> Option<&str> {
        self.pattern.as_deref()
    }

    /// Whether this page is in the run.
    fn admits(&self, work: &Work) -> bool {
        let Some(pattern) = &self.pattern else {
            return true;
        };
        let name = work.name();
        pattern
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .any(|part| name.contains(part))
    }
}

/// Which pages of a document a population offers this gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sheets {
    /// Page one only.
    First,
    /// Every page.
    All,
}

/// One of the corpora pinned as a submodule under `doc/corpora/`.
///
/// # Why these are a table rather than a fifth line in [`work_items`]
///
/// Because the interesting field is [`Corpus::voted`], and it is a *decision per population*
/// rather than a property of the harness. `CLAUDE.md`'s two questions say the corpus and the
/// oracle answer robustness — "what share of the files that actually exist render correctly"
/// — and the oracle answers it by voting three independent implementations. That vote is
/// evidence only where the references are being asked a question the standard answers. Two of
/// these four populations are not such a question, for two different reasons, and ADR 0541
/// has the argument; the field is where the argument reaches the code.
#[derive(Debug)]
struct Corpus {
    /// The prefix its pages carry in the report and in the ratchet lists.
    label: &'static str,
    /// Its directory, relative to the repository root.
    directory: &'static str,
    /// Which pages of each document are compared.
    sheets: Sheets,
    /// Whether the references' vote is evidence about this population, and so whether the
    /// gate holds its pages or only the census reads them.
    voted: bool,
}

/// The four corpora `doc/corpora/` pins, and which of them the oracle votes on.
///
/// Each is a submodule — a pin rather than a copy, so nothing here is redistributed by this
/// tree and no licence question arises from naming one (`doc/third-party-data.md`,
/// ADR 0305). All four are optional: [`corpus_items`] yields nothing for one that is not
/// checked out, and the gate says so rather than failing.
const SUBMODULE_CORPORA: &[Corpus] = &[
    // Seven files the PDF Association publishes to demonstrate ISO 32000-2 itself, written
    // by the people who wrote the clauses. They are valid by construction and small, which
    // is exactly the population a vote is evidence about: three independent readers of a
    // clause agreeing on a file *built to exercise that clause* is the strongest form the
    // triangulation rule takes anywhere in this tree.
    Corpus {
        label: "pdf20examples",
        directory: "doc/corpora/pdf20examples",
        sheets: Sheets::All,
        voted: true,
    },
    // Apache PDFBox's own regression inputs. Every file is there because it broke a PDF
    // library once, which is the same reason the 974 are there — and, like them, they are
    // overwhelmingly *valid* documents that a reader got wrong rather than malformed ones
    // the standard says nothing about. `text_extraction.rs` has read this corpus since ADR
    // 0259 and no raster gate has ever held it.
    Corpus {
        label: "pdfbox",
        directory: "doc/corpora/pdfbox/pdfbox/src/test/resources/input",
        sheets: Sheets::All,
        voted: true,
    },
    // Three directories of `openpreserve/format-corpus`, and every file in all three is
    // *deliberately damaged*: 89 hand-built files carrying one structural defect apiece, 24
    // archival horrors, 54 crawled `.gov` documents that broke somebody else's software.
    //
    // `CLAUDE.md` is explicit that "[t]he standard describes *valid* files and says nothing
    // about the rest", and that is what disqualifies the vote here rather than any property
    // of the references: on a file whose cross-reference table is wrong there is no clause
    // for three programs to agree *about*, so their agreement is a fact about three recovery
    // heuristics and nothing else. `doc/oracle-and-corpus.md` §2b has the instrument this
    // population does have — every one of the 89 draws the same *Hello PDF-world!*, so the
    // corpus states its own expected value and needs no reference at all.
    Corpus {
        label: "format-corpus",
        directory: "doc/corpora/format-corpus",
        sheets: Sheets::First,
        voted: false,
    },
    // The PDF Association's `pdf-differences`, and ADR 0393 decided this one before this
    // gate could reach it: the files exist *because* implementations split on them, so the
    // references are the subject under test and a vote reads the answer off the very
    // programs the corpus was assembled to catch out. On six of its eighteen cases at least
    // one reference is wrong against the clause and on one of them two of the three are.
    Corpus {
        label: "pdf-differences",
        directory: "doc/corpora/pdf-differences",
        sheets: Sheets::All,
        voted: false,
    },
];

/// Every page one submodule corpus offers, or nothing when it is not checked out.
///
/// Walks subdirectories, which none of the other populations needs: `pdfbox`'s inputs are in
/// four directories and `format-corpus`'s in four more, and a flat read would have taken 40
/// of 64 and none of 167 while looking like it had taken them all.
fn corpus_items(corpus: &'static Corpus) -> Vec<Work> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    let mut directories = vec![root.join(corpus.directory)];
    while let Some(directory) = directories.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.is_dir() {
                directories.push(path);
            } else if path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
            {
                files.push(path);
            }
        }
    }
    files.sort();
    let mut items: Vec<Work> = files
        .par_iter()
        .flat_map(|path| {
            let last = match corpus.sheets {
                Sheets::First => 1,
                Sheets::All => page_count(path),
            };
            (1..=last)
                .map(|page| Work {
                    path: path.clone(),
                    page,
                    corpus: Some(corpus),
                })
                .collect::<Vec<_>>()
        })
        .collect();
    items.sort_by(|a, b| a.path.cmp(&b.path).then(a.page.cmp(&b.page)));
    items
}

/// Every page to compare.
///
/// **All pages of every corpus document, and page one of the specification PDFs in
/// `doc/`.** The pdf.js corpus holds these files because each one broke a reader once, and
/// a file reduced from a bug report does not always put the interesting page first —
/// comparing only page one asks 869 single-page documents everything they have and the
/// other 100 almost nothing. The specification PDFs are the opposite case: 1382 pages of
/// consistent typesetting from 14 files, 1023 of them from ISO 32000-2 alone, where page
/// 500 exercises what page 499 did. They stay at page one, where they still cover the
/// heaviest fonts and the largest page trees in the tree.
///
/// That is 1775 pages against 988, and it costs about twice the wall clock.
///
/// **And every page of the [`SUBMODULE_CORPORA`] the references are evidence about**, which
/// is a second population and a second denominator: the 974 are one project's bug reports
/// over fifteen years, and a gate over one population cannot say whether what it measures is
/// a property of this reader or of that project's taste in bugs. ADR 0541.
///
/// A document whose page count cannot be established still yields page one, so that "we
/// cannot open it" is reported by the gate rather than silently absent from it.
///
/// Returns `None` when the corpus submodule is absent, in which case the gate reports being
/// skipped rather than failing — the ratchet only means anything where the corpus is
/// present.
fn work_items() -> Option<Vec<Work>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let pdfs_in = |dir: PathBuf| -> Option<Vec<PathBuf>> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .ok()?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
            .collect();
        files.sort();
        Some(files)
    };

    let corpus = pdfs_in(root.join("doc/pdf.js/test/pdfs"))?;
    if corpus.is_empty() {
        return None;
    }
    let specifications = pdfs_in(root.join("doc")).unwrap_or_default();

    // Counting pages means opening every document, which is a second of work in parallel
    // against the eighty this gate spends in three external renderers.
    let mut items: Vec<Work> = corpus
        .par_iter()
        .flat_map(|path| {
            (1..=page_count(path))
                .map(|page| Work {
                    path: path.clone(),
                    page,
                    corpus: None,
                })
                .collect::<Vec<_>>()
        })
        .collect();
    items.extend(specifications.into_iter().map(|path| Work {
        path,
        page: 1,
        corpus: None,
    }));
    items.sort_by(|a, b| a.path.cmp(&b.path).then(a.page.cmp(&b.page)));
    for corpus in SUBMODULE_CORPORA.iter().filter(|corpus| corpus.voted) {
        items.extend(corpus_items(corpus));
    }
    Some(items)
}

/// How many pages a document has, or one when that cannot be established.
fn page_count(path: &Path) -> u32 {
    let Ok(bytes) = std::fs::read(path) else {
        return 1;
    };
    let Ok(document) = Document::open(bytes) else {
        return 1;
    };
    u32::try_from(pdf_model::Pages::new(&document).len())
        .unwrap_or(u32::MAX)
        .max(1)
}

/// Our own render of a page, with the two facts about it the comparison needs.
#[derive(Debug)]
struct OurRender {
    raster: Raster,
    /// Whether the interpretation reported nothing it was unable to draw.
    complete: bool,
    /// Whether the page drew any glyphs, which decides which bounds apply.
    has_text: bool,
}

/// Renders one page with our own pipeline, from the file's bytes.
///
/// The document is opened per page rather than once per file. That repeats the
/// cross-reference parse — a few milliseconds — and buys parallelism over pages rather than
/// only over documents, which matters because one corpus file has 352 of them and would
/// otherwise be the long pole of the whole run.
fn render_ours(work: &Work) -> Result<OurRender, String> {
    let path = work.path.as_path();
    let index = usize::try_from(work.page.saturating_sub(1)).unwrap_or(usize::MAX);
    let bytes = std::fs::read(path).map_err(|e| format!("unreadable: {e}"))?;
    let document = Document::open(bytes).map_err(|e| format!("will not open: {e}"))?;
    let page = pdf_model::Pages::new(&document)
        .get(index)
        .ok_or_else(|| format!("no page {}", work.page))?;
    let interpretation = pdf_model::interpret(&document, &page);
    let complete = interpretation.is_complete();
    // Whether glyphs *marked the page*, not whether we could name them. A page listing a
    // font it never uses is a vector page; a page of CJK from a subset with no `/ToUnicode`
    // is a text page even though nothing here can say what it says. The thirtieth session
    // changed this line, and the two entries below `CONTRADICTED_GLYPHS_JUDGED_AS_VECTOR`
    // are what it was costing.
    let has_text = interpretation.glyphs > 0;

    let list = interpretation.display_list;
    let target =
        TargetSpec::for_page(&list, SCALE, PIXEL_BUDGET).map_err(|e| format!("no target: {e}"))?;
    let raster = CpuRasterizer::new()
        .rasterize(&list, target)
        .map_err(|e| format!("will not rasterise: {e}"))?;
    Ok(OurRender {
        raster,
        complete,
        has_text,
    })
}

/// Compares one page against the references.
fn examine(work: &Work, work_root: &Path, available: &[Reference], cache: &Cache) -> Examined {
    let name = work.name();
    let case = work.case();
    let work_dir = work_root
        .join(work.artefact_directory())
        .join(format!("p{}", work.page));
    let mut spent = Spent::default();

    let OurRender {
        raster: mut ours,
        complete,
        has_text,
    } = {
        let started = Instant::now();
        let rendered = render_ours(work);
        spent.ours = started.elapsed();
        match rendered {
            Ok(rendered) => rendered,
            Err(detail) => {
                return Examined::unjudged(name, Verdict::NoRender(detail), false, spent);
            }
        }
    };

    let Rendered {
        rendered: mut references,
        absent,
    } = {
        let started = Instant::now();
        let rendered = render_references(work, &work_dir, available, cache);
        spent.references = started.elapsed();
        match rendered {
            Ok(references) => references,
            Err(detail) => {
                // This directory used to be deleted here, on the reasoning that a page with
                // fewer than two references has nothing to look at. It has exactly what a
                // reader of this bucket needs — whichever reference *did* draw, which the
                // cache has already written here, and our own page beside it — and it was the
                // one bucket whose evidence had to be re-rendered from scratch to diagnose,
                // which the five-hundred-and-seventy-ninth session paid for. Thirteen pages.
                let _ = pdfref::png_io::write(&work_dir.join(format!("{case}-ours.png")), &ours);
                return Examined::unjudged(name, Verdict::NotComparable(detail), complete, spent);
            }
        }
    };

    let outvoted = match reconcile(&mut ours, &mut references) {
        Ok(outvoted) => outvoted,
        Err(verdict) => {
            return Examined::unjudged(name, verdict, complete, spent);
        }
    };

    let tolerance = bounds_for(has_text);
    let testimony = what_they_said(&references, &work_dir);
    let triangulation = match pdfref::triangulate_with(
        &ours,
        &references,
        &testimony,
        &tolerance,
        Judgement::CORPUS,
    ) {
        Ok(triangulation) => triangulation,
        Err(e) => {
            let verdict = Verdict::NotComparable(format!("{e}"));
            return Examined::unjudged(name, verdict, complete, spent);
        }
    };

    let flat_sheets = flat_sheets(&references, &testimony);
    let verdict = verdict_of(&triangulation, outvoted.as_deref());
    let distance = Distance::of(&triangulation);
    let nearest_on_every_measure = nearest_on_every_measure(&triangulation);
    let consensus_missed_by = consensus_missed_by(&triangulation);
    let consensus_missed_in_three_measures = consensus_missed_in_three_measures(&triangulation);
    let outside_what_the_closest_pair_would_allow =
        outside_what_the_closest_pair_would_allow(&triangulation);
    let alone_on = AloneOn::of(&triangulation);
    let outside_the_bound = outside_the_bound(&triangulation);
    if matches!(verdict, Verdict::Agrees) {
        // Nothing to look at, and three thousand agreeing pages of PNGs is a gigabyte.
        let _ = std::fs::remove_dir_all(&work_dir);
    } else {
        leave_the_evidence(
            work,
            &work_dir,
            &ours,
            &mut references,
            &triangulation,
            cache,
        );
    }

    Examined {
        name,
        verdict,
        complete,
        distance,
        nearest_on_every_measure,
        consensus_missed_by,
        consensus_missed_in_three_measures,
        outside_what_the_closest_pair_would_allow,
        alone_on,
        outside_the_bound,
        excluded_reference: the_excluded_reference_under_the_same_bound(&triangulation),
        consensus_identity: the_consensus_that_decided_it(&triangulation, &tolerance),
        raised_formation: Some(RaisedFormation::of(&triangulation, &tolerance)),
        abstentions: triangulation.abstained.len(),
        absent,
        flat_sheets,
        consensuses: triangulation.consensuses.len(),
        divided_by: divided_by(&triangulation),
        spent,
    }
}

/// Which class of bounds a page is judged by.
///
/// Text sets the noise floor between independent rasterisers — glyph hinting differs between
/// implementations in a way flat fills do not — so a page carrying glyphs is judged by the
/// bounds measured on text pages.
fn bounds_for(has_text: bool) -> Tolerance {
    if has_text {
        Tolerance::TEXT_HEAVY
    } else {
        Tolerance::VECTOR
    }
}

/// Writes one page's evidence directory, with a fourth render in it for the eye.
///
/// `hayro` shares its font rasteriser, its deflate, its JPEG decoder and both new image codecs
/// with us, so its agreement is not evidence — `Reference::independence` says so and
/// `Reference::voting` keeps it out of the consensus. But a page the three references cannot
/// settle is exactly where a fourth reading helps, and this is the only one of the four written
/// in the same language, so a difference between it and us cannot be blamed on C.
///
/// Called only for a page worth looking at, which is what keeps it off the gate's critical path:
/// an agreeing page has its whole directory deleted instead.
fn leave_the_evidence(
    work: &Work,
    work_dir: &Path,
    ours: &Raster,
    references: &mut Vec<(Reference, Raster)>,
    triangulation: &pdfref::Triangulation,
    cache: &Cache,
) {
    if let Ok(raster) = cache.render(Reference::Hayro, &work.path, work.page, DPI, work_dir)
        && raster.width == ours.width
        && raster.height == ours.height
    {
        references.push((Reference::Hayro, raster));
    }
    let _ = report::write_artefacts(work_dir, &work.case(), ours, references, triangulation);
}

/// What each reference said while it drew, read out of the same directory its raster came from.
///
/// It can decide a verdict — `pdfref::consensus_abstentions`'s second route — so it is collected
/// for every reference that produced a raster rather than only for the flat ones: the rule asks
/// the question, and a caller that pre-filtered would be stating that rule a second time, in a
/// place no test of `pdfref`'s can see.
fn what_they_said(references: &[(Reference, Raster)], work_dir: &Path) -> Vec<pdfref::Testimony> {
    references
        .iter()
        .map(|(reference, _)| reference.testimony(work_dir))
        .collect()
}

/// Every reference whose raster is one colour, with whatever its own log said about that.
///
/// Read after the comparison rather than during it, because the population this describes is
/// the one `pdfref::consensus_abstentions` decides over, and it has to be the rasters that were
/// actually compared — reconciled to a common size, as they are by the time this is called. It
/// takes the *same* testimony the rule was given rather than re-reading the logs, so that the
/// audit cannot describe a different population from the one that was judged.
fn flat_sheets(
    references: &[(Reference, Raster)],
    testimony: &[pdfref::Testimony],
) -> Vec<FlatSheet> {
    references
        .iter()
        .filter(|(_, raster)| pdfref::is_uniform(raster))
        .filter_map(|(reference, _)| {
            let testimony = testimony
                .iter()
                .find(|given| given.reference() == *reference)?;
            let refusal = testimony.refusal().map(ToOwned::to_owned);
            let said = if refusal.is_some() {
                Vec::new()
            } else {
                let mut lines: Vec<String> = testimony
                    .text()
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(ToOwned::to_owned)
                    .collect();
                lines.sort();
                lines.dedup();
                lines
            };
            Some(FlatSheet {
                reference: *reference,
                refusal,
                said,
            })
        })
        .collect()
}

/// The two maximal consensuses that reach different conclusions about our render.
///
/// Since ADR 0617 such a page is `ambiguous`, and this is the sentence that says why: **both**
/// sets, each with what it concludes, because a page whose readings divide is a page a reader has
/// to see both readings of. Before that rule the first of the two was the verdict and the second
/// was invisible, which is ADR 0616's finding.
///
/// Written as a sentence rather than as a flag for the reason the group note gives: which
/// references form each set is the whole of what such a page is about.
fn divided_by(triangulation: &pdfref::Triangulation) -> Option<String> {
    let (taken, rival) = triangulation.divided()?;
    let describe = |consensus: &pdfref::Consensus| {
        let names: Vec<String> = consensus
            .references
            .iter()
            .map(|reference| format!("{reference}"))
            .collect();
        format!(
            "{} {}",
            names.join(" and "),
            if consensus.agrees_with_us() {
                "agree with us"
            } else {
                "contradict us"
            }
        )
    };
    Some(format!("{}, {}", describe(taken), describe(rival)))
}

/// How far the closest pair of references sits outside the bound, in multiples of it.
///
/// The minimum rather than a mean, because the question is whether *any* two of them came
/// close to agreeing: a consensus needs one pair and no more. `None` where fewer than two
/// references were compared with each other, which is a page the gate could not judge.
fn consensus_missed_by(triangulation: &pdfref::Triangulation) -> Option<f64> {
    triangulation
        .between_references
        .iter()
        .map(|(_, _, comparison)| outside_by(comparison, &triangulation.judged_by))
        .fold(None::<f64>, |best, missed| {
            Some(best.map_or(missed, |b: f64| b.min(missed)))
        })
}

/// [`consensus_missed_by`] in [`Distance`]'s unit — the closest pair over three measures.
///
/// The same minimum over the same pairs as [`consensus_missed_by`], reduced by
/// [`outside_by_in_three_measures`] instead of by [`outside_by`], so that it can be read against
/// [`Distance::nearest`] rather than against [`nearest_on_every_measure`].
///
/// # Why the pool needs both readings and not one
///
/// `doc/todo/00` step 1 asks which pages *we are alone on* — our nearest further away than the
/// closest pair of references are from each other — and the answer depends on which measures the
/// question is asked in. Both readings are in one unit and neither is a mixed one; what separates
/// them is `doc/todo/12`'s differing fraction, a bound the references miss by nearly as much as we
/// do. Over the complete ambiguous pool the four-measure reading names seven pages in ten, and the
/// three-measure reading names about one in fourteen — and it is the *three*-measure one that
/// reproduces the reading session 518 took by hand in levels of 255 over a smaller pool, 6.9%
/// against 7.1% (ADR 0643). So this is the number the queue is ordered by, and
/// [`nearest_on_every_measure`]'s stays printed beside it because a page that is alone on both is
/// saying something a page alone on one is not.
///
/// `None` where fewer than two references were compared with each other, which is exactly where
/// [`consensus_missed_by`] is `None`.
fn consensus_missed_in_three_measures(triangulation: &pdfref::Triangulation) -> Option<f64> {
    triangulation
        .between_references
        .iter()
        .map(|(_, _, comparison)| {
            outside_by_in_three_measures(comparison, &triangulation.judged_by)
        })
        .fold(None::<f64>, |best, missed| {
            Some(best.map_or(missed, |b: f64| b.min(missed)))
        })
}

/// Whether our nearest reference is outside the bound the closest *pair* would have set.
///
/// [`consensus_missed_in_three_measures`] says how far apart the closest two references are and
/// [`Distance::nearest`] how far we are from the nearest of them; both are measured against the
/// class floor, because `pdfref::decide` returns the floor unwidened where no consensus formed.
/// This asks the counterfactual the gate answers on every *other* page in the corpus: **had those
/// two references agreed closely enough to be a consensus, would the bounds they set have accepted
/// us?** [`Judgement::CORPUS`] widens a consensus's bounds to twice its members' own spread, so the
/// question is `Tolerance::widened_to` applied to the closest pair's comparison and our nearest
/// comparison measured against the result.
///
/// # Why this is the head of `rank_the_pages_we_are_alone_on` rather than a second ranking
///
/// The seven-hundred-and-fifty-first session required that list's numerator to be outside the
/// **floor**, on the argument that below it the nearest reference would have accepted the page. The
/// floor is where that argument stops: a page can be outside the floor while the references
/// themselves are further outside it than we are, and 751's own filter admits every page whose
/// ratio is above 1.0. This is the same argument taken to the bound the gate actually applies —
/// below it a consensus *at this pair's own spread* would have accepted us, so the page is alone
/// against a constant rather than against the references. ADR 0684.
///
/// **The ratio ≥ 2 is the readable sufficient condition and this is the exact one.** Both sides of
/// the printed ratio are a maximum over three normalised measures, so a page whose ratio reaches
/// [`Judgement::CORPUS`]'s factor is certainly outside the widened bound — our worst measure then
/// exceeds twice the pair's worst, which exceeds twice the pair's on that same measure. The
/// converse does not hold: our worst measure and theirs can be different measures, and there this
/// test fires where the ratio does not. That is why the gate prints it per page rather than leaving
/// a reader to divide.
///
/// `None` where fewer than two references were compared with each other, or where we were compared
/// with none — exactly where [`consensus_missed_in_three_measures`] and [`Distance::of`] are.
fn outside_what_the_closest_pair_would_allow(
    triangulation: &pdfref::Triangulation,
) -> Option<bool> {
    let bounds = &triangulation.judged_by;
    let (_, _, closest) = closest_reference_pair(triangulation)?;
    let widened = bounds.widened_to(closest, corpus_widening_factor());
    let nearest = triangulation
        .ours
        .iter()
        .map(|(_, comparison)| outside_by_in_three_measures(comparison, &widened))
        .fold(None::<f64>, |nearest, ours| {
            Some(nearest.map_or(ours, |n: f64| n.min(ours)))
        })?;
    Some(nearest > 1.0)
}

/// The two references [`consensus_missed_in_three_measures`] is the comparison between.
///
/// One implementation of the reduction, because three callers taking their own minimum over the
/// same list is how two of them eventually name different pairs — which is exactly the defect ADR
/// 0643 found in two *columns* of one printed line, one step further in.
fn closest_reference_pair(
    triangulation: &pdfref::Triangulation,
) -> Option<&(Reference, Reference, raster_compare::Comparison)> {
    let bounds = &triangulation.judged_by;
    triangulation.between_references.iter().min_by(|a, b| {
        outside_by_in_three_measures(&a.2, bounds)
            .partial_cmp(&outside_by_in_three_measures(&b.2, bounds))
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

/// The reference [`Distance::nearest`] is our comparison against.
///
/// The same reduction `Distance::of` folds, kept separately so that the *name* it discards can be
/// recovered without changing a number a hundred notes quote.
fn nearest_reference(
    triangulation: &pdfref::Triangulation,
) -> Option<&(Reference, raster_compare::Comparison)> {
    let bounds = &triangulation.judged_by;
    triangulation.ours.iter().min_by(|a, b| {
        outside_by_in_three_measures(&a.1, bounds)
            .partial_cmp(&outside_by_in_three_measures(&b.1, bounds))
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

/// Which measure, and against which renderers, each half of the *we are alone* ratio is taken on.
///
/// # The gap this fills, which is one level finer than ADR 0497's sixth criterion
///
/// [`Distance::nearest`] and [`consensus_missed_in_three_measures`] are each a **maximum over
/// three measures** and then a **minimum over comparisons**, and the first of those two reductions
/// throws away a name that [`worst_ratio`] keeps for the contradicted ranking. So a note could
/// price a page of [`rank_the_pages_we_are_alone_on`] as *our number is 1.83 and here is the
/// mechanism* without ever saying which of the three the 1.83 is — and **a mechanism that accounts
/// for a mean need not account for a structural similarity**. The seven-hundred-and-sixty-first
/// session found the readable form of it: `freeculture.pdf` page 1 and `copy_paste_ligatures.pdf`
/// are marked `[widened: outside]` below a ratio of 2 precisely because our worst measure and the
/// pair's are *different* measures, so the two halves of the printed ratio are not commensurable
/// even though they are in one unit.
///
/// `--bin unpriced` asks the same question of a contradicted page's failing bound and calls a note
/// that argues about another measure its sharpest rung; ADR 0675 is the standing instance — a
/// note whose mechanism predicted the mean to four decimals while the mean was a bound the page
/// *passes*. This is that question asked of a ranking rather than of a verdict, and it needs the
/// gate to print the answer because no per-page line carries it: the line prints our distance from
/// the consensus's **worst** member, and this ratio is taken against its **nearest**.
///
/// # Why the renderers travel with the measures
///
/// Every note on this list already names them — *2.58, the similarity, against `poppler`*, *the
/// divisor is `mupdf` + `ghostscript` at 0.45* — and until now those names were taken by hand off
/// `examples/compare_rasters`. A claim this tree can print is a claim `--bin quoted`'s rule applies
/// to, and one it cannot print is a claim that decays unwatched.
#[derive(Debug, Clone, Copy)]
struct AloneOn {
    /// The measure [`Distance::nearest`] is the ratio of, and the reference it is measured against.
    ours: (&'static str, Reference),
    /// The measure [`consensus_missed_in_three_measures`] is the ratio of, and the pair it is
    /// measured between.
    theirs: (&'static str, Reference, Reference),
}

impl AloneOn {
    /// `None` on exactly the pages [`consensus_missed_in_three_measures`] and [`Distance::of`] are.
    fn of(triangulation: &pdfref::Triangulation) -> Option<Self> {
        let bounds = &triangulation.judged_by;
        let (nearest, ours) = nearest_reference(triangulation)?;
        let (left, right, theirs) = closest_reference_pair(triangulation)?;
        Some(Self {
            ours: (worst_ratio_in_three_measures(ours, bounds).1, *nearest),
            theirs: (
                worst_ratio_in_three_measures(theirs, bounds).1,
                *left,
                *right,
            ),
        })
    }
}

/// The multiple of a consensus's own spread that [`Judgement::CORPUS`] still counts as agreement.
///
/// Read off the judgement this gate runs under rather than written down a second time: a number
/// stated in two places is a number that will disagree with itself, and this one is the whole of
/// [`outside_what_the_closest_pair_would_allow`]'s argument.
fn corpus_widening_factor() -> f64 {
    match Judgement::CORPUS {
        Judgement::RelativeToReferences { factor } => factor,
        // `Judgement::Absolute` widens by nothing, and a variant added later has to be read here
        // rather than assumed. Either way the honest reading of "not a widening" is 1.
        _ => 1.0,
    }
}

/// Our distance from the nearest reference over **all four** of [`Tolerance::accepts`]' measures.
///
/// [`Distance::nearest`] is the same quantity over three of them — the differing fraction is not
/// among them — so this is the number [`consensus_missed_by`] can be read against, and that is the
/// whole of what it is for.
///
/// # Why this is a second number and not a fourth measure inside [`Distance`]
///
/// The seven-hundred-and-thirty-seventh session priced [`Distance`]'s blindness for the ranking it
/// orders and handed the question on. Taken over the run that asked it, the answer is that the
/// fourth measure does not belong in that unit, and the evidence is two counts:
///
/// - **On the contradicted pool it changes no order anybody reads.** The ten pages
///   [`rank_the_contradicted`] prints are the same ten, in the same order, to the hundredth, under
///   either unit; and [`rank_the_contradicted_by_the_bound`] already prints that pool in four
///   measures beside it.
/// - **On the ambiguous pool it would replace the ordering rather than sharpen it.** The differing
///   fraction is the largest of the four ratios on 762 of the 804 complete ambiguous pages, so a
///   four-measure `Distance` would order that bucket by it alone — and `doc/todo/12`'s bound is the
///   reason it cannot: over the same pages our differing fraction sits at a median **2.08** times
///   the class bound against the closest reference pair's **1.96**, so the measure separates us
///   from the references by 6% at the middle of the population. Read as *we are alone*, the
///   three-measure comparison names **48** of the 804 and the four-measure one names **569**. A
///   signal that fires on seven pages in ten is not one.
///
/// # What it is for, then
///
/// [`rank_the_manufactured_ambiguity`] prints the closest pair's number and ours on one line and
/// its own comment asks a reader to compare them, and until this existed those two columns were
/// **different instruments** — the pair's over four measures, ours over three. That is ADR 0242's
/// defect, a printed line whose numbers are not the ones beside them, surviving in a pair of
/// columns rather than in one. Taking the printed columns as a ratio names **13** of the 804 as
/// pages we are alone on where the like-for-like three-measure reading names 48, so the mixed
/// reading was not a conservative version of either question but an answer to neither.
///
/// `None` where nothing was measured, which is exactly where [`Distance::of`] is `None`.
fn nearest_on_every_measure(triangulation: &pdfref::Triangulation) -> Option<f64> {
    triangulation
        .ours
        .iter()
        .map(|(_, comparison)| outside_by(comparison, &triangulation.judged_by))
        .fold(None::<f64>, |nearest, ratio| {
            Some(nearest.map_or(ratio, |n: f64| n.min(ratio)))
        })
}

/// How far outside its bound a contradicted page sits, and which measure that is.
///
/// `None` unless the verdict is a contradiction. On an `ambiguous` page no two references agreed,
/// so the bound printed beside them decided nothing and a ratio against it would rank a quantity
/// no verdict rests on — the population rule `--bin unpriced` states and applies (ADR 0606).
///
/// # Why the *smallest* of the sets, where a page carries more than one
///
/// Since ADR 0617 a verdict is one **every** maximal consensus reaches, so a contradicted page is
/// one every set rejects and the exemption it is granted is only as strong as the set that rejects
/// it least. Each set's own number is the largest [`outside_by`] over its members, taken against
/// **that set's** widened bounds, because a set's bound is derived from its own members' spread and
/// borrowing another's would price the page against a judgement nothing made. The page's number is
/// then the smallest of those: *even on the references' most forgiving reading, this page is this
/// far outside.*
///
/// On every page of the pool but one this is exactly the ratio the page's own printed line is made
/// of, because it carries one consensus and [`measurements`] folds over that same set with the same
/// function.
fn outside_the_bound(triangulation: &pdfref::Triangulation) -> Option<(f64, &'static str)> {
    if !matches!(triangulation.outcome, Outcome::Regression { .. }) {
        return None;
    }
    triangulation
        .consensuses
        .iter()
        .filter(|consensus| !consensus.agrees_with_us())
        .filter_map(|consensus| {
            triangulation
                .ours
                .iter()
                .filter(|(reference, _)| consensus.references.contains(reference))
                .map(|(_, comparison)| worst_ratio(comparison, &consensus.judged_by))
                .fold(None::<(f64, &'static str)>, |worst, ratio| {
                    Some(worst.map_or(ratio, |w| if w.0 >= ratio.0 { w } else { ratio }))
                })
        })
        .fold(None::<(f64, &'static str)>, |mildest, ratio| {
            Some(mildest.map_or(ratio, |m| if m.0 <= ratio.0 { m } else { ratio }))
        })
}

/// How the voting reference outside the consensus fares under that same consensus and bound.
///
/// Trap 12's own control — *put each reference where our render stands and ask what the sets it
/// is not a member of conclude about it* — reduced to two integers, so that the two questions
/// that have been asked of it are answers rather than citations.
#[derive(Debug)]
struct ExcludedReading {
    /// The voting reference the head consensus leaves out.
    reference: Reference,
    /// How many consensus members hold it outside the bound our own verdict rested on.
    outside_of: usize,
    /// How many members it was compared with.
    members: usize,
}

impl ExcludedReading {
    /// Whether the consensus would contradict the excluded reference as well.
    ///
    /// One member is enough, which is the same rule `verdict_of` applies to us: a page is
    /// contradicted when the *worst* comparison against the consensus is outside the bound.
    fn convicted(&self) -> bool {
        self.outside_of > 0
    }

    /// Whether every member of the consensus holds it outside — the stricter reading.
    ///
    /// ADR 0717's sentence is in this form ("outside the same bound against *both* members"), and
    /// ADR 0771's hand check found the strict population four pages smaller than the loose one.
    /// Both are printed, because a claim written in one of them cannot be checked against the
    /// other.
    fn outside_of_every_member(&self) -> bool {
        self.members > 0 && self.outside_of == self.members
    }
}

/// Reads [`ExcludedReading`] off the numbers the gate has already computed.
///
/// # What it is
///
/// No render and no comparison is added: `between_references` holds the excluded reference
/// against each consensus member, and `Consensus::judged_by` is the bound our own verdict rested
/// on. The set asked is the head of `consensuses`, which is the set `verdict_of` names on the
/// page's own line.
///
/// # Why the gate prints a count *and* a list
///
/// ADR 0717 measured this by hand over one population — the 32 pages convicted on the differing
/// fraction by `poppler` and `mupdf` — and found `ghostscript` outside the same bound on 32 of
/// 32, which read as a property of the shared glyph rasteriser. Taken over the **whole**
/// contradicted pool in the eight-hundred-and-forty-fourth session it holds on most of it, across
/// every mechanism the pool contains: the JBIG2 pages, the `CalRGB` pages, the CMYK shading pages,
/// the link border. So it is the pool's **base rate** and not that population's signature, and a
/// verdict rule resting on it would acquit us wherever two references agree for any reason at all
/// — including the shared decoder and the shared profile, where trap 9's first bullets say the
/// consensus is manufactured and say nothing whatever about who is right. ADR 0771.
///
/// **The count alone was not enough, and the eight-hundred-and-forty-fifth session is why.** The
/// interesting half of that measurement is its *complement* — the pages where the excluded
/// reference meets the bound while we do not — and ADR 0771 named those three pages by reading
/// them off a run by hand. One of the three is not in the population (`issue19633.pdf` page 1,
/// where `ghostscript` is at structural similarity 0.98828 against `mupdf` and the vector floor
/// is 0.9900) and one that is was missing (`freeculture.pdf` page 313). A population handed to
/// the next round in prose is a population the next round cannot check, so the gate names it.
/// ADR 0772.
///
/// `None` where every voting reference is in the consensus, or where the excluded one abstained
/// or never drew.
fn the_excluded_reference_under_the_same_bound(
    triangulation: &pdfref::Triangulation,
) -> Option<ExcludedReading> {
    let consensus = triangulation.consensuses.first()?;
    let voting = Reference::voting();
    let reference = triangulation
        .ours
        .iter()
        .map(|(reference, _)| *reference)
        .find(|reference| {
            voting.contains(reference)
                && !consensus.references.contains(reference)
                && !triangulation.abstained.contains(reference)
        })?;
    let against_members: Vec<bool> = consensus
        .references
        .iter()
        .filter_map(|member| {
            triangulation
                .between_references
                .iter()
                .find(|(left, right, _)| {
                    (*left == reference && right == member)
                        || (left == member && *right == reference)
                })
                .map(|(_, _, comparison)| !consensus.judged_by.accepts(comparison))
        })
        .collect();
    Some(ExcludedReading {
        reference,
        outside_of: against_members.iter().filter(|outside| **outside).count(),
        members: against_members.len(),
    })
}

/// How far apart the rasters of the consensus that decided a page's verdict sit.
///
/// # The question it answers
///
/// ADR 0773 measured one row of one table — the 119 vector pages the `mupdf` + `ghostscript`
/// consensus contradicts `poppler` on — and found that on **97 of the 117 it could compare the
/// two reference rasters are byte-identical**, so `Tolerance::widened_to` widens nothing (twice
/// zero is zero) and the excluded reference is held to the bare class floor with the whole
/// relative-bound mechanism inert. Trap 9 carries it as a mechanism: *a consensus of two
/// identical rasters is one reading counted twice*.
///
/// What that measurement could not say is anything about itself, which is ADR 0771's general
/// shape — **a control measured on the population it was invented for is a hypothesis until it
/// is run on the population it excludes**. So this is the same reading taken over every page the
/// gate judges, in both directions: how often a consensus is one raster twice, and what each
/// verdict pool looks like with those pages set aside.
///
/// # What it costs
///
/// Nothing. `Triangulation::between_references` already holds every pair's
/// [`raster_compare::Comparison`], and `max_error` is the field on it that separates identity
/// from closeness — trap 9's own tell, *because every other number on that line rounds to zero
/// long before the rasters are equal*. No render and no comparison is added.
#[derive(Debug)]
struct ConsensusIdentity {
    /// The set the verdict rests on: the head of `Triangulation::consensuses`.
    references: Vec<Reference>,
    /// The largest single-channel difference between any two of its members, of 255.
    ///
    /// Zero means every pair in the set is the same bytes. One means they are not, however
    /// close the other three measures put them.
    widest_max_error: u8,
    /// Whether the bound this set held us to is the bare class floor.
    ///
    /// The other half of `doc/todo/12`'s question, and not the same population as
    /// [`Self::widest_max_error`] being zero: a spread of zero implies a floor, and a spread
    /// small enough that twice it is still under every class bound does too. This is the
    /// population on which the *relative* bound — the whole reason this gate judges the way it
    /// does — decided nothing.
    at_the_class_floor: bool,
    /// Which class of bounds the page was judged by, for the breakdown.
    ///
    /// It is the discriminator the argument turns on rather than decoration: identical rasters
    /// on a flat vector page are two programs agreeing about something with one answer, and
    /// identical rasters on a page of hinted glyphs would be something else entirely.
    class: &'static str,
}

impl ConsensusIdentity {
    /// Whether every pair in the consensus is byte-identical.
    fn identical(&self) -> bool {
        self.widest_max_error == 0
    }

    /// The set named as `verdict_of` names it, so the two lines can be read side by side.
    fn set(&self) -> String {
        self.references
            .iter()
            .map(|reference| reference.name())
            .collect::<Vec<&str>>()
            .join(" and ")
    }
}

/// Reads [`ConsensusIdentity`] off the numbers the gate has already computed.
///
/// The set asked is the head of `consensuses`, which is the set `verdict_of` names on the page's
/// own line and the one `Examined::excluded_reference` is taken against — so the three lines are
/// about one set rather than three.
///
/// `class` is the page's own [`bounds_for`] result. It is passed rather than re-derived because
/// `Consensus::judged_by` is the *widened* bound and the question is whether it moved.
///
/// `None` where no two references agreed at all, which is the population no consensus judged.
fn the_consensus_that_decided_it(
    triangulation: &pdfref::Triangulation,
    class: &Tolerance,
) -> Option<ConsensusIdentity> {
    let consensus = triangulation.consensuses.first()?;
    let widest_max_error = triangulation
        .between_references
        .iter()
        .filter(|(left, right, _)| {
            consensus.references.contains(left) && consensus.references.contains(right)
        })
        .map(|(_, _, comparison)| comparison.max_error)
        .max()?;
    Some(ConsensusIdentity {
        references: consensus.references.clone(),
        widest_max_error,
        at_the_class_floor: consensus.judged_by == *class,
        class: if *class == Tolerance::TEXT_HEAVY {
            "text"
        } else {
            "vector"
        },
    })
}

/// What one page's verdict would be if the bound that forms a consensus were raised to the
/// spread the references themselves show.
///
/// # The question, which is `doc/todo/12` item 1
///
/// `Tolerance::max_differing_fraction` does two jobs: it decides whether two references **form**
/// a consensus, and it **floors** the bound `Tolerance::widened_to` then derives for us. ADR
/// 0243 measured the formation half once — raising it to the references' own 99th percentile
/// forms 457 new consensuses of which 278 contradict us — and left the number alone, because
/// "278 pages nobody has looked at" is a programme of work rather than an argument. ADR 0771
/// closed the *floor* half. Nothing since has said what those 278 are **made of**, and a
/// population described in an ADR is a population no round can check (ADR 0772).
///
/// So this is the counterfactual, per page, on every run: `pdfref::Triangulation::rejudged` over
/// the comparisons the gate already holds, with the formation bound raised and the floor taken
/// two ways.
///
/// # The two arms, and why both
///
/// [`Self::formation_only`] raises the formation bound and leaves our own floor at the class
/// value — which is the change `doc/todo/12` item 1 actually describes, and the strict one: a
/// consensus that would not have formed now judges us at the bound we are judged at today.
/// [`Self::with_the_floor`] raises both, which is what ADR 0243 ran and what its 457/278 counts.
/// The difference between the two is the price of the floor half on this population, and until
/// this census nothing separated them.
///
/// # What it costs
///
/// Nothing but arithmetic. No render, no comparison, and no verdict changes: the live gate is
/// judged by `Triangulation::outcome` exactly as before, and [`a_raised_formation_bound`]
/// asserts on every page that re-judging at the page's own bounds reproduces it.
#[derive(Debug)]
struct RaisedFormation {
    /// Which class of bounds the page was judged by.
    class: &'static str,
    /// What the gate concludes today.
    now: Standing,
    /// What this counterfactual concludes when handed the page's own bounds.
    ///
    /// It must equal [`Self::now`] on every page, and [`a_raised_formation_bound`] asserts that
    /// it does: an instrument that cannot reproduce the fact says nothing about the alternative.
    control: Standing,
    /// What it would conclude with the formation bound raised and our floor unchanged.
    formation_only: Standing,
    /// What it would conclude with both raised, which is ADR 0243's own arm.
    with_the_floor: Standing,
    /// The set that decides the page under the raised formation bound, where one forms.
    ///
    /// Named as `verdict_of` names it, so the two can be read side by side.
    set: Option<String>,
    /// The widest differing fraction inside that set, of 1.0.
    ///
    /// How far past the class bound the formation had to reach to admit it, which is the
    /// quantity the raise buys and the one a smaller raise would leave behind.
    formed_at: Option<f64>,
    /// Whether every pair in that set is byte-identical — trap 9's tell, ADR 0774's census.
    identical: bool,
    /// The measure a conviction under [`Self::formation_only`] rests on, and by how much.
    ///
    /// `None` unless that arm contradicts us. Same units as [`Examined::outside_the_bound`]:
    /// multiples of the bound the page was held to, so a mechanism can be read off it.
    convicted_on: Option<(f64, &'static str)>,
    /// Whether a reference agrees with **us** more closely than the convicting set agrees with
    /// itself, on the very measure the conviction rests on.
    ///
    /// Trap 12's subject asked of a counterfactual. `decide` holds a third implementation to
    /// twice the distance between the two programs that happened to agree most closely, so a
    /// wider formation bound admits *less* close pairs and the question becomes sharp: on a page
    /// where our own render is nearer to some reference than the convicting pair is to each
    /// other, the manufactured consensus is not the closest reading of the page and the
    /// conviction is a statement about which pair was selected.
    ///
    /// Like for like, which ADR 0688 is the reason for: both halves are read on the one measure
    /// [`Self::convicted_on`] names, never a maximum against a maximum.
    ///
    /// `false` wherever that arm does not convict, and `false` on a page where the pair agrees
    /// more closely than anybody agrees with us — `bug766086.pdf` page 1 is that shape, where the
    /// two that agree are two that draw no link border at all (trap 9's shared gap, ADR 0663).
    nearer_than_the_pair: bool,
}

/// The three standings a page can have in this census, which is [`Verdict`] with its detail off.
///
/// A counterfactual has to be comparable with a fact, and `Verdict` carries a sentence built out
/// of the live numbers — so the transition matrix is taken over this instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Standing {
    /// A consensus formed and accepts our render.
    Agrees,
    /// A consensus formed and does not.
    Contradicted,
    /// No consensus formed, or two formed and parted about us.
    Ambiguous,
    /// Fewer than two references drew, so nothing is triangulated either way.
    Unjudged,
}

impl Standing {
    /// How the census prints it.
    fn word(self) -> &'static str {
        match self {
            Self::Agrees => "agrees",
            Self::Contradicted => "contradicted",
            Self::Ambiguous => "ambiguous",
            Self::Unjudged => "unjudged",
        }
    }

    /// The standing an `Outcome` carries, whether it came from the gate or from a counterfactual.
    fn of(outcome: &Outcome) -> Self {
        match outcome {
            Outcome::Agrees { .. } => Self::Agrees,
            Outcome::Regression { .. } => Self::Contradicted,
            Outcome::Ambiguous => Self::Ambiguous,
            _ => Self::Unjudged,
        }
    }
}

/// The formation bound raised to the 99th percentile of what the references do to each other.
///
/// ADR 0243's own rule and ADR 0243's own number for the text class, re-derived by
/// [`the_fixed_bounds_against_the_references_own_spread`] in the eight-hundred-and-forty-ninth
/// session: on text pages the differing fraction between two voting references runs a 99th
/// percentile of **11.21%** within the pair sharing one `libfreetype.so.6` and **12.04%** across
/// the boundary, against a bound of 5.00%; on vector pages **1.36%** and **1.11%** against a
/// bound of 1.00%.
///
/// Both classes are raised rather than the one ADR 0243 argued about, because "is this a fact
/// about text pages or about the measure?" is a question the census should answer rather than
/// assume — and the class breakdown it prints is the answer.
///
/// The other three bounds are untouched: they already sit at or above their own 99th percentile,
/// which is the finding ADR 0243 recorded and this run reproduces.
fn raised_formation(class: &Tolerance) -> Tolerance {
    Tolerance {
        max_differing_fraction: if *class == Tolerance::TEXT_HEAVY {
            0.12
        } else {
            0.0136
        },
        ..*class
    }
}

impl RaisedFormation {
    /// Runs the counterfactual for one page, and the control beside it.
    ///
    /// The control is the instrument's calibration: handed the page's own bounds, `rejudged`
    /// must reproduce the verdict the page actually got. A counterfactual that cannot reproduce
    /// the fact is measuring its own arithmetic — trap 13, one directory over. It is carried as
    /// a field and asserted by [`a_raised_formation_bound`], which has a name to fail with.
    fn of(triangulation: &pdfref::Triangulation, class: &Tolerance) -> Self {
        let control = triangulation.rejudged(class, class, Judgement::CORPUS);
        let raised = raised_formation(class);
        let formation_only = triangulation.rejudged(&raised, class, Judgement::CORPUS);
        let with_the_floor = triangulation.rejudged(&raised, &raised, Judgement::CORPUS);

        let set = formation_only.2.first();
        let widest = set.and_then(|consensus| {
            triangulation
                .between_references
                .iter()
                .filter(|(left, right, _)| {
                    consensus.references.contains(left) && consensus.references.contains(right)
                })
                .map(|(_, _, comparison)| comparison)
                .fold(None::<(f64, u8)>, |widest, comparison| {
                    let seen = (comparison.differing_fraction, comparison.max_error);
                    Some(widest.map_or(seen, |(fraction, error)| {
                        (fraction.max(seen.0), error.max(seen.1))
                    }))
                })
        });
        let convicted_on = match &formation_only.0 {
            Outcome::Regression { agreeing } => triangulation
                .ours
                .iter()
                .filter(|(reference, _)| agreeing.contains(reference))
                .map(|(_, comparison)| worst_ratio(comparison, &formation_only.1))
                .fold(None::<(f64, &'static str)>, |worst, seen| {
                    Some(worst.map_or(seen, |held| if seen.0 > held.0 { seen } else { held }))
                }),
            _ => None,
        };

        RaisedFormation {
            control: Standing::of(&control.0),
            class: if *class == Tolerance::TEXT_HEAVY {
                "text"
            } else {
                "vector"
            },
            now: Standing::of(&triangulation.outcome),
            formation_only: Standing::of(&formation_only.0),
            with_the_floor: Standing::of(&with_the_floor.0),
            set: set.map(|consensus| {
                consensus
                    .references
                    .iter()
                    .map(|reference| reference.name())
                    .collect::<Vec<&str>>()
                    .join(" and ")
            }),
            formed_at: widest.map(|(fraction, _)| fraction),
            identical: widest.is_some_and(|(_, error)| error == 0),
            convicted_on,
            nearer_than_the_pair: convicted_on.is_some_and(|(_, measure)| {
                let widest_in_the_set = set.and_then(|consensus| {
                    triangulation
                        .between_references
                        .iter()
                        .filter(|(left, right, _)| {
                            consensus.references.contains(left)
                                && consensus.references.contains(right)
                        })
                        .map(|(_, _, comparison)| distance_on(comparison, measure))
                        .fold(None::<f64>, |widest, seen| {
                            Some(widest.map_or(seen, |held: f64| held.max(seen)))
                        })
                });
                let our_nearest = triangulation
                    .ours
                    .iter()
                    .map(|(_, comparison)| distance_on(comparison, measure))
                    .fold(None::<f64>, |nearest, seen| {
                        Some(nearest.map_or(seen, |held: f64| held.min(seen)))
                    });
                match (our_nearest, widest_in_the_set) {
                    (Some(ours), Some(theirs)) => ours < theirs,
                    _ => false,
                }
            }),
        }
    }
}

/// One comparison's distance on one of [`worst_ratio`]'s four measures, by that measure's name.
///
/// Three of the four are distances already; structural similarity runs the other way, so what is
/// returned for it is the distance from identity. The names are `worst_ratio`'s own, which is
/// what keeps a like-for-like reading like for like (ADR 0688).
fn distance_on(comparison: &raster_compare::Comparison, measure: &str) -> f64 {
    match measure {
        "mean" => comparison.mean_error,
        "worst tile" => comparison.worst_tile_error,
        "differing fraction" => comparison.differing_fraction,
        _ => 1.0 - comparison.structural_similarity,
    }
}

/// The largest of [`outside_by`]'s four ratios, and the name of the measure it belongs to.
///
/// Split out of `outside_by` rather than folded into it because a ranked number that does not say
/// what it is a ratio *of* is unreadable: 29.19× on the differing fraction and 29.19× on the mean
/// are two different pages. The spellings are `quoted::Measure::words`' first, which is the
/// vocabulary two sweeps already read notes in.
fn worst_ratio(comparison: &raster_compare::Comparison, bounds: &Tolerance) -> (f64, &'static str) {
    [
        (comparison.mean_error / bounds.max_mean, "mean"),
        (
            comparison.worst_tile_error / bounds.max_worst_tile,
            "worst tile",
        ),
        (
            comparison.differing_fraction / bounds.max_differing_fraction,
            "differing fraction",
        ),
        (
            (1.0 - comparison.structural_similarity) / (1.0 - bounds.min_structural_similarity),
            "structural similarity",
        ),
    ]
    .into_iter()
    .fold((0.0, "mean"), |worst, ratio| {
        if worst.0 >= ratio.0 { worst } else { ratio }
    })
}

/// Renders one page with every available reference.
///
/// A reference that fails on a document is not evidence of anything — many of these files
/// are deliberately damaged, and a renderer refusing one is the correct behaviour — so its
/// absence is tolerated as long as two remain. Fewer than two is reported with every
/// failure's own message, because "not comparable" without a reason is not actionable.
///
/// **And two remaining is reported too, which it was not until the
/// six-hundred-and-ninety-fourth session.** The `Ok` arm used to drop the failures on the
/// floor, so a page judged on two readings printed a line indistinguishable from the same
/// page judged on three — and a verdict computed from a *pair* rather than a trio is a
/// different measurement, because the third reading is what turns two renderers that miss
/// each other into a consensus. `function_based_shading_cmyk.pdf` page 2 left the
/// contradicted list on exactly that: the figure recorded for it, 29.06, is `poppler`
/// against `mupdf` to the hundredth, while `mupdf` against `ghostscript` is inside the page's
/// bound — so the run that removed it had no `ghostscript` in it, and nothing said so.
/// ADR 0542.
///
/// # Why the three run in parallel
///
/// Because a page's cost is now its slowest reference rather than the sum of three, and one
/// page in this corpus decides the whole run's wall clock. `bomb_giant.pdf` is a
/// decompression bomb: `poppler` and `ghostscript` are each given 30 seconds on it and
/// neither returns, so running them one after another put a minute of pure waiting on the
/// critical path of a run that is otherwise 27 seconds of processor time spread over 24
/// cores. Nested inside the outer `par_iter` this is free — rayon's work-stealing has no
/// notion of a nesting level — and it costs nothing on a page whose references are cached,
/// which is now nearly all of them.
fn render_references(
    work: &Work,
    work_dir: &Path,
    available: &[Reference],
    cache: &Cache,
) -> Result<Rendered, String> {
    // Order is preserved by `collect` over an indexed parallel iterator, which matters:
    // `reconcile` reports who disagreed about a page's size by name.
    let attempts: Vec<(Reference, Result<Raster, String>)> = available
        .par_iter()
        .map(|reference| {
            let rendered = cache
                .render(*reference, &work.path, work.page, DPI, work_dir)
                .map_err(|e| format!("{e}"));
            (*reference, rendered)
        })
        .collect();

    let mut rendered = Vec::new();
    let mut failures = Vec::new();
    for (reference, attempt) in attempts {
        match attempt {
            Ok(raster) => rendered.push((reference, raster)),
            Err(detail) => failures.push(format!("{reference} did not render: {detail}")),
        }
    }
    if rendered.len() < 2 {
        return Err(failures.join("; "));
    }
    Ok(Rendered {
        rendered,
        absent: failures,
    })
}

/// What the reference renderers produced for one page, and what they did not.
///
/// A named pair rather than a tuple, and the second field is the whole reason it exists: it was
/// discarded for six hundred sessions, and ADR 0542 is what that cost — a page whose verdict rests
/// on two readings printing a line indistinguishable from one that rests on three.
struct Rendered {
    /// Every reference that produced a raster, in `available`'s order — [`reconcile`] reports a
    /// size disagreement by name and depends on that order.
    rendered: Vec<(Reference, Raster)>,
    /// Every reference that produced none, each with its own message.
    absent: Vec<String>,
}

/// Brings every raster to one size, or says whose geometry disagrees.
///
/// Two steps, so that a size disagreement names who disagreed.
///
/// The references first, and by *majority*: a renderer that reads the page's extent
/// differently from the other two is outvoted and dropped, exactly as it would be on
/// pixels. This is not leniency — 55 corpus documents are otherwise not compared at all,
/// among them pages where `mutool` draws a 196x59 region of a 612x792 page — and the
/// dropped renderer is named in the outcome. Where no two agree there is no consensus
/// about the page at all, and nothing to hold us to.
///
/// Then us against what they settled on, where a failure is ours: a `MediaBox`, `CropBox`
/// or `/Rotate` read differently, which is a more serious defect than any pixel.
fn reconcile(
    ours: &mut Raster,
    references: &mut Vec<(Reference, Raster)>,
) -> Result<Option<String>, Verdict> {
    let describe = |rasters: &[(Reference, Raster)]| {
        rasters
            .iter()
            .map(|(r, raster)| format!("{r} {}x{}", raster.width, raster.height))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let all_sizes = describe(references);

    // The largest set of references agreeing about the page's extent. Sizes within the
    // rounding slack count as the same size, which is what `to_common_size` decides.
    let majority = references
        .iter()
        .map(|(_, raster)| (raster.width, raster.height))
        .max_by_key(|size| {
            references
                .iter()
                .filter(|(_, other)| {
                    other.width.abs_diff(size.0) <= normalise::MAX_ROUNDING_SLACK
                        && other.height.abs_diff(size.1) <= normalise::MAX_ROUNDING_SLACK
                })
                .count()
        })
        .ok_or_else(|| Verdict::ReferenceGeometry("no references".to_owned()))?;

    let outvoted = references
        .iter()
        .filter(|(_, raster)| {
            raster.width.abs_diff(majority.0) > normalise::MAX_ROUNDING_SLACK
                || raster.height.abs_diff(majority.1) > normalise::MAX_ROUNDING_SLACK
        })
        .map(|(r, raster)| format!("{r} {}x{}", raster.width, raster.height))
        .collect::<Vec<_>>()
        .join(", ");
    references.retain(|(_, raster)| {
        raster.width.abs_diff(majority.0) <= normalise::MAX_ROUNDING_SLACK
            && raster.height.abs_diff(majority.1) <= normalise::MAX_ROUNDING_SLACK
    });
    if references.len() < 2 {
        return Err(Verdict::ReferenceGeometry(format!(
            "no two references agree about the page size ({all_sizes})"
        )));
    }

    {
        let mut views: Vec<&mut Raster> = references.iter_mut().map(|(_, r)| r).collect();
        normalise::to_common_size(&mut views)
            .map_err(|e| Verdict::ReferenceGeometry(format!("{e} ({all_sizes})")))?;
    }

    let our_size = (ours.width, ours.height);
    let mut views: Vec<&mut Raster> = std::iter::once(ours)
        .chain(references.iter_mut().map(|(_, r)| r))
        .collect();
    normalise::to_common_size(&mut views).map_err(|e| {
        Verdict::OurGeometry(format!(
            "{e} (ours {}x{}, {all_sizes})",
            our_size.0, our_size.1
        ))
    })?;

    Ok((!outvoted.is_empty()).then_some(outvoted))
}

/// Translates a triangulation into this gate's vocabulary.
///
/// `outvoted` names any reference dropped for disagreeing about the page size, which
/// belongs in the outcome: a verdict reached by two renderers rather than three is worth
/// less, and hiding that would be the harness flattering itself.
fn verdict_of(triangulation: &pdfref::Triangulation, outvoted: Option<&str>) -> Verdict {
    let note = outvoted.map_or_else(String::new, |dropped| {
        format!(" [{dropped} outvoted on page size]")
    });
    match &triangulation.outcome {
        Outcome::Agrees { .. } => Verdict::Agrees,
        Outcome::Regression { agreeing } => {
            let names: Vec<&str> = agreeing.iter().map(|r| r.name()).collect();
            Verdict::Contradicted(format!(
                "{} agree, we differ: {}{note}",
                names.join(" and "),
                measurements(triangulation, Some(agreeing))
            ))
        }
        Outcome::Ambiguous => {
            // A divided page and a page nobody agrees on are one `Outcome` and two very
            // different readings — on the first every renderer in the room is inside somebody's
            // consensus and on the second nobody is — so the page's own line says which it is
            // rather than leaving that to the census fifty lines below (ADR 0617).
            let divided = divided_by(triangulation)
                .map_or_else(String::new, |sets| format!(" [two readings: {sets}]"));
            Verdict::Ambiguous(format!(
                "{}{divided}{note}",
                measurements(triangulation, None)
            ))
        }
        Outcome::NotEnoughReferences { available } if !triangulation.abstained.is_empty() => {
            // The abstaining renderers ran and returned a raster of one colour, which a
            // renderer that drew marks disagrees with. The line says exactly that rather than
            // "drew nothing", because one of these pages is a full-page fill where the flat
            // sheet is the right answer — see `NOT_COMPARABLE_A_FLAT_SHEET_IS_THE_PAGE`. The
            // measurement beside it is taken against whichever reference drew marks, which is
            // the only reading of the page the gate has left.
            let drew: Vec<Reference> = triangulation
                .ours
                .iter()
                .map(|(reference, _)| *reference)
                .filter(|reference| !triangulation.abstained.contains(reference))
                .collect();
            let blank: Vec<&str> = triangulation
                .abstained
                .iter()
                .map(|reference| reference.name())
                .collect();
            Verdict::NotComparable(format!(
                "{} returned one colour, leaving {available} reading: {}{note}",
                blank.join(" and "),
                measurements(triangulation, Some(&drew))
            ))
        }
        Outcome::NotEnoughReferences { available } => {
            Verdict::NotComparable(format!("{available} reference(s)"))
        }
        // `Outcome` is non-exhaustive. A conclusion this gate has never seen must be
        // visible rather than quietly folded into one of the outcomes above.
        other => Verdict::NotComparable(format!("unrecognised outcome {other:?}")),
    }
}

/// Fails the gate if this build cannot reach the sandboxed image decoder.
///
/// `CCITTFaxDecode`, `JBIG2Decode` and `JPXDecode` are decoded by a separate program, and Cargo
/// does not build another package's binaries when it tests this one (trap 10). A build without
/// it draws every other image and none of those three, so what follows would be a measurement of
/// the build rather than of the tree — which is exactly what moved the accessibility census's
/// ratchet by nine elements while four rounds read the difference as something else
/// (ADR 0557, trap 16).
fn require_the_sandbox() {
    if let Err(error) = pdf_model::image::sandboxed_decoder() {
        panic!(
            "the sandboxed image decoder is not available, so the counts below would be \
             wrong: {error}"
        );
    }
}

/// How far we sit from the references, next to the bounds that were applied.
///
/// Both, always: our number means nothing on its own. A worst tile of 30 is a defect on a
/// page where the references agree to 0.4 and unremarkable on one where they differ by 26,
/// and the bound is what carries that distinction — it is derived from this page's own
/// consensus. Printing the references' raw spread instead would mislead, because the pair
/// that sets the bound is the *consensus* pair, not the widest.
///
/// # This line printed three of four bounds and measured against the wrong renderer, for 400 sessions
///
/// [`Tolerance::accepts`] applies **four** bounds and this line printed three: the differing
/// fraction was printed as our number with no bound beside it. It is not a spare — it is what
/// decides many of these verdicts, and the consequence was legible in the gate's own output.
/// **Thirty of the four-hundred-and-fifth session's 68 contradicted pages printed a line on
/// which every visible number was inside the printed bound**, which reads as a page failing for
/// no stated reason. `issue7580.pdf` is the plainest: mean 2.93 of 5.00, worst tile 7.10 of
/// 40.00, ssim 0.9734 of 0.9000, all comfortable — and `differing 6.15%` against a bound of
/// 5.00% that nothing printed.
///
/// The second half is worse and is why `decided_by` exists. The fold ran over **every**
/// reference while the bound beside it is the *consensus* pair's, so a page could be reported
/// against a renderer that takes no part in the verdict.
/// `smask_luminosity_oob_transfer.pdf` is the witness: the consensus is `mupdf` and
/// `ghostscript`, our distance from them is 2.02 against a bound of 1.11, and the line printed
/// **27.02** — `poppler`, which on that page sits 36 of 255 from all four other renderers and
/// is not in the consensus at all. `CONTRADICTED_MASK_QUANTISATION` had to state its own
/// numbers because the gate's were another renderer's.
///
/// So on a [`Outcome::Regression`] the fold runs over the consensus, and picks the comparison
/// **furthest outside the bound** rather than the one with the largest tile — which makes the
/// printed line the failure, always. On an [`Outcome::Ambiguous`] page there is no consensus
/// and no failure to name, so the old rule is kept exactly: the reference we look least like,
/// over all of them.
fn measurements(triangulation: &pdfref::Triangulation, decided_by: Option<&[Reference]>) -> String {
    let bounds = &triangulation.judged_by;
    let worst = match decided_by {
        // The verdict rests on these comparisons and on no others, so rank them by how far
        // outside the bound they are — over all four of `Tolerance::accepts`' measures.
        Some(consensus) => triangulation
            .ours
            .iter()
            .filter(|(reference, _)| consensus.contains(reference))
            .map(|(_, c)| c)
            .fold(
                None::<&raster_compare::Comparison>,
                |worst, c| match worst {
                    Some(previous) if outside_by(previous, bounds) >= outside_by(c, bounds) => {
                        Some(previous)
                    }
                    _ => Some(c),
                },
            ),
        None => triangulation.ours.iter().map(|(_, c)| c).fold(
            None::<&raster_compare::Comparison>,
            |worst, c| match worst {
                Some(previous) if previous.worst_tile_error >= c.worst_tile_error => Some(previous),
                _ => Some(c),
            },
        ),
    };
    let applied = format!(
        "bound mean {:.2} worst tile {:.2} differing {:.2}% ssim {:.4}",
        bounds.max_mean,
        bounds.max_worst_tile,
        bounds.max_differing_fraction * 100.0,
        bounds.min_structural_similarity
    );
    worst.map_or_else(
        || format!("nothing measured; {applied}"),
        |c| {
            format!(
                "ours at worst mean {:.2} worst tile {:.2} differing {:.2}% ssim {:.4}; {applied}",
                c.mean_error,
                c.worst_tile_error,
                c.differing_fraction * 100.0,
                c.structural_similarity
            )
        },
    )
}

/// How far one comparison sits outside the bounds applied to it, as a multiple of each.
///
/// The largest of the four ratios [`Tolerance::accepts`] checks, so that a comparison failing
/// on the differing fraction alone ranks beside one failing on mean error. Above 1.0 means the
/// comparison was rejected, which is the property [`measurements`] relies on: the comparison
/// with the largest ratio among the consensus is a failing one whenever the verdict is a
/// contradiction.
///
/// [`Distance::of`] deliberately keeps a *three*-measure ratio and is not folded into this one.
/// Its numbers are quoted in a hundred entries of this file and in `doc/todo/00`, and a page's
/// recorded "0.16 from the nearest reference" has to stay the number that was recorded. What that
/// costs is priced in [`rank_the_contradicted_by_the_bound`] rather than left as a caution, and
/// [`nearest_on_every_measure`] is this function applied to our own comparisons — the fourth
/// measure as a *second* number beside `Distance` rather than inside it (ADR 0643).
///
/// The arithmetic is [`worst_ratio`]'s, which also says which of the four the largest ratio was.
/// One implementation of it, because two would eventually disagree about a verdict.
fn outside_by(comparison: &raster_compare::Comparison, bounds: &Tolerance) -> f64 {
    worst_ratio(comparison, bounds).0
}

/// [`outside_by`] without the differing fraction — the unit [`Distance`] is in.
///
/// Three of [`Tolerance::accepts`]' four measures, so that a number taken here is comparable with
/// [`Distance::nearest`] and therefore with the figures a hundred notes in this file quote. It is
/// the arithmetic [`Distance::of`] used to carry inline; extracted so that
/// [`consensus_missed_in_three_measures`] can ask the *references* the same question in the same
/// unit, which is the whole of what it is for.
///
/// # One property this does not share with [`outside_by`]
///
/// On an ambiguous page `outside_by` is above 1 for every pair by construction — a pair inside all
/// four bounds would have been a consensus and the page would not be ambiguous. **This one can be
/// below 1**, and exactly where the closest pair misses on the differing fraction alone. That is
/// not a defect of the measure: it is `doc/todo/12`'s bound saying that the pair agreed about
/// everything the other three measures see. What follows for a reader is that a ratio taken over
/// this number can be large because *we* are far or because the pair was close on three measures,
/// and the second is a page where the picture is the instrument (trap 1).
fn outside_by_in_three_measures(
    comparison: &raster_compare::Comparison,
    bounds: &Tolerance,
) -> f64 {
    worst_ratio_in_three_measures(comparison, bounds).0
}

/// [`outside_by_in_three_measures`] with the name of the measure the largest ratio belongs to.
///
/// [`worst_ratio`] and this one are the same split for the same reason, over four measures and
/// over three: **a ranked number that does not say what it is a ratio of is unreadable**. It took
/// until the seven-hundred-and-sixty-fourth session to be made here because the four-measure
/// twin's name is printed beside a *verdict*, where a note that misprices it is an exemption from
/// a bound, and this one's is printed beside a *ranking*, where nothing failed and the cost is a
/// diagnosis that explains the wrong measure (ADR 0688). [`AloneOn`] is where it is used.
fn worst_ratio_in_three_measures(
    comparison: &raster_compare::Comparison,
    bounds: &Tolerance,
) -> (f64, &'static str) {
    [
        (comparison.mean_error / bounds.max_mean, "mean"),
        (
            comparison.worst_tile_error / bounds.max_worst_tile,
            "worst tile",
        ),
        (
            (1.0 - comparison.structural_similarity) / (1.0 - bounds.min_structural_similarity),
            "structural similarity",
        ),
    ]
    .into_iter()
    .fold((0.0, "mean"), |worst, ratio| {
        if worst.0 >= ratio.0 { worst } else { ratio }
    })
}

/// The gate.
#[test]
#[ignore = "renders every corpus document four times; run explicitly, in release"]
fn our_rendering_agrees_with_the_reference_consensus_across_the_corpus() {
    require_the_sandbox();
    let Some(items) = work_items() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    // Voting, not merely available: `hayro` is driven for the artefacts of pages worth
    // looking at, and never counted, because what we share with it is most of a page. See
    // `Reference::independence`.
    let available: Vec<Reference> = Reference::voting()
        .into_iter()
        .filter(|reference| reference.is_available())
        .collect();
    assert!(
        available.len() >= 2,
        "at least two reference renderers are needed to triangulate; found {}. Install: {}",
        available.len(),
        Reference::voting()
            .iter()
            .filter(|r| !r.is_available())
            .map(|r| r.package_hint())
            .collect::<Vec<_>>()
            .join(", ")
    );
    // Versions belong in the record: these renderers change their output between releases,
    // so a disagreement appearing tomorrow may be an upstream change rather than ours.
    for reference in &available {
        println!(
            "{}: {}",
            reference.name(),
            reference.version().unwrap_or_default()
        );
    }

    let work_root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("oracle");
    let cache = reference_cache();

    let selection = Selection::from_environment();
    let total = items.len();
    let items: Vec<Work> = items
        .into_iter()
        .filter(|work| selection.admits(work))
        .collect();
    assert!(
        !items.is_empty(),
        "PDFVIEWER_ORACLE_ONLY={} matched none of the {total} pages",
        selection.pattern().unwrap_or_default()
    );

    let started = Instant::now();
    let mut results: Vec<Examined> = items
        .par_iter()
        .map(|work| examine(work, &work_root, &available, &cache))
        .collect();
    results.sort_by(|a, b| a.name.cmp(&b.name));
    let elapsed = started.elapsed();

    report(&results, elapsed, &cache);
    println!("artefacts under {}", work_root.display());

    if let Some(pattern) = selection.pattern() {
        println!(
            "\nPDFVIEWER_ORACLE_ONLY={pattern} selected {} of {total} pages. The ratchets below \
             are NOT checked: a list held to equality over a subset would report every page the \
             filter excluded as fixed.",
            items.len()
        );
        return;
    }

    check_the_ratchets(&results);
}

/// Every `CONTRADICTED_*` group the ratchet holds, by name, in the order the file declares them.
///
/// One table with two readers. [`check_the_ratchets`] chains the pages into the single ratchet the
/// contradicted pool has always been held by, and [`rank_the_contradicted_by_the_bound`] prints
/// the name beside each ranked row — so that a round handed the pool's head can see, on the line
/// it reads, whether the page's cause is already written down and under which name. Until the
/// eight-hundred-and-seventy-third session that took a round the better part of an hour, by hand,
/// against twenty-three declarations spread over eleven thousand lines (ADR 0805).
///
/// [`CONTRADICTED_ON_A_PAGE_WE_REPORT`] is deliberately **not** in it: its pages are outside the
/// ratchet by construction and held by their own staleness check below, and [`held_by`] looks
/// there second so that the ranking — which does not filter on `complete` — still names it.
const CONTRADICTED_GROUPS: &[(&str, &[&str])] = &[
    ("CONTRADICTED_PAGE_ROUNDING", &CONTRADICTED_PAGE_ROUNDING),
    (
        "CONTRADICTED_COINCIDENT_CLIP_EDGES",
        &CONTRADICTED_COINCIDENT_CLIP_EDGES,
    ),
    (
        "CONTRADICTED_IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE",
        &CONTRADICTED_IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE,
    ),
    (
        "CONTRADICTED_SHARED_JBIG2_DECODER",
        &CONTRADICTED_SHARED_JBIG2_DECODER,
    ),
    (
        "CONTRADICTED_IMAGE_RESAMPLING",
        &CONTRADICTED_IMAGE_RESAMPLING,
    ),
    (
        "CONTRADICTED_CALIBRATED_COLOUR",
        &CONTRADICTED_CALIBRATED_COLOUR,
    ),
    (
        "CONTRADICTED_CALRGB_TO_SCREEN",
        &CONTRADICTED_CALRGB_TO_SCREEN,
    ),
    (
        "CONTRADICTED_REFERENCE_GLYPH_WIDTHS",
        &CONTRADICTED_REFERENCE_GLYPH_WIDTHS,
    ),
    (
        "CONTRADICTED_NEGATIVE_LINE_WIDTH",
        &CONTRADICTED_NEGATIVE_LINE_WIDTH,
    ),
    (
        "CONTRADICTED_DEVICE_CMYK_CONVERSION",
        &CONTRADICTED_DEVICE_CMYK_CONVERSION,
    ),
    ("CONTRADICTED_SUBPIXEL_IMAGE", &CONTRADICTED_SUBPIXEL_IMAGE),
    (
        "CONTRADICTED_MASK_QUANTISATION",
        &CONTRADICTED_MASK_QUANTISATION,
    ),
    (
        "CONTRADICTED_VISIBILITY_EXPRESSION",
        &CONTRADICTED_VISIBILITY_EXPRESSION,
    ),
    (
        "CONTRADICTED_REFERENCES_DREW_NOTHING",
        &CONTRADICTED_REFERENCES_DREW_NOTHING,
    ),
    ("CONTRADICTED_LINK_BORDER", &CONTRADICTED_LINK_BORDER),
    (
        "CONTRADICTED_GLYPHS_JUDGED_AS_VECTOR",
        &CONTRADICTED_GLYPHS_JUDGED_AS_VECTOR,
    ),
    (
        "CONTRADICTED_ANTIALIASED_EDGES",
        &CONTRADICTED_ANTIALIASED_EDGES,
    ),
    ("CONTRADICTED_GLYPH_EDGES", &CONTRADICTED_GLYPH_EDGES),
    (
        "CONTRADICTED_SYMBOLIC_FONT_FLAGS",
        &CONTRADICTED_SYMBOLIC_FONT_FLAGS,
    ),
    (
        "CONTRADICTED_SUBSTITUTED_FONT",
        &CONTRADICTED_SUBSTITUTED_FONT,
    ),
    ("CONTRADICTED_UNEXPLAINED", &CONTRADICTED_UNEXPLAINED),
    (
        "CONTRADICTED_TIGHT_CONSENSUS",
        &CONTRADICTED_TIGHT_CONSENSUS,
    ),
];

/// Every `CONTRADICTED_*` declaration in this file is in [`CONTRADICTED_GROUPS`], or is the one
/// the table's comment excludes by name.
///
/// A group added to the file and not to the table would leave the ratchet exactly as the old
/// hand-written chain would have — silently one group short — so the table is checked against
/// the source it lives in. Not `#[ignore]`d: it reads no corpus and costs nothing.
#[test]
fn every_contradicted_group_is_in_the_table() {
    let declared: Vec<&str> = include_str!("oracle.rs")
        .lines()
        .filter_map(|line| line.strip_prefix("const CONTRADICTED_"))
        .filter_map(|rest| rest.split_once(':'))
        // A page list and nothing else — the rule `tools/conformance`'s `overtaken` reads the
        // same file by, which is what keeps the table itself out of its own count.
        .filter(|(_, tail)| tail.trim_start().starts_with("[&str;"))
        .map(|(name, _)| name.trim())
        .collect();
    let in_the_table: Vec<&str> = CONTRADICTED_GROUPS
        .iter()
        .map(|(name, _)| name.trim_start_matches("CONTRADICTED_"))
        .chain(std::iter::once("ON_A_PAGE_WE_REPORT"))
        .collect();
    let missing: Vec<&&str> = declared
        .iter()
        .filter(|name| !in_the_table.contains(name))
        .collect();
    assert!(
        missing.is_empty(),
        "declared in this file and absent from CONTRADICTED_GROUPS: {missing:?}"
    );
    assert_eq!(
        declared.len(),
        in_the_table.len(),
        "the table names a group this file does not declare"
    );
}

/// The name of the group holding a contradicted page, or `None` where no group does.
///
/// A complete page answers `None` only in a build the ratchet has already failed; an incomplete
/// one can answer it at any time, which is why the ranking asks.
fn held_by(page: &str) -> Option<&'static str> {
    CONTRADICTED_GROUPS
        .iter()
        .find(|(_, pages)| pages.contains(&page))
        .map(|(name, _)| *name)
        .or_else(|| {
            CONTRADICTED_ON_A_PAGE_WE_REPORT
                .contains(&page)
                .then_some("CONTRADICTED_ON_A_PAGE_WE_REPORT")
        })
}

/// Holds every gated outcome to the list that carries its argument.
///
/// Split out of the gate itself because a hundred lines of ratchet in the middle of a test
/// is where a reader stops reading, and because each of the three lists below is a
/// different claim: the contradicted groups are diagnoses of defects, `GEOMETRY` is a class
/// of defect that has been empty since the twenty-ninth session, and the ambiguous pair is
/// a population nobody had watched at all until the hundred-and-seventy-sixth.
fn check_the_ratchets(results: &[Examined]) {
    // Only pages we claim to draw completely are gated: see the module documentation.
    let named = |predicate: &dyn Fn(&Examined) -> bool| -> Vec<&str> {
        results
            .iter()
            .filter(|e| e.complete && predicate(e))
            .map(|e| e.name.as_str())
            .collect()
    };
    // The groups are one ratchet: which group a page belongs to is a hypothesis about it,
    // and holding each group separately would fail the build every time a hypothesis turned
    // out to be wrong rather than every time the rendering changed.
    let contradicted: Vec<&str> = CONTRADICTED_GROUPS
        .iter()
        .flat_map(|(_, pages)| pages.iter().copied())
        .collect();
    assert_ratchet(
        "contradicted by the reference consensus",
        &named(&|e| matches!(e.verdict, Verdict::Contradicted(_))),
        &contradicted,
        "Each is a page two independent implementations agree about and we do not. Read \
         the artefacts named above, then take the disagreement to the specification — \
         never to what the references produce.",
    );
    assert_ratchet(
        "disagreeing with the references about page geometry",
        &named(&|e| matches!(e.verdict, Verdict::OurGeometry(_))),
        &GEOMETRY,
        "A page box, /Rotate or /UserUnit is being read differently from every reference, \
         so the comparison cannot even proceed.",
    );

    check_the_no_render_bucket(results);
    check_the_buckets_reached_without_a_consensus(results);

    // A page this tree reports cannot fail the ratchet above — `named` filters on `complete`,
    // for the reason the module comment gives — so the diagnoses in
    // `CONTRADICTED_ON_A_PAGE_WE_REPORT` are held the way a stale `AMBIGUOUS_*` group is held
    // instead: a name that stops being contradicted fails here. The other direction needs
    // nothing, because a page that stops *reporting* while still contradicted arrives in the
    // gated list and fails there.
    let contradicted_anywhere: Vec<&str> = results
        .iter()
        .filter(|e| matches!(e.verdict, Verdict::Contradicted(_)))
        .map(|e| e.name.as_str())
        .collect();
    let stale_reported: Vec<&str> = CONTRADICTED_ON_A_PAGE_WE_REPORT
        .iter()
        .copied()
        .filter(|name| !contradicted_anywhere.contains(name))
        .collect();
    assert!(
        stale_reported.is_empty(),
        "{} page(s) in CONTRADICTED_ON_A_PAGE_WE_REPORT are no longer contradicted: \
         {stale_reported:?}\nDelete them from the group: a diagnosis that outlives what it \
         diagnosed is the staleness this file's own history is made of.",
        stale_reported.len()
    );

    // The ambiguous bucket, watched in both directions since the hundred-and-seventy-sixth
    // session. A diagnosed page must still be ambiguous or its group is stale; every other
    // ambiguous page is held by name, so a page that used to agree cannot arrive here in
    // silence. See `undiagnosed_ambiguous`.
    let ambiguous = named(&|e| matches!(e.verdict, Verdict::Ambiguous(_)));
    let diagnosed = diagnosed_ambiguous();
    let stale: Vec<&str> = diagnosed
        .iter()
        .copied()
        .filter(|name| !ambiguous.contains(name))
        .collect();
    assert!(
        stale.is_empty(),
        "{} page(s) named in an AMBIGUOUS_* group are no longer ambiguous: {stale:?}\n\
         Delete them from the group: a diagnosis that outlives what it diagnosed is the \
         staleness this file's own history is made of.",
        stale.len()
    );
    let undiagnosed: Vec<&str> = ambiguous
        .iter()
        .copied()
        .filter(|name| !diagnosed.contains(name))
        .collect();
    assert_ratchet(
        "ambiguous without a diagnosis",
        &undiagnosed,
        &undiagnosed_ambiguous(),
        "Each is a page the references used to settle and no longer do — or one this gate \
         has never judged. Read the artefacts, then either fix the page or write down what \
         is going on with it in an AMBIGUOUS_* group.",
    );
}

/// Holds the `no render` bucket by name, in both directions.
///
/// Over **every** page rather than the complete ones, which is the one place in this file that
/// filter is wrong: a page this gate produced no raster of is never `complete`, so a list built
/// through `check_the_ratchets`' `named` would hold an empty list against an empty list and watch
/// nothing at all.
///
/// It is a function of its own for the reason the ratchet exists — the population it reads is not
/// the population every other assertion in that function reads, and a filter that differs by one
/// word is the kind of difference a reader skims past.
fn check_the_no_render_bucket(results: &[Examined]) {
    let no_render: Vec<&str> = results
        .iter()
        .filter(|e| matches!(e.verdict, Verdict::NoRender(_)))
        .map(|e| e.name.as_str())
        .collect();
    assert_ratchet(
        "producing no render at all",
        &no_render,
        &no_render_expected(),
        "A page nobody is shown is the worst outcome this gate has a name for, and it is the \
         one verdict reached without asking the references — so the reason has to be written \
         down rather than measured. Put the page in the NO_RENDER_* group whose argument \
         covers it, or ask the three renderers what they make of it: \
         doc/oracle-and-corpus.md §3d has the recipe.",
    );
}

/// Holds the `not comparable` and `reference geometry` buckets by name, in both directions.
///
/// The other two verdicts nothing watched, taken in the five-hundred-and-seventy-ninth session
/// for the reason ADR 0410 left written down: neither accuses this tree by construction, and
/// "by construction" is a claim of exactly the kind that had been true of `no render` for four
/// hundred rounds.
///
/// Over **every** page and not the complete ones, for [`check_the_no_render_bucket`]'s reason
/// one bucket over: what these two verdicts are about is what the *references* did, so our own
/// completeness is the wrong filter — it would hold seven of the thirteen and watch the rest not
/// at all.
fn check_the_buckets_reached_without_a_consensus(results: &[Examined]) {
    let named = |predicate: &dyn Fn(&Examined) -> bool| -> Vec<&str> {
        results
            .iter()
            .filter(|e| predicate(e))
            .map(|e| e.name.as_str())
            .collect()
    };
    assert_ratchet(
        "not comparable",
        &named(&|e| matches!(e.verdict, Verdict::NotComparable(_))),
        &not_comparable_expected(),
        "Fewer than two references produced an image, so nothing here can be triangulated — \
         which makes this the bucket where a page of ours is least likely to be checked by \
         anybody. Ask the three renderers what they make of it and put the page in the \
         NOT_COMPARABLE_* group whose argument covers it: doc/oracle-and-corpus.md §3e has \
         the recipe and what the last whole run of it found.",
    );
    assert_ratchet(
        "unreconcilable with the references' page size",
        &named(&|e| matches!(e.verdict, Verdict::ReferenceGeometry(_))),
        &reference_geometry_expected(),
        "No two references agree about the page's extent. Read the sizes in the verdict \
         before believing the label: a 1x1 raster from `pdftoppm` is a refusal that exits 0, \
         not an opinion about the page, and both pages this list held when it was written \
         were that. doc/oracle-and-corpus.md §3e.",
    );
}

/// Prints every document that is not agreement, then the totals.
///
/// The per-document lines come first and unabbreviated: on a failure they are the evidence,
/// and a summary that hides which page moved is a summary nobody can act on. Each count is
/// given twice — over everything, and over the pages we claim to draw completely — because
/// only the second is gated and conflating them flatters the result.
fn report(results: &[Examined], elapsed: std::time::Duration, cache: &Cache) {
    for examined in results {
        if !matches!(examined.verdict, Verdict::Agrees) || !examined.absent.is_empty() {
            println!(
                "  {}: {}{} — {}{}",
                examined.name,
                examined.verdict.label(),
                if examined.complete {
                    ""
                } else {
                    " (incomplete)"
                },
                examined.verdict.detail(),
                if examined.absent.is_empty() {
                    String::new()
                } else {
                    format!("  [judged without: {}]", examined.absent.join("; "))
                }
            );
        }
    }

    let count =
        |predicate: &dyn Fn(&Examined) -> bool| results.iter().filter(|e| predicate(e)).count();
    println!(
        "\n{} pages in {:.1}s ({} we call complete, {} incomplete)",
        results.len(),
        elapsed.as_secs_f64(),
        count(&|e| e.complete),
        count(&|e| !e.complete)
    );

    // Where the time went. Summed over threads, so these add up to more than the wall
    // clock; what matters is their ratio.
    let ours: std::time::Duration = results.iter().map(|e| e.spent.ours).sum();
    let references: std::time::Duration = results.iter().map(|e| e.spent.references).sum();
    println!(
        "  processor time: {:.0}s ours, {:.0}s in the three reference renderers",
        ours.as_secs_f64(),
        references.as_secs_f64()
    );
    // Which of those seconds were avoided, and how many were paid. A hit rate below one on
    // an unchanged tree means something in the key moved — a renderer was updated, or the
    // corpus was — and that is worth seeing rather than inferring from the clock.
    let statistics = cache.statistics();
    println!(
        "  reference renders: {} from the cache, {} produced ({:.1}% hit rate){}",
        statistics.hits,
        statistics.misses,
        statistics.hit_rate() * 100.0,
        match cache.root() {
            Some(root) if cache.is_enabled() => format!(", cached under {}", root.display()),
            _ => ", caching disabled".to_owned(),
        }
    );
    if statistics.remembered_timeouts > 0 {
        // Named on its own line because it is the one kind of entry that can be wrong: each
        // one is a page a reference is not being asked about at all. A count that grows is a
        // comparison quietly shrinking, which `PDFREF_CACHE=off` re-checks.
        println!(
            "  {} of those were remembered timeouts, so that many reference renders did not \
             happen at all",
            statistics.remembered_timeouts
        );
    }

    // The slowest pages, because a parallel run's wall clock is its longest pole and nothing
    // else in this report says which page that is. With the references cached the whole run
    // is a few dozen seconds of processor time spread over every core, and a single document
    // given 30 seconds by a reference that never returns is then most of what is left.
    let mut slowest: Vec<&Examined> = results.iter().collect();
    slowest.sort_by_key(|examined| std::cmp::Reverse(examined.spent.total()));
    let slowest: Vec<String> = slowest
        .iter()
        .take(5)
        .map(|e| format!("{} {:.1}s", e.name, e.spent.total().as_secs_f64()))
        .collect();
    println!("  slowest pages: {}", slowest.join(", "));

    let summary = |name: &str, predicate: &dyn Fn(&Examined) -> bool| {
        println!(
            "  {name:<20} {:>4} total, {:>4} of them on pages we call complete",
            count(predicate),
            count(&|e| predicate(e) && e.complete)
        );
    };
    summary("agrees", &|e| matches!(e.verdict, Verdict::Agrees));
    summary("contradicted", &|e| {
        matches!(e.verdict, Verdict::Contradicted(_))
    });
    summary("ambiguous", &|e| matches!(e.verdict, Verdict::Ambiguous(_)));
    summary("our geometry", &|e| {
        matches!(e.verdict, Verdict::OurGeometry(_))
    });
    summary("reference geometry", &|e| {
        matches!(e.verdict, Verdict::ReferenceGeometry(_))
    });
    summary("not comparable", &|e| {
        matches!(e.verdict, Verdict::NotComparable(_))
    });
    summary("no render", &|e| matches!(e.verdict, Verdict::NoRender(_)));

    // The population `pdfref::consensus_abstentions` can reach, printed rather than ratcheted.
    // A ratchet on it would be a ratchet on how often three other programs fail, which is not
    // this tree's to hold; what the number is for is that a rule refusing a vote must say how
    // many votes it refused, and on how many pages that left nothing to compare.
    println!(
        "  a reference returned one colour and took no part in the consensus on {} pages, {} of \
         which are left with fewer than two readings and are therefore not comparable",
        count(&|e| e.abstentions > 0),
        count(&|e| e.abstentions > 0 && matches!(e.verdict, Verdict::NotComparable(_)))
    );

    // The other way a reading can be missing, and the one that had no line at all until ADR
    // 0542: a reference that produced nothing, on a page still judged by the two that did.
    // Printed for the same reason as the line above — a verdict reached on a smaller
    // population than the page beside it has to say so, or a page can move with every input
    // unchanged and the output identical.
    println!(
        "  a reference produced no raster at all, leaving the verdict to the other two, on {} \
         pages",
        count(&|e| !e.absent.is_empty())
    );
    the_censuses_beside_the_verdicts(results);

    rank_the_pools(results);
}

/// The five things this gate counts that no ratchet holds, in the order they are read.
///
/// Each of them is a fact about the *references* rather than about this tree — how often one of
/// them drew nothing, how often a third could not read the file, how often the two that agreed
/// are one raster twice — so none is a ratchet and all of them are printed. They are gathered
/// here rather than listed in [`report`] because that is a function whose length is a lint, and
/// because the order matters to a reader: what was missing, then what was flat, then what the
/// consensus was made of, then what a different consensus would have decided.
fn the_censuses_beside_the_verdicts(results: &[Examined]) {
    name_the_pages_judged_without_a_third_reading(results);
    name_the_pages_with_a_divided_consensus(results);
    what_the_flat_sheets_said(results);
    what_the_consensus_was_made_of(results);
    a_raised_formation_bound(results);
}

/// How many verdicts rest on a consensus that is one raster counted twice, and which ones.
///
/// # Why this is a census and not a ratchet
///
/// It counts how often three *other* programs produce the same bytes, which is not this tree's
/// to hold — the same reason the abstention and absent-reference lines above it are printed
/// rather than gated. What it is for is that `Tolerance::widened_to`'s relative bound is the
/// whole reason this gate judges the way it does, and on a consensus at a spread of zero it
/// decides nothing at all: twice zero is zero, so the excluded reference and our own render are
/// both held to the bare class floor. Until this line nothing said how large that population is.
///
/// # What it is measured against
///
/// ADR 0773 measured identity over one row of one table and found 97 of 117. Its own last
/// sentence asked the mirror question and could not answer it: *should a consensus whose two
/// rasters are identical be a consensus at all?* — which needs the base rate over the population
/// that row was drawn out of, exactly as ADR 0771's control needed the pool it was invented on to
/// be widened before it could be read. This is that base rate, and ADR 0774 is what it decided.
///
/// The breakdown by class is the discriminator rather than decoration. Two independent programs
/// producing the same bytes over a page of hinted glyphs would be remarkable; over a flat
/// axis-aligned fill it is what a page with one correct answer looks like, and
/// `Tolerance::widened_to`'s own doc comment has said so since it was written — *a spread of
/// zero — two references producing identical pixels, which happens on simple pages*.
fn what_the_consensus_was_made_of(results: &[Examined]) {
    let judged: Vec<&ConsensusIdentity> = results
        .iter()
        .filter_map(|examined| examined.consensus_identity.as_ref())
        .collect();
    if judged.is_empty() {
        return;
    }
    let identical = judged.iter().filter(|c| c.identical()).count();
    let at_the_floor = judged.iter().filter(|c| c.at_the_class_floor).count();
    println!(
        "  a consensus decided {} pages; on {identical} of them ({:.1}%) every pair in that set \
         is byte-identical, so `widened_to` widened nothing (ADR 0774)",
        judged.len(),
        share(identical, judged.len()) * 100.0,
    );
    println!(
        "    and on {at_the_floor} ({:.1}%) the bound never left the class floor, which is the \
         population the relative bound decided nothing on — identity is a subset of it",
        share(at_the_floor, judged.len()) * 100.0,
    );

    // Each pool with those pages set aside, which is the half of the question a total cannot
    // answer: a verdict resting on one reading counted twice is a different kind of verdict from
    // one resting on two, and the pools do not carry them in the same proportion.
    let pool = |label: &str, predicate: &dyn Fn(&Examined) -> bool| {
        let carrying: Vec<&ConsensusIdentity> = results
            .iter()
            .filter(|examined| predicate(examined))
            .filter_map(|examined| examined.consensus_identity.as_ref())
            .collect();
        if carrying.is_empty() {
            return;
        }
        let one_raster = carrying.iter().filter(|c| c.identical()).count();
        println!(
            "    {label:<14} {:>4} judged by a consensus, {one_raster:>4} of them by identical \
             rasters ({:.1}%), leaving {:>4} judged by two readings",
            carrying.len(),
            share(one_raster, carrying.len()) * 100.0,
            carrying.len().saturating_sub(one_raster),
        );
    };
    pool("agrees", &|e| matches!(e.verdict, Verdict::Agrees));
    pool("contradicted", &|e| {
        matches!(e.verdict, Verdict::Contradicted(_))
    });
    pool("ambiguous", &|e| matches!(e.verdict, Verdict::Ambiguous(_)));

    // The one sub-population small enough to read page by page and sharp enough to be worth it:
    // a conviction whose whole evidence is one raster counted twice. Named rather than counted,
    // which is ADR 0772's rule — a population handed on in prose is a population the next round
    // cannot check — and it is what a round taking this question further starts from.
    let convicted_by_one_raster: Vec<(&str, String)> = results
        .iter()
        .filter(|examined| matches!(examined.verdict, Verdict::Contradicted(_)))
        .filter_map(|examined| {
            examined
                .consensus_identity
                .as_ref()
                .filter(|consensus| consensus.identical())
                .map(|consensus| (examined.name.as_str(), consensus.set()))
        })
        .collect();
    println!(
        "    and these {} contradicted pages are convicted by a set whose members drew the same \
         bytes:",
        convicted_by_one_raster.len()
    );
    for (name, set) in convicted_by_one_raster {
        println!("      {name} — {set}");
    }

    // Which set, and which class of page. A count that is all one pair is a fact about two
    // programs; one spread across every pair is a fact about the pages.
    let mut by_set: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut by_class: std::collections::BTreeMap<&'static str, (usize, usize)> =
        std::collections::BTreeMap::new();
    for consensus in &judged {
        let seen = by_class.entry(consensus.class).or_default();
        seen.1 = seen.1.saturating_add(1);
        if consensus.identical() {
            seen.0 = seen.0.saturating_add(1);
            let set = by_set.entry(consensus.set()).or_default();
            *set = set.saturating_add(1);
        }
    }
    for (class, (one_raster, all)) in &by_class {
        println!(
            "    {class:<14} {all:>4} judged by a consensus, {one_raster:>4} of them by identical \
             rasters ({:.1}%)",
            share(*one_raster, *all) * 100.0,
        );
    }
    let mut sets: Vec<(&String, &usize)> = by_set.iter().collect();
    sets.sort_by_key(|(set, count)| (std::cmp::Reverse(**count), (*set).clone()));
    for (set, count) in sets {
        println!("      {count:>4}  {set}");
    }
}

/// What raising the bound that *forms* a consensus would decide, and what that population is
/// made of.
///
/// # Why this is a census and not a proposal
///
/// `doc/todo/12` item 1 is the one half of that item nobody had measured: raising
/// `Tolerance::max_differing_fraction` for consensus **formation** makes several hundred
/// `ambiguous` pages judgeable, and ADR 0243 recorded that 278 of them arrive contradicted. That
/// count is the whole of what was known about them — not their class, not which pair of
/// references would convict, not which measure the conviction would rest on, and not whether the
/// pair is one trap 9 names. A number with no composition behind it cannot be argued with, and it
/// is exactly the shape ADR 0771 and ADR 0774 each had to widen a denominator to answer.
///
/// So the gate counts it, every run, off comparisons it already has. Nothing here moves a verdict:
/// the live gate is judged by `Triangulation::outcome` and this reads a counterfactual beside it.
///
/// # What to read off it
///
/// The transition matrix says how large the population is and which way it moves. The breakdown
/// says what the newly convicting consensuses **are**: which pair forms them, at what spread they
/// form, how many are one raster counted twice, and which of the four measures then convicts us.
/// A conviction resting on the differing fraction of a pair that had to be admitted *by raising
/// the differing fraction* is a different object from one resting on structural similarity.
fn a_raised_formation_bound(results: &[Examined]) {
    let judged: Vec<&RaisedFormation> = results
        .iter()
        .filter_map(|examined| examined.raised_formation.as_ref())
        .filter(|raised| raised.now != Standing::Unjudged)
        .collect();
    if judged.is_empty() {
        return;
    }

    let uncalibrated: Vec<&str> = results
        .iter()
        .filter(|examined| {
            examined
                .raised_formation
                .as_ref()
                .is_some_and(|page| page.control != page.now)
        })
        .map(|examined| examined.name.as_str())
        .collect();
    assert!(
        uncalibrated.is_empty(),
        "re-judging at a page's own bounds must reproduce its own verdict: {uncalibrated:?}"
    );

    println!(
        "  raising the bound that forms a consensus to the references' own 99th percentile \
         (0.12 text, 0.0136 vector) over {} judged pages — `doc/todo/12` item 1, counterfactual \
         only:",
        judged.len()
    );

    // The transition matrix, both arms. `formation_only` is the change the item describes;
    // `with_the_floor` is ADR 0243's arm, and the difference between them is what the floor half
    // is worth on this population.
    let arm = |label: &str, standing: &dyn Fn(&RaisedFormation) -> Standing| {
        let mut moved: std::collections::BTreeMap<(Standing, Standing), usize> =
            std::collections::BTreeMap::new();
        for page in &judged {
            let seen = moved.entry((page.now, standing(page))).or_default();
            *seen = seen.saturating_add(1);
        }
        println!("    {label}:");
        for ((from, to), count) in &moved {
            if from == to {
                continue;
            }
            println!("      {:>4}  {} -> {}", count, from.word(), to.word());
        }
        let unmoved = moved
            .iter()
            .filter(|((from, to), _)| from == to)
            .map(|(_, count)| *count)
            .sum::<usize>();
        println!("      {unmoved:>4}  unchanged");
    };
    arm("our floor left at the class bound", &|page| {
        page.formation_only
    });
    arm("the floor raised with it (ADR 0243's arm)", &|page| {
        page.with_the_floor
    });

    what_the_new_convictions_are_made_of(results);
    the_pages_a_raised_formation_bound_would_move(results);
}

/// The breakdown of the pages a raised formation bound would newly convict.
///
/// Split out of [`a_raised_formation_bound`] because the matrix above answers *how many* and this
/// answers *of what*, and only the second is an argument. Every row here is over one population:
/// a page the gate cannot judge today, convicted by a consensus the raise manufactured, with our
/// own floor left where it is.
fn what_the_new_convictions_are_made_of(results: &[Examined]) {
    let newly_convicted: Vec<(&str, &RaisedFormation)> = results
        .iter()
        .filter_map(|examined| {
            examined
                .raised_formation
                .as_ref()
                .map(|page| (examined.name.as_str(), page))
        })
        .filter(|(_, page)| {
            page.now == Standing::Ambiguous && page.formation_only == Standing::Contradicted
        })
        .collect();
    if newly_convicted.is_empty() {
        return;
    }
    println!(
        "    what those {} newly contradicted pages are made of:",
        newly_convicted.len()
    );

    let breakdown = |label: &str, key: &dyn Fn(&str, &RaisedFormation) -> String| {
        let mut counts: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for (name, page) in &newly_convicted {
            let seen = counts.entry(key(name, page)).or_default();
            *seen = seen.saturating_add(1);
        }
        let mut rows: Vec<(&String, &usize)> = counts.iter().collect();
        rows.sort_by_key(|(name, count)| (std::cmp::Reverse(**count), (*name).clone()));
        println!("      by {label}:");
        for (name, count) in rows {
            println!("        {count:>4}  {name}");
        }
    };
    breakdown("tolerance class", &|_, page| page.class.to_owned());
    // The document, which is the row that decides how large this population really is: a
    // programme of 278 diagnoses and a programme of five are different objects, and the corpus
    // is a handful of long books beside a thousand one-page reductions.
    breakdown("the document", &|name, _| {
        name.rsplit_once(" page ")
            .map_or_else(|| name.to_owned(), |(document, _)| document.to_owned())
    });
    breakdown("the set that would convict", &|_, page| {
        page.set.clone().unwrap_or_else(|| "no set".to_owned())
    });
    breakdown("the measure the conviction rests on", &|_, page| {
        page.convicted_on
            .map_or_else(|| "none".to_owned(), |(_, measure)| measure.to_owned())
    });
    breakdown(
        "how far past the class bound the pair had to be admitted",
        &|_, page| {
            // Bands rather than a distribution, because what the decision turns on is whether a
            // smaller raise would leave most of this population behind: a pair admitted at 5.3% is
            // one the class bound nearly admitted, and one admitted at 11% is not.
            match page.formed_at {
                None => "no set".to_owned(),
                Some(fraction) if fraction <= 0.06 => "within a fifth of the raise".to_owned(),
                Some(fraction) if fraction <= 0.08 => "within two fifths".to_owned(),
                Some(fraction) if fraction <= 0.10 => "within three fifths".to_owned(),
                Some(_) => "the last two fifths".to_owned(),
            }
        },
    );
    println!(
        "        {:>4}  of them are a set whose members drew the same bytes",
        newly_convicted
            .iter()
            .filter(|(_, page)| page.identical)
            .count()
    );
    println!(
        "        {:>4}  of them have a reference nearer to *us*, on the deciding measure, than the convicting set is to itself",
        newly_convicted
            .iter()
            .filter(|(_, page)| page.nearer_than_the_pair)
            .count()
    );

    // Named rather than counted, which is ADR 0772's rule, and capped because this list is two
    // orders of magnitude larger than the populations that rule was written for. The cap is what
    // `PDFVIEWER_ORACLE_FORMATION=1` lifts; the counts above it are always complete.
    let all = std::env::var_os("PDFVIEWER_ORACLE_FORMATION").is_some();
    let shown = if all {
        newly_convicted.len()
    } else {
        25.min(newly_convicted.len())
    };
    println!(
        "      the pages themselves ({shown} of {}{}):",
        newly_convicted.len(),
        if all {
            ""
        } else {
            ", PDFVIEWER_ORACLE_FORMATION=1 for all of them"
        }
    );
    for (name, page) in newly_convicted.iter().take(shown) {
        println!("        {name} — {}", describe_a_counterfactual(page));
    }
}

/// One page's counterfactual, as the two lists that print it both want it — without the page's
/// own name, which each of the two prefixes differently.
fn describe_a_counterfactual(page: &RaisedFormation) -> String {
    format!(
        "{} would convict, formed at {:.2}% differing, {}",
        page.set.as_deref().unwrap_or("no set"),
        page.formed_at.unwrap_or(0.0) * 100.0,
        page.convicted_on.map_or_else(
            || "on nothing".to_owned(),
            |(ratio, measure)| format!("on {measure} at {ratio:.2}\u{d7} its bound")
        ),
    )
}

/// The pages a raised formation bound would move *out* of a pool the gate holds by name.
///
/// The census above is about pages nobody can judge today. This is the other direction and it is
/// the one a ratchet would meet first: a page that agrees today and would not, and a page that is
/// contradicted today and would not be. Both populations are small enough to name outright, which
/// is what ADR 0772's rule asks for and what the 276 above are too many for.
fn the_pages_a_raised_formation_bound_would_move(results: &[Examined]) {
    let moved = |from: Standing| {
        let pages: Vec<String> = results
            .iter()
            .filter_map(|examined| {
                examined
                    .raised_formation
                    .as_ref()
                    .map(|page| (examined.name.as_str(), page))
            })
            .filter(|(_, page)| page.now == from && page.formation_only != from)
            .map(|(name, page)| {
                format!(
                    "{name} — {} -> {}, {}",
                    page.now.word(),
                    page.formation_only.word(),
                    describe_a_counterfactual(page)
                )
            })
            .collect();
        if pages.is_empty() {
            return;
        }
        println!(
            "    {} pages leave `{}` under the same raise, with our floor unchanged:",
            pages.len(),
            from.word()
        );
        for page in pages {
            println!("      {page}");
        }
    };
    moved(Standing::Agrees);
    moved(Standing::Contradicted);
}

/// The whole population `pdfref::Reference::refusals` decides over, and what it matched in it.
///
/// Trap 11's audit, printed rather than remembered: **both** the sentences the condition fired
/// on and the distinct sentences of every flat sheet it did not fire on. A condition on a
/// renderer's words is a claim about a vocabulary three other projects own, so it decays the way
/// a ledger row does — a release that rewords `cannot decode jbig2 image` takes the rule with
/// it, silently, and this is the line that says so. It is not a ratchet, for the same reason the
/// abstention count above it is not: how often three other programs fail is not this tree's to
/// hold.
fn what_the_flat_sheets_said(results: &[Examined]) {
    let sheets: Vec<&FlatSheet> = results
        .iter()
        .flat_map(|examined| examined.flat_sheets.iter())
        .collect();
    if sheets.is_empty() {
        return;
    }

    // Keyed by the renderer as well as the sentence: the same words reach the log through two
    // different programs — `jbig2dec`'s do — and which program carried them is half of what a
    // round auditing this condition needs.
    let mut refused: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut quiet: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut silent = 0usize;
    for sheet in &sheets {
        match &sheet.refusal {
            Some(sentence) => {
                let seen = refused
                    .entry(format!("{:<12} {sentence}", sheet.reference.name()))
                    .or_default();
                *seen = seen.saturating_add(1);
            }
            None if sheet.said.is_empty() => silent = silent.saturating_add(1),
            None => {
                for line in &sheet.said {
                    let seen = quiet
                        .entry(format!("{:<12} {line}", sheet.reference.name()))
                        .or_default();
                    *seen = seen.saturating_add(1);
                }
            }
        }
    }

    let matched: usize = refused.values().sum();
    println!(
        "  flat sheets: {} reference rasters of one colour, {matched} of them naming a refusal \
         in the renderer's own words, {silent} saying nothing at all",
        sheets.len()
    );
    let commonest = |what: &str, lines: &std::collections::BTreeMap<String, usize>| {
        let mut ordered: Vec<(&String, &usize)> = lines.iter().collect();
        ordered.sort_by_key(|(line, count)| (std::cmp::Reverse(**count), (*line).clone()));
        println!("    {what}: {} distinct sentences", ordered.len());
        for (line, count) in ordered.iter().take(FLAT_SHEET_SENTENCES) {
            println!("      {count:>4}  {line}");
        }
        if ordered.len() > FLAT_SHEET_SENTENCES {
            println!(
                "      … and {} more",
                ordered.len().saturating_sub(FLAT_SHEET_SENTENCES)
            );
        }
    };
    commonest("refusals matched", &refused);
    commonest("said by a flat sheet the condition did not match", &quiet);
}

/// How many distinct sentences of each side of that audit are printed.
///
/// A bound rather than the whole set, because `mupdf` narrates the path of every document it
/// opens and that alone would be one line per page. The commonest are what a round auditing the
/// condition reads; the rest are in the run's own `<name>.log` files beside the rasters.
const FLAT_SHEET_SENTENCES: usize = 25;

/// The four orderings, in one call because they are one section of the report.
///
/// Each says what it is for in its own comment; what they share is that none of them is a ratchet
/// and none decides a verdict. They are where the next round's page comes from.
fn rank_the_pools(results: &[Examined]) {
    rank_the_undiagnosed(results);
    rank_the_manufactured_ambiguity(results);
    rank_the_pages_we_are_alone_on(results);
    rank_the_contradicted(results);
    rank_the_contradicted_by_the_bound(results);
}

/// Which pages were judged on two readings, against the list that says why for each.
///
/// **A count beside a list is not the list** (`doc/todo/02` §6): the summary line above says how
/// many pages lost a reading, and until this the seventh one arriving would have been the number 6
/// becoming 7. Now it is a name printed under a sentence asking why, which is the same shape every
/// other population in this file has. See [`JUDGED_WITHOUT_A_THIRD_READING`] for the six and the
/// reading of each, and ADR 0575 for why the *count* of references is not what the six are about.
fn name_the_pages_judged_without_a_third_reading(results: &[Examined]) {
    let unnamed: Vec<&str> = results
        .iter()
        .filter(|examined| !examined.absent.is_empty())
        .map(|examined| examined.name.as_str())
        .filter(|name| !JUDGED_WITHOUT_A_THIRD_READING.contains(name))
        .collect();
    if unnamed.is_empty() {
        println!("    every one of them is named in JUDGED_WITHOUT_A_THIRD_READING with why");
    } else {
        println!(
            "    and {} of them are on no list, so nobody has asked why the third reading is \
             missing: {unnamed:?}",
            unnamed.len()
        );
    }
}

/// Where the references formed more than one consensus, and where those sets disagree about us.
///
/// **Agreement is not transitive**, and until the seven-hundred-and-twenty-seventh session
/// nothing in this tree had said so out loud. `pdfref::decide` takes the largest mutually
/// agreeing set of references; where two sets tie for largest — `poppler` with `ghostscript`
/// and `ghostscript` with `mupdf`, on a page where `poppler` and `mupdf` differ — it took
/// whichever the subset enumeration reached first and discarded the other without counting it.
/// On a page where the two reach different verdicts about our render, that was a verdict decided
/// by the order the `Reference` values are declared in (ADR 0616). Since ADR 0617 a verdict is
/// one every maximal consensus reaches and such a page is `ambiguous`, which is what these lines
/// report.
///
/// Printed rather than ratcheted, on the same argument as the abstention line above: the count
/// is a fact about how often three other programs fail to form one answer, which is not this
/// tree's to hold. What a round owes is a *name* for each page it fires on, which is why the
/// pages are listed rather than summed (`doc/todo/02` §6) — [`AMBIGUOUS_DIVIDED_CONSENSUS`] is
/// that list and carries the reading of each.
///
/// **The third number is the one that says the rule is not a one-way ratchet**: pages carrying
/// several sets that *concur* in agreeing with us. Every one of them is a page a moved pixel can
/// divide, and dividing it costs an agreement — which is the direction none of the population
/// this rule was adopted on was in.
fn name_the_pages_with_a_divided_consensus(results: &[Examined]) {
    let several = results
        .iter()
        .filter(|examined| examined.consensuses > 1)
        .count();
    let divided: Vec<&Examined> = results
        .iter()
        .filter(|examined| examined.divided_by.is_some())
        .collect();
    let concurring_agreements = results
        .iter()
        .filter(|examined| examined.consensuses > 1 && examined.divided_by.is_none())
        .filter(|examined| matches!(examined.verdict, Verdict::Agrees))
        .count();
    println!(
        "  the references formed more than one maximal consensus on {several} pages, and on {} of \
         those the sets disagree about us, so the page is ambiguous rather than judged by either; \
         on {concurring_agreements} of the rest the sets concur in agreeing with us, and a moved \
         pixel that divided one would cost that agreement",
        divided.len()
    );
    for examined in divided {
        let sets = examined.divided_by.as_deref().unwrap_or("");
        let listed = if AMBIGUOUS_DIVIDED_CONSENSUS.contains(&examined.name.as_str()) {
            ""
        } else {
            " — ON NO LIST, so nobody has read it"
        };
        println!(
            "    {}: {} — {sets}{listed}",
            examined.name,
            examined.verdict.label()
        );
    }
}

/// The ten contradicted pages we sit furthest from *every* reference on.
///
/// The ambiguous bucket has had a ranking since the hundred-and-seventy-sixth session and the
/// contradicted list, which is four hundred sessions older, has never had one — it was ordered
/// by nothing, and a round picking a page off it picked by eye. [`Distance`] is computed for
/// every page already, so this costs one sort.
///
/// It is the same instrument and it answers the same question, which `doc/todo/00`'s step 1
/// states: **where the two numbers are close we are alone, and where they are far apart the
/// references are the ones disagreeing.** On the contradicted list the second shape has a
/// specific and common cause worth naming — a page whose two agreeing references agree by
/// both declining to draw something. `CONTRADICTED_LINK_BORDER`'s three pages sit at 5.4 to
/// 5.9 of 255 from their nearest reference while the references' own spread is 8.5 to 9.6,
/// because `mupdf` constructs no appearance for a link and `ghostscript` is answering a
/// question about printing; the number that accuses us is small and the one beside it is the
/// group's whole argument.
///
/// # Two rankings, and the difference between them is not a defect in either
///
/// [`Distance`] is in **bounds**, which is what makes a page comparable with one held to a
/// different tolerance class, and it is the unit every "0.16 from the nearest reference" in
/// this file is written in. The four-hundred-and-sixth session also built the same ranking in
/// **levels of 255**, by hand, off the artefacts — and the two put different pages first,
/// which is worth knowing before either is read as *the* order.
///
/// In levels the head is `bug847420.pdf` at 8.65 from the nearest of four renderers that agree
/// among themselves to 4.64: twice as far as any page on the list that is not a link border.
/// In bounds it is nowhere near the head, because a text page is held to a mean of 5.00 where
/// a vector page is held to 1.00, and the seven JBIG2 pages take the top instead at 2.45 to
/// 28.91 — `poppler` reporting *Too many symbols in JBIG2 symbol dictionary* on a page whose
/// tolerance class is the strict one. **Bounds ask "is this page outside what its own
/// references allow"; levels ask "how much of the page is different."** Both are printed
/// elsewhere and only the first can be ranked across the whole list, which is why it is the
/// one here.
///
/// # And there is a third, which is the one `doc/habits.md` asks for
///
/// [`rank_the_contradicted_by_the_bound`] below, since ADR 0636. This one measures our distance
/// from the **nearest** reference in [`Distance`]'s three measures, which is a question about the
/// page; that one measures how far outside its bound the **consensus that convicts us** puts us,
/// which is a question about the verdict. They are printed together and its own comment says what
/// each is for.
fn rank_the_contradicted(results: &[Examined]) {
    let mut ranked: Vec<(&Examined, Distance)> = results
        .iter()
        .filter(|e| e.complete && matches!(e.verdict, Verdict::Contradicted(_)))
        .filter_map(|e| e.distance.map(|d| (e, d)))
        .collect();
    ranked.sort_by(|(_, a), (_, b)| {
        b.nearest
            .partial_cmp(&a.nearest)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    println!("\n  contradicted, and furthest from the nearest reference:");
    for (examined, distance) in ranked.iter().take(10) {
        println!(
            "    {:>6.2} nearest {:>7.2} furthest  {}",
            distance.nearest, distance.furthest, examined.name
        );
    }
}

/// The ten contradicted pages furthest outside the bound they are held to.
///
/// **The ordering ADR 0349 argued for and left unwritten**, and three rounds in a row recorded as
/// owed after it. `doc/habits.md` asks the pool to be ranked by *our worst measurement over the
/// bound it is held to*; [`rank_the_contradicted`] above orders by distance from the nearest
/// reference, which is the ambiguous bucket's instrument borrowed unchanged. The two answer
/// different questions and both are printed — one names the page the references are furthest from,
/// this one names the page furthest outside what it is held to.
///
/// # What the borrowed instrument cannot see, priced rather than asserted
///
/// [`Distance::of`] reduces a comparison to **three** ratios — mean, worst tile, structural
/// similarity — and the differing fraction is not among them. That is deliberate and stated there:
/// a hundred entries in this file quote a `Distance` figure and those numbers have to keep meaning
/// what they meant. What nobody had priced is the consequence for the *ranking* built on it, and it
/// is the same defect ADR 0242 found in the per-page **line** one level up, surviving in the order
/// the lines are printed in: the differing fraction is the bound most of this pool fails on, so a
/// ranking blind to it is blind to most of the pool's accusations. The gate's own run says by how
/// much — the census line below counts the pages whose every other measure is *inside* the bound,
/// and each of those has a `Distance` at or under 1.0, which is the unit's way of saying *nothing
/// here is wrong*.
///
/// So this is not a replacement. It is the second ordering `doc/oracle-and-corpus.md` §3b asks a
/// reader to take, made mechanical so that a round choosing a page off the pool no longer takes it
/// by hand from a log.
///
/// # How to read it
///
/// The number is [`Examined::outside_the_bound`] and the word beside it names which of the four
/// measures it is a ratio of, because 29× on the differing fraction and 29× on the mean are two
/// different pages. Above 1.0 by construction — a page inside every bound is not contradicted —
/// and the distance from 1.0 is what the page's standing exemption is worth. **A page just above
/// 1.0 is trap 12's arithmetic and not a defect's size**: `issue6069.pdf` sits at the foot of this
/// list because its whole verdict is six differing channels of eighty thousand (ADR 0606).
///
/// Nothing here is a ratchet and nothing here decides a verdict. It is a place to look, and its
/// head is a page every group in this file already names.
fn rank_the_contradicted_by_the_bound(results: &[Examined]) {
    let mut ranked: Vec<(&Examined, f64, &str)> = results
        .iter()
        .filter(|e| matches!(e.verdict, Verdict::Contradicted(_)))
        .filter_map(|e| e.outside_the_bound.map(|(ratio, of)| (e, ratio, of)))
        .collect();
    ranked.sort_by(|(_, a, _), (_, b, _)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    // Unlike its neighbour this ranking does **not** filter on `complete`, and that is ADR 0349's
    // own finding rather than an oversight: `check_the_ratchets` filters on it for a reason the
    // module states, and the consequence was that the two largest disagreements on the whole pool
    // sat outside every diagnosis in this file. Both are diagnosed now, and a ranking that hid
    // them again would be re-creating the hole. The label says which they are.
    println!("\n  contradicted, and furthest outside the bound it is held to:");
    for (examined, ratio, of) in ranked.iter().take(10) {
        println!(
            "    {ratio:>7.2}x on the {of:<22} {}{} — {}",
            examined.name,
            if examined.complete {
                ""
            } else {
                " (incomplete)"
            },
            held_by(&examined.name).map_or_else(
                || "held by no group".to_owned(),
                |group| format!("held by {group}")
            )
        );
    }
    name_the_pages_no_group_holds(&ranked);

    // The pool's shape in one line, and it is `doc/todo/12`'s population counted by the gate that
    // makes it rather than by a round with a log. That item says most of this pool fails the
    // differing fraction and no other bound; what it could not say is by how much, because until
    // this ranking nothing put the pool in that unit.
    let on_the_differing_fraction: Vec<f64> = ranked
        .iter()
        .filter(|(_, _, of)| *of == "differing fraction")
        .map(|(_, ratio, _)| *ratio)
        .collect();
    let range = |values: &[f64]| {
        values.iter().fold((f64::MAX, f64::MIN), |(low, high), v| {
            (low.min(*v), high.max(*v))
        })
    };
    if !on_the_differing_fraction.is_empty() {
        let (low, high) = range(&on_the_differing_fraction);
        println!(
            "    of the {} pages, {} are furthest outside on the differing fraction, between \
             {low:.2}x and {high:.2}x — the bound `doc/todo/12` is about",
            ranked.len(),
            on_the_differing_fraction.len(),
        );
        // *Which consensus* convicts them is as much of the population's shape as the measure.
        // `pdftoppm` and `mutool` load one `libfreetype.so.6` between them where `gs` links no
        // FreeType and carries its own statically linked copy (trap 9's tenth mechanism,
        // re-checked on this machine's binaries with `objdump -p` in the seven-hundred-and-
        // -eightieth session) — so a page whose only agreeing pair is `poppler` and `mupdf` is
        // held to a bound derived from the one voting pair that shares a glyph rasteriser. The
        // prefix match is against `verdict_of`'s own format string, and a three-way consensus
        // reads "poppler and mupdf and ghostscript agree", which it deliberately does not match.
        //
        // ADR 0717 measured what the count means: on such a page `ghostscript` fails this same
        // bound against *both* members of the convicting pair, so the verdict rests on a bound a
        // voting reference sits outside on the same page. **That was written as "on every page of
        // this population" and it is not**, which the eight-hundred-and-forty-fifth session found
        // by asking the gate for the exception instead of citing the ADR — so the second figure is
        // counted here rather than quoted, and the page that breaks it is named by the block below
        // (ADR 0772).
        let of_the_sharing_pair: Vec<_> = ranked
            .iter()
            .filter(|(examined, _, of)| {
                *of == "differing fraction"
                    && matches!(&examined.verdict, Verdict::Contradicted(description)
                        if description.starts_with("poppler and mupdf agree"))
            })
            .collect();
        let excluded_outside_of_both = of_the_sharing_pair
            .iter()
            .filter(|(examined, _, _)| {
                examined
                    .excluded_reference
                    .as_ref()
                    .is_some_and(ExcludedReading::outside_of_every_member)
            })
            .count();
        println!(
            "    and {} of those are convicted by `poppler` and `mupdf` alone, the one voting \
             pair that shares a glyph rasteriser (trap 9), with `ghostscript` outside the same \
             bound against both of them on {excluded_outside_of_both} of the {} — ADR 0717 \
             measured that as all of them",
            of_the_sharing_pair.len(),
            of_the_sharing_pair.len(),
        );
    }
    // And the base rate that figure has to be read against, which is what ADR 0717 could not know
    // from one population: on how much of the *whole* pool would the same consensus, at the same
    // bound, also contradict the voting reference it excludes. Printed rather than described,
    // because a control that holds nearly everywhere is not a discriminator and a round reading
    // the line above would otherwise take it for one. See
    // [`the_excluded_reference_under_the_same_bound`], and ADR 0771.
    println!(
        "    and on {} of the {} the consensus would contradict the voting reference it \
         excludes as well, at the same bound — so that control is the pool's base rate rather \
         than any population's signature (ADR 0771)",
        ranked
            .iter()
            .filter(|(examined, _, _)| examined
                .excluded_reference
                .as_ref()
                .is_some_and(ExcludedReading::convicted))
            .count(),
        ranked.len(),
    );
    name_the_pages_the_excluded_reference_survives(&ranked);
    // What the other ranking's unit says about the same pages: at or under 1.0 it says the page is
    // inside every bound it can see. Printed rather than described, because it is the whole reason
    // this second ordering exists.
    println!(
        "    and on {} of them every measure `Distance` can see is inside the bound, so the \
         ranking above them cannot order them at all",
        ranked
            .iter()
            .filter(|(examined, _, _)| examined.distance.is_some_and(|d| d.nearest <= 1.0))
            .count()
    );
}

/// The contradicted pages no `CONTRADICTED_*` group holds, which is the question a round handed
/// the ranking above is asked first — *is the page's cause already written down?* — answered for
/// the whole pool rather than for its ten printed rows.
///
/// Over every contradicted page, complete or not. The ratchet answers the question for the
/// complete ones and fails the build on a `no`; an incomplete page is outside it by construction,
/// and until this line nothing said whether one had arrived unheld — which is the population ADR
/// 0349 found outside every diagnosis in this file, counted every run instead of once (ADR 0805).
fn name_the_pages_no_group_holds(ranked: &[(&Examined, f64, &str)]) {
    let unheld: Vec<&str> = ranked
        .iter()
        .filter(|(examined, _, _)| held_by(&examined.name).is_none())
        .map(|(examined, _, _)| examined.name.as_str())
        .collect();
    if unheld.is_empty() {
        println!(
            "    every one of the {} pages is held by a group by name, so the next page to take \
             is the highest row whose note names a departure of ours rather than a reference's",
            ranked.len()
        );
    } else {
        println!(
            "    {} of the {} pages is held by no group, and each is the next page to take: {}",
            unheld.len(),
            ranked.len(),
            unheld.join(", ")
        );
    }
}

/// The contradicted pages on which the voting reference the consensus excludes meets the bound.
///
/// # Why this one is a list where its neighbour is a count
///
/// The line above prints the base rate — how often the consensus would contradict the excluded
/// reference as well — and a control that holds nearly everywhere is not a discriminator. Its
/// **complement** is: a page where an independent implementation is inside the bound our own
/// render is outside is the sharpest thing this pool produces, and it is small enough to read
/// page by page rather than to rank.
///
/// ADR 0771 measured the base rate and named the complement's members in prose, from a run it had
/// in front of it. Two of the three names were wrong — `issue19633.pdf` page 1 is convicted by the
/// control (`ghostscript` at structural similarity 0.98828 against `mupdf`, where the vector floor
/// is 0.9900) and `freeculture.pdf` page 313 was missing — and the round that inherited the list
/// spent its first hour reconstructing it. **A population handed on in prose is a population the
/// next round cannot check**, which is `CLAUDE.md`'s own rule about counted facts arriving on a
/// list of pages instead of on a number. ADR 0772.
fn name_the_pages_the_excluded_reference_survives(ranked: &[(&Examined, f64, &str)]) {
    let survives: Vec<_> = ranked
        .iter()
        .filter_map(|(examined, _, _)| {
            examined
                .excluded_reference
                .as_ref()
                .filter(|reading| !reading.convicted())
                .map(|reading| (&examined.name, reading.reference))
        })
        .collect();
    println!(
        "    and on {} of them the voting reference the consensus excludes meets that same bound \
         while we do not — the population `doc/todo/12`'s consensus half is read from (ADR 0772):",
        survives.len(),
    );
    for (name, reference) in survives {
        println!("      {name} — {} is inside the bound", reference.name());
    }
}

/// The ten ambiguous pages on which the *references* missed each other by the most.
///
/// # The number nothing printed
///
/// Every other ranking in this file measures **us**. This one measures the verdict: an
/// `ambiguous` page is one where no two voting references agreed, and
/// [`Examined::consensus_missed_by`] says by how much the closest pair missed. It is the
/// instrument trap 9's fifth shape asks for and did not have — *shared code does not only
/// manufacture agreement; it can also manufacture the absence of one, and the second is
/// invisible where the first is at least listed.*
///
/// Read it with [`rank_the_undiagnosed`], which is ordered by our own distance. The two
/// disagree at the head on purpose, and the five-hundred-and-eighteenth session's reading of
/// this one is in `doc/todo/00-ambiguous-bucket.md`:
///
/// - **Very large** is a renderer that failed. The bucket's head is
///   `bitmap-refine-tpgron.pdf`, where `mupdf` renders the page black and `ghostscript`
///   renders it white — 255.00 of 255 apart from each other, which is the whole range —
///   because both are `jbig2dec` giving up on a refinement region in two different ways.
///   `AMBIGUOUS_SHARED_JBIG2_DECODER` by name.
/// - **Moderate with us outside it** is usually the closest pair agreeing through a *shared
///   gap* rather than through a reading. On `bug766086.pdf` the closest pair is `mupdf` and
///   `ghostscript`, and the thing they agree about is drawing no link border — for two
///   unrelated reasons, which is trap 9's fourth shape. On `bug1743245.pdf` it is the same
///   two, and what they share is ignoring §10.7.5's stroke-adjustment sentence.
/// - **A little above 1** is trap 12's arithmetic: a pair that missed by a rounding step.
///
/// Nothing here is a ratchet and nothing here decides a verdict. It is a place to look.
///
/// # The two columns are two instruments, and were read as one
///
/// [`Examined::consensus_missed_by`] is [`outside_by`] — all four of [`Tolerance::accepts`]'
/// measures — and the `ours` column beside it was [`Distance::nearest`], which is three of them.
/// The paragraph above and `doc/todo/00`'s step 1 both ask a reader to compare the two, and on
/// this pool that comparison was between different quantities: taken as a ratio the printed pair
/// names 13 of the 804 complete ambiguous pages as ones we are alone on, where the same question
/// asked in one unit names 48 in three measures and 569 in four. So ours is printed in **both**
/// units. [`nearest_on_every_measure`] has the measurement and the reason [`Distance`] itself was
/// left at three, and [`rank_the_pages_we_are_alone_on`] — printed directly beneath this list — is
/// where that comparison is counted and ordered.
fn rank_the_manufactured_ambiguity(results: &[Examined]) {
    let mut ranked: Vec<(&Examined, f64)> = results
        .iter()
        .filter(|e| e.complete && matches!(e.verdict, Verdict::Ambiguous(_)))
        .filter_map(|e| e.consensus_missed_by.map(|missed| (e, missed)))
        .collect();
    ranked.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    println!(
        "\n  ambiguous, and the closest two references missed each other by the most \
         (their number is over four measures, so ours is printed in three and in four):"
    );
    for (examined, missed) in ranked.iter().take(10) {
        let ours = examined
            .distance
            .map_or_else(|| "  -  ".to_owned(), |d| format!("{:5.2}", d.nearest));
        let every = examined
            .nearest_on_every_measure
            .map_or_else(|| "  -  ".to_owned(), |ratio| format!("{ratio:5.2}"));
        println!(
            "    {missed:>7.2} between them, {ours} ours in three measures, {every} in four  {}",
            examined.name
        );
    }
}

/// The ambiguous pages we sit further from every reference on than the closest two sit from
/// each other — `doc/todo/00` step 1's *we are alone*, counted and ordered.
///
/// [`rank_the_manufactured_ambiguity`] measures how hard the consensus failed; this asks the
/// question its list invites, over the whole pool and in one unit rather than read off ten lines.
/// It is a place to look and not a ratchet, exactly as that ranking is.
///
/// # Two counts, because the answer depends on which measures the question is asked in
///
/// The seven-hundred-and-forty-first session put the two columns of that list into comparable
/// units and counted the result in **four** measures, because that was the only unit the pair's
/// number was then available in (ADR 0643). Read that way the shape names seven pages in ten,
/// which is `doc/todo/12`'s bound arriving as a signal: the differing fraction is one the
/// references miss by nearly as much as we do. [`consensus_missed_in_three_measures`] supplies the
/// other unit, and it is the one that reproduces the reading session 518 took by hand — 6.9%
/// against 7.1% of a smaller pool — so the **three**-measure count is what orders the queue and
/// the four-measure count is printed beside it as the population it is drawn from.
///
/// # What the ratio is and is not
///
/// The head is the largest *ratio*, not the largest distance: a page where we are 3 bounds out
/// while the closest pair are 0.3 apart is the shape step 1 is about, and a page where everybody
/// is far away is not. Three cautions travel with it, and all three are in `doc/todo/00`:
///
/// - **A high ratio means "the closest two agree through a shared gap" at least as often as it
///   means anything about us.** `issue4260_reduced.pdf` and `bug1743245.pdf` are the standing
///   witnesses: a clause says we are right and the pair agrees by both departing from it.
/// - **The denominator can be small for a reason that is not agreement.** Below 1 is possible
///   here — [`outside_by_in_three_measures`] says when — so a pair that missed on the differing
///   fraction alone divides by a number under 1 and lifts its page up this list.
/// - **And it can be small because two references share a glyph rasteriser.** Over
///   `freeculture.pdf`'s 321 compared pages, `poppler` and `mupdf` — which share
///   `libfreetype.so.6` where `ghostscript` links its own copy — are the closest pair on 9 of the
///   11 that reach this list and on 7 of the other 310, at a median MAE of 724 against 1760. Trap
///   9 is a list of ways shared code
///   manufactures an agreement; in a ratio that agreement is the divisor, so the same mechanism
///   accuses us instead of excusing somebody (ADR 0647).
///
/// Read it with the picture, never alone.
///
/// # Why the numerator has to be outside a bound, and it is a filter rather than a caution
///
/// The seven-hundred-and-forty-fourth session read the list and found that on most of it *neither*
/// number is outside anything: the closest pair sat inside all three bounds on 31 of its 48 pages
/// and our own nearest was inside them too on 22. There the ratio ranks a page higher the more
/// closely the references agree, which answers a different question from the one it is named for.
/// So this list requires our own nearest to be **outside** at least one of the three bounds, and
/// the pages it drops are counted underneath rather than left to a caution.
///
/// Three things make 1.0 the honest place to cut, and the first is the one that matters:
///
/// - **On an ambiguous page the bound is the tolerance class's own floor, not a judgement.**
///   `pdfref::decide` returns the class `Tolerance` unwidened where no consensus formed — widening
///   is a consensus's, derived from its members' spread — so `ours > 1` here means *outside the
///   fixed floor for this page's class* rather than outside something the references decided. That
///   is what makes it a threshold and not a quantity: it is the same constant for every text page
///   in the pool. `Examined::outside_the_bound`'s doc comment declines to *rank* ambiguous pages
///   against their bound, and that objection is about ranking by a page-dependent number; nothing
///   in it is an argument against testing a page-independent one.
/// - **Below 1 the numerator says the opposite of the list's name.** Our nearest inside all three
///   bounds means that reference, had it been in a consensus, would have accepted this page. A
///   page where somebody accepts us is not a page we are alone on, on this instrument's own terms.
/// - **The cut costs the head one page and it is the page 744 named as the defect.**
///   `issue11403_reduced.pdf` led the unfiltered list at 9.06× with ours at 0.51 over 0.06, and
///   its own verdict line reads `differing alone` — a page whose whole disagreement is invisible
///   to the three measures the ratio is computed in. Nothing else in the printed ten moves down,
///   and what moves up into it is `endchar.pdf`, which is in the sublist below.
///
/// The same requirement is written into the four-measure count beside it, in the four-measure
/// unit, and **it changes that count by nothing at all** — which is the asymmetry the three-measure
/// list had, stated as arithmetic. `consensus_missed_by` is above 1 on every ambiguous page by
/// construction, because a pair inside all four bounds would have been a consensus; so `ours >
/// missed` already implies `ours > 1` over there and the filter is a no-op. It is written down
/// anyway, because a reader comparing two counts of one shape has to be able to see that the same
/// question was asked, and because the day a fifth measure joins `Tolerance` the implication is
/// the thing that has to be re-checked.
///
/// # The sublist, which the count underneath now names directly
///
/// With the numerator required outside, the pages whose *denominator* is inside all three bounds
/// are exactly `doc/todo/00` step 1's queue: we fail the floor against every reference and the
/// closest two references pass it against each other. That is the shape worth opening, and the
/// count is printed for it rather than for the population it was drawn from.
///
/// # Where the head ends, which is the marker on the rows
///
/// 751's filter is a threshold against the class **floor**, and the floor is the weakest bound in
/// the gate: it is what `pdfref::decide` returns because no consensus formed, not a judgement
/// anybody made about this page. So a page can be on this list while the references are further
/// outside that floor than we are, and most of the list is. The bound the gate *applies* wherever a
/// consensus does form is [`Judgement::CORPUS`]'s — twice the consensus's own spread, floored — and
/// asking that question of the closest pair separates the list into two:
///
/// - **`[widened: outside]`** — we are outside the bound a consensus at that pair's spread would
///   have set. Whatever the page is, no reading of these references forgives it, and the answer is
///   a clause or a mechanism in the divisor.
/// - **unmarked** — a consensus at that spread would have accepted us. The page is alone against a
///   constant, and the ratio is measuring how closely two references happen to agree.
///
/// [`outside_what_the_closest_pair_would_allow`] has the arithmetic and the reason the printed
/// ratio is only a sufficient condition for it. The seven-hundred-and-sixty-first session stopped
/// there and said so: below the mark, `freeculture.pdf` page 255 is four ink ladders inside 0.066
/// of 255 with our render the most central of the five (ADR 0685), which is what an unmarked row
/// looks like when it is opened.
///
/// # Which measure each half is, and why it is on the row rather than in a note
///
/// Both halves are a maximum over three measures, so the printed ratio is a ratio of like for like
/// only where the two maxima fall on the same one — and on most of this list's head they do not.
/// Each row therefore names its own: `[<measure> v <reference>]` for our nearest and
/// `[<measure>, <reference> v <reference>]` for the closest pair, and a count under the list says
/// how many rows are mixed. [`AloneOn`] has the argument (ADR 0688), and the short form is that a
/// note pricing a mechanism has to price *the measure that ranks the page*: `bug1743245.pdf`'s
/// mechanism was argued in whole-page mean grey over a row whose number is a structural similarity,
/// and removing the mechanism from the document moved that number the *other* way.
fn rank_the_pages_we_are_alone_on(results: &[Examined]) {
    let pool: Vec<&Examined> = results
        .iter()
        .filter(|e| e.complete && matches!(e.verdict, Verdict::Ambiguous(_)))
        .filter(|e| e.consensus_missed_by.is_some())
        .collect();
    let alone_on_four = pool
        .iter()
        .filter(
            |e| match (e.nearest_on_every_measure, e.consensus_missed_by) {
                (Some(ours), Some(missed)) => ours > missed && ours > 1.0,
                _ => false,
            },
        )
        .count();
    let ours_and_theirs = |e: &Examined| -> Option<(f64, f64)> {
        Some((e.distance?.nearest, e.consensus_missed_in_three_measures?))
    };
    let mut alone: Vec<(&Examined, f64, f64)> = pool
        .iter()
        .filter_map(|e| {
            let (ours, missed) = ours_and_theirs(e)?;
            (ours > missed && ours > 1.0).then_some((*e, ours, missed))
        })
        .collect();
    // Counted rather than cautioned, because a caution nobody can count is trap 11 with the sign
    // reversed: the pages this list stopped printing have to stay visible as a number.
    let ours_inside = pool
        .iter()
        .filter(|e| ours_and_theirs(e).is_some_and(|(ours, missed)| ours > missed && ours <= 1.0))
        .count();
    alone.sort_by(|(_, a_ours, a_missed), (_, b_ours, b_missed)| {
        (b_ours / b_missed)
            .partial_cmp(&(a_ours / a_missed))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    println!(
        "\n  ambiguous, outside a bound against every reference, and further from all of them \
         than the closest two are from each other — {} of the {} pages in three measures, \
         {alone_on_four} in four, and the three-measure reading is the one calibrated against a \
         hand-taken one (ADR 0643):",
        alone.len(),
        pool.len(),
    );
    let outside_the_widened_bound =
        |examined: &Examined| examined.outside_what_the_closest_pair_would_allow == Some(true);
    let widened_outside = alone
        .iter()
        .filter(|(examined, _, _)| outside_the_widened_bound(examined))
        .count();
    // Ten by this file's convention, and never fewer rows than the criterion's head: a count that
    // names a head of thirteen under a list of ten is a queue a reader cannot open, which is the
    // shape ADR 0643 found in two columns and ADR 0663 in a dropped population.
    for (examined, ours, missed) in alone.iter().take(widened_outside.max(10)) {
        // Both halves carry the measure they are the ratio of, because a maximum over three
        // measures discards the one name a diagnosis needs: a mechanism accounting for a mean does
        // not thereby account for a structural similarity, and two of the rows below are marked at
        // a ratio under 2 for exactly that reason. `AloneOn` has the argument (ADR 0688).
        let (ours_on, theirs_on) = examined.alone_on.map_or_else(
            || ("-".to_owned(), "-".to_owned()),
            |on| {
                (
                    format!("{} v {}", on.ours.0, on.ours.1.name()),
                    format!(
                        "{}, {} v {}",
                        on.theirs.0,
                        on.theirs.1.name(),
                        on.theirs.2.name()
                    ),
                )
            },
        );
        println!(
            "    {:>6.2}× — {ours:5.2} ours [{ours_on}] over {missed:5.2} between them \
             [{theirs_on}]  {}{}",
            ours / missed,
            examined.name,
            if outside_the_widened_bound(examined) {
                "  [widened: outside]"
            } else {
                ""
            }
        );
    }

    // What a reader of those ten numbers cannot see, and it decides how the list may be read. The
    // denominator has no floor at 1 — the closest pair is above 1 in *four* measures on every
    // ambiguous page by construction, and in three it need not be — so a page can sit here with a
    // pair that agrees with everybody underneath it. Those are the queue rather than the noise now
    // that the numerator is required outside, which is what the first count says.
    let pair_inside = alone.iter().filter(|(_, _, missed)| *missed < 1.0).count();
    println!(
        "    on {pair_inside} of the {} the closest pair sits inside all three bounds while we \
         are outside one — the sublist `doc/todo/00` step 1 asks for; and {ours_inside} further \
         pages of the pool are further from every reference than the closest two are from each \
         other with our own nearest inside every bound, and are not listed, because there the \
         ratio measures how closely the references agree (ADR 0663)",
        alone.len(),
    );
    // A ratio whose two halves are maxima over three measures need not be a ratio of like for
    // like, and where it is not, a note explaining the divisor's mechanism has not thereby
    // explained the numerator's measure. Counted rather than cautioned, for trap 11's reason and
    // ADR 0688's: the rows say which two measures they are, and this says how much of the list
    // reads that way at all.
    let mixed = alone
        .iter()
        .filter(|(examined, _, _)| examined.alone_on.is_some_and(|on| on.ours.0 != on.theirs.0))
        .count();
    println!(
        "    and on {mixed} of the {} the two halves are different measures, so the printed ratio \
         divides one measure by another and the mechanism behind the divisor need not reach the \
         one our own number is on (ADR 0688)",
        alone.len(),
    );
    // The head, marked page by page above. Below it a consensus at the closest pair's own spread
    // would have accepted us, so the page is alone against the class floor rather than against the
    // references — which is a weaker sentence than the list's name, and is where a round reading
    // this list downward stops (ADR 0684).
    println!(
        "    and on {widened_outside} of the {} our nearest is outside the bound Judgement::CORPUS \
         would have widened to from that pair's own spread — marked `[widened: outside]` above, \
         and the head this list is read from downward (ADR 0684)",
        alone.len(),
    );
}

/// The ten ambiguous pages we sit furthest from *every* reference on.
///
/// This is the ranking §3a of the handover asks the ambiguous work to be chosen by, printed
/// by the gate itself so that the next session's item does not have to be guessed at. It is
/// ordered by [`Distance::nearest`] rather than by the number the per-page lines print,
/// because the printed one is our distance from the *worst* reference and that is dominated
/// by renderers which failed: nineteen JBIG2 pages sit at 178 bounds from a `mupdf` that
/// drew a black rectangle, and none of them is ours. A page whose nearest reference is far
/// away is the one worth opening.
///
/// Both numbers are printed. Where they are close, every renderer says the same thing and
/// we are alone; where they are far apart, the references are the ones disagreeing.
fn rank_the_undiagnosed(results: &[Examined]) {
    let diagnosed = diagnosed_ambiguous();
    let mut ranked: Vec<(&Examined, Distance)> = results
        .iter()
        .filter(|e| e.complete && matches!(e.verdict, Verdict::Ambiguous(_)))
        .filter(|e| !diagnosed.contains(&e.name.as_str()))
        .filter_map(|e| e.distance.map(|d| (e, d)))
        .collect();
    ranked.sort_by(|(_, a), (_, b)| {
        b.nearest
            .partial_cmp(&a.nearest)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    println!("\n  ambiguous, undiagnosed, and furthest from the nearest reference:");
    for (examined, distance) in ranked.iter().take(10) {
        println!(
            "    {:>6.2} nearest {:>7.2} furthest  {}",
            distance.nearest, distance.furthest, examined.name
        );
    }
}

/// Holds an outcome to an exact set of pages.
///
/// Both directions fail. A new name is a regression; a missing one means the list is stale
/// and the entry must be deleted, which is what keeps a fixed page from silently coming
/// back.
fn assert_ratchet(what: &str, actual: &[&str], expected: &[&str], guidance: &str) {
    let new: Vec<&str> = actual
        .iter()
        .copied()
        .filter(|name| !expected.contains(name))
        .collect();
    let gone: Vec<&str> = expected
        .iter()
        .copied()
        .filter(|name| !actual.contains(name))
        .collect();

    assert!(
        new.is_empty(),
        "{} page(s) newly {what}: {new:?}\n{guidance}",
        new.len()
    );
    assert!(
        gone.is_empty(),
        "{} page(s) no longer {what}: {gone:?}\n\
         Delete them from the list: a fixed page must not be able to come back.",
        gone.len()
    );
}

/// One reference pair's disagreement about one page: the unit a fixed bound is derived from.
///
/// [`Tolerance::VECTOR`] and [`Tolerance::TEXT_HEAVY`] are four numbers apiece, and each is a
/// claim about how far two *independent* implementations of the same clauses sit from one
/// another on a page of that kind. So the population that can check such a claim is pairs of
/// references; a pair including us would be the gate marking its own work.
#[derive(Debug, Clone, Copy)]
struct Spread {
    /// Whether this page is judged by [`Tolerance::TEXT_HEAVY`], decided exactly as
    /// [`examine`] decides it.
    text: bool,
    /// What this pair can and cannot say about the floor.
    kind: PairKind,
    /// The four measurements, as [`pdfref::triangulate_with`] takes them.
    comparison: raster_compare::Comparison,
}

/// What a pair of reference renderers can say about the noise floor between two renderers.
///
/// # The split, and the `ldd` it used to rest on
///
/// This enumeration had two variants until the eight-hundred-and-forty-fourth session, and the
/// doc comment above them read: "`pdftoppm`, `mutool` and `gs` link the same
/// `libfreetype.so.6` and grid-fit glyphs through it, so on a page whose difference is a
/// letter's edges the three of them are one rasteriser". **The second half of that sentence is
/// false and trap 9's fifth bullet has said so since the six-hundred-and-fifty-sixth session**:
/// `ldd` reports a transitive closure, and `gs` was reaching `FreeType` through `libfontconfig`.
/// What each binary *asks for* is `objdump -p`'s `NEEDED`, and re-checked on this machine's
/// packages: `libpoppler.so` and `libmupdf.so` both name `libfreetype.so.6` — one shared
/// object, loaded by both — while `libgs.so.10` names no `FreeType` at all and defines **194**
/// `FT_*` symbols with none undefined, which is a statically linked copy of its own.
///
/// So the three C references are not one population. Two of them hint through a single object;
/// the third carries its own copy, configured by its own font machinery. That is a weaker
/// independence than a different rasteriser would be — it is the same *algorithm*, and this
/// type does not claim otherwise — but it is exactly the boundary
/// [`Tolerance::widened_to`]'s standing sentence is about, and until this session the
/// derivation averaged across it.
///
/// A pair including `hayro` stays the third thing: two implementations sharing no code *with
/// each other*, one of which hints and one of which does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PairKind {
    /// `poppler` against `mupdf`: both load the machine's one `libfreetype.so.6`.
    OneFreeTypeObject,
    /// `ghostscript` against `poppler` or `mupdf`: two separately linked `FreeType` copies.
    SeparateFreeTypeCopies,
    /// `hayro` against one of those three.
    AcrossTheHintingBoundary,
}

impl PairKind {
    /// Which kind a pair is. Every pair involving `hayro` crosses the boundary.
    fn of(left: Reference, right: Reference) -> Self {
        if left == Reference::Hayro || right == Reference::Hayro {
            Self::AcrossTheHintingBoundary
        } else if left == Reference::Ghostscript || right == Reference::Ghostscript {
            Self::SeparateFreeTypeCopies
        } else {
            Self::OneFreeTypeObject
        }
    }

    /// How the printed table names this population.
    fn label(self) -> &'static str {
        match self {
            Self::OneFreeTypeObject => "poppler v mupdf (one libfreetype.so.6)",
            Self::SeparateFreeTypeCopies => {
                "ghostscript v poppler/mupdf (separate FreeType copies)"
            }
            Self::AcrossTheHintingBoundary => "hayro against one of the three (hinting boundary)",
        }
    }
}

/// One of the four bounds [`Tolerance::accepts`] applies, written as a distance and a limit.
///
/// Everything here is *distance from agreement*, larger being further, so that structural
/// similarity — which runs the other way — can be read on the same axis as the three that
/// measure pixels. `1 - ssim` against `1 - min_structural_similarity` is exactly the
/// comparison `accepts` makes, spelled so that a percentile means what it says.
struct Measure {
    /// How the printed table names it, with its unit.
    name: &'static str,
    /// How far this pair sits from agreement on this measure.
    distance: fn(&raster_compare::Comparison) -> f64,
    /// How far the class's fixed bound allows it to be.
    limit: fn(&Tolerance) -> f64,
    /// Multiplier applied for printing, so a fraction reads as a percentage.
    scale: f64,
}

/// The four, in the order [`Tolerance::accepts`] checks them.
const MEASURES: [Measure; 4] = [
    Measure {
        name: "mean (of 255)",
        distance: |c| c.mean_error,
        limit: |t| t.max_mean,
        scale: 1.0,
    },
    Measure {
        name: "worst tile (of 255)",
        distance: |c| c.worst_tile_error,
        limit: |t| t.max_worst_tile,
        scale: 1.0,
    },
    Measure {
        name: "differing (%)",
        distance: |c| c.differing_fraction,
        limit: |t| t.max_differing_fraction,
        scale: 100.0,
    },
    Measure {
        name: "1 - ssim",
        distance: |c| 1.0 - c.structural_similarity,
        limit: |t| 1.0 - t.min_structural_similarity,
        scale: 1.0,
    },
];

/// The environment variable that asks for the census below. Same mechanism, and the same
/// reason, as [`SPREAD_IS_ASKED_FOR`].
const CENSUS_IS_ASKED_FOR: &str = "PDFVIEWER_ORACLE_CENSUS";

/// What the references say about every corpus under `doc/corpora/`, gated and ungated alike.
///
/// # What this is for, and why it is not the gate
///
/// [`SUBMODULE_CORPORA`] carries a decision per population — whether the references' vote is
/// evidence about it — and two of the four say no. A decision recorded only as a `false` is a
/// decision nobody can check, so this prints what the vote *would* have said over all four,
/// per corpus, and thereby makes the two exclusions readable rather than merely asserted.
///
/// It is also the answer to `CLAUDE.md`'s rule that a fact which can be counted is not
/// written down: the numbers for the ungated populations live here, in a command, and not in
/// `doc/oracle-and-corpus.md`.
///
/// It asserts only that it had a population to print, for the reason ADR 0393 gives about the
/// verdict vocabulary and `CLAUDE.md` gives about malformed files: on these two populations a
/// contradicted page is a question rather than a defect, and a ratchet over questions
/// converts them into targets to move toward, which principle 5 forbids outright.
///
/// # Why `#[ignore]` is not enough
///
/// The same reason [`the_fixed_bounds_against_the_references_own_spread`] gives at length:
/// `-- --ignored` is a switch on the binary rather than a filter, so the gate's own command
/// would otherwise run this beside it and pay for 219 more pages on every round. The guard is
/// in the test because an invocation can be copied without its guard and a test cannot be run
/// without itself.
#[test]
#[ignore = "renders every submodule corpus page four times; run explicitly, in release"]
fn what_the_references_say_about_every_submodule_corpus() {
    if std::env::var_os(CENSUS_IS_ASKED_FOR).is_none() {
        println!(
            "skipped: this is a census rather than a gate — set {CENSUS_IS_ASKED_FOR}=1 to \
             run it. See the doc comment for why `--ignored` alone does not ask for it."
        );
        return;
    }
    require_the_sandbox();

    let available: Vec<Reference> = Reference::voting()
        .into_iter()
        .filter(|reference| reference.is_available())
        .collect();
    assert!(
        available.len() >= 2,
        "at least two reference renderers are needed to triangulate; found {}",
        available.len()
    );
    for reference in &available {
        println!(
            "{}: {}",
            reference.name(),
            reference.version().unwrap_or_default()
        );
    }

    let work_root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("oracle");
    let cache = reference_cache();
    let mut populations = 0usize;

    for corpus in SUBMODULE_CORPORA {
        let items = corpus_items(corpus);
        if items.is_empty() {
            println!(
                "\n{}: not checked out at {} — nothing to census",
                corpus.label, corpus.directory
            );
            continue;
        }
        populations = populations.saturating_add(1);
        let documents = {
            let mut paths: Vec<&Path> = items.iter().map(|work| work.path.as_path()).collect();
            paths.dedup();
            paths.len()
        };
        let started = Instant::now();
        let mut results: Vec<Examined> = items
            .par_iter()
            .map(|work| examine(work, &work_root, &available, &cache))
            .collect();
        results.sort_by(|a, b| a.name.cmp(&b.name));
        let elapsed = started.elapsed();

        let count = |label: &str| {
            results
                .iter()
                .filter(|e| e.verdict.label() == label)
                .count()
        };
        println!(
            "\n{}: {documents} documents, {} pages in {:.1}s — {} agrees, {} contradicted, \
             {} ambiguous, {} not comparable, {} no render, {} geometry — {}",
            corpus.label,
            results.len(),
            elapsed.as_secs_f64(),
            count("agrees"),
            count("CONTRADICTED"),
            count("ambiguous"),
            count("not comparable"),
            count("no render"),
            count("GEOMETRY") + count("reference geometry"),
            if corpus.voted {
                "voted, and gated"
            } else {
                "not voted — census only"
            },
        );
        // Every page that is not an agreement, named. A census whose output is six numbers
        // tells a later round that something is there and not which file it is, which is the
        // half a round actually needs to take a defect off it.
        for examined in results
            .iter()
            .filter(|e| !matches!(e.verdict, Verdict::Agrees))
        {
            println!(
                "  {:<20} {}{}: {}",
                examined.verdict.label(),
                examined.name,
                if examined.complete {
                    ""
                } else {
                    " [we report this page]"
                },
                examined.verdict.detail(),
            );
        }
    }

    assert!(
        populations > 0,
        "no corpus under doc/corpora/ is checked out, so this census measured nothing. \
         `git submodule update --init doc/corpora/…`, or see doc/oracle-and-corpus.md §2 for \
         the sparse-checkout recipes the two partial ones need."
    );
    println!("\nartefacts under {}", work_root.display());
}

/// The environment variable that asks for the derivation below. See its doc comment for why
/// an attribute could not carry this.
const SPREAD_IS_ASKED_FOR: &str = "PDFVIEWER_ORACLE_SPREAD";

/// Where the fixed bounds come from, re-derived from the corpus rather than remembered.
///
/// # Why this is a separate run and not part of the gate
///
/// Because it renders `hayro` on every page, which the gate deliberately does not: the gate
/// asks `hayro` only about pages worth looking at, and its verdicts never depend on it. This
/// asks a different question — *what is the floor* — and for that the fourth renderer is the
/// whole point, since it is the only one in the room that does not grid-fit glyphs through
/// `libfreetype` and is not us.
///
/// # The method, which is the one that produced the bounds being checked
///
/// For each measure, the distribution is taken over the pairs that agree by the **other
/// three**. That conditioning is `Tolerance::VECTOR`'s own derivation — "the pages where
/// every reference pair already falls inside the three pixel bounds above have a structural
/// similarity of 0.9971 at worst" — and it is what stops the measurement from being circular:
/// a bound measured over the pairs it already admits returns the bound.
///
/// It prints, and asserts only that it had a population to print. A number in this table is
/// evidence for changing a bound; it is not itself a gate, because a bound that moved
/// whenever a reference renderer was upgraded would be the curve-fitting `CLAUDE.md` forbids
/// wearing a schedule.
///
/// # Why `#[ignore]` is not enough, and there is an environment variable as well
///
/// `#[ignore]` says *run explicitly*, and `cargo test -- --ignored` — which is how every
/// corpus gate in this tree is invoked — overrides it for every test in the binary at once.
/// So the attribute cannot express "not this one", and the gate's own command silently
/// acquired a passenger: measured in the four-hundred-and-forty-seventh session, this ran
/// beside the gate on the same 24 cores for 40 s, taking the oracle line from 48 s to 96 and
/// inflating the per-page spans the gate prints, because both walk the corpus under `rayon`.
/// The guard is here rather than in the invocation for the reason a guard usually is: an
/// invocation can be copied without it, and a test cannot be run without itself. ADR 0282.
#[test]
#[ignore = "renders every corpus page with all four renderers; run explicitly, in release"]
fn the_fixed_bounds_against_the_references_own_spread() {
    if std::env::var_os(SPREAD_IS_ASKED_FOR).is_none() {
        println!(
            "skipped: this is a derivation rather than a gate — set {SPREAD_IS_ASKED_FOR}=1 to \
             run it. See the doc comment for why `--ignored` alone does not ask for it."
        );
        return;
    }
    require_the_sandbox();
    let Some(items) = work_items() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let voting: Vec<Reference> = Reference::voting()
        .into_iter()
        .filter(|reference| reference.is_available())
        .collect();
    assert!(
        voting.len() >= 2,
        "at least two reference renderers are needed; found {}",
        voting.len()
    );
    let fourth = Reference::Hayro.is_available().then_some(Reference::Hayro);
    if fourth.is_none() {
        println!(
            "note: {} is not built, so the hinting boundary cannot be measured — {}",
            Reference::Hayro.name(),
            Reference::Hayro.package_hint()
        );
    }
    for reference in voting.iter().chain(fourth.iter()) {
        println!(
            "{}: {}",
            reference.name(),
            reference.version().unwrap_or_default()
        );
    }

    let work_root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("spread");
    let cache = reference_cache();
    let selection = Selection::from_environment();
    let items: Vec<Work> = items
        .into_iter()
        .filter(|work| selection.admits(work))
        .collect();
    assert!(!items.is_empty(), "no pages selected");

    let started = Instant::now();
    let measured: Vec<(Vec<Spread>, Vec<Substitution>)> = items
        .par_iter()
        .map(|work| spreads_of(work, &work_root, &voting, fourth, &cache))
        .collect();
    let spreads: Vec<Spread> = measured
        .iter()
        .flat_map(|(s, _)| s.iter().copied())
        .collect();
    let substitutions: Vec<Substitution> = measured
        .iter()
        .flat_map(|(_, s)| s.iter().cloned())
        .collect();
    println!(
        "\n{} reference pairs and {} substitution verdicts over {} pages in {:.1}s",
        spreads.len(),
        substitutions.len(),
        items.len(),
        started.elapsed().as_secs_f64()
    );
    assert!(
        !spreads.is_empty(),
        "no reference pair could be compared, so the table below would read as agreement"
    );

    for (class, bounds, text) in [
        (
            "text pages, Tolerance::TEXT_HEAVY",
            Tolerance::TEXT_HEAVY,
            true,
        ),
        ("vector pages, Tolerance::VECTOR", Tolerance::VECTOR, false),
    ] {
        for kind in [
            PairKind::OneFreeTypeObject,
            PairKind::SeparateFreeTypeCopies,
            PairKind::AcrossTheHintingBoundary,
        ] {
            let population: Vec<Spread> = spreads
                .iter()
                .copied()
                .filter(|spread| spread.text == text && spread.kind == kind)
                .collect();
            print_the_distribution(class, kind, &population, &bounds);
        }
        let population: Vec<Substitution> = substitutions
            .iter()
            .filter(|row| row.text == text)
            .cloned()
            .collect();
        print_the_substitutions(class, &population);
    }
}

/// Every reference pair's disagreement about one page, or nothing where the page is not
/// comparable.
///
/// The rasters are the gate's own: rendered at the same resolution, reconciled by
/// [`reconcile`] in the same order, with `hayro` added afterwards only when it already agrees
/// about the page's size — which is what [`examine`] does, and for the same reason. A fourth
/// renderer allowed into the reconciliation could tip a two-against-two about the page's
/// extent and change which rasters the other three are compared at.
fn spreads_of(
    work: &Work,
    work_root: &Path,
    voting: &[Reference],
    fourth: Option<Reference>,
    cache: &Cache,
) -> (Vec<Spread>, Vec<Substitution>) {
    let stem = work.path.file_stem().unwrap_or_default().to_string_lossy();
    let work_dir = work_root
        .join(stem.as_ref())
        .join(format!("p{}", work.page));
    let cleanup = || {
        let _ = std::fs::remove_dir_all(&work_dir);
    };
    let nothing = (Vec::new(), Vec::new());

    let Ok(ours) = render_ours(work) else {
        cleanup();
        return nothing;
    };
    // `absent` is the gate's to report; this is the spread derivation, whose population is pairs
    // of references and which says nothing about any page.
    let Ok(Rendered {
        rendered: mut references,
        ..
    }) = render_references(work, &work_dir, voting, cache)
    else {
        cleanup();
        return nothing;
    };
    let mut raster = ours.raster;
    if reconcile(&mut raster, &mut references).is_err() {
        cleanup();
        return nothing;
    }
    // Collected before the fourth renderer joins and before the directory goes, because the
    // substitution below runs `pdfref::triangulate_with` exactly as `examine` does and that
    // includes what each program said while it drew (ADR 0769).
    let testimony = what_they_said(&references, &work_dir);
    let page: Arc<str> = Arc::from(work.name());
    let substitutions = substitutions_of(&page, &raster, &references, &testimony, ours.has_text);
    if let Some(fourth) = fourth
        && let Ok(extra) = cache.render(fourth, &work.path, work.page, DPI, &work_dir)
        && extra.width == raster.width
        && extra.height == raster.height
    {
        references.push((fourth, extra));
    }
    cleanup();

    let mut spreads = Vec::new();
    for (index, (left_name, left)) in references.iter().enumerate() {
        for (right_name, right) in references.iter().skip(index.saturating_add(1)) {
            if let Ok(comparison) = raster_compare::compare(left, right) {
                spreads.push(Spread {
                    text: ours.has_text,
                    kind: PairKind::of(*left_name, *right_name),
                    comparison,
                });
            }
        }
    }
    (spreads, substitutions)
}

/// One page's answer to *what would this gate say about a program known to be independent?*
///
/// One row per (pair, candidate): the pair of voting references whose agreement is the
/// consensus, and the program judged by it — either the third voting reference or our own
/// render. Both candidates are judged by the same consensus, at the same widened bound, in the
/// same run, which is what makes the two counts comparable.
#[derive(Debug, Clone)]
struct Substitution {
    /// The page this row is about, as [`Work::name`] writes it.
    ///
    /// A row without it is a count, and ADR 0772 is what a count costs: the population of
    /// trap 12's control lived in an ADR's prose for one round, two of its three names were
    /// wrong, and the round that had to read the pages rebuilt the list before it could start.
    /// This table's vector row was handed on the same way — *119 of 226* with no page in it.
    page: Arc<str>,
    /// Whether this page is judged by [`Tolerance::TEXT_HEAVY`], decided as [`examine`] does.
    text: bool,
    /// The two references whose agreement judged this row.
    pair: (Reference, Reference),
    /// Who was put where our render stands — `None` for our render itself.
    stood_in: Option<Reference>,
    /// Whether the pair agreed at all, and whether it contradicted the candidate.
    contradicted: bool,
    /// Which of [`MEASURES`] the candidate was outside, at the bound the pair's own spread set.
    outside: [bool; MEASURES.len()],
}

/// Every voting reference put where our render stands, judged by the other two — and us judged
/// by the same two, in the same call, so the two answers are like for like.
///
/// # Why this is the control the bound needs, and not another spread
///
/// [`print_the_distribution`] measures how far reference pairs sit from one another. That is
/// the population [`Tolerance`]'s four numbers are claims about, and it is not the question a
/// verdict asks. `pdfref::decide` does not take *a* pair: it takes the pairs that agree within
/// the fixed bounds, which on a page where one exists is the **closest** pair in the room. So
/// the bound a verdict rests on is derived from a selected minimum, and what a third
/// implementation owes is not "be as close as two implementations typically are" but "be as
/// close to the closest pair as the excluded one of three manages". Nothing had measured the
/// second, and it is measurable without us: run the gate's own judgement with a *reference*
/// standing where our render stands.
///
/// That is trap 12's `colors.pdf` control and ADR 0617's — *put each reference where our render
/// stands and ask what the sets it is not a member of conclude about it* — taken over the corpus
/// as a distribution instead of over one page as a check. ADR 0771.
///
/// The bound is `Judgement::CORPUS`'s and the tolerance is the page's own class, both exactly as
/// [`examine`] passes them, so a row here is the gate's verdict about that program on that page
/// and not a re-derivation of one.
fn substitutions_of(
    page: &Arc<str>,
    ours: &Raster,
    references: &[(Reference, Raster)],
    testimony: &[pdfref::Testimony],
    text: bool,
) -> Vec<Substitution> {
    let tolerance = bounds_for(text);
    let mut rows = Vec::new();
    for (index, (left_name, left)) in references.iter().enumerate() {
        for (right_name, right) in references.iter().skip(index.saturating_add(1)) {
            let pair = [(*left_name, left.clone()), (*right_name, right.clone())];
            let said: Vec<pdfref::Testimony> = testimony
                .iter()
                .filter(|said| said.reference() == *left_name || said.reference() == *right_name)
                .cloned()
                .collect();
            let third = references
                .iter()
                .find(|(name, _)| name != left_name && name != right_name);
            let candidates = std::iter::once((None, ours))
                .chain(third.map(|(name, raster)| (Some(*name), raster)));
            for (stood_in, raster) in candidates {
                let Ok(triangulation) =
                    pdfref::triangulate_with(raster, &pair, &said, &tolerance, Judgement::CORPUS)
                else {
                    continue;
                };
                // A pair that did not agree judged nobody, and a row for it would count a page
                // on which this instrument has no reading.
                if !matches!(
                    triangulation.outcome,
                    Outcome::Agrees { .. } | Outcome::Regression { .. }
                ) {
                    continue;
                }
                let contradicted = matches!(triangulation.outcome, Outcome::Regression { .. });
                let bound = triangulation.judged_by;
                let mut outside = [false; MEASURES.len()];
                for (index, measure) in MEASURES.iter().enumerate() {
                    outside[index] = triangulation.ours.iter().any(|(_, comparison)| {
                        (measure.distance)(comparison) > (measure.limit)(&bound)
                    });
                }
                rows.push(Substitution {
                    page: Arc::clone(page),
                    text,
                    pair: (*left_name, *right_name),
                    stood_in,
                    contradicted,
                    outside,
                });
            }
        }
    }
    rows
}

/// One block of the substitution table: what one pair's consensus says about each candidate.
///
/// Every row's contradicted pages are **named** under the table, not only counted. ADR 0772 is
/// why: the count is a population somebody will have to read, a count cannot be handed to the
/// next round as a to-do list, and the two times this project wrote such a list into a document
/// instead it wrote it wrong. The 119 vector pages this table's `mupdf` + `ghostscript` row
/// convicts `poppler` on were handed to the eight-hundred-and-forty-sixth session as a number
/// alone, and naming them was that round's first act (ADR 0773).
fn print_the_substitutions(class: &str, rows: &[Substitution]) {
    let mut pairs: Vec<(Reference, Reference)> = rows.iter().map(|row| row.pair).collect();
    pairs.sort_by_key(|(left, right)| (left.name(), right.name()));
    pairs.dedup();
    println!("\n  {class} — the gate's own verdict, with a reference standing where we stand");
    println!(
        "    {:<36} {:>7} {:>16} {:>7} {:>7} {:>7} {:>7}",
        "consensus, and who it judged", "pages", "contradicted", "mean", "tile", "diff", "ssim"
    );
    let mut named: Vec<(String, Vec<&str>)> = Vec::new();
    for pair in pairs {
        let of_pair: Vec<&Substitution> = rows.iter().filter(|row| row.pair == pair).collect();
        let mut candidates: Vec<Option<Reference>> =
            of_pair.iter().map(|row| row.stood_in).collect();
        candidates.sort_by_key(|stood_in| stood_in.map_or("ours", Reference::name));
        candidates.dedup();
        for stood_in in candidates {
            let rows: Vec<&&Substitution> = of_pair
                .iter()
                .filter(|row| row.stood_in == stood_in)
                .collect();
            let pages = rows.len();
            let contradicted: Vec<&str> = rows
                .iter()
                .filter(|row| row.contradicted)
                .map(|row| row.page.as_ref())
                .collect();
            let per_measure: Vec<usize> = (0..MEASURES.len())
                .map(|index| {
                    rows.iter()
                        .filter(|row| row.outside.get(index).copied().unwrap_or(false))
                        .count()
                })
                .collect();
            let heading = format!(
                "{} + {} judging {}",
                pair.0.name(),
                pair.1.name(),
                stood_in.map_or("ours", Reference::name)
            );
            println!(
                "    {heading:<36} {pages:>7} {:>8} ({:>5.1}%) {:>7} {:>7} {:>7} {:>7}",
                contradicted.len(),
                share(contradicted.len(), pages) * 100.0,
                per_measure.first().copied().unwrap_or(0),
                per_measure.get(1).copied().unwrap_or(0),
                per_measure.get(2).copied().unwrap_or(0),
                per_measure.get(3).copied().unwrap_or(0),
            );
            named.push((heading, contradicted));
        }
    }
    for (heading, mut pages) in named {
        if pages.is_empty() {
            continue;
        }
        pages.sort_unstable();
        println!("\n    {heading} — contradicted ({}):", pages.len());
        for page in pages {
            println!("      {page}");
        }
    }
}

/// One block of the table: every measure's distribution over one class and one pair kind.
///
/// The last column is the one a bound is read from — how many of these pairs the fixed bound
/// would call a disagreement. A bound sitting under its own references' spread is not a bound
/// at all, and a bound far above it forgives whatever lies between.
fn print_the_distribution(class: &str, kind: PairKind, population: &[Spread], bounds: &Tolerance) {
    println!("\n  {class} — {}: {} pairs", kind.label(), population.len());
    println!(
        "    {:<20} {:>7} {:>9} {:>9} {:>9} {:>9} {:>9} {:>7}",
        "measure", "n", "median", "p90", "p99", "max", "bound", "over"
    );
    for (index, measure) in MEASURES.iter().enumerate() {
        let limit = (measure.limit)(bounds);
        let mut values: Vec<f64> = population
            .iter()
            .filter(|spread| admitted_by_the_others(spread, index, bounds))
            .map(|spread| (measure.distance)(&spread.comparison))
            .collect();
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let over = values.iter().filter(|value| **value > limit).count();
        println!(
            "    {:<20} {:>7} {:>9.4} {:>9.4} {:>9.4} {:>9.4} {:>9.4} {:>6.1}%",
            measure.name,
            values.len(),
            quantile(&values, 1, 2) * measure.scale,
            quantile(&values, 9, 10) * measure.scale,
            quantile(&values, 99, 100) * measure.scale,
            quantile(&values, 1, 1) * measure.scale,
            limit * measure.scale,
            share(over, values.len()) * 100.0
        );
    }
}

/// `part` as a fraction of `whole`, and zero where there is nothing to divide.
#[expect(
    clippy::cast_precision_loss,
    reason = "both counts are bounded by the corpus's reference pairs, some tens of \
              thousands, which f64 holds exactly"
)]
fn share(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64
    }
}

/// Whether every bound *except* the one indexed admits this pair.
///
/// See [`the_fixed_bounds_against_the_references_own_spread`]: measuring a bound over the
/// pairs that bound already admits returns the bound.
fn admitted_by_the_others(spread: &Spread, exclude: usize, bounds: &Tolerance) -> bool {
    MEASURES
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != exclude)
        .all(|(_, measure)| (measure.distance)(&spread.comparison) <= (measure.limit)(bounds))
}

/// The value `numerator/denominator` of the way through a sorted sample.
///
/// Nearest-rank rather than interpolated: every value here was produced by a real pair of
/// renderers on a real page, and a quantile that is the average of two of them is a number
/// nobody measured.
fn quantile(sorted: &[f64], numerator: usize, denominator: usize) -> f64 {
    let last = sorted.len().saturating_sub(1);
    let index = last
        .saturating_mul(numerator)
        .checked_div(denominator)
        .unwrap_or(0);
    sorted.get(index).copied().unwrap_or(0.0)
}
