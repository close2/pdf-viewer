# Handover

Written 2026-07-26, updated 2026-07-27 at the end of the fourth working session. Read
`/CLAUDE.md` first — it holds the five non-negotiable principles and they are not optional.
**Principle 5 is the one that changes how to work**: the specification is the only source of
truth, and agreement with poppler, mupdf or pdf.js is evidence that we read it right, never
the definition of right. This file is the state of play, the traps, and what to do next.

## What the fourth session changed, in one paragraph

Chasing the performance defect at the top of the previous handover found two correctness
defects underneath it. Both rasterisers were positioning a paint in the *device's*
coordinate space when both underlying libraries read it in the *path's*, so the device
transform was applied twice: every gradient on every page was mirrored about the page's
horizontal centre line, and every image was sampled through a doubled transform — one
photograph came out as a single flat rectangle. Neither showed up in any metric, and the
CPU-versus-GPU comparison could not see them because both backends were wrong in the same
way and every scene comparing them used gradients that do not vary in y. They are fixed,
with tests derived from the clause rather than from a renderer. The performance defect is
fixed too: the worst page in the corpus went from 48.7 s to 0.24 s.

## Where we are

A PDF **renderer** that opens real files and draws pages: geometry, colour, images,
shadings, patterns and embedded text, on both a CPU and a GPU backend. It is not yet a PDF
*viewer* in the full sense — no annotations, forms, encryption or transparency groups — and
the gap between those two words is measured further down rather than guessed at.

- **187 tests**, `clippy` clean under `pedantic` + `unwrap_used`/`panic`/`arithmetic_side_effects`,
  `cargo fmt --check` clean, `cargo deny` clean on all four checks (verified, not assumed).
- **The 14 specification PDFs in `doc/`** — including ISO 32000-2 itself, 1023 pages and
  101 318 objects — all parse, all render page one with only a soft mask reported on three
  of them, and all extract **100% of the words `pdftotext` finds**.
- **The 974-document pdf.js corpus is a gate, not a survey.** All 974 open, 955 reach page
  one, **674 draw with nothing reported at all**, and everything the other 281 cannot draw
  is named. The counts are ratcheted and can only go down. 1501 of 1501 PDF functions
  parse; **all 1793 shadings build**, mesh types included. The whole gate runs in **15 s**
  and has **no named slow document left**.
- Our render of the `basic` fixture agrees with poppler, mupdf and ghostscript, and is
  byte-identical to mupdf — corroboration that we read the specification right, not a
  target (principle 5).
- **Colour resolves from the document.** `ICCBased` profiles are evaluated by an A2B
  evaluator written here, `/DefaultCMYK` and output intents are honoured, and there is
  exactly one `DeviceCMYK` conversion instead of the three that used to disagree. ADR 0009.
- Both backends draw everything the display list can express, and agree on it: **eight**
  headless GPU scenes hold `tiny-skia` and Vello to the same pixels. Two of those eight are
  new, and exist because the other six could not have caught the paint-space defect in
  trap 2 — they all ran at one scale, along one axis.

### Run it

```sh
cargo run --release -p viewer-ui --bin pdf-viewer -- doc/PDF20_AN001-BPC.pdf
```

Arrow keys / Page Up / Down / Space turn pages, Home and End jump to the ends, Escape
quits. The title bar names anything on the page that could not be drawn.

