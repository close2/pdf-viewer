# Handover

Written 2026-07-26, updated 2026-07-27 at the end of the fifth working session. Read
`/CLAUDE.md` first — it holds the five non-negotiable principles and they are not optional.
**Principle 5 is the one that changes how to work**: the specification is the only source of
truth, and agreement with poppler, mupdf or pdf.js is evidence that we read it right, never
the definition of right. This file is the state of play, the traps, and what to do next.

## What the fifth session changed, in one paragraph

The reference oracle now runs over the corpus, which the previous handover named as the
single most valuable unused thing in the tree. `crates/pdf-model/tests/oracle.rs` renders
**every page of all 974 corpus documents**, plus page one of the 14 specification PDFs —
1794 pages — with our renderer and with poppler, mupdf and ghostscript, and applies the
triangulation rule to all four, in **125 seconds**. An ordinary gate, not a nightly job.
Our deviation is bounded by *twice the references' own disagreement on that page* rather
than by a fixed number, because no fixed number can serve both a page of flat
fills and a page of small text; ADR 0011 has the argument. It found four things immediately,
three of them silent: the harness had been comparing against the wrong page box, which put
54 documents beyond comparison; text render modes 4 to 7 do not build the clip they promise,
so `text_clip_cff_cid.pdf` drew a solid bar where the references draw a word; an image's
`/Mask` was ignored outright, so `colorkeymask.pdf` drew a band the references correctly
hide; and `/UserUnit` is neither applied nor reported. The page box is fixed. The other
three are now *reported* rather than silently wrong, and each has a test that will fail when
it is implemented. **174 pages we claim to draw completely are contradicted by two
independent renderers**, every one of them named in the gate, and that list is the work.

## Where we are

A PDF **renderer** that opens real files and draws pages: geometry, colour, images,
shadings, patterns and embedded text, on both a CPU and a GPU backend. It is not yet a PDF
*viewer* in the full sense — no annotations, forms, encryption or transparency groups — and
the gap between those two words is measured further down rather than guessed at.

- **195 tests**, `clippy` clean under `pedantic` + `unwrap_used`/`panic`/`arithmetic_side_effects`,
  `cargo fmt --check` clean, `cargo deny` clean on all four checks (verified, not assumed).
- **The 14 specification PDFs in `doc/`** — including ISO 32000-2 itself, 1023 pages and
  101 318 objects — all parse, all render page one with only a soft mask reported on three
  of them, and all extract **100% of the words `pdftotext` finds**.
- **The 974-document pdf.js corpus is a gate, not a survey.** All 974 open, 955 reach page
  one, **664 draw with nothing reported at all**, and everything the other 291 cannot draw
  is named. The counts are ratcheted and can only go down. 1501 of 1501 PDF functions
  parse; **all 1793 shadings build**, mesh types included. The whole gate runs in **19 s**
  and has **no named slow document left**.
- **A second gate asks whether what we drew is *right*.** `oracle.rs` compares us against
  poppler, mupdf and ghostscript over **1794 pages** — every page of the corpus, plus page
  one of each specification PDF — in 125 s. Of the 1424 pages we claim to draw completely,
  **548 agree with the reference consensus, 174 are contradicted by it and 691 are pages the
  references cannot agree on among themselves**. The 174 are named, grouped and ratcheted in
  both directions. ADR 0011.
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
cargo test --release -p pdf-model --test corpus -- --ignored --nocapture   # 974 docs, ~19 s
cargo test --release -p pdf-model --test oracle -- --ignored --nocapture   # 1794 pages vs 3 renderers, ~125 s
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

There is now a third check, and it is the first one that looks at *pixels*: the corpus
oracle compares our page against three renderers that share no code with us. It cannot be
fooled by a font that loads and draws the wrong glyphs — that is a large structural
difference and it fails the structural-similarity bound. What it cannot do is tell you
*which* of a page's differences matters, so it still does not replace looking.

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

### 3. An oracle is only as good as how it invokes the other renderers

The corpus oracle's first run reported 54 documents whose page *size* we disagreed about,
which looked like a `MediaBox` defect in our page tree. It was not. `pdftoppm` and `gs`
default to the **media box**; `mutool` and we use the **crop box**, which §7.7.3.3 defines
as "the region to which the contents of the page shall be clipped (cropped) when displayed
or printed" and is what a viewer shows. The harness had been asking two of the three
references for a different page.

