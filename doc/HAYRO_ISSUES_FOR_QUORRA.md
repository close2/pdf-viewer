# What hayro's tracker has that a rasteriser should read

Written 2026-08-16 from **this** side, after reading all 167 issues — open and closed — on
`LaurenzV/hayro`, the pure-Rust PDF renderer that is this project's fourth oracle reading. The
selection below is the subset that touches what quorra owns: coverage, blending, clipping, glyph
rendering, colour on a device, and the boundary between a renderer and its host.

## What this document is, and what it is not

**It is a reading list.** Each entry says what the issue is actually about, why it might matter to
a renderer of quorra's shape, and — where this tree can say — what ISO 32000-2 requires and what
this tree already does. It is meant to be skimmed and to have three or four entries opened.

**It is not a defect list.** Nothing here is a claim that quorra has any of these problems. Most
of them are hayro's alone, several are `vello_cpu`'s rather than hayro's, and a few turned out on
inspection not to be defects at all. Where this tree checked the same question against its own
code the answer is given, and in every case checked it was already right — which is the point of
including it, because *why* it is right is the reusable part.

**It is not a claim that hayro is right.** This project's rule (`CLAUDE.md` principle 5) is that
another implementation is evidence about a reading of the specification and never the definition
of correct. Every clause quoted below is quoted from ISO 32000-2, not paraphrased from an issue.
Where hayro and this tree disagree about a page, that is a question for the standard.

**A note on the tracker itself.** Roughly ninety of the 167 issues are automated fuzzer reports —
one file, one panic, one backtrace — mostly from `qarmin/Automated-Fuzzer`, and most of them
closed within a day. They are omitted here except where the panic is in code quorra would
recognise. That population is worth knowing about for its own sake: it is what a Rust PDF stack
looks like when somebody points a fuzzer at it continuously, and the panics land in the same six
places every time — arithmetic overflow on a subtraction, an index into an empty buffer, an
`unwrap` on a dictionary lookup, a zero-size allocation, an unbounded recursion, and a raster
whose dimensions disagree with its buffer.

---

## 1. The rasteriser panics, which are `vello_cpu`'s rather than hayro's

hayro rasterises through `vello_cpu`'s sparse-strips path, so it is a large continuous fuzz run
pointed at the same sparse-strips design quorra shares the lineage of. Four are open at the time
of writing, and they are the ones worth reading first because the input in each case is *geometry
a document is allowed to state*.

### [#717 — "Max. number of lines per path exceeded. Max is 67108864, got 197470408"](https://github.com/LaurenzV/hayro/issues/717) (open)

A page whose flattened path has 197 million line segments trips a hard assertion in
`vello_common/src/tile.rs`. The number is not adversarial in kind — it is what a curve-heavy path
flattens to at a high enough scale — so the interesting question is not the constant but the
*shape of the refusal*: an assertion inside the tiler is a panic in the middle of a frame, where
what a viewer needs is a budget refusal it can report and fall back from.

**Why it matters to quorra.** This tree's whole GPU story rests on a refusal being a *value* — the
coverage or budget refusal that sends a frame to the processor and is reported out loud
(`CLAUDE.md` principle 2). A panic cannot do that job. If quorra has an equivalent ceiling
anywhere in strip generation, the thing to check is not whether it can be raised but whether
crossing it returns rather than aborts.

### [#373 — "range end index 8 out of range for slice of length 6" in `flatten_simd.rs`](https://github.com/LaurenzV/hayro/issues/373) (open)