### Verify it

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets     # must be silent
cargo test --workspace
cargo test --release -p pdf-model --test corpus -- --ignored --nocapture   # 974 docs, ~15 s
cargo bench -p pdf-model                   # interpretation, the time-to-first-page path
cargo deny check
cargo +nightly fuzz run lexer -- -runs=50000     # from fuzz/, needs nightly
```

## Crate map

| Crate | Does | Notes |
|---|---|---|
| `pdf-spec` | Object-model validation tables | Generated from Arlington by `build.rs` |
| `pdf-syntax` | Lexer, objects, xref, filters, `Document` | Touches untrusted bytes first |
| `pdf-model` | Page tree, content interpreter, image decode | Where PDF semantics live |
| `pdf-font` | Glyph outlines via `skrifa` | `cff.rs` adapts `read-fonts`; `encoding.rs` is Annex D data; `substitute.rs` is the only machine-dependent code in the tree |
| `pdf-render` | Display list + `Rasterizer` trait | No PDF semantics, no rasteriser |
| `render-cpu` | `tiny-skia` backend | Correctness oracle **and** startup path |
| `render-gpu` | Vello/wgpu backend | Headless by construction |
| `raster-compare` | Tolerant image metrics | Worst-tile error is the load-bearing one |
| `test-scenes` | Shared fixtures | Holds the same page as a display list *and* as PDF bytes |
| `tools/pdfref` | Reference-comparison harness | Triangulation rule lives here |
| `viewer-ui` | The application | `src/bin/pdf-viewer.rs` |
| `viewer-core`, `pdf-sandbox` | Empty | Documented responsibility only |

Architecture decisions are in `doc/adr/`. `doc/PLAN.md` tracks phases and measured results.

## Traps — read these before writing code

### 1. The metrics lie about fonts. Look at the page.

This is the most important thing in this file.

Wiring bare-CFF support in made every affected document report `unsupported: []` — and
render **almost no text**. The font loaded, nothing was reported, the wrong glyphs were
drawn. It was caught only by rendering a page and looking at it.

`Interpretation::is_complete()` tells you what the interpreter *knows* it skipped. It
cannot tell you that a font loaded and produced garbage. For any font or colour work,
render the corpus pages and **look at them**. There is a test that writes the PNGs:
`cargo test -p pdf-model --test render_real_pdf -- --nocapture writes_inspectable`.
It covers both CFF routes, because no metric distinguishes them.

Since that warning was written, two automated checks have been added that *do* catch a
wrong mapping, both in `crates/pdf-font/src/lib.rs`:

- `the_pdf_widths_agree_with_the_font_programs_own_advances` — the document's `/Widths`
  and the CFF charstring's own advance are independent statements of the same fact, so
  they agree only if the code reached the glyph the producer meant. This is the strongest
  check in the tree: it verifies the mapping without consulting the mapping.
- `an_uncovered_code_has_no_glyph_rather_than_a_guessed_one` — pins the absence of the
  code-as-glyph-index fall-through.

Both were confirmed to fail when the defects they describe are deliberately reintroduced.
They are complementary: an off-by-one charset trips only the first, a reinstated
fall-through only the second. Neither replaces looking at the page.

### 2. A paint is positioned in the *path's* space, not the device's

Both `tiny-skia` and Vello apply the drawing transform to a paint as well as to the shape:
`Pixmap::fill_path` and `stroke_path` post-concatenate it onto the shader, and Vello
encodes a brush transform as `shape transform * brush transform`. So the transform you hand
a gradient, a pattern or an image is read **in the space the path is stated in**, and
composing the page-to-device transform into it yourself applies that transform twice.

Both backends did exactly that, and it shipped:

- Every gradient was **mirrored about the page's horizontal centre line**. At a scale of
  1.0 the page-to-device transform is a y-flip about the page centre and so is its own
  inverse: the second application cancels the geometry and leaves the flip. At any other
  scale it leaves a scale-dependent displacement instead.
- Every image was sampled through a doubled transform. `issue19971.pdf` draws a 2500×1364
  photograph and we drew one flat dark-blue rectangle.

Three things about how this survived are worth carrying forward:

1. **No metric saw it.** `unsupported: []`, the right shape, colours from the right ramp.
   Trap 1's warning about fonts is the same warning: look at the page.
2. **The CPU-versus-GPU comparison could not see it**, because both backends had it and
   therefore agreed. Two implementations agreeing is evidence *only where they can fail
   independently*.
3. **Every scene that compared them used a gradient running along x**, where a y mirror is
   invisible. A test that cannot fail in the axis the defect moves is not a test of it.

The guards now are `render-cpu/tests/shading_placement.rs` and `image_placement.rs`, which
pin values against ISO 32000-2 §8.7.4.5.3 and §8.9.5.2 **at three scales**, and
`headless_gpu.rs`'s vertical-gradient and image scenes. One scale cannot see this class of
defect; that is why every case runs at more than one. All were confirmed to fail when the
defects are reintroduced.

### 3. Test against real documents, not hand-written fragments

Cross-reference streams are compressed *and* PNG-predicted. The code said decoding them
was "the caller's responsibility" and then did not do it, so every modern PDF failed with
a misleading `/Root is not a dictionary`. Unit tests on fragments would never have caught
it; the corpus caught it on the first run.

`crates/pdf-syntax/tests/real_documents.rs` and
`crates/pdf-model/tests/render_real_pdf.rs` run over everything in `doc/`. Keep them
passing.

### 4. Unsupported input must stay loud

Every layer reports what it could not handle rather than skipping it: `Unsupported` in the
interpreter, `FontError`, `ImageError`, `CpuRasterError::UnsupportedCommand`. This is not
politeness — it is what makes the comparison harness trustworthy and what caught trap 1.
Do not "helpfully" fall back to a default that renders something plausible.

### 5. Colour: one conversion, and the specification often does not have an answer

Three separate `DeviceCMYK` → RGB conversions used to live in this tree and they disagreed.
`0.5 0 0 0.5 k` gave a red channel of 0.25; the same colour through `scn` gave 0.0; a CMYK
image gave a third answer. Nothing about a rendered page reveals that — each looks like a
plausible colour. `crates/pdf-model/tests/colour_paths.rs` now drives one value through all
three routes and demands they agree; it was verified to fail when the old code is restored.

Add no fourth path. `ColourSpace::to_rgb` is the only place a colour becomes RGB.

The other half is harder to hold onto: **ISO 32000-2 defines no `DeviceCMYK` conversion at
all**. §8.6.4.4 says "concentrations of process colourants" and stops; §8.6.5.7 NOTE 3 says
nothing in PDF describes the device. What the specification *does* say is where to ask —
`/DefaultCMYK` (§8.6.5.6, normative), an output intent's `/DestOutputProfile` (§14.11.5),
and an `ICCBased` profile — and all three are implemented and all three outrank the
fallback table. When you touch that table, do not reach for what another renderer produces:
read ADR 0009, and if you change it, change it as a documented choice.

### 6. `#[expect]`, never `#[allow]`

