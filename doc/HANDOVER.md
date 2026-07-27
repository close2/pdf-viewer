# Handover

Written 2026-07-26, updated 2026-07-27 at the end of the sixth working session. Read
`/CLAUDE.md` first — it holds the five non-negotiable principles and they are not optional.
**Principle 5 is the one that changes how to work**: the specification is the only source of
truth, and agreement with poppler, mupdf or pdf.js is evidence that we read it right, never
the definition of right. This file is the state of play, the traps, and what to do next.

## What the sixth session changed, in one paragraph

The previous session built the oracle and it named 174 pages we claimed to draw that two
independent renderers contradicted. This session worked that list, and **it is down to 120**.
Two things did it. **`CalGray` and `CalRGB` are now converted through CIE XYZ** as §8.6.5.2
and §8.6.5.3 define them, instead of being passed through as their device equivalents: with
`/Gamma 1.0`, `A = 0.35` was rendering as 89 of 255 where all three references give 160 — a
mid grey drawn as a near-black, on every page anywhere using either space. And **annotation
appearance streams are drawn**, which was the largest single group in the contradicted list
and had been entirely absent: 148 of 988 first pages carry a visible annotation with an
`/AP`, and none of them was drawn *or reported*. That group loses 45 of its 47 entries. The
two that stayed turned out never to have been about annotations — they are a one-pixel raster
rounding difference on a small fractional page — which is exactly what the previous handover
said their staying would mean. Along the way the XYZ-to-sRGB matrix, which lived in two
places, became one; and the corpus's incomplete count rose from 291 to 368, every bit of it
new reporting, for the first time *because a feature landed*.

## Where we are

A PDF **renderer** that opens real files and draws pages: geometry, colour, images,
shadings, patterns, embedded text and annotation appearances, on both a CPU and a GPU
backend. It is not yet a PDF *viewer* in the full sense — no forms, encryption or
transparency groups — and the gap between those two words is measured further down rather
than guessed at.

- **214 tests**, `clippy` clean under `pedantic` + `unwrap_used`/`panic`/`arithmetic_side_effects`,
  `cargo fmt --check` clean, `cargo deny` clean on all four checks (verified, not assumed).
- **The 14 specification PDFs in `doc/`** — including ISO 32000-2 itself, 1023 pages and
  101 318 objects — all parse, all render page one with only a soft mask reported on three
  of them, and all extract **100% of the words `pdftotext` finds**.
- **The 974-document pdf.js corpus is a gate, not a survey.** All 974 open, 955 reach page
  one, **587 draw with nothing reported at all**, and everything the other 368 cannot draw
  is named. The counts are ratcheted. 1501 of 1501 PDF functions parse; **all 1793 shadings
  build**, mesh types included. The whole gate runs in **15 s** and has **no named slow
  document left**.
- **A second gate asks whether what we drew is *right*.** `oracle.rs` compares us against
  poppler, mupdf and ghostscript over **1794 pages** — every page of the corpus, plus page
  one of each specification PDF — in 75 s. Of the 1340 pages we claim to draw completely,
  **555 agree with the reference consensus, 120 are contradicted by it and 654 are pages the
  references cannot agree on among themselves**. The 120 are named, grouped and ratcheted in
  both directions. ADR 0011.
- **Colour resolves from the document.** `ICCBased` profiles are evaluated by an A2B
  evaluator written here, `CalGray`/`CalRGB`/`Lab` are converted through XYZ, `/DefaultCMYK`
  and output intents are honoured, and there is exactly one route from XYZ to a pixel and
  exactly one `DeviceCMYK` conversion. ADRs 0009 and 0012.
- **Annotations draw.** `/AP /N` is placed by §12.5.5's algorithm and run by the same
  machinery as any other form XObject; nothing is synthesised, and an annotation with no
  appearance is reported. ADR 0013.