Two things to carry from that. It cost 54 documents of coverage outright — they could not
be compared at all — and it would have been *worse* than useless on a page whose crop box
has the same size as its media box but a different origin, where it would have compared a
correct render against a displaced one and called us wrong. And the fix was decided by the
clause, not by the fact that `mutool` happened to agree with us: agreement was evidence we
had read §7.7.3.3 the same way, which is the only thing agreement is ever evidence of.

Every reference invocation is now explicit about the page box, including `mutool`'s, whose
default was already right — a default that silently changes is a comparison that silently
changes.

### 4. Test against real documents, not hand-written fragments

Cross-reference streams are compressed *and* PNG-predicted. The code said decoding them
was "the caller's responsibility" and then did not do it, so every modern PDF failed with
a misleading `/Root is not a dictionary`. Unit tests on fragments would never have caught
it; the corpus caught it on the first run.

`crates/pdf-syntax/tests/real_documents.rs` and
`crates/pdf-model/tests/render_real_pdf.rs` run over everything in `doc/`. Keep them
passing.

### 5. Unsupported input must stay loud

Every layer reports what it could not handle rather than skipping it: `Unsupported` in the
interpreter, `FontError`, `ImageError`, `CpuRasterError::UnsupportedCommand`. This is not
politeness — it is what makes the comparison harness trustworthy and what caught trap 1.
Do not "helpfully" fall back to a default that renders something plausible.

The oracle found two places where this rule had been broken by omission rather than by
intent, and both were drawing something visibly wrong in silence: text render modes 4 to 7
add the glyphs to the clipping path and we built no clip, so a rectangle meant to be seen
only through the letters covered its whole area; and an image's `/Mask` was ignored, so a
band the document masks out was painted. Both now report, which moved ten documents from
"complete" to "incomplete" in the corpus gate. **A rise in that count is not a regression
when it is a new report** — it is this rule being applied to somewhere it had not been.

The lesson generalises: a feature that is *partly* implemented is the easiest place to lose
this rule, because the operator is handled and the code path exists. `Tr` was parsed, the
mode was stored, three of its eight values were reported, and the four that change the clip
were not.

### 6. Colour: one conversion, and the specification often does not have an answer

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

### 7. `#[expect]`, never `#[allow]`

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

Three rows are new, and all three were found by the oracle rather than by reading the
specification for gaps: they were features we had *partly* implemented, so nothing reported
them and nothing looked wrong until another renderer disagreed.

