# Handover

Written 2026-07-26, at the end of the first working session. Read `/CLAUDE.md` first —
it holds the four non-negotiable principles and they are not optional. This file is the
state of play, the traps, and what to do next.

## Where we are

A PDF **renderer** that opens real files and draws pages. Not yet a viewer you would read
a document in, because a good share of text still does not render — see
[The single most valuable next task](#the-single-most-valuable-next-task).

- **99 tests**, `clippy` clean under `pedantic` + `unwrap_used`/`panic`/`arithmetic_side_effects`,
  `cargo fmt --check` clean, `cargo deny` clean on all four checks.
- **2 commits unpushed** at time of writing (`git log origin/main..HEAD`).
- The parser reads all fourteen specification PDFs in `doc/`, including ISO 32000-2 itself:
  1023 pages, 101 318 objects.
- Our render of the fixture agrees with poppler, mupdf and ghostscript, and is
  byte-identical to mupdf.

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
cargo deny check
cargo +nightly fuzz run lexer -- -runs=50000     # from fuzz/, needs nightly
```

## Crate map

| Crate | Does | Notes |
|---|---|---|
| `pdf-spec` | Object-model validation tables | Generated from Arlington by `build.rs` |
| `pdf-syntax` | Lexer, objects, xref, filters, `Document` | Touches untrusted bytes first |
| `pdf-model` | Page tree, content interpreter, image decode | Where PDF semantics live |
| `pdf-font` | Glyph outlines via `skrifa` | `cff.rs` is half-finished on purpose |
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
render `doc/PDF20_AN001-BPC.pdf` page 1 and **look at it**. There is a test that writes
the PNG: `cargo test -p pdf-model --test render_real_pdf -- --nocapture writes_an_inspectable`.

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
| Bare CFF code→glyph | Small | Container half done; see below |
| Non-embedded fonts | Small | Needs a substitution *policy* first |
| Embedded CMap streams | Medium | Parse `begincidrange`/`begincidchar` |
| Predefined CMaps | Medium | Needs vendored data — licensing decision |
| Type1 fonts | Large | eexec, Type1 charstrings, convert to Type2 |
| Shadings, patterns | Large | PDF types 1–7 |
| Transparency groups, soft masks | Large | `/SMask` in `/ExtGState` |
| JBIG2, JPX | — | **Blocked on the sandbox, deliberately** |
| Encryption | Medium | RC4/AES, `/Encrypt` |
| Annotations, forms | Large | |
| Sandbox (Spike D) | Medium | seccomp-BPF + Landlock |
| Text extraction metric | Small | Compare against `pdftotext` |

## The single most valuable next task

**Bare CFF code→glyph mapping.** It unblocks four corpus documents that currently render
no text at all, and it is contained.

Half is done: `pdf_font::cff::wrap_in_sfnt` builds an sfnt container skrifa accepts, with
the glyph count read from the font's own `CharStrings` index. It is tested and *not* on the
loading path.

The missing half, in `crates/pdf-font/`:

1. Parse the CFF `charset` (formats 0, 1, 2) to get a name SID per glyph ID, resolved via
   the standard-strings table plus the font's String INDEX. Roughly 80 lines of the same
   byte-parsing already in `cff.rs`.
2. Add `StandardEncoding` / `WinAnsiEncoding` tables (256 entries each) and apply
   `/Encoding`'s `/Differences` array over the base.
3. Join them into `name → GID`, then `code → name → GID`.
4. Remove the refusal at `crates/pdf-font/src/lib.rs` (~line 465) and **replace the
   fall-through in `LoadedFont::glyph_for`** — the `.or_else(|| u16::try_from(code).ok())`
   branch is what silently produced wrong glyphs. It is correct for subset TrueType fonts
   and wrong for CFF, so it must become conditional on the mapping.

Verify by rendering a page and looking at it, not by checking `unsupported`.

After that, **non-embedded fonts** is the next-smallest: drive advances from the PDF
`/Widths` array regardless of which substitute font is used, so glyphs land in the right
places even when their shapes differ.

## Things worth knowing

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