- Both backends draw everything the display list can express, and agree on it: **eight**
  headless GPU scenes hold `tiny-skia` and Vello to the same pixels, at more than one scale
  and along both axes — see trap 2 for why that matters.

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
cargo test --release -p pdf-model --test oracle -- --ignored --nocapture   # 1794 pages vs 3 renderers, ~75 s
cargo bench -p pdf-model                   # interpretation, the time-to-first-page path
cargo deny check
cargo +nightly fuzz run lexer -- -runs=50000     # from fuzz/, needs nightly
```

## Crate map

| Crate | Does | Notes |
|---|---|---|
| `pdf-spec` | Object-model validation tables | Generated from Arlington by `build.rs` |
| `pdf-syntax` | Lexer, objects, xref, filters, `Document` | Touches untrusted bytes first |
| `pdf-model` | Page tree, content interpreter, annotations, image decode | Where PDF semantics live |
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

**This is still live, and there is a named example.** `issue20504.pdf` sets six scripts in
six embedded subset fonts, and we draw `!"#$` — the raw character codes through a
substitute — for all of them while reporting nothing. One of its fonts writes
`/Differences [33 /gid2436 /gid1620 …]`, a subsetter's convention for naming a glyph by
index that §9.6.5 does not define, and an unrecognised name is falling back to the standard
encoding rather than to no glyph. The oracle found it; no metric we own can.

Two automated checks *do* catch a wrong mapping, both in `crates/pdf-font/src/lib.rs`:

- `the_pdf_widths_agree_with_the_font_programs_own_advances` — the document's `/Widths`
  and the CFF charstring's own advance are independent statements of the same fact, so
  they agree only if the code reached the glyph the producer meant. This is the strongest
  check in the tree: it verifies the mapping without consulting the mapping.
- `an_uncovered_code_has_no_glyph_rather_than_a_guessed_one` — pins the absence of the
  code-as-glyph-index fall-through.

Both were confirmed to fail when the defects they describe are deliberately reintroduced.
They are complementary: an off-by-one charset trips only the first, a reinstated
fall-through only the second. Neither replaces looking at the page, and neither caught
`issue20504.pdf`.

The third check is the first that looks at *pixels*: the corpus oracle compares our page
against three renderers that share no code with us. It cannot be fooled by a font that loads
and draws the wrong glyphs. What it cannot do is tell you *which* of a page's differences
matters, so it still does not replace looking.

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

The same reasoning shaped this session's annotation tests: §12.5.5's placement algorithm is
correct for any axis-aligned `/Matrix` even if you measure the *untransformed* `/BBox`, so
the fixtures use a rotation and a non-square `/Rect`. A square rectangle cannot tell the two
axes' scales apart.

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

The converse is trap 8: a corpus cannot find what no document in it happens to contain.

### 5. Unsupported input must stay loud

Every layer reports what it could not handle rather than skipping it: `Unsupported` in the
interpreter, `FontError`, `ImageError`, `CpuRasterError::UnsupportedCommand`. This is not
politeness — it is what makes the comparison harness trustworthy and what caught trap 1.
Do not "helpfully" fall back to a default that renders something plausible.

The oracle found three places where this rule had been broken by omission rather than by
intent, and all three were drawing something visibly wrong in silence: text render modes 4
to 7 add the glyphs to the clipping path and we built no clip; an image's `/Mask` was
ignored, so a band the document masks out was painted; and a page's annotations were absent
entirely. **A rise in the incomplete count is not a regression when it is a new report** —
it is this rule being applied to somewhere it had not been.

The lesson generalises: a feature that is *partly* implemented is the easiest place to lose
this rule, because the operator is handled and the code path exists. `Tr` was parsed, the
mode was stored, three of its eight values were reported, and the four that change the clip
were not.

There is now one place where a report accompanies drawing rather than replacing it, and it
is deliberate. An `/AcroForm` setting `/NeedAppearances` is the document saying its stored
appearance streams are not the ones to draw (§12.7.4.3); we draw them anyway, because they
are all the file offers, and report that they may be stale. Two different true statements,
and suppressing either loses information. Do not generalise it further without the same
argument.

### 6. Colour: one conversion, and the specification often does not have an answer

Three separate `DeviceCMYK` → RGB conversions used to live in this tree and they disagreed.
`0.5 0 0 0.5 k` gave a red channel of 0.25; the same colour through `scn` gave 0.0; a CMYK
image gave a third answer. Nothing about a rendered page reveals that — each looks like a
plausible colour. `crates/pdf-model/tests/colour_paths.rs` now drives one value through all
three routes and demands they agree; it was verified to fail when the old code is restored.