| Missing | Corpus | Size | Notes |
|---|---|---|---|
| JBIG2, JPX | 152 | — | **Blocked on the sandbox, deliberately.** Both are historically severe attack surface; see principle 3. |
| Text: CID encodings, embedded `CMap`s | 73 | Medium | The breakdown from the gate's own output: 27 fonts with no `/ToUnicode` so a substitute cannot be addressed, 25 with a non-identity `/CIDToGIDMap`, 14 with an embedded `CMap` stream, 3 with a predefined `CMap` (`90ms-RKSJ-H`). Only the last needs vendored data, which is a licensing decision rather than a coding one. |
| Transparency groups, soft masks | 45 | Large | 26 report as `Shading`, 19 as `Operator`. The largest *rendering* gap, and the last thing `doc/` reports. **Knockout groups (§11.4.5.6) are a silent subset**: `knockout_*.pdf` render as if the group were not a knockout, and nothing reports it. |
| Encryption | 20 | Medium | RC4/AES, `/Encrypt`. 11 documents cannot reach page one at all and 9 more draw a blank page. |
| Optional content (`/OC`) | 5 | Small–medium | 33 documents carry `/OCProperties`, 8 hide something by default, and **5 draw hidden content on page one**; the oracle contradicts 4 of them. `BDC` is parsed and ignored and `/OC` on an XObject is never read, so a hidden layer **is drawn anyway** and *nothing is reported*. `issue12007_reduced.pdf` draws a whole hidden screenshot over a page the references leave nearly blank. |
| `LZWDecode` | 3 | Small | The one standard filter absent. A test pins the report and will fail when it lands. |
| Text clipping modes (`Tr` 4–7) | 5 | Medium | Modes 4 to 7 add the glyphs to the clipping path, which takes effect at `ET` and lasts until the state is restored (§9.3.6 Table 106, §9.4.1). We build no clip, so a rectangle painted afterwards to show through the letters covers its whole area — `text_clip_cff_cid.pdf` drew a solid bar over the word. Now reported; a test pins that and will fail when the clip lands. Implementing it means accumulating the glyph outlines of a text object into one clip path. |
| Image `/Mask` | 5 | Medium | Stencil mask stream (§8.9.6.4) and colour-key range array (§8.9.6.5). Only `/SMask` is honoured, so the masked-out part is drawn — `colorkeymask.pdf` painted a band all three references hide. Now reported, with a test. The colour-key form must be applied to the *source* samples, before colour conversion, which is why it is not a two-line change. |
| `/UserUnit` | 2 | Small | §7.7.3.3: the size of a default user-space unit in multiples of 1/72 inch. `mutool` and `gs` scale the page by it, we and `poppler` do not — `bug1947248_*.pdf` come out at 612x792 where they produce 1836x2376. Neither applied nor reported; the oracle lists them under `GEOMETRY`. |
| Type1 fonts (`/FontFile`) | 0 | Medium | No corpus page one reaches it, so this is smaller than it looks. `read_fonts::ps::type1` exists — check before writing any. |
| Type3 fonts | 23 documents carry one | Medium | Needs `d0`/`d1` and `/CharProcs` interpretation. |
| Sampled shadings on the GPU | 2 | Small | Type 1 only; the CPU backend draws them. |
| Rendering intents beyond `AbsoluteColorimetric` | — | Small | Read and recorded; `A2B0` is not yet selected for `Perceptual`. |
| Annotation appearances | 148 | Medium | **148 of 988 first pages carry a visible annotation with an `/AP`, and none is drawn or reported. The oracle contradicts 47 pages carrying one**, which is the measured visual cost rather than the count of pages at risk. An appearance is a form XObject and the interpreter already runs those, so drawing them is §12.5.5 plus a pass over `/Annots` — see "What to do next". 244 first pages carry `/Annots`; the other 96 carry only hidden, `Popup`/`Link`, or appearance-less annotations. Synthesising a missing appearance is a separate and much larger job. |
| Forms, actions, the rest of clause 12 | — | Large | Interactivity: field values, calculation order, JavaScript, navigation. Not needed to *draw* an annotation. |
| Tagged PDF, metadata | — | Large | Clause 14 beyond output intents. |
| Sandbox (Spike D) | — | Medium | seccomp-BPF + Landlock. Blocks JBIG2/JPX. |

## How much of the specification is implemented

Measured, not estimated. Three answers, because the honest one depends on what you are
counting — and they are in ascending order of how much they should worry you. The first
counts what we *report*, the second what an implementation that shares no code with us
*sees*, and the third what the standard contains.

### By what real documents need

Over the 974-document pdf.js corpus, page one:

| | count | share |
|---|---|---|
| opens | 974 | 100% |
| reaches page one | 955 | 98% |
| **draws with nothing reported** | **664** | **68%** |
| draws, with something reported | 291 | 30% |

That 68% is the number to quote for *reporting*. Everything in the 30% is named — see the
ratchet table below — and the largest single cause is JBIG2 and JPX, which are deliberately
deferred to the sandbox rather than merely unwritten. It was 674 until this session, and
fell because ten documents started reporting something they had been drawing wrongly in
silence: a rise in the incomplete count that is a *new report* is the tree becoming more
honest, not less capable.

### By what an independent renderer sees

This is the number that was missing until this session, and it is the one to worry about.
Over all 1794 pages compared, of the 1424 we claim to draw completely:

| | count | share of the 1424 |
|---|---|---|
| agree with the reference consensus | 548 | 38% |
| **contradicted by it** | **174** | **12%** |
| the references cannot agree among themselves | 691 | 49% |
| not comparable (geometry, or fewer than two renderers) | 11 | 1% |

**One page in eight that we say we drew completely, two independent implementations say we
did not.** The 174 are named in `oracle.rs` and grouped by what the page carries: 47 have an
annotation appearance we do not draw, 4 hide optional content we ignore, 40 use a font
nobody embeds so every renderer substitutes differently — and **83 have nothing on them to
explain it**. That last group is the most valuable list in the repository. 31 of the 174 are
pages beyond the first, which a page-one comparison would never have seen.