Every lint exception in the tree is `#[expect(..., reason = "...")]`. It errors when it
stops being necessary, which has already removed several stale ones. A bare `allow` hides
that forever.

## Environment

The agent runs as user `AI` via `sudo -u AI`, reaching `/home/cl/projects/pdf-viewer`
through the `coders` group. This causes recurring friction:

- **Launch with a login shell** so `umask 002` applies, or every file the agent creates is
  unwritable by `cl`:
  `sudo -u AI bash -lc 'cd /home/cl/projects/pdf-viewer && claude'`
- **`AI` has no X authority cookie.** Anything needing a window fails at
  `XOpenDisplayFailed`. The GPU backend is headless by construction precisely so it can
  still be tested; the viewer binary cannot be run by the agent past event-loop creation.
- **Build directory**: `AI` builds into `/home/AI/cargo-target/pdf-viewer` via
  `~/.cargo/config.toml`, so the two users never fight over `target/`. Do not "fix" this
  by sharing it again.
- **`pdfref` needs `--work-dir`** for the same reason; its default is `./target/pdfref`.
- **`cargo-fuzz` needs `+nightly`** explicitly, because `rust-toolchain.toml` pins stable
  1.97.1. That pin is deliberate.
- The Arlington model is a **submodule** pinned at `ba7d4d61`; `pdf-spec` will not build
  without `git submodule update --init`.

## What is not implemented

Every one of these is *reported* at runtime rather than silently skipped — that is what
makes the corpus numbers below trustworthy, and it is principle 3's requirement, not a
nicety. Sized by the corpus rather than by intuition: the count is how many of the 974
documents' first pages it affects.

| Missing | Corpus | Size | Notes |
|---|---|---|---|
| JBIG2, JPX | 152 | — | **Blocked on the sandbox, deliberately.** Both are historically severe attack surface; see principle 3. |
| Text: CID encodings, embedded `CMap`s | 73 | Medium | The breakdown from the gate's own output: 27 fonts with no `/ToUnicode` so a substitute cannot be addressed, 25 with a non-identity `/CIDToGIDMap`, 14 with an embedded `CMap` stream, 3 with a predefined `CMap` (`90ms-RKSJ-H`). Only the last needs vendored data, which is a licensing decision rather than a coding one. |
| Transparency groups, soft masks | 45 | Large | 26 report as `Shading`, 19 as `Operator`. The largest *rendering* gap, and the last thing `doc/` reports. |
| Encryption | 20 | Medium | RC4/AES, `/Encrypt`. 11 documents cannot reach page one at all and 9 more draw a blank page. |
| Optional content (`/OC`) | 5 | Small–medium | 33 documents carry `/OCProperties`, 8 hide something by default, and **5 draw hidden content on page one**. `BDC` is parsed and ignored and `/OC` on an XObject is never read, so a hidden layer **is drawn anyway** and *nothing is reported*. `issue12007_reduced.pdf` draws a whole hidden screenshot over a page the references leave nearly blank. |
| `LZWDecode` | 3 | Small | The one standard filter absent. A test pins the report and will fail when it lands. |
| Type1 fonts (`/FontFile`) | 0 | Medium | No corpus page one reaches it, so this is smaller than it looks. `read_fonts::ps::type1` exists — check before writing any. |
| Type3 fonts | 23 documents carry one | Medium | Needs `d0`/`d1` and `/CharProcs` interpretation. |
| Sampled shadings on the GPU | 2 | Small | Type 1 only; the CPU backend draws them. |
| Rendering intents beyond `AbsoluteColorimetric` | — | Small | Read and recorded; `A2B0` is not yet selected for `Perceptual`. |
| Annotation appearances | 148 | Medium | **148 of 988 first pages carry a visible annotation with an `/AP`, and none is drawn or reported.** An appearance is a form XObject and the interpreter already runs those, so drawing them is §12.5.5 plus a pass over `/Annots` — see "What to do next". 244 first pages carry `/Annots`; the other 96 carry only hidden, `Popup`/`Link`, or appearance-less annotations. Synthesising a missing appearance is a separate and much larger job. |
| Forms, actions, the rest of clause 12 | — | Large | Interactivity: field values, calculation order, JavaScript, navigation. Not needed to *draw* an annotation. |
| Tagged PDF, metadata | — | Large | Clause 14 beyond output intents. |
| Sandbox (Spike D) | — | Medium | seccomp-BPF + Landlock. Blocks JBIG2/JPX. |