Add no fourth path. `ColourSpace::to_rgb` is the only place a colour becomes RGB, and — since
this session — `colour::xyz_d50_to_srgb` is the only place an XYZ becomes a pixel. That
second rule was added because the *same* defect had quietly recurred one level down: `lab()`
and `icc::xyz_to_rgb` each held their own copy of the nine-constant D50-to-sRGB matrix.
Nothing had gone wrong yet. It is one edit away from going wrong invisibly.

The other half is harder to hold onto: **ISO 32000-2 defines no `DeviceCMYK` conversion at
all**. §8.6.4.4 says "concentrations of process colourants" and stops; §8.6.5.7 NOTE 3 says
nothing in PDF describes the device. What the specification *does* say is where to ask —
`/DefaultCMYK` (§8.6.5.6, normative), an output intent's `/DestOutputProfile` (§14.11.5),
and an `ICCBased` profile — and all three are implemented and all three outrank the
fallback table. When you touch that table, do not reach for what another renderer produces:
read ADR 0009, and if you change it, change it as a documented choice.

The same shape recurs for a Cal space's `/BlackPoint`: §8.6.5.9 leaves black point
compensation to the processor whenever `/UseBlackPtComp` is `Default`, which is every real
document. It is read and deliberately not applied, and ADR 0012 has the argument — including
the part that decided it, which is that a stretch built from the entry is *undefined* on
input Table 63 permits.

### 7. `#[expect]`, never `#[allow]`

Every lint exception in the tree is `#[expect(..., reason = "...")]`. It errors when it
stops being necessary, which has already removed several stale ones. A bare `allow` hides
that forever.

### 8. A corpus finds what documents contain, not what the specification says

Added this session, because it is the mirror of trap 4 and the two are easy to confuse.

The ICC evaluator agreed with two other readers on every real profile in the corpus. A test
that assembled a profile *by hand*, to check one clause of the ICC encoding, produced a
profile whose darkest colour equalled its white point — and black point compensation divided
by a span of floating-point noise and turned white into pure green. No real profile is shaped
that way.

The same thing happened again this session, from the other direction: `calrgb.pdf` page 14
states `BlackPoint [0.2 1.0 1.7]` against `WhitePoint [1 1 1]`, which Table 63 permits and
which no sane producer writes. It is what proved that the black point stretch has no
well-defined answer at all. **The corpus is not a specification, and a clause nothing in it
exercises is still a clause.** Synthetic fixtures and real corpora catch different things.

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
| Text: CID encodings, embedded `CMap`s | 76 | Medium | The breakdown from the gate's own output: 27 fonts with no `/ToUnicode` so a substitute cannot be addressed, 25 with a non-identity `/CIDToGIDMap`, 14 with an embedded `CMap` stream, 3 with a predefined `CMap` (`90ms-RKSJ-H`). Only the last needs vendored data, which is a licensing decision rather than a coding one. |
| Synthesised annotation appearances | 63 | Medium–large | An annotation with **no** `/AP` must be drawn from `/IC`, `/C`, `/BS`, `/Border` and its subtype's own rules — a different routine per subtype. 26 `Widget`, 18 `Link`, and the rest markup annotations. Reported, never guessed. ADR 0013. |
| Transparency groups, soft masks | 45 | Large | 26 report as `Shading`, 19 as `Operator`. The largest *rendering* gap, and the last thing `doc/` reports. **Knockout groups (§11.4.5.6) are a silent subset**: `knockout_*.pdf` render as if the group were not a knockout, and nothing reports it. |
| Encryption | 20 | Medium | RC4/AES, `/Encrypt`. 11 documents cannot reach page one at all and 9 more draw a blank page. |
| Form field appearance construction | 7 | Medium | `/NeedAppearances` (§12.7.4.3). The field's value is known only at viewing time, so its appearance has to be built from `/V`, `/DA` and `/Q`. The stored appearance is drawn and the staleness reported. |
| Optional content (`/OC`) | 5 | Small–medium | 33 documents carry `/OCProperties`, 8 hide something by default, and **5 draw hidden content on page one**; the oracle contradicts 3 of them. `BDC` is parsed and ignored and `/OC` on an XObject is never read, so a hidden layer **is drawn anyway** and *nothing is reported*. `issue12007_reduced.pdf` draws a whole hidden screenshot over a page the references leave nearly blank. |
| `LZWDecode` | 3 | Small | The one standard filter absent. A test pins the report and will fail when it lands. |
| Text clipping modes (`Tr` 4–7) | 5 | Medium | Modes 4 to 7 add the glyphs to the clipping path, which takes effect at `ET` and lasts until the state is restored (§9.3.6 Table 106, §9.4.1). We build no clip, so a rectangle painted afterwards to show through the letters covers its whole area — `text_clip_cff_cid.pdf` drew a solid bar over the word. Now reported; a test pins that and will fail when the clip lands. Implementing it means accumulating the glyph outlines of a text object into one clip path. |
| Image `/Mask` | 5 | Medium | Stencil mask stream (§8.9.6.4) and colour-key range array (§8.9.6.5). Only `/SMask` is honoured, so the masked-out part is drawn — `colorkeymask.pdf` painted a band all three references hide. Now reported, with a test. The colour-key form must be applied to the *source* samples, before colour conversion, which is why it is not a two-line change. |
| `/UserUnit` | 2 | Small | §7.7.3.3: the size of a default user-space unit in multiples of 1/72 inch. `mutool` and `gs` scale the page by it, we and `poppler` do not — `bug1947248_*.pdf` come out at 612x792 where they produce 1836x2376. Neither applied nor reported; the oracle lists them under `GEOMETRY`. |
| Annotation `NoZoom`, `NoRotate` | — | Small | Table 167 bits 4 and 5 make an appearance's size or orientation depend on the *view*, which a resolution-independent display list cannot express. Rare. |
| Type1 fonts (`/FontFile`) | 0 | Medium | No corpus page one reaches it, so this is smaller than it looks. `read_fonts::ps::type1` exists — check before writing any. |
| Type3 fonts | 23 documents carry one | Medium | Needs `d0`/`d1` and `/CharProcs` interpretation. |
| Sampled shadings on the GPU | 2 | Small | Type 1 only; the CPU backend draws them. |
| Rendering intents beyond `AbsoluteColorimetric` | — | Small | Read and recorded; `A2B0` is not yet selected for `Perceptual`. |
| Forms, actions, the rest of clause 12 | — | Large | Interactivity: field values, calculation order, JavaScript, navigation. Not needed to *draw* an annotation, which is why drawing landed without any of it. |
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
| **draws with nothing reported** | **587** | **60%** |
| draws, with something reported | 368 | 38% |