**Read the 49% ambiguous with care.** It is not "half the corpus is unsettled": 370 of those
691 pages are two long books, `freeculture.pdf` (352 pages) and `pdkids.pdf`, whose text uses
fonts nobody embedded, so each renderer substitutes a different one and the structural bound
separates them. Over first pages alone the ambiguous share is 21%. Ambiguity concentrated in
a handful of documents says more about those documents than about the gate.

**So read the 664 as "reported nothing", not "drew it right".** The previous session found
two defects that misdrew a gradient and an image on pages inside that count, in silence —
trap 2 — and this session's gate is what makes the difference between the two numbers
visible instead of theoretical.

### By clause

ISO 32000-2 has 824 numbered subclauses under its eight technical clauses. Counting them is
a poor proxy — clause 12 is 166 subclauses of annotation subtypes a viewer adds one at a
time, while clause 8's 128 decide whether any page looks right at all — so this is a
judgement about state, not an arithmetic result.

| Clause | Subclauses | State |
|---|---|---|
| 7 Syntax | 138 | **Nearly complete.** Objects, all filters but `LZWDecode`, classic and stream xrefs, object streams, incremental updates, recovery by scanning. **Encryption is absent** and is the largest hole here. |
| 8 Graphics | 128 | **Nearly complete.** Paths, clipping, all eleven colour space families, all seven shading types, both pattern types, form and image XObjects, inline images, ICC colour management. Optional content (`/OC`) is not honoured, so hidden layers draw on 5 corpus first pages; an image's `/Mask` is not applied; and `CalGray`/`CalRGB` come out too dark against all three references. |
| 9 Text | 65 | **Partial.** Simple and composite fonts through embedded TrueType, CFF and OpenType programs; the standard 14 by substitution; `/ToUnicode`. Missing: bare Type1 (`/FontFile`), Type3 fonts, embedded `CMap` streams, predefined `CMap`s, and the clipping text render modes (§9.3.6 modes 4–7). |
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

**The oracle is wired to the corpus, and the corpus answered.** `oracle.rs`, ADR 0011, 125
seconds for 1794 pages against three independent renderers. What it teaches is less about
the harness than about how the previous four sessions' numbers should have been read:

| | |
|---|---|
| pages we call complete | 1424 |
| agreeing with the reference consensus | 548 |
| **contradicted by it** | **174** |
| ambiguous — the references disagree with each other | 691 |

**Self-reporting measures honesty, not correctness.** 68% of corpus first pages report
nothing wrong; of the pages we call complete, 38% actually agree with two independent
implementations, 12% are contradicted and the rest are pages the references cannot settle
between themselves. Neither number is wrong and neither replaces the other — but only one of
them was being quoted, and it was the flattering one.

**Every page of the corpus, not every page of everything.** The pdf.js files are there
because each broke a reader once, and a file reduced from a bug report does not reliably put
the interesting page first — so all 1761 corpus pages are compared. The specification PDFs
are the opposite: 1382 pages from 14 files, where page 500 exercises what page 499 did, so
they stay at page one. Going from 988 pages to 1794 added 31 contradicted pages that a
page-one comparison could never have seen, for 1.5× the wall clock. Checked before
committing to it: all three references seek through the cross-reference table, so page 300
of a 352-page book costs what page 1 does — the run is linear in pages, not quadratic.

**Nine tenths of that wall clock is the external renderers** — 1596 s of processor time
against our 149 s — which the gate now prints, because "where does this gate's time go" is
otherwise answered by intuition. It is also the answer to whether caching reference renders
would help: it would remove most of the run, and it is not done, because 125 s is affordable
and a cache key that omits one variable — the crop-box flag, a renderer upgrade — compares
against stale renders in silence. That is the same defect class this session just fixed, and
it would be harder to see the second time.

**Three of the four things the gate found on its first runs were in features we had already
implemented.** Not missing subsystems — `Tr` was parsed and three of its eight modes
reported, `/SMask` was honoured while `/Mask` beside it was not, `/CropBox` was read by us
and asked of only one of the three references. A gap in something unimplemented announces
itself; a gap inside something implemented does not, and no amount of reading the
specification for "what have we not built" finds it. Comparing output does.

