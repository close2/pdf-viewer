# Handover

Written 2026-07-26, updated at the end of the third working session. Read `/CLAUDE.md`
first — it holds the five non-negotiable principles and they are not optional. **Principle
5 is new and it changes how to work**: the specification is the only source of truth, and
agreement with poppler, mupdf or pdf.js is evidence that we read it right, never the
definition of right. This file is the state of play, the traps, and what to do next.

## Where we are

A PDF **renderer** that opens real files and draws pages, with embedded text rendering
correctly on every document in the corpus.

- **171 tests**, `clippy` clean under `pedantic` + `unwrap_used`/`panic`/`arithmetic_side_effects`,
  `cargo fmt --check` clean, `cargo deny` clean on all four checks.
- The parser reads all fourteen specification PDFs in `doc/`, including ISO 32000-2 itself:
  1023 pages, 101 318 objects.
- Our render of the fixture agrees with poppler, mupdf and ghostscript, and is
  byte-identical to mupdf.
- **Every** corpus document renders page one with nothing unsupported except a soft mask on
  three of them. All fourteen extract **100% of the words `pdftotext` finds**.
- `doc/pdf.js` is a submodule with **974 real test documents**, and they are now a **gate**
  rather than a survey: all 974 open, 955 reach page one, and everything the remaining 271
  cannot draw is *reported*. 1501 of 1501 PDF functions parse; **all 1793 shadings build**,
  mesh types included.
- Colour resolves from the document — `ICCBased` profiles are evaluated by an A2B evaluator
  written here, `/DefaultCMYK` and output intents are honoured, and there is exactly one
  `DeviceCMYK` conversion instead of the three that used to disagree. See ADR 0009.

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
cargo test --release -p pdf-model --test corpus -- --ignored --nocapture   # 974 docs, ~41 s
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

### 2. Test against real documents, not hand-written fragments

Cross-reference streams are compressed *and* PNG-predicted. The code said decoding them
was "the caller's responsibility" and then did not do it, so every modern PDF failed with
a misleading `/Root is not a dictionary`. Unit tests on fragments would never have caught
it; the corpus caught it on the first run.

`crates/pdf-syntax/tests/real_documents.rs` and
`crates/pdf-model/tests/render_real_pdf.rs` run over everything in `doc/`. Keep them
passing.

### 3. Unsupported input must stay loud

Every layer reports what it could not handle rather than skipping it: `Unsupported` in the
interpreter, `FontError`, `ImageError`, `CpuRasterError::UnsupportedCommand`. This is not
politeness — it is what makes the comparison harness trustworthy and what caught trap 1.
Do not "helpfully" fall back to a default that renders something plausible.

### 4. Colour: one conversion, and the specification often does not have an answer

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

### 5. `#[expect]`, never `#[allow]`

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

Each of these is reported at runtime rather than silently skipped.

| Missing | Size | Notes |
|---|---|---|
| Embedded CMap streams | Medium | Parse `begincidrange`/`begincidchar` |
| Predefined CMaps | Medium | Needs vendored data — licensing decision |
| Type1 fonts | Medium | `read_fonts::ps::type1` exists — check before writing any |
| Sampled shadings on the GPU | Small | Type 1 only; reported, 2 documents of 974 |
| Rendering intents beyond `AbsoluteColorimetric` | Small | Read and recorded; `A2B0` is not yet selected for `Perceptual` |
| Transparency groups, soft masks | Large | `/SMask` in `/ExtGState`; the last thing `doc/` reports |
| JBIG2, JPX | — | **Blocked on the sandbox, deliberately** |
| Encryption | Medium | RC4/AES, `/Encrypt` |
| Annotations, forms | Large | |
| Sandbox (Spike D) | Medium | seccomp-BPF + Landlock |

## The single most valuable next task

**The CPU rasteriser builds one page-sized clip mask per distinct clip, and caches every
one without bound.** The corpus gate found it on its first run.

`bug1721218_reduced.pdf` is an 825 kB file with a 612×792 page. It interprets in 414 ms and
then **rasterises in 39.6 seconds**, holding about 1.7 GB while it does. It references
**3576 distinct clips**; `MaskCache` in `crates/render-cpu/src/lib.rs` builds a
`tiny_skia::Mask` the size of the whole page for each one and keeps them all in a
`HashMap` with no eviction.

