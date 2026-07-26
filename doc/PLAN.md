# PDF Viewer — Build & Test System Plan

Status: draft, revised 2026-07-26. Scope: infrastructure only.
Project principles live in `/CLAUDE.md` and take precedence over anything here.

## 1. Stack decisions

| Area | Decision | Notes |
|---|---|---|
| Language | Rust | Eliminates the dominant CVE class in PDF viewers |
| Rasterizer | **CPU first, GPU behind a trait** | `tiny-skia` → `vello`/wgpu (ADR 0002) |
| Fonts | `skrifa` | Memory-safe FreeType replacement; Type1/Type3 in-tree |
| Windowing | `winit` | Qt dropped — see below |
| Dialogs | `ashpd` (XDG portal) | Native KDE dialogs without a Qt dependency |
| Accessibility | `AccessKit` | AT-SPI on Linux |
| Parallelism | `rayon` | Tiles, image decode, thumbnails — not the parser |
| Deflate | `flate2` + `zlib-rs` | Pure Rust at ~C speed |
| Spec model | Arlington PDF Model | Generated validation layer, see §5 |
| Sandbox | seccomp-BPF + Landlock | Renderer holds no fd, no network |

### Rationale, condensed

**Rust.** Memory corruption in parsers, font engines, and image decoders is the historical
CVE class (poppler, MuPDF, Acrobat). Rust removes it. It does *not* remove resource
exhaustion — decompression bombs, xref cycles, pathological shading — so explicit memory
and time budgets are architectural.

**Why not raw Vulkan.** Vulkan provides triangles and compute; PDF needs filled paths with
winding rules, stroking, nested clips, soft masks, transparency groups, blend modes, seven
shading types, and antialiased text. Building that on raw Vulkan is a project the size of
the PDF work itself. GPU rasterization is also not automatically faster: text-heavy pages
are bound by glyph rasterization and caching, and *time-to-first-page is usually dominated
by parsing and font loading, not rendering at all*. GPU wins on continuous zoom/pan, large
vector art, high-DPI, and thumbnail grids.

**Why CPU first.** Reaches a correct rendered page soonest, and yields a **same-scene
oracle**: diffing our own CPU and GPU backends on an identical display list is far tighter
than any cross-viewer comparison, because both consume the same input. `vello_cpu` keeps
both backends on one scene model, so the swap stays cheap.

**Why Qt was dropped.** Qt was justified by native KDE file dialogs and accessibility.
Neither holds: `xdg-desktop-portal-kde` is installed, so *any* toolkit gets native KDE
dialogs through the portal via `ashpd`; and `AccessKit` provides AT-SPI accessibility for
custom-drawn Rust UIs. That removes the justification for the `cxx-qt` bridge, moc, and an
eventual CMake/Corrosion migration — the most fragile part of the build. A pure-Rust stack
with no FFI boundary is also materially better against principle 4 (exemplary code): the
whole stack reads in one language.

Given up: a mature widget set if the UI grows beyond a viewer, and Qt's free
i18n/menu/shortcut infrastructure. Revisit if AcroForm editing UI becomes a goal.

**`rustybuzz` is deliberately excluded.** PDF content streams carry already-positioned
glyphs — the producer shaped them at authoring time. Re-shaping would move glyphs away
from where the document specifies, breaking fidelity precisely on complex-script
documents. Reconsider only for text we generate ourselves.

**Image codecs.** `zune-jpeg` / `zune-png` cover the common cases in pure Rust. **JBIG2 and
JPEG2000 have no mature pure-Rust implementation** and are historically severe attack
surfaces (FORCEDENTRY was a JBIG2 integer overflow). If C libraries are wrapped there,
they must live in the sandboxed process — these two decoders alone justify the sandbox.

## 2. Workspace layout

```
pdf-viewer/
├─ crates/
│  ├─ pdf-spec/       # Arlington codegen output + validation  [forbid(unsafe_code)]
│  ├─ pdf-syntax/     # lexer, objects, xref, streams          [forbid(unsafe_code)]
│  ├─ pdf-model/      # document model, page tree              [forbid(unsafe_code)]
│  ├─ pdf-font/       # skrifa integration, Type1/Type3        [forbid(unsafe_code)]
│  ├─ pdf-render/     # display list, backend trait            [forbid(unsafe_code)]
│  ├─ render-cpu/     # vello_cpu backend — the oracle
│  ├─ render-gpu/     # vello/wgpu backend              [unsafe allowed]
│  ├─ pdf-sandbox/    # seccomp + landlock + IPC
│  ├─ viewer-core/    # app logic, toolkit-agnostic
│  └─ viewer-ui/      # winit + AccessKit + ashpd shell
├─ tools/pdfref/      # reference-comparison harness
├─ tools/corpus/      # corpus fetch/manage
├─ fuzz/  benches/  tests/  doc/  doc/adr/
```