**A tolerance measured on the thing being tolerated beats a tolerance chosen.** The bound
here is twice the references' own disagreement *on that page*, which makes the same rule
strict on a page of flat fills, where they agree to a worst tile of 0.4, and forgiving on a
page of small text, where they differ by 26 among themselves. The evidence for preferring it
is the previous session's spread sample rather than anything measured here: on those 95
pages a fixed bound called 15 contradicted and the relative bound called 8, and the 7 it
dropped were pages where the references simply disagreed with each other more than usual.
The fixed-bound variant has not been run over the whole corpus, so the ratio at this scale
is unmeasured.

**A ratchet has two directions and both were confirmed.** Removing one name from the list
fails the build with "newly contradicted"; adding one that no longer applies fails it with
"no longer contradicted, delete the entry". Checked by doing both, not by reading the code.

## What to do next

The oracle changed the ordering, which is what it was for. The previous list ranked work by
what the corpus *reported*; this one ranks it by what an independent renderer *sees*, and
the two disagree.

The one-line version: **174 pages we claim to draw are contradicted, 83 of them for no
reason visible on the page — start there, not with a feature list.**

### 1. Work the unexplained list

`CONTRADICTED_UNEXPLAINED` in `oracle.rs`: 83 pages carrying no undrawn annotation, no
hidden optional content and no substituted font, so the difference is in something we
believe we implement.

Six pages were examined by opening their artefacts, and three of the six causes are still
live in the lists (the other three became the reported gaps in item 3):

- **`knockout_*.pdf` are knockout transparency groups** (§11.4.5.6), where an object
  composites against the group's initial backdrop rather than against what is already there.
  `mutool` and `gs` show no blend where two rectangles overlap; we and `poppler` show it.
  Unimplemented and, unlike soft masks, unreported. In this list.
- **`mesh_shading_empty.pdf` draws the same mesh displaced horizontally** — a placement
  question, and the class of defect trap 2 is about. In this list.
- **`calgray.pdf` and `calrgb.pdf` come out markedly darker than all three references** —
  `A = 0.35` reads as near-black rather than mid grey. §8.6.5.2 and §8.6.5.3 define both
  spaces in CIE terms, so the conversion ends in XYZ and the destination's encoding transfer
  function still has to be applied; ours looks like linear luminance written straight into an
  sRGB raster. This is a colour defect on *every* page using either space, and trap 6 governs
  how to fix it: one conversion, and read ADR 0009 first. Filed under
  `CONTRADICTED_SUBSTITUTED_FONT`, because the page happens to label its swatches with a
  non-embedded font — which is the caution below, in one example.

The other 81 are unexamined. Each one is a page where two implementations sharing no
code agree and we differ by more than twice their own disagreement, with the artefacts
already written: `<target>/tmp/oracle/<stem>/p<n>/` holds our render, each reference's, a
side-by-side and a difference heatmap. **Look at the side-by-side first** — five minutes of
looking has so far explained every page it was spent on.

Two cautions. A page may be contradicted for a reason other than the one its group names —
`calgray.pdf` sits under substituted fonts and differs in its colour. And principle 5 is not
suspended by a list: each entry is a question to take to the specification, and "make it
match mupdf" is exactly the failure this project forbids.

### 2. Draw annotation appearance streams

Still the largest single group, and now measured twice: **148 of 988 first pages carry a
visible annotation with an `/AP`, and the oracle contradicts 47 pages that carry one.** The
second number is the visual cost; the first is the exposure. Across those 47 pages: 131
`Widget`, 24 `Ink`, 17 `FreeText`, 4 `Stamp`, one `Square`, one `Highlight`.

It remains much smaller work than "implement clause 12", because *drawing* an annotation
needs none of the interactivity:

- An annotation's `/AP /N` is a **form XObject**, and `Interpreter::draw_xobject` already
  runs those, with `/Matrix`, `/BBox` clipping and `/Resources`.
- ISO 32000-2 §12.5.5 gives the whole of the placement: transform `/BBox` by `/Matrix`,
  take the bounding box of the result, and map that onto the annotation's `/Rect`.
- `/AS` selects a state when `/AP /N` is a sub-dictionary rather than a stream; `/F` bits 2
  and 6 (Hidden, NoView) mean draw nothing.

