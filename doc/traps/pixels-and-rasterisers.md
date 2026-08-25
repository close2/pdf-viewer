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

**A comment that states the picture a construction produces is a claim about a raster, and only a
raster settles it.** `Border::inset`'s said that clamping the inset to the centre line "fills the
rectangle solid", the oracle's note about the corpus page that exercises it said "[o]urs fills it",
and the ledger row said it a third time. What `/Border [0 0 112]` on a 150 × 20 `/Rect` actually
drew was a 38 × 20 block in the middle of the rectangle, and a width past *both* dimensions drew
nothing at all — the stroke of a degenerate path loses the two sides that degenerated. Three
documents agreed with each other for eight hundred sessions because each was written from the
arithmetic rather than from the page, and no test, gate or reference could see it: the arithmetic is
right in every case a test covered, the one corpus witness is *ambiguous*, and on an ambiguous page
nothing ranks us at all. Render it and look (ADR 0674).

**A contradicted page's group names a hypothesis, not a diagnosis — twelve for twelve on being
wrong, and the thirteenth examination is the first where the name held.** That one is
`CONTRADICTED_NEGATIVE_LINE_WIDTH` in the six-hundred-and-fifty-first session: `issue19633.pdf`
really is about the `-0.1 w` its group names, and what was a hypothesis was everything written
*under* the name — the clause reading (§8.4.1 decides a value outside a parameter's range and
names the line width while doing it, so our clamp is a `shall` rather than the documented choice
the note claimed), and every attribution to a reference (`poppler` and `ghostscript` stroke the
magnitude, `mupdf` paints nothing within 5° of an axis and its own floor beyond 10°, and the two
that vote were answering different questions). **So the thing to distrust is the note, and the
name is only its first sentence** — ADR 0480.

**And a note has a third way of being wrong, which is neither its name nor its reading: a sentence
that was true when written and that nothing pointed at when the tree moved under it.**
`CONTRADICTED_TIGHT_CONSENSUS` said of `colors.pdf` that ours is the closed form with every edge
rounded to a quarter and `hayro`'s is the exact one. ADR 0476 made ours the exact form **three
sessions later**, and the correction reached the paragraph below, §10.7.4's ledger row and
`doc/todo/11` item 7 — everywhere except the group whose members it is about (ADR 0489). So **when
a round changes what a rasteriser draws, the contradicted groups holding the pages it changed are
part of the diff**, exactly as the ink sweep is.

**Something does point at them now, and its first run found the same claim in a second home.**
`cargo run --release -p conformance --bin overtaken` compares a page-list note's newest cited ADR
against the newest ADR that names one of its own pages, which is *a decision taken after the note
was last revised about a page the note explains* (ADR 0491). It named `CONTRADICTED_TIGHT_CONSENSUS`
at the head of its first rung when 662's sentence was planted back, and at the head of its second
rung it named `CONTRADICTED_ANTIALIASED_EDGES`, which had carried the pre-0476 ssim figures for
nineteen sessions **in the paragraph immediately below the ADR 0476 correction, which said that
paragraph was unaffected**. A correction that scopes itself is a claim, and this one was wrong.

**That tell has an instrument of its own now, and the argument for building one was a count.**
`cargo run --release -p conformance --bin quoted -- <the oracle's log>` compares every figure a
note quotes in the gate's own vocabulary against what the gate prints for that note's pages, and
prints the gate's value under each disagreement (ADR 0495). The round that measured the population
before building it found the earlier estimate had been taken over two tokens of a five-token
vocabulary: `mean`, `worst tile`, `differing` as a percentage in either word order, and `ssim`
under three spellings, which is about a quarter of the tree's page-list notes and over a hundred
figures. **Two rules come with it.** A figure quoted in a note is written to the precision the gate
prints — two decimals for the three that are levels or a percentage, four for the similarity —
because a figure written finer is another instrument's and can only be ranked. And a note whose
list is *emptied* keeps its figures and loses its anchor, which is where the archetypes turned out
to live: `CONTRADICTED_UNEXPLAINED`'s list is empty and its four paragraphs about
`issue7891_bc1.pdf` were corrected in the six-hundred-and-sixty-fifth session and stale again by
the six-hundred-and-seventieth.

**And a note has a *fourth* way of being wrong, which the same sweep found by arithmetic.** A doc
comment attaches to whatever declaration follows it, and both of two adjacent declarations can be
page lists — so a note can end up above the wrong one and nothing says so. Forty lines diagnosing
one paper under fifteen names sat above a one-page `DeviceN` group for an unknown number of
sessions, and the tell was that group's note quoting a band `mean 3.51 to 9.93` that none of its
one page carries.