`forbid(unsafe_code)` is load-bearing: it makes "untrusted bytes never reach unsafe code"
compiler-enforced rather than conventional.

## 3. Phases

### Phase 0 — Foundation — *mostly done*
- [x] `git init`
- [x] rustup adopted; stable 1.97.1 + nightly with Miri installed
- [x] `rust-toolchain.toml` pinned to an exact version
- [x] `rustfmt.toml`, `clippy.toml`, `deny.toml`, `.gitignore`
- [ ] Vulkan packages (§7) — needs a global install
- [x] CI — GitHub Actions in `.github/workflows/ci.yml`: fmt, clippy, tests with
      `mesa-vulkan-drivers` for a software Vulkan adapter, `cargo-deny`, and an advisory
      Miri job

### Phase 1 — Workspace skeleton — *done*
Crate graph above with safety attributes in place. `pdf-render` defines the display list,
the `Rasterizer` trait and `TargetSpec`, with 13 unit tests. Clean under
`clippy::pedantic` with warnings-as-errors, and `cargo fmt --check` clean.

Every lint exception in the tree is an `#[expect(..., reason = "...")]` rather than a
bare `allow`, so an exception that stops being necessary becomes a warning instead of
lingering invisibly.

### Phase 2 — Build system
Cargo only. No CMake, no moc, no Corrosion — dropping Qt removed the need. `build.rs` in
`pdf-spec` runs the Arlington codegen.

### Phase 3 — Reference-comparison harness
See §4. Built before real rendering exists, validated on a hand-written trivial PDF.

### Phase 4 — Test layers
- `cargo-nextest` — unit tests
- `proptest` — parser round-trips
- `cargo-fuzz` — from the first parser commit; every crasher becomes a regression test
- reference harness (§4)
- `criterion` + **perf gate**: cold open, time-to-first-page, page-turn latency, memory
  high-water, measured with a cold page cache. Regression fails the build.
  Startup latency is a first-class requirement — see `CLAUDE.md` principle 2 for the
  rules that follow from it, notably that GPU initialisation stays off the critical path
  and page one renders on the CPU backend while the device is created.
- Miri on the pure-Rust core; ASan/UBSan on any FFI
- `cargo-deny`, `cargo-audit`

### Phase 5 — De-risking spikes (before PDF code)
- **A.** ~~Headless CPU render → byte-deterministic output.~~ **Done.** `render-cpu` on
  `tiny-skia`; fills, strokes and nested clips verified, output byte-identical across
  runs, PNG artefact written for inspection. 9 tests. Confirmed `tiny-skia` covers all
  sixteen PDF blend modes.
- **B.** ~~GPU backend on Vello/wgpu.~~ **Done (headless part).** Offscreen render with
  no window or display server; cross-backend agreement with `render-cpu` verified within
  measured tolerances; row-padding readback covered. 7 tests. See ADR 0004.
  *Outstanding:* the winit window that presents a surface, which cannot be verified
  headlessly and is left for manual confirmation.
- **C.** Arlington TSV → generated Rust validation tables; verify against a known object.
- **D.** Sandboxed child process returning a tile over shared memory.
- **E.** Harness end-to-end: CPU-backend render vs `pdftoppm` on a hand-written PDF.
  Validates metric plumbing before a parser exists.

## 4. Reference-comparison harness

Three independent reference implementations are installed; their *agreement* is the
evidence we rely on.

| Renderer | Command | Version |
|---|---|---|
| poppler | `pdftoppm -r 150 -png -aa yes` | 26.07.0 |
| mupdf | `mutool draw -r 150 -o out.png` | 1.28.0 |
| ghostscript | `gs -sDEVICE=png16m -r150` | 10.07.1 |
| pdfium | *to add (AUR)* | de-facto standard — Chrome's renderer |

### Expect inexact agreement

Exact pixel equality is impossible even *between* poppler and mupdf: they differ in
antialiasing, gamma, subpixel glyph positioning, MediaBox-vs-CropBox choice, and pixel
rounding. A harness built on exact comparison produces false positives until it gets
ignored. Tolerance is a design requirement, not a concession.

### Metrics ladder (cheapest and strictest first)

1. **Geometry** — page count, dimensions, rotation. Exact match required.
2. **Text** — our extraction vs `pdftotext`. Validates encoding and `ToUnicode`
   *independently of rendering*, isolating a whole error class.
3. **Structural similarity** — SSIM / blurred difference, per-corpus tolerance.
4. **Localized max error** — tile the page, report worst tile. Mean metrics average away a
   single missing glyph on a dense page; this is the one people forget.

### Triangulation rule

- ≥2 references agree and we differ → real bug, fail the build.
- All references disagree with each other → ambiguous spec corner; record as
  known-divergent, do not fail.

