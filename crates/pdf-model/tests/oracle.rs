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
/// The four that remain are the ones whose story is still only about rounding.
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
const CONTRADICTED_PAGE_ROUNDING: [&str; 4] = [
    "colorkeymask.pdf page 1",
    "issue21346.pdf page 1",
    "bug1669097.pdf page 1",
    "issue19505.pdf page 1",
];

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
const CONTRADICTED_DEVICE_CMYK_CONVERSION: [&str; 4] = [
    "function_based_shading_cmyk.pdf page 1",
    "function_based_shading_cmyk.pdf page 2",
    "postscript_type4_many_outputs.pdf page 1",
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
/// 14 pages. The weakest entries here, because the difference need not be anyone's defect:
/// every renderer substitutes, and where two references happen to choose the same system
/// font and we choose another, the consensus is about their font rather than about the page.
/// `pdf-font`'s `substitute` module is the only machine-dependent code in the tree, so this
/// is also the group that could legitimately differ on another machine.
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
const CONTRADICTED_SUBSTITUTED_FONT: [&str; 14] = [
    "bad-PageLabels.pdf page 1",
    "calrgb.pdf page 1",
    "calrgb.pdf page 11",
    "calrgb.pdf page 12",
    "calrgb.pdf page 5",
    "franz_2.pdf page 1",
    "issue4304.pdf page 1",
    "issue5238.pdf page 1",
    "issue6108.pdf page 1",
    "issue7580.pdf page 1",
    "issue8088.pdf page 1",
    "issue8088.pdf page 2",
    "issue8088.pdf page 3",
    "issue8125.pdf page 1",
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
const CONTRADICTED_UNEXPLAINED: [&str; 23] = [
    "bug1108301.pdf page 1",
    "bug1151216.pdf page 1",
    "bug1175962.pdf page 1",
    "bug1200096.pdf page 1",
    "bug1252420.pdf page 1",
    "bug894572.pdf page 1",
    "freeculture.pdf page 313",
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
    "issue7891_bc1.pdf page 1",
    "issue7901.pdf page 1",
    "issue8097_reduced.pdf page 1",
    "issue8570.pdf page 1",
    "openoffice.pdf page 1",
    "transparent.pdf page 1",
];

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
                    spent,
                };
            }
        };

    let verdict = verdict_of(&triangulation, outvoted.as_deref());
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
        .chain(&CONTRADICTED_SYMBOLIC_FONT_FLAGS)
        .chain(&CONTRADICTED_SUBSTITUTED_FONT)
        .chain(&CONTRADICTED_UNEXPLAINED)
        .copied()
        .collect();
    assert_ratchet(
        "contradicted by the reference consensus",
        &named(&|e| matches!(e.verdict, Verdict::Contradicted(_))),
        &contradicted,
    );
    assert_ratchet(
        "disagreeing with the references about page geometry",
        &named(&|e| matches!(e.verdict, Verdict::OurGeometry(_))),
        &GEOMETRY,
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
}

/// Holds an outcome to an exact set of pages.
///
/// Both directions fail. A new name is a regression; a missing one means the list is stale
/// and the entry must be deleted, which is what keeps a fixed page from silently coming
/// back.
fn assert_ratchet(what: &str, actual: &[&str], expected: &[&str]) {
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
        "{} page(s) newly {what}: {new:?}\n\
         Each is a page two independent implementations agree about and we do not. Read \
         the artefacts named above, then take the disagreement to the specification — \
         never to what the references produce.",
        new.len()
    );
    assert!(
        gone.is_empty(),
        "{} page(s) no longer {what}: {gone:?}\n\
         Delete them from the list: a fixed page must not be able to come back.",
        gone.len()
    );
}