That 60% is the number to quote for *reporting*, and it went **down** this session, from
68%. Read that carefully, because it is the opposite of what it looks like: 148 first pages
gained an annotation that had been missing entirely, and 63 documents started saying that an
annotation has no appearance for us to draw. Nothing stopped drawing. What changed is that a
class of absence which used to be invisible is now counted. **This number measures honesty,
and honesty can fall as capability rises.**

### By what an independent renderer sees

This is the number to worry about. Over all 1794 pages compared, of the 1340 we claim to
draw completely:

| | count | share of the 1340 |
|---|---|---|
| agree with the reference consensus | 555 | 41% |
| **contradicted by it** | **120** | **9%** |
| the references cannot agree among themselves | 654 | 49% |
| not comparable (geometry, or fewer than two renderers) | 11 | 1% |

**One page in eleven that we say we drew completely, two independent implementations say we
did not** — down from one in eight. The 120 are named in `oracle.rs` and grouped by what the
page carries: 32 use a font nobody embeds so every renderer substitutes differently, 4 are a
one-pixel page-rounding difference, 3 hide optional content we ignore, and **81 have nothing
on them to explain it**. That last group is the most valuable list in the repository. 21 of
the 120 are pages beyond the first, which a page-one comparison would never have seen.

**Read the 49% ambiguous with care.** It is not "half the corpus is unsettled": 370 of those
654 pages are two long books, `freeculture.pdf` (352 pages) and `pdkids.pdf`, whose text uses
fonts nobody embedded, so each renderer substitutes a different one and the structural bound
separates them. Ambiguity concentrated in a handful of documents says more about those
documents than about the gate.

**So read the 587 as "reported nothing", not "drew it right".**

### By clause

ISO 32000-2 has 824 numbered subclauses under its eight technical clauses. Counting them is
a poor proxy — clause 12 is 166 subclauses of annotation subtypes a viewer adds one at a
time, while clause 8's 128 decide whether any page looks right at all — so this is a
judgement about state, not an arithmetic result.