So it is a new pass over `/Annots` after the page content, feeding the interpreter that
already exists. An annotation with no appearance should be **reported**, not synthesised —
generating one from `/IC`, `/C`, `/BS` and the subtype's own rules is a separate and much
larger job, and principle 3 says say so rather than guess.

Landing it should delete `CONTRADICTED_ANNOTATIONS` from `oracle.rs`. If it does not, the
remaining entries were never about annotations, and that is worth knowing too.

### 3. The three silent gaps this session reported but did not fix

Each is now loud, each has a test that will fail when it lands, and each is a page drawn
visibly wrong today:

- **Text clipping modes** (`Tr` 4–7, §9.3.6 Table 106 and §9.4.1). The glyphs of a text
  object join the clipping path at `ET` and stay until the state is restored. Implementing it
  means accumulating each shown glyph's outline, transformed into page space, and pushing one
  `Clip` at `ET` — the display list's `Clip` already carries a path, a transform and a
  parent, so nothing new is needed in `pdf-render`. 5 corpus first pages report it.
- **Image `/Mask`**, stencil (§8.9.6.4) and colour-key (§8.9.6.5). The colour-key form must
  be applied to the *source* samples before colour conversion, which is why it is not a
  two-line change in `image.rs`. 8 corpus first pages report it; 5 of those had reported
  nothing at all before, and the other 3 were already incomplete for another reason.
- **`/UserUnit`** (§7.7.3.3), which scales the page. 2 corpus documents, and the only reason
  it matters more than that count suggests is that getting a page's *size* wrong invalidates
  every comparison on it.

### 4. Honour optional content

Unchanged in substance and now confirmed by the oracle, which contradicts 4 of the 5 pages
that draw a hidden layer:

| | count |
|---|---|
| documents carrying `/OCProperties` | 33 |
| whose default configuration hides something | 8 |
| **whose page one draws content that should be hidden** | **5** |

One scope correction for whoever takes it: an `/OC` entry usually points at an **OCMD**
rather than at an optional content group directly, so this is not merely "read `/OFF` and
skip `BDC /OC`". It needs `/OCGs` (one group or an array), the `/P` policy
(`AnyOn`/`AllOn`/`AnyOff`/`AllOff`), and both entry points — `BDC /OC` marked-content spans
*and* `/OC` on form and image XObjects, which is how `issue12007_reduced.pdf` hides its
layers. `/VE` visibility expressions also occur in the corpus (`visibility_expressions.pdf`
is named after them) and can be reported rather than implemented at first.

### 5. Then, by what the corpus says real documents need

**Soft masks and transparency groups** (45 documents, and the last thing `doc/` reports),
**encryption** (20 documents — 11 cannot reach page one, 9 more draw a blank page and now
say so), and **CID encodings** (73 documents; note that only 3 of those need the predefined
`CMap` data with its licensing question — the other 70 need code). **Type1 fonts** are
smaller than they look: no corpus page one reaches one.

All three announce themselves, which is why they sit below the items above: a gap that
reports is a gap you can measure and schedule, and a gap that does not is a gap that ships.

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

The oracle survey is no longer a throwaway: it is `oracle.rs`, it runs on demand, and its
per-page lines are the evidence for every comparison count above.

The *classification* counts still are throwaway, and deliberately so — scratch-quality
diagnostics do not belong in a repository held to `clippy::pedantic`. All were a sixty-line
`examples/classify.rs` run over the contradicted list and deleted afterwards, and each is
worth rebuilding properly as part of the task it belongs to:

- **The annotation count** walks `page.dict["Annots"]`, skipping `Popup` and `Link`
  subtypes and anything with `/F` bit 2 (Hidden) or bit 6 (NoView), and counts those with
  an `/AP`.
- **The optional-content count** reads `/OCProperties /D /OFF` from the catalog, then looks
  for those groups in page one's `/Properties` resources and on its XObjects' `/OC`. Follow
  OCMDs: the first version of this check looked only for direct group references, reported
  "0 reachable from page one" for `issue12007_reduced.pdf`, and was wrong — the page draws
  a hidden layer through an OCMD. That mistake is why the scope note in item 4 exists.