## How much of the specification is implemented

Measured, not estimated. Two answers, because the honest one depends on what you are
counting — and the second matters more than the first.

### By what real documents need

Over the 974-document pdf.js corpus, page one:

| | count | share |
|---|---|---|
| opens | 974 | 100% |
| reaches page one | 955 | 98% |
| **draws with nothing reported** | **674** | **69%** |
| draws, with something reported | 281 | 29% |

That 69% is the number to quote. Everything in the 29% is *named* — see the ratchet table
below — and the largest single cause is JBIG2 and JPX, which are deliberately deferred to
the sandbox rather than merely unwritten.

**Read it as "reported nothing", not "drew it right".** This session found two defects that
misdrew a gradient and an image on pages counted in that 674, in silence — that is what
trap 2 is about. Three things are *known* to be missing from a page without being counted
here, because nothing looks for them: **annotation appearances** (148 of the 988 first
pages), **optional content** (5 pages draw a hidden layer), and whatever else the reference
oracle finds when someone points it at the corpus — on a spread sample, 8 of 95 pages we
call complete are contradicted by two independent renderers by more than those renderers
disagree with each other. The 674 measures the honesty of the *reporting*, which is worth a
great deal and is not the same as correctness.

### By clause

ISO 32000-2 has 824 numbered subclauses under its eight technical clauses. Counting them is
a poor proxy — clause 12 is 166 subclauses of annotation subtypes a viewer adds one at a
time, while clause 8's 128 decide whether any page looks right at all — so this is a
judgement about state, not an arithmetic result.

| Clause | Subclauses | State |
|---|---|---|
| 7 Syntax | 138 | **Nearly complete.** Objects, all filters but `LZWDecode`, classic and stream xrefs, object streams, incremental updates, recovery by scanning. **Encryption is absent** and is the largest hole here. |
| 8 Graphics | 128 | **Nearly complete.** Paths, clipping, all eleven colour space families, all seven shading types, both pattern types, form and image XObjects, inline images, ICC colour management. Optional content (`/OC`) is not honoured, so hidden layers draw on 5 corpus first pages. |
| 9 Text | 65 | **Partial.** Simple and composite fonts through embedded TrueType, CFF and OpenType programs; the standard 14 by substitution; `/ToUnicode`. Missing: bare Type1 (`/FontFile`), Type3 fonts, embedded `CMap` streams, predefined `CMap`s. |
| 10 Rendering | 36 | **Partial, and much of it is not applicable.** Colour management and rendering intents are done. Halftones, transfer functions, flatness and smoothness describe a marking device rather than a screen. |
| 11 Transparency | 58 | **Minimal.** All sixteen blend modes are implemented and reach both backends. Transparency groups, soft masks, knockout and isolation are not — this is the largest *rendering* gap. |
| 12 Interactive features | 166 | **None.** No annotations, forms, actions or navigation — and annotation *appearances* are the largest unreported gap in the tree, on 148 of 988 first pages. |
| 13 Multimedia | 81 | **None**, and unlikely to be a priority. |
| 14 Document interchange | 152 | **Output intents only.** No tagged PDF, no metadata, no marked-content semantics — `BDC`/`EMC` are parsed and ignored. |

So: the parts of the standard that decide whether a page is drawn correctly are largely
done; the parts that make a document *interactive* are not started.

### Feature-by-feature, from the source

| | |
|---|---|
| Content-stream operators | **71 of 73** in Table 50 (`ID`/`EI` are consumed inside the `BI` handler rather than as arms). The two genuinely missing are `d0` and `d1`, which exist only inside Type 3 fonts. `BMC`/`BDC`/`EMC`/`MP`/`DP`/`BX`/`EX`/`i` are matched and deliberately ignored — correct for all but `BDC` with `/OC`. |
| Filters | **7 of 8** standard filters decode: `ASCIIHex`, `ASCII85`, `Flate`, `RunLength`, `Crypt` (pass-through), plus `DCTDecode` for images. `LZWDecode` is **absent** (3 corpus documents). `CCITTFax`, `JBIG2` and `JPX` are reported, not decoded. |
| Colour spaces | **11 of 11** families. |
| Function types | **4 of 4** (sampled, exponential, stitching, `PostScript` calculator). |
| Shading types | **7 of 7**, on both backends. |
| Pattern types | **2 of 2** (tiling and shading). |
| Blend modes | **16 of 16**. |
| Font programs | TrueType, CFF, CFF-in-OpenType, CID-keyed CFF. Bare Type1 and Type3 are reported. |