| Clause | Subclauses | State |
|---|---|---|
| 7 Syntax | 138 | **Nearly complete.** Objects, all filters but `LZWDecode`, classic and stream xrefs, object streams, incremental updates, recovery by scanning. **Encryption is absent** and is the largest hole here. |
| 8 Graphics | 128 | **Nearly complete.** Paths, clipping, all eleven colour space families, all seven shading types, both pattern types, form and image XObjects, inline images, ICC colour management. Optional content (`/OC`) is not honoured, so hidden layers draw on 5 corpus first pages; an image's `/Mask` is not applied. |
| 9 Text | 65 | **Partial.** Simple and composite fonts through embedded TrueType, CFF and OpenType programs; the standard 14 by substitution; `/ToUnicode`. Missing: bare Type1 (`/FontFile`), Type3 fonts, embedded `CMap` streams, predefined `CMap`s, and the clipping text render modes (§9.3.6 modes 4–7). |
| 10 Rendering | 36 | **Partial, and much of it is not applicable.** Colour management and rendering intents are done. Halftones, transfer functions, flatness and smoothness describe a marking device rather than a screen. |
| 11 Transparency | 58 | **Minimal.** All sixteen blend modes are implemented and reach both backends. Transparency groups, soft masks, knockout and isolation are not — this is the largest *rendering* gap. |
| 12 Interactive features | 166 | **Appearances only.** Annotations are placed and drawn from `/AP` (§12.5.5), with the visibility flags of §12.5.3 honoured. Nothing is synthesised, and no forms, actions or navigation exist. |
| 13 Multimedia | 81 | **None**, and unlikely to be a priority. |
| 14 Document interchange | 152 | **Output intents only.** No tagged PDF, no metadata, no marked-content semantics — `BDC`/`EMC` are parsed and ignored. |

So: the parts of the standard that decide whether a page is drawn correctly are largely
done; the parts that make a document *interactive* are not started.

### Feature-by-feature, from the source

| | |
|---|---|
| Content-stream operators | **71 of 73** in Table 50 (`ID`/`EI` are consumed inside the `BI` handler rather than as arms). The two genuinely missing are `d0` and `d1`, which exist only inside Type 3 fonts. `BMC`/`BDC`/`EMC`/`MP`/`DP`/`BX`/`EX`/`i` are matched and deliberately ignored — correct for all but `BDC` with `/OC`. |
| Filters | **7 of 8** standard filters decode: `ASCIIHex`, `ASCII85`, `Flate`, `RunLength`, `Crypt` (pass-through), plus `DCTDecode` for images. `LZWDecode` is **absent** (3 corpus documents). `CCITTFax`, `JBIG2` and `JPX` are reported, not decoded. |
| Colour spaces | **11 of 11** families, and the three CIE-based ones are converted rather than approximated. |
| Function types | **4 of 4** (sampled, exponential, stitching, `PostScript` calculator). |
| Shading types | **7 of 7**, on both backends. |
| Pattern types | **2 of 2** (tiling and shading). |
| Blend modes | **16 of 16**. |
| Font programs | TrueType, CFF, CFF-in-OpenType, CID-keyed CFF. Bare Type1 and Type3 are reported. |
| Annotation appearances | Placed and drawn; not synthesised where absent. |

## Done this session, and what it teaches

Two changes, and between them the contradicted count fell from 174 to 120.

**`CalGray` and `CalRGB` through XYZ** (ADR 0012). Eight pages left the ratchet, and the
XYZ-to-sRGB matrix went from two copies to one. What it teaches is about the *shape* of the
old shortcut rather than the fix: treating a Cal space as its device equivalent is nearly
correct when the document's `/Gamma` is about 2.2, because decoding by it and re-encoding for
sRGB almost cancel — and §8.6.5.2's own EXAMPLE 2 is exactly that space, so most real
documents are the case the shortcut gets right. **A shortcut that is correct on the common
case is the hardest kind to notice**, because the pages that would reveal it are the rare
ones and they report nothing either.

**Annotation appearance streams** (ADR 0013). 45 of the 47 contradicted pages in that group
fixed, and the two that stayed were never about annotations — the previous handover predicted
that reading. Three things it teaches:

