# Traps: pixels and rasterisers

Status: **standing** — each is a mistake somebody actually made in this tree.
Read by: a round that changes what gets drawn — the interpreter's marks, either rasteriser,
colour, or a cross-backend scene. **If this round can change a pixel, trap 1 is the one that may
not be skipped.**

`doc/HANDOVER.md` is the index and names which group holds which trap. **Every trap keeps its
number**, because `crates/`, `tools/`, `doc/conformance/ledger.toml` and dozens of ADRs cite them
by number and an ADR is not edited to follow a file that moved underneath it (ADR 0232 §2).

## Traps

### 1. The metrics lie. Look at the page.

**The most important thing in this file.** `Interpretation::is_complete()` says what the
interpreter *knows* it skipped. It cannot say a font loaded and produced garbage, a page is upside
down, or a gradient came out opaque. The archetype: wiring bare-CFF support in made every affected
document report `unsupported: []` and render **almost no text**.

`cargo test -p pdf-model --test render_real_pdf -- --nocapture writes_inspectable` writes PNGs;
the oracle's artefacts are better. Two automated checks catch a wrong mapping, both in
`pdf-font/src/loading.rs`: `the_pdf_widths_agree_with_the_font_programs_own_advances` — the `/Widths`
and the charstring's own advance are independent statements of one fact — and
`an_uncovered_code_has_no_glyph_rather_than_a_guessed_one`. Neither replaces looking.

