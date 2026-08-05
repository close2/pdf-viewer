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
/// **`french_diacritics.pdf` left this group in the sixteenth session and not by being rounded
/// differently.** It agrees because area averaging replaced the four-tap filter that was
/// drawing its reduced inline images (ADR 0025) — worst tile 12.60 against a bound of 5.89
/// before, inside the bound after. Its raster really is 595x842 against `poppler`'s and
/// `mupdf`'s 596, which is what put it here; that was true and was not what the references
/// were disagreeing about.
const CONTRADICTED_PAGE_ROUNDING: [&str; 2] = ["colorkeymask.pdf page 1", "issue21346.pdf page 1"];

/// Contradicted, where the difference is this tree's own anti-aliasing at a shape's edge.
///
/// 2 pages, and they are the first entries named for **§10.7.4's first departure** rather than
/// for anything on the page. `colors.pdf` pages 1 and 2 are grids of flat colour swatches, and
/// every pixel of every swatch's interior is identical in all five renderers. What differs is
/// the boundary between two swatches, where the rectangle edge falls inside a pixel:
///
/// | | pixels differing from `poppler` | mean absolute difference |
/// |---|---|---|
/// | `ghostscript` | 3461 | 0.17 |
/// | `mupdf` | 4054 | 0.29 |
/// | `hayro` | 4647 | 0.28 |
/// | ours | 4054 | 0.33 |
///
/// So the five sit on a *spectrum of edge softness* with `poppler` at one end, and the pair the
/// gate votes with is the pair nearest that end. At one sampled edge pixel `poppler` paints the
/// swatch colour outright — `(255, 153, 0)` — `ghostscript` gives `(253, 166, 41)`, `mupdf`
/// `(253, 175, 63)` and we give `(254, 184, 87)`.
///
/// **`poppler` is the one closest to the clause.** §10.7.4: "[a] shape shall be scan-converted
/// by painting any pixel whose half-open square region intersects the shape, no matter how small
/// the intersection is." That is a hard edge, and this tree's anti-aliasing is a *documented
/// departure* from it — the first of the four §10.7.4's ledger row records, licensed by
/// §10.7.1's NOTE that the algorithm is not part of PDF. So this entry is not a defect to fix
/// and is not a page to chase: it is the departure being visible, on the one kind of page where
/// nothing else is.
///
/// The bound is what makes it fail at all, and it is trap 12's shape: mean 0.25 against 1.00 and
/// worst tile 2.79 against 5.00 both pass with room, and structural similarity fails at 0.9857
/// against **0.9886** — a bound the two least-anti-aliased renderers set for each other on a
/// page that is nothing but edges.
const CONTRADICTED_ANTIALIASED_EDGES: [&str; 2] = ["colors.pdf page 1", "colors.pdf page 2"];

/// Contradicted, where the difference is a `CalRGB` space converted rather than assumed.
///
/// 1 page. `issue9940.pdf` draws its cover art through
/// `[/Indexed [/DeviceN [/IBM /None /None /None] [/CalRGB …] tint] 255 table]`, and the
/// alternate space is the whole of the disagreement: we and `poppler` convert a `CalRGB`
/// through CIE XYZ as §8.6.5.3 defines it, while `mupdf` and `ghostscript` take its
/// components for `DeviceRGB` — so their page is pinker than ours by a few levels across
/// every pixel the image covers, which is 6.8% of the page.
///
/// The same difference kept four pages of `calrgb.pdf` in `CONTRADICTED_SUBSTITUTED_FONT`,
/// where it is described as "a residue of colour management rather than of fonts". It is
/// listed separately here because this page has no substituted font to be confused with, and
/// because the argument is ADR 0012's rather than anyone's arithmetic: §8.6.5.3 gives a
/// `CalRGB` a white point, a gamma and a matrix into XYZ, and a renderer that ignores all
/// three is not reading the clause.
///
/// The `None` colourants are not the cause, though they look like one. §8.6.6.5 is explicit:
/// "when the DeviceN colour space reverts to its alternate colour space, those components
/// shall be passed to the tint transformation function", which is what happens here — the
/// space never reaches a device colourant, so it always reverts.
const CONTRADICTED_CALIBRATED_COLOUR: [&str; 1] = ["issue9940.pdf page 1"];

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
const CONTRADICTED_REFERENCE_GLYPH_WIDTHS: [&str; 1] = ["issue9915_reduced.pdf page 1"];

/// Contradicted, where the document asks for a line width the clause forbids.
///
/// 1 page, and one operator in it: `issue19633.pdf` strokes a single diagonal under `-0.1 w`.
/// §8.4.3.2 says the line width "shall be a non-negative number expressed in user space
/// units", so the value is outside the parameter's stated domain, and the clause states no
/// recovery for one that is.
///
/// **Three readings are available and each renderer takes a different one.** We clamp the
/// value into the domain, which makes it zero, and §8.4.3.2's rule for zero is "the thinnest
/// line that can be rendered at device resolution: 1 device pixel wide" — a dark, solid line.
/// `poppler` and `mupdf` draw a very faint one, consistent with the magnitude, 0.1 of a pixel's
/// coverage. `ghostscript` draws something between the two. A *fourth* reading is the clause's
/// own definition of stroking — "painting all points whose perpendicular distance from the path
/// in user space is less than or equal to half the line width" — under which a negative width
/// paints nothing at all, and nobody takes it.
///
/// Listed rather than chased, and the choice is written down where it is made
/// (`content.rs`'s `w` handler and `Stroke::device_width`) rather than left in a `.max(0.0)`
/// that answers a question nobody asked. **One operator in one of 974 documents**, measured —
/// so the corpus cannot rank this and the clause does not decide it.
const CONTRADICTED_NEGATIVE_LINE_WIDTH: [&str; 1] = ["issue19633.pdf page 1"];