- **Drawing is separable from interactivity.** Clause 12 is 166 subclauses and reads like one
  large thing. §12.5.5 alone — an appearance is a form XObject, place its `/BBox` on the
  annotation's `/Rect` — bought the largest single reduction in the contradicted list, and
  needed no field value, no action and no calculation order.
- **A feature landing can *raise* the incomplete count**, and this is the first time it has.
  Before this, an annotation with no appearance was indistinguishable from no annotation at
  all: neither was drawn and neither was reported. Implementing the first made the second
  visible. The rule that a rise which is a new report is honesty rather than regression still
  holds; it just now also applies to features arriving, not only to gaps being confessed.
- **The test that fails is the one you learn from.** An early version conflated "no `/AP` at
  all" with "`/AS` names a state the dictionary omits". The first is a gap in this crate; the
  second is how *every* unchecked check box is written, and §12.5.5 names displaying nothing
  as the correct answer. A fixture written from the clause caught it; the corpus would have
  buried it in 60-odd other reports.

**Looking at four contradicted pages explained all four.** `bug1922766.pdf`,
`bug1934157.pdf`, `bug1669097.pdf` and `issue19505.pdf` all have a fractional page box, and
our raster is one pixel smaller than poppler's and mupdf's and exactly the same size as
ghostscript's. On a 72-row page a one-row shift moves everything on it. Nothing in
ISO 32000-2 says how a fractional page becomes an integer number of pixels. Five minutes of
looking at side-by-sides has now explained every page it has been spent on — six last session,
five this one — which is the strongest recommendation this file can make about how to spend
the next hour.

## What to do next

The one-line version: **120 pages we claim to draw are contradicted, 81 of them for no
reason visible on the page — that list is still the work.**

### 1. Work the unexplained list

`CONTRADICTED_UNEXPLAINED` in `oracle.rs`: 81 pages carrying no undrawn annotation, no hidden
optional content and no substituted font, so the difference is in something we believe we
implement. Three causes are identified and live:

- **`knockout_*.pdf` are knockout transparency groups** (§11.4.5.6), where an object
  composites against the group's initial backdrop rather than against what is already there.
  `mutool` and `gs` show no blend where two rectangles overlap; we and `poppler` show it.
  Unimplemented and, unlike soft masks, unreported.
- **`mesh_shading_empty.pdf` draws the same mesh displaced horizontally** — a placement
  question, and the class of defect trap 2 is about.
- **`issue20504.pdf` draws six scripts as `!"#$`**, silently. See trap 1: an unrecognised
  `/Differences` name (`/gid2436`, a subsetter convention §9.6.5 does not define) is falling
  back to the standard encoding instead of to no glyph. This one is small, self-contained,
  and pure trap 1 — a good first task.

The other 78 are unexamined. Each is a page where two implementations sharing no code agree
and we differ by more than twice their own disagreement, with the artefacts already written:
`<target>/tmp/oracle/<stem>/p<n>/` holds our render, each reference's, a side-by-side and a
difference heatmap. **Look at the side-by-side first.**

Two cautions. A page may be contradicted for a reason other than the one its group names —
`calgray.pdf` sat under substituted fonts and differed in its colour, which is how the whole
of ADR 0012 started. And principle 5 is not suspended by a list: each entry is a question to
take to the specification, and "make it match mupdf" is exactly the failure this project
forbids.

### 2. Honour optional content

The oracle contradicts 3 of the 5 pages that draw a hidden layer:

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

It is now also cheap in a way it was not: the annotation pass established the pattern of a
second walk over the page's own dictionary, and `/OC` on an annotation is one more place the
same visibility question gets asked.

### 3. The three silent gaps still open

Each is loud, each has a test that will fail when it lands, and each is a page drawn visibly
wrong today:

- **Text clipping modes** (`Tr` 4–7, §9.3.6 Table 106 and §9.4.1). The glyphs of a text
  object join the clipping path at `ET` and stay until the state is restored. Implementing it
  means accumulating each shown glyph's outline, transformed into page space, and pushing one
  `Clip` at `ET` — the display list's `Clip` already carries a path, a transform and a
  parent, so nothing new is needed in `pdf-render`. 5 corpus first pages report it.
- **Image `/Mask`**, stencil (§8.9.6.4) and colour-key (§8.9.6.5). The colour-key form must
  be applied to the *source* samples before colour conversion, which is why it is not a
  two-line change in `image.rs`. 5 corpus first pages report it.