## Done this session, and what it teaches

**The clip-mask defect at the top of the last handover is fixed, but not for the reason
that handover gave.** It said the cost was the page-sized masks, with arithmetic that
matched the observed 1.7 GB exactly. `callgrind` put the masks at under 4% of the run and
the raster pipeline's **gradient stage at 78.9%**: the page is thousands of shading fills,
each covering a large area and each clipped to a sliver, so the paint was evaluated across
the whole path and the mask then discarded almost all of it. Making the mask cheaper alone
would have kept nearly all 48 seconds. The fix is to draw each command into the rows its
clip admits — ADR 0010 — and the numbers are:

| | before | after |
|---|---|---|
| `bug1721218_reduced.pdf` rasterise | 48.7 s | 0.24 s |
| peak resident memory | ~1.7 GB | 53 MB |
| masks held | 1.73 GB | 25.5 MB, capped at 32 MiB |
| whole corpus gate | 41 s | 15 s |

**A persuasive arithmetic match is not a measurement.** `3576 × 485 kB = 1.7 GB` was right
about the memory and told us nothing about the time. Profile first, even when the story
already adds up.

## What to do next

Re-evaluated at the end of this session against measurements rather than against the
previous list, because this session showed that the thing everyone was sure about was
wrong. Each item below carries the number that justifies its position, so the next person
can disagree with the ordering on evidence rather than taste.

The one-line version: **the largest gaps left are the ones nothing reports, and the
cheapest instrument for finding them is already written and unused.**

### 1. Point the reference oracle at the corpus

`tools/pdfref` implements the triangulation rule, three reference renderers are installed,
and 974 documents are on disk — and the harness is wired to exactly **one** hand-built
fixture (`basic.pdf`, in `render_real_pdf.rs`). Nothing compares our rendering of a real
document against anything. Both of this session's defects would have been caught the first
time it ran.

Surveyed over a spread sample (every seventh corpus document plus the 14 in `doc/`, 154
documents, of which 138 comparable):

| of the 95 we report as complete | count |
|---|---|
| agree with the reference consensus | 71 |
| **contradicted by it** | **15** |
| — of those, by ≥ 2× the references' own disagreement | **8** |
| — of those, marginal (within 2×, where the tolerance is simply tight) | 7 |

So roughly **one page in twelve that we claim to draw completely is contradicted by two
independent implementations by more than they disagree with each other**. The worst in the
sample is `issue12007_reduced.pdf`: mean error 93 where the references agree to 0.10, SSIM
0.28 — and it reports `unsupported: []`. (Its cause is item 3 below; a hidden layer drawn
in full.)

The comparison itself needs no new code, and it is **affordable**: `cargo run -p pdfref`
over a 98-document spread sample takes 20.9 s — 0.21 s per document for three reference
renders and the comparisons between them — so the whole corpus is minutes, not hours, and
this can be an ordinary gate rather than a nightly job. Measure any such timing on a
*spread* sample: an alphabetical slice of this corpus is not representative of it, and the
first estimate made that way came out forty times too high.

What it does need is a decision about the tolerance, and the table above is the argument:
comparing our deviation against *the references' own spread* separates real defects from a
merely tight bound far better than an absolute threshold, and it is exactly the difference
between the 15 and the 8. Ratchet per document by outcome, the way `corpus.rs` ratchets its
counts, so a new disagreement fails the build and a fixed one can never come back.

### 2. Draw annotation appearance streams

**148 of 988 first pages carry an annotation with an appearance stream that is not hidden,
and we draw none of them and report nothing.** Those pages are counted in the 674 that
"draw with nothing reported". By subtype, across those pages: 467 `Widget`, 63 `FreeText`,
37 `Ink`, 28 `Highlight`, 25 `Text`, 14 `Stamp`, and a tail of 25 more.

This is the largest visible gap that is not blocked on the sandbox — larger than soft masks
(45) and encryption (20) — and it is much smaller work than "implement clause 12", because
*drawing* an annotation needs none of the interactivity:

- An annotation's `/AP /N` is a **form XObject**, and `Interpreter::draw_xobject` already
  runs those, with `/Matrix`, `/BBox` clipping and `/Resources`.
- ISO 32000-2 §12.5.5 gives the whole of the placement: transform `/BBox` by `/Matrix`,
  take the bounding box of the result, and map that onto the annotation's `/Rect`.