The cost is `clips × page area`, and that is measured rather than inferred: halving the
scale gives 10.6 s and quartering it 3.0 s, which is quadratic in the linear scale.

Both halves matter and they are different problems:

- **Time.** A mask only needs to cover the clip's own bounds. Most of these 3576 are small.
  `tiny_skia`'s API wants a mask matching the pixmap, so this is not a one-line change —
  it may mean drawing into a bounded sub-pixmap and compositing, or special-casing
  axis-aligned rectangular clips, which is what `re W n` produces and is very common.
- **Memory.** `MaskCache` needs a budget and an eviction policy regardless. 1.7 GB from one
  small file is denial-of-service surface, and principle 3 asks for exactly this bound.

Measure before choosing: `cargo run --release -p pdf-model --example open_one -- <file>
[scale]` prints the interpret and rasterise split and the distinct clip count, in a process
that can be killed. That example exists because of this bug.

After that, by what the corpus says real documents need rather than by what `doc/` contains:
**soft masks** (26 documents, and the last thing `doc/` reports), **encryption** (11 of the
19 documents that cannot reach page one), and **annotations**. **Type1 fonts** remain worth
checking `read_fonts::ps::type1` for before estimating.

### What the corpus gate reports today

Ratcheted in `crates/pdf-model/tests/corpus.rs`; the numbers only ever go down.

| | count | |
|---|---|---|
| unopenable | 0 | and it should stay there |
| no page one | 19 | 11 encrypted, 8 with unrecoverable page trees |
| draws incompletely | 271 | 152 JBIG2/JPX, 73 text, 26 soft mask, 19 transparency group, 1 bound reached |
| slower than 30 s | 1 | named, not counted — the clip defect above |

**The time budget reports; it cannot enforce.** A Rust thread cannot be cancelled, so a
document that never returns hangs the suite rather than failing it. A real budget has to
live inside the interpreter and the rasteriser. `PDFVIEWER_CORPUS_TRACE=1` names each
document on stderr as it starts and finishes, which is how a hang gets identified from a
killed run.

**`doc/pdf.js` is a submodule** (Apache-2.0, pinned at v6.1.200) and is worth more than the
metrics it already supplied. `test/pdfs/` holds 974 real PDFs and 459 more behind link
files — a corpus two orders of magnitude larger and far nastier than `doc/`, including the
malformed files this parser will eventually meet. Running the interpreter over it is
probably the single highest-value test expansion available. It is optional to clone: the
generated metrics are checked in, so the build never needs it.

### Habits these sessions earned

**Look in `read-fonts` before writing font-format code.** The previous handover specified
~80 lines of CFF charset parsing plus two 256-entry tables, and all of it already existed
in `read_fonts::ps`, which `skrifa` re-exports as `skrifa::raw`. See ADR 0006. The same
module also holds `type1`, `charmap` and `agl` — `agl` is now enabled and carries the
Adobe Glyph List, so nothing needs transcribing.

**Wall-clock benchmarks lie under load; count instructions instead.** A `Command::Fill`
change measured as a 24% *regression* on `cargo bench` and as an 8.5% *improvement* twenty
minutes later, purely from background build load. `valgrind --tool=callgrind` on
`crates/pdf-model/examples/callgrind_interpret.rs` settled it deterministically: 2.065 G
instructions before, 1.951 G after. Always A/B in one sitting, and prefer the instruction
count. `iai-callgrind` wraps this into a bench harness and is the right basis for the CI
perf gates `CLAUDE.md` asks for — not yet wired up.

**Two rasterisers disagreeing is information, not noise.** The CPU-versus-GPU agreement
test is what found that Vello needed the same mesh seam repair `tiny-skia` did, after a
comment here had confidently claimed otherwise. Where the backends differ, one of them is
wrong; sweeping a constant against that test is how its value was chosen.

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
  against `pdftotext` over the whole corpus. It is the only check that catches a code
  reaching a *plausible* wrong glyph. It found the operand-cap defect below on its first
  run, and it is known to bite: reverting that fix scores 93.2%, and shifting every
  `/ToUnicode` entry by one code scores 58.7%.
- **Silent caps are defects, not safety.** The interpreter dropped operands past the 64th,
  which truncated any `TJ` array holding a justified line — three sentences on the
  specification's own title page ended mid-word, with `unsupported: []`. Bounds against
  hostile input are right; reaching one without saying so is not. Every bound now reports.
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