- **`/UserUnit`** (§7.7.3.3), which scales the page. 2 corpus documents, and the only reason
  it matters more than that count suggests is that getting a page's *size* wrong invalidates
  every comparison on it.

### 4. Synthesised annotation appearances, if the corpus count is the argument

63 documents carry an annotation with no `/AP`, which is now the third-largest reported gap.
It is genuinely a different drawing routine per subtype and should not be started as one
task. If it is taken, take it one subtype at a time in corpus order: `Widget` (26), `Link`
(18, and its whole appearance is a border — §12.5.6.5 with §12.5.4), then the markup
annotations. Each one that lands should be measured on the oracle rather than assumed to
help, because a synthesised appearance is a *guess at what the producer meant* and the
references guess differently.

### 5. Then, by what the corpus says real documents need

**Soft masks and transparency groups** (45 documents, and the last thing `doc/` reports),
**encryption** (20 documents — 11 cannot reach page one, 9 more draw a blank page and now
say so), and **CID encodings** (76 documents; note that only 3 of those need the predefined
`CMap` data with its licensing question — the other 73 need code). **Type1 fonts** are
smaller than they look: no corpus page one reaches one.

All three announce themselves, which is why they sit below the items above: a gap that
reports is a gap you can measure and schedule, and a gap that does not is a gap that ships.

### Speed, if it comes up again

Re-measured two sessions ago and not since. `callgrind` over the whole of
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
in two thirds of a second, but both are measured.

One caution now that Cal spaces convert properly: `to_rgb_at` was 2.6% when `CalGray` was a
pass-through. It now runs a Bradford adaptation and a matrix per colour, and per *sample*
for a Cal-space image. Nothing in the corpus made the gate slower — it went from 19 s to
15 s over the same documents — but the profile above predates the change and should not be
quoted as if it did not.

### Reproducing the numbers in this section

The oracle survey is `oracle.rs`, it runs on demand, and its per-page lines are the evidence
for every comparison count above. The corpus counts come from `corpus.rs`.

The *classification* counts are throwaway, and deliberately so — scratch-quality diagnostics
do not belong in a repository held to `clippy::pedantic`. Each is worth rebuilding properly
as part of the task it belongs to:

- **The optional-content count** reads `/OCProperties /D /OFF` from the catalog, then looks
  for those groups in page one's `/Properties` resources and on its XObjects' `/OC`. Follow
  OCMDs: the first version of this check looked only for direct group references, reported
  "0 reachable from page one" for `issue12007_reduced.pdf`, and was wrong — the page draws
  a hidden layer through an OCMD.
- **Whether a page's fonts are embedded** walks each `/Font` resource and its
  `/DescendantFonts` looking for `/FontFile`, `/FontFile2` or `/FontFile3` in the descriptor.
- **The annotation subtype breakdown** now comes free: the corpus gate prints the reported
  detail per document, so `grep -o '"[A-Za-z]*: no appearance stream"' | sort | uniq -c`
  over its output is the whole of it.
- **The page census** — 3143 pages across the 988 documents, 869 of them single-page files,
  1382 pages in the 14 specification PDFs — is `Pages::new(&document).len()` over the same
  file list, which is what settled the scope decision in ADR 0011.

### What the corpus gate reports today

Ratcheted in `crates/pdf-model/tests/corpus.rs`; the numbers only ever go down, except where
a rise is a new report and is written down as one.

| | count | |
|---|---|---|
| unopenable | 0 | and it should stay there |
| no page one | 19 | 11 encrypted, 8 with unrecoverable page trees |
| draws incompletely | 368 | 161 image, 76 text, 68 annotation, 26 shading, 26 operator, 10 undecodable content stream, 1 bound reached |
| slower than 30 s | 0 | `KNOWN_SLOW` is empty, and the next document to cross the budget fails the gate |

### What the oracle gate reports today

Ratcheted in `crates/pdf-model/tests/oracle.rs`, by name and in both directions.

| of the 1340 pages we call complete | count | |
|---|---|---|
| agree with the reference consensus | 555 | |
| **contradicted** | **120** | 4 page rounding, 3 optional content, 32 substituted fonts, **81 unexplained** |
| ambiguous | 654 | the references disagree with each other; 370 of them are two long books set in fonts nobody embedded |
| our page geometry differs | 3 | 2 are `/UserUnit`, 1 unexamined |
| not comparable | 6 | fewer than two references produced an image, or they disagree on the page size |