**Every page a new feature makes drawable is a page nobody has ever looked at**, and the habit has
paid every session since the tenth: dashed squares that should not have been solid; a fax page
**upside down** because `/Rotate` 90 and 270 had been exchanged since the first page tree; a
gradient painted opaque because one `return` dropped §11.6.4.4's alpha; a `0 w` line invisible on
the GPU; `issue7901.pdf` drawing `üãÍ†Ë` because Table 115's presence condition was read as a
condition on meaning. **A page a feature makes drawable can be one that never rendered at all** —
the oracle's `no render` count is a to-do list of pages nobody has looked at, and `tools/state.sh
oracle` prints it — one left it in the hundred-and-seventy-seventh session when a page the file's
own cross-reference table had hidden started rendering (ADR 0148). **That sentence stood for four
hundred rounds and nobody had looked**, which the five-hundred-and-seventy-fifth found out by
looking: it is the one verdict the gate reaches *before* the references are asked, so a page three
readers draw and we do not was indistinguishable there from a page nobody can read — one of each was
in it. Every page of the bucket is diagnosed and held by name now, and the recipe for asking the
references about one is `doc/oracle-and-corpus.md` §3d (ADR 0410).

**And the rule inverts, which is the version worth having**: twice the picture has rejected a
*reading of the specification* rather than finding a defect. `issue6621.pdf` and `issue7901.pdf`
were both code that was right about the clause it cited.

**A contradicted page's group names a hypothesis, not a diagnosis — eleven for eleven on being
wrong**, the newest being `smask_luminosity_oob_transfer.pdf`, whose `CONTRADICTED_MASK_QUANTISATION`
had said since the sixth session that the level it differs by "comes from the mask being quantised" —
and the mask is one byte, the byte is 191, and one byte of mask predicts the closed form exactly.
What produced the level was `tiny-skia` compiling its *low-precision* pipeline, whose `div255` is an
upper bound on a division by 255 rather than its rounding, twice per pixel (ADR 0418). The one before
it is `issue9940.pdf`, whose group had said for hundreds of sessions that
`mupdf` and `ghostscript` "take its components for `DeviceRGB`" — and no renderer does: ours and
`poppler`'s are §8.6.5.3 plus IEC 61966-2-1 to the level, the pair that contradicts us moves *one
channel*, and the `DeviceRGB` reading moves all three (ADR 0349). The one before it is
`calrgb.pdf`'s four pages, which spent four hundred and fifty-five
sessions inside `CONTRADICTED_SUBSTITUTED_FONT` differing from each other in one entry no voting
renderer reads. The one before it, `issue4304.pdf` in the four-hundred-and-fifth session, spent a
hundred and eighty sessions in the same group while the difference was six
spaces of zero width and the side-by-side said so in one look. Open the artefact before believing the label — **and measure it, because a label this
project wrote is still a label**. **A group's note that names another group's mechanism is the
cheapest tell there is**: `calrgb.pdf`'s said "a residue of colour management rather than of fonts"
and stayed under the *font* group's name for four hundred and fifty-five sessions after the session
that wrote it, and `CONTRADICTED_MASK_QUANTISATION`'s argued the verdict from two references
agreeing "within one level of *each other*", which is `CONTRADICTED_TIGHT_CONSENSUS`'s sentence,
while its *name* claimed a cause no line under it measured. **Two claims in one note, one of them
another group's and one of them unmeasured, is the shape to read first.** Twice the instrument that
settled one was the font's own `cmap`, `loca` and `post` tables read directly: ten minutes, exact.

### 2. A paint is positioned in the *path's* space, not the device's

Both `tiny-skia` and Vello apply the drawing transform to a paint as well as to the shape, so
composing the page-to-device transform into it yourself applies it twice. Both backends did, and it
shipped: every gradient mirrored about the page's centre line, and `issue19971.pdf`'s photograph
came out as one flat rectangle. Three things about how it survived: **no metric saw it**; **the
CPU-vs-GPU comparison could not**, because both had it; and **every scene compared them with a
gradient running along x**, where a y mirror is invisible.

Guards: `render-cpu/tests/shading_placement.rs` and `image_placement.rs` at three scales, plus
`headless_gpu.rs`'s vertical-gradient and image scenes. All confirmed to fail when the defects are
reintroduced.

**Five instances now, and each teaches a different edge:**

- **A convention that agrees with the clause is worse than one that does not.** `tiny-skia` draws
  a zero-width stroke as one device pixel, which is exactly §8.4.3.2 — so the rule was never
  written down and every `0 w` line was invisible on the GPU for fifteen sessions. **Where two
  backends are the oracle, a decision either can make alone is a decision neither has made**,
  which is why the device decisions live in `pdf-render`.
- **Three libraries, three answers, none the standard's.** §8.5.3.2's zero-length stroke:
  `tiny-skia` paints a projecting cap where the clause asks for none, `kurbo` drops the contour,
  and a one-`m` path is an error on one and silence on the other. `pdf-render`'s `degenerate.rs`
  states it once.
- **What a library cannot say at all.** §11.4.6's knockout is Porter-Duff Source modulated by
  coverage; Vello's layers compose over the layer's whole *bounding box*, so `Compose::Copy`
  erased a row outside the shape. **Where one backend states a clause directly and the other has
  to build it, the built one needs a scene at the magnitude *and* the fractional coverage where
  the two constructions differ** — the knockout scene has a diagonal edge for that reason.
- **A scene set is worth what its scenes can express.** Fourteen cross-backend scenes existed and
  every `Command` in all of them carried `BlendMode::Normal`, so sixteen blend functions had never
  been compared — and three disagree by 113 of 255. **Ask what parameter every scene leaves at its
  default.** ADR 0046.
- **A scene must fail at the defect's *magnitude* as well as in its axis.** The first reduced-image
  scene was in the right axis and **passed with the GPU's filter removed altogether**. Deleting the
  code a scene guards is one command, and it is the only thing that establishes the scene guards
  it.

### 6. Colour: one conversion, and the specification often has no answer

Three separate `DeviceCMYK` → RGB conversions used to live here and they disagreed. Nothing about
a rendered page reveals that. `pdf-model/tests/colour_paths.rs` drives one value through all three
routes and demands they agree. **Add no fourth path**: `ColourSpace::to_rgb` is the only place a
colour becomes RGB, and `colour::xyz_d50_to_srgb` the only place an XYZ becomes a pixel — that
second rule exists because the same defect recurred one level down in a nine-constant matrix.

This file said for thirty-two sessions that ISO 32000-2 defines no `DeviceCMYK` conversion. **It
does — §10.4.2.5** — and what the standard actually does is *rank two answers*: §10.3's ICC route
for an ICC-enabled processor, which this is, and §10.4.2's "crude approximations" otherwise. The
three sources outranking the table are `/DefaultCMYK` (§8.6.5.6), an output intent's
`/DestOutputProfile` (§14.11.5), an `ICCBased` profile. Read ADRs 0009 and 0042 before touching
it. The same shape recurs for a Cal space's `/BlackPoint`: §8.6.5.9 leaves black point compensation
to the processor whenever `/UseBlackPtComp` is `Default`, which is every real document.

### 12b. A test suite made of small scenes tests small scenes

Fourteen cross-backend fixtures — a gradient, a knockout group, sixteen blend modes — each a
handful of commands at one modest size. **The first real page at a real window's size came back
blank**, and nothing in the tree could see it: the corpus and the oracle rasterise with
`render-cpu`, so the GPU backend's only judge was those fixtures.

Vello sizes its GPU working buffers from constants "hand picked to accommodate the vello test
scenes"; a scene needing more overflows them *on the device*, which sets a flag, stops filling,
and returns `Ok(())` over a blank target. Page 6 of ISO 32000-2 at 1132×1600 is such a scene, and
1132×1600 is an A4 page fitted to a laptop window. ADR 0127.

Three rules out of it. **Ask what size every scene in a suite is**, not only which feature it uses
— ADR 0046 asked the feature question and this is the same question one axis over. **Where a
dependency returns success, ask what it does when it fails**: if the answer is "nothing visible",
that report is this project's to construct. And **a fix belongs on the path the person uses**: the
check first landed in `rasterize`, which is tier 1 and what the tests call, while the window draws
to its own surface through tier 2 — so the test went green while the black page stayed black.
`render_gpu::render_checked` is public for that reason and `Renderer::render_to_texture` is not
called anywhere in this tree.

And a fourth, from what enabling the feature cost: **a feature flag taken for one effect brings
its others.** `debug_layers` also makes vello hand wgpu a zero-length buffer slice whenever a
scene produces no lines — a blank page — and wgpu panics on it, which under `panic = "abort"`
kills the viewer. Two existing fixtures caught it; `keep_the_line_soup_non_empty` works around it
with one transparent rectangle. **Run the whole suite after turning a dependency's feature on**,
not only the test that motivated it.

### 14. A target that *is* the region a clause names cannot tell you whether you applied it

ISO 32000-2 §14.11.2.1 is a `shall`: "[t]he crop box defines the region to which the contents of the
page shall be clipped (cropped) when displayed or printed". `pdf_model::interpret` deliberately
keeps the marks a content stream made outside that box — a display list is what the file says — and
**for the whole life of this tree nothing put the clip back**. No gate could see it, and the reason
is the shape of every gate rather than an oversight in any of them: the corpus, the oracle and the
quorra comparison all rasterise a **page-sized** target, whose extent *is* the crop box, so the
raster's own edge did the cutting. A page that drew a metre beyond its boundary was indistinguishable
from one that drew nothing there.

A window is larger than its page. What a reader saw was ink beside the page and over the next page of
a column — `issue1350.pdf` draws a whole second voucher above its crop box — and the census that
followed found **3690 of 66 887 first pages** do this (ADR 0447).

**The question to ask is not "does a gate cover this clause" but "could this gate distinguish the two
answers".** Where a clause names a region and the instrument's extent is that region, the answer is
no, whatever the coverage table says. `doc/traps/instruments-and-reports.md` is the same subject one
directory over and its trap 11 is the inverse: a *report* whose condition is wrong names the wrong
documents, and a *gate* whose target is the clause's own region names none.

Two things follow, and the second is the cheaper one:

- **A round implementing a region a clause states should rasterise something bigger than it once**,
  by hand, before believing anything. Three minutes.
- **The census belongs before the code.** The condition that reads like the clause's — here, a
  `/CropBox` smaller than the `/MediaBox` — named 1121 documents of which 804 mark anything, and
  missed 2886 that mark outside a boundary equal to their medium. Derive the condition from the
  clause's own words and print what it matched.

## Things worth knowing

- **A command draws into the rows its clip admits, not into the page.** `Band` in
  `render-cpu/src/lib.rs`, ADR 0010. The device transform handed to a command already carries the
  band's row offset, and the clip mask is band-tall and page-wide because `tiny-skia` needs it to
  share the pixmap's row stride.
- **The display list is deliberately flat.** `tiny-skia` wants per-clip masks, Vello a layer stack;
  both translate. That neither library's model is native is the evidence the neutral form is right.
- **RADV and lavapipe produce byte-identical output**, so goldens need not be per-adapter. A test
  pins this; if it fails, the assumption has broken, not the code.
- **`test-scenes` holds the same page twice**, as a display list and as PDF bytes. That pairing let
  the harness work before a parser existed.