The newest wrong *name* is `colors.pdf` pages 1 and 2, whose `CONTRADICTED_ANTIALIASED_EDGES` had
said since the sixty-eighth session that the five renderers "sit on a spectrum of edge softness" with us
at the soft end. They do not. Each page is sixteen axis-aligned rectangles at known sub-pixel
boundaries, so the page is a closed form; ours was that form with every edge's coverage **rounded to
a quarter** — `tiny-skia` samples four times per axis — to one level of 255 over the whole raster,
and `hayro`'s is the exact form to two. From the geometry at the worst pixel: `hayro` 2, `mupdf` 13,
ours 33, `ghostscript` 54, `poppler` 124 — we were *third of five*, not the soft end of anything.
(Ours is the **exact** form since the six-hundred-and-forty-sixth session, ADR 0476, and the pages
are still contradicted — which is the point of the next sentence rather than a change to it.) The
verdict, separately, is trap 12's: the exact form is contradicted
on both pages too (ADR 0474). The one before it is
`smask_luminosity_oob_transfer.pdf`, whose `CONTRADICTED_MASK_QUANTISATION`
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

**Seven instances now, and each teaches a different edge:**

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
- **A boundary a library picks is a decision neither backend made, and it hides better than a
  convention does.** The first bullet above is about a library's answer *agreeing* with the clause;
  this is about a library's answer *approximating* one the shared crate already states exactly.
  `tiny-skia` chooses between a hairline and a stroked outline by mapping the width along the
  transform's two basis vectors and comparing `fast_len` — `max + min/2` — against 1;
  `pdf_render::thinnest_line` is the linear part's larger singular value. The two agree for every
  **similarity** transform, which is what a page transform is, so no ordinary page can tell them
  apart — and under a shear they part by up to a factor of √2, at which point one backend draws a
  hairline for a mark the other strokes. The tell is that the *tree* had a name for the quantity and
  the library had its own; where that is true, the crate both backends read owns the comparison.
  ADR 0535, `render_cpu::draw_stroked_outline`.
- **Where the clause calls the input an *error*, there is no right answer for a library to agree
  with — and that is when its answer hides best.** §8.5.2.1 makes a path segment issued with no
  current point an error and states no recovery, so a reading of the clause cannot contradict what a
  library does with one. `tiny_skia::PathBuilder` begins the subpath at the **origin of user space**
  and draws an edge from the corner of the page; `kurbo::BezPath` fires `debug_assert!(…,
  "uninitialized subpath (missing MoveTo)")`, so the same document is a **panic in a debug build**
  of the graphics backend. **No picture comparison can see that second one at all** — the disagreement
  is a crash in one configuration rather than a pixel in any — which is a shape the four bullets above
  cannot warn about. The rule is `pdf-model`'s now, where the path is built. ADR 0563,
  `content::path::extend_subpath`.

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

### 12c. A dependency that reports through a *handler* has an ordering you have to obey

`wgpu::Surface::configure` returns `()`. When it fails it says so on the device's uncaptured-error
handler, and this program's handler printed a note and returned — after which quorra acquired from
a surface the device had refused to configure, and **wgpu panicked**: under `panic = "abort"`, a
core dump on the launch path. The project owner met it; the next launch was fine, because the
condition is a submission landing inside the configure's wait for the device to come idle, which
is wgpu's own documented validation error and is transient by construction. ADR 0462.

Three things out of it, and the third is the general one.

**A note is not a decision.** Principle 1 says every error is "typed, propagated, and handled
somewhere deliberate", and a handler that prints has satisfied none of the three verbs. The fix is
to make what the handler was told into a value a host can *take* —
`render_quorra::UncapturedErrors` — and then to decide with it at the call that provoked it.

**Ask which of a dependency's failures are fatal and what makes them so.** wgpu's acquire has two
answers to "this surface is not configured", and it picks between them by a field that is set
*only by a configure that succeeded* (`wgpu-30.0.0/src/backend/wgpu_core.rs:3979-3985` against
`:4023-4037`). With it, a status the caller can act on; without it, `handle_error_fatal`. So the
panic is reachable on the **first** configure of a process and never again — which is a fact you
can only have by reading the dependency, and which turns an unfixable panic into an ordering.

**And then let the type hold the ordering.** The first configure is made to happen where nothing
of this program can be submitting — before the render thread exists — and that is enforced rather
than remembered: `Window::split` hands back an `Ungrounded`, the only route to a `Window` is
`Ungrounded::ground`, and the method that spawns the render thread exists only on a `Window`. A
rule ("present once before you start rendering") is a rule somebody forgets; a constructor is not.

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