The 454 incomplete pages are compared and printed too, but cannot fail the gate: a page we
already say we cannot draw is expected to differ, and listing hundreds of them would drown
the signal.

**Where its time goes, measured and printed by the gate itself:** 1033 s of processor time
in the three external renderers against 125 s in ours, for 75 s of wall clock on 24 cores.
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
honoured while `/Mask` beside it was not, `CalGray` was resolved and then converted as
`DeviceGray`. Reading the specification asking "what have we not built" cannot find those,
because the answer is "nothing". Comparing output against another implementation can, and
has, five times now.

**A shortcut that is right on the common case is worse than one that is wrong on all of
them.** The Cal-space pass-through was nearly correct for `/Gamma 2.2`, which is what most
documents write, and badly wrong otherwise. Nothing distinguishes the two populations at
runtime, so nothing reported. Prefer the derivation even where the approximation looks close:
"close on the files I tried" is not a property you can test for.

**Ask the reference the same question you asked yourself.** Two of the three renderers were
being asked for the media box while we rendered the crop box, which put 54 documents beyond
comparison and would have produced false failures on any page whose two boxes differ only in
origin. A comparison harness has its own defects, they look exactly like ours, and the way to
tell them apart is to check the invocation against the clause before believing the verdict.

**Look in `read-fonts` before writing font-format code.** An earlier handover specified
~80 lines of CFF charset parsing plus two 256-entry tables, and all of it already existed
in `read_fonts::ps`, which `skrifa` re-exports as `skrifa::raw`. See ADR 0006. The same
module also holds `type1`, `charmap` and `agl` — `agl` is now enabled and carries the
Adobe Glyph List, so nothing needs transcribing.

**Profile before believing an explanation, even one whose arithmetic matches.** An earlier
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
differ, one of them is wrong. The other half of the rule was learned the hard way: both
backends positioned paints in the wrong space, in the same way, for the same reason — the two
libraries share the convention that was misread — so they agreed with each other perfectly
while both were wrong. **Agreement is evidence only where the implementations can fail
independently.**

**Two copies of a constant is one defect waiting.** Three `DeviceCMYK` conversions disagreed
and nothing looked wrong. When that was fixed, the same shape survived one level down: the
nine-constant D50-to-sRGB matrix sat in `colour.rs` and in `icc.rs`. Nothing had gone wrong
yet, which is exactly the condition under which it is cheap to fix. It is now one function
with a test that recomputes all nine numbers from the two published matrices they were folded
from — so a folded constant, which is otherwise unreadable and unfalsifiable, has a
derivation attached.

**A test written to isolate one rule finds what a corpus cannot.** The ICC evaluator agreed
with two other readers on every real profile in the corpus. Writing a test that assembles a
profile *by hand* produced one whose darkest colour equals its white point, and black point
compensation divided by a span of floating-point noise and turned white into pure green. No
real profile is shaped that way. See trap 8, which is now the general form of this.

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
  swatches at the wrong lightness, a page one pixel short. Pages that agree have theirs
  deleted, so what is on disk is exactly the set worth looking at.
- **A page's tolerance class depends on what *we* drew.** The oracle picks a text tolerance
  or a vector one from our own render's content, so a change that adds text to a page also
  loosens its bound — and can move it from "ambiguous" to "judged". Four pages crossed that
  line this session when annotations started drawing, and all four had *improved*. When a
  page appears in the newly-contradicted list, check whether its bound changed before
  concluding the render got worse.
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
  The files carry base64 images inline, so `grep -v '^!\[Image\]'` before reading a range.
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
  expectations, since `pdftotext` supplies the reference for each. `issue20504.pdf` is the
  argument for doing it: nothing we own noticed six scripts rendering as ASCII.
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
  gate is 15 s in release and minutes in debug. Any test with a timing assertion is
  meaningless at debug speed; run those in release and say so in the test. The oracle gate
  is the exception that proves it: 1033 of its 1158 seconds of processor time is three
  external renderers, whose speed does not depend on how we were built.
- `cargo-deny` is installed in the agent's `~/.cargo/bin`; run it before pushing rather
  than finding out from a red pipeline.