- `/AS` selects a state when `/AP /N` is a sub-dictionary rather than a stream; `/F` bits 2
  and 6 (Hidden, NoView) mean draw nothing.

So it is a new pass over `/Annots` after the page content, feeding the interpreter that
already exists. 244 first pages carry `/Annots` at all; the 96 that are not in the 148
carry only annotations that are hidden, are `Popup` or `Link`, or have no appearance
stream. An annotation with no appearance should be **reported**, not synthesised —
generating one from `/IC`, `/C`, `/BS` and the subtype's own rules is a separate and much
larger job, and principle 3 says say so rather than guess.

### 3. Honour optional content

Measured properly this time, because the previous handover's "31 documents" counted files
that merely carry the feature:

| | count |
|---|---|
| documents carrying `/OCProperties` | 33 |
| whose default configuration hides something | 8 |
| **whose page one draws content that should be hidden** | **5** |

Small, but it is the one thing in the tree that can be *dramatically* wrong on screen while
reporting nothing — `issue12007_reduced.pdf` above draws an entire hidden screenshot layer
over a page the references leave nearly blank.

One scope correction for whoever takes it: an `/OC` entry usually points at an **OCMD**
rather than at an optional content group directly, so this is not merely "read `/OFF` and
skip `BDC /OC`". It needs `/OCGs` (one group or an array), the `/P` policy
(`AnyOn`/`AllOn`/`AnyOff`/`AllOff`), and both entry points — `BDC /OC` marked-content spans
*and* `/OC` on form and image XObjects, which is how `issue12007_reduced.pdf` hides its
layers. `/VE` visibility expressions also occur in the corpus (`visibility_expressions.pdf`
is named after them) and can be reported rather than implemented at first.

### 4. Then, by what the corpus says real documents need

**Soft masks and transparency groups** (45 documents, and the last thing `doc/` reports),
**encryption** (20 documents — 11 cannot reach page one, 9 more draw a blank page and now
say so), and **CID encodings** (73 documents; note that only 3 of those need the predefined
`CMap` data with its licensing question — the other 70 need code). **Type1 fonts** are
smaller than they look: no corpus page one reaches one.

All three announce themselves, which is why they sit below the three items above: a gap
that reports is a gap you can measure and schedule, and a gap that does not is a gap that
ships.

### Speed, if it comes up again

Re-measured after the change rather than assumed. `callgrind` over the whole of
`bug1721218_reduced.pdf` at 612×792 — open, interpret and rasterise, 16.1 G instructions:

| | share |
|---|---|
| `tiny_skia::pipeline::lowp::gradient` | 29.7% |
| `pdf_model::function::Function::parse` | 23.2% |
| `pdf_model::function::Function::eval` | 13.8% |
| `ColourSpace::to_rgb_at` | 2.6% |

Two things to read from that. **The gradient stage is still the largest single item**
because a `Ramp` carries 256 samples, so a shading becomes a 256-stop gradient and
`tiny-skia` scans its stops per pixel batch; handing the *rasteriser* fewer stops would fix
it, while coarsening the `Ramp` in the display list would lose fidelity and is not the same
thing. **And roughly 40% of the run is now building the shadings**, not drawing them: a PDF
function is parsed and then sampled 256 times for every shading, and this page has 3576 of
them. Whether that is 3576 *distinct* functions or one function re-parsed 3576 times has
not been checked, and it decides whether the fix is memoisation by object reference or
something harder — check before designing. Neither item is urgent, since the page now opens
in two thirds of a second, but both are measured, so the next person starts from a number
rather than a guess.

### Reproducing the numbers in this section

Every count in this section came from a throwaway example run against
`doc/*.pdf` and `doc/pdf.js/test/pdfs/*.pdf`, and none of them survives in the tree —
deliberately, because scratch-quality diagnostics do not belong in a repository held to
`clippy::pedantic`. They are cheap to rebuild and each is worth rebuilding *properly* as
part of the task it belongs to:

- **The oracle survey** is item 1 itself: render page one, run
  `pdfref::Reference::available()` over the same file, reconcile with
  `pdfref::normalise::to_common_size`, and classify with `pdfref::triangulate`. Roughly 40
  lines. Record `Interpretation::is_complete()` alongside the outcome — the split between
  complete and incomplete is what makes the result readable, since a document we already
  say we cannot draw is expected to differ.
- **The annotation count** walks `page.dict["Annots"]`, skipping `Popup` and `Link`
  subtypes and anything with `/F` bit 2 (Hidden) or bit 6 (NoView), and counts those with
  an `/AP`.