/// Contradicted, where the difference is how `DeviceCMYK` becomes a pixel.
///
/// 4 pages in 3 documents, and the group with the most evidence behind it of any here —
/// none of which is anybody's rendering.
///
/// # What the pages are
///
/// All three reach `DeviceCMYK`: `type4psfunc.pdf` and `postscript_type4_many_outputs.pdf`
/// through a `/DeviceN` whose alternate it is, `function_based_shading_cmyk.pdf` directly.
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
/// third way: **they share data.** `/usr/share/ghostscript/iccprofiles/default_cmyk.icc`,
/// evaluated by `pdf_model::icc` — our own A2B evaluator, written for `ICCBased` streams and
/// pointed at a file on this machine — produces 255/219/186/150/112/59/0/0/0 in red for the
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
const CONTRADICTED_DEVICE_CMYK_CONVERSION: [&str; 5] = [
    "function_based_shading_cmyk.pdf page 1",
    "function_based_shading_cmyk.pdf page 2",
    "postscript_type4_many_outputs.pdf page 1",
    "transparent.pdf page 1",
    "type4psfunc.pdf page 1",
];

/// Contradicted, where an image is thinner than a device pixel.
///
/// 1 page. `issue4436r.pdf` is the whole test: a 1x1 image mask under
/// `180 0 0 -0.48 10 25 cm`, so it covers 180 pixels by *0.48* of one, and the page says in
/// words that "a thin line should be visible above this text". We draw it antialiased, at
/// 48% coverage of one row; `poppler` and `mupdf` draw a solid black row.
///
/// Nothing in ISO 32000-2 decides this. §8.4.3.2 gives a *stroke* the rule — a zero width
/// "shall denote the thinnest line that can be rendered at device resolution" — and says
/// nothing of the kind about an image, whose geometry is the unit square its matrix maps.
/// Coverage is what the image asks for and is what we draw; snapping it to a full row is a
/// device-specific minimum, which is a defensible choice and not one the standard states.
/// The page is listed rather than chased for that reason: closing it would mean copying a
/// convention rather than reading a clause.
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
/// What `jbig2dec` does on these seven: on four of them it decodes nothing and renders a
/// blank page, on two it produces the drawing strewn with noise blocks, and on
/// `bitmap-symbol-context-reuse.pdf` it prints `segment marks bitmap coding context as
/// retained (NYI)` and gives up. `poppler`, which has its own decoder, agrees with us on six
/// of the seven; on the seventh it fails differently, reporting "Too many symbols in JBIG2
/// symbol dictionary".
///
/// The evidence that *we* are right is not poppler's agreement, which would only be evidence
/// that we read ISO/IEC 14492 the same way. It is `tests/jbig2.rs`: the corpus encodes one
/// image ninety-six ways, through every coding mode the standard defines, and all ninety-six
/// decode to byte-identical pixels here. A decoder wrong about refinement, or about Huffman
/// symbol dictionaries, or about retained coding contexts could not produce that.
///
/// These stay listed rather than being excused, because the gate should keep watching them:
/// if `jbig2dec` is fixed they will leave this list, and if our decode changes they will
/// change too.
const CONTRADICTED_SHARED_JBIG2_DECODER: [&str; 7] = [
    "bitmap-halftone-composite.pdf page 1",
    "bitmap-refine-page-subrect.pdf page 1",
    "bitmap-symbol-context-reuse.pdf page 1",
    "bitmap-symbol-symhuffrefineone.pdf page 1",
    "bitmap-symbol-texthuffrefinecustom.pdf page 1",
    "bitmap-symbol-texthuffrefinecustomposdims.pdf page 1",
    "issue20439.pdf page 1",
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
/// of *each other*, the bound derived from them is a mean of 1.11 — which our mean of 2.02
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
const CONTRADICTED_MASK_QUANTISATION: [&str; 1] = ["smask_luminosity_oob_transfer.pdf page 1"];

/// Contradicted, and **we are the ones who are right**: a visibility expression.
///
/// 1 page. This entry used to hold three, and the other two left when optional content
/// landed (§8.11) — `issue12007_reduced.pdf` was drawing a whole hidden screenshot over a
/// page the references leave nearly blank. What is left is a page where the reference
/// consensus is wrong, which is rare enough to be worth the paragraph.
///
/// `visibility_expressions.pdf` draws five lines twice: once pale, and once dark inside five
/// `BDC /OC` sections whose membership dictionaries each carry a `/VE` visibility expression
/// and *no* `/OCGs` or `/P`. With group C off and A and B on, `[/Not 9 0 R]` and
/// `[/Not [/Or 9 0 R 10 0 R]]` are false, so two of the five dark lines are hidden. We draw
/// them hidden; `poppler` draws them hidden; `mupdf` and `ghostscript` draw all five dark.
///
/// # Why the two that agree are not evidence here
///
/// **Neither of them implements `/VE`**, and this was checked in their source rather than
/// inferred from the picture:
///
/// - `mupdf`, `source/pdf/pdf-layer.c`, in the `OCMD` branch of `pdf_is_ocg_hidden_imp`:
///   `if (pdf_is_array(ctx, obj)) { /* FIXME: Calculate visibility from array */ return 0; }`
///   — a `/VE` array means visible, always.
/// - `ghostscript`, `pdf/pdf_optcontent.c`, in `pdfi_oc_check_OCMD`: `WARNING: OCMD contains
///   VE, which is not supported (ignoring)`.
/// - `poppler` does implement it: the installed library exports
///   `OCGs::evalOCVisibilityExpr(Object const*, int) const` and carries the string
///   `Loop detected in optional content visibility expression`.
/// - `pdf.js` implements it too, and prefers it, in `src/core/evaluator_utils.js` — which is
///   in this repository, under `doc/pdf.js`. Its issue #12097 is closed by PR #13243, and
///   this very file is the test that came with it.
///
/// So the count is three implementations that read the clause the way we do against two that
/// have not implemented it. **Trap 9 in `doc/HANDOVER.md` generalises**: two renderers can
/// agree because they share a decoder, and they can also agree because they share a *gap* —
/// an unimplemented feature almost always falls through to "draw it", so the same silence
/// produces the same picture in both. Agreement is evidence only where the implementations
/// can fail independently, and a missing feature is not an independent failure.
///
/// §8.11.2.2 is not ambiguous — "If the VE key is present it shall be used in preference to
/// the OCGs and P keys" — so the clause settles it, and the other renderers are evidence
/// about that reading rather than a target to move toward.
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
const CONTRADICTED_REFERENCES_DREW_NOTHING: [&str; 2] = [
    "issue11549_reduced.pdf page 1",
    "issue11740_reduced.pdf page 1",
];

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
/// spare, and 9.89% of pixels differing — which on a page that is entirely glyph edges is the
/// anti-aliasing of every letter, against two references that share `FreeType`.
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
/// 19 pages. The weakest entries here, because the difference need not be anyone's defect:
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
/// The four that remain differ by about ten levels in one channel against `mupdf` and
/// `ghostscript`, while agreeing with `poppler` exactly. That is a residue of colour
/// management rather than of fonts, and small enough that closing it would mean choosing
/// whose arithmetic to copy — which principle 5 forbids.
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
/// it still holds what that condition refuses. Page 1 of the same document became a page we
/// draw completely and its mean fell 4.19 to 3.20.
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
/// the same on every machine — which is what §9.6.2.2 means by "[t]hese fonts … shall be available to
/// the PDF processor", and what `CLAUDE.md`'s principle 5 means by not treating agreement with
/// other renderers as the definition of right.
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
const CONTRADICTED_SUBSTITUTED_FONT: [&str; 19] = [
    "bad-PageLabels.pdf page 1",
    "bug847420.pdf page 1",
    "bug850854.pdf page 1",
    "calrgb.pdf page 1",
    "calrgb.pdf page 11",
    "calrgb.pdf page 12",
    "calrgb.pdf page 5",
    "franz_2.pdf page 1",
    "issue11403_reduced.pdf page 1",
    "issue15716.pdf page 1",
    "issue4304.pdf page 1",
    "issue6069.pdf page 1",
    "issue6108.pdf page 1",
    "issue7580.pdf page 1",
    "issue8088.pdf page 1",
    "issue8088.pdf page 2",
    "issue8088.pdf page 3",
    "issue8125.pdf page 1",
    "issue9243.pdf page 1",
];

/// Pages that are almost entirely glyph edges, where our *ink* matches the consensus.
///
/// Eight pages, measured in the seventy-fifth session, and they are one population rather than
/// eight questions. Each fails **only** on mean absolute difference — 5.4 to 6.4 against a bound
/// of 5.00 — while every other measure passes with room: worst tile 13.7 to 22.1 against 40, and
/// structural similarity 0.904 to 0.946 against 0.900. Structural similarity is the bound that
/// "does the work on text" (`pdfref::Tolerance`), and it says the same shapes are in the same
/// places.
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
///            1x       8x
/// ours     5.9139   6.0729
/// poppler  6.0271   6.0658
/// mupdf    6.0725   6.0819
/// ```
///
/// **Ours at eight times the resolution is 6.0729 against a two-ladder limit of 6.0658 and
/// 6.0819** — inside the references' own spread, which is this group's diagnosis exactly: the
/// marks are the right marks and the difference is glyph coverage at the page's own scale, 0.16
/// of 255 of it. Every printed metric is *inside* the class bound — mean 2.56 against 5.00,
/// worst tile 12.54 against 40.00, ssim 0.9445 against 0.9000 — and the page is contradicted
/// only because `poppler` and `mupdf` agree so closely that twice their spread is a tighter
/// bound than the floor. Trap 12, on the eleven pages above's own subject.
const CONTRADICTED_GLYPH_EDGES: [&str; 21] = [
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
/// line where pdf.js drew a black one. Our own numbers fit a thin line: mean 0.22 of a level
/// against a bound of 1.00, one tile at 10.76 against 6.04, 0.52% of pixels differing, with
/// `mupdf` and `ghostscript` the pair that agree. `soft_mask.rs` does read Table 142's `/BC`,
/// so the question is not whether the entry is read but what fills the area the mask's group
/// does not cover — which is the next thing to measure here, and the only entry on this list
/// with a clause and an expected picture attached.
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
/// `issue3694_reduced.pdf` stays on this list at 0.60 instead of 1.81 — mean 3.02 against a
/// bound of 5.00, 8.93% of pixels differing — which is a page of hairline-outlined display
/// type at seventeen pixels against two references that share `FreeType`.
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
/// **`issue7891_bc1.pdf` was the last page of `CONTRADICTED_UNEXPLAINED`, and it left in the
/// two-hundred-and-forty-third session — the list is empty.** It was measured in the sixty-first
/// without being fixed: the difference is one word inside a luminosity soft mask whose group
/// draws a 676 × 436 greyscale image reduced 2.8-fold, the five renderers' column centroids span
/// 0.76 of a pixel, and switching our own reduction between area averaging and point sampling
/// moved the raster without moving any printed metric. What was missing was the instrument, and
/// the two-ladder closed form arrived sixteen sessions ago.
///
/// ```text
///                1x        8x
/// ours        15.6380   15.6472
/// poppler     15.5410   15.6332
/// mupdf       15.5502   15.6346
/// ghostscript 15.6083
/// hayro       15.6228
/// ```
///
/// The two ladders agree to **0.0014 of 255**, which is the tightest limit this file has
/// recorded, and **ours at the page's own scale is 0.004 from it — the nearest of all five.**
/// `poppler` and `mupdf` are 0.09 under, together, which is why they vote: the corpus gate's
/// bound is *twice the consensus pair's own spread* (trap 12), and two renderers that agree to
/// 0.009 produce a bound tighter than any tolerance the file states.
///
/// **So the verdict is a statement about the pair, not about the page.** Every printed metric is
/// inside the class bound, and the one number derived from no reference at all puts us closest
/// to the geometry. That is the shape trap 12 describes, and this page is now its named witness
/// rather than a page nobody had measured.
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
const AMBIGUOUS_SHARED_JBIG2_DECODER: [&str; 19] = [
    "bitmap-composite-and-xnor-refine.pdf page 1",
    "bitmap-composite-or-xor-replace-refine.pdf page 1",
    "bitmap-halftone-refine.pdf page 1",
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
    "bitmap-symbol-texthuffrefinecustomdims.pdf page 1",
    "bitmap-symbol-texthuffrefinecustompos.pdf page 1",
    "bitmap-symbol-texthuffrefinecustomsize.pdf page 1",
    "bitmap-trailing-7fff-stripped-harder-refine.pdf page 1",
];

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
/// Three of our four distances are below every distance between two references. A diagnosis that
/// removes a candidate is worth what one that finds a defect is, and unlike reading the picture
/// it is checkable. ADR 0161.
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
const AMBIGUOUS_IMAGE_REDUCTION: [&str; 16] = [
    "issue269_2.pdf page 1",
    "blendmode.pdf page 1",
    "bug1703683_page2_reduced.pdf page 1",
    "two_pages.pdf page 1",
    // Taken out of the book beside it in the two-hundred-and-sixty-second session, by the band
    // rather than by the ranking: its worst tile is 81.57 where nothing else in `freeculture.pdf`
    // exceeds 29.09. The cartoon is a one-bit stencil, `ghostscript` thresholds it to a black
    // blob where the other four draw a grey halftone — this group's own sentence, one book over —
    // and the ladders put ours between the two references' limits: ours 15.5059 at 8x against
    // `poppler` 15.4710 and `mupdf` 15.5141 at 576 dpi.
    "freeculture.pdf page 171",
    "bug1799927.pdf page 1",
    // Arrived in the judged set in the two-hundred-and-eighteenth session, when the stencil it
    // is made of stopped being refused: a 421x320 one-bit CCITT scan drawn into 252x191 points
    // through a tiling pattern, which is this group's subject with two clauses in front of it.
    // Two ladders agree on the geometry — `poppler` 1.228 at 576 dpi and 1.227 at 2304, `mupdf`
    // 1.235 at 576 — and at the page's own scale ours is 1.236, `hayro` 1.274, `poppler` 1.218,
    // `mupdf` 1.205, `ghostscript` 1.091. Ours is 0.009 from the limit and nearest of the five.
    "issue13561_reduced.pdf page 1",
    "freeculture.pdf page 1",
    "issue5747.pdf page 1",
    "issue7229.pdf page 1",
    "issue7229.pdf page 2",
    "issue13372.pdf page 1",
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
const AMBIGUOUS_DEVICE_CMYK_CONVERSION: [&str; 2] =
    ["cmykjpeg.pdf page 1", "issue269_1.pdf page 1"];

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
/// same place, differently distributed. The handover's list of departures carries the
/// argument: the non-uniformity grid-fitting removes is an artefact of the binary scan
/// conversion §10.7.4 requires, and this tree already departs from that.
///
/// `Stroke::device_width` conditions the rule on `/SA` rather than applying it always, which
/// is what makes this a derivation rather than a coincidence: 30 corpus documents state
/// `/SA true` and this is the one page where it decides a pixel.
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
/// artefacts beside this file: ours **19.79**, `hayro` **19.83**, `ghostscript` 6.29, `poppler`
/// 3.51, `mupdf` 2.16. §10.7.4 asks for the pixel to be *painted* — "no matter how small the
/// intersection is" — which is a full mark and is what the two Rust renderers put down; the
/// three C ones shade it by something under a fifth. That is
/// `CONTRADICTED_ANTIALIASED_EDGES`' departure seen from the other side, and this project's
/// answer is the clause's rather than the consensus's.
///
/// The verdict was `ambiguous` before the fix as well, because the references disagree about the
/// weight among themselves, so nobody's pair ever agreed closely enough to contradict us for
/// drawing **none** of it. A page can be plainly wrong inside this verdict, which is §3a's whole
/// argument, and this is the group that demonstrates it.
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
/// **What is left is the right square, at 0.1159 against 0.1333.** There the clip is
/// load-bearing: the rule sits on the cell's edge and is *meant* to be halved, so each half is
/// drawn by a different cell and the two composite rather than add. Removing that clip would
/// draw the rule twice at full width, which is what `mupdf` does. Fixing it means rasterising
/// the tiling's coverage once rather than cell by cell, which is a different construction
/// altogether — and the page stays here until somebody takes it.
const AMBIGUOUS_TILING_CELL_CLIP: [&str; 1] = ["issue16038.pdf page 1"];

/// Ambiguous, and it is a page made almost entirely of sub-pixel line work.
///
/// `22060_A1_01_Plans.pdf` is an A1 architectural drawing rendered onto 842×1191 pixels: four
/// floor plans, their hatching, their dimension lines and their annotations, nearly all of it
/// strokes narrower than a device pixel. At 13.32 bounds from the nearest reference it was
/// second on the undiagnosed ranking, and the whole of the difference is how heavy a line
/// thinner than a pixel comes out.
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
/// ours 0.121   mupdf 0.263   ghostscript 0.424   hayro 0.857   poppler 1.578
///                                                            └ the limit is 0.174
/// ```
///
/// **`poppler` paints these rules nine times their area and `mupdf` one and a half times**,
/// which is §10.7.4 as written applied to a shape a tenth of a pixel thick, and it is the same
/// disagreement the two pages above hold. Ours is the only one under the limit, and the reason
/// is measured rather than guessed: see below.
///
/// ## Why ours is *under* it, which is a finding rather than the departure
///
/// A synthetic page with the same box and five identical rules — at the top edge, at y 300,
/// 160, 20 and at the bottom edge — says where the ink goes. Four of the five carry
/// **0.098 of an expected 0.1**; the one whose edge lies on the page's *top* carries 0.055.
/// `tiny-skia` draws a stroke under a pixel wide as a hairline smeared symmetrically about the
/// path — the ladder shows it, since each interior rule splits 0.047/0.051 across two rows
/// whatever its sub-pixel position — and for a rule 0.05 above the top edge half of that smear
/// falls above row zero and is lost with the raster. `doc/todo/11` holds it as the third
/// member of its family: a loss the *rasteriser's own construction* causes rather than the
/// display list.
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
/// ours (1x/4x/8x) 0.6753  0.7507    0.7516
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
const AMBIGUOUS_SUB_PIXEL_LINE_WORK: [&str; 9] = [
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
/// and checked.** Table 111 makes `/Widths` "the glyph width for the character code that equals
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
/// there. **We do not take the first route at all**, and these two pages are what that costs:
/// a substitute's glyphs are narrower than the widths the document states, so the difference
/// appears as letter spacing. On `bug1671312_ArialNarrow.pdf` we are the only renderer that
/// finds a *narrow* face at all, and the four that do not draw a better-fitting line.
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
/// every renderer here, and §9.8.1's Table 122 says where the answer comes from: the descriptor
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
const AMBIGUOUS_SUBSTITUTED_FACE: [&str; 7] = [
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
/// page 1 is one 512×512 greyscale image. Both sat undiagnosed on §3a's ranking — S2 at 3.89
/// worst mean over 23.28% of the page, `issue5475.pdf` at 6.68 over 31.05% — and both are made
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
const AMBIGUOUS_WIDGET_BORDER: [&str; 4] = [
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
const AMBIGUOUS_LINK_BORDER: [&str; 2] = ["bug766086.pdf page 1", "issue18030.pdf page 1"];

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

/// Ambiguous, and the reason this file's 320 pages are here is not the one that was written down.
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
/// The two on the ranking, with the closed form:
///
/// ```text
///                  limit             ours     hayro    poppler   mupdf    ghostscript
/// page 255        36.144 / 36.149   36.206   36.791   36.091   36.062   36.252
/// page 333        12.515 / 12.549   12.435   12.141   12.434   12.437   12.529
/// ```
///
/// Ours is 0.06 over the geometry on the first and 0.10 under it on the second, and on both the
/// five renderers are inside 0.7 of 255 of one another. **Nobody here is drawing anything anybody
/// else is not**, which is the finding: these pages are ambiguous by the tolerance's design and
/// not by any defect, and the tail of the ranking below 1.4 is mostly this book.
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
/// A sample is a sample, so the *whole* population's printed metrics were read as well: after the
/// two pages this group does not claim, `freeculture`'s 312 sit at mean 2.43 to 8.98, worst tile
/// 11.59 to 29.09, similarity 0.7209 to 0.9716, and `pdkids`'s 52 at mean 5.78 to 11.67, worst
/// tile 28.27 to 39.42, similarity 0.8075 to 0.9036. One band, no gaps, and the text tolerance —
/// 0.90 similarity, measured over 153 reference-against-reference pairs — running through the
/// middle of it.
///
/// **One page stood outside the band and it was not ours.** `freeculture.pdf` page 171 has a
/// worst tile of 81.57 where nothing else in the book exceeds 29.09; its cartoon is a one-bit
/// stencil, `ghostscript` thresholds it to a black blob where the other four draw a grey
/// halftone, and the page is `AMBIGUOUS_IMAGE_REDUCTION`'s subject rather than this group's.
/// That is the argument for reading the band before claiming the population: **a diagnosis by
/// sampling would have buried it**, and what found it was one number over three hundred pages
/// and then the picture.
const AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE: [&str; 370] = [
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
    "freeculture.pdf page 255",
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

/// Ambiguous, and it is **one paper under twelve names** — 154 of the bucket's 678.
///
/// `tracemonkey.pdf` is pdf.js's canonical fixture: *Trace-based Just-in-Time Type
/// Specialization for Dynamic Languages*, fourteen pages of two-column academic text at about
/// nine points. Eleven other corpus documents are the same fourteen pages with something added
/// — highlights, comments, free text, an editable annotation, an accessibility tree — and
/// `pdftotext` on page 9 gives the *same md5* for all of them. So this group is 154 names and
/// **one** finding, and saying otherwise would make the count a vanity number.
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
/// So nobody is drawing anything anybody else is not. What makes all 154 `ambiguous` is the
/// **text** tolerance — 0.90 structural similarity, measured over 153 reference-against-
/// reference pairs because five rasterisers cannot agree more closely than that about small
/// glyphs — applied to a page that is nothing but small glyphs. Across the 154 the metrics form
/// one band: mean 3.51 to 9.93, worst tile 20.94 to 48.31, ssim 0.7977 to 0.9194, against
/// bounds of 5.00, 40.00 and 0.9000. §10.7.4's own last sentence is what licenses the spread,
/// and `AMBIGUOUS_GLYPH_SCAN_CONVERSION` quotes it.
///
/// **`poppler` on page 11 is the one number worth keeping.** It is 15.7177 at 72 dpi against its
/// own 14.9441 at 576 — 5% over its geometry where every other page has it 1% under. That is
/// §10.7.4 as written on marks a fraction of a pixel wide, one page over from where
/// `AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY` found it, and it is not ours.
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
/// of the pages reach §3a's bucket with a shape nothing else in it has: similarity 0.9319 to
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
/// # What it does not determine, and §10.3.1 says so in a sentence
///
/// > The specific method by which the CIE-based destination colour space is established is
/// > beyond the scope of this document, but may include the use of Output Intents
///
/// So the second half of the journey — an XYZ to a pixel — is each processor's, and this sheet
/// is built to make that visible. Ours is one route and it is written down rather than tuned:
/// Bradford adaptation to D50 and then the sRGB matrix and transfer, in `colour::xyz_d50_to_srgb`,
/// which is the *only* place in the tree where an XYZ becomes a pixel (ADR 0012) — `Lab`,
/// `CalGray`, `CalRGB` and every ICC profile arrive there, so the four cannot drift apart.
///
/// This is §3a's third shape, and the sharpest instance of it the bucket holds: the clause is
/// closed about the part it defines, open about the part it does not, and names the clause that
/// says so.
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

/// # And a second document, on the same instrument, in the two-hundred-and-sixty-third
///
/// `TAMReview.pdf` is 23 pages of a technical review at paper size — dense text, tables, the same
/// nine-point band — and 22 of them are in this bucket. Read as a population the way both books
/// were: mean 4.05 to 9.96, worst tile 11.59 to 33.63, similarity 0.7722 to 0.9214, one band and
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
const AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE: [&str; 180] = [
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
    "comments.pdf page 1",
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
    "highlights.pdf page 1",
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
/// `HT`, `BG`, `BG2`, `UCR`, `UCR2` and `UseBlackPtComp`, and every one of them describes a
/// marking device. `poppler` at 16.64 is between the two, which is a mask applied at some other
/// strength.
///
/// What is left between ours, `mupdf` and `hayro` is half a level of 255 on a blur, and it is
/// §11.6.5.2's own arithmetic on a `DCTDecode` greyscale image — the three of us differ by less
/// than the eight-bit quantisation of the mask at its brightest, which this image reaches at
/// 110 of 255. The page stays `ambiguous` because two renderers are 2× and 6× away, which is a
/// statement about them.
const AMBIGUOUS_MASKED_BLUR: [&str; 1] = ["issue19634.pdf page 1"];

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

/// Ambiguous, and three pages where every renderer paints more than the geometry.
///
/// ```text
///                    limit             ours     hayro    poppler   mupdf    ghostscript
/// issue12963 p8     5.6177 / 5.6180   5.601    5.841    5.921    5.630    5.488
/// two_pages p2      1.0448 / 1.0457   1.032    1.029    1.070    1.073    1.063
/// issue12295 p1     7.4106 / 6.9985   8.792   12.744   11.036   10.504   14.763
/// ```
///
/// On the first two the ladders agree to four figures and ours is nearest the geometry — 0.017
/// and 0.013 under, where the pair that is furthest is 0.3 and 0.03 over.
///
/// **`issue12295.pdf` is the extreme of this group's standing subject and is worth its numbers.**
/// The two ladders themselves are 0.41 apart, so there is no exact limit, but both are near 7
/// and **all five renderers are above them at the page's own scale** — ours by 1.4, `mupdf` by
/// 3.1, `poppler` by 3.6, `hayro` by 5.3, `ghostscript` by 7.4. A page whose marks are thin
/// enough that every renderer paints one to seven levels more than their area is §10.7.4 as
/// written on five implementations at once, and ADR 0025's departure is why ours is the
/// smallest of the five overshoots rather than the largest.
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
/// **`issue12337.pdf` is the case where step 6 has one ladder and not two**: `pdftoppm` at 576
/// dpi produces no image for it at all, so only `mupdf`'s 16.225 is available, and a single
/// ladder cannot tell convergence from drift. What can be said without it is that four of the
/// five renderers are inside 0.24 of each other at the page's own scale and `hayro` is 1.4
/// below all of them, which makes `hayro` the one to explain rather than us. Listed on that
/// basis, which is weaker than the other two and is said so.
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
const AMBIGUOUS_FOUR_ANSWERS: [&str; 2] = ["bug1844583.pdf page 1", "tagged_stamp.pdf page 1"];

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
/// those and FreeType derives each entry's extent from the entry itself.
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
const AMBIGUOUS_REFERENCE_DREW_NOTHING: [&str; 1] = ["issue6006.pdf page 1"];

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
/// rectangle, *is* the rectangle. Ours fills it. **`poppler` strokes 112 units centred on the
/// rectangle's edge**, so its blue covers most of the page — ink **201.31** against ours 29.64,
/// and the document's own text says "this text should be visible". `mupdf` 17.06,
/// `ghostscript` 17.33 and `hayro` 16.85 draw no link border at all, for
/// `CONTRADICTED_LINK_BORDER`'s reasons.
///
/// So four renderers disagree three ways and the clause names one of them. This is the shape
/// step 1 calls everybody-against-us read the other way round: the page reached 1.90 bounds
/// because *one* reference is very far off, and the printed distance from the nearest is the
/// number that accuses us — 1.90 here, against 19.33 from the furthest.
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
/// # What the specification determines, which is `doc/todo/00`'s third shape
///
/// §8.6.5.5 defines the *source*: the profile is the space, and both renderers read the same one.
/// The destination is where the standard stops, and it says so in one sentence — §10.3.1 puts
/// "[t]he characteristics of the output device" beyond the scope of this document, and its NOTE
/// names "assumptions made by the PDF processor software". Neither implementation is departing
/// from a clause, because there is no clause left to depart from: what a matrix-shaper profile's
/// XYZ becomes on a screen is each processor's own assumption about that screen.
///
/// So this page is `AMBIGUOUS_DEVICE_CMYK_CONVERSION`'s argument one colour space over, and the
/// group is separate because the *evidence* is different: there the spread is between four
/// conversions of a device space, here it is between two evaluations of one embedded profile,
/// and only the resolution ladder could tell the second from a scan-conversion difference.
///
/// The remaining 43% of the page's deficit is its text, which is
/// `AMBIGUOUS_GLYPH_SCAN_CONVERSION`'s subject and is measured alone on page **6** of the same
/// document — 456 commands, no image at all, two ladders agreeing to 0.0055 of 255 and ours
/// climbing onto the limit to within 0.025.
const AMBIGUOUS_ICC_MATRIX_PROFILE: [&str; 1] = ["issue19971.pdf page 5"];

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
const AMBIGUOUS_GLYPH_SCAN_CONVERSION: [&str; 17] = [
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
/// pages sit in one band — mean 4.66 to 7.10, worst tile 15.62 to 23.71, ssim 0.8632 to 0.9088 —
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
        .chain(&AMBIGUOUS_LOCA_OUT_OF_ORDER)
        .chain(&AMBIGUOUS_SPACE_DRAWN_AS_A_MARK)
        .chain(&AMBIGUOUS_NEAREST_THE_GEOMETRY)
        .chain(&AMBIGUOUS_FOUR_ANSWERS)
        .chain(&AMBIGUOUS_ONE_LADDER)
        .chain(&AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY)
        .chain(&AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE)
        .chain(&AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE)
        .chain(&AMBIGUOUS_JPEG_COMPONENT_IDS)
        .chain(&AMBIGUOUS_RECOVERED_PAGE_TREE)
        .chain(&AMBIGUOUS_STANDARD_FOURTEEN_FACE)
        .chain(&AMBIGUOUS_MASKED_BLUR)
        .chain(&AMBIGUOUS_MATTE_WITHOUT_A_SOFT_MASK_IMAGE)
        .chain(&AMBIGUOUS_GRADIENT_ON_A_TIGHT_BOUND)
        .chain(&AMBIGUOUS_OVERSIZED_BORDER)
        .chain(&AMBIGUOUS_CONSTRUCTED_WIDGET)
        .chain(&AMBIGUOUS_GLYPH_SCAN_CONVERSION)
        .chain(&AMBIGUOUS_ICC_MATRIX_PROFILE)
        .chain(&AMBIGUOUS_A_REFERENCE_DECODED_THE_IMAGE_WRONG)
        .copied()
        .collect()
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
/// The list is data rather than a `const` because it is 735 names and the argument for each
/// one is that there is *no* argument yet — the diagnoses live in the groups above, where
/// they can be read.
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
    fn of(triangulation: &pdfref::Triangulation) -> Option<Self> {
        let bounds = &triangulation.judged_by;
        let ratio = |c: &raster_compare::Comparison| {
            let mean = c.mean_error / bounds.max_mean;
            let tile = c.worst_tile_error / bounds.max_worst_tile;
            let structural =
                (1.0 - c.structural_similarity) / (1.0 - bounds.min_structural_similarity);
            mean.max(tile).max(structural)
        };
        let mut ratios = triangulation.ours.iter().map(|(_, c)| ratio(c));
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
    /// Processor time spent in our own pipeline, and in the three external renderers.
    ///
    /// Summed across the run and reported, because "where does this gate's time go" is
    /// otherwise answered by intuition — and the intuitive answer, that three subprocesses
    /// must dominate a Rust render, is wrong here by a factor this measures.
    spent: Spent,
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
}

impl Work {
    /// How this page is named in the report and in the ratchet lists.
    fn name(&self) -> String {
        let file = self.path.file_name().map_or_else(
            || self.path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        format!("{file} page {}", self.page)
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
                })
                .collect::<Vec<_>>()
        })
        .collect();
    items.extend(
        specifications
            .into_iter()
            .map(|path| Work { path, page: 1 }),
    );
    items.sort_by(|a, b| a.path.cmp(&b.path).then(a.page.cmp(&b.page)));
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
    let stem = work.path.file_stem().unwrap_or_default().to_string_lossy();
    // One directory per page, so a document's pages cannot overwrite one another's
    // evidence — every renderer writes to a fixed file name inside it.
    let case = format!("{stem}-p{}", work.page);
    let work_dir = work_root
        .join(stem.as_ref())
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
                return Examined {
                    name,
                    verdict: Verdict::NoRender(detail),
                    complete: false,
                    distance: None,
                    spent,
                };
            }
        }
    };

    let mut references = {
        let started = Instant::now();
        let rendered = render_references(work, &work_dir, available, cache);
        spent.references = started.elapsed();
        match rendered {
            Ok(references) => references,
            Err(detail) => {
                let _ = std::fs::remove_dir_all(&work_dir);
                return Examined {
                    name,
                    verdict: Verdict::NotComparable(detail),
                    complete,
                    distance: None,
                    spent,
                };
            }
        }
    };

    let outvoted = match reconcile(&mut ours, &mut references) {
        Ok(outvoted) => outvoted,
        Err(verdict) => {
            return Examined {
                name,
                verdict,
                complete,
                distance: None,
                spent,
            };
        }
    };

    // Text sets the noise floor between independent rasterisers — glyph hinting differs
    // between implementations in a way flat fills do not — so a page carrying glyphs is
    // judged by the bounds measured on text pages.
    let tolerance = if has_text {
        Tolerance::TEXT_HEAVY
    } else {
        Tolerance::VECTOR
    };

    let triangulation =
        match pdfref::triangulate_with(&ours, &references, &tolerance, Judgement::CORPUS) {
            Ok(triangulation) => triangulation,
            Err(e) => {
                return Examined {
                    name,
                    verdict: Verdict::NotComparable(format!("{e}")),
                    complete,
                    distance: None,
                    spent,
                };
            }
        };

    let verdict = verdict_of(&triangulation, outvoted.as_deref());
    let distance = Distance::of(&triangulation);
    if matches!(verdict, Verdict::Agrees) {
        // Nothing to look at, and three thousand agreeing pages of PNGs is a gigabyte.
        let _ = std::fs::remove_dir_all(&work_dir);
    } else {
        // A fourth render, for the eye rather than for the vote. `hayro` shares its font
        // rasteriser, its deflate, its JPEG decoder and both new image codecs with us, so
        // its agreement is not evidence — `Reference::independence` says so and
        // `Reference::voting` keeps it out of the consensus. But a page the three
        // references cannot settle is exactly where a fourth reading helps, and this is the
        // only one of the four written in the same language, so a difference between it and
        // us cannot be blamed on C.
        //
        // Rendered only for pages worth looking at, which is what keeps it off the gate's
        // critical path: an agreeing page has its whole directory deleted a few lines up.
        if let Ok(raster) = cache.render(Reference::Hayro, &work.path, work.page, DPI, &work_dir)
            && raster.width == ours.width
            && raster.height == ours.height
        {
            references.push((Reference::Hayro, raster));
        }
        let _ = report::write_artefacts(&work_dir, &case, &ours, &references, &triangulation);
    }

    Examined {
        name,
        verdict,
        complete,
        distance,
        spent,
    }
}

/// Renders one page with every available reference.
///
/// A reference that fails on a document is not evidence of anything — many of these files
/// are deliberately damaged, and a renderer refusing one is the correct behaviour — so its
/// absence is tolerated as long as two remain. Fewer than two is reported with every
/// failure's own message, because "not comparable" without a reason is not actionable.
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
) -> Result<Vec<(Reference, Raster)>, String> {
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
            Err(detail) => failures.push(detail),
        }
    }
    if rendered.len() < 2 {
        return Err(failures.join("; "));
    }
    Ok(rendered)
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
                measurements(triangulation)
            ))
        }
        Outcome::Ambiguous => Verdict::Ambiguous(format!("{}{note}", measurements(triangulation))),
        Outcome::NotEnoughReferences { available } => {
            Verdict::NotComparable(format!("{available} reference(s)"))
        }
        // `Outcome` is non-exhaustive. A conclusion this gate has never seen must be
        // visible rather than quietly folded into one of the outcomes above.
        other => Verdict::NotComparable(format!("unrecognised outcome {other:?}")),
    }
}