A SIMD flattening path reads past the end of its own scratch buffer. Reported upstream as
[linebender/vello#1277](https://github.com/linebender/vello/issues/1277).

**Why it matters.** It is the failure mode of a lane-width-rounded buffer whose tail is not
padded, and it is invisible to every test whose path length happens to be a multiple of the lane
width. Worth a look at any place quorra rounds a segment count up to a vector width.

### [#646 — "attempt to add with overflow" in `vello_cpu/src/fine/mod.rs`](https://github.com/LaurenzV/hayro/issues/646) (open)

An overflow in the fine-rasterisation stage. Debug-only as a panic; in release it wraps, which is
the worse half — a wrapped index into a scanline is a wrong pixel rather than a stopped frame.

### [#351, #352, #357 — pixmap dimensions disagreeing with the buffer](https://github.com/LaurenzV/hayro/issues/351) (closed)

Three separate files reaching `Expected data to have length of exactly width * height` and
`index out of bounds: the len is 0 but the index is 0` in `vello_common/src/pixmap.rs`. All from
degenerate page geometry — a `/MediaBox` that rounds to zero pixels in one axis, most often.

**Why it matters.** A zero-width surface is a legal thing for a document to ask for, and the
question every renderer answers eventually is whether the zero is caught at the top (where a
frame can be skipped) or at the bottom (where a `Vec` is indexed). This tree catches it at the
top; the three issues are what catching it at the bottom looks like.

### [#40, #8, #63 — a clipping panic and a strip multiplication overflow](https://github.com/LaurenzV/hayro/issues/40) (closed)

`#8` and `#40` are the same defect reached two ways — a panic in `hayro-render/src/fine/mod.rs`
when a page is rendered at scale 2.0 but not at 1.0. Scale-dependent crashes are worth their own
mention: they mean a quantity is being computed in device space and assumed to be in a range only
the default scale guarantees. `#63` is the same family in `strip.rs`, an `attempt to multiply with
overflow` on a strip index.

**Why it matters to quorra specifically.** The zoom lane is exactly where this tree exercises
quorra hardest — `PDFVIEWER_QUORRA_COVERAGE=gpu` at 4× is a separate gate here for that reason —
and a defect that only appears above 1× is one that a test suite rendering everything at 1× cannot
see.

---

## 2. Conflation, and thin marks

### [#104 — "Lines rendered to thick"](https://github.com/LaurenzV/hayro/issues/104) (closed)

A Christmas-tree line drawing whose strokes come out visibly heavier than mupdf's. Closed with
"I'm sure those are just conflation artifacts, so not a lot we can do here I'm afraid."

**This is the most interesting entry in the document, and the reason is the closing comment.**
Conflation — two abutting marks each covering part of a pixel and compositing to less than full
coverage, leaving a seam; or two overlapping marks compositing to more ink than either states —
is treated there as a fact of life. This tree does not treat it that way, and has spent several
rounds proving it does not have to. ISO 32000-2 §10.7.4 states the rule a device pixel is decided
by, and a renderer that applies it gets the thin-mark cases right rather than approximately right:

> ...a conforming reader may need to make a determination about whether the pixel is painted
> or not, and, if painted, what its colour value shall be.

The reading and the measurements are in `doc/todo/11-shapes-that-still-disappear.md` and
`doc/todo/_scan-conversion.md`; the code is `pdf-render`'s `sub_pixel.rs`, which detects a mark
thinner than a device pixel and substitutes a band of exactly one pixel at proportional coverage
rather than letting the rasteriser's antialiasing decide. The general abutting-marks case is
harder and this tree has *not* solved it; what it has done is establish that a substantial part of
what looks like unavoidable conflation is a scan-conversion decision with a clause behind it.

**Why it matters to quorra.** If a hairline or a thin rule ever comes out heavier or lighter on
the device lane than on the processor lane, this is the vocabulary for the conversation. See also
`doc/QUORRA_HAIRLINE_MARKS.md`, which is this tree's standing ask on the subject.

### [#1023 — "Strokes differ in hayro and macOS Preview"](https://github.com/LaurenzV/hayro/issues/1023) (closed)

A stroke-rendering regression bisected to a single commit (`cbba8ac3`). Two screenshots, and the
difference is in stroke *weight* and joint appearance rather than in geometry.

**Worth reading for the method rather than the defect**: the reporter bisected a rendering
difference to a commit, which is the thing a renderer's test suite should be able to do for itself
and usually cannot.

---

## 3. Shadings, and where a renderer stops being able to help

### [#3 — "Improve rendering quality of Type6 and Type7 shadings"](https://github.com/LaurenzV/hayro/issues/3) (closed)

Coons and tensor-product patch meshes (ISO 32000-2 §8.7.4.5.7 and §8.7.4.5.8) approximated by
triangles at a **fixed** grid size, with two named consequences: the tessellation does not adapt
to the resolution being rendered to, and the triangles conflate at their shared edges — the same
seam as §2 above, one patch over.

**Why it matters to quorra.** Mesh shadings are the one part of PDF that no gradient primitive
expresses, and every renderer ends up tessellating them. The two questions this issue raises are
both quorra's: what does the tessellation density depend on, and do adjacent triangles seam? The
issue's own suggested fix — "inflating the triangles by a small factor" — is the conflation
workaround, and it is worth knowing that hayro reached for it.

### [#551 — "Controlling rasterization of PDF shaders in hayro-svg"](https://github.com/LaurenzV/hayro/issues/551) (open)

A shading that cannot be expressed in SVG is rasterised, and the rasterisation is at a fixed low
resolution regardless of the output size.

**Why it matters.** The general shape is "a paint the target cannot express is baked at some
resolution, and nothing tells the baker what resolution to use". This tree has the same problem in
a different place and has an open ask about it: `doc/QUORRA_FUNCTION_PAINT.md` proposes moving a
function-based shading's *evaluation* onto the device rather than baking a grid, precisely so that
a zoom step does not re-bake. #551 is the same question with the answer "bake, but at the right
size"; the ask in that document is "do not bake".

### [#41, #102, #968 — gradients: missing, wrongly placed on text, wrongly clipped](https://github.com/LaurenzV/hayro/issues/41) (closed)

Three unrelated gradient defects. #41 is a gradient that fails to process at all (the reduced case
is an Inkscape-produced linear gradient with a `stop-opacity` ramp, i.e. a shading with a
transparency component — which became #44, soft masks). #102 is a gradient used as the fill of
*text*, which is the pattern-as-text-paint case. #968 is a gradient clipped incorrectly in the SVG
back end, with a curve-stroke and a border-stroke witness.

**Why the third matters to quorra.** "Gradient clipped incorrectly on a stroke" is the shape where
a paint's coordinate space and the stroke outline's coordinate space are composed in the wrong
order. §8.7.4.3 puts a shading pattern's coordinates in the space of the page at the time the
pattern's parent content stream began, *not* the space in force when the paint is used — which is
the rule the composition has to respect and the one that is easy to get wrong by one matrix.

### [#394 — "Improve rendering of certain patterns"](https://github.com/LaurenzV/hayro/issues/394) (open)

One line and a test-case name (`pdftc_100k_0138`). Included only because the label is
`rendering-quality` and the population behind it may be larger than one file.

---

## 4. Images: masks, downsampling, and the cost of a stencil

### [#1315 — a stencil-masked image renders ~5× slower than the same image without its mask](https://github.com/LaurenzV/hayro/issues/1315) (closed)

Careful, well-measured report. A real scanned page draws two full-page XObjects; one of them is a
450×588 JPEG whose `/Mask` is a **1350×1763 JBIG2 image mask** — a stencil whose dimensions differ
from the base image's. hayro: 36 ms for the page, 12.8 ms with the `/Mask` entry removed. MuPDF:
17 ms and 12.6 ms. So the one masked image costs 23 ms against MuPDF's 4.4 ms, and without it the
two renderers are level.

### [#1319 — packed 1-bit mask decoding via per-sample `BitReader` + f32 interpolation](https://github.com/LaurenzV/hayro/issues/1319) (closed)

The follow-up, and it contains a correction worth reading: the reporter's first attribution was
wrong, and they said so. The general path unpacks every sample individually through a bit reader
into a `Vec<u16>` and then runs each through an f32 interpolate — for a mask, where the whole
operation is `bit -> 0 or 255`. Measured on a synthetic 2400×3150 Flate-compressed stencil:
48–60 ms down to 1.5–1.6 ms with a byte-wide lookup-table expansion, output pixel-identical.

**Why both matter to quorra.** A stencil mask at a resolution different from the image it masks is
the ordinary case rather than the exotic one — §8.9.6.4 explicitly allows it, and scanners produce
it constantly. Two things are being measured here: the cost of *decoding* a packed mask, and the
cost of *drawing* through one whose grid does not match. The second is the one that is quorra's,
and 30× is the size of the prize on the first.

### [#2 — "Downsample image masks with larger dimensions than original image"](https://github.com/LaurenzV/hayro/issues/2) (closed)

Nearest-neighbour resampling of a mask that is larger than the image it masks — the same
mismatched-grid case as #1315, from the quality side rather than the speed side.

### [#1310 — override `ImageQuality`/`Interpolate` globally](https://github.com/LaurenzV/hayro/issues/1310) (open)

A user found that small chart marks — a few pixels across — came out noticeably worse than Cairo's
and that rendering at 2× and downscaling fixed it. Their workaround was to force image filtering
on regardless of what the PDF says.

**Why it matters, and the caution.** ISO 32000-2 §8.9.5.1 Table 87 makes `/Interpolate` a request
from the document — "( Optional ) A flag indicating whether image interpolation shall be performed
by a conforming reader" — so overriding it is a viewer preference and not a correctness fix. The
maintainer's reply is the right instinct and worth quoting: Chrome and mupdf also filter here even
though they usually respect the flag, "so i think I will first look into whether there is a bug in
our downsampling logic". A quality complaint about small marks is very often a scan-conversion
defect wearing a filtering costume — cf. §2 above.

### [#494 — an image whose declared size is not its codestream's](https://github.com/LaurenzV/hayro/issues/494) (closed)

`/Width 3024 /Height 4032` on a `DCTDecode` image whose embedded JPEG is 930×1240.

**What ISO 32000-2 says, and what this tree does.** §7.4.8 puts a JPEG's dimensions in the data —
"The values of these parameters ... shall be stored in the encoded data. DCTDecode may obtain the
parameter values it requires directly from the encoded data" — and §8.9.5.1 puts every image in
the same place whatever its resolution: "the unit square of user space, bounded by user
coordinates (0, 0) and (1, 1), corresponds to the boundary of the image in image space". So the
two disagreeing costs nothing: this tree builds the raster on the *codestream's* grid, maps it to
the unit square, draws it undistorted, and reports the disagreement. It used to refuse instead, and
that refusal cost a whole photograph over a one-row discrepancy.

**The contrast is the useful part**: §7.4.9 states the constraint §7.4.8 does not — "Width and
Height shall match the corresponding width and height values in the JPEG 2000 data" — so for
`JPXDecode` this tree *does* refuse. Two adjacent filters, two different answers, both from the
clause.

---

## 5. Glyphs

### [#23 — "Text with bitmap glyphs renders very ugly"](https://github.com/LaurenzV/hayro/issues/23) (closed)

Embedded bitmap glyph strikes (`EBDT`/`EBLC`, or `CBDT`) drawn without regard for the fact that a
bitmap strike has one correct size and every other size is a resample.

### [#296 — "Glyph paths start with extra element"](https://github.com/LaurenzV/hayro/issues/296) (closed)

Every glyph outline came back beginning with a spurious `MoveTo((0,0))` before its real one. It
turned out to be deliberate, and the reporter closed their own issue — but the maintainer's reply
is the part to read: "Perhaps this is not ideal, I'll see whether I can solve it differently."

**Why it matters.** A degenerate leading `MoveTo` is invisible to a fill and *not* invisible to a
stroke: under a round or square line cap it can deposit a dot at the origin (§8.4.3.3's caps are
applied to the ends of every open subpath, and a subpath of one point is an open subpath). This
tree has a whole ADR family about exactly that mark — a single-point closed path under round caps
is a disc, and the test `a_single_point_closed_path_is_a_disc_under_round_caps` pins it. If quorra
ever receives an outline with a leading degenerate `MoveTo` from any producer, the question is
whether it deposits a cap.

### [#6 — "Improve font fallback"](https://github.com/LaurenzV/hayro/issues/6) (open)

Falls back to Times New Roman for every non-embedded non-standard font. Not quorra's layer, but
listed because the *shape* of the ask — "there should be some kind of callback so that users can
supply their own fonts with custom logic" — is a host-boundary question, and host boundaries are
where this tree and quorra spend most of their design conversation.

---

## 6. Colour on a device

### [#4 — `/All` and `/None` colorants](https://github.com/LaurenzV/hayro/issues/4) (closed)

The two special colourant names in a Separation or DeviceN space.

**What the standard says, and what this tree does.** §8.6.6.4 is unusually explicit: "The special
colourant name None shall not produce any visible output. Painting operations in a Separation
space with this colourant name shall have no effect on the current page", and for `All`, "When
outputting to an additive device, such as a computer monitor, the subtractive tint values of the
All colourant shall be complemented by subtracting from 1 before applying to all available
colourants". Both are implemented here as colour-space variants decided *before* the alternate
space and tint transform are read — which is what the clause's third sentence requires, since it
says a processor "shall ignore the alternateSpace and tintTransform parameters" for these two
"although valid values shall still be provided". The `/None` case reached this tree as a real
regression: before it was implemented the tint transform ran and the page came out red.

**Why it matters to quorra.** `/None` is a paint that must composite as though it were never
issued — not as transparent black, not as alpha zero over an unchanged backdrop by accident, but
genuinely no operation. It is the cheapest possible test of whether a "no ink" path in a
compositor is really a no-op.

### [#205, #235, #355, #390 — the ICC engine panicking on a document's profile](https://github.com/LaurenzV/hayro/issues/205) (closed)

Four fuzzer files reaching `assertion failed: transform.transform_fn.is_some()` and `unexpected
colorspace` in `qcms`, and two slice overruns in `moxcms` (reported upstream as
[moxcms#106](https://github.com/awxkee/moxcms/issues/106) and
[#111](https://github.com/awxkee/moxcms/issues/111)).

**Why it matters.** An `ICCBased` colour space carries an arbitrary attacker-supplied profile
(§8.6.5.5), and it is evaluated on the rendering path. This is the one place in a renderer where
untrusted *data* is fed to a parser that most projects treat as infrastructure. Worth knowing which
CMS quorra reaches and what it does with a malformed profile.

### [#630 — "Note about FMA"](https://github.com/LaurenzV/hayro/issues/630) (closed)

The best short read in the tracker. A JPEG 2000 inverse colour transform calls `f32::mul_add`
without any runtime feature detection, so on a target built without FMA — a universal binary on
x86-64 being the case in point — it falls through to the libc software emulation, which is very
slow and does not dispatch. `floor` has the same problem. Godbolt links, glibc and llvm-libc
sources, and a mantissa-shifting replacement for `floor` in the comments.

**Why it matters to quorra.** `mul_add` is the natural way to write a colour transform or a
gradient step, it is a single instruction where the target has it, and it is a function call into
software emulation where the target does not. The maintainer's own comment on how it got there —
translated from `fearless_simd` code by an LLM, tests still passing — is worth reading too.

### [#60 — "Does this/will this support `u16`/`f16`/`f32` channel formats?"](https://github.com/LaurenzV/hayro/issues/60) (closed)

A dithering question underneath a format question: the reporter wants to know whether colour is
processed at higher than 8 bits per channel and dithered on the way down. The maintainer's answer
is candid — "my main priority is definitely speed as opposed to 100% correctness, which implies
doing most of the rasterization in u8/u16" — and the thread goes on to why hayro has a custom
rasteriser at all: PDF's mesh shadings are not expressible in a general vector renderer.

**Why it matters.** Banding in a gradient is an 8-bit compositing artefact and it is one of the few
rendering defects a user reports unprompted. This tree has its own version of the question in
`doc/todo/11`'s item 5 — what an 8-bit raster does to a mark whose ink is under one of its levels.

---

## 7. The renderer/host boundary — four asks quorra will recognise

These are the issues where somebody wants to embed hayro in something, and the design question
they raise is one quorra has already answered or will be asked.

### [#821 — "Vello Scene backend to append vector PDF rendering into an existing `vello::Scene`"](https://github.com/LaurenzV/hayro/issues/821) (open)

Somebody with a Vello-based application wants PDF content composited into their scene as *drawing
commands* rather than as a page-sized bitmap — so that a page can be placed by one affine, stay
resolution-independent, and sit under their own overlays.

**Read the maintainer's reason for hesitating**: "there is also the problem that vello doesn't
support everything that is needed for correct rendering AFAIK (for example masks), so this could
lead to inaccurate rendering in certain cases." That is the same trade this tree makes explicitly
and in the opposite direction — quorra draws page one, and where it refuses a construction the
processor draws that frame and the refusal is reported. The thread's other half is a pointer to
the sparse-strips roadmap and the observation that Vello is moving toward stateful rendering
across CPU and hybrid, which is the direction quorra's API already went.

### [#1316 — expose `vello_cpu` multithreading (`num_threads`) through `RenderSettings`](https://github.com/LaurenzV/hayro/issues/1316) (open)

`hayro::render` hardcodes `num_threads: 0` and re-forces it for nested contexts, so an embedder
cannot opt into multithreaded rendering at all; for large raster targets single-threaded fills
dominate.

**Why it matters.** This is the thread-count question from the other side of the boundary, and this
tree has asked quorra its own version — `doc/QUORRA_ENCODE_THREADS.md` and its answer. Two
independent embedders arriving at "let me choose the thread count, and let a nested context
inherit it" is a signal about where the knob belongs.

### [#1052 — cooperative cancellation](https://github.com/LaurenzV/hayro/issues/1052) (open)

The single most relevant issue in the tracker for an interactive viewer, and the thread is a
genuine design argument. `render()` blocks until the page is fully rasterised, which can be
seconds; a viewer whose user has navigated away has no way to stop it. The proposal is a `Stop`
trait polled in hot loops.

The maintainer's objection is the interesting part: cancellation is only as good as its worst
uncancellable leaf, and image decoding is that leaf — "what if you have a huge image that takes
ages to decode? ... Wouldn't the proper solution here to render the image on a separate thread and
abort it in case it takes too long?" The reporter's answer is that thread termination is neither
reliable nor safe inside a server process, and they then went and filed cooperative-cancellation
support against `miniz_oxide` and `zune-jpeg` so that the leaves would be cancellable too.

**Why it matters to quorra.** This tree solved the same problem structurally rather than
cooperatively — the expensive, untrusted leaf (JBIG2, JPEG 2000, CCITT) is a *separate confined
process*, so cancelling it is killing it, which is reliable and safe by construction and measured
here at 0.83–1.97 ms. That is worth putting beside the thread: process isolation buys cancellation
as a side effect of buying containment. It does not solve cancelling the *rasteriser*, which is
still the frame-level question quorra's retained-frame and non-blocking-render work is about
(`doc/QUORRA_NONBLOCKING_RENDER.md`, `doc/QUORRA_RETAINED_FRAME.md`).

### [#1345 — "Reading only relevant parts of the PDF file"](https://github.com/LaurenzV/hayro/issues/1345) and [#1343 — concurrent object resolution](https://github.com/LaurenzV/hayro/issues/1343) (both open)

Not rasterisation, but the two requests that most shape what a renderer's input looks like:
somebody wants the whole file not to be resident, and somebody else has found that resolving
objects from several threads on one shared document silently yields nulls and occasionally panics
— found "while integrating hayro-syntax into a commercial CAD application under concurrent page
interpretation". #1343 is a careful report with three distinct races named and located.

**Why it matters.** Parallel page interpretation over one immutable document is a thing every
serious embedder eventually wants, and #1343 is a catalogue of what breaks when the caching layer
under an "immutable" document is not itself linearisable.

---

## 8. Two things that are not defects and are worth reading anyway

### [#1195 — "Possible optimizations"](https://github.com/LaurenzV/hayro/issues/1195) (closed)

Somebody profiling hayro against `pdftocairo` and pdfium, with real numbers, real profiles, and a
correction: the first profile was taken with a very high scale instead of a high repeat count and
showed most of the time in `memset`/`memcpy`; the corrected run put 65% in `interpret_page`. The
thread ends with the maintainer applying three of the suggestions and reaching parity with pdfium
on the test file (2.729 ms against 2.681 ms).

**Two things to take from it.** First, the corrected profile's conclusion — for an ordinary page,
*interpretation* dominates and rasterisation does not — is the same finding `CLAUDE.md` states as
a standing rule here ("parsing, xref resolution, and font loading usually dominate
time-to-first-page — not rasterization"), arrived at independently. Second, the initial
misattribution to the renderer is the failure mode: a benchmark run at the wrong scale points at
whatever scales with area.

### [#1188 — "Claude Code Review Report"](https://github.com/LaurenzV/hayro/issues/1188) (open)

A whole-repository LLM review posted as an issue, with the reporter's own honest framing: a
significant portion of previous runs' findings were wrong or trivial, a non-trivial subset were
real, and "the findings generally require manual validation to separate noise from actionable
problems". The two examples given are a one-character typo producing silently wrong CIE Lab output
and a dead conditional in AES padding.

**Why it is here.** This tree independently confirmed the first of the two — the typo is real, it
is in the published `hayro-jpeg2000` 0.3.5 and 0.4.0 on crates.io, and it is already fixed on
hayro's `main`. That is a data point about what such a review is worth: one in a long list, real,
and it took reading the code to know which one.

---

## Where this tree's own side of the conversation lives

`doc/QUORRA_FEEDBACK.md` is the standing document. The design asks are
`doc/QUORRA_FUNCTION_PAINT.md` (a device-evaluated paint for §8.7.4.5.2's function-based shading),
`doc/QUORRA_HAIRLINE_MARKS.md`, `doc/QUORRA_NON_ISOLATED_GROUPS.md`,
`doc/QUORRA_NONBLOCKING_RENDER.md`, `doc/QUORRA_RETAINED_FRAME.md` and
`doc/QUORRA_ENCODE_THREADS.md`. `doc/quorra-gpu-coverage.md` is what the device lane currently
refuses and why.

The reading of the other 130-odd issues — the ones that are this tree's business rather than
quorra's — is `doc/HAYRO_ISSUES.md`.