Two more were added and thrown away this session. **Whether a page's fonts are embedded**
walks each `/Font` resource and its `/DescendantFonts` looking for `/FontFile`, `/FontFile2`
or `/FontFile3` in the descriptor; it is what separates a page whose text we draw with a
substitute — where a disagreement may be nobody's defect — from one where the document
supplied the font and we should match. And **the page census** — 3143 pages across the 988
documents, 869 of them single-page files, 1382 pages in the 14 specification PDFs — is
`Pages::new(&document).len()` over the same file list, which is what settled the scope
decision in ADR 0011.

### What the corpus gate reports today

Ratcheted in `crates/pdf-model/tests/corpus.rs`; the numbers only ever go down.

| | count | |
|---|---|---|
| unopenable | 0 | and it should stay there |
| no page one | 19 | 11 encrypted, 8 with unrecoverable page trees |
| draws incompletely | 291 | 157 image (152 JBIG2/JPX, 5 `/Mask`), 73 text, 26 soft mask, 24 operator, 10 undecodable content stream, 1 bound reached |
| slower than 30 s | 0 | `KNOWN_SLOW` is now empty, and the next document to cross the budget fails the gate |

### What the oracle gate reports today

Ratcheted in `crates/pdf-model/tests/oracle.rs`, by name and in both directions.

| of the 1424 pages we call complete | count | |
|---|---|---|
| agree with the reference consensus | 548 | |
| **contradicted** | **174** | 47 annotation appearances, 4 optional content, 40 substituted fonts, **83 unexplained** |
| ambiguous | 691 | the references disagree with each other; 370 of them are two long books set in fonts nobody embedded |
| our page geometry differs | 3 | 2 are `/UserUnit`, 1 unexamined |
| not comparable | 8 | fewer than two references produced an image, or they disagree on the page size |

The 370 incomplete pages are compared and printed too, but cannot fail the gate: a page we
already say we cannot draw is expected to differ, and listing hundreds of them would drown
the signal.

**Where its time goes, measured and printed by the gate itself:** 1596 s of processor time
in the three external renderers against 149 s in ours, for 125 s of wall clock on 24 cores.
Nine tenths of this gate is `pdftoppm`, `mutool` and `gs`, which is what to remember if it
ever needs to be faster — and why a content-addressed cache of reference renders is the
obvious lever, with the equally obvious risk that a cache key omitting one variable (the
crop-box flag, the renderer version) would compare against stale renders in silence.

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

**A gap inside a feature you have implemented does not announce itself.** Every missing
subsystem in this tree reports — `LZWDecode`, JBIG2, encryption — because somebody wrote the
report while deciding not to write the feature. The gaps that ship are the ones *inside*
something implemented: `Tr` was parsed and three of its eight modes reported, `/SMask` was
honoured while `/Mask` beside it was not. Reading the specification asking "what have we not
built" cannot find those, because the answer is "nothing". Comparing output against another
implementation can, and did, three times in one afternoon.

**Ask the reference the same question you asked yourself.** Two of the three renderers were
being asked for the media box while we rendered the crop box, which put 54 documents beyond
comparison and would have produced false failures on any page whose two boxes differ only in
origin. A comparison harness has its own defects, they look exactly like ours, and the way to
tell them apart is to check the invocation against the clause before believing the verdict.

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

- **The oracle's artefacts are the fastest diagnostic in the tree.** Every page that is not
  agreement leaves `<target>/tmp/oracle/<stem>/p<n>/` holding our render, each reference's, a
  side-by-side strip and a difference heatmap per reference. Open the side-by-side first: it
  is one image, four panels, ours leftmost, and it has explained every page it was pointed at
  so far — a solid bar where a word should be, a band that should have been masked out, grey
  swatches at the wrong lightness. Pages that agree have theirs deleted, so what is on disk
  is exactly the set worth looking at — 570 MB of it.
- **Reference renderers are given 30 seconds and then killed.** A corpus holds files written
  to make a reader loop, and `Command::output` waits forever. `Reference::render_within` polls
  and kills; there is deliberately no unbounded variant.

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
  gate is 19 s in release and minutes in debug. Any test with a timing assertion is
  meaningless at debug speed; run those in release and say so in the test. The oracle gate
  is the exception that proves it: 1596 of its 1745 seconds of processor time is three
  external renderers, whose speed does not depend on how we were built.
- `cargo-deny` is installed in the agent's `~/.cargo/bin`; run it before pushing rather
  than finding out from a red pipeline.