This is what keeps the suite trustworthy enough to stay enabled.

### Goldens

Snapshots of *our own* output, separate from reference comparison, catching
commit-to-commit regressions including in deliberately-divergent areas.

**Correction to an earlier assumption here.** This originally said RADV and lavapipe
would not produce identical pixels, so goldens had to be per-backend. Measurement showed
the opposite: for the vector path they are byte-identical, because Vello's compute
pipeline has no driver-dependent fixed-function rasterisation. Goldens can therefore be
shared across adapters, and a test pins that property so its loss is noticed. Checked on
one vendor and simple scenes only — text and images may still diverge. See ADR 0004.

On failure, emit side-by-side plus difference heatmap as CI artifacts — diagnosis must
take seconds.

### Corpus

`doc/*.pdf` is a strong start: ISO 32000-2 is large, complex, tagged, font-heavy, real.
Add pdf.js and pdfium corpora, veraPDF, Isartor (malformed files), Arlington's own
`TestGrammar/test/` fixtures, and the growing fuzz corpus. Large corpora fetched on
demand, not committed.

Where all open-source renderers are jointly wrong, Acrobat is the gold standard and is not
scriptable on Linux — keep a small manually-captured Acrobat golden set.

## 5. Arlington PDF Model

Cloned at `doc/arlington-pdf-model` — 3468 TSVs, with `tsv/2.0/` defining the PDF 2.0
object model. Columns: `Key, Type, SinceVersion, DeprecatedIn, Required,
IndirectReference, Inheritable, DefaultValue, PossibleValues, SpecialCase, Link, Note`.

**Plan: generate the validation layer, don't hand-write it.** A `build.rs` step in
`pdf-spec` turns the TSVs into typed accessors and validation tables. Benefits:

- Spec conformance becomes reviewable *data* rather than thousands of hand-written checks.
- Version-awareness (`SinceVersion` / `DeprecatedIn`) comes free.
- `Link` encodes the object graph, giving typed traversal.
- Directly serves principles 1 and 4: no shortcuts, and legible to a reader.

Open question: how much of `SpecialCase` (a small predicate language) to implement in
codegen versus by hand. Needs a spike — see Phase 5C.

## 6. Security architecture

Memory safety is necessary, not sufficient.

- Renderer process: unprivileged, seccomp-BPF + Landlock, no filesystem, no network.
  Receives bytes over an fd, returns tiles via shared memory. UI process holds the only fd.
- GPU-touching code ideally its own process — drivers are unsafe C and exploitable.
- Explicit memory/time budgets against decompression bombs and pathological content.
- Any C image codec (JBIG2, JPX) confined to the sandboxed process.
- AcroForm JavaScript, if ever supported, is a separate sandboxing problem. Defer, but
  don't design it out.

Crates: `landlock`, `seccompiler`, `rustix`.

## 7. Environment

Verified: rustc/cargo 1.97.1, cmake 4.4.0, ninja 1.13.2, clang 22.1.8, poppler 26.07.0,
mupdf-tools 1.28.0, ghostscript 10.07.1, qpdf 12.3.2, imagemagick 7.1.2.27, python 3.14.6
+ pillow 12.3.0 + numpy 2.5.1, xdg-desktop-portal 1.22.1 + `-kde` 6.7.3 + `-gtk` 1.15.3,
kio/kconfig/ki18n 6.28.0.

GPU: AMD Strix (Radeon 880M / 890M), RDNA 3.5. Session: X11 (`DISPLAY=:0`).

### Packages

Installed in the last round: `vulkan-radeon`, `vulkan-swrast`,
`vulkan-validation-layers`, `mupdf` — **verify with `vulkaninfo --summary`**.

Note: KDE Frameworks 6 on Arch has **no `kf6-` prefix**. `kio`, `kconfig`, `ki18n` are
already installed; with Qt dropped they are no longer needed anyway.

Still wanted: `pdfium` (AUR, 4th reference renderer). Via cargo: `cargo-fuzz`,
`cargo-nextest`, `cargo-deny`, `cargo-audit`.

`vulkan-swrast` matters more than it looks: it makes GPU output reproducible in CI so
visual diffs don't go flaky on driver updates.

### Caveats

- Not yet a git repository.
- Claude Code may run as `AI` via `sudo -u AI` through the `coders` group; `/home/cl` is
  mode 711. That user has no X authority cookie, so GUI windows cannot be opened from such
  a session — headless lavapipe covers tests; interactive runs need a `cl` session.

## 8. Open questions

- Extent of Arlington `SpecialCase` predicate support in codegen (Phase 5C).
- `vello_cpu` feature coverage vs falling back to `tiny-skia` for backend #1.
- Acrobat golden-set capture process.
- Type1 font strategy: convert to CFF, or implement directly.