- **The optional-content count** reads `/OCProperties /D /OFF` from the catalog, then looks
  for those groups in page one's `/Properties` resources and on its XObjects' `/OC`. Follow
  OCMDs: the first version of this check looked only for direct group references, reported
  "0 reachable from page one" for `issue12007_reduced.pdf`, and was wrong — the page draws
  a hidden layer through an OCMD. That mistake is why the scope note in item 3 exists.

### What the corpus gate reports today

Ratcheted in `crates/pdf-model/tests/corpus.rs`; the numbers only ever go down.

| | count | |
|---|---|---|
| unopenable | 0 | and it should stay there |
| no page one | 19 | 11 encrypted, 8 with unrecoverable page trees |
| draws incompletely | 281 | 152 JBIG2/JPX, 73 text, 26 soft mask, 19 transparency group, 10 undecodable content stream, 1 bound reached |
| slower than 30 s | 0 | `KNOWN_SLOW` is now empty, and the next document to cross the budget fails the gate |

**The time budget reports; it cannot enforce.** A Rust thread cannot be cancelled, so a
document that never returns hangs the suite rather than failing it. A real budget has to
live inside the interpreter and the rasteriser. `PDFVIEWER_CORPUS_TRACE=1` names each
document on stderr as it starts and finishes, which is how a hang gets identified from a
killed run.

**`doc/pdf.js` is a submodule** (Apache-2.0, pinned at v6.1.200), holding those 974 PDFs and
459 more behind link files. It is optional to clone — every test that uses it reports being
skipped rather than failing — but the ratchets only mean anything where it is present, so CI
must have it.

## Habits these sessions earned

**Look in `read-fonts` before writing font-format code.** The previous handover specified
~80 lines of CFF charset parsing plus two 256-entry tables, and all of it already existed
in `read_fonts::ps`, which `skrifa` re-exports as `skrifa::raw`. See ADR 0006. The same
module also holds `type1`, `charmap` and `agl` — `agl` is now enabled and carries the
Adobe Glyph List, so nothing needs transcribing.

**Profile before believing an explanation, even one whose arithmetic matches.** The last
handover attributed a 48-second page to page-sized clip masks and supported it with
`3576 clips × 485 kB = 1.7 GB`, which is exactly what the process held. The arithmetic was
right about the memory and silent about the time: `callgrind` put the masks at under 4% and
the gradient stage at 78.9%. Fixing what the arithmetic named would have kept nearly all 48
seconds. A number that reproduces one symptom is not a diagnosis.

**Wall-clock benchmarks lie under load; count instructions instead.** A `Command::Fill`
change measured as a 24% *regression* on `cargo bench` and as an 8.5% *improvement* twenty
minutes later, purely from background build load. `valgrind --tool=callgrind` on
`crates/pdf-model/examples/callgrind_interpret.rs` settled it deterministically: 2.065 G
instructions before, 1.951 G after. Always A/B in one sitting, and prefer the instruction
count. `iai-callgrind` wraps this into a bench harness and is the right basis for the CI
perf gates `CLAUDE.md` asks for — not yet wired up.

**Two rasterisers disagreeing is information, not noise — and two agreeing is not proof.**
The CPU-versus-GPU agreement test is what found that Vello needed the same mesh seam repair
`tiny-skia` did, after a comment here had confidently claimed otherwise. Where the backends
differ, one of them is wrong; sweeping a constant against that test is how its value was
chosen. The other half of the rule was learned the hard way this session: both backends
positioned paints in the wrong space, in the same way, for the same reason — the two
libraries share the convention that was misread — so they agreed with each other perfectly
while both were wrong. **Agreement is evidence only where the implementations can fail
independently.** When two things share a dependency, a convention or an author's
assumption, they are not independent, and only a value derived from the specification will
say so.

**Survey the corpus before designing.** The shading work started with a survey of what
`ShadingType`, `FunctionType` and `PatternType` values actually occur across 974 documents.
It showed axial shadings outnumbering every mesh type sixty to one, and tiling patterns
outnumbering all meshes combined — which set the order of work and would not have been
guessed. `cargo run --release -p pdf-model --example survey` is gone, but it was twenty
lines over `document.xref().object_numbers()`.

**A test written to isolate one rule finds what a corpus cannot.** The ICC evaluator agreed
with two other readers on every real profile in the corpus. Writing a test that assembles a
profile *by hand* — to check that the `u1Fixed15` PCS encoding is decoded as the ICC format
specifies — produced a profile whose darkest colour equals its white point, and black point
compensation divided by a span of floating-point noise and turned white into pure green. No
real profile is shaped that way, so no amount of corpus agreement would have surfaced it.
Synthetic fixtures and real corpora catch different things; neither replaces the other.