/// Fails the gate if the sandboxed decoder is not available.
///
/// JBIG2 and JPEG 2000 are decoded by a separate program, and Cargo does not build another
/// package's binaries when it tests this one. Without that check a missing worker would not
/// fail anything — it would quietly turn 152 documents' images into reports and move the
/// ratchets, which is the kind of silent number change this whole file exists to prevent.
fn require_the_sandbox() {
    if let Err(error) = pdf_sandbox::Sandbox::shared().confinement() {
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
fn measurements(triangulation: &pdfref::Triangulation) -> String {
    let worst = triangulation.ours.iter().map(|(_, c)| c).fold(
        None::<&raster_compare::Comparison>,
        |worst, c| match worst {
            Some(previous) if previous.worst_tile_error >= c.worst_tile_error => Some(previous),
            _ => Some(c),
        },
    );
    let bounds = &triangulation.judged_by;
    let applied = format!(
        "bound mean {:.2} worst tile {:.2} ssim {:.4}",
        bounds.max_mean, bounds.max_worst_tile, bounds.min_structural_similarity
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
    let contradicted: Vec<&str> = CONTRADICTED_PAGE_ROUNDING
        .iter()
        .chain(&CONTRADICTED_SHARED_JBIG2_DECODER)
        .chain(&CONTRADICTED_IMAGE_RESAMPLING)
        .chain(&CONTRADICTED_CALIBRATED_COLOUR)
        .chain(&CONTRADICTED_REFERENCE_GLYPH_WIDTHS)
        .chain(&CONTRADICTED_NEGATIVE_LINE_WIDTH)
        .chain(&CONTRADICTED_DEVICE_CMYK_CONVERSION)
        .chain(&CONTRADICTED_SUBPIXEL_IMAGE)
        .chain(&CONTRADICTED_MASK_QUANTISATION)
        .chain(&CONTRADICTED_VISIBILITY_EXPRESSION)
        .chain(&CONTRADICTED_REFERENCES_DREW_NOTHING)
        .chain(&CONTRADICTED_LINK_BORDER)
        .chain(&CONTRADICTED_GLYPHS_JUDGED_AS_VECTOR)
        .chain(&CONTRADICTED_ANTIALIASED_EDGES)
        .chain(&CONTRADICTED_GLYPH_EDGES)
        .chain(&CONTRADICTED_SYMBOLIC_FONT_FLAGS)
        .chain(&CONTRADICTED_SUBSTITUTED_FONT)
        .chain(&CONTRADICTED_UNEXPLAINED)
        .chain(&CONTRADICTED_TIGHT_CONSENSUS)
        .copied()
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

/// Prints every document that is not agreement, then the totals.
///
/// The per-document lines come first and unabbreviated: on a failure they are the evidence,
/// and a summary that hides which page moved is a summary nobody can act on. Each count is
/// given twice — over everything, and over the pages we claim to draw completely — because
/// only the second is gated and conflating them flatters the result.
fn report(results: &[Examined], elapsed: std::time::Duration, cache: &Cache) {
    for examined in results {
        if !matches!(examined.verdict, Verdict::Agrees) {
            println!(
                "  {}: {}{} — {}",
                examined.name,
                examined.verdict.label(),
                if examined.complete {
                    ""
                } else {
                    " (incomplete)"
                },
                examined.verdict.detail()
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

    rank_the_undiagnosed(results);
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
