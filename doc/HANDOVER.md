# Handover

Written 2026-07-26, updated at the end of the second working session. Read `/CLAUDE.md`
first — it holds the four non-negotiable principles and they are not optional. This file
is the state of play, the traps, and what to do next.

## Where we are

A PDF **renderer** that opens real files and draws pages, with embedded text rendering
correctly on every document in the corpus.

- **151 tests**, `clippy` clean under `pedantic` + `unwrap_used`/`panic`/`arithmetic_side_effects`,
  `cargo fmt --check` clean, `cargo deny` clean on all four checks.
- The parser reads all fourteen specification PDFs in `doc/`, including ISO 32000-2 itself:
  1023 pages, 101 318 objects.
- Our render of the fixture agrees with poppler, mupdf and ghostscript, and is
  byte-identical to mupdf.
- **Every** corpus document renders page one with nothing unsupported except a soft mask on
  three of them. All fourteen extract **100% of the words `pdftotext` finds**.
- `doc/pdf.js` is a submodule with **974 real test documents**. All 974 open; 1501 of 1501
  PDF functions parse; **all 1793 shadings build**, mesh types included.

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

### 4. `#[expect]`, never `#[allow]`

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
| Colour management | Large | `DeviceCMYK` is uncalibrated; visible against poppler |
| Transparency groups, soft masks | Large | `/SMask` in `/ExtGState`; the last thing `doc/` reports |
| JBIG2, JPX | — | **Blocked on the sandbox, deliberately** |
| Encryption | Medium | RC4/AES, `/Encrypt` |
| Annotations, forms | Large | |
| Sandbox (Spike D) | Medium | seccomp-BPF + Landlock |

## The single most valuable next task

**Run the interpreter over `doc/pdf.js/test/pdfs`.** It is 974 real documents against
`doc/`'s fourteen, and it is deliberately full of the malformed files this parser will meet.
Every survey run against it during the shading work found something: one document that
would not open at all, and the exact distribution of shading and function types that shaped
the design. Wiring it in as a test — open, interpret, rasterise, assert no panic and that
whatever is unsupported is *reported* — is the highest-value thing left, because it turns a
one-off survey into a gate.

Do not expect it to be quiet. Triage will take a session.

After that, by what the corpus says real documents need rather than by what `doc/` contains:
**soft masks** (the last thing `doc/` reports), **encryption**, and **annotations**.
**Type1 fonts** remain worth checking `read_fonts::ps::type1` for before estimating.

**`doc/pdf.js` is a submodule** (Apache-2.0, pinned at v6.1.200) and is worth more than the
metrics it already supplied. `test/pdfs/` holds 974 real PDFs and 459 more behind link
files — a corpus two orders of magnitude larger and far nastier than `doc/`, including the
malformed files this parser will eventually meet. Running the interpreter over it is
probably the single highest-value test expansion available. It is optional to clone: the
generated metrics are checked in, so the build never needs it.

### Two habits this session earned

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
- **Pixel comparison cannot police text.** The reference renderers disagree with each other
  at worst-tile 26–28 on text pages — glyph hinting, not error. `Tolerance::TEXT_HEAVY`
  documents this as a weak gate. Text correctness belongs to the extraction metric.
- **`test-scenes` holds the same page twice**, as a display list and as PDF bytes. That
  pairing is what let the harness work before a parser existed, and it is checked by a test
  that renders both and demands identical pixels.
- `cargo-deny` is installed in the agent's `~/.cargo/bin`; run it before pushing rather
  than finding out from a red pipeline.