**Measure before optimising, and delete what does not measure.** `glyph_for` builds a
`FontRef` per character, which looks like an obvious cache. Caching it changed a dense page
by less than run-to-run noise (3587 lookups, 211 distinct codes), so the cache was removed
and the reason written where the next person will look. The same session's *real* win was
found the same way: hoisting a string allocation out of `substitute::find` took a difficult
lookup from 1.37 ms to 18 µs. `cargo bench -p pdf-model` is the baseline.

## Things worth knowing

- **`doc/md/` holds Markdown conversions of every corpus PDF**, with real tables. When you
  need spec data — encoding tables, operator lists, value constraints — extract it from
  there rather than writing it from memory. The `WinAnsiEncoding` and `MacRomanEncoding`
  tables in `pdf-font` came out of `doc/md/ISO_32000-2_sponsored_EC3.md` Table D.2 that
  way, and the extraction caught three things memory would have got wrong: PDF's
  `MacRomanEncoding` is not Mac OS Roman, and Table D.2's *notes* assign `space` at 160
  and 202, `hyphen` at 173, and every unused WinAnsi code above 32 to `bullet`.
- **The Arlington model is the object model, not the semantics.** It says `/BaseEncoding`
  must be one of three names; it does not say what those encodings contain. Do not expect
  glyph data, operator semantics or rendering rules from it.
- **`Interpretation::text` is a readback of what was drawn**, accumulated by the same loop
  that places the glyphs, and `crates/pdf-model/tests/text_extraction.rs` compares it
  against `pdftotext` over the 14 specification PDFs in `doc/` — not the pdf.js corpus,
  which would need a per-document expectation. It is the only check that catches a code
  reaching a *plausible* wrong glyph. It found the operand-cap defect below on its first
  run, and it is known to bite: reverting that fix scores 93.2%, and shifting every
  `/ToUnicode` entry by one code scores 58.7%. Extending it to the pdf.js corpus is a real
  opportunity — 974 documents against 14 — and would need only a tolerance rather than
  expectations, since `pdftotext` supplies the reference for each.
- **Silent caps are defects, not safety.** The interpreter dropped operands past the 64th,
  which truncated any `TJ` array holding a justified line — three sentences on the
  specification's own title page ended mid-word, with `unsupported: []`. Bounds against
  hostile input are right; reaching one without saying so is not. Every bound now reports.
- **A command draws into the rows its clip admits, not into the page.** `Band` in
  `crates/render-cpu/src/lib.rs`, and ADR 0010 for why rows rather than a rectangle. Two
  consequences to keep in mind when touching that backend: the device transform handed to a
  command already carries the band's row offset, so anything new that composes a transform
  must use *that* one; and the clip mask is band-tall and page-wide, because `tiny-skia`
  needs it to share the pixmap's row stride.
- **The display list is deliberately flat.** `tiny-skia` wants per-clip masks, Vello wants
  a layer stack; both translate. That neither library's model is native is the evidence the
  neutral form is right, and it is what lets the CPU backend validate the GPU one on
  byte-identical input.
- **RADV and lavapipe produce byte-identical output**, so goldens need not be per-adapter.
  A test pins this; if it fails, the assumption has broken, not the code.
- **Pixel comparison cannot police text, so there is a second kind of metric now.** The
  reference renderers disagree with each other at worst-tile 26–28 on text pages — glyph
  hinting, not error — and no threshold fixes that, because the noise floor is above the
  signal. `raster_compare::Comparison::structural_similarity` (SSIM) measures whether the
  same shapes are in the same places instead, and `Tolerance` now bounds it: 0.99 for
  vector, 0.90 for text. Both numbers were measured over 153 reference-against-reference
  pairs from the corpus, and the doc comment records that the distribution is *continuous*
  — 0.8990, 0.8993, 0.8998 and 0.9009 all occur — so 0.90 is a choice about which
  population to exclude (font substitution) and not a discovered boundary. Text
  *correctness* still belongs to the extraction metric.
- **`test-scenes` holds the same page twice**, as a display list and as PDF bytes. That
  pairing is what let the harness work before a parser existed, and it is checked by a test
  that renders both and demands identical pixels.
- **`doc/` holds more than ISO 32000-2.** `PDF20_AN001-BPC.md` is the PDF Association's
  application note on black point compensation, written by ISO 32000's own
  co-project-leader, and it settled a design question the base specification leaves to
  ISO 18619 — which black to align, and why `AbsoluteColorimetric` must not compensate. It
  had been sitting unread while the same question was being answered by looking at what
  other renderers do. Check what is already in `doc/md/` before concluding the
  specification is silent.
- **Debug builds are ~15× slower here, and it changes what a test can assert.** The corpus
  gate is 41 s in release and about ten minutes in debug. Any test with a timing assertion
  is meaningless at debug speed; run those in release and say so in the test.
- `cargo-deny` is installed in the agent's `~/.cargo/bin`; run it before pushing rather
  than finding out from a red pipeline.
